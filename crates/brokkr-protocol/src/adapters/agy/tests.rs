//! Antigravity CLI 1.5's recorded turns (`docs/evidence/adapters/
//! agy-cli-1-5/`) folded byte for byte, the launch the driver composes, the
//! engine's HOME, the workspace refusal, and seats run end to end over a
//! fake `agy` (#1200).

use std::path::Path;

use serde_json::{json, Map, Value};

use super::*;
use crate::adapters::AdapterKind;
use crate::{PreSessionRefusal, RefusalClass};

macro_rules! recorded {
    ($file:literal) => {
        include_str!(concat!(
            "../../../../../docs/evidence/adapters/agy-cli-1-5/",
            $file
        ))
    };
}

const TOOL_LESS: &str = recorded!("tool-less-turn.ndjson");
const STDIN_TURN: &str = recorded!("stdin-stream-json.ndjson");
const NEON: &str = recorded!("named-model-neon-base.ndjson");
const CARBON: &str = recorded!("named-model-carbon-base.ndjson");
const HYDROGEN: &str = recorded!("named-model-hydrogen-base.ndjson");
const ARGON: &str = recorded!("named-model-argon-base.ndjson");
const TOOLS_TURN: &str = recorded!("tools-turn.ndjson");
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
const FLASH: &str = "gemini-3.8-flash-low";

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

/// One recorded stdout read through the fold under `pin`, as `invoke`
/// reads it: what it emitted, the record it left, and the refusal it
/// stopped at, if any.
struct Read {
    fold: AgyFold,
    rows: Vec<Value>,
    record: Map<String, Value>,
    stopped: Option<PreSessionRefusal>,
}

fn read(stdout: &str, pin: &str) -> Read {
    let mut fold = AgyFold::new(pin);
    let (mut rows, mut record, mut stopped) = (Vec::new(), Map::new(), None);
    for line in stdout.lines() {
        let event: Value = serde_json::from_str(line).unwrap();
        fold.observe(&event);
        let mut emitted = |row: &Value| rows.push(row.clone());
        if let Err(refused) = fold.read(event, &mut record, &mut emitted) {
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
fn turn_row(turn: u64, pin: &str) -> Value {
    json!({"step": "seat-turn", "turn": turn, "model": pin, "effort": "not reported"})
}

/// The `response` of the last row of `stdout`.
fn response(stdout: &str) -> Value {
    let last: Value = serde_json::from_str(stdout.lines().last().unwrap()).unwrap();
    last["result"]["response"].clone()
}

#[test]
fn the_tool_less_stdin_and_named_model_turns_answer_ready_on_their_pins() {
    let turns = [
        (TOOL_LESS, FLASH),
        (STDIN_TURN, FLASH),
        (NEON, "gemini-4-neon"),
        (CARBON, "gemini-4-carbon"),
        (HYDROGEN, "gemini-4-hydrogen"),
        (ARGON, "gemini-4-argon"),
    ];
    for (stdout, pin) in turns {
        let read = read(stdout, pin);
        assert_eq!(read.stopped, None, "{pin}");
        assert_eq!(read.fold.verdict(0), Verdict::Answer, "{pin}");
        assert_eq!(read.rows, [turn_row(1, pin)], "{pin}");
        assert_eq!(read.record["model"], pin);
        assert_eq!(response(stdout), "READY\n");
    }
}

/// The tools turn and the gate's own turn answer; the gate's refused
/// command and refused search were tool errors the turn went on past.
#[test]
fn the_tools_and_gate_turns_answer_past_the_calls_the_gate_refused() {
    for (stdout, turns) in [(TOOLS_TURN, 2), (GATE_PRIOR, 5)] {
        let read = read(stdout, FLASH);
        assert_eq!(read.fold.verdict(0), Verdict::Answer);
        let rows: Vec<Value> = (1..=turns).map(|turn| turn_row(turn, FLASH)).collect();
        assert_eq!(read.rows, rows);
    }
    assert_eq!(response(TOOLS_TURN), "DONE\n");
    assert_eq!(
        response(GATE_PRIOR),
        "(1) REFUSED\n(2) OK\n(3) OK\n(4) REFUSED\n"
    );
    for refused in ["run_command", "search_web"] {
        let step =
            format!("\"state\":\"ERROR\",\"step_type\":\"tool\",\"tool_name\":\"{refused}\"");
        assert!(GATE_PRIOR.contains(&step), "{refused}");
    }
    assert!(!GATE_PRIOR.contains("denied_actions"));
}

/// A turn agy stopped at a denial fails, naming each denied action, as
/// the result says it and whatever the denied step's state read, and
/// whatever the CLI exited with.
#[test]
fn a_turn_stopped_at_a_denial_is_a_failure_naming_each_action() {
    let failure = |action: &str| {
        Verdict::Failed(format!(
            "agy stopped the turn at a denial of {action}: a turn agy stopped at a denial is not \
             an answer (#1200)"
        ))
    };
    for (stdout, action) in [
        (DENIED_DEFAULT, "command"),
        (DENIED_ACCEPT, "command"),
        (ISO_08, "mcp"),
    ] {
        let read = read(stdout, FLASH);
        assert_eq!(read.stopped, None);
        for exit in [0, 1] {
            assert_eq!(read.fold.verdict(exit), failure(action), "{action} {exit}");
        }
        assert_eq!(response(stdout), "");
    }
    // The denied step read ERROR once and DONE the other time.
    let denied = "\"state\":\"ERROR\",\"step_type\":\"tool\",\"tool_name\":\"run_command\"";
    assert!(DENIED_DEFAULT.contains(denied));
    assert!(!DENIED_ACCEPT.contains("\"state\":\"ERROR\""));
    // An action that is no plain identifier is never quoted.
    let odd = r#"{"event":"result","result":{"status":"SUCCESS","response":"x","denied_actions":[{"action":"rm -rf /"},{"action":"command"}]}}"#;
    let read = read(
        &format!("{}\n{odd}", TOOL_LESS.lines().next().unwrap()),
        FLASH,
    );
    assert_eq!(read.fold.verdict(0), failure("<unnamed>, command"));
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

/// With no init row there is no answer: a step before it refuses, and a
/// clean result or a clean exit it never confirmed is unconfirmed.
#[test]
fn a_stream_with_no_init_row_gives_no_answer() {
    let unconfirmed = "model_unconfirmed: the seat pinned \"gemini-3.8-flash-low\" and no init \
                       row named the model agy served";
    let headless: String = TOOL_LESS.lines().skip(1).collect::<Vec<_>>().join("\n");
    assert_eq!(read(&headless, FLASH).stopped.unwrap().words(), unconfirmed);
    let result_only = TOOL_LESS.lines().last().unwrap();
    for stdout in [result_only, ""] {
        let Verdict::Refused(refused) = read(stdout, FLASH).fold.verdict(0) else {
            panic!("{stdout}");
        };
        assert_eq!(refused.words(), unconfirmed);
    }
    // An init that names no model, and rows the fold does not type, pass.
    let nameless = r#"{"event":"init","init":{"cwd":"/w"}}"#;
    let read = read(&format!("{nameless}\n5\n{{\"event\":\"other\"}}"), FLASH);
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
    match read(stdout, FLASH).fold.verdict(exit) {
        Verdict::Refused(refused) => refused,
        other => panic!("not a refusal: {other:?}"),
    }
}

#[test]
fn the_model_effort_and_flag_refusals_are_launch_refusals_quoting_nothing() {
    let model = "launch_refused: agy exited 1 before any turn: it does not serve the model or \
                 the effort the seat named";
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
            .verdict(2),
        Verdict::Exited
    );
}

#[test]
fn a_print_timeout_and_a_signed_out_cli_are_refusals_quoting_nothing() {
    let timeout = refusal_of(PRINT_TIMEOUT, 3);
    assert_eq!(
        timeout.words(),
        "print_timeout: agy exited 3 at its own print timeout, which this launch never asks for"
    );
    quotes_nothing(timeout.words(), PRINT_TIMEOUT, PRINT_TIMEOUT_STDERR);
    assert_eq!(refusal_of("", 3), timeout);
    let signed_out = refusal_of(SIGNED_OUT, 1);
    assert_eq!(signed_out.words(), SIGNED_OUT_WORDS);
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

/// A clean exit that is still no answer fails in the driver's own words:
/// an empty response, a status that is not success, and no result at all.
#[test]
fn a_clean_exit_with_no_answer_fails_in_the_drivers_words() {
    let init = TOOL_LESS.lines().next().unwrap();
    let ended = |result: &str| read(&format!("{init}\n{result}"), FLASH).fold.verdict(0);
    let failed = |words: &str| Verdict::Failed(words.to_string());
    assert_eq!(
        ended(r#"{"event":"result","result":{"status":"SUCCESS","response":" \n"}}"#),
        failed("agy's turn ended SUCCESS with an empty response, which is not an answer (#1200)")
    );
    assert_eq!(
        ended(r#"{"event":"result","result":{"status":"CANCELLED","error":"why"}}"#),
        failed("agy's turn ended with status CANCELLED, which is not an answer (#1200)")
    );
    assert_eq!(
        ended(r#"{"event":"result","result":{"status":"NOT OK"}}"#),
        failed("agy's turn ended with status <unnamed>, which is not an answer (#1200)")
    );
    assert_eq!(
        read(init, FLASH).fold.verdict(0),
        failed("agy's turn ended with no result, so it gave no answer (#1200)")
    );
    // A non-zero exit says it all, and a status with no turn at exit 1 is a
    // launch refusal only before any init.
    let error = r#"{"event":"result","result":{"status":"ERROR"}}"#;
    let errored = read(&format!("{init}\n{error}"), FLASH);
    assert_eq!(errored.fold.verdict(1), Verdict::Exited);
    assert_eq!(errored.fold.verdict(4), Verdict::Exited);
    assert_eq!(read(init, FLASH).fold.verdict(5), Verdict::Exited);
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
        let launched = launch("agy", &argv(seat), &[]).unwrap();
        assert_eq!(launched.command, golden);
        assert_eq!(launched.pin, "gemini-4-neon");
    }
    // No grant and no effort: the settings flag still rides, once.
    let bare = launch("agy", &argv(&["--model", "gemini-4-argon"]), &[]).unwrap();
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

/// The engine's rule joins the seat's in the one value, after them, and
/// alone where the seat grants nothing.
#[test]
fn an_engine_rule_joins_the_one_settings_value() {
    let rules = ["mcp(brokkr/workspace)".to_string()];
    let settings_of = |seat: &[&str]| launch("agy", &argv(seat), &rules).unwrap().command.pop();
    assert_eq!(
        settings_of(&["--model", "gemini-4-neon", "--allow", "command(git)"]).unwrap(),
        r#"{"permissions":{"allow":["command(git)","mcp(brokkr/workspace)"]}}"#
    );
    assert_eq!(
        settings_of(&["--model", "gemini-4-neon"]).unwrap(),
        r#"{"permissions":{"allow":["mcp(brokkr/workspace)"]}}"#
    );
}

#[test]
fn a_seat_writes_only_its_three_controls_once_each_with_a_value_a_pin_and_rules() {
    let refused = |parts: &[&str]| launch("agy", &argv(parts), &[]).unwrap_err();
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
    for rule in [
        "command uname",
        "write_file(/w)",
        "command()",
        "mcp(x",
        "command(git),",
    ] {
        let error = refused(&["--model", "gemini-4-neon", "--allow", rule]);
        assert!(matches!(error, LaunchError::Unruled), "{rule}");
    }
    let unruled = launch("agy", &argv(&["--model", "m"]), &["mcp()".to_string()]);
    assert!(matches!(unruled, Err(LaunchError::Unruled)));
}

/// Each launch refusal's words, pinned once.
#[test]
fn each_launch_refusal_states_what_was_written_and_never_a_value() {
    let io = || std::io::Error::from(std::io::ErrorKind::NotFound);
    let stated = [
        (LaunchError::Resumed, "the start offers a resume, and the agy adapter declares no resume (#1200)"),
        (LaunchError::Unadmitted("--sandbox".into()), "the seat's arguments carry '--sandbox', which brokkr driver agy does not admit; an agy seat's arguments are --model, --effort and --allow, each once (#1200)"),
        (LaunchError::Positional, "the seat's arguments carry a value with no option before it, which brokkr driver agy does not admit (#1200)"),
        (LaunchError::Doubled("--model"), "the seat's arguments carry '--model' more than once"),
        (LaunchError::Valueless("--allow"), "the seat's arguments carry '--allow' with no value"),
        (LaunchError::Unpinned, "the seat pins no model with --model; a provisional seat pins its id (#1200)"),
        (LaunchError::Unruled, "the seat's --allow carries a rule that is neither command(…) nor mcp(…), which agy would ignore without a word (#1200)"),
        (LaunchError::Unnamed, "the engine's HOME or executable for the agy seat has a path that is not UTF-8, so nothing names it exactly (#1200)"),
        (LaunchError::Unrooted(io()), "the agy seat's working directory cannot be resolved: entity not found"),
        (LaunchError::Configured("/w/.agents/plugins".into()), "the agy seat's workspace carries /w/.agents/plugins, which agy would start or run in a turn nobody trusted; remove it, or run the seat elsewhere (#1200)"),
        (LaunchError::SignedOut, "Antigravity CLI is signed out: run `agy` once and sign in; the login is ~/.gemini/antigravity-cli/antigravity-oauth-token (#1200)"),
        (LaunchError::Unhomed(io()), "the engine's HOME for the agy seat was not made: entity not found"),
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
    assert_eq!(kind.supports(), Vec::<String>::new());
    assert!(!kind.advertises_probe());
    assert_eq!(
        (kind.driver_name().as_str(), kind.provider_name()),
        ("agy-cli", "agy")
    );
    assert_eq!(kind.served_route("gemini-4-neon"), None);
    let answer = json!({"event": "result", "result": {"status": "SUCCESS"}});
    assert!(!super::super::probe::is_answer(kind, &answer));
    assert_eq!(
        crate::overrides::Override::of_driver(kind).name(),
        "BROKKR_AGY_BIN"
    );
}

/// An operator's HOME under `root`, signed in, with agy's and the user's
/// agent configuration, a tool's home, a file, and a name that is not
/// UTF-8.
fn operator_home(root: &Path) -> std::path::PathBuf {
    use std::os::unix::ffi::OsStrExt;
    let home = root.join("operator");
    for dir in [
        ".gemini/antigravity-cli",
        ".gemini/config",
        ".agents",
        ".cargo/bin",
    ] {
        std::fs::create_dir_all(home.join(dir)).unwrap();
    }
    std::fs::write(
        home.join(".gemini/antigravity-cli/antigravity-oauth-token"),
        "LOGIN",
    )
    .unwrap();
    std::fs::write(
        home.join(".gemini/config/mcp_config.json"),
        "{\"mcpServers\":{\"own\":{}}}",
    )
    .unwrap();
    std::fs::write(home.join(".gitconfig"), "[user]\n").unwrap();
    std::fs::write(home.join(std::ffi::OsStr::from_bytes(b"odd\xff")), "").unwrap();
    home
}

const WRITTEN: Written<'static> = Written {
    mcp: r#"{"mcpServers":{}}"#,
    hooks: r#"{"hooks":{}}"#,
};

/// The engine's HOME: mode 0700, a link to the login, the engine's own
/// configuration, and every other operator entry with a UTF-8 name linked,
/// `.gemini` and `.agents` not; gone with the value, the operator's intact.
#[test]
fn the_engine_home_links_the_login_and_the_operators_entries_but_not_its_agent_state() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let operator = operator_home(root.path());
    let temp = root.path().join("temp");
    std::fs::create_dir(&temp).unwrap();
    let homes = Homes {
        temp: &temp,
        operator: Some(&operator),
    };
    let home = home::made(homes, WRITTEN).unwrap();
    let path = Path::new(home.path());
    assert!(path.starts_with(&temp), "{path:?}");
    let mode = std::fs::metadata(path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o700);
    let login = ".gemini/antigravity-cli/antigravity-oauth-token";
    assert_eq!(
        std::fs::read_link(path.join(login)).unwrap(),
        operator.join(login)
    );
    let config = |name: &str| std::fs::read_to_string(path.join(".gemini/config").join(name));
    assert_eq!(config("mcp_config.json").unwrap(), WRITTEN.mcp);
    assert_eq!(config("hooks.json").unwrap(), WRITTEN.hooks);
    for linked in [".cargo", ".gitconfig"] {
        assert_eq!(
            std::fs::read_link(path.join(linked)).unwrap(),
            operator.join(linked)
        );
    }
    let mut entries: Vec<String> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, [".cargo", ".gemini", ".gitconfig"]);
    assert!(!path
        .join(".gemini")
        .symlink_metadata()
        .unwrap()
        .is_symlink());
    let made = path.to_path_buf();
    drop(home);
    assert!(!made.exists());
    assert_eq!(
        std::fs::read_to_string(operator.join(login)).unwrap(),
        "LOGIN"
    );
    assert!(operator.join(".cargo/bin").is_dir());
}

/// No login, no HOME of the operator's, or an empty one: refused before
/// anything is made. A root the HOME cannot be made under, one whose path
/// is not UTF-8, and an operator HOME that cannot be listed refuse too,
/// and leave nothing behind.
#[test]
fn a_launch_with_no_login_or_no_room_for_its_home_refuses_and_leaves_nothing() {
    use std::os::unix::ffi::OsStrExt;
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
            home::made(homes, WRITTEN),
            Err(LaunchError::SignedOut)
        ));
    }
    let operator = operator_home(root.path());
    let missing = root.path().join("missing");
    let homes = |temp| Homes {
        temp,
        operator: Some(&operator),
    };
    let unmade = home::made(homes(&missing), WRITTEN).map(drop).unwrap_err();
    assert!(
        matches!(&unmade, LaunchError::Unhomed(cause) if cause.kind() == std::io::ErrorKind::NotFound)
    );
    let lossy = root.path().join(std::ffi::OsStr::from_bytes(b"t\xfe"));
    std::fs::create_dir(&lossy).unwrap();
    assert!(matches!(
        home::made(homes(&lossy), WRITTEN),
        Err(LaunchError::Unnamed)
    ));
    // Listable by nobody, yet its login still reachable.
    std::fs::set_permissions(&operator, std::fs::Permissions::from_mode(0o100)).unwrap();
    let unlisted = home::made(homes(&temp), WRITTEN).map(drop);
    std::fs::set_permissions(&operator, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        matches!(unlisted, Err(LaunchError::Unhomed(_))),
        "{unlisted:?}"
    );
    for made in [&temp, &lossy] {
        assert_eq!(std::fs::read_dir(made).unwrap().count(), 0, "{made:?}");
    }
}

/// A repository under `root` with a working directory two levels below its
/// git root, and the directory above that root.
fn repository(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let above = root.join("above");
    let repo = above.join("repo");
    std::fs::create_dir_all(repo.join(".git")).unwrap();
    let workdir = repo.join("crates/sub");
    std::fs::create_dir_all(&workdir).unwrap();
    (above, workdir)
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

/// Each of agy's workspace files, in the working directory or at the git
/// root above it, refuses, naming it; above the root nothing is read, and
/// a directory in no repository is read alone (`iso-02`, `iso-05`,
/// `iso-06`).
#[test]
fn a_workspace_carrying_agys_configuration_up_to_its_git_root_is_refused() {
    for relative in CONFIGURATION {
        for at in ["workdir", "root"] {
            let root = tempfile::tempdir().unwrap();
            let (_, workdir) = repository(root.path());
            let workdir = workdir.canonicalize().unwrap();
            let dir = match at {
                "workdir" => workdir.clone(),
                _ => workdir.parent().unwrap().parent().unwrap().to_path_buf(),
            };
            let planted = plant(&dir, relative);
            let refused = home::unconfigured(workdir.to_str().unwrap());
            assert!(
                matches!(&refused, Err(LaunchError::Configured(path)) if *path == planted),
                "{relative} at {at}: {refused:?}"
            );
        }
        let root = tempfile::tempdir().unwrap();
        let (above, workdir) = repository(root.path());
        plant(&above, relative);
        assert!(
            home::unconfigured(workdir.to_str().unwrap()).is_ok(),
            "{relative}"
        );
        let lone = root.path().join("lone/work");
        std::fs::create_dir_all(&lone).unwrap();
        plant(&root.path().join("lone"), relative);
        assert!(
            home::unconfigured(lone.to_str().unwrap()).is_ok(),
            "{relative}"
        );
        plant(&lone, relative);
        assert!(matches!(
            home::unconfigured(lone.to_str().unwrap()),
            Err(LaunchError::Configured(_))
        ));
    }
    let missing = home::unconfigured("/nonexistent-brokkr-agy-workdir");
    assert!(matches!(missing, Err(LaunchError::Unrooted(_))));
}

/// A fake `agy` under `dir`: it records its argv, its HOME, its stdin, the
/// engine's two configuration files and its login's link, prints `stdout`
/// and `stderr` as recorded, writes the seat's result where `delivers`,
/// and exits `code`.
fn fake_agy(
    dir: &Path,
    stdout: &str,
    stderr: &str,
    delivers: bool,
    code: i32,
) -> std::path::PathBuf {
    for (stream, printed) in [("stdout", stdout), ("stderr", stderr)] {
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
         cat > \"$d/stdin\"\nc=\"$HOME/.gemini/config\"\ncat \"$c/mcp_config.json\" > \"$d/mcp\"\n\
         cat \"$c/hooks.json\" > \"$d/hooks\"\n\
         readlink \"$HOME/.gemini/antigravity-cli/antigravity-oauth-token\" > \"$d/login\"\n\
         cat \"$d/stderr\" >&2\ncat \"$d/stdout\"\n{result}exit {code}\n",
        dir = dir.display()
    );
    super::super::tests::executable(dir, "agy", &body)
}

/// One run's world: the seat's directory and an operator HOME, with the
/// process environment held for the world's life, so no other seat makes
/// an engine HOME while it looks, and each engine HOME standing before it.
struct World {
    root: tempfile::TempDir,
    env: crate::env_guard::EnvGuard,
    before: Vec<std::ffi::OsString>,
}

/// Each engine HOME standing under the temporary root.
fn engine_homes() -> Vec<std::ffi::OsString> {
    let entries = std::fs::read_dir(std::env::temp_dir()).unwrap().flatten();
    let names = entries.map(|entry| entry.file_name());
    let homes = names.filter(|name| name.to_string_lossy().starts_with(home::HOME_PREFIX));
    homes.collect()
}

impl World {
    fn new() -> World {
        let env = crate::env_guard::EnvGuard::lock();
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("seat")).unwrap();
        operator_home(root.path());
        let before = engine_homes();
        World { root, env, before }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.root.path().join(name)
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

    /// Whether the fake recorded its start.
    fn started(&self) -> bool {
        self.seat().join("argv").exists()
    }

    /// Whether every engine HOME the world's seats made is gone.
    fn homes_gone(&self) -> bool {
        engine_homes() == self.before
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
        hooks.contains("--gate read_file write_file edit_file run_command manage_task\""),
        "{hooks}"
    );
    assert!(!hooks.contains("call_mcp_tool"), "{hooks}");
    let home = world.recorded("home");
    let named = Path::new(&home).file_name().unwrap().to_string_lossy();
    assert!(named.starts_with(home::HOME_PREFIX), "{home}");
    assert_eq!(
        Path::new(&home).parent(),
        Some(std::env::temp_dir().as_path())
    );
    assert!(!Path::new(&home).exists(), "{home}");
    let login = world.path("operator/.gemini/antigravity-cli/antigravity-oauth-token");
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
        (status, refusal.as_deref()),
        (crate::ResultStatus::Failed, Some(SIGNED_OUT_WORDS))
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
    let resumed = world.run(&bin, &pin, world.input(None), Some("6bed7c60"));
    assert_eq!(refused(&resumed), Some(LaunchError::Resumed.to_string()));
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
    std::fs::remove_file(world.path("operator/.gemini/antigravity-cli/antigravity-oauth-token"))
        .unwrap();
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
/// cold shape refuses, a set the inputs do not declare refuses whichever
/// way the fence stands, and the standing fence serves unmeasured hands
/// the empty set and no `call_mcp_tool`.
#[test]
fn the_admission_refuses_before_spawn_or_serves_the_empty_set() {
    use crate::native_controls::SealedBoundary::{Namespace, Open};
    let cases = [
        (
            sealed(Open, "empty", json!("unmeasured"), "lifted"),
            "provider 'agy' has no measured strict MCP configuration for 'cold'",
        ),
        (
            sealed(Open, "empty", json!({"unsupported": "a reason"}), "lifted"),
            "provider 'agy' cannot exclude ambient MCP configuration (a reason)",
        ),
        (
            sealed(Namespace, "empty", json!("measured"), "lifted"),
            "provider 'agy' final MCP configuration is not the engine's sealed server set",
        ),
        (
            sealed(Namespace, "empty", json!("measured"), "standing"),
            "provider 'agy' final MCP configuration is not the engine's sealed server set",
        ),
    ];
    for (serving, cause) in cases {
        let mut world = World::new();
        let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
        let messages = world.run(&bin, &["--model", FLASH], world.input(Some(serving)), None);
        let error = ended(&messages).and_then(|(_, error, _)| error);
        assert_eq!(
            error,
            Some(format!("refusing to invoke the agent CLI: {cause}"))
        );
        assert!(!world.started(), "{cause}");
    }
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
    let standing = sealed(Namespace, "hands", json!("unmeasured"), "standing");
    world.run(&bin, &["--model", FLASH], world.input(Some(standing)), None);
    assert_eq!(world.recorded("mcp"), r#"{"mcpServers":{}}"#);
    assert!(!world.recorded("hooks").contains("call_mcp_tool"));
    let settings = world.recorded("argv").lines().last().unwrap().to_string();
    assert_eq!(settings, r#"{"permissions":{"allow":[]}}"#);
}

/// With one engine server: its document in the engine's HOME, its rule in
/// the one settings value and `call_mcp_tool` on the gate's list.
#[test]
fn an_engine_server_rides_the_home_the_settings_and_the_gate() {
    let served = crate::native_controls::mcp::agy::Served {
        document: json!({"mcpServers": {"brokkr": {"command": "/opt/brokkr", "args": []}}}),
        rules: vec!["mcp(brokkr/workspace)".to_string()],
    };
    let launched = launch("agy", &argv(&["--model", "gemini-4-neon"]), &served.rules).unwrap();
    assert_eq!(
        launched.command.last().unwrap(),
        r#"{"permissions":{"allow":["mcp(brokkr/workspace)"]}}"#
    );
    let hooks = gate::hooks(Path::new("/opt/brokkr"), !served.rules.is_empty()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let operator = operator_home(root.path());
    let homes = Homes {
        temp: root.path(),
        operator: Some(&operator),
    };
    let mcp = served.document.to_string();
    let home = home::made(
        homes,
        Written {
            mcp: &mcp,
            hooks: &hooks,
        },
    )
    .unwrap();
    let config = Path::new(home.path()).join(".gemini/config");
    let document: Value =
        serde_json::from_slice(&std::fs::read(config.join("mcp_config.json")).unwrap()).unwrap();
    assert_eq!(document, served.document);
    let hook = std::fs::read_to_string(config.join("hooks.json")).unwrap();
    assert!(hook.contains("manage_task call_mcp_tool\""), "{hook}");
}

/// A short secret bound to the seat, and agy's stderr and `error` text
/// carrying it.
const SECRET: &str = "tok9z";

/// One turn of a fake `agy` pinned to [`FLASH`] printing `stdout` and a
/// stderr that carries [`SECRET`], exiting `code`, run by [`drive`] with
/// the secret bound; beside it, the secret as masked.
fn bound_turn(stdout: &str, delivers: bool, code: i32) -> (Result<Invocation, String>, String) {
    let mut world = World::new();
    let stderr = format!("\u{1b}[31merror: token tok\u{1b}[0m9z expired at https://x/{SECRET}\n");
    let bin = fake_agy(&world.seat(), stdout, &stderr, delivers, code);
    let bindings = [super::super::tests::binding("AT", SECRET)];
    let masked = secret::mask_text(SECRET.as_bytes(), &bindings);
    world.set(&bin);
    let input = world.input(None);
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", FLASH]));
    let pinned = super::super::Pinned {
        binary: &binary,
        extra: &extra,
    };
    let turn = drive(pinned, false, "prompt", &input, &bindings, &mut |_| {});
    assert!(world.homes_gone());
    (turn, masked)
}

/// No bound value reaches a refusal, a failure, a late refusal or the
/// invocation's stderr, which is stripped before it is masked.
#[test]
fn a_bound_secret_in_agys_stderr_or_error_text_reaches_no_word() {
    let init = TOOL_LESS.lines().next().unwrap();
    let signed_out = SIGNED_OUT.replace("timed out", &format!("timed out for {SECRET}"));
    let errored = format!(
        "{init}\n{{\"event\":\"result\",\"result\":{{\"status\":\"ERROR\",\"error\":\"bad {SECRET}\"}}}}"
    );
    let (refused, masked) = bound_turn(&signed_out, false, 1);
    let refused = refused.unwrap();
    assert_eq!(
        refused.refusal.as_ref().map(PreSessionRefusal::words),
        Some(SIGNED_OUT_WORDS)
    );
    assert_eq!(
        refused.stderr,
        format!("error: token {masked} expired at https://x/{masked}\n")
    );
    let (failed, _) = bound_turn(&errored, false, 0);
    assert_eq!(
        failed.map(drop),
        Err("agy's turn ended with status ERROR, which is not an answer (#1200)".to_string())
    );
    let (denied, _) = bound_turn(DENIED_DEFAULT, false, 0);
    assert!(!denied.map(drop).unwrap_err().contains(SECRET));
    // A clean result no init confirmed is a late refusal once delivered.
    let (late, _) = bound_turn(TOOL_LESS.lines().last().unwrap(), true, 0);
    assert_eq!(
        late.map(drop),
        Err(
            "model_unconfirmed: the seat pinned \"gemini-3.8-flash-low\" and no init row named \
             the model agy served"
                .to_string()
        )
    );
    let (answered, masked) = bound_turn(TOOL_LESS, true, 0);
    let answered = answered.unwrap();
    assert_eq!(answered.refusal, None);
    assert!(!answered.stderr.contains(SECRET), "{}", answered.stderr);
    assert!(answered.stderr.contains(&masked));
}
