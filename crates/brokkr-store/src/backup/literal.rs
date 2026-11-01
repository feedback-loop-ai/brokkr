//! One literal identity for every path a verb is handed (decision 0100
//! ruling 3; landing 431L4's H1). The bundled SQLite is built with
//! `SQLITE_USE_URI`, so any name it opens that begins `file:` is read as a
//! URI, whatever the connection's flags: `--to 'file:victim?x=1'` wrote the
//! snapshot over `victim`, outside the directory that was checked and
//! created. An absolute path never begins with `file:`, so each verb binds
//! its paths here before any SQLite or filesystem call, and every check
//! and every write then names the same file.

use std::path::{Path, PathBuf};

use super::BackupError;

/// `path` made absolute against the current directory, without touching
/// the filesystem; an empty path is refused as its I/O error.
pub(super) fn literal(path: &Path) -> Result<PathBuf, BackupError> {
    Ok(std::path::absolute(path)?)
}

/// [`literal`], for a backup's destination, which SQLite names by UTF-8
/// text alone: one it cannot name is refused before anything is created.
/// Asked later, macOS's filesystem refuses such a directory first, as a
/// bare I/O error.
pub(super) fn nameable(path: &Path) -> Result<PathBuf, BackupError> {
    let literal = literal(path)?;
    match literal.to_str() {
        Some(_) => Ok(literal),
        None => Err(BackupError::UnnameablePath(path.to_path_buf())),
    }
}

/// A backup's destination bound by [`nameable`] and then made ready by
/// [`super::prepare_directory`] (absent, or an empty directory), in the
/// one call `backup_through` makes before anything is written.
pub(super) fn destination(path: &Path) -> Result<PathBuf, BackupError> {
    let bound = nameable(path)?;
    super::prepare_directory(&bound)?;
    Ok(bound)
}
