//! The compiler's lints over a table's phases: what the table rules before
//! any seat is read, and its last lints once its seats are read.

use std::collections::BTreeMap;

use brokkr_core::policy::Machine;
use serde_json::Value;

use super::{assert_phase_unavoidable, compose, parse_egress_minimum, CompileError, Dialect, Seat};
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

    let dialect_prompts = dialect_prompts(uses_dialect, dialect)?;
    refuse_undeclared_dialect(&machine, realm_name, dialect)?;

    let declared_minimum = parse_egress_minimum(config)?;
    let protected_phase = protected_phase(config, &machine, table)?;
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
) -> Result<BTreeMap<String, String>, CompileError> {
    let mut dialect_prompts = BTreeMap::new();
    if uses_dialect {
        if let Some(dialect) = dialect {
            for phase in rendered_phases() {
                let rendered = dialect.rendered.get(phase).cloned().ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "dialect '{}' pin carries no rendered instructions for phase '{phase}'",
                        dialect.name
                    ))
                })?;
                dialect_prompts.insert(phase.to_string(), rendered);
            }
        }
    }
    Ok(dialect_prompts)
}

/// A mapped realm that declares no dialect seats no dialect phase.
fn refuse_undeclared_dialect(
    machine: &Machine,
    realm_name: Option<&str>,
    dialect: Option<&Dialect>,
) -> Result<(), CompileError> {
    if let Some(phase) = machine
        .phases
        .iter()
        .find(|phase| DIALECT_PHASES.contains(&phase.as_str()))
    {
        if realm_name.is_some() && dialect.is_none() {
            return Err(CompileError::Invalid(format!(
                "realm '{}' declares no dialect, but phase '{phase}' needs one",
                realm_name.unwrap_or("<unmapped>")
            )));
        }
    }
    Ok(())
}

/// The protected review phase the bundle names, which the table must have
/// and no path may avoid.
fn protected_phase(
    config: &Value,
    machine: &Machine,
    table: &Value,
) -> Result<String, CompileError> {
    let protected_phase = config
        .get("protected_phase")
        .and_then(Value::as_str)
        .unwrap_or(phase_name::REVIEW)
        .to_string();
    if !machine.phases.contains(&protected_phase) {
        return Err(CompileError::Invalid(format!(
            "policy has no '{protected_phase}' phase; the protected review \
             gate is non-removable (extension model, layer 1)"
        )));
    }
    assert_phase_unavoidable(machine, table, &protected_phase)?;
    Ok(protected_phase)
}

/// Every non-terminal phase has a seat to run it, and then, after the seat
/// lints, decision 0050 ruling 4: every valuation of the table is ruled.
pub(super) fn seated_and_ruled(
    machine: &Machine,
    seats: &BTreeMap<String, Seat>,
) -> Result<(), CompileError> {
    for phase in &machine.phases {
        if !machine.terminal.contains(phase) && !seats.contains_key(phase) {
            return Err(CompileError::Invalid(format!(
                "non-terminal phase '{phase}' has no seat (no executor can run it)"
            )));
        }
    }
    machine.refuse_unruled()?;
    Ok(())
}
