# Versioned contracts

**Status**: frozen for the first implementation (delivery-sequence step 1,
under decisions 0003–0005). A frozen contract changes only by a new numbered
version next to the old one, never by editing v1 in place. The Rust
implementation, the Python oracle, and every driver implement these files;
nothing implements a private variant.

| Contract | File | Consumers |
|---|---|---|
| Event envelope | `event-envelope.v1.schema.json` | forge-core, forge-store, export/verify |
| Event vocabulary + fold semantics | this file, below | forge-core |
| Driver protocol | `driver-protocol.v1.schema.json` | forge-protocol, every driver |
| Run manifest | `run-manifest.v1.schema.json` | forge-store, resume, verify-run |
| Evaluator behavior | `../fixtures/evaluator/corpus.ndjson` | forge-core differential tests |

The v1 files above remain frozen. Looper-bound delivery adds, without changing
those bytes:

| Contract | File | Consumers |
|---|---|---|
| Attempt-bound dispatch | `dispatch-envelope.v2.schema.json` | Looper, forge-core, Forge bridge |
| Looper-bound run manifest | `run-manifest.v2.schema.json` | forge-runtime, forge-store export/resume, Forge bridge |

The v2 manifest embeds the complete canonical dispatch envelope. The existing
`runs.manifest` immutability trigger therefore makes Looper correlation,
request grant, recipe/repository pins, budget reference, runtime, producer
registration, bounds, forbidden actions, and callback audience part of the
run's immutable local evidence. A legacy run continues to store and export the
exact v1 shape.

The agent library (decision 0016) adds, again without changing frozen bytes:

| Contract | File | Consumers |
|---|---|---|
| Run manifest with pinned agent resolution | `run-manifest.v3.schema.json` | forge-runtime, forge-store export/resume |
| Effect provenance payload extension | `effect-provenance.v1.schema.json` | forge-runtime, forge-view, every readout |

**Two lineages, not one line.** `run-manifest.v1` → `v3` is the local lineage:
v3 is v1's bytes plus one optional `agents` property, absent when no seat
references an agent, so every non-adopting run stores and exports the exact v1
shape. `run-manifest.v2` is the Looper-bound lineage and is unchanged;
`bundle_manifest_from_run` reconstructs a bundle manifest from six named keys
and drops the rest, so an `agents` key would be silently dropped on the v2
round-trip and every adopting Looper-dispatched run would become unresumable
with a diff that blames no file. Rather than widen a contract a counterpart
system reads, `build_run_manifest_v2` **refuses** a bundle manifest carrying
`agents`, naming the limitation. Lifting that needs a jointly agreed
v2-lineage manifest version.

Realms (decision 0023) add two more files and change none of the bytes above:

| Contract | File | Consumers |
|---|---|---|
| The world's map | `realms.v1.schema.json` | forge-core (shape), forge-runtime (loading), every read surface |
| Run manifest with the world pinned | `run-manifest.v4.schema.json` | forge-runtime, forge-store export/resume |

`forge.realms/v1` is the minimal shape decision 0023 ruled: the `realms` —
each a name, a path and a default branch — and the world's `journal`. The
loader refuses unknown fields at both levels, so decision 0021's per-realm
driver and egress constraints must arrive as `forge.realms/v2` rather than as
drift inside a file still calling itself v1.

`run-manifest.v4` is `v3` plus one optional `realms` property carrying the
map's `source`, its `sha256` and the map itself. It continues the LOCAL
lineage: absent when a run was invoked with no map, so an unmapped run stores
and exports the exact v1/v3 shape. Two properties make the pin honest. The
digest is over the embedded content's canonical JSON, so a reader holding only
the journal can re-derive it without the file. And the map is workspace data,
not bundle data: `bundle_manifest_from_run` drops `realms` before the resume
comparison, so pinning a world moves no bundle digest and makes no run
unresumable. The event vocabulary needed nothing — the manifest already rides
inside `run/started`, which is exactly why embedding it there answers "what
world did this run believe in?" from the journal alone.

The Looper-bound `run-manifest.v2` lineage carries no world, for the reason it
carries no `agents`: its round-trip reconstructs the bundle manifest from six
named keys, so the pin would be dropped in silence. `brokkr run` refuses
`--dispatch` together with a map rather than half-honouring it.

Per-realm facts ride in `transition/decided.inputs`, not in the envelope:
`reviewed_heads` is keyed by realm name in a mapped world (the shape the
heritage protocol recorded as "repository name to observed HEAD"), and
`realm_facts` records that realm's head, dirty worktree and drift. Both are
engine-owned; a seat may neither declare nor claim one. Reading accepts BOTH
shapes — a head recorded before any map, under the unkeyed `repo` key, still
answers — so every existing journal folds exactly as it did.

## Event envelope

One JSON object per event. Canonical bytes: JSON with keys sorted
lexicographically at every level, `,`/`:` separators, no insignificant
whitespace, UTF-8. `event_hash` = SHA-256 hex over the canonical bytes of the
envelope **with the `event_hash` member removed**. `previous_hash` is the
`event_hash` of `seq - 1`, or 64 zeros for `seq` 1. `seq` is contiguous from 1
per run. `recorded_at` (RFC 3339 UTC) is evidence and display data only;
reducer decisions never read it.

## Event vocabulary (v1)

```
run/started            payload: { feature, manifest }
phase/entered          payload: { phase }
effect/requested       payload: { effect_id, phase, seat, idempotency_key, input_digest }
effect/started         payload: { effect_id, attempt_id, driver }
effect/checkpointed    payload: { effect_id, attempt_id, checkpoint }
effect/succeeded       payload: { effect_id, attempt_id, result }
effect/failed          payload: { effect_id, attempt_id, error }
effect/indeterminate   payload: { effect_id, attempt_id, reason }
transition/decided     payload: { from, result, rule_id|null, next|null, severity|null,
                                  inputs, problem|null }
operator/commanded     payload: { command_id, command, args, operator }
operator/accepted      payload: { command_id, operator, reason }
operator/rejected      payload: { command_id, operator, reason }
run/parked             payload: { reason, evidence }
run/completed          payload: {}
run/stopped            payload: { reason }
```

An unknown event type fails fold closed (error, not skip). Payload fields not
listed here are forbidden in v1 (additionalProperties: false).

**Amended (decision 0016), with its reason.** The previous rule read *"a new
field is a v2 event"*, which is unenforceable as written: it forbids a field a
v1 consumer can safely ignore just as strongly as one it must read, and it
gives a reviewer no test to run. The enforceable rule is:

> Additive payload fields that are optional, absent by default, and published
> as a numbered extension schema are permitted at `event_schema: 1`. A field
> that changes the meaning of an existing field, or that a v1 consumer must
> read to fold to correct state, is a v2 event.

The narrowness is what makes the amendment honest, and the last clause is
machine-checked: `fold` never reads an extension field, `RunState` gains
nothing from one, and a run over a bundle that references no agent journals
byte-identical payloads. Bumping `EVENT_SCHEMA` instead was rejected because
`manifest_for` embeds `"event_schema": EVENT_SCHEMA`, so the bump would move
**every** manifest digest — including the byte-identity witnesses — and violate
`{"const": 1}` in both frozen manifest schemas. The same argument rules out a
new event *type*: the `type` enum is closed under `additionalProperties:
false`.

The extension fields defined so far are in
`effect-provenance.v1.schema.json`: `effect/started.provenance`, and
`effect/failed.start_failure` with `start_failure_sites`.

Reforging (decision 0022) adds one more file and changes none of the bytes
above:

| Contract | File | Consumers |
|---|---|---|
| Phase-machine table with the rule-driven park | `phase-machine.v2.schema.json` | forge-core, every bundle that parks by rule |

`forge.phase-machine/v2` is `v1` plus exactly one thing: a rule may rule a
PARK instead of naming a `next` phase. The event vocabulary needs nothing —
a `transition/decided` with the matched `rule_id`, a null `next`, a null
`severity` and the rule's reason as its `problem` is already the shape
`requires_artifacts` writes when its gate blocks an otherwise-matching rule,
and `fold` already parks there. What the version buys is that a park cannot
arrive unannounced: the loader refuses a parking rule in a table that calls
itself `v1`, because a park is not a stop and the difference is the whole
point of the ruling. The park reason the fold builds now names the rule when
one is named (`<rule_id> for (<from>, <result>): <problem>`) and keeps
`no ruling for (…)` for the case that really has no rule.

## Fold semantics (state is derived, never mutated)

`fold(events) -> RunState` with:

- `status`: `running` from `run/started`; `awaiting_operator` on `run/parked`;
  back to `running` on `operator/accepted`; `completed` on `run/completed`;
  `stopped` on `run/stopped`. Terminal statuses accept no further events
  except operator commands.
- `phase`: set by `phase/entered`. A `transition/decided` never moves the
  phase by itself; the engine appends the matching `phase/entered` next.
  (Two facts, one decision: the ruling and its enactment are separate,
  crash-recoverable events.)
- `consecutive_failures[phase]`: incremented when `transition/decided` from
  that phase carries a result in the phase's failure class (`failed`,
  `broken`); reset to 0 on any other decided result from that phase. The
  engine supplies `consecutive_failures = counter + 1` (counting the current
  failure) as the evaluation input, matching the referee
  (`forge-control.py` default 1).
- `visits[phase]`: incremented by every `phase/entered` for that phase — the
  count the graph already renders as `×N`, and the fact the
  `visits_<phase>_gte` predicate reads (decision 0022). Engine-owned like
  every other journal-computed input: the engine supplies it for exactly the
  phases the deciding phase's rules ask about, and a seat may neither declare
  nor claim one.
- `last_result`: the raw result object of the most recent `effect/succeeded`.
  A seat the run RETURNS to (any phase entered more than once) receives it as
  `context.returned_from`, so a review's findings, severities and notes reach
  the implementer who has to answer them. A seat on its first visit receives
  nothing new, so a run that never revisits builds the seat input, and the
  `input_digest`, it always built.
- `reviewed_heads`: replaced by the payload of a review-phase
  `transition/decided` whose inputs carry `reviewed_heads`; consumed by the
  ship gate's drift check.
- `open_effect`: exactly one effect may be outstanding
  (`effect/requested`..`effect/started` without a terminal attempt fact).
  A second `effect/requested` while one is open fails fold closed.
- Attempts within an effect (decision 0006): an `effect/failed` returns the
  open effect to the executable position with its failure counted; the
  engine may start a fresh attempt (`effect/started` again, new
  `attempt_id`) up to the seat's declared limit, or append `run/parked`
  directly from that position when the limit is exhausted. Each attempt
  still carries exactly one terminal fact. An `effect/indeterminate` NEVER
  re-attempts automatically — completion could not be established, so a
  retry could silently duplicate or re-pay for completed work; it always
  parks.
- Replay of the same valid journal is byte-deterministic: same state, same
  pending action.

## Driver protocol — `forge-driver/v1`

NDJSON, one message per line: engine→driver on stdin, driver→engine on
stdout. stdout is protocol-only; stderr is captured as an artifact. Every
message carries `proto: "forge-driver/v1"`, `msg_id`, and `type`. Unknown
message types and schema-invalid messages fail closed (the attempt errors;
nothing is guessed). Message family:

```
engine → driver:  hello · start · resume · cancel · shutdown
driver → engine:  capabilities · accepted · checkpoint · result · cancelled
```

`result.status` is `succeeded | failed`; a driver that exits without a
`result` after `accepted` leaves the attempt `indeterminate` and the run
parks (never converted to success or silently retried as the same attempt).
`resume` is offered only when `capabilities.supports` includes `"resume"`
and the recorded `session_ref` matches.

## Run manifest

Pins the exact bundle a run executes: engine version, `event_schema`,
`database_schema`, `driver_protocol`, and a `sha256` digest per bundle file
(policy, seats, roles, result schemas, driver commands). Resume refuses a
digest mismatch with a diagnostic; it never "helpfully" picks up edited
files. Friendly names are display metadata; digests are identity.

## Evaluator behavior corpus

`tools/generate_evaluator_corpus.py` derives
`fixtures/evaluator/corpus.ndjson` from the production table and the 0004
oracle. For every `(phase, result)` rule group it enumerates the full domain
of every input the group's conditions reference (unreferenced inputs cannot
affect the outcome, so coverage over referenced-input assignments is
exhaustive over behavior classes), plus novel-result, unknown-phase, and
mistyped-input park cases. Expected `problem` strings are diagnostic
evidence, not contract: differential tests compare rule id, next phase,
severity, and park-vs-rule (including whether `problem` is set), never the
prose.

Decision 0021's witness (the reforging of run `implement-decision-0021` ruled
remedy ii) adds one more file and changes none of the bytes above:

| Contract | File | Consumers |
|---|---|---|
| Run manifest with the authorising adapters pinned | `run-manifest.v5.schema.json` | forge-runtime, forge-store export/resume |

`run-manifest.v5` is `v4` plus one optional `drivers` property: invocation
site → driver name → the sha256 of the adapter declaration that authorised an
inline gate or binding seat. Unlike `realms`, this key IS bundle data — a tier
demoted in `adapters/` moves the digest of every bundle whose gates it stood
behind, which is the point of the witness. A bundle that consulted no
declaration carries no key and keeps the exact v4 shape and identity. Agent
sites are not recorded twice: their `agents` resolution records already pin
every adapter the chain consulted.

The Looper-bound `run-manifest.v2` lineage cannot carry `drivers`, for the
reason it carries no `agents`: the v2 round-trip reconstructs the bundle
manifest from six named keys and would drop the pin in silence, leaving the
run unresumable with a diff that blames no file. `build_run_manifest_v2`
refuses `agents` by name and now refuses EVERY key beyond the six it can
round-trip — fail closed, so the next witness key added to the local lineage
is refused loudly on the day it lands rather than dropped quietly.
