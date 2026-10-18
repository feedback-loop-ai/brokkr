# The Library says when a recipe does not compile: tasks

Status: proposed. Asked for by the operator on 2026-10-18.

## 1. Brokkr

- [x] 1.1 `brokkr-view`: `GraphResolution`, `Table::Refused`, `does_not_compile`, `GRAPH_VIEW_VERSION` 2; tests. `crates/brokkr-view/src/graph.rs`, `graph/figures.rs`, `graph/tests.rs`.
- [x] 1.2 `brokkr graph` compiles the realm's shelf; a compile refusal is a refused table and a name in `does_not_compile`; any other failure still fails; the text form prints it; tests. `crates/brokkr-cli/src/verbs/graph.rs`, `graph/tests.rs`.
- [x] 1.3 The public-API snapshot, measured with cargo-public-api 0.52.0 on the pinned nightly. `quality/public-api/brokkr-view.txt`.

## 2. Console

- [x] 2.1 The graph read at wire 2, with the refused table and `does_not_compile`; tests. `lib/host/readouts.js`; `tests/readouts.test.js`.
- [x] 2.2 The refusal panel, the marked picker, and the realm's recipes kept for the picker; tests. `lib/client.js`, `lib/console.css`; `tests/library.test.js`.

## 3. Docs

- [x] 3.1 `docs/guides/dsh-console.md`.
