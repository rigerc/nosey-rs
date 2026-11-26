//! Typed, conservative version observations. Never store arbitrary source
//! URLs, Git credentials, environment values, or unrecognized version text.

use crate::{VersionKind, VersionObservation};
use std::collections::{BTreeMap, BTreeSet};

type Versions = BTreeMap<String, BTreeSet<VersionObservation>>;

/// Keep identifiers that satisfy the size and character constraints.
pub fn safe_identifier(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 256
        || value.starts_with(['.', '/'])
        || value.contains("..")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'/' | b'@'))
    {
        return None;
    }
    Some(value.to_owned())
}

fn safe_constraint(value: &str) -> Option<String> {
    let value = value.trim();
    // Narrow numeric range syntax, including common npm/Cargo/Python/Terraform
    // comparison symbols. Omit branch names, arbitrary prerelease text and URLs.
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().any(|b| b.is_ascii_digit())
        || !value.bytes().all(|b| {
            b.is_ascii_digit()
                || matches!(
                    b,
                    b'v' | b'x'
                        | b'X'
                        | b'.'
                        | b','
                        | b' '
                        | b'<'
                        | b'>'
                        | b'='
                        | b'!'
                        | b'~'
                        | b'^'
                        | b'*'
                        | b'|'
                        | b'-'
                        | b'+'
                )
        })
    {
        return None;
    }
    Some(value.to_owned())
}

fn add(out: &mut Versions, name: &str, value: &str, kind: VersionKind) {
    if let Some(value) = safe_constraint(value) {
        out.entry(name.to_owned())
            .or_default()
            .insert(VersionObservation { kind, value });
    }
}

fn python_requirement(out: &mut Versions, line: &str, normalize: bool) {
    let line = line.trim();
    if line.is_empty() || line.starts_with(['#', '-']) {
        return;
    }
    let end = line
        .find(['[', '<', '>', '=', '!', '~', ';', ' ', '@'])
        .unwrap_or(line.len());
    let name = &line[..end];
    if name.is_empty() {
        return;
    }
    let mut suffix = line[end..].trim_start();
    if suffix.starts_with('[') {
        let Some((_, rest)) = suffix.split_once(']') else {
            return;
        };
        suffix = rest.trim_start();
    }
    let suffix = suffix.split(';').next().unwrap_or(suffix).trim();
    if !suffix.starts_with(['<', '>', '=', '!', '~']) {
        return;
    }
    let name = if normalize {
        name.to_ascii_lowercase().replace('_', "-")
    } else {
        name.to_owned()
    };
    add(out, &name, suffix, VersionKind::ManifestConstraint);
}

fn toml_versions(doc: &toml::Value, ecosystem: &str, out: &mut Versions) {
    let Some(table) = doc.as_table() else {
        return;
    };
    for (key, value) in table {
        if let Some(nested) = value.as_table() {
            if matches!(
                key.as_str(),
                "dependencies"
                    | "dev-dependencies"
                    | "build-dependencies"
                    | "packages"
                    | "dev-packages"
            ) {
                for (alias, spec) in nested {
                    let name = if ecosystem == "cargo" {
                        spec.get("package")
                            .and_then(toml::Value::as_str)
                            .unwrap_or(alias)
                    } else {
                        alias
                    };
                    if let Some(constraint) = spec
                        .as_str()
                        .or_else(|| spec.get("version").and_then(toml::Value::as_str))
                    {
                        add(out, name, constraint, VersionKind::ManifestConstraint);
                    }
                }
            }
            if ecosystem == "python"
                && matches!(key.as_str(), "optional-dependencies" | "dependency-groups")
            {
                for group in nested.values().filter_map(toml::Value::as_array) {
                    for requirement in group.iter().filter_map(toml::Value::as_str) {
                        python_requirement(out, requirement, true);
                    }
                }
            }
            toml_versions(value, ecosystem, out);
        } else if ecosystem == "python"
            && matches!(key.as_str(), "dependencies" | "optional-dependencies")
            && let Some(list) = value.as_array()
        {
            for requirement in list.iter().filter_map(toml::Value::as_str) {
                python_requirement(out, requirement, true);
            }
        }
    }
}

/// Collect version declarations from a recognized manifest.
pub fn observations(ecosystem: &str, path: &str, bytes: &[u8]) -> Versions {
    let mut out = Versions::new();
    let Ok(text) = std::str::from_utf8(bytes) else {
        return out;
    };
    let base = path.rsplit('/').next().unwrap_or(path);
    match ecosystem {
        "npm" | "composer" => {
            if let Ok(doc) = serde_json::from_slice::<serde_json::Value>(bytes) {
                for key in [
                    "dependencies",
                    "devDependencies",
                    "peerDependencies",
                    "optionalDependencies",
                    "require",
                    "require-dev",
                    "imports",
                ] {
                    if let Some(table) = doc.get(key).and_then(serde_json::Value::as_object) {
                        for (name, spec) in table {
                            if let Some(value) = spec.as_str() {
                                add(&mut out, name, value, VersionKind::ManifestConstraint);
                            }
                        }
                    }
                }
            }
        }
        "cargo" | "python" if base.strip_suffix(".toml").is_some() || base == "Pipfile" => {
            if let Ok(doc) = toml::from_str::<toml::Value>(text) {
                toml_versions(&doc, ecosystem, &mut out);
            }
        }
        "python" => {
            for line in text.lines() {
                python_requirement(&mut out, line, false);
            }
        }
        "go" => {
            for line in text.lines() {
                let mut fields = line.split_whitespace();
                let Some(first) = fields.next() else {
                    continue;
                };
                let name = if first == "require" {
                    fields.next().unwrap_or("")
                } else {
                    first
                };
                if !name.contains('/') || !name.contains('.') {
                    continue;
                }
                if let Some(version) = fields.next().filter(|v| v.starts_with('v')) {
                    // go.mod records selected minimum module versions; it is
                    // not proof of a currently installed artifact.
                    add(&mut out, name, version, VersionKind::ManifestConstraint);
                }
            }
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraints_are_typed_and_arbitrary_urls_are_not_retained() {
        let result = observations("npm", "package.json",
            br#"{"dependencies":{"a":"^1.2.3","private":"https://user:secret@example.org/pkg"},"devDependencies":{"a":">=2.0"}}"#);
        assert_eq!(result.get("a").map(BTreeSet::len), Some(2));
        assert!(!result.contains_key("private"));
        assert!(safe_identifier("https://user:secret@example.org").is_none());
        assert_eq!(
            safe_identifier("@scope/package").as_deref(),
            Some("@scope/package")
        );
        let cargo = observations(
            "cargo",
            "Cargo.toml",
            b"[dependencies]\nrenamed = { package = 'actual', version = '1.2' }\n",
        );
        assert!(cargo.contains_key("actual"));
        let py = observations(
            "python",
            "requirements.txt",
            b"Foo_Bar[extra]>=1.0,<2; python_version > '3.10'\n",
        );
        assert!(py.contains_key("Foo_Bar"));
    }
}
