use super::*;

fn moved(start: Option<&str>, end: Option<&str>) -> MovedHead {
    MovedHead {
        head_at_end: end.map(str::to_string),
        head_at_start: start.map(str::to_string),
    }
}

/// The encoded bytes are the ones the engine wrote before the codec:
/// `format!("GATE-MOVED-HEAD {}", json!({"head_at_start": .., "head_at_end": ..}))`,
/// whose sorted map puts `head_at_end` first.
#[test]
fn encode_keeps_the_bytes_the_journal_already_carries() {
    assert_eq!(
        encode(&moved(Some("aaa"), Some("bbb"))),
        r#"GATE-MOVED-HEAD {"head_at_end":"bbb","head_at_start":"aaa"}"#
    );
    assert_eq!(
        encode(&moved(None, Some("bbb"))),
        r#"GATE-MOVED-HEAD {"head_at_end":"bbb","head_at_start":null}"#
    );
}

#[test]
fn decode_reads_back_what_encode_wrote() {
    for value in [moved(Some("aaa"), Some("bbb")), moved(None, Some("bbb"))] {
        assert_eq!(
            decode::<MovedHead>(&encode(&value)).unwrap().unwrap(),
            value
        );
    }
}

/// An unmarked reason is `None`; the bare marker and a marker glued to
/// its evidence are unmarked too, as the strip of "GATE-MOVED-HEAD "
/// they replace read them. A marked reason with unreadable evidence is
/// marked, and names why its evidence does not read.
#[test]
fn decode_separates_the_marker_from_its_evidence() {
    for unmarked in [
        "effect e1 indeterminate: timed out",
        "GATE-MOVED-HEAD",
        r#"GATE-MOVED-HEAD{"head_at_end":"b","head_at_start":"a"}"#,
        r#" GATE-MOVED-HEAD {"head_at_end":"b","head_at_start":"a"}"#,
    ] {
        assert!(decode::<MovedHead>(unmarked).is_none(), "{unmarked}");
    }
    let unreadable = |reason: &str| {
        decode::<MovedHead>(reason)
            .unwrap()
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        unreadable("GATE-MOVED-HEAD not json"),
        "expected ident at line 1 column 2"
    );
    assert_eq!(
        unreadable(r#"GATE-MOVED-HEAD {"head_at_end":"b","head_at_start":"a","x":1}"#),
        "unknown field `x`, expected `head_at_end` or `head_at_start` at line 1 column 42"
    );
}
