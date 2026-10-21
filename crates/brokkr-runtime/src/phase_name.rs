//! The phase names the compiler and the engine compare against, each
//! spelled once (#349, ask 5). A dialect's five phases are
//! [`DialectPhase`](crate::dialect::DialectPhase)'s; these are the
//! offices beside them. A name here is still only a name: which phase
//! ships, verifies or is returned to is not yet a compiled role of the
//! bundle, the way `protected_phase` is, and promoting one waits on a
//! decision.

pub(crate) const INTAKE: &str = "intake";
pub(crate) const TRIAGE: &str = "triage";
pub(crate) const IMPLEMENT: &str = "implement";
pub(crate) const REVIEW: &str = "review";
pub(crate) const VERIFY: &str = "verify";
pub(crate) const SHIP: &str = "ship";

/// Sources read by nearest_change and entry admission (decision 0094).
pub(crate) const CHANGE_SOURCES: [&str; 5] = ["tasks", "design", "specify", TRIAGE, INTAKE];
