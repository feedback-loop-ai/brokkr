# Feature Specification: Enforce `requires_artifacts` at decide time

**Feature slug**: `enforce-requires-artifacts`
**Run**: `enforce-requires-artifacts-polic-4ab9e756`
**Status**: Committed (design phase ruling)
**Input positions**: `.forge/design/positions/simplicity.md`, `.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

A policy rule may declare `requires_artifacts` (the heritage table's
ARCH-OK names `spec.md`, `plan.md`, `repos.yaml` —
`policy/phase-machine.json:70`). The loader validates its shape
(`crates/forge-core/src/policy.rs:232`), the evaluator returns it on
every ruling (`policy.rs:169`), and the engine's decide step then drops
it on the floor: the `Outcome::Ruling` arm at
`crates/forge-runtime/src/engine.rs:1079` binds `rule_id`, `next_phase`,
`severity` and discards the rest with `..`. A declared artifact gate
that never gates is worse than no gate — it reads as enforced and is
not.

After this change an advancing ruling that carries `requires_artifacts`
advances only if every named artifact deterministically exists as a
non-empty regular file in the run's workdir at decide time. A miss
fails closed with the raw evidence, through the existing park
mechanism. No seat is asked, no seat attests, no model repairs —
mechanical checks stay mechanical (decision 0001).

## What

### Gate trigger

- The gate runs if and only if `Machine::evaluate` returned
  `Outcome::Ruling` with a non-empty `requires_artifacts` (the loader
  normalizes absent → empty `Vec`, so "non-empty" is the only check).
- It runs inside the engine's decide step: after `evaluate`, before the
  `TransitionDecided` event is appended. It never runs on
  `Outcome::NoRule` and never inspects the seat's result object — the
  gate acts on the ruling, and seat-supplied facts can never influence
  it (decision 0007; seats cannot attest artifacts into existence).
- The gate applies uniformly to every ruling carrying entries, with no
  destination special-cases in the engine. Declaring
  `requires_artifacts` on a rule whose transition is terminal is an
  authoring smell (a blocked stop parks a run that was ending); it is
  documented here as review guidance, not handled by machinery — no
  in-tree rule does it.

### Artifact predicate — one entry passes iff all of

1. **Lexically valid**: non-empty; relative (no leading `/`); no `.` or
   `..` path components; no backslash; no NUL; and none of the reserved
   characters `{`, `}`, `$`, `<`, `>`. Anything else fails closed as
   class `invalid` — the pinned table cannot be spoofed by layout
   (`../../etc/hosts` never resolves outside the workdir because it
   never resolves at all), and the reserved characters fence the
   substitution door shut (see the static-vs-dynamic ruling below).
2. **Present**: `std::fs::metadata(workdir.join(entry))` succeeds,
   where `workdir` is `Engine::workdir()` (`engine.rs:399`: `--repo` or
   cwd). Any metadata error — ENOENT, EACCES, ELOOP — is class
   `missing`; every error path fails closed, never advances. Symlinks
   are followed (`metadata`, not `symlink_metadata`): the gate asserts
   presence of content, not provenance; a dangling symlink correctly
   reads as missing.
3. **A regular file**: `metadata.is_file()`. A directory is class
   `not-a-file`.
4. **Non-empty**: `metadata.len() > 0`. A zero-byte file is class
   `empty`.

Content is never read. Content validation is check-seat territory
(`recipes/sdd/drivers/speckit_check.sh` is the precedent); presence is
the engine's.

### Failure evidence — all failures, table order, canonical format

The gate evaluates **every** entry and reports the complete failure
list in declaration order — not first-fail. The operator fixes one
park, not N sequential parks. The `problem` string is machine-stable
contract, journal-borne forever, produced by one pure function:

    requires_artifacts unmet for rule <rule_id>: <class>: <entry>; <class>: <entry>

with classes exactly `missing` | `empty` | `not-a-file` | `invalid`,
entries verbatim, `; `-separated, table order. Operators will grep for
this string in `events.jsonl` and the fold embeds it in the park
reason; it is specified character-exact and pinned by machine proof.

### The blocked decision — frozen field set, deliberate values

The `transition-decided` payload keeps exactly its existing seven
fields. On the miss path:

| Field | Value | Ruling |
|---|---|---|
| `from` | the phase, as today | unchanged |
| `result` | the seat's result, as today | unchanged |
| `rule_id` | **retained** (the rule that fired) | a gate block must be journal-distinguishable from NoRule; the rule matched and its identity is the evidence an auditor needs without regex-parsing prose |
| `next` | `null` | fails closed; parks via the existing fold arm (`fold.rs:296-330`) with zero fold changes |
| `severity` | **`null`** | severity is a property of a transition being taken; none is taken. Matches the schema-violation park's posture (`engine.rs:1030`) and keeps severity-scanning consumers (e.g. the ship gate's `severity != hard` reading) from misfiring on a non-transition |
| `inputs` | as computed, unchanged | evidence goes in `problem`, never injected into the provenance-controlled `inputs` vocabulary (decision 0007) |
| `problem` | the canonical string above | the raw evidence |

On the pass path the payload is byte-identical to today's advancing
payload — the gate leaves no residue on the happy path.

### Journal and replay semantics

- The probe runs exactly once, immediately before the append; the
  decided payload is the durable record of what was observed. Replay
  folds the recorded event and never touches the workdir (determinism
  law 2) — a machine proof deletes the workdir and replays to the same
  state.
- The check→append window is not atomic and does not need to be: the
  journal records the gate's observation at decide time, and the
  append chain already serializes writers.
- Runs recorded before this change replay as recorded; the gate exists
  only at decide time and replay never re-decides. No retro-enforcement.
- **`consecutive_failures` semantics, pinned**: the blocked decision
  carries the seat's actual result (e.g. `complete`), which is not in
  `FAILURE_RESULTS`, so the fold resets the phase's failure counter
  even though the run parked. This is intended — the counter tracks
  seat failures; the seat succeeded and the gate blocked. Stated here
  and pinned by proof so nobody later "fixes" it into a behavior
  change on replayed journals.
- **Recovery** is the existing operator loop, no new machinery: park →
  `retry` (`fold.rs`: park → `RequestEffect`) → the seat re-runs →
  the gate re-probes → advances if the artifacts now pass. A gate park
  is not a seat failure and consumes no attempt budget (decision 0006).

## Open question ruled: static declarations vs dynamic paths

**Ruling: option 1 — strictly static, with a reserved-syntax fence.**
Entries are literal workdir-relative paths, verified verbatim. No
substitution, no tokens, no resolution machinery.

Justification against the laws:

- **Determinism law 1 and the pinned-bundle law.** Same journal + same
  pinned bundle ⇒ same decision. A verbatim string means the
  content digest pins the table's *meaning* with no further argument.
  Substitution makes the table's meaning a function of a run fact; the
  digest then pins bytes, not meaning. Option 2 must additionally prove
  its substitution source is engine-owned and journaled (decision
  0007) — nothing journals a feature slug today, so option 2 is
  unconstitutional as things stand — and would import its own failure
  family (unresolvable token, ill-formed expansion) that must itself be
  validated, journaled, and parity-mirrored.
- **Zero users for the machinery.** No recipe in-tree uses
  `requires_artifacts`; the only declaring rule is heritage ARCH-OK
  with three fixed names. Building a substitution grammar for zero
  users is speculative generality — the exact thing a pinned-bundle
  system should refuse.
- **Dynamic gating already has a stronger in-tree precedent.** The sdd
  recipe's `speckit-check` step finds `specs/<slug>/` itself and does
  structural validation, not mere existence. A substitution token would
  hand sdd a strictly weaker gate than the one it ships. The division
  of labor is real and is hereby documented: **static, fixed-layout
  presence gates belong to `requires_artifacts` in the table; dynamic,
  per-feature, content-aware gates belong to deterministic check seats
  (speckit-check is the working model).** Consequence for the sdd
  recipe: none — it keeps speckit-check and does not adopt
  `requires_artifacts` for per-feature paths.
- **The fence makes static forward-compatible instead of naive.** If
  entries were "just strings" and a later feature added substitution,
  any historical bundle whose entries contain `{`/`$`/`<` would
  silently change meaning under the same digest. The lexical predicate
  rejects the reserved characters `{ } $ < >` today — reserved, not
  assigned — so no bundle can ever have depended on their literal
  meaning. If option 2 is ever wanted, it arrives as an explicit
  bundle-schema revision with its own journaled, engine-owned
  substitution source: a compatible extension through a fenced gate,
  not a reinterpretation of frozen strings. This is the designated
  extension point.

## Constraints honored (unchanged surfaces)

- `Machine::evaluate` — signature and behavior byte-identical;
  forge-core's diff for this feature is empty (see plan.md for the
  position reconciliation behind this).
- `transition-decided` field set, event types, condition vocabulary
  (`BOOLEAN_INPUTS` — this is not a `when {artifacts_present}` input),
  result types, fold (`fold.rs` byte-identical; the park reason's
  `no ruling for (...)` prefix is mildly misleading for a gate block,
  accepted — the evidence lives in `problem`).
- `policy/phase-machine.json`, `reference/`,
  `fixtures/evaluator/corpus.ndjson` (zero `requires_artifacts`
  expectations; `differential.rs` compares rulings with `..` — both
  stay that way), `contracts/`.
- The sdd recipe and `speckit_check.sh` — untouched; this spec's
  division-of-labor paragraph is the entire deliverable there.

## Acceptance Criteria

Each criterion is testable in `crates/forge-cli/tests/machine_proof.rs`
(real binary; the `Workspace` scaffolding authors its own bundle
tables, so no production table changes).

1. **Missing-artifact fail-closed**: a rule with
   `requires_artifacts: ["spec.md"]` and no such file → the run parks
   (`awaiting_operator`); the decided event has `next: null`,
   `severity: null`, `rule_id` retained, and `problem` equal to
   `requires_artifacts unmet for rule <id>: missing: spec.md`; the
   phase never advances and no subsequent seat runs.
2. **Zero-byte and directory variants**: a zero-byte file parks
   identically with class `empty`; a directory at the path parks with
   class `not-a-file`.
3. **Invalid entries fail closed**: entries `../escape` and `{slug}`
   park with class `invalid` (the reserved-character fence and the
   traversal predicate are both load-bearing at decide time).
4. **Multiple failures, one park**: a rule naming three failing
   artifacts produces a single park whose `problem` lists all three,
   table order, canonical format.
5. **Artifacts-present advance**: same rule, artifacts present and
   non-empty → the decided payload is byte-equivalent to an
   artifact-free rule's advance.
6. **Recovery loop closes**: park → operator `retry` → seat re-runs →
   artifacts now present → advance.
7. **Replay determinism**: replay of both the parked and the advanced
   run — with the workdir deleted — reproduces the state; export/replay
   stability per the existing
   `full_delivery_completes_exports_and_replays` pattern.
8. **`consecutive_failures` reset pinned**: the blocked decision (seat
   result `complete`) resets the phase's failure counter.
9. **Whole workspace green**: `cargo test` across the workspace passes,
   including the untouched differential corpus and `policy_lint.rs`.
