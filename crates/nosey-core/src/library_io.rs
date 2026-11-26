//! Descriptor-relative reads and directory opens for the Linux library service.
use crate::Error;
use rustix::fs::{AtFlags, FileType, Mode, OFlags, Stat, open, openat, statat};
use std::{
    fs::File,
    io::{self, Read},
    os::fd::OwnedFd,
    path::{Component, Path, PathBuf},
};

pub fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

pub fn relative(path: &Path) -> Result<PathBuf, Error> {
    let mut result = PathBuf::new();
    let mut depth = 0;
    for component in path.components() {
        let Component::Normal(part) = component else {
            return Err(invalid("path must be a relative descendant"));
        };
        if part
            .to_str()
            .is_none_or(|s| s.is_empty() || s.chars().any(char::is_control))
        {
            return Err(invalid("invalid path component"));
        }
        depth += 1;
        if depth > 32 {
            return Err(invalid("path depth limit"));
        }
        result.push(part);
    }
    if result.as_os_str().is_empty() {
        return Err(invalid("empty relative path"));
    }
    Ok(result)
}

fn io_error(error: rustix::io::Errno) -> Error {
    io::Error::from(error).into()
}

pub fn descend(root: &OwnedFd, path: &Path) -> Result<OwnedFd, Error> {
    let path = relative(path)?;
    let mut dir = rustix::io::dup(root).map_err(io_error)?;
    for part in &path {
        dir = openat(
            &dir,
            part,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io_error)?;
    }
    Ok(dir)
}

pub fn directory(path: &Path) -> Result<OwnedFd, Error> {
    let rel = path
        .strip_prefix("/")
        .map_err(|_| invalid("absolute directory required"))?;
    let root = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io_error)?;
    if rel.as_os_str().is_empty() {
        return Ok(root);
    }
    descend(&root, rel)
}

pub fn stat(parent: &OwnedFd, leaf: &Path) -> Result<Option<Stat>, Error> {
    match statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(s) => Ok(Some(s)),
        Err(rustix::io::Errno::NOENT) => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}

/// Device and inode of a stat result; field widths differ between targets.
#[cfg_attr(
    target_pointer_width = "64",
    expect(
        clippy::unnecessary_cast,
        reason = "st_dev/st_ino are not u64 on every Linux target"
    )
)]
pub const fn stat_identity(s: &Stat) -> (u64, u64) {
    (s.st_dev as u64, s.st_ino as u64)
}

pub fn identity(fd: &OwnedFd) -> Result<(u64, u64), Error> {
    Ok(stat_identity(&rustix::fs::fstat(fd).map_err(io_error)?))
}

pub fn read(root: &OwnedFd, path: &Path, max: usize) -> Result<Vec<u8>, Error> {
    let path = relative(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| invalid("missing file parent"))?;
    let dir = if parent.as_os_str().is_empty() {
        rustix::io::dup(root).map_err(io_error)?
    } else {
        descend(root, parent)?
    };
    let leaf = Path::new(
        path.file_name()
            .ok_or_else(|| invalid("missing filename"))?,
    );
    let meta = stat(&dir, leaf)?.ok_or_else(|| invalid("file missing"))?;
    if FileType::from_raw_mode(meta.st_mode) != FileType::RegularFile {
        return Err(invalid("not a regular file"));
    }
    let fd = openat(
        &dir,
        leaf,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io_error)?;
    let file = File::from(fd);
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(invalid("invalid file type or size"));
    }
    let mut bytes = Vec::new();
    file.take(max as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max {
        return Err(invalid("file size limit"));
    }
    Ok(bytes)
}
