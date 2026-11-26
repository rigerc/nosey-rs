//! HCL provider and resource detection from the pinned Nosey-9 grammar.
//! Inspect the syntax tree only: never evaluate expressions, run Terraform,
//! expand variables or resolve external modules.

use hcl::{Expression, ObjectKey};
use std::collections::BTreeSet;

const MAX_HCL_BYTES: usize = 512 * 1024;

fn key_text(key: &ObjectKey) -> Result<&str, &'static str> {
    match key {
        ObjectKey::Identifier(id) => Ok(id.as_str()),
        ObjectKey::Expression(Expression::String(s)) => Ok(s.as_str()),
        _ => Err("dynamic-terraform-key"),
    }
}

fn provider(local: &str, expr: &Expression) -> Result<String, &'static str> {
    let fallback = || format!("registry.terraform.io/hashicorp/{local}");
    match expr {
        Expression::String(_) => Ok(fallback()),
        Expression::Object(entries) => {
            let mut source = None;
            for (key, value) in entries {
                if key_text(key)? != "source" {
                    continue;
                }
                let Expression::String(s) = value else {
                    return Err("dynamic-terraform-source");
                };
                if source.replace(s.as_str()).is_some() {
                    return Err("duplicate-terraform-source");
                }
            }
            let Some(source) = source else {
                return Ok(fallback());
            };
            if source.is_empty() || source.contains(['$', '%']) {
                return Err("invalid-terraform-source");
            }
            if source.matches('/').count() == 1 {
                Ok(format!("registry.terraform.io/{source}"))
            } else {
                Ok(source.to_owned())
            }
        }
        _ => Err("unsupported-terraform-expression"),
    }
}

/// Extract Terraform provider or resource names from an HCL document.
pub fn names(ecosystem: &str, text: &str) -> Result<BTreeSet<String>, &'static str> {
    if text.len() > MAX_HCL_BYTES {
        return Err("terraform-size-limit");
    }
    let body = hcl::parse(text).map_err(|_| "malformed-terraform-hcl")?;
    let mut out = BTreeSet::new();
    for block in body.blocks() {
        if ecosystem == "terraform.resource" {
            if block.identifier() == "resource"
                && let Some(label) = block.labels().first()
            {
                out.insert(label.as_str().to_owned());
            }
        } else if block.identifier() == "terraform" {
            for required in block
                .body()
                .blocks()
                .filter(|b| b.identifier() == "required_providers")
            {
                for attr in required.body().attributes() {
                    out.insert(provider(attr.key(), attr.expr())?);
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_sources_and_resources_are_separate() -> Result<(), &'static str> {
        let input = r#"
terraform {
  required_providers {
    aws = { source = "hashicorp/aws", version = "~> 5.0" }
    custom = { source = "example.org/team/provider" }
    local = { version = "2.0" }
    old = "~> 1.0"
  }
}
resource "aws_s3_bucket" "example" {}
resource "custom_thing" "example" {}
# resource "commented" "not_real" {}
"#;
        assert_eq!(
            names("terraform", input)?,
            BTreeSet::from([
                "registry.terraform.io/hashicorp/aws".into(),
                "example.org/team/provider".into(),
                "registry.terraform.io/hashicorp/local".into(),
                "registry.terraform.io/hashicorp/old".into(),
            ])
        );
        assert_eq!(
            names("terraform.resource", input)?,
            BTreeSet::from(["aws_s3_bucket".into(), "custom_thing".into()])
        );
        Ok(())
    }

    #[test]
    fn dynamic_sources_and_malformed_hcl_are_incomplete() {
        assert!(
            names(
                "terraform",
                "terraform { required_providers { aws = var.source } }"
            )
            .is_err()
        );
        assert!(names("terraform.resource", "resource {").is_err());
    }
}
