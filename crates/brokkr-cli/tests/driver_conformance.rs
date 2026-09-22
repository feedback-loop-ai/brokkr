#![cfg(unix)]

//! Conformance for the built-in adapters (`brokkr driver <kind>`) — the
//! Rust port of the retired Python suite. Shims stand in for the agent
//! CLIs; conformance means capabilities on hello, accepted + checkpoint
//! + exactly one result per start, and the result-file contract honored.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use brokkr_core::realms::Boundary;
use brokkr_runtime::agents::{Adapters, Availability, Library};
use brokkr_runtime::dialect::Dialect;
use brokkr_runtime::engine::{compose_site, BuiltBoundary};
use brokkr_runtime::{
    operator_command, resolve_agent, Bundle, Engine, SeatBody, SeatClass, StepBody,
};
use brokkr_store::{validate_seat_record, SeatRecordVersion, Store};
use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const OBEDIENT_SHIM: &str = r#"#!/bin/sh
# honor the result contract: find the result path in the prompt
# (stdin or the last .md argument) and write a typed result there.
last=""
for a in "$@"; do last="$a"; done
case "$last" in
  *.md) prompt=$(cat "$last") ;;
  *Result?contract*) prompt=$last ;;
  *) prompt=$(cat) ;;
esac
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
printf '{"type":"result","session_id":"s1","num_turns":1,"total_cost_usd":0.0}\n'
printf 'session id: deadbeef1234\n'
"#;

// The claude flavor speaks stream-json: an init with the session id, two
// tool-using assistant turns (the second carrying two tool_use blocks in
// one message), a noise line the adapter must drop, and a final result
// with the session totals — while still honoring the result-file contract.
//
// Each assistant record carries the harness's own top-level `effort`
// echo, and the RESULT — and only the result — carries the thinking
// tokens: that granularity is the measurement decision 0035 ruling 4
// records, and it is why a claude turn checkpoint has no reasoning count
// to report.
const CLAUDE_STREAM_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"stream-1"}\n'
printf '{"type":"assistant","effort":"xhigh","message":{"model":"claude-fable-5-1","usage":{"input_tokens":10,"cache_creation_input_tokens":4,"cache_read_input_tokens":3,"output_tokens":2},"content":[{"type":"text","text":"looking"},{"type":"tool_use","name":"Read","input":{"file_path":"src/lib.rs"}}]}}\n'
printf 'not json, ignorable noise\n'
printf '{"type":"assistant","effort":"high","message":{"model":"claude-fable-5-1","usage":{"input_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":10,"output_tokens":3},"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}},{"type":"tool_use","name":"Write","input":{"file_path":"src/out.rs"}}]}}\n'
printf '{"type":"result","num_turns":2,"total_cost_usd":0.125,"usage":{"input_tokens":15,"cache_creation_input_tokens":4,"cache_read_input_tokens":13,"output_tokens":5,"output_tokens_details":{"thinking_tokens":4}}}\n'
"#;

// Codex meters its reasoning per turn and reports its cache creation as
// `cache_write_input_tokens`; it puts NEITHER its model NOR its effort
// on this stream — `turn.completed` carries `usage` and nothing else —
// and writes both into the thread record decision 0032's locator names.
// This shim files one where a real codex does: under
// `$CODEX_HOME/sessions`, in a dated directory, named for the thread it
// announced, and in the envelope a real rollout uses — every field of a
// record under `payload`, never under a key spelled like the record.
// That envelope is the whole test: pointers written against an imagined
// shape read a real codex not at all, and a shim written to match those
// pointers is an imagined harness agreeing with itself.
//
// The rollout is filed AFTER the thread is announced, deliberately.
// Announcing and filing are two writes and codex promises no order
// between them; a locator that resolved once at `thread.started` and
// cached the miss would report `not reported` here — dsh's sentinel on
// a harness that does echo its effort, which is the one distinction
// ruling 3 rests on. Conformance is where that ordering is held.
const CODEX_JSON_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
thread="$CODEX_HOME/sessions/2026/09/03"
printf '{"type":"thread.started","thread_id":"codex-thread-1"}\n'
mkdir -p "$thread"
printf '{"timestamp":"2026-09-03T00:00:01Z","ordinal":7,"type":"turn_context","payload":{"turn_id":"t1","approval_policy":"never","model":"gpt-5.6-sol","effort":"xhigh","summary":"auto"}}\n' \
  > "$thread/rollout-2026-09-03T00-00-00-codex-thread-1.jsonl"
printf '{"type":"turn.started"}\n'
printf '{"type":"item.started","item":{"type":"command_execution","command":"secret command"}}\n'
printf '{"type":"item.completed","item":{"type":"command_execution","aggregated_output":"private output"}}\n'
printf '{"type":"turn.completed","usage":{"input_tokens":21,"cached_input_tokens":8,"output_tokens":5,"cache_write_input_tokens":6,"reasoning_output_tokens":3}}\n'
"#;

/// A dsh that behaves like the installed one: it prints nothing the
/// driver reads on stdout and writes its session transcript under the
/// root the seat overlay pins, naming the model that served each
/// message the way the JSONL backend does (`data.message.source.model`)
/// and, before it, the request header echoing the level it applied
/// (`data.header.config.reasoningEffort`) — read from the settings
/// document the overlay's `settings` row points at, which is where a
/// real 0.1.2-rc.1 reads it (decision 0035 addendum). A real launcher
/// has no `--effort` flag, so one reaching this shim's argv is a
/// driver that forwarded the pin the wrong way, and the shim fails.
const DSH_USAGE_SHIM: &str = r#"#!/bin/sh
prompt=$*
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
root=
sp=
prev=
for a in "$@"; do
  [ "$a" = --effort ] && exit 3
  if [ "$prev" = --patch ]; then
    root=$(awk -F"'" '/^    root: /{print $2}' "$a")
    sp=$(awk -F"'" '/^    path: /{print $2}' "$a")
  fi
  prev=$a
done
d="$root/--conformance--/session-served"
mkdir -p "$d"
f="$d/session.v3.jsonl"
printf '{"type":"session","version":0,"id":"session-conformance-1","cwd":"/w"}\n' > "$f"
if [ -n "$sp" ]; then
  lvl=$(awk -F"'" '/reasoningEffort/{print $2}' "$sp")
  printf '{"type":"request/header","data":{"header":{"config":{"provider":"deepseek-official","model":"deepseek-v4-flash","reasoningEffort":"%s"}}}}\n' "$lvl" >> "$f"
fi
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"deepseek-v4-flash"}},"usage":{"inputTokens":13,"outputTokens":3}}}\n' >> "$f"
"#;

/// The same shim, writing the name the previously shipped, plugin-free
/// core wrote. A disabled or identity-mismatched launch runs whatever
/// core the host has installed, so this is not a legacy curiosity: it is
/// the cold route AS1 requires to keep the telemetry it already had. The
/// filename is a LITERAL here, so moving a constant cannot quietly move
/// what this shim proves.
const DSH_SHIPPED_USAGE_SHIM: &str = r#"#!/bin/sh
prompt=$*
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work", "model": "seat-claim"}' > "$target"
root=
sp=
prev=
for a in "$@"; do
  [ "$a" = --effort ] && exit 3
  if [ "$prev" = --patch ]; then
    root=$(awk -F"'" '/^    root: /{print $2}' "$a")
    sp=$(awk -F"'" '/^    path: /{print $2}' "$a")
  fi
  prev=$a
done
d="$root/--conformance--/session-served"
mkdir -p "$d"
f="$d/session.jsonl"
printf '{"type":"session","version":0,"id":"session-conformance-1","cwd":"/w"}\n' > "$f"
if [ -n "$sp" ]; then
  lvl=$(awk -F"'" '/reasoningEffort/{print $2}' "$sp")
  printf '{"type":"request/header","data":{"header":{"config":{"provider":"deepseek-official","model":"deepseek-v4-flash","reasoningEffort":"%s"}}}}\n' "$lvl" >> "$f"
fi
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"deepseek-v4-flash"}},"usage":{"inputTokens":13,"outputTokens":3}}}\n' >> "$f"
"#;

const SILENT_SHIM: &str = "#!/bin/sh\ncat > /dev/null 2>&1 || true\necho did nothing\n";

// Decision 0053: a provider that refuses before the first turn. The
// claude/lanetally stream carries the refusal machine-readably — the
// synthetic `assistant` record the #219 run measured, then the error
// `result` — so the driver withholds `accepted` and the attempt is a
// determinate failure to start.
const CLAUDE_REFUSAL_SHIM: &str = r#"#!/bin/sh
cat > /dev/null
printf '{"type":"system","subtype":"init","session_id":"refused-1"}\n'
printf '{"type":"assistant","isApiErrorMessage":true,"error":"rate_limit","message":{"content":[{"type":"text","text":"You have reached your limit"}]}}\n'
printf '{"type":"result","is_error":true,"error":"rate_limit","result":"You have reached your limit"}\n'
"#;

// Codex announces the same refusal as its own machine event, before any
// `turn.started`.
const CODEX_REFUSAL_SHIM: &str = r#"#!/bin/sh
cat > /dev/null
printf '{"type":"error","message":"stream error: rate limit"}\n'
"#;

// dsh's headless profile makes no machine-readable refusal available: a
// rejection reaches the driver as stderr prose plus a non-zero exit, and
// the adapter must NOT sniff it (decision 0001). exec has no provider at
// all. Both therefore accept and then fail, as decision 0006 has it.
const REFUSING_STDERR_SHIM: &str = r#"#!/bin/sh
echo "dsh: error: rate limit exceeded" >&2
exit 1
"#;

/// The determinate shape decision 0053 requires: capabilities, then one
/// failed result, and neither an `accepted` nor a checkpoint ever.
fn assert_determinate_refusal(out: &[Value], label: &str, needle: &str) {
    let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["capabilities", "result"], "{label}: {out:?}");
    assert!(
        !out.iter().any(|m| m["type"] == "accepted"),
        "{label}: a refusal must not accept: {out:?}"
    );
    assert!(
        !out.iter().any(|m| m["type"] == "checkpoint"),
        "{label}: a refusal must not checkpoint: {out:?}"
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed", "{label}: {result}");
    let error = result["error"].as_str().unwrap_or_default();
    assert!(error.contains(needle), "{label}: {error}");
}

/// An adapter whose wire protocol has no machine-readable refusal keeps
/// decision 0016's boundary: it accepts, then fails mid-session.
fn assert_accepted_failure(out: &[Value], label: &str) {
    assert!(
        out.iter().any(|m| m["type"] == "accepted"),
        "{label}: this adapter has no machine-readable refusal, so it accepts: {out:?}"
    );
    assert!(
        out.iter().any(|m| m["type"] == "checkpoint"),
        "{label}: {out:?}"
    );
    let result = out.last().unwrap();
    assert_eq!(result["type"], "result", "{label}: {result}");
    assert_eq!(result["status"], "failed", "{label}: {result}");
}

#[test]
fn claude_refusal_before_the_first_turn_is_determinate() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), CLAUDE_REFUSAL_SHIM);
    let out = drive(
        &["claude", "--", "--model", "claude-fable-5-1"],
        &shim,
        dir.path(),
    );
    assert_determinate_refusal(&out, "claude", "rate_limit");
}

#[test]
fn lanetally_refusal_before_the_first_turn_is_determinate() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), CLAUDE_REFUSAL_SHIM);
    let out = drive(
        &["lanetally", "--", "--model", "claude-fable-5-1"],
        &shim,
        dir.path(),
    );
    assert_determinate_refusal(&out, "lanetally", "rate_limit");
}

#[test]
fn codex_refusal_before_the_first_turn_is_determinate() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), CODEX_REFUSAL_SHIM);
    let out = drive(
        &["codex", "--", "--model", "gpt-5.6-sol"],
        &shim,
        dir.path(),
    );
    assert_determinate_refusal(&out, "codex", "rate limit");
}

#[test]
fn dsh_refusal_has_no_machine_readable_shape_and_stays_mid_session() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), REFUSING_STDERR_SHIM);
    let out = drive(
        &[
            "dsh",
            "--",
            "--model",
            "deepseek/deepseek-v4-flash",
            "--effort",
            "medium",
        ],
        &shim,
        dir.path(),
    );
    assert_accepted_failure(&out, "dsh");
}

/// The DSH admission rule judges the WHOLE payload the operator's
/// command line carried, through the built binary.
///
/// Clap consumes the outer `--` itself, so a terminator still standing in
/// the collected arguments is one the seat wrote. While the CLI cut the
/// payload at that inner terminator, these two argv reached the adapter
/// SHORTER than they were typed and launched: the first arrived empty and
/// launched with no pin at all, and the second arrived as its second
/// `--model` pair and launched on that model. Both are refused now — by
/// the field that owns the slot and by the duplicate-model arity — with
/// no child launched and no retained root created under the admitted
/// home (safety / AS3, evidence / LE2; tasks 8.8(d)/8.10).
#[test]
fn the_dsh_admission_rule_reads_the_whole_payload_the_command_line_carried() {
    for (case, argv, field) in [
        (
            "an effort slot claimed by a later model control, trailing terminator",
            vec!["--effort", "--model", "p/m", "high", "--"],
            "--effort",
        ),
        (
            "a second model behind an inner terminator",
            vec!["--model", "p/m", "--", "--model", "second"],
            "--model",
        ),
    ] {
        let fixture = DshFixture::new();
        let seen = fixture.observe(DshPath::Disabled, &argv, None);
        let error = seen.error();
        assert!(error.contains(field), "{case}: the fixed field: {error}");
        seen.assert_no_provider_work(case);
    }
}

/// A residual `--` is refused wherever it stands, on every launch path,
/// before any provider work — through the BUILT binary (R1).
///
/// Clap consumes the outer separator, so every terminator below is one the
/// seat wrote. While the CLI deleted a LEADING one after clap had already
/// taken the boundary, `brokkr driver dsh -- --` and
/// `brokkr driver dsh -- -- --model <id>` reached admission with that token
/// gone and were ADMITTED: each launched a child, staged an overlay,
/// allocated a retained root and checkpointed. The adapter could only
/// refuse what it received. The interior and trailing spellings — which the
/// CLI never touched — refused all along, and the two orderings are held
/// side by side here so a repair that fixed one position cannot pass.
///
/// The last two cases are the ordering proof: a route overlay is BOUND in
/// the input and its file is absent, so an argv that reaches route
/// resolution refuses by the route's own words. The terminator must refuse
/// first from either position, so neither case may name the route at all.
#[test]
fn a_residual_terminator_refuses_on_every_dsh_path_wherever_it_stands() {
    for path in [DshPath::Disabled, DshPath::EnabledCold, DshPath::Offered] {
        for (case, argv, route) in [
            ("a bare terminator alone", vec!["--"], None),
            (
                "a leading terminator before an admissible pin",
                vec!["--", "--model", DSH_PIN],
                None,
            ),
            (
                "an interior terminator between admissible controls",
                vec!["--model", DSH_PIN, "--", "--effort", "medium"],
                None,
            ),
            (
                "a trailing terminator behind admissible controls",
                vec!["--model", DSH_PIN, "--effort", "medium", "--"],
                None,
            ),
            (
                "a leading terminator with a bound, absent route overlay",
                vec!["--", "--model", DSH_PIN, "--patch", BOUND_ROUTE],
                Some(BOUND_ROUTE),
            ),
            (
                "a trailing terminator with a bound, absent route overlay",
                vec!["--model", DSH_PIN, "--patch", BOUND_ROUTE, "--"],
                Some(BOUND_ROUTE),
            ),
        ] {
            let label = format!("{path:?}/{case}");
            let fixture = DshFixture::new();
            let seen = fixture.observe(path, &argv, route);
            let error = seen.error();
            assert!(
                error.contains(TERMINATOR_REASON),
                "{label}: the fixed option-terminator reason: {error}"
            );
            // The route read is downstream of admission: a bound overlay
            // whose file is absent must never get to say so.
            assert!(
                !error.contains("route_overlay"),
                "{label}: the terminator refuses before route resolution: {error}"
            );
            fixture.assert_echoes_nothing(&label, error);
            seen.assert_no_provider_work(&label);
        }
    }
}

/// The three launch paths the terminator proof runs on are the ones it
/// names: the same fixtures, with an admissible payload, actually reach
/// the provider work the refusals above are measured by its absence.
///
/// Each path is identified by an observation only that path produces. A
/// closed gate probes no version at all. An open one probes it and ships
/// the cold route, because the fixture's declared wrapper digest is not
/// the composite this host computes. An open gate handed a session back
/// declines it in the launch row's own words. Without this, the refusals
/// above would pass over a fixture that refused for some unrelated reason.
#[test]
fn the_dsh_launch_paths_the_terminator_proof_runs_on_are_the_ones_it_names() {
    let admissible = ["--model", DSH_PIN, "--effort", "medium"];

    let fixture = DshFixture::new();
    let cold = fixture.observe(DshPath::Disabled, &admissible, None);
    assert!(
        cold.probes.is_empty(),
        "a closed gate probes no version: {:?}",
        cold.probes
    );
    cold.assert_provider_work("disabled");
    assert!(
        cold.launch_row("disabled").get("resume_refusal").is_none(),
        "no offer, no refusal: {:?}",
        cold.out
    );

    let fixture = DshFixture::new();
    let warm = fixture.observe(DshPath::EnabledCold, &admissible, None);
    assert_eq!(
        warm.probes.len(),
        1,
        "an open gate probes the core version once: {:?}",
        warm.probes
    );
    warm.assert_provider_work("enabled-cold");
    let row = warm.launch_row("enabled-cold");
    assert_eq!(row["launch"], "cold", "{row}");
    assert!(
        row.get("resume_refusal").is_none(),
        "no offer, no refusal: {row}"
    );

    let fixture = DshFixture::new();
    let offered = fixture.observe(DshPath::Offered, &admissible, None);
    assert_eq!(
        offered.probes.len(),
        1,
        "an open gate probes the core version once: {:?}",
        offered.probes
    );
    offered.assert_provider_work("offered");
    let row = offered.launch_row("offered");
    assert_eq!(row["launch"], "cold", "{row}");
    assert_eq!(
        row["resume_refusal"], "unverified-harness",
        "the offer reached the gate and was declined: {row}"
    );
}

/// The one shape a log written by the LAUNCHED shim can never report:
/// the driver staged its seat overlay and then never launched anything,
/// because there was nothing to launch. The staging directory says so
/// even though the overlay is gone by the time the driver exits — which
/// is what makes every "no overlay was staged" absence beside it a fact
/// about staging rather than a fact about child execution (#226 task
/// 8.8(d) D1).
#[test]
fn dsh_overlay_staging_is_observed_without_any_child_execution() {
    let admissible = ["--model", DSH_PIN];
    let fixture = DshFixture::without_a_provider();
    let observed = fixture.observe(DshPath::EnabledCold, &admissible, None);
    assert!(
        observed.launches.is_empty() && observed.overlay.is_empty(),
        "no child ran, so the launched shim's own log stays empty: {:?} {:?}",
        observed.launches,
        observed.overlay
    );
    assert!(
        observed.staging_touched,
        "and the staging directory still reports the overlay this driver staged"
    );
    assert!(
        observed.staging_left.is_empty(),
        "which it removed again before exiting: {:?}",
        observed.staging_left
    );
    // The negative control on the same fixture: a payload refused during
    // admission reaches no staging at all, so the two observations tell
    // staging-then-cleanup apart from staging that never happened.
    let refused = fixture.observe(DshPath::EnabledCold, &["--session", "x"], None);
    assert!(
        !refused.staging_touched,
        "an admission refusal stages nothing: {:?}",
        refused.staging_left
    );
    assert_eq!(refused.out.last().expect("a result")["status"], "failed");
}

/// 8.10's deadline case at the seam that actually owns termination: the
/// runtime's own watchdog, killing the real built DSH driver's process
/// tree while its launch rows are still held.
///
/// Design D7 adopts no asynchronous cancel protocol — `serve_io` invokes
/// synchronously and the watchdog owns process termination — so this IS
/// what cancelling or timing out a running DSH seat does to it. The shim
/// answers the version probe, records its launch, and then `exec`s a
/// stall, so the deadline expires with the driver mid-invocation and its
/// pre-session rows still buffered (decision 0053).
///
/// What must survive: the attempt is a determinate deadline failure and
/// says so on the report rather than only in its prose, NOTHING the
/// driver was holding is flushed as a fabricated launch row, locator or
/// `root_session`, and exactly one child was ever launched — the adapter
/// spends no cold replacement on a killed attempt.
///
/// The adapter-side half — the stream-json launch hold itself never
/// releasing under a timer-driven kill — is proved in `brokkr-protocol`'s
/// `a_dsh_deadline_kill_inside_the_open_launch_hold_fabricates_nothing`,
/// because the qualified route is reachable only over an injected
/// composite and no test here installs a provider.
#[test]
fn a_dsh_deadline_kill_flushes_no_held_launch_row_and_starts_no_replacement() {
    use brokkr_protocol::process::{DriverProcess, SpawnEnv};
    use brokkr_protocol::AttemptOutcome;
    use std::time::{Duration, Instant};

    let fixture = DshFixture::stalling();
    let env: std::collections::BTreeMap<String, String> = [
        ("PATH", "/usr/bin:/bin"),
        ("HOME", fixture.operator_home.to_str().unwrap()),
        ("DSH_HOME", fixture.dsh_home.to_str().unwrap()),
        ("TMPDIR", fixture.staging.to_str().unwrap()),
        ("FORGE_DSH_BIN", fixture.binary.to_str().unwrap()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect();
    let command = vec![
        brokkr_bin().to_string(),
        "driver".into(),
        "dsh".into(),
        "--".into(),
        "--model".into(),
        DSH_PIN.into(),
    ];
    // Long enough that the probe, the composite read and the staging all
    // complete first, so the kill lands on the stalled child and not on
    // the planner.
    let deadline = Duration::from_secs(4);
    let started = Instant::now();
    let report = DriverProcess::spawn(
        &command,
        &fixture.workdir,
        Some(deadline),
        &SpawnEnv::Exactly(env),
    )
    .unwrap()
    .run_attempt(
        "test",
        "fx",
        "a1",
        "intake",
        fixture.start_input(DshPath::EnabledCold, None),
        |_| {},
    );
    let elapsed = started.elapsed();

    assert!(
        matches!(&report.outcome, AttemptOutcome::Failed { error } if error.contains("deadline")),
        "{:?}",
        report.outcome
    );
    assert!(
        report.deadline_killed,
        "the watchdog's kill is on the report, not only in its prose"
    );
    assert!(
        elapsed < deadline + Duration::from_secs(20),
        "the kill unblocks the harness inside the deadline and a bounded \
         margin, took {elapsed:?}"
    );
    // The driver really did reach the provider before it was killed, so
    // the absences below measure a held launch and not a start that never
    // got that far.
    assert_eq!(
        fixture.launches().len(),
        1,
        "exactly one child was launched, and no replacement followed it"
    );
    assert_eq!(fixture.probes().len(), 1, "one version probe");
    // And nothing it was holding escaped: decision 0053 buffers the
    // pre-session rows until the first turn, the stalled child produced
    // none, and a deadline kill does not flush them.
    assert!(!report.accepted, "the killed attempt never accepted");
    assert!(
        report.checkpoints.is_empty(),
        "no launch row, no root_session and no transcript locator were \
         fabricated out of a killed attempt: {:?}",
        report.checkpoints
    );
}

/// The model pin every admissible DSH payload here carries, and the route
/// value the ordering cases bind. Neither may appear in a refusal.
const DSH_PIN: &str = "deepseek/deepseek-v4-flash";
const BOUND_ROUTE: &str = "bound-route.yml";
/// The version the owned shim answers a probe with, and the identity the
/// enabled fixtures declare, so the gate opens on a reachable version.
const DSH_SHIM_VERSION: &str = "0.1.5-rc.1";
/// A well-formed wrapper digest no host's composite equals, so an open
/// gate probes the version and then ships the cold route.
const DECLARED_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// The bound route overlay's manifest digest. The file is never created,
/// so this value is only ever reached by an argv that got past admission.
const BOUND_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
/// The fixed category `dsh_control_conflict` returns for a residual `--`,
/// in the sentence the adapter builds around it.
const TERMINATOR_REASON: &str = "carry the option terminator, which the engine owns";
/// The session a fixture offers back on the offered path.
const DSH_OFFER: &str = "session-0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

/// Which launch path a DSH fixture selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DshPath {
    /// The shipped shape: no resume assessment on the input at all, so the
    /// gate is closed and no version is ever probed.
    Disabled,
    /// A supported assessment naming the identity the owned shim answers
    /// with: the gate opens, the version is probed, and the launch is cold
    /// because the declared wrapper digest is not this host's composite.
    EnabledCold,
    /// The same open gate with a session offered back before the start.
    Offered,
}

/// One temporary root, canonicalized once, with its lifetime owner kept
/// beside it.
///
/// Every path a fixture derives — workdir, shim, logs, `DSH_HOME`, the
/// operator's `HOME` — comes off the canonical form. macOS puts the system
/// temporary directory under `/var`, which is a symlink to `/private/var`,
/// so a raw `TempDir` path and the path the driver canonicalizes are two
/// different STRINGS for one directory: containment checks, recorded-home
/// comparisons and "this path never appears" assertions would all be
/// measuring the wrong spelling. The owner is returned because dropping it
/// removes the directory.
fn canonical_root() -> (tempfile::TempDir, PathBuf) {
    let owner = tempfile::tempdir().unwrap();
    let path = std::fs::canonicalize(owner.path()).expect("a temporary root canonicalizes");
    (owner, path)
}

/// What one DSH start actually did to the world: the protocol stream, and
/// the filesystem evidence an error message cannot supply.
struct DshObservations {
    out: Vec<Value>,
    /// One line per `--version` probe the driver ran.
    probes: Vec<String>,
    /// One line per child the driver launched. A LINE, not the argv: the
    /// dsh adapter hands the seat's prompt over as an argv element, and a
    /// prompt carries newlines, so an argv echo could not be counted.
    launches: Vec<String>,
    /// The overlay bytes a launched child was actually handed by `--patch`.
    overlay: String,
    /// Whether ANYTHING was created in the driver's own temporary
    /// directory — the seat overlay's home — independently of whether a
    /// child ever ran.
    ///
    /// The overlay is a `tempfile` that is removed before the driver
    /// exits, so its absence afterwards proves nothing, and the `overlay`
    /// field above is written only by a launched shim, so it cannot tell
    /// staging-then-cleanup apart from staging that never happened. The
    /// directory is stamped into 1990 before each run, and any create or
    /// unlink inside it moves that stamp to now; a transient file is
    /// therefore as visible as a surviving one (#226 task 8.8(d) D1).
    staging_touched: bool,
    /// Whatever the driver left behind in that directory, which must
    /// always be nothing: the overlay outlives the child and no longer.
    staging_left: Vec<PathBuf>,
    /// The seat roots retained under the admitted harness home.
    retained_roots: Vec<PathBuf>,
}

impl DshObservations {
    fn error(&self) -> &str {
        let result = self.out.last().expect("a result");
        assert_eq!(result["type"], "result", "{result}");
        assert_eq!(result["status"], "failed", "{result}");
        result["error"].as_str().unwrap_or_default()
    }

    /// The whole provider-observation set, absent. An admission refusal
    /// reaches none of them, so each absence is a separate fact the error
    /// text alone could not establish.
    fn assert_no_provider_work(&self, label: &str) {
        assert!(
            self.probes.is_empty(),
            "{label}: no version was probed: {:?}",
            self.probes
        );
        assert!(
            self.launches.is_empty(),
            "{label}: no child was launched: {:?}",
            self.launches
        );
        // Independent of whether a child ever ran: the driver's temporary
        // directory was never written in at all, so no overlay was staged
        // — not merely none copied out by a shim that never started.
        assert!(
            !self.staging_touched,
            "{label}: nothing was staged in the driver's temporary directory"
        );
        assert!(
            self.overlay.is_empty(),
            "{label}: and no overlay reached a child: {:?}",
            self.overlay
        );
        assert!(
            self.retained_roots.is_empty(),
            "{label}: no root was retained: {:?}",
            self.retained_roots
        );
        assert!(
            !self
                .out
                .iter()
                .any(|message| message["type"] == "checkpoint"),
            "{label}: a refusal before any provider work checkpoints nothing: {:?}",
            self.out
        );
    }

    /// The same set, present: what an ADMITTED payload does on this
    /// fixture, so the absences above measure a refusal and not a fixture
    /// that could never have reached the provider.
    fn assert_provider_work(&self, label: &str) {
        assert_eq!(
            self.launches.len(),
            1,
            "{label}: one child was launched: {:?}",
            self.launches
        );
        // The positive control for the absence above: this payload DID
        // stage, the staging directory says so, and the file itself is
        // gone by the time anyone can look.
        assert!(
            self.staging_touched,
            "{label}: the driver staged into its temporary directory"
        );
        assert!(
            self.staging_left.is_empty(),
            "{label}: and left nothing behind: {:?}",
            self.staging_left
        );
        assert!(
            self.overlay.contains("root:"),
            "{label}: the child was handed the staged overlay: {:?}",
            self.overlay
        );
        assert_eq!(
            self.retained_roots.len(),
            1,
            "{label}: one root was retained: {:?}",
            self.retained_roots
        );
    }

    fn launch_row(&self, label: &str) -> &Value {
        let row = self
            .out
            .iter()
            .find(|message| {
                message["type"] == "checkpoint" && message["data"]["step"] == "harness-started"
            })
            .unwrap_or_else(|| panic!("{label}: one launch row: {:?}", self.out));
        &row["data"]
    }
}

/// One DSH start over the built binary with caller-owned, canonicalized
/// roots and an owned harmless shim, so a refusal's own evidence — no
/// probe, no child, no staged overlay, no retained root — is observable
/// after the driver exits.
struct DshFixture {
    _workdir: tempfile::TempDir,
    _dsh_home: tempfile::TempDir,
    _operator_home: tempfile::TempDir,
    _evidence: tempfile::TempDir,
    _staging: tempfile::TempDir,
    workdir: PathBuf,
    dsh_home: PathBuf,
    operator_home: PathBuf,
    /// The driver child's own `TMPDIR`, watched for staging.
    staging: PathBuf,
    shim: PathBuf,
    /// What the driver is pointed at as `dsh`. Normally the owned shim;
    /// a fixture may point it at nothing, which stages an overlay and
    /// then never launches a child.
    binary: PathBuf,
    probe_log: PathBuf,
    launch_log: PathBuf,
    overlay_log: PathBuf,
}

/// The stamp every run starts its staging directory from, far enough in
/// the past that no filesystem's timestamp granularity can confuse it
/// with "now" (POSIX `touch -t [[CC]YY]MMDDhhmm`).
const STAGING_STAMP: &str = "199001010000";

impl DshFixture {
    fn new() -> Self {
        Self::pointed_at(None, "exit 1\n")
    }

    /// The same fixture with no reachable `dsh` at all: the driver still
    /// stages its seat overlay and then fails to spawn, which is staging
    /// followed by cleanup with no child execution — the one shape a log
    /// written by a launched shim can never report.
    fn without_a_provider() -> Self {
        Self::pointed_at(Some("dsh-does-not-exist"), "exit 1\n")
    }

    /// The same fixture whose launched shim never returns. `exec` puts the
    /// stall in the process the driver holds, so a deadline kill reaching
    /// the driver's tree reaches it too.
    fn stalling() -> Self {
        Self::pointed_at(None, "exec sleep 120\n")
    }

    fn pointed_at(missing: Option<&str>, ending: &str) -> Self {
        let (workdir_owner, workdir) = canonical_root();
        let (dsh_home_owner, dsh_home) = canonical_root();
        let (operator_home_owner, operator_home) = canonical_root();
        let (evidence_owner, evidence) = canonical_root();
        let (staging_owner, staging) = canonical_root();
        let probe_log = evidence.join("probes");
        let launch_log = evidence.join("launches");
        let overlay_log = evidence.join("overlay");
        // Harmless by construction: it answers a version probe, records
        // what it was asked and what `--patch` handed it, and fails. It
        // never reaches a provider, and a real dsh is unreachable from
        // here — both spellings of the binary override are pinned at it.
        let shim = make_shim(
            &evidence,
            &format!(
                "#!/bin/sh\n\
                 case \"$1\" in --version) printf 'probe\\n' >> '{probes}'; \
                 printf '{version}\\n'; exit 0 ;; esac\n\
                 printf 'launch\\n' >> '{launches}'\n\
                 prev=\n\
                 for a in \"$@\"; do\n\
                 \x20 [ \"$prev\" = --patch ] && cat \"$a\" >> '{overlay}'\n\
                 \x20 prev=$a\n\
                 done\n\
                 {ending}",
                probes = probe_log.display(),
                launches = launch_log.display(),
                overlay = overlay_log.display(),
                version = DSH_SHIM_VERSION,
            ),
        );
        let binary = match missing {
            Some(name) => evidence.join(name),
            None => shim.clone(),
        };
        Self {
            _workdir: workdir_owner,
            _dsh_home: dsh_home_owner,
            _operator_home: operator_home_owner,
            _evidence: evidence_owner,
            _staging: staging_owner,
            workdir,
            dsh_home,
            operator_home,
            staging,
            shim,
            binary,
            probe_log,
            launch_log,
            overlay_log,
        }
    }

    /// Empty the staging directory and stamp it into 1990, so the only
    /// thing that can move its timestamp forward is the driver creating
    /// or removing something inside it.
    fn stamp_staging(&self) {
        for entry in std::fs::read_dir(&self.staging).unwrap() {
            let path = entry.unwrap().path();
            let _ = std::fs::remove_dir_all(&path).or_else(|_| std::fs::remove_file(&path));
        }
        let stamped = Command::new("touch")
            .arg("-t")
            .arg(STAGING_STAMP)
            .arg(&self.staging)
            .status()
            .expect("the host stamps a directory's timestamp");
        assert!(stamped.success(), "the staging directory is stamped");
    }

    /// Whether anything was created or removed in the staging directory
    /// since it was stamped.
    fn staging_touched(&self) -> bool {
        use std::os::unix::fs::MetadataExt;
        let stamped_year_1990 = 700_000_000;
        std::fs::metadata(&self.staging).unwrap().mtime() > stamped_year_1990
    }

    /// The launches and probes the owned shim recorded, readable without
    /// driving a whole `observe`.
    fn launches(&self) -> Vec<String> {
        log_lines(&self.launch_log)
    }

    fn probes(&self) -> Vec<String> {
        log_lines(&self.probe_log)
    }

    /// The seat input one DSH start carries on the given path.
    fn start_input(&self, path: DshPath, route: Option<&str>) -> Value {
        let mut resume_context = serde_json::Map::new();
        if path != DshPath::Disabled {
            resume_context.insert(
                "assessment".into(),
                json!({"headless-work": {
                    "status": "supported",
                    "identity": {
                        "version": DSH_SHIM_VERSION,
                        "applies_to": DSH_SHIM_VERSION,
                        "wrapper_digest": DECLARED_DIGEST,
                    },
                    "classes": ["work"],
                    "boundaries": ["not applicable"],
                    "hands": "none",
                    "evidence": {"interface": "fixture", "accounting": "fixture"},
                }}),
            );
        }
        if let Some(value) = route {
            resume_context.insert(
                "route_overlay".into(),
                json!({"value": value, "digest": BOUND_DIGEST}),
            );
        }
        let input = json!({
            "feature": "admission", "phase": "intake", "seat": "intake",
            "role_path": self.workdir.join("missing-role.md"),
            "workdir": self.workdir,
            "result_path": self.workdir.join("results/fx.json"),
            "allowed_results": ["resolved"], "context": {},
            // The engine's own facts for an inline no-hands site, which
            // the gate requires before it will open (design D10 F1).
            "boundary": "not applicable",
            "hands": "none",
            "resume_context": Value::Object(resume_context),
        });
        input
    }

    /// Drive one start on the given path with the given payload, and read
    /// back everything it touched.
    fn observe(&self, path: DshPath, extra: &[&str], route: Option<&str>) -> DshObservations {
        let input = self.start_input(path, route);
        let mut messages = vec![json!({"proto": "forge-driver/v1", "msg_id": "m1",
                                       "type": "hello", "engine_version": "test"})];
        if path == DshPath::Offered {
            messages.push(
                json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "resume",
                                 "effect_id": "fx", "attempt_id": "a1",
                                 "session_ref": DSH_OFFER}),
            );
        }
        messages.push(
            json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "start",
                             "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
                             "input": input}),
        );
        messages.push(json!({"proto": "forge-driver/v1", "msg_id": "m4", "type": "shutdown"}));

        self.stamp_staging();
        let mut child = Command::new(brokkr_bin())
            .arg("driver")
            .arg("dsh")
            .arg("--")
            .args(extra)
            // Pinned at the shim, and the newer spelling removed, for the
            // reason `drive` states: no conformance run may reach a real
            // dsh.
            .env_remove("BROKKR_DSH_BIN")
            .env("FORGE_DSH_BIN", &self.binary)
            .env("HOME", &self.operator_home)
            .env("DSH_HOME", &self.dsh_home)
            // The seat overlay is staged under the driver's own TMPDIR,
            // which is this fixture's to watch.
            .env("TMPDIR", &self.staging)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for message in &messages {
            writeln!(stdin, "{message}").unwrap();
        }
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        DshObservations {
            out: String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect(),
            probes: log_lines(&self.probe_log),
            launches: log_lines(&self.launch_log),
            overlay: std::fs::read_to_string(&self.overlay_log).unwrap_or_default(),
            staging_touched: self.staging_touched(),
            staging_left: std::fs::read_dir(&self.staging)
                .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
                .unwrap_or_default(),
            retained_roots: match std::fs::read_dir(self.dsh_home.join("sessions").join("brokkr")) {
                Ok(entries) => entries.map(|entry| entry.unwrap().path()).collect(),
                Err(_) => Vec::new(),
            },
        }
    }

    /// A refusal names its fixed category and nothing of the seat's own
    /// argv or of this machine: no model, no route value, no session id,
    /// no path — and no long-option spelling at all, which is the bound
    /// that covers the refused token itself and every control beside it
    /// (AS3; tasks 8.8(d)/8.10).
    fn assert_echoes_nothing(&self, label: &str, error: &str) {
        let roots = [
            &self.workdir,
            &self.dsh_home,
            &self.operator_home,
            &self.shim,
        ]
        .map(|path| path.to_string_lossy().into_owned());
        for secret in [DSH_PIN, BOUND_ROUTE, DSH_OFFER, "--"]
            .map(str::to_string)
            .into_iter()
            .chain(roots)
        {
            assert!(
                !error.contains(&secret),
                "{label}: the diagnostic echoes {secret:?}: {error}"
            );
        }
    }
}

/// One log file's lines, empty when the shim never wrote it.
fn log_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

#[test]
fn exec_refusal_is_the_scripts_own_failure_not_a_provider_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), REFUSING_STDERR_SHIM);
    let out = drive(&["exec", "--", shim.to_str().unwrap()], &shim, dir.path());
    assert_accepted_failure(&out, "exec");
}

fn make_shim(dir: &Path, body: &str) -> PathBuf {
    make_named_shim(dir, "shim", body)
}

/// The house fix for #255, both halves. The shim is installed by RENAME
/// from a temporary sibling, so the pathname anything execs never names a
/// partially written file — and there is no retry loop. And the sibling's
/// bytes are written by a CHILD, so this process never holds a write
/// descriptor on the inode the rename delivers: `exec` refuses a file any
/// process still holds open for writing, `rename` moves the inode with
/// that write count intact, and a forked child inherits this thread's
/// descriptor until it execs. Both facts are measured in
/// `brokkr-protocol`'s own
/// `a_renamed_shim_inherits_its_writer_and_a_staged_one_carries_none`.
fn make_named_shim(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static STAGED: AtomicU64 = AtomicU64::new(0);
    let path = dir.join(name);
    // Short and unique within this run rather than derived from the
    // destination's name, so a fixture probing a length bound can never
    // fail on the staging name instead of on its own subject.
    let staging = dir.join(format!(
        ".stage-{}-{}",
        std::process::id(),
        STAGED.fetch_add(1, Ordering::Relaxed)
    ));
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg("cat > \"$0\"")
        .arg(&staging)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success(), "the shim is staged");
    let mut permissions = std::fs::metadata(&staging).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&staging, permissions).unwrap();
    std::fs::rename(&staging, &path).expect("the staged shim is renamed into place");
    path
}

fn drive(kind_args: &[&str], shim: &Path, workdir: &Path) -> Vec<Value> {
    // Every harness home is test-owned. Conformance must never make a
    // driver name (or create under) the operator's real transcript home.
    let operator_home = tempfile::tempdir().unwrap();
    let codex_home = tempfile::tempdir().unwrap();
    let dsh_home = tempfile::tempdir().unwrap();
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let mut child = Command::new(brokkr_bin())
        .arg("driver")
        .args(kind_args)
        .env("BROKKR_CLAUDE_BIN", shim)
        // Pinned unconditionally: no conformance test may ever spawn a
        // real claude-lanetally on a LaneTally-equipped machine.
        .env("BROKKR_LANETALLY_BIN", shim)
        // Deliberately split: codex is pinned through the new spelling
        // and dsh through the old one, so a conformance run proves both
        // reach the same adapter for the release the old names survive
        // (decision 0019). The new dsh spelling is REMOVED rather than
        // left inherited: it outranks the old one, so an operator who
        // has it exported would otherwise send this test at a real dsh.
        .env("BROKKR_CODEX_BIN", shim)
        .env_remove("BROKKR_DSH_BIN")
        .env("FORGE_DSH_BIN", shim)
        .env("HOME", operator_home.path())
        .env("CODEX_HOME", codex_home.path())
        .env("DSH_HOME", dsh_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&result_path);
    let parsed: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // A transcript row proves the selected harness home, not merely the
    // vocabulary. Invalid invocations can refuse before a driver exists.
    if let Some(row) = parsed
        .iter()
        .find(|message| message.pointer("/data/step").and_then(Value::as_str) == Some("transcript"))
    {
        let transcript = &row["data"]["transcript"];
        let (kind, expected_home) = match kind_args[0] {
            "claude" | "lanetally" => (
                "claude-session",
                operator_home.path().join(".claude/projects"),
            ),
            "codex" => ("codex-thread", codex_home.path().to_path_buf()),
            "dsh" => ("dsh-session", dsh_home.path().to_path_buf()),
            "exec" => ("none", PathBuf::new()),
            other => panic!("unrecognized conformance driver {other}"),
        };
        assert_eq!(transcript["kind"], kind);
        assert_eq!(transcript["home"], expected_home.to_string_lossy().as_ref());
    }
    parsed
}

fn all_adapters(shim: &Path) -> Vec<(&'static str, Vec<String>)> {
    vec![
        (
            "claude",
            vec![
                "claude".into(),
                "--".into(),
                "--model".into(),
                "claude-fable-5-1".into(),
            ],
        ),
        (
            "lanetally",
            vec![
                "lanetally".into(),
                "--".into(),
                "--model".into(),
                "claude-fable-5-1".into(),
            ],
        ),
        (
            "codex",
            vec![
                "codex".into(),
                "--".into(),
                "--model".into(),
                "gpt-5.6-sol".into(),
            ],
        ),
        (
            "dsh",
            vec![
                "dsh".into(),
                "--".into(),
                "--model".into(),
                "deepseek/deepseek-v4-flash".into(),
                "--effort".into(),
                "medium".into(),
            ],
        ),
        (
            "exec-stdin",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
            ],
        ),
        (
            "exec-promptfile",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
                "{prompt_file}".into(),
            ],
        ),
    ]
}

/// This engine writes seat-record v2, so conformance judges what it
/// writes against v2 — the version its own runs declare (decision 0035
/// ruling 7). A driver whose records only satisfied v1 would still pass
/// v2; the assertions below are what make the new fields non-optional
/// for a BUILT-IN, which is where ruling 3's completeness lives.
fn assert_seat_records_conform(messages: &[Value], label: &str, case: &str) {
    for message in messages {
        if message["type"] == "checkpoint" {
            let record = &message["data"];
            validate_seat_record(record, 0, SeatRecordVersion::V2)
                .unwrap_or_else(|error| panic!("{label}/{case}: {error}: {record}"));
            assert!(
                record.get("model").and_then(Value::as_str).is_some(),
                "{label}/{case}: every current checkpoint carries decision 0031 model evidence: {record}"
            );
            assert!(
                record.get("effort").and_then(Value::as_str).is_some(),
                "{label}/{case}: every current checkpoint carries decision 0035 \
                 effort configuration, sentinel included: {record}"
            );
        }
        if message["type"] == "result" && message["status"] == "succeeded" {
            let record = &message["result"];
            validate_seat_record(record, 0, SeatRecordVersion::V2)
                .unwrap_or_else(|error| panic!("{label}/{case}: {error}: {record}"));
            assert!(record.get("model").and_then(Value::as_str).is_some());
            assert!(record.get("effort").and_then(Value::as_str).is_some());
            assert!(record
                .get("transcript")
                .and_then(Value::as_object)
                .is_some());
        }
    }
}

#[test]
fn the_shipped_cold_transcript_name_keeps_its_seat_telemetry() {
    // The regression this guards: admitting version three moved the one
    // shared transcript name, and the shipped cold route read it too, so
    // a disabled or mismatched launch silently lost every seat turn —
    // succeeding with no model, no effort and no token totals. Discovery
    // now reads both generations, and this drives the built binary to
    // prove it end to end rather than at the unit seam.
    for (name, shim_body) in [
        ("session.v3.jsonl", DSH_USAGE_SHIM),
        ("session.jsonl", DSH_SHIPPED_USAGE_SHIM),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(dir.path(), shim_body);
        let args: Vec<&str> = vec![
            "dsh",
            "--",
            "--model",
            "deepseek/deepseek-v4-flash",
            "--effort",
            "medium",
        ];
        let out = drive(&args, &shim, dir.path());
        let served: Vec<&Value> = out
            .iter()
            .filter(|m| m["type"] == "checkpoint")
            .filter(|m| m["data"]["model"] == "deepseek-v4-flash")
            .collect();
        assert!(
            !served.is_empty(),
            "{name}: no seat turn named what served: {out:?}"
        );
        assert!(
            served
                .iter()
                .any(|m| m["data"]["input_tokens"] == 13 && m["data"]["output_tokens"] == 3),
            "{name}: the seat turn carries no token totals: {served:?}"
        );
        assert!(
            served.iter().any(|m| m["data"]["effort"] == "medium"),
            "{name}: the seat turn carries no effort: {served:?}"
        );
    }
}

#[test]
fn conformance_across_all_builtin_adapters() {
    for case in ["obedient", "silent"] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(
            dir.path(),
            if case == "obedient" {
                OBEDIENT_SHIM
            } else {
                SILENT_SHIM
            },
        );
        let claude_dir = dir.path().join("claude-stream");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let claude_shim = make_shim(&claude_dir, CLAUDE_STREAM_SHIM);
        let codex_dir = dir.path().join("codex-json");
        std::fs::create_dir_all(&codex_dir).unwrap();
        let codex_shim = make_shim(&codex_dir, CODEX_JSON_SHIM);
        let dsh_dir = dir.path().join("dsh-usage");
        std::fs::create_dir_all(&dsh_dir).unwrap();
        let dsh_shim = make_shim(&dsh_dir, DSH_USAGE_SHIM);
        for (label, args) in all_adapters(&shim) {
            // The claude adapter streams its session: its obedient shim
            // speaks stream-json and yields three seat-turn checkpoints
            // (one per tool_use block, the last two sharing a turn)
            // before the session-finished one.
            let claude = label == "claude";
            // The lanetally leg reuses CLAUDE_STREAM_SHIM verbatim: the
            // reuse IS the argv-compatibility proof — the wrapper is
            // driven exactly as the claude binary would be.
            let lanetally = label == "lanetally";
            let codex = label == "codex";
            let dsh = label == "dsh";
            let exec = label.starts_with("exec");
            let shim = if (claude || lanetally) && case == "obedient" {
                &claude_shim
            } else if codex && case == "obedient" {
                &codex_shim
            } else if dsh && case == "obedient" {
                &dsh_shim
            } else {
                &shim
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = drive(&args, shim, dir.path());
            let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
            let expected: &[&str] = if (claude || lanetally) && case == "obedient" {
                // Transcript, the launch row proposed decision 0056
                // ruling 7 requires of every model adapter, three tool
                // turns, session-finished. The launch row is what issue
                // #226 found missing here: claude and dsh reported no
                // launch field at all.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if codex && case == "obedient" {
                // Harness launch, transcript, and the four live codex
                // fold rows before session-finished.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if dsh && case == "obedient" {
                // Transcript, harness-started, one seat-turn naming what
                // served, then the finishing checkpoint.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if dsh || exec || codex {
                // Each still emits an explicit transcript shape: dsh
                // has a retained root, exec says none, and a silent
                // codex has an empty locator.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else {
                // A silent Claude-shaped stream still reports the common
                // transcript shape and its own launch row before the
                // finishing checkpoint. The launch is a fact about what
                // this adapter DID — it took the fresh-session path —
                // so a stream that said nothing does not erase it.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            };
            assert_eq!(kinds, expected, "{label}/{case}: {kinds:?}");
            for message in &out {
                assert_eq!(message["proto"], "forge-driver/v1", "{label}");
            }
            assert_seat_records_conform(&out, label, case);
            let transcript_rows: Vec<&Value> = out
                .iter()
                .filter(|message| {
                    message.pointer("/data/step").and_then(Value::as_str) == Some("transcript")
                })
                .collect();
            assert_eq!(
                transcript_rows.len(),
                1,
                "{label}/{case}: exactly one common transcript row: {out:?}"
            );
            let transcript = &transcript_rows[0]["data"]["transcript"];
            let expected_kind = if claude || lanetally {
                "claude-session"
            } else if codex {
                "codex-thread"
            } else if dsh {
                "dsh-session"
            } else {
                "none"
            };
            assert_eq!(transcript["kind"], expected_kind, "{label}/{case}");
            let locator = transcript["locator"].as_str().unwrap();
            assert!(locator.chars().count() <= 80, "{label}/{case}: {locator}");
            assert!(
                transcript.get("content").is_none(),
                "{label}/{case}: paths or ids only: {transcript}"
            );
            if dsh {
                let locator_path = Path::new(locator);
                assert!(
                    locator_path.starts_with(Path::new("sessions").join("brokkr"))
                        && locator_path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with("seat-")),
                    "{label}/{case}: {locator}"
                );
                assert!(!locator_path.is_absolute(), "{label}/{case}: {locator}");
            } else if exec || case == "silent" {
                assert_eq!(locator, "", "{label}/{case}");
            } else if codex {
                assert_eq!(locator, "codex-thread-1", "{label}/{case}");
            } else {
                assert_eq!(locator, "stream-1", "{label}/{case}");
            }
            if claude && case == "obedient" {
                assert_eq!(
                    out[2]["data"]["step"], "transcript",
                    "{label}: the locator is journaled at init: {}",
                    out[2]
                );
                // The launch row rides directly behind the locator, one
                // per executing model site (proposed decision 0056
                // ruling 7). No offer was made here, so the launch is
                // cold, it names no refusal — a reason without an offer
                // would be invented — and it carries no root, because
                // this shape's assessment is not enabled and no version
                // was observed to record one with. Issue #226's
                // complaint was that this row did not exist.
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "harness-started", "harness": "claude",
                           "launch": "cold", "model": "not reported",
                           "effort": "not reported"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs", "model": "claude-fable-5-1",
                           "effort": "xhigh",
                           "input_tokens":13, "output_tokens":2,
                           "cache_read_tokens":3, "cache_write_tokens":4}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs", "model": "claude-fable-5-1",
                           "effort": "high",
                           "input_tokens":15, "output_tokens":3,
                           "cache_read_tokens":10}),
                    "{label}: {}",
                    out[5]
                );
                assert_eq!(
                    out[6]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs", "model": "claude-fable-5-1",
                           "effort": "high"}),
                    "{label}: {}",
                    out[6]
                );
                let finished = &out[7]["data"];
                assert_eq!(finished["step"], "claude-code-session-finished", "{label}");
                assert_eq!(finished["transcript"], *transcript, "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
                assert_eq!(finished["model"], "claude-fable-5-1", "{label}");
                assert_eq!(finished["effort"], "high", "{label}");
                assert_eq!(finished["input_tokens"], 28, "{label}");
                assert_eq!(finished["output_tokens"], 5, "{label}");
                assert_eq!(finished["cache_read_tokens"], 13, "{label}");
                assert_eq!(finished["cache_write_tokens"], 4, "{label}");
                // Claude reports its thinking ONLY in the result, so the
                // session record carries it and not one turn does. That
                // absence is decision 0035 ruling 4 in the journal, not
                // an omission: never zero, never back-filled per turn.
                assert_eq!(finished["reasoning_output_tokens"], 4, "{label}");
                for turn in &out[4..7] {
                    assert!(
                        turn["data"].get("reasoning_output_tokens").is_none(),
                        "{label}: a claude turn invents no reasoning count: {turn}"
                    );
                }
                // The capture guard is kind-scoped: only lanetally's
                // finished checkpoint ever carries the marker.
                assert!(finished.get("capture").is_none(), "{label}: {finished}");
            } else if lanetally && case == "obedient" {
                // Same stream, same fold, same disciplines as claude —
                // plus the constant ledger-capture marker and the
                // list-price cost flowing through unchanged.
                assert_eq!(out[2]["data"]["step"], "transcript", "{label}: {}", out[2]);
                // The launch row rides directly behind the locator, one
                // per executing model site (proposed decision 0056
                // ruling 7). No offer was made here, so the launch is
                // cold, it names no refusal — a reason without an offer
                // would be invented — and it carries no root, because
                // this shape's assessment is not enabled and no version
                // was observed to record one with. Issue #226's
                // complaint was that this row did not exist.
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "harness-started", "harness": "claude",
                           "launch": "cold", "model": "not reported",
                           "effort": "not reported"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs", "model": "claude-fable-5-1",
                           "effort": "xhigh",
                           "input_tokens":13, "output_tokens":2,
                           "cache_read_tokens":3, "cache_write_tokens":4}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs", "model": "claude-fable-5-1",
                           "effort": "high",
                           "input_tokens":15, "output_tokens":3,
                           "cache_read_tokens":10}),
                    "{label}: {}",
                    out[5]
                );
                assert_eq!(
                    out[6]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs", "model": "claude-fable-5-1",
                           "effort": "high"}),
                    "{label}: {}",
                    out[6]
                );
                let finished = &out[7]["data"];
                assert_eq!(
                    finished["step"], "claude-lanetally-session-finished",
                    "{label}"
                );
                assert_eq!(finished["effort"], "high", "{label}");
                assert_eq!(finished["reasoning_output_tokens"], 4, "{label}");
                assert_eq!(finished["capture"], "lanetally", "{label}: {finished}");
                assert_eq!(finished["transcript"], *transcript, "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
                assert_eq!(finished["model"], "claude-fable-5-1", "{label}");
                assert_eq!(finished["input_tokens"], 28, "{label}");
                assert_eq!(finished["output_tokens"], 5, "{label}");
                assert_eq!(finished["cache_read_tokens"], 13, "{label}");
                assert_eq!(finished["cache_write_tokens"], 4, "{label}");
            } else if codex && case == "obedient" {
                // The locator comes first now and the launch row behind
                // it, because proposed decision 0056 ruling 7 publishes
                // a launch only once the harness has named its own
                // session — which is the same moment the locator is
                // recorded. Before this change the row was emitted
                // before the child even spawned, which said `resumed`
                // ahead of any evidence that a rejoin had happened.
                assert_eq!(
                    out[2]["data"],
                    json!({"step":"transcript", "transcript": transcript,
                           "model":"not reported", "effort":"not reported"}),
                    "{label}: {}",
                    out[2]
                );
                // Nobody offered this attempt a session, so the launch
                // is cold and says so with no reason to give: a reason
                // exists only where an offer could not be taken. No
                // root either — the shape's assessment is not enabled,
                // so no version was observed to record one with.
                assert_eq!(
                    out[3]["data"],
                    json!({"step":"harness-started", "harness":"codex", "launch":"cold",
                           "model":"not reported", "effort":"not reported"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step":"turn-started", "turn":1, "harness":"codex",
                           "model":"not reported", "effort":"not reported"})
                );
                assert_eq!(out[5]["data"]["tool"], "command_execution");
                assert!(out[5]["data"].get("command").is_none());
                assert_eq!(out[7]["data"]["input_tokens"], 21);
                assert_eq!(out[7]["data"]["cache_read_tokens"], 8);
                assert_eq!(out[7]["data"]["model"], "gpt-5.6-sol");
                // Both read from the thread record — not from the
                // stream, which names neither, and not from the pin the
                // seat was launched with: the argv above says
                // `--model gpt-5.6-sol` and no effort at all, and the
                // record is what the harness itself echoed back.
                assert_eq!(out[7]["data"]["effort"], "xhigh");
                // The two counts codex reported all along and the fold
                // dropped until decision 0035 ruling 4 asked for them.
                assert_eq!(out[7]["data"]["cache_write_tokens"], 6);
                assert_eq!(out[7]["data"]["reasoning_output_tokens"], 3);
                assert_eq!(out[8]["data"]["transcript"], *transcript);
                assert_eq!(out[8]["data"]["output_tokens"], 5);
                assert_eq!(out[8]["data"]["model"], "gpt-5.6-sol");
                assert_eq!(out[8]["data"]["effort"], "xhigh");
                assert_eq!(out[8]["data"]["cache_write_tokens"], 6);
                assert_eq!(out[8]["data"]["reasoning_output_tokens"], 3);
            } else if dsh {
                assert_eq!(out[2]["data"]["step"], "transcript");
                assert_eq!(out[3]["data"]["step"], "harness-started");
                assert_eq!(out[3]["data"]["harness"], "deepseek");
                let finished_index = if case == "obedient" {
                    assert_eq!(out[4]["data"]["step"], "seat-turn");
                    assert_eq!(out[4]["data"]["model"], "deepseek-v4-flash");
                    assert_eq!(out[4]["data"]["input_tokens"], 13);
                    5
                } else {
                    4
                };
                assert_eq!(
                    out[finished_index]["data"]["step"],
                    "deepseek-harness-session-finished"
                );
                assert_eq!(out[finished_index]["data"]["transcript"], *transcript);
                assert_eq!(
                    out[finished_index]["data"]["model"],
                    if case == "obedient" {
                        "deepseek-v4-flash"
                    } else {
                        "not reported"
                    }
                );
                // Decision 0035 ruling 3, as its addendum reads it for
                // dsh: the rows written before the first request went
                // out say `not reported`, and every row after carries
                // the level the harness's own header echoed — which is
                // the pin above only because the shim read it from the
                // seat's settings document, never because the driver
                // copied it. The silent shim echoes nothing, and says so
                // on every row.
                let echoed = if case == "obedient" {
                    "medium"
                } else {
                    "not reported"
                };
                assert_eq!(out[2]["data"]["effort"], "not reported", "{label}");
                assert_eq!(out[3]["data"]["effort"], "not reported", "{label}");
                for row in out[4..].iter().filter(|m| m["type"] == "checkpoint") {
                    assert_eq!(row["data"]["effort"], echoed, "{label}: {row}");
                }
                assert!(
                    out.iter()
                        .all(|m| m.pointer("/data/reasoning_output_tokens").is_none()),
                    "{label}: the headless dsh profile reports no reasoning at all"
                );
            } else if exec {
                // Exec has no model usage or transcript. Its command is
                // deliberately not a seat-record target: targets are
                // file paths belonging to numbered tool turns.
                assert_eq!(out[2]["data"]["step"], "transcript", "{label}");
                assert_eq!(out[3]["data"]["step"], "exec-started", "{label}");
                assert!(out[3]["data"].get("target").is_none(), "{label}");
                assert_eq!(out[out.len() - 2]["data"]["transcript"], *transcript);
                assert_eq!(out[out.len() - 2]["data"]["model"], "not applicable");
                // No model turn, therefore no effort to configure for
                // one: the other sentinel, and the distinction between
                // the two is the one dsh above makes visible.
                for row in out.iter().filter(|m| m["type"] == "checkpoint") {
                    assert_eq!(row["data"]["effort"], "not applicable", "{label}: {row}");
                }
            }
            assert_eq!(
                out[out.len() - 2]["data"]["transcript"],
                *transcript,
                "{label}/{case}: finishing session_meta repeats the journal locator"
            );
            let result = out.last().unwrap();
            if case == "obedient" {
                assert_eq!(result["status"], "succeeded", "{label}: {result}");
                assert_eq!(result["result"]["result"], "resolved", "{label}");
                let expected_model = if claude || lanetally {
                    "claude-fable-5-1"
                } else if codex {
                    "gpt-5.6-sol"
                } else if dsh {
                    "deepseek-v4-flash"
                } else {
                    "not applicable"
                };
                assert_eq!(
                    result["result"]["model"], expected_model,
                    "{label}: {result}"
                );
                // The successful result carries the hire's effort beside
                // the model it claims (decision 0035 ruling 3), each
                // driver reporting what its own harness gave it.
                let expected_effort = if claude || lanetally {
                    "high"
                } else if codex {
                    "xhigh"
                } else if dsh {
                    // Off the request header the shim echoed, which
                    // read it from the seat's settings document — the
                    // pin's whole road, walked (decision 0035 addendum).
                    "medium"
                } else {
                    "not applicable"
                };
                assert_eq!(
                    result["result"]["effort"], expected_effort,
                    "{label}: {result}"
                );
                if claude || lanetally {
                    assert_eq!(result["result"]["input_tokens"], 28, "{label}");
                    assert_eq!(result["result"]["output_tokens"], 5, "{label}");
                    assert_eq!(result["result"]["cache_read_tokens"], 13, "{label}");
                    assert_eq!(result["result"]["cache_write_tokens"], 4, "{label}");
                    assert_eq!(result["result"]["reasoning_output_tokens"], 4, "{label}");
                }
                if codex {
                    assert_eq!(result["result"]["cache_write_tokens"], 6, "{label}");
                    assert_eq!(result["result"]["reasoning_output_tokens"], 3, "{label}");
                }
                if dsh {
                    assert!(
                        result["result"].get("reasoning_output_tokens").is_none(),
                        "{label}: {result}"
                    );
                }
            } else {
                assert_eq!(result["status"], "failed", "{label}: {result}");
                assert!(
                    result["error"].as_str().unwrap().contains("no result file"),
                    "{label}: {result}"
                );
            }
        }
    }
}

#[test]
fn adapters_name_themselves_and_exec_requires_template() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), OBEDIENT_SHIM);
    let expected = [
        ("claude", "claude-code"),
        ("lanetally", "claude-lanetally"),
        ("codex", "codex"),
        ("dsh", "deepseek-harness"),
        ("exec-stdin", "exec"),
        ("exec-promptfile", "exec"),
    ];
    for ((label, args), (_, name)) in all_adapters(&shim).iter().zip(expected) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = drive(&args, &shim, dir.path());
        assert_eq!(out[0]["driver"], name, "{label}");
    }
    let out = drive(&["exec"], &shim, dir.path());
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("command template"));
}

// Records its argv, then emits the claude stream shape with an
// adversarial `"capture":"evil"` smuggled into the result event: the
// run_seat source literal must still win on the finished checkpoint.
const LANETALLY_ADVERSARIAL_SHIM: &str = r#"#!/bin/sh
printf '%s\n' "$*" > lanetally-argv.txt
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"adv-1"}\n'
printf '{"type":"result","num_turns":1,"total_cost_usd":0.5,"capture":"evil"}\n'
"#;

#[test]
fn lanetally_argv_is_claude_shaped_and_the_capture_constant_survives_adversarial_streams() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), LANETALLY_ADVERSARIAL_SHIM);
    let out = drive(
        &["lanetally", "--", "--model", "claude-fable-5-1"],
        &shim,
        dir.path(),
    );
    // The wrapper is invoked exactly as claude would be: the stream-json
    // argv, prompt on stdin (the shim finds the result path only if the
    // prompt arrived there — the succeeded result below proves it).
    let argv = std::fs::read_to_string(dir.path().join("lanetally-argv.txt")).unwrap();
    assert_eq!(
        argv.trim_end(),
        "-p --output-format stream-json --verbose --model claude-fable-5-1"
    );
    let finished = &out[out.len() - 2]["data"];
    assert_eq!(finished["step"], "claude-lanetally-session-finished");
    // The constant is inserted after the session_meta extend: no
    // stream-derived key can shadow it.
    assert_eq!(finished["capture"], "lanetally", "{finished}");
    assert_eq!(finished["total_cost_usd"], 0.5, "{finished}");
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
}

// ------------------------------------------------------------------
// Sealed secret bindings (decision 0012): injection discipline and the
// masking choke point, exercised against the real exec adapter — and,
// for the result-payload surface, against the lanetally adapter through
// the same shared run_seat choke point.
// ------------------------------------------------------------------

const SECRET_VALUE: &str = "tok3n+v4lue!7";

/// Writes what it can see: the injected environment, the literal argv
/// it was handed, and a stderr leak of the value.
const SECRET_PROBE_SHIM: &str = r#"#!/bin/sh
cat > /dev/null
printf '%s' "$API_TOKEN" > env.txt
printf '%s' "$UNREF" > unref.txt
printf '%s' "$1" > argv.txt
printf 'stderr-leak %s\n' "$API_TOKEN" 1>&2
"#;

/// Honors the result contract but echoes the injected value into its
/// result notes — the leg of the choke point that would otherwise ride
/// EffectSucceeded into the append-only journal.
const NOTES_LEAK_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{"result": "resolved", "notes": "leaked %s"}' "$API_TOKEN" > "$target"
"#;

fn write_store(dir: &Path, lines: &str) -> PathBuf {
    let store = dir.join("secrets.env");
    std::fs::write(&store, lines).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    store
}

/// Drive an adapter over one attempt with secret bindings in the start
/// input, returning the protocol messages and the driver's own stderr
/// (which carries the masked child-stderr re-emit). `driver_args` is
/// everything after `brokkr driver` — e.g. `["exec", "--", template…]`
/// or `["lanetally"]`.
fn drive_with_secrets(
    driver_args: &[&str],
    workdir: &Path,
    store: &Path,
    secrets: &[&str],
    parent_env: &[(&str, &str)],
) -> (Vec<Value>, String) {
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
        "secrets": secrets,
        "secrets_file": store,
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let operator_home = tempfile::tempdir().unwrap();
    let codex_home = tempfile::tempdir().unwrap();
    let dsh_home = tempfile::tempdir().unwrap();
    let mut command = Command::new(brokkr_bin());
    command.arg("driver").args(driver_args);
    command.env("HOME", operator_home.path());
    command.env("CODEX_HOME", codex_home.path());
    command.env("DSH_HOME", dsh_home.path());
    for (key, value) in parent_env {
        command.env(key, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (parsed, String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn exec_injects_via_env_only_and_masks_the_stderr_reemit() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(
        dir.path(),
        &format!("API_TOKEN={SECRET_VALUE}\nUNREF=unref-value-99\n"),
    );
    let (out, stderr) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap(), "{{secret:API_TOKEN}}"],
        dir.path(),
        &store,
        &["API_TOKEN", "UNREF"],
        // A pre-existing child env entry: the declared secret must win.
        &[("API_TOKEN", "from-parent-env")],
    );
    // Env injection carried the value; the declared name overrode the
    // inherited entry.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("env.txt")).unwrap(),
        SECRET_VALUE
    );
    // Every DECLARED name injects, referenced in the template or not.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("unref.txt")).unwrap(),
        "unref-value-99"
    );
    // argv carries the literal shell reference, never the value.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("argv.txt")).unwrap(),
        "$API_TOKEN"
    );
    // Exec command templates stay in the pinned manifest, never in the
    // prose-free seat record.
    assert_eq!(out[2]["data"]["step"], "transcript");
    assert!(out[3]["data"].get("target").is_none());
    assert!(!serde_json::to_string(&out[3])
        .unwrap()
        .contains(SECRET_VALUE));
    // The child's stderr leak reaches the driver's re-emit masked.
    assert!(stderr.contains("[secret:API_TOKEN]"), "stderr: {stderr}");
    assert!(!stderr.contains(SECRET_VALUE), "stderr: {stderr}");
}

#[test]
fn exec_missing_secret_refuses_before_spawn_naming_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(dir.path(), "OTHER=some-other-value\n");
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        vec!["capabilities", "accepted", "result"],
        "no checkpoint, no spawn: {kinds:?}"
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    let error = result["error"].as_str().unwrap();
    assert!(error.contains("API_TOKEN"), "{error}");
    assert!(error.contains("secrets.env"), "{error}");
    assert!(
        !error.contains("some-other-value"),
        "never the contents: {error}"
    );
    assert!(
        !dir.path().join("env.txt").exists(),
        "the child must never have spawned"
    );
}

#[test]
fn exec_masks_the_child_written_result_payload() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), NOTES_LEAK_SHIM);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}

#[test]
fn lanetally_result_payload_reaches_the_shared_masking_choke_point() {
    // The claude/lanetally arm injects no secret env (bindings feed only
    // the exec arm's spawn), so a shim echoing $API_TOKEN would leak
    // nothing and prove nothing: the store's known plaintext is baked
    // literally into the result notes instead, and run_seat's shared
    // known-plaintext masking — zero lanetally-specific code — must
    // catch it on the way into the journal.
    let dir = tempfile::tempdir().unwrap();
    let leak_shim = format!(
        r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{{"result": "resolved", "notes": "leaked {SECRET_VALUE}"}}' > "$target"
printf '{{"type":"result","num_turns":1,"total_cost_usd":0.0}}\n'
"#
    );
    let shim = make_shim(dir.path(), &leak_shim);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["lanetally"],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[("BROKKR_LANETALLY_BIN", shim.to_str().unwrap())],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}

/// Design D7's terminal half through the real driver protocol: a resumed
/// invocation the provider answers with a DIFFERENT root — or with none —
/// is `failed` even when the child exits zero and writes the current
/// attempt's result. There is no accepted success, no guessed launch row
/// and no replacement; the delivered file is left for diagnosis.
#[test]
fn a_resumed_mismatch_is_never_an_accepted_success() {
    let other = "01a06183-0000-0000-0000-000000000000";
    let offered = "019c4b7e-0000-0000-0000-000000000001";
    for (case, announced) in [("a different root", other), ("no root", "")] {
        let dir = tempfile::tempdir().unwrap();
        let workdir = dir.path();
        let result = workdir.join("results/fx.json");
        std::fs::create_dir_all(workdir.join("results")).unwrap();
        let announce = if announced.is_empty() {
            String::new()
        } else {
            format!("printf '{{\"type\":\"thread.started\",\"thread_id\":\"{announced}\"}}\\n'\n")
        };
        let shim = make_shim(
            workdir,
            &format!(
                "#!/bin/sh\ncase \"$1\" in --version|-V|-v) printf 'codex-cli 0.153.4\\n'; \
                 exit 0 ;; esac\ncat >/dev/null\ncase \"$*\" in *resume*)\n{announce}\
                 printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":3,\
                 \"output_tokens\":1}}}}\\n'\n\
                 printf '{{\"result\":\"delivered\"}}' > {result}\n\
                 exit 0 ;; esac\nexit 0\n",
                result = result.display()
            ),
        );
        let input = json!({
            "feature": "conformance", "phase": "work", "seat": "work",
            "role_path": workdir.join("missing-role.md"),
            "workdir": workdir,
            "result_path": result,
            "allowed_results": ["complete"], "context": {},
            "boundary": "namespace", "hands": "boxed",
            "resume_context": {"assessment": {"work-site": {
                "status": "supported",
                "identity": {"version": "0.153.4", "applies_to": "0.153.4"},
                "classes": ["work"], "boundaries": ["namespace"], "hands": "boxed",
                "evidence": {"interface": "i", "restrictions": "r",
                             "root": "o", "accounting": "a"},
                "limitations": [], "reason": null
            }}}
        });
        let messages = [
            json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                   "engine_version":"test"}),
            json!({"proto":"forge-driver/v1","msg_id":"m2","type":"resume",
                   "effect_id":"fx","attempt_id":"a1","session_ref":offered}),
            json!({"proto":"forge-driver/v1","msg_id":"m3","type":"start",
                   "effect_id":"fx","attempt_id":"a1","seat":"work","input":input}),
            json!({"proto":"forge-driver/v1","msg_id":"m4","type":"shutdown"}),
        ];
        let operator_home = tempfile::tempdir().unwrap();
        let codex_home = tempfile::tempdir().unwrap();
        let mut child = Command::new(brokkr_bin())
            .arg("driver")
            .args(["codex", "--", "--sandbox", "read-only"])
            .env("BROKKR_CODEX_BIN", &shim)
            .env("HOME", operator_home.path())
            .env("CODEX_HOME", codex_home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for message in &messages {
            writeln!(stdin, "{message}").unwrap();
        }
        drop(stdin);
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let parsed: Vec<Value> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert!(
            std::fs::metadata(&result).is_ok(),
            "{case}: the delivered file is retained for diagnosis"
        );
        let result_message = parsed
            .iter()
            .find(|m| m["type"] == "result")
            .unwrap_or_else(|| panic!("{case}: one result: {parsed:?}"));
        assert_eq!(result_message["status"], "failed", "{case}: {parsed:?}");
        assert!(
            !parsed
                .iter()
                .any(|m| m["type"] == "result" && m["status"] == "succeeded"),
            "{case}: never an accepted successful seat: {parsed:?}"
        );
        assert!(
            !parsed
                .iter()
                .any(|m| m["type"] == "checkpoint" && m["data"]["step"] == "harness-started"),
            "{case}: no guessed launch: {parsed:?}"
        );
    }
}

/// The operator's 2026-09-15 ruling, proved through the real driver: a
/// Codex harness work-seat retry that main rejoins under decision 0030
/// still rejoins here, fed the SHIPPED `adapters/codex.json` assessment
/// and the production-composed harness argv — no synthetic supported
/// status. A cold invocation establishes the root; a fresh driver
/// process receives a correlated `resume` for it and must launch
/// `codex exec resume` with the exact thread and current sandbox/effort
/// re-expressed. Reverting the shipped status to `unmeasured` makes the
/// first gate decline with `unsupported-resume` and this test fails, so
/// it proves behavior rather than reading a declaration back.
#[test]
fn the_shipped_codex_harness_work_seat_rejoins_its_retry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let adapters = Adapters::load(&root.join("adapters")).expect("the shipped adapters load");
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    let hands = library
        .agent("reviewer")
        .expect("the shipped reviewer")
        .hands
        .clone();
    // The production resolver composes the boxed-style argv; the same
    // candidate's own `hands_fragment` is stripped so `compose_site`
    // composes it under `harness`, exactly as the compiler does for a
    // harness realm. No hand-written shortened workspace command.
    let resolution = resolve_agent(
        &library,
        &adapters,
        &Availability::unspecified(),
        "reviewer",
    )
    .expect("the shipped reviewer resolves");
    let candidate = resolution
        .candidates
        .iter()
        .find(|candidate| candidate.provider == "codex")
        .expect("reviewer chains the codex lane");
    assert!(candidate.argv.ends_with(&candidate.hands_fragment));
    let mut command = candidate.argv.clone();
    command.truncate(command.len() - candidate.hands_fragment.len());

    let workdir = tempfile::tempdir().unwrap();
    let result_path = workdir.path().join("results/fx.json");
    std::fs::create_dir_all(workdir.path().join("results")).unwrap();
    let result = result_path.to_str().unwrap();
    let spawn = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Work,
        command,
        hands.as_ref(),
        Some(candidate),
        workdir.path(),
        &[],
        result,
        None,
    );
    assert_eq!(
        &spawn.argv[spawn.argv.len() - 2..],
        ["--sandbox", "workspace-write"],
        "{:?}",
        spawn.argv
    );
    let driver: Vec<String> = spawn.argv[1..].to_vec();
    assert_eq!(driver[0], "driver");

    let offered = "0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let argv_log = workdir.path().join("argv.log");
    let shim = make_shim(
        workdir.path(),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v) printf 'codex-cli 0.154.0\\n'; exit 0 ;; esac\n\
             {record}\
             cat > /dev/null\n\
             printf '{{\"result\":\"resolved\",\"notes\":\"shim\",\"model\":\"seat-claim\"}}' > {result}\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{offered}\"}}\\n'\n\
             printf '{{\"type\":\"turn.started\"}}\\n'\n\
             printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":3,\"output_tokens\":1}}}}\\n'\n",
            record = record_argv_snippet(workdir.path()),
            result = result,
            offered = offered,
        ),
    );
    let assessment = adapters
        .adapter("codex")
        .expect("the shipped codex adapter")
        .resume
        .value();
    let input = json!({
        "feature": "conformance", "phase": "work", "seat": "work",
        "role_path": workdir.path().join("missing-role.md"),
        "workdir": workdir.path(),
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
        // The engine's own facts for the harness work seat (composition
        // bridge): the word and the affirmative no-hands marker.
        "boundary": "harness",
        "hands": "none",
        "resume_context": {"assessment": assessment},
    });

    // Cold: the production driver establishes the root.
    let cold = drive_codex(
        &driver,
        &shim,
        &[
            json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                   "engine_version":"test"}),
            json!({"proto":"forge-driver/v1","msg_id":"m2","type":"start",
                   "effect_id":"fx","attempt_id":"a1","seat":"work","input":input.clone()}),
            json!({"proto":"forge-driver/v1","msg_id":"m3","type":"shutdown"}),
        ],
    );
    let cold_launch = launch_row(&cold, "cold");

    // Retry in a fresh driver process, offered the root the cold one
    // established. This is the assertion that names the regression:
    // under a disabled shipped shape the gate declines with
    // `unsupported-resume` and this row is `cold`, not `resumed`.
    let resumed = drive_codex(
        &driver,
        &shim,
        &[
            json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                   "engine_version":"test"}),
            json!({"proto":"forge-driver/v1","msg_id":"m2","type":"resume",
                   "effect_id":"fx","attempt_id":"a1","session_ref":offered}),
            json!({"proto":"forge-driver/v1","msg_id":"m3","type":"start",
                   "effect_id":"fx","attempt_id":"a1","seat":"work","input":input.clone()}),
            json!({"proto":"forge-driver/v1","msg_id":"m4","type":"shutdown"}),
        ],
    );
    let resumed_launch = launch_row(&resumed, "resumed");
    assert_eq!(resumed_launch["root_session"]["id"], offered, "{resumed:?}");
    assert_eq!(resumed_launch["sandbox"], "workspace-write", "{resumed:?}");
    assert!(
        resumed_launch.get("resume_refusal").is_none(),
        "a preserved rejoin carries no refusal: {resumed:?}"
    );
    assert_eq!(
        resumed.last().unwrap()["status"],
        "succeeded",
        "{resumed:?}"
    );
    // The cold invocation recorded the versioned root the retry offered.
    assert_eq!(
        cold_launch["root_session"]["id"], offered,
        "the cold invocation established the root: {cold:?}"
    );

    // The provider actually saw the resume argv: `exec resume`, exactly
    // the offered thread, current sandbox and effort re-expressed.
    let log = std::fs::read_to_string(&argv_log).unwrap();
    let resume_line = log
        .lines()
        .find(|line| line.contains("exec resume"))
        .unwrap_or_else(|| panic!("the shim saw a resume argv: {log:?}"));
    assert!(resume_line.contains(offered), "{resume_line}");
    assert!(
        resume_line.contains("sandbox_mode=\"workspace-write\""),
        "{resume_line}"
    );
    assert!(
        resume_line.contains("model_reasoning_effort=\"xhigh\""),
        "{resume_line}"
    );
    assert!(resume_line.contains("gpt-6-astra"), "{resume_line}");
    // The same argv over element boundaries. The class expected here is
    // the one this coordinate declares and the composed spawn argv above
    // already pinned: `--sandbox workspace-write`, from the shipped
    // `hands.harness.work` fragment. A shipped change to that fragment
    // fails this test rather than passing under a re-derived expectation.
    assert_resume_argv(
        &resume_parts(workdir.path()),
        "workspace-write",
        "xhigh",
        offered,
        "shipped harness work seat",
    );
}

/// The operator's 2026-09-15 ruling at the INLINE coordinate main actually
/// ships: `recipes/standby` and `recipes/wager-harness` seat implement as a
/// raw `brokkr driver codex` command whose own `--sandbox` class is the
/// author's, with no Brokkr boundary and no hands marker. That is the
/// coordinate the engine reports as `boundary: not applicable`,
/// `hands: none`, and the declaration now names it. A cold invocation must
/// establish the qualified root and a fresh driver must rejoin it with the
/// author's sandbox re-expressed as `-c sandbox_mode`. Reverting the shipped
/// status to `unmeasured` — or dropping `not applicable` from the declared
/// boundaries — makes the retry cold and fails this test.
#[test]
fn the_shipped_inline_codex_work_seat_rejoins_its_retry() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let adapters = Adapters::load(&root.join("adapters")).expect("the shipped adapters load");
    let assessment = adapters
        .adapter("codex")
        .expect("the shipped codex adapter")
        .resume
        .value();

    // The exact argv `recipes/standby`/`recipes/wager-harness` ship for
    // implement: author-written, self-sandboxed, no engine-composed hands.
    let driver: Vec<String> = [
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "xhigh",
        "--sandbox",
        "danger-full-access",
    ]
    .iter()
    .map(|part| part.to_string())
    .collect();

    let workdir = tempfile::tempdir().unwrap();
    let result_path = workdir.path().join("results/fx.json");
    std::fs::create_dir_all(workdir.path().join("results")).unwrap();
    let result = result_path.to_str().unwrap();
    let offered = "0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let argv_log = workdir.path().join("argv.log");
    let shim = make_shim(
        workdir.path(),
        &format!(
            "#!/bin/sh\ncase \"$1\" in --version|-V|-v) printf 'codex-cli 0.154.0\\n'; exit 0 ;; esac\n\
             {record}\
             cat > /dev/null\n\
             printf '{{\"result\":\"resolved\",\"notes\":\"shim\",\"model\":\"seat-claim\"}}' > {result}\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{offered}\"}}\\n'\n\
             printf '{{\"type\":\"turn.started\"}}\\n'\n\
             printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":3,\"output_tokens\":1}}}}\\n'\n",
            record = record_argv_snippet(workdir.path()),
            result = result,
            offered = offered,
        ),
    );
    // The engine's own words for an inline no-hands site are affirmative
    // now: `not applicable` and `none`, present on the input the adapter
    // gate requires (design D10 F1).
    let input = json!({
        "feature": "conformance", "phase": "work", "seat": "work",
        "role_path": workdir.path().join("missing-role.md"),
        "workdir": workdir.path(),
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
        "boundary": "not applicable",
        "hands": "none",
        "resume_context": {"assessment": assessment},
    });

    let cold = drive_codex(
        &driver,
        &shim,
        &[
            json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                   "engine_version":"test"}),
            json!({"proto":"forge-driver/v1","msg_id":"m2","type":"start",
                   "effect_id":"fx","attempt_id":"a1","seat":"work","input":input.clone()}),
            json!({"proto":"forge-driver/v1","msg_id":"m3","type":"shutdown"}),
        ],
    );
    let cold_launch = launch_row(&cold, "cold");
    assert_eq!(
        cold_launch["root_session"]["id"], offered,
        "the enabled inline shape records the qualified root the retry offers: {cold:?}"
    );

    let resumed = drive_codex(
        &driver,
        &shim,
        &[
            json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                   "engine_version":"test"}),
            json!({"proto":"forge-driver/v1","msg_id":"m2","type":"resume",
                   "effect_id":"fx","attempt_id":"a1","session_ref":offered}),
            json!({"proto":"forge-driver/v1","msg_id":"m3","type":"start",
                   "effect_id":"fx","attempt_id":"a1","seat":"work","input":input.clone()}),
            json!({"proto":"forge-driver/v1","msg_id":"m4","type":"shutdown"}),
        ],
    );
    let resumed_launch = launch_row(&resumed, "resumed");
    assert_eq!(resumed_launch["root_session"]["id"], offered, "{resumed:?}");
    assert_eq!(
        resumed_launch["sandbox"], "danger-full-access",
        "{resumed:?}"
    );
    assert!(
        resumed_launch.get("resume_refusal").is_none(),
        "a preserved rejoin carries no refusal: {resumed:?}"
    );

    let log = std::fs::read_to_string(&argv_log).unwrap();
    let resume_line = log
        .lines()
        .find(|line| line.contains("exec resume"))
        .unwrap_or_else(|| panic!("the shim saw a resume argv: {log:?}"));
    assert!(resume_line.contains(offered), "{resume_line}");
    assert!(
        resume_line.contains("sandbox_mode=\"danger-full-access\""),
        "{resume_line}"
    );
    // The same argv over element boundaries. The class expected here is
    // `danger-full-access` because that is what THIS coordinate declares
    // in the shipped recipe argv above — not the harness lane's
    // `workspace-write`. AS1 and AS2 preserve the class the seat
    // declared; normalising the two would assert a class this coordinate
    // never carries.
    assert_resume_argv(
        &resume_parts(workdir.path()),
        "danger-full-access",
        "xhigh",
        offered,
        "shipped inline work seat",
    );
}

/// The directory the codex shims here record their argv parts under, one
/// file per invocation, beside the space-joined `$*` line they append to
/// `argv.log`.
fn parts_dir(dir: &Path) -> PathBuf {
    dir.join("argv.parts.d")
}

/// The POSIX preamble every codex shim here shares: the space-joined `$*`
/// line appended to `argv.log`, then this invocation's argv written one
/// part per line into a file of its OWN.
///
/// `$*` joins the argv with spaces and cannot reconstruct element
/// boundaries: a `contains` over that text cannot tell a whole part from
/// a substring of one, and cannot exclude a competing flag sitting next
/// to the one it found. The parts can. They are recorded BESIDE the `$*`
/// line rather than in place of it, so every assertion already made over
/// that line keeps measuring exactly what it measured.
///
/// One file per invocation, never a shared append with a separator line.
/// A part-per-line append is one write per part, a panel runs its members
/// concurrently against this same shim, and the writes of two live
/// invocations therefore interleave: a reader splitting a shared file on
/// a marker would hand one invocation's parts to another, or truncate
/// one, and the argv assertions downstream would be nondeterministic. A
/// file named for the writing shell's pid cannot be shared by two LIVE
/// invocations, and the `-e` probe walks past a name some already-exited
/// pid used, so ownership holds for reuse too. The empty `: >` claims the
/// name before the first part is written, so an invocation with no argv
/// still owns its record rather than yielding the name to the next.
///
/// Creates the directory, because the shim must not race another shim to
/// create it.
fn record_argv_snippet(dir: &Path) -> String {
    let parts = parts_dir(dir);
    std::fs::create_dir_all(&parts).unwrap();
    format!(
        "printf '%s\\n' \"$*\" >> {log}\n\
         n=0\n\
         p={parts}/$$-$n\n\
         while [ -e \"$p\" ]; do n=$((n+1)); p={parts}/$$-$n; done\n\
         : > \"$p\"\n\
         for a in \"$@\"; do printf '%s\\n' \"$a\" >> \"$p\"; done\n",
        log = shell_quote(&dir.join("argv.log").to_string_lossy()),
        parts = shell_quote(&parts.to_string_lossy()),
    )
}

/// The argv of the one invocation that carried `resume`, read whole out
/// of the per-invocation record its own shim wrote.
///
/// Every resume record is required to agree, so which one is read cannot
/// decide what the caller asserts. A run that produced two DIFFERENT
/// resume argvs fails here by name instead of silently asserting whichever
/// the directory happened to list first.
fn resume_parts(dir: &Path) -> Vec<String> {
    let parts = parts_dir(dir);
    let entries = std::fs::read_dir(&parts)
        .unwrap_or_else(|error| panic!("the shim wrote {}: {error}", parts.display()));
    let mut resumes: Vec<Vec<String>> = Vec::new();
    for entry in entries {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let argv: Vec<String> = text.lines().map(str::to_string).collect();
        if argv.iter().any(|part| part == "resume") {
            resumes.push(argv);
        }
    }
    let first = resumes
        .first()
        .cloned()
        .unwrap_or_else(|| panic!("a resume invocation under {}", parts.display()));
    assert!(
        resumes.iter().all(|argv| *argv == first),
        "the resume invocations disagree, so which record is asserted would decide \
         the result: {resumes:?}"
    );
    first
}

/// The 2026-09-18 review's R1: the argv record a codex shim writes must
/// belong to the invocation that wrote it, under the concurrency the panel
/// shapes actually create. `NoHandsMember` runs two members at once against
/// one shim, so the recording preamble — not just the reader — is what has
/// to hold.
///
/// The preamble is exercised directly here: many invocations at once, each
/// with a DISTINCT argv, all recording into one directory. Every record
/// must be exactly one of the argvs handed out and every argv must appear
/// exactly once — no mixed record, no truncated one, none lost. A shared
/// append split on a separator line fails this, because a part-per-line
/// append is one write per part and two live invocations interleave theirs.
#[test]
fn concurrent_codex_shims_each_own_the_argv_record_they_write() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(
        dir.path(),
        &format!("#!/bin/sh\n{}", record_argv_snippet(dir.path())),
    );
    // Long, self-identifying parts. A one- or two-character part makes two
    // invocations' records indistinguishable, which would let a mixed
    // record pass as a whole one; these name their own invocation and
    // their own position in it. Enough parts per invocation to leave a
    // real window between the writes an interleaving would exploit.
    let expected: Vec<Vec<String>> = (0..24)
        .map(|invocation| {
            (0..64)
                .map(|part| format!("brokkr-conformance-invocation-{invocation:02}-part-{part:02}"))
                .collect()
        })
        .collect();
    let children: Vec<_> = expected
        .iter()
        .map(|argv| Command::new(&shim).args(argv).spawn().unwrap())
        .collect();
    for mut child in children {
        let status = child.wait().unwrap();
        assert!(status.success(), "the recording shim exits clean: {status}");
    }

    let mut recorded: Vec<Vec<String>> = std::fs::read_dir(parts_dir(dir.path()))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
                .lines()
                .map(str::to_string)
                .collect()
        })
        .collect();
    recorded.sort();
    let mut wanted = expected.clone();
    wanted.sort();
    assert_eq!(
        recorded, wanted,
        "each invocation owns one whole record of its own argv"
    );
}

/// One `-c <key>="<value>"` pair, occurring exactly once, with the `-c`
/// immediately in front of it. Adjacency is the point: a `-c` that has
/// drifted away from its override is a `-c` applied to something else.
fn assert_paired_override(parts: &[String], key: &str, expected: &str, case: &str) {
    let prefix = format!("{key}=");
    let at: Vec<usize> = parts
        .iter()
        .enumerate()
        .filter(|(_, part)| part.starts_with(&prefix))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(at.len(), 1, "{case}: exactly one {key} part: {parts:?}");
    let index = at[0];
    assert!(index > 0, "{case}: {key} has a part before it: {parts:?}");
    assert_eq!(
        parts[index - 1],
        "-c",
        "{case}: {key} is paired with its own -c: {parts:?}"
    );
    assert_eq!(parts[index], expected, "{case}: {key}: {parts:?}");
}

/// What a preserved rejoin's argv must show at either shipping
/// coordinate, observed over element boundaries rather than over a
/// space-joined line.
///
/// The class is the one THAT coordinate declares and is passed in, never
/// normalised to a single literal across both: the harness lane composes
/// `--sandbox workspace-write` out of the shipped `adapters/codex.json`
/// `hands.harness.work` fragment, while the inline lane declares
/// `--sandbox danger-full-access` in the argv `recipes/standby` and
/// `recipes/wager-harness` ship. Prescribing one class for both would
/// assert a class one coordinate never declares, and the cheapest way to
/// make that pass would be to edit what ships.
///
/// The effort literal genuinely is shared, and for two separate reasons:
/// the harness lane's `xhigh` comes from the shipped `agents/reviewer.json`
/// `efforts.astra`, the inline lane's from the literal `--effort xhigh`
/// in the shipped recipe argv.
fn assert_resume_argv(parts: &[String], class: &str, effort: &str, offered: &str, case: &str) {
    assert_paired_override(
        parts,
        "sandbox_mode",
        &format!("sandbox_mode=\"{class}\""),
        case,
    );
    assert_paired_override(
        parts,
        "model_reasoning_effort",
        &format!("model_reasoning_effort=\"{effort}\""),
        case,
    );
    // `codex exec resume` has no sandbox flag — the 2026-09-16 live proof
    // records `-s` rejected as an unexpected argument — so the class can
    // only travel as the override above, and no spelling of the flag may
    // survive beside it.
    assert!(
        !parts
            .iter()
            .any(|part| part == "-s" || part == "--sandbox" || part.starts_with("--sandbox=")),
        "{case}: no sandbox flag reaches a resume: {parts:?}"
    );
    assert!(parts.len() >= 2, "{case}: a resume argv: {parts:?}");
    assert_eq!(
        &parts[parts.len() - 2..],
        [offered, "-"],
        "{case}: the confirmed root then the stdin positional: {parts:?}"
    );
}

/// The one `harness-started` launch row with the expected word, or a
/// panic naming what the driver actually emitted.
fn launch_row<'a>(parsed: &'a [Value], word: &str) -> &'a Value {
    let row = parsed
        .iter()
        .find(|m| m["type"] == "checkpoint" && m["data"]["step"] == "harness-started")
        .unwrap_or_else(|| panic!("one launch row: {parsed:?}"));
    assert_eq!(row["data"]["launch"], word, "{parsed:?}");
    &row["data"]
}

/// Drive `brokkr driver …` with test-owned homes and the given protocol
/// messages, returning the parsed stdout.
fn drive_codex(driver: &[String], shim: &Path, messages: &[Value]) -> Vec<Value> {
    let operator_home = tempfile::tempdir().unwrap();
    let codex_home = tempfile::tempdir().unwrap();
    let mut child = Command::new(brokkr_bin())
        .args(driver)
        .env("BROKKR_CODEX_BIN", shim)
        .env("HOME", operator_home.path())
        .env("CODEX_HOME", codex_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// THE PROOFS (design D10): the compiled inline Codex shapes at the
// production provider gate.
//
// The engine, not a test, composes the Start the adapter sees. The live
// rows run the real engine with the real `brokkr driver codex` adapter
// and a deterministic provider shim, and read the launch checkpoints the
// engine journals: a cold invocation records the provider-confirmed
// root, and the engine's own retry offers exactly that root back as
// `resumed`. A hands-bearing namespace site cannot record a root, so its
// engine-composed Start is captured through a recording driver and
// forwarded unchanged into a fresh real adapter process, where the
// production gate names `restrictions-unavailable`.
// ---------------------------------------------------------------------------

/// One process-wide gate for the variables the engine's grandchildren
/// inherit. Because the engine spawns the real adapter, the proof tests
/// must set `BROKKR_CODEX_BIN`/`HOME`/`CODEX_HOME` on the test process;
/// two proof tests running together must not borrow each other's shim.
/// The other conformance tests pass their shim per child and never read
/// these process variables.
static PROOF_ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

const PROOF_OFFER: &str = "0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

#[derive(Clone, Copy, Debug)]
enum ProofShape {
    /// A single inline Codex work seat.
    Single,
    /// A panel whose `alpha` member bears workspace hands.
    HandsMember,
    /// A panel whose `x` member has no hands beside a hands-bearing
    /// `checks:x` sibling.
    NoHandsMember,
}

fn proof_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn proof_codex_argv() -> Value {
    json!([
        "{brokkr}",
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "xhigh",
        "--sandbox",
        "danger-full-access"
    ])
}

fn proof_codex_driver() -> Vec<String> {
    [
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "xhigh",
        "--sandbox",
        "danger-full-access",
    ]
    .iter()
    .map(|part| part.to_string())
    .collect()
}

fn proof_member(hands: Option<&str>) -> Value {
    let mut member = json!({
        "role": "roles/role.md",
        "driver": {"command": proof_codex_argv()},
    });
    if let Some(hands) = hands {
        member["hands"] = json!(hands);
    }
    member
}

fn proof_verify(shape: ProofShape) -> Value {
    let limits = json!({"max_attempts": 1, "timeout_seconds": 30});
    match shape {
        ProofShape::Single => json!({
            "results": ["pass", "fail"],
            "class": "work",
            "limits": limits,
            "role": "roles/role.md",
            "driver": {"command": proof_codex_argv()},
        }),
        // A panel carries no seat-level class: its members carry their own
        // (decision 0021 ruling 1), which is why the members need none.
        ProofShape::HandsMember => json!({
            "results": ["pass", "fail"],
            "limits": limits,
            "aggregate": "unanimous-pass",
            "panel": {
                "alpha": proof_member(Some("workspace")),
                "beta": proof_member(None),
            },
        }),
        ProofShape::NoHandsMember => json!({
            "results": ["pass", "fail"],
            "limits": limits,
            "aggregate": "unanimous-pass",
            "panel": {
                "x": proof_member(None),
                "checks:x": proof_member(Some("workspace")),
            },
        }),
    }
}

fn proof_member_of(shape: ProofShape, wrapped: bool) -> Option<&'static str> {
    match (shape, wrapped) {
        (ProofShape::Single, true) => Some("checks"),
        (ProofShape::Single, false) => None,
        (ProofShape::NoHandsMember, true) => Some("checks:x"),
        (ProofShape::NoHandsMember, false) => Some("x"),
        (ProofShape::HandsMember, _) => Some("alpha"),
    }
}

fn proof_seat_label(shape: ProofShape, wrapped: bool) -> &'static str {
    match (shape, wrapped) {
        (ProofShape::HandsMember, true) => "verify:checks:alpha",
        (ProofShape::HandsMember, false) => "verify:alpha",
        _ => panic!("only the hands member has a refusal capture"),
    }
}

fn write_proof_recipe(dir: &Path, shape: ProofShape, wrapped: bool) {
    std::fs::create_dir_all(dir.join("roles")).unwrap();
    std::fs::write(dir.join("roles/role.md"), "# role").unwrap();
    // The wrapper is applied only when the machine names a dialect phase;
    // `design` is terminal here because the proof begins at `verify`.
    let (phases, terminal) = if wrapped {
        (
            json!(["verify", "review", "done", "design"]),
            json!(["done", "design"]),
        )
    } else {
        (json!(["verify", "review", "done"]), json!(["done"]))
    };
    let policy = json!({
        "phases": phases,
        "initial": "verify",
        "terminal": terminal,
        "rules": [
            {"id":"V","from":"verify","result":"pass","next":"review","reason":"pass"},
            {"id":"VF","from":"verify","result":"fail","next":"verify","reason":"retry"},
            {"id":"R","from":"review","result":"clean","next":"done","reason":"clean"},
        ],
    });
    let config = json!({
        "name": "proofs",
        "policy": "policy.json",
        "seats": {
            "verify": proof_verify(shape),
            "review": {
                "results": ["clean"],
                "role": "roles/role.md",
                "driver": {"command": ["driver"]},
            },
        },
    });
    std::fs::write(
        dir.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.join("policy.json"),
        serde_json::to_vec(&policy).unwrap(),
    )
    .unwrap();
}

/// The compiler expands `{brokkr}` to the running test binary; patch it
/// to the built binary for the real adapter process. The rest of the
/// compiled command, the wrapper, the canonical facts and the instance
/// identity are untouched.
fn patch_proof_body(body: &mut SeatBody, from: &str, to: &str) {
    fn patch_command(command: &mut [String], from: &str, to: &str) {
        for part in command.iter_mut() {
            if part == from {
                *part = to.to_string();
            }
        }
    }
    match body {
        SeatBody::Single { command, .. } => patch_command(command, from, to),
        SeatBody::Panel { members, .. } => {
            for member in members.iter_mut() {
                patch_command(&mut member.command, from, to);
            }
        }
        SeatBody::Sequence { steps } => {
            for step in steps.iter_mut() {
                match &mut step.body {
                    StepBody::Single { command, .. } => patch_command(command, from, to),
                    StepBody::Panel { members, .. } => {
                        for member in members.iter_mut() {
                            patch_command(&mut member.command, from, to);
                        }
                    }
                    StepBody::Dialect { execution } => patch_command(&mut execution.argv, from, to),
                }
            }
        }
        SeatBody::Select { cases, default, .. } => {
            for case in cases.values_mut() {
                patch_proof_body(case, from, to);
            }
            if let Some(default) = default {
                patch_proof_body(default, from, to);
            }
        }
    }
}

fn compile_proof_shape(shape: ProofShape, wrapped: bool) -> (tempfile::TempDir, Bundle, String) {
    let root = proof_root();
    let recipe = tempfile::tempdir().unwrap();
    write_proof_recipe(recipe.path(), shape, wrapped);
    let dialect = Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let bundle = Bundle::compile_with_realm(
        recipe.path(),
        &root.join("agents"),
        &root.join("adapters"),
        None,
        if wrapped { Some(&dialect) } else { None },
        Boundary::Namespace,
    )
    .unwrap();
    let from = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    (recipe, bundle, from)
}

fn patch_proof_bundle(bundle: &mut Bundle, from: &str, to: &str) {
    for seat in bundle.seats.values_mut() {
        patch_proof_body(&mut seat.body, from, to);
    }
}

/// A scratch path safe to splice into a generated POSIX shim body. The
/// test's `TMPDIR` is caller-supplied and may contain spaces (the review
/// reproduced exactly that), so an unquoted redirection would split the
/// path and silently lose the log; single-quoting keeps one word and the
/// embedded-quote escape keeps it valid for any POSIX path.
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// The deterministic provider: it announces one fixed thread id on every
/// invocation and writes no result file, so the attempt fails after the
/// launch row is journaled and the engine parks with a retryable effect.
fn proof_shim_body(dir: &Path, offered: &str) -> String {
    format!(
        "#!/bin/sh\ncase \"$1\" in --version|-V|-v) printf 'codex-cli 0.154.0\\n'; exit 0 ;; esac\n\
         {record}\
         cat > /dev/null\n\
         printf '{{\"type\":\"thread.started\",\"thread_id\":\"{offered}\"}}\\n'\n\
         printf '{{\"type\":\"turn.started\"}}\\n'\n\
         printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":3,\"output_tokens\":1}}}}\\n'\n",
        record = record_argv_snippet(dir),
        offered = offered,
    )
}

/// A driver that logs the engine's own `start` line and then fails, so a
/// refused shape's exact composed Start can be forwarded unchanged.
fn make_proof_recorder(dir: &Path) -> PathBuf {
    let path = dir.join("recorder");
    // One file per invocation, never a shared append. A composed `start`
    // is kilobytes long and several seats record concurrently; an append
    // that large has no atomicity guarantee, so a shared log interleaves
    // and a reader meets half a line. Linux happened to win that race and
    // macOS did not — the bug was always there.
    let log = dir.join("starts");
    std::fs::create_dir_all(&log).unwrap();
    let body = format!(
        "#!/bin/sh\n\
         read -r hello\n\
         printf '%s\\n' '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"cap\",\"type\":\"capabilities\",\"driver\":\"test\",\"version\":\"1\",\"supports\":[]}}'\n\
         read -r start\n\
         printf '%s\\n' \"$start\" > {log}/$$.json\n\
         eid=$(printf '%s' \"$start\" | sed -n 's/.*\"effect_id\":\"\\([^\"]*\\)\".*/\\1/p')\n\
         aid=$(printf '%s' \"$start\" | sed -n 's/.*\"attempt_id\":\"\\([^\"]*\\)\".*/\\1/p')\n\
         printf '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"a\",\"type\":\"accepted\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"session_ref\":null}}\\n' \"$eid\" \"$aid\"\n\
         printf '{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"r\",\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"status\":\"failed\",\"error\":\"capture only\"}}\\n' \"$eid\" \"$aid\"\n\
         read -r done\n",
        log = shell_quote(&log.to_string_lossy()),
    );
    std::fs::write(&path, body).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn run_proof_engine(
    run_dir: &Path,
    bundle: Bundle,
) -> (Store, String, Vec<brokkr_core::envelope::EventEnvelope>) {
    std::fs::create_dir_all(run_dir.join("work")).unwrap();
    let store = Store::open(&run_dir.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "proofs", Some(run_dir.join("work"))).unwrap();
    let run_id = engine.run_id.clone();
    let _ = engine.drive();
    let events = engine.store.load(&run_id).unwrap();
    (engine.store, run_id, events)
}

fn proof_launch_rows(
    events: &[brokkr_core::envelope::EventEnvelope],
    member: Option<&str>,
) -> Vec<Value> {
    let mut rows = Vec::new();
    for event in events {
        let value = serde_json::to_value(event).unwrap();
        let checkpoint = &value["payload"]["checkpoint"];
        if checkpoint["step"] != "harness-started" {
            continue;
        }
        if checkpoint.get("member").and_then(Value::as_str) == member {
            rows.push(checkpoint.clone());
        }
    }
    rows
}

fn capture_proof_input(run_dir: &Path, bundle: Bundle, label: &str) -> Value {
    std::fs::create_dir_all(run_dir.join("work")).unwrap();
    let store = Store::open(&run_dir.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "proofs", Some(run_dir.join("work"))).unwrap();
    let _ = engine.drive();
    let mut seen = Vec::new();
    let dir = run_dir.join("starts");
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let Some(line) = text.lines().next() else {
            continue;
        };
        let value: Value = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("{}: {error}: {line:?}", entry.path().display()));
        if value["seat"] == label {
            return value["input"].clone();
        }
        seen.push(value["seat"].clone());
    }
    panic!("the engine composed no start for '{label}'; it composed {seen:?}");
}

/// Design D10 item 1: the compiled no-hands shapes (single and panel
/// member, wrapped and unwrapped) rejoin the provider-confirmed root the
/// cold invocation recorded. The provider announces one fixed root; the
/// engine records it and offers exactly that root back on its own retry,
/// which the real adapter confirms as `resumed` with the current sandbox
/// re-expressed and no refusal.
#[test]
fn the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root() {
    let _guard = PROOF_ENV.lock().unwrap();
    for (shape, wrapped) in [
        (ProofShape::Single, true),
        (ProofShape::Single, false),
        (ProofShape::NoHandsMember, true),
        (ProofShape::NoHandsMember, false),
    ] {
        let (recipe, mut bundle, from) = compile_proof_shape(shape, wrapped);
        patch_proof_bundle(&mut bundle, &from, brokkr_bin());
        let run_dir = tempfile::tempdir().unwrap();
        let shim = make_shim(
            run_dir.path(),
            &proof_shim_body(run_dir.path(), PROOF_OFFER),
        );
        std::env::set_var("BROKKR_CODEX_BIN", &shim);
        std::env::set_var("HOME", run_dir.path());
        std::env::set_var("CODEX_HOME", run_dir.path().join("codex-home"));
        let member = proof_member_of(shape, wrapped);

        let (mut store, run_id, events) = run_proof_engine(run_dir.path(), bundle.clone());
        let cold = proof_launch_rows(&events, member);
        let cold_row = cold
            .last()
            .unwrap_or_else(|| panic!("{shape:?} wrapped={wrapped}: a cold launch row"));
        assert_eq!(
            cold_row["launch"], "cold",
            "{shape:?} wrapped={wrapped}: the first invocation is cold"
        );
        assert_eq!(
            cold_row["root_session"]["id"], PROOF_OFFER,
            "{shape:?} wrapped={wrapped}: the provider-confirmed root is recorded"
        );
        assert_eq!(
            cold_row["boundary"], "not applicable",
            "{shape:?} wrapped={wrapped}: a resolved no-hands site is affirmative"
        );

        operator_command(&mut store, &run_id, "retry", "operator", "once more").unwrap();
        let mut engine =
            Engine::resume(store, bundle, &run_id, Some(run_dir.path().join("work"))).unwrap();
        engine.drive().unwrap();
        let events = engine.store.load(&run_id).unwrap();
        let retry = proof_launch_rows(&events, member);
        let resumed = retry
            .iter()
            .find(|row| row["launch"] == "resumed")
            .unwrap_or_else(|| {
                panic!("{shape:?} wrapped={wrapped}: the retry rejoins, rows={retry:#?}")
            });
        assert_eq!(
            resumed["root_session"]["id"], PROOF_OFFER,
            "{shape:?} wrapped={wrapped}: confirmed as the offered root"
        );
        assert!(
            resumed.get("resume_refusal").is_none(),
            "{shape:?} wrapped={wrapped}: a live rejoin carries no refusal: {resumed}"
        );

        let log = std::fs::read_to_string(run_dir.path().join("argv.log")).unwrap_or_default();
        let resume_line = log
            .lines()
            .find(|line| line.contains("exec resume") && line.contains(PROOF_OFFER))
            .unwrap_or_else(|| {
                panic!("{shape:?} wrapped={wrapped}: the provider saw a resume argv: {log:?}")
            });
        assert!(
            resume_line.contains("sandbox_mode=\"danger-full-access\""),
            "{shape:?} wrapped={wrapped}: the class is re-expressed: {resume_line}"
        );
        assert!(
            resume_line.contains("model_reasoning_effort=\"xhigh\""),
            "{shape:?} wrapped={wrapped}: the effort is re-expressed: {resume_line}"
        );
        // The same argv over element boundaries, at all four compiled
        // shapes. These are inline coordinates, so the class is the
        // `danger-full-access` the recipe declares.
        assert_resume_argv(
            &resume_parts(run_dir.path()),
            "danger-full-access",
            "xhigh",
            PROOF_OFFER,
            &format!("compiled live inline {shape:?} wrapped={wrapped}"),
        );
        drop(recipe);
    }
    std::env::remove_var("BROKKR_CODEX_BIN");
    std::env::remove_var("HOME");
    std::env::remove_var("CODEX_HOME");
}

/// Design D10 item 1: the compiled hands-bearing namespace site (panel
/// member, wrapped and unwrapped) cannot affirm confinement, so the
/// production gate refuses it. The engine cannot offer a root here (the
/// boxed site records none), so its own composed Start is captured and
/// forwarded unchanged into a fresh adapter. With no offer the exchange
/// is cold and carries no invented refusal; with an offer the gate names
/// `restrictions-unavailable`, and the declined offer still falls back
/// to a fresh cold launch of the provider (the refusal is to REJOIN, not
/// to run). The gate exchange and its token are asserted before any
/// supplemental confinement-marker check, so a mutation that publishes a
/// hands site as known no-hands fails at the gate's own decision.
#[test]
fn the_compiled_hands_inline_codex_shapes_refuse_unavailable_confinement() {
    let _guard = PROOF_ENV.lock().unwrap();
    for (shape, wrapped) in [
        (ProofShape::HandsMember, true),
        (ProofShape::HandsMember, false),
    ] {
        let (recipe, mut bundle, from) = compile_proof_shape(shape, wrapped);
        let run_dir = tempfile::tempdir().unwrap();
        let recorder = make_proof_recorder(run_dir.path());
        patch_proof_bundle(&mut bundle, &from, &recorder.to_string_lossy());
        let input = capture_proof_input(run_dir.path(), bundle, proof_seat_label(shape, wrapped));

        let driver = proof_codex_driver();
        let shim = make_shim(
            run_dir.path(),
            &proof_shim_body(run_dir.path(), PROOF_OFFER),
        );
        let hello = json!({"proto":"forge-driver/v1","msg_id":"m1","type":"hello",
                           "engine_version":"test"});
        // No offer: cold, and no invented refusal record.
        let cold = drive_codex(
            &driver,
            &shim,
            &[
                hello.clone(),
                json!({"proto":"forge-driver/v1","msg_id":"m2","type":"start",
                       "effect_id":"fx","attempt_id":"a1","seat":"verify","input":input.clone()}),
                json!({"proto":"forge-driver/v1","msg_id":"m3","type":"shutdown"}),
            ],
        );
        let cold_launch = launch_row(&cold, "cold");
        assert!(
            cold_launch.get("resume_refusal").is_none(),
            "{shape:?} wrapped={wrapped}: no offer invents no refusal: {cold:?}"
        );

        // An offer: the production gate names its refusal. This is the
        // decision assertion, taken ahead of any marker fixture, so the
        // hands->no-hands removal mutation fails HERE.
        let refused = drive_codex(
            &driver,
            &shim,
            &[
                hello,
                json!({"proto":"forge-driver/v1","msg_id":"m2","type":"resume",
                       "effect_id":"fx","attempt_id":"a1","session_ref":PROOF_OFFER}),
                json!({"proto":"forge-driver/v1","msg_id":"m3","type":"start",
                       "effect_id":"fx","attempt_id":"a1","seat":"verify","input":input.clone()}),
                json!({"proto":"forge-driver/v1","msg_id":"m4","type":"shutdown"}),
            ],
        );
        let refused_row = refused
            .iter()
            .find(|m| m["type"] == "checkpoint" && m["data"]["step"] == "harness-started")
            .unwrap_or_else(|| panic!("{shape:?} wrapped={wrapped}: one launch row: {refused:?}"));
        assert_eq!(
            refused_row["data"]["resume_refusal"], "restrictions-unavailable",
            "{shape:?} wrapped={wrapped}: the gate's own token: {refused:?}"
        );
        // The refusal declines to rejoin; the provider still runs cold.
        assert_eq!(
            refused_row["data"]["launch"], "cold",
            "{shape:?} wrapped={wrapped}: the declined offer falls back cold: {refused:?}"
        );

        // Supplemental: the same captured Start still carries the real
        // compiled confinement facts the gate judged.
        assert_eq!(
            input["boundary"], "namespace",
            "{shape:?} wrapped={wrapped}: the compiled site's own boundary"
        );
        assert_eq!(
            input["hands"], "boxed",
            "{shape:?} wrapped={wrapped}: the compiled site's own boxed marker"
        );
        drop(recipe);
    }
}
