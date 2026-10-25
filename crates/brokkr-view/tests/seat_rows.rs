//! The writer's seat rows, grouped by participant, read as the run view's
//! participants (#753; decision 0089 ruling 4 at the projection's level):
//! for every committed journal and every hand-built seat journal, the
//! facts `brokkr_core::seat::row::facts` derives from the rows of the
//! run's account equal what `run_view` shows of each participant, field
//! for field, and each phase's inner columns and each visit's models equal
//! what the rows list; a field that differs is named with its run. Each
//! count is of the participants, phases and visits compared.

use brokkr_core::summary::Account;
use brokkr_core::EventEnvelope;

use crate::{committed_journals, journals};

#[path = "../../../tests/support/seat_facts.rs"]
mod seat_facts;

/// What `seat_facts::disagreements` finds of `run`'s account read whole.
fn disagreements(run: &str, events: &[EventEnvelope]) -> (Vec<String>, usize) {
    let account = Account::whole(run, events);
    seat_facts::disagreements(run, events, account.seats(), account.seats_kept())
}

#[test]
fn every_committed_journal_s_seat_rows_read_as_its_participants() {
    let mut found = Vec::new();
    let mut compared = 0;
    for (path, events) in committed_journals::committed_journals() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let (differ, count) = disagreements(&name, &events);
        found.extend(differ);
        compared += count;
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 212);
}

#[test]
fn every_hand_built_journal_s_seat_rows_read_as_its_participants() {
    let mut found = Vec::new();
    let mut compared = 0;
    for (name, events) in journals::journals() {
        let (differ, count) = disagreements(name, &events);
        found.extend(differ);
        compared += count;
    }
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(compared, 68);
}

/// The rows of a run whose live start names no effect or one never asked
/// do not keep its live block, so they read stale and the run view is read
/// whole; every other journal's rows keep it, every committed one's too.
#[test]
fn only_a_live_start_no_row_is_of_leaves_its_rows_unkept() {
    let committed = committed_journals::committed_journals();
    let committed = committed
        .into_iter()
        .map(|(_, events)| ("committed", events));
    let unkept: Vec<&str> = journals::journals()
        .into_iter()
        .chain(committed)
        .filter(|(_, events)| !Account::whole("run", events).seats_kept())
        .map(|(name, _)| name)
        .collect();
    assert_eq!(unkept, ["effectless", "never asked"]);
}
