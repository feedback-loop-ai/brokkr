#[cfg(target_os = "linux")]
use super::table::Sighting;
use super::*;
#[cfg(target_os = "linux")]
use std::sync::atomic::AtomicBool;

use rustix::io::Errno;

/// Pids no process can have: above Linux's PID_MAX_LIMIT (4,194,304) and
/// macOS's 99,999, so a reap of one reaps nothing, and a row planted at
/// one is born after this test process, whose own row `table` adds and
/// whose pid on a busy Linux host can pass four million.
const GROUP: i32 = 1_000_000_001;
const RECORDED: i32 = 1_000_000_002;
const STRANGER: i32 = 1_000_000_003;

fn me() -> i32 {
    getpid().as_raw_pid()
}

fn row(pid: i32, ppid: i32, pgid: i32) -> Entry {
    Entry {
        id: Identity {
            pid,
            start: pid.to_string(),
        },
        ppid,
        pgid,
        zombie: false,
        exiting: false,
    }
}

fn zombie(entry: Entry) -> Entry {
    Entry {
        zombie: true,
        ..entry
    }
}

/// This process, leading the engine's group, and `rows` beside it.
fn table(rows: impl IntoIterator<Item = Entry>) -> Vec<Entry> {
    let mut entries = vec![row(me(), 1, me())];
    entries.extend(rows);
    entries
}

fn ids<'a>(rows: impl IntoIterator<Item = &'a Entry>) -> BTreeSet<Identity> {
    rows.into_iter().map(|entry| entry.id.clone()).collect()
}

/// A closed attempt leading `group`, with `recorded` and `before`. It is
/// held apart from the engine's registry: the rows these tests plant are
/// not processes, and must reach no live attempt.
fn closed(group: i32, recorded: &[Entry], before: &[Entry]) -> Live {
    Live {
        group: Pid::from_raw(group).unwrap(),
        open: false,
        recorded: ids(recorded),
        before: ids(before),
        doubted: BTreeSet::new(),
        ended: None,
    }
}

/// A walk's sighting: the engine, `leader`, and `extra` when there is one.
#[cfg(target_os = "linux")]
fn sighting(leader: Entry, extra: Option<Entry>) -> Sighting {
    let mut rows = vec![row(me(), 1, me()), leader];
    rows.extend(extra);
    Sighting::new(rows, false)
}

/// A registry holding one open attempt `group` with `recorded`, and the
/// host whose tick is `walk` and whose full table is `full`.
#[cfg(target_os = "linux")]
fn tracked(
    group: i32,
    recorded: &[Entry],
    walk: fn() -> Result<Sighting, TableError>,
    full: fn() -> Result<Vec<Entry>, TableError>,
) -> (Registry, Host) {
    let live = Live {
        open: true,
        ..closed(group, recorded, &[])
    };
    (
        registry([live]),
        Host {
            tick: walk,
            table: full,
            ..Host::REAL
        },
    )
}

/// The leader row the tracker's change tests plant: `GROUP` running, or
/// its zombie once `turned` is set.
#[cfg(target_os = "linux")]
fn turning_leader(turned: &AtomicBool) -> Entry {
    let leader = row(GROUP, me(), GROUP);
    if turned.load(Ordering::Relaxed) {
        zombie(leader)
    } else {
        leader
    }
}

/// A walk of `turning_leader`, with the detached orphan `STRANGER` once
/// `orphan` is set.
#[cfg(target_os = "linux")]
fn turning_walk(turned: &AtomicBool, orphan: &AtomicBool) -> Result<Sighting, TableError> {
    let extra = orphan
        .load(Ordering::Relaxed)
        .then(|| row(STRANGER, me(), STRANGER));
    Ok(sighting(turning_leader(turned), extra))
}

/// The full table a change test reads: the leader, the orphan once
/// `orphan` is set, and a failure when `fail` is set. Each read counts in
/// `snapshots`.
#[cfg(target_os = "linux")]
fn turning_full(
    snapshots: &AtomicU64,
    turned: &AtomicBool,
    fail: &AtomicBool,
    orphan: &AtomicBool,
) -> Result<Vec<Entry>, TableError> {
    snapshots.fetch_add(1, Ordering::Relaxed);
    if fail.load(Ordering::Relaxed) {
        // The unrelated row the read could not read: the snapshot the
        // walk owes is not taken.
        return Err(TableError::Row {
            pid: RECORDED,
            error: Errno::ACCESS.into(),
        });
    }
    let mut rows = vec![turning_leader(turned)];
    if orphan.load(Ordering::Relaxed) {
        rows.push(row(STRANGER, me(), STRANGER));
    }
    Ok(table(rows))
}

/// The change tests' host: kill nothing, but record what the kill hook is
/// handed.
#[cfg(target_os = "linux")]
fn turning_host(host: Host) -> Host {
    Host {
        kill_group: |_| Ok(()),
        kill: recorded,
        ..host
    }
}

/// Start a change test cleanly: no previous walk, a running leader, and no
/// orphan. The first tick owes exactly one snapshot, which this host takes.
#[cfg(target_os = "linux")]
fn first_tick(
    registry: &mut Registry,
    host: Host,
    snapshots: &AtomicU64,
    turned: &AtomicBool,
    orphan: &AtomicBool,
) {
    snapshots.store(0, Ordering::Relaxed);
    turned.store(false, Ordering::Relaxed);
    orphan.store(false, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        snapshots.load(Ordering::Relaxed),
        1,
        "the first tick had no walk to compare"
    );
    assert_eq!(registry.attempts[&0].ended, None, "the leader still ran");
}

/// The planted statics the two change tests share. A `Host` carries only
/// function pointers, so the fixture cannot be passed in, and a lock keeps
/// the two tests from resetting each other's scenario.
#[cfg(target_os = "linux")]
static CHANGE_LOCK: Mutex<()> = Mutex::new(());
#[cfg(target_os = "linux")]
static CHANGE_SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "linux")]
static CHANGE_TURNED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "linux")]
static CHANGE_FAIL: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "linux")]
static CHANGE_ORPHAN: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
fn change_walk() -> Result<Sighting, TableError> {
    turning_walk(&CHANGE_TURNED, &CHANGE_ORPHAN)
}

#[cfg(target_os = "linux")]
fn change_full() -> Result<Vec<Entry>, TableError> {
    turning_full(
        &CHANGE_SNAPSHOTS,
        &CHANGE_TURNED,
        &CHANGE_FAIL,
        &CHANGE_ORPHAN,
    )
}

/// A change test's held lock, registry and host, with the first tick
/// already taken: no previous walk, a running leader, and no orphan.
#[cfg(target_os = "linux")]
fn change_scenario() -> (MutexGuard<'static, ()>, Registry, Host) {
    let held = CHANGE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    CHANGE_FAIL.store(false, Ordering::Relaxed);
    let (mut registry, host) = tracked(GROUP, &[], change_walk, change_full);
    let host = turning_host(host);
    first_tick(
        &mut registry,
        host,
        &CHANGE_SNAPSHOTS,
        &CHANGE_TURNED,
        &CHANGE_ORPHAN,
    );
    (held, registry, host)
}

/// A kill that signals nothing, and one the kernel refuses.
fn spared(_: &Identity) -> std::io::Result<()> {
    Ok(())
}

fn forbidden(_: &Identity) -> std::io::Result<()> {
    Err(Errno::PERM.into())
}

thread_local! {
    /// The pids this thread's kill hook was handed. The hook is a function
    /// pointer, so the tests that count what a kill reached share this.
    static SIGNALLED: std::cell::RefCell<Vec<i32>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Record every identity the kill hook is given.
fn recorded(id: &Identity) -> std::io::Result<()> {
    SIGNALLED.with(|signalled| signalled.borrow_mut().push(id.pid));
    Ok(())
}

/// Kill what the attempt `key` owns on `host` and assert the recording
/// hook saw nothing: the planted stray stayed the engine's own.
fn assert_kill_signals_nothing(registry: &Registry, key: u64, host: Host) {
    SIGNALLED.with(|signalled| signalled.borrow_mut().clear());
    assert_eq!(registry.attempts[&key].kill(host), Ok(()));
    let signalled = SIGNALLED.with(|signalled| std::mem::take(&mut *signalled.borrow_mut()));
    assert_eq!(signalled, Vec::<i32>::new());
    assert_eq!(registry.unowned, ids([&row(STRANGER, me(), STRANGER)]));
}

/// A table whose group `GROUP` has a member still running beside its
/// zombie leader.
fn populated() -> Result<Vec<Entry>, TableError> {
    Ok(table([
        zombie(row(GROUP, me(), GROUP)),
        row(RECORDED, GROUP, GROUP),
    ]))
}

/// The tracker records the leader's descendants to any depth, a process
/// that left its session included, and an orphan still in the group, and
/// keeps following what it recorded once the leader is closed; a zombie
/// and a stranger are not recorded.
#[test]
fn the_tracker_records_every_descendant_and_nothing_else() {
    let child = row(RECORDED, GROUP, GROUP);
    let detached = row(RECORDED + 10, RECORDED, RECORDED + 10);
    let later = row(RECORDED + 11, RECORDED + 10, RECORDED + 10);
    let zombie = zombie(row(RECORDED + 12, GROUP, GROUP));
    let orphan = row(RECORDED + 15, me(), GROUP);
    let stranger = row(STRANGER, 1, STRANGER);
    let mut live = Live {
        open: true,
        ..closed(GROUP, &[], &[])
    };
    live.record(&[
        stranger.clone(),
        later.clone(),
        zombie,
        detached.clone(),
        orphan.clone(),
        child.clone(),
    ]);
    assert_eq!(live.recorded, ids([&child, &detached, &later, &orphan]));
    // Closed, the leader's pid names nothing; what was recorded still leads.
    live.open = false;
    let newer = row(RECORDED + 13, RECORDED + 10, RECORDED + 10);
    let reused = row(RECORDED + 14, GROUP, STRANGER);
    let joined = row(RECORDED + 16, me(), GROUP);
    live.record(&[detached.clone(), newer.clone(), reused, joined]);
    assert_eq!(
        live.recorded,
        ids([&child, &detached, &later, &orphan, &newer])
    );
}

/// What `running` reads as still running, in the order it is reported:
/// the group, then a recorded descendant, then an orphan the attempt
/// doubts. A zombie, whatever holds it, counts as gone.
#[test]
fn running_is_the_group_the_recorded_and_the_doubted() {
    let recorded = row(RECORDED, me(), RECORDED);
    let doubted = row(STRANGER, me(), STRANGER);
    let mut live = closed(GROUP, std::slice::from_ref(&recorded), &[]);
    live.doubted = ids([&doubted]);
    let member = row(STRANGER + 1, 1, GROUP);
    assert_eq!(
        live.running(&table([member.clone(), recorded.clone()]), spared),
        Err(Unsettled::Group { group: GROUP })
    );
    assert_eq!(
        live.running(&table([recorded.clone(), doubted.clone()]), spared),
        Err(Unsettled::Descendants {
            pids: vec![RECORDED]
        })
    );
    assert_eq!(
        live.running(&table([doubted.clone()]), spared),
        Err(Unsettled::Strays {
            pids: vec![STRANGER]
        })
    );
    let zombies = [member, recorded, doubted].map(zombie);
    assert_eq!(live.running(&table(zombies), spared), Ok(()));
}

/// A registry of `attempts`, keyed from 0, that has read nothing.
fn registry(attempts: impl IntoIterator<Item = Live>) -> Registry {
    Registry {
        attempts: (0..).zip(attempts).collect(),
        unowned: BTreeSet::new(),
        latest: None,
        #[cfg(target_os = "linux")]
        walk: None,
        #[cfg(target_os = "linux")]
        owed: false,
    }
}

/// Callers that queued for the lock while one read was taken share the
/// next: a read of the same table that began after they asked serves
/// them; an older one, or one of another table, does not.
#[test]
fn a_read_that_began_after_the_ask_is_shared() {
    static OURS: AtomicU64 = AtomicU64::new(0);
    static THEIRS: AtomicU64 = AtomicU64::new(0);
    fn ours() -> Result<Vec<Entry>, TableError> {
        OURS.fetch_add(1, Ordering::Relaxed);
        Ok(table([row(RECORDED, 1, RECORDED)]))
    }
    fn theirs() -> Result<Vec<Entry>, TableError> {
        THEIRS.fetch_add(1, Ordering::Relaxed);
        Ok(table([]))
    }
    let reads = || (OURS.load(Ordering::Relaxed), THEIRS.load(Ordering::Relaxed));
    let mut registry = registry([]);
    let asked = Instant::now();
    let read = registry.read(ours).unwrap();
    assert_eq!(registry.read_since(ours, asked).unwrap(), read);
    assert_eq!(reads(), (1, 0), "shared, not read again");
    let read = registry.read_since(theirs, asked).unwrap();
    assert_eq!(registry.read_since(theirs, Instant::now()).unwrap(), read);
    assert_eq!(reads(), (1, 2));
}

/// #403: the read before a spawn is taken afresh. An orphan of the
/// engine's own (git's detached maintenance) born after the last read and
/// before the spawn is in the attempt's `before`, so once its driver exits
/// unseen, the orphan is the engine's, and the attempt's close does not
/// signal it.
#[test]
fn an_orphan_born_after_the_last_read_and_before_a_spawn_is_not_the_attempts() {
    use std::sync::atomic::AtomicBool;
    static BORN: AtomicBool = AtomicBool::new(false);
    fn planted() -> Result<Vec<Entry>, TableError> {
        let orphan = BORN.load(Ordering::Relaxed);
        Ok(table(orphan.then(|| row(STRANGER, me(), STRANGER))))
    }
    let mut registry = registry([]);
    registry.read(planted).unwrap();
    BORN.store(true, Ordering::Relaxed);
    let (mut driver, key) = registry
        .admit(
            Host {
                table: planted,
                ..Host::REAL
            },
            &mut Command::new("true"),
        )
        .unwrap();
    driver.wait().unwrap();
    // The next read shows the orphan, and not the driver.
    registry.read(planted).unwrap();
    let host = Host {
        kill_group: |_| Ok(()),
        kill: recorded,
        ..Host::REAL
    };
    assert_kill_signals_nothing(&registry, key, host);
}

/// #918 H2: admission's `before` is the full host table, not the
/// tracker's walk. D predates the attempt, but the walk can miss it while
/// D's parent exits before its own row is read; the full read keeps D in
/// `before`, so the tick that later finds D with the engine does not
/// attribute it to the attempt or signal it. The walk is a sighting only:
/// when it does see D, the tick still takes no snapshot, because `before`
/// already knows it.
#[cfg(target_os = "linux")]
#[test]
fn a_pre_existing_orphan_the_walk_misses_is_not_the_attempts() {
    static REPARENTED: AtomicBool = AtomicBool::new(false);
    fn full() -> Result<Vec<Entry>, TableError> {
        // P and its detached child D both ran before admission; P exits
        // before its own row is read and D is reparented to the engine.
        Ok(table([
            row(RECORDED, me(), RECORDED),
            row(STRANGER, me(), STRANGER),
        ]))
    }
    fn walk() -> Result<Sighting, TableError> {
        // Before the reparenting the walk names neither: P exited before
        // its row was read, so D is not under P. Afterwards it names D.
        Ok(Sighting::new(
            table(
                REPARENTED
                    .load(Ordering::Relaxed)
                    .then(|| row(STRANGER, me(), STRANGER)),
            ),
            false,
        ))
    }
    /// P has exited; D is the engine's orphan now.
    fn later() -> Result<Vec<Entry>, TableError> {
        Ok(table([row(STRANGER, me(), STRANGER)]))
    }
    let mut registry = registry([]);
    let host = Host {
        table: full,
        tick: walk,
        kill_group: |_| Ok(()),
        kill: recorded,
        ..Host::REAL
    };
    let (mut driver, key) = registry.admit(host, &mut Command::new("true")).unwrap();
    driver.wait().unwrap();
    REPARENTED.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        registry.attempts[&key].recorded,
        BTreeSet::new(),
        "the walk recorded D"
    );
    assert_eq!(
        registry.attempts[&key].doubted,
        BTreeSet::new(),
        "the walk doubted D"
    );
    registry.read(later).unwrap();
    assert_kill_signals_nothing(&registry, key, host);
}

/// #918c: the walk records a descendant P, P exits and its numeric pid is
/// reused by an unrelated U with a child V before P's children are read.
/// The walk may notice a change, but the tracker never attributes from it:
/// the one full snapshot reads U's own parent outside the attempt's tree,
/// so V is never recorded, doubted or signalled.
#[cfg(target_os = "linux")]
#[test]
fn a_pid_reused_between_the_walk_and_the_snapshot_is_not_attributed() {
    static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
    fn walk() -> Result<Sighting, TableError> {
        // P's old identity leads to V by P's numeric pid, as the stale
        // walk would have it; U's own row is not what the walk saw.
        Ok(Sighting::new(
            vec![
                row(me(), 1, me()),
                row(RECORDED, me(), RECORDED),
                row(STRANGER, RECORDED, STRANGER),
            ],
            false,
        ))
    }
    fn full() -> Result<Vec<Entry>, TableError> {
        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        // U reused P's numeric pid under a parent outside the tree; V is
        // U's child.
        Ok(table([
            Entry {
                id: Identity {
                    pid: RECORDED,
                    start: format!("{RECORDED}9"),
                },
                ppid: 900,
                pgid: RECORDED,
                zombie: false,
                exiting: false,
            },
            row(STRANGER, RECORDED, STRANGER),
        ]))
    }
    let p = row(RECORDED, me(), RECORDED);
    let v = row(STRANGER, RECORDED, STRANGER);
    let mut registry = registry([closed(GROUP, std::slice::from_ref(&p), &[])]);
    let host = Host {
        tick: walk,
        table: full,
        kill_group: |_| Ok(()),
        kill: recorded,
        ..Host::REAL
    };
    registry.track(host);
    assert_eq!(SNAPSHOTS.load(Ordering::Relaxed), 1, "one snapshot");
    assert_eq!(registry.attempts[&0].recorded, ids([&p]));
    assert_eq!(registry.attempts[&0].doubted, BTreeSet::new());
    assert_eq!(registry.unowned, BTreeSet::new());
    SIGNALLED.with(|signalled| signalled.borrow_mut().clear());
    assert_eq!(registry.attempts[&0].kill(host), Ok(()));
    let signalled = SIGNALLED.with(|signalled| std::mem::take(&mut *signalled.borrow_mut()));
    assert_eq!(signalled, vec![RECORDED], "V {} was signalled", v.id.pid);
}

/// #918c, #918d: a tick whose walk sees only identities the registry
/// already knows, in the same states, takes no snapshot, however many
/// ticks run; the first tick, with no previous walk, takes one, and a walk
/// that cannot complete takes exactly one. The count is held through the
/// injectable `Host`.
#[cfg(target_os = "linux")]
#[test]
fn quiet_ticks_take_no_snapshot_and_a_doubt_takes_one() {
    static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
    static DOUBT: AtomicBool = AtomicBool::new(false);
    fn walk() -> Result<Sighting, TableError> {
        Ok(Sighting::new(
            vec![row(me(), 1, me()), row(GROUP, me(), GROUP)],
            DOUBT.load(Ordering::Relaxed),
        ))
    }
    fn full() -> Result<Vec<Entry>, TableError> {
        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        Ok(table([]))
    }
    let (mut registry, host) = tracked(GROUP, &[], walk, full);
    SNAPSHOTS.store(0, Ordering::Relaxed);
    DOUBT.store(false, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        SNAPSHOTS.load(Ordering::Relaxed),
        1,
        "the first tick had no walk to compare"
    );
    SNAPSHOTS.store(0, Ordering::Relaxed);
    for _ in 0..10 {
        registry.track(host);
    }
    assert_eq!(SNAPSHOTS.load(Ordering::Relaxed), 0, "a quiet tick read");
    DOUBT.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(SNAPSHOTS.load(Ordering::Relaxed), 1, "a doubt was trusted");
}

/// #918e: a quiet walk whose rows the registry knows only as a closed
/// attempt's doubted identity, or only as the engine's own orphan, takes
/// no snapshot: each is a known identity, so the walk owes nothing.
#[cfg(target_os = "linux")]
#[test]
fn a_quiet_walk_of_doubted_and_unowned_identities_takes_no_snapshot() {
    static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
    fn walk() -> Result<Sighting, TableError> {
        Ok(Sighting::new(
            table([row(RECORDED, GROUP, GROUP), row(STRANGER, me(), STRANGER)]),
            false,
        ))
    }
    fn full() -> Result<Vec<Entry>, TableError> {
        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        Ok(table([]))
    }
    let mut attempt = closed(GROUP, &[], &[]);
    attempt.doubted = ids([&row(RECORDED, GROUP, GROUP)]);
    let mut registry = registry([attempt]);
    registry.unowned = ids([&row(STRANGER, me(), STRANGER)]);
    registry.walk = Some(walk().unwrap().sighted());
    let host = Host {
        tick: walk,
        table: full,
        ..Host::REAL
    };
    SNAPSHOTS.store(0, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        SNAPSHOTS.load(Ordering::Relaxed),
        0,
        "a known identity took a snapshot"
    );
}

/// #918: a live driver is known by its bare pid only while its attempt is
/// open. Once the attempt closes, an identity at its group's pid is known
/// only when one of the attempt's sets names it, so a reused pid is a
/// change that takes a snapshot.
#[cfg(target_os = "linux")]
#[test]
fn only_an_open_attempt_knows_its_group_by_its_bare_pid() {
    let leader = row(GROUP, me(), GROUP);
    assert!(
        !registry([closed(GROUP, &[], &[])]).knows(&leader.id),
        "a closed attempt's group pid was known"
    );
    let mut open = closed(GROUP, &[], &[]);
    open.open = true;
    assert!(
        registry([open]).knows(&leader.id),
        "an open attempt's group pid was not known"
    );
}

/// #918: the registry knows an identity by exactly the set that holds it:
/// the engine itself, what it owns, and a closed attempt's `before`,
/// `recorded` and `doubted` identities. Each arm is asked directly, so its
/// coverage never waits on a process-driven tick's timing, and an identity
/// no set holds is unknown.
#[cfg(target_os = "linux")]
#[test]
fn the_registry_knows_each_identity_by_the_set_that_holds_it() {
    let before = row(1_000_000_011, GROUP, GROUP);
    let recorded = row(1_000_000_012, GROUP, GROUP);
    let doubted = row(1_000_000_013, GROUP, GROUP);
    let unowned = row(1_000_000_014, me(), 1_000_000_014);
    let mut attempt = closed(
        GROUP,
        std::slice::from_ref(&recorded),
        std::slice::from_ref(&before),
    );
    attempt.doubted = ids([&doubted]);
    let mut registry = registry([attempt]);
    registry.unowned = ids([&unowned]);
    let engine = row(me(), 1, me());
    for (known, set) in [
        (&engine, "the engine"),
        (&unowned, "unowned"),
        (&before, "before"),
        (&recorded, "recorded"),
        (&doubted, "doubted"),
    ] {
        assert!(registry.knows(&known.id), "{set} was not known");
    }
    assert!(
        !registry.knows(&row(STRANGER, 1, STRANGER).id),
        "an identity no set holds was known"
    );
}

/// #918e: a walk that cannot complete forgets the last walk and reads the
/// table on that tick. A failed read leaves the snapshot owed; a healthy
/// one answers it, and the next walk, having no memory to compare, reads
/// once more.
#[cfg(target_os = "linux")]
#[test]
fn a_walk_that_fails_reads_the_table_and_a_failed_read_stays_owed() {
    static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
    static WALK_FAILS: AtomicBool = AtomicBool::new(false);
    static READ_FAILS: AtomicBool = AtomicBool::new(false);
    fn failure() -> TableError {
        TableError::Row {
            pid: RECORDED,
            error: Errno::ACCESS.into(),
        }
    }
    fn walk() -> Result<Sighting, TableError> {
        if WALK_FAILS.load(Ordering::Relaxed) {
            return Err(failure());
        }
        Ok(sighting(row(GROUP, me(), GROUP), None))
    }
    fn full() -> Result<Vec<Entry>, TableError> {
        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        if READ_FAILS.load(Ordering::Relaxed) {
            return Err(failure());
        }
        Ok(table([row(GROUP, me(), GROUP)]))
    }
    let (mut registry, host) = tracked(GROUP, &[], walk, full);
    let snapshots = || SNAPSHOTS.load(Ordering::Relaxed);
    SNAPSHOTS.store(0, Ordering::Relaxed);
    WALK_FAILS.store(false, Ordering::Relaxed);
    READ_FAILS.store(false, Ordering::Relaxed);
    registry.track(host);
    registry.track(host);
    assert_eq!(snapshots(), 1, "a quiet tick read");

    WALK_FAILS.store(true, Ordering::Relaxed);
    READ_FAILS.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(snapshots(), 2, "a failed walk did not read the table");
    assert_eq!(registry.walk, None, "a failed walk was remembered");
    assert!(registry.owed, "a failed read answered the walk");

    READ_FAILS.store(false, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(snapshots(), 3, "the failed walk's read was not retried");
    assert!(!registry.owed, "a healthy read left the snapshot owed");

    WALK_FAILS.store(false, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(snapshots(), 4, "a walk after a failed one compared");
    registry.track(host);
    assert_eq!(snapshots(), 4, "a quiet tick read");
}

/// #918d HIGH: a known leader that turns zombie is a change the walk must
/// report. Without the snapshot that change takes, `observe` and `note`
/// never run, `ended` stays `None`, and a detached orphan O born after
/// that tick is attributed to the ended attempt and killed with it. The
/// tick that sees the leader turn zombie is the tick that must end the
/// attempt; the orphan's later appearance only makes O the engine's own.
#[cfg(target_os = "linux")]
#[test]
fn a_leader_that_turns_zombie_on_a_quiet_tick_ends_its_attempt() {
    let (_held, mut registry, host) = change_scenario();
    let born = |pid: i32| u64::try_from(pid).unwrap();
    CHANGE_TURNED.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        CHANGE_SNAPSHOTS.load(Ordering::Relaxed),
        2,
        "the leader's state change did not take a snapshot"
    );
    assert_eq!(
        registry.attempts[&0].ended,
        Some(born(GROUP)),
        "the tick that saw the leader turn zombie did not end the attempt"
    );

    CHANGE_ORPHAN.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        CHANGE_SNAPSHOTS.load(Ordering::Relaxed),
        3,
        "the later orphan did not take a snapshot"
    );
    assert_eq!(
        registry.attempts[&0].recorded,
        BTreeSet::new(),
        "the later orphan O was recorded as the attempt's"
    );
    assert_kill_signals_nothing(&registry, 0, host);
}

/// #918e HIGH: a full read that fails does not answer the walk that asked
/// for it. The walk that turned the leader zombie still owes its snapshot,
/// so the next tick takes it even though the walk is unchanged; the
/// attempt's end is set before a later detached orphan appears, and that
/// orphan stays the engine's own. With the read healthy, the quiet tick
/// takes no extra snapshot (`quiet_ticks_take_no_snapshot_and_a_doubt_takes_one`).
#[cfg(target_os = "linux")]
#[test]
fn chief_failed_snapshot_retries_before_attributing_a_later_orphan() {
    let (_held, mut registry, host) = change_scenario();
    let born = |pid: i32| u64::try_from(pid).unwrap();
    CHANGE_TURNED.store(true, Ordering::Relaxed);
    CHANGE_FAIL.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        CHANGE_SNAPSHOTS.load(Ordering::Relaxed),
        2,
        "the leader's state change did not take a snapshot"
    );
    assert_eq!(
        registry.attempts[&0].ended, None,
        "a failed read ended the attempt"
    );

    CHANGE_FAIL.store(false, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        CHANGE_SNAPSHOTS.load(Ordering::Relaxed),
        3,
        "the snapshot the failed read owed was forgotten"
    );
    assert_eq!(
        registry.attempts[&0].ended,
        Some(born(GROUP)),
        "the recovery tick did not end the attempt"
    );

    CHANGE_ORPHAN.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        CHANGE_SNAPSHOTS.load(Ordering::Relaxed),
        4,
        "the later orphan did not take a snapshot"
    );
    assert_eq!(
        registry.attempts[&0].recorded,
        BTreeSet::new(),
        "the later orphan O was recorded as the attempt's"
    );
    assert_kill_signals_nothing(&registry, 0, host);
}

/// #918d: a recorded descendant that vanishes from the walk is a change
/// the tick must report, even though its identity was known. The tick
/// takes one snapshot; the attribution sets keep the identity, which is
/// what `recorded` means here.
#[cfg(target_os = "linux")]
#[test]
fn a_recorded_descendant_that_vanishes_from_the_walk_takes_one_snapshot() {
    static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);
    static VANISHED: AtomicBool = AtomicBool::new(false);
    fn walk() -> Result<Sighting, TableError> {
        let extra = (!VANISHED.load(Ordering::Relaxed)).then(|| row(RECORDED, GROUP, GROUP));
        Ok(sighting(row(GROUP, me(), GROUP), extra))
    }
    fn full() -> Result<Vec<Entry>, TableError> {
        SNAPSHOTS.fetch_add(1, Ordering::Relaxed);
        let mut rows = vec![row(GROUP, me(), GROUP)];
        if !VANISHED.load(Ordering::Relaxed) {
            rows.push(row(RECORDED, GROUP, GROUP));
        }
        Ok(table(rows))
    }
    let descendant = row(RECORDED, GROUP, GROUP);
    let (mut registry, host) = tracked(GROUP, std::slice::from_ref(&descendant), walk, full);
    SNAPSHOTS.store(0, Ordering::Relaxed);
    VANISHED.store(false, Ordering::Relaxed);

    registry.track(host);
    assert_eq!(
        SNAPSHOTS.load(Ordering::Relaxed),
        1,
        "the first tick had no walk to compare"
    );
    registry.track(host);
    assert_eq!(SNAPSHOTS.load(Ordering::Relaxed), 1, "a quiet tick read");

    VANISHED.store(true, Ordering::Relaxed);
    registry.track(host);
    assert_eq!(
        SNAPSHOTS.load(Ordering::Relaxed),
        2,
        "the vanished descendant did not take a snapshot"
    );
}

/// #403 finding 1: every running child of the engine outside its own
/// group that no live attempt explains is a stray, whatever its session:
/// a job that shell job control moved to a group of its own, orphaned
/// before the tracker saw it, included. It is attributed to the one
/// attempt that could have left it, and doubted by each attempt when
/// several could. A child the engine spawned, a zombie, another process's
/// child, a member of a live group and what an attempt already explains
/// are not strays.
#[test]
fn an_orphan_the_engine_adopted_is_attributed_or_doubted() {
    let job = row(STRANGER, me(), STRANGER);
    let others = [
        row(STRANGER + 1, me(), me()),
        zombie(row(STRANGER + 2, me(), STRANGER + 2)),
        row(STRANGER + 3, 1, STRANGER + 3),
        row(STRANGER + 4, me(), GROUP),
    ];
    let entries = table(others.into_iter().chain([job.clone()]));
    let other_group = GROUP + 100;

    let mut alone = registry([closed(GROUP, &[], &[])]);
    alone.observe(&entries);
    assert_eq!(alone.attempts[&0].recorded, ids([&job]));
    alone.observe(&entries);
    let attempt = &alone.attempts[&0];
    assert_eq!((attempt.recorded.len(), attempt.doubted.len()), (1, 0));

    let mut both = registry([closed(GROUP, &[], &[]), closed(other_group, &[], &[])]);
    both.observe(&entries);
    both.observe(&entries);
    for live in both.attempts.values() {
        assert_eq!((live.recorded.len(), &live.doubted), (0, &ids([&job])));
        assert_eq!(
            live.doubts(None),
            Err(Unsettled::Strays {
                pids: vec![STRANGER]
            })
        );
    }

    let mut later = registry([
        closed(GROUP, &[], &[]),
        closed(other_group, &[], std::slice::from_ref(&job)),
    ]);
    later.observe(&entries);
    assert_eq!(
        (
            &later.attempts[&0].recorded,
            later.attempts[&1].recorded.len()
        ),
        (&ids([&job]), 0)
    );
}

/// #403: a running leader is its own tree's subreaper, and an orphan of
/// the tree was born while the tree ran. So an attempt could have left an
/// orphan only if the orphan does not predate it, its leader no longer
/// runs, and the orphan was born no later than the youngest birth the read
/// that found the attempt ended showed. An orphan no attempt could have
/// left is the engine's own (git's detached maintenance), is never
/// attributed later, and is forgotten once it is gone.
#[test]
fn an_orphan_no_attempt_could_have_left_is_the_engines_own() {
    let job = row(STRANGER, me(), STRANGER);
    let other_group = GROUP + 100;
    let open = |group| Live {
        open: true,
        ..closed(group, &[], &[])
    };
    let leaders = [
        row(GROUP, me(), GROUP),
        zombie(row(other_group, me(), other_group)),
    ];
    let led = table(leaders.into_iter().chain([job.clone()]));

    let mut predates = registry([closed(GROUP, &[], std::slice::from_ref(&job))]);
    predates.observe(&led);
    let mut running = registry([open(GROUP)]);
    running.observe(&led);
    for own in [&predates, &running] {
        let attempt = &own.attempts[&0];
        assert_eq!((attempt.recorded.len(), attempt.doubted.len()), (0, 0));
        assert_eq!(own.unowned, ids([&job]));
    }
    // The engine's own stays its own, whatever runs later.
    running.attempts.get_mut(&0).unwrap().open = false;
    running.observe(&led);
    assert_eq!(running.attempts[&0].recorded.len(), 0);
    running.observe(&table([]));
    assert_eq!(running.unowned, BTreeSet::new(), "forgotten once gone");

    let mut exited = registry([open(GROUP), open(other_group)]);
    exited.observe(&led);
    assert_eq!(
        (
            exited.attempts[&0].recorded.len(),
            &exited.attempts[&1].recorded
        ),
        (0, &ids([&job]))
    );

    // Births order by start stamp: these rows are born at their pids.
    let born = |pid: i32| u64::try_from(pid).unwrap();
    let ended = |at| Live {
        ended: Some(born(at)),
        ..closed(GROUP, &[], &[])
    };
    let alone = table([job.clone()]);
    let mut earlier = registry([ended(STRANGER - 1)]);
    earlier.observe(&alone);
    assert_eq!(
        (earlier.attempts[&0].recorded.len(), &earlier.unowned),
        (0, &ids([&job]))
    );
    assert_eq!(earlier.attempts[&0].ended, Some(born(STRANGER - 1)));
    let mut later = registry([ended(STRANGER)]);
    later.observe(&alone);
    assert_eq!(later.attempts[&0].recorded, ids([&job]));
    assert_eq!(later.attempts[&0].ended, None, "its orphan runs");
    let mut unnoted = registry([closed(GROUP, &[], &[])]);
    unnoted.observe(&table([row(STRANGER + 5, 1, STRANGER + 5)]));
    assert_eq!(unnoted.attempts[&0].ended, Some(born(STRANGER + 5)));
}

/// #403: a zombie the engine adopted is reaped at the next read, whatever
/// attempt explains it, or none. Here an orphan leaves the attempt's
/// group (`setsid`) and exits under a driver that never reaps it (`exec
/// sleep`), before any read could see it run, so nothing records it. It
/// comes to the engine once the driver exits, and no attempt explains it,
/// live or dropped. The read is the engine's own, so no live attempt's
/// leader is taken for an orphan. The orphan runs on the host's schedule,
/// not the driver's: on a loaded host it can outlive the driver's 0.2 s,
/// and a read then finds it running with the engine (#504). So reads are
/// taken until one shows it exited, and that read and the next are judged.
#[cfg(target_os = "linux")]
#[test]
fn an_orphan_that_exited_before_its_first_read_is_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("orphan");
    let mut driver = Command::new("sh");
    driver
        .args([
            "-c",
            "setsid true & printf '%s\\n' \"$!\" > \"$1\"; exec sleep 0.2",
        ])
        .arg("driver")
        .arg(&file);
    let (mut leader, attempt) = Attempt::spawn(&mut driver, Host::REAL).unwrap();
    leader.wait().unwrap();
    drop(attempt);
    let orphan: i32 = std::fs::read_to_string(&file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let listed = || {
        let entries = live().read(Host::REAL.table).unwrap();
        entries.into_iter().find(|entry| entry.id.pid == orphan)
    };
    let until = Instant::now() + Duration::from_secs(10);
    let adopted = loop {
        let adopted = listed();
        if adopted.as_ref().is_none_or(|entry| entry.zombie) || Instant::now() >= until {
            break adopted;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        adopted
            .as_ref()
            .is_none_or(|entry| entry.zombie && entry.ppid == me()),
        "{adopted:?}"
    );
    assert_eq!(
        listed(),
        None,
        "the orphan {orphan} stayed the engine's zombie"
    );
}

/// #470: the reap stops at the engine's own group. A child the engine
/// spawned there, and still holds a `Child` to, reads as a same-group
/// grandchild the subreaper adopted would: the engine's child, in the
/// engine's group. So a read that shows its zombie leaves it to its
/// handle, whose wait still returns its status. Widen the reap to every
/// child of the engine and the wait fails. Linux only: the reap is a
/// no-op on macOS.
#[cfg(target_os = "linux")]
#[test]
fn a_zombie_in_the_engines_group_is_left_to_its_handle() {
    let mut child = Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let listed = || {
        let entries = live().read(Host::REAL.table).unwrap();
        entries.into_iter().find(|entry| entry.id.pid == pid)
    };
    let until = Instant::now() + Duration::from_secs(10);
    let seen = loop {
        let seen = listed();
        if seen.as_ref().is_some_and(|entry| entry.zombie) || Instant::now() >= until {
            break seen;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let group = rustix::process::getpgrp().as_raw_pid();
    assert!(
        seen.as_ref()
            .is_some_and(|entry| entry.zombie && (entry.ppid, entry.pgid) == (me(), group)),
        "{seen:?}"
    );
    assert_eq!(child.wait().unwrap().code(), Some(7));
}

/// #403 finding 3: a host that refused the engine a means parks an
/// attempt that had descendants, naming the means, and leaves one that
/// had none settled.
#[test]
fn a_missing_means_parks_an_attempt_that_had_descendants() {
    let subreaper = Unsettled::Subreaper(Errno::INVAL.raw_os_error());
    let pidfd = Unsettled::Pidfd(Errno::NOSYS.raw_os_error());
    let had = closed(GROUP, &[row(RECORDED, GROUP, GROUP)], &[]);
    let had_none = closed(GROUP, &[], &[]);
    assert_eq!(had.doubts(Some(subreaper.clone())), Err(subreaper.clone()));
    assert_eq!(had.doubts(None), Ok(()));
    assert_eq!(had_none.doubts(Some(pidfd.clone())), Ok(()));
    assert_eq!(
        subreaper.to_string(),
        format!(
            "it had descendants, and the engine is no child subreaper here: {}",
            std::io::Error::from(Errno::INVAL)
        )
    );
    assert_eq!(
        pidfd.to_string(),
        format!(
            "it had descendants, and this kernel gives the engine no pidfd: {}",
            std::io::Error::from(Errno::NOSYS)
        )
    );
}

/// A closed attempt's group is never signalled again: its leader may be
/// reaped and its id reused. Its recorded and doubted identities still
/// are, and a refusal of any kill is carried, the group's first, once
/// every kill was tried.
#[test]
fn a_closed_group_is_not_signalled_and_every_refusal_is_carried() {
    type KillGroup = fn(Pid) -> rustix::io::Result<()>;
    let refused: KillGroup = |_| Err(Errno::PERM);
    let gone: KillGroup = |_| Err(Errno::SRCH);
    let host = |kill_group: KillGroup, kill: fn(&Identity) -> std::io::Result<()>| Host {
        kill_group,
        kill,
        table: populated,
        ..Host::REAL
    };
    let mut live = closed(GROUP, &[], &[]);
    assert_eq!(live.kill(host(refused, spared)), Ok(()));
    live.open = true;
    assert_eq!(
        live.kill(host(refused, spared)),
        Err(Unsettled::Kill {
            group: GROUP,
            errno: Errno::PERM.raw_os_error(),
        })
    );
    assert_eq!(
        live.kill(host(gone, spared)),
        Ok(()),
        "an empty group is no refusal"
    );
    live.doubted = ids([&row(STRANGER, me(), STRANGER)]);
    let signal = Unsettled::Signal {
        pid: STRANGER,
        error: std::io::Error::from(Errno::PERM).to_string(),
    };
    assert_eq!(live.kill(host(gone, forbidden)), Err(signal.clone()));
    assert_eq!(
        signal.to_string(),
        format!(
            "its descendant {STRANGER} could not be signalled: {}",
            std::io::Error::from(Errno::PERM)
        )
    );
    live.recorded = ids([&row(RECORDED, GROUP, GROUP)]);
    assert!(matches!(
        live.kill(host(refused, forbidden)),
        Err(Unsettled::Kill { group: GROUP, .. })
    ));
    assert_eq!(
        live.running(&table([row(RECORDED, GROUP, RECORDED)]), forbidden),
        Err(Unsettled::Signal {
            pid: RECORDED,
            error: std::io::Error::from(Errno::PERM).to_string(),
        })
    );
}

/// #403 on macOS: Darwin answers a signal to a group whose members are
/// all zombies or exiting with EPERM. A fresh read of the table decides
/// it: a group with no member running is gone, as ESRCH's is; one with a
/// member running is a refusal, and parks. A table that cannot be read
/// parks it too, on what the read shows: an identity's refusal first,
/// otherwise the read's own failure, as Linux, which signals a zombie
/// group, reports the same attempt.
#[test]
fn a_group_refused_with_eperm_is_gone_only_when_the_table_shows_it_gone() {
    fn zombies() -> Result<Vec<Entry>, TableError> {
        Ok(table([
            zombie(row(GROUP, me(), GROUP)),
            zombie(row(RECORDED, GROUP, GROUP)),
            row(STRANGER, 1, STRANGER),
        ]))
    }
    fn exiting() -> Result<Vec<Entry>, TableError> {
        Ok(table([
            zombie(row(GROUP, me(), GROUP)),
            Entry {
                exiting: true,
                ..row(RECORDED, GROUP, GROUP)
            },
        ]))
    }
    let refused = |table| Host {
        kill_group: |_| Err(Errno::PERM),
        kill: spared,
        table,
        ..Host::REAL
    };
    let live = Live {
        open: true,
        ..closed(GROUP, &[], &[])
    };
    let kill = Unsettled::Kill {
        group: GROUP,
        errno: Errno::PERM.raw_os_error(),
    };
    assert_eq!(live.kill(refused(zombies)), Ok(()));
    assert_eq!(live.kill(refused(exiting)), Ok(()));
    assert_eq!(live.kill(refused(populated)), Err(kill.clone()));
    let unreadable = || Err(TableError::NoSelf { pid: 0 });
    let unread = Unsettled::Table {
        error: TableError::NoSelf { pid: 0 }.to_string(),
    };
    assert_eq!(live.kill(refused(unreadable)), Err(unread.clone()));
    assert_eq!(
        unread.to_string(),
        "the process table could not be read: the table has no row for the engine itself (pid 0)"
    );
    let mut recorded = Live {
        open: true,
        ..closed(GROUP, &[row(RECORDED, GROUP, RECORDED)], &[])
    };
    let forbidding = Host {
        kill: forbidden,
        ..refused(unreadable)
    };
    let signal = Unsettled::Signal {
        pid: RECORDED,
        error: std::io::Error::from(Errno::PERM).to_string(),
    };
    assert_eq!(recorded.kill(forbidding), Err(signal));
    recorded.recorded.clear();
    assert_eq!(recorded.kill(forbidding), Err(unread));
    assert_eq!(
        kill.to_string(),
        format!(
            "its process group {GROUP} could not be signalled: {}",
            std::io::Error::from(Errno::PERM)
        )
    );
    // Only EPERM is read against the table: any other refusal stands,
    // even where the table would show the group gone.
    let invalid = Host {
        kill_group: |_| Err(Errno::INVAL),
        kill: spared,
        table: zombies,
        ..Host::REAL
    };
    assert_eq!(
        live.kill(invalid),
        Err(Unsettled::Kill {
            group: GROUP,
            errno: Errno::INVAL.raw_os_error(),
        })
    );
}

/// #918: the tree walk can miss a recorded identity while the kernel
/// reparents it. `running` reads its own row, so a spared survivor parks
/// the attempt rather than being read as gone. Without that row, the
/// entries hold the engine alone and the attempt settles.
#[cfg(target_os = "linux")]
#[test]
fn a_recorded_identity_missing_from_the_walk_is_read_directly() {
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let id = table::snapshot()
        .unwrap()
        .into_iter()
        .find(|entry| entry.id.pid == pid)
        .expect("the stub is in the table")
        .id;
    let mut live = closed(GROUP, &[], &[]);
    live.recorded.insert(id);
    live.recorded.insert(Identity {
        pid: 999_999_999,
        start: "1".into(),
    });
    let outcome = live.running(&[row(me(), 1, me())], spared);
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(outcome, Err(Unsettled::Descendants { pids: vec![pid] }));
}

/// #918: a recorded identity whose row cannot be read is no proof it is
/// gone; the attempt parks with the read's failure.
#[cfg(target_os = "linux")]
#[test]
fn a_recorded_identity_whose_row_cannot_be_read_parks_the_attempt() {
    let id = Identity {
        pid: 999_999_999,
        start: "1".into(),
    };
    let mut live = closed(GROUP, &[], &[]);
    live.recorded.insert(id);
    assert_eq!(
        live.rows(&[row(me(), 1, me())], |_| {
            Err(TableError::Row {
                pid: 999_999_999,
                error: std::io::Error::from_raw_os_error(libc::EACCES),
            })
        }),
        Err(Unsettled::Table {
            error:
                "the row of process 999999999 could not be read: Permission denied (os error 13)"
                    .into(),
        })
    );
}
