//! #362 found seven verbs that took `--run` literally, and decision
//! 0015's proposed 2026-09-28 addendum gives every `--run` its selector.
//! One test per verb pins that each now asks `selector::resolve_run`: a
//! verb that resolves names the run `latest` stands for, and on a journal
//! holding no run at all it answers with the resolver's own refusal,
//! matched by its [`Refusal`] variant; `selector`'s tests pin the text.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brokkr_core::fold::{fold, Status};
use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::json;

use crate::cli_args::*;
use crate::selector::{refusal_kind, Refusal};
use crate::tests::{at, bundled, cli, running_store, stopped_mid_flight_run, workspace};
use crate::{run, Cmd, Exit};

/// A journal that exists and holds no run.
fn empty_journal(dir: &Path) -> PathBuf {
    let db = dir.join("forge.db");
    Store::open(&db).unwrap();
    db
}

/// Which selector refusal a verb answers with, if any.
fn refusal(command: Cmd) -> Option<Refusal> {
    refusal_kind(&run(cli(command)).unwrap_err())
}

#[test]
fn costs_resolves_latest() {
    let dir = tempfile::tempdir().unwrap();
    let journal = at(&empty_journal(dir.path()));
    let costs = Cmd::Costs(CostsArgs {
        run: "latest".into(),
        journal,
    });
    assert_eq!(refusal(costs), Some(Refusal::Empty));
}

#[test]
fn resume_resolves_latest() {
    let dir = tempfile::tempdir().unwrap();
    let resume = Cmd::Resume(ResumeArgs {
        delivery: bundled(workspace().join("recipes/fast")),
        run: "latest".into(),
        journal: at(&empty_journal(dir.path())),
        repo: None,
    });
    assert_eq!(refusal(resume), Some(Refusal::Empty));
}

#[test]
fn rerun_resolves_latest() {
    let dir = tempfile::tempdir().unwrap();
    let rerun = Cmd::Rerun(RerunArgs {
        run: "latest".into(),
        delivery: DeliveryArgs {
            bundle: None,
            recipe: Some("fast".into()),
            ..bundled(PathBuf::new())
        },
        journal: at(&empty_journal(dir.path())),
        repo: None,
    });
    assert_eq!(refusal(rerun), Some(Refusal::Empty));
}

#[test]
fn conclude_resolves_latest_to_the_run_it_stops_and_refuses_an_empty_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    stopped_mid_flight_run(&db, "stranded", &json!({"engine": "0.3.6", "files": {}}));
    let events = || Store::open(&db).unwrap().load("stranded").unwrap();
    let conclude = |run: &str| {
        Cmd::Conclude(ConcludeArgs {
            run: run.into(),
            reason: "the engine moved on without it".into(),
            journal: at(&db),
        })
    };
    // `--run "$RUN"` with RUN unset names no run, not the sole one.
    let journaled = events().len();
    assert_eq!(refusal(conclude("")), Some(Refusal::Blank));
    assert_eq!(events().len(), journaled);
    assert_eq!(
        run(cli(conclude("latest"))).unwrap(),
        ExitCode::from(Exit::Stopped)
    );
    assert_eq!(fold(&events()).unwrap().status, Status::Stopped);
}

#[test]
fn operator_resolves_latest_for_every_command() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "running-run");
    let operator = |command: &str, db: &Path| OperatorArgs {
        run: "latest".into(),
        command: command.into(),
        reason: "requirements changed".into(),
        findings: Vec::new(),
        by_run: None,
        by_seq: None,
        by_realm: None,
        journal: at(db),
    };
    assert_eq!(
        run(cli(Cmd::Operator(operator("stop", &db)))).unwrap(),
        ExitCode::from(Exit::Completed)
    );
    let events = Store::open(&db).unwrap().load("running-run").unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::OperatorAccepted
    );

    let empty = tempfile::tempdir().unwrap();
    let supersede = OperatorArgs {
        findings: vec![7],
        by_run: Some("other".into()),
        by_seq: Some(9),
        ..operator("supersede", &empty_journal(empty.path()))
    };
    assert_eq!(refusal(Cmd::Operator(supersede)), Some(Refusal::Empty));
}

#[test]
fn bridge_resolves_latest() {
    let dir = tempfile::tempdir().unwrap();
    let bridge = Cmd::Bridge(BridgeArgs {
        run: "latest".into(),
        journal: at(&empty_journal(dir.path())),
        looper_url: "http://127.0.0.1:1".into(),
        token_env: "BROKKR_TEST_TOKEN_NEVER_SET".into(),
        follow: false,
        interval_ms: 0,
    });
    assert_eq!(refusal(bridge), Some(Refusal::Empty));
}

#[test]
fn compare_resolves_both_runs() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "first-run");
    let compare = |run_a: &str, run_b: &str| {
        Cmd::Compare(CompareArgs {
            run_a: run_a.into(),
            run_b: run_b.into(),
            journal: at(&db),
        })
    };
    assert_eq!(
        run(cli(compare("latest", "first"))).unwrap(),
        ExitCode::from(Exit::Completed)
    );
    assert_eq!(
        refusal(compare("first-run", "second")),
        Some(Refusal::Missing)
    );
}
