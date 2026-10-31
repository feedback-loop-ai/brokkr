//! The backup's edges (decision 0100, #431): what a refusal reports when
//! the place it reads or writes is not what the inventory promised, a
//! destination that appears in the race, a directory the host cannot sync,
//! and a snapshot whose tables are not the journal's shape. Each is refused
//! by name or as the I/O or SQLite error it is, and none leaves a byte
//! behind.

use super::*;

/// `store`'s backup into `to`, refused.
fn refused_backup_into(store: &Store, to: &Path) -> BackupError {
    backup(
        store,
        BackupOptions {
            to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
    )
    .unwrap_err()
}

/// A one-run, two-event journal in `dir`, and the backup place beside it.
fn seeded(dir: &Path) -> (Store, PathBuf) {
    let mut store = journal(dir);
    seed(&mut store, "alpha", 2);
    (store, dir.join("backup"))
}

/// A completed backup whose inventory `edit` changes, whose restore
/// refused and left its destination absent.
fn refused_inventory(edit: impl FnOnce(&mut Inventory)) -> BackupError {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let mut inventory = inventory_of(&taken);
    edit(&mut inventory);
    rewrite_inventory(&taken, &inventory);
    let destination = absent_destination(dir.path());
    let refused = restore(&taken.to, &destination).unwrap_err();
    assert_absent(&destination);
    refused
}

/// The SQLite error a completed backup's restore reports once `shape`
/// rewrote its snapshot's tables (with one unowned event planted first
/// when `ghost`), the destination left absent.
fn refused_shape(shape: &str, ghost: bool) -> String {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let refused = refused_edited_restore(dir.path(), &taken, |connection| {
        if ghost {
            plant_unowned(connection);
        }
        connection.execute_batch(shape).unwrap();
    });
    let BackupError::Sqlite(error) = refused else {
        panic!("a snapshot not of the journal's shape is refused as SQLite's error: {refused:?}");
    };
    error.to_string()
}

/// Every name under `dir` that a restore's temporary would carry.
fn temporaries_in(dir: &Path) -> Vec<PathBuf> {
    files_under(dir)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(".brokkr-restore-"))
        })
        .collect()
}

#[test]
fn a_backup_into_an_existing_empty_directory_is_taken() {
    let dir = tempfile::tempdir().unwrap();
    let (store, to) = seeded(dir.path());
    std::fs::create_dir(&to).unwrap();
    let taken = snapshot_of(&store, &to);
    assert_eq!((taken.runs, taken.events), (1, 2));
    let mut names: Vec<_> = files_under(&to)
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, [INVENTORY_FILE, SNAPSHOT_FILE]);
}

#[cfg(unix)]
#[test]
fn a_place_under_a_file_is_refused_as_its_io_error_by_backup_and_restore() {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = completed(dir.path());
    let file = dir.path().join("a-file");
    std::fs::write(&file, "operator bytes").unwrap();

    let refused = refused_backup_into(&store, &file.join("backup"));
    let BackupError::Io(error) = refused else {
        panic!("a backup under a file is refused as its I/O error: {refused:?}");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::NotADirectory);

    // The destination is asked before anything else is read: this backup
    // directory holds no inventory, and that is not what is reported.
    let unread = dir.path().join("unread");
    std::fs::create_dir(&unread).unwrap();
    let refused = restore(&unread, &file.join("restored.db")).unwrap_err();
    let BackupError::Io(error) = refused else {
        panic!("a restore under a file is refused as its I/O error: {refused:?}");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::NotADirectory);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "operator bytes");
}

#[cfg(unix)]
#[test]
fn a_snapshot_that_cannot_be_read_is_refused_as_its_io_error_and_not_as_missing() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let snapshot = taken.to.join(SNAPSHOT_FILE);
    std::fs::remove_file(&snapshot).unwrap();
    // A link to itself: present, and never readable.
    std::os::unix::fs::symlink(SNAPSHOT_FILE, &snapshot).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&taken.to, &destination).unwrap_err();
    let BackupError::Io(error) = refused else {
        panic!("a snapshot that cannot be read is refused as its I/O error: {refused:?}");
    };
    assert!(
        error
            .to_string()
            .starts_with("Too many levels of symbolic links"),
        "{error}"
    );
    assert_absent(&destination);
}

#[test]
fn a_destination_that_appears_before_the_publish_is_refused_by_its_name() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let destination = absent_destination(dir.path());
    let refused = restore_through(&taken.to, &destination, &mut |_, _| {
        Err(std::io::ErrorKind::AlreadyExists.into())
    })
    .unwrap_err();
    let BackupError::RestoreDestinationExists(found) = refused else {
        panic!("a destination that appeared in the race is refused by name: {refused:?}");
    };
    assert_eq!(found, destination);
    assert_absent(&destination);
    assert_eq!(temporaries_in(dir.path()), Vec::<PathBuf>::new());

    let refused = restore_through(&taken.to, &destination, &mut |_, _| {
        Err(std::io::Error::other("the disk went away"))
    })
    .unwrap_err();
    let BackupError::Io(error) = refused else {
        panic!("any other publish failure is its I/O error: {refused:?}");
    };
    assert_eq!(error.to_string(), "the disk went away");
    assert_absent(&destination);
    assert_eq!(temporaries_in(dir.path()), Vec::<PathBuf>::new());
}

#[test]
fn a_snapshot_name_that_appears_before_the_publish_refuses_the_backup_as_not_empty() {
    let dir = tempfile::tempdir().unwrap();
    let (store, to) = seeded(dir.path());
    let refused = backup_through(
        &store,
        BackupOptions {
            to: &to,
            now: TAKEN_AT,
            projection: PROJECTION,
        },
        &mut |_, _| Err(std::io::ErrorKind::AlreadyExists.into()),
    )
    .unwrap_err();
    let BackupError::BackupDestinationNotEmpty(found) = refused else {
        panic!("a snapshot name that appeared in the race is refused by name: {refused:?}");
    };
    assert_eq!(found, to);
    assert_eq!(files_under(&to), Vec::<PathBuf>::new());
}

#[cfg(unix)]
#[test]
fn a_restore_whose_directory_cannot_be_synced_removes_what_it_published() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    // Written and searched, never listed: the publish lands, and opening
    // the directory to sync it is refused.
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o300)).unwrap();
    let destination = locked.join("restored.db");
    let refused = restore(&taken.to, &destination);
    let absent = !destination.exists();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
    let refused = refused.unwrap_err();
    let BackupError::Io(error) = refused else {
        panic!("a directory that cannot be synced is its I/O error: {refused:?}");
    };
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(absent, "a refused restore left {}", destination.display());
    assert_eq!(files_under(&locked), Vec::<PathBuf>::new());
}

#[cfg(target_os = "linux")]
#[test]
fn a_directory_whose_filesystem_cannot_sync_is_not_a_refusal() {
    // procfs answers a directory fsync with EINVAL, as macOS answers every
    // one: the name already landed, and there is no stronger call to make.
    assert!(sync_dir(Path::new("/proc")).is_ok());
}

#[test]
fn a_bare_file_name_lives_in_the_current_directory() {
    assert_eq!(parent_of(Path::new("journal.db")), Path::new("."));
    assert_eq!(parent_of(Path::new("dir/journal.db")), Path::new("dir"));
}

#[test]
fn a_run_count_that_disagrees_with_the_boundary_is_refused_by_name() {
    let refused = refused_inventory(|inventory| inventory.boundary.runs += 1);
    let BackupError::CountsMismatch {
        found_runs,
        found_events,
        inventory_runs,
        inventory_events,
    } = refused
    else {
        panic!("a run count that disagrees with the boundary is refused by name: {refused:?}");
    };
    assert_eq!(
        (found_runs, found_events, inventory_runs, inventory_events),
        (3, 9, 4, 9)
    );
}

#[test]
fn a_head_seq_that_disagrees_with_the_boundary_is_refused_by_name() {
    let mut declared = None;
    let refused = refused_inventory(|inventory| {
        let head = &mut inventory.boundary.heads[0];
        head.seq += 1;
        declared = Some((head.run_id.clone(), head.seq, head.chain_head.clone()));
    });
    let (declared_run, declared_seq, declared_head) = declared.unwrap();
    let BackupError::HeadMismatch {
        run_id,
        found_seq,
        found_head,
        inventory_seq,
        inventory_head,
    } = refused
    else {
        panic!("a head seq that disagrees with the boundary is refused by name: {refused:?}");
    };
    assert_eq!(run_id, declared_run);
    // Only the seq moved: the chain head the snapshot found is the one
    // the boundary declares.
    assert_eq!((found_seq + 1, inventory_seq), (declared_seq, declared_seq));
    assert_eq!(
        (found_head.as_str(), inventory_head.as_str()),
        (declared_head.as_str(), declared_head.as_str())
    );
}

#[test]
fn a_boundary_missing_a_run_the_snapshot_holds_is_refused_by_name() {
    let refused = refused_inventory(|inventory| {
        inventory.boundary.heads.pop();
    });
    let BackupError::BoundaryRun { found, inventory } = refused else {
        panic!("a boundary missing a run is refused by name: {refused:?}");
    };
    assert_eq!((found.as_str(), inventory.as_str()), ("gamma", "<none>"));
}

#[test]
fn a_boundary_naming_a_run_the_snapshot_lacks_is_refused_by_name() {
    let refused = refused_inventory(|inventory| {
        inventory.boundary.heads.push(Head {
            run_id: "zulu".into(),
            seq: 1,
            chain_head: ZERO_HASH.into(),
        });
    });
    let BackupError::BoundaryRun { found, inventory } = refused else {
        panic!("a boundary naming an extra run is refused by name: {refused:?}");
    };
    assert_eq!((found.as_str(), inventory.as_str()), ("<none>", "zulu"));
}

#[test]
fn a_snapshot_whose_integrity_check_reports_problems_is_refused_with_every_row() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    // An index built empty and then declared whole: SQLite's pages are
    // sound, and `PRAGMA integrity_check` answers one row per problem.
    let refused = refused_edited_restore(dir.path(), &taken, |connection| {
        connection
            .execute_batch(
                "CREATE INDEX runs_probe ON runs(run_id) WHERE 0;
                 PRAGMA writable_schema = ON;
                 UPDATE sqlite_schema SET sql = 'CREATE INDEX runs_probe ON runs(run_id)'
                 WHERE name = 'runs_probe';
                 PRAGMA writable_schema = OFF;",
            )
            .unwrap();
    });
    let BackupError::Integrity { reported } = refused else {
        panic!("a snapshot failing integrity is refused by name: {refused:?}");
    };
    assert_eq!(
        reported,
        "wrong # of entries in index runs_probe; row 1 missing from index runs_probe; \
         row 2 missing from index runs_probe; row 3 missing from index runs_probe"
    );
}

#[test]
fn a_snapshot_whose_tables_are_not_the_journals_shape_is_refused_as_sqlites_error() {
    for (shape, ghost, said) in [
        (
            "ALTER TABLE runs RENAME TO runs_moved",
            false,
            "no such table: runs",
        ),
        (
            "ALTER TABLE events RENAME COLUMN run_id TO owner",
            false,
            "no such column: run_id",
        ),
        (
            "ALTER TABLE events RENAME COLUMN seq TO position",
            true,
            "no such column: seq",
        ),
        (
            "ALTER TABLE events RENAME COLUMN envelope TO body",
            false,
            "no such column: envelope",
        ),
    ] {
        let error = refused_shape(shape, ghost);
        assert!(error.contains(said), "{shape}: {error}");
    }
}
