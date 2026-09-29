//! The probe against fake CLIs: small scripts that emit the recorded
//! stream shapes of the three shipped harnesses, and the ways a CLI can
//! fall short of them. No test reaches a provider.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use super::facts::{Fact, Verdict};
use super::measure::{self, Observed};
use super::observe::{Observation, Trial};
use super::*;
use crate::secret;

#[path = "../../../../tests/support/executable.rs"]
mod executable;

const DATE: &str = "2026-09-29T09:00:00Z";
const DEADLINE: Duration = Duration::from_secs(60);
const CLAUDE_SESSION: &str = "5d0c1e2a-7b3f-4c1d-9e8a-2f6b0c4d8e11";
const CODEX_THREAD: &str = "0199aaaa-1111-7222-8333-444455556666";
const DSH_GAP: &str = "no CLI flag disables its shell and file tools";

/// What a Claude-like CLI does under the adapter's hands argv.
#[derive(Clone, Copy)]
enum Boxed {
    Empties,
    Keeps,
    Refuses,
}

/// A Claude-like CLI: stream-json whose `system/init` event lists its
/// tools and MCP servers, and whose one assistant message restates its
/// usage on each of its two events (#402). It reads the planted
/// user-scope server unless `--strict-mcp-config` is given.
fn claude_like(version: &str, boxed: Boxed) -> String {
    let under_hands = match boxed {
        Boxed::Empties => {
            r#"tools='"mcp__brokkr__workspace"'; servers='{"name":"brokkr","status":"connected"}'"#
        }
        Boxed::Keeps => {
            r#"tools='"Bash","WebFetch","mcp__brokkr__workspace"'; servers='{"name":"brokkr","status":"connected"}'"#
        }
        Boxed::Refuses => r#"echo "error: unknown option '--strict-mcp-config'" >&2; exit 1"#,
    };
    r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "@VERSION@ (Fake Claude)"; exit 0 ;;
  *" brokkr-probe-no-such-model "*) echo "API Error: 404 model not found: brokkr-probe-no-such-model" >&2; exit 1 ;;
  *" brokkr-probe-no-such-effort "*) echo "error: option '--effort <level>' argument 'brokkr-probe-no-such-effort' is invalid. Allowed choices are low, medium, high, xhigh, max." >&2; exit 1 ;;
esac
[ -n "$FAKE_TOKEN" ] || { echo "Invalid API key · Please run /login" >&2; exit 1; }
tools='"Bash","Read","WebSearch","WebFetch"'
servers=''
case " $* " in
  *" --strict-mcp-config "*) @UNDER_HANDS@ ;;
  *) grep -q brokkr-probe-user-scope "$HOME/.claude.json" && servers='{"name":"brokkr-probe-user-scope","status":"failed"}' ;;
esac
sid=@SESSION@
dir="$HOME/.claude/projects/$(pwd | tr -c 'A-Za-z0-9\n' '-')"
mkdir -p "$dir"
echo '{"type":"user"}' > "$dir/$sid.jsonl"
echo '{}' > "$HOME/.claude/stats.json"
printf '{"type":"system","subtype":"init","session_id":"%s","tools":[%s],"mcp_servers":[%s]}\n' "$sid" "$tools" "$servers"
printf '{"type":"assistant","message":{"id":"msg_1","content":[{"type":"text","text":"PROBE-OK"}],"usage":{"input_tokens":10,"cache_read_input_tokens":4,"output_tokens":2}},"session_id":"%s"}\n' "$sid"
printf '{"type":"assistant","message":{"id":"msg_1","content":[],"usage":{"input_tokens":10,"cache_read_input_tokens":4,"output_tokens":2}},"session_id":"%s"}\n' "$sid"
printf '{"type":"result","subtype":"success","session_id":"%s","total_cost_usd":0.0125,"usage":{"input_tokens":10,"cache_read_input_tokens":4,"output_tokens":2}}\n' "$sid"
"#
    .replace("@VERSION@", version)
    .replace("@UNDER_HANDS@", under_hands)
    .replace("@SESSION@", CLAUDE_SESSION)
}

/// A Codex-like CLI: `exec --json` events with usage on `turn.completed`
/// alone, no tool list on the stream, and its rollout filed under a
/// dated directory. Its model refusal is a JSON event that echoes the key.
const CODEX_LIKE: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "codex-cli 0.999.0"; exit 0 ;;
  *brokkr-probe-no-such-effort*) echo 'Error loading config: unknown variant `brokkr-probe-no-such-effort`, expected one of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`' >&2; exit 1 ;;
  *" brokkr-probe-no-such-model "*) printf '{"type":"error","message":"model brokkr-probe-no-such-model not found (key %s)"}\n' "$FAKE_TOKEN"; exit 1 ;;
esac
[ -n "$FAKE_TOKEN" ] || { echo "Not logged in. Run codex login." >&2; exit 1; }
tid=0199aaaa-1111-7222-8333-444455556666
dir="$HOME/.codex/sessions/2026/09/29"
mkdir -p "$dir"
echo '{"type":"session_meta"}' > "$dir/rollout-2026-09-29T10-00-00-$tid.jsonl"
printf '{"type":"thread.started","thread_id":"%s"}\n' "$tid"
printf '{"type":"turn.started"}\n'
printf '{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"PROBE-OK"}}\n'
printf '{"type":"turn.completed","usage":{"input_tokens":20,"cached_input_tokens":5,"output_tokens":3,"reasoning_output_tokens":1}}\n'
"#;

/// A dsh-like CLI: the headless profile prints only its answer, and the
/// session transcript's header lists its tools, web search among them.
const DSH_LIKE: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) echo "dsh 0.9.9"; exit 0 ;;
esac
[ -n "$FAKE_TOKEN" ] || { echo "Error: DEEPSEEK_API_KEY is not set" >&2; exit 1; }
mkdir -p "$HOME/.dsh/sessions"
printf '%s\n' '{"type":"header","session_id":"ds-20260929-0001","request":{"tools":[{"name":"bash"},{"name":"read_file"},{"name":"web_search"}]}}' '{"type":"message","role":"assistant","usage":{"prompt_tokens":30,"completion_tokens":4},"cost_usd":0.0001}' > "$HOME/.dsh/sessions/ds-20260929-0001.jsonl"
echo "PROBE-OK"
"#;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn claude_declared() -> Declared {
    Declared {
        adapter: "claude".to_string(),
        model_flag: Some("--model".to_string()),
        effort_flag: Some("--effort".to_string()),
        efforts: strings(&["low", "medium", "high", "xhigh", "max"]),
        hands: Some(strings(&[
            "--tools",
            "",
            "--strict-mcp-config",
            "--mcp-config",
            "{hands_mcp_json}",
            "--allowedTools",
            "mcp__brokkr__workspace",
        ])),
        hands_gap: None,
        passthrough: strings(&["--permission-mode", "acceptEdits"]),
        resume_versions: vec![("boxed-workspace".to_string(), "2.1.266".to_string())],
    }
}

fn codex_declared() -> Declared {
    Declared {
        adapter: "codex".to_string(),
        model_flag: Some("--model".to_string()),
        effort_flag: Some("--effort".to_string()),
        efforts: strings(&["none", "minimal", "low", "medium", "high", "xhigh", "max"]),
        hands: Some(strings(&[
            "--sandbox",
            "read-only",
            "-c",
            "mcp_servers.brokkr.command=\"{brokkr}\"",
            "-c",
            "mcp_servers.brokkr.args={hands_args_toml}",
        ])),
        hands_gap: None,
        passthrough: Vec::new(),
        resume_versions: vec![("work-site".to_string(), "0.154.0".to_string())],
    }
}

fn dsh_declared() -> Declared {
    Declared {
        adapter: "dsh".to_string(),
        model_flag: Some("--model".to_string()),
        effort_flag: Some("--effort".to_string()),
        efforts: strings(&["low", "medium", "high", "xhigh"]),
        hands: None,
        hands_gap: Some(DSH_GAP.to_string()),
        passthrough: Vec::new(),
        resume_versions: vec![("headless-work".to_string(), "0.1.5-rc.1".to_string())],
    }
}

/// A directory of fakes, and a secrets store binding `FAKE_TOKEN`.
struct World {
    dir: tempfile::TempDir,
    bindings: Vec<secret::BoundSecret>,
}

fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("secrets.env");
    secret::store_set(&store, "FAKE_TOKEN", "fake-token-4e1d").unwrap();
    let bindings = secret::resolve_bindings(&store, &["FAKE_TOKEN".to_string()]).unwrap();
    World { dir, bindings }
}

impl World {
    fn fake(&self, name: &str, body: &str) -> PathBuf {
        executable::install(self.dir.path(), name, body)
    }
}

fn probe_with(
    kind: AdapterKind,
    cli: &Path,
    declared: &Declared,
    bindings: &[secret::BoundSecret],
    deadline: Duration,
) -> Result<Report, ProbeError> {
    run(&ProbeInput {
        kind,
        cli: cli.to_str().unwrap(),
        declared,
        bindings,
        brokkr: Path::new("/opt/brokkr/bin/brokkr"),
        date: DATE,
        deadline,
    })
}

fn probe(kind: AdapterKind, cli: &Path, declared: &Declared, world: &World) -> Value {
    let report = probe_with(kind, cli, declared, &world.bindings, DEADLINE).unwrap();
    serde_json::to_value(report).unwrap()
}

fn measured(value: Value, evidence: &str) -> Value {
    json!({"status": "measured", "value": value, "evidence": evidence})
}

fn unmeasured(why: &str) -> Value {
    json!({"status": "unmeasured", "why": why})
}

fn unsupported(evidence: &str) -> Value {
    json!({"status": "unsupported", "evidence": evidence})
}

fn refusal(exit: i32, excerpt: &str) -> Value {
    measured(
        json!({"exit": exit, "excerpt": excerpt}),
        &format!("exit {exit}: {excerpt}"),
    )
}

fn field(field: &str, declared: &str, implied: &str, agreement: &str) -> Value {
    json!({"field": field, "declared": declared, "implied": implied, "agreement": agreement})
}

const RATE_LIMIT: &str = "not provoked: a rate limit spends quota and risks the account";
const OUTAGE: &str = "not provoked: a provider outage cannot be caused safely";
const RESUME: &str = "the probe does not drive a resume turn yet; decision 0056's per-shape \
                      assessment stays the adapter's (#226)";
const ISOLATED: &str =
    "a turn ran under a scratch HOME with only these credentials bound: [FAKE_TOKEN]";
const TRANSCRIPTS: &str = "the .jsonl files the turn created under the scratch HOME";
const DSH_PATCH_ONLY: &str = "dsh takes a model and an effort only through the profile patch \
                              its driver composes, which the probe does not compose";

fn envelope(adapter: &str, cli: &Path, version: &str, rest: Value) -> Value {
    let mut report = json!({
        "probe": "brokkr.harness-probe/v1",
        "adapter": adapter,
        "cli": {
            "command": cli.to_str().unwrap(),
            "version": measured(json!(version), "the first line `--version` printed"),
        },
        "host": {"os": std::env::consts::OS, "arch": std::env::consts::ARCH},
        "date": DATE,
        "drift": [],
    });
    report
        .as_object_mut()
        .unwrap()
        .extend(rest.as_object().unwrap().clone());
    report
}

fn headless(argv: &[&str], exit: i32) -> Value {
    measured(
        json!({"argv": argv, "exit": exit}),
        &format!("one turn ran with stdin closed: exit {exit}"),
    )
}

const CLAUDE_TURN: [&str; 8] = [
    "{cli}",
    "-p",
    "--output-format",
    "stream-json",
    "--verbose",
    "--permission-mode",
    "acceptEdits",
    "{prompt}",
];

#[test]
fn a_claude_like_cli_is_boxed_and_its_repeated_usage_is_named() {
    let world = world();
    let cli = world.fake("claude", &claude_like("9.9.9", Boxed::Empties));
    let listed_servers = "the system/init event listed mcp_servers: 1";
    let expected = envelope(
        "claude",
        &cli,
        "9.9.9 (Fake Claude)",
        json!({
            "facts": {
                "headless": headless(&CLAUDE_TURN, 0),
                "events": measured(json!({
                    "source": "stdout",
                    "format": "ndjson",
                    "non_json_lines": 0,
                    "types": ["system/init", "assistant", "result/success"],
                }), "4 events read from stdout"),
                "session": measured(
                    json!({"event": "system/init", "key": "session_id"}),
                    &format!("the system/init event announced session_id {CLAUDE_SESSION}"),
                ),
                "usage": measured(json!({
                    "locations": ["assistant /message/usage", "result/success /usage"],
                    "counters": ["cache_read_input_tokens", "input_tokens", "output_tokens"],
                    "counting": "repeated-per-message",
                }), "message msg_1 carried its usage on 2 events; count each message once"),
                "cost": measured(
                    json!(["result/success /total_cost_usd"]),
                    "the turn reported its cost at result/success /total_cost_usd",
                ),
                "refusals": {
                    "auth": refusal(1, "Invalid API key · Please run /login"),
                    "config": refusal(1, "API Error: 404 model not found: brokkr-probe-no-such-model"),
                    "rate_limit": unmeasured(RATE_LIMIT),
                    "outage": unmeasured(OUTAGE),
                },
                "efforts": measured(
                    json!(["low", "medium", "high", "xhigh", "max"]),
                    "exit 1: error: option '--effort <level>' argument 'brokkr-probe-no-such-effort' \
                     is invalid. Allowed choices are low, medium, high, xhigh, max.",
                ),
                "tools": measured(
                    json!(["Bash", "Read", "WebSearch", "WebFetch"]),
                    "the system/init event listed tools: 4",
                ),
                "boxed_tools": measured(json!([]), "the system/init event listed tools: 1"),
                "mcp_server": measured(json!("connected"), listed_servers),
                "native_egress": measured(
                    json!(["WebSearch", "WebFetch"]),
                    "the system/init event listed tools: 4",
                ),
                "egress_off": measured(json!(true), "the hands argv removed WebSearch, WebFetch"),
                "config_isolation": measured(json!(true), ISOLATED),
                "user_mcp_unboxed": measured(json!(true), listed_servers),
                "user_mcp_boxed": measured(json!(false), listed_servers),
                "transcripts": measured(
                    json!(["~/.claude/projects/{workdir}/{session}.jsonl"]),
                    TRANSCRIPTS,
                ),
                "resume": unmeasured(RESUME),
            },
            "adapter_fields": [
                field("efforts", "low, medium, high, xhigh, max", "low, medium, high, xhigh, max", "agrees"),
                field("hands", "supported", "supported", "agrees"),
                field("resume.boxed-workspace.identity.version", "2.1.266", "9.9.9 (Fake Claude)", "differs"),
            ],
            "eligibility": {
                "verdict": "boxed",
                "reason": "its own tools switch off and the hands MCP server connects",
            },
        }),
    );
    assert_eq!(
        probe(AdapterKind::Claude, &cli, &claude_declared(), &world),
        expected
    );
}

#[test]
fn a_codex_like_cli_whose_stream_lists_no_tools_is_held_to_tool_less_offices() {
    let world = world();
    let cli = world.fake("codex", CODEX_LIKE);
    let no_tools = "no event of the turn listed its tools";
    let no_servers = "no event of the turn listed its mcp_servers";
    let expected = envelope(
        "codex",
        &cli,
        "codex-cli 0.999.0",
        json!({
            "facts": {
                "headless": headless(&["{cli}", "exec", "--json", "-C", "{workdir}", "{prompt}"], 0),
                "events": measured(json!({
                    "source": "stdout",
                    "format": "ndjson",
                    "non_json_lines": 0,
                    "types": ["thread.started", "turn.started", "item.completed", "turn.completed"],
                }), "4 events read from stdout"),
                "session": measured(
                    json!({"event": "thread.started", "key": "thread_id"}),
                    &format!("the thread.started event announced thread_id {CODEX_THREAD}"),
                ),
                "usage": measured(json!({
                    "locations": ["turn.completed /usage"],
                    "counters": [
                        "cached_input_tokens",
                        "input_tokens",
                        "output_tokens",
                        "reasoning_output_tokens",
                    ],
                    "counting": "per-event",
                }), "no two usage-bearing events named the same message"),
                "cost": measured(json!([]), "no event carried total_cost_usd or cost_usd"),
                "refusals": {
                    "auth": refusal(1, "Not logged in. Run codex login."),
                    "config": refusal(
                        1,
                        r#"{"type":"error","message":"model brokkr-probe-no-such-model not found (key [secret:FAKE_TOKEN])"}"#,
                    ),
                    "rate_limit": unmeasured(RATE_LIMIT),
                    "outage": unmeasured(OUTAGE),
                },
                "efforts": measured(
                    json!(["none", "minimal", "low", "medium", "high", "xhigh", "max"]),
                    "exit 1: Error loading config: unknown variant `brokkr-probe-no-such-effort`, \
                     expected one of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`",
                ),
                "tools": unmeasured(no_tools),
                "boxed_tools": unmeasured(no_tools),
                "mcp_server": unmeasured(no_servers),
                "native_egress": unmeasured(no_tools),
                "egress_off": unmeasured("the plain turn's tools were not read"),
                "config_isolation": measured(json!(true), ISOLATED),
                "user_mcp_unboxed": unmeasured(no_servers),
                "user_mcp_boxed": unmeasured(no_servers),
                "transcripts": measured(
                    json!(["~/.codex/sessions/{n}/{n}/{n}/rollout-{n}-{n}-{n}T{n}-{n}-{n}-{session}.jsonl"]),
                    TRANSCRIPTS,
                ),
                "resume": unmeasured(RESUME),
            },
            "adapter_fields": [
                field(
                    "efforts",
                    "none, minimal, low, medium, high, xhigh, max",
                    "none, minimal, low, medium, high, xhigh, max",
                    "agrees",
                ),
                field("hands", "supported", "unmeasured", "not-compared"),
                field("resume.work-site.identity.version", "0.154.0", "codex-cli 0.999.0", "differs"),
            ],
            "eligibility": {
                "verdict": "tool-less-only",
                "reason": "its native egress has no measured off switch: the plain turn's tools \
                           were not read",
            },
        }),
    );
    assert_eq!(
        probe(AdapterKind::Codex, &cli, &codex_declared(), &world),
        expected
    );
}

#[test]
fn a_dsh_like_cli_is_read_from_its_transcript_and_held_to_tool_less_offices() {
    let world = world();
    let cli = world.fake("dsh", DSH_LIKE);
    let no_hands = format!(
        "the adapter declares no hands argv that switches the CLI's own tools off ({DSH_GAP})"
    );
    let no_user_config = "the probe knows no user-scope MCP configuration file for dsh";
    let not_boxed = format!("the boxed turn was not read: {no_hands}");
    let expected = envelope(
        "dsh",
        &cli,
        "dsh 0.9.9",
        json!({
            "facts": {
                "headless": headless(&["{cli}", "--profile", "headless", "{prompt}"], 0),
                "events": measured(json!({
                    "source": "~/.dsh/sessions/{session}.jsonl",
                    "format": "ndjson",
                    "non_json_lines": 0,
                    "types": ["header", "message"],
                }), "2 events read from ~/.dsh/sessions/{session}.jsonl"),
                "session": measured(
                    json!({"event": "header", "key": "session_id"}),
                    "the header event announced session_id ds-20260929-0001",
                ),
                "usage": measured(json!({
                    "locations": ["message /usage"],
                    "counters": ["completion_tokens", "prompt_tokens"],
                    "counting": "per-event",
                }), "no two usage-bearing events named the same message"),
                "cost": measured(
                    json!(["message /cost_usd"]),
                    "the turn reported its cost at message /cost_usd",
                ),
                "refusals": {
                    "auth": refusal(1, "Error: DEEPSEEK_API_KEY is not set"),
                    "config": unmeasured(DSH_PATCH_ONLY),
                    "rate_limit": unmeasured(RATE_LIMIT),
                    "outage": unmeasured(OUTAGE),
                },
                "efforts": unmeasured(DSH_PATCH_ONLY),
                "tools": measured(
                    json!(["bash", "read_file", "web_search"]),
                    "the header event listed tools: 3",
                ),
                "boxed_tools": unmeasured(&no_hands),
                "mcp_server": unmeasured(&no_hands),
                "native_egress": measured(json!(["web_search"]), "the header event listed tools: 3"),
                "egress_off": unmeasured(&not_boxed),
                "config_isolation": measured(json!(true), ISOLATED),
                "user_mcp_unboxed": unmeasured(no_user_config),
                "user_mcp_boxed": unmeasured(no_user_config),
                "transcripts": measured(json!(["~/.dsh/sessions/{session}.jsonl"]), TRANSCRIPTS),
                "resume": unmeasured(RESUME),
            },
            "adapter_fields": [
                field("efforts", "low, medium, high, xhigh", "unmeasured", "not-compared"),
                field("hands", "unsupported", "unmeasured", "not-compared"),
                field("resume.headless-work.identity.version", "0.1.5-rc.1", "dsh 0.9.9", "differs"),
            ],
            "eligibility": {
                "verdict": "tool-less-only",
                "reason": format!("its native egress has no measured off switch: {not_boxed}"),
            },
        }),
    );
    assert_eq!(
        probe(AdapterKind::Dsh, &cli, &dsh_declared(), &world),
        expected
    );
}

/// The facts a hands argv decides, and the verdict and row they imply.
fn under_hands(report: &Value) -> Value {
    json!({
        "boxed_tools": report["facts"]["boxed_tools"],
        "mcp_server": report["facts"]["mcp_server"],
        "egress_off": report["facts"]["egress_off"],
        "hands": report["adapter_fields"][1],
        "eligibility": report["eligibility"],
    })
}

#[test]
fn a_cli_that_keeps_its_tools_under_the_hands_argv_cannot_be_boxed() {
    let world = world();
    let cli = world.fake("claude", &claude_like("9.9.9", Boxed::Keeps));
    let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
    assert_eq!(
        under_hands(&report),
        json!({
            "boxed_tools": measured(json!(["Bash", "WebFetch"]), "the system/init event listed tools: 3"),
            "mcp_server": measured(json!("connected"), "the system/init event listed mcp_servers: 1"),
            "egress_off": measured(json!(false), "the hands argv left WebFetch"),
            "hands": field("hands", "supported", "unsupported", "differs"),
            "eligibility": {
                "verdict": "tool-less-only",
                "reason": "its native egress has no measured off switch: the hands argv left WebFetch",
            },
        })
    );
}

#[test]
fn a_cli_that_refuses_the_hands_argv_reads_unsupported() {
    let world = world();
    let cli = world.fake("claude", &claude_like("9.9.9", Boxed::Refuses));
    let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
    let refused = "the CLI refused the adapter's hands argv: exit 1: error: unknown option \
                   '--strict-mcp-config'";
    assert_eq!(
        under_hands(&report),
        json!({
            "boxed_tools": unsupported(refused),
            "mcp_server": unsupported(refused),
            "egress_off": unsupported(refused),
            "hands": field("hands", "supported", "unsupported", "differs"),
            "eligibility": {
                "verdict": "tool-less-only",
                "reason": format!("its native egress has no measured off switch: {refused}"),
            },
        })
    );
    assert_eq!(report["facts"]["user_mcp_boxed"], unsupported(refused));
}

#[test]
fn a_turn_without_its_credential_refuses_the_harness_and_measures_nothing_else() {
    let world = world();
    let cli = world.fake("claude", &claude_like("9.9.9", Boxed::Empties));
    let report = probe_with(AdapterKind::Claude, &cli, &claude_declared(), &[], DEADLINE).unwrap();
    let report = serde_json::to_value(report).unwrap();
    let failed = "the headless turn did not succeed: exit 1: Invalid API key · Please run /login";
    assert_eq!(report["facts"]["headless"], headless(&CLAUDE_TURN, 1));
    assert_eq!(
        report["facts"]["refusals"]["auth"],
        unmeasured("no credential was bound, so a turn without one is the plain turn")
    );
    for fact in [
        "events",
        "session",
        "usage",
        "cost",
        "tools",
        "native_egress",
        "config_isolation",
    ] {
        assert_eq!(report["facts"][fact], unmeasured(failed), "{fact}");
    }
    assert_eq!(
        report["eligibility"],
        json!({
            "verdict": "refused",
            "reason": format!(
                "no turn ran under a scratch HOME with only the bound credentials: {failed}"
            ),
        })
    );
}

#[test]
fn a_launch_past_its_deadline_is_killed_and_reports_no_exit_code() {
    let world = world();
    let cli = world.fake("sleeper", "#!/bin/sh\nexec sleep 30\n");
    let deadline = Duration::from_millis(200);
    let report = probe_with(
        AdapterKind::Dsh,
        &cli,
        &dsh_declared(),
        &world.bindings,
        deadline,
    );
    let report = serde_json::to_value(report.unwrap()).unwrap();
    let no_exit = "no exit code (a signal, or the probe's deadline)";
    assert_eq!(
        report["cli"]["version"],
        unmeasured(&format!(
            "`--version` printed no version: {no_exit}: (no output)"
        ))
    );
    assert_eq!(
        report["facts"]["headless"],
        measured(
            json!({"argv": ["{cli}", "--profile", "headless", "{prompt}"], "exit": null}),
            &format!("one turn ran with stdin closed: {no_exit}"),
        )
    );
    assert_eq!(
        report["facts"]["refusals"]["auth"],
        measured(
            json!({"exit": null, "excerpt": "(no output)"}),
            &format!("{no_exit}: (no output)"),
        )
    );
}

#[test]
fn a_credential_that_is_not_utf8_is_refused_through_the_one_injector() {
    use std::os::unix::fs::PermissionsExt;
    let world = world();
    let cli = world.fake("claude", &claude_like("9.9.9", Boxed::Empties));
    let store = world.dir.path().join("raw.env");
    std::fs::write(&store, b"FAKE_TOKEN=abcd\xff\n").unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    let bindings = secret::resolve_bindings(&store, &["FAKE_TOKEN".to_string()]).unwrap();
    let error = probe_with(
        AdapterKind::Claude,
        &cli,
        &claude_declared(),
        &bindings,
        DEADLINE,
    )
    .unwrap_err();
    assert!(matches!(error, ProbeError::Credential(_)), "{error:?}");
    assert_eq!(error.to_string(), "secret 'FAKE_TOKEN' is not valid UTF-8");
}

#[test]
fn a_cli_that_cannot_be_launched_is_refused_with_the_io_error() {
    let world = world();
    let missing = world.dir.path().join("no-such-cli");
    let error = probe_with(
        AdapterKind::Codex,
        &missing,
        &codex_declared(),
        &world.bindings,
        DEADLINE,
    )
    .unwrap_err();
    assert!(matches!(error, ProbeError::Io { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        format!(
            "could not launch {}: No such file or directory (os error 2)",
            missing.display()
        )
    );
}

#[test]
fn an_adapter_that_is_not_a_harness_is_refused_before_anything_runs() {
    let declared = Declared {
        adapter: "exec".to_string(),
        ..dsh_declared()
    };
    let error = probe_with(
        AdapterKind::Exec,
        Path::new("/nonexistent"),
        &declared,
        &[],
        DEADLINE,
    )
    .unwrap_err();
    assert!(matches!(error, ProbeError::NotAHarness { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        "adapter 'exec' is not an agent harness the probe knows; it measures claude, codex and dsh"
    );
}

#[test]
fn a_rerun_on_a_new_cli_version_reports_its_drift_against_the_previous_report() {
    let world = world();
    let old = world.fake("claude-old", &claude_like("9.9.9", Boxed::Empties));
    let new = world.fake("claude-new", &claude_like("10.0.0", Boxed::Refuses));
    let declared = claude_declared();
    let before = probe_with(
        AdapterKind::Claude,
        &old,
        &declared,
        &world.bindings,
        DEADLINE,
    )
    .unwrap();
    let text = serde_json::to_string_pretty(&before).unwrap();
    let previous = Report::parse(&text).unwrap();
    assert_eq!(previous, before);
    let after = probe_with(
        AdapterKind::Claude,
        &new,
        &declared,
        &world.bindings,
        DEADLINE,
    )
    .unwrap()
    .with_drift_from(&previous);
    assert_eq!(
        after.drift_lines(),
        vec![
            r#"drift: cli.version: measured "9.9.9 (Fake Claude)" -> measured "10.0.0 (Fake Claude)""#,
            "drift: boxed_tools: measured [] -> unsupported",
            r#"drift: mcp_server: measured "connected" -> unsupported"#,
            "drift: egress_off: measured true -> unsupported",
            "drift: user_mcp_boxed: measured false -> unsupported",
            r#"drift: eligibility: "boxed" -> "tool-less-only""#,
        ]
    );
    let unchanged = before.clone().with_drift_from(&previous);
    assert_eq!(unchanged.drift_lines(), Vec::<String>::new());
}

#[test]
fn a_previous_report_that_is_not_this_probe_s_is_refused() {
    let mut report = serde_json::to_value(sample_report()).unwrap();
    report["probe"] = json!("brokkr.harness-probe/v0");
    let error = Report::parse(&report.to_string()).unwrap_err();
    assert!(matches!(error, ProbeError::Report { .. }), "{error:?}");
    assert_eq!(
        error.to_string(),
        "not a brokkr.harness-probe/v1 report: its format is 'brokkr.harness-probe/v0'"
    );
    let error = Report::parse("{\"probe\": 1}").unwrap_err();
    assert_eq!(
        error.to_string(),
        "not a brokkr.harness-probe/v1 report: invalid type: integer `1`, expected a string \
         at line 1 column 11"
    );
}

/// A report as the drift and parse tests need one, from recorded
/// observations rather than a launch.
fn sample_report() -> Report {
    let declared = claude_declared();
    let plan = plan::plan(AdapterKind::Claude, &declared).unwrap();
    let observed = observed(observation(Some(0), "", ""));
    let facts = measure::facts(&plan, &observed, &[]);
    let version = measure::version(&observed.version);
    Report {
        probe: facts::PROBE_VERSION.to_string(),
        adapter: "claude".to_string(),
        adapter_fields: judge::adapter_fields(&declared, &facts, &version),
        eligibility: judge::eligibility(&facts),
        cli: Cli {
            command: "claude".to_string(),
            version,
        },
        host: Host {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
        },
        date: DATE.to_string(),
        facts,
        drift: Vec::new(),
    }
}

fn observation(exit: Option<i32>, stdout: &str, stderr: &str) -> Observation {
    Observation {
        exit,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        transcripts: Vec::new(),
    }
}

/// Every launch observed as `turn`, `--version` included.
fn observed(turn: Observation) -> Observed {
    Observed {
        version: turn.clone(),
        turn: turn.clone(),
        no_credentials: Trial::Observed(turn.clone()),
        bad_model: Trial::Observed(turn.clone()),
        bad_effort: Trial::Observed(turn.clone()),
        boxed: Trial::Observed(turn),
    }
}

fn facts_of(turn: Observation) -> super::facts::Facts {
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    measure::facts(&plan, &observed(turn), &[])
}

#[test]
fn a_clean_exit_that_prints_nothing_measures_no_version_and_no_refusal() {
    let silent = observation(Some(0), "", "");
    assert_eq!(
        measure::version(&silent),
        Fact::unmeasured("`--version` printed no version: exit 0: (no output)")
    );
    let facts = facts_of(silent);
    let no_refusal = Fact::unmeasured("the CLI exited 0, so there was no refusal to read");
    assert_eq!(facts.refusals.auth, no_refusal);
    assert_eq!(facts.refusals.config, no_refusal);
    assert_eq!(
        facts.efforts,
        Fact::unmeasured(
            "the CLI accepted the unknown effort 'brokkr-probe-no-such-effort' and exited 0, \
             so no refusal lists its levels"
        )
    );
    assert_eq!(
        facts.events,
        Fact::unmeasured(
            "the turn printed no JSON event and wrote no .jsonl transcript under the scratch HOME"
        )
    );
}

#[test]
fn a_refusal_that_lists_no_levels_leaves_efforts_unmeasured_and_clap_s_listing_is_read() {
    let facts = facts_of(observation(Some(2), "", "error: unexpected argument"));
    assert_eq!(
        facts.efforts,
        Fact::unmeasured(
            "the refusal names no accepted levels: exit 2: error: unexpected argument"
        )
    );
    let clap =
        "error: invalid value 'x' for '--effort <EFFORT>'\n  [possible values: low, high or max]";
    let facts = facts_of(observation(Some(2), "", clap));
    assert_eq!(
        facts.efforts.value(),
        Some(&strings(&["low", "high", "max"]))
    );
}

#[test]
fn an_unreadable_listing_or_usage_is_not_read_as_an_empty_one() {
    let stream = [
        r#"{"subtype":"init","tools":[{"id":1}],"mcp_servers":["brokkr"]}"#,
        r#"{"type":"assistant","usage":7,"total_cost_usd":null}"#,
        "not json",
    ]
    .join("\n");
    let facts = facts_of(observation(Some(0), &stream, ""));
    assert_eq!(
        facts.tools,
        Fact::unmeasured("the (untyped)/init event's tools held an entry it does not name")
    );
    assert_eq!(
        facts.mcp_server,
        Fact::unmeasured("the (untyped)/init event's mcp_servers held an entry it does not name")
    );
    assert_eq!(
        facts.usage,
        Fact::unmeasured("no event of the turn carried a usage object")
    );
    assert_eq!(
        facts.cost,
        Fact::measured(Vec::new(), "no event carried total_cost_usd or cost_usd")
    );
    assert_eq!(
        facts.session,
        Fact::unmeasured("no event named a session_id or thread_id")
    );
    assert_eq!(
        facts.events.value().map(|events| events.non_json_lines),
        Some(1)
    );
    assert_eq!(
        facts.transcripts,
        Fact::unmeasured("the turn wrote no .jsonl file under the scratch HOME")
    );
}

#[test]
fn a_transcript_path_without_a_session_keeps_its_name_and_loses_its_digits() {
    assert_eq!(
        measure::normalise("~/.cli/2026/run-7/tmp.brokkr-probe-Ab12/log.jsonl", None),
        "~/.cli/{n}/run-{n}/{workdir}/log.jsonl"
    );
}

#[test]
fn a_mapped_fact_keeps_its_reading() {
    let refused: Fact<Vec<String>> = Fact::Unsupported {
        evidence: "refused".to_string(),
    };
    assert_eq!(
        refused.map(|tools| tools.len()),
        Fact::Unsupported {
            evidence: "refused".to_string()
        }
    );
}

#[test]
fn a_plan_without_a_model_or_effort_flag_or_a_hands_reason_says_so() {
    let declared = Declared {
        model_flag: None,
        effort_flag: None,
        hands: None,
        hands_gap: None,
        ..claude_declared()
    };
    let plan = plan::plan(AdapterKind::Claude, &declared).unwrap();
    assert_eq!(
        (plan.bad_model, plan.bad_effort, plan.boxed),
        (
            plan::Step::Untried("the adapter declares no model flag".to_string()),
            plan::Step::Untried("the adapter declares no effort flag".to_string()),
            plan::Step::Untried(
                "the adapter declares no hands argv that switches the CLI's own tools off \
                 (no reason recorded)"
                    .to_string()
            ),
        )
    );
}

/// Facts with the hands argv's two readings set, the rest as a clean,
/// tool-less turn left them.
fn with_hands(
    boxed_tools: Fact<Vec<String>>,
    mcp_server: Fact<String>,
    egress_off: Fact<bool>,
) -> super::facts::Facts {
    let mut facts = facts_of(observation(
        Some(0),
        r#"{"type":"system","subtype":"init","tools":[],"mcp_servers":[]}"#,
        "",
    ));
    facts.boxed_tools = boxed_tools;
    facts.mcp_server = mcp_server;
    facts.egress_off = egress_off;
    facts
}

#[test]
fn a_hands_server_that_does_not_connect_is_not_boxed_and_removable_egress_is_unboxed() {
    let facts = with_hands(
        Fact::measured(Vec::new(), "emptied"),
        Fact::measured("failed".to_string(), "the init event listed mcp_servers: 1"),
        Fact::measured(true, "the plain turn listed no native egress tool"),
    );
    let eligibility = judge::eligibility(&facts);
    assert_eq!(eligibility.verdict, Verdict::UnboxedOnly);
    assert_eq!(
        eligibility.reason,
        "it is not shown to stand behind the box (emptied; the init event listed \
         mcp_servers: 1), and the plain turn listed no native egress tool"
    );
    let rows = judge::adapter_fields(
        &claude_declared(),
        &facts,
        &Fact::measured("v2.1.266 (Claude Code)".to_string(), "printed"),
    );
    assert_eq!(
        serde_json::to_value(&rows[1..]).unwrap(),
        json!([
            field("hands", "supported", "unsupported", "differs"),
            field(
                "resume.boxed-workspace.identity.version",
                "2.1.266",
                "v2.1.266 (Claude Code)",
                "differs"
            ),
        ])
    );
    let rows = judge::adapter_fields(
        &claude_declared(),
        &facts,
        &Fact::measured("2.1.266 (Claude Code)".to_string(), "printed"),
    );
    assert_eq!(rows[2].agreement, super::facts::Agreement::Agrees);
}
