//! Decision 0041 ruling 6, as structure rather than recipe prose.

use std::path::{Path, PathBuf};

use brokkr_core::policy::{Outcome, STRATEGIES};
use brokkr_runtime::{Bundle, CompileError, ENGINE_VERSION};
use serde_json::json;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn compile(path: &Path) -> Bundle {
    let root = workspace();
    Bundle::compile_with(path, &root.join("agents"), &root.join("adapters"))
        .unwrap_or_else(|error| panic!("{} compiles: {error}", path.display()))
}

fn triage() -> Bundle {
    compile(&workspace().join("recipes/triage"))
}

#[test]
fn triage_is_the_only_front_door_and_each_class_has_one_rule() {
    let bundle = triage();
    assert_eq!(bundle.machine.initial, "triage");
    assert_eq!(
        bundle.machine.phases,
        [
            "triage",
            "design",
            "implement",
            "verify",
            "review",
            "ship",
            "done",
            "stop",
        ]
    );
    assert!(!bundle.seats.contains_key("intake"));

    for strategy in STRATEGIES {
        let rules: Vec<_> = bundle
            .machine
            .rules
            .iter()
            .filter(|rule| rule.from == "triage" && rule.result == strategy)
            .collect();
        assert_eq!(rules.len(), 1, "{strategy} must have exactly one arm");
        match strategy {
            "chore" | "feature" => assert_eq!(rules[0].next.as_deref(), Some("implement")),
            "design" | "engine" => assert_eq!(rules[0].next.as_deref(), Some("design")),
            "escalate" => assert_eq!(rules[0].next, None, "escalate parks"),
            _ => unreachable!("STRATEGIES is closed"),
        }
    }

    for rule in bundle
        .machine
        .rules
        .iter()
        .filter(|rule| rule.from == "triage")
    {
        assert!(
            rule.next.is_none() || matches!(rule.next.as_deref(), Some("design" | "implement")),
            "{} skips from triage to {:?}",
            rule.id,
            rule.next
        );
    }
}

#[test]
fn oversized_returns_once_then_parks_and_review_stays_fasts_gate() {
    let bundle = triage();
    let below = json!({"visits_triage": 1});
    assert!(matches!(
        bundle.machine.evaluate(
            "implement",
            "oversized",
            below.as_object().unwrap()
        ),
        Outcome::Ruling { ref rule_id, ref next_phase, .. }
            if rule_id == "IMPL-OVERSIZED" && next_phase == "triage"
    ));
    let exhausted = json!({"visits_triage": 2});
    assert!(matches!(
        bundle.machine.evaluate(
            "implement",
            "oversized",
            exhausted.as_object().unwrap()
        ),
        Outcome::Park { ref rule_id, .. } if rule_id == "IMPL-OVERSIZED-EXHAUSTED"
    ));

    assert_eq!(bundle.protected_phase, "review");
    let fast = compile(&workspace().join("recipes/fast"));
    let review_shape = |bundle: &Bundle| {
        bundle
            .machine
            .rules
            .iter()
            .filter(|rule| rule.from == "review")
            .map(|rule| {
                (
                    rule.id.clone(),
                    rule.result.clone(),
                    rule.next.clone(),
                    rule.reason.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let routed = review_shape(&bundle);
    assert_eq!(
        routed
            .iter()
            .take(4)
            .map(|rule| rule.0.as_str())
            .collect::<Vec<_>>(),
        [
            "REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED",
            "REVIEW-CLEAN-SPEC-DEFECT",
            "REVIEW-SPEC-DEFECT-EXHAUSTED",
            "REVIEW-SPEC-DEFECT",
        ],
        "design-bearing routes keep the specification return edges"
    );
    assert_eq!(&routed[4..], review_shape(&fast).as_slice());
}

#[test]
fn strategy_is_engine_owned_a_claim_is_dropped_and_a_declaration_refused() {
    assert!(brokkr_runtime::bundle::is_engine_owned("strategy"));
    assert!(brokkr_runtime::bundle::ENGINE_OWNED_INPUTS.contains(&"strategy"));

    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("policy.json"),
        serde_json::to_vec_pretty(&json!({
            "phases": ["pre", "work", "done"],
            "initial": "pre",
            "terminal": ["done"],
            "rules": [
                {"id": "PRE", "from": "pre", "result": "complete",
                 "next": "work", "reason": "work next"},
                {"id": "WORK", "from": "work", "result": "complete",
                 "next": "done", "reason": "done"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(temp.path().join("role.md"), "# work\n").unwrap();
    std::fs::write(
        temp.path().join("bundle.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "declares-strategy",
            "policy": "policy.json",
            "protected_phase": "work",
            "seats": {
                "pre": {"role": "role.md", "results": ["complete"],
                        "driver": {"command": ["unused"]}},
                "work": {
                    "role": "role.md",
                    "results": ["complete"],
                    "inputs": ["strategy"],
                    "driver": {"command": ["unused"]}
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let root = workspace();
    let error = Bundle::compile_with(temp.path(), &root.join("agents"), &root.join("adapters"))
        .unwrap_err();
    assert!(matches!(error, CompileError::Invalid(_)));
    assert!(
        error.to_string().contains("engine-owned input 'strategy'"),
        "{error}"
    );

    // A seat claim is dropped by the same predicate the decision path uses;
    // the end-to-end fake-driver proof asserts the journal keeps the fold's
    // value. Keep the engine-version use here so a future ownership move also
    // moves the recipe's compile witness rather than becoming dead test prose.
    assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
}
