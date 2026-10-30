//! Durable runtime: bundles, the engine loop, recovery, and operator
//! commands. Decision authority stays in brokkr-core's evaluator; this
//! crate only performs journaled effects around it.

#![forbid(unsafe_code)]

pub mod admission;
pub mod agents;
pub mod anchor;
pub mod argv;
pub mod boundary;
pub mod bundle;
pub mod capabilities;
pub mod dialect;
mod dispatch;
pub mod engine;
pub mod keep_refs;
pub mod launch;
mod lock_file;
mod phase_name;
pub mod places;
pub mod realms;

pub use agents::{
    report as report_agent, resolve as resolve_agent, resolve_route, Adapters, Availability,
    Candidate, EgressClass, HarnessHands, Library, LibraryError, Presence, ResolveError,
    ResultDoor, TrustTier,
};
pub use anchor::{anchor, verify as verify_anchor, AnchorError};
pub use bundle::compose::Ancestor;
pub use bundle::{
    Aggregate, Bundle, CompileError, Invocation, PanelMember, Seat, SeatBody, SeatClass,
    SequenceStep, StepBody, ENGINE_VERSION,
};
pub use dispatch::{reconcile, take_dispatch, Dispatcher, DispatcherError, Settled};
pub use engine::{
    apply_fenced_operator_command, conclude, git_head, operator_command, operator_supersede,
    CarryRefusal, CommandWord, ConcludeRefusal, DriveEnd, Engine, EngineError, FencedCommand,
    FencedCommandOutcome, MemberRetryOutcome, Supersede,
};
pub use keep_refs::{
    delete as delete_keep_refs, list as list_keep_refs, plant as plant_keep_refs, plant_or_report,
    KeepRefsError, Planted,
};
pub use realms::{World, WorldError};

// The workspace's shared test support (#357).
#[cfg(test)]
#[path = "../../../tests/support/contract_lifecycle.rs"]
mod contract_lifecycle;
#[cfg(test)]
#[path = "../../../tests/support/dispatch.rs"]
mod dispatch_fixture;
#[cfg(test)]
#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
#[cfg(test)]
#[path = "../../../tests/support/run_state.rs"]
mod run_state;

#[cfg(test)]
#[path = "../../../tests/support/entry_records.rs"]
mod entry_records;
