# 0095 — A queue entry is claimed before its run exists, by the one dispatcher a journal admits

Status: accepted — operator ruled 2026-10-23 (PR #849), every choice asked taken as recommended except 8, taken as (b) with a per-journal bound of 3 unless set
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
build stops and asks (decision 0082). This decision gives the answers
the operator ruled on 2026-10-23, recorded on #430. It builds nothing.
The operator queued it on 2026-10-22 ("dispatcher after the queue").

**What main holds, measured at 9be5d53d.**

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
  `Bundle::manifest_digest` (`crates/brokkr-runtime/src/bundle.rs:1031`)
  is what an envelope is verified against (`engine.rs:576`).
- **The start path.** `launch::start`
  (`crates/brokkr-runtime/src/launch.rs:271-295`) calls
  `Engine::start_in_world`, which mints a random `<slug>-<uuid8>` run id
  (`crates/brokkr-runtime/src/engine/start.rs:86`, reached from
  `start::door` at `crates/brokkr-runtime/src/engine.rs:534`), writes
  the run row with `create_run` in one transaction (`:535`) and appends
  `run/started` in another (`:536-542`). `start_with_dispatch` does the
  same with the run id a forge-dispatch/v2 envelope names (`:574`,
  `:580-587`). Drivers spawn later, in `drive` (`:685`). `create_run`
  refuses a run id that exists (`refuse_existing`,
  `crates/brokkr-store/src/lib.rs:236-246`).
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
  (`crates/brokkr-store/src/lib.rs:759-780`): `SQLITE_BUSY` is retried
  to a deadline, then refused as `Contended` with nothing written. The
  store's header calls its rule "optimistic fencing instead of a lease
  service" (`lib.rs:9`). The queue's one such fence is the `Seen` check
  on latches and pins (`crates/brokkr-store/src/queue/beside.rs:60-84`).
- **OS locks already in the tree.** The broker's admission lock opens a
  file of this user's own with one link, `O_NOFOLLOW` and `O_CLOEXEC`,
  and takes a non-blocking exclusive `flock` until a deadline
  (`crates/brokkr-runtime/src/engine/broker/root.rs:218-242`); `recipes
  add` holds one on its library (`crates/brokkr-cli/src/recipes.rs:380-392`).
  Both use `rustix`, on Linux and macOS alike.
- **The CLI.** `brokkr queue add|list|judge|move|hold|release|repin|drop`
  with its arguments in `crates/brokkr-cli/src/cli_args/queue.rs`
  (96 lines). There is no `dispatch` verb. `cli_args.rs` is 712 lines.
  `queue add` flattens `LaunchArgs`, which carries `--dispatch`, a
  forge-dispatch/v2 envelope pinned into a run-manifest/v2
  (`crates/brokkr-cli/src/cli_args.rs:196-226`).
- **The manifest.** Run-manifest v13, the latest, is closed
  (`contracts/run-manifest.v13.schema.json:8`) and names no queue entry.

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
      holds it), and the `link` command records it, in one `Immediate`
      transaction. The entry reads `Claimed { run }` as today.
   4. **spawn**: the `spawn` command is written in one `Immediate`
      transaction, then the driver process starts.

   Every step is a `queue_commands` row whose operator is the word
   `dispatch` and whose reason states what the dispatcher saw. A step
   whose token is not the claim standing on the entry refuses, writing
   nothing; each step checks its token and generation inside its own
   transaction, so no step lands half. The new command words (`claim`,
   `link`, `reclaim`, `spawn`) are read into one closed enum (decision
   0071 ruling 2), and an unknown stored word refuses the read as
   `read_word` does today. The existing `queue_claim` is replaced, not
   kept beside (0071 ruling 6). The dispatcher's own `hold` of a refused
   start (ruling 7) is a step under its token, as `reclaim` is. An
   operator command (`move`, `hold`, `release`, `repin`, `drop`), or
   admission's `latch`, on a `launching` entry refuses as
   `QueueRefusal::Launching`, by a `Launching` arm in `admit`
   (`crates/brokkr-store/src/queue.rs:452-462`) and `left_the_queue`
   (`queue/beside.rs:52-58`), never a wildcard, until the dispatcher
   links, reclaims or holds it or the next one reconciles it (ruling 3).

   **Enforcement binding:** the v3 migration's tables and triggers in
   `crates/brokkr-store/src/queue/storage.rs`, with a trigger that
   refuses `run_id` on an entry that is not `launching`. Store tests
   pin each refusal's variant and text once, including each operator
   command's `Launching` refusal, and a migration test opens a v2
   journal and reads its order, holds, latches, pins and links unchanged
   at v3.

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
   liveness; the start time is for the operator. Every step (ruling 1,
   and the dispatcher's `reclaim` and `hold`) also refuses unless its
   generation is the latest holder row, so a lock file removed and
   recreated under a live dispatcher cannot admit a second one past the
   journal, and the dispatcher fenced out writes no run (ruling 4).

   **Enforcement binding:** a runtime test holds the lock in one handle
   and asserts the second's refusal and text. A test whose child
   process takes the lock and is killed shows the next dispatcher
   taking it. A store test claims with a stale generation and is
   refused by variant.

3. **A restart reconciles under the lock, before its first pass.** For
   each entry left `launching`, the dispatcher looks for a run whose
   manifest carries that entry's standing claim (ruling 5), by one store
   query over `runs.manifest` read as `ENGINE_OF_RUN` reads `engine`
   (`crates/brokkr-store/src/lib.rs:296-305`).

   - A run carries it: the entry is linked to that run, adopted and
     never relaunched, never stopped and never concluded (0068 ruling
     7). The `link` row's reason says so.
   - No run carries it: the entry goes back to `queued` at its place, by
     a `reclaim` command whose reason names the claim it ends. The next
     claim is a new seq. No `reclaim` ends a claim whose run exists:
     this one finds none, and the dispatcher's own follows only a start
     that wrote no run (ruling 4), never a `Contended` link or spawn
     (ruling 7). So no run carries a reclaimed claim, and none is
     adopted under the next.
   - An entry linked with no `spawn` row is spawned: such a run started
     no driver, so nothing it does is repeated.
   - An entry with a `spawn` row is never spawned again. If its driver
     died, its run reads `Running`, as a crashed `brokkr run` does, and
     the operator resumes it.
   - A standing dispatcher that left an entry of its own `launching`, or
     linked with no `spawn` row, because a link or spawn was `Contended`
     past its patience (ruling 7), reconciles before its next pass by these same
     rules, under its own generation, as a restart does before its
     first.

   **Enforcement binding:** one crash test per durable boundary (after
   claim, start, link and the `spawn` row), each a fault seam that ends
   the pass there, then a second dispatcher's reconciliation asserted by
   the queue's state, its command rows and the runs that exist: #430's
   "crash tests cover each durable boundary".

4. **A run row never stands without `run/started`, and one written
   before this rule is named, not fatal.** The store gains one
   transaction that writes the run row and its `run/started`, and both
   starts in `engine.rs` call it in place of their two
   (`engine.rs:535-542`, `:580-587`). Then no crash leaves a run with
   no events, from the queue or from `brokkr run`. It runs a fence, a
   closure (0071 ruling 2), before either write: a dispatched start
   passes the queue's `claim_stands`, which refuses unless the entry is
   `launching` under this claim, its generation is the latest holder
   row and the bundle has the claim's digest (ruling 7). The queue owns
   what the fence reads; `brokkr run` passes none. A dispatcher fenced
   out by ruling 2 leaves no run unlinked. A row an older binary left
   with no events is read by the census and by `observe` as an
   unstarted run: named in the pass's output as its own reason
   (`unstarted_run`), counted against no ceiling, since it spawned
   nothing, and the pass judges every other entry. An entry that waits
   on one is held. The pass also holds an entry queued with
   `--dispatch`, with the reason `dispatch_envelope`: its run would be a
   run-manifest/v2, the counterpart system's contract, which carries no
   `queue_entry` (ruling 5).

   **Enforcement binding:** store tests assert the run row and its first
   event land together or not at all, and that a start under a reclaimed
   claim or a superseded generation refuses by variant with no run row.
   An admission test plants a run row with no events (by SQL, as the
   store's fixtures do) and asserts the pass completes and names it,
   and another asserts an entry queued with `--dispatch` is held with
   the reason `dispatch_envelope`.

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
   version was. The manifest test at `engine/tests.rs:4159-4164`
   validates a dispatched run against v14, and a resume test resumes one
   with no drift.

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
   row. Each would start a run past 0068 ruling 3's refusal: the pass
   measures the bundle it compiled and the census it counted, and a
   fence on its reads holds neither. This fence answers both, and its
   last part a race within one pass:

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
   - **One entry at a time.** The pass judges every entry against one
     census (`sized`, `capacity.rs:255-276`), and a claim writes no
     event, so two entries admitted in one pass could both start past a
     ceiling. Within one dispatcher, claim, start, link and spawn, or
     the claim, its refused start and that refusal's `reclaim` or
     `hold`, complete for one entry before the next claim, and a claim
     refuses as `InFlight` while another entry stands `launching`. The
     reclaim or hold ends the entry's `launching` in its own
     transaction, so a counted refusal frees the next claim. The next
     claim's re-fold then counts the prior start's run by its
     `run/started` above the high-water, with no second count of lanes
     (0071 ruling 5). A dead holder's `launching` entry refuses fresh
     claims the same way until reconciliation, before the first pass,
     adopts it (its run counted) or reclaims it (nothing ran); its stale
     generation cannot start it (ruling 2), and an entry reconciliation
     cannot settle leaves claims refused.

   A refused claim writes nothing. A refused start writes no run, and
   the dispatcher disposes of it by variant, as a step under its token
   (ruling 1):

   - **A digest refusal** writes a `reclaim` naming both digests, so the
     next pass judges what it compiles then. It does not count.
   - **A `Contended` start** writes a `reclaim`. It does not count.
   - **A refusal by the start's admission** (the crossing fence, the
     compile or `refuse_unboxable`, `launch.rs:381-388`, which
     `compiled()` leaves to the start, `queued.rs:334-338`) may recur on
     the same bytes, so it counts. Its `reclaim`'s reason names the
     refusal's variant, and v3's append-only `queue_refusals` records
     it: one row per counted refusal, written in the same transaction
     as the `reclaim` or `hold` command that disposes of the refusal and
     keyed by that command's seq, as latches and pins are, holding the
     entry and the variant's word, read into a closed enum (0071 rulings
     2 and 3). The count is therefore a count of those commands' typed
     refusal rows, the refusals counted on the `reclaim` rows as the
     operator ruled, not a second counter (0071 ruling 5). The entry's
     count is its `queue_refusals` rows above the
     seq of its latest `release`, or all of them if it was never
     released, read in the disposal's own transaction, so no counter
     column can drift from the journal (0071 ruling 5). The count
     includes this refusal. While it is below the journal's bound, the
     dispatcher writes the `reclaim`; once it is at or above the bound,
     it writes a `hold` whose reason is the refusal's variant, for the
     operator, in the reclaim's place. Under the default bound of 3,
     the first two refusals reclaim and the third holds; under a bound
     of 1, the first holds. The count resets at a release, so a release
     gives a fresh bound; counting since the entry was queued would
     instead hold a released entry at its next refusal. The
     digest-refusal and `Contended` reclaims write no `queue_refusals`
     row, so they neither count nor reset.
   - **A `Contended` link or spawn** (ruling 1's steps 3 and 4) is not
     a refusal of the start: its run exists. The store retries it under
     `patiently`, and past its patience the dispatcher leaves the entry
     as it stands, `launching` or linked with no `spawn` row, for the
     next reconciliation (ruling 3), which links or spawns it. It is
     never reclaimed.
   - **A `Contended` disposal is retried until it lands.** A refused
     start's `reclaim` or `hold` and, for a counted refusal, its
     `queue_refusals` row are one transaction, so contention on any of
     them writes none of them. The dispatcher is alive and knows the
     refusal, so it writes that same transaction again, each attempt
     under `patiently`, and makes no other claim until it lands: the
     entry stands `launching`, another entry's claim refuses as
     `InFlight`, and the pass does not advance. Contention at the
     disposal is the journal's write lock, held by another writer, and
     has nothing to do with the refusal's bytes, so it is never left to
     reconciliation while the dispatcher lives; reconciliation's
     `reclaim` would not count. During the retry, an operator command
     on the entry refuses as `Launching` (ruling 1), and a second
     dispatcher refuses at the lock (ruling 2). Only a dispatcher that
     dies before the disposal lands, or one fenced out by a newer
     generation (ruling 2), whose retry then refuses with nothing
     written and which ends as a dead one does, leaves the entry to
     reconciliation's uncounted `reclaim` (ruling 3). That is the one
     attempt a crash costs, indistinguishable from a crash between claim
     and start, and each further attempt past the bound needs a further
     crash.

   The retry stalls only this dispatcher's queue work, which one entry
   at a time already orders behind the disposal. It cannot deadlock: an
   `Immediate` transaction that cannot begin holds no lock, so the
   writer holding the journal's lock never waits on the dispatcher, and
   no writer but the dispatcher takes the dispatch lock. A driver's
   append, an operator's queue command on another entry and a direct
   `brokkr run` still land under their own `patiently`. A writer that
   never yields stalls every writer alike, the next claim included, and
   stopping the dispatcher then is the crash above.

   **The bound is a per-journal setting, 3 unless set.** Its home is
   v3's append-only `dispatch_settings`: one row per setting command,
   keyed by its seq, holding the bound, the operator, the reason and
   the time, the latest row the one in force. This is the smallest home
   that fits: a queue command row names an entry and the bound names
   none, `meta` keeps neither who set it nor why, and a column on
   `dispatch_holders` would lapse with the holder's generation. The
   operator sets it by a journaled command with its reason, as 0068
   rules every operator command: a `dispatch` subcommand taking the
   bound and `--reason`, its spelling the build's. It accepts a whole
   number of at least 1, and 1 holds on the first refusal, as (a)
   would. The command refuses 0 and anything that does not read as such
   a number, and a `CHECK` on the column refuses one written past it.
   The dispatcher reads the latest row at the start of each pass, before
   any claim, and a row it cannot read as a bound of at least 1 refuses
   the pass with a typed error, claiming nothing: no default stands in
   for a bound that is set and unreadable. A journal with no row has a
   bound of 3. A bound changed during a pass governs from the next.

   The operator ruled the count, the bound and its default. Where the
   bound and the counted refusals are stored, the setting command's
   shape, the reset at a release, the retry of a `Contended` disposal,
   and a standing dispatcher's reconciling before its next pass are
   this decision's, made under
   that ruling and the operator's that a `Contended` link or spawn is
   retried, left for reconciliation and never reclaimed.

   Free space and `host.json` are read by the pass and not again inside
   the write lock. A direct `brokkr run` or an `operator retry` after
   the claim's transaction is the accepted race.

   **Enforcement binding:** store tests plant a hold, a re-pin and a
   latch between a pass and its claim and assert each refusal by
   variant. Under a ceiling of 1, runtime tests retry a parked run
   between pass and claim (a capacity refusal), append to a live run
   there (the claim lands), and move a seat to a provider at its ceiling
   between pass and start (the start refuses by variant, no run row, the
   entry `queued` by its `reclaim`). With two entries admitted in one
   pass, the second's claim refuses as `InFlight` while the first stands
   `launching` and on capacity after its link, and a planted dead
   generation's `launching` entry refuses a fresh claim until
   reconciled. The bound's tests:

   - three admission refusals of one entry under the default reclaim
     twice and hold on the third, with the variant as the hold's reason;
   - a bound set to 1 holds on the first;
   - a digest refusal and a `Contended` start between two admission
     refusals count for nothing;
   - a release resets the count;
   - the setting command and the column refuse 0, and a pass over an
     unreadable row refuses with no claim.

   A runtime test plants `Contended` on link and on spawn, asserts no
   `reclaim` is written, and asserts that reconciliation then links or
   spawns it. A runtime test plants `Contended` on the disposal of the
   third admission refusal under the default bound, and asserts that no
   further claim, of that entry or another, lands before its `hold` and
   its `queue_refusals` row are written; it binds by a mutation that
   drops the retry, under which the entry is reclaimed uncounted and
   claimed again. Each binds by a mutation that drops its check (0071
   ruling 9).

8. **Where the code goes.** `crates/brokkr-store/src/queue.rs` (757
   lines) gains only the `Launching` state and its module lines, and
   `claim_once` leaves it. `crates/brokkr-runtime/src/admission.rs`
   (769) gains only its reason variants. New homes:

   - `crates/brokkr-store/src/queue/claim.rs`: the steps, the refusal
     count and the reconciliation query; v3's migration joins
     `storage.rs` (237).
   - `queue/holder.rs`: `dispatch_holders` and `dispatch_settings`. A
     child of `crates/brokkr-store/src/lib.rs` (791): ruling 4's
     transaction.
   - `crates/brokkr-runtime/src/dispatch.rs`, with `dispatch/lock.rs`
     and `dispatch/reconcile.rs`. The broker's open-and-check of its
     lock file (`root.rs:218-242`) moves to one shared home that both
     call, never a copy (0071 ruling 5).
   - `crates/brokkr-runtime/src/admission/capacity.rs` (637): the
     census's unstarted run, and the digest, the `events` high-water and
     the census the pass measured, which ruling 7's claim re-checks.
   - `crates/brokkr-runtime/src/launch.rs` (607): `NewRun` carries the
     claim. `rebuild` gives none (the payload holds none, and
     `compiled()` keeps only the map); the dispatcher sets it on the
     `NewRun` it rebuilds, the first start through `rebuild`.
     `engine.rs` (3316) changes at its two starts and shrinks.
   - `crates/brokkr-cli/src/cli_args/dispatch.rs` and
     `crates/brokkr-cli/src/verbs/dispatch.rs`, as #815 moved the
     queue's arguments to `cli_args/queue.rs`.

## Slices

Each lands alone, tested, through the merge queue.

- **3a: the claim and its storage.** Queue storage v3, `launching`,
  `queue_claims`, the four command words, their refusals and texts, the
  migration test. Ruling 1, and ruling 7's fence on the entry and the
  census, its `InFlight` refusal, the digest on the claim row, and
  `queue_refusals` with the count read from it.
- **3b: the lock and its holder record.** Ruling 2, with the shared
  lock-file home, and ruling 7's `dispatch_settings`, its `CHECK` and
  its reader.
- **3c: one start transaction and the unstarted run.** Ruling 4, with
  its claim fence and ruling 7's digest check at the start.
- **3d: the manifest field.** Run-manifest v14 and its pin. Ruling 5.
- **3e: reconciliation.** Ruling 3 and its crash tests at each boundary.
- **Then #430's slices 4 to 6.** Cooldowns and probes; `shared-local`
  capacity and the scratch sweep; the `brokkr dispatch` command and its
  standing loop, which first calls the claim, start, link and spawn
  these slices build, all four for one entry before the next claim
  (ruling 7), disposes of a refused start by its variant, holding at
  the journal's bound and retrying a `Contended` disposal until it
  lands, leaves a `Contended` link or spawn to
  reconciliation, and carries the command that sets the bound.

## Asked of the operator

Each choice is listed with its recommendation first.

- **When the entry is claimed.** (a) Before the run exists: `launching`
  with its token, then the run with the claim in its manifest, then the
  link, each journaled (ruling 1). (b) The run id derived from the entry
  (`q<entry>-<attempt>`), so `runs`' key makes a second creation
  collide, keeping today's claim after create. A run id named from
  outside already exists for envelopes (`engine.rs:574`). (c) Claim
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
- **Capacity between the pass and the claim.** Changed since the story
  run, by fix rounds 430cp2 and 430cp3. (a) Under one dispatcher, the
  claim re-checks the entry's standing and what the pass measured
  inside its transaction: the census, re-counted for each run with an
  event past the pass's `events` high-water, and the bundle's manifest
  digest, which the start's fence holds it to. One entry's claim,
  start, link and spawn complete before the next claim, which refuses
  while another stands `launching`, so each re-fold counts the runs
  started before it. A direct run or a retry after the claim is the
  accepted race (ruling 7). The alternative within (a): the claim's
  census also counts each standing `launching` claim by the lanes of its
  recorded digest. Claims could overlap, but lanes are counted a second
  way, from a digest beside the census (0071 ruling 5). (b) Refuse
  the claim on any event past the high-water: no fold under the write
  lock, but a journal with live drivers keeps refusing claims. (c) Hand
  the pass's compiled bundle to the start instead of compiling again:
  no digest to check, but the run starts bytes read before its claim,
  and an edit since is first seen as drift at resume.
- **A start refused on the same bytes.** New in fix round 430cp3.
  (a) A start refused by its admission (the crossing fence, the compile
  or `refuse_unboxable`) is held by the dispatcher, with the refusal's
  variant as its journaled reason, for the operator to release; a
  digest refusal and `Contended` reclaim (ruling 7). (b) Count refusals
  on the `reclaim` rows and hold at a bound: a passing refusal is
  retried, but each round up to the bound claims and journals again.
  (c) Reclaim every refused start, paced by slice 4's cooldowns. Not
  recommended: a refusal that recurs never ends. The operator ruled
  (b), with a per-journal bound of 3 unless set. The refusals are
  counted on the `reclaim` and `hold` commands that dispose of them, by
  the typed `queue_refusals` row each writes in its own transaction,
  keyed by its seq (ruling 7).
- **An entry queued with `--dispatch`.** Found in the code, not in the
  issue. (a) The pass holds it with the reason `dispatch_envelope`. Its
  run would be a run-manifest/v2, the counterpart system's contract,
  which carries no `queue_entry`. (b) Start it, linked by `run_id`
  alone. Its envelope's fixed run id already makes a second creation
  collide (`lib.rs:236-246`), but 0068 ruling 1 is departed from for
  that lineage. (c) `queue add` refuses `--dispatch`, which leaves
  entries already queued with one to (a).
- **The contract.** Run-manifest v14, as ruling 5 states it.
- **Queue storage v3.** `launching`, `queue_claims`, `queue_refusals`,
  `dispatch_holders` and `dispatch_settings`. Moved by ask 8's ruling:
  `queue_refusals` and `dispatch_settings` are new, each append-only.
  A v2 journal is migrated by the first open that
  may write, as v1 is today. A v3 journal read by an older binary is
  refused by the queue verbs alone, as `queue_stored` refuses an
  unknown version (`storage.rs:231-237`). The `entries` reader's
  fallback arm (`queue.rs:605-608`) is matched by version in the build,
  not by a wildcard.
- **Public API, estimated.** `brokkr-store`: about sixteen items (the
  `Launching` state, the claim token, four step methods, the holder
  record and its reader, the one start transaction and its fence, the
  new refusal variants, and, moved by ask 8's ruling, the counted
  refusal's enum, the refusal count's reader, and the bound's setter
  and reader). `brokkr-runtime`: about ten (the dispatcher's
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
  yet linked, so a crash in flight shows before the next dispatcher.
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
- Ruling 7 also closes a race within one pass. Under a ceiling of 1,
  entries A and B on one provider are both admitted against one census;
  claimed both before either start, neither re-fold sees an event past
  the high-water, and both start. Now B's claim refuses as `InFlight`
  until A is linked, then counts A's run and refuses on capacity. A dead
  holder's `launching` entry refuses fresh claims until reconciled. A
  start its admission refuses is reclaimed and counted, and held at the
  journal's bound, 3 unless the operator sets another, so a refusal
  that recurs on the same bytes claims at most that many times before
  the operator sees it. A release gives a fresh bound. A contended
  disposal is retried before any other claim, so only a dispatcher's
  death before it lands costs an attempt, one per crash.
- No `reclaim` ends a claim whose run exists. A `Contended` link or
  spawn waits for reconciliation, which a standing dispatcher runs
  before its next pass; an entry it leaves `launching` refuses other
  claims until then.
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
- The operator accepted this decision on 2026-10-23. Accepting it
  builds nothing, moves no digest, and certifies no host run, remote CI
  or publication.
