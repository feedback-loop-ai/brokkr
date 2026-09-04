//! Durable runtime: bundles, the engine loop, recovery, and operator
//! commands. Decision authority stays in brokkr-core's evaluator; this
//! crate only performs journaled effects around it.

pub mod agents;
pub mod anchor;
pub mod bundle;
pub mod engine;
pub mod keep_refs;
pub mod realms;

pub use agents::{
    report as report_agent, resolve as resolve_agent, resolve_route, Adapters, Availability,
    Candidate, EgressClass, Library, LibraryError, Presence, ResolveError, TrustTier,
};
pub use anchor::{anchor, verify as verify_anchor, AnchorError};
pub use bundle::compose::Ancestor;
pub use bundle::{
    Aggregate, Bundle, CompileError, Confine, PanelMember, Seat, SeatBody, SeatClass, SequenceStep,
    StepBody, ENGINE_VERSION,
};
pub use engine::{
    apply_fenced_operator_command, conclude, git_head, operator_command, DriveEnd, Engine,
    EngineError, FencedCommandOutcome, LOST_FENCE,
};
pub use keep_refs::{
    delete as delete_keep_refs, list as list_keep_refs, plant as plant_keep_refs, plant_or_report,
    KeepRefsError, Planted,
};
pub use realms::{World, WorldError};
