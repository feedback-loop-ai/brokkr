## Purpose

Keep specification work documentary and implementation narrow,
independently reviewable, inert for MCP until enablement, and honestly proved.

## ADDED Requirements

### Requirement: SD1 specifications preserve authority and durable provenance

Specification changes SHALL follow the dialect's artifact dependency order:
proposal before deltas and design, deltas and design before tasks. Answers
SHALL be scenarios; choices and reasoned refusals SHALL be recorded under
Decisions. Every available council position SHALL be reconciled explicitly
against evidence. A demonstrated fault SHALL be repaired at its earliest
owning artifact with dependent artifacts kept coherent; downstream exceptions
SHALL not hide an upstream fault. The workflow runner SHALL not be invoked.

Decision amendments SHALL remain proposed until operator acceptance, with
index and reciprocal amendment pointers. Operator rulings SHALL be preserved
verbatim. The completed plan requires operator ruling before implementation.
Specification work SHALL claim no production change, harness measurement or
implementation proof that has not occurred.

Each design visit's chronology and review provenance SHALL have one durable
record at its commissioned evidence path in the change, referenced by file
and section without requiring branch-local commits. A new visit SHALL NOT
overwrite earlier evidence or reinterpret completed unit facts. Requirements and scenarios SHALL describe behavior independently of
particular visits. Archive provenance SHALL point to the archived change;
review chronology SHALL remain in its evidence record, not become living
capability requirements.

#### Scenario: Proposed artifacts confer no runtime authority

- **WHEN** a specification and amendment are complete but unaccepted and unimplemented
- **THEN** decisions remain proposed, implementation tasks remain open and MCP admission stays fenced
- **AND** document validation supplies no harness qualification or runtime proof

#### Scenario: A requirement fault is repaired at its owner

- **WHEN** evidence shows a requirement cannot preserve broker ownership or journal integrity
- **THEN** its owning requirement and scenarios are repaired or the claim is refuted with reasons, before dependent design and tasks change
- **AND** a downstream omission cannot close the fault; unresolved upstream faults remain identified

#### Scenario: Provenance survives a squash and archive

- **WHEN** specification commits are squashed and the completed change is later archived
- **THEN** its evidence record still contains review dispositions and reasons, and file/section references resolve without those commits
- **AND** only behavior requirements and scenarios enter living specs, whose provenance points to the archive containing that record

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
invent a dependency. MCP preparation SHALL remain inert until U9b; this does
not delay independent native-only protections. No new unused public API or
unmeasured bypass flag SHALL be added merely for staging (0071 ruling 6).

| Operator objective | Requirements the later tasks must cover |
| --- | --- |
| U0 measure; no code | SI1: strictness for each harness, Codex config/telemetry/discovery, dsh loading; dated adapter evidence and unknowns. |
| U1 strict MCP every seat, #467 | SI2, with MB2's refusal of unsupported carriage; cold, resume, replacement, empty grants and generated scaffolds migrated before admission. |
| U2 both latent panics | SC5: exhaustive native/MCP handling in resolver and doctor before any new loaded path. |
| U3 gate and DATA rules | GP1–GP2 for native and MCP, independent of broker runtime. |
| U4 native attribution first | CC1, CC3, SC4; store and all engine consumers before new emission, with legacy native compile-to-journal proofs at every merge. |
| U5 dialect model and manifest | SC1–SC3: typed preserved fields, reserved veto, additive versions and compatibility; compile still refuses MCP. |
| U6 unwired broker with boxed fake child | MB3–MB5: checked bind/reach identity, shared namespace builder, confined secret injection, tools, protocol and cleanup; no compile admission. |
| U7 multi-server wiring | MB1–MB4, SI2; adapter mcp consumer, selected box/reach/egress intent and discovery notice. |
| U8 ledger, retention and inspect | CC2, CR1–CR5: control/evidence roots excluded from server mounts, box-failure settlement, idempotent folding, artifact integrity and recovery. |
| U9 enablement and guides | MB2–MB5, SD3; doctor, U5a2 clearance and real boxed compile/launch/fold evidence; U9b alone lifts the fence. |
| U10 removal audits and final gates | SD4: proof inventory, unresolved measurement gaps, frozen-byte audit and final validation. |

Design SHALL account for already oversized capabilities.rs, bundle.rs,
engine.rs, adapters.rs, native_controls.rs, doctor.rs, init.rs and engine/resume.rs before adding code:
0071 ruling 4 permits no growth above baseline. Any extraction and new module
is an actual named production-file change, not a ceiling exemption. The
Hot files note SHALL list every heavily touched path, unit ownership and
other-thread conflicts, including contract/index/pin changes and #487's ownership of realms v7. Later moves on main require an updated inventory.

The amended U6c SHALL restart from current main's U6a shared injector and
U6b closed handler; held attempts supply reference evidence only. Re-plan
U6c, U6d, U6e, U6f and every affected U7/U8/U9 row in one dependency order.
Prefer a consumed extraction of hands' common box builder to another namespace
implementation. Name which current box functions are reused, how hands keeps
its current consumer, and where the distinct server profile removes workspace,
Git, declared binds, host-backed HOME/TMPDIR and `--new-session`. Count source,
new module, registration, and cross-crate injector access among each row's
maximum three production files; split further if necessary. The current
crate-private injector is not already a CLI API. A new public helper needs
its same-unit consumer. Preserve completed U6a/U6b facts and stable IDs for
unchanged work; add unchecked IDs for new boxing work, never tick by design.
Update the merge-order count and Hot files together, preserving the tasks
preamble's historical counts. The amended design SHALL explicitly reconcile
all seven supplied holds against MB3's construction and remaining checks.

#### Scenario: Wiring needs more than three production files

- **WHEN** U7 needs changes in engine.rs, hands.rs, native_controls.rs and an adapter file
- **THEN** design splits it into dependency-ordered PR rows with at most three production files apiece and assigns every task/proof
- **AND** the final multi-server serving path stays compile-inert across intermediate merges

#### Scenario: Independent native work need not await broker construction

- **WHEN** U3's native gate checks and U4's native checkpoints are ready independently of U1/U2 or U6–U8
- **THEN** their PRs can proceed under their own stated prerequisites, without enabling MCP
- **AND** the final plan still selects one merge order and records independence explicitly

#### Scenario: Generated declarations migrate before strict admission

- **WHEN** the ordered plan activates mandatory strict MCP admission
- **THEN** an earlier bounded unit has migrated generated Claude/Codex/dsh declarations and scaffold instructions, inventoried init.rs and its consumed extraction, and assigned init_doctor/init_stacks proofs
- **AND** measured-support success and exact unsupported/unmeasured refusals remain required for fresh scaffolds without changing their empty grants

#### Scenario: Native producers wait for their consumers

- **WHEN** normalization, v6 acceptance and attribution are delivered in separate PRs
- **THEN** drivers keep valid legacy emission until single, panel, sequence, fallback and resume consumers all exist; activation is a separately inventoried unit
- **AND** native compile-to-journal tests prove every intermediate boundary, including append/export/verify, independently of the MCP compile fence

#### Scenario: Reusing the builder needs an additional production split

- **WHEN** the box requires changing hands.rs, a new shared builder, its consuming broker module and secret.rs visibility or handoff
- **THEN** the plan splits extraction, consumption and handoff into named ordered rows, each with at most three production files and a real consumer
- **AND** hands behavior remains independently proved, the server profile excludes its workspace/session defaults, and no incomplete broker path becomes callable without its full admission checks

#### Scenario: An amended session starts from the landed base

- **WHEN** U6c resumes after a security-held implementation attempt
- **THEN** the unit starts from current main and implements the amended boxed-server contract; it does not merge the held arbitrary-startup analyzer
- **AND** later rows name box admission, readiness, secret injection, process ownership, ledger settlement and integrated U9b proofs in dependency order; unchanged task IDs retain their meaning

### Requirement: SD3 U9b alone removes the unbuilt MCP admission fence

Until U9b, every structurally valid selected-realm MCP grant SHALL retain
slice one's exact realm-wide cause:
"realm '<realm>' grants capability '<capability>' through dialect '<dialect>'
of kind 'mcp', whose broker support is not implemented until decision 0065
slice two". This includes wants, unused grants and offices [].
Earlier units may parse/test internal records or execute a fake child through
a bound test plan, but SHALL add no production compile escape. A directly
invoked broker command SHALL also refuse before spawn while its serving
protections are incomplete, with "broker serving protections are incomplete".
Once nonretaining serving is complete but retention is not, a retained plan
SHALL refuse "broker retained-response support is incomplete" rather than
silently discard its retention obligation. These temporary preparation causes
are removed with their corresponding protections, not retained as user flags.

U9b SHALL remove the fence only after strictness, safe typed resolution, gate
rules, attribution, broker box admission/readiness/filtering/secrets/cleanup, final exact-server proof,
ledger and retention all work together. Doctor SHALL use the same complete
site plan as compile/launch, report grants as distinct from holdings, retain
static versus measured scope, and display effective retention and every
exact refusal without native-binding assumptions. It SHALL start no capability
server for reporting. Guides SHALL state the actual Linux namespace/stdio/empty-restriction limits,
conservative installed-entry package layout, pre-secret box refusals versus
post-admission failures, separate hands/server network rules, retained U0
read-isolation requirement, and Codex discovery behavior. Guidance SHALL
name MB3's installation credential/history limits and MB4's host-loopback
and Linux abstract Unix socket reach under shared network; it SHALL NOT
claim credential discovery, historical byte authentication or a network
allowlist from these checks.

The engine SHALL render a fixed capability discovery notice, following
0069's adapter-facts/engine-applicability pattern, listing the held capability
and tool identifiers plus the measured discovery tool when required.
The selected attempt alone determines it; model prose cannot add/suppress it.
It SHALL not change the requested effect digest or qualify a resume shape.

#### Scenario: A preparation merge cannot run an MCP capability

- **WHEN** any head before U9b, including U9a, compiles a valid MCP realm grant, requested, wanted or unused
- **THEN** it returns exactly the old unbuilt-kind cause and spawns no server
- **AND** removing that fence prematurely fails its regression

#### Scenario: Direct invocation cannot exploit a preparation merge

- **WHEN** a valid bound test plan reaches the public command before U6f's serving protections, or a retained plan reaches it before U8b
- **THEN** the exact incomplete-serving or incomplete-retention cause above refuses before child spawn
- **AND** the existing unbound-plan refusal still wins for an untrusted locator; private fake-child tests confer no production authority

#### Scenario: The enabling proof uses the real path

- **WHEN** U9b compiles a temporary real realm, v1 fake MCP dialect and boxed model seat, then launches through the production engine and a deterministic harness fixture
- **THEN** the harness starts brokkr hands plus cap-library-docs, the real broker launches the fake MCP child inside its separate box, only the granted tool succeeds, and engine-folded checkpoints/artifacts match exact expectations
- **AND** denied tools, nonnamespace sites, bad strictness, box reach/hard-link/alias/store/fixed-key refusals, secret leaks, forged ledger and vetoed retention have independent negative controls; all box admission refusals have zero lookups and zero dialect-server starts
- **AND** the harness fixture is identified as deterministic integration evidence; real harness isolation is still supplied by U0, not claimed from the fixture

#### Scenario: The enabling proof distinguishes confinement from startup success

- **WHEN** U9b exercises the real broker with system and user-installed fake servers, a missing dependency, each egress class, and cancellation while the empty box waits
- **THEN** real Linux evidence proves read-only approved mounts, absent seat/store/evidence reach, isolated private tmpfs, the confined environment handoff and MB5 settlement, with each exact cause and call count
- **AND** the missing dependency fails inside the box without extra mounts; a success-reporting harness cannot hide its failed session; macOS keeps R2/namespace refusal
- **AND** the host launcher has no secret/code-loading bindings, U0 read-isolation and machine_proof's one injector remain separate controls, and an independently compiling removal must fail each protection's assertion

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

Source-observer feasibility SHALL be measured with the actual U6c5 observer,
not inferred from a metadata survey. U6c5 SHALL record repeatable Linux
x86_64 and aarch64 profiles using the unpruned system source set specified by
MB3, a protected package and a singleton system entry, including ordinary
protected system hard links. Record kernel/filesystem, source and mount
counts, mapped credentials, ACL/privilege assumptions, cold versus warm cache,
FD high-water mark, peak metadata memory and elapsed time. Qualification
budgets are at most 10 s for complete source observation and 20 s through
readiness, handoff and initialization on those recorded profiles, leaving
headroom within MB3's unchanged absolute 30 s runtime refusal bound.
U6c5 measures observation; U6f and U9b SHALL repeat the complete path with
readiness/handoff/startup timings. Use at least one cold and five warm samples
per profile and record every sample, not just an average. Missing host legs
stay pending and block the corresponding positive claim and U9b enablement.
Large source sets beyond MB3's limits fail closed; no source pruning, cached
verdict, timeout increase or weaker proof may manufacture a passing budget.
If ordinary installations cannot meet these budgets with the required
observer, return to the owning specification before activation.

A docs-only change SHALL run staged/unstaged git diff --check and
openspec validate --all --strict without altering tests. Where the commission
authorizes an unsigned document commit and external formatting/spelling/signing,
those obligations SHALL be recorded as pending until their results exist.
No publication or push SHALL be inferred from local validation.

#### Scenario: Source admission has a measured Linux budget

- **WHEN** U6c5, U6f and U9b report installed-server positive controls
- **THEN** the recorded Linux profiles include ordinary protected system hard links, unpruned admitted sources, actual observer and full-startup timings within the qualification budgets above; macOS remains a refusal control
- **AND** a metadata-only or boxed survey is labeled partial feasibility evidence, never namespace/identity or supported-host qualification
- **AND** exceeding an entry/depth/metadata/mount/symlink bound or expiring during source observation still takes "MCP server box filesystem identity is not protected" before lookup; expiry during readiness takes "MCP server box could not be established"; later phases retain their own exact causes

#### Scenario: Unavailable document checks remain an explicit handoff

- **WHEN** the commissioned box cannot run formatting, spelling or signing, and the documents pass both in-box gates
- **THEN** the authorized unsigned document commit may proceed with those external checks recorded as pending
- **AND** the required result records only observed work; local validation neither supplies the missing results nor selects the next phase

#### Scenario: A compiling removal proves the intended check

- **WHEN** filtering, native attribution, gate scope, retention veto, ledger deduplication or final server equality is removed independently
- **THEN** its own exact assertion fails and passes after restoration, with test name/revision/output recorded
- **AND** an earlier table-row failure or compiler error proves no later case

#### Scenario: Unavailable host coverage stays pending

- **WHEN** local checks pass but exact coverage or supported-host evidence cannot run
- **THEN** the evidence names that limitation and the owning unit remains pending for that gate
- **AND** no threshold/exclusion/fixture regeneration or claim that the docs are proof is accepted

## Decisions

R5 requires narrow PRs from main through the merge queue after operator
ruling. U0 records evidence; runnable support changes need inventoried
production rows. MCP enablement remains U9b alone.

Generated adapter declarations migrate before strict activation because they
are independent consumers of the strict-MCP metadata. New native emission
waits until complete consumers exist, because the MCP fence cannot protect
native checkpoints. These preserve the three-file limit and binding proof
ownership (0071 rulings 3, 5, 9).

Evidence stays separate from behavior: branch ancestry is not durable
provenance for a squash, and review history is not a reusable scenario.
Archive retains the evidence record and adds living-spec provenance pointers
without folding historical routing into requirements (0071 ruling 5).

The box ruling changes the preparation dependencies; it does not let a
three-file budget disguise a fourth edit. Reuse the hands builder and shared
injector with immediate consumers, retaining their independent tests and the
U9b fence (0071 rulings 4–6, 9, 10). Source evidence and H1–H7 dispositions for
this amendment live in the commissioned design-visit record, not evidence.md.
Design D9 and Slice two units own the complete ordered inventory; tasks.md
assigns stable IDs and new unchecked groups to each added cut. The plan
counts visibility, bootstrap dispatch and consumed store extraction explicitly,
rather than carrying the held attempt's fourth-file exception forward.
