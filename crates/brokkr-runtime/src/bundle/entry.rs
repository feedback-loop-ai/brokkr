//! Run-level admission; no recipe identity moves (decision 0094, proposed).
use std::collections::BTreeSet;
use std::fmt;

use brokkr_core::entry::{is_entry_name, RunEntry};
use brokkr_core::policy::{Direction, Machine, Rule};
use thiserror::Error;

use super::{bounded_site, Bundle, GateAssessment, SeatBody, StepBody, WorkSite};
use crate::phase_name::{CHANGE_SOURCES, TRIAGE};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("cannot enter '{phase}': {refusal}; admitted entries: {}", .admitted.join(", "))]
pub struct EntryError {
    pub phase: String,
    pub refusal: Box<EntryRefusal>,
    pub admitted: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRefusal {
    Absent,
    Terminal,
    Unprintable,
    Work {
        site: WorkSite,
    },
    AvoidsProtected {
        protected: String,
        terminal: String,
    },
    ForwardWork {
        work: String,
        rule: String,
    },
    NeedsStrategy {
        seat: String,
        reached_by: Option<String>,
    },
    NeedsStrategyRuling {
        group: (String, String),
        reached_by: Option<String>,
    },
    NeedsVisits {
        group: (String, String),
        phase: String,
        reached_by: Option<String>,
    },
    NeedsChange {
        step: String,
        sources: Vec<String>,
        reached_by: Option<String>,
    },
}

impl fmt::Display for EntryRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => f.write_str("phase is absent from the table"),
            Self::Terminal => f.write_str("phase is terminal, with no seat"),
            Self::Unprintable => f.write_str("phase is outside the bounded entry-name grammar"),
            Self::Work { site } => write!(
                f,
                "work site {} (compiled: {})",
                bounded_site(&site.path.join(":")),
                site.compiled
            ),
            Self::AvoidsProtected {
                protected,
                terminal,
            } => write!(
                f,
                "terminal '{terminal}' is reachable without protected phase '{protected}'"
            ),
            Self::ForwardWork { work, rule } => {
                write!(f, "forward rule '{rule}' reaches work phase '{work}'")
            }
            Self::NeedsStrategy { seat, reached_by } => write!(
                f,
                "seat '{seat}' needs strategy, reached by {}",
                witness(reached_by)
            ),
            Self::NeedsStrategyRuling {
                group: (phase, result),
                reached_by,
            } => write!(
                f,
                "ruling group ({phase}, {result}) needs strategy, reached by {}",
                witness(reached_by)
            ),
            Self::NeedsVisits {
                group: (from, result),
                phase,
                reached_by,
            } => write!(
                f,
                "ruling group ({from}, {result}) needs the visits of '{phase}', reached by {}",
                witness(reached_by)
            ),
            Self::NeedsChange {
                step,
                sources,
                reached_by,
            } => write!(
                f,
                "step '{step}' needs change from [{}], reached by {}",
                sources.join(", "),
                witness(reached_by)
            ),
        }
    }
}

fn witness(rule: &Option<String>) -> &str {
    rule.as_deref().unwrap_or("entry itself")
}

pub fn admit(bundle: &Bundle, phase: &str) -> Result<(), EntryError> {
    match refusal(bundle, phase) {
        None => Ok(()),
        Some(refusal) => Err(EntryError {
            phase: phase.into(),
            refusal: Box::new(refusal),
            admitted: admitted(bundle),
        }),
    }
}

pub fn admitted(bundle: &Bundle) -> Vec<String> {
    bundle
        .machine
        .phases
        .iter()
        .filter(|phase| refusal(bundle, phase).is_none())
        .cloned()
        .collect()
}

pub(crate) fn first_phase<'a>(
    bundle: &'a Bundle,
    entry: Option<&'a RunEntry>,
) -> Result<&'a str, EntryError> {
    match entry {
        None => Ok(&bundle.machine.initial),
        Some(entry) => {
            admit(bundle, &entry.phase)?;
            Ok(&entry.phase)
        }
    }
}

fn refusal(bundle: &Bundle, phase: &str) -> Option<EntryRefusal> {
    let machine = &bundle.machine;
    if !machine.phases.iter().any(|name| name == phase) {
        return Some(EntryRefusal::Absent);
    }
    if machine.terminal.iter().any(|name| name == phase) {
        return Some(EntryRefusal::Terminal);
    }
    if !is_entry_name(phase) {
        return Some(EntryRefusal::Unprintable);
    }
    if let GateAssessment::ContainsWork(site) = &bundle.seats[phase].gate {
        return Some(EntryRefusal::Work { site: site.clone() });
    }
    protected(bundle, phase)
        .or_else(|| forward_work(bundle, phase))
        .or_else(|| strategy(bundle, phase))
        .or_else(|| change(bundle, phase))
        .or_else(|| visits(bundle, phase))
}

fn protected(bundle: &Bundle, phase: &str) -> Option<EntryRefusal> {
    let protected = &bundle.protected_phase;
    if phase == protected {
        return None;
    }
    super::phases::bypassing_terminal(&bundle.machine, phase, protected).map(|terminal| {
        EntryRefusal::AvoidsProtected {
            protected: protected.clone(),
            terminal: terminal.into(),
        }
    })
}

fn forward_work(bundle: &Bundle, phase: &str) -> Option<EntryRefusal> {
    let machine = &bundle.machine;
    walk(
        machine,
        phase,
        |rule| {
            rule.next.as_deref().is_some_and(|next| {
                Direction::of(&machine.phases, &rule.from, next) == Some(Direction::Forward)
            })
        },
        &[],
    )
    .into_iter()
    .find_map(|(work, rule)| {
        bundle
            .seats
            .get(work)
            .filter(|seat| !seat.has_gate())
            .map(|_| EntryRefusal::ForwardWork {
                work: work.into(),
                rule: rule.unwrap_or("").into(),
            })
    })
}

fn strategy(bundle: &Bundle, phase: &str) -> Option<EntryRefusal> {
    if phase == TRIAGE {
        return None;
    }
    let reached = walk(&bundle.machine, phase, |_| true, &[TRIAGE]);
    for (seat, rule) in &reached {
        if matches!(
            bundle.seats.get(*seat).map(|seat| &seat.body),
            Some(SeatBody::Select { default: None, .. })
        ) {
            return Some(EntryRefusal::NeedsStrategy {
                seat: (*seat).into(),
                reached_by: rule.map(str::to_string),
            });
        }
    }
    let groups = bundle.machine.groups_reading_strategy();
    for (seat, rule) in reached {
        if let Some(&(phase, result)) = groups.iter().find(|(phase, _)| *phase == seat) {
            return Some(EntryRefusal::NeedsStrategyRuling {
                group: (phase.into(), result.into()),
                reached_by: rule.map(str::to_string),
            });
        }
    }
    None
}

fn visits(bundle: &Bundle, phase: &str) -> Option<EntryRefusal> {
    let met = walk(&bundle.machine, phase, |_| true, &[]);
    let reached: BTreeSet<_> = walk(
        &bundle.machine,
        phase,
        |rule| rule.reads_visits().is_empty(),
        &[],
    )
    .into_iter()
    .map(|(phase, _)| phase)
    .collect();
    let groups = bundle.machine.groups_reading_visits();
    for (seat, rule) in met {
        for ((from, result), phases) in &groups {
            if *from != seat {
                continue;
            }
            if let Some(phase) = phases.iter().find(|phase| !reached.contains(**phase)) {
                return Some(EntryRefusal::NeedsVisits {
                    group: ((*from).into(), (*result).into()),
                    phase: (*phase).into(),
                    reached_by: rule.map(str::to_string),
                });
            }
        }
    }
    None
}

fn change(bundle: &Bundle, entry: &str) -> Option<EntryRefusal> {
    for (index, phase) in bundle.machine.phases.iter().enumerate() {
        let Some(seat) = bundle.seats.get(phase) else {
            continue;
        };
        let SeatBody::Sequence { steps } = &seat.body else {
            continue;
        };
        let declares = seat.inputs.iter().any(|input| input == "change");
        for (step_index, step) in steps.iter().enumerate() {
            let StepBody::Dialect { execution } = &step.body else {
                continue;
            };
            if !crate::engine::sequence::needs_change(execution) || (declares && step_index > 0) {
                continue;
            }
            let sources: Vec<&str> = bundle.machine.phases[..index]
                .iter()
                .filter(|name| CHANGE_SOURCES.contains(&name.as_str()))
                .filter(|name| {
                    bundle
                        .seats
                        .get(*name)
                        .is_some_and(|seat| seat.inputs.iter().any(|input| input == "change"))
                })
                .map(String::as_str)
                .collect();
            if sources.contains(&entry) {
                continue;
            }
            if let Some((_, rule)) = walk(&bundle.machine, entry, |_| true, &sources)
                .into_iter()
                .find(|(met, _)| *met == phase)
            {
                return Some(EntryRefusal::NeedsChange {
                    step: format!("{phase}:{}", step.name),
                    sources: sources.into_iter().map(str::to_string).collect(),
                    reached_by: rule.map(str::to_string),
                });
            }
        }
    }
    None
}

fn walk<'a>(
    machine: &'a Machine,
    entry: &'a str,
    follows: impl Fn(&Rule) -> bool,
    avoid: &[&str],
) -> Vec<(&'a str, Option<&'a str>)> {
    let mut reached = vec![(entry, None)];
    let mut seen = BTreeSet::from([entry]);
    let mut index = 0;
    while let Some(&(phase, _)) = reached.get(index) {
        index += 1;
        if machine.terminal.iter().any(|terminal| terminal == phase) {
            continue;
        }
        for rule in machine
            .rules
            .iter()
            .filter(|rule| rule.from == phase && follows(rule))
        {
            if let Some(next) = rule.next.as_deref() {
                if !avoid.contains(&next) && seen.insert(next) {
                    reached.push((next, Some(rule.id.as_str())));
                }
            }
        }
    }
    reached
}
