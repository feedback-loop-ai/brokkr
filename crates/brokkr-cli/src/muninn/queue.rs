//! The work queue a report proposes, checked entry by entry: a crossing
//! finding cited by realm and crossing, or a run fact cited by run id and
//! sequence. A run is queued by its ending (decision 0064 ruling 6): a
//! settled run is never offered as work, and a run whose work was never
//! judged is offered as what it wants, never as its seat's failure.

use brokkr_core::ending::Ending;
use serde::Serialize;
use serde_json::Value;

use super::document::{Queued, QueuedCrossing, QueuedRun};
use super::{citation, crossing_citation, string_field, Cited, CitedCrossing, Dossier};
use crate::render::Safe;

/// What a queued run is offered as, when its ending says the work was not
/// judged: the action it wants, not a failure of the seat that was in it.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Offer {
    /// The host, a provider or the engine failed: retry once the cause is
    /// gone.
    Retry,
    /// The smith could not proceed: remove the block.
    Unblock,
}

/// Why a work-queue entry's run may not be offered as work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The cited run ended `superseded`, `withdrawn` or
    /// `delivered-elsewhere`: its work is settled.
    Settled { run_id: String, ending: Ending },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Settled { run_id, ending } => write!(
                formatter,
                "a work-queue entry cites {run_id}, which ended {}: a settled run is never \
                 offered as work",
                ending.word()
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// What the run `cited` resolved to lets a work-queue entry offer it as,
/// read by that run's own ending in the realm the citation names (decision
/// 0026 ruling 3), never by another hearth's run of the same id: refused
/// for a settled run, the action for one never judged, and nothing to add
/// for any other, a live run included.
pub(super) fn offer(dossier: &Dossier, cited: &Cited) -> Result<Option<Offer>, Refusal> {
    let (realm, run_id, _) = cited;
    let key = (realm.clone(), run_id.clone());
    match dossier.endings.get(&key).copied() {
        Some(ending @ (Ending::Superseded | Ending::Withdrawn | Ending::DeliveredElsewhere)) => {
            Err(Refusal::Settled {
                run_id: run_id.clone(),
                ending,
            })
        }
        Some(Ending::Infrastructure) => Ok(Some(Offer::Retry)),
        Some(Ending::Blocked) => Ok(Some(Offer::Unblock)),
        Some(
            Ending::Shipped
            | Ending::ShippedFlagged
            | Ending::Refused
            | Ending::Failed
            | Ending::Exhausted
            | Ending::OperatorStopped
            | Ending::Unclassified,
        )
        | None => Ok(None),
    }
}

/// One work-queue entry about a run fact, cited by run id and sequence.
pub(super) fn run(dossier: &Dossier, entry: &Value) -> Result<(Cited, Queued), String> {
    let what = "a work-queue entry";
    let cited = citation(what, dossier, entry)?;
    let offered_as = offer(dossier, &cited).map_err(|refusal| refusal.to_string())?;
    let (realm, run_id, seq) = cited;
    let finding = string_field(what, entry, "finding")?;
    let reasoning = string_field(what, entry, "reasoning")?;
    let queued = QueuedRun {
        finding,
        offered_as,
        realm: realm.clone(),
        reasoning,
        run_id: run_id.clone(),
        seq,
    };
    Ok(((realm, run_id, seq), Queued::Run(queued)))
}

/// One work-queue entry about a crossing finding, cited by realm and
/// crossing.
pub(super) fn crossing(
    dossier: &Dossier,
    entry: &Value,
) -> Result<(CitedCrossing, Queued), String> {
    let what = "a work-queue entry";
    let (realm, crossing) = crossing_citation(what, dossier, entry)?;
    let finding = string_field(what, entry, "finding")?;
    let reasoning = string_field(what, entry, "reasoning")?;
    let queued = QueuedCrossing {
        crossing: crossing.clone(),
        finding,
        realm: realm.clone(),
        reasoning,
    };
    Ok(((realm, crossing), Queued::Crossing(queued)))
}

/// How a rendered queue line says what its run is offered as: nothing for
/// an entry that offers it as no action. The record is read back from a
/// file anyone may have edited, so the word goes through `Safe` as every
/// other recorded string does.
pub(super) fn offered(item: &Value) -> String {
    match item.get("offered_as").and_then(Value::as_str) {
        Some(offer) => format!(" · offered as {}", Safe::new(offer).as_str()),
        None => String::new(),
    }
}
