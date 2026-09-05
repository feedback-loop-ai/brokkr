# boundary-record

The record says which boundary stood: the engine's effect-start path,
the seat record as `seat-record.v4`, and the seat prompt (decision 0046
ruling 3; decisions 0031, 0034 and 0035 on how the record carries facts).

## ADDED Requirements

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
checkpoint and successful result (decision 0046 ruling 3; the extension
rule of decision 0016).

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
property `boundary` on the finishing checkpoint and on the successful
result — the five words or the sentinel `not applicable` — with v1, v2
and v3 not edited and the store embedding v4 beside them under the
embedded-copy pin test. The engine SHALL stamp `boundary` onto every
finishing checkpoint and successful result before it is appended — the
realm's word for a site with hands, `not applicable` for a site without
— because the engine is the only party that knows which boundary it
built; drivers and their conformance field sets SHALL not change. The
store's append fence SHALL validate the stamped record under v4 and
refuse a word outside the vocabulary at the seq it would have taken
(decision 0046 ruling 3 with the commission's erratum; decision 0034
rulings 1, 5 and 6; decision 0031 ruling 1 for the sentinel).

#### Scenario: A boxed exec gate's record carries the word
- **WHEN** a boxed exec gate under `namespace` succeeds
- **THEN** its finishing checkpoint and its successful result both carry `boundary` `namespace`, and `brokkr export` and `verify-run` accept the journal

#### Scenario: A site without hands carries the sentinel
- **WHEN** an inline exec seat with no hands succeeds, or a model seat on the tool-list path succeeds
- **THEN** its finishing checkpoint and successful result carry `boundary` `not applicable`

#### Scenario: A wrong word is refused at append
- **WHEN** a result carrying `boundary` `chroot` is offered to the store's append fence under v4
- **THEN** the append is refused naming the schema path, nothing is written, and the engine journals the attempt's failure as decision 0034 ruling 6 reads

#### Scenario: v4 dispatch is the 0.9 line
- **WHEN** the store reads the `engine` string of a run's manifest
- **THEN** an engine at or after `0.9.0` is judged under v4, an engine in the `0.8` line under v3, and an earlier or unparseable engine under v1
- **AND** a journal written by the tagged 0.9.0 or 0.9.1 engine, which carries no `boundary`, still validates, because v4 adds an optional property and takes none away

#### Scenario: The contract file is published beside the frozen ones
- **WHEN** the frozen-contracts test and the store's embedded-copy test run
- **THEN** `contracts/seat-record.v4.schema.json` exists with the title `Forge seat record v4`, the embedded copy equals it byte for byte, the v1, v2 and v3 files keep their bytes, and the contracts README carries a v4 row

### Requirement: The seat input and prompt name the boundary the seat stands under
The seat input of a site with hands SHALL carry `boundary`, the realm's
word, under every boundary, and SHALL carry the `hands: boxed` marker
exactly when Brokkr builds the box — under `namespace`, `seatbelt` and
`container` — so the marker is never a false statement: under `harness`
and `open` no box of Brokkr's stands and no workspace tool is served,
and the marker is absent. The rendered prompt paragraph SHALL follow
`boundary`: under `namespace`, `seatbelt` and `container` exactly
today's paragraph, which names the `mcp__brokkr__workspace` tool as the
only writer; under `harness` a paragraph that names the word, says the
seat runs under its harness's own sandbox with no workspace tool served,
and says how the result file reaches the engine as the adapter's
measured fragment requires; under `open` the same with the word `open`.
A site without hands SHALL carry neither field and today's prompt
unchanged (decision 0046 rulings 1, 3 and 4; decision 0043 as amended by
the boxed-marker fix).

#### Scenario: namespace keeps today's words
- **WHEN** a boxed site's input and prompt are rendered under `namespace`
- **THEN** the input carries `hands: boxed` and `boundary: namespace`, and the paragraph is byte-identical to today's, naming the workspace tool as the only writer

#### Scenario: harness does not claim the workspace tool
- **WHEN** a boxed site's input and prompt are rendered under `harness`
- **THEN** the input carries `boundary: harness` and no `hands` marker, and the paragraph names `harness`, does not name `mcp__brokkr__workspace`, and says how the result file is written

#### Scenario: open carries the word and no marker
- **WHEN** a boxed site's input is rendered under `open`
- **THEN** it carries `boundary: open` and no `hands` marker

#### Scenario: A site without hands carries neither
- **WHEN** a site without hands is rendered under `harness`
- **THEN** its input carries neither `hands` nor `boundary` and its prompt has no hands paragraph
