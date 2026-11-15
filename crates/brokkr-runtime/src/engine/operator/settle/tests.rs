//! `brokkr operator settle`: the record that proves a quarantined run's
//! writers ended where the broker's own proof cannot be read (decision 0108
//! ruling 3). Every refusal writes nothing, and the record is read by type,
//! never by folding the run.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::fold;
use serde_json::json;

use super::{operator_settle, settle_with, Settle};
use crate::engine::broker::admission::tests::cr2;
use crate::engine::broker::admission::tests::{
    boxed_claim, dead_group, isolated, quarantined, reservation_path, unacknowledged, Fixture,
    LiveGroup,
};
use crate::engine::broker::admission::{Conflict, Unadmitted, Unknown, Witnessed};
use crate::engine::broker::settlement::release;
use crate::engine::broker::settlement::release::{Live, UnprovenWhy};
use crate::engine::tests::published_args;
use crate::engine::SettleRefusal;
use crate::run_lock;
use crate::EngineError;

/// `operator_settle` on `fixture` refused and wrote nothing: the head it
/// found is the head it leaves. The caller matches the typed refusal and
/// its words are pinned once in [`every_settle_refusal_is_typed_and_its_words_are_pinned_once`]
/// (S4; decision 0071 ruling 8).
fn refused(fixture: &mut Fixture) -> EngineError {
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = operator_settle(&mut fixture.store, "run-a", &ask()).unwrap_err();
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "nothing was written"
    );
    refused
}

/// The operator's words as every test types them.
fn ask() -> Settle<'static> {
    Settle {
        reason: "the group is gone",
        operator: "operator",
    }
}

/// A later run's claim on the fixture's worktree: what the release admits
/// or refuses.
fn next(
    fixture: &Fixture,
) -> Result<crate::engine::broker::admission::Held, crate::engine::broker::admission::Unadmitted> {
    fixture
        .host
        .admit(&boxed_claim(&fixture.work, ["run-b", "b1"]))
}

/// Settle writes the contract's record where the records prove nothing but
/// the host does, reads it back by type, follows it with no acceptance, and
/// leaves the run as unfolded as it found it: settle is no repair.
#[test]
fn settle_writes_the_record_its_contract_names_and_leaves_the_run_unfolded() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the records do not prove the attempt, so the worktree holds"
    );
    assert_eq!(
        fold(&fixture.store.load("run-a").unwrap()).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 7 },
        "the run does not fold before settle either"
    );
    let written = operator_settle(&mut fixture.store, "run-a", &ask()).expect("the facts hold");
    let mut payload = written.payload.clone();
    let id = payload["command_id"].as_str().unwrap().to_string();
    payload["command_id"] = json!("the id");
    assert_eq!(written.event_type, EventType::OperatorCommanded);
    assert!(!id.is_empty());
    let group = written.payload["args"]["attempts"][0]["groups"][0].clone();
    assert_eq!(
        payload,
        json!({
            "command_id": "the id",
            "command": "settle",
            "operator": "operator",
            "args": {
                "run": "run-a",
                "attempts": [{"attempt": "a1", "proof": "group", "groups": [group]}],
                "reason": "the group is gone",
            },
        }),
        "the record names the run, the attempt and the group it proved"
    );
    // The args are the published contract, not this test's idea of it.
    published_args(
        "contracts/operator-settle.v1.schema.json",
        &written.payload["args"],
        &json!({"attempts": [], "reason": "why"}),
    );
    published_args(
        "contracts/operator-settle.v1.schema.json",
        &written.payload["args"],
        &json!({"run": "run-a", "attempts": [],
                "reason": "why", "extra": true}),
    );
    let after = fixture.store.load("run-a").unwrap();
    assert!(
        !after
            .iter()
            .any(|event| event.event_type == EventType::OperatorAccepted && event.seq > written.seq),
        "no acceptance follows a settle"
    );
    assert_eq!(
        fold(&after).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 7 },
        "settle is no repair: the run is as unfolded as it was"
    );
    let read = release::record(&after).expect("the record reads back by type");
    assert_eq!(read.seq, written.seq);
    assert_eq!(read.args.run, "run-a");
}

/// A run no acknowledgement stands on is refused by its typed cause,
/// naming the command that answers it. Nothing is written.
#[test]
fn settle_refuses_a_run_that_is_not_acknowledged_and_names_acknowledge() {
    let mut fixture = unacknowledged(&[Witnessed::Settling], None);
    let refused = refused(&mut fixture);
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Unacknowledged { ref run }) if run == "run-a"
        ),
        "the unacknowledged run is refused by type: {refused:?}"
    );
}

/// A process group the host still shows alive is refused by name, and
/// nothing is written.
#[test]
fn settle_refuses_a_live_process_group_naming_it_and_writes_nothing() {
    if !isolated("engine::operator::settle::tests::settle_refuses_a_live_process_group_naming_it_and_writes_nothing") {
        return;
    }
    let live = LiveGroup::new();
    let mut fixture = quarantined(&[Witnessed::Settling], Some(live.group()));
    let refused = refused(&mut fixture);
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Group {
                attempt: ref found,
                group,
            })) if found == "a1" && group == live.group()
        ),
        "the live group is named by its type: {refused:?}"
    );
    drop(live);
}

/// A run another process drives is refused with the lock's own words, and
/// nothing is written.
#[test]
fn settle_refuses_a_run_a_driver_holds_naming_the_holder() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    let held = run_lock::take(&fixture.store, "run-a").expect("the run's lock is taken");
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = operator_settle(&mut fixture.store, "run-a", &ask()).unwrap_err();
    let words = refused.to_string();
    assert!(
        words.contains(&format!(
            "run 'run-a' is driven by pid {}",
            std::process::id()
        )),
        "{words}"
    );
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "nothing was written"
    );
    drop(held);
}

/// A record that proves nothing and names no group is refused: only what
/// the broker's own record names can be proven, and the refusal carries the
/// typed fact (`Live::Unproven` with `NoGroup`), never only its text (S3;
/// decision 0071 ruling 8).
#[test]
fn settle_refuses_where_the_records_prove_nothing_and_name_no_group() {
    let mut fixture = quarantined(&[Witnessed::Settling], None);
    let refused = refused(&mut fixture);
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Unproven {
                ref attempt,
                why: UnprovenWhy::NoGroup,
            })) if attempt == "a1"
        ),
        "the typed refusal is the fact, not flattened text: {refused:?}"
    );
}

/// A run that has not ended is refused: settle is an annotation on a
/// terminal run in `ROWS`.
#[test]
fn settle_refuses_a_run_with_no_terminal_event() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = crate::engine::tests::in_flight_store(&db, "live");
    let head = store.head_hash("live").unwrap();
    let refused = operator_settle(&mut store, "live", &ask()).unwrap_err();
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::NoTerminal { ref run }) if run == "live"
        ),
        "a run with no terminal event is refused by type: {refused:?}"
    );
    assert_eq!(
        store.head_hash("live").unwrap(),
        head,
        "nothing was written"
    );
}

/// A journal that moved beneath the read refuses the fenced write and lands
/// nothing: the head the record would name is no longer the head.
#[test]
fn settle_refuses_a_head_a_peer_moved_and_writes_nothing() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    let error = settle_with(&mut fixture.store, "run-a", &ask(), |store| {
        store
            .append_next(
                "run-a",
                EventType::OperatorRejected,
                json!({"command_id": "peer", "reason": "peer"}),
                None,
                None,
            )
            .unwrap();
    })
    .unwrap_err();
    assert!(
        matches!(
            error,
            EngineError::Store(brokkr_store::StoreError::HeadMoved { .. })
        ),
        "{error}"
    );
    assert!(
        release::record(&fixture.store.load("run-a").unwrap()).is_none(),
        "no settle record stands after a refused write"
    );
}

/// A repaired run is settled even though the repair lapsed its
/// acknowledgement (#511): the repair record is the operator's word, and
/// the broker's settled drivers are the proof.
#[test]
fn settle_settles_a_repaired_run_the_acknowledgement_lapsed() {
    let mut fixture = quarantined(&[Witnessed::Settled], None);
    assert!(
        fixture.store.acknowledgement("run-a").unwrap().is_some(),
        "the run is acknowledged before the repair lapses it"
    );
    crate::engine::operator_repair(
        &mut fixture.store,
        "run-a",
        &crate::engine::operator::Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    assert_eq!(
        fixture.store.acknowledgement("run-a").unwrap(),
        None,
        "the repair moved the head past the acknowledgement"
    );
    let written = operator_settle(&mut fixture.store, "run-a", &ask()).expect("the repair stands");
    assert_eq!(written.payload["args"]["attempts"][0]["proof"], "records");
}

/// The settle record carries no `lock` fact: whether a lock file stood says
/// nothing of the run, and a refused dispatch creates the file before this
/// verb ever looks (decision 0071 ruling 6). The record names only what it
/// proved.
#[test]
fn settle_writes_no_lock_fact_about_the_run() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "a refused dispatch comes first, and creates the lock file"
    );
    let written = operator_settle(&mut fixture.store, "run-a", &ask()).unwrap();
    assert!(
        written.payload["args"].get("lock").is_none(),
        "the record states no fact about the lock file: {}",
        written.payload["args"]
    );
    assert_eq!(
        written.payload["args"]["attempts"][0]["proof"], "group",
        "the host's word is the proof where no driver is settled"
    );
}

/// A reservation this build cannot read through the view refuses the
/// settle by its typed format, naming the record: the proof never guesses.
#[test]
fn settle_refuses_a_reservation_the_inventory_cannot_read() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    let at = reservation_path(fixture.host.home(), &fixture.work, "run-a");
    std::fs::write(&at, json!({"format": 99}).to_string()).unwrap();
    let refused = refused(&mut fixture);
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Unadmitted(Unadmitted::Identity(
                Unknown::Formatted(ref found, 99)
            ))) if found == &at
        ),
        "the unreadable reservation is refused by its typed format: {refused:?}"
    );
}

/// A settle record whose args are not the contract is no record: the
/// release never reads a malformed annotation as a proof.
#[test]
fn a_settle_record_whose_args_do_not_read_is_no_record() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    fixture
        .store
        .append_next(
            "run-a",
            EventType::OperatorCommanded,
            json!({
                "command_id": "c",
                "command": "settle",
                "operator": "operator",
                "args": {"run": "run-a", "attempts": [{"attempt": "a1",
                          "proof": "vibes"}], "reason": "why"},
            }),
            None,
            None,
        )
        .unwrap();
    assert!(
        release::record(&fixture.store.load("run-a").unwrap()).is_none(),
        "a proof the release does not know does not read"
    );
}

/// A run no journal holds is refused by the store's own words, and nothing
/// is read past it.
#[test]
fn settle_refuses_a_run_the_journal_does_not_hold() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let refused = operator_settle(&mut store, "missing", &ask()).unwrap_err();
    assert!(matches!(refused, EngineError::Store(_)), "{refused}");
}

/// No protected coordination root means no reservation can be read, so the
/// proof cannot be taken: settle refuses and writes nothing.
#[test]
fn settle_refuses_where_no_coordination_root_can_be_protected() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    let _unrooted =
        crate::engine::broker::admission::tests::Coordinated::unrooted(fixture.host.home());
    let refused = refused(&mut fixture);
    assert!(
        matches!(refused, EngineError::SettleRefused(SettleRefusal::NoRoot)),
        "no protected root is refused by type: {refused:?}"
    );
}

/// Every settle refusal a test asserts is a type, and each of those words is
/// pinned once here: the live group, the group-less attempt's route to
/// release, the run's own pre-proof refusals, and the view-only record's
/// route (S4, S5; decision 0071 ruling 8).
#[test]
fn every_settle_refusal_is_typed_and_its_words_are_pinned_once() {
    assert_eq!(
        Live::Group {
            attempt: "a1".into(),
            group: 7,
        }
        .to_string(),
        "attempt 'a1' stands, and its process group 7 still has members on this host"
    );
    assert_eq!(
        Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::NoGroup,
        }
        .to_string(),
        "attempt 'a1' stands: its driver records are not sealed Settled, and it recorded no \
         process group to ask the host about; repair the run, then `brokkr operator release` is \
         the operator's route where the attempt's journaled ending is indeterminate"
    );
    assert_eq!(
        SettleRefusal::NoTerminal { run: "live".into() }.to_string(),
        "run 'live' has no terminal event; settle records that a run which has already ended no \
         longer holds its worktree"
    );
    assert_eq!(
        SettleRefusal::Unacknowledged {
            run: "run-a".into(),
        }
        .to_string(),
        "run 'run-a' is not acknowledged, so it is not quarantined; answer it with `brokkr \
         operator acknowledge --run run-a --reason <why>` first"
    );
    assert_eq!(
        SettleRefusal::NoRoot.to_string(),
        "this host names no protected coordination root, so no reservation can be read"
    );
    let at = std::path::PathBuf::from("/protected/run-a/reservation.json");
    let unread = EngineError::SettleRefused(SettleRefusal::Unadmitted(
        crate::engine::broker::admission::Unadmitted::Identity(
            crate::engine::broker::admission::Unknown::ViewOnly(at.clone()),
        ),
    ));
    assert_eq!(
        unread.to_string(),
        format!(
            "settle refused, nothing was written: dispatch refused: MCP server box filesystem \
             identity is not protected: the protected inventory holds a reservation at '{}' \
             which this build reads only through the view: it names no process group, so settle \
             and release cannot prove its attempt ended here; archive the journal, re-provision \
             the worktree, and repair the archived copy for its history",
            at.display()
        ),
        "the view-only record names the operator's route once"
    );
}
