//! Fix round 0106b1r5, H1: the pass reads a route's cool-down rows and its
//! probe holds in one snapshot. A probe's refusal commits its renewed row
//! and ends its hold in one transaction; committed between the pass's two
//! statements, it would leave neither visible to a pass reading in two, and
//! the route would read open though it was held in every committed state.

use brokkr_store::{CooldownAppend, CooldownRound, ProbeHold, RoundAppend};

use super::*;
use crate::admission::capacity::tests::{appended, requested};

/// A running `run`, seated on alpha, and its terminal effect event, whose
/// one `waits` entry is beta/cloud's and whose row ended at `until`: the
/// episode a probe's round extends.
fn waited(fixture: &mut Fixture, run: &str, until: OffsetDateTime) -> EventEnvelope {
    fixture.seating(run, &[("alpha", "a-1")], false);
    requested(fixture, run, "effect");
    let started = json!({"effect_id": "effect", "attempt_id": "a1"});
    appended(fixture, run, vec![(EventType::EffectStarted, started)]);
    let store = &mut fixture.store;
    let wait = json!({"provider": "beta", "route": "cloud", "words": "slow down",
                      "seconds": 60, "until": until.format(&Rfc3339).unwrap(), "rounds": 1});
    store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: run,
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": "effect", "attempt_id": "a1", "error": "slow down",
                                "waits": [wait]}),
                causation_id: None,
                attempt_id: None,
            },
            &[row("beta", "cloud", until, "slow down")],
        )
        .unwrap()
        .envelope
}

/// With beta/cloud's old row expired and a probe's hold standing, the
/// probe's refusal commits its renewed row and ends its hold on a second
/// connection after the pass read the rows and before it reads the holds.
/// The pass reads the route cooling down to the renewed row's end, and the
/// claim, at the same instant, refuses it too.
#[test]
fn a_refusal_committed_between_the_pass_reads_leaves_its_route_cooling_down() {
    let (mut fixture, file, room) = room_before_rows();
    let event = waited(&mut fixture, "run-probe", at(-60));
    let hold = ProbeHold {
        run_id: "run-probe",
        attempt_id: "p-1",
        site: "",
        provider: "beta",
        route: "cloud",
        deadline: at(30),
    };
    let hold = fixture.store.hold_probe(&hold, at(-1)).unwrap();
    let (journal, renewed) = (fixture.store.path().to_path_buf(), at(300));
    let refusal = move || {
        let round = CooldownRound {
            run_id: "run-probe",
            event_seq: event.seq,
            site: "",
            provider: "beta",
            route: "cloud",
            until_at: renewed,
            reason: "renewed",
            retry_after_seconds: Some(300),
            rounds: 2,
            seconds: 300,
            ends: Some(hold),
        };
        let mut second = Store::open(&journal).unwrap();
        assert_eq!(
            second.append_cooldown_round(round).unwrap(),
            RoundAppend::Written
        );
    };
    let (judged, committed) = brokkr_store::test_support::before_statement(
        &mut fixture.store,
        "FROM probe_holds",
        refusal,
        |store| judged_in(store, &file, base()),
    );
    assert!(committed, "the refusal committed between the pass's reads");
    // The pass read the state before the commit: the hold standing.
    let words = "a cool-down probe of run run-probe (attempt p-1) holds the route until it \
                 answers, its renewed cool-down commits, or its process tree is settled";
    assert!(judged[0].room.is_none(), "the pass offered room");
    let probing = cooling_by(("beta", "cloud"), hold_alone(at(31), words));
    assert_eq!(verdicts(judged), vec![vec![probing]]);
    // The claim, after it, reads the renewed row.
    match (room.claim_at(&mut fixture.store, Generation(1), "admitted", base())).unwrap() {
        Err(reason) => assert_eq!(reason, cooling("beta", "cloud", renewed, "renewed")),
        Ok(claim) => panic!("the claim took {claim:?} over a renewed cool-down"),
    }
}
