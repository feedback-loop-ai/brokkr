//! What a participant's records say as the console shows them (#753;
//! decision 0089 slice 2b): its last `seat-turn`'s turn, tool and target,
//! a member's finish, the session its session record names, and the live
//! block: which rows are of the run's last start, and the lines records
//! before their effect's request open, all kept on its rows so the run
//! view reads them by key. A value a record states in any JSON shape is
//! kept as the text the console shows of it ([`shown`]), never as the
//! JSON, so the row holds text alone (ruling 10).

use serde_json::Value;

use super::listed::Kind;
use super::{Member, Meter, Pin, SeatRow, Seats};
use crate::envelope::EventEnvelope;
use crate::residual::text as field;

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

/// A live line a record before its effect's first request opens: the
/// attempt and the member the record names, and the last `seat-turn`'s
/// turn and tool among such records of both, as shown.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Early {
    pub(super) attempt: Option<String>,
    pub(super) member: Member,
    pub(super) turn: Option<(String, String)>,
}

impl Early {
    /// A record of `attempt` before its effect's first request, among
    /// `lines`: a line of its attempt and member, opened where there is
    /// none, takes its turn where it is a `seat-turn`.
    pub(super) fn noted(lines: &mut Vec<Early>, attempt: Option<&str>, checkpoint: &Value) {
        let member = Member::of(field(checkpoint, "member"));
        let attempt = attempt.map(str::to_string);
        let known = |line: &Early| line.attempt == attempt && line.member == member;
        let at = match lines.iter().position(known) {
            Some(at) => at,
            None => {
                let turn = None;
                lines.push(Early {
                    attempt,
                    member,
                    turn,
                });
                lines.len() - 1
            }
        };
        if field(checkpoint, "step") == Some("seat-turn") {
            let turned = Turned::of(checkpoint);
            lines[at].turn = Some((turned.turn, turned.tool));
        }
    }
}

impl Seats {
    /// `event`, as the live block reads the run's last start: a start
    /// makes its effect's and attempt's rows the live ones, and no other;
    /// a terminal naming both ends them; and the request of an effect whose
    /// live start came before it opens that attempt's seat row, live.
    pub(super) fn lived(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        let effect_id = field(payload, "effect_id").map(str::to_string);
        let named = (effect_id, field(payload, "attempt_id").map(str::to_string));
        match Kind::of(event.event_type) {
            Kind::Started => self.aside.started = Some(named),
            Kind::Ended => {
                if self.aside.started.as_ref() != Some(&named) {
                    return;
                }
                self.aside.started = None;
            }
            Kind::Requested => return self.asked_live(named.0),
            Kind::Checkpointed | Kind::Other => return,
        }
        let aside = &self.aside;
        for row in &mut self.rows {
            row.live = aside.lights(row);
        }
    }

    /// The request of `effect_id`: where the live start names its effect
    /// and no row is live, the start came before it, and its attempt's seat
    /// row under the request is opened, live.
    fn asked_live(&mut self, effect_id: Option<String>) {
        let Some((Some(effect), attempt)) = self.aside.started.clone() else {
            return;
        };
        if effect_id.as_ref() != Some(&effect) || self.rows.iter().any(|row| row.live) {
            return;
        }
        let request = self.by_effect[&effect];
        self.row(request, attempt.as_deref(), Member::Seat);
    }
}

/// A live line: its label, and the last `seat-turn` under it, with its seq.
type Line = (String, Option<(u64, (String, String))>);

/// A line's label, as the trail writes it: its seat, and its member after
/// a `:`.
fn label(seat: &str, member: &Member) -> String {
    match member {
        Member::Seat => seat.to_string(),
        Member::Named(member) => format!("{seat}:{member}"),
    }
}

/// `turn` under `label` among `lines`: a line opened for a label not yet
/// among them, and each line's turn the one stated last, by its seq, which
/// no two of a line's turns share.
fn heard(lines: &mut Vec<Line>, label: String, turn: Option<(u64, (String, String))>) {
    match lines.iter_mut().find(|(known, _)| *known == label) {
        Some((_, kept)) => *kept = kept.take().max(turn),
        None => lines.push((label, turn)),
    }
}

/// The live block (decision 0089 slice 2b), each line its label and the
/// last `seat-turn` under it, its turn and tool as shown, or none, as the
/// trail reads the run's last start while no terminal naming its effect
/// and attempt has followed it: a line per label its attempt's records
/// carry, in the order of each label's first record; a record before the
/// effect's first request labelled by the seat `?`, and a later one by the
/// seat the effect was last asked by that names one, else `?`; one bare
/// line for that seat while no record of the attempt is kept; and none
/// while no row is live. A start naming no effect, or an effect never
/// asked, is in no row, so its block reads as none.
pub fn live(rows: &[SeatRow]) -> Vec<(String, Option<(String, String)>)> {
    let Some(open) = rows.iter().find(|row| row.live) else {
        return Vec::new();
    };
    let effect = || rows.iter().filter(|row| row.effect_id == open.effect_id);
    // Every row keeps the seat its request hired, and every request keeps
    // a row of its own, so the effect's rows name each request's seat.
    let seat = |through: u64| {
        let asked = effect().filter(|row| row.request_seq <= through);
        let named = asked.filter_map(|row| {
            let seat = row.seat.as_deref().filter(|seat| !seat.is_empty());
            Some((row.request_seq, seat?))
        });
        let last = named.max_by_key(|(asked, _)| *asked);
        last.map_or("?", |(_, seat)| seat)
    };
    let mut lines = Vec::new();
    let early = effect().flat_map(|row| &row.listed.lines);
    for line in early.filter(|line| line.attempt == open.attempt_id) {
        let turn = line.turn.clone().map(|turn| (0, turn));
        heard(&mut lines, label("?", &line.member), turn);
    }
    let mut kept: Vec<&SeatRow> = rows.iter().filter(|row| row.live).collect();
    kept.retain(|row| row.first.is_some());
    kept.sort_by_key(|row| row.first.as_ref().map(|first| first.seq));
    for row in kept {
        let turn = row.turn.as_ref();
        let turn = turn.map(|(seq, turn)| (*seq, (turn.turn.clone(), turn.tool.clone())));
        heard(&mut lines, label(seat(row.request_seq), &row.member), turn);
    }
    if lines.is_empty() {
        return vec![(seat(u64::MAX).to_string(), None)];
    }
    let lines = lines.into_iter();
    lines
        .map(|(label, turn)| (label, turn.map(|(_, turn)| turn)))
        .collect()
}

#[cfg(test)]
mod tests;
