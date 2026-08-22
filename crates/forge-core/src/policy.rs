//! Strict policy evaluation — the Rust port of `src/forge/machine.py`
//! under decisions 0001/0002/0004, differential-tested against the
//! committed oracle corpus (`fixtures/evaluator/corpus.ndjson`).
//!
//! Laws carried over exactly:
//! - first matching rule wins, in table order;
//! - the condition vocabulary is closed and enforced at load
//!   (`PolicyError`), so a typo'd key can never silently deaden a rule;
//! - an absent (or null) input never satisfies a condition;
//! - a present input the vocabulary cannot read parks the evaluation
//!   (`Outcome::NoRule` with `problem`) — never coerced, never guessed;
//! - an unmatched (phase, result) pair parks with `problem: None`.

use serde_json::{Map, Value};
use thiserror::Error;

/// Residual-finding severity axis, lowest to highest — the value
/// vocabulary of `max_residual_severity`. Distinct from ruling severity.
pub const SEVERITY_ORDER: [&str; 6] = ["none", "info", "low", "medium", "high", "critical"];

/// Ruling severity axis — the forge.phase-event/v1 vocabulary.
pub const RULING_SEVERITIES: [&str; 3] = ["normal", "flagged", "hard"];

pub const BOOLEAN_INPUTS: [&str; 6] = [
    "skip_verify",
    "fixes_applied",
    "has_security_residual",
    "high_risk_uncovered",
    "drift_detected",
    "dirty_worktrees",
];
pub const COUNTER_INPUTS: [&str; 1] = ["consecutive_failures"];
pub const SEVERITY_INPUTS: [&str; 1] = ["max_residual_severity"];

#[derive(Debug, Error)]
#[error("malformed phase machine table: {0}")]
pub struct PolicyError(pub String);

fn severity_rank(name: &str) -> Option<usize> {
    SEVERITY_ORDER.iter().position(|s| *s == name)
}

#[derive(Debug, Clone, PartialEq)]
enum Condition {
    CounterGte { name: String, threshold: f64 },
    SeverityAbove { name: String, threshold_rank: usize },
    Flag { name: String, expected: bool },
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub from: String,
    pub result: String,
    pub next: String,
    pub severity: String,
    pub reason: String,
    pub requires_artifacts: Vec<String>,
    when: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Ruling {
        rule_id: String,
        next_phase: String,
        severity: String,
        reason: String,
        requires_artifacts: Vec<String>,
    },
    /// No ruling is possible: the engine MUST park (law 0001). `problem`
    /// None means no rule matched; Some means the machine refused to rule
    /// (unknown phase, unreadable input).
    NoRule { problem: Option<String> },
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub phases: Vec<String>,
    pub initial: String,
    pub terminal: Vec<String>,
    pub shippable_from: Vec<String>,
    pub rules: Vec<Rule>,
}

impl Machine {
    pub fn from_table(table: &Value) -> Result<Machine, PolicyError> {
        let obj = table
            .as_object()
            .ok_or_else(|| PolicyError("table must be an object".into()))?;
        for key in ["phases", "initial", "terminal", "rules"] {
            if !obj.contains_key(key) {
                return Err(PolicyError(format!("table missing '{key}'")));
            }
        }
        let phases = string_array(&obj["phases"], "phases")?;
        let initial = obj["initial"]
            .as_str()
            .ok_or_else(|| PolicyError("initial must be a string".into()))?
            .to_string();
        if !phases.contains(&initial) {
            return Err(PolicyError("initial phase not in phases".into()));
        }
        let terminal = string_array(&obj["terminal"], "terminal")?;
        for t in &terminal {
            if !phases.contains(t) {
                return Err(PolicyError(format!("terminal phase '{t}' not in phases")));
            }
        }
        let shippable_from = match obj.get("shippable_from") {
            Some(v) => string_array(v, "shippable_from")?,
            None => Vec::new(),
        };

        let raw_rules = obj["rules"]
            .as_array()
            .ok_or_else(|| PolicyError("rules must be an array".into()))?;
        let mut rules = Vec::with_capacity(raw_rules.len());
        let mut seen_ids: Vec<String> = Vec::new();
        let mut ruled_unconditionally: Vec<(String, String)> = Vec::new();
        for raw in raw_rules {
            let rule = parse_rule(raw, &phases, &terminal)?;
            if seen_ids.contains(&rule.id) {
                return Err(PolicyError(format!("duplicate rule id {}", rule.id)));
            }
            seen_ids.push(rule.id.clone());
            let group = (rule.from.clone(), rule.result.clone());
            if ruled_unconditionally.contains(&group) {
                return Err(PolicyError(format!(
                    "rule {} is unreachable: an unconditional rule for \
                     ({}, {}) precedes it and first match wins",
                    rule.id, rule.from, rule.result
                )));
            }
            if rule.when.is_empty() {
                ruled_unconditionally.push(group);
            }
            rules.push(rule);
        }
        Ok(Machine {
            phases,
            initial,
            terminal,
            shippable_from,
            rules,
        })
    }

    /// First matching rule wins, in table order (deny-before-allow is a
    /// property of table AUTHORING, preserved here by strict ordering).
    pub fn evaluate(&self, phase: &str, result: &str, inputs: &Map<String, Value>) -> Outcome {
        if !self.phases.iter().any(|p| p == phase) {
            return Outcome::NoRule {
                problem: Some(format!("unknown phase '{phase}'")),
            };
        }
        for rule in &self.rules {
            if rule.from != phase || rule.result != result {
                continue;
            }
            match conditions_met(&rule.when, inputs) {
                Ok(false) => continue,
                Ok(true) => {
                    return Outcome::Ruling {
                        rule_id: rule.id.clone(),
                        next_phase: rule.next.clone(),
                        severity: rule.severity.clone(),
                        reason: rule.reason.clone(),
                        requires_artifacts: rule.requires_artifacts.clone(),
                    }
                }
                Err(problem) => return Outcome::NoRule { problem: Some(problem) },
            }
        }
        Outcome::NoRule { problem: None }
    }
}

fn string_array(value: &Value, what: &str) -> Result<Vec<String>, PolicyError> {
    value
        .as_array()
        .ok_or_else(|| PolicyError(format!("{what} must be an array")))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| PolicyError(format!("{what} entries must be strings")))
        })
        .collect()
}

fn parse_rule(raw: &Value, phases: &[String], terminal: &[String]) -> Result<Rule, PolicyError> {
    let obj = raw
        .as_object()
        .ok_or_else(|| PolicyError("rule must be an object".into()))?;
    let field = |key: &str| -> Result<String, PolicyError> {
        obj.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                PolicyError(format!(
                    "rule {} missing '{key}'",
                    obj.get("id").and_then(Value::as_str).unwrap_or("?")
                ))
            })
    };
    let id = field("id")?;
    let from = field("from")?;
    let result = field("result")?;
    let next = field("next")?;
    let reason = field("reason")?;
    if !phases.contains(&from) || !phases.contains(&next) {
        return Err(PolicyError(format!("rule {id} references unknown phase")));
    }
    if terminal.contains(&from) {
        return Err(PolicyError(format!("rule {id} leaves terminal phase '{from}'")));
    }
    let severity = match obj.get("severity") {
        None => "normal".to_string(),
        Some(v) => {
            let s = v
                .as_str()
                .ok_or_else(|| PolicyError(format!("rule {id} severity must be a string")))?;
            if !RULING_SEVERITIES.contains(&s) {
                return Err(PolicyError(format!(
                    "rule {id} severity '{s}' not in {RULING_SEVERITIES:?}"
                )));
            }
            s.to_string()
        }
    };
    let requires_artifacts = match obj.get("requires_artifacts") {
        None => Vec::new(),
        Some(v) => string_array(v, &format!("rule {id} requires_artifacts"))?,
    };
    let when = match obj.get("when") {
        None => Vec::new(),
        Some(v) => {
            let map = v
                .as_object()
                .ok_or_else(|| PolicyError(format!("rule {id} 'when' must be an object")))?;
            let mut conditions = Vec::with_capacity(map.len());
            for (key, expected) in map {
                conditions.push(parse_condition(&id, key, expected)?);
            }
            conditions
        }
    };
    Ok(Rule {
        id,
        from,
        result,
        next,
        severity,
        reason,
        requires_artifacts,
        when,
    })
}

/// Load-time half of the closed vocabulary: every condition names a
/// declared input and carries a threshold of the right type.
fn parse_condition(rule_id: &str, key: &str, expected: &Value) -> Result<Condition, PolicyError> {
    if let Some(name) = key.strip_suffix("_gte") {
        if !COUNTER_INPUTS.contains(&name) {
            return Err(PolicyError(format!(
                "rule {rule_id}: unknown counter '{name}' in condition '{key}'; \
                 known: {COUNTER_INPUTS:?}"
            )));
        }
        let threshold = expected.as_f64().ok_or_else(|| {
            PolicyError(format!(
                "rule {rule_id}: condition '{key}' needs a numeric threshold, got {expected}"
            ))
        })?;
        return Ok(Condition::CounterGte {
            name: name.to_string(),
            threshold,
        });
    }
    if let Some(name) = key.strip_suffix("_above") {
        if !SEVERITY_INPUTS.contains(&name) {
            return Err(PolicyError(format!(
                "rule {rule_id}: unknown severity axis '{name}' in condition '{key}'; \
                 known: {SEVERITY_INPUTS:?}"
            )));
        }
        let threshold_rank = expected
            .as_str()
            .and_then(severity_rank)
            .ok_or_else(|| {
                PolicyError(format!(
                    "rule {rule_id}: condition '{key}' threshold {expected} not in \
                     {SEVERITY_ORDER:?}"
                ))
            })?;
        return Ok(Condition::SeverityAbove {
            name: name.to_string(),
            threshold_rank,
        });
    }
    if BOOLEAN_INPUTS.contains(&key) {
        let expected = expected.as_bool().ok_or_else(|| {
            PolicyError(format!(
                "rule {rule_id}: condition '{key}' expects true/false, got {expected}"
            ))
        })?;
        return Ok(Condition::Flag {
            name: key.to_string(),
            expected,
        });
    }
    Err(PolicyError(format!(
        "rule {rule_id}: unknown condition key '{key}'; known: {BOOLEAN_INPUTS:?} \
         plus *_gte over {COUNTER_INPUTS:?} and *_above over {SEVERITY_INPUTS:?}"
    )))
}

/// Runtime half of the vocabulary. Absent (or null) inputs never satisfy
/// a condition; present-but-unreadable inputs return Err so `evaluate`
/// parks instead of coercing (law 0001). Presence requirements belong to
/// the result schemas — the evaluator only guarantees that absence is
/// never an advantage.
fn conditions_met(when: &[Condition], inputs: &Map<String, Value>) -> Result<bool, String> {
    for condition in when {
        match condition {
            Condition::CounterGte { name, threshold } => {
                match inputs.get(name) {
                    None | Some(Value::Null) => return Ok(false),
                    Some(Value::Number(n)) => {
                        let actual = n.as_f64().ok_or_else(|| {
                            format!("{name} must be a number, got {n}")
                        })?;
                        if actual < *threshold {
                            return Ok(false);
                        }
                    }
                    Some(other) => return Err(format!("{name} must be a number, got {other}")),
                }
            }
            Condition::SeverityAbove {
                name,
                threshold_rank,
            } => match inputs.get(name) {
                None | Some(Value::Null) => return Ok(false),
                Some(Value::String(s)) => match severity_rank(s) {
                    Some(rank) if rank > *threshold_rank => {}
                    Some(_) => return Ok(false),
                    None => {
                        return Err(format!(
                            "{name} severity '{s}' not in {SEVERITY_ORDER:?}"
                        ))
                    }
                },
                Some(other) => {
                    return Err(format!("{name} severity {other} not in {SEVERITY_ORDER:?}"))
                }
            },
            Condition::Flag { name, expected } => match inputs.get(name) {
                None | Some(Value::Null) => return Ok(false),
                Some(Value::Bool(actual)) => {
                    if actual != expected {
                        return Ok(false);
                    }
                }
                Some(other) => return Err(format!("{name} must be a boolean, got {other}")),
            },
        }
    }
    Ok(true)
}
