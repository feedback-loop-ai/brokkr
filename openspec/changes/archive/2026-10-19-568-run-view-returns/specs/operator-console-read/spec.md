## MODIFIED Requirements

### Requirement: The run graph is Brokkr's phases, steps and roads back

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each phase `returns` names, solid and in the rail's own colour, with the ruling that sent it back. `returns` SHALL name only a ruling whose `next` stands strictly before its `from` on the rail, whose order is first entry: a forward step into a phase entered before, and a phase ruled onto itself, SHALL NOT be a road back. Each road SHALL take a lane of its own below the phase names, leaving and landing below them, its label at least a text line from any other, and the graph SHALL be tall enough for its last label.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions sit in a bracket either side of the rail, and the rail runs on to the chief

#### Scenario: One reforge, then a forward second pass

- **WHEN** a run goes implement, verify, review, back to implement on REVIEW-REFORGE, forward again through verify and review, and ship is ruled onto itself before it is done
- **THEN** one road is drawn, "sent back to implement by review · REVIEW-REFORGE", and no other phase names a return

#### Scenario: Two roads into one phase

- **WHEN** implement was sent back by verify and by review
- **THEN** two roads are drawn below the phase names, the shorter nearer, each label on a line of its own

### Requirement: A pick on the graph shows its seats as the TUI reads them

A phase's name, every step and every arrow between phases SHALL be pickable; picking one again SHALL let it go. A phase SHALL pick every step in it; an arrow SHALL pick the ruling that moved the run from one phase to the next, found by the `from` and `next` its payload records. The reader SHALL show the pick, or with nothing picked where the run is now (the phase the view marks current, with the latest seat recorded in it, or the phase alone while it has none), and the seat behind it with Brokkr's own cells: model (claimed) and boundary, effort, tokens as Σ total · ↑ in · ↓ out, cache hit, cost, turns and attempts, and its transcript. An absent cell SHALL show Brokkr's note. A seats table SHALL compare every seat (a picked phase shows only its own), mark the seat behind the pick, and pick a seat when clicked. A seat whose `visit` is 2 or more SHALL say which visit to its phase it belongs to, as Brokkr counted it; a seat of a first visit SHALL read as before. A pick SHALL outlive the run view's live refreshes and SHALL be dropped once the view no longer holds it.

#### Scenario: A review member

- **WHEN** the operator picks the security member of a review step
- **THEN** the reader reads model `gpt-5.5`, tokens `Σ 3.01M tok · ↑ 3.00M · ↓ 1.2k`, cache hit `99.9%` and cost `$2.4179`

#### Scenario: An arrow

- **WHEN** the operator picks the arrow from implement to review
- **THEN** the reader shows the ruling `IMPL-OK` with its words, and the implement seat behind it

#### Scenario: A phase

- **WHEN** the operator picks the review phase
- **THEN** both review members are picked and the seats table shows only them

#### Scenario: A second pass

- **WHEN** implement's seat of the run's second pass has `visit` 2
- **THEN** its row in the seats table reads "implement · visit 2", and the first pass's row reads "implement"
