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

/// A whole reservation read only through the view of another build, its one
/// attempt's cleanup as `cleanup` says: no whole record, so its groups are
/// not this build's to check.
fn viewed_standing(run: &str, cleanup: &str) -> Standing {
    serde_json::from_value(serde_json::json!({
        "repo": "r", "run": run, "artifacts": "/a", "journal": "/j",
        "attempts": {"a1": {"protects": "nothing", "writers": [], "cleanup": cleanup}},
        "settled": [],
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

/// The whole reservation of attempt `a1` with each driver's own cleanup and
/// process group, as the dispatch door writes it: what `honoured` reads when
/// a settle record's group proof names its drivers' groups.
fn grouped_whole(
    cleanup: Witnessed,
    drivers: &[(Witnessed, Option<u32>)],
) -> crate::engine::broker::admission::Reservation {
    let drivers: Vec<Value> = drivers
        .iter()
        .enumerate()
        .map(|(index, (cleanup, group))| {
            let mut driver = serde_json::json!({
                "site": format!("work:{index}"),
                "cleanup": cleanup,
            });
            if let Some(group) = group {
                driver["group"] = serde_json::json!(group);
            }
            driver
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "repo": "r", "run": "a", "workdir": "/w", "artifacts": "/a", "journal": "/j",
        "attempts": {"a1": {
            "protects": "nothing", "writers": [], "cleanup": cleanup, "drivers": drivers,
        }},
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
            why: UnprovenWhy::NoGroup,
        })
    );

    let unresolved = reserved(Witnessed::Unresolved, &[], None);
    assert_eq!(
        proven(&unresolved, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::Unresolved,
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
            why: UnprovenWhy::NoRecords,
        }),
        "the exclusion never releases on the host's word alone"
    );
    assert_eq!(
        proven_records(&gone, "a1").unwrap_err().to_string(),
        "attempt 'a1' stands: only the host's word proves it: its process group is gone, and no \
         driver record is sealed Settled; `brokkr operator settle` is the operator's route"
    );

    // A group recorded for a Settled driver is not asked: the seal is the
    // settlement proof, and a reused id names an unrelated process (F9's
    // correction). Asking it would hold the worktree with no route.
    let live = LiveGroup::new();
    let reused = reserved(
        Witnessed::Settled,
        &[Witnessed::Settled],
        Some(live.group()),
    );
    assert_eq!(
        proven(&reused, "a1"),
        Ok(Proof::Records),
        "a settled driver's reused group id is not the driver"
    );
    drop(live);

    // A cleanup witnessed unresolved refuses even when every driver record
    // is Settled, or when an unsettled driver recorded no group.
    let unresolved_driver = reserved(Witnessed::Unresolved, &[Witnessed::Settling], None);
    assert_eq!(
        proven(&unresolved_driver, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::Unresolved,
        })
    );
    let unresolved_settled = reserved(Witnessed::Unresolved, &[Witnessed::Settled], None);
    assert_eq!(
        proven(&unresolved_settled, "a1"),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::Unresolved,
        })
    );
    assert_eq!(
        Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::Unresolved,
        }
        .to_string(),
        "attempt 'a1' stands: its cleanup was witnessed unresolved, so its drivers' records do \
         not prove that the tree it left over has ended",
        "the unresolved refusal says what it knows, and no missing group it never asked about"
    );
    // A driver's own group gone proves the process it led ended, even when
    // its cleanup was witnessed unresolved: the host answers the driver the
    // records do not (S2; H1's rule, decision 0108 ruling 3).
    let unresolved_gone = reserved(
        Witnessed::Unresolved,
        &[Witnessed::Settling],
        Some(dead_group()),
    );
    assert_eq!(proven(&unresolved_gone, "a1"), Ok(Proof::Group));
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
            why: UnprovenWhy::NoGroup,
        }),
        "a driver with no recorded group refuses"
    );

    // A group one driver recorded is still asked even where a sibling
    // recorded none: the live group is named, not hidden behind NoGroup
    // (S1(b)).
    let mixed = grouped_drivers(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(live.group())),
            (Witnessed::Settling, None),
        ],
    );
    assert_eq!(
        proven(&mixed, "a1"),
        Err(Live::Group {
            attempt: "a1".into(),
            group: live.group(),
        }),
        "a recorded live group is asked before a sibling's missing one refuses"
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
            why: UnprovenWhy::NoGroup,
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
            why: UnprovenWhy::Unrecorded,
        })
    );
    assert_eq!(
        view_records(&settling, "a1").unwrap_err().to_string(),
        "attempt 'a1' stands: the record names no settled attempt, and carries no process group \
         to ask about"
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
    // A malformed last record is no record at all: it never falls back to
    // an earlier one (decision 0089 ruling 9).
    let mut malformed = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {"run": "a"}}),
    );
    malformed.seq = 10;
    let mut earlier = crate::engine::tests::event(
        EventType::OperatorCommanded,
        serde_json::json!({"command": "settle", "args": {
            "run": "a", "attempts": [], "reason": "first"}}),
    );
    earlier.seq = 6;
    assert_eq!(
        record(&[terminal.clone(), earlier, malformed]),
        None,
        "a malformed last record is never an earlier one"
    );
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

/// A group proof must name every group of every driver the records do not
/// prove settled, and no group the reservation does not record: a dead
/// sibling's group cannot stand for a live one, and a name invented by the
/// record proves nothing (F6, decision 0108 ruling 3).
#[test]
fn a_group_proof_must_name_every_unsettled_drivers_own_group() {
    if !isolated("engine::broker::settlement::release::tests::a_group_proof_must_name_every_unsettled_drivers_own_group") {
        return;
    }
    let (g1, g2) = (dead_group(), dead_group());
    let both = Standing::from(grouped_whole(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(g1)),
            (Witnessed::Settling, Some(g2)),
        ],
    ));
    assert!(
        honoured(&both, &settled("a", vec![grouped("a1", &[g1, g2])])),
        "both recorded groups gone prove both drivers"
    );
    assert!(
        !honoured(&both, &settled("a", vec![grouped("a1", &[g1])])),
        "one of two unsettled drivers' groups is not every one"
    );
    assert!(
        !honoured(
            &both,
            &settled("a", vec![grouped("a1", &[g1, g2, dead_group()])])
        ),
        "a group the reservation does not record proves nothing"
    );
    let missing = Standing::from(grouped_whole(
        Witnessed::Settling,
        &[(Witnessed::Settling, Some(g1)), (Witnessed::Settling, None)],
    ));
    assert!(
        !honoured(&missing, &settled("a", vec![grouped("a1", &[g1])])),
        "a driver with no recorded group cannot be stood in for by a sibling's"
    );
    let live = LiveGroup::new();
    let alive_sibling = Standing::from(grouped_whole(
        Witnessed::Settling,
        &[
            (Witnessed::Settling, Some(g1)),
            (Witnessed::Settling, Some(live.group())),
        ],
    ));
    assert!(
        !honoured(&alive_sibling, &settled("a", vec![grouped("a1", &[g1])])),
        "the record leaves the reservation's live driver unnamed, so it does not cover it"
    );
    drop(live);
}

/// A group proof and the records proof agree on a cleanup witnessed
/// unresolved: where `proven` writes a Group proof (every unsettled driver's
/// own group gone), `honoured` accepts it; where it refuses (no driver left
/// to ask), `honoured` refuses it too (S2's chief addition; 0071 ruling 5).
#[test]
fn a_group_proof_and_the_records_agree_on_an_unresolved_cleanup() {
    if !isolated("engine::broker::settlement::release::tests::a_group_proof_and_the_records_agree_on_an_unresolved_cleanup") {
        return;
    }
    let gone = dead_group();
    let proven_group = Standing::from(grouped_whole(
        Witnessed::Unresolved,
        &[(Witnessed::Settling, Some(gone))],
    ));
    assert_eq!(
        proven(
            proven_group
                .whole
                .as_ref()
                .unwrap()
                .attempts
                .get("a1")
                .unwrap(),
            "a1"
        ),
        Ok(Proof::Group)
    );
    assert!(
        honoured(&proven_group, &settled("a", vec![grouped("a1", &[gone])])),
        "a group proof `proven` writes is honoured"
    );
    let proven_records = Standing::from(grouped_whole(
        Witnessed::Unresolved,
        &[(Witnessed::Settled, None)],
    ));
    assert_eq!(
        proven(
            proven_records
                .whole
                .as_ref()
                .unwrap()
                .attempts
                .get("a1")
                .unwrap(),
            "a1"
        ),
        Err(Live::Unproven {
            attempt: "a1".into(),
            why: UnprovenWhy::Unresolved,
        })
    );
    assert!(
        !honoured(&proven_records, &settled("a", vec![grouped("a1", &[])])),
        "a group proof `proven` refuses is not honoured"
    );
}

/// The record releases the standing reservation it names and covers: never
/// another run's, never a reservation with an attempt it left out, and
/// never one naming a group the host still shows alive.
#[test]
fn a_settle_record_releases_only_the_reservation_it_covers() {
    if !isolated("engine::broker::settlement::release::tests::a_settle_record_releases_only_the_reservation_it_covers") {
        return;
    }
    let gone = dead_group();
    let held = Standing::from(grouped_whole(
        Witnessed::Settling,
        &[(Witnessed::Settling, Some(gone))],
    ));
    assert!(
        honoured(&held, &settled("a", vec![grouped("a1", &[gone])])),
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
        !honoured(
            &proven_by_records,
            &settled("a", vec![grouped("a1", &[dead_group()])])
        ),
        "a group proof over a reservation with no unsettled driver proves nothing"
    );
    assert!(
        !honoured(&held, &settled("b", vec![grouped("a1", &[gone])])),
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
    // A record read only through another build's view names no group this
    // build can check against the broker's record, so no group proof stands
    // (S6).
    let viewed = viewed_standing("a", "settling");
    assert!(
        !honoured(&viewed, &settled("a", vec![grouped("a1", &[dead_group()])])),
        "a view-only record records no group the broker can name"
    );
    assert!(
        honoured(
            &viewed_standing("a", "settled"),
            &settled("a", vec![record_proven("a1")])
        ),
        "the view's own settled fact is a records proof"
    );
}

/// The contract says a record that names an attempt the reservation does
/// not name does not release: it is not a superset, it is a mismatch.
#[test]
fn a_settle_record_naming_an_unknown_attempt_does_not_release() {
    let gone = dead_group();
    let mut reservation = grouped_whole(Witnessed::Settling, &[(Witnessed::Settling, Some(gone))]);
    reservation.settled.insert("a9".into());
    let held = Standing::from(reservation);
    assert!(
        !honoured(
            &held,
            &settled(
                "a",
                vec![grouped("a1", &[gone]), grouped("a9", &[dead_group()])],
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
            why: UnprovenWhy::NoGroup,
        }),
        "the last driver's slot cannot prove a sibling"
    );
}
