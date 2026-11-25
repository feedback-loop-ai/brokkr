//! Decision 0125 rulings 3, 4 and 7: `forge.host/v5` sizes entries by
//! what their recipes used. It is v4 with `runs` and `boxed_builds` each
//! optional (an absent boxed-build ceiling is none, ruling D) and one
//! optional `demand` block: the window of a recipe's measured runs and the
//! margin over their average (ruling B), and a demand a recipe declares by
//! name, read only while its window supplies no measured profile. Which
//! version may write what is judged here: a v1 to v4 file that writes
//! `demand`, or leaves out `boxed_builds`, refuses by its own line, and
//! reads with the defaults otherwise.

use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

use serde::Deserialize;
use thiserror::Error;

use super::shape::{self, Integral};
use super::{Ceiling, Schema};
use crate::capabilities::manifest::present;

/// The window a v1 to v4 file, or a v5 file that writes none, sizes by:
/// the recipe's last 20 measured runs (ruling B).
pub(crate) const WINDOW_RUNS: u32 = 20;
/// The margin over the average a file that writes none adds (ruling B).
pub(crate) const MARGIN_PERCENT: u32 = 25;
/// The largest window and margin the loader reads.
const MAX_WINDOW_RUNS: u32 = 1000;
const MAX_MARGIN_PERCENT: u32 = 1000;

/// How the machine sizes an entry by its recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Demand {
    pub(crate) window_runs: NonZeroU32,
    pub(crate) margin_percent: u32,
    pub(crate) recipes: BTreeMap<String, Declared>,
}

/// The expected peaks a recipe declares, in bytes and thousandths of a
/// core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Declared {
    #[serde(deserialize_with = "shape::positive_u64")]
    pub(crate) peak_memory_bytes: NonZeroU64,
    #[serde(deserialize_with = "shape::positive_u64")]
    pub(crate) peak_load_millicores: NonZeroU64,
}

/// The block as a v5 file writes it, each key optional.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrittenDemand {
    #[serde(default, deserialize_with = "present")]
    window_runs: Option<Integral<u32>>,
    #[serde(default, deserialize_with = "present")]
    margin_percent: Option<Integral<u32>>,
    #[serde(default, deserialize_with = "shape::strict_map")]
    recipes: BTreeMap<String, Declared>,
}

/// Why a file's demand block or boxed-build ceiling is not one the loader
/// can obey.
#[derive(Debug, Error)]
pub enum Problem {
    #[error("{0} declares no demand; only forge.host/v5 does")]
    NotV5(&'static str),
    #[error("{0} requires boxed_builds; only forge.host/v5 may leave it out")]
    Boxed(&'static str),
    #[error("demand.window_runs is {0}; it is a whole number from 1 to {MAX_WINDOW_RUNS}")]
    Window(u32),
    #[error("demand.margin_percent is {0}; it is a whole number from 0 to {MAX_MARGIN_PERCENT}")]
    Margin(u32),
}

impl Problem {
    /// The contract the refusal names: the file's own version for a key
    /// it may not write or leave out, and v5 for a number out of range.
    pub(super) fn contract(&self) -> &'static str {
        match self {
            Problem::NotV5(declared) | Problem::Boxed(declared) => declared,
            Problem::Window(_) | Problem::Margin(_) => Schema::V5.contract(),
        }
    }
}

/// The demand block a file of `schema` declares in `written`: v5's, each
/// key it leaves out the default, and the defaults for an earlier version,
/// which must write none.
pub(super) fn read(schema: Schema, written: Option<WrittenDemand>) -> Result<Demand, Problem> {
    let written = match (schema, written) {
        (Schema::V5, written) => written.unwrap_or_default(),
        (Schema::V1 | Schema::V2 | Schema::V3 | Schema::V4, None) => WrittenDemand::default(),
        (older @ (Schema::V1 | Schema::V2 | Schema::V3 | Schema::V4), Some(_)) => {
            return Err(Problem::NotV5(older.contract()))
        }
    };
    let window = written
        .window_runs
        .map_or(WINDOW_RUNS, |Integral(runs)| runs);
    let window_runs = NonZeroU32::new(window)
        .filter(|runs| runs.get() <= MAX_WINDOW_RUNS)
        .ok_or(Problem::Window(window))?;
    let margin_percent = written
        .margin_percent
        .map_or(MARGIN_PERCENT, |Integral(at)| at);
    if margin_percent > MAX_MARGIN_PERCENT {
        return Err(Problem::Margin(margin_percent));
    }
    Ok(Demand {
        window_runs,
        margin_percent,
        recipes: written.recipes,
    })
}

/// The boxed-build ceiling a file of `schema` declares: optional in v5
/// (ruling D), required before it.
pub(super) fn boxed(schema: Schema, written: Option<Ceiling>) -> Result<Option<Ceiling>, Problem> {
    match (schema, written) {
        (Schema::V5, written) => Ok(written),
        (Schema::V1 | Schema::V2 | Schema::V3 | Schema::V4, Some(ceiling)) => Ok(Some(ceiling)),
        (older @ (Schema::V1 | Schema::V2 | Schema::V3 | Schema::V4), None) => {
            Err(Problem::Boxed(older.contract()))
        }
    }
}

#[cfg(test)]
mod tests;
