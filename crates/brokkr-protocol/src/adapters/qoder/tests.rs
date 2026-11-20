//! Qoder CLI 1.1's recorded turns (`docs/evidence/adapters/qoder-cli-1-1/`)
//! folded byte for byte, the launch it is given, and the refusals R6 and
//! R8 of #1178 rule.

use serde_json::{json, Map, Value};

use super::*;
use crate::adapters::AdapterKind;
use crate::RefusalClass;

macro_rules! recorded {
    ($file:literal) => {
        include_str!(concat!(
            "../../../../../docs/evidence/adapters/qoder-cli-1-1/",
            $file
        ))
    };
}

const TOOL_LESS: &str = recorded!("tool-less-turn.ndjson");
const STDIN_PROMPT: &str = recorded!("stdin-prompt-turn.ndjson");
const NAMED_MODEL: &str = recorded!("named-model-turn.ndjson");
const TOOLS_TURN: &str = recorded!("tools-turn.ndjson");
const DENIED_DEFAULT: &str = recorded!("denied-command-default.ndjson");
const DENIED_DONT_ASK: &str = recorded!("denied-command-dont-ask.ndjson");
const COMMA_ALLOW: &str = recorded!("comma-allow-list.ndjson");
const UNKNOWN_MODEL: &str = recorded!("unknown-model-fallback.ndjson");
const UNKNOWN_MODEL_STDERR: &str = recorded!("unknown-model-fallback.stderr");
const SIGNED_OUT_STDERR: &str = recorded!("signed-out.stderr");
const BAD_EFFORT_STDERR: &str = recorded!("bad-effort.stderr");

/// One recorded stdout folded under `pin`, as the driver folds it.
struct Folded {
    fold: QoderFold,
    rows: Vec<Value>,
    session_meta: Map<String, Value>,
    outcome: Result<Option<PreSessionRefusal>, PreSessionRefusal>,
}

fn fold(stdout: &str, pin: &str) -> Folded {
    let mut fold = QoderFold::new(pin);
    let mut rows = Vec::new();
    let mut session_meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::None).unwrap();
    let mut outcome = Ok(None);
    for line in stdout.lines() {
        let event: Value = serde_json::from_str(line).unwrap();
        let mut emit = |row: &Value| rows.push(row.clone());
        match fold.read(event, &mut session_meta, &mut transcript, &mut emit) {
            Ok(None) => {}
            Ok(Some(refusal)) => outcome = Ok(Some(refusal)),
            Err(refused) => {
                outcome = Err(refused);
                break;
            }
        }
    }
    transcript.finish(&mut session_meta, &mut |row: &Value| rows.push(row.clone()));
    Folded {
        fold,
        rows,
        session_meta,
        outcome,
    }
}

/// The rows a fold emitted with their observation taken off, which the
/// engine consumes and the store never sees.
fn shown(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .map(|row| {
            let mut row = row.clone();
            row.as_object_mut().unwrap().remove("observation");
            row
        })
        .collect()
}

fn none_transcript() -> Value {
    json!({"step": "transcript", "transcript": {"kind": "none", "locator": "", "home": ""}})
}

fn turn(turn: u64, model: &str) -> Value {
    json!({"step": "seat-turn", "turn": turn, "model": model, "effort": "not reported"})
}

#[test]
fn the_tool_less_and_stdin_prompt_turns_fold_to_one_answered_turn_on_the_pin() {
    for stdout in [TOOL_LESS, STDIN_PROMPT] {
        let folded = fold(stdout, "Fast");
        assert_eq!(folded.outcome, Ok(None));
        assert_eq!(
            shown(&folded.rows),
            vec![turn(1, "Fast"), none_transcript()]
        );
        assert_eq!(
            Value::Object(folded.session_meta),
            json!({"model": "Fast", "num_turns": 1, "transcript": none_transcript()["transcript"]})
        );
        assert!(folded.fold.worked());
        let last: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
        assert_eq!(last["result"], "READY");
    }
}

#[test]
fn the_named_model_turn_folds_on_its_pin_and_never_on_the_internal_code() {
    let folded = fold(NAMED_MODEL, "Qwen4-Max");
    assert_eq!(folded.outcome, Ok(None));
    // Its thinking and its text are two rows of one message: one turn.
    assert_eq!(
        shown(&folded.rows),
        vec![
            turn(1, "Qwen4-Max"),
            turn(1, "Qwen4-Max"),
            none_transcript()
        ]
    );
    assert_eq!(folded.session_meta["model"], "Qwen4-Max");
    let last: Value = serde_json::from_str(NAMED_MODEL.lines().last().unwrap()).unwrap();
    assert_eq!(last["result"], "READY");
    assert!(NAMED_MODEL.contains("qmodel_latest"));
}

#[test]
fn a_named_model_turn_pinned_to_another_id_is_refused_naming_both() {
    let folded = fold(NAMED_MODEL, "GLM-5.3");
    let refused = folded.outcome.unwrap_err();
    assert_eq!(refused.class(), RefusalClass::Other);
    assert_eq!(
        refused.words(),
        "model_substituted: the seat pinned \"GLM-5.3\" and Qoder served \"Qwen4-Max\""
    );
    // Nothing of a refused seat is folded: only the transcript's end.
    assert_eq!(folded.rows, vec![none_transcript()]);
}

#[test]
fn the_unknown_model_turn_is_refused_at_init_naming_auto() {
    let folded = fold(UNKNOWN_MODEL, "NoSuchModel");
    assert_eq!(
        folded.outcome.unwrap_err().words(),
        "model_substituted: the seat pinned \"NoSuchModel\" and Qoder served \"Auto\""
    );
    assert_eq!(folded.rows, vec![none_transcript()]);
    assert!(!folded.fold.worked());
}

#[test]
fn the_tools_turn_folds_its_write_and_bash_calls() {
    let folded = fold(TOOLS_TURN, "Fast");
    assert_eq!(folded.outcome, Ok(None));
    let rows = shown(&folded.rows);
    let tools: Vec<&str> = rows.iter().filter_map(|row| row["tool"].as_str()).collect();
    assert_eq!(tools, ["Write", "Bash"]);
    assert_eq!(rows.last(), Some(&none_transcript()));
    // Each call rides its observation, Claude's format and Qoder's own id.
    let calls: Vec<&str> = folded
        .rows
        .iter()
        .filter_map(|row| row.pointer("/observation/call").and_then(Value::as_str))
        .collect();
    assert_eq!(
        calls,
        [
            "chatcmpl-tool-8c07364b0a8e4da6",
            "chatcmpl-tool-bb505c1c60a02fa8"
        ]
    );
    // Each call's tool_result answers it, after init, and is no refusal.
    for call in calls {
        assert!(TOOLS_TURN.contains(&format!("\"tool_use_id\":\"{call}\"")));
    }
}

#[test]
fn the_denial_turns_fold_their_refused_call_and_are_not_refused() {
    for (stdout, calls) in [(DENIED_DEFAULT, 3), (DENIED_DONT_ASK, 3), (COMMA_ALLOW, 2)] {
        let folded = fold(stdout, "Fast");
        assert_eq!(folded.outcome, Ok(None));
        let rows = shown(&folded.rows);
        let bash = rows.iter().filter(|row| row["tool"] == "Bash").count();
        assert_eq!(bash, calls);
        // The refused call is one of the folded ones.
        let result: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
        let denied = result["permission_denials"][0]["tool_use_id"]
            .as_str()
            .unwrap();
        assert!(folded
            .rows
            .iter()
            .any(|row| row.pointer("/observation/call") == Some(&json!(denied))));
    }
}

#[test]
fn credits_dollars_and_tokens_of_zero_are_not_reported() {
    for stdout in [TOOL_LESS, TOOLS_TURN, NAMED_MODEL] {
        let folded = fold(
            stdout,
            if stdout == NAMED_MODEL {
                "Qwen4-Max"
            } else {
                "Fast"
            },
        );
        for key in [
            "total_cost_usd",
            "input_tokens",
            "output_tokens",
            "cache_read_tokens",
            "cache_write_tokens",
            "total_credits",
        ] {
            assert!(!folded.session_meta.contains_key(key), "{key}");
            assert!(
                folded.rows.iter().all(|row| row.get(key).is_none()),
                "{key}"
            );
        }
    }
}

#[test]
fn a_turn_before_any_init_is_refused_and_a_row_that_is_no_object_passes() {
    let stdout = TOOL_LESS.lines().skip(1).collect::<Vec<_>>().join("\n");
    let refused = fold(&stdout, "Fast").outcome.unwrap_err();
    assert_eq!(
        refused.words(),
        "model_unconfirmed: the seat pinned \"Fast\" and no init row named the model Qoder served"
    );
    let folded = fold("5\n", "Fast");
    assert_eq!(folded.outcome, Ok(None));
    assert_eq!(folded.rows, vec![none_transcript()]);
}

#[test]
fn a_fallback_line_refuses_the_seat_even_with_no_init() {
    let folded = fold("", "NoSuchModel");
    let refused = folded
        .fold
        .identity(&seen(UNKNOWN_MODEL_STDERR), 0)
        .unwrap();
    assert_eq!(
        refused.words(),
        "model_substituted: the seat pinned \"NoSuchModel\" and Qoder reported on stderr that \
         it fell back to another model"
    );
    // Whatever init said, and whatever the CLI exited with.
    let confirmed = fold(TOOL_LESS, "Fast").fold;
    assert_eq!(
        confirmed
            .identity(&seen(UNKNOWN_MODEL_STDERR), 1)
            .unwrap()
            .words(),
        "model_substituted: the seat pinned \"Fast\" and Qoder reported on stderr that it fell \
         back to another model"
    );
}

#[test]
fn a_clean_exit_with_no_init_is_refused_and_one_with_init_is_not() {
    let empty = fold("", "Fast");
    assert_eq!(
        empty.fold.identity(&seen(""), 0).unwrap().words(),
        UNCONFIRMED
    );
    assert_eq!(fold(TOOL_LESS, "Fast").fold.identity(&seen(""), 0), None);
    assert_eq!(empty.fold.identity(&seen(""), 2), None);
    assert_eq!(QoderFold::failed(&seen("something else\n"), 2), None);
    assert_eq!(QoderFold::failed(&seen(SIGNED_OUT_STDERR), 0), None);
}

/// The unconfirmed refusal's words for a seat pinned to `Fast`.
const UNCONFIRMED: &str =
    "model_unconfirmed: the seat pinned \"Fast\" and no init row named the model Qoder served";

/// `stderr` as the checks observe it.
fn seen(stderr: &str) -> Observed {
    Observed::of(stderr.as_bytes())
}

/// Init's row of the tool-less turn with its `model` taken off.
fn nameless_init() -> String {
    let mut init: Value = serde_json::from_str(TOOL_LESS.lines().next().unwrap()).unwrap();
    assert_eq!(init["subtype"], "init");
    init.as_object_mut().unwrap().remove("model").unwrap();
    format!("{init}\n")
}

/// V5: an init that names no model names nothing. Folded, it is no
/// refusal; a fallback notice then gives the "reported on stderr" words,
/// and with none a clean exit is unconfirmed, as when no init arrived.
#[test]
fn an_init_naming_no_model_leaves_the_served_model_unset() {
    let folded = fold(&nameless_init(), "Fast");
    assert_eq!(folded.outcome, Ok(None));
    assert_eq!(
        folded
            .fold
            .identity(&seen(UNKNOWN_MODEL_STDERR), 0)
            .unwrap()
            .words(),
        FELL_BACK
    );
    assert_eq!(
        folded.fold.identity(&seen(""), 0).unwrap().words(),
        UNCONFIRMED
    );
}

#[test]
fn signed_out_is_a_refusal_naming_the_sign_in_with_no_ansi_byte() {
    assert!(SIGNED_OUT_STDERR.contains('\u{1b}'));
    let refused = QoderFold::failed(&seen(SIGNED_OUT_STDERR), 1).unwrap();
    assert_eq!(refused.words(), SIGNED_OUT_WORDS);
    assert!(!refused.words().contains('\u{1b}'));
    assert!(!refused.words().contains("[31m"));
}

/// The signed-out refusal's fixed words (U1).
const SIGNED_OUT_WORDS: &str = "not_logged_in: Qoder CLI is signed out: run `qodercli login`, or \
                                set QODER_PERSONAL_ACCESS_TOKEN under a private QODER_CONFIG_DIR";

#[test]
fn a_bad_effort_is_a_launch_refusal() {
    let refused = QoderFold::failed(&seen(BAD_EFFORT_STDERR), 2).unwrap();
    assert_eq!(
        refused.words(),
        "launch_refused: Qoder exited 2 on an option it refuses"
    );
}

/// V2: an escape ends where ECMA-48 ends it, and the text after it is
/// intact. A lone `ESC` goes and the byte after it stays (V2 moves this
/// golden: round four's stripper dropped that byte too).
#[test]
fn escapes_are_stripped_as_ecma_48_ends_them() {
    assert_eq!(without_ansi("\u{1b}[1;31mred\u{1b}[0m."), "red.");
    assert_eq!(without_ansi("plain"), "plain");
    assert_eq!(without_ansi("a\u{1b}b"), "ab");
    assert_eq!(without_ansi("a\u{1b}"), "a");
    // A CSI's parameter and intermediate bytes, then each kind of final
    // byte, the final consumed and nothing after it.
    for last in ['m', '@', '`', '~'] {
        let text = format!("\u{1b}[12;3? !{last}using it");
        assert_eq!(without_ansi(&text), "using it", "{last}");
    }
    // A malformed CSI ends at a byte outside its ranges, which is kept: a
    // newline, a non-ASCII letter, or the escape of the next sequence.
    assert_eq!(without_ansi("\u{1b}[1;\nusing"), "\nusing");
    assert_eq!(without_ansi("\u{1b}[1é"), "é");
    assert_eq!(without_ansi("\u{1b}[1\u{1b}[0musing"), "using");
    // An OSC runs to BEL or to ST, an ESC inside it that is no ST
    // included, and to the text's end when it has neither.
    assert_eq!(without_ansi("\u{1b}]0;title\u{7}using"), "using");
    assert_eq!(without_ansi("\u{1b}]8;;x\u{1b}y\u{1b}\\using"), "using");
    assert_eq!(without_ansi("a\u{1b}]0;title"), "a");
    // Any other ESC before a byte from 0x40 to 0x5F is two bytes.
    assert_eq!(without_ansi("\u{1b}Musing \u{1b}_x"), "using x");
}

/// V3: a marker is said where the raw text or the stripped text holds it,
/// whichever the escapes left whole.
#[test]
fn a_marker_is_caught_in_the_raw_or_the_stripped_text() {
    // Split by a colour sequence: whole only once stripped.
    let split = seen("\u{1b}[31mNot \u{1b}[1mlogged in\u{1b}[0m\n");
    assert_eq!(
        QoderFold::failed(&split, 1).unwrap().words(),
        SIGNED_OUT_WORDS
    );
    // An ESC the stripping reads as a two-byte sequence breaks the marker
    // it leads: whole only raw.
    assert_eq!(without_ansi("\u{1b}Not logged in"), &SIGNED_OUT[1..]);
    let broken = seen("\u{1b}Not logged in\n");
    assert_eq!(
        QoderFold::failed(&broken, 1).unwrap().words(),
        SIGNED_OUT_WORDS
    );
    // A CSI left open before the fallback line ends at its `i`.
    let line = "Model x \u{1b}[1is not available right now; using auto\n";
    assert!(!without_ansi(line).contains(FALLBACK));
    let confirmed = fold(TOOL_LESS, "Fast").fold;
    assert_eq!(
        confirmed.identity(&seen(line), 0).unwrap().words(),
        FELL_BACK
    );
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The engine's empty set, its file at `/tmp/e/mcp.json`.
const EMPTY: Engine<'static> = Engine {
    file: "/tmp/e/mcp.json",
    grants: &[],
};

/// The launch of `extra` served `engine`, its command alone.
fn command(extra: &[&str], engine: Engine<'_>) -> Vec<String> {
    launch("qodercli", &argv(extra), engine).unwrap().command
}

/// The print head every launch leads with.
const LEAD: [&str; 7] = [
    "qodercli",
    "-p",
    "--output-format",
    "stream-json",
    "--no-session-persistence",
    "--permission-mode",
    "accept_edits",
];

/// The isolation and the native tools every launch carries after the seat.
const ISOLATED: [&str; 12] = [
    "--strict-mcp-config",
    "--mcp-config",
    "/tmp/e/mcp.json",
    "--setting-sources",
    "",
    "--tools",
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "Bash",
];

/// With no engine server: the print head, the seat's controls, the
/// three-flag isolation, the fixed tools and the meta-tools denied.
#[test]
fn a_qoder_launch_is_the_print_head_the_seat_controls_the_isolation_and_the_fixed_tools() {
    let extra = [
        "-m",
        "Qwen4-Max",
        "--reasoning-effort",
        "high",
        "--allowed-tools",
        "Bash(cargo:*),Bash(git:*)",
    ];
    let denied = ["--disallowed-tools", "mcp_call,mcp_get,mcp_list"];
    assert_eq!(
        launch("qodercli", &argv(&extra), EMPTY).unwrap(),
        Launch {
            command: argv(&[&LEAD[..], &extra, &ISOLATED, &denied].concat()),
            pin: "Qwen4-Max".to_string(),
        }
    );
}

/// With one engine server: the meta-tools are kept, and its tool is granted
/// in the seat's one `--allowed-tools` value beside its Bash pattern, or as
/// that control where the seat grants nothing of its own (Q12 to Q15).
#[test]
fn an_engine_server_keeps_the_meta_tools_and_is_granted_in_the_one_allow_list() {
    let grants = ["mcp__brokkr__workspace".to_string()];
    let served = Engine {
        file: "/tmp/e/mcp.json",
        grants: &grants,
    };
    let golden = |seat: &[&str]| argv(&[&LEAD[..], seat, &ISOLATED].concat());
    assert_eq!(
        command(&["-m", "Fast", "--allowed-tools", "Bash(git:*)"], served),
        golden(&[
            "-m",
            "Fast",
            "--allowed-tools",
            "Bash(git:*),mcp__brokkr__workspace"
        ])
    );
    assert_eq!(
        command(&["--allowed-tools", "Bash(ls:*)", "-m", "Fast"], served),
        golden(&[
            "--allowed-tools",
            "Bash(ls:*),mcp__brokkr__workspace",
            "-m",
            "Fast"
        ])
    );
    assert_eq!(
        command(&["-m", "Fast"], served),
        golden(&["-m", "Fast", "--allowed-tools", "mcp__brokkr__workspace"])
    );
}

/// No launch carries `--strict-mcp-config` without `--setting-sources ""`
/// beside it (Q03 to Q06): every launch, served a server or none, carries
/// the fragment's one home once and no flag of it elsewhere, and a seat
/// may write none of its flags.
#[test]
fn no_launch_carries_the_strict_flag_without_the_empty_setting_sources() {
    let grants = ["mcp__brokkr__workspace".to_string()];
    let served = Engine {
        file: "/tmp/e/mcp.json",
        grants: &grants,
    };
    let seats: [&[&str]; 2] = [
        &["-m", "Fast"],
        &["-m", "Fast", "--allowed-tools", "Bash(ls:*)"],
    ];
    for engine in [EMPTY, served] {
        for seat in seats {
            let command = command(seat, engine);
            let strict = command
                .iter()
                .position(|part| part == "--strict-mcp-config");
            let strict = strict.expect("the strict flag");
            assert_eq!(command[strict..strict + 5], isolation("/tmp/e/mcp.json"));
            for flag in ["--strict-mcp-config", "--mcp-config", "--setting-sources"] {
                let count = command.iter().filter(|part| *part == flag).count();
                assert_eq!(count, 1, "{flag} in {command:?}");
            }
        }
    }
    for flag in ["--strict-mcp-config", "--setting-sources", "--mcp-config"] {
        let refused = launch("qodercli", &argv(&["-m", "Fast", flag, "x"]), EMPTY);
        assert!(
            matches!(&refused, Err(LaunchError::Unadmitted(said)) if said == flag),
            "{refused:?}"
        );
    }
}

#[test]
fn a_launch_admits_only_its_three_controls_once_each_with_a_value_and_a_pin() {
    let refused = |parts: &[&str]| launch("qodercli", &argv(parts), EMPTY).unwrap_err();
    let tools = refused(&["-m", "Fast", "--tools", "WebFetch"]);
    assert!(
        matches!(&tools, LaunchError::Unadmitted(said) if said == "--tools"),
        "{tools:?}"
    );
    assert!(matches!(refused(&["Fast"]), LaunchError::Positional));
    assert!(matches!(
        refused(&["-m", "Fast", "-m", "Auto"]),
        LaunchError::Doubled("-m")
    ));
    for valueless in [
        &["-m"][..],
        &["-m", "--reasoning-effort", "high"],
        &["-m", ""],
    ] {
        assert!(matches!(refused(valueless), LaunchError::Valueless("-m")));
    }
    assert!(matches!(
        refused(&["--reasoning-effort", "low"]),
        LaunchError::Unpinned
    ));
    assert_eq!(
        LaunchError::Unadmitted("--tools".into()).to_string(),
        "refusing to invoke the agent CLI: the seat's arguments carry '--tools', which brokkr \
         driver qoder does not admit; a qoder seat's arguments are -m, --reasoning-effort and \
         --allowed-tools, each once (#1178)"
    );
    assert_eq!(
        LaunchError::Positional.to_string(),
        "refusing to invoke the agent CLI: the seat's arguments carry a value with no option \
         before it, which brokkr driver qoder does not admit (#1178)"
    );
    assert_eq!(
        LaunchError::Doubled("-m").to_string(),
        "refusing to invoke the agent CLI: the seat's arguments carry '-m' more than once"
    );
    assert_eq!(
        LaunchError::Valueless("-m").to_string(),
        "refusing to invoke the agent CLI: the seat's arguments carry '-m' with no value"
    );
    assert_eq!(
        LaunchError::Unpinned.to_string(),
        "refusing to invoke the agent CLI: the seat pins no model with -m, and Qoder serves a \
         model of its own choosing when none is named; a provisional seat pins its id (#1178)"
    );
    assert_eq!(
        LaunchError::Unnamed.to_string(),
        "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat has a path \
         that is not UTF-8, so no argument names it exactly (#1178)"
    );
}

/// A fake `qodercli` under `dir`: it records its argv, reads the prompt,
/// prints `stdout` and `stderr` as recorded, writes `result` when one is
/// given, and exits `code`.
#[cfg(unix)]
fn fake_qoder(
    dir: &std::path::Path,
    stdout: impl AsRef<[u8]>,
    stderr: &str,
    result: Option<&std::path::Path>,
    code: i32,
) -> std::path::PathBuf {
    std::fs::write(dir.join("stdout"), stdout).unwrap();
    std::fs::write(dir.join("stderr"), stderr).unwrap();
    let write = result.map_or(String::new(), |path| {
        format!(
            "printf '{{\"result\":\"complete\"}}' > '{}'\n",
            path.display()
        )
    });
    // The MCP file the launch names is copied while the turn runs.
    let body = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{dir}/argv'\ncat > '{dir}/prompt'\n\
         prev=; for a in \"$@\"; do [ \"$prev\" = --mcp-config ] && cat \"$a\" > '{dir}/mcp'; \
         prev=$a; done\n\
         cat '{dir}/stderr' >&2\ncat '{dir}/stdout'\n{write}exit {code}\n",
        dir = dir.display()
    );
    super::super::tests::executable(dir, "qodercli", &body)
}

/// One qoder seat run through the driver's own `run_seat` on `bin`.
#[cfg(unix)]
fn run(dir: &std::path::Path, bin: &std::path::Path, extra: &[&str]) -> Vec<crate::Body> {
    run_sealed(dir, bin, extra, None)
}

/// [`run`] with `serving` as the sealed serving inputs, where given.
#[cfg(unix)]
fn run_sealed(
    dir: &std::path::Path,
    bin: &std::path::Path,
    extra: &[&str],
    serving: Option<Value>,
) -> Vec<crate::Body> {
    run_offered(dir, bin, extra, serving, None)
}

/// [`run_sealed`] with the start offered `session` to resume, where given.
#[cfg(unix)]
fn run_offered(
    dir: &std::path::Path,
    bin: &std::path::Path,
    extra: &[&str],
    serving: Option<Value>,
    session: Option<&str>,
) -> Vec<crate::Body> {
    let mut input = seat_input(dir);
    if let Some(serving) = serving {
        input[crate::native_controls::SERVING_INPUTS] = serving;
    }
    run_input(extra, input, session, &[("BROKKR_QODER_BIN", bin)])
}

/// A qoder seat's input in `dir`, its result at `result.json`.
#[cfg(unix)]
fn seat_input(dir: &std::path::Path) -> Value {
    json!({
        "role_path": "/dev/null", "phase": "work", "feature": "f",
        "allowed_results": ["complete"], "result_path": dir.join("result.json"), "workdir": dir,
    })
}

/// [`run`] with [`SECRET`] bound to the seat from a store under `dir`, and
/// `vars` set beside the binary.
#[cfg(unix)]
fn run_bound(
    dir: &std::path::Path,
    bin: &std::path::Path,
    extra: &[&str],
    vars: &[(&str, &std::path::Path)],
) -> Vec<crate::Body> {
    let store = dir.join("secrets.env");
    secret::store_set(&store, "QT", SECRET).unwrap();
    let mut input = seat_input(dir);
    input["secrets"] = json!(["QT"]);
    input["secrets_file"] = json!(store);
    let vars = [&[("BROKKR_QODER_BIN", bin)], vars].concat();
    run_input(extra, input, None, &vars)
}

/// One qoder seat on `input` run through the driver's own `run_seat`, with
/// `vars` set while it runs.
#[cfg(unix)]
fn run_input(
    extra: &[&str],
    input: Value,
    session: Option<&str>,
    vars: &[(&str, &std::path::Path)],
) -> Vec<crate::Body> {
    let mut env = crate::env_guard::EnvGuard::lock();
    for (key, value) in vars {
        env.set(key, value);
    }
    let start = json!({"attempt_id": "attempt", "effect_id": "effect", "input": input});
    let mut messages = Vec::new();
    super::super::run_seat(
        AdapterKind::Qoder,
        &argv(extra),
        &start,
        session,
        &mut |body| messages.push(body),
    );
    messages
}

/// The status and typed refusal of a run's one result, if it sent one.
#[cfg(unix)]
fn result_of(
    messages: &[crate::Body],
) -> Option<(&crate::ResultStatus, Option<&PreSessionRefusal>)> {
    messages.iter().find_map(|body| match body {
        crate::Body::Result {
            status, refusal, ..
        } => Some((status, refusal.as_ref())),
        _ => None,
    })
}

#[cfg(unix)]
#[test]
fn a_qoder_seat_runs_the_overridden_binary_and_finishes_on_its_pin() {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", Some(&result), 0);
    let messages = run(
        dir.path(),
        &bin,
        &["-m", "Fast", "--reasoning-effort", "low"],
    );
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!((status, refusal), (&crate::ResultStatus::Succeeded, None));
    let finished = messages.iter().find_map(|body| match body {
        crate::Body::Checkpoint { data, .. } if data["step"] == "qoder-cli-session-finished" => {
            Some(data.clone())
        }
        _ => None,
    });
    assert_eq!(
        finished.unwrap(),
        json!({
            "step": "qoder-cli-session-finished", "exit_code": 0, "model": "Fast",
            "effort": "not reported", "num_turns": 1,
            "transcript": {"kind": "none", "locator": "", "home": ""},
        })
    );
    let argv = std::fs::read_to_string(dir.path().join("argv")).unwrap();
    let mut argv: Vec<&str> = argv.lines().collect();
    // The engine's file is private to the turn and gone after it.
    let file = argv[12].to_string();
    assert!(file.ends_with("/mcp.json"), "{file}");
    assert!(!std::path::Path::new(&file).exists(), "{file}");
    argv[12] = "<file>";
    assert_eq!(
        std::fs::read_to_string(dir.path().join("mcp")).unwrap(),
        r#"{"mcpServers":{}}"#
    );
    assert_eq!(
        argv,
        [
            "-p",
            "--output-format",
            "stream-json",
            "--no-session-persistence",
            "--permission-mode",
            "accept_edits",
            "-m",
            "Fast",
            "--reasoning-effort",
            "low",
            "--strict-mcp-config",
            "--mcp-config",
            "<file>",
            "--setting-sources",
            "",
            "--tools",
            "Read",
            "Edit",
            "Write",
            "Glob",
            "Grep",
            "Bash",
            "--disallowed-tools",
            "mcp_call,mcp_get,mcp_list",
        ]
    );
    let prompt = std::fs::read_to_string(dir.path().join("prompt")).unwrap();
    assert!(
        prompt.contains("result"),
        "the prompt went on stdin: {prompt}"
    );
}

#[cfg(unix)]
#[test]
fn a_qoder_line_that_is_not_utf8_ends_the_reading_as_it_always_has() {
    // Behind the line, the recorded turn is never read: the clean exit
    // confirmed no model, so the delivered file is no answer and the
    // attempt sends no result (#1200's byte seam leaves qoder unchanged).
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let stdout = [b"\xff\n".as_slice(), TOOL_LESS.as_bytes()].concat();
    let bin = fake_qoder(dir.path(), &stdout, "", Some(&result), 0);
    let messages = run(dir.path(), &bin, &["-m", "Fast"]);
    assert_eq!(result_of(&messages), None);
    let turned = |body: &crate::Body| matches!(body, crate::Body::Checkpoint { data, .. } if data["step"] == "seat-turn");
    assert!(!messages.iter().any(turned), "{messages:?}");
}

#[cfg(unix)]
#[test]
fn a_substituted_model_is_refused_at_init_and_its_harness_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let bin = fake_qoder(
        dir.path(),
        UNKNOWN_MODEL,
        UNKNOWN_MODEL_STDERR,
        Some(&result),
        0,
    );
    let messages = run(dir.path(), &bin, &["-m", "NoSuchModel"]);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    let refusal = refusal.unwrap();
    assert_eq!(
        refusal.words(),
        "model_substituted: the seat pinned \"NoSuchModel\" and Qoder served \"Auto\""
    );
    assert_eq!(refusal.provider(), "qoder");
    let quiet = messages.iter().all(|body| {
        !matches!(
            body,
            crate::Body::Accepted { .. } | crate::Body::Checkpoint { .. }
        )
    });
    assert!(quiet, "{messages:?}");
}

#[cfg(unix)]
#[test]
fn a_signed_out_qoder_is_a_refusal_with_no_ansi_byte() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), "", SIGNED_OUT_STDERR, None, 1);
    let messages = run(dir.path(), &bin, &["-m", "Fast"]);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    assert_eq!(refusal.unwrap().words(), SIGNED_OUT_WORDS);
}

#[cfg(unix)]
#[test]
fn a_fallback_line_behind_a_folded_turn_leaves_no_result() {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let bin = fake_qoder(
        dir.path(),
        TOOL_LESS,
        UNKNOWN_MODEL_STDERR,
        Some(&result),
        0,
    );
    let messages = run(dir.path(), &bin, &["-m", "Fast"]);
    assert_eq!(result_of(&messages), None, "{messages:?}");
}

/// Sealed serving inputs the engine's set cannot be read from launch
/// nothing: no MCP file is served unread.
#[cfg(unix)]
#[test]
fn an_unreadable_mcp_intent_launches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let unread = Some(json!({"unknown": true}));
    let messages = run_sealed(dir.path(), &bin, &["-m", "Fast"], unread);
    assert_eq!(
        error_of(&messages),
        Some((
            &crate::ResultStatus::Failed,
            Some(
                "refusing to invoke the agent CLI: the engine's MCP isolation intent cannot be \
                 read, so no strict MCP configuration is built"
                    .to_string()
            )
        ))
    );
    assert!(!dir.path().join("argv").exists());
}

/// The status and error of a run's one result, if it sent one.
#[cfg(unix)]
fn error_of(messages: &[crate::Body]) -> Option<(&crate::ResultStatus, Option<String>)> {
    messages.iter().find_map(|body| match body {
        crate::Body::Result { status, error, .. } => Some((status, error.clone())),
        _ => None,
    })
}

#[cfg(unix)]
#[test]
fn an_unadmitted_argument_launches_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let messages = run(dir.path(), &bin, &["-m", "Fast", "--continue"]);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!((status, refusal), (&crate::ResultStatus::Failed, None));
    assert!(!dir.path().join("argv").exists());
}

#[cfg(unix)]
#[test]
fn a_fallback_line_with_no_init_is_a_refusal_to_start_or_no_result() {
    // A result the unconfirmed model wrote is never accepted.
    let delivered = tempfile::tempdir().unwrap();
    let result = delivered.path().join("result.json");
    let bin = fake_qoder(delivered.path(), "", UNKNOWN_MODEL_STDERR, Some(&result), 0);
    let messages = run(delivered.path(), &bin, &["-m", "NoSuchModel"]);
    assert_eq!(result_of(&messages), None, "{messages:?}");
    // With nothing written, it is a refusal to start.
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), "", UNKNOWN_MODEL_STDERR, None, 0);
    let messages = run(dir.path(), &bin, &["-m", "NoSuchModel"]);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    assert_eq!(
        refusal.unwrap().words(),
        "model_substituted: the seat pinned \"NoSuchModel\" and Qoder reported on stderr that \
         it fell back to another model"
    );
}

/// What the driver declares of itself: no resume receipt, no probe, its
/// own name and provider, and no route.
#[test]
fn a_qoder_driver_declares_no_resume_and_no_probe() {
    let kind = AdapterKind::parse("qoder").unwrap();
    assert_eq!(kind, AdapterKind::Qoder);
    assert_eq!(kind.supports(), Vec::<String>::new());
    assert!(!kind.advertises_probe());
    assert_eq!(kind.driver_name(), "qoder-cli");
    assert_eq!(kind.provider_name(), "qoder");
    assert_eq!(kind.served_route("Fast"), None);
    assert!(!super::super::probe::is_answer(
        kind,
        &json!({"type": "assistant"})
    ));
}

/// The launch's typed errors render once: the admission's refusal as every
/// driver states it, and an MCP file that could not be written with the
/// I/O error's words.
#[test]
fn a_launch_whose_set_is_refused_or_unwritten_states_why() {
    let refused = LaunchError::from(McpRefusal::Unreadable);
    assert!(matches!(refused, LaunchError::Mcp(McpRefusal::Unreadable)));
    // The prefix is stated once (U5).
    let stated = refused.to_string();
    assert_eq!(stated, McpRefusal::Unreadable.at_launch());
    assert_eq!(
        stated.matches("refusing to invoke the agent CLI").count(),
        1
    );
    let missing = Path::new("/nonexistent-brokkr-qoder-root");
    let unwritten = staged(&json!({"mcpServers": {}}), missing).unwrap_err();
    let LaunchError::Unstaged(cause) = &unwritten else {
        panic!("{unwritten:?}");
    };
    assert_eq!(cause.kind(), std::io::ErrorKind::NotFound);
    // The directory's name is random, so the words are read up to it.
    let words = "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat was \
                 not written: No such file or directory (os error 2) at path \
                 \"/nonexistent-brokkr-qoder-root/brokkr-qoder-mcp-";
    let stated = unwritten.to_string();
    assert!(stated.starts_with(words), "{stated}");
    assert_eq!(
        stated.matches("refusing to invoke the agent CLI").count(),
        1
    );
}

/// Serving inputs sealing `servers` beside the default hands spec under
/// `stands`, their cold assessment `cold`, the replacement measured, under
/// `fence`.
fn sealed(
    stands: crate::native_controls::SealedBoundary,
    servers: &str,
    cold: Value,
    fence: &str,
) -> Value {
    let dialect = crate::native_controls::SealedDialect {
        stands: Some(stands),
        ..Default::default()
    };
    let spec = Some(crate::hands::HandsSpec::default());
    let mut serving = super::super::mcp::tests::dispatched(dialect, Vec::new(), spec);
    serving["isolation"] = json!({
        "servers": servers, "cold": cold, "replacement": "measured", "resume": {}, "fence": fence,
    });
    serving
}

/// The error a seat run on `serving` failed with before its fake
/// `qodercli` started, which it records on start.
#[cfg(unix)]
fn refused_before_spawn(serving: Value) -> Option<String> {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let messages = run_sealed(dir.path(), &bin, &["-m", "Fast"], Some(serving));
    assert!(!dir.path().join("argv").exists(), "{messages:?}");
    messages.iter().find_map(|body| match body {
        crate::Body::Result {
            status: crate::ResultStatus::Failed,
            error,
            ..
        } => error.clone(),
        _ => None,
    })
}

/// SI2 before spawn (T1): past the lifted fence an unmeasured or
/// unsupported cold shape refuses, and a sealed empty intent beside boxed
/// hands refuses whichever way the fence stands; the harness never starts.
#[cfg(unix)]
#[test]
fn a_launch_the_admission_refuses_never_starts_its_harness() {
    use crate::native_controls::SealedBoundary::{Namespace, Open};
    let lifted = |cold: Value| sealed(Open, "empty", cold, "lifted");
    assert_eq!(
        refused_before_spawn(lifted(json!("unmeasured"))).as_deref(),
        Some(
            "refusing to invoke the agent CLI: provider 'qoder' has no measured strict MCP \
             configuration for 'cold'"
        )
    );
    assert_eq!(
        refused_before_spawn(lifted(json!({"unsupported": "a reason"}))).as_deref(),
        Some(
            "refusing to invoke the agent CLI: provider 'qoder' cannot exclude ambient MCP \
             configuration (a reason)"
        )
    );
    for fence in ["lifted", "standing"] {
        let mismatched = sealed(Namespace, "empty", json!("measured"), fence);
        assert_eq!(
            refused_before_spawn(mismatched).as_deref(),
            Some(
                "refusing to invoke the agent CLI: provider 'qoder' final MCP configuration is \
                 not the engine's sealed server set"
            ),
            "{fence}"
        );
    }
}

/// A hands set the standing fence serves unmeasured is served the empty
/// set: the engine's file is the empty document and the meta-tools are
/// denied, so no hands and no ambient server start.
#[cfg(unix)]
#[test]
fn an_unadmitted_hands_set_under_the_standing_fence_is_served_the_empty_set() {
    use crate::native_controls::SealedBoundary::Namespace;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let serving = sealed(Namespace, "hands", json!("unmeasured"), "standing");
    run_sealed(dir.path(), &bin, &["-m", "Fast"], Some(serving));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("mcp")).unwrap(),
        r#"{"mcpServers":{}}"#
    );
    let argv = std::fs::read_to_string(dir.path().join("argv")).unwrap();
    let argv: Vec<&str> = argv.lines().collect();
    assert_eq!(
        argv[argv.len() - 2..],
        ["--disallowed-tools", "mcp_call,mcp_get,mcp_list"]
    );
}

/// A short secret bound to the seat, and Qoder's stderr lines carrying it,
/// each coloured as Qoder colours its refusals: whole, split by a colour
/// reset, and (bound with a space) split by a tab that a whitespace
/// collapse would rejoin.
const SECRET: &str = "tok9z";
const SPACED: &str = "tok 9z";
const FALLBACK_SPLIT: &str = "\u{1b}[33mModel \"tok\u{1b}[0m9z\" is not available right now; \
                              using \"auto\" instead.\u{1b}[0m\n";
const FALLBACK_TABBED: &str =
    "\u{1b}[33mModel \"tok\t9z\" is not available right now; using \"auto\" instead.\u{1b}[0m\n";
const SIGNED_OUT_SECRET: &str = "\u{1b}[31mNot logged in · token tok9z expired\u{1b}[0m\n";
const CRASHED_SECRET: &str = "An unexpected critical error occurred:Error: bad token tok9z\n";

/// The fallback refusal's fixed words for a seat pinned to `Fast` (U1).
const FELL_BACK: &str = "model_substituted: the seat pinned \"Fast\" and Qoder reported on \
                         stderr that it fell back to another model";

/// One turn of a fake `qodercli` pinned to `Fast` that prints `stdout` and
/// `stderr`, writes its result where `delivered`, and exits `code`, run by
/// [`drive`] with `bound` bound; beside it, `bound` as masked.
#[cfg(unix)]
fn invoked(
    stdout: &str,
    stderr: &str,
    delivered: bool,
    code: i32,
    bound: &str,
) -> (Result<Invocation, String>, String) {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let written = delivered.then_some(result.as_path());
    let bin = fake_qoder(dir.path(), stdout, stderr, written, code);
    let input = json!({"workdir": dir.path(), "result_path": result});
    let (bindings, masked) = bound_as(bound);
    let bin = bin.to_string_lossy();
    let extra = argv(&["-m", "Fast"]);
    let turn = drive(
        &bin,
        &extra,
        false,
        "prompt",
        &input,
        &bindings,
        &mut |_| {},
    );
    (turn, masked)
}

/// `bound` bound to the seat, and as its mask writes it.
fn bound_as(bound: &str) -> ([secret::BoundSecret; 1], String) {
    let bindings = [super::super::tests::binding("QT", bound)];
    let masked = secret::mask_text(bound.as_bytes(), &bindings);
    assert_ne!(masked, bound);
    (bindings, masked)
}

/// Round four's counterexample (V2): a CSI ending in `@` just before
/// `using`. Init names the pin and the CLI exits 0 having delivered its
/// result, and the seat is still refused as substituted: late, behind a
/// folded turn, and before the session, behind init alone. Its evidence
/// keeps `using` whole.
#[cfg(unix)]
#[test]
fn a_fallback_line_with_a_csi_ending_in_an_at_sign_is_refused() {
    let stderr = "Model \"x\" is not available right now; \u{1b}[@using \"auto\" instead.\n";
    let (late, _) = invoked(TOOL_LESS, stderr, true, 0, SECRET);
    assert_eq!(late.map(drop), Err(FELL_BACK.to_string()));
    let init = format!("{}\n", TOOL_LESS.lines().next().unwrap());
    let (invocation, _) = invoked(&init, stderr, false, 0, SECRET);
    let invocation = invocation.unwrap();
    let refused = invocation.refusal.as_ref().map(PreSessionRefusal::words);
    assert_eq!(refused, Some(FELL_BACK));
    assert_eq!(
        invocation.stderr,
        "Model \"x\" is not available right now; using \"auto\" instead.\n"
    );
}

/// V4: a bound value in an option the seat may not write is masked in
/// `drive`'s error, which keeps the rest of its words, and in the error
/// `lost::invocation_failed` sends for it, since nothing was spawned.
#[cfg(unix)]
#[test]
fn a_bound_value_in_an_unadmitted_option_is_masked() {
    let dir = tempfile::tempdir().unwrap();
    let input = json!({"workdir": dir.path(), "result_path": dir.path().join("r.json")});
    let (bindings, masked) = bound_as(SECRET);
    let extra = argv(&["-m", "Fast", "--tok9z"]);
    let driven = drive(
        "qodercli",
        &extra,
        false,
        "p",
        &input,
        &bindings,
        &mut |_| {},
    );
    let unadmitted = LaunchError::Unadmitted(format!("--{masked}")).to_string();
    assert_eq!(driven.map(drop), Err(unadmitted.clone()));
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let messages = run_bound(dir.path(), &bin, &["-m", "Fast", "--tok9z"], &[]);
    let sent = (&crate::ResultStatus::Failed, Some(unadmitted));
    assert_eq!(error_of(&messages), Some(sent));
    assert!(!dir.path().join("argv").exists());
}

/// Set in the child [`what_a_seat_prints_or_sends_carries_no_bound_value`]
/// runs itself in, so the child's environment is its own.
const PRINTED_CASE: &str = "BROKKR_QODER_PRINTED_CASE";

/// V4, captured: run in a child of its own, a seat pinned to a bound value
/// whose Qoder delivered a result behind a fallback line naming it ends
/// lost, and what `lost::invocation_failed` prints is the masked late
/// error; and a seat whose temporary directory names the value is sent the
/// staging error masked. The parent reads the child's stderr.
#[cfg(unix)]
#[test]
fn what_a_seat_prints_or_sends_carries_no_bound_value() {
    let (_, masked) = bound_as(SECRET);
    if std::env::var_os(PRINTED_CASE).is_some() {
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let stderr = format!("Model \"{SECRET}\" is not available right now; using auto.\n");
        let bin = fake_qoder(dir.path(), "", &stderr, Some(&result), 0);
        let lost = run_bound(dir.path(), &bin, &["-m", SECRET], &[]);
        assert_eq!(error_of(&lost), None, "{lost:?}");
        let unstaged = Path::new("/nonexistent").join(SECRET);
        let vars = [("TMPDIR", unstaged.as_path())];
        let messages = run_bound(dir.path(), &bin, &["-m", "Fast"], &vars);
        let (status, error) = error_of(&messages).unwrap();
        assert_eq!(status, &crate::ResultStatus::Failed);
        let words = format!(
            "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat was not \
             written: No such file or directory (os error 2) at path \"/nonexistent/{masked}/\
             brokkr-qoder-mcp-"
        );
        let error = error.unwrap();
        assert!(error.starts_with(&words), "{error}");
        assert!(!error.contains(SECRET), "{error}");
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "adapters::qoder::tests::what_a_seat_prints_or_sends_carries_no_bound_value",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PRINTED_CASE, "1");
    let output = child.output().unwrap();
    let printed = String::from_utf8_lossy(&output.stderr).into_owned();
    let said = String::from_utf8_lossy(&output.stdout).into_owned() + &printed;
    // A filter that matches nothing exits zero and asserts nothing.
    assert!(said.contains("1 passed"), "{said}");
    let late = format!(
        "{}; the command was spawned, so no result is sent\n",
        FELL_BACK.replace("\"Fast\"", &format!("\"{masked}\""))
    );
    assert!(printed.contains(&late), "{printed}");
    assert!(!said.contains(SECRET), "{said}");
}

/// U1 and U3: a bound value an escape or a tab splits in the fallback line
/// reaches neither the late error, which `lost::invocation_failed` prints
/// as it is, after a folded turn or a delivered result, nor the refusal to
/// start, nor the invocation's stderr, which is stripped before it is
/// masked.
#[cfg(unix)]
#[test]
fn a_split_secret_in_the_fallback_line_reaches_no_word_and_no_stderr() {
    for (bound, stderr) in [(SECRET, FALLBACK_SPLIT), (SPACED, FALLBACK_TABBED)] {
        for (stdout, delivered) in [(TOOL_LESS, false), ("", true)] {
            let (late, _) = invoked(stdout, stderr, delivered, 0, bound);
            assert_eq!(late.map(drop), Err(FELL_BACK.to_string()), "{bound}");
        }
        let (invocation, masked) = invoked("", stderr, false, 0, bound);
        let invocation = invocation.unwrap();
        let refused = invocation.refusal.as_ref().map(PreSessionRefusal::words);
        assert_eq!(refused, Some(FELL_BACK), "{bound}");
        assert!(!invocation.stderr.contains(bound), "{}", invocation.stderr);
        if bound == SECRET {
            assert_eq!(
                invocation.stderr,
                format!("Model \"{masked}\" is not available right now; using \"auto\" instead.\n")
            );
        }
    }
}

/// U2: a binding that overlaps the fallback marker erases nothing the
/// check reads, so a turn init confirmed, that exited 0 and delivered its
/// result, is still refused as substituted, in words with no byte of
/// stderr and none of the binding.
#[cfg(unix)]
#[test]
fn a_binding_overlapping_the_fallback_marker_never_hides_the_fallback() {
    let (late, _) = invoked(TOOL_LESS, UNKNOWN_MODEL_STDERR, true, 0, "using");
    assert_eq!(late.map(drop), Err(FELL_BACK.to_string()));
    // The late error passes the seat's bindings last (U1): a bound pin is
    // masked in it.
    let (late, masked) = invoked("", UNKNOWN_MODEL_STDERR, true, 0, "Fast");
    let words = FELL_BACK.replace("\"Fast\"", &format!("\"{masked}\""));
    assert_eq!(late.map(drop), Err(words));
}

/// U1: the signed-out and crash refusals are fixed words, with no byte of
/// the stderr line that carried a bound value.
#[cfg(unix)]
#[test]
fn a_secret_in_a_signed_out_or_crash_line_reaches_no_word() {
    let cases = [
        (SIGNED_OUT_SECRET, 1, SIGNED_OUT_WORDS),
        (
            CRASHED_SECRET,
            3,
            "launch_refused: Qoder exited 3 on an option it refuses",
        ),
    ];
    for (stderr, code, words) in cases {
        let (invocation, _) = invoked("", stderr, false, code, SECRET);
        let invocation = invocation.unwrap();
        let refused = invocation.refusal.as_ref().map(PreSessionRefusal::words);
        assert_eq!(refused, Some(words), "{stderr}");
        assert!(!invocation.stderr.contains(SECRET), "{}", invocation.stderr);
    }
}

/// U4: a start offered a resume is refused before any provider work, even
/// where the replacement is measured and the cold launch is not: the fake
/// `qodercli`, which records its start, never runs.
#[cfg(unix)]
#[test]
fn a_start_offered_a_resume_never_starts_its_harness() {
    use crate::native_controls::SealedBoundary::Open;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
    let serving = sealed(Open, "empty", json!("unmeasured"), "lifted");
    let messages = run_offered(
        dir.path(),
        &bin,
        &["-m", "Fast"],
        Some(serving),
        Some("s-1"),
    );
    assert_eq!(
        error_of(&messages),
        Some((
            &crate::ResultStatus::Failed,
            Some(LaunchError::Resumed.to_string())
        ))
    );
    assert_eq!(
        LaunchError::Resumed.to_string(),
        "refusing to invoke the agent CLI: the start offers a resume, and the qoder adapter \
         declares no resume (#1178)"
    );
    assert!(!dir.path().join("argv").exists(), "{messages:?}");
}
