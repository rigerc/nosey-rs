//! Restricted Agent Skill frontmatter validation for imported fixtures.

use crate::{Error, library_io::invalid};

const MAX_FRONTMATTER_BYTES: usize = 16 * 1024;

/// Read and validate the name and required metadata of a bounded skill manifest.
pub fn manifest(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("manifest is not UTF-8"))?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(invalid("missing skill frontmatter"));
    }
    let mut header = String::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        header.push_str(line);
        header.push('\n');
        if header.len() > MAX_FRONTMATTER_BYTES {
            return Err(invalid("frontmatter limit"));
        }
    }
    if !closed {
        return Err(invalid("unsupported skill frontmatter"));
    }
    nosey_registry::validate_yaml_tokens(&header).map_err(|error| invalid(&error.to_string()))?;
    let doc: serde_json::Value = serde_saphyr::from_str_with_options(
        &header,
        serde_saphyr::options! {
            duplicate_keys: serde_saphyr::options::DuplicateKeyPolicy::Error,
            merge_keys: serde_saphyr::options::MergeKeyPolicy::AsOrdinary,
            budget: serde_saphyr::budget! {
                max_documents: 1,
                max_depth: 64,
                max_anchors: 0,
                max_aliases: 0,
                max_total_scalar_bytes: MAX_FRONTMATTER_BYTES,
            },
        },
    )
    .map_err(|_| invalid("invalid skill frontmatter"))?;
    let name = doc
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid("skill name missing"))?;
    skill_name(name)?;
    let description = doc
        .get("description")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid("skill description missing"))?;
    if description.trim().is_empty()
        || description.chars().count() > 1024
        || description.chars().any(|c| c.is_control() && c != '\n')
    {
        return Err(invalid("invalid skill description"));
    }
    Ok(name.into())
}

/// Validate the manifest name used as an install path component.
pub fn skill_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('-')
        || name.ends_with('-')
        || name.contains("--")
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(invalid("invalid skill name"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn description_accepts_yaml_punctuation_as_text() {
        for description in [
            "Build & test",
            "'Build & test * ! %'",
            "|\n  Build & test\n  %YAML 1.2\n  *example !text",
        ] {
            let skill = format!("---\nname: build-test\ndescription: {description}\n---\n");
            assert!(matches!(manifest(skill.as_bytes()), Ok(name) if name == "build-test"));
        }
    }

    #[test]
    fn rejects_yaml_syntax_in_frontmatter() {
        for header in [
            "name: build\ndescription: &unused Build",
            "name: build\ndescription: *missing",
            "name: build\ndescription: !custom Build",
            "name: build\ndescription: !!str Build",
            "name: build\ndescription: Build\nmetadata: {!!str key: value}",
            "%YAML 1.2\n--- {name: build, description: Build}",
            "%TAG !x! tag:example.com,2026:\n--- {name: build, description: Build}",
            "%RESERVED data\n--- {name: build, description: Build}",
            "name: build\nname: test\ndescription: Build",
        ] {
            let skill = format!("---\n{header}\n---\n");
            assert!(manifest(skill.as_bytes()).is_err(), "{header}");
        }
    }

    #[test]
    fn preserves_frontmatter_validation() {
        for header in [
            "name: Bad-name\ndescription: Build".to_owned(),
            "name: build--test\ndescription: Build".to_owned(),
            "name: build\ndescription: true".to_owned(),
            r#"name: build
description: "Build\tTest""#
                .to_owned(),
            format!("name: build\ndescription: {}", "a".repeat(1025)),
            format!(
                "name: build\ndescription: Build\n# {}",
                "a".repeat(MAX_FRONTMATTER_BYTES)
            ),
        ] {
            let skill = format!("---\n{header}\n---\n");
            assert!(manifest(skill.as_bytes()).is_err());
        }
        assert!(manifest(b"---\nname: build\ndescription: Build\n").is_err());
    }
}
