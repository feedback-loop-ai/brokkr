//! The analytics readout (#568), metric by metric, on one builder-made
//! fleet of three hearths: `alpha` and `beta` load, `gamma` does not.
//! Each run's journal is built for its times, parks, rulings and seats;
//! its folded state, from the shared state builder, for how it ended.

use super::words::{change_text, listed, span_text, stamp};
use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};
use crate::{js, ABSENT};
use brokkr_core::fold::Status;
use brokkr_core::EventType;
use serde_json::{json, Value};

const NOW: &str = "2026-10-13T12:00:00Z";

/// One run's journal, event by event.
struct Journal {
    events: Vec<EventEnvelope>,
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

    fn then(mut self, event_type: EventType, payload: Value, at: &str) -> Journal {
        let seq = self.events.len() as u64 + 1;
        self.events.push(ev(seq, event_type, payload, at));
        self
    }

    fn entered(self, phase: &str, at: &str) -> Journal {
        self.then(EventType::PhaseEntered, json!({"phase": phase}), at)
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
                rule_id: "REVIEW-FAIL-TWICE".to_string(),
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
                model: "claude-opus".into(),
                attempts: 1,
                percent: 50,
                text: "claude-opus 50%".into()
            },
            ModelShare {
                model: "gpt-5".into(),
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
    let at = stamp(js::parse_millis(NOW).unwrap() - ago);
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
