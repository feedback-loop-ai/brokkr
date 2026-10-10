//! The browser paints the terminal's scope (#351): what `/api/view/<id>`
//! serves for a seat is what `brokkr inspect --seat` prints for it, and
//! the retired `/api/run/<id>` answers as any unknown path does.

use super::*;
use crate::render::{self, Style};
use brokkr_view::{lens_for, Scope};

/// The page's own fixture, entered at intake, then intake concluded and
/// design entered with its seat at work.
fn two_phase_fixture() -> (tempfile::TempDir, PathBuf) {
    let (dir, db) = fixture();
    let mut store = Store::open(&db).unwrap();
    for (event_type, payload) in [
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff-i", "seat": "intake", "phase": "intake"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff-i", "attempt_id": "a1"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "eff-i", "attempt_id": "a1", "result": {"result": "intook"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"rule_id": "INTAKE-OK", "from": "intake", "next": "design", "result": "intook"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "design"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff-d", "seat": "design", "phase": "design"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff-d", "attempt_id": "a2"}),
        ),
    ] {
        store
            .append_next("r1", event_type, payload, None, None)
            .unwrap();
    }
    (dir, db)
}

/// The lines of `printed`'s block under `heading`, up to the blank line
/// that ends it, whose first column holds a value: a row, never its
/// indented continuation.
fn block_rows<'a>(printed: &'a str, heading: &str) -> Vec<&'a str> {
    printed
        .lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .map(|line| line.split_whitespace().next().unwrap())
        .collect()
}

#[test]
fn the_served_seat_scope_is_what_inspect_seat_prints() {
    let (_dir, db) = two_phase_fixture();
    let served: Value = serde_json::from_str(&handle(&db, "/api/view/r1").body).unwrap();
    let scope = &served["scopes"]["seats"]["eff-d"];
    assert_eq!(
        scope,
        &json!({"phases": ["design"], "participants": ["eff-d"], "rows": [7, 8, 9]})
    );

    // `inspect --seat eff-d`, from the journal as the verb reads it.
    let store = Store::open_read_only(&db).unwrap();
    let events = store.load("r1").unwrap();
    let view = brokkr_view::run_view(&events, Some(&fold(&events).unwrap()));
    let lens = lens_for(&view, Some(&Scope::Seat("eff-d".into()))).unwrap();
    let printed = render::inspect(&view, lens.as_ref(), true, &Style::plain(100));

    let label = |key: &Value| {
        let part = served["participants"].as_array().unwrap().iter();
        part.clone().find(|part| part["key"] == *key).unwrap()["label"].clone()
    };
    let seats: Vec<Value> = scope["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(label)
        .collect();
    // The claim line and the header precede the seat rows.
    assert_eq!(block_rows(&printed, "seats")[2..], [json!("design")]);
    assert_eq!(seats, [json!("design")]);
    let in_trail = |row: &&Value| {
        served["journal"]
            .as_array()
            .unwrap()
            .iter()
            .any(|kept| kept["seq"] == **row && kept["in_trail"] == json!(true))
    };
    let trail: Vec<String> = scope["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(in_trail)
        .map(ToString::to_string)
        .collect();
    assert_eq!(block_rows(&printed, "trail"), trail);
    assert_eq!(
        block_rows(&printed, "graph"),
        scope["phases"].as_array().unwrap()[..]
    );
}

/// #351 retired `/api/run/<id>`: no page read it, and it served seat
/// results the view keeps from what it renders.
#[test]
fn the_retired_run_route_answers_not_found() {
    let (_dir, db) = two_phase_fixture();
    let retired = handle(&db, "/api/run/r1");
    assert_eq!(retired.status, "404 Not Found");
    assert_eq!(retired.body, r#"{"error":"/api/run/r1 not found"}"#);
}
