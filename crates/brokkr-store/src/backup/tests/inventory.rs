//! The inventory's read (decision 0100 ruling 2, #945): it is read from a
//! regular file only and under a byte bound, so no backup directory can
//! block a restore in `open(2)` or grow its memory until the host refuses
//! it. Each planted inventory is refused by name, and none leaves a byte
//! behind.

use super::*;
use crate::backup::inventory::INVENTORY_MAX_BYTES;

/// The child half of the inventory tests: the backup directory and the
/// destination arrive through the environment, and the child answers the
/// call it is named for.
#[cfg(unix)]
const INVENTORY_BACKUP: &str = "BROKKR_INVENTORY_BACKUP";
#[cfg(unix)]
const INVENTORY_DESTINATION: &str = "BROKKR_INVENTORY_DESTINATION";
#[cfg(unix)]
const INVENTORY_DEV_ZERO_CHILD_TEST: &str =
    "backup::tests::inventory::an_inventory_that_is_a_link_to_dev_zero_is_refused_by_name";
#[cfg(unix)]
const INVENTORY_PAST_BOUND_CHILD_TEST: &str =
    "backup::tests::inventory::an_inventory_that_is_a_regular_file_past_the_bound_dies_small";

/// The memory bound a child that plants an inventory dies under: 256 MiB,
/// replaced by `ulimit -d` where the host does not enforce `-v`. A read
/// that lost the inventory's bound dies there instead of filling the host.
#[cfg(unix)]
const INVENTORY_MEMORY_LIMIT: &str = "ulimit -v 262144 2>/dev/null || ulimit -d 262144";

/// Run one inventory test in a child under [`INVENTORY_MEMORY_LIMIT`],
/// with the backup directory and destination handed through the
/// environment.
#[cfg(unix)]
fn inventory_child(test: &str, backup: &Path, destination: &Path) -> std::process::Output {
    child_under(
        INVENTORY_MEMORY_LIMIT,
        test,
        &[
            (INVENTORY_BACKUP, backup.as_os_str()),
            (INVENTORY_DESTINATION, destination.as_os_str()),
        ],
    )
}

/// A completed backup whose inventory name is free for a test to plant.
#[cfg(unix)]
fn planted_inventory(dir: &Path) -> (Backup, PathBuf) {
    let (_, taken) = completed(dir);
    let inventory = taken.to.join(INVENTORY_FILE);
    std::fs::remove_file(&inventory).unwrap();
    (taken, inventory)
}

/// The child's backup directory and destination, when this process is the
/// child half of a test.
#[cfg(unix)]
fn inventory_child_env() -> Option<(PathBuf, PathBuf)> {
    let backup = PathBuf::from(std::env::var_os(INVENTORY_BACKUP)?);
    let destination = PathBuf::from(
        std::env::var_os(INVENTORY_DESTINATION).expect("the child names its destination"),
    );
    Some((backup, destination))
}

/// Restore `taken` into `destination`, answer its counts, and remove the
/// restored file so the same destination is free for the next call.
fn restore_counts(taken: &Backup, destination: &Path) -> (u64, u64) {
    let restored = restore(&taken.to, destination).unwrap();
    let counts = (restored.runs, restored.events);
    std::fs::remove_file(destination).unwrap();
    counts
}

/// A FIFO with no writer is refused by name without blocking `open(2)`:
/// the nonblocking open answers, the handle's own metadata says it is not
/// a regular file, and no byte is read. The restore runs on a thread so a
/// regression that blocks fails this test at its deadline instead of
/// hanging the suite.
#[cfg(unix)]
#[test]
fn a_fifo_at_the_inventory_is_refused_by_name_without_a_writer() {
    const DEADLINE: Duration = Duration::from_secs(10);

    let dir = tempfile::tempdir().unwrap();
    let (taken, inventory) = planted_inventory(dir.path());
    let made = std::process::Command::new("mkfifo")
        .arg(&inventory)
        .status();
    assert!(made.unwrap().success(), "mkfifo {}", inventory.display());
    let destination = absent_destination(dir.path());
    let (answered, heard) = mpsc::channel();
    let asked = taken.to.clone();
    let asked_destination = destination.clone();
    let restore_thread = std::thread::spawn(move || {
        let _ = answered.send(restore(&asked, &asked_destination));
    });
    let refused = heard
        .recv_timeout(DEADLINE)
        .expect("the restore did not answer: a FIFO blocked open(2)")
        .unwrap_err();
    restore_thread.join().unwrap();
    let BackupError::InventoryNotRegular(found) = refused else {
        panic!("a FIFO at the inventory is refused by name: {refused:?}");
    };
    assert_eq!(found, inventory);
    assert_absent(&destination);
}

/// A directory, or a device reached through a link, is not a regular file
/// and is refused by name before it is read.
#[cfg(unix)]
#[test]
fn a_directory_or_a_device_at_the_inventory_is_refused_by_name() {
    for planted in ["directory", "device"] {
        let dir = tempfile::tempdir().unwrap();
        let (_, taken) = completed(dir.path());
        let inventory = taken.to.join(INVENTORY_FILE);
        std::fs::remove_file(&inventory).unwrap();
        match planted {
            "directory" => std::fs::create_dir(&inventory).unwrap(),
            _ => std::os::unix::fs::symlink("/dev/null", &inventory).unwrap(),
        }
        let destination = absent_destination(dir.path());
        let refused = restore(&taken.to, &destination).unwrap_err();
        let BackupError::InventoryNotRegular(found) = refused else {
            panic!("a {planted} at the inventory is refused by name: {refused:?}");
        };
        assert_eq!(found, inventory, "{planted}");
        assert_absent(&destination);
    }
}

/// A socket cannot be opened as a file at all, so it is refused as the I/O
/// error that names the path; the refusal still comes before any read.
#[cfg(unix)]
#[test]
fn a_socket_at_the_inventory_is_refused_by_name() {
    use std::os::unix::net::UnixListener;

    let dir = tempfile::tempdir().unwrap();
    let (taken, inventory) = planted_inventory(dir.path());
    let _socket = UnixListener::bind(&inventory).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&taken.to, &destination).unwrap_err();
    let BackupError::UnreadableInventory { path, .. } = refused else {
        panic!("a socket at the inventory is refused by name: {refused:?}");
    };
    assert_eq!(path, inventory);
    assert_absent(&destination);
}

/// A symlink to a regular file stays readable as it always was. A backup
/// this build writes never holds one — its inventory is published by a
/// hard link from a regular temporary — but a hand-edited directory can.
#[cfg(unix)]
#[test]
fn a_symlinked_inventory_to_a_regular_file_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let inventory = taken.to.join(INVENTORY_FILE);
    std::fs::rename(&inventory, taken.to.join("real-inventory.json")).unwrap();
    std::os::unix::fs::symlink("real-inventory.json", &inventory).unwrap();
    let destination = absent_destination(dir.path());
    let restored = restore(&taken.to, &destination).unwrap();
    assert_eq!((restored.runs, restored.events), (3, 9));
}

/// A link to `/dev/zero` is a character device, and the handle's metadata
/// refuses it before it is read. The child runs under the memory bound so
/// a regression that read the stream unbounded would die there, not fill
/// the host.
#[cfg(unix)]
#[test]
fn an_inventory_that_is_a_link_to_dev_zero_is_refused_by_name() {
    if let Some((backup, destination)) = inventory_child_env() {
        let refused = restore(&backup, &destination).unwrap_err();
        let BackupError::InventoryNotRegular(found) = refused else {
            panic!("a /dev/zero inventory is refused as not regular: {refused:?}");
        };
        assert_eq!(found, backup.join(INVENTORY_FILE));
        assert_refused_without_trace(&backup, &destination);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (taken, inventory) = planted_inventory(dir.path());
    std::os::unix::fs::symlink("/dev/zero", &inventory).unwrap();
    let destination = absent_destination(dir.path());
    let output = inventory_child(INVENTORY_DEV_ZERO_CHILD_TEST, &taken.to, &destination);
    assert_child_passed(&output, "the /dev/zero inventory");
}

/// A backup that would write an inventory past the bound refuses before it
/// publishes one, so every inventory this build writes is one its restore
/// reads. The caller's `taken_at` is unbounded, and is the cheapest way to
/// make a real inventory too large without seeding 32,000 runs.
#[test]
fn a_backup_whose_inventory_would_exceed_the_bound_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    let to = dir.path().join("backup");
    let now = "x".repeat(INVENTORY_MAX_BYTES as usize + 1);
    let refused = backup(
        &store,
        BackupOptions {
            to: &to,
            now: &now,
            projection: PROJECTION,
        },
    )
    .unwrap_err();
    let BackupError::InventoryTooLarge { path, bound } = refused else {
        panic!("an inventory past the bound is refused by name: {refused:?}");
    };
    assert_eq!(
        (path, bound),
        (to.join(INVENTORY_FILE), INVENTORY_MAX_BYTES)
    );
    assert!(
        !to.join(INVENTORY_FILE).exists(),
        "a refused backup published no inventory"
    );
    assert!(
        !to.join(SNAPSHOT_FILE).exists(),
        "the bound was checked before the snapshot was published"
    );
    assert_eq!(
        std::fs::read_dir(&to).unwrap().count(),
        0,
        "a refused backup left nothing in the destination"
    );
}

/// An inventory at exactly the bound is read and parsed; one byte past it
/// is refused as oversized before it is parsed. The build's own inventory
/// reads both before the padding and at the bound, so the bound holds
/// every inventory this build writes in this fixture.
#[test]
fn an_inventory_at_the_bound_is_read_and_one_byte_past_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let inventory = taken.to.join(INVENTORY_FILE);
    let text = std::fs::read(&inventory).unwrap();
    let bound = INVENTORY_MAX_BYTES as usize;

    let destination = absent_destination(dir.path());
    assert_eq!(restore_counts(&taken, &destination), (3, 9));

    let mut at = text.clone();
    at.resize(bound, b' ');
    std::fs::write(&inventory, &at).unwrap();
    assert_eq!(restore_counts(&taken, &destination), (3, 9));

    let mut past = text;
    past.resize(bound + 1, b' ');
    std::fs::write(&inventory, &past).unwrap();
    let refused = restore(&taken.to, &destination).unwrap_err();
    let BackupError::InventoryTooLarge { path, bound: named } = refused else {
        panic!("an inventory one byte past the bound is refused as oversized: {refused:?}");
    };
    assert_eq!((path, named), (inventory, INVENTORY_MAX_BYTES));
    assert_absent(&destination);
}

/// A *regular* file that runs past the bound is the case the bound itself
/// answers: the read stops at one byte past the bound and refuses. The
/// file is sparse, so the test costs no disk; the child runs under the
/// memory bound, so restoring an unbounded read dies there rather than
/// filling the host.
#[cfg(unix)]
#[test]
fn an_inventory_that_is_a_regular_file_past_the_bound_dies_small() {
    const SPARSE_BYTES: u64 = 1 << 30;

    if let Some((backup, destination)) = inventory_child_env() {
        let refused = restore(&backup, &destination).unwrap_err();
        let BackupError::InventoryTooLarge { path, bound } = refused else {
            panic!("an inventory past the bound is refused as oversized: {refused:?}");
        };
        assert_eq!(
            (path, bound),
            (backup.join(INVENTORY_FILE), INVENTORY_MAX_BYTES)
        );
        assert_refused_without_trace(&backup, &destination);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let inventory = taken.to.join(INVENTORY_FILE);
    let mut past = std::fs::read(&inventory).unwrap();
    past.resize(INVENTORY_MAX_BYTES as usize + 1, b' ');
    std::fs::write(&inventory, &past).unwrap();
    let sparse = std::fs::OpenOptions::new()
        .write(true)
        .open(&inventory)
        .unwrap();
    sparse.set_len(SPARSE_BYTES).unwrap();
    drop(sparse);
    let destination = absent_destination(dir.path());
    let output = inventory_child(INVENTORY_PAST_BOUND_CHILD_TEST, &taken.to, &destination);
    assert_child_passed(&output, "the sparse inventory");
}

/// `/proc/self/mem` is a regular file whose first read is `EIO`: it passes
/// the regular-file check and reaches the read, which is refused as the
/// I/O error that names the inventory.
#[cfg(target_os = "linux")]
#[test]
fn an_inventory_whose_bytes_cannot_be_read_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let (taken, inventory) = planted_inventory(dir.path());
    std::os::unix::fs::symlink("/proc/self/mem", &inventory).unwrap();
    let destination = absent_destination(dir.path());
    let refused = restore(&taken.to, &destination).unwrap_err();
    let BackupError::UnreadableInventory { path, source } = refused else {
        panic!("an inventory that cannot be read is refused by name: {refused:?}");
    };
    assert_eq!(path, inventory);
    assert_eq!(source.raw_os_error(), Some(5), "{source}");
    assert_absent(&destination);
}

#[test]
fn a_journal_before_work_orders_backs_up_with_null_and_restores() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    seed(&mut store, "alpha", 1);
    store
        .conn
        .execute_batch(
            "DELETE FROM meta WHERE key = 'orders_schema'; DROP TABLE work_order_events;",
        )
        .unwrap();

    let taken = snapshot_of(&store, &dir.path().join("backup"));
    let inventory = inventory_of(&taken);
    assert!(matches!(
        inventory.schemas.orders,
        OrdersSchema::Recorded(None)
    ));
    let destination = absent_destination(dir.path());
    assert_eq!(restore_counts(&taken, &destination), (1, 1));
}

#[test]
fn v1_inventory_restores_with_no_work_order_schema_field() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let mut inventory = inventory_of(&taken);
    inventory.format = "brokkr.journal-backup.v1".into();
    inventory.schemas.orders = OrdersSchema::Legacy;
    rewrite_inventory(&taken, &inventory);

    let destination = absent_destination(dir.path());
    assert_eq!(restore_counts(&taken, &destination), (3, 9));
}

#[test]
fn v2_requires_orders_and_checks_it_against_the_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let (_, taken) = completed(dir.path());
    let mut inventory = inventory_of(&taken);
    let destination = absent_destination(dir.path());
    for (orders, reason) in [
        (OrdersSchema::Legacy, "schemas.orders is required in v2"),
        (
            OrdersSchema::Recorded(None),
            "the snapshot's work-order schema is 1; schemas.orders says null",
        ),
        (
            OrdersSchema::Recorded(Some(2)),
            "the snapshot's work-order schema is 1; schemas.orders says 2",
        ),
    ] {
        inventory.schemas.orders = orders;
        rewrite_inventory(&taken, &inventory);
        let refused = restore(&taken.to, &destination).unwrap_err();
        let BackupError::InvalidInventory { source } = refused else {
            panic!("expected an invalid inventory: {refused:?}");
        };
        assert_eq!(source.to_string(), reason);
        assert_absent(&destination);
    }
    inventory.format = "brokkr.journal-backup.v1".into();
    rewrite_inventory(&taken, &inventory);
    let refused = restore(&taken.to, &destination).unwrap_err();
    let BackupError::InvalidInventory { source } = refused else {
        panic!("expected v1 to refuse the v2 field: {refused:?}");
    };
    assert_eq!(source.to_string(), "schemas.orders is absent in v1");
    assert_absent(&destination);
}

#[test]
fn backup_refuses_damaged_work_order_storage_before_publication() {
    for sql in [
        "UPDATE meta SET value = 'damaged' WHERE key = 'orders_schema'",
        "DROP TABLE work_order_events",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let store = journal(dir.path());
        let to = dir.path().join("backup");
        store.conn.execute_batch(sql).unwrap();
        let refused = backup(
            &store,
            BackupOptions {
                to: &to,
                now: TAKEN_AT,
                projection: PROJECTION,
            },
        )
        .unwrap_err();
        assert!(matches!(
            refused,
            BackupError::Store(StoreError::Damaged { journal, damage: crate::Damage::Orders })
                if journal == store.path()
        ));
        assert!(!to.join(INVENTORY_FILE).exists());
        assert!(!to.join(SNAPSHOT_FILE).exists());
    }
}
