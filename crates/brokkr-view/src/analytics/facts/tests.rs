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
    let mut union = FactUnion::new(vec![Box::new(beta), Box::new(alpha)]);
    let expected = [("alpha", "r1"), ("alpha", "r2"), ("beta", "r1")];
    let identity = |hearth: &str, run: &str| (hearth.to_string(), run.to_string());
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(hearth, run)| identity(hearth, run))
        .collect();
    for order in [RunOrder::Rule, RunOrder::Recipe] {
        let runs: Vec<_> = union.runs_in(order).collect();
        let order_of: Vec<_> = runs
            .iter()
            .map(|run| identity(&run.hearth, &run.run))
            .collect();
        assert_eq!(order_of, expected, "runs by {order:?}");
    }
    let parks: Vec<_> = union.parks().collect();
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
        let sites: Vec<_> = union.sites_in(order).collect();
        let order_of: Vec<_> = sites
            .iter()
            .map(|site| identity(&site.hearth, &site.run))
            .collect();
        assert_eq!(order_of, expected, "sites by {order:?}");
    }
}

/// A hearth that holds one run, its one park and its one site, or none.
struct OneFact {
    facts: Option<(RunFacts, ParkFact, SiteFact)>,
}

impl FactSource for OneFact {
    fn runs(&self, _: &mut dyn FnMut(RunFacts)) {}
    fn runs_in(&mut self, _: RunOrder) -> Pulled<'_, RunFacts> {
        Box::new(self.facts.iter().map(|(run, _, _)| run.clone()))
    }
    fn durations(&self, _: &mut dyn FnMut(&str, i64)) {}
    fn parks(&mut self) -> Pulled<'_, ParkFact> {
        Box::new(self.facts.iter().map(|(_, park, _)| park.clone()))
    }
    fn spends(&self, _: &mut dyn FnMut(SpendFact)) {}
    fn sites_in(&mut self, _: SiteOrder) -> Pulled<'_, SiteFact> {
        Box::new(self.facts.iter().map(|(_, _, site)| site.clone()))
    }
}

/// How many hearths the union merges at once in the cardinality test:
/// past the 8,192 at which a merge that nested a frame a source overflowed
/// its stack.
const HEARTHS: usize = 16_384;

/// A union of [`HEARTHS`] sources returns for every merged figure, holding
/// no frame a source: each empty, nothing; each with one fact tying on the
/// figure's key, every fact, by hearth.
#[test]
fn a_union_of_sixteen_thousand_hearths_merges_every_figure() {
    let stopped = state(
        Some("stop"),
        Status::Stopped,
        ruled("TIED-RULE", "review", Some("stop")),
    );
    let mut tied = whole("tied", &[(tied_run("r1"), stopped)]);
    let run = tied.runs_in(RunOrder::Recipe).next().unwrap();
    let park = tied.parks().next().unwrap();
    let site = tied.sites_in(SiteOrder::Model).next().unwrap();
    let hearth = |index: usize| format!("h{index:05}");
    let union = |facts: &dyn Fn(usize) -> Option<(RunFacts, ParkFact, SiteFact)>| {
        // Handed in the reverse of the hearths' order, so every fact is
        // placed by the merge.
        let sources = (0..HEARTHS)
            .rev()
            .map(|index| {
                Box::new(OneFact {
                    facts: facts(index),
                }) as Box<dyn FactSource>
            })
            .collect();
        FactUnion::new(sources)
    };
    let mut empty = union(&|_| None);
    for order in [RunOrder::Rule, RunOrder::Recipe] {
        assert_eq!(empty.runs_in(order).count(), 0, "runs by {order:?}");
    }
    assert_eq!(empty.parks().count(), 0, "parks");
    assert_eq!(empty.sites_in(SiteOrder::Model).count(), 0, "sites");
    let mut full = union(&|index| {
        let (mut run, mut park, mut site) = (run.clone(), park.clone(), site.clone());
        (run.hearth, park.hearth, site.hearth) = (hearth(index), hearth(index), hearth(index));
        Some((run, park, site))
    });
    let expected: Vec<String> = (0..HEARTHS).map(hearth).collect();
    for order in [RunOrder::Rule, RunOrder::Recipe] {
        let hearths: Vec<String> = full.runs_in(order).map(|run| run.hearth).collect();
        assert!(hearths == expected, "runs by {order:?} out of hearth order");
    }
    let hearths: Vec<String> = full.parks().map(|park| park.hearth).collect();
    assert!(hearths == expected, "parks out of hearth order");
    for order in [SiteOrder::Model, SiteOrder::RecipeProvider] {
        let hearths: Vec<String> = full.sites_in(order).map(|site| site.hearth).collect();
        assert!(
            hearths == expected,
            "sites by {order:?} out of hearth order"
        );
    }
}
