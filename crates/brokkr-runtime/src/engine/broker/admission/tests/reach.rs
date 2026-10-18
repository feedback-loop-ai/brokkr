//! The reach an attempt's boxes are built from (decision 0065 slice two,
//! U8a2c; CR2, MB3): the one its admission sealed for each boxed writer,
//! held for the attempt under a lease or holding none, which the engine
//! composes every box of those hands from and carries to hands as a typed
//! fact; a box whose hands it sealed nothing for refuses.

use brokkr_core::realms::Boundary;

use super::*;
use crate::bundle::{Aggregate, Invocation};
use crate::engine::boxed::Unnamed;

/// An exec seat's command, which `hands exec` boxes whole.
fn exec_seat() -> Vec<String> {
    ["/usr/local/bin/brokkr", "driver", "exec", "--", "true"]
        .map(String::from)
        .to_vec()
}

/// A fresh engine whose single work seat is boxed under `namespace` by
/// hands binding `pad` in its worktree, absent: its directory, the engine,
/// the hands, and `pad`.
fn boxed_seat() -> (tempfile::TempDir, Engine, HandsSpec, PathBuf) {
    let (dir, mut engine) = engine(single_body(exec_seat()));
    let pad = worktree(&engine).join("pad");
    let spec = hands(&[&pad]);
    set_site_hands(&mut engine.bundle, "work", spec.clone());
    engine.boundary = Boundary::Namespace;
    (dir, engine, spec, pad)
}

/// The work seat's attempt `attempt` admitted under `coordination`.
fn admitted(engine: &mut Engine, coordination: Coordination<'_>) -> bool {
    let seat = engine.bundle.seats["work"].clone();
    let (body, _) = seat.body.selected(None).unwrap();
    let ids = ["effect", "attempt", "work"];
    let admitted = engine.admit_in(coordination, ids, body, &Selection::new(), soon());
    admitted.unwrap()
}

/// The reach `--reach` carries in `argv`.
fn carried(argv: &[String]) -> String {
    let at = argv
        .iter()
        .position(|part| part == "--reach")
        .expect("a carried reach");
    argv[at + 1].clone()
}

/// A boxed seat's box is built from the reach its admission sealed, under
/// a lease (the reach its reservation holds) or holding none (C1): its
/// spawn, composed once it was admitted, carries that very reach to `hands
/// exec`, though a source absent then stands now and a reach sealed now
/// would differ; hands then refuses the box by that source.
#[test]
fn a_seats_box_is_built_from_the_reach_its_admission_sealed() {
    let host = Host::new();
    for leased in [true, false] {
        let (_dir, mut engine, spec, pad) = boxed_seat();
        let root = || leased.then(|| opened(&host.home));
        assert!(
            admitted(&mut engine, (host.home.clone(), &root)),
            "{leased}"
        );
        let held = engine
            .reach
            .sealed(&spec)
            .cloned()
            .expect("sealed at admission");
        let at = host.reservation(&worktree(&engine), &engine.run_id);
        if leased {
            let read: Reservation = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
            let Reach::Boxed { sealed, .. } = &read.attempts["attempt"].writers[0].reach else {
                panic!("a boxed writer")
            };
            assert_eq!(**sealed, held, "the reach its reservation holds");
        } else {
            assert!(!at.exists(), "no reservation was taken");
        }
        std::fs::create_dir(&pad).unwrap();
        let work = worktree(&engine);
        let now = hands::seal(&spec, &host.home, &work, &hands::git_facts(&work));
        assert_ne!(now, held, "a reach sealed now differs");
        let spawn = engine.compose_at(
            Some("work"),
            "attempt",
            true,
            exec_seat(),
            Some(&spec),
            None,
            "",
        );
        assert_eq!(spawn.refusal, None, "{leased}");
        assert_eq!(
            carried(&spawn.argv),
            serde_json::to_string(&held).unwrap(),
            "{leased}"
        );
        assert_eq!(
            hands::drifted(&held, &spec, &work),
            Some(pad.clone()),
            "{leased}"
        );
    }
}

/// A box whose hands its admission sealed no reach for, as no admitted
/// attempt's are, refuses its spawn with MB3's identity cause: nothing
/// derives a reach for it again.
#[test]
fn a_box_whose_hands_were_sealed_nothing_refuses_its_spawn() {
    let (_dir, mut engine, spec, _) = boxed_seat();
    let spawn = engine.compose_at(
        Some("work"),
        "attempt",
        true,
        exec_seat(),
        Some(&spec),
        None,
        "",
    );
    let text = "dispatch refused: MCP server box filesystem identity is not protected: the box's \
                reach was not sealed at admission, or is not UTF-8, so no box of these hands is \
                built from it (U8a2c)";
    assert_eq!(Unnamed::Unsealed.to_string(), text);
    assert_eq!(spawn.refusal.as_deref(), Some(text));
    let other = hands(&[&worktree(&engine).join("other")]);
    engine.sealed_for(Path::new("/home/fixture"), &other);
    let spawn = engine.compose_at(
        Some("work"),
        "attempt",
        true,
        exec_seat(),
        Some(&spec),
        None,
        "",
    );
    assert_eq!(spawn.refusal.as_deref(), Some(text), "another hands' reach");
}

/// A panel member or a step of the ship phase is composed from its site's
/// own hands, which the phase's journal bind widens for the reservation:
/// admission seals both, so every box the attempt composes is built from a
/// reach it sealed.
#[test]
fn a_ship_members_own_hands_are_sealed_beside_the_widened_ones() {
    let members = vec![member("m", exec_seat())];
    let aggregate = Aggregate::UnanimousPass;
    let body = SeatBody::Invocation(Invocation::Panel { members, aggregate });
    let (_dir, mut engine) = engine(body.clone());
    let own = hands(&[&worktree(&engine).join("pad")]);
    set_site_hands(&mut engine.bundle, "ship:m", own.clone());
    engine.boundary = Boundary::Namespace;
    let widened = engine.runtime_hands("ship:m").unwrap();
    assert_ne!(widened, own, "the journal lies outside the worktree");
    let (body, _) = body.selected(None).unwrap();
    let work = worktree(&engine);
    let (writers, boxes) = engine.writers(Path::new("/home/fixture"), &work, "ship", body);
    let specs: Vec<&HandsSpec> = boxes.iter().map(|(spec, _)| spec).collect();
    assert_eq!(specs, [&own, &widened]);
    assert_eq!(writers.len(), 1, "one writer, its reach the widened one");
    assert_eq!(writers[0].reach, boxes[1].1);
}
