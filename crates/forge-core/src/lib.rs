//! The deterministic heart of the engine. No I/O, no clock reads, no
//! randomness, no process execution (decision 0003, constitutional
//! boundary 1). Given the same journal and pinned bundle, fold and
//! evaluate always return the same state and ruling.

pub mod canonical;
pub mod dispatch;
pub mod envelope;
pub mod fold;
pub mod policy;
pub mod realms;

pub use envelope::{EventEnvelope, EventType};
pub use fold::{fold, Cursor, FoldError, RunState, Status};
pub use policy::{Machine, Outcome, PolicyError};
