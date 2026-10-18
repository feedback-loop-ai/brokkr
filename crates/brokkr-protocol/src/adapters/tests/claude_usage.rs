//! Claude's result usage as the driver folds it into the session record:
//! each result reports its own query, so the results are summed, the
//! thinking count is a subset of the output count or it is absent
//! (seat-record v6; decision 0035 ruling 4), and a resumed process's cost
//! is its own, less what `--resume` restored.

use super::super::*;

/// The session record after Claude's `result` events, each as
/// `(output_tokens, thinking_tokens)`, folded in order into one session.
fn folded(results: &[(u64, u64)]) -> (Option<Value>, Option<Value>) {
    let mut fold = ClaudeFold::default();
    fold.turns = 1;
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    for (output, thinking) in results {
        let event = json!({"type": "result", "num_turns": 92, "usage": {"input_tokens": 4,
            "output_tokens": output, "output_tokens_details": {"thinking_tokens": thinking}}});
        fold_stream_event(&event, &mut fold, &mut meta, &mut transcript, &mut |_| {});
    }
    (
        meta.get("output_tokens").cloned(),
        meta.get("reasoning_output_tokens").cloned(),
    )
}

#[test]
fn claudes_thinking_is_a_subset_of_its_output_or_it_is_left_out() {
    // Claude Code 2.1.300 reported a long session's thinking above its
    // output on the result; the store refuses such a record and the
    // seat failed at its last step. The count is kept when it is a
    // subset, left out when it cannot be one, and never zero.
    assert_eq!(folded(&[(50, 40)]), (Some(json!(50)), Some(json!(40))));
    assert_eq!(folded(&[(50, 50)]), (Some(json!(50)), Some(json!(50))));
    assert_eq!(folded(&[(43, 900)]), (Some(json!(43)), None));
    assert_eq!(folded(&[(43, 0)]), (Some(json!(43)), None));
}

#[test]
fn a_later_result_adds_its_thinking_and_output_to_the_earlier_ones() {
    // A message queued into the session (a background task's notice)
    // runs another query, and that query ends with a result reporting
    // its own usage only. Claude Code 2.1.302 reported 176 output and 43
    // thinking, then 51 and 32, its `modelUsage` 227 and 75.
    assert_eq!(
        folded(&[(176, 43), (51, 32)]),
        (Some(json!(227)), Some(json!(75)))
    );
    // A query that did not think adds no thinking, and a sum that cannot
    // be a subset of the summed output is left out: a stale count above
    // the output failed a review seat at its last step (landing-the-
    // console-library-slic-b5217290).
    assert_eq!(
        folded(&[(50, 40), (5, 0)]),
        (Some(json!(55)), Some(json!(40)))
    );
    assert_eq!(folded(&[(50, 40), (30, 900)]), (Some(json!(80)), None));
}

/// One assistant message as Claude Code streams it: a line per content
/// block, each repeating the message's id and usage.
fn message(id: &str, blocks: &[&str], read: u64, write: u64, output: u64) -> Vec<Value> {
    let usage = json!({"input_tokens": 2, "cache_read_input_tokens": read,
        "cache_creation_input_tokens": write, "output_tokens": output});
    let line = |block: &&str| {
        json!({"type": "assistant", "message": {"id": id, "model": "claude-sonnet-5-5",
            "usage": usage, "content": [{"type": block, "name": "Bash"}]}})
    };
    blocks.iter().map(line).collect()
}

#[test]
fn a_queued_query_journals_beside_the_first_not_over_it() {
    // The stream Claude Code 2.1.302 wrote on 2026-10-18 for a session
    // that started a background `sleep`, ended its turn, and answered
    // the task's notice in a second query, with the usage it reported.
    // Before the fix the record kept the second query alone: one turn
    // and 22,927 input of the session's 56,776.
    let result = |turns: u64, read: u64, write: u64, output: u64| {
        json!({"type": "result", "num_turns": turns, "usage": {"input_tokens": 4,
            "cache_read_input_tokens": read, "cache_creation_input_tokens": write,
            "output_tokens": output}})
    };
    let mut stream = message("msg_a", &["thinking", "tool_use"], 11_228, 11_389, 16);
    stream.extend(message("msg_b", &["text"], 22_617, 306, 3));
    stream.push(result(2, 33_845, 11_695, 114));
    stream.extend(message("msg_c", &["text"], 22_923, 419, 3));
    let mut fold = ClaudeFold::default();
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    let mut fold_into = |event: &Value, meta: &mut Map<String, Value>| {
        fold_stream_event(event, &mut fold, meta, &mut transcript, &mut |_| {});
    };
    for event in &stream {
        fold_into(event, &mut meta);
    }
    // Between results the record is the settled first query plus the
    // live estimate of the second, the first's repeated usage settled.
    assert_eq!(meta["cache_read_tokens"], 33_845 + 22_923);
    fold_into(&result(1, 22_923, 419, 3), &mut meta);
    // The sums are what its `modelUsage` carried: 8 uncached input,
    // 56,768 read, 12,114 written and 117 output.
    let usage = [
        "num_turns",
        "input_tokens",
        "cache_read_tokens",
        "cache_write_tokens",
    ];
    let usage = usage.map(|key| meta[key].clone());
    assert_eq!(
        usage,
        [json!(3), json!(56_776), json!(56_768), json!(12_114)]
    );
    assert_eq!(meta["output_tokens"], 117);
}

#[test]
fn a_message_streamed_line_by_block_is_one_turn_counted_once() {
    // Claude Code 2.1.302 streamed its first message as a thinking line
    // and a tool_use line, each repeating the message's id and usage. They
    // are one turn, its usage on the first checkpoint and in the live sum
    // once; the next message is the next turn.
    let mut stream = message("msg_a", &["thinking", "tool_use"], 11_228, 11_389, 16);
    stream.extend(message("msg_b", &["text"], 22_617, 306, 3));
    let mut fold = ClaudeFold::default();
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    let mut rows = Vec::new();
    for event in &stream {
        let mut emit = |row: &Value| rows.push(row.clone());
        fold_stream_event(event, &mut fold, &mut meta, &mut transcript, &mut emit);
    }
    let turns: Vec<_> = rows
        .iter()
        .map(|row| (row["turn"].clone(), row.get("cache_read_tokens").cloned()))
        .collect();
    let read = |count: u64| Some(json!(count));
    assert_eq!(
        turns,
        [
            (json!(1), read(11_228)),
            (json!(1), None),
            (json!(2), read(22_617))
        ]
    );
    let live = [&meta["num_turns"], &meta["cache_read_tokens"]];
    assert_eq!(live, [&json!(2), &json!(11_228 + 22_617)]);
}

/// A claude home holding one project whose transcript of `session` is
/// `lines`, one record a line as Claude Code writes them.
fn home_with(session: &str, lines: &[String]) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("-work-repo");
    std::fs::create_dir(&project).unwrap();
    let body: String = lines.iter().map(|line| format!("{line}\n")).collect();
    std::fs::write(project.join(format!("{session}.jsonl")), body).unwrap();
    home
}

/// The cost a process rejoining `session` journals for a result of `total`.
fn resumed_cost(home: &Path, session: &str, total: f64) -> Option<Value> {
    let mut fold = ClaudeFold::rejoining(home, Some(session));
    let mut meta = Map::new();
    fold.settle(
        &json!({"type": "result", "total_cost_usd": total}),
        &mut meta,
    );
    meta.get("total_cost_usd").cloned()
}

#[test]
fn a_resumed_process_journals_its_own_cost_not_the_session_it_restored() {
    // Claude Code 2.1.302 on 2026-10-18: the session's newest cost-state
    // said $0.0553188, and the resumed process's one short query ended in
    // a result saying $0.057847, the restored cost and its own together.
    let id = "a9db5870-28af-45b1-9e2d-f35a16b85394";
    let state = |session: &str, cost: f64| {
        let record = json!({"type": "cost-state", "sessionId": session, "totalCostUSD": cost});
        record.to_string()
    };
    // Newest last. A cut-off line, a negative figure, a message naming the
    // record's type and another session's record are none of them what
    // was restored.
    let home = home_with(
        id,
        &[
            state(id, 0.012),
            state(id, 0.055_318_8),
            state("b0b0", 0.04),
            json!({"type": "user", "message": {"content": "cost-state"}}).to_string(),
            state(id, -1.0),
            r#"{"type": "cost-state", "sessionId": "#.to_string(),
        ],
    );
    assert_eq!(
        resumed_cost(home.path(), id, 0.057_847),
        Some(json!(0.057_847 - 0.055_318_8))
    );
    // A total below what was restored cannot be split, so it is absent.
    assert_eq!(resumed_cost(home.path(), id, 0.05), None);
    // With nothing restored, a result's cost is the process's own: no
    // transcript of that session, no claude home, an id that is no id.
    let nowhere = home.path().join("absent");
    for (home, session) in [
        (home.path(), "c0ffee"),
        (&*nowhere, id),
        (home.path(), "-r"),
    ] {
        assert_eq!(resumed_cost(home, session, 0.25), Some(json!(0.25)));
    }
}
