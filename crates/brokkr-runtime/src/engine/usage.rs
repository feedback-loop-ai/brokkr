//! What an attempt's process tree used, on its ending (decision 0112
//! ruling 3): the optional `usage` object of
//! `contracts/effect-usage.v1.schema.json` on `effect/succeeded`,
//! `effect/failed` and `effect/indeterminate`, at `event_schema: 1`. A
//! single seat carries its report's measures; a panel carries one entry
//! per measured member. The fold reads none of it, and nothing that
//! settles, retries or admits an attempt does.
//!
//! The attempt's report is held here as it concludes, keyed by its
//! attempt id, and the fenced append stamps it on whichever ending that
//! attempt journals, so every arm of a conclusion, the fence's own
//! refusals and a held outcome landed later included, carries it without
//! each building it. An ending of another attempt never matches the key,
//! and a report without usage holds nothing, so the event keeps its v1
//! bytes.

use std::collections::BTreeMap;

use brokkr_core::envelope::EventType;
use brokkr_protocol::{AttemptReport, AttemptUsage};
use serde::Serialize;
use serde_json::Value;

use super::DriverRun;

/// The usage `effect-usage/v1` writes: a seat's own measures, or a
/// panel's, by member. Numbers only, so no seat-controlled text reaches
/// the journal through it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
enum Usage {
    Seat(AttemptUsage),
    Panel {
        members: BTreeMap<String, AttemptUsage>,
    },
}

/// The concluding attempt's usage, and the attempt it is of.
#[derive(Debug, Default)]
pub(super) struct Pending {
    held: Option<(String, Usage)>,
}

impl Pending {
    /// Hold a single seat's `run` for the ending of `attempt_id`: a driver
    /// that never spawned measured nothing.
    pub(super) fn seat(&mut self, attempt_id: &str, run: &DriverRun) {
        self.held = match run {
            DriverRun::Ran(report) => report.usage.as_deref().copied(),
            DriverRun::SpawnFailed(_) => None,
        }
        .map(|usage| (attempt_id.to_string(), Usage::Seat(usage)));
    }

    /// Hold a panel's `reports`, one entry per member a tick measured,
    /// for the ending of `attempt_id`.
    pub(super) fn panel(&mut self, attempt_id: &str, reports: &[(String, AttemptReport)]) {
        let members: BTreeMap<String, AttemptUsage> = reports
            .iter()
            .filter_map(|(name, report)| Some((name.clone(), *report.usage.as_deref()?)))
            .collect();
        self.held =
            (!members.is_empty()).then(|| (attempt_id.to_string(), Usage::Panel { members }));
    }

    /// `payload` with the held usage stamped on it, when `event_type` is
    /// an ending of the held attempt; otherwise `payload` as it was.
    pub(super) fn stamp(&self, event_type: EventType, mut payload: Value) -> Value {
        let ending = matches!(
            event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        );
        let Some((attempt_id, usage)) = self.held.as_ref().filter(|_| ending) else {
            return payload;
        };
        if let Some(fields) = payload
            .as_object_mut()
            .filter(|fields| fields.get("attempt_id").and_then(Value::as_str) == Some(attempt_id))
        {
            fields.insert("usage".into(), serde_json::json!(usage));
        }
        payload
    }
}

#[cfg(test)]
mod tests;
