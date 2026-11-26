//! Regression fixtures inspired by what-stack, adapted to aggregate Nosey scans.
#![forbid(unsafe_code)]
use nosey_registry::{CatalogIndex, RegistryInput, compile, parse_catalog};
use nosey_scanner::{Completeness, Settings, scan};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn index() -> Result<CatalogIndex, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/nosey/what-stack-registry");
    let bytes = compile(&root)?;
    assert_eq!(bytes, compile(&root)?);
    let catalog = parse_catalog(&bytes)?;
    Ok(CatalogIndex::build(
        vec![RegistryInput {
            id: "what-stack-reference".into(),
            priority: 0,
            approved: true,
            enabled: true,
            catalog,
        }],
        "org.example.nosey.intelligence",
    )?)
}

#[test]
fn detects_multiple_nested_stacks_without_upstream_priority_suppression()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fs::create_dir_all(root.path().join("web"))?;
    fs::create_dir_all(root.path().join("api"))?;
    fs::create_dir_all(root.path().join("tools"))?;
    fs::write(root.path().join("web/next.config.mjs"), "")?;
    fs::write(root.path().join("web/vite.config.ts"), "")?;
    fs::write(
        root.path().join("web/package.json"),
        r#"{"dependencies":{"next":"15","@remix-run/react":"2"}}"#,
    )?;
    fs::write(
        root.path().join("api/requirements.txt"),
        "fastapi>=1\nflask==3\n",
    )?;
    fs::write(
        root.path().join("tools/Cargo.toml"),
        "[package]\nname='fixture'\n",
    )?;
    let result = scan(
        root.path(),
        &index()?,
        &Settings::default(),
        &AtomicBool::new(false),
    )?;
    assert_eq!(result.completeness, Completeness::Complete);
    for (tech, path) in [
        ("next-js", "web/next.config.mjs"),
        ("remix", "web/package.json"),
        ("vite", "web/vite.config.ts"),
        ("fastapi", "api/requirements.txt"),
        ("flask", "api/requirements.txt"),
        ("rust", "tools/Cargo.toml"),
    ] {
        let id = format!("what-stack-reference:technology.{tech}");
        assert!(
            result
                .technologies
                .get(&id)
                .is_some_and(|e| e.iter().any(|v| v.path == Path::new(path)))
        );
    }
    assert_eq!(result.technologies.len(), 6);
    let no_rules = scan(
        root.path(),
        &CatalogIndex::default(),
        &Settings::default(),
        &AtomicBool::new(false),
    )?;
    assert!(no_rules.technologies.is_empty());
    Ok(())
}

#[test]
fn shared_configs_and_unrelated_names_do_not_create_framework_hits()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fs::write(root.path().join("svelte.config.js"), "")?;
    fs::write(root.path().join("next.config.js.backup"), "")?;
    fs::write(
        root.path().join("package.json"),
        r#"{"scripts":{"next":"next dev"},"dependencies":{"@remix-run/router":"1","next-extra":"1"}}"#,
    )?;
    fs::write(
        root.path().join("requirements.txt"),
        "flask-login==1\npytest-django==1\n",
    )?;
    let result = scan(
        root.path(),
        &index()?,
        &Settings::default(),
        &AtomicBool::new(false),
    )?;
    assert_eq!(result.completeness, Completeness::Complete);
    assert!(result.technologies.is_empty());
    fs::write(
        root.path().join("package.json"),
        r#"{"dependencies":{"@sveltejs/kit":"2"}}"#,
    )?;
    let result = scan(
        root.path(),
        &index()?,
        &Settings::default(),
        &AtomicBool::new(false),
    )?;
    assert!(
        result
            .technologies
            .contains_key("what-stack-reference:technology.sveltekit")
    );
    Ok(())
}
