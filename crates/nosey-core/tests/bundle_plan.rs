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

#[path = "bundle_plan/remote_update.rs"]
mod remote_update;

const SKILL: &str = "---\nname: example\ndescription: Synthetic skill\n---\n12345";

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
        source: bundle.into(),
        install_name: Some("example".into()),
    };
    let plan = library.plan(&state, &[install.clone(), acquire.clone(), acquire])?;
    assert_eq!(plan.eligible(), 2);
    assert_eq!(plan.changes[0].estimated_tokens, None);
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
    assert_eq!(offline.changes[0].estimated_tokens, Some(2));
    assert!(project_root.join("skills/example/SKILL.md").is_file());
    assert!(project_root.join("offline/example/SKILL.md").is_file());
    Ok(())
}

#[test]
fn scoped_install_waits_for_acquisition_in_the_same_plan() -> Result<(), Box<dyn std::error::Error>>
{
    use nosey_core::{
        changes::InstallReason,
        plan::{ChangeOutcome, ChangeRequest},
        targets::InstallScope,
    };
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let project = tempfile::tempdir()?;
    let home = tempfile::tempdir()?;
    let mut state = State::default();
    let project_root = state.register(project.path())?;
    fs::create_dir_all(project_root.join(".agents/skills"))?;
    fs::create_dir_all(home.path().join(".agents/skills"))?;
    let library = Library::open(&state, central.path())?;
    let bytes = archive("SKILL.md", SKILL.as_bytes())?;
    let bundle = source(&bytes);
    let fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    let scoped = |scope| ChangeRequest::InstallScoped {
        scope,
        target: None,
        artifact: bundle.artifact.clone(),
        reason: InstallReason::Manual,
    };
    let requests = [
        scoped(InstallScope::Project {
            root: project_root.clone(),
        }),
        scoped(InstallScope::Global {
            home: home.path().to_path_buf(),
        }),
        ChangeRequest::AcquireLibrarySkill {
            source: bundle.clone().into(),
            install_name: Some("example".into()),
        },
    ];
    let plan = library.plan(&state, &requests)?;
    assert_eq!(plan.eligible(), 3);
    let results = library.apply_plan_with_fetch(&state, &plan, &fetch)?;
    assert!(results.iter().all(|r| r.outcome == ChangeOutcome::Applied));
    assert_eq!(fetch.requests.get(), 1);
    assert!(
        project_root
            .join(".agents/skills/example/SKILL.md")
            .is_file()
    );
    assert!(
        home.path()
            .join(".agents/skills/example/SKILL.md")
            .is_file()
    );
    Ok(())
}

#[test]
fn git_source_can_bind_a_pending_install_without_fetching() -> Result<(), Box<dyn std::error::Error>>
{
    use nosey_core::{changes::InstallReason, plan::ChangeRequest};

    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let project = tempfile::tempdir()?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let target = PathBuf::from("skills");
    fs::create_dir(root.join(&target))?;
    let library = Library::open(&state, central.path())?;
    let acquire = serde_json::from_value(serde_json::json!({
        "action": "acquire_library_skill",
        "install_name": "example",
        "source": {
            "artifact": "fixture:git",
            "repo": "https://example.org/repo",
            "revision": "0123456789012345678901234567890123456789",
            "path": "skills/example"
        }
    }))?;
    let plan = library.plan(
        &state,
        &[
            acquire,
            ChangeRequest::InstallLink {
                project: root,
                target,
                artifact: "fixture:git".into(),
                reason: InstallReason::Manual,
            },
        ],
    )?;
    assert_eq!(plan.eligible(), 2);
    assert!(matches!(
        plan.changes[1].detail,
        Some(nosey_core::plan::ChangeDetail::PendingInstall { .. })
    ));
    Ok(())
}

#[test]
fn catalog_advisories_cover_pending_and_managed_installs() -> Result<(), Box<dyn std::error::Error>>
{
    use nosey_core::{
        changes::InstallReason,
        plan::{ChangeOutcome, ChangeRequest, Warning},
    };

    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let project = tempfile::tempdir()?;
    let mut state = State::default();
    let project = state.register(project.path())?;
    let target = PathBuf::from("skills");
    fs::create_dir(project.join(&target))?;
    let library = Library::open(&state, central.path())?;
    let bytes = archive("SKILL.md", SKILL.as_bytes())?;
    let bundle = source(&bytes);
    let fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes: archive(
            "SKILL.md",
            SKILL.replace("name: example", "name: other").as_bytes(),
        )?,
    };
    let mut other = source(&fetch.bytes);
    other.artifact = OTHER.into();
    let index = advisory_catalog(&bundle)?;

    let install = |artifact: &str| ChangeRequest::InstallLink {
        project: project.clone(),
        target: target.clone(),
        artifact: artifact.into(),
        reason: InstallReason::Manual,
    };
    let other_install = install(OTHER);
    let acquire = ChangeRequest::AcquireLibrarySkill {
        install_name: Some("example".into()),
        source: bundle.clone().into(),
    };
    let requests = [install(&bundle.artifact), acquire];
    let plan = library.plan_with_catalog(&state, &requests, Some(&index))?;
    let missing = Warning::MissingDependency {
        reference: OTHER.into(),
    };
    let unknown = Warning::UnresolvedReference {
        reference: UNRESOLVED.into(),
    };
    let conflict = Warning::DeclaredConflict {
        reference: OTHER.into(),
    };
    assert_eq!(plan.changes[0].warnings, [missing, unknown.clone()]);
    assert_eq!(plan.eligible(), 2);
    assert_ne!(
        plan.fingerprint,
        library.plan(&state, &requests)?.fingerprint
    );
    assert!(library.apply_plan(&state, &plan).is_err());

    library.acquire_bundle(&other, &fetch)?;
    let unmanaged = project.join(&target).join("external");
    let other_preview = library.preview(&state, &project, &target, OTHER, InstallReason::Manual)?;
    std::os::unix::fs::symlink(other_preview.source, &unmanaged)?;
    let with_unmanaged = library.plan_with_catalog(&state, &requests, Some(&index))?;
    assert_eq!(
        with_unmanaged.changes[0].warnings,
        [conflict.clone(), unknown.clone()]
    );
    fs::remove_file(unmanaged)?;

    let mut batch = requests.to_vec();
    batch.push(other_install.clone());
    let together = library.plan_with_catalog(&state, &batch, Some(&index))?;
    assert_eq!(
        together.changes[0].warnings,
        [conflict.clone(), unknown.clone()]
    );
    let existing = library.plan(&state, &[other_install])?;
    assert_eq!(
        library.apply_plan(&state, &existing)?[0].outcome,
        ChangeOutcome::Applied
    );
    let with_existing = library.plan_with_catalog(&state, &requests, Some(&index))?;
    assert_eq!(with_existing.changes[0].warnings, [conflict, unknown]);
    let fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    let results =
        library.apply_plan_with_catalog_and_fetch(&state, &with_existing, Some(&index), &fetch)?;
    assert!(
        results
            .iter()
            .all(|result| result.outcome == ChangeOutcome::Applied)
    );
    Ok(())
}

const OTHER: &str = "urn:example:skill:other";
const UNRESOLVED: &str = "urn:example:skill:unknown";
const UNSUPPORTED: &str = "urn:example:skill:unsupported";

fn advisory_catalog(
    bundle: &BundleSource,
) -> Result<nosey_registry::CatalogIndex, Box<dyn std::error::Error>> {
    use nosey_registry::{CatalogIndex, RegistryInput, parse_catalog};
    use serde_json::json;

    let document = json!({
        "specVersion": "1.0",
        "entries": [
            {
                "identifier": bundle.artifact.as_str(),
                "type": "application/agent-skills+md",
                "url": "https://example.org/example/SKILL.md",
                "extensions": {"io.example.nosey.skill": {
                    "schemaVersion": "1.4.2", "installName": "example",
                    "dependencies": [OTHER, UNRESOLVED], "conflicts": [OTHER]
                }}
            },
            {
                "identifier": OTHER,
                "type": "application/agent-skills+md",
                "url": "https://example.org/other/SKILL.md",
                "extensions": {"io.example.nosey.skill": {"schemaVersion": "1.0.0", "installName": "other"}}
            },
            {
                "identifier": UNSUPPORTED,
                "type": "application/agent-skills+md",
                "url": "https://example.org/unsupported/SKILL.md",
                "extensions": {"io.example.nosey.skill": {"schemaVersion": "2.0.0"}}
            }
        ]
    });
    Ok(CatalogIndex::build(
        vec![RegistryInput {
            id: bundle.registry.clone(),
            priority: 0,
            approved: true,
            enabled: true,
            catalog: parse_catalog(&serde_json::to_vec(&document)?)?,
        }],
        "",
    )?)
}

#[test]
fn catalog_index_retains_standard_entries_and_reports_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let index = advisory_catalog(&source(&[]))?;
    assert!(
        index
            .warnings
            .iter()
            .any(|warning| warning.contains(UNRESOLVED))
    );
    assert!(
        index
            .warnings
            .iter()
            .any(|warning| warning.contains("unsupported"))
    );
    assert!(index.skills[UNSUPPORTED].extension.is_none());
    Ok(())
}
