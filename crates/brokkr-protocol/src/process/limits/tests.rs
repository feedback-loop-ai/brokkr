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
/// file of its own, in order, and stays as long as the last one does. It
/// plays nothing before the engine's `start` reaches it: `accepting`
/// reads only the `hello`, and a driver that exited before `start` was
/// written would fail the attempt on a broken pipe instead.
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
        let process = self.spawned(limits, stderr, then, None);
        let mut journaled = Vec::new();
        let report = process.run_attempt("test", "effect", "attempt", "seat", json!({}), |data| {
            journaled.push(data.clone());
        });
        (report, journaled)
    }

    /// The driver under `limits` and `deadline`: it writes `stderr`, plays,
    /// and then runs `then`.
    fn spawned(
        &self,
        limits: Limits,
        stderr: &[u8],
        then: &str,
        deadline: Option<Duration>,
    ) -> DriverProcess {
        let file = |name: String, bytes: &[u8]| {
            let path = self.dir.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            format!("cat '{}'", path.display())
        };
        let stderr = file("stderr".into(), stderr);
        let mut script = format!("{}; read -r start; {stderr} >&2", accepting());
        for (at, bytes) in self.played.iter().enumerate() {
            script = format!("{script}; {}", file(format!("{at}"), bytes));
        }
        script.push_str(then);
        let driver = command(&script);
        let mut process = DriverProcess::spawn_limited(
            &driver,
            Path::new("."),
            deadline,
            &SpawnEnv::Inherit,
            Host::REAL,
            limits,
        )
        .unwrap();
        process.bounds.grace = Duration::from_millis(200);
        process
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

/// The refusal of a line past `limit`, whose first `limit + 1` bytes
/// begin with a `first` byte and hash to `sha256`.
fn frame_exceeded(limit: usize, first: &str, sha256: &str) -> AttemptOutcome {
    indeterminate(&format!(
        "driver stdout line exceeded the frame limit of {limit} bytes; its first {} bytes \
         (first byte: {first}; sha256: {sha256})",
        limit + 1
    ))
}

/// A line of exactly the limit is read; one byte past it ends the
/// attempt indeterminate, naming the line's first bytes by length, class
/// and digest and nothing of their content, and neither journals nor
/// keeps the checkpoint it would have been.
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
    assert_eq!(
        report.outcome,
        frame_exceeded(limit - 1, "punctuation", "edca62a8cb5d7a94")
    );
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
    assert_eq!(
        report.outcome,
        frame_exceeded(512, "control", "8ae7498b01f40e9d")
    );
    assert_eq!(report.cleanup, Cleanup::Settled);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
}

/// A refused line outranks the deadline. The engine's journal lags, so
/// the checkpoint before the line returns only after the deadline and its
/// drain, when the attempt loop's next read gives up on stdout; the
/// attempt still ends on the frame limit's refusal, indeterminate, never
/// as the deadline's failure that a retry may safely repeat.
#[test]
fn a_refused_line_outranks_the_deadline_while_a_checkpoint_is_journaled() {
    let deadline = Duration::from_secs(1);
    let played = [checkpoint(8), vec![b'x'; 513]].concat();
    let driver = Driver::playing(vec![played]);
    let mut process = driver.spawned(framing(512), b"", "; read -r never", Some(deadline));
    process.bounds.drain = Duration::from_millis(100);
    let give_up = process.started + deadline + process.bounds.drain;
    let report = process.run_attempt("test", "effect", "attempt", "seat", json!({}), |_| {
        std::thread::sleep(
            give_up.saturating_duration_since(Instant::now()) + Duration::from_millis(1),
        );
    });
    assert_eq!(
        report.outcome,
        frame_exceeded(512, "letter", "35ade0090e64e74d")
    );
    assert_eq!(report.settled_outcome(), report.outcome);
    assert!(report.deadline_killed);
    assert_eq!(report.cleanup, Cleanup::Settled);
}

/// Zero admits nothing (decision 0079): a zero frame limit refuses the
/// driver's first line, a zero stderr limit keeps none of its first byte,
/// and a zero checkpoint count or byte limit refuses its first checkpoint.
#[test]
fn a_zero_limit_admits_nothing() {
    let frame = checkpoint(8);
    let bytes = frame.len();
    let driver = Driver::playing(vec![frame, the_result()]);
    let zero = [
        (framing(0), b"".as_slice()),
        (
            Limits {
                stderr_bytes: 0,
                ..Limits::DEFAULT
            },
            b"e",
        ),
        (
            Limits {
                checkpoints: 0,
                ..Limits::DEFAULT
            },
            b"",
        ),
        (
            Limits {
                checkpoint_bytes: 0,
                ..Limits::DEFAULT
            },
            b"",
        ),
    ];
    let ended = zero.map(|(limits, stderr)| {
        let (report, journaled) = driver.attempt(limits, stderr);
        (report.outcome, report.stderr, journaled.len())
    });
    let first = |limit: &str| {
        indeterminate(&format!(
            "driver checkpoints exceeded the limit of 0 {limit}: 0 checkpoints held 0 bytes \
             when one of {bytes} bytes arrived"
        ))
    };
    assert_eq!(
        ended,
        [
            (
                frame_exceeded(0, "punctuation", "021fb596db81e6d0"),
                String::new(),
                0
            ),
            (
                complete(),
                "\n[1 bytes of driver stderr dropped]\n".into(),
                1
            ),
            (first("checkpoints"), String::new(), 0),
            (first("bytes"), String::new(), 0),
        ]
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
    assert_eq!(
        report.outcome,
        frame_exceeded(512, "non-ASCII", "ea032debaa72c17d")
    );
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
    assert_eq!(
        report.outcome,
        frame_exceeded(limit - 1, "punctuation", "d897781e5826b22c")
    );
}

/// A synthetic credential a driver printed by accident.
const TOKEN: &str = "sk-test-NOTASECRET-0000";

/// A refused line is named by its length, its first byte's class and a
/// digest, never by its content: a credential at the head of a line past
/// the limit, in a line that is not JSON, in one whose JSON error quotes
/// it, and in a line both past the limit and not JSON, appears nowhere
/// in the outcome or in what the attempt journaled.
#[test]
fn a_refused_line_discloses_none_of_its_content() {
    let pad = "x".repeat(600);
    let proto = r#"{"proto":"forge-driver/v1","msg_id":"#;
    let quoted = format!(r#"{proto}"m","type":"{TOKEN}"}}"#);
    let quoting = serde_json::from_str::<Message>(&quoted).unwrap_err();
    assert!(quoting.to_string().contains(TOKEN), "{quoting}");
    let unreadable = |error: &str| AttemptOutcome::Failed {
        error: format!("unreadable driver message: {error}"),
    };
    let played = [
        format!(r#"{proto}"{TOKEN}","pad":"{pad}"}}"#),
        format!("{TOKEN} is not json"),
        quoted,
        format!("{TOKEN} {pad}"),
    ];
    let (mut outcomes, mut leaked) = (Vec::new(), Vec::new());
    for (case, played) in played.iter().enumerate() {
        let (report, journaled) =
            Driver::playing(vec![format!("{played}\n").into_bytes()]).attempt(framing(512), b"");
        let kept = format!("{:?} {journaled:?}", report.outcome);
        if kept.contains(TOKEN) {
            leaked.push(case);
        }
        outcomes.push(report.outcome);
    }
    assert_eq!(leaked, Vec::<usize>::new());
    assert_eq!(
        outcomes,
        [
            frame_exceeded(512, "punctuation", "dff6e0410bc48663"),
            unreadable("a JSON syntax error at line 1 column 1 of 36 bytes (first byte: letter; sha256: cb9a3d9c47d85a38)"),
            unreadable("a JSON data error at line 1 column 73 of 74 bytes (first byte: punctuation; sha256: b6d931a90c06f3d0)"),
            frame_exceeded(512, "letter", "aecc524cb360a311"),
        ]
    );
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

    // Either refusal names its evidence as counts, never as a line.
    let past = |limit: String| {
        indeterminate(&format!(
            "driver checkpoints exceeded the limit of {limit}: 2 checkpoints held {} bytes \
             when one of {bytes} bytes arrived",
            2 * bytes
        ))
    };
    let (report, journaled) = driver.attempt(counting(2), b"");
    assert_eq!(report.outcome, past("2 checkpoints".into()));
    assert_eq!((report.checkpoints, journaled), (kept(2), kept(2)));

    let (report, journaled) = driver.attempt(weighing(3 * bytes - 1), b"");
    assert_eq!(report.outcome, past(format!("{} bytes", 3 * bytes - 1)));
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
    let (stdout, _) = crate::process::read_stdout(Reads(vec![failed(), Ok(b"unread"), Ok(b"")]), 2);
    let Ok(crate::process::Stdout::Failed(error)) = stdout.recv() else {
        panic!("the failed read was not handed over")
    };
    assert_eq!(error.to_string(), "injected read fault");
    assert!(matches!(stdout.recv(), Ok(crate::process::Stdout::Eof)));

    let stderr = Reads(vec![Ok(b"kept"), interrupted(), Ok(b"!"), failed()]);
    assert_eq!(retained_stderr(stderr, 8), "kept!");
}
