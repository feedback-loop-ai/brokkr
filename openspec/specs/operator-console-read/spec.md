# operator-console-read Specification

## Purpose
Give the operator console a read of every realm's runs, the queue and the
run view as Brokkr's readouts derive them, led by human titles and
refreshed without the console deriving meaning of its own.

## Requirements

### Requirement: The console reads Brokkr only through its readouts

The DSH host plugin SHALL obtain every run, fleet, queue, Muninn, analytics
and transcript fact from Brokkr's own readouts. It SHALL NOT open a journal
or derive a standing, need, admissibility, reason or metric itself.

#### Scenario: A parked run shows Brokkr's own reason

- **WHEN** a run is parked by a rule
- **THEN** "why it waits" shows the park reason as the readout gives it, word for word, with the rule id
- **AND** no text on the card is computed by the client from raw events

### Requirement: The Brokkr entry and panel

The client SHALL register a **Brokkr** entry in DSH's panel list and a **Brokkr** tab in every conversation's view ring, after Chat and Trajectory. The entry's icon SHALL show the number of runs waiting on the operator. Each SHALL open the console with two views: Realms and Analytics. Runs SHALL be labelled by title. Run ids SHALL appear only under a collapsed "Details for audit".

#### Scenario: Waiting count

- **WHEN** four runs across all realms need the operator
- **THEN** the Brokkr entry's icon shows 4 and the Realms header reads "4 waiting on you"

#### Scenario: Beside Chat

- **WHEN** a conversation with a first message is open
- **THEN** its view tabs read Chat, Trajectory (with developer tools on) and Brokkr, and the left-panel entry is still there

### Requirement: Every hearth in realms.json is shown

The Realms view SHALL show one card per realm in every hearth `realms.json`
names. Each card SHALL show:

- each run that needs the operator, with its title, step and Brokkr's
  short reason;
- the count of running runs;
- the counts shipped and stopped in the last 24 hours.

A journal that does not load SHALL show its realm as quarantined with the
refusal.

#### Scenario: A quiet run

- **WHEN** a lanetally run has recorded nothing for longer than the staleness bound
- **THEN** its card shows "no activity for 3h · last step: specify" under the realm

### Requirement: The queue shows admission, not dispatch

The Realms view SHALL list queued entries with their route and Brokkr's
admission verdict:

- could start, with the route's room;
- waiting, with its reason;
- held, with its waits and who held it.

It SHALL show no control that starts a queued run. It SHALL show the note
that starting comes with Brokkr's dispatcher.

#### Scenario: Route full

- **WHEN** an entry's route is at its host ceiling
- **THEN** it shows "waiting · route full: 3 of 3 runs on this machine"

### Requirement: The run view

Selecting a run SHALL show:

- the run list grouped needs you / running / last 24 hours / older, every run
  in exactly one group (older holding acknowledged quarantines and every run
  that finished before the last day), with a text filter;
- the run header with Brokkr's standing;
- the graph of steps with their members, the model that served each, visits
  and the return paths taken;
- the control plane on the graph, and why the run waits when it does;
- the timeline, newest first.

The view SHALL use only Brokkr's standings and SHALL NOT show a
queued/started/verified state machine.

#### Scenario: A review sent work back

- **WHEN** security review once ruled the run back to implement
- **THEN** the graph shows the return path with the ruling's time and rule

### Requirement: Muninn's last review is readable

The Realms view SHALL show when Muninn last reviewed the fleet and how many
runs and findings it flagged, linking to its proposal. It SHALL NOT run
Muninn.

#### Scenario: No Muninn record

- **WHEN** no Muninn record exists
- **THEN** the panel omits the Muninn line rather than showing an empty one

### Requirement: Live refresh is bounded and resumable

The host SHALL push changes only for the visible realm and the open run. A
reconnecting client SHALL receive a fresh snapshot. Closing the panel SHALL
NOT affect any run.

#### Scenario: Reconnect

- **WHEN** the client reconnects after its socket dropped
- **THEN** it receives the current snapshot, and any open command card is re-validated against the new head

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

The graph SHALL draw the run as `brokkr tui` draws it: the run view's phases left to right on one rail, each phase's steps in order as dots, the members of one step in a bracket above and below the rail, phase names under it, and the current phase boxed in the colour of the run's status. A phase with no recorded steps SHALL be one dot coloured as the TUI's `class_for_phase` colours it, and no state word Brokkr did not give SHALL be shown. A road back SHALL be drawn beneath for each road in a phase's `returns`, solid and in the rail's own colour, with the rules that sent the run back and how many times it was taken. `returns` SHALL name only a ruling whose `next` stands strictly before its `from` on the rail, whose order is first entry: a forward step into a phase entered before, and a phase ruled onto itself, SHALL NOT be a road back. Each road SHALL run square below the phase names, down, across its lane and up, from a departure and to a landing of its own, so no two roads share an arrowhead; roads SHALL nest, the longest outermost. No label SHALL meet any road, and the graph SHALL be tall and wide enough for every label. A label SHALL say as much as its room allows, and the whole sentence on hover.

#### Scenario: Review positions

- **WHEN** a design phase records two positions and then a chief
- **THEN** the positions sit in a bracket either side of the rail, and the rail runs on to the chief

#### Scenario: One reforge, then a forward second pass

- **WHEN** a run goes implement, verify, review, back to implement on REVIEW-REFORGE, forward again through verify and review, and ship is ruled onto itself before it is done
- **THEN** one road is drawn, "sent back to implement by review · REVIEW-REFORGE", and no other phase names a return

#### Scenario: Two roads into one phase

- **WHEN** implement was sent back once by verify (VERIFY-FAIL) and once by review (REVIEW-REFORGE)
- **THEN** two roads land on two arrowheads under implement, the road from review outside the road from verify, and neither crosses the other's label, which reads "by verify · VERIFY-FAIL ×1" or the whole sentence

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

`brokkr inspect --json`'s participants SHALL carry `tokens`: `total` (`3.99M tok`), `input` (`3.98M`), `output` (`14k`) and `cache_hit`, cache reads over input and cache writes to one decimal; each a cell, absent with its reason when the harness reported nothing to read. The view's wire version SHALL be 22.

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

## Provenance

- `2026-10-13-568-operator-console-slice-one` — folded 2026-10-14
- `2026-10-14-568-console-view-tab` — folded 2026-10-14
- `2026-10-19-568-run-view-returns` — folded 2026-10-19
- `2026-10-19-568-run-view-roads` — folded 2026-10-19
