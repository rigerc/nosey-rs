//! Bounded blocking HTTPS transport for explicit registry discovery and trust checks.
//! Run on a worker thread, never a GPUI or async-runtime thread.

use ai_catalog_discover::{Fetcher, Response};
use ai_catalog_validate::Fetch;
use reqwest::{blocking::Client, dns::{Addrs, Name, Resolve, Resolving}, header, redirect::Policy};
use std::{
    cell::Cell,
    io::{self, Read},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::Arc,
    time::{Duration, Instant},
};
use url::{Host, Url};

const MAX_BODY: usize = 10 * 1024 * 1024;
const TOTAL_BODY: usize = 32 * 1024 * 1024;
const MAX_REQUESTS: usize = 32;
const MAX_REDIRECTS: usize = 5;
const DEADLINE: Duration = Duration::from_secs(60);
const PER_REQUEST: Duration = Duration::from_secs(15);

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a,b,c,_] = ip.octets();
    // Reject private, local, documentation, multicast, reserved, benchmark,
    // shared-address and other special-purpose ranges conservatively.
    !(matches!(a, 0 | 10 | 127 | 224..=255)
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && (b == 0 || b == 168 || (b == 88 && c == 99)))
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113))
}

fn public_v6(ip: Ipv6Addr) -> bool {
    let [a,b,_,_,_,_,_,_] = ip.segments();
    // Accept only global unicast; exclude special-purpose ranges, transition
    // addresses and documentation. IPv4-mapped and NAT64 addresses fail.
    (0x2000..=0x3fff).contains(&a)
        && !(a == 0x2001 && (b < 0x0200 || b == 0x0db8))
        && a != 0x2002
        && !(a == 0x3fff && b < 0x1000)
}

fn public(ip: IpAddr) -> bool {
    match ip { IpAddr::V4(ip) => public_v4(ip), IpAddr::V6(ip) => public_v6(ip) }
}

fn valid_domain(name: &str) -> bool {
    let lower = name.trim_end_matches('.').to_ascii_lowercase();
    lower.contains('.') && !["localhost", "local", "internal", "lan", "home", "test", "invalid"]
        .iter().any(|s| lower == *s || lower.ends_with(&format!(".{s}")))
}

pub(crate) fn validate_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
        || url.port_or_known_default() != Some(443) || url.as_str().len() > 4096
    {
        return Err("registry URL policy rejected".into());
    }
    let allowed = match url.host() {
        Some(Host::Domain(name)) => valid_domain(name),
        Some(Host::Ipv4(ip)) => public_v4(ip),
        Some(Host::Ipv6(ip)) => public_v6(ip),
        None => false,
    };
    if !allowed { return Err("registry destination is not public".into()); }
    Ok(())
}

struct PublicDns;

impl Resolve for PublicDns {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let domain = name.as_str();
            if !valid_domain(domain) {
                return Err(io::Error::other("DNS name rejected").into());
            }
            // Use an absolute name to avoid resolver search-suffix expansion.
            let fqdn = format!("{}.", domain.trim_end_matches('.'));
            let iter = tokio::time::timeout(Duration::from_secs(5),
                tokio::net::lookup_host((fqdn.as_str(), 0))).await
                .map_err(|_| io::Error::other("DNS timeout"))??;
            let mut addresses = Vec::new();
            for addr in iter {
                if !public(addr.ip()) || addresses.len() >= 32 {
                    return Err(io::Error::other("DNS returned prohibited address or too many addresses").into());
                }
                addresses.push(addr);
            }
            if addresses.is_empty() { return Err(io::Error::other("no public DNS address").into()); }
            // These vetted addresses are what reqwest connects to; no second
            // independent DNS resolution is performed by our transport.
            let addrs: Addrs = Box::new(addresses.into_iter());
            Ok(addrs)
        })
    }
}

/// One bounded operation using direct HTTPS with a restrictive DNS resolver.
/// Not a general-purpose browser or an authenticated client.
pub struct SafeTransport {
    client: Client,
    started: Instant,
    requests: Cell<usize>,
    bytes: Cell<usize>,
}

impl SafeTransport {
    /// Create a fresh transport for one discovery or verification operation.
    ///
    /// # Errors
    ///
    /// Returns a sanitized client initialization failure.
    pub fn new() -> Result<Self, crate::Error> {
        let client = Client::builder().https_only(true).no_proxy()
            .redirect(Policy::none()).referer(false)
            .dns_resolver(Arc::new(PublicDns))
            .connect_timeout(Duration::from_secs(5)).timeout(PER_REQUEST)
            .pool_max_idle_per_host(0)
            .user_agent("nosey-registry")
            .build().map_err(|_| crate::Error::Invalid("HTTPS transport initialization".into()))?;
        Ok(Self { client, started: Instant::now(), requests: Cell::new(0), bytes: Cell::new(0) })
    }

    fn get_response(&self, start: &Url) -> Result<Response, String> {
        let mut url = start.clone();
        url.set_fragment(None);
        for redirects in 0..=MAX_REDIRECTS {
            validate_url(&url)?;
            let remaining = DEADLINE.checked_sub(self.started.elapsed())
                .ok_or("registry request deadline")?;
            if self.requests.get() >= MAX_REQUESTS { return Err("registry request budget".into()); }
            self.requests.set(self.requests.get()+1);
            let response = self.client.get(url.clone()).timeout(remaining.min(PER_REQUEST))
                .header(header::ACCEPT, "application/ai-catalog+json, application/json, text/html, */*")
                .header(header::ACCEPT_ENCODING, "identity")
                .send().map_err(|_| "HTTPS request failed")?;
            if !response.remote_addr().is_some_and(|a| public(a.ip())) {
                return Err("connection destination could not be verified as public".into());
            }
            let status = response.status().as_u16();
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                if redirects == MAX_REDIRECTS { return Err("registry redirect limit".into()); }
                let location = response.headers().get(header::LOCATION)
                    .and_then(|v| v.to_str().ok()).ok_or("invalid redirect")?;
                if location.len() > 4096 { return Err("redirect URL too large".into()); }
                url = url.join(location).map_err(|_| "invalid redirect URL")?;
                url.set_fragment(None);
                continue;
            }
            let content_type = response.headers().get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()).filter(|v| v.len() <= 256).map(str::to_owned);
            let mut link_headers = Vec::new();
            let mut header_bytes = 0_usize;
            for value in response.headers().get_all(header::LINK) {
                let value = value.to_str().map_err(|_| "invalid registry link header")?;
                header_bytes = header_bytes.saturating_add(value.len());
                if header_bytes > 16*1024 || link_headers.len() >= 32 {
                    return Err("registry link header budget".into());
                }
                link_headers.push(value.to_owned());
            }
            let max = MAX_BODY.min(TOTAL_BODY.saturating_sub(self.bytes.get()));
            if max == 0 || response.content_length().is_some_and(|n| n > max as u64) {
                return Err("registry body budget".into());
            }
            let mut body = Vec::new();
            response.take(max as u64+1).read_to_end(&mut body).map_err(|_| "registry body read failed")?;
            self.bytes.set(self.bytes.get().saturating_add(body.len()));
            if body.len() > max { return Err("registry body budget".into()); }
            return Ok(Response { url, status, content_type, link_headers, body });
        }
        Err("registry redirect limit".into())
    }
}

impl Fetcher for SafeTransport {
    fn get(&self, url: &Url) -> Result<Response, String> { self.get_response(url) }
}

impl Fetch for SafeTransport {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let url = Url::parse(url).map_err(|_| "invalid verification URL")?;
        let response = self.get_response(&url)?;
        if !response.is_success() { return Err("verification fetch failed".into()); }
        Ok(response.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_special_ranges_and_credentials() -> Result<(), Box<dyn std::error::Error>> {
        for address in ["127.0.0.1","10.0.0.1","100.64.0.1","169.254.169.254",
            "192.168.1.1","198.18.0.1","203.0.113.1","::1","::ffff:127.0.0.1",
            "fe80::1","fc00::1","2001:db8::1","2002:7f00:1::1"] {
            assert!(!public(address.parse()?));
        }
        assert!(public("8.8.8.8".parse()?));
        assert!(public("2606:4700::1111".parse()?));
        for url in ["http://example.com", "https://user@example.com", "https://localhost",
            "https://host.local", "https://127.0.0.1", "https://[::1]", "https://example.com:8443"] {
            assert!(validate_url(&Url::parse(url)?).is_err());
        }
        assert!(validate_url(&Url::parse("https://example.com/catalog")?).is_ok());
        Ok(())
    }
}
