# Change: the Library says when a recipe does not compile, and lists every recipe of a realm (#568)

Status: proposed. Asked for by the operator on 2026-10-18, after the Library drew a graph only for the brokkr and swarmail realms. lanetally, looper and xbox-inference showed "The graph could not be read: error: bundle: adapter 'claude' (…/adapters/claude.json) has unknown key 'binding_grant'; known keys: …", or a "malformed phase machine table" refusal. The operator took it for "no runs yet".
Change: `2026-10-18-568-library-compile-refusal`.
Builds on: the Library's slice one, `2026-10-17-568-library-graph-and-pareto` (`78d980c7`).
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568).

## Why

`brokkr graph` compiled only the recipe it drew, and a compile refusal failed the whole verb (exit 1). Those realms' recipe libraries predate Brokkr's contract changes, so their recipes do not compile against this Brokkr. Two defects followed in the console:

- **The refusal was a raw compiler line.** Nothing said that the realm's recipes do not compile against this Brokkr. The line carried `error: bundle: bundle:` and ran as one long line.
- **The picker emptied.** With no graph, the recipe picker fell back to the recipes with runs. A realm with no runs listed none, so the operator could not pick another recipe.

## What Changes

- **`brokkr graph` compiles every recipe on the realm's shelf.**
  - A recipe that does not compile is drawn rather than failing the verb. Its table is `refused`, carrying Brokkr's words, the compiler's reason without the `bundle: ` words that wrap it, and the bundle directory, named in the realm's folder.
  - As with a table not found, only the rules its runs fired are drawn.
  - `does_not_compile` names every recipe of the realm that does not compile.
  - Any other failure to compile, such as an unreadable realm dialect, still fails the verb.
  - The text form prints the refusal under the heading.
- **`GRAPH_VIEW_VERSION` moves 1 → 2.** The table gains its `refused` kind and the view gains `does_not_compile`, so a console built for 1 refuses 2 and says so.
- **The console's Library:**
  - A refused recipe shows a panel headed with Brokkr's words (`dependencies doesn't compile against this Brokkr`). The compiler's reason sits beneath it, one clause to a line in a wrapping monospace block, so the file it names reads whole. The bundle directory comes last.
  - With no run in range, nothing else is drawn, and a note says there is no traffic to draw either.
  - The picker marks each recipe that does not compile (`· doesn’t compile`) and counts them.
  - The picker keeps the realm's recipes as its last graph named them, so a graph that cannot be read still leaves every recipe to pick.

## Impact

- `crates/brokkr-view`: `GraphResolution` (compiled, not found, refused) replaces `GraphQuery.table`'s `Option<GraphTable>`; `GraphQuery.does_not_compile`; `Table::Refused`; `GraphView.does_not_compile`; `GRAPH_VIEW_VERSION` 2; tests. The public-API snapshot gains these items.
- `crates/brokkr-cli`: `verbs/graph.rs` compiles the shelf (`compiled`, `reason_of`, `within`) and paints the refusal; tests use a fixture adopter whose adapter carries a key this Brokkr does not know.
- `extensions/dsh/brokkr-console/`: `lib/host/readouts.js` (graph wire 2, `refused`, `does_not_compile`), `lib/client.js` (`Refusal`, `RecipePicker`, `Library`), `lib/console.css`; tests.
- Docs: `docs/guides/dsh-console.md`.
- Specs: `library-graph` and `library-tab`, one added requirement each.
