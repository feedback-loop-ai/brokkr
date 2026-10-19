## MODIFIED Requirements

### Requirement: The run graph is Brokkr's phases, steps and roads back

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each road in a phase's `returns`, solid and in the rail's own colour, with the rules that sent the run back and how many times it was taken. `returns` SHALL name only a ruling whose `next` stands strictly before its `from` on the rail, whose order is first entry: a forward step into a phase entered before, and a phase ruled onto itself, SHALL NOT be a road back. Each road SHALL run square below the phase names, down, across its lane and up, from a departure and to a landing of its own, so no two roads share an arrowhead; roads SHALL nest, the longest outermost. No label SHALL meet any road, and the graph SHALL be tall and wide enough for every label. A label SHALL say as much as its room allows, and the whole sentence on hover. The graph SHALL lay itself out against its pane's width as the pane resizes: in a wider pane it SHALL spread the phases to fill it, each arrow between phases growing up to a cap; in a narrower pane it SHALL keep the least spacing and scroll to the current phase.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions sit in a bracket either side of the rail, and the rail runs on to the chief

#### Scenario: One reforge, then a forward second pass

- **WHEN** a run goes implement, verify, review, back to implement on REVIEW-REFORGE, forward again through verify and review, and ship is ruled onto itself before it is done
- **THEN** one road is drawn, "sent back to implement by review · REVIEW-REFORGE", and no other phase names a return

#### Scenario: Two roads into one phase

- **WHEN** implement was sent back once by verify (VERIFY-FAIL) and once by review (REVIEW-REFORGE)
- **THEN** two roads land on two arrowheads under implement, the road from review outside the road from verify, and neither crosses the other's label, which reads "by verify · VERIFY-FAIL ×1" or the whole sentence
