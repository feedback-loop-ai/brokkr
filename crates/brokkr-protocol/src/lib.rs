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
pub mod probe;
pub mod process;
pub mod secret;
mod served_home;
mod transcript;
pub mod zstd;

use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use served_home::{dsh_served, holds_dotenv, DshRefusal, DshServed};

pub const PROTO: &str = "forge-driver/v1";

/// The numbered extension schema this crate's optional driver additions
/// are published under (decision 0103 ruling 1(a)): the typed `refusal`
/// object on a `result`, and the `probe` marker on `capabilities` and
/// `start`. The protocol word stays [`PROTO`]; the extension moves with
/// it.
pub const DRIVER_REFUSAL_SCHEMA: &str = "driver-refusal/v1";

/// The class a provider's pre-session refusal falls into (decision 0103
/// ruling 1(a)). Closed: an unknown class is refused where it is read,
/// never mapped onto the nearest known one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalClass {
    RateLimit,
    Other,
}

/// One provider refusal before the first turn (decisions 0053 and 0103
/// ruling 1(a)): the machine class, the provider and route the driver
/// already resolved, the provider's bounded evidence, and the two
/// optional facts a later wait needs. It is built where the refusal text
/// is built, crosses the driver boundary as the optional `refusal` object
/// on a `result`, and is never derived from prose.
///
/// `words` is the bounded evidence line the driver's own words have
/// always carried — the provider's machine token when it gave one, the
/// HTTP status when it gave that, and a short excerpt of its prose — so
/// `Display` keeps every operator-facing reason byte for byte. The typed
/// fields sit beside it for the engine's decision; neither is recomputed
/// from the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PreSessionRefusal {
    class: RefusalClass,
    provider: String,
    route: Option<String>,
    words: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

/// The most `words` may carry: the refusal token's 80, a status and the
/// excerpt's 160, with room for their punctuation. A greater bound from a
/// driver is refused at the edge rather than journaled.
const REFUSAL_WORDS_LIMIT: usize = 300;

/// Why a present `refusal` object could not be read (decision 0103 ruling
/// 1(a)). Each variant names the field, never the driver's content: the
/// text becomes a journal reason, and a secret a driver sent by accident
/// must not be copied there (decision 0012).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RefusalError {
    #[error("driver refusal is not an object")]
    Object,
    #[error("driver refusal `class` is neither `rate_limit` nor `other`")]
    Class,
    #[error("driver refusal names no provider")]
    Provider,
    #[error("driver refusal names no `route`; an unresolved route is written as null")]
    Route,
    #[error("driver refusal `route` is not a route identifier")]
    RouteForm,
    #[error("driver refusal `words` is not a bounded non-empty string")]
    Words,
    #[error("driver refusal `status` is not an HTTP status")]
    Status,
    #[error("driver refusal `retry_after_seconds` is not a positive whole number")]
    RetryAfter,
    #[error("driver refusal carries a field outside {0}")]
    Unknown(&'static str),
}

impl PreSessionRefusal {
    /// Build one from the evidence a classifier read. `provider` and
    /// `route` are left unresolved here; the invocation that resolved the
    /// launch facts attributes it with [`PreSessionRefusal::attributed`],
    /// so no classifier invents either from prose.
    pub(crate) fn from_parts(
        class: RefusalClass,
        words: String,
        status: Option<i64>,
        retry_after_seconds: Option<u64>,
    ) -> PreSessionRefusal {
        PreSessionRefusal {
            class,
            provider: String::new(),
            route: None,
            words,
            status,
            retry_after_seconds,
        }
    }

    /// Fill the provider and route from the adapter and launch facts the
    /// driver already resolved. `route` is a typed absence where the
    /// driver serves the model on none.
    pub(crate) fn attributed(mut self, provider: &str, route: Option<&str>) -> PreSessionRefusal {
        self.provider = provider.to_string();
        self.route = route.map(str::to_string);
        self
    }

    /// Replace the bounded evidence with its masked form. It is the same
    /// one choke point the refusal text has always passed through
    /// (decision 0012), applied to the field that carries the harness's
    /// bytes.
    pub(crate) fn with_words(mut self, words: String) -> PreSessionRefusal {
        self.words = words;
        self
    }

    /// Read one present refusal object at the edge. Every malformed shape
    /// is refused by name, never read as absent (decision 0103 ruling
    /// 1(a)); an unresolved route is the explicit `null`, not a missing
    /// key.
    pub(crate) fn read(value: &serde_json::Value) -> Result<PreSessionRefusal, RefusalError> {
        let object = value.as_object().ok_or(RefusalError::Object)?;
        for key in object.keys() {
            if !matches!(
                key.as_str(),
                "class" | "provider" | "route" | "words" | "status" | "retry_after_seconds"
            ) {
                return Err(RefusalError::Unknown(DRIVER_REFUSAL_SCHEMA));
            }
        }
        let class = match object.get("class").and_then(serde_json::Value::as_str) {
            Some("rate_limit") => RefusalClass::RateLimit,
            Some("other") => RefusalClass::Other,
            _ => return Err(RefusalError::Class),
        };
        let provider = object
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .filter(|provider| refusal_identifier(provider))
            .ok_or(RefusalError::Provider)?;
        let route = match object.get("route") {
            None => return Err(RefusalError::Route),
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(route)) if refusal_identifier(route) => {
                Some(route.clone())
            }
            Some(_) => return Err(RefusalError::RouteForm),
        };
        let words = object
            .get("words")
            .and_then(serde_json::Value::as_str)
            .filter(|words| !words.is_empty() && words.chars().count() <= REFUSAL_WORDS_LIMIT)
            .ok_or(RefusalError::Words)?;
        let status = match object.get("status") {
            None => None,
            Some(value) => {
                let status = value.as_i64().ok_or(RefusalError::Status)?;
                if !(100..=599).contains(&status) {
                    return Err(RefusalError::Status);
                }
                Some(status)
            }
        };
        let retry_after_seconds = match object.get("retry_after_seconds") {
            None => None,
            Some(value) => {
                let seconds = value.as_i64().ok_or(RefusalError::RetryAfter)?;
                let seconds = u64::try_from(seconds)
                    .ok()
                    .filter(|seconds| *seconds > 0)
                    .ok_or(RefusalError::RetryAfter)?;
                Some(seconds)
            }
        };
        Ok(PreSessionRefusal {
            class,
            provider: provider.to_string(),
            route,
            words: words.to_string(),
            status,
            retry_after_seconds,
        })
    }

    pub fn class(&self) -> RefusalClass {
        self.class
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn route(&self) -> Option<&str> {
        self.route.as_deref()
    }

    pub fn words(&self) -> &str {
        &self.words
    }

    pub fn status(&self) -> Option<i64> {
        self.status
    }

    pub fn retry_after_seconds(&self) -> Option<u64> {
        self.retry_after_seconds
    }
}

/// A provider, route or other wire identifier: 1–80 ASCII characters,
/// alphanumeric first, then alphanumerics, underscores, hyphens and dots.
/// The error never echoes the value, so this is a shape test only.
fn refusal_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphanumeric())
        && value.len() <= 80
        && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

impl std::fmt::Display for PreSessionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "provider refused before the first turn: {}", self.words)
    }
}

impl<'de> Deserialize<'de> for PreSessionRefusal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        PreSessionRefusal::read(&value).map_err(serde::de::Error::custom)
    }
}

/// A typed probe marker (decision 0103 ruling 4(a)). It is written as
/// `true` where present and never as `false`: an absent field is the only
/// absence, so a doubly-optional boolean cannot stand in for a closed
/// vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probe;

impl Serialize for Probe {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(true)
    }
}

impl<'de> Deserialize<'de> for Probe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if bool::deserialize(deserializer)? {
            Ok(Probe)
        } else {
            Err(serde::de::Error::custom(
                "the probe marker is present but not true",
            ))
        }
    }
}

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
        /// Decision 0103 ruling 4(a): this invocation is a one-turn probe,
        /// present and `true` only under `driver-refusal/v1`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        probe: Option<Probe>,
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
        /// Decision 0103 ruling 4(a): the driver can run a one-turn probe,
        /// present and `true` only under `driver-refusal/v1`. It is never
        /// a `supports` entry: that list is closed by the frozen v1
        /// schema to `resume`, `checkpoint` and `cancel`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        probe: Option<Probe>,
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
        /// Decision 0103 ruling 1(a): the one typed provider refusal this
        /// invocation classified before its first turn, present only under
        /// `driver-refusal/v1`. Absent reads as no typed refusal and the
        /// engine keeps today's path; a present object that cannot be read
        /// is refused by name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refusal: Option<PreSessionRefusal>,
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
    /// The one typed provider refusal this invocation classified before
    /// its first turn (decision 0103 ruling 1(a)). A panel or sequence
    /// drives each member or step as its own invocation, so each report
    /// carries its own and a single seat carries at most one.
    pub refusal: Option<Box<PreSessionRefusal>>,
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
    /// proves the process tree gone and nothing about what the attempt
    /// did, so its outcome is `Indeterminate` and it parks (decision
    /// 0006's 2026-10-11 addendum, ruling 1). Nor does it say whether a
    /// session ever opened, because the driver is SIGKILLed with no
    /// chance to report: decision 0053 ruling 5 keeps a deadline kill off
    /// the fail-to-start side of the boundary, since the engine ended it
    /// and no provider refused it.
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
