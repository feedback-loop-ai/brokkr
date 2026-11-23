use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

use brokkr_protocol::adapters::{self, AdapterKind};
use brokkr_protocol::{secret, Body, Message};

use super::*;

const SERVING: &str = "BROKKR_TEST_0114A_SERVING";

struct Harness {
    kind: &'static str,
    script: &'static str,
    format: Format,
    calls: &'static [(&'static str, &'static str, &'static str)],
    secret: &'static str,
}

const CLAUDE_CALLS: &[(&str, &str, &str)] = &[
    ("toolu_1", "WebSearch", "Web[secret:W]ch"),
    ("toolu_2", "Bash", "[secret:B]"),
];

const HARNESSES: &[Harness] = &[
    Harness {
        kind: "claude",
        script: r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"assistant","message":{"id":"m","content":[{"type":"tool_use","id":"toolu_1","name":"WebSearch"},{"type":"tool_use","id":"toolu_2","name":"Bash"}]}}'
"#,
        format: Format::Claude,
        calls: CLAUDE_CALLS,
        secret: "Sear",
    },
    Harness {
        kind: "dsh",
        script: r#"#!/bin/sh
root=$(sed -n "s/^    root: '\(.*\)'/\1/p" "$4")
d="$root/--project--/session-fake"
mkdir -p "$d"
cat > "$d/session.v3.jsonl" <<'ROWS'
{"type":"session","version":3,"id":"session-fake","delegationDepth":0}
{"type":"assistant/message","seq":1,"data":{"turn":1,"step":1}}
{"type":"tool/call","seq":2,"data":{"callId":"toolu_1","name":"WebSearch"}}
{"type":"tool/call","seq":3,"data":{"callId":"toolu_2","name":"Bash"}}
ROWS
"#,
        format: Format::Dsh,
        calls: CLAUDE_CALLS,
        secret: "Sear",
    },
    Harness {
        kind: "codex",
        script: r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"type":"turn.started"}' '{"type":"item.completed","item":{"id":"item_1","type":"web_search"}}'
"#,
        format: Format::Codex,
        calls: &[("item_1", "web_search", "web_[secret:W]ch")],
        secret: "sear",
    },
];

/// Re-enter this libtest as the production driver, piping its start and
/// checkpoint stream without touching the test process's environment.
fn driver_calls(harness: &Harness, bound: bool) -> Vec<Value> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("harness");
    std::fs::write(&path, harness.script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let store = dir.path().join("secrets.env");
    let names = if bound {
        secret::store_set(&store, "W", harness.secret).unwrap();
        secret::store_set(&store, "B", "Bash").unwrap();
        vec!["W", "B"]
    } else {
        Vec::new()
    };
    let filter = concat!(module_path!(), "::bound_native_tool_name_fails_closed");
    let (_, filter) = filter.split_once("::").unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", filter, "--nocapture", "--quiet"])
        .env(SERVING, harness.kind);
    for bin in ["BROKKR_CLAUDE_BIN", "BROKKR_CODEX_BIN", "BROKKR_DSH_BIN"] {
        command.env(bin, &path);
    }
    for home in ["HOME", "CODEX_HOME", "DSH_HOME"] {
        command.env(home, dir.path());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let start = [
        Body::Hello {
            engine_version: "test".into(),
        },
        Body::Start {
            effect_id: "effect".into(),
            attempt_id: "attempt".into(),
            seat: "work".into(),
            probe: None,
            input: json!({
                "workdir":dir.path(), "result_path":dir.path().join("result.json"),
                "allowed_results":["complete"], "feature":"f", "phase":"work",
                "role_path":"/dev/null", "secrets":names, "secrets_file":store,
            }),
        },
    ]
    .map(|body| serde_json::to_string(&Message::new(body)).unwrap() + "\n")
    .concat();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(start.as_bytes()).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let wire = String::from_utf8(output.stdout).unwrap();
    let calls = wire
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(
            |line| match serde_json::from_str::<Message>(line).unwrap().body {
                Body::Checkpoint { data, .. } if data.get(OBSERVATION_KEY).is_some() => Some(data),
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), harness.calls.len(), "{wire}");
    calls
}

fn authority(harness: &Harness, held: bool) -> Calls {
    let mut calls = Calls {
        known: harness
            .calls
            .iter()
            .map(|(_, name, _)| name.to_string())
            .collect(),
        owner: Some(Owner {
            attempt: "attempt".into(),
            site: "site".into(),
            instance: "instance".into(),
            provider: harness.kind.into(),
            history: RootHistory::default(),
        }),
        ..Calls::default()
    };
    if held {
        for (_, name, _) in harness.calls {
            calls.held.insert(
                name.to_string(),
                Holder {
                    capability: "capability".into(),
                    dialect: "dialect".into(),
                },
            );
        }
    }
    calls
}

fn assert_calls(harness: &Harness, rows: &[Value], bound: bool) {
    for held in [false, true] {
        let mut calls = authority(harness, held);
        for (row, &(call, native, masked)) in rows.iter().zip(harness.calls) {
            let shown = if bound { masked } else { native };
            assert_eq!(row["tool"], shown);
            let observation: Observation =
                serde_json::from_value(row[OBSERVATION_KEY].clone()).unwrap();
            assert_eq!(
                observation,
                Observation {
                    format: harness.format,
                    call: Some(call.into()),
                    tool: Tool::Named { name: shown.into() },
                }
            );
            let verdict = calls.consume(&mut row.clone());
            if bound {
                assert_eq!(
                    verdict,
                    Err(Refusal::Unattributable),
                    "{}: {native}, held={held}",
                    harness.kind
                );
                assert!(calls.seen.is_empty());
            } else if held {
                assert_eq!(
                    verdict,
                    Ok(Some(Stamp {
                        tool: native.into(),
                        capability: "capability".into(),
                        dialect: "dialect".into(),
                        call_id: calls.owner.as_ref().unwrap().call_id(call),
                    }))
                );
            } else {
                assert_eq!(
                    verdict,
                    Err(Refusal::Unheld {
                        tool: native.into()
                    })
                );
            }
        }
    }
}

#[test]
fn bound_native_tool_name_fails_closed() {
    if let Ok(kind) = std::env::var(SERVING) {
        let kind = AdapterKind::parse(&kind).unwrap();
        adapters::serve(kind, Vec::new(), || Ok(())).unwrap();
        std::process::exit(0);
    }
    for harness in HARNESSES {
        for bound in [false, true] {
            assert_calls(harness, &driver_calls(harness, bound), bound);
        }
    }
}
