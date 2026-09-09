# boundary-record Specification

## Purpose
The record says which boundary stood: the engine's effect-start path,
the seat record as `seat-record.v4`, and the seat prompt (decision 0046
ruling 3; decisions 0031, 0034 and 0035 on how the record carries facts).

## Requirements

### Requirement: effect/started carries the boundary beside provenance
The engine SHALL add `boundary` to the `effect/started` payload beside
`provenance` whenever at least one invocation site of the attempt
declares hands: a list with one entry per invocation site of the
attempt, keyed by the same `member` tag `provenance` uses (`null` for a
single seat, `<member>` for a panel member, `<step>` for a sequence
step, `<step>:<member>` inside a step panel), each entry carrying
`boundary` — the realm's word for a site with hands, the sentinel `not
applicable` for a site without — and `gate`, a boolean saying whether
the site is gate class. An attempt with no boxed site SHALL carry no
`boundary` key, so a run over a bundle that boxes nothing journals
byte-identical payloads. The field SHALL be published as
`contracts/effect-boundary.v1.schema.json` under the contracts README's
amended rule: optional, absent by default, never read by `fold`, and
`RunState` gains nothing from it. It is absent by default because the
README's rule admits an extension field at `event_schema: 1` only on
that condition — a key on every `effect/started` would be a v2 event,
which the closed `type` enum forbids — and ruling 3's *every* is carried
by the seat record instead, which carries `boundary` on every finishing
checkpoint and successful result. This is a narrowed reading of ruling
3, named as such; the literal reading is a new event lineage for the
operator to ask for (design DD5). The entries SHALL be built from the
same invocation-site traversal that builds `provenance`, so the two tag
sets cannot drift (decision 0046 ruling 3; the extension rule of
decision 0016).

#### Scenario: A boxed single seat
- **WHEN** a gate-class single seat that declares hands starts an attempt under `harness`
- **THEN** its `effect/started` carries `boundary` with one entry: `member` null, `boundary` `harness`, `gate` true

#### Scenario: A sequence with one boxed step
- **WHEN** a sequence of a hands-less author step and a boxed dialect validate step starts under `namespace`
- **THEN** `boundary` lists the author step as `not applicable` and the validate step as `namespace` with `gate` true

#### Scenario: A plain bundle journals no key
- **WHEN** a bundle with no hands site runs to completion
- **THEN** no `effect/started` in its journal carries `boundary`, and the journal's payloads are byte-identical to those the engine wrote before this change

#### Scenario: fold never reads it
- **WHEN** a journal whose `effect/started` events carry `boundary` is folded
- **THEN** the folded state equals the fold of the same journal with the field removed

#### Scenario: The extension schema is published beside the frozen one
- **WHEN** the frozen-contracts test runs
- **THEN** `contracts/effect-boundary.v1.schema.json` exists with the title `Forge effect boundary v1`, `contracts/effect-provenance.v1.schema.json` is not edited, every entry the engine writes validates against the new schema, and the contracts README lists the file among the extension schemas `fold` never reads

### Requirement: The seat record carries the boundary as seat-record/v4
`contracts/seat-record.v4.schema.json` SHALL be v3 plus one optional
property `boundary` on the checkpoint and on the successful result —
the five words or the sentinel `not applicable` — with v1, v2 and v3
not edited and the store embedding v4 beside them under the
embedded-copy pin test. The schema admits the property on any
checkpoint because draft-07 cannot tell one record from another; the
engine SHALL stamp it by one rule — a record that names a `model`
carries `boundary` beside it, and a record that names none carries no
`boundary` — applied to every driver checkpoint and successful result
at the two pass-throughs through which they reach the store, and to
the engine's own `panel-member-finished` and `sequence-step-finished`
markers, which name a member's or a step's model. So the finishing
checkpoint, whose `model` the driver conformance suite asserts for
every built-in driver, and the successful result carry it, as does any
per-turn checkpoint on which a driver names a model; a per-turn
checkpoint that names none carries none (design DD19). The stamp is
the realm's word for a site with hands and `not applicable` for a site
without, written before the record is appended, and it SHALL replace
any `boundary` a driver wrote — dropping it from a record that names no
model — because the engine is the only party that knows which boundary
it built; drivers and their conformance field sets SHALL not change. A
panel's aggregate result, which the engine composes and which names no
model, SHALL carry no `boundary`; a sequence's ending result is its
ending step's driver result and SHALL carry that step's word (design
DD19). The
store's append fence SHALL validate the stamped record under the version
selected from the run manifest's `engine` as specified below, and refuse a
word outside that version's vocabulary at the seq it would have taken
(decision 0046 ruling 3 with the commission's erratum; decision 0034
rulings 1, 5 and 6; decision 0031 ruling 1 for the sentinel).

Seat-record/v5 SHALL extend v4 additively with the bounded session and launch
vocabulary required by `adapter-launch-evidence`, preserving every valid v4
record. Its published and embedded files SHALL be new versions beside v1–v4;
all existing published and embedded contract bytes SHALL remain unchanged.
The v5 additions SHALL NOT change the boundary-stamping rule above: a provider
root or launch claim never selects the realm's boundary.

Append, export, import verification and offline verification SHALL use the
same manifest-engine dispatch and validation rules. An engine at or after
`0.10.0` SHALL select v5; at or after `0.9.0` but before `0.10.0`, v4; at or
after `0.8.0` but before `0.9.0`, v3; an earlier or unparseable engine, v1.
The existing engine-version parsing convention remains unchanged. Version
selection SHALL NOT depend on the reader's version, a launch value, or the
presence of a new field. Frozen versions remain directly testable, including
v2 even though no manifest-engine string selects it.

Existing valid 0.10.0 rows without the new fields SHALL remain valid under v5,
without rewriting or backfilling history. In particular, an unstamped resumed
row without confirmed-root evidence and unstamped third-party refusal-bearing
rows retain their v4 meaning. Only records carrying the new engine-owned
`site_ref` stamp SHALL be subject to the new fence conditions that resumed
requires confirmed `root_session` evidence and `resume_refusal` requires
`launch: cold`. Every resumed launch this change produces SHALL carry confirmed
root evidence, and every refusal reason it produces SHALL accompany cold,
under `adapter-launch-evidence` LE1–LE3. The historical compatibility exception
SHALL NOT weaken those current-producer obligations. Boundary stamping and the
existing privacy/failure rules apply independently of these new conditions.

#### Scenario: A boxed exec gate's record carries the word
- **WHEN** a boxed exec gate under `namespace` succeeds
- **THEN** its finishing checkpoint and its successful result both carry `boundary` `namespace`, and `brokkr export` and `verify-run` accept the journal

#### Scenario: A site without hands carries the sentinel
- **WHEN** an inline exec seat with no hands succeeds, or a model seat on the tool-list path succeeds
- **THEN** its finishing checkpoint and successful result carry `boundary` `not applicable`

#### Scenario: The engine's word wins
- **WHEN** a driver writes `boundary` `open` onto the finishing checkpoint of a seat boxed under `namespace`
- **THEN** the appended checkpoint carries `namespace`; a per-turn checkpoint on which the driver wrote `boundary` and named no `model` is appended without it; and a per-turn checkpoint on which the driver names a `model` carries `namespace` beside it

#### Scenario: A panel's aggregate carries none, a sequence's ending result its step's word
- **WHEN** a panel of boxed members under `harness` succeeds, and a sequence whose ending step is a boxed dialect check step under `namespace` succeeds
- **THEN** the panel's `effect/succeeded` result carries no `boundary`, as it carries no `model`; each member's finishing checkpoint and the engine's `panel-member-finished` marker for it carry `harness`; and the sequence's result and its `sequence-step-finished` marker carry `namespace`

#### Scenario: A wrong word is refused at append
- **WHEN** a result carrying `boundary` `chroot` is offered to the store's append fence under v4
- **THEN** the append is refused naming the schema path, nothing is written, and the engine journals the attempt's failure as decision 0034 ruling 6 reads

#### Scenario: v4 dispatch is the 0.9 line
- **WHEN** append, export, import verification or offline verification reads the `engine` string of a run's manifest
- **THEN** an engine at or after `0.10.0` is judged under v5, an engine at or after `0.9.0` but before `0.10.0` under v4, an engine at or after `0.8.0` but before `0.9.0` under v3, and an earlier or unparseable engine under v1
- **AND** a journal written by the tagged 0.9.0 or 0.9.1 engine, which carries no `boundary`, still validates, because v4 adds an optional property and takes none away

#### Scenario: The contract file is published beside the frozen ones
- **WHEN** the frozen-contracts test and the store's embedded-copy test run
- **THEN** `contracts/seat-record.v4.schema.json` exists with the title `Forge seat record v4`, the embedded copy equals it byte for byte, the v1, v2 and v3 files keep their bytes, and the contracts README carries a v4 row

#### Scenario: Every fence agrees at the version boundaries
- **WHEN** the same records are checked at append, export, import verification and offline verification with manifest engines `0.7.9`, `0.8.0`, `0.8.99`, `0.9.0`, `0.9.1`, `0.9.99`, `0.10.0`, `0.10.1`, `1.0.0` and an unparseable string
- **THEN** all four paths select respectively v1, v3, v3, v4, v4, v4, v5, v5, v5 and v1, and agree on acceptance or refusal for each record
- **AND** a v5-only field is refused under v4 instead of selecting v5 by its presence

#### Scenario: Valid unstamped 0.10.0 history remains readable
- **GIVEN** v4-valid 0.10.0 checkpoints with no `site_ref`: a resumed Codex row without `root_session`, a third-party row carrying only a refusal reason, and a third-party row carrying resumed beside a refusal reason
- **WHEN** append, export, import verification and offline verification judge those records under v5
- **THEN** all remain valid without new fields, rewritten bytes or invented confirmation, and historical absent launch evidence remains absent
- **AND** adding `site_ref` makes each of these incomplete or inconsistent new-format records fail the corresponding new fence condition, without changing the engine's boundary-stamping rule

#### Scenario: v5 retains boundary authority and refusal behavior
- **WHEN** a 0.10.0 model invocation's launch checkpoint reaches the engine with a driver-supplied boundary that differs from its current realm boundary
- **THEN** the engine replaces it with the current boundary before v5 validation, as under v4; a record naming no model loses any driver-supplied boundary
- **AND** a record offered directly to the v5 fence with boundary `chroot` is refused at its would-be sequence with no append, with the same bounded schema-path diagnostic and attempt-failure behavior as v4

#### Scenario: v5 is published without changing any frozen version
- **WHEN** the published-contract and embedded-copy checks run for the new version
- **THEN** `contracts/seat-record.v5.schema.json` has title `Forge seat record v5`, its embedded copy is byte-identical, and the contracts README registers v5 beside v4
- **AND** every published and embedded v1–v4 file retains its original bytes, including the v4 boundary vocabulary and optional-property compatibility

### Requirement: The seat input and prompt name the boundary the seat stands under
The seat input of a site with hands SHALL carry `boundary`, the realm's
word, under every boundary, and SHALL carry the `hands: boxed` marker
exactly when Brokkr builds the box — under `namespace`, `seatbelt` and
`container` — so the marker is never a false statement: under `harness`
and `open` no box of Brokkr's stands and no workspace tool is served,
and the marker is absent. Under `harness` the input of a gate-class site
whose resolved adapter declares `hands.harness.result` as `last-message`
SHALL also carry `result_delivery: last-message`; with `file`, or for a
work-class site, no such field is present. The rendered prompt SHALL
follow the two: under `namespace`, `seatbelt` and `container` exactly
today's paragraph, which names the `mcp__brokkr__workspace` tool as the
only writer; under `harness` a paragraph that names the word, says the
seat runs under its harness's own sandbox with no workspace tool served,
and — with `file` — says the result path is the one file that sandbox
lets it write, or — with `last-message` — says the seat's final message
must be exactly the result object, which the harness writes to the
result path, in which case the result contract's line that asks for a
file written says so too; under `open` the same with the word `open`
and no delivery change. The paragraph — under every boundary — is
rendered for model-backed sites only: an exec site's prompt SHALL carry
no hands paragraph, because its script reads the composed environment
and not prose (proposal D31). A site without hands SHALL carry neither
field and today's prompt unchanged (decision 0046 rulings 1, 3 and 4;
decision 0043 as amended by the boxed-marker fix).

#### Scenario: namespace keeps today's words
- **WHEN** a boxed site's input and prompt are rendered under `namespace`
- **THEN** the input carries `hands: boxed` and `boundary: namespace`, and the paragraph is byte-identical to today's, naming the workspace tool as the only writer

#### Scenario: harness does not claim the workspace tool
- **WHEN** a boxed gate site's input and prompt are rendered under `harness` on an adapter whose `result` is `file`
- **THEN** the input carries `boundary: harness`, no `hands` marker and no `result_delivery`, and the paragraph names `harness`, does not name `mcp__brokkr__workspace`, and says the result path is the one file the sandbox lets the seat write

#### Scenario: An exec site's prompt carries no hands paragraph
- **WHEN** the shipped verify seat of `bundles/self` — an exec site with hands — has its input and prompt rendered under `namespace`, `harness` and `open` in turn
- **THEN** the input carries `boundary` each time and `hands: boxed` only under `namespace`, and the rendered prompt names neither `mcp__brokkr__workspace` nor the boundary word in a hands paragraph under any of the three

#### Scenario: harness with a last-message door changes the contract
- **WHEN** a boxed gate site's input and prompt are rendered under `harness` on an adapter whose `result` is `last-message`
- **THEN** the input carries `boundary: harness` and `result_delivery: last-message`, and the prompt says the final message must be exactly the result object, names the result path the harness writes it to, and does not ask the seat to write a file

#### Scenario: open carries the word and no marker
- **WHEN** a boxed site's input is rendered under `open`
- **THEN** it carries `boundary: open`, no `hands` marker and no `result_delivery`

#### Scenario: A site without hands carries neither
- **WHEN** a site without hands is rendered under `harness`
- **THEN** its input carries neither `hands` nor `boundary` and its prompt has no hands paragraph

## Provenance

- `2026-09-06-boundary-named-slice-i` — folded 2026-09-06
- `2026-09-09-226-session-resumption` — folded 2026-09-09
