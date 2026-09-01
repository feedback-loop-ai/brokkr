//! Git-ref anchoring: tamper-EVIDENCE, not tamper-proofing (the
//! referee-era lore in reference/handoff-protocol.md, ported). Every
//! anchor is a commit on `refs/forge/<run_id>` recording the journal's
//! seq and head hash; commit objects are content-addressed, so a
//! consistent rewrite of the database still fails against the ref chain.
//! Built with plumbing on a dedicated ref: the index, working tree, and
//! checked-out branch are never touched. The ref carries no signature —
//! anyone able to run git here can rebuild it; it raises the cost of
//! forgery and makes honest corruption visible (the signing service
//! stays deferred, decision 0008).

use std::path::Path;
use std::process::Command;

use brokkr_store::Store;
use serde_json::{json, Value};
use thiserror::Error;

/// git's well-known empty tree.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[derive(Debug, Error)]
pub enum AnchorError {
    #[error("git {verb} failed: {detail}")]
    Git { verb: &'static str, detail: String },
    #[error(transparent)]
    Store(#[from] brokkr_store::StoreError),
    #[error("anchor mismatch: {0}")]
    Mismatch(String),
    #[error("no anchor ref for run '{0}'")]
    NoAnchor(String),
}

fn git_io<T>(result: std::io::Result<T>, verb: &'static str) -> Result<T, AnchorError> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(AnchorError::Git {
            verb,
            detail: error.to_string(),
        }),
    }
}

fn git(repo: &Path, args: &[&str], stdin: Option<&str>) -> Result<String, AnchorError> {
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
        return Err(AnchorError::Git {
            verb: "command",
            detail: format!("{args:?}: {}", String::from_utf8_lossy(&out.stderr).trim()),
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn ref_name(run_id: &str) -> String {
    format!("refs/forge/{run_id}")
}

/// Append an anchor commit for the run's current journal head. Chains to
/// the previous anchor when one exists. Returns the commit sha.
pub fn anchor(store: &Store, repo: &Path, run_id: &str) -> Result<String, AnchorError> {
    let (seq, head_hash) = store.head_hash(run_id)?;
    let reference = ref_name(run_id);
    let parent = git(
        repo,
        &["rev-parse", "--verify", "--quiet", &reference],
        None,
    )
    .ok();
    let message = json!({
        "anchor": "forge.journal-anchor/v1",
        "run_id": run_id,
        "seq": seq,
        "journal_head_hash": head_hash,
    })
    .to_string();
    let mut args = vec!["commit-tree", EMPTY_TREE];
    if let Some(parent_sha) = parent.as_deref() {
        args.extend(["-p", parent_sha]);
    }
    let sha = git(repo, &args, Some(&message))?;
    git(repo, &["update-ref", &reference, &sha], None)?;
    Ok(sha)
}

/// Verify the anchor tip against the journal's current head: same seq,
/// same head hash. A mismatch distinguishes a moved journal (stale
/// anchor) from agreement; either way the caller gets the facts.
pub fn verify(store: &Store, repo: &Path, run_id: &str) -> Result<Value, AnchorError> {
    let reference = ref_name(run_id);
    let tip = git(
        repo,
        &["rev-parse", "--verify", "--quiet", &reference],
        None,
    )
    .map_err(|_| AnchorError::NoAnchor(run_id.to_string()))?;
    let message = git(repo, &["log", "-1", "--format=%B", &tip], None)?;
    let recorded: Value = serde_json::from_str(message.trim())
        .map_err(|e| AnchorError::Mismatch(format!("unreadable anchor message: {e}")))?;
    let (seq, head_hash) = store.head_hash(run_id)?;
    let anchored_seq = recorded["seq"].as_u64().unwrap_or(0);
    let anchored_hash = recorded["journal_head_hash"].as_str().unwrap_or("");
    if anchored_seq != seq || anchored_hash != head_hash {
        return Err(AnchorError::Mismatch(format!(
            "anchor records seq {anchored_seq} hash {anchored_hash}; journal has \
             seq {seq} hash {head_hash} — the journal moved after anchoring, or \
             was rewritten",
        )));
    }
    let chain_length = git(repo, &["rev-list", "--count", &tip], None)?;
    Ok(json!({
        "ref": reference,
        "tip": tip,
        "seq": seq,
        "journal_head_hash": head_hash,
        "chain_length": chain_length.parse::<u64>().unwrap_or(0),
        "verdict": "anchored",
    }))
}

#[cfg(test)]
mod tests;
