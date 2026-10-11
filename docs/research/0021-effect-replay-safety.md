# 0021 — Effect replay safety: what each attempt's end proves, measured from the code

Source: https://github.com/feedback-loop-ai/brokkr/issues/434
Authors: Brokkr's implementer seat (Claude Opus 5.5), from the issue's architecture review of `8d7da844`
Read: 2026-10-11
Status: proposed
Intake: run `design-story-434-define-effect-r-a3442867`, on main at `3ad0f66c` (code-health epic #330)

## Summary

Issue #434 asks whether ending an attempt's process is evidence that its
effect may be repeated. Its review of `8d7da844` found that the transport
classifies a deadline kill as `Failed`, which the engine retries under the
seat's attempt limit, and that the engine journals an idempotency key no
executor is shown to read. The review did not show a duplicated remote
transaction. It named an inference the code makes without evidence.

This entry inventories the same paths at `3ad0f66c`, after #403's process
settlement, decision 0079's bounded transport and decision 0053's
withheld `accepted`. Every claim cites a file and line read at that base.
The inference stands, and it has instances the issue did not name: a
protocol violation or an unreadable line after `start` is also `Failed`
and retried, and so is the built-in driver's own `failed` when its
command exits 0 and writes no result file, or when writing the prompt
to the command, or waiting for it, fails after the spawn. Two adjacent hazards came to
light, a result file shared by an effect's attempts and a sequence retry
that re-runs steps that succeeded. Both are named below as findings for
the operator. The proposed addendum to decision 0006 rules on the ends
of an attempt and leaves these two unruled, as questions in its Asked
paragraph.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep four facts apart: the attempt's process ended, its result was received, it was refused before execution, and what its side effects were | planned | #434; `docs/decisions/0006-bounded-attempts-and-deadlines.md` |
| 2 | Never retry automatically an attempt whose end left completion unknown | implemented | decision 0006, decision 0079, `crates/brokkr-core/src/fold.rs` |
| 3 | Read a deadline kill, and every other end after `start` with no result of the executor's read, as unknown completion, not as a determinate failure | planned | #434; `crates/brokkr-protocol/src/process.rs` |
| 4 | Retry, and fall back down the model chain, within a bound, on a failure known to precede execution | implemented | decision 0016, decision 0053, `crates/brokkr-runtime/src/engine.rs` |
| 5 | Keep one effect identity across an effect's attempts, with a fresh id for each attempt | implemented | `crates/brokkr-runtime/src/engine.rs`, `contracts/README.md` |
| 6 | Give replay authority only to an executor that receives the effect identity and acknowledges it, on pinned data, never on a payload string | planned | #434; `contracts/driver-protocol.v1.schema.json` |
| 7 | Retry only once the attempt's whole process tree is proven gone, and never read that proof as a remote rollback | implemented | decision 0006, `crates/brokkr-protocol/src/lib.rs` |
| 8 | Bind a seat's result file to the attempt that wrote it | not-planned | |
| 9 | Resume a failed sequence at its failed step rather than re-running steps that succeeded | not-planned | |
| 10 | Promise exactly-once execution of an effect | declined | decision 0006 |

Rows 8 and 9 are new and unruled. Row 8: the result path is
`.forge/results/<effect_id>.json`, one path for every attempt of an
effect, and no driver clears it before an attempt. A retry whose seat
exits 0 without writing would read an earlier attempt's file as its own.
Row 9: a sequence that fails at a later step re-runs its earlier steps,
and their effects, on retry. Row 10 is declined because no executor
Brokkr runs can bound an agent session's remote actions. The addendum
states that limit rather than a guarantee.

## The attempt's ends today

The transport is `DriverProcess` in `crates/brokkr-protocol/src/process.rs`.
It ends an attempt in one of four internal states (`enum Ended`,
`process.rs:118`): `Failed(String)`, `Lost`, `Reached(AttemptOutcome)` and
`Exceeded(Exceeded)`. `end` (`process.rs:643-659`) turns each into the
public `AttemptOutcome` (`crates/brokkr-protocol/src/lib.rs:113`):
`Succeeded`, `Failed` or `Indeterminate`. `finish` (`process.rs:405`) then
ends the process tree through #403's teardown and records `cleanup`
(`process.rs:443-446`). `settled_outcome` (`lib.rs:181-192`) is what the
engine acts on. A cleanup that is not `Settled` turns any outcome into
`Indeterminate`.

| End | Where it is decided | Outcome reported | Engine journals | Retried today |
|---|---|---|---|---|
| Engine refuses before execution: the rebuilt seat input does not match the digest recorded at request | `engine.rs:1317-1336` | none: no driver runs | `effect/started` with `driver: "none"`, then `effect/failed` with no `start_failure` | yes, under `max_attempts`; the chain does not descend |
| Engine refuses before spawn: a site refusal, the driver binary missing, the process table unreadable | `spawn_site` `engine.rs:3146`; `run_driver` `engine.rs:2034`; `SpawnError` `process.rs:59-71` | none: `DriverRun::SpawnFailed` (`engine.rs:1921-1930`) | `effect/failed`, with `start_failure` only when the site's model was agent-selected (`start_failure_fields` `engine.rs:3109-3123`) | yes, under `max_attempts`; the chain descends where it has one |
| Handshake fails before `start` is sent | `process.rs:533-574` | `Failed`, except where the transport reads the driver as gone: a broken pipe at hello (`process.rs:540-541`) and `recv`'s end-of-stream, `Exceeded` and closed-stream arms (`process.rs:369`) end `Lost`, which `eof_outcome` makes `Indeterminate` (`process.rs:467-475`) | `effect/failed`, or `effect/indeterminate` for a `Lost` end | yes, except a `Lost` end, which parks |
| Driver's own refusal before a turn: a classified provider refusal (0053) | the claude, lanetally and codex classifiers in `crates/brokkr-protocol/src/adapters.rs` | `Failed`, no `accepted`, no checkpoint | `effect/failed`, `start_failure` | yes, and the chain descends |
| Driver's own refusal after sending `accepted` but before invoking anything: a missing secret, an unverified charter, or an invocation error before the command is spawned (exec's prompt file could not be staged, `adapters.rs:5640`; the spawn itself failed, `run_cli` `adapters.rs:755`) | `adapters.rs:6553-6572`, `adapters.rs:6582-6597`; the invoke `Err` arm `adapters.rs:6675-6702` | `Failed`, `accepted` | `effect/failed` | yes, without chain descent |
| The built-in driver synthesizes failure from an invocation error after the command was spawned: writing the prompt on its stdin failed (exec feeds the prompt there when its command names no `{prompt_file}`, `adapters.rs:5650-5655`; `run_cli` spawns at `:755` and writes at `:758`, `write_prompt` `:689-693`), waiting for the command failed (`:762`, and the model harnesses' waits at `:2386`, `:2486`, `:3726`, `:5022`), or exec's `dialect_exec` state command failed after its command ended (`:5674`) | the same invoke `Err` arm, `adapters.rs:6675-6702` | `Failed`, `accepted`, with the error text | `effect/failed` | **yes** |
| Executor reports failure after work began: its command returned a non-zero exit status (a third-party driver's `failed` result is read the same way) | `reached` `process.rs:663-675`; `adapters.rs:6885-6893` | `Failed`: "agent CLI exited N" | `effect/failed` | yes |
| The built-in driver synthesizes failure with no executor result: the command exits 0 and writes no result file, or ends by a signal and so returns no exit status (`code().unwrap_or(-1)`, `adapters.rs:5681`) | `run_seat_with` `adapters.rs:6895-6903`, and `:6885-6893` for the signal | `Failed`: "seat wrote no result file (the result contract was not met)", or "agent CLI exited -1" | `effect/failed` | **yes** |
| Deadline kill | watchdog `process.rs:294-309`; `eof_outcome` `process.rs:460-466`; `deadline_killed` `process.rs:453` | `Failed`: "attempt exceeded its Ns deadline and was killed", whether or not the driver had accepted | `effect/failed` | **yes**, under `max_attempts` |
| Protocol violation after `start`: a foreign effect id, a message out of place (`converse`); a failed stdout read, an unreadable line or a wrong `proto` (`recv`) | `converse` `process.rs:599-632`; `recv` `process.rs:366-381`; both mapped at `process.rs:650-652` | `Failed`, even after `accepted` | `effect/failed` | **yes** |
| Driver exits after `start` with no result | `Ended::Lost`, `eof_outcome` `process.rs:467-475` | `Indeterminate`, accepted or not | `effect/indeterminate` | no, parks |
| A line or checkpoint past a transport limit (0079) | `Exceeded::outcome` `process/limits.rs:101`; `process.rs:438`, `process.rs:656` | `Indeterminate` | `effect/indeterminate` | no, parks |
| Cleanup not proven (#403) | `finish` `process.rs:443-446`; `settled_outcome` `lib.rs:183-191` | `Indeterminate`, with `received` and `cleanup` kept | `effect/indeterminate` with `contracts/effect-cleanup.v1.schema.json` fields | no, parks |
| Engine restarted while the attempt was in flight | `engine.rs:942-958` | none | `effect/indeterminate`: "completion cannot be established" | no, parks |

Four rows are in bold. Each reports `Failed` for an attempt whose
executor's result the engine never received, after `start` had reached
the driver. In the first two, the `failed` is the driver's own
synthesis. A missing result file is a lost result, not the command's
report of failure; the seat contract makes the file the only channel the
engine reads. An invocation error after the spawn is the same: a
command that commits its work and then closes its stdin before reading
a prompt larger than the pipe buffer ends with the driver's `failed`,
and no result of the command's was read. `recv`'s arms are also how a handshake fails before `start`; only
after `start` are they a lost result. The row below the bold rows shows that the code already treats that fact as unknown
completion when the driver merely exits. The doc comment on `eof_outcome`
gives the deadline kill's reason: "we killed it before any result". That
is a fact about who ended the process, not about the effect. The driver
guide writes the inference down: the kill "is what makes non-completion
determinate, which is what makes bounded retry safe"
(`docs/guides/driver-authoring.md`, "Deadlines and kills"). Its outcome
table says a killed attempt parks, which holds only at a limit of 1: at
2 it is retried once first (`engine.rs:900`). Decision
0053 ruling 5 reached the same point from the other side: a killed attempt
"answers neither yes nor no" to whether a session opened.

`accepted` does not mark the first effect on every path. A driver
withholds it until a checkpoint proves a turn began (0053 ruling 1,
`begins_work` `adapters.rs:6400-6402`). Exec emits `exec-started` before
it runs its command (`adapters.rs:5637`), so for exec `accepted` precedes
the script. Dsh follows its session transcript by polling, so a turn's
tool call can run before the driver reads the row that would send
`accepted`. The refusal paths at `adapters.rs:6553` and `:6582` send
`accepted` and then `Failed` with nothing invoked, and so does the
invoke `Err` arm at `:6676` when the command was never spawned; the same
arm also carries errors from after the spawn. So whether
`accepted` arrived tells the engine nothing certain in either direction
about effects.

## The retry decision and its evidence

Fold (`crates/brokkr-core/src/fold.rs`) holds the line decision 0006
draws. `effect/requested` opens an effect with `failed_attempts: 0`
(`fold.rs:426`). `effect/failed` returns it to `ExecuteEffect` with the
count raised (`fold.rs:491`). `effect/indeterminate` parks it, "NEVER
auto-retried" (`fold.rs:515-516`). The engine compares the count with the
seat's `max_attempts` (`engine.rs:900`). It parks with the last error when
the limit is spent (`engine.rs:914-925`) and otherwise executes a fresh
attempt (`engine.rs:935`).

The evidence behind a retry is whatever the previous terminal event
carried. No event records that a retry was chosen or why. A retry shows
only as a second `effect/started` with a new `attempt_id`. The engine
adds `start_failure` and `start_failure_sites` to `effect/failed`
(`engine.rs:3109-3123`), and `chain_index` (`engine.rs:2848`) reads them
to pick the next model. Nothing else reaches the decision.

The limits are seat data. `max_attempts` defaults to 1 and
`timeout_seconds` to 3600 (`crates/brokkr-runtime/src/bundle.rs:361-367`).
Every seat of `recipes/self` reaches 2 through its agents and its extends
chain, `self` to `panel-review` to `fast` (`recipes/self/bundle.json:5`,
`recipes/panel-review/bundle.json:5`): intake, implement, review, verify
and ship. So a
deadline kill of the self implementer, after up to 90 minutes of work, is
retried once, on the same worktree.

## Identity: what is journaled and who reads it

`request_or_finish` (`engine.rs:1042-1075`) mints the effect: `effect_id`
is a fresh UUID (`engine.rs:1059`) and `idempotency_key` is
`"{run_id}:{seq}"`, the journal sequence of the `effect/requested` event
itself (`engine.rs:1062`, `engine.rs:1069`). It is written once per
effect, so every automatic attempt of that effect shares it. Each
attempt's `attempt_id` is a fresh UUID (`engine.rs:1359`). A panel's
members and a sequence's steps run under their effect's one `effect_id`
and one `attempt_id` (`engine/sequence.rs:38-39`), so the key names the
effect and no site within its body. The contract
names the key in prose only (`contracts/README.md:166`). No schema under
`contracts/` or `reference/` declares it.

No executor receives it. `Body::Start` carries `effect_id`, `attempt_id`,
`seat` and `input` (`lib.rs:44-49`), as `driver-protocol.v1` does. The
seat input does not carry the key. `effect_id` reaches the seat only
through `result_path` (`engine.rs:1152-1155`). The exec driver substitutes
`{workdir}`, `{prompt_file}` and secret references into its command
(`resolve_exec_part`, `adapters.rs:6367`), and no identity. A search of
`scripts/`, `recipes/` and `agents/` found no reader of `attempt_id`,
`effect_id` or `idempotency_key`. The key's only readers are fold-blind
projections and tests (`crates/brokkr-bridge/src/lib.rs:409`). The issue
called the key evidence of nothing, and that holds: no executor
deduplicates on it.

One capability does travel on the wire as a string: the driver's
`capabilities.supports` list. Its `resume` member decides whether the
engine offers a prior session (`process.rs:556-557`). It confers no
retry authority today. It is the precedent the addendum's ruling 3 is
written against.

An operator's `retry` re-requests the phase (`fold.rs:636-640`). The
engine then mints a new `effect_id` and a new key and resets the count.
The parked effect stays closed. A work seat's retry may still be offered
its prior harness session (`eligible_offer`,
`crates/brokkr-runtime/src/engine/resume.rs:500-523`; never a gate seat),
so the retried model can read what it did before. That is the session's
context, not evidence the engine reads.

## Effect classes

A compiled seat is `Single`, `Panel`, `Sequence` or `Select`
(`SeatBody`, `bundle.rs:132-158`). `Select` resolves to one of the others
from a journaled strategy. Exec is not a body: it is a driver,
`AdapterKind::Exec`, inside a `Single` body or a step, on the same
transport. Limits are per seat. `max_attempts` counts whole-body attempts,
and every member or step gets the full `timeout_seconds`.

| Class | Run path | A member or step fails | One is uncertain | On retry |
|---|---|---|---|---|
| Single (any adapter: claude, codex, dsh, lanetally, exec) | `run_driver`, `conclude_single` `engine.rs:1912` | the attempt fails | the attempt is indeterminate | the one driver runs again |
| Panel | `execute_panel` `engine.rs:2123`, `run_panel` `engine.rs:2278`, members concurrent | `panel_outcome` fails the attempt (`engine.rs:4291-4315`) | the attempt is indeterminate, with `unresolved_members` | every member runs again |
| Sequence, dialect steps included | `crates/brokkr-runtime/src/engine/sequence.rs:174-181` | `end_failed` (`sequence.rs:635`) | `end_indeterminate` (`sequence.rs:650`) | every step runs again from the first |
| Select | the selected body | as that body | as that body | as that body |

## What lies outside Brokkr's control

- **An agent session's remote actions.** A claude, codex or dsh seat
  acts through its harness. A boxed seat's commands run in the hands box,
  whose network the realm grants (decision 0043). The harness itself
  talks to its provider, and a networked seat can act remotely.
  `agents/release-manager.json` grants `network: true` with
  `max_attempts: 2`. Its charter leaves pushes, merges, tags and
  publication to the operator (`agents/charters/release-manager.md`), but
  no mechanism enforces that. No such action is visible to the engine except through
  the driver's outcome.
- **A git push.** No code under `crates/`, `recipes/`, `scripts/` or
  `agents/` pushes. The ship seat's script says of itself "write, then
  confirm, never commit or push" (`recipes/fast/scripts/ship-seat.sh`).
  Publication is the operator's (`recipes/release/README.md`).
- **Provider usage.** A lost or retried attempt has already paid for its
  tokens. Its usage is journaled only on checkpoints that reached the
  engine. Repeating an attempt repeats the cost. That is #224's
  cost-per-accepted-change question, not a correctness one.
- **The run's own worktree** is inside Brokkr's view. Every attempt of a
  seat runs in the same `workdir`, so an earlier attempt's edits, local
  commits and result file are in front of the retry and of every gate
  that later judges the tree.

## Method and limits

The inventory was read from source at `3ad0f66c` with `grep -n` and
direct reads, and each line number above was re-read before it was
written. No run was driven and no fault was injected: this entry states
what the code decides, not a measured incidence. How often a deadline
kill or a protocol violation after `start` occurs in the operator's
journals was not measured. The addendum's Asked paragraph names that cost
as open.
