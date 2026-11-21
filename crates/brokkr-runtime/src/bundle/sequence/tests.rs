//! A seat's sequence, its steps' names and their results, and a dialect
//! step's operation: each refusal matched by its variant, each admitted
//! shape by its value, and every refusal's text pinned once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use serde_json::{json, Value};

use super::{
    dialect_operation, dialect_step, sequence_steps, step_name, step_results, unsupplied,
    CompileError, DialectExecution, SeatClass, SeatScope, SequenceRefusal, SequenceStep, StepAt,
    StepBody, StepFault,
};
use crate::bundle::{Charters, SiteCompiler};
use crate::dialect::Dialect;

/// The seat's own vocabulary, which the final step receives.
const SEAT: [&str; 2] = ["approved", "rejected"];

/// The seat scope a test step compiles in, under the seat's `results`.
fn scope(results: &[String]) -> SeatScope<'_> {
    SeatScope {
        dir: Path::new("."),
        results,
        secrets: &[],
        residual: brokkr_core::findings::ResidualPair::declared_in::<&str>(&[]),
        declares_change: false,
    }
}

/// The shipped openspec dialect's file.
fn openspec_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dialects/openspec.json")
}

/// The shipped openspec dialect: `validate` for specify, design and tasks,
/// `check` for clarify and analyze.
fn openspec() -> Dialect {
    Dialect::load(&openspec_path()).unwrap().0
}

/// openspec with clarify's `check` declared unsupported.
fn without_clarify_check() -> Dialect {
    let path = openspec_path();
    let mut file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    file["phases"]["clarify"]["check"] = json!({"unsupported": "no clarify check here"});
    let dialect = Dialect::parse(&path.display().to_string(), &file.to_string())
        .unwrap()
        .0;
    assert!(
        dialect.validation("clarify").is_none(),
        "clarify's check is unsupplied"
    );
    dialect
}

/// Step `what` of seat `phase`, written as `raw`; its name is what follows
/// `phase:`.
fn dialect_at<'s>(phase: &'s str, what: &'s str, raw: &'s Value) -> StepAt<'s> {
    StepAt {
        phase,
        name: &what[phase.len() + 1..],
        what,
        raw,
    }
}

/// `dialect_operation` on `step`, its dialect read as whether it is the one
/// passed in.
fn resolved<'a>(
    step: StepAt<'a>,
    dialect: Option<&'a Dialect>,
) -> Result<(&'a str, bool), SequenceRefusal<'a>> {
    dialect_operation(step, dialect)
        .map(|(operation, got)| (operation, dialect.is_some_and(|d| std::ptr::eq(d, got))))
}

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
        declares_change: false,
    }
}

/// `step_results` on `raw` as step `plan:draft`, final or not.
fn results_of(raw: &Value, final_step: bool) -> Result<Vec<String>, StepFault<'_>> {
    let seat: Vec<String> = SEAT.map(str::to_string).to_vec();
    let at = scope(&seat);
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
fn each_dialect_step_refusal_names_its_own_check() {
    let (spec, bare) = (openspec(), without_clarify_check());
    let (design, clarify, engine) = ("design:lint", "clarify:count", "clarify:engine:count");
    let seven = json!({"dialect": 7});
    assert_eq!(
        resolved(dialect_at("design", design, &seven), Some(&spec)),
        Err(SequenceRefusal::NoOperation { what: design })
    );
    let check = json!({"dialect": "check"});
    assert_eq!(
        resolved(dialect_at("design", design, &check), Some(&spec)),
        Err(SequenceRefusal::WrongOperation {
            what: design,
            operation: "check",
            phase: "design",
            expected: "validate",
        })
    );
    let bogus = json!({"dialect": "bogus"});
    assert_eq!(
        resolved(dialect_at("clarify", clarify, &bogus), Some(&spec)),
        Err(SequenceRefusal::WrongOperation {
            what: clarify,
            operation: "bogus",
            phase: "clarify",
            expected: "check",
        })
    );
    let validate = json!({"dialect": "validate"});
    assert_eq!(
        resolved(dialect_at("design", design, &validate), None),
        Err(SequenceRefusal::NoDialect {
            phase: "design",
            name: "lint",
        })
    );
    let tooled = json!({"dialect": "check", "tools": []});
    assert_eq!(
        unsupplied(dialect_at("clarify", clarify, &tooled), "check", &bare),
        Err(SequenceRefusal::ToolsWithoutSite {
            what: clarify,
            operation: "check",
        })
    );
    assert_eq!(
        unsupplied(
            dialect_at("clarify:engine", engine, &validate),
            "validate",
            &spec
        ),
        Err(SequenceRefusal::Unsupported {
            dialect: "openspec",
            phase: "clarify:engine",
            operation: "validate",
        })
    );
}

#[test]
fn a_dialect_step_s_checks_run_operation_first_and_tools_only_for_a_check() {
    let (design, engine) = ("design:lint", "clarify:engine:count");
    let check = json!({"dialect": "check"});
    assert_eq!(
        resolved(dialect_at("design", design, &check), None),
        Err(SequenceRefusal::WrongOperation {
            what: design,
            operation: "check",
            phase: "design",
            expected: "validate",
        })
    );
    let seven = json!({"dialect": 7});
    assert_eq!(
        resolved(dialect_at("design", design, &seven), None),
        Err(SequenceRefusal::NoOperation { what: design })
    );
    let tooled = json!({"dialect": "validate", "tools": []});
    assert_eq!(
        unsupplied(
            dialect_at("clarify:engine", engine, &tooled),
            "validate",
            &openspec()
        ),
        Err(SequenceRefusal::Unsupported {
            dialect: "openspec",
            phase: "clarify:engine",
            operation: "validate",
        })
    );
}

#[test]
fn a_dialect_step_its_phase_and_dialect_admit_is_admitted() {
    let (spec, bare) = (openspec(), without_clarify_check());
    let check = json!({"dialect": "check"});
    assert_eq!(
        unsupplied(
            dialect_at("clarify", "clarify:count", &check),
            "check",
            &bare
        ),
        Ok(())
    );
    let validate = json!({"dialect": "validate"});
    assert_eq!(
        resolved(dialect_at("design", "design:lint", &validate), Some(&spec)),
        Ok(("validate", true))
    );
    assert_eq!(
        resolved(dialect_at("clarify", "clarify:count", &check), Some(&spec)),
        Ok(("check", true))
    );
}

#[test]
fn a_dialect_step_without_a_realm_dialect_is_refused_in_its_text() {
    let (charters, case_origin) = (Charters::default(), BTreeMap::new());
    let compiler = &mut SiteCompiler::new(
        crate::bundle::tests::unopened(),
        (Boundary::Namespace, &crate::argv::tests::ENGINE),
        None,
        &charters,
        &[],
        &case_origin,
        Path::new(""),
    );
    let seat: Vec<String> = SEAT.map(str::to_string).to_vec();
    let validate = json!({"dialect": "validate"});
    let step = dialect_at("design", "design:lint", &validate);
    let Err(CompileError::Invalid(text)) = dialect_step(compiler, scope(&seat), step) else {
        panic!("a dialect step without a dialect must be refused");
    };
    assert_eq!(
        text,
        "phase 'design' needs a realm dialect to resolve step 'lint'"
    );
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
        (
            SequenceRefusal::NoOperation { what },
            "sequence step 'plan:draft' dialect must be 'validate' or 'check'",
        ),
        (
            SequenceRefusal::WrongOperation {
                what,
                operation: "check",
                phase,
                expected: "validate",
            },
            "sequence step 'plan:draft' uses dialect operation 'check'; phase 'plan' needs 'validate'",
        ),
        (
            SequenceRefusal::NoDialect {
                phase,
                name: "draft",
            },
            "phase 'plan' needs a realm dialect to resolve step 'draft'",
        ),
        (
            SequenceRefusal::ToolsWithoutSite {
                what,
                operation: "check",
            },
            "sequence step 'plan:draft' declares 'tools' on a dialect step whose 'check' the \
             dialect does not supply; no executable site exists to own the declaration, so it \
             could only be discarded — refused (decision 0065 slice one, design D5)",
        ),
        (
            SequenceRefusal::Unsupported {
                dialect: "openspec",
                phase,
                operation: "validate",
            },
            "dialect 'openspec' declares phase 'plan' validate unsupported",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
}
