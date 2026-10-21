use super::*;

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
