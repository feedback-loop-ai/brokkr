//! Why a table does not load, and why the machine refuses to rule (#353):
//! each refusal is a variant, and its `Display` is the text the operator
//! and the journal have always read, byte for byte.

use std::fmt;

use thiserror::Error;

use super::{
    BOOLEAN_INPUTS, COUNTER_INPUTS, RULING_SEVERITIES, SEVERITY_INPUTS, SEVERITY_ORDER, STRATEGIES,
    TABLE_SCHEMA_V2, VISIT_PREFIX,
};

/// A key a table holds at its top level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKey {
    Phases,
    Initial,
    Terminal,
    ShippableFrom,
    Rules,
}

impl TableKey {
    /// The four keys every table must hold.
    pub(crate) const REQUIRED: [TableKey; 4] = [
        TableKey::Phases,
        TableKey::Initial,
        TableKey::Terminal,
        TableKey::Rules,
    ];

    /// The key as a table spells it.
    pub(crate) fn field(self) -> &'static str {
        match self {
            TableKey::Phases => "phases",
            TableKey::Initial => "initial",
            TableKey::Terminal => "terminal",
            TableKey::ShippableFrom => "shippable_from",
            TableKey::Rules => "rules",
        }
    }
}

/// A key a rule may hold: the closed rule vocabulary of a `v2` table, as
/// `contracts/phase-machine.v2.schema.json` declares it. A misspelt rule
/// key would otherwise drop an artifact gate or a condition in silence.
/// `v1` stays open: the frozen production table carries annotation keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKey {
    Id,
    From,
    Result,
    Next,
    Park,
    Severity,
    RequiresArtifacts,
    Reason,
    When,
}

impl RuleKey {
    pub(crate) const ALL: [RuleKey; 9] = [
        RuleKey::Id,
        RuleKey::From,
        RuleKey::Result,
        RuleKey::Next,
        RuleKey::Park,
        RuleKey::Severity,
        RuleKey::RequiresArtifacts,
        RuleKey::Reason,
        RuleKey::When,
    ];

    /// The key as a rule spells it.
    pub(crate) fn field(self) -> &'static str {
        match self {
            RuleKey::Id => "id",
            RuleKey::From => "from",
            RuleKey::Result => "result",
            RuleKey::Next => "next",
            RuleKey::Park => "park",
            RuleKey::Severity => "severity",
            RuleKey::RequiresArtifacts => "requires_artifacts",
            RuleKey::Reason => "reason",
            RuleKey::When => "when",
        }
    }
}

/// Where in a table a list was expected.
#[derive(Debug, PartialEq, Eq)]
pub enum Place {
    /// A list the table itself holds: `phases`, `terminal`,
    /// `shippable_from` or `rules`.
    Table(TableKey),
    /// A rule's artifact gate.
    Artifacts { rule: String },
    /// The values of a rule's enumeration condition.
    Condition { rule: String, key: String },
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            Place::Table(key) => key.field().to_string(),
            Place::Artifacts { rule } => format!("rule {rule} requires_artifacts"),
            Place::Condition { rule, key } => format!("rule {rule} condition '{key}'"),
        })
    }
}

/// A rule's id as a refusal prints it: `?` where it has none.
fn unnamed(rule: &Option<String>) -> &str {
    rule.as_deref().unwrap_or("?")
}

/// The schema a table declares, as a refusal prints it.
fn declared(schema: &Option<String>) -> &str {
    schema.as_deref().unwrap_or("no schema")
}

/// Why a table's structure or closed vocabulary does not parse. A value
/// the table wrote is carried as its JSON text.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum Malformed {
    #[error("table must be an object")]
    NotAnObject,
    #[error("table missing '{}'", .0.field())]
    MissingKey(TableKey),
    #[error("{0} must be an array")]
    NotAnArray(Place),
    #[error("{0} entries must be strings")]
    NotStrings(Place),
    #[error("initial must be a string")]
    InitialNotAString,
    #[error("initial phase not in phases")]
    InitialUnknown,
    #[error("terminal phase '{0}' not in phases")]
    TerminalUnknown(String),
    #[error("rule must be an object")]
    RuleNotAnObject,
    #[error("duplicate rule id {0}")]
    DuplicateId(String),
    #[error(
        "rule {rule} is unreachable: an unconditional rule for ({from}, {result}) precedes it \
         and first match wins"
    )]
    Unreachable {
        rule: String,
        from: String,
        result: String,
    },
    /// A rule without a required field; `rule` is `None` when the
    /// missing field is the id itself, or the id is not a string.
    #[error("rule {} missing '{}'", unnamed(.rule), .key.field())]
    MissingField { rule: Option<String>, key: RuleKey },
    #[error("rule {rule} declares '{key}', which is not {TABLE_SCHEMA_V2} rule vocabulary")]
    UnknownRuleKey { rule: String, key: String },
    #[error(
        "rule {rule}: 'park' must be true when present, got {written}; a park is a ruling, \
         not a switch to leave off"
    )]
    ParkNotTrue { rule: String, written: String },
    #[error("rule {0} both parks and names a next phase; a parked run takes no transition")]
    ParkAndNext(String),
    #[error(
        "rule {rule} parks, which is {TABLE_SCHEMA_V2} vocabulary, but the table declares {}",
        declared(.schema)
    )]
    ParkOutsideV2 {
        rule: String,
        schema: Option<String>,
    },
    #[error(
        "rule {rule} parks and declares '{}'; a park takes no transition, so it has \
         neither a ruling severity nor an artifact gate",
        .key.field()
    )]
    ParkDeclares { rule: String, key: RuleKey },
    #[error("rule {0} references unknown phase")]
    UnknownPhase(String),
    #[error("rule {rule} leaves terminal phase '{phase}'")]
    LeavesTerminal { rule: String, phase: String },
    #[error("rule {0} severity must be a string")]
    SeverityNotAString(String),
    #[error("rule {rule} severity '{severity}' not in {RULING_SEVERITIES:?}")]
    UnknownSeverity { rule: String, severity: String },
    #[error("rule {0} 'when' must be an object")]
    WhenNotAnObject(String),
    #[error(
        "rule {rule}: identifier input '{key}' may be declared by a seat but never used as a \
         condition key"
    )]
    IdentifierCondition { rule: String, key: String },
    #[error("rule {rule}: condition '{key}' needs at least one value")]
    NoValues { rule: String, key: String },
    #[error("rule {rule}: condition '{key}' value '{value}' not in {vocabulary:?}")]
    OutsideVocabulary {
        rule: String,
        key: String,
        value: String,
        vocabulary: &'static [&'static str],
    },
    #[error(
        "rule {rule}: unknown counter '{name}' in condition '{key}'; known: \
         {COUNTER_INPUTS:?} plus '{VISIT_PREFIX}<phase>' over this table's phases {phases:?}"
    )]
    UnknownCounter {
        rule: String,
        name: String,
        key: String,
        phases: Vec<String>,
    },
    #[error("rule {rule}: condition '{key}' needs a numeric threshold, got {written}")]
    NotNumeric {
        rule: String,
        key: String,
        written: String,
    },
    #[error(
        "rule {rule}: unknown severity axis '{name}' in condition '{key}'; known: \
         {SEVERITY_INPUTS:?}"
    )]
    UnknownAxis {
        rule: String,
        name: String,
        key: String,
    },
    #[error("rule {rule}: condition '{key}' threshold {written} not in {SEVERITY_ORDER:?}")]
    Unranked {
        rule: String,
        key: String,
        written: String,
    },
    #[error("rule {rule}: condition '{key}' expects true/false, got {written}")]
    NotBoolean {
        rule: String,
        key: String,
        written: String,
    },
    #[error(
        "rule {rule}: unknown condition key '{key}'; known: {BOOLEAN_INPUTS:?} plus \
         strategy_in over {STRATEGIES:?}, *_gte over {COUNTER_INPUTS:?} and \
         *_above/*_at_most over {SEVERITY_INPUTS:?}"
    )]
    UnknownCondition { rule: String, key: String },
}

/// Why the machine refuses to rule (law 0001): the phase is not the
/// table's, or a present input is one its vocabulary cannot read. `name`
/// is the input; a value it was given is carried as its JSON text.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Unreadable {
    #[error("unknown phase '{0}'")]
    UnknownPhase(String),
    #[error("{name} must be a number, got {written}")]
    NotANumber { name: String, written: String },
    #[error("{name} must be a boolean, got {written}")]
    NotABoolean { name: String, written: String },
    #[error("{name} must be a string, got {written}")]
    NotAString { name: String, written: String },
    #[error("{name} severity '{word}' not in {SEVERITY_ORDER:?}")]
    UnrankedSeverity { name: String, word: String },
    #[error("{name} severity {written} not in {SEVERITY_ORDER:?}")]
    SeverityNotAWord { name: String, written: String },
    #[error("{name} '{word}' not in {vocabulary:?}")]
    OutsideVocabulary {
        name: String,
        word: String,
        vocabulary: &'static [&'static str],
    },
}
