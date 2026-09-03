# Recipe authoring — bundles, policy tables, and composition

A **recipe** is a delivery strategy written as data: a `bundle.json`
naming seats, a policy table naming phases and rules, and a `roles/`
directory of charters. It has no code in it. The engine compiles it into
one flat pinned object identified by a content digest, and runs that.

This guide is the anatomy. It assumes you have read
[quickstart.md](quickstart.md) and have a scaffold from `brokkr init` in
front of you.

- [The three files](#the-three-files)
- [`bundle.json` anatomy](#bundlejson-anatomy)
- [Seat bodies: single, panel, sequence](#seat-bodies-single-panel-sequence)
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
crucible	17dfdff6f440	6 phases	implement, review[positions>chief], ship, verify	high	Engine, store, protocol, or contract changes needing a review panel and chief.	recipes/crucible
ember	894e6e72e763	7 phases	implement, intake, review, ship, verify	low	Docs, chores, and small fixes using the shared agent roster.	recipes/ember
fast	f6f960da0503	6 phases	implement, review, ship, verify	medium	Default Rust delivery from implementation through verification, review, and ship.	recipes/fast
night-shift	cefe6b617107	6 phases	implement, review, ship, verify	medium-high	Unattended work that should park on the first unusual result instead of retrying.	recipes/night-shift
node	2ebd5ac5ad6a	6 phases	implement, review, ship, verify	medium	Node and TypeScript repositories using JavaScript-specific seats and tools.	recipes/node
panel-review	5e9b058d5b1a	7 phases	implement, intake, review[correctness+security], ship, verify	high	General delivery needing independent correctness and security reviewers.	recipes/panel-review
preflight	ffa8c3a07b99	4 phases	review, verify	medium	Verify and review an existing branch without implementing or shipping it.	recipes/preflight
sdd	d3bbfc195ae1	8 phases	design[positions>chief>speckit-check], implement, intake, review[security+spec-compliance], ship, verify	high	Spec-driven work that needs a design panel, chief synthesis, and spec-kit check.	recipes/sdd
sdd-paranoid	d990d7d92b68	8 phases	design[positions>chief>speckit-check], implement, intake, review[adversarial+security], ship, verify	very high	Spec-driven high-risk work needing adversarial and security review.	recipes/sdd-paranoid
wager-harness	6ea8645805e3	6 phases	implement, review, ship, verify	medium	Driver evaluation that swaps only implementation to Codex for a fair wager.	recipes/wager-harness
wager-harness-dsh	02c227af9917	6 phases	implement, review, ship, verify	medium	Driver evaluation that swaps only implementation to DSH for a fair wager.	recipes/wager-harness-dsh
self	f083edddbbd4	7 phases	implement, intake, review, ship, verify			./bundles/self
verify	162fef593349	4 phases	review, verify			./bundles/verify
```

| Recipe | Reach for it when | The difference it states |
|---|---|---|
| [`fast`](../../recipes/fast) | the default delivery: implement → verify → review → ship | the base every recipe below extends |
| [`ember`](../../recipes/ember/README.md) | docs, chores, small fixes | `extends fast`: adds intake and seats the shared roster on all five phases |
| [`crucible`](../../recipes/crucible/README.md) | engine, store, protocol or contract changes | `extends fast`: seats `implementer-engine`, then a correctness+security panel whose verdict `review-chief` synthesises |
| [`night-shift`](../../recipes/night-shift/README.md) | an unattended overnight queue | `extends fast`: `max_attempts: 1` on every seat, so anything unusual parks for morning instead of retrying |
| [`wager-harness`](../../recipes/wager-harness/README.md) | weighing a new driver for a trust-tier promotion | `extends fast`: one seat's driver swapped to `codex`, plus the parity checklist that makes the comparison mean something |
| [`wager-harness-dsh`](../../recipes/wager-harness-dsh/README.md) | running the same driver wager through DSH | `extends fast`: only the implement seat moves, so the judging seats stay comparable |
| [`node`](../../recipes/node/README.md) | a Node/TypeScript repository | `fast`'s constitution with JavaScript drivers and charters |
| [`panel-review`](../../recipes/panel-review) | a second reviewer's read | `review` is a flat two-member panel joined by an aggregate |
| [`preflight`](../../recipes/preflight/README.md) | verifying and reviewing an existing branch without delivery | a two-seat table that stops after its ruling: no intake, implement, or ship |
| [`sdd`](../../recipes/sdd) | spec-driven delivery | adds a `design` sequence: positions → chief → a deterministic spec-kit check |
| [`sdd-paranoid`](../../recipes/sdd-paranoid/README.md) | SDD with a harsher panel | `extends sdd`, replacing exactly one seat |

Eight of the entries above carry pinned manifest digests in
`crates/brokkr-runtime/tests/witness_digests.rs` — `fast`, `node`, the
four roster recipes, `preflight`, and `bundles/verify`; the rest are
covered by the tree-wide compile test but not pinned. The `low` through `very high`
bands printed by the library are relative strategy labels for the
sixty-second contributor choice, never price quotes. Dollar figures
remain absent unless a run backs them — economics is LaneTally's ledger,
not this engine's (decision 0021 ruling 6).

`recipes/node` is the same four-seat constitution as `fast`, driving a
Node/TypeScript repository instead of this Rust one: the phase table is
identical, only the seats' driver commands and role charters are
JavaScript. [Adopting a Node repo](adopting-a-node-repo.md)
walks a stranger from an unmodified repo to a first run.

Recipes **compose** (decision 0017). `recipes/sdd-paranoid` is sixty
lines: it extends `sdd` and replaces exactly one seat, and it has to say
so out loud.

```json
{
  "name": "sdd-paranoid",
  "extends": "sdd",
  "override": { "seats": ["review"] },
  "seats": {
    "review": { "…": "an adversarial panel instead of SDD's" }
  }
}
```

Named things merge by name; redefining one the base already has without
listing it under `override` fails compilation rather than silently
winning. Composition resolves at compile time into ONE flat bundle — no
inheritance at run time — and the run manifest records the chain, so a
run states what it was composed from.

Swap a strategy and compare the outcomes:

```
brokkr rerun --run <id> --recipe panel-review    # same feature, other strategy
brokkr compare <a> <b>                           # trails, first divergence, per-seat costs
```

## The three files

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
- `recipes/sdd/bundle.json` — the full case. A `sequence` seat with a
  nested panel, a panel seat with an aggregate, declared inputs, and an
  inline shell-script step alongside agent-backed ones.

## `bundle.json` anatomy

```json
{
  "name": "sdd",
  "description": "Spec-driven work that needs a design panel, chief synthesis, and spec-kit check.",
  "cost": "high",
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
| `override`, `remove` | no | Resolver markers, only meaningful with `extends`. |

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
| `hands` | Decision 0043, Linux only: `"workspace"` or `{"kind":"workspace","network":bool,"binds":[{path,mode,mask}]}` with mode `ro`, `rw` or `overlay` (the host path as a read-only lower layer, writes kept in a per-seat upper layer that never touches the host — the mode for a toolchain cache). The seat's commands run inside an empty-root box holding only the worktree; a tool allow-list is not consulted, and a boxed `exec` command may hold a gate. Refused beside secret bindings and beside `agent:` (the agent declares its own). |

Two argv tokens are expanded at spawn time: `{brokkr}` becomes this
engine's own executable (so a bundle can name the built-in adapters),
and a `./`-prefixed entry is bundle-relative. `{forge}` is the same
token under its pre-rename name; it still expands and warns once on
stderr. The expansion is machine-local, which is why the manifest
records driver *names*, never resolved argv.

Every inline Claude, LaneTally, Codex, or dsh command must carry a
non-empty `--model <concrete-model-id>` (decision 0031). Compilation
refuses the complete set of unpinned invocation sites and names this
same fix. Agent-backed seats satisfy the rule through their resolved
candidate argv. Exec is model-free and needs no pin.

**Engine-owned inputs are never seat-declarable.** `drift_detected`,
`dirty_worktrees`, `reviewed_heads`, `realm_facts`, `fixes_docs_only`
(decision 0039: whether every commit the review itself added lies in the
docs class of the repository's `.github/delivery-classes.json` as it was
committed at the head the review was entered at, never the working tree's
copy — absent when the repository declared no class there), the
`consecutive_failures` counter and the
whole `visits_<phase>` family are computed by the engine from the journal
and the tree. A seat that claims one has the
claim dropped; a bundle that declares one fails compilation.

## Seat bodies: single, panel, sequence

A seat takes exactly one of three shapes.

**Single** — one driver, one charter, one result. The `role`+`driver`
example above.

**Panel** — members run in parallel, and a named aggregate joins their
results into the one typed result the machine sees:

```json
"review": {
  "results": ["clean", "residual", "security-hold"],
  "inputs": ["fixes_applied", "has_security_residual", "max_residual_severity"],
  "limits": { "max_attempts": 2, "timeout_seconds": 3600 },
  "aggregate": "review-panel",
  "panel": {
    "spec-compliance": { "agent": "review-spec-compliance" },
    "security":        { "agent": "review-security" }
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
Later steps see earlier steps' result objects as context, and the
**final** step's result is the effect's single typed result:

```json
"design": {
  "results": ["designed", "fail"],
  "limits": { "max_attempts": 2, "timeout_seconds": 3600 },
  "sequence": [
    { "name": "positions", "aggregate": "unanimous-pass",
      "panel": { "simplicity": {"agent": "position-simplicity"},
                 "robustness": {"agent": "position-robustness"} } },
    { "name": "chief", "agent": "chief-architect" },
    { "name": "speckit-check", "role": "roles/speckit-check.md",
      "driver": { "command": ["{brokkr}", "driver", "exec", "--", "bash",
                              "recipes/sdd/drivers/speckit_check.sh", "{prompt_file}"] } }
  ]
}
```

Step order is load-bearing, which is why `sequence` is an array and not
an object — a JSON object would sort its keys and the digest would not
rebuild identically. A step's body is either single or panel: the same
two forms a seat itself may take. A non-final step's aggregate output
never reaches the rule table, so only the final step's vocabulary is
checked against the seat's declared `results`.

`recipes/sdd`'s `speckit-check` is worth noting: it is a shell script
with no model at all. Inline seats stay first-class; the agent library
is an option, not a requirement.

**The non-final-step rule has teeth, so read it before you use a
sequence on a gate.** `recipes/crucible` puts one on `review`: a
`positions` panel of `security` and `correctness`, then a single `chief`
step that synthesises them. The panel's `review-panel` output is *not*
the seat's result — it is a checkpoint, handed to the chief as
`context.prior_results.positions` — so a `security-hold` from the panel
reaches the rule table only if the chief reproduces it. The engine will
accept a chief that rules lower.

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
instruction to follow. `recipes/crucible`'s chief charter is the worked
example.

When a sequence's later step judges earlier ones, put the floor in that
step's charter and test the plumbing:
[`recipes/crucible/README.md`](../../recipes/crucible/README.md#the-review-sequence--the-one-new-shape-here)
walks the shape, and
`crates/brokkr-runtime/tests/crucible_review_sequence.rs` pins it.

## Composition: `extends` and `override`

Decision 0017. `recipes/sdd-paranoid/bundle.json` is the worked example,
and it is sixty lines against SDD's 103:

```json
{
  "name": "sdd-paranoid",
  "extends": "sdd",
  "override": { "seats": ["review"] },
  "seats": {
    "review": {
      "results": ["clean", "residual", "security-hold"],
      "inputs": ["fixes_applied", "has_security_residual", "max_residual_severity"],
      "limits": { "max_attempts": 2, "timeout_seconds": 3600 },
      "aggregate": "review-panel",
      "panel": {
        "adversarial": { "agent": "review-adversarial" },
        "security":    { "agent": "review-security" }
      }
    }
  }
}
```

It has no `policy.json`, no intake role and no shipper role of its own,
because it redefines none of them. Everything else — the intake seat,
the design sequence, the implementer, the verifier, the shipper, and the
whole phase machine including the protected review gate — is inherited.

Four more recipes extend `recipes/fast` the same way, each stating one
kind of difference, and they are worth reading as a set:
[`ember`](../../recipes/ember/README.md) adds a phase and re-pins every
seat's model, [`crucible`](../../recipes/crucible/README.md) replaces a
seat body with a sequence,
[`night-shift`](../../recipes/night-shift/README.md) changes limits,
model pins and charters, and
[`wager-harness`](../../recipes/wager-harness/README.md)
changes only one seat's driver. `ember` is also the one that needs a
table marker as well as seat markers — see below.

The rules:

- **`extends` names a recipe in the library**, not a path. Names match
  `^[a-z0-9][a-z0-9-]*$`, checked before any path is built, so `../x`,
  `a/b`, `SDD` and `.` are refused as names and never become paths.
- **Named things merge by name.** A derived layer adding a seat the base
  does not have is a plain addition.
- **Redefining a name the base already defines requires an explicit
  marker.** Without it, compilation fails rather than the derived value
  silently winning:

  ```
  …/recipes/sdd-paranoid/bundle.json: redefines seat 'review', which
  …/recipes/sdd/bundle.json already defines; mark it 'override.seats:
  ["review"]' to replace it deliberately, or give it another name to
  add one
  ```

  (paths print absolute — the composer canonicalizes every layer's
  directory before it speaks; abbreviated here with `…`)

- **A marker that describes nothing is also a refusal.** `override` or
  `remove` naming something no ancestor defines fails with "a marker
  that describes nothing is a lie about the composition."
- **`override` is keyed by** `seats`, `rules`, `table` or `bundle`;
  **`remove` by** `seats`, `rules` or `phases`.
- **Table name arrays merge by union.** `phases`, `terminal` and
  `shippable_from` union rather than collide, so a derived recipe
  re-declaring an inherited phase says nothing new instead of erroring.
  A table **scalar** is not an array and does not union: `recipes/ember`
  adds an `intake` phase and an `INTAKE-OK` rule to `fast`'s table as
  plain additions, but moving `initial` from `"implement"` to
  `"intake"` redefines a scalar the base already set, so its bundle
  carries `"override": { "table": ["description", "initial"] }`.
  Without it, compilation refuses the bundle naming the collision.
  Adding a phase is free; changing where the machine starts is not.
- **Chains are bounded at eight layers**, and a repeated directory is a
  cycle reported with the whole loop in order.
- **Seat values are opaque to the resolver.** It decides only *which*
  value wins for a given name and never opens one — which is why
  `override` and `remove` are top-level keys beside the values rather
  than keys inside them.

**Composition resolves at compile time into one flat bundle.** There is
no inheritance at run time and no dynamic lookup. `brokkr compile`
prints the resolved result plus a `composed_from` chain naming each
ancestor and its digest, and that chain rides in the run manifest under
the reserved `@compose/` prefix — so a run states not only what it ran
but what it was composed from.

## Digest identity

A bundle's identity is the SHA-256 of its canonical manifest, which
covers a per-file digest of every bundle file: the policy table, the
seat definitions, the role charters, the result schemas, the driver
command names. Friendly names are display metadata; the digest is
identity.

```
$ brokkr compile --bundle recipes/sdd-paranoid
{
  "bundle": "sdd-paranoid",
  "digest": "368569ad218d…",
  "phases": ["intake", "design", "implement", "verify", "review", "ship", "done", "stop"],
  "seats": ["design", "implement", "intake", "review", "ship", "verify"],
  "manifest": { "…": "one sha256 per bundle file" },
  "composed_from": [ { "recipe": "sdd", "digest": "3743484daa2b…", "dir": "…/recipes/sdd" } ]
}
```

`brokkr recipes show <name>` prints the same object from the same code,
so the two surfaces cannot drift. `brokkr recipes list` prints the first
twelve hex characters per recipe.

Three consequences worth internalising:

1. **An ancestor's digest transitively covers its own ancestors.**
   Change `recipes/sdd`, and `sdd-paranoid`'s digest moves: it is a
   different strategy.
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
  "phases": ["intake", "design", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "intake",
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
| bare name | a boolean input, by name | `skip_verify`, `fixes_applied`, `has_security_residual`, `high_risk_uncovered`, `drift_detected`, `dirty_worktrees`, `fixes_docs_only` |
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

`recipes/sdd/policy.json`'s `review` rules are the fullest worked
example of the grammar in one place — booleans, a counter, both severity
comparisons, the visit predicate, and both `next` and `park` outcomes,
in a single ordered family.

## The reforging ladder

Decision 0022. The problem it solves: a security residual at review used
to be a dead end — the run stopped and a human started over. The ladder
sends it back into implement instead, bounded, and then rules explicitly
on what survives.

Five of the seven `review`/`residual` rules in
`recipes/sdd/policy.json` are the ladder (the other two are the ordinary
non-security residual arms below it). Read them bottom-up to see the
shape:

**The bottom rung — `REVIEW-REFORGE`.** A security residual of any
severity goes back to `implement`, severity `flagged`. The implementer
receives the review's findings, notes and severities as
`context.returned_from` — the fold hands a phase the run *returns* to
the result that sent it back, so a precise finding reaches whoever has
to answer it. Then verify and review rule again.

**The bound — `visits_implement_gte: 3`.** Nothing counts reforgings
directly; the run's visit count to `implement` does. First visit is the
original implementation, second and third are the two reforgings. When
the table sees `implement` entered three times, the four
`REVIEW-REFORGE-EXHAUSTED-*` rules become eligible and, sitting above
`REVIEW-REFORGE` in the table, take precedence over another trip:

| Rule | Also requires | Outcome |
|---|---|---|
| `…-EXHAUSTED-ABOVE-MEDIUM` | `max_residual_severity_above: "medium"` | `next: stop`, severity `hard`. Above medium after two reforgings is the operator's. |
| `…-EXHAUSTED-MEDIUM` | `max_residual_severity_above: "low"` | `park: true`. A surviving medium parks, the residual as the park reason — so the ruling lands inside the run's journal instead of after its death. |
| `…-EXHAUSTED-DEBT` | `fixes_applied: true`, `max_residual_severity_above: "none"`, `max_residual_severity_at_most: "low"` | `next: ship`, severity `flagged`. Ships as tracked debt named in the notes. |
| `…-EXHAUSTED-UNFIXED` | (nothing further) | `park: true`. Low or info surviving unfixed parks, the same door a medium takes. |

The debt arm is the one to read carefully. It requires the severity to
be **positively** ranked low or info — `above: "none"` and `at_most:
"low"` together — precisely because absence never satisfies a condition.
An absent or unranked severity cannot take the shipping arm; it falls
through to `…-EXHAUSTED-UNFIXED` and parks. That asymmetry is the whole
safety property: the ladder can ship a *named* small residual, and never
an unknown one.

Above the ladder sits `REVIEW-SECURITY-HOLD`: a `security-hold` result
is `next: stop`, severity `hard`, unconditionally. Risk acceptance is
the operator's, never an agent's, and no amount of reforging reaches it.

If you are writing your own table, the transferable pattern is: put the
bounded retry at the bottom, the exhaustion arms above it ordered
strictly-worst-first, and make the one arm that *proceeds* demand
positive evidence rather than the absence of bad evidence.

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
