# boundary-readouts

Every readout that shows a seat's model shows its boundary in the same
row, and a run whose gate stood under `harness` or `open` is rendered
*unboxed* wherever the run is summarised. The word in the data is the
plain one; the word on the screen is the adjective (decision 0046
ruling 3; decision 0013's one derivation; decision 0031 ruling 3 for
old journals).

## ADDED Requirements

### Requirement: The view derives the boundary beside the model
`brokkr-view` SHALL expose on every `Participant` a `boundary` cell
beside `model`, derived from the site's `effect/started.boundary` entry
with the last attempt winning, as provenance does, and SHALL expose on
`RunView` a run-level `boundary` fact carrying the run's word, whether
the run is rendered *unboxed*, and a rendered text. The data SHALL
carry the plain word and the adjective SHALL appear only in rendered
text. A journal written before this change SHALL render an explicit
absence for a site that declared hands — the absent mark with the note
`no boundary recorded` — and never a default; a site that declared no
hands SHALL read `not applicable`. `VIEW_VERSION` SHALL advance to 9,
because an additive model field moves the wire version (decision 0046
ruling 3; decision 0031 ruling 3; decision 0013).

#### Scenario: A boxed seat's row carries the word
- **WHEN** a journal's boxed gate seat started under `harness`
- **THEN** its participant's `boundary` cell reads `harness`, present, beside its `model` cell

#### Scenario: A pre-0046 journal renders absence
- **WHEN** the view derives a journal whose `run/started` manifest carries `hands` but no `boundary` and whose `effect/started` events carry no `boundary`
- **THEN** each boxed participant's `boundary` cell is absent with the note `no boundary recorded`, the run-level fact is absent with the same note, and no surface prints `namespace` for it

#### Scenario: A site without hands reads not applicable
- **WHEN** a participant's site declared no hands in a journal this engine wrote
- **THEN** its `boundary` cell reads `not applicable`

#### Scenario: The wire version moves
- **WHEN** `--json` is emitted by `inspect`
- **THEN** `view_version` is 9 and the participant and run-level boundary fields are present as `null`-bearing cells, never skipped

### Requirement: A run whose gate stood under harness or open is rendered unboxed
The run-level fact SHALL be *unboxed* exactly when any
`effect/started.boundary` entry of the run has `gate` true and a word of
`harness` or `open`; a run with no boxed site is neither boxed nor
unboxed and renders nothing for it; a run whose boxed sites stood under
`namespace`, `seatbelt` or `container` renders the word alone. Every
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
The seats table of `brokkr inspect` and `brokkr watch`, the seat detail
of `inspect --seat`, the TUI seat table and seat detail, and the web
console's participants table and seat detail SHALL render the boundary
cell in the same row or block as the model cell, from the same
derivation, and a roster-style pin test SHALL fail when a source that
renders a model cell renders no boundary cell. `brokkr export` renders
no prose and is read as the record itself: the exported journal carries
the plain word in `effect/started.boundary` and in every seat record
this engine writes, and `verify-run` accepts it (decision 0046 ruling
3's binding on `roster.rs`-style pins).

#### Scenario: inspect's seats table
- **WHEN** `brokkr inspect` renders a run with a boxed seat
- **THEN** the seats table has a `boundary` column beside `model`, and the seat detail prints the boundary beside the model line

#### Scenario: The TUI
- **WHEN** the TUI renders the seat table and a seat's detail pane
- **THEN** both carry the boundary cell beside the model cell

#### Scenario: The web console
- **WHEN** the console renders the participants table and a seat's detail
- **THEN** both carry the boundary cell beside the model cell, read from the model served by `/api/view/<run>` and computed nowhere on the page

#### Scenario: export carries the word as data
- **WHEN** `brokkr export` writes the journal of a run whose boxed gate stood under `harness`
- **THEN** the exported `effect/started` events and seat records carry the plain word, `verify-run` accepts the file, and no adjective appears in the export

#### Scenario: The pin test
- **WHEN** a readout source renders `model` without `boundary`
- **THEN** the pin test fails naming the source

### Requirement: The delivery gate's check summary says unboxed
`scripts/delivered-by-brokkr.sh` SHALL, after verifying the anchored
run's journal, read that run's `effect/started` events and append
` · unboxed` to the tier line and to the vouch line when any entry has
`gate` true and a word of `harness` or `open`; append ` · boundary not
recorded` when the run's manifest carries `hands` and no `boundary`;
and append nothing for a run that boxes nothing. A docs-tier preflight
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

#### Scenario: The binding is pinned
- **WHEN** the contributing test reads the gate script
- **THEN** it finds the bindings that read `effect/started.boundary` and the word `unboxed`
