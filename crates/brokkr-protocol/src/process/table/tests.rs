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
/// attempt: in a group of its own, it would otherwise read, to another
/// test's attempt, as an orphan this process adopted.
#[test]
fn the_table_reads_a_child_that_leads_its_group() {
    use std::os::unix::process::CommandExt;
    let mut sleep = Command::new("sleep");
    sleep.arg("30").process_group(0);
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

#[cfg(not(target_os = "linux"))]
#[test]
fn a_ps_row_keeps_its_start_time_whole() {
    assert_eq!(
        parse_ps(" 7  1  7 Z+  Mon Sep 28 10:00:00 2026"),
        Some(Entry {
            id: Identity {
                pid: 7,
                start: "Mon Sep 28 10:00:00 2026".into(),
            },
            ppid: 1,
            pgid: 7,
            zombie: true,
        })
    );
    assert_eq!(parse_ps("7 1"), None);
}

/// #403: a `ps` that exits nonzero read no table, whatever it printed,
/// and a row it printed that does not parse fails the read.
#[cfg(not(target_os = "linux"))]
#[test]
fn a_failed_or_unparsed_ps_is_no_table() {
    use std::os::unix::process::ExitStatusExt;
    let output = |code: i32, stdout: &str| std::process::Output {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    };
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
