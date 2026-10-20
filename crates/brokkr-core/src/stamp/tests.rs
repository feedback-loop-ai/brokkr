use super::*;

#[test]
fn parse_millis_reads_journal_stamps_and_refuses_everything_else() {
    assert_eq!(parse_millis("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(parse_millis("1970-01-01T00:00:01Z"), Some(1_000));
    // January exercises the civil-date month shift on the other side.
    assert_eq!(
        parse_millis("2026-01-02T00:00:00Z"),
        Some(1_767_312_000_000)
    );
    assert_eq!(
        parse_millis("2026-06-02T00:00:00Z"),
        Some(1_780_358_400_000)
    );
    // Milliseconds are kept; nanosecond digits multiply by a scale that
    // has already run down to zero.
    assert_eq!(parse_millis("1970-01-01T00:00:00.123Z"), Some(123));
    assert_eq!(parse_millis("1970-01-01T00:00:00.123456789Z"), Some(123));
    // Offsets, both signs.
    assert_eq!(parse_millis("1970-01-01T01:00:00+01:00"), Some(0));
    assert_eq!(parse_millis("1969-12-31T23:00:00-01:00"), Some(0));
    // Anything else is the JS NaN.
    assert_eq!(parse_millis("2026-01-02"), None, "too short");
    assert_eq!(parse_millis("2026x01-02T00:00:00Z"), None, "separator");
    assert_eq!(parse_millis("20a6-01-02T00:00:00Z"), None, "digit");
    assert_eq!(parse_millis("1970-01-01T00:00:00XYZ"), None, "offset");
    assert_eq!(
        parse_millis("1970-01-01T00:00:00.5"),
        None,
        "no zone at all"
    );
    assert_eq!(parse_millis(""), None);
}

/// The minutes of a time and of an offset, and February, each counted:
/// the cases the stamps above leave at zero or in January.
#[test]
fn parse_millis_counts_minutes_offsets_and_february() {
    assert_eq!(parse_millis("1970-01-01T00:01:00Z"), Some(60_000));
    assert_eq!(parse_millis("1970-01-01T01:30:00+01:30"), Some(0));
    assert_eq!(parse_millis("1969-12-31T22:30:00-01:30"), Some(0));
    assert_eq!(
        parse_millis("2026-02-15T00:00:00Z"),
        Some(1_771_113_600_000)
    );
    assert_eq!(parse_millis("1970-01-01T00:00:00"), None, "no zone");
}
