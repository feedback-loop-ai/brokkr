//! The analytics readout (#568), metric by metric, on one builder-made
//! fleet of three hearths: `alpha` and `beta` load, `gamma` does not.
//! Each run's journal is built for its times, parks, rulings and seats;
//! its folded state, from the shared state builder, for how it ended.

use super::median::medians;
use super::words::{change_text, listed, span_text, stamp};
use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};
use crate::ABSENT;
use brokkr_core::ending::Ending;
use brokkr_core::fold::{fold, Status};
use brokkr_core::EventType;
use serde_json::{json, Value};

const NOW: &str = "2026-10-13T12:00:00Z";

/// One run's journal, event by event.
struct Journal {
    events: Vec<EventEnvelope>,
}

/// A journal built event by event: each fixture appends with its own
/// `then`, and enters a phase the one way. The graph's fixture shares it.
pub(crate) trait Builds: Sized {
    fn then(self, event_type: EventType, payload: Value, at: &str) -> Self;

    fn entered(self, phase: &str, at: &str) -> Self {
        self.then(EventType::PhaseEntered, json!({"phase": phase}), at)
    }
}

impl Builds for Journal {
    fn then(mut self, event_type: EventType, payload: Value, at: &str) -> Journal {
        let seq = self.events.len() as u64 + 1;
        self.events.push(ev(seq, event_type, payload, at));
        self
    }
}

impl Journal {
    /// A run started at `at`, pinned to `realm` when one is named.
    fn started(realm: Option<&str>, at: &str) -> Journal {
        let manifest = match realm {
            Some(realm) => json!({"realms": {"source": "/w/realms.json", "realm": realm,
                                  "map": {"realms": [{"name": realm, "path": realm}]}}}),
            None => json!({}),
        };
        Journal { events: Vec::new() }.then(
            EventType::RunStarted,
            json!({"feature": "f", "manifest": manifest}),
            at,
        )
    }

    fn ruled(self, from: &str, next: Option<&str>, at: &str) -> Journal {
        let decision = ruled("R", from, next).unwrap();
        self.then(EventType::TransitionDecided, decision, at)
    }

    /// One attempt of a seat its agent resolution sent to `provider`,
    /// reporting `model` and, when it has one, `cost`.
    fn seat(
        self,
        attempt: &str,
        provider: &str,
        model: &str,
        cost: Option<f64>,
        at: &str,
    ) -> Journal {
        let mut checkpoint = json!({"step": "claude-session-finished", "model": model});
        if let Some(cost) = cost {
            checkpoint["total_cost_usd"] = json!(cost);
        }
        self.then(
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": attempt,
                   "provenance": [{"agent": "a", "model": model, "provider": provider}]}),
            at,
        )
        .then(
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": attempt, "checkpoint": checkpoint}),
            at,
        )
    }
}

fn ended(status: Status, phase: &str, rule: Option<&str>, implement: u64) -> RunState {
    let mut ended = state(
        Some(phase),
        status,
        rule.and_then(|rule| ruled(rule, "review", Some(phase))),
    );
    ended.visits.insert("implement".to_string(), implement);
    ended
}

/// The fleet: every run, and the state each folds to.
struct Fleet {
    alpha: Vec<(Journal, Option<RunState>)>,
    beta: Vec<(Journal, Option<RunState>)>,
}

fn fleet() -> Fleet {
    // Shipped in 4h, sent back by its first review, on at its second.
    let a = Journal::started(Some("alpha"), "2026-10-10T00:00:00Z")
        .entered("implement", "2026-10-10T00:00:00Z")
        .seat(
            "a1",
            "claude",
            "claude-opus",
            Some(1.5),
            "2026-10-10T00:10:00Z",
        )
        .entered("review", "2026-10-10T01:00:00Z")
        .ruled("review", Some("implement"), "2026-10-10T01:30:00Z")
        .entered("implement", "2026-10-10T02:00:00Z")
        .entered("review", "2026-10-10T03:00:00Z")
        .ruled("review", Some("done"), "2026-10-10T03:30:00Z")
        .then(EventType::RunCompleted, json!({}), "2026-10-10T04:00:00Z");
    // Parked at review 2h before the operator answered, 4h before it
    // moved; then stopped by a rule, 6h after it started.
    let b = Journal::started(Some("alpha"), "2026-10-11T00:00:00Z")
        .entered("review", "2026-10-11T00:30:00Z")
        .ruled("review", None, "2026-10-11T00:59:00Z")
        .then(EventType::RunParked, json!({}), "2026-10-11T01:00:00Z")
        .then(
            EventType::OperatorCommanded,
            json!({"command": "retry"}),
            "2026-10-11T03:00:00Z",
        )
        .then(
            EventType::OperatorAccepted,
            json!({}),
            "2026-10-11T03:00:01Z",
        )
        .entered("review", "2026-10-11T05:00:00Z")
        .then(EventType::RunStopped, json!({}), "2026-10-11T06:00:00Z");
    // Its journal does not fold: counted, read by no figure.
    let f = Journal::started(Some("alpha"), "2026-10-12T00:00:00Z");
    // Shipped in 2h by a seat that reports no cost.
    let c = Journal::started(Some("beta"), "2026-10-12T00:00:00Z")
        .seat("c1", "codex", "gpt-5", None, "2026-10-12T00:30:00Z")
        .then(EventType::RunCompleted, json!({}), "2026-10-12T02:00:00Z");
    // Shipped in the range before this one.
    let d = Journal::started(Some("beta"), "2026-09-30T23:00:00Z").then(
        EventType::RunCompleted,
        json!({}),
        "2026-10-01T00:00:00Z",
    );
    // Parked at clarify 2h ago, and no command since.
    let e = Journal::started(None, "2026-10-13T09:00:00Z")
        .entered("clarify", "2026-10-13T09:30:00Z")
        .then(EventType::RunParked, json!({}), "2026-10-13T10:00:00Z");
    let parked = state(Some("clarify"), Status::AwaitingOperator, None);
    Fleet {
        alpha: vec![
            (a, Some(ended(Status::Completed, "done", None, 2))),
            (
                b,
                Some(ended(Status::Stopped, "stop", Some("REVIEW-FAIL-TWICE"), 1)),
            ),
            (f, None),
        ],
        beta: vec![
            (c, Some(ended(Status::Completed, "done", None, 1))),
            (d, Some(ended(Status::Completed, "done", None, 0))),
            (e, Some(parked)),
        ],
    }
}

fn runs(journals: &[(Journal, Option<RunState>)]) -> Vec<AnalyticsRun<'_>> {
    journals
        .iter()
        .map(|(journal, state)| AnalyticsRun {
            events: &journal.events,
            state: state.as_ref(),
        })
        .collect()
}

/// The readout over the fleet for `range` and `realm`, at [`NOW`].
fn read(fleet: &Fleet, range: Range, realm: Option<&str>) -> Analytics {
    let (alpha, beta) = (runs(&fleet.alpha), runs(&fleet.beta));
    let names = |name: &str| vec![name.to_string()];
    let (alpha_realms, beta_realms, gamma_realms) = (names("alpha"), names("beta"), names("gamma"));
    let hearths = [
        AnalyticsHearth {
            label: "alpha",
            realms: &alpha_realms,
            journal: "a.db",
            runs: &alpha,
            refusal: None,
        },
        AnalyticsHearth {
            label: "beta",
            realms: &beta_realms,
            journal: "b.db",
            runs: &beta,
            refusal: None,
        },
        AnalyticsHearth {
            label: "gamma",
            realms: &gamma_realms,
            journal: "g.db",
            runs: &[],
            refusal: Some("no such journal"),
        },
    ];
    analytics(&Query {
        hearths: &hearths,
        range,
        realm,
        now: NOW,
    })
    .unwrap()
}

#[test]
fn shipped_counts_against_the_previous_equal_range() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.headline.shipped,
        Shipped {
            count: 2,
            previous: 1,
            change: 1,
            change_text: "▲ 1 vs previous 7d".to_string()
        }
    );
    assert_eq!(
        (view.from.as_str(), view.to.as_str()),
        ("2026-10-06T12:00:00Z", NOW)
    );
    assert_eq!(change_text(-2, Range::Day), "▼ 2 vs previous 24h");
    assert_eq!(change_text(0, Range::Month), "no change vs previous 30d");
}

#[test]
fn the_median_runs_start_to_end_with_parked_time_in_it() {
    let view = read(&fleet(), Range::Week, None);
    // 2h, 4h and 6h: the parked run's four hours are in its six.
    let four = 4 * HOUR_MILLIS;
    assert_eq!(
        view.headline.median,
        Span {
            millis: Some(four),
            text: "4h 00m".to_string()
        }
    );
    let alpha = &view.realms[0];
    assert_eq!(
        alpha.median,
        Span {
            millis: Some(5 * HOUR_MILLIS),
            text: "5h 00m".to_string()
        }
    );
    assert_eq!(span_text(42 * 60_000), "42m");
}

#[test]
fn waiting_runs_from_a_park_to_the_operators_next_command() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.headline.waiting,
        Waiting {
            waiting_millis: 4 * HOUR_MILLIS,
            parked_millis: 6 * HOUR_MILLIS,
            text: "4.0h".to_string(),
            share_percent: Some(67),
            share_text: "67% of all parked time".to_string(),
        }
    );
    assert_eq!(
        view.waiting_by_step,
        [
            StepWait {
                phase: Some("clarify".into()),
                waiting_millis: 2 * HOUR_MILLIS,
                runs: 1,
                text: "2.0h · 1".into()
            },
            StepWait {
                phase: Some("review".into()),
                waiting_millis: 2 * HOUR_MILLIS,
                runs: 1,
                text: "2.0h · 1".into()
            },
        ]
    );
}

#[test]
fn stopped_by_a_rule_names_the_most_frequent_rule() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.headline.stopped_by_rule,
        StoppedByRule {
            count: 1,
            top: Some(RuleCount {
                rule_id: Some("REVIEW-FAIL-TWICE".to_string()),
                count: 1
            }),
            text: "1 by REVIEW-FAIL-TWICE".to_string(),
            rules_more: 0,
        }
    );
}

#[test]
fn reported_cost_names_the_harnesses_that_report_none() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.headline.cost,
        CostFigure {
            cost: ReportedCost::Reported { usd: 1.5 },
            text: "$1.50".to_string(),
            reporting: vec!["claude".to_string()],
            silent: vec!["codex".to_string()],
            note: Some("claude only · codex report none".to_string()),
            providers_more: 0,
        }
    );
    let beta = &view.realms[1];
    assert_eq!(
        (&beta.cost, beta.cost_text.as_str()),
        (&ReportedCost::NotReported, "not reported")
    );
    assert_eq!(listed(&["a".into(), "b".into(), "c".into()]), "a, b and c");
}

#[test]
fn each_day_counts_shipped_against_stopped() {
    let view = read(&fleet(), Range::Week, None);
    let days: Vec<(&str, usize, usize)> = view
        .per_day
        .iter()
        .map(|day| (day.date.as_str(), day.shipped, day.stopped))
        .collect();
    assert_eq!(
        days,
        [
            ("2026-10-06", 0, 0),
            ("2026-10-07", 0, 0),
            ("2026-10-08", 0, 0),
            ("2026-10-09", 0, 0),
            ("2026-10-10", 1, 0),
            ("2026-10-11", 0, 1),
            ("2026-10-12", 1, 0),
            ("2026-10-13", 0, 0),
        ]
    );
}

#[test]
fn rework_counts_implement_visits_and_reviews_that_sent_work_back() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.rework,
        Rework {
            runs: 3,
            implement_visits: Some(4.0 / 3.0),
            implement_visits_text: "1.3".to_string(),
            sent_back: 1,
            second_try_passed: 1,
            second_try_of: 1,
            second_try_percent: Some(100),
            second_try_text: "100%".to_string(),
        }
    );
}

#[test]
fn step_attempts_are_shared_by_the_model_that_served_them() {
    let view = read(&fleet(), Range::Week, None);
    assert_eq!(
        view.models,
        [
            ModelShare {
                model: Some("claude-opus".into()),
                attempts: 1,
                percent: 50,
                text: "claude-opus 50%".into()
            },
            ModelShare {
                model: Some("gpt-5".into()),
                attempts: 1,
                percent: 50,
                text: "gpt-5 50%".into()
            },
        ]
    );
}

#[test]
fn every_realm_has_its_row_and_a_hearth_that_will_not_load_is_quarantined() {
    let view = read(&fleet(), Range::Week, None);
    let rows: Vec<(&str, usize, usize, &str, usize)> = view
        .realms
        .iter()
        .map(|row| {
            (
                row.realm.as_str(),
                row.shipped,
                row.stopped,
                row.waiting_text.as_str(),
                row.unreadable_runs,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [("alpha", 1, 1, "2.0h", 1), ("beta", 1, 0, "2.0h", 0)]
    );
    assert_eq!(
        view.quarantined,
        [QuarantinedHearth {
            realm: "gamma".into(),
            journal: "g.db".into(),
            refusal: "no such journal".into()
        }]
    );
    assert_eq!(view.not_recorded, NOT_RECORDED);
}

#[test]
fn a_realm_keeps_only_its_own_runs_rows_and_refusals() {
    let view = read(&fleet(), Range::Week, Some("beta"));
    assert_eq!(view.realm.as_deref(), Some("beta"));
    assert_eq!(
        (view.headline.shipped.count, view.headline.shipped.previous),
        (1, 1)
    );
    assert_eq!(view.headline.cost.cost, ReportedCost::NotReported);
    assert_eq!(view.realms.len(), 1);
    assert_eq!(view.quarantined, []);
}

/// A run that shipped exactly `ago` before [`NOW`].
fn shipped_at(ago: i64) -> (Journal, Option<RunState>) {
    let at = stamp(brokkr_core::stamp::parse_millis(NOW).unwrap() - ago);
    let journal =
        Journal::started(Some("alpha"), &at).then(EventType::RunCompleted, json!({}), &at);
    (journal, Some(ended(Status::Completed, "done", None, 0)))
}

#[test]
fn each_range_holds_its_first_moment_and_not_now() {
    for range in Range::ALL {
        let span = range.millis();
        let fleet = Fleet {
            alpha: vec![shipped_at(span), shipped_at(span + 1), shipped_at(0)],
            beta: Vec::new(),
        };
        let shipped = read(&fleet, range, None).headline.shipped;
        assert_eq!(
            (shipped.count, shipped.previous),
            (1, 1),
            "{}",
            range.word()
        );
    }
    assert_eq!(
        Range::ALL.map(Range::millis),
        [DAY_MILLIS, 7 * DAY_MILLIS, 30 * DAY_MILLIS]
    );
}

#[test]
fn now_is_an_argument_and_a_range_is_a_word() {
    let later = |now: &str| {
        analytics(&Query {
            hearths: &[],
            range: Range::Day,
            realm: None,
            now,
        })
        .map(|view| view.from)
    };
    assert_eq!(
        later("2026-10-14T00:00:00Z").as_deref(),
        Ok("2026-10-13T00:00:00Z")
    );
    assert_eq!(
        later("yesterday"),
        Err(AnalyticsError::Clock("yesterday".to_string()))
    );
    assert_eq!(
        AnalyticsError::Clock("yesterday".to_string()).to_string(),
        "the clock read 'yesterday', which is not an RFC 3339 time"
    );
    assert_eq!("30d".parse::<Range>(), Ok(Range::Month));
    let unknown = "2d".parse::<Range>().unwrap_err();
    assert_eq!(unknown, AnalyticsError::UnknownRange("2d".to_string()));
    assert_eq!(
        unknown.to_string(),
        "unknown range '2d': one of 24h, 7d or 30d"
    );
    assert_eq!(serde_json::to_value(Range::Week).unwrap(), json!("7d"));
}

#[test]
fn an_empty_fleet_states_its_absences() {
    let view = analytics(&Query {
        hearths: &[],
        range: Range::Day,
        realm: None,
        now: NOW,
    })
    .unwrap();
    assert_eq!(view.headline.median.text, ABSENT);
    assert_eq!(view.headline.waiting.share_text, "nothing parked");
    assert_eq!(view.headline.stopped_by_rule.text, "");
    assert_eq!(view.headline.cost.note, None);
    assert_eq!(view.rework.implement_visits_text, ABSENT);
    assert_eq!(view.rework.second_try_text, ABSENT);
    assert_eq!(
        view.per_day.len(),
        2,
        "a 24h range ending at noon spans two dates"
    );
}

/// The readout over one hearth of `runs` for `range`, at [`NOW`].
fn read_one(runs: &[AnalyticsRun], range: Range) -> Analytics {
    let realms = vec!["edge".to_string()];
    let hearths = [AnalyticsHearth {
        label: "edge",
        realms: &realms,
        journal: "e.db",
        runs,
        refusal: None,
    }];
    analytics(&Query {
        hearths: &hearths,
        range,
        realm: None,
        now: NOW,
    })
    .unwrap()
}

/// What the journal can hold that the main fleet does not: a run its
/// operator stopped, rules that tie and rules that do not, an event with no
/// readable time, operator events that are not the answer, a second answer,
/// a park outside the range, a refusal journaled after the stop, a ruling
/// from a step other than review, and seat records that name no provider,
/// no model, no record or a member no start announced.
#[test]
fn the_edges_of_the_journal_are_read_as_the_readout_says() {
    let stopped = |rule: &str, at: &str| {
        (
            Journal::started(None, "2026-10-10T00:00:00Z").then(
                EventType::RunStopped,
                json!({}),
                at,
            ),
            Some(ended(Status::Stopped, "stop", Some(rule), 1)),
        )
    };
    let attempt = |payload: Value| {
        let mut payload = payload;
        payload["effect_id"] = json!("e");
        payload["attempt_id"] = json!("x1");
        payload
    };
    let operator = Journal::started(None, "2026-10-10T00:00:00Z")
        .entered("intake", "2026-10-10T00:10:00Z")
        // A ruling from a step other than review is no review's route,
        // even back to a step the run had entered.
        .ruled("implement", Some("intake"), "2026-10-10T00:20:00Z")
        .entered("review", "2026-10-10T00:30:00Z")
        .then(EventType::PhaseEntered, json!({"phase": "lost"}), "not a time")
        .then(EventType::RunParked, json!({}), "2026-10-10T01:00:00Z")
        .then(EventType::OperatorAccepted, json!({}), "2026-10-10T02:00:00Z")
        .then(EventType::OperatorCommanded, json!({"command": "retry"}), "2026-10-10T03:00:00Z")
        .then(EventType::OperatorCommanded, json!({"command": "stop"}), "2026-10-10T04:00:00Z")
        .entered("review", "2026-10-10T05:00:00Z")
        .then(
            EventType::EffectStarted,
            attempt(json!({"provenance": [{"agent": "a", "member": "m"}, {"agent": "a", "member": "s", "provider": "claude"}]})),
            "2026-10-10T05:10:00Z",
        )
        .then(EventType::EffectCheckpointed, attempt(json!({})), "2026-10-10T05:20:00Z")
        .then(EventType::EffectCheckpointed, attempt(json!({"checkpoint": {"member": "nobody", "model": "ghost"}})), "2026-10-10T05:30:00Z")
        .then(EventType::EffectCheckpointed, attempt(json!({"checkpoint": {"member": "s"}})), "2026-10-10T05:40:00Z")
        .then(EventType::EffectSucceeded, attempt(json!({"result": {"member": "s", "model": "m-succeeded"}})), "2026-10-10T05:50:00Z")
        .then(EventType::EffectFailed, attempt(json!({})), "2026-10-10T05:55:00Z")
        .then(EventType::RunStopped, json!({}), "2026-10-10T06:00:00Z")
        // A command refused after the stop: the run still finished at its
        // terminal event, not at the journal's last.
        .then(EventType::OperatorRejected, json!({}), "2026-10-10T07:00:00Z");
    let journals = [
        (operator, Some(state(Some("review"), Status::Stopped, None))),
        stopped("A-ONCE", "2026-10-10T01:00:00Z"),
        stopped("B-TWICE", "2026-10-10T02:00:00Z"),
        stopped("B-TWICE", "2026-10-10T03:00:00Z"),
        stopped("C-ONCE", "2026-10-10T04:00:00Z"),
    ];
    let runs = runs(&journals);

    let week = read_one(&runs, Range::Week);
    // The operator's stop is not a rule's; the rules' leader beats an
    // earlier and a later single.
    assert_eq!(
        week.headline.stopped_by_rule,
        StoppedByRule {
            count: 4,
            top: Some(RuleCount {
                rule_id: Some("B-TWICE".to_string()),
                count: 2
            }),
            text: "2 by B-TWICE".to_string(),
            rules_more: 0,
        }
    );
    // Waiting runs from the park to the first command: the acceptance
    // before it and the second command after it answer nothing.
    assert_eq!(week.waiting_by_step.len(), 1);
    assert_eq!(week.waiting_by_step[0].phase.as_deref(), Some("review"));
    assert_eq!(week.waiting_by_step[0].waiting_millis, 2 * 3_600_000);
    // The implement ruling back to intake is no review's: no review sent
    // the run back, so no first review did either.
    assert_eq!(week.rework.sent_back, 0);
    assert_eq!(week.rework.second_try_of, 0);
    // Only the seat a start announced with a provider is a site, and its
    // model is the one its success named.
    let models: Vec<&str> = week
        .models
        .iter()
        .map(|share| share.model.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(models, ["m-succeeded"]);

    // A day that ends long after the park holds none of its wait.
    assert!(read_one(&runs, Range::Day).waiting_by_step.is_empty());
}

/// January and February are the civil calendar's shifted months.
#[test]
fn a_stamp_reads_january_and_february_as_their_own_months() {
    for text in [
        "2026-01-31T23:59:59Z",
        "2026-02-01T00:00:00Z",
        "2024-02-29T12:00:00Z",
        "2026-12-31T00:00:00Z",
    ] {
        assert_eq!(stamp(brokkr_core::stamp::parse_millis(text).unwrap()), text);
    }
}

/// An absence is a variant on the wire, not a word (decision 0071 ruling
/// 3): a stop no rule recorded and a seat that reported no model are
/// `None`, and only their text says so.
#[test]
fn a_missing_rule_and_a_missing_model_are_absent_not_words() {
    let journals = [(
        Journal::started(None, "2026-10-10T00:00:00Z")
            .then(
                EventType::EffectStarted,
                json!({"effect_id": "e", "attempt_id": "q1",
                       "provenance": [{"agent": "a", "provider": "dsh"}]}),
                "2026-10-10T00:10:00Z",
            )
            .then(EventType::RunStopped, json!({}), "2026-10-10T01:00:00Z"),
        Some(ended(Status::Stopped, "stop", None, 1)),
    )];
    let runs = runs(&journals);
    let week = read_one(&runs, Range::Week);
    let top = week.headline.stopped_by_rule.top.as_ref().unwrap();
    assert_eq!(top.rule_id, None);
    assert_eq!(week.headline.stopped_by_rule.text, format!("1 by {ABSENT}"));
    assert_eq!(week.models.len(), 1);
    assert_eq!(week.models[0].model, None);
    assert_eq!(week.models[0].text, "not reported 100%");
    let wire = serde_json::to_value(&week).unwrap();
    assert_eq!(wire["models"][0]["model"], Value::Null);
    assert_eq!(
        wire["headline"]["stopped_by_rule"]["top"]["rule_id"],
        Value::Null
    );
}

// ------------------------------------------------------------ recipes

/// How a recipe's run stands: done, stopped by a rule, still open, or
/// ended as [`ended_as`] ends it.
#[derive(Clone, Copy)]
enum Fate {
    Done,
    Stop,
    Open,
    Ended(Ending),
}

/// A run of `recipe` in `realm` on 10 October whose one seat ran on
/// `provider`, reporting `cost` and, when it reports any, `tokens`.
fn recipe_run(
    realm: &str,
    recipe: &str,
    fate: Fate,
    provider: &str,
    cost: Option<f64>,
) -> (Journal, Option<RunState>) {
    let mut journal = Journal::started(Some(realm), "2026-10-10T00:00:00Z").seat(
        "s1",
        provider,
        "m",
        cost,
        "2026-10-10T00:10:00Z",
    );
    journal.events[0].payload["manifest"]["bundle_name"] = json!(recipe);
    journal.events[2].payload["checkpoint"]["input_tokens"] = json!(900);
    journal.events[2].payload["checkpoint"]["output_tokens"] = json!(100);
    let (journal, state) = match fate {
        Fate::Done => (
            journal.then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z"),
            ended(Status::Completed, "done", None, 1),
        ),
        Fate::Stop => (
            journal.then(EventType::RunStopped, json!({}), "2026-10-10T01:00:00Z"),
            ended(Status::Stopped, "stop", Some("VERIFY-FAIL"), 1),
        ),
        Fate::Open => (journal, state(Some("review"), Status::Running, None)),
        Fate::Ended(ending) => ended_as(journal, ending, "2026-10-10T01:00:00Z"),
    };
    (journal, Some(state))
}

/// `done` runs that reached done and `stopped` that did not, each seat on
/// `provider` reporting `cost`.
fn recipe_runs(
    realm: &str,
    recipe: &str,
    (done, stopped): (usize, usize),
    provider: &str,
    cost: Option<f64>,
) -> Vec<(Journal, Option<RunState>)> {
    let fates =
        std::iter::repeat_n(Fate::Done, done).chain(std::iter::repeat_n(Fate::Stop, stopped));
    fates
        .map(|fate| recipe_run(realm, recipe, fate, provider, cost))
        .collect()
}

fn recipes_of(journals: &[(Journal, Option<RunState>)]) -> Vec<RecipeRow> {
    read_one(&runs(journals), Range::Week).recipes
}

fn row<'a>(rows: &'a [RecipeRow], realm: &str, recipe: &str) -> &'a RecipeRow {
    let found = rows
        .iter()
        .find(|row| row.realm == realm && row.recipe == recipe);
    found.unwrap()
}

#[test]
fn a_recipes_cost_per_run_is_what_reported_over_all_its_runs() {
    let journals = [
        recipe_run("alpha", "self", Fate::Done, "claude", Some(3.0)),
        recipe_run("alpha", "self", Fate::Stop, "dsh", None),
        recipe_run("alpha", "self", Fate::Open, "codex", None),
        recipe_run("alpha", "self", Fate::Stop, "claude", Some(1.4)),
    ];
    let rows = recipes_of(&journals);
    assert_eq!(
        rows,
        [RecipeRow {
            realm: "alpha".into(),
            recipe: "self".into(),
            runs: 4,
            done: 1,
            // Of the three that were judged; the open run has no ending.
            done_rate: 100.0 / 3.0,
            done_text: "33%".into(),
            ledger: Ledger {
                judged: 3,
                shipped: 1,
                rate_percent: Some(33),
                excluded: excluded([0; 6]),
                unclassified: 0,
                text: "1 of 3 shipped (33%) · 0 excluded · 0 unclassified".into(),
            },
            cost_per_run: CostFigure {
                cost: ReportedCost::Reported {
                    usd: (3.0 + 1.4) / 4.0
                },
                text: "$1.10".into(),
                reporting: vec!["claude".into()],
                silent: vec!["codex".into(), "dsh".into()],
                note: Some("claude only · codex and dsh report none".into()),
                providers_more: 0,
            },
            tokens_per_run: crate::tokens::usage_cells(
                Some(&crate::TokenUsage {
                    input_tokens: Some(900),
                    output_tokens: Some(100),
                    total_tokens: Some(1000),
                    ..crate::TokenUsage::default()
                }),
                ""
            )
            .1,
            on_frontier: Some(true),
        }]
    );
}

#[test]
fn a_recipe_reaching_done_more_often_for_less_dominates_another() {
    let mut journals = recipe_runs("alpha", "a", (2, 3), "claude", Some(2.0));
    journals.extend(recipe_runs("alpha", "b", (3, 7), "claude", Some(3.0)));
    let rows = recipes_of(&journals);
    let a = row(&rows, "alpha", "a");
    assert_eq!((a.done_rate, a.on_frontier), (40.0, Some(true)));
    let b = row(&rows, "alpha", "b");
    assert_eq!((b.done_rate, b.on_frontier), (30.0, Some(false)));
    assert_eq!(b.cost_per_run.text, "$3.00");
}

#[test]
fn recipes_that_tie_are_both_on_the_frontier_and_neither_dominates_across_realms() {
    let mut journals = recipe_runs("alpha", "c", (1, 1), "claude", Some(2.0));
    journals.extend(recipe_runs("alpha", "d", (1, 1), "claude", Some(2.0)));
    // Better on both, and in another realm: it dominates neither.
    journals.extend(recipe_runs("beta", "e", (2, 0), "claude", Some(1.0)));
    // Cheaper at the same rate, and more reliable at the same cost.
    journals.extend(recipe_runs("gamma", "f", (1, 1), "claude", Some(2.0)));
    journals.extend(recipe_runs("gamma", "g", (1, 1), "claude", Some(1.0)));
    journals.extend(recipe_runs("gamma", "h", (2, 0), "claude", Some(2.0)));
    let rows = recipes_of(&journals);
    let frontier: Vec<(&str, Option<bool>)> = rows
        .iter()
        .map(|row| (row.recipe.as_str(), row.on_frontier))
        .collect();
    assert_eq!(
        frontier,
        [
            ("c", Some(true)),
            ("d", Some(true)),
            ("e", Some(true)),
            ("f", Some(false)),
            ("g", Some(true)),
            ("h", Some(true)),
        ]
    );
}

#[test]
fn a_recipe_with_no_reported_cost_is_off_the_plot_and_dominates_nothing() {
    let mut journals = recipe_runs("alpha", "silent", (3, 0), "codex", None);
    journals.extend(recipe_runs("alpha", "priced", (1, 1), "claude", Some(2.0)));
    // A run before the range, and one whose start names no recipe.
    let mut early = recipe_run("alpha", "early", Fate::Done, "claude", Some(1.0));
    early.0.events[3].recorded_at = "2026-09-01T00:00:00Z".into();
    journals.push(early);
    journals.push((
        Journal::started(Some("alpha"), "2026-10-10T00:00:00Z").then(
            EventType::RunCompleted,
            json!({}),
            "2026-10-10T01:00:00Z",
        ),
        Some(ended(Status::Completed, "done", None, 0)),
    ));
    let rows = recipes_of(&journals);
    let names: Vec<&str> = rows.iter().map(|row| row.recipe.as_str()).collect();
    assert_eq!(names, ["priced", "silent"]);
    let silent = row(&rows, "alpha", "silent");
    assert_eq!(
        (
            &silent.cost_per_run.cost,
            silent.cost_per_run.text.as_str(),
            silent.on_frontier
        ),
        (&ReportedCost::NotReported, "not reported", None)
    );
    assert_eq!(row(&rows, "alpha", "priced").on_frontier, Some(true));
    let wire = serde_json::to_value(read_one(&runs(&journals), Range::Week)).unwrap();
    assert_eq!(wire["view_version"], json!(29));
    assert_eq!(wire["recipes"][1]["on_frontier"], Value::Null);
    assert_eq!(
        wire["recipes"][1]["cost_per_run"]["cost"],
        json!({"kind": "not_reported"})
    );
}

/// Decision 0064 ruling 7: a recipe none of whose runs was judged has no
/// rate, so it is not ranked and dominates nothing. Cheaper at the same
/// zero, the superseded recipe would otherwise push the failed one off.
#[test]
fn chief_unjudged_recipe_does_not_dominate_a_judged_recipe() {
    let superseded = Fate::Ended(Ending::Superseded);
    let journals = [
        recipe_run("alpha", "settled", superseded, "claude", Some(1.0)),
        recipe_run("alpha", "tried", Fate::Stop, "claude", Some(2.0)),
    ];
    let rows = recipes_of(&journals);
    let ranked: Vec<(&str, Option<i64>, Option<bool>)> = rows
        .iter()
        .map(|row| {
            let rate = row.ledger.rate_percent;
            (row.recipe.as_str(), rate, row.on_frontier)
        })
        .collect();
    assert_eq!(
        ranked,
        [("settled", None, None), ("tried", Some(0), Some(true))]
    );
}

#[test]
fn a_recipe_whose_seats_meter_no_tokens_states_the_absence() {
    let mut journals = vec![recipe_run(
        "alpha",
        "quiet",
        Fate::Done,
        "claude",
        Some(1.0),
    )];
    let checkpoint = &mut journals[0].0.events[2].payload["checkpoint"];
    checkpoint["input_tokens"] = Value::Null;
    checkpoint["output_tokens"] = Value::Null;
    let rows = recipes_of(&journals);
    assert_eq!(cells(&rows[0].tokens_per_run), [ABSENT; 4]);
}

#[test]
fn a_seat_started_before_the_range_counts_what_it_metered_inside_it() {
    let mut journals = vec![recipe_run(
        "alpha",
        "spanning",
        Fate::Open,
        "claude",
        Some(1.0),
    )];
    // The run and its attempt start before the week begins; the attempt's
    // session finishes inside it.
    let events = &mut journals[0].0.events;
    events[0].recorded_at = "2026-10-06T11:00:00Z".into();
    events[1].recorded_at = "2026-10-06T11:30:00Z".into();
    events[2].recorded_at = "2026-10-06T12:30:00Z".into();
    // A checkpoint before the window: the readout and the recipe drop it,
    // and a site that only ran before the window names no provider.
    let (journal, state) = journals.pop().unwrap();
    let outside = json!({"effect_id": "e", "attempt_id": "s1",
                         "checkpoint": {"total_cost_usd": 5.0}});
    journals.push((
        journal.then(
            EventType::EffectCheckpointed,
            outside,
            "2026-09-01T00:00:00Z",
        ),
        state,
    ));
    let rows = recipes_of(&journals);
    let cost = &rows[0].cost_per_run;
    assert_eq!(
        (
            cost.text.as_str(),
            cost.reporting.as_slice(),
            cells(&rows[0].tokens_per_run)
        ),
        (
            "$1.00",
            ["claude".to_string()].as_slice(),
            ["1k tok", "900", "100", ABSENT]
        )
    );
}

#[test]
fn a_recipe_site_outside_the_window_names_no_provider() {
    let (mut journal, state) = recipe_run("alpha", "old", Fate::Open, "claude", Some(1.0));
    for event in &mut journal.events {
        event.recorded_at = "2026-09-01T00:00:00Z".into();
    }
    let rows = recipes_of(&[(journal, state)]);
    let old = row(&rows, "alpha", "old");
    assert_eq!(old.runs, 1);
    assert_eq!(old.cost_per_run.cost, ReportedCost::NotReported);
    assert_eq!(old.cost_per_run.reporting, Vec::<String>::new());
    assert_eq!(old.cost_per_run.silent, Vec::<String>::new());
}

#[test]
fn a_recipes_cache_hit_counts_its_cache_writes_as_misses() {
    // Each run read 800 of its 900 input from cache and wrote 100 more to
    // it, as a Claude seat does: 800 of every 1,000 prompt tokens hit.
    let mut journals = recipe_runs("alpha", "self", (1, 1), "claude", Some(1.0));
    for (journal, _) in &mut journals {
        let checkpoint = &mut journal.events[2].payload["checkpoint"];
        checkpoint["cache_read_tokens"] = json!(800);
        checkpoint["cache_write_tokens"] = json!(100);
    }
    let rows = recipes_of(&journals);
    assert_eq!(
        cells(&rows[0].tokens_per_run),
        ["1k tok", "900", "100", "80.0%"]
    );
}

#[test]
fn a_recipes_claude_messages_are_metered_once_whatever_lines_carried_them() {
    // The driver journaled a turn for each line Claude Code streamed, each
    // repeating its message's usage: the attempt walk meters each message
    // once, as the run view does, and takes the output from the seat's
    // session record (100 here), which sums what its results reported.
    let (mut journal, state) = recipe_run("alpha", "self", Fate::Open, "claude", Some(1.0));
    for checkpoint in crate::tests::cache_hit::streamed_turns() {
        let payload = json!({"effect_id": "e", "attempt_id": "s1", "checkpoint": checkpoint});
        journal = journal.then(
            EventType::EffectCheckpointed,
            payload,
            "2026-10-10T00:20:00Z",
        );
    }
    let rows = recipes_of(&[(journal, state)]);
    assert_eq!(
        cells(&rows[0].tokens_per_run),
        ["34k tok", "34k", "100", "74.3%"]
    );
}

/// A token figure as its four cells read: total, input, output, cache hit.
pub(crate) fn cells(tokens: &crate::TokenCells) -> [&str; 4] {
    [
        &tokens.total.text,
        &tokens.input.text,
        &tokens.output.text,
        &tokens.cache_hit.text,
    ]
}

/// `journal` ended as `ending` at `at`, with the state it folds to: a
/// policy ending by the ruling that stopped it, an operator's by the cause
/// its accepted stop names (decision 0064 rulings 3 and 4). The graph's
/// fixture shares it.
pub(crate) fn ended_as<J: Builds>(journal: J, ending: Ending, at: &str) -> (J, RunState) {
    let stop = |journal: J, rule: &str, from: &str, result: &str| {
        let ruling = json!({"rule_id": rule, "from": from, "result": result, "next": "stop"});
        let journal = journal.then(EventType::RunStopped, json!({}), at);
        (journal, state(Some("stop"), Status::Stopped, Some(ruling)))
    };
    let mut args = json!({"cause": ending.word()});
    match ending {
        Ending::Shipped => {
            let journal = journal.then(EventType::RunCompleted, json!({}), at);
            return (journal, state(Some("done"), Status::Completed, None));
        }
        Ending::ShippedFlagged => {
            let ruling = json!({"rule_id": "REVIEW-RESIDUAL-OK", "from": "review",
                                "result": "residual", "next": "done", "severity": "flagged"});
            let journal = (journal.then(EventType::TransitionDecided, ruling, at)).then(
                EventType::RunCompleted,
                json!({}),
                at,
            );
            return (journal, state(Some("done"), Status::Completed, None));
        }
        Ending::Refused => {
            return stop(journal, "REVIEW-SECURITY-HOLD", "review", "security-hold");
        }
        Ending::Failed => return stop(journal, "VERIFY-FAIL", "verify", "fail"),
        Ending::Blocked => return stop(journal, "IMPL-BLOCKED", "implement", "blocked"),
        Ending::Superseded => args["by"] = json!({"realm": null, "run_id": "later"}),
        Ending::DeliveredElsewhere => args["ref"] = json!("abc123"),
        Ending::Unclassified => args = Value::Null,
        Ending::Exhausted
        | Ending::Infrastructure
        | Ending::Withdrawn
        | Ending::OperatorStopped => {}
    }
    let mut stop = json!({"command_id": "c", "command": "stop"});
    if !args.is_null() {
        stop["args"] = args;
    }
    let journal = (journal.then(EventType::OperatorCommanded, stop, at))
        .then(EventType::OperatorAccepted, json!({"command_id": "c"}), at)
        .then(EventType::RunStopped, json!({}), at);
    (journal, state(Some("implement"), Status::Stopped, None))
}

/// A run of `self` in the one hearth for each ending in `endings`, each
/// finished on 10 October.
fn ended_runs(endings: &[Ending]) -> Vec<(Journal, Option<RunState>)> {
    let run = |ending: &Ending| {
        let mut started = Journal::started(None, "2026-10-10T00:00:00Z");
        started.events[0].payload["manifest"]["bundle_name"] = json!("self");
        let (journal, state) = ended_as(started, *ending, "2026-10-10T01:00:00Z");
        (journal, Some(state))
    };
    endings.iter().map(run).collect()
}

/// The six exclusions, in ruling 2's order, with their counts.
pub(crate) fn excluded(counts: [usize; 6]) -> std::collections::BTreeMap<Ending, usize> {
    let endings = [
        Ending::Blocked,
        Ending::Superseded,
        Ending::Infrastructure,
        Ending::DeliveredElsewhere,
        Ending::Withdrawn,
        Ending::OperatorStopped,
    ];
    endings.into_iter().zip(counts).collect()
}

/// Decision 0064 ruling 7: one run of each of the twelve endings. The rate
/// counts the five outcomes alone, two of which shipped; each of the six
/// exclusions is counted by its ending, and `unclassified` apart. Every
/// figure that sets shipped beside stopped reads the outcomes the same way.
#[test]
fn the_ledger_rates_the_five_outcomes_and_counts_each_exclusion_by_ending() {
    let journals = ended_runs(&Ending::ALL);
    let view = read_one(&runs(&journals), Range::Week);
    let ledger = Ledger {
        judged: 5,
        shipped: 2,
        rate_percent: Some(40),
        excluded: excluded([1; 6]),
        unclassified: 1,
        text: "2 of 5 shipped (40%) · 6 excluded: 1 blocked, 1 superseded, 1 infrastructure, \
               1 delivered-elsewhere, 1 withdrawn, 1 operator-stopped · 1 unclassified"
            .into(),
    };
    assert_eq!(view.headline.ledger, ledger);
    let edge = &view.realms[0];
    assert_eq!((edge.shipped, edge.stopped, &edge.ledger), (2, 3, &ledger));
    let day = view.per_day.iter().find(|day| day.date == "2026-10-10");
    assert_eq!(day.map(|day| (day.shipped, day.stopped)), Some((2, 3)));
    let recipe = row(&view.recipes, "edge", "self");
    assert_eq!(
        (
            recipe.runs,
            recipe.done,
            recipe.done_rate,
            &recipe.done_text
        ),
        (12, 2, 40.0, &"40%".to_string())
    );
    assert_eq!(recipe.ledger, ledger);
}

/// A recipe none of whose runs was judged has no rate to state: its done
/// rate is 0 and its text the absence mark, and the ledger says what it
/// excluded, a count of each.
#[test]
fn a_recipe_none_of_whose_runs_was_judged_states_no_rate() {
    let journals = ended_runs(&[Ending::Superseded, Ending::Superseded, Ending::Unclassified]);
    let view = read_one(&runs(&journals), Range::Week);
    let recipe = row(&view.recipes, "edge", "self");
    assert_eq!((recipe.done_rate, recipe.done_text.as_str()), (0.0, ABSENT));
    assert_eq!(
        recipe.ledger,
        Ledger {
            judged: 0,
            shipped: 0,
            rate_percent: None,
            excluded: excluded([0, 2, 0, 0, 0, 0]),
            unclassified: 1,
            text: "0 of 0 shipped (—) · 2 excluded: 2 superseded · 1 unclassified".into(),
        }
    );
    assert_eq!(
        Ledger::of([]).text,
        "0 of 0 shipped (—) · 0 excluded · 0 unclassified"
    );
}

// ------------------------------------------------- the 64 groups and cuts

/// A run of `count` seats, each serving a model of its own.
fn many_models(count: usize) -> (Journal, Option<RunState>) {
    let mut journal = Journal::started(Some("alpha"), "2026-10-10T00:00:00Z");
    for index in 0..count {
        journal = journal.seat(
            &format!("s{index}"),
            "claude",
            &format!("model-{index:05}"),
            None,
            "2026-10-10T00:10:00Z",
        );
    }
    (
        journal.then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z"),
        Some(ended(Status::Completed, "done", None, 0)),
    )
}

/// A run that parks once in each of `count` phases, each a minute apart and
/// each answered half a minute later.
/// `journal` enters `phase` at `at`, parks, and is answered half a minute
/// later.
fn parks_at(journal: Journal, phase: &str, at: i64) -> Journal {
    let entered = stamp(at);
    let answered = stamp(at + 30_000);
    journal
        .entered(phase, &entered)
        .then(EventType::RunParked, json!({}), &entered)
        .then(
            EventType::OperatorCommanded,
            json!({"command": "retry"}),
            &answered,
        )
}

fn many_phases(count: usize) -> (Journal, Option<RunState>) {
    let start = brokkr_core::stamp::parse_millis("2026-10-06T12:30:00Z").unwrap();
    let mut journal = Journal::started(Some("alpha"), "2026-10-06T12:30:00Z");
    for index in 0..count {
        journal = parks_at(
            journal,
            &format!("phase-{index:05}"),
            start + index as i64 * 60_000,
        );
    }
    let end = stamp(start + count as i64 * 60_000);
    (
        journal.then(EventType::RunStopped, json!({}), &end),
        Some(ended(Status::Stopped, "stop", None, 0)),
    )
}

/// The readout of a run of `count` models and one of `count` phases.
fn two_grouped_figures(count: usize) -> Analytics {
    let (models, state) = many_models(count);
    let (phases, phase_state) = many_phases(count);
    read_one(
        &runs(&[(models, state), (phases, phase_state)]),
        Range::Week,
    )
}

fn keeps_64(view: &Analytics) {
    assert_eq!(view.models.len(), 64);
    assert_eq!(view.waiting_by_step.len(), 64);
}

#[test]
fn a_figure_past_64_groups_keeps_64_and_counts_the_rest() {
    let view = two_grouped_figures(65);
    keeps_64(&view);
    assert_eq!(view.models_more, 1);
    assert_eq!(view.waiting_by_step_more, 1);
    assert_eq!(
        view.models.first().and_then(|share| share.model.as_deref()),
        Some("model-00000")
    );
    assert_eq!(
        view.models.last().and_then(|share| share.model.as_deref()),
        Some("model-00063"),
        "a tie keeps the key order"
    );
    assert_eq!(
        view.waiting_by_step
            .first()
            .and_then(|step| step.phase.as_deref()),
        Some("phase-00000")
    );
    assert_eq!(
        view.waiting_by_step
            .last()
            .and_then(|step| step.phase.as_deref()),
        Some("phase-00063")
    );

    let view = two_grouped_figures(9_000);
    keeps_64(&view);
    assert_eq!(view.models_more, 8_936);
    assert_eq!(view.waiting_by_step_more, 8_936);
}

/// A run of `count` models where the last key serves two attempts more:
/// the replacement `ranked` makes when a later key outranks a kept one.
fn many_models_with_a_leader(count: usize) -> (Journal, Option<RunState>) {
    let (mut journal, state) = many_models(count);
    for extra in 0..2 {
        journal = journal.seat(
            &format!("lead{extra}"),
            "claude",
            &format!("model-{:05}", count - 1),
            None,
            "2026-10-10T00:20:00Z",
        );
    }
    (journal, state)
}

#[test]
fn a_later_key_with_more_attempts_outranks_the_last_kept_one() {
    let (models, state) = many_models_with_a_leader(65);
    let view = read_one(&runs(&[(models, state)]), Range::Week);
    assert_eq!(view.models.len(), 64);
    assert_eq!(view.models_more, 1);
    assert_eq!(
        view.models.first().and_then(|share| share.model.as_deref()),
        Some("model-00064")
    );
    assert_eq!(
        view.models.last().and_then(|share| share.model.as_deref()),
        Some("model-00062"),
        "the last key, with one attempt, is the one dropped"
    );
}

/// A run of models `a00..a63` once, `b00..b63` twice and `c00` three
/// times: the tie at two attempts is decided while a group is evicted,
/// once the kept order no longer follows the key.
fn tied_models() -> (Journal, Option<RunState>) {
    let mut journal = Journal::started(Some("alpha"), "2026-10-10T00:00:00Z");
    for index in 0..64 {
        journal = journal.seat(
            &format!("a{index}"),
            "claude",
            &format!("a{index:02}"),
            None,
            "2026-10-10T00:10:00Z",
        );
    }
    for index in 0..64 {
        for extra in 0..2 {
            journal = journal.seat(
                &format!("b{index}-{extra}"),
                "claude",
                &format!("b{index:02}"),
                None,
                "2026-10-10T00:10:00Z",
            );
        }
    }
    for extra in 0..3 {
        journal = journal.seat(
            &format!("c-{extra}"),
            "claude",
            "c00",
            None,
            "2026-10-10T00:10:00Z",
        );
    }
    (
        journal.then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z"),
        Some(ended(Status::Completed, "done", None, 0)),
    )
}

#[test]
fn a_tie_while_evicting_keeps_the_earlier_key() {
    let (models, state) = tied_models();
    let view = read_one(&runs(&[(models, state)]), Range::Week);
    let names: Vec<&str> = view
        .models
        .iter()
        .map(|share| share.model.as_deref().unwrap())
        .collect();
    let expected: Vec<String> = std::iter::once("c00".to_string())
        .chain((0..63).map(|index| format!("b{index:02}")))
        .collect();
    assert_eq!(names, expected, "the eviction drops b63, not b00");
    assert_eq!(view.models_more, 65);
}

/// 64 recipes at $2 a run and one, the last key, at $1: the omitted one
/// still dominates every kept row of the realm.
#[test]
fn an_omitted_recipe_still_dominates_a_kept_one() {
    let mut journals = Vec::new();
    for index in 0..64 {
        journals.extend(recipe_runs(
            "alpha",
            &format!("r{index:02}"),
            (1, 0),
            "claude",
            Some(2.0),
        ));
    }
    journals.extend(recipe_runs("alpha", "r64", (1, 0), "claude", Some(1.0)));
    let view = read_one(&runs(&journals), Range::Week);
    assert_eq!(view.recipes.len(), 64);
    assert_eq!(view.recipes_more, 1);
    for row in &view.recipes {
        assert_eq!(
            row.on_frontier,
            Some(false),
            "{} is dominated by the omitted r64",
            row.recipe
        );
    }
    assert_eq!(
        row(&view.recipes, "alpha", "r00").cost_per_run.text,
        "$2.00"
    );
}

/// The readout over one hearth naming `realms`, handing it `journals`.
fn read_realms(journals: &[(Journal, Option<RunState>)], realms: &[String]) -> Analytics {
    let runs = runs(journals);
    let hearths = [AnalyticsHearth {
        label: "edge",
        realms,
        journal: "e.db",
        runs: &runs,
        refusal: None,
    }];
    analytics(&Query {
        hearths: &hearths,
        range: Range::Week,
        realm: None,
        now: NOW,
    })
    .unwrap()
}

/// A row per realm, and the 64 a window keeps: the 65th realm's runs feed
/// the headline and are counted in `realms_more`, not given a row.
/// A hearth whose map names no realm takes its label as the realm the
/// refusal is quarantined under.
#[test]
fn a_hearth_whose_map_names_no_realm_takes_its_label() {
    let hearths = [AnalyticsHearth {
        label: "hearth",
        realms: &[],
        journal: "h.db",
        runs: &[],
        refusal: Some("no such journal"),
    }];
    let view = analytics(&Query {
        hearths: &hearths,
        range: Range::Week,
        realm: None,
        now: NOW,
    })
    .unwrap();
    assert_eq!(
        view.quarantined,
        [QuarantinedHearth {
            realm: "hearth".into(),
            journal: "h.db".into(),
            refusal: "no such journal".into(),
        }]
    );
    assert_eq!(view.realms, []);
}

#[test]
fn a_readout_past_64_realms_keeps_64_rows_and_counts_the_rest() {
    let realms: Vec<String> = (0..65).map(|index| format!("realm-{index:02}")).collect();
    let journals: Vec<(Journal, Option<RunState>)> = realms
        .iter()
        .enumerate()
        .map(|(index, realm)| {
            let journal = Journal::started(Some(realm), "2026-10-10T00:00:00Z")
                .seat(
                    &format!("s{index}"),
                    "claude",
                    "m",
                    Some(1.0),
                    "2026-10-10T00:10:00Z",
                )
                .entered("review", "2026-10-10T00:20:00Z")
                .then(EventType::RunParked, json!({}), "2026-10-10T00:30:00Z")
                .then(
                    EventType::OperatorCommanded,
                    json!({"command": "retry"}),
                    "2026-10-10T00:40:00Z",
                )
                .then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z");
            (journal, Some(ended(Status::Completed, "done", None, 0)))
        })
        .collect();
    let view = read_realms(&journals, &realms);
    assert_eq!(view.headline.shipped.count, 65);
    assert_eq!(view.realms.len(), 64);
    assert_eq!(view.realms_more, 1);
    assert_eq!(
        view.realms.first().map(|row| row.realm.as_str()),
        Some("realm-00")
    );
    assert_eq!(
        view.realms.last().map(|row| row.realm.as_str()),
        Some("realm-63")
    );
    assert_eq!(view.realms[0].median.millis, Some(HOUR_MILLIS));
    assert_eq!(view.realms[0].cost, ReportedCost::Reported { usd: 1.0 });
}

// ------------------------------------- the chief's cross-realm demonstrators

/// A run that completed on 2026-10-10 with one seat on `provider`,
/// reporting `cost` when it has one.
fn completed_run(
    realm: &str,
    provider: &str,
    cost: Option<f64>,
    at: &str,
) -> (Journal, Option<RunState>) {
    let journal = Journal::started(Some(realm), at)
        .seat("s", provider, "m", cost, at)
        .then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z");
    (journal, Some(ended(Status::Completed, "done", None, 0)))
}

/// A completed run of `count` seats, each on a provider of its own, named
/// `provider-00000` upwards.
fn many_providers(realm: &str, count: usize) -> (Journal, Option<RunState>) {
    let mut journal = Journal::started(Some(realm), "2026-10-10T00:00:00Z");
    for index in 0..count {
        journal = journal.seat(
            &format!("{realm}-{index:05}"),
            &format!("provider-{index:05}"),
            "m",
            Some(1.0),
            "2026-10-10T00:10:00Z",
        );
    }
    (
        journal.then(EventType::RunCompleted, json!({}), "2026-10-10T01:00:00Z"),
        Some(ended(Status::Completed, "done", None, 0)),
    )
}

/// `count` stopped runs, each by a rule of its own, named `rule-00000`
/// upwards.
fn many_rules(count: usize) -> Vec<(Journal, Option<RunState>)> {
    (0..count)
        .map(|index| {
            let rule = format!("rule-{index:05}");
            (
                Journal::started(Some("alpha"), "2026-10-10T00:00:00Z").then(
                    EventType::RunStopped,
                    json!({}),
                    "2026-10-10T01:00:00Z",
                ),
                Some(ended(Status::Stopped, "stop", Some(&rule), 0)),
            )
        })
        .collect()
}

#[test]
fn chief753_same_provider_across_realms_is_reporting() {
    let alpha = completed_run("alpha", "claude", Some(1.0), "2026-10-10T00:10:00Z");
    let beta = completed_run("beta", "claude", None, "2026-10-10T02:10:00Z");
    let view = read_one(&runs(&[alpha, beta]), Range::Week);
    let cost = &view.headline.cost;
    assert_eq!(cost.reporting, ["claude"]);
    assert!(cost.silent.is_empty(), "{:?}", cost.silent);
    assert_eq!(cost.note, None);
    assert_eq!(cost.cost, ReportedCost::Reported { usd: 1.0 });
}

#[test]
fn chief753_provider_more_counts_distinct_providers_across_realms() {
    let alpha = many_providers("alpha", 65);
    let beta = many_providers("beta", 65);
    let view = read_one(&runs(&[alpha, beta]), Range::Week);
    let cost = &view.headline.cost;
    assert_eq!(cost.providers_more, 1);
    assert_eq!(cost.reporting.len(), 64);
    assert!(cost.silent.is_empty(), "{:?}", cost.silent);
    assert_eq!(
        cost.reporting.first().map(String::as_str),
        Some("provider-00000")
    );
    assert_eq!(
        cost.reporting.last().map(String::as_str),
        Some("provider-00063")
    );
}

#[test]
fn chief753_realm_rows_preserve_discovery_order() {
    let realms: Vec<String> = ["zeta", "alpha"].into_iter().map(str::to_string).collect();
    let view = read_realms(&[], &realms);
    let names: Vec<&str> = view.realms.iter().map(|row| row.realm.as_str()).collect();
    assert_eq!(names, ["zeta", "alpha"]);
    assert_eq!(view.realms_more, 0);
}

#[test]
fn chief753_realm_selection_obeys_declared_key_ranking() {
    let realms: Vec<String> = (0..65)
        .rev()
        .map(|index| format!("realm-{index:02}"))
        .collect();
    let view = read_realms(&[], &realms);
    let names: Vec<&str> = view.realms.iter().map(|row| row.realm.as_str()).collect();
    let expected: Vec<String> = (1..=64)
        .rev()
        .map(|index| format!("realm-{index:02}"))
        .collect();
    assert_eq!(names, expected);
    assert_eq!(view.realms_more, 1);
}

#[test]
fn a_headline_count_past_64_rules_and_providers_is_positive_on_the_wire() {
    let mut journals = many_rules(65);
    journals.push(many_providers("alpha", 65));
    journals.push(many_providers("beta", 65));
    let view = read_one(&runs(&journals), Range::Week);
    assert_eq!(view.headline.stopped_by_rule.rules_more, 1);
    assert_eq!(view.headline.cost.providers_more, 1);
    let wire = serde_json::to_value(&view).unwrap();
    assert_eq!(
        wire.pointer("/headline/stopped_by_rule/rules_more"),
        Some(&json!(1))
    );
    assert_eq!(
        wire.pointer("/headline/cost/providers_more"),
        Some(&json!(1))
    );
}

#[test]
fn a_key_past_its_bound_groups_with_its_cut_twin() {
    let sixty_four = "p".repeat(64);
    let megabyte = "x".repeat(1 << 20);
    let long_phase = format!("{sixty_four}{megabyte}");
    // The same 64 bytes, two names: one group.
    let start = brokkr_core::stamp::parse_millis("2026-10-06T12:30:00Z").unwrap();
    let mut journal = Journal::started(Some("alpha"), "2026-10-06T12:30:00Z");
    for (index, phase) in [sixty_four.as_str(), long_phase.as_str()]
        .into_iter()
        .enumerate()
    {
        journal = parks_at(journal, phase, start + index as i64 * 60_000);
    }
    let end = stamp(brokkr_core::stamp::parse_millis("2026-10-06T12:32:00Z").unwrap());
    let phases = journal.then(EventType::RunStopped, json!({}), &end);

    let eighty = "m".repeat(80);
    let long_model = format!("{eighty}{megabyte}");
    let twin_model = format!("{eighty}y");
    let (mut models, _) = many_models(0);
    models = models.seat("long", "claude", &long_model, None, "2026-10-10T00:11:00Z");
    models = models.seat("twin", "claude", &twin_model, None, "2026-10-10T00:12:00Z");
    let view = read_one(
        &runs(&[
            (models, Some(ended(Status::Completed, "done", None, 0))),
            (phases, Some(ended(Status::Stopped, "stop", None, 0))),
        ]),
        Range::Week,
    );
    let names: Vec<&str> = view
        .waiting_by_step
        .iter()
        .map(|step| step.phase.as_deref().unwrap())
        .collect();
    assert_eq!(names, [sixty_four.as_str()], "printed cut");
    assert_eq!(
        view.models.first().and_then(|share| share.model.as_deref()),
        Some(eighty.as_str())
    );
    assert_eq!(view.models.len(), 1);
    assert_eq!(view.models[0].attempts, 2);
}

/// A query's realm is matched after the same cut a row groups by: asking
/// for the printed realm selects the whole cut group, both raw names.
#[test]
fn a_realm_filter_selects_its_displayed_cut_group() {
    let realm = "r".repeat(64);
    let tail = format!("{realm}-tail");
    let run = |realm: &str, at: &str| {
        (
            Journal::started(Some(realm), at).then(EventType::RunCompleted, json!({}), at),
            Some(ended(Status::Completed, "done", None, 0)),
        )
    };
    let journals = [
        run(&realm, "2026-10-10T00:00:00Z"),
        run(&tail, "2026-10-10T01:00:00Z"),
    ];
    let reads = runs(&journals);
    let hearths = [AnalyticsHearth {
        label: "edge",
        realms: &[],
        journal: "e.db",
        runs: &reads,
        refusal: None,
    }];
    let view = analytics(&Query {
        hearths: &hearths,
        range: Range::Week,
        realm: Some(&realm),
        now: NOW,
    })
    .unwrap();
    assert_eq!(view.headline.shipped.count, 2);
    assert_eq!(view.realms.len(), 1);
    assert_eq!(view.realms[0].realm, realm);
}

/// Whether no object key is a "N more" count, however deep.
fn no_more_keys(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.keys()
                .all(|key| key != "more" && !key.ends_with("_more"))
                && map.values().all(no_more_keys)
        }
        Value::Array(values) => values.iter().all(no_more_keys),
        _ => true,
    }
}

// ------------------------------------------- the fixture journals' bytes

/// The clock every fixture golden was derived at: the base's.
const FIXTURE_NOW: &str = "2026-09-02T00:00:00Z";

/// One frozen fixture journal, and its serialized readout at each of
/// [`Range::ALL`]'s three ranges: committed goldens generated at
/// a45fc694f, in that same order.
struct Fixture {
    name: &'static str,
    journal: &'static str,
    goldens: [&'static str; 3],
}

const FIXTURES: [Fixture; 6] = [
    Fixture {
        name: "conclude-already-done-hand-built",
        journal: include_str!(
            "../../../../fixtures/journals/conclude-already-done-hand-built.ndjson"
        ),
        goldens: [
            include_str!("goldens/conclude-already-done-hand-built.24h.golden"),
            include_str!("goldens/conclude-already-done-hand-built.7d.golden"),
            include_str!("goldens/conclude-already-done-hand-built.30d.golden"),
        ],
    },
    Fixture {
        name: "conclude-broken-chain-hand-built",
        journal: include_str!(
            "../../../../fixtures/journals/conclude-broken-chain-hand-built.ndjson"
        ),
        goldens: [
            include_str!("goldens/conclude-broken-chain-hand-built.24h.golden"),
            include_str!("goldens/conclude-broken-chain-hand-built.7d.golden"),
            include_str!("goldens/conclude-broken-chain-hand-built.30d.golden"),
        ],
    },
    Fixture {
        name: "conclude-parked-hand-built",
        journal: include_str!("../../../../fixtures/journals/conclude-parked-hand-built.ndjson"),
        goldens: [
            include_str!("goldens/conclude-parked-hand-built.24h.golden"),
            include_str!("goldens/conclude-parked-hand-built.7d.golden"),
            include_str!("goldens/conclude-parked-hand-built.30d.golden"),
        ],
    },
    Fixture {
        name: "conclude-stopped-mid-effect-hand-built",
        journal: include_str!(
            "../../../../fixtures/journals/conclude-stopped-mid-effect-hand-built.ndjson"
        ),
        goldens: [
            include_str!("goldens/conclude-stopped-mid-effect-hand-built.24h.golden"),
            include_str!("goldens/conclude-stopped-mid-effect-hand-built.7d.golden"),
            include_str!("goldens/conclude-stopped-mid-effect-hand-built.30d.golden"),
        ],
    },
    Fixture {
        name: "reforging-the-road-back-hand-built",
        journal: include_str!(
            "../../../../fixtures/journals/reforging-the-road-back-hand-built.ndjson"
        ),
        goldens: [
            include_str!("goldens/reforging-the-road-back-hand-built.24h.golden"),
            include_str!("goldens/reforging-the-road-back-hand-built.7d.golden"),
            include_str!("goldens/reforging-the-road-back-hand-built.30d.golden"),
        ],
    },
    Fixture {
        name: "tui-graph-the-selection-box-gets-80f98deb",
        journal: include_str!(
            "../../../../fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson"
        ),
        goldens: [
            include_str!("goldens/tui-graph-the-selection-box-gets-80f98deb.24h.golden"),
            include_str!("goldens/tui-graph-the-selection-box-gets-80f98deb.7d.golden"),
            include_str!("goldens/tui-graph-the-selection-box-gets-80f98deb.30d.golden"),
        ],
    },
];

/// The fixture journals are frozen, and every range's serialized readout is
/// the base's bytes: a committed golden per fixture and range, generated at
/// a45fc694f, which no `more` field appears in.
#[test]
fn every_fixture_journal_serializes_as_the_base_derived_it() {
    for fixture in FIXTURES {
        let events: Vec<EventEnvelope> = fixture
            .journal
            .lines()
            .map(|line| serde_json::from_str(line).expect("a fixture event is an envelope"))
            .collect();
        let state = fold(&events).ok();
        let runs = vec![AnalyticsRun {
            events: &events,
            state: state.as_ref(),
        }];
        let hearths = [AnalyticsHearth {
            label: fixture.name,
            realms: &[],
            journal: fixture.name,
            runs: &runs,
            refusal: None,
        }];
        for (range, golden) in Range::ALL.into_iter().zip(fixture.goldens) {
            let view = analytics(&Query {
                hearths: &hearths,
                range,
                realm: None,
                now: FIXTURE_NOW,
            })
            .unwrap();
            let wire = format!("{}\n", serde_json::to_string_pretty(&view).unwrap());
            assert_eq!(wire, golden, "{} {}", fixture.name, range.word());
            let parsed: Value = serde_json::from_str(&wire).unwrap();
            assert!(
                no_more_keys(&parsed),
                "{} {}: {wire}",
                fixture.name,
                range.word()
            );
        }
    }
}

#[test]
fn the_wire_carries_no_more_field_under_sixty_five_groups() {
    let wire = serde_json::to_value(read(&fleet(), Range::Week, None)).unwrap();
    assert!(no_more_keys(&wire), "{wire}");
    assert_eq!(wire.pointer("/headline/stopped_by_rule/rules_more"), None);
    assert_eq!(wire.pointer("/headline/cost/providers_more"), None);
    assert_eq!(wire.get("models_more"), None);
    assert_eq!(wire.get("waiting_by_step_more"), None);
    assert_eq!(wire.get("realms_more"), None);
    assert_eq!(wire.get("recipes_more"), None);
}

#[test]
fn medians_equal_middle_in_at_most_twelve_passes() {
    let mut seed = 0x5eed_u64;
    let mut random = |count: usize| -> Vec<i64> {
        (0..count)
            .map(|_| {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (seed >> 33) as i64 - 1_000_000
            })
            .collect()
    };
    let datasets = vec![
        vec![Vec::new()],
        vec![vec![i64::MIN]],
        vec![vec![i64::MAX, i64::MIN]],
        vec![vec![7; 7]],
        vec![vec![4; 4]],
        vec![vec![1, 2, 1, 2, 1]],
        vec![vec![-5, 1_000, -5, 42, 9]],
        vec![random(1), random(63), random(64), random(4_096)],
        vec![vec![i64::MIN; 1], vec![i64::MAX; 1]],
    ];
    for dataset in datasets {
        let mut passes = 0;
        let medians = medians(dataset.len(), &mut |emit: &mut dyn FnMut(usize, i64)| {
            passes += 1;
            for (target, values) in dataset.iter().enumerate() {
                for value in values {
                    emit(target, *value);
                }
            }
        });
        assert!(passes <= 12, "{passes} passes for {dataset:?}");
        for (target, values) in dataset.iter().enumerate() {
            let mut sorted = values.clone();
            sorted.sort_unstable();
            assert_eq!(
                medians[target],
                middle(&sorted),
                "target {target} of {dataset:?}"
            );
        }
    }
}
