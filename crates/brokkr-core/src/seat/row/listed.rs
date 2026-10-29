//! What a run's phases and visits read of its seat rows (#753; decision
//! 0089 slice 2b): the steps a sequence finished and the members its
//! records tag, from which the rail draws a phase's inner columns, and
//! every model a row's records reported, which the dashboard's path joins
//! for a visit. Each list is kept typed (ruling 10): one column of
//! length-prefixed entries, read back only as its writer writes it, so a
//! list edited past its writer is no row and reads stale, never wrong.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::kept::both;
use super::said::Early;
use super::{Member, SeatRow, Seats};
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;

/// What a row's records list: each step a `sequence-step-finished` record
/// names, with the seq it was first finished at on this row; the seq of a
/// member's first record of any other step, which tags the member; every
/// model its records and its effect's concluding terminals reported, each
/// once, in the order first reported; and, on a request's own row, each
/// member a record before the effect's first request tagged, with the seq
/// it first did, and the live lines such records open (`said::Early`), in
/// the order first opened.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Listed {
    pub(super) steps: Vec<(u64, String)>,
    pub(super) tagged: Option<u64>,
    pub(super) models: Vec<String>,
    pub(super) early: Vec<(u64, String)>,
    pub(super) lines: Vec<Early>,
}

impl Listed {
    /// A checkpoint at `seq` on `member`'s row, as the rail reads its step
    /// and member, and the dashboard its model.
    pub(super) fn noted(&mut self, seq: u64, checkpoint: &Value, member: &Member) {
        if !self.finished(seq, checkpoint) && *member != Member::Seat {
            self.tagged.get_or_insert(seq);
        }
        self.served(field(checkpoint, "model"));
    }

    /// Whether `checkpoint` is a `sequence-step-finished` record, keeping
    /// the step it names at `seq` where this row has not kept it yet.
    fn finished(&mut self, seq: u64, checkpoint: &Value) -> bool {
        let finished = field(checkpoint, "step") == Some("sequence-step-finished");
        let name = field(checkpoint, "step_name").filter(|_| finished);
        if let Some(name) = name.filter(|name| !known(&self.steps, name)) {
            self.steps.push((seq, name.to_string()));
        }
        finished
    }

    /// A checkpoint of `attempt` at `seq` before its effect's first
    /// request, as the rail reads it: its step, or the member it tags; and
    /// as the live block reads it, its line. The dashboard joins its model
    /// to no visit.
    fn early(&mut self, seq: u64, attempt: Option<&str>, checkpoint: &Value) {
        let finished = self.finished(seq, checkpoint);
        let member = field(checkpoint, "member").filter(|_| !finished);
        if let Some(member) = member.filter(|member| !known(&self.early, member)) {
            self.early.push((seq, member.to_string()));
        }
        Early::noted(&mut self.lines, attempt, checkpoint);
    }

    /// A model its records or its effect's concluding terminal reported.
    pub(super) fn served(&mut self, model: Option<&str>) {
        let known = |model: &&str| self.models.iter().any(|known| known == model);
        if let Some(model) = model.filter(|model| !known(model)) {
            self.models.push(model.to_string());
        }
    }

    /// The steps' column: each step's seq and its name.
    pub(super) fn steps_kept(&self) -> Option<String> {
        paired(&self.steps)
    }

    /// The models' column.
    pub(super) fn models_kept(&self) -> Option<String> {
        kept(self.models.iter().map(String::as_str))
    }

    /// The early tags' column: each member's seq and its tag.
    pub(super) fn early_kept(&self) -> Option<String> {
        paired(&self.early)
    }

    /// The early lines' column: each line's attempt, member, turn and
    /// tool, each an optional entry ([`optional`]).
    pub(super) fn lines_kept(&self) -> Option<String> {
        let each = self.lines.iter().flat_map(|line| {
            let member = match &line.member {
                Member::Seat => None,
                Member::Named(member) => Some(member.as_str()),
            };
            let turn = line.turn.as_ref();
            let (turn, tool) = (
                turn.map(|turn| turn.0.as_str()),
                turn.map(|turn| turn.1.as_str()),
            );
            [line.attempt.as_deref(), member, turn, tool].map(optional)
        });
        let each: Vec<String> = each.collect();
        kept(each.iter().map(String::as_str))
    }

    /// The lists its columns hold, or `None` for any its writer does not
    /// write: an entry cut short or its length not as written, a seq that
    /// is not one, a step or a tag without its seq, a step, a model or a
    /// tag twice, steps or tags out of their seqs' order, a line as
    /// [`lines`] refuses it, or no entry at all.
    pub(super) fn of_kept(
        steps: Option<String>,
        tagged: Option<u64>,
        models: Option<String>,
        early: Option<String>,
        lines: Option<String>,
    ) -> Option<Listed> {
        let served = models.as_deref().map_or(Some(Vec::new()), entries)?;
        let listed = Listed {
            steps: pairs(steps.as_deref())?,
            tagged,
            models: served.into_iter().map(str::to_string).collect(),
            early: pairs(early.as_deref())?,
            lines: self::lines(lines.as_deref())?,
        };
        let ordered = |pairs: &[(u64, String)]| pairs.windows(2).all(|pair| pair[0].0 < pair[1].0);
        let names = |pairs: &'_ [(u64, String)]| once(pairs.iter().map(|(_, name)| name));
        let lists = [&listed.steps, &listed.early];
        let pairs_read = lists.iter().all(|list| ordered(list) && names(list));
        let canonical = listed.steps_kept() == steps
            && listed.models_kept() == models
            && listed.early_kept() == early
            && listed.lines_kept() == lines;
        (pairs_read && once(listed.models.iter()) && canonical).then_some(listed)
    }
}

/// An optional text as one entry: `-` for none, else `+` and the text.
fn optional(text: Option<&str>) -> String {
    text.map_or_else(|| "-".to_string(), |text| format!("+{text}"))
}

/// The optional text an [`optional`] entry holds, or `None` for another.
fn optional_of(entry: &str) -> Option<Option<String>> {
    match entry.strip_prefix('+') {
        Some(text) => Some(Some(text.to_string())),
        None => (entry == "-").then_some(None),
    }
}

/// The lines a column [`Listed::lines_kept`] keeps holds, none for
/// `None`, or `None` where it cuts a line short, an entry is no optional
/// text, a turn is kept without its tool or a tool without its turn, or
/// two lines name one attempt and member.
fn lines(text: Option<&str>) -> Option<Vec<Early>> {
    let read = text.map_or(Some(Vec::new()), entries)?;
    read.chunks(4)
        .try_fold(Vec::new(), |mut found: Vec<Early>, line| {
            let [attempt, member, turn, tool] = line else {
                return None;
            };
            let [attempt, member, turn, tool] =
                [attempt, member, turn, tool].map(|entry| optional_of(entry));
            let turn = both(turn?, tool?)?;
            let (attempt, member) = (attempt?, Member::of(member?.as_deref()));
            let known = |line: &Early| line.attempt == attempt && line.member == member;
            if found.iter().any(known) {
                return None;
            }
            found.push(Early {
                attempt,
                member,
                turn,
            });
            Some(found)
        })
}

/// What the rail and the dashboard read of a run's records beside its
/// rows (fix round 753p2): the steps and members a checkpoint before its
/// effect's first request lists, which the rail buckets by effect id
/// however early, and each effect's open attempt as the dashboard reads
/// it, by effect id across its requests, not by request as a participant
/// reads it; and the effect and attempt the run's last start names, as
/// the live block reads them (`said`), until a terminal names both.
#[derive(Debug, Clone, Default)]
pub(super) struct Aside {
    early: BTreeMap<String, Listed>,
    open: BTreeMap<String, String>,
    pub(super) started: Option<(Option<String>, Option<String>)>,
}

impl Aside {
    /// Whether `row` is of the effect and attempt the live start names.
    pub(super) fn lights(&self, row: &SeatRow) -> bool {
        let started = self.started.as_ref();
        started.is_some_and(|(effect_id, attempt_id)| {
            effect_id.as_deref() == Some(row.effect_id.as_str()) && *attempt_id == row.attempt_id
        })
    }

    /// `row`, live as [`Aside::lights`] reads it.
    pub(super) fn lit(&self, mut row: SeatRow) -> SeatRow {
        row.live = self.lights(&row);
        row
    }
}

impl Seats {
    /// `event`, beside the rows: a checkpoint before its effect's first
    /// request is listed aside, and that request's own row takes the list;
    /// a start naming an attempt opens it for its effect; and a terminal
    /// concluding its effect's open attempt lists the model its result
    /// reports on the own row of the effect's latest request, the visit the
    /// dashboard joins it to.
    pub(super) fn aside(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        let Some(effect_id) = field(payload, "effect_id") else {
            return;
        };
        let attempt = field(payload, "attempt_id");
        let own = self.by_effect.get(effect_id);
        let own = own.map(|&request| self.at[&(request, None)][&Member::Seat]);
        let aside = &mut self.aside;
        match Kind::of(event.event_type) {
            Kind::Requested => {
                if let (Some(early), Some(own)) = (aside.early.remove(effect_id), own) {
                    self.rows[own].listed = early;
                }
            }
            Kind::Checkpointed if own.is_none() => {
                let checkpoint = payload.get("checkpoint").unwrap_or(&Value::Null);
                let early = aside.early.entry(effect_id.to_string()).or_default();
                early.early(event.seq, attempt, checkpoint);
            }
            Kind::Started => {
                if let Some(attempt) = attempt {
                    aside
                        .open
                        .insert(effect_id.to_string(), attempt.to_string());
                }
            }
            Kind::Ended => {
                if aside.open.get(effect_id).map(String::as_str) != attempt {
                    return;
                }
                aside.open.remove(effect_id);
                let model = payload.pointer("/result/model").and_then(Value::as_str);
                if let Some(own) = own {
                    self.rows[own].listed.served(model);
                }
            }
            Kind::Checkpointed | Kind::Other => {}
        }
    }
}

/// What an event is to what is read beside the rows: an effect's request,
/// start, checkpoint or terminal, or another kind, which none of it reads.
pub(super) enum Kind {
    Requested,
    Started,
    Checkpointed,
    Ended,
    Other,
}

impl Kind {
    pub(super) fn of(event_type: EventType) -> Kind {
        match event_type {
            EventType::EffectRequested => Kind::Requested,
            EventType::EffectStarted => Kind::Started,
            EventType::EffectCheckpointed => Kind::Checkpointed,
            EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate => Kind::Ended,
            EventType::RunStarted
            | EventType::PhaseEntered
            | EventType::TransitionDecided
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => Kind::Other,
        }
    }
}

/// Whether `pairs` holds `name`.
fn known(pairs: &[(u64, String)], name: &str) -> bool {
    pairs.iter().any(|(_, known)| known == name)
}

/// Whether each of `values` is there once.
fn once<'a>(mut values: impl Iterator<Item = &'a String>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| seen.insert(value))
}

/// `pairs` kept in one column: each pair's seq, and then its name.
fn paired(pairs: &[(u64, String)]) -> Option<String> {
    let seqs: Vec<String> = pairs.iter().map(|(seq, _)| seq.to_string()).collect();
    let names = pairs.iter().map(|(_, name)| name.as_str());
    let each = seqs.iter().map(String::as_str).zip(names);
    kept(each.flat_map(|(seq, name)| [seq, name]))
}

/// The pairs a column [`paired`] keeps holds, none for `None`, or `None`
/// where it cuts one short or a seq is not one.
fn pairs(text: Option<&str>) -> Option<Vec<(u64, String)>> {
    let read = text.map_or(Some(Vec::new()), entries)?;
    read.chunks(2).try_fold(Vec::new(), |mut found, pair| {
        let [seq, name] = pair else { return None };
        found.push((seq.parse().ok()?, (*name).to_string()));
        Some(found)
    })
}

/// `entries` kept in one column: each its length in bytes, a `:`, and
/// itself; `None` for no entry.
pub(super) fn kept<'a>(entries: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut text = String::new();
    for entry in entries {
        text.push_str(&entry.len().to_string());
        text.push(':');
        text.push_str(entry);
    }
    (!text.is_empty()).then_some(text)
}

/// The entries `text` keeps, or `None` where it cuts one short.
pub(super) fn entries(text: &str) -> Option<Vec<&str>> {
    let mut rest = text;
    let mut found = Vec::new();
    while !rest.is_empty() {
        let (length, tail) = rest.split_once(':')?;
        let length: usize = length.parse().ok()?;
        found.push(tail.get(..length)?);
        rest = tail.get(length..)?;
    }
    Some(found)
}

/// Each of `seen`, once, by the seq it was first seen at.
fn firsts(mut seen: Vec<(u64, &str)>) -> Vec<String> {
    seen.sort_by_key(|(seq, _)| *seq);
    let mut found: Vec<String> = Vec::new();
    for (_, value) in seen {
        if !found.iter().any(|known| known == value) {
            found.push(value.to_string());
        }
    }
    found
}

/// What the rail reads of `effect_id`, the effect a phase last asked for,
/// from every row of it: the steps its records finished, by name in the
/// order first finished, and the members they tag, in the order of each
/// one's first record of another step than a finished one, a record
/// before the effect's first request among them.
pub fn inner(rows: &[SeatRow], effect_id: &str) -> (Vec<String>, Vec<String>) {
    let rows = || rows.iter().filter(|row| row.effect_id == effect_id);
    let steps = rows().flat_map(|row| &row.listed.steps);
    let finished = steps.map(|(seq, name)| (*seq, name.as_str()));
    let tags = rows().filter_map(|row| match &row.member {
        Member::Named(tag) => Some((row.listed.tagged?, tag.as_str())),
        Member::Seat => None,
    });
    let early = rows().flat_map(|row| &row.listed.early);
    let early = early.map(|(seq, tag)| (*seq, tag.as_str()));
    (
        firsts(finished.collect()),
        firsts(tags.chain(early).collect()),
    )
}

/// Every model reported by the rows of the requests asked at a seq from
/// `from` and before `to`, a visit's from its entry to the next: the set
/// the dashboard's path joins, sorted as it shows them.
pub fn models(rows: &[SeatRow], from: u64, to: u64) -> BTreeSet<String> {
    let asked = |row: &&SeatRow| from <= row.request_seq && row.request_seq < to;
    let rows = rows.iter().filter(asked);
    rows.flat_map(|row| row.listed.models.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests;
