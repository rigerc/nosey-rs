use super::*;
use crate::State;
use std::{cell::Cell, io::Write};
use zip::write::SimpleFileOptions;

const SKILL: &str = "---\nname: example\ndescription: Synthetic skill\n---\n";

struct MemoryFetch {
    requests: Cell<usize>,
    bytes: Vec<u8>,
}

impl Fetch for MemoryFetch {
    fn get(&self, _: &str) -> Result<Vec<u8>, String> {
        self.requests.set(self.requests.get() + 1);
        Ok(self.bytes.clone())
    }
}

fn archive(name: &str, contents: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer.start_file(name, SimpleFileOptions::default())?;
    writer.write_all(contents)?;
    Ok(writer.finish()?.into_inner())
}

fn source(bytes: &[u8]) -> BundleSource {
    BundleSource {
        artifact: "example-registry:skill.example".into(),
        registry: "example-registry".into(),
        url: "https://example.org/skill.zip".into(),
        media_type: "application/agent-skills+zip".into(),
        digest: format!("sha256:{}", hash(bytes)),
        revision: None,
    }
}

#[test]
fn verifies_and_reuses_exact_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
        .tempdir()?;
    let library = Library::open(&State::default(), central.path())?;
    let bytes = archive("SKILL.md", SKILL.as_bytes())?;
    let valid = source(&bytes);
    let fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    let mut markdown = valid.clone();
    markdown.media_type = "application/agent-skills+md".into();
    assert!(library.acquire_bundle(&markdown, &fetch).is_err());
    assert_eq!(fetch.requests.get(), 0);
    let mut mismatch = valid.clone();
    mismatch.digest = source(b"different").digest;
    assert!(library.acquire_bundle(&mismatch, &fetch).is_err());
    assert!(library.get(&valid.artifact).is_err());
    let installed = library.acquire_bundle(&valid, &fetch)?;
    assert_eq!(library.acquire_bundle(&valid, &fetch)?, installed);
    assert_eq!(fetch.requests.get(), 2);
    assert_eq!(library.get(&valid.artifact)?, installed);
    Ok(())
}

#[test]
fn rejects_unsafe_and_oversized_entries() -> Result<(), Box<dyn std::error::Error>> {
    let stage = tempfile::tempdir()?;
    for path in [
        "../escape",
        "/absolute",
        "directory/../escape",
        "C:/absolute",
    ] {
        assert!(extract(&archive(path, b"")?, stage.path()).is_err());
    }
    assert!(extract(&archive("large", &vec![0; MAX_FILE + 1])?, stage.path()).is_err());
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer.add_symlink("link", "target", SimpleFileOptions::default())?;
    assert!(extract(&writer.finish()?.into_inner(), stage.path()).is_err());
    Ok(())
}
