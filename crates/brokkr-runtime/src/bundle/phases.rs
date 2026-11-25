//! The compiler's lints over a table's phases: what the table rules before
//! any seat is read, and its last lints once its seats are read.

use std::collections::BTreeMap;

use brokkr_core::policy::Machine;
use serde_json::Value;

use super::{compose, parse_egress_minimum, CompileError, Dialect, Seat};
use crate::agents::EgressClass;
use crate::dialect::{rendered_phases, DIALECT_PHASES};
use crate::phase_name;

/// What the bundle's table and document rule before any seat is read.
pub(super) struct Table {
    pub(super) machine: Machine,
    /// A dialect phase is in the table.
    pub(super) uses_dialect: bool,
    pub(super) dialect_prompts: BTreeMap<String, String>,
    /// The egress minimum the bundle declares (decision 0036).
    pub(super) declared_minimum: EgressClass,
    pub(super) protected_phase: String,
}

/// Why the compiler's phase lints refused a table, one variant per check,
/// each name borrowed from the table, the document, the realm or its
/// dialect. Its `Display` is the operator's text byte for byte; `refused`
/// folds it into the compile refusal, the one place it becomes text (#353).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum PhaseRefusal<'a> {
    /// The realm dialect's pin lacks a rendered phase's instructions.
    #[error("dialect '{dialect}' pin carries no rendered instructions for phase '{phase}'")]
    MissingRendered { dialect: &'a str, phase: &'a str },
    /// A mapped realm declares no dialect, but the table has a dialect phase.
    #[error("realm '{realm}' declares no dialect, but phase '{phase}' needs one")]
    UndeclaredDialect { realm: &'a str, phase: &'a str },
    /// The table lacks the protected review phase.
    #[error(
        "policy has no '{phase}' phase; the protected review \
         gate is non-removable (extension model, layer 1)"
    )]
    NoProtectedPhase { phase: &'a str },
    /// A non-terminal phase has no seat to run it.
    #[error("non-terminal phase '{phase}' has no seat (no executor can run it)")]
    Unseated { phase: &'a str },
    /// A path from the initial phase reaches a terminal around the gate.
    #[error(
        "policy reaches terminal '{terminal}' without passing '{protected}'; \
         a path to shipping that bypasses the protected review gate is \
         constitutionally rejected"
    )]
    Avoidable {
        terminal: &'a str,
        protected: &'a str,
    },
}

/// The one place a phase refusal becomes the compile refusal's text.
fn refused(refusal: PhaseRefusal<'_>) -> CompileError {
    CompileError::Invalid(refusal.to_string())
}

/// The machine and the table, judged in order: the realm dialect's prompts
/// and its presence, the declared egress minimum, the protected phase.
pub(super) fn table(
    resolved: &compose::Resolved,
    realm_name: Option<&str>,
    dialect: Option<&Dialect>,
) -> Result<Table, CompileError> {
    let config = &resolved.document;
    let table = &resolved.table;
    let machine = Machine::from_table(table)?;
    let uses_dialect = machine
        .phases
        .iter()
        .any(|phase| DIALECT_PHASES.contains(&phase.as_str()));

    let dialect_prompts = dialect_prompts(uses_dialect, dialect).map_err(refused)?;
    refuse_undeclared_dialect(&machine, realm_name, dialect).map_err(refused)?;

    let declared_minimum = parse_egress_minimum(config)?;
    let protected_phase = protected_phase(config, &machine).map_err(refused)?;
    Ok(Table {
        machine,
        uses_dialect,
        dialect_prompts,
        declared_minimum,
        protected_phase,
    })
}

/// The realm dialect's rendered instructions for each rendered phase, when
/// a dialect phase is seated and the realm declares a dialect.
fn dialect_prompts(
    uses_dialect: bool,
    dialect: Option<&Dialect>,
) -> Result<BTreeMap<String, String>, PhaseRefusal<'_>> {
    let mut dialect_prompts = BTreeMap::new();
    if uses_dialect {
        if let Some(dialect) = dialect {
            for phase in rendered_phases() {
                let Some(rendered) = dialect.rendered.get(phase) else {
                    return Err(PhaseRefusal::MissingRendered {
                        dialect: &dialect.name,
                        phase,
                    });
                };
                dialect_prompts.insert(phase.to_string(), rendered.clone());
            }
        }
    }
    Ok(dialect_prompts)
}

/// A mapped realm that declares no dialect seats no dialect phase.
fn refuse_undeclared_dialect<'a>(
    machine: &'a Machine,
    realm_name: Option<&'a str>,
    dialect: Option<&Dialect>,
) -> Result<(), PhaseRefusal<'a>> {
    if let Some(phase) = machine
        .phases
        .iter()
        .find(|phase| DIALECT_PHASES.contains(&phase.as_str()))
    {
        if realm_name.is_some() && dialect.is_none() {
            return Err(PhaseRefusal::UndeclaredDialect {
                realm: realm_name.unwrap_or("<unmapped>"),
                phase,
            });
        }
    }
    Ok(())
}

/// The protected review phase the bundle names, which the table must have
/// and no path may avoid.
fn protected_phase<'a>(
    config: &'a Value,
    machine: &'a Machine,
) -> Result<String, PhaseRefusal<'a>> {
    let protected_phase = config
        .get("protected_phase")
        .and_then(Value::as_str)
        .unwrap_or(phase_name::REVIEW);
    if !machine.phases.iter().any(|phase| phase == protected_phase) {
        return Err(PhaseRefusal::NoProtectedPhase {
            phase: protected_phase,
        });
    }
    assert_phase_unavoidable(machine, protected_phase)?;
    Ok(protected_phase.to_string())
}

/// Every non-terminal phase has a seat to run it, and then, after the seat
/// lints, decision 0050 ruling 4: every valuation of the table is ruled.
pub(super) fn seated_and_ruled(
    machine: &Machine,
    seats: &BTreeMap<String, Seat>,
) -> Result<(), CompileError> {
    unseated(machine, seats).map_err(refused)?;
    machine.refuse_unruled()?;
    Ok(())
}

/// The first non-terminal phase, in table order, that no seat runs.
fn unseated<'a>(
    machine: &'a Machine,
    seats: &BTreeMap<String, Seat>,
) -> Result<(), PhaseRefusal<'a>> {
    for phase in &machine.phases {
        if !machine.terminal.contains(phase) && !seats.contains_key(phase) {
            return Err(PhaseRefusal::Unseated { phase });
        }
    }
    Ok(())
}

/// Decision 0094 shares the core walk with entry admission.
fn assert_phase_unavoidable<'a>(
    machine: &'a Machine,
    protected: &'a str,
) -> Result<(), PhaseRefusal<'a>> {
    if let Some(terminal) = bypassing_terminal(machine, &machine.initial, protected) {
        return Err(PhaseRefusal::Avoidable {
            terminal,
            protected,
        });
    }
    Ok(())
}

/// The constitutional cut, shared by the table lint and entry admission
/// (decision 0094; decision 0071 ruling 5), with each caller's start.
pub(super) fn bypassing_terminal<'a>(
    machine: &'a Machine,
    start: &str,
    protected: &str,
) -> Option<&'a str> {
    let reachable = machine.reach_avoiding(start, &[protected]);
    machine
        .terminal
        .iter()
        .find(|terminal| terminal.as_str() != "stop" && reachable.contains(terminal.as_str()))
        .map(String::as_str)
}

#[cfg(test)]
mod tests;
