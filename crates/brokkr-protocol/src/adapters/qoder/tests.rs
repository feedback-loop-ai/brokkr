//! Qoder CLI 1.1's recorded turns (`docs/evidence/adapters/qoder-cli-1-1/`)
//! folded byte for byte, the launch it is given, and the refusals R6 and
//! R8 of #1178 rule.

use serde_json::{json, Map, Value};

use super::*;
use crate::adapters::escapes::without_ansi;
use crate::adapters::AdapterKind;
use crate::RefusalClass;

#[cfg(unix)]
mod live;

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
    fold_seeing(stdout, pin, "")
}

/// [`fold`], a stop read against `stderr` as the driver reads it.
fn fold_seeing(stdout: &str, pin: &str, stderr: &str) -> Folded {
    fold_bound(stdout.as_bytes(), pin, stderr, &[])
}

/// The plain id `pin` as a launch builds it.
fn pin(pin: &str) -> ModelPin {
    ModelPin::parse(pin).unwrap()
}

/// [`fold_seeing`] over the bytes `stdout`, split into lines as the driver
/// splits them, with `bindings` bound.
fn fold_bound(
    stdout: &[u8],
    pinned: &str,
    stderr: &str,
    bindings: &[secret::BoundSecret],
) -> Folded {
    let mut fold = QoderFold::new(&pin(pinned));
    let mut rows = Vec::new();
    let mut session_meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::None).unwrap();
    let mut outcome = Ok(None);
    for line in std::io::BufRead::split(stdout, b'\n') {
        let mut emit = |row: &Value| rows.push(row.clone());
        let line = line.unwrap();
        match fold.read(
            &line,
            bindings,
            &mut session_meta,
            &mut transcript,
            &mut emit,
        ) {
            Ok(None) => {}
            Ok(Some(refusal)) => outcome = Ok(Some(refusal)),
            Err(stop) => {
                let ended = fold.ended(Some(stop), &seen(stderr), STOPPED_AT_REFUSAL);
                outcome = Err(ended.unwrap());
                break;
            }
        }
    }
    let mut emit = |row: &Value| rows.push(row.clone());
    transcript.finish(&mut session_meta, &mut emit);
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
        "model_substituted: the seat pinned 'GLM-5.3' and Qoder served 'Qwen4-Max'"
    );
    // Nothing of a refused seat is folded: only the transcript's end.
    assert_eq!(folded.rows, vec![none_transcript()]);
}

#[test]
fn the_unknown_model_turn_is_refused_at_init_naming_auto() {
    let folded = fold(UNKNOWN_MODEL, "NoSuchModel");
    assert_eq!(
        folded.outcome.unwrap_err().words(),
        "model_substituted: the seat pinned 'NoSuchModel' and Qoder served 'Auto'"
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

/// Z3 moves this golden: round six passed a row that is no object; it now
/// refuses, as every line no measured stream carries does.
#[test]
fn a_turn_before_any_init_is_refused_and_so_is_a_row_that_is_no_object() {
    let stdout = TOOL_LESS.lines().skip(1).collect::<Vec<_>>().join("\n");
    let refused = fold(&stdout, "Fast").outcome.unwrap_err();
    assert_eq!(refused.words(), UNCONFIRMED);
    for row in ["5\n", "[1]\n", "\"x\"\n", "\n", "not json\n"] {
        let folded = fold(&format!("{}{row}", init()), "Fast");
        assert_eq!(folded.outcome.unwrap_err().words(), UNMEASURED, "{row}");
        assert_eq!(folded.rows, vec![none_transcript()], "{row}");
    }
}

/// The words of a stdout line no measured stream carries (Z3).
const UNMEASURED: &str = "stream_unmeasured: Qoder wrote a stdout line that is not a JSON \
                          object, which no measured Qoder stream carries";

#[test]
fn a_fallback_line_refuses_the_seat_even_with_no_init() {
    let folded = fold("", "NoSuchModel");
    let refused = folded
        .fold
        .ended(None, &seen(UNKNOWN_MODEL_STDERR), 0)
        .unwrap();
    assert_eq!(
        refused.words(),
        "model_substituted: the seat pinned 'NoSuchModel' and Qoder reported on stderr that \
         it fell back to another model"
    );
    // Whatever init said, and whatever the CLI exited with.
    let confirmed = fold(TOOL_LESS, "Fast").fold;
    assert_eq!(
        confirmed
            .ended(None, &seen(UNKNOWN_MODEL_STDERR), 1)
            .unwrap()
            .words(),
        FELL_BACK
    );
}

#[test]
fn a_clean_exit_with_no_init_is_refused_and_one_with_init_is_not() {
    let empty = fold("", "Fast");
    assert_eq!(
        empty.fold.ended(None, &seen(""), 0).unwrap().words(),
        UNCONFIRMED
    );
    assert_eq!(fold(TOOL_LESS, "Fast").fold.ended(None, &seen(""), 0), None);
    assert_eq!(empty.fold.ended(None, &seen(""), 2), None);
    assert_eq!(QoderFold::failed(&seen("something else\n"), 2), None);
    assert_eq!(QoderFold::failed(&seen(SIGNED_OUT_STDERR), 0), None);
}

/// The unconfirmed refusal's words for a seat pinned to `Fast`.
const UNCONFIRMED: &str =
    "model_unconfirmed: the seat pinned 'Fast' and no init row named the model Qoder served";

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
            .ended(None, &seen(UNKNOWN_MODEL_STDERR), 0)
            .unwrap()
            .words(),
        FELL_BACK
    );
    assert_eq!(
        folded.fold.ended(None, &seen(""), 0).unwrap().words(),
        UNCONFIRMED
    );
}

/// The first row of `stdout` whose `type` is `kind`, with its newline.
fn row_of(stdout: &str, kind: &str) -> String {
    let row = stdout.lines().find(|line| {
        let row: Value = serde_json::from_str(line).unwrap();
        row["type"] == kind
    });
    format!("{}\n", row.unwrap())
}

/// Init's row of the tool-less turn, which names `Fast`.
fn init() -> String {
    row_of(TOOL_LESS, "system")
}

/// The tool-less turn's init, assistant and result rows, which name
/// `Fast`: a whole Qoder stream for a test outside this module (#1199's
/// all-adapter stderr sentinel).
pub(in crate::adapters) fn tool_less_rows() -> String {
    [
        init(),
        row_of(TOOL_LESS, "assistant"),
        row_of(TOOL_LESS, "result"),
    ]
    .concat()
}

/// The words of a refused second init row (Y1).
const SECOND_INIT: &str =
    "model_unconfirmed: Qoder sent a second init row, which no measured Qoder stream carries";

/// Y1: a second init row refuses the seat whatever it names, the pin
/// included, and nothing after the first init is folded.
#[test]
fn a_second_init_row_refuses_the_seat_whatever_it_names() {
    for second in [nameless_init(), init()] {
        let stdout = format!("{}{second}{}", init(), row_of(TOOL_LESS, "result"));
        let folded = fold(&stdout, "Fast");
        assert_eq!(folded.outcome.unwrap_err().words(), SECOND_INIT);
        assert_eq!(folded.rows, vec![none_transcript()]);
        assert!(!folded.fold.worked());
    }
}

/// Y1 and Y3: a first init naming no model confirms nothing, so the first
/// assistant, user or result row stops the stream, as substituted where
/// the fallback line is on stderr by then, and unconfirmed where not.
#[test]
fn a_nameless_first_init_stops_at_the_first_turn_row_as_stderr_says() {
    let rows = [
        row_of(TOOL_LESS, "assistant"),
        row_of(TOOLS_TURN, "user"),
        row_of(TOOL_LESS, "result"),
    ];
    for row in rows {
        let stdout = format!("{}{row}", nameless_init());
        let unconfirmed = fold(&stdout, "Fast").outcome.unwrap_err();
        assert_eq!(unconfirmed.words(), UNCONFIRMED, "{row}");
        let fell_back = fold_seeing(&stdout, "Fast", UNKNOWN_MODEL_STDERR);
        assert_eq!(fell_back.outcome.unwrap_err().words(), FELL_BACK, "{row}");
    }
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
        confirmed.ended(None, &seen(line), 0).unwrap().words(),
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
            pin: pin("Qwen4-Max"),
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

/// The file [`fake_qoder`] records when its CLI was not seen to exit.
const UNREAPED: &str = "unreaped";

/// Fails the run in `dir` whose [`fake_qoder`] never saw its CLI exit, so
/// no run passes on a stream the fake gave up printing (J1).
#[cfg(unix)]
fn reaped(dir: &std::path::Path) {
    let unreaped = dir.join(UNREAPED).exists();
    assert!(
        !unreaped,
        "fake qodercli: its CLI was not seen to exit within 20 seconds"
    );
}

/// A fake `qodercli` under `dir`: it records its argv, reads the prompt,
/// prints `stderr` as recorded, writes `result` when one is given, and
/// exits `code`; only once it has exited, a child of its own records
/// `exited` and prints `stdout`. So a stream stopped at any row was
/// stopped after the CLI's own exit and its result, which the stop leaves
/// as they were (H3). The child waits for that exit 20 seconds at most;
/// past them it records [`UNREAPED`] and prints nothing, so no run waits
/// on it without bound and every run helper fails on the marker (J1).
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
    // The MCP file the launch names is copied while the turn runs. The
    // shell is a zombie, unreaped, from its exit until the driver waits.
    let body = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{dir}/argv'\ncat > '{dir}/prompt'\n\
         prev=; for a in \"$@\"; do [ \"$prev\" = --mcp-config ] && cat \"$a\" > '{dir}/mcp'; \
         prev=$a; done\n\
         cat '{dir}/stderr' >&2\n{write}\
         (end=$(($(date +%s) + 20)); until ps -o stat= -p $$ | grep -q Z; do \
         [ \"$(date +%s)\" -lt \"$end\" ] || {{ : > '{dir}/{UNREAPED}'; exit 1; }}; \
         sleep 0.01; done; : > '{dir}/exited'; cat '{dir}/stdout') &\n\
         exit {code}\n",
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
    let messages = run_input(extra, input, session, &[("BROKKR_QODER_BIN", bin)]);
    reaped(dir);
    messages
}

/// A qoder seat's input in `dir`, its result at `result.json`.
#[cfg(unix)]
fn seat_input(dir: &std::path::Path) -> Value {
    json!({
        "role_path": "/dev/null", "phase": "work", "feature": "f",
        "allowed_results": ["complete"], "result_path": dir.join("result.json"), "workdir": dir,
    })
}

/// [`run`] with `bound` bound to the seat from a store under `dir`, and
/// `vars` set beside the binary.
#[cfg(unix)]
fn run_bound(
    dir: &std::path::Path,
    bin: &std::path::Path,
    extra: &[&str],
    vars: &[(&str, &std::path::Path)],
    bound: &str,
) -> Vec<crate::Body> {
    let store = dir.join("secrets.env");
    secret::store_set(&store, "QT", bound).unwrap();
    let mut input = seat_input(dir);
    input["secrets"] = json!(["QT"]);
    input["secrets_file"] = json!(store);
    let vars = [&[("BROKKR_QODER_BIN", bin)], vars].concat();
    let messages = run_input(extra, input, None, &vars);
    reaped(dir);
    messages
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
        "model_substituted: the seat pinned 'NoSuchModel' and Qoder reported on stderr that \
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
        &super::super::Row::new(&json!({"type": "assistant"}), &[])
    ));
}

/// The launch's typed errors render once: the admission's refusal as every
/// driver states it, and an MCP file that could not be written in fixed
/// words and the I/O error's kind.
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
    // Fixed words and the error's kind, never its text or its path (Y2).
    assert_eq!(unwritten.to_string(), UNSTAGED);
}

/// The staging error's words for a temporary root that does not exist.
const UNSTAGED: &str = "refusing to invoke the agent CLI: the engine's MCP file for the qoder \
                        seat was not written: entity not found";

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
const FELL_BACK: &str = "model_substituted: the seat pinned 'Fast' and Qoder reported on \
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
    let turn = Turn {
        pin: "Fast",
        stdout: stdout.as_bytes(),
        stderr,
        delivered,
        code,
    };
    driven(&turn, bound)
}

/// One fake `qodercli` turn: the pin its seat writes, what it prints and
/// exits with, and whether it writes its result.
#[cfg(unix)]
struct Turn<'a> {
    pin: &'a str,
    stdout: &'a [u8],
    stderr: &'a str,
    delivered: bool,
    code: i32,
}

/// `turn` run by [`drive`] with `bound` bound; beside it, `bound` as masked.
#[cfg(unix)]
fn driven(turn: &Turn<'_>, bound: &str) -> (Result<Invocation, String>, String) {
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let written = turn.delivered.then_some(result.as_path());
    let bin = fake_qoder(dir.path(), turn.stdout, turn.stderr, written, turn.code);
    let input = json!({"workdir": dir.path(), "result_path": result});
    let (bindings, masked) = bound_as(bound);
    let bin = bin.to_string_lossy();
    let extra = argv(&["-m", turn.pin]);
    let turn = drive(
        &bin,
        &extra,
        false,
        "prompt",
        &input,
        &bindings,
        &mut |_| {},
    );
    reaped(dir.path());
    (turn, masked)
}

/// `bound` bound to the seat, and as its mask writes it.
fn bound_as(bound: &str) -> ([secret::BoundSecret; 1], String) {
    let bindings = [super::super::tests::binding("QT", bound)];
    let masked = secret::mask_projected(bound, &bindings);
    assert_ne!(masked, bound);
    (bindings, masked)
}

/// A bound value the mask's own replacement text carries, so words masked
/// twice differ from words masked once (Y4).
const ONCE: &str = "secret";

/// Y4: [`ONCE`] binds "once": masked a second time, its words change.
#[test]
fn a_value_masked_twice_differs_from_one_masked_once() {
    let (bindings, masked) = bound_as(ONCE);
    assert_eq!(masked, "[secret:QT]");
    assert_eq!(
        secret::mask_projected(&masked, &bindings),
        "[[secret:QT]:QT]"
    );
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

/// V4 and Y4: a bound value in an option the seat may not write is masked
/// once in `drive`'s error, which keeps the rest of its words, and in the
/// error `lost::invocation_failed` sends for it, since nothing was spawned.
#[cfg(unix)]
#[test]
fn a_bound_value_in_an_unadmitted_option_is_masked() {
    let dir = tempfile::tempdir().unwrap();
    let input = json!({"workdir": dir.path(), "result_path": dir.path().join("r.json")});
    let (bindings, masked) = bound_as(ONCE);
    let extra = argv(&["-m", "Fast", "--secret"]);
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
    let messages = run_bound(dir.path(), &bin, &["-m", "Fast", "--secret"], &[], ONCE);
    let sent = (&crate::ResultStatus::Failed, Some(unadmitted));
    assert_eq!(error_of(&messages), Some(sent));
    assert!(!dir.path().join("argv").exists());
}

/// Set in the child [`what_a_seat_prints_or_sends_carries_no_bound_value`]
/// runs itself in, so the child's environment is its own.
const PRINTED_CASE: &str = "BROKKR_QODER_PRINTED_CASE";

/// Values bound to a seat whose temporary directory names them, each
/// escaped by a `Debug` or JSON rendering (Y2).
const STAGED: [&str; 3] = ["tok\"9z", "tok\\9z", "tok\t9z"];

/// What the child of [`what_a_seat_prints_or_sends_carries_no_bound_value`]
/// runs: four late errors of split secrets behind a folded turn or a
/// delivered result, one of a pin bound to [`ONCE`] behind a fallback line
/// splitting it, and a staging error for each of [`STAGED`].
#[cfg(unix)]
fn printed_cases() {
    let late = |stdout: &str, stderr: &str, delivered: bool, pin: &str, bound: &str| {
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let written = delivered.then_some(result.as_path());
        let bin = fake_qoder(dir.path(), stdout, stderr, written, 0);
        let lost = run_bound(dir.path(), &bin, &["-m", pin], &[], bound);
        assert_eq!(error_of(&lost), None, "{lost:?}");
    };
    let split_once = "Model \"sec\u{1b}[0mret\" is not available right now; using auto.\n";
    late("", split_once, true, ONCE, ONCE);
    for (bound, stderr) in [(SECRET, FALLBACK_SPLIT), (SPACED, FALLBACK_TABBED)] {
        late(TOOL_LESS, stderr, false, "Fast", bound);
        late("", stderr, true, "Fast", bound);
    }
    // Z3 and Z5: a bad line, or a second init, after a real assistant turn,
    // the result delivered and the CLI exiting 0.
    for stdout in [high_one(), second_init_after_work()] {
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let bin = fake_qoder(dir.path(), stdout, "", Some(&result), 0);
        let lost = run_bound(dir.path(), &bin, &["-m", "Fast"], &[], SECRET);
        assert_eq!(error_of(&lost), None, "{lost:?}");
        let turned = lost.iter().any(|body| {
            matches!(body, crate::Body::Checkpoint { data, .. } if data["step"] == "seat-turn")
        });
        assert!(turned, "{lost:?}");
    }
    // H3: init, a result row and no assistant turn, the result delivered
    // and the CLI exited 0 on its own, then the line `[1]`. The stop left
    // the file and the exit as they were.
    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    let stdout = format!("{}{}[1]\n", init(), row_of(TOOL_LESS, "result"));
    let bin = fake_qoder(dir.path(), stdout, "", Some(&result), 0);
    let lost = run_bound(dir.path(), &bin, &["-m", "Fast"], &[], SECRET);
    assert_eq!(error_of(&lost), None, "{lost:?}");
    assert!(result.exists() && dir.path().join("exited").exists());
    for bound in STAGED {
        let dir = tempfile::tempdir().unwrap();
        let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
        let unstaged = Path::new("/nonexistent").join(bound);
        let vars = [("TMPDIR", unstaged.as_path())];
        let messages = run_bound(dir.path(), &bin, &["-m", "Fast"], &vars, bound);
        let sent = (&crate::ResultStatus::Failed, Some(UNSTAGED.to_string()));
        assert_eq!(error_of(&messages), Some(sent), "{bound:?}");
        let (debug, json) = (format!("{bound:?}"), Value::from(bound).to_string());
        for form in [bound, &debug[1..debug.len() - 1], &json[1..json.len() - 1]] {
            assert!(!UNSTAGED.contains(form), "{form}");
        }
    }
}

/// V4, Y2 and Y4, captured: run in a child of its own, seats whose Qoder
/// fell back behind a folded turn or a delivered result end lost, and what
/// `lost::invocation_failed` prints is each late error with its bound
/// value masked exactly once; seats whose temporary directory names a
/// bound value are sent the staging error in fixed words. The parent reads
/// the child's stderr.
#[cfg(unix)]
#[test]
fn what_a_seat_prints_or_sends_carries_no_bound_value() {
    if std::env::var_os(PRINTED_CASE).is_some() {
        printed_cases();
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
    let lost = "; the command was spawned, so no result is sent\n";
    // Masked twice, the pin would read "[[secret:QT]:QT]"; unmasked, "secret".
    let once = FELL_BACK.replace("'Fast'", "'[secret:QT]'") + lost;
    assert_eq!(printed.matches(&once).count(), 1, "{printed}");
    let split = FELL_BACK.to_string() + lost;
    assert_eq!(printed.matches(&split).count(), 4, "{printed}");
    // Z5: the operator reads why, never `agent CLI exited 1`. H3's stop
    // after a delivered result is the second bad line printed (H4).
    for (words, count) in [(UNMEASURED, 2), (SECOND_INIT, 1)] {
        let late = words.to_string() + lost;
        assert_eq!(printed.matches(&late).count(), count, "{printed}");
    }
    assert!(!said.contains("agent CLI exited"), "{said}");
    for value in [SECRET, SPACED].into_iter().chain(STAGED) {
        assert!(!said.contains(value), "{value:?}: {said}");
    }
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
    let words = FELL_BACK.replace("'Fast'", &format!("'{masked}'"));
    assert_eq!(late.map(drop), Err(words));
}

/// The refusal a driven turn ends with, where it ends with one.
#[cfg(unix)]
fn refusal_words(turn: Result<Invocation, String>) -> Option<String> {
    let invocation = turn.unwrap();
    invocation
        .refusal
        .as_ref()
        .map(|refused| refused.words().to_string())
}

/// Y1, driven: after an init naming the pin, a second init row, nameless
/// or naming the pin again, refuses the seat. H3 moves this golden: where
/// the CLI exited 0 having delivered its result, the seat ends lost with
/// the words as its error, and is refused to start only where it did not.
#[cfg(unix)]
#[test]
fn a_driven_second_init_row_is_refused_or_ends_lost_behind_a_delivered_result() {
    for second in [nameless_init(), init()] {
        let stdout = format!("{}{second}{}", init(), row_of(TOOL_LESS, "result"));
        let (turn, _) = invoked(&stdout, "", true, 0, SECRET);
        assert_eq!(turn.map(drop), Err(SECOND_INIT.to_string()));
        let (turn, _) = invoked(&stdout, "", false, 0, SECRET);
        assert_eq!(refusal_words(turn).as_deref(), Some(SECOND_INIT));
    }
}

/// Y3, driven: a fallback notice, a nameless init and a result row are
/// refused as substituted in the reported-on-stderr words, and the same
/// stream with no notice as unconfirmed.
#[cfg(unix)]
#[test]
fn a_fallback_notice_behind_a_nameless_init_is_a_substitution_at_the_result_row() {
    let stdout = format!("{}{}", nameless_init(), row_of(TOOL_LESS, "result"));
    let (turn, _) = invoked(&stdout, UNKNOWN_MODEL_STDERR, false, 0, SECRET);
    assert_eq!(refusal_words(turn).as_deref(), Some(FELL_BACK));
    let (turn, _) = invoked(&stdout, "", false, 0, SECRET);
    assert_eq!(refusal_words(turn).as_deref(), Some(UNCONFIRMED));
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

/// The words of a pin that is not a plain id (Z1).
const UNPLAIN: &str = "refusing to invoke the agent CLI: the qoder model pin is not a plain \
                       model id (an ASCII letter or digit, then ASCII letters, digits, '.', \
                       '_' and '-')";

/// Pins that are not plain ids: a quote, a backslash, a tab, ESC, a space,
/// `[`, `:`, a non-ASCII letter, and 81 characters.
fn unplain_pins() -> Vec<String> {
    let marked = [
        "tok\"9z",
        "tok\\9z",
        "tok\t9z",
        "tok\u{1b}9z",
        "tok 9z",
        "tok[9z",
        "tok:9z",
    ];
    let mut pins: Vec<String> = marked.iter().map(|pin| pin.to_string()).collect();
    pins.extend(["tokä9z".to_string(), "a".repeat(81)]);
    pins
}

/// Every id the example adapter maps, sorted.
fn mapped_ids() -> Vec<String> {
    let example: Value = serde_json::from_str(include_str!(
        "../../../../../docs/examples/adapters/qoder.json"
    ))
    .unwrap();
    let models = example["models"].as_object().unwrap().values();
    let mut ids: Vec<String> = models
        .map(|model| model["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

/// Z1: every id the example adapter maps, and one of 80 characters,
/// launches on its pin; a pin that is not a plain id is refused at launch.
#[test]
fn every_mapped_id_launches_and_a_pin_that_is_no_plain_id_is_refused() {
    let mut ids = mapped_ids();
    assert_eq!(
        ids,
        [
            "DeepSeek-V4-Pro",
            "Fast",
            "GLM-5.3",
            "Kimi-K3.1-Pro",
            "Qwen4-Max"
        ]
    );
    ids.extend(["a".repeat(80), "a_b".to_string()]);
    for id in ids {
        let launched = launch("qodercli", &argv(&["-m", &id]), EMPTY).unwrap();
        assert_eq!(launched.pin.to_string(), id);
    }
    for value in unplain_pins() {
        let refused = launch("qodercli", &argv(&["-m", &value]), EMPTY).unwrap_err();
        assert!(matches!(refused, LaunchError::Unplain), "{value}");
    }
    assert_eq!(LaunchError::Unplain.to_string(), UNPLAIN);
}

/// Z1, driven: a pin that is not a plain id never starts its harness, and
/// the seat is sent the fixed words, with no byte of the value.
#[cfg(unix)]
#[test]
fn a_pin_that_is_no_plain_id_never_starts_its_harness() {
    for value in unplain_pins() {
        let dir = tempfile::tempdir().unwrap();
        let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
        let messages = run(dir.path(), &bin, &["-m", &value]);
        let sent = (&crate::ResultStatus::Failed, Some(UNPLAIN.to_string()));
        assert_eq!(error_of(&messages), Some(sent), "{value}");
        assert!(!dir.path().join("argv").exists(), "{value}");
    }
}

/// Values bound inside the pin `Qwen4-Max` and equal to it. The store binds
/// no value under four bytes, so the value inside is `4-Max`.
const OVERLAPS: [&str; 2] = ["4-Max", "Qwen4-Max"];

/// Z2 and H1: a binding inside the pin, or equal to it, masks what the
/// Claude fold reads and not what Qoder's rules decide on: init naming the
/// pin confirms it. H1 moves this golden: the fold reads the pin masked,
/// which is no model token, so no row and no session record names a model.
#[test]
fn a_binding_inside_or_equal_to_the_pin_still_confirms_it_and_masks_what_leaves() {
    for bound in OVERLAPS {
        let bindings = [super::super::tests::binding("QT", bound)];
        let folded = fold_bound(NAMED_MODEL.as_bytes(), "Qwen4-Max", "", &bindings);
        assert_eq!(folded.outcome, Ok(None), "{bound}");
        let unreported = super::super::MODEL_NOT_REPORTED;
        assert_eq!(
            shown(&folded.rows),
            vec![turn(1, unreported), turn(1, unreported), none_transcript()]
        );
        assert_eq!(folded.session_meta.get("model"), None);
    }
}

/// Z2 and H1, driven: the same seats finish on the wire with no
/// checkpoint naming the pin or any form of it.
#[cfg(unix)]
#[test]
fn a_seat_whose_binding_overlaps_its_pin_finishes_naming_no_model() {
    for bound in OVERLAPS {
        let dir = tempfile::tempdir().unwrap();
        let result = dir.path().join("result.json");
        let bin = fake_qoder(dir.path(), NAMED_MODEL, "", Some(&result), 0);
        let messages = run_bound(dir.path(), &bin, &["-m", "Qwen4-Max"], &[], bound);
        let (status, refusal) = result_of(&messages).unwrap();
        assert_eq!((status, refusal), (&crate::ResultStatus::Succeeded, None));
        let steps = ["seat-turn", "qoder-cli-session-finished"];
        let models: Vec<(&Value, &Value)> = messages
            .iter()
            .filter_map(|body| match body {
                crate::Body::Checkpoint { data, .. } => Some((&data["step"], &data["model"])),
                _ => None,
            })
            .filter(|(step, _)| steps.iter().any(|named| *step == named))
            .collect();
        let (turned, finished) = (json!(steps[0]), json!(steps[1]));
        // The fold reads no masked id as a model token (H1).
        let unreported = json!(super::super::MODEL_NOT_REPORTED);
        let golden = [
            (&turned, &unreported),
            (&turned, &unreported),
            (&finished, &unreported),
        ];
        assert_eq!(models, golden, "{bound}");
    }
}

/// Round six's HIGH 1 stream: init naming `Fast`, an assistant row, a line
/// that is not UTF-8, a nameless init and a result.
fn high_one() -> Vec<u8> {
    let (init, nameless) = (init(), nameless_init());
    let assistant = row_of(TOOL_LESS, "assistant");
    let result = row_of(TOOL_LESS, "result");
    let rows: [&[u8]; 5] = [
        init.as_bytes(),
        assistant.as_bytes(),
        b"\xff\xfe\n",
        nameless.as_bytes(),
        result.as_bytes(),
    ];
    rows.concat()
}

/// Init naming `Fast`, an assistant row, init naming `Fast` again, and a
/// result (Z5).
fn second_init_after_work() -> Vec<u8> {
    let init = init();
    let rows = [
        init.clone(),
        row_of(TOOL_LESS, "assistant"),
        init,
        row_of(TOOL_LESS, "result"),
    ];
    rows.concat().into_bytes()
}

/// Z3 and Z5, driven: a bad line or a second init after an assistant turn
/// is no refusal to start, though the result was delivered and the CLI
/// exited 0. `drive` ends with its words as the error, masked once.
#[cfg(unix)]
#[test]
fn a_bad_line_or_a_second_init_after_work_ends_with_its_words() {
    for (stdout, words) in [
        (high_one(), UNMEASURED),
        (second_init_after_work(), SECOND_INIT),
    ] {
        let turn = Turn {
            pin: "Fast",
            stdout: &stdout,
            stderr: "",
            delivered: true,
            code: 0,
        };
        let (late, _) = driven(&turn, SECRET);
        assert_eq!(late.map(drop), Err(words.to_string()));
    }
}

/// Z3, driven: a line that is not JSON before init refuses the seat to
/// start, in the bad-line words, on the wire. H3 moves its fake: it
/// delivers no result, which would end the seat lost.
#[cfg(unix)]
#[test]
fn a_line_that_is_not_json_before_init_is_a_refusal_to_start() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = format!("not json\n{TOOL_LESS}");
    let bin = fake_qoder(dir.path(), stdout, "", None, 0);
    let messages = run(dir.path(), &bin, &["-m", "Fast"]);
    assert!(dir.path().join("exited").exists(), "{messages:?}");
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    assert_eq!(refusal.map(PreSessionRefusal::words), Some(UNMEASURED));
}

/// Z4: a fallback notice, a nameless init and a result refuse a seat whose
/// pin is bound in words that carry the pin masked once and no form of it,
/// as `drive` returns them and as the wire carries them.
#[cfg(unix)]
#[test]
fn a_bound_pin_is_masked_once_in_the_refusal_words() {
    let stdout = format!("{}{}", nameless_init(), row_of(TOOL_LESS, "result"));
    for bound in [SECRET, ONCE] {
        let turn = Turn {
            pin: bound,
            stdout: stdout.as_bytes(),
            stderr: UNKNOWN_MODEL_STDERR,
            delivered: false,
            code: 0,
        };
        let (driven, masked) = driven(&turn, bound);
        let words = FELL_BACK.replace("'Fast'", &format!("'{masked}'"));
        assert_eq!(masked, "[secret:QT]");
        assert_eq!(refusal_words(driven), Some(words.clone()), "{bound}");
    }
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), stdout, UNKNOWN_MODEL_STDERR, None, 0);
    let messages = run_bound(dir.path(), &bin, &["-m", SECRET], &[], SECRET);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed);
    let words = FELL_BACK.replace("'Fast'", "'[secret:QT]'");
    assert_eq!(refusal.map(PreSessionRefusal::words), Some(words.as_str()));
}

/// Z4: an init serving a model that is not a plain id is refused as a
/// substitution in fixed words, with no form of the value.
#[test]
fn a_served_model_that_is_no_plain_id_is_named_in_fixed_words() {
    for served in ["tok\"9z", "tok\u{1b}9z"] {
        let mut init: Value = serde_json::from_str(&init()).unwrap();
        init["model"] = json!(served);
        let folded = fold(&format!("{init}\n"), "Fast");
        assert_eq!(
            folded.outcome.unwrap_err().words(),
            "model_substituted: the seat pinned 'Fast' and Qoder served a model that is not a \
             plain id"
        );
    }
}

/// A seat pinned to `pin` whose CLI prints `stdout` and `stderr` and exits 0
/// with no result, run with `bound` bound: the refusal its one result sends.
#[cfg(unix)]
fn refused_on_the_wire(stdout: &str, stderr: &str, pin: &str, bound: &str) -> PreSessionRefusal {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_qoder(dir.path(), stdout, stderr, None, 0);
    let messages = run_bound(dir.path(), &bin, &["-m", pin], &[], bound);
    let (status, refusal) = result_of(&messages).unwrap();
    assert_eq!(status, &crate::ResultStatus::Failed, "{messages:?}");
    refusal.unwrap().clone()
}

/// A provider refusal of class `other`, and of class `rate_limit` (H2).
const PROVIDER_REFUSED: &str = "api_error: Qoder's provider refused the turn";
const RATE_LIMITED: &str = "rate_limit: Qoder's provider refused the turn at its rate limit";

/// H1 and H2: round seven's two leaks in full. A provider's error result
/// quoting a bound value past the classifier's clip, or split by a tab it
/// collapses, is named by its class in fixed words, with no three
/// consecutive characters of the value.
#[cfg(unix)]
#[test]
fn a_provider_refusal_quoting_a_bound_value_is_named_by_its_class_alone() {
    let clipped = format!("{}abcdefghij tail", "x".repeat(155));
    let cases = [
        ("abcdefghij", clipped.as_str()),
        ("ab\tcd", "bad key ab\tcd ef"),
    ];
    for (bound, text) in cases {
        let row = json!({"type": "result", "is_error": true, "result": text});
        let refused = refused_on_the_wire(&format!("{}{row}\n", init()), "", "Fast", bound);
        assert_eq!(refused.words(), PROVIDER_REFUSED, "{bound:?}");
        let value: Vec<char> = bound.chars().collect();
        for triple in value.windows(3).map(String::from_iter) {
            assert!(!refused.words().contains(&triple), "{triple:?}");
        }
    }
}

/// H2: a rate limit event and an assistant error text are named by their
/// class alone; the status and retry-after the row gave stay as fields.
#[cfg(unix)]
#[test]
fn a_rate_limit_or_an_assistant_error_is_named_by_its_class_with_its_numbers() {
    let resets = super::super::refusal::epoch_now().unwrap() + 3600;
    let limited = json!({
        "type": "rate_limit_event", "apiErrorStatus": 429, "message": "slow down, tok9z",
        "rate_limit_info": {"resetsAt": resets},
    });
    let refused = refused_on_the_wire(&format!("{}{limited}\n", init()), "", "Fast", SECRET);
    let typed = (refused.words(), refused.class(), refused.status());
    assert_eq!(typed, (RATE_LIMITED, RefusalClass::RateLimit, Some(429)));
    let waited = refused.retry_after_seconds().unwrap();
    assert!((3590..=3600).contains(&waited), "{waited}");
    let errored = json!({
        "type": "assistant", "isApiErrorMessage": true, "apiErrorStatus": 401,
        "error": "authentication_failed",
        "message": {"id": "m-1", "content": [{"type": "text", "text": "Invalid key tok9z"}]},
    });
    let refused = refused_on_the_wire(&format!("{}{errored}\n", init()), "", "Fast", SECRET);
    let typed = (refused.words(), refused.class(), refused.status());
    assert_eq!(typed, (PROVIDER_REFUSED, RefusalClass::Other, Some(401)));
    assert_eq!(refused.retry_after_seconds(), None);
}

/// H2: a substitution names an 80-character pin and an 80-character served
/// id whole: no clipping step reads Qoder's own words.
#[cfg(unix)]
#[test]
fn a_substitution_names_an_eighty_character_pin_and_served_id_whole() {
    let (pin, served) = ("a".repeat(80), "b".repeat(80));
    let mut row: Value = serde_json::from_str(&init()).unwrap();
    row["model"] = json!(served);
    let refused = refused_on_the_wire(&format!("{row}\n"), "", &pin, SECRET);
    let words = format!("model_substituted: the seat pinned '{pin}' and Qoder served '{served}'");
    assert_eq!(refused.words(), words);
}

/// H5: with Qoder's fallback line on stderr, a second init and a bad line
/// each refuse the seat as substituted, in the reported-on-stderr words.
#[cfg(unix)]
#[test]
fn a_fallback_notice_wins_a_second_init_and_a_bad_line() {
    for stdout in [format!("{}{}", init(), init()), format!("{}[1]\n", init())] {
        let refused = refused_on_the_wire(&stdout, UNKNOWN_MODEL_STDERR, "Fast", SECRET);
        assert_eq!(refused.words(), FELL_BACK, "{stdout}");
    }
}

/// H6: a pin led by `_` or `.` is refused at launch in the plain-id words,
/// and one led by `-` as valueless, each before its harness starts.
#[cfg(unix)]
#[test]
fn a_pin_led_by_an_underscore_a_dot_or_a_dash_never_starts_its_harness() {
    let valueless = LaunchError::Valueless("-m").to_string();
    for (pin, words) in [("_x", UNPLAIN), (".x", UNPLAIN), ("-x", &valueless)] {
        let dir = tempfile::tempdir().unwrap();
        let bin = fake_qoder(dir.path(), TOOL_LESS, "", None, 0);
        let messages = run(dir.path(), &bin, &["-m", pin]);
        let sent = (&crate::ResultStatus::Failed, Some(words.to_string()));
        assert_eq!(error_of(&messages), Some(sent), "{pin}");
        assert!(!dir.path().join("argv").exists(), "{pin}");
    }
}
