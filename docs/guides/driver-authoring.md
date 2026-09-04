# Driver authoring — the `forge-driver/v1` wire contract

A **driver** is the process a seat runs. Brokkr ships adapters for
Claude Code, Codex, DSH, LaneTally and a generic `exec`, but nothing
about the engine is tied to those: a driver is any program, in any
language, that speaks `forge-driver/v1` over stdio.

This guide is that contract, written for someone outside this
repository. The normative sources are
[`contracts/driver-protocol.v1.schema.json`](../../contracts/driver-protocol.v1.schema.json)
for transport and
[`contracts/seat-record.v1.schema.json`](../../contracts/seat-record.v1.schema.json)
for the checkpoint and successful-result record, and the protocol section of
[`contracts/README.md`](../../contracts/README.md); where the shipped
engine is narrower than the schema, this guide says so.

- [Transport](#transport)
- [The message family](#the-message-family)
- [The exchange, in order](#the-exchange-in-order)
- [What the engine actually sends](#what-the-engine-actually-sends)
- [`resume` — rejoining the session you opened](#resume--rejoining-the-session-you-opened)
- [`accepted` is the load-bearing message](#accepted-is-the-load-bearing-message)
- [Checkpoints](#checkpoints)
- [Results](#results)
- [The result-file contract](#the-result-file-contract)
- [Deadlines and kills](#deadlines-and-kills)
- [A minimal driver, in prose](#a-minimal-driver-in-prose)
- [The conformance suite is the acceptance test](#the-conformance-suite-is-the-acceptance-test)
- [Wiring it into a bundle](#wiring-it-into-a-bundle)

The shipped verifier and shipper are not agents or bespoke wire
implementations. Their inline commands dispatch through the generic
`exec` driver and declare workspace hands, so the engine runs the whole
script through `brokkr hands exec`. The script reads the staged prompt
only for journal context and the result path, then writes the ordinary
typed result file.

## Transport

NDJSON over stdio, one JSON object per line.

- **engine → driver on your stdin.**
- **driver → engine on your stdout.**
- **stdout is protocol-only.** Any line that is not a valid protocol
  message fails the attempt closed. Do not print banners, progress bars
  or logs there.
- **stderr is captured as an artifact** and rides on failed and
  indeterminate outcomes. Put your logging there. It is masked for
  declared secret values on raw bytes before any string exists (decision
  0012), so an echoed token does not reach the journal.

Every message, in both directions, carries three keys:

```json
{ "proto": "forge-driver/v1", "msg_id": "<unique string>", "type": "<kind>" }
```

Unknown message types and schema-invalid messages fail closed: the
attempt errors and nothing is guessed at.

## The message family

```
engine → driver:  hello · start · resume · cancel · shutdown
driver → engine:  capabilities · accepted · checkpoint · result · cancelled
```

| Message | Direction | Required fields beyond the base three |
|---|---|---|
| `hello` | → driver | `engine_version` |
| `capabilities` | → engine | `driver`, `version`, `supports` (array; items from `resume`, `checkpoint`, `cancel`) |
| `start` | → driver | `effect_id`, `attempt_id`, `seat`, `input` (object) |
| `accepted` | → engine | `effect_id`, `attempt_id`; optional `session_ref` (string or null) |
| `checkpoint` | → engine | `effect_id`, `attempt_id`, `data` (object) |
| `result` | → engine | `effect_id`, `attempt_id`, `status` (`succeeded` \| `failed`); optional `result` (object or null), `error` (string or null) |
| `resume` | → driver | `effect_id`, `attempt_id`, `session_ref` |
| `cancel` | → driver | `effect_id` |
| `cancelled` | → engine | `effect_id` |
| `shutdown` | → driver | — |

## The exchange, in order

```
engine → { "type": "hello", "engine_version": "0.5.0" }
driver → { "type": "capabilities", "driver": "my-driver", "version": "1.0.0", "supports": [] }
engine → { "type": "start", "effect_id": "fx", "attempt_id": "a1",
           "seat": "implement", "input": { … } }
driver → { "type": "accepted", "effect_id": "fx", "attempt_id": "a1", "session_ref": null }
driver → { "type": "checkpoint", "effect_id": "fx", "attempt_id": "a1", "data": { … } }
driver → { "type": "result", "effect_id": "fx", "attempt_id": "a1",
           "status": "succeeded", "result": { "result": "complete", "notes": "…" } }
engine → { "type": "shutdown" }
```

Rules over that sequence:

- **`capabilities` must be your first message.** The engine sends
  `hello` and then reads exactly one message. Anything other than
  `capabilities` fails the attempt with `expected capabilities, got …`.
- **Exactly one `result` per `start`.** The engine returns as soon as it
  reads one.
- **Every `effect_id` you send back must match the one from `start`.**
  An `accepted`, `checkpoint` or `result` naming a different effect
  fails the attempt.
- **Any message type other than `accepted`, `checkpoint` or `result`
  after `start` fails the attempt.**

## What the engine actually sends

The subprocess transport
(`crates/brokkr-protocol/src/process.rs`) is narrower than the schema,
and you should build against the narrower shape:

- **`hello`, an optional `resume`, and `start` are the only messages
  sent before your result.**
- **`shutdown` is sent once, after your result or after EOF**, followed
  by the engine closing your stdin. Treat both as "stop now"; exit
  cleanly on either.
- **`cancel` is defined and honored by the built-in adapters, but the
  engine does not currently send it.** Cancellation happens as a process
  kill on deadline expiry, not as a protocol message. Implement
  `cancel` → `cancelled` if you want the contract covered; do not rely
  on receiving one.
- **`supports` is read on exactly one axis: `resume`.** The engine will
  not offer a session to a driver that did not declare it can rejoin
  one. Nothing else in `supports` is branched on — the built-in adapters
  emit checkpoints and answer `cancel` whatever they list. Fill it in
  honestly anyway.

## `resume` — rejoining the session you opened

The engine may hand a seat back the session that seat's own earlier
attempt opened, so a retry or a phase-machine re-entry continues a warm
provider session instead of paying for a cold one (decision 0030; the
measured effect on codex is a cache-read ratio of ~74% cold against ~88%
on the resumed attempt).

```
engine → { "type": "hello", "engine_version": "0.7.0" }
driver → { "type": "capabilities", "driver": "my-driver", "version": "1.0.0",
           "supports": ["resume"] }
engine → { "type": "resume", "effect_id": "fx", "attempt_id": "a2",
           "session_ref": "<the id you reported earlier>" }
engine → { "type": "start", "effect_id": "fx", "attempt_id": "a2",
           "seat": "implement", "input": { … } }
```

What a driver has to do to participate:

- **Advertise `"resume"` in `supports`.** Without it the engine never
  sends the message, and every attempt starts cold.
- **Report the transcript.** Use the common `session_meta.transcript`
  shape described below. For a resumable harness its `locator` is the
  session or thread id. Report it as soon as your harness announces one,
  not only when the session ends — an attempt killed on its deadline is
  exactly the attempt whose retry wants the id.
- **Handle `resume` as a modifier on the `start` that follows it.** It
  arrives BEFORE that `start` and carries no `seat` and no `input`: it
  is the session handle for the attempt the next `start` describes, and
  nothing else. One offer belongs to one seat; a `start` with no
  `resume` in front of it is a cold start.
- **Rejoin that session itself.** What you resume is your provider's own
  session, by the id you captured — never a fork, a copy, or a new
  session seeded with the old transcript. A session belongs to the
  credential and client that opened it; resuming it from anywhere else
  is a terms violation with the account at the end of it, not a clever
  optimisation.
- **Re-express every restriction the seat declared.** If your harness's
  resume path drops a sandbox class, a permission mode or a tool
  allow-list, put it back explicitly. Where you cannot, start cold and
  say why in a checkpoint. A resumed attempt that runs with more power
  than its seat declared is the failure this whole mechanism is fenced
  against — the codex adapter translates its seat's `--sandbox <class>`
  into `-c sandbox_mode="<class>"` for exactly this reason.
- **Treat a refused resume as a cold spawn, not a failure.** An unknown
  or expired session is not the attempt's fault: start cold, journal the
  refusal, and carry on.

What the ENGINE checks before it offers you anything (decision 0030
ruling 4). Every one of these is a refusal — the offer is made only when
all of them agree:

- the same run and the same seat: a retry of that seat, or a re-entry
  into it. Never another seat, and never another run;
- the same driver binary and the same resolved model, provider and agent
  as the attempt that opened the session. A decision-0016 chain fallback
  to another model gets nothing;
- the same pinned bundle: an edited adapter declaration, a rewritten
  charter or an upgraded engine moves the run's pin, and a moved pin
  offers nothing;
- the same machine and the same account: the journal is still where it
  was when it started the run. A run adopted from elsewhere (decision
  0027), a journal file carried to another host, or an installation that
  cannot identify itself at all — each gets nothing. This one is not
  read out of the chain, because 0027 makes an adopted run deliberately
  indistinguishable inside it; the store answers it from bookkeeping an
  export does not carry.

And what the engine does NOT check, because it cannot: **whose
credential your provider will bill.** It compares journaled facts and
one local fingerprint. It does not hold your API key, cannot ask your
provider who owns a session, and cannot tell that two attempts a second
apart ran with a re-pointed `CODEX_HOME` or a rotated token. Everything
above narrows the offer to one seat of one run on one machine; the last
step — "and this session is really mine to rejoin" — is yours. Fail
closed on a handle your harness does not recognise, and treat the
provider's refusal as the answer it is (a cold spawn), not as an error
to work around.

The engine journals nothing about the offer itself: whether it made one
is a function of the journal a reader already has, plus whether that
journal is still at home. What you DID with it — rejoined, or started
cold and why — is yours to journal, in a checkpoint, and it is the only
record of it there will be. Send one.

## `accepted` is the load-bearing message

Send `accepted` as soon as you have committed to the work — before
spawning your agent session, not after it finishes. It is the single bit
the engine uses to tell two failures apart:

| What happened | Outcome | What the engine does |
|---|---|---|
| Your process exits **without** `accepted` and without a result | `indeterminate`, reason `driver exited before accepting the attempt` | The run **parks**. |
| Your process exits **after** `accepted` and without a result | `indeterminate`, reason `driver exited after accepting, before a result — attempt cannot be established as complete` | The run **parks**. |
| You send `result` with `status: "failed"` | `failed` | A retry may follow, inside the seat's `max_attempts`. |
| You violate the protocol | `failed` (driver defect) | A retry is a new attempt. |

The reason indeterminacy always parks rather than retrying is decision
0003: the engine cannot distinguish "did nothing" from "already opened a
billed session and lost the pipe." A silent retry could duplicate — or
re-pay for — completed work, so it never happens automatically. Only an
operator's `retry` command moves a parked run.

Practical consequence: **an `accepted` you send too late is a park**,
and **an `accepted` you send and then abandon is also a park**. If you
know you cannot do the work, send a `result` with `status: "failed"` and
an `error` string. A determinate failure is strictly better for the
operator than an indeterminate one.

The schema requires `accepted` before checkpoints and results as the
protocol's shape. The shipped transport does not currently reject a
`result` that arrives without one — but it records `accepted: false` on
the attempt report, and that bit is what the fail-to-start fallback
predicate reads. Send it.

## Checkpoints

A checkpoint is a bounded fact about progress. Send as many as you like;
the engine journals each one as `effect/checkpointed`.

**The journal is evidence, not transcript.** The closed
[`seat-record/v2`](../../contracts/seat-record.v2.schema.json) vocabulary is
the definition of a seat's per-turn checkpoint, finishing checkpoint, and
successful result. It is `v1`'s vocabulary plus the two fields decision
0035 added — `effort` and `reasoning_output_tokens`, both below — and
[`v1`](../../contracts/seat-record.v1.schema.json) is unedited and
remains the contract for every record written under it: a record is
validated against the version its run's engine wrote, never against
whichever is newest. Checkpoints may carry only:

- bounded turn / tool / usage fields — turn counts, token counts, cost,
  exit codes, a step name, a tool name;
- or a **file-path-only** target.

They must **not** carry prose, commands, or reasoning. A model's output,
a shell command line, a diff, a rationale: none of these belong in a
seat record. Numeric accounting is absent when a harness does not report it;
zero and string sentinels are not measurements and must not be written.
Before export the Looper bridge hashes targets and withholds
transcript locators outright (only `observed-redacted` leaves), and the
full session transcript stays wherever your harness put it.

Two field names are read by `brokkr costs` if you supply them:
`num_turns` (summed into the seat's turn count) and `total_cost_usd`
(summed into its cost). A third, `model`, is the provider-reported model
id displayed by every read surface (decision 0031). Put it on every
turn checkpoint. If the harness cannot report it, write the literal
`not reported`; never copy the configured pin or adapter default into
this evidentiary field. A model-free driver writes `not applicable`.

**`model` is the provider's claim, not proof.** No harness discloses
quantization, hardware routing, or a substitution made at peak load, and
the same model string comes back whichever of those happened. It is the
best-attested name available and worth strictly more than a pin — which
is why decision 0031's rule stands unchanged and nothing may write a
pin, an adapter default, or an abstract agent name into it — but it is
testimony from the party being audited, and this ledger does not call
testimony proof (decision 0035 ruling 2). What survives that scrutiny is
not a name but a meter: tokens, money and elapsed time are costly to
fabricate and are what settle a dispute. Label the field accordingly in
anything you build on it; every built-in readout does.

**`effort` is configuration, and it is a fourth field to put on every
turn.** It is the effort the model was configured with, as *the harness
itself echoes it back* — the value that applied after every profile and
plugin layer, which is strictly better evidence than reading back our
own bundle's pin, and is the reason this is read rather than copied.
Both harnesses that report one report it per turn, so a thread that
changes effort mid-seat says so turn by turn. It is never dressed as a
report of what the model did: a configured effort is worth recording and
is not worth trusting, and `deepseek-v4-flash` is the lane that proves
it — measured on 2026-09-03, it spent roughly a *seventh* as much
reasoning at `high` as at `low`. The configuration said one thing, the
meter said another, and only the meter could say so.

The two sentinels are decision 0031's, reused rather than reinvented,
and the distinction between them is load-bearing:

- `not reported` — the harness's lanes carry a real effort control, but
  neither it nor the providers behind it echo any value, so there is
  nothing to read. This is dsh exactly.
- `not applicable` — the driver has no model turn at all. This is `exec`.

A control that exists but goes unreported is not a control that does not
exist, and the record must not blur the two.

A level is one bounded word: up to forty characters of ASCII
alphanumerics, `-`, `_`, `.` and `:`, starting with an alphanumeric —
`seat-record.v2`'s `effort` pattern exactly. Clamp what your harness
echoes against that shape *at the boundary*, as the built-in folds do,
and record no effort for a turn whose echo fails it. The value crosses
into an append-only journal: an echo journaled unclamped is refused
later at export, and the cost of that refusal is not one turn's missing
field but the whole run's export.

**`reasoning_output_tokens` is a reported subset of `output_tokens`,**
in exactly the way `cache_read_tokens` is a subset of `input_tokens`,
and it is never added to a total a second time. The three built-in
harnesses meter it at three different granularities and the field admits
all three without inventing the others: codex reports it per turn (from
`turn.completed.usage`), claude only in its result (from
`output_tokens_details.thinking_tokens`), and a dsh lane's providers
report it per call though the `headless` profile discards it before
brokkr sees it. **Where a harness reports no figure for a record, the
field is absent — never zero, and never back-filled from the run
total.** Zero is a measurement, and a harness that stayed silent did not
make it.

**Every driver speaks per turn, not only at exit.** This is the standard
each built-in driver meets and the one a new driver is held to: every
assistant turn becomes at least one checkpoint while the process is
still running; the turn count and the harness's usage ride in
`num_turns` and `total_cost_usd` (token counts in `input_tokens`,
`output_tokens`, `cache_read_tokens`, `cache_write_tokens`) accumulated across the whole
session, not overwritten per turn; and the harness's transcript locator
lands in `session_meta` the moment the harness reveals it, not at exit.
`brokkr watch`, the tui and the seat cost surfaces all read
checkpoints, so a driver that speaks only at the end is invisible to
every one of them while it works. Report nothing you were not told:
a harness that reports no cost leaves `total_cost_usd` absent rather
than claiming zero. Where the harness offers no event stream on stdout,
follow whatever it writes as it goes — the dsh driver pins its own
per-seat session-transcript root through the launcher's `--patch`
overlay and tails the one file that appears there, which is
unambiguous by construction and never a scan for the newest file.
And however you watch it, **conclude**: a driver that cannot tell
whether its child is still alive reports the failure and exits. Treating
a failed `wait` as "not finished yet" reproduces the silent seat this
standard exists to prevent, by a longer road.

**A tailed file is a weaker source than a pipe.** Stream-json on a pipe
can only have come from the child; a session log on disk sits where the
seat's own agent can append to it, and the driver publishes its path in
the shared transcript row. Fold such a file only under the clamps above
— a bounded id, a bounded tool name, numeric counts — and never derive a
path to execute, a command, or a control decision from it. The journal
then holds, at worst, a number a compromised seat chose; it never holds
anything that acts.

**`input_tokens` is inclusive of `cache_read_tokens`.** Harnesses split
that count both ways — codex reports `input_tokens: 14830` beside
`cached_input_tokens: 11264`, dsh reports the same step as `inputTokens:
94, cacheReadTokens: 7` from a `prompt_tokens: 101` — so a driver
normalizes to the inclusive form before journaling, and reports the
cache read separately in `cache_read_tokens` as the subset it is. A harness
that reports cache creation writes it as `cache_write_tokens`; it is shown
separately and is not part of the input-plus-output total.
`brokkr costs` and the seat surfaces sum `input_tokens` and
`output_tokens` only; adding the cache read would double-count it. One
journal key means one thing across every driver, not whatever its
harness happened to mean.

**The sum rules, stated once and in full.** Every read surface derives
the same way, and a driver that journals under these names gets that
derivation for free:

| Field | Relationship | Summed into the total? |
|---|---|---|
| `input_tokens` | Inclusive of `cache_read_tokens` | **Yes** |
| `output_tokens` | Inclusive of `reasoning_output_tokens` | **Yes** |
| `cache_read_tokens` | A reported subset of `input_tokens` | No — shown beside it |
| `reasoning_output_tokens` | A reported subset of `output_tokens` | No — shown beside it |
| `cache_write_tokens` | Neither an input nor an output; its own axis | No — shown beside it |

So `total_tokens` is exactly `input_tokens + output_tokens`, and the
three subset/adjacent fields are displayed and never re-added. A surface
that summed a subset a second time would inflate the figure the wager
compares seats on, which is why the rule lives here rather than in each
surface.

The efforts are not summed at all: `effort` is one configured value per
record, and a seat whose members ran at different levels reports all of
them rather than reducing to one. The view carries the plan's **pin** and
the harness's **applied** value as two separate cells, labels both as
configuration, and fills neither from the other — a run journaled before
decision 0035 shows two visible absences rather than one borrowed answer
(ruling 6).

**Every transcript is the operator's, and it stays.** Every driver sends
a `checkpoint` whose `data.step` is `transcript`, and repeats the same
object in its finishing `session_meta`:

```json
{"kind":"codex-thread",
 "locator":"019c…",
 "home":"/home/operator/.codex"}
```

The locator is always a path or id clamped to 80 characters. The closed
kinds and their homes are shared by the built-in base:
`claude-session` uses `~/.claude/projects`, `codex-thread` uses
`$CODEX_HOME` or `~/.codex`, and `dsh-session` uses `$DSH_HOME` or
`~/.dsh`. Claude and Codex put their session or thread id in `locator`.
DSH stages and retains one root below
`<dsh-home>/sessions/brokkr/<seat>/` and records its forward-slashed
path relative to the separately recorded home. A driver with no
transcript, including `exec`, reports `kind: "none"` with empty
`locator` and `home` rather than omitting the row.

The shape, clamp, retention, and journal row are shared; an adapter arm
supplies only its harness locator. Drivers never delete transcripts.
They record paths or ids only, never prompts, tool arguments, tool
results, or other prose: paths in, prose out (decision 0032).

## Results

Exactly one `result` per `start`.

```json
{ "type": "result", "effect_id": "fx", "attempt_id": "a1",
  "status": "succeeded",
  "result": { "result": "complete", "inputs": { "fixes_applied": true },
              "notes": "…" } }
```

- `status: "succeeded"` **must** carry a `result` object. A succeeded
  result with a null payload is converted to a failure with the error
  `succeeded result carried no payload`.
- `status: "failed"` should carry an `error` string; the engine
  substitutes `driver reported failure` if you omit it.

The `result` object you send is what the phase machine rules on, and it
has its own shape:

| Key | Meaning |
|---|---|
| `result` | **Required string**, and it must be one of the seat's declared `results`. Anything else — a non-object payload, a missing `result` string, or a string outside the vocabulary — fails schema validation and **parks** the run with the raw evidence attached. It is never repaired, coerced, or handed to a model to fix. |
| `inputs` | Optional typed facts for the table. Only the seat's **declared** inputs survive; engine-owned keys and undeclared claims are dropped before evaluation and never enter the journal record (decision 0007). |
| `notes` | Optional human summary. Display and evidence only. |
| `model` | The provider-reported served model, under the same rules as checkpoint `model` — the provider's claim, not proof; required of new driver results by decision 0031 even though the result object remains extensible for wire compatibility. |
| `effort` | The effort configured for that model, as the harness echoed it, under the same rules and the same two sentinels as checkpoint `effort`. Configuration, never a report (decision 0035 ruling 3). |
| `reasoning_output_tokens` | The reasoning subset of `output_tokens` for the whole session. This is the only place claude can report one; absent, never zero, where the harness reported none. |

Your driver does not decide the next phase and should not try to. It
reports a typed result; the pinned policy table rules.

## The result-file contract

The built-in adapters do not ask the agent to speak the protocol. They
put the shape into the prompt and read a file back — and any driver
wrapping an interactive agent CLI will want the same pattern.

The `start` message's `input` object carries, for a single seat:

```json
{
  "feature":         "the run's feature text",
  "phase":           "implement",
  "seat":            "implement",
  "role_path":       "<abs path to the seat's charter .md>",
  "workdir":         "<abs path the seat works in>",
  "result_path":     "<workdir>/.forge/results/<effect_id>.json",
  "allowed_results": ["complete", "broken", "blocked"],
  "context":         { "run_id": "…", "last_decision": { … } }
}
```

Panel seats carry a `members` object instead of `role_path`/`result_path`
(one `role_path` + `result_path` per member); sequence seats carry a
`steps` array in execution order. A seat with declared secret bindings
additionally carries `secrets` (the **names**) and `secrets_file` (the
store path) — never values.

`context` also carries `returned_from` when the run has entered this
phase more than once: the phase and result that sent it back, so a
reforged implementer reads the review findings it has to answer.

The adapters then:

1. Read the charter at `role_path`, append the feature, phase, workdir,
   the pretty-printed `context`, and a **Result contract** block naming
   `result_path` and the `allowed_results` vocabulary verbatim.
2. Spawn the agent CLI with that prompt (on stdin, or written to a file
   and passed as `{prompt_file}`).
3. Enrich the parsed result object with the served `model` learned from
   the harness; the agent-authored file does not supply this field.
4. On exit: a nonzero exit code is a `failed` result naming the code. A
   missing result file is a `failed` result with `seat wrote no result
   file (the result contract was not met)`. An unparseable one is
   reported as a succeeded result whose payload is
   `{"__unparseable_result_file__": "<parse error>"}` — which then fails
   schema validation and parks with the evidence, because adapters
   repair nothing.

The prompt states plainly that the file is the only channel the engine
reads, and that printing the JSON instead of writing it counts as
producing no result. Reuse that framing; it is the difference between a
seat that works and a seat that parks every run.

## Deadlines and kills

The seat's `limits` in the bundle bound every attempt:

- **`timeout_seconds`** (default 3600). The engine arms a watchdog at
  spawn. On expiry it **kills your process** and reports the attempt as
  a determinate `failed`: `attempt exceeded its <N>s deadline and was
  killed`. The kill is what makes non-completion determinate, which is
  what makes bounded retry safe (decision 0006). You will not receive a
  `cancel` first.
- **`max_attempts`** (default 1). A determinate failure may be retried,
  with a fresh `attempt_id`, up to this limit. Exhausting it parks the
  run.

What this asks of you: do your own cleanup on process exit, and do not
assume you will be told the deadline is near. If your work has a natural
checkpoint boundary, emit a checkpoint at it — a killed attempt's
checkpoints are already in the journal, and they are what the operator
reads to see how far it got.

## A minimal driver, in prose

A "hello world" driver for a seat that always resolves. No code here on
purpose — the sequence is the whole thing, and it is short in any
language.

1. **Set up.** Read stdin line by line. For each line, parse JSON;
   ignore anything that does not parse (the engine speaks the protocol,
   so noise on your stdin is not yours to interpret). Write every
   outgoing message as one compact JSON line to stdout, then flush —
   the engine reads line-by-line and a buffered write looks like
   silence. Generate a fresh `msg_id` per outgoing message.

2. **On `hello`.** Emit `capabilities` immediately: `proto`, a fresh
   `msg_id`, `"type": "capabilities"`, your `driver` name, your
   `version`, and `supports` — `[]` is honest for a first driver.

3. **On `start`.** Read `effect_id`, `attempt_id`, `seat` and `input`
   out of the message and hold them; every message you send from here
   carries the first two.

   a. Emit `accepted` **before doing any work**, with `effect_id`,
      `attempt_id`, and `session_ref` — your harness's session handle if
      it has one, otherwise `null`.

   b. Do the work. If you are wrapping an agent CLI, this is where you
      compose the prompt from `input.role_path` and the result contract
      naming `input.result_path` and `input.allowed_results`, then spawn
      it in `input.workdir`.

   c. Emit one or more `checkpoint` messages as you go, each with a
      `data` object of bounded fields only — a step name, a turn count,
      an exit code, `num_turns`, `total_cost_usd`. No prose.

   d. Emit exactly one `result`. On success: `"status": "succeeded"` and
      a `result` object whose `result` field is one of
      `input.allowed_results` — for the always-resolving driver, the
      first entry. On any failure you can name: `"status": "failed"`
      with an `error` string. Never both, never neither.

4. **On `shutdown`, or on stdin EOF.** Exit 0.

5. **On `cancel` (optional).** Emit `cancelled` with the `effect_id` and
   exit. The engine does not send this today, but the contract has it.

The two ways this goes wrong, both worth testing deliberately: writing
anything non-JSON to stdout (fails the attempt closed), and exiting
after `accepted` without a `result` (parks the run).

## The conformance suite is the acceptance test

`crates/brokkr-cli/tests/driver_conformance.rs` is how the built-in
adapters are proved, and it is the pattern to copy. It does not mock the
engine: it drives the real binary.

The shape of each case:

1. Write a **shim** — a small shell script standing in for the agent CLI
   — and point the adapter at it with an env override. The suite has
   four, and each one pins a different property:
   - `OBEDIENT_SHIM` finds the result path in the prompt and writes a
     typed result there — the happy path and the result-file contract.
   - `CLAUDE_STREAM_SHIM` emits `stream-json` including a deliberate
     non-JSON noise line the adapter must drop, and two `tool_use`
     blocks in one message — checkpoint extraction under realistic
     streams.
   - `CODEX_JSON_SHIM` emits Codex's thread/turn/item events including a
     command execution with output — proving the adapter reports usage
     totals and does **not** put the command or its output into a
     checkpoint.
   - `SILENT_SHIM` consumes stdin and produces nothing — the
     no-result-file failure path.

2. Spawn the driver as a subprocess and write three lines to its stdin:
   `hello`, `start` (with a fully-formed `input` including
   `result_path`, `allowed_results` and `workdir`), and `shutdown`.

3. Collect stdout, parse **every line** as JSON — a line that does not
   parse is itself the failure — and assert:
   - a `capabilities` message came first;
   - an `accepted` arrived for the right `effect_id`;
   - checkpoints, if any, carry only bounded fields;
   - **exactly one** `result` arrived, with the expected `status`;
   - on the obedient path, the result payload is the typed JSON the shim
     wrote to `result_path`.

Replicate that harness against your own driver and you have covered the
contract. Two properties worth adding cases for, because they are the
ones outside drivers get wrong: a non-JSON line on stdout must be
treated as a defect by your own test (the engine will fail the attempt),
and an abandoned attempt after `accepted` must produce the park you
expect rather than a silent success.

**One honest limit: this suite is `#![cfg(unix)]`.** It does not compile
or run on Windows, because its shims are `/bin/sh` scripts. Windows
driver conformance is therefore **not verified by CI today**. The engine
itself is tested on Windows in the `engine` job; the driver-adapter
conformance layer specifically is not. If you are writing a Windows
driver, you are the first line of testing for it.

## Wiring it into a bundle

A seat names your driver as argv:

```json
"implement": {
  "role": "roles/implementer.md",
  "results": ["complete", "broken", "blocked"],
  "limits": { "max_attempts": 2, "timeout_seconds": 5400 },
  "driver": { "command": ["./drivers/my-driver", "--flag", "value"] }
}
```

A `./`-prefixed entry is resolved relative to the bundle directory.
`{brokkr}` expands to the engine's own executable, which is how the
built-in adapters are named (`["{brokkr}", "driver", "claude", "--", …]`).
The manifest records driver names, never resolved argv, because the
expansion is machine-local.

Optional confinement:

```json
"driver": {
  "command": ["./drivers/my-driver"],
  "confine": { "image": "…", "network": false, "mounts": ["…"] }
}
```

Then check it before you run it:

```
brokkr doctor --bundle my-bundle    # is the driver actually reachable here
brokkr compile --bundle my-bundle   # does the bundle hold together
```

If your driver does not need a protocol implementation at all — you just
want to run a command and have it honor the result-file contract — the
built-in `exec` adapter already speaks `forge-driver/v1` on your behalf:
`["{brokkr}", "driver", "exec", "--", "bash", "./my-script.sh",
"{prompt_file}"]`. `recipes/sdd`'s boxed `speckit-check` gate is an
example.

## See also

- [`contracts/driver-protocol.v1.schema.json`](../../contracts/driver-protocol.v1.schema.json)
  — the normative message schema.
- [`contracts/README.md`](../../contracts/README.md) — the protocol
  section and the fold semantics that consume your results.
- [recipe-authoring.md](recipe-authoring.md) — the seat that names your
  driver.
- [decision 0003](../decisions/0003-native-rust-runtime.md) — why
  indeterminate parks.
- [decision 0006](../decisions/0006-bounded-attempts-and-deadlines.md) —
  attempts and deadlines.
- [decision 0012](../decisions/0012-sealed-secret-bindings.md) — secret
  names on the wire, values never.
