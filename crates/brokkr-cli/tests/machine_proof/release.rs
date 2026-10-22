//! U8e3 through the binary (MB5, CR2; operator ruling 2026-10-21, "A"): a
//! run whose engine was killed while its driver ran keeps its worktree, and
//! a conflicting run is refused by its lease. `brokkr operator release`
//! refuses while the attempt has not ended; once `brokkr conclude` ended
//! it, the release is journaled with its reason, the reservation is gone,
//! and the conflicting run admits. A second release finds nothing to
//! release. Its seats are unboxed, so no namespace is needed; the HOME is
//! one whose ancestors protect it, as writer_lease's is, and Linux's
//! `/proc` shows the operator that the killed run's seat has ended.

#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::{Command, Output};

use serde_json::json;

use crate::writer_lease::{cr2, failures, succeeded, until, Host};

/// What the operator attests.
const REASON: &str = "its engine was killed and every process its attempt owned has ended";

/// `brokkr` run with `args` under the host's HOME.
fn brokkr(host: &Host, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .args(args)
        .current_dir(&host.base)
        .env("HOME", host.base.join("home"))
        .output()
        .unwrap()
}

/// What `brokkr` said on stderr, where it exited with `code`.
fn said(output: &Output, code: i32) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(code), "{stderr}");
    stderr
}

#[test]
fn a_killed_engines_worktree_is_released_by_the_operator_and_a_conflicting_run_admits() {
    let Some(host) = Host::new() else {
        return;
    };
    let (work, _) = host.worktree("work", None);
    let (started, go, pid) = (work.join("started"), work.join("go"), work.join("seat.pid"));
    let waits = format!(
        "echo $$ > '{}'; touch '{}'; while [ ! -e '{}' ]; do sleep 0.05; done",
        pid.display(),
        started.display(),
        go.display()
    );
    let held = host.unboxed("held", &waits);
    let mut killed = host.start(&work, &held);
    let run = killed.run();
    until("its driver runs", || started.exists());
    assert_eq!(host.standing(&work, &run), 1, "its lease stands");
    killed.child.kill().unwrap();
    killed.child.wait().unwrap();
    // What the operator confirms before attesting it: the seat has ended.
    std::fs::write(&go, "").unwrap();
    let seat = std::fs::read_to_string(&pid).unwrap().trim().to_string();
    until("its seat ended", || {
        !Path::new("/proc").join(&seat).exists()
    });

    let mark = work.join("second.ran");
    let second = host.unboxed("second", &format!("touch '{}'", mark.display()));
    let (_, events) = host.ran(&work, &second);
    let holds = format!("run '{run}' holds this worktree, its writers not proven settled");
    assert_eq!(failures(&events), [cr2(&holds)]);
    assert!(!mark.exists(), "the conflicting run's driver never started");

    let db = host.journal("held");
    let db = db.to_str().unwrap();
    let release = [
        "operator", "--run", &run, "--reason", REASON, "release", "--db", db,
    ];
    let refused = |cause: String| {
        format!("error: the operator's release was refused, and the run's writer lease stands: {cause}\n")
    };
    let events = host.events(&held, &run);
    let attempt = events.iter().find_map(|event| event["attempt_id"].as_str());
    let attempt = attempt.unwrap();
    let running = said(&brokkr(&host, &release), 1);
    let not_ended = format!(
        "attempt '{attempt}' has not ended; release only an attempt that ended, once every \
         process it owned has"
    );
    assert_eq!(running, refused(not_ended));
    let conclude = [
        "conclude",
        "--run",
        &run,
        "--reason",
        "its engine was killed",
        "--db",
        db,
    ];
    said(&brokkr(&host, &conclude), 3);
    assert_eq!(
        host.standing(&work, &run),
        1,
        "the ended attempt still holds it"
    );

    let released = said(&brokkr(&host, &release), 0);
    let recorded = format!("recorded operator release; continue with: brokkr resume --run {run}\n");
    assert_eq!(released, recorded);
    let events = host.events(&held, &run);
    let last = events.last().unwrap();
    assert_eq!(last["type"], "operator/commanded");
    let (command, args) = (&last["payload"]["command"], &last["payload"]["args"]);
    assert_eq!(
        (command, args),
        (&json!("release"), &json!({"reason": REASON}))
    );
    assert_eq!(host.standing(&work, &run), 0, "released");

    let (_, events) = host.ran(&work, &second);
    assert!(succeeded(&events), "the conflicting run admits: {events:?}");
    assert!(mark.exists(), "and its driver ran");
    let again = said(&brokkr(&host, &release), 1);
    let unreserved =
        format!("run '{run}' holds no standing reservation, so there is nothing to release");
    assert_eq!(again, refused(unreserved));
}
