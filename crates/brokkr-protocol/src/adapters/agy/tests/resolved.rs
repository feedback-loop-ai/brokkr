//! Round four of #1200: the child runs in the resolved working directory
//! its `PWD` names, a stdout that cannot be read to its end fails the turn,
//! a turn the driver stopped reads no exit of agy's, and the first result's
//! status is named first.

use super::judged::{driven, driven_with, stream};
use super::*;
use crate::adapters::stream_turn::{json_rows, read_lines, turn_read, Line, Read as Reading};
use fold::Seen;

/// The recorded tool-less turn's init, its finished response step, and its
/// `SUCCESS` result.
pub(super) fn turn_rows() -> (&'static str, &'static str, &'static str) {
    let lines: Vec<&'static str> = TOOL_LESS.lines().collect();
    (lines[0], lines[1], lines[lines.len() - 1])
}

/// What was seen of a turn whose first result had `status`.
fn seen(status: &str) -> Seen {
    Seen {
        status: Some(status.into()),
        timed_out: false,
    }
}

/// A working directory given through a link, from a driver whose own `PWD`
/// names that link: the child runs in the resolved path and its `PWD`
/// names it, so agy's own walk is the one the check read, and the link's
/// own parent, which carries agy's configuration, refuses nothing (L2).
#[test]
fn a_working_directory_given_through_a_link_is_the_resolved_pwd_of_agy() {
    let mut world = World::new();
    let resolved = sub(&world.seat().canonicalize().unwrap().join("real"));
    let links = world.path("links");
    std::fs::create_dir(&links).unwrap();
    let link = links.join("link");
    std::os::unix::fs::symlink(&resolved, &link).unwrap();
    plant(&links, ".agents/mcp_config.json");
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let mut input = world.input(None);
    input["workdir"] = json!(link);
    // As a shell that entered the link would leave it.
    world.env.set("PWD", &link);
    let messages = world.run(&bin, &["--model", FLASH], input, None);
    assert_eq!(
        ended(&messages),
        Some((crate::ResultStatus::Succeeded, None, None))
    );
    assert_eq!(Path::new(&world.recorded("pwd")), resolved);
    assert_eq!(Path::new(world.recorded("cwd").trim_end()), resolved);
}

/// A reader that gives its bytes and then fails.
struct Failing(std::io::Cursor<Vec<u8>>);

impl std::io::Read for Failing {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match std::io::Read::read(&mut self.0, buf)? {
            0 => Err(std::io::Error::other("the pipe failed")),
            read => Ok(read),
        }
    }
}

/// A pinned `SUCCESS` turn's lines, and then a failed read.
fn failing() -> std::io::BufReader<Failing> {
    let (init, step, result) = turn_rows();
    let bytes = stream(&[init, step, result]) + "\n";
    std::io::BufReader::new(Failing(std::io::Cursor::new(bytes.into_bytes())))
}

/// The turn a spawned fake `agy`, exiting 0 with nothing on stderr, ends
/// with when its stdout is read from `reader` by the function that builds
/// the production [`Turn`] ([`turn_read`]), concluded with the seat's
/// result file delivered: whether the turn was unread, and its answer.
fn concluded_on(reader: impl std::io::BufRead) -> (bool, Result<Option<Value>, String>) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    let result = dir.join("result.json");
    std::fs::write(&result, "{\"result\":\"complete\"}").unwrap();
    let command = [fake_agy(dir, "", "", false, 0)
        .to_string_lossy()
        .into_owned()];
    let fed = spawn_fed_in(&command, "", (dir.to_str().unwrap(), &[]), &[]).unwrap();
    let (mut fold, mut record) = (AgyFold::new(FLASH, &[]), Map::new());
    let turn = turn_read(fed, reader, |line| {
        match fold.read(line, &mut record, &mut |_| {}) {
            Ok(()) => Line::Read,
            Err(refused) => Line::Stopped(refused),
        }
    });
    let turn = turn.unwrap();
    let unread = turn.unread;
    let input = json!({ "result_path": result });
    let concluded = concluded(fold, turn, record, &input, &[]);
    let answer = concluded.map(|invocation| invocation.answer);
    (unread, answer.map_err(|ended| ended.to_string()))
}

/// A pinned `SUCCESS` and then a read that fails: the production turn is
/// marked unread, agy's verdict is the fault, and a turn so marked sends no
/// result behind a delivered result file; read to its end, the same turn
/// answers. Qoder reads the same rows over the same reader and ends there
/// (L3, M5).
#[test]
fn a_stdout_that_fails_before_its_end_fails_the_turn_and_qoder_ends_its_reading() {
    let unread = Reading {
        stopped: None,
        unread: true,
    };
    let failure = Failed {
        failure: Failure::Unreadable,
        seen: seen("SUCCESS"),
    };
    assert_eq!(
        failure.to_string(),
        "agy's result had status SUCCESS; agy's stdout could not be read to its end, so the turn \
         gave no answer (#1200)"
    );
    assert_eq!(concluded_on(failing()), (true, Err(failure.to_string())));
    let (init, step, result) = turn_rows();
    let whole = stream(&[init, step, result]) + "\n";
    assert_eq!(
        concluded_on(std::io::Cursor::new(whole)),
        (false, Ok(Some(json!("READY\n"))))
    );
    // A fault seen earlier stands.
    let mut faulted = read(b"\xff", FLASH).fold;
    faulted.unreadable(true);
    assert_eq!(
        faulted.verdict(0, false),
        Verdict::Failed(Failed {
            failure: Failure::NotText,
            seen: Seen::default(),
        })
    );
    let mut rows = Vec::new();
    let qoder = read_lines(
        failing(),
        json_rows(|row| {
            rows.push(row);
            Ok(())
        }),
    );
    assert_eq!(qoder, unread);
    let (init, step, result) = turn_rows();
    let expected: Vec<Value> = [init, step, result]
        .iter()
        .map(|row| serde_json::from_str(row).unwrap())
        .collect();
    assert_eq!(rows, expected);
}

/// A substituted init behind which agy exits 3 is the substitution
/// refusal, with no word of a timeout: the driver stopped the turn, so
/// what agy exits with is the driver's doing (L4).
#[test]
fn a_substituted_init_behind_which_agy_exits_3_says_nothing_of_a_timeout() {
    let neon = r#"{"event":"init","init":{"model":"gemini-4-neon"}}"#;
    let words =
        format!("model_substituted: the seat pinned \"{FLASH}\" and agy served \"gemini-4-neon\"");
    let substituted = read(neon, FLASH);
    match substituted.fold.verdict(3, false) {
        Verdict::Refused(refused) => assert_eq!(refused.words(), words),
        other => panic!("{other:?}"),
    }
    let turn = driven((neon, PRINT_TIMEOUT_STDERR, 3), FLASH, &[]);
    let refusal = turn.turn.unwrap().refusal;
    assert_eq!(refusal.map(|r| r.words().to_string()), Some(words));
}

/// The status named is the first typed result's, a fault before it or not:
/// a line that is not UTF-8 and then `WAITING` fails as not text naming
/// `WAITING`, which only a row behind the line can give, and an unread row,
/// then `WAITING`, then `ERROR` names `WAITING` (L5, L7).
#[test]
fn the_first_typed_result_names_its_status_behind_a_fault() {
    let (init, step, _) = turn_rows();
    let waiting = r#"{"event":"result","result":{"status":"WAITING"}}"#;
    let error = r#"{"event":"result","result":{"status":"ERROR"}}"#;
    let not_text = [init.as_bytes(), b"\xff", waiting.as_bytes()].join(&b'\n');
    let named = Failed {
        failure: Failure::NotText,
        seen: seen("WAITING"),
    };
    assert_eq!(
        read(&not_text, FLASH).fold.verdict(0, false),
        Verdict::Failed(named.clone())
    );
    assert_eq!(
        named.to_string(),
        "agy's result had status WAITING; agy wrote a line that is not UTF-8, so the turn gave no \
         answer (#1200)"
    );
    let unread = r#"{"event":"step_update","step_update":{"state":"DONE"}}"#;
    let rows = stream(&[init, step, unread, waiting, error]);
    assert_eq!(
        read(rows, FLASH).fold.verdict(3, false),
        Verdict::Failed(Failed {
            failure: Failure::Unread(fold::Event::StepUpdate),
            seen: Seen {
                status: Some("WAITING".into()),
                timed_out: true,
            },
        })
    );
    let turn = driven_with((not_text.as_slice(), "", 0), true, FLASH, &[]).turn;
    assert_eq!(turn.map(drop), Err(named.to_string()));
}

/// A status of 40 characters, behind a pin of 200, is named whole by every
/// refusal that names a status, each builder among them, and the
/// signed-out refusal names `WAITING_FOR_AUTHENTICATION` whole: the status
/// stands first (L5, M5).
#[test]
fn every_refusal_that_names_a_status_names_it_whole_before_its_words() {
    let status = "S".repeat(40);
    let pin = "p".repeat(200);
    let result = |status: &str, extra: &str| {
        format!(r#"{{"event":"result","result":{{"status":"{status}"{extra}}}}}"#)
    };
    let signed = r#","error":"authentication failed or timed out""#;
    let unnamed = r#"{"event":"init","init":{}}"#;
    let success = r#"{"event":"result","result":{"status":"SUCCESS","response":"x"}}"#;
    let authenticating = "WAITING_FOR_AUTHENTICATION";
    let cases = [
        (result(&status, signed), 0, "not_logged_in", status.as_str()),
        (result(&status, ""), 3, "print_timeout", status.as_str()),
        (result(&status, ""), 1, "launch_refused", status.as_str()),
        (
            stream(&[unnamed, success]),
            0,
            "model_unconfirmed",
            "SUCCESS",
        ),
        (
            result(authenticating, signed),
            0,
            "not_logged_in",
            authenticating,
        ),
    ];
    for (rows, exit, token, named) in cases {
        let words = match read(&rows, &pin).fold.verdict(exit, false) {
            Verdict::Refused(refused) => refused.words().to_string(),
            other => panic!("{token}: {other:?}"),
        };
        let lead = format!("{token}: agy's result had status {named}; ");
        assert!(words.starts_with(&lead), "{words}");
    }
    // Each builder that takes what the result showed, called with the long
    // status and pin, the unconfirmed model's among them, which no stream
    // reaches with a status other than SUCCESS (M5).
    let long = seen(&status);
    let built = [
        fold::unconfirmed(&pin, &long),
        fold::signed_out(&long),
        fold::launch_refused(&pin, &long),
        fold::print_timeout(&long),
    ];
    let tokens = [
        "model_unconfirmed",
        "not_logged_in",
        "launch_refused",
        "print_timeout",
    ];
    for (refused, token) in built.iter().zip(tokens) {
        let lead = format!("{token}: agy's result had status {status}; ");
        assert!(refused.words().starts_with(&lead), "{}", refused.words());
    }
}
