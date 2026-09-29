use super::*;
use serde_json::{json, Value};

/// A workspace whose one adapter maps `steady` and, when asked, a
/// provisional `fresh`; and a v6 map listing `offices`, when given.
fn workspace(provisional: bool, offices: Option<Value>) -> (tempfile::TempDir, Option<World>) {
    let dir = tempfile::tempdir().unwrap();
    let mut models = json!({"steady": "steady-1"});
    if provisional {
        models["fresh"] = json!({"id": "fresh-1", "tier": "provisional"});
    }
    std::fs::create_dir_all(dir.path().join("adapters")).unwrap();
    let adapter = json!({
        "provider": "newcomer", "binary": "newcomer",
        "driver": ["{brokkr}", "driver", "newcomer", "--"], "models": models,
        "model_flag": "--model", "efforts": [], "effort_flag": "unsupported",
        "tool_permissions": "unsupported", "mcp": "unsupported",
    });
    std::fs::write(
        dir.path().join("adapters/newcomer.json"),
        adapter.to_string(),
    )
    .unwrap();
    let world = offices.map(|offices| {
        let map = json!({
            "schema": "forge.realms/v6",
            "realms": [{"name": "app", "path": ".", "default_branch": "main"}],
            "journal": "forge.db",
            "provisional_offices": offices,
        });
        std::fs::write(dir.path().join("realms.json"), map.to_string()).unwrap();
        World::load(&dir.path().join("realms.json")).unwrap()
    });
    (dir, world)
}

fn lines(dir: &Path, world: Option<&World>) -> String {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };
    report_provisional(&mut report, &dir.join("adapters"), world);
    assert!(report.healthy, "the section reports and never fails doctor");
    report.render()
}

/// Proposed decision 0075 ruling 5: doctor names each provisional model
/// with the offices the world lets it hold, and says so plainly where the
/// list names none, so an operator sees the fail-closed default.
#[test]
fn doctor_lists_each_provisional_model_and_the_offices_it_may_hold() {
    let (dir, world) = workspace(true, Some(json!(["researcher", "review-correctness"])));
    assert_eq!(
        lines(dir.path(), world.as_ref()),
        "ok       provisional fresh: adapter 'newcomer' · may hold researcher, \
         review-correctness · never a gate"
    );
    let (unlisted, world) = workspace(true, Some(json!([])));
    let nowhere = "warn     provisional fresh: adapter 'newcomer' · may hold no office: \
                   realms.json lists no provisional_offices · never a gate";
    assert_eq!(lines(unlisted.path(), world.as_ref()), nowhere);
    assert_eq!(lines(unlisted.path(), None), nowhere);
}

/// Every shipped model is promoted, so the section says there is none;
/// and an adapter tree that will not load is the agents line's to name.
#[test]
fn doctor_says_when_no_model_is_provisional_and_is_silent_without_adapters() {
    let (dir, world) = workspace(false, Some(json!(["researcher"])));
    assert_eq!(
        lines(dir.path(), world.as_ref()),
        "ok       provisional: no adapter declares a provisional model"
    );
    assert_eq!(lines(&dir.path().join("absent"), None), "");
}
