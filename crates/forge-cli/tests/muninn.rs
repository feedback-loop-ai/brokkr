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

fn forge_bin() -> &'static str {
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
                "binary": forge_bin(),
                "driver": [
                    forge_bin(), "fake-driver",
                    "--script", ws.path().join("script.json").to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy(),
                ],
                "models": {"work": "fake/work"},
                "model_flag": "--model",
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
        Command::new(forge_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap()
    }

    /// Drive one run to its conclusion and return its run id.
    fn run_once(&self, feature: &str) -> String {
        let output = self.brokkr(&[
            "run",
            "--bundle",
            "bundle",
            "--feature",
            feature,
            "--db",
            ".forge/forge.db",
            "--repo",
            "repo",
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
        let store = forge_store::Store::open_read_only(&self.db()).unwrap();
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
    let store = forge_store::Store::open_read_only(&ws.db()).unwrap();
    let events = store.load(&run_id).unwrap();
    let state = forge_core::fold(&events).unwrap();
    assert_eq!(
        forge_view::status_str(&state.status),
        "stopped",
        "the staged run reaches a hard stop on its residual"
    );
    let findings = forge_view::residual_findings(&run_id, &events);
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
    let mut reader = forge_store::Store::open_read_only(&ws.db()).unwrap();
    assert!(reader
        .append_next(
            &before[0].0,
            forge_core::EventType::PhaseEntered,
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
    // that never comes, and the deadline kill takes it down. Unix
    // only, honestly: Windows never actually exercised this path (the
    // old sleep-based stall returned instantly there and the run
    // concluded in time), and its kill-a-blocked-driver semantics are
    // an untested territory of their own — a recorded gap for a
    // Windows-runner slice, not a silent one.
    #[cfg(unix)]
    {
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
    let store = forge_store::Store::open_read_only(&ws.db()).unwrap();
    for (run_id, _, _) in &before {
        for event in store.load(run_id).unwrap() {
            assert!(
                !matches!(
                    event.event_type,
                    forge_core::EventType::OperatorCommanded
                        | forge_core::EventType::OperatorAccepted
                        | forge_core::EventType::OperatorRejected
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
        crates_dir.join("forge-cli/src/muninn.rs"),
        crates_dir.join("forge-cli/src/muninn/record.rs"),
        crates_dir.join("forge-protocol/src/oneshot.rs"),
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
        root.join("crates/forge-cli/src/muninn.rs"),
        root.join("crates/forge-cli/src/muninn/record.rs"),
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
