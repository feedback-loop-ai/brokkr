//! What a participant's records say as the console shows them (#753;
//! decision 0089 slice 2b): its last `seat-turn`'s turn, tool and target,
//! a member's finish, the session its session record names, and the live
//! block's members, kept on its rows so the run view reads them by key. A
//! value a record states in any JSON shape is kept as the text the console
//! shows of it ([`shown`]), never as the JSON, so the row holds text alone
//! (ruling 10).

use serde_json::Value;

use super::{Member, Meter, Pin, SeatRow};

/// A seat's finishing or opening session record: its meter, the cost it
/// reported, and the session it names.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Session {
    pub(super) meter: Meter,
    pub(super) cost: Option<f64>,
    pub(super) id: Option<String>,
}

/// A value a record states, as the console shows it: JavaScript's string
/// conversion, as `'a' + value` performs it, the literal `undefined` for
/// an absent field included. The one home of that rule: brokkr-view's
/// `js::to_display` paints through it, and the seat rows keep what it
/// gives, so a row and the view show a value alike.
pub fn shown(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => {
            // `serde_json` renders an integral float as `1.0`; JS
            // renders `1`, and this is display, not the payload dump.
            let text = number.to_string();
            text.strip_suffix(".0").unwrap_or(&text).to_string()
        }
        Some(Value::Array(items)) => {
            let parts: Vec<String> = items.iter().map(element).collect();
            parts.join(",")
        }
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// `Array.prototype.join` renders `null` and `undefined` as the empty
/// string, unlike bare concatenation.
fn element(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        other => shown(Some(other)),
    }
}

/// A `seat-turn` record: its turn and tool as shown, and its target where
/// it names one as text that says anything, as the console's activity
/// reads a target.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Turned {
    pub(super) turn: String,
    pub(super) tool: String,
    pub(super) target: Option<String>,
}

impl Turned {
    pub(super) fn of(checkpoint: &Value) -> Turned {
        let target = match checkpoint.get("target") {
            Some(Value::String(target)) if !target.is_empty() => Some(target.clone()),
            _ => None,
        };
        Turned {
            turn: shown(checkpoint.get("turn")),
            tool: shown(checkpoint.get("tool")),
            target,
        }
    }
}

/// A member's panel finish, as its terminal line shows it: its time, and
/// its outcome as shown, `None` for a JSON null, which the line passes
/// over for its effect's terminal; an absent outcome shows `undefined`.
#[derive(Debug, Clone, PartialEq)]
pub struct Finish {
    pub at: String,
    pub outcome: Option<String>,
}

/// A `panel-member-finished` record stamped `at`: the outcome it pins, and
/// its finish.
pub(super) fn finished(checkpoint: &Value, at: &str) -> (Pin, Finish) {
    let outcome = checkpoint.get("outcome");
    let pin = outcome
        .and_then(Value::as_str)
        .and_then(super::Outcome::named);
    let finish = Finish {
        at: at.to_string(),
        outcome: (outcome != Some(&Value::Null)).then(|| shown(outcome)),
    };
    (pin.map_or(Pin::Unpinned, Pin::Pinned), finish)
}

/// The live block's participants (decision 0089 slice 2b): those of the
/// attempt the run's last start opened, while no terminal has concluded
/// it since, in the order of their first checkpoint in it, each with its
/// last `seat-turn`'s turn and tool in it as shown, or none; none while
/// no attempt is in flight, or the one in flight has no checkpoint yet.
pub fn live(rows: &[SeatRow]) -> Vec<(Member, Option<(String, String)>)> {
    let seats = rows.iter().filter(|row| row.member == Member::Seat);
    let last = seats
        .filter_map(|row| Some((row.since?, row)))
        .max_by_key(|(since, _)| *since);
    // A terminal and a start are two events, so their seqs are never
    // equal, as `Effect::of` reads them.
    let open = last.filter(|(since, row)| row.ended.is_none_or(|(seq, _)| seq.cmp(since).is_lt()));
    let Some((_, open)) = open else {
        return Vec::new();
    };
    let mut members: Vec<&SeatRow> = rows
        .iter()
        .filter(|row| row.request_seq == open.request_seq && row.attempt_id == open.attempt_id)
        .filter(|row| row.first.is_some())
        .collect();
    members.sort_by_key(|row| row.first.as_ref().map(|first| first.seq));
    let turned = |row: &SeatRow| {
        let turn = row.turn.as_ref().map(|(_, turn)| turn);
        turn.map(|turn| (turn.turn.clone(), turn.tool.clone()))
    };
    let live = members
        .into_iter()
        .map(|row| (row.member.clone(), turned(row)));
    live.collect()
}

#[cfg(test)]
mod tests;
