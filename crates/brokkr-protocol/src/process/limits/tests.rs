//! The transport's limits (#433), each met by a driver past it and by
//! its control at the limit. No provider is contacted: every driver is a
//! shell script that plays prepared bytes.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::process::tests::{accepting, command, succeeded};
use crate::process::{DriverProcess, Host, SpawnEnv};
use crate::{AttemptReport, Body, Cleanup, Message, ResultStatus, PROTO};

const PAST: &str = "; the driver was ended, and what it did is unknown";

/// A driver that accepts the attempt, then plays each of `played` from a
/// file of its own, in order, and stays as long as the last one does.
struct Driver {
    dir: tempfile::TempDir,
    played: Vec<Vec<u8>>,
}

impl Driver {
    fn playing(played: Vec<Vec<u8>>) -> Self {
        Driver {
            dir: tempfile::tempdir().unwrap(),
            played,
        }
    }

    /// The attempt under `limits`: its report, and what it journaled.
    fn attempt(&self, limits: Limits, stderr: &[u8]) -> (AttemptReport, Vec<Value>) {
        self.attempt_then(limits, stderr, "")
    }

    fn attempt_then(
        &self,
        limits: Limits,
        stderr: &[u8],
        then: &str,
    ) -> (AttemptReport, Vec<Value>) {
        let file = |name: String, bytes: &[u8]| {
            let path = self.dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            format!("cat '{}'", path.display())
        };
        let mut script = format!("{}; {} >&2", accepting(), file("stderr".into(), stderr));
        for (at, bytes) in self.played.iter().enumerate() {
            script = format!("{script}; {}", file(format!("{at}"), bytes));
        }
        script.push_str(then);
        let driver = command(&script);
        let mut process = DriverProcess::spawn_limited(
            &driver,
            Path::new("."),
            None,
            &SpawnEnv::Inherit,
            Host::REAL,
            limits,
        )
        .unwrap();
        process.bounds.grace = Duration::from_millis(200);
        let mut journaled = Vec::new();
        let report = process.run_attempt("test", "effect", "attempt", "seat", json!({}), |data| {
            journaled.push(data.clone());
        });
        (report, journaled)
    }
}

/// `body` on the wire with a fixed message id, so its bytes are known.
fn line(body: Body) -> Vec<u8> {
    let message = Message {
        proto: PROTO.into(),
        msg_id: "m".into(),
        body,
    };
    let mut line = serde_json::to_vec(&message).unwrap();
    line.push(b'\n');
    line
}

fn checkpoint(pad: usize) -> Vec<u8> {
    line(Body::Checkpoint {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        data: json!({ "pad": "x".repeat(pad) }),
    })
}

fn result(pad: usize) -> Vec<u8> {
    line(Body::Result {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        status: ResultStatus::Succeeded,
        result: Some(json!({ "pad": "x".repeat(pad) })),
        error: None,
    })
}

fn the_result() -> Vec<u8> {
    format!("{}\n", succeeded()).into_bytes()
}

fn complete() -> AttemptOutcome {
    AttemptOutcome::Succeeded {
        result: json!({"result": "complete"}),
    }
}

fn framing(frame_bytes: usize) -> Limits {
    Limits {
        frame_bytes,
        ..Limits::DEFAULT
    }
}

fn indeterminate(reason: &str) -> AttemptOutcome {
    AttemptOutcome::Indeterminate {
        reason: format!("{reason}{PAST}"),
    }
}

fn frame_exceeded(limit: usize, head: &str) -> AttemptOutcome {
    indeterminate(&format!(
        "driver stdout line exceeded the frame limit of {limit} bytes; it began \"{head}\""
    ))
}

/// The escaped head every fixed-id checkpoint line begins with.
const CHECKPOINT_HEAD: &str =
    r#"{\"proto\":\"forge-driver/v1\",\"msg_id\":\"m\",\"type\":\"checkpoint\",\"eff"#;

/// A line of exactly the limit is read; one byte past it ends the
/// attempt indeterminate, quoting its head and nothing else of it, and
/// neither journals nor keeps the checkpoint it would have been.
#[test]
fn a_line_one_byte_past_the_frame_limit_ends_the_attempt_indeterminate() {
    let frame = checkpoint(300);
    let limit = frame.len() - 1;
    let driver = Driver::playing(vec![frame, the_result()]);

    let (report, journaled) = driver.attempt(framing(limit), b"");
    assert_eq!(report.outcome, complete());
    assert_eq!(report.checkpoints, [json!({ "pad": "x".repeat(300) })]);
    assert_eq!(journaled, report.checkpoints);

    let (report, journaled) = driver.attempt(framing(limit - 1), b"");
    assert_eq!(report.outcome, frame_exceeded(limit - 1, CHECKPOINT_HEAD));
    assert_eq!(report.settled_outcome(), report.outcome);
    assert!(report.accepted);
    assert!(report.checkpoints.is_empty());
    assert!(journaled.is_empty());
}

/// A stream that never sends a newline is refused at the limit, and the
/// driver still writing it is ended within the grace: memory stays
/// bounded, and the attempt ends.
#[test]
fn an_endless_line_with_no_newline_is_refused_and_its_driver_ended() {
    let started = Instant::now();
    let (report, _) =
        Driver::playing(Vec::new()).attempt_then(framing(512), b"", "; exec cat /dev/zero");
    assert_eq!(report.outcome, frame_exceeded(512, &"\\x00".repeat(64)));
    assert_eq!(report.cleanup, Cleanup::Settled);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
}

/// A line that is not UTF-8 is a driver defect within the limit, as it
/// always was, and past the limit is refused before it is decoded.
#[test]
fn invalid_utf8_is_a_failure_within_the_limit_and_refused_past_it() {
    let (report, _) = Driver::playing(vec![b"\xff\n".to_vec()]).attempt(framing(512), b"");
    assert_eq!(
        report.outcome,
        AttemptOutcome::Failed {
            error: "driver stdout read failed: stream did not contain valid UTF-8".into()
        }
    );

    let (report, _) = Driver::playing(vec![[0xff; 513].to_vec()]).attempt(framing(512), b"");
    assert_eq!(report.outcome, frame_exceeded(512, &"\\xff".repeat(64)));
}

/// A result is one line, held by the frame limit: at the limit it is
/// the outcome, one byte past it the attempt is indeterminate. What a
/// driver writes after its result is not protocol, whatever its length.
#[test]
fn a_result_past_the_frame_limit_ends_the_attempt_indeterminate() {
    let frame = result(400);
    let limit = frame.len() - 1;
    let reached = AttemptOutcome::Succeeded {
        result: json!({ "pad": "x".repeat(400) }),
    };
    let driver = Driver::playing(vec![frame, vec![b'x'; 4096]]);

    let (report, _) = driver.attempt(framing(limit), b"");
    assert_eq!(report.outcome, reached);
    assert_eq!(report.cleanup, Cleanup::Settled);

    let (report, _) = driver.attempt(framing(limit - 1), b"");
    let head = r#"{\"proto\":\"forge-driver/v1\",\"msg_id\":\"m\",\"type\":\"result\",\"effect_"#;
    assert_eq!(report.outcome, frame_exceeded(limit - 1, head));
}

/// A stderr flood is drained, so the driver finishes, and is kept as its
/// head and tail around the exact count of what was dropped; stderr
/// within the limit is kept whole.
#[test]
fn a_stderr_flood_keeps_its_head_and_tail_and_counts_the_rest() {
    let limits = Limits {
        stderr_bytes: 8,
        ..Limits::DEFAULT
    };
    let driver = Driver::playing(vec![the_result()]);

    let (report, _) = driver.attempt(limits, b"headtail");
    assert_eq!(report.outcome, complete());
    assert_eq!(report.stderr, "headtail");

    let flood = [b"head".as_slice(), &[b'.'; 1024 * 1024], b"tail"].concat();
    let (report, _) = driver.attempt(limits, &flood);
    assert_eq!(report.outcome, complete());
    assert_eq!(
        report.stderr,
        "head\n[1048576 bytes of driver stderr dropped]\ntail"
    );
}

/// Checkpoints past the count limit, or past the byte limit, end the
/// attempt indeterminate; the ones within are journaled and kept, and
/// the one past is neither.
#[test]
fn a_checkpoint_flood_past_either_limit_ends_the_attempt_indeterminate() {
    let frame = checkpoint(8);
    let bytes = frame.len();
    let driver = Driver::playing(vec![frame.clone(), frame.clone(), frame, the_result()]);
    let kept = |count: usize| vec![json!({ "pad": "x".repeat(8) }); count];
    let counting = |checkpoints| Limits {
        checkpoints,
        ..Limits::DEFAULT
    };
    let weighing = |checkpoint_bytes| Limits {
        checkpoint_bytes,
        ..Limits::DEFAULT
    };

    for limits in [counting(3), weighing(3 * bytes)] {
        let (report, journaled) = driver.attempt(limits, b"");
        assert_eq!(report.outcome, complete());
        assert_eq!((report.checkpoints, journaled), (kept(3), kept(3)));
    }

    let (report, journaled) = driver.attempt(counting(2), b"");
    let reason = "driver checkpoints exceeded the limit of 2 checkpoints";
    assert_eq!(report.outcome, indeterminate(reason));
    assert_eq!((report.checkpoints, journaled), (kept(2), kept(2)));

    let (report, journaled) = driver.attempt(weighing(3 * bytes - 1), b"");
    let reason = format!(
        "driver checkpoints exceeded the limit of {} bytes: 2 checkpoints held {} bytes \
         when one of {bytes} bytes arrived",
        3 * bytes - 1,
        2 * bytes
    );
    assert_eq!(report.outcome, indeterminate(&reason));
    assert_eq!((report.checkpoints, journaled), (kept(2), kept(2)));
}

/// A read the kernel interrupts is read again, on stdout and on stderr;
/// a stdout read that fails is handed over, and the pipe drained to its
/// EOF; a stderr read that fails keeps what was read before it.
#[test]
fn interrupted_reads_are_retried_and_a_failed_stderr_read_keeps_what_came_before() {
    struct Reads(Vec<std::io::Result<&'static [u8]>>);
    impl Read for Reads {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let bytes = self.0.remove(0)?;
            buf[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        }
    }
    let interrupted = || Err(std::io::Error::from(ErrorKind::Interrupted));
    let failed = || Err(std::io::Error::other("injected read fault"));

    let mut stdout = std::io::BufReader::new(Reads(vec![interrupted(), Ok(b"x\n")]));
    assert_eq!(
        read_frame(&mut stdout, 2).unwrap(),
        Frame::Line(b"x\n".to_vec())
    );
    let stdout = crate::process::read_stdout(Reads(vec![failed(), Ok(b"unread"), Ok(b"")]), 2);
    let Ok(crate::process::Stdout::Failed(error)) = stdout.recv() else {
        panic!("the failed read was not handed over")
    };
    assert_eq!(error.to_string(), "injected read fault");
    assert!(matches!(stdout.recv(), Ok(crate::process::Stdout::Eof)));

    let stderr = Reads(vec![Ok(b"kept"), interrupted(), Ok(b"!"), failed()]);
    assert_eq!(retained_stderr(stderr, 8), "kept!");
}
