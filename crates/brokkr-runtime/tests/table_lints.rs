//! Decision 0050's experiment, as a pinned sweep over every shipped table.
//!
//! Four checks the decision rules into the loader and the compiler, run
//! here against the tables as they stand — every bundle and every recipe,
//! composed ones included — through the real composer and the core's
//! `Machine::audit_with`, which sweeps with the real `Machine::evaluate`
//! and which `brokkr compile` reports (#429):
//!
//! 1. **order** — no rule is dead: every rule rules some swept valuation,
//!    so none sits behind earlier rules that match wherever it does (the
//!    swap in the decision's context);
//! 2. **liveness** — every phase reachable from `initial`, every phase
//!    ends at a terminal or a parking rule (decision 0004's unported
//!    lints);
//! 3. **presence** — a seat-declared input a `hard` rule reads is read by
//!    every rule in its group that advances (decision 0004's deferred
//!    lint, the fail-open of the 0022 run's seq 335);
//! 4. **totality** — every valuation of present, well-typed inputs a
//!    group reads is ruled, so `Unmatched` is left to what a table cannot
//!    foresee.
//!
//! The operator accepted the decision on the #429 audit
//! (`docs/evidence/decision-0050-audit.md`). Its first enactment slice
//! refuses order, liveness and v2 presence at load; its second moves
//! `recipes/verify` and `recipes/preflight` to v2, names the `residual`
//! verdict at severity `none` in every table that left it unruled
//! (`REVIEW-RESIDUAL-NONE`), and refuses an unruled valuation at compile.
//! So every table here loads and is total, and the one finding left is
//! the frozen heritage table's presence, which v1 reads as decision 0004
//! recorded. The sweep's size per table stays pinned: a table change that
//! moves it is a reviewed change.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brokkr_core::policy::audit::{Finding, Refusal, Setting, SWEEP_BUDGET};
use brokkr_core::policy::{Machine, Outcome, PolicyError};
use brokkr_runtime::bundle::compose::resolve;
use brokkr_runtime::bundle::is_engine_owned;
use serde_json::{json, Value};

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

/// Every table a run can be pinned to: the frozen heritage table and
/// every recipe, Brokkr's own included — composed through the real
/// `resolve`, so a derived recipe is read as the flat table the engine
/// sees.
fn shipped_tables() -> Vec<Table> {
    let root = workspace();
    let mut tables = vec![table(
        "policy/phase-machine.json",
        read_json(&root.join("policy/phase-machine.json")),
    )];
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

/// An unruled valuation, as `(phase, result, valuation)`.
type Unruled = (String, String, Vec<(String, Setting)>);

#[derive(Debug, Default)]
struct Findings {
    /// `<later> is dead behind <earlier, ...>`, or `<rule> is dead`.
    order: Vec<String>,
    unreachable: Vec<String>,
    dead_end: Vec<String>,
    /// `<rule> lets the run go on without reading <inputs>`.
    presence: Vec<String>,
    /// Every valuation the sweep walked.
    valuations: usize,
    /// The valuations no rule rules.
    unruled: Vec<Unruled>,
}

/// The core's audit of one table, sorted by check.
fn sweep(t: &Table) -> Findings {
    let audit = t
        .machine
        .audit_with(SWEEP_BUDGET, is_engine_owned)
        .unwrap_or_else(|error| panic!("{}: {error}", t.label));
    let mut findings = Findings {
        valuations: audit.valuations,
        ..Findings::default()
    };
    for finding in audit.findings {
        match finding {
            Finding::Shadowed { rule, behind } => findings
                .order
                .push(format!("{rule} is dead behind {behind}")),
            Finding::Covered { rule, by } => findings
                .order
                .push(format!("{rule} is dead behind {}", by.join(", "))),
            Finding::Unsatisfiable { rule } => findings.order.push(format!("{rule} is dead")),
            Finding::Unreachable { phase } => findings.unreachable.push(phase),
            Finding::DeadEnd { phase } => findings.dead_end.push(phase),
            Finding::Unread { rule, inputs } => findings.presence.push(format!(
                "{rule} lets the run go on without reading {inputs:?}"
            )),
            Finding::Unruled {
                phase,
                result,
                valuation,
            } => findings.unruled.push((phase, result, valuation)),
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
/// reaches the fallback — exactly as in production." The frozen heritage
/// table keeps it, read under v1 as its corpus records; every other table
/// is v2, where presence is refused at load (decision 0050, ruling 1).
#[test]
fn presence_is_reported_for_the_heritage_table_alone() {
    for t in shipped_tables() {
        let findings = sweep(&t);
        if t.label != "policy/phase-machine.json" {
            assert_eq!(findings.presence, Vec::<String>::new(), "{}", t.label);
            assert_eq!(t.json["schema"], "forge.phase-machine/v2", "{}", t.label);
            continue;
        }
        assert_eq!(
            findings.presence,
            [
                "REVIEW-CLEAN-UNVERIFIED lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]",
                "REVIEW-RESIDUAL-OK lets the run go on without reading [\"has_security_residual\", \"max_residual_severity\"]",
            ]
        );
        assert_eq!(t.json["schema"], "forge.phase-machine/v1");
    }
}

/// The sweep's size and its holes, per table: none, since the second
/// enactment slice (#429) named the one shape every hole had — a
/// `residual` verdict whose severity is `none` — and the compiler refuses
/// a hole. The heritage table has no hole because its fallback is
/// unconditional, which is the presence finding above.
#[test]
fn the_unruled_valuations_are_pinned_per_table() {
    let expected: BTreeMap<&str, (usize, usize)> = BTreeMap::from([
        ("policy/phase-machine.json", (57, 0)),
        ("recipes/self", (47, 0)),
        ("recipes/verify", (16, 0)),
        ("recipes/fast", (46, 0)),
        ("recipes/landing", (48, 0)),
        ("recipes/night-shift", (1072, 0)),
        ("recipes/node", (46, 0)),
        ("recipes/panel-review", (47, 0)),
        ("recipes/preflight", (16, 0)),
        ("recipes/release", (46, 0)),
        ("recipes/research", (5, 0)),
        ("recipes/research-dsh", (5, 0)),
        ("recipes/review-first", (46, 0)),
        ("recipes/standby", (46, 0)),
        ("recipes/gpt-flash", (1072, 0)),
        ("recipes/triage", (1072, 0)),
        ("recipes/wager-harness", (46, 0)),
        ("recipes/wager-harness-dsh", (46, 0)),
        ("recipes/wager-harness-muse", (46, 0)),
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
        assert_eq!(t.machine.refuse_unruled(), Ok(()), "{}", t.label);
    }
}

/// Ruling 4's named ending: where the journal recorded no ruling for a
/// `residual` verdict at severity `none`, it now records
/// `REVIEW-RESIDUAL-NONE` — a park in every delivery table and in
/// `recipes/verify`, and a stop in `recipes/preflight`, which has no
/// operator to park for. `recipes/fast` carries it once, and composition
/// carries it to every recipe that extends fast.
#[test]
fn a_residual_rated_none_is_ruled_by_its_named_rule() {
    let none = json!({
        "max_residual_severity": "none",
        "has_security_residual": false,
        "visits_implement": 1,
        "fixes_applied": false
    });
    let mut ruled: BTreeMap<String, Outcome> = BTreeMap::new();
    for t in shipped_tables() {
        if t.label == "policy/phase-machine.json" || !t.machine.phases.iter().any(|p| p == "review")
        {
            continue;
        }
        let outcome = t
            .machine
            .evaluate("review", "residual", none.as_object().unwrap());
        ruled.insert(t.label, outcome);
    }
    let park = |reason: &str| Outcome::Park {
        rule_id: "REVIEW-RESIDUAL-NONE".into(),
        reason: reason.into(),
    };
    let fast = park(
        "A residual verdict that rates its worst residual none names no debt to track \
         and no defect to return; the run parks for the operator with the notes \
         (decision 0050, ruling 4).",
    );
    let verify = park(
        "A residual verdict rated none is no verification verdict at all: it names no \
         tracked debt and no defect. The run parks for the operator with the notes \
         (decision 0050, ruling 4).",
    );
    let Some(Outcome::Ruling {
        rule_id,
        next_phase,
        severity,
        ..
    }) = ruled.remove("recipes/preflight")
    else {
        panic!("recipes/preflight rules a residual rated none");
    };
    assert_eq!(
        (rule_id.as_str(), next_phase.as_str(), severity.as_str()),
        ("REVIEW-RESIDUAL-NONE", "stop", "hard")
    );
    assert_eq!(ruled.remove("recipes/verify"), Some(verify));
    let delivery: BTreeSet<&str> = ruled.keys().map(String::as_str).collect();
    assert_eq!(delivery.len(), 14, "{delivery:?}");
    for (label, outcome) in ruled {
        assert_eq!(outcome, fast, "{label}");
    }
}

/// The compiler's refusal, on a shipped table: `recipes/self` without its
/// named park has the hole every delivery table had, loads, and is refused
/// at its first unruled valuation (decision 0050, ruling 4).
#[test]
fn recipes_self_without_its_named_park_is_refused() {
    let mut json = self_table();
    json["rules"]
        .as_array_mut()
        .unwrap()
        .retain(|rule| rule["id"] != "REVIEW-RESIDUAL-NONE");
    let refusal = Machine::from_table(&json)
        .expect("the hole is no load finding")
        .refuse_unruled()
        .unwrap_err();
    assert_eq!(
        refusal,
        PolicyError::Refused(Refusal::Totality(Finding::Unruled {
            phase: "review".into(),
            result: "residual".into(),
            valuation: vec![
                ("max_residual_severity".into(), Setting::Word("none")),
                ("visits_implement".into(), Setting::Count(0)),
            ],
        }))
    );
    assert_eq!(
        refusal.to_string(),
        "malformed phase machine table: no rule rules (review, residual) at \
         max_residual_severity=none, visits_implement=0; name it with a rule that \
         parks it and says why (decision 0050, ruling 4)"
    );
}

/// Decision 0022's arc as a property: the table that ruled a diligent
/// reviewer's note a death was total, deterministic and wrong.
#[test]
fn the_stated_properties_hold_on_every_shipped_table() {
    // Ruling 5's clean property is held at the plain verdict only. A clean
    // verdict carrying an exhausted spec defect parks in the tables below,
    // and whether the property ranges over that valuation awaits the
    // operator (`docs/evidence/decision-0050-audit.md`, item 9).
    let exhausted = json!({
        "strategy": "design",
        "spec_defect": true,
        "visits_specify": 3,
        "fixes_applied": false
    });
    let parked: Vec<String> = shipped_tables()
        .into_iter()
        .filter(|t| {
            matches!(
                t.machine
                    .evaluate("review", "clean", exhausted.as_object().unwrap()),
                Outcome::Park { .. }
            )
        })
        .map(|t| t.label)
        .collect();
    assert_eq!(
        parked,
        ["recipes/gpt-flash", "recipes/night-shift", "recipes/triage"]
    );
    for t in shipped_tables() {
        // Ruling 5: `security-hold` rules a hard stop from every phase
        // that admits it, in every table: at every swept valuation of its
        // group some rule rules it, and every rule that can is a hard stop.
        let findings = sweep(&t);
        let held: Vec<&Unruled> = findings
            .unruled
            .iter()
            .filter(|(_, result, _)| result == "security-hold")
            .collect();
        assert_eq!(held, Vec::<&Unruled>::new(), "{}", t.label);
        for rule in t
            .machine
            .rules
            .iter()
            .filter(|r| r.result == "security-hold")
        {
            assert_eq!(
                (rule.next.as_deref(), rule.severity.as_str()),
                (Some("stop"), "hard"),
                "{}: {}",
                t.label,
                rule.id
            );
        }
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
    resolve(&workspace().join("recipes/self"))
        .expect("recipes/self resolves")
        .table
}

fn position(table: &Value, id: &str) -> usize {
    table["rules"]
        .as_array()
        .unwrap()
        .iter()
        .position(|rule| rule["id"] == id)
        .unwrap_or_else(|| panic!("{id} is in the table"))
}

/// The decision's context, reproduced: exchange the two exhaustion arms,
/// which parked a high residual at the bound where the constitution says
/// it stops, and the loader refuses the table (#429).
#[test]
fn the_exchanged_self_table_is_refused_behind_its_weaker_arm() {
    let mut json = self_table();
    let above = position(&json, "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM");
    let medium = position(&json, "REVIEW-REFORGE-EXHAUSTED-MEDIUM");
    json["rules"].as_array_mut().unwrap().swap(above, medium);
    assert_eq!(
        Machine::from_table(&json).unwrap_err(),
        PolicyError::Refused(Refusal::Order(Finding::Shadowed {
            rule: "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM".into(),
            behind: "REVIEW-REFORGE-EXHAUSTED-MEDIUM".into(),
        }))
    );
}

/// Seq 335 of `implement-decision-0022-reforgin-54a88e9b`, reproduced:
/// permissive arms that read no severity shipped an absent one, security
/// flag and all. Presence refuses the v2 table by name (#429).
#[test]
fn the_0022_era_permissive_arms_are_refused() {
    let mut json = self_table();
    // The 0022-era table had no named park: behind an unconditional
    // `REVIEW-RESIDUAL-OK` it would be dead.
    json["rules"]
        .as_array_mut()
        .unwrap()
        .retain(|rule| rule["id"] != "REVIEW-RESIDUAL-NONE");
    for rule in json["rules"].as_array_mut().unwrap() {
        if rule["id"] == "REVIEW-REFORGE-EXHAUSTED-DEBT" || rule["id"] == "REVIEW-RESIDUAL-OK" {
            let when = rule["when"].as_object_mut().unwrap();
            when.retain(|key, _| !key.starts_with("max_residual_severity"));
        }
    }
    assert_eq!(
        Machine::from_table(&json).unwrap_err(),
        PolicyError::Refused(Refusal::Presence(Finding::Unread {
            rule: "REVIEW-REFORGE-EXHAUSTED-DEBT".into(),
            inputs: vec!["max_residual_severity".into()],
        }))
    );
}
