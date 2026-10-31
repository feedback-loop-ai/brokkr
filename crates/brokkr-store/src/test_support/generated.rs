//! The generated journal the fleet's page is measured on (decision 0089
//! ruling 9): runs of any standing and any number of events, each row
//! folded at its run's head as a writer leaves it, written past every
//! writer in one transaction so a journal of a million events is built in
//! seconds. A page reads no event body, so the events are short and their
//! hashes chain nothing; only each run's last event is read, by its key,
//! and its seq and hash are the ones the row was folded to.

use brokkr_core::page::Lane;
use brokkr_core::policy::Severity;
use brokkr_core::residual::{Citation, Claim, Mark, ResidualPhase};
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::{
    feature_line, Bounded, Hired, Refused, Ruled, Stated, Summary, FEATURE_LINE_BYTES, NAME_BYTES,
    PARK_REASON_BYTES, REFUSAL_BYTES, STAMP_BYTES,
};
use brokkr_core::Status;
use rusqlite::params;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{seat_summary, summary, Store, StoreError};

/// When every generated run was created.
const CREATED: &str = "2026-01-01T00:00:00Z";

/// How wide a value past its bound is: a mebibyte.
const WIDE: usize = 1 << 20;

/// How a generated run stands: the lane its row is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Running,
    Parked,
    Finished,
    Refused,
    Acknowledged,
}

impl Standing {
    /// The lane a row of this standing stands in at its head.
    pub fn lane(self) -> Lane {
        match self {
            Standing::Running => Lane::Running,
            Standing::Parked => Lane::Parked,
            Standing::Finished => Lane::Finished,
            Standing::Refused => Lane::Refused,
            Standing::Acknowledged => Lane::Acknowledged,
        }
    }
}

/// How wide a generated run's values are: short, each exactly at its
/// bound, or each a mebibyte with ten thousand residual findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Narrow,
    Bound,
    Wide,
}

/// One run of a generated journal.
#[derive(Debug, Clone)]
pub struct Generated {
    pub run_id: String,
    pub standing: Standing,
    /// Its events, seq 1 onward; at least one.
    pub events: u64,
    /// Its last event's stamp, which orders and sections it.
    pub last_at: String,
    pub width: Width,
}

/// The runs a fleet page is measured on (ruling 9): 2,000 runs of
/// `events` each, twenty each parked, refused, running and finished within
/// the day before `now`, then an acknowledged refusal and the older
/// finished runs, so a page of a hundred lists in every section. The
/// parked run, the acknowledged one and the first finished within the day
/// are `stretched`: the rows that would stretch a page.
pub fn fleet(now: &str, events: u64, stretched: Width) -> Vec<Generated> {
    const DAY: i64 = 24 * 60;
    let now = OffsetDateTime::parse(now, &Rfc3339).expect("the clock is a stamp");
    let ago = |minutes: i64| {
        let at = now - time::Duration::minutes(minutes);
        at.format(&Rfc3339).expect("a stamp formats")
    };
    (0..2_000i64)
        .map(|n| {
            let (standing, minutes) = match n {
                0..20 => (Standing::Parked, n),
                20..40 => (Standing::Refused, n),
                40..60 => (Standing::Running, n),
                60..80 => (Standing::Finished, n),
                80 => (Standing::Acknowledged, DAY + 1),
                _ => (Standing::Finished, 2 * DAY + n),
            };
            Generated {
                run_id: format!("run-{n:05}"),
                standing,
                events,
                last_at: ago(minutes),
                width: match n {
                    0 | 60 | 80 => stretched,
                    _ => Width::Narrow,
                },
            }
        })
        .collect()
}

/// Write `runs` into `store`'s journal in one transaction: each run's
/// `runs` row, its events, its summary row folded at its head, its seat
/// head there with no seat row, as its events state no seat, and, for an
/// acknowledged one, the acknowledgement standing there.
pub fn generate(store: &Store, runs: &[Generated]) -> Result<(), StoreError> {
    let tx = store.conn.unchecked_transaction()?;
    for run in runs {
        let summary = summary_of(run);
        tx.execute(
            "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
             VALUES (?1, ?2, 'self', '{}', ?3)",
            params![run.run_id, summary.feature, CREATED],
        )?;
        tx.execute(
            "WITH RECURSIVE n(seq) AS (SELECT 1 UNION ALL SELECT seq + 1 FROM n WHERE seq < ?2)
             INSERT INTO events (run_id, seq, event_hash, envelope)
             SELECT ?1, seq, printf('%064d', seq), json_object('recorded_at', ?3) FROM n",
            params![run.run_id, run.events as i64, run.last_at],
        )?;
        summary::write(&tx, &run.run_id, &summary)?;
        seat_summary::write_head(&tx, &run.run_id, (run.events, &summary.folded_hash))?;
        if run.standing == Standing::Acknowledged {
            let words = |narrow: &str, chars: usize| value(run.width, narrow, chars);
            tx.execute(
                "INSERT INTO quarantine_acknowledgements
                     (run_id, head_seq, head_hash, operator, reason, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    run.run_id,
                    run.events as i64,
                    summary.folded_hash,
                    words("op", 64),
                    words("looked at it", 256),
                    CREATED
                ],
            )?;
        }
    }
    Ok(tx.commit()?)
}

/// `narrow`, or a value exactly `bound` long, or a mebibyte.
fn value(width: Width, narrow: &str, bound: usize) -> String {
    match width {
        Width::Narrow => narrow.to_string(),
        Width::Bound => "b".repeat(bound),
        Width::Wide => "w".repeat(WIDE),
    }
}

/// The summary a writer would keep for `run`.
fn summary_of(run: &Generated) -> Summary {
    let bounded =
        |narrow: &str, bound: usize| Bounded::within(&value(run.width, narrow, bound), bound);
    let feature = value(
        run.width,
        &format!("{} feature\nin detail", run.run_id),
        FEATURE_LINE_BYTES,
    );
    let refused = matches!(run.standing, Standing::Refused | Standing::Acknowledged);
    let status = match run.standing {
        Standing::Running => Some(Status::Running),
        Standing::Parked => Some(Status::AwaitingOperator),
        Standing::Finished => Some(Status::Completed),
        Standing::Refused | Standing::Acknowledged => None,
    };
    let findings = match run.width {
        Width::Wide => 10_000,
        Width::Narrow | Width::Bound => 1,
    };
    let hash = format!("{:064}", run.events);
    Summary {
        feature_line: feature_line(&feature).to_string(),
        created_at: Bounded::within(CREATED, STAMP_BYTES),
        status,
        stop_accepted_seq: None,
        phase: (!refused).then(|| bounded("review", NAME_BYTES)),
        park_reason: (run.standing == Standing::Parked)
            .then(|| bounded("REVIEW-HOLD for (review, hold): held", PARK_REASON_BYTES)),
        ruled: (!refused).then(|| Ruled {
            rule_id: Some(bounded("REVIEW-HOLD", NAME_BYTES)),
            from: bounded("review", NAME_BYTES),
            routes: run.standing != Standing::Parked,
        }),
        hired: (!refused).then(|| Hired {
            seat: bounded("reviewer", NAME_BYTES),
            failed_attempts: 0,
        }),
        residual_open: Some(Severity::Low),
        residual_count: findings,
        residuals: residuals(&run.run_id, findings),
        refused: refused.then(|| Refused {
            seq: run.events,
            words: bounded("run/started is not the first event", REFUSAL_BYTES),
        }),
        folded_seq: run.events,
        folded_hash: hash.clone(),
        last_at_ms: parse_millis(&run.last_at),
        head_seq: run.events,
        head_hash: hash,
        last_recorded_at: Some(Bounded::within(&run.last_at, STAMP_BYTES)),
        feature,
        aggregate: brokkr_core::summary::aggregate(&[], None),
    }
}

/// `count` open low findings of `run_id`, the first closed by a mark, as
/// the `residuals` column states them.
fn residuals(run_id: &str, count: u64) -> String {
    let stated: Vec<Stated> = (1..=count)
        .map(|seq| Stated {
            seq,
            phase: ResidualPhase::Review,
            rule_id: Some("REVIEW-HOLD".to_string()),
            claim: Claim::MaxSeverity(Severity::Low),
            superseded: (seq == 1).then(|| Mark {
                seq: count + 1,
                by: Citation {
                    realm: None,
                    run_id: run_id.to_string(),
                    seq: 1,
                },
                reason: Some("fixed".to_string()),
                operator: None,
                recorded_at: CREATED.to_string(),
            }),
        })
        .collect();
    serde_json::to_value(&stated)
        .expect("findings serialize")
        .to_string()
}
