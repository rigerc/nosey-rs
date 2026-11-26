//! Phase 4 local-fixture library and link planning vertical slice.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
use nosey_core::{
    State,
    changes::{InstallReason, Status},
    library::Library,
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};

/// The library requires a private (0700) root; `tempfile` uses the process umask.
fn private_tempdir() -> std::io::Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
}

#[test]
fn explicit_install_is_idempotent_and_unregistered_links_are_retained()
-> Result<(), Box<dyn std::error::Error>> {
    let central = private_tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    fs::create_dir(project.path().join("skills"))?;
    fs::write(
        source.path().join("SKILL.md"),
        "---\nname: sample\ndescription: Fixture skill\n---\nNo execution.\n",
    )?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    let record = library.import_fixture("urn:nosey:fixture:sample", source.path())?;
    let preview = library.preview(
        &state,
        &root,
        Path::new("skills"),
        &record.id,
        InstallReason::Manual,
    )?;
    assert_eq!(preview.status, Status::Available);
    assert!(!preview.destination.exists());
    assert_eq!(library.apply(&state, &preview)?, Status::Managed);
    assert!(library.apply(&state, &preview).is_err()); // stale
    assert_eq!(fs::read_link(&preview.destination)?, preview.source);
    let second = library.preview(
        &state,
        &root,
        Path::new("skills"),
        &record.id,
        InstallReason::Preset("one".into()),
    )?;
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
fn existing_link_is_never_adopted_and_stale_preview_never_overwrites()
-> Result<(), Box<dyn std::error::Error>> {
    let central = private_tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    fs::create_dir(project.path().join("skills"))?;
    fs::write(
        source.path().join("SKILL.md"),
        "---\nname: sample\ndescription: Fixture\n---\n",
    )?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    let skill = library.import_fixture("urn:nosey:fixture:sample", source.path())?;
    let preview = library.preview(
        &state,
        &root,
        Path::new("skills"),
        &skill.id,
        InstallReason::Manual,
    )?;
    symlink(&preview.source, &preview.destination)?;
    assert!(library.apply(&state, &preview).is_err());
    let blocked = library.preview(
        &state,
        &root,
        Path::new("skills"),
        &skill.id,
        InstallReason::Manual,
    )?;
    assert_eq!(blocked.status, Status::Occupied);
    assert!(library.apply(&state, &blocked).is_err());
    assert!(library.audit_links(&state)?.is_empty());
    assert_eq!(fs::read_link(&preview.destination)?, preview.source);
    symlink(source.path(), root.join("linked"))?;
    assert!(
        library
            .preview(
                &state,
                &root,
                Path::new("linked"),
                &skill.id,
                InstallReason::Manual
            )
            .is_err()
    );
    assert!(
        library
            .preview(
                &state,
                &root,
                Path::new("../escape"),
                &skill.id,
                InstallReason::Manual
            )
            .is_err()
    );
    Ok(())
}

const FIXTURE_ID: &str = "urn:nosey:fixture:sample";
const MANIFEST: &str = "---\nname: sample\ndescription: Fixture\n---\n";

#[test]
fn recovery_requires_fresh_approval_and_preserves_destination()
-> Result<(), Box<dyn std::error::Error>> {
    let central = private_tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let target = Path::new("skills");
    fs::create_dir(project.path().join(target))?;
    fs::write(source.path().join("SKILL.md"), MANIFEST)?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    library.import_fixture(FIXTURE_ID, source.path())?;
    let install = library.preview(&state, &root, target, FIXTURE_ID, InstallReason::Manual)?;
    library.apply(&state, &install)?;
    let journal = fs::read_dir(central.path().join("links"))?
        .next()
        .ok_or("missing journal")??
        .path();
    let mut pending: serde_json::Value = serde_json::from_slice(&fs::read(&journal)?)?;
    pending["link_identity"] = serde_json::Value::Null;
    fs::write(&journal, serde_json::to_vec(&pending)?)?;

    let confirm = library.preview_recover(&install.destination)?;
    fs::remove_file(&install.destination)?;
    assert!(library.apply_recover(&confirm).is_err());
    let drop_record = library.preview_recover(&install.destination)?;
    fs::write(&install.destination, "foreign")?;
    assert!(library.apply_recover(&drop_record).is_err());
    let blocked = library.preview_recover(&install.destination)?;
    assert!(library.apply_recover(&blocked).is_err());
    assert_eq!(fs::read_to_string(&install.destination)?, "foreign");

    fs::remove_file(&install.destination)?;
    symlink(&install.source, &install.destination)?;
    let confirm = library.preview_recover(&install.destination)?;
    library.apply_recover(&confirm)?;
    assert_eq!(fs::read_link(&install.destination)?, install.source);
    assert_eq!(library.audit_links(&state)?[0].status, Status::Managed);

    fs::rename(root.join(target), root.join("moved"))?;
    fs::create_dir(root.join(target))?;
    let moved = library.preview_recover(&install.destination)?;
    assert!(library.apply_recover(&moved).is_err());
    fs::remove_dir(root.join(target))?;
    fs::rename(root.join("moved"), root.join(target))?;
    fs::remove_file(&install.destination)?;
    let drop_record = library.preview_recover(&install.destination)?;
    library.apply_recover(&drop_record)?;
    assert!(library.audit_links(&state)?.is_empty());
    Ok(())
}
