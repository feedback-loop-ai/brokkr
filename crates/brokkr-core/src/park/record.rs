//! A typed refusal record read into its park-cause variant. The record
//! (`ProviderRefusal`) is public and holds its route and lengths as options,
//! so a hand-built one can contradict its own cause; the reader of a journal
//! never yields one (`provider_refusal` requires each variant's fields), but
//! this edge does not trust that. An inconsistent record is a typed error,
//! never a panic at a value boundary (decision 0071 rulings 3 and 8).

use thiserror::Error;
use time::OffsetDateTime;

use crate::{ProviderRefusal, RefusalCause};

use super::ProviderLimitRefusal;

/// A refusal record that lacks a fact its own cause requires.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("a {cause} refusal records no {lacks}")]
pub(crate) struct InconsistentRefusal {
    pub cause: &'static str,
    pub lacks: Lacks,
}

/// The fact a refusal's cause requires and its record does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lacks {
    Route,
    Seconds,
    Maximum,
}

impl std::fmt::Display for Lacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Lacks::Route => "route",
            Lacks::Seconds => "seconds",
            Lacks::Maximum => "maximum",
        })
    }
}

/// The record's cause as the contract spells it.
fn word(cause: RefusalCause) -> &'static str {
    match cause {
        RefusalCause::RouteUnresolved => "route-unresolved",
        RefusalCause::HostUnreadable => "host-unreadable",
        RefusalCause::EpisodeUnreadable => "episode-unreadable",
        RefusalCause::PastMaximum => "past-maximum",
        RefusalCause::NoEnd => "no-end",
    }
}

/// The one variant `refused` is, carrying exactly the fields it has. `until`
/// is the computed end a `past-maximum` refusal records in its wait; a
/// past-maximum record whose wait did not read has no end to carry, and the
/// representable variant for that case is `no-end`, which states the same
/// computed length with no `until`.
pub(super) fn variant(
    refused: &ProviderRefusal,
    until: Option<OffsetDateTime>,
) -> Result<ProviderLimitRefusal, InconsistentRefusal> {
    let cause = word(refused.cause);
    let route = || {
        refused.route.clone().ok_or(InconsistentRefusal {
            cause,
            lacks: Lacks::Route,
        })
    };
    let lengths = || {
        let seconds = refused.seconds.ok_or(InconsistentRefusal {
            cause,
            lacks: Lacks::Seconds,
        })?;
        let max_seconds = refused.max_seconds.ok_or(InconsistentRefusal {
            cause,
            lacks: Lacks::Maximum,
        })?;
        Ok((seconds, max_seconds))
    };
    Ok(match (refused.cause, until) {
        (RefusalCause::RouteUnresolved, _) => ProviderLimitRefusal::RouteUnresolved,
        (RefusalCause::HostUnreadable, _) => {
            ProviderLimitRefusal::HostUnreadable { route: route()? }
        }
        (RefusalCause::EpisodeUnreadable, _) => {
            ProviderLimitRefusal::EpisodeUnreadable { route: route()? }
        }
        (RefusalCause::PastMaximum, Some(until)) => {
            let (seconds, max_seconds) = lengths()?;
            ProviderLimitRefusal::PastMaximum {
                route: route()?,
                until,
                seconds,
                max_seconds,
            }
        }
        (RefusalCause::PastMaximum, None) | (RefusalCause::NoEnd, _) => {
            let (seconds, max_seconds) = lengths()?;
            ProviderLimitRefusal::NoEnd {
                route: route()?,
                seconds,
                max_seconds,
            }
        }
    })
}
