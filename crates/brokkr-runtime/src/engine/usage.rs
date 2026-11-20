//! What an attempt's process tree used, on its ending (decision 0112
//! ruling 3): the optional `usage` object of
//! `contracts/effect-usage.v1.schema.json` on `effect/succeeded`,
//! `effect/failed` and `effect/indeterminate`, at `event_schema: 1`. A
//! single seat carries its report's measures; an attempt of several
//! drivers carries one entry per measured driver under `members`: a
//! panel's by member, a sequence's by the tag its checkpoints ride under,
//! `<step>` or `<step>:<member>`. The fold reads none of it, and nothing
//! that settles, retries or admits an attempt does.
//!
//! The attempt's reports are held here as its drivers settle, keyed by
//! its attempt id, and the fenced append stamps them on whichever ending
//! that attempt journals, so every arm of a conclusion, a sequence's early
//! endings, the fence's own refusals and a held outcome landed later
//! included, carries it without each building it. An ending of another
//! attempt never matches the key, and an attempt no tick measured holds
//! nothing, so its event keeps its v1 bytes.
//!
//! The stamp inserts one key into the payload the append already holds
//! as a `Value` (fence.rs's `append_raw`), rather than a flattened field
//! on each ending's payload type: a dozen endings are built across the
//! engine's modules, several as `json!` objects, and one stamp at the one
//! append keeps the fact in one home (decision 0071 ruling 5). Nothing is
//! read back out of the payload.

use std::collections::BTreeMap;

use brokkr_core::envelope::EventType;
use brokkr_protocol::{AttemptReport, AttemptUsage};
use serde::Serialize;
use serde_json::Value;

use super::DriverRun;

/// The usage `effect-usage/v1` writes: a seat's own measures, or several
/// drivers', by member. Numbers only, so no seat-controlled text reaches
/// the journal through it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
enum Usage {
    Seat(AttemptUsage),
    Members {
        members: BTreeMap<String, AttemptUsage>,
    },
}

/// The concluding attempt's usage, and the attempt it is of.
#[derive(Debug, Default)]
pub(super) struct Pending {
    held: Option<(String, Usage)>,
}

/// What `run` measured: a driver that never spawned measured nothing.
fn measured(run: &DriverRun) -> Option<AttemptUsage> {
    match run {
        DriverRun::Ran(report) => report.usage.as_deref().copied(),
        DriverRun::SpawnFailed(_) => None,
    }
}

impl Pending {
    /// Hold a single seat's `run` for the ending of `attempt_id`.
    pub(super) fn seat(&mut self, attempt_id: &str, run: &DriverRun) {
        self.held = measured(run).map(|usage| (attempt_id.to_string(), Usage::Seat(usage)));
    }

    /// Hold a panel's `reports`, one entry per member a tick measured,
    /// for the ending of `attempt_id`.
    pub(super) fn panel(&mut self, attempt_id: &str, reports: &[(String, AttemptReport)]) {
        self.held = None;
        self.panel_step(attempt_id, "", reports);
    }

    /// Add a sequence step's single driver `run`, under the step's name,
    /// to what `attempt_id` holds.
    pub(super) fn step(&mut self, attempt_id: &str, step: &str, run: &DriverRun) {
        let entry = measured(run).map(|usage| (step.to_string(), usage));
        self.add(attempt_id, entry);
    }

    /// Add a sequence's panel step's `reports`, each member under its
    /// `tag_prefix`ed tag, to what `attempt_id` holds.
    pub(super) fn panel_step(
        &mut self,
        attempt_id: &str,
        tag_prefix: &str,
        reports: &[(String, AttemptReport)],
    ) {
        let entries = reports.iter().filter_map(|(name, report)| {
            Some((format!("{tag_prefix}{name}"), *report.usage.as_deref()?))
        });
        self.add(attempt_id, entries);
    }

    /// `entries` beside the members `attempt_id` already holds; what
    /// another attempt held is dropped.
    fn add(&mut self, attempt_id: &str, entries: impl IntoIterator<Item = (String, AttemptUsage)>) {
        let mut members = match self.held.take() {
            Some((held, Usage::Members { members })) if held == attempt_id => members,
            _ => BTreeMap::new(),
        };
        members.extend(entries);
        self.held =
            (!members.is_empty()).then(|| (attempt_id.to_string(), Usage::Members { members }));
    }

    /// `payload` with the held usage stamped on it, when `event_type` is
    /// an ending of the held attempt, as the append names it; otherwise
    /// `payload` as it was.
    pub(super) fn stamp(
        &self,
        event_type: EventType,
        attempt_id: Option<&str>,
        mut payload: Value,
    ) -> Value {
        let ending = matches!(
            event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        let held = self.held.as_ref();
        let Some((_, usage)) = held.filter(|(held, _)| ending && attempt_id == Some(held.as_str()))
        else {
            return payload;
        };
        if let Some(fields) = payload.as_object_mut() {
            fields.insert("usage".into(), serde_json::json!(usage));
        }
        payload
    }
}

#[cfg(test)]
mod tests;
