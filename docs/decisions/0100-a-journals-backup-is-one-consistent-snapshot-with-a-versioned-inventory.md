# 0100 — A journal's backup is one consistent snapshot with a versioned inventory

Status: proposed
Date: 2026-10-26
Built: partial (#431)

## Context

#431's acceptance asks for a backup of a journal as a whole and a restore
procedure. A byte copy of a WAL journal while a driver appends is not a
consistent snapshot: recent commits may sit in the `-wal` beside the file,
and SQLite may write pages while the copy runs. Nothing today says when a
backup is complete, so a directory left half-written by an interrupted run
is indistinguishable from a good one.

This decision proposes the recovery unit for the journal half only
(slice 1, the story on `story-431-journal-backup`). The later halves are
named in ruling 5 and are not designed here. The ruling applies decision
0071's principles 2, 3, 4, 5, 6, 8, 9 and 10; it follows decisions 0027
(journals never merge; runs relocate), 0065 (bindings name secrets and
never carry their values), and 0068 (nothing is claimed by reading).

## Proposed rulings

1. **The recovery unit is the journal's database, whole.** A backup is one
   SQLite snapshot of the journal: the hash-chained events and the store's
   own `runs`, queue, acknowledgement and projection bookkeeping as one
   database at one instant. It deliberately carries no Git objects or
   refs/forge anchors, no artifacts outside the journal, no harness
   transcripts, and no credentials. The inventory names every exclusion,
   so no reader mistakes the backup for a complete recovery unit. The
   journal holds no credential by design: decision 0065's bindings name
   secrets, and their values live in the operator's store, which the backup
   never opens. Enforcement: the `excluded` field's closed vocabulary and
   `brokkr-store`'s `backup::tests::no_secret_value_is_copied`, which plants
   a bound secret's name in a run and finds its value nowhere in the backup.

2. **The inventory has one versioned format.** `inventory.json` declares
   `"format": "brokkr.journal-backup.v1"`; the `format` string is the whole
   version rule. A change to the field set that a previous build could not
   read takes a new string, and a build reads exactly the formats it names,
   refusing any other by name. Within one `format` the field set is closed:
   `format`, `engine` (the taking Brokkr version), `schemas` (the
   `database` and `queue` versions as the journal's `meta` records them,
   and the `projection` version the taking build reads), `snapshot`
   (`bytes` and `sha256`), `boundary` (`taken_at`, `runs`, `events`, and
   each run's `run_id`, `seq` and `chain_head`), and `excluded`. The projection version is stated by the taking build because
   the journal records none of its own; the database and queue versions are
   read from the journal. Enforcement: the typed `Inventory` with
   `deny_unknown_fields`, the `format` refusal, and
   `backup::tests::a_backup_records_the_versioned_inventory_this_build_reads`
   and `an_inventory_of_another_format_is_refused_by_name`.

3. **Completeness is by construction.** The snapshot is SQLite's own
   `VACUUM INTO`, a consistent copy of one read transaction taken while
   writers may run. The boundary is read from that snapshot, so the declared
   boundary and the bytes it declares are one database by construction
   (reading them in one transaction is not available: `VACUUM` refuses to
   run inside one). The snapshot and the inventory are written under
   temporary names and fsynced, and the inventory is published LAST, so a
   directory without an inventory, or whose inventory does not match its
   files, is an incomplete backup. No byte is written over anything: a
   backup refuses a destination that is not absent or empty, and each name
   is published atomically, never over an existing name. Enforcement: the publish order in
   `brokkr-store`'s `backup`, `prepare_directory`, `publish_noclobber` and
   `sync_dir`; `backup::tests::an_interrupted_backup_is_incomplete_and_restore_refuses_a_missing_inventory`
   interrupts the second publish and proves the snapshot stands without an
   inventory, and `a_backup_into_a_place_that_holds_a_file_is_refused_by_name`
   proves the refusal.

4. **Restore accepts only what verifies, and never overwrites.** A restore
   refuses an existing destination by name before reading anything else: a
   live journal is never a destination. It accepts a backup only when the
   inventory parses and its `format` is one this build reads; the snapshot's
   size and SHA-256 equal the inventory's; `PRAGMA integrity_check` answers
   `ok`; every run's chain verifies with the store's existing chain
   verification and its head seq and chain head equal the boundary; and the
   run and event counts equal the boundary. It writes to a temporary name
   beside the destination and publishes it under the destination's name
   only after every check has passed, so any refusal leaves the destination
   absent and an existing name is never written over. It starts nothing: no driver,
   no queue claim, no resume. A run that was in flight at the snapshot stays
   exactly as it was recorded, under the existing indeterminate-recovery
   rule. Enforcement: `restore`, `verify_snapshot` and `compared_boundary`,
   with `backup::tests`' refusals for the truncated snapshot, the altered
   byte, the missing inventory, the other format, the disagreeing head, the
   disagreeing count, the live destination and the non-empty place.
   Dropping the early destination check changes the diagnostic
   `a_live_destination_is_refused_by_name_and_left_byte_equal` asserts.

5. **The later slices, in order, and none built here.** After this journal
   half: (a) Git objects and refs/forge anchors; (b) artifacts outside the
   journal; (c) retention with a dry-run inventory; (d) the offline restore
   drill with its measured duration, size and recovery point; (e)
   upgrade/downgrade across the contract window (#361). Each takes its own
   design and its own decision where it changes a contract. This ruling is
   judgment-guidance for the order; the only determinable part is that the
   inventory's `excluded` names these things, and ruling 1's test binds
   that.

## Alternatives and costs

A byte copy of the file is not a snapshot, and a copy that quietly omits
the `-wal` is worse than none. The online backup API would also work, but
`VACUUM INTO` needs no new crate feature, is one statement, and leaves a
compact standalone database; the boundary read from its output is stronger
than a second read of the live file. Reading the boundary from the live
journal after the snapshot would be a different instant and is rejected.

Writing the inventory first would let an interrupted backup present an
inventory with no snapshot; publishing it last makes the inventory the
completion mark, and the interruption test fails if the order is reversed.
A restore that wrote in place, or that opened the destination to check it,
could overwrite a live journal or start a run; both are refused by
construction instead.

The cost is that a backup is a separate copy, not a physical reflink, and
that a restored journal is a standalone (rollback-journal) database until a
writable open puts it in WAL; neither changes a byte of the journal's
content. The exclusions mean a backup taken alone is not a complete
recovery unit for a run whose evidence includes Git objects or artifacts.

## Consequences

Slice 1 adds `brokkr backup --to <dir>` and
`brokkr restore --from <dir> --to <journal path>`, the
`brokkr.journal-backup.v1` inventory, and the public
`brokkr_store::backup` module (`backup`, `restore`, `BackupOptions`,
`Backup`, `Restored`, `BackupError`) that the verbs call. The store's
journal and its full-chain verification are unchanged; the frozen
contracts, fixtures, policy and reference directories are untouched. The
verbs are the ask this proposal puts to the operator; the API additions
are the ask in `brokkr-store`.

A storage format that needs the whole database in one file is the reason
`VACUUM INTO` is the recommended snapshot: it is the SQLite-supported way
to produce one consistent file while writers run. The later slices remain
pending, so the `Built:` marker is `partial (#431)`.
