//! Antigravity CLI 1.5's recorded turns (`docs/evidence/adapters/
//! agy-cli-1-5/`) folded byte for byte, the launch the driver composes, the
//! engine's HOME, the workspace refusal, and seats run end to end over a
//! fake `agy` (#1200).

use std::path::Path;

use serde_json::{json, Map, Value};

use super::*;
use crate::adapters::AdapterKind;
use crate::{PreSessionRefusal, RefusalClass};
use fold::Failure;

macro_rules! recorded {
    ($file:literal) => {
        include_str!(concat!(
            "../../../../../docs/evidence/adapters/agy-cli-1-5/",
            $file
        ))
    };
}

pub(in crate::adapters) const TOOL_LESS: &str = recorded!("tool-less-turn.ndjson");
const STDIN_TURN: &str = recorded!("stdin-stream-json.ndjson");
const NEON: &str = recorded!("named-model-neon-base.ndjson");
const CARBON: &str = recorded!("named-model-carbon-base.ndjson");
const HYDROGEN: &str = recorded!("named-model-hydrogen-base.ndjson");
const ARGON: &str = recorded!("named-model-argon-base.ndjson");
const CLAUDE_SONNET_FIRST: &str = recorded!("claude-sonnet-first.ndjson");
const CLAUDE_SONNET_SUFFIX: &str = recorded!("claude-sonnet-suffix.ndjson");
const CLAUDE_SONNET_LOW: &str = recorded!("claude-sonnet-effort-low.ndjson");
const CLAUDE_SONNET_MEDIUM: &str = recorded!("claude-sonnet-effort-medium.ndjson");
const CLAUDE_SONNET_HIGH: &str = recorded!("claude-sonnet-effort-high.ndjson");
const CLAUDE_OPUS_NOEFFORT: &str = recorded!("claude-opus-noeffort.ndjson");
const CLAUDE_OPUS_LOW: &str = recorded!("claude-opus-effort-low.ndjson");
const CLAUDE_OPUS_MEDIUM: &str = recorded!("claude-opus-effort-medium.ndjson");
const CLAUDE_OPUS_HIGH: &str = recorded!("claude-opus-effort-high.ndjson");
pub(in crate::adapters) const TOOLS_TURN: &str = recorded!("tools-turn.ndjson");
const GATE_PRIOR: &str = recorded!("gate-prior-grant.ndjson");
const DENIED_DEFAULT: &str = recorded!("denied-command-default.ndjson");
const DENIED_ACCEPT: &str = recorded!("denied-command-accept-edits.ndjson");
const ISO_08: &str = recorded!("iso-08-engine-carriage-no-rule.ndjson");
const SIGNED_OUT: &str = recorded!("signed-out.ndjson");
const SIGNED_OUT_STDERR: &str = recorded!("signed-out.stderr");
const PRINT_TIMEOUT: &str = recorded!("print-timeout.ndjson");
const PRINT_TIMEOUT_STDERR: &str = recorded!("print-timeout.stderr");
const BAD_FLAG_STDERR: &str = recorded!("bad-flag.stderr");

/// The slug every recorded turn but the named-model ones pinned.
pub(in crate::adapters) const FLASH: &str = "gemini-3.8-flash-low";

/// The launch refusals the four model and effort turns end with: each an
/// `ERROR` result before any turn, exit 1.
const MODEL_REFUSALS: [(&str, &str); 4] = [
    (
        recorded!("unknown-model.ndjson"),
        recorded!("unknown-model.stderr"),
    ),
    (
        recorded!("bad-effort.ndjson"),
        recorded!("bad-effort.stderr"),
    ),
    (
        recorded!("effort-unsupported-for-model.ndjson"),
        recorded!("effort-unsupported-for-model.stderr"),
    ),
    (
        recorded!("effort-conflicts-with-slug.ndjson"),
        recorded!("effort-conflicts-with-slug.stderr"),
    ),
];

/// The five Claude effort-refusal turns from the eighth measurement (#1382):
/// `xhigh` and `max` on both slugs, plus an unknown effort, each exiting 1
/// with an `ERROR` result before any turn.
const CLAUDE_EFFORT_REFUSALS: [(&str, &str, &str); 5] = [
    (
        "claude-sonnet-5-5",
        recorded!("claude-sonnet-effort-xhigh.ndjson"),
        recorded!("claude-sonnet-effort-xhigh.stderr"),
    ),
    (
        "claude-sonnet-5-5",
        recorded!("claude-sonnet-effort-max.ndjson"),
        recorded!("claude-sonnet-effort-max.stderr"),
    ),
    (
        "claude-sonnet-5-5",
        recorded!("claude-sonnet-effort-bogus.ndjson"),
        recorded!("claude-sonnet-effort-bogus.stderr"),
    ),
    (
        "claude-opus-5-5",
        recorded!("claude-opus-effort-xhigh.ndjson"),
        recorded!("claude-opus-effort-xhigh.stderr"),
    ),
    (
        "claude-opus-5-5",
        recorded!("claude-opus-effort-max.ndjson"),
        recorded!("claude-opus-effort-max.stderr"),
    ),
];

/// One recorded stdout read through the fold under `pin`, as `invoke`
/// reads it: what it emitted, the record it left, and the refusal it
/// stopped at, if any.
struct Read {
    fold: AgyFold<'static>,
    rows: Vec<Value>,
    record: Map<String, Value>,
    stopped: Option<PreSessionRefusal>,
}

fn cold_hold() -> LaunchHold {
    LaunchHold::new(
        "agy",
        super::super::LaunchPlan::cold(Vec::new(), "agy-conversation", None),
    )
}

fn read(stdout: impl AsRef<[u8]>, pin: &str) -> Read {
    let mut fold = AgyFold::new(pin, &[]);
    let mut hold = cold_hold();
    let (mut rows, mut record, mut stopped) = (Vec::new(), Map::new(), None);
    for line in stdout.as_ref().split(|byte| *byte == b'\n') {
        let mut emitted = |row: &super::super::Emitted| rows.push(row.row.clone());
        if let crate::adapters::stream_turn::Line::Stopped(refused) =
            fold.read(line, &mut record, &mut hold, &mut emitted)
        {
            stopped = Some(refused);
            break;
        }
    }
    Read {
        fold,
        rows,
        record,
        stopped,
    }
}

/// The one turn row a recorded answer's model response folds to.
pub(in crate::adapters) fn turn_row(turn: u64, pin: &str) -> Value {
    json!({"step": "seat-turn", "turn": turn, "model": pin, "effort": "not reported"})
}

/// The answer a fold's verdict accepted.
fn answer(verdict: Verdict) -> String {
    match verdict {
        Verdict::Answer(response) => response,
        other => panic!("no answer: {other:?}"),
    }
}

/// The `response` of the last row of `stdout`, as the record holds it.
fn response(stdout: &str) -> Value {
    let last: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
    last["result"]["response"].clone()
}

/// Each `(stdout, pin)` in `turns` finishes cleanly with `expected`, one turn
/// row on `pin`, and `pin` recorded as its model.
fn assert_answers(turns: &[(&str, &str)], expected: &str) {
    for &(stdout, pin) in turns {
        let read = read(stdout, pin);
        assert_eq!(read.stopped, None, "{pin}");
        assert_eq!(answer(read.fold.verdict(0, false)), expected, "{pin}");
        assert_eq!(read.rows, [turn_row(1, pin)], "{pin}");
        assert_eq!(read.record["model"], pin);
    }
}

/// Each answer is the one the fold's verdict carries (J7).
#[test]
fn the_tool_less_stdin_and_named_model_turns_answer_ready_on_their_pins() {
    assert_answers(
        &[
            (TOOL_LESS, FLASH),
            (STDIN_TURN, FLASH),
            (NEON, "gemini-4-neon"),
            (CARBON, "gemini-4-carbon"),
            (HYDROGEN, "gemini-4-hydrogen"),
            (ARGON, "gemini-4-argon"),
        ],
        "READY\n",
    );
}

/// The tools turn and the gate's own turn answer; the gate's refused
/// command and refused search were tool errors the turn went on past.
#[test]
fn the_tools_and_gate_turns_answer_past_the_calls_the_gate_refused() {
    let answers = [
        (TOOLS_TURN, 2, "DONE\n"),
        (GATE_PRIOR, 5, "(1) REFUSED\n(2) OK\n(3) OK\n(4) REFUSED\n"),
    ];
    for (stdout, turns, answered) in answers {
        let read = read(stdout, FLASH);
        assert_eq!(answer(read.fold.verdict(0, false)), answered);
        let rows: Vec<Value> = (1..=turns).map(|turn| turn_row(turn, FLASH)).collect();
        assert_eq!(read.rows, rows);
    }
    for refused in ["run_command", "search_web"] {
        let step =
            format!("\"state\":\"ERROR\",\"step_type\":\"tool\",\"tool_name\":\"{refused}\"");
        assert!(GATE_PRIOR.contains(&step), "{refused}");
    }
    assert!(!GATE_PRIOR.contains("denied_actions"));
}

/// `failure` as a verdict, with the status a result named and whether agy
/// timed out behind it.
fn failed(failure: Failure, status: Option<&str>, timed_out: bool) -> Verdict {
    let seen = fold::Seen {
        status: status.map(str::to_string),
        timed_out,
    };
    Verdict::Failed(Failed { failure, seen })
}

/// A turn agy stopped at a denial fails, naming each denied action, as
/// the result says it and whatever the denied step's state read, and
/// whatever the CLI exited with: its result's status beside it, and the
/// print timeout behind it.
#[test]
fn a_turn_stopped_at_a_denial_is_a_failure_naming_each_action() {
    let failure = |actions: &[&str], timed_out| {
        failed(Failure::Denied(argv(actions)), Some("SUCCESS"), timed_out)
    };
    for (stdout, action) in [
        (DENIED_DEFAULT, "command"),
        (DENIED_ACCEPT, "command"),
        (ISO_08, "mcp"),
    ] {
        let read = read(stdout, FLASH);
        assert_eq!(read.stopped, None);
        for exit in [0, 1, 3] {
            assert_eq!(
                read.fold.verdict(exit, false),
                failure(&[action], exit == 3),
                "{action} {exit}"
            );
        }
        assert_eq!(response(stdout), "");
    }
    // The denied step read ERROR once and DONE the other time.
    let denied = "\"state\":\"ERROR\",\"step_type\":\"tool\",\"tool_name\":\"run_command\"";
    assert!(DENIED_DEFAULT.contains(denied));
    assert!(!DENIED_ACCEPT.contains("\"state\":\"ERROR\""));
    // An action is named escaped, on one line (K3).
    let odd = r#"{"event":"result","result":{"status":"SUCCESS","response":"x","denied_actions":[{"action":"rm -rf /\n"},{"action":"command"}]}}"#;
    let read = read(
        format!("{}\n{odd}", TOOL_LESS.lines().next().unwrap()),
        FLASH,
    );
    assert_eq!(
        read.fold.verdict(0, false),
        failure(&["rm -rf /\\n", "command"], false)
    );
}

#[test]
fn a_named_model_turn_pinned_to_another_id_is_refused_naming_both() {
    let read = read(NEON, "gemini-4-carbon");
    let refused = read.stopped.unwrap();
    assert_eq!(refused.class(), RefusalClass::Other);
    assert_eq!(
        refused.words(),
        "model_substituted: the seat pinned \"gemini-4-carbon\" and agy served \"gemini-4-neon\""
    );
    assert_eq!(read.rows, Vec::<Value>::new());
    assert!(!read.fold.worked());
}

/// The Claude 5.5 turns from the eighth measurement (#1382) answer on their
/// pinned slugs across no-effort, `low`, `medium`, `high` and a suffixed slug,
/// and a pin mismatch on either slug is refused naming both.
#[test]
fn the_claude_turns_answer_on_their_pins_and_substitution_is_refused() {
    assert_answers(
        &[
            (CLAUDE_SONNET_FIRST, "claude-sonnet-5-5"),
            (CLAUDE_SONNET_LOW, "claude-sonnet-5-5"),
            (CLAUDE_SONNET_MEDIUM, "claude-sonnet-5-5"),
            (CLAUDE_SONNET_HIGH, "claude-sonnet-5-5"),
            (CLAUDE_SONNET_SUFFIX, "claude-sonnet-5-5-high"),
            (CLAUDE_OPUS_NOEFFORT, "claude-opus-5-5"),
            (CLAUDE_OPUS_LOW, "claude-opus-5-5"),
            (CLAUDE_OPUS_MEDIUM, "claude-opus-5-5"),
            (CLAUDE_OPUS_HIGH, "claude-opus-5-5"),
        ],
        "ready\n",
    );
    let mismatches = [
        (CLAUDE_SONNET_FIRST, "claude-opus-5-5", "claude-sonnet-5-5"),
        (CLAUDE_OPUS_LOW, "claude-sonnet-5-5", "claude-opus-5-5"),
        (
            CLAUDE_SONNET_SUFFIX,
            "claude-sonnet-5-5",
            "claude-sonnet-5-5-high",
        ),
    ];
    for (stdout, pin, served) in mismatches {
        let read = read(stdout, pin);
        let refused = read.stopped.unwrap();
        assert_eq!(refused.class(), RefusalClass::Other);
        assert_eq!(
            refused.words(),
            format!("model_substituted: the seat pinned \"{pin}\" and agy served \"{served}\"")
        );
        assert_eq!(read.rows, Vec::<Value>::new());
        assert!(!read.fold.worked());
    }
}

/// `xhigh`, `max` and an unknown effort on the two Claude 5.5 slugs end as
/// launch refusals quoting nothing from stdout or stderr, by the same path as
/// `gemini-4-carbon` (#1382).
#[test]
fn the_claude_effort_refusals_are_launch_refusals_quoting_nothing() {
    let expected = "launch_refused: agy's result had status ERROR; agy exited 1 before any turn: \
                    it does not serve the model or the effort the seat named";
    for (pin, stdout, stderr) in CLAUDE_EFFORT_REFUSALS {
        let Verdict::Refused(refused) = read(stdout, pin).fold.verdict(1, false) else {
            panic!("not a refusal for {pin}");
        };
        assert_eq!(
            (refused.class(), refused.words()),
            (RefusalClass::Other, expected)
        );
        quotes_nothing(refused.words(), stdout, stderr);
    }
}

/// With no init row there is no answer: a step before it refuses, and a
/// clean result or a clean exit it never confirmed is unconfirmed.
#[test]
fn a_stream_with_no_init_row_gives_no_answer() {
    let unconfirmed = "model_unconfirmed: the seat pinned \"gemini-3.8-flash-low\" and no init \
                       row named the model agy served";
    let headless: String = TOOL_LESS.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(read(&headless, FLASH).stopped.unwrap().words(), unconfirmed);
    let result_only = TOOL_LESS.lines().last().unwrap();
    let behind_result = status_first(unconfirmed, "SUCCESS");
    for (stdout, words) in [(result_only, behind_result.as_str()), ("", unconfirmed)] {
        let Verdict::Refused(refused) = read(stdout, FLASH).fold.verdict(0, false) else {
            panic!("{stdout}");
        };
        assert_eq!(refused.words(), words);
    }
    // An init that names no model, and rows the fold does not type, pass.
    let nameless = r#"{"event":"init","init":{"cwd":"/w"}}"#;
    let read = read(format!("{nameless}\n5\n{{\"event\":\"other\"}}"), FLASH);
    assert_eq!(read.stopped, None);
    assert!(read.record.is_empty());
}

/// Each line of `text` long enough to be distinctive: no refusal may hold
/// one.
fn lines_of(text: &str) -> Vec<String> {
    let lines = text.lines().map(str::trim).filter(|line| line.len() > 12);
    lines.map(str::to_string).collect()
}

/// `words` hold no line of `stderr` and no line of the recorded result's
/// `error` text in `stdout`.
fn quotes_nothing(words: &str, stdout: &str, stderr: &str) {
    let error = stdout.lines().last().map_or(String::new(), |last| {
        let last: Value = serde_json::from_str(last).unwrap();
        last["result"]["error"].as_str().unwrap_or("").to_string()
    });
    for line in lines_of(stderr).into_iter().chain(lines_of(&error)) {
        assert!(!words.contains(&line), "{words} quotes {line}");
    }
}

/// The refusal `stdout` and `exit` end a seat pinned to [`FLASH`] with.
fn refusal_of(stdout: &str, exit: i32) -> PreSessionRefusal {
    match read(stdout, FLASH).fold.verdict(exit, false) {
        Verdict::Refused(refused) => refused,
        other => panic!("not a refusal: {other:?}"),
    }
}

#[test]
fn the_model_effort_and_flag_refusals_are_launch_refusals_quoting_nothing() {
    let model = "launch_refused: agy's result had status ERROR; agy exited 1 before any turn: \
                 it does not serve the model or the effort the seat named";
    for (stdout, stderr) in MODEL_REFUSALS {
        let refused = refusal_of(stdout, 1);
        assert_eq!(
            (refused.class(), refused.words()),
            (RefusalClass::Other, model)
        );
        quotes_nothing(refused.words(), stdout, stderr);
    }
    let flag = refusal_of("", 2);
    assert_eq!(
        flag.words(),
        "launch_refused: agy exited 2 on an option it does not define"
    );
    quotes_nothing(flag.words(), "", BAD_FLAG_STDERR);
    // Exit 2 behind a stream is no flag refusal.
    assert_eq!(
        read(TOOL_LESS.lines().next().unwrap(), FLASH)
            .fold
            .verdict(2, false),
        failed(Failure::Exited(2), None, false)
    );
}

#[test]
fn a_print_timeout_and_a_signed_out_cli_are_refusals_quoting_nothing() {
    let timeout = refusal_of(PRINT_TIMEOUT, 3);
    assert_eq!(timeout.words(), status_first(PRINT_TIMEOUT_WORDS, "ERROR"));
    quotes_nothing(timeout.words(), PRINT_TIMEOUT, PRINT_TIMEOUT_STDERR);
    assert_eq!(refusal_of("", 3).words(), PRINT_TIMEOUT_WORDS);
    let signed_out = refusal_of(SIGNED_OUT, 1);
    assert_eq!(signed_out.words(), signed_out_words("ERROR"));
    quotes_nothing(signed_out.words(), SIGNED_OUT, SIGNED_OUT_STDERR);
    // No piece of the sign-in URL the stderr holds reaches the words.
    let url = SIGNED_OUT_STDERR
        .split_whitespace()
        .find(|word| word.starts_with("https://"))
        .unwrap();
    let pieces = url.split(|c: char| !c.is_ascii_alphanumeric());
    for piece in pieces.filter(|piece| piece.len() > 5) {
        assert!(!signed_out.words().contains(piece), "{piece}");
    }
}

/// The signed-out refusal's fixed words, naming how to sign in.
const SIGNED_OUT_WORDS: &str = "not_logged_in: Antigravity CLI is signed out: run `agy` once \
                                and sign in; the login is \
                                ~/.gemini/antigravity-cli/antigravity-oauth-token";

/// `words`, a refusal's, with a result of `status` named first (K5, L5).
fn status_first(words: &str, status: &str) -> String {
    let (token, rest) = words.split_once(": ").unwrap();
    format!("{token}: agy's result had status {status}; {rest}")
}

/// [`SIGNED_OUT_WORDS`] behind a result of `status`.
fn signed_out_words(status: &str) -> String {
    status_first(SIGNED_OUT_WORDS, status)
}

/// The failure of a pinned `SUCCESS` whose response is empty or blank: the
/// status first, through [`fold::Seen`], and then the empty response (M4).
const EMPTY_WORDS: &str = "agy's result had status SUCCESS; agy's response was empty, which is \
                           not an answer (#1200)";

/// The print-timeout refusal's fixed words.
const PRINT_TIMEOUT_WORDS: &str =
    "print_timeout: agy exited 3 at its own print timeout, which this launch never asks for";

/// A clean exit that is still no answer fails in the driver's own words:
/// an empty response, a status that is not success, and no result at all.
#[test]
fn a_clean_exit_with_no_answer_fails_in_the_drivers_words() {
    let init = TOOL_LESS.lines().next().unwrap();
    let ended = |result: &str| {
        read(format!("{init}\n{result}"), FLASH)
            .fold
            .verdict(0, false)
    };
    // An empty response, and one of blanks only, name the status first (M4).
    for response in ["", "   ", " \n"] {
        let row = json!({"event": "result", "result": {"status": "SUCCESS", "response": response}});
        assert_eq!(
            ended(&row.to_string()),
            failed(Failure::Empty, Some("SUCCESS"), false),
            "{response:?}"
        );
    }
    assert_eq!(
        ended(r#"{"event":"result","result":{"status":"NOT OK"}}"#),
        failed(Failure::Unsuccessful, Some("NOT OK"), false)
    );
    assert_eq!(
        read(init, FLASH).fold.verdict(0, false),
        failed(Failure::Unended, None, false)
    );
    // A non-zero exit is named with the status a result named, and a status
    // with no turn at exit 1 is a launch refusal only before any init.
    let error = r#"{"event":"result","result":{"status":"ERROR"}}"#;
    let errored = read(format!("{init}\n{error}"), FLASH);
    for code in [1, 4] {
        assert_eq!(
            errored.fold.verdict(code, false),
            failed(Failure::Exited(code), Some("ERROR"), false)
        );
    }
    assert_eq!(
        read(init, FLASH).fold.verdict(5, false),
        failed(Failure::Exited(5), None, false)
    );
}

/// One figure per invocation, from the result: input with its cache read,
/// output, thinking as reasoning output, and the cache read itself. Cost
/// and cache writes are never reported, and a zero is not reported either.
#[test]
fn a_results_tokens_are_carried_and_cost_and_cache_writes_are_not_reported() {
    let tool_less = read(TOOL_LESS, FLASH).record;
    assert_eq!(
        Value::Object(tool_less),
        json!({"model": FLASH, "num_turns": 1, "input_tokens": 8869, "output_tokens": 1})
    );
    let tools = read(TOOLS_TURN, FLASH).record;
    let tokens = ["input_tokens", "output_tokens", "cache_read_tokens"].map(|key| &tools[key]);
    assert_eq!(tokens, [&json!(11186 + 6851), &json!(81), &json!(6851)]);
    let neon = read(NEON, "gemini-4-neon").record;
    assert_eq!(neon["reasoning_output_tokens"], 19);
    for record in [&tools, &neon] {
        for absent in ["total_cost_usd", "cache_write_tokens", "effort"] {
            assert!(!record.contains_key(absent), "{absent}");
        }
    }
    assert!(!tools.contains_key("reasoning_output_tokens"));
}

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// What every launch leads with.
const LEAD: [&str; 7] = [
    "agy",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--mode",
    "accept-edits",
];

/// A pin, an effort and two grants, in whatever order the seat wrote them:
/// the pin, the effort and the one `--settings` value, and never `-p`.
#[test]
fn a_launch_is_the_stream_head_the_pin_the_effort_and_one_settings_value() {
    let tail = [
        "--model",
        "gemini-4-neon",
        "--effort",
        "high",
        "--settings",
        r#"{"permissions":{"allow":["command(cargo)","command(gh pr view)"]}}"#,
    ];
    let golden = argv(&[&LEAD[..], &tail].concat());
    let seats: [&[&str]; 2] = [
        &[
            "--model",
            "gemini-4-neon",
            "--effort",
            "high",
            "--allow",
            "command(cargo),command(gh pr view)",
        ],
        &[
            "--allow",
            "command(cargo),command(gh pr view)",
            "--effort",
            "high",
            "--model",
            "gemini-4-neon",
        ],
    ];
    for seat in seats {
        let launched = launch("agy", &argv(seat), Hands::Native).unwrap();
        assert_eq!(launched.command, golden);
        assert_eq!(launched.pin, "gemini-4-neon");
    }
    // No grant and no effort: the settings flag still rides, once.
    let bare = launch("agy", &argv(&["--model", "gemini-4-argon"]), Hands::Native).unwrap();
    let tail = [
        "--model",
        "gemini-4-argon",
        "--settings",
        r#"{"permissions":{"allow":[]}}"#,
    ];
    assert_eq!(bare.command, argv(&[&LEAD[..], &tail].concat()));
    for launched in [&golden, &bare.command] {
        assert_eq!(
            launched.iter().filter(|part| *part == "--settings").count(),
            1
        );
        assert!(!launched.iter().any(|part| part == "-p"));
    }
}

#[test]
fn a_seat_writes_only_its_three_controls_once_each_with_a_value_a_pin_and_rules() {
    let refused = |parts: &[&str]| launch("agy", &argv(parts), Hands::Native).unwrap_err();
    for never in [
        "--add-dir",
        "--conversation",
        "--continue",
        "--dangerously-skip-permissions",
        "--sandbox",
        "--settings",
        "-p",
        "--print-timeout",
    ] {
        let error = refused(&["--model", "gemini-4-neon", never, "x"]);
        assert!(
            matches!(&error, LaunchError::Unadmitted(said) if said == never),
            "{error:?}"
        );
    }
    assert!(matches!(
        refused(&["gemini-4-neon"]),
        LaunchError::Positional
    ));
    assert!(matches!(
        refused(&["--effort", "low", "--effort", "high"]),
        LaunchError::Doubled("--effort")
    ));
    for valueless in [
        &["--model"][..],
        &["--model", "--effort", "low"],
        &["--model", ""],
    ] {
        assert!(matches!(
            refused(valueless),
            LaunchError::Valueless("--model")
        ));
    }
    assert!(matches!(
        refused(&["--effort", "low"]),
        LaunchError::Unpinned
    ));
    for rule in ["command uname", "write_file(/w)", "mcp(x", "command(git),"] {
        let error = refused(&["--model", "gemini-4-neon", "--allow", rule]);
        assert!(matches!(error, LaunchError::Unruled), "{rule}");
    }
    let rules = ["mcp()".to_string()];
    let untargeted = launch("agy", &argv(&["--model", "m"]), Hands::Boxed(&rules));
    assert!(matches!(untargeted, Err(LaunchError::Untargeted)));
}

/// A pin that is not a plain model id is refused before any launch, so no
/// refusal's `{pin:?}` escapes a bound value past the mask; a plain one
/// launches (#1178 Z1, checked for agy).
#[test]
fn a_pin_that_is_not_a_plain_model_id_is_refused() {
    for pin in [
        "sec\"ret-model",
        "gemini 4",
        "gemini\\4",
        "_gemini",
        "gémini",
    ] {
        let refused = launch("agy", &argv(&["--model", pin]), Hands::Native);
        assert!(matches!(refused, Err(LaunchError::Unplain)), "{pin}");
    }
    let plain = launch("agy", &argv(&["--model", "gemini-4.0_neon"]), Hands::Native);
    assert_eq!(plain.unwrap().pin, "gemini-4.0_neon");
}

/// Each launch refusal's words, pinned once.
#[test]
fn each_launch_refusal_states_what_was_written_and_never_a_value() {
    let io = || std::io::Error::from(std::io::ErrorKind::NotFound);
    let stated = [
        (LaunchError::Owned("HOME"), "the seat binds a secret named HOME, which the engine owns for an agy seat (#1200)"),
        (LaunchError::Owned("PWD"), "the seat binds a secret named PWD, which the engine owns for an agy seat (#1200)"),
        (LaunchError::Untargeted, "the seat's --allow carries a rule whose target is blank, has a blank at an end, or holds a control character or a parenthesis (#1200)"),
        (LaunchError::Unread("/w/.agents/plugins".into(), io()), "the agy seat's workspace path /w/.agents/plugins cannot be read, so whether it carries agy's configuration is unknown: entity not found (#1200)"),
        (LaunchError::Unsessioned(io()), "the engine's HOME for the agy seat was not made: hands session: entity not found"),
        (LaunchError::Unlocked(io()), "the engine's HOME for the agy seat was not made: its session's lock could not be taken: entity not found"),
        (LaunchError::Unowned, "the parent of the engine's HOME directories under the temporary directory is not a directory of this user's that nobody else may enter, so it holds no engine HOME for the agy seat (#1200)"),
        (LaunchError::Unadmitted("--sandbox".into()), "the seat's arguments carry '--sandbox', which brokkr driver agy does not admit; an agy seat's arguments are --model, --effort and --allow, each once (#1200)"),
        (LaunchError::Positional, "the seat's arguments carry a value with no option before it, which brokkr driver agy does not admit (#1200)"),
        (LaunchError::Doubled("--model"), "the seat's arguments carry '--model' more than once"),
        (LaunchError::Valueless("--allow"), "the seat's arguments carry '--allow' with no value"),
        (LaunchError::Unpinned, "the seat pins no model with --model; a provisional seat pins its id (#1200)"),
        (LaunchError::Unplain, "the agy model pin is not a plain model id (1 to 80 characters: an ASCII letter or digit, then ASCII letters, digits, '.', '_' and '-')"),
        (LaunchError::Unruled, "the seat's --allow carries a rule that is neither command(…) nor mcp(…), which agy would ignore without a word (#1200)"),
        (LaunchError::Unnamed, "the engine's HOME or executable for the agy seat has a path that is not UTF-8, so nothing names it exactly (#1200)"),
        (LaunchError::Unrooted("w/sub".into(), io()), "the agy seat's working directory w/sub cannot be resolved: entity not found"),
        (LaunchError::Untexted, "the agy seat's working directory resolves to a path that is not UTF-8, so the child's PWD cannot name it (#1200)"),
        (LaunchError::Configured("/w/.agents/plugins".into()), "/w/.agents/plugins stands in the agy seat's working directory or a directory above it, and brokkr driver agy refuses a seat there; remove it, or run the seat elsewhere (#1200)"),
        (LaunchError::SignedOut, "Antigravity CLI is signed out: run `agy` once and sign in; the login is ~/.gemini/antigravity-cli/antigravity-oauth-token (#1200)"),
        (LaunchError::Unhomed(io()), "the engine's HOME for the agy seat was not made: entity not found"),
        (LaunchError::BoxedGrant, "the seat's arguments carry '--allow', and an agy seat whose hands are boxed takes no native grant: its gate switches off every native tool, so the grant would do nothing (#1224)"),
        (LaunchError::Unboxed(crate::native_controls::SealedBoundary::Seatbelt), "the agy seat stands under the seatbelt boundary, which boxes its hands, and the box's hands were not served, so agy would run with its native tools; an agy seat under a boundary that boxes is boxed or refused (#1224)"),
        (LaunchError::Unserved(crate::native_controls::SealedBoundary::Container, McpRefusal::NotSealed { provider: "agy" }), "the agy seat stands under the container boundary, which boxes its hands, and the box's hands were not served: provider 'agy' final MCP configuration is not the engine's sealed server set; an agy seat under a boundary that boxes is boxed or refused (#1224)"),
    ];
    for (error, words) in stated {
        assert_eq!(
            error.to_string(),
            format!("refusing to invoke the agent CLI: {words}")
        );
    }
    let admission = LaunchError::from(McpRefusal::Unreadable);
    assert_eq!(admission.to_string(), McpRefusal::Unreadable.at_launch());
}

/// Each failure's words, pinned once, and the words a seat ends with on
/// each path out: the launch's, the failure's, a late refusal's and the
/// shared driver's own.
#[test]
fn each_failure_states_the_drivers_words_and_names_only_plain_words() {
    use fold::Event;
    let stated = [
        (Failure::Denied(argv(&["command", "mcp"])), "agy stopped the turn at a denial of command, mcp: a turn agy stopped at a denial is not an answer (#1200)"),
        (Failure::Empty, "agy's response was empty, which is not an answer (#1200)"),
        (Failure::Unsuccessful, "agy's turn ended with a status that is not SUCCESS, which is not an answer (#1200)"),
        (Failure::Unreadable, "agy's stdout could not be read to its end, so the turn gave no answer (#1200)"),
        (Failure::Unended, "agy's turn ended with no result, so it gave no answer (#1200)"),
        (Failure::Reinitialized, "agy wrote a second init row, and one launch is one turn (#1200)"),
        (Failure::Behind(Event::StepUpdate), "agy wrote a step_update row behind the turn's result, and one launch is one turn (#1200)"),
        (Failure::Behind(Event::Result), "agy wrote a result row behind the turn's result, and one launch is one turn (#1200)"),
        (Failure::Unread(Event::Init), "agy wrote an init row the driver cannot read, so the turn gave no answer (#1200)"),
        (Failure::Exited(5), "agy exited 5, so the turn gave no answer (#1200)"),
        (Failure::NotText, "agy wrote a line that is not UTF-8, so the turn gave no answer (#1200)"),
    ];
    for (failure, words) in stated {
        assert_eq!(failure.to_string(), words);
    }
    // What a result and an exit add, each and both: the status before the
    // failure's words, the timeout behind them (L5).
    let seen = |status: Option<&str>, timed_out| fold::Seen {
        status: status.map(str::to_string),
        timed_out,
    };
    let timeout = "; agy then exited 3 at its own print timeout";
    let added = [
        (seen(None, false), "", ""),
        (
            seen(Some("ERROR"), false),
            "agy's result had status ERROR; ",
            "",
        ),
        (seen(None, true), "", timeout),
        (
            seen(Some("ERROR"), true),
            "agy's result had status ERROR; ",
            timeout,
        ),
    ];
    for (seen, before, after) in added {
        let failed = Failed {
            failure: Failure::Empty,
            seen,
        };
        assert_eq!(
            failed.to_string(),
            format!("{before}{}{after}", Failure::Empty)
        );
    }
    let late = refusal_of(PRINT_TIMEOUT, 3);
    let empty = Failed {
        failure: Failure::Empty,
        seen: fold::Seen::default(),
    };
    let ended = [
        (
            Ended::Launch(LaunchError::Unpinned),
            LaunchError::Unpinned.to_string(),
        ),
        (Ended::Failed(empty), Failure::Empty.to_string()),
        (Ended::Late(late.clone()), late.words().to_string()),
        (
            Ended::from("could not".to_string()),
            "could not".to_string(),
        ),
    ];
    for (ended, words) in ended {
        assert_eq!(ended.to_string(), words);
    }
}

/// The prompt is one stream-json user line, serialized whole.
#[test]
fn the_prompt_is_one_user_line_whatever_it_holds() {
    assert_eq!(
        stdin_line("say \"hi\"\nthen C:\\done"),
        "{\"event\":\"user\",\"message\":{\"content\":\"say \\\"hi\\\"\\nthen C:\\\\done\"}}\n"
    );
}

/// What the driver declares of itself: no resume receipt, no probe, its
/// own names, and no route.
#[test]
fn an_agy_driver_declares_no_resume_and_no_probe() {
    let kind = AdapterKind::parse("agy").unwrap();
    assert_eq!(kind, AdapterKind::Agy);
    assert_eq!(kind.supports(), vec!["resume".to_string()]);
    assert!(!kind.advertises_probe());
    assert_eq!(
        (kind.driver_name().as_str(), kind.provider_name()),
        ("agy-cli", "agy")
    );
    assert_eq!(kind.served_route("gemini-4-neon"), None);
    let answer = json!({"event": "result", "result": {"status": "SUCCESS"}});
    let answer = super::super::row::Row::new(&answer, &[]);
    assert!(!super::super::probe::is_answer(kind, &answer));
    assert_eq!(
        crate::overrides::Override::of_driver(kind).name(),
        "BROKKR_AGY_BIN"
    );
}

/// agy's login under a HOME: the file [`operator_home`] signs in with,
/// and the one each test that reads or removes a login names.
const LOGIN: &str = ".gemini/antigravity-cli/antigravity-oauth-token";

/// An operator's HOME under `root`, signed in, with agy's and the user's
/// agent configuration, a tool's home, a file, and, on Linux, a name that
/// is not UTF-8: macOS's filesystem refuses such a name, so it is made only
/// where the host takes one, and a Linux host that refuses it fails here.
pub(in crate::adapters) fn operator_home(root: &Path) -> std::path::PathBuf {
    let home = root.join("operator");
    for dir in [
        ".gemini/antigravity-cli",
        ".gemini/config",
        ".agents",
        ".cargo/bin",
    ] {
        std::fs::create_dir_all(home.join(dir)).unwrap();
    }
    std::fs::write(home.join(LOGIN), "LOGIN").unwrap();
    std::fs::write(
        home.join(".gemini/config/mcp_config.json"),
        "{\"mcpServers\":{\"own\":{}}}",
    )
    .unwrap();
    std::fs::write(home.join(".gitconfig"), "[user]\n").unwrap();
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;
        std::fs::write(home.join(std::ffi::OsStr::from_bytes(b"odd\xff")), "").unwrap();
    }
    home
}

/// The engine's set when it serves nothing.
fn empty_set() -> Value {
    json!({"mcpServers": {}})
}

/// What a test HOME is furnished with: `mcp` and a hooks document.
fn written(mcp: &Value) -> Written<'_> {
    Written {
        mcp,
        hooks: r#"{"brokkr-gate":{}}"#,
    }
}

/// A root holding a signed-in operator HOME and an empty temporary root:
/// the root, the operator's HOME and the temporary root.
fn home_root() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let operator = operator_home(root.path());
    let temp = root.path().join("temp");
    std::fs::create_dir(&temp).unwrap();
    (root, operator, temp)
}

/// The names in `dir`, sorted.
fn listed(dir: &Path) -> Vec<String> {
    let entries = std::fs::read_dir(dir).unwrap();
    let mut names: Vec<String> = (entries.flatten())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The engine's HOME: mode 0700, a link to the login, the engine's own
/// configuration, and every other operator entry with a UTF-8 name linked,
/// `.gemini` and `.agents` not; gone with the value, the operator's intact.
/// The operator's name that is not UTF-8, left unlinked, stands on Linux
/// alone (see [`operator_home`]).
#[test]
fn the_engine_home_links_the_login_and_the_operators_entries_but_not_its_agent_state() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, operator, temp) = home_root();
    let homes = Homes {
        temp: &temp,
        operator: Some(&operator),
    };
    let set = empty_set();
    let home = home::made(homes, written(&set)).unwrap();
    let path = Path::new(home.path());
    // The `home` of a session of its own under agy's one parent, each
    // owner-only (K7).
    assert_eq!(path.file_name().unwrap(), "home");
    let session = path.parent().unwrap();
    assert_eq!(session.parent(), Some(home::parent(&temp).as_path()));
    let name = session.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("brokkr-hands-agy-"), "{name}");
    for dir in [path, &home::parent(&temp)] {
        let mode = std::fs::metadata(dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "{dir:?}");
    }
    assert_eq!(
        std::fs::read_link(path.join(LOGIN)).unwrap(),
        operator.join(LOGIN)
    );
    let config = |name: &str| std::fs::read_to_string(path.join(".gemini/config").join(name));
    assert_eq!(config("mcp_config.json").unwrap(), set.to_string());
    assert_eq!(config("hooks.json").unwrap(), written(&set).hooks);
    for linked in [".cargo", ".gitconfig"] {
        assert_eq!(
            std::fs::read_link(path.join(linked)).unwrap(),
            operator.join(linked)
        );
    }
    assert_eq!(listed(path), [".cargo", ".gemini", ".gitconfig"]);
    assert!(!path
        .join(".gemini")
        .symlink_metadata()
        .unwrap()
        .is_symlink());
    let made = session.to_path_buf();
    drop(home);
    assert!(!made.exists());
    assert_eq!(
        std::fs::read_to_string(operator.join(LOGIN)).unwrap(),
        "LOGIN"
    );
    assert!(operator.join(".cargo/bin").is_dir());
}

/// A temporary root whose path is not UTF-8 refuses the HOME unnamed and
/// leaves nothing in its parent. Linux only: macOS's filesystem refuses
/// to make a directory whose name is not UTF-8.
#[cfg(target_os = "linux")]
#[test]
fn a_temporary_root_whose_path_is_not_utf8_refuses_and_leaves_nothing() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let operator = operator_home(root.path());
    let lossy = root.path().join(std::ffi::OsStr::from_bytes(b"t\xfe"));
    std::fs::create_dir(&lossy).unwrap();
    let homes = Homes {
        temp: &lossy,
        operator: Some(&operator),
    };
    assert!(matches!(
        home::made(homes, written(&empty_set())),
        Err(LaunchError::Unnamed)
    ));
    let left = std::fs::read_dir(home::parent(&lossy)).unwrap().count();
    assert_eq!(left, 0);
}

/// No login, no HOME of the operator's, or an empty one: refused before
/// anything is made. A root the HOME cannot be made under and an operator
/// HOME that cannot be listed refuse too, and leave nothing behind.
#[test]
fn a_launch_with_no_login_or_no_room_for_its_home_refuses_and_leaves_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let temp = root.path().join("temp");
    std::fs::create_dir(&temp).unwrap();
    let signed_out = root.path().join("signed-out");
    std::fs::create_dir(&signed_out).unwrap();
    for operator in [Some(signed_out.as_path()), Some(Path::new("")), None] {
        let homes = Homes {
            temp: &temp,
            operator,
        };
        assert!(matches!(
            home::made(homes, written(&empty_set())),
            Err(LaunchError::SignedOut)
        ));
    }
    assert!(!home::parent(&temp).exists());
    let operator = operator_home(root.path());
    let missing = root.path().join("missing");
    let homes = |temp| Homes {
        temp,
        operator: Some(&operator),
    };
    let set = empty_set();
    let unmade = home::made(homes(&missing), written(&set))
        .map(drop)
        .unwrap_err();
    assert!(
        matches!(&unmade, LaunchError::Unhomed(cause) if cause.kind() == std::io::ErrorKind::NotFound)
    );
    // Listable by nobody, yet its login still reachable.
    std::fs::set_permissions(&operator, std::fs::Permissions::from_mode(0o100)).unwrap();
    let unlisted = home::made(homes(&temp), written(&set)).map(drop);
    std::fs::set_permissions(&operator, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(unlisted, Err(LaunchError::Unhomed(_))),
        "{unlisted:?}"
    );
    // A parent nobody may write in holds no session.
    let parent = home::parent(&temp);
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o500)).unwrap();
    let unsessioned = home::made(homes(&temp), written(&set)).map(drop);
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(unsessioned, Err(LaunchError::Unsessioned(_))),
        "{unsessioned:?}"
    );
    let left = std::fs::read_dir(home::parent(&temp)).unwrap().count();
    assert_eq!(left, 0);
    // A parent others may enter, or one that is no directory, is not this
    // user's alone.
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o750)).unwrap();
    let shared = home::made(homes(&temp), written(&set)).map(drop);
    assert!(matches!(&shared, Err(LaunchError::Unowned)), "{shared:?}");
    let linked = root.path().join("linked");
    std::fs::create_dir(&linked).unwrap();
    std::os::unix::fs::symlink(&parent, home::parent(&linked)).unwrap();
    let followed = home::made(homes(&linked), written(&set)).map(drop);
    assert!(
        matches!(followed, Err(LaunchError::Unowned)),
        "{followed:?}"
    );
}

/// A pid no process holds: a child's, reaped.
fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("true").spawn().unwrap();
    child.wait().unwrap();
    child.id()
}

/// A HOME a dead owner left is reaped by the next launch; one a live owner
/// holds, and one that cannot be judged, are left alone (J5).
#[test]
fn the_next_launch_reaps_a_home_whose_owner_is_dead_and_leaves_the_rest() {
    let (_root, operator, temp) = home_root();
    let homes = Homes {
        temp: &temp,
        operator: Some(&operator),
    };
    let set = empty_set();
    let live = home::made(homes, written(&set)).unwrap();
    let parent = home::parent(&temp);
    let named = |pid: u32| parent.join(format!("brokkr-hands-agy-{pid}-{}", uuid::Uuid::new_v4()));
    let (dead, unjudged) = (named(dead_pid()), named(dead_pid()));
    for tree in [&dead, &unjudged] {
        std::fs::create_dir_all(tree.join(".gemini/config")).unwrap();
    }
    std::fs::write(dead.join(".owner.lock"), "").unwrap();
    std::os::unix::fs::symlink(&operator, dead.join(".cargo")).unwrap();
    let next = home::made(homes, written(&set)).unwrap();
    assert!(!dead.exists());
    assert!(operator.join(".cargo/bin").is_dir());
    for left in [Path::new(live.path()), &unjudged, Path::new(next.path())] {
        assert!(left.is_dir(), "{left:?}");
    }
}

/// `dir/sub`, made: each layout's working directory.
fn sub(dir: &Path) -> std::path::PathBuf {
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    sub
}

/// `relative` made under `dir`, a directory where it names `plugins`.
fn plant(dir: &Path, relative: &str) -> std::path::PathBuf {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    match relative.ends_with("plugins") {
        true => std::fs::create_dir(&path).unwrap(),
        false => std::fs::write(&path, "{}").unwrap(),
    }
    path
}

const CONFIGURATION: [&str; 3] = [
    ".agents/mcp_config.json",
    ".agents/plugins",
    ".agents/hooks.json",
];

/// What the workspace check says of `workdir`: refused naming the planted
/// path, or admitted as its resolved path.
fn checked(workdir: &Path) -> Option<std::path::PathBuf> {
    let resolved = home::resolved(workdir.to_str().unwrap()).unwrap();
    assert_eq!(Path::new(&resolved), workdir.canonicalize().unwrap());
    match home::unconfigured(&resolved) {
        Ok(()) => None,
        Err(LaunchError::Configured(path)) => Some(path),
        Err(other) => panic!("{workdir:?}: {other:?}"),
    }
}

/// What stands between a working directory and the directory above it
/// that may carry agy's configuration: nothing, or a `.git` of either type.
const BETWEEN: [Option<&str>; 3] = [None, Some("dir"), Some("file")];

/// `between` made in `dir`: an empty `.git` directory, or a `.git` file.
fn dot_git(dir: &Path, between: Option<&str>) {
    match between {
        Some("dir") => std::fs::create_dir(dir.join(".git")).unwrap(),
        Some(_) => std::fs::write(dir.join(".git"), "gitdir: nowhere\n").unwrap(),
        None => {}
    }
}

/// Each of agy's workspace files, three directories above the working
/// directory, refuses, naming it, with no `.git` between and with a `.git`
/// directory or file between; a clean chain is admitted. No git is on the
/// PATH, so none is asked (L1).
/// This test empties `PATH`, which every sibling that starts a tool by name
/// or re-executes this binary reads, so it runs in an engine of its own (#1368).
#[test]
fn a_workspace_carrying_agys_configuration_anywhere_above_is_refused_with_no_git() {
    if !crate::process::in_its_own_engine(
        "adapters::agy::tests::a_workspace_carrying_agys_configuration_anywhere_above_is_refused_with_no_git",
    ) {
        return;
    }
    let mut env = crate::env_guard::EnvGuard::lock();
    let empty = tempfile::tempdir().unwrap();
    env.set("PATH", empty.path());
    for between in BETWEEN {
        let root = tempfile::tempdir().unwrap();
        let above = root.path().canonicalize().unwrap();
        let workdir = above.join("a/b/c");
        std::fs::create_dir_all(&workdir).unwrap();
        dot_git(&above.join("a/b"), between);
        assert_eq!(checked(&workdir), None, "clean, {between:?}");
        for relative in CONFIGURATION {
            let planted = plant(&above, relative);
            assert_eq!(checked(&workdir), Some(planted), "{relative} {between:?}");
            std::fs::remove_dir_all(above.join(".agents")).unwrap();
        }
    }
    drop(env);
    // An entry of any type: a directory, a file, a dangling link.
    let root = tempfile::tempdir().unwrap();
    let repo = sub(&root.path().canonicalize().unwrap());
    std::fs::create_dir_all(repo.join(".agents/hooks.json")).unwrap();
    assert_eq!(checked(&repo), Some(repo.join(".agents/hooks.json")));
    std::fs::remove_dir(repo.join(".agents/hooks.json")).unwrap();
    std::fs::write(repo.join(".agents/plugins"), "").unwrap();
    assert_eq!(checked(&repo), Some(repo.join(".agents/plugins")));
    let dangling = repo.join(".agents/mcp_config.json");
    std::os::unix::fs::symlink(repo.join("nowhere"), &dangling).unwrap();
    assert_eq!(checked(&repo), Some(dangling));
}

/// A working directory given as text, through a link, to a directory whose
/// name is not UTF-8 resolves to a path that is none, refused naming
/// nothing of it (U1). Linux only: macOS's filesystem refuses to make a
/// directory whose name is not UTF-8.
#[cfg(target_os = "linux")]
#[test]
fn a_workspace_path_that_resolves_to_no_text_refuses_naming_nothing() {
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let lone = sub(&root.path().canonicalize().unwrap());
    let lossy = lone.join(std::ffi::OsStr::from_bytes(b"t\xfe"));
    std::fs::create_dir(&lossy).unwrap();
    let link = lone.join("link");
    std::os::unix::fs::symlink(&lossy, &link).unwrap();
    let untexted = home::resolved(link.to_str().unwrap());
    assert!(
        matches!(&untexted, Err(LaunchError::Untexted)),
        "{untexted:?}"
    );
}

/// What the walk cannot read refuses, naming it: an unreadable path and a
/// working directory that is not there. An `.agents` that is a file holds
/// none of the three paths.
#[test]
fn a_workspace_path_that_cannot_be_read_refuses_naming_it() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let lone = sub(&root.path().canonicalize().unwrap());
    std::fs::write(lone.join(".agents"), "").unwrap();
    assert_eq!(checked(&lone), None);
    std::fs::remove_file(lone.join(".agents")).unwrap();
    let agents = lone.join(".agents");
    std::fs::create_dir(&agents).unwrap();
    std::fs::set_permissions(&agents, std::fs::Permissions::from_mode(0o000)).unwrap();
    let unread = home::unconfigured(lone.to_str().unwrap()).map(drop);
    std::fs::set_permissions(&agents, std::fs::Permissions::from_mode(0o700)).unwrap();
    let named = agents.join("mcp_config.json");
    assert!(
        matches!(&unread, Err(LaunchError::Unread(path, _)) if *path == named),
        "{unread:?}"
    );
    // Named as the seat gave it (K4).
    let given = "/nonexistent-brokkr-agy-workdir/sub";
    let missing = home::resolved(given);
    assert!(
        matches!(&missing, Err(LaunchError::Unrooted(named, _)) if named == given),
        "{missing:?}"
    );
}

/// A fake `agy` under `dir`: it records its argv, its HOME, its `PWD` and
/// the directory it runs in, its update switch (`unset` where it has none),
/// its stdin, the
/// engine's two configuration files and its login's link, prints `stdout`
/// and `stderr` as recorded, writes the seat's result where `delivers`,
/// and exits `code`.
fn fake_agy(
    dir: &Path,
    stdout: impl AsRef<[u8]>,
    stderr: &str,
    delivers: bool,
    code: i32,
) -> std::path::PathBuf {
    for (stream, printed) in [("stdout", stdout.as_ref()), ("stderr", stderr.as_bytes())] {
        std::fs::write(dir.join(stream), printed).unwrap();
    }
    let answer = dir.join("result.json");
    let result = if delivers {
        format!(
            "echo '{{\"result\":\"complete\"}}' > '{}'\n",
            answer.display()
        )
    } else {
        String::new()
    };
    let body = format!(
        "#!/bin/sh\nd='{dir}'\nprintf '%s\\n' \"$@\" > \"$d/argv\"\nprintf '%s' \"$HOME\" > \"$d/home\"\n\
         printf '%s' \"$PWD\" > \"$d/pwd\"\npwd -P > \"$d/cwd\"\n\
         printf '%s' \"${{AGY_CLI_DISABLE_AUTO_UPDATE-unset}}\" > \"$d/update\"\n\
         cat > \"$d/stdin\"\nc=\"$HOME/.gemini/config\"\ncat \"$c/mcp_config.json\" > \"$d/mcp\"\n\
         cat \"$c/hooks.json\" > \"$d/hooks\"\n\
         readlink \"$HOME/.gemini/antigravity-cli/antigravity-oauth-token\" > \"$d/login\"\n\
         cat \"$d/stderr\" >&2\ncat \"$d/stdout\"\n{result}exit {code}\n",
        dir = dir.display()
    );
    super::super::tests::executable(dir, "agy", &body)
}

/// One run's world: the seat's directory and an operator HOME of its own,
/// with the process environment held for the world's life, and how many
/// engine HOME sessions this thread had made before it. Every path of the
/// world is under `dir`, its root resolved, so each is the path the driver
/// resolves and names: a temporary directory reached through a link, as on
/// macOS, ends where the driver's words do.
struct World {
    _root: tempfile::TempDir,
    dir: std::path::PathBuf,
    env: crate::env_guard::EnvGuard,
    before: usize,
}

/// The session of each engine HOME made on this thread, in order.
fn made_here() -> Vec<std::path::PathBuf> {
    home::MADE.with(|made| made.borrow().clone())
}

impl World {
    fn new() -> World {
        let env = crate::env_guard::EnvGuard::lock();
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().canonicalize().unwrap();
        std::fs::create_dir(dir.join("seat")).unwrap();
        operator_home(&dir);
        let before = made_here().len();
        World {
            _root: root,
            dir,
            env,
            before,
        }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.dir.join(name)
    }

    fn seat(&self) -> std::path::PathBuf {
        self.path("seat")
    }

    /// The seat's input, its result at `result.json`, sealed `serving`
    /// where given.
    fn input(&self, serving: Option<Value>) -> Value {
        let seat = self.seat();
        let mut input = json!({
            "role_path": "/dev/null", "phase": "work", "feature": "f",
            "allowed_results": ["complete"], "result_path": seat.join("result.json"),
            "workdir": seat,
        });
        if let Some(serving) = serving {
            input[crate::native_controls::SERVING_INPUTS] = serving;
        }
        input
    }

    /// The binary a run starts and the operator's HOME, set while the world
    /// stands.
    fn set(&mut self, bin: &Path) {
        let operator = self.path("operator");
        self.env.set("BROKKR_AGY_BIN", bin);
        self.env.set("HOME", operator);
    }

    /// One seat on `input` run through the driver's own `run_seat`, offered
    /// `session` where given.
    fn run(
        &mut self,
        bin: &Path,
        extra: &[&str],
        input: Value,
        session: Option<&str>,
    ) -> Vec<crate::Body> {
        self.set(bin);
        let start = json!({"attempt_id": "attempt", "effect_id": "effect", "input": input});
        let mut messages = Vec::new();
        super::super::run_seat(
            AdapterKind::Agy,
            &argv(extra),
            &start,
            session,
            &mut |body| messages.push(body),
        );
        messages
    }

    /// One seat pinned to [`FLASH`] run over a fake printing the recorded
    /// tool-less turn and delivering, which succeeds.
    fn answers(&mut self) {
        let bin = fake_agy(&self.seat(), TOOL_LESS, "", true, 0);
        let input = self.input(None);
        let messages = self.run(&bin, &["--model", FLASH], input, None);
        assert_eq!(
            ended(&messages),
            Some((crate::ResultStatus::Succeeded, None, None))
        );
    }

    /// Whether the fake recorded its start.
    fn started(&self) -> bool {
        self.seat().join("argv").exists()
    }

    /// Whether every engine HOME made on this thread since the world began
    /// is gone, each by the path its session was recorded at as it was
    /// created, whatever stood in it and whatever another seat makes or
    /// reaps beside it in the shared parent. The HOME the fake ran under,
    /// where it started, is one of them.
    fn operator_home(&self) -> PathBuf {
        self.path("operator")
    }

    fn homes_gone(&self) -> bool {
        let made = made_here().split_off(self.before);
        let ran = std::fs::read_to_string(self.seat().join("home")).ok();
        let session = ran.as_deref().and_then(|home| Path::new(home).parent());
        assert!(session.is_none_or(|ran| made.iter().any(|made| made == ran)));
        let stands = |session: &std::path::PathBuf| std::fs::symlink_metadata(session).is_ok();
        !made.iter().any(stands)
    }

    fn recorded(&self, name: &str) -> String {
        std::fs::read_to_string(self.seat().join(name)).unwrap()
    }
}

/// The status, error and typed refusal of a run's one result, if it sent one.
fn ended(
    messages: &[crate::Body],
) -> Option<(crate::ResultStatus, Option<String>, Option<String>)> {
    messages.iter().find_map(|body| match body {
        crate::Body::Result {
            status,
            error,
            refusal,
            ..
        } => Some((
            *status,
            error.clone(),
            refusal.as_ref().map(|r| r.words().to_string()),
        )),
        _ => None,
    })
}

/// The seat's finishing checkpoint, if it sent one.
fn finished(messages: &[crate::Body]) -> Option<Value> {
    messages.iter().find_map(|body| match body {
        crate::Body::Checkpoint { data, .. } if data["step"] == "agy-cli-session-finished" => {
            Some(data.clone())
        }
        _ => None,
    })
}

#[test]
fn an_agy_seat_runs_the_overridden_binary_under_the_engine_home_and_answers_on_its_pin() {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let extra = [
        "--model",
        FLASH,
        "--effort",
        "low",
        "--allow",
        "command(git),command(ls)",
    ];
    let messages = world.run(&bin, &extra, world.input(None), None);
    assert_eq!(
        ended(&messages),
        Some((crate::ResultStatus::Succeeded, None, None))
    );
    assert_eq!(
        finished(&messages).unwrap(),
        json!({
            "step": "agy-cli-session-finished", "exit_code": 0, "model": FLASH,
            "effort": "not reported", "num_turns": 1, "input_tokens": 8869, "output_tokens": 1,
            "transcript": {"kind": "none", "locator": "", "home": ""},
        })
    );
    let settings = r#"{"permissions":{"allow":["command(git)","command(ls)"]}}"#;
    let tail = ["--model", FLASH, "--effort", "low", "--settings", settings];
    assert_eq!(
        world.recorded("argv").lines().collect::<Vec<_>>(),
        [&LEAD[1..], &tail].concat()
    );
    let stdin = world.recorded("stdin");
    let line: Value = serde_json::from_str(stdin.strip_suffix('\n').unwrap()).unwrap();
    assert_eq!(line["event"], "user");
    assert!(
        line["message"]["content"]
            .as_str()
            .unwrap()
            .contains("result.json"),
        "{stdin}"
    );
    assert_eq!(world.recorded("mcp"), r#"{"mcpServers":{}}"#);
    let hooks = world.recorded("hooks");
    assert!(
        hooks.contains(
            "'--gate' 'read_file' 'write_file' 'edit_file' 'run_command' 'manage_task'\""
        ),
        "{hooks}"
    );
    assert!(!hooks.contains("call_mcp_tool"), "{hooks}");
    // The child's HOME is the `home` directory of the session tree (K7).
    let home = world.recorded("home");
    let session = Path::new(&home).parent().unwrap();
    assert_eq!(Path::new(&home).file_name().unwrap(), "home");
    let named = session.file_name().unwrap().to_string_lossy();
    assert!(named.starts_with("brokkr-hands-agy-"), "{home}");
    assert_eq!(
        session.parent(),
        Some(home::parent(&std::env::temp_dir()).as_path())
    );
    assert!(!Path::new(&home).exists(), "{home}");
    let login = world.path("operator").join(LOGIN);
    assert_eq!(world.recorded("login").trim_end(), login.to_str().unwrap());
    assert!(world.homes_gone());
}

/// Refused at init, signed out, unspawnable, or refused before spawn: each
/// ends with its refusal or error and leaves no engine HOME behind.
#[test]
fn a_refused_or_unspawned_seat_states_why_and_leaves_no_home() {
    // One seat in a world of its own, over `recorded` where a fake runs,
    // and whether its engine HOME is gone after it.
    let seat_over = |recorded: Option<(&str, &str, i32)>, pin: &str| {
        let mut world = World::new();
        let bin = match recorded {
            Some((stdout, stderr, code)) => fake_agy(&world.seat(), stdout, stderr, false, code),
            None => world.path("no-agy"),
        };
        let messages = world.run(&bin, &["--model", pin], world.input(None), None);
        (ended(&messages).unwrap(), messages, world.homes_gone())
    };
    let ((status, _, refusal), messages, gone) = seat_over(Some((NEON, "", 0)), "gemini-4-carbon");
    let substituted = "model_substituted: the seat pinned \"gemini-4-carbon\" and agy served \
                       \"gemini-4-neon\"";
    assert_eq!(
        (status, refusal.as_deref()),
        (crate::ResultStatus::Failed, Some(substituted))
    );
    assert!(messages
        .iter()
        .all(|body| !matches!(body, crate::Body::Checkpoint { .. })));
    assert!(gone);
    let signed_out = Some((SIGNED_OUT, SIGNED_OUT_STDERR, 1));
    let ((status, error, refusal), _, gone) = seat_over(signed_out, FLASH);
    assert_eq!(
        (status, refusal),
        (crate::ResultStatus::Failed, Some(signed_out_words("ERROR")))
    );
    assert!(!error.unwrap().contains("accounts.google.com"));
    assert!(gone);
    let ((status, error, _), _, gone) = seat_over(None, FLASH);
    assert_eq!(status, crate::ResultStatus::Failed);
    let error = error.unwrap();
    assert!(
        error.starts_with("could not invoke the agent CLI"),
        "{error}"
    );
    assert!(gone);
}

/// A missing login, an offered resume, an argument the driver does not
/// admit and a workspace carrying agy's configuration each refuse before
/// spawn: the fake, which records its start, never runs.
#[test]
fn what_refuses_before_spawn_never_starts_agy() {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let pin = ["--model", FLASH];
    let refused = |messages: &[crate::Body]| ended(messages).and_then(|(_, error, _)| error);
    let sandbox = world.run(
        &bin,
        &["--model", FLASH, "--sandbox"],
        world.input(None),
        None,
    );
    assert_eq!(
        refused(&sandbox),
        Some(LaunchError::Unadmitted("--sandbox".into()).to_string())
    );
    let planted = plant(&world.seat(), ".agents/hooks.json");
    let configured = world.run(&bin, &pin, world.input(None), None);
    assert_eq!(
        refused(&configured),
        Some(LaunchError::Configured(planted.clone()).to_string())
    );
    std::fs::remove_file(planted).unwrap();
    std::fs::remove_file(world.path("operator").join(LOGIN)).unwrap();
    let signed_out = world.run(&bin, &pin, world.input(None), None);
    assert_eq!(
        refused(&signed_out),
        Some(LaunchError::SignedOut.to_string())
    );
    assert!(!world.started());
    assert!(world.homes_gone());
}

/// Serving inputs sealing the default hands spec under `stands` and
/// `servers`, the cold assessment `cold`, the replacement measured, under
/// `fence`.
fn sealed(
    stands: crate::native_controls::SealedBoundary,
    servers: &str,
    cold: Value,
    fence: &str,
) -> Value {
    let mut serving = super::super::mcp::tests::dispatched(
        crate::native_controls::SealedDialect {
            stands: Some(stands),
            ..Default::default()
        },
        Vec::new(),
        Some(crate::hands::HandsSpec::default()),
    );
    let measured = json!("measured");
    serving["isolation"] = json!({
        "fence": fence, "servers": servers, "resume": {}, "replacement": measured, "cold": cold,
    });
    serving
}

/// SI2 before spawn: past the lifted fence an unmeasured or unsupported
/// cold shape refuses, and a set the inputs do not declare refuses
/// whichever way the fence stands, under a boundary that boxes naming it.
/// Hands the standing fence serves as before are refused by the agy launch
/// itself (`boxed.rs`).
#[test]
fn the_admission_refuses_before_spawn() {
    use crate::native_controls::SealedBoundary::{Namespace, Open};
    let launch = |cause: &str| format!("refusing to invoke the agent CLI: {cause}");
    let not_sealed = McpRefusal::NotSealed { provider: "agy" };
    let boxed = LaunchError::Unserved(Namespace, not_sealed).to_string();
    let cases = [
        (
            sealed(Open, "empty", json!("unmeasured"), "lifted"),
            launch("provider 'agy' has no measured strict MCP configuration for 'cold'"),
        ),
        (
            sealed(Open, "empty", json!({"unsupported": "a reason"}), "lifted"),
            launch("provider 'agy' cannot exclude ambient MCP configuration (a reason)"),
        ),
        (
            sealed(Namespace, "empty", json!("measured"), "lifted"),
            boxed.clone(),
        ),
        (
            sealed(Namespace, "empty", json!("measured"), "standing"),
            boxed,
        ),
    ];
    for (serving, words) in cases {
        let (world, error) = served_refusal(serving);
        assert_eq!(error.as_ref(), Some(&words));
        assert!(!world.started(), "{words}");
    }
}

/// The error a seat pinned to [`FLASH`] on sealed `serving` ends with
/// through a fake `agy`, beside the world it ran in.
fn served_refusal(serving: Value) -> (World, Option<String>) {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
    let messages = world.run(&bin, &["--model", FLASH], world.input(Some(serving)), None);
    let error = ended(&messages).and_then(|(_, error, _)| error);
    (world, error)
}

/// An engine HOME made under the shared parent from `operator`.
fn made_from(operator: &Path) -> Home {
    let temp = std::env::temp_dir();
    let homes = Homes {
        temp: &temp,
        operator: Some(operator),
    };
    home::made(homes, written(&empty_set())).unwrap()
}

/// Another engine HOME standing in the shared parent beside a launch, made
/// by another seat's thread from another operator's HOME, is no HOME of the
/// launch's: the world's cleanup check holds while it stands, the launch's
/// own gone (Z3).
#[test]
fn the_cleanup_check_holds_beside_another_engine_home_in_the_shared_parent() {
    let mut world = World::new();
    let other = tempfile::tempdir().unwrap();
    let operator = operator_home(other.path());
    let beside = std::thread::scope(|scope| scope.spawn(|| made_from(&operator)).join());
    let beside = beside.unwrap();
    world.answers();
    assert!(Path::new(beside.path()).exists());
    assert!(world.homes_gone());
}

/// A HOME the world's own seat made still stands while it is held, though
/// no child ran under it and its login link is gone: the cleanup check
/// knows it by the path its session was made at, and holds once it goes
/// (Z3).
#[test]
fn the_cleanup_check_knows_a_standing_home_by_its_session_not_by_what_it_holds() {
    let world = World::new();
    let held = made_from(&world.path("operator"));
    std::fs::remove_file(Path::new(held.path()).join(LOGIN)).unwrap();
    assert!(!world.started());
    assert!(!world.homes_gone());
    drop(held);
    assert!(world.homes_gone());
}

/// A short secret bound to the seat, and agy's stderr and `error` text
/// carrying it.
const SECRET: &str = "tok9z";

/// One turn of a fake `agy` pinned to [`FLASH`] printing `stdout` and a
/// stderr that carries [`SECRET`], exiting `code`, run by [`drive`] with
/// the secret bound; beside it, the secret as masked.
pub(super) fn drive_flash(
    bin: &Path,
    mode: StartMode<'_>,
    input: &Value,
    bindings: &[secret::BoundSecret],
) -> Result<Invocation, String> {
    drive_flash_ended(bin, mode, input, bindings, &mut |_| {})
        .map_err(|ended| rendered(&ended, bindings))
}

pub(super) fn drive_flash_ended(
    bin: &Path,
    mode: StartMode<'_>,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
) -> Result<Invocation, Ended> {
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", FLASH]));
    let pinned = super::super::Pinned {
        binary: &binary,
        extra: &extra,
    };
    turn(pinned, mode, ("prompt", input, bindings), emit)
}

fn bound_turn(stdout: &str, delivers: bool, code: i32) -> (Result<Invocation, String>, String) {
    let mut world = World::new();
    let stderr = format!("\u{1b}[31merror: token tok\u{1b}[0m9z expired at https://x/{SECRET}\n");
    let bin = fake_agy(&world.seat(), stdout, &stderr, delivers, code);
    let bindings = [super::super::tests::binding("AT", SECRET)];
    let masked = secret::mask_text(SECRET.as_bytes(), &bindings);
    world.set(&bin);
    let input = world.input(None);
    let turn = drive_flash(&bin, StartMode::default(), &input, &bindings);
    assert!(world.homes_gone());
    (turn, masked)
}

/// No bound value reaches a refusal, a failure, a late refusal or the
/// invocation's stderr, which is stripped before it is masked.
#[test]
fn a_bound_secret_in_agys_stderr_or_error_text_reaches_no_word() {
    let init = TOOL_LESS.lines().next().unwrap();
    let signed_out = SIGNED_OUT.replace("timed out", &format!("timed out for {SECRET}"));
    let error_row = format!(
        "{{\"event\":\"result\",\"result\":{{\"status\":\"ERROR\",\"error\":\"bad {SECRET}\"}}}}"
    );
    let errored = format!("{init}\n{error_row}");
    let (refused, masked) = bound_turn(&signed_out, false, 1);
    let refused = refused.unwrap();
    assert_eq!(
        refused.refusal.map(|refusal| refusal.words().to_string()),
        Some(signed_out_words("ERROR"))
    );
    assert_eq!(refused.stderr, SIGN_IN_WITHHELD);
    // A turn refused another way keeps its stderr, stripped and masked.
    let (exited, _) = bound_turn(&error_row, false, 1);
    assert_eq!(
        exited.unwrap().stderr,
        format!("error: token {masked} expired at https://x/{masked}\n")
    );
    let (failed, _) = bound_turn(&errored, false, 0);
    assert_eq!(
        failed.map(drop),
        Err(
            "agy's result had status ERROR; agy's turn ended with a status that is not SUCCESS, \
             which is not an answer (#1200)"
                .to_string()
        )
    );
    let (denied, _) = bound_turn(DENIED_DEFAULT, false, 0);
    assert!(!denied.map(drop).unwrap_err().contains(SECRET));
    // A clean result no init confirmed is a late refusal once delivered.
    let (late, _) = bound_turn(TOOL_LESS.lines().last().unwrap(), true, 0);
    assert_eq!(
        late.map(drop),
        Err(
            "model_unconfirmed: agy's result had status SUCCESS; the seat pinned \
             \"gemini-3.8-flash-low\" and no init row named the model agy served"
                .to_string()
        )
    );
    let (answered, masked) = bound_turn(TOOL_LESS, true, 0);
    let answered = answered.unwrap();
    assert_eq!(answered.refusal, None);
    assert!(!answered.stderr.contains(SECRET), "{}", answered.stderr);
    assert!(answered.stderr.contains(&masked));
}

mod boxed;
mod judged;
mod keep;
mod last;
mod rejoin;
mod rejoin_launch;
mod resolved;
mod whole;
