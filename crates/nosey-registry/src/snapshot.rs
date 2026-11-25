//! Original-byte atomic cache. No fetch or trust escalation is performed here.
use crate::{Error, parse_catalog, read_bounded, MAX_CATALOG_BYTES};
use std::{io::Write, path::Path};
/// Validated cached document, retaining exact input bytes.
#[derive(Debug)]
pub struct Snapshot {
    /// Original bytes for future SDK trust verification.
    pub bytes: Vec<u8>,
    /// Parsed catalog.
    pub catalog: ai_catalog::AiCatalog,
}
/// Read and revalidate the previous snapshot.
/// # Errors
/// Invalid or inaccessible cache is rejected.
pub fn load_snapshot(path: &Path) -> Result<Snapshot, Error> {
    let bytes = read_bounded(path, MAX_CATALOG_BYTES)?;
    let catalog = parse_catalog(&bytes)?;
    Ok(Snapshot { bytes, catalog })
}
/// Validate before replacing the cache, using a temporary sibling and atomic rename.
/// # Errors
/// Invalid data or write failure leaves the previous snapshot untouched.
pub fn save_snapshot(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    parse_catalog(bytes)?;
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
