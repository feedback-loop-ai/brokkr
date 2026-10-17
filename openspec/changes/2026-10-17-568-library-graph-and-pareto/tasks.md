# The Library, slice one — tasks

Status: proposed. Fired by the operator on 2026-10-17. Each task closes on observed evidence.

## 0. Design

- [x] 0.1 The boards: the recipe's state machine with traffic, and the Graph Pareto frontier in four quadrants. Agreed with the operator on 2026-10-17; the in-scope boards are under `ux/`.

## 1. Brokkr side

- [x] 1.1 `brokkr-view`: `GraphView` (D2–D5), covering the states, the rules with counts and kinds, retired rules, `drifted_runs`, per-state figures, ends and live runs. It is built from a resolved policy table and the journal's events, with view version 1. Tests on synthetic journals cover each scenario of `specs/library-graph`. `crates/brokkr-view/src/graph.rs`, `graph/facts.rs`, `graph/figures.rs`, the shared attempt walk `attempts.rs`, a rule's `when` in words `brokkr-core/src/policy/words.rs`; `graph/tests.rs`.
- [x] 1.2 `brokkr-cli`: the verb `brokkr graph --recipe <name> [--realm] [--range] [--json]`. It resolves the table as `recipes show` does, or from a bundle directory named by its `bundle.json`. It prints text and JSON, and refuses an unknown recipe by naming the known ones. CLI tests cover each. `crates/brokkr-cli/src/verbs/graph.rs`, `Cmd::Graph`; `verbs/graph/tests.rs`; `docs/reference/cli.md` regenerated.
- [x] 1.3 `brokkr-view`: analytics `recipes` (D6), covering runs, done, done rate, cost per run with reporting and silent providers, tokens per run and `on_frontier`. The analytics view version moves by one. Tests cover dominance, ties and an unreported cost. `analytics/recipes.rs`, `RecipeRow`, view 23; `analytics/tests.rs`.
- [x] 1.4 The public-API snapshot, measured, for `GraphView` and the analytics `recipes` figure. `quality/public-api/brokkr-view.txt` and `brokkr-core.txt` (`Rule::when_words`), measured with cargo-public-api 0.52.0 on nightly-2026-09-05; `brokkr-cli.txt` unchanged.

## 2. Plugin

- [x] 2.1 Host: `/api/graph?realm&recipe&range` runs `brokkr graph --json` with the runner's bounds. The analytics route passes the new version through. Tests.
- [x] 2.2 Client: the Library tab beside Analytics in both mounts, with the realm picker, the range, and Graph and Pareto (D7). Tests.
- [x] 2.3 Client: the Graph view (D3, D5), covering states with seats, edges by kind with width by count, dashed dormant rules, the stop tone, the pulse on live states, the picked state's panel, and retired rules with the drift note. Tests.
- [x] 2.4 Client: the Pareto view (D6), covering the four named quadrants at the targets, the boundaries going to the better quadrant, the frontier step line, recipes not plotted and not measured, and targets kept in storage that fall back when it is refused. Tests.
- [x] 2.5 Light and dark through the console's tokens. The version check refuses unknown versions with the version named.

2.1–2.5 are in `lib/index.js`, `lib/host/brokkr.js`, `lib/host/readouts.js`, `lib/client.js` and `lib/console.css`, with `tests/library.test.js` and `tests/readouts.test.js`. The implement seat could not run node, so `npm test` was run by hand afterwards: 170 of 170 pass, including the polish found on the operator's console (initial state first, the graph fitted to its pane, equal Pareto quadrants).

## 4. Tokens one way everywhere (D9)

- [x] 4.1 `brokkr-view`: `GraphView` state tokens and analytics `tokens_per_run` carry `TokenCells`; `TokenCells::arrows()` and `line()`; tests. `tokens.rs`, `graph/figures.rs`, `analytics/recipes.rs`; `tokens.rs` tests, `graph/tests.rs`, `analytics/tests.rs`. Public-API snapshot measured.
- [x] 4.2 TUI: the seat pane's tokens cell reads `Σ total ↑ in ↓ out · cache hit`; `brokkr graph`'s text the same. `tui/seats.rs`, `verbs/graph.rs`; `tui/tests.rs` `a_seats_tokens_read_with_arrows_and_the_cache_hit`, the graph text tests.
- [x] 4.3 Console: the state panel's Tokens and Cache hit; the Pareto table's Cache hit column; the host validates the cells. `lib/client.js`, `lib/host/readouts.js`; `tests/library.test.js`.

## 5. The graph reworked, the picker, the question boxes (D3, D7, D10)

- [x] 5.1 `brokkr-view`: `GraphView.links`, one per pair of states with its outcomes by result; test. `graph.rs` `Link`, `Outcome`; `graph/figures.rs` `links`; `graph/tests.rs` `the_graph_draws_one_link_per_pair_of_states_with_the_outcomes_that_fired`. Public-API snapshot measured.
- [x] 5.2 Client: links drawn thin with outcome chips, dormant links faint and unlabelled, no chip over a chip or a state; the host validates links. `lib/client.js` `layoutOf`, `StateMachine`, `chipsOf`; `lib/host/readouts.js` `graphLink`; `tests/library.test.js`.
- [x] 5.3 Client: the recipe dropdown. `lib/client.js` `RecipePicker`; `tests/library.test.js`.
- [x] 5.4 The question boxes hidden unless `BROKKR_CONSOLE_ASK=1`. `lib/host/config.js`, `lib/host/poller.js`, `lib/client.js`; `tests/client.test.js`, `tests/poller.test.js`.

## 3. Docs and observation

- [x] 3.1 The guide, the status row and `brokkr graph` in the CLI reference. `docs/guides/dsh-console.md`, `docs/status.md`, `docs/reference/cli.md`.
- [x] 3.2 The console loaded into DSH against the real journals, read-only, with both views observed and screenshots taken; the brokkr journal byte-identical before and after. `evidence/dsh-0.2.0-rc.2/console/`. It ran in the operator's own DSH web rather than an isolated one, with nothing asked of a model and no command sent; the README says so.
