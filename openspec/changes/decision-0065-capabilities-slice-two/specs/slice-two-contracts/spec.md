## Purpose

State the additive public contracts and typed meanings needed before MCP
admission, preserving every published schema version.

## ADDED Requirements

### Requirement: SC1 executable dialect fields survive typed loading

Implementation SHALL retain connection, version, secrets and retained on the
MCP variant, alongside tools, source/digest, abstract class assertion, sends,
egress and restriction schema. Closed kind, connection, egress, retention and
failure vocabularies SHALL be exhaustive enums/typed fields, not strings read
from Value across modules. Loading SHALL perform no server launch, credential
read or network access. File parsing and hashing SHALL use the same contained,
bound bytes, preserving slice one's identity and strict duplicate-key rules.

The frozen tool-dialect v1 already declares these fields and SHALL remain the
source contract. No v2 is minted solely to preserve fields the loader dropped.
Connection remains exactly one nonempty argv vector or credential-free URL;
version is a nonempty expected initialize serverInfo version; secrets is a
unique list of decision 0012 names; omitted retained means false. Schema,
kind, connection, version, secrets, retained and restrictions are checked in
that order, preserving existing source/duplicate/schema causes. Strict typed
parsing SHALL not silently default malformed fields. The new MCP typed path
SHALL return typed errors with the existing safe operator text.

Execution uses direct argv and declared child-environment bindings only.
V1 declared secret references remain valid DATA under the frozen grammar;
they cannot be executed by substitution. After U9 an applicable request with
an argv secret reference SHALL refuse requires or drop wants with cause
"MCP connection argv cannot contain secret references; declare environment
bindings in secrets". URL execution likewise has cause "MCP URL connections
are not implemented in decision 0065 slice two". Both use GP1's complete
required/optional diagnostic forms; unused valid grants stay pinned/inactive.
Neither incompatibility becomes a new schema-wide v1 migration refusal.
Before U9, SD3's existing unbuilt-kind fence wins after structural validity.
Undeclared or malformed references retain the existing loader refusal.

#### Scenario: Typed MCP fields are not discarded

- **WHEN** v1 docs-mcp declares a stdio argv, version 1.2.3, secrets [DOCS_TOKEN] and retained true
- **THEN** the parsed MCP variant retains those exact values and its digest without reading a store or launching a process
- **AND** changing any of those fields moves identity; rotating the secret value does not

#### Scenario: Existing contract validation stays authoritative

- **WHEN** either connection is malformed, both connections are present, version/secrets is missing, retained is nonboolean, or duplicate/unknown/mixed-kind fields occur
- **THEN** the v1 schema and strict loader refuse the existing field-specific cause without echoing credential payloads
- **AND** native retained remains invalid, native v1 behavior remains subject to gate/isolation rules, and valid old MCP data needs no schema rename

#### Scenario: A valid reference is not permission to expose a value in argv

- **WHEN** an otherwise eligible request selects a v1 MCP dialect with a declared argv secret reference after U9
- **THEN** requires refuses and wants drops with the exact argv-reference cause above, with no interpolation or child spawn
- **AND** removing the reference and retaining the declared environment binding makes that connection executable; no shell is added by Brokkr
- **AND** an unused valid reference-bearing grant stays pinned/inactive, while before U9 all valid MCP grants retain SD3's unbuilt-kind refusal

#### Scenario: URL declarations are preserved without pretending support

- **WHEN** an otherwise eligible site requests a v1 URL-connected capability after U9
- **THEN** requires refuses and wants drops with the exact URL cause above
- **AND** no URL request, credential lookup or stdio substitute occurs; an unused valid URL grant stays pinned/inactive

### Requirement: SC2 only the new realm version reserves the retention veto

Implementation SHALL publish **the next realms version after v7**,
preserving all fields of v7 (including #487 provisional offices) and adding reserved grant key retain with only false
admitted. Omission inherits the dialect declaration. The new version's reserved keys SHALL
be dialect, tools, offices and retain; retain SHALL be excluded from dialect
restriction payloads and preserved in grant identity, even when false has no
effect on a native or nonretaining grant.

Old realm versions SHALL retain their exact grammar: v1–v5 grant nothing;
v6 and v7 nonreserved keys, including a literal retain if its dialect schema
admits it, are still restrictions. A v6/v7 restriction SHALL never become the
new veto by spelling alone. Nonempty restriction transport remains D11's
refusal/drop/inactive behavior. A grant in the next realms version after v7 selecting a v1 dialect whose
restriction schema claims retain SHALL refuse an authority collision instead
of changing that schema's meaning. No frozen schema is patched in place.

#### Scenario: A new-version veto cannot be passed to a server as a restriction

- **WHEN** private grants library-docs with dialect docs-mcp and retain false in the next realms version after v7
- **THEN** the typed grant records the veto, restrictions remain exactly the other schema-validated keys, and effective retention is false
- **AND** a dialect schema claiming retain refuses "tool dialect '<dialect>' restriction schema redefines reserved grant key 'retain'"

#### Scenario: An older spelling is not retroactive authority

- **WHEN** a v6 or v7 grant contains retain false and its existing dialect schema admits that restriction
- **THEN** it stays a nonempty restriction, not a retention veto, and reaches only D11's refusal/drop/inactive outcomes
- **AND** an explicit migration to the next realms version after v7 removes that ambiguity and changes manifest identity; an older engine rejects the new schema version

#### Scenario: Invalid veto data refuses before optionality

- **WHEN** a grant in the next realms version after v7 writes retain true or any non-false value, even unused or wanted
- **THEN** it refuses CR1's exact realm/capability veto cause before seat compatibility
- **AND** omission and false are the only valid forms, never coerced from null

### Requirement: SC3 manifest v12 pins MCP execution and effective retention

Implementation SHALL publish run-manifest.v12.schema.json beside v11.
Every candidate's held record SHALL retain all v11 fields and add a closed
implementation record plus retention. Native implementation SHALL identify
kind provider-native, provider and adapter_key. MCP implementation SHALL
identify kind mcp, server name, typed connection, pinned version and secret
names; no resolved value, transient ledger path, process id or call id belongs
in manifest identity. The retention object SHALL carry declared boolean,
realm disposition inherit/veto, and effective boolean. Native uses declared
false and effective false; the realm disposition is still truthful.

Per-site/candidate separation, empty holdings, not-held causes, notices,
consulted definition/dialect digests and inactive realm context SHALL remain.
Changing any declared connection, version, tool set, restriction, office
scope, retention/veto, dialect/definition or strictness adapter evidence
SHALL move identity; identical bound bytes produce identical identity.
Runtime SHALL use the sealed selected record, never read fresh grants into
a resumed attempt.

#### Scenario: Native and MCP holdings are distinguishable

- **WHEN** otherwise identical fixtures hold library-docs by native and MCP dialects
- **THEN** v12 records each implementation explicitly and records the MCP server as cap-library-docs with its connection/version/secret names
- **AND** each fallback retains its own record, never the union

#### Scenario: A veto still contributes identity

- **WHEN** retained true becomes false while retain false stays present
- **THEN** both effective results are false but their manifests/digests differ because the declaration changed
- **AND** a store value rotation changes neither digest nor secret-name record

#### Scenario: Resume cannot acquire new execution authority

- **WHEN** a v11 run or differently pinned v12 run is resumed against changed capability semantics
- **THEN** the existing manifest mismatch path refuses the changed identity, naming capabilities, and no broker starts
- **AND** old manifests remain readable and unchanged; no inferred defaults retrofit an old run into MCP eligibility

### Requirement: SC4 seat-record v6 carries a complete attribution group

Implementation SHALL add optional capability, dialect, call_id, call_state
and response_sha256 fields to seat-record v6 checkpoints, with the existing
tool field holding the actual normalized concrete tool. Presence of any
attribution field SHALL require capability, dialect, tool, call_id and
call_state together; response_sha256 SHALL require that group and terminal
call_state succeeded or failed. The engine SHALL additionally verify effective
retention and actual artifact publication before adding a digest.

Capability and dialect use the existing safe name grammar with an explicit
128-byte bound. Call_id uses bounded nonsecret ASCII identifiers (128 bytes);
call_state is exactly observed, succeeded, failed, refused or
interrupted. response_sha256 is exactly 64 lowercase hexadecimal characters.
For attributed records the concrete tool name SHALL be bounded to 256 ASCII
bytes in the existing tool identifier vocabulary; v6 may widen v5's 80-byte
tool ceiling additively, but attribution SHALL never use truncation.
Unrepresentable held identifiers SHALL refuse compile with "capability call
identity cannot be represented by seat-record v6", rather than collide after
clamping. A tools/call name exceeding that runtime byte bound SHALL refuse
"MCP request exceeds the broker limit"; invalid identifier vocabulary SHALL
refuse "MCP tool call request is invalid" before acceptance. These unrepresentable
inputs SHALL never be echoed into journal payloads. A valid bounded name outside
the grant instead follows MB3's recorded ungranted-tool refusal.

V6 SHALL accept every valid old-shaped v5 record. Native observed records
have no response digest. Broker calls have exactly one settled checkpoint;
a local denial has outcome refused without child forwarding. Private Started ledger records are not
public checkpoints.
Existing tool-to-turn requirements remain for old-shaped and native observed
records. Attributed broker settled records MAY omit turn when
no measured correlation supplies it: their mandatory attempt/call identity
provides ownership. V6 SHALL express that conditional additive widening.
The engine SHALL never invent a model turn or usage to satisfy the old
dependency. A verified correlated turn, when available, remains the real one.

#### Scenario: New fields have literal typed constraints

- **WHEN** a checkpoint has only capability, an uppercase/short digest, call_state started or another unknown state, overlong identity or a digest beside refused/interrupted/observed
- **THEN** v6 rejects the exact field/dependency violation at append without writing anything
- **AND** a complete native observed row and broker succeeded/failed row with a valid published digest pass

#### Scenario: Private observations cannot pass as public attribution

- **WHEN** driver data contains a private normalized observation or only part of an attribution group
- **THEN** the engine consumes the observation into a complete engine-owned group before append, or refuses it; v6 never admits private transport fields as journal data
- **AND** direct append of private fields or a partial group fails the closed fence with no event written, while valid legacy checkpoints still append/export/verify during preparation

#### Scenario: Historical tool records need no invented group

- **WHEN** a valid v5 tool checkpoint lacks every new field
- **THEN** v6 accepts it unchanged, and old-version dispatch still uses its recorded engine-line rule
- **AND** readers display attribution as unrecorded, never backfilled

#### Scenario: A long name does not become a different tool

- **WHEN** a dialect's held tool name exceeds the v6 bound or vocabulary
- **THEN** compilation refuses "capability call identity cannot be represented by seat-record v6" with bounded site/dialect context
- **AND** two names sharing an 80-character prefix cannot collapse to the same attributed tool

#### Scenario: Runtime identity validation does not hide a bounded tool denial

- **WHEN** a caller sends a 257-byte tool name or a name outside the tool vocabulary
- **THEN** the broker refuses respectively "MCP request exceeds the broker limit" or "MCP tool call request is invalid" before acceptance or forwarding, with no invented attribution
- **AND** a valid 256-byte name is not truncated; if ungranted, its exact identity appears in the one refused checkpoint under MB3 and CC2

### Requirement: SC5 MCP support never relies on a native-only binding assumption

Before admitting a loadable MCP grant, implementation SHALL replace
capabilities.rs:1626's self.bindings[capability] and doctor.rs:999–1001's
"a grant that loaded is bound to a provider" expectation with exhaustive
kind-specific handling. A valid MCP declaration SHALL not be required to
have a native provider/key pair. Missing necessary state SHALL return a
typed refusal, never panic, unreachable, silent empty provider or native
substitution. Until U9 the ordinary compiler still returns the old refusal.

#### Scenario: Both latent panic paths have regressions

- **WHEN** a test supplies a structurally valid non-native grant directly to the bounded resolver and doctor seams
- **THEN** both return an exact typed MCP-not-enabled outcome or MCP-specific facts appropriate to their stage, without indexing a native binding or expecting one
- **AND** native valid and missing-binding controls retain exact behavior; the public compile path still refuses all MCP grants before U9

## Decisions

New realm reservation changes the old restriction namespace, so the next
realms version after v7 is required. Tool-dialect v1 already supplies the
fields; reject an unnecessary v2 as unused version proliferation (0071
ruling 6). Explicit execution refusals preserve its data-only references. URL execution is refused with a
named cause rather than inventing an authentication or remote transport.

Seat-record v6 and run-manifest v12 are commissioned and clear. The operator
assigns v7 to #487. Slice two takes
the next realms version after v7; recheck main before minting its filename. Never overwrite a frozen file to resolve it.

0071 rulings 2, 3 and 8 govern the new typed variants and errors; ruling 1
keeps storage/clock/environment effects above core/view. Opaque third-party
MCP payloads may be decoded as edge data for validated forwarding, but engine
authority/ledger state does not retain a generic JSON value vocabulary.

One settled broker record, optional measured turn and actual publication
before a digest preserve truthful evidence. Keep old-shaped v5 acceptance,
native observed semantics and bounded refusal identities without truncation
(0071 rulings 3, 7–9). S2 installs these consumers before new driver emission;
private observations never become public schema extensions.
