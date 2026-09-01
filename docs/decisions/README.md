# The constitution

Every semantic change to the Forge is a numbered operator ruling, kept
here in full. The engine cites them by number in code, in error text and
in the README, so a rule can always be read back to its reason. Only the
operator accepts a decision; an implementer may only propose one.

| # | Title | What it rules | Status |
|---|---|---|---|
| [0001](0001-no-llm-repair-of-control-plane.md) | Schema mismatches are never repaired by a model | An invalid or unmatched result parks with the raw evidence — never guessed at, coerced, or handed to a model to fix. | accepted |
| [0002](0002-linear-outer-machine.md) | The outer machine is linear; we keep the FSM | One active phase at a time, a totally-ordered journal, `evaluate()` over a data table — no graph engine, no concurrency in the control plane. | accepted |
| [0003](0003-native-rust-runtime.md) | The production runtime is one native Rust binary | One prebuilt executable with the web UI and SQLite embedded: no Python, no Node, no services. | accepted |
| [0004](0004-strict-condition-evaluation.md) | Strict condition evaluation in the pure core | Conditions fail closed: a missing or wrong-typed input never satisfies a rule. | accepted |
| [0005](0005-self-forging-first-scope.md) | Self-forging first: the initial implementation scope | The first target is the smallest engine that can drive its own delivery, and what that deliberately leaves out. | accepted |
| [0006](0006-bounded-attempts-and-deadlines.md) | Bounded automated attempts and seat deadlines | Per-seat attempt limits and deadlines; determinate failures retry, indeterminate outcomes always park. | accepted |
| [0007](0007-input-provenance.md) | Input provenance: declared, engine-owned, or dropped | Every evaluation input is engine-computed or seat-declared; everything else is dropped before the table or the record sees it. | accepted |
| [0008](0008-second-wave-scope.md) | Second wave: the 0005 deferrals, delivered and bounded | The deferred scope delivered slice by slice, each verified by the forge's own verify agents. | accepted |
| [0009](0009-rust-only.md) | Rust only: the oracle retires, drivers move into the binary | The repository is Rust-only; the drivers become built-in adapters of the one binary. | accepted |
| [0010](0010-composable-recipes.md) | Composable recipes: delivery strategies as swappable, comparable data | Recipes are a library — named, installable, swappable, comparable by run id. | accepted |
| [0011](0011-standalone-identity.md) | Standalone identity: no origin, only heritage | The Forge is a standalone product; no document, string or comment references the workspace it came from. | accepted |
| [0012](0012-sealed-secret-bindings.md) | Sealed secret bindings: seats reference secrets, only the runner resolves them | Bundles and journals carry `{{secret:NAME}}` — names only; values live in an operator-side store and are masked on the way out. | accepted |
| [0013](0013-one-derivation-two-surfaces.md) | One derivation, two surfaces: `forge-view` and the terminal readout | Every readout renders the same pure view models, so one question has one answer, tested once. | accepted |
| [0014](0014-interactive-tui.md) | `forge tui`: an interactive, read-only console in the terminal | Keyboard navigation over the same derivation: three levels, no operator commands, nothing written to the journal. | accepted |
| [0015](0015-run-selectors.md) | Run selectors: a prefix or `latest`, resolved in one place | `--run` takes any unique run-id prefix or `latest`, through one resolver shared by every readout. | proposed |
| [0016](0016-agent-library.md) | The agent library: seats reference agents, adapters map them to providers | An agent is one file; a provider adapter is data; what a provider cannot express is written down and fails compilation. | accepted |
| [0017](0017-composable-recipes.md) | Composable recipes: extend, override, and compose delivery strategies | `extends` plus explicit overrides, resolved at compile time into one flat bundle recorded in the run manifest. | accepted |
| [0018](0018-dual-license.md) | Dual license: MIT OR Apache-2.0 | Permissive, never copyleft: MIT's frictionlessness or Apache-2.0's explicit patent grant, the user's choice. | accepted |
| [0019](0019-brokkr.md) | Brokkr: the name, the verb, and the lore layer | The product is Brokkr; "forge" survives as the verb and mechanisms keep plain names; the lore layer under `docs/lore/` is bound by five laws. | accepted |
| [0020](0020-muninn.md) | Muninn: the raven that reads everything and rules nothing | The standing overseer reads only journal-derived models, proposes and never rules, and records every proposal as evidence with provenance; delegation only by future recorded grant. | accepted |
| [0021](0021-model-policy.md) | Model policy: the law, not the scorecard | Work and gate seats; operator-granted driver trust tiers and egress rights, compile-refused when violated; park, never substitute; economics stays LaneTally's. | accepted |
| [0022](0022-reforging.md) | Reforging: the graph gets its way back into the fire | A security residual returns the run to implement with the finding as declared input, bounded at two reforgings; the exhaustion ladder stops, parks, or ships-as-debt by severity. | accepted |
| [0023](0023-realms.md) | Realms: the map is the world, chosen at invocation | realms.json (minimal v1 schema, this repo its own bootstrap) picked by --realms on run and every read surface, pinned and embedded per run; per-realm facts on decisions; Bifröst and multi-realm runs are later phases. | accepted |
| [0025](0025-skirnir.md) | Skírnir and the grant: the sword is a signed loan | The standing executor acts only within an operator-GPG-signed, expiring, runtime-configurable grant, under a compiled never-list ceiling; every exercise journaled as the grant's, escalation the default. | accepted |
| [0026](0026-many-hearths.md) | Many hearths: per-realm journals and the tabbed fleet | realms/v2 journals per realm (products, not worktrees) with per-realm tabs; same-realm parallelism instead hardens concurrent writers into ONE journal; muninn flies the whole world; journals never merge. | accepted |
| [0027](0027-import.md) | Import: journals never merge, runs relocate | One run moves from a canonical export into a destination journal byte-identically, behind full verification; a broken chain, a run-id collision and a redacted derivative each refuse the import whole; arrival is recorded beside the chain, never inside it. | proposed |
| [0028](0028-keep-refs.md) | Keep-refs: the journal's exhibits outlive the branch | Every SHA a run's journal cites gets `refs/forge/keep/<run>/<sha>`, planted automatically at conclusion and by verb; idempotent, listed by one `for-each-ref`, deleted only by the operator. | proposed |

## How a decision is made

1. An implementer writes the file with status `proposed` and the
   context that forced the question.
2. The operator accepts it — and only the operator. Acceptance is
   recorded in the file's status line with the date.
3. The code cites the number where it enforces the ruling, so the rule
   and its enforcement stay findable from each other.

A decision is never edited into a different meaning. When a ruling
changes, a new numbered decision supersedes it and says so.
