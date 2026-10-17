//! The recipe's graph (#568, the Library), figure by figure, on one
//! builder-made table and one builder-made fleet: six runs of `self` in
//! `alpha` that the week keeps, and runs it does not — another recipe,
//! another realm, a run that ended before the range.

use super::figures::p90;
use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};
use crate::{AnalyticsRun, ABSENT};
use brokkr_core::fold::{RunState, Status};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::json;

const NOW: &str = "2026-10-13T12:00:00Z";

/// The table: a forward row from intake to ship, a self-loop at design, a
/// back edge from review, and rules into both terminals.
fn machine() -> Machine {
    Machine::from_table(&table()).unwrap()
}

/// The table [`machine`] is read from.
fn table() -> serde_json::Value {
    let rule = |id: &str, from: &str, result: &str, next: &str| json!({"id": id, "from": from, "result": result, "next": next, "reason": id});
    let mut again = rule("REVIEW-AGAIN", "review", "residual", "implement");
    again["when"] = json!({"fixes_applied": false});
    json!({
        "phases": ["intake", "design", "implement", "verify", "review", "ship", "done", "stop"],
        "initial": "intake",
        "terminal": ["done", "stop"],
        "rules": [
            rule("INTAKE-OK", "intake", "complete", "design"),
            rule("DESIGN-RETRY", "design", "retry", "design"),
            rule("DESIGN-OK", "design", "complete", "implement"),
            rule("IMPL-OK", "implement", "complete", "verify"),
            rule("VERIFY-PASS", "verify", "pass", "review"),
            rule("VERIFY-FAIL", "verify", "fail", "stop"),
            rule("REVIEW-CLEAN", "review", "clean", "ship"),
            again,
            rule("SHIP-COMPLETE", "ship", "complete", "done"),
        ],
    })
}

/// The compiled manifest's pins: the table's file, a composed base layer,
/// and a role file that is not the table.
fn resolved() -> serde_json::Value {
    json!({"files": {"policy.json": "now", "@compose/0000/base": "b1", "roles/x.md": "z"}})
}

/// One run's journal, event by event, under its own id.
struct Journal {
    run_id: &'static str,
    events: Vec<EventEnvelope>,
}

impl Journal {
    /// A run of `recipe` in `realm` started at `at`, pinning `pins`.
    fn started(run_id: &'static str, recipe: &str, realm: &str, at: &str) -> Journal {
        let manifest = json!({"bundle_name": recipe,
            "realms": {"source": "/w/realms.json", "realm": realm,
                       "map": {"realms": [{"name": realm, "path": realm}]}}});
        Journal {
            run_id,
            events: Vec::new(),
        }
        .then(
            EventType::RunStarted,
            json!({"feature": "f", "manifest": manifest}),
            at,
        )
    }

    fn pinned(mut self, pins: serde_json::Value) -> Journal {
        let manifest = &mut self.events[0].payload["manifest"];
        for (key, value) in pins.as_object().unwrap() {
            manifest[key] = value.clone();
        }
        self
    }

    fn then(mut self, event_type: EventType, payload: serde_json::Value, at: &str) -> Journal {
        let mut event = ev(self.events.len() as u64 + 1, event_type, payload, at);
        event.run_id = self.run_id.to_string();
        self.events.push(event);
        self
    }

    fn entered(self, phase: &str, at: &str) -> Journal {
        self.then(EventType::PhaseEntered, json!({"phase": phase}), at)
    }

    fn ruled(self, rule: &str, from: &str, next: &str, at: &str) -> Journal {
        let decision = ruled(rule, from, Some(next)).unwrap();
        self.then(EventType::TransitionDecided, decision, at)
    }

    /// One attempt on `provider` with one checkpoint: `record`, its cost
    /// and tokens.
    fn seat(self, attempt: &str, provider: &str, record: serde_json::Value, at: &str) -> Journal {
        self.then(
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": attempt,
                   "provenance": [{"agent": "a", "provider": provider}]}),
            at,
        )
        .then(
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": attempt, "checkpoint": record}),
            at,
        )
    }

    fn ended(self, event_type: EventType, at: &str) -> Journal {
        self.then(event_type, json!({}), at)
    }
}

/// A day's moment: `2026-10-<day>T<time>Z`.
fn on(day: u8, time: &str) -> String {
    format!("2026-10-{day:02}T{time}Z")
}

/// Intake for `minutes` from midnight of `day`, then design, implement
/// and verify ten minutes each, ruled forward.
fn to_verify(journal: Journal, day: u8, minutes: u32) -> Journal {
    let at = |offset: u32| on(day, &format!("{:02}:{:02}:00", offset / 60, offset % 60));
    journal
        .entered("intake", &at(0))
        .ruled("INTAKE-OK", "intake", "design", &at(minutes))
        .entered("design", &at(minutes))
        .ruled("DESIGN-OK", "design", "implement", &at(minutes + 10))
        .entered("implement", &at(minutes + 10))
        .ruled("IMPL-OK", "implement", "verify", &at(minutes + 20))
        .entered("verify", &at(minutes + 20))
}

fn stopped_by(rule: &str) -> RunState {
    state(
        Some("stop"),
        Status::Stopped,
        ruled(rule, "verify", Some("stop")),
    )
}

fn verify_failed(run_id: &'static str, day: u8, minutes: u32) -> Journal {
    let journal = Journal::started(run_id, "self", "alpha", &on(day, "00:00:00"));
    to_verify(journal, day, minutes)
        .ruled("VERIFY-FAIL", "verify", "stop", &on(day, "02:00:00"))
        .ended(EventType::RunStopped, &on(day, "02:00:00"))
}

/// The fleet and the state each run folds to.
fn fleet() -> Vec<(Journal, Option<RunState>)> {
    let old = json!({"files": {"policy.json": "old"}});
    // Shipped; a dsh seat at design reports no cost, and at review a
    // claude seat reports one beside a codex seat that does not.
    let shipped = to_verify(
        Journal::started("r1", "self", "alpha", &on(8, "00:00:00")),
        8,
        10,
    )
    .pinned(old.clone())
    .ruled("VERIFY-PASS", "verify", "review", &on(8, "00:40:00"))
    .entered("review", &on(8, "00:40:00"))
    .seat(
        "a1",
        "claude",
        json!({"step": "claude-session-finished", "total_cost_usd": 1.5,
            "input_tokens": 12000, "output_tokens": 3000, "cache_read_tokens": 8000}),
        &on(8, "00:50:00"),
    )
    .seat(
        "a2",
        "codex",
        json!({"step": "turn-completed", "input_tokens": 6000,
            "output_tokens": 1000}),
        &on(8, "00:55:00"),
    )
    .ruled("REVIEW-CLEAN", "review", "ship", &on(8, "01:00:00"))
    .entered("ship", &on(8, "01:00:00"))
    .ruled("SHIP-COMPLETE", "ship", "done", &on(8, "01:10:00"))
    .entered("done", &on(8, "01:10:00"))
    .ended(EventType::RunCompleted, &on(8, "01:10:00"));
    let shipped = {
        let at = on(8, "00:15:00");
        let mut journal = shipped;
        let seat = Journal {
            run_id: "r1",
            events: Vec::new(),
        }
        .seat("a0", "dsh", json!({"step": "x"}), &at);
        // At design, between its entry and the next.
        journal.events.splice(5..5, seat.events);
        journal
    };
    let failed = |run_id, day, minutes, pins: &serde_json::Value| {
        verify_failed(run_id, day, minutes).pinned(pins.clone())
    };
    // A rule the table no longer holds, fired twice, under the same policy
    // file and a base layer that has since moved.
    let retired = verify_failed("r4", 11, 40)
        .pinned(json!({"files": {"policy.json": "now", "@compose/0000/base": "b0"}}))
        .ruled("OLD-RULE", "design", "design", &on(11, "00:45:00"))
        .ruled("OLD-RULE", "design", "design", &on(11, "00:46:00"));
    // Open in review after review sent it back once; the table it pinned
    // is the resolved one, a role file apart.
    let live = to_verify(
        Journal::started("r5", "self", "alpha", &on(12, "00:00:00")),
        12,
        50,
    )
    .pinned(
        json!({"files": {"policy.json": "now", "@compose/0000/base": "b1", "roles/x.md": "y"},
            "agents": {"review:tests": {"provider": "codex", "model": "gpt-5"},
                       "review:security": {"provider": "claude", "model": "opus"},
                       "design": {"agent": "d"}}}),
    )
    .ruled("VERIFY-PASS", "verify", "review", &on(12, "01:20:00"))
    .entered("review", &on(12, "01:20:00"))
    .ruled("REVIEW-AGAIN", "review", "implement", &on(12, "01:30:00"))
    .entered("implement", &on(12, "01:30:00"))
    .entered("verify", &on(12, "01:40:00"))
    .entered("review", &on(12, "01:50:00"));
    let mut open = state(Some("review"), Status::Running, None);
    open.feature = Some("Add the graph\nand its figures".to_string());
    // Stopped by the operator at verify, pinning nothing.
    let operator = to_verify(
        Journal::started("r6", "self", "alpha", &on(10, "00:00:00")),
        10,
        60,
    )
    .ended(EventType::RunStopped, &on(10, "03:00:00"));
    vec![
        (shipped, Some(state(Some("done"), Status::Completed, None))),
        (failed("r2", 9, 20, &old), Some(stopped_by("VERIFY-FAIL"))),
        (failed("r3", 9, 30, &old), Some(stopped_by("VERIFY-FAIL"))),
        (retired, Some(stopped_by("VERIFY-FAIL"))),
        (live, Some(open)),
        (operator, Some(state(Some("verify"), Status::Stopped, None))),
        // Kept by nothing: another recipe, another realm, an end before
        // the range, and a journal that does not fold.
        (
            verify_failed("r7", 9, 10).pinned(json!({"bundle_name": "frontier"})),
            Some(stopped_by("VERIFY-FAIL")),
        ),
        (
            Journal::started("r8", "self", "beta", &on(9, "00:00:00")),
            Some(state(None, Status::Running, None)),
        ),
        (verify_failed("r9", 1, 10), Some(stopped_by("VERIFY-FAIL"))),
        (
            Journal::started("r10", "self", "alpha", &on(9, "00:00:00")),
            None,
        ),
    ]
}

fn runs(journals: &[(Journal, Option<RunState>)]) -> Vec<AnalyticsRun<'_>> {
    let runs = journals.iter().map(|(journal, state)| AnalyticsRun {
        events: &journal.events,
        state: state.as_ref(),
    });
    runs.collect()
}

/// The graph of `recipe` over the fleet, with the table when `resolved`.
fn read(recipe: &str, table: bool) -> Result<GraphView, GraphError> {
    read_over(fleet(), recipe, table)
}

/// The graph of `recipe` over `journals`, with the table when `table`.
fn read_over(
    journals: Vec<(Journal, Option<RunState>)>,
    recipe: &str,
    table: bool,
) -> Result<GraphView, GraphError> {
    read_under(machine(), journals, recipe, table)
}

/// The graph of `recipe` over `journals`, under `machine` when `table`.
fn read_under(
    machine: Machine,
    journals: Vec<(Journal, Option<RunState>)>,
    recipe: &str,
    table: bool,
) -> Result<GraphView, GraphError> {
    let manifest = resolved();
    let runs = runs(&journals);
    let realms = vec!["alpha".to_string(), "beta".to_string()];
    let hearths = [
        AnalyticsHearth {
            label: "alpha",
            realms: &realms,
            journal: "a.db",
            runs: &runs,
            refusal: None,
        },
        AnalyticsHearth {
            label: "gamma",
            realms: &[],
            journal: "g.db",
            runs: &[],
            refusal: Some("no such journal"),
        },
    ];
    graph(&GraphQuery {
        hearths: &hearths,
        recipe,
        realm: "alpha",
        table: table.then_some(GraphTable {
            machine: &machine,
            manifest: &manifest,
        }),
        shelved: &["shelved".to_string()],
        range: Range::Week,
        now: NOW,
    })
}

fn rule<'a>(view: &'a GraphView, id: &str) -> &'a Edge {
    view.rules.iter().find(|rule| rule.id == id).unwrap()
}

fn named<'a>(view: &'a GraphView, name: &str) -> &'a State {
    view.states.iter().find(|state| state.name == name).unwrap()
}

#[test]
fn a_rule_that_fired_counts_its_rulings_in_the_range() {
    let view = read("self", true).unwrap();
    assert_eq!(
        rule(&view, "VERIFY-FAIL"),
        &Edge {
            id: "VERIFY-FAIL".into(),
            from: "verify".into(),
            next: Some("stop".into()),
            result: "fail".into(),
            when: vec![],
            severity: Some(RuleSeverity::Normal),
            count: 3,
            kind: RuleKind::Terminal,
            text: "VERIFY-FAIL · 3".into(),
        }
    );
    assert_eq!(rule(&view, "REVIEW-AGAIN").when, ["not fixes_applied"]);
    assert_eq!(view.runs, 6);
}

#[test]
fn a_rule_that_never_fired_is_drawn_with_none() {
    let view = read("self", true).unwrap();
    let dormant = rule(&view, "DESIGN-RETRY");
    assert_eq!(
        (dormant.count, dormant.text.as_str()),
        (0, "DESIGN-RETRY · 0")
    );
    assert_eq!(
        view.never_fired,
        NeverFired {
            count: 1,
            of: 9,
            text: "1 of 9".into()
        }
    );
}

#[test]
fn each_rule_has_its_kind_and_each_state_its_place() {
    let view = read("self", true).unwrap();
    let kinds: Vec<(&str, RuleKind)> = view
        .rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("INTAKE-OK", RuleKind::Forward),
            ("DESIGN-RETRY", RuleKind::SelfLoop),
            ("DESIGN-OK", RuleKind::Forward),
            ("IMPL-OK", RuleKind::Forward),
            ("VERIFY-PASS", RuleKind::Forward),
            ("VERIFY-FAIL", RuleKind::Terminal),
            ("REVIEW-CLEAN", RuleKind::Forward),
            ("REVIEW-AGAIN", RuleKind::Back),
            ("SHIP-COMPLETE", RuleKind::Terminal),
        ]
    );
    let places: Vec<(&str, Place)> = view
        .states
        .iter()
        .map(|state| (state.name.as_str(), state.place))
        .collect();
    assert_eq!(
        places,
        [
            ("intake", Place::Row),
            ("design", Place::Row),
            ("implement", Place::Row),
            ("verify", Place::Row),
            ("review", Place::Row),
            ("ship", Place::Row),
            ("done", Place::End),
            ("stop", Place::Beneath),
        ]
    );
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(wire["view_version"], json!(1));
    assert_eq!(wire["table"], json!({"kind": "resolved"}));
    let words: Vec<&str> = (1..=7)
        .map(|at| {
            wire["rules"][at]
                .get("kind")
                .and_then(|kind| kind.as_str())
                .unwrap()
        })
        .collect();
    assert_eq!(
        words,
        ["self", "forward", "forward", "forward", "terminal", "forward", "back"]
    );
    assert_eq!(
        (
            wire["states"][6]["place"].clone(),
            wire["states"][7]["place"].clone()
        ),
        (json!("end"), json!("beneath"))
    );
}

#[test]
fn the_initial_state_leads_the_row_wherever_the_table_lists_it() {
    // A composed recipe can list the state it starts in last: `landing`
    // extends `fast` and adds `classify`, which lands after `stop`.
    let mut table = table();
    table["phases"] = json!([
        "design",
        "implement",
        "verify",
        "review",
        "ship",
        "done",
        "stop",
        "intake"
    ]);
    let view = read_under(Machine::from_table(&table).unwrap(), fleet(), "self", true).unwrap();
    let order: Vec<&str> = view
        .states
        .iter()
        .map(|state| state.name.as_str())
        .collect();
    assert_eq!(
        order,
        [
            "intake",
            "design",
            "implement",
            "verify",
            "review",
            "ship",
            "done",
            "stop"
        ]
    );
    assert_eq!(rule(&view, "INTAKE-OK").kind, RuleKind::Forward);
    assert_eq!(named(&view, "intake").place, Place::Row);
}

#[test]
fn a_rule_the_table_no_longer_holds_is_retired_with_its_count() {
    let view = read("self", true).unwrap();
    assert_eq!(
        view.retired,
        [RuleCount {
            rule_id: Some("OLD-RULE".into()),
            count: 2
        }]
    );
}

#[test]
fn runs_pinned_under_another_table_are_counted_as_drifted() {
    let view = read("self", true).unwrap();
    // r1-r3 pinned another policy file, r4 the same one under a base layer
    // that moved; r5's other role file is not the table, and r6 pinned
    // nothing to compare.
    assert_eq!(view.drifted_runs, 4);
    assert_eq!(
        view.drift_text.as_deref(),
        Some(
            "3 runs ran under an earlier version of this table · \
             1 run ran under an earlier version of a base this recipe composes, \
             which may not have changed this table"
        )
    );
}

#[test]
fn a_table_that_does_not_resolve_draws_only_the_fired_rules() {
    let view = read("self", false).unwrap();
    assert_eq!(
        view.table,
        Table::NotFound {
            text: "this recipe's table was not found in alpha".into()
        }
    );
    let fired: Vec<(&str, usize, RuleKind)> = view
        .rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule.count, rule.kind))
        .collect();
    assert_eq!(
        fired,
        [
            ("INTAKE-OK", 6),
            ("DESIGN-OK", 6),
            ("IMPL-OK", 6),
            ("VERIFY-PASS", 2),
            ("REVIEW-CLEAN", 1),
            ("SHIP-COMPLETE", 1),
            ("VERIFY-FAIL", 3),
            ("OLD-RULE", 2),
            ("REVIEW-AGAIN", 1),
        ]
        .map(|(id, count)| (id, count, rule(&view, id).kind))
    );
    assert_eq!(
        [
            rule(&view, "SHIP-COMPLETE").kind,
            rule(&view, "VERIFY-FAIL").kind,
            rule(&view, "OLD-RULE").kind,
            rule(&view, "REVIEW-AGAIN").kind
        ],
        [
            RuleKind::Terminal,
            RuleKind::Terminal,
            RuleKind::SelfLoop,
            RuleKind::Back
        ]
    );
    assert_eq!(rule(&view, "VERIFY-FAIL").result, "ruled");
    assert_eq!((view.retired.len(), view.drifted_runs), (0, 0));
    assert_eq!(named(&view, "done").place, Place::End);
    assert_eq!(named(&view, "stop").place, Place::Beneath);
}

#[test]
fn an_unknown_recipe_is_refused_naming_the_recipes_the_realm_knows() {
    let refused = read("nosuch", false).unwrap_err();
    assert_eq!(
        refused,
        GraphError::UnknownRecipe {
            recipe: "nosuch".into(),
            realm: "alpha".into(),
            known: vec!["frontier".into(), "self".into(), "shelved".into()],
        }
    );
    assert_eq!(
        refused.to_string(),
        "no recipe 'nosuch' in alpha: the recipes it knows are frontier, self and shelved"
    );
    let empty = GraphError::UnknownRecipe {
        recipe: "x".into(),
        realm: "beta".into(),
        known: vec![],
    };
    assert_eq!(
        empty.to_string(),
        "no recipe 'x' in beta: it knows no recipe"
    );
    // A recipe the realm resolves needs no run to be drawn.
    let shelved = read("shelved", true).unwrap();
    assert_eq!((shelved.runs, shelved.recipes.len()), (0, 3));
}

#[test]
fn a_state_counts_the_distinct_runs_that_entered_it_and_their_stays() {
    let view = read("self", true).unwrap();
    let intake = named(&view, "intake");
    // Six runs stayed 10, 20, 30, 40, 50 and 60 minutes.
    assert_eq!(intake.entered, 6);
    assert_eq!(
        (intake.median.millis, intake.median.text.as_str()),
        (Some(35 * 60_000), "35m")
    );
    assert_eq!(
        (intake.p90.millis, intake.p90.text.as_str()),
        (Some(60 * 60_000), "1h 00m")
    );
    // r5 entered implement twice and is one run that entered it.
    assert_eq!(named(&view, "implement").entered, 6);
    // A state no run entered states its absences.
    let shipless = named(&view, "stop");
    assert_eq!(
        (shipless.entered, shipless.median.text.as_str()),
        (0, ABSENT)
    );
    assert_eq!(p90(&[]), None);
    assert_eq!(p90(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]), Some(10));
}

#[test]
fn a_states_cost_names_the_providers_that_reported_none() {
    let view = read("self", true).unwrap();
    let design = &named(&view, "design").cost;
    assert_eq!(
        (
            &design.cost,
            design.silent.as_slice(),
            design.reporting.len()
        ),
        (
            &crate::analytics::ReportedCost::NotReported,
            ["dsh".to_string()].as_slice(),
            0
        )
    );
    let review = named(&view, "review");
    assert_eq!(review.cost.text, "$1.50");
    assert_eq!(
        review.cost.note.as_deref(),
        Some("claude only · codex report none")
    );
    // Read apart as the run view's seats are: the cache hit is cache
    // reads over input, worked out here and nowhere else.
    assert_eq!(cells(&review.tokens), ["22k tok", "18k", "4k", "44.4%"]);
    assert_eq!(cells(&named(&view, "intake").tokens), [ABSENT; 4]);
}

#[test]
fn a_seat_started_before_the_range_counts_where_and_when_it_metered() {
    // The attempt starts at implement before the week begins; its session
    // finishes in review inside it.
    let spanning = Journal::started("r11", "self", "alpha", &on(6, "10:00:00"))
        .entered("implement", &on(6, "11:00:00"))
        .then(
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a9",
                   "provenance": [{"agent": "a", "provider": "claude"}]}),
            &on(6, "11:30:00"),
        )
        .entered("review", &on(6, "11:50:00"))
        .then(
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": "a9",
                   "checkpoint": {"step": "claude-session-finished", "total_cost_usd": 1.0,
                                  "input_tokens": 5000, "output_tokens": 1000}}),
            &on(6, "12:30:00"),
        );
    let open = state(Some("review"), Status::Running, None);
    let view = read_over(vec![(spanning, Some(open))], "self", true).unwrap();
    let review = &named(&view, "review");
    assert_eq!(
        (
            review.cost.text.as_str(),
            review.cost.reporting.as_slice(),
            review.cost.note.as_deref()
        ),
        ("$1.00", ["claude".to_string()].as_slice(), None)
    );
    assert_eq!(cells(&review.tokens), ["6k tok", "5k", "1k", ABSENT]);
    assert_eq!(cells(&named(&view, "implement").tokens), [ABSENT; 4]);
}

#[test]
fn a_states_seats_are_the_latest_runs_pins_by_human_name() {
    let view = read("self", true).unwrap();
    assert_eq!(
        named(&view, "review").seats,
        [
            SeatPin {
                key: "review:security".into(),
                provider: Some("claude".into()),
                model: Some("opus".into()),
                text: "review:security · claude opus".into(),
            },
            SeatPin {
                key: "review:tests".into(),
                provider: Some("codex".into()),
                model: Some("gpt-5".into()),
                text: "review:tests · codex gpt-5".into(),
            },
        ]
    );
    assert_eq!(
        named(&view, "design").seats[0].text,
        "design · not recorded"
    );
}

#[test]
fn ends_count_done_stop_by_rule_the_operator_and_the_open() {
    let view = read("self", true).unwrap();
    assert_eq!(
        view.ends,
        Ends {
            done: 1,
            stop: vec![RuleCount {
                rule_id: Some("VERIFY-FAIL".into()),
                count: 3
            }],
            stopped: 3,
            operator: 1,
            open: 1,
            text: "1 done · 3 stop by a rule · 1 stopped by you · 1 open".into(),
        }
    );
}

#[test]
fn stops_by_rule_are_listed_the_most_frequent_first() {
    let journals = vec![
        (verify_failed("s1", 9, 10), Some(stopped_by("A-ONCE"))),
        (verify_failed("s2", 9, 20), Some(stopped_by("B-TWICE"))),
        (verify_failed("s3", 9, 30), Some(stopped_by("B-TWICE"))),
    ];
    let view = read_over(journals, "self", true).unwrap();
    let stop: Vec<(Option<&str>, usize)> = view
        .ends
        .stop
        .iter()
        .map(|rule| (rule.rule_id.as_deref(), rule.count))
        .collect();
    assert_eq!(stop, [(Some("B-TWICE"), 2), (Some("A-ONCE"), 1)]);
}

#[test]
fn a_live_run_is_named_by_its_fleet_title_and_its_state() {
    let view = read("self", true).unwrap();
    assert_eq!(
        view.live,
        [LiveRun {
            run_id: "r5".into(),
            label: "Add the graph".into(),
            state: Some("review".into()),
        }]
    );
    assert_eq!(
        (view.from.as_str(), view.to.as_str()),
        ("2026-10-06T12:00:00Z", NOW)
    );
}

#[test]
fn a_clock_that_is_no_time_is_refused() {
    let refused = graph(&GraphQuery {
        hearths: &[],
        recipe: "self",
        realm: "alpha",
        table: None,
        shelved: &[],
        range: Range::Day,
        now: "noon",
    })
    .unwrap_err();
    assert_eq!(refused, GraphError::Clock("noon".into()));
    assert_eq!(
        refused.to_string(),
        "the clock read 'noon', which is not an RFC 3339 time"
    );
}

/// A token figure as its four cells read: total, input, output, cache hit.
fn cells(tokens: &crate::TokenCells) -> [&str; 4] {
    [
        &tokens.total.text,
        &tokens.input.text,
        &tokens.output.text,
        &tokens.cache_hit.text,
    ]
}
