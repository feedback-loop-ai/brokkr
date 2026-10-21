//! Synthetic malformed entry records, shared by the readers and the gate.
use serde_json::{json, Value};

pub(crate) fn invalid_records() -> Vec<(&'static str, Value)> {
    vec![
        ("shape", json!(null)),
        ("shape", json!("verify")),
        ("shape", json!({"continues":[]})),
        ("shape", json!({"phase":"verify"})),
        ("shape", json!({"phase":"verify","continues":{}})),
        (
            "shape",
            json!({"phase":"verify","continues":[],"unknown":true}),
        ),
        (
            "shape",
            json!({"phase":"verify","continues":[{"run_id":"a-12345678"}]}),
        ),
        (
            "shape",
            json!({"phase":"verify","continues":[{"run_id":"a-12345678","feature":1}]}),
        ),
        (
            "shape",
            json!({"phase":"verify","continues":[{"run_id":"a-12345678","feature":"a","unknown":true}]}),
        ),
        ("phase", json!({"phase":"VERIFY","continues":[]})),
        ("phase", json!({"phase":"verify\n","continues":[]})),
        (
            "run_id",
            json!({"phase":"verify","continues":[{"run_id":"a-12345678\n","feature":"a"}]}),
        ),
        (
            "run_id",
            json!({"phase":"verify","continues":[{"run_id":"forged\nline","feature":"a"}]}),
        ),
        (
            "twice",
            json!({"phase":"verify","continues":[
            {"run_id":"a-12345678","feature":"a"},{"run_id":"a-12345678","feature":"b"}]}),
        ),
    ]
}
