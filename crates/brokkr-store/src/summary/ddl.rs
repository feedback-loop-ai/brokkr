//! The run summary table's DDL: its definition, the seed that gives every
//! run already here a row, and the statement that sends a stale row
//! behind its head. Moved out of `summary.rs` so the summary stays within
//! one file's size bound (decision 0071 ruling 4); no public path moved.

use brokkr_core::canonical::ZERO_HASH;

use super::columns;
use super::{aggregate, cut_stamp, ending, lane, late, stop};
use super::{ACK_AT_CHARS, ACK_OPERATOR_CHARS, ACK_REASON_CHARS, STAMP_BYTES};

/// The columns an acknowledgement standing at the row's head sets,
/// `acknowledgement` naming its row, its words copied cut.
pub(super) fn acknowledged_by(acknowledgement: &str) -> String {
    format!(
        "acknowledged = 1,
         ack_operator = substr({acknowledgement}.operator, 1, {ACK_OPERATOR_CHARS}),
         ack_reason = substr({acknowledgement}.reason, 1, {ACK_REASON_CHARS}),
         ack_at = substr({acknowledgement}.recorded_at, 1, {ACK_AT_CHARS})"
    )
}

/// The table, its indexes and triggers, `IF NOT EXISTS` throughout but the
/// head trigger, which is dropped and made anew so an older build's
/// definition is replaced on the next writable open. The whole values sit
/// last, the citation's too, so a page that selects none reads none; a
/// stamp past its bound is cut by its own trigger and kept whole there. A
/// writer from before the stopping or ending columns leaves their stamp
/// off, and a trigger sends its row behind.
pub(super) fn migration() -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS run_summary (
            run_id TEXT PRIMARY KEY,
            feature_line TEXT, created_at TEXT, status TEXT, {stopping}, {ended},
            phase TEXT,
            park_cause TEXT, park_reason TEXT, rule_id TEXT, rule_from TEXT, rule_routes INTEGER,
            hire_seat TEXT, hire_failed_attempts INTEGER,
            residual_open TEXT, residual_count INTEGER,
            refusal_seq INTEGER, refusal_words TEXT,
            folded_seq INTEGER, folded_hash TEXT,
            head_seq INTEGER NOT NULL, head_hash TEXT NOT NULL, last_recorded_at TEXT,
            last_at_ms INTEGER,
            acknowledged INTEGER NOT NULL DEFAULT 0,
            ack_operator TEXT, ack_reason TEXT, ack_at TEXT,
            lane TEXT GENERATED ALWAYS AS ({lane}) VIRTUAL,
            order_at INTEGER GENERATED ALWAYS AS
                (COALESCE(last_at_ms, 9223372036854775807)) VIRTUAL,
            {aggregated}, feature TEXT, residuals TEXT, created_at_whole TEXT,
            last_recorded_at_whole TEXT, phase_whole TEXT, park_reason_whole TEXT,
            rule_id_whole TEXT, rule_from_whole TEXT, hire_seat_whole TEXT,
            refusal_words_whole TEXT, {ended_whole}, {late}
        );
        CREATE INDEX IF NOT EXISTS run_summary_page
            ON run_summary (lane, order_at DESC, run_id DESC);
        CREATE INDEX IF NOT EXISTS {REALM_INDEX}
            ON run_summary (created_at, run_id, realm, folded_seq, head_seq,
                            aggregated_seq, stop_folded_seq, ending_folded_seq,
                            late_folded_seq, refusal_words);
        CREATE TRIGGER IF NOT EXISTS run_summary_of_a_run AFTER INSERT ON runs BEGIN
            INSERT OR IGNORE INTO run_summary (run_id, head_seq, head_hash)
                VALUES (NEW.run_id, 0, '{ZERO_HASH}');
        END;
        DROP TRIGGER IF EXISTS run_summary_of_an_event;
        {head_trigger};
        CREATE TRIGGER IF NOT EXISTS run_summary_of_an_acknowledgement
            AFTER INSERT ON quarantine_acknowledgements BEGIN
            UPDATE run_summary SET {acknowledged}
                WHERE run_id = NEW.run_id AND head_seq = NEW.head_seq
                  AND head_hash = NEW.head_hash;
        END;
        CREATE TRIGGER IF NOT EXISTS run_summary_stamp_bounded
            AFTER UPDATE OF last_recorded_at ON run_summary
            WHEN length(CAST(NEW.last_recorded_at AS BLOB)) > {STAMP_BYTES} BEGIN
            UPDATE run_summary SET last_recorded_at_whole = NEW.last_recorded_at,
                last_recorded_at = {cut}
                WHERE run_id = NEW.run_id;
        END;
        {stopping_trigger} {ended_trigger} {late_trigger} {indexed}",
        lane = lane(),
        aggregated = aggregate::defined(),
        late = late::PART.defined(),
        late_trigger = late::PART.migration(),
        indexed = aggregate::migration(),
        REALM_INDEX = columns::REALM_INDEX,
        head_trigger = columns::head_trigger(),
        cut = cut_stamp("NEW.last_recorded_at"),
        acknowledged = acknowledged_by("NEW"),
        stopping = stop::PART.defined(),
        stopping_trigger = stop::PART.migration(),
        ended = ending::PART.defined(),
        ended_trigger = ending::PART.migration(),
        ended_whole = ending::defined_whole(),
    )
}

/// Every run the journal already carries, given the row the triggers
/// would have kept for it: present, at its head, its stamp bounded,
/// acknowledged where an acknowledgement stands there. A row already
/// folded at its head keeps the fold's values, so a replacement re-derives
/// no row; a row not folded there is left `behind` for the backfill to
/// refold. Each head is found by its seq on the `(run_id, seq)` key first
/// and then read by that whole key, so the seed reads one event a run,
/// never a run's every event.
pub(super) fn seed() -> String {
    format!(
        "INSERT OR IGNORE INTO run_summary (run_id, head_seq, head_hash)
             SELECT run_id, 0, '{ZERO_HASH}' FROM runs;
         UPDATE run_summary SET head_seq =
             COALESCE((SELECT MAX(seq) FROM events WHERE run_id = run_summary.run_id), 0);
         UPDATE run_summary SET {moved} FROM events AS head
             WHERE head.run_id = run_summary.run_id AND head.seq = run_summary.head_seq
               AND run_summary.folded_seq IS NOT run_summary.head_seq;
         UPDATE run_summary SET {acknowledged} FROM quarantine_acknowledgements AS ack
             WHERE ack.run_id = run_summary.run_id AND ack.head_seq = run_summary.head_seq
               AND ack.head_hash = run_summary.head_hash;",
        moved = columns::moved_to("head"),
        acknowledged = acknowledged_by("ack"),
    )
}

/// Send behind every row a stamp says is not its folded head's, as the
/// triggers do: an older writer may have folded while they were gone. The
/// open's backfill refolds the rows this sends behind (rulings 4 and 8).
pub(super) fn send_stale_behind() -> String {
    format!(
        "UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL
         WHERE {stale}",
        stale = columns::stale("")
    )
}
