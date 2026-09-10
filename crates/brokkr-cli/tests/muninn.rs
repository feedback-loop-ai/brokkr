//! The standing overseer, proven end to end (decision 0020).
//!
//! Every guarantee here is a claim about what Muninn CANNOT do, so each
//! test is written as an observation of the real binary rather than as a
//! reading of the code: a fleet is staged, `brokkr muninn run` is spawned
//! against a driver this test scripts, and what the driver saw — plus
//! what the journals look like afterwards — is the evidence.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
  "phases": ["implement", "review", "done", "stop"],
  "initial": "implement",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "review",
     "reason": "Implementation complete."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "Clean review; done."}
  ]
}"#;

/// A workspace with a real fleet, an agent library holding `muninn`, and
/// a scripted `forge-driver/v1` participant standing in for the harness.
struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        for sub in [
            "bundle",
            "agents/charters",
            "adapters",
            "state",
            "seen",
            "repo",
        ] {
            std::fs::create_dir_all(ws.path().join(sub)).unwrap();
        }
        std::fs::write(ws.path().join("bundle/policy.json"), POLICY).unwrap();
        std::fs::write(ws.path().join("agents/charters/work.md"), "# work\n").unwrap();
        std::fs::write(
            ws.path().join("agents/charters/muninn.md"),
            "# read the fleet and propose\n",
        )
        .unwrap();
        // A canary the overseer must never see, in a directory that
        // looks exactly like a repository it might be tempted to read.
        std::fs::write(ws.path().join("repo/CANARY.md"), "untouched\n").unwrap();
        ws.write(
            "adapters/fake.json",
            json!({
                "provider": "fake",
                "binary": brokkr_bin(),
                "driver": [
                    brokkr_bin(), "fake-driver",
                    "--script", ws.path().join("script.json").to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy(),
                ],
                "models": {"work": "fake/work"},
                "model_flag": "--model",
                "efforts": ["low", "medium", "high"],
                "effort_flag": "--effort",
                "tool_permissions": "unsupported",
                "mcp": "unsupported",
            }),
        );
        ws.write(
            "agents/work.json",
            json!({
                "description": "does the work",
                "charter": "charters/work.md",
                "models": ["work"],
                "efforts": {"work": "medium"},
                "limits": {"max_attempts": 1, "timeout_seconds": 60},
            }),
        );
        ws.write(
            "bundle/bundle.json",
            json!({
                "name": "fleet",
                "policy": "policy.json",
                "seats": {
                    "implement": {"agent": "work", "results": ["complete"]},
                    "review": {"agent": "work", "results": ["residual", "clean"]},
                },
            }),
        );
        ws.write(
            "script.json",
            json!({"seats": {
                "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
                "review": [{"behavior": "succeed", "result": {
                    "result": "residual",
                    "inputs": {"max_residual_severity": "high"},
                    "notes": "one high residual remains",
                }}],
            }}),
        );
        ws.muninn_agent(1, 60);
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, value: Value) {
        std::fs::write(
            self.path().join(relative),
            serde_json::to_string_pretty(&value).unwrap(),
        )
        .unwrap();
    }

    fn db(&self) -> PathBuf {
        self.path().join(".forge/forge.db")
    }

    fn record(&self) -> PathBuf {
        self.path().join(".forge/muninn.ndjson")
    }

    /// The overseer's own definition, resolved against the scripted
    /// driver below rather than a real harness.
    fn muninn_agent(&self, max_attempts: u64, timeout_seconds: u64) {
        self.write(
            "agents/muninn.json",
            json!({
                "description": "reads the fleet and proposes",
                "charter": "charters/muninn.md",
                "models": ["overseer"],
                "efforts": {"overseer": "medium"},
                "limits": {
                    "max_attempts": max_attempts,
                    "timeout_seconds": timeout_seconds,
                },
            }),
        );
        self.write(
            "adapters/overseer.json",
            json!({
                "provider": "overseer",
                "binary": "sh",
                "driver": [
                    "sh",
                    self.path().join("driver.sh").to_string_lossy(),
                    self.path().join("seen").to_string_lossy(),
                ],
                "models": {"overseer": "overseer/one"},
                "model_flag": "--model",
                "efforts": ["low", "medium", "high"],
                "effort_flag": "--effort",
                "tool_permissions": "unsupported",
                "mcp": "unsupported",
            }),
        );
    }

    /// Script the driver: what it answers, and — through `extra` — any
    /// shell it runs before answering.
    fn script_driver(&self, conclusion: Value, extra: &str) {
        let capabilities = json!({
            "proto": "forge-driver/v1", "msg_id": "m1", "type": "capabilities",
            "driver": "scripted", "version": "1", "supports": [],
        });
        let mut bodies = vec![json!({
            "proto": "forge-driver/v1", "msg_id": "m2", "type": "accepted",
            "effect_id": "__EID__", "attempt_id": "a",
        })
        .to_string()];
        bodies.push(conclusion.to_string());
        std::fs::write(
            self.path().join("seen/capabilities"),
            capabilities.to_string() + "\n",
        )
        .unwrap();
        std::fs::write(self.path().join("seen/bodies"), bodies.join("\n") + "\n").unwrap();
        // `$1` is the evidence directory. The start message — which
        // carries the entire seat input — is kept verbatim, along with
        // the working directory the seat was actually given and every
        // name visible in it.
        let script = format!(
            "read -r hello\n\
             cat \"$1/capabilities\"\n\
             read -r start\n\
             printf '%s\\n' \"$start\" > \"$1/start.json\"\n\
             pwd > \"$1/cwd.txt\"\n\
             ls -a > \"$1/listing.txt\"\n\
             {extra}\n\
             id=$(printf '%s' \"$start\" | sed 's/.*\"effect_id\":\"\\([^\"]*\\)\".*/\\1/')\n\
             sed \"s/__EID__/$id/\" \"$1/bodies\"\n\
             read -r bye\n"
        );
        std::fs::write(self.path().join("driver.sh"), script).unwrap();
    }

    /// A conclusion that succeeds with the given seat result.
    fn proposes(&self, inputs: Value) {
        self.script_driver(
            json!({
                "proto": "forge-driver/v1", "msg_id": "m3", "type": "result",
                "effect_id": "__EID__", "attempt_id": "a", "status": "succeeded",
                "result": {"result": "proposed", "inputs": inputs,
                           "notes": "read the fleet"},
            }),
            "true",
        );
    }

    fn brokkr(&self, args: &[&str]) -> Output {
        Command::new(brokkr_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap()
    }

    /// Drive one run to its conclusion and return its run id.
    fn run_once(&self, feature: &str) -> String {
        self.run_once_in(feature, ".forge/forge.db")
    }

    /// The same, into a named hearth — how a many-hearth world is staged.
    fn run_once_in(&self, feature: &str, db: &str) -> String {
        self.run_once_in_repo(feature, db, "repo")
    }

    /// The same again, in a named REPOSITORY: what a world of two real
    /// trees needs, where every staging above shares one.
    fn run_once_in_repo(&self, feature: &str, db: &str, repo: &str) -> String {
        let output = self.brokkr(&[
            "run",
            "--bundle",
            "bundle",
            "--feature",
            feature,
            "--db",
            db,
            "--repo",
            repo,
        ]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        stderr
            .lines()
            .find_map(|line| line.strip_prefix("run started: "))
            .unwrap_or_else(|| panic!("a run id on stderr: {stderr}"))
            .to_string()
    }

    fn muninn(&self) -> Output {
        self.brokkr(&[
            "muninn",
            "run",
            "--db",
            ".forge/forge.db",
            "--agents-dir",
            "agents",
            "--adapters-dir",
            "adapters",
            "--record",
            ".forge/muninn.ndjson",
        ])
    }

    /// The same command with no `--db`: the world's own map decides which
    /// hearths are read (decision 0026 ruling 3).
    fn muninn_over_world(&self) -> Output {
        self.brokkr(&[
            "muninn",
            "run",
            "--agents-dir",
            "agents",
            "--adapters-dir",
            "adapters",
            "--record",
            ".forge/muninn.ndjson",
        ])
    }

    fn seen(&self, name: &str) -> String {
        std::fs::read_to_string(self.path().join("seen").join(name)).unwrap()
    }

    fn start_input(&self) -> Value {
        let start: Value = serde_json::from_str(self.seen("start.json").trim()).unwrap();
        start["input"].clone()
    }

    fn records(&self) -> Vec<Value> {
        match std::fs::read_to_string(self.record()) {
            Err(_) => Vec::new(),
            Ok(raw) => raw
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| serde_json::from_str(line).unwrap())
                .collect(),
        }
    }

    /// Every run's journal, as (run id, event count, head hash).
    fn journals(&self) -> Vec<(String, usize, String)> {
        let store = brokkr_store::Store::open_read_only(&self.db()).unwrap();
        store
            .list_runs()
            .unwrap()
            .into_iter()
            .map(|(run_id, _, _)| {
                let events = store.load(&run_id).unwrap();
                let head = store.head_hash(&run_id).unwrap();
                (run_id, events.len(), head.1)
            })
            .collect()
    }
}

/// A fleet with one run carrying a residual finding: the exact shape the
/// overseer exists to summarize. The third value is the sequence number
/// of the review ruling that finding was read from — the citation a
/// proposal about it must name.
fn staged() -> (Workspace, String, u64) {
    let ws = Workspace::new();
    let run_id = ws.run_once("prove the overseer reads it");
    let store = brokkr_store::Store::open_read_only(&ws.db()).unwrap();
    let events = store.load(&run_id).unwrap();
    let state = brokkr_core::fold(&events).unwrap();
    assert_eq!(
        brokkr_view::status_str(&state.status),
        "stopped",
        "the staged run reaches a hard stop on its residual"
    );
    let findings = brokkr_view::residual_findings(&run_id, &events);
    assert_eq!(findings.len(), 1, "one high residual, from one ruling");
    let seq = findings[0].seq;
    drop(store);
    (ws, run_id, seq)
}

#[test]
fn a_proposal_is_recorded_with_its_citations_and_read_back_by_list() {
    let (ws, run_id, seq) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run, stopped on a high residual",
        "parked_runs": [],
        "work_queue": [{
            "run_id": run_id,
            "seq": seq,
            "finding": "max_residual_severity: high",
            "reasoning": "the only residual in the fleet",
        }],
    }));
    let output = ws.muninn();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let records = ws.records();
    assert_eq!(records.len(), 1, "exactly one entry per invocation");
    let entry = &records[0];
    assert_eq!(entry["record_version"], 1);
    assert_eq!(entry["agent"]["name"], "muninn");
    assert_eq!(entry["agent"]["deadline_seconds"], 60);
    assert!(entry["recorded_at"].as_str().unwrap().starts_with("20"));
    assert_eq!(
        entry["citations"],
        json!([{"run_id": run_id, "seq": seq}]),
        "every proposal names the journal fact behind it"
    );
    assert_eq!(
        entry["work_queue"][0]["finding"],
        "max_residual_severity: high"
    );
    assert_eq!(entry["dossier"]["fleet"]["runs"], 1);
    assert_eq!(
        entry["usage"],
        Value::Null,
        "a driver that reports no cost is recorded as reporting none"
    );

    // A second invocation appends; it never rewrites the first.
    let output = ws.muninn();
    assert!(output.status.success());
    let records = ws.records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], *entry, "the first entry is byte-identical");

    let listed = ws.brokkr(&["muninn", "list", "--record", ".forge/muninn.ndjson"]);
    let text = String::from_utf8_lossy(&listed.stdout);
    assert!(listed.status.success());
    assert_eq!(
        text.matches("cites:").count(),
        2,
        "both entries read back, citations included: {text}"
    );
    assert!(
        text.contains(&format!("queue {run_id} seq {seq}")),
        "{text}"
    );
    assert!(
        text.contains("one run, stopped on a high residual"),
        "{text}"
    );

    let json = ws.brokkr(&[
        "muninn",
        "list",
        "--record",
        ".forge/muninn.ndjson",
        "--json",
    ]);
    let parsed: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

/// Decision 0026 ruling 3, end to end: with a many-hearth map and no
/// `--db`, the overseer reads EVERY journal the map names, the dossier
/// states which realm each run came from, and the proposal it records —
/// and prints — cites the realm behind every fact.
#[test]
fn the_overseer_reads_every_hearth_the_map_names_and_cites_each_realm() {
    let (ws, alpha_run, seq) = staged();
    let beta_run = ws.run_once_in("beta's own work", ".forge/beta.db");
    // The map is written AFTER both runs, so nothing about the runs
    // themselves changed — only how the fleet is read afterwards.
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main"},
                {"name": "beta", "path": "repo", "default_branch": "main",
                 "journal": ".forge/beta.db"},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    ws.proposes(json!({
        "fleet_summary": "two hearths, a stopped run in each",
        "parked_runs": [],
        "work_queue": [{
            "realm": "alpha",
            "run_id": alpha_run,
            "seq": seq,
            "finding": "max_residual_severity: high",
            "reasoning": "the highest residual in the world",
        }],
    }));

    let bytes_before = std::fs::read(ws.db()).unwrap();
    let output = ws.muninn_over_world();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    // The dossier the seat received names both hearths, and every run in
    // it says which one it was read from.
    let context = ws.start_input()["context"].clone();
    assert_eq!(context["fleet"]["realms"], json!(["alpha", "beta"]));
    assert_eq!(
        context["fleet"]["runs"], 2,
        "the whole world, not one hearth"
    );
    let runs: Vec<(&str, &str)> = context["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["realm"].as_str().unwrap(),
                row["run_id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        runs,
        vec![("alpha", alpha_run.as_str()), ("beta", beta_run.as_str())]
    );
    // Findings are cited per realm too.
    let findings = context["residual_findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|finding| finding["realm"] == json!("beta")),
        "{findings:?}"
    );

    // The record, and the line an operator reads, both name the realm.
    let entry = &ws.records()[0];
    assert_eq!(entry["work_queue"][0]["realm"], json!("alpha"));
    assert_eq!(
        entry["citations"],
        json!([{"realm": "alpha", "run_id": alpha_run, "seq": seq}])
    );
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        printed.contains(&format!("queue alpha/{alpha_run}")),
        "{printed}"
    );

    // Ruling 5: reading two journals wrote to neither.
    assert_eq!(bytes_before, std::fs::read(ws.db()).unwrap());
}

// ---------------------- crossings (decision 0057, slice vi)

/// The bytes the crossing world publishes while its pin is true.
const CROSSING_BYTES: &str = "{\"title\": \"orders\"}\n";

/// A map that draws a crossing across two realms of the staged world:
/// `alpha` publishes `orders.api` in the shared tree, `beta` pins its
/// bytes. Written against the already-staged journal, so the run half of
/// the dossier is unchanged.
fn crossing_world(ws: &Workspace) -> (PathBuf, String) {
    let contract = ws.path().join("repo/orders.v1.schema.json");
    std::fs::write(&contract, CROSSING_BYTES).unwrap();
    let pin = brokkr_core::canonical::sha256_bytes(CROSSING_BYTES.as_bytes());
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main",
                 "publishes": [{"name": "orders.api",
                                "path": "orders.v1.schema.json"}]},
                {"name": "beta", "path": "repo", "default_branch": "main",
                 "consumes": [{"name": "orders.api", "realm": "alpha", "sha256": pin}]},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    (contract, pin)
}

/// Phase 2 slice (vi): the raven carries the world's crossings into the
/// dossier. A matching pin is stated per realm and is not a finding; a
/// pin that moved becomes a finding under the CONSUMING realm — the one
/// whose run would refuse — and a proposal may cite it. Reading a world
/// writes to none of its journals.
#[test]
fn the_overseer_reads_crossings_and_a_moved_pin_is_the_consumers_finding() {
    let (ws, _, _) = staged();
    let (contract, pin) = crossing_world(&ws);

    // Sound: the crossing is stated per realm, and no finding is raised.
    ws.proposes(json!({
        "fleet_summary": "one run and one matching contract",
        "parked_runs": [],
        "work_queue": [],
    }));
    let before = std::fs::read(ws.db()).unwrap();
    let sound = ws.muninn_over_world();
    assert!(
        sound.status.success(),
        "{}",
        String::from_utf8_lossy(&sound.stderr)
    );
    let context = ws.start_input()["context"].clone();
    let stated = context["crossings"].as_array().unwrap();
    assert_eq!(stated[0]["realm"], "alpha");
    assert_eq!(
        stated[0]["publishes"],
        json!([{"name": "orders.api", "path": "orders.v1.schema.json"}]),
        "a publication keeps its declared repository-relative path"
    );
    assert_eq!(stated[0]["consumes"], json!([]));
    assert_eq!(stated[1]["realm"], "beta");
    assert_eq!(stated[1]["publishes"], json!([]));
    assert_eq!(
        stated[1]["consumes"],
        json!([{"name": "orders.api", "realm": "alpha", "pin": "matching",
                "detail": null}]),
        "a matching pin is stated, not a bare count"
    );
    assert!(
        context["residual_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding.get("crossing").is_none()),
        "no crossing finding while the pin matches"
    );

    // Moved: the same readout, now with the finding charged to beta.
    std::fs::write(&contract, "{\"title\": \"Orders\"}\n").unwrap();
    let observed = brokkr_core::canonical::sha256_bytes(&std::fs::read(&contract).unwrap());
    assert_ne!(observed, pin);
    ws.proposes(json!({
        "fleet_summary": "one contract moved under beta",
        "parked_runs": [],
        "work_queue": [{
            "realm": "beta",
            "crossing": "orders.api",
            "finding": "the contract beta consumed moved",
            "reasoning": "beta's next run would refuse at load",
        }],
    }));
    let moved = ws.muninn_over_world();
    assert!(
        moved.status.success(),
        "{}",
        String::from_utf8_lossy(&moved.stderr)
    );
    let context = ws.start_input()["context"].clone();
    let beta = &context["crossings"][1];
    assert_eq!(beta["consumes"][0]["pin"], "moved");
    assert_eq!(beta["consumes"][0]["realm"], "alpha");
    let detail = beta["consumes"][0]["detail"].as_str().unwrap();
    assert!(detail.contains(&observed), "{detail}");
    assert!(
        detail.contains("orders.v1.schema.json"),
        "the refusal names the publisher's own path: {detail}"
    );
    assert!(
        !detail.contains(&format!("{}", ws.path().display())),
        "the seat never learns the operator's host path: {detail}"
    );
    let finding = context["residual_findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["crossing"] == json!("orders.api"))
        .expect("the moved crossing is a finding");
    assert_eq!(finding["realm"], "beta", "charged to the consumer");
    assert_eq!(finding["publisher"], "alpha");
    assert_eq!(finding["input"], "crossing_pins");
    assert_eq!(finding["value"], "moved");

    // The record follows the citation back, and the operator reads it.
    let entry = ws.records().last().unwrap().clone();
    assert_eq!(
        entry["crossing_citations"],
        json!([{"realm": "beta", "crossing": "orders.api"}])
    );
    assert_eq!(entry["citations"], json!([]));
    let printed = String::from_utf8_lossy(&moved.stdout);
    assert!(printed.contains("queue beta/orders.api"), "{printed}");
    assert!(printed.contains("cites: beta/orders.api"), "{printed}");

    // Ruling 5: the flight wrote to no journal it read.
    assert_eq!(before, std::fs::read(ws.db()).unwrap());
}

/// A proposal may not invent a contract problem: a crossing the dossier
/// does not state as a finding is refused and nothing is recorded.
#[test]
fn an_invented_crossing_citation_is_refused_and_not_recorded() {
    let (ws, _, _) = staged();
    crossing_world(&ws);
    ws.proposes(json!({
        "fleet_summary": "an invented contract",
        "parked_runs": [],
        "work_queue": [{
            "realm": "beta",
            "crossing": "ghost.api",
            "finding": "invented",
            "reasoning": "invented",
        }],
    }));
    let output = ws.muninn_over_world();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not state as a finding"),
        "the refusal names what is wrong: {stderr}"
    );
    assert!(ws.records().is_empty());
}

// ---------------------- two DISTINCT repositories (phase 2 slice (i))

/// A repository with one commit of its own: the file it adds, that
/// file's content and the commit message are all `name`, so two
/// repositories built in the same second are still two trees and two
/// shas rather than one commit made twice.
fn git_repo(root: &Path, name: &str) -> String {
    let repo = root.join(name);
    std::fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Brokkr Test"]);
    git(&["config", "user.email", "brokkr@test"]);
    git(&["config", "commit.gpgSign", "false"]);
    std::fs::write(repo.join(format!("{name}.txt")), name).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", name]);
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// [`staged`], parametrized by repository and hearth: one run, stopped
/// on its high residual, with the sequence of the ruling that finding
/// was read from.
fn staged_in(ws: &Workspace, feature: &str, db: &str, repo: &str) -> (String, u64) {
    let run_id = ws.run_once_in_repo(feature, db, repo);
    let store = brokkr_store::Store::open_read_only(&ws.path().join(db)).unwrap();
    let events = store.load(&run_id).unwrap();
    let state = brokkr_core::fold(&events).unwrap();
    assert_eq!(
        brokkr_view::status_str(&state.status),
        "stopped",
        "the staged run reaches a hard stop on its residual"
    );
    let findings = brokkr_view::residual_findings(&run_id, &events);
    assert_eq!(findings.len(), 1, "one high residual, from one ruling");
    (run_id, findings[0].seq)
}

/// The heads a run recorded, read back out of the journal it wrote them
/// to — the fleet readouts carry no head of their own, so this is where
/// "which tree was this?" is answerable.
fn recorded_heads(ws: &Workspace, db: &str, run_id: &str) -> Value {
    let store = brokkr_store::Store::open_read_only(&ws.path().join(db)).unwrap();
    store
        .load(run_id)
        .unwrap()
        .iter()
        .find_map(|event| {
            event
                .payload
                .get("inputs")
                .and_then(|inputs| inputs.get("reviewed_heads"))
                .cloned()
        })
        .unwrap_or_else(|| panic!("run {run_id} recorded no heads in {db}"))
}

/// Phase 2 slice (i), proof 5, the muninn half: the overseer reads two
/// REAL repositories, each under its own realm, and splits every fact
/// between them — never merged (decision 0026 rulings 3 and 5).
///
/// `the_overseer_reads_every_hearth_the_map_names_and_cites_each_realm`
/// above already proves the per-realm citation — but both its realms are
/// `path: "repo"`, one tree, so nothing in it could tell a world of two
/// repositories from one repository read twice under two names. What is
/// new here is that `alpha` and `beta` are two distinct git trees at two
/// distinct heads, and the proof of it is that each realm's run recorded
/// its OWN tree's head under its OWN realm's name. Reading one tree for
/// both, or folding the hearths together, would put one sha in both
/// journals.
#[test]
fn the_overseer_reads_two_real_repositories_each_under_its_own_realm() {
    let ws = Workspace::new();
    let alpha_head = git_repo(ws.path(), "alpha");
    let beta_head = git_repo(ws.path(), "beta");
    assert_ne!(
        alpha_head, beta_head,
        "two repositories, two commits, two shas"
    );
    // The map is written BEFORE the runs here, unlike the one-tree test
    // above: a run only keys its heads by realm when a map named the
    // tree it stands in, and that keying is the evidence this proof
    // turns on.
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "alpha", "path": "alpha", "default_branch": "main"},
                {"name": "beta", "path": "beta", "default_branch": "trunk",
                 "journal": ".forge/beta.db"},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    let (alpha_run, seq) = staged_in(&ws, "alpha's own work", ".forge/forge.db", "alpha");
    let (beta_run, beta_seq) = staged_in(&ws, "beta's own work", ".forge/beta.db", "beta");

    ws.proposes(json!({
        "fleet_summary": "two repositories, a stopped run in each",
        "parked_runs": [],
        "work_queue": [{
            "realm": "alpha",
            "run_id": alpha_run,
            "seq": seq,
            "finding": "max_residual_severity: high",
            "reasoning": "alpha's own residual, in alpha's own tree",
        }],
    }));

    let alpha_bytes = std::fs::read(ws.db()).unwrap();
    let beta_bytes = std::fs::read(ws.path().join(".forge/beta.db")).unwrap();
    let output = ws.muninn_over_world();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    // The dossier names both realms and keys every run to the one it was
    // read from — side by side, in map order, never a merged list.
    let context = ws.start_input()["context"].clone();
    assert_eq!(context["fleet"]["realms"], json!(["alpha", "beta"]));
    assert_eq!(context["fleet"]["runs"], 2);
    let runs: Vec<(&str, &str)> = context["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["realm"].as_str().unwrap(),
                row["run_id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        runs,
        vec![("alpha", alpha_run.as_str()), ("beta", beta_run.as_str())]
    );
    // Each repository's finding is cited under its own realm, and under
    // its own run — the pairing a merge would scramble.
    let findings: Vec<(&str, &str, u64)> = context["residual_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["realm"].as_str().unwrap(),
                finding["run_id"].as_str().unwrap(),
                finding["seq"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        findings,
        vec![
            ("alpha", alpha_run.as_str(), seq),
            ("beta", beta_run.as_str(), beta_seq),
        ]
    );

    // The record and the printed line name the realm the fact came from.
    let entry = &ws.records()[0];
    assert_eq!(entry["work_queue"][0]["realm"], json!("alpha"));
    assert_eq!(
        entry["citations"],
        json!([{"realm": "alpha", "run_id": alpha_run, "seq": seq}])
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!("queue alpha/{alpha_run}")),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );

    // And the two realms are two trees: each run recorded its OWN
    // repository's head, keyed by its own realm name.
    let alpha_heads = recorded_heads(&ws, ".forge/forge.db", &alpha_run);
    let beta_heads = recorded_heads(&ws, ".forge/beta.db", &beta_run);
    assert_eq!(alpha_heads, json!({ "alpha": alpha_head }));
    assert_eq!(beta_heads, json!({ "beta": beta_head }));
    assert_ne!(alpha_heads, beta_heads);

    // Ruling 5: reading two hearths wrote to neither.
    assert_eq!(alpha_bytes, std::fs::read(ws.db()).unwrap());
    assert_eq!(
        beta_bytes,
        std::fs::read(ws.path().join(".forge/beta.db")).unwrap()
    );
}

/// A realm the map names before its first run has no journal yet, and
/// the world's dossier is not withheld for it: the flight reads the
/// hearths that are there, says out loud which one stated nothing, and
/// creates no journal on the way past — the same degrading `brokkr runs`
/// already does per hearth (decision 0026 rulings 3 and 5). A ONE-hearth
/// world keeps its refusal: the single journal it was pointed at must
/// open or there is nothing to report on.
#[test]
fn a_realm_with_no_journal_yet_does_not_withhold_the_worlds_dossier() {
    let (ws, alpha_run, seq) = staged();
    let unborn = ws.path().join(".forge/beta.db");
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main"},
                {"name": "beta", "path": "repo", "default_branch": "main",
                 "journal": ".forge/beta.db"},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    ws.proposes(json!({
        "fleet_summary": "one hearth answered, one has not run yet",
        "parked_runs": [],
        "work_queue": [{
            "realm": "alpha",
            "run_id": alpha_run,
            "seq": seq,
            "finding": "max_residual_severity: high",
            "reasoning": "the only hearth that has run",
        }],
    }));

    let output = ws.muninn_over_world();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("realm beta states nothing"),
        "the absence is said, not swallowed: {stderr}"
    );
    let context = ws.start_input()["context"].clone();
    assert_eq!(context["fleet"]["realms"], json!(["alpha"]));
    assert_eq!(context["fleet"]["runs"], 1);
    assert_eq!(context["runs"][0]["realm"], json!("alpha"));
    assert!(!unborn.exists(), "a read created a journal");
    assert!(!ws.path().join(".forge/beta.db-wal").exists());

    // The one-hearth world's refusal is unchanged: no dossier, nothing
    // recorded, nonzero.
    let output = ws.brokkr(&[
        "muninn",
        "run",
        "--db",
        ".forge/nowhere.db",
        "--agents-dir",
        "agents",
        "--adapters-dir",
        "adapters",
        "--record",
        ".forge/muninn.ndjson",
    ]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("nowhere.db"),
        "the refusal names the journal it was pointed at"
    );
    assert!(!ws.path().join(".forge/nowhere.db").exists());
}

/// A fresh world that has never run still has a dossier when its map
/// draws a crossing: the crossing report comes off the map, so the raven
/// carries it before the first journal exists. The absent journals are
/// still said out loud, and a map with neither a readable journal nor a
/// crossing still has nothing to report.
#[test]
fn a_world_with_crossings_reports_them_before_any_journal_exists() {
    let ws = Workspace::new();
    let contract = ws.path().join("repo/orders.v1.schema.json");
    std::fs::write(&contract, CROSSING_BYTES).unwrap();
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main",
                 "journal": ".forge/alpha.db",
                 "publishes": [{"name": "orders.api",
                                "path": "orders.v1.schema.json"}]},
                {"name": "beta", "path": "repo", "default_branch": "main",
                 "journal": ".forge/beta.db",
                 "consumes": [{"name": "orders.api", "realm": "alpha",
                               "sha256": "a".repeat(64)}]},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    ws.proposes(json!({
        "fleet_summary": "no run yet, one contract moved",
        "parked_runs": [],
        "work_queue": [{"realm": "beta", "crossing": "orders.api",
                        "finding": "the contract moved", "reasoning": "beta would refuse"}],
    }));

    let output = ws.muninn_over_world();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("realm alpha states nothing")
            && stderr.contains("realm beta states nothing"),
        "each absent journal is said out loud: {stderr}"
    );
    let context = ws.start_input()["context"].clone();
    assert_eq!(context["fleet"]["runs"], 0, "no run exists yet");
    assert!(context["runs"].as_array().unwrap().is_empty());
    let beta = &context["crossings"][1];
    assert_eq!(beta["realm"], "beta");
    assert_eq!(beta["consumes"][0]["pin"], "moved");
    let finding = context["residual_findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["crossing"] == json!("orders.api"))
        .expect("the moved crossing is a finding with no run at all");
    assert_eq!(finding["realm"], "beta");
    assert_eq!(ws.records().len(), 1, "the proposal was recorded");
    assert!(!ws.db().exists(), "a read created a journal");
    assert!(!ws.path().join(".forge/alpha.db").exists());
    assert!(!ws.path().join(".forge/beta.db").exists());

    // No crossing to carry and no readable journal: still nothing to
    // report on, and the refusal says so.
    let bare = Workspace::new();
    bare.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main",
                 "journal": ".forge/alpha.db"},
                {"name": "beta", "path": "repo", "default_branch": "main",
                 "journal": ".forge/beta.db"},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    let refused = bare.muninn_over_world();
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("nothing to report on"),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
}

/// A ONE-hearth world whose map draws a crossing still yields a dossier
/// when its single journal is not there yet: the crossing report is a
/// dossier in its own right, and the absent journal is said out loud. A
/// one-hearth world with NO crossing keeps its unchanged refusal, which
/// the test above proves.
#[test]
fn a_sole_realm_with_a_crossing_reports_it_before_its_journal_exists() {
    let ws = Workspace::new();
    let contract = ws.path().join("repo/orders.v1.schema.json");
    std::fs::write(&contract, CROSSING_BYTES).unwrap();
    ws.write(
        "realms.json",
        json!({
            "schema": "forge.realms/v5",
            "realms": [
                {"name": "alpha", "path": "repo", "default_branch": "main",
                 "publishes": [{"name": "orders.api",
                                "path": "orders.v1.schema.json"}]},
            ],
            "journal": ".forge/forge.db",
        }),
    );
    ws.proposes(json!({
        "fleet_summary": "one publication, no run yet",
        "parked_runs": [],
        "work_queue": [],
    }));
    let output = ws.muninn_over_world();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("realm alpha states nothing"), "{stderr}");
    let context = ws.start_input()["context"].clone();
    assert_eq!(context["crossings"][0]["realm"], "alpha");
    assert_eq!(
        context["crossings"][0]["publishes"][0]["name"],
        "orders.api"
    );
    assert!(!ws.db().exists(), "a read created a journal");
    assert_eq!(ws.records().len(), 1);
}

#[test]
fn no_run_journal_gains_an_event_and_the_store_is_opened_read_only() {
    let (ws, _, _) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run",
        "parked_runs": [],
        "work_queue": [],
    }));
    let before = ws.journals();
    let bytes_before = std::fs::read(ws.db()).unwrap();
    assert!(ws.muninn().status.success());
    assert_eq!(before, ws.journals(), "no journal moved");
    assert_eq!(
        bytes_before,
        std::fs::read(ws.db()).unwrap(),
        "the database file is byte-identical after the invocation"
    );

    // The structural half: the connection this path opens refuses a
    // write at SQLite, so an append is impossible rather than merely
    // absent.
    let mut reader = brokkr_store::Store::open_read_only(&ws.db()).unwrap();
    assert!(reader
        .append_next(
            &before[0].0,
            brokkr_core::EventType::PhaseEntered,
            json!({"phase": "review"}),
            None,
            None,
        )
        .is_err());
}

#[test]
fn the_seat_is_given_a_scratch_directory_and_never_a_repository() {
    let (ws, _, _) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run",
        "parked_runs": [],
        "work_queue": [],
    }));
    assert!(ws.muninn().status.success());

    let input = ws.start_input();
    let workdir = input["workdir"].as_str().unwrap();
    assert_ne!(workdir, ws.path().to_string_lossy());
    assert!(
        !Path::new(workdir).starts_with(ws.path()),
        "the seat's directory is not inside the workspace: {workdir}"
    );
    assert!(
        !Path::new(workdir).exists(),
        "and it is gone once the invocation ends"
    );
    let cwd = ws.seen("cwd.txt");
    // The driver reports its cwd through its own shell: macOS prints
    // the /private canonicalization and Windows' sh prints an MSYS
    // spelling, so the unique scratch component is what both agree on.
    assert_eq!(
        Path::new(cwd.trim()).file_name(),
        Path::new(workdir).file_name(),
        "the driver ran there too: {cwd} vs {workdir}"
    );
    let listing = ws.seen("listing.txt");
    assert!(
        !listing.contains("CANARY"),
        "the repository is not in reach: {listing}"
    );
    assert_eq!(
        std::fs::read_to_string(ws.path().join("repo/CANARY.md")).unwrap(),
        "untouched\n"
    );
    let raw = ws.seen("start.json");
    let repo = ws.path().join("repo").to_string_lossy().into_owned();
    assert!(
        !raw.contains("CANARY") && !raw.contains(&repo),
        "no repository path is named in the seat's input: {raw}"
    );
}

#[test]
fn no_secret_is_resolved_for_the_seat() {
    let (ws, _, _) = staged();
    std::fs::write(
        ws.path().join(".forge/secrets.env"),
        "API_TOKEN=super-secret-value\n",
    )
    .unwrap();
    ws.proposes(json!({
        "fleet_summary": "one run",
        "parked_runs": [],
        "work_queue": [],
    }));
    let output = ws.muninn();
    assert!(output.status.success());

    let input = ws.start_input();
    assert!(
        input.get("secrets").is_none() && input.get("secrets_file").is_none(),
        "the input declares no binding and names no store: {input}"
    );
    for evidence in [
        ws.seen("start.json"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        std::fs::read_to_string(ws.record()).unwrap(),
    ] {
        assert!(
            !evidence.contains("super-secret-value"),
            "no bound value reaches anything the overseer touches: {evidence}"
        );
    }
}

#[test]
fn a_refused_invocation_records_nothing_and_exits_nonzero() {
    let (ws, _, _) = staged();

    // A driver that fails outright.
    ws.script_driver(
        json!({
            "proto": "forge-driver/v1", "msg_id": "m3", "type": "result",
            "effect_id": "__EID__", "attempt_id": "a", "status": "failed",
            "error": "the harness refused",
        }),
        "true",
    );
    let output = ws.muninn();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("muninn produced no report and recorded nothing"),
        "{stderr}"
    );
    assert!(stderr.contains("the harness refused"), "{stderr}");
    assert!(ws.records().is_empty());

    // A driver that runs past its deadline: it stalls on a message
    // that never comes, and the deadline kill takes it down. Every
    // platform now, the recorded gap closed: PR #81 left this unix-only
    // because Windows had never exercised a kill against a genuinely
    // blocked driver, and `Child::kill()` reaches only the process the
    // harness holds a handle to. The deadline kill takes the driver's
    // whole process tree on Windows, so the pipes close and the harness
    // sees the EOF it was waiting on.
    ws.muninn_agent(1, 1);
    ws.script_driver(
        json!({
            "proto": "forge-driver/v1", "msg_id": "m3", "type": "result",
            "effect_id": "__EID__", "attempt_id": "a", "status": "succeeded",
            "result": {"result": "proposed", "inputs": {
                "fleet_summary": "too late", "parked_runs": [], "work_queue": []}},
        }),
        "read -r stall",
    );
    let output = ws.muninn();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("deadline"), "{stderr}");
    assert!(
        ws.records().is_empty(),
        "an out-of-deadline invocation records nothing"
    );
    assert!(
        !std::fs::exists(ws.record()).unwrap(),
        "the record file is not even created"
    );
}

#[test]
fn a_report_that_does_not_validate_is_not_recorded() {
    let (ws, _, _) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run",
        "parked_runs": [],
        "work_queue": [{
            "run_id": "a-run-that-never-existed", "seq": 3,
            "finding": "invented", "reasoning": "invented",
        }],
    }));
    let output = ws.muninn();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("was not usable and was not recorded"),
        "{stderr}"
    );
    assert!(
        stderr.contains("which the dossier does not state"),
        "{stderr}"
    );
    assert!(ws.records().is_empty());
}

#[test]
fn muninn_issues_no_operator_command_and_starts_no_run() {
    let (ws, run_id, seq) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run",
        "parked_runs": [],
        "work_queue": [{
            "run_id": run_id, "seq": seq, "finding": "high residual",
            "reasoning": "worth fixing first",
        }],
    }));
    let before = ws.journals();
    assert!(ws.muninn().status.success());
    assert_eq!(before, ws.journals());
    assert_eq!(before.len(), 1, "no second run was started");

    // The proposal exists as a record and nowhere else: no journal
    // carries an operator event, and the run's status is unchanged.
    let store = brokkr_store::Store::open_read_only(&ws.db()).unwrap();
    for (run_id, _, _) in &before {
        for event in store.load(run_id).unwrap() {
            assert!(
                !matches!(
                    event.event_type,
                    brokkr_core::EventType::OperatorCommanded
                        | brokkr_core::EventType::OperatorAccepted
                        | brokkr_core::EventType::OperatorRejected
                ),
                "no operator event was recorded by any muninn path"
            );
        }
    }
    assert_eq!(ws.records().len(), 1);
}

/// The structural half of the guarantees above, as decision 0012's
/// single-egress proof is written: grep. A module that never names the
/// journal writer, the engine, the operator-command path or the secret
/// resolver cannot reach any of them, and the reason it cannot stays
/// visible in CI rather than in a reviewer's memory.
#[test]
fn the_muninn_path_names_no_journal_writer_no_engine_and_no_secret_resolver() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let sources = [
        crates_dir.join("brokkr-cli/src/muninn.rs"),
        crates_dir.join("brokkr-cli/src/muninn/record.rs"),
        crates_dir.join("brokkr-protocol/src/oneshot.rs"),
    ];
    let forbidden = [
        "append_next(",
        "Engine::",
        "operator_command(",
        "resolve_bindings(",
        "secret::",
        "Store::open(",
    ];
    for source in &sources {
        let text = std::fs::read_to_string(source)
            .unwrap_or_else(|e| panic!("{} must exist: {e}", source.display()));
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} names '{needle}'",
                source.display()
            );
        }
    }
}

/// 0019 law 4: the command carries the name, the output does not carry
/// the lore. What a human reads — the charter the seat is handed and the
/// text this command prints — stays plain mechanic language.
#[test]
fn nothing_a_human_reads_from_this_command_carries_the_lore() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    // Deliberately narrow: words that can only be lore. A broader list
    // catches ordinary English ("overseer" carries "verse") and would
    // make this test a nuisance rather than a guard.
    let lore = [
        "raven", "odin", "dusk", "saga", "norse", "valhalla", "asgard", "myth",
    ];
    let readable = [
        root.join("agents/charters/muninn.md"),
        root.join("agents/muninn.json"),
        root.join("crates/brokkr-cli/src/muninn.rs"),
        root.join("crates/brokkr-cli/src/muninn/record.rs"),
    ];
    for path in &readable {
        let text = std::fs::read_to_string(path).unwrap().to_lowercase();
        for word in lore {
            assert!(!text.contains(word), "{} carries '{word}'", path.display());
        }
    }

    let (ws, _, _) = staged();
    ws.proposes(json!({
        "fleet_summary": "one run", "parked_runs": [], "work_queue": [],
    }));
    let output = ws.muninn();
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();
    assert!(printed.contains("muninn"), "the name is fine: {printed}");
    for word in lore {
        assert!(!printed.contains(word), "the output carries '{word}'");
    }
}

/// The production charter is the instruction set a real seat reads, so
/// the citation shapes the validator accepts must be the ones it teaches.
/// The staged fixtures script their driver through a trivial charter, so
/// this asserts the shipped file directly: a charter-compliant seat can
/// emit the `realm`/`crossing` entry, and the citation rule covers both
/// shapes rather than demanding `run_id`/`seq` of every entry.
#[test]
fn the_shipped_charter_teaches_the_citation_shapes_the_validator_accepts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let charter = std::fs::read_to_string(root.join("agents/charters/muninn.md")).unwrap();
    assert!(charter.contains("`realm`"), "{charter}");
    assert!(charter.contains("`crossing`"), "{charter}");
    assert!(
        charter.contains("`run_id`") && charter.contains("`seq`"),
        "{charter}"
    );
    let rules = charter
        .split("## The rules the report is judged by")
        .nth(1)
        .expect("the charter carries its rules section");
    assert!(
        rules.contains("crossing") && rules.contains("realm"),
        "the citation rule names the crossing shape: {rules}"
    );
    assert!(
        rules.contains("run_id") && rules.contains("seq"),
        "the citation rule still names the run shape: {rules}"
    );
}

/// The shipped definition is the one the operator gets, so its bounds
/// are asserted on the shipped file rather than on a fixture.
#[test]
fn the_shipped_definition_runs_once_and_declares_no_retry_ladder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let raw = std::fs::read_to_string(root.join("agents/muninn.json")).unwrap();
    let definition: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(definition["limits"]["max_attempts"], 1);
    assert!(definition["limits"]["timeout_seconds"].as_u64().unwrap() > 0);
    assert!(
        definition["charter"] == "charters/muninn.md",
        "the two-file shape of the library"
    );
}
