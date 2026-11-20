//! No journal event carries a byte of a driver's stderr (#1199, decision
//! 0012's addendum of 2026-11-20): a failed or indeterminate attempt names
//! it by content-free evidence, whose digest prefix is the SHA-256 of the
//! retained stderr, and a spawn failure, which has none, names none.

use super::*;
use crate::engine::sequence::SequenceAt;

/// What the fake harness prints that the seat binds nowhere, so no driver
/// masks it.
const SENTINEL: &str = "UNBOUND-SENTINEL-1199";

/// The whole of the driver's stderr: the sentinel's line.
const SAID: &str = "the harness said UNBOUND-SENTINEL-1199\n";

/// `command`, a `sh -c` driver, printing [`SAID`] on its stderr first.
fn saying(mut command: Vec<String>) -> Vec<String> {
    command[2] = format!("printf '%s' '{SAID}' >&2; {}", command[2]);
    command
}

/// [`SAID`] as the journal names it.
fn evidence() -> String {
    let digest = &brokkr_core::canonical::sha256_bytes(SAID.as_bytes())[..16];
    format!("driver stderr: 39 bytes (first byte: letter; sha256: {digest})")
}

fn failing() -> Vec<String> {
    driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Failed {
            error: "boom".into(),
        },
    )
}

/// Every envelope of the run's journal, none of which holds the sentinel.
fn assert_sealed(engine: &Engine) -> Vec<EventEnvelope> {
    let events = engine.store.load(&engine.run_id).unwrap();
    for event in &events {
        let written = serde_json::to_string(event).unwrap();
        assert!(!written.contains(SENTINEL), "journaled: {written}");
    }
    events
}

/// The `field` text of the run's one `event_type` event.
fn terminal(events: &[EventEnvelope], event_type: EventType, field: &str) -> String {
    let event = events.iter().find(|e| e.event_type == event_type).unwrap();
    event.payload[field].as_str().unwrap().to_string()
}

fn executed(command: Vec<String>) -> (tempfile::TempDir, Engine) {
    let (dir, mut driven) = engine(single_body(command));
    let requested = requested(&driven, "effect");
    execute_work(&mut driven, &[requested]).unwrap();
    (dir, driven)
}

#[test]
fn a_failed_attempt_journals_its_stderr_by_evidence_alone() {
    assert_eq!(SAID.len(), 39);
    let (_dir, driven) = executed(saying(failing()));
    let events = assert_sealed(&driven);
    assert_eq!(
        terminal(&events, EventType::EffectFailed, "error"),
        format!("boom; {}", evidence())
    );
}

#[test]
fn an_indeterminate_attempt_journals_its_stderr_by_evidence_alone() {
    let lost = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    );
    let (_dir, driven) = executed(saying(lost));
    let events = assert_sealed(&driven);
    let reason = terminal(&events, EventType::EffectIndeterminate, "reason");
    assert!(reason.ends_with(&format!("; {}", evidence())), "{reason}");
}

#[test]
fn a_sequence_step_failure_journals_its_stderr_by_evidence_alone() {
    let (_dir, mut driven) = engine(single_body(vec!["driver".into()]));
    run_sequence(
        &mut driven,
        &SequenceAt::new("effect", "attempt", "work"),
        &[step("failing", saying(failing()))],
        &json!({"role_path": "role.md", "inputs": {}}),
        5,
        &Selection::new(),
    )
    .unwrap();
    let events = assert_sealed(&driven);
    let error = terminal(&events, EventType::EffectFailed, "error");
    assert!(error.contains(&format!("boom; {}", evidence())), "{error}");
}

#[test]
fn an_empty_stderr_is_named_in_fixed_words() {
    let (_dir, driven) = executed(failing());
    assert_eq!(failed_error(&driven), "boom; driver stderr: empty");
}

#[test]
fn a_spawn_failure_names_no_stderr() {
    let (_dir, driven) = executed(vec!["/nonexistent/brokkr-1199-driver".into()]);
    assert_eq!(
        failed_error(&driven),
        "driver did not spawn: failed to spawn driver /nonexistent/brokkr-1199-driver: \
         No such file or directory (os error 2)"
    );
}
