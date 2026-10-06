//! One handler per CLI verb (decision 0071 ruling 10: a verb is a `Cmd`
//! variant plus a handler). `run_with` matches the parsed command
//! exhaustively and hands each variant's arguments to its handler here;
//! nothing in this module decides which verb runs.
//!
//! The handlers are grouped by what they do to a run:
//! - [`delivery`] starts, continues or ends one;
//! - [`queue`] keeps the runs waiting to start (decision 0068);
//! - [`readouts`] reads journals and the world, and writes nothing;
//! - [`exchange`] moves a run's evidence across a machine's edge: anchors,
//!   exports, imports and the Looper bridge;
//! - [`setup`] prepares the workspace, its libraries and its drivers;
//! - [`probe`] measures an agent CLI outside every run.

pub(super) mod delivery;
pub(super) mod exchange;
pub(super) mod probe;
pub(super) mod queue;
pub(super) mod readouts;
pub(super) mod setup;

#[cfg(test)]
mod tests;
