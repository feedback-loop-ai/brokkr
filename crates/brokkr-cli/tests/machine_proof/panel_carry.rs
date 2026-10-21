use super::*;
use brokkr_core::EventType;
use brokkr_store::Store;

fn git(ws: &Workspace, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(ws.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn panel(stuck: bool) -> Workspace {
    let mut script = happy_script();
    script["seats"]["review:correctness"] =
        json!([{"behavior":"succeed", "result":{"result":"clean", "notes":"settled"}}]);
    let fresh = json!({"behavior":"succeed", "result":{"result":"clean", "notes":"retried"}});
    script["seats"]["review:security"] = if stuck {
        json!([{"behavior":"vanish"}, fresh])
    } else {
        json!([fresh])
    };
    fixture(script, POLICY)
}

fn fixture(script: Value, policy: &str) -> Workspace {
    let ws = Workspace::with_policy(script, policy);
    ws.make_panel("review", &["correctness", "security"], "review-panel");
    std::fs::write(
        ws.path().join(".gitignore"),
        "state/\n.forge/\nforge.db*\nrecipes/\n",
    )
    .unwrap();
    git(&ws, &["init", "-q"]);
    git(&ws, &["config", "user.name", "Synthetic proof"]);
    git(&ws, &["config", "user.email", "proof@example.invalid"]);
    git(&ws, &["config", "commit.gpgsign", "false"]);
    git(&ws, &["add", "."]);
    git(&ws, &["commit", "-qm", "synthetic subject"]);
    ws
}

fn retry(ws: &Workspace, run: &str) -> (Option<i32>, Value, String) {
    ws.brokkr(&[
        "operator",
        "retry",
        "--member",
        "security",
        "--reason",
        "synthetic recovery",
        "--run",
        run,
        "--bundle",
        ws.bundle_dir().to_str().unwrap(),
        "--db",
        ws.db().to_str().unwrap(),
    ])
}

fn resume(ws: &Workspace, run: &str) -> (Option<i32>, Value, String) {
    ws.brokkr(&[
        "resume",
        "--run",
        run,
        "--bundle",
        ws.bundle_dir().to_str().unwrap(),
        "--db",
        ws.db().to_str().unwrap(),
    ])
}

fn journal(ws: &Workspace, run: &str) -> Vec<brokkr_core::EventEnvelope> {
    Store::open(&ws.db()).unwrap().load(run).unwrap()
}

fn aggregate(events: &[brokkr_core::EventEnvelope]) -> Value {
    events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectSucceeded
                && event.payload["result"]["notes"]["members"]
                    .get("correctness")
                    .is_some()
        })
        .unwrap()
        .payload["result"]
        .clone()
}

fn parked(ws: &Workspace) -> String {
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(2), "{stderr}");
    Workspace::run_id(&stderr)
}

fn recover(ws: &Workspace, run: &str) {
    assert_eq!(retry(ws, run).0, Some(0));
    let (code, summary, stderr) = resume(ws, run);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn one_member_runs_and_the_aggregate_and_readouts_keep_the_settled_verdict() {
    let ws = panel(true);
    let run = parked(&ws);
    let before = journal(&ws, &run);
    let original = before
        .iter()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap()
        .payload["attempt_id"]
        .clone();
    recover(&ws, &run);
    let events = journal(&ws, &run);
    let start = events
        .iter()
        .find(|event| event.payload.get("carried").is_some())
        .unwrap();
    assert_eq!(
        start.payload["carried"],
        json!([{"member":"correctness", "attempt_id":original}])
    );
    let markers: Vec<_> = events
        .iter()
        .filter(|event| {
            event.payload["attempt_id"] == start.payload["attempt_id"]
                && event.payload["checkpoint"]["step"] == "panel-member-finished"
        })
        .map(|event| event.payload["checkpoint"]["member"].clone())
        .collect();
    assert_eq!(markers, [json!("security")]);
    let view = brokkr_view::run_view(&events, None);
    let carried = view
        .participants
        .iter()
        .find(|part| part.status == "carried")
        .unwrap();
    assert_eq!(carried.label, "review:correctness");
    assert_eq!(
        carried.carried_from.as_ref().unwrap().attempt_id,
        original.as_str().unwrap()
    );
    assert_eq!(carried.activity.text, "carried from review attempt 1");
    for verb in ["inspect", "seats"] {
        let (code, stdout, stderr) =
            ws.brokkr_raw(&[verb, "--run", &run, "--db", ws.db().to_str().unwrap()]);
        assert_eq!(code, Some(0), "{stderr}");
        assert!(stdout.contains("carried"), "{stdout}");
    }
    let full = panel(false);
    let (code, _, stderr) = full.run();
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(
        aggregate(&events),
        aggregate(&journal(&full, &Workspace::run_id(&stderr)))
    );
}

#[test]
fn a_moved_head_refuses_at_the_command_and_again_at_the_drive() {
    let ws = panel(true);
    let run = parked(&ws);
    let head = git(&ws, &["rev-parse", "HEAD"]);
    git(&ws, &["commit", "-q", "--allow-empty", "-m", "moved"]);
    let before = journal(&ws, &run).len();
    let (code, _, stderr) = retry(&ws, &run);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("reviewed head moved"), "{stderr}");
    assert_eq!(journal(&ws, &run).len(), before);
    git(&ws, &["reset", "--hard", &head]);
    assert_eq!(retry(&ws, &run).0, Some(0));
    git(
        &ws,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "moved after acceptance",
        ],
    );
    let (code, _, stderr) = resume(&ws, &run);
    assert_eq!(code, Some(2), "{stderr}");
    let events = journal(&ws, &run);
    let refusal = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap();
    assert!(refusal.payload["reason"]
        .as_str()
        .unwrap()
        .contains("reviewed head moved"));
    let start = events
        .iter()
        .find(|event| {
            event.payload["attempt_id"] == refusal.payload["attempt_id"]
                && event.event_type == EventType::EffectStarted
        })
        .unwrap();
    assert_eq!(start.payload["driver"], "none");
    assert_eq!(start.payload.get("carried"), None);
}

#[test]
fn a_retried_security_residual_returns_through_implement_verify_and_a_full_panel() {
    let mut script = happy_script();
    script["seats"]["review:correctness"] =
        json!([{"behavior":"succeed","result":{"result":"clean"}}]);
    script["seats"]["review:security"] = json!([
        {"behavior":"vanish"},
        {"behavior":"succeed","result":{"result":"residual","inputs":{"has_security_residual":true,"max_residual_severity":"low"},"notes":"repair this"}},
        {"behavior":"succeed","result":{"result":"clean"}},
    ]);
    let ws = fixture(script, REFORGING_POLICY);
    let run = parked(&ws);
    recover(&ws, &run);
    let events = journal(&ws, &run);
    let rules: Vec<_> = events
        .iter()
        .filter(|event| event.event_type == EventType::TransitionDecided)
        .filter_map(|event| event.payload["rule_id"].as_str())
        .collect();
    let repair = rules
        .iter()
        .position(|rule| *rule == "REVIEW-REFORGE")
        .unwrap();
    assert_eq!(
        &rules[repair..repair + 3],
        ["REVIEW-REFORGE", "IMPL-OK", "VERIFY-PASS"]
    );
    let starts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.event_type == EventType::EffectStarted
                && event.payload["driver"]
                    .as_str()
                    .is_some_and(|driver| driver.starts_with("panel["))
        })
        .collect();
    assert_eq!(starts.len(), 3);
    assert_eq!(starts[1].payload["carried"].as_array().unwrap().len(), 1);
    assert_eq!(starts[2].payload.get("carried"), None);
    let members: Vec<_> = events
        .iter()
        .filter(|event| {
            event.payload["attempt_id"] == starts[2].payload["attempt_id"]
                && event.payload["checkpoint"]["step"] == "panel-member-finished"
        })
        .map(|event| event.payload["checkpoint"]["member"].clone())
        .collect();
    assert_eq!(members, [json!("correctness"), json!("security")]);
}
