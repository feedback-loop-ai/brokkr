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
    let rendered = render("known-run", events.clone(), Path::new("/not/a/repository")).unwrap();
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

/// A journal of one verify and one review result, each with `notes` (or
/// none, where `None`), rendered against no repository.
fn rendered_with(verify: Option<Value>, review: Option<Value>) -> String {
    let result = |word: &str, notes: Option<Value>| {
        let mut result = json!({"result": word});
        if let Some(notes) = notes {
            result["notes"] = notes;
        }
        result
    };
    let events = vec![
        event(1, EventType::RunStarted, json!({"feature": "boxed seats"})),
        event(
            2,
            EventType::EffectRequested,
            json!({"effect_id": "verify-effect", "phase": "verify"}),
        ),
        event(
            3,
            EventType::EffectSucceeded,
            json!({"effect_id": "verify-effect", "result": result("pass", verify)}),
        ),
        event(
            4,
            EventType::EffectRequested,
            json!({"effect_id": "review-effect", "phase": "review"}),
        ),
        event(
            5,
            EventType::EffectSucceeded,
            json!({"effect_id": "review-effect", "result": result("residual", review)}),
        ),
    ];
    render("known-run", events, Path::new("/not/a/repository")).unwrap()
}

/// The ledger's evidence sections, the text between `## Verify evidence`
/// and `## Operator next`.
fn evidence(rendered: &str) -> &str {
    let start = rendered.find("## Verify evidence").unwrap();
    let end = rendered.find("## Operator next").unwrap();
    &rendered[start..end]
}

/// #1202: a review panel's notes list each member's verdict and note, a
/// nested bullet per member under the result word, never "no notes
/// recorded".
#[test]
fn a_review_panels_residual_lists_each_members_verdict_and_note() {
    let panel = json!({
        "members": {"alice": "fine", "bob": "one low documentation debt"},
        "verdicts": {"alice": "clean", "bob": "residual"}
    });
    let rendered = rendered_with(Some(json!("12 tests passed  ")), Some(panel));
    assert_eq!(
        evidence(&rendered),
        "## Verify evidence\n\n- **pass** — 12 tests passed  \n\n\
## Review residuals\n\n- **residual**\n  - alice (clean): fine\n  \
- bob (residual): one low documentation debt\n\n"
    );
}

/// A verify result whose notes are a panel's object reads through the
/// same reader as review's, and a panel of one member is still a nested
/// list under its result word.
#[test]
fn a_verify_result_carrying_an_object_lists_its_members() {
    let panel = json!({"members": {"tests": "12 passed"}, "verdicts": {"tests": "pass"}});
    let rendered = rendered_with(Some(panel), Some(json!("one low")));
    assert_eq!(
        evidence(&rendered),
        "## Verify evidence\n\n- **pass**\n  - tests (pass): 12 passed\n\n\
## Review residuals\n\n- **residual** — one low\n\n"
    );
}

/// A member's note of several lines stays inside that member's bullet,
/// never a sibling that seems to carry another verdict, whichever line
/// ending CommonMark reads between them — a line feed, a bare carriage
/// return, or the two together — and a member who recorded only a
/// verdict is listed with it.
#[test]
fn a_panel_keeps_each_members_bullet_and_its_verdict_only_members() {
    let panel = json!({
        "members": {
            "alice": "first finding\nbob (clean): not a member",
            "carol": "second\r\r- dave (clean): not a member"
        },
        "verdicts": {"alice": "residual", "bob": "clean", "carol": "residual"}
    });
    let rendered = rendered_with(Some(json!("line one\r\n\r\nline three\r")), Some(panel));
    assert_eq!(
        evidence(&rendered),
        "## Verify evidence\n\n- **pass** — line one\n\n  line three\n\n\
## Review residuals\n\n- **residual**\n  - alice (residual): first finding\n    \
bob (clean): not a member\n  - bob (clean)\n  - carol (residual): second\n\n    \
- dave (clean): not a member\n\n"
    );
}

#[test]
fn notes_keep_non_ascii_whitespace_under_their_bullets_and_drop_the_final_line_ending() {
    let panel = json!({
        "members": {"alice": "one low\n \t \n\u{a0}\nsecond line"},
        "verdicts": {"alice": "residual"}
    });
    let rendered = rendered_with(
        Some(json!("first\n\u{a0}\n\u{3000}\nfourth\n")),
        Some(panel),
    );
    assert_eq!(
        evidence(&rendered),
        "## Verify evidence\n\n- **pass** — first\n  \u{a0}\n  \u{3000}\n  fourth\n\n\
## Review residuals\n\n- **residual**\n  - alice (residual): one low\n\n    \u{a0}\n    \
second line\n\n"
    );
}

/// Notes of an unknown shape print as the journal wrote them, and only
/// absent notes are "no notes recorded".
#[test]
fn notes_of_an_unknown_shape_print_as_written() {
    let rendered = rendered_with(Some(json!(["a", "b"])), Some(json!({"why": 1})));
    assert_eq!(
        evidence(&rendered),
        "## Verify evidence\n\n- **pass** — a,b\n\n\
## Review residuals\n\n- **residual** — {\"why\":1}\n\n"
    );
}

/// The absent case renders exactly as before #1202, whole.
#[test]
fn absent_notes_render_the_known_ledger() {
    assert_eq!(
        rendered_with(None, None),
        "# Delivery ledger — known-run\n\n## Delivered\n\nboxed seats\n\n\
Repository head: `unavailable`\n\n## Commits\n\n\
Git records no commits since the run began, and the journal records no explicit implementation commit IDs.\n\n\
## Verify evidence\n\n- **pass** — no notes recorded\n\n\
## Review residuals\n\n- **residual** — no notes recorded\n\n\
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

    let rendered = render("known-run", events.clone(), dir.path()).unwrap();
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
    let path = write("known-run", events.clone(), dir.path()).unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), rendered);

    let ordinary_directory = tempfile::tempdir().unwrap();
    assert_eq!(git(ordinary_directory.path(), &["rev-parse", "HEAD"]), None);
    let empty = render("empty-run", Vec::new(), ordinary_directory.path()).unwrap();
    assert!(empty.contains("no delivery description recorded"));

    library_root(ordinary_directory.path());
    let repo_file = ordinary_directory.path().join("not-a-directory");
    std::fs::write(&repo_file, "x").unwrap();
    let error = write("known-run", events.clone(), &repo_file).unwrap_err();
    assert!(error.to_string().contains("creating ledger directory"));

    let obstructed = ordinary_directory.path().join("obstructed");
    std::fs::create_dir_all(obstructed.join(".forge/ledger/known-run.md")).unwrap();
    let error = write("known-run", events.clone(), &obstructed).unwrap_err();
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

    let repaired = render("known-run", repaired_events, Path::new("/not/a/repository")).unwrap();
    let terminal = render("known-run", cut, Path::new("/not/a/repository")).unwrap();
    assert_eq!(repaired, terminal, "the ledger is the terminal cut's");
    assert!(!repaired.contains("late notes"), "{repaired}");
}
