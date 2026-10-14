//! The resolver's markers (decision 0017): what a layer's `remove` and
//! `override` may name, and the one table every marker is checked by.
//!
//! A marker kind is a closed enum. What each kind reads and changes is an
//! exhaustive match, so a kind added without its arms does not compile,
//! and every word the vocabularies admit is a kind's word, so no word is
//! admitted that no row checks. The table's rows run in the order the
//! refusals have always been reported in: removals first, kind by kind,
//! then overrides, each entry judged against the layers beneath before
//! this layer.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::{
    drop_phase_name, drop_rule, invalid, rule_id, rules_of, stale, CompileError, Layer, Merged,
};

/// A member kind `remove` may be keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Removal {
    Seats,
    Rules,
    Phases,
}

/// A member kind `override` may be keyed by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Overriding {
    Cases,
    Limits,
    Seats,
    Bundle,
    Table,
    Rules,
}

/// Member kinds `override` may be keyed by, in the order its refusals
/// list them.
const OVERRIDE_KINDS: [Overriding; 6] = [
    Overriding::Seats,
    Overriding::Cases,
    Overriding::Limits,
    Overriding::Rules,
    Overriding::Table,
    Overriding::Bundle,
];

/// Member kinds `remove` may be keyed by, in the order its refusals list
/// them.
const REMOVE_KINDS: [Removal; 3] = [Removal::Seats, Removal::Rules, Removal::Phases];

/// One row of the marker table: the kind, the marker every refusal names,
/// and why the marker is stale at each check, in the order they run. A
/// removal makes one check (a layer beneath has the name) and an override
/// two (beneath, then this layer), so a row with the wrong number of
/// reasons does not compile.
struct MarkerRule<K, const CHECKS: usize> {
    kind: K,
    marker: &'static str,
    stale: [&'static str; CHECKS],
}

/// The removals, in the order they are checked and applied.
const REMOVALS: [MarkerRule<Removal, 1>; 3] = [
    MarkerRule {
        kind: Removal::Seats,
        marker: "remove.seats",
        stale: ["no ancestor defines it"],
    },
    MarkerRule {
        kind: Removal::Rules,
        marker: "remove.rules",
        stale: ["no ancestor's table has it"],
    },
    MarkerRule {
        kind: Removal::Phases,
        marker: "remove.phases",
        stale: ["no ancestor's table names it"],
    },
];

/// The overrides, in the order they are checked.
const OVERRIDES: [MarkerRule<Overriding, 2>; 6] = [
    MarkerRule {
        kind: Overriding::Cases,
        marker: "override.cases",
        stale: ["no ancestor defines it", "this recipe does not redefine it"],
    },
    MarkerRule {
        kind: Overriding::Limits,
        marker: "override.limits",
        stale: ["no ancestor defines it", "this recipe does not redefine it"],
    },
    MarkerRule {
        kind: Overriding::Seats,
        marker: "override.seats",
        stale: ["no ancestor defines it", "this recipe does not redefine it"],
    },
    MarkerRule {
        kind: Overriding::Bundle,
        marker: "override.bundle",
        stale: ["no ancestor sets it", "this recipe does not set it"],
    },
    MarkerRule {
        kind: Overriding::Table,
        marker: "override.table",
        stale: [
            "no ancestor's table sets it",
            "this recipe's table does not set it",
        ],
    },
    MarkerRule {
        kind: Overriding::Rules,
        marker: "override.rules",
        stale: [
            "no ancestor's table has it",
            "this recipe's table does not have it",
        ],
    },
];

impl Removal {
    /// The word a layer writes under `remove`.
    const fn word(self) -> &'static str {
        match self {
            Removal::Seats => "seats",
            Removal::Rules => "rules",
            Removal::Phases => "phases",
        }
    }

    /// Remove `name` from what the layers beneath resolved; `false`, with
    /// nothing touched, when no layer beneath has it.
    fn apply(self, merged: &mut Merged, name: &str) -> bool {
        match self {
            Removal::Seats => {
                let held = merged.seat_from.remove(name).is_some();
                if held {
                    merged.seats.remove(name);
                }
                held
            }
            Removal::Rules => {
                let held = merged.rule_from.remove(name).is_some();
                if held {
                    drop_rule(&mut merged.table, name);
                }
                held
            }
            Removal::Phases => drop_phase_name(&mut merged.table, name),
        }
    }
}

impl Overriding {
    /// The word a layer writes under `override`.
    const fn word(self) -> &'static str {
        match self {
            Overriding::Cases => "cases",
            Overriding::Limits => "limits",
            Overriding::Seats => "seats",
            Overriding::Bundle => "bundle",
            Overriding::Table => "table",
            Overriding::Rules => "rules",
        }
    }

    /// Refuse an entry this kind cannot name, before it is judged.
    fn admit(self, layer: &Layer, named: &str) -> Result<(), CompileError> {
        match self {
            Overriding::Cases => match case_mark(named) {
                Some(_) => Ok(()),
                None => Err(invalid(format!(
                    "{}: 'override.cases' entry '{named}' must be '<seat>:<case>'",
                    layer.file.display()
                ))),
            },
            Overriding::Limits
            | Overriding::Seats
            | Overriding::Bundle
            | Overriding::Table
            | Overriding::Rules => Ok(()),
        }
    }

    /// Whether the layers beneath define what `name` names.
    fn inherited(self, merged: &Merged, name: &str) -> bool {
        match self {
            Overriding::Cases => holds_case(Some(&merged.seats), name),
            Overriding::Limits => holds_limits(Some(&merged.seats), name),
            Overriding::Seats => merged.seats.contains_key(name),
            Overriding::Bundle => merged.document.contains_key(name),
            Overriding::Table => merged.table.contains_key(name),
            Overriding::Rules => merged.rule_from.contains_key(name),
        }
    }

    /// Whether this layer, or its own `table`, writes what `name` names.
    fn written(self, layer: &Layer, table: Option<&Map<String, Value>>, name: &str) -> bool {
        let seats = layer.document.get("seats").and_then(Value::as_object);
        match self {
            Overriding::Cases => holds_case(seats, name),
            Overriding::Limits => holds_limits(seats, name),
            Overriding::Seats => seats.is_some_and(|seats| seats.contains_key(name)),
            Overriding::Bundle => layer.document.contains_key(name),
            Overriding::Table => table.is_some_and(|own| own.contains_key(name)),
            Overriding::Rules => {
                table.is_some_and(|own| rules_of(own).iter().any(|r| rule_id(r) == Some(name)))
            }
        }
    }
}

/// The `<seat>:<case>` grammar of an `override.cases` entry, in one place.
fn case_mark(named: &str) -> Option<(&str, &str)> {
    named.split_once(':')
}

/// Whether `seats` holds the select case an `override.cases` entry names.
fn holds_case(seats: Option<&Map<String, Value>>, named: &str) -> bool {
    case_mark(named).is_some_and(|(seat, case)| {
        seats
            .and_then(|seats| seats.get(seat))
            .and_then(|value| value.pointer("/select/cases"))
            .and_then(Value::as_object)
            .is_some_and(|cases| cases.contains_key(case))
    })
}

/// Whether `seats` holds the seat `name` with its own `limits`.
fn holds_limits(seats: Option<&Map<String, Value>>, name: &str) -> bool {
    seats
        .and_then(|seats| seats.get(name))
        .is_some_and(|value| value.get("limits").is_some())
}

/// A layer's resolver-owned markers, validated on read: an unknown kind
/// or a non-string entry is refused here, naming the file and the key.
pub(super) struct Markers {
    overrides: BTreeMap<String, Vec<String>>,
    removals: BTreeMap<String, Vec<String>>,
}

impl Markers {
    pub(super) fn read(layer: &Layer) -> Result<Markers, CompileError> {
        Ok(Markers {
            overrides: marker_map(layer, "override", &OVERRIDE_KINDS.map(Overriding::word))?,
            removals: marker_map(layer, "remove", &REMOVE_KINDS.map(Removal::word))?,
        })
    }

    fn overriding(&self, kind: Overriding) -> &[String] {
        self.overrides.get(kind.word()).map_or(&[], Vec::as_slice)
    }

    fn removing(&self, kind: Removal) -> &[String] {
        self.removals.get(kind.word()).map_or(&[], Vec::as_slice)
    }

    pub(super) fn overrides_name(&self, kind: Overriding, name: &str) -> bool {
        self.overriding(kind).iter().any(|listed| listed == name)
    }

    /// The cases `override.cases` names on `seat`.
    pub(super) fn cases_of(&self, seat: &str) -> Vec<&str> {
        self.overriding(Overriding::Cases)
            .iter()
            .filter_map(|entry| case_mark(entry))
            .filter_map(|(named, case)| (named == seat).then_some(case))
            .collect()
    }

    /// Walk the table over one layer: every removal is checked against the
    /// layers beneath and applied before the next, so removing a name twice
    /// is stale at the second; then every override is admitted and checked
    /// beneath, then in this layer. The first stale entry is refused.
    pub(super) fn check(
        &self,
        merged: &mut Merged,
        layer: &Layer,
        table: Option<&Map<String, Value>>,
    ) -> Result<(), CompileError> {
        for rule in &REMOVALS {
            let [unseen] = rule.stale;
            for name in self.removing(rule.kind) {
                if !rule.kind.apply(merged, name) {
                    return Err(stale(layer, rule.marker, name, unseen));
                }
            }
        }
        for rule in &OVERRIDES {
            let [unseen, unwritten] = rule.stale;
            for name in self.overriding(rule.kind) {
                rule.kind.admit(layer, name)?;
                if !rule.kind.inherited(merged, name) {
                    return Err(stale(layer, rule.marker, name, unseen));
                }
                if !rule.kind.written(layer, table, name) {
                    return Err(stale(layer, rule.marker, name, unwritten));
                }
            }
        }
        Ok(())
    }
}

fn marker_map(
    layer: &Layer,
    key: &str,
    kinds: &[&str],
) -> Result<BTreeMap<String, Vec<String>>, CompileError> {
    let mut out = BTreeMap::new();
    let Some(raw) = layer.document.get(key) else {
        return Ok(out);
    };
    let shape = || {
        invalid(format!(
            "{}: '{key}' must be an object of name arrays keyed by member kind ({})",
            layer.file.display(),
            kinds.join(", ")
        ))
    };
    for (kind, listed) in raw.as_object().ok_or_else(shape)? {
        if !kinds.contains(&kind.as_str()) {
            return Err(invalid(format!(
                "{}: '{key}.{kind}' is not a member kind; known: {}",
                layer.file.display(),
                kinds.join(", ")
            )));
        }
        let mut names = Vec::new();
        for item in listed.as_array().ok_or_else(shape)? {
            names.push(item.as_str().ok_or_else(shape)?.to_string());
        }
        out.insert(kind.clone(), names);
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
