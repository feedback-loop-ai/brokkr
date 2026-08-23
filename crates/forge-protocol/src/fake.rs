//! The fake driver: a real `forge-driver/v1` participant over real stdio,
//! scripted for machine proof (delivery-sequence step 5). Behaviors per
//! seat advance one script entry per attempt via a state directory, so
//! retries observe the next scripted outcome across separate invocations.
//!
//! Script file (JSON):
//! ```json
//! { "seats": { "implement": [
//!     {"behavior": "succeed", "result": {"result": "complete"}},
//!     {"behavior": "fail", "error": "boom"},
//!     {"behavior": "vanish"},
//!     {"behavior": "garbage"}
//! ]}}
//! ```
//! `vanish` exits without a result (=> indeterminate); `garbage` writes a
//! non-protocol line (=> failed closed). The last entry repeats.

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::Value;

use crate::{Body, Message, ResultStatus};

pub fn run_fake_driver(script_path: &Path, state_dir: &Path) -> std::io::Result<()> {
    let script: Value = serde_json::from_str(&std::fs::read_to_string(script_path)?)?;
    std::fs::create_dir_all(state_dir)?;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    let mut send = |body: Body| -> std::io::Result<()> {
        let line = serde_json::to_string(&Message::new(body))?;
        stdout.write_all(line.as_bytes())?;
        stdout.write_all(b"\n")?;
        stdout.flush()
    };

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let message: Message = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue, // engine speaks the protocol; ignore noise
        };
        match message.body {
            Body::Hello { .. } => {
                send(Body::Capabilities {
                    driver: "fake".into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    supports: vec!["cancel".into()],
                })?;
            }
            Body::Start {
                effect_id,
                attempt_id,
                seat,
                ..
            } => {
                let counter_file = state_dir.join(format!("{seat}.attempt"));
                let attempt_index: usize = std::fs::read_to_string(&counter_file)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                std::fs::write(&counter_file, (attempt_index + 1).to_string())?;

                let entries = script["seats"][&seat].as_array().cloned().unwrap_or_default();
                let entry = entries
                    .get(attempt_index)
                    .or_else(|| entries.last())
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({"behavior": "vanish"}));

                send(Body::Accepted {
                    effect_id: effect_id.clone(),
                    attempt_id: attempt_id.clone(),
                    session_ref: Some(format!("fake-session-{seat}-{attempt_index}")),
                })?;

                match entry["behavior"].as_str().unwrap_or("vanish") {
                    "succeed" => {
                        send(Body::Checkpoint {
                            effect_id: effect_id.clone(),
                            attempt_id: attempt_id.clone(),
                            data: serde_json::json!({"step": "working"}),
                        })?;
                        send(Body::Result {
                            effect_id,
                            attempt_id,
                            status: ResultStatus::Succeeded,
                            result: Some(entry["result"].clone()),
                            error: None,
                        })?;
                    }
                    "fail" => {
                        send(Body::Result {
                            effect_id,
                            attempt_id,
                            status: ResultStatus::Failed,
                            result: None,
                            error: Some(
                                entry["error"].as_str().unwrap_or("scripted failure").into(),
                            ),
                        })?;
                    }
                    "garbage" => {
                        stdout.write_all(b"this is not a protocol message\n")?;
                        stdout.flush()?;
                        return Ok(());
                    }
                    "hang" => {
                        // Accepted, then silence: the engine's deadline
                        // watchdog must kill this attempt.
                        std::thread::sleep(std::time::Duration::from_secs(3600));
                        return Ok(());
                    }
                    _ => return Ok(()), // "vanish": exit without a result
                }
            }
            Body::Shutdown => return Ok(()),
            Body::Cancel { effect_id } => {
                send(Body::Cancelled { effect_id })?;
                return Ok(());
            }
            _ => {}
        }
    }
    Ok(())
}
