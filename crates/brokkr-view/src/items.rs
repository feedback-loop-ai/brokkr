//! The seat's completed items (#550): the one derivation that counts a
//! codex seat's work beside its turn. A codex turn is the whole
//! autonomous task; the work inside it — shell commands, file reads,
//! reasoning, messages — arrives as `item-completed` checkpoints the
//! driver already journals, each carrying its `tool` type. Counting them
//! is a derivation of journal facts: no new checkpoint, no driver
//! change, and the count grows live while the seat works.

/// One completed item of the seat's work: the step name decides, and a
/// checkpoint of any other step is ignored rather than guessed at.
pub(crate) fn count_item(items: &mut Option<u64>, step: Option<&str>) {
    if step == Some("item-completed") {
        *items.get_or_insert(0) += 1;
    }
}

/// The turns cell's text, derived once for every surface that paints it:
/// the turn count — `Σ`-marked when it is an aggregation — with the
/// seat's completed items beside it (`1 · 47 items`), or the items alone
/// while the seat's first turn is still working. A seat whose harness
/// journals no items reads exactly as before, and a seat with neither
/// fact keeps the absence mark.
pub(crate) fn turns_text(turns: Option<u64>, items: Option<u64>, prefix: &str) -> Option<String> {
    match (turns, items) {
        (Some(turns), Some(items)) => Some(format!("{prefix}{turns} · {items} items")),
        (Some(turns), None) => Some(format!("{prefix}{turns}")),
        (None, Some(items)) => Some(format!("{items} items")),
        (None, None) => None,
    }
}
