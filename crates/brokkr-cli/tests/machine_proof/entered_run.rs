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
    let mut map: Value = read_fixture_json(&map_path);
    map["realms"][0]["path"] = json!("work");
    std::fs::write(map_path, map.to_string()).unwrap();
    let bundle_path = ws.bundle_dir().join("bundle.json");
    let mut config: Value = read_fixture_json(&bundle_path);
    config["seats"]["intake"] = config["seats"]["design"].clone();
    config["seats"]["intake"]["results"] = json!(["resolved"]);
    config["seats"]["design"]["results"] = json!(["designed"]);
    config["seats"]["implement"] = config["seats"]["design"].clone();
    config["seats"]["implement"]["results"] = json!(["complete", "broken", "blocked"]);
    config["seats"]["review"] = config["seats"]["design"].clone();
    config["seats"]["review"]["results"] = json!(["clean", "residual", "security-hold"]);
    config["seats"].as_object_mut().unwrap().remove("triage");
    std::fs::write(ws.path().join("adapters/judge.json"),crate::fake_office::measured(json!({
        "provider":"judge","binary":"judge","driver":["{brokkr}","driver","judge","--"],
        "trust_tier":"trusted","egress":"contracted","models":{"judge":"judge-1"},"judges":["judge"],
        "model_flag":"--model","efforts":[],"effort_flag":"unsupported",
        "tool_permissions":"unsupported"
    })).to_string()).unwrap();
    let fake =
        crate::fake_office::fake_driver(&ws.path().join("script.json"), &ws.path().join("state"));
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
        let command = fake.as_array().unwrap();
        let mut gate = vec![
            json!(wrapper),
            json!("driver"),
            json!("judge"),
            json!("--model"),
            json!("judge-1"),
        ];
        gate.extend(command[2..].iter().cloned());
        let seat = config["seats"][phase].as_object_mut().unwrap();
        seat.remove("agent");
        seat.insert("role".into(), json!("roles/role.md"));
        seat.insert("driver".into(), json!({"command": gate}));
        seat.insert("class".into(), json!("gate"));
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
fn dispatch_entry_conflict_precedes_continuation_resolution() {
    let ws = fixture();
    let (code, _, stderr) = ws.brokkr(&[
        "run",
        "--feature",
        "synthetic conflict",
        "--bundle",
        ws.bundle_dir().to_str().unwrap(),
        "--db",
        ws.db().to_str().unwrap(),
        "--dispatch",
        "missing-synthetic-dispatch.json",
        "--enter",
        "verify",
        "--continues",
        "missing-synthetic-run",
    ]);
    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr.trim(),
        "error: --dispatch and --enter cannot be combined: a Looper-bound run takes no entry"
    );
    assert!(!ws.db().exists());
}

#[test]
fn protected_review_is_itself_an_entry() {
    let ws = fixture();
    let before = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(ws.workdir())
        .output()
        .unwrap();
    assert!(before.status.success());
    let head = String::from_utf8(before.stdout).unwrap().trim().to_string();
    let events = completed_entry(&ws, "review");
    assert_eq!(
        events[0]["payload"]["entry"],
        json!({"phase":"review","continues":[]})
    );
    let first = events
        .iter()
        .find(|event| event["type"] == "phase/entered")
        .unwrap();
    assert_eq!(first["payload"]["head"], head);
    let review = events
        .iter()
        .find(|event| event["type"] == "transition/decided" && event["payload"]["from"] == "review")
        .unwrap();
    let heads = review["payload"]["inputs"]["reviewed_heads"]
        .as_object()
        .unwrap();
    assert_eq!(heads.values().collect::<Vec<_>>(), [&json!(head)]);
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
    let mut policy: Value = read_fixture_json(&policy_path);
    for rule in policy["rules"].as_array_mut().unwrap() {
        if rule["id"] == "VERIFY-FAIL" {
            rule["next"] = json!("implement");
            rule["severity"] = json!("flagged");
        }
    }
    std::fs::write(policy_path, policy.to_string()).unwrap();
    let script_path = ws.path().join("script.json");
    let mut script: Value = read_fixture_json(&script_path);
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
    let fake =
        crate::fake_office::fake_driver(&ws.path().join("script.json"), &ws.path().join("state"));
    let mut work = vec![json!(work_driver)];
    work.extend(fake.as_array().unwrap()[2..].iter().cloned());
    crate::fake_office::hire_driver(ws.path(), ["work-office", "work"], json!(work));
    config["seats"]["implement"]["agent"] = json!("work-office");
}

fn landing_fixture() -> Workspace {
    let ws = fixture();
    let policy_path = ws.bundle_dir().join("policy.json");
    let mut policy: Value = read_fixture_json(&policy_path);
    policy["phases"] = json!([
        "classify",
        "implement",
        "verify",
        "review",
        "ship",
        "done",
        "stop"
    ]);
    policy["initial"] = json!("classify");
    let rules = policy["rules"].as_array_mut().unwrap();
    rules.retain(|rule| rule["from"] != "intake" && rule["from"] != "design");
    for (id, result, next) in [
        ("CLASSIFY-CODE", "code", "verify"),
        ("CLASSIFY-DOCS", "docs", "review"),
    ] {
        rules.push(json!({"id":id,"from":"classify","result":result,"next":next,"reason":"synthetic classification"}));
    }
    let rework = rules
        .iter_mut()
        .find(|rule| rule["id"] == "REVIEW-RESIDUAL-OK")
        .unwrap();
    rework["id"] = json!("REVIEW-RESIDUAL-REWORK");
    rework["next"] = json!("implement");
    rework["reason"] = json!("synthetic REVIEW-REFORGE road");
    std::fs::write(policy_path, policy.to_string()).unwrap();
    let bundle_path = ws.bundle_dir().join("bundle.json");
    let mut config: Value = read_fixture_json(&bundle_path);
    config["seats"]["classify"] = config["seats"]["verify"].clone();
    config["seats"]["classify"]["results"] = json!(["docs", "code"]);
    for phase in ["intake", "design"] {
        config["seats"].as_object_mut().unwrap().remove(phase);
    }
    std::fs::write(bundle_path, config.to_string()).unwrap();
    let script_path = ws.path().join("script.json");
    let mut script: Value = read_fixture_json(&script_path);
    script["seats"]["classify"] = json!([{"behavior":"succeed","result":{"result":"code"}}]);
    for report in script["seats"]["verify"].as_array_mut().unwrap() {
        report["behavior"] = json!("echo");
    }
    std::fs::write(script_path, script.to_string()).unwrap();
    ws
}

fn asked_seats(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter(|event| event["type"] == "effect/requested")
        .map(|event| event["payload"]["seat"].as_str().unwrap())
        .collect()
}

fn classification_input(events: &[Value]) -> &Value {
    events
        .iter()
        .find_map(|event| {
            let input = event.pointer("/payload/result/seat_input")?;
            (input["seat"] == "verify").then_some(&input["context"])
        })
        .unwrap()
}

#[test]
fn a_landing_starts_at_classify_and_its_entry_continues_a_run() {
    let ws = landing_fixture();
    let (code, _, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(3), "{stderr}");
    let first = Workspace::run_id(&stderr);
    let ordinary = ws.exported_events(&first);
    assert_eq!(entered_phases(&ordinary), ["classify", "verify", "stop"]);
    assert_eq!(asked_seats(&ordinary), ["classify", "verify"]);
    assert_eq!(ordinary[0]["payload"].get("entry"), None);
    let (code, _, stderr) = entered(&ws, "classify", &[&first]);
    assert_eq!(code, Some(0), "{stderr}");
    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let entry =
        json!({"phase":"classify","continues":[{"run_id":first,"feature":"proof feature"}]});
    assert_eq!(events[0]["payload"]["entry"], entry);
    assert_eq!(
        entered_phases(&events),
        ["classify", "verify", "review", "ship", "ship", "done"]
    );
    assert_eq!(
        asked_seats(&events),
        ["classify", "verify", "review", "ship", "ship"]
    );
    let phases = json!([
        "classify",
        "implement",
        "verify",
        "review",
        "ship",
        "done",
        "stop"
    ]);
    for journal in [&ordinary, &events] {
        assert_eq!(journal[0]["payload"]["phases"], phases);
        let context = classification_input(journal);
        assert_eq!(context["results"]["classify"]["result"], "code");
        assert_eq!(context.get("returned_from"), None);
    }
    assert_eq!(classification_input(&ordinary).get("entry"), None);
    assert_eq!(classification_input(&events)["entry"], entry);
}

#[test]
fn a_landing_entered_at_review_requests_neither_classify_nor_verify() {
    let ws = landing_fixture();
    let events = completed_entry(&ws, "review");
    assert_eq!(
        events[0]["payload"]["entry"],
        json!({"phase":"review","continues":[]})
    );
    assert_eq!(entered_phases(&events), ["review", "ship", "ship", "done"]);
    assert_eq!(asked_seats(&events), ["review", "ship", "ship"]);
    assert_eq!(
        events
            .iter()
            .find(|event| event["payload"]["rule_id"] == "SHIP-COMPLETE")
            .unwrap()["payload"]["next"],
        "done"
    );
}

fn read_fixture_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
