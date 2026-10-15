## MODIFIED Requirements

### Requirement: The Brokkr entry and panel

The client SHALL register a **Brokkr** entry in DSH's panel list and a **Brokkr** tab in every conversation's view ring, after Chat and Trajectory. The entry's icon SHALL show the number of runs waiting on the operator. Each SHALL open the console with two views: Realms and Analytics. Runs SHALL be labelled by title. Run ids SHALL appear only under a collapsed "Details for audit".

#### Scenario: Waiting count

- **WHEN** four runs across all realms need the operator
- **THEN** the Brokkr entry's icon shows 4 and the Realms header reads "4 waiting on you"

#### Scenario: Beside Chat

- **WHEN** a conversation with a first message is open
- **THEN** its view tabs read Chat, Trajectory (with developer tools on) and Brokkr, and the left-panel entry is still there

## ADDED Requirements

### Requirement: A conversation's tab opens on the conversation's realm

The tab SHALL open on the realm the conversation's folder belongs to, as `brokkr realms --json` places it: inside a realm's folder, or in a git worktree of one, read from the worktree's `.git` file without running git. With no realms map, the workspace's folder SHALL be the one hearth. The tab SHALL say which realm the conversation is in, or that its folder is in none, and then show every realm. A realm's bar SHALL list every realm to move between.

#### Scenario: A conversation in a worktree

- **WHEN** a conversation's folder is a git worktree of the `brokkr` checkout
- **THEN** its Brokkr tab opens on `brokkr` and reads "this conversation is in brokkr"

#### Scenario: A folder in no realm

- **WHEN** a conversation's folder is outside every realm
- **THEN** the tab shows every realm and reads "this conversation's folder is in no realm Brokkr lists"

### Requirement: The console follows DSH's theme

The console SHALL draw in a light set of colours, and in a dark set whenever DSH marks its page dark. No rule SHALL name a colour outside the two sets.

#### Scenario: Dark

- **WHEN** the operator chooses Dark in DSH's Settings
- **THEN** the console, in the panel and in the tab, is drawn in the dark set, and every realm name is legible

### Requirement: The run graph is Brokkr's phases, steps and roads back

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each phase `returns` names, solid and in the rail's own colour, with the ruling that sent it back.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions sit in a bracket either side of the rail, and the rail runs on to the chief

### Requirement: A pick on the graph shows its seats as the TUI reads them

A phase's name, every step and every arrow between phases SHALL be pickable; picking one again SHALL let it go. A phase SHALL pick every step in it; an arrow SHALL pick the ruling that moved the run from one phase to the next, found by the `from` and `next` its payload records. The reader SHALL show the pick, or with nothing picked the run's latest seat, and the seat behind it with Brokkr's own cells: model (claimed) and boundary, effort, tokens as Σ total · ↑ in · ↓ out, cache hit, cost, turns and attempts, and its transcript. An absent cell SHALL show Brokkr's note. A seats table SHALL compare every seat (a picked phase shows only its own), mark the seat behind the pick, and pick a seat when clicked. A pick SHALL outlive the run view's live refreshes and SHALL be dropped once the view no longer holds it.

#### Scenario: A review member

- **WHEN** the operator picks the security member of a review step
- **THEN** the reader reads model `gpt-5.5`, tokens `Σ 3.01M tok · ↑ 3.00M · ↓ 1.2k`, cache hit `99.9%` and cost `$2.4179`

#### Scenario: An arrow

- **WHEN** the operator picks the arrow from implement to review
- **THEN** the reader shows the ruling `IMPL-OK` with its words, and the implement seat behind it

#### Scenario: A phase

- **WHEN** the operator picks the review phase
- **THEN** both review members are picked and the seats table shows only them

### Requirement: A question is about the pick first and its run second

The "Asking about" bar SHALL name what a question is about, closest first: the picked step, arrow or phase, then its run, then its realm. The question's prompt SHALL open with the pick, naming a seat by its participant key so the model can read its transcript, and SHALL give the run as the outer context. With no run picked, it SHALL ask about the realm.

#### Scenario: A picked seat

- **WHEN** the security seat is picked and the operator asks "Is the finding real?"
- **THEN** the prompt opens "About the seat “security” (participant key …)" and gives the run as its outer context

### Requirement: Live work wears Brokkr's pulse

While a run is driven, its active nodes SHALL strike and throw a halo, as ui.html's forge pulse does; a parked or finished run SHALL NOT pulse. The console's mark and the left-panel icon SHALL strike while any run in the fleet is running. With reduced motion asked for, nothing SHALL animate.

#### Scenario: A running run

- **WHEN** a running run's security member is active
- **THEN** its dot strikes with a halo, and no other node does

### Requirement: A participant's tokens are read apart

`brokkr inspect --json`'s participants SHALL carry `tokens`: `total` (`3.99M tok`), `input` (`3.98M`), `output` (`14k`) and `cache_hit`, cache reads over input to one decimal; each a cell, absent with its reason when the harness reported nothing to read. The view's wire version SHALL be 22.

#### Scenario: A codex seat

- **WHEN** a seat reports 3,975,322 input tokens, 3,830,272 of them cache reads, and 14,051 output
- **THEN** its tokens read `3.99M tok`, `3.98M`, `14k` and `96.4%`

### Requirement: Why the run waits, beside the reader

When the run waits, "why it waits" SHALL stand beside the reader, the same height: the park reason word for word with its rule, where it parked and since when, and a way to read the seat it parked at; a clarify park's questions with Open the spec; the rule that stopped it; or the quarantine. A run with nothing to wait on SHALL give the reader the whole row.

#### Scenario: A parked run

- **WHEN** a run is parked by REVIEW-SECURITY-RESIDUAL
- **THEN** "why it waits" quotes the park reason and names the rule, and the reader stands beside it

### Requirement: A seat's transcript is read in the reader

The reader SHALL show the seat's transcript through `brokkr transcript --json`, asked for by realm, run and participant key through a fenced route whose journal is the listing's, never a path the browser sent. It SHALL show the last turns, newest last, each block bounded, and SHALL read it again whenever the run's head moves, so a working seat's follows its tail. A seat whose view records no transcript SHALL show Brokkr's note and ask for nothing.

#### Scenario: A seat with no transcript

- **WHEN** a script seat's view records no transcript reference
- **THEN** the reader shows "no transcript reference recorded" and no transcript is asked for
