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
The repository's own realms.json SHALL remain without grants.

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
schema and passed through unchanged as structured data to that binding and
the capability manifest. Reserved keys SHALL retain their engine-defined
meaning. Invalid values or a native binding unable to express an admitted
restriction SHALL refuse rather than ignore it or claim it was enforced.

#### Scenario: A valid restriction survives each boundary

- **WHEN** a test native binding supports a dialect-defined restriction object and the realm provides a valid value
- **THEN** resolution, the binding's configuration and the manifest carry that value unchanged
- **AND** changing the restriction changes the manifest digest

#### Scenario: Schema acceptance is not native enforcement

- **WHEN** a restriction validates but its native binding cannot express it
- **THEN** compilation refuses naming the realm, capability, dialect and unsupported restriction
- **AND** the capability is not launched unrestricted

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
