//! The optional run/started entry record (decision 0094, proposed).
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const ENTRY_NAME_MAX: usize = 64;

pub fn is_entry_name(name: &str) -> bool {
    name.len() <= ENTRY_NAME_MAX && crate::policy::is_identifier(name)
}

pub fn is_native_run_id(id: &str) -> bool {
    let Some((slug, suffix)) = id.rsplit_once('-') else {
        return false;
    };
    slug.len() <= 32
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && suffix.len() == 8
        && suffix
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEntry {
    pub phase: String,
    pub continues: Vec<ContinuedRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuedRun {
    pub run_id: String,
    pub feature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EntryRead {
    #[error("entry has an unreadable shape: {detail}")]
    Shape { detail: String },
    #[error("entry phase '{phase}' is outside the entry-name grammar")]
    Phase { phase: String },
    #[error("continued run '{run_id}' is outside the native run-id grammar")]
    RunId { run_id: String },
    #[error("continued run '{run_id}' is named twice")]
    Twice { run_id: String },
}

pub fn check_record<'a>(
    phase: &str,
    run_ids: impl IntoIterator<Item = &'a str>,
) -> Result<(), EntryRead> {
    if !is_entry_name(phase) {
        return Err(EntryRead::Phase {
            phase: phase.into(),
        });
    }
    let mut seen = BTreeSet::new();
    for id in run_ids {
        if !seen.insert(id) {
            return Err(EntryRead::Twice { run_id: id.into() });
        }
        if !is_native_run_id(id) {
            return Err(EntryRead::RunId { run_id: id.into() });
        }
    }
    Ok(())
}

impl RunEntry {
    pub fn check(&self) -> Result<(), EntryRead> {
        check_record(
            &self.phase,
            self.continues.iter().map(|run| run.run_id.as_str()),
        )
    }

    pub fn of_started(payload: &Value) -> Result<Option<Self>, EntryRead> {
        let Some(raw) = payload.get("entry") else {
            return Ok(None);
        };
        let entry: Self =
            serde_json::from_value(raw.clone()).map_err(|error| EntryRead::Shape {
                detail: error.to_string(),
            })?;
        entry.check()?;
        Ok(Some(entry))
    }
}

#[cfg(test)]
mod tests;
