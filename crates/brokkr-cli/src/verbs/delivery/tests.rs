//! What leaves the delivery verbs when the launch refuses.

use std::process::ExitCode;

use brokkr_runtime::launch::LaunchError;
use brokkr_store::StoreError;

use super::{rerun, resume, run};
use crate::cli_args::{DeliveryArgs, JournalArgs, RerunArgs, ResumeArgs, RunArgs};
use crate::selector::{refusal_kind, Refusal};
use crate::{contention, failure_line, report, Exit};

/// Each handler's stderr line when its request trips an earlier refusal
/// AND names a recipe that is not installed: the recipe is resolved
/// inside the launch, where each verb always resolved it, so the line is
/// the earlier refusal's (#350) and no journal is created.
#[test]
fn a_missing_recipe_never_masks_the_refusal_each_verb_met_first() {
    let dir = tempfile::tempdir().unwrap();
    let ws = dir.path();
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
        recipes_dir: ws.join("recipes"),
        secrets_file: None,
    };

    let dispatched = run(
        ws,
        RunArgs {
            delivery: missing(),
            feature: "f".into(),
            realms: Some(ws.join("map.json")),
            db: Some(db.clone()),
            repo: None,
            dispatch: Some(ws.join("d.json")),
        },
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
        },
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
