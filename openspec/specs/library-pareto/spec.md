# library-pareto Specification

## Purpose
Give the operator one figure per recipe, from `brokkr analytics`, and
Brokkr's own reading of which recipes stand on the frontier, so the console
plots a realm's recipes by done rate and cost per run against the viewer's
targets without working out any figure of its own.

## Requirements

### Requirement: Analytics carries one figure per recipe

`brokkr analytics --json` SHALL carry `recipes`, one entry per realm and recipe name with runs in range. Each entry SHALL hold:
- `runs`;
- `done`, the runs that reached `done`;
- `done_rate`, a percent;
- `cost_per_run`: the reported cost over runs, absent when no seat of the recipe reported cost, with the `reporting` and `silent` providers;
- `tokens_per_run`, the shared token figure (total, input, output, cache hit) averaged over the runs;
- `on_frontier`.

The analytics view version SHALL move by one.

#### Scenario: A recipe with partial cost

- **WHEN** a recipe's runs report cost from Claude seats only, and its dsh and Codex seats report none
- **THEN** `cost_per_run` is the Claude cost over runs, `reporting` names `claude`, and `silent` names `dsh` and `codex`

### Requirement: The frontier is Brokkr's reading

`on_frontier` SHALL be true for a recipe when no other recipe of the same realm with runs and a reported cost has a done rate at least as high and a cost per run at most as low, with one of the two strictly better. A recipe with no reported cost SHALL have `on_frontier` absent.

#### Scenario: One recipe dominates another

- **WHEN** recipe A is 40% done at $2 a run and recipe B is 30% done at $3 a run in the same realm
- **THEN** A is on the frontier and B is not

#### Scenario: A tie

- **WHEN** two recipes have the same done rate and cost per run
- **THEN** both are on the frontier

### Requirement: The console plots recipes in four classified quadrants

The Library's Pareto view SHALL plot each recipe of the realm with a cost per run, with done rate up and cost per run right. It SHALL split the plane at two viewer targets, done rate (default 50%) and cost per run (default $3.00), into:
- **Pareto frontier quadrant**: at or above the done-rate target and at or below the cost target;
- **Reliable but costly**;
- **Cheap but leaky**;
- **Costly and leaky**.

It SHALL join the `on_frontier` points with a step line. It SHALL list recipes with no reported cost as "cost not reported", and the realm's recipes with no runs in range as "not measured yet". The targets SHALL be kept in the browser for this console and edited on the view. A refused or empty storage SHALL fall back to the defaults.

#### Scenario: A point on a target

- **WHEN** a recipe is exactly 50% done at exactly $3.00 a run with the default targets
- **THEN** it is in the Pareto frontier quadrant

#### Scenario: Storage refused

- **WHEN** the browser refuses storage
- **THEN** the view draws at 50% and $3.00, and editing a target still moves the lines for this page

## Provenance

- `2026-10-17-568-library-graph-and-pareto` — folded 2026-10-17
