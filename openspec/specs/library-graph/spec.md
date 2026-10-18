# library-graph Specification

## Purpose
Give the operator one derivation of a recipe's state machine with the
journal's traffic on it, from `brokkr graph`, so the console draws what
Brokkr reads: the rules with their counts and latest runs, the retired
rules and the drift, each state's figures, and how the runs ended.

## Requirements

### Requirement: A recipe's graph is its policy table with the journal's traffic

`brokkr graph --recipe <name>` SHALL resolve the recipe's policy table in the realm, as `brokkr recipes show` resolves it, or from a bundle directory whose `bundle.json` name is `<name>`. It SHALL read the journal's runs that carry that recipe name within `--range` (default `7d`), and emit `GraphView`. The view SHALL hold:
- the states in `phases` order;
- every rule with its `count` of `transition/decided` events in range, and its `latest` runs: those that fired it in range, the latest firing of each, newest first, at most five;
- per-state figures;
- the runs' ends;
- the live runs.

`--json` SHALL print the view verbatim. Without `--json`, the verb SHALL print one line per state and per rule, and the ends.

#### Scenario: A rule that fired

- **WHEN** three runs of recipe `self` decided `VERIFY-FAIL` in range
- **THEN** the rule `VERIFY-FAIL` from `verify` to `stop` has `count` 3 and `kind` `terminal`

#### Scenario: A rule that never fired

- **WHEN** the table holds `DESIGN-RETRY` and no run decided it in range
- **THEN** it is in the view with `count` 0

#### Scenario: An unknown recipe

- **WHEN** `--recipe nosuch` names no recipe in the realm and no run carries it
- **THEN** the verb refuses, naming the recipes it knows, and prints no view

### Requirement: Drift and retired rules are stated, not hidden

The view SHALL list, as `retired`, the rule ids that fired in range but are not in the resolved table, each with its count. When a run's pinned `policy.json` digest differs from the resolved table's, the view SHALL carry `drifted_runs`, the number of such runs. When the table cannot be resolved but runs carry the name, the view SHALL carry only the fired rules and SHALL say the table was not found in the realm.

#### Scenario: A rule removed from the table

- **WHEN** `OLD-RULE` fired twice in range and the current table has no `OLD-RULE`
- **THEN** `retired` lists `OLD-RULE` with count 2

#### Scenario: The table moved

- **WHEN** four runs pinned a `policy.json` digest other than the resolved one
- **THEN** `drifted_runs` is 4 and the text names it

### Requirement: Each state carries its figures

Each state SHALL carry:
- `entered`, the distinct runs that entered it in range;
- the median and p90 time from its entry to the run's next phase entry or terminal event;
- reported cost, with the providers that reported and the ones that did not;
- its tokens, summed and read apart as total, input, output and the cache hit;
- the seats and the models the latest run in range pinned, by human name.

#### Scenario: A state whose seat reports no cost

- **WHEN** every seat of `design` ran on a provider that reports no cost
- **THEN** its cost is absent, and the provider is listed as not reporting

### Requirement: Ends and live runs

The view SHALL count runs by how they ended in range: `done`, `stop` per rule id, `operator`, and `open`. It SHALL list each open run with its fleet label and its current state.

#### Scenario: A live run

- **WHEN** one run of the recipe is in `review` and not concluded
- **THEN** `ends.open` is 1 and `live` names that run with state `review`

### Requirement: A recipe that does not compile is drawn with its refusal

`brokkr graph` SHALL compile every recipe the realm resolves by name.

A recipe that resolves and does not compile against this Brokkr SHALL NOT fail the verb. Its view's `table` SHALL be `refused`, carrying:
- Brokkr's words naming the recipe;
- the compiler's reason, without the `bundle: ` words that wrap it;
- the bundle directory it resolves to, named in the realm's folder when it lies there.

The view SHALL then draw only the rules its runs fired, as for a table not found. Every view SHALL carry `does_not_compile`, the realm's recipes that do not compile, sorted. A failure to compile that is not the recipe's own refusal SHALL still fail the verb. The graph's wire version SHALL be 2.

#### Scenario: An adapter written for an older Brokkr

- **WHEN** the realm's `claude` adapter carries `binding_grant`, a key this Brokkr does not know, and `--recipe starter` is asked
- **THEN** the verb prints a view whose `table` is `refused` with the text "starter doesn't compile against this Brokkr", the reason "adapter 'claude' (<path>) has unknown key 'binding_grant'; known keys: …", and the bundle `.brokkr`, and `does_not_compile` is `["starter"]`

#### Scenario: A realm's dialect that cannot be read

- **WHEN** the realm's map names a dialect that does not exist
- **THEN** the verb fails, and names no recipe as not compiling

## Provenance

- `2026-10-17-568-library-graph-and-pareto` — folded 2026-10-17
- `2026-10-18-568-library-compile-refusal` — folded 2026-10-18
