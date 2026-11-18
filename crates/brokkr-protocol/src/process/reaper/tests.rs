//! The driver's reaper (#1133). The sweep's decision is held to planted
//! rows; the reaper itself is held to a real driver: this test binary,
//! rerun filtered to the calling test, made a subreaper as the engine's
//! `detach` makes one, serving the exec driver over its stdio.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::adapters::AdapterKind;
use crate::{Body, Message, ResultStatus};

/// How long any wait here may take before the test fails.
const PATIENCE: Duration = Duration::from_secs(30);

/// Set in a rerun: the exec driver's command template, as JSON.
const DRIVING: &str = "BROKKR_TEST_REAPER_DRIVER";

fn identity(pid: i32) -> Identity {
    Identity {
        pid,
        start: format!("{}", pid + 100),
    }
}

fn row(pid: i32, zombie: bool) -> Entry {
    Entry {
        id: identity(pid),
        ppid: 1,
        pgid: 1,
        zombie,
        exiting: false,
    }
}

fn started_at(pid: i32, start: &str) -> Entry {
    Entry {
        id: Identity {
            pid,
            start: start.to_string(),
        },
        ..row(pid, true)
    }
}

/// A zombie no handle holds is reaped on the sweep after the one that
/// first saw it; a registered one never is. A read that failed changes
/// nothing.
#[test]
fn a_sweep_reaps_only_unregistered_zombies_it_saw_before() {
    let mut handled = Handled::default();
    handled.held.insert(10, Hold::Unread);
    let rows = [row(10, true), row(11, true), row(12, false)];
    assert_eq!(handled.doomed(Some(&rows)), Vec::<i32>::new());
    assert_eq!(handled.seen, BTreeSet::from([identity(11)]));
    assert_eq!(handled.doomed(None), Vec::<i32>::new());
    assert_eq!(handled.seen, BTreeSet::from([identity(11)]));
    assert_eq!(handled.doomed(Some(&rows)), [11]);
    assert_eq!(handled.doomed(Some(&rows)), [11]);

    // A pid reused by another process is not the zombie the last sweep saw.
    let reused = started_at(11, "7");
    assert_eq!(handled.doomed(Some(&[reused])), Vec::<i32>::new());
}

/// A child held by its start is released only by its own row: gone, or
/// another process's; one held unread only once its row is gone. A read
/// that fails keeps either, and a pidfd the kernel will not wait on keeps
/// its child.
#[test]
fn a_held_child_is_released_once_its_own_row_is_gone_or_anothers() {
    let mut handled = Handled::default();
    let start = |start: &str| Hold::Start(start.to_string());
    let null = std::fs::File::open("/dev/null").unwrap();
    handled.held.extend([
        (10, start("110")),
        (11, start("111")),
        (12, start("112")),
        (13, Hold::Unread),
        (14, start("114")),
        (15, Hold::Unread),
        (16, Hold::Unread),
        (17, Hold::Pidfd(null.into())),
    ]);
    handled.retire(|pid| match pid {
        11 | 15 => Ok(None),
        12 => Ok(Some(started_at(12, "7"))),
        14 | 16 => Err(std::io::ErrorKind::Other.into()),
        _ => Ok(Some(row(pid, true))),
    });
    assert_eq!(
        handled.held.keys().copied().collect::<Vec<_>>(),
        [10, 13, 14, 16, 17]
    );
}

fn sleeping() -> Command {
    let mut command = Command::new("sleep");
    command.arg("30");
    command
}

fn serving() -> Handled {
    Handled {
        serving: true,
        ..Handled::default()
    }
}

/// A child held by its pidfd stays held while it runs and while it stands
/// a zombie, whatever its row says, and is released by its own handle's
/// reap, even where a row of its pid looks like its own: a pid an orphan
/// reuses holds nothing.
#[test]
fn a_child_held_by_its_pidfd_is_released_by_its_own_reap() {
    let mut handled = serving();
    let mut child = handled
        .spawn(&mut sleeping(), pidfd, table::stat_of)
        .unwrap();
    let pid = Pid::from_child(&child).as_raw_pid();
    let own = table::stat_of(pid).unwrap().unwrap();
    handled.retire(|_| Ok(None));
    assert!(matches!(handled.held.get(&pid), Some(Hold::Pidfd(_))));
    child.kill().unwrap();
    let zombie = until(|| table::stat_of(pid).unwrap().filter(|row| row.zombie));
    assert!(zombie.is_some(), "the child stands a zombie in time");
    handled.retire(|_| Ok(None));
    assert!(handled.held.contains_key(&pid));
    assert_eq!(child.wait().unwrap().signal(), Some(9));
    handled.retire(|_| Ok(Some(own.clone())));
    assert!(handled.held.is_empty(), "{:?}", handled.held);
}

/// A serving driver holds a child in the spawn's own critical section, by
/// its pidfd, else by its start as read then, else unread; a process that
/// does not serve holds nothing, and a spawn that fails holds nothing.
#[test]
fn a_spawn_holds_its_child_before_its_lock_is_released() {
    let ended = |mut child: Child| {
        child.kill().unwrap();
        child.wait().unwrap();
    };
    let closed = |_| Err(std::io::ErrorKind::Other.into());
    let unread = |_| Err(std::io::ErrorKind::Other.into());
    let mut handled = serving();
    let child = handled
        .spawn(&mut sleeping(), pidfd, table::stat_of)
        .unwrap();
    let first = Pid::from_child(&child).as_raw_pid();
    assert!(matches!(handled.held.get(&first), Some(Hold::Pidfd(_))));
    ended(child);

    let child = handled
        .spawn(&mut sleeping(), closed, table::stat_of)
        .unwrap();
    let second = Pid::from_child(&child).as_raw_pid();
    let start = table::stat_of(second).unwrap().unwrap().id.start;
    assert!(matches!(handled.held.get(&second), Some(Hold::Start(held)) if *held == start));
    ended(child);

    let child = handled.spawn(&mut sleeping(), closed, unread).unwrap();
    let third = Pid::from_child(&child).as_raw_pid();
    assert!(matches!(handled.held.get(&third), Some(Hold::Unread)));
    ended(child);

    let missing = handled.spawn(
        &mut Command::new("/nonexistent/brokkr-reaper"),
        pidfd,
        unread,
    );
    assert_eq!(missing.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    assert_eq!(handled.held.len(), 3);

    let mut idle = Handled::default();
    ended(idle.spawn(&mut sleeping(), pidfd, table::stat_of).unwrap());
    assert!(idle.held.is_empty());
}

/// However often a driver serves, only its first serve starts a reaper.
#[test]
fn only_the_first_serve_starts_a_reaper() {
    let mut handled = Handled::default();
    assert_eq!([handled.serve(), handled.serve()], [true, false]);
    assert!(handled.serving);
}

/// A child of the driver's own whose output another process holds open
/// stands a zombie across two sweeps before its handle waits: the door
/// held it, so its status is its own, never `ECHILD`.
#[test]
fn a_probe_whose_output_outlives_it_keeps_its_status() {
    const TEST: &str = "process::reaper::tests::a_probe_whose_output_outlives_it_keeps_its_status";
    if !crate::process::in_its_own_engine(TEST) {
        return;
    }
    start();
    start();
    let mut probe = Command::new("sh");
    probe.args(["-c", "(sleep 2.5 &); echo probed; exit 5"]);
    let output = crate::process::spawned::output(&mut probe).expect("the probe's own status");
    assert_eq!(
        (output.status.code(), &output.stdout[..]),
        (Some(5), &b"probed\n"[..])
    );
}

/// The exec driver this binary serves when it is rerun with [`DRIVING`]
/// set, and nothing otherwise.
fn served() -> bool {
    let Some(extra) = std::env::var_os(DRIVING) else {
        return false;
    };
    let extra: Vec<String> = serde_json::from_str(extra.to_str().unwrap()).unwrap();
    crate::adapters::serve(AdapterKind::Exec, extra, || Ok(())).unwrap();
    true
}

/// Whether `test` runs its body elsewhere: as the driver of a rerun, or
/// in an engine of its own, where no other test's engine reads its
/// driver, which leads a session of its own, as a stray to end.
fn elsewhere(test: &str) -> bool {
    served() || !crate::process::in_its_own_engine(test)
}

/// A driver: this binary rerun as `test`, a subreaper leading a session
/// of its own, serving `script` as its harness in `dir`.
struct Driver {
    child: Child,
    results: Receiver<Message>,
    dir: tempfile::TempDir,
}

impl Driver {
    fn new(test: &str, script: &str) -> Driver {
        let dir = tempfile::tempdir().unwrap();
        let extra = json!(["sh", "-c", script, "{prompt_file}"]);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", test, "--test-threads=1", "--quiet"])
            .env(DRIVING, extra.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // SAFETY: `detach` is async-signal-safe, as every driver's pre-exec.
        unsafe { command.pre_exec(super::super::attempts::detach) };
        let mut child = command.spawn().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (send, results) = mpsc::channel();
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if let Ok(message) = serde_json::from_str::<Message>(&line) {
                    let _ = send.send(message);
                }
            }
        });
        Driver {
            child,
            results,
            dir,
        }
    }

    fn pid(&self) -> i32 {
        Pid::from_child(&self.child).as_raw_pid()
    }

    /// Send one start.
    fn start(&mut self) {
        let dir = self.dir.path();
        let start = json!({"type": "start", "effect_id": "effect", "attempt_id": "attempt",
            "input": {"feature": "feature", "phase": "work", "workdir": dir,
                "result_path": dir.join("result.json"), "allowed_results": ["complete"]}});
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{start}").unwrap();
    }

    /// The status and error of the next result the driver sends.
    fn result(&self) -> (ResultStatus, Option<String>) {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self.results.recv_timeout(left).expect("a result in time");
            if let Body::Result { status, error, .. } = message.body {
                return (status, error);
            }
        }
    }

    /// The harness's pid, once it has written it to `harness.pid`.
    fn harness(&self) -> i32 {
        let path = self.dir.path().join("harness.pid");
        let read = until(|| {
            let text = std::fs::read_to_string(&path).ok()?;
            text.trim().parse().ok()
        });
        read.expect("the harness writes its pid in time")
    }

    /// The driver's children other than `harness`, and whether `harness`
    /// still runs among them.
    fn adopted(&self, harness: i32) -> Option<(usize, bool)> {
        let rows = table::children_of(self.pid())?;
        let runs = rows.iter().any(|row| row.id.pid == harness && !row.zombie);
        let adopted = rows.iter().filter(|row| row.id.pid != harness).count();
        Some((adopted, runs))
    }
}

impl Driver {
    /// Close the driver's stdin, and see it exit as a passing test, so the
    /// rerun's own report stands.
    fn finish(mut self) {
        drop(self.child.stdin.take());
        let status = until(|| self.child.try_wait().unwrap());
        assert!(status.is_some_and(|status| status.success()), "{status:?}");
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        // Bounded: a wait that fails has nothing left to wait for.
        let _ = until(|| match self.child.try_wait() {
            Ok(None) => None,
            _ => Some(()),
        });
    }
}

/// `probe`'s first answer within [`PATIENCE`].
fn until<T>(mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + PATIENCE;
    while Instant::now() < deadline {
        if let Some(found) = probe() {
            return Some(found);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// A harness that orphans `orphans` before it writes its pid, then runs
/// until `done` appears, for [`PATIENCE`] at most, and exits 7.
fn orphaning(orphans: &str) -> String {
    format!(
        "{orphans}; echo $$ > harness.pid; i=0; \
         while [ ! -e done ] && [ $i -lt 600 ]; do sleep 0.05; i=$((i+1)); done; exit 7"
    )
}

/// While the harness runs, every orphan the driver adopted from its tree
/// is reaped; the harness's own exit is reported as before. Each orphan
/// writes its pid to `orphans`, and its subshell has exited before the
/// harness writes its own: all `count` are the driver's by then. Orphans
/// that are `living` still then are each seen among the driver's children.
fn adopted_orphans_are_reaped(test: &str, orphans: &str, count: usize, living: bool) {
    if elsewhere(test) {
        return;
    }
    let mut driver = Driver::new(test, &orphaning(orphans));
    driver.start();
    let harness = driver.harness();
    let recorded = std::fs::read_to_string(driver.dir.path().join("orphans")).unwrap();
    let recorded: BTreeSet<i32> = recorded.lines().map(|pid| pid.parse().unwrap()).collect();
    assert_eq!(recorded.len(), count, "{recorded:?}");
    if living {
        let rows = table::children_of(driver.pid()).unwrap();
        let children: BTreeSet<i32> = rows.iter().map(|row| row.id.pid).collect();
        let strays: Vec<_> = recorded.difference(&children).collect();
        assert_eq!(
            strays,
            Vec::<&i32>::new(),
            "orphans the driver did not adopt"
        );
    }
    let cleared = until(|| driver.adopted(harness).filter(|&(adopted, _)| adopted == 0));
    assert_eq!(
        cleared,
        Some((0, true)),
        "an adopted orphan is left a zombie"
    );
    std::fs::write(driver.dir.path().join("done"), "").unwrap();
    let exited = (ResultStatus::Failed, Some("agent CLI exited 7".to_string()));
    assert_eq!(driver.result(), exited);
    driver.finish();
}

#[test]
fn a_driver_reaps_the_orphan_its_harness_leaves_while_the_harness_runs() {
    adopted_orphans_are_reaped(
        "process::reaper::tests::a_driver_reaps_the_orphan_its_harness_leaves_while_the_harness_runs",
        "(sleep 0.2 & echo $! >> orphans)",
        1,
        false,
    );
}

/// Each orphan sleeps 5 s, past the 200 forks, so all are seen adopted.
#[test]
fn a_driver_reaps_every_one_of_many_adopted_orphans() {
    adopted_orphans_are_reaped(
        "process::reaper::tests::a_driver_reaps_every_one_of_many_adopted_orphans",
        "i=0; while [ $i -lt 200 ]; do \
         (sleep 5 </dev/null >/dev/null 2>&1 & echo $! >> orphans); i=$((i+1)); done",
        200,
        true,
    );
}

/// A harness that exits at once is reaped by its own handle, never by the
/// reaper: each start reports its exit, with no `ECHILD`. The first two
/// leave an orphan holding the harness's stdio for 2.5 s, so the driver
/// reaps the harness only that late, and its zombie stands across sweeps.
#[test]
fn a_harness_that_exits_at_once_is_left_to_its_handle() {
    const TEST: &str = "process::reaper::tests::a_harness_that_exits_at_once_is_left_to_its_handle";
    if elsewhere(TEST) {
        return;
    }
    let script = "n=$(cat runs 2>/dev/null || echo 0); echo $((n+1)) > runs; \
        if [ $n -lt 2 ]; then (sleep 2.5 &); fi; exit 3";
    let mut driver = Driver::new(TEST, script);
    let exited = (ResultStatus::Failed, Some("agent CLI exited 3".to_string()));
    for _ in 0..20 {
        driver.start();
        assert_eq!(driver.result(), exited);
    }
    driver.finish();
}
