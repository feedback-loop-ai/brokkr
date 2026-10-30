//! A rule's `when` vocabulary, one helper per condition kind (#288):
//! the load-time half that refuses a word outside the closed set, and
//! the runtime half that parks on an input it cannot read. `parse_rule`
//! still assembles the list and `conditions_met` still folds it, in the
//! order the table checks it.

use serde_json::{Map, Value};

use super::{
    choice_keyed, severity_of, severity_rank, string_array, vocabulary, Condition, Malformed,
    Place, Unreadable, COUNTER_INPUTS, IDENTIFIER_INPUTS, SEVERITY_INPUTS, VISIT_PREFIX,
};

/// Which side of a severity threshold a condition reads: `_above` and
/// `_at_most` differ only in this, at load and at evaluation alike.
#[derive(Clone, Copy)]
pub(super) enum SeverityBound {
    Above,
    AtMost,
}

impl SeverityBound {
    /// Does a rank on this side of the threshold hold?
    fn holds(self, rank: usize, threshold_rank: usize) -> bool {
        match self {
            SeverityBound::Above => rank > threshold_rank,
            SeverityBound::AtMost => rank <= threshold_rank,
        }
    }
}

/// Is `key` a handoff identity the policy may never read? A condition
/// names one directly, or through its `_in` form.
pub(super) fn identifier_condition(key: &str) -> bool {
    IDENTIFIER_INPUTS.contains(&key)
        || key
            .strip_suffix("_in")
            .is_some_and(|name| IDENTIFIER_INPUTS.contains(&name))
}

/// A choice condition's allowed words, or `None` when `key` is not a
/// choice condition at all.
pub(super) fn parse_choice(
    rule_id: &str,
    key: &str,
    expected: &Value,
) -> Result<Option<Condition>, Malformed> {
    let Some((name, vocabulary)) = choice_keyed(key) else {
        return Ok(None);
    };
    let place = Place::Condition {
        rule: rule_id.to_string(),
        key: key.to_string(),
    };
    let allowed = string_array(expected, place)?;
    if allowed.is_empty() {
        return Err(Malformed::NoValues {
            rule: rule_id.to_string(),
            key: key.to_string(),
        });
    }
    for value in &allowed {
        if !vocabulary.contains(&value.as_str()) {
            return Err(Malformed::OutsideVocabulary {
                rule: rule_id.to_string(),
                key: key.to_string(),
                value: value.clone(),
                vocabulary,
            });
        }
    }
    Ok(Some(Condition::EnumIn {
        name: name.to_string(),
        allowed,
    }))
}

/// A `_gte` counter, or the phase-visit predicate when its suffix names a
/// phase this table holds.
pub(super) fn parse_counter(
    rule_id: &str,
    key: &str,
    name: &str,
    expected: &Value,
    phases: &[String],
) -> Result<Condition, Malformed> {
    // Counters are the declared list plus one family: the phase-visit
    // predicate (decision 0022), whose suffix must name a phase THIS
    // table has — the vocabulary stays closed, it just closes over
    // the table's own graph.
    let visit_phase = name
        .strip_prefix(VISIT_PREFIX)
        .filter(|phase| phases.iter().any(|known| known == phase));
    if !COUNTER_INPUTS.contains(&name) && visit_phase.is_none() {
        return Err(Malformed::UnknownCounter {
            rule: rule_id.to_string(),
            name: name.to_string(),
            key: key.to_string(),
            phases: phases.to_vec(),
        });
    }
    let threshold = expected.as_f64().ok_or_else(|| Malformed::NotNumeric {
        rule: rule_id.to_string(),
        key: key.to_string(),
        written: expected.to_string(),
    })?;
    Ok(Condition::CounterGte {
        name: name.to_string(),
        threshold,
    })
}

/// A severity condition on one side of the threshold.
fn parse_severity(
    bound: SeverityBound,
    rule_id: &str,
    key: &str,
    name: &str,
    expected: &Value,
) -> Result<Condition, Malformed> {
    let threshold_rank = severity_threshold(rule_id, key, name, expected)?;
    let name = name.to_string();
    Ok(match bound {
        SeverityBound::Above => Condition::SeverityAbove {
            name,
            threshold_rank,
        },
        SeverityBound::AtMost => Condition::SeverityAtMost {
            name,
            threshold_rank,
        },
    })
}

/// A `_above` severity condition.
pub(super) fn parse_severity_above(
    rule_id: &str,
    key: &str,
    name: &str,
    expected: &Value,
) -> Result<Condition, Malformed> {
    parse_severity(SeverityBound::Above, rule_id, key, name, expected)
}

/// An `_at_most` severity condition.
pub(super) fn parse_severity_at_most(
    rule_id: &str,
    key: &str,
    name: &str,
    expected: &Value,
) -> Result<Condition, Malformed> {
    parse_severity(SeverityBound::AtMost, rule_id, key, name, expected)
}

/// A boolean condition, keyed by the input's own name.
pub(super) fn parse_flag(
    rule_id: &str,
    key: &str,
    expected: &Value,
) -> Result<Condition, Malformed> {
    let expected = expected.as_bool().ok_or_else(|| Malformed::NotBoolean {
        rule: rule_id.to_string(),
        key: key.to_string(),
        written: expected.to_string(),
    })?;
    Ok(Condition::Flag {
        name: key.to_string(),
        expected,
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

/// One condition against the inputs: `false` when it does not hold,
/// `Err` when an input is present but unreadable.
pub(super) fn condition_met(
    condition: &Condition,
    inputs: &Map<String, Value>,
) -> Result<bool, Unreadable> {
    match condition {
        Condition::SeverityAbove {
            name,
            threshold_rank,
        } => severity_met(SeverityBound::Above, name, *threshold_rank, inputs),
        // Fail-closed at-most: an ABSENT severity is an unknown
        // severity and never qualifies — the ruling that ships as
        // debt must be earned by a value, not by silence (the
        // review that repealed the blind stop caught exactly this).
        Condition::SeverityAtMost {
            name,
            threshold_rank,
        } => severity_met(SeverityBound::AtMost, name, *threshold_rank, inputs),
        Condition::CounterGte { name, threshold } => {
            let Some(value) = present(name, inputs) else {
                return Ok(false);
            };
            counter_met(name, *threshold, value)
        }
        Condition::Flag { name, expected } => {
            let Some(value) = present(name, inputs) else {
                return Ok(false);
            };
            flag_met(name, *expected, value)
        }
        Condition::EnumIn { name, allowed } => {
            let Some(value) = present(name, inputs) else {
                return Ok(false);
            };
            enum_in_met(name, allowed, value)
        }
    }
}

/// An input the condition reads, or `None` for the absent and the
/// written-null values that never satisfy a condition.
fn present<'a>(name: &str, inputs: &'a Map<String, Value>) -> Option<&'a Value> {
    match inputs.get(name) {
        None | Some(Value::Null) => None,
        Some(value) => Some(value),
    }
}

/// A severity condition against the inputs: absent is false, and only a
/// rank on the condition's side of the threshold holds.
fn severity_met(
    bound: SeverityBound,
    name: &str,
    threshold_rank: usize,
    inputs: &Map<String, Value>,
) -> Result<bool, Unreadable> {
    match severity_of(name, inputs.get(name))? {
        Some(rank) => Ok(bound.holds(rank, threshold_rank)),
        None => Ok(false),
    }
}

/// A `_gte` counter: a present value must be a number.
fn counter_met(name: &str, threshold: f64, value: &Value) -> Result<bool, Unreadable> {
    match value {
        Value::Number(n) => {
            // serde_json rejects non-finite numbers at construction,
            // so every Number is representable through this API.
            let actual = n.as_f64().expect("JSON numbers are finite");
            Ok(actual >= threshold)
        }
        other => Err(Unreadable::NotANumber {
            name: name.to_string(),
            written: other.to_string(),
        }),
    }
}

/// A boolean input: a present value must be a boolean.
fn flag_met(name: &str, expected: bool, value: &Value) -> Result<bool, Unreadable> {
    match value {
        Value::Bool(actual) => Ok(*actual == expected),
        other => Err(Unreadable::NotABoolean {
            name: name.to_string(),
            written: other.to_string(),
        }),
    }
}

/// A choice input: a word outside its vocabulary is a refusal, a word
/// outside the condition's allowed list is false.
fn enum_in_met(name: &str, allowed: &[String], value: &Value) -> Result<bool, Unreadable> {
    match value {
        Value::String(actual) if !vocabulary(name).contains(&actual.as_str()) => {
            Err(Unreadable::OutsideVocabulary {
                name: name.to_string(),
                word: actual.clone(),
                vocabulary: vocabulary(name),
            })
        }
        Value::String(actual) => Ok(allowed.contains(actual)),
        other => Err(Unreadable::NotAString {
            name: name.to_string(),
            written: other.to_string(),
        }),
    }
}
