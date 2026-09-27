use super::*;

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

/// The table names this process with its parent, group and a start stamp
/// that reads the same twice.
#[test]
fn the_table_reads_this_process() {
    let entry = row(me()).expect("this process is in the table");
    assert_eq!(entry.ppid, rustix::process::getppid().unwrap().as_raw_pid());
    assert_eq!(entry.pgid, rustix::process::getpgrp().as_raw_pid());
    assert!(!entry.zombie);
    assert!(!entry.id.start.is_empty());
    assert_eq!(row(me()).unwrap().id, entry.id);
}

/// A kill reaches the identity it names and nothing else: a pid with
/// another start stamp is not signalled, and a pid already gone is left.
#[test]
fn a_kill_reaches_only_the_identity_it_names() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let pid = i32::try_from(child.id()).unwrap();
    let id = row(pid).unwrap().id;
    kill(&Identity {
        pid,
        start: format!("{}0", id.start),
    });
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        child.try_wait().unwrap().is_none(),
        "a stranger was signalled"
    );
    kill(&id);
    assert!(
        child.wait().unwrap().code().is_none(),
        "the identity was not killed"
    );
    // Reaped: the pid is free, and a second kill finds nothing to signal.
    kill(&id);
    assert!(row(pid).is_none_or(|entry| entry.id != id));
}

/// A zombie is read as one: it has exited, whoever still holds its pid.
#[test]
fn a_zombie_reads_as_a_zombie() {
    let mut child = std::process::Command::new("true").spawn().unwrap();
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
            session: "7".into(),
            zombie: false,
        })
    );
    assert_eq!(parse_stat(7, "7 (cut short) Z 1"), None);
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
            session: String::new(),
            zombie: true,
        })
    );
    assert_eq!(parse_ps("7 1"), None);
}
