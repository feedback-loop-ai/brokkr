//! The run view composed from what the writer keeps is the run view of the
//! run read whole (#753; decision 0089 slice 2b's acceptance, at the
//! view's level): for every committed journal and every hand-built seat
//! journal whose rows keep its live block, its trail, its account's seat
//! rows and each page of 50 of its journal, with what caused the page's
//! rows and its head, compose what `run_view` shows of it, but each
//! participant's checkpoints and the journal's other pages. A journal
//! whose rows do not keep it is read whole by the store's read. Each count
//! is of the journals compared.

use std::collections::BTreeMap;

use brokkr_core::fold::fold;
use brokkr_core::seat::row::{facts, Member};
use brokkr_core::summary::Account;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_view::{composed_view, run_view, Parts, RunView};
use serde_json::{json, Value};

use crate::{committed_journals, journals};

#[path = "../../../tests/support/composed_view.rs"]
mod composed;

/// The rows a journal page holds (ruling 7).
const PAGE: usize = 50;

/// The run view of `events` composed a page at a time from its trail and
/// `rows`, as the store's read hands them over, its fold read whole.
fn composed_pages(
    events: &[EventEnvelope],
    rows: &[brokkr_core::seat::row::SeatRow],
) -> Vec<RunView> {
    let trail: Vec<EventEnvelope> = events
        .iter()
        .filter(|event| event.event_type != EventType::EffectCheckpointed)
        .cloned()
        .collect();
    let ids: BTreeMap<&str, u64> = events
        .iter()
        .map(|event| (event.event_id.as_str(), event.seq))
        .collect();
    let state = fold(events).ok();
    let head = events.last().unwrap();
    let compose = |page: &[EventEnvelope]| {
        let caused = page
            .iter()
            .filter_map(|event| event.causation_id.as_deref());
        let causes: BTreeMap<String, u64> = caused
            .filter_map(|id| Some((id.to_string(), *ids.get(id)?)))
            .collect();
        let parts = Parts {
            trail: &trail,
            seats: rows,
            page,
            causes: &causes,
            head_seq: head.seq,
            last_at: &head.recorded_at,
        };
        composed_view(&parts, state.as_ref()).unwrap()
    };
    events.chunks(PAGE).map(compose).collect()
}

/// What composing `events` page by page shows otherwise than `run_view`
/// of them read whole, and whether its rows keep its live block, so that
/// it was composed at all.
fn disagreements(run: &str, events: &[EventEnvelope]) -> (Vec<String>, bool) {
    let account = Account::whole(run, events);
    if !account.seats_kept() {
        return (Vec::new(), false);
    }
    let whole = run_view(events, fold(events).ok().as_ref());
    let pages = composed_pages(events, account.seats());
    (composed::disagreements(run, &whole, &pages), true)
}

/// What composing each of `journals` shows otherwise than its run view,
/// and how many were composed.
fn compared<N: AsRef<str>>(
    journals: impl IntoIterator<Item = (N, Vec<EventEnvelope>)>,
) -> (Vec<String>, usize) {
    let (mut found, mut compared) = (Vec::new(), 0);
    for (name, events) in journals {
        let (differ, kept) = disagreements(name.as_ref(), &events);
        found.extend(differ);
        compared += usize::from(kept);
    }
    (found, compared)
}

#[test]
fn every_committed_journal_composed_from_its_trail_and_rows_is_its_run_view() {
    let journals = committed_journals::committed_journals();
    let (found, compared) = compared(journals.into_iter().map(|(path, events)| {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        (name, events)
    }));
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 16);
}

#[test]
fn every_hand_built_journal_composed_from_its_trail_and_rows_is_its_run_view() {
    let (found, compared) = compared(journals::journals());
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 21);
}

/// A seat with no model of its own whose member states one in a finished
/// step that names no step, so its phase draws the seat's node: the node
/// shows the seat's own model, which it has none of, never its members'.
fn joined() -> Vec<EventEnvelope> {
    let finished = json!({"member": "m", "step": "sequence-step-finished", "model": "opus"});
    journals::journal(vec![
        (EventType::PhaseEntered, json!({"phase": "implement"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "s1", "seat": "solo", "phase": "implement"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "s1", "attempt_id": "a1"}),
        ),
        journals::checkpoint("s1", "a1", finished),
    ])
}

/// A panel retried, its member's first attempt pinning its failure and its
/// retry's finish giving its outcome as an array of one status word, which
/// pins nothing: the retry's outcome stands for its node, and its node
/// shows the model its retry stated, the last.
fn unpinned() -> Vec<EventEnvelope> {
    let finished = |outcome: Value, model: &str| {
        json!({"member": "a", "step": "panel-member-finished", "outcome": outcome,
               "model": model})
    };
    let attempt = |attempt: &str| json!({"effect_id": "p1", "attempt_id": attempt});
    journals::journal(vec![
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "p1", "seat": "panel", "phase": "review"}),
        ),
        (EventType::EffectStarted, attempt("c1")),
        journals::checkpoint("p1", "c1", finished(json!("failed"), "m1")),
        (EventType::EffectFailed, attempt("c1")),
        (EventType::EffectStarted, attempt("c2")),
        journals::checkpoint("p1", "c2", finished(json!(["failed"]), "m2")),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "p1", "attempt_id": "c2", "result": {"result": "pass"}}),
        ),
    ])
}

#[test]
fn a_seat_s_members_model_and_a_member_s_unpinned_finish_compose_as_read_whole() {
    let (found, compared) = compared([("joined", joined()), ("unpinned", unpinned())]);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 2);
}

/// An effect whose id is `e:m`, hired solo, beside effect `e`'s member
/// `m`: two participants the seat rows keep apart, whose node keys are one,
/// so the whole read draws them as one.
fn colliding() -> Vec<EventEnvelope> {
    let member = json!({"member": "m", "step": "panel-member-finished", "outcome": "passed"});
    journals::journal(vec![
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e:m", "seat": "solo", "phase": "review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e:m", "attempt_id": "a1"}),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "panel", "phase": "review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a2"}),
        ),
        journals::checkpoint("e", "a2", member),
    ])
}

#[test]
fn two_participants_whose_node_keys_are_one_are_not_composed() {
    let events = colliding();
    let account = Account::whole("colliding", &events);
    assert!(account.seats_kept());
    let keys: Vec<(String, Member)> = facts(account.seats())
        .into_iter()
        .map(|seat| (seat.effect_id, seat.member))
        .collect();
    let (solo, panel) = ("e:m".to_string(), "e".to_string());
    let named = Member::Named("m".to_string());
    assert_eq!(
        keys,
        [
            (solo, Member::Seat),
            (panel.clone(), Member::Seat),
            (panel, named)
        ]
    );
    let trail = events.clone();
    let parts = Parts {
        trail: &trail,
        seats: account.seats(),
        page: &events,
        causes: &BTreeMap::new(),
        head_seq: 6,
        last_at: "2026-01-01T00:00:06Z",
    };
    let state = fold(&events).ok();
    let composed = composed_view(&parts, state.as_ref());
    assert_eq!(composed.map(|view| view.event_count), None);
}
