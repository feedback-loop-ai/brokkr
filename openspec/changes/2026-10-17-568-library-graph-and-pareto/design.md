# Design: the Library, slice one — the graph and the Graph Pareto frontier

## Context

The console draws what Brokkr reads; it derives no Brokkr meaning (decision 0014, slice one's D1). A recipe's state machine is its resolved policy table (`policy.json` through `extends`, decision 0017). Its traffic is in the journal: `phase/entered`, `transition/decided` with the rule id, the terminal events, and the checkpoints.

A run's manifest pins `policy.json` by digest in `files`, but not the table itself. So the graph is drawn from the recipe as it resolves now, and the counts come from the runs that carry the recipe's name.

The boards under `ux/` fix the layout and the copy:
- `L1Graph.dc.html` is the graph. Its red escape markings are out of scope; see D8.
- `L1Pareto.dc.html` is the Graph Pareto frontier.

## D1 — Brokkr computes, the console draws

Every figure comes from Brokkr:
- `GraphView` comes from the new verb `brokkr graph --recipe <name> --json`;
- the per-recipe figures come from `brokkr analytics --json`, under `recipes`.

The console adds no counts, medians or costs of its own. The only judgement the console makes is the quadrant a point falls in, because the targets are the viewer's (D6).

## D2 — Which table, and drift

The table is resolved the way `brokkr recipes show` resolves `<name>` in the realm:
1. its recipe shelf;
2. else a bundle directory under the realm whose `bundle.json` `name` equals `<name>`, as `self` is in an adopter.

Each run's pinned `policy.json` digest is compared with the resolved one.
- **Counts are matched by rule id** across both.
- **A rule id that fired but is not in the table** is listed as **retired**, with its count.
- **When the pinned digest differs**, `GraphView` says so: "N runs ran under an earlier version of this table".
- **When the recipe cannot be resolved here**, the view carries only the fired rules, with "this recipe's table was not found in <realm>", and no dormant edges.

## D3 — Layout

The layout is deterministic and owned by the view:
- **States** are ranked with the table's initial state first, then in `phases` order, left to right.
- **Terminals:** `done` sits at the right end; `stop`, and any terminal other than the last, sits beneath the row.
- **Self-loops** sit above their state.
- **A rule whose `next` is earlier in the order** is a back edge, drawn above the row.
- **A rule to a terminal beneath** is drawn down to it.

The view emits, per rule:
- `from`, `next`, `result`;
- `when`, as Brokkr's words;
- `severity`;
- `count`;
- `kind`: `forward`, `self`, `back` or `terminal`.

The view also emits `links`: the rules joining one state to one next state, as one link per pair, in the order its first rule holds. Each link carries:
- `count`, the rules' counts summed, and `rules`, how many rules it joins;
- `outcomes`, one per result that fired, its rules' counts summed, the most runs first (`pass 21`, `residual 12`);
- `text`, the outcomes joined, empty when none fired.

The graph draws links, not rules. The operator saw one edge per rule as clutter: three near-parallel curves from review into `stop`, a label per dormant rule, rule ids where outcomes belong.
- **Width:** a fired link runs from 1.5 to 4 px by the square root of its share of the busiest link. A link whose rules never fired is 1.25 px, dashed and faint, with no label.
- **Outcomes** are chips on their own ground, Brokkr's `<result> <count>`, in the stop tone into `stop` and the done tone into `done`.
- **Placement:** a gap between neighbours is as wide as the widest chips any neighbour link carries. Fired arcs take the inner lanes. Chips into a terminal sit just under their state. No chip overlaps another or a state.
- **Rule ids** are each link's title, and the state panel lists every rule with its count, dormant ones included.

## D4 — Per-state figures

All figures are over the range.
- **`entered`:** distinct runs that entered the state.
- **`median_millis` and `p90_millis`:** time from a `phase/entered` to the run's next `phase/entered` or terminal event.
- **`cost`:** the sum of reported `total_cost_usd` over the state's effects, as analytics' cost reads it. It carries the providers that reported and the ones that did not.
- **`tokens`:** summed, then read apart in the shared token figure (`TokenCells`): total, input, output and the cache hit. Every surface shows them the same way: `Σ total ↑ in ↓ out` with the cache hit beside them, as the run view's seats already do (D9).
- **`seats`:** from the latest run's manifest `agents`. Each is a seat key (`design:positions:robustness`) with the provider and model it pinned, in human names (decision 0016).

## D5 — Ends and live runs

`ends` counts runs by how they ended in range:
- `done`;
- `stop` by each rule id;
- `operator` (an operator stop);
- `open`.

`live` lists each open run's id, its human label as the fleet shows it, and its current state. The client draws the pulse on that state, the console's mark for a forging run.

## D6 — The Graph Pareto frontier

**Per recipe.** `analytics --json` `recipes[]` carries, per realm and recipe over the range:
- `runs` and `done`;
- `done_rate`, as a percent;
- `cost_per_run`, the reported cost divided by runs, with `reporting` and `silent` providers;
- `tokens_per_run`, the shared token figure averaged over the runs (D9);
- `on_frontier`: true when no other recipe of the same realm with runs has a done rate at least as high and a cost per run at most as low, with one of the two strictly better.

**What is plotted.** A recipe whose cost is wholly unreported has `cost_per_run: null`. It is listed under the plot as "cost not reported", not plotted.

**Quadrants.** They split at two viewer targets: done rate (default 50%) and cost per run (default $3.00). Both are kept in the browser's storage for this console, and both are editable on the view. The four quadrants are:
- top-left, **Pareto frontier quadrant**, "cheap and reliable";
- top-right, **Reliable but costly**;
- bottom-left, **Cheap but leaky**;
- bottom-right, **Costly and leaky**.

**The frontier line.** It joins the `on_frontier` points as a step line. Recipes the realm lists but which have no runs in range are named under the plot as "not measured yet".

## D7 — Where it lives

**The tab.** The Library is a top-level tab of the console, next to Realms and Analytics, in the sidebar page and in the conversation tab alike. In a conversation's tab it opens on the conversation's realm.

**The views.** It has two views, **Graph** and **Pareto**, under one realm picker and one range. Graph also has a recipe picker: one dropdown however many recipes a realm keeps. The recipes that ran in range come first, with their runs as Brokkr counts them, most first. The rest follow, by name.

**Navigation.**
- Clicking a recipe's point in Pareto opens its Graph.
- Clicking a live run's pulse opens the run in Realms, as a fleet row does.

## D8 — Not in this slice

These are drawn on the boards but need recording that does not exist yet:
- **escapes**: which earlier check missed what a later one caught;
- **the bench**: isolated runs of one seat on recorded cases;
- **editing**: write verbs, which need a decision;
- **the cast board and the charter studio**.

The graph draws no escape markings and no bench actions in this slice.

## D10 — The console's question boxes are hidden

The operator found the assistant surfaces from slice two (Ask Brokkr, Ask about this run) not yet a good experience, and asked for them hidden until they are thought through again. They are off unless `BROKKR_CONSOLE_ASK=1` is set where DSH starts. The host passes the setting to the client with the workspace. Nothing else changes: the model tools and the suggestion card stay as they were.

## D9 — Tokens read one way on every surface

The operator asked for one token reading everywhere. The run view's seat
row already gives it: `Σ total ↑ in ↓ out`, with the cache hit, the share
of input served from cache, in its own column.

- **Brokkr:** `TokenCells` is the one figure, worked out in one place.
  - `GraphView`'s state tokens and analytics' `tokens_per_run` carry it.
  - `TokenCells::arrows()` and `TokenCells::line()` give the text for a
    surface with one cell.
- **The console:** the Library's state panel shows Tokens and Cache hit
  as two facts. The Pareto table gains a Cache hit column.
- **The TUI:** the seat pane's tokens cell reads `Σ total ↑ in ↓ out ·
  cache hit`, folded into one cell so an 80-column pane keeps every other
  column whole. The old long form (`in … · out … · cache read … · cache
  write … · reasoning …`) is gone from the pane.
- **`brokkr graph`'s text:** each state line ends with the same reading.

## Testing

- **brokkr-view:** unit tests on synthetic journals and tables, never real session content. They cover:
  - counts by rule;
  - dormant rules;
  - retired rules;
  - drift;
  - an unresolvable recipe;
  - a self-loop, a back edge and terminals;
  - medians;
  - cost with silent providers;
  - ends;
  - live runs;
  - `on_frontier`, including ties.
- **CLI:** `brokkr graph` text and `--json` on a fixture journal, and a refusal for an unknown recipe that names the known ones.
- **Plugin (`npm test`):**
  - edge widths and dashes;
  - the selected state's panel;
  - quadrant classification at the targets and on their boundaries;
  - targets persisting and surviving refused storage;
  - an unplotted recipe with unreported cost;
  - the version check.
- **Observed:** the console loaded into an isolated DSH against a real journal, read-only, with both views and screenshots, and the journal byte-identical before and after, as slice two observed it.
