//! The deterministic heart of the engine. No I/O, no clock reads, no
//! randomness, no process execution (decision 0003, constitutional
//! boundary 1). Given the same journal and pinned bundle, fold and
//! evaluate always return the same state and ruling.

#![forbid(unsafe_code)]

pub mod canonical;
pub mod decision;
pub mod dispatch;
pub mod entry;
pub mod envelope;
pub mod event_index;
pub mod fold;
pub mod gate_moved_head;
pub mod keep_refs;
pub mod page;
mod payload;
pub mod policy;
pub mod realms;
pub mod residual;
pub mod seat;
pub mod stamp;
pub mod summary;

pub use envelope::{EventEnvelope, EventType};
pub use fold::{fold, Cursor, FoldError, RunState, Status};
pub use keep_refs::cited_shas;
pub use policy::{Machine, Outcome, PolicyError};

// The workspace's shared test support names this crate as every other
// crate's tests do (#357).
#[cfg(test)]
extern crate self as brokkr_core;
#[cfg(test)]
#[path = "../tests/committed_journals.rs"]
mod committed_journals;
#[cfg(test)]
#[path = "../../../tests/support/contract_lifecycle.rs"]
mod contract_lifecycle;
#[cfg(test)]
#[path = "../../../tests/support/envelope.rs"]
mod envelope_builder;
#[cfg(test)]
#[path = "../../../tests/support/run_state.rs"]
mod run_state;

#[cfg(test)]
#[path = "../../../tests/support/entry_records.rs"]
mod entry_records;
