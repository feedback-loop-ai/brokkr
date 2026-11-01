//! The coordination comparison (U8a2a; D6 and the operator's ruling of
//! 2026-10-18): every participant's writers held, in both directions, to
//! the coordination root and every protected or historical artifact root,
//! as the fence and the evidence at stake ask; historical roots outliving
//! settlement and verified where admission relies on them; and proven
//! attempts recovered from durable facts alone.
//!
//! A boxed writer admitted beside evidence passes the mount table's
//! judgement, which only Linux reads (decision 0063), so every test that
//! admits one is Linux only.

// The bind-alias proof (Linux only) and the FIFO helper shell out.
use std::process::Command;

use super::*;
use crate::capabilities::McpFence;
#[cfg(target_os = "linux")]
use crate::engine::broker::exclusion::{self, Shown};
use crate::engine::broker::exclusion::{compared, covers_spelled, Comparison};
use crate::engine::broker::historical;
use crate::engine::broker::sources::records_kept;

/// The artifact root of a [`claim`] on `worktree`.
fn artifacts(worktree: &Path) -> PathBuf {
    worktree.join(".forge/artifacts")
}

/// `outcome` without its lease, compared with `refused`, or admitted.
fn judged(outcome: Result<Held, Unadmitted>, refused: Option<Conflict>) {
    let expected = refused.map_or(Ok(()), |conflict| Err(Unadmitted::Evidence(conflict)));
    assert_eq!(outcome.map(drop), expected);
}

/// The fence standing or lifted, a zero-grant writer outside any box is
/// admitted where nothing is at stake and refused where something is (operator
/// ruling of 2026-10-29 on U9b1's fourth run): the incoming attempt protects
/// evidence, a standing attempt does, or only a historical root remains. A box
/// whose writable reach includes the coordination root is refused in all four
/// by MB3's identity cause naming that root (operator ruling of 2026-10-30),
/// none of it sealed; a box kept apart from the root is admitted.
#[cfg(target_os = "linux")]
#[test]
fn either_fence_refuses_unboxed_writers_at_stake_and_root_reaching_boxes_on_every_run() {
    for lifted in [false, true] {
        let _lift = lifted.then(McpFence::lift);
        at_stake_only();
    }
}

/// [`either_fence_refuses_unboxed_writers_at_stake_and_root_reaching_boxes_on_every_run`]
/// under the fence this thread stands under.
#[cfg(target_os = "linux")]
fn at_stake_only() {
    let host = Host::new();
    let (work, kept) = (host.worktree("work"), host.worktree("kept"));
    let writers = [
        unbounded("work"),
        boxed("work", &[&work, &host.home]),
        boxed("work", &[&work]),
    ];
    // The attempt `index` of `run` on `work`, its one writer `writers`'s.
    let at = |run: &str, index: usize, protects: Protects| {
        let attempt = format!("{run}-{index}");
        let writer = std::slice::from_ref(&writers[index]);
        claim(&work, [run, &attempt], protects, writer)
    };
    // The refusals of `run`'s two writers no comparison can keep apart.
    let refused = |run: &str, protects: Protects| {
        let (incoming, reaching) = (at(run, 0, protects), at(run, 1, protects));
        judged(host.admit(&incoming), Some(unboxed("work", run)));
        assert_eq!(host.admit(&reaching), exposed("work", run, &host.root()));
    };
    freed(&host.admit(&at("run-n", 0, Protects::Nothing)).unwrap());
    freed(&host.admit(&at("run-a", 2, Protects::Nothing)).unwrap());
    assert_eq!(
        host.admit(&at("run-b", 1, Protects::Nothing)),
        exposed("work", "run-b", &host.root())
    );
    assert!(!host.reservation(&work, "run-b").exists());
    refused("run-e", Protects::Evidence);
    let standing = host.admit(&guarded(&kept, ["run-k", "k1"])).unwrap();
    refused("run-s", Protects::Nothing);
    freed(&standing);
    refused("run-h", Protects::Nothing);
}

/// The check an attempt holding no lease meets, on every host: a boxed writer
/// is refused by MB3's identity cause where it reaches the root's path, which
/// need not stand, at it or above it, kept where it lies apart, a sibling of
/// the root under its HOME included; and where the host names no HOME, so the
/// root's path is not absolute, no boxed writer is proved kept, whether the
/// path leads nowhere or resolves against the working directory. A writer
/// Brokkr cannot box is compared with nothing.
#[test]
fn an_unopened_roots_path_keeps_boxed_writers_on_every_host() {
    let host = Host::new();
    let work = host.worktree("work");
    let home = host.dir.path().join("unopened");
    let beside = home.join("beside");
    std::fs::create_dir_all(&beside).unwrap();
    let records = coordination(&home);
    let budget = Budget::new(soon());
    let of = |root: &Path| {
        claim(
            &work,
            ["run-w", "w1"],
            Protects::Nothing,
            &[boxed("work", &[root])],
        )
    };
    let keep = |claim: &Claim, root: &Path| records_kept(claim, root, shown, &budget);
    for reach in [
        &records,
        &home,
        &records.join("below"),
        &host.dir.path().to_path_buf(),
    ] {
        let refused = exposing("work", "run-w", &records);
        assert_eq!(keep(&of(reach), &records), Err(refused), "{reach:?}");
        assert!(covers_spelled(reach, &records), "{reach:?}");
    }
    // The comparison alone, which is all that decides off Linux: a HOME
    // spelled through a link leads to the same root, its sibling does not.
    let link = host.dir.path().join("link");
    std::os::unix::fs::symlink(&home, &link).unwrap();
    assert!(covers_spelled(&home, &coordination(&link)));
    assert!(covers_spelled(&coordination(&link).join("below"), &records));
    assert!(!covers_spelled(&beside, &coordination(&link)));
    assert!(!covers_spelled(&beside, &records));
    assert_eq!(keep(&of(&beside), &records), Ok(()));
    assert_eq!(keep(&open(&work, ["run-w", "w1"]), &records), Ok(()));
    for spelled in [Path::new(""), Path::new(".")] {
        let nameless = coordination(spelled);
        let refused = keep(&of(&work), &nameless);
        assert_eq!(refused, Err(Unknown::Homeless), "{spelled:?}");
        assert_eq!(keep(&open(&work, ["run-w", "w1"]), &nameless), Ok(()));
    }
}

/// R1: a writer whose bind source lies beneath a writable root is refused
/// before spawn, because that root's owner could replace the source with a
/// symlink and redirect the box when hands rebuilds it from the authored
/// sources; a source outside every writable root is admitted. The refusal
/// applies where evidence is at stake, so the attempts protect it here.
/// Linux only, as it admits a boxed writer beside the mount table's
/// judgement (decision 0063).
#[cfg(target_os = "linux")]
#[test]
fn a_replaceable_bind_source_is_refused_before_spawn() {
    let host = Host::new();
    let (work, apart) = (host.worktree("work"), host.worktree("apart"));
    // A read-only source beneath the writable worktree: replaceable.
    let inside = boxed_binding("work", &[&work], &[&work.join("src")]);
    let refused = claim(&work, ["run-a", "a0"], Protects::Evidence, &[inside]);
    assert_eq!(
        host.admit(&refused),
        cr2(Conflict::Replaceable {
            site: "work".into(),
            run: "run-a".into(),
            bind: work.join("src"),
        })
    );
    // A disjoint source outside every writable root: admitted.
    let outside = boxed_binding("work", &[&work], &[&apart]);
    let admitted = claim(&work, ["run-a", "a1"], Protects::Evidence, &[outside]);
    judged(host.admit(&admitted), None);
}

/// A run's earlier attempt not proven settled is still a participant: the
/// run's own lease admits its next attempt, but that attempt's evidence is
/// held from the earlier one's writer outside any box (H2).
#[test]
fn a_runs_earlier_unsettled_attempt_is_compared_with_its_next() {
    let host = Host::new();
    let work = host.worktree("work");
    drop(host.admit(&open(&work, ["run-a", "a1"])).unwrap());
    let next = guarded(&work, ["run-a", "a2"]);
    assert_eq!(host.admit(&next), cr2(unboxed("work", "run-a")));
}

/// A run is its worktree's key and its id: an unrelated worktree reusing a
/// run's id holds no lease of it, and its evidence is compared with that
/// run's standing writer, whose bind of it refuses (H2).
#[test]
fn a_run_id_reused_in_another_worktree_is_compared_not_exempted() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let bind = [boxed("work", &[&kept, &other.join(".forge")])];
    let standing = claim(&kept, ["same", "k1"], Protects::Nothing, &bind);
    let _held = host.admit(&standing).unwrap();
    let reused = guarded(&other, ["same", "o1"]);
    let refused = reaches("work", "same", &artifacts(&other));
    assert_eq!(host.admit(&reused), cr2(refused));
    let apart = claim(&other, ["same", "o2"], Protects::Nothing, &[]);
    judged(host.admit(&apart), None);
}

/// Evidence stays protected once every attempt that wrote it has settled
/// (H3): its root's record outlives the release, and a later writer of
/// another worktree that can reach it refuses.
#[cfg(target_os = "linux")]
#[test]
fn settled_evidence_stays_protected_from_a_later_worktree() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    freed(&host.admit(&guarded(&kept, ["run-a", "a1"])).unwrap());
    let repo = canonical::sha256_bytes(kept.as_os_str().as_bytes());
    let record = host
        .root()
        .join(repo)
        .join(historical::name(&artifacts(&kept)));
    assert!(record.exists(), "the record outlives the release");
    let bind = [boxed("work", &[&other, &kept.join(".forge")])];
    let later = claim(&other, ["run-b", "b1"], Protects::Nothing, &bind);
    assert_eq!(
        host.admit(&later),
        cr2(reaches("work", "run-b", &artifacts(&kept)))
    );
}

/// A writer that can reach protected evidence is refused, whichever way it
/// is spelled (H4): through a bind of its parent, outside any box, or through
/// a symlink, by CR2's cause; through a HOME bind that reaches the coordination
/// root, by MB3's identity cause at the door (operator ruling of 2026-10-30);
/// a box kept to its own worktree is admitted beside it.
#[cfg(target_os = "linux")]
#[test]
fn a_writer_that_can_reach_protected_evidence_is_refused() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let _held = host.admit(&guarded(&kept, ["run-a", "a1"])).unwrap();
    let alias = other.join("alias");
    std::os::unix::fs::symlink(kept.join(".forge"), &alias).unwrap();
    let evidence = artifacts(&kept);
    let writers = [
        (
            boxed("work:bind", &[&other, &kept.join(".forge")]),
            Some(reaches("work:bind", "run-b", &evidence)),
        ),
        (unbounded("work:open"), Some(unboxed("work:open", "run-b"))),
        (
            boxed("work:link", &[&other, &alias]),
            Some(reaches("work:link", "run-b", &evidence)),
        ),
        (boxed("work:kept", &[&other]), None),
    ];
    for (index, (writer, refused)) in writers.into_iter().enumerate() {
        let attempt = format!("b{index}");
        let incoming = claim(&other, ["run-b", &attempt], Protects::Nothing, &[writer]);
        judged(host.admit(&incoming), refused);
    }
    let home = [boxed("work:home", &[&other, &host.home])];
    let incoming = claim(&other, ["run-b", "home"], Protects::Nothing, &home);
    assert_eq!(
        host.admit(&incoming),
        exposed("work:home", "run-b", &host.root())
    );
}

/// A protected root a reservation spells through a symlink is still the
/// root a writer's canonical bind reaches: their identities meet where
/// their paths do not (H4).
#[cfg(target_os = "linux")]
#[test]
fn a_root_spelled_through_a_symlink_is_compared_by_identity() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let spelled = host.dir.path().join("spelled");
    std::os::unix::fs::symlink(&kept, &spelled).unwrap();
    let _held = host.admit(&guarded(&spelled, ["run-a", "a1"])).unwrap();
    let bind = [boxed("work", &[&other, &kept.join(".forge")])];
    let incoming = claim(&other, ["run-b", "b1"], Protects::Nothing, &bind);
    let refused = reaches("work", "run-b", &artifacts(&spelled));
    assert_eq!(host.admit(&incoming), cr2(refused));
}

/// A run that would protect evidence is refused beside a running writer of
/// another worktree outside any box, or one whose box can write the
/// coordination root, and admitted beside one kept to its own box. The second
/// is planted, as no such writer is admitted now: a record an earlier build
/// sealed is still compared.
#[cfg(target_os = "linux")]
#[test]
fn a_protecting_run_is_refused_beside_a_writer_that_can_reach_its_root() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let standing = host.admit(&open(&other, ["run-s", "s1"])).unwrap();
    let protecting = guarded(&kept, ["run-a", "a1"]);
    assert_eq!(host.admit(&protecting), cr2(unboxed("work", "run-s")));
    freed(&standing);
    let apart = [boxed("work:home", &[&other])];
    let standing = host.admit(&claim(&other, ["run-s", "s2"], Protects::Nothing, &apart));
    let home = vec![boxed("work:home", &[&other, &host.home])];
    planted_writers(&host, &other, ["run-s", "s2"], home);
    let refused = reaches("work:home", "run-s", &host.root());
    assert_eq!(host.admit(&protecting), cr2(refused));
    freed(&standing.unwrap());
    let confined = [boxed("work", &[&other])];
    let standing = claim(&other, ["run-s", "s3"], Protects::Nothing, &confined);
    let _standing = host.admit(&standing).unwrap();
    let admitted = host.admit(&protecting);
    assert!(admitted.is_ok_and(|held| held.root.is_some()));
}

/// A protecting attempt refuses its own writers that cannot be kept from
/// its evidence or the coordination root: one outside any box, by CR2's cause,
/// and one whose box can write the coordination root, by MB3's identity cause.
#[test]
fn a_protecting_attempt_refuses_its_own_unprotectable_writers() {
    let host = Host::new();
    let work = host.worktree("work");
    let unboxed_in = [boxed("work:a", &[&work]), unbounded("work:b")];
    let protecting = claim(&work, ["run-a", "a1"], Protects::Evidence, &unboxed_in);
    assert_eq!(host.admit(&protecting), cr2(unboxed("work:b", "run-a")));
    let home = [
        boxed("work:a", &[&work]),
        boxed("work:b", &[&work, &host.home]),
    ];
    let protecting = claim(&work, ["run-a", "a1"], Protects::Evidence, &home);
    assert_eq!(
        host.admit(&protecting),
        exposed("work:b", "run-a", &host.root())
    );
    // Neither refused attempt sealed anything.
    assert!(!host.reservation(&work, "run-a").exists());
}

/// Where admission relies on historical evidence, every artifact it
/// consults there is one file of one link whose bytes digest to its name
/// (M10): a valid artifact admits, and one whose bytes differ, one whose
/// name is no digest, a link and a hard link each refuse by CR2's cause.
#[test]
fn historical_artifacts_are_verified_against_their_names() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let recorded = claim(&kept, ["run-a", "a1"], Protects::Evidence, &[]);
    freed(&host.admit(&recorded).unwrap());
    let digests = artifacts(&kept).join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    let valid = digests.join(canonical::sha256_bytes(b"evidence"));
    std::fs::write(&valid, "evidence").unwrap();
    let incoming = |attempt: &str| claim(&other, ["run-b", attempt], Protects::Nothing, &[]);
    judged(host.admit(&incoming("b0")), None);
    let forged = digests.join(canonical::sha256_bytes(b"other"));
    let (named, linked) = (
        digests.join("notes"),
        digests.join(canonical::sha256_bytes(b"l")),
    );
    type Plant<'a> = Box<dyn Fn() + 'a>;
    let cases: [(Plant<'_>, PathBuf, PathBuf); 4] = [
        (
            Box::new(|| std::fs::write(&forged, "evidence").unwrap()),
            forged.clone(),
            forged.clone(),
        ),
        (
            Box::new(|| std::fs::write(&named, "notes").unwrap()),
            named.clone(),
            named.clone(),
        ),
        (
            Box::new(|| std::os::unix::fs::symlink(&valid, &linked).unwrap()),
            linked.clone(),
            linked.clone(),
        ),
        (
            Box::new(|| std::fs::hard_link(&valid, other.join("linked")).unwrap()),
            valid.clone(),
            other.join("linked"),
        ),
    ];
    for (index, (plant, refused, planted)) in cases.into_iter().enumerate() {
        plant();
        let outcome = host.admit(&incoming(&format!("b{}", index + 1)));
        judged(outcome, Some(Conflict::Corrupt(refused)));
        std::fs::remove_file(planted).unwrap();
    }
    judged(host.admit(&incoming("b9")), None);
    assert_eq!(std::fs::read(&valid).unwrap(), b"evidence");
}

/// R3: a FIFO planted among the digests refuses with CR2's cause at once,
/// never blocking the admission on an open that waits for a writer.
#[test]
fn a_fifo_artifact_refuses_without_blocking() {
    let host = Host::new();
    let dir = host.worktree("root");
    let digests = dir.join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    let fifo = digests.join(canonical::sha256_bytes(b"x"));
    made_fifo(&fifo);
    let mut budget = Budget::new(soon());
    assert_eq!(
        historical::verified(&dir, &mut budget),
        Err(Unadmitted::Evidence(Conflict::Corrupt(fifo)))
    );
}

/// A FIFO made at `at` by the POSIX `mkfifo` utility, on every platform
/// this suite runs on: rustix offers `mknodat` on Linux only.
fn made_fifo(at: &Path) {
    let made = Command::new("mkfifo").arg(at).status();
    assert!(
        made.is_ok_and(|made| made.success()),
        "mkfifo {}",
        at.display()
    );
}

/// A historical root whose digests cannot be looked at, as where the root
/// is no directory, refuses with CR2's cause.
#[test]
fn a_historical_root_that_is_no_directory_refuses() {
    let host = Host::new();
    let root = host.dir.path().join("root");
    std::fs::write(&root, b"x").unwrap();
    assert_eq!(
        historical::verified(&root, &mut Budget::new(soon())),
        Err(Unadmitted::Evidence(Conflict::Corrupt(root.join("sha256"))))
    );
}

/// A historical record staged beside a worktree's records and never
/// renamed into place, as a seal that died before its commit leaves one,
/// is no unknown entry: a later admission's inventory passes over it, as
/// it refuses any other file there.
#[test]
fn a_staged_historical_record_is_passed_over_by_the_inventory() {
    let host = Host::new();
    let (kept, other) = (host.worktree("kept"), host.worktree("other"));
    let _held = host.admit(&claim(&kept, ["run-a", "a1"], Protects::Nothing, &[]));
    let repo = canonical::sha256_bytes(kept.as_os_str().as_bytes());
    let repo = host.root().join(repo);
    let staged = format!("{}{STAGED}", historical::name(&artifacts(&kept)));
    std::fs::write(repo.join(staged), b"{").unwrap();
    let later = |attempt: &str| claim(&other, ["run-b", attempt], Protects::Nothing, &[]);
    std::fs::write(repo.join("unknown"), b"{").unwrap();
    let refused = host.admit(&later("b1"));
    assert!(
        matches!(refused, Err(Unadmitted::Identity(_))),
        "{refused:?}"
    );
    std::fs::remove_file(repo.join("unknown")).unwrap();
    judged(host.admit(&later("b2")), None);
}

/// M2; R2 (U8a2c): a historical artifact is read and hashed a chunk at a
/// time, the shared deadline checked once each chunk is hashed, so the
/// lock is never held past it by one artifact however large, reading or
/// hashing it: a deadline that passes inside its first chunk ends the
/// verification by MB3's identity cause with that chunk hashed and no
/// further chunk read; within its deadline the same artifact verifies,
/// each chunk hashed as it is read, to its end: the digest handed on after
/// each chunk is the digest of every byte read so far.
#[test]
fn a_deadline_that_passes_inside_an_artifacts_read_ends_its_verification() {
    use sha2::Digest;
    let host = Host::new();
    let root = host.worktree("root");
    let chunk = 64 << 10;
    let bytes: Vec<u8> = (0..3 * chunk).map(|at| (at % 251) as u8).collect();
    let digests = root.join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    std::fs::write(digests.join(canonical::sha256_bytes(&bytes)), &bytes).unwrap();
    let hashed = RefCell::new(Vec::new());
    let read = |hasher: &sha2::Sha256| format!("{:x}", hasher.clone().finalize());
    let spent = |hasher: &sha2::Sha256| {
        hashed.borrow_mut().push(read(hasher));
        std::thread::sleep(Duration::from_millis(300));
    };
    let mut budget = Budget::new(Instant::now() + Duration::from_millis(200));
    let verified = historical::verified_with(&root, &mut budget, &spent);
    assert_eq!(verified, Err(Unadmitted::Identity(Unknown::Expired)));
    let prefix = |chunks: usize| canonical::sha256_bytes(&bytes[..chunks * chunk]);
    assert_eq!(
        *hashed.borrow(),
        [prefix(1)],
        "one chunk hashed, none read past the deadline"
    );
    hashed.borrow_mut().clear();
    let counted = |hasher: &sha2::Sha256| hashed.borrow_mut().push(read(hasher));
    let verified = historical::verified_with(&root, &mut Budget::new(soon()), &counted);
    let each = [prefix(1), prefix(2), prefix(3), prefix(3)];
    assert_eq!(
        (verified, hashed.take()),
        (Ok(()), each.to_vec()),
        "three chunks and the end"
    );
}

/// M1 (run four's second review): no artifact retention publishes is
/// larger than MB3's 8 MiB per response and canonical artifact, so one of
/// exactly 8 MiB verifies, and one a byte over is refused by CR2's cause
/// even where its name is its bytes' digest, read no further than that one
/// byte.
#[test]
fn an_artifact_past_the_artifact_bound_is_refused_though_its_name_digests_it() {
    let host = Host::new();
    let root = host.worktree("root");
    let digests = root.join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    let allowance = 8 << 20;
    let exact = vec![1_u8; allowance];
    std::fs::write(digests.join(canonical::sha256_bytes(&exact)), &exact).unwrap();
    assert_eq!(
        historical::verified(&root, &mut Budget::new(soon())),
        Ok(())
    );
    let over = vec![2_u8; allowance + 1];
    let at = digests.join(canonical::sha256_bytes(&over));
    std::fs::write(&at, &over).unwrap();
    // Whatever order the directory lists them in, the refusal is the
    // one-over artifact's.
    assert_eq!(
        historical::verified(&root, &mut Budget::new(soon())),
        Err(Unadmitted::Evidence(Conflict::Corrupt(at)))
    );
}

/// L5: recovery opens a reservation's journal only where it is a regular
/// file of one link, opened without following a link or blocking: a FIFO
/// in its place, which SQLite's own open would wait on past every deadline,
/// and a link to a journal that would certify the attempt's end, each
/// prove nothing at once, and the attempt keeps its reservation.
#[test]
fn a_journal_that_is_no_regular_file_proves_nothing_at_once() {
    let host = Host::new();
    let (_dir, mut engine) = engine(single_body(Vec::new()));
    let run = engine.run_id.clone();
    settled_a1(&mut engine, &run);
    let fifo = host.dir.path().join("fifo.db");
    made_fifo(&fifo);
    let link = host.dir.path().join("link.db");
    std::os::unix::fs::symlink(engine.store.path(), &link).unwrap();
    for (name, journal) in [("link", link), ("fifo", fifo)] {
        survives(&host, name, [&run, "a1"], journal);
    }
}

/// A run of `host` that died on the worktree `name` once its attempt
/// `attempt`'s positive settlement fact was sealed, its reservation naming
/// `journal`, holds that worktree: a later run there is refused by its
/// lease, and the reservation stands byte for byte as it was sealed.
fn survives(host: &Host, name: &str, [run, attempt]: [&str; 2], journal: PathBuf) {
    let work = host.worktree(name);
    let mut died = claim(&work, [run, attempt], Protects::Nothing, &[]);
    died.journal = journal;
    died_proved(host, &died);
    let sealed = std::fs::read(host.reservation(&work, run)).unwrap();
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    let held = cr2(Conflict::Held(run.to_string()));
    assert_eq!(host.admit(&next), held, "{name}");
    let kept = std::fs::read(host.reservation(&work, run)).unwrap();
    assert_eq!(kept, sealed, "{name}");
}

/// Every step admission takes under its lock spends the one deadline it
/// shares with provisioning, and D5's entry bound (M8, M10): the
/// comparison of a writer, the recovery of a reservation and the
/// verification of an artifact each refuse once the deadline passed, and
/// the verification once the bound is spent.
#[test]
fn every_step_under_the_lock_spends_the_shared_deadline() {
    let host = Host::new();
    let work = host.worktree("work");
    let incoming = claim(
        &work,
        ["run-a", "a1"],
        Protects::Evidence,
        &[unbounded("work")],
    );
    let comparison = Comparison {
        claim: &incoming,
        standing: &[],
        historical: &[],
        coordination: &host.root,
    };
    let expired = || Budget::new(Instant::now());
    let unknown = |unknown| Err(Unadmitted::Identity(unknown));
    assert_eq!(
        compared(&comparison, &mut expired()),
        unknown(Unknown::Expired)
    );
    let held = host
        .admit(&claim(&work, ["run-a", "a1"], Protects::Nothing, &[]))
        .unwrap();
    let bytes = std::fs::read(host.reservation(&work, "run-a")).unwrap();
    let whole: Reservation = serde_json::from_slice(&bytes).unwrap();
    let standing = vec![Standing::from(whole)];
    let recovered =
        super::super::super::settlement::recovered(&host.root, standing, &[], &expired());
    assert_eq!(recovered.map(drop), Err(Unknown::Expired));
    let digests = artifacts(&work).join("sha256");
    std::fs::create_dir_all(&digests).unwrap();
    for bytes in [&b"one"[..], b"two"] {
        std::fs::write(digests.join(canonical::sha256_bytes(bytes)), bytes).unwrap();
    }
    let verified = |mut budget: Budget| historical::verified(&artifacts(&work), &mut budget);
    assert_eq!(verified(expired()), unknown(Unknown::Expired));
    let one = Budget {
        entries: 1,
        deadline: soon(),
    };
    assert_eq!(verified(one), unknown(Unknown::Bounds));
    let two = Budget {
        entries: 2,
        deadline: soon(),
    };
    assert_eq!(verified(two), Ok(()));
    freed(&held);
}

/// An engine that died after its attempt's certified end and its positive
/// settlement fact leaves only durable facts, which the next admission
/// reads (M7, M1): the journal its reservation names, read only, shows the
/// attempt succeeded, so the attempt is released under the lock and the
/// next run is admitted.
#[test]
fn a_crash_after_a_certified_end_is_recovered_at_the_next_admission() {
    let host = Host::new();
    let (_dir, _engine, work, run) = died_after_its_end(&host);
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    judged(host.admit(&next), None);
    assert_eq!(host.attempts(&work, &run), (vec![], vec!["a1".into()]));
}

/// On `host`, a run of a fresh engine that died on the worktree `work`
/// after its attempt `a1`'s certified end, its positive fact sealed and its
/// reservation naming that engine's own journal, as a crash after both
/// leaves it: the engine's directory, the engine, the worktree and the run.
pub(super) fn died_after_its_end(host: &Host) -> (tempfile::TempDir, Engine, PathBuf, String) {
    let (dir, mut engine) = engine(single_body(Vec::new()));
    let (work, run) = (host.worktree("work"), engine.run_id.clone());
    let mut died = claim(&work, [&run, "a1"], Protects::Nothing, &[]);
    died.journal = std::fs::canonicalize(engine.store.path()).unwrap();
    died_proved(host, &died);
    settled_a1(&mut engine, &run);
    (dir, engine, work, run)
}

/// `died` admitted on `host`, its positive settlement fact sealed, and its
/// engine gone before the release.
pub(super) fn died_proved(host: &Host, died: &Claim) {
    let held = host.admit(died).unwrap();
    assert_eq!(proved(&held, soon()), Ok(Commit::Synced));
}

/// Append a genuine settled end for attempt `a1` of `run` to `engine`'s
/// journal, as a crash leaves behind after the terminal event.
pub(super) fn settled_a1(engine: &mut Engine, run: &str) {
    let payload = json!({"effect_id": "e", "attempt_id": "a1", "result": {"result": "complete"}});
    let appended = engine.store.append_next(
        run,
        EventType::EffectSucceeded,
        payload,
        None,
        Some("a1".into()),
    );
    appended.unwrap();
}

/// R2; M1 and R2 of run four's review: a reservation whose journal stands
/// inside a writer's reach is not trusted, because that writer could forge
/// its unkeyed hash chain. No attempt ever ran here: the attempt's
/// positive fact is sealed, and a settled end for it is forged into the
/// engine's actual sealed journal, the very journal its reservation names,
/// as a writer reaching it could append one. It does not release the
/// reservation, which keeps refusing; a journal outside every writer's
/// reach releases
/// (`a_crash_after_a_certified_end_is_recovered_at_the_next_admission`).
#[test]
fn an_end_forged_into_a_reachable_journal_with_no_attempt_behind_it_keeps_its_reservation() {
    let host = Host::new();
    let (_dir, mut engine) = engine(single_body(Vec::new()));
    let (work, run) = (host.worktree("work"), engine.run_id.clone());
    let journal = std::fs::canonicalize(engine.store.path()).unwrap();
    // The died run's own writer boxes the directory that holds its
    // journal, so it could have forged any end written there.
    let jdir = journal.parent().unwrap().to_path_buf();
    let mut died = claim(
        &work,
        [&run, "a1"],
        Protects::Nothing,
        &[boxed("work", &[&jdir])],
    );
    died.journal = journal;
    died_proved(&host, &died);
    settled_a1(&mut engine, &run);
    // The next run on the worktree finds a1 still standing, because its
    // journal was never read, and is refused by the lease.
    let next = claim(&work, ["run-b", "b1"], Protects::Nothing, &[]);
    assert_eq!(host.admit(&next), cr2(Conflict::Held(run.clone())));
    assert_eq!(host.attempts(&work, &run), (vec!["a1".to_string()], vec![]));
}

/// An attempt its durable facts cannot prove over keeps its reservation,
/// byte for byte, and keeps refusing (M7), its positive fact sealed: one
/// whose journal shows an indeterminate end, one whose journal shows no
/// end, and one whose journal cannot be read.
#[test]
fn an_uncertain_survivor_keeps_its_reservation_and_refuses() {
    let host = Host::new();
    let (_dir, mut engine) = engine(single_body(Vec::new()));
    let run = engine.run_id.clone();
    let journal = std::fs::canonicalize(engine.store.path()).unwrap();
    let payload = json!({"effect_id": "e", "attempt_id": "i1", "reason": "restarted"});
    let ended = EventType::EffectIndeterminate;
    (engine
        .store
        .append_next(&run, ended, payload, None, Some("i1".into())))
    .unwrap();
    let survivors = [
        ("indeterminate", "i1", journal.clone()),
        ("unended", "u1", journal),
        ("unreadable", "r1", host.dir.path().join("missing.db")),
    ];
    for (name, attempt, journal) in survivors {
        survives(&host, name, [&run, attempt], journal);
    }
}

/// The mount table's judgement (H4): a root is shown once where no other
/// mount of its device shows a part holding it or inside it; a bind of
/// its parent elsewhere, a bind of a part inside it, a mount of its device
/// whose root is no path, its own mount's root no path, a root outside its
/// own mount's point, a mount the table does not name, and a table that
/// does not parse each prove nothing.
#[cfg(target_os = "linux")]
#[test]
fn the_mount_table_shows_a_root_once_or_proves_nothing() {
    let table = "1 0 8:1 / / rw - ext4 /dev/sda1 rw\n\
                 2 1 8:1 /home/u/repo/.forge /mnt/alias rw - ext4 /dev/sda1 rw\n\
                 \n\
                 3 1 0:5 / /tmp rw - tmpfs tmpfs rw\n\
                 4 1 8:2 /@home /home rw - btrfs /dev/sda2 rw\n\
                 5 1 8:2 /@srv /srv/with\\040space rw - btrfs /dev/sda2 rw\n\
                 6 1 0:4 net:[4026531840] /run/netns/x rw - nsfs nsfs rw\n\
                 7 1 0:9 / /var/run rw - tmpfs tmpfs rw\n\
                 8 1 0:9 mnt:[4026531841] /var/run/ns rw - nsfs nsfs rw\n";
    let table = table.as_bytes();
    let path = Path::new;
    let (once_, elsewhere) = (Shown::Once, Shown::Elsewhere);
    assert_eq!(
        once(table, 3, path("/tmp/x")),
        once_,
        "its device mounted once"
    );
    assert_eq!(
        once(table, 5, path("/srv/with space/x")),
        once_,
        "another subvolume apart"
    );
    assert_eq!(
        once(table, 1, path("/home/u/repo/.forge/artifacts")),
        elsewhere,
        "its parent bound"
    );
    assert_eq!(
        once(table, 1, path("/home/u/repo")),
        elsewhere,
        "a part of it bound"
    );
    assert_eq!(once(table, 1, path("/srv")), once_, "a part elsewhere");
    assert_eq!(
        once(table, 7, path("/var/run/x")),
        elsewhere,
        "a mount of its device with no path"
    );
    assert_eq!(
        once(table, 6, path("/run/netns/x/y")),
        elsewhere,
        "its own mount's root no path"
    );
    assert_eq!(
        once(table, 3, path("/var/x")),
        elsewhere,
        "a root outside its own mount's point"
    );
    assert_eq!(
        once(table, 9, path("/tmp/x")),
        elsewhere,
        "a mount the table does not name"
    );
    assert_eq!(
        once(b"1 0 8:1\n", 1, path("/x")),
        elsewhere,
        "a table that does not parse"
    );
    assert_eq!(
        once(b"1 0 8:1 / relative rw\n", 1, path("/x")),
        elsewhere,
        "a point not absolute"
    );
}

/// [`exclusion::once`] on the bytes `table`, well within its deadline.
#[cfg(target_os = "linux")]
fn once(table: &[u8], mount: u64, root: &Path) -> Shown {
    exclusion::once(&mut &table[..], mount, root, &Budget::new(soon()))
}

/// A mount table that never ends (M3): each record its own mount on one
/// device, made only as the reader asks for it, counted as it is begun.
#[cfg(target_os = "linux")]
struct Endless {
    record: Vec<u8>,
    at: usize,
    served: usize,
    consumed: usize,
    /// Whether each record never ends either.
    unended: bool,
    /// Whether each record is empty (L3 of run four's second review).
    blank: bool,
}

#[cfg(target_os = "linux")]
impl Endless {
    fn new(unended: bool) -> Endless {
        let (record, at, served, consumed) = (Vec::new(), 0, 0, 0);
        Endless {
            record,
            at,
            served,
            consumed,
            unended,
            blank: false,
        }
    }
}

#[cfg(target_os = "linux")]
impl std::io::Read for Endless {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let ready = std::io::BufRead::fill_buf(self)?;
        let read = ready.len().min(into.len());
        into[..read].copy_from_slice(&ready[..read]);
        std::io::BufRead::consume(self, read);
        Ok(read)
    }
}

#[cfg(target_os = "linux")]
impl std::io::BufRead for Endless {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.at == self.record.len() {
            self.at = 0;
            self.record = match self.unended {
                true => vec![b'/'; 4096],
                false => {
                    self.served += 1;
                    let id = self.served + 1;
                    let record = format!("{id} 0 8:1 / /m{id} rw - ext4 /dev/sda1 rw\n");
                    if self.blank { "\n".into() } else { record }.into_bytes()
                }
            };
        }
        Ok(&self.record[self.at..])
    }

    fn consume(&mut self, read: usize) {
        self.at += read;
        self.consumed += read;
    }
}

/// The mount table is collected one record at a time, each counted and
/// the deadline checked as it is read (M3): a table that never ends is
/// read to record 65,537 and no further, and refused by D5's bound, empty
/// records counted as any other (L3 of run four's second review); a
/// record that never ends is read no further than its own bound; a table
/// whose deadline passed is not read at all; and each judgement refuses
/// by its own cause.
#[cfg(target_os = "linux")]
#[test]
fn the_mount_table_is_collected_one_record_at_a_time_within_its_bounds() {
    let root = Path::new("/one/x");
    let mut endless = Endless::new(false);
    let shown = exclusion::once(&mut endless, 1, root, &Budget::new(soon()));
    assert_eq!((shown, endless.served), (Shown::TooMany, 65_537));
    let mut blank = Endless::new(false);
    blank.blank = true;
    let shown = exclusion::once(&mut blank, 1, root, &Budget::new(soon()));
    assert_eq!(
        (shown, blank.served),
        (Shown::TooMany, 65_537),
        "empty records"
    );
    let mut unended = Endless::new(true);
    let shown = exclusion::once(&mut unended, 1, root, &Budget::new(soon()));
    assert_eq!((shown, unended.consumed), (Shown::TooMany, (64 << 10) + 1));
    let mut endless = Endless::new(false);
    let shown = exclusion::once(&mut endless, 1, root, &Budget::new(Instant::now()));
    assert_eq!((shown, endless.served), (Shown::Expired, 0));
    let refusal = |shown| exclusion::refusal(shown, root);
    let shown = Unadmitted::Evidence(Conflict::Shown(root.to_path_buf()));
    assert_eq!(refusal(Shown::Once), Ok(()));
    assert_eq!(refusal(Shown::Elsewhere), Err(shown));
    assert_eq!(refusal(Shown::TooMany), Err(Unknown::Bounds.into()));
    assert_eq!(refusal(Shown::Expired), Err(Unknown::Expired.into()));
}

/// D5's mount bound is enforced as the table is read (R4): a table of
/// exactly the bound is read whole, one record more is MB3's identity
/// cause ([`Shown::TooMany`]), never a silent judgement.
#[cfg(target_os = "linux")]
#[test]
fn the_mount_table_is_bounded_as_it_is_read() {
    // Mount 1 is alone on its device, so it is shown once; the rest fill
    // the table on one other device.
    let queried = "1 0 9:9 / /one rw - ext4 /dev/x rw\n";
    let filler = |id: usize| format!("{id} 0 8:1 / /m{id} rw - ext4 /dev/sda1 rw\n");
    let table = |records: usize| {
        let fill: String = (2..=records).map(filler).collect();
        format!("{queried}{fill}")
    };
    // 65,536 total records: the queried mount, then 65,535 fillers.
    let at_bound = table(65_536);
    let over = table(65_537);
    let root = Path::new("/one/x");
    assert_eq!(
        once(at_bound.as_bytes(), 1, root),
        Shown::Once,
        "the bound is read"
    );
    assert_eq!(
        once(over.as_bytes(), 1, root),
        Shown::TooMany,
        "one over refuses"
    );
}

/// Set in the mount namespace [`a_bind_alias_of_protected_evidence_is_refused`]
/// runs its proof in, naming its fixture.
#[cfg(target_os = "linux")]
const ALIASED: &str = "BROKKR_U8A2A_ALIASED";

/// A bind alias of protected evidence is refused (H4): a box whose
/// writable root is a bind of the evidence's parent, by the identity it
/// shares, and a box whose writable root only holds that bind, by the
/// mount table showing the evidence at a second mount point. Both are
/// admitted before the bind. It binds in a user and mount namespace of
/// its own, which a seat's box cannot nest: the controller runs it on the
/// host.
#[cfg(target_os = "linux")]
#[test]
fn a_bind_alias_of_protected_evidence_is_refused() {
    if let Some(dir) = std::env::var_os(ALIASED) {
        return aliased(Path::new(&dir));
    }
    let required = brokkr_protocol::hands::boundary_evidence_required();
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        let reason = "a_bind_alias_of_protected_evidence_is_refused: a box cannot nest one";
        return brokkr_protocol::hands::skip_boundary_proof(required, reason);
    }
    let host = Host::new();
    std::fs::create_dir_all(artifacts(&host.worktree("kept"))).unwrap();
    std::fs::create_dir_all(host.worktree("other").join("holder/alias")).unwrap();
    let name = "engine::broker::admission::tests::coordination::a_bind_alias_of_protected_evidence_is_refused";
    let status = Command::new("unshare")
        .args(["--user", "--map-root-user", "--mount", "--"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(ALIASED, host.dir.path())
        .status();
    if !host.dir.path().join("entered").exists() {
        let reason = "a_bind_alias_of_protected_evidence_is_refused: no user and mount namespace";
        return brokkr_protocol::hands::skip_boundary_proof(required, reason);
    }
    assert!(
        status.unwrap().success(),
        "the proof in its namespace failed"
    );
}

/// [`a_bind_alias_of_protected_evidence_is_refused`] in its namespace, on
/// the fixture at `dir`.
#[cfg(target_os = "linux")]
fn aliased(dir: &Path) {
    std::fs::write(dir.join("entered"), "").unwrap();
    let (kept, other) = (dir.join("kept"), dir.join("other"));
    let (holder, alias) = (other.join("holder"), other.join("holder/alias"));
    let evidence = [artifacts(&kept)];
    let coordination = opened(&dir.join("home"));
    let judged = |site: &str, root: &Path| {
        let incoming = claim(
            &other,
            ["run-b", site],
            Protects::Nothing,
            &[boxed(site, &[root])],
        );
        let comparison = Comparison {
            claim: &incoming,
            standing: &[],
            historical: &evidence,
            coordination: &coordination,
        };
        compared(&comparison, &mut Budget::new(soon()))
    };
    assert_eq!(judged("alias", &alias), Ok(()), "before the bind");
    assert_eq!(judged("holder", &holder), Ok(()), "before the bind");
    // A root that does not stand, as an unopened one need not: only its
    // standing ancestor's identity leads the alias to it.
    let unopened = kept.join(".forge/absent");
    assert!(!covers_spelled(&alias, &unopened), "before the bind");
    let mut bind = Command::new("mount");
    let bound = bind
        .arg("--bind")
        .arg(kept.join(".forge"))
        .arg(&alias)
        .status();
    assert!(bound.unwrap().success(), "the bind was made");
    assert!(covers_spelled(&alias, &unopened), "by the bind's identity");
    let below = alias.join("artifacts");
    assert!(covers_spelled(&below, &kept.join(".forge")), "from below");
    let refused = reaches("alias", "run-b", &evidence[0]);
    assert_eq!(judged("alias", &alias), Err(refused.into()));
    let shown = Conflict::Shown(evidence[0].clone());
    assert_eq!(judged("holder", &holder), Err(shown.into()));
}
