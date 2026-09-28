use super::*;

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

/// A kill that signals nothing, and one the kernel refuses.
fn spared(_: &Identity) -> std::io::Result<()> {
    Ok(())
}

fn forbidden(_: &Identity) -> std::io::Result<()> {
    Err(Errno::PERM.into())
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
/// leader is taken for an orphan.
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
    let adopted = listed();
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
