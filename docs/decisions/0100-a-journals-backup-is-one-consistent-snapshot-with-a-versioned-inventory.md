# 0100 — A journal's backup is one consistent snapshot with a versioned inventory

Status: accepted — operator ruled in chat 2026-11-01, "go with recommended on #943" (https://github.com/feedback-loop-ai/brokkr/issues/431#issuecomment-6556152545)
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
   read through the store's own `schema_version` and `queue_schema` readers,
   never a second vocabulary. Enforcement: the typed `Inventory` with
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

4. **Restore accepts only what verifies, and never overwrites.** A
   destination is occupied when `<to>` or any of SQLite's sidecars for it —
   `<to>-journal`, `<to>-wal`, `<to>-shm` — exists, asked with
   `symlink_metadata` so a symlink or a sidecar both count. A restore
   refuses an occupied destination by the name it found, before reading
   anything else, so a later writable open cannot roll a hot sidecar into
   the restored bytes. A file created beside the destination by another
   process while a restore runs is not detected; that residual is left open
   here, and the publish below still refuses the destination's own name.
   It accepts a backup only when the inventory parses and its `format` is
   one this build reads; the snapshot's size and SHA-256 equal the
   inventory's; `PRAGMA integrity_check` answers `ok`; every run's chain
   verifies with the store's existing chain verification and its head seq
   and chain head equal the boundary; every event row's SQL key —
   `run_id` and `seq` — equals its own envelope's, and its stored
   `event_hash` equals the hash the store computes over that envelope on
   append, the first disagreement refused by its run, SQL position and
   field; and the run and event counts equal the boundary. It writes to a
   temporary name beside the destination and
   publishes it under the destination's name only after every check has
   passed, so any refusal leaves the destination absent and an existing
   name is never written over. It starts nothing: no driver, no queue
   claim, no resume. A run that was in flight at the snapshot stays
   exactly as it was recorded, under the existing indeterminate-recovery
   rule.

   What the checks establish, table by table:
   - **Chains — `runs` and `events`:** every run's chain is verified whole
     from its first event, every event's `run_id` is a run the snapshot
     holds, and the boundary's run and event counts are the whole `runs`
     and `events` tables' — never the sum of the chains a reader loaded. A
     snapshot holding an event no run owns is refused by its count and its
     first `(run_id, seq)`. `backup` reads the same boundary, so a source
     journal holding an unowned event refuses backup by name too: that
     fail-closed behaviour is an ask of this proposal. Every owned row is
     also held to its own envelope: its SQL `(run_id, seq)` and stored
     `event_hash` must be the envelope's key and the hash the store
     computes on append, so the boundary `Store::load` reads is the one
     `head_hash`, `load_after` and `append_next` later use. The first row
     that disagrees is refused by its run, its SQL position and the field,
     and a malformed SQL position is reported as the integer the table
     holds, never repaired into `0`.
   - **Projections — `event_index`, `run_summary`, `seat_summary` and
     `seat_head`:** restore holds them to the store's own
     `Store::summary_disagreements`, the comparison `brokkr doctor` reports
     as failures, and refuses a snapshot whose list is not empty, naming
     the first entry. It writes no second comparison.
   - **`meta`:** read for the database and queue schema versions the
     inventory records.
   - **Recorded state — `queue_entries`, `queue_waits`, `queue_commands`,
     `queue_latches`, `queue_pins`, `queue_claims`, `queue_refusals`,
     `dispatch_holders`, `dispatch_settings` and
     `quarantine_acknowledgements`:** carried as recorded under the existing
     recovery rules; restore starts nothing.
   - **Authenticity:** the checks detect corruption, truncation, an
     interrupted backup and an internally inconsistent snapshot. They do
     not authenticate the backup against someone who rewrites the snapshot
     and the inventory together; that belongs to a later slice — anchoring
     or signing the inventory — and is an ask.

   Enforcement: `restore`, `verify_snapshot`, `refuse_existing`,
   `boundary_of`, `consistency::refuse_unowned`, `consistency::chain` and
   `compared_boundary`, with `backup::tests`' refusals for the truncated
   snapshot, the altered byte, the missing inventory, the other format, the
   disagreeing head, the disagreeing count, the live destination, each
   SQLite sidecar, an unowned event, a row whose SQL key or stored hash
   disagrees with its envelope, a malformed unowned position, a projection
   disagreement and the non-empty place.
   Dropping the early destination check changes the variant
   `a_live_destination_is_refused_by_name_and_left_byte_equal` matches.

   *Amended, 2026-11-15 (the operator's standing ruling of 2026-11-04,
   recorded on #945):* the inventory has one bound, 16 MiB
   (`INVENTORY_MAX_BYTES`), on both ends.
   - **The restore** opens `inventory.json` without blocking and asks the
     handle's own metadata first. A FIFO, a directory, a device or a link
     to `/dev/zero` is refused as `InventoryNotRegular` before a byte is
     read. A socket cannot be opened, so it is refused as
     `UnreadableInventory`. A symlink to a regular file reads as it always
     has. It then reads at most one byte past the
     bound, and a longer file is refused as `InventoryTooLarge`. So no file
     a backup directory holds at that name can grow a restore's memory past
     the bound or block it.
   - **The backup** holds its own inventory to the same bound before the
     snapshot is published, so every inventory this build writes is one it
     reads. A journal whose inventory would pass the bound is refused with
     its destination still empty. A head is under 512 bytes, so that is
     past 32,000 runs at their largest. A restore also refuses an inventory
     past the bound that an earlier build wrote.
   - **Enforcement:** `inventory::read_inventory` and `inventory::serialize`,
     with `backup::tests::inventory`'s
     `a_fifo_at_the_inventory_is_refused_by_name_without_a_writer`,
     `a_directory_or_a_device_at_the_inventory_is_refused_by_name`,
     `a_socket_at_the_inventory_is_refused_by_name`,
     `a_symlinked_inventory_to_a_regular_file_is_read`,
     `an_inventory_at_the_bound_is_read_and_one_byte_past_is_refused`,
     `an_inventory_that_is_a_regular_file_past_the_bound_dies_small` and
     `a_backup_whose_inventory_would_exceed_the_bound_is_refused_by_name`.

5. **The later slices, in order, and none built here.** After this journal
   half: (a) Git objects and refs/forge anchors; (b) artifacts outside the
   journal; (c) retention with a dry-run inventory; (d) the offline restore
   drill with its measured duration, size and recovery point; (e)
   upgrade/downgrade across the contract window (#361); (f) signing or
   anchoring the inventory, so authenticity is checked and not only
   consistency. Each takes its own
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

## Ruled (operator, in chat, 2026-11-01)

"go with recommended on #943" (https://github.com/feedback-loop-ai/brokkr/issues/431#issuecomment-6556152545),
on PR #943's asks: the decision is accepted as proposed.
- **Later slices, carried by #431:**
  - Git objects and the `refs/forge` anchors;
  - artifacts outside the journal;
  - retention, with a dry-run inventory;
  - the measured offline restore drill;
  - upgrade and downgrade across the contract window (#361), then
    signing and anchoring.
- **Accepted as built:**
  - a backup refuses a source holding an unowned event;
  - a restore verifies the snapshot against its inventory, and does not
    authenticate it;
  - the public backup API.

As built in #943, each verb also binds its paths to one literal identity
before SQLite or the filesystem sees them. The bundled SQLite reads a name
beginning `file:` as a URI (landing 431L4's finding), so without that
binding the checks above and the publication could name different files.
The same exposure on the CLI's other journal paths is #954.

## Addendum — 2026-11-19, accepted 2026-11-19: every journal open binds its path, and two names SQLite read specially are literal files (#954)

Addendum status: accepted (operator, 2026-11-19)

The operator accepted this addendum in chat on 2026-11-19, "go with
recommended" (https://github.com/feedback-loop-ai/brokkr/issues/954#issuecomment-7005169956),
ruling both asked items below as recommended: `--db :memory:` is the
literal file `./:memory:`, and an empty path is refused by both opens.

Under the operator's 2026-11-01 ruling on #431, #954 moves the binding
into the store's one open, so `Store::open` and `Store::open_read_only`
make every journal path absolute before SQLite sees it, and backup and
restore use the same home. Ordinary paths open the file they named
before. Two names SQLite read specially do not, and the operator rules on
them:

1. **`:memory:`.** SQLite opened `--db :memory:` as a private database
   that vanished with the process. It is now the durable file `./:memory:`,
   created by a writer and read back by a later reader. Recommended:
   accept, because one literal identity has no name that is not a file.
2. **The empty path.** SQLite opened `--db ''` for writing as a private
   temporary database. Both opens now refuse it as SQLite's invalid path
   before SQLite sees it. Recommended: accept.

A refusal to open a journal names the path as the operator gave it, not
the absolute path SQLite was handed.

## Addendum — 2026-11-24, accepted 2026-11-24: work-order storage in the inventory (#1272)

Under ruling 2, a backup now writes `brokkr.journal-backup.v2`. Its
`schemas.orders` records the work-order schema from the snapshot, with
JSON `null` when the snapshot has no work-order schema marker. A damaged
marker or missing table with a recorded marker refuses the backup.

A v2 build restores both `brokkr.journal-backup.v1` inventories, which
omit `schemas.orders`, and v2 inventories, which require it. For v2,
the stated work-order schema must agree with the snapshot's recorded
version, including absence of the marker for a stated `null`.

The operator accepted the backup inventory under ask 7 on 2026-11-24.
[Operator ruling on #1272](https://github.com/feedback-loop-ai/brokkr/issues/1272#issuecomment-7143427836)
