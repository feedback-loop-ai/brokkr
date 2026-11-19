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
//! result passes; so is one whose nested result reports a finding, which
//! belongs only to a verdict (fix round 0109a1r5, V4).

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

    /// The marker's `step` word, read only in this module (fix round
    /// 0109a1r5, V6).
    const fn word(self) -> &'static str {
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

    /// The member `checkpoint` marks as settled: the engine's own
    /// panel-member-finished marker naming it with outcome `succeeded`.
    /// It is the one mark a settled member's payload is read as a verdict
    /// by, beside it in the same attempt, wherever it is read: by inspect
    /// and by a partial retry's carry alike (fix rounds 0109a1r5, V2, and
    /// 0109a1r6, W2).
    pub fn settled_member(checkpoint: &Value) -> Option<&str> {
        (EngineMarker::authored(checkpoint) == Some(EngineMarker::MemberFinished)).then_some(())?;
        let succeeded = checkpoint.get("outcome").and_then(Value::as_str) == Some("succeeded");
        succeeded.then_some(())?;
        checkpoint.get("member").and_then(Value::as_str)
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
    #[error(
        "a driver's checkpoint carries a finding: findings belong only to the seat's result, \
         its verdict (decision 0109 ruling 1)"
    )]
    Finding,
}

/// Admit a driver's live `checkpoint` written at `reporter`: one that
/// carries `authored` or names an engine marker's word is refused, and a
/// nested `result` passes [`admit`] as a standalone result does. A live
/// checkpoint is telemetry, never a verdict, so a nested result that
/// reports a finding is refused rather than stamped: no keyed finding
/// enters the journal outside a verdict (fix round 0109a1r5, V4).
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
    match admit(result, reporter)? {
        Some(findings) if !findings.is_empty() => Err(LiveRefusal::Finding),
        // An empty set reports nothing, and is journaled as it was sent.
        Some(none) => {
            journal(result, &none);
            Ok(())
        }
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
