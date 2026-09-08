# 0053 — A provider's pre-session refusal is a failure to start, and the driver classifies it

Status: proposed
Date: 2026-09-08

## Context

Decision 0016 makes fallback narrow and structural: an attempt that
**fails to start** — the driver binary is absent, the provider rejects
the model, or no `Accepted` message ever arrives — retries on the next
model in the chain, and a mid-session failure does not. The engine
mechanises that boundary in `failed_to_start`: `Failed`, never
`Accepted`, no checkpoint. The guide
(`docs/guides/agent-library.md`, the limits section) already names the
consequence and the fix: "No `Accepted` ever arrives" parks, and "the
honest fix is at the driver — report a provider's pre-session model
rejection as a determinate failure — not at the engine".

Issue #219 measured the gap between that rule and the drivers. The
claude driver answers `accepted` the moment the CLI spawns. The
account's limit refusal (`rate_limit`, "You've reached your ... limit")
arrives on the stream after that, so the engine sees an accepted attempt
that failed and — correctly, under 0016 — refuses to fall back. On run
`decision-0046-enactment-slice-i--165601b4` the engine smith chained
`fable@high -> opus@high`, exited 1 twice with empty stderr (seq
3646/3652), both seat transcripts carrying `rate_limit`, while a probe
seconds later answered `ok` on both `claude-fable-5-1` and
`claude-opus-5`: the limit is per model, opus was serving throughout, and
the chain's own second link would have taken the work. The same run
parked this way once before in design (seq 2133). Two of that run's six
parks are this bug.

The refusal is not a mid-session failure in fact. It is a refusal to
start: the provider rejected the request before any inference, so no
turn opened and no work exists for a different model to fail to inherit.
What is wrong is not 0016's boundary but the driver's report of which
side of it the attempt stands on.

Withholding `accepted` moves one other shape, and a review of the first
delivery measured it: an attempt the engine's own deadline watchdog
KILLS. The watchdog SIGKILLs the driver tree, so the driver reports
nothing at all; `eof_outcome` calls that `Failed` — the kill is what
makes non-completion determinate under decision 0006 — and with
`accepted` no longer sent at spawn, a first turn that simply hangs
reaches the engine as `Failed`, never accepted, no checkpoint. That is
the structural fail-to-start predicate, so a vendor outage that hangs
the CLI would walk the chain down every link, each one hanging for a
full deadline, and journal per-model `start_failure_sites` for one
vendor-wide stall. Nothing about that attempt says a provider refused
it; the only fact is that we ended it. Ruling 5 answers the question the
withholding forced.

Alternatives weighed:

- **Widen the engine's predicate to treat an accepted attempt with no
  checkpoint as a failure to start.** Rejected: `accepted` means the
  session opened, and the engine reading a driver's timing to guess that
  it did not is the control-plane inference decision 0001 forbids. The
  guide already says the fix belongs at the driver.
- **Sniff stderr for "limit", "quota", "rate_limit".** Rejected: that is
  a model reading a provider's prose to make a control decision
  (decision 0001), and it would be a classifier that drifts with every
  provider's wording. Only machine-readable wire records may decide it.
- **Withhold `accepted` from every driver until the attempt ends.**
  Rejected: `accepted` is what tells the engine a session opened and
  starts the live readout; deferring it to the end would blind the run
  for its whole life. It is withheld only until a checkpoint proves a
  turn began.
- **Make every built-in classify a refusal, including dsh and exec.**
  Rejected: dsh's headless profile emits no machine-readable pre-session
  refusal and exec has no model turn at all. Inventing a classification
  from their stderr prose is the same forbidden read.
- **Move the hold from the driver to the engine.** The engine survives
  the kill it orders, so it could journal the pre-session rows it was
  holding when its own watchdog fired and drop them only for a
  classified refusal — which would close ruling 8's window entirely.
  Rejected for this delivery, and recorded here because it is the one
  design that does: it puts a driver's `step` vocabulary inside the
  engine's journaling path, so what reaches the record would depend on
  the engine reading rows it otherwise only appends, and the seat-record
  fence (decision 0034 ruling 6) would judge a held row at a different
  moment than it was written. The cost ruling 8 names is a cold retry in
  a bounded window; that is smaller than the seam this would open, and
  the operator can rule otherwise on the evidence rather than on the
  guess.
- **Let the deadline kill fall where the withholding puts it, and
  record the widening.** Rejected: it makes the chain's own bound
  meaningless exactly when a vendor is down — the case fallback exists
  for is a provider that refuses fast, not one that hangs — and it
  journals a per-model start failure for a fact about no model. The
  boundary this decision draws is "did a provider refuse before its
  first turn"; an attempt the engine killed answers neither yes nor no.

## Rulings

1. **A provider refusal recorded before the first turn is a determinate
   failure to start.** A driver withholds `accepted` until a checkpoint
   proves the harness began work — the first record that is neither the
   transcript locator nor the harness launch row. A refusal recorded
   before that point is reported as `result: failed` carrying the
   refusal's own machine token and, where the harness gave one, its HTTP
   status and a bounded excerpt of its message, with **no `accepted` and
   no checkpoint**. That is exactly the engine's structural
   `failed_to_start`, so the chain advances to the next judge as it does
   for a driver the machine cannot reach at all. The refused attempt
   stays in the journal with its reason; neither the fold nor any readout
   invents a success for it.

2. **The boundary does not move.** A refusal that arrives after the
   first turn is what decision 0016 says it is — a mid-session failure
   that follows decision 0006 unchanged. This ruling classifies only the
   pre-session shape; it widens nothing about mid-session failure, and
   `accepted` still reaches the engine before any checkpoint on every
   path that starts work. The one other shape the withheld `accepted`
   would otherwise have moved — an attempt the engine's own watchdog
   killed — is held on its old side by ruling 5, not left to fall.

3. **The classification is the driver's, per built-in adapter, from
   machine-readable records only.** No classifier reads model or
   provider prose. Each built-in states what it does with the shape:

   | Adapter | Wire protocol carries a pre-session refusal? | What the driver does |
   |---|---|---|
   | `claude` | Yes: the synthetic `assistant` record with `isApiErrorMessage` and an `error` token (`rate_limit`, `authentication_error`, …), the error `result` record with `is_error`, or a `rate_limit_event` lifecycle message | Classifies it before the first counted turn; refuses to start |
   | `lanetally` | Yes: byte-for-byte the claude stream | Same arm, same classifier |
   | `codex` | Yes: the harness's own `error` event, or a `turn.failed` before the first `turn.started` | Classifies it before the first counted turn; refuses to start |
   | `dsh` | No: the headless profile prints only its final answer, and a provider rejection reaches the driver as stderr prose plus a non-zero exit | Classifies nothing; a refusal follows decision 0006 unchanged |
   | `exec` | Not applicable: no model turn and no provider to refuse it | Classifies nothing; its failures are the script's own |

   **Enforcement binding:** the classifiers and the withheld-`accepted`
   path in `crates/brokkr-protocol/src/adapters.rs`;
   `crates/brokkr-protocol/src/adapters/tests.rs` folds each measured
   record shape and pins that a record after the first turn is not
   classified; `crates/brokkr-cli/tests/driver_conformance.rs` drives a
   refusing shim per built-in adapter; `docs/guides/agent-library.md` and
   `docs/guides/driver-authoring.md` state the table above.

4. **A refusal keeps the refused attempt with its reason, and the chain
   descent is journaled.** The reason is the provider's own token, never
   a verdict about the model; the engine's `start_failure` fields record
   which site failed to start, so the readout shows the chain descending
   and why. No success is synthesized for the refused attempt.

   **Enforcement binding:**
   `crates/brokkr-runtime/src/engine/agent_tests.rs` proves a
   refusal-shaped report is `failed_to_start` and advances `chain_index`
   to the next candidate; the existing fold-blindness test keeps every
   new field out of the fold.

5. **An attempt the engine's own watchdog killed is not a failure to
   start.** The deadline kill makes non-completion determinate
   (decision 0006), which is why its outcome is `Failed` and not
   `Indeterminate` — but the driver died with no chance to say whether a
   session had opened, so the question this decision answers has no
   answer for it. `AttemptReport` carries the fact (`deadline_killed`),
   the structural predicate reads it as its fourth term, and a hung
   attempt stays on the mid-session side of the boundary exactly as it
   did before `accepted` was withheld. This includes the one shape that
   changes in the other direction: a driver that hangs its whole
   deadline BEFORE the handshake used to descend the chain and now
   parks — the same ambiguity decision 0003 parks on, reached by a
   driver that stalled rather than one the machine could not reach,
   which fails immediately and still descends.

   **Enforcement binding:**
   `crates/brokkr-protocol/src/process/tests.rs`'s deadline-kill test
   pins the fact on the report; the structural-predicate test in
   `agent_tests.rs` pins that a killed attempt is not a failure to start.

6. **A refusal is bounded wire data, and it names its own evidence.**
   Every field the classifiers read — the token as much as the prose
   excerpt — comes off the harness's stream, so all of it is
   whitespace-collapsed, stripped of control characters and clamped
   before it enters an append-only record: the token to 80 characters
   like a tool name, the excerpt to 160. A refused attempt emits no
   checkpoint, so the transcript locator it would have carried rides its
   reason instead (`[transcript <kind>/<locator>]`) — that transcript is
   the evidence #219 itself was diagnosed from, and a refused attempt
   that named nowhere to look would make the next such diagnosis start
   from scratch.

7. **A classified refusal does not end the fold.** The driver keeps
   reading its harness's whole stream after classifying one: a harness
   that reported an error and then went on to work has not refused to
   start, and its turns are the seat's served model, usage, cost and
   resumable session id (decision 0030). Whether the refusal ends the
   attempt is settled by whether any work began — the checkpoint the
   fold emitted, not the point the classifier stopped at — and, behind
   that, by whether the seat delivered: a classified refusal never
   discards a session that exited clean with its result file written.
   The classifier reads a harness's machine fields; the result contract
   is the seat's own, and where they disagree the delivered work wins.

   **Enforcement binding:** `crates/brokkr-protocol/src/adapters/tests.rs`
   drives a shim that errs and then works, and pins that the attempt
   accepts, keeps its thread id and totals, and reports its own result.
   A second shim errs and then DELIVERS, with no turn row at all: a
   record the classifier read as a refusal never discards a session that
   exited clean with its result file written. No measured CLI both
   refuses and delivers — `began_work` catches every shape #219 saw —
   but `rate_limit_event` is the one shape in ruling 3's table that is
   asserted rather than measured, and a newer CLI could emit it as an
   advisory. Delivery outranks it.

8. **A held row is lost to a kill before the first turn, and that is
   ruling 1's price, named.** A driver has its harness's
   session locator in hand from the harness's first message and holds it
   until a work checkpoint flushes it. The two facts cannot both hold at
   the driver: the row must be SENT early to survive a SIGKILL, and it
   must NOT EXIST for the refusal to be structurally a failure to start.
   Ruling 1 takes the second, so an attempt the engine's watchdog kills
   between the harness's announcement and its first turn journals no
   transcript locator; `seat_session` finds none, and the operator's
   retry opens a cold session and re-pays that turn's input. The window
   is the harness's launch to its first turn — for codex under a boxed
   step, the MCP servers' startup, which runs to minutes. Outside it
   nothing moves: a kill after the first turn hands its thread over
   exactly as it did before `accepted` was withheld, and a refused
   attempt carries its locator in its reason under ruling 6.

   **Enforcement binding:**
   `crates/brokkr-protocol/src/adapters/tests.rs` drives a codex shim
   through the window and pins that nothing reaches the engine before
   `accepted`, that the held rows are then flushed in their own order,
   and that the locator is among them; `resume_tests.rs` pins that an
   attempt which journaled no session is handed nothing. The doc
   comments on `seat_session` and on the codex `thread.started` arm
   state the window rather than the guarantee it replaced.

## Consequences

- An exhausted per-model limit no longer parks a seat whose chain has
  another judge that a probe would have served. On the #219 run this
  turns two of six parks into a fallback that does the work.
- The driver protocol's `accepted` now means "a turn began", not "the
  process spawned". The engine's predicate is unchanged, and no
  checkpoint is ever sent before `accepted`.
- The shared transcript row reaches the JOURNAL one beat later than it
  did: the driver still records the locator at the harness's first
  message, but `run_seat` holds that checkpoint until work begins,
  because a checkpoint before then would put a refused attempt on the
  mid-session side of the boundary. During a long first turn a live
  drilldown therefore cannot yet locate the seat's prose; the row is
  flushed, in order and unchanged, the moment the first turn checkpoints.
  An attempt KILLED inside that window journals no locator at all, so
  its retry starts cold — ruling 8 names that window and why it is the
  price of ruling 1.
- A refused attempt has no checkpoint at all — not only no transcript
  row, but no `<driver>-session-finished` row either, and that row is
  the one carrying exit code, served model, applied effort, usage and
  the lanetally ledger marker. Every readout keyed on it shows a refused
  attempt through its failure event alone, exactly as it already does
  for a driver that never spawned; any usage the harness reported beside
  its refusal is dropped, which for a per-model limit or an auth
  rejection is zero. The locator is in the failure reason (ruling 6),
  which is where a readout of a fail-to-start attempt already looks.
- A seat whose provider refuses before its first turn on every link
  still parks — the chain is bounded and a gate whose judges are all
  unavailable parks rather than descends (decision 0041 ruling 3). This
  ruling changes which failures count as "failed to start", not what
  happens when they are all spent.
- **Deliberately unruled.** Whether a future dsh release exposes a
  machine-readable pre-session refusal; if it does, this decision's
  table gains a row and dsh gains a classifier. The exact prose of a
  provider's message stays out of every control decision, before the
  first turn and after it.
