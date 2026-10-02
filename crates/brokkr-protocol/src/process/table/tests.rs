use super::*;

use std::process::Command;
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

/// #504: the kernel flags in `stat` say whether the thread is inside its
/// exit (`PF_EXITING`). The first row is the one the engine read of a
/// box's pid-namespace init at a park on a loaded host: waiting on the
/// disk, its flags 0x40014c. Flags that do not parse fail the row.
#[cfg(target_os = "linux")]
#[test]
fn a_stat_line_reads_the_exiting_flag() {
    let init = "948271 (bwrap) D 948236 948271 948271 0 -1 4194636 138 37506 0 0 0 2 11 8 \
                20 0 1 0 27315461 0 0";
    let read = parse_stat(948_271, init).unwrap();
    assert_eq!(
        (read.zombie, read.exiting, read.runs()),
        (false, true, false)
    );
    let running = init.replace(" 4194636 ", " 4194632 ");
    let read = parse_stat(948_271, &running).unwrap();
    assert_eq!(
        (read.zombie, read.exiting, read.runs()),
        (false, false, true)
    );
    assert_eq!(
        parse_stat(948_271, &init.replace(" 4194636 ", " -4 ")),
        None
    );
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

/// The `stat` of process or thread `pid` in `state` with kernel `flags`.
#[cfg(target_os = "linux")]
fn stat(pid: i32, state: &str, flags: u32) -> String {
    format!("{pid} (bwrap) {state} 1 {pid} {pid} 0 -1 {flags} 0 0 0 0 0 0 0 0 20 0 1 0 12345 0 0")
}

/// Plant the thread `tid` of `pid` under `proc`, its `stat` the given
/// text, or absent when it is `None`: a thread gone since the listing.
#[cfg(target_os = "linux")]
fn thread(proc: &std::path::Path, pid: i32, tid: i32, stat: Option<&str>) {
    let dir = proc
        .join(pid.to_string())
        .join("task")
        .join(tid.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    stat.into_iter()
        .for_each(|stat| std::fs::write(dir.join("stat"), stat).unwrap());
}

/// #504: the flag in a process's `stat` is its leader thread's alone, and
/// a leader can exit first and leave the others running. So a process
/// reads exiting only when every thread its `task` lists is exiting or
/// has ended, a thread or a task list gone since the listing included;
/// a process whose leader is not exiting is not read thread by thread.
/// A thread whose row cannot be read could still run: it fails the read.
#[cfg(target_os = "linux")]
#[test]
fn a_process_reads_exiting_only_when_every_thread_does() {
    let rows = [
        ("7", Some(stat(7, "D", PF_EXITING))),
        ("20", Some(stat(20, "R", PF_EXITING))),
        ("30", Some(stat(30, "D", PF_EXITING))),
        ("40", Some(stat(40, "S", 0))),
    ];
    let rows: Vec<(&str, Option<&str>)> = rows
        .iter()
        .map(|(pid, stat)| (*pid, stat.as_deref()))
        .collect();
    let proc = proc_of(&rows);
    thread(proc.path(), 7, 7, Some(&stat(7, "D", PF_EXITING)));
    thread(proc.path(), 7, 8, Some(&stat(8, "X", 0)));
    thread(proc.path(), 7, 9, None);
    thread(proc.path(), 20, 20, Some(&stat(20, "R", PF_EXITING)));
    thread(proc.path(), 20, 21, Some(&stat(21, "S", 0)));
    thread(proc.path(), 40, 41, Some("not a row"));
    let read: Vec<(i32, bool)> = snapshot_in(proc.path())
        .unwrap()
        .into_iter()
        .map(|entry| (entry.id.pid, entry.exiting))
        .collect();
    assert_eq!(
        read.into_iter().collect::<std::collections::BTreeSet<_>>(),
        [(7, true), (20, false), (30, true), (40, false)].into()
    );

    thread(proc.path(), 7, 10, None);
    std::fs::create_dir(proc.path().join("7/task/10/stat")).unwrap();
    assert_eq!(
        snapshot_in(proc.path()).unwrap_err().to_string(),
        format!(
            "the row of process 7 could not be read: {}",
            std::io::Error::from_raw_os_error(libc::EISDIR)
        )
    );
    let unlisted = proc_of(&[("50", Some(&stat(50, "D", PF_EXITING)))]);
    std::fs::write(unlisted.path().join("50/task"), "").unwrap();
    assert_eq!(
        snapshot_in(unlisted.path()).unwrap_err().to_string(),
        format!(
            "the row of process 50 could not be read: {}",
            std::io::Error::from_raw_os_error(libc::ENOTDIR)
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
    let mut child = Command::new("sleep").arg("30").spawn().unwrap();
    let pid = i32::try_from(child.id()).unwrap();
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
    assert!(child.try_wait().unwrap().is_none(), "signalled unconfirmed");
    assert!(kill_listed(&id, listed(output(0, &row))).is_ok());
    assert!(
        child.wait().unwrap().code().is_none(),
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
