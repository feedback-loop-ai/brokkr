//! A seat's phase, its results and the dialect wrapper's verify body,
//! check by check: each refusal is its own variant, read exactly, and what
//! each admits is what it always admitted (#353).

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_core::policy::Machine;
use serde_json::{json, Value};

use super::{covered_results, declared_results, known_phase, wrapped_body, SeatRefusal};
use crate::bundle::assemble::seat_results;
use crate::bundle::{Aggregate, CompileError, Invocation, SeatBody, SeatClass};

/// The smallest live table over `phases`: a chain from the first to the
/// last, each step taken on `complete`, and one rule to the terminal per
/// extra `(from, result)` pair.
fn machine(phases: &[&str], covered: &[(&str, &str)]) -> Machine {
    let terminal = phases[phases.len() - 1];
    let chain = phases.windows(2).map(|pair| (pair[0], "complete", pair[1]));
    let extra = covered
        .iter()
        .map(|&(from, result)| (from, result, terminal));
    let rules: Vec<Value> = chain
        .chain(extra)
        .map(|(from, result, next)| {
            json!({
                "id": format!("{from}-{result}"),
                "from": from,
                "result": result,
                "next": next,
                "reason": "planted",
            })
        })
        .collect();
    Machine::from_table(&json!({
        "phases": phases,
        "initial": phases[0],
        "terminal": [terminal],
        "rules": rules,
    }))
    .unwrap()
}

/// A lone inline driver, the body a single seat compiles to.
fn single() -> Invocation {
    Invocation::Single {
        role_path: PathBuf::new(),
        command: vec!["true".to_string()],
        candidates: Vec::new(),
        written: Vec::new(),
        class: SeatClass::Gate,
    }
}

/// A select seat over `default`, with no named case.
fn select(default: Option<SeatBody>) -> SeatBody {
    SeatBody::Select {
        cases: BTreeMap::new(),
        default: default.map(Box::new),
    }
}

/// A phase the machine has is admitted; one it lacks is refused by name.
#[test]
fn a_seat_s_phase_is_one_the_machine_has() {
    let machine = machine(&["review", "shipped"], &[]);
    assert_eq!(known_phase(&machine, "review"), Ok(()));
    assert_eq!(
        known_phase(&machine, "deploy"),
        Err(SeatRefusal::UnknownPhase { phase: "deploy" })
    );
}

/// Absent, empty, not an array, or holding no string: each is NoResults.
/// A non-string element is dropped before the emptiness test.
#[test]
fn a_seat_s_results_are_its_strings_and_never_none() {
    let refused = Err(SeatRefusal::NoResults { phase: "review" });
    for raw in [
        json!({}),
        json!({"results": []}),
        json!({"results": "pass"}),
        json!({"results": [7]}),
    ] {
        assert_eq!(declared_results("review", &raw), refused, "{raw}");
    }
    let mixed = json!({"results": ["pass", 7]});
    assert_eq!(
        declared_results("review", &mixed),
        Ok(vec!["pass".to_string()])
    );
}

/// The first result no rule covers is the one named; a covered set is
/// admitted.
#[test]
fn the_first_uncovered_result_is_the_one_refused() {
    let machine = machine(&["review", "shipped"], &[("review", "pass")]);
    let results = ["pass".to_string(), "stray".to_string()];
    assert_eq!(
        covered_results(&machine, "review", &results),
        Err(SeatRefusal::Uncovered {
            phase: "review",
            result: "stray"
        })
    );
    assert_eq!(covered_results(&machine, "review", &results[..1]), Ok(()));
}

/// Decision 0041: `implement`'s `oversized` needs no rule while the machine
/// has no `triage`, and is ordinary coverage once it does.
#[test]
fn implement_s_oversized_is_reserved_only_before_triage() {
    let results = ["complete".to_string(), "oversized".to_string()];
    let before = machine(&["implement", "done"], &[]);
    assert_eq!(covered_results(&before, "implement", &results), Ok(()));
    let after = machine(&["implement", "triage", "done"], &[]);
    assert_eq!(
        covered_results(&after, "implement", &results),
        Err(SeatRefusal::Uncovered {
            phase: "implement",
            result: "oversized"
        })
    );
}

/// The panel marker's refusal still fires between the two results checks,
/// in `Aggregate`'s own words.
#[test]
fn the_member_marker_refuses_in_its_own_words_between_the_checks() {
    let machine = machine(&["review", "shipped"], &[]);
    let raw = json!({"results": ["pass", Aggregate::MEMBER_SCHEMA_INVALID]});
    match seat_results(&machine, "review", &raw) {
        Err(CompileError::Invalid(text)) => assert_eq!(
            text,
            "seat 'review' may not declare '__member-schema-invalid__': a malformed panel \
             member's marker is never a reportable result"
        ),
        other => panic!("expected the marker refusal, got {other:?}"),
    }
}

/// A sequence or a select with no default selects nothing the wrapper can
/// take; a select whose default is an invocation is not one itself. Both
/// print the same words, and are told apart by their variants.
#[test]
fn each_wrapper_check_refuses_in_its_own_variant() {
    let sequence = SeatBody::Sequence { steps: Vec::new() };
    assert_eq!(
        wrapped_body(&sequence).err(),
        Some(SeatRefusal::WrappedUnselected)
    );
    assert_eq!(
        wrapped_body(&select(None)).err(),
        Some(SeatRefusal::WrappedUnselected)
    );
    let defaulted = select(Some(SeatBody::Invocation(single())));
    assert_eq!(
        wrapped_body(&defaulted).err(),
        Some(SeatRefusal::WrappedNotInvocation)
    );
    let seat = SeatBody::Invocation(single());
    let (_, invocation) = wrapped_body(&seat).unwrap();
    assert!(
        matches!(invocation, Invocation::Single { command, .. } if command == &["true"]),
        "{invocation:?}"
    );
}

/// Each variant's text, written out apart from its `#[error]`: the words
/// main has always printed.
#[test]
fn every_seat_refusal_reads_as_it_always_has() {
    let (phase, result) = ("review", "stray");
    let wrapped = "dialect verify currently requires a single or panel verify seat";
    for (refusal, text) in [
        (
            SeatRefusal::UnknownPhase { phase },
            "seat 'review' names a phase the policy does not have",
        ),
        (
            SeatRefusal::NoResults { phase },
            "seat 'review' needs non-empty 'results'",
        ),
        (
            SeatRefusal::Uncovered { phase, result },
            "seat 'review' may emit 'stray' but no rule covers it; result variants without an \
             outer rule are rejected",
        ),
        (SeatRefusal::WrappedUnselected, wrapped),
        (SeatRefusal::WrappedNotInvocation, wrapped),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}
