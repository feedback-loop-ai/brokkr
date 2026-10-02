## Purpose

Keep this design run documentary and make later implementation narrow,
independently reviewable, inert for MCP until enablement, and honestly proved.

## ADDED Requirements

### Requirement: SD1 the design run follows the dialect and awaits the operator

This run SHALL produce documents only through specify, clarify, council
design, tasks and analyze; it SHALL not invoke the dialect's workflow runner.
Specify SHALL create/adopt proposal before spec deltas and record answers as
scenarios and reasons under Decisions. Council design SHALL read every
available position, explicitly adopt/reject/combine its claims with evidence,
and end design.md with "Slice two units". Tasks SHALL be numbered checkbox
groups, naming the requirement each task closes. Analyze SHALL assess all
dependent artifacts. A returned finding SHALL be answered at its earliest
owning artifact and dependencies made coherent; an upstream fault SHALL not
be hidden by a downstream exception.

The run SHALL preserve R1–R5 verbatim in operator-ruling-2026-10-03.md and
deliver proposed decision 0077 (unless claimed), with index and reciprocal
Amends/Amended by pointers. Every decision stays proposed until the operator
accepts it. The operator rules on the completed change before implementation.
No production/test/schema/adapter edits, harness measurement execution or
MCP grants SHALL enter this run.

#### Scenario: A specify draft is not a finished council design

- **WHEN** specify has authored proposal, deltas, the ruling record and proposed amendment
- **THEN** it returns inputs.change as decision-0065-capabilities-slice-two and claims only the artifacts it actually authored
- **AND** it does not invent council assent, task completion, implementation, measurements or later-phase artifacts

#### Scenario: A design finding repairs its owner

- **WHEN** a later position demonstrates that a requirement cannot preserve the broker ownership or journal invariant
- **THEN** the council identifies that earliest requirement and returns upstream with the evidence
- **AND** changing a downstream task to silently omit the obligation is not closure

### Requirement: SD2 one ordered plan splits every oversized implementation unit

The final design SHALL publish one true dependency order based on the
operator's U0–U10 objectives below. Each row SHALL be exactly one PR from main,
signed, through the merge queue, with dependencies, task IDs, exact touched
files and proof ownership. A row SHALL touch at most three production files;
shipped JSON and embedded/new contracts count as production for planning.
Tests, evidence and measured pins SHALL be inventoried separately. If an
objective needs more files, design SHALL split it into lettered narrow rows
and explain the checked source reason. A phase-sized placeholder or one row
that silently means several PRs is insufficient.

U1–U4 SHALL be explicitly marked independent of one another, with U0 evidence
dependencies stated where applicable. Listing them in a merge order does not
invent a dependency. MCP preparation SHALL remain inert until U9; this does
not delay independent native-only protections. No new unused public API or
unmeasured bypass flag SHALL be added merely for staging (0071 ruling 6).

| Operator objective | Requirements the later tasks must cover |
| --- | --- |
| U0 measure; no code | SI1: strictness for each harness, Codex config/telemetry/discovery, dsh loading; dated adapter evidence and unknowns. |
| U1 strict MCP every seat, #467 | SI2, with MB2's refusal of unsupported carriage; cold, resume, replacement and empty-grant paths. |
| U2 both latent panics | SC5: exhaustive native/MCP handling in resolver and doctor before any new loaded path. |
| U3 gate and DATA rules | GP1–GP2 for native and MCP, independent of broker runtime. |
| U4 native attribution first | CC1, CC3, SC4 and their new record/reader tests; no broker required. |
| U5 dialect model and manifest | SC1–SC3: typed preserved fields, reserved veto, additive versions and compatibility; compile still refuses MCP. |
| U6 unwired broker with fake child | MB3–MB5: tools, protocol, secret injector and cleanup; no compile admission. |
| U7 multi-server wiring | MB1, MB2, SI2; adapter mcp consumer and selected-attempt discovery notice. |
| U8 ledger, retention and inspect | CC2, CR1–CR5: idempotent engine folding, artifact integrity, reader behavior and recovery. |
| U9 enablement and guides | MB2, SD3; doctor, real compile/launch fake-dialect end to end; all dependencies green. |
| U10 removal audits and final gates | SD4: proof inventory, unresolved measurement gaps, frozen-byte audit and final validation. |

Design SHALL account for already oversized capabilities.rs, bundle.rs,
engine.rs, adapters.rs, native_controls.rs and doctor.rs before adding code:
0071 ruling 4 permits no growth above baseline. Any extraction and new module
is an actual named production-file change, not a ceiling exemption. The
Hot files note SHALL list every heavily touched path, unit ownership and
other-thread conflicts, including contract/index/pin changes and #487's ownership of realms v7. Later moves on main require an updated inventory.

#### Scenario: Wiring needs more than three production files

- **WHEN** U7 needs changes in engine.rs, hands.rs, native_controls.rs and an adapter file
- **THEN** design splits it into dependency-ordered PR rows with at most three production files apiece and assigns every task/proof
- **AND** the final multi-server serving path stays compile-inert across intermediate merges

#### Scenario: Independent native work need not await broker construction

- **WHEN** U3's native gate checks and U4's native checkpoints are ready independently of U1/U2 or U6–U8
- **THEN** their PRs can proceed under their own stated prerequisites, without enabling MCP
- **AND** the final plan still selects one merge order and records independence explicitly

### Requirement: SD3 U9 alone removes the unbuilt MCP admission fence

Until U9, every structurally valid selected-realm MCP grant SHALL retain
slice one's exact realm-wide cause:
"realm '<realm>' grants capability '<capability>' through dialect '<dialect>'
of kind 'mcp', whose broker support is not implemented until decision 0065
slice two". This includes wants, unused grants and offices [].
Earlier units may parse/test internal records or execute a fake child through
a bound test plan, but SHALL add no production compile escape.

U9 SHALL remove the fence only after strictness, safe typed resolution, gate
rules, attribution, broker filtering/secrets/cleanup, final exact-server proof,
ledger and retention all work together. Doctor SHALL use the same complete
site plan as compile/launch, report grants as distinct from holdings, retain
static versus measured scope, and display effective retention and every
exact refusal without native-binding assumptions. It SHALL start no capability
server for reporting. Guides SHALL state the actual namespace/stdio/empty-
restriction limits and Codex discovery behavior.

The engine SHALL render a fixed capability discovery notice, following
0069's adapter-facts/engine-applicability pattern, listing the held capability
and tool identifiers plus the measured discovery tool when required.
The selected attempt alone determines it; model prose cannot add/suppress it.
It SHALL not change the requested effect digest or qualify a resume shape.

#### Scenario: A preparation merge cannot run an MCP capability

- **WHEN** any U1–U8 intermediate head compiles a valid MCP realm grant, requested, wanted or unused
- **THEN** it returns exactly the old unbuilt-kind cause and spawns no server
- **AND** removing that fence prematurely fails its regression

#### Scenario: The enabling proof uses the real path

- **WHEN** U9 compiles a temporary real realm, v1 fake MCP dialect and boxed model seat, then launches through the production engine and a deterministic harness fixture
- **THEN** the harness starts brokkr hands plus cap-library-docs, the real broker launches the fake MCP child, only the granted tool succeeds, and engine-folded checkpoints/artifacts match exact expectations
- **AND** denied tools, nonnamespace sites, bad strictness, secret leaks, forged ledger and vetoed retention have independent negative controls
- **AND** the harness fixture is identified as deterministic integration evidence; real harness isolation is still supplied by U0, not claimed from the fixture

#### Scenario: Discovery follows the serving adapter

- **WHEN** a measured Codex attempt defers cap-library-docs tools behind tool_search
- **THEN** its engine-rendered notice names library-docs, the exact callable tool identifiers and tool_search
- **AND** a fallback without that discovery declaration hears no stale Codex notice, a dropped holding advertises no tool, and the existing hands notice remains coherent

#### Scenario: Doctor reports admission without exercising a capability

- **WHEN** doctor examines an admitted namespace MCP site, a retention veto, an incompatible provider and a nonnamespace site
- **THEN** it prints the selected dialect/tools, declared versus effective retention and complete plan admission/refusal for each at its actual scope
- **AND** a deterministic no-spawn assertion observes zero broker/model/capability calls

### Requirement: SD4 every proof names what was observed

Implementation SHALL extend the owning suites and bind every new test by a
compiling behavior-removal mutation, intended failing assertion and restored
pass (0071 ruling 9). Tests SHALL assert exact values/typed variants and pin
operator text once per module, not is_err alone. Shared fixture builders
and unwind-safe environment guards SHALL be used. No frozen corpus shall be
regenerated; new contract fixtures are additive.

Each unit SHALL run cargo test --workspace, compile bundles/self, and satisfy
formatting, clippy and the unchanged exact-coverage gate before full success
is reported. Namespace/exact-coverage checks SHALL run on a capable host or
CI outside a box that refuses nesting; until observed they remain pending.
Final U10 SHALL audit all removal evidence, final-head Linux/macOS results,
frozen contracts/fixtures/policy/reference bytes and measured witness/compose
pins. It SHALL not substitute a historical pass, zero-test run or docs
validation for behavioral evidence. Production remains Rust under crates/.

This docs visit SHALL run git diff --check (staged and unstaged) and
openspec validate --all --strict; it SHALL change no tests. Documents SHALL
be committed with plain unsigned git commit in repository style, never
pushed. The operator runs cargo fmt and typos --hidden outside the box and
signs the single squash commit before pushing. Those handoff steps remain
pending until their results exist; they do not block this document commit.

#### Scenario: The second visit supersedes unavailable local obligations

- **WHEN** the documents pass both in-box gates in the commissioned box without cargo, typos or the signing program
- **THEN** the seat commits the documents unsigned and writes its result file with inputs.change
- **AND** outside-box formatting, spelling and squash signing remain explicitly pending; the first visit's unavailable checks are historical rather than repeated blockers

#### Scenario: A compiling removal proves the intended check

- **WHEN** filtering, native attribution, gate scope, retention veto, ledger deduplication or final server equality is removed independently
- **THEN** its own exact assertion fails and passes after restoration, with test name/revision/output recorded
- **AND** an earlier table-row failure or compiler error proves no later case

#### Scenario: Unavailable host coverage stays pending

- **WHEN** local checks pass but exact coverage or supported-host evidence cannot run
- **THEN** the evidence names that limitation and the owning unit remains pending for that gate
- **AND** no threshold/exclusion/fixture regeneration or claim that the docs are proof is accepted

## Decisions

R5 is a staged PR process, not a long-lived implementation branch containing
all units. U0 records evidence; the measurement's runnable scripts or adapter
support changes need their own inventoried later production rows.

The second commission authorizes completing the dialect's document sequence
in one visit. This does not decide the engine's next phase or invent absent
council positions. Design records the positions actually available and the
analysis; all implementation checkboxes stay open.
