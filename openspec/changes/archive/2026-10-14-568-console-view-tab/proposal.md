# Change: the operator console as a conversation tab, in DSH's theme, with the run graph the boards drew (#568)

Status: proposed. Asked for by the operator on 2026-10-14: "make it a tab and keep the sidebar"; "opening it inside a workspace means contextualizing it"; "the appearance is light no matter what is selected in dsh, I want light / dark options"; "I don't see a selection of the realms, nor a graph".
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

No Brokkr (Rust) change. The view stays at version 20.

## Impact

- `extensions/dsh/brokkr-console/`:
  - `lib/client.js`, `lib/console.css`, `package.json` (client inject adds `dsh-client-ui-conversation`);
  - the host: `lib/index.js` (route `GET /brokkr-console/api/here`, fenced like every route), `lib/host/here.js`, `lib/host/brokkr.js` (`realms`), `lib/host/readouts.js` (`realmsOf`; phases carry `strategy` and `columns`);
  - tests: `tests/here.test.js`, `tests/tab.test.js`, `tests/fence.test.js`, `tests/client.test.js`.
- Docs: `docs/guides/dsh-console.md`, `docs/status.md`.
- Spec: `operator-console-read` (modified and added requirements).
