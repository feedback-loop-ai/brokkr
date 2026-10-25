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
use brokkr_core::summary::Account;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_view::{composed_view, run_view, Parts, RunView};

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
            head: (head.seq, &head.recorded_at),
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

#[test]
fn every_committed_journal_composed_from_its_trail_and_rows_is_its_run_view() {
    let (mut found, mut compared) = (Vec::new(), 0);
    for (path, events) in committed_journals::committed_journals() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (differ, kept) = disagreements(&name, &events);
        found.extend(differ);
        compared += usize::from(kept);
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 16);
}

#[test]
fn every_hand_built_journal_composed_from_its_trail_and_rows_is_its_run_view() {
    let (mut found, mut compared) = (Vec::new(), 0);
    for (name, events) in journals::journals() {
        let (differ, kept) = disagreements(name, &events);
        found.extend(differ);
        compared += usize::from(kept);
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 21);
}
