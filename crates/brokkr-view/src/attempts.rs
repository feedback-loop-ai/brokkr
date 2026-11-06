//! Every attempt's seats as the journal records them (decision 0016): the
//! sites an agent resolution chose, with the provider each ran on, the
//! model it reported, and each checkpoint it metered — its time and phase,
//! its reported cost and its tokens; and every reported cost with its time
//! and phase. The sites are brokkr-core's one reading (decision 0089
//! ruling 7), which the writer keeps and the analytics readout and the
//! recipe's graph share (decision 0071 ruling 5).

use crate::meter::{sum_usage, Metering};
use crate::TokenUsage;
use brokkr_core::summary::{checkpoints, sites, Checkpoint, Site as Read};
use brokkr_core::EventEnvelope;

/// One invocation site of one attempt that an agent resolution chose.
pub(crate) struct Site {
    pub(crate) at: i64,
    /// The phase the run was in when the attempt started.
    pub(crate) phase: Option<String>,
    pub(crate) provider: String,
    pub(crate) model: Option<String>,
    metered: Vec<Checkpoint>,
    by: Metering,
}

/// Which moments a figure keeps, by a time and the phase the run was in
/// then: the window alone, or the window and one state.
pub(crate) type Keeps<'a> = &'a dyn Fn(i64, Option<&str>) -> bool;

impl Site {
    /// The site brokkr-core read, as this readout queries it.
    fn of(read: Read) -> Site {
        Site {
            at: read.at,
            phase: read.phase,
            provider: read.provider,
            model: read.model,
            metered: read.metered,
            by: read.metering,
        }
    }

    /// Whether the site ran in what `keeps` holds: it started there, or
    /// metered a checkpoint there.
    pub(crate) fn ran(&self, keeps: Keeps) -> bool {
        keeps(self.at, self.phase.as_deref()) || self.kept(keeps).next().is_some()
    }

    /// Whether a checkpoint `keeps` holds reported a cost.
    pub(crate) fn reported(&self, keeps: Keeps) -> bool {
        self.kept(keeps).any(|metered| metered.reported)
    }

    /// What the checkpoints `keeps` holds metered, by the rule
    /// [`Metering`] states.
    pub(crate) fn usage(&self, keeps: Keeps) -> Option<TokenUsage> {
        let counted = self.kept(keeps);
        sum_usage(counted.filter_map(|metered| metered.meter.counted(self.by, metered.session)))
    }

    fn kept<'a>(&'a self, keeps: Keeps<'a>) -> impl Iterator<Item = &'a Checkpoint> + 'a {
        let metered = self.metered.iter();
        metered.filter(move |metered| keeps(metered.at, metered.phase.as_deref()))
    }
}

/// One reported cost: a checkpoint record's, as `brokkr costs` sums it.
pub(crate) struct Spend {
    pub(crate) at: i64,
    pub(crate) phase: Option<String>,
    pub(crate) usd: f64,
}

/// What one run's attempts recorded.
pub(crate) struct Attempts {
    pub(crate) costs: Vec<Spend>,
    pub(crate) sites: Vec<Site>,
}

/// Every site and reported cost of one run's journal.
pub(crate) fn attempts(events: &[EventEnvelope]) -> Attempts {
    let costs = checkpoints(events)
        .into_iter()
        .filter_map(|checkpoint| {
            let usd = checkpoint.usd?;
            Some(Spend {
                at: checkpoint.at,
                phase: checkpoint.phase,
                usd,
            })
        })
        .collect();
    Attempts {
        costs,
        sites: sites(events).into_iter().map(Site::of).collect(),
    }
}
