//! An event's row in the journal's event index (decision 0089 rulings 1
//! and 3): its type, the attempt and member it names, and its time,
//! projected from the event alone.
//!
//! The store keeps the row by a trigger that reads the same fields from
//! the envelope's text in SQL, so every writer keeps it, an older
//! binary's included. Ruling 4's comparison holds each row equal to this
//! projection of its event, so the trigger and every reader derive the
//! row one way.

use serde_json::Value;

use crate::envelope::{EventEnvelope, EventType};
use crate::summary::{cut, STAMP_BYTES};

/// One event's index row, keyed by its run and `seq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indexed<'a> {
    pub seq: u64,
    pub event_type: EventType,
    /// The attempt the payload names, which the fold and every reader key
    /// an attempt by; never the envelope's own `attempt_id`.
    pub attempt_id: Option<&'a str>,
    /// The participant the payload's record names: its checkpoint's, or
    /// where it carries no checkpoint, its result's. A seat's own record
    /// names none.
    pub member: Option<&'a str>,
    /// The envelope's `recorded_at`, cut as the run summary cuts a stamp,
    /// to at most [`STAMP_BYTES`] at a character boundary.
    pub recorded_at: &'a str,
}

impl<'a> Indexed<'a> {
    /// `event`'s row. Each name is read only where it is text, as every
    /// reader reads it.
    pub fn of(event: &'a EventEnvelope) -> Indexed<'a> {
        let payload = &event.payload;
        let record = payload.get("checkpoint").or_else(|| payload.get("result"));
        Indexed {
            seq: event.seq,
            event_type: event.event_type,
            attempt_id: text(Some(payload), "attempt_id"),
            member: text(record, "member"),
            recorded_at: cut(&event.recorded_at, STAMP_BYTES),
        }
    }
}

/// `key` of `value`, where it is text.
fn text<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a str> {
    value?.get(key)?.as_str()
}

#[cfg(test)]
mod tests;
