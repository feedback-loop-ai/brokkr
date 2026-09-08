# Versioned contracts

**Status**: frozen for the first implementation (delivery-sequence step 1,
under decisions 0003–0005). A frozen contract changes only by a new numbered
version next to the old one, never by editing v1 in place. The Rust
implementation, the Python oracle, and every driver implement these files;
nothing implements a private variant.

| Contract | File | Consumers |
|---|---|---|
| Event envelope | `event-envelope.v1.schema.json` | brokkr-core, brokkr-store, export/verify |
| Event vocabulary + fold semantics | this file, below | brokkr-core |
| Driver protocol | `driver-protocol.v1.schema.json` | brokkr-protocol, every driver |
| Run manifest | `run-manifest.v1.schema.json` | brokkr-store, resume, verify-run |
| Evaluator behavior | `../fixtures/evaluator/corpus.ndjson` | brokkr-core differential tests |

The v1 files above remain frozen. Looper-bound delivery adds, without changing
those bytes:

| Contract | File | Consumers |
|---|---|---|
| Attempt-bound dispatch | `dispatch-envelope.v2.schema.json` | Looper, brokkr-core, Brokkr bridge |
| Looper-bound run manifest | `run-manifest.v2.schema.json` | brokkr-runtime, brokkr-store export/resume, Brokkr bridge |
| Seat record | `seat-record.v1.schema.json` | every driver, brokkr-store append/export/verify, every seat readout (superseded for new runs by `seat-record.v4.schema.json`, below) |

The v2 manifest embeds the complete canonical dispatch envelope. The existing
`runs.manifest` immutability trigger therefore makes Looper correlation,
request grant, recipe/repository pins, budget reference, runtime, producer
registration, bounds, forbidden actions, and callback audience part of the
run's immutable local evidence. A legacy run continues to store and export the
exact v1 shape.

The agent library (decision 0016) adds, again without changing frozen bytes:

| Contract | File | Consumers |
|---|---|---|
| Run manifest with pinned agent resolution | `run-manifest.v3.schema.json` | brokkr-runtime, brokkr-store export/resume |
| Effect provenance payload extension | `effect-provenance.v1.schema.json` | brokkr-runtime, brokkr-view, every readout |

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
| The world's map | `realms.v1.schema.json` | brokkr-core (shape), brokkr-runtime (loading), every read surface |
| Run manifest with the world pinned | `run-manifest.v4.schema.json` | brokkr-runtime, brokkr-store export/resume |

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

Many hearths (decision 0026) add one more file and change none of the bytes
above — v1's included, which is now pinned by digest alongside the original
frozen five:

| Contract | File | Consumers |
|---|---|---|
| The world's map, with per-realm journals | `realms.v2.schema.json` | brokkr-core (shape), brokkr-runtime (hearths), every fleet read surface |

`forge.realms/v2` is `v1` plus exactly one optional property per realm:
`journal`. A realm that names none falls back to the world's journal, which is
what every v1 realm already resolves to — so a v1 map loads, and every surface
draws it, exactly as before. The vocabulary stays closed at both levels, and
the one new word is refused in a map still calling itself v1: a version is a
promise about what a file may say. Journals never merge (ruling 5): a
many-hearth world is several append-only truths read side by side, so `runs`
groups by realm, `tui` tabs by realm, and `muninn` cites the realm each fact
came from — and no fold ever crosses a journal boundary.

Realm prompt data adds a third map version while leaving v1 and v2 unchanged:

| Contract | File | Consumers |
|---|---|---|
| The world's map, with house and dialect declarations | `realms.v3.schema.json` | brokkr-core (shape), brokkr-runtime (loading and pins), prompt assembly |
| Specification dialect | `dialect.v1.schema.json` | superseded for new files by v2, below; bytes frozen |
| Specification dialect, with the install identity | `dialect.v2.schema.json` | brokkr-runtime (loading, map checks and boxed dialect steps) |

`forge.realms/v3` adds optional `house` and `dialect` fields per realm. A
house is a repository-relative Markdown file; its content and digest are
pinned inside the run's realms pin and rendered into every seat prompt. A
dialect is a library name or repository-relative path. Its closed v1 shape
maps artifact and judge phases to framework artifacts, instructions,
validators, dependency order and lifecycle commands. The loader checks the
map and pins the resolved JSON content, not merely its declaration. A v2 map
remains byte- and behavior-compatible.

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
`effect/failed.start_failure` with `start_failure_sites`; and in
`phase-entered-head.v1.schema.json` (decision 0039): `phase/entered.head`,
the repository head the protected phase was entered at, which the engine
reads back to tell the review's own commits from the ones it judged.
`fold` reads neither file's fields.
Decision 0041 ruling 7 adds `phase-entered-case.v1.schema.json` on the same
additive rule: `phase/entered.case` names the selected strategy case and is
absent when the seat does not select. The fold does not read it; resume
recomputes selection from the journal's `strategy` fact.
Decision 0046 ruling 3 adds `effect-boundary.v1.schema.json` on the same
rule: `effect/started.boundary` is a list with one entry per invocation site
of the attempt — `member`, the site tag checkpoints and provenance already
use; `boundary`, one of the five words or `not applicable` for a site
without hands; `gate`, whether the site is gate class — present only when at
least one site of the attempt declares hands. `fold` never reads it; a run
over a bundle that boxes nothing journals byte-identical payloads. It is a
narrowed reading of ruling 3's "every `effect/started`", named as such: a
field on every start would be a v2 event, and the ruling's "every" is
carried by the seat record instead.

Reforging (decision 0022) adds one more file and changes none of the bytes
above:

| Contract | File | Consumers |
|---|---|---|
| Phase-machine table with the rule-driven park | `phase-machine.v2.schema.json` | brokkr-core, every bundle that parks by rule |

Decision 0043 adds one more manifest version and changes none of the bytes
above:

| Contract | File | Consumers |
|---|---|---|
| Run manifest with the boxed-hands sites | `run-manifest.v6.schema.json` | brokkr-runtime, every bundle whose seats declare `hands` |

Decision 0041 adds two further manifest versions, again beside the frozen
ones:

| Contract | File | Consumers |
|---|---|---|
| Run manifest with strategy-selected cases | `run-manifest.v7.schema.json` | brokkr-runtime, every bundle whose seats declare `select` |
| Run manifest with per-step result vocabularies | `run-manifest.v8.schema.json` | brokkr-runtime, prompt assembly for sequence steps |

The house and dialect pins fit additively inside v7's deliberately open
`realms` object. The manifest version advances because v7 closes each sequence
step and cannot carry a non-final step's new `results` array. Under v8, a
non-final step receives that vocabulary; a final step receives the enclosing
seat's vocabulary.

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
| Run manifest with the authorising adapters pinned | `run-manifest.v5.schema.json` | brokkr-runtime, brokkr-store export/resume |

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

Decision 0035 adds one more file and changes none of the bytes above:

| Contract | File | Consumers |
|---|---|---|
| Seat record with the hire's effort and the reasoning it spent | `seat-record.v2.schema.json` | every driver, brokkr-store export/verify, every seat readout |

`seat-record.v2` is `v1`'s vocabulary plus two optional properties on the
per-turn checkpoint, the finishing checkpoint and the successful result:
`effort`, the configured effort as the harness echoes it back, and
`reasoning_output_tokens`, the reasoning subset of `output_tokens`. `v1` is
not edited — its bytes are pinned by the embedded-copy test in
`crates/brokkr-store/src/seat_record.rs` and did not move — and it remains
the contract for every record already written under it.

The two versions are not interchangeable and nothing guesses between them:
**a record is validated against the version its run's engine wrote**, read
from the `engine` string in the `run/started` manifest that every lineage
carries. A run from an engine older than the one v2 landed in is validated
against v1, so a journal written before this decision stays readable exactly
as decision 0034 ruling 5 requires, and a v2-only field on such a record is
refused rather than quietly admitted. `effort` is CONFIGURATION and says so
in the schema: it is the harness's own echo of the value applied after every
profile and plugin layer, never the bundle's pin read back, and never a
measurement of what the model did. `reasoning_output_tokens` is a reported
subset, on exactly the terms `cache_read_tokens` already had — a view shows
it and never adds it to a total a second time — and it is absent, never
zero, where a harness reports no figure for that record.

Decision 0034's second addendum (ruled 2026-09-05) adds one more file and
changes none of the bytes above:

| Contract | File | Consumers |
|---|---|---|
| Seat record with the dialect step's state | `seat-record.v3.schema.json` | every driver, brokkr-store append/export/verify, every seat readout |

`seat-record.v3` is `v2`'s vocabulary plus one optional property on the
successful result: `state`, the output of the dialect's own state command,
which the exec driver has captured beside `notes` since decision 0042's
first slice. It is admitted to the **typed report** — the family of
`result`, `inputs` and `notes` — and not to the accounting vocabulary: this
contract admits the name and governs none of the content, exactly as it
already does for `notes`, which carries the same command's stdout and
stderr. The accounting fields are unchanged and remain what decision 0034
froze: an accounting record, never a transcript.

`v2` is not edited; its bytes are pinned by the embedded-copy test and did
not move. The engine boundary for v3 is the same 0.8.0 line v2 landed in,
because `engine` is the crate version and carries no position within a
line: both landed after the 0.8.0 tag and cannot be told apart by it. Within
a line the newest contract wins, which refuses nothing a v2 record could
have carried — v3 adds an optional property and takes none away. Naming the
unreleased line instead would judge every record this engine writes under
v2 and refuse the `state` field it is already writing, which is the defect
this addendum fixes.

Decision 0046 (the boundary is named) adds four files and changes none of
the bytes above:

| Contract | File | Consumers |
|---|---|---|
| The world's map, with the boundary each realm's boxed hands stand behind | `realms.v4.schema.json` | brokkr-core (shape), brokkr-runtime (loading and the per-site resolution), `brokkr doctor` |
| Run manifest with the boundary pinned per hands site | `run-manifest.v9.schema.json` | brokkr-runtime, brokkr-store export/resume, every bundle whose seats declare `hands` |
| Seat record with the boundary that stood | `seat-record.v4.schema.json` | the engine's stamp, brokkr-store append/export/verify, every seat readout |
| Effect boundary payload extension | `effect-boundary.v1.schema.json` | brokkr-runtime, brokkr-view, every readout that renders *unboxed* |

`forge.realms/v4` is `v3` plus exactly one optional property per realm:
`boundary`, one of `namespace`, `seatbelt`, `container`, `harness` or `open`
— the mechanism that stands between a boxed seat's hands and that realm's
machine, named for what varies and never for the operating system (ruling
1). Absent, it reads `namespace`, which is what every bundle meant before
the word existed, so a v3 map loads and resolves exactly as before. The
enumeration is frozen the way a contract is: a new boundary is a new
decision. The boundary is the realm's fact because the machine a realm runs
on is the realm's fact; a bundle never names it, and the site parser refuses
a `boundary` key in any site or `hands` object naming the realm as its home.

`run-manifest.v9` is `v8` plus one optional `boundary` property: a map from
hands site label to the realm's one resolved word, present exactly when the
manifest has a `hands` key and absent with it — the schema's `dependencies`
bind the two both ways — so a bundle that boxes nothing keeps its exact v8
identity, and a run under `seatbelt` and a run under `namespace` are two
identities, as decision 0043 ruling 4 requires. Like `drivers`, this key IS
bundle data: it moves the digest, which is the point of the pin.

`seat-record.v4` is `v3`'s vocabulary plus one optional property on the
checkpoint and the successful result: `boundary`, the five words or decision
0031's sentinel `not applicable` for a site without hands. The engine stamps
it, never the driver — it is the only party that knows which boundary it
built, and a driver's value never survives the stamp: a record that names a
`model` carries `boundary` beside it, a record that names none carries no
`boundary`. `v3` is not edited; its bytes are pinned by the embedded-copy test
and did not move. The store dispatches the 0.9 engine line and later to v4
and the 0.8 line to v3, on the same rule v2 and v3 stated: a record is
validated against the version its run's engine wrote, and journals from the
tagged 0.9.0 and 0.9.1 engines validate under v4 because it is additive. The
data carries the plain word; every readout renders the adjective *unboxed*
for a run whose gate stood under `harness` or `open`, and a record written
before the word existed renders an explicit absence, never a default.

`effect-boundary.v1` is the extension-field schema described under the event
vocabulary above. Rulings 3 and 6 of the decision name the seat-record file
`v3`; the decision's erratum records that v3 already existed and the field
therefore lands as v4, nothing else renumbered.

Decision 0047 adds one more file and changes none of the bytes above:

| Contract | File | Consumers |
|---|---|---|
| Operator supersede args | `operator-supersede.v1.schema.json` | brokkr-runtime (the verb), brokkr-view, the fleet dossier |

`operator-supersede.v1` is a payload schema over `operator/commanded.args`
for `command: "supersede"` and nothing else. The v1 event envelope, the
`type` enum and every witness digest are unmoved, and deliberately: `command`
is already an open string in the vocabulary above and `args` an already-legal
open object the reducer never reads, so a new command word with structured
`args` is a v1 event byte for byte. The annotation is written only on a run
that has gone `completed` or `stopped`, where `fold` admits an operator
annotation that changes nothing, and no `operator/accepted` follows it —
there is nothing to execute, and an acceptance after a terminal is exactly
what `fold` refuses.

The amended rule's last clause needs no argument here: `fold` does not read
the field, `RunState` gains nothing, and a terminal run folded with the
annotation present is byte-identical to the same run folded without it. What
reads it is `brokkr_view::residual_findings`, which marks the named findings
as superseded and leaves them in the journal and in every readout — a
superseded finding is closed, never deleted.
