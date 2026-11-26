//! Passive Compose and GitHub Actions providers adapted from Nosey-9.
//! Conservative YAML subset: no aliases, anchors, explicit tags or directives.

use serde_yaml::Value;
use std::collections::BTreeSet;

fn parse(text: &str) -> Result<Value, &'static str> {
    // Deliberately reject even quoted occurrences; avoids alias expansion
    // and custom tags prior to decoding. False exclusions are incomplete.
    if text.contains(['&', '*', '!', '%']) {
        return Err("unsupported-provider-yaml");
    }
    let v: Value = serde_yaml::from_str(text).map_err(|_| "malformed-provider-yaml")?;
    if !v.is_mapping() {
        return Err("invalid-provider-yaml-root");
    }
    Ok(v)
}

/// Remove the tag and digest while preserving any registry port.
pub fn image_name(value: &str) -> &str {
    let value = value.split('@').next().unwrap_or(value);
    if let Some(i) = value.rfind(':')
        && value.rfind('/').is_none_or(|j| i > j)
    {
        return &value[..i];
    }
    value
}

/// Collect image names from a Compose manifest.
pub fn compose(text: &str) -> Result<BTreeSet<String>, &'static str> {
    let doc = parse(text)?;
    let mut out = BTreeSet::new();
    if let Some(services) = doc.get("services") {
        for (key, service) in services.as_mapping().ok_or("invalid-compose-services")? {
            if key.as_str() == Some("<<") || service.get("<<").is_some() {
                return Err("unsupported-compose-merge");
            }
            if !service.is_mapping() {
                return Err("invalid-compose-service");
            }
            if let Some(image) = service.get("image") {
                let image = image.as_str().ok_or("invalid-compose-image")?;
                if image.contains('$') {
                    return Err("dynamic-docker-image");
                }
                if !image.is_empty() {
                    out.insert(image_name(image).to_owned());
                }
            }
        }
    }
    Ok(out)
}

fn action(node: &Value, out: &mut BTreeSet<String>) -> Result<(), &'static str> {
    if node.get("<<").is_some() || !node.is_mapping() {
        return Err("invalid-action-node");
    }
    if let Some(uses) = node.get("uses") {
        let uses = uses.as_str().ok_or("invalid-action-uses")?;
        if uses.contains("${{") {
            return Err("dynamic-action-uses");
        }
        let name = uses.split('@').next().unwrap_or(uses);
        if !name.is_empty() {
            out.insert(name.to_owned());
        }
    }
    Ok(())
}

fn steps(node: Option<&Value>, out: &mut BTreeSet<String>) -> Result<(), &'static str> {
    if let Some(node) = node {
        for step in node.as_sequence().ok_or("invalid-action-steps")? {
            action(step, out)?;
        }
    }
    Ok(())
}

/// Collect action references from a GitHub Actions workflow.
pub fn actions(text: &str) -> Result<BTreeSet<String>, &'static str> {
    let doc = parse(text)?;
    let mut out = BTreeSet::new();
    if let Some(jobs) = doc.get("jobs") {
        for (key, job) in jobs.as_mapping().ok_or("invalid-action-jobs")? {
            if key.as_str() == Some("<<") {
                return Err("unsupported-action-merge");
            }
            action(job, &mut out)?;
            steps(job.get("steps"), &mut out)?;
        }
    }
    steps(doc.get("runs").and_then(|v| v.get("steps")), &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compose_normalizes_tags_and_preserves_registry_port() -> Result<(), &'static str> {
        let got = compose(
            "services:\n  app:\n    image: localhost:5000/team/app:1\n  db:\n    image: postgres@sha256:abc\n",
        )?;
        assert_eq!(
            got,
            BTreeSet::from(["localhost:5000/team/app".into(), "postgres".into()])
        );
        assert!(compose("services:\n  app:\n    image: $IMAGE").is_err());
        assert!(compose("services:\n  app: &app\n    image: x\n  db: *app").is_err());
        Ok(())
    }
    #[test]
    fn actions_supports_jobs_steps_and_composite_runs() -> Result<(), &'static str> {
        let got = actions(
            "jobs:\n  build:\n    steps:\n      - uses: actions/checkout@v4\n  reuse:\n    uses: owner/repo/.github/workflows/build.yml@main\nruns:\n  steps:\n    - uses: actions/setup-node@v4",
        )?;
        assert_eq!(
            got,
            BTreeSet::from([
                "actions/checkout".into(),
                "actions/setup-node".into(),
                "owner/repo/.github/workflows/build.yml".into()
            ])
        );
        Ok(())
    }
}
