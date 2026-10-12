# Design: DSH operator console, slice one

Status: proposed. UX source: the canvas page **Step 1 · feasible today**.

## Context

Feasibility pass against `main` (`67a1886e`) and DSH `0.2.0-rc.2`:

- **Read data exists:**
  - `brokkr runs` / `tui` read every hearth in `realms.json`.
  - `FleetView` names each run's `Need` (parked, quarantined, stale) and
    already renders fixes as command text (`fleet.rs`).
  - `inspect --json` emits `RunView`: phases, columns, return paths, visits,
    the dashboard path and participants (attempts, served model, cost).
  - `queue list --json` carries each entry's admission verdict.
  - `brokkr muninn list` reads Muninn's proposals.
- **Commands** (`brokkr-core/src/fold.rs`):
  - retry is legal only when the run awaits the operator, and re-runs its
    phase; resume then drives it;
  - stop is legal on any live run;
  - conclude closes a stopped or parked run;
  - appends are fenced (0029).
- **DSH:**
  - a third-party client module registers a root `main` panel and a
    `sidebar.panellist` entry (as `ui-schedule` does);
  - plugin-owned web-server routes are the proven host↔client transport;
  - there is no per-tab badge API;
  - "Files" is a right-sidebar tab kind, not a top-level view.

## Decisions

### D1 — Meaning is derived in brokkr-view; the plugin paints

Admissibility, refusal texts, the "not available yet" reasons, clarify
questions and analytics are derived in `brokkr-view` and emitted by
readouts. The plugin holds no Brokkr policy (decision 0013). In particular,
it never decides whether a command is legal; it shows what the view model
says.

### D2 — Admissible commands are a closed enum with reasons

`RunView.commands` lists `retry {phase}`, `stop`, `resume` and `conclude`.
Each is either `admitted` or `refused {reason}`, decided from the folded
state by the same rules the verbs apply. `RunView.unavailable` lists the
disabled actions with fixed reasons: `dismiss_finding`, `talk_to_reviewer`,
`hold`, `answer_here`. The reason texts are the canvas copy:

- *dismiss_finding:* "Brokkr only lets you dismiss findings once a run has
  finished."
- *talk_to_reviewer:* "Reviews start fresh each time; your note can't reach
  the reviewer yet."
- *hold:* "A parked run already waits for you; it holds nothing else back."
- *answer_here:* "Not yet: Brokkr has no way to pass an answer to a run
  directly."

### D3 — The plugin host runs readouts and verbs, never the journal

The host shells out to `brokkr runs --json`, `inspect --json`,
`queue list --json`, `muninn list`, `analytics --json` and `transcript`,
and to `operator retry|stop`, `resume` and `conclude`. Every call has a
bounded output and a timeout. The host never opens a journal.

### D4 — Commands carry an id and the head they were chosen at

- Each click sends `{command_id, run, command, reason, expected_seq}`.
- The host re-reads the run's head before invoking. If the head moved,
  the outcome is "not sent" with a fresh view.
- A repeated `command_id` returns the first outcome.
- A lost acknowledgement is reconciled by finding the command's
  `operator/commanded` event.
- Retry is followed by resume, as the CLI flow requires.

### D5 — Live refresh is polling, bounded

- The host polls each hearth's fleet every few seconds.
- It re-reads a run only when that run's head moves.
- It pushes to the client over one plugin WebSocket, for the visible realm
  and the open run only.
- A reconnect gets a fresh snapshot.

### D6 — The assistant reads; the operator acts

DSH model tools: `brokkr_runs`, `brokkr_inspect`, `brokkr_transcript` and
`brokkr_queue`. A suggestion renders as the same command card, re-checked at
use. The panels are titled "Ask Brokkr" (all realms) and "Ask about this
run". The names Triage and Dispatcher are not used, because both are Brokkr
mechanisms.

### D7 — Clarify answers go through the spec

When the view model carries clarify questions, the card shows them. "Open
the spec" asks DSH to open the run's worktree spec file in its Files
sidebar. The operator edits the file, then uses Retry clarify. Nothing
writes the answer on the operator's behalf.
