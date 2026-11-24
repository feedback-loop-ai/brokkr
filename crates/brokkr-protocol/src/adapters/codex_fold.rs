//! Codex's live stream checkpoints and session totals.

use serde_json::{json, Map, Value};

use super::{
    add_usage, fold_result_meta, model_in_json, CapturedId, CodexThreadEcho, Emitted, Locator,
    Observation, PreSessionRefusal, Row, Step, Transcript, EFFORT_NOT_REPORTED, MODEL_NOT_REPORTED,
};

#[derive(Clone, Copy)]
enum Event {
    Thread,
    Start,
    Item(Step),
    Complete,
    Result,
}

const EVENT_KINDS: &[(&str, Event)] = &[
    ("thread.started", Event::Thread),
    ("turn.started", Event::Start),
    ("item.started", Event::Item(Step::ItemStarted)),
    ("item.completed", Event::Item(Step::ItemCompleted)),
    ("turn.completed", Event::Complete),
    ("result", Event::Result),
];

/// Fold Codex's stable `exec --json` JSONL into bounded live telemetry.
/// Commands, item bodies, model output, reasoning, and filesystem paths are
/// intentionally ignored; the journal records progress and usage, not a
/// transcript.
pub(super) fn fold_codex_event(
    row: &Row<'_>,
    identity: Option<CapturedId>,
    turn: &mut u64,
    session_meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    echo: &mut CodexThreadEcho<'_>,
    emit: &mut impl FnMut(&Emitted),
) -> Option<PreSessionRefusal> {
    match row.kind("/type", EVENT_KINDS) {
        Some(Event::Thread) => {
            if let Some(identity) = identity {
                echo.locate(transcript.home(), identity.ident.clone());
                // Publish the locator before work so a deadline after
                // the first turn still leaves a resume handle (0053 ruling 8).
                transcript.record(Locator::Session(identity), session_meta, emit);
            }
        }
        Some(Event::Start) => {
            *turn += 1;
            emit(&Emitted::new(
                Step::TurnStarted,
                json!({"turn": *turn, "harness":"codex"}),
            ));
        }
        Some(Event::Item(step)) => {
            let checkpoint = json!({"turn": *turn, "harness":"codex"});
            let checkpoint = Observation::codex(row).onto_codex(row, checkpoint);
            emit(&Emitted::new(step, checkpoint));
        }
        Some(Event::Complete) => {
            let mut checkpoint = turn_checkpoint(row, *turn, session_meta, echo);
            if let Some(usage) = row.at("/usage") {
                fold_usage(&usage, session_meta, &mut checkpoint);
            }
            // The turn count is journaled HERE, not only from `result`.
            // Verified against codex-cli 0.148.0: a real `codex exec
            // --json` run ends at `turn.completed` and emits no `result`
            // event at all, so a codex seat's num_turns never reached
            // `brokkr stats costs` — the aggregator saw per-turn token keys it
            // does not sum and nothing else. Cost in USD is reported
            // nowhere in that stream (`usage` is token counts only), so
            // total_cost_usd stays absent rather than invented.
            session_meta.insert("num_turns".into(), Value::from(*turn));
            emit(&Emitted::new(
                Step::TurnCompleted,
                Value::Object(checkpoint),
            ));
        }
        // Conformance shims and older clients may provide only final metadata.
        Some(Event::Result) => {
            fold_result_meta(
                row,
                model_in_json(row),
                identity,
                session_meta,
                transcript,
                emit,
            );
            if let Some(value) = row.unsigned_count("/num_turns").filter(|value| *value > 0) {
                session_meta.insert("num_turns".into(), Value::from(value));
            }
            if let Some(value) = row.number("/total_cost_usd").filter(|value| *value > 0.0) {
                session_meta.insert("total_cost_usd".into(), Value::from(value));
            }
        }
        None => {}
    }
    None
}

fn turn_checkpoint(
    row: &Row<'_>,
    turn: u64,
    session_meta: &mut Map<String, Value>,
    echo: &mut CodexThreadEcho<'_>,
) -> Map<String, Value> {
    // Codex usually reports these through its rollout (0035 ruling 3).
    // A model the stream names is the more direct report.
    let (thread_model, effort) = echo.echo();
    let model = model_in_json(row).or(thread_model);
    if let Some(model) = &model {
        session_meta.insert("model".into(), Value::String(model.clone()));
    }
    if let Some(effort) = &effort {
        session_meta.insert("effort".into(), Value::String(effort.clone()));
    }
    Map::from_iter([
        ("turn".into(), Value::from(turn)),
        ("harness".into(), Value::from("codex")),
        (
            "model".into(),
            Value::from(model.as_deref().unwrap_or(MODEL_NOT_REPORTED)),
        ),
        (
            "effort".into(),
            Value::from(effort.as_deref().unwrap_or(EFFORT_NOT_REPORTED)),
        ),
    ])
}

fn fold_usage(usage: &Row<'_>, meta: &mut Map<String, Value>, checkpoint: &mut Map<String, Value>) {
    for (source, target) in [
        ("/input_tokens", "input_tokens"),
        ("/cached_input_tokens", "cache_read_tokens"),
        ("/output_tokens", "output_tokens"),
        ("/cache_write_input_tokens", "cache_write_tokens"),
        // Reasoning is part of output_tokens and keeps its own counter.
        ("/reasoning_output_tokens", "reasoning_output_tokens"),
    ] {
        if let Some(value) = usage.unsigned_count(source).filter(|value| *value > 0) {
            checkpoint.insert(target.into(), Value::from(value));
            add_usage(meta, target, value);
        }
    }
}
