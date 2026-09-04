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
            let summary = git(repo, &["show", "-s", "--format=%h %s", &commit])
                .unwrap_or_else(|| format!("{commit} (not present in this repository)"));
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
mod tests {
    use super::*;
    use brokkr_core::canonical::ZERO_HASH;
    use serde_json::json;

    fn event(seq: u64, event_type: EventType, payload: Value) -> EventEnvelope {
        EventEnvelope {
            run_id: "known-run".into(),
            seq,
            event_id: format!("e{seq}"),
            event_schema_version: 1,
            event_type,
            payload,
            causation_id: None,
            correlation_id: "known-run".into(),
            attempt_id: None,
            recorded_at: "2026-09-04T00:00:00Z".into(),
            previous_hash: ZERO_HASH.into(),
            event_hash: String::new(),
        }
    }

    #[test]
    fn a_known_journal_renders_a_known_ledger() {
        let events = vec![
            event(1, EventType::RunStarted, json!({"feature": "boxed seats"})),
            event(
                2,
                EventType::EffectSucceeded,
                json!({"result": {
                    "result": "pass", "notes": "12 tests passed"
                }}),
            ),
            event(
                3,
                EventType::EffectSucceeded,
                json!({"result": {
                    "result": "residual", "notes": "one low documentation debt"
                }}),
            ),
        ];
        let rendered = render("known-run", &events, Path::new("/not/a/repository")).unwrap();
        assert_eq!(
            rendered,
            "# Delivery ledger — known-run\n\n## Delivered\n\nboxed seats\n\n\
Repository head: `unavailable`\n\n## Commits\n\n\
Git records no commits since the run began, and the journal records no explicit implementation commit IDs.\n\n\
## Verify evidence\n\n- **pass** — 12 tests passed\n\n\
## Review residuals\n\n- **residual** — one low documentation debt\n\n\
## Operator next\n\nReview the commits and evidence above, then push and merge under the repository's protected workflow.\n"
        );
    }

    #[test]
    fn commits_absent_evidence_and_the_file_writer_are_all_literal() {
        let dir = tempfile::tempdir().unwrap();
        for args in [
            ["init", "-q"].as_slice(),
            ["config", "user.email", "test@example.invalid"].as_slice(),
            ["config", "user.name", "Test"].as_slice(),
        ] {
            assert!(Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success());
        }
        std::fs::write(dir.path().join("delivered.txt"), "delivered\n").unwrap();
        assert!(Command::new("git")
            .args(["add", "delivered.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success());
        assert!(Command::new("git")
            .args([
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "boxed delivery"
            ])
            .current_dir(dir.path())
            .status()
            .unwrap()
            .success());
        let head = git(dir.path(), &["rev-parse", "HEAD"]).unwrap();
        let events = vec![
            event(1, EventType::RunStarted, json!({})),
            event(
                2,
                EventType::EffectSucceeded,
                json!({"result": {
                    "result": "complete",
                    "inputs": {"commit": head, "commits": ["absent-commit", "absent-commit", 7]}
                }}),
            ),
            event(
                3,
                EventType::EffectSucceeded,
                json!({"result": {"result": "complete", "inputs": {}}}),
            ),
        ];

        let rendered = render("known-run", &events, dir.path()).unwrap();
        assert!(rendered.contains("boxed delivery"), "{rendered}");
        assert!(
            rendered.contains("absent-commit (not present in this repository)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("No verify result is recorded."),
            "{rendered}"
        );
        assert!(
            rendered.contains("No review result is recorded."),
            "{rendered}"
        );
        assert!(
            rendered.contains("no delivery description recorded"),
            "{rendered}"
        );

        let path = write("known-run", &events, dir.path()).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), rendered);

        let ordinary_directory = tempfile::tempdir().unwrap();
        assert_eq!(git(ordinary_directory.path(), &["rev-parse", "HEAD"]), None);
        let empty = render("empty-run", &[], ordinary_directory.path()).unwrap();
        assert!(empty.contains("no delivery description recorded"));

        let repo_file = ordinary_directory.path().join("not-a-directory");
        std::fs::write(&repo_file, "x").unwrap();
        let error = write("known-run", &events, &repo_file).unwrap_err();
        assert!(error.to_string().contains("creating ledger directory"));

        let obstructed = ordinary_directory.path().join("obstructed");
        std::fs::create_dir_all(obstructed.join(".forge/ledger/known-run.md")).unwrap();
        let error = write("known-run", &events, &obstructed).unwrap_err();
        assert!(error.to_string().contains("writing ledger"));
    }
}
