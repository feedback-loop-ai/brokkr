//! One provider's pre-session refusal, typed where the text is built
//! (decision 0103 ruling 1(a)). The classifiers read only their harness's
//! machine fields; the provider and route are filled later, never from
//! prose, and the bounded evidence line lives here alone.

use serde_json::Value;

use super::{row::Row, AdapterKind};
use crate::{PreSessionRefusal, RefusalClass};

mod codex_thread;

pub(super) use codex_thread::{
    codex_thread_refusal, codex_thread_refusal_since, find_codex_thread, thread_header,
};

/// One line of wire text, whitespace-collapsed and clamped: every field a
/// refusal quotes comes off the harness's stream and may be arbitrarily
/// long and newline-bearing. Control characters go too, so a terminal
/// escape never lands in a durable record.
pub(super) fn bounded_wire_line(text: &str, limit: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

/// The token's own bound: a machine token is a short identifier.
pub(super) const REFUSAL_TOKEN_LIMIT: usize = 80;

/// And the excerpt's, which may be a sentence.
const REFUSAL_TEXT_LIMIT: usize = 160;

/// Bound one refusal for the journal: a single clamped line, never empty.
/// The provider's machine token and prose are bounded here exactly as the
/// stderr tail is, because both are untrusted stream text.
pub(super) fn refusal_reason(
    token: &str,
    status: Option<i64>,
    text: Option<&str>,
) -> PreSessionRefusal {
    let class = Row::new(&Value::String(token.to_string()), &[])
        .token("", REFUSAL_CLASSES)
        .unwrap_or(RefusalClass::Other);
    refusal_reason_with(class, token, status, text, None)
}

/// [`refusal_reason`] carrying the provider's own reset as a retry-after
/// (decision 0103 ruling 3). The class comes only from the machine token;
/// the words are bounded exactly as before.
fn refusal_reason_with(
    class: RefusalClass,
    token: &str,
    status: Option<i64>,
    text: Option<&str>,
    retry_after_seconds: Option<u64>,
) -> PreSessionRefusal {
    let token = bounded_wire_line(token, REFUSAL_TOKEN_LIMIT);
    // Never empty: a refusal with no token at all is still a refusal,
    // and an empty one would leave the line dangling after its colon.
    let token = if token.is_empty() {
        "api_error".to_string()
    } else {
        token
    };
    let mut words = token;
    if let Some(status) = status {
        words.push_str(&format!(" (HTTP {status})"));
    }
    if let Some(text) = text {
        let excerpt = bounded_wire_line(text, REFUSAL_TEXT_LIMIT);
        if !excerpt.is_empty() {
            words.push_str(": ");
            words.push_str(&excerpt);
        }
    }
    PreSessionRefusal::from_parts(class, words, status, retry_after_seconds)
}

/// The claude (and, byte-for-byte, lanetally) stream-json record of a
/// provider refusal. Three measured shapes, all meaning the request was
/// rejected before any inference (decision 0053): the synthetic assistant
/// message with `isApiErrorMessage`/`error`; the error `result` with
/// `is_error`; and the `rate_limit_event` newer CLIs emit. A record with
/// none of those fields is an ordinary turn and is never read here.
pub(super) fn claude_refusal(event: &Row<'_>) -> Option<PreSessionRefusal> {
    claude_refusal_at(event, epoch_now())
}

const RATE_LIMIT: &str = "rate_limit";
const API_ERROR: &str = "api_error";
const REFUSAL_CLASSES: &[(&str, RefusalClass)] = &[(RATE_LIMIT, RefusalClass::RateLimit)];

#[derive(Clone, Copy)]
enum ClaudeRefusalRow {
    RateLimit,
    Assistant,
    Result,
}

/// [`claude_refusal`] over the caller's clock reading.
pub(super) fn claude_refusal_at(event: &Row<'_>, now: Option<i64>) -> Option<PreSessionRefusal> {
    use ClaudeRefusalRow::{Assistant, RateLimit, Result};
    let kind = event.kind(
        "/type",
        &[
            ("rate_limit_event", RateLimit),
            ("assistant", Assistant),
            ("result", Result),
        ],
    )?;
    let status = event.count("/apiErrorStatus");
    let error = event.text("/error");
    match kind {
        RateLimit => {
            let retry_after = event
                .count("/rate_limit_info/resetsAt")
                .filter(|at| *at >= 0)
                .and_then(|at| seconds_between(now?, at));
            let message = event.text("/message").or(error);
            Some(refusal_reason_with(
                RefusalClass::RateLimit,
                RATE_LIMIT,
                status,
                message.as_deref(),
                retry_after,
            ))
        }
        Assistant if event.flag("/isApiErrorMessage") == Some(true) || error.is_some() => {
            let text = assistant_excerpt(event);
            Some(claude_error(
                event,
                error.as_deref(),
                status,
                text.as_deref(),
            ))
        }
        Result if event.flag("/is_error") == Some(true) || error.is_some() => {
            let text = event.text("/result");
            Some(claude_error(
                event,
                error.as_deref(),
                status,
                text.as_deref(),
            ))
        }
        Assistant | Result => None,
    }
}

fn assistant_excerpt(event: &Row<'_>) -> Option<String> {
    match event.items("/message/content") {
        Some(mut blocks) => blocks.next()?.text("/text"),
        // JSON Pointer also admits a member named "0" on an object.
        None => event.text("/message/content/0/text"),
    }
}

/// The harness's own token takes precedence over a synthetic HTTP token,
/// including when its normalized spelling is unknown.
fn claude_error(
    event: &Row<'_>,
    error: Option<&str>,
    status: Option<i64>,
    text: Option<&str>,
) -> PreSessionRefusal {
    let (class, token) = match error {
        Some(token) => (
            event
                .token("/error", REFUSAL_CLASSES)
                .unwrap_or(RefusalClass::Other),
            token,
        ),
        None if status == Some(429) => (RefusalClass::RateLimit, RATE_LIMIT),
        None => (RefusalClass::Other, API_ERROR),
    };
    refusal_reason_with(class, token, status, text, None)
}

/// The codex `exec --json` record of a pre-session refusal: an `error` or
/// `turn.failed` before any `turn.started`. A usage limit is the
/// provider's rate limit (decision 0103 ruling 1(a)); every other error
/// keeps `api_error`, class `Other`. The reset may be stated here too.
pub(super) fn codex_refusal(event: &Row<'_>) -> Option<PreSessionRefusal> {
    if !event.is("/type", "error") && !event.is("/type", "turn.failed") {
        return None;
    }
    let error = event.with_fallback("/error");
    // A missing clock is no reset, never no refusal (decision 0103).
    let class = codex_class(&error);
    let now = (class == RefusalClass::RateLimit).then(Now::read).flatten();
    Some(codex_error(&error, class, now.as_ref()))
}

/// One Codex refusal from the machine fields and masked prose of an error.
fn codex_error(error: &Row<'_>, class: RefusalClass, now: Option<&Now>) -> PreSessionRefusal {
    let status = error.count("/status");
    let message = error.text("/message");
    let (token, retry_after) = match class {
        RefusalClass::Other => (API_ERROR, None),
        RefusalClass::RateLimit => (RATE_LIMIT, now.and_then(|now| codex_reset(error, now))),
    };
    refusal_reason_with(class, token, status, message.as_deref(), retry_after)
}

/// Whether codex's own machine fields mark this error as a rate limit.
fn codex_class(error: &Row<'_>) -> RefusalClass {
    if error.count("/status") == Some(429)
        || error.is("/codex_error_info", "usage_limit_exceeded")
        || error.is("/codex_error_info", "rate_limit_exceeded")
    {
        RefusalClass::RateLimit
    } else {
        RefusalClass::Other
    }
}

/// One `rate_limits` window: how full it is and its Unix-second reset.
/// Only the exhausted window's reset is a wait (decision 0103 ruling 3).
#[derive(Debug, Clone, Copy)]
struct ResetWindow {
    used_percent: Option<f64>,
    resets_at: i64,
}

/// The provider's stated reset, in whole seconds from the refusal.
fn codex_reset(error: &Row<'_>, now: &Now) -> Option<u64> {
    if !error.contains("/message", RESET_PREFIX) {
        return None;
    }
    let stated = stated_reset(&error.text("/message")?)?;
    seconds_between(now.epoch, stated.epoch()?)
}

/// The latest reset among the windows codex marks full: only an
/// exhausted window can be the one that refused.
fn exhausted_reset(now: &Now, resets: &[ResetWindow]) -> Option<u64> {
    resets
        .iter()
        .filter(|window| window.used_percent.is_some_and(|used| used >= 100.0))
        .filter_map(|window| seconds_between(now.epoch, window.resets_at))
        .max()
}

/// Every window a codex `token_count` row states; one without
/// `resets_at` is skipped rather than read as zero.
fn reset_windows(limits: &Row<'_>) -> Vec<ResetWindow> {
    ["/primary", "/secondary"]
        .iter()
        .filter_map(|key| {
            let window = limits.at(key)?;
            Some(ResetWindow {
                used_percent: window.number("/used_percent"),
                resets_at: window.count("/resets_at")?,
            })
        })
        .collect()
}

/// Whole seconds forward, only: a passed reset gives no wait.
fn seconds_between(now: i64, later: i64) -> Option<u64> {
    u64::try_from(later.checked_sub(now)?)
        .ok()
        .filter(|seconds| *seconds > 0)
}

/// The Unix second a reset's civil text is measured from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Now {
    pub(super) epoch: i64,
}

impl Now {
    /// The host's clock now, read only on a codex refusal with a reset.
    pub(super) fn read() -> Option<Now> {
        Some(Now {
            epoch: epoch_now()?,
        })
    }
}

/// The Unix second now, alone: a reset needs only the instant.
pub(super) fn epoch_now() -> Option<i64> {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    i64::try_from(epoch.as_secs()).ok()
}

/// Nanoseconds in one second, the unit every boundary below compares in.
const NANOS_PER_SECOND: i128 = 1_000_000_000;

/// The instant at nanosecond precision: `…:00.900100Z` is not
/// `…:00.900900Z`, and a coarser boundary reads the first as current.
pub(super) fn epoch_now_nanos() -> Option<i128> {
    system_time_nanos(std::time::SystemTime::now())
}

/// A filesystem or clock reading in the rows' nanosecond unit.
fn system_time_nanos(at: std::time::SystemTime) -> Option<i128> {
    let since = at.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(i128::from(since.as_secs()) * NANOS_PER_SECOND + i128::from(since.subsec_nanos()))
}

/// A record's timestamp at nanosecond precision, its fraction scaled here
/// rather than truncated to milliseconds.
pub(super) fn parse_nanos(text: &str) -> Option<i128> {
    let millis = brokkr_core::stamp::parse_millis(text)?;
    let seconds = i128::from(millis.div_euclid(1_000));
    let fraction = text
        .split_once('.')
        .map(|(_, tail)| tail)
        .unwrap_or("")
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("");
    let mut nanos = 0i128;
    let mut scale = 1i128;
    for byte in fraction.bytes().take(9) {
        nanos = nanos * 10 + i128::from(byte - b'0');
        scale *= 10;
    }
    Some(seconds * NANOS_PER_SECOND + nanos * (NANOS_PER_SECOND / scale))
}

/// A civil date and time without a zone: codex states its reset in the
/// host's own words ("Nov 11th, 2026 10:38 AM") and gives no zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LocalTime {
    pub(super) year: i32,
    pub(super) month: u8,
    pub(super) day: u8,
    pub(super) hour: u8,
    pub(super) minute: u8,
    pub(super) second: u8,
}

impl LocalTime {
    /// The real instant this civil time denotes in the host's own zone
    /// (`mktime` applies the daylight rules). The round trip refuses a
    /// reading the zone does not have rather than normalising a guess.
    pub(super) fn epoch(self) -> Option<i64> {
        let at = self.instant()?;
        (local_time(at)? == self).then_some(at)
    }

    fn instant(self) -> Option<i64> {
        let mut broken: libc::tm = unsafe { std::mem::zeroed() };
        broken.tm_year = self.year - 1900;
        broken.tm_mon = i32::from(self.month) - 1;
        broken.tm_mday = i32::from(self.day);
        broken.tm_hour = i32::from(self.hour);
        broken.tm_min = i32::from(self.minute);
        broken.tm_sec = i32::from(self.second);
        // Let the zone decide the daylight flag: the provider names a
        // civil time and no zone, and this is the host's reading of it.
        broken.tm_isdst = -1;
        // SAFETY: `mktime` reads and writes the one `tm` this stack frame
        // owns; no pointer outlives the call.
        let at = unsafe { libc::mktime(&mut broken) };
        // A date the zone folds into another (`Feb 30th`, a spring-forward
        // hour) is caught by the caller's round trip. `mktime`'s `-1` error
        // value is also the last second of 1969, which the round trip
        // either confirms or rejects on the same terms.
        Some(at)
    }
}

/// The host's local civil time at `epoch`, through its own zone rules.
pub(super) fn local_time(epoch: i64) -> Option<LocalTime> {
    let at = libc::time_t::try_from(epoch).ok()?;
    // SAFETY: `localtime_r` reads `at` and writes one `tm` through the
    // pointer; both live for the length of the call.
    let mut broken: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&at, &mut broken) }.is_null() {
        return None;
    }
    // A successful `localtime_r` fills the fields inside their C ranges,
    // so none of these is a fallible conversion: the `tm` contract is
    // what the null check above has already established.
    Some(LocalTime {
        year: broken.tm_year + 1900,
        month: (broken.tm_mon + 1) as u8,
        day: broken.tm_mday as u8,
        hour: broken.tm_hour as u8,
        minute: broken.tm_min as u8,
        second: broken.tm_sec as u8,
    })
}

const RESET_PREFIX: &str = "try again at ";

/// Codex's own words for its reset, parsed strictly: `try again at Nov
/// 11th, 2026 10:38 AM.` Anything else gives no reset rather than a guess.
fn stated_reset(message: &str) -> Option<LocalTime> {
    let tail = message.split_once(RESET_PREFIX)?.1;
    let mut words = tail.split_whitespace();
    let month = month_of(words.next()?)?;
    let day = day_of(words.next()?.strip_suffix(',')?)?;
    let year = four_digits(words.next()?)?;
    let (hour, minute) = clock_of(words.next()?)?;
    let hour = meridiem(words.next()?, hour)?;
    if words.next().is_some() {
        return None;
    }
    Some(LocalTime {
        year,
        month,
        day,
        hour,
        minute,
        second: 0,
    })
}

fn month_of(word: &str) -> Option<u8> {
    Some(match word {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    })
}

/// A day with its ordinal suffix, the provider's own spelling (`11th`).
fn day_of(word: &str) -> Option<u8> {
    let digits = ["st", "nd", "rd", "th"]
        .iter()
        .find_map(|suffix| word.strip_suffix(suffix))?;
    let day = digits.parse::<u8>().ok()?;
    (1..=31).contains(&day).then_some(day)
}

fn four_digits(word: &str) -> Option<i32> {
    if word.len() != 4 || !word.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    word.parse().ok()
}

/// A twelve-hour clock, `H:MM`.
fn clock_of(word: &str) -> Option<(u8, u8)> {
    let (hour, minute) = word.split_once(':')?;
    if hour.is_empty() || hour.len() > 2 || minute.len() != 2 {
        return None;
    }
    let hour = hour.parse::<u8>().ok()?;
    let minute = minute.parse::<u8>().ok()?;
    if !(1..=12).contains(&hour) || minute > 59 {
        return None;
    }
    Some((hour, minute))
}

fn meridiem(word: &str, hour: u8) -> Option<u8> {
    match word.strip_suffix('.').unwrap_or(word) {
        "AM" => Some(if hour == 12 { 0 } else { hour }),
        "PM" => Some(if hour == 12 { 12 } else { hour + 12 }),
        _ => None,
    }
}

/// The dsh session record's end-of-turn error: a `QUOTA` code, 402 or 429
/// is the provider's rate limit (decision 0103 ruling 1(a)); every other
/// error before the first turn is `Other`. Only machine fields are read.
pub(super) fn dsh_refusal(event: &Row<'_>) -> Option<PreSessionRefusal> {
    if !event.is("/data/reason/kind", "error") {
        return None;
    }
    const CODE: &str = "/data/reason/error/code";
    let status = event.count("/data/reason/error/status");
    let (class, token) = if matches!(status, Some(402 | 429)) || event.is(CODE, "QUOTA") {
        (RefusalClass::RateLimit, RATE_LIMIT.to_string())
    } else {
        let class = event
            .token(CODE, REFUSAL_CLASSES)
            .unwrap_or(RefusalClass::Other);
        (
            class,
            event.text(CODE).unwrap_or_else(|| API_ERROR.to_string()),
        )
    };
    let message = event.text("/data/reason/error/message");
    Some(refusal_reason_with(
        class,
        &token,
        status,
        message.as_deref(),
        None,
    ))
}

impl AdapterKind {
    /// The provider name the adapter's declaration uses, which is the one
    /// a typed refusal carries (decision 0103 ruling 1(a)). It is filled
    /// from the adapter, never read off a harness's prose.
    pub(crate) fn provider_name(self) -> &'static str {
        match self {
            AdapterKind::Claude => "claude",
            AdapterKind::Lanetally => "lanetally",
            AdapterKind::Codex => "codex",
            AdapterKind::Dsh => "dsh",
            AdapterKind::Qoder => "qoder",
            AdapterKind::Agy => "agy",
            AdapterKind::Exec => "exec",
        }
    }

    /// Whether this adapter advertises the one-turn probe (decision 0103
    /// ruling 4(a)). It is the `capabilities.probe` field, never a
    /// `supports` entry: the frozen v1 schema closes that list to
    /// `resume`, `checkpoint` and `cancel`. Exec has no model turn, so it
    /// advertises nothing; Qoder's and Agy's probes are outside level 1
    /// (#1178, #1200).
    pub(super) fn advertises_probe(self) -> bool {
        match self {
            AdapterKind::Claude
            | AdapterKind::Lanetally
            | AdapterKind::Codex
            | AdapterKind::Dsh => true,
            AdapterKind::Qoder | AdapterKind::Agy | AdapterKind::Exec => false,
        }
    }
}
