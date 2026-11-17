//! A park cause read from its journal value, field by field, into typed
//! faults (decision 0106 ruling 1). The contract's shapes are closed: the
//! `kind` chooses one, a field the kind forbids is refused by name, a field
//! it requires is refused by name, and every form the schema states is held
//! ([`super::forms`]). Reading by hand rather than through serde's derive is
//! what lets each refusal be a variant a reader matches, not text it greps
//! (decision 0071 ruling 8).

use serde_json::{Map, Value};

use super::{forms, until, FailureClass, ParkCause, ParkCauseError, ProviderLimitRefusal};

/// The four words a writer states.
const KINDS: [&str; 4] = ["ruled", "attempt-cap", "infrastructure", "provider-limit"];

/// The fields of one object, read as the kind that chose it.
struct Fields<'a> {
    kind: &'static str,
    object: &'a Map<String, Value>,
}

impl<'a> Fields<'a> {
    /// `object` as a `kind`, which may hold `allowed` and its `kind`.
    fn of(
        kind: &'static str,
        object: &'a Map<String, Value>,
        allowed: &[&str],
    ) -> Result<Fields<'a>, ParkCauseError> {
        let forbidden = object
            .keys()
            .find(|key| *key != "kind" && !allowed.contains(&key.as_str()));
        match forbidden {
            Some(field) => Err(ParkCauseError::Forbidden {
                kind,
                field: field.clone(),
            }),
            None => Ok(Fields { kind, object }),
        }
    }

    fn missing(&self, field: &'static str) -> ParkCauseError {
        ParkCauseError::Missing {
            kind: self.kind,
            field,
        }
    }

    fn wrong(&self, field: &'static str, expected: &'static str) -> ParkCauseError {
        ParkCauseError::WrongType {
            kind: self.kind,
            field,
            expected,
        }
    }

    /// A required string.
    fn text(&self, field: &'static str) -> Result<&'a str, ParkCauseError> {
        match self.object.get(field) {
            None => Err(self.missing(field)),
            Some(Value::String(text)) => Ok(text),
            Some(_) => Err(self.wrong(field, "a string")),
        }
    }

    /// A string that may be absent; a present `null` is not absent.
    fn optional_text(&self, field: &'static str) -> Result<Option<&'a str>, ParkCauseError> {
        match self.object.get(field) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text)),
            Some(_) => Err(self.wrong(field, "a string")),
        }
    }

    fn flag(&self, field: &'static str) -> Result<bool, ParkCauseError> {
        match self.object.get(field) {
            None => Err(self.missing(field)),
            Some(Value::Bool(flag)) => Ok(*flag),
            Some(_) => Err(self.wrong(field, "a boolean")),
        }
    }

    /// A required whole number of seconds, at least one.
    fn seconds(&self, field: &'static str) -> Result<u64, ParkCauseError> {
        let seconds = match self.object.get(field) {
            None => return Err(self.missing(field)),
            Some(value) => value
                .as_u64()
                .ok_or_else(|| self.wrong(field, "a whole number"))?,
        };
        forms::seconds(field, seconds)?;
        Ok(seconds)
    }
}

/// The cause `value` states.
pub(super) fn cause(value: &Value) -> Result<ParkCause, ParkCauseError> {
    let object = value.as_object().ok_or(ParkCauseError::NotAnObject)?;
    let text = match object.get("kind") {
        Some(Value::String(text)) => text,
        _ => return Err(ParkCauseError::UnknownKind(None)),
    };
    let kind = KINDS
        .into_iter()
        .find(|kind| kind == text)
        .ok_or_else(|| ParkCauseError::UnknownKind(Some(text.clone())))?;
    let cause = match kind {
        "ruled" => ruled(object)?,
        "attempt-cap" => attempt_cap(object)?,
        "infrastructure" => infrastructure(object)?,
        _ => provider_limit(object)?,
    };
    forms::detail(cause.detail())?;
    Ok(cause)
}

fn ruled(object: &Map<String, Value>) -> Result<ParkCause, ParkCauseError> {
    let fields = Fields::of("ruled", object, &["rule_id", "detail"])?;
    let rule_id = fields.optional_text("rule_id")?;
    if let Some(rule_id) = rule_id {
        forms::rule_id(rule_id)?;
    }
    Ok(ParkCause::ruled(
        rule_id.map(str::to_string),
        fields.text("detail")?,
    ))
}

fn attempt_cap(object: &Map<String, Value>) -> Result<ParkCause, ParkCauseError> {
    let fields = Fields::of("attempt-cap", object, &["failure_class", "detail"])?;
    let class = fields.text("failure_class")?;
    Ok(ParkCause::attempt_cap(
        FailureClass::parse(class)?,
        fields.text("detail")?,
    ))
}

fn infrastructure(object: &Map<String, Value>) -> Result<ParkCause, ParkCauseError> {
    let fields = Fields::of("infrastructure", object, &["retryable", "detail"])?;
    Ok(ParkCause::infrastructure(
        fields.flag("retryable")?,
        fields.text("detail")?,
    ))
}

fn provider_limit(object: &Map<String, Value>) -> Result<ParkCause, ParkCauseError> {
    let fields = Fields::of("provider-limit", object, &["provider", "detail", "refusal"])?;
    let provider = fields.text("provider")?;
    forms::provider(provider)?;
    let detail = fields.text("detail")?;
    let refusal = match object.get("refusal") {
        None => return Err(fields.missing("refusal")),
        Some(Value::Object(refusal)) => refusal,
        Some(_) => return Err(fields.wrong("refusal", "an object")),
    };
    Ok(ParkCause::ProviderLimit {
        provider: provider.to_string(),
        detail: detail.to_string(),
        refusal: self::refusal(refusal)?,
    })
}

/// The refusal variant an object states, with exactly the fields it has.
fn refusal(object: &Map<String, Value>) -> Result<ProviderLimitRefusal, ParkCauseError> {
    let text = match object.get("kind") {
        Some(Value::String(text)) => text.as_str(),
        _ => return Err(ParkCauseError::UnknownRefusal(None)),
    };
    Ok(match text {
        "route-unresolved" => {
            Fields::of("route-unresolved", object, &[])?;
            ProviderLimitRefusal::RouteUnresolved
        }
        "host-unreadable" => ProviderLimitRefusal::HostUnreadable {
            route: route(&Fields::of("host-unreadable", object, &["route"])?)?,
        },
        "episode-unreadable" => ProviderLimitRefusal::EpisodeUnreadable {
            route: route(&Fields::of("episode-unreadable", object, &["route"])?)?,
        },
        "past-maximum" => {
            let names = ["route", "until", "seconds", "max_seconds"];
            let fields = Fields::of("past-maximum", object, &names)?;
            let route = route(&fields)?;
            let end = fields.text("until")?;
            ProviderLimitRefusal::PastMaximum {
                route,
                until: until::parse(end).ok_or_else(|| ParkCauseError::Until(end.to_string()))?,
                seconds: fields.seconds("seconds")?,
                max_seconds: fields.seconds("max_seconds")?,
            }
        }
        "no-end" => {
            let fields = Fields::of("no-end", object, &["route", "seconds", "max_seconds"])?;
            ProviderLimitRefusal::NoEnd {
                route: route(&fields)?,
                seconds: fields.seconds("seconds")?,
                max_seconds: fields.seconds("max_seconds")?,
            }
        }
        other => return Err(ParkCauseError::UnknownRefusal(Some(other.to_string()))),
    })
}

/// A refusal's required route, in the route form.
fn route(fields: &Fields<'_>) -> Result<String, ParkCauseError> {
    let route = fields.text("route")?;
    forms::route(route)?;
    Ok(route.to_string())
}
