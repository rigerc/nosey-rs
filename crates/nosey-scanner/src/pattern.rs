//! Bounded rule regexes. Linear-time Rust matching is preferred; the
//! fallback supports lookaround and backreferences with an operation budget.
//! This is an ECMAScript-oriented subset, not a complete regexp2 equivalence.

pub(crate) enum Pattern {
    Linear(regex::Regex),
    Fancy(fancy_regex::Regex),
}

impl Pattern {
    pub(crate) fn new(pattern: &str, insensitive: bool) -> Result<Self, &'static str> {
        if pattern.is_empty() || pattern.len() > 4096 {
            return Err("invalid-regex");
        }
        if let Ok(r) = regex::RegexBuilder::new(pattern)
            .case_insensitive(insensitive)
            .size_limit(1024 * 1024)
            .dfa_size_limit(1024 * 1024)
            .build()
        {
            return Ok(Self::Linear(r));
        }
        let r = fancy_regex::RegexBuilder::new(pattern)
            .case_insensitive(insensitive)
            .backtrack_limit(50_000)
            .delegate_size_limit(1024 * 1024)
            .delegate_dfa_size_limit(1024 * 1024)
            .build()
            .map_err(|_| "unsupported-regex")?;
        Ok(Self::Fancy(r))
    }

    pub(crate) fn matches(&self, value: &str) -> Result<bool, &'static str> {
        match self {
            Self::Linear(r) => Ok(r.is_match(value)),
            Self::Fancy(r) => {
                if value.len() > 256 * 1024 {
                    return Err("complex-regex-input-limit");
                }
                r.is_match(value).map_err(|_| "complex-regex-budget")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_common_lookaround_and_backreference_patterns() -> Result<(), &'static str> {
        let r = Pattern::new(r"^pkg-(?!test$)\w+$", false)?;
        assert!(r.matches("pkg-app")?);
        assert!(!r.matches("pkg-test")?);
        let r = Pattern::new(r"^(\w+)-\1$", true)?;
        assert!(r.matches("abc-abc")?);
        assert!(!r.matches("abc-def")?);
        assert!(Pattern::new("(", false).is_err());
        Ok(())
    }
}
