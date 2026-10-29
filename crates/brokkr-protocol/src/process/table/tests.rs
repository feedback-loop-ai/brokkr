use super::*;

use std::process::{Child, Command};
use std::time::{Duration, Instant};

fn me() -> i32 {
    rustix::process::getpid().as_raw_pid()
}

fn row(pid: i32) -> Option<Entry> {
    snapshot()
        .unwrap()
        .into_iter()
        .find(|entry| entry.id.pid == pid)
}

/// Poll the table until `pid` is gone or a zombie, for up to two seconds.
fn ended(pid: i32) -> bool {
    let until = Instant::now() + Duration::from_secs(2);
    while row(pid).is_some_and(|entry| !entry.zombie) {
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// A stub a test may fail before it ends it: killed and reaped on drop,
/// so a failed assertion cannot leak it for its own half minute (#470).
struct Guard(Child);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The guard ends its stub even when the test does not: dropped with
/// nothing done to it, the stub is killed at once, not left for its own
/// half minute, and reaped, so the table holds no row of it, not even a
/// zombie. Without the kill the drop waits the stub out; without the
/// wait its zombie stays.
#[test]
fn the_guard_ends_its_stub_on_drop() {
    let stub = Guard(Command::new("sleep").arg("30").spawn().unwrap());
    let pid = i32::try_from(stub.0.id()).unwrap();
    let id = row(pid).expect("the stub is in the table").id;
    let began = Instant::now();
    drop(stub);
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "the stub was not killed"
    );
    let left = row(pid).filter(|entry| entry.id == id);
    assert_eq!(left, None, "the stub was not reaped");
}

/// The table names a child that leads a group of its own with its parent,
/// its group and a start stamp that reads the same twice, whatever group
/// the harness put this process in. The child is registered as an
/// attempt, which leads a session and group of its own: it would
/// otherwise read, to another test's attempt, as an orphan this process
/// adopted.
#[test]
fn the_table_reads_a_child_that_leads_its_group() {
    let mut sleep = Command::new("sleep");
    sleep.arg("30");
    let host = super::super::tree::Host::REAL;
    let (mut child, _registered) =
        super::super::attempts::Attempt::spawn(&mut sleep, host).unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let entry = row(pid).expect("the child is in the table");
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!((entry.ppid, entry.pgid, entry.zombie), (me(), pid, false));
    assert!(!entry.id.start.is_empty());
    assert!(row(me()).is_some(), "this process is in the table");
}

/// A kill reaches the identity it names and nothing else: a pid with
/// another start stamp is not signalled, and a pid already gone is left.
/// None of them is a refusal.
#[test]
fn a_kill_reaches_only_the_identity_it_names() {
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let id = row(pid).unwrap().id;
    let stranger = Identity {
        pid,
        start: format!("{}0", id.start),
    };
    assert!(kill(&stranger).is_ok());
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        child.try_wait().unwrap().is_none(),
        "a stranger was signalled"
    );
    assert!(kill(&id).is_ok());
    assert!(
        child.wait().unwrap().code().is_none(),
        "the identity was not killed"
    );
    // Reaped: the pid is free, and a second kill finds nothing to signal.
    assert!(kill(&id).is_ok());
    assert!(row(pid).is_none_or(|entry| entry.id != id));
}

/// A zombie is read as one: it has exited, whoever still holds its pid.
#[test]
fn a_zombie_reads_as_a_zombie() {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    assert!(ended(pid));
    assert!(row(pid).unwrap().zombie);
    child.wait().unwrap();
    assert!(row(pid).is_none());
}

#[cfg(target_os = "linux")]
#[test]
fn a_stat_line_is_read_from_its_last_parenthesis() {
    let stat = "7 (a) b) S 1 7 7 0 -1 4194560 0 0 0 0 0 0 0 0 20 0 1 0 12345 0 0";
    assert_eq!(
        parse_stat(7, stat),
        Some(Entry {
            id: Identity {
                pid: 7,
                start: "12345".into(),
            },
            ppid: 1,
            pgid: 7,
            zombie: false,
            exiting: false,
        })
    );
    assert_eq!(parse_stat(7, "7 (cut short) Z 1"), None);
}

/// A `/proc` of `rows`, each a pid directory holding `stat` as a file of
/// the given text, or as a directory when the text is `None`.
#[cfg(target_os = "linux")]
fn proc_of(rows: &[(&str, Option<&str>)]) -> tempfile::TempDir {
    let proc = tempfile::tempdir().unwrap();
    for (name, stat) in rows {
        let dir = proc.path().join(name);
        std::fs::create_dir(&dir).unwrap();
        match stat {
            Some(stat) => std::fs::write(dir.join("stat"), stat).unwrap(),
            None => std::fs::create_dir(dir.join("stat")).unwrap(),
        }
    }
    proc
}

/// A `/proc` of a tree: `root` and each `(pid, ppid)` in `tree` has a
/// `stat` and a single task's `children` naming its direct children.
/// `unrelated` further processes are on the host, outside the tree.
#[cfg(target_os = "linux")]
fn tree_proc(root: i32, tree: &[(i32, i32)], unrelated: usize) -> tempfile::TempDir {
    let proc = tempfile::tempdir().unwrap();
    let stat = |pid: i32, ppid: i32| {
        format!(
            "{pid} (row) S {ppid} {pid} {pid} 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 {} 0 0",
            pid + 100
        )
    };
    for (pid, ppid) in std::iter::once(&(root, 1)).chain(tree) {
        let dir = proc.path().join(pid.to_string());
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("stat"), stat(*pid, *ppid)).unwrap();
        let task = dir.join("task").join(pid.to_string());
        std::fs::create_dir_all(&task).unwrap();
        let children: String = tree
            .iter()
            .filter(|(_, parent)| parent == pid)
            .map(|(child, _)| format!("{child} "))
            .collect();
        std::fs::write(task.join("children"), children).unwrap();
    }
    for pid in 10_000..10_000 + i32::try_from(unrelated).unwrap() {
        let dir = proc.path().join(pid.to_string());
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("stat"), stat(pid, 1)).unwrap();
    }
    proc
}

/// A planted tree of `root` and `(501, 500)` whose `path` is a
/// directory, so reading it fails with `EISDIR`.
#[cfg(target_os = "linux")]
fn unreadable(path: &str) -> tempfile::TempDir {
    let proc = tree_proc(500, &[(501, 500)], 0);
    std::fs::remove_file(proc.path().join(path)).unwrap();
    std::fs::create_dir(proc.path().join(path)).unwrap();
    proc
}

/// #918: the Linux tracker reads the engine's own subtree, so the stat
/// files it opens do not grow with the host's unrelated processes. Ten
/// and two thousand of them cost the same three reads, root included,
/// and the rows are the tree's alone.
#[cfg(target_os = "linux")]
#[test]
fn the_engine_tree_read_does_not_grow_with_the_host() {
    let tree = [(501, 500), (502, 501)];
    let stat = |pid: i32, ppid: i32| {
        format!(
            "{pid} (row) S {ppid} {pid} {pid} 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 {} 0 0",
            pid + 100
        )
    };
    let expected: Vec<Entry> = std::iter::once((500, 1))
        .chain(tree)
        .map(|(pid, ppid)| parse_stat(pid, &stat(pid, ppid)).unwrap())
        .collect();
    let reads = [10, 2_000].map(|unrelated| {
        let proc = tree_proc(500, &tree, unrelated);
        clear_stat_reads();
        let mut entries = own_tree_in(proc.path(), 500).unwrap();
        entries.sort_by_key(|entry| entry.id.pid);
        assert_eq!(entries, expected);
        stat_reads()
    });
    assert_eq!(reads, [3, 3]);
}

/// The row error `error` carries when it names `pid`.
#[cfg(target_os = "linux")]
fn row_failure(error: TableError, pid: i32) -> std::io::Error {
    match error {
        TableError::Row { pid: named, error } => {
            assert_eq!(named, pid, "wrong row: {error}");
            error
        }
        other => panic!("expected the row of process {pid}, got {other:?}"),
    }
}

/// #918: a process or task that exits between being named and its read
/// is skipped; a row or task the walk cannot read fails it, as the
/// snapshot fails on one, because it could be hiding the attempt's. A
/// returned entry keeps the parent its own `stat` names.
#[cfg(target_os = "linux")]
#[test]
fn a_tree_walk_skips_what_exited_and_fails_what_it_cannot_read() {
    let tree = [(501, 500)];
    let pids =
        |entries: &[Entry]| -> Vec<i32> { entries.iter().map(|entry| entry.id.pid).collect() };

    // A child that exited between being named and its own read is gone.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_dir_all(proc.path().join("501")).unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500]);

    // A task that exited after its process was listed is skipped.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_dir_all(proc.path().join("501/task")).unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500, 501]);

    // A task whose children file is gone is skipped: it exited.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_file(proc.path().join("501/task/501/children")).unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500, 501]);

    // A task list with no task in it names no children.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_dir_all(proc.path().join("501/task")).unwrap();
    std::fs::create_dir(proc.path().join("501/task")).unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500, 501]);

    // A child named twice is followed once.
    let proc = tree_proc(500, &tree, 0);
    std::fs::write(proc.path().join("500/task/500/children"), "501 501").unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500, 501]);

    // A children file that cannot be read is no tree.
    let proc = unreadable("501/task/501/children");
    assert_eq!(
        row_failure(own_tree_in(proc.path(), 500).unwrap_err(), 501).raw_os_error(),
        Some(libc::EISDIR)
    );

    // A task directory that cannot be listed is no tree.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_dir_all(proc.path().join("501/task")).unwrap();
    std::fs::write(proc.path().join("501/task"), "").unwrap();
    assert_eq!(
        row_failure(own_tree_in(proc.path(), 500).unwrap_err(), 501).raw_os_error(),
        Some(libc::ENOTDIR)
    );

    // A child's row that cannot be read is no tree.
    let proc = unreadable("501/stat");
    assert_eq!(
        row_failure(own_tree_in(proc.path(), 500).unwrap_err(), 501).raw_os_error(),
        Some(libc::EISDIR)
    );

    // A children line that does not name a pid is no tree.
    let proc = tree_proc(500, &tree, 0);
    std::fs::write(proc.path().join("501/task/501/children"), "x").unwrap();
    let error = row_failure(own_tree_in(proc.path(), 500).unwrap_err(), 501);
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "unparsed child pid \"x\"");

    // The engine's own row is required; one it cannot read is no tree.
    let proc = tree_proc(500, &tree, 0);
    std::fs::remove_file(proc.path().join("500/stat")).unwrap();
    assert!(matches!(
        own_tree_in(proc.path(), 500).unwrap_err(),
        TableError::NoSelf { pid: 500 }
    ));
    std::fs::create_dir(proc.path().join("500/stat")).unwrap();
    assert_eq!(
        row_failure(own_tree_in(proc.path(), 500).unwrap_err(), 500).raw_os_error(),
        Some(libc::EISDIR)
    );
}

/// #918 H1: a child's own `stat` names its parent, never the listing. A
/// pid that A's `children` named, now reused by an unrelated process U
/// with another parent, is dropped from the walk: `Live::record` never
/// records it and `Live::kill` never signals it. A process that truly
/// moved is found under its real parent on the next read.
#[cfg(target_os = "linux")]
#[test]
fn a_reused_child_pid_is_not_walked_from_the_listing() {
    let pids =
        |entries: &[Entry]| -> Vec<i32> { entries.iter().map(|entry| entry.id.pid).collect() };
    let proc = tree_proc(500, &[(501, 500), (502, 501)], 0);
    // 502 is now U: its own stat names 900, a process outside the tree.
    let stranger = "502 (u) S 900 502 502 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 602 0 0";
    std::fs::write(proc.path().join("502/stat"), stranger).unwrap();
    assert_eq!(pids(&own_tree_in(proc.path(), 500).unwrap()), [500, 501]);

    // A process that truly moved to the engine is found there on the next
    // read, with the parent its own stat names.
    std::fs::write(
        proc.path().join("502/stat"),
        "502 (row) S 500 502 502 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 602 0 0",
    )
    .unwrap();
    std::fs::write(proc.path().join("500/task/500/children"), "501 502 ").unwrap();
    let entries = own_tree_in(proc.path(), 500).unwrap();
    let moved = entries
        .iter()
        .find(|entry| entry.id.pid == 502)
        .expect("the moved process is under its real parent");
    assert_eq!(moved.ppid, 500);
}

/// #918: the table errors name the row they could not read once.
#[cfg(target_os = "linux")]
#[test]
fn the_table_errors_name_what_they_could_not_read() {
    let row = TableError::Row {
        pid: 501,
        error: std::io::Error::from_raw_os_error(libc::EISDIR),
    };
    assert_eq!(
        row.to_string(),
        format!(
            "the row of process 501 could not be read: {}",
            std::io::Error::from_raw_os_error(libc::EISDIR)
        )
    );
    assert_eq!(
        TableError::NoSelf { pid: 500 }.to_string(),
        "the table has no row for the engine itself (pid 500)"
    );
}

/// #918: `Host::REAL.tick` is the subtree walk on Linux, not the whole
/// host table, and `Host::REAL.table` is the full table admission and the
/// kill read: the engine's parent is not its descendant, so the tick does
/// not read it, and the full read does.
#[cfg(target_os = "linux")]
#[test]
fn the_real_host_reads_its_own_tree_at_the_tick_and_the_full_table_elsewhere() {
    let me = rustix::process::getpid().as_raw_pid();
    let parent = rustix::process::getppid()
        .expect("a test process has a parent")
        .as_raw_pid();
    let pids = |entries: Vec<Entry>| -> std::collections::BTreeSet<i32> {
        entries.iter().map(|entry| entry.id.pid).collect()
    };
    let tick = pids((super::super::tree::Host::REAL.tick)().unwrap());
    assert!(tick.contains(&me), "the engine itself is not in its tree");
    assert!(
        !tick.contains(&parent),
        "the tick listed the engine's parent {parent}"
    );
    let full = pids((super::super::tree::Host::REAL.table)().unwrap());
    assert!(full.contains(&me), "the full table omits the engine");
    assert!(
        full.contains(&parent),
        "the full table omits the engine's parent {parent}"
    );
}

/// #403: a row that vanished between the listing and its read is gone,
/// and the snapshot reads on; a row that cannot be read, or parsed, could
/// be the attempt's, so it fails the snapshot rather than being dropped;
/// and a `/proc` that cannot be listed is no table at all.
#[cfg(target_os = "linux")]
#[test]
fn a_row_that_cannot_be_read_fails_the_snapshot() {
    let stat = "7 (sh) S 1 7 7 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 12345 0 0";
    let proc = proc_of(&[("7", Some(stat)), ("self", Some("not a row"))]);
    std::fs::create_dir(proc.path().join("8")).unwrap();
    let entries = snapshot_in(proc.path()).unwrap();
    assert_eq!(entries, vec![parse_stat(7, stat).unwrap()]);

    let unreadable = proc_of(&[("9", None)]);
    assert_eq!(
        snapshot_in(unreadable.path()).unwrap_err().to_string(),
        format!(
            "the row of process 9 could not be read: {}",
            std::io::Error::from_raw_os_error(libc::EISDIR)
        )
    );
    let unparsed = proc_of(&[("10", Some("garbage"))]);
    assert_eq!(
        snapshot_in(unparsed.path()).unwrap_err().to_string(),
        "the row of process 10 could not be read: unparsed stat \"garbage\""
    );
    let gone = proc.path().join("gone");
    assert_eq!(
        snapshot_in(&gone).unwrap_err().to_string(),
        format!(
            "/proc could not be listed: {}",
            std::io::Error::from_raw_os_error(libc::ENOENT)
        )
    );
}

#[test]
fn a_ps_row_keeps_its_start_time_whole() {
    assert_eq!(
        parse_ps(" 7  1  7 Z+  Mon Sep  8 10:00:00 2026"),
        Some(Entry {
            id: Identity {
                pid: 7,
                start: "Mon Sep 8 10:00:00 2026".into(),
            },
            ppid: 1,
            pgid: 7,
            zombie: true,
            exiting: false,
        })
    );
    assert_eq!(parse_ps("7 1"), None);
}

/// #403 on macOS: BSD `ps` flags a process trying to exit `E`, after its
/// state letter, and the listing reads it exiting, not a zombie.
#[test]
fn a_ps_row_flagged_e_reads_as_exiting() {
    let stamp = "Mon Sep 28 10:00:00 2026";
    let rows = listed(output(
        0,
        &format!("7 1 7 RE {stamp}\n8 7 7 SE+ {stamp}\n9 7 7 Ss {stamp}\n"),
    ))
    .unwrap();
    let states: Vec<(i32, bool, bool)> = rows
        .iter()
        .map(|entry| (entry.id.pid, entry.zombie, entry.exiting))
        .collect();
    assert_eq!(
        states,
        [(7, false, true), (8, false, true), (9, false, false)]
    );
}

/// #403: a row whose start time is missing, is `-` (what BSD `ps` prints
/// for a process it cannot inspect), or is not `lstart`'s date is not
/// read whole: its stamp would name no process.
#[test]
fn a_ps_row_without_its_start_time_is_not_read() {
    assert_eq!(parse_ps(" 7  1  7 S"), None, "four fields");
    assert_eq!(parse_ps(" 7  1  7 S  -"), None, "a dash for the stamp");
    for stamp in [
        "Mon Sep 28 10:00:00",
        "Xyz Sep 28 10:00:00 2026",
        "Mon Xyz 28 10:00:00 2026",
        "Mon Sep x 10:00:00 2026",
        "Mon Sep 0 10:00:00 2026",
        "Mon Sep 32 10:00:00 2026",
        "Mon Sep 28 10:0x:00 2026",
        "Mon Sep 28 10:00 2026",
        "Mon Sep 28 24:00:00 2026",
        "Mon Sep 28 10:60:00 2026",
        "Mon Sep 28 10:00:61 2026",
        "Mon Sep 28 10:00:00 226",
        "Mon Sep 28 10:00:00 2o26",
    ] {
        assert_eq!(parse_ps(&format!("7 1 7 S {stamp}")), None, "{stamp}");
    }
    assert!(parse_ps("7 1 7 S Sat Dec 31 23:59:60 2016").is_some());
}

/// The rows `ps` printed, as `output` with exit status `code`.
fn output(code: i32, stdout: &str) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;
    std::process::Output {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

/// #403: macOS's kill signals only an identity `ps` lists running; a pid
/// `ps` no longer lists (exit 1) is gone, and a listing that cannot be
/// read, such as a row without its stamp, is returned rather than read as
/// the identity gone.
#[test]
fn a_kill_by_listing_needs_the_row_whole() {
    let mut child = Guard(Command::new("sleep").arg("30").spawn().unwrap());
    let pid = i32::try_from(child.0.id()).unwrap();
    let row = format!("{pid} 1 {pid} S Mon Sep 28 10:00:00 2026");
    let id = parse_ps(&row).unwrap().id;
    assert!(kill_listed(&id, listed(output(1, ""))).is_ok());
    let zombie = row.replace(" S ", " Z ");
    assert!(kill_listed(&id, listed(output(0, &zombie))).is_ok());
    assert_eq!(
        kill_listed(&id, listed(output(2, &row)))
            .unwrap_err()
            .to_string(),
        format!("ps exited {}", output(2, "").status)
    );
    let stampless = kill_listed(&id, listed(output(0, &format!("{pid} 1 {pid} S\n"))));
    assert_eq!(
        stampless.unwrap_err().to_string(),
        format!("the ps row \"{pid} 1 {pid} S\" could not be read")
    );
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "signalled unconfirmed"
    );
    assert!(kill_listed(&id, listed(output(0, &row))).is_ok());
    assert!(
        child.0.wait().unwrap().code().is_none(),
        "the listed identity was not killed"
    );
}

/// #403: a row that lists the pid under another start stamp names the
/// process that reused it, not `id`, and is not signalled.
#[test]
fn a_kill_by_listing_spares_a_reused_pid() {
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let id = parse_ps(&format!("{pid} 1 {pid} S Mon Sep 28 10:00:00 2026"))
        .unwrap()
        .id;
    let reused = format!("{pid} 1 {pid} S Tue Sep 29 10:00:00 2026");
    kill_listed(&id, listed(output(0, &reused))).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        child.try_wait().unwrap().is_none(),
        "a reused pid was signalled"
    );
    child.kill().unwrap();
    child.wait().unwrap();
}

/// #403: a `ps` that exits nonzero read no table, whatever it printed,
/// and a row it printed that does not parse fails the read.
#[test]
fn a_failed_or_unparsed_ps_is_no_table() {
    let row = " 7  1  7 S  Mon Sep 28 10:00:00 2026";
    assert_eq!(
        listed(output(0, row)).unwrap(),
        vec![parse_ps(row).unwrap()]
    );
    assert!(matches!(
        listed(output(1, row)),
        Err(TableError::Status(status)) if status.code() == Some(1)
    ));
    assert_eq!(
        listed(output(0, "7 1\n")).unwrap_err().to_string(),
        "the ps row \"7 1\" could not be read"
    );
}

/// #918: an identity is read by its own row; a pid that now names
/// another process, or names none, reads as gone, and a row that cannot
/// be read is no proof either way.
#[cfg(target_os = "linux")]
#[test]
fn a_recorded_identity_is_read_by_its_own_row() {
    let stub = Guard(Command::new("sleep").arg("30").spawn().unwrap());
    let pid = i32::try_from(stub.0.id()).unwrap();
    let id = row(pid).expect("the stub is in the table").id;
    assert_eq!(row_of(&id).unwrap().map(|entry| entry.id), Some(id.clone()));
    assert_eq!(
        row_of(&Identity {
            pid,
            start: format!("{}0", id.start),
        })
        .unwrap(),
        None
    );
    assert_eq!(
        row_of(&Identity {
            pid: 999_999_999,
            start: "1".into(),
        })
        .unwrap(),
        None
    );

    let stat = "7 (sh) S 1 7 7 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 12345 0 0";
    let planted = proc_of(&[("7", Some(stat))]);
    let planted_id = parse_stat(7, stat).unwrap().id;
    assert_eq!(
        row_in(planted.path(), &planted_id)
            .unwrap()
            .map(|entry| entry.id),
        Some(planted_id.clone())
    );
    let reused = Identity {
        pid: 7,
        start: "54321".into(),
    };
    assert_eq!(row_in(planted.path(), &reused).unwrap(), None);
    let unreadable = proc_of(&[("7", None)]);
    assert_eq!(
        row_failure(row_in(unreadable.path(), &planted_id).unwrap_err(), 7).raw_os_error(),
        Some(libc::EISDIR)
    );
}
