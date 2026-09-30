## MODIFIED Requirements

### Requirement: Every shipped bundle compiles under harness once the fragments are measured
Decision 0046 ruling 6 promises that after this slice a macOS operator
runs every shipped bundle, the review offices under their harness's own
sandbox. The work offices that declare hands are the chief architect,
which chains claude and codex (fable, sol, opus, astra) and is seated by
`recipes/triage`'s specify and design steps and, through inheritance,
by `recipes/night-shift`, and the sdd intake, which chains claude alone
(sonnet, opus) and is hired by no shipped bundle — `recipes/sdd` folded
into `recipes/triage`'s strategy select in #176 and no longer exists.
So the promise holds, for every shipped bundle without a dialect step,
exactly when `adapters/codex.json` and `adapters/claude.json` declare
`hands.harness.gate` and `hands.harness.work` as fragments, each `gate`
with a measured door; `recipes/triage` and `recipes/night-shift`, the
two with dialect steps, refuse under `harness` on a second ground until
a decision admits the step (design DD8), and the implementation SHALL
report that unmet part of the promise beside the measurement. Every
bundle under `recipes/` and `bundles/` without a dialect step SHALL
compile under `harness` on that condition. If the measurement finds a
claude mode that cannot be declared as a fragment, the refusal SHALL
name the adapter, the member and the site, and the implementation SHALL
report ruling 6's promise as unmet for the operator to rule on — never
widen the rule, seat another provider or declare an unmeasured fragment
to make the shipped bundles compile. The implementing seat cannot make
the claude measurement — its tool grant is `cargo` and `git` — so until
the operator records it the members are undeclared and the same refusal
applies by name; the implementation SHALL finish every other task and
commit, SHALL NOT report itself blocked for want of the measurement,
and SHALL name the measurement as the operator's in its completion note
with the recipe, the candidates and the version. The tree's own proof
of the promise SHALL therefore run in a scratch copy of the shipped
adapter library with the two members planted as fragments, and the same
test's second half SHALL pin, against the shipped adapters as they
stand, exactly which shipped bundles refuse and, per bundle, the ground
the compiler reaches first — one test, two halves, the second a pin
that moves for a known reason: the measurement landing, or a decision
admitting the dialect step (design DD20; decision 0046 rulings 4 and 6;
decision 0042's addendum, ruling 1: a decision is amended only by a
decision).

#### Scenario: The shipped bundles compile under harness
- **GIVEN** an adapter library in which the codex and claude adapters declare both `hands.harness` members as fragments — the shipped files once the measurement lands, a scratch copy with the members planted until then
- **WHEN** every bundle under `recipes/` and `bundles/` compiles under `harness` against it, in a realm that declares the openspec dialect
- **THEN** each bundle without a dialect step compiles — eleven of the thirteen — with each hands site's manifest `boundary` entry reading `harness`, and `recipes/triage` and `recipes/night-shift` refuse naming the `analyze` sequence's `check` step — the first dialect step the compiler reaches, phases compiling in name order — and decision 0046 ruling 4

#### Scenario: The measurement is not reachable from the implementing seat
- **WHEN** the shipped adapters are loaded as they stand, `adapters/claude.json` declaring no `hands.harness` member because no claude the implementing seat may run was reachable, and every bundle under `recipes/` and `bundles/` compiles under `harness` in a realm that declares the openspec dialect
- **THEN** exactly four bundles refuse, each naming the ground the compiler reaches first: `bundles/self` at `review`, whose reviewer chains `sol`, `fable`, `opus`, and `recipes/panel-review` at `review:correctness`, whose judge chains `sol`, `opus`, each naming `claude`, `hands.harness.gate` and the site; and `recipes/triage` and `recipes/night-shift`, which inherits the seat, naming the `analyze` sequence's `check` step, decision 0046 ruling 4 and decision 0042 ruling 4 — the compiler walks phases in name order (`serde_json::Map` is a `BTreeMap` in this tree; `preserve_order` is off), so `analyze` compiles first and its first step is the dialect check, reached before any claude link, and the claude ground stands behind that refusal unreached; the pin names `claude` for those two only if a decision admits the dialect step before the measurement lands; every other shipped bundle compiles; and the implementation completes and commits every other task, reports nothing blocked, and names the measurement in its completion note as the operator's with the recipe, the candidates and the version

#### Scenario: A measured gap is reported, not papered over
- **WHEN** the measurement declares claude's `work` member unsupported
- **THEN** a fixture bundle that seats the chief architect as a work seat with hands refuses under `harness` naming `claude`, `hands.harness.work` and the site; `recipes/triage` and `recipes/night-shift` — every shipped bundle under `recipes/` and `bundles/` that seats the chief architect — refuse under `harness`; every other shipped bundle still compiles; and the implementation reports the unmet promise instead of amending the adapter, the roster or the rule
