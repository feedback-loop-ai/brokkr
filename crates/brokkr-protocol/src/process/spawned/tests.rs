//! The door's two waits report the child's own status and output, and a
//! program that cannot run is the spawn's own error.

use std::io::ErrorKind;
use std::process::Command;

use super::*;

#[test]
fn the_door_reports_the_own_status_and_output_of_a_child() {
    let mut command = Command::new("sh");
    command.args(["-c", "read line; echo out$line; echo err >&2; exit 4"]);
    let ran = output(&mut command).unwrap();
    assert_eq!(
        (ran.status.code(), &ran.stdout[..], &ran.stderr[..]),
        (Some(4), &b"out\n"[..], &b"err\n"[..])
    );
    let ran = status(Command::new("sh").args(["-c", "exit 5"])).unwrap();
    assert_eq!(ran.code(), Some(5));
    let probes = ["exit 0", "exit 6"];
    let succeeded = probes.map(|probe| succeeds(Command::new("sh").args(["-c", probe])));
    assert_eq!(succeeded, [true, false]);
}

#[test]
fn a_program_that_cannot_run_is_the_spawns_own_error() {
    let missing = "/nonexistent/brokkr-spawned-test";
    let kinds = [
        output(&mut Command::new(missing)).unwrap_err().kind(),
        status(&mut Command::new(missing)).unwrap_err().kind(),
    ];
    assert_eq!(kinds, [ErrorKind::NotFound, ErrorKind::NotFound]);
    assert!(!succeeds(&mut Command::new(missing)));
}
