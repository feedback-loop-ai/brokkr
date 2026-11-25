//! What a failed engine-test assertion prints, so a failure says why
//! (#1110): built once for the cool-down tests and the deadline tests
//! (decision 0071 ruling 5).
//!
//! The runtime's test build includes this file once through `#[path]`; it
//! is test code by its path, so it stays outside the coverage denominator.

use std::path::Path;

use crate::engine::Engine;

/// The payload fields that say what an event did: a failure's words, a
/// refusal, a wait, each panel member's outcome, any cleanup left
/// unproven, and a park's cause (#1132).
const SAID: [&str; 8] = [
    "cause",
    "checkpoint",
    "error",
    "reason",
    "refused",
    "cleanup",
    "unresolved_members",
    "waits",
];

/// The run's journal, every shim log in `dir`, and the limits each seat
/// gave its attempts: the journal's event types in order with what each
/// said, each `<tag>.log` a fixture driver wrote beside the test (in name
/// order), and each seat's configured limit under the run's host file.
pub(crate) fn witness(engine: &Engine, dir: &Path) -> String {
    let mut seen = String::from("\njournal:");
    for event in engine.store.load(&engine.run_id).unwrap() {
        seen.push_str(&format!("\n  {} {}", event.seq, event.event_type.as_str()));
        for field in SAID {
            if let Some(value) = event.payload.get(field) {
                seen.push_str(&format!("\n    {field}: {value}"));
            }
        }
    }
    seen.push_str("\nshims:");
    for (tag, lines) in shims(dir) {
        seen.push_str(&format!("\n  {tag}: {lines:?}"));
    }
    seen.push_str(&format!("\ndeadlines (host {:?}):", engine.cooldown.host));
    for (name, seat) in &engine.bundle.seats {
        let limits = &seat.limits;
        seen.push_str(&format!(
            "\n  {name}: {}s per attempt and probe, {} attempts",
            limits.timeout_seconds, limits.max_attempts
        ));
    }
    seen
}

/// Every `<tag>.log` directly in `dir`, by tag, with its lines.
fn shims(dir: &Path) -> Vec<(String, Vec<String>)> {
    let mut shims: Vec<(String, Vec<String>)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let tag = name.strip_suffix(".log")?.to_string();
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            Some((tag, text.lines().map(str::to_string).collect()))
        })
        .collect();
    shims.sort();
    shims
}
