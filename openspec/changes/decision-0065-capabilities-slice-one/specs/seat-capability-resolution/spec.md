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


### Requirement: Refusal proofs assert the full reason

Each commissioned finding SHALL first have a regression that fails on the
delivered implementation at its intended assertion, then a repair and an
independent removal/restoration proof. Tests of capability compilation SHALL
assert the complete diagnostic and complete optional notice, including every required identity and cause.
Assertions that merely test is_err(), success/failure status or an isolated
substring SHALL NOT constitute the commissioned refusal proof. Removal
experiments SHALL restore the implementation and demonstrate a final pass;
a compiler error or unrelated fixture failure SHALL NOT count as enforcement
proof.

#### Scenario: A wrong refusal cannot satisfy a denial test

- **WHEN** a missing-grant case instead fails for an invalid model or malformed unrelated input
- **THEN** the expected full capability diagnostic does not match and the test fails
- **AND** the case passes only when it reaches the intended named authorization refusal

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
removal: required failures cannot prove later assertions ran. The two optional
experiments above must stand alone with their intended failure and restored
pass recorded. The macOS correction preserves exact diagnostic equality by
fixing fixture identity, not by changing refusal semantics or inventing a
temporary pathname. These tests belong to the owning suites, not fixtures/.
