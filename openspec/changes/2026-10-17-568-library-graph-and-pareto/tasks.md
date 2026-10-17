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

- [ ] 2.1 Host: `/api/graph?realm&recipe&range` runs `brokkr graph --json` with the runner's bounds. The analytics route passes the new version through. Tests.
- [ ] 2.2 Client: the Library tab beside Analytics in both mounts, with the realm picker, the range, and Graph and Pareto (D7). Tests.
- [ ] 2.3 Client: the Graph view (D3, D5), covering states with seats, edges by kind with width by count, dashed dormant rules, the stop tone, the pulse on live states, the picked state's panel, and retired rules with the drift note. Tests.
- [ ] 2.4 Client: the Pareto view (D6), covering the four named quadrants at the targets, the boundaries going to the better quadrant, the frontier step line, recipes not plotted and not measured, and targets kept in storage that fall back when it is refused. Tests.
- [ ] 2.5 Light and dark through the console's tokens. The version check refuses unknown versions with the version named.

2.1–2.5 are written (`lib/index.js`, `lib/host/brokkr.js`, `lib/host/readouts.js`, `lib/client.js`, `lib/console.css`; `tests/library.test.js`, `tests/readouts.test.js`) and stay open until `npm test` has run: the implement seat could not run node.

## 3. Docs and observation

- [x] 3.1 The guide, the status row and `brokkr graph` in the CLI reference. `docs/guides/dsh-console.md`, `docs/status.md`, `docs/reference/cli.md`.
- [ ] 3.2 The console loaded into an isolated DSH against a real journal, read-only, with both views observed and screenshots taken. The journal must be byte-identical before and after. Evidence goes under `evidence/`.
