use super::*;

#[test]
fn loads_legacy_record() -> Result<(), Box<dyn std::error::Error>> {
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
        .tempdir()?;
    let source = tempfile::tempdir()?;
    fs::write(
        source.path().join("SKILL.md"),
        "---\nname: demo\ndescription: Fixture\n---\n",
    )?;
    let library = Library::open(&State::default(), central.path())?;
    let current = library.import_fixture("urn:nosey:fixture:legacy", source.path())?;
    fs::write(
        library.object_dir(&current.id).join("record.json"),
        serde_json::to_vec(&serde_json::json!({
            "id": current.id,
            "name": current.name,
            "source": current.source,
        }))?,
    )?;
    assert_eq!(library.get(&current.id)?, current);
    Ok(())
}

#[test]
fn imports_once_and_rejects_mutation_or_symlink() -> Result<(), Box<dyn std::error::Error>> {
    let central = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()?;
    let source = tempfile::tempdir()?;
    fs::write(
        source.path().join("SKILL.md"),
        "---\nname: demo\ndescription: Fixture\n---\nDo not execute.\n",
    )?;
    let library = Library::open(&State::default(), central.path())?;
    let first = library.import_fixture("urn:nosey:fixture:demo", source.path())?;
    assert_eq!(library.import_fixture(&first.id, source.path())?, first);
    assert_eq!(
        fs::metadata(library.object_dir(&first.id))?
            .permissions()
            .mode()
            & 0o777,
        PRIVATE_MODE
    );
    fs::write(source.path().join("script.sh"), "new")?;
    assert!(library.import_fixture(&first.id, source.path()).is_err());
    std::os::unix::fs::symlink("script.sh", source.path().join("link"))?;
    assert!(
        library
            .import_fixture("urn:nosey:fixture:other", source.path())
            .is_err()
    );
    assert_eq!(library.get(&first.id)?, first);
    assert!(Library::open(&State::default(), central.path()).is_err());
    Ok(())
}
