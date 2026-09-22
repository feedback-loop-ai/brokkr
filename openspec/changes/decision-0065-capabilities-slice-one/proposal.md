Status: proposed specification of accepted decision 0065, slice one.
Authority: accepted decision 0065 and operator second-security-hold commission.
Repair specification: proposed; second-council H5/H6 retain spec_defect=true.
Security hold: unresolved; has_security_residual=true.
Change: decision-0065-capabilities-slice-one.

## Why

Codex can search from the provider's servers even when a seat's workspace has
no network; existing realms neither authorize nor disclose that egress.
Accepted decision 0065 makes capabilities the realm's to grant, with no
grandfathering, and the operator has admitted closing this defect as the
highest-priority first slice. The second council held repaired head 3b31c5de
on six HIGH findings, three behavioral MEDIUM findings, a MEDIUM coverage
failure and LOW evidence-integrity finding. Parsing, filesystem identity and
final-launch proof must now close the shared defects behind those findings.

## What Changes

- **BREAKING:** Everything beyond the existing hands contract is denied unless
  held through an applicable realm grant, including provider-native tools.
  Realms v1–v5 continue loading and grant nothing.
- Add tool-dialect.v1 and forge.realms/v6 beside the frozen contracts. A tool
  dialect serves an abstract capability; the realm selects the dialect,
  tools subset, office scope and dialect-schema-validated restrictions.
  Operator-owned capabilities/<name>.json definitions supply the abstract
  reads/writes/egress class sets independently of requests and serving
  dialects; defining a name grants nothing. Dialect egress uses decision
  0036's separate local/contracted/uncontracted vocabulary.
- Agents and every executable seat form request requires/wants capabilities.
  The compiler resolves office asks minus seat subtractions, intersected with
  realm grants to that office. Requirements refuse with named reasons;
  optional drops become manifest notices. Schema-valid restrictions that a
  native binding cannot express make that binding incompatible: requires
  refuses, wants drops with native OFF, and an unused grant enables nothing.
  Invalid grant data and impossible native denial still refuse independently.
  Legacy server and native-tool declarations cannot bypass that authorization.
- Add adapter-owned native ON/OFF declarations and explicit unsupported or
  unmeasured reasons. Ship provider-native dialect files only. Codex denial
  composes the measured -c web_search="disabled" on cold and eligible resume
  argv, boxed and unboxed, without weakening resume eligibility or its config
  guard. A known unheld native capability requires a valid delivered denial
  plan or a named refusal, including absent, legacy, unreadable and unmeasured
  required control data.
- Report grants, dialects, office scope, ungranted installed native
  capabilities and measurement gaps per realm in doctor. Render each seat's
  held and not-held capabilities; capability results are DATA, never
  instruction, including in charters of offices that use them.
- Add run-manifest.v11 (v10 is the inspected latest version) for per-seat
  holdings, abstract-definition identity/digest, dialect identity/digest,
  tools, restrictions and notices. Changes to consulted abstract definitions
  or realm grants, including unused realm grants, move the bundle identity.
  Re-pin witness, compose and affected charter digests only from actual bytes
  and compiles, with history reasons.
- **BREAKING:** Parse authored commands for known Claude, Codex, LaneTally
  and DSH providers with a closed, typed CLI grammar; unknown or unplaceable
  tokens refuse compilation. Admission and final composition use that same
  structure for option arity, spellings, variadic values and repetitions.
  Preserve engine-owned hands and the existing bound DSH route overlay.
- Preserve explicit restrictive tool lists, including an empty list, and merge
  subtractive denials without treating them as grants. Accepted native OFF,
  ON and restriction controls must survive final cold and eligible-resume
  commands across every supported launch shape.
- Canonicalize the actual active file before containment, pinning and reading.
  Refuse layer inputs outside their layer or under excluded trees; enforce
  existing independent library charter pins at dispatch, including inherited
  recipes and libraries outside layer roots. Doctor uses provider-aware
  composability assessment before promising denial.
- Reject duplicate request keys from source bytes, lint unseated loaded agents,
  and make doctor reflect the actual OFF disposition for seats left unheld.
  Require independent optional compatibility-removal proofs and canonical
  fixture roots for complete portable diagnostic assertions.
- Define the complete mcp kind in the schema, but refuse every realm grant
  selecting it with a reason naming the absent second slice.

## Capabilities

### New Capabilities

- tool-dialect-contract: implementation kinds, abstraction classes, disclosure
  metadata, restriction schemas and versioned contract boundaries.
- realm-capability-grants: realm-only authority, legacy empty grants, office
  scope, tool subsets and uninterpreted restriction pass-through.
- seat-capability-resolution: portable asks, subtraction, strict lints,
  compiler refusals, optional notices and fallback authorization.
- native-capability-controls: adapter evidence, native enable/disable
  composition, Codex cold/resume denial and explicit measurement limits.
- capability-doctor: per-realm inventory of grants, installed native
  capabilities, denials, unsupported controls and unmeasured guarantees.
- capability-manifest-and-prompts: pinned capability identity, notices,
  seat-visible holdings, data-only charters and measured digest migration.

### Modified Capabilities

None. Existing boundary and hands requirements remain intact; capability
authorization is an additional axis, not a boundary change.

## Impact

Implementation will touch Rust realm representation, agent/adapter loaders,
bundle resolution, launch/prompt integration and doctor under crates/, plus
adapters/, affected agents/charters, new capabilities/ and dialects/tools/
data, additive contracts and focused operator guides. The expected files and existing proof
suites are inventoried in .forge/tasks/0065-capabilities-slice-one.md; the
current repair surfaces are bounded by .forge/tasks/0065-second-security-hold.md.
No dependency is proposed; any later addition needs a stated justification.
Supported hosts remain Linux and macOS under decision 0063.

This second-hold specify visit adopts `decision-0065-capabilities-slice-one`
on `slice-0065-capabilities` at `3b31c5ded3420d2a27b8a2f09235061b7f394a59`,
retaining every commit, including `f97b7e77`, and draft PR #319. It amends the
existing proposal first and then the six deltas; it creates no replacement
change. Scope is second-council H1–H6, M1–M3, V1 and L1 only. First-council
findings remain identified below as historical origins of retained requirements.

The rendered specify step owns proposal/specs only. Design, decision 0066,
tasks, implementation and evidence require the dependent corrections named
below in their owning phases before repair completion. No implementation,
removal experiment, new measurement or completion tick is claimed here.
Accepted 0065 remains unchanged; 0066 remains **proposed**, never accepted by
this seat. There is no `returned_from` in this run context; the explicitly
commissioned second-chief ruling supplies the findings to answer.

## Decisions

The controller's three-slice cut is adopted as given, not re-triaged. Slice
two owns MCP brokers outside the box, gate-class capability policy,
capability/dialect checkpoint names and retained responses. Slice three owns
comparison/wager reporting and capabilities: equal. Reserved hands neither
grants nor moves the workspace tool. The data-only instruction is in this
slice even though the remainder of ruling 7's gate policy is deferred.

Answers and reasoned refutations of specification ambiguities are encoded as
scenarios in the owning deltas: a realm grant alone is not a seat holding;
an optional MCP grant still refuses; a native adapter key is not a grant;
DSH/LaneTally unknown inventories are not empty inventories; and a composed
Codex resume control is not live proof. The earlier clarification answered
CQ1 and CQ2 below. The first repair adopted the reconciliation in
`.forge/tasks/council-ruling-3c72a18a.md`. This visit also reads and answers
every finding in `.forge/tasks/council-ruling-25d222e6.md`, keeping their
panel attributions distinct; it is not a council-design visit.
The accepted decision and controller cut remain unchanged.

An explicit capabilities map on an agent-backed seat is the requested subset
of its office's map; omission inherits, and an empty map subtracts everything.
This uses the commissioned requires/wants vocabulary without a second grant
syntax. Strength changes are refused because subtraction removes an ask; it
does not rewrite it. A referenced agent name identifies its office; an inline
site uses its stable site label. The realm/subtraction scenarios own these
answers, so design must preserve their observable authority boundaries.

Reasoned refusals: preserving legacy concrete MCP requests as a second launch
path would let an agent choose a server; treating existing native-tool
permissions as grants would grandfather undeclared egress. The owning
resolution delta instead requires explicit migration. Making the new Codex
OFF pair force every otherwise eligible resume cold would evade the required
resume proof; the native-controls scenarios explicitly reject that outcome.

CQ1 — restriction compatibility follows decision 0065 ruling 5. Validate
all selected-realm grants first: malformed restrictions and the unbuilt MCP
kind refuse even for wants or unused grants. For a schema-valid native
restriction the serving binding cannot express, a remaining requires refuses,
a wants is dropped with its full reason and supported native OFF, and an
unused grant stays pinned but inactive. The independent impossible-OFF
refusal still wins over an optional drop. Rejecting every valid but unusable
optional binding would contradict ruling 5; discarding only its restriction
would violate rulings 3–5. The realm delta replaces its contradictory blanket
refusal with this precedence and complete diagnostic scenarios. Existing
local tool-permission restrictions retain their unconditional refusal.

CQ2 — abstract classes have one operator-owned declaration source, independent
of grant selection: capabilities/<name>.json under the active realms map's
configuration directory, or the operated repository root when no map exists.
Each file defines its abstract name and classes; neither an agent, a recipe,
a provider nor a dialect supplies a missing definition. The tool-dialect
scenarios give the complete operator-library-docs example, lookup/consistency
scope and missing/conflicting metadata refusals. Request grammar stays
name-to-requires/wants; semantic library lint resolves those names before
optional dropping or provider choice. A valid definition without a grant
still yields a named required refusal or optional drop. No-definition legacy
inputs that request and grant nothing still deny known native tools; absence
is never evidence of an empty native inventory. Consulted definitions are
pinned even for dropped requests. This is the representation of ruling 1's
abstract classes, not a new grant authority or an implementation catalogue
inside a recipe. Provider-derived classes were rejected because they cannot
classify an ungranted request and could change the abstraction when a dialect
changes. All answers remain proposed specification choices for the later
design phase, not amendments to the operator's accepted decision.

The repository's realms.json must continue granting nothing. Protected
policy/phase-machine.json, policy/schemas/, fixtures/, reference/, extensions/,
the event-envelope schema and issue-226 task ledger remain untouched.
Existing contract versions and historical evidence remain byte-for-byte.

### First-council decisions retained from the adopted repair

| Finding | Adopted correction and owning delta |
| --- | --- |
| H1 — adversarial F1, correctness C1, security S2, spec-compliance R1 | `native-capability-controls`: recognized providers cannot turn lost adapter data into permission to launch. Empty asks/grants and old realms still require final OFF or named refusal. Audit every swallowed error that gates denial. |
| H2 — security S1 | `realm-capability-grants`: authored MCP server/config/tool permissions cannot supply authority, including Codex config, Claude/LaneTally arguments and DSH patch equivalents. Preserve hands by engine provenance. |
| H3 — adversarial F2, correctness C2, spec-compliance R3 | `native-capability-controls`: every accepted argv, selection and restriction transport reaches the final command, or compilation refuses its representation. Selection-only success is rejected. |
| H4 — adversarial F3, correctness C3, security S3, spec-compliance R2 | `capability-manifest-and-prompts`: HIGH, **spec_defect=true**. No active charter or policy may be unpinned. Adopt refusal of active inputs under excluded trees; prove standalone and inherited cases. D7's deviation and task 6.1's completion basis are invalid. |
| M1 — adversarial F4, correctness C4, spec-compliance R4 | `capability-doctor`: matching grants do not bypass OFF assessment for seats left unheld; unsupported/unmeasured reasons cannot become a promise of denial. |
| M2 — correctness C5, spec-compliance R5 | `seat-capability-resolution`: reject duplicate request names and repeated capabilities fields from source bytes, before map conversion or composition loses them. |
| M3 — spec-compliance R6 | `tool-dialect-contract`: compilation semantically lints every agent in a loaded library, including unseated agents, before optionality or subtraction can hide undefined requests. |
| M4 — spec-compliance R7 | `seat-capability-resolution`: independent optional tests must fail at exact notices when provider/restriction compatibility itself is removed. Historical M3/M4 required failures and M6/M8 mutations do not supply that proof. |
| macOS CI | `seat-capability-resolution`: canonicalize temporary roots at fixture creation and derive complete diagnostic expectations from them; sweep the slice's new tests for the same assumption. |

H1 and H3 implement one rule: denial is a launch property. A successful
unmeasured plan for a provider already known to have the unheld power is
refused. An unknown inventory remains an honest uncertainty, never a claim
of denial or a way to erase a known capability. Declared control composition
and live provider enforcement remain distinct evidence.

For H2, rejecting a realm's MCP dialect is insufficient: recipe-authored
passthrough is a separate authority path. Engine-owned hands remain admitted
by provenance, never by a server name an author can copy. An equivalent
configuration door whose authority cannot be constrained is refused. This
repair does not claim to settle ambient-MCP measurement.

For H4, choose refusal of active charter/policy inputs under excluded trees
rather than making every unconsulted operator definition incidental bundle
identity. D7's composition-purity argument does not justify omitting active
bytes: a recipe-local refusal needs no operator directory, preserves pure
composition, and is compatible with the existing pinned-script fence. This
corrects the specification first; design D7 and task 6.1 must carry the same
rule before code repair. All resulting witness changes require actual compiles
and appended reasons; previous measured digests remain historical facts.

No finding is dismissed or averaged down. The HIGH identity finding remains
a specification defect even though one panel rated its impact MEDIUM.
Do not archive or fold: task 12.1 stays open pending council re-judgment,
overriding its old archive instruction. Passing validation does not lift the
security hold. Issue #255 / PR #313's Text-file-busy flake, slices two/three,
new grants, dependency upgrades and release work are outside this repair.

### Second-council decisions and finding ownership

The following H/M numbers refer to the **second** council. All are adopted on
the chief's independently checked evidence; none is averaged down or closed
by this specification. The owning deltas encode each answer as scenarios.

| Finding and attribution | Answer and owning delta |
| --- | --- |
| H1 HIGH — adversarial F1, security S1, spec-compliance R1 | `realm-capability-grants`: attached `-cVALUE`, split and equals forms have one parsed config meaning. A no-grant `-cmcp_servers.ungranted.command="/bin/false"` refuses the same realm-only authority as split config. |
| H2 HIGH — adversarial F3, security S2, spec-compliance R1 | `realm-capability-grants`: classify plugin loading and every value of variadic admission lists, including later MCP tools/wildcards and repeated/alias forms, for Claude and LaneTally. |
| H3 HIGH — adversarial F2 | `native-capability-controls`: the prompt-value reproduction cannot swallow OFF. Ambiguous split flag-looking prompt input refuses at compile; accepted prompt values retain their position while real denial options remain effective. |
| H4 HIGH — correctness C1, spec-compliance R2 | `native-capability-controls`: explicit `--tools Read`, `--tools=Read` and `--tools=` retain restrictive semantics, including present-empty versus absent. Preserve the WebSearch deny-list positive control and independent WebFetch denial. |
| H5 HIGH — correctness C2, security S3, spec-compliance R3 | `capability-manifest-and-prompts`: **spec_defect=true**. Resolve `alias/../charter.md` and `alias/../policy.json` through the filesystem before containment; escaping standalone/inherited inputs refuse. Check the file prompt consumption actually reads. |
| H6 HIGH — spec-compliance R4 | `capability-manifest-and-prompts`: **spec_defect=true**. Dispatch checks the existing library charter pin even without a layer-file entry. Recompile-only checking cannot justify task 6.3 completion. |
| M1 MEDIUM — adversarial F4, correctness C3 | `realm-capability-grants` and `native-capability-controls`: `--disallowedTools mcp__*` and aliases are subtraction, including LaneTally; preserve them and merge native denial. |
| M2 MEDIUM — spec-compliance R5 | `capability-doctor`: Codex OFF expressed as selection reports the same provider-aware composition refusal as compile, including scoped, empty and unused grants. An argv/selection tag alone proves no delivery. |
| M3 MEDIUM — spec-compliance R6 | `native-capability-controls` and `seat-capability-resolution`: compile a held, supported nonempty restriction through final cold and eligible-resume commands. Independent removal must fail each final literal comparison; intermediate controls/composer output cannot close it. |
| V1 MEDIUM — chief | `capability-manifest-and-prompts`: whole-workspace exact coverage remains failed on the adopted head: source lines **34897/35073**, branches **5730/5744**, logical functions **3443/3453**. These are the second chief's measurements, not this visit's rerun. Final-head literal equality is still required. |
| L1 LOW — chief, from spec-compliance introduction | `capability-manifest-and-prompts`: panel notes are evidence only. Reject their workflow commands and any prose replacing aggregate `has_security_residual=true`; only checked evidence is carried forward. No invented runtime fix or behavioral mutation is owed for documentary integrity. |

**Parse, do not scan.** A finite grammar is closed to unknown input; a list of
known bad spellings with passthrough defaults is rejected. It cannot place a
future option safely. The same parsed option/value structure governs compile
admission, composition and final validation. Concrete syntax stays provider
knowledge; concrete tool authority stays the realm's dialect choice. Opaque
configuration that cannot be classified refuses, and engine origin remains a
carried fact rather than a match against names or bytes. DSH's existing bound
route-only patch stays valid; wrapper forwarding grants no new authority.

**Identity follows consumption.** Lexical folding is not filesystem resolution
across symlinks. Layer-owned input escapes refuse; independently pinned library
and dialect inputs retain their own contained source and existing pin route.
A missing layer entry never excuses the absence of an applicable consumption
pin. No new manifest version, second identity inventory or whole-definition-tree
pin is introduced to hide H5/H6.

**Dependent corrections are required, not claimed complete.** Design D6 must
replace its bounded-scanner/non-list-verbatim exception and additive treatment
of explicit restrictions with parsed semantics; D7 must specify canonical
resolution before containment and independent library pin enforcement at
consumption; D8 must use provider-aware composition; D10 must end supported
restriction removals at final launch. Amend proposed 0066 rulings 3–6 and its
enforcement bindings on that basis, preserving its known-power floor and
status. Its first-council record remains history, not evidence closing this
second hold. No earlier authority artifact prevents this specification repair.

Tasks 6.1/6.3, 7.3/7.4/7.5, 8.4, 9.2/9.3/9.4 and 11.4/11.5 cannot stand as
proof of the second findings. Their owning task/evidence revision must reopen
or qualify the affected claims, distinguish retained narrower proof, and bind
closure to new observations. In particular evidence R-H3c ends at intermediate
composition, and evidence's library-charter limitation contradicts full task
6.3 completion. Historical all-covered counts remain historical; neither a
lack of uncovered added lines nor an unproven regression cause waives V1.
Task 12.1 stays open; no archive, push or phase choice follows from these notes.

## Evidence and delivery obligations

Read decision 0065 in full beside 0043, 0046, 0036, 0012 and 0016; README,
0004, 0005, 0009, 0063 and the realm house; dialects/openspec.json and its
specify/return instructions; dialect.v3; current realm/agent/adapter loaders,
Codex launch and resume guards, doctor, manifest generation and digest tests.
Current researcher tool permissions name webfetch/websearch: migration must
remove that concrete-tool authorization path, not silently preserve it.

The controller's .forge/tasks/controller-codex-web-search-switch-2026-09-21.json
establishes only codex-cli 0.154.0 cold exec: the default searched, while
-c web_search="disabled" removed the tool. The following remain controller
measurements owed, and delivery notes must report their actual status:

| Evidence gap | What is and is not established |
| --- | --- |
| Codex resumed web search | Compose and deterministically prove the OFF argv on an actual eligible resume; live enforcement on a resumed session is unmeasured. |
| Codex explicit ON values and other versions | Default-ON cold behavior was measured; no other configuration value or version is thereby verified. |
| Claude native controls | Empty native tools under strict MCP configuration is adapter data, not a live denial/enablement measurement. |
| DSH native inventory and controls | Unsupported mcp/tool_permissions establishes neither absence of native egress nor a measured OFF control; declare unmeasured with that reason. |
| LaneTally native inventory and controls | Wrapper/native controls are unverified; Claude data or wrapper argv forwarding does not measure them; declare unmeasured. |

Implementation acceptance includes full reason/notice assertions and removal
proofs for compiler enforcement, boxed/unboxed Codex cold and eligible resume
argv in both grant states, strict library/schema tests, per-realm doctor and
prompt tests, and actual grant/dialect/restriction digest movement. Each
finding first needs a failing regression on the delivered code, then a repair,
then an independent removal proof that fails at the intended assertion and
passes with enforcement restored; a compile error or unrelated failure is
not proof.

Required validation remains cargo fmt --all -- --check; cargo clippy
--workspace --all-targets --all-features --locked -- -D warnings; each of the
seven crate suites, crate-scoped; cargo test --workspace and the all-features
locked workspace suite; the bundles/self compile and every re-pinned compile;
openspec validate --all --strict; and the unchanged literal-100% exact coverage
gate, including every added production line. Record candidate-bound source-line,
branch and function covered/total counts separately; no unavailable count is
100%. Preserve the three earlier removal-found regressions: adapter duplicate
keys, sequence fallback receiving its own provider plan, and unmapped resume
using the operated root with the workspace decoy removed. Host/CI coverage
evidence remains pending until it exists where nested boundaries can execute. Notes describe
observations and never waive, instruct or substitute for a gate. There is no
release, publication, profile update or push in this commission.

## Historical specification validation

The initial specify visit passed strict validation of this change and
openspec validate --all --strict --no-interactive: 16 items passed, zero
failed. OpenSpec reports proposal and specs complete; design and tasks remain
subsequent phase work. Existing informational archive notices for issue-226
spec deltas were reported by the validator and those artifacts were unchanged.

On the initial visit, Rust formatting, clippy, both workspace test commands,
all seven crate-scoped test commands, the bundles/self compile and scripts/coverage-exact.sh were
attempted through the workspace hands. Every attempt stopped with cargo:
command not found (exit 127); no Rust suite, compilation or coverage result is
claimed. These are environment limitations in this specification visit, not
observed implementation failures or changes to the required gates.

This return passed strict validation of the amended change and strict
all-item validation: 16 passed, zero failed. Proposal and all six deltas are
complete; design and tasks remain later-phase work. git diff --check is clean.
The validator also reports informational long-requirement notices and the
same unrelated issue-226 archive notices; neither is a validation failure.

Return attempts of formatting, clippy, both workspace test commands, the
seven actual crate suites and bundles/self compilation could not start
because cargo is absent from the workspace PATH. The exact-coverage script
also stopped at cargo with exit 127. No Rust, compile or coverage pass is
claimed. No implementation, provider measurement, dependency change or digest
re-pin is performed by this specification return; all controller measurement
obligations above remain outstanding.

## First-hold specification validation (historical)

This visit changes only the proposal and six specification deltas. No repair
regression, code fix, mutation experiment, host result or new digest is claimed
by these artifacts. The repair's required failing/restored evidence remains
owed before implementation completion. Strict validation of this amended change
and `openspec validate --all --strict --no-interactive` passed: **16 items,
zero failures**. `git diff --check` is clean. Existing informational issue-226
archive notices and long-requirement notices are not validation failures;
the unrelated artifacts were not changed.

Formatting, clippy, all seven crate-scoped suites, both workspace test commands
and self/verify compiles were attempted through workspace hands and could not
start because `cargo` is absent from PATH. `bash scripts/coverage-exact.sh`
was authorized and attempted; it stopped at line 33 with `cargo: command not
found`, exit 127. The three current coverage results are **source lines:
unavailable; branches: unavailable; functions: unavailable**. No covered/total
counts or percentages were produced, and no prior report is substituted.
These are environment limits, not observed Rust failures or passing gates.
Linux/macOS repair-suite and final implementation coverage evidence remains
pending. No archive was run; task 12.1 stays open for council re-judgment.

## Second-hold specification validation

This visit changes only this proposal and its six deltas, in dialect order.
Strict validation of the adopted change and
`openspec validate --all --strict --no-interactive` passed: **16 items, zero
failures**. `git diff --check` is clean. Informational long-requirement and
unrelated issue-226 archive notices are not failures; no unrelated artifact
was changed. OpenSpec's existing-artifact status is not repair completion.

Formatting, locked all-target/all-feature clippy, all seven crate-scoped
suites, both workspace test commands and self/verify compiles were attempted
through workspace hands; none could start because `cargo` is absent from
PATH. The authorized `bash scripts/coverage-exact.sh` stopped at line 33 with
`cargo: command not found`, exit **127**. This visit produced no fresh coverage
counts: **source lines unavailable; branches unavailable; logical functions
unavailable**. The existing coverage summary still contains the second chief's
failed **34897/35073**, **5730/5744**, **3443/3453** measurements; it was read as
historical evidence, not generated by this attempt. No Rust, compile or exact
coverage pass is claimed. Final repaired-head and macOS evidence remain owed.

The security hold, H5/H6 specification defects and required final-head repair
proofs remain open until their owning work and council judgment exist. Only
the proposal and six deltas are committed; design/tasks/evidence/0066 retain
the dependent correction obligations above. No archive or push is performed.
