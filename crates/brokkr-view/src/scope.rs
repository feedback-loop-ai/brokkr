//! Scoping a run to one phase or one seat: the terminal's rule, ruled the
//! one rule by the operator on 2026-10-10 (#351). A phase scope keeps that
//! phase, its participants and its journal rows. A seat scope keeps the
//! seat's phases, matches participants by key, and keeps only those
//! phases' journal rows. The TUI and `brokkr inspect` ask [`lens_for`] and
//! the `keeps_*` predicates; the browser cannot call Rust, so the run view
//! serves every scope's [`Membership`] already decided, and the page only
//! paints it (decision 0013).

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

use crate::{JournalRow, Participant, Phase, RunView};

/// The console's exclusive scoping, as the verbs a terminal has.
pub enum Scope {
    Phase(String),
    Seat(String),
}

/// A resolved scope: which phases survive, and — for a seat — which
/// participant keys. Membership itself is a model field on both sides,
/// so no surface re-implements the predicate.
pub struct Lens {
    phases: Vec<String>,
    keys: Vec<String>,
    by_key: bool,
}

impl Lens {
    fn phase(name: &str) -> Lens {
        Lens {
            phases: vec![name.to_string()],
            keys: Vec::new(),
            by_key: false,
        }
    }

    /// The lens over every participant a seat scope matched: their
    /// phases, in the order first seen, and their keys.
    fn seats(matched: &[&Participant]) -> Lens {
        let mut phases = Vec::new();
        for part in matched {
            if let Some(phase) = &part.phase {
                if !phases.contains(phase) {
                    phases.push(phase.clone());
                }
            }
        }
        Lens {
            phases,
            keys: matched.iter().map(|part| part.key.clone()).collect(),
            by_key: true,
        }
    }
}

/// A scope that matches nothing in the run, naming what the run offers.
/// The names are the journal's own: a terminal sanitizes the text it
/// prints, as it does every journal string. Its `Display` is the
/// operator's text; this pure crate takes no `thiserror` and names no
/// `std::error` (decision 0071 ruling 1), so the shell wraps the text.
#[derive(Debug, PartialEq, Eq)]
pub enum ScopeError {
    UnknownPhase {
        phase: String,
        visited: Vec<String>,
    },
    Unseated {
        seat: String,
        participants: Vec<String>,
    },
}

impl fmt::Display for ScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            ScopeError::UnknownPhase { phase, visited } => format!(
                "no phase '{phase}' in this run; visited phases: {}",
                visited.join(", ")
            ),
            ScopeError::Unseated { seat, participants } => format!(
                "no seat '{seat}' in this run; participants: {}",
                participants.join(", ")
            ),
        })
    }
}

/// Every participant a seat scope names. ALL occurrences: a re-entered
/// phase really did run that seat twice, and hiding one is a false
/// statement about the run. An exact participant key matches too — that
/// is what a click selects.
fn seat_matches<'a>(view: &'a RunView, label: &str) -> Vec<&'a Participant> {
    view.participants
        .iter()
        .filter(|part| part.label == label || part.key == label)
        .collect()
}

/// Resolve a scope against a run, or say what the run actually offers.
/// A value matching nothing is an error rather than an empty table: an
/// empty seats table reads as "this phase did nothing", which is a claim
/// about a run no surface can make.
pub fn lens_for(view: &RunView, scope: Option<&Scope>) -> Result<Option<Lens>, ScopeError> {
    match scope {
        None => Ok(None),
        Some(Scope::Phase(name)) => {
            if view.phases.iter().any(|phase| phase.name == *name) {
                Ok(Some(Lens::phase(name)))
            } else {
                Err(ScopeError::UnknownPhase {
                    phase: name.clone(),
                    visited: view.phases.iter().map(|phase| phase.name.clone()).collect(),
                })
            }
        }
        Some(Scope::Seat(label)) => {
            let matched = seat_matches(view, label);
            if matched.is_empty() {
                let mut participants: Vec<String> = view
                    .participants
                    .iter()
                    .map(|part| part.label.clone())
                    .collect();
                participants.dedup();
                Err(ScopeError::Unseated {
                    seat: label.clone(),
                    participants,
                })
            } else {
                Ok(Some(Lens::seats(&matched)))
            }
        }
    }
}

/// The scope predicate for a phase, in one place: the terminal's graph
/// and the TUI ask the same question and must never answer it twice.
pub fn keeps_phase(lens: Option<&Lens>, phase: &Phase) -> bool {
    match lens {
        None => true,
        Some(lens) => lens.phases.contains(&phase.name),
    }
}

pub fn keeps_participant(lens: Option<&Lens>, part: &Participant) -> bool {
    match lens {
        None => true,
        Some(lens) if lens.by_key => lens.keys.contains(&part.key),
        Some(lens) => match &part.phase {
            Some(phase) => lens.phases.contains(phase),
            None => false,
        },
    }
}

pub fn keeps_row(lens: Option<&Lens>, row: &JournalRow) -> bool {
    match lens {
        None => true,
        Some(lens) => row.phases.iter().any(|name| lens.phases.contains(name)),
    }
}

/// What one scope keeps, decided: the phases by name, the participants by
/// key and the journal rows by `seq`, each in the run view's order.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Membership {
    pub phases: Vec<String>,
    pub participants: Vec<String>,
    pub rows: Vec<u64>,
}

/// Every scope a click can select, served so the browser paints the
/// terminal's answer: each phase by name, and each participant by key.
#[derive(Serialize, Default)]
pub struct Scopes {
    pub phases: BTreeMap<String, Membership>,
    pub seats: BTreeMap<String, Membership>,
}

fn membership(view: &RunView, lens: &Lens) -> Membership {
    let lens = Some(lens);
    Membership {
        phases: view
            .phases
            .iter()
            .filter(|phase| keeps_phase(lens, phase))
            .map(|phase| phase.name.clone())
            .collect(),
        participants: view
            .participants
            .iter()
            .filter(|part| keeps_participant(lens, part))
            .map(|part| part.key.clone())
            .collect(),
        rows: view
            .journal
            .iter()
            .filter(|row| keeps_row(lens, row))
            .map(|row| row.seq)
            .collect(),
    }
}

/// The membership of every phase scope and every seat scope by key, by
/// the same lens [`lens_for`] resolves for a terminal.
pub(crate) fn scopes(view: &RunView) -> Scopes {
    Scopes {
        phases: view
            .phases
            .iter()
            .map(|phase| {
                let lens = Lens::phase(&phase.name);
                (phase.name.clone(), membership(view, &lens))
            })
            .collect(),
        seats: view
            .participants
            .iter()
            .map(|part| {
                let lens = Lens::seats(&seat_matches(view, &part.key));
                (part.key.clone(), membership(view, &lens))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests;
