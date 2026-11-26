//! Shared change plan: batch review, single approval and per-item outcomes.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
use nosey_core::{
    State,
    changes::InstallReason,
    library::Library,
    plan::{ChangeOutcome, ChangeRequest, load_requests},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A registered project, a private library and fixture skills `alpha` and `beta`.
struct Fixture {
    _dirs: Vec<tempfile::TempDir>,
    state: State,
    library: Library,
    root: PathBuf,
    sources: PathBuf,
}

fn skill(dir: &Path, name: &str, body: &str) -> std::io::Result<PathBuf> {
    let path = dir.join(name);
    fs::create_dir_all(&path)?;
    fs::write(
        path.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Fixture {name}\n---\n{body}\n"),
    )?;
    Ok(path)
}

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let sources = tempfile::tempdir()?;
    let project = tempfile::tempdir()?;
    fs::create_dir(project.path().join("skills"))?;
    let mut state = State::default();
    let root = state.register(project.path())?;
    let library = Library::open(&state, central.path())?;
    for name in ["alpha", "beta"] {
        let source = skill(sources.path(), name, "Original.")?;
        library.import_fixture(&id(name), &source)?;
    }
    let sources_path = sources.path().to_path_buf();
    Ok(Fixture {
        _dirs: vec![central, sources, project],
        state,
        library,
        root,
        sources: sources_path,
    })
}

fn id(name: &str) -> String {
    format!("urn:nosey:fixture:{name}")
}

fn install(root: &Path, name: &str, reason: InstallReason) -> ChangeRequest {
    ChangeRequest::InstallLink {
        project: root.to_path_buf(),
        target: PathBuf::from("skills"),
        artifact: id(name),
        reason,
    }
}

#[test]
fn batch_applies_eligible_changes_and_never_runs_blocked_ones() -> TestResult {
    let f = fixture()?;
    fs::create_dir(f.root.join("skills/beta"))?; // unmanaged real directory
    let requests = [
        install(&f.root, "alpha", InstallReason::Manual),
        install(&f.root, "beta", InstallReason::Manual),
    ];
    let plan = f.library.plan(&f.state, &requests)?;
    assert_eq!(plan.changes.len(), 2);
    assert!(plan.changes[0].blocker.is_none());
    assert!(plan.changes[1].blocker.is_some());

    let results = f.library.apply_plan(&f.state, &plan)?;
    assert!(matches!(results[0].outcome, ChangeOutcome::Applied));
    assert!(matches!(results[1].outcome, ChangeOutcome::Blocked { .. }));
    assert!(fs::symlink_metadata(f.root.join("skills/alpha"))?.is_symlink());
    assert!(fs::symlink_metadata(f.root.join("skills/beta"))?.is_dir());
    Ok(())
}

#[test]
fn approval_is_bound_to_the_reviewed_plan() -> TestResult {
    let f = fixture()?;
    let requests = [install(&f.root, "alpha", InstallReason::Manual)];
    let plan = f.library.plan(&f.state, &requests)?;
    fs::write(f.root.join("skills/alpha"), "appeared after review")?;
    assert!(f.library.apply_plan(&f.state, &plan).is_err());
    assert_eq!(
        fs::read_to_string(f.root.join("skills/alpha"))?,
        "appeared after review"
    );
    Ok(())
}

#[test]
fn items_invalidated_by_earlier_items_are_stale_and_others_continue() -> TestResult {
    let f = fixture()?;
    // Both reasons target the same missing destination; the second preview
    // describes a creation that the first one makes obsolete.
    let requests = [
        install(&f.root, "alpha", InstallReason::Manual),
        install(&f.root, "alpha", InstallReason::Recommendation),
        install(&f.root, "beta", InstallReason::Manual),
    ];
    let plan = f.library.plan(&f.state, &requests)?;
    let results = f.library.apply_plan(&f.state, &plan)?;
    let outcomes: Vec<_> = results.iter().map(|r| &r.outcome).collect();
    assert!(matches!(outcomes[0], ChangeOutcome::Applied));
    assert!(matches!(outcomes[1], ChangeOutcome::Stale));
    assert!(matches!(outcomes[2], ChangeOutcome::Applied));

    // Retry is a fresh plan: the remaining reason is now added to the managed link.
    let retry = f.library.plan(&f.state, &requests[1..2])?;
    let results = f.library.apply_plan(&f.state, &retry)?;
    assert!(matches!(results[0].outcome, ChangeOutcome::Applied));
    let audit = f.library.audit_links(&f.state)?;
    let alpha = audit
        .iter()
        .find(|a| a.artifact == id("alpha"))
        .ok_or("alpha link missing")?;
    assert_eq!(alpha.reasons.len(), 2);
    Ok(())
}

#[test]
fn mixed_actions_apply_in_safe_order() -> TestResult {
    let f = fixture()?;
    let first = f.library.plan(
        &f.state,
        &[install(&f.root, "alpha", InstallReason::Manual)],
    )?;
    f.library.apply_plan(&f.state, &first)?;
    let updated = skill(&f.sources.join("next"), "beta", "Updated.")?;
    let requests = [
        // Listed first, but library replacement runs after link changes.
        ChangeRequest::UpdateLibrarySkill {
            artifact: id("beta"),
            source: updated,
        },
        install(&f.root, "beta", InstallReason::Manual),
        ChangeRequest::UnlinkManaged {
            destination: f.root.join("skills/alpha"),
            reason: InstallReason::Manual,
        },
    ];
    let plan = f.library.plan(&f.state, &requests)?;
    assert!(plan.changes.iter().all(|c| c.blocker.is_none()));
    let results = f.library.apply_plan(&f.state, &plan)?;
    assert!(
        results
            .iter()
            .all(|r| matches!(r.outcome, ChangeOutcome::Applied))
    );
    assert!(fs::symlink_metadata(f.root.join("skills/alpha")).is_err());
    let beta = fs::read_to_string(f.root.join("skills/beta/SKILL.md"))?;
    assert!(beta.contains("Updated."));
    Ok(())
}

#[test]
fn duplicate_requests_collapse_and_errors_become_blockers() -> TestResult {
    let f = fixture()?;
    let request = install(&f.root, "alpha", InstallReason::Manual);
    let missing = ChangeRequest::InstallLink {
        project: f.root.clone(),
        target: PathBuf::from("../escape"),
        artifact: id("alpha"),
        reason: InstallReason::Manual,
    };
    let plan = f
        .library
        .plan(&f.state, &[request.clone(), request, missing])?;
    assert_eq!(plan.changes.len(), 2);
    assert!(plan.changes[1].blocker.is_some());
    Ok(())
}

#[test]
fn request_files_are_strict_and_bounded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("requests.json");
    fs::write(
        &path,
        r#"{"changes":[{"action":"unlink_managed","destination":"/p/skills/a","reason":{"kind":"manual"}}]}"#,
    )?;
    assert_eq!(load_requests(&path)?.len(), 1);
    fs::write(&path, r#"{"changes":[],"extra":true}"#)?;
    assert!(load_requests(&path).is_err());
    let many =
        vec![r#"{"action":"unlink_managed","destination":"/p/a","reason":{"kind":"manual"}}"#; 300];
    fs::write(&path, format!(r#"{{"changes":[{}]}}"#, many.join(",")))?;
    assert!(load_requests(&path).is_err());
    Ok(())
}
