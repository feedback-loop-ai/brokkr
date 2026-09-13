//! Run selectors — the one place a `--run` string becomes a run id.
//!
//! A run id is 41 characters, and typing it in full is painful exactly
//! when a run is worth watching. So every readout accepts a unique id
//! prefix and the literal `latest` as well as the full id. The rules
//! live here once: `resolve` is a pure function over the run list plus
//! the requested string, and `resolve_run` is the thin read-only lookup
//! that hands it the workspace database's runs. Resolution reads the
//! run table and never writes the journal.

use anyhow::Result;
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

/// Why a selector did not resolve. A caller that walks several hearths
/// must keep an ambiguous prefix visible even when another hearth
/// answers uniquely, so the refusal remembers its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The hearth records no runs at all.
    Empty,
    /// No run in this hearth matched the requested selector.
    Missing,
    /// Several runs matched: a guess would be unsafe.
    Ambiguous,
}

/// A selector refusal that remembers its kind.
#[derive(Debug)]
struct RefusalError {
    kind: Refusal,
    message: String,
}

impl std::fmt::Display for RefusalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RefusalError {}

/// The refusal kind an error carries, when it is a selector refusal.
pub fn refusal_kind(error: &anyhow::Error) -> Option<Refusal> {
    error.downcast_ref::<RefusalError>().map(|error| error.kind)
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
                refusal(
                    Refusal::Empty,
                    "no runs in this workspace database; 'latest' resolves to nothing".to_string(),
                )
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
        [] => Err(refusal(
            Refusal::Missing,
            format!(
                "no run matching '{}' in this workspace database",
                Safe::new(requested).as_str()
            ),
        )),
        ambiguous => Err(refusal(
            Refusal::Ambiguous,
            format!(
                "'{}' matches {} runs: {}; use more characters",
                Safe::new(requested).as_str(),
                ambiguous.len(),
                ambiguous
                    .iter()
                    .map(|id| Safe::new(id).as_str().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

/// Wrap one refusal in the error the caller downcasts to classify it.
fn refusal(kind: Refusal, message: String) -> anyhow::Error {
    anyhow::Error::new(RefusalError { kind, message })
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
