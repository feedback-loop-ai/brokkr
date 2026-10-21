//! The runs route's query values, form-decoded (decision 0089 ruling 6),
//! beside `tests.rs`'s route test of the limit, the cursor and the
//! refusals.

use super::*;
use serde_json::json;

/// A `next` whose run id holds a space or a plus reads back as
/// `URLSearchParams` encodes it: `+` for the space and `%2B` for the plus.
#[test]
fn a_form_encoded_cursor_keeps_a_space_and_a_plus() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    // Two spaced ids, so one stands in a `next` wherever the clock lists it.
    for run_id in ["r1", "r 2", "r+3", "r 4"] {
        crate::tests::running_store(&db, run_id);
    }
    let read = |path: &str| -> Value { serde_json::from_str(&handle(&db, path).body).unwrap() };
    let form = |next: &str| {
        next.replace('+', "%2B")
            .replace(' ', "+")
            .replace(':', "%3A")
    };
    let mut page = read("/api/runs?limit=1");
    let mut listed = vec![page["runs"][0]["run_id"].clone()];
    // Bounded, so a cursor read back as another id cannot page forever.
    for _ in 0..4 {
        let Some(next) = page["next"].as_str().map(form) else {
            break;
        };
        page = read(&format!("/api/runs?limit=1&after={next}"));
        listed.push(page["runs"][0]["run_id"].clone());
    }
    // Every run once to the end; their order is the journal's clock.
    listed.sort_by_key(Value::to_string);
    assert_eq!(
        listed,
        [json!("r 2"), json!("r 4"), json!("r+3"), json!("r1")]
    );
}
