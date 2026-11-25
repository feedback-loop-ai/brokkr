//! Decision 0109 ruling 2, bound case by case: the closed non-passing
//! enum, the findings arm, the aggregate arm, and every refusal a typed
//! variant that leaves the word as the seat wrote it.

use serde_json::{json, Map, Value};

use super::super::{stamp, SiteDigest};
use super::*;

const SITE: &str = "5a1e000000000000000000000000000000000000000000000000000000000000";
const BOTH: [&str; 2] = ["max_residual_severity", "has_security_residual"];

/// One stamped finding of `severity`, flagged `security` or not.
fn finding(id: &str, severity: &str, security: bool) -> Finding {
    let site = SiteDigest::try_from(SITE.to_string()).unwrap();
    let reported = json!([{"id": id, "file": "crates/a/src/lib.rs", "summary": "a finding",
                           "severity": severity, "security": security}]);
    stamp(&reported, &site).unwrap().remove(0)
}

fn inputs(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn clear() -> Map<String, Value> {
    inputs(json!({"max_residual_severity": "none", "has_security_residual": false}))
}

fn pair() -> ResidualPair {
    ResidualPair::declared_in(&BOTH)
}

fn refused(word: &str) -> String {
    word.to_string()
}

#[test]
fn the_closed_enum_names_eight_non_passing_words_and_every_other_word_passes() {
    let non_passing = [
        "residual",
        "drift",
        "ambiguous",
        "clean-unverified",
        "fail",
        "security-hold",
        "upstream",
        "escalate",
    ];
    for word in non_passing {
        assert_eq!(NonPassing::named(word).map(NonPassing::word), Some(word));
    }
    let shipped_passing = ["clean", "pass", "consistent", "clear", "ready", "shipped"];
    for word in shipped_passing.into_iter().chain(["my-own-word"]) {
        assert_eq!(NonPassing::named(word), None, "{word}");
    }
}

#[test]
fn a_blocking_finding_refuses_every_passing_word_an_unnamed_one_included() {
    let medium = [finding("medium-one", "medium", false)];
    for word in ["clean", "consistent", "my-own-word"] {
        assert_eq!(
            bind(word, &medium, Some(&clear()), pair()),
            Err(WordRefusal::BlockingFinding {
                word: refused(word)
            })
        );
    }
    let low = [finding("low-one", "low", false)];
    assert_eq!(bind("clean", &low, Some(&clear()), pair()), Ok(()));
}

#[test]
fn a_blocking_finding_admits_residual_clean_unverified_and_the_hold() {
    let high = [finding("high-one", "high", false)];
    for word in ["residual", "clean-unverified", "security-hold", "drift"] {
        assert_eq!(bind(word, &high, None, pair()), Ok(()), "{word}");
    }
}

#[test]
fn a_security_finding_requires_the_hold_and_nothing_else_admits_it() {
    let security = [finding("token-leak", "high", true)];
    for word in ["clean", "residual", "fail", "my-own-word"] {
        assert_eq!(
            bind(word, &security, Some(&clear()), pair()),
            Err(WordRefusal::SecurityFinding {
                word: refused(word)
            })
        );
    }
    assert_eq!(bind("security-hold", &security, None, pair()), Ok(()));
    let low_security = [finding("low-leak", "low", true)];
    assert_eq!(
        bind(
            "clean",
            &low_security,
            Some(&clear()),
            ResidualPair::declared_in::<&str>(&[])
        ),
        Err(WordRefusal::SecurityFinding {
            word: refused("clean")
        })
    );
}

#[test]
fn a_declared_residual_refuses_a_passing_word() {
    let high = inputs(json!({"max_residual_severity": "high", "has_security_residual": false}));
    assert_eq!(
        bind("clean", &[], Some(&high), pair()),
        Err(WordRefusal::ResidualSeverity {
            word: refused("clean"),
            severity: Severity::High
        })
    );
    let security = inputs(json!({"max_residual_severity": "none", "has_security_residual": true}));
    assert_eq!(
        bind("clean", &[], Some(&security), pair()),
        Err(WordRefusal::SecurityResidual {
            word: refused("clean")
        })
    );
    assert_eq!(bind("residual", &[], Some(&high), pair()), Ok(()));
}

#[test]
fn an_absent_null_or_unknown_declared_input_refuses_rather_than_reads_as_none() {
    let only_severity = inputs(json!({"max_residual_severity": "none"}));
    assert_eq!(
        bind("clean", &[], Some(&only_severity), pair()),
        Err(WordRefusal::Absent {
            word: refused("clean"),
            input: ResidualInput::HasSecurityResidual
        })
    );
    assert_eq!(
        bind("clean", &[], None, pair()),
        Err(WordRefusal::Absent {
            word: refused("clean"),
            input: ResidualInput::MaxResidualSeverity
        })
    );
    for (input, value) in [
        (ResidualInput::MaxResidualSeverity, json!(null)),
        (ResidualInput::MaxResidualSeverity, json!("severe")),
        (ResidualInput::HasSecurityResidual, json!(null)),
        (ResidualInput::HasSecurityResidual, json!("false")),
    ] {
        let mut written = clear();
        written.insert(input.name().to_string(), value);
        assert_eq!(
            bind("clean", &[], Some(&written), pair()),
            Err(WordRefusal::Unreadable {
                word: refused("clean"),
                input
            })
        );
    }
}

#[test]
fn only_the_pair_the_seat_declares_is_held_and_its_other_inputs_never_are() {
    assert_eq!(
        bind("clean", &[], None, ResidualPair::declared_in::<&str>(&[])),
        Ok(())
    );
    let severity_only = ResidualPair::declared_in(&["max_residual_severity", "spec_defect"]);
    let written = inputs(json!({"max_residual_severity": "none"}));
    assert_eq!(bind("clean", &[], Some(&written), severity_only), Ok(()));
    let with_spec =
        ResidualPair::declared_in(&BOTH.into_iter().chain(["spec_defect"]).collect::<Vec<_>>());
    assert_eq!(bind("clean", &[], Some(&clear()), with_spec), Ok(()));
}

#[test]
fn a_charter_shaped_clean_is_admitted_and_an_engine_carried_follow_up_leaves_it() {
    let result = inputs(json!({
        "result": "clean",
        "inputs": {"max_residual_severity": "none", "has_security_residual": false},
        "follow_ups": [{"id": "later", "file": "a.rs", "summary": "later", "severity": "low"}],
    }));
    assert_eq!(bind_journaled(&result, "clean", pair()), Ok(()));
}

#[test]
fn a_journaled_result_is_bound_by_its_stamped_findings() {
    let mut result = inputs(json!({"result": "clean", "inputs": clear()}));
    let high = vec![finding("high-one", "high", true)];
    result.insert("findings".into(), serde_json::to_value(&high).unwrap());
    assert_eq!(
        bind_journaled(&result, "clean", pair()),
        Err(WordRefusal::SecurityFinding {
            word: refused("clean")
        })
    );
    assert_eq!(bind_journaled(&result, "security-hold", pair()), Ok(()));
    result.insert("findings".into(), json!([{"id": "unstamped"}]));
    assert_eq!(
        bind_journaled(&result, "security-hold", pair()),
        Err(WordRefusal::Findings)
    );
}

#[test]
fn each_refusal_says_what_refused_it_once() {
    let word = || refused("clean");
    let texts = [
        (WordRefusal::SecurityFinding { word: word() }, "the word 'clean' stands over a security finding, which requires 'security-hold' (decision 0109 ruling 2)"),
        (WordRefusal::BlockingFinding { word: word() }, "the passing word 'clean' stands over a blocking finding (severity above low); a blocking finding admits 'residual', 'clean-unverified' or 'security-hold' (decision 0109 ruling 2)"),
        (WordRefusal::ResidualSeverity { word: word(), severity: Severity::Medium }, "the passing word 'clean' stands over a declared max_residual_severity of 'medium' (decision 0109 ruling 2)"),
        (WordRefusal::SecurityResidual { word: word() }, "the passing word 'clean' stands over a declared has_security_residual of true (decision 0109 ruling 2)"),
        (WordRefusal::Absent { word: word(), input: ResidualInput::HasSecurityResidual }, "the passing word 'clean' omits 'has_security_residual', which the seat declares (decision 0109 ruling 2)"),
        (WordRefusal::Unreadable { word: word(), input: ResidualInput::MaxResidualSeverity }, "the passing word 'clean' carries 'max_residual_severity' as null or as no value of its vocabulary (decision 0109 ruling 2)"),
        (WordRefusal::Findings, "the result's findings are not the engine's stamped records"),
    ];
    for (refusal, text) in texts {
        assert_eq!(refusal.to_string(), text);
    }
}
