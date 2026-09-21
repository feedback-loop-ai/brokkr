## Purpose

Define the versioned, operator-owned data that binds a portable capability to
a concrete implementation without putting provider or server knowledge in offices.

## ADDED Requirements

### Requirement: A tool dialect binds one capability to exactly one implementation kind

The system SHALL publish contracts/tool-dialect.v1.schema.json as a new
contract. A dialect under dialects/tools/ SHALL name the abstract capability
in serves and select exactly one kind from provider-native, mcp and reserved
hands. The contract and loader SHALL reject unknown kinds, missing required
data and mixtures of kind-specific data. Loading or validating a dialect
SHALL execute no server, provider or network request.

#### Scenario: Native implementation is adapter data

- **WHEN** a provider-native dialect serves web-search through provider codex and its declared native adapter key
- **THEN** it validates with the provider, adapter key and named tools that realize the capability
- **AND** its concrete identities are not required in an office's request

#### Scenario: Mixed and unknown implementations refuse

- **WHEN** a dialect declares an unknown kind, omits serves, omits its kind's required binding, or combines native and MCP implementation fields
- **THEN** schema validation and loading refuse with the file, invalid field and complete reason
- **AND** neither executes anything while diagnosing it

### Requirement: Capability classes belong to the abstraction

Each capability SHALL carry a nonempty set drawn only from reads, writes and
egress, with multiple classes permitted. The class set SHALL describe the
abstract operation, remain available without choosing a provider, and remain
consistent across declarations and serving dialects of the same capability.
A concrete implementation SHALL NOT remove egress or writes from a capability
to widen its eligibility. The declaration grammar SHALL preserve the
decision's requests map, for example capabilities: {"web-search": "wants"}.

#### Scenario: Search retains its disclosure class across implementations

- **WHEN** web-search is declared as reads plus egress and two dialects serve it
- **THEN** both bindings retain the same abstract class set
- **AND** replacing one implementation cannot turn a seat-composed search into a reads-only capability

#### Scenario: Invalid or inconsistent classes refuse

- **WHEN** a capability declares an empty class set, the class network, or class metadata inconsistent with another declaration of that capability
- **THEN** loading refuses naming the capability, offending declaration and reads/writes/egress vocabulary or the conflicting sets

### Requirement: Dialects describe their disclosure using the existing egress vocabulary

A dialect SHALL describe what it sends, including whether it sends
seat-composed content. Its optional egress class SHALL use exactly local,
contracted or uncontracted; absence SHALL mean uncontracted. This class SHALL
remain separate from the capability's reads/writes/egress classes and SHALL
remain subject to decision 0036's existing rules. A dialect SHALL NOT promote
a provider route, grant secret bindings or change a judging tier.

#### Scenario: Missing disclosure clearance does not imply local

- **WHEN** a valid native search dialect describes sending a model-composed query and omits its egress class
- **THEN** it resolves to uncontracted while the capability still carries reads and egress

#### Scenario: Similar words do not merge the two vocabularies

- **WHEN** a dialect supplies reads as its egress class or a capability supplies contracted as an operation class
- **THEN** loading refuses naming the field and its own closed vocabulary
- **AND** a valid dialect class leaves route and secret-binding checks unchanged

### Requirement: The MCP contract is whole before its runtime exists

The mcp kind SHALL describe exactly one connection form: a stdio launch with
nonempty argv, or a server URL. It SHALL declare the named tools realizing
the capability and a pinned implementation version. Needed credentials SHALL
be references/bindings under decision 0012, never secret values or resolved
credentials in argv, URLs or the dialect. The contract SHALL admit the
dialect's result-retention declaration without implementing retention in this
slice. Schema validity SHALL NOT imply that the compiler can run the kind.

#### Scenario: Both connection forms can be validated as data

- **WHEN** one test dialect declares a stdio MCP server and another declares a URL server, each with version, named tools and any needed named bindings
- **THEN** both validate without installing, starting or contacting a server
- **AND** either used as a realm grant is refused by compilation because slice two is absent

#### Scenario: Incomplete and unsafe MCP declarations refuse

- **WHEN** a dialect omits its pinned version or tools, gives both connection forms, or supplies credentials by value instead of declared binding references
- **THEN** it is rejected with a complete reason naming the offending field
- **AND** the diagnostic does not disclose a credential value

### Requirement: Restriction keys are defined by the selected dialect

A dialect SHALL define the allowed restriction shape using a schema. Realm
grant keys other than the reserved dialect, tools and offices keys SHALL be
validated against that schema and preserved as structured values. The engine
SHALL NOT interpret a domain restriction such as allow.hosts as its own
network policy, fetch a remote schema to compile it, or silently discard a
restriction. Dialect schemas SHALL NOT redefine the reserved grant keys.

#### Scenario: A dialect-defined object passes unchanged

- **WHEN** a dialect schema admits allow.hosts and a realm supplies {"allow":{"hosts":["sourceware.org","yaml.org"]}}
- **THEN** the valid structure and values survive resolution and manifest pinning unchanged
- **AND** the engine does not turn the host strings into a new hands network policy

#### Scenario: Invalid restriction data is not silently accepted

- **WHEN** a grant contains a restriction key its dialect schema forbids or a value of the wrong type
- **THEN** compilation refuses naming the realm, capability, dialect and failing restriction
- **AND** it cannot gain support by ignoring the restriction or fetching an external schema

### Requirement: Reserved hands preserves the existing workspace authority

The hands kind SHALL be reserved for decision 0043's workspace abstraction.
This slice SHALL NOT add a second hands implementation, require a capability
grant to keep existing hands, or use a tools grant to change its boundary,
binds or network. Brokkr SHALL ship only provider-native tool dialect data in
this slice, with no MCP server or general-purpose capability tool.

#### Scenario: Empty grants leave hands working

- **WHEN** an existing seat with workspace hands compiles in a realm granting no capabilities
- **THEN** its existing hands policy and boundary are preserved while additional native capabilities remain subject to denial

#### Scenario: Reserved hands cannot be an alternative grant path

- **WHEN** a realm attempts to select the reserved hands kind as a new tool grant
- **THEN** compilation refuses explaining that hands remains governed by decisions 0043 and 0046
- **AND** no second workspace tool or wider hands policy is composed
