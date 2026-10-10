use super::tests::event;
use super::*;

fn marked(start: Option<&str>, end: Option<&str>) -> String {
    gate_moved_head::encode(&MovedHead {
        head_at_end: end.map(str::to_string),
        head_at_start: start.map(str::to_string),
    })
}

fn evidence(reasons: &[&str]) -> String {
    let events: Vec<_> = reasons
        .iter()
        .map(|reason| {
            event(
                EventType::EffectIndeterminate,
                json!({"effect_id": "e1", "attempt_id": "a1", "reason": reason}),
            )
        })
        .collect();
    gate_head_evidence(&events).to_string()
}

/// The evidence a parked run carries is the newest marked indeterminate's
/// that reads as JSON, in the bytes the reason packed: both heads, a `null`
/// head that could not be read, or whatever JSON the marker carries, as the
/// pre-codec reader echoed it. Evidence that does not read is passed over,
/// so a run with none left parks with `{}`.
#[test]
fn a_parked_run_carries_the_newest_marked_evidence_byte_for_byte() {
    let both = marked(Some("aaa"), Some("bbb"));
    let start_unread = marked(None, Some("bbb"));
    let older = marked(Some("old"), Some("aaa"));
    let extra_key = r#"GATE-MOVED-HEAD {"head_at_end":"b","head_at_start":"a","x":1}"#;
    for (reasons, bytes) in [
        (
            vec![both.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":"aaa"}"#,
        ),
        (
            vec![start_unread.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":null}"#,
        ),
        (
            vec![older.as_str(), both.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":"aaa"}"#,
        ),
        (
            vec![older.as_str(), extra_key],
            r#"{"head_at_end":"b","head_at_start":"a","x":1}"#,
        ),
        (vec!["GATE-MOVED-HEAD 42"], "42"),
        (
            vec![older.as_str(), "GATE-MOVED-HEAD not json"],
            r#"{"head_at_end":"aaa","head_at_start":"old"}"#,
        ),
        (vec!["GATE-MOVED-HEAD not json"], "{}"),
        (vec!["effect e1 indeterminate: timed out"], "{}"),
    ] {
        assert_eq!(evidence(&reasons), bytes, "{reasons:?}");
    }
}
