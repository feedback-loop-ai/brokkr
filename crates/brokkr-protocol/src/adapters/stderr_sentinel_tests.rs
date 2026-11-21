//! #1199, F5: no adapter puts a byte of its harness's stderr on the wire
//! the engine journals from. Everything the engine journals arrives as a
//! wire message or as the driver's own stderr, which the engine names by
//! evidence alone, so a sentinel the harness writes only to its stderr,
//! a `model:` header line included, must reach no line of the driver's
//! stdout, whether the harness delivered its result or failed.

use super::tests::executable;
use super::*;
use crate::env_guard::EnvGuard;

/// What the harness prints that no seat binds, so no driver masks it.
const SENTINEL: &str = "UNBOUND-SENTINEL-1199";

/// How `kind`'s seat is staged with its harness pinned to `shim`: the
/// passthrough it runs with, and the kind the test visits after it.
/// Exhaustive with no wildcard, and the one place the kinds are listed:
/// a new kind must say how it is staged, and is visited only once the
/// kind before it names it, as the last names none.
fn staged(
    kind: AdapterKind,
    env: &mut EnvGuard,
    dir: &Path,
    shim: &Path,
) -> (Vec<String>, Option<AdapterKind>) {
    let pinned = |model: &str| vec!["--model".to_string(), model.to_string()];
    match kind {
        AdapterKind::Claude => {
            env.set("BROKKR_CLAUDE_BIN", shim);
            (pinned("claude-x"), Some(AdapterKind::Lanetally))
        }
        AdapterKind::Lanetally => {
            env.set("BROKKR_LANETALLY_BIN", shim);
            env.set("BROKKR_CLAUDE_BIN", shim);
            (pinned("claude-x"), Some(AdapterKind::Codex))
        }
        AdapterKind::Codex => {
            env.set("BROKKR_CODEX_BIN", shim);
            (pinned("gpt-x"), Some(AdapterKind::Dsh))
        }
        AdapterKind::Dsh => {
            env.set("BROKKR_DSH_BIN", shim);
            env.set("DSH_HOME", dir);
            (Vec::new(), Some(AdapterKind::Exec))
        }
        // The exec seat is a dialect validator: its command is the shim.
        AdapterKind::Exec => (vec![shim.to_string_lossy().into_owned()], None),
    }
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

/// The driver's stdout, line by line, for one start of `kind` on `extra`.
fn served(kind: AdapterKind, extra: &[String], input: Value) -> Vec<String> {
    let (hello, start) = start::hello_and_start(input);
    let mut stdout = Vec::new();
    let wire = format!("{hello}\n{start}\n");
    start::serve_io(kind, extra, wire.as_bytes(), &mut stdout).unwrap();
    String::from_utf8(stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The terminal result's status and its result's verdict, `null` where
/// it carries none.
fn terminal(stdout: &[String]) -> (Value, Value) {
    let last: Value = serde_json::from_str(stdout.last().unwrap()).unwrap();
    assert_eq!(last["type"], "result", "{stdout:?}");
    (last["status"].clone(), last["result"]["result"].clone())
}

#[cfg(unix)]
#[test]
fn no_adapter_puts_its_harness_stderr_on_the_wire() {
    let mut env = EnvGuard::lock();
    let mut visited = Vec::new();
    let mut next = Some(AdapterKind::Claude);
    while let Some(kind) = next {
        visited.push(kind);
        for code in [0, 3] {
            let dir = tempfile::tempdir().unwrap();
            let shim = executable(dir.path(), "harness", &harness(dir.path(), code));
            let (extra, after) = staged(kind, &mut env, dir.path(), &shim);
            next = after;
            let result = dir.path().join("result.json");
            let mut input = json!({"role_path": "/dev/null", "allowed_results": ["complete"],
                                   "result_path": result, "workdir": dir.path()});
            if kind == AdapterKind::Exec {
                input["dialect_exec"] = json!({"failure_result": "fail"});
            }
            let stdout = served(kind, &extra, input);
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
            for message in &stdout {
                assert!(!message.contains(SENTINEL), "{kind:?} {code}: {message}");
            }
        }
    }
    assert_eq!(visited.len(), 5, "{visited:?}");
}
