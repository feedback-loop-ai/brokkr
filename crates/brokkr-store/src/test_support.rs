//! Rows planted past every writer the store keeps, for tests (#354).
//!
//! A test that needs a journal the store would never write — a broken
//! chain, a record from before a fence, a row an older binary left —
//! plants it here rather than in its own SQL, so the statements a planted
//! row goes through are the ones the store's own writers use. The `runs`
//! and `events` tables admit anything the schema's guards do; nothing
//! here chains, seals or validates.
//!
//! Compiled for this crate's own tests and behind the `test-support`
//! feature, which only dev-dependencies enable.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use rusqlite::{params, Connection, ToSql};

use crate::import::INSERT_EVENT;

/// Plant a `runs` row carrying this manifest text verbatim, valid JSON or
/// not: a run as a foreign or older writer left it, with no origin and no
/// arrival. Inside a transaction a test holds open, it is also how the
/// test takes the journal's write lock.
pub fn plant_run(conn: &Connection, run_id: &str, manifest: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
         VALUES (?1, 'feat', 'self', ?2, '2026-01-01T00:00:00Z')",
        params![run_id, manifest],
    )
}

/// Plant an event row exactly as given — any seq SQLite will bind, any
/// hash, any envelope text — past the append fence and the chain.
pub fn plant_event(
    conn: &Connection,
    run_id: &str,
    seq: &dyn ToSql,
    event_hash: &str,
    envelope: &str,
) -> rusqlite::Result<usize> {
    conn.execute(INSERT_EVENT, params![run_id, seq, event_hash, envelope])
}

/// Plant a sealed envelope under its own run, seq and hash, serialized
/// the way the store serializes the envelopes it writes.
pub fn plant_envelope(conn: &Connection, envelope: &EventEnvelope) -> rusqlite::Result<usize> {
    let text = serde_json::to_string(envelope).expect("an envelope serializes");
    plant_event(
        conn,
        &envelope.run_id,
        &(envelope.seq as i64),
        &envelope.event_hash,
        &text,
    )
}

/// Plant a copy of `event` at `seq` that chains onto no event — its
/// `previous_hash` is [`ZERO_HASH`] and its recorded hash still `event`'s
/// — as a chain broken behind the store's back.
pub fn plant_broken_link(
    conn: &Connection,
    event: &EventEnvelope,
    seq: u64,
) -> rusqlite::Result<usize> {
    let broken = EventEnvelope {
        seq,
        previous_hash: ZERO_HASH.to_string(),
        ..event.clone()
    };
    plant_envelope(conn, &broken)
}

/// Take the seq guard (#354) off a journal, leaving it as a journal
/// written before the guard existed: one that may hold a row at a seq
/// that is not a position. Opening the journal read-write restores it.
pub fn drop_seq_guard(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DROP TRIGGER IF EXISTS events_seq_is_a_position")
}
