use super::*;
use crate::policy::tests::shipped_table;
use crate::policy::{BOOLEAN_INPUTS, STRATEGIES, VISIT_PREFIX};
use serde_json::json;

/// The engine-owned inputs `bundles/self` reads, as the runtime names
/// them; presence exempts them.
fn engine_owned(name: &str) -> bool {
    name.starts_with(VISIT_PREFIX)
        || [
            "consecutive_failures",
            "drift_detected",
            "dirty_worktrees",
            "fixes_docs_only",
        ]
        .contains(&name)
}

/// The audit of a table the loader may refuse: these tests plant the
/// findings its refusals are built on.
fn audit(table: &Value) -> Audit {
    Machine::parse(table)
        .unwrap()
        .audit_with(SWEEP_BUDGET, engine_owned)
        .unwrap()
}

fn self_table() -> Value {
    shipped_table("../../bundles/self/policy.json")
}

/// A v2 table over `work → review → done`, where review's
/// `security-hold` stops, holding `rules` after those two.
fn table(rules: Value) -> Value {
    let mut all = vec![
        json!({"id": "WORK", "from": "work", "result": "complete", "next": "review",
               "reason": "r"}),
        json!({"id": "HALT", "from": "review", "result": "security-hold", "next": "stop",
               "severity": "hard", "reason": "r"}),
    ];
    all.extend(rules.as_array().unwrap().iter().cloned());
    json!({
        "schema": "forge.phase-machine/v2",
        "phases": ["work", "review", "done", "stop"],
        "initial": "work",
        "terminal": ["done", "stop"],
        "rules": all,
    })
}

fn residual(id: &str, when: Value, next: &str) -> Value {
    json!({"id": id, "from": "review", "result": "residual", "when": when,
           "next": next, "reason": "r"})
}

fn unruled(valuation: &[(&str, Setting)]) -> Finding {
    Finding::Unruled {
        phase: "review".into(),
        result: "residual".into(),
        valuation: valuation
            .iter()
            .map(|(name, setting)| (name.to_string(), *setting))
            .collect(),
    }
}

/// The exchange #429 reported: the loader refuses it, naming both rules,
/// because the table it would load parks a high residual at the bound
/// where the ordered table stops.
#[test]
fn the_swapped_self_arms_are_refused_and_the_audit_names_both_rules() {
    let ordered = Machine::from_table(&self_table()).unwrap();
    let mut swapped = self_table();
    let rules = swapped["rules"].as_array_mut().unwrap();
    let ids: Vec<String> = rules.iter().map(|rule| rule["id"].to_string()).collect();
    let above = ids
        .iter()
        .position(|id| id == "\"REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM\"")
        .unwrap();
    assert_eq!(ids[above + 1], "\"REVIEW-REFORGE-EXHAUSTED-MEDIUM\"");
    rules.swap(above, above + 1);
    assert_eq!(
        Machine::from_table(&swapped).unwrap_err().to_string(),
        "malformed phase machine table: REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM is dead \
         behind REVIEW-REFORGE-EXHAUSTED-MEDIUM: its guard holds wherever \
         REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM's does, and first match wins; it fires on \
         no present valuation (decision 0050, ruling 2)"
    );
    let machine = Machine::parse(&swapped).unwrap();
    let bound = json!({"max_residual_severity": "high", "visits_implement": 3,
                       "has_security_residual": true});
    let bound = bound.as_object().unwrap();
    assert!(matches!(
        ordered.evaluate("review", "residual", bound),
        Outcome::Ruling { rule_id, next_phase, .. }
            if rule_id == "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM" && next_phase == "stop"
    ));
    assert!(matches!(
        machine.evaluate("review", "residual", bound),
        Outcome::Park { rule_id, .. } if rule_id == "REVIEW-REFORGE-EXHAUSTED-MEDIUM"
    ));
    assert_eq!(
        audit(&swapped).to_string(),
        "policy sweep (decision 0050, accepted; reported, not yet refused): 47 \
         valuations over 11 groups, 4 unruled\n  \
         REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM is dead behind \
         REVIEW-REFORGE-EXHAUSTED-MEDIUM: its guard holds wherever \
         REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM's does, and first match wins\n  \
         no rule rules (review, residual) at max_residual_severity=none, \
         visits_implement=0\n"
    );
}

/// The ordered table carries one finding shape only: the `residual`
/// verdict at severity `none` that decision 0050 measured, at every
/// visit count the sweep reads.
#[test]
fn the_ordered_self_table_leaves_only_the_severity_none_hole() {
    let none = Setting::Word("none");
    assert_eq!(
        audit(&self_table()),
        Audit {
            groups: 11,
            valuations: 47,
            findings: [0, 2, 3, 4]
                .into_iter()
                .map(|visits| unruled(&[
                    ("max_residual_severity", none),
                    ("visits_implement", Setting::Count(visits)),
                ]))
                .collect(),
        }
    );
}

#[test]
fn a_guard_is_shadowed_only_by_an_earlier_guard_it_implies() {
    let above = |word: &str| json!({"max_residual_severity_above": word});
    let at_most = |word: &str| json!({"max_residual_severity_at_most": word});
    let cases = [
        // Counters: a lower or equal floor, on the same counter.
        (
            json!({"visits_work_gte": 2}),
            json!({"visits_work_gte": 3}),
            true,
        ),
        (
            json!({"visits_work_gte": 2}),
            json!({"visits_work_gte": 2}),
            true,
        ),
        (
            json!({"visits_work_gte": 3}),
            json!({"visits_work_gte": 2}),
            false,
        ),
        (
            json!({"consecutive_failures_gte": 1}),
            json!({"visits_work_gte": 5}),
            false,
        ),
        // Severity: a lower floor, a higher ceiling, never across forms.
        (above("low"), above("medium"), true),
        (above("low"), above("low"), true),
        (above("medium"), above("low"), false),
        (at_most("medium"), at_most("low"), true),
        (at_most("low"), at_most("low"), true),
        (at_most("low"), at_most("medium"), false),
        (above("none"), at_most("none"), false),
        // Flags: an equal flag on the same input.
        (
            json!({"skip_verify": true}),
            json!({"skip_verify": true, "fixes_applied": false}),
            true,
        ),
        (
            json!({"skip_verify": true}),
            json!({"skip_verify": false}),
            false,
        ),
        (
            json!({"skip_verify": true}),
            json!({"fixes_applied": true}),
            false,
        ),
        // Enumerations: a superset.
        (
            json!({"strategy_in": ["chore", "feature"]}),
            json!({"strategy_in": ["chore"]}),
            true,
        ),
        (
            json!({"strategy_in": ["chore"]}),
            json!({"strategy_in": ["chore", "feature"]}),
            false,
        ),
        // Every condition of the earlier guard is implied, or none is.
        (
            json!({"skip_verify": true, "fixes_applied": true}),
            json!({"skip_verify": true}),
            false,
        ),
    ];
    for (earlier, later, shadowed) in cases {
        let findings = audit(&table(json!([
            residual("EARLIER", earlier.clone(), "done"),
            residual("LATER", later.clone(), "done"),
            residual("FALLBACK", json!({}), "stop"),
        ])))
        .findings;
        let expected = Finding::Shadowed {
            rule: "LATER".into(),
            behind: "EARLIER".into(),
        };
        assert_eq!(
            findings.contains(&expected),
            shadowed,
            "{earlier} then {later}"
        );
    }
}

#[test]
fn a_shadowed_rule_names_the_first_rule_that_subsumes_it() {
    let findings = audit(&table(json!([
        residual("FIRST", json!({"visits_work_gte": 1}), "done"),
        residual("SECOND", json!({"visits_work_gte": 2}), "done"),
        residual("THIRD", json!({"visits_work_gte": 3}), "done"),
        residual("FALLBACK", json!({}), "stop"),
    ])))
    .findings;
    let shadowed: Vec<String> = findings.iter().map(Finding::to_string).collect();
    assert_eq!(
        shadowed,
        [
            "SECOND is dead behind FIRST: its guard holds wherever SECOND's does, \
             and first match wins",
            "THIRD is dead behind FIRST: its guard holds wherever THIRD's does, \
             and first match wins",
        ]
    );
}

/// Deadness is read from the sweep, not from the guards' forms: a
/// vacuous guard of another form, or earlier arms that partition an axis,
/// leave a hard rule dead as surely as one stronger guard (#429's return).
#[test]
fn a_rule_is_dead_where_it_rules_no_swept_valuation() {
    let deny = || {
        json!({"id": "DENY", "from": "review", "result": "residual", "next": "stop",
               "severity": "hard", "reason": "r",
               "when": {"max_residual_severity_above": "medium"}})
    };
    // Presence also names an arm that skips the severity; its own test
    // pins that, so this one reads the rest.
    let findings = |arms: Value| -> Vec<Finding> {
        let mut findings = audit(&table(arms)).findings;
        findings.retain(|finding| !matches!(finding, Finding::Unread { .. }));
        findings
    };
    let permit = |when: Value| residual("PERMIT", when, "done");
    for vacuous in [
        json!({"max_residual_severity_at_most": "critical"}),
        json!({"visits_work_gte": 0}),
        json!({"strategy_in": STRATEGIES}),
    ] {
        let dead = findings(json!([permit(vacuous.clone()), deny()]));
        assert_eq!(
            dead,
            [Finding::Shadowed {
                rule: "DENY".into(),
                behind: "PERMIT".into(),
            }],
            "{vacuous}"
        );
        // The valid order: the hard floor first, and nothing is dead.
        assert_eq!(
            findings(json!([deny(), permit(vacuous.clone())])),
            [],
            "{vacuous}"
        );
    }
    // Two flag arms partition an axis ahead of the hard rule.
    let [yes, no] = [true, false].map(|flag| {
        residual(
            &format!("SHIP-{flag}"),
            json!({"skip_verify": flag}),
            "done",
        )
    });
    let covered = findings(json!([yes.clone(), no.clone(), deny()]));
    assert_eq!(
        covered,
        [Finding::Covered {
            rule: "DENY".into(),
            by: vec!["SHIP-true".into(), "SHIP-false".into()],
        }]
    );
    assert_eq!(
        covered[0].to_string(),
        "DENY is dead behind SHIP-true, SHIP-false: together their guards hold \
         wherever DENY's does, and first match wins"
    );
    assert_eq!(findings(json!([deny(), yes, no])), []);
    // A guard no valuation satisfies is dead in any order, and names no
    // rule it is behind.
    let never = |id: &str| {
        residual(
            id,
            json!({"max_residual_severity_above": "critical"}),
            "done",
        )
    };
    let fallback = residual("FALLBACK", json!({}), "stop");
    let unsatisfiable = findings(json!([never("NEVER"), never("AGAIN"), fallback]));
    assert_eq!(
        unsatisfiable,
        [
            Finding::Unsatisfiable {
                rule: "NEVER".into()
            },
            Finding::Unsatisfiable {
                rule: "AGAIN".into()
            },
        ]
    );
    assert_eq!(
        unsatisfiable[0].to_string(),
        "NEVER is dead: its guard holds on no valuation of its inputs"
    );
}

#[test]
fn liveness_names_the_unreachable_phase_and_the_dead_end() {
    let looped = |rules: Value| {
        let mut table = table(rules);
        table["phases"] = json!(["work", "review", "loop", "done", "stop"]);
        audit(&table).findings
    };
    let clean = |next: &str| {
        json!({"id": "CLEAN", "from": "review", "result": "clean", "next": next,
               "reason": "r"})
    };
    let spin = json!({"id": "SPIN", "from": "loop", "result": "again", "next": "loop",
                      "reason": "r"});
    let hold = json!({"id": "HOLD", "from": "review", "result": "residual", "park": true,
                      "reason": "r"});
    // `loop` is never entered: unreachable, and not also a dead end.
    let findings = looped(json!([clean("done"), spin]));
    assert_eq!(
        findings,
        [Finding::Unreachable {
            phase: "loop".into()
        }]
    );
    assert_eq!(
        findings[0].to_string(),
        "phase 'loop' is unreachable from the initial phase"
    );
    // Entered, it spins forever: a dead end, though review parks.
    let findings = looped(json!([clean("loop"), spin, hold]));
    assert_eq!(
        findings,
        [
            Finding::DeadEnd {
                phase: "loop".into()
            },
            Finding::Unreachable {
                phase: "done".into()
            },
        ]
    );
    assert_eq!(
        findings[0].to_string(),
        "phase 'loop' reaches no terminal phase and no parking rule"
    );
    // A parking rule is an ending: the same spin that may park is live.
    let stuck = json!({"id": "STUCK", "from": "loop", "result": "stuck", "park": true,
                       "reason": "r"});
    let into = json!({"id": "INTO", "from": "review", "result": "odd", "next": "loop",
                      "reason": "r"});
    assert_eq!(looped(json!([clean("done"), spin, hold, stuck, into])), []);
}

#[test]
fn presence_names_an_advancing_rule_that_skips_a_hard_input() {
    let presence = |arm: &Value, owned: fn(&str) -> bool| -> Vec<Finding> {
        let deny = json!({"id": "DENY", "from": "review", "result": "residual",
                          "when": {"has_security_residual": true}, "next": "stop",
                          "severity": "hard", "reason": "r"});
        let clean = json!({"id": "CLEAN", "from": "review", "result": "clean", "next": "done",
                           "reason": "r"});
        let machine = Machine::parse(&table(json!([deny, arm, clean]))).unwrap();
        let audit = machine.audit_with(SWEEP_BUDGET, owned).unwrap();
        audit
            .findings
            .into_iter()
            .filter(|finding| matches!(finding, Finding::Unread { .. }))
            .collect()
    };
    let seat_supplied: fn(&str) -> bool = |_| false;
    let engine_supplied: fn(&str) -> bool = |_| true;
    let unread = presence(&residual("SHIP", json!({}), "done"), seat_supplied);
    assert_eq!(
        unread,
        [Finding::Unread {
            rule: "SHIP".into(),
            inputs: vec!["has_security_residual".into()],
        }]
    );
    assert_eq!(
        unread[0].to_string(),
        "SHIP lets the run go on without reading has_security_residual, which a \
         hard rule of its group reads"
    );
    let hold = json!({"id": "HOLD", "from": "review", "result": "residual", "park": true,
                      "reason": "r"});
    for (arm, owned) in [
        // Reads the input the hard rule reads.
        (
            residual("SHIP", json!({"has_security_residual": false}), "done"),
            seat_supplied,
        ),
        // Returns: every road onward re-enters review.
        (residual("RETURN", json!({}), "work"), seat_supplied),
        (residual("AGAIN", json!({}), "review"), seat_supplied),
        // Stops, or parks.
        (residual("QUIT", json!({}), "stop"), seat_supplied),
        (hold, seat_supplied),
        // The engine always supplies it.
        (residual("SHIP", json!({}), "done"), engine_supplied),
    ] {
        assert_eq!(presence(&arm, owned), [], "{arm}");
    }
}

#[test]
fn the_sweep_walks_each_axis_at_and_around_its_thresholds() {
    let swept = |when: Value| audit(&table(json!([residual("R", when, "done")])));
    for (when, valuations) in [
        (json!({"skip_verify": true}), 2),
        (json!({"visits_work_gte": 3}), 4),
        (json!({"visits_work_gte": 2.5}), 4),
        (json!({"visits_work_gte": 0}), 2),
        (json!({"visits_work_gte": -1}), 2),
        (json!({"max_residual_severity_above": "low"}), 6),
        (json!({"strategy_in": ["chore"]}), 5),
        (json!({"drift_in": ["design"]}), 3),
        (
            json!({"skip_verify": true, "max_residual_severity_at_most": "low"}),
            12,
        ),
    ] {
        // `WORK` and `HALT` read nothing: one valuation each.
        assert_eq!(swept(when.clone()).valuations, 2 + valuations, "{when}");
    }
    let count = |visits| unruled(&[("visits_work", Setting::Count(visits))]);
    let holes = swept(json!({"visits_work_gte": 3})).findings;
    assert_eq!(holes, [count(0), count(2)]);
    assert_eq!(
        holes[1].to_string(),
        "no rule rules (review, residual) at visits_work=2"
    );
    let holes = swept(json!({"visits_work_gte": 2.5})).findings;
    assert_eq!(holes, [count(0), count(1), count(2)]);
    let holes = swept(json!({"skip_verify": true, "drift_in": ["design"]})).findings;
    let holes: Vec<String> = holes.iter().map(Finding::to_string).collect();
    assert_eq!(
        holes,
        [
            "no rule rules (review, residual) at drift_in=specify, skip_verify=true",
            "no rule rules (review, residual) at drift_in=tasks, skip_verify=true",
            "no rule rules (review, residual) at drift_in=specify, skip_verify=false",
            "no rule rules (review, residual) at drift_in=design, skip_verify=false",
            "no rule rules (review, residual) at drift_in=tasks, skip_verify=false",
        ]
    );
    let total = audit(&table(json!([
        residual("R", json!({"skip_verify": true}), "done"),
        residual("FALLBACK", json!({}), "stop"),
    ])));
    assert_eq!((total.valuations, total.findings), (4, vec![]));
}

#[test]
fn the_sweep_is_measured_before_it_runs_and_refuses_past_its_budget() {
    let machine = Machine::from_table(&self_table()).unwrap();
    assert_eq!(machine.audit_with(47, engine_owned).unwrap().valuations, 47);
    let over = machine.audit_with(46, engine_owned).unwrap_err();
    assert_eq!(
        over,
        AuditError::Budget {
            phase: "verify".into(),
            result: "pass".into(),
            valuations: 47,
            budget: 46,
        }
    );
    assert_eq!(
        over.to_string(),
        "the policy sweep reaches 47 valuations at (verify, pass), over its \
         budget of 46; the table was not swept"
    );
    // Every closed axis in one group: 2^8 × 6 × 5 × 3 × 4 = 92,160.
    let mut when = json!({"max_residual_severity_above": "low", "strategy_in": ["chore"],
                          "drift_in": ["design"], "consecutive_failures_gte": 2});
    for flag in BOOLEAN_INPUTS {
        when[flag] = json!(true);
    }
    let wide = table(json!([residual("WIDE", when, "done")]));
    assert_eq!(
        Machine::from_table(&wide)
            .unwrap()
            .audit_with(SWEEP_BUDGET, engine_owned)
            .unwrap_err(),
        AuditError::Budget {
            phase: "review".into(),
            result: "residual".into(),
            valuations: 92_160,
            budget: SWEEP_BUDGET,
        }
    );
    // The loader sweeps nothing past the budget, and still refuses what
    // needs no sweep (#429).
    let mut unentered = wide;
    unentered["phases"] = json!(["work", "review", "loop", "done", "stop"]);
    assert_eq!(
        Machine::from_table(&unentered).unwrap_err().to_string(),
        "malformed phase machine table: phase 'loop' is unreachable from the initial \
         phase (decision 0050, ruling 3)"
    );
}
