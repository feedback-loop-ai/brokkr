use super::decision_0114a::{bindings, Stream};
use super::*;

const LOCATOR: &str = "seat-1234";
const PROJECT: &str = "--w--";
const SESSION: &str = "session-1234";

fn append_rows(home: &Path, rows: &[Value]) {
    use std::io::Write;

    let transcript = home
        .join(LOCATOR)
        .join(PROJECT)
        .join(SESSION)
        .join(DSH_TRANSCRIPT);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(transcript)
        .unwrap();
    for row in rows {
        writeln!(file, "{row}").unwrap();
    }
}

fn message(seq: u64, model: &str) -> Value {
    json!({"type":"assistant/message", "seq":seq, "data":{
        "message":{"source":{"model":model}},
        "usage":{"inputTokens":5, "outputTokens":4, "cacheReadTokens":3, "reasoningTokens":2}
    }})
}

#[test]
fn bound_dsh_transcript_keeps_current_usage_and_masks_the_model() {
    let streams = Stream::rows(&[], &[("STRUCTURE", "type"), ("MODEL", "private")]);
    let bound = bindings(&streams);
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join(LOCATOR);
    plant_dsh_session(dir.path(), LOCATOR, PROJECT, SESSION, 2);
    append_rows(
        dir.path(),
        &[
            message(1, "old-model"),
            message(3, "valid-model"),
            message(4, "private-model"),
        ],
    );

    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted = Vec::new();
    drain_into(
        &root,
        &mut DshTail::default(),
        Some(2),
        &mut turns,
        &mut meta,
        &bound,
        &mut emitted,
    );
    assert_eq!(turns, 2);
    assert_eq!(emitted.len(), 2);
    assert_eq!(emitted[0]["model"], "valid-model");
    assert_eq!(emitted[1]["model"], MODEL_NOT_REPORTED);
    assert_eq!(meta["model"], "valid-model");
    for (key, count) in [
        ("input_tokens", 8),
        ("output_tokens", 4),
        ("cache_read_tokens", 3),
        ("reasoning_output_tokens", 2),
    ] {
        assert_eq!(emitted[0][key], count, "{key}");
        assert_eq!(emitted[1][key], count, "{key}");
        assert_eq!(meta[key], count * 2, "{key}");
    }
}
