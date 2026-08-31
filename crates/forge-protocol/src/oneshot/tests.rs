use super::*;
use serde_json::json;

use crate::{Body, Message, ResultStatus};

/// The literal the scripted driver rewrites with the effect id the
/// transport actually chose — the transport refuses a foreign one.
const EID: &str = "__EID__";

fn wire(body: Body) -> String {
    serde_json::to_string(&Message::new(body)).unwrap()
}

/// A real `forge-driver/v1` participant in `sh`: greet, accept,
/// checkpoint, conclude, then wait for shutdown. The bodies are composed
/// with serde and rewritten with the live effect id at run time, so the
/// test never hand-writes protocol JSON.
struct Scripted {
    dir: tempfile::TempDir,
}

impl Scripted {
    fn new(conclusion: Body, checkpoints: Vec<Value>) -> Scripted {
        let dir = tempfile::tempdir().unwrap();
        let capabilities = wire(Body::Capabilities {
            driver: "test".into(),
            version: "1".into(),
            supports: Vec::new(),
        });
        let mut bodies = vec![wire(Body::Accepted {
            effect_id: EID.into(),
            attempt_id: "a".into(),
            session_ref: None,
        })];
        for data in checkpoints {
            bodies.push(wire(Body::Checkpoint {
                effect_id: EID.into(),
                attempt_id: "a".into(),
                data,
            }));
        }
        bodies.push(wire(conclusion));
        std::fs::write(dir.path().join("capabilities"), capabilities + "\n").unwrap();
        std::fs::write(dir.path().join("bodies"), bodies.join("\n") + "\n").unwrap();
        std::fs::write(
            dir.path().join("driver.sh"),
            concat!(
                "read -r hello\n",
                "cat \"$1/capabilities\"\n",
                "read -r start\n",
                "id=$(printf '%s' \"$start\" | sed 's/.*\"effect_id\":\"\\([^\"]*\\)\".*/\\1/')\n",
                "sed \"s/__EID__/$id/\" \"$1/bodies\"\n",
                "echo 'a word of stderr' >&2\n",
                "read -r bye\n",
            ),
        )
        .unwrap();
        Scripted { dir }
    }

    fn command(&self) -> Vec<String> {
        vec![
            "sh".into(),
            self.dir.path().join("driver.sh").to_string_lossy().into(),
            self.dir.path().to_string_lossy().into(),
        ]
    }
}

fn concluded(status: ResultStatus, result: Option<Value>, error: Option<String>) -> Body {
    Body::Result {
        effect_id: EID.into(),
        attempt_id: "a".into(),
        status,
        result,
        error,
    }
}

fn seconds(count: u64) -> Duration {
    Duration::from_secs(count)
}

#[test]
fn a_produced_attempt_carries_its_result_and_checkpoints() {
    let driver = Scripted::new(
        concluded(
            ResultStatus::Succeeded,
            Some(json!({"result": "proposed"})),
            None,
        ),
        vec![json!({"step": "working"})],
    );
    let mut seen = std::path::PathBuf::new();
    let outcome = run_once(&driver.command(), "muninn", seconds(30), |scratch| {
        assert!(scratch.is_dir(), "the seat is given a scratch directory");
        seen = scratch.to_path_buf();
        json!({"scratch": scratch.to_string_lossy()})
    });
    match outcome {
        OneShot::Produced {
            result,
            checkpoints,
        } => {
            assert_eq!(result, json!({"result": "proposed"}));
            assert_eq!(checkpoints, vec![json!({"step": "working"})]);
        }
        OneShot::Refused { reason } => panic!("expected a result: {reason}"),
    }
    assert!(
        !seen.exists(),
        "the scratch directory does not outlive the seat"
    );
}

#[test]
fn a_failed_a_silent_and_an_unspawnable_driver_all_refuse() {
    let driver = Scripted::new(
        concluded(
            ResultStatus::Failed,
            None,
            Some("scripted refusal".to_string()),
        ),
        Vec::new(),
    );
    let reason = match run_once(&driver.command(), "muninn", seconds(30), |_| json!({})) {
        OneShot::Refused { reason } => reason,
        OneShot::Produced { .. } => panic!("a failed result is not a proposal"),
    };
    assert!(reason.contains("scripted refusal"), "{reason}");
    assert!(reason.contains("a word of stderr"), "{reason}");

    // Exits without ever accepting: indeterminate, and still nothing.
    let silent = run_once(
        &["sh".to_string(), "-c".to_string(), "exit 0".to_string()],
        "muninn",
        seconds(30),
        |_| json!({}),
    );
    match silent {
        OneShot::Refused { reason } => assert!(reason.contains("before accepting"), "{reason}"),
        OneShot::Produced { .. } => panic!("a silent driver produces nothing"),
    }

    let missing = run_once(
        &["forge-no-such-driver-binary".to_string()],
        "muninn",
        seconds(30),
        |_| json!({}),
    );
    match missing {
        OneShot::Refused { reason } => {
            assert!(reason.starts_with("driver did not spawn"), "{reason}")
        }
        OneShot::Produced { .. } => panic!("a missing binary produces nothing"),
    }
}

#[test]
fn an_unstageable_scratch_directory_refuses_before_anything_spawns() {
    let outcome = run_once_in(
        Err(std::io::Error::other("no space left on device")),
        &["forge-never-spawned".to_string()],
        "muninn",
        seconds(30),
        |_| panic!("the input is never built without a scratch directory"),
    );
    match outcome {
        OneShot::Refused { reason } => {
            assert!(
                reason.contains("could not stage a scratch directory"),
                "{reason}"
            );
        }
        OneShot::Produced { .. } => panic!("nothing ran"),
    }
}

#[test]
fn the_stderr_tail_is_clamped_on_a_character_boundary() {
    // A trailing ASCII byte puts the clamp one byte INSIDE a two-byte
    // character, which is the case the walk back exists for: slicing
    // there would panic, and a panic in a refusal message is worse than
    // any lost byte.
    let wide = "é".repeat(STDERR_TAIL) + "a";
    let clamped = tail(&wide);
    assert_eq!(
        clamped.len(),
        STDERR_TAIL + 1,
        "the walk back to a boundary keeps one byte more, never a panic"
    );
    assert!(wide.ends_with(clamped));
    assert_eq!(tail("short"), "short");
}
