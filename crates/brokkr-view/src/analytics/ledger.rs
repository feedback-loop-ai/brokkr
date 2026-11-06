//! The ledger (decision 0064 ruling 7): how a comparison drawn from the
//! journal counts runs by their endings. Five endings are outcomes of the
//! work, and only they reach a success rate. The six in which the work
//! was never judged to its end are excluded and counted by ending, and an
//! `unclassified` run is excluded and counted apart. The one reading every
//! comparison takes: analytics, the recipe's graph, `brokkr compare` and
//! Muninn's dossier (decision 0071 ruling 7).

use std::collections::BTreeMap;

use brokkr_core::ending::Ending;
use serde::Serialize;

use super::words::percent;
use crate::ABSENT;

/// How ruling 7 reads one ending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// An outcome of the work: `shipped`, `shipped-flagged`, `refused`,
    /// `failed` or `exhausted`.
    Outcome { shipped: bool },
    /// The work was not judged to its end.
    Excluded,
    /// The journal does not say.
    Unclassified,
}

const fn reading(ending: Ending) -> Reading {
    match ending {
        Ending::Shipped | Ending::ShippedFlagged => Reading::Outcome { shipped: true },
        Ending::Refused | Ending::Failed | Ending::Exhausted => Reading::Outcome { shipped: false },
        Ending::Blocked
        | Ending::Infrastructure
        | Ending::Superseded
        | Ending::Withdrawn
        | Ending::DeliveredElsewhere
        | Ending::OperatorStopped => Reading::Excluded,
        Ending::Unclassified => Reading::Unclassified,
    }
}

/// Whether a run that ended `ending` is an outcome of the work that did
/// not ship: what a figure counts as stopped beside the shipped.
pub(crate) const fn judged_unshipped(ending: Ending) -> bool {
    matches!(reading(ending), Reading::Outcome { shipped: false })
}

/// A set of finished runs, read as ruling 7 reads them.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Ledger {
    /// The runs whose ending is an outcome of the work.
    pub judged: usize,
    /// Of those, the runs that shipped, flagged or not.
    pub shipped: usize,
    /// `shipped` as a percent of `judged`; `None` when nothing was judged.
    pub rate_percent: Option<i64>,
    /// The runs excluded from the rate, by ending: each of the six, zero
    /// included, keyed by its word.
    pub excluded: BTreeMap<Ending, usize>,
    /// The runs the journal does not say the ending of, excluded and
    /// counted apart.
    pub unclassified: usize,
    pub text: String,
}

impl Ledger {
    /// The ledger of runs that ended as `endings` say.
    pub fn of(endings: impl IntoIterator<Item = Ending>) -> Ledger {
        let mut excluded: BTreeMap<Ending, usize> = Ending::ALL
            .into_iter()
            .filter(|ending| reading(*ending) == Reading::Excluded)
            .map(|ending| (ending, 0))
            .collect();
        let (mut judged, mut shipped, mut unclassified) = (0, 0, 0);
        for ending in endings {
            match reading(ending) {
                Reading::Outcome { shipped: ships } => {
                    judged += 1;
                    shipped += usize::from(ships);
                }
                Reading::Excluded => *excluded.entry(ending).or_default() += 1,
                Reading::Unclassified => unclassified += 1,
            }
        }
        let rate_percent = percent(shipped as i64, judged as i64);
        Ledger {
            text: text(judged, shipped, rate_percent, &excluded, unclassified),
            judged,
            shipped,
            rate_percent,
            excluded,
            unclassified,
        }
    }
}

/// The ledger in words: `2 of 5 shipped (40%) · 6 excluded: 1 blocked, …
/// · 1 unclassified`, each excluded ending named with its count, in
/// ruling 2's order.
fn text(
    judged: usize,
    shipped: usize,
    rate: Option<i64>,
    excluded: &BTreeMap<Ending, usize>,
    unclassified: usize,
) -> String {
    let rate = rate.map_or_else(|| ABSENT.to_string(), |rate| format!("{rate}%"));
    let named: Vec<String> = excluded
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(ending, count)| format!("{count} {}", ending.word()))
        .collect();
    let total: usize = excluded.values().sum();
    let excluded = match named.is_empty() {
        true => "0 excluded".to_string(),
        false => format!("{total} excluded: {}", named.join(", ")),
    };
    format!("{shipped} of {judged} shipped ({rate}) · {excluded} · {unclassified} unclassified")
}
