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

use crate::envelope_builder;

#[path = "../../brokkr-core/tests/committed_journals.rs"]
mod committed_journals;
#[path = "../../../tests/support/seat_rows.rs"]
mod journals;
#[path = "../../../tests/support/seat_facts.rs"]
mod seat_facts;

/// What `seat_facts::disagreements` finds of `run`'s account read whole.
fn disagreements(run: &str, events: &[EventEnvelope]) -> (Vec<String>, usize) {
    seat_facts::disagreements(run, events, Account::whole(run, events).seats())
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
    assert_eq!(compared, 46);
}

/// An effect asked again while the attempt a start opened before its
/// first request is in flight: its phases' steps and tags and its visits'
/// models are what the rows keep, the attempt's terminal read open per
/// effect, as the dashboard reads it, and not per request. The view counts
/// the two requests as one participant where the rows count two, so only
/// the lists are compared.
#[test]
fn an_effect_asked_again_lists_as_its_run_view() {
    let events = journals::journal(journals::unasked_events(true));
    let rows = Account::whole("again", &events);
    let view = brokkr_view::run_view(&events, None);
    let (differ, compared) = seat_facts::lists_differ("again", &events, rows.seats(), &view);
    assert_eq!(differ, Vec::<String>::new());
    assert_eq!(compared, 4);
}
