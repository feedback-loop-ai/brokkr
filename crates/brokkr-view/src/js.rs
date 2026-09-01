//! The JavaScript primitives the console's derivation was written
//! against. Each one closes a divergence that would otherwise produce a
//! *wrong displayed value* with no crash and no failing test: `toFixed`
//! ties away from zero where Rust ties to even, `String.length` counts
//! UTF-16 code units where Rust counts bytes or `char`s, `Math.round`
//! is half-up where Rust's `round` is half-away-from-zero on negatives,
//! and JS string concatenation of an absent field yields the literal
//! `undefined`.
//!
//! These are ports, not improvements. Where a rule looks wrong it is
//! still the rule the page has shipped, and changing it here would make
//! the new Rust surface disagree with the old console — the exact drift
//! this crate exists to end.

use serde_json::Value;

/// `Number.prototype.toFixed(4)`.
///
/// ECMA-262 picks the integer `n` minimising `|n / 10^4 - x|` over the
/// *exact* value of `x`, resolving a tie to the larger `n` — away from
/// zero, since the sign is stripped first. Rust's `format!("{:.4}")`
/// ties to even: `0.03125` prints `0.0312` there and `0.0313` on the
/// console. Money is not a place to disagree with the shipped surface.
pub fn to_fixed_4(value: f64) -> String {
    let negative = value < 0.0;
    let magnitude = if negative { -value } else { value };
    // An f64's decimal expansion terminates within 1074 fractional
    // digits, so this string IS the real value rather than an
    // approximation of it — which makes the tie test below exact.
    let exact = format!("{magnitude:.1074}");
    let point = exact.len() - 1075;
    let bytes = exact.as_bytes();
    let mut digits: Vec<u8> = Vec::with_capacity(point + 4);
    digits.extend_from_slice(&bytes[..point]);
    digits.extend_from_slice(&bytes[point + 1..point + 5]);
    // The tail is exact, so "first dropped digit >= 5" covers both
    // "above the halfway point" and "exactly on it".
    if bytes[point + 5] >= b'5' {
        let mut index = digits.len();
        loop {
            if index == 0 {
                digits.insert(0, b'1');
                break;
            }
            index -= 1;
            if digits[index] == b'9' {
                digits[index] = b'0';
            } else {
                digits[index] += 1;
                break;
            }
        }
    }
    let split = digits.len() - 4;
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    for (index, digit) in digits.iter().enumerate() {
        if index == split {
            out.push('.');
        }
        out.push(char::from(*digit));
    }
    out
}

/// `String.prototype.length` — UTF-16 code units, not bytes and not
/// `char`s. An emoji counts as two.
pub fn len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// `String.prototype.slice(start, end)` on UTF-16 code units.
///
/// A boundary falling inside a surrogate pair yields a lone surrogate
/// in JavaScript, which is not a Rust `char`. The character is dropped:
/// inventing U+FFFD in its place would be repair (decision 0001).
pub fn slice(text: &str, start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut unit = 0usize;
    for character in text.chars() {
        let width = character.len_utf16();
        if unit >= start && unit + width <= end {
            out.push(character);
        }
        unit += width;
        if unit >= end {
            break;
        }
    }
    out
}

/// `Math.round` — half-up, so 59500ms of duration is `1m00s` and not
/// `59s`. Rust's `f64::round` is half-away-from-zero, which differs
/// below zero; durations never reach here negative, but the rule is
/// the ported one either way.
pub fn round_half_up(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// JS string conversion, as `'a' + value` performs it — including the
/// literal `undefined` for an absent field. The console prints
/// `turn undefined · tool undefined` for a checkpoint that carries
/// neither; the quirk is ported deliberately.
pub fn to_display(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => {
            // `serde_json` renders an integral float as `1.0`; JS
            // renders `1`, and this is display, not the payload dump.
            let text = number.to_string();
            text.strip_suffix(".0").unwrap_or(&text).to_string()
        }
        Some(Value::Array(items)) => {
            let parts: Vec<String> = items.iter().map(element_display).collect();
            parts.join(",")
        }
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// `Array.prototype.join` renders `null` and `undefined` as the empty
/// string, unlike bare concatenation.
fn element_display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        other => to_display(Some(other)),
    }
}

/// The shape check every fixed-width timestamp field goes through:
/// `#` means "one ASCII digit", any other byte must match literally.
fn matches_shape(bytes: &[u8], shape: &[u8]) -> bool {
    if bytes.len() != shape.len() {
        return false;
    }
    for (index, expected) in shape.iter().enumerate() {
        let ok = if *expected == b'#' {
            bytes[index].is_ascii_digit()
        } else {
            bytes[index] == *expected
        };
        if !ok {
            return false;
        }
    }
    true
}

/// Decimal digits of an already shape-checked slice.
fn number(bytes: &[u8]) -> i64 {
    let mut value = 0i64;
    for byte in bytes {
        value = value * 10 + i64::from(byte - b'0');
    }
    value
}

/// Days from the Unix epoch to a proleptic-Gregorian date (Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let shifted = year - i64::from(month <= 2);
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let month_shift = if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * (month + month_shift) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// `Date.parse` over the RFC 3339 stamps a journal carries. Anything
/// else is `None` — the JS `NaN`, which every caller already treats as
/// "no duration". A half-read stamp is not a time.
pub fn parse_millis(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    if !matches_shape(&bytes[..19], b"####-##-##T##:##:##") {
        return None;
    }
    let year = number(&bytes[..4]);
    let month = number(&bytes[5..7]);
    let day = number(&bytes[8..10]);
    let hour = number(&bytes[11..13]);
    let minute = number(&bytes[14..16]);
    let second = number(&bytes[17..19]);
    let mut index = 19;
    let mut millis = 0i64;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        // Scale runs 100, 10, 1, 0 — digits past the millisecond are
        // multiplied by zero rather than branched away.
        let mut scale = 100i64;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            millis += i64::from(bytes[index] - b'0') * scale;
            scale /= 10;
            index += 1;
        }
    }
    let rest = &bytes[index..];
    let offset_minutes = if rest == b"Z" {
        0
    } else if matches_shape(rest, b"+##:##") {
        -(number(&rest[1..3]) * 60 + number(&rest[4..6]))
    } else if matches_shape(rest, b"-##:##") {
        number(&rest[1..3]) * 60 + number(&rest[4..6])
    } else {
        return None;
    };
    let seconds = days_from_civil(year, month, day) * 86_400
        + hour * 3_600
        + minute * 60
        + second
        + offset_minutes * 60;
    Some(seconds * 1_000 + millis)
}

#[cfg(test)]
mod tests;
