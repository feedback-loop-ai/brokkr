//! `forge-driver/v1` (contracts/driver-protocol.v1.schema.json).
//!
//! NDJSON, one message per line: engine→driver on stdin, driver→engine on
//! stdout. stdout is protocol-only; stderr is captured as evidence.
//! Unknown message types and schema-invalid lines fail closed. A driver
//! that goes away after `accepted` without a `result` leaves the attempt
//! indeterminate — never converted to success, never silently retried.

pub mod adapters;
pub mod fake;
pub mod process;
pub mod secret;

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
/// outbox discipline step 4).
#[derive(Debug, Clone)]
pub enum AttemptOutcome {
    Succeeded { result: Value },
    Failed { error: String },
    Indeterminate { reason: String },
}

#[derive(Debug, Clone)]
pub struct AttemptReport {
    pub outcome: AttemptOutcome,
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
}
