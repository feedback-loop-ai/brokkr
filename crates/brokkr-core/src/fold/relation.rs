//! The fold's transition relation, written once (decision 0050, ruling
//! 6): for every event type, at every position a run can stand at, the
//! answer `apply` gives — the transition it takes, or the refusal or
//! annotation it is. `apply` reads the answer before anything else, so no
//! arm of the fold places an event by a wildcard.

use super::{terminal, Cursor, RunState};
use crate::envelope::EventType;

/// Where a run stands, as the relation reads it: the cursor's variant, or
/// `Terminal` once the run has completed or stopped, which the fold reads
/// before the cursor. The fold reaches `Idle` only by parking or by
/// concluding, so a run at `Idle` that is not terminal is parked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Position {
    Start,
    EnterPhase,
    RequestEffect,
    ExecuteEffect,
    EffectInFlight,
    Decide,
    Park,
    Stop,
    Idle,
    /// The last position: [`COLUMNS`] is typed by it.
    Terminal,
}

impl Position {
    pub(super) fn of(state: &RunState) -> Position {
        if terminal(state.status) {
            return Position::Terminal;
        }
        match state.cursor {
            Cursor::Start => Position::Start,
            Cursor::EnterPhase { .. } => Position::EnterPhase,
            Cursor::RequestEffect => Position::RequestEffect,
            Cursor::ExecuteEffect { .. } => Position::ExecuteEffect,
            Cursor::EffectInFlight { .. } => Position::EffectInFlight,
            Cursor::Decide { .. } => Position::Decide,
            Cursor::Park { .. } => Position::Park,
            Cursor::Stop => Position::Stop,
            Cursor::Idle => Position::Idle,
        }
    }
}

/// The transitions an event takes where the relation admits it, one per
/// event type that has one: `run/started` opens the state and takes none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Transition {
    Enter,
    Request,
    Start,
    Checkpoint,
    Succeed,
    Fail,
    Close,
    Rule,
    Command,
    Accept,
    Reject,
    Park,
    Complete,
    Stop,
}

/// The answer the fold gives an event at a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Arm {
    /// The event takes this transition. Its arm still checks what the
    /// event names against what the position holds (the phase entered,
    /// the open effect, the pending command) and refuses a mismatch as
    /// out of place.
    Moves(Transition),
    /// The event cannot happen here: [`super::FoldError::OutOfPlace`].
    OutOfPlace,
    /// An operator annotation on a terminal run: admitted, and it changes
    /// nothing.
    Annotates,
    /// Any other event on a terminal run: [`super::FoldError::AfterTerminal`].
    AfterTerminal,
}

/// One column per [`Position`], in declaration order.
const COLUMNS: usize = Position::Terminal as usize + 1;

/// The relation: one row per event type, in [`EventType`]'s declaration
/// order, and one column per [`Position`]. Its length is typed by the last
/// event type and each row's by the last position, so a missing row or
/// column does not compile; `the_relation_answers_every_pair_and_the_fold_answers_by_it`
/// states every pair's answer apart from this table, so a row out of order,
/// or a new event type, fails it. The columns are `Start`, `EnterPhase`,
/// `RequestEffect`, `ExecuteEffect`, `EffectInFlight`, `Decide`, `Park`,
/// `Stop`, `Idle` and `Terminal`. `m` is the row's transition, `O` out of
/// place, `A` an annotation and `T` after terminal.
#[rustfmt::skip]
const ROWS: [[Arm; COLUMNS]; EventType::RunStopped as usize + 1] = {
    use Arm::{AfterTerminal as T, Annotates as A, Moves, OutOfPlace as O};
    [
        //                                                            St EP RE EE IF De Pk Sp Id Te
        /* RunStarted */                                             [O, O, O, O, O, O, O, O, O, T],
        /* PhaseEntered */        { let m = Moves(Transition::Enter);      [m, m, O, O, O, O, O, O, O, T] },
        /* EffectRequested */     { let m = Moves(Transition::Request);    [O, O, m, O, O, O, O, O, O, T] },
        /* EffectStarted */       { let m = Moves(Transition::Start);      [O, O, O, m, O, O, O, O, O, T] },
        /* EffectCheckpointed */  { let m = Moves(Transition::Checkpoint); [O, O, O, O, m, O, O, O, O, T] },
        /* EffectSucceeded */     { let m = Moves(Transition::Succeed);    [O, O, O, O, m, O, O, O, O, T] },
        /* EffectFailed */        { let m = Moves(Transition::Fail);       [O, O, O, O, m, O, O, O, O, T] },
        // A requested effect never started is closed indeterminate in
        // crash recovery.
        /* EffectIndeterminate */ { let m = Moves(Transition::Close);      [O, O, O, m, m, O, O, O, O, T] },
        /* TransitionDecided */   { let m = Moves(Transition::Rule);       [O, O, O, O, O, m, O, O, O, T] },
        /* OperatorCommanded */   { let m = Moves(Transition::Command);    [m, m, m, m, m, m, m, m, m, A] },
        /* OperatorAccepted */    { let m = Moves(Transition::Accept);     [m, m, m, m, m, m, m, m, m, T] },
        /* OperatorRejected */    { let m = Moves(Transition::Reject);     [m, m, m, m, m, m, m, m, m, A] },
        // At ExecuteEffect when the seat's attempts are exhausted
        // (decision 0006); at Start and EnterPhase when a selector has
        // no strategy and no default.
        /* RunParked */           { let m = Moves(Transition::Park);       [m, m, O, m, O, O, m, O, O, T] },
        /* RunCompleted */        { let m = Moves(Transition::Complete);   [m, m, m, m, m, m, m, m, O, T] },
        /* RunStopped */          { let m = Moves(Transition::Stop);       [m, m, m, m, m, m, m, m, O, T] },
    ]
};

/// The answer for `event` at `position`.
pub(super) const fn arm(event: EventType, position: Position) -> Arm {
    ROWS[event as usize][position as usize]
}

/// The fields an event names its target by. The fold reads them before it
/// places the event, so an event that is both malformed and out of place
/// is refused for its payload.
pub(super) const fn named_first(event: EventType) -> &'static [&'static str] {
    use EventType::*;
    match event {
        PhaseEntered => &["phase"],
        EffectStarted => &["effect_id", "attempt_id"],
        EffectSucceeded | EffectFailed | EffectIndeterminate => &["effect_id"],
        RunStarted | EffectRequested | EffectCheckpointed | TransitionDecided
        | OperatorCommanded | OperatorAccepted | OperatorRejected | RunParked | RunCompleted
        | RunStopped => &[],
    }
}
