# The console as a conversation tab: tasks

Status: proposed. Asked for by the operator on 2026-10-14.

## 1. Host

- [x] 1.1 `brokkr realms --json` read as `realmsOf`; `brokkr.realms()`. `lib/host/readouts.js`, `lib/host/brokkr.js`.
- [x] 1.2 `hereOf`: a folder's realm, by real path or git worktree link, the one hearth with no map (D1); tests. `lib/host/here.js`; `tests/here.test.js`.
- [x] 1.3 `GET /brokkr-console/api/here`, fenced; the stylesheet served `no-cache`; tests. `lib/index.js`; `tests/fence.test.js`, `tests/here.test.js`.
- [x] 1.4 Phases carry `strategy` and `columns` for the graph. `lib/host/readouts.js`.

## 2. Client

- [x] 2.1 The `conversation.view` entry at order 20 beside the left-panel entry; the tab opens on the conversation's realm and says so (D2); tests. `lib/client.js` `Tab`, `useFolder`; `tests/tab.test.js`.
- [x] 2.2 The realm switcher, and the conversation's realm marked on its card; tests. `lib/client.js` `Realm`, `RealmCard`; `tests/tab.test.js`.
- [x] 2.3 The graph from phases, columns and roads back (D4); tests. `lib/client.js` `layoutOf`, `Graph`; `tests/tab.test.js`.
- [x] 2.4 Light and dark sets keyed to DSH's mark; no colour outside them (D3); the narrow layout; room for the composer; tests. `lib/console.css`; `tests/tab.test.js`.
- [x] 2.5 The run-list test dates its runs by the clock. `tests/client.test.js`.

## 3. Evidence and docs

- [x] 3.1 Observed in a browser against an isolated DSH: the tab, the realm, the graph, Dark, the narrow layout (design, Observed).
- [x] 3.2 `docs/guides/dsh-console.md`, `docs/status.md`.

## 4. Second round

- [x] 4.1 `brokkr-view`: `Participant.cache_hit_cell`; view wire 22 (21 at the time; #511 took 21 on main); tests; the public-API snapshot, measured with cargo-public-api 0.52.0. `crates/brokkr-view/src/lib.rs` `cache_hit_text`; `tests.rs`; `quality/public-api/brokkr-view.txt`.
- [x] 4.2 Host: participants and node keys read; view 22 (21 at the time). `lib/host/readouts.js`.
- [x] 4.3 Client: picking phases and nodes, the picked seats, the context chain and prompt (D5–D7); tests. `lib/client.js` `Graph`, `focusOf`, `Picked`, `SeatFacts`, `Realm`, `Ask`; `tests/pick.test.js`.
- [x] 4.4 The pulse and the mark (D8); the solid road back; tests. `lib/client.js` `dot`, `Mark`, `Icon`; `lib/console.css`; `tests/pick.test.js`.
- [x] 4.5 The first landing's findings; tests. `lib/host/here.js`, `lib/index.js`, `lib/client.js`; `tests/here.test.js`, `tests/tab.test.js`.

## 5. Third round: design D

- [x] 5.1 `brokkr-view`: `Participant.tokens` (`TokenCells`) from one `usage_cells` helper, replacing `cache_hit_cell`; `fmt_count`; view-level test; the public-API snapshot, measured. `crates/brokkr-view/src/lib.rs`; `src/tests/cache_hit.rs`; `quality/public-api/brokkr-view.txt`.
- [x] 5.2 Host: token cells and `transcript_cell` read; journal rows carry their ruling; `brokkr transcript` and the fenced transcript route; tests. `lib/host/readouts.js`, `brokkr.js`, `lib/index.js`; `tests/readouts.test.js`, `tests/fence.test.js`.
- [x] 5.3 Client: the rail (D9, D10), the control plane on the graph (D11), why it waits, the reader and transcripts (D13), the seats table, the Asking about bar (D12); tests. `lib/client.js`; `tests/pick.test.js`, `tests/client.test.js`, `tests/assistant.test.js`, `tests/tab.test.js`.
- [x] 5.4 The stylesheet for the rail and the panes, in both palettes. `lib/console.css`.
- [x] 5.5 Specs `operator-console-read`, `-commands`, `-assistant`; the guide and status.
- [x] 5.6 Observed in a browser (design, Observed, third round).

## 6. Fourth round

- [x] 6.1 The tab scrolls in DSH's scroller; a seat with no transcript answers in Brokkr's words; refusals drop Brokkr's notes; tests. `lib/console.css`, `lib/host/brokkr.js`, `lib/host/readouts.js`; `tests/transcript-refusal.test.js`.
- [x] 6.2 The realms pane and a realm's runs pane; each view opens at its top; tests. `lib/client.js` `RealmRow`, `RunList`, `Page`; `tests/tab.test.js`.
- [x] 6.3 The conversation's context through `POST /brokkr-console/api/focus` and an `agent/pre-step` snapshot; tests; observed against a stub model. `lib/host/focus.js`, `lib/index.js`, `lib/client.js` `AskBar`; `tests/focus.test.js`, `tests/assistant.test.js`.
- [x] 6.4 A seat's selected model and provider. `lib/host/readouts.js`, `lib/client.js` `selectedOf`.
- [x] 6.5 The third landing's findings. Commit 7101e9a9.
