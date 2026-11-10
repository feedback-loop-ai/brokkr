//! A retirement a later evidence-protecting admission supersedes
//! (decision 0105, the controller's H1 ruling on #1020): the retirement
//! covered the evidence its root held when it was retired, not the root
//! forever. The admission that protects evidence there again seals the
//! retirement into the root's history as `superseded-<key>-<n>.json` —
//! every field of the retirement plus `superseded_at` and the superseding
//! `run` — and only then removes `retired-<key>.json`. A crash between
//! the two leaves both, and the next admission finishes the move without
//! a second copy of the same retirement. No record is ever deleted, so an
//! archive's location stays on record.

use rustix::fs::{fsync, unlinkat, AtFlags};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::key;
use super::retired::{self, Retirement};
use crate::engine::artifacts::hex;
use crate::engine::broker::admission::{Claim, Unknown};
use crate::engine::broker::inventory::{Budget, HistoricalRoot};
use crate::engine::broker::root::{Commit, Root};

/// What a superseded retirement's record begins with.
const PREFIX: &str = "superseded-";

/// What a superseded retirement's record ends with.
const SUFFIX: &str = ".json";

/// One retirement kept as history, with when and by whose run it was
/// superseded. It nests the closed retirement beside those two facts, so
/// every level of the record refuses a field it does not know (decision
/// 0071 ruling 3; 0105, the controller's S7 ruling on #1020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Superseded {
    /// The retirement this record keeps, every one of its fields.
    pub retirement: Retirement,
    /// When the admission that superseded it ran, RFC 3339.
    pub superseded_at: String,
    /// The run whose evidence-protecting admission superseded it.
    pub run: String,
}

impl Superseded {
    /// `retirement` kept as history by `run`'s admission at
    /// `superseded_at`.
    pub fn of(retirement: &Retirement, run: &str, superseded_at: &str) -> Superseded {
        Superseded {
            retirement: retirement.clone(),
            superseded_at: superseded_at.to_string(),
            run: run.to_string(),
        }
    }

    /// This record in the operator's words: the retirement's own words,
    /// with when and by whose run it was superseded. A surface escapes it
    /// its own way.
    pub fn words(&self) -> String {
        format!(
            "superseded on {} by run '{}': {}",
            self.superseded_at,
            self.run,
            self.retirement.words()
        )
    }
}

/// The record's name for the historical record's key `key` and its
/// sequence `n`.
pub(in crate::engine::broker) fn name(key: &str, n: u64) -> String {
    format!("{PREFIX}{key}-{n}{SUFFIX}")
}

/// The key and sequence a superseded record's name `file` carries, where
/// it is one.
pub(in crate::engine::broker) fn named(file: &str) -> Option<(&str, u64)> {
    let rest = file.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let (key, n) = rest.rsplit_once('-')?;
    hex(key).then_some(())?;
    let n: u64 = n.parse().ok().filter(|n| *n >= 1)?;
    // Only the one spelling `name` makes is a record: `-01` and `+1` parse
    // as 1 but do not round-trip (decision 0071 ruling 3).
    (name(key, n) == file).then_some((key, n))
}

/// Supersede the retirement of `claim`'s artifact root, where one stands
/// and the claim protects evidence: seal it into the root's history, then
/// remove the retirement, in that order and in this locked step (H1).
/// `Commit` is the history seal's; nothing where no retirement stands, the
/// claim protects nothing, or a crash already sealed this very retirement.
pub(in crate::engine::broker) fn superseded(
    root: &Root,
    historical: &mut [HistoricalRoot],
    claim: &Claim,
    budget: &Budget,
) -> Result<Commit, Unknown> {
    if !claim.protects() {
        return Ok(Commit::Synced);
    }
    let Some(held) = historical
        .iter_mut()
        .find(|held| held.artifacts == claim.artifacts)
    else {
        return Ok(Commit::Synced);
    };
    let Some(retirement) = held.retirement.clone() else {
        return Ok(Commit::Synced);
    };
    let repo = root.below(&[&held.repo])?;
    let key = key(&retirement.artifacts);
    let next = match held
        .history
        .iter()
        .find(|(_, record)| record.retirement == retirement)
    {
        Some(_) => None,
        None => {
            let last = held.history.iter().map(|(n, _)| *n).max().unwrap_or(0);
            Some(last.checked_add(1).ok_or(Unknown::Overflow)?)
        }
    };
    let commit = match next {
        Some(n) => {
            let at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default();
            let record = Superseded::of(&retirement, &claim.run, &at);
            let bytes = serde_json::to_vec(&record).or(Err(Unknown::Unsealed))?;
            root.seal(&repo, &name(&key, n), &bytes, budget)?
        }
        None => Commit::Synced,
    };
    // Only once the retirement is on record as history is the one standing
    // removed; a crash between the two leaves both for the next admission.
    unlinkat(&repo, retired::name(&key).as_str(), AtFlags::empty()).or(Err(Unknown::Unsealed))?;
    fsync(&repo).or(Err(Unknown::Unsealed))?;
    held.retirement = None;
    Ok(commit)
}
