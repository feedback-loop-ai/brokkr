//! The runs the whole read takes, as one SQL fragment (decision 0089
//! ruling 7, slice 4's unit U5b1): `window_fallbacks`'s set, and the same
//! set every window statement leaves out, so the row path streams without
//! a Rust set of run ids.
//!
//! The open chose which arm the fragment is read by: a journal whose
//! stale-seat set is intact reads the set and the read-time shape arm
//! through their keys; one whose set is absent or damaged reads the
//! seat-currentness rule over `run_summary`, as a reader that cannot write
//! must (decision 0014). [`Store::window_fallbacks`] and every window
//! statement ask [`stale_intact`] once and prepare the arm it names, so no
//! statement and the fallback can disagree about a run.
//!
//! [`arms`] is that one home: each arm is a key probe of `seat_lag`,
//! `seat_head` or the checkpoint index (or, by rule, the one
//! `run_summary` rule). The fallback joins them for the runs it names; a
//! window statement asks the same arms as correlated `EXISTS` probes, so
//! SQLite inlines its plain `whole` CTE and holds no set of the journal's
//! stale runs in its heap (ruling 9's aggregate exception, as amended on
//! 2026-11-13 for SQLite without lookaside). The fallback's distinct union
//! is the runs it must stream, one row at a time.

use crate::aggregate::outside;
use crate::event_index::{CHECKPOINTED, CHECKPOINT_STALE};
use crate::seat_summary::{lags as seat_lags, SHAPE};

/// The arms the whole read takes: the stale-seat set or the rule, the
/// read-time shape arm, and the checkpoint-stale arm. `run` names the key
/// a window statement constrains an arm by, or `None` for the fallback's
/// own enumeration. `shape` is this build's seat-shape bound.
fn arms(intact: bool, shape: &str, run: Option<&str>) -> Vec<String> {
    let key = |column: &str| match run {
        Some(run) => format!(" AND {column} = {run}"),
        None => String::new(),
    };
    let checkpoint = format!(
        "SELECT event_index.run_id FROM event_index
           INDEXED BY event_index_checkpoint_stale
          WHERE event_index.type = '{CHECKPOINTED}' AND {CHECKPOINT_STALE}{key}",
        key = key("event_index.run_id")
    );
    match intact {
        true => vec![
            format!(
                "SELECT run_id FROM seat_lag WHERE 1{key}",
                key = key("run_id")
            ),
            format!(
                "SELECT run_id FROM seat_head
                  WHERE shape IS NOT '{}' AND shape IS NOT {shape}{key}",
                SHAPE.as_str(),
                key = key("run_id")
            ),
            checkpoint,
        ],
        false => vec![
            format!(
                "SELECT w.run_id FROM run_summary AS w
                   LEFT JOIN seat_head AS wh ON wh.run_id = w.run_id
                  WHERE {lags}{key}",
                lags = seat_lags("w", "wh", shape),
                key = key("w.run_id")
            ),
            checkpoint,
        ],
    }
}

/// The runs the whole read takes, as a set selecting each run's id and its
/// stored realm key. `intact` names the arm the open chose, and `shape` the
/// bound this build's seat shape reaches the statement by.
pub(crate) fn whole_read(intact: bool, shape: &str) -> String {
    let probes: Vec<String> = arms(intact, shape, Some("r.run_id"))
        .into_iter()
        .map(|arm| format!("EXISTS ({arm})"))
        .collect();
    format!(
        "SELECT r.run_id AS run_id, r.realm AS realm
           FROM run_summary AS r
          WHERE {outside} AND ({probes})",
        outside = outside("r."),
        probes = probes.join(" OR ")
    )
}

/// The `WITH` entry a window statement names. A plain CTE, so SQLite
/// inlines it into the statement's one correlated probe rather than
/// materializing the journal's stale set.
pub(crate) fn clause(intact: bool, shape: &str) -> String {
    format!("whole AS ({})", whole_read(intact, shape))
}

/// The predicate a window statement leaves the whole read's runs out by;
/// `run` is the statement's own run-key expression.
pub(crate) fn not_whole(run: &str) -> String {
    format!("NOT EXISTS (SELECT 1 FROM whole WHERE whole.run_id = {run})")
}

/// The fallback's statement over the same arms: each arm's runs joined to
/// their summary row, narrowed to the stored `realm` key where one is
/// named, then made distinct and ordered by the run's id. Its two binds are
/// the realm key or NULL, and the seat shape (`?2`).
pub(crate) fn fallbacks(intact: bool) -> String {
    let selected: Vec<String> = arms(intact, "?2", None)
        .into_iter()
        .map(|arm| {
            format!(
                "SELECT r.run_id FROM ({arm}) AS a CROSS JOIN run_summary AS r
                  WHERE r.run_id = a.run_id
                    AND {outside} AND (?1 IS NULL OR r.realm = ?1)",
                outside = outside("r.")
            )
        })
        .collect();
    format!(
        "SELECT run_id FROM ({}) AS named GROUP BY run_id ORDER BY run_id",
        selected.join(" UNION ALL ")
    )
}
