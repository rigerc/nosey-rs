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
    assert_eq!(
        library.audit_links(&state)?,
        [] as [nosey_core::changes::LinkAudit; 0]
    );
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

    let cleanup = library.preview_remove_unused(&state, FIXTURE_ID, true)?;
    assert_eq!(
        cleanup.references[0].connection,
        nosey_core::changes::JournalConnection::Pending
    );
    assert!(library.apply_remove_unused(&state, &cleanup).is_err());

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
    symlink(source.path(), &install.destination)?;
    let wrong_target = library.preview_recover(&install.destination)?;
    assert!(library.apply_recover(&wrong_target).is_err());
    assert_eq!(fs::read_link(&install.destination)?, source.path());
    fs::remove_file(&install.destination)?;
    symlink(&install.source, &install.destination)?;
    let active_manifest = install.source.join("SKILL.md");
    fs::write(&active_manifest, format!("{MANIFEST}tampered"))?;
    let changed = library.preview_recover(&install.destination)?;
    assert!(library.apply_recover(&changed).is_err());
    fs::write(active_manifest, MANIFEST)?;
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
    fs::remove_dir_all(&install.source)?;
    let drop_record = library.preview_recover(&install.destination)?;
    library.apply_recover(&drop_record)?;
    assert_eq!(
        library.audit_links(&state)?,
        [] as [nosey_core::changes::LinkAudit; 0]
    );
    Ok(())
}

#[test]
fn unlink_removes_one_reason_and_refuses_foreign_replacements()
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
    let manual = library.preview(&state, &root, target, FIXTURE_ID, InstallReason::Manual)?;
    library.apply(&state, &manual)?;
    let recommended = library.preview(
        &state,
        &root,
        target,
        FIXTURE_ID,
        InstallReason::Recommendation,
    )?;
    library.apply(&state, &recommended)?;
    let preview = library.preview_unlink(&manual.destination, &InstallReason::Recommendation)?;
    library.apply_unlink(&preview)?;
    assert!(library.apply_unlink(&preview).is_err());
    assert_eq!(fs::read_link(&manual.destination)?, manual.source);

    let preview = library.preview_unlink(&manual.destination, &InstallReason::Manual)?;
    let saved = root.join("original");
    fs::rename(&manual.destination, &saved)?;
    symlink(&manual.source, &manual.destination)?;
    assert!(library.apply_unlink(&preview).is_err());
    let foreign = library.preview_unlink(&manual.destination, &InstallReason::Manual)?;
    assert!(library.apply_unlink(&foreign).is_err());
    assert_eq!(fs::read_link(&manual.destination)?, manual.source);
    fs::rename(saved, &manual.destination)?;
    fs::write(manual.source.join("SKILL.md"), "corrupted")?;
    let preview = library.preview_unlink(&manual.destination, &InstallReason::Manual)?;
    library.apply_unlink(&preview)?;
    assert!(manual.destination.symlink_metadata().is_err());
    assert_eq!(
        library.audit_links(&state)?,
        [] as [nosey_core::changes::LinkAudit; 0]
    );
    Ok(())
}

#[test]
fn update_preserves_managed_link_and_refuses_stale_or_incompatible_content()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::MetadataExt;
    let central = private_tempdir()?;
    let source = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    let target = Path::new("skills");
    fs::create_dir(project.path().join(target))?;
    let manifest = source.path().join("SKILL.md");
    fs::write(&manifest, MANIFEST)?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    let original = library.import_fixture(FIXTURE_ID, source.path())?;
    let install = library.preview(&state, &root, target, FIXTURE_ID, InstallReason::Manual)?;
    library.apply(&state, &install)?;
    let inode = install.destination.symlink_metadata()?.ino();

    fs::write(&manifest, MANIFEST.replace("sample", "renamed"))?;
    assert!(library.preview_update(FIXTURE_ID, source.path()).is_err());
    fs::write(&manifest, format!("{MANIFEST}First edit\n"))?;
    let first_preview = library.preview_update(FIXTURE_ID, source.path())?;
    fs::write(&manifest, format!("{MANIFEST}Second edit\n"))?;
    assert!(library.apply_update(&first_preview).is_err());
    assert_eq!(library.get(FIXTURE_ID)?, original);
    let approved = library.preview_update(FIXTURE_ID, source.path())?;
    let updated = library.apply_update(&approved)?;
    assert_eq!(library.get(FIXTURE_ID)?, updated);
    assert_eq!(install.destination.symlink_metadata()?.ino(), inode);
    assert_eq!(
        fs::read_to_string(install.destination.join("SKILL.md"))?,
        format!("{MANIFEST}Second edit\n")
    );
    assert_eq!(library.audit_links(&state)?[0].status, Status::Managed);
    assert!(library.apply_update(&approved).is_err());
    Ok(())
}

#[test]
fn cleanup_requires_coverage_and_respects_detached_links() -> Result<(), Box<dyn std::error::Error>>
{
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
    assert_eq!(library.cleanup_report(&state)?.unused.len(), 1);
    let removed_source = library.preview_removed_source(&state, source.path())?;
    assert_eq!(removed_source.len(), 1);
    assert!(library.get(FIXTURE_ID).is_ok());
    let unknown = library.preview_remove_unused(&state, FIXTURE_ID, false)?;
    assert!(library.apply_remove_unused(&state, &unknown).is_err());
    let unused = library.preview_remove_unused(&state, FIXTURE_ID, true)?;
    assert!(unused.blocker.is_none());
    let installed = library.preview(&state, &root, target, FIXTURE_ID, InstallReason::Manual)?;
    library.apply(&state, &installed)?;
    assert!(library.apply_remove_unused(&state, &unused).is_err());
    state.projects.remove(&root);
    let detached = library.preview_remove_unused(&state, FIXTURE_ID, true)?;
    assert!(detached.references[0].detached);
    assert!(library.apply_remove_unused(&state, &detached).is_err());
    library
        .apply_unlink(&library.preview_unlink(&installed.destination, &InstallReason::Manual)?)?;
    let request = nosey_core::plan::ChangeRequest::RemoveUnusedLibrarySkill {
        artifact: FIXTURE_ID.into(),
        external_references_checked: true,
    };
    let plan = library.plan(&state, &[request])?;
    assert_eq!(plan.eligible(), 1);
    let result = library.apply_plan(&state, &plan)?;
    assert_eq!(result[0].outcome, nosey_core::plan::ChangeOutcome::Applied);
    assert!(library.get(FIXTURE_ID).is_err());
    Ok(())
}
