//! AI Catalog adapter and typed Nosey intelligence; no alternative wire model.
#![forbid(unsafe_code)]
mod authoring;
/// Read-only in-memory search over catalog entities.
pub mod explore;
mod index;
mod model;
/// Explicit discovery, approval and refresh using original-byte verification.
pub mod review;
mod snapshot;
mod transport;
use ai_catalog::AiCatalog;
pub use authoring::{compile, read_bounded};
pub use index::{CatalogIndex, RegistryInput, SelectedSkill};
pub use model::{Combination, Intelligence, Mapping, Rule, RuleKind, Technology};
pub use snapshot::{Snapshot, load_snapshot, save_snapshot};
use thiserror::Error;

/// Maximum accepted catalog or authoring input in bytes.
pub const MAX_CATALOG_BYTES: usize = 16 * 1024 * 1024;
/// Registry boundary failures. Messages never contain raw input documents.
#[derive(Debug, Error)]
pub enum Error {
    /// Input or output failed.
    #[error("registry I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Malformed standard or extension document.
    #[error("invalid registry: {0}")]
    Invalid(String),
    /// Input exceeds the configured byte budget.
    #[error("registry input too large")]
    TooLarge,
}
/// Parse the original bytes strictly, then apply SDK semantic validation.
/// Validation is not signature verification and does not grant trust.
/// # Errors
/// Rejects oversized, malformed or semantically invalid catalogs.
pub fn parse_catalog(bytes: &[u8]) -> Result<AiCatalog, Error> {
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(Error::TooLarge);
    }
    ai_catalog_trust::parse_json(bytes).map_err(|_| Error::Invalid("strict JSON".into()))?;
    let catalog = ai_catalog::parse_slice(bytes).map_err(|_| Error::Invalid("SDK parse".into()))?;
    let result = ai_catalog_validate::validate(&catalog);
    if !result.is_valid {
        return Err(Error::Invalid(format!("SDK diagnostics: {result:?}")));
    }
    Ok(catalog)
}
/// Decode the explicitly configured extension namespace; unknown major versions are skipped.
/// # Errors
/// Malformed supported extensions are rejected.
pub fn intelligence(catalog: &AiCatalog, key: &str) -> Result<Option<Intelligence>, Error> {
    let Some(value) = catalog.extensions.as_ref().and_then(|m| m.get(key)) else {
        return Ok(None);
    };
    let version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::Invalid("missing schemaVersion".into()))?;
    let version =
        semver::Version::parse(version).map_err(|_| Error::Invalid("extension version".into()))?;
    if version.major != 1 {
        return Ok(None);
    }
    let data: Intelligence = serde_json::from_value(value.clone())
        .map_err(|_| Error::Invalid("intelligence model".into()))?;
    data.validate()?;
    Ok(Some(data))
}
