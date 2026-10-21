## MODIFIED Requirements

### Requirement: The run view

Selecting a run SHALL show:

- the run list grouped needs you / running / last 24 hours / older, every run
  in exactly one group (older holding acknowledged quarantines and every run
  that finished before the last day), with a text filter;
- the run header with Brokkr's standing and, for a run entered at a gate, the
  gate it entered at and the runs it names as continued, each led by its
  feature with its id after, as the run view derives them from the run's own
  start (`run-continuation`), claiming nothing beyond the naming;
- the graph of steps with their members, the model that served each, visits
  and the return paths taken;
- the control plane on the graph, and why the run waits when it does;
- the timeline, newest first.

The view SHALL use only Brokkr's standings and SHALL NOT show a
queued/started/verified state machine.

#### Scenario: A review sent work back

- **WHEN** security review once ruled the run back to implement
- **THEN** the graph shows the return path with the ruling's time and rule

#### Scenario: A run entered at verify continuing two runs

- **WHEN** the console shows a run whose `run/started` records an entry at `verify` continuing runs `a` and `b`
- **THEN** the header names `verify` and the two runs by their features, ids after, in the order named, and derives nothing the run view did not

### Requirement: The run graph is Brokkr's phases, steps and roads back

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. The rail's order SHALL be the table's `phases` list as the run's start recorded it (`run-continuation`), each phase the journal entered standing once with its visits and a phase the order does not name standing after them by first entry; for a journal whose start recorded no order, first entry. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each road in a phase's `returns`, solid and in the rail's own colour, with the rules that sent the run back and how many times it was taken. `returns` SHALL name only a ruling whose `next` stands strictly before its `from` in the rail's order, by the one predicate `brokkr-core` holds and the engine hands `returned_from` by (`gate-entry`), so a journal with a recorded order names the returns the engine took, an entered run's or a landing's first return into a phase listed ahead of where the run began included, and never the forward step after one; a forward step into a phase entered before, and a phase ruled onto itself, SHALL NOT be a road back; a ruling naming a phase the rail does not hold SHALL be no road. Each road SHALL run square below the phase names, down, across its lane and up, from a departure and to a landing of its own, so no two roads share an arrowhead; roads SHALL nest, the longest outermost. No label SHALL meet any road, and the graph SHALL be tall and wide enough for every label. A label SHALL say as much as its room allows, and the whole sentence on hover.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions sit in a bracket either side of the rail, and the rail runs on to the chief

#### Scenario: One reforge, then a forward second pass

- **WHEN** a run goes implement, verify, review, back to implement on REVIEW-REFORGE, forward again through verify and review, and ship is ruled onto itself before it is done
- **THEN** one road is drawn, "sent back to implement by review · REVIEW-REFORGE", and no other phase names a return

#### Scenario: Two roads into one phase

- **WHEN** implement was sent back once by verify (VERIFY-FAIL) and once by review (REVIEW-REFORGE)
- **THEN** two roads land on two arrowheads under implement, the road from review outside the road from verify, and neither crosses the other's label, which reads "by verify · VERIFY-FAIL ×1" or the whole sentence

#### Scenario: A run entered at verify is sent back once

- **WHEN** a run whose start recorded `implement, verify, review, ship, done, stop` entered at `verify`, verify ruled it back to implement by VERIFY-FAIL, and it went forward again through verify and review to ship
- **THEN** the rail reads implement, verify, review, ship, implement visited once and verify twice; one road is drawn, "sent back to implement by verify · VERIFY-FAIL ×1"; and no road lands on verify, the forward IMPL-OK being none

#### Scenario: A run entered at review is reforged

- **WHEN** a run of the same shape entered at `review` was ruled back to implement by REVIEW-REFORGE and went forward again through verify to review
- **THEN** one road is drawn, "sent back to implement by review · REVIEW-REFORGE ×1", and VERIFY-PASS lands no road on review

#### Scenario: A landing's first return is drawn

- **WHEN** a `landing` run started without an entry, whose start recorded `classify, implement, verify, review, ship, done, stop`, went classify, verify, review, back to implement on REVIEW-REFORGE, and forward through verify and review to ship
- **THEN** the rail reads classify, implement, verify, review, ship; one road is drawn, "sent back to implement by review · REVIEW-REFORGE ×1"; and IMPL-OK lands no road on verify

#### Scenario: A journal that recorded no order reads as before

- **WHEN** a journal whose `run/started` carries no `phases` is viewed
- **THEN** its rail stands in first-entry order, its `returns` are the rulings whose `next` stands strictly before its `from` on that rail, and the scenarios "One reforge, then a forward second pass" and "Two roads into one phase" read as written

### Requirement: A participant's tokens are read apart

`brokkr inspect --json`'s participants SHALL carry `tokens`: `total` (`3.99M tok`), `input` (`3.98M`), `output` (`14k`) and `cache_hit`, cache reads over input and cache writes to one decimal; each a cell, absent with its reason when the harness reported nothing to read. The view's wire version SHALL be 25, the console host pinning it.

#### Scenario: A codex seat

- **WHEN** a seat reports 3,975,322 input tokens, 3,830,272 of them cache reads, and 14,051 output
- **THEN** its tokens read `3.99M tok`, `3.98M`, `14k` and `96.4%`
