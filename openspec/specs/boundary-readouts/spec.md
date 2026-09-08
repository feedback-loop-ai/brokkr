# boundary-readouts Specification

## Purpose
Every readout that shows a seat's model shows its boundary in the same
row, and a run whose gate stood under `harness` or `open` is rendered
*unboxed* wherever the run is summarised. The word in the data is the
plain one; the word on the screen is the adjective (decision 0046
ruling 3; decision 0013's one derivation; decision 0031 ruling 3 for
old journals).

## Requirements

### Requirement: The view derives the boundary beside every model cell
`brokkr-view` SHALL carry a `boundary` cell beside every `model` cell it
exposes — on `Participant`, on the phase rail's `Node`, on every
`CheckpointRow` and on every `JournalRow`, the pair carried as one
`ModelAtBoundary { model, boundary }` unit flattened onto the carrier as
`served`, so the wire keeps `model` and gains `boundary` as siblings and
a renderer cannot take one without the other in reach (design DD12) —
each derived from the same
`effect/started.boundary` entry: a participant's from its site's entry
with the last attempt winning, as provenance does; a node's from its
participant; a checkpoint row's from the attempt the checkpoint belongs
to, the same word on every row of that attempt, because the boundary is
the attempt's fact and not the turn's; a journal row's from the effect
and site the row's model is read beside, and absent with the note
`no boundary recorded` for an event that belongs to no effect, exactly
as the row's model cell is absent there. `RunView` SHALL expose a
run-level `boundary` fact carrying the run's word, whether the run is
rendered *unboxed*, and a rendered text. The data SHALL carry the plain
word and the adjective SHALL appear only in rendered text. A
participant whose attempt journaled no entry — an attempt no site of
which declares hands — SHALL read the `boundary` a record of its
attempt carries beside a `model` — its finishing checkpoint, its
successful result or the engine's own member- or step-finished marker
(design DD19) — `not applicable` when that is the stamp; a participant with neither an entry nor a stamp SHALL render
an explicit absence, the absent mark with the note `no boundary
recorded`, and never a default — every model row of a journal written
before this change, boxed or not, and a running seat whose finishing
record has not landed; `not applicable` is rendered only where an entry
or a record carries it and never derived from the manifest (design
DD13). Whether the run declares hands at all is read once from the
`run/started` manifest — the one manifest the view already reads there,
beside the agents roster — for the run-level fact alone; and an entry
whose word is outside the vocabulary or which lacks a `member` tag
SHALL be read as not recorded for that site, never as boxed or unboxed
(design DD14). `VIEW_VERSION` SHALL advance to 9,
because an additive model field moves the wire version (decision 0046
ruling 3; decision 0031 ruling 3; decision 0013).

#### Scenario: A boxed seat's row carries the word
- **WHEN** a journal's boxed gate seat started under `harness`
- **THEN** its participant's `boundary` cell reads `harness`, present, beside its `model` cell

#### Scenario: A pre-0046 journal renders absence
- **WHEN** the view derives a journal whose `run/started` manifest carries `hands` but no `boundary` and whose `effect/started` events carry no `boundary`
- **THEN** every participant's `boundary` cell, boxed or not, is absent with the note `no boundary recorded`, the run-level fact is absent with the same note, and no surface prints `namespace` for it

#### Scenario: A site without hands reads not applicable from its record
- **WHEN** a participant's attempt journaled no `boundary` entry and its finishing checkpoint carries `not applicable`, as the engine stamps a site without hands
- **THEN** its `boundary` cell reads `not applicable`, read from the record and never from the manifest

#### Scenario: Every model cell has a boundary cell beside it
- **WHEN** the view is derived for a run with a boxed seat that journaled three turns
- **THEN** the seat's participant, its phase-rail node, each of its three checkpoint rows and each journal row of its effect carry a `boundary` cell reading the same word as the participant's, and a `run/started` row's `boundary` cell is absent

#### Scenario: An old plain journal renders absence and no run-level fact
- **WHEN** the view derives a journal written before this change whose `run/started` manifest carries no `hands` key at all
- **THEN** every participant's `boundary` cell is absent with the note `no boundary recorded`, none reads `not applicable`, and the run-level fact renders nothing, because the run declares no hands

#### Scenario: A hands-less seat still running renders absence
- **WHEN** a hands-less seat's attempt has started and its finishing checkpoint has not yet landed
- **THEN** its `boundary` cell is absent with the note `no boundary recorded`, never `namespace` and never `not applicable`

#### Scenario: An entry outside the vocabulary is not recorded
- **WHEN** a journal's `effect/started.boundary` carries an entry whose word is `chroot`, or one without `member`
- **THEN** that site's `boundary` cell is absent with the note `no boundary recorded`, the run-level fact does not read *unboxed* on its account, and no surface prints the word

#### Scenario: The wire version moves
- **WHEN** `--json` is emitted by `inspect`
- **THEN** `view_version` is 9 and the participant and run-level boundary fields are present as `null`-bearing cells, never skipped

### Requirement: A run whose gate stood under harness or open is rendered unboxed
The run-level fact SHALL be *unboxed* exactly when any
`effect/started.boundary` entry of the run has `gate` true and a word of
`harness` or `open`; a run with no boxed site is neither boxed nor
unboxed and renders nothing for it; a run whose boxed sites stood under
`namespace`, `seatbelt` or `container` renders the word alone; a run
whose manifest declares hands and whose journal holds no valid entry —
a journal written before this change, or a run before its first boxed
attempt — renders the fact absent with the note `no boundary recorded`
and never a word (design DD13). Every
surface that summarises a run — `brokkr inspect`'s header line, `brokkr
watch`'s frame header, the TUI's run header, and the web console's run
header — SHALL print the adjective from this one derivation and never
compose its own (decision 0046 ruling 3; decision 0013).

#### Scenario: A harness-judged run renders unboxed everywhere
- **WHEN** a run's boxed gate seat started under `harness`
- **THEN** `inspect`'s header, `watch`'s header, the TUI run header and the console's run header all print `harness · unboxed`

#### Scenario: A namespace run renders the word alone
- **WHEN** every boxed site of a run started under `namespace`
- **THEN** the run is summarised with `namespace` and never with `unboxed`

#### Scenario: A boxed work seat under harness alone is not unboxed
- **WHEN** a run's only boxed site is a work-class seat that started under `harness`
- **THEN** the run is summarised with `harness` and not with `unboxed`, because no gate stood there

#### Scenario: A plain run says nothing
- **WHEN** a run boxes no site
- **THEN** no surface prints a boundary for the run

### Requirement: Every readout that names a seat's model names its boundary
Every surface that prints a model cell SHALL print the boundary cell
beside it, from the same derivation and never composed on its own. In
this tree those surfaces are: the terminal's seats table and its
per-seat lines under `brokkr inspect`, `inspect --seat`, `brokkr watch`
and `brokkr seats` — a thin verb that renders the seats block `inspect`
renders, from the same view, and with `--json` prints the view model
verbatim, the bytes `inspect --json` prints under the same
`view_version`, so no wire object is versioned for the verb (design
DD11) —
and the terminal's decision-trail rows, which print `· model <x>`; the TUI's seat table, seat detail, checkpoint rows and
journal rows; and the web console's participants table, seat detail,
checkpoint stream and journal rows. `brokkr costs` and `brokkr compare`
name a seat's model from the seat records themselves, through the one
seat-costs derivation, so their per-seat record SHALL gain `boundary`
reduced exactly as `model` is — the set of words the seat's records
that name a model carry, one word or a joined list —
reading `not recorded` when no record carries one, an explicit absence
and never a default; and `compare` SHALL report a boundary difference
between two runs as a first-class divergence, the way it reports a
model difference. `compare` also names each participant's model a
second time, from the view, in the `resolution` map `resolution_of`
builds and `resolution_divergence` compares; that map SHALL carry
`boundary` beside `model` per participant, read through the pair
helper's JSON face, and the divergence SHALL report a boundary
difference as it reports a model difference. A roster-style pin test
SHALL read every readout source and fail, naming the source, where
`served.model` is read outside the one pair helper — a text face for
the renderers, a JSON face for `compare`'s `resolution` map — or where
the `model` key of a seat-costs record is rendered without the boundary
beside it. The web console is a page and not a Rust source: `ui.html`
reads the flattened wire, where `model` and `boundary` are siblings on
the participant, the node, the checkpoint row and the journal row, and
has no Rust helper to route through, so a rule phrased over
`served.model` alone would never reach it. The page SHALL therefore
carry one page-side pair helper — a single script function that takes
a carrier and returns the pair's two cells, the only place in the page
that names `.model` — and the pin test SHALL scan `ui.html` too,
failing and naming the line where `.model` is read off a carrier
outside that function, so the page is held to the rule the Rust sources
are held to; the console's rendering tests prove the two cells land in
one row (design DD12).
`brokkr export` renders no prose and is read as the record itself: the
exported journal carries the plain word in `effect/started.boundary`
and in every seat record this engine writes, and `verify-run` accepts
it (decision 0046 ruling 3's binding on `roster.rs`-style pins;
decision 0031 ruling 1's list of the readouts).

#### Scenario: inspect's seats table and trail
- **WHEN** `brokkr inspect` renders a run with a boxed seat
- **THEN** the seats table has a `boundary` column beside `model`, the per-seat lines print the boundary beside the model, and a decision-trail row that prints `· model <x>` prints `· boundary <y>` beside it

#### Scenario: brokkr seats
- **WHEN** `brokkr seats --run <id>` renders a run with a boxed seat, and again with `--json`
- **THEN** the seats block is the one `brokkr inspect` prints, with the `boundary` column beside `model`, and the JSON is byte-identical to `brokkr inspect --run <id> --json`

#### Scenario: The TUI
- **WHEN** the TUI renders the seat table, a seat's detail pane, its checkpoint rows and the journal rows
- **THEN** each carries the boundary cell beside the model cell

#### Scenario: The web console
- **WHEN** the console renders the participants table, a seat's detail, its checkpoint stream and the journal rows
- **THEN** each carries the boundary cell beside the model cell, read from the model served by `/api/view/<run>` and computed nowhere on the page, every `.model` read on the page passing through its one pair helper

#### Scenario: costs and compare name the boundary
- **WHEN** `brokkr costs` and `brokkr compare` report a run whose boxed gate stood under `harness` beside a run whose boxed gate stood under `namespace`
- **THEN** each per-seat record carries `boundary` beside `model`, `costs` prints the plain word, `compare` reports the difference as a divergence, and a pre-0046 journal's seat reads `not recorded`

#### Scenario: compare's resolution map carries the pair
- **WHEN** `brokkr compare` reports a run whose boxed gate stood under `harness` beside a run whose boxed gate stood under `namespace`
- **THEN** each participant's entry in the `resolution` map carries `boundary` beside `model`, and `resolution_divergence` names the boundary difference for that site as it would a model difference

#### Scenario: export carries the word as data
- **WHEN** `brokkr export` writes the journal of a run whose boxed gate stood under `harness`
- **THEN** the exported `effect/started` events and seat records carry the plain word, `verify-run` accepts the file, and no adjective appears in the export

#### Scenario: The pin test
- **WHEN** a Rust readout source reads a `model` cell — a participant's, a node's, a checkpoint row's or a journal row's — outside the pair helper, or emits the `model` key of a seat-costs record without the boundary beside it, or `ui.html` reads `.model` off a carrier outside its page-side pair helper
- **THEN** the pin test fails naming the source, and for the page the line

### Requirement: The delivery gate's check summary says unboxed
`scripts/delivered-by-brokkr.sh` SHALL, after verifying the anchored
run's journal, read that run's `effect/started` events and append
` · unboxed` to the tier line and to the vouch line when any entry has
`gate` true and a word of `harness` or `open`; append ` · boundary not
recorded` when the run's manifest carries `hands` and no `boundary`;
append ` · boundary not recorded` too when an entry's word is outside
the vocabulary or an entry lacks its tag, never `unboxed` and never
nothing (design DD14); and append nothing for a run that boxes nothing. A docs-tier preflight
run SHALL be read the same way on its own line. The data read is the
plain word; the adjective is the script's rendering. The script SHALL
stay bash 3.2-compatible (decision 0046 ruling 3; decision 0038's gate).

#### Scenario: A harness-judged run
- **WHEN** the gate judges a pull request whose anchored run's boxed gate started under `harness`
- **THEN** the tier line and the vouch line end with `· unboxed`

#### Scenario: A boxed run
- **WHEN** the anchored run's boxed gate started under `namespace`
- **THEN** neither line carries `unboxed`

#### Scenario: An old journal
- **WHEN** the anchored run's manifest carries `hands` and no `boundary`
- **THEN** the lines end with `· boundary not recorded`

#### Scenario: A malformed entry
- **WHEN** the anchored run's `effect/started.boundary` carries an entry whose word is outside the vocabulary
- **THEN** the lines end with `· boundary not recorded`

#### Scenario: The binding is pinned
- **WHEN** the contributing test reads the gate script
- **THEN** it finds the bindings that read `effect/started.boundary` and the word `unboxed`
