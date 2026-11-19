//! One literal identity for every path the store is handed (decision 0100
//! ruling 3; #954). The bundled SQLite is built with `SQLITE_USE_URI`, so
//! any name it opens that begins `file:` is read as a URI, whatever the
//! connection's flags: `--db 'file:victim?x=1'` opened `victim`, and
//! `--to 'file:victim?x=1'` wrote a snapshot over it, outside the path that
//! was checked and shown. An absolute path never begins with `file:`, so
//! every open of a journal and every backup and restore binds its paths
//! here before any SQLite or filesystem call, and each check and each write
//! then names the same file. Messages keep the path the operator gave.

use std::path::{Path, PathBuf};

/// `path` made absolute against the current directory, without touching
/// the filesystem; an empty path is refused as its I/O error.
pub(crate) fn literal(path: &Path) -> std::io::Result<PathBuf> {
    std::path::absolute(path)
}

/// [`literal`], for a journal SQLite is about to open: a path that cannot
/// be bound, an empty one or a relative one under a working directory that
/// is gone, is refused as SQLite's own invalid path, before SQLite sees it.
pub(crate) fn journal(path: &Path) -> Result<PathBuf, rusqlite::Error> {
    literal(path)
        .ok()
        .ok_or(rusqlite::Error::InvalidPath(path.to_path_buf()))
}

/// `error` from SQLite's open of `bound`, the [`journal`] of `given`, with
/// the bound path its diagnostic names shown as `given`, so a refusal to
/// open names the path the operator gave. Any other error passes as it is.
pub(crate) fn as_given(error: rusqlite::Error, given: &Path, bound: &Path) -> rusqlite::Error {
    match error {
        rusqlite::Error::SqliteFailure(code, Some(message)) => {
            let shown = message.replace(&*bound.to_string_lossy(), &given.to_string_lossy());
            rusqlite::Error::SqliteFailure(code, Some(shown))
        }
        error => error,
    }
}

#[cfg(test)]
mod tests;
