# 0125 — An entry starts when its recipe's measured demand, beside what running runs still reserve, fits what the machine has now

Status: accepted — operator ruled in chat 2026-11-24, "all recommended" (https://github.com/feedback-loop-ai/brokkr/issues/1193#issuecomment-7126565397), after ruling #1193's asks A to G "all recommended" on 2026-11-23 (https://github.com/feedback-loop-ai/brokkr/issues/1193#issuecomment-7105395628)
Date: 2026-11-23

Built: partial (#1193) — Linux is built by 1193b; macOS's measures, `brokkr system host upgrade` and `init`'s placeholder ceilings are 1193c's
Amends: 0068

## Context

Decision 0068 ruling 3 says an entry starts only when the host has room.
Its addendum of 2026-10-10 makes that room the machine's own host file:
provider and route ceilings, free-space floors and a boxed-build ceiling.
Decision 0110 ruling 7 added a run ceiling. Every one of them is a count
the operator declares and guesses. None of them is what a run costs the
machine.

The operator, on #1193 (2026-11-20): "the dispatcher should make sure the
work scheduled can be executed, and there are ample resources for that";
"average per recipe and base our assumptions there"; "BROKKR should be
measuring"; "min, max, peak load, per phase"; "just cpu and memory"; and
disk as policy, "have 30 gb+ space to start a run".

Decision 0112 (accepted 2026-11-20, built as #1208) records each attempt's
CPU and memory under `effect-usage/v1`, and its ruling 6 says admission
reads none of it. This decision is the reader. The operator ruled #1193's
asks A to G "all recommended" on 2026-11-23, and split the work into 1193b
(this decision's Linux build) and 1193c (macOS's measures, the host-file
upgrade verb and `init`'s placeholder ceilings).

## Rulings

1. **What is measured, on Linux, on every pass.** Admission measures what
   the machine has available now, inside the pass, never carried from an
   earlier pass, and nothing latches:
   - **memory:** `MemAvailable` from `/proc/meminfo`, which already counts
     a tmpfs's pages against it; the whole machine is `MemTotal`;
   - **CPU:** idle cores, from two readings of `/proc/stat`'s `cpu` line
     500 ms apart: the idle and iowait ticks' share of all ticks between
     them, times the `cpuN` lines the file lists, in thousandths of a core;
     the whole machine is those cores (ruling A);
   - **each running run's use now:** its driver's process tree, the pid its
     run lock records (decision 0108), summed root and descendants by
     parent as decision 0112's meter sums them, from two process-table
     readings over the same 500 ms: its resident memory at the second, and
     the CPU time it spent between them as its load.

   The measures are one seam, so every test injects them and none reads the
   real machine's numbers. The only measurement a pass waits on is the CPU
   window, and only a pass that judges an entry's room takes it.

2. **Where availability cannot be measured, admission is decision 0068's
   as it stood.** On macOS until 1193c, and where `/proc/meminfo` or
   `/proc/stat` cannot be read or does not say what is asked of it, every
   check is main's, exactly: the run ceiling stays required, a host file
   that declares none holds every entry as `runs_undeclared`, and no demand
   is judged. `brokkr doctor` says the machine is unmeasured. An entry is
   never held for being unmeasured on such a machine.

3. **What is learned, and from which history.** A recipe's expected demand
   is learned from the machine's own history (rulings B and G):
   - **which runs:** the last 20 ended runs of the recipe, by recipe name, that
     carry `effect-usage/v1`, newest first by when each was created, read
     across the pass's own journal and every distinct journal the realms map
     the workspace reads lists (the map `brokkr system realms` reads); the
     host file's `demand.window_runs` overrides the 20;
   - **the read:** one bounded statement per journal: of the latest twice
     `window_runs` ended runs of each recipe, only the `phase/entered` events and
     the endings that carry `usage`. A run among them that carries none is
     passed over, so a window may hold fewer runs than it asks;
   - **per phase:** an attempt's peak memory and peak load are its own, or,
     for an attempt of several drivers, their sum (a panel's members; a
     sequence's steps are summed too, an upper estimate); its phase is the
     one the last `phase/entered` before its ending names. A phase's
     expected peak memory and peak load are the means of those attempt
     peaks over the window;
   - **an entry's demand:** for each resource, the largest of its recipe's
     phases' expected peaks, plus the margin, 25% by default and the host
     file's `demand.margin_percent` where it declares one (ruling B),
     rounded up.

   **The other journals are read and never written.** Each is opened
   without a `-wal` or `-shm` where none was: through SQLite's `immutable`
   URI where no connection holds it, and read-only over the side files a
   holder left otherwise. None is created, migrated, leased or locked.
   A journal the map lists that is not there holds no runs. One that is
   there and cannot be read contributes no history and no reservation, is
   named once on `brokkr queue list`, and never fails the pass; ruling 6
   counts it as one that may hold a run.

4. **What is declared.** A recipe may declare its demand in the host file,
   keyed by recipe name: `demand.recipes.<name>` with
   `peak_memory_bytes` and `peak_load_millicores`, taken as its expected
   peaks, the margin added as to a learned one. A declaration is read only
   while the recipe's window holds no measured run: history, once there is
   any, wins. The recipe format does not change.

5. **What running runs reserve.** Every run that reads `running` in any
   journal the pass reads (its own included) reserves, for each resource,
   `max(0, expected peak − its use now)`, where the expected peak is the
   largest of its recipe's expected phase peaks over its current phase and
   every phase its recipe's window entered after that phase in any run
   (every measured phase, where the window never entered its current one
   or measured any remaining phase), without the margin. A run in implement still holds its verify build. A run
   whose driver's tree cannot be read (no recorded holder, a pid the table
   does not list, a table that cannot be read) reserves its whole expected
   peak, and a reason that names the reservation says how many were. Each
   entry admitted in a pass reserves its own demand before the next entry
   is judged, so one headroom is never handed out twice in a pass.

6. **The rule, and every reason it gives.** The checks of decision 0068
   ruling 3, its addendum and decision 0110 ruling 7 run first, in their
   order, and keep their own reasons exactly: the host file, the seats,
   the provider and route ceilings, the cool-downs, the free-space floors
   and the boxed builds. Then, where availability is measured:
   - **`demand_unknown`:** an entry whose recipe has no measured run and no
     declaration starts only when no run reads `running` in any journal the
     pass reads, none of them is unreadable, and no entry was admitted
     earlier in the pass; otherwise it waits, and its first run then
     measures it (ruling C). A run that runs, or an entry admitted earlier
     in the pass, whose own demand is unknown holds the whole machine the
     same way: every later entry waits as `demand_unknown`, naming it.
   - **larger than the whole machine:** a demand larger than `MemTotal`, or
     a load larger than every core (a build whose peak load is every core
     and its margin), is judged against the whole machine: it starts only
     when nothing runs, as ruling C's entry does, and waits as
     `memory_short` or `cpu_short` naming the whole machine otherwise.
   - **`memory_short`:** the entry's memory demand plus every running run's
     memory reservation and every earlier admission's is more than
     `MemAvailable`.
   - **`cpu_short`:** the same, for load against idle cores. Memory is
     judged first.

   Each reason names the resource and its numbers: the demand, as the
   average and the margin it adds; what is available; and what is
   reserved, by how many runs. All three wait and clear by themselves. No
   admission path asks a model or reads the network.

7. **The caps that remain are policy, and optional.** Where availability is
   measured, a host file that declares no run ceiling (`forge.host/v1` to
   `v3`, and a `v5` that leaves it out) no longer holds every entry as
   `runs_undeclared`: entries are judged by demand, and a declared ceiling
   is still obeyed. The boxed-build ceiling stays as an optional operator
   override, absent by default (ruling D). `forge.host/v5`, minted beside
   the frozen v4, makes `runs` and `boxed_builds` optional and adds the
   optional `demand` block of rulings 3 and 4; every v1 to v4 file reads as
   it did, its demand block the defaults.

8. **`brokkr queue list` shows why.** The reason an entry waits names the
   demand it was judged by (its average, over which runs or a declaration,
   and the margin), what is available, and what is reserved and by how
   many runs, in the table and in `--json` alike. `--json` also carries,
   for every entry the pass sized, a `demand` object: the recipe, the
   demand's source, each resource's average, margin and demand, what is
   available and what is reserved; an entry the pass sized nothing for
   carries none, so an unmeasured pass lists as before. Each unreadable
   journal is named once on stderr. The per-recipe, per-phase view of
   decision 0112 (`brokkr library recipes usage`) is unchanged.

## The asks of #1193, as ruled

The operator ruled them "all recommended" on 2026-11-23.

- **A. macOS's measures, and Linux's CPU.** macOS's memory is free plus
  reclaimable pages, with memory pressure at warn or critical as a veto,
  and its CPU is idle cores from tick deltas: 1193c's slice. Linux uses the
  same idle-core measure from `/proc/stat` (ruling 1).
- **B. The average and its margin.** A 25% margin over the average of the
  recipe's last 20 runs, both overridable in the host file (rulings 3
  and 7).
- **C. No history and no declaration.** The entry starts only when nothing
  else runs; otherwise it waits as `demand_unknown` (ruling 6).
- **D. The boxed-build ceiling.** An optional override, absent by default
  (ruling 7).
- **E. `init`'s placeholder provider ceilings.** `init` seeds ceilings of 1
  and never overwrites: 1193c's.
- **F. `brokkr system host upgrade`.** It brings a v1 to v3 host file up:
  1193c's. Until it exists, `brokkr doctor` names what this decision
  changed rather than the verb.
- **G. Every journal of the realms map.** History and reservations read
  every journal the realms map lists, since per-worktree journals would
  otherwise see no history (rulings 3 and 5).

## What 1193c inherits

- macOS's measures (ask A) behind the same seam ruling 1 names, so ruling 2
  stops applying there once they are built;
- `brokkr system host upgrade` (ask F), which writes a v5 file, and
  `brokkr doctor`'s floors line naming it once it exists;
- `init`'s placeholder provider ceilings of 1 (ask E).

## What this decision does not do

- It does not judge disk by demand: the free-space floors stay the policy
  the operator declares ("have 30 gb+ space to start a run").
- It does not re-judge demand inside a claim's transaction. A dispatcher
  claims and starts what its pass admitted, in order, and each was judged
  with every earlier one's reservation. Two dispatchers of two journals
  that pass inside the same 500 ms can each admit into one headroom; the
  next pass of each sees the other's run and its reservation. A
  machine-wide admission lock would close it, and is a follow-up.
- It does not check that the pid a run lock records is still the driver's:
  a pid the kernel handed to another process is read as that process's
  tree, which uses real memory the measure of what is available already
  subtracts.
- It does not change what decision 0112 records, nor the fold.

## Consequences

- A machine whose recipes have history starts as many runs as fit, with no
  guessed run ceiling; the floors, the provider and route ceilings and the
  cool-downs bind exactly as before.
- A recipe with no history runs alone until its first run ends with measurements.
  Until the fleet's engines record `effect-usage/v1` (#1208, in no release
  yet), every recipe is unknown, and a measured machine runs one entry at a
  time.
- `queue list` and the dispatcher's pass each wait 500 ms for the CPU
  window when they judge an entry's room, and read one history statement
  and one statement for the unended runs per journal of the map, folding
  each unended run. Over 20 journals of 120 runs each (about 12 MB each,
  60 running runs in all) the reads took about 0.95 s in a debug test
  build, measured on 2026-11-23; the window comes on top.
- `forge.host/v5` is minted under this decision, and decision 0082 ruling
  1 mints a contract version only under an accepted ruling: the ledger's
  minting check holds the file until this decision is accepted.
