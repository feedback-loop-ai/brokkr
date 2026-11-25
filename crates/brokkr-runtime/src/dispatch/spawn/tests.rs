//! Decision 0110 rulings 3 and 4 in safe code: the binary a dispatcher was
//! started from, checked by its file; the driver's log, opened as the run
//! lock is; and a driver that resumes only a run spawned under its claim.

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

use super::*;
use crate::launch::tests::claimed::{claimed, Claimed};

/// A file at `dir/brokkr` the test stands in for the dispatcher's binary.
fn binary_in(dir: &Path) -> PathBuf {
    let path = dir.join("brokkr");
    std::fs::write(&path, "the first build").unwrap();
    path
}

/// Ruling 4's refusal of `path`, in its words.
fn replaced(path: &Path) -> String {
    format!(
        "dispatch: the brokkr at {} is not the one this dispatcher started from; it claimed \
         nothing after; start the dispatcher again",
        path.display()
    )
}

#[test]
fn the_binary_stands_while_its_path_names_the_file_it_was_started_from() {
    let dir = tempfile::tempdir().unwrap();
    let path = binary_in(dir.path());
    let binary = Binary::started(&Executable(Some(path.clone()))).unwrap();
    assert_eq!(binary.path(), path);
    assert!(binary.stands().is_ok());
    // Rewritten in place, it is still the file it was.
    std::fs::write(&path, "the same inode").unwrap();
    assert!(binary.stands().is_ok());

    // An upgrade renames a new build into place.
    let staged = dir.path().join("brokkr.new");
    std::fs::write(&staged, "the second build").unwrap();
    std::fs::rename(&staged, &path).unwrap();
    let ended = binary.stands().unwrap_err();
    assert!(matches!(&ended, Ending::Replaced(at) if *at == path));
    assert_eq!(ended.to_string(), replaced(&path));

    std::fs::remove_file(&path).unwrap();
    assert!(matches!(binary.stands(), Err(Ending::Replaced(at)) if at == path));
}

#[test]
fn a_binary_that_cannot_be_named_or_read_spawns_nothing() {
    let unnamed = Binary::started(&Executable(None)).unwrap_err();
    assert!(matches!(unnamed, DriverError::Unnamed));
    assert_eq!(
        unnamed.to_string(),
        "dispatch: this process cannot name the brokkr it was started from, so it spawns no \
         driver (decision 0110)"
    );
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("brokkr");
    let unread = Binary::started(&Executable(Some(gone.clone()))).unwrap_err();
    assert!(matches!(&unread, DriverError::Unread(at) if *at == gone));
    assert_eq!(
        unread.to_string(),
        format!(
            "dispatch: the brokkr at {} cannot be read, so this dispatcher spawns no driver \
             (decision 0110)",
            gone.display()
        )
    );
}

/// The log of run `run` beside the journal in `dir`.
fn log_at(dir: &Path, run: &str) -> PathBuf {
    dir.join("forge.db.runs").join(format!("{run}.log"))
}

/// The path a refused log names.
fn refused_log(journal: &Path, run: &str) -> PathBuf {
    match driver_log(journal, run).unwrap_err() {
        DriverError::Log(at) => at,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_driver_log_is_this_users_own_file_of_mode_0600_appended_and_closed_on_exec() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("forge.db");
    let at = log_at(dir.path(), "run-1");
    use std::io::Write as _;
    let mut first = driver_log(&journal, "run-1").unwrap();
    first.write_all(b"one\n").unwrap();
    let mut second = driver_log(&journal, "run-1").unwrap();
    second.write_all(b"two\n").unwrap();
    assert_eq!(std::fs::read_to_string(&at).unwrap(), "one\ntwo\n");
    let read = std::fs::symlink_metadata(&at).unwrap();
    assert!(read.file_type().is_file());
    assert_eq!(read.mode() & 0o777, 0o600);
    assert_eq!(read.uid(), rustix::process::getuid().as_raw());
    assert_eq!(read.nlink(), 1);
    let flags = rustix::io::fcntl_getfd(&second).unwrap();
    assert!(flags.contains(rustix::io::FdFlags::CLOEXEC));
}

#[test]
fn a_driver_log_that_is_not_this_users_own_single_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("forge.db");
    drop(driver_log(&journal, "made").unwrap());

    // Never through a link.
    let elsewhere = dir.path().join("elsewhere");
    std::fs::write(&elsewhere, "").unwrap();
    let linked = log_at(dir.path(), "linked");
    std::os::unix::fs::symlink(&elsewhere, &linked).unwrap();
    assert_eq!(refused_log(&journal, "linked"), linked);

    // Others may read it.
    let open = log_at(dir.path(), "open");
    std::fs::write(&open, "").unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(refused_log(&journal, "open"), open);

    // A second link.
    let twice = log_at(dir.path(), "twice");
    std::fs::hard_link(log_at(dir.path(), "made"), &twice).unwrap();
    assert_eq!(refused_log(&journal, "twice"), twice);

    // A run id that is no plain file name.
    assert_eq!(
        refused_log(&journal, "../escape"),
        log_at(dir.path(), "../escape")
    );
    let refused = DriverError::Log(open.clone());
    assert_eq!(
        refused.to_string(),
        format!(
            "dispatch: the driver log {} is not this user's own single file in a directory of \
             mode 0700, so the run's driver was not started (decision 0110)",
            open.display()
        )
    );
}

#[test]
fn a_driver_log_in_a_runs_directory_others_may_enter_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("forge.db");
    let runs = dir.path().join("forge.db.runs");
    std::fs::create_dir(&runs).unwrap();
    std::fs::set_permissions(&runs, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(refused_log(&journal, "run"), log_at(dir.path(), "run"));
    assert!(!log_at(dir.path(), "run").exists());
}

/// One entry claimed and its run started and linked, its `spawn` row
/// written where `spawned`.
struct Linked {
    entry: EntryId,
    seq: i64,
    request: LaunchRequest,
    run: String,
    journal: PathBuf,
}

fn linked(root: &Path, spawned: bool) -> Linked {
    let Claimed {
        generation,
        claim,
        request,
        run,
        ..
    } = claimed(root, None);
    let journal = request.journal.clone();
    let engine = crate::launch::start(request.clone(), run, &mut |_| {});
    let run = engine.unwrap().run_id.clone();
    let mut store = Store::open(&journal).unwrap();
    store.queue_link(claim, generation, &run, "r").unwrap();
    if spawned {
        store.queue_spawn(claim, generation, "r").unwrap();
    }
    let (entry, seq) = claim.parts();
    Linked {
        entry,
        seq,
        request,
        run,
        journal,
    }
}

/// What `journal` holds that a driver could write: its commands and its
/// events, counted.
fn written(journal: &Path) -> (i64, i64) {
    let conn = rusqlite::Connection::open(journal).unwrap();
    let count = |table: &str| {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    };
    (count("queue_commands"), count("events"))
}

/// Run `script` on `journal`.
fn sql(journal: &Path, script: &str) {
    rusqlite::Connection::open(journal)
        .unwrap()
        .execute_batch(script)
        .unwrap();
}

#[test]
fn a_driver_resumes_the_run_spawned_under_its_claim_rebuilt_as_the_start_rebuilt_it() {
    let dir = tempfile::tempdir().unwrap();
    let linked = linked(dir.path(), true);
    let (journal, entry) = (&linked.journal, linked.entry);
    let host_path = OsString::from("/host/path");
    let (request, run) = driven(journal, entry, linked.seq, host_path.clone()).unwrap();
    assert_eq!(run, linked.run);
    let mut expected = linked.request.clone();
    expected.host_path = host_path;
    // `LaunchRequest` holds no `PartialEq`; its debug form names every field.
    assert_eq!(format!("{request:?}"), format!("{expected:?}"));
    assert_eq!(workspace(journal, entry).unwrap(), linked.request.workspace);
}

#[test]
fn a_driver_of_a_claim_with_no_spawn_row_refuses_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let Linked {
        entry,
        seq,
        journal,
        ..
    } = linked(dir.path(), false);
    let before = written(&journal);
    let refused = driven(&journal, entry, seq, OsString::new()).unwrap_err();
    assert!(matches!(
        refused,
        DriverError::Unspawned { entry: at, claim } if at == entry && claim == seq
    ));
    assert_eq!(
        refused.to_string(),
        format!(
            "queue dispatch drive: queue entry {entry} is not linked under claim {seq} with a `spawn` \
             row, so this driver resumes nothing and wrote nothing (decision 0110)"
        )
    );
    assert_eq!(written(&journal), before, "nothing written");
}

#[test]
fn a_driver_of_a_journal_or_entry_that_does_not_read_is_refused_by_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("absent.db");
    let refused = driven(&journal, EntryId(1), 1, OsString::new()).unwrap_err();
    assert!(matches!(refused, DriverError::Store(StoreError::Sqlite(_))));
    let refused = workspace(&journal, EntryId(1)).unwrap_err();
    assert!(matches!(refused, DriverError::Store(StoreError::Sqlite(_))));

    let Linked {
        entry,
        seq,
        journal,
        ..
    } = linked(dir.path(), true);
    let unknown = workspace(&journal, EntryId(99)).unwrap_err();
    assert!(matches!(
        unknown,
        DriverError::Store(StoreError::Queue(brokkr_store::QueueRefusal::UnknownEntry(
            EntryId(99)
        )))
    ));

    // A payload whose workspace is relative is refused by the rebuild.
    let payload: String = rusqlite::Connection::open(&journal)
        .unwrap()
        .query_row(
            "SELECT payload FROM queue_entries WHERE entry_id = ?1",
            [entry.0],
            |row| row.get(0),
        )
        .unwrap();
    let mut moved: serde_json::Value = serde_json::from_str(&payload).unwrap();
    moved["workspace"] = serde_json::json!("relative/workspace");
    // The store guards a payload; this journal is the test's to break.
    let update = format!(
        "DROP TRIGGER queue_entries_fixed;
         UPDATE queue_entries SET payload = '{moved}' WHERE entry_id = {}",
        entry.0
    );
    sql(&journal, &update);
    let refused = driven(&journal, entry, seq, OsString::new()).unwrap_err();
    assert!(
        matches!(
            refused,
            DriverError::Launch(LaunchError::QueuedWorkspaceRelative(_))
        ),
        "{refused:?}"
    );

    // An entry that does not read, its claim and `spawn` row standing.
    sql(&journal, "UPDATE queue_entries SET payload = '{}'");
    let refused = driven(&journal, entry, seq, OsString::new()).unwrap_err();
    assert!(matches!(
        refused,
        DriverError::Launch(LaunchError::DecodeQueued(_))
    ));
    let refused = workspace(&journal, entry).unwrap_err();
    assert!(matches!(
        refused,
        DriverError::Launch(LaunchError::DecodeQueued(_))
    ));

    // An entry whose own row does not read, though its claim is spawned.
    sql(
        &journal,
        "PRAGMA ignore_check_constraints = ON; UPDATE queue_entries SET state = 'bogus'",
    );
    let refused = driven(&journal, entry, seq, OsString::new()).unwrap_err();
    assert!(matches!(refused, DriverError::Store(_)), "{refused:?}");
}
