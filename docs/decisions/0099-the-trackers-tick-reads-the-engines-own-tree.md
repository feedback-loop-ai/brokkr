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
every read use it. Review found three wrong-kill orderings in that
choice:

- A child's pid named by its parent's `children` file can be reused by a
  stranger before the child's own row is read. Assigning the listing's
  parent to that row gives the stranger false ancestry, records it as
  the attempt's and passes it to the kill.
- Admission's `before` read was the walk. A process that already ran
  before the attempt can be missing from a walk (its parent exits before
  its own row is read, or it is reparented while the walk runs), so
  `before` is incomplete and a pre-existing process is attributed to the
  new attempt and killed.
- A recorded descendant P can exit and have its numeric pid reused by an
  unrelated U before P's children are read. The walk then follows U's
  child V through P's stale numeric ancestry, `Live::record` accepts V,
  and a later full snapshot does not remove it from `recorded`.

The operator's ruling of 2026-10-29, "Fix round, split reads
(Recommended)", fixed the first two: the tick walks, admission and the
kill read the full table. The operator's ruling of 2026-10-29, "go with
recommended on all four" (#918), rules the third by changing the design
rather than patching one more race: **the walk detects; the snapshot
attributes.**

## Rulings

1. On Linux the tracker's periodic tick (`attempts::start`'s loop) walks
   the engine's own tree through `/proc/<pid>/task/<tid>/children`, and
   only to answer one question: is there an identity in it the registry
   does not already know? The known identities are every live attempt's
   `before`, `recorded` and `doubted`, what the engine owns, the engine
   itself, and each live driver. A walk returns a `Sighting`: the rows it
   saw and whether it could complete. Its rows are never passed to
   `Live::record`, `observe`, `recorded`, `doubted` or a kill, and no
   parent is inferred from the listing.
2. When the walk sees an identity the registry does not know, or cannot
   complete (a named process or thread vanished or was replaced under it,
   an unreadable stat, any doubt), the tick takes exactly one full
   `table::snapshot`, and attribution goes through main's existing logic:
   `Registry::read` and `observe`. A walk that misses a process because it
   was read after that process's `children` file was finished only means
   the next tick sees it under its real parent; the walk cannot attribute.
3. Admission's read before a spawn, the kill-time read in
   `read_and_kill`, the settlement reads and the stop sweep keep the full
   table, exactly as on main. `before` names every process that already
   ran, including one a walk would miss while it is reparented.
4. macOS is unchanged: with no subreaper it reads the whole table with
   `ps` at every tick, as main did. The operator's ruling of 2026-09-28,
   "LINUX CLOSED, MACOS RESIDUAL ACCEPTED", stands; this proposal neither
   widens nor narrows that accepted residual.
5. **Why this closes the class.** Every identity that can be signalled
   was attributed by the same full-table read main already trusts. The
   walk can only cause an extra snapshot, never an attribution: a stale
   numeric pid, a reused child pid, or an inferred edge can at most make
   the tick read the table, and that read is what decides. A full
   snapshot removes an identity it no longer shows from the attribution
   sets, so no walk-only record can survive into the kill set.
6. The choice changes no wire, journal, contract or public API, and
   installs no new process containment. Per-attempt cgroup containment
   remains #472's work.

## Enforcement binding

The split is bound by tests that fail on a compiling mutation:

- `process::attempts::tests::a_pid_reused_between_the_walk_and_the_snapshot_is_not_attributed`
  plants the 918c race: P is recorded, its numeric pid is reused by an
  unrelated U with child V, and the walk notices a change. The one full
  snapshot does not record V, and the kill signals P's identity only.
  Feeding the walk's rows to `observe` instead fails it by name.
- `process::attempts::tests::quiet_ticks_take_no_snapshot_and_a_doubt_takes_one`
  drives ten quiet ticks through the injectable `Host` and counts zero
  snapshots, then one doubtful walk and counts one. Taking a snapshot on
  every tick fails it.
- `process::attempts::tests::a_pre_existing_orphan_the_walk_misses_is_not_the_attempts`
  keeps admission's full `before` excluding a pre-existing orphan the
  walk misses.
- `process::table::tests::a_reused_child_pid_is_not_walked_from_the_listing`
  keeps a reused child pid out of the walk, and both it and
  `a_tree_walk_skips_what_exited_and_fails_what_it_cannot_read` hold a
  vanished or replaced entry to a doubtful sighting.
- `process::tests::tracker_scope::a_detached_descendant_whose_parent_exits_between_ticks_is_ended_with_its_attempt`
  keeps LINUX CLOSED: a new descendant that leaves its group still ends at
  the attempt's end.
- `process::table::tests::the_engine_tree_read_does_not_grow_with_the_host`
  counts a quiet tick's `stat` files and holds them at three for ten and
  for two thousand unrelated host rows.
- `process::table::tests::the_real_host_reads_its_own_tree_at_the_tick_and_the_full_table_elsewhere`
  asserts the real host's two reads.
- `process::table::tests::the_table_errors_name_what_they_could_not_read`
  pins the typed table errors' display text once, and the walk and row
  tests match `TableError::Row` and `TableError::NoSelf` by variant.

`cargo test -p brokkr-protocol --locked --lib process::` is the gate.

## Consequences

An idle engine's tick costs its own tree, not the host's process count,
and a quiet tick takes no full snapshot. A changed tree, or a walk that
could not complete, costs one full snapshot, which is what main paid on
every tick. Admission pays one full table read per attempt, and a kill or
settlement pays full reads during its bounded window. A pid reused by a
stranger, a stale child pid or a stale ancestry can no longer reach the
kill set from a walk: the walk only makes the tracker take the snapshot
that main already trusted to attribute. The macOS residual is unchanged
and remains accepted. Only the operator accepts this decision; this
document claims no acceptance.
