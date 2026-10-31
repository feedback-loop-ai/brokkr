//! What a run in "needs you" asks of the operator (#503), and the seat
//! at work on it: the fleet's one reading of a park, a quarantine, a
//! stale run and an accepted stop that no ending has closed.

use brokkr_core::stamp::parse_millis;

use super::{
    Hire, Quarantine, RunRow, Standing, ATTEMPT_DEADLINE_SECONDS, STALE_AFTER_SECONDS,
    STALE_MARGIN_SECONDS,
};
use crate::fmt_millis;

/// What a run in "needs you" asks of the operator (#503).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Need {
    /// Parked on a ruling or a limit; its verdict says which and why.
    Parked,
    /// Its journal does not load or does not fold; the row's `detail` is
    /// the refusal.
    Quarantined(Quarantine),
    /// Its journal folds to running and has not moved for longer than
    /// the staleness bound, so nothing drives it. `silent` is how long
    /// it has been quiet, rendered.
    Stale { silent: String },
    /// A stop was accepted and no ending was written; the detail is why,
    /// in the standing's words.
    Stopping { detail: String },
}

impl Need {
    /// The word a row's standing cell prints for it.
    pub fn label(&self) -> &'static str {
        match self {
            Need::Parked => Standing::Parked.label(),
            Need::Quarantined(_) => Standing::Quarantined.label(),
            Need::Stale { .. } => "stale",
            Need::Stopping { .. } => Standing::Stopping.label(),
        }
    }

    /// The line a row prints under its title: what to do, in a few
    /// words. A park's verdict already says why it waits, so it has none.
    pub fn prompt(&self) -> Option<String> {
        match self {
            Need::Parked => None,
            Need::Quarantined(Quarantine::DoesNotFold) => {
                Some("quarantined: export and inspect".to_string())
            }
            Need::Quarantined(Quarantine::DoesNotLoad) => {
                Some("quarantined: the journal does not load; inspect it by hand".to_string())
            }
            Need::Stale { silent } => Some(format!(
                "stale: no event since {silent}; resume or conclude"
            )),
            Need::Stopping { detail } => Some(detail.clone()),
        }
    }

    /// The commands that answer it for run `run_id`, each whole: a line
    /// `brokkr` parses once its `<placeholder>` words are filled. A stale
    /// run still folds to running, and the fold admits `operator retry`
    /// only on a parked run, so a stale run is answered by `resume`, which
    /// drives it again under its pinned bundle, or by `conclude`, which
    /// closes a run believed dead. `conclude` folds a journal before it
    /// appends, so it refuses a quarantined one. `export` writes one that
    /// loads and does not fold as it stands, and refuses one the store
    /// does not load, as every verb that reads it does. `operator
    /// acknowledge` answers either quarantine without folding or loading
    /// past it (#511).
    pub fn commands(&self, run_id: &str) -> Vec<String> {
        match self {
            Need::Parked => Vec::new(),
            Need::Quarantined(Quarantine::DoesNotLoad) => vec![acknowledge(run_id)],
            Need::Quarantined(Quarantine::DoesNotFold) => {
                vec![export(run_id), acknowledge(run_id)]
            }
            Need::Stale { .. } => vec![resume(run_id), conclude(run_id)],
            Need::Stopping { .. } => vec![conclude(run_id)],
        }
    }

    /// The detail's lines for run `run_id`: why it needs the operator and
    /// the [`Need::commands`] that answer it.
    pub fn way_out(&self, run_id: &str) -> Vec<String> {
        match self {
            Need::Parked => Vec::new(),
            Need::Quarantined(Quarantine::DoesNotFold) => vec![
                format!("way out   {}", export(run_id)),
                "          and inspect the journal it writes".to_string(),
                acknowledged(run_id),
            ],
            Need::Quarantined(Quarantine::DoesNotLoad) => vec![
                "way out   none in brokkr: the store refuses this journal, export included;"
                    .to_string(),
                "          inspect the hearth's journal file by hand".to_string(),
                acknowledged(run_id),
            ],
            Need::Stopping { .. } => vec![format!("way out   {}", conclude(run_id))],
            Need::Stale { silent } => vec![
                format!(
                    "stale     no event since {silent}, past the {}h bound: an attempt's {}h \
                     deadline and a {}h margin",
                    STALE_AFTER_SECONDS / 3600,
                    ATTEMPT_DEADLINE_SECONDS / 3600,
                    STALE_MARGIN_SECONDS / 3600,
                ),
                format!("way out   {}", resume(run_id)),
                "          drives it again under its pinned bundle (or --recipe <name>);"
                    .to_string(),
                format!("          or {} closes it", conclude(run_id)),
            ],
        }
    }
}

fn acknowledge(run_id: &str) -> String {
    format!("brokkr operator acknowledge --run {run_id} --reason <why>")
}

/// The way-out line a quarantine ends on (#511).
fn acknowledged(run_id: &str) -> String {
    format!(
        "          then {} lists it under older",
        acknowledge(run_id)
    )
}

fn conclude(run_id: &str) -> String {
    format!("brokkr conclude --run {run_id} --reason <why>")
}

fn export(run_id: &str) -> String {
    format!("brokkr export --run {run_id}")
}

fn resume(run_id: &str) -> String {
    format!("brokkr resume --run {run_id} --bundle <its-bundle>")
}

/// What run `row` needs of the operator at `now`, if anything: a park,
/// a journal that does not load or fold and that the operator has not
/// acknowledged where it stands (#511), or a running journal silent for longer
/// than the bound — the longest attempt deadline a shipped seat holds
/// and a margin past it, against the time of the run's last event. A
/// run is stale only on that evidence: a clock or a last event whose
/// time does not read never makes one.
pub fn need(row: &RunRow, now: &str) -> Option<Need> {
    match row.verdict.standing {
        Standing::Quarantined if row.acknowledgement.is_some() => None,
        Standing::Quarantined => Some(Need::Quarantined(row.quarantine.unwrap_or_default())),
        Standing::Parked => Some(Need::Parked),
        Standing::Stopping => row
            .verdict
            .detail
            .clone()
            .map(|detail| Need::Stopping { detail }),
        Standing::Running => silence(row, now).map(|silent| Need::Stale { silent }),
        Standing::Shipped | Standing::Stopped | Standing::OperatorStopped => None,
    }
}

/// The seat at work on run `row` at `now`, as every surface shows it: the
/// fold's hire, except on a run [`need`] calls stale, which nothing drives
/// whatever effect its fold still holds in flight (#503 item 3).
pub fn at_work<'a>(row: &'a RunRow, now: &str) -> Option<&'a Hire> {
    match need(row, now) {
        Some(Need::Stale { .. }) => None,
        Some(Need::Parked | Need::Quarantined(_)) | Some(Need::Stopping { .. }) | None => {
            row.hire.as_ref()
        }
    }
}

/// How long the run's journal has been silent, rendered, when that is
/// past the staleness bound.
fn silence(row: &RunRow, now: &str) -> Option<String> {
    row.last_recorded_at.as_ref()?;
    let silent = parse_millis(now)?.checked_sub(row.order_at)?;
    match silent > STALE_AFTER_SECONDS * 1000 {
        true => fmt_millis(silent),
        false => None,
    }
}
