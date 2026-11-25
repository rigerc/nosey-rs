//! Phase 4 local-fixture library and link planning vertical slice.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
use nosey_core::{State, library::Library, changes::{InstallReason, Status}};
use std::{fs, path::Path, os::unix::fs::symlink};

#[test]
fn explicit_install_is_idempotent_and_unregistered_links_are_retained() -> Result<(), Box<dyn std::error::Error>> {
    let central = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    fs::create_dir(project.path().join("skills"))?;
    fs::write(source.path().join("SKILL.md"), "---\nname: sample\ndescription: Fixture skill\n---\nNo execution.\n")?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    let record = library.import_fixture("urn:nosey:fixture:sample", source.path())?;
    let preview = library.preview(&state, &root, Path::new("skills"), &record.id, InstallReason::Manual)?;
    assert_eq!(preview.status, Status::Available);
    assert!(!preview.destination.exists());
    assert_eq!(library.apply(&state, &preview)?, Status::Managed);
    assert!(library.apply(&state, &preview).is_err()); // stale
    assert_eq!(fs::read_link(&preview.destination)?, preview.source);
    let second = library.preview(&state, &root, Path::new("skills"), &record.id, InstallReason::Preset("one".into()))?;
    assert_eq!(second.status, Status::Managed);
    library.apply(&state, &second)?;
    let audit = library.audit_links(&state)?;
    assert_eq!(audit[0].reasons.len(), 2);
    state.unregister(&root)?;
    assert!(library.audit_links(&state)?[0].detached);
    assert!(preview.destination.exists());
    Ok(())
}

#[test]
fn existing_link_is_never_adopted_and_stale_preview_never_overwrites() -> Result<(), Box<dyn std::error::Error>> {
    let central = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    fs::create_dir(project.path().join("skills"))?;
    fs::write(source.path().join("SKILL.md"), "---\nname: sample\ndescription: Fixture\n---\n")?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    let skill = library.import_fixture("urn:nosey:fixture:sample", source.path())?;
    let preview = library.preview(&state, &root, Path::new("skills"), &skill.id, InstallReason::Manual)?;
    symlink(&preview.source, &preview.destination)?;
    assert!(library.apply(&state, &preview).is_err());
    let blocked = library.preview(&state, &root, Path::new("skills"), &skill.id, InstallReason::Manual)?;
    assert_eq!(blocked.status, Status::Occupied);
    assert!(library.apply(&state, &blocked).is_err());
    assert!(library.audit_links(&state)?.is_empty());
    assert_eq!(fs::read_link(&preview.destination)?, preview.source);
    symlink(source.path(), root.join("linked"))?;
    assert!(library.preview(&state, &root, Path::new("linked"), &skill.id, InstallReason::Manual).is_err());
    assert!(library.preview(&state, &root, Path::new("../escape"), &skill.id, InstallReason::Manual).is_err());
    Ok(())
}
