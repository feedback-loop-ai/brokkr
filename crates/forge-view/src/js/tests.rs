use super::*;
use serde_json::json;

#[test]
fn to_fixed_4_ties_away_from_zero_like_the_console() {
    // 0.03125 is exactly representable and an entirely plausible seat
    // cost. `format!("{:.4}")` ties to even and prints 0.0312; the
    // console prints 0.0313, and this crate is the console's answer.
    assert_eq!(to_fixed_4(0.03125), "0.0313");
    assert_eq!(format!("{:.4}", 0.03125_f64), "0.0312");
    assert_eq!(to_fixed_4(-0.03125), "-0.0313");
    // 0.00005 is stored as 0.0000500000000000000023…, above the halfway
    // point rather than on it.
    assert_eq!(to_fixed_4(0.00005), "0.0001");
    assert_eq!(to_fixed_4(0.0), "0.0000");
    // A carry that runs off the front of the number.
    assert_eq!(to_fixed_4(9.99999), "10.0000");
    // A carry that walks back through a run of nines.
    assert_eq!(to_fixed_4(0.09999999), "0.1000");
    // A Σ sum whose f64 tail must not leak into the rendered cost.
    assert_eq!(to_fixed_4(0.1 + 0.2), "0.3000");
    assert_eq!(to_fixed_4(123.456789), "123.4568");
}

#[test]
fn len_and_slice_count_utf16_code_units() {
    assert_eq!(len("abc"), 3);
    assert_eq!(len("é"), 1);
    assert_eq!(len("😀"), 2, "an emoji is a surrogate pair");
    assert_eq!(slice("abcd", 1, 3), "bc");
    // The window ends inside the pair: JavaScript would emit a lone
    // trailing surrogate, which is not a Rust char. Dropping it is the
    // honest answer; U+FFFD would be repair.
    assert_eq!(slice("a😀b", 0, 2), "a");
    // The window starts inside the pair: the lone leading surrogate is
    // dropped the same way.
    assert_eq!(slice("😀b", 1, 3), "b");
    assert_eq!(slice("aé", 0, 10), "aé");
}

#[test]
fn round_half_up_is_js_math_round() {
    assert_eq!(round_half_up(59.5), 60);
    assert_eq!(round_half_up(59.4), 59);
    assert_eq!(round_half_up(0.0), 0);
}

#[test]
fn to_display_reproduces_js_string_conversion() {
    // The console concatenates checkpoint fields straight into a live
    // line, so an absent field prints the literal `undefined`.
    assert_eq!(to_display(None), "undefined");
    assert_eq!(to_display(Some(&Value::Null)), "null");
    assert_eq!(to_display(Some(&json!(true))), "true");
    assert_eq!(to_display(Some(&json!("Read"))), "Read");
    assert_eq!(to_display(Some(&json!(3))), "3");
    // `serde_json` renders an integral float as `1.0`; JS renders `1`.
    assert_eq!(to_display(Some(&json!(1.0))), "1");
    assert_eq!(to_display(Some(&json!(1.5))), "1.5");
    assert_eq!(to_display(Some(&json!([1, null, "a"]))), "1,,a");
    assert_eq!(to_display(Some(&json!({"a": 1}))), "[object Object]");
}

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
    assert_eq!(parse_millis("1970-01-01T00:00:00.5"), None, "no zone at all");
    assert_eq!(parse_millis(""), None);
}
