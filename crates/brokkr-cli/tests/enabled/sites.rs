//! Task 46.1 (U9b1b; GP1, MB2, CR3, MB5): every site that can hold a server,
//! through real dispatch: a panel member, a sequence step, a fallback link and
//! a bounded retry, each served by a plan of its own; the shared retention
//! budget's refusal; and a gate's holdings by class and named office.

use serde_json::{json, Value};

use super::{put, World, REVIEWS};

/// The review seat every bundle here shares: an exec seat that says clean.
fn review() -> Value {
    json!({"results": ["clean"], "role": "roles/role.md", "hands": super::boxed(),
           "driver": {"command": ["{brokkr}", "driver", "exec", "--", "sh", "-c", REVIEWS,
                                  "seat", "{prompt_file}"]}})
}

impl World {
    /// The bundle whose `work` seat is a unanimous panel of `count` workers.
    pub(super) fn worker_panel(&self, count: usize) {
        let members: serde_json::Map<String, Value> = (0..count)
            .map(|at| (format!("m{at}"), json!({"agent": "worker"})))
            .collect();
        self.working(
            json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass", "panel": members}),
            &["pass", "fail"],
        );
    }

    /// The bundle whose `work` seat is `work`, its result moving on by
    /// whichever of `results` it says.
    fn working(&self, work: Value, results: &[&str]) {
        let rules: Vec<Value> = results
            .iter()
            .map(|result| {
                json!({"id": format!("WORK-{result}"), "from": "work", "result": result,
                       "next": "review", "reason": "w"})
            })
            .chain([json!({"id": "CLEAN", "from": "review", "result": "clean",
                           "next": "done", "reason": "r"})])
            .collect();
        let policy = json!({"phases": ["work", "review", "done"], "initial": "work",
                            "terminal": ["done"], "rules": rules});
        self.write("policy.json", &policy);
        self.write(
            "bundle.json",
            &json!({"name": "enabled", "policy": "policy.json",
                    "seats": {"work": work, "review": review()}}),
        );
    }

    /// The office `name`, with boxed hands and no capability.
    fn plain_office(&self, name: &str) {
        self.write(
            &format!("agents/{name}.json"),
            &json!({"description": "reads nothing", "charter": "charters/worker.md",
                    "models": ["opus"], "efforts": {"opus": "high"},
                    "hands": super::boxed()}),
        );
    }

    /// The `worker` office over `models`, retried by `limits` where given.
    fn chain(&self, models: &[&str], limits: Option<Value>) {
        let efforts: serde_json::Map<String, Value> = models
            .iter()
            .map(|model| ((*model).to_string(), json!("high")))
            .collect();
        let mut worker = json!({"description": "reads the docs", "charter": "charters/worker.md",
            "models": models, "efforts": efforts, "hands": super::boxed(),
            "capabilities": {"library-docs": "requires"}});
        if let Some(limits) = limits {
            worker["limits"] = limits;
        }
        self.write("agents/worker.json", &worker);
    }

    /// The realm's grant of `library-docs`, with `grant`'s keys beside its
    /// dialect, and the capability's `classes`.
    fn granting(&self, grant: &Value, classes: &[&str]) {
        let mut holding = json!({"dialect": "docs-mcp"});
        for (key, value) in grant.as_object().unwrap() {
            holding[key] = value.clone();
        }
        self.write(
            "capabilities/library-docs.json",
            &json!({"name": "library-docs", "classes": classes}),
        );
        self.write(
            "realms.json",
            &json!({"schema": "forge.realms/v6", "journal": "state/forge.db",
                    "realms": [{"name": "fixture", "path": "..", "default_branch": "main",
                                "capabilities": {"library-docs": holding}}]}),
        );
    }
}

/// What the seat's harness was last told its `--model` is.
fn model(world: &World) -> String {
    let argv = std::fs::read_to_string(world.bin("argv")).unwrap();
    let argv: Vec<&str> = argv.lines().collect();
    let at = argv.iter().position(|arg| *arg == "--model").unwrap();
    argv[at + 1].to_string()
}

/// MB2 through a panel: the member holding the grant is served and its call
/// retained; its sibling, of another office, holds nothing and is handed no
/// broker. Both ran, and the panel agreed.
#[test]
fn a_panel_member_is_served_beside_a_sibling_that_holds_nothing() {
    let Some(world) = World::new() else {
        return;
    };
    world.plain_office("plain");
    world.working(
        json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
               "panel": {"search": {"agent": "worker"}, "plain": {"agent": "plain"}}}),
        &["pass", "fail"],
    );
    put(&world.bin("results"), "pass\npass\n");
    let ended = world.completes();
    assert_eq!(world.starts(), 2, "both members ran");
    assert_eq!(world.broker_code(), Some(0));
    assert_eq!(
        world
            .calls(&ended.run)
            .into_iter()
            .map(|row| row.1)
            .collect::<Vec<_>>(),
        ["succeeded"]
    );
}

/// A sequence step holds its own: the step that holds the grant is served, the
/// next step, an exec seat, runs on after it.
#[test]
fn a_sequence_step_is_served_and_the_next_step_runs_on() {
    let Some(world) = World::new() else {
        return;
    };
    let next = "printf '{\"result\": \"complete\"}' > \"$(sed -n 's/^    \\(.*\\.json\\)$/\\1/p' \"$1\" | head -1)\"";
    world.working(
        json!({"results": ["complete"], "sequence": [
            {"name": "one", "results": ["complete"], "agent": "worker"},
            {"name": "two", "role": "roles/role.md", "hands": super::boxed(),
             "driver": {"command": ["{brokkr}", "driver", "exec", "--", "sh", "-c", next,
                                    "seat", "{prompt_file}"]}}]}),
        &["complete"],
    );
    let ended = world.completes();
    assert_eq!(world.broker_code(), Some(0));
    assert_eq!(world.calls(&ended.run).len(), 1);
}

/// A first link that cannot spawn falls to the next, and that attempt's own
/// plan serves it: the first adapter is the shipped one with an absent driver,
/// so it structurally fails to start (decision 0016), and the harness runs
/// once, on the fallback's own model.
#[test]
fn a_fallback_link_is_served_by_a_plan_of_its_own() {
    let Some(world) = World::new() else {
        return;
    };
    let shipped = std::fs::read(world.repo.join(".brokkr/adapters/claude.json")).unwrap();
    let mut absent: Value = serde_json::from_slice(&shipped).unwrap();
    absent["provider"] = json!("gone");
    absent["driver"][0] = json!("brokkr-absent-driver-that-is-not-installed");
    absent["models"] = json!({"gone": "gone/first"});
    absent["judges"] = json!(["gone"]);
    world.write("adapters/gone.json", &absent);
    world.chain(
        &["gone", "opus"],
        Some(json!({"max_attempts": 2, "timeout_seconds": 90})),
    );
    let ended = world.completes();
    assert_eq!(world.starts(), 1);
    assert_eq!(model(&world), "claude-opus-5-5");
    assert_eq!(world.calls(&ended.run).len(), 1);
    assert_eq!(world.broker_code(), Some(0));
}

/// A retry on the one model is a cold start of a fresh attempt, its plan minted
/// anew, and only its call is retained.
#[test]
fn a_cold_retry_is_served_afresh_and_keeps_only_its_own_call() {
    let Some(world) = World::new() else {
        return;
    };
    world.chain(
        &["opus"],
        Some(json!({"max_attempts": 2, "timeout_seconds": 90})),
    );
    put(&world.bin("fail-once"), "");
    let ended = world.completes();
    assert_eq!(world.starts(), 2);
    assert_eq!(world.calls(&ended.run).len(), 1);
    let plan = world.plan();
    let attempt = plan.parent().unwrap();
    let run = attempt.parent().unwrap();
    let minted = std::fs::read_dir(run).unwrap().flatten();
    let minted = minted.filter(|at| at.path().join("inventory.json").exists());
    assert_eq!(minted.count(), 2, "one inventory for each attempt");
}

/// CR3's budget at its door: thirty-three retaining members would each be
/// given less than the floor of the attempt's 256 MiB, so the attempt refuses
/// before anything starts. `engine::broker`'s `share` holds the exact bound.
#[test]
fn more_retaining_members_than_the_budget_shares_refuse_before_anything_starts() {
    let Some(world) = World::new() else {
        return;
    };
    world.worker_panel(33);
    let reads = world.store_reads();
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        (
            "awaiting_operator",
            Some("capability evidence budget is exhausted")
        ),
        "{ended:?}"
    );
    assert_eq!((reads.lookups(), world.starts()), (0, 0));
}

/// The gate `review`, of the office `reviewer`, in a bundle whose work seat
/// holds no capability, asking `strength` of `library-docs`.
fn gate(world: &World, strength: &str) {
    world.write(
        "agents/reviewer.json",
        &json!({"description": "reviews", "charter": "charters/worker.md",
                "models": ["opus"], "efforts": {"opus": "high"}, "hands": super::boxed(),
                "capabilities": {"library-docs": strength}}),
    );
    world.plain_office("worker");
    world.write(
        "bundle.json",
        &json!({"name": "enabled", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "worker"},
            "review": {"results": ["clean"], "agent": "reviewer", "class": "gate"}}}),
    );
    put(&world.bin("results"), "complete\nclean\n");
}

/// GP1's table through `brokkr run`, each row a world of its own: a gate's
/// reads capability holds with no offices, and its broker serves it; egress
/// holds only for the office named; writes never does, named or not, and writes
/// wins over egress. A required ask refuses with no harness started, and a
/// wanted one drops with the same cause and its gate served no broker.
#[test]
fn a_gate_holds_by_its_class_and_named_office_only() {
    let site = "seat 'review' (office 'reviewer') in realm 'fixture'";
    let through = "capability 'library-docs' through dialect 'docs-mcp'";
    let egress =
        "a gate's egress capability requires the realm grant to name office 'reviewer' explicitly";
    let writes = "gate offices cannot hold a capability with class 'writes'";
    let cases: [(&[&str], Value, Option<&str>); 5] = [
        (&["reads"], json!({}), None),
        (&["reads", "egress"], json!({"offices": ["reviewer"]}), None),
        (&["reads", "egress"], json!({}), Some(egress)),
        (
            &["reads", "writes"],
            json!({"offices": ["reviewer"]}),
            Some(writes),
        ),
        (&["reads", "writes", "egress"], json!({}), Some(writes)),
    ];
    for (classes, grant, cause) in cases {
        for strength in ["requires", "wants"] {
            let Some(world) = World::new() else {
                return;
            };
            world.granting(&grant, classes);
            gate(&world, strength);
            let args = world.run_args();
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = world.brokkr(&args).output().unwrap();
            let said = String::from_utf8_lossy(&out.stderr).into_owned();
            match (cause, strength) {
                (Some(cause), "requires") => {
                    let refusal = format!("{site}: requires {through}, but {cause}");
                    assert!(said.contains(&refusal), "{refusal}: {said}");
                    assert_eq!((out.status.success(), world.starts()), (false, 0));
                }
                (Some(cause), _) => {
                    let notice = format!("{site}: dropped wanted {through} because {cause}");
                    let ended = super::ended(out);
                    assert_eq!(ended.status, "completed", "{ended:?}");
                    assert_eq!((world.starts(), world.broker_code()), (2, None));
                    let view = world.inspected(&ended.run).to_string();
                    assert!(view.contains(&notice), "{notice}: {view}");
                }
                (None, _) => {
                    let ended = super::ended(out);
                    assert_eq!(ended.status, "completed", "{ended:?} {said}");
                    assert_eq!((world.starts(), world.broker_code()), (2, Some(0)));
                    assert_eq!(world.calls(&ended.run).len(), 1);
                }
            }
        }
    }
}
