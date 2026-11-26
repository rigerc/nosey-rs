//! Token validation for restricted YAML inputs.

use crate::Error;
use serde_saphyr::granit_parser::{Scanner, StrInput, TokenType};

/// Reject explicit YAML tags and directives before decoding bounded input.
///
/// # Errors
///
/// Returns an error for forbidden tokens or invalid token syntax.
pub fn validate_yaml_tokens(text: &str) -> Result<(), Error> {
    for token in Scanner::new(StrInput::new(text)) {
        let token = token.map_err(|_| Error::Invalid("invalid YAML token".into()))?;
        if matches!(
            token.token_type(),
            TokenType::Tag(..)
                | TokenType::VersionDirective(..)
                | TokenType::TagDirective(..)
                | TokenType::ReservedDirective(..)
        ) {
            return Err(Error::Invalid("YAML tags and directives prohibited".into()));
        }
    }
    Ok(())
}
