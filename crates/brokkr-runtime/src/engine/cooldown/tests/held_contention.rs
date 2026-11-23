//! Fix round 0106b1r13, H1 and H2: a standing probe hold holds the engine's
//! own reads too. Two real runs over one journal, each in its own worktree,
//! each resumed at the end of its own held wait on the same route, now
//! expired. Run `a`'s probe is held at its driver's barrier once its hold
//! has committed and while its tree runs; run `b` is driven meanwhile.
//! Whether `a`'s hold lands before `b`'s wait or before `b`'s retry,
//! nothing of `b` dispatches until `a`'s answer commits, never a second
//! hold stands, and then `b` proceeds.

use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use super::held_route::{holds_written, slicing, unended_holds};
use super::held_wait::{retried_without_a_park, standing};
use super::*;

/// Where run `a`'s hold lands in run `b`'s drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lands {
    /// Before `b`'s drive begins: `b`'s wait reads it.
    BeforeWait,
    /// Once `b`'s own probe has answered and ended its hold, before its
    /// retry's re-read: the retry reads it.
    BeforeRetry,
}

/// Run `a`'s flight, as `b`'s clock drives it: `a` before it flies, its
/// drive's thread while it flies, `b`'s log as `a`'s hold stood once it
/// was in flight and again as `a` was released, and the most probe holds
/// `b`'s reads ever found standing at once.
struct Flight {
    root: PathBuf,
    waiting: Option<Engine>,
    flying: Option<JoinHandle<Engine>>,
    landed: Option<Vec<String>>,
    released: Option<Vec<String>>,
    most_standing: i64,
}

impl Flight {
    /// Drive `a` on its own thread until its probe stands at the barrier:
    /// its hold committed and its driver running.
    fn fly(&mut self) {
        let mut a = self.waiting.take().expect("run a flies once");
        self.flying = Some(std::thread::spawn(move || {
            a.drive().unwrap();
            a
        }));
        let started = self.root.join("a.started");
        let begun = std::time::Instant::now();
        while !started.exists() {
            assert!(
                begun.elapsed().as_secs() < 30,
                "run a's probe never started"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let journal = self.root.join("forge.db");
        assert_eq!(unended_holds(&journal), 1, "run a's hold did not stand");
        self.landed = Some(logs(&self.root, "b"));
    }

    /// Release `a`'s probe and wait for its drive to end: its answer, and
    /// the end of its hold, committed. `b`'s log is taken first.
    fn release(&mut self) {
        let Some(flying) = self.flying.take() else {
            return;
        };
        self.released = Some(logs(&self.root, "b"));
        std::fs::write(self.root.join("a.release"), "").unwrap();
        self.waiting = Some(flying.join().unwrap());
    }
}

/// One run of the shared journal under `root`, in worktree `tree`: its
/// driver, tagged `tree`, refuses claude/first with a reset of five seconds
/// past the two-second maximum, then succeeds, and its probes do what
/// `probed` says. Driven once, the run holds its wait, running.
fn held_run(root: &Path, tree: &str, probed: Probed) -> (Engine, Arc<Mutex<OffsetDateTime>>) {
    let command = driver(
        root,
        tree,
        &["refuse", "succeed"],
        probed,
        Some(5),
        "claude",
        "first",
    );
    // A deadline no test step nears, so the barrier, not the watchdog,
    // ends `a`'s probe.
    let limits = Limits {
        max_attempts: 2,
        timeout_seconds: 30,
    };
    let mut engine = started_in(root, tree, single_body(command), limits);
    engine.cooldown.host = Some(host(root, 1, 2));
    let (mut engine, now) = with_clock(engine);
    engine.drive().unwrap();
    assert_eq!(standing(&engine), (Status::Running, 0));
    (engine, now)
}

/// Both runs' clocks set where both held rows have ended: the route holds
/// nothing.
fn at_the_held_ends(clocks: [&Arc<Mutex<OffsetDateTime>>; 2], store: &brokkr_store::Store) {
    let early = *clocks[0].lock().unwrap();
    let held = store.cooldown("claude", "first", early).unwrap();
    let end = held.expect("both held rows hold the route").until;
    for clock in clocks {
        *clock.lock().unwrap() = end;
    }
    assert_eq!(store.cooldown("claude", "first", end).unwrap(), None);
}

/// Run `b`'s clock: every read counts the holds standing, and, where `a`'s
/// hold lands before `b`'s retry, flies `a` at the first read after `b`'s
/// own hold has ended; every slice `b` sleeps first releases `a`, whose
/// answer then commits, and then advances `b`'s time.
fn b_clock(
    now: &Arc<Mutex<OffsetDateTime>>,
    flight: &Arc<Mutex<Flight>>,
    lands: Lands,
    run_b: String,
) -> Clock {
    let (read, watched) = (Arc::clone(now), Arc::clone(flight));
    let journal = flight.lock().unwrap().root.join("forge.db");
    let ended = move || {
        rusqlite::Connection::open(&journal)
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM probe_holds WHERE run_id = ?1 AND ended_at IS NOT NULL",
                [&run_b],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
    };
    let mut advance = slicing(now);
    let sleeping = Arc::clone(flight);
    Clock {
        now: Box::new(move || {
            let mut flight = watched.lock().unwrap();
            let standing = unended_holds(&flight.root.join("forge.db"));
            flight.most_standing = flight.most_standing.max(standing);
            if lands == Lands::BeforeRetry && flight.landed.is_none() && ended() > 0 {
                flight.fly();
            }
            *read.lock().unwrap()
        }),
        sleep: Box::new(move |slice| {
            sleeping.lock().unwrap().release();
            advance(slice);
        }),
    }
}

/// Drive the two runs with `a`'s hold landing where `lands` says, and pin
/// that `b` dispatched nothing while it stood, that no second hold ever
/// stood, and that both then proceeded: `b`'s log is `b_ran`.
fn contended(lands: Lands, b_ran: &[&str]) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let (a, a_now) = held_run(root, "a", Probed::Barrier);
    let (mut b, b_now) = held_run(root, "b", Probed::Answer);
    at_the_held_ends([&a_now, &b_now], &b.store);
    let flight = Arc::new(Mutex::new(Flight {
        root: root.to_path_buf(),
        waiting: Some(a),
        flying: None,
        landed: None,
        released: None,
        most_standing: 0,
    }));
    if lands == Lands::BeforeWait {
        flight.lock().unwrap().fly();
    }
    b.cooldown.clock = b_clock(&b_now, &flight, lands, b.run_id.clone());
    b.drive().unwrap();
    // A `b` that never slept never released `a`: release it before any
    // assertion, so a failing case ends rather than hangs.
    let mut flight = flight.lock().unwrap();
    flight.release();
    let a = flight.waiting.take().expect("run a was driven");
    assert!(
        flight.landed.is_some(),
        "{lands:?}: run a's hold never landed"
    );
    assert_eq!(
        flight.released, flight.landed,
        "{lands:?}: run b dispatched while run a's hold stood, or never waited on it"
    );
    assert_eq!(flight.most_standing, 1, "{lands:?}: a second hold stood");
    assert_eq!(
        logs(root, "a"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(logs(root, "b"), b_ran, "{lands:?}");
    // Each run retried its effect with no park before it succeeded.
    retried_without_a_park(&a);
    retried_without_a_park(&b);
    let journal = root.join("forge.db");
    // One hold for each probe that ran, `a`'s and `b`'s, each ended.
    let b_probes = b_ran.iter().filter(|ran| **ran == "probe").count();
    let probes = i64::try_from(b_probes + 1).unwrap();
    assert_eq!(
        (holds_written(&journal), unended_holds(&journal)),
        (probes, 0)
    );
}

/// Fix round 0106b1r13, H1: run `a`'s standing hold holds run `b`'s wait
/// and run `b`'s retry, as it holds the pass and the claim. Removal
/// control: read `Store::cooldown`'s aggregate from `provider_cooldowns`
/// alone again, and `b` probes and retries under `a`'s hold.
#[test]
fn no_admitted_run_dispatches_on_a_route_another_runs_probe_holds() {
    contended(
        Lands::BeforeWait,
        &["attempt:refuse", "probe", "attempt:succeed"],
    );
    contended(
        Lands::BeforeRetry,
        &["attempt:refuse", "probe", "probe", "attempt:succeed"],
    );
}
