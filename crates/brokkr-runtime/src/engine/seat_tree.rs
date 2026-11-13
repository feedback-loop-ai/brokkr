//! The attempt's seat-tree record and the tree it makes (decision 0104
//! rulings 6 and 7, slice 5b): the row is appended before the tree, and the
//! tree's two private directories are made only after it. A refused record
//! returns the spawn's refusal and makes no tree. The unboxed environment
//! is composed here too, so `engine.rs` keeps only the call site.

use std::collections::BTreeMap;
use std::path::Path;

use brokkr_protocol::hands::HandsSpec;

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

/// The unboxed spawn's environment, its tree made only once its row is
/// recorded. A refused record or an unmade tree is the spawn's refusal, so
/// no tree is made after a refused append.
pub(super) fn unboxed(
    engine: &mut Engine,
    ids: AttemptIds<'_>,
    spec: &HandsSpec,
) -> Result<Unboxed, String> {
    let root = engine.run_local().join(SEAT_SCRATCH);
    engine
        .store
        .record_seat_tree(&engine.run_id, ids.effect, ids.attempt, &root)
        .map_err(|cause| refused(ids.attempt, &cause))?;
    let scratch = root.join(ids.attempt);
    let private_home = scratch.join("home");
    let private_tmp = scratch.join("tmp");
    std::fs::create_dir_all(&private_home)
        .map_err(|cause| tree_unmade(ids.attempt, &private_home, &cause))?;
    std::fs::create_dir_all(&private_tmp)
        .map_err(|cause| tree_unmade(ids.attempt, &private_tmp, &cause))?;
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
) -> (Option<Unboxed>, Option<String>) {
    match hands {
        Some(spec) if !engine.boundary.is_boxed() && is_exec_dispatch(command) => {
            match unboxed(engine, ids, spec) {
                Ok(unboxed) => (Some(unboxed), None),
                Err(refusal) => (None, Some(refusal)),
            }
        }
        _ => (None, None),
    }
}

/// The spawn's refusal when the row could not be recorded: the record and
/// its cause, so the attempt fails to start with both.
fn refused(attempt: &str, cause: &dyn std::fmt::Display) -> String {
    format!("the seat tree record for attempt '{attempt}' was not written: {cause}")
}

/// The spawn's refusal when the row stands but the tree cannot be made.
fn tree_unmade(attempt: &str, path: &Path, cause: &std::io::Error) -> String {
    format!(
        "the seat tree for attempt '{attempt}' could not be made at '{}': {cause}",
        path.display()
    )
}

#[cfg(test)]
mod tests;
