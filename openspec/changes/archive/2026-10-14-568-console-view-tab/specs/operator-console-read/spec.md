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

The graph SHALL draw the run view's phases left to right, each phase's steps in order, and the members of one step side by side, with an arrow from each node to every node of the next step. Each node SHALL show Brokkr's state and served model. The last step of a phase SHALL show the visit's duration and rule. A road back SHALL be drawn, dashed, for each phase `returns` names.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions stand side by side, both arrows lead into the chief, and the chief's arrow leads on
