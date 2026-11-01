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
    emit: &mut impl FnMut(&Value),
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
    emit(&json!({"step": "exec-started"}));
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
    // stdout is captured-and-dropped today; stderr is re-emitted
    // by run_seat and journaled as the stderr tail on failure —
    // the path that fires exactly when a credentialed command
    // prints the offending header.
    let stderr = secret::mask_bytes(&out.stderr, bindings);
    let stdout = secret::mask_bytes(&out.stdout, bindings);
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
        .transpose()?;
    Ok(Invocation {
        answer: None,
        exit_code: lost::exit_code(out.status),
        session_meta,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        state,
        // Decision 0053: exec has no model turn and no provider
        // to refuse it. Its failures are the script's own, and
        // an inline exec site carries an empty chain anyway, so
        // there is no fallback for a refusal to reach.
        refusal: None,
        launch: LaunchTerminal::Cold,
    })
}
