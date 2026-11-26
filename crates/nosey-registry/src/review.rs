//! Discovery, signature verification and explicit approval boundary.
//! No registry is activated by discovery, signature presence, or a successful fetch alone.

use crate::transport::{SafeTransport, validate_url};
use crate::{Error, MAX_CATALOG_BYTES, Snapshot, intelligence, parse_catalog, save_snapshot};
use ai_catalog_discover::{Fetcher, Method, discover_with};
use ai_catalog_validate::{Fetch, Options, Outcome, Verification, verify};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

const DEFAULT_TTL: u64 = 24 * 60 * 60;

/// Identity presented to the user before a registry is approved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Final catalog URL, not the discovery input.
    pub url: String,
    /// Catalog-declared host ID, if present.
    pub host_id: Option<String>,
    /// Display name from the catalog, not an authenticated publisher identity.
    pub display_name: Option<String>,
}

/// Locally approved source and policy. Importers must remove approvals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Local registry namespace.
    pub id: String,
    /// Smaller rank has higher precedence.
    pub priority: u32,
    /// Registry source is enabled.
    pub enabled: bool,
    /// Approved endpoint and declared host identity.
    pub identity: Identity,
    /// Explicit local approval, never inferred from signatures.
    pub approved: bool,
    /// Catalog refresh interval in seconds.
    pub ttl_secs: u64,
    /// Operator identities authorized locally, not from catalog data.
    pub authorized_operators: Vec<String>,
    /// Require a verified signature from an authorized catalog operator.
    pub require_operator: bool,
    /// Last successful, persisted refresh time (Unix seconds).
    pub refreshed_at: Option<u64>,
}

impl RegistryConfig {
    /// Clear approval on importing settings from another installation.
    pub fn clear_imported_approval(&mut self) {
        self.approved = false;
        self.refreshed_at = None;
    }

    /// Whether an online refresh is needed. Unknown/invalid clock is stale.
    #[must_use]
    pub fn refresh_due(&self, now: SystemTime) -> bool {
        let Ok(now) = now.duration_since(UNIX_EPOCH) else {
            return true;
        };
        let Some(last) = self.refreshed_at else {
            return true;
        };
        now.as_secs()
            .checked_sub(last)
            .is_none_or(|age| age >= self.ttl_secs)
    }
}

/// Reviewed catalog still requiring explicit local approval for activation.
#[derive(Debug)]
pub struct ReviewedCatalog {
    identity: Identity,
    snapshot: Snapshot,
    verification: Verification,
    discovery: Option<Method>,
}

impl ReviewedCatalog {
    /// Identity as reviewed.
    #[must_use]
    pub const fn identity(&self) -> &Identity {
        &self.identity
    }

    /// SDK verification report; distinguish unverified from failed assertions.
    #[must_use]
    pub const fn verification(&self) -> &Verification {
        &self.verification
    }

    /// Method used for an explicitly requested discovery.
    #[must_use]
    pub const fn discovery_method(&self) -> Option<Method> {
        self.discovery
    }

    /// Read-only original bytes and parsed catalog for preview.
    #[must_use]
    pub const fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Approve the exact reviewed source after a user confirms its identity.
    ///
    /// # Errors
    ///
    /// Rejects invalid local namespace or mismatched intelligence ownership.
    pub fn approve(
        &self,
        id: &str,
        priority: u32,
        extension: &str,
    ) -> Result<RegistryConfig, Error> {
        // Official namespace is reserved until its built-in endpoint is configured.
        if id.eq_ignore_ascii_case("official") {
            return Err(Error::Invalid(
                "official registry namespace is reserved".into(),
            ));
        }
        validate_namespace(&self.snapshot, id, extension)?;
        Ok(RegistryConfig {
            id: id.into(),
            priority,
            enabled: true,
            identity: self.identity.clone(),
            approved: true,
            ttl_secs: DEFAULT_TTL,
            authorized_operators: Vec::new(),
            require_operator: false,
            refreshed_at: None,
        })
    }

    /// Persist original bytes only after a matching local approval.
    ///
    /// # Errors
    ///
    /// Rejects unapproved, disabled, or changed identities and failed trust
    /// policy; failed cache writes leave the previous snapshot intact.
    pub fn commit(
        &self,
        config: &mut RegistryConfig,
        path: &Path,
        extension: &str,
        now: SystemTime,
    ) -> Result<(), Error> {
        if !config.enabled || !config.approved || config.identity != self.identity {
            return Err(Error::Invalid(
                "registry requires explicit matching approval".into(),
            ));
        }
        if config.require_operator
            && (!self.verification.catalog_authorized
                || !self
                    .snapshot
                    .catalog
                    .signature
                    .as_ref()
                    .is_some_and(|signature| {
                        config.authorized_operators.contains(&signature.signer)
                    }))
        {
            return Err(Error::Invalid(
                "required operator verification unavailable".into(),
            ));
        }
        validate_namespace(&self.snapshot, &config.id, extension)?;
        let timestamp = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Invalid("invalid refresh clock".into()))?
            .as_secs();
        save_snapshot(path, &self.snapshot.bytes)?;
        config.refreshed_at = Some(timestamp);
        Ok(())
    }
}

fn validate_namespace(snapshot: &Snapshot, id: &str, key: &str) -> Result<(), Error> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(Error::Invalid("invalid local registry ID".into()));
    }
    if let Some(intel) = intelligence(&snapshot.catalog, key)? {
        if intel.registry_id != id {
            return Err(Error::Invalid("registry namespace mismatch".into()));
        }
    }
    Ok(())
}

fn failed(outcome: Option<&Outcome>) -> bool {
    outcome.is_some_and(Outcome::is_failed)
}

fn review_bytes(
    bytes: Vec<u8>,
    url: Url,
    fetch: &dyn Fetch,
    operators: &[String],
    method: Option<Method>,
) -> Result<ReviewedCatalog, Error> {
    validate_url(&url).map_err(Error::Invalid)?;
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(Error::TooLarge);
    }
    let catalog = parse_catalog(&bytes)?;
    let options = Options::default()
        .with_base_url(url.as_str())
        .with_authorized_operators(operators.iter().cloned());
    let verification =
        verify(&bytes, &options, fetch).map_err(|_| Error::Invalid("SDK verification".into()))?;
    if !verification.validation.is_valid
        || failed(verification.catalog_signature.as_ref())
        || verification.entries.iter().any(|e| {
            e.publisher_failed
                || failed(e.artifact.as_ref())
                || e.manifests
                    .iter()
                    .any(|m| failed(m.signature.as_ref()) || failed(m.release.as_ref()))
        })
    {
        return Err(Error::Invalid(
            "catalog signature, release, digest or validation failed".into(),
        ));
    }
    let identity = Identity {
        url: url.to_string(),
        host_id: catalog.host.as_ref().and_then(|h| h.identifier.clone()),
        display_name: catalog.host.as_ref().map(|h| h.display_name.clone()),
    };
    Ok(ReviewedCatalog {
        identity,
        snapshot: Snapshot { bytes, catalog },
        verification,
        discovery: method,
    })
}

/// Perform explicit discovery using the SDK and the bounded Nosey transport.
/// Must be run on a blocking worker, with connectivity established by caller.
///
/// # Errors
///
/// Offline, failed discovery, malformed catalogs and failed assertions are rejected.
pub fn discover_registry(input: &str, online: bool) -> Result<ReviewedCatalog, Error> {
    if !online {
        return Err(Error::Invalid(
            "registry discovery unavailable offline".into(),
        ));
    }
    if input.len() > 4096 {
        return Err(Error::Invalid("discovery input limit".into()));
    }
    let transport = SafeTransport::new()?;
    let discovery = discover_with(&transport, input)
        .map_err(|_| Error::Invalid("SDK discovery failed".into()))?;
    let url =
        Url::parse(&discovery.url).map_err(|_| Error::Invalid("invalid resolved URL".into()))?;
    review_bytes(
        discovery.bytes,
        url,
        &transport,
        &[],
        Some(discovery.method),
    )
}

/// Fetch the already approved endpoint, never implicitly rediscover it.
/// Changed identity requires review and approval; the old cache is untouched.
///
/// # Errors
///
/// Rejects offline, disabled, unapproved or failed refreshes and identity changes.
pub fn refresh_registry(config: &RegistryConfig, online: bool) -> Result<ReviewedCatalog, Error> {
    if !online || !config.approved || !config.enabled {
        return Err(Error::Invalid("registry refresh unavailable".into()));
    }
    let url = Url::parse(&config.identity.url)
        .map_err(|_| Error::Invalid("invalid approved URL".into()))?;
    let transport = SafeTransport::new()?;
    let response = Fetcher::get(&transport, &url).map_err(Error::Invalid)?;
    if !response.is_success() {
        return Err(Error::Invalid("registry refresh HTTP failure".into()));
    }
    let reviewed = review_bytes(
        response.body,
        response.url,
        &transport,
        &config.authorized_operators,
        None,
    )?;
    if reviewed.identity != config.identity
        || (config.require_operator && !reviewed.verification.catalog_authorized)
    {
        return Err(Error::Invalid(
            "registry identity or required trust changed".into(),
        ));
    }
    Ok(reviewed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn approval_and_ttl_are_not_inferred_from_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let bytes = br#"{"specVersion":"1.0","host":{"displayName":"Fixture","identifier":"example.org"},"entries":[]}"#.to_vec();
        let no_network =
            |_: &str| -> Result<Vec<u8>, String> { Err("fixture never fetches".into()) };
        let review = review_bytes(
            bytes.clone(),
            Url::parse("https://example.org/catalog")?,
            &no_network,
            &[],
            None,
        )?;
        assert!(review.verification().catalog_signature.is_none());
        let mut config = review.approve("fixture", 1, "org.example.nosey.intelligence")?;
        assert!(config.refresh_due(UNIX_EPOCH));
        let temp = tempfile::tempdir()?;
        let cache = temp.path().join("catalog.json");
        review.commit(
            &mut config,
            &cache,
            "org.example.nosey.intelligence",
            UNIX_EPOCH + Duration::from_secs(100),
        )?;
        assert_eq!(std::fs::read(&cache)?, bytes);
        assert!(!config.refresh_due(UNIX_EPOCH + Duration::from_secs(101)));
        assert!(config.refresh_due(UNIX_EPOCH + Duration::from_secs(100 + DEFAULT_TTL)));
        config.clear_imported_approval();
        assert!(
            review
                .commit(
                    &mut config,
                    &cache,
                    "org.example.nosey.intelligence",
                    UNIX_EPOCH
                )
                .is_err()
        );
        assert!(refresh_registry(&config, true).is_err());
        assert!(discover_registry("example.org", false).is_err());
        Ok(())
    }
}
