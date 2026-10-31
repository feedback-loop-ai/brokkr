//! An office hired over an operator adapter, as both crates' fixtures
//! write one (decision 0065 slice two, U9b1): the adapter, already carrying
//! the synthetic MCP facts it compiles by past the lifted fence
//! (`synthetic_mcp`), and the office hiring its one model, named as the
//! office is.

use std::path::Path;

use serde_json::{json, Value};

/// Writes, under `root`, `adapter` as its provider's and the office
/// `office` hiring its one model, holding `limits` where given.
pub(crate) fn write(root: &Path, office: &str, adapter: &Value, limits: Option<Value>) {
    let charter = format!("charters/{office}.md");
    let mut hired = json!({
        "description": "a fixture's own driver",
        "charter": charter,
        "models": [office],
        "efforts": {office: "high"},
    });
    if let Some(limits) = limits {
        hired["limits"] = limits;
    }
    for dir in ["adapters", "agents/charters"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    let provider = adapter["provider"].as_str().expect("a provider");
    let write = |relative: String, body: &Value| {
        let bytes = serde_json::to_vec_pretty(body).unwrap();
        std::fs::write(root.join(relative), bytes).unwrap();
    };
    write(format!("adapters/{provider}.json"), adapter);
    write(format!("agents/{office}.json"), &hired);
    std::fs::write(root.join("agents").join(charter), "# fixture\n").unwrap();
}
