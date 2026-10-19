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

#[cfg(test)]
mod tests;
