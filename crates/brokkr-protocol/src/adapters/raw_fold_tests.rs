//! Unbound inputs for the existing tests of the partially migrated folds.

use super::*;

pub(super) fn fold_stream_event(
    event: &Value,
    fold: &mut ClaudeFold,
    meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    emit: &mut impl FnMut(&Value),
) -> Option<PreSessionRefusal> {
    let refusal = (fold.turns == 0)
        .then(|| super::claude_refusal(&Row::new(event, &[])))
        .flatten();
    super::fold_stream_event(event, refusal, fold, meta, transcript, &mut |row| {
        emit(&row.row)
    })
}

pub(super) fn fold_codex_event(
    event: &Value,
    turn: &mut u64,
    meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    echo: &mut CodexThreadEcho,
    emit: &mut impl FnMut(&Value),
) -> Option<PreSessionRefusal> {
    let refusal = (*turn == 0)
        .then(|| super::codex_refusal(&Row::new(event, &[])))
        .flatten();
    super::fold_codex_event(event, refusal, turn, meta, transcript, echo, &mut |row| {
        emit(&row.row)
    })
}

pub(super) fn claude_refusal(event: &Value) -> Option<PreSessionRefusal> {
    super::claude_refusal(&Row::new(event, &[]))
}

pub(super) fn codex_refusal(event: &Value) -> Option<PreSessionRefusal> {
    super::codex_refusal(&Row::new(event, &[]))
}
