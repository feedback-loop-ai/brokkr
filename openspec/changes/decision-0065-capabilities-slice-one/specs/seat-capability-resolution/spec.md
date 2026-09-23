## Purpose

Resolve portable office requests into exactly the capabilities each executable
seat may hold, with explicit refusals and recorded optional losses.

## ADDED Requirements

### Requirement: Requests use abstract names and a closed strength vocabulary

Agents and inline executable sites SHALL accept a capabilities map from
abstract capability names to exactly requires or wants. Capability names
SHALL remain open; adding an operator-defined abstraction SHALL NOT require
a provider-name match arm or a new engine release. Strict library and site
lints SHALL validate names and values, resolve abstract classes from the
operator's capabilities/ definitions as specified in tool-dialect-contract,
and reject concrete-grant data or request-side class overrides. Missing
abstract metadata SHALL be an invalid declaration before requires/wants
resolution, not an optional capability gap. Absence SHALL mean no requests for
an inline site and inheritance for a site naming an agent.
Request sources SHALL reject duplicate capability names and repeated
`capabilities` fields from original source bytes before ordinary map
conversion, inheritance, selection or composition can overwrite an earlier
value. This applies to agent files and every direct/nested executable body
in each loaded recipe layer, including data later overridden or subtracted.
The complete diagnostic SHALL name the source, repeated key and source
location; duplicate values are invalid even when equal.

#### Scenario: A new name is a request rather than an implicit grant

- **GIVEN** the operator's capabilities/operator-library-docs.json defines {"name":"operator-library-docs","classes":["reads","egress"]} independently of any dialect
- **WHEN** an office requests operator-library-docs as wants
- **THEN** semantic library lint accepts the request without a built-in catalogue entry or selected provider
- **AND** without a realm grant compilation records its drop, not a fabricated implementation

#### Scenario: Misspelled strengths do not become optional

- **WHEN** an agent or site supplies required, optional, true, null or a concrete dialect object as a request value
- **THEN** it refuses with the declaration, capability and the requires/wants vocabulary
- **AND** the erroneous request cannot be silently dropped

#### Scenario: M2 duplicate strength cannot become a weaker request

- **WHEN** original agent or inline seat JSON writes `"capabilities":{"web-search":"requires","web-search":"wants"}`, reverses those strengths, or repeats an equal strength
- **THEN** loading or compilation refuses the duplicate web-search key with the entire source/key/location diagnostic
- **AND** no optional drop, later map value or first-value preference replaces that syntax refusal

#### Scenario: M2 repeated maps cannot erase an earlier requirement

- **WHEN** an agent or site repeats its `capabilities` field, including an earlier required map followed by `{}`
- **THEN** it refuses the repeated capabilities field from source bytes with the complete diagnostic before semantic resolution

#### Scenario: M2 nested and composed request sources remain strict

- **WHEN** either duplicate form appears independently in a panel member, sequence step, selected-case body, nested body, leaf layer or inherited ancestor later overridden by the leaf
- **THEN** every case refuses the repeated key in its original source and location rather than validating only the surviving flattened map
- **AND** equivalent valid unique-key inputs retain omission, inheritance and subtraction behavior
- **AND** removing source-byte duplicate enforcement makes those exact refusal assertions fail; restoration passes

### Requirement: A seat can subtract but cannot widen its office

For an agent-backed site, an omitted capabilities map SHALL inherit the
agent's asks. An explicitly supplied map SHALL be a subset of the agent's
map with unchanged strengths: omitted entries in that explicit map are the
seat's subtractions. An empty map SHALL subtract all asks. A seat SHALL NOT
add names to, or change requires/wants within, the agent's map. An inline
site's map SHALL be its office asks. These rules SHALL apply uniformly to
ordinary seats, panel members, sequence steps and selected case bodies,
including their inherited/composed forms.

#### Scenario: Explicit narrowing computes office asks minus seat subtractions

- **GIVEN** researcher asks web-search: wants and library-docs: requires
- **WHEN** a seat hires researcher with capabilities containing only library-docs: requires
- **THEN** web-search is subtracted before realm intersection, even if the realm grants it
- **AND** the seat does not hold or enable web-search and its prompt identifies the subtraction

#### Scenario: Omission differs from an explicit empty map

- **WHEN** one seat hires researcher without a capabilities key and another hires it with capabilities: {}
- **THEN** the first inherits all researcher asks and the second subtracts all of them
- **AND** subtracting an inherited requires is deliberate narrowing, not an unmet remaining requirement

#### Scenario: Seat overrides cannot alter the office contract

- **WHEN** a seat adds web-fetch absent from its agent's asks or changes an inherited requires to wants
- **THEN** compilation refuses naming the seat, office and unauthorized addition or strength change

#### Scenario: All executable forms enforce the same subtraction

- **WHEN** identical asks, explicit subsets and realm facts are placed at a top-level seat, panel member, sequence step and selected case body
- **THEN** each resolves the same held capability names with its own stable site label
- **AND** no nested or composed form bypasses the strict vocabulary or authority rule

### Requirement: Compilation holds only the request and applicable grant intersection

After subtraction, the compiler SHALL intersect the remaining office asks
with realm grants scoped to that office. A remaining requires without a
usable grant SHALL refuse compilation naming seat, capability and realm,
with the complete reason. A wants without one SHALL be dropped and recorded
in manifest notices with the same context and reason. A realm grant without
an ask SHALL never enable the capability.

#### Scenario: A missing requirement refuses by name

- **WHEN** seat research in realm private requires web-search and private grants nothing
- **THEN** compilation refuses with the complete reason that seat research requires capability web-search but realm private does not grant it to office researcher
- **AND** the assertion checks the entire diagnostic, including those identities and the denial cause

#### Scenario: An unmet want is visible

- **WHEN** the same seat instead wants web-search
- **THEN** compilation succeeds without that holding
- **AND** the manifest has a deterministic notice naming research, researcher, private and web-search and saying it was dropped because the realm does not grant it to that office

#### Scenario: Office scope is part of the grant

- **WHEN** private grants web-search only to another office and research requires it
- **THEN** compilation refuses naming research, web-search, private and the office-scope exclusion
- **AND** changing the request to wants records the same exclusion as a drop

#### Scenario: Removal proves the intersection

- **WHEN** a positive requires case is compiled, then its grant is removed, then its office is removed from scope in a separate case
- **THEN** each negative case refuses for its precise missing authorization
- **AND** removing the corresponding compiler enforcement makes its full-reason assertion fail; restoring it passes

#### Scenario: Removing optional notice recording is caught

- **WHEN** recording the missing-grant wants notice is removed while compilation still drops the capability
- **THEN** the exact notice assertion fails for the missing explanation
- **AND** restoring recording restores the pass; an empty held set alone is insufficient proof

### Requirement: Provider compatibility cannot expand a holding

An applicable provider-native dialect SHALL be usable only by the provider
and native adapter key it names, with expressible admitted tools and
restrictions. Realm-grant validity SHALL be established first under
realm-capability-grants; invalid grant data SHALL not be an optional
compatibility gap. Schema-valid but inexpressible native restrictions SHALL
follow the CQ1 requires/wants/unused precedence in that delta. A granted but
incompatible requires SHALL refuse with seat, realm, capability, dialect,
provider and cause. An incompatible wants SHALL
be dropped with that complete reason in manifest notices, leaving the native
capability OFF subject to independent valid-denial requirements. Known
unheld native power with absent, unsupported or unmeasured OFF authority
SHALL still refuse. Every executable fallback candidate SHALL be checked and pinned against these rules; choosing
another candidate SHALL NOT grant a capability from an unrelated provider,
realm or office.

#### Scenario: A different provider is not a silent dialect substitution

- **WHEN** a Claude seat requires web-search and its realm selects a Codex-only native search dialect
- **THEN** compilation refuses naming the seat, realm, web-search, selected dialect and claude as unable to carry that binding
- **AND** it does not substitute a Claude dialect without a realm edit

#### Scenario: An incompatible want is an explicit loss

- **WHEN** the same request is wants
- **THEN** the seat compiles without web-search and a notice records the dialect/provider mismatch

#### Scenario: CQ1 restriction incompatibility follows the same strength rule

- **WHEN** the valid search-native grant's allow.hosts is inexpressible by a serving test-native candidate
- **THEN** requires and wants produce exactly the complete refusal and drop notice in realm-capability-grants' CQ1 scenarios, respectively
- **AND** removing the compatibility check makes both assertions fail, while removing wants notice recording or OFF composition independently breaks its notice or final-argv assertion
- **AND** all checks are restored for the final pass; an unrelated compile error is not removal proof

#### Scenario: CQ2 optionality cannot hide missing class metadata

- **WHEN** researcher requests operator-library-docs without its operator definition, even with no grant or selected dialect
- **THEN** semantic library lint and compilation refuse the missing definition as specified in tool-dialect-contract before provider selection or optional-drop recording
- **AND** supplying the valid reads/egress definition converts that failure to the ordinary missing-grant refusal for requires or the recorded drop for wants

#### Scenario: Fallback cannot borrow another candidate's controls

- **WHEN** a model chain contains native-compatible and native-incompatible candidates
- **THEN** each runnable candidate has its own validated holding and native denial controls
- **AND** a requires incompatibility cannot survive as an executable candidate, while a wants loss remains recorded for the affected candidate and appears correctly if it serves the seat
- **AND** existing fallback eligibility does not become a new source of authorization

#### Scenario: Provider compatibility is proved by removal

- **WHEN** the provider/dialect compatibility check is removed independently for the requires and wants cases
- **THEN** the requires test fails at its full refusal-text assertion and the independently runnable wants test reaches and fails its exact drop-notice assertion without executing an earlier required-refusal assertion
- **AND** restoring the check restores both results

#### Scenario: M4 optional provider compatibility proof is independent

- **GIVEN** a wants-only case with valid metadata, a mismatched provider binding and supported native OFF
- **WHEN** provider compatibility enforcement itself is removed while optional-notice recording and OFF composition are unchanged
- **THEN** the test reaches and fails its full expected provider-mismatch notice equality without any required case preceding it
- **AND** restoring compatibility enforcement restores that exact notice and the pass

#### Scenario: M4 optional restriction compatibility proof is independent

- **GIVEN** a separate wants-only CQ1 case with a schema-valid inexpressible restriction and supported native OFF
- **WHEN** restriction compatibility enforcement itself is removed while notice recording and OFF composition are unchanged
- **THEN** the test reaches and fails its exact CQ1 restriction-drop notice equality without any required-refusal assertion preceding it
- **AND** restoring the check restores the exact notice and the pass
- **AND** both experiments record mutation, test name, intended assertion, observed failure and restored pass; a required-case failure, fallback holding check or different notice/OFF mutation is insufficient

### Requirement: An impossible native denial is a compile refusal

For each known native capability of a serving provider, the compiler SHALL
consult its native-control declaration even when the office requests nothing
and even when hands has network false. A native capability whose OFF control
is declared unsupported with a measured reason SHALL make seating on that
provider refuse when the capability is unheld. Missing, legacy or unreadable
metadata and unmeasured OFF SHALL likewise refuse for a known unheld power
without a valid delivered denial plan; declaring uncertainty is not permission
to launch. An unheld capability SHALL NOT be left on merely because the realm grants it to someone else or the
seat subtracted it. Unsupported OFF SHALL NOT become an optional-drop path.

#### Scenario: Native default power is checked without a request

- **WHEN** a provider declares web-search OFF unsupported with a measured reason, seat implement requests no capabilities and realm private grants none
- **THEN** compilation refuses naming implement, private, web-search and the provider, includes the measured reason, and says the ungranted native capability cannot be disabled
- **AND** the same refusal holds for boxed and unboxed seats

#### Scenario: Wants does not excuse impossible denial

- **WHEN** the same seat wants web-search without a realm grant
- **THEN** compilation still refuses for the impossible OFF control instead of succeeding with only an optional-drop notice

#### Scenario: Removal proves the native refusal

- **WHEN** the ungranted unsupported-OFF check is removed from the preceding test and then restored
- **THEN** its full diagnostic assertion fails for lost enforcement and passes after restoration

### Requirement: Any MCP realm grant refuses until slice two

A realm entry selecting an mcp tool dialect SHALL refuse compilation in
slice one, naming the realm, capability, dialect and decision 0065 slice
two's absent MCP broker implementation. This refusal SHALL precede optional
request dropping and SHALL apply even if no seat requests the entry or its
office scope is empty. The compiler SHALL NOT launch a server.

#### Scenario: Optional MCP requests cannot mask the missing slice

- **WHEN** realm private grants library-docs using MCP dialect docs-mcp and a seat merely wants it
- **THEN** compilation refuses with the complete reason that realm private grants capability library-docs through dialect docs-mcp of kind mcp, whose broker support is not implemented until decision 0065 slice two

#### Scenario: An unused MCP grant still refuses

- **WHEN** the same entry has no requesting seat or has offices: []
- **THEN** compilation produces the same slice-two refusal rather than silently ignoring the grant

#### Scenario: Removal proves the unbuilt-kind fence

- **WHEN** the compiler's MCP-kind refusal is removed and its tests run
- **THEN** their full-reason assertions fail even for the optional and unused grant cases
- **AND** restoring the check restores the pass

### Requirement: Legacy concrete permissions cannot grandfather a capability

Existing workspace hands and local command restrictions SHALL retain their
meaning. Legacy nonempty tools.mcp requests SHALL NOT launch a capability
server from agent or recipe data; they SHALL refuse with a migration reason
pointing to abstract requests and realm tool dialects. Legacy native-tool
allow entries SHALL NOT authorize capability access. The shipped researcher
SHALL request web-fetch and web-search abstractly as wants, retaining its
local command restrictions; its old webfetch/websearch entries SHALL no
longer enable anything without a realm grant.

#### Scenario: Empty legacy MCP lists remain harmless

- **WHEN** an existing agent declares tools.mcp: [] and ordinary local command restrictions
- **THEN** it still loads with no capability grant and those restrictions do not authorize external capability access

#### Scenario: A concrete server request is not a second authority path

- **WHEN** an agent's tools.mcp names an actual server, including an optional one
- **THEN** loading or compilation refuses with the complete migration reason
- **AND** it cannot start that server from the adapter's legacy server map

#### Scenario: Research loses undeclared native access visibly

- **WHEN** the migrated researcher compiles in the repository's empty realm
- **THEN** it holds neither web-fetch nor web-search and receives two optional-drop notices
- **AND** its provider's controllable native equivalents remain OFF while existing local command restrictions remain in force


### Requirement: Shipped inline permissions migrate before refusal lands

Every shipped recipe or agent authoring a catalogue flag SHALL migrate to
typed tool declarations before authored-option refusal is enabled. Local
command names SHALL use typed tool permissions, capability names SHALL use
requires/wants plus a realm grant, and permission/boundary choices SHALL be
engine-composed from typed data. Migration SHALL NOT introduce a grant or
silently broaden a restriction. The file inventory SHALL cover adapters,
recipes, agents, extensions and shipped bundles; provider templates SHALL be
explicit engine origins, never string-matched exemptions for recipes.

Typed local permissions SHALL use abstract command names in `tools.allow`,
never harness tool patterns or native-capability aliases. Typed `tools.sandbox`
SHALL accept only read-only, workspace-write and danger-full-access under the
existing realm/boundary authority; it SHALL NOT grant a capability or bypass
hands. Agent-backed seats SHALL only narrow their office's restrictions.
An unrepresentable local restriction SHALL refuse, never silently disappear.
For inline local tools, omission SHALL preserve the existing local default;
`tools.allow: []` SHALL express an empty local allow set. For agent-backed
sites, omission SHALL inherit the office's local declaration and an explicit
list SHALL only narrow it, with an empty list subtracting all local entries.
Neither form grants native powers; independent native OFF remains required.
Concrete mappings SHALL preserve their exact command-prefix restrictions.

The typed `tools` object SHALL have a closed vocabulary: `allow`, `sandbox`
and the harmless legacy `mcp: []` only. Null, a non-object or an unknown key
SHALL refuse. `allow`, when present, SHALL be a duplicate-free ordered array
of abstract names matching `^[a-z][a-z0-9-]*$`; empty is valid and distinct
from omission, including in an agent definition. `sandbox`, when present,
SHALL be one of the three class strings above, never null or an opaque object.
Malformed declarations SHALL refuse before optional capability handling can
turn them into a drop. Existing original-source duplicate-key refusal applies.

Inheritance SHALL apply separately to each local field. An omitted `tools`
object, an empty object, or an omitted field within it SHALL NOT clear an
agent's corresponding restriction. An explicit allow list SHALL retain its
written order and names, and SHALL be a subset when the agent declares a list.
When the agent declares no allow restriction, a seat MAY introduce a local
list; this only restricts the existing local default. Sandbox narrowing SHALL
preserve or reduce local execution reach: read-only is narrower than
workspace-write, which is narrower than danger-full-access. An agent with no
sandbox restriction MAY be narrowed by a seat's explicit class. These local
comparisons SHALL NOT replace the realm's boundary, authorize a missing hands
fragment, or widen a gate's read-only authority.

These declaration rules SHALL apply to agents and every executable inline or
agent-backed site: ordinary seats, panel members, sequence steps and selected
case bodies, including inherited/composed bodies. A `tools` declaration beside
a panel, sequence or selection container SHALL refuse rather than silently
apply to its children. Refusals SHALL identify the owning agent/source or
executable site, the offending field and the complete validation or widening
cause; boundary conflicts SHALL also name the resolved boundary. No invalid
field SHALL be coerced, ignored or treated as omitted.

Typed lowering SHALL retain the effective local declaration and its concrete
mapping separately from serialized argv. It SHALL preserve each mapped value
and the declared list order, without broadening a command prefix, substituting
a provider default or reclassifying local permissions as a native grant.
Omitted, empty and nonempty local restrictions SHALL remain distinct in the
expected state even where a serving path cannot yet represent one of them.
Hands replacement and realm/boundary checks remain independent constraints.
Producing a private lowering value SHALL NOT by itself admit a previously
unsupported runnable path.

#### Scenario: Unit 3 lowering preserves exact mapped limits

- **GIVEN** an effective direct allow list `["pytest", "cargo"]` and adapter mappings `pytest` to `Bash(.venv/bin/pytest:*)` and `cargo` to `Bash(cargo:*)`, with `--allowedTools` and separator `,`
- **WHEN** the local lowering primitive composes this declaration
- **THEN** its local contribution is exactly `["--allowedTools", "Bash(.venv/bin/pytest:*),Bash(cargo:*)"]`, its typed expectation retains the ordered names and mapped limits, and it grants no native capability
- **AND** the adapter driver template, including any `--permission-mode acceptEdits`, remains a separate template contribution; identical authored bytes never acquire that origin

#### Scenario: Unit 3 lowering retains absence empty and sandbox intent

- **WHEN** the lowering primitives receive omitted allow, explicit `[]`, a nonempty subset, or each valid typed sandbox class
- **THEN** their expected values distinguish all three allow states and preserve exactly read-only, workspace-write or danger-full-access when requested; omitted sandbox remains unspecified
- **AND** no empty string joined into an allow flag is accepted as proof of an empty local tool surface, and no provider default is accepted as proof of a sandbox class
- **AND** an unsupported representation retains its full refusal; the existing matching Codex hands fragments retain their exact class and hands origin without gaining a second competing local control

#### Scenario: Unit 3 primitives cannot bypass the delivery handoff

- **GIVEN** unit 2's refused inline direct allow, direct empty or unsupported sandbox path, or a local list dormant beside hands
- **WHEN** unit 3 adds local lowering and origin primitives without unit 4's bundle and dispatch transport
- **THEN** the unsupported paths remain refused and the hands path keeps replacement semantics, without adding a direct allow flag or silently discarding an effective restriction
- **AND** exact lowering tests close only the primitive obligation; compiled delivery, shipped migration and final launch evidence remain in their assigned later units

#### Scenario: Strict typed decoding preserves exact local values

- **WHEN** an agent or executable site declares `tools: {"allow":["git","cargo"],"sandbox":"read-only"}`
- **THEN** its local declaration retains exactly the ordered names `["git","cargo"]` and class `read-only`, subject to existing authority checks
- **AND** replacing the class independently with `workspace-write` and `danger-full-access` retains exactly that class when authority admits it; none becomes a realm boundary or a capability grant

#### Scenario: Malformed tools cannot become defaults

- **WHEN** a declaration supplies a null/non-object tools value, an unknown tools key, null/non-array allow, a non-string allow member, a duplicate or malformed name, or null/non-string/unknown sandbox
- **THEN** each case refuses its specific field and complete cause, without accepting a valid sibling field as a substitute
- **AND** a raw harness pattern such as `Bash(cargo:*)` is not an abstract name, and nonempty legacy MCP still refuses with the migration reason
- **AND** repeated tools, allow or sandbox keys refuse from the original source even when repeated values are equal

#### Scenario: Field omission inherits while an explicit empty list subtracts

- **GIVEN** an agent declares `allow: ["cargo","git"]` and `sandbox: "workspace-write"`
- **WHEN** its seat omits tools, supplies `tools: {}`, or supplies only `sandbox: "read-only"`
- **THEN** the effective allow list remains exactly `["cargo","git"]`; sandbox is respectively `workspace-write`, `workspace-write` and `read-only`
- **AND** `tools: {"allow":[]}` instead retains an explicit empty allow list and inherits `workspace-write`, while `tools: {"allow":["git"]}` retains exactly `["git"]` and inherits that same class
- **AND** for an inline site or agent with no local declaration, omission remains unspecified while `allow: []` remains explicitly empty; neither is rewritten to the other

#### Scenario: A local override cannot widen its agent

- **GIVEN** an agent declares `allow: ["cargo"]` and `sandbox: "read-only"`
- **WHEN** a seat independently adds `git`, supplies `workspace-write`, or supplies `danger-full-access`
- **THEN** each refuses with the site, agent, offending local field and exact addition or widening cause
- **AND** the same-class sandbox and the same or empty allow set preserve their exact values, while an unrestricted agent can be narrowed by an explicit local list or sandbox class

#### Scenario: A local sandbox cannot replace boundary authority

- **GIVEN** a model gate with hands under the realm's `harness` boundary has an otherwise valid adapter read-only fragment
- **WHEN** it requests typed `workspace-write` or `danger-full-access`
- **THEN** compilation refuses the sandbox/boundary conflict with the complete site and boundary cause instead of replacing the gate restriction
- **AND** typed `read-only` preserves that restriction; it cannot supply a missing adapter fragment or admit the same model gate under `open`
- **AND** typed danger-full-access never removes a realm-selected box, changes hands reach or authorizes native web access

#### Scenario: Each executable body owns its local declaration

- **WHEN** the same valid subset, explicit empty and invalid widening are placed independently at an ordinary seat, panel member, sequence step and selected case body, including inherited forms
- **THEN** each has the same effective local values or complete owning-site refusal
- **AND** placing tools on the enclosing panel, sequence or selection container refuses its non-executable placement; it does not create a shared grant or an ignored restriction

#### Scenario: Decoding cannot admit a runnable unrestricted command

- **GIVEN** a typed local declaration is syntactically valid and narrows its office, but the current serving path cannot yet express it
- **WHEN** unit 2 validates inline direct allow, direct explicit empty, or a sandbox requiring new lowering
- **THEN** the pure decoder/narrower preserves the exact value and runnable compilation refuses with the owning site, field and complete unsupported-representation cause
- **AND** authored flags cannot satisfy the missing representation; the refusal remains until the owning lowering and transport work proves delivery

#### Scenario: Existing hands semantics do not require direct-tool support

- **GIVEN** an agent has a valid workspace hands fragment and a syntactically valid local list, while its adapter declares direct tool_permissions unsupported
- **WHEN** a seat inherits or narrows that list under existing hands authority
- **THEN** declaration syntax, duplicates and subset checks still run and the effective value remains exact, while the existing hands replacement supplies no direct local flags
- **AND** the list does not filter broker commands or disable hands when empty; no direct mapping is invented or required for that dormant list
- **AND** where an adapter does map an entry to a native capability, that alias refuses with its migration cause even beside hands; malformed or widened declarations cannot disappear in the hands branch

#### Scenario: An existing sandbox fragment must match the typed request

- **GIVEN** an agent actually dispatches through Codex and all independent model, hands and boundary checks admit it
- **WHEN** it requests read-only at a harness gate or boxed site, or workspace-write at a harness work site
- **THEN** admission additionally requires the selected engine fragment to express exactly that class without a competing control; the exact effective value and selected fragment are retained
- **AND** a harness work fragment expressing workspace-write cannot satisfy a read-only request by being called narrower, and a boxed read-only fragment cannot satisfy a different requested class by silently clamping it
- **AND** a missing fragment, an open work default, an opaque driver named codex, or a Claude permission mode is not evidence of sandbox-class support; the unsupported combination refuses

#### Scenario: Resolved native contributions cannot compete with a typed sandbox

- **GIVEN** a Codex site has an otherwise admissible typed sandbox and a matching selected hands fragment
- **WHEN** its resolved native web-search OFF argv independently adds `--add-dir=/srv/shared` at a harness workspace-write site, `--dangerously-bypass-approvals-and-sandbox` at a harness read-only gate, or `sandbox_workspace_write.network_access=true` at a harness workspace-write site
- **THEN** compilation refuses each competing sandbox effect with the complete bounded option-naming cause and no option value; a legitimate denial beside the competing control does not make the contribution admissible
- **AND** the same admission rule checks actual resolved native ON and restriction contributions as well as OFF contributions; engine provenance or a capability's disposition label supplies no exemption
- **AND** this is a compile-admission obligation of unit 2-fix, independent of later final-launch validation

#### Scenario: Canonical root-changing options cannot contest a typed sandbox

- **GIVEN** a Codex harness work site has typed workspace-write and a matching hands fragment
- **WHEN** each of `--cd /`, `--cd=/`, `-C /` and `-C/` occurs independently in authored argv or the selected hands fragment
- **THEN** each of those eight cases refuses at compilation with the complete bounded cause naming canonical `--cd`, never the path value
- **AND** every checked resolved native ON/OFF/restriction contribution obeys the same refusal for every spelling
- **AND** a grammar classification of inert or an assumed priority between repeated root selectors cannot make uncertain competing root control admissible

#### Scenario: Valid native denial preserves matching typed sandboxes

- **GIVEN** otherwise admissible Codex harness sites request read-only at a gate and workspace-write at a work site, under empty realm grants
- **WHEN** their resolved native web-search OFF contribution is exactly `-c`, `web_search="disabled"`, without a competing sandbox or root control
- **THEN** both compile with the exact requested class, matching selected hands fragment, empty holdings and unchanged native OFF facts
- **AND** checking resolved contributions preserves every valid native denial control; blanket refusal of native configuration cannot satisfy the competing-control rule

#### Scenario: Site-local narrowing cannot contaminate a shared office

- **GIVEN** two executable sites reference the same office and declare different valid local subsets
- **WHEN** they are compiled in either traversal order
- **THEN** each checked site fact retains exactly its own effective fields and identity, and the office's authored source and digest remain unchanged
- **AND** an unspecified checked value is distinct from an unvisited site, and wrapper relocation carries the local value with the other site facts

#### Scenario: An incompatible later candidate cannot hide behind selection

- **GIVEN** the primary candidate can express the effective direct local restriction and a later candidate cannot
- **WHEN** the later candidate is unavailable or capability wants would otherwise be dropped
- **THEN** compilation still refuses the later candidate's complete local restriction cause after existing constitutional checks, rather than accepting only the primary's compatibility

#### Scenario: Typed permissions preserve limits without inline options

- **WHEN** a migrated Claude seat requests its local command subset, or a migrated Codex seat declares its existing sandbox restriction as typed data
- **THEN** the engine alone emits the mapped controls, preserving the exact local limits and native OFF under empty realm grants
- **AND** unknown local names/classes, native capability aliases, agent-backed widening and boundary-incompatible restrictions refuse with full bounded causes

#### Scenario: Omitted and empty local permissions are distinct

- **WHEN** an inline or agent-backed site omits local tools, declares an explicit empty allow list, or narrows an inherited local list
- **THEN** omission retains the applicable default or inherited limit, explicit empty admits no local entries, and a subset preserves only its declared entries
- **AND** neither absence nor subtraction enables a native capability; widening and unrepresentable emptiness refuse instead of restoring provider defaults

#### Scenario: Migration preserves concrete prefix and sandbox limits

- **WHEN** fast/node/preflight/verify and the three Codex recipe files in design's Migration Plan are migrated
- **THEN** `pytest` still maps to `Bash(.venv/bin/pytest:*)`, verify keeps separate `Bash(gh pr view:*)` and `Bash(gh run view:*)`, and Claude permission templates preserve `acceptEdits`
- **AND** standby implement and wager-harness implement retain danger-full-access, while standby review and review-first review retain workspace-write, subject to their existing realm/boundary constraints
- **AND** independent compiled final-command expectations prove these limits, with no added realm grant or unrestricted replacement

#### Scenario: The preflight reviewer has no inline authority flags

- **WHEN** preflight's reviewer and the other inventoried shipped seats migrate
- **THEN** their intended local tool limits are typed, the authored command contains no capability-bearing flag, and compiled final commands preserve those limits with native OFF under empty grants
- **AND** all shipped recipes compile or retain their independently required unsupported-provider refusal; migration cannot replace a narrow limit with provider defaults

#### Scenario: A newly found shipped flag cannot be grandfathered

- **WHEN** the rebase or migration inventory discovers another authored catalogue option
- **THEN** its file is added to the migration evidence and converted before refusal lands, with an exact final-command or full-refusal regression
- **AND** no allowlisted filename, adapter alias or historical success bypasses the rule

### Requirement: Refusal proofs assert the full reason

Each commissioned behavioral finding SHALL first have a regression that fails
on the delivered implementation at its intended assertion, then a repair and
an independent removal/restoration proof. The second chief's named
reproductions SHALL become regressions, not be replaced by nearby examples.
Second M3 SHALL add the missing compiled final-launch positive and its
independent failing removal/restoration. Second V1 SHALL retain the actual
failing coverage gate and require a fresh final-head exact pass. Second L1
SHALL receive an evidence/claim audit, not a fabricated behavior mutation.
Tests of capability compilation SHALL assert the complete diagnostic and
complete optional notice, including every required identity and cause.
Assertions that merely test is_err(), success/failure status or an isolated
substring SHALL NOT constitute the commissioned refusal proof. Removal
experiments SHALL restore the implementation and demonstrate a final pass;
a compiler error or unrelated fixture failure SHALL NOT count as enforcement
proof. Launch proofs SHALL compile the fixture through production admission,
carry its own realm and provider candidate into final boundary/driver
composition, and compare the whole ordered command with an independently
written literal. Manually assembled control objects, resolver argv and
intermediate composer output SHALL NOT establish final delivery.

#### Scenario: A wrong refusal cannot satisfy a denial test

- **WHEN** a missing-grant case instead fails for an invalid model or malformed unrelated input
- **THEN** the expected full capability diagnostic does not match and the test fails
- **AND** the case passes only when it reaches the intended named authorization refusal

#### Scenario: Second-council regressions remain individually accountable

- **WHEN** H1–H6 and M1–M3 are presented for closure
- **THEN** each named chief reproduction has its own indexed exact refusal, complete final-command, consumed-file identity or full doctor-line assertion in the owning suite, including all standalone/inherited identity cases and supported cold/eligible-resume restriction cases
- **AND** each behavioral repair records its baseline red, fix, independent removal failure at the intended assertion and restored pass with revision and test name
- **AND** second M3's new successful final-launch case is independently removed and restored at that boundary; historical intermediate R-H3c does not satisfy it
- **AND** launch matrix rows identify the tested serving shapes and exact unsupported-shape refusals; no row infers a fallback, panel or sequence proof from an ordinary primary launch

#### Scenario: Typed subtraction cannot license an authored denial

- **WHEN** typed request subtraction leaves a seat unheld but it authors an MCP deny list
- **THEN** compilation refuses the authored option under ruling 1
- **AND** removing the inline flag preserves typed subtraction and engine-composed native OFF

#### Scenario: The three earlier removal-found gaps stay closed

- **WHEN** adapter duplicate-key enforcement, sequence fallback plan selection and unmapped resume root selection are independently removed
- **THEN** their existing regressions still fail at the intended full duplicate diagnostic, selected fallback provider plan and operated-root identity assertions respectively
- **AND** the sequence case includes DSH fallback after Codex primary, and the resume case removes the workspace decoy before testing
- **AND** all mutations are restored and the regressions pass before completion is claimed

#### Scenario: macOS canonical fixture roots preserve exact diagnostics

- **WHEN** the repeated-native-key/uncomposable-selection regression creates its temporary fixture on Linux or macOS, including a system temp root reached through a path alias
- **THEN** it canonicalizes that root once at creation and derives fixture paths and the complete expected diagnostic from the same root
- **AND** the `/var` versus `/private/var` spelling difference cannot fail the diagnostic equality or be hidden by weakening it to a substring
- **AND** new slice-one tests are checked for the same root-construction habit; no expected temp path is constructed by gluing a host prefix
- **AND** Linux and macOS executions are reported only when actually observed

## Decisions

M2 applies D3's already declared strictness to request source bytes; map-level
validation is rejected because it cannot recover overwritten authority.
M3's loaded-library semantic lint is required before site resolution, including
unseated requests; optionality cannot forgive a malformed abstract declaration.
M4 rejects the prior task 9.1 completion claim for optional compatibility
removal (retained foundation now task 0.20): required failures cannot prove later assertions ran. The two optional
experiments above must stand alone with their intended failure and restored
pass recorded. The macOS correction preserves exact diagnostic equality by
fixing fixture identity, not by changing refusal semantics or inventing a
temporary pathname. These tests belong to the owning suites, not fixtures/.

Second-council parsing and consumption proofs extend the retained first-repair
invariants; they do not reopen strict source-key parsing, whole-loaded-library
lint or the independently runnable optional notices as different feature work.
Second M3 rejects manual Controls/composer-only proof for an accepted held
restriction: it must survive compile and the production final launch on cold
and actual eligible resume. V1 is a whole-workspace gate obligation and L1 an
observational audit, so neither is satisfied by inventing a behavioral test.
Linux and macOS remain the only hosts; fixture roots are canonical facts.

Ruling 1 supersedes the old authored MCP subtraction positive. Typed request
subtraction and independently proven optional drops survive. R10/R12 require
compiled final-boundary assertions with one canonical temporary root on Linux
and macOS; existing hand-built plan tests cannot close those obligations.

Unit 2 specification clarification (2026-09-23, based on 5a47b090): D5 already
rules explicit empty as no local entries, so reject the loader's historical
"empty is ambiguous" interpretation. Omission is field-wise inheritance;
`tools: {}` is not an escape from an office restriction. Subset checks compare
abstract names without merging provider patterns. The three sandbox classes
are ordered only for local narrowing; 0046's boundary and hands law remains
independent and wins over a conflicting local request. These choices are
encoded in the scenarios above. Raw flags, opaque settings and concrete server
requests are rejected as alternative typed representations under ruling 1.

This clarification specifies decoding and local validation for task 2.1. It
does not close later lowering, origin propagation, migration or final-command
proofs. Retaining an explicit empty value is a decoding obligation; proving
that the provider actually receives an empty restriction remains a separate
obligation in the named later units. No success may claim enforcement by merely
accepting and discarding the typed declaration.

Unit 2 design answers (2026-09-23, based on d9816374): the new scenarios apply
SCM's unrepresentable-restriction refusal to intermediate commits too. Pure
value acceptance cannot mean runnable acceptance while lowering is absent.
The existing SC7 hands meaning is retained: direct mapping support is required
where direct restrictions apply, not merely because an agent with hands still
carries a dormant list. Reject the blanket requirement to invent direct-tool
support for hands; reject using that exception to skip syntax, narrowing or
a known native-alias refusal. A matching existing Codex engine fragment is the
bounded sandbox representation for unit 2, as enumerated in design D5.3;
unknown or conflicting representations refuse. These are compile-admission
proofs, not provider live measurements or final-launch proof. Unit 2 does not
change the three sandbox classes, realm grants or the accepted unit order.

Unit 2-fix specification adoption (2026-09-23, based on 6a7044e5): adopt the
chief's third-review S1/A1 findings as omissions against D5.3/D5.5 and SCM's
existing no-competing-control scenario. The explicit scenarios above record
the commissioned acceptance cases, without changing sandbox classes, grants,
provider support or the accepted unit order. S1 requires checking resolved
native contributions while preserving their valid denial meaning; neither an
engine origin exemption nor blanket native-config refusal answers it. A1 uses
the existing canonical option identity across all checked contributions.
Reject the panel's unestablished claim that a later `-C` necessarily wins:
the demonstrated fault is admission of an uncertain competing root selector,
not a proven provider escape. No provider was launched in the chief's cold
command reproduction. No new design choice or upstream specification fault
is introduced; implementation and independent refusal proofs remain open in
2-fix, not deferred to units 13–15. Decision 0066 remains proposed.

Unit 3 specification clarification (2026-09-23, based on b4839426): adopt
D5's separation of local typed input, concrete mappings and emitted argv.
Reject treating an adapter template or a generated local permission as
recipe-authored merely because Candidate currently flattens it into the base
array. Reject inferring an empty restriction from a joined empty string or a
sandbox class from provider defaults. These primitives do not relax D5.3's
admission guards: unit 4 must prove any newly admitted path reaches dispatch.
The existing three sandbox classes and hands replacement semantics stand;
no new provider mapping or live-enforcement claim is made here. Unit 3 closes
3.1 only after its own proofs; 4.2 and all later delivery proofs remain open.
