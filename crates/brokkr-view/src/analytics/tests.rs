//! The analytics readout (#568), metric by metric, on one builder-made
//! fleet of three hearths: `alpha` and `beta` load, `gamma` does not.
//! Each run's journal is built for its times, parks, rulings and seats;
//! its folded state, from the shared state builder, for how it ended.

use super::words::{change_text, listed, span_text, stamp};
use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};
use crate::ABSENT;
use brokkr_core::fold::Status;
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

/// How a recipe's run stands: done, stopped by a rule, or still open.
#[derive(Clone, Copy)]
enum Fate {
    Done,
    Stop,
    Open,
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
            done_rate: 25.0,
            done_text: "25%".into(),
            cost_per_run: CostFigure {
                cost: ReportedCost::Reported {
                    usd: (3.0 + 1.4) / 4.0
                },
                text: "$1.10".into(),
                reporting: vec!["claude".into()],
                silent: vec!["codex".into(), "dsh".into()],
                note: Some("claude only · codex and dsh report none".into()),
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
    assert_eq!(wire["view_version"], json!(28));
    assert_eq!(wire["recipes"][1]["on_frontier"], Value::Null);
    assert_eq!(
        wire["recipes"][1]["cost_per_run"]["cost"],
        json!({"kind": "not_reported"})
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
