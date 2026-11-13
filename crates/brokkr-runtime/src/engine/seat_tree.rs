//! The attempt's seat-tree record and the tree it makes (decision 0104
//! rulings 6 and 7, slice 5b): the row is appended before the tree, and the
//! tree's two private directories are made only after it. A refused record
//! returns the spawn's refusal and makes no tree. The unboxed environment
//! is composed here too, so `engine.rs` keeps only the call site.

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_protocol::hands::HandsSpec;
use brokkr_store::StoreError;
use thiserror::Error;

use super::{is_exec_dispatch, network_prefix_if, Engine, Unboxed, SEAT_SCRATCH};

/// The two ids one attempt carries: the effect it serves and the attempt id
/// the engine minted. A parameter object, so [`super::Engine::compose_at`]
/// keeps its seven parameters and gains no suppression.
#[derive(Debug, Clone, Copy)]
pub(in crate::engine) struct AttemptIds<'a> {
    pub(in crate::engine) effect: &'a str,
    pub(in crate::engine) attempt: &'a str,
}

impl<'a> AttemptIds<'a> {
    pub(in crate::engine) const fn new(effect: &'a str, attempt: &'a str) -> Self {
        AttemptIds { effect, attempt }
    }
}

/// Why an unboxed spawn has no environment: its seat-tree row was refused,
/// or the tree the row names could not be made. Its `Display` is the spawn's
/// refusal, word for word.
#[derive(Debug, Error)]
pub(super) enum Unready {
    /// The append was refused; the store's own words are the cause.
    #[error("the seat tree record for attempt '{attempt}' was not written: {cause}")]
    Unrecorded {
        attempt: String,
        #[source]
        cause: StoreError,
    },
    /// The row stands but a private directory of the tree cannot be made.
    #[error(
        "the seat tree for attempt '{attempt}' could not be made at '{}': {cause}",
        path.display()
    )]
    Unmade {
        attempt: String,
        path: PathBuf,
        #[source]
        cause: std::io::Error,
    },
}

/// What an unboxed exec dispatch's seat-tree step yielded: the three states
/// `compose_at` reads, so neither a ready environment nor a refusal is a
/// bare option.
pub(super) enum Made {
    /// A boxed dispatch, a site with no hands, or no exec dispatch: no row
    /// and no tree.
    NotUnboxed,
    /// The row is recorded and the tree made: the environment it starts in.
    Ready(Unboxed),
    /// The row or the tree was refused: the spawn carries these words.
    Refused(Unready),
}

impl Made {
    /// The environment, when the row and the tree stand.
    pub(super) fn unboxed(&self) -> Option<&Unboxed> {
        match self {
            Made::Ready(unboxed) => Some(unboxed),
            Made::NotUnboxed | Made::Refused(_) => None,
        }
    }

    /// The refusal, when the row or the tree was refused.
    pub(super) fn refusal(self) -> Option<Unready> {
        match self {
            Made::Refused(refusal) => Some(refusal),
            Made::NotUnboxed | Made::Ready(_) => None,
        }
    }
}

/// The unboxed spawn's environment, its tree made only once its row is
/// recorded. A refused record or an unmade tree is the spawn's refusal, so
/// no tree is made after a refused append.
pub(super) fn unboxed(
    engine: &mut Engine,
    ids: AttemptIds<'_>,
    spec: &HandsSpec,
) -> Result<Unboxed, Unready> {
    let root = engine.run_local().join(SEAT_SCRATCH);
    engine
        .store
        .record_seat_tree(&engine.run_id, ids.effect, ids.attempt, &root)
        .map_err(|cause| Unready::Unrecorded {
            attempt: ids.attempt.to_string(),
            cause,
        })?;
    let scratch = root.join(ids.attempt);
    let private_home = scratch.join("home");
    let private_tmp = scratch.join("tmp");
    std::fs::create_dir_all(&private_home).map_err(|cause| Unready::Unmade {
        attempt: ids.attempt.to_string(),
        path: private_home.clone(),
        cause,
    })?;
    std::fs::create_dir_all(&private_tmp).map_err(|cause| Unready::Unmade {
        attempt: ids.attempt.to_string(),
        path: private_tmp.clone(),
        cause,
    })?;
    let engine_env: BTreeMap<String, String> = std::env::vars().collect();
    let identity = brokkr_protocol::hands::git_facts(&engine.workdir()).identity;
    let env = brokkr_protocol::hands::unboxed_environment(
        &engine_env,
        &brokkr_protocol::hands::home_dir(&engine_env),
        spec,
        &identity,
        &private_home,
        &private_tmp,
    );
    let (uid, gid) = brokkr_protocol::hands::ids();
    let passes = engine.network_narrowing(&env, uid, gid);
    Ok(Unboxed {
        env,
        prefix: network_prefix_if(passes, uid, gid),
    })
}

/// An unboxed exec dispatch's unboxed facts, or the refusal that keeps its
/// tree: the one predicate `compose_at` reads, so the decision lives here.
pub(super) fn made(
    engine: &mut Engine,
    ids: AttemptIds<'_>,
    hands: Option<&HandsSpec>,
    command: &[String],
) -> Made {
    match hands {
        Some(spec) if !engine.boundary.is_boxed() && is_exec_dispatch(command) => {
            match unboxed(engine, ids, spec) {
                Ok(unboxed) => Made::Ready(unboxed),
                Err(refusal) => Made::Refused(refusal),
            }
        }
        _ => Made::NotUnboxed,
    }
}

#[cfg(test)]
mod tests;
