//! `phases.rs`'s own unit tests: each phase lint refuses in its own
//! variant, naming what it names in its place, and every variant reads as
//! the text the operator has always seen (#353).

use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::bundle::{GateAssessment, Limits, SeatBody};

/// draft → review → done, with a way out to stop; `extra` adds rules.
fn gated(first: &str, extra: &[Value]) -> Machine {
    let mut rules = vec![
        json!({"id": "DRAFTED", "from": first, "result": "complete", "next": "review",
               "reason": "drafted"}),
        json!({"id": "GAVE-UP", "from": first, "result": "blocked", "next": "stop",
               "reason": "gave up"}),
        json!({"id": "PASSED", "from": "review", "result": "clean", "next": "done",
               "reason": "passed"}),
    ];
    rules.extend_from_slice(extra);
    Machine::from_table(&json!({
        "phases": [first, "review", "done", "stop"],
        "initial": first,
        "terminal": ["done", "stop"],
        "rules": rules,
    }))
    .unwrap()
}

/// A seat whose body is an empty sequence: the lints read only its key.
fn seat() -> Seat {
    Seat {
        gate: GateAssessment::AllGate,
        results: Vec::new(),
        limits: Limits::default(),
        inputs: Vec::new(),
        secrets: Vec::new(),
        body: SeatBody::Sequence { steps: Vec::new() },
    }
}

/// The shipped OpenSpec dialect, its every rendered phase pinned.
fn openspec() -> Dialect {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dialects/openspec.json");
    Dialect::load(&path).unwrap().0
}

#[test]
fn a_path_around_the_protected_phase_names_the_terminal_and_the_gate() {
    let shortcut = json!({"id": "SHORTCUT", "from": "draft", "result": "rushed",
                          "next": "done", "reason": "skipped review"});
    let machine = gated("draft", &[shortcut]);
    assert_eq!(
        assert_phase_unavoidable(&machine, "review"),
        Err(PhaseRefusal::Avoidable {
            terminal: "done",
            protected: "review",
        })
    );
    assert_eq!(
        protected_phase(&json!({}), &machine),
        Err(PhaseRefusal::Avoidable {
            terminal: "done",
            protected: "review",
        })
    );
}

#[test]
fn a_table_whose_every_path_passes_review_is_admitted() {
    let machine = gated("draft", &[]);
    assert_eq!(assert_phase_unavoidable(&machine, "review"), Ok(()));
    assert_eq!(
        protected_phase(&json!({}), &machine),
        Ok("review".to_string())
    );
}

#[test]
fn a_protected_phase_the_table_lacks_is_named() {
    let machine = gated("draft", &[]);
    let config = json!({"protected_phase": "audit"});
    assert_eq!(
        protected_phase(&config, &machine),
        Err(PhaseRefusal::NoProtectedPhase { phase: "audit" })
    );
}

#[test]
fn the_first_unseated_phase_is_named_and_a_seated_table_is_admitted() {
    let machine = gated("draft", &[]);
    let mut seats = BTreeMap::new();
    assert_eq!(
        unseated(&machine, &seats),
        Err(PhaseRefusal::Unseated { phase: "draft" })
    );
    seats.insert("draft".to_string(), seat());
    assert_eq!(
        unseated(&machine, &seats),
        Err(PhaseRefusal::Unseated { phase: "review" })
    );
    seats.insert("review".to_string(), seat());
    assert_eq!(unseated(&machine, &seats), Ok(()));
}

#[test]
fn a_mapped_realm_without_a_dialect_names_the_realm_and_the_phase() {
    let machine = gated("specify", &[]);
    assert_eq!(
        refuse_undeclared_dialect(&machine, Some("house"), None),
        Err(PhaseRefusal::UndeclaredDialect {
            realm: "house",
            phase: "specify",
        })
    );
    assert_eq!(refuse_undeclared_dialect(&machine, None, None), Ok(()));
}

#[test]
fn a_pin_without_a_rendered_phase_names_the_dialect_and_the_phase() {
    let mut dialect = openspec();
    let full = dialect_prompts(true, Some(&dialect)).unwrap();
    assert_eq!(full.len(), rendered_phases().count());
    dialect.rendered.remove("tasks");
    assert_eq!(
        dialect_prompts(true, Some(&dialect)),
        Err(PhaseRefusal::MissingRendered {
            dialect: "openspec",
            phase: "tasks",
        })
    );
}

#[test]
fn every_phase_refusal_reads_as_it_always_has() {
    let texts = [
        PhaseRefusal::MissingRendered {
            dialect: "openspec",
            phase: "tasks",
        },
        PhaseRefusal::UndeclaredDialect {
            realm: "house",
            phase: "specify",
        },
        PhaseRefusal::NoProtectedPhase { phase: "audit" },
        PhaseRefusal::Unseated { phase: "draft" },
        PhaseRefusal::Avoidable {
            terminal: "done",
            protected: "review",
        },
    ]
    .map(|refusal| refusal.to_string());
    assert_eq!(
        texts,
        [
            "dialect 'openspec' pin carries no rendered instructions for phase 'tasks'",
            "realm 'house' declares no dialect, but phase 'specify' needs one",
            "policy has no 'audit' phase; the protected review gate is non-removable \
             (extension model, layer 1)",
            "non-terminal phase 'draft' has no seat (no executor can run it)",
            "policy reaches terminal 'done' without passing 'review'; a path to shipping \
             that bypasses the protected review gate is constitutionally rejected",
        ]
    );
}
