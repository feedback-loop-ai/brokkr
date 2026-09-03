# 0042 — The council is Brokkr's, the dialect is the realm's: spec-driven delivery speaks OpenSpec or spec-kit, and the tool validates

Status: proposed (drafted at the operator's direction in chat, 2026-09-04: "draft 0042 for the SDD dialects too"; amended the same day on the operator's observation that the frameworks are machines)
Date: 2026-09-04

## Context

The operator's question on 2026-09-03: "the SDD was meant to be
spec-kit, and it is not, seems like OpenSpec. Maybe we need two separate
flows? Or abstracting the specification with two adapters?" — with the
standing constraint that spec tools stay leaf effects and the machine
keeps its modus operandi. Decision 0041 deferred the answer to its own
number. This is it.

**What `recipes/sdd` does today, read from the tree.** Intake, then a
design sequence — two contrarian positions, a chief that synthesises,
a validator — then implement, verify, a review panel and ship. The
council is the good part: neither methodology has one, and a design
argued by two positions and ruled by a fresh chief is the reason to run
this recipe at all. What the chief is told to write is the problem. Its
charter names four files: `specs/<slug>/spec.md`, `plan.md` and
`tasks.md` "following spec-kit conventions", and
`openspec/changes/<slug>/proposal.md` with OpenSpec's three headings.
The validator, `recipes/sdd/drivers/speckit_check.sh`, finds the
feature directory by newest modification time, checks the four files
exist, greps for an acceptance heading, a checkbox and a why-heading,
and folds in `specify check` — which, in spec-kit 0.8.7, probes that
tools are installed and validates nothing. The implementer and the
spec-compliance reviewer are told to read "the newest `specs/`
directory for this run". No `.specify/` was ever scaffolded here, and
the repository's `openspec/` tree holds eight change directories with a
proposal each and no `openspec/specs/` truth for any of them to change.
The recipe is a home-grown design council writing spec-kit-shaped files
beside an OpenSpec-shaped proposal, validated by a heading grep: neither
dialect, and honest about being neither only in its script's comments.

**What the two tools actually are, measured on 2026-09-04.** Spec-kit
0.8.7 (`specify`) was scaffolded into a scratch directory; OpenSpec
1.12.0 (`openspec`, not installed here, fetched by `npx`) was
initialised in another and a change created, inspected and validated.

| Fact | spec-kit 0.8.7 | OpenSpec 1.12.0 |
|---|---|---|
| scaffold | `.specify/` — templates, bash scripts, `memory/constitution.md`, a workflow runner — and nine `speckit-*` skills for the harness | `openspec/config.yaml` (`schema: spec-driven`) and `openspec/specs/` |
| unit of work | `specs/NNN-slug/`, created together with a git branch of the same name by `create-new-feature.sh` | `openspec/changes/<id>/`, created by `openspec new change <id>` |
| artifacts | `spec.md` (user stories, functional requirements, success criteria), `plan.md` (technical context, constitution check, structure), `tasks.md` (`[ID] [P?] [Story]` rows in phases), optional research, data model, contracts, quickstart, checklists | `proposal.md`, `specs/<capability>/spec.md` deltas, `design.md`, `tasks.md`, in a dependency order the tool tracks (`openspec status`) |
| where the truth lives | the feature directory; no living spec tree | `openspec/specs/<capability>/spec.md`, updated by archive |
| constitution | `.specify/memory/constitution.md` | none shipped; conventions are the project's |
| instructions for the author | templates in the repository; skills for the harness | `openspec instructions <artifact> --change <id>` renders them, output path included |
| validator | `check-prerequisites.sh` checks the directory, `plan.md`, `tasks.md` and the branch name exist; `specify check` probes tools; content consistency is a skill, `/speckit-analyze` | `openspec validate <id> --strict`, JSON on request, `--archived` for a completed change's tasks |
| close-out | none; the directory is the record | `openspec archive <id> --yes` folds the deltas into the truth; `--skip-specs` for a change without spec deltas |
| workflow runner | `specify workflow run` drives a harness through specify, plan, tasks, implement with approve-or-reject gates | slash skills, `/opsx:*` |

**The frameworks are machines.** The operator's observation the same
day, and the measurement bears it out: "those two spec frameworks are
FSMs on their own, and there is a generic SDD FSM as well." OpenSpec
says so in its own vocabulary. Its `spec-driven` workflow is a
*schema* — `openspec schemas` lists it, `openspec schema` manages them —
a graph of artifacts with dependencies, and `openspec status --change`
reports the state: proposal open, specs and design blocked by proposal,
tasks blocked by specs and design. Spec-kit's `workflow.yml` is a step
list with gates: `specify`, a review gate that approves or rejects,
`plan`, a second gate, `tasks`, `implement`. Above both sits the
machine every framework instantiates: state the what, then the how,
then the work, then build, verify, review, ship. The operator named the
mapping the same day: "specify (the what), design (the high-level how)
and task breakdown (the low-level how)" — and asked where the
frameworks' own sub-machines go, which ruling 3 answers with steps.

The first draft of this decision flattened all of that into one
`design` phase whose chief wrote every artifact in a single pass and one
validator at the end. That hides a framework's machine inside a
charter, which is the one thing this repository refuses to do with its
own: policy is data (0004), a phase is a journaled fact with its own
bounds (0006), and a machine hidden in prose cannot be replayed, bounded
or returned to. The rulings below put the machine in the table and make
the dialect a map from the framework's graph onto it.

Three things fall out of the measurements. Both tools are artifact
conventions plus prompts for a harness, and a Brokkr seat is a harness:
the seat can read the templates or ask the tool for its instructions,
and neither tool's own runner belongs inside a run that already has
typed gates. Only OpenSpec ships a validator worth the name; spec-kit's
checks are existence checks, and its consistency check is a prompt. And
the two have different shapes of truth: OpenSpec keeps a living
specification a change amends and archives into — the shape of this
repository's own decision record, which deliberates, rules, encodes,
enforces and amends — while spec-kit keeps one directory per feature
with a constitution above them, the shape of greenfield feature work
and of the artifacts the layer above this machine produces.

**Where the choice belongs.** A recipe is a strategy: how a delivery
advances. A dialect is a convention of the repository being delivered
to: where its specifications live and what shape they take. Running a
spec-kit recipe against an OpenSpec repository is a category error, and
today nothing but the operator's memory prevents it. Decision 0041
ruling 8 already gives the realm a `house` file; the dialect is the
same kind of fact about the same object. And under 0041 ruling 7 the
strategy is a case a triage seat selects inside one bundle, so a
dialect carried by the recipe would have no recipe to ride on.

Alternatives weighed:

- **Two recipes, `sdd-speckit` and `sdd-openspec`, by composition.**
  Decision 0017 makes them cheap and they need no engine change. Rejected
  for the reason above: the choice would sit in the wrong object, the
  operator would pick it by hand every run, and it dissolves the moment
  0041 ruling 7 lands.
- **One design phase, every artifact in a pass.** The first draft.
  Rejected on the operator's observation: it hides the framework's
  machine, so a crash while writing tasks re-authors the spec, a wrong
  spec cannot be returned to without re-doing the design, and the
  journal records one effect where the framework has three states.
- **Keep the hybrid, validate harder.** Rejected: a layout that is
  neither dialect can be validated by neither tool, so the validator
  stays a grep, and the repository's own `openspec/` tree stays a set of
  proposals that change nothing.
- **Run the tools' own workflows inside seats** — `/speckit-plan`,
  `/opsx:propose`, `specify workflow run`. Rejected: a harness inside a
  harness, with gates that are prompts inside gates that are typed. The
  seat may read a template or ask the tool for its instructions; it does
  not hand its turn to another orchestrator.
- **A dialect adapter in Rust with a match arm per tool.** Rejected by
  0016's rule: adapters are data, never match arms, and a capability a
  tool lacks is declared as the string `unsupported` with its reason.

## Rulings

1. **The council is the constant; the dialect is data the realm
   declares.** `forge.realms/v3` — the version decision 0041 ruling 8
   opens for `house` — gains one more optional field per realm,
   `dialect`: the name of a dialect in Brokkr's `dialects/` library, or
   a repository-relative path to a dialect file of the same shape. A
   bundle whose table has an artifact phase (ruling 2) compiles only
   against a realm that declares one; the refusal names the realm and
   says the phase needs a dialect. The resolved dialect is pinned by
   content digest in the run manifest beside the adapters, so a change
   to a dialect moves the digest of every run that designs under it.

   **Enforcement binding:** `contracts/realms.v3.schema.json` and the
   frozen-contracts test; the dialect loader in `brokkr-runtime`; the
   compile refusal and its test; the manifest pin.

2. **The SDD machine is a table, and it has three artifact phases.**
   `recipes/sdd`'s table carries `specify` (the what), `design` (the
   high-level how) and `tasks` (the low-level how: the work breakdown)
   ahead of implement, each a phase of its own: its own seat, attempts
   and deadline, a typed result, and a dialect-supplied validate step as
   its final step (ruling 4). Each phase's seat is a sequence the engine
   composes from the dialect's steps for that phase (ruling 3), the
   validate step last. `specify` seats the chief alone. `design` seats the council —
   positions in parallel, then the chief. `tasks` seats the smith: the
   implementer plans its own work, and the dialect checks the plan before
   a line is built. Each phase reports `drafted`, `fail`, or `upstream`
   — the artifact above is at fault — and the table routes them:
   `drafted` advances; `fail` re-enters the phase once, then stops;
   `upstream` returns to the previous artifact phase under a visits
   bound, and from `specify` it parks, because the commission itself is
   at fault and that is triage's office (0041 ruling 6). Decision 0041
   ruling 5c's edge — the spec-compliance judge's `spec_defect` — lands
   on `specify`, the earliest artifact, and the chain re-runs under the
   same bound. A framework's human gates are table rules: a gated
   variant may park after `specify` and after `design`; the unattended
   variant parks nowhere.

   **Enforcement binding:** `recipes/sdd/policy.json`;
   `crates/brokkr-runtime/tests/sdd_shape.rs` (new): three artifact
   phases precede implement, each ends in a boxed validate step, each
   `upstream` edge lands one phase earlier and is bounded, the
   protected review gate is unchanged; a table test per arm in
   `brokkr-core`.

3. **A dialect maps its framework's graph onto the table, and the
   compiler checks the map.** `contracts/dialect.v1.schema.json` closes
   the vocabulary: `tool` (binary and the version measured); `requires`
   (repository files that must exist before an artifact phase may run);
   `change` (the change's path with `{change}`); `truth` (the living
   specification tree, or `unsupported`); `phases` — for each of
   `specify`, `design` and `tasks`: its `steps`, an ordered list in
   which each step names the artifacts one session writes, the office
   that holds it (`chief`, `council`, `smith`, or `check` for a
   read-only judge), whether it is optional, and the instructions the
   seat is rendered; and the phase's `validate` argv or `unsupported`
   with the reason; `order` (the framework's own dependency edges
   between artifacts); `verify` and `archive` (argv with `{change}`, or
   `unsupported`); and `house` (the dialect's own constitution path, or
   `unsupported`). Every artifact is assigned to exactly one step of
   exactly one phase, every phase has at least one required step, and
   the order of steps across the three phases must be a linear extension
   of `order`: a map that places an artifact before one it depends on
   is refused at compile time, naming both. A framework that cannot fill
   a phase is refused rather than served a no-op. Two ship:

   | | `dialects/openspec.json` | `dialects/speckit.json` |
   |---|---|---|
   | `tool` | `openspec` 1.12.0 | `specify` 0.8.7, init-time only |
   | `requires` | `openspec/config.yaml` | `.specify/templates/spec-template.md`, `.specify/scripts/bash/check-prerequisites.sh` |
   | `change` | `openspec/changes/{change}` | `specs/{change}`, `{change}` being `NNN-slug` |
   | `truth` | `openspec/specs` | `unsupported`: the feature directory is the record |
   | `specify` steps | `propose` — chief: `proposal.md`, then the `specs/<capability>/spec.md` deltas it declares, one session | `specify` — chief: `spec.md`; `clarify` is `unsupported` inside a run: the questions it would ask become the template's Assumptions section, recorded |
   | `design` steps | `design` — council: positions, then the chief writes `design.md` | `plan` — council: positions, then the chief writes `plan.md` with the research, data model, contracts and quickstart the template asks for; `checklist`, optional — chief: `checklists/*.md` |
   | `tasks` steps | `tasks` — smith: `tasks.md` | `tasks` — smith: `tasks.md`; `analyze`, optional — check: a read-only cross-artifact consistency judge, spec-kit's `/speckit-analyze` held by a gate seat |
   | after implement | `archive` — the smith's fold (`archive`, below) | none: the directory is the record |
   | `order` | proposal → specs, proposal → design, specs → tasks, design → tasks | spec → plan → tasks |
   | `validate`, per phase | `openspec validate {change} --strict --no-interactive`, and `openspec status --change {change} --json` must show the phase's artifacts complete | `check-prerequisites.sh` for the files it knows, and Brokkr's own check of the template headings, declared as Brokkr's |
   | `verify` | `openspec validate --archived --strict --no-interactive` | `unsupported` |
   | `archive` | `openspec archive {change} --yes` | `unsupported`: spec-kit has no close-out |
   | `house` | `unsupported`: the realm's `house` file is the constitution | `.specify/memory/constitution.md` |

   Two granularities, both data, both journaled. The table holds the
   generic machine — phases, rules, returns, bounds — as transitions.
   The dialect's steps hold the framework's own sub-machine — composed
   into each phase's sequence, journaled as checkpoints, checked against
   the framework's `order` across the whole run. A step is the unit of a
   session: a dialect groups artifacts into one step where one session
   should write them in order, as OpenSpec's proposal and the spec
   deltas it declares, because the validator checks per artifact
   regardless. An optional step runs only when a recipe asks for it —
   `"steps": "full"` on the phase's seat; `sdd-paranoid` asks — so the
   framework's optional states cost a session only where they are
   wanted. A return lands on a phase, never inside one, because that is
   the framework's own granularity of truth: `openspec status` knows
   artifacts, not half-written ones. OpenSpec's graph lets specs and
   design proceed in parallel after the proposal; the table linearises
   them, specify then design, which the graph permits. Creating the
   change is the `specify` seat's first act
   — `openspec new change {change}` where the tool has it; the numbered
   directory from the templates where it does not, because spec-kit's
   `create-new-feature.sh` also creates a git branch and a run already
   stands on one.

   **Enforcement binding:** the two dialect files; the schema; loader
   tests that refuse an unknown field, an unfilled phase and an order
   the table violates, and accept `unsupported` where the schema allows
   it; `brokkr doctor` reports the tool, its version against the pin,
   and each `requires` file.

4. **The validator is the tool's, per phase, run as a boxed exec step;
   where the tool has none, the check is Brokkr's and says so; and the
   framework's own state is evidence.** Each artifact phase's final
   step is `{"name": "validate", "dialect": "validate"}` and resolves to
   that phase's argv from the dialect, boxed under decision 0040 ruling
   3, class gate, its typed result `drafted` or `fail` with the tool's
   findings as notes — and, where the tool reports state, that state
   recorded verbatim beside them, so the journal carries the framework's
   view of the change next to Brokkr's. Where a dialect declares
   `verify`, the verify seat's boxed exec runs it after the suite, so a
   folded change with an unticked task fails verify rather than review.
   The mtime search is deleted.

   **Enforcement binding:** the `dialect:` site form in
   `crates/brokkr-runtime/src/bundle.rs`; the engine's argv expansion;
   `sdd_shape.rs`: every validate step is boxed and a gate,
   `speckit_check.sh` is gone; a test that a validate step's notes
   carry the tool's status output when the dialect declares one.

5. **The handoff is typed and journaled.** `change` joins the input
   vocabulary as an identifier kind — a string matching
   `^[a-z0-9][a-z0-9._-]*$` — that a seat may declare and a rule may
   never test. The `specify` seat's result carries it, every validate
   step echoes it, and each artifact phase's result carries it into the
   journal. The engine exposes to every seat `context.results.<phase>`:
   the `result` and declared `inputs` of the last successful effect of
   each earlier phase — typed facts, never `notes`, which stay in the
   journal where prose belongs. A `{change}` token in a dialect argv
   expands from the nearest preceding result that carries the input, the
   previous step inside a sequence before the latest phase. No seat
   discovers a change by modification time or by reading a directory
   listing.

   **Enforcement binding:** `IDENTIFIER_INPUTS` in
   `crates/brokkr-core/src/policy.rs` and the decision 0007 lint in
   `bundle.rs`; context assembly and token expansion in the engine;
   tests that an undeclared `change` is dropped, that `context.results`
   carries no notes, and that the token refuses to expand when nothing
   carries the input.

6. **Under a dialect the offices keep their walls.** The chief authors
   the `specify` and `design` artifacts and commits exactly those,
   reading the dialect's own instructions through its hands —
   `openspec instructions <artifact> --change {change}` or the templates
   under `.specify/templates/` — never the tool's runner. A commission
   that names an existing change is adopted, not re-authored: the
   `specify` seat validates what was handed in and amends with reasons
   or leaves it, the positions argue against it, and every validator
   runs regardless. The smith writes `tasks`, works them and ticks them,
   and where the dialect declares `archive`, folds the change into the
   truth as its last task, so the pull request carries the code and the
   truth it changes in one head; a smith returned under 0041 ruling 5
   reopens the archived change with `git mv` before amending and folds
   it again. A change that alters no specified behaviour says so in the
   dialect's own words — OpenSpec's `skip_specs` — rather than inventing
   a requirement to pass validation. The spec-compliance judge compares
   the diff to the change's artifacts as the dialect shapes them,
   scenarios or success criteria, and may rule `spec_defect` (0041
   ruling 5c); no gate touches an artifact (0041 ruling 4). The charters
   carry none of this: the dialect's per-phase instructions do, rendered
   by the engine under `## Spec dialect` into the seat that holds the
   phase and into the review panel.

   **Enforcement binding:** the dialect instructions;
   `implementer-speckit` and `intake-speckit` renamed `implementer-sdd`
   and `intake-sdd` and made dialect-free; the `specify` tool grant
   leaves every agent; the roster test of 0041 refuses a library charter
   naming `specs/`, `openspec/` or `.specify/`.

7. **This repository's realm speaks OpenSpec, and the hybrid retires.**
   `realms.json` declares `"dialect": "openspec"`; `openspec init`
   scaffolds `openspec/config.yaml` and `openspec/specs/`. The eight
   change directories under `openspec/changes/` and the eight feature
   directories under `specs/` are the record of the runs that wrote
   them and move, by `git mv` with history intact, under
   `docs/evidence/sdd-2026-08/`; the journal cites their commits, not
   their paths, and every citation stays true. New spec-driven work
   here writes OpenSpec changes only. The reason is shape: a proposal
   that amends a living specification and is archived into it is what
   this repository already does with its decisions, one directory over.
   Spec-kit is shipped for the repositories whose work is greenfield
   features under a constitution, and for the artifacts the layer above
   hands down in that shape.

   **Enforcement binding:** `realms.json`; the moves; the sdd witness
   pin; `brokkr doctor` green on this realm.

8. **`brokkr init` detects the dialect and `doctor` reports it.** A
   `.specify/` directory declares spec-kit; `openspec/config.yaml`
   declares OpenSpec; both present, init refuses to guess and asks;
   neither, init writes no dialect and says an artifact phase will need
   one. `doctor` prints the realm's dialect, the tool it found and the
   version it measured against the pin.

   **Enforcement binding:** `detect` in `crates/brokkr-cli/src/init.rs`
   and its tests; `crates/brokkr-cli/src/doctor.rs`.

## Consequences

- **What dies.** The single design pass and its four hard-coded paths;
  `speckit_check.sh` and its role file; discovery by modification time;
  the two `-speckit` agent names and the `specify` grant; the hybrid
  layout, into the evidence tree with its history.
- **What it costs.** Three artifact seats where there was one: a
  chief's pass on `specify`, the council on `design`, the smith's pass
  on `tasks`, each with a boxed validate step. Each is smaller than the
  single pass it replaces and each is bounded, journaled and returnable
  on its own, which is what the extra sessions buy. A framework's
  optional states — spec-kit's checklist and analyze — cost a session
  each and run only when a recipe asks. The tool must be
  installed on the machine that runs an artifact phase — Node for
  OpenSpec — and `doctor` says so before a run does. A dialect's version
  is a pin: bumping it moves the digest of every bundle that designs,
  which is the point.
- **How the two decisions compose.** Under 0041 ruling 6 a `chore`
  never enters an artifact phase, so a chore creates no change under
  either dialect, which is exactly what OpenSpec's `skip_specs` and
  spec-kit's absence of a chore artifact both say. Under 0041 ruling 7
  the sdd recipes become the `design` and `engine` cases of
  `recipes/triage`, and the dialect they write in comes from the realm,
  so one bundle designs correctly in two repositories that speak
  different dialects. Until ruling 7 lands, `recipes/sdd` stays a recipe
  and is dialect-free.
- **Enactment, in order, after 0041's first slice moves the agent
  files.** (i) The dialect schema and library, the realm field, the
  loader and its map check, the `dialect:` site form, the identifier
  input and the typed handoff — one engine slice; `forge.realms/v3` is
  opened by whichever of 0041 ruling 8 and this ruling 1 lands first and
  carries both fields. (ii) The three-phase table, the per-phase
  instructions, the charters, the renames, and this repository's own
  migration under ruling 7. (iii) `init` and `doctor`.
- **Deliberately unruled.** The gated variant's parks are approvals,
  and the operator command vocabulary has `retry` and `stop` but no
  `proceed`; a variant that waits for approval wants that command, and
  its own ruling. Which phase the council sits on is data — moving the
  positions from `design` to `specify` is a seat edit — and the default
  here is a judgment, not a measurement. Custom OpenSpec schemas
  (`openspec schema`, experimental) and stores; a realm that speaks two
  dialects; a dialect wager — the same commission designed under both,
  compared by artifacts — which the wager harness could run once ruling
  1 lands and which would be the evidence for changing ruling 7's
  default.
