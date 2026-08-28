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
listed here are forbidden in v1 (additionalProperties: false); a new field is
a v2 event.

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
