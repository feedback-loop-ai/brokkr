//! Decision 0108 ruling 3's proof, in isolation: what a record says about
//! an attempt, what the host answers about its process group, and what an
//! operator's settle record must name to release a standing reservation.

use super::*;
use crate::engine::broker::admission::tests::{dead_group, isolated, LiveGroup};
use crate::engine::broker::admission::Reserved;
use crate::engine::broker::settlement::Driver;

/// The reservation record of attempt `attempt`: its cleanup, its drivers,
/// and the single group 440d's shape recorded, where `group` names one.
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
                group: None,
            })
            .collect(),
    }
}

/// The reservation record of attempt `attempt` with each driver's own
/// cleanup and process group, as the dispatch door now writes it.
fn grouped_drivers(cleanup: Witnessed, drivers: &[(Witnessed, Option<u32>)]) -> Reserved {
    Reserved {
        protects: crate::engine::broker::admission::Protects::Nothing,
        writers: Vec::new(),
        cleanup,
        group: None,
        serves: Vec::new(),
        drivers: drivers
            .iter()
            .enumerate()
            .map(|(index, (cleanup, group))| Driver {
                site: format!("work:{index}"),
                begun: None,
                cleanup: *cleanup,
                group: *group,
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
            attempts,
            reason: "why".into(),
        },
    }
}

/// [`Proven`] for `attempt` as the record names it, by the host's word.
fn grouped(attempt: &str, groups: &[u32]) -> Proven {
    Proven {
        attempt: attempt.into(),
        proof: Proof::Group,
        groups: groups.to_vec(),
    }
}

/// [`Proven`] for `attempt` as the record names it, by the broker's records.
fn record_proven(attempt: &str) -> Proven {
    Proven {
        attempt: attempt.into(),
        proof: Proof::Records,
        groups: Vec::new(),
    }
}

/// A whole reservation of attempt `a1` with `cleanup`, and 440d's single
/// group slot where `group` names one, its drivers cleared: the exclusion
/// reads the attempt's cleanup and slot, not its driver records.
fn whole(cleanup: &str, group: Option<u32>) -> crate::engine::broker::admission::Reservation {
    let mut value = serde_json::json!({
        "repo": "r", "run": "a", "workdir": "/w", "artifacts": "/a", "journal": "/j",
        "attempts": {"a1": {"protects": "nothing", "writers": [], "cleanup": cleanup}},
        "settled": [],
    });
    if let Some(group) = group {
        value["attempts"]["a1"]["group"] = serde_json::json!(group);
    }
    let mut reservation: Reservation = serde_json::from_value(value).unwrap();
    reservation.attempts.get_mut("a1").unwrap().drivers.clear();
    reservation
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
    if !isolated("engine::broker::settlement::release::tests::only_the_kernels_no_such_process_proves_a_group_gone") {
        return;
    }
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
    if !isolated("engine::broker::settlement::release::tests::the_records_and_the_host_prove_an_attempt_by_their_own_facts") {
        return;
    }
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

/// A panel attempt has one record per driver, each with its own group: a
/// settled sibling's dead group never stands for the driver that is still
/// settling, and a driver whose group the amend never wrote refuses.
#[test]
fn a_panels_drivers_are_asked_each_by_its_own_group() {
    if !isolated("engine::broker::settlement::release::tests::a_panels_drivers_are_asked_each_by_its_own_group") {
        return;
    }
    let live = LiveGroup::new();
    // Each driver's own group is asked: a settled sibling's dead group
    // never stands for the driver that is still settling.
    let panel = grouped_drivers(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(live.group())),
            (Witnessed::Settled, None),
        ],
    );
    assert_eq!(
        proven(&panel, "a1"),
        Err(Live::Group {
            attempt: "a1".into(),
            group: live.group(),
        }),
        "the first driver's own group is the one asked"
    );
    let panel_gone = grouped_drivers(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(dead_group())),
            (Witnessed::Settled, None),
        ],
    );
    assert_eq!(proven(&panel_gone, "a1"), Ok(Proof::Group));
    assert_eq!(
        unsettled_groups(&panel_gone),
        Some(vec![panel_gone.drivers[0].group.unwrap()]),
        "the group proof names the driver the records did not prove"
    );

    // A driver whose group the amend never wrote refuses: an earlier
    // driver's dead group is not its proof.
    let amended = grouped_drivers(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(dead_group())),
            (Witnessed::Settling, None),
        ],
    );
    assert_eq!(
        proven(&amended, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::NoGroup,
        }),
        "a driver with no recorded group refuses"
    );

    // 440d's one slot names the last driver, so it may prove only a
    // single-driver attempt.
    let legacy_panel = reserved(
        Witnessed::Settling,
        &[Witnessed::Settling, Witnessed::Settling],
        Some(dead_group()),
    );
    assert_eq!(
        proven(&legacy_panel, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::NoGroup,
        }),
        "a legacy single slot never proves a multi-driver attempt"
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
    let mut terminal = crate::engine::tests::event(EventType::RunStopped, serde_json::json!({}));
    terminal.seq = 5;
    let mut first = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "attempts": [], "reason": "first"}}),
    );
    first.seq = 8;
    let mut second = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "attempts": [], "reason": "second"}}),
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
    let found =
        record(&[terminal.clone(), first, second]).expect("the last settle reads after the end");
    assert_eq!(found.seq, 9);
    assert_eq!(found.args.reason, "second");
    let mut before = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "attempts": [], "reason": "pending"}}),
    );
    before.seq = 4;
    assert_eq!(
        record(&[before, terminal]),
        None,
        "a settle word before the terminal is the pending command, not a record"
    );
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
    if !isolated("engine::broker::settlement::release::tests::a_group_proof_must_name_the_group_the_broker_recorded") {
        return;
    }
    let live = LiveGroup::new();
    let held = Standing::from(whole("settling", Some(live.group())));
    assert!(
        !honoured(&held, &settled("a", vec![grouped("a1", &[dead_group()])])),
        "the broker's live group is the one that decides"
    );
    drop(live);
}

/// The record releases the standing reservation it names and covers: never
/// another run's, never a reservation with an attempt it left out, and
/// never one naming a group the host still shows alive.
#[test]
fn a_settle_record_releases_only_the_reservation_it_covers() {
    if !isolated("engine::broker::settlement::release::tests::a_settle_record_releases_only_the_reservation_it_covers") {
        return;
    }
    let held = standing("a");
    assert!(
        honoured(&held, &settled("a", vec![grouped("a1", &[dead_group()])])),
        "a gone group the record names releases an attempt the records cannot prove"
    );
    assert!(
        !honoured(&held, &settled("a", vec![grouped("a1", &[])])),
        "a group proof with no group releases nothing"
    );
    let proven_by_records = Standing::from(whole("settled", None));
    assert!(
        honoured(&proven_by_records, &settled("a", vec![record_proven("a1")])),
        "a record that names settled drivers releases"
    );
    assert!(
        !honoured(&held, &settled("b", vec![grouped("a1", &[dead_group()])])),
        "another run's record releases nothing"
    );
    assert!(
        !honoured(&held, &settled("a", Vec::new())),
        "a record that names no attempt does not cover a standing one"
    );
    assert!(
        !honoured(&held, &settled("a", vec![grouped("a0", &[dead_group()])])),
        "a record that names another attempt does not cover it"
    );
    let live = LiveGroup::new();
    assert!(
        !honoured(&held, &settled("a", vec![grouped("a1", &[live.group()])])),
        "a group the host still shows alive does not release"
    );
    drop(live);
}

/// The contract says a record that names an attempt the reservation does
/// not name does not release: it is not a superset, it is a mismatch.
#[test]
fn a_settle_record_naming_an_unknown_attempt_does_not_release() {
    let held = standing("a");
    assert!(
        !honoured(
            &held,
            &settled(
                "a",
                vec![
                    grouped("a1", &[dead_group()]),
                    grouped("a9", &[dead_group()]),
                ],
            )
        ),
        "an attempt the reservation does not name is not covered by naming it"
    );
}

/// The `living` check is not the group-coverage check: a record whose
/// groups match the reservation's dead ones but also names a live group
/// releases nothing, even when its records proof would otherwise stand.
#[test]
fn a_named_live_group_keeps_the_worktree_held_on_its_own() {
    if !isolated("engine::broker::settlement::release::tests::a_named_live_group_keeps_the_worktree_held_on_its_own") {
        return;
    }
    let live = LiveGroup::new();
    let held = Standing::from(whole("settled", None));
    let record = Proven {
        attempt: "a1".into(),
        proof: Proof::Records,
        groups: vec![live.group()],
    };
    assert!(
        !honoured(&held, &settled("a", vec![record])),
        "the record names a live group, so nothing releases"
    );
    drop(live);
}

/// 440d's on-disk shape — one `group` slot on the attempt, drivers without
/// their own — still reads, and its single group proves a single-driver
/// attempt only. A multi-driver record of that shape is refused: the slot
/// names the last driver, never both.
#[test]
fn a_440d_record_still_reads_and_its_group_proves_only_one_driver() {
    if !isolated("engine::broker::settlement::release::tests::a_440d_record_still_reads_and_its_group_proves_only_one_driver") {
        return;
    }
    let one = serde_json::json!({
        "protects": "nothing", "writers": [], "cleanup": "settling",
        "group": dead_group(),
        "drivers": [{"site": "work", "cleanup": "settling"}],
    });
    let reserved: Reserved = serde_json::from_value(one).expect("440d's shape reads");
    assert_eq!(proven(&reserved, "a1"), Ok(Proof::Group));
    assert_eq!(
        unsettled_groups(&reserved),
        Some(vec![reserved.group.unwrap()]),
        "the one slot stands for the one driver"
    );
    let two = serde_json::json!({
        "protects": "nothing", "writers": [], "cleanup": "settling",
        "group": dead_group(),
        "drivers": [
            {"site": "m1", "cleanup": "settling"},
            {"site": "m2", "cleanup": "settling"},
        ],
    });
    let reserved: Reserved = serde_json::from_value(two).expect("440d's shape reads");
    assert_eq!(
        proven(&reserved, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: Unproven::NoGroup,
        }),
        "the last driver's slot cannot prove a sibling"
    );
}
