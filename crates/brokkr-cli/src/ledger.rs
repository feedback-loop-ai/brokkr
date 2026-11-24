//! Deterministic delivery ledger rendered from journal evidence and git.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use brokkr_core::{EventEnvelope, EventType};
use brokkr_view::{applied_events, result_notes, ResultNotes};
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

fn phase_results(
    events: &[EventEnvelope],
    phase: &str,
    vocabulary: &[&str],
) -> Vec<(String, Option<ResultNotes>)> {
    let requested: BTreeMap<&str, &str> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .filter_map(|event| {
            Some((
                event.payload.get("effect_id")?.as_str()?,
                event.payload.get("phase")?.as_str()?,
            ))
        })
        .collect();
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .filter_map(|event| {
            let effect_id = event.payload.get("effect_id")?.as_str()?;
            if requested.get(effect_id).copied() != Some(phase) {
                return None;
            }
            let result = event.payload.pointer("/result/result")?.as_str()?;
            vocabulary
                .contains(&result)
                .then(|| (result.to_string(), result_notes(&event.payload)))
        })
        .collect()
}

/// One result's bullet. A text follows its result word; a panel is a
/// nested list under it, one bullet per member, however many members it
/// has. Every line after a note's first is indented under its own bullet,
/// so a note of several lines stays inside it and the markdown stays a
/// list. Only absent notes are "no notes recorded".
fn result_line(result: &str, notes: Option<ResultNotes>) -> String {
    match notes {
        None => format!("- **{result}** — no notes recorded\n"),
        Some(ResultNotes::Text(text)) => format!("- **{result}** — {}\n", indented(&text, "  ")),
        Some(ResultNotes::Panel(members)) => {
            let mut line = format!("- **{result}**\n");
            for member in members {
                line.push_str(&format!("  - {}\n", indented(&member, "    ")));
            }
            line
        }
    }
}

/// `text` with every line after its first indented by `indent`, and
/// its trailing line endings dropped. A CommonMark blank line (empty or
/// containing only spaces and tabs) stays blank. A line ends at a line
/// feed, a carriage return, or the two together, so no bare carriage
/// return can start a line outside the bullet. Every other byte, trailing
/// spaces included, is kept.
fn indented(text: &str, indent: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = text.trim_end_matches('\n').split('\n');
    let mut out = lines.next().unwrap_or_default().to_string();
    for line in lines {
        out.push('\n');
        if !line.trim_matches([' ', '\t']).is_empty() {
            out.push_str(indent);
            out.push_str(line);
        }
    }
    out
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
pub(crate) fn render(run_id: &str, events: Vec<EventEnvelope>, repo: &Path) -> Result<String> {
    // The ledger derives its commits and results from the applied events:
    // a repaired tail is kept and shown and never delivered (decision
    // 0108 ruling 4). It owns the one journal it filters, in place.
    let state = brokkr_core::fold::fold(&events).ok();
    let applied = applied_events(events, state.as_ref());
    let events = applied.as_slice();
    let feature = events
        .first()
        .and_then(|event| event.payload.get("feature"))
        .and_then(Value::as_str)
        .unwrap_or("no delivery description recorded");
    let head = git(repo, &["rev-parse", "HEAD"]).unwrap_or_else(|| "unavailable".to_string());
    let commits = commit_ids(events, repo);
    let verify = phase_results(events, "verify", &["pass", "fail"]);
    let review = phase_results(events, "review", &["clean", "residual", "security-hold"]);

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
            ledger.push_str(&result_line(&result, notes));
        }
    }
    ledger.push_str("\n## Review residuals\n\n");
    if review.is_empty() {
        ledger.push_str("No review result is recorded.\n");
    } else {
        for (result, notes) in review {
            ledger.push_str(&result_line(&result, notes));
        }
    }
    ledger.push_str(
        "\n## Operator next\n\nReview the commits and evidence above, then push and merge under the repository's protected workflow.\n",
    );
    Ok(ledger)
}

pub(crate) fn write(
    run_id: &str,
    events: Vec<EventEnvelope>,
    repo: &Path,
) -> Result<std::path::PathBuf> {
    let ledger = render(run_id, events, repo)?;
    // In the repository's run-local state (decision 0083 ruling 3).
    let path = crate::folder::run_local(repo)?
        .join("ledger")
        .join(format!("{run_id}.md"));
    std::fs::create_dir_all(path.parent().expect("ledger path has a parent"))
        .with_context(|| format!("creating ledger directory for {}", path.display()))?;
    std::fs::write(&path, ledger).with_context(|| format!("writing ledger {}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests;
