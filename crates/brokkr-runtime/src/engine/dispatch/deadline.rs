//! Decision 0104 ruling 2: the deadline an attempt runs under is the
//! larger of the seat's own `timeout_seconds` and the highest floor among
//! the routes of the candidates THIS attempt launches. The floor is read
//! from the host file the launch named, again at each attempt, and only a
//! `shared-local` route carries one. A chain's cloud link, a probe and an
//! attempt whose host file cannot be read keep the seat's own limit; a
//! panel or sequence takes its launched sites' highest floor because it
//! hands one deadline to every member or step.
//!
//! The floor never shortens an authored limit, and a file that cannot be
//! read or is invalid only raises the attempt's words, never its bound: a
//! floor that cannot be read cannot shorten a deadline either.
//!
//! A panel's and a sequence's terminal words name the same source too. The
//! panel names its joined outcome once, before it is journaled, from
//! whether any member was deadline-killed; a sequence names each step's
//! outcome where it is built, and an ending built without one (a missing
//! or malformed change, a contradictory result, a result outside its
//! step's vocabulary, a refused record) names an unreadable or invalid
//! host itself. Only the step that ends the attempt reaches its terminal
//! words, so each attempt's words name the deadline's source once, however
//! many members or steps it ran.

use std::collections::BTreeMap;
use std::fmt;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use brokkr_protocol::{AttemptOutcome, AttemptReport};

use super::super::{site_label, DriverRun, Selection};
use crate::admission::{read_host, HostError, Hosting};
use crate::agents::Candidate;
use crate::bundle::SiteFacts;
use crate::capabilities::manifest::ModelPins;

/// One attempt's deadline, and why it is longer than the seat's own where
/// it is: the raised floor, or the host file that could not be read.
pub(in crate::engine) struct Deadline {
    pub(super) seconds: u64,
    pub(super) cause: Option<Cause>,
}

/// What one attempt's body runs on: the effective `Deadline` for its
/// words beside what provisioning left of it (`remaining`) for its bound.
/// A panel's or a sequence's members and steps all run on `remaining`; the
/// deadline names the source once, at the joined outcome or the ending
/// step.
#[derive(Clone, Copy)]
pub(in crate::engine) struct AttemptBound<'a> {
    pub(in crate::engine) deadline: &'a Deadline,
    pub(in crate::engine) remaining: Duration,
}

impl<'a> AttemptBound<'a> {
    pub(in crate::engine) fn new(deadline: &'a Deadline, remaining: Duration) -> Self {
        Self {
            deadline,
            remaining,
        }
    }
}

/// Where an attempt's deadline came from, when its own limit did not
/// govern it. Every variant is wording, never a bound: the seat's own
/// number is the deadline in the unreadable and invalid cases.
pub(super) enum Cause {
    Raised {
        provider: String,
        route: String,
        authored: u64,
    },
    Unreadable {
        path: PathBuf,
        kind: ErrorKind,
    },
    Invalid {
        error: HostError,
    },
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cause::Raised {
                provider,
                route,
                authored,
            } => write!(
                f,
                "the shared-local floor host.json declares for {provider}/{route}; the seat's \
                 own limit is {authored} seconds"
            ),
            Cause::Unreadable { path, kind } => {
                write!(
                    f,
                    "the host configuration at {} cannot be read: {kind}",
                    path.display()
                )
            }
            Cause::Invalid { error } => match error {
                HostError::Invalid { source, .. } => write!(f, "{error}: {source}"),
                HostError::Unplaced => write!(f, "{error}"),
            },
        }
    }
}

/// Which surface's words an outcome takes: a single attempt replaces the
/// protocol's bare number, while a panel keeps its joined member list and
/// appends the deadline to it.
#[derive(Clone, Copy)]
enum OutcomeWords {
    Single,
    Panel,
}

impl Deadline {
    /// The attempt's deadline: `authored` unless a shared-local route this
    /// attempt launches declares a higher floor.
    pub(super) fn of(
        host: Option<&Path>,
        sites: &BTreeMap<String, SiteFacts>,
        site_name: &str,
        selection: &Selection,
        authored: u64,
    ) -> Deadline {
        let Some(file) = host else {
            return Deadline::seat(authored);
        };
        match read_host(file) {
            Ok(Hosting::Absent) => Deadline::seat(authored),
            Ok(Hosting::Unreadable(kind)) => Deadline {
                seconds: authored,
                cause: Some(Cause::Unreadable {
                    path: file.to_path_buf(),
                    kind,
                }),
            },
            Ok(Hosting::Declared(config)) => {
                let mut highest: Option<(u32, String, String)> = None;
                for (site, candidate) in &selection.candidates {
                    let Some(route) = route_of(sites, site_name, site, candidate) else {
                        continue;
                    };
                    let Some(floor) = config.timeout_floor(&candidate.provider, &route) else {
                        continue;
                    };
                    let higher = u64::from(floor) > authored
                        && highest.as_ref().is_none_or(|(best, _, _)| floor > *best);
                    if higher {
                        highest = Some((floor, candidate.provider.clone(), route));
                    }
                }
                match highest {
                    Some((floor, provider, route)) => Deadline {
                        seconds: u64::from(floor),
                        cause: Some(Cause::Raised {
                            provider,
                            route,
                            authored,
                        }),
                    },
                    None => Deadline::seat(authored),
                }
            }
            Err(error) => Deadline {
                seconds: authored,
                cause: Some(Cause::Invalid { error }),
            },
        }
    }

    pub(in crate::engine) fn seat(authored: u64) -> Deadline {
        Deadline {
            seconds: authored,
            cause: None,
        }
    }

    /// The attempt's bound.
    pub(super) fn bound(&self) -> Duration {
        Duration::from_secs(self.seconds)
    }

    /// The attempt's deadline in whole seconds.
    #[cfg(test)]
    pub(super) fn seconds(&self) -> u64 {
        self.seconds
    }

    /// The source of the deadline, where the seat's own limit did not
    /// govern it.
    #[cfg(test)]
    pub(super) fn explanation(&self) -> Option<String> {
        self.cause.as_ref().map(ToString::to_string)
    }

    /// Name the deadline in a single attempt's own words. A raised deadline
    /// the watchdog ended keeps the protocol's facts and replaces its bare
    /// number with both numbers and the source (decision 0104 ruling
    /// 3(a)); a host file that could not be read or was invalid names
    /// itself however the attempt ended, because it is why the seat's own
    /// limit governed. Anything else reads exactly as before. A panel and a
    /// sequence name the source once per attempt: the panel through
    /// [`Self::name_joined_outcome`] at its joined outcome, a sequence's
    /// step through this method where its outcome is built, and an ending
    /// that builds no outcome through [`Self::name_words`].
    pub(in crate::engine) fn name(&self, run: &mut DriverRun) {
        let Some(cause) = &self.cause else {
            return;
        };
        match run {
            DriverRun::SpawnFailed(error) => {
                if !matches!(cause, Cause::Raised { .. }) {
                    *error = format!("{error}; {cause}");
                }
            }
            DriverRun::Ran(report) => {
                self.name_outcome(
                    &mut report.outcome,
                    report.deadline_killed,
                    OutcomeWords::Single,
                );
            }
        }
    }

    /// Name the deadline in an attempt outcome, from whether the watchdog
    /// ended it and which surface's words the outcome takes.
    fn name_outcome(&self, outcome: &mut AttemptOutcome, killed: bool, words: OutcomeWords) {
        let Some(cause) = &self.cause else {
            return;
        };
        match cause {
            Cause::Raised { .. } => {
                if !killed {
                    return;
                }
                if let AttemptOutcome::Indeterminate { reason } = outcome {
                    let raised = self.raised(cause);
                    *reason = match words {
                        OutcomeWords::Panel => format!("{reason}: {raised}"),
                        OutcomeWords::Single => {
                            format!(
                                "{raised}; its process tree is gone, and what it did is unknown"
                            )
                        }
                    };
                }
            }
            Cause::Unreadable { .. } | Cause::Invalid { .. } => {
                append_cause(outcome, cause);
            }
        }
    }

    /// Name the deadline in a panel's joined outcome, seat-level or a
    /// sequence's panel step: once, keeping the member list a raised
    /// deadline's indeterminate reason already carries. A cause that is
    /// not a raised deadline is appended as it is to a single attempt.
    pub(in crate::engine) fn name_joined_outcome(
        &self,
        outcome: &mut AttemptOutcome,
        killed: bool,
    ) {
        self.name_outcome(outcome, killed, OutcomeWords::Panel);
    }

    /// The one namer a panel's joined outcome uses: the watchdog's fact is
    /// read from `reports` now, so the outcome can be built after they are
    /// consumed and then named once. A seat-level panel and a sequence's
    /// panel step share it, so neither can name the deadline differently.
    pub(in crate::engine) fn panel_namer<'a>(
        &'a self,
        reports: &[(String, AttemptReport)],
    ) -> impl FnOnce(&mut AttemptOutcome) + 'a {
        let killed = reports.iter().any(|(_, report)| report.deadline_killed);
        move |outcome| self.name_joined_outcome(outcome, killed)
    }

    /// Name an unreadable or invalid host in terminal words a sequence
    /// builds without an `AttemptOutcome`: an ending that never formed a
    /// step's outcome. A raised deadline names nothing here, since only a
    /// watchdog kill raises a raised deadline's words.
    pub(in crate::engine) fn name_words(&self, words: &mut String) {
        if let Some(cause) = self.unreadable() {
            *words = format!("{words}; {cause}");
        }
    }

    /// The host file's cause, where that is what the deadline carries: a
    /// raised floor names itself only in an attempt the watchdog ended.
    fn unreadable(&self) -> Option<&Cause> {
        match self.cause.as_ref()? {
            cause @ (Cause::Unreadable { .. } | Cause::Invalid { .. }) => Some(cause),
            Cause::Raised { .. } => None,
        }
    }

    /// The raised deadline's own words: both numbers and the source.
    fn raised(&self, cause: &Cause) -> String {
        format!("the seat's {}-second deadline: {cause}", self.seconds)
    }
}

/// Append a host file's cause to a failed or indeterminate attempt's
/// words, once. A success's words are not failure words and are left
/// alone.
fn append_cause(outcome: &mut AttemptOutcome, cause: &Cause) {
    match outcome {
        AttemptOutcome::Failed { error } => *error = format!("{error}; {cause}"),
        AttemptOutcome::Indeterminate { reason } => *reason = format!("{reason}; {cause}"),
        AttemptOutcome::Succeeded { .. } => {}
    }
}

/// The route a launched candidate's driver serves it on, from the site's
/// sealed model pins: the candidate's own link, never another chain
/// entry's (decision 0104 ruling 2).
fn route_of(
    sites: &BTreeMap<String, SiteFacts>,
    site_name: &str,
    site: &Option<String>,
    candidate: &Candidate,
) -> Option<String> {
    let facts = sites.get(&site_label(site_name, site))?;
    let index = facts.chain.iter().position(|link| link == candidate)?;
    match facts.pins.get(index)? {
        ModelPins::Read(served) => served.iter().find_map(|served| served.route.clone()),
        ModelPins::Unreadable(_) => None,
    }
}
