//! One provider's pre-session refusal, typed where the text is built
//! (decision 0103 ruling 1(a)). The classifiers read only their harness's
//! machine fields; the provider and route are filled later, from the
//! adapter and the launch facts the driver already resolved, never from
//! prose. The bounded evidence line is this module's one home, so
//! `PreSessionRefusal`'s `Display` keeps the operator's text byte for
//! byte.

use serde_json::Value;

use super::AdapterKind;
use crate::{PreSessionRefusal, RefusalClass};

/// One line of wire text, whitespace-collapsed and clamped: every field
/// a refusal reason quotes comes off the harness's stream, so it is
/// arbitrary-length and may carry newlines. The journal is append-only
/// and every readout renders what lands in it, so a reason is bounded
/// here rather than trusted — the same discipline as the ≤80-char tool
/// names and targets and the 4000-byte stderr tail. Control characters
/// go with the newlines: a terminal escape belongs in a record no more
/// than a line break does.
pub(super) fn bounded_wire_line(text: &str, limit: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

/// The token's own bound. A machine token is a short identifier
/// (`rate_limit`, `authentication_error`); the checkpoint bound for a
/// harness-supplied name governs it too.
pub(super) const REFUSAL_TOKEN_LIMIT: usize = 80;

/// And the excerpt's, which may be a sentence.
const REFUSAL_TEXT_LIMIT: usize = 160;

/// Bound one refusal for the journal: a single line, clamped, and never
/// empty. The evidence is the provider's own machine token when it gave
/// one, the HTTP status when it gave that, and a short excerpt of its
/// prose when it gave that — the same evidence a stderr tail already
/// carries on a failed attempt, and nothing executable is derived from it.
///
/// The token is bounded exactly like the excerpt. It is read from the
/// harness's stream, so "the provider's own token" is only ever a claim
/// about where the bytes came from: a harness emitting a 200KB
/// newline-bearing `error` must not put that in a durable record.
pub(super) fn refusal_reason(
    token: &str,
    status: Option<i64>,
    text: Option<&str>,
) -> PreSessionRefusal {
    let token = bounded_wire_line(token, REFUSAL_TOKEN_LIMIT);
    // Never empty: a refusal with no token at all is still a refusal,
    // and an empty one would leave the line dangling after its colon.
    let token = if token.is_empty() {
        "api_error"
    } else {
        &token
    };
    let class = if token == "rate_limit" {
        RefusalClass::RateLimit
    } else {
        RefusalClass::Other
    };
    let mut words = token.to_string();
    if let Some(status) = status {
        words.push_str(&format!(" (HTTP {status})"));
    }
    if let Some(text) = text {
        let excerpt = bounded_wire_line(text, REFUSAL_TEXT_LIMIT);
        if !excerpt.is_empty() {
            words.push_str(": ");
            words.push_str(&excerpt);
        }
    }
    PreSessionRefusal::from_parts(class, words, status, None)
}

/// The claude (and, byte-for-byte, lanetally) stream-json record of a
/// provider refusal. Three measured shapes, all meaning the request was
/// rejected before any inference (decision 0053):
///
/// - the synthetic assistant message #68816 records, carrying
///   `isApiErrorMessage: true` and an `error` token (`rate_limit`,
///   `authentication_error`, …) beside an ordinary terminal
///   `stop_reason: "stop_sequence"` — the shape the #219 run measured,
///   and the reason a `type` test alone cannot see it;
/// - the error `result` #79500 records, `is_error: true` with the prose
///   in `result` and no turn behind it;
/// - the `rate_limit_event` lifecycle message newer CLIs emit.
///
/// A record with none of those fields is an ordinary turn and is never
/// read here; nothing in this classifier looks at a model's prose.
pub(super) fn claude_refusal(event: &Value) -> Option<PreSessionRefusal> {
    match event.get("type").and_then(Value::as_str) {
        Some("rate_limit_event") => Some(refusal_reason(
            "rate_limit",
            event.get("apiErrorStatus").and_then(Value::as_i64),
            event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.get("error").and_then(Value::as_str)),
        )),
        Some("assistant") => {
            let api_error = event
                .get("isApiErrorMessage")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            api_error.then(|| {
                refusal_reason(
                    event
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("api_error"),
                    event.get("apiErrorStatus").and_then(Value::as_i64),
                    event
                        .pointer("/message/content/0/text")
                        .and_then(Value::as_str),
                )
            })
        }
        Some("result") => {
            let is_error = event
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            is_error.then(|| {
                refusal_reason(
                    event
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("api_error"),
                    event.get("apiErrorStatus").and_then(Value::as_i64),
                    event.get("result").and_then(Value::as_str),
                )
            })
        }
        _ => None,
    }
}

/// The codex `exec --json` record of a pre-session refusal: the harness's
/// own machine-readable `error` event, or a `turn.failed` that arrived
/// before any `turn.started`. Codex puts the provider's message in
/// `message` on the one and under `error.message` on the other; neither
/// is model prose, and a record with any other type is never read here.
pub(super) fn codex_refusal(event: &Value) -> Option<PreSessionRefusal> {
    match event.get("type").and_then(Value::as_str) {
        Some("error") => Some(refusal_reason(
            "api_error",
            None,
            event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.pointer("/error/message").and_then(Value::as_str)),
        )),
        Some("turn.failed") => Some(refusal_reason(
            "api_error",
            None,
            event
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| event.get("message").and_then(Value::as_str)),
        )),
        _ => None,
    }
}

impl AdapterKind {
    /// The provider name the adapter's declaration uses, which is the one
    /// a typed refusal carries (decision 0103 ruling 1(a)). It is filled
    /// from the adapter, never read off a harness's prose.
    pub(crate) fn provider_name(self) -> &'static str {
        match self {
            AdapterKind::Claude => "claude",
            AdapterKind::Lanetally => "lanetally",
            AdapterKind::Codex => "codex",
            AdapterKind::Dsh => "dsh",
            AdapterKind::Exec => "exec",
        }
    }

    /// Whether this adapter advertises the one-turn probe (decision 0103
    /// ruling 4(a)). It is the `capabilities.probe` field, never a
    /// `supports` entry: the frozen v1 schema closes that list to
    /// `resume`, `checkpoint` and `cancel`. Exec has no model turn, so it
    /// advertises nothing.
    pub(super) fn advertises_probe(self) -> bool {
        match self {
            AdapterKind::Claude
            | AdapterKind::Lanetally
            | AdapterKind::Codex
            | AdapterKind::Dsh => true,
            AdapterKind::Exec => false,
        }
    }
}
