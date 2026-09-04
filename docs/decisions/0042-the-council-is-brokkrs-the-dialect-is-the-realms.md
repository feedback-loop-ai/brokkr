# 0042 — The council is Brokkr's, the dialect is the realm's: spec-driven delivery speaks OpenSpec or spec-kit, and the tool validates

Status: accepted (operator ruled in chat, 2026-09-04; drafted the same day at the operator's direction, "draft 0042 for the SDD dialects too", and amended five times in the same conversation on the operator's observations)
Date: 2026-09-04

## Context

The operator's question on 2026-09-03: "the SDD was meant to be
spec-kit, and it is not, seems like OpenSpec. Maybe we need two separate
flows? Or abstracting the specification with two adapters?" — with the
standing constraint that spec tools stay leaf effects and the machine
keeps its modus operandi. Decision 0041 deferred the answer to its own
number. This is it. As in 0041, "decision 0040" here means the
boxed-hands decision, drafted under that number and landed as 0043
after the flag-grammar decision took 0040 on main.

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
| what a script can count | `[NEEDS CLARIFICATION: …]` markers in the spec; `check-prerequisites.sh` knows which files exist | `validate --strict` names the requirement that lacks a scenario; `show --json --deltas-only` parses every delta into capability, operation, requirement text and scenarios; `status --change --json` reports each artifact's completion; the proposal template's Capabilities section is, in the tool's own words, the contract with the specs phase; the design template has a Decisions section and no open-questions section; the tasks template is numbered groups with no requirement references |
| clarify and analyze | both, as skills: `speckit-clarify` scans a taxonomy, asks at most five questions a session and encodes the answers into the spec; `speckit-analyze` is strictly read-only, treats the constitution as non-negotiable, reports six dimensions with four severities | neither. `explore` is a thinking partner for a human before proposing; `update` is an author revising a change's artifacts to keep them coherent; `validate --strict` is structural — at least one delta, delta headers, a scenario per requirement. No read-only judge, no zero exit, no constitution |

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
and task breakdown (the low-level how)" — asked where the frameworks'
own sub-machines go, which ruling 3 answers with steps, and ruled the
two states a first draft had marked optional: "analyze is not optional,
analyze to zero is mandatory (analyzing N amount of times until there
is no constitutional drift), and n amount of clarify until there is no
more ambiguity." Measured against spec-kit's skills the same day: its
clarify skill scans a taxonomy, asks at most five questions a session
and encodes the answers into the spec; its analyze skill is strictly
read-only, treats the constitution as non-negotiable, and reports six
dimensions with four severities. Ruling 2 makes both generic states of
the machine. The operator then asked whether that is generic at all,
and what the design would do for OpenSpec, which ships neither skill.
The honest answer: the offices, the zero exit and the bound are
generic; the teeth are the dialect's, and a first draft gave OpenSpec
none, which decision 0025 ruling 6 forbids where a count is possible.
OpenSpec, measured, gives a script more to count than spec-kit does,
and ruling 3 now spends it.

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

## Ruling — 2026-09-04, operator: accepted as proposed

Accepted in chat the day it was drafted ("accept 0041 and 0042"),
after five amendments the operator drove in the same conversation: the
frameworks are machines, so the artifact phases are three; their
sub-machines are the dialect's steps; clarify and analyze are judged
loops that exit only at zero; OpenSpec ships neither, so under it the
loops are Brokkr's offices; and each loop has a deterministic half
counted from the framework's own data before the judge reads. The
eight rulings and their enforcement bindings stand as written and are
the commission of the enactment slices in the order the consequences
record, after 0041's first slice. What the consequences leave unruled
— a `proceed` operator command, how an answer reaches a returning
seat, which phase the council sits on, custom schemas, a dialect wager
— stays unruled here.

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

2. **The SDD machine is a table: three artifact phases and two judged
   loops that exit only at zero.** `recipes/sdd`'s table carries
   `specify` (the what), `clarify`, `design` (the high-level how),
   `tasks` (the low-level how: the work breakdown) and `analyze` ahead
   of implement, each a phase of its own: its own seat, attempts and
   deadline, a typed result, and — for the three artifact phases — a
   dialect-supplied validate step as its final step (ruling 4). Each phase's seat is a sequence the engine
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
   same bound.

   The two loops are the operator's ruling in his own words: "analyze
   to zero", and "n amount of clarify until there is no more
   ambiguity". `clarify` seats a read-only judge that scans the
   specification for ambiguity — the taxonomy spec-kit's skill names:
   underspecified areas, unquantified adjectives, missing decision
   points, markers left open — and reports `clear` only when its list
   is empty. Otherwise it reports `ambiguous`, every open question in
   its notes with the evidence that would settle it, and the table
   returns the run to `specify`, where the chief answers from the
   commission, the house rules and the tree, and records what it
   assumed and why. `analyze` seats a read-only judge that reads the
   three artifacts and the realm's house file and reports drift —
   duplication, ambiguity, underspecification, constitution alignment,
   coverage gaps and inconsistency: Brokkr's six dimensions, as
   spec-kit's analyze skill first wrote them down, with its severities
   — and reports `consistent` only at zero findings. Each loop has a
   deterministic half and a judged half, in that order: a boxed `check`
   step counts what the framework's own data lets a script count
   (ruling 3), and the judge rules only on what no script can. The back
   edge carries the judge's finding — result, inputs and notes — as
   `returned_from`, the way decision 0022 hands a review's finding to
   the returning smith; the forward handoff of ruling 5 stays typed
   facts only. The returning author answers every item inside the
   artifact, at the place the dialect names for decisions: an answer
   becomes the artifact's own text — under OpenSpec a scenario, because
   an ambiguity there is a missing scenario — and a refusal is recorded
   with its reason. The next judge, fresh and blind by design, reads
   the refutation as part of the artifact instead of raising the
   finding again; a judge that re-raises a recorded refutation without
   new evidence has made a finding about the record, not the artifact,
   and says so. Otherwise it reports `drift`, with a declared enumerated
   input `drift_in` naming the earliest artifact at fault, and the
   table returns the run to that phase; a return to `specify` re-runs
   the whole chain. Both loops are bounded by their own visits the way
   0022 bounds reforging: three passes, then the run parks with the open
   questions or the residual drift as its reason, which is the
   operator's door. A question no evidence can answer parks the same
   way from `specify`, the questions themselves as the reason, and the
   operator's answer returns with the resume. Neither judge writes a
   file (0041 ruling 4). The constitution is non-negotiable within the
   analysis, as spec-kit's skill puts it, and here the constitution is
   the realm's house file. The loops belong to the generic machine, not
   to a dialect: a framework that ships a skill for them contributes its
   taxonomy and its marker count (ruling 3); one that ships none is
   judged by the same offices on the same terms. OpenSpec ships none,
   measured: its `explore` is a conversation with a human before
   proposing, which the park replaces in an unattended run; its `update`
   is an author keeping artifacts coherent, which is the chief's return
   to `specify`, not a judge; and its strict validation is structural.
   Under the OpenSpec dialect the judges therefore work from Brokkr's
   own taxonomy, the six dimensions spec-kit's skill spells out, and
   the constitution is the house file because the framework has none. A framework's human
   approval gates are table rules a gated variant may add after
   `specify` and `design`; the unattended variant has none, and the
   parks above are where a human enters it.

   **Enforcement binding:** `recipes/sdd/policy.json`;
   `crates/brokkr-runtime/tests/sdd_shape.rs` (new): three artifact
   phases and two judged loops precede implement, each artifact phase
   ends in a boxed validate step, each `upstream` edge lands one phase
   earlier and is bounded, `clarify` advances only on `clear` and
   `analyze` only on `consistent`, each loop's exhaustion parks, the
   protected review gate is unchanged; `drift_in` joins the enumerated
   input kinds as seat-declarable over the artifact phase names; a table
   test per arm in `brokkr-core`.

3. **A dialect maps its framework's graph onto the table, and the
   compiler checks the map.** `contracts/dialect.v1.schema.json` closes
   the vocabulary: `tool` (binary and the version measured); `requires`
   (repository files that must exist before an artifact phase may run);
   `change` (the change's path with `{change}`); `truth` (the living
   specification tree, or `unsupported`); `phases` — for each of
   `specify`, `design` and `tasks`: its `steps`, an ordered list in
   which each step names the artifacts one session writes, the office
   that holds it (`chief`, `council`, `smith`, or `check` for a
   read-only judge), whether it is optional, the instructions the seat
   is rendered, and the instructions it is rendered when a judge
   returns it; and the phase's `validate` argv or `unsupported` with
   the reason; for `clarify` and `analyze`, the taxonomy rendered to the
   judge and a `check` argv — the deterministic half of the loop, the
   framework's own count where it has one and Brokkr's boxed script
   over the framework's data where it does not, `unsupported` only
   where nothing can be counted; `decisions` (where the artifacts record
   the loops' answers and refutations); `order` (the framework's own
   dependency edges between artifacts); `verify` and `archive` (argv with `{change}`, or
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
   | `specify` steps | `propose` — chief: `proposal.md`, then the `specs/<capability>/spec.md` deltas it declares, one session | `specify` — chief: `spec.md`, with the template's `[NEEDS CLARIFICATION: …]` marker wherever the commission leaves a decision open |
   | `clarify` check | Brokkr's boxed script over the tool's own data: `openspec validate --strict` (a scenario per requirement, refused by name); the proposal's Capabilities section against the delta files (every named capability has a delta, every delta is named); requirements with a single scenario listed as candidates for the judge, not as findings | the marker count over the feature directory must be zero — the framework's own marker, counted deterministically beside the judge's list |
   | `design` steps | `design` — council: positions, then the chief writes `design.md` | `plan` — council: positions, then the chief writes `plan.md` with the research, data model, contracts and quickstart the template asks for; `checklist`, optional — chief: `checklists/*.md` |
   | `tasks` steps | `tasks` — smith: `tasks.md`, every task citing the requirement it serves by name, since the template carries no references | `tasks` — smith: `tasks.md` |
   | `analyze` check | Brokkr's boxed script: modified, removed and renamed requirements exist in the truth and an added one does not already exist by name (`openspec show --json --deltas-only` against `openspec spec show`); every delta requirement is cited by at least one task; `openspec status --change --json` shows the artifacts complete | `check-prerequisites.sh --require-tasks`; the `speckit-analyze` skill's coverage table, every requirement mapped to a task |
   | `analyze` taxonomy | Brokkr's six dimensions; the judge rules on near-duplicates, ambiguity inside a requirement's text, terminology drift, design-to-tasks semantics, and alignment with the house file, which stands in for the constitution the framework lacks | the `speckit-analyze` skill's six dimensions and its CRITICAL, HIGH, MEDIUM and LOW severities, rendered to the judge; `.specify/memory/constitution.md` as the constitution |
   | `decisions` | `design.md`, `## Decisions`, which the template already has; an answered ambiguity becomes a scenario in the delta | `spec.md`, `## Clarifications`, which the clarify skill writes with dated answers |
   | return instructions | the `openspec-update-change` skill: revise the artifacts and keep them coherent | the clarify skill's encoding rules: a functional ambiguity becomes a requirement, the obsolete statement is replaced, no contradiction is left |
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
   regardless. An optional step — spec-kit's checklist — runs only when
   a recipe asks for it, `"steps": "full"` on the phase's seat, which
   `sdd-paranoid` does; the loops are never optional. A return lands on a phase, never inside one, because that is
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
   ruling 5c); no gate touches an artifact (0041 ruling 4). Two gate
   offices join the library for the loops, read-only like every judge:
   `analyst`, hired at the frontier — fable-5-1 at xhigh, then opus-5 at
   xhigh — because a drift it misses goes straight into code; and
   `clarifier`, one tier cheaper — opus-5 at xhigh, then sol at xhigh —
   because an ambiguity it misses surfaces at analyze or at implement,
   where the machine still has a way back. That is 0041's rule applied:
   spend where the error is invisible downstream. The charters carry none of
   this: the dialect's per-phase instructions do, rendered by the engine
   under `## Spec dialect` into the seat that holds the phase and into
   the review panel.

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
  the idea that analysis and clarification are optional;
  `speckit_check.sh` and its role file; discovery by modification time;
  the two `-speckit` agent names and the `specify` grant; the hybrid
  layout, into the evidence tree with its history.
- **What it costs.** Three artifact seats where there was one: a
  chief's pass on `specify`, the council on `design`, the smith's pass
  on `tasks`, each with a boxed validate step. Each is smaller than the
  single pass it replaces and each is bounded, journaled and returnable
  on its own, which is what the extra sessions buy. A clean
  design-class chain is seven sessions before a line of code: the chief
  on specify, the clarifier, two positions and the chief on design, the
  smith on tasks, the analyst. Each pass of a loop adds a judge session
  plus the artifact phase it returns to, and the bound caps a loop at
  three passes; the deterministic checks cost nothing but a boxed
  script. Spec-kit's checklist stays optional and
  costs a session only when a recipe asks. The tool must be
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
  its own ruling. The same vocabulary carries no answer: when `clarify`
  parks on a question no evidence settles, the operator's answer must
  reach the returning `specify` seat, and whether that is a note on the
  resume command or a file the seat is pointed at is the enactment's
  to decide and record. Which phase the council sits on is data — moving the
  positions from `design` to `specify` is a seat edit — and the default
  here is a judgment, not a measurement. Custom OpenSpec schemas
  (`openspec schema`, experimental) and stores; a realm that speaks two
  dialects; a dialect wager — the same commission designed under both,
  compared by artifacts — which the wager harness could run once ruling
  1 lands and which would be the evidence for changing ruling 7's
  default.

## Addendum — 2026-09-04, the operator's rulings: the decision is authoritative, and the fold runs one way

Ruling 7 declared this realm's dialect and gave one reason, shape: a
proposal that amends a living specification is what this repository
already does with its decisions. A review the same day found the reason
true but unearned, and found a boundary the ruling left unwritten. Both
are settled here by the operator's rulings in chat.

**What the tree showed.** The hybrid ruling 7 retires was never two
dialects side by side. The eight OpenSpec changes hold a proposal each
and nothing more; the eight spec-kit features hold a spec, a plan and a
task list each; and `openspec/specs/`, the truth a proposal is supposed
to amend, has never existed here. Spec-kit did the work behind an
OpenSpec front. So the shape argument was a claim about a fold this
repository had never performed. The rulings below give that fold its
direction, which is what makes ruling 7 hold rather than merely sound
right.

1. **The decision is the authority; a specification never amends one.**
   An accepted decision governs. A capability specification that
   contradicts a numbered ruling is a defect in the specification, and
   is fixed there. No spec artifact, no clarification scenario and no
   design note changes what a ruling means. Only a superseding decision
   does, and it takes its own number and says so, exactly as this
   directory has always required.

2. **The fold runs one way, and its unit is the delta.** The operator's
   own formulation: *state + decision = delta, the delta becomes the
   spec, and the work fixes the delta.* The decision's prose is not
   what lands in the truth. What lands is the difference between what
   the system is specified to be today and what the ruling requires,
   which is precisely OpenSpec's unit of work and precisely what its
   archive folds into `openspec/specs/`. So the decision record is this
   realm's stream of changes and the truth tree is what that stream
   accumulates into. The two are not rival bodies of normative text;
   they are the two ends of one fold. This is the same algebra the
   engine already runs on, where state is a fold over a journal
   (decision 0002): here the specification is a fold over deltas, and
   every delta has a ruling behind it.

   A consequence for ruling 7's enactment, which the operator confirms
   when it is enacted: the truth tree is seeded from the accepted
   decisions rather than started empty. A capability whose text no
   ruling authorises is either work nobody ruled or a ruling nobody
   wrote down, and both are findings rather than specifications.

   **Enforcement binding:** judgment guidance until the truth tree
   exists, and stated as such. When ruling 7 is enacted, a test walks
   `openspec/specs/` and refuses a capability specification that cites
   no decision, the way `crates/brokkr-cli/tests/decisions_index.rs`
   holds the index equal to the files and decision 0044's registry test
   refuses a citation that does not resolve.

3. **A dialect names what installs its tool, not just what runs it.**
   The binary's name is not the tool's identity, and on both shipped
   dialects it is actively misleading. Measured 2026-09-04: OpenSpec's
   binary is `openspec`, but the bare npm name of that spelling is a
   placeholder at version 0.0.0 and the tool is published under a
   different scope, so an install by binary name silently fetches the
   wrong package; spec-kit's `specify` is not a registry package at
   all, but a project installed by uv from a git tag. A dialect
   therefore declares the manager, the package, and the source where
   the manager's default registry is not where the tool comes from.
   The manager vocabulary is closed, like every other vocabulary here:
   an installer this engine does not know is a refusal, never a shell
   command it guesses at.

   **Enforcement binding:** `contracts/dialect.v2.schema.json`, a new
   file beside v1 whose bytes do not move, pinned by
   `crates/brokkr-runtime/tests/frozen_contracts.rs`; the `install`
   block on `Tool` in `crates/brokkr-runtime/src/dialect.rs`, required
   and non-empty; a test that no shipped dialect installs by its binary
   name. This build reads `brokkr.dialect/v2` only, which costs nothing
   today: the dialect landed the same day with no realm declaring one,
   so no journal pins a v1 dialect for a resume to reload.
