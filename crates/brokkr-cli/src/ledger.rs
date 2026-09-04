//! Deterministic delivery ledger rendered from journal evidence and git.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn result_notes(events: &[EventEnvelope], vocabulary: &[&str]) -> Vec<(String, String)> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .filter_map(|event| {
            let result = event.payload.pointer("/result/result")?.as_str()?;
            vocabulary.contains(&result).then(|| {
                (
                    result.to_string(),
                    event
                        .payload
                        .pointer("/result/notes")
                        .and_then(Value::as_str)
                        .unwrap_or("no notes recorded")
                        .to_string(),
                )
            })
        })
        .collect()
}

fn commit_ids(events: &[EventEnvelope], repo: &Path) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut commits = Vec::new();
    if let Some(started_at) = events.first().map(|event| event.recorded_at.as_str()) {
        let since = format!("--since={started_at}");
        if let Some(log) = git(repo, &["log", "--format=%H", &since]) {
            for commit in log.lines().filter(|line| !line.is_empty()) {
                seen.insert(commit.to_string());
                commits.push(commit.to_string());
            }
        }
    }
    for event in events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
    {
        let Some(inputs) = event.payload.pointer("/result/inputs") else {
            continue;
        };
        if let Some(commit) = inputs.get("commit").and_then(Value::as_str) {
            if seen.insert(commit.to_string()) {
                commits.push(commit.to_string());
            }
        }
        if let Some(items) = inputs.get("commits").and_then(Value::as_array) {
            for commit in items.iter().filter_map(Value::as_str) {
                if seen.insert(commit.to_string()) {
                    commits.push(commit.to_string());
                }
            }
        }
    }
    commits
}

fn is_commit_id(value: &str) -> bool {
    (7..=40).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Render only facts the journal or repository can answer. In particular,
/// the v1 journal has no run-start Git head, so the bounded range is the
/// repository's commits timestamped after the first journal event, augmented by
/// any commit IDs a newer result records explicitly.
pub fn render(run_id: &str, events: &[EventEnvelope], repo: &Path) -> Result<String> {
    let feature = events
        .first()
        .and_then(|event| event.payload.get("feature"))
        .and_then(Value::as_str)
        .unwrap_or("no delivery description recorded");
    let head = git(repo, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unavailable".to_string());
    let commits = commit_ids(events, repo);
    let verify = result_notes(events, &["pass", "fail"]);
    let review = result_notes(events, &["clean", "residual", "security-hold"]);

    let mut ledger = format!(
        "# Delivery ledger — {run_id}\n\n## Delivered\n\n{feature}\n\n\
         Repository head: `{head}`\n\n## Commits\n\n"
    );
    if commits.is_empty() {
        ledger.push_str(
            "Git records no commits since the run began, and the journal records no explicit implementation commit IDs.\n",
        );
    } else {
        for commit in commits {
            let summary = if is_commit_id(&commit) {
                git(
                    repo,
                    &["show", "-s", "--format=%h %s", "--end-of-options", &commit],
                )
                .unwrap_or_else(|| format!("{commit} (not present in this repository)"))
            } else {
                format!("{commit} (not a valid commit id)")
            };
            ledger.push_str(&format!("- {summary}\n"));
        }
    }
    ledger.push_str("\n## Verify evidence\n\n");
    if verify.is_empty() {
        ledger.push_str("No verify result is recorded.\n");
    } else {
        for (result, notes) in verify {
            ledger.push_str(&format!("- **{result}** — {notes}\n"));
        }
    }
    ledger.push_str("\n## Review residuals\n\n");
    if review.is_empty() {
        ledger.push_str("No review result is recorded.\n");
    } else {
        for (result, notes) in review {
            ledger.push_str(&format!("- **{result}** — {notes}\n"));
        }
    }
    ledger.push_str(
        "\n## Operator next\n\nReview the commits and evidence above, then push and merge under the repository's protected workflow.\n",
    );
    Ok(ledger)
}

pub fn write(run_id: &str, events: &[EventEnvelope], repo: &Path) -> Result<std::path::PathBuf> {
    let ledger = render(run_id, events, repo)?;
    let path = repo.join(".forge/ledger").join(format!("{run_id}.md"));
    std::fs::create_dir_all(path.parent().expect("ledger path has a parent"))
        .with_context(|| format!("creating ledger directory for {}", path.display()))?;
    std::fs::write(&path, ledger).with_context(|| format!("writing ledger {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests;
