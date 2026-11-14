use super::*;
use serde_json::json;

/// The whole tail's wire shape, as `inspect` states it.
fn whole_tail() -> serde_json::Value {
    json!({
        "repaired_at": 8,
        "terminal_seq": 5,
        "operator": "repairer",
        "events": [
            {"seq": 6, "type": "operator/commanded"},
            {"seq": 7, "type": "operator/accepted"},
        ],
    })
}

/// L5: the whole tail types its event word as core's `EventType` and
/// closes its contract, so a row no writer writes is refused, not painted.
#[test]
fn a_late_tails_wire_shape_refuses_an_unknown_type_and_an_unknown_field() {
    let tail: LateTail = serde_json::from_value(whole_tail()).unwrap();
    assert_eq!(
        tail.events
            .iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>(),
        [
            crate::envelope::EventType::OperatorCommanded,
            crate::envelope::EventType::OperatorAccepted
        ]
    );

    let mut unknown_type = whole_tail();
    unknown_type["events"][0]["type"] = json!("operator/imagined");
    let refused = serde_json::from_value::<LateTail>(unknown_type).unwrap_err();
    assert!(
        refused
            .to_string()
            .contains("unknown variant `operator/imagined`"),
        "{refused}"
    );

    let mut unknown_field = whole_tail();
    unknown_field["events"][0]["note"] = json!("no writer writes this");
    let refused = serde_json::from_value::<LateTail>(unknown_field).unwrap_err();
    assert!(
        refused.to_string().contains("unknown field `note`"),
        "{refused}"
    );
}

/// L6: the row keeps only the sentence's facts, and its operator name is
/// cut to the journal's own bound.
#[test]
fn a_late_range_states_the_count_the_range_and_a_cut_operator() {
    let operator = "ö".repeat(crate::summary::ACK_OPERATOR_CHARS + 3);
    let tail = crate::fold::LateTail {
        repaired_at: 9,
        terminal_seq: 5,
        first_seq: 6,
        last_seq: 8,
        event_types: vec![
            crate::envelope::EventType::OperatorCommanded,
            crate::envelope::EventType::OperatorRejected,
            crate::envelope::EventType::RunStarted,
        ],
        operator: operator.clone(),
    };
    let range = LateRange::of(&tail);
    assert_eq!((range.first_seq, range.last_seq, range.count), (6, 8, 3));
    assert_eq!(range.repaired_at, 9);
    assert_eq!(
        range.operator.chars().count(),
        crate::summary::ACK_OPERATOR_CHARS
    );
    assert!(operator.starts_with(&range.operator));
    // L10: the whole tail a row or `inspect` reads cuts the same name, so
    // `runs` and `inspect` never name two repairers.
    assert_eq!(LateTail::of(&tail).operator, range.operator);

    let written = serde_json::to_value(&range).unwrap();
    assert_eq!(
        written,
        json!({
            "repaired_at": 9, "first_seq": 6,
            "last_seq": 8, "count": 3, "operator": range.operator,
        })
    );
    let mut unknown = written;
    unknown["events"] = json!([]);
    let refused = serde_json::from_value::<LateRange>(unknown).unwrap_err();
    assert!(
        refused.to_string().contains("unknown field `events`"),
        "{refused}"
    );
}

#[test]
fn every_lane_is_the_word_its_column_holds() {
    let lanes = [
        Lane::Behind,
        Lane::Refused,
        Lane::Acknowledged,
        Lane::Parked,
        Lane::Running,
        Lane::Finished,
    ];
    let words: Vec<&str> = lanes.into_iter().map(Lane::as_str).collect();
    assert_eq!(
        words,
        [
            "behind",
            "refused",
            "acknowledged",
            "parked",
            "running",
            "finished"
        ]
    );
}

#[test]
fn a_section_lists_its_newest_first_and_an_unreadable_time_before_all() {
    let mut places = vec![
        (5, "b"),
        (i64::MAX, "a"),
        (9, "a"),
        (5, "c"),
        (i64::MIN, "z"),
    ];
    places.sort_by_key(|place| newest_first(*place));
    assert_eq!(
        places,
        [
            (i64::MAX, "a"),
            (9, "a"),
            (5, "c"),
            (5, "b"),
            (i64::MIN, "z")
        ]
    );
    assert_eq!(newest_first((5, "b")), Reverse((5, "b")));
}
