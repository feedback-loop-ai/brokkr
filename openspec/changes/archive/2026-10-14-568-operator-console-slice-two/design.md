# Design: DSH operator console, slice two

Status: proposed. The DSH facts come from `evidence/dsh-0.2.0-rc.2/README.md`, measured on 2026-10-14 against a loopback stub model.

## Context

- **Tools.** `ctx.tools.register(def)` (host half, `inject: ['tools']`) registers globally. The `standard` preset's sessions see the tools, and the model calls them with no approval prompt. Third-party packages cannot import `defineTool`, so a definition is a plain object, and the plugin validates its own arguments.
- **Sessions.**
  - A session opened with `workspaces.create({path})` → `uiWorkspace.openWorkspace(id, beforeOpen)` gets a working composer.
  - `conversation.input.for(sessions.scope(sid))` gives `setDraft(text)` and `submit()`.
  - A session with no workspace for its directory has an inert composer.
- **Cards.** The keyed slot `tool.call.toolview` draws a plugin tool's calls with the call's arguments and `block.meta`. It persists across reloads, and DSH folds it under "Called tools" once a turn ends. A card is the only transcript UI a plugin can add.
- **Files.** The Files tab is rooted at the session's cwd. `sidebarRight.openTab('files')` and `openResource('dsh-resource://<file>/session/<sid>/<path>')` work once a session is on screen. File reads are not contained to the session; that is DSH's property.
- **Recipe.** Every `run/started` manifest records `bundle_name`, and `runs.bundle_name` holds it too. No readout carries it yet.

## Decisions

### D1 — The recipe is read, not configured

`RunView.recipe` is the `bundle_name` of the run's first event, when that event is `run/started` and names one. The console resumes with `--recipe <recipe>`, resolved by `brokkr resume` against the workspace's recipes, as the operator would type it. `BROKKR_CONSOLE_RESUME_BUNDLE`, then `_RECIPE`, still override. With neither, and no recipe in the view, retry and resume are not sent, as before.

### D2 — Tools are Brokkr's readouts, bounded, read-only

| Tool | Runs | Arguments |
|---|---|---|
| `brokkr_runs` | `runs --json` | optional `realm` |
| `brokkr_inspect` | `inspect --json` | `run` |
| `brokkr_queue` | `queue list --json` | none |
| `brokkr_transcript` | `transcript --json` | `run`, `seat`, optional `turn` |

- **Argument checks:** each tool checks its arguments against the same id grammar the command desk uses. A run id or seat outside it is refused before a process starts.
- **Bounds:** each answer is the readout's JSON, cut at the runner's byte bound. A cut is said, not hidden.
- **Read-only:** no tool runs a verb that writes. The tool list is the whole surface.

### D3 — A suggestion is re-checked twice and pressed once

- **Admission.** `brokkr_suggest {run, command, reason}` reads the run's view now. It answers with a card only when that command is admitted, naming the head seq it was read at. Otherwise it answers with the refusal, or the unavailable reason, in Brokkr's words, and no card.
- **The card** says "Suggested · <command label>", the run's title, the reason, "only if the run hasn't changed since you read this", and **Use this**.
- **Use this** stores the suggestion in the browser store, keyed by run. The Brokkr panel's decision card for that run then shows it selected, with the reason filled in and the seq the suggestion was read at. Pressing goes through slice one's desk, which refuses a moved head as "not sent". The model can call `brokkr_suggest`, but nothing it can call appends to a journal.

### D4 — Asking opens a logged session with the tools

- **Opening.** Ask Brokkr and Ask about this run each register a DSH workspace for a directory, then open it with a prefilled draft and send it:
  - Ask Brokkr uses the console's workspace;
  - Ask about this run uses the run's worktree, or the console's workspace when the view names none.
- **The draft** names the scope (all realms, or one run by title, realm and id) and lists the tools. It asks the model to cite the record each fact came from, and to say what is not recorded instead of guessing.
- **Why not inline.** The measured inline call (`llm.stream`) is not used: it is unlogged and has no tools, and slice one's spec asks for answers that cite records.

### D5 — "Open the spec" opens the worktree, not a guessed file

The clarify readout names the worktree and the questions, but not a spec file. "Open the spec":
1. opens a session in the worktree, as in D4, with a draft listing the questions and "answer these in the spec, then Retry clarify" (not sent);
2. once the session is on screen, opens the Files tab.

Without a worktree in the view, the button is disabled with "no worktree recorded for this run". Recording the spec's path is a follow-up.

### D6 — The plugin's tests gate in CI

A `plugin tests` job installs the Node version pinned by `.github/workflows/research-weekly.yml`'s `actions/setup-node` and runs `npm test` in `extensions/dsh/brokkr-console/`, on `ubuntu-latest` and `macos-latest`. Whether it becomes a required check is the operator's ruling in branch protection.
