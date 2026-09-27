#![cfg(unix)]

//! A driver whose override cannot be read refuses its seat by name, and
//! runs nothing: neither what the override pins nor the built-in one on
//! `PATH`. That is an override set only by the spelling decision 0019
//! retired, and a current one whose value is not UTF-8 (landing reviews
//! of #355, 2026-09-27). Proved against the built binary.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

/// The retired prefix, split so the rename's grep finds no live spelling.
const RETIRED: &str = concat!("FOR", "GE_");

/// Each driver: its kind, its override's stem, and its built-in name.
const DRIVERS: [(&str, &str, &str); 5] = [
    ("claude", "CLAUDE_BIN", "claude"),
    ("lanetally", "LANETALLY_BIN", "claude-lanetally"),
    ("codex", "CODEX_BIN", "codex"),
    ("dsh", "DSH_BIN", "dsh"),
    ("exec", "EXEC_NAME", "exec-template"),
];

/// A script that leaves `marker` behind when anything runs it.
fn marking_script(path: &Path, marker: &Path) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_path_buf()
}

/// One driver's seat with `variable` pinning a marking script, or `pin`
/// in its place when given: its wire messages and whether anything ran.
fn drive_pinned(
    kind: &str,
    variable: &str,
    pin: Option<&OsStr>,
    builtin: &str,
) -> (Vec<Value>, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let marks = dir.path().join("marks");
    std::fs::create_dir_all(&marks).unwrap();
    let script = marking_script(&dir.path().join("pinned/bin"), &marks.join("pinned"));
    let pinned = pin.map_or_else(|| script.into_os_string(), OsString::from);
    marking_script(
        &dir.path().join("path").join(builtin),
        &marks.join("builtin"),
    );
    let charter = dir.path().join("role.md");
    std::fs::write(&charter, "# charter\n").unwrap();
    let input = json!({"feature": "f", "phase": "p", "seat": "s", "role_path": charter,
        "workdir": dir.path(), "result_path": dir.path().join("result.json"),
        "allowed_results": ["complete"], "context": {}});
    let mut command = Command::new(env!("CARGO_BIN_EXE_brokkr"));
    command.args(["driver", kind]);
    if kind == "exec" {
        command.args(["--", builtin]);
    }
    for (_, current, _) in DRIVERS {
        command.env_remove(format!("BROKKR_{current}"));
    }
    let path = format!("{}:/usr/bin:/bin", dir.path().join("path").display());
    let mut child = command
        .env(variable, &pinned)
        .env("PATH", path)
        .env("HOME", dir.path())
        .env("CODEX_HOME", dir.path())
        .env("DSH_HOME", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for (msg_id, body) in [
        ("m1", json!({"type": "hello", "engine_version": "test"})),
        (
            "m2",
            json!({"type": "start", "effect_id": "fx", "attempt_id": "a1",
                      "seat": "s", "input": input}),
        ),
        ("m3", json!({"type": "shutdown"})),
    ] {
        let mut message = json!({"proto": "forge-driver/v1", "msg_id": msg_id});
        message
            .as_object_mut()
            .unwrap()
            .extend(body.as_object().unwrap().clone());
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{kind}: the driver itself exits cleanly"
    );
    let messages = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut ran: Vec<String> = std::fs::read_dir(&marks)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    ran.sort();
    (messages, ran)
}

/// A seat refused before any launch: its capabilities, then a failed
/// result carrying `error`, and nothing ran.
fn assert_refused(kind: &str, (messages, ran): (Vec<Value>, Vec<String>), error: &str) {
    let types: Vec<&str> = messages
        .iter()
        .map(|m| m["type"].as_str().unwrap())
        .collect();
    assert_eq!(types, ["capabilities", "result"], "{kind}: {messages:?}");
    assert_eq!(messages[1]["status"], "failed", "{kind}");
    assert_eq!(
        messages[1]["error"],
        format!("seat refused to start: {error}"),
        "{kind}"
    );
    assert_eq!(ran, Vec::<String>::new(), "{kind}: nothing ran");
}

#[test]
fn a_retired_override_alone_runs_neither_its_pin_nor_the_built_in() {
    for (kind, stem, builtin) in DRIVERS {
        let (retired, current) = (format!("{RETIRED}{stem}"), format!("BROKKR_{stem}"));
        let error = format!(
            "{retired} is set but is no longer read: it was renamed {current}; \
             set {current} instead, or unset {retired}"
        );
        assert_refused(kind, drive_pinned(kind, &retired, None, builtin), &error);
    }
}

/// The reproduction: the current name pins a path that is not UTF-8, and
/// the built-in is first on `PATH`. The one reader refuses the value by
/// name, so the built-in never runs in the pin's place.
#[test]
fn a_current_override_that_is_not_unicode_runs_nothing() {
    let pin = OsStr::from_bytes(b"/opt/\xff/pinned");
    for (kind, stem, builtin) in DRIVERS {
        let current = format!("BROKKR_{stem}");
        let error = format!(
            "{current} is set, but its value is not UTF-8, so what it names cannot be \
             read; give {current} a UTF-8 value, or unset it"
        );
        assert_refused(
            kind,
            drive_pinned(kind, &current, Some(pin), builtin),
            &error,
        );
    }
}
