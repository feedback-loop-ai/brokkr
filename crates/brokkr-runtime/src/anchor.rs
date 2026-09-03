//! Git-ref anchoring: tamper-EVIDENCE, not tamper-proofing (the
//! referee-era lore in reference/handoff-protocol.md, ported). Every
//! anchor is a commit on `refs/forge/<run_id>` recording the journal's
//! seq, head hash, and the repository HEAD it vouches for. Its tree
//! carries the canonical NDJSON, so the ref can be pushed under
//! `refs/heads/brokkr-runs/<run_id>` and verified without publishing the
//! operator's SQLite journal (decision 0033). Commit objects are
//! content-addressed, so a consistent rewrite of the database still
//! fails against the ref chain. Built with plumbing on a dedicated ref:
//! the index, working tree, and checked-out branch are never touched.
//! The ref carries no signature — anyone able to run git here can
//! rebuild it; it raises the cost of forgery and makes honest corruption
//! visible (the signing service stays deferred, decision 0008).

use std::path::Path;
use std::process::Command;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::Store;
use serde_json::{json, Map, Value};
use thiserror::Error;

/// Every anchor version this engine has written and still reads. An
/// anchor naming any other version is refused, never guessed at.
const KNOWN_ANCHORS: [&str; 3] = [
    "forge.journal-anchor/v1",
    "forge.journal-anchor/v2",
    "forge.journal-anchor/v3",
];

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

/// Run git and return its stdout trimmed: a sha, a ref name, a message.
fn git(repo: &Path, args: &[&str], stdin: Option<&str>) -> Result<String, AnchorError> {
    git_raw(repo, args, stdin).map(|out| out.trim().to_string())
}

/// Run git and return its stdout byte for byte. A diff fed to `patch-id
/// --verbatim` must keep its final newline and any trailing space, or the
/// id here and the id the gate computes from a pipe disagree.
fn git_raw(repo: &Path, args: &[&str], stdin: Option<&str>) -> Result<String, AnchorError> {
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
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn ref_name(run_id: &str) -> String {
    format!("refs/forge/{run_id}")
}

/// The run, not the caller's current checkout, says which commit was
/// reviewed and then drift-checked at ship. One repository is the
/// delivered shape today, so more than one recorded realm is ambiguous
/// and vouches for nothing rather than picking one by map order.
fn vouched_head(events: &[EventEnvelope]) -> Option<String> {
    let recorded = events.iter().rev().find_map(|event| {
        (event.event_type == EventType::TransitionDecided)
            .then(|| event.payload.pointer("/inputs/reviewed_heads"))
            .flatten()
    })?;
    let heads = recorded.as_object()?;
    if heads.len() != 1 {
        return None;
    }
    let head = heads.values().next()?.as_str()?;
    (head.len() == 40 && head.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| head.to_ascii_lowercase())
}

/// The branch a slice is measured against (decision 0038 ruling 1): the
/// remote's HEAD when the repository names one, else a local `main`,
/// else a local `master`. A repository with none of them has no branch
/// to measure against, and says so with `None`.
fn default_branch(repo: &Path) -> Option<String> {
    git(
        repo,
        &["symbolic-ref", "-q", "--short", "refs/remotes/origin/HEAD"],
        None,
    )
    .ok()
    .or_else(|| {
        ["main", "master"]
            .into_iter()
            .find(|name| {
                git(
                    repo,
                    &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")],
                    None,
                )
                .is_ok()
            })
            .map(str::to_string)
    })
}

/// What the slice changed, per file (decision 0038 ruling 1): the
/// merge-base of the vouched head with the default branch, and for every
/// path the diff from that base touches, the stable patch id of that
/// file's diff. Ancestry is not in the id, so a clean rebase keeps every
/// entry; a changed hunk moves exactly the entry it lives in. Whitespace
/// IS in the id (`--verbatim`): a space is semantic in shell, YAML and
/// Python, and `--stable` alone strips it, so a re-indented hunk would
/// keep a vouch it no longer earns. Renames are never paired: a moved
/// file is a deletion and an addition, each a path like any other, so no
/// path leaves the map by being moved. `None` when there is no branch to
/// measure against.
fn patch_identity(repo: &Path, head: &str) -> Option<(String, Map<String, Value>)> {
    let branch = default_branch(repo)?;
    let base = git(repo, &["merge-base", &branch, head], None).ok()?;
    let listed = git(
        repo,
        &["diff", "--no-renames", "--name-only", &base, head],
        None,
    )
    .ok()?;
    let mut patch = Map::new();
    for path in listed.lines() {
        let diff = git_raw(
            repo,
            &["diff", "--no-renames", &base, head, "--", path],
            None,
        )
        .ok()?;
        let id = git(repo, &["patch-id", "--verbatim"], Some(&diff)).ok()?;
        let id = id.split_whitespace().next()?.to_string();
        patch.insert(path.to_string(), Value::String(id));
    }
    Some((base, patch))
}

/// Append an anchor commit for the run's current journal head. Chains to
/// the previous anchor when one exists. Returns the commit sha.
pub fn anchor(store: &Store, repo: &Path, run_id: &str) -> Result<String, AnchorError> {
    let (seq, head_hash) = store.head_hash(run_id)?;
    let journal = store.export_ndjson(run_id)?;
    let repo_head = vouched_head(&store.load(run_id)?);
    let reference = ref_name(run_id);
    let parent = git(
        repo,
        &["rev-parse", "--verify", "--quiet", &reference],
        None,
    )
    .ok();
    let blob = git(repo, &["hash-object", "-w", "--stdin"], Some(&journal))?;
    let tree_entry = format!("100644 blob {blob}\t{run_id}.ndjson\n");
    let tree = git(repo, &["mktree"], Some(&tree_entry))?;
    let (base, patch) = match repo_head
        .as_deref()
        .and_then(|head| patch_identity(repo, head))
    {
        Some((base, patch)) => (Value::String(base), Value::Object(patch)),
        None => (Value::Null, Value::Null),
    };
    let message = json!({
        "anchor": "forge.journal-anchor/v3",
        "run_id": run_id,
        "seq": seq,
        "journal_head_hash": head_hash,
        "repo_head": repo_head,
        "base": base,
        "patch": patch,
    })
    .to_string();
    let mut args = vec!["commit-tree", &tree];
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
    let version = recorded["anchor"].as_str().unwrap_or("");
    if !KNOWN_ANCHORS.contains(&version) {
        return Err(AnchorError::Mismatch(format!(
            "unknown anchor version {version:?}; this engine reads {}",
            KNOWN_ANCHORS.join(", ")
        )));
    }
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
        "anchor": version,
        "seq": seq,
        "journal_head_hash": head_hash,
        "repo_head": recorded.get("repo_head").cloned().unwrap_or(Value::Null),
        "base": recorded.get("base").cloned().unwrap_or(Value::Null),
        "patch": recorded.get("patch").cloned().unwrap_or(Value::Null),
        "chain_length": chain_length.parse::<u64>().unwrap_or(0),
        "verdict": "anchored",
    }))
}

#[cfg(test)]
mod tests;
