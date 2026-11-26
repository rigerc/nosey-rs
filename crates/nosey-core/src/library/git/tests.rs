use super::*;
use std::{os::unix::fs::symlink, process::Command};

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
    fs::create_dir(work.join("sample"))?;
    fs::write(
        work.join("sample/SKILL.md"),
        "---\nname: sample\ndescription: Sample\n---\n",
    )?;
    fs::write(work.join("outside"), "outside")?;
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
    let hook = work.join(".git/hooks/reference-transaction");
    fs::create_dir_all(
        hook.parent()
            .ok_or_else(|| invalid("fixture hook parent"))?,
    )?;
    fs::write(&hook, "#!/bin/sh\ntouch hook-ran\n")?;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700))?;
    git(&work, &["update-ref", "refs/heads/control", &linked])?;
    let marker = work.join("hook-ran");
    assert!(marker.is_file());
    fs::remove_file(&marker)?;
    let isolated = Git::new(&work, GitProtocol::File)?;
    isolated.run(&["update-ref", "refs/heads/isolated", &linked], 0)?;
    assert!(!marker.exists());
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
    assert!(
        library
            .acquire_pinned_git(&wrong, None, GitProtocol::File)
            .is_err()
    );
    wrong.revision = linked;
    assert!(
        library
            .acquire_pinned_git(&wrong, None, GitProtocol::File)
            .is_err()
    );
    let record = library.acquire_pinned_git(&source, Some("sample"), GitProtocol::File)?;
    assert!(
        !library
            .object_dir(&source.artifact)
            .join("skill/outside")
            .exists()
    );
    fs::remove_dir_all(bare)?;
    assert_eq!(
        library.acquire_pinned_git(&source, None, GitProtocol::File)?,
        record
    );
    source.revision = "main".into();
    assert!(
        library
            .acquire_pinned_git(&source, None, GitProtocol::File)
            .is_err()
    );
    Ok(())
}
