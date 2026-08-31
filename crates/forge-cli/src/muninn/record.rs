//! Muninn's own record: append-only NDJSON, beside the run journals and
//! inside none of them.
//!
//! Decision 0020 ruling 3 puts two demands on this file, and the module
//! surface is how they are met rather than remembered. Proposals are
//! DURABLE, so they go to a file the operator keeps, not to a buffer. And
//! proposals are never written into a run's journal, because the run
//! engine is the single writer of those — so this module holds its own
//! path and its own format, and shares no code with `forge-store` at all.
//!
//! Append-only is a property of the surface: there is one write verb, it
//! opens with `append`, and there is no update or delete verb to reach
//! for. A frozen contract would have had to change to put this anywhere
//! else; a new file never has to.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

/// Append one entry. The only write this module has. The path is named
/// once, on the open: a write that fails after the file opened has
/// nothing left to say about which file it was.
pub fn append(path: &Path, entry: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut line = serde_json::to_string(entry)?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {} to append", path.display()))?;
    Ok(file.write_all(line.as_bytes())?)
}

/// Read the record back, oldest first. A record that does not exist yet
/// is an empty record — nothing has been proposed. A line that does not
/// parse is an error naming the line, never a skipped entry: a record
/// that silently drops evidence is not a record (decision 0001).
pub fn read(path: &Path) -> Result<Vec<Value>> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(anyhow::Error::new(error).context(format!("reading {}", path.display())))
        }
    };
    let mut out = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        out.push(
            serde_json::from_str(line)
                .with_context(|| format!("reading {} line {}", path.display(), index + 1))?,
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
