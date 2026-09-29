//! Shared by the heap-budget binaries (#342): one deterministic transcript
//! per kind at the reader's source cap, and the peak heap its projection
//! holds, measured by dhat. dhat allows one profiler per process, so each
//! kind is its own test binary and this module is its whole body. The
//! binaries live here, in the crate that serves the transcript, rather than
//! in brokkr-view: a pure crate's dependency set is closed (decision 0071
//! ruling 1), and a heap profiler clocks, walks stacks and writes a file,
//! which no pure crate may name, even in its tests.

use std::path::PathBuf;

use brokkr_view::transcript::{project, Snapshot, TranscriptKind, SOURCE_CAP};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// A Claude session: a user question, then an assistant answer that calls
/// a tool, alternating.
fn claude_row(index: usize) -> String {
    match index % 2 {
        0 => format!(
            "{{\"type\":\"user\",\"message\":{{\"content\":[{{\"type\":\"text\",\
             \"text\":\"question {index}: why does the fold refuse this journal?\"}}]}}}}\n"
        ),
        _ => format!(
            "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[\
             {{\"type\":\"text\",\"text\":\"answer {index}: the sequence is not contiguous\"}},\
             {{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{{\"file_path\":\"src/fold.rs\"}}}}]}},\
             \"timestamp\":\"2026-09-26T00:00:00Z\"}}\n"
        ),
    }
}

/// A Codex thread: a message, a reasoning summary, a call and its output.
fn codex_row(index: usize) -> String {
    let payload = match index % 4 {
        0 => format!(
            "{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\
             \"text\":\"request {index}: run the suite\"}}]}}"
        ),
        1 => format!(
            "{{\"type\":\"reasoning\",\"summary\":[{{\"type\":\"summary_text\",\
             \"text\":\"thinking {index} about the suite\"}}]}}"
        ),
        2 => format!(
            "{{\"type\":\"function_call\",\"name\":\"shell\",\"call_id\":\"c{index}\",\
             \"arguments\":\"{{}}\"}}"
        ),
        _ => format!(
            "{{\"type\":\"function_call_output\",\"call_id\":\"c{}\",\
             \"output\":\"test result: ok. {index} passed\"}}",
            index - 1
        ),
    };
    format!("{{\"timestamp\":\"t{index}\",\"type\":\"response_item\",\"payload\":{payload}}}\n")
}

/// A DSH session streaming its answer: a user message, then text deltas,
/// which is the shape that once took a console to 1.8 GiB (#277).
fn dsh_row(index: usize) -> String {
    let turn = index / 64;
    match index % 64 {
        0 => format!(
            "{{\"type\":\"user/message\",\"data\":{{\"content\":[{{\"type\":\"text\",\
             \"text\":\"question {turn}\"}}]}},\"time\":{index}}}\n"
        ),
        step => format!(
            "{{\"type\":\"assistant/chunk\",\"seq\":{index},\"time\":{index},\"data\":\
             {{\"turn\":{turn},\"step\":1,\"chunk\":{{\"type\":\"text-delta\",\
             \"text\":\"delta {step} of turn {turn} \"}}}}}}\n"
        ),
    }
}

/// The largest source of `kind` the reader admits whole: rows until the
/// next would pass `SOURCE_CAP`.
fn source(kind: TranscriptKind) -> Vec<u8> {
    let (header, row): (&str, fn(usize) -> String) = match kind {
        TranscriptKind::ClaudeSession => ("", claude_row),
        TranscriptKind::CodexThread => ("", codex_row),
        TranscriptKind::DshSession => ("{\"type\":\"session\",\"version\":0}\n", dsh_row),
        TranscriptKind::None => panic!("the `None` kind names no transcript to measure"),
    };
    let cap = usize::try_from(SOURCE_CAP).expect("the cap fits in memory");
    let mut text = String::with_capacity(cap);
    text.push_str(header);
    for index in 0.. {
        let next = row(index);
        if text.len() + next.len() > cap {
            break;
        }
        text.push_str(&next);
    }
    text.into_bytes()
}

/// The committed budget of `kind`, read from `quality/heap-bytes.json` by
/// the kind's own name. A kind with no whole-number budget refuses.
fn budget(kind: TranscriptKind) -> u64 {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../quality/heap-bytes.json");
    let text = std::fs::read_to_string(&path).expect("quality/heap-bytes.json reads");
    let file: serde_json::Value =
        serde_json::from_str(&text).expect("quality/heap-bytes.json is JSON");
    let name = serde_json::to_value(kind).expect("a kind serializes");
    let name = name.as_str().expect("a kind serializes to its name");
    file["budgets"][name]
        .as_u64()
        .unwrap_or_else(|| panic!("quality/heap-bytes.json holds no whole budget for {name}"))
}

/// Project the largest admitted source of `kind` under dhat and hold its
/// peak heap to the committed budget. The source is built before the
/// profiler starts, so only the projection's own allocations count. The
/// measurement is printed for `scripts/measure-budgets.sh`.
pub(crate) fn assert_within(kind: TranscriptKind) {
    let bytes = source(kind);
    let profiler = dhat::Profiler::builder().testing().build();
    let snapshot = Snapshot {
        bytes: &bytes,
        overflow: false,
        eof: true,
    };
    let projection = project(kind, &snapshot);
    let peak = dhat::HeapStats::get().max_bytes as u64;
    drop(profiler);
    assert!(projection.unavailable.is_none(), "the source projects");
    assert!(!projection.turns.is_empty(), "the projection holds turns");
    let name = serde_json::to_value(kind).expect("a kind serializes");
    println!(
        "heap\t{}\t{peak}",
        name.as_str().expect("a kind serializes to its name")
    );
    let budget = budget(kind);
    assert!(
        peak <= budget,
        "projecting {} bytes of {kind:?} into {} turns held {peak} heap bytes at its peak; \
         the budget is {budget}",
        bytes.len(),
        projection.turns.len()
    );
}
