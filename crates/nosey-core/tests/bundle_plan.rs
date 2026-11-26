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
        source: bundle.into(),
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

fn catalog(
    source: &BundleSource,
    include: bool,
) -> Result<nosey_registry::CatalogIndex, Box<dyn std::error::Error>> {
    let entries = if include {
        vec![serde_json::json!({
            "identifier": source.artifact,
            "type": source.media_type,
            "url": source.url,
            "digest": source.digest,
            "version": source.revision,
        })]
    } else {
        Vec::new()
    };
    let document = serde_json::json!({
        "specVersion": "1.0",
        "host": {"displayName": "Fixture", "identifier": "example.org"},
        "entries": entries,
    });
    Ok(nosey_registry::CatalogIndex::build(
        vec![nosey_registry::RegistryInput {
            id: source.registry.clone(),
            priority: 0,
            approved: true,
            enabled: true,
            catalog: serde_json::from_value(document)?,
        }],
        "",
    )?)
}

#[test]
fn remote_update_preserves_links_and_requires_review_for_source_changes()
-> Result<(), Box<dyn std::error::Error>> {
    use nosey_core::{
        UpdatePolicy,
        changes::InstallReason,
        library::AutomaticUpdateOutcome,
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
    let original = source(&bytes);
    let old_fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    library.acquire_bundle(&original, &old_fetch)?;
    let install = library.preview(
        &state,
        &project_root,
        &target,
        &original.artifact,
        InstallReason::Manual,
    )?;
    library.apply(&state, &install)?;
    let link = project_root.join("skills/example");
    let destination = fs::read_link(&link)?;

    let new_skill = format!("{SKILL}\nReplacement");
    let bytes = archive("SKILL.md", new_skill.as_bytes())?;
    let mut replacement = source(&bytes);
    replacement.url = "https://example.org/v2.zip".into();
    let new_fetch = MemoryFetch {
        requests: Cell::new(0),
        bytes,
    };
    let index = catalog(&replacement, true)?;
    let updates = library.available_updates(&index)?;
    assert_eq!(updates.len(), 1);
    assert!(!updates[0].provenance_changed);
    let plan = library.plan(
        &state,
        &[ChangeRequest::UpdateRemoteLibrarySkill {
            source: replacement.clone(),
        }],
    )?;
    assert!(matches!(
        library.apply_plan_with_fetch(&state, &plan, &old_fetch)?[0].outcome,
        ChangeOutcome::Failed { .. }
    ));
    assert_eq!(fs::read_to_string(link.join("SKILL.md"))?, SKILL);

    assert!(
        library
            .apply_automatic_updates(&state, &index, &new_fetch)?
            .is_empty()
    );
    state.update_policy = UpdatePolicy::Automatic;
    let mut switched = replacement.clone();
    switched.registry = "alternate".into();
    switched.url = "https://other.example.org/skill.zip".into();
    let switched_index = catalog(&switched, true)?;
    assert!(matches!(
        library.apply_automatic_updates(&state, &switched_index, &new_fetch)?[0].1,
        AutomaticUpdateOutcome::ReviewRequired
    ));
    assert_eq!(new_fetch.requests.get(), 0);
    assert!(matches!(
        library.apply_automatic_updates(&state, &index, &new_fetch)?[0].1,
        AutomaticUpdateOutcome::Applied
    ));
    assert_eq!(fs::read_link(&link)?, destination);
    assert_eq!(fs::read_to_string(link.join("SKILL.md"))?, new_skill);
    assert!(library.available_updates(&index)?.is_empty());
    assert_availability(&library, &index, &switched_index, &replacement)?;
    approve_priority_switch(
        &library,
        &state,
        &project_root,
        &replacement,
        &new_fetch,
        &new_skill,
    )
}

fn assert_availability(
    library: &Library,
    index: &nosey_registry::CatalogIndex,
    switched_index: &nosey_registry::CatalogIndex,
    replacement: &BundleSource,
) -> Result<(), Box<dyn std::error::Error>> {
    use nosey_core::library::Availability;
    assert_eq!(
        library.refresh_availability(index)?[0].availability,
        Availability::Available
    );
    assert_eq!(
        library.refresh_availability(&catalog(replacement, false)?)?[0].availability,
        Availability::MissingFromCatalog
    );
    assert_eq!(
        library.refresh_availability(switched_index)?[0].availability,
        Availability::RegistryRemoved
    );
    assert_eq!(
        library.get(&replacement.artifact)?.availability,
        Availability::Available
    );

    Ok(())
}

fn approve_priority_switch(
    library: &Library,
    state: &State,
    project_root: &std::path::Path,
    replacement: &BundleSource,
    new_fetch: &MemoryFetch,
    new_skill: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use nosey_core::{
        changes::InstallReason,
        library::{AutomaticUpdateOutcome, Availability},
        plan::{ChangeOutcome, ChangeRequest},
    };
    let index = catalog(replacement, true)?;
    let mut switched = replacement.clone();
    switched.registry = "alternate".into();
    // The same declaration becoming the priority winner is still a source change.
    let mut prioritized = catalog(&switched, true)?;
    prioritized.registries.insert(replacement.registry.clone());
    prioritized
        .skills
        .get_mut(&replacement.artifact)
        .ok_or("missing bundle")?
        .alternatives
        .push((
            replacement.registry.clone(),
            index.skills[&replacement.artifact].entry.clone(),
        ));
    assert_eq!(
        library.refresh_availability(&prioritized)?[0].availability,
        Availability::Available
    );
    assert!(matches!(
        library.apply_automatic_updates(state, &prioritized, new_fetch)?[0].1,
        AutomaticUpdateOutcome::ReviewRequired
    ));
    let target = PathBuf::from("second-target");
    fs::create_dir(project_root.join(&target))?;
    let manual_plan = library.plan(
        state,
        &[
            ChangeRequest::UpdateRemoteLibrarySkill {
                source: switched.clone(),
            },
            ChangeRequest::InstallLink {
                project: project_root.to_path_buf(),
                target,
                artifact: switched.artifact.clone(),
                reason: InstallReason::Manual,
            },
        ],
    )?;
    assert!(
        library
            .apply_plan_with_fetch(state, &manual_plan, new_fetch)?
            .iter()
            .all(|item| item.outcome == ChangeOutcome::Applied)
    );
    assert_eq!(
        library.get(&switched.artifact)?.registry.as_deref(),
        Some(switched.registry.as_str())
    );
    assert_eq!(
        fs::read_to_string(project_root.join("second-target/example/SKILL.md"))?,
        new_skill
    );
    assert!(
        library
            .apply_plan_with_fetch(state, &manual_plan, new_fetch)
            .is_err()
    );
    Ok(())
}

#[test]
fn old_state_uses_notify_policy() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("state.json");
    fs::write(&path, r#"{"projects":{}}"#)?;
    assert_eq!(
        State::load(&path)?.update_policy,
        nosey_core::UpdatePolicy::Notify
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
