//! Regression cases adapted from projectdetect's project-marker and walk behavior.
#![forbid(unsafe_code)]
use nosey_registry::{CatalogIndex, RegistryInput, compile, parse_catalog};
use nosey_scanner::{Completeness, Settings, scan};
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn index() -> Result<CatalogIndex, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/nosey");
    let mut inputs = Vec::new();
    for (id, folder) in [
        ("what-stack-reference", "what-stack-registry"),
        ("projectdetect-reference", "projectdetect-registry"),
    ] {
        let bytes = compile(&root.join(folder))?;
        assert_eq!(bytes, compile(&root.join(folder))?);
        inputs.push(RegistryInput {
            id: id.into(), priority: 0, approved: true, enabled: true,
            catalog: parse_catalog(&bytes)?,
        });
    }
    Ok(CatalogIndex::build(inputs, "org.example.nosey.intelligence")?)
}

fn write(root: &Path, relative: &str, contents: &str) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
    fs::write(path, contents)
}

#[test]
fn combines_project_markers_and_frameworks_without_duplicate_types()
    -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    for (file, contents) in [
        ("web/package.json", r#"{"dependencies":{"next":"15"}}"#),
        ("web/next.config.mjs", ""),
        ("api/PYPROJECT.TOML", ""),
        ("native/CMakeLists.txt", ""),
        ("native/build.zig", ""),
        ("infra/main.tf", ""),
        ("infra/compose.yml", "services: {}"),
        ("docs/mkdocs.yml", ""),
        ("dotnet/app.csproj", ""),
        ("dotnet/NuGet.Config", ""),
        ("dotnet/Directory.Build.props", ""),
        ("tool/Cargo.toml", "[package]\nname='fixture'\n"),
    ] { write(root.path(), file, contents)?; }
    let index = index()?;
    let result = scan(root.path(), &index, &Settings::default(), &AtomicBool::new(false))?;
    assert_eq!(result.completeness, Completeness::Complete);
    for (id, path) in [
        ("projectdetect-reference:technology.node", "web/package.json"),
        ("what-stack-reference:technology.next-js", "web/next.config.mjs"),
        ("projectdetect-reference:technology.python", "api/PYPROJECT.TOML"),
        ("projectdetect-reference:technology.cmake", "native/CMakeLists.txt"),
        ("projectdetect-reference:technology.zig", "native/build.zig"),
        ("projectdetect-reference:technology.terraform", "infra/main.tf"),
        ("projectdetect-reference:technology.docker-compose", "infra/compose.yml"),
        ("projectdetect-reference:technology.mkdocs", "docs/mkdocs.yml"),
        ("what-stack-reference:technology.dotnet", "dotnet/NuGet.Config"),
        ("what-stack-reference:technology.rust", "tool/Cargo.toml"),
    ] {
        assert!(result.technologies.get(id).is_some_and(|e| e.iter().any(|v| v.path == Path::new(path))), "{id}");
    }
    assert_eq!(result.technologies.len(), 10);
    assert_eq!(result.technologies.get("what-stack-reference:technology.dotnet").map(std::collections::BTreeSet::len), Some(3));
    Ok(())
}

#[test]
fn exclusions_do_not_prune_ambiguous_source_names_or_report_incomplete()
    -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    for path in [".terraform/main.tf", "nested/.gradle/build.zig", ".pytest_cache/Gemfile",
        "custom/mkdocs.yml", "vendor/CMakeLists.txt", "public/hugo.toml",
        "resources/Package.swift", "bin/build.sbt", "site/pelicanconf.py",
        "legit/.astro", "misc/_config.yml", "misc/example.prj", "misc/DESCRIPTION"] {
        write(root.path(), path, "")?;
    }
    let settings = Settings { exclusions: vec!["custom".into()], ..Settings::default() };
    let result = scan(root.path(), &index()?, &settings, &AtomicBool::new(false))?;
    assert_eq!(result.completeness, Completeness::Complete);
    assert!(result.diagnostics.contains("default-exclusions-applied"));
    assert!(result.diagnostics.contains("custom-exclusions-applied"));
    for id in ["terraform","zig","ruby","mkdocs","jekyll","matlab","r"] {
        assert!(!result.technologies.contains_key(&format!("projectdetect-reference:technology.{id}")));
    }
    for id in ["cmake","hugo","swift","scala-sbt","pelican"] {
        assert!(result.technologies.contains_key(&format!("projectdetect-reference:technology.{id}")));
    }
    Ok(())
}

#[test]
fn cancelled_and_depth_limited_results_are_never_complete()
    -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let index = index()?;
    let cancelled = scan(root.path(), &index, &Settings::default(), &AtomicBool::new(true))?;
    assert_eq!(cancelled.completeness, Completeness::Cancelled);
    write(root.path(), "nested/deeper/main.tf", "")?;
    let settings = Settings { max_depth: 1, ..Settings::default() };
    let partial = scan(root.path(), &index, &settings, &AtomicBool::new(false))?;
    assert_eq!(partial.completeness, Completeness::Incomplete);
    assert!(partial.diagnostics.contains("depth-limit"));
    Ok(())
}
