//! The probe's adapter bound (decision 0103 ruling 4(a)): a probe ends
//! only on a provider's answer, and that answer is a model-output event,
//! never a session start, `turn.started`, `accepted`, a spawn, a hang or
//! an error. The first answer is admitted and no second; a probe that
//! ends with none carries no answer.

use serde_json::Value;

use super::refusal::claude_refusal;
use super::AdapterKind;
use crate::Probe;

/// A start's optional probe marker. Absent is not a probe; present must
/// be the marker, so a `false` is refused rather than read as absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum ProbeRequest {
    #[error("seat refused to start: the start's `probe` field is present but not true")]
    Malformed,
}

pub(super) fn requested(start: &Value) -> Result<bool, ProbeRequest> {
    match start.get("probe") {
        None => Ok(false),
        Some(value) => serde_json::from_value::<Probe>(value.clone())
            .map(|_| true)
            .map_err(|_| ProbeRequest::Malformed),
    }
}

/// One invocation's probe bound: the first answer's checkpoint, and
/// nothing after it.
#[derive(Default)]
pub(super) struct ProbeAnswer {
    requested: bool,
    answer: Option<Value>,
}

impl ProbeAnswer {
    pub(super) fn new(requested: bool) -> ProbeAnswer {
        ProbeAnswer {
            requested,
            answer: None,
        }
    }

    pub(super) fn requested(&self) -> bool {
        self.requested
    }

    /// Has the first answer been admitted? The adapter stops its harness
    /// at that point (decision 0103 ruling 4(a)).
    pub(super) fn answered(&self) -> bool {
        self.answer.is_some()
    }

    /// Admit this event's checkpoints if it is the adapter's answer. The
    /// first answer latches; a later one is not admitted.
    pub(super) fn admit(&mut self, kind: AdapterKind, event: &Value, emitted: &[Value]) {
        if !self.requested || self.answer.is_some() || !is_answer(kind, event) {
            return;
        }
        self.answer = emitted.first().cloned();
    }

    pub(super) fn take(&mut self) -> Option<Value> {
        self.answer.take()
    }
}

/// Is this event the adapter's model-output answer for a probe?
pub(super) fn is_answer(kind: AdapterKind, event: &Value) -> bool {
    match kind {
        AdapterKind::Claude | AdapterKind::Lanetally => {
            event.get("type").and_then(Value::as_str) == Some("assistant")
                && claude_refusal(event).is_none()
        }
        AdapterKind::Codex => {
            event.get("type").and_then(Value::as_str) == Some("item.completed")
                && super::capability_calls::codex_item_type(event).as_deref()
                    == Some("agent_message")
        }
        AdapterKind::Dsh => event.get("type").and_then(Value::as_str) == Some("assistant/message"),
        AdapterKind::Exec => false,
    }
}

/// One checkpoint as a seat row: masked on the parsed strings (decision
/// 0012, layer 5), then the two sentinels the seat record's fields require
/// when the harness reported neither. Every fold emits a JSON object and
/// the seat record (0034) is defined on objects.
pub(super) fn seat_row(
    kind: AdapterKind,
    bindings: &[crate::secret::BoundSecret],
    data: &Value,
) -> Value {
    let mut data = data.clone();
    if !bindings.is_empty() {
        crate::secret::mask_json(&mut data, bindings);
    }
    let checkpoint = data
        .as_object_mut()
        .expect("driver checkpoints are JSON objects");
    checkpoint.entry("model").or_insert_with(|| {
        Value::String(
            if kind == AdapterKind::Exec {
                super::MODEL_NOT_APPLICABLE
            } else {
                super::MODEL_NOT_REPORTED
            }
            .to_string(),
        )
    });
    // The same default, one field over (decision 0035 ruling 3): a fold
    // that read its harness's echo has already written the level.
    checkpoint.entry("effort").or_insert_with(|| {
        Value::String(
            if kind == AdapterKind::Exec {
                super::EFFORT_NOT_APPLICABLE
            } else {
                super::EFFORT_NOT_REPORTED
            }
            .to_string(),
        )
    });
    data
}

/// The concrete model id a seat's argv pins, where it pins one. The
/// engine composes exactly one `--model`; a driver reading its own launch
/// facts takes the first, never a later duplicate.
pub(super) fn pinned_model(extra: &[String]) -> Option<&str> {
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if part == "--model" {
            return parts.next().map(String::as_str);
        }
        if let Some(value) = part.strip_prefix("--model=") {
            return Some(value);
        }
    }
    None
}

/// Report a probe's one answer, or its typed refusal, or say it has
/// neither. The answer is the checkpoint the model-output event emitted;
/// `accepted` alone is never one (decision 0103 ruling 4(a)). A refusal
/// before any answer travels exactly as an ordinary attempt's does, so a
/// rate-limited probe is not erased into "no answer" (decision 0103
/// ruling 1(a)).
pub(super) fn report_answer(
    send: &mut impl FnMut(crate::Body),
    kind: AdapterKind,
    bindings: &[crate::secret::BoundSecret],
    effect_id: String,
    attempt_id: String,
    answer: Option<Value>,
    refusal: Option<crate::PreSessionRefusal>,
) {
    match answer {
        Some(answer) => {
            let row = seat_row(kind, bindings, &answer);
            send(crate::Body::Accepted {
                effect_id: effect_id.clone(),
                attempt_id: attempt_id.clone(),
                session_ref: None,
            });
            send(crate::Body::Checkpoint {
                effect_id: effect_id.clone(),
                attempt_id: attempt_id.clone(),
                data: row.clone(),
            });
            send(crate::Body::Result {
                refusal: None,
                effect_id,
                attempt_id,
                status: crate::ResultStatus::Succeeded,
                result: Some(row),
                error: None,
            });
        }
        None => send(crate::Body::Result {
            refusal,
            effect_id,
            attempt_id,
            status: crate::ResultStatus::Failed,
            result: None,
            error: Some("probe ended without an answer".into()),
        }),
    }
}

/// A start refused before anything launches: `failed`, no `accepted` and
/// no checkpoint (decision 0053's failure to start).
pub(super) fn refused_start(
    send: &mut impl FnMut(crate::Body),
    effect_id: String,
    attempt_id: String,
    error: String,
) {
    send(crate::Body::Result {
        refusal: None,
        effect_id,
        attempt_id,
        status: crate::ResultStatus::Failed,
        result: None,
        error: Some(error),
    });
}

/// A determinate configuration refusal after the driver has accepted: it
/// keeps `accepted` exactly as it always was, so only a provider refusal
/// moves the fail-to-start boundary.
pub(super) fn determinate_failure(
    send: &mut impl FnMut(crate::Body),
    effect_id: &str,
    attempt_id: &str,
    error: String,
) {
    send(crate::Body::Accepted {
        effect_id: effect_id.to_string(),
        attempt_id: attempt_id.to_string(),
        session_ref: None,
    });
    send(crate::Body::Result {
        refusal: None,
        effect_id: effect_id.to_string(),
        attempt_id: attempt_id.to_string(),
        status: crate::ResultStatus::Failed,
        result: None,
        error: Some(error),
    });
}

/// Send `accepted` once, flushing the pre-session rows buffered while the
/// session was opening. The one home of decision 0053's boundary and the
/// buffer's flush discipline.
pub(super) fn accept_and_flush(
    send: &mut impl FnMut(crate::Body),
    effect_id: &str,
    attempt_id: &str,
    accepted_sent: &mut bool,
    buffered: &mut Vec<Value>,
) {
    if *accepted_sent {
        return;
    }
    *accepted_sent = true;
    send(crate::Body::Accepted {
        effect_id: effect_id.to_string(),
        attempt_id: attempt_id.to_string(),
        session_ref: None,
    });
    for row in buffered.drain(..) {
        send(crate::Body::Checkpoint {
            effect_id: effect_id.to_string(),
            attempt_id: attempt_id.to_string(),
            data: row,
        });
    }
}

/// Fold one event through `fold`, capturing the checkpoints it emitted so
/// a probe can admit the first answer and no second. The capture wraps
/// the same `watch` the ordinary path uses, so a probe's answer is the
/// checkpoint that event emitted — never one rebuilt here.
pub(super) fn fold_capturing(
    probe: &mut ProbeAnswer,
    kind: AdapterKind,
    event: &Value,
    watch: &mut impl FnMut(&Value),
    fold: impl FnOnce(&mut dyn FnMut(&Value)) -> Option<crate::PreSessionRefusal>,
) -> Option<crate::PreSessionRefusal> {
    let answer_event = probe.requested() && is_answer(kind, event);
    let mut emitted: Vec<Value> = Vec::new();
    let classified = {
        let mut capture = |data: &Value| {
            if answer_event {
                emitted.push(data.clone());
            }
            watch(data);
        };
        fold(&mut capture)
    };
    probe.admit(kind, event, &emitted);
    classified
}
