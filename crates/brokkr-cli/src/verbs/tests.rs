//! #362 found seven verbs that took `--run` literally, and decision
//! 0015's proposed 2026-09-28 addendum gives every `--run` its selector.
//! One test per verb pins that each now asks `selector::resolve_run`: a
//! verb that resolves names the run `latest` stands for, and on a journal
//! holding no run at all it answers with the resolver's own refusal,
//! matched by its [`Refusal`] variant; `selector`'s tests pin the text.

use std::env::VarError;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brokkr_core::envelope::ChainError;
use brokkr_core::fold::{fold, Status};
use brokkr_core::EventType;
use brokkr_store::{Store, StoreError};
use serde_json::json;

use crate::cli_args::*;
use crate::selector::{refusal_kind, Refusal};
use crate::tests::env_guard::EnvGuard;
use crate::tests::{
    at, broken_chain_store, bundled, cli, running_store, stopped_mid_flight_run, workspace,
};
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

/// A run the selector finds but the store cannot load (a broken chain)
/// is named as the source the rerun could not read.
#[test]
fn rerun_names_a_resolved_run_it_cannot_load() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    broken_chain_store(&db, "broken");
    let rerun = Cmd::Rerun(RerunArgs {
        run: "latest".into(),
        delivery: bundled(workspace().join("recipes/fast")),
        journal: at(&db),
        repo: None,
    });
    let unloaded = run(cli(rerun)).unwrap_err();
    assert_eq!(unloaded.to_string(), "loading source run 'broken'");
    assert_eq!(
        unloaded.root_cause().downcast_ref::<ChainError>(),
        Some(&ChainError::BrokenChain {
            seq: 3,
            prev_seq: 2
        })
    );
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

/// `brokkr operator <command> --run latest` on `journal`, with no
/// supersede-only argument.
fn operator(command: &str, journal: JournalArgs) -> OperatorArgs {
    OperatorArgs {
        run: "latest".into(),
        command: command.into(),
        reason: "requirements changed".into(),
        findings: Vec::new(),
        by_run: None,
        by_seq: None,
        by_realm: None,
        journal,
    }
}

#[test]
fn operator_resolves_latest_for_every_command() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "running-run");
    assert_eq!(
        run(cli(Cmd::Operator(operator("stop", at(&db))))).unwrap(),
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
        ..operator("supersede", at(&empty_journal(empty.path())))
    };
    assert_eq!(refusal(Cmd::Operator(supersede)), Some(Refusal::Empty));
}

/// As at 5ca1e335, bridge reads its credential before it asks for the
/// run: with both wrong, the credential is what it refuses.
#[test]
fn bridge_reads_its_credential_then_resolves_latest() {
    let dir = tempfile::tempdir().unwrap();
    let journal = empty_journal(dir.path());
    let mut env = EnvGuard::lock();
    let token_env = format!("BROKKR_TEST_TOKEN_LATEST_{}", std::process::id());
    env.remove(&token_env);
    let bridge = || {
        Cmd::Bridge(BridgeArgs {
            run: "latest".into(),
            journal: at(&journal),
            looper_url: "http://127.0.0.1:1".into(),
            token_env: token_env.clone(),
            follow: false,
            interval_ms: 0,
        })
    };
    let unset = run(cli(bridge())).unwrap_err();
    assert_eq!(
        unset.root_cause().downcast_ref::<VarError>(),
        Some(&VarError::NotPresent)
    );
    env.set(&token_env, "test-token");
    assert_eq!(refusal(bridge()), Some(Refusal::Empty));
}

/// As at 5ca1e335, supersede opens the journal its citation names before
/// it asks for the run: with both wrong, the journal is what it refuses.
#[test]
fn supersede_opens_the_cited_journal_then_resolves_latest() {
    let world = tempfile::tempdir().unwrap();
    let map = world.path().join("realms.json");
    let realm = |name: &str, journal: &str| {
        json!({
            "name": name, "path": ".", "default_branch": "main", "journal": journal,
        })
    };
    let realms = json!({
        "schema": "forge.realms/v2",
        "realms": [realm("here", "held.db"), realm("next-door", "missing.db")],
        "journal": "held.db",
    });
    std::fs::write(&map, realms.to_string()).unwrap();
    let mapped = || JournalArgs {
        realms: Some(map.clone()),
        db: None,
    };
    let supersede = |by_realm: &str| {
        Cmd::Operator(OperatorArgs {
            findings: vec![6],
            by_run: Some("later".into()),
            by_seq: Some(6),
            by_realm: Some(by_realm.into()),
            ..operator("supersede", mapped())
        })
    };
    let unopened = run(cli(supersede("next-door"))).unwrap_err();
    assert!(
        matches!(
            unopened.downcast_ref::<StoreError>(),
            Some(StoreError::Sqlite(_))
        ),
        "{unopened:#}"
    );
    assert_eq!(refusal(supersede("here")), Some(Refusal::Empty));
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
