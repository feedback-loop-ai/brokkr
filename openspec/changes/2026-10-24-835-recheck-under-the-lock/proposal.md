# Change: the dispatch door reads the attempt's reservation under the admission lock, so a sibling's seal never refuses a panel member's spawn (#835)

Status: proposed, under the operator's ruling of 2026-10-24 on run `835-flake-resume-tests-sequence--3c7f7333`'s escalation. No decision document: the ruling is on the record in the commission and in this proposal, and the read rule it governs (M2 of U8a2a's fourth run) is unchanged (S7).
Change: `2026-10-24-835-recheck-under-the-lock`.
Parent: [#835](https://github.com/feedback-loop-ai/brokkr/issues/835), one load flake of the sweep [#857](https://github.com/feedback-loop-ai/brokkr/issues/857). Framing: `.forge/tasks/flake-835-broker-standing-read-race.md` on this worktree; its diagnostic print is commit `efa9af75`.

## Why

Every spawn door, each panel member's included, rechecks the lease at `admission::recheck` (`dispatch.rs:377`), which reads the reservation without the admission lock. Panel members spawn on scoped threads over one shared lease (`panel.rs:270-291`), so one member's recheck runs beside a sibling's `settlement::spawning`, which seals the same record under the lock by link, rename, then unlink of the prior. A read that stats the record between the link and the unlink finds two links and no prior, and `standing` refuses it; `recheck` maps that to `Unknown::Lost`, and the member never spawns: `member driver did not spawn: dispatch refused: … the attempt's reservation no longer stands`. That is the CI signature of both resume tests in #835, verified line by line and reproduced by a scratch stress (201 refusals of 26,604 unlocked reads beside 3,000 seals).

## What Changes

- **The door's read is under the admission lock** (operator's ruling 2, mechanism (a)). `recheck` takes `root::locked` on the coordination root before it reads the reservation, exactly as `amend` does for `spawning` moments later, and releases it once the record is read: before the reach is observed again (`redirected`), before the artifact root's identity is compared, and before the door goes on to `spawning`, which takes its own lock (S1, S2). The seal's window between link and unlink is thereby never observed by a reader: a seal holds the lock across all its steps, and a reader waits for it.
- **The deadline is the one `spawning` takes**: `startup(Instant::now(), Duration::MAX)`, MB3's 30-second startup bound, computed inside `recheck` in the effect crate, as `spawning` and `witnessed` compute theirs. `recheck(held)` keeps its signature at the door and in every test that calls it today; the lock-bound variant is `recheck_by(held, deadline)`, after the `witnessed` / `witnessed_by` precedent in `settlement.rs` (S1).
- **Each refusal keeps its own variant** (decision 0071 ruling 8). A lock not had by the deadline refuses `Unadmitted::Identity(Unknown::Expired)`; a lock file that is not this user's single regular file refuses `Unknown::Lock`; both as `amend` refuses them today, and neither is folded into `Unknown::Lost`. `Lost` stays the refusal for a reservation that cannot be read or no longer names the attempt standing. The door prints the `Unadmitted` text each variant already carries; no text changes (S3).
- **Every reader of a reservation is audited** (ruling 2). Every read of `RESERVATION` goes through `root::read` or `root::bytes`, both through `standing`. The production callers and their locks: `admission::recheck` (none today; locked by this change); `admission::amend` (locked at `admission.rs:757`; `spawning`, `release`, the unresolved mark and the positive fact go through it); `settlement::standing` (its only caller `adopted` locks at `settlement.rs:512`); `inventory/standing.rs` through `inventory()` (`admit_with` locks at `admission.rs:623`, `sources::extended_with` at `sources.rs:590`, `settlement::attestation::attested` at `attestation.rs:43` before `reserved_for`). The implementer confirms each in the commit message, and names the same way any reader this audit missed (S4).
- **Proof, the operator's acceptance.** A deterministic test binds on the lock: another thread holds the admission lock while `recheck_by` is called on a lease whose reservation stands; the door returns admitted only once the lock is released, and refuses `Expired` on a spent deadline; with the lock line removed it returns admitted through the held lock, and the test fails (ruling 9; the mutation is recorded in the commit). A bounded stress carries the before and after: readers calling `recheck` beside a sibling resealing the same reservation through `amend`, bounded in iterations and wall time with a fail-closed floor on the reads completed, asserting zero `Lost` refusals; its count with the lock line removed is quoted in the commit (S5). The two resume tests of #835 are looped alone and in-suite on this host before and after, their counts quoted in the report and the commit; no assertion of theirs moves (S6).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `capability-writer-lease`: this run's own reservation is read at the dispatch door only under the admission lock, so a sibling's seal is waited for and never read as a loss; the lock's own refusals keep their names; and every reader of a reservation holds the lock.

## Not changed

- `standing`'s read rule (M2 of U8a2a's fourth run, run four's second review): which file a read finds beside a seal's prior and staged copies. The operator ruled mechanism (b), a second look in `standing`, out.
- The seal's steps (`seal_with`, `commit`, `retired`, `undone`, `restored`) and `amend`'s lock-read-seal order (U8a2a).
- What the door checks after its read: `verify_record`, the charter read, the re-walk, `redirected` under the fence rule, the artifact root's identity, then `spawning`. Their order and their refusals are as today, and none runs under the lock.
- Every refusal the broker makes today for any other reason, and every `Unadmitted` text the door prints.
- Both resume tests' assertions, and `resume_tests.rs` beyond what `efa9af75` carries. A refused spawn still earns no retry (decision 0006, 2026-10-11 addendum): the change removes a false refusal and retries past no true one.
- Decision 0065 slice two's other open work (#853): the settlement records, adoption, release and the inventory's view (0090). No contract, `policy/phase-machine.json`, `reference/`, fixture, recipe, agent, flag or knob.

## Impact

- `crates/brokkr-runtime/src/engine/broker/admission.rs` (796 lines): `recheck` becomes the startup-deadline wrapper of `recheck_by`, whose lock is scoped to the read. The file has four lines of headroom (ruling 4), and the lock scope and the wrapper need about ten, so one helper moves to `root.rs` (457 lines): `identity`, the device-and-inode observation of a directory, is the recommended move, with the one `use` line in `settlement.rs` that names it following; `sealed` is the alternative. The council confirms which (S8). No behaviour in `root.rs` changes.
- Tests: a new module `crates/brokkr-runtime/src/engine/broker/admission/tests/door.rs`, holding the deterministic lock test and the bounded stress, built from the suite's builders (`Host`, `lease`, `engine`, `opened`); `admission/tests.rs` (1,820 lines) takes only its `mod` line. The ten direct callers of `recheck(&held)` in `fold/tests/routes.rs` and `fold/tests/restart.rs` are unchanged; none holds the lock on its thread. Every `sealing.rs` and `coordination.rs` test stays green and unchanged.
- Confirmed, not changed: no caller of `spawn_site` holds the admission lock when it reaches the door, for `flock` contends between two descriptors of one process and a lock held on the calling thread would wait out the deadline; `admit_with` and `attested` release theirs before any door (S2).
- Public API: nothing. Every broker item is `pub(in crate::engine)` or narrower.
- Specs: `capability-writer-lease`, three requirements added.

## Operator rulings

The operator ruled on 2026-10-24, on the escalation of run `835-flake-resume-tests-sequence--3c7f7333`: (1) the decision 0065 slice two fence is lifted for this one mechanism, so `root.rs` and `admission.rs` may change to fix the race, and nothing else of #853 is in scope; (2) the mechanism is (a), `recheck` reads the reservation under the admission lock as `spawning` already does, `standing`'s M2 read rule stays untouched, and every other caller that reads the reservation without the lock is audited and named in the commit; (3) this is an engine change, a broker refusal path, routed to the heaviest council. Acceptance: a test shows the wrong refusal before the fix and none after; the two resume tests hold in-suite on this host, with counts; no assertion weakened. Each ruling is written where it binds: ruling 1 in Impact, ruling 2 in What Changes and in the delta's first and third requirements, the acceptance in What Changes (proof) and the delta's scenarios.

The door's recheck itself is the operator's ruling of 2026-10-18 after U8a2a's third run (`admission.rs:691-692`): what the door rechecks is unchanged by this change, only how it reads.

## Decisions

### S1 — The deadline is computed inside `recheck`, and the lock-bound entry is `recheck_by`

`recheck(held)` computes `startup(Instant::now(), Duration::MAX)` and calls `recheck_by(held, deadline)`, after `witnessed` / `witnessed_by`. Handing the deadline in at the door was refused: it would change the door's call and the ten test callers for no gain, and the door's two lock takes (`recheck`, then `spawning`) are separate waits with the bound each already observes. The crate is an effect crate; reading the clock there is ruling 1's rule, not its exception.

### S2 — The lock is scoped to the read, not to the door

The lock covers the one read and is dropped before `redirected`, the identity comparison and `spawning`, which takes its own. U8a2a keeps the host admission lock short-lived: the re-walk and `redirected` observe hands and Git, and holding the lock across them would stall every sibling. Between the two takes a sibling may seal; `spawning` re-reads under its lock and refuses `Lost` only where the attempt is truly gone (H1 of U8e2), as today.

### S3 — A lock not had is not a loss

`Expired` and `Lock` are the admission lock's own refusals and keep their variants and texts at the door; `Lost` keeps the read-failed and attempt-missing meaning. A reader that cannot tell "the lock was busy" from "the reservation is gone" would hide a fail-open path behind a retry or a fail-closed one behind a wrong name (ruling 8; the judges' high row).

### S4 — Every read of a reservation is under the lock, and the audit is the commit's

The invariant is written in the delta as a requirement so that a later reader is held to it. The audit table above is triage's, confirmed by the implementer in the commit message; no production reader outside it exists in this tree.

### S5 — The deterministic test binds; the stress carries the evidence

The `Hook` seam (`root.rs:86`) has no step between the seal's link and its rename, so the window cannot be held open deterministically without adding a `Step`, which changes the seal's steps and is out of scope. The binding test therefore holds the lock from another thread and asserts the door returns admitted only after release, and `Expired` on a spent deadline; the stress asserts zero `Lost` beside real seals, bounded in iterations and wall time, with a floor on the reads completed so a reader starved of the lock cannot pass by not reading, and a comment saying why the floor is where it is.

### S6 — Counts, not sleeps

No sleep, retry loop or widened timeout is the fix; `recheck` waits on the lock only, to its deadline, then refuses. The before and after counts of the two resume tests, alone and in-suite on this host, and the stress's refusals with and without the lock line, are quoted in the report and the commit.

### S7 — No decision document

The change alters no rule: D6's atomic admission operation and U8a2a's short-lived, owner-bound lock already say every operation on the reservation runs under it; the door's read was the one that did not. The operator's ruling is recorded here and in the commission.

### S8 — Which helper leaves `admission.rs`

`identity` is recommended: nine lines, observing a directory as `root.rs` observes files, with two callers in `admission.rs` and one in `settlement.rs`. `sealed` would move eleven lines and is named by four modules. The council confirms at design; whichever moves, its doc comment and tests move with it and nothing else in `root.rs` changes.
