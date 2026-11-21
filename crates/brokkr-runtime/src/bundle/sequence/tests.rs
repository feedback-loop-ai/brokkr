//! A seat's sequence, its steps' names and their results: each refusal
//! matched by its variant, each admitted shape by its value, and every
//! refusal's text pinned once.

use std::path::Path;

use serde_json::{json, Value};

use super::{
    sequence_steps, step_name, step_results, CompileError, DialectExecution, SeatClass, SeatScope,
    SequenceRefusal, SequenceStep, StepAt, StepBody, StepFault,
};

/// The seat's own vocabulary, which the final step receives.
const SEAT: [&str; 2] = ["approved", "rejected"];

/// A step already parsed under `name`, as `step_name` sees its forerunners.
fn parsed(name: &str) -> SequenceStep {
    SequenceStep {
        name: name.to_string(),
        class: SeatClass::Gate,
        results: Vec::new(),
        body: StepBody::Dialect {
            execution: DialectExecution {
                argv: Vec::new(),
                state: None,
            },
        },
        ends_on: Vec::new(),
        hard_results: Vec::new(),
        residual: brokkr_core::findings::ResidualPair::declared_in::<&str>(&[]),
    }
}

/// `step_results` on `raw` as step `plan:draft`, final or not.
fn results_of(raw: &Value, final_step: bool) -> Result<Vec<String>, StepFault<'_>> {
    let seat: Vec<String> = SEAT.map(str::to_string).to_vec();
    let at = SeatScope {
        dir: Path::new("."),
        results: &seat,
        secrets: &[],
        residual: brokkr_core::findings::ResidualPair::declared_in::<&str>(&[]),
    };
    let step = StepAt {
        phase: "plan",
        name: "draft",
        what: "plan:draft",
        raw,
    };
    step_results(at, step, final_step)
}

/// The refusal `step_results` returned for `raw`, or a panic naming what
/// came back instead.
fn refusal_of(raw: &Value, final_step: bool) -> SequenceRefusal<'_> {
    match results_of(raw, final_step) {
        Err(StepFault::Refused(refusal)) => refusal,
        other => panic!("expected a sequence refusal, got {other:?}"),
    }
}

#[test]
fn a_sequence_that_is_no_array_or_one_step_is_refused_by_its_own_check() {
    let not_array = json!({"sequence": "bad"});
    assert_eq!(
        sequence_steps("plan", &not_array),
        Err(SequenceRefusal::NotArray { phase: "plan" })
    );
    let one_step = json!({"sequence": [{"name": "x"}]});
    assert_eq!(
        sequence_steps("plan", &one_step),
        Err(SequenceRefusal::TooFew { phase: "plan" })
    );
    let two = json!({"sequence": [{"name": "x"}, {"name": "y"}]});
    assert_eq!(
        sequence_steps("plan", &two),
        Ok(&[json!({"name": "x"}), json!({"name": "y"})][..])
    );
}

#[test]
fn a_nameless_or_case_insensitively_repeated_step_is_refused_by_its_own_check() {
    let nameless = json!({"name": ""});
    assert_eq!(
        step_name("plan", 0, &nameless, &[]),
        Err(SequenceRefusal::Nameless {
            phase: "plan",
            index: 0
        })
    );
    assert_eq!(
        step_name("plan", 2, &json!({}), &[parsed("a"), parsed("b")]),
        Err(SequenceRefusal::Nameless {
            phase: "plan",
            index: 2
        })
    );
    let upper = json!({"name": "B"});
    assert_eq!(
        step_name("plan", 1, &upper, &[parsed("b")]),
        Err(SequenceRefusal::Duplicate {
            phase: "plan",
            name: "B"
        })
    );
    let other = json!({"name": "c"});
    assert_eq!(step_name("plan", 1, &other, &[parsed("b")]), Ok("c"));
}

#[test]
fn each_results_refusal_names_its_own_check_though_two_share_their_words() {
    let what = "plan:draft";
    assert_eq!(
        refusal_of(&json!({"results": ["done"]}), true),
        SequenceRefusal::FinalWritesResults { what }
    );
    assert_eq!(
        refusal_of(&json!({"results": []}), false),
        SequenceRefusal::EmptyResults { what }
    );
    assert_eq!(
        refusal_of(&json!({"results": [7]}), false),
        SequenceRefusal::EmptyResults { what }
    );
    assert_eq!(
        refusal_of(&json!({}), false),
        SequenceRefusal::NoVocabulary { what }
    );
    assert_eq!(
        refusal_of(&json!({"aggregate": "no-such-aggregate"}), false),
        SequenceRefusal::NoVocabulary { what }
    );
}

#[test]
fn the_member_marker_refusal_is_carried_through_unchanged() {
    let marked = json!({"results": ["a", "__member-schema-invalid__"]});
    let Err(StepFault::Compile(CompileError::Invalid(text))) = results_of(&marked, false) else {
        panic!("expected the aggregate's own refusal");
    };
    assert_eq!(
        text,
        "sequence step 'plan:draft' may not declare '__member-schema-invalid__': a malformed \
         panel member's marker is never a reportable result"
    );
}

#[test]
fn a_step_s_vocabulary_is_its_own_strings_its_aggregate_s_or_the_seat_s() {
    let mixed = json!({"results": ["a", 7]});
    assert_eq!(results_of(&mixed, false).unwrap(), ["a"]);
    let aggregate = json!({"aggregate": "unanimous-pass"});
    assert_eq!(results_of(&aggregate, false).unwrap(), ["pass", "fail"]);
    assert_eq!(results_of(&json!({}), true).unwrap(), SEAT);
}

#[test]
fn every_sequence_refusal_reads_as_it_always_has() {
    let (phase, what) = ("plan", "plan:draft");
    let cases = [
        (
            SequenceRefusal::NotArray { phase },
            "seat 'plan' sequence must be an array",
        ),
        (
            SequenceRefusal::TooFew { phase },
            "seat 'plan' sequence needs at least two steps; a one-step sequence is a single seat",
        ),
        (
            SequenceRefusal::Nameless { phase, index: 0 },
            "seat 'plan' sequence step 0 needs a non-empty 'name'",
        ),
        (
            SequenceRefusal::Duplicate { phase, name: "B" },
            "seat 'plan' sequence has duplicate step name 'B' (step names are case-insensitive)",
        ),
        (
            SequenceRefusal::FinalWritesResults { what },
            "sequence step 'plan:draft' is final and receives the seat's results; \
             remove its ignored 'results'",
        ),
        (
            SequenceRefusal::EmptyResults { what },
            "sequence step 'plan:draft' needs its own non-empty 'results' vocabulary",
        ),
        (
            SequenceRefusal::NoVocabulary { what },
            "sequence step 'plan:draft' needs its own non-empty 'results' vocabulary",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
}
