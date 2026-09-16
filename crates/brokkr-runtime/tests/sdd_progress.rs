//! Proposed decision 0056 ruling 10: the SDD smith persists task
//! progress before the next group, and a successor recovers it from the
//! worktree rather than from a session it does not have.
//!
//! Both tests here prove distribution and mechanism, never obedience.
//! A charter is an instruction, not an engine guarantee, and the decision
//! and the guides both say so; what is testable is that the instruction
//! is where it should be, that a judge is not granted the edit, and that
//! an artifact written the way the charter asks carries the two facts a
//! successor needs across an interruption with no commit and no session.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// What a successor reads off a dialect task artifact: which task
/// identifiers are ticked, and which groups are recorded in progress.
///
/// Deliberately the whole of the recovery mechanism. There is no journal
/// event for progress, no engine service and no store: the artifact in
/// the worktree IS the record, which is what makes it survive a death
/// that takes the session, the process and everything uncommitted with
/// it — except the files.
fn read_progress(text: &str) -> (Vec<String>, Vec<String>) {
    let mut done = Vec::new();
    let mut in_progress = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("- [x] ") {
            done.push(rest.split_whitespace().next().unwrap_or("").to_string());
        }
        if let Some(rest) = line.strip_prefix("## ") {
            if rest.contains("(in progress") {
                in_progress.push(rest.split_whitespace().next().unwrap_or("").to_string());
            }
        }
    }
    (done, in_progress)
}

/// The interruption the charter is written for, in a temporary worktree:
/// one group finished and ticked with its evidence, a second recorded in
/// progress, and then the process dies before any commit.
///
/// The successor has no session, no transcript and no commit to read.
/// What it has is the file, and the file distinguishes the three states
/// the charter requires: done, in progress, and not started. Before this
/// rule the same interruption left every box unticked beside a worktree
/// full of edits — issue #226's 25 modified files against 0 of 124.
#[test]
fn an_interrupted_group_is_recovered_from_the_artifact_not_from_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let artifact = dir.path().join("tasks.md");

    // The smith finishes group 1 and persists it BEFORE starting group 2
    // — not at commit time, which is the whole of the timing rule.
    std::fs::write(
        &artifact,
        "## 1. The contract (checks: the store's own suite)\n\
         \n\
         - [x] 1.1 Publish the schema — `cargo test -p brokkr-store` green.\n\
         - [x] 1.2 Embed and dispatch it — same suite, same run.\n\
         \n\
         ## 2. The engine (in progress; checks: the resume suite)\n\
         \n\
         - [ ] 2.1 Derive the structural site key.\n\
         - [ ] 2.2 Offer it at every work topology.\n\
         \n\
         ## 3. The adapters\n\
         \n\
         - [ ] 3.1 Publish one confirmed launch per site.\n",
    )
    .unwrap();

    // The interruption: uncommitted edits survive, nothing else does.
    std::fs::write(dir.path().join("engine.rs"), "// half of 2.1\n").unwrap();

    let (done, in_progress) = read_progress(&std::fs::read_to_string(&artifact).unwrap());
    assert_eq!(
        done,
        ["1.1", "1.2"],
        "the successor sees group one's finished work, with its evidence"
    );
    assert_eq!(
        in_progress,
        ["2."],
        "and sees that group two was started and not finished"
    );
    assert!(
        !done.iter().any(|task| task.starts_with("2.")),
        "a partially implemented task stays unchecked: {done:?}"
    );
    assert!(
        dir.path().join("engine.rs").exists(),
        "the surviving edits are the successor's to reconcile, never to erase \
         because it does not remember writing them"
    );
}

/// Proposed decision 0056 ruling 10's last clause: judges stay read-only.
///
/// A judging office that could tick a box could close its own finding,
/// and the progress record would stop being the smith's own report of
/// what it verified. Each review charter says where a finding goes; none
/// of them is granted the task artifact.
#[test]
fn no_judging_office_is_granted_the_task_artifact() {
    let charters = root().join("agents/charters");
    // Each judging office, and the phrase in which it says it writes
    // nothing. Read as a pair with the grants below: one of these has to
    // be present, and none of those may be.
    for (office, refusal) in [
        ("analyst.md", "without\nediting them"),
        ("clarifier.md", "without editing it"),
        ("reviewer.md", "strictly read-only"),
        ("review-chief.md", "change no files"),
        ("review-adversarial.md", "Strictly read-only"),
        ("review-correctness.md", "strictly read-only"),
        ("review-security.md", "Strictly read-only"),
        ("review-spec-compliance.md", "read-only member"),
    ] {
        let text = std::fs::read_to_string(charters.join(office))
            .unwrap_or_else(|e| panic!("{office} must exist: {e}"));
        assert!(
            text.contains(refusal),
            "{office} no longer states that it writes nothing"
        );
        let flat = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        for granted in [
            "tick each",
            "tick the",
            "check the box",
            "mark the task",
            "update the work breakdown",
            "edit the task",
        ] {
            assert!(
                !flat.contains(granted),
                "{office} appears to grant a judge {granted:?}"
            );
        }
    }
    // And the office that DOES own the ticks says so, so the division is
    // readable from the two charters together rather than inferred.
    let smith = std::fs::read_to_string(charters.join("implementer-sdd.md")).unwrap();
    assert!(smith.contains("tick each completed task"));
}
