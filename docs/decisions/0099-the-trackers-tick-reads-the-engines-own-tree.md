# 0099 — The tracker's tick reads the engine's own tree; admission and the kill read the full table

Status: proposed
Date: 2026-10-29
Built: built

## Context

Issue #918, part of #330. The tracker read the whole host process table
every 100 ms while any attempt was live: one `/proc` listing and one
`stat` per host process. On a host with 1,326 processes that is 1,326
reads eleven times a second, for an engine that is idle between its own
tree's changes. The story run took option (a), the subtree walk, but made
every read use it. Review found two wrong-kill orderings in that choice:

- A child's pid named by its parent's `children` file can be reused by a
  stranger before the child's own row is read. Assigning the listing's
  parent to that row gives the stranger false ancestry, records it as the
  attempt's and passes it to the kill.
- Admission's `before` read was the walk. A process that already ran
  before the attempt can be missing from a walk (its parent exits before
  its own row is read, or it is reparented while the walk runs), so
  `before` is incomplete and a pre-existing process is attributed to the
  new attempt and killed.

The operator's ruling of 2026-10-29, "Fix round, split reads
(Recommended)", fixes the split: the tick walks, admission and the kill
read the full table. The alternatives were weighed and rejected. Keeping
the walk for admission is the second defect above. Keeping the full table
for the tick is the cost #918 exists to remove. Reading the `/proc`
top-level listing only for admission and trusting pid ordering does not
tell a pre-existing identity from a reused one, and the kernel offers no
atomic snapshot of the table.

## Rulings

1. On Linux the tracker's periodic tick (`attempts::start`'s loop) reads
   the engine's own tree: the engine's row and its descendants, found by
   `/proc/<pid>/task/<tid>/children`. Each walked entry keeps the parent
   and start stamp its own `/proc/<pid>/stat` reports; the walk never
   assigns a parent from the listing. A child whose own stat names a
   parent other than the process whose `children` file listed it is
   dropped from that walk, with its subtree. A process that truly moved
   is found under its real parent on a later tick, as an orphan is
   found today. A row that cannot be read still fails the tick, as it
   failed the full table, because it could be the attempt's.
2. Admission's read before a spawn keeps the full table, exactly as on
   main, so `before` names every process that already ran, including one
   a walk would miss while it is reparented. The read is once per
   attempt, not once per tick, and is taken afresh.
3. The kill-time read in `read_and_kill`, the settlement reads and the
   stop sweep keep the full table, exactly as on main. A read the tick
   took is never shared as an admission or kill read: the registry pairs
   reads by the function that made them, so the walk and the full table
   are distinct reads.
4. The tick's walk cannot add to the kill set a process that admission's
   full `before` excludes. The kill set is the recorded and doubted
   identities. Attribution only reaches an identity absent from
   `before`, so a process that existed at admission is never attributed
   from a later tick; a process that did not exist then is a birth after
   admission, which the existing attribution rules already judge. The
   walk's narrower view changes what a tick sees, never the guarantee
   `before` gives.
5. macOS is unchanged: with no subreaper it reads the whole table with
   `ps` at every read, including the tick. The operator's ruling of
   2026-09-28, "LINUX CLOSED, MACOS RESIDUAL ACCEPTED", stands; this
   proposal neither widens nor narrows that accepted residual.
6. The choice changes no wire, journal, contract or public API, and
   installs no new process containment. Per-attempt cgroup containment
   remains #472's work.

## Enforcement binding

The split is bound by tests that fail on a compiling mutation:

- `process::table::tests::the_engine_tree_read_does_not_grow_with_the_host`
  counts the `stat` files a tick opens and holds them at three for ten
  and for two thousand unrelated host rows.
- `process::table::tests::a_reused_child_pid_is_not_walked_from_the_listing`
  plants a reused child pid under a stranger parent; the walk omits it,
  so `Live::record` never records it and `Live::kill` never signals it.
  Restoring `entry.ppid = pid` fails it.
- `process::attempts::tests::a_pre_existing_orphan_the_walk_misses_is_not_the_attempts`
  plants a pre-existing detached process the walk misses; admission's
  full `before` keeps it the engine's own and no kill reaches it.
  Making admission read the walk fails it.
- `process::table::tests::the_real_host_reads_its_own_tree_at_the_tick_and_the_full_table_elsewhere`
  asserts the real host's two reads: the tick excludes the engine's
  parent, the full table includes it.
- `process::table::tests::the_table_errors_name_what_they_could_not_read`
  pins the typed table errors' display text once, and the walk and row
  tests match `TableError::Row` and `TableError::NoSelf` by variant.

`cargo test -p brokkr-protocol --locked --lib process::` is the gate.

## Consequences

An idle engine's tick costs its own tree, not the host's process count.
Admission pays one full table read per attempt, and a kill or settlement
pays full reads during its bounded window, which is what main paid
before. A process that predates an attempt can no longer be misread as
its own because a walk missed it, and a pid reused by a stranger can no
longer be given the attempt's ancestry from a stale listing. A walked
process that exits or is reparented mid-walk is simply found on a later
tick or under its real parent, as the tree already handles orphans. The
macOS residual is unchanged and remains accepted. Only the operator
accepts this decision; this document claims no acceptance.
