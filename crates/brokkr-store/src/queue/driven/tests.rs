//! Decision 0110 rulings 3 and 6: a driver resumes only a run linked under
//! its claim with a `spawn` row, and status names each entry left
//! launching or linked with no `spawn` row, both read on a journal opened
//! read-only.

use super::super::tests::{claimed_now, queue, started};
use super::*;
use crate::queue::Generation;

/// Entry 1 linked to `run-a` and spawned, entry 2 linked to `run-b` with no
/// `spawn` row, entry 3 launching, and each one's claim.
fn three_entries(store: &mut Store) -> (Vec<EntryId>, [Claim; 3]) {
    let ids = queue(store, 3);
    started(store, "run-a");
    started(store, "run-b");
    let one = Generation(1);
    let first = claimed_now(store, ids[0]).unwrap().unwrap();
    store.queue_link(first, one, "run-a", "r").unwrap();
    store.queue_spawn(first, one, "r").unwrap();
    let second = claimed_now(store, ids[1]).unwrap().unwrap();
    store.queue_link(second, one, "run-b", "r").unwrap();
    let third = claimed_now(store, ids[2]).unwrap().unwrap();
    (ids, [first, second, third])
}

#[test]
fn a_driver_reads_only_a_run_linked_under_its_claim_with_a_spawn_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (ids, [first, second, third]) = three_entries(&mut Store::open(&path).unwrap());
    let store = Store::open_read_only(&path).unwrap();
    let spawned = |claim: Claim| store.queue_spawned(claim.parts().0, claim.parts().1);
    assert_eq!(spawned(first).unwrap(), Some("run-a".to_string()));
    assert_eq!(spawned(second).unwrap(), None, "no spawn row");
    assert_eq!(spawned(third).unwrap(), None, "not linked");
    let (_, seq) = first.parts();
    assert_eq!(
        store.queue_spawned(ids[1], seq).unwrap(),
        None,
        "another's claim"
    );
    assert_eq!(store.queue_spawned(ids[0], seq + 100).unwrap(), None);
}

#[test]
fn status_reads_each_entry_left_launching_or_linked_with_no_spawn_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (_, [_, second, third]) = three_entries(&mut Store::open(&path).unwrap());
    let store = Store::open_read_only(&path).unwrap();
    assert_eq!(
        store.queue_unspawned().unwrap(),
        vec![(second, Some("run-b".to_string())), (third, None)]
    );
}

#[test]
fn a_journal_whose_queue_this_binary_does_not_read_refuses_both() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .conn
        .execute(
            "UPDATE meta SET value = '999' WHERE key = 'queue_schema'",
            [],
        )
        .unwrap();
    let mismatch = |error: StoreError| {
        matches!(
            error,
            StoreError::Queue(crate::QueueRefusal::SchemaMismatch { found: 999 })
        )
    };
    assert!(mismatch(store.queue_spawned(EntryId(1), 1).unwrap_err()));
    assert!(mismatch(store.queue_unspawned().unwrap_err()));
}

/// A page of the queue's claims SQLite reads as corrupt refuses both reads
/// as the journal's damage, naming the file (#1000), never bare SQLite.
#[test]
fn a_corrupt_claims_page_is_the_journals_damage_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let (ids, [first, ..]) = three_entries(&mut Store::open(&path).unwrap());
    crate::test_support::damaged::damage_table(&path, "queue_claims");
    let store = Store::open_read_only(&path).unwrap();
    let damaged = |error: StoreError| match error {
        StoreError::Damaged { journal, damage } => (journal, damage),
        other => panic!("not damage: {other:?}"),
    };
    let expected = (path.clone(), crate::Damage::Malformed);
    let spawned = store.queue_spawned(ids[0], first.parts().1).unwrap_err();
    assert_eq!(
        spawned.to_string(),
        format!(
            "the journal '{}' is damaged (SQLite reads its disk image as malformed); nothing \
             was read from it; restore it from a backup with `brokkr journal restore` and see the \
             guide's \"A damaged journal\" (#1000)",
            path.display()
        )
    );
    assert_eq!(damaged(spawned), expected);
    assert_eq!(damaged(store.queue_unspawned().unwrap_err()), expected);
}
