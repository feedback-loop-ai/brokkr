//! `brokkr library recipes usage` through the shipped binary (decision 0112 ruling
//! 4): over a journal planted under the test's own temporary directory,
//! `--json` emits exactly the rows brokkr-view derives from the same
//! journal, and the table paints them, an attempt without usage counted
//! as unmeasured.

use std::path::Path;

use brokkr_core::EventType;
use serde_json::{json, Value};

fn measures(n: u64) -> Value {
    json!({
        "peak_memory_bytes": 1_048_576 * n,
        "mean_memory_bytes": 524_288 * n,
        "peak_load_millicores": 1000 * n,
        "mean_load_millicores": 500 * n,
        "cpu_ms": 1000 * n,
        "wall_ms": 2000 * n
    })
}

/// A run of `recipe` journaled into `db`: started, in `phase`, then one
/// ending per entry of `usages`, each with that usage when it has one.
fn plant(db: &Path, run_id: &str, recipe: &str, phase: &str, usages: &[Option<Value>]) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .create_run(run_id, "feature", "test", &json!({"files": {}}))
        .unwrap();
    let mut events = vec![
        (
            EventType::RunStarted,
            json!({"feature": "feature", "manifest": {"bundle_name": recipe}}),
        ),
        (EventType::PhaseEntered, json!({"phase": phase})),
    ];
    for (index, usage) in usages.iter().enumerate() {
        let mut ending = json!({"effect_id": "e", "attempt_id": format!("a{index}"), "error": "x"});
        if let Some(usage) = usage {
            ending["usage"] = usage.clone();
        }
        events.push((EventType::EffectFailed, ending));
    }
    for (kind, payload) in events {
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    }
}

/// What `brokkr library recipes usage <args> --db <db>` printed, run in `dir`,
/// once it exited 0.
fn usage(dir: &Path, db: &Path, args: &[&str]) -> String {
    let mut command = crate::built::brokkr(["library", "recipes", "usage"]);
    command.current_dir(dir).args(args);
    let ran = command.arg("--db").arg(db).output().unwrap();
    let said = String::from_utf8(ran.stderr).unwrap();
    assert_eq!(ran.status.code(), Some(0), "{said}");
    String::from_utf8(ran.stdout).unwrap()
}

#[test]
fn recipes_usage_emits_the_view_s_rows_and_paints_them() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let panel = json!({"members": {"a": measures(1), "b": measures(3)}});
    plant(&db, "r1", "self", "implement", &[Some(measures(2)), None]);
    plant(&db, "r2", "self", "implement", &[Some(panel)]);
    plant(&db, "r3", "verify", "check", &[None]);
    let store = brokkr_store::Store::open_read_only(&db).unwrap();
    let runs: Vec<_> = ["r1", "r2", "r3"]
        .iter()
        .map(|run| store.load(run).unwrap())
        .collect();
    drop(store);

    let emitted: Value = serde_json::from_str(&usage(dir.path(), &db, &["--json"])).unwrap();
    let derived = serde_json::to_value(brokkr_view::recipe_usage(&runs, None)).unwrap();
    assert_eq!(emitted, derived);
    assert_eq!(emitted[0]["recipe"], "self");
    assert_eq!(emitted[0]["unmeasured"], 1);
    assert_eq!(emitted[0]["measured"], 3);
    assert_eq!(
        emitted[0]["peak_memory_bytes"],
        json!({"min": 1_048_576, "average": 2_097_152, "max": 3_145_728})
    );
    assert_eq!(emitted[1]["recipe"], "verify");
    assert_eq!(emitted[1]["peak_memory_bytes"], Value::Null);

    let out = usage(dir.path(), &db, &["verify"]);
    assert_eq!(
        out.lines().next(),
        Some("verify / check: 1 attempts, 1 unmeasured, 0 drivers measured")
    );
    assert_eq!(out.lines().nth(1), Some("  peak memory  unmeasured"));
}
