//! What a run's phases and visits read of its seat rows (#753; decision
//! 0089 slice 2b): the steps a sequence finished and the members its
//! records tag, from which the rail draws a phase's inner columns, and
//! every model a row's records reported, which the dashboard's path joins
//! for a visit. Each list is kept typed (ruling 10): one column of
//! length-prefixed entries, read back only as its writer writes it, so a
//! list edited past its writer is no row and reads stale, never wrong.

use std::collections::BTreeSet;

use serde_json::Value;

use super::{Member, SeatRow};
use crate::residual::text as field;

/// What a row's records list: each step a `sequence-step-finished` record
/// names, with the seq it was first finished at on this row; the seq of a
/// member's first record of any other step, which tags the member; and
/// every model its records and its attempt's terminal reported, each once,
/// in the order first reported.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Listed {
    pub(super) steps: Vec<(u64, String)>,
    pub(super) tagged: Option<u64>,
    pub(super) models: Vec<String>,
}

impl Listed {
    /// A checkpoint at `seq` on `member`'s row, as the rail reads its step
    /// and member, and the dashboard its model.
    pub(super) fn noted(&mut self, seq: u64, checkpoint: &Value, member: &Member) {
        if field(checkpoint, "step") == Some("sequence-step-finished") {
            let name = field(checkpoint, "step_name");
            let known = |name: &&str| self.steps.iter().any(|(_, step)| step == name);
            if let Some(name) = name.filter(|name| !known(name)) {
                self.steps.push((seq, name.to_string()));
            }
        } else if *member != Member::Seat {
            self.tagged.get_or_insert(seq);
        }
        self.served(field(checkpoint, "model"));
    }

    /// A model its records or its attempt's terminal reported.
    pub(super) fn served(&mut self, model: Option<&str>) {
        let known = |model: &&str| self.models.iter().any(|known| known == model);
        if let Some(model) = model.filter(|model| !known(model)) {
            self.models.push(model.to_string());
        }
    }

    /// The steps' column: each step's seq and its name.
    pub(super) fn steps_kept(&self) -> Option<String> {
        let seqs: Vec<String> = self.steps.iter().map(|(seq, _)| seq.to_string()).collect();
        let names = self.steps.iter().map(|(_, name)| name.as_str());
        kept(
            seqs.iter()
                .map(String::as_str)
                .zip(names)
                .flat_map(|(seq, name)| [seq, name]),
        )
    }

    /// The models' column.
    pub(super) fn models_kept(&self) -> Option<String> {
        kept(self.models.iter().map(String::as_str))
    }

    /// The lists its columns hold, or `None` for any its writer does not
    /// write: an entry cut short or its length not as written, a seq that
    /// is not one, a step without its seq, a step or a model twice, steps
    /// out of their seqs' order, or no entry at all.
    pub(super) fn of_kept(
        steps: Option<String>,
        tagged: Option<u64>,
        models: Option<String>,
    ) -> Option<Listed> {
        let stepped = steps.as_deref().map_or(Some(Vec::new()), entries)?;
        let served = models.as_deref().map_or(Some(Vec::new()), entries)?;
        let mut pairs = stepped.chunks(2);
        let steps_read = pairs.try_fold(Vec::new(), |mut read, pair| {
            let [seq, name] = pair else { return None };
            read.push((seq.parse().ok()?, (*name).to_string()));
            Some(read)
        })?;
        let listed = Listed {
            steps: steps_read,
            tagged,
            models: served.into_iter().map(str::to_string).collect(),
        };
        let ordered = listed.steps.windows(2).all(|pair| pair[0].0 < pair[1].0);
        let distinct =
            once(listed.steps.iter().map(|(_, name)| name)) && once(listed.models.iter());
        let canonical = listed.steps_kept() == steps && listed.models_kept() == models;
        (ordered && distinct && canonical).then_some(listed)
    }
}

/// Whether each of `values` is there once.
fn once<'a>(mut values: impl Iterator<Item = &'a String>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| seen.insert(value))
}

/// `entries` kept in one column: each its length in bytes, a `:`, and
/// itself; `None` for no entry.
fn kept<'a>(entries: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut text = String::new();
    for entry in entries {
        text.push_str(&entry.len().to_string());
        text.push(':');
        text.push_str(entry);
    }
    (!text.is_empty()).then_some(text)
}

/// The entries `text` keeps, or `None` where it cuts one short.
fn entries(text: &str) -> Option<Vec<&str>> {
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
/// one's first record of another step than a finished one.
pub fn inner(rows: &[SeatRow], effect_id: &str) -> (Vec<String>, Vec<String>) {
    let rows = || rows.iter().filter(|row| row.effect_id == effect_id);
    let steps = rows().flat_map(|row| &row.listed.steps);
    let finished = steps.map(|(seq, name)| (*seq, name.as_str()));
    let tags = rows().filter_map(|row| match &row.member {
        Member::Named(tag) => Some((row.listed.tagged?, tag.as_str())),
        Member::Seat => None,
    });
    (firsts(finished.collect()), firsts(tags.collect()))
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
