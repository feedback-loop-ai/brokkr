//! Keep-refs: the journal's exhibits stay reachable (decision 0028).
//!
//! `refs/forge/keep/<run-id>/<sha>` — one ref per distinct object the
//! run's journal cites, named by the object and pointing straight at it.
//! Unlike `anchor.rs`'s chain there is nothing to chain and no synthetic
//! commit to write: a ref's only job here is to be a root, so the
//! squash-merge / branch-delete / `git gc` sequence that follows a
//! landing cannot collect the commits the journal names.
//!
//! Three properties the shape buys, all of them from naming the ref
//! after the object it holds:
//!
//! - **Idempotent.** Replanting writes the same name to the same value.
//!   `git update-ref` on an unchanged target is a no-op, so a second
//!   plant errors on nothing and moves nothing.
//! - **Cheap to list.** One `for-each-ref` answers which runs hold which
//!   exhibits — never one `rev-parse` per ref.
//! - **Deliberately deletable.** [`delete`] removes exactly one run's
//!   refs: the operator saying these exhibits may go. Nothing in the
//!   engine ever deletes a keep-ref, and nothing here ever pushes one —
//!   keep-refs are local repository state, like anchors.
//!
//! Which objects are cited is `brokkr_core::keep_refs`'s answer, folded
//! from the journal. This module is only the git side of it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use brokkr_core::keep_refs::cited_shas;
use brokkr_store::Store;
use thiserror::Error;

/// The namespace keep-refs live in. Under `refs/forge/` beside the
/// anchors, and under its own `keep/` so a ref that keeps an exhibit is
/// never mistaken for a ref that anchors a journal.
pub const KEEP_PREFIX: &str = "refs/forge/keep";

#[derive(Debug, Error)]
pub enum KeepRefsError {
    #[error("git {verb} failed: {detail}")]
    Git { verb: &'static str, detail: String },
    #[error(transparent)]
    Store(#[from] brokkr_store::StoreError),
    #[error("'{0}' cannot name a keep-ref: a run id here is letters, digits, '.', '_' and '-'")]
    UnusableRunId(String),
}

/// What one plant did. The kept set is the plan carried out; `absent`
/// names citations this repository does not hold — a head recorded in
/// another realm's repository, or an object already collected before
/// anyone planted anything. Silence there would be the failure mode
/// keep-refs exists to end, so the gap is returned rather than swallowed.
#[derive(Debug, PartialEq, Eq)]
pub struct Planted {
    pub run_id: String,
    /// Objects now held by a keep-ref, sorted.
    pub kept: Vec<String>,
    /// Objects the journal cites that this repository cannot resolve.
    pub absent: Vec<String>,
}

fn git_io<T>(result: std::io::Result<T>, verb: &'static str) -> Result<T, KeepRefsError> {
    result.map_err(|error| KeepRefsError::Git {
        verb,
        detail: error.to_string(),
    })
}

/// The same plumbing call `anchor.rs` makes, kept local so a ref-planting
/// failure names its own boundary instead of borrowing the anchor's.
/// Plumbing on dedicated refs only: the index, the working tree and the
/// checked-out branch are never touched.
fn git(repo: &Path, args: &[&str], stdin: Option<&str>) -> Result<String, KeepRefsError> {
    use std::io::Write;
    let mut command = Command::new("git");
    command.args(args).current_dir(repo);
    let out = if let Some(input) = stdin {
        let child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let mut child = git_io(child, "spawn")?;
        let write = child
            .stdin
            .take()
            .expect("piped")
            .write_all(input.as_bytes());
        git_io(write, "write")?;
        git_io(child.wait_with_output(), "wait")?
    } else {
        git_io(command.output(), "run")?
    };
    if !out.status.success() {
        return Err(KeepRefsError::Git {
            verb: "command",
            detail: format!("{args:?}: {}", String::from_utf8_lossy(&out.stderr).trim()),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A run id becomes a ref path component, so it is checked before it
/// reaches git rather than after: nothing outside this alphabet may
/// shape the namespace keep-refs are listed and deleted by.
fn usable(run_id: &str) -> Result<(), KeepRefsError> {
    let unusable = run_id.is_empty()
        || run_id.contains("..")
        || run_id.starts_with(['.', '-'])
        || run_id.ends_with('.')
        || !run_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    match unusable {
        true => Err(KeepRefsError::UnusableRunId(run_id.to_string())),
        false => Ok(()),
    }
}

fn ref_name(run_id: &str, sha: &str) -> String {
    format!("{KEEP_PREFIX}/{run_id}/{sha}")
}

/// Which of these objects does this repository actually hold? One
/// `cat-file --batch-check` for the whole set — the same "ask git once"
/// discipline [`list`] keeps.
fn present(repo: &Path, shas: &BTreeSet<String>) -> Result<BTreeSet<String>, KeepRefsError> {
    if shas.is_empty() {
        return Ok(BTreeSet::new());
    }
    let query: String = shas.iter().map(|sha| format!("{sha}\n")).collect();
    let report = git(repo, &["cat-file", "--batch-check"], Some(&query))?;
    // Each queried name is echoed back with its type and size, or with
    // `missing`. The names are the full lowercase ones just sent, so the
    // reply needs reading, not re-resolving.
    Ok(report
        .lines()
        .filter(|line| !line.ends_with("missing"))
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_string)
        .collect())
}

/// Plant a keep-ref for every object the run's journal cites.
///
/// Idempotent by construction: the ref name is derived from the object,
/// and `update-ref` writing a ref's existing value changes nothing. One
/// batched `update-ref --stdin` writes the whole plan.
pub fn plant(store: &Store, repo: &Path, run_id: &str) -> Result<Planted, KeepRefsError> {
    usable(run_id)?;
    let cited = cited_shas(&store.load(run_id)?);
    let held = present(repo, &cited)?;
    let batch: String = held
        .iter()
        .map(|sha| format!("update {} {sha}\n", ref_name(run_id, sha)))
        .collect();
    if !batch.is_empty() {
        git(repo, &["update-ref", "--stdin"], Some(&batch))?;
    }
    Ok(Planted {
        run_id: run_id.to_string(),
        absent: cited.difference(&held).cloned().collect(),
        kept: held.into_iter().collect(),
    })
}

/// Plant at a run's conclusion: best-effort, never fatal, and never
/// silent about what it could not keep. Returns the line the caller
/// should print — a planting failure, or the citations this repository
/// does not hold — and nothing at all when every exhibit is held.
///
/// The engine calls exactly this at every conclusion the anchor covers
/// (decision 0028): a gap in the exhibits is reported, like an anchor
/// gap, and never fails a run that has already ended.
pub fn plant_or_report(store: &Store, repo: &Path, run_id: &str) -> Option<String> {
    match plant(store, repo, run_id) {
        Ok(planted) if planted.absent.is_empty() => None,
        Ok(planted) => Some(format!(
            "keep-ref gap for {run_id}: {} cited object(s) are not in this repository: {}",
            planted.absent.len(),
            planted.absent.join(", "),
        )),
        Err(error) => Some(format!("keep-ref gap for {run_id}: {error}")),
    }
}

/// One listed line — `<prefix>/<run-id>/<sha> <objectname>` — read as
/// the run that holds the exhibit and the object it holds. The run id is
/// everything between the namespace and the last separator, because a
/// run id holds no `/` and an object name holds no `/` either.
///
/// Anything else found in the namespace is passed over rather than
/// guessed at: a hand-planted ref that names no run and no object is
/// not a keep-ref, and a listing that invented a holder for it would be
/// reporting something no run ever cited.
fn holder(line: &str) -> Option<(&str, &str)> {
    let (name, target) = line.split_once(' ')?;
    let (run_id, _) = name
        .strip_prefix(KEEP_PREFIX)?
        .strip_prefix('/')?
        .rsplit_once('/')?;
    Some((run_id, target))
}

/// Which runs hold which exhibits: one `for-each-ref` over the whole
/// namespace, run id to the objects its keep-refs point at.
///
/// The ref's target is reported, not the name it was planted under —
/// the name is a claim, the target is the fact, and a listing that
/// prints the claim could not show them disagreeing.
pub fn list(repo: &Path) -> Result<BTreeMap<String, Vec<String>>, KeepRefsError> {
    let listing = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            KEEP_PREFIX,
        ],
        None,
    )?;
    let mut held: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (run_id, target) in listing.lines().filter_map(holder) {
        held.entry(run_id.to_string())
            .or_default()
            .push(target.to_string());
    }
    for shas in held.values_mut() {
        shas.sort();
    }
    Ok(held)
}

/// Remove one run's keep-refs — the operator saying the exhibits may go.
/// Returns how many refs were removed; a run holding none is not an
/// error, because "already gone" is the state the caller asked for.
pub fn delete(repo: &Path, run_id: &str) -> Result<u64, KeepRefsError> {
    usable(run_id)?;
    let listing = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)",
            &format!("{KEEP_PREFIX}/{run_id}"),
        ],
        None,
    )?;
    let names: Vec<&str> = listing.lines().filter(|line| !line.is_empty()).collect();
    if names.is_empty() {
        return Ok(0);
    }
    let batch: String = names
        .iter()
        .map(|name| format!("delete {name}\n"))
        .collect();
    git(repo, &["update-ref", "--stdin"], Some(&batch))?;
    Ok(names.len() as u64)
}

#[cfg(test)]
mod tests;
