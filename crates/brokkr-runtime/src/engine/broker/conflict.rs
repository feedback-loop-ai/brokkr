//! What keeps protected evidence or the coordination root from being kept
//! from every managed writer (CR2; the operator's ruling of 2026-10-18),
//! and what keeps a released worktree held (decision 0108 ruling 3). Moved
//! out of `admission.rs`, which this would take past its 800 lines
//! (decision 0071 ruling 4; the same move its own header records).

use std::path::PathBuf;

use thiserror::Error;

use crate::run_lock;

/// What keeps protected evidence or the coordination root from being kept
/// from every managed writer (CR2; the operator's ruling of 2026-10-18).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Conflict {
    #[error("run '{0}' holds this worktree, its writers not proven settled")]
    Held(String),
    /// A quarantined run's writers are proven ended, but a process still
    /// drives it, so its worktree stays held (decision 0108 ruling 3).
    #[error("{}", run_lock::named(run, holder))]
    Driven {
        run: String,
        holder: Option<run_lock::Holder>,
    },
    #[error(
        "site '{site}' of run '{run}' writes outside a workspace box, which cannot be kept from \
         protected evidence or the coordination root"
    )]
    Unboxed { site: String, run: String },
    #[error(
        "site '{site}' of run '{run}' can reach '{}', which evidence or coordination depends on",
        .root.display()
    )]
    Reaches {
        site: String,
        run: String,
        root: PathBuf,
    },
    #[error(
        "site '{site}' of run '{run}' binds '{}', which a managed writable root could replace \
         to redirect the box it is rebuilt into",
        .bind.display()
    )]
    Replaceable {
        site: String,
        run: String,
        bind: PathBuf,
    },
    #[error("'{}' is visible at a second mount point, or its mount cannot be read", .0.display())]
    Shown(PathBuf),
    #[error(
        "the artifact '{}' is not one file of one link holding the bytes its name digests",
        .0.display()
    )]
    Corrupt(PathBuf),
    /// A retirement that cannot be read refuses with CR2's cause (0105).
    #[error("the retirement of the historical root '{}' cannot be read", .0.display())]
    Retirement(PathBuf),
    #[error("the artifact root '{}' was replaced since its attempt was admitted", .0.display())]
    Replaced(PathBuf),
    #[error(
        "'{}', which a box of the attempt is built from, no longer resolves as when its attempt \
         was admitted",
        .0.display()
    )]
    Redirected(PathBuf),
}
