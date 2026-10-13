//! The words the readout's figures are rendered in, each written once.

use super::{Range, HOUR_MILLIS};
use crate::js;

/// `part` as a whole percentage of `whole`, half up; `None` of nothing.
pub(super) fn percent(part: i64, whole: i64) -> Option<i64> {
    (whole > 0).then(|| js::round_half_up(part as f64 * 100.0 / whole as f64))
}

/// A duration as the board writes one: `42m`, or `4h 12m`.
pub(super) fn span_text(millis: i64) -> String {
    let minutes = millis / 60_000;
    match minutes < 60 {
        true => format!("{minutes}m"),
        false => format!("{}h {:02}m", minutes / 60, minutes % 60),
    }
}

/// Hours to one decimal: `19.4h`.
pub(super) fn hours_text(millis: i64) -> String {
    format!("{:.1}h", millis as f64 / HOUR_MILLIS as f64)
}

/// The shipped count's change against the range before it.
pub(super) fn change_text(change: i64, range: Range) -> String {
    let previous = format!("vs previous {}", range.word());
    if change > 0 {
        format!("▲ {change} {previous}")
    } else if change < 0 {
        format!("▼ {} {previous}", -change)
    } else {
        format!("no change {previous}")
    }
}

/// Names as a sentence lists them: `a`, `a and b`, `a, b and c`.
pub(super) fn listed(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The proleptic-Gregorian date `day` days from the Unix epoch
/// (Hinnant's `civil_from_days`).
fn civil(day: i64) -> (i64, i64, i64) {
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// The UTC date `day` days from the Unix epoch: `2026-10-13`.
pub(super) fn date(day: i64) -> String {
    let (year, month, day) = civil(day);
    format!("{year:04}-{month:02}-{day:02}")
}

/// An RFC 3339 stamp, to the second, for a time `parse_millis` read.
pub(super) fn stamp(millis: i64) -> String {
    let seconds = millis.div_euclid(1_000);
    let of_day = seconds.rem_euclid(86_400);
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        date(seconds.div_euclid(86_400)),
        of_day / 3_600,
        of_day % 3_600 / 60,
        of_day % 60
    )
}
