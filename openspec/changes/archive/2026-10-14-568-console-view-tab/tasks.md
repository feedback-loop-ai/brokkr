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
