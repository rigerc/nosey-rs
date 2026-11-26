//! Local-only skill body estimates.

use super::{Library, MAX_FILE};
use crate::{
    Error,
    library_io::{directory, invalid, read},
};
use std::path::Path;

const BYTES_PER_TOKEN: usize = 4;

impl Library {
    /// Estimate tokens as the UTF-8 body byte count divided by four, rounded up.
    ///
    /// # Errors
    /// Returns an error if the local skill cannot be verified or read.
    pub fn estimate_skill_tokens(&self, artifact: &str) -> Result<usize, Error> {
        self.get(artifact)?;
        let skill = directory(&self.object_dir(artifact).join("skill"))?;
        let bytes = read(&skill, Path::new("SKILL.md"), MAX_FILE)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| invalid("manifest is not UTF-8"))?;
        let mut lines = text.split_inclusive('\n').skip(1);
        for line in lines.by_ref() {
            if matches!(line, "---" | "---\n" | "---\r\n") {
                return Ok(lines.map(str::len).sum::<usize>().div_ceil(BYTES_PER_TOKEN));
            }
        }
        Err(invalid("missing frontmatter delimiter"))
    }
}
