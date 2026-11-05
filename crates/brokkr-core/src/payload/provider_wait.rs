//! The optional provider-wait extension at event_schema 1 (decision 0103
//! ruling 2(a)): one refused site's cool-down on a terminal event, the one
//! answer a dedicated probe gave, and the typed refusal that parks. `fold`
//! reads none of it; the engine, and later the readout, read it from the
//! effect's own events (decision 0103 rulings 2 to 4). A present entry that
//! cannot be read refuses by name, never as absent.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::envelope::EventEnvelope;

/// One entry of `effect/failed.waits` (and, from decision 0103 slice 4c2,
/// `effect/indeterminate.waits`): one refused site's cool-down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderWait {
    /// The invocation-site tag, `None` for an inline seat.
    pub site: Option<String>,
    pub provider: String,
    pub route: String,
    /// The provider's bounded words.
    pub words: String,
    /// The provider's own retry-after, when it gave one.
    pub retry_after_seconds: Option<u64>,
    /// The length the engine computed under THIS entry's route's bounds
    /// (decision 0103 ruling 3): the retry-after when given, else that
    /// route's default doubled for the round. An episode sums it and never
    /// recomputes an earlier wait under a later route's bounds.
    pub seconds: u64,
    /// The computed end of this round's wait.
    pub until: OffsetDateTime,
    /// Which round of the effect's and site's episode this is.
    pub rounds: u64,
}

/// One cool-down probe's answer (decision 0103 ruling 4(a)): the provider
/// and route the probe asked, and when its first model-output answer
/// arrived. It is carried typed through the engine and serialized only
/// where the retry's `effect/started` journals it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAnswer {
    pub provider: String,
    pub route: String,
    pub answered_at: OffsetDateTime,
}

impl Serialize for ProbeAnswer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut answer = serializer.serialize_struct("ProbeAnswer", 3)?;
        answer.serialize_field("provider", &self.provider)?;
        answer.serialize_field("route", &self.route)?;
        answer.serialize_field(
            "answered_at",
            &self
                .answered_at
                .format(&Rfc3339)
                .map_err(serde::ser::Error::custom)?,
        )?;
        answer.end()
    }
}

/// Why a typed rate-limit refusal ended in a park (decision 0103 slice
/// 4c1, this fix round): the refusal that survives on the failed event even
/// when no `waits` entry can be written. Closed: an unknown cause refuses
/// where it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefusalCause {
    /// The provider's route could not be resolved, so no route-wide hold
    /// was recorded.
    RouteUnresolved,
    /// The host file could not be read or was not a valid host file.
    HostUnreadable,
    /// An earlier wait of the same effect did not read.
    EpisodeUnreadable,
    /// The wait is past the declared maximum, so no wait is taken.
    PastMaximum,
    /// The journal cannot spell the wait's end, so no wait is taken.
    NoEnd,
}

/// One typed rate-limit refusal that ends in a park (decision 0103 slice
/// 4c1, this fix round), written on the effect's failed event beside the
/// `run/parked` that follows. It keeps the facts a crash between the two
/// appends would otherwise lose, so a resume can re-park from the record
/// without inventing an `until`. A `waits` entry is written only where an
/// `until` exists; this object exists even where none can.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderRefusal {
    pub cause: RefusalCause,
    pub provider: String,
    /// Where the refusal resolved one; absent for [`RefusalCause::RouteUnresolved`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    /// The provider's bounded words.
    pub words: String,
    /// The provider's own retry-after, when it gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
    /// The length the engine computed, where it reached that far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,
    /// The declared maximum the computed length was judged against, where
    /// the engine reached that far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_seconds: Option<u64>,
}

/// Why a present `refused` object does not read: the field that did not
/// read, named as the fold's own `BadPayload` does. Kept apart from
/// [`ProviderWaitError`] so each record's refusal names itself.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderRefusalError {
    #[error("event {seq}: the refused record does not read: {field}")]
    Field { seq: u64, field: String },
}

/// Why a present `waits` array does not read. `fold` never reads the
/// extension, so this is the engine's refusal, naming the field that did
/// not read as the fold's own `BadPayload` does.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderWaitError {
    #[error("event {seq}: the waits entry does not read: {field}")]
    Field { seq: u64, field: String },
}

/// The keys one `waits` entry may carry.
const WAIT_FIELDS: [&str; 8] = [
    "site",
    "provider",
    "route",
    "words",
    "retry_after_seconds",
    "seconds",
    "until",
    "rounds",
];

/// The schema's bound on a wait's `words`.
const WORDS_MAX: usize = 300;
/// The schema's bound on a wait's `site`.
const SITE_MAX: usize = 200;
/// The schema's bound on a provider's or route's length.
const NAME_MAX: usize = 80;

/// `event`'s optional `waits` array, typed. Absent is no wait; a present
/// array is every entry and is NEVER read as no wait: one entry that does
/// not read refuses by name (decision 0103 ruling 2(a)).
pub fn provider_waits(
    event: &EventEnvelope,
) -> Result<Option<Vec<ProviderWait>>, ProviderWaitError> {
    let Some(value) = event.payload.get("waits") else {
        return Ok(None);
    };
    let entries = value
        .as_array()
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| unreadable_wait(event, "waits"))?;
    entries
        .iter()
        .map(|entry| wait_entry(event, entry))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// One `waits` entry, its closed set of keys and then each field.
fn wait_entry(event: &EventEnvelope, value: &Value) -> Result<ProviderWait, ProviderWaitError> {
    let object = value
        .as_object()
        .ok_or_else(|| unreadable_wait(event, "waits"))?;
    for key in object.keys() {
        if !WAIT_FIELDS.contains(&key.as_str()) {
            return Err(unreadable_wait(event, key));
        }
    }
    let seconds = required_seconds(event, object, "seconds")?;
    let retry_after_seconds = optional_seconds(event, object, "retry_after_seconds")?;
    if retry_after_seconds.is_some_and(|retry| retry != seconds) {
        return Err(unreadable_wait(event, "seconds"));
    }
    Ok(ProviderWait {
        site: optional_site(event, object)?,
        provider: required_form(event, object, "provider", provider_form)?,
        route: required_form(event, object, "route", route_form)?,
        words: required_bounded(event, object, "words", WORDS_MAX)?,
        retry_after_seconds,
        seconds,
        until: until(event, object)?,
        rounds: rounds(event, object)?,
    })
}

/// A required non-empty string.
fn required_word(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<String, ProviderWaitError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| unreadable_wait(event, field))
}

/// A required word the schema bounds by characters.
fn required_bounded(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
    maximum: usize,
) -> Result<String, ProviderWaitError> {
    let word = required_word(event, object, field)?;
    if word.chars().count() > maximum {
        return Err(unreadable_wait(event, field));
    }
    Ok(word)
}

/// A required word the schema bounds by `form`.
fn required_form(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
    form: fn(&str) -> bool,
) -> Result<String, ProviderWaitError> {
    let word = required_word(event, object, field)?;
    if !form(&word) {
        return Err(unreadable_wait(event, field));
    }
    Ok(word)
}

/// The schema's provider pattern: an ASCII letter or digit, then up to 79
/// more letters, digits, `_`, `.` or `-`.
fn provider_form(value: &str) -> bool {
    name_form(value, false)
}

/// The schema's route pattern: the provider form, and `:`.
fn route_form(value: &str) -> bool {
    name_form(value, true)
}

fn name_form(value: &str, colon: bool) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_alphanumeric())
        && value.len() <= NAME_MAX
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'_' | b'.' | b'-')
                || (colon && byte == b':')
        })
}

/// A non-empty string when it is present; `None` when the key is absent.
/// A present `null` is refused: an absent site is the only absence.
fn optional_word(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<String>, ProviderWaitError> {
    match object.get(field) {
        None => Ok(None),
        Some(value) => value
            .as_str()
            .filter(|value| !value.is_empty())
            .map(|value| Some(value.to_string()))
            .ok_or_else(|| unreadable_wait(event, field)),
    }
}

/// The invocation-site tag, at most the schema's bound.
fn optional_site(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<Option<String>, ProviderWaitError> {
    match optional_word(event, object, "site")? {
        Some(site) if site.chars().count() > SITE_MAX => Err(unreadable_wait(event, "site")),
        site => Ok(site),
    }
}

/// A whole number above zero.
fn whole_seconds(value: &Value) -> Option<u64> {
    value.as_u64().filter(|value| *value > 0)
}

/// The wait's own length, required: an entry that does not carry it does
/// not read (decision 0103 ruling 3).
fn required_seconds(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, ProviderWaitError> {
    object
        .get(field)
        .and_then(whole_seconds)
        .ok_or_else(|| unreadable_wait(event, field))
}

/// A whole number above zero when it is present; `None` when absent.
fn optional_seconds(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ProviderWaitError> {
    match object.get(field) {
        None => Ok(None),
        Some(value) => whole_seconds(value)
            .map(Some)
            .ok_or_else(|| unreadable_wait(event, field)),
    }
}

/// The computed end, read as RFC 3339.
fn until(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<OffsetDateTime, ProviderWaitError> {
    let text = required_word(event, object, "until")?;
    OffsetDateTime::parse(&text, &Rfc3339).map_err(|_| unreadable_wait(event, "until"))
}

/// A whole round above zero.
fn rounds(event: &EventEnvelope, object: &Map<String, Value>) -> Result<u64, ProviderWaitError> {
    object
        .get("rounds")
        .and_then(Value::as_u64)
        .filter(|rounds| *rounds > 0)
        .ok_or_else(|| unreadable_wait(event, "rounds"))
}

fn unreadable_wait(event: &EventEnvelope, field: &str) -> ProviderWaitError {
    ProviderWaitError::Field {
        seq: event.seq,
        field: field.to_string(),
    }
}

/// The keys one `refused` object may carry.
const REFUSAL_FIELDS: [&str; 7] = [
    "cause",
    "provider",
    "route",
    "words",
    "retry_after_seconds",
    "seconds",
    "max_seconds",
];

/// `event`'s optional `refused` object, typed. Absent is no refusal; a
/// present object is NEVER read as absent: one field that does not read
/// refuses by name (decision 0103 slice 4c1, this fix round).
pub fn provider_refusal(
    event: &EventEnvelope,
) -> Result<Option<ProviderRefusal>, ProviderRefusalError> {
    let Some(value) = event.payload.get("refused") else {
        return Ok(None);
    };
    refusal_entry(event, value).map(Some)
}

/// One `refused` object: its closed set of keys, each field, and then the
/// shape its cause decides. Each field has its own reader so no function
/// grows past ruling 4's complexity ceiling.
fn refusal_entry(
    event: &EventEnvelope,
    value: &Value,
) -> Result<ProviderRefusal, ProviderRefusalError> {
    let object = value
        .as_object()
        .ok_or_else(|| unreadable_refusal(event, "refused"))?;
    closed_refusal(event, object)?;
    let refusal = ProviderRefusal {
        cause: refusal_cause_of(event, object)?,
        provider: refusal_provider(event, object)?,
        route: refusal_route(event, object)?,
        words: refusal_words(event, object)?,
        retry_after_seconds: refusal_length(event, object, "retry_after_seconds")?,
        seconds: refusal_length(event, object, "seconds")?,
        max_seconds: refusal_length(event, object, "max_seconds")?,
    };
    refusal_shape(event, &refusal)?;
    Ok(refusal)
}

/// One `refused` object's closed key set.
fn closed_refusal(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<(), ProviderRefusalError> {
    for key in object.keys() {
        if !REFUSAL_FIELDS.contains(&key.as_str()) {
            return Err(unreadable_refusal(event, key));
        }
    }
    Ok(())
}

/// The closed cause vocabulary read from the object.
fn refusal_cause_of(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<RefusalCause, ProviderRefusalError> {
    object
        .get("cause")
        .and_then(Value::as_str)
        .and_then(refusal_cause)
        .ok_or_else(|| unreadable_refusal(event, "cause"))
}

/// The provider name, read in the schema's own form.
fn refusal_provider(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<String, ProviderRefusalError> {
    object
        .get("provider")
        .and_then(Value::as_str)
        .filter(|provider| provider_form(provider))
        .map(str::to_string)
        .ok_or_else(|| unreadable_refusal(event, "provider"))
}

/// The bounded words, read at the schema's bound.
fn refusal_words(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<String, ProviderRefusalError> {
    object
        .get("words")
        .and_then(Value::as_str)
        .filter(|words| !words.is_empty() && words.chars().count() <= WORDS_MAX)
        .map(str::to_string)
        .ok_or_else(|| unreadable_refusal(event, "words"))
}

/// The route, absent only where the cause says none resolved.
fn refusal_route(
    event: &EventEnvelope,
    object: &Map<String, Value>,
) -> Result<Option<String>, ProviderRefusalError> {
    match object.get("route") {
        None => Ok(None),
        Some(Value::String(route)) if route_form(route) => Ok(Some(route.clone())),
        Some(_) => Err(unreadable_refusal(event, "route")),
    }
}

/// One optional computed length, refused by name when present and not a
/// whole number above zero.
fn refusal_length(
    event: &EventEnvelope,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, ProviderRefusalError> {
    optional_whole(object, field).ok_or_else(|| unreadable_refusal(event, field))
}

/// The one rule the schema's `if`/`then` states: an unresolved route names
/// none, a resolved one is named, and only a computed length carries its
/// `seconds` and `max_seconds`.
fn refusal_shape(
    event: &EventEnvelope,
    refusal: &ProviderRefusal,
) -> Result<(), ProviderRefusalError> {
    let resolved = refusal.cause != RefusalCause::RouteUnresolved;
    if resolved != refusal.route.is_some() {
        return Err(unreadable_refusal(event, "route"));
    }
    let computed = matches!(
        refusal.cause,
        RefusalCause::PastMaximum | RefusalCause::NoEnd
    );
    if !computed {
        if refusal.seconds.is_some() {
            return Err(unreadable_refusal(event, "seconds"));
        }
        if refusal.max_seconds.is_some() {
            return Err(unreadable_refusal(event, "max_seconds"));
        }
        return Ok(());
    }
    if refusal.seconds.is_none() {
        return Err(unreadable_refusal(event, "seconds"));
    }
    if refusal.max_seconds.is_none() {
        return Err(unreadable_refusal(event, "max_seconds"));
    }
    Ok(())
}

/// The closed cause vocabulary, parsed from the wire.
fn refusal_cause(value: &str) -> Option<RefusalCause> {
    match value {
        "route-unresolved" => Some(RefusalCause::RouteUnresolved),
        "host-unreadable" => Some(RefusalCause::HostUnreadable),
        "episode-unreadable" => Some(RefusalCause::EpisodeUnreadable),
        "past-maximum" => Some(RefusalCause::PastMaximum),
        "no-end" => Some(RefusalCause::NoEnd),
        _ => None,
    }
}

/// A whole number above zero when present; `None` when absent, and an outer
/// `None` when present and not one.
fn optional_whole(object: &Map<String, Value>, field: &'static str) -> Option<Option<u64>> {
    match object.get(field) {
        None => Some(None),
        Some(value) => whole_seconds(value).map(Some),
    }
}

fn unreadable_refusal(event: &EventEnvelope, field: &str) -> ProviderRefusalError {
    ProviderRefusalError::Field {
        seq: event.seq,
        field: field.to_string(),
    }
}
