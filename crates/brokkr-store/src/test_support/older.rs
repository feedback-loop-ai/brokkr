//! Journals left as a binary from before a part of the store left them:
//! each part dropped, so opening the journal read-write migrates it again.

use rusqlite::Connection;

/// Leave a journal as a binary from before the summary (decision 0089)
/// left it: none of the summary's table, index or triggers, none of the
/// event index's or the seat summary's, and no backfill recorded. Opening
/// it read-write migrates it again.
pub fn unmigrate(conn: &Connection) -> rusqlite::Result<()> {
    let unsummarized = conn.execute_batch(
        "DROP TABLE run_summary;
         DROP TRIGGER run_summary_of_a_run;
         DROP TRIGGER run_summary_of_an_event;
         DROP TRIGGER run_summary_of_an_acknowledgement;
         DELETE FROM meta WHERE key = 'run_summary_backfill';",
    );
    unsummarized?;
    unindex(conn)?;
    unseat(conn)
}

/// Leave a journal as a binary from before the seat summary (decision
/// 0089 slice 2) left it: neither of its tables, and no backfill
/// recorded.
pub fn unseat(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TABLE seat_summary;
         DROP TABLE seat_head;
         DELETE FROM meta WHERE key = 'seat_summary_backfill';",
    )
}

/// Leave a journal as a binary from before the event index (decision
/// 0089 slice 2) left it: no table, indexes or triggers of it, and no
/// backfill recorded.
pub fn unindex(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DROP TABLE event_index;
         DROP TRIGGER event_index_of_an_event;
         DELETE FROM meta WHERE key = 'event_index_backfill';",
    )
}

/// Leave a journal as a binary from before journals named themselves
/// (decision 0095, #430) left it: no id. Opening it read-write names it.
pub fn unname(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("DELETE FROM meta WHERE key = 'journal_id'")
}
