//! Isolated Git subprocesses sharing one acquisition deadline.

use super::GitProtocol;
use crate::{Error, library_io::invalid};
use rustix::{
    fs::{OFlags, fcntl_setfl},
    process::{Pid, Signal, kill_process_group},
};
use std::{
    fs,
    io::{self, Read},
    net::IpAddr,
    os::unix::process::CommandExt as _,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const BUFFER_SIZE: usize = 8192;
const VERSION_LIMIT: usize = 256;
const MINIMUM_VERSION: (u32, u32) = (2, 42);

pub(super) struct Git<'a> {
    repo: &'a Path,
    home: PathBuf,
    protocol: GitProtocol,
    deadline: Instant,
}

impl<'a> Git<'a> {
    pub(super) fn new(repo: &'a Path, protocol: GitProtocol) -> Result<Self, Error> {
        let home = repo.join("home");
        fs::create_dir(&home)?;
        Ok(Self {
            repo,
            home,
            protocol,
            deadline: Instant::now() + DEADLINE,
        })
    }

    pub(super) const fn protocol(&self) -> GitProtocol {
        self.protocol
    }

    pub(super) fn fetch(&self, url: &str, revision: &str) -> Result<(), Error> {
        let fetch = [
            "fetch",
            "--depth=1",
            "--no-tags",
            "--no-recurse-submodules",
            "origin",
            revision,
        ];
        match self.protocol {
            GitProtocol::Https => {
                let version = self.run(&["--version"], VERSION_LIMIT)?;
                let version =
                    std::str::from_utf8(&version).map_err(|_| invalid("invalid Git version"))?;
                let mut parts = version
                    .trim()
                    .strip_prefix("git version ")
                    .unwrap_or_default()
                    .split('.');
                let major = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
                let minor = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
                if (major, minor) < MINIMUM_VERSION {
                    return Err(invalid("HTTPS acquisition requires Git 2.42 or newer"));
                }
                let repo =
                    url::Url::parse(url).map_err(|_| invalid("invalid Git repository URL"))?;
                let pin = dns_pin(repo, self.deadline)?;
                let mut args = Vec::new();
                if let Some(pin) = pin.as_deref() {
                    args.extend(["-c", pin]);
                }
                args.extend(fetch);
                self.run(&args, 0)?;
            }
            #[cfg(test)]
            GitProtocol::File => {
                self.run(&fetch, 0)?;
            }
        }
        Ok(())
    }

    pub(super) fn run(&self, args: &[&str], limit: usize) -> Result<Vec<u8>, Error> {
        if Instant::now() >= self.deadline {
            return Err(invalid("Git acquisition timed out"));
        }
        let (protocol, file) = match self.protocol {
            GitProtocol::Https => ("https", "protocol.file.allow=never"),
            #[cfg(test)]
            GitProtocol::File => ("file", "protocol.file.allow=always"),
        };
        let mut child = Command::new("git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_ALLOW_PROTOCOL", protocol)
            .current_dir(self.repo)
            .process_group(0)
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                file,
                "-c",
                "core.symlinks=false",
                "-c",
                "submodule.recurse=false",
                "-c",
                "http.followRedirects=false",
                "-c",
                "protocol.version=2",
                "-c",
                "fetch.fsckObjects=true",
                "-c",
                "maintenance.auto=false",
                "-c",
                "gc.auto=0",
            ])
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        match self.collect(&mut child, limit) {
            Ok((status, bytes)) if status.success() => Ok(bytes),
            Ok(_) => Err(invalid(&format!("Git {} failed", args[0]))),
            Err(error) => {
                let _ = kill_process_group(Pid::from_child(&child), Signal::KILL);
                let _ = child.wait();
                Err(error)
            }
        }
    }

    fn collect(&self, child: &mut Child, limit: usize) -> Result<(ExitStatus, Vec<u8>), Error> {
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| invalid("Git stdout unavailable"))?;
        fcntl_setfl(&stdout, OFlags::NONBLOCK).map_err(io::Error::from)?;
        let mut bytes = Vec::new();
        let mut buffer = [0; BUFFER_SIZE];
        loop {
            if Instant::now() >= self.deadline {
                return Err(invalid("Git acquisition timed out"));
            }
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    if let Some(status) = child.try_wait()? {
                        return Ok((status, bytes));
                    }
                    thread::sleep(POLL_INTERVAL);
                }
                Ok(count) if count <= limit.saturating_sub(bytes.len()) => {
                    bytes.extend_from_slice(&buffer[..count]);
                }
                Ok(_) => return Err(invalid("Git output size limit")),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(POLL_INTERVAL);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}

fn dns_pin(repo: url::Url, deadline: Instant) -> Result<Option<String>, Error> {
    let Some(url::Host::Domain(host)) = repo.host() else {
        return Ok(None);
    };
    let host = host.to_owned();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    thread::Builder::new().spawn(move || {
        let _ = sender.send(nosey_registry::public_https_addresses(&repo));
    })?;
    let addresses = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| invalid("Git HTTPS host resolution timed out"))?
        .map_err(|error| invalid(&error))?;
    let ips: Vec<_> = addresses
        .into_iter()
        .map(|ip| match ip {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        })
        .collect();
    Ok(Some(format!(
        "http.curloptResolve={host}:443:{}",
        ips.join(",")
    )))
}
