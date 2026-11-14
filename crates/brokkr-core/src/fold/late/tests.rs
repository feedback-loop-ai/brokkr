//! Decision 0108 ruling 4: a run broken by an event after its terminal
//! one folds again once an operator's repair record names that tail, and
//! the tail is kept, shown and never applied.

use super::*;
use crate::canonical::ZERO_HASH;
use crate::envelope_builder::EnvelopeBuilder;
use crate::fold::{fold, fold_onto, FoldError, OperatorCommand, Run};
use crate::summary::{Account, Carried};
use serde_json::{json, Value};

/// Append `event_type` to `events`, chained onto the head and sealed as
/// the store seals one.
fn push(events: &mut Vec<EventEnvelope>, event_type: EventType, payload: Value) {
    let seq = events.len() as u64 + 1;
    let previous = events
        .last()
        .map_or_else(|| ZERO_HASH.to_string(), |event| event.event_hash.clone());
    let event = EnvelopeBuilder::new(event_type, payload)
        .run("r1")
        .seq(seq)
        .event_id(format!("e{seq}"))
        .at(format!("2026-11-13T00:00:0{seq}Z"))
        .previous(previous)
        .sealed();
    events.push(event);
}

/// A run stopped at seq 5, standing at an effect that never settled.
fn stopped() -> Vec<EventEnvelope> {
    opened(&[
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a"}),
        ),
        (EventType::RunStopped, json!({"reason": "concluded"})),
    ])
}

/// A run opened and in `work`, with `after` appended in order.
fn opened(after: &[(EventType, Value)]) -> Vec<EventEnvelope> {
    let mut events = Vec::new();
    push(
        &mut events,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
    );
    push(
        &mut events,
        EventType::PhaseEntered,
        json!({"phase": "work"}),
    );
    for (event_type, payload) in after {
        push(&mut events, *event_type, payload.clone());
    }
    events
}

/// [`stopped`] with `late` appended behind the store, then `repair`: the
/// one shape every repair test starts from.
fn stopped_with(late: &[(EventType, Value)], repair: Value) -> Vec<EventEnvelope> {
    let mut events = stopped();
    for (event_type, payload) in late {
        push(&mut events, *event_type, payload.clone());
    }
    push(&mut events, EventType::OperatorCommanded, repair);
    events
}

/// The repair payload a verb writes for `first_seq`..=`last_seq`.
fn repaired(terminal_seq: u64, first_seq: u64, last_seq: u64, event_types: &[&str]) -> Value {
    repaired_by(terminal_seq, first_seq, last_seq, event_types, "operator")
}

/// [`repaired`], attributed to `operator`.
fn repaired_by(
    terminal_seq: u64,
    first_seq: u64,
    last_seq: u64,
    event_types: &[&str],
    operator: &str,
) -> Value {
    json!({
        "command_id": "c-repair",
        "command": REPAIR,
        "operator": operator,
        "args": {
            "first_seq": first_seq,
            "last_seq": last_seq,
            "terminal_seq": terminal_seq,
            "event_types": event_types,
            "reason": "kept",
        },
    })
}

/// A late settlement: shape (A) of #440.
fn settlement() -> (EventType, Value) {
    (
        EventType::EffectSucceeded,
        json!({"effect_id": "e", "attempt_id": "a", "result": {"result": "complete"}}),
    )
}

/// A late stop command: the first half of shape (B), as 0.12.0 wrote it.
fn commanded() -> (EventType, Value) {
    (
        EventType::OperatorCommanded,
        json!({"command_id": "c-stop", "command": "stop", "operator": "operator", "args": {}}),
    )
}

/// A lawful classify annotation, admitted after any terminal run.
fn classified() -> (EventType, Value) {
    (
        EventType::OperatorCommanded,
        json!({"command_id": "c-classify", "command": "classify", "operator": "operator", "args": {}}),
    )
}

/// Shape (B) of #440, as 0.12.0 wrote it: the stop command and the
/// acceptance that disposes of it after the conclusion.
fn shape_b() -> Vec<EventEnvelope> {
    stopped_with(
        &[
            commanded(),
            (
                EventType::OperatorAccepted,
                json!({"command_id": "c-stop", "operator": "operator", "reason": "stop it"}),
            ),
        ],
        repaired(5, 6, 7, &["operator/commanded", "operator/accepted"]),
    )
}

/// A command and its rejection after the conclusion, named as one tail.
fn multi_tail() -> Vec<EventEnvelope> {
    stopped_with(
        &[
            commanded(),
            (
                EventType::OperatorRejected,
                json!({"command_id": "c-stop", "operator": "operator",
                       "reason": "after_terminal"}),
            ),
        ],
        repaired(5, 6, 7, &["operator/commanded", "operator/rejected"]),
    )
}

/// A repair naming a range that is not the late tail.
fn wrong_range_tail() -> Vec<EventEnvelope> {
    stopped_with(&[settlement()], repaired(5, 7, 6, &["effect/succeeded"]))
}

/// A repair whose types lie about the carried prefix.
fn lying_types_tail() -> Vec<EventEnvelope> {
    stopped_with(
        &[classified(), settlement()],
        repaired(5, 6, 7, &["operator/commanded", "effect/failed"]),
    )
}

/// A repair whose types lie about seq 6, an event a carried fold has
/// already applied: the case that proves the carried fold checks the
/// prefix's types as the whole fold does. `lying_types_tail` lies about
/// seq 7, which no prefix through the repair can hold, so it exercises
/// only the suffix.
fn lying_carried_types_tail() -> Vec<EventEnvelope> {
    stopped_with(
        &[classified(), settlement()],
        repaired(5, 6, 7, &["effect/failed", "effect/succeeded"]),
    )
}

/// The honest shape L7 exists for: a lawful classify lands after the
/// terminal event, then a repair names it late. A carried account that
/// already applied the classify reads a moved ending; the whole one does
/// not.
fn late_classify_tail() -> Vec<EventEnvelope> {
    stopped_with(
        &[classified(), settlement()],
        repaired(5, 6, 7, &["operator/commanded", "effect/succeeded"]),
    )
}

/// A `repair` word standing before the terminal event: the pending
/// command it is, never a record.
fn pre_terminal_repair() -> Vec<EventEnvelope> {
    opened(&[
        (
            EventType::OperatorCommanded,
            repaired(3, 4, 4, &["effect/requested"]),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "work"}),
        ),
        (EventType::RunStopped, json!({"reason": "concluded"})),
    ])
}

/// Shape (A) of #440: a settlement landed after `run/stopped`. The run
/// does not fold; a repair naming that one event makes it fold as
/// stopped, with the tail kept and never applied.
#[test]
fn a_repaired_late_settlement_folds_as_stopped_and_changes_nothing() {
    let events = stopped_with(&[settlement()], repaired(5, 6, 6, &["effect/succeeded"]));
    assert_eq!(
        fold(&events[..6]).unwrap_err(),
        FoldError::AfterTerminal { seq: 6 },
        "an unrepaired late tail still fails, exactly as today"
    );
    let before = fold(&events[..5]).unwrap();

    let state = fold(&events).unwrap();
    let tail = state.late_tail.as_ref().expect("the repair is honoured");
    assert_eq!(tail.repaired_at, 7);
    assert_eq!(tail.terminal_seq, 5);
    assert_eq!((tail.first_seq, tail.last_seq), (6, 6));
    assert_eq!(tail.event_types, [EventType::EffectSucceeded]);
    assert_eq!(tail.operator, "operator");
    assert_eq!(
        tail.events().collect::<Vec<_>>(),
        [(6, EventType::EffectSucceeded)]
    );
    assert_eq!(state.run, Run::Stopped, "the terminal status stands");
    assert_eq!(state.seq, 7);
    // Every fact a late event would have moved is the terminal-only fold's:
    // the settlement is kept and shown, never applied.
    assert_eq!(state.last_result, before.last_result);
    assert_eq!(state.phase, before.phase);
    assert_eq!(state.consecutive_failures, before.consecutive_failures);
}

/// Shape (B) of #440, as 0.12.0 wrote it: an `operator/commanded` stop
/// and the acceptance that disposes of it landed after the conclusion.
#[test]
fn a_repaired_late_command_folds_as_stopped() {
    let events = shape_b();
    assert_eq!(
        fold(&events[..7]).unwrap_err(),
        FoldError::AfterTerminal { seq: 7 }
    );
    let state = fold(&events).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert_eq!(
        state.late_tail.unwrap().event_types,
        [EventType::OperatorCommanded, EventType::OperatorAccepted]
    );
}

/// A multi-event tail is named as one range, and every event in it folds
/// as late.
#[test]
fn a_multi_event_late_tail_is_named_as_one_range() {
    let events = multi_tail();
    let state = fold(&events).unwrap();
    let tail = state.late_tail.unwrap();
    assert_eq!((tail.first_seq, tail.last_seq), (6, 7));
    assert_eq!(
        tail.event_types,
        [EventType::OperatorCommanded, EventType::OperatorRejected]
    );
    // The rejection disposed of a command this run never took: the late
    // command was never applied, so nothing is pending.
    assert!(state.pending_command.is_none());
    assert_eq!(state.run, Run::Stopped);
}

/// A repair naming any range but exactly the late tail is refused by the
/// fold, in words that name it.
#[test]
fn a_repair_naming_another_range_is_refused_by_name() {
    let cases = [
        (
            repaired(5, 7, 6, &["effect/succeeded"]),
            "it names seqs 7..=6, but the late tail is seqs 6..=6",
        ),
        (
            repaired(4, 6, 6, &["effect/succeeded"]),
            "it names the terminal event at seq 4, but the run's terminal event is at seq 5",
        ),
        (
            repaired(5, 6, 6, &["effect/failed"]),
            "it names seq 6 as effect/failed, but the run holds effect/succeeded",
        ),
        (
            repaired(5, 5, 6, &["effect/succeeded"]),
            "it names seqs 5..=6, but the late tail is seqs 6..=6",
        ),
    ];
    for (payload, detail) in cases {
        let events = stopped_with(&[settlement()], payload);
        let refused = fold(&events).unwrap_err();
        assert_eq!(
            refused,
            FoldError::BadRepair {
                seq: 7,
                detail: detail.to_string()
            }
        );
        assert_eq!(
            refused.to_string(),
            format!("event 7: operator repair does not name the late tail: {detail}")
        );
    }
}

/// A repair whose `event_types` names a type the run does not hold, or
/// whose count is wrong, is refused as well.
#[test]
fn a_repair_whose_types_do_not_read_is_refused() {
    let events = stopped_with(
        &[settlement()],
        repaired(5, 6, 6, &["effect/succeeded", "effect/failed"]),
    );
    assert_eq!(
        fold(&events).unwrap_err(),
        FoldError::BadRepair {
            seq: 7,
            detail: "it names 2 event type(s), but the late tail holds 1".to_string()
        }
    );

    let events = stopped_with(&[settlement()], repaired(5, 6, 6, &["not-an-event"]));
    let refused = fold(&events).unwrap_err();
    let FoldError::BadRepair { seq, detail } = &refused else {
        panic!("not a repair refusal: {refused:?}");
    };
    assert_eq!(*seq, 7);
    assert!(
        detail.starts_with("its args do not read: unknown variant `not-an-event`"),
        "{detail}"
    );
}

/// A repair that stands before the run's terminal event is no repair
/// record: it is the pending command the fold has always read
/// `operator/commanded` as, and the run folds to its terminal status.
#[test]
fn a_repair_before_the_terminal_event_is_the_pending_command_it_is() {
    let events = pre_terminal_repair();
    let state = fold(&events).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert!(
        state.late_tail.is_none(),
        "a pre-terminal word names no tail"
    );
    assert_eq!(
        state.pending_command,
        Some(("c-repair".to_string(), REPAIR.to_string())),
        "the word is the command the run was given, never a repair record"
    );
}

/// The reserved annotation words are one list in core, and none of them
/// is a command the bridge may journal.
#[test]
fn the_reserved_annotation_words_are_not_bridge_commands() {
    for word in ["repair", "settle", "classify", "supersede"] {
        assert!(crate::fold::reserved_annotation(word), "{word} is reserved");
        assert_eq!(OperatorCommand::parse(word), None, "{word} is no verb");
    }
    assert!(!crate::fold::reserved_annotation("dance"));
    assert!(!crate::fold::reserved_annotation("stop"));
}

/// A repair carrying no args, or no operator, is refused by name: both
/// are what the published schema requires of the record.
#[test]
fn a_repair_without_args_or_operator_is_refused() {
    let no_args = stopped_with(
        &[settlement()],
        json!({"command_id": "c-repair", "command": REPAIR, "operator": "operator"}),
    );
    assert_eq!(
        fold(&no_args).unwrap_err(),
        FoldError::BadRepair {
            seq: 7,
            detail: "it carries no args".to_string()
        }
    );

    let no_operator = stopped_with(
        &[settlement()],
        json!({
            "command_id": "c-repair",
            "command": REPAIR,
            "args": {
                "first_seq": 6, "last_seq": 6, "terminal_seq": 5,
                "event_types": ["effect/succeeded"], "reason": "kept",
            },
        }),
    );
    assert_eq!(
        fold(&no_operator).unwrap_err(),
        FoldError::BadRepair {
            seq: 7,
            detail: "it carries no operator".to_string()
        }
    );
}

/// A repair that names a seq the run does not hold, because the carried
/// state has no event there and the suffix does not either, is refused
/// rather than read as a type it cannot check.
#[test]
fn a_repair_naming_an_event_the_run_does_not_hold_is_refused() {
    let state = fold(&stopped()).unwrap();
    let repair = EnvelopeBuilder::new(
        EventType::OperatorCommanded,
        repaired(5, 6, 6, &["effect/succeeded"]),
    )
    .run("r1")
    .seq(7)
    .event_id("e7")
    .at("2026-11-13T00:00:07Z")
    .previous("hash")
    .sealed();
    assert_eq!(
        fold_onto(state, &[repair]).unwrap_err(),
        FoldError::BadRepair {
            seq: 7,
            detail: "it names seq 6, which the run does not hold".to_string()
        }
    );
}

/// A repair immediately after the terminal event names no tail, and is
/// refused: repair exists for a run that does not fold.
#[test]
fn a_repair_that_names_no_tail_is_refused() {
    let events = stopped_with(&[], repaired(5, 6, 5, &[]));
    assert_eq!(
        fold(&events).unwrap_err(),
        FoldError::BadRepair {
            seq: 6,
            detail: "it names no late tail: seq 5 is the terminal event and seq 6 is the repair"
                .to_string()
        }
    );
}

/// A command word `repair` on a run that has not terminated is a pending
/// command, as every `operator/commanded` is, and no repair record: the
/// verb's own pre-check is what keeps one off a live run.
#[test]
fn a_repair_on_a_live_run_is_a_pending_command() {
    let events = opened(&[(
        EventType::OperatorCommanded,
        repaired(3, 4, 4, &["effect/requested"]),
    )]);
    let state = fold(&events).unwrap();
    assert_eq!(
        state.pending_command,
        Some(("c-repair".to_string(), REPAIR.to_string()))
    );
    assert!(state.late_tail.is_none());
}

/// Annotations after the repair are admitted as on any terminal run, and
/// do not move the tail.
#[test]
fn annotations_after_the_repair_are_admitted() {
    let mut events = stopped_with(&[settlement()], repaired(5, 6, 6, &["effect/succeeded"]));
    push(&mut events, classified().0, classified().1);
    let state = fold(&events).unwrap();
    assert_eq!(state.run, Run::Stopped);
    assert_eq!(state.late_tail.unwrap().last_seq, 6);
}

/// Every planted shape the two property tests read: (A), (B), the
/// multi-event tail, the wrong-range repair, the lying-types repairs
/// (suffix and carried prefix), the honest late classify and the
/// pre-terminal repair.
fn shapes() -> Vec<Vec<EventEnvelope>> {
    vec![
        stopped_with(&[settlement()], repaired(5, 6, 6, &["effect/succeeded"])),
        shape_b(),
        multi_tail(),
        wrong_range_tail(),
        lying_types_tail(),
        lying_carried_types_tail(),
        late_classify_tail(),
        pre_terminal_repair(),
    ]
}

/// The whole fold and the carried fold agree at every cut point over the
/// planted shapes. A cut after the terminal event but before the repair
/// exercises the carried types; the prefix-lying shape makes the cut at
/// seq 6 prove the carried fold reads the state's own types.
#[test]
fn the_whole_and_carried_folds_agree_at_every_cut_point() {
    for (at, events) in shapes().iter().enumerate() {
        let whole = fold(events);
        for split in 1..=events.len() {
            let Ok(prefix) = fold(&events[..split]) else {
                continue;
            };
            let carried = fold_onto(prefix, &events[split..]);
            assert_eq!(
                format!("{carried:?}"),
                format!("{whole:?}"),
                "shape {at} carried over split {split} is the whole fold"
            );
        }
    }
}

/// L7: the carried account equals the whole one, or says it must refold
/// whole. A suffix repair that names a carried event cannot be applied to
/// the account after the fact, so the signal is the only honest answer and
/// the refold is the whole account.
#[test]
fn the_whole_and_carried_accounts_agree_or_ask_for_a_whole_refold() {
    for (at, events) in shapes().iter().enumerate() {
        let whole = Account::whole("r1", events);
        for split in 1..=events.len() {
            let prefix = Account::whole("r1", &events[..split]);
            if prefix.state().is_err() {
                continue;
            }
            match prefix.onto(&events[split..]) {
                Carried::Account(carried) => assert_eq!(
                    format!("{carried:?}"),
                    format!("{whole:?}"),
                    "shape {at} carried over split {split} is the whole account"
                ),
                Carried::Refold => {
                    // The caller's contract: refold the whole journal,
                    // which is exactly `whole` above. The classify cut
                    // below proves this is the arm the signal takes.
                }
            }
        }
    }

    // The classify cut is the one the signal exists for: the prefix
    // through the lawful classify folds, and the suffix's repair names it.
    let events = late_classify_tail();
    let carried = Account::whole("r1", &events[..6]).onto(&events[6..]);
    assert!(
        matches!(carried, Carried::Refold),
        "a repair naming a carried classify asks for a whole refold"
    );
}

/// The tail a reader folded past carries across [`fold_onto`]: a repair
/// in the suffix is checked against the state, and the whole fold and the
/// carried one are one state.
#[test]
fn a_repair_in_the_suffix_carries_across_fold_onto() {
    let events = stopped_with(
        &[classified(), settlement()],
        repaired(5, 6, 7, &["operator/commanded", "effect/succeeded"]),
    );
    let whole = fold(&events).unwrap();
    let tail = whole.late_tail.clone().unwrap();
    assert_eq!((tail.first_seq, tail.last_seq), (6, 7));
    assert_eq!(
        tail.event_types,
        [EventType::OperatorCommanded, EventType::EffectSucceeded]
    );

    // The prefix through the lawful annotation folds, and the suffix it
    // is carried over holds the late event and the repair.
    let prefix = fold(&events[..6]).unwrap();
    let carried = fold_onto(prefix, &events[6..]).unwrap();
    assert_eq!(format!("{carried:?}"), format!("{whole:?}"));
}

/// A repair a carried fold reads still checks every type it names in the
/// suffix: the state's prefix is trusted, and the events after it are not.
#[test]
fn a_carried_repair_checks_the_types_it_names_in_the_suffix() {
    let events = stopped_with(
        &[classified(), settlement()],
        repaired(5, 6, 7, &["operator/commanded", "effect/failed"]),
    );
    let prefix = fold(&events[..6]).unwrap();
    assert_eq!(
        fold_onto(prefix, &events[6..]).unwrap_err(),
        FoldError::BadRepair {
            seq: 8,
            detail: "it names seq 7 as effect/failed, but the run holds effect/succeeded"
                .to_string()
        },
        "the suffix's type is read and compared"
    );
}

/// M2: a carried fold checks a type the state's prefix already applied,
/// not only the suffix's. Seq 6 is inside the carried prefix, so the
/// state's own `tail_types` must answer, and a lie there is refused with
/// the whole fold's words.
#[test]
fn a_carried_repair_checks_the_types_it_names_in_the_carried_prefix() {
    let events = lying_carried_types_tail();
    let refused = FoldError::BadRepair {
        seq: 8,
        detail: "it names seq 6 as effect/failed, but the run holds operator/commanded".to_string(),
    };
    assert_eq!(
        fold(&events).unwrap_err(),
        refused,
        "the whole fold refuses"
    );
    let prefix = fold(&events[..6]).unwrap();
    assert_eq!(
        fold_onto(prefix, &events[6..]).unwrap_err(),
        refused,
        "the carried fold refuses at the same cut with the same words"
    );
}
