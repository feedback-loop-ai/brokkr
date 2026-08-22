//! Durable runtime: bundles, the engine loop, recovery, and operator
//! commands. Decision authority stays in forge-core's evaluator; this
//! crate only performs journaled effects around it.

pub mod bundle;
pub mod engine;

pub use bundle::{Bundle, CompileError, Seat, ENGINE_VERSION};
pub use engine::{operator_command, DriveEnd, Engine, EngineError};
