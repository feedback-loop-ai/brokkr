//! Decision 0064's enforcement binding (its 2026-11-03 addendum, ruling
//! 1): every rule that stops or completes a run, in every bundle this tree
//! ships, composed, and in the v1 table, ends that run in a word the view
//! can name.

use super::workspace_root;
use crate::bundle::compose::resolve;
use crate::bundle::sequence::stops_run;
use brokkr_core::ending::Ending;
use brokkr_core::policy::{Machine, RuleSeverity};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The ending each rule of `machine` that stops or completes a run ends
/// it in, by rule id, as the engine ends it: a rule whose `next` is
/// terminal stops the run at `stop` and completes it anywhere else. A
/// completing rule from review reads its own severity as the last
/// review's; a park ends nothing.
fn endings(machine: &Machine) -> Vec<(&str, Ending)> {
    let mut endings = Vec::new();
    for rule in &machine.rules {
        let Some(next) = rule.next.as_deref() else {
            continue;
        };
        if !machine.terminal.iter().any(|terminal| terminal == next) {
            continue;
        }
        let ending = if stops_run(machine, next) {
            Ending::ruled(&rule.from, &rule.result)
        } else {
            let review = (rule.from == "review").then(|| RuleSeverity::named(&rule.severity));
            Ending::completed(review.flatten())
        };
        endings.push((rule.id.as_str(), ending));
    }
    endings
}

/// Every composed bundle under `recipes/`, by its directory, and the v1
/// table, each as the table its machine is read from. A bundle that does
/// not compose fails the sweep by name rather than leaving it.
fn shipped_tables() -> BTreeMap<String, Value> {
    let root = workspace_root();
    let mut tables = BTreeMap::new();
    for entry in std::fs::read_dir(root.join("recipes")).unwrap() {
        let path = entry.unwrap().path();
        if path.join("bundle.json").is_file() {
            let resolved = resolve(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
            let name = path.strip_prefix(&root).unwrap().display().to_string();
            tables.insert(name, resolved.table);
        }
    }
    let v1 = std::fs::read_to_string(root.join("policy/phase-machine.json")).unwrap();
    tables.insert(
        "policy/phase-machine.json".into(),
        serde_json::from_str(&v1).unwrap(),
    );
    tables
}

#[test]
fn every_shipped_rule_that_ends_a_run_ends_it_in_a_named_word() {
    let tables = shipped_tables();
    let (mut unclassified, mut reached) = (Vec::<String>::new(), BTreeSet::new());
    for (name, table) in &tables {
        let machine = Machine::from_table(table).unwrap_or_else(|error| panic!("{name}: {error}"));
        for (rule, ending) in endings(&machine) {
            if ending == Ending::Unclassified {
                unclassified.push(format!("{name}: {rule}"));
            }
            reached.insert(ending);
        }
    }
    assert_eq!(unclassified, Vec::<String>::new());
    // The sweep read the recipes and the v1 table, whose `env-degraded`
    // is the only infrastructure stop; and no policy stop is `exhausted`,
    // which is the operator's word.
    let expected = [
        Ending::Shipped,
        Ending::ShippedFlagged,
        Ending::Refused,
        Ending::Failed,
        Ending::Blocked,
        Ending::Infrastructure,
    ];
    assert_eq!(reached, BTreeSet::from(expected));
}
