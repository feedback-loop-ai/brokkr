//! Fix round 0106b1r12, S4: a probe that parks while its tree may still
//! run keeps its hold through the park, until a settlement ends it. Its
//! report's cleanup is unresolved (a process of its tree outlived the
//! kill), so nothing proves the tree over: the run parks with the cleanup
//! refusal, and both production gates read the route cooling down through
//! the park, a day past its stored end. A settlement of the probe's
//! attempt, the store's one way the reaper and the operator's settle
//! record end a hold, opens it.

use brokkr_protocol::process::Unsettled;
use brokkr_protocol::Cleanup;

use super::held_route::{held_to, legs, probing, unended_holds};
use super::*;

#[test]
fn a_probe_parked_with_its_tree_unsettled_keeps_its_hold_until_settlement() {
    let (_dir, mut engine, _now, probe_at, gates) = probing(Probed::Answer);
    let run = engine.run_id.clone();
    let hold = brokkr_store::ProbeHold {
        run_id: &run,
        attempt_id: "parked",
        site: "",
        provider: "claude",
        route: "first",
        fallback: probe_at,
    };
    engine.cooldown.hold = Some(engine.store.hold_probe(&hold, probe_at).unwrap());
    let unresolved = Cleanup::Unresolved {
        reason: Unsettled::Descendants { pids: vec![4242] },
    };
    let answered = vec![json!({"step": "answer"})];
    let ended = engine.probe_ended(
        ("effect", "work"),
        &wait(),
        report(None, unresolved, answered),
    );
    let super::super::Probe::Parked(park) = ended else {
        panic!("a probe whose tree may still run did not park");
    };
    assert!(
        park.reason().contains("cleanup failure"),
        "{}",
        park.reason()
    );
    let journal = engine.store.path().to_path_buf();
    assert_eq!(unended_holds(&journal), 1);
    // The first whole second after the end the hold was given, its readout.
    let end = probe_at + time::Duration::seconds(1);
    for at in [probe_at, end + time::Duration::days(1)] {
        assert!(held_to(&legs(&gates, at), end), "parked, at {at}");
    }
    engine
        .store
        .settle_probe_holds(&run, &["parked".into()], probe_at)
        .unwrap();
    assert_eq!(unended_holds(&journal), 0);
    assert_eq!(legs(&gates, probe_at), [Ok(()), Ok(())]);
}
