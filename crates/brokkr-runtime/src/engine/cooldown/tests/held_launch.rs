//! Fix round 0106b1r15, R1: a retry is admitted over every route it will
//! launch, read from its launch's own selection, never from the probes'
//! answers. A panel member that succeeded is launched again by the whole
//! panel's retry, though no wait refused it, so a hold already standing on
//! its route keeps the retry waiting. A retry whose start was admitted
//! before a hold existed launches.

use brokkr_store::{ProbeHoldId, Store};
use time::Duration as TimeDuration;

use super::held_charge::{hold_route, sleeping};
use super::held_start::retried_panel_engine;
use super::panel::{
    b_only_panel, plain_engine, shared_log, two_candidate_panel, two_route_drivers,
};
use super::*;
use crate::bundle::{McpIntent, SeatClass, SiteFacts};
use crate::capabilities::manifest::{ModelPins, Served};
use crate::capabilities::{NativePlan, Outcome, SiteAsks, SiteCapabilities};
use crate::engine::fence::tests::{after_the_start, before_the_start};

/// The two routes every panel case here launches on.
const ROUTES: [(&str, &str); 2] = [("claude", "first"), ("codex", "second")];

/// Run `peer` at the retry's start, the second this thread journals,
/// before its `effect/started`: the first attempt's start passes.
fn before_the_retry_start(peer: Box<dyn FnOnce()>) {
    before_the_start(Box::new(move || before_the_start(peer)));
}

/// Run `peer` once the retry's `effect/started` is admitted and before its
/// launch.
fn after_the_retry_start(peer: Box<dyn FnOnce()>) {
    before_the_retry_start(Box::new(move || after_the_start(peer)));
}

/// The mixed panel: `a` on claude/first succeeds every attempt, `b` on
/// codex/second refuses once and then succeeds; `a`'s site carries the pin
/// its compile seals, claude/first, whether it runs an agent or inline.
fn mixed_panel(dir: &Path, inline: bool) -> (Engine, Arc<Mutex<OffsetDateTime>>) {
    let (a, b) = two_route_drivers(
        dir,
        &["succeed", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
    );
    let pinned = candidate("claude", "claude/first", a.clone());
    let body = if inline {
        b_only_panel(dir, a)
    } else {
        two_candidate_panel(a, b)
    };
    let mut engine = plain_engine(dir, body, 2);
    let served = Served {
        id: "claude/first".into(),
        route: Some("first".into()),
    };
    let facts = SiteFacts {
        chain: if inline { Vec::new() } else { vec![pinned] },
        capabilities: inline.then(inline_capabilities),
        local: inline.then(Default::default),
        inline_dialect: inline.then(Default::default),
        inline_mcp: inline.then_some(McpIntent::Empty),
        pins: vec![ModelPins::Read(vec![served])],
        ..SiteFacts::default()
    };
    engine.bundle.sites.insert("work:a".into(), facts);
    with_clock(engine)
}

fn inline_capabilities() -> SiteCapabilities {
    let asks = SiteAsks::at(SeatClass::Work, "work:a", None, None).unwrap();
    let outcome = Outcome {
        provider: "claude".into(),
        harness: "claude".into(),
        model: None,
        held: Default::default(),
        not_held: Default::default(),
        notices: Vec::new(),
        native: NativePlan::Unmeasured {
            declaration: None,
            reason: "test driver".into(),
            provenance: Default::default(),
        },
    };
    SiteCapabilities::unjudged(asks, vec![outcome])
}

/// Only `b` refused and only `b` was probed, but the whole panel retries,
/// `a` with it. A peer's hold on `a`'s route, committed before the retry's
/// start, is read by that start: the retry is refused, waits on `a`'s
/// route, and launches `a` only once the peer's hold has ended, probing
/// `b` again first. Removal control: admit the retry over the routes its
/// probes answered, `b`'s alone, as r14 did, and `a` is launched again
/// while the peer holds its route, the panel never waiting.
#[test]
fn a_mixed_panels_retry_waits_on_the_route_of_the_member_no_wait_refused() {
    mixed_panel_retry_waits_for_the_peer(false);
}

/// A successful inline member is absent from the agent candidate map.
/// Its compiled route still needs to be clear before the panel retries.
#[test]
fn a_mixed_panels_retry_waits_on_the_route_of_the_inline_member() {
    mixed_panel_retry_waits_for_the_peer(true);
}

fn mixed_panel_retry_waits_for_the_peer(inline: bool) {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, now) = mixed_panel(dir.path(), inline);
    let (journal, run) = (engine.store.path().to_path_buf(), engine.run_id.clone());
    let held: Arc<Mutex<Option<ProbeHoldId>>> = Arc::default();
    let released: Arc<Mutex<Option<Vec<String>>>> = Arc::default();
    let (holding, ended, log) = (
        Arc::clone(&held),
        Arc::clone(&released),
        dir.path().to_owned(),
    );
    let peer = Mutex::new(Store::open(&journal).unwrap());
    let (clock, _) = sleeping(&now, TimeDuration::seconds(1), move |_, at| {
        if let Some(hold) = holding.lock().unwrap().take() {
            *ended.lock().unwrap() = Some(logs(&log, "a"));
            peer.lock().unwrap().end_probe_hold(Some(hold), at).unwrap();
        }
    });
    engine.cooldown.clock = clock;
    let (holding, read) = (Arc::clone(&held), Arc::clone(&now));
    before_the_retry_start(Box::new(move || {
        let at = *read.lock().unwrap();
        let hold = hold_route(&journal, (&run, "peer-on-a"), ROUTES[0], at);
        *holding.lock().unwrap() = Some(hold);
    }));
    engine.drive().unwrap();
    assert_eq!(
        released.lock().unwrap().as_deref(),
        Some(&["attempt:succeed".to_string()][..]),
        "the retry launched a while the peer held its route, or never waited"
    );
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:succeed", "attempt:succeed"]
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "probe", "attempt:succeed"]
    );
}

/// Once `engine`'s retry's start is admitted and before it launches, a
/// peer holds each of `routes` at the instant on `now`: the count of holds
/// written.
fn holding_after_the_retry_start(
    engine: &Engine,
    now: Arc<Mutex<OffsetDateTime>>,
    routes: &'static [(&'static str, &'static str)],
) -> Arc<Mutex<usize>> {
    let (journal, run) = (engine.store.path().to_path_buf(), engine.run_id.clone());
    let held = Arc::new(Mutex::new(0));
    let holding = Arc::clone(&held);
    after_the_retry_start(Box::new(move || {
        let at = *now.lock().unwrap();
        for (route, attempt) in routes.iter().zip(["after-start-a", "after-start-b"]) {
            hold_route(&journal, (&run, attempt), *route, at);
            *holding.lock().unwrap() += 1;
        }
    }));
    held
}

/// The retry's `effect/started` commits, then a peer's hold commits on its
/// route before the driver launches: the retry was admitted before the
/// hold existed, and launches. Removal control: veto the admitted start
/// again on a read of its routes after the commit, and the retry never
/// launches.
#[test]
fn a_retry_admitted_before_a_hold_on_its_route_launches() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    let held = holding_after_the_retry_start(&engine, now, &ROUTES[..1]);
    engine.drive().unwrap();
    assert_eq!(
        *held.lock().unwrap(),
        1,
        "no hold was taken after the start"
    );
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
}

/// A two-route panel both of whose members refused: its retry's start
/// commits, then a peer holds both routes before the members launch. The
/// retry was admitted before either hold, and both members launch.
/// Removal control: veto the admitted start again on a read of its routes
/// after the commit, and neither member launches again.
#[test]
fn a_panels_retry_admitted_before_holds_on_both_its_routes_launches() {
    let dir = tempfile::tempdir().unwrap();
    let (mut engine, now) = retried_panel_engine(dir.path());
    let held = holding_after_the_retry_start(&engine, now, &ROUTES);
    engine.drive().unwrap();
    assert_eq!(
        *held.lock().unwrap(),
        2,
        "no hold was taken after the start"
    );
    assert_eq!(
        shared_log(dir.path()),
        [
            "a:attempt:refuse",
            "b:attempt:refuse",
            "a:probe",
            "b:probe",
            "a:attempt:succeed",
            "b:attempt:succeed",
        ]
    );
}
