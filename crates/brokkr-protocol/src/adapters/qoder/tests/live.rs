//! A qoder CLI still running when the driver stops it at a refusing row,
//! and each id the example adapter maps launched on the wire (#1178, J1
//! and J2).

use super::*;

/// How long the live fake runs unkilled before it exits on its own.
const DEADLINE_SECONDS: u32 = 30;

/// The file the live fake writes as it exits on its own at its deadline.
const UNKILLED: &str = "unkilled";

/// The file the live fake's `EXIT` trap writes, with the status, on any
/// exit the shell makes itself, whatever its code. A signal it cannot
/// catch, the driver's kill, ends it with no trap run.
const SELF_EXIT: &str = "self-exit";

/// A fake `qodercli` under `dir` that is still running when its stream is
/// read: it reads the prompt, prints `stderr`, writes its result at
/// `result`, prints `stdout`, and then runs on until it is killed. Unkilled,
/// it exits 0 after [`DEADLINE_SECONDS`], writing [`UNKILLED`] as it does.
/// Any exit of its own, that one or another, writes [`SELF_EXIT`]. So a
/// run it ends, reaped, with neither marker was ended by a signal.
fn live_qoder(
    dir: &std::path::Path,
    stdout: &str,
    stderr: &str,
    result: &std::path::Path,
) -> std::path::PathBuf {
    std::fs::write(dir.join("stdout"), stdout).unwrap();
    std::fs::write(dir.join("stderr"), stderr).unwrap();
    let body = format!(
        "#!/bin/sh\ntrap 'echo $? > '\\''{dir}/{SELF_EXIT}'\\''' EXIT\n\
         cat > '{dir}/prompt'\ncat '{dir}/stderr' >&2\n\
         printf '{{\"result\":\"complete\"}}' > '{result}'\ncat '{dir}/stdout'\n\
         end=$(($(date +%s) + {DEADLINE_SECONDS}))\n\
         while [ \"$(date +%s)\" -lt \"$end\" ]; do sleep 0.1; done\n\
         : > '{dir}/{UNKILLED}'\nexit 0\n",
        dir = dir.display(),
        result = result.display(),
    );
    super::super::super::tests::executable(dir, "qodercli", &body)
}

/// `stdout` and `stderr` run pinned to `pin` on both fakes, each having
/// delivered its result before the refusing row. The fake that exited 0
/// before its stream was read delivered, so its seat ends lost and sends
/// no result (H3); its `exited` marker proves it read the rows after that
/// exit. The live one was stopped at the row: the driver reaped it with
/// neither of its markers written, so no exit of its own ended it, of any
/// code, and a signal did. It delivered nothing and its seat is refused to
/// start. The refusal, as sent.
fn stopped_live(stdout: &str, stderr: &str, pin: &str) -> Vec<crate::Body> {
    let natural = tempfile::tempdir().unwrap();
    let result = natural.path().join("result.json");
    let bin = fake_qoder(natural.path(), stdout, stderr, Some(&result), 0);
    let messages = run(natural.path(), &bin, &["-m", pin]);
    assert!(natural.path().join("exited").exists(), "{messages:?}");
    assert_eq!(result_of(&messages), None, "{messages:?}");
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let bin = live_qoder(dir.path(), stdout, stderr, &result);
    let messages = run(dir.path(), &bin, &["-m", pin]);
    assert!(result.exists(), "the live fake wrote its result");
    let marked = [UNKILLED, SELF_EXIT].map(|marker| dir.path().join(marker).exists());
    assert_eq!(marked, [false, false], "{messages:?}");
    messages
}

/// The words of the one refusal `messages` sent, a failed result's.
fn refused(messages: &[crate::Body]) -> &str {
    let (status, refusal) = result_of(messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    refusal.unwrap().words()
}

/// H5 moves this golden: the fallback line on stderr wins the stop at
/// init, so the words are the reported-on-stderr ones, not init's `Auto`.
/// J1 moves its fake: the CLI is still running at init, and is killed.
#[test]
fn a_substituted_model_is_refused_at_init_and_its_harness_stopped() {
    let messages = stopped_live(UNKNOWN_MODEL, UNKNOWN_MODEL_STDERR, "NoSuchModel");
    assert_eq!(
        refused(&messages),
        "model_substituted: the seat pinned 'NoSuchModel' and Qoder reported on stderr that \
         it fell back to another model"
    );
    let (_, refusal) = result_of(&messages).unwrap();
    assert_eq!(refusal.unwrap().provider(), "qoder");
    let quiet = messages.iter().all(|body| {
        !matches!(
            body,
            crate::Body::Accepted { .. } | crate::Body::Checkpoint { .. }
        )
    });
    assert!(quiet, "{messages:?}");
}

/// J1: a second init row, with the CLI still running, stops it there.
#[test]
fn a_second_init_row_stops_a_cli_still_running() {
    let stdout = format!("{}{}{}", init(), init(), row_of(TOOL_LESS, "result"));
    let messages = stopped_live(&stdout, "", "Fast");
    assert_eq!(refused(&messages), SECOND_INIT);
}

/// J1: a line that is no JSON object, with the CLI still running, stops
/// it there.
#[test]
fn a_bad_line_stops_a_cli_still_running() {
    let stdout = format!("{}not json\n{}", init(), row_of(TOOL_LESS, "result"));
    let messages = stopped_live(&stdout, "", "Fast");
    assert_eq!(refused(&messages), UNMEASURED);
}

/// J2: each id the example adapter maps, pinned, confirmed by an init
/// naming it, answered by an assistant row and ended by a result row, with
/// the result delivered and exit 0, succeeds on the wire with that id
/// served and launched with it.
#[test]
fn every_mapped_id_succeeds_on_the_wire_naming_it_served() {
    for id in mapped_ids() {
        let id = id.as_str();
        let mut init: Value = serde_json::from_str(&init()).unwrap();
        init["model"] = json!(id);
        let (assistant, result) = (row_of(TOOL_LESS, "assistant"), row_of(TOOL_LESS, "result"));
        let stdout = format!("{init}\n{assistant}{result}");
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let bin = fake_qoder(dir.path(), stdout, "", Some(&result), 0);
        let messages = run(dir.path(), &bin, &["-m", id]);
        let (status, refusal) = result_of(&messages).unwrap();
        assert_eq!(
            (status, refusal),
            (&crate::ResultStatus::Succeeded, None),
            "{id}"
        );
        let served = messages.iter().find_map(|body| match body {
            crate::Body::Checkpoint { data, .. }
                if data["step"] == "qoder-cli-session-finished" =>
            {
                Some(data["model"].clone())
            }
            _ => None,
        });
        assert_eq!(served, Some(json!(id)), "{messages:?}");
        let argv = std::fs::read_to_string(dir.path().join("argv")).unwrap();
        let pinned: Vec<&str> = argv.lines().skip(6).take(2).collect();
        assert_eq!(pinned, ["-m", id]);
    }
}
