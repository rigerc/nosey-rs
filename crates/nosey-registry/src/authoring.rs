//! Restricted, deterministic local YAML authoring adapter.
use crate::{Error, Intelligence, MAX_CATALOG_BYTES, parse_catalog};
use ai_catalog::{AiCatalog, CatalogEntry, HostInfo};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};
/// Read only regular non-symlink files and enforce byte bounds.
/// # Errors
/// Rejects symlinks, special files, excessive size and I/O errors.
pub fn read_bounded(path: &Path, max: usize) -> Result<Vec<u8>, Error> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(Error::Invalid("not a regular file".into()));
    }
    let mut result = Vec::new();
    fs::File::open(path)?
        .take(max as u64 + 1)
        .read_to_end(&mut result)?;
    if result.len() > max {
        return Err(Error::TooLarge);
    }
    Ok(result)
}
const MAX_ENTITY_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    ai_catalog_version: String,
    id: String,
    host: HostInfo,
    intelligence_extension: String,
}
fn yaml<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    let bytes = read_bounded(path, MAX_ENTITY_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::Invalid("non-UTF-8 YAML".into()))?;
    parse_yaml(text)
}

fn parse_yaml<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    crate::validate_yaml_tokens(text)?;
    serde_saphyr::from_str_with_options(
        text,
        serde_saphyr::options! {
            duplicate_keys: serde_saphyr::options::DuplicateKeyPolicy::Error,
            merge_keys: serde_saphyr::options::MergeKeyPolicy::AsOrdinary,
            budget: serde_saphyr::budget! {
                max_documents: 1,
                max_depth: 64,
                max_anchors: 0,
                max_aliases: 0,
                max_total_scalar_bytes: MAX_ENTITY_BYTES,
            },
        },
    )
    .map_err(|_| Error::Invalid("invalid YAML entity".into()))
}
fn entities<T: DeserializeOwned>(root: &Path, folder: &str) -> Result<Vec<T>, Error> {
    let dir = root.join(folder);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !fs::symlink_metadata(&dir)?.is_dir() {
        return Err(Error::Invalid(
            "entity directory is not a real directory".into(),
        ));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut pending = vec![(dir, 0_usize)];
    while let Some((dir, depth)) = pending.pop() {
        if depth > 16 || paths.len() > 4096 {
            return Err(Error::TooLarge);
        }
        for item in fs::read_dir(dir)? {
            let item = item?;
            let ty = item.file_type()?;
            if ty.is_symlink() {
                return Err(Error::Invalid("symlink in authoring tree".into()));
            }
            if ty.is_dir() {
                pending.push((item.path(), depth + 1));
            } else if ty.is_file()
                && item
                    .path()
                    .extension()
                    .is_some_and(|x| x == "yaml" || x == "yml")
            {
                paths.push(item.path());
            }
            if paths.len() + pending.len() > 4096 {
                return Err(Error::TooLarge);
            }
        }
    }
    paths.sort();
    let mut total = 0_u64;
    let mut result = Vec::new();
    for path in paths {
        total = total.saturating_add(fs::metadata(&path)?.len());
        if total > MAX_CATALOG_BYTES as u64 {
            return Err(Error::TooLarge);
        }
        result.push(yaml(&path)?);
    }
    Ok(result)
}
/// Compile local YAML entities into SDK models; validate emitted bytes before returning them.
/// This development subset uses standard SDK entry field names in skills/*.yaml.
/// # Errors
/// Rejects malformed authoring data or invalid output.
pub fn compile(root: &Path) -> Result<Vec<u8>, Error> {
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(Error::Invalid("registry root is not a directory".into()));
    }
    let m: Manifest = yaml(&root.join("registry.yaml"))?;
    if m.schema_version != "1.0.0"
        || m.ai_catalog_version != "1.0"
        || m.intelligence_extension.is_empty()
    {
        return Err(Error::Invalid(
            "unsupported manifest version or namespace".into(),
        ));
    }
    let intel = Intelligence {
        schema_version: m.schema_version,
        registry_id: m.id,
        technologies: entities(root, "technologies")?,
        rules: entities(root, "rules")?,
        mappings: entities(root, "mappings")?,
        combinations: entities(root, "combinations")?,
    };
    intel.validate()?;
    let entries: Vec<CatalogEntry> = entities(root, "skills")?;
    let mut extensions = BTreeMap::new();
    extensions.insert(
        m.intelligence_extension,
        serde_json::to_value(intel)
            .map_err(|_| Error::Invalid("extension serialization".into()))?,
    );
    let catalog = AiCatalog {
        spec_version: m.ai_catalog_version,
        host: Some(m.host),
        entries,
        signature: None,
        additional_signatures: None,
        extensions: Some(extensions),
        extra_fields: BTreeMap::new(),
    };
    let bytes = ai_catalog::to_string_pretty(&catalog)
        .map_err(|_| Error::Invalid("SDK serialization".into()))?
        .into_bytes();
    parse_catalog(&bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn yaml_accepts_scalar_punctuation_and_rejects_restricted_syntax() {
        assert!(super::parse_yaml::<serde_json::Value>("value: a && b * c ! %20").is_ok());
        for input in [
            "a: &anchor value",
            "a: *alias",
            "a: !!str value",
            "%YAML 1.2\n---\na: b",
        ] {
            assert!(
                super::parse_yaml::<serde_json::Value>(input).is_err(),
                "{input}"
            );
        }
    }
}
