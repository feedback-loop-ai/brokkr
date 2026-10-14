//! A DSH 0.2 seat's session file under the driver (#678; change
//! `admit-dsh-session-v4`, design D7): the tail finds `session.v4.jsonl`,
//! so the seat's first turn crosses decision 0053's boundary while the
//! seat still works and its held transcript row reaches the journal then,
//! not with the finishing row. The warm admission's census keeps naming
//! the selected core's generation alone.

use super::tests::executable;
use super::*;
use crate::env_guard::EnvGuard;

/// A cold DSH seat that writes a version-four session file under the
/// root its overlay pins, then waits for `seen` — which the test's sink
/// writes only once the transcript row has been sent — before it writes
/// its result. A driver that never folds the version-four file holds the
/// row to the end, the wait runs out, and the seat fails (`exit 9`).
fn version_four_seat(seen: &Path, result: &Path) -> String {
    format!(
        r#"#!/bin/sh
root=
prev=
for a in "$@"; do
  if [ "$prev" = --patch ]; then root=$(awk -F"'" '/^    root: /{{print $2}}' "$a"); fi
  prev=$a
done
d="$root/--project--/session-fake"
mkdir -p "$d"
f="$d/session.v4.jsonl"
printf '{{"type":"session","version":4,"id":"session-fake","createdAt":1,"isSeeded":false,"delegationDepth":0}}\n' > "$f"
printf '{{"type":"assistant/message","seq":0,"time":2,"data":{{"turn":1,"step":1,"message":{{"id":"m","role":"assistant","source":{{"kind":"llm","model":"served-by-dsh"}},"content":[]}}}}}}\n' >> "$f"
i=0
while [ ! -f '{seen}' ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done
[ -f '{seen}' ] || exit 9
printf '{{"result":"complete"}}' > '{result}'
"#,
        seen = seen.display(),
        result = result.display()
    )
}

/// A cold work seat's request, writing its result to `result`.
fn request(workdir: &Path, result: &Path) -> Value {
    let input = json!({
        "role_path": "/dev/null", "phase": "work", "feature": "f",
        "allowed_results": ["complete"], "result_path": result, "workdir": workdir,
    });
    json!({"attempt_id": "attempt", "effect_id": "effect", "input": input})
}

/// A transcript root under `dir` holding one session directory named `id`.
fn session_under(dir: &Path, id: &str) -> (PathBuf, PathBuf) {
    let root = dir.join("root");
    let session = root.join("--project--").join(id);
    std::fs::create_dir_all(&session).unwrap();
    (root, session)
}

#[cfg(unix)]
#[test]
fn a_version_four_seat_journals_its_transcript_while_it_works() {
    let mut env = EnvGuard::lock();
    let dir = tempfile::tempdir().unwrap();
    let (seen, result) = (dir.path().join("seen"), dir.path().join("result.json"));
    let shim = executable(dir.path(), "dsh-v4", &version_four_seat(&seen, &result));
    env.set("BROKKR_DSH_BIN", &shim);
    env.set("DSH_HOME", dir.path());
    let mut messages = Vec::new();
    run_seat(
        AdapterKind::Dsh,
        &[],
        &request(dir.path(), &result),
        None,
        &mut |body| {
            if let Body::Checkpoint { data, .. } = &body {
                if data["step"] == "transcript" {
                    std::fs::write(&seen, b"").unwrap();
                }
            }
            messages.push(body);
        },
    );
    let steps: Vec<&str> = messages
        .iter()
        .filter_map(|body| match body {
            Body::Checkpoint { data, .. } => data["step"].as_str(),
            _ => None,
        })
        .collect();
    assert_eq!(
        steps,
        [
            "transcript",
            "harness-started",
            "seat-turn",
            "deepseek-harness-session-finished"
        ],
        "{messages:?}"
    );
    let turn = messages.iter().find_map(|body| match body {
        Body::Checkpoint { data, .. } if data["step"] == "seat-turn" => Some(data),
        _ => None,
    });
    assert_eq!(turn.unwrap()["model"], "served-by-dsh");
    assert!(
        matches!(
            messages.last(),
            Some(Body::Result {
                status: ResultStatus::Succeeded,
                ..
            })
        ),
        "the seat saw its row journaled while it worked: {messages:?}"
    );
}

/// The tail asks for the newest generation first: `session.v4.jsonl` is
/// found alone and preferred beside `session.v3.jsonl`. Names are
/// literals so an edit to a constant cannot move what this claims.
#[test]
fn the_tail_finds_the_version_four_name_first() {
    let dir = tempfile::tempdir().unwrap();
    let (root, session) = session_under(dir.path(), "s-1");
    let header = b"{\"type\":\"session\",\"id\":\"s\",\"delegationDepth\":0}\n";
    std::fs::write(session.join("session.v4.jsonl"), header).unwrap();
    assert_eq!(
        find_dsh_transcript(&root).as_deref(),
        Some(&*session.join("session.v4.jsonl"))
    );
    std::fs::write(session.join("session.v3.jsonl"), header).unwrap();
    assert_eq!(
        find_dsh_transcript(&root).as_deref(),
        Some(&*session.join("session.v4.jsonl"))
    );
}

/// The warm admission's census reads the selected core's
/// `session.v3.jsonl` alone (decision 0056): a stored version-four file
/// is no rejoinable session, so an offer naming it is refused, never
/// resolved to it.
#[test]
fn the_rejoin_census_does_not_admit_the_version_four_name() {
    let dir = tempfile::tempdir().unwrap();
    let (root, session) = session_under(dir.path(), "session-1");
    std::fs::write(
        session.join("session.v4.jsonl"),
        b"{\"type\":\"session\",\"version\":4,\"id\":\"session-1\",\"delegationDepth\":0}\n",
    )
    .unwrap();
    assert_eq!(
        dsh_session_file_with(&root, "session-1", 16),
        Err("dsh driver: the stored session file is unreadable".to_string())
    );
}
