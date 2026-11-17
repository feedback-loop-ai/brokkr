//! Decision 0110 ruling 7: the journal's run ceiling, judged by the pass
//! against the census and again by the claim's re-check, and a host file
//! that declares none waiting each entry the capacity checks judge.

use std::path::PathBuf;

use brokkr_store::{EntryState, QueueCommand};
use serde_json::{json, Value};

use super::super::tests::{
    admitted, claimed, full, host, passed, route, seated, write_cooling, Fixture, BY,
};
use super::super::{Capacity, Reason};

/// A v4 host: alpha at `alpha` runs, beta's cloud route at nine, and the
/// journal at `runs`.
fn ceiled(alpha: u32, runs: u32) -> Value {
    let mut host = host(json!({"alpha": {"ceiling": alpha},
                              "beta": {"ceiling": 9, "routes": {"cloud": route(9, "cloud")}}}));
    host["runs"]["ceiling"] = json!(runs);
    host
}

/// [`ceiled`] at alpha's nine, written as `schema`, which declares no run
/// ceiling.
fn undeclared(fixture: &Fixture, schema: &str) -> PathBuf {
    let mut host = ceiled(9, 9);
    host["schema"] = json!(schema);
    host.as_object_mut().unwrap().remove("runs");
    fixture.declare(&host)
}

fn runs_full(running: usize, ceiling: u32) -> Option<Capacity> {
    Some(Capacity::RunsFull { running, ceiling })
}

/// A fixture with one entry queued and `running` runs running, each
/// seating alpha's one lane, under [`ceiled`] at `alpha` and `runs`.
fn running(alpha: u32, runs: u32, running: usize) -> (Fixture, PathBuf) {
    let mut fixture = Fixture::new();
    fixture.queue();
    for run in 0..running {
        fixture.seating(&format!("r{run}"), &[("alpha", "a-1")], false);
    }
    let file = fixture.declare(&ceiled(alpha, runs));
    (fixture, file)
}

#[test]
fn at_its_run_ceiling_the_journal_waits_the_entry_as_runs_full() {
    let (fixture, file) = running(9, 2, 2);
    let full = runs_full(2, 2);
    assert_eq!(fixture.room(&file), full);
    let reason = Reason::Capacity(full.unwrap());
    assert_eq!(
        (reason.kind(), reason.holds(), reason.to_string()),
        (
            "runs_full",
            false,
            "the journal drives 2 runs, at its ceiling of 2 (forge.host/v4 `runs.ceiling`)".into()
        )
    );
}

#[test]
fn below_its_run_ceiling_the_entry_is_admitted() {
    let (fixture, file) = running(9, 2, 1);
    assert_eq!(fixture.room(&file), None);
    assert!(passed(&fixture.store, &file).unwrap()[0].room.is_some());
}

/// A `brokkr run` run is created with no queue entry and no claim, and the
/// census counts it as it counts any running run (decision 0095 ruling 6).
#[test]
fn a_direct_run_counts_against_the_run_ceiling() {
    let (mut fixture, file) = running(9, 1, 0);
    let manifest = seated(&[("beta", "cloud/b-2")], false);
    fixture
        .store
        .create_run("direct", "f", "b", &manifest)
        .unwrap();
    let started = json!({"feature": "f"});
    (fixture.store)
        .append_next(
            "direct",
            brokkr_core::EventType::RunStarted,
            started,
            None,
            None,
        )
        .unwrap();
    assert_eq!(fixture.store.queue_list().unwrap().len(), 1);
    assert_eq!(fixture.room(&file), runs_full(1, 1));
}

/// A run with a row and no events spawned nothing (decision 0095 ruling 4),
/// so it takes no place under the run ceiling.
#[test]
fn an_unstarted_run_counts_against_no_run_ceiling() {
    let (fixture, file) = running(9, 1, 0);
    fixture.unstarted("r0");
    assert_eq!(fixture.room(&file), None);
}

#[test]
fn a_host_that_declares_no_run_ceiling_waits_the_entry_as_runs_undeclared() {
    let mut fixture = Fixture::new();
    fixture.queue();
    for schema in ["forge.host/v1", "forge.host/v2", "forge.host/v3"] {
        let file = undeclared(&fixture, schema);
        let reason = fixture.room(&file).map(Reason::Capacity);
        assert_eq!(
            reason,
            Some(Reason::Capacity(Capacity::RunsUndeclared(file.clone()))),
            "{schema}"
        );
        let reason = reason.unwrap();
        assert_eq!(
            (reason.kind(), reason.holds(), reason.to_string()),
            (
                "runs_undeclared",
                false,
                format!(
                    "the host file {} declares no run ceiling; forge.host/v4 declares \
                     `runs.ceiling`",
                    file.display()
                )
            )
        );
    }
    // Nothing latches: the operator's v4 file admits the entry next pass.
    let file = fixture.declare(&ceiled(9, 9));
    assert_eq!(fixture.room(&file), None);
}

/// Decision 0110 ruling 7 and the controller's R2: `runs_undeclared` is a
/// declaration check, met before a cool-down, and `runs_full` is the first
/// of the ceilings, met before a provider's.
#[test]
fn an_undeclared_run_ceiling_comes_before_a_cool_down_and_a_full_one_before_a_provider() {
    let mut fixture = Fixture::new();
    fixture.seat("cloudy", "cloudy");
    fixture.queue();
    write_cooling(&mut fixture.store, "cooled");
    let file = undeclared(&fixture, "forge.host/v3");
    assert_eq!(fixture.room(&file), Some(Capacity::RunsUndeclared(file)));
    let cooling = fixture.room(&fixture.declare(&ceiled(9, 9)));
    assert!(
        matches!(cooling, Some(Capacity::CoolingDown { .. })),
        "{cooling:?}"
    );

    let (fixture, file) = running(1, 1, 1);
    assert_eq!(fixture.room(&file), runs_full(1, 1));
    let file = fixture.declare(&ceiled(1, 2));
    assert_eq!(fixture.room(&file), full("alpha", 1, 1));
}

/// Decision 0095 ruling 7 under the run ceiling: a run started between
/// the pass and the claim takes the place the pass found, and the claim
/// refuses on the run ceiling with nothing written.
#[test]
fn a_run_started_after_the_pass_refuses_the_claim_on_the_run_ceiling() {
    let (mut fixture, file) = running(9, 1, 0);
    let held = fixture.queue();
    (fixture.store)
        .queue_command(held, QueueCommand::Hold, BY)
        .unwrap();
    let room = admitted(&fixture, &file);
    fixture.seating("late", &[("beta", "cloud/b-2")], false);
    let before = fixture.store.queue_list().unwrap();
    let refused = claimed(&mut fixture, &room);
    assert_eq!(refused, Err(Reason::Capacity(runs_full(1, 1).unwrap())));
    assert_eq!(fixture.store.queue_list().unwrap(), before);
    let states: Vec<EntryState> = before.into_iter().map(|entry| entry.state).collect();
    assert_eq!(states, [EntryState::Queued, EntryState::Held]);
}
