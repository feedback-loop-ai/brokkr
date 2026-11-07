//! What one run's journal and folded state say, read once into the facts
//! every figure is aggregated from, and the runs a query's realm keeps.
//! What the writer keeps of a run for them, brokkr-core reads
//! ([`aggregate`]), so the readout and the run's summary row read it one
//! way (decision 0089 ruling 7).

use std::collections::BTreeMap;

use brokkr_core::fold::{concluded, RunState};
use brokkr_core::summary::{aggregate, cut, parks as project_parks, MODEL_BYTES, NAME_BYTES};
use brokkr_core::EventEnvelope;

use super::{AnalyticsRun, QuarantinedHearth, Query};
use crate::attempts::{attempts, Attempts, Site};

pub(crate) use brokkr_core::summary::Finished;

/// A name a run's data states, cut where every figure groups it (decision
/// 0089 ruling 7): a phase, rule, provider, realm or recipe.
pub(super) fn name_key(text: &str) -> &str {
    cut(text, NAME_BYTES)
}

/// A model, cut where every figure groups it: the seat record's bound.
pub(super) fn model_key(text: &str) -> &str {
    cut(text, MODEL_BYTES)
}

/// An absent or stated name, cut the same way.
fn optional(text: Option<&str>) -> Option<&str> {
    text.map(name_key)
}

/// Whether a finished run completed: its ending implies `completed`
/// (decision 0064 ruling 6).
pub(crate) fn shipped(finished: &Finished) -> bool {
    finished.ending.status() == brokkr_core::fold::Status::Completed
}

/// A half-open span of time: its first moment held, its last not.
#[derive(Clone, Copy)]
pub(crate) struct Window {
    pub(crate) from: i64,
    pub(crate) to: i64,
}

impl Window {
    pub(crate) fn holds(self, at: i64) -> bool {
        at >= self.from && at < self.to
    }

    pub(super) fn overlap(self, start: i64, end: i64) -> i64 {
        (end.min(self.to) - start.max(self.from)).max(0)
    }
}

pub(super) struct Park {
    pub(super) phase: Option<String>,
    pub(super) at: i64,
    pub(super) answered: i64,
    pub(super) ended: i64,
}

/// Where a run's review rulings sent it: whether its first sent it back,
/// whether its second routed it on, and whether any sent it back.
pub(super) struct Reviews {
    pub(super) first: bool,
    pub(super) second: bool,
    pub(super) back: bool,
}

pub(super) struct RunFacts {
    pub(super) realm: String,
    /// The recipe the run started under, when its start names one.
    pub(super) recipe: Option<String>,
    readable: bool,
    /// Still running or parked: not concluded.
    pub(super) open: bool,
    pub(super) started: Option<i64>,
    pub(super) finished: Option<Finished>,
    pub(super) parks: Vec<Park>,
    pub(super) implement_visits: u64,
    pub(super) reviews: Reviews,
    pub(super) costs: Vec<(i64, f64)>,
    pub(super) sites: Vec<Site>,
}

/// One park a figure folds, with the realm of the run it belongs to and an
/// identity that tells one run from another in the same group.
pub(super) struct ParkFact<'a> {
    pub(super) realm: &'a str,
    pub(super) run: usize,
    pub(super) phase: Option<&'a str>,
    pub(super) at: i64,
    pub(super) answered: i64,
    pub(super) ended: i64,
}

/// One reported cost a figure folds.
pub(super) struct SpendFact<'a> {
    pub(super) realm: &'a str,
    pub(super) recipe: Option<&'a str>,
    pub(super) open: bool,
    pub(super) ended: Option<i64>,
    pub(super) at: i64,
    pub(super) usd: f64,
}

/// One seat site a figure folds.
pub(super) struct SiteFact<'a> {
    pub(super) realm: &'a str,
    pub(super) recipe: Option<&'a str>,
    pub(super) open: bool,
    pub(super) ended: Option<i64>,
    pub(super) site: &'a Site,
}

/// The facts every figure folds, one at a time: today the whole read, and
/// in unit U4 the store's rows. The seam spans four streams and the realm
/// scope, which must agree on the query and on a run's identity across
/// them, so it is one trait rather than four closures (decision 0071
/// ruling 2). Parks are yielded by their cut phase and then their run, so
/// `waiting_by_step` counts a change of run, never a set of them.
pub(super) trait FactSource {
    fn realms(&self) -> &[String];
    fn unreadable(&self) -> &BTreeMap<String, usize>;
    fn quarantined(&self) -> Vec<QuarantinedHearth>;
    fn runs(&self, visit: &mut dyn FnMut(&RunFacts));
    fn parks(&self, visit: &mut dyn FnMut(ParkFact<'_>));
    fn spends(&self, visit: &mut dyn FnMut(SpendFact<'_>));
    fn sites(&self, visit: &mut dyn FnMut(SiteFact<'_>));
}

/// The runs a range keeps: those that ended inside it, and those still
/// open. The one reading of "in range" the per-recipe figures and the
/// recipe's graph share.
pub(crate) fn in_range(finished: Option<i64>, open: bool, window: Window) -> bool {
    open || finished.is_some_and(|ended| window.holds(ended))
}

/// Whether the run is still running or parked: its fold has not
/// concluded it.
pub(crate) fn is_open(state: &RunState) -> bool {
    concluded(state).is_none()
}

/// Every park, its open moments read as `now`: it waits on the operator
/// until their next command, and stays parked until the run moves again.
/// Brokkr-core reads which moments those are (decision 0089 ruling 7).
fn parks(events: &[EventEnvelope], now: i64) -> Vec<Park> {
    project_parks(events)
        .into_iter()
        .map(|park| Park {
            phase: park.phase,
            at: park.at,
            answered: park.answered.or(park.ended).unwrap_or(now),
            ended: park.ended.unwrap_or(now),
        })
        .collect()
}

fn facts(run: &AnalyticsRun, label: &str, now: i64) -> RunFacts {
    let Attempts { costs, sites } = attempts(run.events);
    let read = aggregate(run.events, run.state);
    RunFacts {
        realm: read.realm.unwrap_or_else(|| label.to_string()),
        recipe: read.recipe,
        readable: run.state.is_some(),
        open: run.state.is_some_and(is_open),
        started: read.started_ms,
        finished: read.finished,
        parks: parks(run.events, now),
        implement_visits: read.implement_visits,
        reviews: Reviews {
            first: read.review_first,
            second: read.review_second,
            back: read.review_back,
        },
        costs: costs.iter().map(|spend| (spend.at, spend.usd)).collect(),
        sites,
    }
}

// ------------------------------------------------------------ the scope

/// The runs, realms and refusals the query's realm keeps.
#[derive(Default)]
pub(super) struct Scoped {
    pub(super) runs: Vec<RunFacts>,
    pub(super) realms: Vec<String>,
    pub(super) unreadable: BTreeMap<String, usize>,
    pub(super) quarantined: Vec<QuarantinedHearth>,
}

impl Scoped {
    pub(super) fn read(query: &Query, now: i64) -> Scoped {
        let mut scoped = Scoped::default();
        for hearth in query.hearths {
            let named: Vec<&str> = match hearth.realms.is_empty() {
                true => vec![hearth.label],
                false => hearth.realms.iter().map(String::as_str).collect(),
            };
            if let Some(refusal) = hearth.refusal {
                if named.iter().any(|realm| in_scope(query, realm)) {
                    scoped.quarantined.push(QuarantinedHearth {
                        realm: hearth.label.to_string(),
                        journal: hearth.journal.to_string(),
                        refusal: refusal.to_string(),
                    });
                }
                continue;
            }
            for realm in named {
                scoped.name(query, realm);
            }
            for run in hearth.runs {
                scoped.keep(query, facts(run, hearth.label, now));
            }
        }
        scoped
    }

    /// Names `realm` once, cut where a row groups it (decision 0089 ruling
    /// 7): two names that agree to their bound are one group.
    fn name(&mut self, query: &Query, realm: &str) {
        if !in_scope(query, realm) {
            return;
        }
        let realm = name_key(realm);
        if !self.realms.iter().any(|named| named == realm) {
            self.realms.push(realm.to_string());
        }
    }

    /// A run in the query's realm is kept; one whose journal does not
    /// load or fold is counted, and no figure reads it.
    fn keep(&mut self, query: &Query, run: RunFacts) {
        if !in_scope(query, &run.realm) {
            return;
        }
        self.name(query, &run.realm);
        match run.readable {
            true => self.runs.push(run),
            false => {
                *self
                    .unreadable
                    .entry(name_key(&run.realm).to_string())
                    .or_default() += 1;
            }
        }
    }
}

impl FactSource for Scoped {
    fn realms(&self) -> &[String] {
        &self.realms
    }

    fn unreadable(&self) -> &BTreeMap<String, usize> {
        &self.unreadable
    }

    fn quarantined(&self) -> Vec<QuarantinedHearth> {
        self.quarantined
            .iter()
            .map(|hearth| QuarantinedHearth {
                realm: hearth.realm.clone(),
                journal: hearth.journal.clone(),
                refusal: hearth.refusal.clone(),
            })
            .collect()
    }

    fn runs(&self, visit: &mut dyn FnMut(&RunFacts)) {
        for run in &self.runs {
            visit(run);
        }
    }

    fn parks(&self, visit: &mut dyn FnMut(ParkFact<'_>)) {
        let mut by_phase: BTreeMap<Option<&str>, Vec<(usize, &RunFacts, &Park)>> = BTreeMap::new();
        for (index, run) in self.runs.iter().enumerate() {
            for park in &run.parks {
                by_phase
                    .entry(optional(park.phase.as_deref()))
                    .or_default()
                    .push((index, run, park));
            }
        }
        for (phase, parks) in by_phase {
            for (run, facts, park) in parks {
                visit(ParkFact {
                    realm: &facts.realm,
                    run,
                    phase,
                    at: park.at,
                    answered: park.answered,
                    ended: park.ended,
                });
            }
        }
    }

    fn spends(&self, visit: &mut dyn FnMut(SpendFact<'_>)) {
        for run in &self.runs {
            let (open, ended) = (run.open, run.finished.as_ref().map(|finished| finished.at));
            for (at, usd) in &run.costs {
                visit(SpendFact {
                    realm: &run.realm,
                    recipe: run.recipe.as_deref(),
                    open,
                    ended,
                    at: *at,
                    usd: *usd,
                });
            }
        }
    }

    fn sites(&self, visit: &mut dyn FnMut(SiteFact<'_>)) {
        for run in &self.runs {
            let (open, ended) = (run.open, run.finished.as_ref().map(|finished| finished.at));
            for site in &run.sites {
                visit(SiteFact {
                    realm: &run.realm,
                    recipe: run.recipe.as_deref(),
                    open,
                    ended,
                    site,
                });
            }
        }
    }
}

fn in_scope(query: &Query, realm: &str) -> bool {
    query.realm.is_none_or(|wanted| wanted == realm)
}
