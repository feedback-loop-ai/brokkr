use super::*;

use forge_core::EventType;

// ------------------------------------------------------- a fixture fleet

struct Fleet {
    dir: tempfile::TempDir,
}

impl Fleet {
    fn new() -> Fleet {
        Fleet {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    fn db(&self) -> PathBuf {
        self.dir.path().join("forge.db")
    }

    fn store(&self) -> Store {
        Store::open(&self.db()).unwrap()
    }

    /// A parked run: one review seat that reported a session cost, a
    /// ruling carrying two residual findings, and a park.
    fn parked(&self, run_id: &str) {
        let mut store = self.store();
        store
            .create_run(run_id, "make it shippable", "self", &json!({}))
            .unwrap();
        let mut append = |kind, payload| {
            store
                .append_next(run_id, kind, payload, None, None)
                .unwrap();
        };
        append(
            EventType::RunStarted,
            json!({"feature": "make it shippable", "manifest": {}}),
        );
        append(EventType::PhaseEntered, json!({"phase": "review"}));
        append(
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "review", "phase": "review"}),
        );
        append(
            EventType::EffectStarted,
            json!({"effect_id": "eff", "attempt_id": "att"}),
        );
        append(
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff", "attempt_id": "att", "checkpoint": {
                "step": "claude-code-session-finished",
                "session_id": "sess-1",
                "total_cost_usd": 1.25,
                "num_turns": 9,
            }}),
        );
        append(
            EventType::EffectSucceeded,
            json!({"effect_id": "eff", "attempt_id": "att",
                   "result": {"result": "residual"}}),
        );
        append(
            EventType::TransitionDecided,
            json!({"from": "review", "result": "residual",
                   "rule_id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "next": null,
                   "severity": null,
                   "inputs": {"max_residual_severity": "high",
                              "has_security_residual": true},
                   "problem": "residual severity above medium"}),
        );
        append(
            EventType::RunParked,
            json!({"reason": "residual severity above medium; not shippable"}),
        );
    }

    /// A completed run with no seat telemetry at all.
    fn completed(&self, run_id: &str) {
        let mut store = self.store();
        store
            .create_run(run_id, "ship it", "self", &json!({}))
            .unwrap();
        let mut append = |kind, payload| {
            store
                .append_next(run_id, kind, payload, None, None)
                .unwrap();
        };
        append(
            EventType::RunStarted,
            json!({"feature": "ship it", "manifest": {}}),
        );
        append(EventType::PhaseEntered, json!({"phase": "ship"}));
        append(
            EventType::EffectRequested,
            json!({"effect_id": "e2", "seat": "ship", "phase": "ship"}),
        );
        append(
            EventType::EffectStarted,
            json!({"effect_id": "e2", "attempt_id": "a2"}),
        );
        append(
            EventType::EffectSucceeded,
            json!({"effect_id": "e2", "attempt_id": "a2", "result": {"result": "shipped"}}),
        );
        append(
            EventType::TransitionDecided,
            json!({"from": "ship", "result": "shipped", "rule_id": "SHIP-OK",
                   "next": "done", "severity": "normal", "inputs": {},
                   "problem": null}),
        );
        append(EventType::PhaseEntered, json!({"phase": "done"}));
        append(EventType::RunCompleted, json!({}));
    }
}

/// Deliberately far ahead of any wall clock a fixture is written under,
/// so `age` is a real elapsed span rather than a negative one.
const NOW: &str = "2099-01-01T00:00:00Z";

fn parked_dossier() -> Dossier {
    let fleet = Fleet::new();
    fleet.parked("parked-run");
    fleet.completed("done-run");
    let store = Store::open_read_only(&fleet.db()).unwrap();
    dossier(&store, NOW).unwrap()
}

// ------------------------------------------------------------- the dossier

#[test]
fn the_dossier_is_derived_from_the_view_models_and_carries_its_citations() {
    let derived = parked_dossier();
    assert_eq!(derived.value["dossier_version"], 1);
    assert_eq!(derived.value["generated_at"], NOW);
    assert_eq!(derived.value["fleet"]["runs"], 2);
    assert_eq!(derived.value["fleet"]["awaiting_operator"], 1);
    assert_eq!(derived.value["fleet"]["completed"], 1);
    assert_eq!(derived.value["fleet"]["running"], 0);
    assert_eq!(derived.value["fleet"]["stopped"], 0);

    let runs = derived.value["runs"].as_array().unwrap();
    let parked = runs
        .iter()
        .find(|run| run["run_id"] == "parked-run")
        .unwrap();
    assert_eq!(parked["status"], "awaiting_operator");
    assert_eq!(parked["seq"], 8);
    assert_eq!(parked["phase"], "review");
    assert_eq!(
        parked["park_reason"],
        "residual severity above medium; not shippable"
    );
    assert_eq!(parked["operator_commands"], json!(["retry", "stop"]));
    assert_eq!(parked["cost_usd"], 1.25);
    assert!(
        parked["age"].as_str().is_some_and(|age| age.ends_with('m')),
        "age is derived against the clock the caller passed: {}",
        parked["age"]
    );
    assert_eq!(
        parked["last_ruling"]["rule_id"],
        "REVIEW-RESIDUAL-ABOVE-MEDIUM"
    );
    assert_eq!(parked["seats"][0]["seat"], "review");
    assert_eq!(parked["seats"][0]["cost_usd"], 1.25);

    let done = runs.iter().find(|run| run["run_id"] == "done-run").unwrap();
    assert_eq!(done["status"], "completed");
    assert_eq!(
        done["operator_commands"],
        json!([]),
        "a completed run admits no operator command"
    );
    assert_eq!(
        done["cost_usd"],
        Value::Null,
        "no telemetry means no claim, not a zero"
    );

    let findings = derived.value["residual_findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0]["run_id"], "parked-run");
    assert_eq!(findings[0]["seq"], 7);

    // The head of each run and every ruling a finding came from.
    assert!(derived.states("parked-run", 8));
    assert!(derived.states("parked-run", 7));
    assert!(derived.states("done-run", 8));
    assert!(!derived.states("parked-run", 3));
    assert!(!derived.states("no-such-run", 8));
    assert!(derived.admits("parked-run", "retry"));
    assert!(!derived.admits("parked-run", "ship"));
    assert!(!derived.admits("done-run", "retry"));
    assert!(!derived.admits("no-such-run", "retry"));
}

#[test]
fn an_empty_fleet_is_an_empty_dossier_not_a_failure() {
    let fleet = Fleet::new();
    drop(fleet.store());
    let store = Store::open_read_only(&fleet.db()).unwrap();
    let derived = dossier(&store, NOW).unwrap();
    assert_eq!(derived.value["fleet"]["runs"], 0);
    assert_eq!(derived.value["runs"], json!([]));
    assert_eq!(derived.value["residual_findings"], json!([]));
    assert!(derived.facts.is_empty());
}

#[test]
fn a_run_the_journal_cannot_read_names_itself_rather_than_vanishing() {
    // In a console one bad run is a dim row; in a verb it is an error
    // and a nonzero exit, naming the run so the next command is obvious.
    let fleet = Fleet::new();
    let mut store = fleet.store();
    store
        .create_run("empty-run", "never started", "self", &json!({}))
        .unwrap();
    drop(store);
    let reader = Store::open_read_only(&fleet.db()).unwrap();
    let error = dossier(&reader, NOW).unwrap_err().to_string();
    assert!(error.contains("folding run 'empty-run'"), "{error}");
    drop(reader);

    let broken = Fleet::new();
    let mut store = broken.store();
    store
        .create_run("broken-run", "unreadable", "self", &json!({}))
        .unwrap();
    drop(store);
    let conn = rusqlite::Connection::open(broken.db()).unwrap();
    conn.execute(
        "INSERT INTO events (run_id, seq, event_hash, envelope)
         VALUES ('broken-run', 1, 'x', 'not an envelope')",
        [],
    )
    .unwrap();
    drop(conn);
    let reader = Store::open_read_only(&broken.db()).unwrap();
    let error = dossier(&reader, NOW).unwrap_err().to_string();
    assert!(error.contains("loading run 'broken-run'"), "{error}");
}

#[test]
fn a_database_that_cannot_be_opened_is_named_before_anything_is_asked_of_it() {
    let dir = tempfile::tempdir().unwrap();
    let error = run(
        &dir.path().join("absent.db"),
        &dir.path().join("agents"),
        &dir.path().join("adapters"),
        &dir.path().join("muninn.ndjson"),
        NOW,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("absent.db"), "{error}");
    assert!(error.contains("for reading"), "{error}");
}

// ------------------------------------------------------------ the seat

struct Staged {
    dir: tempfile::TempDir,
}

impl Staged {
    fn new(limits: Value) -> Staged {
        let staged = Staged {
            dir: tempfile::tempdir().unwrap(),
        };
        for sub in ["agents/charters", "adapters"] {
            std::fs::create_dir_all(staged.dir.path().join(sub)).unwrap();
        }
        std::fs::write(
            staged.dir.path().join("agents/charters/muninn.md"),
            "# read and propose\n",
        )
        .unwrap();
        std::fs::write(
            staged.dir.path().join("agents/muninn.json"),
            serde_json::to_string(&json!({
                "description": "reads the fleet and proposes",
                "charter": "charters/muninn.md",
                "models": ["opus", "sonnet"],
                "limits": limits,
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            staged.dir.path().join("adapters/test.json"),
            serde_json::to_string(&json!({
                "provider": "test",
                "binary": "true",
                "driver": ["{forge}", "driver", "claude", "--"],
                "models": {"opus": "test/opus", "sonnet": "test/sonnet"},
                "model_flag": "--model",
                "tool_permissions": "unsupported",
                "mcp": "unsupported",
            }))
            .unwrap(),
        )
        .unwrap();
        staged
    }

    fn seat(&self) -> Result<Seat> {
        seat(
            &self.dir.path().join("agents"),
            &self.dir.path().join("adapters"),
        )
    }
}

#[test]
fn the_seat_resolves_through_the_agent_library_and_pins_its_deadline() {
    let staged = Staged::new(json!({"max_attempts": 1, "timeout_seconds": 900}));
    let resolved = staged.seat().unwrap();
    assert_eq!(resolved.deadline, Duration::from_secs(900));
    assert_eq!(resolved.model, "opus");
    assert_eq!(resolved.provider, "test");
    assert!(resolved.charter.ends_with("charters/muninn.md"));
    assert_eq!(
        resolved.command[1..],
        ["driver", "claude", "--", "--model", "test/opus"],
        "the token expands to this executable and the rest is the adapter's"
    );
    assert_ne!(resolved.command[0], "{forge}");
}

#[test]
fn a_seat_with_a_retry_ladder_of_its_own_is_refused() {
    let staged = Staged::new(json!({"max_attempts": 2, "timeout_seconds": 900}));
    let error = staged.seat().unwrap_err().to_string();
    assert!(error.contains("max_attempts 2"), "{error}");
    assert!(error.contains("no retry ladder of its own"), "{error}");
}

#[test]
fn an_unknown_agent_or_an_unreadable_library_is_a_plain_refusal() {
    let empty = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(empty.path().join("agents")).unwrap();
    std::fs::create_dir_all(empty.path().join("adapters")).unwrap();
    let error = seat(&empty.path().join("agents"), &empty.path().join("adapters"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("not in the library"), "{error}");
}

#[test]
fn the_seat_input_names_a_scratch_directory_and_no_store_of_secrets() {
    let staged = Staged::new(json!({"max_attempts": 1, "timeout_seconds": 900}));
    let resolved = staged.seat().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let input = seat_input(&resolved, &parked_dossier(), scratch.path());
    assert_eq!(input["workdir"], scratch.path().to_string_lossy().as_ref());
    assert_eq!(
        input["result_path"],
        scratch
            .path()
            .join("report.json")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(input["allowed_results"], json!(["proposed"]));
    assert_eq!(input["phase"], "muninn");
    assert_eq!(input["context"]["dossier_version"], 1);
    assert!(
        input.get("secrets").is_none() && input.get("secrets_file").is_none(),
        "0012 bindings are for seats that do work: {input}"
    );
    let object = input.as_object().unwrap();
    assert!(
        !object.keys().any(|key| key == "repo"),
        "no repository is named in the input: {input}"
    );
}

// -------------------------------------------------------- the validation

fn reported(inputs: Value) -> Value {
    json!({"result": "proposed", "inputs": inputs, "notes": "read the fleet"})
}

fn empty_report() -> Value {
    reported(json!({
        "fleet_summary": "two runs; one is parked",
        "parked_runs": [],
        "work_queue": [],
    }))
}

fn refused(result: Value) -> String {
    validate(&parked_dossier(), &result).err().expect("refused")
}

#[test]
fn a_well_formed_report_validates_with_its_citations_collected() {
    let report = validate(
        &parked_dossier(),
        &reported(json!({
            "fleet_summary": "two runs; one is parked on residual severity",
            "parked_runs": [{
                "run_id": "parked-run", "seq": 8, "command": "stop",
                "reasoning": "the ruling was hard; a retry re-runs the same review",
            }],
            "work_queue": [{
                "run_id": "parked-run", "seq": 7,
                "finding": "max_residual_severity: high",
                "reasoning": "the highest residual in the fleet",
            }],
        })),
    )
    .unwrap();
    assert_eq!(
        report.fleet_summary,
        "two runs; one is parked on residual severity"
    );
    assert_eq!(report.parked_runs.len(), 1);
    assert_eq!(report.parked_runs[0]["command"], "stop");
    assert_eq!(report.work_queue.len(), 1);
    assert_eq!(
        report.citations,
        vec![("parked-run".to_string(), 7), ("parked-run".to_string(), 8),]
    );

    // A report with nothing to advise is still a complete report.
    let quiet = validate(&parked_dossier(), &empty_report()).unwrap();
    assert!(quiet.parked_runs.is_empty() && quiet.work_queue.is_empty());
    assert!(quiet.citations.is_empty());
}

#[test]
fn a_report_that_does_not_meet_the_shape_is_refused_and_never_repaired() {
    assert!(refused(json!({"result": "complete"})).contains("reached 'complete'"));
    assert!(refused(json!({})).contains("reached '—'"));
    assert!(refused(json!({"result": "proposed"})).contains("no 'inputs' object"));
    assert!(refused(reported(json!({}))).contains("missing a non-empty 'fleet_summary'"));
    assert!(
        refused(reported(json!({"fleet_summary": ""}))).contains("non-empty 'fleet_summary'"),
        "an empty summary is as absent as no summary"
    );
    assert!(refused(reported(json!({"fleet_summary": "s"})))
        .contains("missing the 'parked_runs' array"));
    assert!(refused(reported(json!({
        "fleet_summary": "s", "parked_runs": []
    })))
    .contains("missing the 'work_queue' array"));
}

#[test]
fn a_proposal_that_cannot_be_followed_back_to_a_journal_fact_is_refused() {
    let parked = |entry: Value| {
        reported(json!({
            "fleet_summary": "s", "parked_runs": [entry], "work_queue": [],
        }))
    };
    assert!(refused(parked(json!({}))).contains("missing a non-empty 'run_id'"));
    assert!(refused(parked(json!({"run_id": "parked-run"}))).contains("missing a numeric 'seq'"));
    assert!(refused(parked(json!({"run_id": "parked-run", "seq": 3})))
        .contains("cites parked-run seq 3, which the dossier does not state"));
    assert!(
        refused(parked(json!({"run_id": "ghost-run", "seq": 8}))).contains("cites ghost-run seq 8")
    );
    assert!(refused(parked(json!({"run_id": "parked-run", "seq": 8})))
        .contains("a parked-run proposal is missing a non-empty 'command'"));
    assert!(refused(parked(
        json!({"run_id": "parked-run", "seq": 8, "command": "ship"})
    ))
    .contains("suggests 'ship' for parked-run, which is not an operator command"));
    assert!(refused(parked(
        json!({"run_id": "parked-run", "seq": 8, "command": "retry"})
    ))
    .contains("a parked-run proposal is missing a non-empty 'reasoning'"));

    let queued = |entry: Value| {
        reported(json!({
            "fleet_summary": "s", "parked_runs": [], "work_queue": [entry],
        }))
    };
    assert!(refused(queued(json!({"run_id": "parked-run", "seq": 4})))
        .contains("a work-queue entry cites parked-run seq 4"));
    assert!(refused(queued(json!({"run_id": "parked-run", "seq": 7})))
        .contains("a work-queue entry is missing a non-empty 'finding'"));
    assert!(refused(queued(
        json!({"run_id": "parked-run", "seq": 7, "finding": "high"})
    ))
    .contains("a work-queue entry is missing a non-empty 'reasoning'"));
}

// ------------------------------------------------------ usage and rendering

#[test]
fn usage_is_read_from_the_session_checkpoint_or_claimed_at_all() {
    assert_eq!(usage(&[]), Value::Null);
    assert_eq!(usage(&[json!({"step": "working"})]), Value::Null);
    assert_eq!(usage(&[json!({"turn": 1})]), Value::Null);
    assert_eq!(
        usage(&[
            json!({"step": "seat-turn", "turn": 1}),
            json!({"step": "claude-code-session-finished", "total_cost_usd": 0.5,
                   "num_turns": 4, "session_id": "sess-1"}),
        ]),
        json!({"cost_usd": 0.5, "turns": 4, "session_id": "sess-1"})
    );
}

#[test]
fn a_recorded_entry_renders_every_proposal_with_its_citation() {
    let staged = Staged::new(json!({"max_attempts": 1, "timeout_seconds": 900}));
    let resolved = staged.seat().unwrap();
    let derived = parked_dossier();
    let report = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one run is parked",
            "parked_runs": [{
                "run_id": "parked-run", "seq": 8, "command": "stop",
                "reasoning": "the ruling was hard",
            }],
            "work_queue": [{
                "run_id": "parked-run", "seq": 7, "finding": "severity high",
                "reasoning": "the highest residual",
            }],
        })),
    )
    .unwrap();
    let line = entry(
        NOW,
        &resolved,
        &derived,
        &report,
        usage(&[json!({"step": "claude-code-session-finished",
                       "total_cost_usd": 0.5, "num_turns": 4})]),
    );
    assert_eq!(line["record_version"], 1);
    assert_eq!(line["recorded_at"], NOW);
    assert_eq!(line["agent"]["name"], "muninn");
    assert_eq!(line["agent"]["model"], "opus");
    assert_eq!(line["agent"]["deadline_seconds"], 900);
    assert_eq!(line["dossier"]["fleet"]["runs"], 2);
    assert_eq!(line["usage"]["cost_usd"], 0.5);
    assert_eq!(
        line["citations"],
        json!([
            {"run_id": "parked-run", "seq": 7},
            {"run_id": "parked-run", "seq": 8},
        ])
    );

    let rendered = render(&line);
    assert!(rendered.contains("1 proposals for parked runs · 1 findings queued"));
    assert!(rendered.contains("summary: one run is parked"));
    assert!(rendered.contains("parked parked-run seq 8 · suggest 'stop' · the ruling was hard"));
    assert!(rendered.contains("queue parked-run seq 7 · severity high · the highest residual"));
    assert!(rendered.contains("cites: parked-run seq 7, parked-run seq 8"));
}

/// The report is model-authored prose, which makes it the most hostile
/// string this binary prints: an escape sequence in a reasoning line
/// could clear the frame or overwrite the citation above it, and a
/// right-to-left override could reverse the command being suggested. The
/// rendering goes through the same sanitizer every other readout uses.
#[test]
fn a_rendered_proposal_cannot_forge_the_line_above_it() {
    let rendered = render(&json!({
        "recorded_at": "2026-08-31T00:00:00Z",
        "fleet_summary": "one run\u{1b}[2Jcleared",
        "parked_runs": [{
            "run_id": "parked-run", "seq": 8, "command": "stop",
            "reasoning": "safe\rforged: every run is green",
        }],
        "work_queue": [],
        "citations": [{"run_id": "parked-run", "seq": 8}],
    }));
    assert!(
        !rendered.contains('\u{1b}'),
        "no escape survives: {rendered:?}"
    );
    assert!(!rendered.contains('\r'), "no carriage return survives");
    assert!(rendered.contains("summary: one run[2Jcleared"));
    assert!(rendered.contains("safeforged: every run is green"));
    assert!(
        !render(&json!({"fleet_summary": "seat\u{202E}gnippots"})).contains('\u{202E}'),
        "the reordering characters go too"
    );
}

#[test]
fn a_record_line_missing_its_fields_renders_absence_rather_than_panicking() {
    let rendered = render(&json!({"parked_runs": [{}], "work_queue": [{}], "citations": [{}]}));
    assert!(rendered.starts_with("— · 1 proposals for parked runs · 1 findings queued"));
    assert!(rendered.contains("parked — seq — · suggest '—' · —"));
    assert!(rendered.contains("queue — seq — · — · —"));
    assert!(rendered.contains("cites: — seq —"));
}

// ------------------------------------------------------------ the listing

#[test]
fn the_listing_reads_the_record_back_plainly_and_as_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("muninn.ndjson");
    assert!(
        list(&path, false).is_ok(),
        "an absent record lists as empty"
    );
    assert!(list(&path, true).is_ok());
    record::append(
        &path,
        &json!({"recorded_at": NOW, "fleet_summary": "quiet",
                "parked_runs": [], "work_queue": [], "citations": []}),
    )
    .unwrap();
    assert!(list(&path, false).is_ok());
    assert!(list(&path, true).is_ok());
}
