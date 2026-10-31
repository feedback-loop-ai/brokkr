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

The 918c review held that split but found the detection too narrow: it
asked only whether the walk held an identity the registry did not know.
A known leader that turned zombie, or a recorded descendant that
vanished, was therefore no change at all, the tick took no snapshot,
`observe` and `note` never ran, and the attempt kept `ended: None`. A
detached orphan born after that tick was then attributed to the attempt
with no end bound and killed with it. The operator's ruling of
2026-10-30, "go with recommended on all the rulings" (#918), amends
ruling 2: any change in the tree since the last walk takes one full
snapshot; 0099 and `process/tree.rs` are corrected to say so.

The 918e review found that the 918d tick saved the walk before the full
read and discarded that read's error. A walk that reported the leader
turning zombie took its snapshot, the snapshot failed on an unrelated row,
and the walk was nevertheless remembered as answered; the next tick, with
an identical zombie walk, took no snapshot, left `ended` unset, and
attributed a detached orphan born after it to the ended attempt and killed
it. The operator's ruling of 2026-10-30, "go with recommended" (#918),
fixes it: a snapshot the walk owes stays owed until a full read succeeds.

## Rulings

1. On Linux the tracker's periodic tick (`attempts::start`'s loop) walks
   the engine's own tree through `/proc/<pid>/task/<tid>/children`, and
   answers two questions: did any row change since the last walk, and is
   there an identity in it the registry does not already know? A change is
   an identity that appears or disappears, or a row whose `zombie` or
   `exiting` state changed. The known identities are every live attempt's
   `before`, `recorded` and `doubted`, what the engine owns, the engine
   itself, and each live driver. A walk returns a `Sighting`: the rows it
   saw and whether it could complete. The tick keeps the last walk as its
   own memory, compared by identity and state; that memory is never passed
   to `Live::record`, `observe`, `recorded`, `doubted` or a kill, and no
   parent is inferred from the listing.
2. When this tick's walk differs from the last tick's walk in any row,
   when it sees an identity the registry does not know, when it cannot
   complete (a named process or thread vanished or was replaced under it,
   an unreadable stat, any doubt), or when there is no previous walk (the
   first tick, or the first after the registry was empty), the tick takes
   exactly one full `table::snapshot`, and attribution goes through main's
   existing logic: `Registry::read` and `observe`. A snapshot a changed walk
   owes stays owed until a full read succeeds: a read that fails is not an
   answered change, and the next tick takes the full snapshot again even
   when its walk is unchanged. Only while no snapshot is owed does a quiet
   tick, the same rows in the same states, take none. The tick's own read
   error is still dropped there, as 918d had it; admission, every kill, the
   settlement reads and the stop sweep keep surfacing their typed errors as
   `Unsettled`. A walk that misses a process
   because it was read after that process's `children` file was finished
   only means the next tick sees it under its real parent; the walk cannot
   attribute.
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
   the tick read the table, and that read is what decides. The snapshot
   does not remove the identities it no longer shows from the attribution
   sets: `recorded` and `doubted` retain them, and `unowned` alone is
   pruned. The test
   `a_pid_reused_between_the_walk_and_the_snapshot_is_not_attributed`
   keeps P in `recorded` and passes P's old identity to the kill hook
   while V is excluded. The safety is instead that the walk's rows never
   enter those sets at all, and that a signal is identity-checked: on
   Linux a pidfd pins the process and its start stamp is read again
   before the signal (`table::kill`), so a reused pid is not the recorded
   identity; on macOS `ps` checks the stamp first (`table::kill_listed`),
   with the read-then-signal window the macOS residual names.
6. The choice changes no wire, journal, contract or public API, and
   installs no new process containment. Per-attempt cgroup containment
   remains #472's work.

## What the walk must report

`observe` and `note` read these facts from the full snapshot when they
decide an open attempt's lifecycle, and the walk's comparison covers each
one:

- **An identity.** `Live::record`, `Live::explains`, `before`, `recorded`
  and `doubted` all turn on the pid and start stamp. An identity that
  appears is a new row and one that disappears is a missing row, both
  changes. A reused pid carries a different start stamp, so it is one
  identity disappearing and another appearing in the same tick.
- **Running versus zombie.** `Live::note` and `Live::record` skip a
  zombie, and `Live::running` counts only a non-zombie row. The row's
  `zombie` is compared. `exiting` is compared too; Linux lists no
  exiting process (`table::parse_stat`), so it cannot change there.
- **Explained by group.** `Live::record` seeds its roots with the group
  while the leader is open, `Live::explains` matches `pgid`, and
  `Live::running` refuses a group member. While the attempt is open, a
  process can join its group only from within the group's session, and
  that session is the driver's own, entered only by fork from the
  driver; so every group member is the driver's descendant and the
  engine's, and the walk lists it. A process that leaves the group
  changes `pgid` without changing identity; it stays in the engine's own
  tree by descent, and `recorded` or `doubted` explains it by identity,
  so the group clause is an extra way to explain a process, never the
  only one for one the walk has already seen. `pgid` is not compared
  because it cannot make a new process appear in an open attempt's group
  or make a recorded one cease to be the attempt's.
- **The youngest birth.** `Live::note` sets `ended` from the start stamps
  the snapshot shows, and `Live::left` bounds attribution by it. The
  start stamp is part of the identity, so a changed stamp is an identity
  change. The snapshot that computes it runs on the first tick that
  shows the last running row of the attempt turn zombie or disappear,
  which is exactly the change the walk now reports; before 918d a quiet
  tick could skip that snapshot and leave `ended` unset indefinitely.
- **A parent.** `Live::record` follows `ppid` from a recorded identity.
  On Linux a process's parent changes only when its old parent exits and
  the subreaper adopts it, and the old parent is in the engine's tree, so
  its disappearance is a change reported in the same tick. A live parent
  cannot hand its child to a new parent. `ppid` is not compared because
  the cause of its change is.

In particular, a process in an open attempt's group that is not the
engine's descendant cannot exist on Linux. The group is the driver's
session, a `setsid` session of its own; a process enters a session only
by being forked from a member, and `setpgid` refuses a group in another
session. Every group member is therefore a descendant of the driver, and
the walk sees it. A running member keeps the attempt open and is a row
the walk lists; one that turns zombie is a compared state change; one
that is reaped is a disappearance. There is no quiet tick on which a
group member can hold an open attempt open unseen.

After the leader is reaped the group is no longer the attempt's:
`Live::record` and `Live::kill` gate their group clauses on `open`, so a
group number reused by a stranger cannot add a recorded identity or take
a group signal. `Live::note` and `Live::running` still compare the bare
`pgid`, so a stranger that reuses the number after the reap can keep
`ended` unset on an attempt that is already closing. That is main's
close-time reading of a reused group number, not a change the walk failed
to report: main read the whole table every tick and saw the same. It is
left as a follow-up rather than widened here.

The one fact the walk does not report is the full table's own
non-atomicity: a process can be listed and exit before its row is read,
so a snapshot can miss a birth the next snapshot would have seen. That is
main's accepted table-read residual (`process/tree.rs`, decision 0006),
not the walk-attribution race this decision closes, and #472 closes it by
cgroup containment.

## Enforcement binding

The split is bound by tests that fail on a compiling mutation:

- `process::attempts::tests::a_pid_reused_between_the_walk_and_the_snapshot_is_not_attributed`
  plants the 918c race: P is recorded, its numeric pid is reused by an
  unrelated U with child V, and the walk notices a change. The one full
  snapshot does not record V, and the kill signals P's identity only.
  Feeding the walk's rows to `observe` instead fails it by name.
- `process::attempts::tests::quiet_ticks_take_no_snapshot_and_a_doubt_takes_one`
  takes one snapshot on the first tick, which has no previous walk, then
  drives ten quiet ticks through the injectable `Host` and counts zero,
  then one doubtful walk and counts one. Taking a snapshot on every tick
  fails it.
- `process::attempts::tests::a_leader_that_turns_zombie_on_a_quiet_tick_ends_its_attempt`
  plants a leader that turns zombie, asserts the tick takes a snapshot and
  sets `ended`, then plants a detached orphan born after that end and
  asserts it is not recorded and never reaches the kill hook. Comparing
  the walk by identity only fails it by name, and so does dropping the
  last-walk comparison.
- `process::attempts::tests::a_recorded_descendant_that_vanishes_from_the_walk_takes_one_snapshot`
  plants a recorded descendant that disappears and asserts the tick takes
  one snapshot after two quiet ones. Dropping the last-walk comparison
  fails it by name.
- `process::attempts::tests::chief_failed_snapshot_retries_before_attributing_a_later_orphan`
  plants a leader that turns zombie while the full read fails on an
  unrelated row, then a healthy tick with the identical walk: it asserts
  the third read, `ended` set, the later orphan unowned and absent from the
  kill hook. Saving the walk before the read and dropping its error fails
  it by name ("the snapshot the failed read owed was forgotten": left 2,
  right 3).
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
and a quiet tick takes no full snapshot. A changed tree, a walk that
could not complete, or the first tick after the registry was empty costs
one full snapshot, which is what main paid on every tick. A known row
that turns zombie or disappears is a change, so the snapshot that ends an
attempt can no longer be skipped. A full read that fails leaves the walk's
snapshot owed, so the next tick retries it even when the walk is unchanged,
and only a successful read records the walk as seen. Admission pays one full table read per attempt, and a kill or
settlement pays full reads during its bounded window. A pid reused by a
stranger, a stale child pid or a stale ancestry can no longer reach the
kill set from a walk: the walk only makes the tracker take the snapshot
that main already trusted to attribute. The macOS residual is unchanged
and remains accepted. Only the operator accepts this decision; this
document claims no acceptance.
