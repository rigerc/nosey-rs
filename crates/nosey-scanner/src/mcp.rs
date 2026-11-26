//! Passive MCP package observation adapted from Nosey-9 providers.go.
//! Never execute commands, interpolate environment variables or read remote data.
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Default, Deserialize)]
#[serde(default)]
struct Document {
    #[serde(rename = "mcpServers")]
    camel: BTreeMap<String, Server>,
    mcp_servers: BTreeMap<String, Server>,
    servers: BTreeMap<String, Server>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct Server {
    command: String,
    args: Vec<String>,
    disabled: bool,
    enabled: Option<bool>,
}

/// Check whether the path names a supported MCP manifest.
pub fn applies(path: &str) -> bool {
    path.rsplit('/').next() == Some(".mcp.json")
        || path.ends_with("/mcp.json")
        || path == ".codex/config.toml"
}

fn normalize(pkg: &str) -> Result<Option<String>, &'static str> {
    if pkg.contains(['$', '=']) || pkg.contains("://") {
        return Err("dynamic-mcp-package");
    }
    if pkg.starts_with(['.', '/']) || pkg.is_empty() {
        return Ok(None);
    }
    let name = if pkg.starts_with('@') {
        pkg.rfind('@').filter(|i| *i > 0).map_or(pkg, |i| &pkg[..i])
    } else {
        pkg.split('@').next().unwrap_or(pkg)
    };
    Ok(Some(name.to_owned()))
}

fn packages(server: &Server, out: &mut BTreeSet<String>) -> Result<(), &'static str> {
    if server.disabled || server.enabled == Some(false) {
        return Ok(());
    }
    let command = Path::new(&server.command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let args = server.args.as_slice();
    let args = match command {
        "npx" | "bunx" | "uvx" => args,
        "npm" if args.first().is_some_and(|v| v == "exec") => &args[1..],
        "pnpm" | "yarn" if args.first().is_some_and(|v| v == "dlx") => &args[1..],
        "uv" if args.first().is_some_and(|v| v == "tool")
            && args.get(1).is_some_and(|v| v == "run") =>
        {
            &args[2..]
        }
        _ => return Ok(()),
    };
    let mut args = args.iter();
    let mut flagged = false;
    while let Some(arg) = args.next() {
        let value = if matches!(arg.as_str(), "-p" | "--package" | "--from") {
            flagged = true;
            args.next().ok_or("missing-mcp-package")?.as_str()
        } else if let Some(value) = arg
            .strip_prefix("--package=")
            .or_else(|| arg.strip_prefix("--from="))
        {
            flagged = true;
            value
        } else if arg.starts_with('-') {
            if !arg.contains('=')
                && !matches!(
                    arg.as_str(),
                    "-y" | "--yes" | "--no-install" | "--quiet" | "--" | "--no-progress"
                )
            {
                return Err("unknown-mcp-flag");
            }
            continue;
        } else {
            if !flagged && let Some(name) = normalize(arg)? {
                out.insert(name);
            }
            break;
        };
        if let Some(name) = normalize(value)? {
            out.insert(name);
        }
    }
    Ok(())
}

/// Collect package names from a passive MCP manifest inspection.
pub fn names(path: &str, bytes: &[u8]) -> Result<BTreeSet<String>, &'static str> {
    if bytes.len() > 65_536 {
        return Err("mcp-size-limit");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "invalid-mcp-text")?;
    let doc: Document = if path.strip_suffix(".toml").is_some() {
        toml::from_str(text).map_err(|_| "malformed-mcp-toml")?
    } else {
        serde_json::from_str(text).map_err(|_| "malformed-mcp-json")?
    };
    let mut out = BTreeSet::new();
    for server in doc
        .camel
        .values()
        .chain(doc.mcp_servers.values())
        .chain(doc.servers.values())
    {
        packages(server, &mut out)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifies_enabled_packages_without_execution() -> Result<(), &'static str> {
        let got = names(
            ".mcp.json",
            br#"{"mcpServers":{
          "a":{"command":"npx","args":["-y","@example/tool@1.2.3"]},
          "b":{"command":"uv","args":["tool","run","--from","sample@2","cli"]},
          "c":{"command":"npx","args":["ignored"],"disabled":true},
          "d":{"command":"sh","args":["-c","never execute"]}}}"#,
        )?;
        assert_eq!(
            got,
            BTreeSet::from(["@example/tool".into(), "sample".into()])
        );
        assert!(
            names(
                ".mcp.json",
                br#"{"servers":{"a":{"command":"npx","args":["--package"]}}}"#
            )
            .is_err()
        );
        Ok(())
    }
}
