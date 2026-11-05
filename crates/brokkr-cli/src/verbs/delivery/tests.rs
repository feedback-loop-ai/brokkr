//! What leaves the delivery verbs when the launch refuses.

use std::process::ExitCode;

use brokkr_runtime::launch::LaunchError;
use brokkr_runtime::{CauseRefusal, Verb};
use brokkr_store::{Store, StoreError};

use super::{operator, rerun, resume, run};
use crate::cli_args::{
    ConcludeArgs, DeliveryArgs, JournalArgs, LaunchArgs, OperatorArgs, RerunArgs, ResumeArgs,
    RunArgs,
};
use crate::run_view::PRODUCTION;
use crate::selector::{refusal_kind, Refusal};
use crate::tests::at;
use crate::{contention, failure_line, report, Exit};

/// Each handler's stderr line when its request trips an earlier refusal
/// AND names a recipe that is not installed: the recipe is resolved
/// inside the launch, where each verb always resolved it, so the line is
/// the earlier refusal's (#350) and no journal is created.
#[test]
fn a_missing_recipe_never_masks_the_refusal_each_verb_met_first() {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path();
    crate::folder::folder_tests::plant_library(ws);
    let map = serde_json::json!({"schema": "forge.realms/v4", "journal": "forge.db",
        "realms": [{"name": "here", "path": ".", "default_branch": "main"}]});
    std::fs::write(ws.join("map.json"), map.to_string()).unwrap();
    let db = ws.join("never-created.db");
    let journal = || JournalArgs {
        realms: None,
        db: Some(db.clone()),
    };
    let line = |refusal: anyhow::Result<ExitCode>| failure_line(&refusal.unwrap_err());
    let selected = |refusal: anyhow::Result<ExitCode>| {
        let error = refusal.unwrap_err();
        (failure_line(&error), refusal_kind(&error))
    };
    let absent = || {
        (
            "error: no run matching 'absent' in this workspace database".to_string(),
            Some(Refusal::Missing),
        )
    };
    let missing = || DeliveryArgs {
        bundle: None,
        recipe: Some("missing".to_string()),
        recipes_dir: Some(ws.join("recipes")),
        secrets_file: None,
    };

    let dispatched = run(
        ws,
        RunArgs {
            entry: Default::default(),
            launch: LaunchArgs {
                delivery: missing(),
                feature: "f".into(),
                realms: Some(ws.join("map.json")),
                db: Some(db.clone()),
                repo: None,
                dispatch: Some(ws.join("d.json")),
            },
            no_view: true,
        },
        &PRODUCTION,
    );
    assert_eq!(
        line(dispatched),
        format!("error: {}", LaunchError::RealmsWithDispatch)
    );
    assert!(!db.exists());

    // A source run, like a resumed one, is looked for in the journal
    // before its bundle is named, as it always was: its `--run` is
    // resolved (decision 0015) before the launch is asked.
    let reran = rerun(
        ws,
        RerunArgs {
            run: "absent".into(),
            delivery: missing(),
            journal: journal(),
            repo: None,
        },
    );
    assert_eq!(selected(reran), absent());
    let resumed = resume(
        ws,
        ResumeArgs {
            delivery: missing(),
            run: "absent".into(),
            journal: journal(),
            repo: None,
            no_view: true,
        },
        &PRODUCTION,
    );
    assert_eq!(selected(resumed), absent());
}

/// A launch that met a peer's lock on the journal — opening it, or
/// through the engine it started — leaves with the contended exit, found
/// through the launch's own door; anything else it refuses keeps exit 1.
#[test]
fn a_contended_launch_leaves_with_the_contended_exit() {
    let contended = || StoreError::Contended {
        operation: "open",
        waited_ms: 30_000,
    };
    let opened = anyhow::Error::from(LaunchError::Store(contended()));
    assert!(contention(&opened).unwrap().is_contention());
    assert_eq!(report(&opened), ExitCode::from(Exit::Contended));
    let started = anyhow::Error::from(LaunchError::Engine(brokkr_runtime::EngineError::Store(
        contended(),
    )));
    assert_eq!(report(&started), ExitCode::from(Exit::Contended));
    let refused = anyhow::Error::from(LaunchError::RealmsWithDispatch);
    assert!(contention(&refused).is_none());
    assert_eq!(report(&refused), ExitCode::from(Exit::Failed));
}

/// A run an operator stopped without naming a cause, which `classify`
/// exists to name.
fn operator_stopped(db: &std::path::Path, run_id: &str) {
    crate::tests::running_store(db, run_id);
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .append_next(
            run_id,
            brokkr_core::EventType::RunStopped,
            serde_json::json!({"reason": "operator 'operator' commanded stop: enough"}),
            None,
            None,
        )
        .unwrap();
}

/// A world of two hearths in `dir`: `here`, whose journal is `forge.db`,
/// and `next-door`, whose journal holds the run `later`, still going. Its
/// map is written as `realms.json`, and returned.
fn next_door(dir: &std::path::Path) -> std::path::PathBuf {
    crate::tests::running_store(&dir.join("next-door.db"), "later");
    let map = dir.join("realms.json");
    std::fs::write(
        &map,
        serde_json::json!({
            "schema": "forge.realms/v2",
            "realms": [
                {"name": "here", "path": ".", "default_branch": "main", "journal": "forge.db"},
                {"name": "next-door", "path": ".", "default_branch": "main", "journal": "next-door.db"},
            ],
            "journal": "forge.db",
        })
        .to_string(),
    )
    .unwrap();
    map
}

/// A refusal for the missing `--cause` on `verb`: the runtime's own usage
/// text, which names the six, by its variant.
fn assert_missing_cause(refused: &anyhow::Error, verb: Verb) {
    assert_eq!(
        failure_line(refused),
        format!(
            "error: {}",
            brokkr_runtime::named_cause(verb, None).unwrap_err()
        )
    );
    assert!(matches!(
        refused.downcast_ref(),
        Some(CauseRefusal::Missing { verb: missing }) if *missing == verb
    ));
}

/// `brokkr operator classify` as a test types it, varying only the
/// citation under test.
fn classifying(
    db: &std::path::Path,
    cause: Option<&str>,
    by_run: Option<&str>,
    by_realm: Option<&str>,
    by_ref: Option<&str>,
) -> OperatorArgs {
    OperatorArgs {
        member: crate::cli_args::MemberRetryArgs::default(),
        run: "held".into(),
        command: "classify".into(),
        reason: "the operator named the ending".into(),
        cause: cause.map(str::to_string),
        findings: Vec::new(),
        by_run: by_run.map(str::to_string),
        by_seq: None,
        by_realm: by_realm.map(str::to_string),
        by_ref: by_ref.map(str::to_string),
        journal: at(db),
    }
}

/// Ruling 5: the verb writes one `operator/commanded` with
/// `command: "classify"`; a second stands over the first; and a live run
/// is refused with nothing written.
#[test]
fn the_classify_verb_records_the_latest_annotation_and_refuses_a_live_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    operator_stopped(&db, "held");
    crate::tests::running_store(&db, "later");
    crate::tests::running_store(&db, "going");
    assert_eq!(
        operator(
            dir.path(),
            classifying(&db, Some("superseded"), Some("later"), None, None)
        )
        .unwrap(),
        ExitCode::SUCCESS
    );
    assert_eq!(
        operator(
            dir.path(),
            classifying(&db, Some("withdrawn"), None, None, None)
        )
        .unwrap(),
        ExitCode::SUCCESS
    );
    let events = brokkr_store::Store::open(&db)
        .unwrap()
        .load("held")
        .unwrap();
    let state = brokkr_core::fold(&events).unwrap();
    assert_eq!(
        brokkr_core::ending::ended(&events, &state),
        Some(brokkr_core::ending::Ended {
            ending: brokkr_core::ending::Ending::Withdrawn,
            cited: None,
        }),
        "the later classify stands over the earlier one"
    );
    let classify = |events: &[brokkr_core::EventEnvelope]| {
        events
            .iter()
            .filter(|event| {
                event.event_type == brokkr_core::EventType::OperatorCommanded
                    && event.payload["command"] == "classify"
            })
            .count()
    };
    assert_eq!(classify(&events), 2);
    let live = Store::open(&db).unwrap().load("going").unwrap().len();
    let refused = operator(
        dir.path(),
        OperatorArgs {
            run: "going".into(),
            ..classifying(&db, Some("withdrawn"), None, None, None)
        },
    )
    .unwrap_err();
    assert_eq!(
        failure_line(&refused),
        "error: classify refused, nothing was written: run 'going' is running; classify names a \
         stopped run's ending"
    );
    assert_eq!(
        Store::open(&db).unwrap().load("going").unwrap().len(),
        live,
        "the refused classify wrote nothing"
    );
}

/// Ruling 6: a completed run's ending is its last review's, so classify
/// refuses it rather than journaling an annotation the reading ignores.
#[test]
fn the_classify_verb_refuses_a_completed_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::running_store(&db, "done");
    {
        let mut store = Store::open(&db).unwrap();
        store
            .append_next(
                "done",
                brokkr_core::EventType::RunCompleted,
                serde_json::json!({}),
                None,
                None,
            )
            .unwrap();
    }
    let before = Store::open(&db).unwrap().load("done").unwrap().len();
    let refused = operator(
        dir.path(),
        OperatorArgs {
            run: "done".into(),
            ..classifying(&db, Some("withdrawn"), None, None, None)
        },
    )
    .unwrap_err();
    assert_eq!(
        failure_line(&refused),
        "error: classify refused, nothing was written: run 'done' is completed; classify names a \
         stopped run's ending"
    );
    assert_eq!(
        Store::open(&db).unwrap().load("done").unwrap().len(),
        before,
        "the refused classify wrote nothing"
    );
}

/// Ruling 5's syntax and choice 7's shape: a missing cause is a usage
/// refusal, `--by-ref` rides with `delivered-elsewhere`, and `--by-realm`
/// reads the realm's own journal.
#[test]
fn the_classify_verb_names_its_cause_and_reads_the_cited_realm() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    operator_stopped(&db, "held");
    let missing = operator(dir.path(), classifying(&db, None, None, None, None)).unwrap_err();
    assert_missing_cause(&missing, Verb::Classify);
    // The cause is parsed where it was typed, before any journal is read:
    // a word outside the vocabulary is refused even for a run no journal
    // holds, in the engine's one text for it.
    let unknown = OperatorArgs {
        run: "no-such-run".into(),
        ..classifying(&db, Some("not-a-word"), None, None, None)
    };
    assert_eq!(
        failure_line(&operator(dir.path(), unknown).unwrap_err()),
        format!(
            "error: {}",
            brokkr_runtime::named_cause(Verb::Classify, Some("not-a-word")).unwrap_err()
        )
    );
    assert_eq!(
        operator(
            dir.path(),
            classifying(&db, Some("delivered-elsewhere"), None, None, Some("abc123"))
        )
        .unwrap(),
        ExitCode::SUCCESS
    );
    let written = brokkr_store::Store::open(&db)
        .unwrap()
        .load("held")
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        written.payload["args"],
        serde_json::json!({"cause": "delivered-elsewhere", "ref": "abc123",
                           "reason": "the operator named the ending"})
    );

    // A citation into another hearth opens that hearth's journal, and an
    // unknown realm is refused before anything is written.
    let map = next_door(dir.path());
    let mapped = |realm: &str| OperatorArgs {
        cause: Some("superseded".into()),
        by_run: Some("later".into()),
        by_realm: Some(realm.into()),
        journal: crate::cli_args::JournalArgs {
            realms: Some(map.clone()),
            db: None,
        },
        ..classifying(&db, Some("superseded"), Some("later"), None, None)
    };
    assert_eq!(
        operator(dir.path(), mapped("next-door")).unwrap(),
        ExitCode::SUCCESS
    );
    let cited = brokkr_store::Store::open(&db)
        .unwrap()
        .load("held")
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        cited.payload["args"]["by"],
        serde_json::json!({"realm": "next-door", "run_id": "later"})
    );
    let absent = operator(dir.path(), mapped("absent")).unwrap_err();
    assert_eq!(
        failure_line(&absent),
        "error: --by-realm 'absent' names no realm in this world; --realms names the map \
         that would"
    );
}

/// The operator subcommand keeps its flags on their own verbs: `classify`
/// refuses the supersede-only pair, and `supersede` refuses the classify
/// pair.
#[test]
fn classify_and_supersede_refuse_each_others_flags() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    operator_stopped(&db, "held");
    let mut args = classifying(&db, Some("withdrawn"), None, None, None);
    args.by_seq = Some(2);
    assert_eq!(
        failure_line(&operator(dir.path(), args).unwrap_err()),
        "error: --findings and --by-seq belong to 'supersede'; 'classify' takes --run, \
         --cause, --reason, --by-run, --by-realm, --by-ref, --realms and --db"
    );
    let mut findings = classifying(&db, Some("withdrawn"), None, None, None);
    findings.findings = vec![2];
    assert_eq!(
        failure_line(&operator(dir.path(), findings).unwrap_err()),
        "error: --findings and --by-seq belong to 'supersede'; 'classify' takes --run, \
         --cause, --reason, --by-run, --by-realm, --by-ref, --realms and --db"
    );
    let supersede = OperatorArgs {
        command: "supersede".into(),
        findings: vec![2],
        by_run: Some("later".into()),
        by_seq: Some(2),
        cause: Some("superseded".into()),
        ..classifying(&db, Some("withdrawn"), None, None, None)
    };
    let supersede_only = "error: --cause and --by-ref belong to 'stop' and 'classify'";
    assert_eq!(
        failure_line(&operator(dir.path(), supersede).unwrap_err()),
        supersede_only
    );
    let supersede_ref = OperatorArgs {
        command: "supersede".into(),
        findings: vec![2],
        by_run: Some("later".into()),
        by_seq: Some(2),
        cause: None,
        ..classifying(&db, Some("withdrawn"), None, None, Some("abc123"))
    };
    assert_eq!(
        failure_line(&operator(dir.path(), supersede_ref).unwrap_err()),
        supersede_only
    );
    // `stop` takes the cause and its citation as `classify` does, and
    // refuses supersede's pair; a verb that ends nothing refuses the cause.
    let stop = OperatorArgs {
        command: "stop".into(),
        by_seq: Some(2),
        ..classifying(&db, Some("withdrawn"), None, None, None)
    };
    assert_eq!(
        failure_line(&operator(dir.path(), stop).unwrap_err()),
        "error: --findings and --by-seq belong to 'supersede'; 'stop' takes --run, --cause, \
         --reason, --by-run, --by-realm, --by-ref, --realms and --db"
    );
    for (cause, by_ref) in [(Some("withdrawn"), None), (None, Some("abc123"))] {
        let release = OperatorArgs {
            command: "release".into(),
            ..classifying(&db, cause, None, None, by_ref)
        };
        assert_eq!(
            failure_line(&operator(dir.path(), release).unwrap_err()),
            format!("{supersede_only}; 'release' takes --run, --reason, --realms and --db")
        );
    }
}

/// `brokkr operator stop` and `brokkr conclude` as a test types them on
/// `run`, with the cause and its `--by-run`, `--by-realm` and `--by-ref`.
fn ending_args(
    db: &std::path::Path,
    run: &str,
    cause: Option<&str>,
    [by_run, by_realm, by_ref]: [Option<&str>; 3],
) -> (OperatorArgs, ConcludeArgs) {
    let stop = OperatorArgs {
        command: "stop".into(),
        run: run.into(),
        ..classifying(db, cause, by_run, by_realm, by_ref)
    };
    let conclude = ConcludeArgs {
        run: run.into(),
        reason: stop.reason.clone(),
        cause: stop.cause.clone(),
        by_run: stop.by_run.clone(),
        by_realm: stop.by_realm.clone(),
        by_ref: stop.by_ref.clone(),
        journal: at(db),
    };
    (stop, conclude)
}

/// Decision 0064 ruling 4: `--cause` is required on `stop` and on
/// `conclude`. A missing one is the usage refusal that names the six, a
/// cause that does not fit its citation is the runtime's typed refusal,
/// and neither writes anything.
#[test]
fn stop_and_conclude_refuse_a_missing_or_misfit_cause_and_write_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::running_store(&db, "going");
    let journal = || Store::open(&db).unwrap().load("going").unwrap();
    let before = journal();
    let (stop, conclude) = ending_args(&db, "going", None, [None; 3]);
    assert_missing_cause(&operator(dir.path(), stop).unwrap_err(), Verb::Stop);
    assert_missing_cause(
        &super::conclude(dir.path(), conclude).unwrap_err(),
        Verb::Conclude,
    );
    let (stop, conclude) = ending_args(&db, "going", Some("superseded"), [None; 3]);
    for (refused, verb) in [
        (operator(dir.path(), stop).unwrap_err(), Verb::Stop),
        (
            super::conclude(dir.path(), conclude).unwrap_err(),
            Verb::Conclude,
        ),
    ] {
        let Some(brokkr_runtime::EngineError::CauseRefused(cause)) = refused.downcast_ref() else {
            panic!("{verb} refused with another error: {refused}");
        };
        assert!(
            matches!(**cause, CauseRefusal::NoReplacement { verb: refusing } if refusing == verb)
        );
    }
    let after = journal();
    assert_eq!(after.len(), before.len(), "nothing was written");
}

/// Ruling 4: `stop` writes its cause in the command's `args`, a citation
/// into another hearth read in that hearth's journal; `conclude` on that
/// stop under another cause appends one `classify` (choice 4 (a)); and a
/// conclusion that commands its own stop carries its cause there.
#[test]
fn stop_and_conclude_write_the_cause_they_name() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    crate::tests::running_store(&db, "held");
    crate::tests::running_store(&db, "landed");
    let map = next_door(dir.path());
    let cited = [Some("later"), Some("next-door"), None];
    let (mut stop, _) = ending_args(&db, "held", Some("superseded"), cited);
    stop.journal = crate::cli_args::JournalArgs {
        realms: Some(map),
        db: None,
    };
    assert_eq!(operator(dir.path(), stop).unwrap(), ExitCode::SUCCESS);
    let last_commanded = |run: &str| {
        let events = Store::open(&db).unwrap().load(run).unwrap();
        let commanded = events
            .iter()
            .rev()
            .find(|event| event.event_type == brokkr_core::EventType::OperatorCommanded);
        (
            commanded.unwrap().payload.clone(),
            events.last().unwrap().event_type,
        )
    };
    let (stopped, _) = last_commanded("held");
    assert_eq!(
        (&stopped["command"], &stopped["args"]),
        (
            &serde_json::json!("stop"),
            &serde_json::json!({"cause": "superseded",
                                "by": {"realm": "next-door", "run_id": "later"}})
        )
    );

    let (_, withdrawn) = ending_args(&db, "held", Some("withdrawn"), [None; 3]);
    let stopped_exit = ExitCode::from(Exit::Stopped);
    assert_eq!(
        super::conclude(dir.path(), withdrawn).unwrap(),
        stopped_exit
    );
    let (classified, last) = last_commanded("held");
    assert_eq!(last, brokkr_core::EventType::OperatorCommanded);
    assert_eq!(classified["command"], "classify");
    assert_eq!(classified["args"]["cause"], "withdrawn");

    let by_ref = [None, None, Some("abc123")];
    let (_, landed) = ending_args(&db, "landed", Some("delivered-elsewhere"), by_ref);
    assert_eq!(super::conclude(dir.path(), landed).unwrap(), stopped_exit);
    let (commanded, last) = last_commanded("landed");
    assert_eq!(last, brokkr_core::EventType::RunStopped, "no classify");
    assert_eq!(
        commanded["args"],
        serde_json::json!({"cause": "delivered-elsewhere", "ref": "abc123"})
    );
}
