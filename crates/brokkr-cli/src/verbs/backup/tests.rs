//! `brokkr backup` and `brokkr restore` through their handlers (decision
//! 0100, the journal half of #431): a backup written from the journal the
//! house's rule resolves, an inventory this build reads, a restore that
//! keeps every run `inspect` reads and starts nothing, and each refusal by
//! name.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::{backup, restore, BackupArgs, RestoreArgs};
use crate::cli_args::{InspectArgs, JournalArgs};
use crate::verbs::readouts;

fn at(db: &Path) -> JournalArgs {
    JournalArgs {
        realms: None,
        db: Some(db.to_path_buf()),
    }
}

/// The repo's fixture journals, read and never regenerated.
fn fixture(name: &str) -> (String, PathBuf, serde_json::Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let journal = root
        .join("fixtures/journals")
        .join(format!("{name}.ndjson"));
    let ndjson = std::fs::read_to_string(&journal).unwrap();
    let manifest = serde_json::from_str(
        &std::fs::read_to_string(journal.with_extension("manifest.json")).unwrap(),
    )
    .unwrap();
    (ndjson, journal, manifest)
}

/// A source journal holding a completed run, a parked run and one still in
/// flight, every projection caught up.
fn source(dir: &Path) -> PathBuf {
    let db = dir.join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("in-flight", "feature", "test", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "in-flight",
            EventType::RunStarted,
            json!({"feature": "feature", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "in-flight",
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            None,
            None,
        )
        .unwrap();
    drop(store);
    let mut store = Store::open(&db).unwrap();
    for name in [
        "conclude-already-done-hand-built",
        "conclude-parked-hand-built",
    ] {
        let (ndjson, origin, manifest) = fixture(name);
        store.import_run(&ndjson, &manifest, &origin).unwrap();
    }
    db
}

/// `inspect --json`'s exact bytes for `run` in `db`.
fn inspected(workspace: &Path, db: &Path, run: &str) -> String {
    let mut out = Vec::new();
    let args = InspectArgs {
        run: run.into(),
        realms: None,
        db: Some(db.to_path_buf()),
        json: true,
        phase: None,
        seat: None,
        capability_call: None,
        repo: None,
    };
    readouts::inspect(workspace, args, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// Every regular file under `dir`, in name order.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    found
}

fn sha256(path: &Path) -> [u8; 32] {
    Sha256::digest(std::fs::read(path).unwrap()).into()
}

#[test]
fn a_backup_and_restore_keep_what_inspect_reads_and_start_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let db = source(dir.path());
    let backup_dir = dir.path().join("backup");
    backup(
        &workspace,
        BackupArgs {
            to: backup_dir.clone(),
            journal: at(&db),
        },
    )
    .unwrap();
    let inventory: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(backup_dir.join("inventory.json")).unwrap())
            .unwrap();
    assert_eq!(
        inventory["format"],
        brokkr_store::backup::JOURNAL_BACKUP_FORMAT
    );
    assert_eq!(inventory["schemas"]["database"], 1);
    assert_eq!(inventory["schemas"]["queue"], 3);
    assert_eq!(
        inventory["schemas"]["projection"],
        brokkr_view::VIEW_VERSION as u64
    );
    assert_eq!(inventory["boundary"]["runs"], 3);
    assert!(inventory["snapshot"]["sha256"].as_str().unwrap().len() == 64);

    let restored_dir = dir.path().join("restored");
    std::fs::create_dir(&restored_dir).unwrap();
    let restored = restored_dir.join("journal.db");
    restore(RestoreArgs {
        from: backup_dir.clone(),
        to: restored.clone(),
    })
    .unwrap();
    // A completed run, a parked one and one still in flight, each read
    // from both journals.
    let runs = [
        ("conclude-already-done-hand-built", "completed"),
        ("conclude-parked-hand-built", "awaiting_operator"),
        ("in-flight", "running"),
    ];
    for (run, state) in runs {
        let before = inspected(&workspace, &db, run);
        let after = inspected(&workspace, &restored, run);
        let before: serde_json::Value = serde_json::from_str(&before).unwrap();
        let after: serde_json::Value = serde_json::from_str(&after).unwrap();
        assert_eq!(
            before["summary"]["status"], state,
            "the fixture for {run} is not {state}"
        );
        assert_eq!(before, after, "inspect changed for {run}");
    }
    // Nothing launched: the restored journal is the only file beside it,
    // and reading it wrote nothing beside it either.
    assert_eq!(files_under(&restored_dir), vec![restored.clone()]);
    let bytes = sha256(&restored);
    for (run, _) in runs {
        inspected(&workspace, &restored, run);
    }
    assert_eq!(
        sha256(&restored),
        bytes,
        "inspect wrote to the restored journal"
    );
    // The advertised boundary is exactly what was restored.
    let restored_store = Store::open_read_only(&restored).unwrap();
    let mut events = 0usize;
    for (run, _) in runs {
        events += restored_store.load(run).unwrap().len();
    }
    assert_eq!(inventory["boundary"]["events"], events as u64);
}
