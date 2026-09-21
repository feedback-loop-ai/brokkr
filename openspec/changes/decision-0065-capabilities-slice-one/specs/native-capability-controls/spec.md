## Purpose

Declare native provider powers and compose their controls from realm-authorized
holdings, keeping measured enforcement separate from unverified provider claims.

## ADDED Requirements

### Requirement: Native capability controls are adapter-owned evidence-bearing data

Adapters SHALL declare their native capability inventory and, per known
capability, how to enable it and disable it. Supported controls SHALL be
adapter data, following the existing tool_permissions pattern; impossible
controls SHALL use unsupported with a nonempty measured reason. Unknown
inventory or unverified controls SHALL be represented explicitly as unmeasured
with a nonempty reason, not as an empty supported map, unsupported-by-analogy
or a fabricated switch. Provider-native tool dialects SHALL reference these
adapter keys and SHALL NOT themselves confer authorization.

#### Scenario: Missing halves and reasons are rejected

- **WHEN** a native declaration asserts support without an ON or OFF disposition, or asserts unsupported/unmeasured without its reason
- **THEN** loading refuses naming the adapter, capability or inventory, missing field and complete cause
- **AND** no missing control becomes an empty success by default

#### Scenario: A measured default can be stated as the ON mechanism

- **WHEN** the Codex declaration identifies the measured cold-exec default as its ON mechanism and the disabled configuration as its OFF mechanism
- **THEN** the declaration explicitly records both dispositions and the evidence scope
- **AND** the engine does not invent a measured explicit ON configuration value

#### Scenario: Missing inventory is not a verified empty inventory

- **WHEN** an older third-party adapter has no native capability assessment
- **THEN** its native inventory is reported as unmeasured because the declaration is absent
- **AND** absence neither grants a capability nor establishes verified native denial
- **AND** a recognized provider already known to have an unheld native capability refuses compilation unless a valid denial plan can still be established and delivered

#### Scenario: An adapter declaration is not a grant

- **WHEN** an adapter lists supported web-search and a dialect binds it but the realm grants nothing
- **THEN** the seat receives its OFF control rather than its ON control

#### Scenario: CQ1 dropping a restricted want composes denial

- **WHEN** a valid native grant has a schema-valid restriction its selected provider binding cannot express and the seat only wants the capability
- **THEN** the capability is dropped with the complete CQ1 compatibility notice and the final argv uses its supported native OFF control
- **AND** neither its ON control nor an unrestricted replacement is composed; a measured unsupported OFF still refuses independently
- **AND** a grant unused after asks, subtraction or scope likewise composes only native OFF, without evaluating an unused ON/restriction configuration

### Requirement: Known native powers require a valid delivered denial or refusal

For every recognized provider known to have a native capability, compilation
SHALL establish a valid control plan for each unheld capability or refuse
before launch. Missing computed authority, absent or legacy assessments,
unreadable/malformed adapter data, unmeasured OFF and errors erased on any
security-relevant load path SHALL NOT become a successful no-control plan.
An adapter's omission SHALL NOT erase an already known native power. Refusals
SHALL name the seat, realm, provider, known capability and the complete cause,
including the failing source and load reason when applicable. No requests,
empty grants, an unused grant or legacy realm data SHALL bypass this check.
The final launch boundary SHALL refuse missing or undeliverable managed
controls rather than accept a plan that merely calls denial unmeasured.

#### Scenario: H1 missing Codex adapter data cannot restore default search

- **GIVEN** an inline Codex work seat requests nothing in a realm granting nothing
- **WHEN** its adapter root is absent, the root contains no Codex declaration, the declaration predates native metadata, or it omits the known web-search power from its inventory
- **THEN** each case either produces the exact final `-c`, `web_search="disabled"` pair from a valid engine plan or refuses with the complete missing-denial cause naming the seat, realm, codex and web-search
- **AND** no case launches with only an unmeasured record or no native control
- **AND** absence of an operator definition directory does not relax denial

#### Scenario: H1 a load failure remains a failure of denial authority

- **WHEN** the same no-ask seat encounters unreadable or malformed Codex adapter data, or an unrelated malformed adapter makes loading the adapter library fail
- **THEN** compilation refuses with the full source/load cause and the affected seat, realm, codex and web-search denial context unless valid independently loaded denial authority remains available
- **AND** an ignored load error cannot produce a launch without OFF

#### Scenario: H1 legacy realms and unmeasured OFF have no exemption

- **WHEN** the absent, legacy and malformed adapter cases are repeated without a realm map and with each supported v1–v5 realm, or a known native power explicitly declares OFF unmeasured
- **THEN** each unheld known power still requires delivered OFF or the complete named refusal
- **AND** an empty holding or a manifest notice alone cannot make the seat launchable

### Requirement: Codex web-search OFF uses the controller's measured fragment

adapters/codex.json SHALL declare web-search with the OFF argv pair "-c",
"web_search=\"disabled\"". Its evidence SHALL cite
.forge/tasks/controller-codex-web-search-switch-2026-09-21.json and limit
the live claim to cold codex exec on codex-cli 0.154.0. A seat without an
effective web-search holding SHALL receive that OFF pair whether it is
agent-backed or inline, boxed or unboxed, and whether it requests nothing,
loses a wants, is out of scope, or subtracts the capability.

#### Scenario: Cold denial and admission cover both boundaries

- **WHEN** otherwise valid Codex seats are composed with boxed workspace hands and without boxed hands, each first without and then with an effective web-search holding
- **THEN** both denied final cold argv contain the exact measured OFF pair
- **AND** both held cases use the adapter's declared ON mechanism without a conflicting OFF pair
- **AND** ordinary sandbox, model, effort, hands and result controls remain intact

#### Scenario: A realm-wide entry alone does not enable the seat

- **WHEN** the realm lists web-search but the seat does not ask for it, subtracts it, or falls outside its offices scope
- **THEN** each final Codex argv contains the OFF pair

#### Scenario: Box networking does not replace the switch

- **WHEN** workspace hands already has network false
- **THEN** an unheld web-search still receives the explicit native OFF pair because provider-server search is not confined by the workspace network

### Requirement: Eligible Codex resumes reimpose the capability control

An actual eligible Codex resume SHALL carry the current pinned capability
control in its final exec resume argv, including the OFF pair when web-search
is not held. Integration SHALL preserve the existing session, version,
sandbox, effort, accounting and boundary eligibility checks. The engine-owned
native control SHALL NOT by itself trigger incompatible-argv and turn every
eligible resume into a cold launch. Arbitrary -c passthrough SHALL NOT become
eligible merely to accommodate this control.

#### Scenario: Resume denial is proved on a resume rather than a fallback

- **GIVEN** a valid currently supported unboxed Codex resume offer, matching measured harness identity, sandbox class and all other eligibility facts
- **WHEN** the seat holds no web-search
- **THEN** the final plan actually invokes exec resume with the offered session and stdin prompt positional
- **AND** its argv includes "-c", "web_search=\"disabled\"" alongside the reimposed sandbox and effort controls

#### Scenario: A granted resume takes the declared ON disposition

- **GIVEN** the same eligible resume setup
- **WHEN** web-search is requested and held through an applicable realm grant
- **THEN** the actual resume argv uses the adapter's ON disposition with no conflicting OFF pair
- **AND** this deterministic composition does not claim live resumed enablement was measured

#### Scenario: Boxed resume eligibility is not widened

- **WHEN** a boxed Codex site whose existing resume shape is unsupported receives a resume offer
- **THEN** its existing eligibility refusal and cold fallback remain in force, and the resulting denied cold argv still contains OFF
- **AND** that cold plan is not counted as the proof of an actual resume
- **AND** this slice does not activate boxed resume to fill a test matrix

#### Scenario: Arbitrary config cannot ride the native-control exception

- **WHEN** an otherwise eligible resume includes a caller-authored arbitrary -c override
- **THEN** the existing unsafe-passthrough refusal remains
- **AND** only the engine-owned, resolved capability control receives the narrowly scoped treatment needed for composition

### Requirement: Neither inline arguments nor fallback can override native denial

All serving Codex paths SHALL enforce the same capability decision, including
inline driver declarations and executable model fallbacks. A conflicting
native enable request in authored arguments SHALL be refused with its source,
capability and missing authority named; native ON/OFF fragment ordering SHALL
NOT accidentally decide authorization. Existing strict MCP configuration and
hands replacement SHALL remain intact.

#### Scenario: A concrete enable flag cannot override the realm

- **WHEN** an inline seat without a web-search holding supplies an argument or config setting that requests native search enablement
- **THEN** it refuses with a complete reason naming the seat, web-search and the conflicting authored control
- **AND** it does not execute with two competing ON/OFF settings

#### Scenario: A fallback Codex candidate is also denied

- **WHEN** an authorized fallback selects Codex for a seat that holds no web-search
- **THEN** its cold or eligible resume command contains the OFF pair just as the primary Codex case does

### Requirement: Other provider declarations preserve the commissioned uncertainty

Claude's native WebSearch and WebFetch controls SHALL be declared through its
adapter and native dialects with their abstract capability names. Its existing
empty native tool list under strict MCP configuration SHALL be described as
adapter evidence, not a live enforcement measurement. Enabling a held native
tool SHALL retain the hands tool and strict MCP policy and SHALL not restore
all built-in tools.

DSH SHALL declare its native inventory/controls unmeasured because unsupported
mcp and tool_permissions do not establish absence of native egress.
LaneTally SHALL declare native inventory/controls unmeasured because they have
not been verified through that wrapper. Unmeasured declarations SHALL NOT
authorize a holding or claim verified denial; they SHALL remain distinct from
a known capability with a measured unsupported OFF control. Neither
unmeasured state SHALL permit a provider already known to have an unheld
native capability to launch without a valid denial plan; the refusal above
applies independently of whether the uncertainty was explicitly declared.

#### Scenario: Claude admits only held native tools beside hands

- **WHEN** a test Claude seat holds web-search but not web-fetch
- **THEN** the composed native tool selection admits only WebSearch for these capabilities, retains workspace MCP when applicable, and keeps strict MCP configuration
- **AND** with neither held the declared native search/fetch controls disable both
- **AND** deterministic argv assertions are labeled composition evidence, not live Claude proof

#### Scenario: DSH's unsupported fields do not prove a negative

- **WHEN** DSH's adapter is read with mcp and tool_permissions unsupported
- **THEN** its native capability assessment remains unmeasured with the commissioned reason
- **AND** neither compiler output nor doctor describes it as having no native egress or verified native denial

#### Scenario: LaneTally does not inherit Claude's evidence

- **WHEN** the LaneTally adapter is read beside Claude's supported native-control declarations
- **THEN** the wrapper's native assessment remains unmeasured and retains its own reason
- **AND** neither Claude's flags nor evidence of wrapper argv forwarding is counted as measured native enforcement

#### Scenario: An unmeasured control cannot satisfy a request

- **WHEN** a seat requires a capability whose selected native binding has unmeasured controls
- **THEN** compilation refuses naming the seat, realm, capability, provider and unmeasured reason
- **AND** if the request is wants it is dropped with that reason only when independent native denial remains valid; a known unheld native capability with unmeasured OFF refuses compilation instead
- **AND** no successful optional drop claims that an unknown inventory has been proven free of native egress

### Requirement: Every accepted native control reaches the final command

Every accepted native argv disposition, selection contribution and restriction
transport SHALL be represented in the final serving command after hands,
local permissions and boundary composition. A representation that cannot be
safely composed for that provider SHALL refuse during compilation, naming the
seat, realm, capability, provider and unsupported representation or conflict.
Successful compilation SHALL NOT record OFF, ON or an enforced restriction
that the launch drops. Controls SHALL compose once, preserving current
restriction arity, duplicate checks, strict MCP and engine-owned hands.

#### Scenario: H3 Claude argv denial is executable denial

- **GIVEN** Claude web-search OFF is declared as argv `["--disallowedTools", "WebSearch"]` instead of a selection contribution
- **WHEN** an unheld seat compiles and its actual cold or eligible resumed command is assembled
- **THEN** the final command denies WebSearch with that disposition composed into the authoritative tool lists, or compilation has already refused that representation with the complete reason
- **AND** accepting the declaration and emitting only selection is a test failure
- **AND** existing hands and local restrictions remain present without conflicting duplicate flags

#### Scenario: H3 a held restriction survives final composition

- **GIVEN** a synthetic supported native grant has a schema-valid nonempty restriction and a declared argv transport, with a supported ON disposition
- **WHEN** a Claude seat holds it and reaches final command construction
- **THEN** the command contains both the accepted ON control and the exact encoded restriction value, or compilation refuses the representation before claiming a holding
- **AND** the original structured restriction remains pinned, and the native capability cannot launch unrestricted
- **AND** rejecting a supported test representation is not evidence that its transport was composed

#### Scenario: H1 through H3 cover every serving path

- **WHEN** denial/admission, authored-configuration and accepted-control regressions exercise inline and agent-backed work/gate seats, primary and fallback links, panel members, sequence steps and selected/inherited bodies, boxed and unboxed, cold and eligible resumed
- **THEN** every supported path asserts the exact final command and effective ON/OFF/restriction disposition, while an unsupported path asserts its complete refusal
- **AND** the actual resume assertions verify the offered session and resumed command; boxed ineligible cold fallback stays a separate case
- **AND** an always-OFF mutation fails authorized ON assertions, and removing a delivered argv or restriction fragment fails its own final-command assertion
- **AND** this matrix tests the current gate launch paths without implementing slice-two gate capability policy

### Requirement: Denial and admission have removal proofs and bounded live claims

Deterministic tests SHALL inspect final composed argv for inline and
agent-backed Codex seats, boxed and unboxed, with and without an effective
holding, plus actual eligible resumed denial/admission. Tests SHALL remove
the relevant OFF composition and observe the denial assertion fail, restore
it and pass; admission tests SHALL independently detect an always-OFF
implementation. Existing ineligible resume cases SHALL keep their reasons.
These tests SHALL NOT substitute for a live provider measurement.

#### Scenario: Removing OFF is caught where it protects the launch

- **WHEN** each cold and eligible resume OFF composition is removed in turn
- **THEN** the corresponding final-argv denial assertion fails for the missing exact pair
- **AND** restoring the control restores the pass without changing the assertion or unrelated eligibility

#### Scenario: Always disabling is not a grant implementation

- **WHEN** the ON path is replaced with unconditional OFF
- **THEN** the held-capability cold and eligible resume assertions fail
- **AND** restoring the declared grant behavior restores them

#### Scenario: Delivery carries every measurement gap forward

- **WHEN** deterministic controls pass but no new controller measurements are supplied
- **THEN** delivery notes still list Codex live resumed denial and enablement, explicit ON values and other versions, Claude live controls, DSH native inventory/controls and LaneTally native inventory/controls as unmeasured and owed to the controller
- **AND** no failed, absent or unrun live check is described as passed

## Decisions

H1 and H3 are adopted as one launch invariant. An unmeasured record describes
knowledge; it is not a valid replacement for mandatory denial of a known
power. The historical distinction between unknown inventory and impossible
OFF remains useful for reporting, but neither may swallow a required control.
Selection-only composition is rejected because the accepted contract also
admits argv dispositions and restriction transports. Tests prove the final
command or full compile refusal, not only a resolver plan. Live Codex resume,
Claude, DSH and LaneTally evidence remains separately unmeasured.
