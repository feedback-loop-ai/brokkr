//! Recovery from an engine's own sealed journal (U8a2a; D6, R2, M1): a
//! real dispatch, boxed to its worktree under `namespace`, runs its attempt
//! to its terminal event; its reservation is then put back as it stood
//! while the attempt ran, or as it stood once its engine sealed the
//! attempt's positive settlement fact, as a crash before the release
//! leaves it; and a fresh admission of another run on the same worktree
//! reads that very journal. Only the positive fact and a settled end, in a
//! journal no participant's writer could have written, release the
//! attempt; a missing fact, a moved head's end, or an end forged into a
//! journal a writer could reach keeps it, and the fresh run is refused. An
//! unresolved cleanup keeps it too: `unsettled`, beside the dispatches
//! whose drivers' output is held open.

use super::*;
use brokkr_core::realms::Boundary;

/// The attempt a dispatch ran on `host`, its reservation put back as it
/// stood before its release: the engine, its worktree, and the event its
/// attempt ended with.
struct Crashed {
    _dir: tempfile::TempDir,
    engine: Engine,
    work: PathBuf,
    ended: (EventType, serde_json::Value),
}

/// A dispatch on `host` of a work seat boxed by the hands `spec` names for
/// its worktree, under `namespace`, a gate where `gate`, whose driver
/// copies its run's reservation aside as it starts and then runs
/// `before`; once it ended, the reservation copied back over the one its
/// engine left, with the attempt's positive settlement fact sealed in it
/// where `proved`, as its engine seals it before the release
/// ([`Crashed`]).
fn crashed(host: &Host, spec: impl Fn(&Path) -> HandsSpec, gate: bool, before: &str) -> Crashed {
    crashed_as(host, spec, (gate, true), before)
}

/// [`crashed`], the positive fact sealed only where `proved`.
fn crashed_as(
    host: &Host,
    spec: impl Fn(&Path) -> HandsSpec,
    (gate, proved): (bool, bool),
    before: &str,
) -> Crashed {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let work = worktree(&engine);
    if gate {
        crate::engine::tests::git_commit(&work, "base");
    }
    let reservation = host.reservation(&work, &engine.run_id);
    let saved = dir.path().join("saved");
    let copied = format!(
        "cp '{}' '{}'; {before}",
        reservation.display(),
        saved.display()
    );
    engine.boundary = Boundary::Namespace;
    set_site_hands(&mut engine.bundle, "work", spec(&work));
    let seat = engine.bundle.seats.get_mut("work").unwrap();
    (seat.body, seat.has_gate) = (single_body(driver(&copied)), gate);
    let ended = executed(&mut engine);
    let mut stood: Reservation = serde_json::from_slice(&std::fs::read(&saved).unwrap()).unwrap();
    let fact = [Witnessed::Settling, Witnessed::Settled][usize::from(proved)];
    stood
        .attempts
        .values_mut()
        .for_each(|reserved| reserved.cleanup = fact);
    std::fs::write(&reservation, serde_json::to_vec(&stood).unwrap()).unwrap();
    Crashed {
        _dir: dir,
        engine,
        work,
        ended,
    }
}

/// Hands binding nothing: the box is its worktree alone.
fn confined(_: &Path) -> HandsSpec {
    hands(&[])
}

impl Crashed {
    /// A fresh run's admission on the crashed attempt's worktree, its one
    /// writer boxed to that worktree, nothing at stake.
    fn fresh(&self, host: &Host) -> Result<Held, Unadmitted> {
        let work = [boxed("work", &[&self.work])];
        host.admit(&claim(
            &self.work,
            ["run-b", "b1"],
            Protects::Nothing,
            &work,
        ))
    }

    /// The crashed run's settled attempts.
    fn settled(&self, host: &Host) -> Vec<String> {
        host.attempts(&self.work, &self.engine.run_id).1
    }

    /// The crashed attempt's end is `event`, and the fresh run is refused
    /// by the crashed run's lease, which was never certified.
    fn holds(&self, host: &Host, event: EventType) {
        let (ended, payload) = &self.ended;
        assert_eq!(*ended, event, "{payload}");
        let run = self.engine.run_id.clone();
        assert_eq!(self.fresh(host), cr2(Conflict::Held(run)));
        assert_eq!(self.settled(host), Vec::<String>::new(), "never certified");
    }
}

/// A genuine settled end in a journal outside every participant's reach,
/// beside the attempt's positive settlement fact, releases the attempt at
/// the next admission, which is admitted.
#[test]
fn a_settled_end_in_a_protected_journal_releases_its_attempt() {
    if !alone("recovery::a_settled_end_in_a_protected_journal_releases_its_attempt") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let crashed = crashed(&host, confined, false, ":");
    let (event, payload) = &crashed.ended;
    assert_eq!(*event, EventType::EffectSucceeded, "{payload}");
    let standing = host.attempts(&crashed.work, &crashed.engine.run_id).0;
    assert_eq!(standing.len(), 1, "standing as it ran");
    assert_eq!(crashed.fresh(&host).map(drop), Ok(()));
    assert_eq!(crashed.settled(&host), standing, "recovered settled");
}

/// M1: the same genuine settled end in the same protected journal proves
/// nothing where the reservation holds no positive settlement fact, as
/// one that crashed before its engine sealed it stands: the attempt keeps
/// its reservation, and the fresh run is refused.
#[test]
fn a_settled_end_without_its_positive_fact_keeps_its_attempt_reserved() {
    if !alone("recovery::a_settled_end_without_its_positive_fact_keeps_its_attempt_reserved") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let crashed = crashed_as(&host, confined, (false, false), ":");
    crashed.holds(&host, EventType::EffectSucceeded);
}

/// A gate that moved its head ends indeterminate, its cleanup settled and
/// its positive fact sealed: its engine released it, but a fresh admission
/// after the crash cannot certify an indeterminate end from the journal,
/// and is refused.
#[test]
fn a_moved_head_end_keeps_its_attempt_reserved() {
    if !alone("recovery::a_moved_head_end_keeps_its_attempt_reserved") {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let moves = "git commit -q --allow-empty -m gate-moved-head";
    let crashed = crashed(&host, confined, true, moves);
    let reason = crashed.ended.1["reason"].as_str().unwrap_or_default();
    assert!(
        reason.starts_with("GATE-MOVED-HEAD "),
        "{}",
        crashed.ended.1
    );
    crashed.holds(&host, EventType::EffectIndeterminate);
}

/// M1, R2: a certified end forged into the attempt's actual sealed
/// journal, which its writer's box can reach, is no proof. The box binds
/// the directory that holds the worktree and the journal; the attempt's
/// gate moved its head, so its genuine end is indeterminate; its positive
/// fact is sealed; and then a success for it is appended to that very
/// journal, as a writer able to reach it could forge the unkeyed hash
/// chain. The attempt keeps its reservation, and the fresh run is refused.
#[test]
fn a_forged_end_in_a_journal_its_writer_could_reach_keeps_its_attempt_reserved() {
    if !alone(
        "recovery::a_forged_end_in_a_journal_its_writer_could_reach_keeps_its_attempt_reserved",
    ) {
        return;
    }
    let host = Host::new();
    let _home = Coordinated::at(&host.home);
    let holder = |work: &Path| hands(&[work.parent().unwrap()]);
    let moves = "git commit -q --allow-empty -m gate-moved-head";
    let mut crashed = crashed(&host, holder, true, moves);
    let run = crashed.engine.run_id.clone();
    let attempt = host.attempts(&crashed.work, &run).0.remove(0);
    let forged = json!({"effect_id": "forged", "attempt_id": attempt, "result": {"result": "x"}});
    let store = &mut crashed.engine.store;
    let id = Some(attempt.clone());
    (store.append_next(&run, EventType::EffectSucceeded, forged, None, id)).unwrap();
    crashed.holds(&host, EventType::EffectIndeterminate);
}
