//! JSONC comments and trailing commas, adapted from Nosey-9 providers.go.
//! Preserve quoted strings; do not allow other JSON extensions.

pub(crate) fn strip(text: &str) -> Result<Vec<u8>, &'static str> {
    let mut out = text.as_bytes().to_vec();
    let mut i = 0;
    let mut quoted = false;
    let mut escaped = false;
    while i < out.len() {
        let b = out[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else if b == b'"' {
            quoted = true;
        } else if b == b'/' && out.get(i + 1) == Some(&b'/') {
            while i < out.len() && out[i] != b'\n' {
                out[i] = b' ';
                i += 1;
            }
            continue;
        } else if b == b'/' && out.get(i + 1) == Some(&b'*') {
            out[i] = b' ';
            out[i + 1] = b' ';
            i += 2;
            loop {
                if i + 1 >= out.len() {
                    return Err("unterminated-jsonc-comment");
                }
                if out[i] == b'*' && out[i + 1] == b'/' {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                    break;
                }
                if out[i] != b'\n' {
                    out[i] = b' ';
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    if quoted {
        return Err("unterminated-jsonc-string");
    }
    quoted = false;
    escaped = false;
    for i in 0..out.len() {
        let b = out[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else if b == b'"' {
            quoted = true;
        } else if b == b',' {
            let next = out[i + 1..]
                .iter()
                .copied()
                .find(|c| !c.is_ascii_whitespace());
            if matches!(next, Some(b'}' | b']')) {
                let prev = out[..i]
                    .iter()
                    .rev()
                    .copied()
                    .find(|c| !c.is_ascii_whitespace());
                if matches!(prev, None | Some(b'{' | b'[' | b',' | b':')) {
                    return Err("invalid-jsonc-comma");
                }
                out[i] = b' ';
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_urls_and_removes_comments_and_trailing_commas()
    -> Result<(), Box<dyn std::error::Error>> {
        let bytes = strip(
            r#"{"imports": {"x": "https://example.org/a/*b*/",}, // comment
        /* block */ "other": [1,2,],}"#,
        )?;
        let v: serde_json::Value = serde_json::from_slice(&bytes)?;
        assert_eq!(v["imports"]["x"], "https://example.org/a/*b*/");
        assert!(strip("{/*").is_err());
        assert!(strip("{,}").is_err());
        Ok(())
    }
}
