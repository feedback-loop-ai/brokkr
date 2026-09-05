# Recipe authoring — bundles, policy tables, and composition

A **recipe** is a delivery strategy written as data: a `bundle.json`
naming seats, a policy table naming phases and rules, and a `roles/`
directory of charters and, where a deterministic office needs one, a
script. The engine compiles it into
one flat pinned object identified by a content digest, and runs that.

This guide is the anatomy. It assumes you have read
[quickstart.md](quickstart.md) and have a scaffold from `brokkr init` in
front of you.

- [The three files](#the-three-files)
- [`bundle.json` anatomy](#bundlejson-anatomy)
- [Seat bodies: single, panel, sequence, select](#seat-bodies-single-panel-sequence-select)
- [Composition: `extends` and `override`](#composition-extends-and-override)
- [Digest identity](#digest-identity)
- [The policy table](#the-policy-table)
- [The condition vocabulary](#the-condition-vocabulary)
- [The reforging ladder](#the-reforging-ladder)
- [Compile it](#compile-it)

## Recipes and composition

A recipe is a delivery strategy as reviewable data, identified by
content digest. The library is a directory of them.

```
$ brokkr recipes list
fast	e0c1eb0757cd	6 phases	implement, review, ship, verify	medium	Default Rust delivery from implementation through verification, review, and ship.	recipes/fast
night-shift	1fa050796f20	12 phases	analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement, review{chore=reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify]	medium-high	Unattended triage routing that parks on the first unusual result and uses the dsh implementation lane.	recipes/night-shift
node	7abfe34bba48	6 phases	implement, review, ship, verify	medium	Node and TypeScript repositories using JavaScript-specific seats and tools.	recipes/node
panel-review	1bf81bd7249f	7 phases	implement, intake, review[correctness+security], ship, verify	high	General delivery needing independent correctness and security reviewers.	recipes/panel-review
preflight	d28c15ee4994	4 phases	review, verify	medium	Verify and review an existing branch without implementing or shipping it.	recipes/preflight
research	50c7db63c6f5	4 phases	research, verify	low	Read articles and propose registry entries with cited classifications; the operator rules them.	recipes/research
research-dsh	8334be119867	4 phases	research, verify	low	The research intake on the dsh lane: the same charter and gate, the researcher seated on Qwen3.8-Max with page fetch turned on.	recipes/research-dsh
triage	2bc18b05f87a	12 phases	analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement{chore=implementer;design=implementer-sdd;engine=implementer-engine;feature=implementer}, review{chore=reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify]	variable	Routing delivery: a chief-grade triage gate rules the class before Fast's crew, adding the current SDD design council when ruled.	recipes/triage
wager-harness	57de5240560b	6 phases	implement, review, ship, verify	medium	Driver evaluation that swaps only implementation to Codex for a fair wager.	recipes/wager-harness
wager-harness-dsh	9ef87077a982	6 phases	implement, review, ship, verify	medium	Driver evaluation that swaps only implementation to DSH for a fair wager.	recipes/wager-harness-dsh
wager-harness-muse	8cc242bb8bd6	6 phases	implement, review, ship, verify	low	Driver evaluation that swaps only implementation to Muse Spark 1.3 on its contributor terms, through dsh, for a fair wager.	recipes/wager-harness-muse
self	d06e486c3ef8	7 phases	implement, intake, review, ship, verify			./bundles/self
verify	8054a434cc74	4 phases	review, verify			./bundles/verify
```

| Recipe | Reach for it when | The difference it states |
|---|---|---|
| [`fast`](../../recipes/fast) | the default delivery: implement → verify → review → ship | the base the composed recipes below extend |
| [`triage`](../../recipes/triage/README.md) | routing a commission by delivery class | `extends fast`: the triage result selects the later offices, with the five-phase SDD route included for the classes that need it |
| [`night-shift`](../../recipes/night-shift/README.md) | an unattended overnight queue | `extends triage`: one-attempt limits and a dsh implementation lane, so anything unusual parks for morning instead of retrying |
| [`wager-harness`](../../recipes/wager-harness/README.md) | weighing a new driver for a trust-tier promotion | `extends fast`: one seat's driver swapped to `codex`, plus the parity checklist that makes the comparison mean something |
| [`wager-harness-dsh`](../../recipes/wager-harness-dsh/README.md) | running the same driver wager through DSH | `extends fast`: only the implement seat moves, so the judging seats stay comparable |
| [`wager-harness-muse`](../../recipes/wager-harness-muse/README.md) | the same wager on Muse Spark 1.3, contributor terms, through dsh | `extends fast`: only the implement seat moves, to a third vendor at `xhigh` |
| [`node`](../../recipes/node/README.md) | a Node/TypeScript repository | `fast`'s constitution with JavaScript drivers and charters |
| [`panel-review`](../../recipes/panel-review) | a second reviewer's read | `review` is a flat two-member panel joined by an aggregate |
| [`preflight`](../../recipes/preflight/README.md) | verifying and reviewing an existing branch without delivery | a two-seat table that stops after its ruling: no intake, implement, or ship |

Nine entries carry pinned manifest digests in
`crates/brokkr-runtime/tests/witness_digests.rs`; the rest are covered
by the tree-wide compile test but not pinned. The cost bands printed by
the library are relative strategy labels for the sixty-second
contributor choice, never price quotes. Dollar figures remain absent
unless a run backs them — economics belongs in the run ledger, not this
engine's recipe metadata (decision 0021 ruling 6).

`recipes/node` is the same four-seat constitution as `fast`, driving a
Node/TypeScript repository instead of this Rust one: the phase table is
identical, only the seats' driver commands and role charters are
JavaScript. [Adopting a Node repo](adopting-a-node-repo.md)
walks a stranger from an unmodified repo to a first run.

Every shipped `verify` and `ship` seat is an inline gate-class `exec`
site with workspace hands. No model reports evidence that a script can
measure: the verifier writes `pass` only after every fixed command exits
zero, and the shipper calls `brokkr ledger` and writes no commit. A
recipe with different gates, such as Node or preflight, keeps its own
verifier script beside its roles under the same `pass`/`fail` contract.

Recipes **compose** (decision 0017). `recipes/night-shift` extends
`triage`, replaces one seat, and changes only the other seats' limits;
it has to say both changes out loud.

```json
{
  "name": "night-shift",
  "extends": "triage",
  "override": {
    "seats": ["implement"],
    "limits": ["triage", "specify", "clarify", "design", "tasks", "analyze", "verify", "review", "ship"]
  },
  "seats": {
    "implement": { "…": "the replacement dsh seat" },
    "verify": { "limits": { "max_attempts": 1, "timeout_seconds": 3600 } }
  }
}
```

Named things merge by name; redefining one the base already has without
the matching `override` marker fails compilation rather than silently
winning. Composition resolves at compile time into ONE flat bundle — no
inheritance at run time — and the run manifest records the chain, so a
run states what it was composed from.

Swap a strategy and compare the outcomes:

```
brokkr rerun --run <id> --recipe panel-review    # same feature, other strategy
brokkr compare <a> <b>                           # trails, first divergence, per-seat costs
```

## The three files

These recipe files choose the office, but they do not carry all prompt text.
The prompt has three sources: the selected charter (the portable office), the
selected realm's optional house Markdown (repository conventions), and the
engine-rendered result contract (the site's closed vocabulary and result-file
path). Keep toolchain commands and repository paths in the house, not in a
library charter. Recipe-local roles may remain specific to the recipe.

```
recipes/my-recipe/
  bundle.json      # the strategy: seats, limits, drivers, which table
  policy.json      # the phase machine: phases, rules, terminals
  roles/*.md       # one charter per seat, panel member or sequence step
```

Only `bundle.json` is mandatory by name. The policy table's filename is
whatever `bundle.json`'s `"policy"` key says, resolved relative to that
layer's own directory. Role paths are likewise bundle-relative.

Two worked shapes are in the tree:

- `bundles/self/bundle.json` — the flat case. Five seats, each one agent
  reference, nothing else.
- `recipes/triage/bundle.json` — the full case. Strategy-selected
  single, panel, and sequence bodies; the five-phase SDD route with a nested
  design panel and deterministic checks; declared inputs; and inline
  shell-script steps alongside agent-backed ones.

## `bundle.json` anatomy

```json
{
  "name": "triage",
  "description": "Routing delivery selected by a chief-grade triage gate.",
  "cost": "variable",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": { "…": "one entry per working phase" }
}
```

| Key | Required | Meaning |
|---|---|---|
| `name` | yes | The strategy's identity. For a composed recipe this is the leaf's name. |
| `description` | shipped recipes | The one-line answer to "when should I use this?", rendered by `brokkr recipes list` and the contributing guide. |
| `cost` | shipped recipes | A relative cost band for choosing a strategy, never a provider quote. |
| `policy` | in the layer that supplies a table | Path to this layer's phase-machine file, relative to this layer's directory. A layer that declares no `policy` contributes no table. |
| `protected_phase` | no | The phase every path to a non-`stop` terminal must pass through. Defaults to `"review"`. |
| `egress_minimum` | no | The egress class a seat's route must meet before that seat may bind secrets (decision [0036](../decisions/0036-egress-is-a-property-of-the-route.md) ruling 4): `"local"`, `"contracted"` or `"uncontracted"`. Defaults to `"contracted"`. |
| `seats` | in the layer that supplies them | Phase name → seat definition. Every working phase needs one by the time composition has resolved. |
| `extends` | no | The name of a recipe in the library this one builds on. See [composition](#composition-extends-and-override). |
| `override`, `remove` | no | Resolver markers, only meaningful with `extends`; `override.cases` replaces selected cases and `override.limits` replaces only named seats' limits. |

`protected_phase` is enforced structurally at compile time, not by
convention. The compiler walks the rule graph from `initial`, refusing
to traverse into the protected phase, and rejects the bundle if any
terminal other than `stop` is still reachable:

```
policy reaches terminal 'done' without passing 'review'; a path to
shipping that bypasses the protected review gate is constitutionally
rejected
```

A rule that parks draws no edge in that walk — a park reaches no phase,
so it cannot smuggle a route around review.

### A seat

```json
"implement": {
  "role": "roles/implementer.md",
  "results": ["complete", "broken", "blocked"],
  "inputs": ["fixes_applied"],
  "limits": { "max_attempts": 2, "timeout_seconds": 5400 },
  "secrets": ["GITHUB_TOKEN"],
  "driver": {
    "command": ["{brokkr}", "driver", "claude", "--", "--model",
                "claude-fable-5-1", "--permission-mode", "acceptEdits"]
  }
}
```

| Key | Meaning |
|---|---|
| `results` | **Required.** The seat's closed result vocabulary. A result outside this list fails schema validation and parks the run with the raw evidence. |
| `role` + `driver` | The inline form: a charter file and the argv to spawn. |
| `agent` | The library form (decision 0016): `"agent": "implementer"` resolves charter, driver, limits and declared inputs from `agents/`. Mutually exclusive with the inline form. |
| `inputs` | The typed facts this seat may supply (decision 0007). Defaults to the non-engine-owned inputs this phase's own rules reference. Anything undeclared is dropped before evaluation and never enters the journal record. |
| `limits` | `max_attempts` and `timeout_seconds`. Defaults: one attempt, 3600 seconds. |
| `secrets` | Secret **names** this seat binds (decision 0012). Values live in an operator-side store outside version control; bundles and journals carry names only. The seat's driver must reach a route whose egress class meets the bundle's `egress_minimum`, or compilation refuses (decision 0036 ruling 4). |
| `driver.confine` | Optional container confinement: `image`, `network`, `mounts`. |
| `hands` | Decision 0043, Linux only: `"workspace"` or `{"kind":"workspace","network":bool,"binds":[{path,mode,mask}]}` with mode `ro`, `rw` or `overlay` (the host path as a read-only lower layer, writes kept in a per-seat upper layer that never touches the host — the mode for a toolchain cache). The seat's commands run inside an empty-root box holding the worktree; an exec seat also gets its bundle root read-only at `/runtime/bundle`, so its `./` script travels with the strategy. A tool allow-list is not consulted, and a boxed `exec` command may hold a gate. Refused beside secret bindings and beside `agent:` (the agent declares its own). |

Two argv tokens are expanded at spawn time: `{brokkr}` becomes this
engine's own executable (so a bundle can name the built-in adapters),
and a `./`-prefixed entry is bundle-relative. For a boxed exec seat that
entry is expanded to the read-only `/runtime/bundle` mount at spawn, so
the operated-on repository need not carry or copy the strategy's script.
`{forge}` is the same
token under its pre-rename name; it still expands and warns once on
stderr. The expansion is machine-local, which is why the manifest
records driver *names*, never resolved argv.

Every inline claude, codex or dsh command must carry a
non-empty `--model <concrete-model-id>` (decision 0031). Compilation
refuses the complete set of unpinned invocation sites and names this
same fix. Agent-backed seats satisfy the rule through their resolved
candidate argv. Exec is model-free and needs no pin.

**Engine-owned inputs are never seat-declarable.** `strategy`,
`drift_detected`, `dirty_worktrees`, `reviewed_heads`, `realm_facts`,
`fixes_docs_only`
(decision 0039: whether every commit the review itself added lies in the
docs class of the repository's `.github/delivery-classes.json` as it was
committed at the head the review was entered at, never the working tree's
copy — absent when the repository declared no class there), the
strategy selected from the triage result, the
`consecutive_failures` counter and the
whole `visits_<phase>` family are computed by the engine from the journal
and the tree. A seat that claims one has the
claim dropped; a bundle that declares one fails compilation.

## Seat bodies: single, panel, sequence, select

A seat takes exactly one of four shapes.

**Single** — one driver, one charter, one result. The `role`+`driver`
example above.

**Panel** — members run in parallel, and a named aggregate joins their
results into the one typed result the machine sees:

```json
"review": {
  "results": ["clean", "residual", "security-hold"],
  "inputs": ["spec_defect", "has_security_residual", "max_residual_severity"],
  "limits": { "max_attempts": 2, "timeout_seconds": 3600 },
  "aggregate": "review-panel",
  "panel": {
    "correctness": { "agent": "review-correctness", "class": "gate" },
    "security":    { "agent": "review-security", "class": "gate" }
  }
}
```

The aggregate vocabulary is closed — named in data, implemented in the
engine, never arbitrary code:

| Aggregate | Emits | Rule |
|---|---|---|
| `unanimous-pass` | `pass`, `fail` | `pass` only when every member reports `pass`. |
| `review-panel` | `clean`, `residual`, `security-hold` | Worst member wins over `clean` < `residual` < `security-hold`; severity is the max, and the security and fixes flags are OR-ed. |

A seat's declared `results` must cover its aggregate's vocabulary or
compilation fails. Any indeterminate member makes the whole attempt
indeterminate, which parks; otherwise any failed member fails the
attempt.

**Sequence** — named steps run one after another inside one effect.
Later steps see earlier steps' result objects as context. The `results` array
on every non-final step is that step's own prompt vocabulary; the final step
receives the enclosing seat's vocabulary because its result is what the phase
machine consumes; declaring `results` on the final step is refused rather than
silently ignored. The engine refuses an intermediate result outside that
declared vocabulary before it can become later context. A panel may omit the
array only because its named aggregate supplies a fixed vocabulary; when the
array is present, the compiler checks that it covers every aggregate result
instead of ignoring it. This distinction is pinned by the v8 run-manifest shape;
the **final** step's result is the effect's single typed result:

```json
"design": {
  "results": ["drafted", "fail"],
  "inputs": ["change"],
  "limits": { "max_attempts": 2, "timeout_seconds": 3600 },
  "sequence": [
    { "name": "positions", "results": ["pass", "fail"],
      "aggregate": "unanimous-pass",
      "panel": { "simplicity": {"agent": "position-simplicity"},
                 "robustness": {"agent": "position-robustness"} } },
    { "name": "chief", "results": ["drafted"],
      "agent": "chief-architect" },
    { "name": "validate", "dialect": "validate" }
  ]
}
```

Step order is load-bearing, which is why `sequence` is an array and not
an object — a JSON object would sort its keys and the digest would not
rebuild identically. A step's body is either single or panel. A
non-final step's aggregate output never reaches the rule table, so only
the final step's vocabulary is checked against the seat's declared
`results`.

Each SDD artifact sequence (`specify`, `design`, and `tasks`) ends in a
`validate` gate resolved from the realm's dialect at compile time. It is boxed
as a deterministic exec with no model, and its `{change}` token comes from the
nearest preceding typed result. The `clarify` and `analyze` loop sequences run
their dialect check before the read-only judge, which receives that output as
`prior_results`. A realm that does not declare a dialect cannot compile a
bundle containing these dialect-owned steps. Ordinary inline seats remain
first-class for work that is not dialect-owned.

In this slice, `brokkr doctor --bundle ...` exercises that compile-time
refusal but does not yet print the realm's dialect, its tool, or the measured
tool version. That doctor readout is the next decision-0042 slice; use
`brokkr realms` to confirm realm selection and treat a successful compile as
the current dialect check.

**Select** — the engine chooses a complete single, panel, or sequence
body from the journaled delivery strategy:

```json
"review": {
  "results": ["clean", "residual", "security-hold"],
  "inputs": ["spec_defect", "has_security_residual", "max_residual_severity"],
  "limits": { "max_attempts": 2, "timeout_seconds": 7200 },
  "select": {
    "on": "strategy",
    "cases": {
      "chore": {"agent": "reviewer", "class": "gate"},
      "feature": {
        "aggregate": "review-panel",
        "panel": {
          "correctness": {"agent": "review-correctness", "class": "gate"},
          "security": {"agent": "review-security", "class": "gate"}
        }
      },
      "design": {"sequence": ["… full named steps …"]},
      "engine": {"sequence": ["… full named steps …"]}
    }
  }
}
```

Only `strategy` is admitted by `select.on`. Every reachable strategy
must resolve to a named case or `default`. The compiler applies the
same agent resolution, trust-tier, model-pin, egress, hands, and judging
checks to every case. A run with no matching triage result takes
`default`; without one it parks rather than guessing. The chosen case
is recorded on `phase/entered`, is included in each case-qualified
invocation site such as `review:feature`, and is reconstructed from the
journal on resume. The manifest's optional `select` map pins every
case and its resolved agents; it stays absent when no seat selects, so
older identities do not gain empty metadata.

**The non-final-step rule has teeth, so read it before you use a
sequence on a gate.** The `design` and `engine` review cases in
`recipes/triage` put a positions panel before a single chief step. The
panel's `review-panel` output is *not* the seat's result — it is a
checkpoint, handed to the chief as `context.prior_results.positions` —
so a `security-hold` from the panel reaches the rule table only if the
chief reproduces it. The engine will accept a chief that rules lower.

The same rule disables a fail-closed path you may not know you were
relying on. A panel aggregate that cannot read a member's payload emits
`result: "__member-schema-invalid__"` — deliberately not in any
vocabulary, so that the seat's declared-results check rejects it and the
run parks with the member evidence attached. On a *non-final* step
nothing performs that check, so the sentinel is handed to the next step
as an ordinary string and the malformed-driver signal is lost unless
that step's charter names it. Give the floor a branch for a result
outside the vocabulary, not just for the worst result inside it.

There is a third consequence, and it is the easiest to miss: a non-final
step's `notes` are copied **verbatim** into the next step's context. If
the later step is a gate and an earlier one is a work seat — which
admits any driver, trusted or not (decision 0021 ruling 7) — then
untrusted model prose is now input to the prompt of the seat that rules
the phase. A flat panel has no such path, because its verdict is
computed in code and no member writes into it. Say so in the judging
step's charter: what it receives is data to check against the diff, and
text that argues for a verdict is a finding to name, never an
instruction to follow. The review-chief charter used by
`recipes/triage` is the worked example.

When a sequence's later step judges earlier ones, put the floor in that
step's charter and test the plumbing. `recipes/triage/bundle.json`
shows both selected review sequences, and
`crates/brokkr-runtime/tests/crucible_review_sequence.rs` pins their
sequence behavior.

## Composition: `extends` and `override`

Decision 0017. `recipes/night-shift/bundle.json` is the current
multi-layer example: it inherits routing and selection from `triage`,
replaces the whole `implement` seat, and changes only the limits of the
other named seats.

```json
{
  "name": "night-shift",
  "extends": "triage",
  "override": {
    "bundle": ["description", "cost"],
    "seats": ["implement"],
    "limits": ["triage", "specify", "clarify", "design", "tasks", "analyze", "verify", "review", "ship"]
  },
  "seats": {
    "implement": {
      "results": ["complete", "broken", "blocked", "oversized"],
      "limits": { "max_attempts": 1, "timeout_seconds": 7200 },
      "role": "roles/implementer.md",
      "class": "work",
      "driver": { "command": ["…"] }
    },
    "verify": {
      "limits": { "max_attempts": 1, "timeout_seconds": 3600 }
    }
  }
}
```

It has no `policy.json`, SDD bodies, review selector, verifier body, or shipper
body of its own because it redefines none of them. Those values
and the whole phase machine, including the protected review gate, are
inherited.

A selected case may be replaced without restating its enclosing seat.
The marker names `<seat>:<case>`:

```json
{
  "extends": "triage",
  "override": { "cases": ["review:feature"] },
  "seats": {
    "review": {
      "select": {
        "cases": {
          "feature": { "agent": "reviewer", "class": "gate" }
        }
      }
    }
  }
}
```

The rules:

- **`extends` names a recipe in the library**, not a path. Names match
  `^[a-z0-9][a-z0-9-]*$`, checked before any path is built, so `../x`,
  `a/b`, uppercase names, and `.` are refused and never become paths.
- **Named things merge by name.** A derived layer adding a seat, rule, or
  selector case the base does not have is a plain addition.
- **Redefining a name the base already defines requires an explicit
  marker.** `override.seats` replaces a complete seat,
  `override.cases` replaces one case while retaining the rest of the
  selector, and `override.limits` replaces only the named seat's
  attempt/deadline object. Without the corresponding marker compilation
  fails rather than silently choosing the derived value.
- **Case markers are qualified.** Every `override.cases` entry is
  `<seat>:<case>`; both the inherited seat and inherited case must
  exist, and the derived value must supply that case.
- **A marker that describes nothing is also a refusal.** `override` or
  `remove` naming something no ancestor defines fails with “a marker
  that describes nothing is a lie about the composition.”
- **`override` is keyed by** `seats`, `cases`, `limits`, `rules`,
  `table`, or `bundle`; **`remove` by** `seats`, `rules`, or
  `phases`.
- **Table name arrays merge by union.** `phases`, `terminal`, and
  `shippable_from` union rather than collide, so a derived recipe
  re-declaring an inherited phase says nothing new instead of erroring.
  A table scalar is not an array and does not union: changing `initial`
  or `description` requires naming it in `override.table`.
- **Chains are bounded at eight layers**, and a repeated directory is a
  cycle reported with the whole loop in order.
- **Origin stays attached to inherited and partial values.** Relative
  role and command paths resolve against the layer that wrote the seat
  or selected case. Paths inside a hands box are POSIX strings joined
  with `/`, independent of the host.

**Composition resolves at compile time into one flat bundle.** There is
no inheritance at run time and no dynamic lookup. `brokkr compile`
prints the resolved result plus a `composed_from` chain naming each
ancestor and its digest, and that chain rides in the run manifest under
the reserved `@compose/` prefix — so a run states not only what it ran
but what it was composed from.

## Digest identity

A bundle's identity is the SHA-256 of its canonical manifest, which
covers a per-file digest of every bundle file: the policy table, the
seat definitions and every selected case, the role charters, the result
schemas, and the driver command names. Friendly names are display
metadata; the digest is identity.

```
$ brokkr compile --bundle recipes/night-shift
{
  "bundle": "night-shift",
  "digest": "1fa050796f20…",
  "phases": ["triage", "specify", "clarify", "design", "tasks", "analyze",
             "implement", "verify", "review", "ship", "done", "stop"],
  "seats": ["analyze", "clarify", "design", "implement", "review", "ship",
            "specify", "tasks", "triage", "verify"],
  "manifest": {
    "…": "one sha256 per bundle file",
    "select": { "review": { "…": "every resolved case" } }
  },
  "composed_from": [
    { "recipe": "triage", "digest": "4488af3d1165…", "dir": "…/recipes/triage" },
    { "recipe": "fast", "digest": "8d3e5dc7acfb…", "dir": "…/recipes/fast" }
  ]
}
```

`brokkr recipes show <name>` prints the same object from the same code,
so the two surfaces cannot drift. `brokkr recipes list` prints the first
twelve hex characters per recipe.

Three consequences worth internalising:

1. **An ancestor's digest transitively covers its own ancestors.**
   Change `recipes/fast`, and both `triage` and `night-shift` move:
   they are different strategies.
2. **Compile-time resolution never probes the machine.** Agent
   resolution depends on exactly two digested inputs — the agent library
   and the adapters — and deliberately not on availability. A compile
   that probed `PATH` would give one bundle two digests and make an
   in-flight run unresumable after an `apt install`.
3. **Resume refuses a digest mismatch** with a diagnostic; it never
   picks up edited files. Editing a recipe mid-run means the run is no
   longer resumable under it — start a new one.

## The policy table

`forge.phase-machine/v2` (schema:
`contracts/phase-machine.v2.schema.json`; evaluator:
`crates/brokkr-core/src/policy.rs`).

```json
{
  "schema": "forge.phase-machine/v2",
  "description": "…",
  "phases": ["triage", "design", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "triage",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [ … ]
}
```

`phases`, `initial`, `terminal` and `rules` are what the loader
hard-requires. `schema` is read when present — and a table that declares
a park rule MUST declare `forge.phase-machine/v2`, or the loader refuses
it; the reference JSON Schema in `contracts/` lists `schema` as required
for conformance, but the runtime loader does not enforce its absence.
`initial` and every `terminal` must be one of `phases`, and no rule may
leave a terminal phase.

### A rule

```json
{
  "id": "IMPL-BROKEN-TWICE",
  "from": "implement",
  "result": "broken",
  "when": { "consecutive_failures_gte": 2 },
  "next": "stop",
  "severity": "hard",
  "reason": "Two consecutive broken implement runs; stop and report rather than thrash."
}
```

| Key | Required | Meaning |
|---|---|---|
| `id` | yes | The rule id journaled in `transition/decided` and printed by every readout. |
| `from` | yes | The phase this rule rules from. |
| `result` | yes | The typed result it matches. |
| `reason` | yes | Why it rules what it rules. Read back verbatim by the operator; for a parking rule, this is the park reason. |
| `next` | one of | The phase to advance to. |
| `park` | one of | `true` — rule a park instead of a transition. Mutually exclusive with `next`. |
| `severity` | no | `normal` \| `flagged` \| `hard`. The ruling severity of the transition taken; defaults to `normal`. Forbidden on a parking rule. |
| `when` | no | Conditions, all of which must hold. |
| `requires_artifacts` | no | Workdir-relative paths that must exist for this rule to take. Forbidden on a parking rule. |

**`park` is `v2` vocabulary and the table must declare it.** The loader
refuses a parking rule in a table that calls itself
`forge.phase-machine/v1` — a park is not a stop, and the difference is
the whole point of having the version. A parking rule takes no
transition, so it declares neither `severity` (a property of a taken
transition) nor `requires_artifacts` (a gate on one).

**Evaluation is first-match-wins, in table order.** Order your rules
from most specific to least. A rule made unreachable by an
unconditional rule for the same `(from, result)` above it is refused at
load — a deadened rule is a bug, not a comment.

## The condition vocabulary

The vocabulary is **closed and validated at load** (decision 0004), so a
typo'd key can never silently deaden a rule; it fails compilation
naming the rule and the known set.

| Form | Reads | Known names |
|---|---|---|
| bare name | a boolean input, by name | `skip_verify`, `fixes_applied`, `spec_defect`, `has_security_residual`, `high_risk_uncovered`, `drift_detected`, `dirty_worktrees`, `fixes_docs_only` |
| `strategy_in` | whether the engine-owned `strategy` is one of a non-empty array of delivery classes | `chore`, `feature`, `design`, `engine`, `escalate` |
| `<counter>_gte` | numeric threshold | `consecutive_failures` |
| `visits_<phase>_gte` | how many times the run has entered `<phase>` | any phase **this table** declares |
| `<axis>_above` | severity strictly above the threshold | `max_residual_severity` |
| `<axis>_at_most` | severity at or below the threshold | `max_residual_severity` |

The severity axis, lowest to highest, is `none` · `info` · `low` ·
`medium` · `high` · `critical`. (It is a different axis from the ruling
`severity` on a rule, which is `normal` · `flagged` · `hard`.)

Three evaluation laws:

1. **Absence never satisfies a condition.** An absent or null input
   fails every predicate, including `_at_most` — which is the
   fail-closed direction: an unranked severity does not slip onto a
   "small enough to ship" arm.
2. **A present input the vocabulary cannot read parks the run.** It is
   never coerced and never guessed at.
3. **An unmatched `(phase, result)` pair parks**, with no problem string
   — there was simply no ruling for it.

`visits_<phase>_gte` closes over the table's own graph: the suffix must
name a phase this table declares. The count is engine-owned, folded from
`phase/entered` events; a seat can neither declare nor claim one.
`strategy_in` likewise reads only the engine-owned strategy folded from
triage's successful result. A seat can neither declare nor overwrite it,
and every listed value is checked against the closed delivery-class
vocabulary.

`recipes/fast/policy.json` and `recipes/triage/policy.json` together
are the fullest worked example of the grammar — booleans, strategy
membership, a counter, both severity comparisons, visit predicates, and
both `next` and `park` outcomes.

## The reforging ladder

Decision 0022 introduced the first bounded return: a security residual
at review goes back into implement instead of ending the run. Decision
0041 adds two more ownership-directed returns in `triage`: a
specification defect goes back to design, and an implementation that
outgrows its ruled class goes back to triage.

**Review → implement.** `REVIEW-REFORGE` sends a security residual of
any severity to `implement` with flagged severity. The implementer
receives the review result as `context.returned_from`: findings, notes,
and severities reach the seat that must answer them. Verify and review
then run again.

The bound is `visits_implement_gte: 3`. The first visit is the original
implementation and the second and third are the two reforgings. Once
three visits have been entered, the exhaustion rules above the return
take precedence:

| Rule | Also requires | Outcome |
|---|---|---|
| `…-EXHAUSTED-ABOVE-MEDIUM` | `max_residual_severity_above: "medium"` | `next: stop`, severity `hard`. Above medium after two reforgings is the operator's. |
| `…-EXHAUSTED-MEDIUM` | `max_residual_severity_above: "low"` | `park: true`. A surviving medium parks with the residual as reason. |
| `…-EXHAUSTED-DEBT` | `fixes_applied: true`, `max_residual_severity_above: "none"`, `max_residual_severity_at_most: "low"` | `next: ship`, severity `flagged`. Ships as named, tracked debt. |
| `…-EXHAUSTED-UNFIXED` | nothing further | `park: true`. A low or info residual that remains unfixed parks. |

The debt arm requires a severity positively ranked low or info —
`above: "none"` and `at_most: "low"` together — because absence never
satisfies a condition. An absent or unranked severity cannot take the
shipping arm. Above the ladder, `REVIEW-SECURITY-HOLD` stops
unconditionally with hard severity; risk acceptance remains the
operator's.

**Review → design.** Only the `design` and `engine` strategies can take
a specification-defect return, expressed by `strategy_in` together
with `spec_defect: true`. Both a `clean` and a `residual` review can
send the defect to `design`, because ownership of the defect is
independent of the ordinary review verdict. The returning design seat
receives that review result in `context.returned_from`.

The bound is `visits_design_gte: 3`: the original design plus two
returns. `REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED` and
`REVIEW-SPEC-DEFECT-EXHAUSTED` sit above their return rules and park
instead of attempting a fourth design visit.

**Implement → triage.** `IMPL-OVERSIZED` returns an `oversized`
implementation once so the delivery class can be ruled again. The
triage seat remains a fresh-and-blind office: it receives the commission
and current tree, not journal history. The bound is
`visits_triage_gte: 2`: after the original ruling and one re-triage,
`IMPL-OVERSIZED-EXHAUSTED` parks rather than looping.

The transferable pattern is the same on every edge: put the bounded
return below its exhaustion arm, count entries to the destination phase,
and make any arm that proceeds demand positive evidence rather than the
absence of bad evidence. Because selection is reconstructed from the
journaled strategy, a returned run keeps the same case provenance across
resume until triage deliberately rules a new class.

## Compile it

```
brokkr compile --bundle recipes/my-recipe      # validate, print manifest + digest
brokkr doctor --bundle recipes/my-recipe       # plus: are the drivers actually here
brokkr recipes list                            # the library, one digest per recipe
brokkr recipes show my-recipe                  # resolved bundle + composition chain
```

`brokkr compile` takes `--bundle <path>` only — there is no `--recipe`
form; use `brokkr recipes show` for a library name. Compilation is where
every structural law above is enforced, so a bundle that compiles is a
bundle whose review gate is unavoidable, whose aggregates match their
declared results, whose conditions are all in the vocabulary, and whose
composition markers all describe something real.

## See also

- [driver-authoring.md](driver-authoring.md) — what goes behind
  `driver.command` when it is not a built-in adapter.
- [versioning.md](versioning.md) — which of these shapes are frozen.
- [decision 0004](../decisions/0004-strict-condition-evaluation.md) —
  strict condition evaluation.
- [decision 0006](../decisions/0006-bounded-attempts-and-deadlines.md) —
  attempts and deadlines.
- [decision 0007](../decisions/0007-input-provenance.md) — input
  provenance.
- [decision 0017](../decisions/0017-composable-recipes.md) — composition.
- [decision 0022](../decisions/0022-reforging.md) — the reforging ladder.
