# Triage routing

This is the routing form of decision 0041 ruling 6. A fresh, blind
chief-grade gate reads the commission and named tree, writes the framing in
`.forge/tasks/<slug>.md`, and rules exactly one class:

| Class | Route |
|---|---|
| `chore` | implement → verify → review → ship |
| `feature` | implement → verify → review → ship |
| `design` | design → implement → verify → review → ship |
| `engine` | design → implement → verify → review → ship |
| `escalate` | park with triage's reasoning |

The closed class vocabulary is `chore`, `feature`, `design`, `engine`, and
`escalate`. The class is recorded as the run's engine-owned `strategy` fact.
Below triage, this recipe keeps Fast's bounded verify, review, and ship
constitution and the current SDD design council. Ruling 7's strategy-selected
seats come later; until then even a `chore` deliberately runs Fast's crew.

Verification and shipping are deterministic boxed exec gates. Cargo runs
offline inside the verifier's box from the bound registry cache; a cache miss
fails closed.
