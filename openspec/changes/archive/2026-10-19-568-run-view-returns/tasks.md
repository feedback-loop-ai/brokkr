# A run with returns reads plainly: tasks

Status: proposed. Asked for by the operator on 2026-10-19.

## 1. Brokkr

- [x] 1.1 `brokkr-view`: a return lands strictly earlier on the rail than its departure, never on itself; tests for one reforge with a forward second pass and ship twice, two different returns, and a retry. `crates/brokkr-view/src/lib.rs`, `src/tests/returns.rs`.
- [x] 1.2 `brokkr-view`: `Participant.visit`, `VIEW_VERSION` 24; tests. `crates/brokkr-view/src/participant.rs`, `src/lib.rs`, `src/tests/returns.rs`.
- [x] 1.3 The public-API snapshot, measured with cargo-public-api on the pinned nightly. `quality/public-api/brokkr-view.txt`.

## 2. Console

- [x] 2.1 The run view read at wire 24, with each seat's `visit`. `lib/host/readouts.js`; `tests/readouts.test.js`, `tests/support/world.js`.
- [x] 2.2 Each road back in a lane of its own below the phase names, labels a line apart, the rail sized to fit; tests. `lib/client.js`, `lib/console.css`; `tests/pick.test.js`.
- [x] 2.3 A later pass's seat says its visit; the run's bar says its realm's running count; tests. `lib/client.js`, `lib/console.css`; `tests/pick.test.js`.

## 3. Docs

- [x] 3.1 `docs/guides/dsh-console.md`, `docs/guides/read-surfaces.md`.
