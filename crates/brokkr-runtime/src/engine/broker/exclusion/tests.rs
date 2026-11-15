//! Decision 0108 ruling 3's worktree release: a quarantined run stops
//! holding its worktree once its run lock is free and every attempt its own
//! reservation names is proven ended by the broker's records and the host,
//! never by the run's fold. The proof's own unit tests live beside the
//! verb; these are the admission's, over the comparison that refuses.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Run};
use serde_json::json;

use crate::engine::broker::admission::tests::cr2;
use crate::engine::broker::admission::tests::{
    acknowledge, boxed_claim, child_lock, dead_group, isolated, quarantined, quarantined_drivers,
    reservation_path, unacknowledged, Fixture, LiveGroup,
};
use crate::engine::broker::admission::{Conflict, Unadmitted, Unknown, Witnessed};
use crate::engine::broker::settlement::release::{self, Live, UnprovenWhy};
use crate::engine::broker::settlement::Unreleased;
use crate::engine::operator::{operator_repair, operator_settle, Repair, Settle};
use crate::engine::{EngineError, SettleRefusal};

/// Rewrite the reservation of `fixture`'s run through `edit`, as a build
/// that wrote another shape would have left it.
fn rewrite_reservation(fixture: &Fixture, edit: impl FnOnce(&mut serde_json::Value)) {
    let at = crate::engine::broker::admission::tests::reservation_path(
        fixture.host.home(),
        &fixture.work,
        "run-a",
    );
    let mut reservation: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    edit(&mut reservation);
    std::fs::write(&at, serde_json::to_vec(&reservation).unwrap()).unwrap();
}

/// A claim of the fixture's worktree that is not run A's: what a later
/// admission compares.
fn next(fixture: &Fixture) -> Result<crate::engine::broker::admission::Held, Unadmitted> {
    fixture
        .host
        .admit(&boxed_claim(&fixture.work, ["run-b", "b1"]))
}

/// The settlement refusal `refused` carries, so a test compares its typed
/// cause rather than its text.
fn unreleased(refused: EngineError) -> Unreleased {
    match refused {
        EngineError::Unreleased(unreleased) => unreleased,
        other => panic!("not a settlement refusal: {other:?}"),
    }
}

/// `operator_settle` on `fixture`, refused for `reason`: the call every
/// refusal test makes.
fn settle_refused(fixture: &mut Fixture, reason: &str) -> EngineError {
    operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason,
            operator: "operator",
        },
    )
    .unwrap_err()
}

/// Assert `refused` is the live process group `group` of `attempt`, the
/// typed fact, not its text.
fn live_group_refused(refused: &EngineError, attempt: &str, group: u32) {
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Group {
                attempt: ref found,
                group: found_group,
            })) if found == attempt && *found_group == group
        ),
        "the live group is named by its type: {refused:?}"
    );
}

/// Assert `refused` is the group-less refusal of `attempt`.
fn no_group_refused(refused: &EngineError, attempt: &str) {
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Unproven {
                attempt: ref found,
                why: UnprovenWhy::NoGroup,
            })) if found == attempt
        ),
        "the group-less attempt refuses by its type: {refused:?}"
    );
}

/// Settle `fixture` refused with `group` still live, writing nothing, and
/// with the worktree still held: the assertion every recorded live group
/// shares.
fn refused_live(fixture: &mut Fixture, group: u32) {
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = settle_refused(fixture, "both are done");
    live_group_refused(&refused, "a1", group);
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "the refusal wrote nothing"
    );
    assert_eq!(
        next(fixture),
        cr2(Conflict::Held("run-a".into())),
        "the live group keeps the worktree held"
    );
}

/// A fresh host, its worktree and journal, and a shape (B) run `run-a`
/// already journaled there: what both old-run route tests start from.
fn old_run() -> (
    crate::engine::broker::admission::tests::Host,
    std::path::PathBuf,
    std::path::PathBuf,
    brokkr_store::Store,
) {
    let host = crate::engine::broker::admission::tests::Host::new();
    let work = host.worktree("work");
    let db = host.journal("run-a.db");
    let store = old_shape_b_journal(&db, "run-a");
    (host, work, db, store)
}

/// Acknowledge `store`'s old run and stand in its host, with the worktree
/// held: the state every old-run route test takes its proof from. The
/// returned guard must outlive the test's later admissions.
fn old_run_holds(
    host: &crate::engine::broker::admission::tests::Host,
    work: &std::path::Path,
    store: &mut brokkr_store::Store,
) -> crate::engine::broker::admission::tests::Coordinated {
    acknowledge(store, "run-a");
    let stood = crate::engine::broker::admission::tests::Coordinated::at(host.home());
    assert_eq!(
        host.admit(&boxed_claim(work, ["run-b", "b1"])),
        cr2(Conflict::Held("run-a".into())),
        "the old run holds the worktree"
    );
    stood
}

/// The binding: an acknowledged, unfoldable run whose driver ended settled
/// releases its worktree, and a new claim in that worktree is admitted. The
/// proof reads the broker's record, not the run's fold, which still fails.
#[test]
fn a_quarantined_run_whose_writers_are_settled_releases_its_worktree() {
    let mut fixture = unacknowledged(&[Witnessed::Settled], None);
    assert_eq!(
        fold(&fixture.store.load("run-a").unwrap()).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 7 },
        "shape (B) does not fold"
    );
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "an unacknowledged run holds the worktree, its writers proven or not"
    );
    acknowledge(&mut fixture.store, "run-a");
    assert!(
        next(&fixture).is_ok(),
        "the acknowledged run's settled attempt releases the worktree"
    );
}

/// The binding's other half: one attempt's process group still alive keeps
/// the worktree held, with today's words. Its driver is not sealed Settled,
/// and the host says a member of the group still runs.
#[test]
fn a_quarantined_run_with_a_live_process_group_still_holds_its_worktree() {
    if !isolated("engine::broker::exclusion::tests::a_quarantined_run_with_a_live_process_group_still_holds_its_worktree") {
        return;
    }
    let live = LiveGroup::new();
    let fixture = quarantined(&[Witnessed::Settling], Some(live.group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the host still shows the attempt's process group, so it holds"
    );
    drop(live);
}

/// A group id recorded for a driver sealed Settled is not a writer: the
/// seal is the settlement proof, and the kernel may have reused the id for
/// an unrelated process. Asking it would keep the worktree held with no
/// route (F9's correction; M-D).
#[test]
fn a_settled_drivers_reused_group_id_does_not_hold_the_worktree() {
    if !isolated("engine::broker::exclusion::tests::a_settled_drivers_reused_group_id_does_not_hold_the_worktree") {
        return;
    }
    let live = LiveGroup::new();
    let (fixture, held) = quarantined_drivers(&[("m1", Witnessed::Settled)]);
    release::spawned(&held, "m1", live.group());
    assert!(
        next(&fixture).is_ok(),
        "a settled driver's group is not asked, so a reused id does not hold"
    );
    drop(live);
}

/// A child process that holds the run's lock keeps the worktree held, and
/// the refusal names the child by its typed cause (decision 0108 ruling 1):
/// the kernel excludes across processes, not only across open descriptions.
/// The holder's sentence is pinned once, exactly, in
/// `run_lock_tests::a_held_run_refuses_resume_naming_its_driver` (R1;
/// decision 0071 ruling 8).
#[test]
fn a_quarantined_run_whose_lock_a_child_holds_is_refused_naming_it() {
    let fixture = quarantined(&[Witnessed::Settled], None);
    let held = child_lock(&fixture.store, "run-a");
    let refused = next(&fixture).unwrap_err();
    let Unadmitted::Evidence(Conflict::Driven {
        run,
        holder: Some(holder),
    }) = refused
    else {
        panic!("a child's lock is refused by its type: {refused:?}");
    };
    assert_eq!(run, "run-a");
    assert_eq!(holder.pid, held.holder());
    drop(held);
    assert!(
        next(&fixture).is_ok(),
        "once the child lets go, the settled run releases"
    );
}

/// Acknowledgement alone never releases: a run with an attempt no record
/// proves keeps its worktree however it is looked at.
#[test]
fn an_acknowledged_run_whose_record_proves_nothing_still_holds() {
    let fixture = quarantined(&[Witnessed::Settling], None);
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "an unsettled driver with no group record keeps the worktree held"
    );
}

/// The same run acknowledged and then repaired (440c): the repair record
/// stands for the acknowledgement a repair lapses (#511), so the settled
/// attempt releases the worktree and the run folds again.
#[test]
fn a_repaired_run_whose_writers_are_settled_releases_its_worktree() {
    let mut fixture = quarantined(&[Witnessed::Settled], None);
    assert!(
        fixture.store.acknowledgement("run-a").unwrap().is_some(),
        "the run is acknowledged before the repair lapses it"
    );
    operator_repair(
        &mut fixture.store,
        "run-a",
        &Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    let state = fold(&fixture.store.load("run-a").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped, "the repaired run folds again");
    assert_eq!(
        fixture.store.acknowledgement("run-a").unwrap(),
        None,
        "the repair moved the head past the acknowledgement"
    );
    assert!(
        next(&fixture).is_ok(),
        "the repaired run's settled attempt releases the worktree"
    );
}

/// Where the records prove nothing, the operator's settle record is the
/// proof the exclusion honours: a run whose only driver is not sealed
/// Settled, and whose recorded process group is gone from the host, is
/// released once `brokkr operator settle` names it. This is 440c's F5 test
/// rewritten to the new truth.
#[test]
fn a_settle_record_releases_what_the_records_cannot_prove() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the broker's own records do not prove a Settling driver ended"
    );
    let written = operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "the group is gone",
            operator: "operator",
        },
    )
    .expect("the operator proves the group gone");
    assert_eq!(written.payload["command"], "settle");
    assert!(
        next(&fixture).is_ok(),
        "the settle record is the proof the exclusion honours"
    );
}

/// A settle record that names an attempt the host still shows alive does
/// not release, even with the lock free.
#[test]
fn a_settle_record_naming_a_live_group_does_not_release() {
    use crate::engine::broker::settlement::release::{Proof, Proven, SettleArgs};
    if !isolated(
        "engine::broker::exclusion::tests::a_settle_record_naming_a_live_group_does_not_release",
    ) {
        return;
    }
    let live = LiveGroup::new();
    let mut fixture = quarantined(&[Witnessed::Settling], Some(live.group()));
    // A record written when the group was still live: it names the attempt
    // and its group, and the exclusion asks the host again. Its proof is
    // `group`, so only `living` refuses it.
    let args = SettleArgs {
        run: "run-a".into(),
        attempts: vec![Proven {
            attempt: "a1".into(),
            proof: Proof::Group,
            groups: vec![live.group()],
        }],
        reason: "the group was gone when I looked".into(),
    };
    let payload = json!({
        "command_id": "c-settle",
        "command": brokkr_core::fold::SETTLE,
        "operator": "operator",
        "args": args,
    });
    let (seq, hash) = fixture.store.head_hash("run-a").unwrap();
    fixture
        .store
        .append_next_if_head(
            "run-a",
            seq,
            &hash,
            EventType::OperatorCommanded,
            payload,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the record names a group the host still shows alive"
    );
    drop(live);
}

/// A journal any participant's writer could reach proves nothing, whatever
/// it says: an unkeyed chain a writer can forge is no proof (R2). The
/// release refuses rather than trust it.
#[test]
fn a_journal_a_writer_could_reach_proves_nothing() {
    let fixture = quarantined(&[Witnessed::Settled], None);
    let inside = fixture.work.join("forge.db");
    std::fs::copy(&fixture.db, &inside).unwrap();
    rewrite_reservation(&fixture, |reservation| {
        reservation["journal"] = serde_json::json!(inside);
    });
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "a reachable journal is no proof"
    );
}

/// A released run's later commands still refuse: the release frees the
/// worktree, not the run. `retry` and `stop` meet the terminal gate before
/// they write, exactly; `resume` on a released run is proved by
/// [`a_released_run_resumes_to_its_fold`].
#[test]
fn a_released_run_still_refuses_retry_and_stop() {
    use crate::engine::operator::{operator_stop, Cause, Named};
    use crate::engine::EngineError;
    use brokkr_core::fold::OperatorCommand;
    let mut fixture = quarantined(&[Witnessed::Settled], None);
    operator_repair(
        &mut fixture.store,
        "run-a",
        &Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    // The fixture's planted tail leaves its summary behind its head; this
    // test reads the terminal gate, not the summary.
    brokkr_store::test_support::lags(&mut fixture.store, &["run-a"]);
    assert_eq!(
        fixture.store.acknowledgement("run-a").unwrap(),
        None,
        "the repair lapsed the acknowledgement it followed"
    );
    assert!(next(&fixture).is_ok(), "the worktree is released");
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = crate::engine::operator_command(
        &mut fixture.store,
        "run-a",
        OperatorCommand::Retry,
        "operator",
        "again",
    );
    assert!(
        matches!(
            refused,
            Err(EngineError::AlreadyConcluded {
                verb: OperatorCommand::Retry,
                ..
            })
        ),
        "retry: {refused:?}"
    );
    let cited = brokkr_store::Store::open_read_only(fixture.store.path()).unwrap();
    let named = Named {
        cause: Cause {
            ending: brokkr_core::ending::Ending::Withdrawn,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "again",
        operator: "operator",
    };
    let refused = operator_stop(&mut fixture.store, &cited, "run-a", &named);
    assert!(
        matches!(
            refused,
            Err(EngineError::AlreadyConcluded {
                verb: OperatorCommand::Stop,
                ..
            })
        ),
        "stop: {refused:?}"
    );
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "neither command wrote anything"
    );
}

/// A released run resumes to its fold: the release frees the worktree, not
/// the run (S4). The run is shape (B), repaired, its settled attempt
/// released by the broker's own records; `Engine::resume` takes it again and
/// drives it to the status its terminal event states. The run's own start
/// record carries the empty charter record this test bundle pins.
#[test]
fn a_released_run_resumes_to_its_fold() {
    use crate::engine::broker::admission::tests::{planted_drivers, Coordinated, Host};
    use crate::engine::tests::{bundle, single_body};
    use brokkr_core::fold::Run;

    let host = Host::new();
    let work = host.worktree("work");
    let db = host.journal("run-a.db");
    let mut store = crate::engine::tests::in_flight_store_started(
        &db,
        "run-a",
        json!({"feature": "feature", "manifest": {"files": {}}, "charters": []}),
    );
    store
        .append_next(
            "run-a",
            EventType::EffectIndeterminate,
            json!({"effect_id": "effect", "attempt_id": "attempt"}),
            None,
            Some("attempt".into()),
        )
        .unwrap();
    store
        .append_next(
            "run-a",
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    drop(store);
    let mut store = crate::engine::broker::admission::tests::broken(&db, "run-a");
    planted_drivers(
        &host,
        &work,
        ["run-a", "attempt"],
        &[("work", Witnessed::Settled, None)],
        None,
        &db,
    );
    acknowledge(&mut store, "run-a");
    let _stood = Coordinated::at(host.home());
    operator_repair(
        &mut store,
        "run-a",
        &Repair {
            operator: "operator",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    assert!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])).is_ok(),
        "the settled attempt's records release the worktree"
    );
    let bundle_dir = tempfile::tempdir().unwrap();
    let mut bundle = bundle(
        bundle_dir.path(),
        single_body(vec!["missing-driver".into()]),
    );
    bundle.manifest = json!({"files": {}});
    let mut resumed = crate::engine::Engine::resume(
        brokkr_store::Store::open(&db).unwrap(),
        bundle,
        "run-a",
        Some(work),
    )
    .expect("the released run resumes");
    let end = resumed.drive().unwrap();
    assert_eq!(end.state.run, Run::Stopped, "resume meets the run's fold");
}

/// A panel attempt has one driver per member, each leading its own process
/// group. A settle record must prove *every* one: with only one member's
/// group gone, the other member's own group keeps the worktree held, and
/// once both are gone the same settle record releases it.
#[test]
fn a_panel_attempt_with_one_live_group_still_holds_its_worktree() {
    if !isolated(
        "engine::broker::exclusion::tests::a_panel_attempt_with_one_live_group_still_holds_its_worktree",
    ) {
        return;
    }
    let mut g1 = LiveGroup::new();
    let mut g2 = LiveGroup::new();
    let (mut fixture, held) =
        quarantined_drivers(&[("m1", Witnessed::Settling), ("m2", Witnessed::Settling)]);
    release::spawned(&held, "m1", g1.group());
    release::spawned(&held, "m2", g2.group());
    // The second member's group only is gone.
    g2.end();
    refused_live(&mut fixture, g1.group());
    // Both are gone now: the same proof releases.
    g1.end();
    operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "both are done",
            operator: "operator",
        },
    )
    .expect("every driver's own group is gone");
    assert!(
        next(&fixture).is_ok(),
        "both groups gone admits the next claim"
    );
}

/// A same-site spawn sequence (a cool-down probe, then the seat's driver)
/// writes each group on its own driver's record, never on the first one
/// again: with the later driver sealed and its group gone, the earlier
/// driver's live group still refuses. Reading the first record of the site
/// would overwrite it and prove the wrong driver (M-C).
#[test]
fn a_same_site_spawn_records_each_drivers_own_group() {
    if !isolated(
        "engine::broker::exclusion::tests::a_same_site_spawn_records_each_drivers_own_group",
    ) {
        return;
    }
    use brokkr_protocol::Cleanup;
    let g1 = LiveGroup::new();
    let mut g2 = LiveGroup::new();
    let (mut fixture, held) = quarantined_drivers(&[("work", Witnessed::Settling)]);
    release::spawned(&held, "work", g1.group());
    crate::engine::broker::settlement::spawning(&held, "work", None)
        .expect("the second driver at the same site starts");
    release::spawned(&held, "work", g2.group());
    let drivers = fixture.host.drivers(&fixture.work, "run-a");
    assert_eq!(drivers.len(), 2, "the two drivers are two records");
    assert_eq!(
        drivers[0].group,
        Some(g1.group()),
        "the first driver's own group"
    );
    assert_eq!(
        drivers[1].group,
        Some(g2.group()),
        "the second driver's own group"
    );
    // The later driver seals Settled and its group ends; the earlier one
    // still lives, so its own group is the one that decides.
    let mut held = Some(held);
    crate::engine::broker::settlement::ended(&mut held, "work", &Cleanup::Settled);
    g2.end();
    let refused = settle_refused(&mut fixture, "both are done");
    live_group_refused(&refused, "a1", g1.group());
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the live earlier driver keeps the worktree held"
    );
}

/// A spawn whose amend writes no group — the site has no driver record —
/// leaves the driver refusing: the spawn path never stands an earlier
/// group in for the one it failed to record (H1's third test; L-8).
#[test]
fn a_spawn_without_a_driver_record_leaves_it_refusing() {
    let (mut fixture, held) = quarantined_drivers(&[("work", Witnessed::Settling)]);
    release::spawned(&held, "no-such-site", dead_group());
    let drivers = fixture.host.drivers(&fixture.work, "run-a");
    assert_eq!(drivers.len(), 1, "the failed amend added no record");
    assert_eq!(
        drivers[0].group, None,
        "the failed amend wrote no group on an earlier driver"
    );
    let refused = settle_refused(&mut fixture, "the driver is gone");
    no_group_refused(&refused, "a1");
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "nothing released it"
    );
}

/// `operator release` reads the run's fold to journal its command: before
/// the repair a shape (B) run does not fold, so the command refuses and
/// writes nothing.
fn release_before_the_repair_refuses(store: &mut brokkr_store::Store) {
    use brokkr_core::fold::OperatorCommand;
    let head = store.head_hash("run-a").unwrap();
    let refused = crate::engine::operator_command(
        store,
        "run-a",
        OperatorCommand::Release,
        "operator",
        "every process the attempt owned has ended",
    )
    .unwrap_err();
    assert!(
        matches!(
            refused,
            crate::engine::EngineError::Fold(brokkr_core::FoldError::AfterTerminal { .. })
        ),
        "release before repair meets the unfoldable tail: {refused:?}"
    );
    assert_eq!(
        store.head_hash("run-a").unwrap(),
        head,
        "the refused release wrote nothing"
    );
}

/// The shape (B) journal 0.12.0 left for `run`: the attempt closed
/// indeterminate, a conclusion, and the stop an older binary journaled
/// after it, so the run does not fold until repaired.
fn old_shape_b_journal(db: &std::path::Path, run: &str) -> brokkr_store::Store {
    let mut store = crate::engine::tests::in_flight_store(db, run);
    store
        .append_next(
            run,
            EventType::EffectIndeterminate,
            json!({"effect_id": "effect", "attempt_id": "attempt"}),
            None,
            Some("attempt".into()),
        )
        .unwrap();
    store
        .append_next(
            run,
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    drop(store);
    crate::engine::broker::admission::tests::broken(db, run)
}

/// v0.13.0's reservation serialisation, in the tag's own field order and
/// optionality, with `{repo}`, `{workdir}`, `{artifacts}` and `{journal}`
/// replaced by JSON strings (S6). Field for field, from
/// `git show v0.13.0:crates/brokkr-runtime/src/engine/broker/admission.rs`,
/// `.../settlement.rs` and
/// `crates/brokkr-protocol/src/hands/sealed.rs`:
/// - `Reservation`: `repo` (:204), `run` (:205), `workdir` (:206),
///   `artifacts` (:207), `journal` (:208), `attempts` (:209), `settled`
///   (:210);
/// - `Reserved`: `protects` (:221), `writers` (:222), `cleanup` (:223),
///   `serves` (:225), `drivers` (:227); `serves` is empty and so skipped,
///   and the tag has no `group` field (:220-228);
/// - `Driver`: `site` (settlement.rs:363), `begun` (:364), `cleanup`
///   (:365); the tag has no `group` field there either;
/// - `Writer`: `site` (admission.rs:255), `reach` (:256);
/// - `Reach::Boxed` (:276): `writable` (:277), `readable` (:278), `sources`
///   (:279), `sealed` (:280);
/// - `Sealed`: `home` (sealed.rs:73), `worktree` (:74), `git` (:75),
///   `binds` (:76), `store` (:78), `store` null;
/// - `Source` (sealed.rs:93): `path` (:94), `identity` (:95), both `null`
///   or empty here; `Git` (:84) is null, so `dir` (:85) and `common` (:86)
///   are absent;
/// - `Store` (sealed/hidden/place.rs:21): `place` (:22), `spelling` (:23),
///   absent with the null store.
const V0_13_0_RESERVATION: &str = concat!(
    r#"{"repo":{repo},"run":"run-a","workdir":{workdir},"artifacts":{artifacts},"#,
    r#""journal":{journal},"attempts":{"attempt":{"protects":"nothing","writers":"#,
    r#"[{"site":"work","reach":{"boxed":{"writable":[{workdir}],"readable":[],"#,
    r#""sources":[{workdir}],"sealed":{"home":"","worktree":{"path":"","identity":null},"#,
    r#""git":null,"binds":[],"store":null}}}}],"cleanup":"settling","drivers":"#,
    r#"[{"site":"work","begun":null,"cleanup":"settling"}]}},"settled":[]}"#,
);

/// [`V0_13_0_RESERVATION`] with the test's own paths, each a JSON string.
fn v0_13_0_reservation(
    repo: &str,
    workdir: &std::path::Path,
    artifacts: &std::path::Path,
    journal: &std::path::Path,
) -> String {
    let quote = |value: &str| serde_json::to_string(value).unwrap();
    V0_13_0_RESERVATION
        .replace("{repo}", &quote(repo))
        .replace("{workdir}", &quote(&workdir.to_string_lossy()))
        .replace("{artifacts}", &quote(&artifacts.to_string_lossy()))
        .replace("{journal}", &quote(&journal.to_string_lossy()))
}

/// Assert the fields `keys` appear, each as a quoted name, in that order in
/// the object that begins at `start` in `fixture`.
fn assert_field_order(fixture: &str, start: &str, keys: &[&str]) {
    let mut at = fixture
        .find(start)
        .unwrap_or_else(|| panic!("{start} is missing"))
        + start.len();
    for key in keys {
        let quoted = format!("\"{key}\"");
        let found = fixture[at..]
            .find(&quoted)
            .unwrap_or_else(|| panic!("{quoted} is missing or out of order after {start}"));
        at += found + quoted.len();
    }
}

/// The object keys of `value`, as the fixture states them.
fn object_keys(value: &serde_json::Value) -> std::collections::BTreeSet<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

/// Assert the planted reservation is v0.13.0's: its top level and every
/// nested object in the tag's order and names, its attempt and driver
/// without a group, and the whole record reads as this build's `Reservation`
/// (S6). The order is read from the planted bytes, not from the constant
/// that wrote them, so an edit to either is caught where it stands.
fn assert_v0_13_0_reservation(at: &std::path::Path) {
    let fixture = std::fs::read_to_string(at).unwrap();
    assert_field_order(
        &fixture,
        "",
        &[
            "repo",
            "run",
            "workdir",
            "artifacts",
            "journal",
            "attempts",
            "settled",
        ],
    );
    assert_field_order(
        &fixture,
        r#""attempts":{"attempt":{"#,
        &["protects", "writers", "cleanup", "drivers"],
    );
    assert_field_order(&fixture, r#""writers":[{"#, &["site", "reach"]);
    assert_field_order(
        &fixture,
        r#""reach":{"boxed":{"#,
        &["writable", "readable", "sources", "sealed"],
    );
    assert_field_order(
        &fixture,
        r#""sealed":{"#,
        &["home", "worktree", "git", "binds", "store"],
    );
    assert_field_order(&fixture, r#""worktree":{"#, &["path", "identity"]);
    assert_field_order(&fixture, r#""drivers":[{"#, &["site", "begun", "cleanup"]);
    let planted: serde_json::Value = serde_json::from_slice(&std::fs::read(at).unwrap()).unwrap();
    let attempt = &planted["attempts"]["attempt"];
    assert!(
        attempt.get("group").is_none(),
        "v0.13.0's Reserved has no one-group slot (admission.rs:220 at the tag)"
    );
    assert_eq!(
        object_keys(attempt),
        ["cleanup", "drivers", "protects", "writers"]
            .into_iter()
            .collect(),
        "v0.13.0's Reserved names these four fields, and no `group` or `serves`"
    );
    assert_eq!(
        object_keys(&attempt["writers"][0]),
        ["reach", "site"].into_iter().collect(),
        "v0.13.0's Writer has site and reach (admission.rs:255 at the tag)"
    );
    assert_eq!(
        object_keys(&attempt["writers"][0]["reach"]["boxed"]),
        ["readable", "sealed", "sources", "writable"]
            .into_iter()
            .collect(),
        "v0.13.0's Reach::Boxed names these four fields (admission.rs:276 at the tag)"
    );
    assert_eq!(
        object_keys(&attempt["writers"][0]["reach"]["boxed"]["sealed"]),
        ["binds", "git", "home", "store", "worktree"]
            .into_iter()
            .collect(),
        "v0.13.0's Sealed names these five fields (sealed.rs:72 at the tag)"
    );
    assert_eq!(
        object_keys(&attempt["writers"][0]["reach"]["boxed"]["sealed"]["worktree"]),
        ["identity", "path"].into_iter().collect(),
        "v0.13.0's Source names path and identity (sealed.rs:93 at the tag)"
    );
    assert_eq!(
        object_keys(&attempt["drivers"][0]),
        ["begun", "cleanup", "site"].into_iter().collect(),
        "v0.13.0's Driver has site, begun and cleanup (settlement.rs:362 at the tag)"
    );
    assert_eq!(
        object_keys(&planted),
        [
            "repo",
            "run",
            "workdir",
            "artifacts",
            "journal",
            "attempts",
            "settled"
        ]
        .into_iter()
        .collect(),
        "the tag's top level names these seven fields and no other"
    );
    let _: crate::engine::broker::admission::Reservation =
        serde_json::from_value(planted).expect("v0.13.0's whole record reads");
}

/// The route for a reservation written before this build (v0.13.0's shape:
/// drivers, no process group): `operator settle` cannot prove it, and the
/// operator repairs the run and then uses the existing
/// `brokkr operator release` attestation. The 0.12.0-era journal is the
/// shape (B) one; the reservation is the tag's own bytes, not this build's
/// serialisation (S6). A build that reported 0.12.0 from main between #745
/// and #822 wrote an earlier shape, whose view or reachable journal this
/// test separately shows cannot release. The next claim is admitted only
/// after that release, and only where the attempt ended indeterminate.
#[test]
fn an_old_shape_b_reservation_is_released_by_repair_then_operator_release() {
    use brokkr_core::fold::OperatorCommand;

    let (host, work, db, mut store) = old_run();
    let mut claim = boxed_claim(&work, ["run-a", "attempt"]);
    claim.journal = db.clone();
    let (repo, artifacts) = (claim.repo.clone(), claim.artifacts.clone());
    // The directories the inventory reads are admission's; the reservation
    // planted in them is the tag's, byte for byte.
    let held = host
        .admit(&claim)
        .expect("the old reservation's directories are made");
    drop(held);
    let fixture = v0_13_0_reservation(&repo, &work, &artifacts, &db);
    let at = reservation_path(host.home(), &work, "run-a");
    std::fs::write(&at, fixture.as_bytes()).unwrap();
    assert_v0_13_0_reservation(&at);
    let _stood = old_run_holds(&host, &work, &mut store);
    let refused = operator_settle(
        &mut store,
        "run-a",
        &Settle {
            reason: "the driver is gone",
            operator: "operator",
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Unproven {
                ref attempt,
                why: UnprovenWhy::NoGroup,
            })) if attempt == "attempt"
        ),
        "settle cannot prove a reservation with no group: {refused:?}"
    );
    // `release` needs the repaired fold: before the repair the run does not
    // fold, and the operator's command refuses before it writes.
    release_before_the_repair_refuses(&mut store);
    operator_repair(
        &mut store,
        "run-a",
        &Repair {
            operator: "operator",
            reason: "0.12.0 left a stop after the conclusion",
        },
    )
    .expect("the repair names the late tail");
    crate::engine::operator_command(
        &mut store,
        "run-a",
        OperatorCommand::Release,
        "operator",
        "every process the attempt owned has ended",
    )
    .expect("the operator's release attests the old attempt ended");
    assert!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])).is_ok(),
        "the released old run no longer holds the worktree"
    );
}

/// An attempt whose cleanup was witnessed unresolved is never released by
/// `settle`, even with every recorded group gone: the witness may have seen
/// a stray the group question cannot. The worktree stays held, nothing is
/// written, and only the operator's own `release` attests it ended (S2;
/// decision 0108 ruling 3).
#[test]
fn an_unresolved_cleanup_holds_until_the_operator_releases_it() {
    use brokkr_core::fold::OperatorCommand;

    let (host, work, db, mut store) = old_run();
    let held = crate::engine::broker::admission::tests::planted_drivers(
        &host,
        &work,
        ["run-a", "attempt"],
        &[("work", Witnessed::Settling, Some(dead_group()))],
        None,
        &db,
    );
    drop(held);
    let at = reservation_path(host.home(), &work, "run-a");
    let mut reservation: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    reservation["attempts"]["attempt"]["cleanup"] = json!("unresolved");
    std::fs::write(&at, serde_json::to_vec(&reservation).unwrap()).unwrap();
    let _stood = old_run_holds(&host, &work, &mut store);
    let head = store.head_hash("run-a").unwrap();
    let refused = operator_settle(
        &mut store,
        "run-a",
        &Settle {
            reason: "every group is gone",
            operator: "operator",
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Live(Live::Unproven {
                ref attempt,
                why: UnprovenWhy::Unresolved,
            })) if attempt == "attempt"
        ),
        "settle refuses the unresolved cleanup by type: {refused:?}"
    );
    assert_eq!(
        store.head_hash("run-a").unwrap(),
        head,
        "the refusal wrote nothing"
    );
    assert_eq!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])),
        cr2(Conflict::Held("run-a".into())),
        "the claim stays held"
    );
    operator_repair(
        &mut store,
        "run-a",
        &Repair {
            operator: "operator",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    crate::engine::operator_command(
        &mut store,
        "run-a",
        OperatorCommand::Release,
        "operator",
        "every process the attempt owned has ended",
    )
    .expect("the operator's release attests the unresolved attempt ended");
    assert!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])).is_ok(),
        "the operator's release admits the next claim"
    );
}

/// 440d's one slot on a multi-driver record is asked by settle too, as
/// `release` asks it: a live slot is named by its type instead of falling
/// through to `NoGroup`, and nothing is written (S1; decision 0108 ruling
/// 3).
#[test]
fn settle_names_a_recorded_slot_that_still_runs_for_a_multi_driver_record() {
    if !isolated(
        "engine::broker::exclusion::tests::settle_names_a_recorded_slot_that_still_runs_for_a_multi_driver_record",
    ) {
        return;
    }
    let live = LiveGroup::new();
    let (mut fixture, held) =
        quarantined_drivers(&[("m1", Witnessed::Settling), ("m2", Witnessed::Settling)]);
    drop(held);
    rewrite_reservation(&fixture, |reservation| {
        reservation["attempts"]["a1"]["group"] = json!(live.group());
    });
    refused_live(&mut fixture, live.group());
    drop(live);
}

/// The workspace this unit serves: a binary that reported 0.12.0 from main
/// between #745 and b8e2caff8 wrote a reservation whose boxed reader has no
/// `sealed.store`, and a #745..#822 build listed no drivers. This build
/// reads such a record only through the view, so the attempt holds, and
/// both `settle` and, after a repair, `release` refuse it as a view-only
/// record and name the operator's route: the F4 route does not hold here,
/// and the commit is the operator's ask. The missing `store` is a field the
/// record lacks, not one it names, so it keeps `ViewOnly`'s words and route,
/// unlike a newer writer's unknown field, which takes the format advice
/// (R5). The test pins the refusal, not a new proof.
#[test]
fn an_old_boxed_reservation_without_store_is_read_through_the_view() {
    use crate::engine::broker::admission::tests::Coordinated;
    use brokkr_core::fold::OperatorCommand;

    fn without_store(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("store");
                for value in map.values_mut() {
                    without_store(value);
                }
            }
            serde_json::Value::Array(items) => {
                for value in items {
                    without_store(value);
                }
            }
            serde_json::Value::Null
            | serde_json::Value::Bool(_)
            | serde_json::Value::Number(_)
            | serde_json::Value::String(_) => {}
        }
    }

    let mut fixture = quarantined(&[Witnessed::Settling], None);
    rewrite_reservation(&fixture, |reservation| {
        for attempt in reservation["attempts"]
            .as_object_mut()
            .unwrap()
            .values_mut()
        {
            attempt.as_object_mut().unwrap().remove("drivers");
        }
        without_store(reservation);
    });
    let _stood = Coordinated::at(fixture.host.home());
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "a record read only through the view proves no driver ended"
    );
    let at = reservation_path(fixture.host.home(), &fixture.work, "run-a");
    let refused = operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "the old driver is gone",
            operator: "operator",
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            refused,
            EngineError::SettleRefused(SettleRefusal::Unadmitted(Unadmitted::Identity(
                Unknown::ViewOnly(ref found)
            ))) if found == &at
        ),
        "settle refuses the view-only record, naming its route: {refused:?}"
    );
    operator_repair(
        &mut fixture.store,
        "run-a",
        &Repair {
            operator: "operator",
            reason: "keep the old stop as evidence",
        },
    )
    .expect("the repair names the late tail");
    let refused = crate::engine::operator_command(
        &mut fixture.store,
        "run-a",
        OperatorCommand::Release,
        "operator",
        "every process the attempt owned has ended",
    )
    .unwrap_err();
    assert_eq!(
        unreleased(refused),
        Unreleased::standing(Unadmitted::Identity(Unknown::ViewOnly(at))),
        "release also refuses the view-only record, naming its route"
    );
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the worktree stays held: this shape has no route here"
    );
}

/// A repaired run still meets its fold on `resume`: the run reads as the
/// status its terminal event states, the resume is not a second driver, and
/// the run is the same stopped run. This is the command the release does not
/// make lawful; it is asserted here because the released run's later
/// commands were only ever checked for `retry` and `stop`.
#[test]
fn a_repaired_run_resumes_to_its_fold() {
    use crate::engine::operator::{Cause, Named};
    use brokkr_core::fold::Run;

    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let db = dir.path().join("forge.db");
    let bundle = crate::engine::tests::bundle(
        dir.path(),
        crate::engine::tests::single_body(vec!["missing-driver".into()]),
    );
    let mut engine = crate::engine::Engine::start(
        brokkr_store::Store::open(&db).unwrap(),
        bundle.clone(),
        "resume",
        Some(work.clone()),
    )
    .unwrap();
    let run = engine.run_id.clone();
    let _ = engine.drive();
    drop(engine);
    let mut store = brokkr_store::Store::open(&db).unwrap();
    let cited = brokkr_store::Store::open_read_only(&db).unwrap();
    let named = Named {
        cause: Cause {
            ending: brokkr_core::ending::Ending::Withdrawn,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "concluded",
        operator: "operator",
    };
    let state = crate::engine::conclude(&mut store, &cited, &run, &named).unwrap();
    assert_eq!(state.run, Run::Stopped);
    let head = store.load(&run).unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_chained(&connection, &run, &head, &crate::late::late_stop())
        .unwrap();
    drop(connection);
    let mut store = brokkr_store::Store::open(&db).unwrap();
    crate::engine::operator_repair(
        &mut store,
        &run,
        &Repair {
            operator: "operator",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    let mut resumed = crate::engine::Engine::resume(
        brokkr_store::Store::open(&db).unwrap(),
        bundle,
        &run,
        Some(work),
    )
    .expect("the repaired run resumes");
    let end = resumed.drive().unwrap();
    assert_eq!(end.state.run, Run::Stopped, "resume meets the run's fold");
}
