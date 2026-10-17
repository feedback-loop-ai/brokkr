//! dsh 0.2's effort channel (#728; decision 0035 addendum 2026-10-17):
//! the pinned level rides the composition row as well as the settings
//! document, and a header whose adapter defaulted the level never
//! becomes the seat's.

use super::dsh_effort::{applied_level, model_row};
use super::*;

/// The patch and the settings document one pinned seat stages.
fn staged(model: &str, effort: Option<&str>) -> (String, Option<String>) {
    let root = std::path::Path::new("/nonexistent/dsh-root");
    let overlay = dsh_seat_overlay_with(Some(model), effort, root, None, None).unwrap();
    let patch = std::fs::read_to_string(overlay.path()).unwrap();
    let document = overlay
        .settings
        .as_ref()
        .map(|file| std::fs::read_to_string(file.path()).unwrap());
    (patch, document)
}

/// Every line of a staged file that is not a comment.
fn rows(text: &str) -> Vec<&str> {
    text.lines().filter(|l| !l.starts_with('#')).collect()
}

/// One pin, both channels: 0.2 reads the level on the `agent-default-model`
/// row, 0.1 reads it in the document the `settings` row names, and the two
/// carry the same clamped, single-quoted scalar.
#[test]
fn a_pinned_level_rides_the_model_row_and_the_settings_document() {
    let (patch, document) = staged("deepseek-flash", Some("low"));
    assert!(
        patch.contains(
            "- id: agent-default-model\n  config:\n    provider: deepseek-official\n    \
             model: deepseek-flash\n    reasoningEffort: 'low'\n"
        ),
        "{patch}"
    );
    assert_eq!(patch.matches("reasoningEffort").count(), 1, "{patch}");
    assert_eq!(
        rows(&document.expect("a settings document beside the patch")),
        [
            "agent-default-model:",
            "  provider: deepseek-official",
            "  model: deepseek-flash",
            "  reasoningEffort: 'low'",
        ]
    );

    // No pin, no level on either channel.
    let (patch, document) = staged("deepseek-flash", None);
    assert!(
        patch.contains(
            "- id: agent-default-model\n  config:\n    provider: deepseek-official\n    \
             model: deepseek-flash\n"
        ),
        "{patch}"
    );
    assert!(!patch.contains("reasoningEffort"), "{patch}");
    assert_eq!(document, None);

    // The row's clamp is the document's, word for word.
    assert_eq!(
        model_row("deepseek-flash", Some("think hard")),
        Err("dsh driver: the pinned effort is not one bounded word".to_string())
    );
}

/// Fold one header and then one step from a seat whose effort stood at
/// `seed`, and return what the step's checkpoint and the seat say.
fn fold_header(header: serde_json::Value, seed: Option<&str>) -> (Value, Option<Value>) {
    let mut turns = 0u64;
    let mut meta = Map::new();
    if let Some(seed) = seed {
        meta.insert("effort".into(), Value::String(seed.into()));
    }
    let mut emitted = Vec::new();
    let header = serde_json::json!({"type": "request/header", "data": {"header": header}});
    let step = serde_json::json!({"type":"assistant/message","data":{"turn":1,"step":1,
        "message":{"source":{"model":"deepseek-flash"}}}});
    for event in [&header, &step] {
        fold_dsh_event(event, None, &mut turns, &mut meta, &mut |value| {
            emitted.push(value.clone())
        });
    }
    assert_eq!(emitted.len(), 1, "{emitted:?}");
    (emitted[0]["effort"].clone(), meta.get("effort").cloned())
}

/// The 0.2.1-rc.2 header #728 measured for a seat pinned `low` whose pin
/// did not reach the wire: the adapter's default, and the marker saying
/// so. It is never the seat's level.
#[test]
fn a_header_whose_adapter_defaulted_the_level_leaves_the_seat_effort_standing() {
    let defaulted = serde_json::json!({
        "config": {"provider": "deepseek-official", "model": "deepseek-flash",
                   "maxTokens": 256_000, "reasoningEffort": "high"},
        "adapterDefaults": {"reasoningEffort": true, "maxTokens": true}
    });
    assert_eq!(
        fold_header(defaulted.clone(), None),
        (Value::String(EFFORT_NOT_REPORTED.into()), None)
    );
    let seed = Value::String(EFFORT_NOT_APPLICABLE.into());
    assert_eq!(
        fold_header(defaulted, Some(EFFORT_NOT_APPLICABLE)),
        (seed.clone(), Some(seed))
    );
}

/// A header with no adapter-default marker on the level is the level the
/// seat ran at: 0.1.5-rc.1 reading the settings document, and 0.2.1-rc.2
/// reading the composition row, measured with the same shape.
#[test]
fn a_header_that_applied_the_pin_is_the_seat_effort_on_either_release() {
    let applied = serde_json::json!({
        "config": {"provider": "deepseek-official", "model": "deepseek-flash",
                   "maxTokens": 256_000, "reasoningEffort": "low"},
        "adapterDefaults": {"maxTokens": true}
    });
    let low = Value::String("low".into());
    assert_eq!(fold_header(applied, None), (low.clone(), Some(low)));
    let applied = serde_json::json!({"config": {"reasoningEffort": "high"}});
    let high = Value::String("high".into());
    assert_eq!(
        fold_header(applied, Some(EFFORT_NOT_APPLICABLE)),
        (high.clone(), Some(high))
    );
}

/// What the header reader cannot read it does not record: a header that
/// is not there, a marker that is not a boolean, a level that is not a
/// word.
#[test]
fn a_header_the_reader_cannot_read_records_no_level() {
    for event in [
        serde_json::json!({"type": "request/header"}),
        serde_json::json!({"type": "request/header", "data": {"header": {
            "config": {"reasoningEffort": "high"}, "adapterDefaults": {"reasoningEffort": "no"}}}}),
        serde_json::json!({"type": "request/header", "data": {"header": {
            "config": {"reasoningEffort": 3}}}}),
        serde_json::json!({"type": "request/header", "data": {"header": {
            "config": {"reasoningEffort": "think hard"}}}}),
    ] {
        assert_eq!(applied_level(&event), None, "{event}");
    }
}
