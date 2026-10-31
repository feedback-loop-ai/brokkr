//! The steps of [`parse_sequence`](super::parse_sequence) (#288, slice G):
//! the step list, each step's name, its results vocabulary, its body by its
//! shape (the dialect step's its own), and its class. `parse_sequence` reads
//! as their sequence.

use std::collections::BTreeSet;

use brokkr_core::policy::{Machine, Rule};
use brokkr_protocol::hands::HandsSpec;
use serde_json::Value;

use super::charters::parse_role;
use super::gate::{self, GateAssessment, WorkSite};
use super::parse::{SeatScope, SiteCompiler};
use super::site_form::{classify, StepShape, STEP};
use super::{
    command_parts, dialect_gate_site, dialect_results, parse_class, parse_command,
    record_inline_tools, refuse_class_without_a_driver, resolve_reference, Aggregate, CompileError,
    DialectExecution, Invocation, SeatBody, SeatClass, SequenceStep, StepBody,
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
                command: parse_command((compiler.engine, at.dir), what, step_raw, at.secrets)?,
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
        (compiler.engine, at.dir),
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
            SeatClass::of(matches!(
                super::gate::panel_assessment(members),
                super::GateAssessment::AllGate
            ))
        }
        StepBody::Dialect { .. } => SeatClass::Gate,
    })
}

/// Parse a sequence body: named steps, each a single driver or a panel,
/// run serially inside one effect. At least two steps (a one-step
/// sequence is a single seat); names unique case-insensitively. Beside them,
/// `has_gate`: every step, a dialect one or an unsupplied check too, WROTE `gate`.
pub(super) fn parse_sequence(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    phase: &str,
    raw: &Value,
) -> Result<(Vec<SequenceStep>, GateAssessment), CompileError> {
    let steps_raw = sequence_steps(phase, raw)?;
    let mut steps: Vec<SequenceStep> = Vec::with_capacity(steps_raw.len());
    let mut whole_gate = GateAssessment::AllGate;
    for (index, step_raw) in steps_raw.iter().enumerate() {
        let name = step_name(phase, index, step_raw, &steps)?;
        let what = format!("{phase}:{name}");
        let shape = classify(&what, step_raw, &STEP)?;
        let step = StepAt {
            phase,
            name,
            what: &what,
            raw: step_raw,
        };
        let step_results = step_results(at, step, index + 1 == steps_raw.len())?;
        let written = (shape == StepShape::Dialect)
            .then(|| parse_class(&what, step_raw).unwrap_or(SeatClass::Work));
        let parsed = step_body(compiler, at, step, shape, step_results)?;
        if written == Some(SeatClass::Work) {
            whole_gate.first_work(WorkSite {
                path: vec![name.into()],
                compiled: parsed.is_some(),
            });
        }
        let Some(parsed) = parsed else { continue };
        let class = step_class(compiler, at, step, &parsed.body, parsed.hands)?;
        if written.is_none() {
            if let GateAssessment::ContainsWork(site) =
                gate::step_assessment(name, class, &parsed.body)
            {
                whole_gate.first_work(site);
            }
        }
        steps.push(SequenceStep {
            name: name.to_string(),
            class,
            results: parsed.results,
            body: parsed.body,
            ends_on: Vec::new(),
            hard_results: Vec::new(),
        });
    }
    Ok((steps, whole_gate))
}

/// Derive every sequence step's compiled endings, and refuse a table arm
/// whose word no step of the sequence can emit (decision 0050, ruling 7).
pub(super) fn stamp_endings(
    machine: &Machine,
    bundle: &str,
    phase: &str,
    seat_results: &[String],
    body: &mut SeatBody,
) -> Result<(), CompileError> {
    match body {
        SeatBody::Invocation(_) => Ok(()),
        SeatBody::Sequence { steps } => stamp_sequence(machine, bundle, phase, seat_results, steps),
        SeatBody::Select { cases, default } => {
            for case in cases.values_mut() {
                stamp_endings(machine, bundle, phase, seat_results, case)?;
            }
            if let Some(default) = default.as_deref_mut() {
                stamp_endings(machine, bundle, phase, seat_results, default)?;
            }
            Ok(())
        }
    }
}

/// A sequence's compiled endings: each non-final step's ending words, and
/// the arms its phase's table holds that the sequence cannot reach.
fn stamp_sequence(
    machine: &Machine,
    bundle: &str,
    phase: &str,
    seat_results: &[String],
    steps: &mut [SequenceStep],
) -> Result<(), CompileError> {
    let hard = hard_results(machine, phase);
    let last = steps.len().saturating_sub(1);
    for index in 0..last {
        let (before, later) = steps.split_at_mut(index + 1);
        let step = &mut before[index];
        step.ends_on = endings(&step.results, later, seat_results, &hard);
        step.hard_results = step
            .results
            .iter()
            .filter(|word| hard.contains(*word))
            .cloned()
            .collect();
    }
    // The final step (the schema gives a sequence at least one) ends the
    // sequence by definition, so it holds no endings of its own.
    for step in steps.iter_mut().skip(last) {
        step.ends_on.clear();
        step.hard_results.clear();
    }
    refuse_unreachable(machine, bundle, phase, steps)
}

/// The words `results` holds that end a sequence at a non-final step: the
/// seat's words no later step can emit, and every hard word the step can
/// report even when a later step can emit it too.
fn endings(
    results: &[String],
    later: &[SequenceStep],
    seat_results: &[String],
    hard: &BTreeSet<String>,
) -> Vec<String> {
    let later: BTreeSet<&str> = later
        .iter()
        .flat_map(|step| step.results.iter().map(String::as_str))
        .collect();
    let mut words: BTreeSet<String> = results
        .iter()
        .filter(|word| seat_results.contains(word) && !later.contains(word.as_str()))
        .cloned()
        .collect();
    words.extend(results.iter().filter(|word| hard.contains(*word)).cloned());
    words.into_iter().collect()
}

/// Whether entering `phase` stops the run: it is terminal AND it is the
/// stopped terminal. This is the engine's own test (`request_or_finish`),
/// named once so the compiler cannot call a hard arm to a phase merely
/// NAMED `stop` a stop. Both callers read it here.
pub(crate) fn stops_run(machine: &Machine, phase: &str) -> bool {
    machine.terminal.iter().any(|terminal| terminal == phase) && phase == "stop"
}

/// Whether this table arm is a hard stop: hard severity AND a destination
/// the engine stops at, because `Machine::evaluate` follows `next` and the
/// engine tells a stopped run from a completed one by the terminal phase,
/// not by severity. A hard arm that advances to `done` ends nothing.
fn is_hard_stop(machine: &Machine, rule: &Rule) -> bool {
    rule.severity == "hard"
        && rule
            .next
            .as_deref()
            .is_some_and(|next| stops_run(machine, next))
}

/// The phase's hard vocabulary: every word whose every arm in the composed
/// table is hard and goes to `stop` (decision 0050, ruling 7).
pub(super) fn hard_results(machine: &Machine, phase: &str) -> BTreeSet<String> {
    let mut words: std::collections::BTreeMap<&str, bool> = std::collections::BTreeMap::new();
    for rule in machine.rules.iter().filter(|rule| rule.from == phase) {
        words
            .entry(rule.result.as_str())
            .and_modify(|hard| *hard &= is_hard_stop(machine, rule))
            .or_insert_with(|| is_hard_stop(machine, rule));
    }
    words
        .into_iter()
        .filter(|(_, hard)| *hard)
        .map(|(word, _)| word.to_string())
        .collect()
}

/// Refuse a table arm for `phase` whose word no step the sequence can reach
/// can emit, naming the rule and the word (decision 0050, ruling 7). A
/// middle step's word reaches the table only through a compiled ending: its
/// plain checkpoint is context for the later steps, never the effect's
/// result. The walk stops at a step every result of which ends the
/// sequence: no later step runs, so none of their words admits an arm.
fn refuse_unreachable(
    machine: &Machine,
    bundle: &str,
    phase: &str,
    steps: &[SequenceStep],
) -> Result<(), CompileError> {
    let last = steps.len().saturating_sub(1);
    let mut emittable: BTreeSet<&str> = BTreeSet::new();
    for (index, step) in steps.iter().enumerate() {
        if index == last {
            emittable.extend(step.results.iter().map(String::as_str));
            continue;
        }
        emittable.extend(step.ends_on.iter().map(String::as_str));
        if step.results.iter().all(|word| step.ends_on.contains(word)) {
            break;
        }
    }
    for rule in machine.rules.iter().filter(|rule| rule.from == phase) {
        if !emittable.contains(rule.result.as_str()) {
            return Err(CompileError::Invalid(format!(
                "bundle '{bundle}' sequence for phase '{phase}' cannot emit table arm '{}' \
                 result '{}': no step of the sequence can report it",
                rule.id, rule.result
            )));
        }
    }
    Ok(())
}
