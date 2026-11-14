use super::*;
use crate::tests::envelope_builder::EnvelopeBuilder;
use brokkr_store::Store;
use serde_json::json;

fn event(seq: u64, event_type: EventType, payload: Value) -> EventEnvelope {
    EnvelopeBuilder::new(event_type, payload)
        .run("known-run")
        .seq(seq)
        .event_id(format!("e{seq}"))
        .at("2026-09-04T00:00:00Z")
        .build()
}

#[test]
fn a_known_journal_renders_a_known_ledger() {
    let events = vec![
        event(1, EventType::RunStarted, json!({"feature": "boxed seats"})),
        event(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "verify-effect", "phase": "verify", "seat": "work"}),
        ),
        event(
            3,
            EventType::EffectSucceeded,
            json!({"effect_id": "verify-effect", "result": {
                "result": "pass", "notes": "12 tests passed"
            }}),
        ),
        event(
            4,
            EventType::EffectRequested,
            json!({"effect_id": "review-effect", "phase": "review"}),
        ),
        event(
            5,
            EventType::EffectSucceeded,
            json!({"effect_id": "review-effect", "result": {
                "result": "residual", "notes": "one low documentation debt"
            }}),
        ),
        event(
            6,
            EventType::EffectRequested,
            json!({"effect_id": "design-effect", "phase": "design"}),
        ),
        event(
            7,
            EventType::EffectSucceeded,
            json!({"effect_id": "design-effect", "result": {
                "result": "fail", "notes": "design asked for another pass"
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

/// A recipe library's root at `dir`, so a ledger written anywhere under
/// it has the library's `.forge/` (decision 0083 ruling 3).
fn library_root(dir: &std::path::Path) {
    std::fs::create_dir(dir.join(".git")).unwrap();
    crate::folder::folder_tests::plant_library(dir);
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
            "boxed delivery",
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
                "inputs": {
                    "commit": head,
                    "commits": [
                        "absent-commit",
                        "absent-commit",
                        "x",
                        "--output=.git/hooks/pre-commit",
                        7
                    ]
                }
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
        rendered.contains("absent-commit (not a valid commit id)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("--output=.git/hooks/pre-commit (not a valid commit id)"),
        "{rendered}"
    );
    assert!(
        !dir.path().join(".git/hooks/pre-commit").exists(),
        "journal text must never become a git option"
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

    crate::folder::folder_tests::plant_library(dir.path());
    let path = write("known-run", &events, dir.path()).unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), rendered);

    let ordinary_directory = tempfile::tempdir().unwrap();
    assert_eq!(git(ordinary_directory.path(), &["rev-parse", "HEAD"]), None);
    let empty = render("empty-run", &[], ordinary_directory.path()).unwrap();
    assert!(empty.contains("no delivery description recorded"));

    library_root(ordinary_directory.path());
    let repo_file = ordinary_directory.path().join("not-a-directory");
    std::fs::write(&repo_file, "x").unwrap();
    let error = write("known-run", &events, &repo_file).unwrap_err();
    assert!(error.to_string().contains("creating ledger directory"));

    let obstructed = ordinary_directory.path().join("obstructed");
    std::fs::create_dir_all(obstructed.join(".forge/ledger/known-run.md")).unwrap();
    let error = write("known-run", &events, &obstructed).unwrap_err();
    assert!(error.to_string().contains("writing ledger"));
}

/// M3: the ledger derives its commits and results from the applied events,
/// so a repaired late pass and its notes never reach the document. The
/// repaired journal renders byte for byte as its terminal cut does.
#[test]
fn a_repaired_ledgers_late_evidence_is_never_delivered() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("ledger.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("known-run", "boxed seats", "bundle", &json!({}))
        .unwrap();
    for (event_type, payload) in [
        (
            EventType::RunStarted,
            json!({"feature": "boxed seats", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "verify-effect", "phase": "verify", "seat": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "verify-effect", "attempt_id": "a1"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "verify-effect",
                   "result": {"result": "pass", "notes": "12 tests passed"}}),
        ),
        (EventType::RunStopped, json!({"reason": "concluded"})),
    ] {
        store
            .append_next("known-run", event_type, payload, None, None)
            .unwrap();
    }
    let cut = store.load("known-run").unwrap();
    let head = cut.last().unwrap().clone();
    drop(store);
    let connection = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_chained(
        &connection,
        "known-run",
        &head,
        &[
            (
                EventType::EffectRequested,
                json!({"effect_id": "late-effect", "phase": "verify", "seat": "work"}),
            ),
            (
                EventType::EffectStarted,
                json!({"effect_id": "late-effect", "attempt_id": "late"}),
            ),
            (
                EventType::EffectSucceeded,
                json!({"effect_id": "late-effect",
                       "result": {"result": "pass", "notes": "late notes"}}),
            ),
            (
                EventType::OperatorCommanded,
                json!({"command_id": "c-repair", "command": "repair", "operator": "operator",
                       "args": {"first_seq": 7, "last_seq": 9, "terminal_seq": 6,
                                "event_types": ["effect/requested", "effect/started",
                                                "effect/succeeded"],
                                "reason": "kept"}}),
            ),
        ],
    )
    .unwrap();
    drop(connection);
    let store = Store::open(&db).unwrap();
    let repaired_events = store.load("known-run").unwrap();

    let repaired = render(
        "known-run",
        &repaired_events,
        Path::new("/not/a/repository"),
    )
    .unwrap();
    let terminal = render("known-run", &cut, Path::new("/not/a/repository")).unwrap();
    assert_eq!(repaired, terminal, "the ledger is the terminal cut's");
    assert!(!repaired.contains("late notes"), "{repaired}");
}
