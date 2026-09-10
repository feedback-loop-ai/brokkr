//! The `recipes/gpt-flash` strategy's defining shape, as compiled data
//! rather than recipe prose.
//!
//! The feature framing requires that EVERY strategy keeps the same
//! division: GPT Sol rules triage, specification, clarification, planning
//! and analysis; DeepSeek Flash 4.1 implements; a GPT/Flash review panel
//! states positions; and the GPT Astra chief alone rules the protected
//! review phase. The recipe extends `recipes/triage`, so the deterministic
//! verify/ship/validate gates are inherited rather than restated, and the
//! scoped `gpt-flash-*` offices replace the standard roster without
//! hiring a Claude model or a fallback Flash.
//!
//! `library_data.rs` resolves each office in isolation and
//! `crucible_review_sequence.rs` pins the shared panel-then-chief
//! mechanics on `recipes/triage`; this file is the missing link between
//! them: the compiled GPT/Flash recipe itself. It would stay green only
//! if the strategy assignments, panel composition, chief gate, scoped
//! roster and inherited gates all survive an edit.
//!
//! Decision 0058 rules the scoped roster this file checks: the fifteen
//! `gpt-flash-*` offices are ordinary library agents that reuse their
//! charters and pin exactly one model, so the mandated crew is forced and
//! no fallback can silently hire another vendor or an older Flash.

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_runtime::{Aggregate, Bundle, Candidate, SeatBody, SeatClass, StepBody};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn compile(relative: &str) -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join(relative),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|error| panic!("{relative} must compile: {error}"))
}

fn gpt_flash() -> Bundle {
    compile("recipes/gpt-flash")
}

fn triage() -> Bundle {
    compile("recipes/triage")
}

/// One resolved invocation out of a single-driver body, asserting the
/// office was pinned to exactly one model.
fn sole_candidate<'a>(body: &'a SeatBody, what: &str) -> &'a Candidate {
    let SeatBody::Single { candidates, .. } = body else {
        panic!("{what} must be a single driver");
    };
    assert_eq!(
        candidates.len(),
        1,
        "{what} must pin one model, so no older Flash can be reached silently"
    );
    &candidates[0]
}

fn sole_step_candidate<'a>(body: &'a StepBody, what: &str) -> &'a Candidate {
    let StepBody::Single { candidates, .. } = body else {
        panic!("{what} must be a single driver");
    };
    assert_eq!(candidates.len(), 1, "{what} must pin one model");
    &candidates[0]
}

fn select_cases<'a>(bundle: &'a Bundle, seat: &str) -> &'a BTreeMap<String, SeatBody> {
    match &bundle.seats[seat].body {
        SeatBody::Select { cases, default, .. } => {
            assert!(default.is_none(), "{seat} must select every strategy");
            cases
        }
        other => panic!("{seat} must select by strategy, found {other:?}"),
    }
}

fn agent_record<'a>(bundle: &'a Bundle, site: &str) -> &'a serde_json::Value {
    &bundle.manifest["agents"][site]
}

fn assert_model(bundle: &Bundle, site: &str, agent: &str, model: &str, provider: &str) {
    let record = agent_record(bundle, site);
    assert_eq!(
        record["agent"].as_str(),
        Some(agent),
        "{site} hires the wrong office"
    );
    assert_eq!(
        record["model"].as_str(),
        Some(model),
        "{site} pins the wrong model"
    );
    assert_eq!(
        record["provider"].as_str(),
        Some(provider),
        "{site} lands on the wrong provider"
    );
    assert_eq!(record["chosen_index"], 0);
    assert_eq!(
        record["chain"].as_array().map(Vec::len),
        Some(1),
        "{site} must carry a one-link chain, not a fallback"
    );
    assert_eq!(
        record["notices"].as_array().map(Vec::len),
        Some(0),
        "{site} must resolve with no capability gap"
    );
}

/// The four delivery strategies in the compiled `BTreeMap` order, and the
/// Flash office each one hires.
const IMPLEMENT: [(&str, &str); 4] = [
    ("chore", "gpt-flash-implementer"),
    ("design", "gpt-flash-implementer-sdd"),
    ("engine", "gpt-flash-implementer-engine"),
    ("feature", "gpt-flash-implementer"),
];

/// The Sol offices: triage, specification, clarification, design chief,
/// task planning and analysis.
const SOL_OFFICES: [(&str, &str); 7] = [
    ("analyze:judge", "gpt-flash-analyst"),
    ("clarify:judge", "gpt-flash-clarifier"),
    ("design:chief", "gpt-flash-chief-architect"),
    (
        "design:positions:robustness",
        "gpt-flash-position-robustness",
    ),
    ("specify:author", "gpt-flash-chief-architect"),
    ("tasks:author", "gpt-flash-task-planner"),
    ("triage", "gpt-flash-triage"),
];

#[test]
fn every_strategy_implements_with_flash_41() {
    let bundle = gpt_flash();
    let cases = select_cases(&bundle, "implement");
    assert_eq!(
        cases.keys().map(String::as_str).collect::<Vec<_>>(),
        IMPLEMENT.map(|(strategy, _)| strategy),
        "implement must select every delivery strategy"
    );

    for (strategy, agent) in IMPLEMENT {
        let candidate = sole_candidate(&cases[strategy], strategy);
        assert_eq!(candidate.agent, agent, "{strategy} hires the wrong smith");
        assert_eq!(
            candidate.model, "flash-experiment",
            "{strategy} must hire the Flash lane, not another model"
        );
        assert_eq!(candidate.provider, "dsh", "{strategy} must run on dsh");
        assert!(
            candidate
                .argv
                .iter()
                .any(|token| token == "deepseek-v4.1-flash-expires-on-0910"),
            "{strategy} must compose the 4.1 beta id, not an older Flash: {:?}",
            candidate.argv
        );
        for older in ["deepseek-v4-flash", "deepseek-v4-flash-0731"] {
            assert!(
                !candidate.argv.iter().any(|token| token == older),
                "{strategy} reached the older Flash lane '{older}'"
            );
        }
    }
}

#[test]
fn every_strategy_reviews_with_a_mixed_panel_before_the_astra_chief() {
    let bundle = gpt_flash();
    let cases = select_cases(&bundle, "review");
    assert_eq!(
        cases.keys().map(String::as_str).collect::<Vec<_>>(),
        IMPLEMENT.map(|(strategy, _)| strategy),
        "review must answer every delivery strategy"
    );

    // The panel composition the framing promises, strategy by strategy.
    let panels: [(&str, &[&str]); 4] = [
        ("chore", &["correctness", "security"]),
        ("design", &["correctness", "security", "spec-compliance"]),
        (
            "engine",
            &["adversarial", "correctness", "security", "spec-compliance"],
        ),
        ("feature", &["correctness", "security"]),
    ];

    for (strategy, members) in panels {
        let SeatBody::Sequence { steps } = &cases[strategy] else {
            panic!("{strategy} review must be a sequence");
        };
        assert_eq!(
            steps
                .iter()
                .map(|step| step.name.as_str())
                .collect::<Vec<_>>(),
            ["positions", "chief"],
            "step order is load-bearing: the panel states, then the chief rules"
        );
        assert_eq!(steps[0].class, SeatClass::Work);
        assert_eq!(steps[1].class, SeatClass::Gate);

        let StepBody::Panel {
            members: compiled,
            aggregate,
        } = &steps[0].body
        else {
            panic!("{strategy} review must open with a panel");
        };
        assert_eq!(*aggregate, Aggregate::ReviewPanel);
        assert_eq!(
            compiled
                .iter()
                .map(|member| member.name.as_str())
                .collect::<Vec<_>>(),
            members,
            "{strategy} panel composition moved"
        );

        // Vendor diversity: the panel is GPT and Flash together, never one
        // vendor judging alone.
        let providers: std::collections::BTreeSet<&str> = compiled
            .iter()
            .map(|member| member.candidates[0].provider.as_str())
            .collect();
        assert_eq!(
            providers,
            ["codex", "dsh"].into_iter().collect(),
            "{strategy} panel lost its GPT/Flash diversity"
        );
        assert_eq!(
            compiled
                .iter()
                .find(|member| member.name == "correctness")
                .map(|member| member.candidates[0].model.as_str()),
            Some("sol"),
            "{strategy} correctness position must be GPT Sol"
        );
        assert_eq!(
            compiled
                .iter()
                .find(|member| member.name == "security")
                .map(|member| member.candidates[0].model.as_str()),
            Some("flash-experiment"),
            "{strategy} security position must be DeepSeek Flash"
        );
        for member in compiled {
            assert_eq!(
                member.candidates.len(),
                1,
                "{strategy} panel member {} carries a fallback",
                member.name
            );
        }

        let chief = sole_step_candidate(&steps[1].body, strategy);
        assert_eq!(chief.agent, "gpt-flash-review-chief");
        assert_eq!(chief.model, "astra", "{strategy} chief must be Astra");
        assert_eq!(chief.provider, "codex");
        assert!(
            chief.argv.iter().any(|token| token == "gpt-6-astra"),
            "{strategy} chief must compose Astra's concrete id: {:?}",
            chief.argv
        );
    }
}

#[test]
fn sol_rules_specification_and_planning() {
    let bundle = gpt_flash();
    for (site, agent) in SOL_OFFICES {
        assert_model(&bundle, site, agent, "sol", "codex");
    }
    // The design council keeps one Sol and one Flash position, so the
    // planning step is still a cross-vendor argument.
    assert_model(
        &bundle,
        "design:positions:simplicity",
        "gpt-flash-position-simplicity",
        "flash-experiment",
        "dsh",
    );
}

/// The recipe hires only its own `gpt-flash-*` offices: no standard
/// roster agent survives the override, no provider is Claude, and every
/// office is pinned to a single model (no silent fallback).
#[test]
fn the_scoped_roster_excludes_the_standard_offices_and_claude() {
    let bundle = gpt_flash();
    let agents = bundle.manifest["agents"]
        .as_object()
        .expect("the recipe witnesses its roster");
    assert!(!agents.is_empty());

    for (site, record) in agents {
        let agent = record["agent"].as_str().expect("an agent name");
        assert!(
            agent.starts_with("gpt-flash-"),
            "{site} hires the unscoped office {agent}"
        );
        let provider = record["provider"].as_str().expect("a provider");
        assert_ne!(provider, "claude", "{site} depends on Claude");
        assert!(
            matches!(provider, "codex" | "dsh"),
            "{site} landed on an unexpected provider {provider}"
        );
        assert_eq!(record["chosen_index"], 0);
        assert_eq!(
            record["chain"].as_array().map(Vec::len),
            Some(1),
            "{site} carries a fallback chain"
        );
        assert_eq!(record["skipped"].as_array().map(Vec::len), Some(0));
    }

    // The full recipe still seats every scoped office it defines; a
    // silently dropped override would leave a standard roster hire above.
    let hired: std::collections::BTreeSet<&str> = agents
        .values()
        .filter_map(|record| record["agent"].as_str())
        .collect();
    let expected: std::collections::BTreeSet<&str> = [
        "gpt-flash-analyst",
        "gpt-flash-chief-architect",
        "gpt-flash-clarifier",
        "gpt-flash-implementer",
        "gpt-flash-implementer-engine",
        "gpt-flash-implementer-sdd",
        "gpt-flash-position-robustness",
        "gpt-flash-position-simplicity",
        "gpt-flash-review-adversarial",
        "gpt-flash-review-chief",
        "gpt-flash-review-correctness",
        "gpt-flash-review-security",
        "gpt-flash-review-spec-compliance",
        "gpt-flash-task-planner",
        "gpt-flash-triage",
    ]
    .into_iter()
    .collect();
    assert_eq!(hired, expected, "the scoped roster moved");
}

/// `recipes/gpt-flash` extends `recipes/triage` and overrides only its
/// seats: the full policy table and the deterministic verify/ship/validate
/// gates are inherited byte-for-byte, never restated.
#[test]
fn the_policy_and_deterministic_gates_are_inherited_unchanged() {
    let bundle = gpt_flash();
    let triage = triage();

    assert_eq!(bundle.protected_phase, triage.protected_phase);
    assert_eq!(bundle.protected_phase, "review");
    assert_eq!(bundle.machine.initial, triage.machine.initial);
    assert_eq!(bundle.machine.phases, triage.machine.phases);
    assert_eq!(bundle.machine.terminal, triage.machine.terminal);
    let rules = |machine: &brokkr_core::policy::Machine| {
        machine
            .rules
            .iter()
            .map(|rule| {
                (
                    rule.id.clone(),
                    rule.from.clone(),
                    rule.result.clone(),
                    rule.next.clone(),
                    rule.reason.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        rules(&bundle.machine),
        rules(&triage.machine),
        "the inherited policy table moved"
    );

    let drivers = bundle.manifest["drivers"]
        .as_object()
        .expect("the inherited deterministic gates are witnessed");
    assert_eq!(
        drivers.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "analyze:check",
            "clarify:check",
            "design:validate",
            "ship",
            "specify:validate",
            "tasks:validate",
            "verify:checks",
            "verify:dialect-verify"
        ],
    );
    assert_eq!(bundle.manifest["drivers"], triage.manifest["drivers"]);
    for (site, pin) in drivers {
        assert!(
            pin.get("exec").is_some(),
            "gate {site} must be the deterministic exec driver"
        );
    }
}
