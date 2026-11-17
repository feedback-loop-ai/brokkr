//! A figure's facts in their full key's order across hearths (#1068's F5):
//! the cut key a figure groups by, then the hearth, then the run, whichever
//! source yields them.

use brokkr_core::fold::Status;
use brokkr_core::EventType;
use serde_json::json;

use super::super::{windows, AnalyticsHearth, Range};
use super::*;
use crate::fleet::tests::ruled;
use crate::tests::{ev, state};

const NOW: &str = "2026-10-13T12:00:00Z";

/// One run `run_id` of the shared realm and recipe, parked at review and
/// stopped by the policy's hard stop, its one seat reporting a cost: every
/// figure's key ties with another hearth's copy of it.
fn tied_run(run_id: &str) -> Vec<EventEnvelope> {
    let manifest = json!({
        "realms": {"source": "/w/realms.json", "realm": "shared",
                   "map": {"realms": [{"name": "shared", "path": "shared"}]}},
        "bundle_name": "tied-recipe",
    });
    let steps = [
        (
            EventType::RunStarted,
            json!({"feature": "f", "manifest": manifest}),
        ),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (EventType::RunParked, json!({})),
        (EventType::OperatorCommanded, json!({"command": "retry"})),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a",
                   "provenance": [{"agent": "a", "model": "m", "provider": "claude"}]}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "e", "attempt_id": "a",
                   "checkpoint": {"step": "claude-session-finished", "model": "m",
                                  "total_cost_usd": 0.5}}),
        ),
        (EventType::RunStopped, json!({})),
    ];
    steps
        .into_iter()
        .enumerate()
        .map(|(index, (event_type, payload))| {
            let mut event = ev(
                index as u64 + 1,
                event_type,
                payload,
                "2026-10-13T10:00:00Z",
            );
            event.run_id = run_id.to_string();
            event
        })
        .collect()
}

/// The whole read of one hearth `label` holding `runs`, for the Day window.
fn whole(label: &str, runs: &[(Vec<EventEnvelope>, RunState)]) -> WholeRead {
    let runs: Vec<AnalyticsRun<'_>> = runs
        .iter()
        .map(|(events, state)| AnalyticsRun {
            events,
            state: Some(state),
        })
        .collect();
    let hearth = AnalyticsHearth {
        label,
        realms: &[],
        journal: "",
        runs: &runs,
        refusal: None,
    };
    let query = Query {
        hearths: std::slice::from_ref(&hearth),
        range: Range::Day,
        realm: None,
        now: NOW,
    };
    let (window, _) = windows(&query).unwrap();
    WholeRead::read(&query, window)
}

/// Every fact a pulled stream yields, in its order.
fn pulled<T>(pull: impl FnOnce(&mut dyn FnMut(&mut dyn Iterator<Item = T>))) -> Vec<T> {
    let mut facts = Vec::new();
    pull(&mut |stream| facts.extend(stream));
    facts
}

/// Two hearths whose runs, parks and sites tie on every figure's cut key,
/// handed to the union in the reverse of their order: each stream yields
/// them by hearth and then run, never by the order the sources came in.
#[test]
fn facts_that_tie_on_a_key_across_hearths_stand_by_hearth_and_run() {
    let stopped = || {
        state(
            Some("stop"),
            Status::Stopped,
            ruled("TIED-RULE", "review", Some("stop")),
        )
    };
    let alpha = whole(
        "alpha",
        &[(tied_run("r2"), stopped()), (tied_run("r1"), stopped())],
    );
    let beta = whole("beta", &[(tied_run("r1"), stopped())]);
    let union = FactUnion::new(vec![Box::new(beta), Box::new(alpha)]);
    let expected = [("alpha", "r1"), ("alpha", "r2"), ("beta", "r1")];
    let identity = |hearth: &str, run: &str| (hearth.to_string(), run.to_string());
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(hearth, run)| identity(hearth, run))
        .collect();
    for order in [RunOrder::Rule, RunOrder::Recipe] {
        let runs = pulled(|read| union.runs_in(order, read));
        let order_of: Vec<_> = runs
            .iter()
            .map(|run| identity(&run.hearth, &run.run))
            .collect();
        assert_eq!(order_of, expected, "runs by {order:?}");
    }
    let parks = pulled(|read| union.parks(read));
    let order_of: Vec<_> = parks
        .iter()
        .map(|park| identity(&park.hearth, &park.run))
        .collect();
    assert_eq!(order_of, expected, "parks");
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        let sites = pulled(|read| union.sites_in(order, read));
        let order_of: Vec<_> = sites
            .iter()
            .map(|site| identity(&site.hearth, &site.run))
            .collect();
        assert_eq!(order_of, expected, "sites by {order:?}");
    }
}
