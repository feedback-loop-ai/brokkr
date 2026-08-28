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

/// Seat keys carry ':' member separators (`seat:step:member`), which
/// Windows cannot spell in a file name — NTFS reads ':' as a stream
/// separator, and a second one is an outright error. State files use a
/// sanitized spelling; a hash of the original key keeps distinct seats
/// distinct after sanitizing.
fn attempt_file_name(seat: &str) -> String {
    let safe: String = seat
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a
    for byte in seat.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{safe}-{hash:016x}.attempt")
}

pub fn run_fake_driver(script_path: &Path, state_dir: &Path) -> std::io::Result<()> {
    let script: Value = serde_json::from_str(&std::fs::read_to_string(script_path)?)?;
    std::fs::create_dir_all(state_dir)?;
    let stdin = std::io::stdin();
    run_fake_session(
        &script,
        state_dir,
        stdin.lock(),
        std::io::stdout(),
        std::thread::park,
    )
}

fn run_fake_session(
    script: &Value,
    state_dir: &Path,
    input: impl BufRead,
    mut output: impl Write,
    mut hang: impl FnMut(),
) -> std::io::Result<()> {
    let mut send = |body: Body| -> std::io::Result<()> {
        let line = serde_json::to_string(&Message::new(body))?;
        output.write_all(line.as_bytes())?;
        output.write_all(b"\n")?;
        output.flush()
    };

    for line in input.lines() {
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
                let counter_file = state_dir.join(attempt_file_name(&seat));
                let attempt_index: usize = std::fs::read_to_string(&counter_file)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                std::fs::write(&counter_file, (attempt_index + 1).to_string())?;

                let entries = script["seats"][&seat]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
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
                        output.write_all(b"this is not a protocol message\n")?;
                        output.flush()?;
                        return Ok(());
                    }
                    "hang" => {
                        // Accepted, then silence: the engine's deadline
                        // watchdog must kill this attempt.
                        hang();
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

#[cfg(test)]
mod tests;
