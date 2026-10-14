//! Version four's scenarios (change `admit-dsh-session-v4`, delta
//! "DSH version four"): the header matrix, the projected kinds, every
//! quiet name, the namespaced unknown, the retired wrapper, the tool-role
//! result mapped field by field, the producer sources and the remapped
//! references, each against version three's reading of the same rows.

use serde_json::json;

use super::super::tests::{dsh, row};
use super::super::{Block, Projection, Turn, Unavailable};

/// The opening header the synthetic sessions share: version four, its
/// association gate open as the writer's native header records it.
fn header(version: &str) -> String {
    format!("{{\"type\":\"session\",\"version\":{version},\"id\":\"s\",\"createdAt\":1,\"isSeeded\":false,\"delegationDepth\":0}}\n")
}

/// `rows` under a version-four header.
fn four(rows: &[String]) -> Projection {
    dsh(&[header("4")]
        .iter()
        .chain(rows)
        .cloned()
        .collect::<String>())
}

/// `rows` under a version-three header.
fn three(rows: &[String]) -> Projection {
    dsh(&[header("3")]
        .iter()
        .chain(rows)
        .cloned()
        .collect::<String>())
}

fn turn(role: &str, ts: &str, block: Block) -> Turn {
    Turn {
        role: role.to_string(),
        ts: ts.to_string(),
        blocks: vec![block],
    }
}

/// A version-four `tool/result` row settling `call` with `content`.
fn tool_role(seq: i64, call: &str, content: serde_json::Value) -> String {
    row(json!({
        "type": "tool/result", "seq": seq, "time": 1000 + seq, "surfaceOp": "append",
        "data": {"turn": 1, "step": 1, "message": {
            "id": format!("m-{seq}"), "role": "tool", "toolCallId": call,
            "source": {"kind": "tool", "callId": call}, "content": content}}
    }))
}

/// A user message from `source` saying `text`.
fn user(seq: i64, source: serde_json::Value, text: &str) -> String {
    row(json!({
        "type": "user/message", "seq": seq, "time": 1000 + seq, "surfaceOp": "append",
        "data": {"id": format!("u-{seq}"), "role": "user", "source": source,
                 "content": [{"type": "text", "text": text}]}
    }))
}

/// A row of `kind` with an empty payload.
fn bare(kind: &str, seq: i64) -> String {
    row(json!({"type": kind, "seq": seq, "time": 1000 + seq, "data": {}}))
}

#[test]
fn the_version_four_header_admits_by_its_value_and_its_spelling() {
    for version in ["4", "4.0", "4e0", "40e-1", "0.4e1", "4.000"] {
        let projection = dsh(&header(version));
        assert_eq!(projection, Projection::default(), "version {version}");
    }
    for version in [
        "-4",
        "4e1",
        "4.1",
        "\"4\"",
        "5",
        "3.9999999999999999",
        "4.0000000000000001",
    ] {
        let projection = dsh(&format!("{}{}", header(version), bare("turn/start", 0)));
        assert_eq!(
            projection,
            Projection {
                unavailable: Some(Unavailable::UnsupportedFormat),
                ..Projection::default()
            },
            "version {version}"
        );
    }
}

/// The eighteen types a version-four seat log carries project their user,
/// assistant, call and tool-role result turns in order, the embedded call
/// shown once beside its dedicated row, with nothing counted.
#[test]
fn a_version_four_session_projects_its_turns_in_order() {
    let rows = [
        bare("turn/start", 0),
        bare("request/header", 1),
        bare("request/context", 2),
        bare("permission/preset", 3),
        bare("sandbox/mode", 4),
        bare("approval/policy", 5),
        bare("step/start", 6),
        row(
            json!({"type": "system/message", "seq": 7, "time": 1007, "data": {"turn": 1, "step": 1,
            "message": {"id": "m-s", "role": "system", "source": {"kind": "system-prompt"},
                        "content": [{"type": "text", "text": "the system prompt"}]}}}),
        ),
        user(8, json!({"kind": "cli"}), "read a.txt"),
        row(
            json!({"type": "assistant/message", "seq": 9, "time": 1009, "data": {"turn": 1, "step": 1,
            "message": {"id": "m-a", "role": "assistant", "source": {"kind": "llm", "model": "m"},
                        "content": [{"type": "text", "text": "reading"},
                                    {"type": "tool-call", "id": "call-1", "name": "read", "arguments": {"path": "a.txt"}}]}}}),
        ),
        row(
            json!({"type": "tool/call", "seq": 10, "time": 1010, "data": {"turn": 1, "step": 1,
            "callId": "call-1", "name": "read", "arguments": {"path": "a.txt"}}}),
        ),
        tool_role(11, "call-1", json!([{"type": "text", "text": "alpha"}])),
        bare("step/end", 12),
        bare("agent/inbox/spliced", 13),
        bare("compaction/prune", 14),
        row(
            json!({"type": "assistant/message", "seq": 15, "time": 1015, "data": {"turn": 1, "step": 2,
            "message": {"content": [{"type": "text", "text": "it says alpha"}]}}}),
        ),
        bare("turn/end", 16),
        bare("session/title", 17),
    ];
    assert_eq!(
        four(&rows),
        Projection {
            turns: vec![
                turn("user", "1008", Block::text("read a.txt")),
                turn("assistant", "1009", Block::text("reading")),
                turn(
                    "assistant",
                    "1010",
                    Block::tool("read {\"path\":\"a.txt\"} [call-1]")
                ),
                turn("tool", "1011", Block::tool_result("alpha [call-1]")),
                turn("assistant", "1015", Block::text("it says alpha")),
            ],
            ..Projection::default()
        }
    );
}

/// Every name of version four's quiet vocabulary the read reached, and
/// `developer/message`, which only version four has, supplies no turn and
/// counts nothing; under version three `developer/message` is a required
/// unknown.
#[test]
fn each_quiet_version_four_name_supplies_no_turn() {
    for kind in [
        "turn/start",
        "turn/end",
        "step/start",
        "step/end",
        "request/header",
        "request/context",
        "permission/preset",
        "sandbox/mode",
        "approval/policy",
        "system/message",
        "agent/inbox/spliced",
        "compaction/prune",
        "session/title",
        "subagent/catalog",
        "tool/ptc-dispatch",
        "developer/message",
    ] {
        assert_eq!(four(&[bare(kind, 0)]), Projection::default(), "{kind}");
    }
    assert_eq!(
        three(&[bare("developer/message", 0)]),
        Projection {
            unrecognized_records: 1,
            unavailable: Some(Unavailable::UnsupportedFormat),
            ..Projection::default()
        }
    );
}

/// The writer namespaces an unknown ignorable event as `plugin:<name>`
/// with its marker kept: a counted omission beside the turns it does not
/// touch. The same name without the marker is a required unknown, and so
/// is a version-zero storage row or retired dispatch name.
#[test]
fn a_namespaced_unknown_is_omitted_only_with_its_marker() {
    let marked = row(
        json!({"type": "plugin:acme/note", "seq": 1, "time": 1001, "ignorable": true, "data": {"x": 1}}),
    );
    let projection = four(&[user(0, json!({"kind": "cli"}), "q"), marked]);
    assert_eq!(
        projection.turns,
        vec![turn("user", "1000", Block::text("q"))]
    );
    assert_eq!(
        (projection.unrecognized_records, projection.unavailable),
        (1, None)
    );
    let refused = Projection {
        unrecognized_records: 1,
        unavailable: Some(Unavailable::UnsupportedFormat),
        ..Projection::default()
    };
    for kind in [
        "plugin:acme/note",
        "assistant/chunk",
        "text-chunks",
        "tool/code-dispatch",
    ] {
        assert_eq!(four(&[bare(kind, 0)]), refused, "{kind}");
    }
    assert_eq!(
        four(&[bare("assistant/attempt", 0)]),
        Projection {
            unrecognized_records: 1,
            ..Projection::default()
        }
    );
}

/// The tool-role message maps field by field: `toolCallId` names the
/// call, the direct content is the output rendered as version three
/// renders the wrapper's, an empty list leaves the id alone, and
/// `isError` and the source change nothing. Version three's wrapper row
/// and its version-four lift project the same turn.
#[test]
fn a_tool_role_result_maps_field_by_field() {
    let lifted = tool_role(
        1,
        "c1",
        json!([{"type": "text", "text": "out"}, {"type": "image"}]),
    );
    let mut failed =
        serde_json::from_str::<serde_json::Value>(&tool_role(2, "c2", json!([]))).unwrap();
    failed["data"]["message"]["isError"] = json!(true);
    let anonymous = tool_role(3, "", json!([{"type": "text", "text": "who"}]));
    assert_eq!(
        four(&[lifted.clone(), row(failed), anonymous]).turns,
        vec![
            turn(
                "tool",
                "1001",
                Block::tool_result("out [image omitted] [c1]")
            ),
            turn("tool", "1002", Block::tool_result("[c2]")),
            turn("tool", "1003", Block::tool_result("who")),
        ]
    );
    let wrapped = row(
        json!({"type": "tool/result", "seq": 1, "time": 1001, "data": {"turn": 1, "step": 1,
        "message": {"role": "user", "source": {"kind": "tool", "callId": "c1"},
                    "content": [{"type": "tool-result", "toolCallId": "c1",
                                 "content": [{"type": "text", "text": "out"}, {"type": "image"}]}]}}}),
    );
    assert_eq!(three(&[wrapped]).turns, four(&[lifted]).turns);
}

/// Version four retires the `tool-result` wrapper from every content list
/// it reads: inside a tool-role result the row is counted and shows no
/// result, inside a message the member is counted beside its kept
/// siblings. Version three reads the same rows as it always has.
#[test]
fn the_retired_wrapper_is_counted_never_a_result() {
    let wrapper = json!([{"type": "tool-result", "toolCallId": "c1", "content": "out"}]);
    let in_result = tool_role(1, "c1", wrapper);
    let in_message = row(
        json!({"type": "user/message", "seq": 2, "time": 1002, "data": {"content": [
        {"type": "text", "text": "kept"}, {"type": "tool-result", "toolCallId": "c9", "content": "x"}]}}),
    );
    let rows = [in_result, in_message];
    assert_eq!(
        four(&rows),
        Projection {
            turns: vec![turn("user", "1002", Block::text("kept"))],
            unrecognized_records: 2,
            ..Projection::default()
        }
    );
    let today = three(&rows);
    assert_eq!(
        (today.unrecognized_records, today.turns.len()),
        (0, 2),
        "{today:?}"
    );
    assert_eq!(
        [&today.turns[0].blocks, &today.turns[1].blocks],
        [
            &vec![Block::tool_result("out [c1]")],
            &vec![Block::text("kept"), Block::tool_result("x [c9]")]
        ]
    );
}

/// A producer's renamed source kind, a namespaced plugin kind and even
/// the retired plugin wrapper change no projection: the reader reads no
/// `source` under any version.
#[test]
fn producer_sources_change_no_projection() {
    let sources = [
        json!({"kind": "cli"}),
        json!({"kind": "compact-checkpoint", "compactionId": "k"}),
        json!({"kind": "runtime-context"}),
        json!({"kind": "plugin:acme"}),
        json!({"kind": "plugin", "plugin": "acme"}),
        json!("not an object"),
    ];
    for source in sources {
        assert_eq!(
            four(&[user(0, source.clone(), "same")]),
            Projection {
                turns: vec![turn("user", "1000", Block::text("same"))],
                ..Projection::default()
            },
            "{source}"
        );
    }
}

/// A log whose references the historical conversion remapped, an inserted
/// interrupted `turn/end` among them, projects as any log does: earlier
/// citations and `[start, end]` pairs are admitted, and a citation of a
/// later or the citing row's own sequence refuses the read under version
/// four as under version three.
#[test]
fn a_remapped_reference_log_projects_as_today() {
    let cite = |seq: i64, cites: serde_json::Value| {
        let mut value =
            serde_json::from_str::<serde_json::Value>(&user(seq, json!({"kind": "cli"}), "again"))
                .unwrap();
        value["sourceEventSeqs"] = cites;
        row(value)
    };
    let rows = [
        user(0, json!({"kind": "cli"}), "first"),
        row(
            json!({"type": "turn/end", "seq": 1, "time": 1001, "data": {"turn": 1, "reason": {"kind": "interrupted"}}}),
        ),
        bare("agent/inbox/spliced", 2),
        bare("turn/start", 3),
        row(
            json!({"type": "session/title", "seq": 4, "time": 1004, "data": {"title": "t", "messageSeqs": [0]}}),
        ),
        row(
            json!({"type": "compaction/prune", "seq": 5, "time": 1005, "data": {"shadowedRange": [0, 2], "shadowedSeqs": [0]}}),
        ),
        cite(6, json!([[0, 2], 4])),
    ];
    assert_eq!(
        four(&rows).turns,
        vec![
            turn("user", "1000", Block::text("first")),
            turn("user", "1006", Block::text("again")),
        ]
    );
    for refusing in [json!([6]), json!([[0, 7]])] {
        let rows = [
            user(0, json!({"kind": "cli"}), "first"),
            cite(6, refusing.clone()),
        ];
        assert_eq!(
            four(&rows).unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{refusing}"
        );
        assert_eq!(
            four(&rows).unavailable,
            three(&rows).unavailable,
            "{refusing}"
        );
    }
}
