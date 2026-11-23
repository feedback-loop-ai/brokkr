//! What an awaited entry has come to, and whether a wait on it holds
//! (decision 0068 ruling 2): the pass's reading, and its claim's. A run an
//! older brokkr left with a row and no events, which a crash between its
//! two writes made before a run's start was one transaction (decision 0095
//! ruling 4), is an unstarted run: it spawned nothing, and the entry that
//! waits on it is held, never refused as a run that does not fold.

use std::fmt;

use brokkr_core::ending::{closing, Ending};
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, Status};
use brokkr_store::{EntryId, EntryState, Store, Wait, WaitOn};

use super::{AdmissionError, Reason};

/// What an awaited entry has come to.
#[derive(Debug)]
pub(super) enum Awaited {
    /// Queued, held or launching: no run linked yet.
    Unstarted,
    Dropped,
    /// It started this run, whose row holds no event.
    UnstartedRun(String),
    /// It started this run, which stands so, and has ended so once it is
    /// over (decision 0064's `closing`, read afresh on every reading).
    Run {
        run: String,
        status: Status,
        ending: Option<Ending>,
    },
}

/// Read what `entry` has come to: its standing, and its run's status.
pub(super) fn observe(store: &Store, entry: EntryId) -> Result<Awaited, AdmissionError> {
    let state = store.queue_entry(entry)?.state;
    let events = match state.run() {
        Some(run) => store.load(run)?,
        None => Vec::new(),
    };
    came_to(state, &events)
}

/// Whether a run holding `events` is an unstarted run, a row with no
/// event: the one reading of it, the census's and an awaited entry's.
pub(super) fn never_started(events: &[EventEnvelope]) -> bool {
    events.is_empty()
}

/// What an entry standing so has come to, `events` its run's when it has
/// one: the pass's reading, and its claim's.
pub(super) fn came_to(
    state: EntryState,
    events: &[EventEnvelope],
) -> Result<Awaited, AdmissionError> {
    Ok(match state {
        EntryState::Queued | EntryState::Held | EntryState::Launching => Awaited::Unstarted,
        EntryState::Dropped => Awaited::Dropped,
        EntryState::Claimed { run } if never_started(events) => Awaited::UnstartedRun(run),
        EntryState::Claimed { run } => {
            let state = match fold(events) {
                Ok(state) => state,
                Err(source) => return Err(AdmissionError::Fold { run, source }),
            };
            let ending = closing(events, &state).map(|closing| closing.ended.ending);
            Awaited::Run {
                run,
                status: state.run.status(),
                ending,
            }
        }
    })
}

/// Why `wait` does not hold of what its entry came to, or `None` when it
/// holds.
pub(super) fn awaiting(wait: Wait, awaited: Awaited) -> Option<Reason> {
    use WaitOn::{Completed, Ended, Named};
    match (awaited, wait.on) {
        (Awaited::Unstarted, Completed | Ended | Named(_)) => Some(Reason::NotStarted(wait)),
        (Awaited::Dropped, Completed | Ended | Named(_)) => Some(Reason::Dropped(wait)),
        (Awaited::UnstartedRun(run), Completed | Ended | Named(_)) => Some(Reason::UnstartedRun {
            wait: Some(wait),
            run,
        }),
        (
            Awaited::Run {
                status: Status::Completed,
                ..
            },
            Completed | Ended,
        )
        | (
            Awaited::Run {
                status: Status::Stopped,
                ..
            },
            Ended,
        ) => None,
        (
            Awaited::Run {
                run,
                status: Status::Stopped,
                ..
            },
            Completed,
        ) => Some(Reason::Stopped { wait, run }),
        (
            Awaited::Run {
                run,
                status: Status::Running | Status::AwaitingOperator,
                ..
            },
            Completed | Ended,
        ) => Some(Reason::Unended { wait, run }),
        (Awaited::Run { run, ending, .. }, Named(named)) => named_ending(wait, run, ending, named),
    }
}

/// Why a wait on the `named` ending does not hold of a run that has
/// `ending` once it is over, or `None` when it holds: a run not yet over
/// has not ended, and one over that ended otherwise holds the entry until
/// the operator classifies it so, drops or re-queues.
fn named_ending(wait: Wait, run: String, ending: Option<Ending>, named: Ending) -> Option<Reason> {
    match ending {
        None => Some(Reason::Unended { wait, run }),
        Some(ending) if ending == named => None,
        Some(ending) => Some(Reason::EndedOtherwise { wait, run, ending }),
    }
}

/// [`Reason::EndedOtherwise`]'s text, beside the judgement that finds it,
/// in its neighbours' form: `never` is theirs.
pub(super) fn otherwise(
    f: &mut fmt::Formatter<'_>,
    never: &str,
    wait: &Wait,
    run: &str,
    ending: &Ending,
) -> fmt::Result {
    let ending = ending.word();
    write!(
        f,
        "waits on {wait}, {never}: its run '{run}' ended {ending}"
    )
}
