//! Decision 0094: synthetic recovery through the public CLI; no library charter.
use super::*;
use std::os::unix::fs::PermissionsExt;

fn fixture() -> Workspace {
    let mut script = happy_script();
    script["seats"]["design"] = json!([{"behavior":"succeed","result":{"result":"designed"}}]);
    script["seats"]["verify"] = json!([
        {"behavior":"succeed","result":{"result":"fail"}},
        {"behavior":"succeed","result":{"result":"pass"}}
    ]);
    script["seats"]["review"] = json!([{"behavior":"echo",
        "result":{"result":"clean","inputs":{"fixes_applied":false}}}]);
    let mut policy: Value = serde_json::from_str(POLICY).unwrap();
    policy["phases"]
        .as_array_mut()
        .unwrap()
        .insert(1, json!("design"));
    policy["rules"][0]["next"] = json!("design");
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id":"DESIGN-OK","from":"design","result":"designed","next":"implement","reason":"synthetic"
    }));
    let ws = Workspace::triage(script);
    std::fs::write(ws.bundle_dir().join("policy.json"), policy.to_string()).unwrap();
    let map_path = ws.path().join("realms.json");
    let mut map: Value =
        serde_json::from_str(&std::fs::read_to_string(&map_path).unwrap()).unwrap();
    map["realms"][0]["path"] = json!("work");
    std::fs::write(map_path, map.to_string()).unwrap();
    let bundle_path = ws.bundle_dir().join("bundle.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle_path).unwrap()).unwrap();
    config["seats"]["intake"] = config["seats"]["design"].clone();
    config["seats"]["intake"]["results"] = json!(["resolved"]);
    config["seats"]["design"]["results"] = json!(["designed"]);
    config["seats"]["implement"] = config["seats"]["design"].clone();
    config["seats"]["implement"]["results"] = json!(["complete", "broken", "blocked"]);
    config["seats"]["review"] = config["seats"]["design"].clone();
    config["seats"]["review"]["results"] = json!(["clean", "residual", "security-hold"]);
    config["seats"].as_object_mut().unwrap().remove("triage");
    std::fs::write(ws.path().join("adapters/judge.json"),json!({
        "provider":"judge","binary":"judge","driver":["{brokkr}","driver","judge","--"],
        "trust_tier":"trusted","egress":"contracted","models":{"judge":"judge-1"},"judges":["judge"],
        "model_flag":"--model","efforts":[],"effort_flag":"unsupported",
        "tool_permissions":"unsupported","mcp":"unsupported"
    }).to_string()).unwrap();
    // A synthetic trusted provider delegates the protocol to the fake driver.
    let wrapper = ws.path().join("judge");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nshift 4\nexec '{}' fake-driver \"$@\"\n",
            brokkr_bin()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    for phase in ["verify", "review", "ship"] {
        let command = config["seats"][phase]["driver"]["command"]
            .as_array()
            .unwrap();
        let mut gate = vec![
            json!(wrapper),
            json!("driver"),
            json!("judge"),
            json!("--model"),
            json!("judge-1"),
        ];
        gate.extend(command[2..].iter().cloned());
        config["seats"][phase]["driver"]["command"] = json!(gate);
        config["seats"][phase]["class"] = json!("gate");
    }
    work_driver(&ws, &mut config);
    std::fs::write(bundle_path, config.to_string()).unwrap();
    let repo = ws.workdir();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.name", "synthetic"],
        vec!["config", "user.email", "synthetic@example.invalid"],
        vec!["config", "commit.gpgsign", "false"],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(repo.join("finished.rs"), "// finished synthetic work\n").unwrap();
    assert!(Command::new("git")
        .args(["add", "."])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["commit", "-qm", "synthetic finished work"])
        .current_dir(&repo)
        .status()
        .unwrap()
        .success());
    ws
}

fn entered(ws: &Workspace, phase: &str, chain: &[&str]) -> (Option<i32>, Value, String) {
    let bundle = ws.bundle_dir();
    let db = ws.db();
    let repo = ws.workdir();
    let mut args = vec![
        "run",
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
        "--repo",
        repo.to_str().unwrap(),
        "--feature",
        "land the parked slice\nsynthetic detail",
        "--enter",
        phase,
    ];
    for run in chain {
        args.extend(["--continues", run]);
    }
    ws.brokkr(&args)
}

#[test]
fn a_finished_branch_enters_verify_and_ships_without_a_work_seat() {
    let ws = fixture();
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(3), "{stderr}");
    assert_eq!(summary["status"], "stopped");
    let first = Workspace::run_id(&stderr);
    let earlier = ws.exported_events(&first);
    assert_eq!(earlier[0]["payload"].get("entry"), None);
    assert!(!stderr.contains("entered at"));
    let (code, summary, stderr) = entered(&ws, "verify", &[&first]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(summary["status"], "completed");
    let run = Workspace::run_id(&stderr);
    let events = ws.exported_events(&run);
    let phases = entered_phases(&events);
    assert_eq!(phases, ["verify", "review", "ship", "ship", "done"]);
    let asked: Vec<_> = events
        .iter()
        .filter(|e| e["type"] == "effect/requested")
        .map(|e| e["payload"]["seat"].as_str().unwrap())
        .collect();
    assert_eq!(asked, ["verify", "review", "ship", "ship"]);
    assert_eq!(
        events[0]["payload"]["manifest"],
        earlier[0]["payload"]["manifest"]
    );
    assert_eq!(
        events[0]["payload"]["phases"],
        earlier[0]["payload"]["phases"]
    );
    assert_eq!(
        events[0]["payload"]["entry"],
        json!({"phase":"verify","continues":[
        {"run_id":first,"feature":"proof feature"}]})
    );
    assert!(events
        .iter()
        .any(|e| e["payload"]["rule_id"] == "SHIP-COMPLETE"));
    let handed = events
        .iter()
        .find_map(|event| event.pointer("/payload/result/seat_input/context/entry"))
        .unwrap();
    assert_eq!(handed, &events[0]["payload"]["entry"]);

    let db = ws.db();
    let (code, view, err) = ws.brokkr(&[
        "inspect",
        "--run",
        &run,
        "--db",
        db.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(view["entry"]["continues"][0]["title"], "proof feature");
    assert_eq!(view["entry"]["continues"][0]["run_id"], first);
    let mut lines = stderr
        .lines()
        .skip_while(|line| !line.starts_with("run started:"));
    assert_eq!(lines.next(), Some(format!("run started: {run}").as_str()));
    assert_eq!(
        lines.next(),
        Some(format!("entered at verify, continuing proof feature ({first})").as_str())
    );
}

#[test]
fn work_entry_and_duplicate_resolved_selectors_create_no_run() {
    let ws = fixture();
    let (code, _, stderr) = entered(&ws, "implement", &[]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("work site"), "{stderr}");
    assert!(!ws.db().exists());
    let (_, _, stderr) = ws.run_in_workdir();
    let first = Workspace::run_id(&stderr);
    let before = brokkr_store::Store::open_read_only(&ws.db())
        .unwrap()
        .list_runs()
        .unwrap();
    let (code, _, stderr) = entered(&ws, "verify", &["no-such-synthetic-run"]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("no-such-synthetic-run"), "{stderr}");
    let (code, _, stderr) = entered(&ws, "verify", &["latest", &first]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("named twice"), "{stderr}");
    assert_eq!(
        brokkr_store::Store::open_read_only(&ws.db())
            .unwrap()
            .list_runs()
            .unwrap(),
        before
    );
}

#[test]
fn protected_review_is_itself_an_entry() {
    let ws = fixture();
    let events = completed_entry(&ws, "review");
    assert_eq!(
        events[0]["payload"]["entry"],
        json!({"phase":"review","continues":[]})
    );
    let phases = entered_phases(&events);
    assert_eq!(phases, ["review", "ship", "ship", "done"]);
}

fn entered_phases(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter(|event| event["type"] == "phase/entered")
        .map(|event| event["payload"]["phase"].as_str().unwrap())
        .collect()
}

#[test]
fn the_first_implement_return_receives_the_verify_finding() {
    let ws = fixture();
    let policy_path = ws.bundle_dir().join("policy.json");
    let mut policy: Value =
        serde_json::from_str(&std::fs::read_to_string(&policy_path).unwrap()).unwrap();
    for rule in policy["rules"].as_array_mut().unwrap() {
        if rule["id"] == "VERIFY-FAIL" {
            rule["next"] = json!("implement");
        }
    }
    std::fs::write(policy_path, policy.to_string()).unwrap();
    let script_path = ws.path().join("script.json");
    let mut script: Value =
        serde_json::from_str(&std::fs::read_to_string(&script_path).unwrap()).unwrap();
    script["seats"]["implement"][0]["behavior"] = json!("echo");
    script["seats"]["verify"][0]["result"]["notes"] = json!("synthetic failing gate");
    std::fs::write(script_path, script.to_string()).unwrap();
    let events = completed_entry(&ws, "verify");
    assert_eq!(
        entered_phases(&events),
        [
            "verify",
            "implement",
            "verify",
            "review",
            "ship",
            "ship",
            "done"
        ]
    );
    let finding = events
        .iter()
        .find_map(|event| event.pointer("/payload/result/seat_input/context/returned_from"))
        .unwrap();
    assert_eq!(finding["phase"], "verify");
    assert_eq!(finding["result"]["result"], "fail");
    assert_eq!(finding["result"]["notes"], "synthetic failing gate");
}

fn completed_entry(ws: &Workspace, phase: &str) -> Vec<Value> {
    let (code, summary, stderr) = entered(ws, phase, &[]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(summary["status"], "completed");
    ws.exported_events(&Workspace::run_id(&stderr))
}

fn work_driver(ws: &Workspace, config: &mut Value) {
    let work_driver = ws.path().join("synthetic-work");
    std::fs::write(&work_driver,format!(
        "#!/bin/sh\ngit -c commit.gpgsign=false commit -q --allow-empty -m 'synthetic implementation' >&2\nexec '{}' fake-driver \"$@\"\n",
        brokkr_bin())).unwrap();
    std::fs::set_permissions(&work_driver, std::fs::Permissions::from_mode(0o755)).unwrap();
    let old = config["seats"]["implement"]["driver"]["command"]
        .as_array()
        .unwrap();
    let mut work = vec![json!(work_driver)];
    work.extend(old[2..].iter().cloned());
    config["seats"]["implement"]["driver"]["command"] = json!(work);
}
