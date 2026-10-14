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

The graph SHALL draw the run view's phases left to right, each phase's steps in order, and the members of one step side by side, with an arrow from each node to every node of the next step. Each node SHALL show Brokkr's state and served model, and no state word Brokkr did not give. The last step of a phase SHALL show the visit's duration and rule. A road back SHALL be drawn for each phase `returns` names, solid and in the rail's own colour, as ui.html draws a road that was taken.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions stand side by side, both arrows lead into the chief, and the chief's arrow leads on

### Requirement: A pick on the graph shows its seats as the TUI reads them

A phase's label and every node SHALL be pickable. Picking a phase SHALL pick every node in it; picking a node SHALL pick that node; picking either again SHALL let it go. The picked phase or step SHALL show each of its seats with Brokkr's own cells: model (claimed) and boundary, tokens, cache hit, cost, turns and attempts, and effort applied and pinned. An absent cell SHALL show Brokkr's note. A pick SHALL outlive the run view's live refreshes and SHALL be dropped once the view no longer holds it.

#### Scenario: A review member

- **WHEN** the operator picks the security member of a review step
- **THEN** it reads model `gpt-5.5`, tokens `3.01M tok · in 3.00M tok · out 1k tok · …`, cache hit `99.9%` and cost `$2.4179`

#### Scenario: A phase

- **WHEN** the operator picks the review phase
- **THEN** both review members are picked and both seats are shown

### Requirement: A question is about the pick first and its run second

The assistant panel SHALL name what a question is about, closest first: the picked step or phase, then its run, then its realm. The question's prompt SHALL open with the pick, naming a seat by its participant key so the model can read its transcript, and SHALL give the run as the outer context. With no run picked, the panel SHALL ask about the realm.

#### Scenario: A picked seat

- **WHEN** the security seat is picked and the operator asks "Is the finding real?"
- **THEN** the prompt opens "About the seat “security” (participant key …)" and gives the run as its outer context

### Requirement: Live work wears Brokkr's pulse

While a run is driven, its active nodes SHALL strike and throw a halo, as ui.html's forge pulse does; a parked or finished run SHALL NOT pulse. The console's mark and the left-panel icon SHALL strike while any run in the fleet is running. With reduced motion asked for, nothing SHALL animate.

#### Scenario: A running run

- **WHEN** a running run's security member is active
- **THEN** its dot strikes with a halo, and no other node does

### Requirement: A participant names its cache hit

`brokkr inspect --json`'s participants SHALL carry `cache_hit_cell`: cache reads over input, to one decimal, or absent with its reason when either is unreported or input is zero. The view's wire version SHALL be 21.

#### Scenario: A review seat

- **WHEN** a seat reports 3,002,733 cache-read tokens of 3,004,371 input
- **THEN** its `cache_hit_cell` reads `99.9%`
