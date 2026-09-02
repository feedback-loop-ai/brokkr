# The constitution

Every semantic change to Brokkr is a numbered operator ruling, kept
here in full. The engine cites them by number in code, in error text and
in the README, so a rule can always be read back to its reason. Only the
operator accepts a decision; an implementer may only propose one.

**Anyone may propose one.** A decision proposal is an ordinary pull
request adding a numbered document to this directory, and the door is
open to contributors, not just the machine's own seats. The grammar it
must carry:

- `Status: proposed` — always. Acceptance is the operator's, recorded
  in the file by the operator's own ruling, never claimed by the
  author.
- A **Context** (the problem and the alternatives weighed), numbered
  **Rulings** (what is being ruled, one rule per number), and
  **Consequences**.
- For every ruling that *can* be enforced deterministically, the
  **enforcement binding**: the config, loader refusal, CI gate, or
  schema that will refuse violations. A determinable ruling with no
  named mechanism is judgment-guidance and must say so.
- The **next free number** — claim it in the PR itself, since parallel
  authors have collided on numbers before; the PR's merge is the
  reservation.

Rulings are never edited into a different meaning: corrections are
dated, attributed errata inside the document, and a superseding rule
takes a new number and says so. A proposal the operator declines stays
in the PR record, not in this directory.

| # | Title | What it rules | Status |
|---|---|---|---|
| [0001](0001-no-llm-repair-of-control-plane.md) | Schema mismatches are never repaired by a model | An invalid or unmatched result parks with the raw evidence — never guessed at, coerced, or handed to a model to fix. | accepted |
| [0002](0002-linear-outer-machine.md) | The outer machine is linear; we keep the FSM | One active phase at a time, a totally-ordered journal, `evaluate()` over a data table — no graph engine, no concurrency in the control plane. | accepted |
| [0003](0003-native-rust-runtime.md) | The production runtime is one native Rust binary | One prebuilt executable with the web UI and SQLite embedded: no Python, no Node, no services. | accepted |
| [0004](0004-strict-condition-evaluation.md) | Strict condition evaluation in the pure core | Conditions fail closed: a missing or wrong-typed input never satisfies a rule. | accepted |
| [0005](0005-self-forging-first-scope.md) | Self-forging first: the initial implementation scope | The first target is the smallest engine that can drive its own delivery, and what that deliberately leaves out. | accepted |
| [0006](0006-bounded-attempts-and-deadlines.md) | Bounded automated attempts and seat deadlines | Per-seat attempt limits and deadlines; determinate failures retry, indeterminate outcomes always park. | accepted |
| [0007](0007-input-provenance.md) | Input provenance: declared, engine-owned, or dropped | Every evaluation input is engine-computed or seat-declared; everything else is dropped before the table or the record sees it. | accepted |
| [0008](0008-second-wave-scope.md) | Second wave: the 0005 deferrals, delivered and bounded | The deferred scope delivered slice by slice, each verified by Brokkr's own verify agents. | accepted |
| [0009](0009-rust-only.md) | Rust only: the oracle retires, drivers move into the binary | The repository is Rust-only; the drivers become built-in adapters of the one binary. | accepted |
| [0010](0010-composable-recipes.md) | Composable recipes: delivery strategies as swappable, comparable data | Recipes are a library — named, installable, swappable, comparable by run id. | accepted |
| [0011](0011-standalone-identity.md) | Standalone identity: no origin, only heritage | Brokkr is a standalone product; no document, string or comment references the workspace it came from. | accepted |
| [0012](0012-sealed-secret-bindings.md) | Sealed secret bindings: seats reference secrets, only the runner resolves them | Bundles and journals carry `{{secret:NAME}}` — names only; values live in an operator-side store and are masked on the way out. | accepted |
| [0013](0013-one-derivation-two-surfaces.md) | One derivation, two surfaces: `brokkr-view` and the terminal readout | Every readout renders the same pure view models, so one question has one answer, tested once. | accepted |
| [0014](0014-interactive-tui.md) | `brokkr tui`: an interactive, read-only console in the terminal | Keyboard navigation over the same derivation: three levels, no operator commands, nothing written to the journal. | accepted |
| [0015](0015-run-selectors.md) | Run selectors: a prefix or `latest`, resolved in one place | `--run` takes any unique run-id prefix or `latest`, through one resolver shared by every readout. | proposed |
| [0016](0016-agent-library.md) | The agent library: seats reference agents, adapters map them to providers | An agent is one file; a provider adapter is data; what a provider cannot express is written down and fails compilation. | accepted |
| [0017](0017-composable-recipes.md) | Composable recipes: extend, override, and compose delivery strategies | `extends` plus explicit overrides, resolved at compile time into one flat bundle recorded in the run manifest. | accepted |
| [0018](0018-dual-license.md) | Dual license: MIT OR Apache-2.0 | Permissive, never copyleft: MIT's frictionlessness or Apache-2.0's explicit patent grant, the user's choice. | accepted |
| [0019](0019-brokkr.md) | Brokkr: the name, the verb, and the lore layer | The product is Brokkr; "forge" survives as the verb and mechanisms keep plain names; the lore layer under `docs/lore/` is bound by five laws. | accepted |
| [0020](0020-muninn.md) | Muninn: the raven that reads everything and rules nothing | The standing overseer reads only journal-derived models, proposes and never rules, and records every proposal as evidence with provenance; delegation only by future recorded grant. | accepted |
| [0021](0021-model-policy.md) | Model policy: the law, not the scorecard | Work and gate seats; operator-granted driver trust tiers and egress rights, compile-refused when violated; park, never substitute; economics stays LaneTally's. | accepted |
| [0022](0022-reforging.md) | Reforging: the graph gets its way back into the fire | A security residual returns the run to implement with the finding as declared input, bounded at two reforgings; the exhaustion ladder stops, parks, or ships-as-debt by severity. | accepted |
| [0023](0023-realms.md) | Realms: the map is the world, chosen at invocation | realms.json (minimal v1 schema, this repo its own bootstrap) picked by --realms on run and every read surface, pinned and embedded per run; per-realm facts on decisions; Bifröst and multi-realm runs are later phases. | accepted |
| [0029](0029-fenced-append.md) | The fenced append: a writer commits onto the head it folded | A control-plane write states the head its fold was taken from and refuses when the journal has moved; a stale fold surfaces the drift rather than picking a winner. A fence, not a lease. Narrowed by erratum: the primitive and the operator/conclude fences landed; the tail (resume's fresh-process branch) remains. | accepted |
| [0025](0025-skirnir.md) | Skírnir and the grant: the sword is a signed loan | The standing executor acts only within an operator-GPG-signed, expiring, runtime-configurable grant, under a compiled never-list ceiling; every exercise journaled as the grant's, escalation the default. | accepted |
| [0026](0026-many-hearths.md) | Many hearths: per-realm journals and the tabbed fleet | realms/v2 journals per realm (products, not worktrees) with per-realm tabs; same-realm parallelism instead hardens concurrent writers into ONE journal; muninn flies the whole world; journals never merge. | accepted |
| [0027](0027-import.md) | Import: journals never merge, runs relocate | One run moves from a canonical export into a destination journal byte-identically, behind full verification; a broken chain, a run-id collision and a redacted derivative each refuse the import whole; arrival is recorded beside the chain, never inside it. | accepted |
| [0028](0028-keep-refs.md) | Keep-refs: the journal's exhibits outlive the branch | Every SHA a run's journal cites gets `refs/forge/keep/<run>/<sha>`, planted automatically at conclusion and by verb; idempotent, listed by one `for-each-ref`, deleted only by the operator. | accepted |
| [0030](0030-codex-session-resume.md) | Codex session resume: the cache win and the sandbox it drops | `codex exec resume` lifts the prompt-cache hit from a ~75% cold plateau to 92–96% (measured), but refuses `--sandbox` and does not inherit it — a resumed read-only thread writes. Safe only via `-c sandbox_mode=`. Ruled: a retry or a re-entry of the same seat rejoins its own thread with the class re-imposed, or the driver spawns cold; only the instance that opened a session may resume it. | accepted |
| [0031](0031-seat-model-pin.md) | The served model is evidence; every model seat is pinned | Drivers record the provider-reported served model, never a configured guess; every model-backed invocation is compile-refused unless explicitly pinned. | accepted |
| [0033](0033-contributing-through-brokkr.md) | Contributing goes through Brokkr | Every pull request to `main` names a completed Brokkr run whose published anchor carries an offline-verifiable journal and vouches for the proposed head; only the operator's visible `by-hand` label may skip the check. | accepted |

## How a decision is made

1. An implementer writes the file with status `proposed` and the
   context that forced the question.
2. The operator accepts it — and only the operator. Acceptance is
   recorded in the file's status line with the date.
3. The code cites the number where it enforces the ruling, so the rule
   and its enforcement stay findable from each other.

A decision is never edited into a different meaning. When a ruling
changes, a new numbered decision supersedes it and says so.

## The decision culture

Every semantic change is a numbered operator ruling in
[`docs/decisions/`](./), kept in full, cited by number in
the code that enforces it. [The index](README.md) lists
them with their status.

An implementer may write a decision, but only ever with status
`proposed`; acceptance is the operator's, recorded in the file. A ruling
is never edited into a different meaning — a new number supersedes it
and says so. That is why the README, the error messages and the tests
can all cite "decision 0007" and mean the same paragraph.
