//! One policy table rule, read field by field (#288): each refusal keeps
//! the order `parse_rule` checks it in, so a rule that trips two is
//! refused by the first. The helpers are private to the policy module;
//! `parse_rule` remains the one place that assembles a rule.

use serde_json::{Map, Value};

use super::{
    parse_condition, string_array, Condition, Malformed, Place, RuleKey, RULING_SEVERITIES,
    TABLE_SCHEMA_V2,
};

/// One rule string field, or the refusal naming the rule and the key it
/// did not write.
pub(super) fn field(obj: &Map<String, Value>, key: RuleKey) -> Result<String, Malformed> {
    obj.get(key.field())
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Malformed::MissingField {
            rule: obj
                .get(RuleKey::Id.field())
                .and_then(Value::as_str)
                .map(str::to_string),
            key,
        })
}

/// A v2 rule may write only the closed vocabulary; a v1 rule keeps its
/// annotation keys.
pub(super) fn check_keys(
    obj: &Map<String, Value>,
    id: &str,
    schema: Option<&str>,
) -> Result<(), Malformed> {
    if schema == Some(TABLE_SCHEMA_V2) {
        let known = |key: &String| RuleKey::ALL.iter().any(|k| k.field() == key);
        if let Some(key) = obj.keys().find(|key| !known(key)) {
            return Err(Malformed::UnknownRuleKey {
                rule: id.to_string(),
                key: key.clone(),
            });
        }
    }
    Ok(())
}

/// `park` is v2 vocabulary: absent is false, and only a written `true`
/// is a park. Any other written value is refused with its own spelling.
pub(super) fn parse_park(obj: &Map<String, Value>, id: &str) -> Result<bool, Malformed> {
    match obj.get(RuleKey::Park.field()) {
        None => Ok(false),
        Some(Value::Bool(true)) => Ok(true),
        Some(other) => Err(Malformed::ParkNotTrue {
            rule: id.to_string(),
            written: other.to_string(),
        }),
    }
}

/// A rule either advances the run or parks it — never both, never
/// neither. A parking rule writes no `next`; every other rule must.
pub(super) fn parse_next(
    obj: &Map<String, Value>,
    id: &str,
    parks: bool,
) -> Result<Option<String>, Malformed> {
    match (parks, obj.get(RuleKey::Next.field())) {
        (false, _) => Ok(Some(field(obj, RuleKey::Next)?)),
        (true, None) => Ok(None),
        (true, Some(_)) => Err(Malformed::ParkAndNext(id.to_string())),
    }
}

/// `park` under a table that does not call itself v2, or beside the
/// fields a park must not carry, is refused rather than read.
pub(super) fn check_park(
    obj: &Map<String, Value>,
    id: &str,
    schema: Option<&str>,
) -> Result<(), Malformed> {
    if schema != Some(TABLE_SCHEMA_V2) {
        return Err(Malformed::ParkOutsideV2 {
            rule: id.to_string(),
            schema: schema.map(str::to_string),
        });
    }
    for forbidden in [RuleKey::Severity, RuleKey::RequiresArtifacts] {
        if obj.get(forbidden.field()).is_some() {
            return Err(Malformed::ParkDeclares {
                rule: id.to_string(),
                key: forbidden,
            });
        }
    }
    Ok(())
}

/// Where a rule leaves and where it goes must be phases this table
/// declares, and a rule may not leave a terminal phase.
pub(super) fn check_phases(
    id: &str,
    from: &str,
    next: Option<&String>,
    phases: &[String],
    terminal: &[String],
) -> Result<(), Malformed> {
    if !phases.iter().any(|phase| phase == from) || next.is_some_and(|next| !phases.contains(next))
    {
        return Err(Malformed::UnknownPhase(id.to_string()));
    }
    if terminal.iter().any(|phase| phase == from) {
        return Err(Malformed::LeavesTerminal {
            rule: id.to_string(),
            phase: from.to_string(),
        });
    }
    Ok(())
}

/// A rule's ruling severity, `normal` when it writes none.
pub(super) fn parse_severity(obj: &Map<String, Value>, id: &str) -> Result<String, Malformed> {
    match obj.get(RuleKey::Severity.field()) {
        None => Ok("normal".to_string()),
        Some(v) => {
            let s = v
                .as_str()
                .ok_or_else(|| Malformed::SeverityNotAString(id.to_string()))?;
            if !RULING_SEVERITIES.contains(&s) {
                return Err(Malformed::UnknownSeverity {
                    rule: id.to_string(),
                    severity: s.to_string(),
                });
            }
            Ok(s.to_string())
        }
    }
}

/// A rule's artifact gate, empty when it writes none.
pub(super) fn parse_requires_artifacts(
    obj: &Map<String, Value>,
    id: &str,
) -> Result<Vec<String>, Malformed> {
    match obj.get(RuleKey::RequiresArtifacts.field()) {
        None => Ok(Vec::new()),
        Some(v) => string_array(
            v,
            Place::Artifacts {
                rule: id.to_string(),
            },
        ),
    }
}

/// A rule's `when` conditions, in the table's key order.
pub(super) fn parse_when(
    obj: &Map<String, Value>,
    id: &str,
    phases: &[String],
) -> Result<Vec<Condition>, Malformed> {
    match obj.get(RuleKey::When.field()) {
        None => Ok(Vec::new()),
        Some(v) => {
            let map = v
                .as_object()
                .ok_or_else(|| Malformed::WhenNotAnObject(id.to_string()))?;
            let mut conditions = Vec::with_capacity(map.len());
            for (key, expected) in map {
                conditions.push(parse_condition(id, key, expected, phases)?);
            }
            Ok(conditions)
        }
    }
}
