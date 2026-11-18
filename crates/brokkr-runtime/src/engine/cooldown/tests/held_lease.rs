//! Fix round 0106b1r3, R2 (decision 0106 ruling 2, 0103 ruling 4): a
//! probe's renewed hold is committed to the cool-down table BEFORE the
//! probe's lease is released, so no pass or claim after the release finds
//! the route unheld while the provider still refuses it; and a round the
//! store does not take leaves the lease standing and parks, failing
//! closed. A protected root's lease is Linux only, as `lease.rs`'s is.
//!
//! The competitor runs inside the engine's own clock: every read after the
//! barrier probe started asks the protected root for a competing claim
//! and, where the probe's lease is gone, a second store connection asks
//! admission's real pass and claim legs for claude/first at that instant.

use std::path::PathBuf;

use crate::admission::{legs_at, Capacity};
use crate::engine::broker::admission::tests::{protecting_refused, Coordinated};
use crate::engine::broker::admission::{Conflict, Unadmitted};

use super::held_route::{acting_on_read, held_to};
use super::held_wait::standing;
use super::*;

/// What the competitor saw at one clock read after the probe started.
#[derive(Debug, Clone, PartialEq)]
enum Seen {
    /// The probe's lease still stood.
    Standing,
    /// The lease was gone, and the pass's and claim's legs said this.
    Released([Result<(), Capacity>; 2]),
}

/// One test's protected root: its home, the worktree a competing claim
/// names, the journal a second connection opens, and the probe's start
/// marker.
struct Root {
    home: PathBuf,
    worktree: PathBuf,
    journal: PathBuf,
    started: PathBuf,
}

impl Root {
    fn of(dir: &Path, engine: &Engine) -> Root {
        Root {
            home: dir.join("home"),
            worktree: dir.join("work"),
            journal: engine.store.path().to_path_buf(),
            started: dir.join("work.started"),
        }
    }

    /// The competitor's one look at `at`: nothing before the probe
    /// started, the lease standing, or the legs where it was released.
    fn look(&self, run_id: &str, at: OffsetDateTime) -> Option<Seen> {
        if !self.started.exists() {
            return None;
        }
        match protecting_refused(&self.home, &self.worktree) {
            Err(Unadmitted::Evidence(Conflict::Held(run))) if run == run_id => Some(Seen::Standing),
            Ok(()) => {
                let store = brokkr_store::Store::open(&self.journal).unwrap();
                Some(Seen::Released(legs_at(&store, at, "claude", "first")))
            }
            other => panic!("the competing claim met neither the lease nor its absence: {other:?}"),
        }
    }
}

/// A single seat held five seconds past its two-second maximum, journaled
/// under a protected root; the clock set to the stored end of that held
/// row, so the old row no longer holds; and the barrier probe released
/// ahead, so it states another five seconds at once.
fn held_at_its_end() -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>, Root) {
    let (dir, engine, now) =
        super::held_route::held_at_its_end(Probed::BarrierRefuse, |dir, engine| {
            let _stand = Coordinated::at(&super::lease::home(dir));
            engine.drive().unwrap();
        });
    let root = Root::of(dir.path(), &engine);
    std::fs::write(dir.path().join("work.release"), "").unwrap();
    (dir, engine, now, root)
}

#[test]
fn a_probes_renewed_hold_is_committed_before_its_lease_is_released() {
    let (_dir, mut engine, now, root) = held_at_its_end();
    let probe_at = *now.lock().unwrap();
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
    let (watch, run_id) = (Arc::clone(&seen), engine.run_id.clone());
    let root = Arc::new(root);
    let looking = Arc::clone(&root);
    engine.cooldown.clock = acting_on_read(&now, move |at| {
        let mut seen = watch.lock().unwrap();
        // One competing claim taken at most: an admitted one stands.
        if !seen.iter().any(|look| matches!(look, Seen::Released(_))) {
            seen.extend(looking.look(&run_id, at));
        }
    });
    {
        let _stand = Coordinated::at(&root.home);
        engine.drive().unwrap();
    }
    assert_eq!(standing(&engine), (Status::Running, 0));
    let seen = seen.lock().unwrap().clone();
    let end = engine
        .store
        .cooldown("claude", "first", probe_at)
        .unwrap()
        .expect("the probe's held round holds the route")
        .until;
    assert!(end >= probe_at + time::Duration::seconds(5), "{end}");
    // Wherever the lease was gone, both legs already held the route.
    for look in &seen {
        if let Seen::Released(legs) = look {
            assert!(held_to(legs, end), "{seen:?}");
        }
    }
    // The round was judged and appended under the probe's standing lease.
    assert!(seen.contains(&Seen::Standing), "{seen:?}");
    // After the release, a second connection's pass and claim hold the
    // route until the new end, and admit at it.
    let competitor = brokkr_store::Store::open(&root.journal).unwrap();
    for at in [probe_at, end - time::Duration::seconds(1)] {
        let legs = legs_at(&competitor, at, "claude", "first");
        assert!(held_to(&legs, end), "at {at}: {legs:?}");
    }
    assert_eq!(
        legs_at(&competitor, end, "claude", "first"),
        [Ok(()), Ok(())]
    );
    if !seen.iter().any(|look| matches!(look, Seen::Released(_))) {
        assert_eq!(protecting_refused(&root.home, &root.worktree), Ok(()));
    }
}

/// A probe round the store refuses (a competing round 2 planted between
/// the refusal and the append) parks the run and leaves the probe's lease
/// standing: the release is not sealed over a round the table never took.
#[test]
fn a_probe_round_the_store_refuses_parks_and_leaves_its_lease_standing() {
    let (_dir, mut engine, now, root) = held_at_its_end();
    let seq = failed(&engine)[0].seq;
    let (journal, run_id, started) = (
        root.journal.clone(),
        engine.run_id.clone(),
        root.started.clone(),
    );
    let planted = Arc::new(Mutex::new(false));
    engine.cooldown.clock = acting_on_read(&now, move |at| {
        let mut planted = planted.lock().unwrap();
        if *planted || !started.exists() {
            return;
        }
        *planted = true;
        let recorded = at.unix_timestamp();
        rusqlite::Connection::open(&journal)
            .unwrap()
            .execute(
                "INSERT INTO provider_cooldowns
                    (run_id, event_seq, site, provider, route, until_at, reason,
                     rounds, recorded_at)
                 VALUES (?1, ?2, '', 'claude', 'first', ?3, 'planted', 2, ?4)",
                rusqlite::params![run_id, seq as i64, recorded + 60, recorded],
            )
            .unwrap();
    });
    {
        let _stand = Coordinated::at(&root.home);
        engine.drive().unwrap();
    }
    assert_eq!(standing(&engine), (Status::AwaitingOperator, 1));
    assert_eq!(
        protecting_refused(&root.home, &root.worktree),
        Err(Unadmitted::Evidence(Conflict::Held(engine.run_id.clone()))),
        "the refused round's probe lease was released"
    );
}
