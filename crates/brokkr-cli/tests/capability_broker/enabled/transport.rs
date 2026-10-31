//! Task 42.2 (U9b1b; CR3, CR4, MB3, MB4, MB5): the broker's real transport
//! faults through the success-reporting harness and the settlement barrier.
//! Each harness below reports success. The attempt fails anyway, with the
//! cause the broker latched at the stage it observed, and no call checkpoint
//! is invented for it. A second look at the parked run says the same, and
//! nothing external runs again.

use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::broker::Refusal;
use serde_json::{json, Value};

use super::{answering, World, STARTS};

/// The harness's report, overruled by `cause`, with no call row and the
/// harness having run once; and `inspect` of the parked run says the same,
/// executing nothing. `resume`'s half is the restart tests' below.
fn parked_by(world: &World, cause: String) {
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.phase.as_str(), &ended.cause),
        ("awaiting_operator", "work", &Some(cause)),
        "{ended:?}"
    );
    assert_eq!(world.broker_code(), Some(1));
    assert_eq!(world.calls(&ended.run), []);
    assert_eq!(world.starts(), 1);
    let notes = world.inspected(&ended.run)["dashboard"]["last_seat"]["notes"].clone();
    assert_eq!(
        notes
            .as_str()
            .and_then(|notes| notes.split_once("; stderr tail")),
        ended.cause.as_deref().map(|cause| (cause, ": ")),
    );
    assert_eq!(world.starts(), 1, "looking again executes nothing");
}

/// The restart half (42.3's, repeated on a real broker's real fault): the
/// engine is killed while the harness that outlived the broker's failure
/// still runs, so the attempt stays reserved. Once the harness has reported
/// success and gone, the broker having exited 1 and closed its ledger on
/// `closed`, a restarted engine settles the attempt indeterminate, as it holds
/// no lease and no proof of settlement: its harness's success stands for
/// nothing, no call is invented, and nothing starts again.
fn restarted_after(world: &World, closed: &Value) {
    let run = super::killed(world.held_serving());
    let held = Instant::now() + Duration::from_secs(90);
    let reservations = world.base.join("home/.local/state/brokkr/capabilities");
    let reserved = |root: &std::path::Path| {
        let dirs = std::fs::read_dir(root).unwrap();
        (dirs.flatten()).any(|repo| repo.path().join(&run).join("reservation.json").exists())
    };
    assert!(reserved(&reservations), "kept through the engine's death");
    std::fs::remove_file(world.bin("hold")).unwrap();
    while world.broker_code().is_none() {
        assert!(Instant::now() < held, "the harness never ended");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(world.broker_code(), Some(1), "the broker met its fault");
    let ledger = world.ledger();
    assert_eq!(
        ledger.last().map(|last| &last["disposition"]),
        Some(closed),
        "{ledger:?}"
    );
    let resumed = world.brokkr(&["resume", "--run", &run]).output().unwrap();
    let ended = super::ended(resumed);
    let restarted =
        "engine restarted while the attempt was in flight; completion cannot be established";
    let settled = ended
        .cause
        .as_deref()
        .and_then(|cause| cause.split_once(" indeterminate: "));
    assert_eq!(
        (ended.status.as_str(), settled.map(|(_, why)| why)),
        ("awaiting_operator", Some(restarted))
    );
    assert_eq!((world.calls(&run), world.starts()), (vec![], 1));
    assert!(
        reserved(&reservations),
        "unsettled, so kept past the restart"
    );
    let again = world.brokkr(&["resume", "--run", &run]).output().unwrap();
    let again = super::ended(again);
    assert_eq!(
        (again.status, again.cause, world.starts()),
        (ended.status, ended.cause, 1),
        "looking again rules the same and executes nothing"
    );
}

#[test]
fn a_restarted_engine_settles_an_exec_failure_indeterminate_and_executes_nothing_again() {
    let Some(world) = World::new() else {
        return;
    };
    world.server("#!/nonexistent/interpreter\n");
    restarted_after(&world, &json!({"failed": {"refused": "exec"}}));
}

#[test]
fn a_restarted_engine_settles_a_handoff_failure_indeterminate_and_executes_nothing_again() {
    let Some(world) = World::new() else {
        return;
    };
    world.store(b"DOCS_TOKEN=ab\0cd\n");
    restarted_after(&world, &json!({"failed": {"refused": "handoff"}}));
}

#[test]
fn a_restarted_engine_settles_status_eof_indeterminate_and_executes_nothing_again() {
    let Some(world) = World::new() else {
        return;
    };
    world.server("#!/bin/sh\n:\n");
    restarted_after(&world, &json!({"failed": "protocol"}));
}

/// The bootstrap's handoff tag: a bound value holding NUL crosses whole to a
/// bootstrap that refuses it, after the store's one lookup and with no
/// server started.
#[test]
fn the_handoff_failure_tag_fails_its_attempt_past_the_harness_success() {
    let Some(world) = World::new() else {
        return;
    };
    world.store(b"DOCS_TOKEN=ab\0cd\n");
    let started = world.server_reads();
    parked_by(&world, Refusal::Handoff.to_string());
    assert_eq!(started.lookups(), 0, "no server started");
}

/// The bootstrap's exec tag: an interpreter absent from the box, after the
/// lookup. Its exec was attempted, so [`World::server_reads`] proves nothing here.
#[test]
fn the_exec_failure_tag_fails_its_attempt_past_the_harness_success() {
    let Some(world) = World::new() else {
        return;
    };
    world.server("#!/nonexistent/interpreter\n");
    parked_by(&world, Refusal::Exec.to_string());
}

/// The status pipe ending with no tag after a successful exec certifies
/// nothing: a server that ends with no word of MCP fails initialization with
/// the protocol's own cause.
#[test]
fn status_eof_then_a_failed_initialization_keeps_the_protocols_cause() {
    let Some(world) = World::new() else {
        return;
    };
    world.server("#!/bin/sh\n:\n");
    parked_by(&world, Failure::Protocol.to_string());
}

/// Zero calls at the pinned version: a server off it ends the session before
/// the harness is served.
#[test]
fn a_zero_call_version_failure_is_not_hidden_by_the_harness() {
    let Some(world) = World::new() else {
        return;
    };
    world.server(&answering("").replace(r#""version":"1.4.2""#, r#""version":"9.9.9""#));
    parked_by(&world, Failure::Version.to_string());
}

/// A broker killed mid-session cannot persist its Closed, so what stands is
/// the ledger's verified prefix and the lifecycle refusal: the harness says a
/// real call, the broker makes its Started durable and is killed before the
/// server answers. The broker's cause is not invented, the begun call stands
/// interrupted with no digest, and the attempt fails past the harness's success.
#[test]
fn a_closed_that_cannot_persist_reads_as_the_lifecycle_refusal_and_no_invented_cause() {
    let Some(world) = World::new() else {
        return;
    };
    // The server holds the call, so the broker dies with it begun.
    world.server(&format!("{STARTS}sleep 30\n"));
    std::fs::write(world.bin("kill-broker"), "").unwrap();
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        (
            "awaiting_operator",
            Some("broker ledger lifecycle is invalid")
        ),
        "{ended:?}"
    );
    assert_eq!(world.broker_code(), Some(137));
    let ledger = world.ledger();
    let kinds: Vec<&str> = (ledger.iter())
        .map(|record| record["record"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["opened", "started"], "{ledger:?}");
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".into(), "interrupted".into(), None)]
    );
    assert_eq!(world.starts(), 1);
}
