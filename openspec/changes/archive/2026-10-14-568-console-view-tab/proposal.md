# Change: the operator console as a conversation tab, in DSH's theme, with the run graph the boards drew (#568)

Status: proposed. Asked for by the operator on 2026-10-14: "make it a tab and keep the sidebar"; "opening it inside a workspace means contextualizing it"; "the appearance is light no matter what is selected in dsh, I want light / dark options"; "I don't see a selection of the realms, nor a graph". Then, in a second round: "I would like to be able to click on each phase (so it selects all nodes in the graph) and each node", with the TUI's model, tokens, cache hit and cost; questions about the pick first and the run as outer context; the graph live; "the PULSE is our trademark, so we need to have it here as well". Then, in a third round, the run page reworked after the TUI ("the way we have them in the TUI (boxing the graph edges) is better"): the control plane as "pause / resume / close … 3 buttons on the graph" with a reason; the freed space for "the transcript when we click on an edge"; the two chats reworked; tokens with up and down arrows, cache write dropped. The operator chose design D on the canvas (C's panes and seats table, A's full-width graph and arrow reader, B's park reason word for word): "go with D, build it into the console".
Change: `2026-10-14-568-console-view-tab`.
Builds on: slice one (`e91d881f`) and slice two (`f6429ec4`).
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568).

## Why

The step-1 boards put Brokkr beside DSH's own views. Slice one placed it only behind a left-panel entry, on the reading that DSH has no top tabs. DSH `0.2.0-rc.2` does have them: Chat and Trajectory are entries in the `conversation.view` slot list, and a plugin may add one.

Seen in the operator's own DSH (dark theme):

- **The console was always light.** Its colours were fixed, and DSH's dark text leaked into its buttons, so the realm names on the cards read white on white. This is why the operator saw no realms to pick.
- **"Steps" was a row of boxes.** The boards drew a graph: phases left to right, parallel members side by side, arrows, and a dashed road back. `operator-console-read` already asks for "the graph of steps with their members ... and the return paths taken"; the build did not meet it.

## What Changes

- **A Brokkr tab in every conversation.** It sits after Chat (0) and Trajectory (10), at order 20, and the left-panel entry stays.
  - The tab asks the host which realm the conversation's folder belongs to. The host answers from `brokkr realms --json`: a folder inside a realm's folder, or a git worktree of one (read from its `.git` file and `commondir`, never by running git), belongs to it.
  - The tab opens on that realm and says so. A folder in no realm gets every realm, and the tab says that too.
  - In a tab, Ask Brokkr opens its session in the conversation's folder.
- **A realm switcher.** A realm's bar lists every realm, the current one pressed.
- **Light and dark.** Every colour is a `--bk-*` variable. The light set is the boards'; the dark set applies under DSH's `body[data-ds-dark-theme]`, which DSH sets for Dark, and for System on a dark OS. The stylesheet is served `cache-control: no-cache`, so an upgrade is never painted with the old sheet.
- **The graph.** It is drawn from `RunView.phases[].columns[].nodes[]` (now read by the host) and the dashboard's visits, with `returns` as dashed roads back. Each node shows Brokkr's state and served model. The last step of a phase also shows the visit's duration and rule.
- **The narrower tab.** Below 900px the run list, the run and the assistant stack. The tab leaves room for DSH's floating composer (`--dsh-composer-height`).
- **A dated test fixture.** The run-list test dated its runs 2026-10-13 and broke once a day had passed; it now dates them by the clock.

### Second round

- **Picking.** A phase's label and every node are buttons. A phase picks all its nodes and outlines its segment; a node picks itself; again lets go.
- **The seat, as the TUI reads it.** The pick shows each of its seats with Brokkr's cells: model (claimed) and boundary, tokens, cache hit, cost, turns and attempts, effort applied and pinned, and the note of any absent cell. The host now reads `RunView.participants`; a node's `key` is its participant's.
- **Cache hit, in Brokkr.** `brokkr-view`'s participant gains `cache_hit_cell`, cache reads over input to one decimal (`99.9%`), absent with its reason when either is unreported or input is zero. The view's wire version moves 20 → 21; the console paints 21 only.
- **Context, closest first.** The assistant panel names the pick, then the run, then the realm, and the prompt opens with the pick (a seat by its participant key, for `brokkr_transcript`) with the run as outer context. With no run picked, it asks about the realm.
- **Live.** The open run's view was already re-read and pushed whenever its head moved; a pick now outlives each refresh, and is dropped when the view no longer holds it.
- **The pulse.** ui.html's `forge-strike` and `forge-halo`: active nodes of a driven run strike and throw a halo; the console's mark and the left-panel icon strike while any run is running. Reduced motion stills them.
- **The road back** is solid, in the rail's colour, as ui.html draws a road that was taken (decision 0022's grammar), not dashed.
- **The first landing's findings** (`landing-the-dsh-operator-console-3ca873fd`, residual, low): the one-hearth fallback only without `realms.json`; a refused lookup shown, not painted as no realm; no state words Brokkr did not give; `columns` required; one query parse; the badge's colours in the sheet.

## Impact

- `extensions/dsh/brokkr-console/`:
  - `lib/client.js`, `lib/console.css`, `package.json` (client inject adds `dsh-client-ui-conversation`);
  - the host: `lib/index.js` (route `GET /brokkr-console/api/here`, fenced like every route), `lib/host/here.js`, `lib/host/brokkr.js` (`realms`), `lib/host/readouts.js` (`realmsOf`; phases carry `strategy` and `columns`);
  - tests: `tests/here.test.js`, `tests/tab.test.js`, `tests/fence.test.js`, `tests/client.test.js`.
- Docs: `docs/guides/dsh-console.md`, `docs/status.md`.
- Spec: `operator-console-read` (modified and added requirements).

### Third round: the run page as design D

- **The rail.** The graph is `brokkr tui`'s rail: steps as dots on one line, a step's members in a bracket above and below it, phase names under the rail, the current phase boxed in the run status's colour, roads back beneath. Phases, steps and arrows between phases can be picked.
- **The control plane on the graph.** Stop · Resume · Close, and Retry when admitted; a refused command is disabled with Brokkr's reason (Brokkr has no pause). A press opens a reason bar. The decision card is gone; "Not available yet" folds under the buttons.
- **Why it waits** beside **the reader**, the same height: the park reason word for word, the clarify questions, the stopping rule or the quarantine; the reader shows the picked arrow (its ruling and the seat behind it), step or phase, or where the run is now, with the seat's facts and its transcript.
- **Transcripts.** A fenced `GET /brokkr-console/api/transcript?realm&run&seat` runs `brokkr transcript --json` against the listing's journal, bounded to the last 80 turns; the reader reads it again when the head moves. A seat whose view records no transcript says so and asks nothing.
- **Tokens read apart, in Brokkr.** `Participant.tokens` (`total`, `input`, `output`, `cache_hit`) replaces the unreleased `cache_hit_cell`; one `usage_cells` helper builds it with the usage cell, keeping `participants` at 150 lines. View 21.
- **The seats table** compares every seat (Σ ↑ ↓, cache hit, cost, model); a picked phase narrows it; ▸ marks the seat behind the pick.
- **One chat.** The right-hand panels are gone. One "Asking about" bar names the pick, the run and the realm; in a conversation's tab it puts that context in the conversation's own message box, unsent; in the left panel it opens a session.
- **Rulings are read structurally.** Journal rows carry `ruling` (`from`, `next`, `rule`, `result`, `severity`) from the payload Brokkr wrote, so an arrow finds its ruling without parsing words.
- **The second landing's findings** (`landing-the-dsh-operator-console-3a87d008`, residual, low): the token cells are bound at view level; the console's pulse keyframes are held to ui.html's by a test; a plain current phase is coloured by the run's status as the TUI's `class_for_phase` does; `participants` is back at its baseline; the stale assistant comment is gone.
