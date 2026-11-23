//! The exec site's invocation: no model turn, no provider and no tool
//! target. It lives here so `invoke_with_stager` stays inside its measured
//! ceiling.

use super::*;

pub(super) fn invoke_exec(
    extra: &[String],
    prompt: &str,
    workdir: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
    mut stage: impl FnMut(&str) -> Result<tempfile::NamedTempFile, String>,
) -> Result<Invocation, String> {
    if extra.is_empty() {
        return Err("exec driver needs a command template after '--'".to_string());
    }
    // Exec has no model turn and therefore no tool target in the
    // seat record. Its unresolved command remains pinned in the
    // bundle manifest; no command-shaped prose enters a checkpoint.
    let mut session_meta = Map::new();
    Transcript::resolve(TranscriptKind::None)?.finish(&mut session_meta, emit);
    emit(&Emitted::new(Step::ExecStarted, json!({})));
    let mut prompt_file: Option<tempfile::NamedTempFile> = None;
    if extra.iter().any(|part| part.contains("{prompt_file}")) {
        prompt_file = Some(stage(prompt)?);
    }
    let prompt_path = prompt_file
        .as_ref()
        .map(|f| f.path().to_string_lossy().into_owned())
        .unwrap_or_default();
    let command: Vec<String> = extra
        .iter()
        .map(|part| resolve_exec_part(part, workdir, &prompt_path, bindings))
        .collect();
    let stdin_payload = if prompt_file.is_none() {
        Some(prompt)
    } else {
        None
    };
    let out = run_cli(&command, stdin_payload, workdir, bindings)?;
    // Known-plaintext masking choke point (decision 0012, layer
    // 5), on RAW captured bytes before any string conversion.
    // stdout becomes a dialect check's notes; stderr is named there by
    // evidence over those same masked bytes, re-emitted by run_seat,
    // and its tail told to the engine's own log on failure (the
    // journal names it by evidence alone, #1199) — the path that
    // fires when a credentialed command prints it.
    let stderr = secret::mask_bytes(&out.stderr, bindings);
    let notes = dialect_notes(&secret::mask_bytes(&out.stdout, bindings), &stderr);
    let state = input
        .pointer("/dialect_exec/state")
        .and_then(Value::as_array)
        .filter(|argv| !argv.is_empty())
        .map(|argv| {
            let argv = argv
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            run_cli(&argv, None, workdir, bindings).map(|output| {
                let bytes = secret::mask_bytes(&output.stdout, bindings);
                String::from_utf8_lossy(&bytes).into_owned()
            })
        })
        .transpose();
    let stderr_text = String::from_utf8_lossy(&stderr);
    let state = match state {
        Ok(state) => state,
        Err(mut error) => {
            let start = stderr_tail_start(&stderr_text);
            let _ = std::io::stderr().write_all(stderr_text[start..].as_bytes());
            if let Some(evidence) = validator_evidence(&stderr) {
                error.push('\n');
                error.push_str(&evidence);
            }
            return Err(error);
        }
    };
    Ok(Invocation {
        answer: None,
        exit_code: lost::exit_code(out.status),
        session_meta,
        notes,
        stderr: stderr_text.into_owned(),
        state,
        // Decision 0053: exec has no model turn and no provider
        // to refuse it. Its failures are the script's own, and
        // an inline exec site carries an empty chain anyway, so
        // there is no fallback for a refusal to reach.
        refusal: None,
        launch: LaunchTerminal::Cold,
    })
}

/// A dialect check's notes: its validator's trimmed stdout, then, where
/// its stderr is not empty, one line naming that stderr by content-free
/// evidence (#1199, decision 0012). The notes reach the journal as a step
/// checkpoint or as the attempt's result, and a validator can print a
/// value the seat binds nowhere, so no byte of its stderr is copied here:
/// a realm that wants its validator's diagnostics in the notes redirects
/// them to stdout inside its own command. The evidence counts and digests
/// the captured masked bytes themselves, never a lossy reading of them.
fn dialect_notes(stdout: &[u8], stderr: &[u8]) -> String {
    let mut notes = String::from_utf8_lossy(stdout).trim().to_string();
    if let Some(evidence) = validator_evidence(stderr) {
        if !notes.is_empty() {
            notes.push('\n');
        }
        notes.push_str(&evidence);
    }
    notes
}

fn validator_evidence(stderr: &[u8]) -> Option<String> {
    if stderr.is_empty() {
        return None;
    }
    let evidence = crate::process::Evidence::of(stderr);
    Some(format!("validator stderr: {evidence}"))
}

#[cfg(test)]
mod tests;
