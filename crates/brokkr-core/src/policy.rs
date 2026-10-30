//! Strict policy evaluation — the Rust port of the heritage Python machine
//! under decisions 0001/0002/0004, differential-tested against the
//! committed oracle corpus (`fixtures/evaluator/corpus.ndjson`).
//!
//! Laws carried over exactly:
//! - first matching rule wins, in table order;
//! - the condition vocabulary is closed and enforced at load
//!   (`PolicyError`), so a typo'd key can never silently deaden a rule;
//! - an absent (or null) input never satisfies a condition;
//! - a present input the vocabulary cannot read parks the evaluation
//!   (`Outcome::Refused` with its `problem`) — never coerced, never guessed;
//! - an unmatched (phase, result) pair parks as `Outcome::Unmatched`.

use serde_json::{Map, Value};
use thiserror::Error;

/// Residual-finding severity axis, lowest to highest — the value
/// vocabulary of `max_residual_severity`. Distinct from ruling severity.
pub const SEVERITY_ORDER: [&str; 6] = ["none", "info", "low", "medium", "high", "critical"];

/// A residual finding's severity, typed: the closed set [`SEVERITY_ORDER`]
/// names, ordered as it orders them. It serializes as its name.
#[derive(serde::Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    None,
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Every severity, lowest first, at its [`SEVERITY_ORDER`] rank.
    const ALL: [Severity; 6] = [
        Severity::None,
        Severity::Info,
        Severity::Low,
        Severity::Medium,
        Severity::High,
        Severity::Critical,
    ];

    /// The severity `name` names, or `None` for a word outside the set.
    pub fn named(name: &str) -> Option<Severity> {
        severity_rank(name).map(|rank| Severity::ALL[rank])
    }

    /// The name the table and every surface write for it.
    pub fn name(self) -> &'static str {
        SEVERITY_ORDER[self as usize]
    }
}

/// Ruling severity axis — the phase-event/v1 vocabulary.
pub const RULING_SEVERITIES: [&str; 3] = ["normal", "flagged", "hard"];

/// A ruling's severity, typed: the closed set [`RULING_SEVERITIES`]
/// names, in its order. It reads and serializes as its name; the read is
/// [`crate::decision`]'s, beside the record that holds it.
#[derive(serde::Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Normal,
    Flagged,
    Hard,
}

impl RuleSeverity {
    /// Every ruling severity, at its [`RULING_SEVERITIES`] place.
    const ALL: [RuleSeverity; 3] = [
        RuleSeverity::Normal,
        RuleSeverity::Flagged,
        RuleSeverity::Hard,
    ];

    /// The severity `name` names, or `None` for a word outside the set.
    pub fn named(name: &str) -> Option<RuleSeverity> {
        let place = RULING_SEVERITIES.iter().position(|word| *word == name)?;
        Some(RuleSeverity::ALL[place])
    }

    /// The word the journal writes for it.
    pub fn label(self) -> &'static str {
        RULING_SEVERITIES[self as usize]
    }
}

/// Delivery classes ruled by the triage chief (decision 0041 ruling 6).
/// This is both the result vocabulary of that office and the value
/// vocabulary of the engine-owned `strategy` input.
pub const STRATEGIES: [&str; 5] = ["chore", "feature", "design", "engine", "escalate"];

/// The phase whose ruling, when it is one of [`STRATEGIES`], sets the
/// run's `strategy`.
pub(crate) const STRATEGY_PHASE: &str = "triage";

/// The table format this evaluator reads. `v2` adds exactly one thing to
/// `v1` (decision 0022): a rule may rule a PARK instead of a transition.
/// A `v1` table is still read as it always was; a table that parks must
/// say so in its own `schema`, because a park is not a stop and the
/// difference is the whole point of the ruling.
pub const TABLE_SCHEMA_V1: &str = "forge.phase-machine/v1";
pub const TABLE_SCHEMA_V2: &str = "forge.phase-machine/v2";

/// Condition prefix of the phase-visit predicate (decision 0022):
/// `visits_<phase>_gte` reads how many times the run has entered
/// `<phase>`. Engine-owned like every other counter — the journal counts
/// visits, a seat never claims one.
pub const VISIT_PREFIX: &str = "visits_";

/// The boolean inputs, each read by the condition keyed by its name.
pub const BOOLEAN_INPUTS: [&str; count(Pick::Flags)] = names(Pick::Flags);
pub const COUNTER_INPUTS: [&str; count(Pick::Counters)] = names(Pick::Counters);
pub const SEVERITY_INPUTS: [&str; count(Pick::Severities)] = names(Pick::Severities);
/// Typed handoff identifiers. Seats may declare these, but policy may never
/// branch on them: identity is data passed to effects, not a control signal.
pub const IDENTIFIER_INPUTS: [&str; count(Pick::Identifiers)] = names(Pick::Identifiers);

/// Inputs the engine owns. A seat may never supply or declare these:
/// journal-computed truth is never accepted from a caller (README law 2).
pub const ENGINE_OWNED_INPUTS: [&str; count(Pick::EngineOwned)] = names(Pick::EngineOwned);

/// The protected phase's record: realm name (or the legacy unkeyed
/// `repo`, per [`crate::realms::LEGACY_REALM_KEY`]) to observed head.
pub(crate) const REVIEWED_HEADS: &str = "reviewed_heads";

/// The per-realm repository facts a decision records in a mapped world:
/// realm name -> observed HEAD, dirty worktree, drift.
pub const REALM_FACTS: &str = "realm_facts";

/// The same law over the phase-visit family (decision 0022): every
/// `visits_<phase>` is counted by the fold from `phase/entered` events,
/// so no seat may declare one and no seat may claim one. Presence
/// (decision 0050, ruling 1) exempts exactly these inputs.
pub fn is_engine_owned(name: &str) -> bool {
    ENGINE_OWNED_INPUTS.contains(&name) || name.starts_with(VISIT_PREFIX)
}

/// Artifact ownership reported by the read-only analysis judge. Unlike an
/// identifier, this is a closed enum which policy may branch on.
pub const DRIFT_PHASES: [&str; 3] = ["specify", "design", "tasks"];

pub fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum PolicyError {
    /// The table's structure or closed vocabulary does not parse.
    #[error("malformed phase machine table: {0}")]
    Malformed(Malformed),
    /// The table parses, and holds a finding decision 0050 refuses at load
    /// (`Machine::refuse_findings`).
    #[error("malformed phase machine table: {0}")]
    Refused(audit::Refusal),
}

/// By hand, not `#[from]`: `#[from]` makes the fault the error's source,
/// and a `{:#}` chain would print its text a second time on stderr.
impl From<Malformed> for PolicyError {
    fn from(fault: Malformed) -> Self {
        PolicyError::Malformed(fault)
    }
}

fn severity_rank(name: &str) -> Option<usize> {
    SEVERITY_ORDER.iter().position(|s| *s == name)
}

#[derive(Debug, Clone, PartialEq)]
enum Condition {
    CounterGte { name: String, threshold: f64 },
    SeverityAbove { name: String, threshold_rank: usize },
    SeverityAtMost { name: String, threshold_rank: usize },
    Flag { name: String, expected: bool },
    EnumIn { name: String, allowed: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub from: String,
    pub result: String,
    /// The phase this rule advances to, or `None` when the rule rules a
    /// PARK (decision 0022, table schema v2): the run stops running and
    /// waits for the operator, and the journal says a rule put it there.
    pub next: Option<String>,
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
    /// A rule ruled a PARK (decision 0022): the run awaits the operator
    /// with this rule's reason. Distinct from the two below — the machine
    /// DID rule; the ruling was "this one is yours".
    Park { rule_id: String, reason: String },
    /// No rule matched: no ruling is possible, and the engine MUST park
    /// (law 0001).
    Unmatched,
    /// The machine refused to rule (unknown phase, unreadable input), and
    /// the engine MUST park (law 0001).
    Refused { problem: Unreadable },
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
    /// Parse a whole table, then refuse what decision 0050 refuses at load
    /// (`Machine::refuse_findings`).
    pub fn from_table(table: &Value) -> Result<Machine, PolicyError> {
        let machine = Machine::parse(table)?;
        machine.refuse_findings(
            table.get("schema").and_then(Value::as_str) == Some(TABLE_SCHEMA_V2),
        )?;
        Ok(machine)
    }

    /// The table's structure and closed vocabulary, rule by rule. A
    /// recipe's overlay is not a whole table, so its unit tests read it
    /// here.
    fn parse(table: &Value) -> Result<Machine, PolicyError> {
        let obj = table.as_object().ok_or(Malformed::NotAnObject)?;
        let (phases, initial, terminal) = parse_header(obj)?;
        let shippable_from = match obj.get(TableKey::ShippableFrom.field()) {
            Some(v) => string_array(v, Place::Table(TableKey::ShippableFrom))?,
            None => Vec::new(),
        };
        let schema = obj.get("schema").and_then(Value::as_str);
        let rules = parse_rules(&obj[TableKey::Rules.field()], &phases, &terminal, schema)?;
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
            return Outcome::Refused {
                problem: Unreadable::UnknownPhase(phase.to_string()),
            };
        }
        for rule in &self.rules {
            if rule.from != phase || rule.result != result {
                continue;
            }
            match conditions_met(&rule.when, inputs) {
                Ok(false) => continue,
                Ok(true) => {
                    return match &rule.next {
                        Some(next_phase) => Outcome::Ruling {
                            rule_id: rule.id.clone(),
                            next_phase: next_phase.clone(),
                            severity: rule.severity.clone(),
                            reason: rule.reason.clone(),
                            requires_artifacts: rule.requires_artifacts.clone(),
                        },
                        None => Outcome::Park {
                            rule_id: rule.id.clone(),
                            reason: rule.reason.clone(),
                        },
                    }
                }
                Err(problem) => return Outcome::Refused { problem },
            }
        }
        Outcome::Unmatched
    }

    /// The phases whose visit counts the rules of `from` read (decision
    /// 0022) — exactly the `visits_<phase>` facts the engine must supply
    /// when it decides a result from `from`, and no others: a table that
    /// asks nothing about visits journals nothing about them.
    pub fn visit_phases(&self, from: &str) -> Vec<String> {
        self.reads(from)
            .into_iter()
            .filter_map(|condition| condition.name().strip_prefix(VISIT_PREFIX))
            .map(str::to_string)
            .collect()
    }

    /// Whether rules leaving `from` read one engine-owned counter. Runtime
    /// uses this to compute strategy-local failure words without changing
    /// unrelated journals that happen to use the same word.
    pub fn reads_counter(&self, from: &str, counter: &str) -> bool {
        self.reads(from).into_iter().any(
            |condition| matches!(condition, Condition::CounterGte { name, .. } if name == counter),
        )
    }

    /// The seat-supplied inputs the rules leaving `from` read, sorted: a
    /// seat's default (and minimum) declaration (decision 0007).
    pub fn seat_inputs(&self, from: &str) -> Vec<String> {
        let mut names: Vec<String> = self
            .reads(from)
            .into_iter()
            .map(Condition::name)
            .filter(|name| !is_engine_owned(name))
            .map(str::to_string)
            .collect();
        names.sort();
        names
    }

    /// The first condition on each input the rules leaving `from` read,
    /// in table order: the one projection the three above are read from.
    /// The grammar binds a name to one kind, so the first is its kind.
    fn reads(&self, from: &str) -> Vec<&Condition> {
        let mut read: Vec<&Condition> = Vec::new();
        for rule in self.rules.iter().filter(|rule| rule.from == from) {
            for condition in &rule.when {
                if !read.iter().any(|seen| seen.name() == condition.name()) {
                    read.push(condition);
                }
            }
        }
        read
    }
}

fn string_array(value: &Value, place: Place) -> Result<Vec<String>, Malformed> {
    let Some(entries) = value.as_array() else {
        return Err(Malformed::NotAnArray(place));
    };
    entries
        .iter()
        .map(|v| v.as_str().map(str::to_string))
        .collect::<Option<Vec<String>>>()
        .ok_or(Malformed::NotStrings(place))
}

/// A table's header: the four keys it must hold, its phases, the initial
/// phase among them and its terminal phases among them.
fn parse_header(obj: &Map<String, Value>) -> Result<(Vec<String>, String, Vec<String>), Malformed> {
    for key in TableKey::REQUIRED {
        if !obj.contains_key(key.field()) {
            return Err(Malformed::MissingKey(key));
        }
    }
    let phases = string_array(
        &obj[TableKey::Phases.field()],
        Place::Table(TableKey::Phases),
    )?;
    let initial = obj[TableKey::Initial.field()]
        .as_str()
        .ok_or(Malformed::InitialNotAString)?
        .to_string();
    if !phases.contains(&initial) {
        return Err(Malformed::InitialUnknown);
    }
    let terminal = string_array(
        &obj[TableKey::Terminal.field()],
        Place::Table(TableKey::Terminal),
    )?;
    for t in &terminal {
        if !phases.contains(t) {
            return Err(Malformed::TerminalUnknown(t.clone()));
        }
    }
    Ok((phases, initial, terminal))
}

/// A table's rules, in order: each parsed, no id twice, and none behind an
/// unconditional rule of its own group, since first match wins.
fn parse_rules(
    raw_rules: &Value,
    phases: &[String],
    terminal: &[String],
    schema: Option<&str>,
) -> Result<Vec<Rule>, Malformed> {
    let raw_rules = raw_rules
        .as_array()
        .ok_or(Malformed::NotAnArray(Place::Table(TableKey::Rules)))?;
    let mut rules = Vec::with_capacity(raw_rules.len());
    let mut seen_ids: Vec<String> = Vec::new();
    let mut ruled_unconditionally: Vec<(String, String)> = Vec::new();
    for raw in raw_rules {
        let rule = parse_rule(raw, phases, terminal, schema)?;
        if seen_ids.contains(&rule.id) {
            return Err(Malformed::DuplicateId(rule.id));
        }
        seen_ids.push(rule.id.clone());
        let group = (rule.from.clone(), rule.result.clone());
        if ruled_unconditionally.contains(&group) {
            return Err(Malformed::Unreachable {
                rule: rule.id,
                from: rule.from,
                result: rule.result,
            });
        }
        if rule.when.is_empty() {
            ruled_unconditionally.push(group);
        }
        rules.push(rule);
    }
    Ok(rules)
}

/// One rule: its identity, its movement, its gate and its conditions, in
/// the order the table has always checked them.
fn parse_rule(
    raw: &Value,
    phases: &[String],
    terminal: &[String],
    schema: Option<&str>,
) -> Result<Rule, Malformed> {
    let obj = raw.as_object().ok_or(Malformed::RuleNotAnObject)?;
    let id = rule::field(obj, RuleKey::Id)?;
    let from = rule::field(obj, RuleKey::From)?;
    let result = rule::field(obj, RuleKey::Result)?;
    let reason = rule::field(obj, RuleKey::Reason)?;
    rule::check_keys(obj, &id, schema)?;
    let parks = rule::parse_park(obj, &id)?;
    let next = rule::parse_next(obj, &id, parks)?;
    if parks {
        rule::check_park(obj, &id, schema)?;
    }
    rule::check_phases(&id, &from, next.as_ref(), phases, terminal)?;
    let severity = rule::parse_severity(obj, &id)?;
    let requires_artifacts = rule::parse_requires_artifacts(obj, &id)?;
    let when = rule::parse_when(obj, &id, phases)?;
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
fn parse_condition(
    rule_id: &str,
    key: &str,
    expected: &Value,
    phases: &[String],
) -> Result<Condition, Malformed> {
    if conditions::identifier_condition(key) {
        return Err(Malformed::IdentifierCondition {
            rule: rule_id.to_string(),
            key: key.to_string(),
        });
    }
    if let Some(condition) = conditions::parse_choice(rule_id, key, expected)? {
        return Ok(condition);
    }
    if let Some(name) = key.strip_suffix("_gte") {
        return conditions::parse_counter(rule_id, key, name, expected, phases);
    }
    if let Some(name) = key.strip_suffix("_above") {
        return conditions::parse_severity_above(rule_id, key, name, expected);
    }
    if let Some(name) = key.strip_suffix("_at_most") {
        return conditions::parse_severity_at_most(rule_id, key, name, expected);
    }
    if BOOLEAN_INPUTS.contains(&key) {
        return conditions::parse_flag(rule_id, key, expected);
    }
    Err(Malformed::UnknownCondition {
        rule: rule_id.to_string(),
        key: key.to_string(),
    })
}

/// Runtime half of the vocabulary. Absent (or null) inputs never satisfy
/// a condition; present-but-unreadable inputs return Err so `evaluate`
/// parks instead of coercing (law 0001). Presence requirements belong to
/// the result schemas — the evaluator only guarantees that absence is
/// never an advantage.
fn conditions_met(when: &[Condition], inputs: &Map<String, Value>) -> Result<bool, Unreadable> {
    for condition in when {
        if !conditions::condition_met(condition, inputs)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// A severity input's rank, or `None` when it is absent (or null). A
/// present word outside [`SEVERITY_ORDER`], or a value that is not a
/// word, is refused rather than ranked.
fn severity_of(name: &str, value: Option<&Value>) -> Result<Option<usize>, Unreadable> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(word)) => {
            severity_rank(word)
                .map(Some)
                .ok_or_else(|| Unreadable::UnrankedSeverity {
                    name: name.to_string(),
                    word: word.clone(),
                })
        }
        Some(other) => Err(Unreadable::SeverityNotAWord {
            name: name.to_string(),
            written: other.to_string(),
        }),
    }
}
pub mod audit;
mod conditions;
mod direction;
pub use direction::Direction;
mod errors;
mod inputs;
pub(crate) use inputs::STRATEGY;
mod rule;
mod words;

pub use errors::{Malformed, Place, RuleKey, TableKey, Unreadable};
use inputs::{choice_keyed, count, names, vocabulary, Pick};
pub use inputs::{is_declarable, CONSECUTIVE_FAILURES};

#[cfg(test)]
mod tests;
