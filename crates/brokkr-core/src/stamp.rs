//! The one reading of a journal's time stamps (decision 0089 ruling 3):
//! the writer's summary row orders a run by it and brokkr-view dates and
//! sections the fleet by it, so the two read a stamp one way (decision
//! 0071 ruling 7). Moved here from brokkr-view's `js`, unchanged: it is
//! the console's `Date.parse`, ported.

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
    if !matches_shape(bytes.get(..19)?, b"####-##-##T##:##:##") {
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
