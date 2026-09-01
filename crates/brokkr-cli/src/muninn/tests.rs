use super::*;

use brokkr_core::EventType;

/// The fleet of ONE journal — what a world with a single hearth has
/// always handed the seat, and the shape every test below that predates
/// many hearths (decision 0026) still asserts.
fn dossier(store: &Store, now: &str) -> Result<Dossier> {
    dossier_of(&[Source { realm: None, store }], now)
}

/// One hearth, named by the journal it reads.
fn hearth(realm: &str, journal: &Path) -> Hearth {
    Hearth {
        realms: vec![realm.to_string()],
        journal: journal.to_path_buf(),
    }
}

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

    /// A journal that genuinely does not fold: an `operator/accepted`
    /// naming a command this run never carried. No rule can read an
    /// unattached acceptance at any cursor — the case the aide must
    /// surface rather than choke on.
    fn poisoned(&self, run_id: &str) {
        let mut store = self.store();
        store
            .create_run(
                run_id,
                "the acceptance that names no command",
                "self",
                &json!({}),
            )
            .unwrap();
        let mut append = |kind, payload| {
            store
                .append_next(run_id, kind, payload, None, None)
                .unwrap();
        };
        append(
            EventType::RunStarted,
            json!({"feature": "the acceptance that names no command", "manifest": {}}),
        );
        append(EventType::PhaseEntered, json!({"phase": "verify"}));
        append(
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "verify", "phase": "verify"}),
        );
        append(
            EventType::EffectStarted,
            json!({"effect_id": "eff", "attempt_id": "att"}),
        );
        append(
            EventType::OperatorCommanded,
            json!({"command_id": "cmd", "command": "stop", "args": {},
                   "operator": "operator"}),
        );
        append(
            EventType::OperatorAccepted,
            json!({"command_id": "a-command-never-issued", "operator": "operator",
                   "reason": "stop it now"}),
        );
    }

    /// The live journal that used to be this fleet's poisoned one: the
    /// verbatim fixture export, replayed pair by pair — an operator
    /// `stop` accepted while the verify seat's effect was in flight.
    fn stopped_mid_flight(&self, run_id: &str) {
        crate::tests::stopped_mid_flight_store(&self.db(), run_id);
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

/// The run the quarantine was written for is no longer quarantined:
/// the fold has the arm for an operator stop accepted mid-flight, so
/// the aide reads the live journal as what it is. No finding, no
/// `quarantined` count — a run that reads is not a fault to surface.
#[test]
fn an_operator_stop_mid_flight_leaves_the_dossier_no_quarantine_to_report() {
    let fleet = Fleet::new();
    fleet.parked("parked-run");
    fleet.stopped_mid_flight("stopped-mid-flight");
    let store = Store::open_read_only(&fleet.db()).unwrap();
    let derived = dossier(&store, NOW).unwrap();

    assert_eq!(derived.value["fleet"]["runs"], 2);
    assert_eq!(derived.value["fleet"]["running"], 1);
    assert_eq!(
        derived.value["fleet"]["quarantined"], 0,
        "nothing to quarantine: {}",
        derived.value["fleet"]
    );
    let runs = derived.value["runs"].as_array().unwrap();
    let stopped = runs
        .iter()
        .find(|run| run["run_id"] == "stopped-mid-flight")
        .expect("the run is listed");
    assert_eq!(stopped["status"], "running");
    assert_eq!(stopped["seq"], 105);
    assert_eq!(stopped["phase"], "verify");
    assert_eq!(stopped["fold_error"], Value::Null);

    // The quarantine finding is gone with it: the only findings left
    // are the parked run's own residuals.
    let findings = derived.value["residual_findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .all(|finding| finding["run_id"] == "parked-run"),
        "no quarantine finding survives: {findings:?}"
    );
    assert!(
        findings
            .iter()
            .all(|finding| finding["input"] != "journal_folds"),
        "{findings:?}"
    );
}

/// The fleet fault that grounded the aide: one run whose journal does
/// not fold used to abort the whole dossier, so a single poisoned
/// journal blinded the overseer to every healthy run. It is now a
/// quarantined row AND a finding — an unfoldable journal is exactly
/// what an operator's aide exists to surface — and the seat can propose
/// about it, because the dossier states the sequence the fold refused
/// at as a citable fact.
#[test]
fn one_unfoldable_journal_is_a_finding_and_the_rest_of_the_fleet_still_reads() {
    let fleet = Fleet::new();
    fleet.parked("parked-run");
    fleet.poisoned("poisoned-run");
    fleet.completed("done-run");
    let store = Store::open_read_only(&fleet.db()).unwrap();
    let derived = dossier(&store, NOW).unwrap();

    // Every run is still read, and the healthy ones keep their facts.
    assert_eq!(derived.value["fleet"]["runs"], 3);
    assert_eq!(derived.value["fleet"]["awaiting_operator"], 1);
    assert_eq!(derived.value["fleet"]["completed"], 1);
    assert_eq!(derived.value["fleet"]["quarantined"], 1);
    let runs = derived.value["runs"].as_array().unwrap();
    let poisoned = runs
        .iter()
        .find(|run| run["run_id"] == "poisoned-run")
        .expect("the poisoned run is listed, not dropped");
    assert_eq!(poisoned["status"], "?");
    assert_eq!(poisoned["seq"], 6, "the sequence the fold refused at");
    assert_eq!(poisoned["phase"], Value::Null);
    assert_eq!(poisoned["feature"], "the acceptance that names no command");
    assert_eq!(poisoned["operator_commands"], json!([]));
    let fold_error = poisoned["fold_error"].as_str().unwrap();
    assert!(
        fold_error.starts_with("event 6: operator/accepted without a matching command"),
        "the row carries the fold's own words: {fold_error}"
    );

    // …and it is raised as a finding beside the evaluator's own.
    let findings = derived.value["residual_findings"].as_array().unwrap();
    let quarantined = findings
        .iter()
        .find(|finding| finding["run_id"] == "poisoned-run")
        .expect("the quarantined run is a finding the seat can read");
    assert_eq!(quarantined["seq"], 6);
    assert_eq!(quarantined["input"], "journal_folds");
    assert_eq!(quarantined["value"], "false");
    assert!(
        quarantined["line"]
            .as_str()
            .unwrap()
            .contains("journal does not fold"),
        "{quarantined}"
    );
    assert_eq!(
        findings.len(),
        3,
        "the parked run's two residuals are still there: {findings:?}"
    );

    // The seat may propose about it: the citation validates, and no
    // operator command is admitted for a run nothing could fold.
    let queued = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one journal does not fold",
            "parked_runs": [],
            "work_queue": [{
                "run_id": "poisoned-run", "seq": 6,
                "finding": "the journal does not fold",
                "reasoning": "an unreadable run is the first thing to look at",
            }],
        })),
    )
    .unwrap();
    assert_eq!(queued.citations, [(None, "poisoned-run".to_string(), 6)]);
    let refused = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one journal does not fold",
            "parked_runs": [{
                "run_id": "poisoned-run", "seq": 6, "command": "retry",
                "reasoning": "just retry it",
            }],
            "work_queue": [],
        })),
    )
    .err()
    .expect("a run nothing could fold admits no command");
    assert!(
        refused.contains("not an operator command the dossier states"),
        "{refused}"
    );

    // And the seat may not dress the unfoldable run in a realm: the
    // dossier states that fact keyed by NO realm, so a citation that
    // claims one disagrees with the record and is refused saying so.
    let dressed = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one journal does not fold",
            "parked_runs": [],
            "work_queue": [{
                "run_id": "poisoned-run", "seq": 6, "realm": "alpha",
                "finding": "the journal does not fold",
                "reasoning": "an unreadable run is the first thing to look at",
            }],
        })),
    )
    .err()
    .expect("a realm the dossier does not state is refused");
    assert!(dressed.contains("no realm"), "{dressed}");
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
    // A journal that does not FOLD is quarantined and reported: the aide
    // still reads the rest of the fleet. A journal that cannot be READ
    // at all is a store fault, not a protocol one, and stays fatal —
    // nothing here can say which run the unreadable bytes belonged to.
    let fleet = Fleet::new();
    let mut store = fleet.store();
    store
        .create_run("empty-run", "never started", "self", &json!({}))
        .unwrap();
    drop(store);
    let reader = Store::open_read_only(&fleet.db()).unwrap();
    let derived = dossier(&reader, NOW).unwrap();
    assert_eq!(derived.value["runs"][0]["status"], "?");
    assert_eq!(derived.value["runs"][0]["fold_error"], "journal is empty");
    assert_eq!(derived.value["fleet"]["quarantined"], 1);
    // An empty journal has no event to cite: the citation is the
    // position before the first one.
    assert!(derived.states("empty-run", 0));
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
        &[hearth("solo", &dir.path().join("absent.db"))],
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

// -------------------------------- many hearths (0026 rulings 3 and 5)

/// Two hearths, each with its own runs, read side by side.
fn many_hearth_dossier(alpha: &Fleet, beta: &Fleet) -> Dossier {
    let alpha_store = Store::open_read_only(&alpha.db()).unwrap();
    let beta_store = Store::open_read_only(&beta.db()).unwrap();
    dossier_of(
        &[
            Source {
                realm: Some("alpha"),
                store: &alpha_store,
            },
            Source {
                realm: Some("beta"),
                store: &beta_store,
            },
        ],
        NOW,
    )
    .unwrap()
}

/// The raven flies over every hearth the map names, and every fact it
/// states says which one it came from (decision 0026 ruling 3).
#[test]
fn a_many_hearth_dossier_states_the_realm_every_fact_came_from() {
    let alpha = Fleet::new();
    alpha.parked("parked-run");
    let beta = Fleet::new();
    beta.completed("done-run");
    beta.poisoned("poisoned-run");
    let derived = many_hearth_dossier(&alpha, &beta);

    let rows = derived.value["runs"].as_array().unwrap();
    let seen: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| {
            (
                row["realm"].as_str().unwrap(),
                row["run_id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            ("alpha", "parked-run"),
            ("beta", "done-run"),
            ("beta", "poisoned-run"),
        ]
    );
    // The world states its shape, and the counts are the whole world's.
    assert_eq!(derived.value["fleet"]["realms"], json!(["alpha", "beta"]));
    assert_eq!(derived.value["fleet"]["runs"], json!(3));
    assert_eq!(derived.value["fleet"]["quarantined"], json!(1));

    // Findings are cited per realm too — the quarantined journal is
    // beta's, and the parked run's residuals are alpha's.
    let findings = derived.value["residual_findings"].as_array().unwrap();
    assert!(findings
        .iter()
        .any(|f| f["realm"] == json!("beta") && f["run_id"] == json!("poisoned-run")));
    assert!(findings
        .iter()
        .any(|f| f["realm"] == json!("alpha") && f["run_id"] == json!("parked-run")));

    // And the closed set a proposal may cite carries the realm.
    assert!(derived
        .facts
        .iter()
        .any(|(realm, run_id, _)| realm.as_deref() == Some("alpha") && run_id == "parked-run"));
}

/// A validated report carries the realm back out, whether or not the
/// seat wrote one — and a seat that names the WRONG realm is refused
/// rather than quietly corrected (decision 0001).
#[test]
fn a_proposal_is_recorded_under_the_realm_the_dossier_states_for_it() {
    let alpha = Fleet::new();
    alpha.parked("parked-run");
    let beta = Fleet::new();
    beta.completed("done-run");
    let derived = many_hearth_dossier(&alpha, &beta);

    let report = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one parked run in alpha",
            "parked_runs": [{
                "realm": "alpha",
                "run_id": "parked-run", "seq": 8, "command": "stop",
                "reasoning": "the ruling was hard",
            }],
            "work_queue": [{
                "run_id": "parked-run", "seq": 7,
                "finding": "max_residual_severity: high",
                "reasoning": "the highest residual in the world",
            }],
        })),
    )
    .unwrap();
    // Named or unnamed in the report, the record cites the realm.
    assert_eq!(report.parked_runs[0]["realm"], json!("alpha"));
    assert_eq!(report.work_queue[0]["realm"], json!("alpha"));
    assert_eq!(
        report.citations,
        vec![
            (Some("alpha".to_string()), "parked-run".to_string(), 7),
            (Some("alpha".to_string()), "parked-run".to_string(), 8),
        ]
    );

    let wrong = validate(
        &derived,
        &reported(json!({
            "fleet_summary": "one parked run",
            "parked_runs": [{
                "realm": "beta",
                "run_id": "parked-run", "seq": 8, "command": "stop",
                "reasoning": "in the wrong world",
            }],
            "work_queue": [],
        })),
    )
    .err()
    .expect("a report naming the wrong realm is refused");
    assert!(wrong.contains("in realm 'beta'"), "{wrong}");
    assert!(wrong.contains("realm 'alpha'"), "{wrong}");
}

/// A reader must be able to follow a proposal back to the journal it was
/// read in, so the record spells the realm — and a one-hearth world's
/// record is exactly the record it always was.
#[test]
fn the_record_names_the_realm_only_where_there_was_more_than_one() {
    let many = render(&json!({
        "recorded_at": NOW,
        "fleet_summary": "one parked run in alpha",
        "parked_runs": [{"realm": "alpha", "run_id": "parked-run", "seq": 8,
                         "command": "stop", "reasoning": "hard ruling"}],
        "work_queue": [],
        "citations": [{"realm": "alpha", "run_id": "parked-run", "seq": 8}],
    }));
    assert!(many.contains("parked alpha/parked-run seq 8"), "{many}");
    assert!(many.contains("cites: alpha/parked-run seq 8"), "{many}");

    let one = render(&json!({
        "recorded_at": NOW,
        "fleet_summary": "one parked run",
        "parked_runs": [{"run_id": "parked-run", "seq": 8,
                         "command": "stop", "reasoning": "hard ruling"}],
        "work_queue": [],
        "citations": [{"run_id": "parked-run", "seq": 8}],
    }));
    assert!(one.contains("parked parked-run seq 8"), "{one}");
    assert!(one.contains("cites: parked-run seq 8"), "{one}");
}

/// Ruling 5: journals never merge. Two hearths holding a run under the
/// SAME id are two rows in two realms — never one row, and never one
/// fold across both journals.
#[test]
fn two_hearths_holding_one_run_id_stay_two_runs() {
    let alpha = Fleet::new();
    alpha.completed("shared-id");
    let beta = Fleet::new();
    beta.parked("shared-id");
    let derived = many_hearth_dossier(&alpha, &beta);
    let rows = derived.value["runs"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!(rows[0]["realm"], json!("alpha"));
    assert_eq!(rows[0]["status"], json!("completed"));
    assert_eq!(rows[1]["realm"], json!("beta"));
    assert_eq!(rows[1]["status"], json!("awaiting_operator"));
    // The two sides of a citation agree on which hearth answers for a
    // shared id: the first the map names, for the realm and for the
    // commands alike. Alpha's run is completed, so it admits none.
    assert_eq!(
        derived.realm_of("shared-id", rows[0]["seq"].as_u64().unwrap()),
        Some(Some("alpha"))
    );
    assert_eq!(derived.commands["shared-id"], Vec::<String>::new());
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
                "driver": ["{brokkr}", "driver", "claude", "--"],
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
    assert_ne!(resolved.command[0], "{brokkr}");
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
        vec![
            (None, "parked-run".to_string(), 7),
            (None, "parked-run".to_string(), 8),
        ]
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
