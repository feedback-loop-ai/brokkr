//! One provider's pre-session refusal, typed where the text is built
//! (decision 0103 ruling 1(a)). The classifiers read only their harness's
//! machine fields; the provider and route are filled later, from the
//! adapter and the launch facts the driver already resolved, never from
//! prose. The bounded evidence line is this module's one home, so
//! `PreSessionRefusal`'s `Display` keeps the operator's text byte for
//! byte.

use serde_json::Value;

use super::AdapterKind;
use crate::{PreSessionRefusal, RefusalClass};

/// One line of wire text, whitespace-collapsed and clamped: every field
/// a refusal reason quotes comes off the harness's stream, so it is
/// arbitrary-length and may carry newlines. The journal is append-only
/// and every readout renders what lands in it, so a reason is bounded
/// here rather than trusted — the same discipline as the ≤80-char tool
/// names and targets and the 4000-byte stderr tail. Control characters
/// go with the newlines: a terminal escape belongs in a record no more
/// than a line break does.
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

/// The token's own bound. A machine token is a short identifier
/// (`rate_limit`, `authentication_error`); the checkpoint bound for a
/// harness-supplied name governs it too.
pub(super) const REFUSAL_TOKEN_LIMIT: usize = 80;

/// And the excerpt's, which may be a sentence.
const REFUSAL_TEXT_LIMIT: usize = 160;

/// Bound one refusal for the journal: a single line, clamped, and never
/// empty. The evidence is the provider's own machine token when it gave
/// one, the HTTP status when it gave that, and a short excerpt of its
/// prose when it gave that — the same evidence a stderr tail already
/// carries on a failed attempt, and nothing executable is derived from it.
///
/// The token is bounded exactly like the excerpt. It is read from the
/// harness's stream, so "the provider's own token" is only ever a claim
/// about where the bytes came from: a harness emitting a 200KB
/// newline-bearing `error` must not put that in a durable record.
pub(super) fn refusal_reason(
    token: &str,
    status: Option<i64>,
    text: Option<&str>,
) -> PreSessionRefusal {
    refusal_reason_with(token, status, text, None)
}

/// [`refusal_reason`] carrying the provider's own reset as a retry-after
/// (decision 0103 ruling 3). The class comes only from the machine token;
/// the words are bounded exactly as before.
pub(super) fn refusal_reason_with(
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
    let class = if token == "rate_limit" {
        RefusalClass::RateLimit
    } else {
        RefusalClass::Other
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
/// rejected before any inference (decision 0053):
///
/// - the synthetic assistant message #68816 records, carrying
///   `isApiErrorMessage: true` and an `error` token (`rate_limit`,
///   `authentication_error`, …) beside an ordinary terminal
///   `stop_reason: "stop_sequence"` — the shape the #219 run measured,
///   and the reason a `type` test alone cannot see it;
/// - the error `result` #79500 records, `is_error: true` with the prose
///   in `result` and no turn behind it;
/// - the `rate_limit_event` lifecycle message newer CLIs emit.
///
/// A record with none of those fields is an ordinary turn and is never
/// read here; nothing in this classifier looks at a model's prose.
pub(super) fn claude_refusal(event: &Value) -> Option<PreSessionRefusal> {
    match event.get("type").and_then(Value::as_str) {
        Some("rate_limit_event") => Some(refusal_reason(
            "rate_limit",
            event.get("apiErrorStatus").and_then(Value::as_i64),
            event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.get("error").and_then(Value::as_str)),
        )),
        Some("assistant") => {
            let api_error = event
                .get("isApiErrorMessage")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            api_error.then(|| {
                let status = event.get("apiErrorStatus").and_then(Value::as_i64);
                refusal_reason(
                    claude_token(event, status),
                    status,
                    event
                        .pointer("/message/content/0/text")
                        .and_then(Value::as_str),
                )
            })
        }
        Some("result") => {
            let is_error = event
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            is_error.then(|| {
                let status = event.get("apiErrorStatus").and_then(Value::as_i64);
                refusal_reason(
                    claude_token(event, status),
                    status,
                    event.get("result").and_then(Value::as_str),
                )
            })
        }
        _ => None,
    }
}

/// The token a claude refusal speaks. The harness's own `error` token wins
/// whenever it names one; a record whose only rate-limit mark is the HTTP
/// status sets the class's own token, which is the one measured shape
/// missing before decision 0103 ruling 1(a).
fn claude_token(event: &Value, status: Option<i64>) -> &str {
    event
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or(if status == Some(429) {
            "rate_limit"
        } else {
            "api_error"
        })
}

/// The codex `exec --json` record of a pre-session refusal: the harness's
/// own machine-readable `error` event, or a `turn.failed` that arrived
/// before any `turn.started`. Codex puts the provider's message in
/// `message` on the one and under `error.message` on the other; neither
/// is model prose, and a record with any other type is never read here.
///
/// A usage limit is the provider's own rate limit (decision 0103 ruling
/// 1(a)): `codex_error_info` names it, and an HTTP 429 is its status.
/// Every other error keeps `api_error`, class `Other`, unchanged. The
/// reset is read here too, since the stream's error may state it.
pub(super) fn codex_refusal(event: &Value) -> Option<PreSessionRefusal> {
    match event.get("type").and_then(Value::as_str) {
        Some("error" | "turn.failed") => {}
        _ => return None,
    }
    Some(codex_error(event, Now::read().as_ref(), None))
}

/// One codex refusal, from the error record a stream event or a thread
/// record carries. `resets_at` is the structured reset a thread record's
/// `token_count` states, preferred over the provider's prose (decision
/// 0103 ruling 3). A class the machine fields do not mark is `Other`.
fn codex_error(error: &Value, now: Option<&Now>, resets_at: Option<u64>) -> PreSessionRefusal {
    let info = machine_field(error, "codex_error_info").and_then(Value::as_str);
    let status = machine_field(error, "status").and_then(Value::as_i64);
    let message = machine_field(error, "message").and_then(Value::as_str);
    let rate_limit = status == Some(429) || info.is_some_and(rate_limit_info);
    if !rate_limit {
        return refusal_reason("api_error", status, message);
    }
    let retry_after = now.and_then(|now| codex_reset(error, now, resets_at));
    refusal_reason_with("rate_limit", status, message, retry_after)
}

/// A field codex spells on the record itself, or under the `error` object
/// a `turn.failed` carries. Both are codex's own machine fields; neither
/// is read from a model's prose.
fn machine_field<'a>(error: &'a Value, key: &str) -> Option<&'a Value> {
    error
        .get(key)
        .or_else(|| error.get("error").and_then(|inner| inner.get(key)))
}

/// The `codex_error_info` values that name a rate or usage limit. The
/// closed set holds the value the 2026-11-08 record measured
/// (`usage_limit_exceeded`) and codex's other rate-limit spelling; every
/// other value stays `Other`.
fn rate_limit_info(info: &str) -> bool {
    matches!(info, "usage_limit_exceeded" | "rate_limit_exceeded")
}

/// The provider's stated reset, in whole seconds from the refusal's own
/// time. A structured `resets_at` (codex's Unix second) is preferred;
/// otherwise the provider's "try again at …" text is parsed strictly.
/// An absent, stale or unparseable reset is no retry-after, never a
/// guess (decision 0103 ruling 3).
fn codex_reset(error: &Value, now: &Now, resets_at: Option<u64>) -> Option<u64> {
    if let Some(at) = resets_at {
        return seconds_between(now.epoch, i64::try_from(at).ok()?);
    }
    let message = machine_field(error, "message").and_then(Value::as_str)?;
    let stated = stated_reset(message)?;
    seconds_between(now.local.seconds()?, stated.seconds()?)
}

/// Whole seconds from `now` to `later`, only forward: a reset the
/// provider has already passed gives no wait rather than a negative one.
fn seconds_between(now: i64, later: i64) -> Option<u64> {
    u64::try_from(later.checked_sub(now)?)
        .ok()
        .filter(|seconds| *seconds > 0)
}

/// One wall-clock reading of a refusal: the Unix second, and the host's
/// local civil date and time at it. The two are read together because
/// the provider's text names no zone and the structured field is an
/// absolute second (decision 0103 ruling 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Now {
    pub(super) epoch: i64,
    pub(super) local: LocalTime,
}

impl Now {
    /// The host's clock now, or nothing where either reading is
    /// unavailable. Read only on a codex refusal that may carry a reset,
    /// so an ordinary seat pays nothing.
    pub(super) fn read() -> Option<Now> {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let epoch = i64::try_from(epoch.as_secs()).ok()?;
        Some(Now {
            epoch,
            local: local_time(epoch)?,
        })
    }
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
}

impl LocalTime {
    /// The second this civil time denotes, read as civil time. The zone
    /// cancels because both sides are read the same way; the arithmetic
    /// is `brokkr-core`'s one civil-date reader, not a second copy.
    fn seconds(self) -> Option<i64> {
        let text = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
            self.year, self.month, self.day, self.hour, self.minute
        );
        brokkr_core::stamp::parse_millis(&text).map(|millis| millis / 1000)
    }
}

/// The host's local civil time at `epoch`: `localtime_r` applies the
/// host's own zone rules to the one clock reading a refusal takes.
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
    })
}

/// Codex's own words for its reset, parsed strictly: `try again at Nov
/// 11th, 2026 10:38 AM.` No prose past that one pattern is interpreted,
/// so a message that says it differently gives no reset rather than a
/// guessed one.
fn stated_reset(message: &str) -> Option<LocalTime> {
    let tail = message.split_once("try again at ")?.1;
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

/// A day with its ordinal suffix. Only the shape is checked: the
/// provider's own spelling is `11th`, and the pattern is the provider's,
/// not a second calendar to maintain.
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

/// Read a codex thread record's own error, through the locator the
/// adapter already holds (decision 0035), when the CLI ended before its
/// first turn without a stream refusal. The record puts every field under
/// `payload`: a `task_complete`'s `error` and an `error` event are the two
/// shapes read. A normal completion is not a refusal, and a row that is
/// not JSON answers nothing.
pub(super) fn codex_thread_refusal(body: &str, now: &Now) -> Option<PreSessionRefusal> {
    let mut resets_at = None;
    let mut error = None;
    for line in body.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(at) = row
            .pointer("/payload/rate_limits/primary/resets_at")
            .and_then(Value::as_u64)
        {
            resets_at = Some(at);
        }
        if let Some(found) = thread_error(&row) {
            error = Some(found);
        }
    }
    Some(codex_error(&error?, Some(now), resets_at))
}

/// The error a rollout row names, or nothing: the last one in the file
/// is the refusal the early exit left behind.
fn thread_error(row: &Value) -> Option<Value> {
    let payload = match row.get("type").and_then(Value::as_str)? {
        "error" => return Some(row.clone()),
        "event_msg" => row.get("payload")?,
        _ => return None,
    };
    match payload.get("type").and_then(Value::as_str)? {
        "task_complete" => payload.get("error").cloned(),
        "error" => Some(payload.clone()),
        _ => None,
    }
}

/// The dsh session record's end-of-turn error (`turn/end`, its reason
/// under `data.reason`): a `QUOTA` code, 402 or 429 is the provider's rate
/// limit, typed here (decision 0103 ruling 1(a)); every other error before
/// the first turn is `Other`. Only the machine fields are read.
pub(super) fn dsh_refusal(event: &Value) -> Option<PreSessionRefusal> {
    /// The one code the 2026-11-08 session record measured.
    const DSH_RATE_LIMIT_CODES: [&str; 1] = ["QUOTA"];
    let reason = event.pointer("/data/reason")?;
    if reason.get("kind").and_then(Value::as_str) != Some("error") {
        return None;
    }
    let error = reason.get("error").unwrap_or(&Value::Null);
    let code = error.get("code").and_then(Value::as_str);
    let status = error.get("status").and_then(Value::as_i64);
    let rate_limit = status == Some(402)
        || status == Some(429)
        || code.is_some_and(|code| DSH_RATE_LIMIT_CODES.contains(&code));
    let token = if rate_limit {
        "rate_limit"
    } else {
        code.unwrap_or("api_error")
    };
    Some(refusal_reason(
        token,
        status,
        error.get("message").and_then(Value::as_str),
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
            AdapterKind::Exec => "exec",
        }
    }

    /// Whether this adapter advertises the one-turn probe (decision 0103
    /// ruling 4(a)). It is the `capabilities.probe` field, never a
    /// `supports` entry: the frozen v1 schema closes that list to
    /// `resume`, `checkpoint` and `cancel`. Exec has no model turn, so it
    /// advertises nothing.
    pub(super) fn advertises_probe(self) -> bool {
        match self {
            AdapterKind::Claude
            | AdapterKind::Lanetally
            | AdapterKind::Codex
            | AdapterKind::Dsh => true,
            AdapterKind::Exec => false,
        }
    }
}
