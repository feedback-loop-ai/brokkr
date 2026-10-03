//! `brokkr probe harness` end to end: the verb resolves the shipped
//! adapter files and the secrets store, writes the report, and reports
//! the drift since the report it replaces. The CLI it launches is a fake
//! that speaks one Claude-shaped turn; no test reaches a provider.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

#[path = "../../../tests/support/executable.rs"]
mod executable;

/// One Claude-shaped turn: the init event names the hands server when the
/// hands argv is given, lists the two network tools unless the OFF
/// controls are, and every deliberate mistake is refused. It reads its
/// version from the file beside it, so a rerun can change it.
const FAKE: &str = r#"#!/bin/sh
case " $* " in
  *" --version "*) cat "$(dirname "$0")/version"; exit 0 ;;
  *brokkr-probe-no-such-*) echo "rejected" >&2; exit 1 ;;
esac
[ -n "$FAKE_TOKEN" ] || { echo "not logged in" >&2; exit 1; }
case " $* " in
  *" --strict-mcp-config "*) printf '{"type":"system","subtype":"init","session_id":"s-1","tools":[],"mcp_servers":[{"name":"brokkr","status":"connected"}]}\n' ;;
  *" --disallowedTools "*) printf '{"type":"system","subtype":"init","session_id":"s-1","tools":["Bash"],"mcp_servers":[]}\n' ;;
  *) printf '{"type":"system","subtype":"init","session_id":"s-1","tools":["Bash","WebFetch","WebSearch"],"mcp_servers":[]}\n' ;;
esac
printf '{"type":"result","total_cost_usd":0.5,"usage":{"input_tokens":1,"output_tokens":1}}\n'
"#;

/// The OFF controls the engine composes from the shipped claude adapter's
/// `native_capabilities`, as a row names them.
const SWITCHED: &str = "switched off by --disallowedTools WebFetch,WebSearch";

fn repo_adapters() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters")
}

/// A fake CLI at version `1.0.0 (Fake)` and a store binding `FAKE_TOKEN`.
struct Host {
    dir: tempfile::TempDir,
    cli: PathBuf,
    store: PathBuf,
}

fn host() -> Host {
    let dir = tempfile::tempdir().unwrap();
    let cli = executable::install(dir.path(), "fake-cli", FAKE);
    std::fs::write(dir.path().join("version"), "1.0.0 (Fake)\n").unwrap();
    let store = dir.path().join("secrets.env");
    brokkr_protocol::secret::store_set(&store, "FAKE_TOKEN", "fake-token-4e1d").unwrap();
    Host { dir, cli, store }
}

impl Host {
    fn probe(&self, adapters: &Path, extra: &[&str]) -> Output {
        self.launch(Some(&self.cli), adapters, extra)
    }

    /// `brokkr probe harness`, given `--cli` when `cli` names one.
    fn launch(&self, cli: Option<&Path>, adapters: &Path, extra: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
        command.args(["probe", "harness"]);
        if let Some(cli) = cli {
            command.arg("--cli").arg(cli);
        }
        command
            .arg("--secrets-file")
            .arg(&self.store)
            .arg("--adapters-dir")
            .arg(adapters)
            .args(extra)
            .output()
            .unwrap()
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Adapter rows, each written as field, declared, implied, agreement.
fn rows(rows: &[[&str; 4]]) -> Value {
    let keys = ["field", "declared", "implied", "agreement"];
    rows.iter()
        .map(|row| {
            let pairs = keys
                .iter()
                .zip(row)
                .map(|(key, cell)| (key.to_string(), json!(cell)));
            pairs.collect::<serde_json::Map<String, Value>>().into()
        })
        .collect::<Vec<Value>>()
        .into()
}

#[test]
fn the_verb_writes_the_shipped_adapter_s_report_and_its_drift_on_a_new_version() {
    let host = host();
    let out = host.path("claude.json");
    let args = [
        "--adapter",
        "claude",
        "--credential",
        "FAKE_TOKEN",
        "--out",
        out.to_str().unwrap(),
    ];
    let first = host.probe(&repo_adapters(), &args);
    let written = format!("probe report written to {}\n", out.display());
    assert_eq!(
        (first.status.code(), stderr(&first)),
        (Some(0), written.clone())
    );
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(
        report["facts"]["headless"]["value"]["argv"],
        json!([
            "{cli}",
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "{prompt}",
        ])
    );
    assert_eq!(
        report["adapter_fields"],
        rows(&[
            [
                "efforts",
                "low, medium, high, xhigh, max",
                "unmeasured",
                "not-compared"
            ],
            ["hands", "supported", "supported", "agrees"],
            [
                "resume.boxed-workspace.identity.version",
                "2.1.266",
                "1.0.0 (Fake)",
                "differs"
            ],
            [
                "native_capabilities.tools",
                "WebFetch, WebSearch",
                "WebFetch, WebSearch",
                "agrees"
            ],
            [
                "native_capabilities.known.web-fetch.off",
                "switched off",
                SWITCHED,
                "agrees"
            ],
            [
                "native_capabilities.known.web-search.off",
                "switched off",
                SWITCHED,
                "agrees"
            ],
        ])
    );
    assert_eq!(
        report["eligibility"],
        json!({
            "verdict": "boxed",
            "reason": "its own tools switch off and the hands MCP server connects",
        })
    );
    assert_eq!(report["drift"], json!([]));

    std::fs::write(host.path("version"), "2.0.0 (Fake)\n").unwrap();
    let rerun = host.probe(&repo_adapters(), &args);
    assert_eq!(
        (rerun.status.code(), stderr(&rerun)),
        (
            Some(0),
            format!(
                "drift: cli.version: measured \"1.0.0 (Fake)\" -> measured \"2.0.0 (Fake)\"\n\
                 {written}"
            )
        )
    );
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(
        report["drift"],
        json!([{
            "fact": "cli.version",
            "before": "measured \"1.0.0 (Fake)\"",
            "after": "measured \"2.0.0 (Fake)\"",
        }])
    );
}

#[test]
fn without_out_the_report_is_printed_and_a_turn_with_no_credential_is_refused() {
    let host = host();
    let output = host.probe(&repo_adapters(), &["--adapter", "codex"]);
    assert_eq!(
        (output.status.code(), stderr(&output)),
        (Some(0), String::new())
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        (
            &report["adapter"],
            &report["facts"]["headless"]["value"],
            &report["eligibility"]["verdict"]
        ),
        (
            &json!("codex"),
            &json!({"argv": ["{cli}", "exec", "--json", "-C", "{workdir}", "{prompt}"], "exit": 1}),
            &json!("refused"),
        )
    );
}

/// An adapters directory holding the shipped claude adapter with the fake
/// as its binary and its resume identity unknown, and a custom adapter no
/// built-in driver runs.
fn custom_adapters(host: &Host) -> PathBuf {
    let dir = host.path("adapters");
    std::fs::create_dir(&dir).unwrap();
    let read = |name: &str| -> Value {
        serde_json::from_str(&std::fs::read_to_string(repo_adapters().join(name)).unwrap()).unwrap()
    };
    let mut claude = read("claude.json");
    claude["binary"] = json!(host.cli);
    claude["resume"]["boxed-workspace"]["identity"] = json!({"unknown": "never measured"});
    let mut custom = read("exec.json");
    custom["provider"] = json!("custom");
    custom["driver"] = json!(["/usr/local/bin/custom-driver"]);
    for (name, adapter) in [("claude.json", claude), ("custom.json", custom)] {
        std::fs::write(dir.join(name), adapter.to_string()).unwrap();
    }
    dir
}

/// The claude adapter's report from the adapters under `adapters`,
/// probed with `--cli` when `cli` names one.
fn claude_report_in(host: &Host, cli: Option<&Path>, adapters: &Path) -> Value {
    let output = host.launch(
        cli,
        adapters,
        &["--adapter", "claude", "--credential", "FAKE_TOKEN"],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The report of the custom claude adapter, probed with `--cli` when
/// `cli` names one.
fn custom_claude_report(host: &Host, cli: Option<&Path>) -> Value {
    claude_report_in(host, cli, &custom_adapters(host))
}

/// The report of the shipped claude adapter with `alter` applied to its
/// `native_capabilities`, written under `dir`.
fn altered_native_report(host: &Host, dir: &str, alter: impl Fn(&mut Value)) -> Value {
    let adapters = host.path(dir);
    std::fs::create_dir(&adapters).unwrap();
    let text = std::fs::read_to_string(repo_adapters().join("claude.json")).unwrap();
    let mut claude: Value = serde_json::from_str(&text).unwrap();
    alter(&mut claude["native_capabilities"]);
    std::fs::write(adapters.join("claude.json"), claude.to_string()).unwrap();
    claude_report_in(host, Some(&host.cli), &adapters)
}

#[test]
fn the_adapter_s_native_declaration_is_read_typed_and_its_uncomposed_off_is_unmeasured() {
    let host = host();
    let report = altered_native_report(&host, "no-off", |native| {
        native["known"]["web-fetch"]["off"] = json!({"unmeasured": "never tried"});
        native["known"]["web-search"]["off"] = json!({"unsupported": "no switch"});
    });
    let native_rows = report["adapter_fields"].as_array().unwrap();
    assert_eq!(
        json!(native_rows[3..]),
        rows(&[
            [
                "native_capabilities.tools",
                "WebFetch, WebSearch",
                "WebFetch, WebSearch",
                "agrees"
            ],
            [
                "native_capabilities.known.web-fetch.off",
                "unmeasured",
                "unmeasured",
                "not-compared"
            ],
            [
                "native_capabilities.known.web-search.off",
                "unsupported",
                "unmeasured",
                "not-compared"
            ],
        ])
    );
    assert_eq!(
        report["facts"]["capabilities"]["value"][0]["off"],
        json!({
            "status": "unmeasured",
            "why": "the turn under the declared OFF controls was not read: the engine composes \
                    no OFF control for a seat granted nothing: seat 'adapter-plan' (office \
                    'adapter-plan') in realm '<unmapped>': provider 'claude' is known to carry \
                    native capability 'web-fetch', which this seat does not hold, and no valid \
                    control denies it: its OFF control is unmeasured (never tried). A known \
                    native power is launched only with a delivered denial, never on what \
                    absence implies; repair the adapter data (decision 0066 ruling 1)",
        })
    );
    let report = altered_native_report(&host, "unmeasured", |native| {
        *native = json!({"unmeasured": "never measured"});
    });
    assert_eq!(
        (
            &report["adapter_fields"][3],
            &report["facts"]["capabilities"]
        ),
        (
            &rows(&[[
                "native_capabilities.tools",
                "unmeasured",
                "WebFetch, WebSearch",
                "differs"
            ]])[0],
            &json!({
                "status": "unmeasured",
                "why": "the adapter declares its native capabilities unmeasured: never measured",
            }),
        )
    );
}

#[test]
fn a_resume_shape_of_unknown_identity_is_named_as_a_difference() {
    let host = host();
    let report = custom_claude_report(&host, Some(&host.cli));
    assert_eq!(
        json!([report["adapter_fields"][2]]),
        rows(&[[
            "resume.boxed-workspace.identity.version",
            "unknown: never measured",
            "1.0.0 (Fake)",
            "differs",
        ]])
    );
}

#[test]
fn without_cli_the_verb_launches_the_adapter_s_own_binary() {
    let host = host();
    assert_eq!(
        custom_claude_report(&host, None)["cli"],
        json!({
            "command": host.cli,
            "version": {
                "status": "measured",
                "value": "1.0.0 (Fake)",
                "evidence": "the first line `--version` printed",
            },
        })
    );
}

#[test]
fn what_the_verb_cannot_probe_is_refused_by_name() {
    let host = host();
    let adapters = custom_adapters(&host);
    let not_a_report = host.path("not-a-report.json");
    std::fs::write(&not_a_report, "{}").unwrap();
    let missing = host.path("no-such-cli");
    let cases: [(&Path, Vec<&str>, String); 5] = [
        (
            &host.cli,
            vec!["--adapter", "gemini"],
            format!("no adapter 'gemini' under {}", adapters.display()),
        ),
        (
            &host.cli,
            vec!["--adapter", "custom"],
            "adapter 'custom' is not launched by a built-in driver, so the probe has no \
             launch grammar for its CLI"
                .to_string(),
        ),
        (
            &host.cli,
            vec!["--adapter", "claude", "--credential", "NOPE"],
            format!(
                "secret 'NOPE' is not in the store at {} (brokkr secrets set NOPE)",
                host.store.display()
            ),
        ),
        (
            &host.cli,
            vec![
                "--adapter",
                "claude",
                "--out",
                not_a_report.to_str().unwrap(),
            ],
            "not a brokkr.harness-probe/v1 report: missing field `probe` at line 1 column 2"
                .to_string(),
        ),
        (
            &missing,
            vec!["--adapter", "claude"],
            format!(
                "could not launch {0}: No such file or directory (os error 2): No such file \
                 or directory (os error 2)",
                missing.display()
            ),
        ),
    ];
    for (cli, args, refusal) in cases {
        let output = host.launch(Some(cli), &adapters, &args);
        assert_eq!(
            (output.status.code(), stderr(&output)),
            (Some(1), format!("error: {refusal}\n")),
            "{args:?}"
        );
    }
}
