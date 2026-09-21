## Purpose

Make realms.json the sole source of capability grants, preserving old realm
loading while removing implicit native authority from every realm.

## ADDED Requirements

### Requirement: Realms v6 adds grants without changing frozen versions

The system SHALL add contracts/realms.v6.schema.json with schema
forge.realms/v6, preserving v5 fields and adding an optional per-realm
capabilities map. Each entry SHALL select a tool dialect, with optional
tools subset, offices scope and dialect-defined restriction keys. Existing
versions v1–v5 SHALL keep loading and grant no capabilities. Missing map,
missing capabilities and an empty capabilities map SHALL all grant nothing.
The repository's own realms.json SHALL remain without grants. Abstract
capabilities/ definitions SHALL be separate operator data under the lookup
rules in tool-dialect-contract; their presence SHALL NOT constitute a realm
grant or require changing a legacy map's frozen schema.

#### Scenario: Every older version denies by absence

- **WHEN** each valid forge.realms/v1 through forge.realms/v5 map is loaded and a Codex seat compiles against its realm
- **THEN** the map loads with no capability grants and the seat's web-search is composed OFF
- **AND** no migration or historical use grants it implicitly

#### Scenario: Absence and an explicit empty map are equivalent

- **WHEN** the same seat compiles without a realms map, in a v6 realm omitting capabilities, or in a v6 realm with capabilities: {}
- **THEN** each holds no requested capability and enforces the same native denial policy

#### Scenario: New fields cannot hide under an older version

- **WHEN** a v1–v5 realm supplies capabilities even as an empty map
- **THEN** loading refuses naming the realm, capabilities and forge.realms/v6 as the version that admits the field
- **AND** existing realm contracts retain their frozen bytes

#### Scenario: This repository authorizes no search

- **WHEN** the shipped realms.json is loaded after implementation
- **THEN** every realm in it grants no capabilities
- **AND** upgrading its schema, if done, does not add any grant

### Requirement: Only an operator's realm selects concrete implementations

Recipes, agents, inline seats, panel members, sequence steps and selected
case bodies SHALL declare abstract requests only, never a grant catalogue,
tool dialect selection, native provider key, server definition or restriction
that widens authority. Tool dialect serves SHALL equal the realm capability
key. A missing, invalid, escaping or mismatched dialect reference SHALL be a
named compile refusal. Recipe composition SHALL NOT manufacture realm data.

#### Scenario: The office remains portable

- **WHEN** an office requests web-search and two independently configured realms select different valid native dialects serving it
- **THEN** the office declaration and charter need no server, provider or tool-name edit
- **AND** each realm alone determines the concrete binding

#### Scenario: A recipe cannot supply the missing grant

- **WHEN** an agent or executable site attempts to put dialect, server, tools or grant restrictions in its capability request
- **THEN** loading or compilation refuses naming the site and explaining that requests use requires/wants and grants belong to realms.json
- **AND** a recipe inherited through composition cannot evade that refusal

#### Scenario: A dialect cannot serve a different abstraction accidentally

- **WHEN** realm private grants web-search using a dialect whose serves is web-fetch
- **THEN** compilation refuses naming private, web-search, the dialect and its actual serves value

### Requirement: Office scopes and tool subsets only narrow a grant

Omitted offices SHALL make a grant available to all requesting offices;
an explicit list SHALL make it available only to the named offices; an empty
list SHALL make it available to none. Omitted tools SHALL admit the dialect's
named tools; an explicit list SHALL be a subset, never an expansion. Empty
tools SHALL admit no tool and SHALL NOT produce a usable holding: a remaining
requires refuses and a wants is dropped with a reason. Malformed scope/tool
lists and unknown tool names SHALL be rejected.

The office key SHALL be the referenced agent name for an agent-backed site,
and the stable executable site label for an inline site. Provider choice
SHALL NOT change the office. Diagnostics and manifests SHALL also retain the
site label so several seats hiring one office remain distinguishable.

#### Scenario: Scope does not bleed between offices

- **WHEN** private grants web-search to researcher and seats research and implement hire researcher and implementer respectively
- **THEN** only research has an applicable grant
- **AND** implement remains denied even if it uses the same provider or model

#### Scenario: Unused grants do not enable tools

- **WHEN** an office is in the grant's scope but does not request its capability, or explicitly subtracts it
- **THEN** the seat holds nothing from that entry and any controllable native equivalent remains OFF

#### Scenario: Tool narrowing is exact

- **WHEN** a test dialect serves a capability with tools lookup and search and its realm admits only lookup
- **THEN** the holding, composed native controls and manifest admit only lookup
- **AND** a grant adding a tool not declared by the dialect refuses rather than widening the set

#### Scenario: Empty lists do not mean unrestricted

- **WHEN** a grant has offices: [] or tools: []
- **THEN** the first authorizes no office and the second admits no tool
- **AND** a remaining requires refuses with the specific scope or empty-tool reason while a wants records that reason as a drop

### Requirement: Restrictions are validated, carried and pinned without engine interpretation

The realm's restriction values SHALL be validated against its chosen dialect's
schema and preserved unchanged as structured data in the pinned realm grant.
For a held capability the binding's configuration and capability record SHALL
carry those values unchanged. Reserved keys SHALL retain their engine-defined
meaning. The compiler SHALL distinguish grant validity from the serving
provider's ability to enforce a valid grant; it SHALL NOT drop a restriction
while retaining the capability.

All selected-realm grants SHALL pass structural, reference, class-consistency,
tool-subset and restriction-schema validation before seat compatibility is
resolved. Invalid restrictions SHALL refuse compilation irrespective of asks,
strength or office scope. The MCP slice-two refusal and reserved-hands refusal
also SHALL apply independently of demand. These realm-wide refusals SHALL NOT
be converted to optional drops.

After valid native grants are established, inability to express a restriction
SHALL make the binding unusable for that provider candidate. A remaining
requires SHALL refuse; a wants SHALL be dropped with its full compatibility
reason in manifest notices and the native capability SHALL remain OFF. A
grant unused after asks, subtraction and office scope SHALL stay pinned but
inactive; native ON/restriction composition SHALL not be required for that
unused binding. Known native OFF checks SHALL still run independently for
every serving candidate: an unsupported OFF control SHALL refuse even where
a wants could otherwise drop or a grant is unused. Successful cases SHALL
never launch an unrestricted version of the rejected binding.

#### Scenario: A valid restriction survives each boundary

- **WHEN** a test native binding supports a dialect-defined restriction object and the realm provides a valid value
- **THEN** resolution, the binding's configuration and the manifest carry that value unchanged
- **AND** changing the restriction changes the manifest digest

#### Scenario: CQ1 a required inexpressible restriction refuses

- **GIVEN** private grants web-search to researcher through native dialect search-native with schema-valid allow.hosts and the test-native adapter cannot express allow.hosts but can switch web-search OFF
- **WHEN** seat research requires web-search
- **THEN** compilation refuses "seat 'research' (office 'researcher') in realm 'private': requires capability 'web-search' through dialect 'search-native', but provider 'test-native' cannot express restriction 'allow.hosts'; the capability cannot be held under this grant"
- **AND** no seat launches and the restriction is not removed to make the grant usable

#### Scenario: CQ1 an optional inexpressible restriction drops with OFF

- **GIVEN** exactly the same valid grant, dialect, provider and supported OFF control as the preceding scenario
- **WHEN** research instead wants web-search
- **THEN** compilation succeeds without that holding and records "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability 'web-search' through dialect 'search-native' because provider 'test-native' cannot express restriction 'allow.hosts'; native capability remains OFF"
- **AND** the final argv contains the adapter's OFF disposition, never an unrestricted ON disposition
- **AND** the manifest retains the grant's original restriction as realm context, and the prompt explains the lost capability without claiming the restriction was enforced on an enabled tool

#### Scenario: CQ1 an unused inexpressible grant stays inactive

- **GIVEN** exactly the same valid grant, dialect, provider and supported OFF control as the preceding scenarios
- **WHEN** no seat asks for web-search or every asking seat subtracts it
- **THEN** compilation succeeds without a holding or native ON/restriction configuration for that entry
- **AND** all serving test-native seats receive OFF and the unchanged restriction remains pinned in realm context
- **AND** no optional restriction-compatibility notice is invented for a nonexistent or subtracted ask
- **AND** if a remaining ask is instead excluded by office scope, requires refuses for scope and wants records its scope-drop notice before restriction compatibility is relevant

#### Scenario: CQ1 invalid restrictions refuse before optionality

- **WHEN** allow.hosts in any of those three cases has a value forbidden by search-native's schema
- **THEN** compilation refuses naming private, web-search, search-native and the complete allow.hosts schema violation even for wants or an unused grant
- **AND** no compatibility drop substitutes for the invalid-grant refusal

#### Scenario: CQ1 impossible OFF overrides an optional drop or unused grant

- **WHEN** the optional or unused case instead has web-search OFF declared unsupported with a measured reason
- **THEN** compilation refuses with the complete native-denial reason naming research, private, web-search, test-native and its measured OFF limitation
- **AND** an existing realm grant cannot authorize a capability that the seat does not effectively hold

### Requirement: Realm context is resolved before capability authorization

Compile, run and rerun SHALL authorize against the operated repository's
realm, not a neighboring realm or the recipe's home. Resume SHALL use the
run's pinned realm and capability identity; it SHALL NOT borrow newly added
workspace grants or restore the old default-ON behavior from a legacy run.
Any inability to reproduce an authorized identity SHALL use the existing
manifest-mismatch refusal, with the capability difference named. Starting a
run SHALL refuse a world whose operated realm has different capability
authority from the context under which the bundle compiled, before creating
the run or spawning any seat.

#### Scenario: Neighboring realms do not share a grant

- **WHEN** a world contains public with web-search and private without it and the operated repository belongs to private
- **THEN** compiling or rerunning the same recipe denies search in private despite public's grant

#### Scenario: Resume cannot acquire a grant from an edited map

- **WHEN** a run pinned no web-search grant and the workspace map now grants it
- **THEN** with the original capability-capable inputs unchanged, resume reproduces the pinned no-grant context and composes OFF despite the edited map
- **AND** a supplied bundle that changes that pinned capability identity is refused by manifest comparison rather than silently enabling search

#### Scenario: A pre-capability manifest gets no grandfathering

- **WHEN** an older run manifest lacks capability authorization and resume is attempted after this slice
- **THEN** compilation adds the explicit denied capability identity required by v11 and the old manifest comparison refuses that identity change
- **AND** no pre-capability privilege is restored and historical records are not rewritten


#### Scenario: A start cannot substitute a different grant context

- **WHEN** a bundle compiled for one capability grant context is started with an operated realm whose capability grants differ
- **THEN** start refuses the inconsistent capability authority before creating a run or spawning a seat
- **AND** a matching context starts under its compiled holdings

## Decisions

CQ1 replaces the earlier unconditional native-restriction refusal with a
validation/compatibility distinction. Ruling 5's unusable requires/wants
semantics govern a valid native binding: dropping the whole optional
capability with OFF preserves every restriction and grants less power.
Discarding a restriction and launching unrestricted is refused. Refusing
all optional or unused valid grants solely for an inexpressible restriction
is also rejected because neither grants an unrestricted capability. Invalid
grant data, unbuilt kinds and impossible native denial remain independent
refusals. This does not relax existing local tool-permission restrictions,
which constrain the continuing seat rather than an optional capability.
