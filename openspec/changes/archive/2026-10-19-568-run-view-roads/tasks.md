# The roads back are square, nested and counted: tasks

Status: proposed. Asked for by the operator on 2026-10-19.

## 1. Brokkr

- [x] 1.1 `brokkr-view`: `Road {from, taken, rules}` in `Phase.returns`; tests for counts, a road taken under several rules, and two roads into one phase. `crates/brokkr-view/src/lib.rs`, `src/tests/returns.rs`, `src/tests.rs`.
- [x] 1.2 The TUI and `brokkr ui` read `road.from`. `crates/brokkr-cli/src/tui/layout.rs`, `src/ui.html`; `tui/tests.rs`, `ui/tests.rs`.
- [x] 1.3 The public-API snapshot, measured with cargo-public-api on the pinned nightly. `quality/public-api/brokkr-view.txt`.

## 2. Console

- [x] 2.1 Roads read as `{from, taken, rules}`. `lib/host/readouts.js`; `tests/support/world.js`.
- [x] 2.2 Square, nested roads from their own slots, labels clear of every road, counted; tests for one, two and three roads, two into one phase. `lib/client.js`; `tests/pick.test.js`.

- [x] 2.3 The last phase's name and NOW box count in the rail's width. `lib/client.js`.

## 3. Docs

- [x] 3.1 `docs/guides/dsh-console.md`, `docs/guides/read-surfaces.md`.
