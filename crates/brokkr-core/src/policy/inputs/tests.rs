use super::*;
use crate::policy::{
    is_engine_owned, Machine, Malformed, BOOLEAN_INPUTS, COUNTER_INPUTS, ENGINE_OWNED_INPUTS,
    IDENTIFIER_INPUTS, SEVERITY_INPUTS,
};
use serde_json::{json, Value};

/// The rows of `INPUTS` a runtime filter keeps, by name.
fn rows(keep: impl Fn(&Input) -> bool) -> Vec<&'static str> {
    INPUTS
        .iter()
        .filter(|row| keep(row))
        .map(|row| row.name)
        .collect()
}

/// Each list the loader, the compiler and the engine read is the table's
/// rows of one kind or owner, in table order — computed here a second way,
/// at run time, so a row added to the table is a word added to its list.
#[test]
fn every_input_list_is_a_projection_of_the_table() {
    let lists: [(Pick, &[&str], Vec<&str>); 5] = [
        (
            Pick::Flags,
            &BOOLEAN_INPUTS,
            rows(|row| row.kind == InputKind::Flag),
        ),
        (
            Pick::Counters,
            &COUNTER_INPUTS,
            rows(|row| row.kind == InputKind::Counter),
        ),
        (
            Pick::Severities,
            &SEVERITY_INPUTS,
            rows(|row| row.kind == InputKind::Severity),
        ),
        (
            Pick::Identifiers,
            &IDENTIFIER_INPUTS,
            rows(|row| row.kind == InputKind::Identifier),
        ),
        (
            Pick::EngineOwned,
            &ENGINE_OWNED_INPUTS,
            rows(|row| row.owner == Owner::Engine),
        ),
    ];
    for (pick, list, expected) in lists {
        assert_eq!(list, expected.as_slice(), "{pick:?}");
        assert_eq!(count(pick), expected.len(), "{pick:?}");
        let projected = names::<{ INPUTS.len() }>(pick);
        assert_eq!(projected[..expected.len()], expected[..], "{pick:?}");
    }
    // The loader's refusals print the boolean list; it keeps its order.
    assert_eq!(
        BOOLEAN_INPUTS,
        [
            "skip_verify",
            "fixes_applied",
            "spec_defect",
            "has_security_residual",
            "high_risk_uncovered",
            "drift_detected",
            "dirty_worktrees",
            "fixes_docs_only",
        ]
    );
}

/// A one-rule table leaving `work` that reads `when`.
fn reading(when: Value) -> Result<Machine, Malformed> {
    Machine::parse(&json!({
        "phases": ["work", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [{"id": "R", "from": "work", "result": "ok", "next": "done",
                   "when": when, "reason": "reads one input"}],
    }))
    .map_err(|error| match error {
        crate::policy::PolicyError::Malformed(fault) => fault,
        crate::policy::PolicyError::Refused(refusal) => panic!("{refusal}"),
    })
}

/// Every row is honoured by every reader the table feeds: a condition of
/// its kind loads (or is refused, for the two kinds policy may not read),
/// the compiler's default declaration names exactly the seat-owned ones,
/// and ownership decides what a seat may declare.
#[test]
fn every_row_is_read_by_its_kind_and_owned_by_its_owner() {
    for row in INPUTS {
        let name = row.name;
        // The table's shorthand builds the row it names.
        assert_eq!(input(name, row.kind, row.owner), row);
        let seat_owned = row.owner == Owner::Seat;
        assert_eq!(is_declarable(name), seat_owned, "{name}");
        assert_eq!(is_engine_owned(name), !seat_owned, "{name}");
        let when = match row.kind {
            InputKind::Flag => json!({ name: true }),
            InputKind::Counter => json!({ format!("{name}_gte"): 2 }),
            InputKind::Severity => {
                json!({ format!("{name}_above"): "low", format!("{name}_at_most"): "high" })
            }
            InputKind::Choice { key, vocabulary } => {
                assert_eq!(super::vocabulary(name), vocabulary);
                json!({ key: [vocabulary[0]] })
            }
            InputKind::Identifier => {
                assert_eq!(
                    reading(json!({ name: "c1" })).unwrap_err(),
                    Malformed::IdentifierCondition {
                        rule: "R".into(),
                        key: name.into(),
                    }
                );
                continue;
            }
            InputKind::Record => {
                assert_eq!(
                    reading(json!({ name: true })).unwrap_err(),
                    Malformed::UnknownCondition {
                        rule: "R".into(),
                        key: name.into(),
                    }
                );
                continue;
            }
        };
        let machine = reading(when).unwrap();
        assert_eq!(
            machine.reads_counter("work", name),
            row.kind == InputKind::Counter,
            "{name}"
        );
        let declared = match seat_owned {
            true => vec![name.to_string()],
            false => Vec::new(),
        };
        assert_eq!(machine.seat_inputs("work"), declared, "{name}");
    }
    assert!(!is_declarable("invented"));
    assert_eq!(super::vocabulary("invented"), &[] as &[&str]);
}
