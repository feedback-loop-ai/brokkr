//! The store reader: one open handle checked, read and parsed once.
//!
//! The mode check and the read go through the same descriptor, so a path
//! replaced between them cannot pass one file's mode off as another's
//! contents, and a later caller can hand in a descriptor it already
//! admitted. Every cause names the store path, a line number or a binding
//! name, never a value. The `String` surfaces in the parent render these
//! causes at their own edge.

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::{valid_name, wipe, BoundSecret, Secret};

/// Why the store yielded no binding.
#[derive(Debug, thiserror::Error)]
pub(super) enum StoreError {
    #[error("cannot read secrets store {}: {source}", .path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "refusing secrets store {}: permissions {mode:03o} are broader than 0600",
        .path.display()
    )]
    BroadMode { path: PathBuf, mode: u32 },
    #[error("secrets store {} line {line} is not NAME=value", .path.display())]
    NotAssignment { path: PathBuf, line: usize },
    #[error("secrets store {} line {line} has a non-UTF-8 name", .path.display())]
    NonUtf8Name { path: PathBuf, line: usize },
    #[error("secrets store {} line {line} has an ill-formed name", .path.display())]
    IllFormedName { path: PathBuf, line: usize },
    #[error(
        "secret '{name}' is not in the store at {} (brokkr secrets set {name})",
        .path.display()
    )]
    MissingName { name: String, path: PathBuf },
}

/// Read the store behind `file`, which `path` only names in a cause.
/// Refuses a store whose permissions are broader than 0600 — ssh's
/// posture; a silent read of a world-readable file would make the
/// create-time mode meaningless.
pub(super) fn read_store_file(
    mut file: std::fs::File,
    path: &Path,
) -> Result<Vec<(String, Secret)>, StoreError> {
    let io = |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    };
    let meta = file.metadata().map_err(io)?;
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        return Err(StoreError::BroadMode {
            path: path.to_path_buf(),
            mode,
        });
    }
    // Sized from the same handle, so the buffer is not reallocated (and
    // an unwiped copy left behind) while it fills.
    let mut buf = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    let read = file.read_to_end(&mut buf).map_err(io);
    let parsed = read.and_then(|_| parse_store(&buf, path));
    wipe(&mut buf);
    parsed
}

/// The env format: `NAME=value` lines split at the first `=`, CRLF
/// tolerated, blank and `#` lines skipped, and a later assignment
/// overriding an earlier one. Values keep their raw bytes.
fn parse_store(buf: &[u8], path: &Path) -> Result<Vec<(String, Secret)>, StoreError> {
    let path = || path.to_path_buf();
    let mut entries: Vec<(String, Secret)> = Vec::new();
    for (index, line) in buf.split(|b| *b == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || line.first() == Some(&b'#') {
            continue;
        }
        let line_number = index + 1;
        let Some(eq) = line.iter().position(|b| *b == b'=') else {
            return Err(StoreError::NotAssignment {
                path: path(),
                line: line_number,
            });
        };
        let Ok(name) = std::str::from_utf8(&line[..eq]) else {
            return Err(StoreError::NonUtf8Name {
                path: path(),
                line: line_number,
            });
        };
        if !valid_name(name) {
            return Err(StoreError::IllFormedName {
                path: path(),
                line: line_number,
            });
        }
        let value = Secret::new(line[eq + 1..].to_vec());
        if let Some(existing) = entries.iter_mut().find(|(n, _)| n == name) {
            existing.1 = value;
        } else {
            entries.push((name.to_string(), value));
        }
    }
    Ok(entries)
}

/// Take each requested name out of the parsed store, in request order. A
/// name the store lacks refuses: there is no fallback to the ambient
/// environment and no empty-string binding.
pub(super) fn bind_names(
    mut entries: Vec<(String, Secret)>,
    names: &[String],
    path: &Path,
) -> Result<Vec<BoundSecret>, StoreError> {
    let mut bindings = Vec::with_capacity(names.len());
    for name in names {
        let index =
            entries
                .iter()
                .position(|(n, _)| n == name)
                .ok_or_else(|| StoreError::MissingName {
                    name: name.clone(),
                    path: path.to_path_buf(),
                })?;
        let (name, secret) = entries.swap_remove(index);
        bindings.push(BoundSecret { name, secret });
    }
    Ok(bindings)
}
