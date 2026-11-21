//! Decision 0007's input-provenance lint, check by check: each refusal is
//! its own variant, read exactly, and what each admits is what it always
//! admitted (#353).

use std::path::PathBuf;

use brokkr_core::policy::Machine;
use serde_json::json;

use super::{declared, seat_inputs, InputRefusal};
use crate::bundle::{CompileError, ResolvedSeat};

/// A planted table whose one `review` rule reads two seat-declarable flags.
fn machine() -> Machine {
    Machine::from_table(&json!({
        "phases": ["review", "shipped"],
        "initial": "review",
        "terminal": ["shipped"],
        "rules": [{
            "id": "REVIEW-SHIPPED",
            "from": "review",
            "result": "complete",
            "when": {"spec_defect": false, "has_security_residual": false},
            "next": "shipped",
            "reason": "clean",
        }],
    }))
    .unwrap()
}

/// The inputs `review`'s rules reference, sorted as the machine reads them.
fn referenced() -> Vec<String> {
    let referenced = machine().seat_inputs("review");
    assert_eq!(referenced, ["has_security_residual", "spec_defect"]);
    referenced
}

/// Every check refuses in its own variant, in the lint's order.
#[test]
fn each_input_check_refuses_in_its_own_variant() {
    let referenced = referenced();
    let not_array = json!("spec_defect");
    assert_eq!(
        declared("review", Some(&not_array), &referenced),
        Err(InputRefusal::NotStrings { phase: "review" })
    );
    // Engine-owned is refused before the unknown check would see it.
    let engine = json!(["spec_defect", "strategy", "nonsense"]);
    assert_eq!(
        declared("review", Some(&engine), &referenced),
        Err(InputRefusal::EngineOwned {
            phase: "review",
            name: "strategy"
        })
    );
    let unknown = json!(["spec_defect", "nonsense"]);
    assert_eq!(
        declared("review", Some(&unknown), &referenced),
        Err(InputRefusal::Unknown {
            phase: "review",
            name: "nonsense"
        })
    );
    let lacking = json!(["spec_defect"]);
    assert_eq!(
        declared("review", Some(&lacking), &referenced),
        Err(InputRefusal::Undeclared {
            phase: "review",
            needed: "has_security_residual"
        })
    );
}

/// No declaration takes the referenced inputs; a complete one is itself;
/// a non-string element is dropped silently, as it always was.
#[test]
fn an_admitted_declaration_reads_as_it_always_has() {
    let referenced = referenced();
    assert_eq!(
        declared("review", None, &referenced),
        Ok(referenced.clone())
    );
    let complete = json!([
        "spec_defect",
        "has_security_residual",
        "high_risk_uncovered"
    ]);
    assert_eq!(
        declared("review", Some(&complete), &referenced),
        Ok(vec![
            "spec_defect".to_string(),
            "has_security_residual".to_string(),
            "high_risk_uncovered".to_string(),
        ])
    );
    let mixed = json!(["spec_defect", 7, "has_security_residual", null]);
    assert_eq!(
        declared("review", Some(&mixed), &referenced),
        Ok(vec![
            "spec_defect".to_string(),
            "has_security_residual".to_string(),
        ])
    );
}

/// An agent's declaration is the one judged, over the seat's own, and its
/// refusal reaches the compile as the operator's text.
#[test]
fn an_agent_declaration_is_judged_over_the_seat_s_own() {
    let agent = |inputs: &[&str]| {
        Some(ResolvedSeat {
            role_path: PathBuf::new(),
            command: Vec::new(),
            candidates: Vec::new(),
            limits: None,
            inputs: Some(inputs.iter().map(|name| name.to_string()).collect()),
            hands: None,
            results: None,
        })
    };
    let raw = json!({"inputs": "not an array"});
    let admitted = seat_inputs(
        &machine(),
        "review",
        &raw,
        &agent(&["has_security_residual", "spec_defect"]),
    );
    assert_eq!(admitted.unwrap(), ["has_security_residual", "spec_defect"]);
    let refused = seat_inputs(&machine(), "review", &raw, &agent(&["visits_review"]));
    match refused {
        Err(CompileError::Invalid(text)) => assert_eq!(
            text,
            "seat 'review' declares engine-owned input 'visits_review'; journal-computed \
             truth is never accepted from a seat"
        ),
        other => panic!("expected the engine-owned refusal, got {other:?}"),
    }
}

/// Each variant's text, written out apart from its `#[error]`: the words
/// main has always printed.
#[test]
fn every_seat_input_refusal_reads_as_it_always_has() {
    let (phase, name) = ("review", "spec_defect");
    for (refusal, text) in [
        (
            InputRefusal::NotStrings { phase },
            "seat 'review' inputs must be an array of strings",
        ),
        (
            InputRefusal::EngineOwned { phase, name },
            "seat 'review' declares engine-owned input 'spec_defect'; journal-computed truth \
             is never accepted from a seat",
        ),
        (
            InputRefusal::Unknown { phase, name },
            "seat 'review' declares unknown input 'spec_defect'; known: the evaluator's closed \
             vocabulary minus engine-owned",
        ),
        (
            InputRefusal::Undeclared {
                phase,
                needed: name,
            },
            "phase 'review' rules reference input 'spec_defect' but seat 'review' does not \
             declare it; the rule could never fire from seat data",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}
