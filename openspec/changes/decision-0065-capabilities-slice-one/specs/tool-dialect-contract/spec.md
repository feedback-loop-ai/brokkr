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

#### Scenario: A dialect cannot authorize an authored flag

- **WHEN** a valid provider-native dialect and realm grant exist but a seat also authors its harness's ON or OFF flag
- **THEN** the authored option still refuses; only typed requests resolved through that dialect supply engine controls
- **AND** both adapter-declared argv halves must parse at load even when this dialect selects only one
- **AND** concrete control data retains engine origin through dispatch and still must pass the final exact-state check; a dialect cannot install new grammar or exempt its argv from validation

#### Scenario: Mixed and unknown implementations refuse

- **WHEN** a dialect declares an unknown kind, omits serves, omits its kind's required binding, or combines native and MCP implementation fields
- **THEN** schema validation and loading refuse with the file, invalid field and complete reason
- **AND** neither executes anything while diagnosing it

### Requirement: Capability classes belong to the abstraction

Each capability SHALL carry a nonempty, duplicate-free set drawn only from
reads, writes and egress, with multiple classes permitted. The operator's
abstract definition at capabilities/<name>.json SHALL be the authoritative
source of that set. The file SHALL contain its name and classes, with the
name matching the filename. These definitions SHALL describe no provider,
server, tool, dialect choice or grant. Shipped web-search and web-fetch
abstract definitions SHALL each declare reads and egress independently of
the provider-native dialect files.

The definition root SHALL be capabilities/ beside the active realms.json,
shared by the realms in that map. Without a map it SHALL be capabilities/
under the resolved operated repository root. Compilation and library lint
SHALL use that explicit operator context, never search a recipe's directory,
borrow a neighboring configuration's definitions or infer classes from an
adapter, chosen dialect, capability spelling or model. Existing library-root
selection SHALL NOT implicitly change the definition authority. Definition
paths SHALL remain contained in that root; loading SHALL perform no network
lookup. A missing root SHALL be an empty definition set, not built-in or
provider-derived metadata.

The definition loader SHALL validate every definition in that root and
refuse malformed or conflicting declarations. Semantic library/site lint
SHALL resolve every request in its loaded agents and inline sites, including
asks later subtracted, against that one definition set. Compilation SHALL
also resolve every grant in the selected realm, including unused grants.
Missing class metadata SHALL refuse before requires/wants resolution, even
without a selected dialect; optionality SHALL NOT forgive an invalid abstract
request. Compilation SHALL perform that semantic lint over every agent in a
library it loads, not only agents referenced by an executable seat. An unreferenced
agent's invalid request SHALL refuse with its agent name, capability and
complete definition cause before any seat launches. This does not require
loading unrelated libraries. Request syntax parsing alone or a separate CLI
readout SHALL NOT count as successful semantic library lint during compile.

Every serving dialect loaded for those grants SHALL agree with the abstract
class set; any class annotation it declares SHALL be an equality assertion,
never an override or a substitute for a missing definition. The consistency
scope SHALL be the operator definition set, loaded requests and selected-realm
grant dialects, not every unrelated dialect file in the repository. Agents
and seats SHALL continue to request names using the decision's map, for
example capabilities: {"web-search": "wants"}; they SHALL NOT redefine
classes or carry an implementation catalogue. A different provider or dialect
SHALL NOT remove egress or writes from the abstraction to widen eligibility.

#### Scenario: CQ2 supplies an operator-defined abstraction without a dialect

- **GIVEN** the operator configuration root is /world and /world/capabilities/operator-library-docs.json contains exactly {"name":"operator-library-docs","classes":["reads","egress"]}
- **AND** /world/realms.json contains {"schema":"forge.realms/v6","realms":[{"name":"private","path":"repo","default_branch":"main","capabilities":{}}],"journal":".forge/forge.db"}
- **AND** an otherwise valid researcher agent requests {"capabilities":{"operator-library-docs":"wants"}} and seat research hires it in private
- **WHEN** semantic library lint and compilation resolve those inputs with no dialect serving operator-library-docs
- **THEN** the abstract request loads with classes reads and egress and no built-in catalogue entry or provider choice is needed to classify it
- **AND** compilation succeeds with no holding and the complete notice "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability 'operator-library-docs' because the realm does not grant it to this office"
- **AND** the definition alone enables nothing; with requires instead, compilation refuses "seat 'research' (office 'researcher') in realm 'private': requires capability 'operator-library-docs' but the realm does not grant it to this office"

#### Scenario: CQ2 missing metadata is invalid even for an ungranted want

- **WHEN** the preceding example omits operator-library-docs.json while retaining its request, with no selected dialect and no grant
- **THEN** semantic library lint refuses "agent 'researcher': capability 'operator-library-docs' has no abstract definition at 'capabilities/operator-library-docs.json' in the operator configuration; declare its classes before requesting it"
- **AND** compilation of that loaded agent library refuses the same complete agent-lint diagnostic before seat resolution rather than emitting a normal missing-grant drop
- **AND** an inline site's missing definition likewise refuses naming its site, capability and expected definition, for either requires or wants and even if later subtraction would remove an inherited ask

#### Scenario: M3 an unseated loaded agent cannot hide an undefined request

- **GIVEN** a loaded library contains a valid seated worker and unreferenced agent researcher requesting undefined operator-library-docs
- **WHEN** the bundle compiles in its explicit operator context
- **THEN** compilation refuses "agent 'researcher': capability 'operator-library-docs' has no abstract definition at 'capabilities/operator-library-docs.json' in the operator configuration; declare its classes before requesting it"
- **AND** requires, wants and later-subtracted requests obey the same semantic refusal
- **AND** declaring the valid definition restores compilation without granting or inventing a seat holding for the unreferenced agent

#### Scenario: M3 loaded-library validation is a compiler obligation

- **WHEN** only the compile-time semantic library check is removed for the preceding unseated case
- **THEN** its complete agent-named refusal assertion fails even though standalone CLI lint remains intact
- **AND** restoring compile-time lint restores the pass; a malformed unrelated fixture cannot satisfy the expected cause

#### Scenario: Second H6 whole-library lint does not replace charter consumption pins

- **WHEN** a valid loaded library passes request lint and a seated agent's charter is changed after the bundle compiles, including a library outside all recipe layer roots
- **THEN** semantic lint remains evidence about requests only; dispatch still enforces that selected agent's existing charter digest under capability-manifest-and-prompts
- **AND** a missing layer-file entry supplies no exemption, while unseated request definitions retain their existing consulted pins
- **AND** restoring the charter and retaining valid definitions restores the selected agent's ordinary launch without granting any capability

#### Scenario: CQ2 rejects conflicting implementation metadata

- **GIVEN** the operator definition for web-search declares reads and egress
- **WHEN** a selected realm grant uses native dialect search-native whose class annotation declares only reads
- **THEN** compilation refuses "realm 'private': capability 'web-search' in dialect 'search-native' declares classes [reads], conflicting with abstract definition 'capabilities/web-search.json' classes [reads, egress]"
- **AND** it refuses even for an optional request or an unused grant, without replacing the definition's classes or enabling native search

#### Scenario: Search retains its disclosure class across implementations

- **WHEN** the operator defines web-search as reads plus egress and two independently selected dialects serve it consistently
- **THEN** both bindings resolve the same abstract class set from capabilities/web-search.json
- **AND** replacing one implementation cannot turn a seat-composed search into a reads-only capability

#### Scenario: Invalid or inconsistent definitions refuse

- **WHEN** a definition declares an empty set, duplicate class, the class network, a name different from its filename, or a second conflicting declaration of the same name
- **THEN** loading refuses naming the capability and offending declaration, with the closed vocabulary, naming mismatch or both conflicting sets as appropriate
- **AND** directory order does not choose a winning declaration

#### Scenario: A recipe cannot supply its own class authority

- **WHEN** the operator definition is absent or declares reads and egress but the recipe supplies a local reads-only definition or a class override in an agent or seat
- **THEN** the recipe-local file does not satisfy the missing operator definition and the class override is rejected as outside the request grammar
- **AND** adding a provider-native dialect containing classes cannot cure the missing definition

#### Scenario: Legacy denial needs no inferred abstract definition

- **WHEN** the operated realm at any supported version grants no capabilities, its loaded agents and inline sites declare no requests, and its definition directory is absent
- **THEN** it still loads and known native capabilities still receive valid adapter-declared OFF controls or a complete native-denial refusal, including absent, legacy, unreadable or unmeasured required control data
- **AND** no capability is held or assigned invented classes, and an unmeasured native inventory stays unmeasured

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

## Decisions

CQ2 assigns classification authority to operator-owned abstract definitions
because ruling 1 makes classes independent of implementations and ruling 3
keeps catalogues out of requests. The scenarios above settle missing metadata,
no-grant requests and conflicts without choosing a serving dialect. Treating
a provider's metadata as authoritative is refused: it leaves optional
ungranted names unclassifiable and permits an implementation swap to change
the abstraction. Request strengths remain requires/wants. The JSON example
fixes the definition source and minimum data; additional wire layout is
subsequent design work within these semantics.

M3 closes the distinction between having a lint helper and invoking it during
compilation. The semantic input is the entire loaded library, not the subset
of agents that happen to be seated. The earlier CQ2 example's seat-level
missing-definition error is superseded for agent libraries by the same named
agent-lint error compilation must now emit first; inline sites retain their
site-specific diagnostics. This ordering keeps the requirements coherent and
does not broaden class or grant authority.

Second H6 preserves whole-library lint/pinning from the first repair and rejects
using it as proof of charter integrity at consumption. The library owns its
charter's existing pin even outside recipe layers; the manifest-and-prompts
delta owns enforcement at dispatch. No new grant source, tool dialect kind,
manifest version or loaded-library expansion is introduced by this correction.

The 2026-09-23 ruling retains abstraction ownership and realm-only dialect
selection. Adapter declarations supply concrete controls, not a second grant
source or permission to reconcile authored flags. All declared halves are
subject to native-capability-controls load and final-command requirements.
