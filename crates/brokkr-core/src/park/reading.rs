//! What a readout holds of a park's cause (decision 0106 ruling 1): the
//! cause its latest `run/parked` states, or the NAMED refusal of a present
//! `cause` that does not read. "A present cause that does not read is
//! refused by name, never as absent": every readout carries this value, so
//! a refusal is never painted as a park that stated nothing.
//!
//! The derivation lives here once, in the lowest crate that needs it: the
//! summary row's writer, the whole read, `inspect`, `runs` and the TUI all
//! read the journal's latest `run/parked` through [`CauseRead::latest`], and
//! the readouts only paint.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::envelope::{EventEnvelope, EventType};
use crate::summary::PARK_REASON_BYTES;

use super::ParkCause;

/// A park's cause as a readout holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CauseRead {
    /// The cause the park stated.
    Stated(ParkCause),
    /// A present `cause` that does not read, in the refusal's own words.
    Refused(String),
}

/// The wire form of a refusal: `{"refused": "<why>"}`, which no cause's
/// `kind`-tagged object can be.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefusedWire {
    refused: String,
}

impl CauseRead {
    /// What `event` states of its cause: nothing for an event that stated
    /// none, the cause where it reads, and the refusal where it does not.
    pub fn of_event(event: &EventEnvelope) -> Option<CauseRead> {
        match ParkCause::read(event) {
            Ok(None) => None,
            Ok(Some(cause)) => Some(CauseRead::Stated(cause)),
            // The refusal echoes the journal's own bytes (an unknown kind, a
            // forbidden field's name), so it is bounded where it is made:
            // the paged row and the whole read hold the same words.
            Err(error) => {
                let why = error.to_string();
                let why = crate::summary::cut(&why, PARK_REASON_BYTES);
                Some(CauseRead::Refused(why.to_string()))
            }
        }
    }

    /// What the journal's latest `run/parked` states of its cause.
    pub fn latest(events: &[EventEnvelope]) -> Option<CauseRead> {
        events
            .iter()
            .rev()
            .find(|event| event.event_type == EventType::RunParked)
            .and_then(CauseRead::of_event)
    }

    /// The one line every readout paints.
    pub fn line(&self) -> String {
        match self {
            CauseRead::Stated(cause) => cause.line(),
            CauseRead::Refused(why) => format!("refused ({why})"),
        }
    }
}

impl Serialize for CauseRead {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CauseRead::Stated(cause) => cause.serialize(serializer),
            CauseRead::Refused(why) => RefusedWire {
                refused: why.clone(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for CauseRead {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.get("refused").is_some() {
            return serde_json::from_value::<RefusedWire>(value)
                .map(|wire| CauseRead::Refused(wire.refused))
                .map_err(serde::de::Error::custom);
        }
        serde_json::from_value(value)
            .map(CauseRead::Stated)
            .map_err(serde::de::Error::custom)
    }
}
