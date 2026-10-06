//! `forge-driver/v1` (contracts/driver-protocol.v1.schema.json).
//!
//! NDJSON, one message per line: engine→driver on stdin, driver→engine on
//! stdout. stdout is protocol-only; stderr is captured as evidence.
//! Unknown message types and schema-invalid lines fail closed. A driver
//! that goes away after `accepted` without a `result` leaves the attempt
//! indeterminate — never converted to success, never silently retried.

pub mod adapters;
pub mod broker;
pub mod dsh_sandbox;
pub mod fake;
pub mod hands;
pub mod native_controls;
pub mod oneshot;
pub mod overrides;
pub mod process;
pub mod secret;
mod transcript;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTO: &str = "forge-driver/v1";

// NOTE: no deny_unknown_fields here — serde does not support it together
// with flatten. Field strictness lives in the per-variant definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub proto: String,
    pub msg_id: String,
    #[serde(flatten)]
    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    // engine -> driver
    Hello {
        engine_version: String,
    },
    Start {
        effect_id: String,
        attempt_id: String,
        seat: String,
        input: Value,
    },
    Resume {
        effect_id: String,
        attempt_id: String,
        session_ref: String,
    },
    Cancel {
        effect_id: String,
    },
    Shutdown,
    // driver -> engine
    Capabilities {
        driver: String,
        version: String,
        supports: Vec<String>,
    },
    Accepted {
        effect_id: String,
        attempt_id: String,
        #[serde(default)]
        session_ref: Option<String>,
    },
    Checkpoint {
        effect_id: String,
        attempt_id: String,
        data: Value,
    },
    Result {
        effect_id: String,
        attempt_id: String,
        status: ResultStatus,
        #[serde(default)]
        result: Option<Value>,
        #[serde(default)]
        error: Option<String>,
    },
    Cancelled {
        effect_id: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResultStatus {
    Succeeded,
    Failed,
}

impl Message {
    pub fn new(body: Body) -> Message {
        Message {
            proto: PROTO.to_string(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            body,
        }
    }
}

/// What one driver attempt came to. `Indeterminate` is a first-class
/// outcome: the engine parks rather than guessing (target-architecture,
/// outbox discipline step 4). Serialized, it is the `received` evidence
/// of an attempt whose cleanup is unresolved.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AttemptOutcome {
    Succeeded { result: Value },
    Failed { error: String },
    Indeterminate { reason: String },
}

impl AttemptOutcome {
    /// The outcome in words, for a reason that has to name it.
    fn reached(&self) -> &str {
        match self {
            AttemptOutcome::Succeeded { .. } => "the driver reported success",
            AttemptOutcome::Failed { error } => error,
            AttemptOutcome::Indeterminate { reason } => reason,
        }
    }
}

/// Is the attempt's process tree proven over (#403)? Kept beside the
/// outcome the driver reached, never folded into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Cleanup {
    /// Every process of the attempt is gone.
    Settled,
    /// Something of the attempt may still be running.
    Unresolved { reason: process::Unsettled },
}

#[derive(Debug, Clone)]
pub struct AttemptReport {
    /// What the driver reached, as it reached it: the `received`
    /// evidence. Act on [`AttemptReport::settled_outcome`], which also
    /// reads `refused` and `cleanup`.
    pub outcome: AttemptOutcome,
    /// The outcome the engine put in place of `outcome` when the journal
    /// refused one of the attempt's checkpoints (decision 0034, ruling
    /// 6). It replaces the outcome acted on, never the one received.
    pub refused: Option<AttemptOutcome>,
    pub cleanup: Cleanup,
    pub session_ref: Option<String>,
    pub checkpoints: Vec<Value>,
    pub stderr: String,
    /// Did the driver ever send `accepted`? The process layer already
    /// tracks this; surfacing it is what lets the engine decide the
    /// bounded-fallback question STRUCTURALLY (decision 0016) instead of
    /// sniffing stderr for a provider's prose.
    ///
    /// `Failed` plus never accepted plus no checkpoint IS "failed to
    /// start"; once `accepted` arrives, fallback is unreachable by
    /// construction — decision 0016's mid-session boundary mechanised
    /// rather than described.
    pub accepted: bool,
    /// Did the ENGINE's own deadline watchdog end this attempt? A kill
    /// makes non-completion determinate (decision 0006), which is why
    /// the outcome is `Failed` rather than `Indeterminate` — but it says
    /// nothing about whether a session ever opened, because the driver
    /// is SIGKILLed with no chance to report. Decision 0053 ruling 5
    /// therefore keeps a deadline kill off the fail-to-start side of the
    /// boundary: the engine ended it, no provider refused it.
    pub deadline_killed: bool,
}

impl AttemptReport {
    /// The outcome a caller may act on (#403, decision 0006): the one the
    /// driver reached, or the one a refusal put in its place, once its
    /// tree is proven over, and `Indeterminate`, which parks, while it is
    /// not. No settlement, retry or fallback is certified beside what may
    /// still be running.
    pub fn settled_outcome(&self) -> AttemptOutcome {
        let reached = self.refused.as_ref().unwrap_or(&self.outcome);
        match &self.cleanup {
            Cleanup::Settled => reached.clone(),
            Cleanup::Unresolved { reason } => AttemptOutcome::Indeterminate {
                reason: format!(
                    "{}; the attempt is not proven over: {reason}",
                    reached.reached()
                ),
            },
        }
    }

    /// The evidence a terminal event carries for an unresolved cleanup:
    /// the outcome received, typed, beside the cleanup. None once settled,
    /// so a settled attempt's event keeps its bytes.
    pub fn cleanup_evidence(&self) -> Option<CleanupEvidence> {
        match &self.cleanup {
            Cleanup::Settled => None,
            Cleanup::Unresolved { .. } => Some(CleanupEvidence {
                received: self.outcome.clone(),
                cleanup: self.cleanup.clone(),
            }),
        }
    }
}

/// An attempt not proven over, as its terminal event carries it (#403):
/// the `received` and `cleanup` fields of
/// `contracts/effect-cleanup.v1.schema.json`.
#[derive(Debug, Clone, Serialize)]
pub struct CleanupEvidence {
    received: AttemptOutcome,
    cleanup: Cleanup,
}

// The binary's one environment guard (#357).
#[cfg(test)]
#[path = "../../../tests/support/env_guard.rs"]
mod env_guard;
