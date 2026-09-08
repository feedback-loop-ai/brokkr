//! Decision 0050's experiment, as a pinned sweep over every shipped table.
//!
//! Four checks the decision rules into the loader and the compiler, run
//! here against the tables as they stand — the nine with their own
//! `policy.json` and the six `extends` composes — through the real
//! composer and the real `Machine::evaluate`:
//!
//! 1. **order** — no rule is dead behind an earlier rule whose guard
//!    subsumes it (the swap in the decision's context);
//! 2. **liveness** — every phase reachable from `initial`, every phase
//!    ends at a terminal or a parking rule (decision 0004's unported
//!    lints);
//! 3. **presence** — a seat-declared input a `hard` rule reads is read by
//!    every rule in its group that advances (decision 0004's deferred
//!    lint, the fail-open of the 0022 run's seq 335);
//! 4. **totality** — every valuation of present, well-typed inputs a
//!    group reads is ruled, so `NoRule` is left to what a table cannot
//!    foresee.
//!
//! Today's findings are PINNED, not asserted away: the three v1 tables
//! fail presence exactly as decision 0004 recorded, and every v2 delivery
//! table leaves one valuation shape unruled — a `residual` verdict at
//! severity `none`. The enactment slice moves the checks into
//! `Machine::from_table` and `brokkr compile`, names the closed
//! valuations with a parking rule, and retires these pins by driving
//! them to zero. Until then a table change that moves a pin is a
//! reviewed change.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brokkr_core::policy::{Machine, Outcome, DRIFT_PHASES, SEVERITY_ORDER, STRATEGIES};
use brokkr_runtime::bundle::compose::resolve;
use brokkr_runtime::bundle::is_engine_owned;
use serde_json::{json, Map, Value};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("{} parses: {error}", path.display()))
}

/// One guard condition, read the way `parse_condition` reads it: the
/// key names the input and the form, the value the threshold.
#[derive(Clone, Debug, PartialEq)]
enum Guard {
    Counter {
        name: String,
        threshold: i64,
    },
    Above {
        name: String,
        rank: usize,
    },
    AtMost {
        name: String,
        rank: usize,
    },
    Flag {
        name: String,
        expected: bool,
    },
    Member {
        name: String,
        allowed: BTreeSet<String>,
    },
}

impl Guard {
    fn name(&self) -> &str {
        match self {
            Guard::Counter { name, .. }
            | Guard::Above { name, .. }
            | Guard::AtMost { name, .. }
            | Guard::Flag { name, .. }
            | Guard::Member { name, .. } => name,
        }
    }

    /// Whether every input satisfying `self` satisfies `other`.
    fn implies(&self, other: &Guard) -> bool {
        match (self, other) {
            (
                Guard::Counter {
                    name: a,
                    threshold: x,
                },
                Guard::Counter {
                    name: b,
                    threshold: y,
                },
            ) => a == b && x >= y,
            (Guard::Above { name: a, rank: x }, Guard::Above { name: b, rank: y }) => {
                a == b && x >= y
            }
            (Guard::AtMost { name: a, rank: x }, Guard::AtMost { name: b, rank: y }) => {
                a == b && x <= y
            }
            (
                Guard::Flag {
                    name: a,
                    expected: x,
                },
                Guard::Flag {
                    name: b,
                    expected: y,
                },
            ) => a == b && x == y,
            (
                Guard::Member {
                    name: a,
                    allowed: x,
                },
                Guard::Member {
                    name: b,
                    allowed: y,
                },
            ) => a == b && x.is_subset(y),
            _ => false,
        }
    }
}

fn rank(value: &Value) -> usize {
    let word = value.as_str().expect("a severity word");
    SEVERITY_ORDER
        .iter()
        .position(|known| known == &word)
        .unwrap_or_else(|| panic!("{word} is on the severity axis"))
}

fn guards(rule: &Value) -> Vec<Guard> {
    let Some(when) = rule.get("when").and_then(Value::as_object) else {
        return Vec::new();
    };
    when.iter()
        .map(|(key, value)| {
            if key == "strategy_in" {
                return Guard::Member {
                    name: "strategy".into(),
                    allowed: value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect(),
                };
            }
            if key == "drift_in" {
                return Guard::Member {
                    name: "drift_in".into(),
                    allowed: value
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect(),
                };
            }
            if let Some(name) = key.strip_suffix("_gte") {
                return Guard::Counter {
                    name: name.into(),
                    threshold: value.as_f64().expect("a counter threshold") as i64,
                };
            }
            if let Some(name) = key.strip_suffix("_above") {
                return Guard::Above {
                    name: name.into(),
                    rank: rank(value),
                };
            }
            if let Some(name) = key.strip_suffix("_at_most") {
                return Guard::AtMost {
                    name: name.into(),
                    rank: rank(value),
                };
            }
            Guard::Flag {
                name: key.clone(),
                expected: value.as_bool().expect("a flag"),
            }
        })
        .collect()
}

/// Guard `earlier` matches everywhere guard `later` matches, so a rule
/// carrying `later` behind one carrying `earlier` can never fire.
fn subsumes(earlier: &[Guard], later: &[Guard]) -> bool {
    earlier.iter().all(|a| later.iter().any(|b| b.implies(a)))
}

/// The domain the sweep walks for one input, from the guards that read
/// it: both flags, the counter at, below and above every threshold, the
/// whole severity axis, the closed enumeration.
fn domain(group: &[Guard], name: &str) -> Vec<Value> {
    let reads: Vec<&Guard> = group.iter().filter(|g| g.name() == name).collect();
    if reads.iter().any(|g| matches!(g, Guard::Flag { .. })) {
        return vec![json!(true), json!(false)];
    }
    if reads.iter().any(|g| matches!(g, Guard::Member { .. })) {
        let vocabulary: &[&str] = if name == "strategy" {
            &STRATEGIES
        } else {
            &DRIFT_PHASES
        };
        return vocabulary.iter().map(|word| json!(word)).collect();
    }
    if reads.iter().any(|g| matches!(g, Guard::Counter { .. })) {
        let mut values: BTreeSet<i64> = BTreeSet::from([0]);
        for guard in &reads {
            if let Guard::Counter { threshold, .. } = guard {
                values.extend([threshold - 1, *threshold, threshold + 1]);
            }
        }
        return values
            .into_iter()
            .filter(|value| *value >= 0)
            .map(|value| json!(value))
            .collect();
    }
    SEVERITY_ORDER.iter().map(|word| json!(word)).collect()
}

struct Table {
    label: String,
    json: Value,
    machine: Machine,
}

fn table(label: &str, json: Value) -> Table {
    let machine = Machine::from_table(&json)
        .unwrap_or_else(|error| panic!("{label} loads under the strict loader: {error}"));
    Table {
        label: label.to_string(),
        json,
        machine,
    }
}

/// Every table a run can be pinned to: the frozen heritage table, the two
/// bundles, and every recipe — composed through the real `resolve`, so a
/// derived recipe is read as the flat table the engine sees.
fn shipped_tables() -> Vec<Table> {
    let root = workspace();
    let mut tables = vec![table(
        "policy/phase-machine.json",
        read_json(&root.join("policy/phase-machine.json")),
    )];
    for bundle in ["self", "verify"] {
        tables.push(table(
            &format!("bundles/{bundle}"),
            read_json(&root.join(format!("bundles/{bundle}/policy.json"))),
        ));
    }
    let mut recipes: Vec<PathBuf> = std::fs::read_dir(root.join("recipes"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("bundle.json").is_file())
        .collect();
    recipes.sort();
    for dir in recipes {
        let resolved =
            resolve(&dir).unwrap_or_else(|error| panic!("{} resolves: {error}", dir.display()));
        tables.push(table(
            &format!("recipes/{}", dir.file_name().unwrap().to_string_lossy()),
            resolved.table,
        ));
    }
    tables
}

fn rules(t: &Table) -> &Vec<Value> {
    t.json["rules"].as_array().expect("rules")
}

fn next_of(rule: &Value) -> Option<&str> {
    rule.get("next").and_then(Value::as_str)
}

fn parks(rule: &Value) -> bool {
    rule.get("park") == Some(&Value::Bool(true))
}

fn edges(t: &Table) -> BTreeMap<String, BTreeSet<String>> {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for rule in rules(t) {
        if let Some(next) = next_of(rule) {
            edges
                .entry(rule["from"].as_str().unwrap().to_string())
                .or_default()
                .insert(next.to_string());
        }
    }
    edges
}

/// Whether `start` reaches a non-stop terminal along edges that never
/// enter `avoid` — the constitution's cut-vertex question, asked per
/// group: a rule that advances lets the run go on; a rule that returns
/// sends it back through the phase whose hard rule rules again.
fn advances(t: &Table, start: &str, avoid: &str) -> bool {
    let edges = edges(t);
    let goal: BTreeSet<&str> = t
        .machine
        .terminal
        .iter()
        .map(String::as_str)
        .filter(|phase| *phase != "stop")
        .collect();
    let mut seen: BTreeSet<String> = BTreeSet::from([start.to_string()]);
    let mut frontier = vec![start.to_string()];
    while let Some(node) = frontier.pop() {
        if goal.contains(node.as_str()) {
            return true;
        }
        for next in edges.get(&node).into_iter().flatten() {
            if next != avoid && seen.insert(next.clone()) {
                frontier.push(next.clone());
            }
        }
    }
    false
}

#[derive(Debug, Default)]
struct Findings {
    /// `<later> is dead behind <earlier>`.
    order: Vec<String>,
    unreachable: Vec<String>,
    dead_end: Vec<String>,
    /// `<rule> lets the run go on without reading <inputs>`.
    presence: Vec<String>,
    /// Every valuation the sweep walked.
    valuations: usize,
    /// The valuations no rule rules, as `(phase, result, inputs)`.
    unruled: Vec<(String, String, Map<String, Value>)>,
}

fn sweep(t: &Table) -> Findings {
    let mut findings = Findings::default();
    let mut groups: BTreeMap<(String, String), Vec<&Value>> = BTreeMap::new();
    for rule in rules(t) {
        groups
            .entry((
                rule["from"].as_str().unwrap().to_string(),
                rule["result"].as_str().unwrap().to_string(),
            ))
            .or_default()
            .push(rule);
    }
    for ((phase, result), group) in &groups {
        let guarded: Vec<Vec<Guard>> = group.iter().map(|rule| guards(rule)).collect();
        for (i, earlier) in guarded.iter().enumerate() {
            for (j, later) in guarded.iter().enumerate().skip(i + 1) {
                if subsumes(earlier, later) {
                    findings.order.push(format!(
                        "{} is dead behind {}",
                        group[j]["id"].as_str().unwrap(),
                        group[i]["id"].as_str().unwrap()
                    ));
                }
            }
        }
        let all: Vec<Guard> = guarded.iter().flatten().cloned().collect();
        let names: BTreeSet<&str> = all.iter().map(Guard::name).collect();
        let domains: Vec<(&str, Vec<Value>)> = names
            .iter()
            .map(|name| (*name, domain(&all, name)))
            .collect();
        let mut valuations: Vec<Map<String, Value>> = vec![Map::new()];
        for (name, values) in &domains {
            valuations = valuations
                .iter()
                .flat_map(|partial| {
                    values.iter().map(move |value| {
                        let mut next = partial.clone();
                        next.insert((*name).to_string(), value.clone());
                        next
                    })
                })
                .collect();
        }
        for inputs in valuations {
            findings.valuations += 1;
            if let Outcome::NoRule { problem } = t.machine.evaluate(phase, result, &inputs) {
                assert!(
                    problem.is_none(),
                    "{}: the sweep supplies well-typed inputs only",
                    t.label
                );
                findings
                    .unruled
                    .push((phase.clone(), result.clone(), inputs));
            }
        }
        let deny: BTreeSet<&str> = group
            .iter()
            .zip(&guarded)
            .filter(|(rule, _)| rule.get("severity") == Some(&json!("hard")))
            .flat_map(|(_, guards)| guards.iter().map(Guard::name))
            .filter(|name| !is_engine_owned(name))
            .collect();
        for (rule, guards) in group.iter().zip(&guarded) {
            let Some(next) = next_of(rule) else {
                continue;
            };
            if next == "stop" || !advances(t, next, phase) {
                continue;
            }
            let reads: BTreeSet<&str> = guards.iter().map(Guard::name).collect();
            let missing: Vec<&str> = deny.difference(&reads).copied().collect();
            if !missing.is_empty() {
                findings.presence.push(format!(
                    "{} lets the run go on without reading {missing:?}",
                    rule["id"].as_str().unwrap()
                ));
            }
        }
    }
    let edges = edges(t);
    let mut seen: BTreeSet<String> = BTreeSet::from([t.machine.initial.clone()]);
    let mut frontier = vec![t.machine.initial.clone()];
    while let Some(node) = frontier.pop() {
        for next in edges.get(&node).into_iter().flatten() {
            if seen.insert(next.clone()) {
                frontier.push(next.clone());
            }
        }
    }
    let terminal: BTreeSet<&String> = t.machine.terminal.iter().collect();
    let parking: BTreeSet<&str> = rules(t)
        .iter()
        .filter(|rule| parks(rule))
        .map(|rule| rule["from"].as_str().unwrap())
        .collect();
    let ends = |phase: &String| -> bool {
        let mut seen: BTreeSet<String> = BTreeSet::from([phase.clone()]);
        let mut frontier = vec![phase.clone()];
        while let Some(node) = frontier.pop() {
            if terminal.contains(&node) || parking.contains(node.as_str()) {
                return true;
            }
            for next in edges.get(&node).into_iter().flatten() {
                if seen.insert(next.clone()) {
                    frontier.push(next.clone());
                }
            }
        }
        false
    };
    for phase in &t.machine.phases {
        if !seen.contains(phase) {
            findings.unreachable.push(phase.clone());
        } else if !terminal.contains(phase) && !ends(phase) {
            findings.dead_end.push(phase.clone());
        }
    }
    findings
}

#[test]
fn every_shipped_table_is_ordered_and_live() {
    for t in shipped_tables() {
        let findings = sweep(&t);
        assert!(
            findings.order.is_empty(),
            "{}: {:?}",
            t.label,
            findings.order
        );
        assert!(
            findings.unreachable.is_empty(),
            "{}: {:?}",
            t.label,
            findings.unreachable
        );
        assert!(
            findings.dead_end.is_empty(),
            "{}: {:?}",
            t.label,
            findings.dead_end
        );
    }
}

/// Decision 0004 recorded the shape: "when a deny rule is conditional and
/// the permissive fallback is unconditional, an absent input still
/// reaches the fallback — exactly as in production." The three v1 tables
/// carry it; no v2 table does.
#[test]
fn presence_refuses_exactly_the_three_v1_tables() {
    let expected: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        (
            "policy/phase-machine.json",
            vec![
                "REVIEW-CLEAN-UNVERIFIED lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]",
                "REVIEW-RESIDUAL-OK lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]",
            ],
        ),
        (
            "bundles/verify",
            vec!["REVIEW-RESIDUAL-OK lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]"],
        ),
        (
            "recipes/preflight",
            vec!["REVIEW-RESIDUAL-OK lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]"],
        ),
    ]);
    for t in shipped_tables() {
        let findings = sweep(&t);
        let want: Vec<String> = expected
            .get(t.label.as_str())
            .map(|rows| rows.iter().map(|row| row.to_string()).collect())
            .unwrap_or_default();
        assert_eq!(findings.presence, want, "{}", t.label);
        if !want.is_empty() {
            assert_eq!(t.json["schema"], "forge.phase-machine/v1", "{}", t.label);
        }
    }
}

/// The sweep's size and its holes, per table. Every hole today is one
/// shape: a `residual` verdict whose severity is `none`, which the v2
/// tables close by requiring a severity above `none` on every permissive
/// arm — closed, and unnamed. The heritage table has no hole because its
/// fallback is unconditional, which is the presence finding above.
#[test]
fn the_unruled_valuations_are_pinned_per_table() {
    let expected: BTreeMap<&str, (usize, usize)> = BTreeMap::from([
        ("policy/phase-machine.json", (57, 0)),
        ("bundles/self", (47, 4)),
        ("bundles/verify", (16, 0)),
        ("recipes/fast", (46, 4)),
        ("recipes/landing", (48, 4)),
        ("recipes/night-shift", (1072, 128)),
        ("recipes/node", (46, 4)),
        ("recipes/panel-review", (47, 4)),
        ("recipes/preflight", (16, 0)),
        ("recipes/release", (46, 4)),
        ("recipes/research", (5, 0)),
        ("recipes/research-dsh", (5, 0)),
        ("recipes/standby", (46, 4)),
        ("recipes/triage", (1072, 128)),
        ("recipes/wager-harness", (46, 4)),
        ("recipes/wager-harness-dsh", (46, 4)),
        ("recipes/wager-harness-muse", (46, 4)),
    ]);
    let tables = shipped_tables();
    let labels: BTreeSet<&str> = tables.iter().map(|t| t.label.as_str()).collect();
    assert_eq!(labels, expected.keys().copied().collect::<BTreeSet<&str>>());
    for t in &tables {
        let findings = sweep(t);
        assert_eq!(
            (findings.valuations, findings.unruled.len()),
            expected[t.label.as_str()],
            "{}: (valuations, unruled)",
            t.label
        );
        for (phase, result, inputs) in &findings.unruled {
            assert_eq!(
                (phase.as_str(), result.as_str()),
                ("review", "residual"),
                "{}",
                t.label
            );
            assert_eq!(
                inputs["max_residual_severity"], "none",
                "{}: {inputs:?}",
                t.label
            );
        }
    }
}

/// Decision 0022's arc as a property: the table that ruled a diligent
/// reviewer's note a death was total, deterministic and wrong.
#[test]
fn the_stated_properties_hold_on_every_shipped_table() {
    for t in shipped_tables() {
        if !t.machine.phases.iter().any(|phase| phase == "review") {
            continue;
        }
        let clean = json!({"fixes_applied": false});
        match t
            .machine
            .evaluate("review", "clean", clean.as_object().unwrap())
        {
            Outcome::Ruling { next_phase, .. } => {
                assert!(
                    advances(&t, &next_phase, ""),
                    "{}: clean can go on",
                    t.label
                )
            }
            other => panic!("{}: clean rules {other:?}", t.label),
        }
        let low = json!({
            "max_residual_severity": "low",
            "has_security_residual": false,
            "visits_implement": 1,
            "fixes_applied": true
        });
        match t
            .machine
            .evaluate("review", "residual", low.as_object().unwrap())
        {
            Outcome::Ruling { next_phase, .. } => {
                assert_ne!(
                    next_phase, "stop",
                    "{}: a low non-security residual does not stop",
                    t.label
                )
            }
            other => panic!("{}: a low residual rules {other:?}", t.label),
        }
    }
}

fn self_table() -> Value {
    read_json(&workspace().join("bundles/self/policy.json"))
}

fn position(table: &Value, id: &str) -> usize {
    table["rules"]
        .as_array()
        .unwrap()
        .iter()
        .position(|rule| rule["id"] == id)
        .unwrap_or_else(|| panic!("{id} is in the table"))
}

/// The decision's context, reproduced: exchange the two exhaustion arms
/// and the table still loads, while a high residual at the bound parks
/// where the constitution says it stops.
#[test]
fn the_exchanged_self_table_is_dead_behind_its_weaker_arm() {
    let mut json = self_table();
    let above = position(&json, "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM");
    let medium = position(&json, "REVIEW-REFORGE-EXHAUSTED-MEDIUM");
    json["rules"].as_array_mut().unwrap().swap(above, medium);
    let t = table("bundles/self, exchanged", json);
    assert_eq!(
        sweep(&t).order,
        vec![
            "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM is dead behind REVIEW-REFORGE-EXHAUSTED-MEDIUM"
        ]
    );
    let high = json!({"max_residual_severity": "high", "visits_implement": 3});
    assert!(
        matches!(
            t.machine.evaluate("review", "residual", high.as_object().unwrap()),
            Outcome::Park { ref rule_id, .. } if rule_id == "REVIEW-REFORGE-EXHAUSTED-MEDIUM"
        ),
        "the exchanged table parks a high residual at the bound"
    );
}

/// Seq 335 of `implement-decision-0022-reforgin-54a88e9b`, reproduced:
/// permissive arms that read no severity ship an absent one, security
/// flag and all. Presence refuses the shape by name.
#[test]
fn the_0022_era_permissive_arms_fail_open() {
    let mut json = self_table();
    for rule in json["rules"].as_array_mut().unwrap() {
        if rule["id"] == "REVIEW-REFORGE-EXHAUSTED-DEBT" || rule["id"] == "REVIEW-RESIDUAL-OK" {
            let when = rule["when"].as_object_mut().unwrap();
            when.retain(|key, _| !key.starts_with("max_residual_severity"));
        }
    }
    let t = table("bundles/self, 0022-era", json);
    assert_eq!(
        sweep(&t).presence,
        vec![
            "REVIEW-REFORGE-EXHAUSTED-DEBT lets the run go on without reading [\"max_residual_severity\"]",
            "REVIEW-RESIDUAL-OK lets the run go on without reading [\"max_residual_severity\"]",
        ]
    );
    let absent = json!({"visits_implement": 3, "has_security_residual": true});
    assert!(
        matches!(
            t.machine.evaluate("review", "residual", absent.as_object().unwrap()),
            Outcome::Ruling { ref rule_id, ref next_phase, .. }
                if rule_id == "REVIEW-REFORGE-EXHAUSTED-DEBT" && next_phase == "ship"
        ),
        "an absent severity with the security flag set ships"
    );
}
