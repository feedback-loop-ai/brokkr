//! The checkpoints the engine itself authors beside a verdict, and the
//! one admission a driver's live checkpoint passes (decision 0109 rulings
//! 1 and 3, fix round 0109a1r4, K1).
//!
//! The engine journals a sequence step's verdict and a panel member's
//! finish as checkpoints of its own. Each carries `authored: "engine"`, a
//! field seat-record v6's closed checkpoint refuses and the live sink
//! refuses from a driver, so it is the provenance a reader admits a
//! verdict by, never the marker's word. A driver's live checkpoint that
//! claims either, or whose nested `result` writes what only the engine
//! writes, is refused by [`admit_live`], the same admission a standalone
//! result passes.

use serde_json::Value;
use thiserror::Error;

use super::{admit, journal, AdmissionRefusal, Reporter};

/// The checkpoint field that says the engine authored the record.
pub const AUTHORED: &str = "authored";

/// `authored`'s one value.
const BY_THE_ENGINE: &str = "engine";

/// A checkpoint the engine authors beside a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMarker {
    /// A sequence step before the last finished, or a gate step ended the
    /// sequence (fix round 0109a1r5, V1), its result beside it.
    StepFinished,
    /// A panel member finished, with the site its findings carry.
    MemberFinished,
}

impl EngineMarker {
    const ALL: [EngineMarker; 2] = [EngineMarker::StepFinished, EngineMarker::MemberFinished];

    /// The marker's `step` word.
    pub const fn word(self) -> &'static str {
        match self {
            EngineMarker::StepFinished => "sequence-step-finished",
            EngineMarker::MemberFinished => "panel-member-finished",
        }
    }

    /// `fields` as this marker: its `step` word and the engine's
    /// authorship written over whatever `fields` held.
    pub fn record(self, mut fields: Value) -> Value {
        fields["step"] = Value::from(self.word());
        fields[AUTHORED] = Value::from(BY_THE_ENGINE);
        fields
    }

    /// The marker `checkpoint` is, read only where the engine authored it:
    /// a record that names a marker's word without that provenance is no
    /// marker.
    pub fn authored(checkpoint: &Value) -> Option<EngineMarker> {
        if checkpoint.get(AUTHORED).and_then(Value::as_str) != Some(BY_THE_ENGINE) {
            return None;
        }
        let word = checkpoint.get("step").and_then(Value::as_str)?;
        EngineMarker::ALL
            .into_iter()
            .find(|marker| marker.word() == word)
    }
}

/// Why a driver's live checkpoint is refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LiveRefusal {
    #[error(
        "a driver's checkpoint claims a record only the engine authors \
         (decision 0109 ruling 1)"
    )]
    EngineAuthored,
    #[error("a driver's checkpoint result was refused: {0}")]
    Result(#[from] AdmissionRefusal),
}

/// Admit a driver's live `checkpoint` written at `reporter`: one that
/// carries `authored` or names an engine marker's word is refused, and a
/// nested `result` passes [`admit`] as a standalone result does, its
/// findings stamped in place.
pub fn admit_live(
    checkpoint: &mut Value,
    reporter: Option<Reporter<'_>>,
) -> Result<(), LiveRefusal> {
    let word = checkpoint.get("step").and_then(Value::as_str);
    let claims = |marker: EngineMarker| word == Some(marker.word());
    if checkpoint.get(AUTHORED).is_some() || EngineMarker::ALL.into_iter().any(claims) {
        return Err(LiveRefusal::EngineAuthored);
    }
    let Some(result) = checkpoint.get_mut("result") else {
        return Ok(());
    };
    if let Some(findings) = admit(result, reporter)? {
        journal(result, &findings);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
