## ADDED Requirements

### Requirement: The console has a Library tab beside Analytics

The console SHALL show a **Library** tab after Realms and Analytics, in the sidebar page and in a conversation's tab. In a conversation's tab, it SHALL open on the conversation's realm when Brokkr lists one. It SHALL offer a realm picker, a range (24h, 7d, 30d), and two views, **Graph** and **Pareto**. It SHALL send no command and write nothing to the journal.

#### Scenario: Opening the tab

- **WHEN** the operator picks Library
- **THEN** the Graph view opens on the first realm, or on the conversation's realm in its tab, and on the recipe with the most runs in range

### Requirement: The Graph view draws the recipe's state machine

The Graph view SHALL draw:
- each state as a box holding its seats and models;
- each rule as an edge from its state to its `next`:
  - solid, with its count, when it fired;
  - dashed when its count is 0;
  - in the stop tone when it ends in `stop`;
- self-loops above their state;
- back edges above the row;
- `stop` beneath the row and `done` at its end.

It SHALL draw the pulse on each live run's state. It SHALL list retired rules and the drift note beneath the graph when the view carries them. Picking a state SHALL show its rules with counts, its figures, and its seats.

#### Scenario: A rule that never fired

- **WHEN** a rule's count is 0
- **THEN** its edge is dashed, and its label reads its name with "0"

#### Scenario: Picking a state

- **WHEN** the operator picks `review`
- **THEN** the side panel lists review's rules with their counts, the runs that entered, the median and p90 time, the cost with the providers that did not report, and the seats with their models

#### Scenario: A live run

- **WHEN** a run of the recipe is in `verify`
- **THEN** `verify` carries the pulse, and picking the pulse opens that run in Realms

### Requirement: The views read only Brokkr's figures

The views SHALL draw only what `brokkr graph --json` and `brokkr analytics --json` carry. The console SHALL accept only the view versions it was built for, and SHALL otherwise say which version it found.

#### Scenario: A newer Brokkr

- **WHEN** `brokkr graph --json` reports a `GraphView` version the console does not know
- **THEN** the Graph view says which version it found, and draws nothing
