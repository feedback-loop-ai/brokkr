//! #1199, F5: no adapter puts a byte of its harness's stderr on the wire
//! the engine journals from. Everything the engine journals arrives as a
//! wire message or as the driver's own stderr, which the engine names by
//! evidence alone, so a sentinel the harness writes only to its stderr,
//! a `model:` header line included, must reach no wire message, whether
//! the harness delivered its result or failed.

use super::tests::executable;
use super::*;
use crate::env_guard::EnvGuard;

/// What the harness prints that no seat binds, so no driver masks it.
const SENTINEL: &str = "UNBOUND-SENTINEL-1199";

/// The passthrough `kind`'s seat runs with, its harness pinned to `shim`.
/// Exhaustive with no wildcard: a new kind must say how it is staged.
fn staged(kind: AdapterKind, env: &mut EnvGuard, dir: &Path, shim: &Path) -> Vec<String> {
    let pinned = |model: &str| vec!["--model".to_string(), model.to_string()];
    match kind {
        AdapterKind::Claude => {
            env.set("BROKKR_CLAUDE_BIN", shim);
            pinned("claude-x")
        }
        AdapterKind::Lanetally => {
            env.set("BROKKR_LANETALLY_BIN", shim);
            env.set("BROKKR_CLAUDE_BIN", shim);
            pinned("claude-x")
        }
        AdapterKind::Codex => {
            env.set("BROKKR_CODEX_BIN", shim);
            pinned("gpt-x")
        }
        AdapterKind::Dsh => {
            env.set("BROKKR_DSH_BIN", shim);
            env.set("DSH_HOME", dir);
            Vec::new()
        }
        // The exec seat is a dialect validator: its command is the shim.
        AdapterKind::Exec => vec![shim.to_string_lossy().into_owned()],
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

#[cfg(unix)]
#[test]
fn no_adapter_puts_its_harness_stderr_on_the_wire() {
    let mut env = EnvGuard::lock();
    let kinds = [
        AdapterKind::Claude,
        AdapterKind::Lanetally,
        AdapterKind::Codex,
        AdapterKind::Dsh,
        AdapterKind::Exec,
    ];
    for kind in kinds {
        for code in [0, 3] {
            let dir = tempfile::tempdir().unwrap();
            let shim = executable(dir.path(), "harness", &harness(dir.path(), code));
            let extra = staged(kind, &mut env, dir.path(), &shim);
            let result = dir.path().join("result.json");
            let mut input = json!({"role_path": "/dev/null", "allowed_results": ["complete"],
                                   "result_path": result, "workdir": dir.path()});
            if kind == AdapterKind::Exec {
                input["dialect_exec"] = json!({"failure_result": "fail"});
            }
            let start = json!({"input": input, "attempt_id": "attempt", "effect_id": "effect"});
            let mut wire = Vec::new();
            run_seat(kind, &extra, &start, None, &mut |body| {
                wire.push(serde_json::to_string(&body).unwrap())
            });
            assert!(dir.path().join("ran").exists(), "{kind:?} {code}: {wire:?}");
            assert!(
                wire.last()
                    .is_some_and(|last| last.contains(r#""type":"result""#)),
                "{kind:?} {code}: the seat ends in a result: {wire:?}"
            );
            for message in &wire {
                assert!(!message.contains(SENTINEL), "{kind:?} {code}: {message}");
            }
        }
    }
}
