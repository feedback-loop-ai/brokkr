# DSH operator console, slice one — tasks

Status: proposed. Fired by the operator on 2026-10-13. Each task closes on
observed evidence. The UX source is the canvas page **Step 1 · feasible
today**.

## 0. Prerequisites

- [x] 0.1 Author a proposed decision: the console plugin lives under `extensions/dsh/brokkr-console/` with its own source and tests, outside every hashed runtime set. It is never built, loaded or executed by a Brokkr crate, and never published.
- [x] 0.2 Measure on DSH `0.2.0-rc.2` that a third-party client module registers a `main` panel and a `sidebar.panellist` entry, and that plugin web-server routes and a WebSocket upgrade reach it. Record the evidence beside the provenance note. Observed 2026-10-13, all four points, in `evidence/dsh-0.2.0-rc.2/`. Plugin HTTP routes and WebSocket upgrades are open by default: every route the plugin registers SHALL refuse through `ctx.connection.requestRejection(req)` first, and the upgrade fence is a test the plugin owns, unmeasured here.

## 1. Brokkr side: commands in the view model (operator-console-commands)

- [ ] 1.1 `brokkr-view`: a closed enum of operator commands, each admitted or refused with the verb's reason, derived from the folded state.
- [ ] 1.2 `brokkr-view`: unavailable actions, with the fixed reasons from design D2.
- [ ] 1.3 `inspect --json` emits both. Tests per scenario, under the exact coverage gate.

## 2. Brokkr side: clarify questions and worktree (operator-console-read)

- [ ] 2.1 `brokkr-view`: carry a clarify-exhausted park's open questions from the clarifier seat's result, plus the run's worktree path.
- [ ] 2.2 Tests: present for a clarify-exhausted park, absent otherwise.

## 3. Brokkr side: analytics (operator-console-analytics)

- [ ] 3.1 `brokkr-view`: a pure cross-run aggregation, given events, hearths, range, optional realm and `now`.
- [ ] 3.2 `brokkr-cli`: an `analytics --json` readout over every hearth in `realms.json`.
- [ ] 3.3 Tests per metric and per scenario, including "not reported" cost.

## 4. DSH side: plugin (`extensions/dsh/brokkr-console/`)

- [ ] 4.1 Host: readout and verb runner with bounded output and timeouts; the head check, command ids and reconciliation (design D4); polling and push (D5).
- [ ] 4.2 Client: the Brokkr panel-list entry with the waiting count in its icon; the Realms view (board S1Realms).
- [ ] 4.3 Client: the run view and decision card (S1Run), the other decision cards and outcomes (S1Decisions).
- [ ] 4.4 Client: the Analytics view (S1Analytics).
- [ ] 4.5 Assistant: read tools, the two panels, suggestion cards (operator-console-assistant).
- [ ] 4.6 Plugin tests for the host's fencing and idempotency, and a provenance note pinning DSH `0.2.0-rc.2`.

## 5. Docs

- [ ] 5.1 A guide entry for installing the console into a DSH profile, and a `docs/status.md` row saying what it does and does not do.

## Follow-ups (separate changes)

- A decision on passing operator guidance to seats on retry or resume. This unlocks answering in the console and a weaker "talk to the reviewer".
- An amendment to decision 0047 for dismissing findings on parked runs.
- Holding a parked run.
- The dispatch loop (#430), then queue controls.
- Recording capacity and host.
- `--expect-seq` on the verbs.
- Multi-operator authority.
