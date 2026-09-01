//! Run selectors — the one place a `--run` string becomes a run id.
//!
//! A run id is 41 characters, and typing it in full is painful exactly
//! when a run is worth watching. So every readout accepts a unique id
//! prefix and the literal `latest` as well as the full id. The rules
//! live here once: `resolve` is a pure function over the run list plus
//! the requested string, and `resolve_run` is the thin read-only lookup
//! that hands it the workspace database's runs. Resolution reads the
//! run table and never writes the journal.

use anyhow::{anyhow, Result};
use brokkr_store::Store;

use crate::render::Safe;

/// The selector that means "the run I started most recently".
pub const LATEST: &str = "latest";

/// What selection needs to know about a run: its identity and when it
/// was created. Nothing else bears on the choice, so nothing else is
/// asked for — the rules stay testable without a database.
#[derive(Debug, Clone, Copy)]
pub struct RunRef<'a> {
    pub run_id: &'a str,
    pub created_at: &'a str,
}

/// Resolve a user's `--run` string against the runs a workspace holds.
///
/// An exact id wins outright, so a run whose id is also another run's
/// prefix is never ambiguous with itself. Otherwise a prefix must match
/// exactly one run: matching several is an error that names the
/// candidates, because picking one for the operator would be a guess
/// about which run they meant.
pub fn resolve(runs: &[RunRef<'_>], requested: &str) -> Result<String> {
    if requested == LATEST {
        // Ordering by the recorded stamp rather than trusting the
        // query's ORDER BY: "newest" is a property of the runs, not of
        // how they arrived here.
        return runs
            .iter()
            .max_by_key(|run| run.created_at)
            .map(|run| run.run_id.to_string())
            .ok_or_else(|| {
                anyhow!("no runs in this workspace database; 'latest' resolves to nothing")
            });
    }
    if runs.iter().any(|run| run.run_id == requested) {
        return Ok(requested.to_string());
    }
    let matched: Vec<&str> = runs
        .iter()
        .filter(|run| run.run_id.starts_with(requested))
        .map(|run| run.run_id)
        .collect();
    // These strings reach the operator's tty through anyhow, so the
    // requested selector and every candidate id are sanitized.
    match matched.as_slice() {
        [only] => Ok((*only).to_string()),
        [] => Err(anyhow!(
            "no run matching '{}' in this workspace database",
            Safe::new(requested).as_str()
        )),
        ambiguous => Err(anyhow!(
            "'{}' matches {} runs: {}; use more characters",
            Safe::new(requested).as_str(),
            ambiguous.len(),
            ambiguous
                .iter()
                .map(|id| Safe::new(id).as_str().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The store-facing form every command that takes `--run` calls: read
/// the run table, apply the rules above. Read-only.
pub fn resolve_run(store: &Store, requested: &str) -> Result<String> {
    let runs = store.list_runs()?;
    let refs: Vec<RunRef<'_>> = runs
        .iter()
        .map(|(run_id, _feature, created_at)| RunRef { run_id, created_at })
        .collect();
    resolve(&refs, requested)
}

#[cfg(test)]
mod tests;
