//! How a run's projections disagree with its fold (ruling 4), and the
//! words `brokkr doctor` names each by.

use crate::event_index::{place, IndexFault};
use crate::{seat_summary, StoreError};

/// How a row stands against its run's fold (ruling 4): a row that stands
/// at its head is a judgement of that head, so one that is not its fold's
/// is a bug or an edit, never a lag.
#[derive(Debug)]
pub enum Disagreement {
    /// The run has no summary row at all.
    Missing { run_id: String },
    /// The row stands at its head, and `column` is the first that is not
    /// what the run's fold gives it.
    Column {
        run_id: String,
        column: &'static str,
    },
    /// The run does not load, so no fold stands behind its row, whether
    /// the row stands at its head or behind it.
    Unread { run_id: String, error: StoreError },
    /// The event index is not what the run's events project to, first at
    /// `seq`: a trigger's row never lags, so wherever the summary row
    /// stands, this is a bug or an edit.
    Indexed {
        run_id: String,
        seq: u64,
        fault: IndexFault,
    },
    /// The run's seat rows stand at its head, and are not its fold's,
    /// first at the place `row`, where the projection opened it.
    Seated {
        run_id: String,
        row: u64,
        fault: IndexFault,
    },
    /// The seat summary holds a seat head of a run the journal does not
    /// hold.
    SeatHead { run_id: String },
}

/// The words `brokkr doctor` names a disagreement by.
impl std::fmt::Display for Disagreement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Disagreement::Missing { run_id } => write!(f, "run '{run_id}' has no summary row"),
            Disagreement::Column { run_id, column } => write!(
                f,
                "run '{run_id}': its summary row stands at its head, and its {column} is not \
                 its fold's"
            ),
            Disagreement::Unread { run_id, error } => {
                write!(f, "run '{run_id}' does not load: {error}")
            }
            Disagreement::Indexed { run_id, seq, fault } => match fault {
                IndexFault::Missing => {
                    write!(f, "run '{run_id}': its event {seq} has no event index row")
                }
                IndexFault::Column(column) => write!(
                    f,
                    "run '{run_id}': the event index row of its event {seq} is not the event's \
                     in its {column}"
                ),
                IndexFault::Stray => write!(
                    f,
                    "run '{run_id}': the event index holds a row at {}, where the run holds \
                     no event",
                    place(*seq)
                ),
            },
            Disagreement::Seated { run_id, row, fault } => {
                seat_summary::named(f, run_id, *row, *fault)
            }
            Disagreement::SeatHead { run_id } => seat_summary::headless(f, run_id),
        }
    }
}
