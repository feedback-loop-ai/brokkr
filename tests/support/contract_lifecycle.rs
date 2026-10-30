//! Test support for decision 0082 ruling 4's private rows: read the one
//! lifecycle table from the workspace root at run time. The owning crate
//! drives the loader; this module only finds the row. Test code by its
//! `#[cfg(test)]` declaration, so it stays outside the coverage denominator.

use std::path::PathBuf;

pub(crate) fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// One row's `Version` and `Reader` cells, read from `versioning.md`.
pub(crate) struct Row {
    pub(crate) version: String,
    pub(crate) reader: String,
}

/// The one table, embedded so `brokkr-core`'s pure test build does no I/O
/// (decision 0071 ruling 1); cargo re-reads it at every compile, and an edit
/// to the guide rebuilds this test.
const GUIDE_TEXT: &str = include_str!("../../docs/guides/versioning.md");

pub(crate) fn row(file: &str) -> Row {
    let guide = workspace().join("docs/guides/versioning.md");
    let text = GUIDE_TEXT;
    let heading =
        "| Version | File | Class | Minted under | Writer | Reader | State | Last written | Reader until |";
    let start = text
        .lines()
        .position(|line| line == heading)
        .expect("the lifecycle table")
        + 2;
    let line = text
        .lines()
        .skip(start)
        .take_while(|line| line.starts_with('|'))
        .find(|line| {
            line.trim_matches('|')
                .split('|')
                .map(str::trim)
                .nth(1)
                .map(|cell| cell.trim_matches('`'))
                == Some(file)
        })
        .unwrap_or_else(|| panic!("{file} has a lifecycle row in {}", guide.display()));
    let cells: Vec<&str> = line
        .trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().trim_matches('`'))
        .collect();
    Row {
        version: cells[0].to_string(),
        reader: cells[5].to_string(),
    }
}

/// A sealed event, for the private rows whose loader reads one.
pub(crate) fn event(
    event_type: brokkr_core::EventType,
    payload: serde_json::Value,
    seq: u64,
) -> brokkr_core::EventEnvelope {
    brokkr_core::EventEnvelope {
        run_id: "run".into(),
        seq,
        event_id: format!("event-{seq}"),
        event_schema_version: brokkr_core::envelope::EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-01-01T00:00:00Z".into(),
        previous_hash: brokkr_core::canonical::ZERO_HASH.into(),
        event_hash: String::new(),
    }
    .sealed()
}
