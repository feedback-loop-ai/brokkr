//! The runs the whole read takes, as one SQL fragment (decision 0089
//! ruling 7, slice 4's unit U5b1): `window_fallbacks`'s set, and the same
//! set every window statement leaves out, so the row path streams without
//! a Rust set of run ids.
//!
//! The open chose which arm the fragment is read by: a journal whose
//! stale-seat set is intact reads the set and the read-time shape arm
//! through their indexes; one whose set is absent or damaged reads the
//! seat-currentness rule over `run_summary`, as a reader that cannot write
//! must (decision 0014). [`Store::window_fallbacks`] and every window
//! statement ask [`stale_intact`] once and prepare the arm it names, so no
//! statement and the fallback can disagree about a run.

use crate::aggregate::outside;
use crate::event_index::{CHECKPOINTED, CHECKPOINT_STALE};
use crate::seat_summary::{lags as seat_lags, stale_runs};

/// The runs the whole read takes: the stale-seat set or the rule, then the
/// checkpoint-stale arm, as one subquery selecting each run's id and its
/// stored realm key. `intact` names the arm the open chose, and `shape`
/// the bound this build's seat shape reaches the statement by.
pub(crate) fn whole_read(intact: bool, shape: &str) -> String {
    let seats = match intact {
        true => format!(
            "SELECT r.run_id AS run_id, r.realm AS realm
               FROM ({set}) AS l CROSS JOIN run_summary AS r ON r.run_id = l.run_id
              WHERE {outside}",
            set = stale_runs(shape),
            outside = outside("r."),
        ),
        false => format!(
            "SELECT r.run_id AS run_id, r.realm AS realm
               FROM run_summary AS r LEFT JOIN seat_head AS h ON h.run_id = r.run_id
              WHERE {outside} AND {lags}",
            outside = outside("r."),
            lags = lags("r", "h", shape),
        ),
    };
    format!(
        "{seats}
         UNION
         SELECT r.run_id AS run_id, r.realm AS realm
           FROM event_index AS e INDEXED BY event_index_checkpoint_stale
           JOIN run_summary AS r ON r.run_id = e.run_id
          WHERE e.type = '{CHECKPOINTED}' AND {CHECKPOINT_STALE} AND {outside}",
        outside = outside("r."),
    )
}

/// The `WITH` entry a window statement names: the whole read's runs
/// materialized once, so every outer row probes the one set by its key.
pub(crate) fn clause(intact: bool, shape: &str) -> String {
    format!("whole AS MATERIALIZED ({})", whole_read(intact, shape))
}

/// The predicate a window statement leaves the whole read's runs out by;
/// `run` is the statement's own run-key expression.
pub(crate) fn not_whole(run: &str) -> String {
    format!("NOT EXISTS (SELECT 1 FROM whole WHERE whole.run_id = {run})")
}

/// The fallback's statement over the same fragment: the runs the whole
/// read takes, narrowed to the stored `realm` key where one is named. Its
/// two binds are the realm key or NULL, and the seat shape (`?2`).
pub(crate) fn fallbacks(intact: bool) -> String {
    format!(
        "SELECT stale.run_id FROM ({whole}) AS stale
          WHERE (?1 IS NULL OR stale.realm = ?1)
          ORDER BY stale.run_id",
        whole = whole_read(intact, "?2")
    )
}

/// Whether `run`'s seat rows or checkpoint facts are not current (ruling
/// 8): its seat head lags the summary head it names by the one rule
/// `seat_summary` keeps, or a checkpoint row's facts are not current by the
/// one rule `event_index` keeps. A window read leaves such a run whole, so
/// it never trusts a stale row, and [`Store::window_fallbacks`] names it
/// for the caller's whole read. The shape the statement binds is `shape`.
fn lags(run: &str, head: &str, shape: &str) -> String {
    format!(
        "({} OR EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_checkpoint_stale
                      WHERE type = '{CHECKPOINTED}' AND run_id = {run}.run_id
                        AND {CHECKPOINT_STALE}))",
        seat_lags(run, head, shape)
    )
}
