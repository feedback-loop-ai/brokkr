//! Durable runtime: bundles, the engine loop, recovery, and operator
//! commands. Decision authority stays in forge-core's evaluator; this
//! crate only performs journaled effects around it.

pub mod anchor;
pub mod bundle;
pub mod engine;

pub use bundle::{
    Aggregate, Bundle, CompileError, Confine, PanelMember, Seat, SeatBody, SequenceStep,
    StepBody, ENGINE_VERSION,
};
pub use anchor::{anchor, verify as verify_anchor, AnchorError};
pub use engine::{
    apply_fenced_operator_command, operator_command, DriveEnd, Engine, EngineError,
    FencedCommandOutcome,
};
