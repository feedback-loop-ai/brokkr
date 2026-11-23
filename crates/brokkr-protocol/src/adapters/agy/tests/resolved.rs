//! Round four of #1200: the child runs in the resolved working directory
//! its `PWD` names, a stdout that cannot be read to its end fails the turn,
//! a turn the driver stopped reads no exit of agy's, and the first result's
//! status is named first.

use super::judged::{driven, driven_with, stream};
use super::*;
use crate::adapters::stream_turn::{streamed_lines, turn_read, Line};
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
    let resolved = sub(&world.seat().join("real"));
    let links = world.path("links");
    std::fs::create_dir(&links).unwrap();
    let link = links.join("link");
    std::os::unix::fs::symlink(&resolved, &link).unwrap();
    plant(&links, ".agents/mcp_config.json");
    // As a shell that entered the link would leave it.
    world.env.set("PWD", &link);
    runs_in(&mut world, json!(link), &resolved);
}

/// What a rerun of [`an_empty_working_directory_is_the_drivers_own_resolved`]
/// in a working directory of its own reads: that it is one.
const OWN_CWD: &str = "BROKKR_TEST_AGY_OWN_CWD";

/// [`an_empty_working_directory_is_the_drivers_own_resolved`] by its full
/// name, which the rerun is filtered to.
const EMPTY_WORKDIR: &str =
    "adapters::agy::tests::resolved::an_empty_working_directory_is_the_drivers_own_resolved";

/// A seat that gives an empty working directory runs agy in the driver's
/// own, resolved, and its `PWD` names that path. Run as itself, this test
/// reruns its binary, filtered to itself, in a fresh temporary directory,
/// so no configuration of agy's around the checkout decides the launch;
/// so rerun, it is the proof.
#[test]
fn an_empty_working_directory_is_the_drivers_own_resolved() {
    if std::env::var_os(OWN_CWD).is_some() {
        let mut world = World::new();
        let own = std::env::current_dir().unwrap().canonicalize().unwrap();
        runs_in(&mut world, json!(""), &own);
        return;
    }
    let cwd = tempfile::tempdir().unwrap();
    let rerun = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", EMPTY_WORKDIR, "--test-threads=1"])
        .current_dir(cwd.path())
        .env(OWN_CWD, "1")
        .output()
        .unwrap();
    let ran = String::from_utf8_lossy(&rerun.stdout);
    assert!(rerun.status.success(), "{ran}");
    assert!(ran.contains("1 passed"), "{ran}");
}

/// A tool-less turn given `workdir` succeeds, and the child ran in
/// `resolved` with its `PWD` naming it.
fn runs_in(world: &mut World, workdir: Value, resolved: &Path) {
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let mut input = world.input(None);
    input["workdir"] = workdir;
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

/// A spawned child under `dir` that reads nothing and simply exits 0, with
/// nothing on stderr.
fn exits(
    dir: &Path,
) -> (
    crate::process::spawned::Spawned,
    crate::adapters::stderr::StderrCapture,
) {
    let exits = super::super::super::tests::executable(dir, "exits", "#!/bin/sh\nexit 0\n");
    let command = [exits.to_string_lossy().into_owned()];
    spawn_fed_in(&command, "", (dir.to_str().unwrap(), &[]), &[]).unwrap()
}

/// The turn a spawned child that simply exits ends with when its stdout is
/// read from `reader` by the function that builds the production [`Turn`]
/// ([`turn_read`]), concluded with the seat's result file delivered:
/// whether the turn was unread, and its answer.
fn concluded_on(reader: impl std::io::BufRead) -> (bool, Result<Option<Value>, Ended>) {
    concluded_after(reader, |_| {})
}

/// [`concluded_on`], `before` handed the spawned child before its stdout is
/// read.
fn concluded_after(
    reader: impl std::io::BufRead,
    before: impl FnOnce(&crate::process::spawned::Spawned),
) -> (bool, Result<Option<Value>, Ended>) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    let result = dir.join("result.json");
    std::fs::write(&result, "{\"result\":\"complete\"}").unwrap();
    let fed = exits(dir);
    before(&fed.0);
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
    (unread, concluded.map(|invocation| invocation.answer))
}

/// A turn stopped at a substituted init after agy had exited 0 on its own,
/// its result delivered, is no refusal to start: the stop's reported code
/// is not agy's exit, so the turn ends late, in the refusal's words, with
/// no result (#1178 H3, checked for agy).
#[test]
fn a_stop_after_agy_exited_clean_with_its_result_delivered_ends_late() {
    use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
    let neon = r#"{"event":"init","init":{"model":"gemini-4-neon"}}"#;
    let reader = std::io::Cursor::new(format!("{neon}\n"));
    let (_, ended) = concluded_after(reader, |child| {
        // Until the child has exited, observed and left unreaped, so the
        // stop meets an exit of its own.
        let pid = Pid::from_raw(i32::try_from(child.id()).unwrap()).unwrap();
        let exited = WaitIdOptions::EXITED | WaitIdOptions::NOWAIT;
        assert!(waitid(WaitId::Pid(pid), exited).unwrap().is_some());
    });
    let words =
        format!("model_substituted: the seat pinned \"{FLASH}\" and agy served \"gemini-4-neon\"");
    match ended {
        Err(Ended::Late(late)) => assert_eq!(late.words(), words),
        other => panic!("{other:?}"),
    }
}

/// A turn whose harness cannot be waited on, because it was reaped behind
/// the driver's back, ends in the production read ([`streamed_lines`])
/// with the wait's own cause, and prints in the shared driver's words.
#[test]
fn a_turn_that_does_not_conclude_ends_in_the_shared_drivers_words() {
    let root = tempfile::tempdir().unwrap();
    let fed = exits(root.path());
    let pid = libc::pid_t::try_from(fed.0.id()).unwrap();
    let mut status = 0;
    // SAFETY: `pid` is this test's own child, and `status` outlives the call.
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    let Err(unconcluded) = streamed_lines(fed, |_| Line::Read) else {
        panic!("a reaped harness was waited on")
    };
    assert_eq!(unconcluded.0.raw_os_error(), Some(libc::ECHILD));
    let words = format!("agent CLI did not conclude: {}", unconcluded.0);
    assert_eq!(Ended::from(unconcluded).to_string(), words);
}

/// A pinned `SUCCESS` and then a read that fails: the production turn is
/// marked unread, agy's verdict is the fault, and a turn so marked sends no
/// result behind a delivered result file; read to its end, the same turn
/// answers (L3, M5).
#[test]
fn a_stdout_that_fails_before_its_end_fails_the_turn() {
    let failure = Failed {
        failure: Failure::Unreadable,
        seen: seen("SUCCESS"),
    };
    assert_eq!(
        failure.to_string(),
        "agy's result had status SUCCESS; agy's stdout could not be read to its end, so the turn \
         gave no answer (#1200)"
    );
    let (unread_turn, ended) = concluded_on(failing());
    assert!(unread_turn);
    let Err(Ended::Failed(failed)) = ended else {
        panic!("{ended:?}")
    };
    assert_eq!(failed, failure);
    let (init, step, result) = turn_rows();
    let whole = stream(&[init, step, result]) + "\n";
    let (unread_turn, answer) = concluded_on(std::io::Cursor::new(whole));
    assert!(!unread_turn);
    assert_eq!(answer.unwrap(), Some(json!("READY\n")));
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
        fold::unconfirmed(&pin, &[], &long),
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

/// The refusal a fold over `pin` and `bindings` ends `rows` at, each read
/// in turn and then the verdict of an exit of 0: the live path every
/// `model_unconfirmed` and `model_substituted` refusal is built on.
fn refused_after(pin: &str, bindings: &[secret::BoundSecret], rows: &[&str]) -> PreSessionRefusal {
    let mut fold = AgyFold::new(pin, bindings);
    for row in rows {
        if let Err(refused) = fold.read(row.as_bytes(), &mut Map::new(), &mut |_| {}) {
            return refused;
        }
    }
    match fold.verdict(0, false) {
        Verdict::Refused(refused) => refused,
        other => panic!("{other:?}"),
    }
}

/// A bound value of 4 bytes at the tail of a pin, the pin as short as lets
/// the bound of 160 characters on a refusal's words fall one character
/// inside the value, on each live path that refuses the pin: 111 behind a
/// `SUCCESS` result no init confirmed (50 characters lead the pin there),
/// and 144 at a step before any init, at an exit of 0 with no row at all,
/// or at an init naming another model (17 lead it). No plain pin, at most
/// 80 characters, reaches the bound behind a lead of 50 or less, so each
/// pin here is one the launch refuses as not plain: the fold's order is
/// proved apart from that rule, and the row below proves it on a pin the
/// launch admits. The pin is masked before the bound reads it, so neither
/// the words nor the shared mask after them hold a byte of the value (Z1).
#[test]
fn a_bound_value_at_the_tail_of_a_long_pin_reaches_no_refusal_in_any_form() {
    let value = "Zq7W";
    let bindings = [crate::adapters::tests::binding("AT", value)];
    let shown =
        |refused: &PreSessionRefusal| secret::mask_text(refused.words().as_bytes(), &bindings);
    let (init, step, result) = turn_rows();
    let init = init.replace("gemini-3.8-flash-low", "gemini-x");
    let (unconfirmed, lead) = ("model_unconfirmed", "agy's result had status SUCCESS; ");
    let cases: [(usize, &[&str], &str, &str); 4] = [
        (107, &[result], unconfirmed, lead),
        (140, &[step], unconfirmed, ""),
        (140, &[], unconfirmed, ""),
        (140, &[&init], "model_substituted", ""),
    ];
    for (length, rows, token, status) in cases {
        let (filler, pin) = ("p".repeat(length), format!("{}{value}", "p".repeat(length)));
        assert!(!crate::adapters::stream_turn::plain(&pin));
        let refused = refused_after(&pin, &bindings, rows);
        let words = format!("{token}: {status}the seat pinned \"{filler}[se");
        assert_eq!(shown(&refused), words, "{rows:?}");
        assert!(!shown(&refused).contains(&value[..3]), "{rows:?}");
    }
}

/// The same value at the tail of a plain pin of 78 characters, which the
/// launch admits: behind a `SUCCESS` result no init confirmed, a binding
/// under a name of 31 characters masks the status to the 40 a named word
/// keeps, so 83 characters lead the pin and the bound of 160 falls one
/// character inside the value. Neither the words nor the shared mask after
/// them hold a byte of it (Z1).
#[test]
fn a_bound_value_at_the_tail_of_a_plain_pin_reaches_no_refusal_in_any_form() {
    let (value, name) = ("Zq7W", "S".repeat(31));
    let bindings = [
        crate::adapters::tests::binding(&name, "SUCCESS"),
        crate::adapters::tests::binding("AT", value),
    ];
    let filler = "p".repeat(74);
    let pin = format!("{filler}{value}");
    assert!(crate::adapters::stream_turn::plain(&pin));
    let refused = refused_after(&pin, &bindings, &[turn_rows().2]);
    let shown = secret::mask_text(refused.words().as_bytes(), &bindings);
    let status = format!("[secret:{name}]");
    assert_eq!(status.len(), 40);
    let words = format!(
        "model_unconfirmed: agy's result had status {status}; the seat pinned \"{filler}[se"
    );
    assert_eq!(shown, words);
    assert!(!shown.contains(&value[..3]), "{shown}");
}
