use crate::tests::envelope_builder;
use serde_json::{json, Value};

#[path = "../../../../tests/support/seat_journal.rs"]
pub(crate) mod seat_journal;

/// The divergence as `brokkr compare` prints it.
fn first_divergence(a: &[String], b: &[String]) -> Value {
    serde_json::to_value(super::first_divergence(a, b)).unwrap()
}

#[test]
fn divergence_handles_equal_prefix_and_mismatch_trails() {
    let a = vec!["one".to_string(), "two".to_string()];
    assert_eq!(first_divergence(&a, &a), Value::Null);
    assert_eq!(
        first_divergence(&a, &["one".to_string()]),
        json!({"index": 1, "a": "two", "b": "end"})
    );
    assert_eq!(
        first_divergence(&a, &["other".to_string(), "two".to_string()]),
        json!({"index": 0, "a": "one", "b": "other"})
    );
}
