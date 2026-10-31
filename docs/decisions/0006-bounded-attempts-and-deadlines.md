# 0006 — Bounded automated attempts and seat deadlines

**Status**: accepted (operator goal directive, 2026-08-23 — "fully
autonomous, end-to-end"; enumeration by Claude under that directive)

Built: partial (#434) — slice 1 built the 2026-10-11 addendum's rulings 1 and 6 and ruling 2's timeout and lost-result rows, in the transport and the built-in driver; ruling 2's partial-side-effect row and rulings 3, 4, 5 and 7's effect identity are unbuilt
Amended by: 0067

## Ruling

Autonomy requires bounded self-recovery. Two per-seat limits become bundle
data (`seats.<phase>.limits`), defaulting to the previous behavior:

- `max_attempts` (default 1): a **determinately failed** driver attempt —
  protocol violation, driver-reported failure, non-zero exit, or deadline
  kill — may be retried automatically with a fresh `attempt_id` on the
  same open effect, up to this limit. When it is exhausted the run parks,
  citing the attempt count and the last recorded error.
- `timeout_seconds` (default 3600): a watchdog kills the driver process at
  the deadline. Because the engine killed it before any result, the
  non-completion is **determinate**: the attempt is `failed` (retryable),
  not `indeterminate`. A hung seat session can no longer hang a run.

An `effect/indeterminate` attempt is NEVER retried automatically,
whatever the limit: completion could not be established, so a retry could
silently duplicate or re-pay for completed work. Indeterminate always
parks into operator judgment — unchanged from decision 0003's outbox
discipline.

## Why

- This is the engine-level analog of what decision 0001 explicitly
  allows *inside* an executor: bounded retries are the seat producing its
  own result, not the control plane repairing one. No retry ever selects
  a transition; the policy still rules on whatever result finally arrives.
- The failed/indeterminate boundary is the safety line: `failed` means we
  KNOW no work product was accepted; `indeterminate` means we don't know,
  and guessing is what this engine exists to refuse.
- Without deadlines, "fully autonomous" is false advertising: one hung
  CLI process wedges the run forever with no journal evidence.

## Mechanics

- Fold: `effect/failed` returns the open effect to the executable
  position with `failed_attempts` counted; `run/parked` is legal from
  that position (engine exhausted the limit). Both are recorded in
  contracts/README.md's fold semantics.
- Every attempt still follows the durable discipline: `effect/started`
  durable before spawn, exactly one terminal fact per attempt.
- Limits are compile-validated (integers ≥ 1; unknown keys rejected).

## Consequences

- Machine proof grows: transient-failure retry completes; exhausted
  limit parks with the last error; a hung driver is killed at its
  deadline; indeterminate never auto-retries even with attempts left.
- Default limits (1 attempt, 1-hour deadline) preserve every previously
  proven behavior; bundles opt into more attempts per seat.
- The self bundle adopts explicit limits in a follow-up once the
  in-flight ship-taxonomy delivery lands (avoids conflicting with that
  run's own edits to `bundles/self/`).

## Addendum — 2026-09-27, the kill takes the attempt's process tree (#403), proposed

Addendum status: proposed

The deadline kill above reached only the driver process. The harness a
driver starts, and that harness's tool subprocesses, outlived it: on
2026-09-25 a Claude session went on writing for 22 minutes after the
journal recorded its attempt killed, and a retry could start beside it
in the same worktree. The non-completion this decision calls
determinate was not.

- Two means reach the tree. The driver leads a session, and so a
  process group, of its own, and the kill is SIGKILL to the group. While the attempt runs, the
  engine also records every descendant by pid and start time, a
  descendant that left the group (`setsid`, as Node's `detached` spawn
  does, or a job that shell job control moves to a group of its own)
  included. Every kill reads the table once more first, and ends those
  identities too. A pid alone is never signalled. Nothing is chosen by
  working directory, so a concurrent run in the same checkout is never
  signalled.
- On Linux the engine and every driver are child subreapers. An orphan
  of the tree goes to its driver while the driver runs, where it is
  recorded as a descendant. Once the driver is gone, it goes to the
  engine and not to init, whatever its session. Every running child of
  the engine outside the engine's own group that no live attempt leads,
  records or began after is a stray. A child the engine spawns stays in
  that group, and nothing of an attempt can join it: the driver's session
  is its own, and the kernel refuses `setpgid` into a group of another
  session. A stray is attributed to the attempts that could have left it:
  those it does not predate, whose driver no longer runs, and that had
  not yet ended when it was born, as the start stamps order it. One such
  attempt ends it with its own tree. When there are several, the stray
  is ended and every one of them parks. When there is none, the stray is
  the engine's own, as git's detached maintenance is. The read before a
  driver starts is taken afresh, never shared with an earlier one, so an
  orphan of the engine's own born before it is not taken for the
  attempt's. So on Linux, but for the first residual below, no descendant the
  engine can see is left running while an attempt is certified settled.
  A kernel that refuses the engine the subreaper or a pidfd leaves this
  second means absent, and then a kill that saw any descendant parks,
  naming the means that was missing.
- Every attempt ends the same way, whatever ended it, but for the
  engine's signal stop below: `shutdown`, written
  without waiting past the grace; a bounded grace for the driver to exit;
  a read of the table, then SIGKILL to the group and to every identity the
  attempt owns; a bounded reap; and a bounded wait for the process table
  to show nothing of the tree running. A zombie counts as gone. The group
  is signalled only while its leader is unreaped, so its id cannot name
  another process.
- When SIGINT, SIGTERM, SIGHUP or SIGQUIT tells the engine to stop, it
  ends every live attempt more bluntly: no `shutdown` message, no grace,
  and no reap of the driver. It reads the table once, SIGKILLs every
  attempt's group whose leader is unreaped and every identity each
  attempt owns, strays included, then reads the table again, signalling
  what still runs, until it reads every attempt gone or the settle bound
  passes. It exits with 128 plus the signal when every attempt read gone
  and no missing means casts doubt on one. Otherwise it says why and
  exits 125. The driver no longer shares the engine's session, so a
  terminal's Ctrl-C, Ctrl-\ or hangup would not otherwise reach it. Nor
  does a SIGKILL to the engine's process group, which reached a seat's
  tree while the driver shared that group: only the stop signals the
  engine handles end it now. A signal the engine inherited as ignored
  (`nohup`) stays ignored.
- The report returns only once the tree is proven gone, so the retry
  this decision allows cannot overlap it.
- An end that cannot be proven is `indeterminate`, by this decision's own
  line: completion of the cleanup is not known. That covers a group or a
  recorded descendant still running, a stray that could not be attributed
  to one attempt, a kill the kernel refused on a live identity, a leader
  not reaped, a missing means, and a pipe something outside the tree
  still holds. It covers a kill the kernel refused on the group too, but
  for one reading: an EPERM on the group is read against the table,
  since Darwin refuses a group whose only members are zombies or exiting
  with EPERM. A group whose fresh, whole read shows no member running is
  gone. An EPERM group with a member running, or whose table cannot be
  read whole, and any other refusal on the group are indeterminate. The
  cleanup names the group's refusal first, except for an EPERM whose
  table cannot be read whole: that one names what the read shows, a
  refusal on a live identity if there was one and otherwise the read's
  own failure. It
  also covers a table that cannot be
  read whole: a row that cannot be read or parsed, a `ps` that exits
  nonzero, and a snapshot without the engine's own row. A row that
  vanished between the listing and its read is gone. A table that cannot
  be read before a driver starts refuses the spawn, so nothing runs
  without the read that tells its orphans from what ran before it. The
  report keeps
  the outcome the attempt reached, typed, beside an unresolved cleanup,
  and the engine journals both on `effect/indeterminate`, as the
  numbered extension `contracts/effect-cleanup.v1.schema.json`
  publishes them: `received` and `cleanup` for a seat or a step, and the
  same pair per member under `unresolved_members` for a panel. An
  attempt proven over journals the v1 payload unchanged. A checkpoint
  the journal refused changes
  the outcome acted on, never the outcome received. The engine never
  certifies the outcome or retries it.
- Two Linux residuals remain. The first, the table-read race, is
  accepted by the operator's ruling of 2026-09-28. A Linux read of the table is a
  `/proc` listing followed by one stat read per pid. Suppose a failed
  driver's last live descendant forks a detached child after the listing
  and exits before its own row is read. That read shows nothing of the
  attempt running, and so sets the attempt's end. The next read, up to
  the tracker's 100 ms later, finds the child with the engine, born
  after that end, and files it as the engine's own. Cleanup can then be
  certified while the child runs. The same read has a second ordering:
  once the pid counter has wrapped, a detached child can have a lower
  pid than its parent, so its row is read first, still naming that
  parent, and the parent exits before its own row is read. That read
  records the child neither as a descendant nor as a stray, and the same
  certification follows. A third ordering needs no wrap: a detached
  descendant born after one read is listed by the next, whose row for the
  driver is read first and shows it running; the descendant's parent and
  the driver then exit before the descendant's own row is read, so that
  row names the engine as its parent while the driver still reads as
  leading. No attempt could have left it, so it is filed, once, as the
  engine's own, and a later read that finds nothing of the attempt
  running certifies the same settlement. It is inherent to polling the table;
  per-attempt cgroup containment, filed as #472, closes it by
  construction.
- The second Linux residual, the read-to-fork instant, is accepted by the
  operator's further ruling of 2026-09-28. It is a wrong kill, not a
  missed one. No read of the table is atomic with the fork, so an orphan
  of the engine's own (git's detached maintenance, say) that the engine
  adopts between the fresh read before a spawn and the fork itself is
  absent from what ran before the attempt. If the driver exits before
  the next read, the orphan is attributed to the attempt, and the
  attempt's close kills it. #472 closes it by construction too.
- macOS has no subreaper and no pidfd. There the engine reads the table
  synchronously at the kill, ends what it attributes, and parks on any
  doubt. The operator's earlier ruling of 2026-09-28 (LINUX CLOSED,
  MACOS RESIDUAL ACCEPTED) accepts one residual there, beside the Linux
  ones above: a descendant that
  leaves the group and whose parent exits between two reads of the
  table, faster than the tracker's 100 ms interval, is reparented to
  launchd unseen. Separately, and not a limit of settlement: without a
  pidfd, a pid reused between `ps`'s confirmation and the signal could
  be signalled.

## Addendum — 2026-10-11, accepted 2026-10-12: ending a process proves nothing about its effects, and a retry needs evidence (#434)

Addendum status: accepted (operator, 2026-10-12)

The operator accepted this addendum in chat on 2026-10-12, ruling each
asked item as recommended (Ruled, below). Retry behaviour changes when
#434's build lands; until then the rulings above stand as written.

The ruling above calls a deadline kill determinate "because the engine
killed it before any result". That fact is about who ended the process.
It is silent on what the attempt did first. #403's addendum since proves
the attempt's process tree gone before a retry starts, and that proof is
about processes too. Research 0021 inventories every end of an attempt
at `3ad0f66c`. Several ends report `failed`, and so retry, although no
result of the executor's was read after the driver received `start`:
the deadline kill (`process.rs:460-466`); a protocol violation, an
unreadable line or a failed read of the driver's stdout after `start`
(`converse`, `process.rs:599-632`, and `recv`, `process.rs:366-381`);
and the built-in driver's own `failed` when its command exits 0 and
writes no result file (`adapters.rs:6895-6903`), or ends by a signal,
or when writing the prompt to the spawned command or waiting for it
fails (`adapters.rs:6675-6702`). A driver that exits without a result is already `indeterminate`
(`process.rs:467-475`), and so is a driver past a transport limit
(decision 0079 ruling 3). The `idempotency_key` the engine journals
(`engine.rs:1069`) reaches no executor, so nothing deduplicates on it.
The operator's threat model of 2026-09-26 asks the engine to catch
realistic accidental misuse and to fail closed on what it cannot read.
A duplicated side effect is the harm it names, so where the evidence
cannot show the executor's own report, these rulings park.

1. **Four facts, kept apart.** An attempt's end is read for four facts,
   and none of them implies another:
   - *termination*: the attempt's process tree is proven gone
     (`Cleanup::Settled`, #403);
   - *receipt*: the executor's own result reached the engine whole and
     readable, within the transport's limits (decision 0079). For a
     built-in driver the executor is the command it runs, and its result
     is the result file or the exit status the command returned. A
     `failed` the driver writes because the command left neither is the
     driver's synthesis, not a receipt;
   - *refusal before execution*: the engine knows the executor never
     began. Either the engine refused before spawning, or the driver
     never received `start`, or the driver's own result refused with no
     `accepted`, no checkpoint and no deadline kill (decision 0053's
     predicate, `failed_to_start`);
   - *effect certainty*: what the attempt changed. Brokkr knows this
     only for the run's own worktree, and only by reading it.

   A failure the ruling above may retry needs receipt or a refusal
   before execution. Termination alone never makes one.

   **Enforcement binding:** the outcome classification in
   `crates/brokkr-protocol/src/process.rs` (`eof_outcome` and `end`)
   holds this ruling. Every end after `start` with no result read
   becomes `Indeterminate` there, whether `converse` or `recv` decided
   it, and `deadline_killed` stays on the report. An end decided by
   `recv` during the handshake, before `start` is sent, stays `Failed`,
   except an end the transport reads as lost (a broken pipe at hello,
   or `recv`'s end-of-stream arms), which parks as it does today.
   In `crates/brokkr-protocol/src/adapters.rs`, `run_seat_with`'s
   no-result-file arm, an exit by signal, and its invoke `Err` arm for
   an error after the command was spawned no longer send `failed`;
   they end with no result, which the transport already reads as lost.
   The `Err` arm keeps sending `failed` for an error before the spawn.
   The tests are named under ruling 2.

2. **The retry matrix.** Each end retries or parks as follows:

   | End | What the engine knows | Ruled |
   |---|---|---|
   | Pre-session refusal: the engine refused the spawn, the handshake failed before `start`, a classified provider refusal, or a driver refusal sent before anything was invoked (a missing secret, an unverified charter, a command that could not be staged or spawned) | the executor never began, or it said so in its own result | Retries automatically within `max_attempts`, except a handshake end the transport reads as lost, which parks as it does today. A model chain descends where decisions 0016 and 0053 allow, which excludes the refusals sent after `accepted`. Unchanged. |
   | Process timeout: the deadline kill | the tree is gone, and no result was read | The effect is uncertain, and the attempt is `effect/indeterminate`. It retries only under ruling 4. Otherwise it parks for the operator. **Changed.** |
   | Lost result: no result of the executor's was read after `start`. That is a driver exit with no result; a protocol violation, an unreadable line, a wrong `proto` or a failed stdout read after `start` (`converse` and `recv`); the built-in driver's `failed` when its command exits 0 and writes no result file, or ends by a signal; the built-in driver's `failed` for an invocation error after the command was spawned (writing the prompt to its stdin, waiting for it, or exec's state command); or an engine restart in flight | no result was read; whatever the driver wrote in its place is not a receipt | As for the timeout. **Changed** for every arm that reports `failed` today. An engine restart always parks, because nothing proves its tree gone. |
   | Transport limit: a line, result or checkpoint past a limit of decision 0079 (`Exceeded`) | the driver broke its bounds; nothing it sent is read | Parks for the operator, always, even on a site declared `deduplicates` whose driver delivered the identity. Accepted decision 0079 ruling 3 stands over this addendum: such an attempt is never failed in a way a retry may repeat. Ruling 4 makes no exception to it. Unchanged. |
   | Partial side effect: the executor's own `failed` report after it began work. That is a third-party driver's `failed` result, or a built-in driver's command that returned a non-zero exit status | the executor answered and its result failed; what it changed is not known | A seat's reach is its worktree only when all three facts, read from the pinned run manifest, say so: boxed hands, network off, and no capability granted of class `writes` (decision 0065: it changes something outside the workspace). Such a seat retries automatically within `max_attempts`, as the ruling above has it. The retry runs in the same worktree, on the same result path and under the same effect identity, so it reconciles against what the attempt left there. Every other seat parks for the operator: network on, an unboxed boundary, a `writes` grant, or any of the three facts the engine cannot read. A retry there may repeat an effect outside the tree (ruled 2026-10-12). Nothing is claimed about effects beyond the worktree (ruling 6). Changed for every seat whose reach is not proven to be its worktree. |
   | Cleanup failure: the tree is not proven gone | an outcome was received, and the cleanup is unresolved | Parks for the operator, always, whatever was received or declared (#403's addendum). Unchanged. |

   A panel or sequence attempt is uncertain when any member or step is,
   as it is today. An uncertain panel or sequence attempt always parks,
   because ruling 4 refuses `deduplicates` inside one.

   **Enforcement binding:** in
   `crates/brokkr-protocol/src/process/tests.rs`, the `deadline_failure`
   helper's pinned outcome becomes the indeterminate reason of ruling 6.
   New tests pin the lost-result row:
   `a_protocol_violation_after_start_is_indeterminate`, and
   `an_unreadable_line_after_start_is_indeterminate` for `recv`'s arms,
   beside a handshake case that pins `Failed` before `start`. In
   `crates/brokkr-protocol/src/adapters.rs`'s suite,
   `a_command_that_exits_zero_without_a_result_ends_lost` pins that the
   driver sends no `failed` for that arm, and
   `a_prompt_write_failure_after_the_spawn_ends_lost` pins the same for
   an exec command that closes its stdin before reading its prompt. In
   `crates/brokkr-cli/tests/machine_proof.rs`, where the engine reads the
   pinned manifest, `a_reported_failure_on_a_seat_beyond_its_worktree_parks`
   pins the partial-side-effect row's split from the run's journal, with
   a limit of 2: the same `failed` result retries on a boxed, network-off
   seat with no `writes` grant, with two `effect/started`, and parks after
   one, with its exact reason, on a network-on seat, on a boxed,
   network-off one granted a `writes` capability, and on one whose pinned
   boundary or network fact cannot be read. Also in `machine_proof.rs`,
   `a_transport_limit_end_parks_even_when_declared_and_delivered` drives
   a site declared `deduplicates`, whose `accepted` echoes the identity,
   past the frame limit, with a limit of 2. It pins the park, the
   0079 reason, and one `effect/started`. There too,
   `hung_driver_is_killed_at_its_deadline` and
   `indeterminate_is_never_auto_retried_even_with_attempts_left` keep
   their names and assertions. A new test,
   `a_deadline_kill_with_attempts_left_parks_after_one_attempt`, drives a
   hanging seat with a limit of 2. Another,
   `a_reported_failure_retries_on_the_tree_the_attempt_left`, drives a
   fake executor that writes a marker and reports failure, and pins that
   its retry reads the marker. For the acceptance item on a known failure
   before acceptance, `crates/brokkr-cli/tests/agent_fallback.rs` adds
   `a_refusal_before_the_session_still_descends_within_its_bound`.

3. **No payload earns replay authority.** Authority to retry an
   uncertain attempt has one source: the operator's closed declaration
   in the compiled bundle, digested into the manifest, on a command the
   compile proved names the identity's placeholder (ruling 4). The
   driver's echo of the identity is proof that the identity was
   delivered, not authority: without it the attempt parks, and with it
   nothing more is earned. The following confer none, whatever they
   say: the driver's `capabilities.supports` list, a checkpoint, a
   result object, stderr, and every other string on the wire. An unknown
   declaration value refuses the compile. So does a `deduplicates`
   declaration on a site whose executor is a model harness (claude,
   codex, dsh or lanetally), because its session can act beyond anything
   that could deduplicate (ruling 6). So does a `deduplicates`
   declaration inside a panel or a sequence, or inside a select arm that
   is one, because the identity is the effect's, and two sites in one
   body would share it (ruling 4). An explicit `none` is the default and
   refuses nowhere.

   **Enforcement binding:** compile refusals in
   `crates/brokkr-runtime/src/bundle.rs`, each tested with its exact
   text: an unknown `replay` value; `deduplicates` on a model-harness
   site; `deduplicates` on a command without the placeholder; and
   `two_deduplicating_steps_in_one_sequence_refuse_the_compile`, with a
   panel-member case beside it. A machine-proof test,
   `a_driver_advertising_replay_on_the_wire_parks`, drives a fake driver
   that lists an extra `supports` string and writes a replay-shaped field
   into its checkpoint, then is killed at its deadline. It pins the park.

4. **The effect's identity is stable, and attempt ids stay distinct.**
   - **The identity.** It is the `idempotency_key` that `effect/requested`
     already journals, `{run_id}:{seq}`. Its bytes and derivation do not
     change. Every automatic attempt of the effect carries it, and each
     attempt keeps a fresh `attempt_id`. It names the effect, not a site
     within it: a panel or sequence is one effect whose members and
     steps share one `effect_id` and one `attempt_id`
     (`engine/sequence.rs:38-39`). So in this slice a declaration stands
     only where the site is the effect's whole body: a `Single` body, or
     a select arm that is one. A declaration in a panel or a sequence
     refuses the compile (ruling 3).
   - **The declaration.** An `exec` site may declare `replay`, a closed
     vocabulary of `none` (the default) and `deduplicates`. With
     `deduplicates`, the operator states that the command acts at most
     once per identity. Handed an identity it has already acted on, the
     command reports the committed outcome instead of acting again.
     Reconciliation is that same report, so it takes no second value.
     The exec driver passes the identity to the command through a
     template placeholder, and a declaration whose command does not name
     the placeholder refuses the compile. The declaration, pinned in the
     manifest, is the authority for a retry, and it is the operator's
     statement about the command. Brokkr does not verify that the
     command keeps it.
   - **Delivery.** The next version of the driver protocol delivers the
     identity in `start` and `resume`. The driver echoes it exactly in
     `accepted`. The exec driver sends `accepted` before it spawns the
     command (`adapters.rs:5637`), so the echo is the driver's, not the
     command's. It proves that this attempt's driver held the identity
     it substitutes into the command. It acknowledges nothing on the
     command's behalf, and an attempt without it parks.
   - **When it retries.** An uncertain end of a `deduplicates` site
     retries within `max_attempts` only when all of these hold: its
     cleanup is settled; its journaled `accepted` echoed the effect's
     identity byte for byte; its end was a deadline kill or a lost
     result (ruling 2); and its end was neither an engine restart nor a
     transport-limit end of decision 0079. The attempt's terminal event
     is then `effect/failed`. It carries the evidence behind the
     decision: the declaration, the identity, the delivering attempt id
     and the outcome received. A numbered contract extension beside
     `effect-cleanup.v1` holds that evidence, and fold never reads it.
     If any condition fails, the attempt is `effect/indeterminate` and
     parks.
   - **An operator's `retry`** of a parked effect not declared
     `deduplicates` opens a new effect with a new identity, as it does
     today. It is the operator's decision to act again, after reconciling
     what the parked effect did. An operator's `retry` of a parked
     `deduplicates` effect is the exception below: it keeps the effect's
     identity.

   **Enforcement binding:** the issue's acceptance tests, in
   `crates/brokkr-cli/tests/machine_proof.rs`. Each test's fake executor
   counts its commits in a marker file outside the worktree.
   - `a_marker_committed_then_lost_parks_and_is_never_committed_twice`:
     an undeclared exec commits its marker and its result is lost, with
     a limit of 2, in three cases: it is killed at its deadline; it
     exits 0 without writing a result file; and it closes its stdin
     before reading a prompt larger than the pipe buffer, so the
     driver's prompt write fails after the spawn. Each run parks, with a
     count of 1 and one `effect/started`.
   - `a_deduplicating_executor_sees_one_identity_and_acts_once`: the same
     executor is declared `deduplicates` and reads the identity. Its
     first attempt commits and is killed. Its second receives the same
     identity, finds its marker and reports the committed outcome. The
     test pins: a count of 1; two `effect/started` with different
     attempt ids; both `accepted` echoing one identity equal to
     `effect/requested.idempotency_key`; the replay evidence on the first
     `effect/failed`; and a completed run.
   - `an_undelivered_identity_parks`: the site is declared, but its
     `accepted` does not echo the identity, so the run parks.
   - `an_operator_retry_keeps_the_effect_identity`: the operator's
     `retry` of a parked `deduplicates` effect sends the same identity,
     so the executor's deduplication still applies (ruled 2026-10-12).
     It pins that the retry's `effect/requested` carries the parked
     effect's own `effect_id` and `idempotency_key`.

   An operator's `retry` of a parked `deduplicates` effect keeps its
   effect identity, and opens no new effect.

   A fold test in `crates/brokkr-core/src/fold.rs`'s suite pins that an
   `effect/failed` carrying the evidence folds exactly as one without it.

5. **No retry overlaps the attempt it repeats.** Every retry this
   addendum allows starts only after the earlier attempt's cleanup is
   `Settled`. The settled outcome is read before any declaration, so an
   unresolved cleanup is indeterminate and parks, `deduplicates` or not.

   **Enforcement binding:**
   `a_deduplicating_site_whose_descendant_outlives_the_kill_parks` in
   `crates/brokkr-cli/tests/machine_proof.rs`. Its executor leaves a
   descendant that the teardown cannot prove gone, built from #403's
   fixtures in `crates/brokkr-protocol/src/process/tests.rs`. The test
   pins the park with the cleanup evidence, a marker count of 1, and no
   second `effect/started`.

6. **The limits, stated.** Brokkr does not promise exactly-once
   execution. It promises three things:
   - an attempt known not to have begun may be repeated;
   - an attempt whose result was not read is repeated only on an exec
     site that is its effect's whole body, that the operator declared to
     deduplicate on the effect's identity, and whose driver delivered
     that identity; never after a transport-limit end or an engine
     restart;
   - every other uncertain end is the operator's to judge.

   A settled cleanup proves that the local process tree ended. It is
   never reported, journaled or shown as a rollback of anything the
   attempt did, locally or remotely. Some effects are outside what
   Brokkr can observe or deduplicate: an agent session's remote actions,
   a push made from inside a session, and provider usage already paid
   for. A retry may repeat them, so a reported failure on a seat whose
   reach is not proven to be its worktree parks (ruling 2's
   partial-side-effect row). On such a seat a `max_attempts` above 1
   confers no authority to retry a reported failure. It governs only the
   pre-session refusals, which never began, and ruling 4's deduplicated
   retries.

   **Enforcement binding:** the deadline kill's indeterminate reason is
   pinned once, in `crates/brokkr-protocol/src/process/tests.rs`. It
   reads: "attempt exceeded its Ns deadline and was killed; its process
   tree is gone, and what it did is unknown". The outcome table and the
   "Deadlines and kills" section of `docs/guides/driver-authoring.md`
   state this ruling's three promises and its limits. Today that section
   says the kill "is what makes bounded retry safe", which is the
   inference this addendum withdraws. The guide also states that a
   driver sends `failed` only for a failure its executor reported, and
   ends with no result when the executor left none. Beyond that text,
   the ruling is judgment guidance for reviewers: no readout may call a
   settled cleanup a rollback.

7. **Compatibility.** No journal byte moves, and fold does not change:
   `effect/failed` still returns the effect for a retry, and
   `effect/indeterminate` still parks. Every existing journal replays to
   the state it did. A deadline kill that an earlier engine journaled as
   `effect/failed` stays one.
   - Ruling 2 is a stricter engine reading of how an attempt ended. It
     is journaled with the v1 `effect/indeterminate` event and its
     `reason`. It applies to attempts that start after the build lands,
     those of a resumed run included. It takes this ruling, not a
     contract version.
   - Ruling 4's guarantee takes explicit versions: the driver protocol's
     next version, the bundle's `replay` key pinned through the next
     run-manifest version, and the evidence's contract extension.
   - A v1 driver, or a bundle without `replay`, never earns replay
     authority. An existing bundle compiles to `none` and keeps every
     behaviour except ruling 2's.
   - An operator's `retry` of a parked `deduplicates` effect keeps its
     identity with the events v1 already has. Fold's `retry` still
     returns the cursor to request an effect (`fold.rs`). The engine,
     finding that the effect the run parked on was declared
     `deduplicates`, journals that request as `effect/requested` with the
     parked effect's own `effect_id` and `idempotency_key`, read from its
     first `effect/requested`, instead of minting new ones
     (`engine.rs`). Fold opens the effect a request names, whatever id
     it carries, so it does not change. The request opens the effect's
     next attempt, not a new effect. A reader that counts effects by
     their `effect/requested` events rather than by `effect_id` is named
     by the build and counted by id.

   **Enforcement binding:** `crates/brokkr-cli/tests/witness_journal.rs`
   replays its pinned journals unchanged. A witness digest moves only
   where its test drives an end of ruling 2's lost-result or timeout
   rows that reports `failed` today, and it is re-pinned from that
   test's reported value. A
   bundle test pins that a bundle with no `replay` compiles to `none` and
   the same manifest digest it had.

**Ruled** (operator, in chat, 2026-10-12). Each item below was asked of
the operator, and each was ruled as recommended:
- The cost of ruling 2 was not measured. A hung seat with a limit of 2
  now parks after its first deadline instead of retrying once. Should
  that rate be counted from the journals before acceptance?
  **Ruled:** accepted now; the build's first step counts that rate from the journals.
- A seat's reach can be pinned to its worktree: boxed, with network off
  and no capability granted that writes elsewhere. Should a deadline
  kill on such a seat retry as a reconciliation, since its effects are
  in the tree the retry reads, rather than park? The simplest rule the
  evidence supports is no, and that is what ruling 2 rules.
  **Ruled:** no; such a seat parks, as ruling 2 rules, until the count says otherwise.
- Should a reported failure on a seat that can reach beyond its
  worktree (network on, or an unboxed boundary) park rather than retry?
  In the shipped library that is `agents/release-manager.json`.
  **Ruled:** yes; the partial-side-effect row of ruling 2 now parks it, and every seat whose reach the pinned manifest does not prove to be its worktree.
- Should an operator's `retry` of a parked `deduplicates` effect keep
  its identity, rather than opening a new effect?
  **Ruled:** yes; ruling 4 now says so.
- The issue asks for an identity "acknowledged by the actual executor".
  Ruling 4 makes the pinned declaration the authority and the driver's
  echo proof of delivery only, because no channel today carries the
  command's own word before it acts. The stricter form is a
  command-level acknowledgement: the command writes a receipt of the
  identity before acting, the driver forwards it, and a retry needs it.
  Its binding test would pin that a driver-only echo parks. Should the
  build take that form instead?
  **Ruled:** the pinned declaration plus the driver's echo, as ruling 4 rules; the command-level acknowledgement is a later option.
- Ruling 4 refuses `deduplicates` inside a panel or a sequence. The
  alternative scopes the delivered identity to the structural site, the
  `idempotency_key` with the site's path in the body, stable across
  retries. Its binding test would drive two declared exec steps in one
  sequence and pin two distinct identities, each acted on once. Is that
  a follow-up?
  **Ruled:** a follow-up; `deduplicates` stays refused inside a panel or a sequence until then.
- Research 0021 rows 8 and 9 are unruled: a result file shared by an
  effect's attempts, and a sequence retry that re-runs steps that
  succeeded. Are they follow-ups, or part of this addendum?
  **Ruled:** follow-ups.
- Decision 0053 ruling 5 reasons from a deadline kill's outcome being
  `failed`. Its conclusion still holds once this is accepted, because an
  indeterminate attempt is never a failure to start. Does 0053 take an
  erratum that says so?
  **Ruled:** yes; 0053's ruling 5 carries the erratum.
