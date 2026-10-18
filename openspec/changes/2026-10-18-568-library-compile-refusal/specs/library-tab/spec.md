## ADDED Requirements

### Requirement: A recipe that does not compile says so, and every recipe stays pickable

When the graph's table is `refused`, the Graph view SHALL show a panel headed with Brokkr's words that the recipe does not compile against this Brokkr. Beneath the heading SHALL be the compiler's reason, one clause to a line and wrapped, then the bundle directory. With no state to draw, the view SHALL say there is no traffic to draw, and SHALL draw no tiles and no empty graph. The recipe picker SHALL list every recipe the realm's graph names, with or without runs, and SHALL mark each recipe in `does_not_compile`. When a graph cannot be read, the picker SHALL keep the recipes the realm's last graph named.

#### Scenario: A realm whose recipes predate this Brokkr

- **WHEN** lanetally has no runs in range and Brokkr draws `dependencies`, which does not compile
- **THEN** the panel reads "dependencies doesn't compile against this Brokkr" with the adapter's refusal beneath it, and the picker lists `dependencies · doesn’t compile`, `fast` and `triage · doesn’t compile`, each of which can be picked

#### Scenario: A graph that cannot be read

- **WHEN** the operator picks `muninn` and its graph cannot be read
- **THEN** the error is shown, and the picker still lists every recipe the realm's last graph named, those that never ran included
