## MODIFIED Requirements

### Requirement: The run view

Selecting a run SHALL show:

- the run list grouped needs you / running / last 24 hours / older, every run
  in exactly one group (older holding acknowledged quarantines and every run
  that finished before the last day), with a text filter;
- the run header with Brokkr's standing and, for a run entered at a gate, the
  gate it entered at and the runs it names as continued, each led by its
  title, `brokkr_view::title` of its feature, with its id after, on one line
  however many lines a feature holds, as the run view derives them from the
  run's own start (`run-continuation`), claiming nothing beyond the naming;
  and, for a run whose recorded entry the view cannot read, that line stating
  the entry unreadable with the reader's error, never a header painted as an
  ordinary run's;
- the graph of steps with their members, the model that served each, visits
  and the return paths taken;
- the control plane on the graph, and why the run waits when it does;
- the timeline, newest first.

The view SHALL use only Brokkr's standings and SHALL NOT show a
queued/started/verified state machine.

The header's entry SHALL be the run view's one sentence, painted as given:
each continued run led by `brokkr_view::title` of its feature, the feature's
first line that says anything clamped at a word to 60 display columns, so a
multi-line feature never spills onto a second header line; the whole feature
stays in the run view's parts and in `brokkr inspect --json`, never on the
one-line header. A present `run/started.entry` the view cannot read SHALL be
stated in the header as unreadable with the reader's error (the view's law 2:
a refusal is reported, never repaired), and the rest of the run SHALL be
painted as the fold gives it.

#### Scenario: A review sent work back

- **WHEN** security review once ruled the run back to implement
- **THEN** the graph shows the return path with the ruling's time and rule

#### Scenario: A run entered at verify continuing two runs

- **WHEN** the console shows a run whose `run/started` records an entry at `verify` continuing runs `a` and `b`
- **THEN** the header names `verify` and the two runs by their titles, ids after, in the order named, and derives nothing the run view did not

#### Scenario: A continued run's multi-line feature is its title on the header

- **WHEN** the console shows a run entered at `verify` continuing run `a`, whose recorded feature is three lines, the first `land the parked slice`, the second `It stopped at REVIEW-REFORGE-EXHAUSTED with every commit on the branch.` and the third `Judge the whole branch again.`
- **THEN** the header's entry is one line, `entered at verify, continuing land the parked slice (a)`, and neither the second nor the third line is painted on it

#### Scenario: An entry the view cannot read is stated in the header

- **WHEN** the console shows a run whose `run/started.entry` is the string `verify`
- **THEN** the header states the entry unreadable with the reader's error, names no gate entered and no continued run, and the graph and the timeline read as the fold gives them

### Requirement: The run graph is Brokkr's phases, steps and roads back

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. The rail's order SHALL be the table's `phases` list as the run's start recorded it (`run-continuation`), each phase the journal entered standing once with its visits and a phase the order does not name standing after them by first entry; for a journal whose start recorded no order, or recorded one the view cannot read (`run-continuation`), first entry. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each road in a phase's `returns`, solid and in the rail's own colour, with the rules that sent the run back and how many times it was taken. `returns` SHALL name only a ruling whose `next` stands strictly before its `from` in the rail's order, by the one predicate `brokkr-core` holds and the engine hands `returned_from` by (`gate-entry`), so a journal with a recorded order names the returns the engine took, an entered run's or a landing's first return into a phase listed ahead of where the run began included, and never the forward step after one; a forward step into a phase entered before, and a phase ruled onto itself, SHALL NOT be a road back; a ruling naming a phase the rail does not hold SHALL be no road. Each road SHALL run square below the phase names, down, across its lane and up, from a departure and to a landing of its own, so no two roads share an arrowhead; roads SHALL nest, the longest outermost. No label SHALL meet any road, and the graph SHALL be tall and wide enough for every label. A label SHALL say as much as its room allows, and the whole sentence on hover.

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

#### Scenario: A recorded order the view cannot read reads as none

- **WHEN** the journal of "One reforge, then a forward second pass" is viewed with its `run/started.phases` a string, or a list holding a number
- **THEN** its rail and its one road read exactly as that scenario reads them, and no state word or road the journal did not give is shown

### Requirement: A participant's tokens are read apart

`brokkr inspect --json`'s participants SHALL carry `tokens`: `total` (`3.99M tok`), `input` (`3.98M`), `output` (`14k`) and `cache_hit`, cache reads over input and cache writes to one decimal; each a cell, absent with its reason when the harness reported nothing to read. The view's wire version SHALL be one past `main`'s `VIEW_VERSION` when this change merges `main`, the console host pinning it: `main` stands at 24, and #787 (pull request #815), in the merge path ahead of this change, takes 25, so this change's wire is expected to be 26. Whatever number `main` holds at the merge, the change SHALL take the next, and every pin of the current version SHALL move with it.

#### Scenario: A codex seat

- **WHEN** a seat reports 3,975,322 input tokens, 3,830,272 of them cache reads, and 14,051 output
- **THEN** its tokens read `3.99M tok`, `3.98M`, `14k` and `96.4%`
