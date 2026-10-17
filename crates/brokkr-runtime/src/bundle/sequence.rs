//! The steps of [`parse_sequence`](super::parse_sequence) (#288, slice G):
//! the step list, each step's name, its results vocabulary, its body by its
//! shape (the dialect step's its own), and its class. `parse_sequence` reads
//! as their sequence.

use brokkr_protocol::hands::HandsSpec;
use serde_json::Value;

use super::charters::parse_role;
use super::parse::{SeatScope, SiteCompiler};
use super::site_form::{StepShape, STEP};
use super::{
    command_parts, dialect_gate_site, dialect_results, every, parse_command, record_inline_tools,
    refuse_class_without_a_driver, resolve_reference, Aggregate, CompileError, DialectExecution,
    Invocation, SeatClass, SequenceStep, StepBody,
};
use crate::dialect::DialectPhase;

/// One sequence step as its seat wrote it: the seat's phase, the step's
/// name and `phase:name`, and its declaration.
#[derive(Clone, Copy)]
pub(super) struct StepAt<'s> {
    pub(super) phase: &'s str,
    pub(super) name: &'s str,
    pub(super) what: &'s str,
    pub(super) raw: &'s Value,
}

/// A step's body, the results vocabulary it compiles with, and the hands
/// its agent declared, before its class is judged.
pub(super) struct ParsedStep {
    pub(super) body: StepBody,
    pub(super) results: Vec<String>,
    pub(super) hands: Option<HandsSpec>,
}

/// The seat's `sequence`: an array of at least two steps.
pub(super) fn sequence_steps<'v>(phase: &str, raw: &'v Value) -> Result<&'v [Value], CompileError> {
    let steps_raw = raw
        .get("sequence")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{phase}' sequence must be an array"))
        })?;
    if steps_raw.len() < 2 {
        return Err(CompileError::Invalid(format!(
            "seat '{phase}' sequence needs at least two steps; \
             a one-step sequence is a single seat"
        )));
    }
    Ok(steps_raw)
}

/// Step `index`'s non-empty name, unique case-insensitively among the
/// `steps` parsed before it.
pub(super) fn step_name<'v>(
    phase: &str,
    index: usize,
    step_raw: &'v Value,
    steps: &[SequenceStep],
) -> Result<&'v str, CompileError> {
    let name = step_raw
        .get("name")
        .and_then(Value::as_str)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!(
                "seat '{phase}' sequence step {index} needs a non-empty 'name'"
            ))
        })?;
    if steps.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
        return Err(CompileError::Invalid(format!(
            "seat '{phase}' sequence has duplicate step name '{name}' \
             (step names are case-insensitive)"
        )));
    }
    Ok(name)
}

/// The step's results vocabulary: the final step receives the seat's, and
/// writes none of its own; another step writes its own non-empty
/// `results`, or takes what its declared aggregate requires.
pub(super) fn step_results(
    at: SeatScope<'_>,
    step: StepAt<'_>,
    final_step: bool,
) -> Result<Vec<String>, CompileError> {
    let StepAt {
        what,
        raw: step_raw,
        ..
    } = step;
    if final_step && step_raw.get("results").is_some() {
        return Err(CompileError::Invalid(format!(
            "sequence step '{what}' is final and receives the seat's results; remove its ignored 'results'"
        )));
    }
    if final_step {
        Ok(at.results.to_vec())
    } else if step_raw.get("results").is_some() {
        step_raw
            .get("results")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|values| !values.is_empty())
            .ok_or_else(|| {
                CompileError::Invalid(format!(
                    "sequence step '{what}' needs its own non-empty 'results' vocabulary"
                ))
            })
    } else if let Some(aggregate) = step_raw
        .get("aggregate")
        .and_then(Value::as_str)
        .and_then(Aggregate::parse)
    {
        Ok(aggregate
            .required_results()
            .iter()
            .map(|result| (*result).to_string())
            .collect())
    } else {
        Err(CompileError::Invalid(format!(
            "sequence step '{what}' needs its own non-empty 'results' vocabulary"
        )))
    }
}

/// The step's body by its shape, with the `step_results` it compiles with
/// and its agent's hands; `None` for a dialect check its dialect does not
/// supply, which compiles to no step.
pub(super) fn step_body(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    step: StepAt<'_>,
    shape: StepShape,
    step_results: Vec<String>,
) -> Result<Option<ParsedStep>, CompileError> {
    let StepAt {
        what,
        raw: step_raw,
        ..
    } = step;
    let mut agent_hands = None;
    let body = match shape {
        StepShape::Dialect => return dialect_step(compiler, at, step),
        StepShape::Agent => {
            let resolved =
                resolve_reference(compiler, at.dir, what, step_raw, at.secrets, STEP.role)?;
            agent_hands = resolved.hands;
            StepBody::Invocation(Invocation::Single {
                role_path: resolved.role_path,
                command: resolved.command,
                candidates: resolved.candidates,
                written: Vec::new(),
                class: (),
            })
        }
        StepShape::Panel => {
            let step_at = SeatScope {
                results: &step_results,
                ..at
            };
            let (members, aggregate) = compiler.parse_panel(step_at, what, step_raw)?;
            StepBody::Invocation(Invocation::Panel { members, aggregate })
        }
        StepShape::Inline => {
            let written = compiler.inline_written(at, what, step_raw, false)?;
            let sites = &mut compiler.sites;
            StepBody::Invocation(Invocation::Single {
                role_path: parse_role(at.dir, what, step_raw, compiler.charters, sites)?,
                command: parse_command(at.dir, what, step_raw, at.secrets)?,
                candidates: Vec::new(),
                written,
                class: (),
            })
        }
    };
    Ok(Some(ParsedStep {
        body,
        results: step_results,
        hands: agent_hands,
    }))
}

/// A dialect step: the operation its phase needs, resolved by the realm's
/// dialect into an exec-generated check judged as a site; `None` for a
/// `check` the dialect does not supply.
fn dialect_step(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    step: StepAt<'_>,
) -> Result<Option<ParsedStep>, CompileError> {
    let StepAt {
        phase,
        name,
        what,
        raw: step_raw,
    } = step;
    let operation = step_raw
        .get("dialect")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CompileError::Invalid(format!(
                "sequence step '{what}' dialect must be 'validate' or 'check'"
            ))
        })?;
    let expected = if DialectPhase::parse(phase).is_some_and(DialectPhase::is_loop) {
        "check"
    } else {
        "validate"
    };
    if operation != expected {
        return Err(CompileError::Invalid(format!(
            "sequence step '{what}' uses dialect operation '{operation}'; phase '{phase}' needs '{expected}'"
        )));
    }
    let dialect = compiler.dialect.ok_or_else(|| {
        CompileError::Invalid(format!(
            "phase '{phase}' needs a realm dialect to resolve step '{name}'"
        ))
    })?;
    let Some(command) = dialect.validation(phase) else {
        if operation == "check" {
            // No executable site is compiled for an unsupported
            // check, so a `tools` declaration here would have no
            // owner and could only be discarded (design D5.2).
            if step_raw.get("tools").is_some() {
                return Err(CompileError::Invalid(format!(
                    "sequence step '{what}' declares 'tools' on a dialect step whose \
                     '{operation}' the dialect does not supply; no executable site \
                     exists to own the declaration, so it could only be discarded — \
                     refused (decision 0065 slice one, design D5)"
                )));
            }
            return Ok(None);
        }
        return Err(CompileError::Invalid(format!(
            "dialect '{}' declares phase '{phase}' {operation} unsupported",
            dialect.name
        )));
    };
    let synthetic = dialect_gate_site(what, compiler.boundary)?;
    // The dialect's validator is an exec-generated check: it can
    // own a checked empty declaration and represents no nonempty
    // local field (design D5.2), judged against the command the
    // validator actually runs.
    record_inline_tools(
        at.dir,
        what,
        step_raw,
        false,
        &command_parts(&synthetic),
        Some(compiler.agents.adapters()?),
        &mut compiler.sites,
    )?;
    compiler.judge(at, what, &synthetic, &[], None)?;
    // A final dialect step emits the deterministic adapter's two
    // outcomes, not every result the enclosing seat accepts. Keeping
    // that actual vocabulary on the compiled step lets the engine see
    // an earlier author result which no remaining step can produce
    // (notably `upstream`) and end the sequence at that boundary.
    let step_results = dialect_results(phase)
        .into_iter()
        .map(str::to_string)
        .collect();
    Ok(Some(ParsedStep {
        body: StepBody::Dialect {
            execution: DialectExecution {
                argv: command.argv.clone(),
                state: command.state.clone(),
            },
        },
        results: step_results,
        hands: None,
    }))
}

/// The step's canonical class (GP1): a dialect step's validator
/// judges, so it is a gate; a panel step is one when its members are.
pub(super) fn step_class(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    step: StepAt<'_>,
    body: &StepBody,
    agent_hands: Option<HandsSpec>,
) -> Result<SeatClass, CompileError> {
    let StepAt {
        what,
        raw: step_raw,
        ..
    } = step;
    Ok(match body {
        StepBody::Invocation(Invocation::Single { candidates, .. }) => {
            compiler.judge(at, what, step_raw, candidates, agent_hands)?
        }
        StepBody::Invocation(Invocation::Panel { members, .. }) => {
            refuse_class_without_a_driver(what, step_raw)?;
            SeatClass::of(every(members.iter().map(|m| m.class == SeatClass::Gate)))
        }
        StepBody::Dialect { .. } => SeatClass::Gate,
    })
}
