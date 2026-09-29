# boxed-hands-discovery Specification

## Purpose
Let a boxed model seat discover its writable workspace tool even when its
provider defers MCP tools, through narrowly declared provider facts and
engine-owned guidance that preserves the existing confinement boundary.

## Requirements

### Requirement: A provider declares only the tool identifiers needed for discovery

A provider adapter SHALL be able to declare an optional `hands.notice` object
alongside its supported `hands.workspace` fragment. The object SHALL contain
exactly two required strings, `workspace_tool` and `discovery_tool`. Each SHALL
be a tool identifier of 1 through 128 ASCII bytes matching
`^[A-Za-z_][A-Za-z0-9_]*$`. The declaration SHALL NOT carry arbitrary prose,
templates, paths, commands, environment, configuration or a recipe-controlled
switch. A notice SHALL require a nonempty workspace fragment. Absence SHALL
preserve the adapter's existing behavior; a present malformed declaration
SHALL be refused rather than silently omitted or coerced.

Codex's shipped declaration SHALL name `mcp__brokkr__workspace` and
`tool_search`, respectively. The engine SHALL use the declared identifiers,
not a hard-coded provider/model name test. Claude's shipped adapter SHALL
remain without this notice. This declaration SHALL NOT change MCP registration,
permissions, hands policy, boundary admission or provider launch arguments.

#### Scenario: Codex declares the two discovery identifiers
- **WHEN** the shipped Codex adapter loads with `hands.notice` equal to `{"workspace_tool":"mcp__brokkr__workspace","discovery_tool":"tool_search"}`
- **THEN** both identifiers are retained exactly as declared alongside the existing workspace fragment
- **AND** the existing read-only sandbox, server attachment and MCP approval arguments are unchanged

#### Scenario: Absence is compatible with existing adapters
- **WHEN** an otherwise valid adapter omits `hands.notice`, including the shipped Claude adapter
- **THEN** it loads with no discovery notice and preserves its existing hands and harness behavior
- **AND** omission is distinct from an invalid explicit `null`, `false`, string or array notice

#### Scenario: Invalid declarations fail with attributable diagnostics
- **WHEN** a notice is not an object, omits either required identifier, has an unknown member, has a non-string identifier, or contains an empty, 129-byte, whitespace-bearing, newline-bearing, containing punctuation outside the identifier grammar or non-ASCII identifier
- **THEN** adapter loading refuses and identifies the provider, offending `hands.notice` field and violated shape or identifier rule
- **AND** a notice beside unsupported hands or without a nonempty workspace fragment refuses as incompatible with workspace discovery
- **AND** tests compare the exact diagnostic for each input rather than merely asserting an error

#### Scenario: Identifier limits are literal
- **WHEN** both identifiers use the declared ASCII grammar at lengths 1 and 128 bytes
- **THEN** those notice values load unchanged in an otherwise valid adapter
- **AND** the same valid prefix extended to 129 bytes is refused with the identifier-limit diagnostic

#### Scenario: Declaration controls discovery independently of the provider name
- **GIVEN** a temporary provider mapping with valid workspace hands and a notice naming `fixture_workspace` and `fixture_search`
- **WHEN** a boxed model seat is served by that provider
- **THEN** its engine-owned discovery guidance names exactly those declared identifiers
- **AND** a Codex-shaped temporary adapter with the notice omitted receives no new discovery guidance

### Requirement: Only an actually boxed seat receives its selected provider's notice

For every executing model site, the engine SHALL supply discovery guidance
if and only if that site has workspace hands, its effective boundary is boxed,
and its selected provider declares a notice. The engine SHALL select and
render the notice for the provider serving that attempt, including fallback,
not for the first, last or any merely consulted chain candidate. A site with
no hands or unresolved hands SHALL receive no notice. `harness` and `open`
SHALL receive no notice even if the agent declares hands or the adapter can
serve workspace hands elsewhere. An exec script SHALL receive no model-facing
discovery paragraph.

This rule SHALL hold for a single seat, a selected strategy's executing body,
each panel member, each sequence step and each nested panel member. Guidance
SHALL NOT leak across siblings or be inherited from a container seat that
owns no executing hands. Existing boundary availability and gate admission
rules SHALL remain unchanged; data-only tests SHALL NOT claim an unbuilt box
has run.

#### Scenario: Boxed Codex is told how to find its hands
- **GIVEN** the shipped Codex declaration and a workspace-hands model site admitted under a boxed boundary
- **WHEN** its real adapter loading, provider resolution, engine dispatch input assembly and prompt rendering path is exercised without a live provider
- **THEN** the mandatory engine-owned result contract includes exactly one discovery paragraph with the following literal text

> Your workspace tool is `mcp__brokkr__workspace`. If it is not listed, use `tool_search` to load it before doing workspace work. Native shell and apply_patch writes are refused by design; this is not a blocker. Use the workspace tool for all workspace writes, including the result file.

- **AND** the existing generic hands paragraph and exact result-file path remain present

#### Scenario: Other seats keep their existing prompt contract
- **WHEN** otherwise valid prompts are rendered for a boxed Claude model seat, an unboxed Codex work seat under `harness`, an unboxed Codex work seat under `open`, a Codex site without hands, and an exec script
- **THEN** each prompt has zero engine-owned discovery paragraphs
- **AND** boxed Claude retains its existing paragraph naming `mcp__brokkr__workspace`; absence of discovery guidance does not mean removing existing hands guidance
- **AND** each unboxed seat keeps its existing result-delivery instructions, including a separately admitted harness gate's last-message door

#### Scenario: Fallback uses only the provider serving the attempt
- **GIVEN** a boxed model site whose otherwise admissible chain contains Codex and Claude
- **WHEN** the existing failure-to-start rule selects Claude after a Codex candidate
- **THEN** the new attempt contains no discovery notice, despite the consulted Codex declaration
- **AND** in a separate admissible chain that falls from Claude to Codex, the Codex attempt contains the exact Codex notice
- **AND** neither transition creates a different-effect refusal merely because the selected provider's notice changed

#### Scenario: Every executing site has its own applicability
- **WHEN** deterministic dispatch exercises single, selected-body, panel-member, sequence-step and nested-panel-member model sites with mixed notice-bearing and notice-free providers
- **THEN** each prompt's discovery paragraph is exactly its own selected provider's paragraph when its own hands are boxed, and absent otherwise
- **AND** unselected strategy bodies, sibling providers and non-executing panel or sequence containers supply no notice to that site

#### Scenario: Boxed means the canonical boundary fact
- **WHEN** applicability is evaluated over each declared boxed boundary (`namespace`, `seatbelt`, `container`) with resolved workspace hands, and over `harness`, `open`, no-hands and unresolved-hands cases
- **THEN** only the resolved boxed cases with a provider declaration qualify for guidance
- **AND** tests that inspect unbuilt-boundary data leave the existing runtime refusal intact and claim no live namespace, seatbelt or container execution

### Requirement: Discovery guidance is engine-owned and exposes no other configuration

The engine SHALL construct the discovery paragraph from a fixed template and
only the validated two-identifier declaration. It SHALL append it in the
engine-owned result contract independently of charter, house, feature and
recipe-authored context. A recipe SHALL have no field, input or override that
can supply, replace, disable or suppress the structured notice or forge the
canonical provider, hands or boundary facts controlling it. Recipe-origin
text SHALL NOT be interpreted as discovery metadata. Rendered ordinary prose
can quote tool names; that quotation SHALL NOT constitute an engine-authored
notice or count as a proof of engine ownership.

Only the two tool identifiers SHALL flow from this declaration into the new
paragraph. No other adapter, MCP, launch, binding or environment value SHALL
be echoed as part of adding discovery guidance. The baseline prompt's already
required workdir, result path, house and task text SHALL retain their existing
roles; this restriction SHALL NOT remove them. The notice SHALL NOT promise
that native reads are confined or that discovering hands grants new powers.

#### Scenario: A silent charter cannot suppress the engine paragraph
- **GIVEN** a minimal valid recipe and office charter containing no workspace-tool or discovery guidance
- **WHEN** a boxed Codex site is resolved and rendered through the engine
- **THEN** the exact engine-owned discovery paragraph is present once in its result contract
- **AND** supplying a different charter with instructions to omit it leaves that engine-owned paragraph unchanged

#### Scenario: Recipe fields cannot author or disable discovery
- **WHEN** a recipe attempts to add or override `hands.notice` or the internal notice carrier at the real recipe/site/input boundary, including values `false`, `null` and forged tool identifiers
- **THEN** unsupported structural fields receive the boundary's exact rejection diagnostic, and any otherwise legal declared input or context value cannot change the engine-owned notice
- **AND** after rendering an applicable boxed Codex seat the engine-owned paragraph still equals the expected paragraph, or the malformed recipe was refused before dispatch
- **AND** a recipe-authored value cannot create a notice for boxed Claude, unboxed Codex or a no-hands site
- **AND** tests exercise both replacement and suppression through actual recipe ingestion and input propagation; calling the renderer with a hand-built trusted input is insufficient proof of this boundary

#### Scenario: Quoted text stays ordinary text
- **GIVEN** valid recipe text or context that quotes notice-like wording and false provider, boundary or hands claims
- **WHEN** the engine renders the actual selected site
- **THEN** that ordinary text does not become structured discovery data and does not override the canonical applicability result
- **AND** assertions distinguish the engine-generated result-contract paragraph from quoted charter, task or context text, rather than searching the entire prompt for a tool name

#### Scenario: Unrelated configuration remains outside the prompt
- **GIVEN** distinct synthetic sentinel values in unrelated adapter evidence and valid launch/MCP configuration, with neither sentinel present in authored task text
- **WHEN** an applicable boxed prompt is rendered through the real resolution and dispatch path
- **THEN** both notice identifiers and the fixed guidance are present exactly as expected
- **AND** all unrelated sentinels are absent from the complete prompt; no serialized adapter, command line, bind policy, environment or MCP configuration was appended

### Requirement: Dated hands evidence does not requalify Codex resume

The Codex adapter's existing evidence or limitations SHALL record the supplied
host measurement dated 2026-09-24 as a hands-discovery observation. It SHALL
identify installed Codex CLI `0.156.0` separately from the resume-qualified
`0.154.0`, preserve the measurement's provenance and limits, and state that it
does not qualify resume. `resume.work-site.identity.version`, `applies_to`,
status, classes, boundaries, hands coordinate and all resume eligibility
semantics SHALL remain unchanged. Task 11.1's separate operator ruling SHALL
remain its prerequisite. Existing dated evidence SHALL remain historical
rather than be rewritten as a 0.156.0 claim.

#### Scenario: The adapter records exactly the supplied discovery observation
- **WHEN** the Codex evidence or limitations are updated for this feature
- **THEN** the dated entry attributes the observations to the operator-supplied host measurement: Codex 0.156.0 deferred MCP tools behind `tool_search` with the adapter-equivalent hands attachment; neither gpt-5.6-sol nor gpt-6-sol listed the workspace tool in the no-tool-call probe
- **AND** it records `tool_search_always_defer_mcp_tools` as reported `removed, true`, the ineffectiveness of both `features.tool_search_always_defer_mcp_tools=false` and `features.tool_search=false`, and the host's finding of no per-server loading switch
- **AND** it identifies this as observed 0.156.0 behavior, not a prediction about future releases or a new probe by the implementation seat

#### Scenario: Wager evidence motivates guidance without claiming a new qualification
- **WHEN** the change's evidence is read
- **THEN** it records the supplied six-arm 0065 rebuild-unit-3 wager observations: gpt-5.6-sol searched four times, used the workspace tool 225 times and delivered; gpt-6-sol and gpt-6-luna never searched and stopped after native-write failures without result files; gpt-5.6-luna briefly found hands and fell back to apply_patch
- **AND** it claims no unreported outcomes for the other arms and no successful 0.156.0 resume proof
- **AND** the generic hands paragraph already present at `aa58d07a` is retained; the measured failure motivates adding discovery guidance, not claiming the current source never names the tool

#### Scenario: Qualification remains pinned at the measured older version
- **WHEN** the completed Codex adapter is compared with its pre-change declaration
- **THEN** `resume.work-site.identity` still equals `{"version":"0.154.0","applies_to":"0.154.0"}` and all resume qualification fields and behavior retain their pre-change values
- **AND** the added observation explicitly leaves 0.156.0 resume and task 11.1 to the separate operator ruling; neither metadata nor tests silently enable boxed resume

### Requirement: Adapter byte changes receive measured identity updates

A change to the Codex adapter declaration or evidence SHALL move the identities
that pin its bytes under the existing digest rules. Implementation SHALL
compile and measure all affected witness and composed bundles, including
chains that consult Codex only as a fallback, and update only changed expected
pins with their specific cause. Unaffected pins SHALL remain unchanged.
Expected digests SHALL come from actual compilation, never guesses or broad
replacement. No engine version bump, new serialized contract, recipe change
or frozen-surface edit SHALL be introduced to accommodate the notice.

#### Scenario: Every consulted adapter remains identity-bearing
- **GIVEN** otherwise unchanged witness and compose inputs, including a chain served by Claude that also consults Codex
- **WHEN** the Codex notice and dated evidence are added and the affected bundles are compiled
- **THEN** recorded before/after manifest digests account for the changed Codex file even on that fallback-only chain
- **AND** the resulting pin updates name the adapter-byte change as their reason and correspond exactly to the actual compiled values

#### Scenario: Unrelated identities and frozen bytes stay fixed
- **WHEN** the measured results and final diff are compared with the base
- **THEN** unaffected witness and compose pins remain identical and no guess replaces a digest
- **AND** `contracts/`, `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`, `reference/` and `extensions/` retain their bytes
- **AND** the qualified Codex version remains 0.154.0 even though adapter and dependent bundle digests changed

### Requirement: Every discovery guarantee has deterministic proof and restored mutation evidence

Tests SHALL prove this capability through production loading, resolution,
dispatch and prompt rendering without a live provider. Temporary fixtures
SHALL live outside the frozen corpus. Assertions SHALL compare exact prompt
paragraphs, selected facts, absence counts or reason-bearing diagnostics.
Assertions SHALL NOT use `is_err()`. The implementation SHALL add a
numbered semantic decision with `Status: proposed`, alternatives and named
enforcement bindings; only the operator accepts it.

Each claimed protection SHALL have a compiling mutation that violates that
protection and fails its targeted test on the relevant assertion. A record
under `.forge/tasks/` SHALL name the change, mutation, command, successful
compilation, failing test and assertion, restoration, and successful rerun.
A build error, unrelated failure, changed test expectation or predicted failure
SHALL NOT count. Every mutation SHALL be restored before the final checks.

#### Scenario: Positive and negative prompt guarantees are mutation-bound
- **WHEN** proof for the rendered notice and applicability is reviewed
- **THEN** recorded compiling mutations cover loss of the paragraph or either identifier, loss of the native-refusal/not-a-blocker explanation or result-writing direction, erroneous rendering for boxed Claude, unboxed Codex under each of harness and open, and a site without hands
- **AND** they cover using an unselected provider's declaration, leaking between execution sites and rendering a notice for exec
- **AND** each produces the relevant assertion failure, followed by restoration and a passing rerun; one mutation can cover multiple claims only when each is named and observed

#### Scenario: Authority and disclosure guarantees are mutation-bound
- **WHEN** recipe-ownership, declaration-validation and narrow-rendering evidence is reviewed
- **THEN** compiling mutations that admit malformed discovery data, permit recipe injection or suppression, depend on charter guidance, hard-code the provider instead of reading its declaration, or echo unrelated configuration each fail the corresponding exact assertions
- **AND** ownership mutations are tested through real recipe/input boundaries, not only by directly constructing trusted renderer input
- **AND** each mutation is restored and the same proof passes again

#### Scenario: Completion names the checks actually performed
- **WHEN** implementation is reported complete
- **THEN** the evidence records `cargo test --workspace` (the all-features locked form satisfies it), `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, compilation of `bundles/self`, strict OpenSpec validation and the exact-coverage gate result
- **AND** if namespace restrictions prevent exact coverage inside the workspace box, host/CI execution remains explicitly pending until its actual result exists; the gate is neither weakened nor reported clean from skipped boundary proof
- **AND** the checks and new tests cover supported Linux/macOS behavior without new native-Windows obligations under decision 0063
- **AND** no live provider call, resume qualification, workflow-runner invocation or push is required or claimed by this capability

## Provenance

- `2026-09-24-codex-0156-boxed-seat` — folded 2026-09-24
