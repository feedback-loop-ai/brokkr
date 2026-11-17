//! Decision 0106 ruling 4: a contention park is `retryable` only when the
//! journal's typed evidence proves the last attempt never executed. Each
//! shape is planted as the real writer journals it, then ended through
//! [`Engine::lawful_end_under_contention`], and the park's cause is read
//! back.

use super::*;
use crate::engine::broker::admission::tests::{
    evidence_leased, protecting, unstaked_lease, Coordinated,
};
use crate::engine::broker::settlement::witnessed;
use brokkr_protocol::process::Unsettled;
use brokkr_protocol::Cleanup;

/// An engine at `ExecuteEffect` for `effect-1`, nothing attempted yet.
fn requested(dir: &Path) -> Engine {
    let mut engine = started(dir);
    plant(
        &mut engine,
        EventType::PhaseEntered,
        json!({"phase": "work"}),
        None,
    );
    plant(
        &mut engine,
        EventType::EffectRequested,
        json!({"effect_id": "effect-1", "seat": "work"}),
        None,
    );
    engine
}

fn plant(engine: &mut Engine, event_type: EventType, payload: Value, attempt: Option<&str>) {
    let run_id = engine.run_id.clone();
    engine
        .store
        .append_next(
            &run_id,
            event_type,
            payload,
            None,
            attempt.map(str::to_string),
        )
        .unwrap();
}

fn begin(engine: &mut Engine, attempt: &str) {
    plant(
        engine,
        EventType::EffectStarted,
        json!({"effect_id": "effect-1", "attempt_id": attempt}),
        Some(attempt),
    );
}

/// A terminal `effect/failed` as the engine writes it for a start failure
/// at the sites `sites`, carrying `extra`'s fields beside them.
fn fail(engine: &mut Engine, attempt: &str, sites: Value, extra: Value) {
    let mut payload = json!({
        "effect_id": "effect-1",
        "attempt_id": attempt,
        "error": "the driver never started",
        "start_failure": true,
        "start_failure_sites": sites,
    });
    for (field, value) in extra.as_object().into_iter().flatten() {
        payload[field] = value.clone();
    }
    plant(engine, EventType::EffectFailed, payload, Some(attempt));
}

/// The cause the contention park wrote.
fn contended_cause(engine: &mut Engine) -> Value {
    engine
        .lawful_end_under_contention(contended("append"))
        .expect("a park at an effect is lawful")
        .expect("the park ends the drive");
    let parked = engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .rfind(|event| event.event_type == EventType::RunParked)
        .expect("the contention parked");
    parked.payload["cause"].clone()
}

fn assert_retryable(engine: &mut Engine, expected: bool) {
    let cause = contended_cause(engine);
    assert_eq!(cause["kind"], "infrastructure", "{cause}");
    assert_eq!(cause["retryable"], json!(expected), "{cause}");
}

#[test]
fn a_single_seat_that_failed_before_its_first_turn_reads_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    begin(&mut engine, "a1");
    fail(&mut engine, "a1", json!([null]), json!({}));
    assert_retryable(&mut engine, true);
}

/// The checkpoint `journal_panel_members` writes for `member`'s `outcome`,
/// the writer's own step and fields (panel.rs), before the terminal event.
fn member_finished(engine: &mut Engine, member: &str, outcome: &str) {
    plant(
        engine,
        EventType::EffectCheckpointed,
        json!({"effect_id": "effect-1", "attempt_id": "a1",
               "checkpoint": {"step": "panel-member-finished", "member": member,
                              "outcome": outcome, "model": "not reported",
                              "session_ref": null, "inner_checkpoints": 0}}),
        Some("a1"),
    );
}

#[test]
fn a_panel_whose_other_member_ran_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    begin(&mut engine, "a1");
    // The panel journals every member's outcome as checkpoint evidence
    // before its terminal event; member `b` ran and succeeded.
    member_finished(&mut engine, "a", "failed");
    member_finished(&mut engine, "b", "succeeded");
    fail(&mut engine, "a1", json!(["a"]), json!({}));
    assert_retryable(&mut engine, false);
}

/// The panel checkpoints a member that failed to start as it checkpoints one
/// that ran, so a panel whose members ALL failed to start never reads as a
/// clean start failure: fail-closed for `retryable` (an ask names it).
#[test]
fn a_panel_whose_members_all_failed_to_start_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    begin(&mut engine, "a1");
    member_finished(&mut engine, "a", "failed");
    member_finished(&mut engine, "b", "failed");
    fail(&mut engine, "a1", json!(["a", "b"]), json!({}));
    assert_retryable(&mut engine, false);
}

#[test]
fn a_sequence_step_that_failed_to_start_after_an_accepted_step_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    begin(&mut engine, "a1");
    plant(
        &mut engine,
        EventType::EffectCheckpointed,
        json!({"effect_id": "effect-1", "attempt_id": "a1",
               "checkpoint": {"step": "sequence-step-finished", "step_name": "one",
                              "model": "m", "result": {"result": "ok"}}}),
        Some("a1"),
    );
    fail(&mut engine, "a1", json!(["two"]), json!({}));
    assert_retryable(&mut engine, false);
}

#[test]
fn a_typed_refusal_that_recorded_a_wait_reads_not_retryable() {
    for extra in [
        json!({"waits": [{"provider": "anthropic", "route": "r", "until": "2030-01-01T00:00:00Z",
                          "seconds": 60}]}),
        json!({"refused": {"provider": "anthropic", "cause": "route-unresolved"}}),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = requested(dir.path());
        begin(&mut engine, "a1");
        fail(&mut engine, "a1", json!([null]), extra);
        assert_retryable(&mut engine, false);
    }
}

#[test]
fn an_effect_at_its_cap_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    // The fixture seat's cap is two attempts.
    for attempt in ["a1", "a2"] {
        begin(&mut engine, attempt);
        fail(&mut engine, attempt, json!([null]), json!({}));
    }
    assert_retryable(&mut engine, false);
}

#[test]
fn a_failure_that_was_not_a_start_failure_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = requested(dir.path());
    begin(&mut engine, "a1");
    // A deadline kill: `failed_to_start` excludes it, so the engine writes
    // no `start_failure` field at all.
    plant(
        &mut engine,
        EventType::EffectFailed,
        json!({"effect_id": "effect-1", "attempt_id": "a1", "error": "the deadline killed it"}),
        Some("a1"),
    );
    assert_retryable(&mut engine, false);
}

/// What the reservation fact is read from: the coordination root dispatch
/// observes, never the engine's lease (ruling 4, "no standing writer
/// reservation"). Each test plants its attempt's clean start failure, takes
/// a lease on a host this thread observes, and ends the attempt as
/// [`Engine::settle`] does: the lease taken off the engine first.
///
/// The engine, the host (its directory, and the run's directory in its
/// root), and the guard that points dispatch's observation at the host:
/// each is bound by name by the caller, so none drops early.
type Leased = (Engine, (tempfile::TempDir, PathBuf), Coordinated);

fn leased(dir: &Path, lease: fn(&mut Engine, &str) -> (tempfile::TempDir, PathBuf)) -> Leased {
    let mut engine = requested(dir);
    begin(&mut engine, "a1");
    fail(&mut engine, "a1", json!([null]), json!({}));
    let host = lease(&mut engine, "a1");
    let coordinated = Coordinated::at(&host.0.path().join("home"));
    (engine, host, coordinated)
}

#[test]
fn a_standing_writer_reservation_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, _host, _coordinated) = leased(dir.path(), evidence_leased);
    // The reservation stands in the root, and the engine holds nothing: the
    // state a settle that returned early leaves.
    drop(engine.held.take());
    assert_retryable(&mut engine, false);
}

#[test]
fn a_settle_that_met_contention_after_taking_the_lease_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, _host, _coordinated) = leased(dir.path(), evidence_leased);
    // `ran` is the attempt's own error, after its terminal landed. Settle
    // returns it at the `?` right after taking the lease, as it returns a
    // contended `load` at the next one: a WAL journal cannot be made to
    // refuse a read, so the first is the one planted, and both leave the
    // lease dropped and the reservation standing.
    let ended = engine.settle("a1", Err(contended("append")));
    assert!(matches!(ended, Err(EngineError::Store(_))), "{ended:?}");
    assert!(engine.held.is_none(), "settle took the lease off first");
    assert_retryable(&mut engine, false);
}

#[test]
fn a_release_not_made_for_an_unstaked_attempt_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, _host, _coordinated) = leased(dir.path(), unstaked_lease);
    // A driver whose cleanup is unresolved proves nothing settled, so no
    // release is made; the attempt staked no evidence, so the settle
    // answers Ok with the reservation standing.
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Stdout,
    };
    witnessed(&mut engine.held, &unresolved);
    engine
        .settle("a1", Ok(()))
        .expect("an unstaked attempt settles Ok");
    assert!(engine.held.is_none());
    assert_retryable(&mut engine, false);
}

/// [`leased`] on a host protecting evidence, then settled as the engine
/// settles a proven attempt: the release committed, nothing stands.
fn released(dir: &Path) -> Leased {
    let (mut engine, host, coordinated) = leased(dir, evidence_leased);
    engine
        .settle("a1", Ok(()))
        .expect("the attempt is proven settled");
    (engine, host, coordinated)
}

#[test]
fn a_release_that_committed_leaves_nothing_standing_and_reads_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, _host, _coordinated) = released(dir.path());
    assert_retryable(&mut engine, true);
}

/// Only the run's own standing attempt is its reservation: another run's
/// attempt standing in the same root, beside this run's released one (its
/// entry standing with no attempt left), leaves this run retryable.
#[test]
fn another_runs_standing_attempt_is_not_this_runs_reservation() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, host, _coordinated) = released(dir.path());
    let elsewhere = host.0.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let _other = protecting(&host.0.path().join("home"), &elsewhere);
    assert_retryable(&mut engine, true);
}

#[test]
fn a_reservation_that_cannot_be_read_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, host, _coordinated) = released(dir.path());
    // A record that does not read is a participant nothing can vouch for:
    // absence is not proven.
    std::fs::write(host.1.join("reservation.json"), b"not a reservation").unwrap();
    assert_retryable(&mut engine, false);
}

#[test]
fn a_root_that_cannot_be_opened_reads_not_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, host, _coordinated) = released(dir.path());
    // A HOME with no root that could be protected cannot be read at all.
    let _unrooted = Coordinated::unrooted(&host.0.path().join("home"));
    assert_retryable(&mut engine, false);
}
