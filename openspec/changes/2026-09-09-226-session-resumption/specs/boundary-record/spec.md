## MODIFIED Requirements

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
