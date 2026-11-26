use super::*;
use std::{io::Read as _, os::unix::fs::symlink, process::Command};

const HOOK_TEST_LIMIT: u64 = 1024 * 1024;
const TEST_LIMIT: u64 = 32 * 1024;
const FILE: GitProtocol = GitProtocol::File {
    staging_limit: TEST_LIMIT,
};

fn git(path: &Path, args: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .current_dir(path)
        .args(args)
        .output()?;
    assert!(output.status.success(), "{:?}", output.stderr);
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn assert_hooks_disabled(work: &Path, linked: &str) -> Result<(), Error> {
    let hook = work.join(".git/hooks/reference-transaction");
    fs::create_dir_all(
        hook.parent()
            .ok_or_else(|| invalid("fixture hook parent"))?,
    )?;
    fs::write(&hook, "#!/bin/sh\ntouch hook-ran\n")?;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700))?;
    git(work, &["update-ref", "refs/heads/control", linked])?;
    let marker = work.join("hook-ran");
    assert!(marker.is_file());
    fs::remove_file(&marker)?;
    let isolated = Git::new(
        work,
        GitProtocol::File {
            staging_limit: HOOK_TEST_LIMIT,
        },
    )?;
    isolated.run(&["update-ref", "refs/heads/isolated", linked], 0)?;
    assert!(!marker.exists());
    Ok(())
}

#[test]
fn pinned_subtree_is_verified_and_reused_offline() -> Result<(), Error> {
    if Command::new("git")
        .arg("--version")
        .output()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(());
    }
    let fixture = tempfile::tempdir()?;
    let work = fixture.path().join("work");
    let bare = fixture.path().join("bare");
    fs::create_dir(&work)?;
    fs::create_dir(&bare)?;
    git(&work, &["init", "--template="])?;
    git(&bare, &["init", "--bare", "--template="])?;
    git(&bare, &["config", "uploadpack.allowFilter", "true"])?;
    git(&bare, &["config", "uploadpack.allowAnySHA1InWant", "true"])?;
    fs::create_dir(work.join("sample"))?;
    fs::write(
        work.join("sample/SKILL.md"),
        "---\nname: sample\ndescription: Sample\n---\n",
    )?;
    let mut large_blob = vec![0; 128 * 1024];
    fs::File::open("/dev/urandom")?.read_exact(&mut large_blob)?;
    fs::write(work.join("outside"), &large_blob)?;
    git(&work, &["add", "."])?;
    let commit = [
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.com",
        "commit",
        "-m",
        "fixture",
    ];
    git(&work, &commit)?;
    let revision = git(&work, &["rev-parse", "HEAD"])?;
    symlink("../outside", work.join("sample/link"))?;
    git(&work, &["add", "."])?;
    git(&work, &commit)?;
    let linked = git(&work, &["rev-parse", "HEAD"])?;
    fs::remove_file(work.join("sample/link"))?;
    fs::write(work.join("sample/large"), &large_blob)?;
    git(&work, &["add", "."])?;
    git(&work, &commit)?;
    let oversized = git(&work, &["rev-parse", "HEAD"])?;
    assert_hooks_disabled(&work, &linked)?;
    let repo = format!("file://{}", bare.display());
    git(&work, &["push", &repo, "HEAD:refs/heads/main"])?;
    let library_dir = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
        .tempdir()?;
    let library = Library::open(&crate::State::default(), library_dir.path())?;
    let mut source = GitSource {
        artifact: "sample".into(),
        repo,
        path: "sample".into(),
        revision,
    };
    assert!(library.acquire_git(&source).is_err());
    let mut wrong = source.clone();
    wrong.revision = "1".repeat(SHA_LEN);
    assert!(library.acquire_pinned_git(&wrong, None, FILE).is_err());
    wrong.revision = linked;
    assert!(library.acquire_pinned_git(&wrong, None, FILE).is_err());
    wrong.revision = oversized;
    let oversized_result = library.acquire_pinned_git(&wrong, None, FILE);
    assert!(
        matches!(oversized_result, Err(error) if error.to_string().contains("Git staging size limit"))
    );
    assert_eq!(fs::read_dir(library.root.join("objects"))?.count(), 0);
    let record = library.acquire_pinned_git(&source, Some("sample"), FILE)?;
    assert!(
        !library
            .object_dir(&source.artifact)
            .join("skill/outside")
            .exists()
    );
    git(&bare, &["config", "uploadpack.allowFilter", "false"])?;
    let mut unfiltered = source.clone();
    unfiltered.artifact = "unfiltered".into();
    let unfiltered_result = library.acquire_pinned_git(&unfiltered, None, FILE);
    assert!(
        matches!(unfiltered_result, Err(error) if error.to_string().contains("Git staging size limit"))
    );
    assert!(!library.object_dir(&unfiltered.artifact).exists());
    fs::remove_dir_all(bare)?;
    assert_eq!(library.acquire_pinned_git(&source, None, FILE)?, record);
    source.revision = "main".into();
    assert!(library.acquire_pinned_git(&source, None, FILE).is_err());
    Ok(())
}

#[test]
fn old_git_version_reports_found_and_required() {
    let version = super::process::check_version(b"git version 2.36.9\n");
    assert!(matches!(version, Err(error) if {
        let message = error.to_string();
        message.contains("2.36.9") && message.contains("2.37")
    }));
}

#[test]
#[ignore = "requires a public HTTPS Git fixture"]
fn live_pinned_https_acquisition() -> Result<(), Error> {
    let library_dir = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
        .tempdir()?;
    let library = Library::open(&crate::State::default(), library_dir.path())?;
    let fixture = |key| std::env::var(key).map_err(|_| invalid(&format!("set {key}")));
    let source = GitSource {
        artifact: "live-fixture".into(),
        repo: fixture("NOSEY_TEST_GIT_URL")?,
        path: fixture("NOSEY_TEST_GIT_PATH")?.into(),
        revision: fixture("NOSEY_TEST_GIT_REVISION")?,
    };
    let record = library.acquire_git(&source)?;
    assert_eq!(record.acquisition, source.provenance());
    assert_eq!(library.get(&source.artifact)?, record);
    Ok(())
}
