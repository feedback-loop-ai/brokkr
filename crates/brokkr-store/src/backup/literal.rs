//! A backup's destination, bound to one literal identity by the store's
//! one home for it, [`crate::literal`] (decision 0100 ruling 3; landing
//! 431L4's H1; #954), and held to the UTF-8 text SQLite names it by.

use std::path::{Path, PathBuf};

use super::BackupError;
use crate::literal::literal;

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
