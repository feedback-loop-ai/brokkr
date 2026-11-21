//! #1199, F5: no adapter puts a byte of its harness's stderr on the wire
//! the engine journals from. Everything the engine journals arrives as a
//! wire message or as the driver's own stderr, which the engine names by
//! evidence alone, so a sentinel the harness writes only to its stderr,
//! a `model:` header line included, must reach no line of the driver's
//! stdout, whether the harness delivered its result or failed.

use super::tests::executable;
use super::*;
use std::process::{Command, Output};

/// What the harness prints that no seat binds, so no driver masks it.
const SENTINEL: &str = "UNBOUND-SENTINEL-1199";

/// The passthrough for `kind`, with its harness pinned to `shim`.
/// This match must stage every kind in the enum's generated inventory.
fn staged(kind: AdapterKind, child: &mut Command, dir: &Path, shim: &Path) -> Vec<String> {
    let pinned = |model: &str| vec!["--model".to_string(), model.to_string()];
    match kind {
        AdapterKind::Claude => {
            child.env("BROKKR_CLAUDE_BIN", shim);
            pinned("claude-x")
        }
        AdapterKind::Lanetally => {
            child
                .env("BROKKR_LANETALLY_BIN", shim)
                .env("BROKKR_CLAUDE_BIN", shim);
            pinned("claude-x")
        }
        AdapterKind::Codex => {
            child.env("BROKKR_CODEX_BIN", shim);
            pinned("gpt-x")
        }
        AdapterKind::Dsh => {
            child.env("BROKKR_DSH_BIN", shim).env("DSH_HOME", dir);
            Vec::new()
        }
        AdapterKind::Qoder => {
            child.env("BROKKR_QODER_BIN", qoder(dir, shim));
            vec!["-m".to_string(), "Fast".to_string()]
        }
        AdapterKind::Exec => vec![shim.to_string_lossy().into_owned()],
    }
}

/// A Qoder CLI that prints its recorded tool-less turn's init, assistant
/// and result rows on its stdout, then runs `shim` as the rest of itself.
fn qoder(dir: &Path, shim: &Path) -> PathBuf {
    let rows = super::qoder::tests::tool_less_rows();
    let stdout = dir.join("qoder-stdout");
    std::fs::write(&stdout, rows).unwrap();
    let body = format!(
        "#!/bin/sh\ncat '{}'\nexec '{}'\n",
        stdout.display(),
        shim.display()
    );
    executable(dir, "qodercli", &body)
}

/// A harness that marks it ran, writes the sentinel only to its stderr,
/// as a `model:` header and as prose, delivers its result where `code` is
/// 0, and exits `code`.
fn harness(dir: &Path, code: i32) -> String {
    let deliver = match code {
        0 => format!(
            "printf '{{\"result\":\"complete\"}}' > '{}'\n",
            dir.join("result.json").display()
        ),
        _ => String::new(),
    };
    format!(
        "#!/bin/sh\n: > '{ran}'\ncat >/dev/null\n\
         printf 'model: {SENTINEL}\\nsaid {SENTINEL}\\n' >&2\n{deliver}exit {code}\n",
        ran = dir.join("ran").display(),
    )
}

const DRIVER: &str = "BROKKR_TEST_STDERR_ADAPTER";
const ARGS: &str = "BROKKR_TEST_STDERR_PASSTHROUGH";

/// Run the production stdio entry point when the parent starts this test
/// binary as a driver. Libtest banners are outside the driver's JSON lines.
#[test]
fn stdio_driver_child() {
    let Ok(name) = std::env::var(DRIVER) else {
        return;
    };
    let kind = AdapterKind::parse(&name).unwrap();
    let extra = serde_json::from_str(&std::env::var(ARGS).unwrap()).unwrap();
    super::serve(kind, extra, super::tests::unconfined).unwrap();
}

fn driver(kind: AdapterKind) -> Command {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            concat!(module_path!(), "::stdio_driver_child")
                .strip_prefix("brokkr_protocol::")
                .unwrap(),
            "--nocapture",
            "--quiet",
        ])
        .env(DRIVER, kind.command_name())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    child
}

fn served(mut child: Command, extra: &[String], input: Value) -> Output {
    let (hello, start) = start::hello_and_start(input);
    let mut child = child
        .env(ARGS, serde_json::to_string(extra).unwrap())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{hello}\n{start}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    output
}

fn wire(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|line| line.starts_with('{'))
        .collect()
}

/// The terminal result's status and its result's verdict, `null` where
/// it carries none.
fn terminal(stdout: &[&str]) -> (Value, Value) {
    let last: Value = serde_json::from_str(stdout.last().unwrap()).unwrap();
    assert_eq!(last["type"], "result", "{stdout:?}");
    (last["status"].clone(), last["result"]["result"].clone())
}

#[cfg(unix)]
#[test]
fn no_adapter_puts_its_harness_stderr_on_the_wire() {
    for &kind in AdapterKind::ALL {
        for code in [0, 3] {
            let dir = tempfile::tempdir().unwrap();
            let shim = executable(dir.path(), "harness", &harness(dir.path(), code));
            let mut child = driver(kind);
            let extra = staged(kind, &mut child, dir.path(), &shim);
            let result = dir.path().join("result.json");
            let mut input = json!({"role_path": "/dev/null", "allowed_results": ["complete"],
                                   "result_path": result, "workdir": dir.path()});
            if kind == AdapterKind::Exec {
                input["dialect_exec"] = json!({"failure_result": "fail"});
            }
            let output = served(child, &extra, input);
            let raw = String::from_utf8(output.stdout).unwrap();
            assert!(!raw.contains(SENTINEL), "{kind:?} {code}: {raw}");
            let stdout = wire(&raw);
            assert!(
                dir.path().join("ran").exists(),
                "{kind:?} {code}: {stdout:?}"
            );
            // Exec's verdict is its exit; every model harness's is its
            // delivered result, and a failed exit delivers none.
            let expected = match (kind, code) {
                (AdapterKind::Exec, 0) => (json!("succeeded"), json!("drafted")),
                (AdapterKind::Exec, _) => (json!("succeeded"), json!("fail")),
                (_, 0) => (json!("succeeded"), json!("complete")),
                (_, _) => (json!("failed"), Value::Null),
            };
            assert_eq!(terminal(&stdout), expected, "{kind:?} {code}: {stdout:?}");
        }
    }
}

fn assert_codex_stderr_model(extra: &[&str], header: &str, expected: &str) {
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "-mgpt-y",
        "#!/bin/sh\ncat >/dev/null\nprintf 'model: %s\n' \"$BROKKR_TEST_HEADER\" >&2\n",
    );
    assert_eq!(shim.file_name().unwrap(), "-mgpt-y");
    let path = std::env::join_paths(
        std::iter::once(dir.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let extra: Vec<_> = extra.iter().map(|part| part.to_string()).collect();
    let mut child = driver(AdapterKind::Codex);
    child
        .env("PATH", &path)
        .env("BROKKR_CODEX_BIN", shim.file_name().unwrap())
        .env("BROKKR_TEST_HEADER", header);
    let output = served(
        child,
        &extra,
        json!({
            "role_path": "/dev/null", "workdir": dir.path(),
            "result_path": dir.path().join("result.json"),
            "allowed_results": ["complete"]
        }),
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let model = wire(&stdout)
        .into_iter()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .find(|row| row["data"]["step"] == "codex-session-finished")
        .unwrap_or_else(|| {
            panic!(
                "{stdout}; stderr: {:?}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    assert_eq!(model["data"]["model"], expected, "{extra:?}, {header}");
}

#[test]
fn codex_stderr_does_not_take_the_executable_name_as_its_pin() {
    assert_codex_stderr_model(&["--model", "gpt-x"], "gpt-y", MODEL_NOT_REPORTED);
}

#[test]
fn codex_stderr_admits_the_seat_pin_with_a_short_flag_shaped_executable() {
    assert_codex_stderr_model(&["--model", "gpt-x"], "gpt-x", "gpt-x");
}

#[test]
fn codex_stderr_with_no_passthrough_pin_does_not_report_a_model() {
    assert_codex_stderr_model(&[], "gpt-y", MODEL_NOT_REPORTED);
}

/// An exec validator `script` whose state command cannot start.
fn exec_with_missing_state(script: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let extra = ["sh".into(), "-c".into(), script.to_string()];
    served(
        driver(AdapterKind::Exec),
        &extra,
        json!({
            "role_path": "/dev/null", "workdir": dir.path(),
            "result_path": dir.path().join("result.json"),
            "dialect_exec": {"state": [dir.path().join("missing-state")]}
        }),
    )
}

#[test]
fn an_exec_state_spawn_failure_preserves_masked_validator_stderr() {
    let output =
        exec_with_missing_state(&format!("cat >/dev/null; printf '%s\n' '{SENTINEL}' >&2"));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains(SENTINEL), "{stdout}");
    let rows = wire(&stdout);
    let last: Value = serde_json::from_str(rows.last().unwrap()).unwrap();
    assert_eq!(last["data"]["step"], "exec-started");
    let stderr = String::from_utf8(output.stderr).unwrap();
    let evidence = crate::process::Evidence::of(format!("{SENTINEL}\n").as_bytes());
    assert_eq!(stderr.lines().next(), Some(SENTINEL), "{stderr}");
    assert_eq!(stderr.matches(SENTINEL).count(), 1, "{stderr}");
    assert_eq!(
        stderr.lines().last().unwrap(),
        format!("validator stderr: {evidence}; the command was spawned, so no result is sent")
    );
}

/// A validator that printed nothing has no stderr to name: the state
/// command's failure is reported alone, with no tail before it.
#[test]
fn an_exec_state_spawn_failure_after_a_silent_validator_names_no_validator_stderr() {
    let output = exec_with_missing_state("cat >/dev/null");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("validator stderr:"), "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(
        lines[0].ends_with("; the command was spawned, so no result is sent"),
        "{stderr}"
    );
}
