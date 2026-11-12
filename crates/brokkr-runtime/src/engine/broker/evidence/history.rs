//! The retirements of one historical root as a readout reads them
//! (decision 0105 rulings 3(a) and 5, H1): the one that stands now, and
//! the history later evidence-protecting admissions superseded. The
//! readout that names a retained response reads the one that covered the
//! response's own recorded time.

use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use super::super::admission::beside;
use super::super::historical::retired::Retirement;
use super::super::historical::superseded::Superseded;
use super::super::inventory::{inventory, Budget};
use super::super::root::{locked_existing, Root};
use super::{find, EvidenceError};

/// Every retirement of one historical root: the one that stands, if any,
/// and the history later admissions superseded (decision 0105, H1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RetirementHistory {
    pub current: Option<Retirement>,
    pub superseded: Vec<Superseded>,
}

impl RetirementHistory {
    /// The retirement that covered evidence the journal recorded at
    /// `retained_at`: the earliest retirement sealed at or after it,
    /// whether that retirement is current or superseded. None where the
    /// evidence was retained after every retirement, and the current
    /// retirement where `retained_at` is not an RFC 3339 time, which a
    /// readout cannot compare and so reads as before this rule.
    pub fn covering(&self, retained_at: &str) -> Option<Retirement> {
        let Ok(retained) = time::OffsetDateTime::parse(
            retained_at,
            &time::format_description::well_known::Rfc3339,
        ) else {
            return self.current.clone();
        };
        let sealed = |retirement: &&Retirement| {
            time::OffsetDateTime::parse(
                &retirement.retired_at,
                &time::format_description::well_known::Rfc3339,
            )
            .is_ok_and(|at| at >= retained)
        };
        let all: Vec<Retirement> = self
            .current
            .iter()
            .cloned()
            .chain(
                self.superseded
                    .iter()
                    .map(|record| record.retirement.clone()),
            )
            .collect();
        all.iter()
            .filter(sealed)
            .min_by_key(|retirement| {
                time::OffsetDateTime::parse(
                    &retirement.retired_at,
                    &time::format_description::well_known::Rfc3339,
                )
                .ok()
            })
            .cloned()
    }
}

/// Every retirement of the historical root `artifacts`, the one that
/// stands and the history later admissions superseded, read under the
/// admission lock; empty where no historical record names it. It writes
/// nothing, and a coordination root that does not stand holds no
/// retirement.
pub fn retirements(
    home: &Path,
    artifacts: &Path,
    deadline: Instant,
) -> Result<RetirementHistory, EvidenceError> {
    match super::existing(home)? {
        Some(root) => retirements_in(&root, artifacts, deadline),
        None => Ok(RetirementHistory::default()),
    }
}

/// [`retirements`] in an already opened coordination root.
#[cfg(test)]
pub(in crate::engine) fn retirement_in(
    root: &Root,
    artifacts: &Path,
    deadline: Instant,
) -> Result<Option<Retirement>, EvidenceError> {
    Ok(retirements_in(root, artifacts, deadline)?.current)
}

/// [`retirements`] in an already opened coordination root.
pub(in crate::engine) fn retirements_in(
    root: &Root,
    artifacts: &Path,
    deadline: Instant,
) -> Result<RetirementHistory, EvidenceError> {
    let Some(_lock) = locked_existing(root.dir(), deadline)? else {
        return Ok(RetirementHistory::default());
    };
    let mut budget = Budget::new(deadline);
    let found = inventory(root.dir(), root.path(), ["", ""], &mut budget)?;
    // Admission records the artifact root as `beside` resolves it; a repo
    // named through a link is the same root, so the lookup takes the same
    // resolution (decision 0105, the controller's R2 ruling on #1020).
    let held = find(&found, &beside(artifacts));
    Ok(RetirementHistory {
        current: held.and_then(|held| held.retirement.clone()),
        superseded: held
            .map(|held| {
                held.history
                    .iter()
                    .map(|(_, record)| record.clone())
                    .collect()
            })
            .unwrap_or_default(),
    })
}
