//! Acquisition transport and grouping for batch plans.

use super::ChangeRequest;
use crate::{
    Error,
    library::{
        AcquisitionPreview, BundleSource, GitAcquisitionPreview, GitSource, Library, SkillRecord,
    },
};
use serde::{Deserialize, Serialize};

use nosey_registry::{Fetch, SafeTransport};
use std::collections::BTreeMap;

pub(super) struct HttpsFetch;

impl Fetch for HttpsFetch {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        SafeTransport::new().map_err(|e| e.to_string())?.get(url)
    }
}

pub(super) type Acquisitions<'a> = BTreeMap<&'a str, Option<&'a ChangeRequest>>;

pub(super) fn acquisitions(requests: &[ChangeRequest]) -> Acquisitions<'_> {
    let mut acquisitions = BTreeMap::new();
    for request in requests {
        if let ChangeRequest::AcquireLibrarySkill { source, .. } = request {
            acquisitions
                .entry(source.artifact())
                .and_modify(|prior| {
                    if *prior != Some(request) {
                        *prior = None;
                    }
                })
                .or_insert(Some(request));
        }
    }
    acquisitions
}

/// Source selected for a library acquisition request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AcquisitionSource {
    /// Digest-verified catalog ZIP.
    Bundle(BundleSource),
    /// Explicitly approved repository subtree and exact commit.
    Git(GitSource),
}

impl From<BundleSource> for AcquisitionSource {
    fn from(source: BundleSource) -> Self {
        Self::Bundle(source)
    }
}

impl AcquisitionSource {
    pub(super) fn artifact(&self) -> &str {
        match self {
            Self::Bundle(source) => &source.artifact,
            Self::Git(source) => &source.artifact,
        }
    }
}

/// Observed active library state for an approved acquisition source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AcquisitionReview {
    /// Bundle review.
    Bundle(AcquisitionPreview),
    /// Git review.
    Git(GitAcquisitionPreview),
}

impl AcquisitionReview {
    pub(super) fn source(&self) -> AcquisitionSource {
        match self {
            Self::Bundle(preview) => preview.source.clone().into(),
            Self::Git(preview) => AcquisitionSource::Git(preview.source.clone()),
        }
    }

    pub(crate) const fn cached(&self) -> Option<&SkillRecord> {
        match self {
            Self::Bundle(preview) => preview.cached.as_ref(),
            Self::Git(preview) => preview.cached.as_ref(),
        }
    }
}

impl Library {
    pub(crate) fn preview_acquisition(
        &self,
        source: &AcquisitionSource,
    ) -> Result<AcquisitionReview, Error> {
        match source {
            AcquisitionSource::Bundle(source) => {
                self.preview_acquire(source).map(AcquisitionReview::Bundle)
            }
            AcquisitionSource::Git(source) => {
                self.preview_acquire_git(source).map(AcquisitionReview::Git)
            }
        }
    }

    pub(super) fn acquire_source(
        &self,
        source: AcquisitionSource,
        name: Option<&str>,
        fetch: &impl Fetch,
    ) -> Result<SkillRecord, Error> {
        match source {
            AcquisitionSource::Bundle(source) => self.acquire_named_bundle(&source, name, fetch),
            AcquisitionSource::Git(source) => self.acquire_named_git(&source, name),
        }
    }
}
