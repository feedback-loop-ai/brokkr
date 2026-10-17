# Change: the console's Library, slice one — the recipe's graph and the Graph Pareto frontier (#568)

Status: proposed. Fired for delivery by the operator on 2026-10-17 ("let's do it").
Change: `2026-10-17-568-library-graph-and-pareto`.
Builds on: `2026-10-14-568-operator-console-slice-two` (archived), the console's Brokkr tab and `brokkr analytics`.
Parent story: [#568](https://github.com/feedback-loop-ai/brokkr/issues/568), part of #330.

## Why

The operator can see each run, and analytics totals across runs. What they cannot see is the **recipe as a graph**:
- which rules of its state machine actually fire, and how often;
- where runs end;
- which rules never fire;
- how one recipe compares with another on cost and on how often a run reaches `done`.

The operator's stated value is exactly this diagnosis: which graph is more efficient, and which part of it to change.

A design round on swarmail's journal (the boards under `ux/`) showed that the recipe's own state machine, drawn with traffic on its edges, makes that legible at a glance. For `self` since 10 October, the figures were:
- 28 runs, 6 reached `done`, 16 ended in `stop` by a rule;
- 11 of 22 rules never fired;
- **there is no repair edge**: every catch at verify or review ends the run, and the operator re-fires a new run from intake.

Recipes also need a common scale. The Graph Pareto frontier plots each recipe's done rate against its cost per run, in four classified quadrants split at the operator's targets.

## What Changes

### Brokkr side (Rust)

- **`brokkr-view` gains `GraphView`**, one recipe's state machine with its traffic over a range. It is derived from the recipe's resolved policy table and the journal; there is no new recording. It holds:
  - the states;
  - every rule with its fire count;
  - per state:
    - runs entered;
    - median time;
    - the seats and the models the latest run pinned;
    - priced cost and tokens;
  - how runs ended:
    - `done`;
    - `stop` by each rule;
    - stopped by the operator;
    - still open;
  - the live runs and their current state;
  - rules that fired under an earlier version of the table and are not in it now.
- **New verb `brokkr graph --recipe <name>`**, with `--realm`, `--range 24h|7d|30d` and `--json`. It is read-only and prints the graph as text (states, rules with counts, ends). The JSON is `GraphView` verbatim.
- **`brokkr analytics --json` gains `recipes`**: one figure per realm and recipe:
  - runs;
  - runs that reached `done`;
  - done rate;
  - priced cost per run;
  - tokens per run;
  - the providers that reported no cost.

  The analytics view version moves by one.

### DSH side (plugin, `extensions/dsh/brokkr-console/`)

- **A Library tab beside Analytics**, with a realm picker, a recipe picker and a range. It is read-only.
- **Graph view:**
  - the recipe's states as boxes, holding their seats and models;
  - its rules as edges:
    - a fired rule is solid, its width and label from its count;
    - a rule that never fired is dashed;
    - edges into `stop` are drawn in the stop tone;
  - the live run's state carries the pulse;
  - clicking a state shows its rules with counts and its figures;
  - retired rules are listed under the graph.
- **Pareto view:** each recipe in the realm is a point, done rate against cost per run, on four quadrants:
  - **Pareto frontier quadrant**: cheap and reliable;
  - **Reliable but costly**;
  - **Cheap but leaky**;
  - **Costly and leaky**.

  The quadrants split at two targets the operator sets, kept in the browser, defaulting to 50% done and $3.00 a run. A recipe with no runs in range is listed, not plotted. The line that marks the non-dominated recipes is drawn from the figures.
- Light and dark follow DSH's theme, as the rest of the console does.

## Impact

- **Wire:** the analytics view version moves by one, and `GraphView` has its own version starting at 1. The console accepts only the new analytics version.
- **Public API:** brokkr-view's grows by `GraphView` and the analytics `recipes` figure. The PR names the raised snapshot with a `Ruling:` line citing this change.
- **No writes:** the console stays read-only (decision 0014). The only new state is the two targets, in the viewer's browser.
- **Not in this slice** (each needs recording that does not exist yet):
  - **escapes**, meaning which earlier check missed what a later one caught; this needs a map of which check owns which kind of defect;
  - **the bench**, isolated runs of one seat or slice on recorded cases; this needs each seat's work order kept by its digest;
  - **editing charters and recipes**; this needs write verbs and a decision;
  - **the cast board and the charter studio**.
