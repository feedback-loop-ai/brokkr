//! Where a run stands, as one of the four states the fold reaches (#345):
//! running at a cursor, parked for the operator with a reason, or
//! concluded either way. The status, the cursor and the park reason are
//! read from it, so a combination the fold never produces cannot be held.

use std::fmt;

use super::{Cursor, Status};
use crate::gate_moved_head;

#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// The engine acts at the cursor.
    Running(Cursor),
    /// Parked: nothing moves until the operator answers the reason.
    AwaitingOperator {
        reason: ParkReason,
    },
    Completed,
    Stopped,
}

impl Run {
    /// The word-bearing status this state is.
    pub const fn status(&self) -> Status {
        match self {
            Run::Running(_) => Status::Running,
            Run::AwaitingOperator { .. } => Status::AwaitingOperator,
            Run::Completed => Status::Completed,
            Run::Stopped => Status::Stopped,
        }
    }

    /// The cursor of a running run; a parked or concluded run has none.
    pub fn cursor(&self) -> Option<&Cursor> {
        match self {
            Run::Running(cursor) => Some(cursor),
            Run::AwaitingOperator { .. } | Run::Completed | Run::Stopped => None,
        }
    }

    /// Why a parked run waits. A running run at [`Cursor::Park`] has not
    /// parked yet, so its reason is not answered here.
    pub fn park_reason(&self) -> Option<&ParkReason> {
        match self {
            Run::AwaitingOperator { reason } => Some(reason),
            Run::Running(_) | Run::Completed | Run::Stopped => None,
        }
    }
}

/// The run as readouts and refusals name its position: a running run's
/// cursor, and `Idle` for a run with nothing to do without an operator.
impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Run::Running(cursor) => fmt::Display::fmt(cursor, f),
            Run::AwaitingOperator { .. } | Run::Completed | Run::Stopped => f.write_str("Idle"),
        }
    }
}

/// Why a run parks: the gate that moved the operated repo's HEAD, which
/// the engine answers apart, or the reason as it was stated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParkReason {
    /// [`gate_moved_head::PREFIX`], exactly.
    GateMovedHead,
    Stated(String),
}

impl ParkReason {
    /// The reason a park's text names: the one way to read it from text.
    pub fn read(text: String) -> ParkReason {
        match text == gate_moved_head::PREFIX {
            true => ParkReason::GateMovedHead,
            false => ParkReason::Stated(text),
        }
    }

    /// The text the reason is journaled and shown as.
    pub fn as_str(&self) -> &str {
        match self {
            ParkReason::GateMovedHead => gate_moved_head::PREFIX,
            ParkReason::Stated(text) => text,
        }
    }
}
