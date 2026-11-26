//! Acquisition transport and grouping for batch plans.

use super::ChangeRequest;
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
                .entry(source.artifact.as_str())
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
