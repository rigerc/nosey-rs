//! Isolated Git subprocesses sharing one acquisition deadline.

use super::{GitProtocol, MAX_METADATA};
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
const MINIMUM_VERSION: (u32, u32) = (2, 37);
const STAGING_LIMIT: u64 = 128 * 1024 * 1024;

pub(super) struct Git<'a> {
    repo: &'a Path,
    home: PathBuf,
    protocol: GitProtocol,
    deadline: Instant,
    pin: Option<String>,
    staging_limit: u64,
}

impl<'a> Git<'a> {
    pub(super) fn new(repo: &'a Path, protocol: GitProtocol) -> Result<Self, Error> {
        let home = repo.join("home");
        fs::create_dir(&home)?;
        let git = Self {
            repo,
            home,
            protocol,
            deadline: Instant::now() + DEADLINE,
            pin: None,
            staging_limit: match protocol {
                GitProtocol::Https => STAGING_LIMIT,
                #[cfg(test)]
                GitProtocol::File { staging_limit } => staging_limit,
            },
        };
        if matches!(protocol, GitProtocol::Https) {
            check_version(&git.run(&["--version"], VERSION_LIMIT)?)?;
        }
        Ok(git)
    }

    pub(super) const fn protocol(&self) -> GitProtocol {
        self.protocol
    }

    /// Configure the promisor remote and pin HTTPS for initial and lazy fetches.
    pub(super) fn prepare_remote(&mut self, url: &str) -> Result<(), Error> {
        if matches!(self.protocol, GitProtocol::Https) {
            let repo = url::Url::parse(url).map_err(|_| invalid("invalid Git repository URL"))?;
            self.pin = dns_pin(repo, self.deadline)?;
        }
        self.run(&["remote", "add", "origin", url], MAX_METADATA)?;
        self.run(&["config", "remote.origin.promisor", "true"], 0)?;
        Ok(())
    }

    pub(super) fn run(&self, args: &[&str], limit: usize) -> Result<Vec<u8>, Error> {
        if Instant::now() >= self.deadline {
            return Err(invalid("Git acquisition timed out"));
        }
        let (protocol, file) = match self.protocol {
            GitProtocol::Https => ("https", "protocol.file.allow=never"),
            #[cfg(test)]
            GitProtocol::File { .. } => ("file", "protocol.file.allow=always"),
        };
        let mut command = Command::new("git");
        let mut child = command
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
            .args(self.pin.iter().flat_map(|pin| ["-c", pin.as_str()]))
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
        let mut next_size_check = Instant::now();
        loop {
            if Instant::now() >= self.deadline {
                return Err(invalid("Git acquisition timed out"));
            }
            if Instant::now() >= next_size_check {
                self.check_staging_size()?;
                next_size_check = Instant::now() + POLL_INTERVAL;
            }
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    if let Some(status) = child.try_wait()? {
                        self.check_staging_size()?;
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

    fn check_staging_size(&self) -> Result<(), Error> {
        let mut remaining = self.staging_limit;
        for entry in walkdir::WalkDir::new(self.repo).follow_links(false) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error)
                    if error
                        .io_error()
                        .is_some_and(|io| io.kind() == io::ErrorKind::NotFound) =>
                {
                    continue;
                }
                Err(error) => return Err(io::Error::other(error).into()),
            };
            if entry.file_type().is_file() {
                // Git may rename a temporary pack between enumeration and stat.
                let size = match entry.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(error)
                        if error
                            .io_error()
                            .is_some_and(|io| io.kind() == io::ErrorKind::NotFound) =>
                    {
                        continue;
                    }
                    Err(error) => return Err(io::Error::other(error).into()),
                };
                remaining = remaining.checked_sub(size).ok_or_else(|| {
                    invalid(&format!(
                        "Git staging size limit ({} bytes)",
                        self.staging_limit
                    ))
                })?;
            }
        }
        Ok(())
    }
}

/// Validate the version output from the isolated Git executable.
pub(super) fn check_version(output: &[u8]) -> Result<(), Error> {
    let output = String::from_utf8_lossy(output);
    let output = output.trim();
    let found = output.strip_prefix("git version ").unwrap_or(output);
    let mut parts = found.split('.');
    let version = parts
        .next()
        .and_then(|major| major.parse().ok())
        .zip(parts.next().and_then(|minor| minor.parse().ok()));
    if version.is_some_and(|version| version >= MINIMUM_VERSION) {
        return Ok(());
    }
    let (major, minor) = MINIMUM_VERSION;
    Err(invalid(&format!(
        "HTTPS acquisition requires Git {major}.{minor} or newer (found {found:?})"
    )))
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
