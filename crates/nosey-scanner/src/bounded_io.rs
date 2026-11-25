// SPDX-License-Identifier: MIT
//! Bounded file reads adapted from what-stack v0.1.1, src/config/files.rs.
//! Upstream: https://github.com/ehsan18t/what-stack/tree/aa792571b5829b50dda71644857532dd80e302b8
//! Nosey additionally anchors each open to the registered directory and
//! refuses symlinks, truncation and nonregular handles. No lossy decoding.

use std::{fs::File, io::{self, Read}, path::Path};

pub(crate) struct RootReader {
    #[cfg(unix)]
    root: std::os::fd::OwnedFd,
}

impl RootReader {
    pub(crate) fn new(root: &Path) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use rustix::fs::{Mode, OFlags, open};
            let root = open(root, OFlags::RDONLY | OFlags::DIRECTORY |
                OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty())?;
            Ok(Self { root })
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            Err(io::Error::new(io::ErrorKind::Unsupported, "safe scanner file opens require Unix"))
        }
    }

    #[cfg(unix)]
    fn open(&self, relative: &Path) -> io::Result<File> {
        use rustix::fs::{Mode, OFlags, openat};
        use std::path::Component;
        let mut components = relative.components().peekable();
        let mut parent = rustix::io::dup(&self.root)?;
        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid relative path"));
            };
            if components.peek().is_some() {
                parent = openat(&parent, name, OFlags::RDONLY | OFlags::DIRECTORY |
                    OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty())?;
            } else {
                // Nonblocking prevents a substituted FIFO from blocking the
                // scan. Verify type on this exact handle before reading.
                let fd = openat(&parent, name, OFlags::RDONLY | OFlags::NOFOLLOW |
                    OFlags::NONBLOCK | OFlags::CLOEXEC, Mode::empty())?;
                return Ok(File::from(fd));
            }
        }
        Err(io::Error::new(io::ErrorKind::InvalidInput, "empty relative path"))
    }

    #[cfg(not(unix))]
    fn open(&self, _relative: &Path) -> io::Result<File> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "safe scanner file opens require Unix"))
    }

    pub(crate) fn read(&self, relative: &Path, max: usize) -> io::Result<Vec<u8>> {
        let file = self.open(relative)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > max as u64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "nonregular or oversized file"));
        }
        let mut bytes = Vec::new();
        file.take((max as u64).saturating_add(1)).read_to_end(&mut bytes)?;
        if bytes.len() > max {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "file size limit"));
        }
        Ok(bytes)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::symlink};

    #[test]
    fn rejects_escaping_paths_linked_ancestors_and_final_symlinks()
        -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        fs::create_dir(root.path().join("nested"))?;
        fs::write(root.path().join("nested/ok"), "abcd")?;
        fs::write(outside.path().join("secret"), "unread")?;
        symlink(outside.path(), root.path().join("linked"))?;
        symlink(outside.path().join("secret"), root.path().join("leaf"))?;
        let reader = RootReader::new(root.path())?;
        assert_eq!(reader.read(Path::new("nested/ok"), 4)?, b"abcd");
        assert!(reader.read(Path::new("nested/ok"), 3).is_err());
        for path in ["linked/secret", "leaf", "../secret", "/etc/passwd", "nested"] {
            assert!(reader.read(Path::new(path), 100).is_err());
        }
        Ok(())
    }

    #[test]
    fn fifo_is_rejected_without_blocking() -> Result<(), Box<dyn std::error::Error>> {
        use rustix::fs::{Mode, mknodat, FileType};
        let root = tempfile::tempdir()?;
        let reader = RootReader::new(root.path())?;
        mknodat(&reader.root, "pipe", FileType::Fifo, Mode::RUSR | Mode::WUSR, 0)?;
        assert!(reader.read(Path::new("pipe"), 32).is_err());
        Ok(())
    }
}
