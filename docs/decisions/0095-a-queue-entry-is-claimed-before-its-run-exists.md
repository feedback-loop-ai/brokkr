# 0095 — A queue entry is claimed before its run exists, by the one dispatcher a journal admits

Status: proposed
Date: 2026-10-23

Built: unbuilt (#430)
Amends: 0068

## Context

This is #430's third slice: "Fence queue claims so concurrent dispatch
processes or restart cannot launch the same entry twice. Reconcile a
crash between claim, durable run creation and spawn without inventing
success or duplicating effects. Define how directly started runs are
accounted for; preserve the decision's direct `brokkr run` path."
Decision 0068 rules the queue, its order and its admission. It says
nothing about claims, leases, fencing or a crash between them, so the
build stops and asks (decision 0082). This decision proposes the
answers. It builds nothing. The operator queued it on 2026-10-22
("dispatcher after the queue").

**What main holds, measured at 42b9ab93.**

- **The queue's storage.** `queue_entries` (`entry_id` primary key,
  `payload`, `priority`, `added_at`, `state`, `position`, and `run_id
  TEXT UNIQUE REFERENCES runs`), `queue_waits` and the append-only
  `queue_commands` (`crates/brokkr-store/src/queue/storage.rs:52-99`).
  Version 2 adds `queue_latches` and `queue_pins`, each row keyed by the
  command that wrote it (`:108-131`). `QUEUE_SCHEMA` is 2 (`:44`).
  Triggers fix an entry's launch, priority and arrival, write `run_id`
  once (`queue_entries_run_once`, `:80-83`) and delete no entry.
- **Three stored states.** `queued`, `held` and `dropped`
  (`crates/brokkr-store/src/queue.rs:152-171`). `Claimed { run }` is
  read from a non-null `run_id` (`:211-220`). `claim_once`
  (`:560-594`) takes an `Immediate` transaction, requires the entry
  `Queued`, requires the run row to exist already (`:569-571`), refuses
  `RunTaken`, and writes no `queue_commands` row. No production code
  calls `queue_claim` (`:688-697`); it names this slice as its caller.
- **What a pass measures.** `QueuedLaunch::rebuild`
  (`crates/brokkr-runtime/src/launch/queued.rs:301-332`) has one
  production caller, measurement's `compiled()` (`:339-348`), reached
  from `brokkr queue list` and `judge` (`verbs/queue.rs:123`, `:140` →
  `admission.rs:433`, `:443` → `capacity.rs:269-271`, `:302-304`), and
  it keeps only the map of the `NewRun` it rebuilds. No production start
  runs through `rebuild`. The pass compiles the recipe from disk
  (`Entry::measure`, `capacity.rs:505-515`) and counts what its manifest
  pins (`Seats::of`, `:433-461`); a start compiles it again (`admit_new`,
  `launch.rs:281`, `:381-388`). The payload holds a bundle path, not a
  digest, and `Seen` (`queue/beside.rs:19-23`) only pin and latch seqs.
  `Bundle::manifest_digest` (`crates/brokkr-runtime/src/bundle.rs:1074`)
  is what an envelope is verified against (`engine.rs:656`).
- **The start path.** `launch::start`
  (`crates/brokkr-runtime/src/launch.rs:271-295`) calls
  `Engine::start_in_world`, which mints a random `<slug>-<uuid8>` run id
  (`crates/brokkr-runtime/src/engine.rs:542`), writes the run row with
  `create_run` in one transaction (`:615`) and appends `run/started` in
  another (`:616-622`). `start_with_dispatch` does the same with the run
  id a forge-dispatch/v2 envelope names (`:654`, `:660-667`). Drivers
  spawn later, in `drive` (`:765`). `create_run` refuses a run id that
  exists (`refuse_existing`, `crates/brokkr-store/src/lib.rs:231-241`).
- **Admission.** `pass` and `pass_within`
  (`crates/brokkr-runtime/src/admission.rs:425-434`) read outside any
  transaction and write nothing. The live-run census (`census`,
  `crates/brokkr-runtime/src/admission/capacity.rs:477-494`) folds every
  run of this journal and counts those that read `Running`, so direct
  `brokkr run` runs are counted. Nothing checks liveness: a crashed
  process's run reads `Running` until the operator ends it. `operator
  retry` makes a parked run `Running` by an event, with no new run row
  (`crates/brokkr-core/src/fold/operator.rs:39`). `events` is a rowid
  table that refuses update and delete (`brokkr-store/src/schema.rs:33-45`),
  so its highest rowid only rises.
- **Store writes.** `Immediate` transactions under `patiently`
  (`crates/brokkr-store/src/lib.rs:758-779`): `SQLITE_BUSY` is retried
  to a deadline, then refused as `Contended` with nothing written. The
  store's header calls its rule "optimistic fencing instead of a lease
  service" (`lib.rs:9`). The queue's one such fence is the `Seen` check
  on latches and pins (`crates/brokkr-store/src/queue/beside.rs:60-84`).
- **OS locks already in the tree.** The broker's admission lock opens a
  file of this user's own with one link, `O_NOFOLLOW` and `O_CLOEXEC`,
  and takes a non-blocking exclusive `flock` until a deadline
  (`crates/brokkr-runtime/src/engine/broker/root.rs:218-245`); `recipes
  add` holds one on its library (`crates/brokkr-cli/src/recipes.rs:380-392`).
  Both use `rustix`, on Linux and macOS alike.
- **The CLI.** `brokkr queue add|list|judge|move|hold|release|repin|drop`
  with its arguments in `crates/brokkr-cli/src/cli_args/queue.rs`
  (96 lines). There is no `dispatch` verb. `cli_args.rs` is 712 lines.
  `queue add` flattens `LaunchArgs`, which carries `--dispatch`, a
  forge-dispatch/v2 envelope pinned into a run-manifest/v2
  (`crates/brokkr-cli/src/cli_args.rs:196-226`).
- **The manifest.** Run-manifest v13 is the latest, and it is closed
  (`"additionalProperties": false`,
  `contracts/run-manifest.v13.schema.json:8`). No manifest names a
  queue entry.

**What fails today.** Each of these follows from the code above.

1. Two processes that launch one entry both create and start a run.
   Only one claim wins, and the other run is already driving.
2. A crash between `create_run` and `run/started` leaves a run row with
   no events. Its fold is `FoldError::Empty`
   (`crates/brokkr-core/src/fold.rs:423`). The census then fails the
   whole pass (`capacity.rs:480-485`), and so does `observe` for an entry
   that waits on it (`admission.rs:647-650`).
3. A crash after `run/started` and before the claim leaves an orphan run
   reading `Running` and the entry `Queued`. The next pass launches it
   again, and no manifest says which entry the orphan was.
4. An entry queued with `--dispatch` starts a run-manifest/v2 run, whose
   contract is a counterpart system's and has no queue field.

## Rulings

1. **An entry is claimed before its run exists.** Queue storage v3 adds
   the stored state `launching` and the table `queue_claims`, one row per
   `claim` command keyed by its seq, as latches and pins are. A claim row
   holds the entry, the dispatcher generation of ruling 2, the manifest
   digest of the bundle the pass admitted (ruling 7) and, once linked,
   the run. A start from the queue has four durable steps, each written
   before what it stands for:

   1. **claim**: `queued` becomes `launching`, and the `claim` command
      and its claim row are written in one `Immediate` transaction. The
      token is the claim's seq.
   2. **start**: the run row and its `run/started` are written with the
      claim in the manifest (ruling 5), by ruling 4's one transaction,
      which runs the claim's fence inside it.
   3. **link**: `run_id` is written (the existing once-only trigger
      holds it), and the `link` command records it. The entry reads
      `Claimed { run }` as today.
   4. **spawn**: the `spawn` command is written, then the driver
      process starts.

   Every step is a `queue_commands` row whose operator is the word
   `dispatch` and whose reason states what the dispatcher saw. A step
   whose token is not the claim standing on the entry refuses, writing
   nothing. The new command words (`claim`, `link`, `reclaim`, `spawn`)
   are read into one closed enum (decision 0071 ruling 2), and an
   unknown stored word refuses the read as `read_word` does today. The
   existing `queue_claim` is replaced, not kept beside (0071 ruling 6).

   **Enforcement binding:** the v3 migration's tables and triggers in
   `crates/brokkr-store/src/queue/storage.rs`, with a trigger that
   refuses `run_id` on an entry that is not `launching`. Store tests
   pin each refusal's variant and text once, and a migration test opens
   a v2 journal and reads its order, holds, latches, pins and links
   unchanged at v3.

2. **One dispatcher per journal, held by an OS lock.** `brokkr dispatch`,
   standing or one pass, takes an exclusive non-blocking `flock` on
   `<journal>.dispatch.lock` beside the journal. The file is this user's
   own regular file of one link, opened `O_NOFOLLOW | O_CLOEXEC`, as the
   broker's is, so a driver it spawns never inherits the lock. Holding
   the lock, it appends a holder row to `dispatch_holders`: generation
   (rising on every start), host (the store's own fingerprint,
   `crates/brokkr-store/src/origin.rs:26`), pid and start time. A
   second dispatcher refuses at once with a typed error naming the
   holder it reads there:

   ```text
   dispatch: the journal <path> is dispatched by generation <n> (pid <pid> on this host since <time>); one dispatcher per journal (decision 0095)
   ```

   Liveness is the lock: the kernel drops it when the process dies.
   There is no lease expiry, no heartbeat and no clock read for
   liveness; the start time is for the operator. Every claim, start,
   link, reclaim and spawn also refuses unless its generation is the
   latest holder row, so a lock file removed and recreated under a
   live dispatcher cannot admit a second one past the journal, and the
   dispatcher fenced out writes no run (ruling 4).

   **Enforcement binding:** a runtime test holds the lock in one handle
   and asserts the second's refusal and text. A test whose child
   process takes the lock and is killed shows the next dispatcher
   taking it. A store test claims with a stale generation and is
   refused by variant.

3. **A restart reconciles under the lock, before its first pass.** For
   each entry left `launching`, the dispatcher looks for a run whose
   manifest carries that entry's standing claim (ruling 5), by one store
   query over `runs.manifest` read as `ENGINE_OF_RUN` reads `engine`
   (`crates/brokkr-store/src/lib.rs:291-300`).

   - A run carries it: the entry is linked to that run, adopted and
     never relaunched, never stopped and never concluded (0068 ruling
     7). The `link` row's reason says so.
   - No run carries it: the entry goes back to `queued` at its place, by
     a `reclaim` command whose reason names the claim it ends. The next
     claim is a new seq, so a run that carries the old one is never
     adopted.
   - An entry linked with no `spawn` row is spawned: such a run started
     no driver, so nothing it does is repeated.
   - An entry with a `spawn` row is never spawned again. If its driver
     died, its run reads `Running`, as a crashed `brokkr run` does, and
     the operator resumes it.

   **Enforcement binding:** one crash test per durable boundary (after
   claim, start, link and the `spawn` row), each a fault seam that ends
   the pass there, then a second dispatcher's reconciliation asserted by
   the queue's state, its command rows and the runs that exist: #430's
   "crash tests cover each durable boundary".

4. **A run row never stands without `run/started`, and one written
   before this rule is named, not fatal.** The store gains one
   transaction that writes the run row and its `run/started`, and both
   starts in `engine.rs` call it in place of their two
   (`engine.rs:615-622`, `:660-667`). Then no crash leaves a run with
   no events, from the queue or from `brokkr run`. It runs a fence, a
   closure (0071 ruling 2), before either write: a dispatched start
   passes the queue's `claim_stands`, which refuses unless the entry is
   `launching` under this claim, its generation is the latest holder
   row and the bundle has the claim's digest (ruling 7). The queue owns
   what the fence reads; `brokkr run` passes none. So step 2's token
   check is inside its transaction, and a dispatcher fenced out by
   ruling 2 leaves no run unlinked. A row an older binary left with no
   events is read by the census and by `observe` as an unstarted run:
   named in the pass's output as its own reason (`unstarted_run`),
   counted against no ceiling, since it spawned nothing, and the pass
   judges every other entry. An entry that waits on one is held.

   **Enforcement binding:** store tests assert the run row and its first
   event land together or not at all, and that a start under a reclaimed
   claim or a superseded generation refuses by variant with no run row.
   An admission test plants a run row with no events (by SQL, as the
   store's fixtures do) and asserts the pass completes and names it.

5. **The run carries its claim: run-manifest v14.** v14 is v13 with
   one optional `queue_entry`, `{"entry": <id>, "claim": <seq>}`,
   written only into the manifest of a run the dispatcher starts. It is
   pinned at start beside the realms pin (`World::pinned`,
   `crates/brokkr-runtime/src/realms.rs:756`). It is never in a compiled
   bundle, so resume reads it as it reads that pin, never as drift. A
   direct `brokkr run` writes none. This is 0068 ruling 1's "A run
   started from the queue carries its entry id in its manifest", and the
   claim makes the run a reconciliation adopts unambiguous.

   **Enforcement binding:** `contracts/run-manifest.v14.schema.json`
   beside v13, with v13 added to `FROZEN`
   (`crates/brokkr-runtime/tests/frozen_contracts.rs:27`) as each prior
   version was. The manifest test at
   `crates/brokkr-runtime/src/engine/tests.rs:4159-4164` validates a
   dispatched run against v14, and a resume test resumes one with no
   drift.

6. **Direct runs stay outside the queue.** `brokkr run` starts a run as
   today, takes no lock and needs no dispatcher. The census counts it
   against capacity because it reads `Running` in the same journal.
   Runs in other journals on the same machine are not counted. Decision
   0074's many-host claims are its own, and deferred.

   **Enforcement binding:** judgment-guidance, with the existing census
   tests (`crates/brokkr-runtime/src/admission/tests.rs`) standing.

7. **A claim re-checks what the pass measured, not only what it read.**
   A fence on the pass's reads (entry state, `Seen`, waits, the `runs`
   rowid) misses a recipe or adapter edit that moves a seat to a
   provider at its ceiling, since the start compiles again, and a
   parked run made `Running` by `operator retry`, which adds no `runs`
   row. Each would start a run past 0068 ruling 3's refusal. They share
   one root: the pass measures the bundle it compiled and the census it
   counted, and a fence on its reads holds neither. This one fence
   answers both:

   - **The entry still stands.** Inside the claim's `Immediate`
     transaction, the entry is `queued`, its pin and latch are the ones
     the pass read (`beside.rs:60-84`) and its waits hold.
   - **The census is re-counted where it moved.** The pass reads the
     highest `events` rowid before its census. Inside the claim's
     transaction, each run with an event above it is folded again, the
     census is corrected by them, and the entry's room is checked again
     by the lanes it measured. A retried, new (ruling 4) or ended run
     has such an event, so the `runs` rowid adds nothing. A run that
     moved and seats what it seated refuses nothing, so live drivers do
     not starve the claim. The store reads the moved runs and the
     runtime judges them, by a closure the claim takes (`claim_with`,
     0071 ruling 2).
   - **The bundle started is the bundle measured.** The claim row
     records the `manifest_digest` of what the pass compiled. No pass
     admits an entry under a host that declares no capacity (0068's
     2026-10-10 addendum), so every claim names a bundle. The
     start compiles as today, and ruling 4's fence refuses it unless its
     digest is the claim's. The digest, not the per-seat provider set:
     the manifest is what `Seats::of` reads, so its digest covers every
     lane, route, model pin and boundary measured, by a function that
     exists, where a provider set would derive `Seats` a second time
     (0071 ruling 5). An edit that moves no lane also refuses, once.

   A refused claim writes nothing. A refused start writes no run, and
   the dispatcher writes a `reclaim` naming both digests, so the next
   pass judges the entry again. Free space and `host.json` are read by
   the pass and not again inside the write lock. A direct `brokkr run`
   or an `operator retry` after the claim's transaction is the accepted
   race.

   **Enforcement binding:** store tests plant a hold, a re-pin and a
   latch between a pass and its claim and assert each refusal by
   variant. Under a ceiling of 1, runtime tests retry a parked run
   between pass and claim (a capacity refusal), append to a live run
   there (the claim lands), and move a seat to a provider at its ceiling
   between pass and start (the start refuses by variant, no run row, the
   entry `queued` by its `reclaim`). Each binds by a mutation that drops
   its check (0071 ruling 9).

8. **Where the code goes.** `crates/brokkr-store/src/queue.rs` (757
   lines) gains only the `Launching` state and its module lines, and
   `claim_once` leaves it. `crates/brokkr-runtime/src/admission.rs`
   (769) gains only its reason variants. New homes:

   - `crates/brokkr-store/src/queue/claim.rs`: claim, link, reclaim,
     spawn and the reconciliation query; v3's migration joins
     `storage.rs` (237).
   - `crates/brokkr-store/src/queue/holder.rs`: `dispatch_holders`.
   - A child of `crates/brokkr-store/src/lib.rs` (790): the one
     transaction of ruling 4.
   - `crates/brokkr-runtime/src/dispatch.rs`, with `dispatch/lock.rs`
     and `dispatch/reconcile.rs`. The broker's open-and-check of its
     lock file (`root.rs:218-245`) moves to one shared home that both
     call, never a copy (0071 ruling 5).
   - `crates/brokkr-runtime/src/admission/capacity.rs` (637): the
     census's unstarted run, and the digest, the `events` high-water and
     the census the pass measured, which ruling 7's claim re-checks.
   - `crates/brokkr-runtime/src/launch.rs` (607): `NewRun` carries the
     claim. `rebuild` gives none (the payload holds none, and
     `compiled()` keeps only the map); the dispatcher sets it on the
     `NewRun` it rebuilds, the first start through `rebuild`.
     `engine.rs` (3494) changes at its two starts and shrinks.
   - `crates/brokkr-cli/src/cli_args/dispatch.rs` and
     `crates/brokkr-cli/src/verbs/dispatch.rs`, as #815 moved the
     queue's arguments to `cli_args/queue.rs`.

## Slices

Each lands alone, tested, through the merge queue.

- **3a: the claim and its storage.** Queue storage v3, `launching`,
  `queue_claims`, the four command words, their refusals and texts, the
  migration test. Ruling 1, and ruling 7's fence on the entry and the
  census, with the digest on the claim row.
- **3b: the lock and its holder record.** Ruling 2, with the shared
  lock-file home.
- **3c: one start transaction and the unstarted run.** Ruling 4, with
  its claim fence and ruling 7's digest check at the start.
- **3d: the manifest field.** Run-manifest v14 and its pin. Ruling 5.
- **3e: reconciliation.** Ruling 3 and its crash tests at each boundary.
- **Then #430's slices 4 to 6, unchanged.** Cooldowns and probes;
  `shared-local` capacity and the scratch sweep; the `brokkr dispatch`
  command and its standing loop, which first calls the claim, start,
  link and spawn these slices build.

## Asked of the operator

Each choice is listed with its recommendation first.

- **When the entry is claimed.** (a) Before the run exists: `launching`
  with its token, then the run with the claim in its manifest, then the
  link, each journaled (ruling 1). (b) The run id derived from the entry
  (`q<entry>-<attempt>`), so `runs`' key makes a second creation
  collide, keeping today's claim after create. A run id named from
  outside already exists for envelopes (`engine.rs:654`). (c) Claim
  after create, as today, and reconcile duplicates afterwards. Not
  recommended: a duplicate may already be driving. (d) Claim, run row,
  `run/started` and link in one transaction. This needs no `launching`
  state, but makes the store's run creation write the queue's state,
  where (a)'s start only calls the queue's fence, and a crash during
  the start's compile leaves no record that a start was tried.
- **What fences two dispatchers.** (a) One per journal, by an OS lock
  beside it plus a journaled holder record with a rising generation
  (ruling 2). (b) A lease row with an expiry renewed by heartbeat: more
  parts, and a clock. (c) Optimistic fencing on a claim sequence,
  `Seen`-style, with no exclusion of a second dispatcher.
- **Crash reconciliation.** (a) Adopt a `launching` entry whose run
  exists, requeue one whose run does not by a journaled `reclaim`, and
  spawn a linked run with no `spawn` row (ruling 3). (b) Hold such
  entries for the operator instead of returning them to the queue.
- **A run row with no `run/started`.** (a) Close the window with one
  start transaction, which `brokkr run` uses too and which runs the
  claim's fence for a dispatched start, and have the census name an
  older row as an unstarted run, counted against nothing, while it
  judges every other entry (ruling 4). (b) Count it as a live run
  against capacity. (c) Skip it silently. Not recommended: that fails
  open. (d) Name it without closing the window, so a crash keeps
  making them.
- **The run's link to its entry.** (a) Run-manifest v14's optional
  `queue_entry`, `{entry, claim}`, written only by the dispatcher
  (ruling 5). (b) A store-side table alone, with no manifest field.
  That departs from 0068 ruling 1. (c) The entry id alone. Then
  reconciliation cannot tell apart two runs started for one entry
  across a reclaim.
- **Direct runs.** (a) Outside the queue, counted by the census of their
  own journal (ruling 6).
- **Capacity between the pass and the claim.** (a) Under one
  dispatcher, the claim re-checks the entry's standing and what the pass
  measured inside its transaction: the census, re-counted for each run
  with an event past the pass's `events` high-water, and the bundle's
  manifest digest, which the start's fence holds it to. A direct run or
  a retry after the claim is the accepted race (ruling 7). (b) Refuse
  the claim on any event past the high-water: no fold under the write
  lock, but a journal with live drivers keeps refusing claims. (c) Hand
  the pass's compiled bundle to the start instead of compiling again:
  no digest to check, but the run starts bytes read before its claim,
  and an edit since is first seen as drift at resume.
- **An entry queued with `--dispatch`.** Found in the code, not in the
  issue. (a) The pass holds it with the reason `dispatch_envelope`. Its
  run would be a run-manifest/v2, the counterpart system's contract,
  which carries no `queue_entry`. (b) Start it, linked by `run_id`
  alone. Its envelope's fixed run id already makes a second creation
  collide (`lib.rs:231-241`), but 0068 ruling 1 is departed from for
  that lineage. (c) `queue add` refuses `--dispatch`, which leaves
  entries already queued with one to (a).
- **The contract.** Run-manifest v14, as ruling 5 states it.
- **Queue storage v3.** `launching`, `queue_claims` and
  `dispatch_holders`. A v2 journal is migrated by the first open that
  may write, as v1 is today. A v3 journal read by an older binary is
  refused by the queue verbs alone, as `queue_stored` refuses an
  unknown version (`storage.rs:231-237`). The `entries` reader's
  fallback arm (`queue.rs:605-608`) is matched by version in the build,
  not by a wildcard.
- **Public API, estimated.** `brokkr-store`: about twelve items (the
  `Launching` state, the claim token, four step methods, the holder
  record and its reader, the one start transaction and its fence, the
  new refusal variants). `brokkr-runtime`: about ten (the dispatcher's
  lock, reconciliation and step entry points, `DispatcherError`, named
  apart from `brokkr_core::dispatch::DispatchError`
  (`crates/brokkr-core/src/dispatch.rs:148`), two admission reasons).
  `brokkr-core` and `brokkr-view`: none predicted. Each crate's
  `quality/public-api/` snapshot moves in its slice, by this ruling.
- **Witness digests, predicted and not measured.** None. The field is
  pinned at start, never compiled into a bundle, so no row of
  `crates/brokkr-runtime/tests/witnesses.json` moves, and a direct run's
  `run/started` payload is unchanged. `FROZEN` gains v13. If the build
  finds a digest moves, it re-pins from the tests' own values and names
  each in its pull request.

## Consequences

- Competing dispatchers cannot both admit: the second refuses at its
  start. Within one dispatcher, an entry is started once, and a restart
  finds every start in flight by its claim.
- `brokkr queue list` shows `launching` for an entry claimed and not
  yet linked, so a crash in flight is visible before a dispatcher runs
  again.
- Liveness is the lock. A journal on a filesystem where `flock` does not
  exclude, such as some network mounts, is outside this guarantee, as
  the store's own SQLite locking already is.
- A driver that dies after its `spawn` row leaves its run `Running`,
  counted against capacity until the operator resumes or concludes it.
  The dispatcher never does either (0068 ruling 7). Driver liveness is
  #403's to rule, not this decision's.
- Ruling 7 closes two races between a pass and its start: an edit that
  moves a seat to a provider at its ceiling (the start's digest is not
  the claim's) and a parked run retried (its event is past the
  high-water). An edit that moves no lane is refused too, once. Ruling
  4's fence leaves a dispatcher fenced out no run to orphan.
- The claim holds the write lock while it folds the runs that moved
  since the pass. A driver's append waits under `patiently`, and past
  its deadline is refused as `Contended`, as any contended write is.
- The accepted races: a direct `brokkr run` or an `operator retry` that
  lands after the claim's transaction can take capacity the claim
  counted on, and free space and `host.json` are read by the pass
  alone. The next pass sees both runs.
- The start transaction of ruling 4 changes `brokkr run` too: its run
  row and first event land together. A journal written by an older
  binary may still hold an unstarted run, which the census names.
- The word "dispatch" now names the command and stays the name of the
  forge-dispatch/v2 `--dispatch` flag. The lock file and the holder
  table are named for the command.
- Only the operator accepts this proposal. Filing it builds nothing,
  moves no digest, and certifies no host run, remote CI or publication.
