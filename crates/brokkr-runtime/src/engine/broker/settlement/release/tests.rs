//! Decision 0108 ruling 3's proof, in isolation: what a record says about
//! an attempt, what the host answers about its process group, and what an
//! operator's settle record must name to release a standing reservation.

use super::*;
use crate::engine::broker::admission::tests::{dead_group, LiveGroup};
use crate::engine::broker::admission::Reserved;
use crate::engine::broker::settlement::Driver;

/// The reservation record of attempt `attempt`: its cleanup, its drivers,
/// and the process group it recorded.
fn reserved(cleanup: Witnessed, drivers: &[Witnessed], group: Option<u32>) -> Reserved {
    Reserved {
        protects: crate::engine::broker::admission::Protects::Nothing,
        writers: Vec::new(),
        cleanup,
        group,
        serves: Vec::new(),
        drivers: drivers
            .iter()
            .map(|cleanup| Driver {
                site: "work".into(),
                begun: None,
                cleanup: *cleanup,
            })
            .collect(),
    }
}

/// The view of a record another build wrote, as the release reads it: only
/// its attempt's own settlement fact.
fn viewed(cleanup: &str) -> Attempt {
    serde_json::from_value(serde_json::json!({
        "protects": "nothing",
        "writers": [],
        "cleanup": cleanup,
    }))
    .unwrap()
}

/// The operator's record, as the release reads it back.
fn settled(run: &str, attempts: Vec<Proven>) -> Settled {
    Settled {
        seq: 8,
        args: SettleArgs {
            run: run.into(),
            lock: LockState::Free,
            attempts,
            reason: "why".into(),
        },
    }
}

/// [`Proven`] for `attempt` as the record names it, by the host's word.
fn grouped(attempt: &str, group: Option<u32>) -> Proven {
    Proven {
        attempt: attempt.into(),
        proof: Proof::Group,
        group,
    }
}

/// The standing reservation `run` `attempt` of a boxed writer, for the
/// inventory's own view of it.
fn standing(run: &str) -> Standing {
    serde_json::from_value(serde_json::json!({
        "repo": "r", "run": run, "artifacts": "/a", "journal": "/j",
        "attempts": {"a1": {"protects": "nothing", "writers": [], "cleanup": "settling"}},
        "settled": [],
    }))
    .unwrap()
}

/// The binding: a group the kernel does not know is gone; a live one is
/// alive; and a group id this host cannot name counts as alive, so it never
/// releases a worktree.
#[test]
fn only_the_kernels_no_such_process_proves_a_group_gone() {
    assert!(!alive(dead_group()), "a reaped group is gone");
    let live = LiveGroup::new();
    assert!(alive(live.group()), "a running child is alive");
    assert!(
        alive(u32::MAX),
        "a group this host cannot name is never read as gone"
    );
    drop(live);
}

/// A record proves an attempt ended by every driver it spawned, or by
/// spawning none; where a driver is unsettled, only the host's word about
/// the group can, and the exclusion will not use it alone.
#[test]
fn the_records_and_the_host_prove_an_attempt_by_their_own_facts() {
    let none = reserved(Witnessed::Settling, &[], None);
    assert_eq!(proven(&none, "a1"), Ok(Proof::Records));
    assert_eq!(proven_records(&none, "a1"), Ok(()));

    let settled = reserved(Witnessed::Settling, &[Witnessed::Settled], None);
    assert_eq!(proven(&settled, "a1"), Ok(Proof::Records));

    let unsettled = reserved(Witnessed::Settling, &[Witnessed::Settling], None);
    assert_eq!(
        proven(&unsettled, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::NoGroup,
        })
    );

    let unresolved = reserved(Witnessed::Unresolved, &[], None);
    assert_eq!(
        proven(&unresolved, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::Unresolved,
        })
    );

    let gone = reserved(
        Witnessed::Settling,
        &[Witnessed::Settling],
        Some(dead_group()),
    );
    assert_eq!(proven(&gone, "a1"), Ok(Proof::Group));
    assert_eq!(
        proven_records(&gone, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::NoRecords,
        }),
        "the exclusion never releases on the host's word alone"
    );

    let live = LiveGroup::new();
    let still = reserved(
        Witnessed::Settled,
        &[Witnessed::Settled],
        Some(live.group()),
    );
    assert_eq!(
        proven(&still, "a1"),
        Err(Live::Group {
            attempt: "a1".into(),
            group: live.group(),
        }),
        "the host's live group refuses whatever the record says"
    );
    drop(live);
}

/// A record read through another build's view proves only its attempt's own
/// settlement fact; nothing else is guessed.
#[test]
fn the_views_settled_fact_proves_it_and_nothing_else_does() {
    let settled = viewed("settled");
    assert_eq!(view_records(&settled, "a1"), Ok(()));
    let settling = viewed("settling");
    assert_eq!(
        view_records(&settling, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::Unrecorded,
        })
    );
}

/// The record the release reads is the last settle annotation, parsed at
/// the edge; a journal without one, or with another word, carries none.
#[test]
fn the_last_settle_annotation_is_the_record() {
    let none = record(&[]);
    assert_eq!(none, None);
    let mut other = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "repair", "args": {}}),
    );
    other.seq = 7;
    assert_eq!(record(&[other]), None);
    let mut first = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "lock": "free", "attempts": [], "reason": "first"}}),
    );
    first.seq = 8;
    let mut second = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "lock": "none", "attempts": [], "reason": "second"}}),
    );
    second.seq = 9;
    let no_args = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle"}),
    );
    assert_eq!(
        record(&[no_args]),
        None,
        "a record without args is no record"
    );
    let found = record(&[first, second]).expect("the last settle reads");
    assert_eq!(found.seq, 9);
    assert_eq!(found.args.reason, "second");
    assert_eq!(found.args.lock, LockState::None);
}

/// A repair record is the operator's word only after the terminal event: a
/// `repair` standing before it is the pending command it has always been.
#[test]
fn a_repair_record_counts_only_after_the_terminal_event() {
    let terminal = crate::engine::tests::event(EventType::RunStopped, serde_json::json!({}));
    let mut after = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "repair", "args": {}}),
    );
    after.seq = 3;
    assert!(repaired(&[terminal.clone(), after.clone()]));
    let mut before = after;
    before.seq = 1;
    assert!(
        !repaired(&[before, terminal.clone()]),
        "a pre-terminal repair is no record"
    );
    assert!(!repaired(&[terminal]), "no repair record at all");
}

/// A group proof names the group the broker recorded, not one of its own
/// choosing: a record that names a gone group while the reservation shows
/// another alive does not release.
#[test]
fn a_group_proof_must_name_the_group_the_broker_recorded() {
    let live = LiveGroup::new();
    let mut whole = serde_json::from_value::<crate::engine::broker::admission::Reservation>(
        serde_json::json!({
            "repo": "r", "run": "a", "workdir": "/w", "artifacts": "/a", "journal": "/j",
            "attempts": {"a1": {"protects": "nothing", "writers": [], "cleanup": "settling",
                                "group": live.group()}},
            "settled": [],
        }),
    )
    .unwrap();
    whole.attempts.get_mut("a1").unwrap().drivers.clear();
    let held = Standing::from(whole);
    assert!(
        !honoured(
            &held,
            &settled("a", vec![grouped("a1", Some(dead_group()))])
        ),
        "the broker's live group is the one that decides"
    );
    drop(live);
}

/// The record releases the standing reservation it names and covers: never
/// another run's, never a reservation with an attempt it left out, and
/// never one naming a group the host still shows alive.
#[test]
fn a_settle_record_releases_only_the_reservation_it_covers() {
    let held = standing("a");
    assert!(
        honoured(
            &held,
            &settled("a", vec![grouped("a1", Some(dead_group()))])
        ),
        "a gone group the record names releases an attempt the records cannot prove"
    );
    assert!(
        !honoured(&held, &settled("a", vec![grouped("a1", None)])),
        "a group proof with no group releases nothing"
    );
    let mut settled_record =
        serde_json::from_value::<crate::engine::broker::admission::Reservation>(
            serde_json::json!({
                "repo": "r", "run": "a", "workdir": "/w", "artifacts": "/a", "journal": "/j",
                "attempts": {"a1": {"protects": "nothing", "writers": [], "cleanup": "settled"}},
                "settled": [],
            }),
        )
        .unwrap();
    settled_record
        .attempts
        .get_mut("a1")
        .unwrap()
        .drivers
        .clear();
    let proven_by_records = Standing::from(settled_record);
    assert!(
        honoured(
            &proven_by_records,
            &settled(
                "a",
                vec![Proven {
                    attempt: "a1".into(),
                    proof: Proof::Records,
                    group: None,
                }]
            )
        ),
        "a record that names settled drivers releases"
    );
    assert!(
        !honoured(
            &held,
            &settled("b", vec![grouped("a1", Some(dead_group()))])
        ),
        "another run's record releases nothing"
    );
    assert!(
        !honoured(&held, &settled("a", Vec::new())),
        "a record that names no attempt does not cover a standing one"
    );
    assert!(
        !honoured(
            &held,
            &settled("a", vec![grouped("a0", Some(dead_group()))])
        ),
        "a record that names another attempt does not cover it"
    );
    let live = LiveGroup::new();
    assert!(
        !honoured(
            &held,
            &settled("a", vec![grouped("a1", Some(live.group()))])
        ),
        "a group the host still shows alive does not release"
    );
    drop(live);
}
