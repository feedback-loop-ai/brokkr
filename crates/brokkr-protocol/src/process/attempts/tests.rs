use super::*;

/// Pids no test process has as a child, so a reap of one reaps nothing.
const GROUP: i32 = 4_000_001;
const RECORDED: i32 = 4_000_002;
const STRANGER: i32 = 4_000_003;

fn me() -> i32 {
    getpid().as_raw_pid()
}

fn row(pid: i32, ppid: i32, pgid: i32, session: &str) -> Entry {
    Entry {
        id: Identity {
            pid,
            start: format!("start-{pid}"),
        },
        ppid,
        pgid,
        session: session.into(),
        zombie: false,
    }
}

/// This process, in session "mine", and `rows` beside it.
fn table(rows: impl IntoIterator<Item = Entry>) -> Vec<Entry> {
    let mut entries = vec![row(me(), 1, me(), "mine")];
    entries.extend(rows);
    entries
}

/// An attempt planted in the registry with `recorded` and `before`.
fn planted(recorded: &[Entry], before: &[Entry]) -> Attempt {
    let key = NEXT.fetch_add(1, Ordering::Relaxed);
    let ids = |rows: &[Entry]| rows.iter().map(|entry| entry.id.clone()).collect();
    live().insert(
        key,
        Live {
            group: Pid::from_raw(GROUP).unwrap(),
            open: false,
            recorded: ids(recorded),
            before: ids(before),
        },
    );
    Attempt { key }
}

/// The tracker records the leader's descendants to any depth, a process
/// that left its session included, and keeps following what it recorded
/// once the leader is closed; a zombie and a stranger are not recorded.
#[test]
fn the_tracker_records_every_descendant_and_nothing_else() {
    let child = row(RECORDED, GROUP, GROUP, "s");
    let detached = row(RECORDED + 10, RECORDED, RECORDED + 10, "detached");
    let later = row(RECORDED + 11, RECORDED + 10, RECORDED + 10, "detached");
    let zombie = Entry {
        zombie: true,
        ..row(RECORDED + 12, GROUP, GROUP, "s")
    };
    let stranger = row(STRANGER, 1, STRANGER, "other");
    let mut live = Live {
        group: Pid::from_raw(GROUP).unwrap(),
        open: true,
        recorded: BTreeSet::new(),
        before: BTreeSet::new(),
    };
    live.record(&[
        stranger.clone(),
        later.clone(),
        zombie,
        detached.clone(),
        child.clone(),
    ]);
    let ids = |rows: &[&Entry]| {
        rows.iter()
            .map(|entry| entry.id.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(live.recorded, ids(&[&child, &detached, &later]));
    // Closed, the leader's pid names nothing; what was recorded still leads.
    live.open = false;
    let newer = row(RECORDED + 13, RECORDED + 10, RECORDED + 10, "detached");
    let reused = row(RECORDED + 14, GROUP, STRANGER, "other");
    live.record(&[detached.clone(), newer.clone(), reused]);
    assert_eq!(live.recorded, ids(&[&child, &detached, &later, &newer]));
}

/// What `survivors` reads as still running, in the order it is reported:
/// the group, then a recorded descendant, then an adopted orphan nothing
/// recorded. A zombie, whatever holds it, counts as gone.
#[test]
fn survivors_are_the_group_the_recorded_and_the_unattributed_adopted() {
    let recorded = row(RECORDED, me(), RECORDED, "detached");
    let attempt = planted(std::slice::from_ref(&recorded), &[]);
    let member = row(STRANGER, 1, GROUP, "mine");
    assert_eq!(
        attempt.survivors(&table([member.clone(), recorded.clone()])),
        Err(Unsettled::Group { group: GROUP })
    );
    assert_eq!(
        attempt.survivors(&table([recorded.clone()])),
        Err(Unsettled::Descendants {
            pids: vec![RECORDED]
        })
    );
    let stray = row(STRANGER, me(), STRANGER, "detached");
    assert_eq!(
        attempt.survivors(&table([stray.clone()])),
        Err(Unsettled::Strays {
            pids: vec![STRANGER]
        })
    );
    let zombies = [member, recorded, stray].map(|entry| Entry {
        zombie: true,
        ..entry
    });
    assert_eq!(attempt.survivors(&table(zombies)), Ok(()));
}

/// An adopted orphan is not this attempt's to answer for when it was
/// there before the attempt began, or when another attempt recorded it,
/// or when it shares the engine's session (a child the engine spawned).
#[test]
fn an_adopted_orphan_another_record_explains_is_not_a_stray() {
    let before = row(STRANGER, me(), STRANGER, "detached");
    let theirs = row(STRANGER + 1, me(), STRANGER + 1, "detached");
    let spawned = row(STRANGER + 2, me(), STRANGER + 2, "mine");
    let attempt = planted(&[], std::slice::from_ref(&before));
    let other = planted(std::slice::from_ref(&theirs), &[]);
    assert_eq!(attempt.survivors(&table([before, spawned])), Ok(()));
    assert_eq!(
        attempt.survivors(&table([theirs])),
        Ok(()),
        "another live attempt recorded it"
    );
    drop(other);
}

/// A closed attempt's group is never signalled again: its leader may be
/// reaped and its id reused. Its recorded descendants still are.
#[test]
fn a_closed_group_is_not_signalled() {
    let attempt = planted(&[], &[]);
    let refused = |_: Pid| -> rustix::io::Result<()> { Err(Errno::PERM) };
    assert_eq!(live()[&attempt.key].kill(refused), None);
    live().get_mut(&attempt.key).unwrap().open = true;
    assert_eq!(attempt.close(refused), Some(Errno::PERM));
    assert!(!live()[&attempt.key].open);
    let gone = |_: Pid| -> rustix::io::Result<()> { Err(Errno::SRCH) };
    live().get_mut(&attempt.key).unwrap().open = true;
    assert_eq!(attempt.close(gone), None, "an empty group is no refusal");
}
