//! Integration coverage for acquiring and linking an artifact in one plan.
#![forbid(unsafe_code)]
#![cfg(target_os = "linux")]

use nosey_core::{
    State,
    library::{BundleSource, Library},
};
use nosey_registry::Fetch;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use std::cell::Cell;
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
        digest: format!("sha256:{:x}", Sha256::digest(bytes)),
        revision: None,
    }
}

#[test]
fn plan_acquires_once_then_installs_and_reuses_offline() -> Result<(), Box<dyn std::error::Error>> {
    use nosey_core::{
        changes::InstallReason,
        plan::{ChangeOutcome, ChangeRequest},
    };
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let project = tempfile::tempdir()?;
    let mut state = State::default();
    let project_root = state.register(project.path())?;
    let target = PathBuf::from("skills");
    fs::create_dir(project_root.join(&target))?;
    let library = Library::open(&state, central.path())?;
    let bytes = archive("SKILL.md", SKILL.as_bytes())?;
    let bundle = source(&bytes);
    let fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    let install = ChangeRequest::InstallLink {
        project: project_root.clone(),
        target,
        artifact: bundle.artifact.clone(),
        reason: InstallReason::Manual,
    };
    let acquire = ChangeRequest::AcquireLibrarySkill {
        source: bundle,
        install_name: Some("example".into()),
    };
    let plan = library.plan(&state, &[install.clone(), acquire.clone(), acquire])?;
    assert_eq!(plan.eligible(), 2);
    let results = library.apply_plan_with_fetch(&state, &plan, &fetch)?;
    assert!(results.iter().all(|r| r.outcome == ChangeOutcome::Applied));
    assert_eq!(fetch.requests.get(), 1);
    drop(library);
    let library = Library::open(&state, central.path())?;
    let offline_target = PathBuf::from("offline");
    fs::create_dir(project_root.join(&offline_target))?;
    let ChangeRequest::InstallLink { artifact, .. } = install else {
        return Err("expected install request".into());
    };
    let offline = library.plan(
        &state,
        &[ChangeRequest::InstallLink {
            project: project_root.clone(),
            target: offline_target,
            artifact,
            reason: InstallReason::Manual,
        }],
    )?;
    assert_eq!(
        library.apply_plan(&state, &offline)?[0].outcome,
        ChangeOutcome::Applied
    );
    assert!(project_root.join("skills/example/SKILL.md").is_file());
    assert!(project_root.join("offline/example/SKILL.md").is_file());
    Ok(())
}
