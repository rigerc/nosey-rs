//! Resolved catalog bundle provenance and byte verification.

use crate::{Error, Fetch, MAX_FETCH_BYTES, SelectedSkill, transport::validate_url};
use ai_catalog::MEDIA_TYPE_AGENT_SKILLS_ZIP;
use serde::{Deserialize, Serialize};

/// A resolved installable representation from a selected registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleSource {
    /// Canonical identifier assigned by the catalog.
    pub artifact: String,
    /// Identity of the registry supplying the entry.
    pub registry: String,
    /// HTTPS representation URL.
    pub url: String,
    /// SDK media type.
    pub media_type: String,
    /// Digest declared for the representation bytes.
    pub digest: String,
    /// Catalog release version.
    pub revision: Option<String>,
}

impl BundleSource {
    /// Resolve the supported bundle representation of an approved selection.
    ///
    /// # Errors
    /// Rejects incomplete or unsupported representations.
    pub fn from_selected(selected: &SelectedSkill) -> Result<Self, Error> {
        let entry = &selected.entry;
        let source = Self {
            artifact: entry.identifier.clone(),
            registry: selected.registry.clone(),
            url: entry
                .url
                .clone()
                .ok_or_else(|| Error::Invalid("bundle URL required".into()))?,
            media_type: entry.entry_type.clone(),
            digest: entry
                .digest
                .clone()
                .ok_or_else(|| Error::Invalid("bundle digest required".into()))?,
            revision: entry.version.clone(),
        };
        source.validate()?;
        Ok(source)
    }

    /// Validate source metadata without accessing the network.
    ///
    /// # Errors
    /// Rejects unsupported media types, invalid URL and malformed metadata.
    pub fn validate(&self) -> Result<(), Error> {
        const MAX_METADATA: usize = 4096;
        if self.media_type != MEDIA_TYPE_AGENT_SKILLS_ZIP {
            return Err(Error::Invalid(
                "only remote Agent Skill ZIP bundles are supported; Git acquisition is unavailable"
                    .into(),
            ));
        }
        if [&self.artifact, &self.registry, &self.url, &self.digest]
            .into_iter()
            .chain(self.revision.as_ref())
            .any(|text| {
                text.is_empty()
                    || text.len() > MAX_METADATA
                    || text.chars().any(char::is_whitespace)
                    || text.chars().any(char::is_control)
            })
        {
            return Err(Error::Invalid("invalid bundle metadata".into()));
        }
        let url =
            url::Url::parse(&self.url).map_err(|_| Error::Invalid("invalid bundle URL".into()))?;
        validate_url(&url).map_err(Error::Invalid)?;
        ai_catalog_trust::verify_digest(&self.digest, &[])
            .map_err(|_| Error::Invalid("invalid declared bundle digest".into()))?;
        Ok(())
    }

    /// Fetch and verify the representation before the caller parses it.
    ///
    /// # Errors
    /// Rejects invalid metadata, a fetch failure, oversized bytes or a digest mismatch.
    pub fn fetch_verified(&self, fetch: &impl Fetch) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let bytes = fetch
            .get(&self.url)
            .map_err(|_| Error::Invalid("bundle fetch failed".into()))?;
        if bytes.len() > MAX_FETCH_BYTES {
            return Err(Error::TooLarge);
        }
        if !ai_catalog_trust::verify_digest(&self.digest, &bytes)
            .map_err(|_| Error::Invalid("invalid declared bundle digest".into()))?
        {
            return Err(Error::Invalid("bundle digest mismatch".into()));
        }
        Ok(bytes)
    }
}
