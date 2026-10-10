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

/// Delivery classes ruled by the triage chief (decision 0041 ruling 6).
/// This is both the result vocabulary of that office and the value
/// vocabulary of the engine-owned `strategy` input.
pub const STRATEGIES: [&str; 5] = ["chore", "feature", "design", "engine", "escalate"];

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

pub const BOOLEAN_INPUTS: [&str; 8] = [
    "skip_verify",
    "fixes_applied",
    // Decision 0041: a review panel may return a defective specification
    // to design. Seat-declarable; unlike visit counts, the judge observes it.
    "spec_defect",
    "has_security_residual",
    "high_risk_uncovered",
    "drift_detected",
    "dirty_worktrees",
    // Decision 0039: every commit the protected phase added since it
    // was entered lies in the repository's docs class. Engine-owned,
    // like the two above it.
    "fixes_docs_only",
];
pub const COUNTER_INPUTS: [&str; 1] = ["consecutive_failures"];
pub const SEVERITY_INPUTS: [&str; 1] = ["max_residual_severity"];
/// Typed handoff identifiers. Seats may declare these, but policy may never
/// branch on them: identity is data passed to effects, not a control signal.
pub const IDENTIFIER_INPUTS: [&str; 1] = ["change"];

/// Inputs the engine owns. A seat may never supply or declare these:
/// journal-computed truth is never accepted from a caller (README law 2).
pub const ENGINE_OWNED_INPUTS: [&str; 7] = [
    "consecutive_failures",
    "drift_detected",
    "dirty_worktrees",
    REVIEWED_HEADS,
    // The fold remembers the last successful triage result. A seat may
    // neither declare nor overwrite the class that governs its run.
    "strategy",
    // Read from the tree at the protected phase's ruling (decision
    // 0039): the review's own commits, classified by the repository's
    // declared docs class.
    "fixes_docs_only",
    // The same repository facts, keyed by realm (decision 0023). Read
    // from the tree by the engine, exactly like the two above it.
    REALM_FACTS,
];

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
        let mut names: Vec<String> = Vec::new();
        for rule in self.rules.iter().filter(|rule| rule.from == from) {
            for condition in &rule.when {
                let Condition::CounterGte { name, .. } = condition else {
                    continue;
                };
                let Some(phase) = name.strip_prefix(VISIT_PREFIX) else {
                    continue;
                };
                if !names.iter().any(|known| known == phase) {
                    names.push(phase.to_string());
                }
            }
        }
        names
    }

    /// Whether rules leaving `from` read one engine-owned counter. Runtime
    /// uses this to compute strategy-local failure words without changing
    /// unrelated journals that happen to use the same word.
    pub fn reads_counter(&self, from: &str, counter: &str) -> bool {
        self.rules.iter().filter(|rule| rule.from == from).any(|rule| {
            rule.when.iter().any(
                |condition| matches!(condition, Condition::CounterGte { name, .. } if name == counter),
            )
        })
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

#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn parse_rule(
    raw: &Value,
    phases: &[String],
    terminal: &[String],
    schema: Option<&str>,
) -> Result<Rule, Malformed> {
    let obj = raw.as_object().ok_or(Malformed::RuleNotAnObject)?;
    let get = |key: RuleKey| obj.get(key.field());
    let field = |key: RuleKey| -> Result<String, Malformed> {
        get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| Malformed::MissingField {
                rule: get(RuleKey::Id).and_then(Value::as_str).map(str::to_string),
                key,
            })
    };
    let id = field(RuleKey::Id)?;
    let from = field(RuleKey::From)?;
    let result = field(RuleKey::Result)?;
    let reason = field(RuleKey::Reason)?;
    if schema == Some(TABLE_SCHEMA_V2) {
        let known = |key: &String| RuleKey::ALL.iter().any(|k| k.field() == key);
        if let Some(key) = obj.keys().find(|key| !known(key)) {
            return Err(Malformed::UnknownRuleKey {
                rule: id,
                key: key.clone(),
            });
        }
    }
    // A rule either advances the run or parks it — never both, never
    // neither. `park` is v2 vocabulary and the table must declare it:
    // a park read out of a table calling itself v1 would be a ruling
    // nobody reviewed (decision 0022).
    let parks = match get(RuleKey::Park) {
        None => false,
        Some(Value::Bool(true)) => true,
        Some(other) => {
            return Err(Malformed::ParkNotTrue {
                rule: id,
                written: other.to_string(),
            })
        }
    };
    let next = match (parks, get(RuleKey::Next)) {
        (false, _) => Some(field(RuleKey::Next)?),
        (true, None) => None,
        (true, Some(_)) => return Err(Malformed::ParkAndNext(id)),
    };
    if parks {
        if schema != Some(TABLE_SCHEMA_V2) {
            return Err(Malformed::ParkOutsideV2 {
                rule: id,
                schema: schema.map(str::to_string),
            });
        }
        for forbidden in [RuleKey::Severity, RuleKey::RequiresArtifacts] {
            if get(forbidden).is_some() {
                return Err(Malformed::ParkDeclares {
                    rule: id,
                    key: forbidden,
                });
            }
        }
    }
    if !phases.contains(&from) || next.as_ref().is_some_and(|next| !phases.contains(next)) {
        return Err(Malformed::UnknownPhase(id));
    }
    if terminal.contains(&from) {
        return Err(Malformed::LeavesTerminal {
            rule: id,
            phase: from,
        });
    }
    let severity = match get(RuleKey::Severity) {
        None => "normal".to_string(),
        Some(v) => {
            let s = v
                .as_str()
                .ok_or_else(|| Malformed::SeverityNotAString(id.clone()))?;
            if !RULING_SEVERITIES.contains(&s) {
                return Err(Malformed::UnknownSeverity {
                    rule: id,
                    severity: s.to_string(),
                });
            }
            s.to_string()
        }
    };
    let requires_artifacts = match get(RuleKey::RequiresArtifacts) {
        None => Vec::new(),
        Some(v) => string_array(v, Place::Artifacts { rule: id.clone() })?,
    };
    let when = match get(RuleKey::When) {
        None => Vec::new(),
        Some(v) => {
            let map = v
                .as_object()
                .ok_or_else(|| Malformed::WhenNotAnObject(id.clone()))?;
            let mut conditions = Vec::with_capacity(map.len());
            for (key, expected) in map {
                conditions.push(parse_condition(&id, key, expected, phases)?);
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

/// A severity condition's threshold rank, once its axis is a declared
/// severity input: `_above` and `_at_most` read it alike.
fn severity_threshold(
    rule_id: &str,
    key: &str,
    name: &str,
    expected: &Value,
) -> Result<usize, Malformed> {
    if !SEVERITY_INPUTS.contains(&name) {
        return Err(Malformed::UnknownAxis {
            rule: rule_id.to_string(),
            name: name.to_string(),
            key: key.to_string(),
        });
    }
    expected
        .as_str()
        .and_then(severity_rank)
        .ok_or_else(|| Malformed::Unranked {
            rule: rule_id.to_string(),
            key: key.to_string(),
            written: expected.to_string(),
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
    let (rule, key_text) = (rule_id.to_string(), key.to_string());
    if IDENTIFIER_INPUTS.contains(&key)
        || key
            .strip_suffix("_in")
            .is_some_and(|name| IDENTIFIER_INPUTS.contains(&name))
    {
        return Err(Malformed::IdentifierCondition {
            rule,
            key: key_text,
        });
    }
    if key == "strategy_in" || key == "drift_in" {
        let place = Place::Condition {
            rule: rule.clone(),
            key: key_text.clone(),
        };
        let allowed = string_array(expected, place)?;
        if allowed.is_empty() {
            return Err(Malformed::NoValues {
                rule,
                key: key_text,
            });
        }
        let name = if key == "strategy_in" {
            "strategy"
        } else {
            "drift_in"
        };
        let vocabulary = vocabulary(name);
        for value in &allowed {
            if !vocabulary.contains(&value.as_str()) {
                return Err(Malformed::OutsideVocabulary {
                    rule,
                    key: key_text,
                    value: value.clone(),
                    vocabulary,
                });
            }
        }
        return Ok(Condition::EnumIn {
            name: name.to_string(),
            allowed,
        });
    }
    if let Some(name) = key.strip_suffix("_gte") {
        // Counters are the declared list plus one family: the phase-visit
        // predicate (decision 0022), whose suffix must name a phase THIS
        // table has — the vocabulary stays closed, it just closes over
        // the table's own graph.
        let visit_phase = name
            .strip_prefix(VISIT_PREFIX)
            .filter(|phase| phases.iter().any(|known| known == phase));
        if !COUNTER_INPUTS.contains(&name) && visit_phase.is_none() {
            return Err(Malformed::UnknownCounter {
                rule,
                name: name.to_string(),
                key: key_text,
                phases: phases.to_vec(),
            });
        }
        let threshold = expected.as_f64().ok_or_else(|| Malformed::NotNumeric {
            rule,
            key: key_text,
            written: expected.to_string(),
        })?;
        return Ok(Condition::CounterGte {
            name: name.to_string(),
            threshold,
        });
    }
    if let Some(name) = key.strip_suffix("_above") {
        let threshold_rank = severity_threshold(rule_id, key, name, expected)?;
        return Ok(Condition::SeverityAbove {
            name: name.to_string(),
            threshold_rank,
        });
    }
    if let Some(name) = key.strip_suffix("_at_most") {
        let threshold_rank = severity_threshold(rule_id, key, name, expected)?;
        return Ok(Condition::SeverityAtMost {
            name: name.to_string(),
            threshold_rank,
        });
    }
    if BOOLEAN_INPUTS.contains(&key) {
        let expected = expected.as_bool().ok_or_else(|| Malformed::NotBoolean {
            rule,
            key: key_text.clone(),
            written: expected.to_string(),
        })?;
        return Ok(Condition::Flag {
            name: key_text,
            expected,
        });
    }
    Err(Malformed::UnknownCondition {
        rule,
        key: key_text,
    })
}

/// Runtime half of the vocabulary. Absent (or null) inputs never satisfy
/// a condition; present-but-unreadable inputs return Err so `evaluate`
/// parks instead of coercing (law 0001). Presence requirements belong to
/// the result schemas — the evaluator only guarantees that absence is
/// never an advantage.
fn conditions_met(when: &[Condition], inputs: &Map<String, Value>) -> Result<bool, Unreadable> {
    for condition in when {
        match condition {
            Condition::CounterGte { name, threshold } => match inputs.get(name) {
                None | Some(Value::Null) => return Ok(false),
                Some(Value::Number(n)) => {
                    // serde_json rejects non-finite numbers at construction,
                    // so every Number is representable through this API.
                    let actual = n.as_f64().expect("JSON numbers are finite");
                    if actual < *threshold {
                        return Ok(false);
                    }
                }
                Some(other) => {
                    return Err(Unreadable::NotANumber {
                        name: name.clone(),
                        written: other.to_string(),
                    })
                }
            },
            Condition::SeverityAbove {
                name,
                threshold_rank,
            } => match severity_of(name, inputs.get(name))? {
                Some(rank) if rank > *threshold_rank => {}
                Some(_) | None => return Ok(false),
            },
            // Fail-closed at-most: an ABSENT severity is an unknown
            // severity and never qualifies — the ruling that ships as
            // debt must be earned by a value, not by silence (the
            // review that repealed the blind stop caught exactly this).
            Condition::SeverityAtMost {
                name,
                threshold_rank,
            } => match severity_of(name, inputs.get(name))? {
                Some(rank) if rank <= *threshold_rank => {}
                Some(_) | None => return Ok(false),
            },
            Condition::Flag { name, expected } => match inputs.get(name) {
                None | Some(Value::Null) => return Ok(false),
                Some(Value::Bool(actual)) => {
                    if actual != expected {
                        return Ok(false);
                    }
                }
                Some(other) => {
                    return Err(Unreadable::NotABoolean {
                        name: name.clone(),
                        written: other.to_string(),
                    })
                }
            },
            Condition::EnumIn { name, allowed } => match inputs.get(name) {
                None | Some(Value::Null) => return Ok(false),
                Some(Value::String(actual)) if !vocabulary(name).contains(&actual.as_str()) => {
                    return Err(Unreadable::OutsideVocabulary {
                        name: name.clone(),
                        word: actual.clone(),
                        vocabulary: vocabulary(name),
                    });
                }
                Some(Value::String(actual)) => {
                    if !allowed.contains(actual) {
                        return Ok(false);
                    }
                }
                Some(other) => {
                    return Err(Unreadable::NotAString {
                        name: name.clone(),
                        written: other.to_string(),
                    })
                }
            },
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

/// The closed vocabulary an enumeration input's value is drawn from:
/// `strategy` or `drift_in`, the two names `parse_condition` gives one.
fn vocabulary(name: &str) -> &'static [&'static str] {
    if name == "strategy" {
        &STRATEGIES
    } else {
        &DRIFT_PHASES
    }
}

pub mod audit;
mod errors;

pub use errors::{Malformed, Place, RuleKey, TableKey, Unreadable};

#[cfg(test)]
mod tests;
