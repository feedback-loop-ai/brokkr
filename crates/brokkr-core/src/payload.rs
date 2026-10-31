//! The payloads the fold reads, typed (#345, the reader half): one struct
//! per event type the fold takes a field from, read from the journal
//! once, here. Each types exactly what the fold reads, as strictly as the
//! fold always read it, and keeps every other key as the journal wrote
//! it: the v1 fields the fold does not read and every extension field,
//! which the fold never reads (contracts/README.md, AC-13). So a struct
//! serializes to the payload it was read from, `null` and absent kept
//! apart, and no journal byte moves.
//!
//! A struct is read by hand rather than by serde's derive: the fold
//! refuses the first field that does not read, in the order it reads
//! them, and in its own words.

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::decision::Decision;
use crate::envelope::{EventEnvelope, EventType};
use crate::fold::FoldError;

/// The keys a payload carries that its struct does not type, as the
/// journal wrote them. They are borrowed from the event, never copied:
/// the fold reads every checkpoint a long run journals, and copying each
/// one's body would cost it several times the fold itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Verbatim<'a> {
    /// The payload, when it is the object the contract says it is.
    payload: Option<&'a Map<String, Value>>,
    /// The keys read into the struct's typed fields.
    typed: Vec<&'static str>,
}

impl Serialize for Verbatim<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let untyped = self.payload.into_iter().flatten();
        serializer.collect_map(untyped.filter(|(key, _)| !self.typed.contains(&key.as_str())))
    }
}

/// One event's payload, read field by field in the fold's order. A field
/// that reads leaves the verbatim keys for its typed field; one that does
/// not stays among them.
struct Fields<'a> {
    seq: u64,
    rest: Verbatim<'a>,
}

impl<'a> Fields<'a> {
    fn of(event: &'a EventEnvelope) -> Fields<'a> {
        Fields {
            seq: event.seq,
            rest: Verbatim {
                payload: event.payload.as_object(),
                typed: Vec::new(),
            },
        }
    }

    /// `field`, when it is present and `read` reads it.
    fn read<T>(&mut self, field: &'static str, read: fn(&'a Value) -> Option<T>) -> Option<T> {
        let value = read(self.rest.payload?.get(field)?)?;
        self.rest.typed.push(field);
        Some(value)
    }

    /// `field` as the contract requires it, or the refusal naming it.
    fn required<T>(
        &mut self,
        field: &'static str,
        read: fn(&'a Value) -> Option<T>,
    ) -> Result<T, FoldError> {
        self.read(field, read).ok_or_else(|| FoldError::BadPayload {
            seq: self.seq,
            field: field.to_string(),
        })
    }

    /// A string the fold cannot do without.
    fn string(&mut self, field: &'static str) -> Result<&'a str, FoldError> {
        self.required(field, Value::as_str)
    }

    /// A string the fold reads when it is one and does without otherwise.
    fn lenient(&mut self, field: &'static str) -> Option<&'a str> {
        self.read(field, Value::as_str)
    }

    /// Any value, `null` included, that must be present.
    fn present(&mut self, field: &'static str) -> Result<&'a Value, FoldError> {
        self.required(field, Some)
    }

    fn rest(self) -> Verbatim<'a> {
        self.rest
    }
}

/// `phase/entered`: the phase entered.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PhaseEntered<'a> {
    pub(crate) phase: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> PhaseEntered<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<PhaseEntered<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(PhaseEntered {
            phase: fields.string("phase")?,
            rest: fields.rest(),
        })
    }
}

/// `effect/requested`: the effect, and the seat hired for it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectRequested<'a> {
    pub(crate) effect_id: &'a str,
    pub(crate) seat: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectRequested<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<EffectRequested<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(EffectRequested {
            effect_id: fields.string("effect_id")?,
            seat: fields.string("seat")?,
            rest: fields.rest(),
        })
    }
}

/// `effect/started`: the effect, and the attempt that started it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectStarted<'a> {
    pub(crate) effect_id: &'a str,
    pub(crate) attempt_id: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectStarted<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<EffectStarted<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(EffectStarted {
            effect_id: fields.string("effect_id")?,
            attempt_id: fields.string("attempt_id")?,
            rest: fields.rest(),
        })
    }
}

/// `effect/checkpointed`: the effect whose attempt checkpointed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectCheckpointed<'a> {
    pub(crate) effect_id: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectCheckpointed<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<EffectCheckpointed<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(EffectCheckpointed {
            effect_id: fields.string("effect_id")?,
            rest: fields.rest(),
        })
    }
}

/// `effect/failed`: the effect whose attempt failed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectFailed<'a> {
    pub(crate) effect_id: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectFailed<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<EffectFailed<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(EffectFailed {
            effect_id: fields.string("effect_id")?,
            rest: fields.rest(),
        })
    }
}

/// `operator/commanded`: the command's id and its word.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct OperatorCommanded<'a> {
    pub(crate) command_id: &'a str,
    pub(crate) command: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> OperatorCommanded<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<OperatorCommanded<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(OperatorCommanded {
            command_id: fields.string("command_id")?,
            command: fields.string("command")?,
            rest: fields.rest(),
        })
    }
}

/// `operator/accepted`: the command accepted.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct OperatorAccepted<'a> {
    pub(crate) command_id: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> OperatorAccepted<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<OperatorAccepted<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(OperatorAccepted {
            command_id: fields.string("command_id")?,
            rest: fields.rest(),
        })
    }
}

/// `run/parked`: why the run parked.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct RunParked<'a> {
    pub(crate) reason: &'a str,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> RunParked<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<RunParked<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(RunParked {
            reason: fields.string("reason")?,
            rest: fields.rest(),
        })
    }
}

/// `run/started`: the feature, when it is a string. A run whose feature
/// does not read still starts, with none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct RunStarted<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) feature: Option<&'a str>,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> RunStarted<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> RunStarted<'a> {
        let mut fields = Fields::of(event);
        RunStarted {
            feature: fields.lenient("feature"),
            rest: fields.rest(),
        }
    }
}

/// `effect/succeeded`: the effect, and its result, which is any value,
/// `null` included, but must be present.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectSucceeded<'a> {
    pub(crate) effect_id: &'a str,
    pub(crate) result: &'a Value,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectSucceeded<'a> {
    /// The effect is read first and `placed` asks whether the fold takes
    /// it there; the result is read only then, so a succeeded event that
    /// names another effect is out of place whatever its body.
    pub(crate) fn read(
        event: &'a EventEnvelope,
        placed: impl FnOnce(&str) -> Result<(), FoldError>,
    ) -> Result<EffectSucceeded<'a>, FoldError> {
        let mut fields = Fields::of(event);
        let effect_id = fields.string("effect_id")?;
        placed(effect_id)?;
        Ok(EffectSucceeded {
            effect_id,
            result: fields.present("result")?,
            rest: fields.rest(),
        })
    }
}

/// `effect/indeterminate`: the effect, and the reason, when it is a
/// string. A close whose reason does not read records no detail.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct EffectIndeterminate<'a> {
    pub(crate) effect_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<&'a str>,
    #[serde(flatten)]
    pub(crate) rest: Verbatim<'a>,
}

impl<'a> EffectIndeterminate<'a> {
    pub(crate) fn read(event: &'a EventEnvelope) -> Result<EffectIndeterminate<'a>, FoldError> {
        let mut fields = Fields::of(event);
        Ok(EffectIndeterminate {
            effect_id: fields.string("effect_id")?,
            reason: fields.lenient("reason"),
            rest: fields.rest(),
        })
    }
}

/// `transition/decided`: its `from` and `result`, each refused by name,
/// then the whole closed [`Decision`], refused as a decision that does
/// not read.
pub(crate) fn decided(event: &EventEnvelope) -> Result<Decision, FoldError> {
    let mut fields = Fields::of(event);
    fields.string("from")?;
    fields.string("result")?;
    Decision::deserialize(&event.payload).map_err(|error| FoldError::BadDecision {
        seq: event.seq,
        detail: error.to_string(),
    })
}

/// The refusal for an event the fold cannot place. An event that names
/// its target is read for it first, so one both malformed and out of
/// place is refused for the field that does not read; any other is
/// refused as `out_of_place`.
pub(crate) fn unplaced(event: &EventEnvelope, out_of_place: FoldError) -> FoldError {
    use EventType as Type;
    let malformed = match event.event_type {
        Type::PhaseEntered => PhaseEntered::read(event).err(),
        Type::EffectStarted => EffectStarted::read(event).err(),
        Type::EffectSucceeded => EffectSucceeded::read(event, |_| Err(out_of_place.clone())).err(),
        Type::EffectFailed => EffectFailed::read(event).err(),
        Type::EffectIndeterminate => EffectIndeterminate::read(event).err(),
        Type::RunStarted
        | Type::EffectRequested
        | Type::EffectCheckpointed
        | Type::TransitionDecided
        | Type::OperatorCommanded
        | Type::OperatorAccepted
        | Type::OperatorRejected
        | Type::RunParked
        | Type::RunCompleted
        | Type::RunStopped => None,
    };
    malformed.unwrap_or(out_of_place)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contract_lifecycle_tests;
