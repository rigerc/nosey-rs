//! Compiled rule predicates; unsupported mechanisms fail closed.
use crate::pattern::Pattern;
use crate::{MatchKind, VersionObservation, providers, versions};
use globset::{GlobBuilder, GlobMatcher};
use nosey_registry::RuleKind;

/// A matched identifier and its optional version evidence.
pub struct Observation {
    pub(crate) kind: MatchKind,
    pub(crate) identifier: Option<String>,
    pub(crate) version: Option<VersionObservation>,
}
impl Observation {
    const fn simple(kind: MatchKind) -> Self {
        Self {
            kind,
            identifier: None,
            version: None,
        }
    }
}

fn regex(pattern: &str, insensitive: bool) -> Result<Pattern, &'static str> {
    Pattern::new(pattern, insensitive)
}
fn glob(pattern: &str) -> Result<GlobMatcher, &'static str> {
    if pattern.is_empty() || pattern.len() > 4096 {
        return Err("invalid-glob");
    }
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map(|g| g.compile_matcher())
        .map_err(|_| "invalid-glob")
}
/// A literal or regular expression used to match names.
pub enum Name {
    Literal(String),
    Pattern(Pattern),
}
impl Name {
    fn new(lit: &str, pat: &str) -> Result<Self, &'static str> {
        if !pat.is_empty() {
            Ok(Self::Pattern(regex(pat, false)?))
        } else if !lit.is_empty() {
            Ok(Self::Literal(lit.into()))
        } else {
            Err("empty-name")
        }
    }
    fn matches(&self, value: &str) -> Result<bool, &'static str> {
        match self {
            Self::Literal(s) => Ok(s == value),
            Self::Pattern(r) => r.matches(value),
        }
    }
}
/// A compiled registry rule ready to evaluate against a project path.
pub enum Matcher {
    FileGlob(GlobMatcher),
    FileRegex(Pattern, bool),
    Extension(String),
    Content(GlobMatcher, Name),
    Dotenv(Option<GlobMatcher>, Name),
    Dependency(String, Name),
}
impl Matcher {
    pub(crate) fn new(rule: &RuleKind, mcp: bool) -> Result<Self, &'static str> {
        match rule {
            RuleKind::File {
                glob: g,
                path_regex,
                path_scope,
            } => {
                if g.is_empty() {
                    Ok(Self::FileRegex(
                        regex(path_regex, false)?,
                        path_scope == "basename",
                    ))
                } else {
                    Ok(Self::FileGlob(glob(g)?))
                }
            }
            RuleKind::Extension { extension } if !extension.is_empty() => {
                Ok(Self::Extension(extension.clone()))
            }
            RuleKind::Content {
                glob: g,
                contains,
                regex: r,
                regex_flags,
            } => {
                let name = if contains.is_empty() {
                    if !regex_flags.is_empty() && regex_flags != "i" {
                        return Err("unsupported-regex-flags");
                    }
                    Name::Pattern(regex(r, regex_flags == "i")?)
                } else {
                    Name::Literal(contains.clone())
                };
                Ok(Self::Content(glob(g)?, name))
            }
            RuleKind::Dotenv {
                glob: g,
                key,
                key_regex,
            } => Ok(Self::Dotenv(
                if g.is_empty() { None } else { Some(glob(g)?) },
                Name::new(key, key_regex)?,
            )),
            RuleKind::Dependency {
                ecosystem,
                name,
                name_regex,
            } => {
                if !providers::supported(ecosystem) {
                    return Err("unsupported-provider");
                }
                Ok(Self::Dependency(
                    ecosystem.clone(),
                    Name::new(name, name_regex)?,
                ))
            }
            RuleKind::McpPackage { .. } if !mcp => Err("mcp-disabled"),
            RuleKind::McpPackage { name, name_regex } => {
                Ok(Self::Dependency("mcp".into(), Name::new(name, name_regex)?))
            }
            RuleKind::Detector { .. } => Err("external-detector-unavailable"),
            _ => Err("unsupported-rule"),
        }
    }
    fn base(path: &str) -> &str {
        path.rsplit('/').next().unwrap_or(path)
    }
    fn dotenv(g: Option<&GlobMatcher>, path: &str) -> bool {
        g.map_or_else(
            || {
                let b = Self::base(path);
                b.starts_with(".env") && !b.contains("example") && !b.contains("sample")
            },
            |x| x.is_match(path),
        )
    }
    pub(crate) fn needs_bytes(&self, path: &str) -> bool {
        match self {
            Self::Content(g, _) => g.is_match(path),
            Self::Dotenv(g, _) => Self::dotenv(g.as_ref(), path),
            Self::Dependency(e, _) => providers::applies(e, path),
            _ => false,
        }
    }
    pub(crate) fn matches(
        &self,
        path: &str,
        is_dir: bool,
        bytes: Option<&[u8]>,
    ) -> Result<Vec<Observation>, &'static str> {
        let hit = |yes, kind| {
            if yes {
                vec![Observation::simple(kind)]
            } else {
                Vec::new()
            }
        };
        match self {
            Self::FileGlob(g) => Ok(hit(g.is_match(path), MatchKind::File)),
            Self::FileRegex(r, b) => Ok(hit(
                r.matches(if *b { Self::base(path) } else { path })?,
                MatchKind::File,
            )),
            _ if is_dir => Ok(Vec::new()),
            Self::Extension(s) => Ok(hit(path.ends_with(s.as_str()), MatchKind::Extension)),
            Self::Content(g, n) if g.is_match(path) => {
                let text = std::str::from_utf8(bytes.ok_or("missing-file-data")?)
                    .map_err(|_| "invalid-text")?;
                Ok(hit(
                    match n {
                        Name::Literal(s) => text.contains(s.as_str()),
                        Name::Pattern(r) => r.matches(text)?,
                    },
                    MatchKind::Content,
                ))
            }
            Self::Dotenv(g, n) if Self::dotenv(g.as_ref(), path) => {
                let text = std::str::from_utf8(bytes.ok_or("missing-file-data")?)
                    .map_err(|_| "invalid-text")?;
                let mut ids = Vec::new();
                for line in text.lines() {
                    let t = line.trim();
                    let t = t.strip_prefix("export ").unwrap_or(t);
                    if let Some((k, _)) = t.split_once('=') {
                        let k = k.trim();
                        if !k.is_empty()
                            && k.as_bytes()
                                .first()
                                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                            && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                            && n.matches(k)?
                        {
                            ids.push(Observation::simple(MatchKind::Dotenv));
                        }
                    }
                }
                Ok(ids)
            }
            Self::Dependency(e, n) if providers::applies(e, path) => {
                let bytes = bytes.ok_or("missing-file-data")?;
                let names = providers::names(e, path, bytes)?;
                let constraints = versions::observations(e, path, bytes);
                let kind = if e == "mcp" {
                    MatchKind::McpPackage
                } else {
                    MatchKind::Dependency
                };
                let mut ids = Vec::new();
                for name in names {
                    if n.matches(&name)? {
                        let identifier = versions::safe_identifier(&name);
                        if let Some(values) = constraints.get(&name) {
                            for value in values {
                                ids.push(Observation {
                                    kind,
                                    identifier: identifier.clone(),
                                    version: Some(value.clone()),
                                });
                            }
                        } else {
                            ids.push(Observation {
                                kind,
                                identifier,
                                version: None,
                            });
                        }
                    }
                }
                Ok(ids)
            }
            _ => Ok(Vec::new()),
        }
    }
}
