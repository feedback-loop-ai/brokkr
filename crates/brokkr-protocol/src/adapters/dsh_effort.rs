//! A dsh seat's effort on the two channels the releases read it from,
//! and the level a request header says the seat ran at (decision 0035,
//! addenda 2026-09-05 and 2026-10-17).
//!
//! dsh 0.1 reads the level from the seat's settings document and ignores
//! it on the composition row; dsh 0.2 dropped the settings document's
//! fields and reads it on the composition row (measured 2026-10-17 on
//! 0.1.5-rc.1 and 0.2.1-rc.2, #728). The driver writes it on both.

use serde_json::{Map, Value};

use super::{dsh_model_row, effort_token, Row, EFFORT_NOT_REPORTED};

/// The pinned level as the one YAML scalar both channels write: clamped
/// to one bounded word by `effort_token`, and single-quoted with its
/// quotes doubled anyway, because both channels are YAML and the
/// discipline is the transcript root's.
pub(super) fn pinned_level(effort: &str) -> Result<String, String> {
    let Some(effort) = effort_token(effort) else {
        return Err("dsh driver: the pinned effort is not one bounded word".to_string());
    };
    Ok(format!("'{}'", effort.replace('\'', "''")))
}

/// The pinned-model row with the pinned level on it, where dsh 0.2's
/// `agent-default-model` reads it as a live composition field. 0.1
/// ignores the key without a word and reads the settings document
/// instead, so a seat pinned on either release reaches the wire.
pub(super) fn model_row(model: &str, effort: Option<&str>) -> Result<String, String> {
    let mut row = dsh_model_row(model)?;
    if let Some(effort) = effort {
        row.push_str(&format!("    reasoningEffort: {}\n", pinned_level(effort)?));
    }
    Ok(row)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum AppliedLevel {
    Reported(String),
    Masked,
    Unchanged,
}

/// The level a request header attributes to the seat. A malformed
/// header or a level marked as the adapter's default leaves it standing.
pub(super) fn applied_level(event: &Row<'_>) -> AppliedLevel {
    let Some(header) = event.at("/data/header") else {
        return AppliedLevel::Unchanged;
    };
    if adapter_defaulted_or_unreadable(&header) {
        return AppliedLevel::Unchanged;
    }
    reported_level(&header)
}

fn adapter_defaulted_or_unreadable(header: &Row<'_>) -> bool {
    const DEFAULTS: &str = "/adapterDefaults";
    const DEFAULTED: &str = "/adapterDefaults/reasoningEffort";

    if header.has(DEFAULTS) && !header.is_object(DEFAULTS) {
        return true;
    }
    match header.flag(DEFAULTED) {
        Some(defaulted) => defaulted,
        None => header.has(DEFAULTED),
    }
}

fn reported_level(header: &Row<'_>) -> AppliedLevel {
    const LEVEL: &str = "/config/reasoningEffort";
    let Some(effort) = header.text(LEVEL) else {
        return AppliedLevel::Unchanged;
    };
    if !header.equals(LEVEL, &effort) {
        return AppliedLevel::Masked;
    }
    effort_token(&effort).map_or(AppliedLevel::Unchanged, AppliedLevel::Reported)
}

/// The level each step shows: the seat's retained one, or not reported
/// while a header whose level a binding masks stands. dsh writes one
/// header per request and every step after it runs under it, so a masked
/// header settles all of them, until the next header replaces it; the
/// seat keeps its last valid or seeded level (decision 0114, D4).
#[derive(Default)]
pub(super) enum Echo {
    #[default]
    Retained,
    Masked,
}

impl Echo {
    pub(super) fn header(&mut self, event: &Row<'_>, meta: &mut Map<String, Value>) {
        *self = match applied_level(event) {
            AppliedLevel::Reported(effort) => {
                meta.insert("effort".into(), Value::String(effort));
                Self::Retained
            }
            AppliedLevel::Masked => Self::Masked,
            AppliedLevel::Unchanged => Self::Retained,
        };
    }

    pub(super) fn shown(&self, meta: &Map<String, Value>) -> String {
        match self {
            Self::Masked => EFFORT_NOT_REPORTED.to_owned(),
            Self::Retained => meta
                .get("effort")
                .and_then(Value::as_str)
                .unwrap_or(EFFORT_NOT_REPORTED)
                .to_owned(),
        }
    }
}
