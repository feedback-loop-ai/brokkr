# 0104 — Shared local hardware sets a deadline floor, and the host sweeps the seat trees Brokkr made

Status: proposed — asked of the operator on #430
Date: 2026-11-06

Built: unbuilt (#430)

## Context

This is #430's fifth slice. It covers decision 0068 rulings 5 and 6, which
0095's plan sends here as "`shared-local` capacity and the scratch sweep":

> 5. **Local hardware declares itself.** A route may be declared
> `shared-local` in the realm's host configuration, with its own concurrency
> ceiling and a seat limit default that replaces the cloud-sized one. The
> operator's Spark ruling becomes data instead of a remembered rule.
>
> 6. **The host is swept, not reaped by hand.** Seat scratch trees carry the
> effect that made them. When that effect is terminal and no process holds the
> tree, the dispatcher removes it on its next pass and journals what it
> removed and how much space it freed. A tree whose effect is still open is
> never touched, whatever its age.

0068's addendum of 2026-10-10 moved the host configuration from the realm to
the machine: `forge.host/v1`, at `$XDG_CONFIG_HOME/brokkr/host.json`. Slice 3
(#621) built rule 5's concurrency half: a route the host file declares
`shared-local` is counted against its own ceiling in place of its provider's.
Rule 5's deadline half is not built, and neither is any of rule 6.

The evidence is the controller's, from the house workstation:
- **The Spark deadline.** 0068 records a seat routed to the operator's DGX
  Spark that inherited a one-hour limit sized for a cloud API and was killed
  with nothing written. The operator ruled that Spark seats never carry that
  limit. Today the ruling is one hand-written number in one recipe
  (night-shift's implement seat, 7200 seconds).
- **The trees.**
  - 0068 records sixteen gigabytes of dead seat trees reaped by hand on
    2026-09-23.
  - On 2026-09-21 the 31 GB `/tmp` tmpfs filled, and review members parked
    "seat wrote no result file". It held a seat's coverage tree of 7.6 GB and
    a dozen `brokkr-hands-exec-*` session trees of about 1.3 GB each.
  - On 2026-11-02, 19 GB on the same tmpfs exhausted the host's memory: an
    8.7 GB cargo target and about fifty test tempdirs that killed test
    processes left behind.
  - On 2026-11-06 the run-local seat scratch under the workstation's project
    directories held 4.5 MB in four trees.
  - So of that volume, only the hands session trees were trees Brokkr made,
    and their reaper (#415, #620) now removes a dead owner's tree at each
    start. The rest was written by seats and tests outside any Brokkr tree.
    This decision says plainly which trees the sweep can attribute and remove
    (ruling 5), and leaves the rest as a named question.

This decision is docs only. It chooses what slice 5 builds, so the operator
can rule before code exists. Decision 0082 makes a new contract version
mintable only under an accepted decision; the version named below is minted
when this one is accepted.

## What main holds at HEAD

Measured at be0b6a871.

**Rule 5, capacity: built.**
- `crates/brokkr-runtime/src/admission/host.rs:396-420`: `Route { ceiling, class }` and `RouteClass { Cloud, SharedLocal }`, documented as decision 0068 ruling 5. `forge.host/v1` and `/v2` both carry `class`.
- `crates/brokkr-runtime/src/admission/capacity.rs:367` and `:671-702`: a shared-local lane is left out of its provider's count and checked against its own ceiling. An undeclared route counts as cloud.

**Rule 5, the deadline: not built.**
- `crates/brokkr-runtime/src/bundle.rs:349-361`: `Limits { max_attempts, timeout_seconds }`, whose default is one attempt and 3600 seconds.
- `crates/brokkr-runtime/src/bundle/assemble.rs:273-290`: a seat's inline `limits` win, else its agent's, else the default.
- `crates/brokkr-runtime/src/engine/dispatch.rs:106`: the attempt's deadline is `seat.limits.timeout_seconds`. No route, host or class reaches it.
- **Authored limits are the norm.** For example, `agents/implementer.json:10` sets 5400, `agents/reviewer.json:11` 3600, `agents/triage.json:16` 1800 and `agents/implementer-engine.json:32` 7200. `recipes/fast/bundle.json` sets 5400, 3600 and 1800 per seat. A host value that replaced only `Limits::default()` would reach almost no shipped seat.
- **The Spark ruling is hand-written.** `recipes/night-shift/bundle.json:22` sets the implement seat's 7200 seconds, and `recipes/night-shift/README.md:44` cites "the `spark-glm` route the operator ruled `local` on 2026-09-16".
- `adapters/dsh.json:5-8` declares the routes `spark` and `spark-glm` as `local`. That is decision 0036's egress class (`EgressClass::Local`), a network fact, not capacity. Nothing ties it to a host route's class.
- `crates/brokkr-view/src/fleet.rs:178-193`: "the journal records no attempt's deadline". The fleet's staleness bound is the longest shipped deadline, 2 hours, plus an hour. A brokkr-cli test holds every recipe, agent and bundle to that 2 hours.

**Rule 6: not built.**
- **Run-local seat scratch.**
  - `crates/brokkr-runtime/src/engine.rs:1300-1303` and `:1348-1353`: an unboxed exec dispatch with hands starts in `<run-local>/scratch/<attempt_id>/{home,tmp}` (`SEAT_SCRATCH`, `:3201`).
  - `<run-local>` is the folder's `state/`, else `<workdir>/.forge` (`crates/brokkr-runtime/src/places.rs:689`).
  - The tree's name is the attempt id alone. Nothing removes it and nothing journals it.
  - `crates/brokkr-runtime/src/launch/queued.rs:363` measures the same directory for the `seat-scratch` floor.
- **Hands session trees.**
  - `crates/brokkr-protocol/src/hands/session.rs` names a tree `brokkr-hands-<label>-<pid>-<uuid>` under `$TMPDIR`, with an `.owner.lock` its owner holds.
  - Boxed exec, server boxes and `brokkr doctor` make them, inside the process that runs the box.
  - The owner's drop and signal handler remove the tree. `reap_dead_sessions` (`:342`; `reapable`, `:381`) removes one whose pid is dead and whose lock is free. It runs at every start, resume and rerun (`crates/brokkr-cli/src/lib.rs:505`) and prints `Reaped` (`:261`) to stderr. Nothing is journaled.
- **Holder evidence.**
  - An attempt's process tree is proven over or not (#403, `Cleanup` in `crates/brokkr-protocol/src/lib.rs:479-484`).
  - An unresolved cleanup makes the attempt indeterminate (`:536`), and its `effect/indeterminate` carries the unproven evidence, per seat or per panel member (`crates/brokkr-runtime/src/engine.rs:110-140`).
  - So a succeeded or failed attempt is proven over by construction.
- **Storage.** Queue storage is at v3 (`crates/brokkr-store/src/queue/storage.rs:59`), with append-only tables guarded by triggers, `dispatch_holders` (`:233`) and `dispatch_settings` (`:245`) among them. Decision 0103 reserves v4 for `provider_cooldowns` (its slice 4d).
- **The dispatcher.** `crates/brokkr-runtime/src/dispatch.rs:6` leaves "its pass and the `brokkr dispatch` verb" to later slices. The journal's dispatch lock (`dispatch/lock.rs`, `take_dispatch`) is built.

## Rulings

Each ruling gives the options, what each costs and refuses, and the
recommended option. The operator may accept all or pick.

### 1. The deadline on shared local hardware

**Recommended: (a) a floor per shared-local route, declared in the host file,
that raises a seat's deadline and never shortens it.**

- **(a) `providers.<name>.routes.<route>.timeout_floor_seconds` on a shared-local route, in `forge.host/v3`.**
  - An attempt whose launched candidate is on that route runs under the larger of the seat's own `timeout_seconds` and the floor.
  - The key is valid only on a route whose `class` is `shared-local`; on a cloud route it refuses with its own line. It is a whole number above 0 and at most 7200.
  - **The cap.** 7200 is the fleet's attempt bound (`fleet.rs:183`), the same number on purpose. The staleness bound is `ATTEMPT_DEADLINE_SECONDS` plus an hour, so a floor at or under it keeps that bound true. Any longer, a live attempt could pass the staleness bound and read as a dead engine, and the shipped-deadline test would no longer hold.
  - A v1 or v2 file declares no floor. Every object stays closed, so an unknown key refuses as it does in v2.
  - This writes the Spark ruling once, as the machine's data. A seat authored longer keeps its length, and one authored shorter is raised on that route only. Cloud routes and `max_attempts` are untouched.
- **(b) Replace only `Limits::default()`.** Refused. The compiler would have to mark which limits were not authored, which means a manifest field, and so a digest move. It would also reach almost no shipped seat, because the agents author their limits.
- **(c) Override every limit on the route, longer or shorter.** Refused. A host file would shorten a deliberate authored deadline (implementer-engine's 7200) to its own number.
- **(d) Keep writing it in recipes.** Refused. That is the remembered rule rule 5 retires.

Cost: `forge.host/v3`, and one host read per attempt, from the host file the
launch names (the seam decision 0103's slice 4c1 gives the engine for its
cool-down bounds). The bundle, its manifest and every digest stay unmoved, so
a host edit drifts no pin and refuses no resume.

### 2. Which attempt the floor reaches

**Recommended: the floor follows the candidate the attempt launches, read
again at each attempt.**

- **The route** is the one 0036 ruling 2 defines: the concrete model id's prefix before its first `/`, the same predicate capacity counts by.
- **A chain** that falls back from a shared-local link to a cloud link runs the cloud link under the seat's own limit.
- **A panel's attempt** has one deadline (`run_panel`), and the panel waits for its slowest member. So the attempt takes the highest floor among the sites it launches.
- **A sequence step** takes the floor of the candidate its own invocation launches.
- **A probe (0103 ruling 4)** is not an attempt. This decision leaves its bound as 0103's slices build it.

### 3. The record of a raised deadline

**Recommended: (a) the attempt's own words, with no new event field.**

- **(a)** When an attempt ends at a raised deadline, its failure text names both numbers and the source. For example: "the seat's 7200-second deadline: the shared-local floor host.json declares for dsh/spark-glm; the seat's own limit is 3600 seconds". `brokkr doctor` prints each declared floor.
- **(b) An optional `deadline` extension on `effect/started`, under a minted schema.** The most exact record, at a contract's cost. The fleet's staleness bound does not need it, because ruling 1 caps the floor at that bound. Not recommended.

### 4. Egress `local` and capacity `shared-local`

**Recommended: `brokkr doctor` names a mismatch and never refuses.**

The two facts stay separate. An adapter's egress class (0036) is the
adapter's network fact. A route's class (0068's addendum) is this machine's
capacity fact, and a machine may not run the route at all.

Doctor reports each route an adapter declares egress `local` that this host
file does not declare `shared-local`, whether it is absent or declared
`cloud`. It says that the route's seats keep their authored deadlines and
count against their provider. Neither class implies the other.

### 5. Which trees the sweep removes

**Recommended: (a) the trees Brokkr itself makes for an attempt, and nothing
else.**

- **Run-local seat scratch** (`<run-local>/scratch/<attempt_id>`) is removed by its effect's state, under rulings 6 and 7.
- **Hands session trees** (`brokkr-hands-*` under `$TMPDIR`) belong to a process, not an effect: a box's tree lives exactly as long as its owner. Their reaper's rule stands: the pid is dead and the owner lock is free. When the sweep pass runs that reaper, what it removes is journaled like any swept tree (ruling 8).
- **Not swept:** files a seat writes outside a Brokkr tree (the host's `/tmp`, caches, cargo targets), worktrees, and test tempdirs. They are not Brokkr's to attribute; see What stays out.
- **(b) Also sweep by age under `$TMPDIR`.** Refused. Rule 6 forbids age as evidence ("whatever its age"), and neither age nor an open-file scan proves a tree is free. A deletion by hand that trusted such a scan once removed a live seat's tree: the scan missed a child writing an 8 GB tree.

### 6. How a tree carries its effect

**Recommended: (a) a journal row written before the tree is made.**

- **(a) `seat_trees`, in queue storage v5** (after decision 0103's v4).
  - **Columns:** rising `seq`; `run_id`; `effect_id`; `attempt_id`; `root` (the run-local scratch directory, absolute); `recorded_at`. The tree's path is always `root/attempt_id` and is never stored apart. A unique index is on `(run_id, attempt_id)`.
  - **`CHECK`s:** non-empty identifiers; an absolute `root` whose last component is `scratch`, as `SEAT_SCRATCH` names it; and an `attempt_id` spelled as the UUID the engine mints for an attempt (`crates/brokkr-runtime/src/engine/dispatch.rs:86`), so it is one path component and never `..`.
  - Triggers refuse update and delete, as 0095's tables do.
  - The engine appends the row before it makes the tree. A crash in between leaves a row with no tree, which the sweep closes as gone (ruling 8).
  - **The row, not the disk, is the authority.** The sweep reads its work from the rows and never walks a directory to find work, so it never removes a tree no row names. `brokkr doctor` lists such trees under each recorded root, with their size: trees an older engine or a person made.
- **(b) An owner record inside the tree.** Refused as the authority: the seat writes in its own tree, so the record would be the seat's to rewrite.
- **(c) A path layout `<run>/<effect>/<attempt>`.** Refused. It moves the layout of the measured `seat-scratch` place, it still needs the journal to say whether the effect is closed, and the disk would become the work list.
- **(d) Look attempt ids up through `event_index`.** Refused. It needs a disk walk to find work and a scan by attempt, since that index is keyed by run first.

### 7. When a tree may go

**Recommended: all three of the following, each read from the journal at the
pass.**

- **The effect is closed:** an `effect/succeeded` exists for its `effect_id`, or its run is terminal (completed, failed or stopped; a concluded run is stopped). A parked run's open effect keeps every attempt's tree, whatever its age. They are the operator's evidence until the operator retries or concludes.
- **No process holds it:** the attempt has a terminal event, and that event carries no unproven-cleanup evidence. A succeeded or failed attempt is proven by construction. An indeterminate one is proven only when it carries none. An attempt with no terminal event, or with unproven cleanup, keeps its tree and is reported.
- **The path is the row's, and only the engine's shape:** `root` ends in `scratch`, `attempt_id` is a UUID, and `root/attempt_id` is a real directory, not a link, on the same filesystem as `root`. A row of any other shape is reported by name and is never removed, even though the `CHECK`s should make one impossible.

There is no retention period: rule 6 says "on its next pass". The dry-run
listing (ruling 9) is the window to look before a pass. A declared
`keep_hours` in the host file is possible, but it is not recommended.

### 8. The removal and its record

**Recommended:**

- **The walk.**
  - It follows no link and crosses no filesystem.
  - It makes each directory owner-writable before removing its entries, because hands overlays leave read-only directories.
  - It counts each inode's allocated bytes once, and removes the root last.
- **One row per complete removal, in `tree_sweeps`** (same storage version as `seat_trees`).
  - **Columns:** rising `seq`; `kind` (`scratch` or `hands`); `tree_seq` (the `seat_trees` row, required for `scratch`, absent for `hands`); `path`; `bytes`; `removed_at`; `generation`, the dispatch holder's generation, absent for an on-demand pass.
  - A unique index is on `tree_seq`. Triggers refuse update and delete.
- **A partial removal** records nothing, and the pass reports its error. The next pass tries again.
- **A recorded tree whose path is already absent** is recorded with `bytes` absent, meaning gone before the pass. So every `seat_trees` row closes.

### 9. Who sweeps, before slice 6

**Recommended: (a) one sweep function, and `brokkr sweep` to run one pass on
demand.**

- **(a)**
  - The pass holds the journal's dispatch lock (`take_dispatch`), so it never runs beside a dispatcher's pass. A second sweeper is refused by name.
  - `brokkr sweep --dry-run` lists what the pass would remove, with sizes, and writes nothing.
  - Slice 6's `brokkr dispatch` calls the same function on every pass, as 0068 ruling 7 has it ("starts runs, records cool-downs and sweeps").
- **(b) Sweep at every run start, beside the hands reaper.** Automatic now. But a start is not the dispatcher, holds no dispatch lock, and puts removal work into a run's startup.
- **(c) Build the function for slice 6 only.** Nothing sweeps until slice 6 exists.

## Asked of the operator

- Rulings 1 to 9, each as recommended or a picked option.
- The floor's cap of 7200 seconds, tied to the fleet's staleness bound.
- The verb's name, `brokkr sweep`.

## What stays out

- **The unit a shared-local ceiling counts.** Slice 3 counts a run once per provider and route (#621). Panels that reach the Spark in parallel count once. Counting seats would be its own decision.
- **`max_attempts` on shared-local routes.**
- **Files a seat writes outside a Brokkr tree.** This was most of the measured volume. Giving every harness seat a Brokkr-owned `TMPDIR` inside its attempt's tree would make those files attributable and sweepable. It changes the seat's environment, and a temp directory inside a worktree has leaked files into tests, so it is a separate decision.
- **Trees an older engine made.** `brokkr doctor` lists them, and they are never adopted.
- **Worktrees and their cargo targets,** which the controller owns.
- **A retention period** (ruling 7).
- **The standing loop**, which is slice 6.

## The slices

Smallest first. Each lands alone, tested, through the merge queue.

**Queue order:**
- 5a queues after decision 0103's slice 4c1 merges, because 4c1 is reshaping `engine/dispatch.rs`, where the deadline is read.
- 5b and 5c queue after 0103's slice 4d merges, not only after its design, because their tables take queue storage v5 and 4d's take v4.
- 5c follows 5b.

1. **5a: the host floor and the deadline.** It needs nothing from decision 0103's open slices except 4c1's merge (see the queue order above).
   - **Files:** `crates/brokkr-runtime/src/admission/host.rs` and `host/tests.rs`; the new `forge.host/v3` schema; `crates/brokkr-runtime/src/engine/dispatch.rs` (the deadline read, with its logic in a child module); the host-file seam where 0103's 4c1 did not already carry it; `brokkr doctor` (the floors, and ruling 4's line); `frozen_contracts.rs`; `docs/guides/versioning.md`'s lifecycle table; and `contracts/README.md`.
   - **Contract:** `forge.host/v3`.
   - **Tests:**
     - a v2 file reads as no floor, and a v3 floor reads;
     - a floor on a cloud route, a zero and a value past 7200 each refuse with their own line;
     - an attempt on a shared-local route runs under the floor;
     - a longer authored limit is kept;
     - a chain that falls back to a cloud link keeps the seat's limit;
     - a panel takes the highest floor;
     - the timeout's text names both numbers and the source;
     - doctor's floor line and its egress mismatch line.
2. **5b: the tree record.**
   - **Files:** `crates/brokkr-store/src/queue/storage.rs` (queue storage v5: `seat_trees`) and its tests; the store's append; and the engine's write before `create_dir_all`. engine.rs takes the call site only, with the logic in a child module.
   - **Tests:**
     - the row lands before the tree;
     - a crash between them leaves a row and no tree;
     - the uniqueness and the `CHECK`s;
     - the triggers refuse update and delete.
3. **5c: the sweep.**
   - **Files:** `tree_sweeps` in the same storage version; the sweep function in brokkr-runtime; `brokkr sweep [--dry-run]` in brokkr-cli; the hands reaper's removals reported to the pass; and doctor's line for unrecorded trees.
   - **Tests:**
     - a closed effect's tree goes, with its bytes recorded;
     - a parked run's open effect keeps every tree;
     - unproven cleanup keeps its tree;
     - a link at the path, and a tree on another filesystem, each refuse;
     - a partial removal records nothing;
     - an absent path closes as gone;
     - a second sweeper is refused by the lock;
     - `--dry-run` writes nothing;
     - a hands tree whose owner lives is kept;
     - a tree no row names is listed by doctor and never removed.
4. **Then slice 6**, `brokkr dispatch` and its standing loop. It calls the sweep on every pass.

## Consequences

- The Spark ruling is written once, in the machine's host file. night-shift's
  hand-written 7200 can be removed once 5a lands; this decision does not edit
  it.
- Every tree Brokkr makes for an attempt has a journal row. Once its effect is
  closed and its processes are proven over, it is removed with a record of
  what it freed. Trees an older engine made are listed, never removed.
- This does not fix the volume seats write outside Brokkr's trees. That is
  named as the next question, not hidden in this one.
- The price: `forge.host/v3`, two append-only tables in queue storage v5, one
  verb, and one host read per attempt.

## Evidence

- The 2026-09-21 and 2026-11-02 `/tmp` measurements and the 2026-11-06
  scratch measurement were taken by the controller on the house workstation
  and recorded in the session that took them. The 2026-09-23 sixteen
  gigabytes are 0068's.
- The line references above are at be0b6a871.
