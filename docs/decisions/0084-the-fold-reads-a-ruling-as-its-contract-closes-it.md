# 0084 — The fold reads a ruling as its contract closes it

Status: proposed
Date: 2026-10-13

Built: built

## Context

Story #345, slice two, types the `transition/decided` payload the fold keeps
as `brokkr_core::decision::Decision` (decision 0071 ruling 3: typed at the
edge, with `deny_unknown_fields` where the contract is closed). The fold used
to keep the payload as a `serde_json::Value` and read it field by field:

- `from` and `result` had to be strings, or the fold failed with
  `FoldError::BadPayload`. That is unchanged.
- `next`, `problem` and `rule_id` were read with `as_str`, so a value that was
  not a string read as absent: a `next` of `5` parked the run, and a park
  whose `rule_id` key was misspelled read as "no ruling for (…)".
- `inputs` was read only when it was an object, and kept as recorded in
  `phase_results` whatever it held, so `inputs: []` folded.
- `severity` was kept as whatever string the payload held.
- A key the contract does not list was kept in `last_decision` and read by
  no one.

The typed read refuses each of those payloads with
`FoldError::BadDecision { seq, detail }`, the serde detail naming the field.
That narrows what the fold accepts, which is a semantic change, so it is ruled
here rather than left in a refactor's commit message.

What the narrowing touches today:

- `contracts/README.md` lists the seven `transition/decided` fields and
  forbids any other in v1 (`additionalProperties: false`).
- The extension rule in `contracts/README.md` (decision 0016) admits optional
  fields at `event_schema: 1` only once they are published as a numbered
  extension schema, and only fields the fold never reads. The five extension
  schemas in `contracts/` (`effect-boundary`, `effect-cleanup`,
  `effect-provenance`, `phase-entered-case`, `phase-entered-head`) cover
  `effect/*` and `phase/entered` payloads. None covers `transition/decided`.
- The engine writes this event at two sites in `decide`
  (`crates/brokkr-runtime/src/engine.rs`), the schema-invalid ruling and the
  policy's ruling, each with exactly the seven contract keys, and the
  policy loader bounds a rule's severity to `RULING_SEVERITIES`
  (`crates/brokkr-core/src/policy.rs`).
- Every committed decision, 68 of them across `fixtures/journals` and
  `docs/evidence`, reads into `Decision` and serializes back to its own
  bytes (`every_committed_decision_folds_to_its_own_bytes`).

So no journal Brokkr wrote is refused. The payloads refused are hand-built or
damaged ones, which the old fold read as a different ruling from the one
written.

Two alternatives were weighed:

- **Keep the lenient read**, with every field a `Value` and unknown keys
  retained. Rejected: it takes back ask 3 of #345, since readers would again
  dig with `.and_then(Value::as_str)`, and it keeps reading a named rule's
  park with a misspelled `rule_id` key as "no ruling" (decision 0022: a stop
  is never mislabeled).
- **Ignore unknown keys** and refuse only mistyped listed fields. Rejected:
  the ignored key would vanish from `last_decision`, so `inspect --json` would
  print bytes for such a journal that differ from before, and it would vanish
  silently. Refusing it says why.

## Rulings

1. The fold reads a `transition/decided` payload once, into `Decision`, and
   refuses it with `FoldError::BadDecision` when it carries a field the v1
   contract does not list, when `next`, `problem` or `rule_id` is neither a
   string nor `null`, when `inputs` is neither an object nor `null`, or when
   `severity` is not one of `RULING_SEVERITIES` or `null`. A missing `from` or
   `result` still fails as `FoldError::BadPayload`, naming the field.
   *Enforced by* `deny_unknown_fields` and the field types on `Decision`
   (`crates/brokkr-core/src/decision.rs`), and pinned by
   `a_decision_outside_its_contract_is_refused` and
   `a_decision_outside_its_contract_fails_the_fold`.
2. A `Decision` serializes to the bytes it was read from. `null` and absent
   stay apart for every optional field. *Enforced by* `Recorded<T>`, and
   pinned by `every_decision_shape_round_trips_null_and_absent_apart` and
   `every_committed_decision_folds_to_its_own_bytes`.
3. A field published for `transition/decided` as a numbered extension schema
   lands in the same change as an optional `Decision` field that the fold
   never reads. Until then the fold refuses it under ruling 1. A binary older
   than that change refuses a journal that carries the field. That is the
   price of ruling 1, accepted because the event has no extension today.
   *Judgment guidance*: no gate ties a new extension schema to `Decision`'s
   fields.

## Consequences

- A hand-built or damaged journal with an off-contract ruling stops folding
  and names the event's sequence and the field. `brokkr inspect` and export
  refuse it where they used to print a guessed state.
- `RunState.last_decision` holds the payload's seven fields and nothing else.
- If the operator declines this decision, the fallback is the first
  alternative above. It is confined to `decision.rs` and the fold's `rule`
  function, but it gives up `Decision`'s typed fields.
