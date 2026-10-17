## ADDED Requirements

### Requirement: The console has a Library tab beside Analytics

The console SHALL show a **Library** tab after Realms and Analytics, in the sidebar page and in a conversation's tab. In a conversation's tab, it SHALL open on the conversation's realm when Brokkr lists one. It SHALL offer a realm picker, a range (24h, 7d, 30d), and two views, **Graph** and **Pareto**. It SHALL send no command and write nothing to the journal.

#### Scenario: Opening the tab

- **WHEN** the operator picks Library
- **THEN** the Graph view opens on the first realm, or on the conversation's realm in its tab, and on the recipe with the most runs in range

### Requirement: The Graph view draws the recipe's state machine

The Graph view SHALL draw:
- each state as a box holding its seats and models;
- the rules joining each pair of states as one link:
  - solid when any of its rules fired, as wide as the runs it moved, with each outcome that fired as a chip (`fail 3`);
  - faint and dashed, with no chip, when none fired;
  - in the stop tone into `stop`, and in the done tone into the terminal that completes a run;
- no outcome chip over another chip or over a state;
- self-loops above their state;
- back edges above the row;
- `stop` beneath the row and `done` at its end.

It SHALL draw the pulse on each live run's state. It SHALL list retired rules and the drift note beneath the graph when the view carries them. Picking a state SHALL show its rules with counts, its figures, and its seats.

#### Scenario: A rule that never fired

- **WHEN** every rule from `review` to `verify` has count 0
- **THEN** their link is faint and dashed with no chip, and the state panel lists each of them with "0"

#### Scenario: Two rules into one state

- **WHEN** `REVIEW-RESIDUAL-SECURITY` (residual, 6) and `REVIEW-SECURITY-HOLD` (security-hold, 2) both go from `review` to `stop`
- **THEN** one link joins them, with the chips `residual 6` and `security-hold 2`

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

### Requirement: Tokens read one way on every surface

Every surface that shows a seat's or a state's tokens SHALL read them from Brokkr's shared token figure, as `Σ total ↑ in ↓ out` with the cache hit beside them: the run view's seat row, the Library's state panel and Pareto table, the TUI's seat pane, and `brokkr graph`'s text. Where a surface has one cell, the cache hit SHALL follow the arrows in it.

#### Scenario: A seat with cached input

- **WHEN** a seat metered 1.19M input, 48k output and 1.19M cache reads
- **THEN** the TUI's seat pane reads `Σ 1.24M tok ↑ 1.19M ↓ 48k · 99.9% cache`, and the console shows the same arrows with 99.9% as the cache hit

#### Scenario: Nothing reported

- **WHEN** a state's seats reported no tokens
- **THEN** its tokens read as Brokkr's absence mark, with no arrows

### Requirement: One recipe picker however many recipes

The Graph view SHALL offer its recipes in one dropdown: those that ran in range first, each with its runs, the most first, then those that did not, by name, and SHALL say how many recipes the realm keeps.

#### Scenario: A realm with eighteen recipes

- **WHEN** a realm keeps 18 recipes and two ran in range
- **THEN** the dropdown lists those two first with their runs, then the other sixteen by name, and the line beside it reads "18 recipes in this realm"

### Requirement: The question boxes are hidden unless turned on

The console SHALL show no question box (Ask Brokkr, Ask about this run) on the Realms views or a run unless `BROKKR_CONSOLE_ASK` is `1` where DSH starts.

#### Scenario: The default

- **WHEN** `BROKKR_CONSOLE_ASK` is unset
- **THEN** neither the Realms view nor a run shows a question box

