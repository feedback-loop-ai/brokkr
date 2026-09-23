# Decision 0065, slice one — refuse, never reconcile

Status: proposed design; decision 0066 remains proposed.
Adopted: every commit through 44430402, specification draft a84197cd and
design revision 3c5402be on slice-0065-capabilities; the replay, unit 1b and
unit 2 specification, design, tasks and implementation through f9a691cf, and
proof correction 368bc34e are adopted intact. This rerun addresses unit 2/task
2.1 only; D5.5 records the current council disposition without replacing D5.
Authority: [the complete operator ruling](operator-ruling-2026-09-23.md).
This visit authors documents only. No behavioral finding or security hold is closed.

## Context

See proposal.md for the four governing changes. The earlier whole-rebuild
revision adopted the first and second chief rulings and read all three
completed third-council positions
in `.forge/tasks/council-positions-80bfd784.md`. The third adversarial position
was blocked twice; no position or chief judgment is invented for it. The run
context has no returned_from. The named ruling supplies the return obligations.

Local origin/main is 072cdd9b, seven commits beyond the shared branch ancestry:
#313, #315, #320 (model generations), #321 (v0.11.0), #322, #323 and #326.
Unit 1 has since replayed the branch onto that main; unit 1b bound the guard
order. Their observed results and outstanding external gates remain in
evidence.md. Neither is replayed here. Production remains Rust under crates/.
Hosts remain Linux and macOS (0063).

Before unit 2, agents/load.rs::parse_tools rejected empty allow and knew no
sandbox field, and bundle.rs rejected tools at its executable key lists. The
adopted implementation now decodes both fields, narrows a private office clone
and records site facts. agents::compose still substitutes hands for direct
local tools, and Adapter has no general sandbox-class mapping. The rerun
identifies missing exact proofs and a D5.3 implementation gap: recognized
bypass/configuration controls can coexist with an admitted typed sandbox.
See D5.5 and evidence.md; the existing specification already forbids this.

## Goals / Non-Goals

Use one typed plan and one final command check; refuse authored capability
controls before composition. Keep explicit origin, independent realm authority,
canonical input containment and whole-plan doctor assessment. Preserve existing
requires/wants, identity, hands and boundary contracts and evidence limits.

This visit implements no code, tests, data migration, rebase or release. Slice
two's MCP broker, gate policy and response retention and slice three's comparison
work remain deferred. No grant is added to the repository; no frozen bytes move.

## Decisions

### D1. Reconcile every position against the operator ruling

All reported findings are adopted as defects or proof gaps unless a superseded
remedy is explicitly rejected below. Findings are combined by common cause,
not by averaging severity. Historical HIGH security and specification defects
remain recorded; docs do not establish behavioral closure.

| Third position / finding | Disposition, evidence and owning work |
| --- | --- |
| Security S1; correctness C1; compliance R1 | Adopt the reproduced malformed-list and widening failures. Ruling 1 removes authored lists entirely; reject another parenthesis/union repair. Realm refusal delta; D6. |
| Security S2 | Adopt explicit-empty/Read widening reproduction. Refuse the competing authored include; engine restrictions still require exact final proof. D6. |
| Correctness C2 | Adopt separator `:` producing one denial instead of two. Validate mappings at load and actual serialized meaning at launch; parser round-trip alone without state comparison is insufficient. D6. |
| Security S3; correctness C4; compliance R4/R8 | Combine unchecked managed Codex argv, duplicate engine prefixes and raw-selector disagreement. Parse both declared halves and the complete final command, with shared consumers. D6. |
| Security S4; correctness C5; compliance R7 | Adopt the independently valid but jointly conflicting OFF examples. Ruling 4 requires the complete plan, not fabricated per-capability Controls. D8. |
| Correctness C3; compliance R2 | Adopt the FIFO policy identity reproduction. Refuse nonregular input and bind the parsed regular-file buffer to identity. D7. |
| Correctness C7; compliance R3 (spec_defect=true) | Adopt the upstream artifact fault. Ruling 3 revokes outward-link admission even when bytes are pinned. This design and 0066 correct the owning rule; no downstream pinning exception survives. D7. |
| Correctness C8; compliance R5 | Adopt equal-byte external-library retarget reproduction. Carry owner, original reference, canonical target and digest; recheck all at consumption. D7. |
| Correctness C6; compliance R10 | Adopt missing production-compiled restriction proofs and inflated matrix/removal claims. Hand-built plans and holdings-only tests remain narrower evidence. D10. |
| Compliance R6 | Adopt wildcard/hands overlap failure, reject its old authored-subtraction remedy: all authored deny lists now refuse. Engine-only conflicts still require exact-state validation. D6. |
| Compliance R9 | Adopt unclassified/malformed config passthrough. Bound non-capability config meanings; refuse other assignments without reading them as opaque inert values. D6. |
| Compliance R11 | Adopt joined-token payload disclosure. Build bounded diagnostics from option identity, never raw argv tokens or values. D6. |
| Compliance R12 | Adopt noncanonical temporary-root proof gap. Canonicalize one retained TempDir root on Linux/macOS. D10. |
| Correctness V1; compliance R13; security validation caveat | Preserve distinct failed and blocked coverage attempts. No previous pass substitutes for final-head external exact equality. D10. |
| Adversarial position | Unavailable due to provider classifier; no inferred assent, clearance or finding. |

First-chief H1 (lost mandatory denial), H2 (authored MCP), H3 (lost controls),
H4 (excluded active input), M1 (grant-first doctor), M2 (duplicate sources),
M3 (unseated library lint), M4 (optional-removal gap) all remain accounted for.
The source/lint/optional proofs survive in evidence; H1/H3 gain the total check,
H2 is replaced by unconditional authored-option refusal, H4 by D7, M1 by D8.
Second-chief H1/H2/H3 are subsumed by D6; H4's engine-owned Read/empty positive
survives while authored merges do not. H5/H6 map to D7; M1's requested authored
subtraction merge is expressly rejected by ruling 1; M2 maps to D8; M3 to D10.
V1 stays pending; L1's rejection of embedded workflow directions remains valid.
Panel prose is evidence, never authority to change this commission or its gates.

The following table retains the earlier whole-rebuild council synthesis
accepted before unit 1. Its run-local position paths have since been replaced
by unit 2 positions; it is historical, not a description of those new files.
The adopted first unit 2 synthesis follows under D5.1, and the current rerun
synthesis under D5.5. Earlier pass verdicts did not prove the rebuild:

| Position claims | Decision and evidence |
| --- | --- |
| Both: unconditional authored refusal and closed effect grammar | Adopt. `grammar.rs` currently calls permission-mode/sandbox/add-dir inert, and `native_controls.rs` still folds authored lists. D6 removes that route; the realm delta also names add-dir and Codex include-plan-tool so current grammar entries cannot escape the inventory. Unknown syntax refuses without inventing provider aliases. |
| Robustness §§1–2: structural effects and origins through runtime; simplicity §§1–2: reuse existing grammar and private composition | Combine. `Candidate::parts` counts only trailing hands and `engine.rs::SiteSpawn::launch_arguments` reconstructs just two arrays. D5 needs a dedicated runtime wiring unit before migration. Reject a public provenance schema, byte-matching exemption or second command interpreter. |
| Robustness §3: consume a checked command at spawn; simplicity §3: keep existing builders | Combine in D6: one private checked value returned by the existing pure final builder, with no later argv edits. Codex's verbatim managed arm and Claude's later prefix demonstrate why intermediate checking fails. No generic execution framework or new production module is commissioned. |
| Both: bound config/list/restriction meaning; robustness: expected state independent of generated argv | Adopt. `config_key` deletes quotes/whitespace and cannot establish assignment semantics; C2's separator changes denial meaning even when it parses. A closed set of needed inert assignments and managed patterns suffices. Reject a general TOML evaluator and arbitrary pattern algebra; uncertain forms refuse. |
| Both: regular, owner-bound inputs and same-buffer policy identity; robustness §4: bind the actual read to containment | Adopt D7's handle-bound read or conservative refusal. `active_input` returns a joined path, `own_table` reads separately from the file walk, and `CharterPin` lacks owner/target. A hash or canonicalize-then-reopen cannot close the FIFO and equal-byte-retarget findings. Reject external-input caches, owner guessing and race-safety claims from path checks alone. |
| Both: the complete plan reaches doctor in both paths | Adopt D8. `denial_on` marks the whole floor denied while carrying one OFF; doctor calls it at both restriction/drop and native reporting sites. Reject another per-capability solver. Add a complete admitted-plan scenario as well as conflict scenarios. |
| Both: exact typed migration and inventory; robustness §6: omission/empty/subtraction | Adopt D5 and Migration Plan. Source lists confirm `.venv/bin/pytest` and two narrow gh prefixes. Preserve acceptEdits and each Codex sandbox class. Reject broader replacement mappings and new agent files merely to avoid inline typed support. |
| Both: restriction qualification stays open; bounded visits and independent proofs | Adopt Open Questions and the single Rebuild units list. Add runtime origin wiring, split final assessment/cold/resume and charter dispatch/start-resume work, and separate matrix proof visits. Keep each production-file budget at three or fewer; the former three-filename launch unit was still too broad. |
| Simplicity: cut new dependencies/contracts/frameworks and later-slice work; robustness: do not cut boundary proofs | Combine. Existing Rust types, loaders, builders, pins and owning suites suffice. Accept refusal of unmodeled provider syntax and unknown live behavior; reject shortcuts that remove realm authority, final equality, containment or real compiled-boundary proof. |

These amendments close design omissions in the adopted draft, not implementation
findings. R3/C7 was an upstream specification fault; its owning delta and proposed
0066 already revoke the exception. If later evidence defeats an owning requirement,
return upstream with that evidence instead of relaxing it in code.

Task-seat coverage audit: the executable checklist now follows the exact
Rebuild units order, retaining earlier IDs beside each row. Every requirement
is named by its tasks. The previously uncited native requirement “Neither inline
arguments nor fallback can override native denial” is explicitly served by
tasks 12.1 and 15.1. This is a coverage correction, not new behavior or evidence.

### D2. Thread an explicit operator capability context through compilation

Add `CapabilityContext` alongside the existing compile context: operated realm
identity (including `<unmapped>`), selected grants, and explicit definition
and tool-dialect roots. With an active map, `capabilities/` and
`dialects/tools/` are relative to that map's configuration directory. Without
a map, both are under the resolved operated repository. A grant names a
contained dialect by library name; neither recipe ancestry nor an agent-library
override supplies a fallback root. Roots locate operator data; they grant
nothing.

CLI compile/run/rerun and semantic library lint construct this context
explicitly. Direct runtime compile entry points receive it too; convenience
entry points construct explicit empty grants and a documented operated-root
context. None means "skip native denial". Resume uses D7's pinned operation,
not discovery of today's grants.

`Realm` gains v6 grants with optional `tools`/`offices` lists and an object of
dialect-owned restrictions. Check presence before defaulting: v1–v5 reject
`capabilities` even as `{}`; v6 omission and `{}` grant nothing; `null` is
invalid. Omitted scope means all requesting offices, `[]` none. Omitted tools
means the dialect set, `[]` no usable holding. Lists are duplicate-free.

Requests remain name-to-`requires | wants`. Preserve the optional agent-backed
site override: omission inherits, an explicit map is an unchanged-strength
subset, and `{}` subtracts all. An inline map is its office ask. The office
is the referenced agent name or inline site's stable authoring identity.
Store it beside the execution label before wrapper relocation: moving
`verify` to `verify:checks` moves facts, not the office's identity. Generated
helper sites get their own empty outcomes. Existing collision checks remain.

Alternative rejected: derive authority from `--agents-dir`, recipe ancestry
or a late launch-time map lookup. Each could replace the realm authority
under which compilation actually occurred.

### D3. Load abstraction, dialect and grant data separately and strictly

Implementation adds `tool-dialect.v1.schema.json`, `realms.v6.schema.json`
and `run-manifest.v11.schema.json`, registered additively in the frozen suite.
v11 follows the local manifest lineage without changing the event envelope,
`event_schema` or driver protocol.

Abstract definitions use CQ2's strict two-field shape, for example
`{"name":"web-search","classes":["reads","egress"]}`. Reuse the library's
safe filename grammar, without a closed catalogue of capability names.
Validate every definition in the explicit root, filename/name agreement,
uniqueness and nonempty unique closed class sets. Resolve every loaded request
(including later subtraction) and selected-realm grant (including unused).
Missing roots are empty; denying known native tools needs no invented abstract
metadata. Pin consulted definitions. A selected dialect's optional `classes`
asserts set equality, never supplies or overrides the definition.

Tool-dialect v1 is a closed discriminated object:

| Fields | Meaning |
| --- | --- |
| `schema`, `name`, `serves`, `kind` | `brokkr.tool-dialect/v1`, filename identity, abstract capability, exactly one of `provider-native`, `mcp`, `hands`. |
| `tools` | Nonempty unique concrete tool names realizing the binding. Reserved hands names only `workspace`. |
| optional `classes`, optional `egress`, required `sends` | Class equality assertion; existing egress enum with absent = uncontracted; `sends.description` and boolean `sends.seat_composed`. No route/trust promotion. |
| `restrictions` | Embedded object schema for grant fields after removing `dialect`, `tools`, `offices`; it cannot redefine those reserved keys. |
| native: `provider`, `adapter_key` | One native adapter declaration; no connection/executable/credential fields. |
| MCP: `connection`, `version`, `secrets`, optional `retained` | Exactly one stdio `argv` or URL connection, pinned implementation version, named 0012 bindings (empty if none), boolean retention declaration. No native fields. |
| reserved hands | `serves: workspace`; no new implementation or hands policy override. |

MCP declarations remain data only. Secret declarations use 0012 binding names
and declared-reference checks; no literal credential field, credential-bearing
URL userinfo or resolved credential belongs in the connection. Stdio references
retain environment-injection meaning, never value substitution into argv.
Diagnostics identify fields without echoing credentials. Loading opens no
secret store. This structural contract does not claim to detect arbitrary
secrets disguised as ordinary text.

Restriction schemas use Draft 7, matching repository contracts, and only
references within the embedded schema document. Disable retrieval and refuse
external/file/network references and unsupported drafts before compilation.
Reserved-key checks follow root references/composition so `$ref` cannot
redefine the grant envelope. Promote the existing `jsonschema = 0.53` runtime
dev-dependency to production with `default-features = false`. The reason is
production validation of operator schemas: an ad hoc evaluator risks ignoring
constraints. No new package/version or registry upgrade is needed.

Read each definition/dialect once and parse/hash the same buffer with existing
SHA-256 helpers. Validate lexical and canonical containment, including symlink
escapes. Reject duplicate keys in new authority objects before map conversion
erases them; scope this to definitions, dialects and capability request/grant/
control data, not unrelated legacy realm semantics. Traverse deterministically;
preserve restriction object values and array order. Reuse safe diagnostics.

**First-council M2 (retained):** `compose::read_layers` strictly parses each original `bundle.json`
buffer using `brokkr_core::canonical::parse_strict` before converting it to a
map. `agents/load::read_json` does the same for agent source bytes. This
includes nested panels, steps, selected cases and ancestor layers later
overridden or removed. Both repeated capability names and repeated outer
`capabilities` fields refuse, even equal repetitions; neither strength order
nor a replacement `{}` gets last-wins semantics. Accept rejection of other
duplicate fields in these same source documents: selectively preserving them
would complicate the authority parser. Do not rewrite unrelated event/policy
parsers. Diagnostics retain source, repeated key and parser location; use raw
JSON fixture text, not `json!`, to prove the duplicate survived to the reader.
Keep adapter duplicate rejection and its removal regression intact.

**First-council M3 (retained):** Immediately after a library is loaded in `Bundle::assemble`, call
`authority.definitions.lint(&library)` before resolving/subtracting seats.
Return deterministic agent-named diagnostics as `CompileError::Capability`,
using the same helper as CLI readouts. A valid seated worker cannot hide an
unseated undefined request. Do not load a library solely to lint one that the
recipe never uses. Include every loaded agent's requested definition name in
`Authority::manifest`'s consulted set, even unseated or later subtracted;
otherwise lint introduces an unpinned semantic dependency. Definitions not
consulted by requests or grants remain outside identity.

Alternatives rejected: a fourth public abstraction schema/registry, remote
schema lookup or Rust interpretation of hosts/repositories. CQ2 already fixes
the small definition shape; dialects own restriction meaning, and 0036 retains
route and secret-binding clearance.

### D4. Resolve grants before compatibility; deny native power independently

Use one pure derivation with typed causes and one complete diagnostic renderer.
Keep loading separate; no subprocess, probe or ambient configuration read
occurs in authorization. Supplied availability facts still select candidates.

1. Validate request syntax, all loaded abstract metadata and subtraction.
   Validate every selected-realm grant's contained dialect, `serves`, classes,
   tools, scope and restriction schema before seat compatibility. Structural
   failures cannot become optional notices.
2. Refuse every selected MCP grant with the slice-two reason, even when unused
   or `offices: []`. Refuse reserved hands as governed by 0043/0046. Schema
   validity never promises an implemented launcher.
3. Per site/candidate, evaluate remaining asks: grant presence, office scope,
   nonempty tools, provider/native-key compatibility, exact tool narrowing,
   usable control assessment, ON disposition and restriction transport. A requirement refuses; a want
   loses the whole holding with its exact notice. An unused entry remains
   pinned and inactive, requiring no ON/restriction plan.
4. Independently inspect the candidate's known native inventory. Every native
   capability/tool not effectively held receives OFF. Measured unsupported
   OFF refuses even after a wanted drop, unused grant, scope exclusion or
   subtraction. Unmeasured OFF of a known unheld power also refuses, retaining
   its reason rather than claiming measured impossibility. Unknown inventory
   cannot satisfy a holding and cannot erase an independently known power.
5. Check the combined control plan expresses exactly the admitted set without
   enabling an excluded tool; then seal the candidate outcome.

**First-council H1 admission (retained):** At the existing built-in provider recognition seam, retain a
small known-power floor: Codex has web-search; Claude has web-search/web-fetch.
Additional adapter-declared native powers add obligations. This floor neither
grants capabilities nor supplies classes, concrete switches or live evidence.
An empty `known` map, omitted key or absent operator definition cannot remove
these obligations. DSH, LaneTally and opaque custom drivers keep their own
unmeasured assessment, not an inherited Claude/Codex inventory.

Load capability-relevant adapters once through a `Result`; pass the same
validated data to model-pin and capability resolution. Separate the optional
effort-exemption semantics from mandatory denial. Preserve missing roots,
missing providers, malformed/unreadable files and legacy assessments as named
causes. It is acceptable for an unrelated malformed adapter to block this
compile; it is not acceptable to erase the error and launch Codex without OFF.
Refuse rather than invent a fallback discovery path or embedded OFF catalogue.
Attribute the error to the first affected site/office, realm, provider, known
capability and original failing source/cause. Audit all error suppression that
can change authorization, including the retry read in `codex_managed`; an
unrelated display-only `.ok()` is outside this repair.

Use the complete CQ1/CQ2 strings from the owning scenarios. Other causes use
the same voice and include seat, office, realm, capability and relevant
provider/dialect/cause. Notice metadata retains candidate identity. Stable
traversal makes diagnostics deterministic. Existing local tool restriction
failures remain unconditional refusals.

The missing-grant requirement is:

> seat 'research' (office 'researcher') in realm 'private': requires capability 'web-search' but the realm does not grant it to this office

The MCP refusal is:

> realm 'private' grants capability 'library-docs' through dialect 'docs-mcp' of kind 'mcp', whose broker support is not implemented until decision 0065 slice two

Alternatives rejected: refuse every schema-valid optional restriction, discard
only the restriction, or test only whether the map contains a capability name.
The first contradicts CQ1/ruling 5; the others widen power. Preserve
`resolve_report`'s whole-chain required incompatibility refusal; do not prune a
required candidate as a new fallback policy. Every executable optional
fallback keeps its own drops/controls, never the union of its peers' holdings.

### D5. Carry one plan and explicit origins

Keep sealed site/candidate holdings, not-held reasons, typed tool restrictions,
known native ON/OFF dispositions, evidence and identity. Primary, fallback,
panel, sequence, selected and inherited launches carry their selected outcome.
Do not union candidates or recompute grants in a driver. Private plan decoding
remains fallible; malformed or missing mandatory authority never means empty.

Separate authored inert argv, adapter templates, typed local permissions,
engine hands and realm-derived native controls before flattening. Existing
hands-length provenance alone does not identify generated local permissions
or an adapter's permission-mode template. Extend private origin carriage and
verify that its parts reassemble actual argv; never recognize engine ownership
by server name or equal bytes. Every engine origin still undergoes grammar and
final-state checks. Preserve frozen public wire/manifest contracts.

Carry that private record from agents::compose/Candidate through bundle site
facts to engine::SiteSpawn, selected fallback projection, boundary fragments,
placeholder expansion and final input assembly. Extend SiteSpawn's current
trailing managed count; updating only agents and protocol is insufficient.
The driver validates origin segments against the actual argv it receives.
Missing/malformed/reordered segments or an authored override refuse. Engine
origin identifies the supplying typed input; it is not an exemption from its
constraints. The sealed expected capability state is independent of generated
argv, never reconstructed from the output being checked.

Typed inline tools use the agent vocabulary `tools.allow` (abstract local
command names, no native capability aliases) and abstract `capabilities` asks.
Agent-backed seats only narrow their office. Preserve explicit local sandbox
restrictions as typed `tools.sandbox` with the existing read-only,
workspace-write and danger-full-access classes; this is a requested local
execution restriction, checked against 0046's realm/boundary authority, never
a capability grant. Unknown classes and unrepresentable restrictions refuse.
Adapter permission modes remain engine-owned mappings; raw option names or
opaque settings are not accepted as typed permissions. This migration support
must exist and be tested before editing shipped declarations or enabling refusal.
For inline local tools, omission preserves the existing local default and an
explicit empty allow list permits no local entries. Agent-backed omission
inherits; an explicit unchanged-meaning subset narrows, including empty.
None changes native OFF or grants a capability. Preserve command patterns
exactly: pytest is `.venv/bin/pytest`, not another executable of that name.

### D5.1. Unit 2 council disposition and bounded implementation

Run `build-decision-0065-slice-one-re-9f0b932b`, adopted head `d9816374`.
The positions then present were read in full from
`.forge/design/positions/robustness.md` and
`.forge/design/positions/simplicity.md`; those paths now hold the rerun
positions reconciled in D5.5. This table is the adopted historical synthesis.
There was no `returned_from` finding. The decisions below refine D5; the
accepted Rebuild units order stays intact.
Decision 0066 remains proposed. Task 2.1 remains open until implementation
and its own observed proofs exist.

| Current position claims | Disposition and evidence |
| --- | --- |
| Both: one decoder, independent presence, ordered narrowing | Adopt. `parse_tools` is the existing decoder; `Some([])` must survive and object replacement would erase an inherited sibling. Use it for agents and executable sites, with no normalization or second JSON parser. |
| Simplicity: consolidate Agent.allow; robustness: retaining it is acceptable | Retain `Agent.allow` and add the sandbox enum field. A small local value returned by the shared decoder/narrower is assembled from those fields, not stored beside a duplicate allow field. This avoids an unnecessary public storage rename; there remains one authoritative value per field. |
| Both: validate the complete chain and keep authority separate | Adopt. `resolve_report` already refuses a gap even in an unavailable fallback. Apply the site override to the cloned office before composition; retain existing model-policy/hands precedence over candidate compatibility gaps. |
| Both: strict validation beside hands; robustness: no tool_permissions requirement for existing hands | Combine with a precise boundary: syntax and subset checks always run; direct mapping support is required where the direct list applies. Reject a mapped native alias even beside hands. Reject the interpretation that every hands agent needs a direct-tool mapping: `hands_replace_the_tool_list_with_the_adapters_fragment` explicitly proves the opposite. A dormant list supplies neither direct tools nor a broker policy. |
| Both: existing site facts and executable paths; robustness: distinguish unchecked from unrestricted | Adopt a defaulted optional local fact in `SiteFacts`: absent means not visited, present with two absent fields means checked and unspecified. Containers never own it. Whole-value relocation already exists; avoid a new visitor or parallel map. |
| Robustness: specify sandbox support, actual driver identity and work/boxed cases | Adopt the bounded table in D5.3. Adapter prose, provider labels and a vaguely similar permission mode are not support evidence. The current Codex hands fragments and existing grammar are sufficient for the few matching cases; all other explicit sandbox combinations refuse for now. |
| Robustness: intermediate runnable acceptance must fail closed; simplicity: emission stays in unit 3 | Combine in D5.3. Admit only restrictions already represented by the existing path. Otherwise retain pure decoding/narrowing positives and refuse runnable compilation until the owning lowering/transport work exists. Reject acceptance that merely stores a restriction next to an unchanged command. |
| Both: exact proofs, canonical fixtures, no scope expansion | Adopt D5.4. Reject new schemas, modules, dependency changes, Candidate fields, shipped data edits or provider probes in this unit. No broad cleanup or later-unit completion follows from this design. |

### D5.2. Shared decoding, narrowing and site ownership

Define the three-case sandbox enum and a small two-field local value in
`agents.rs`: optional ordered allow and optional sandbox. Expose
`agents/load.rs::parse_tools` narrowly through agents for bundle use; decode
only allow, sandbox and absent/exactly-empty legacy MCP. MCP has no retained
field. Use presence before type checks, preserve exact names/order and reject
semantic duplicates. `read_request_source`, not the ordinary `read_json`
helper, is the current strict agent reader; recipe layers already use
`compose::read_layers`. Keep both strict readers and test duplicate keys from
raw bytes without changing bundle/compose.rs.

Narrow each field independently. Omission, `{}` and an omitted sibling inherit;
explicit empty stays empty. Membership checks never reorder or silently
intersect a list. An office empty list permits only empty; an unspecified
list can be restricted. Compare sandbox reach explicitly as read-only <=
workspace-write <= danger-full-access, never lexically and never as boundary
ordering. Equal or narrower requests retain their exact requested class;
widening refuses rather than clamping. Keep office source/digest unchanged;
apply the override to a private clone, so two sites cannot affect one another.

Factor `report_under` only enough to compose this effective clone. Do not first
resolve a broader office command and then attach narrower facts. Preserve
whole-chain compatibility, including unavailable fallbacks, and existing
hands/model-policy error priority. Optional capability handling cannot forgive
malformed local input, widening or an inexpressible direct restriction.
Where direct tools apply, resolve each name using that candidate's
`ToolPermissions.names` and reuse the `native.capability_of` check, including
arbitrary aliases of native tools. Unknown mappings refuse; no raw name is
passed through. With agent hands, the standing replacement rule still applies:
validate declaration shape and narrowing, reject native aliases when mapped,
and retain the declaration without describing it as broker-command filtering.
Do not require otherwise-unused direct-tool support or append local flags
beside the workspace fragment. Empty local entries do not disable hands.

In `bundle.rs`, admit the tools key in SEAT_KEYS/BODY_KEYS/MEMBER_KEYS/STEP_KEYS
and explicitly reject its presence on containers, even `{}`. Use the existing
ordinary/member/step/selected-body paths, including every case/default and
inherited body. A panel-valued step remains a container. A dialect step with
no executable check cannot own tools; an exec-generated check cannot represent
nonempty local fields and refuses them. No synthetic path may discard them.
Typed declarations needing adapter validation must obtain the existing
fallible adapter context; extend needs_adapters without opening an unused
agent library or swallowing load failures. Validate shape/placement before
asking an adapter to represent a declaration.

Store the effective value beside the execution site's other facts, using
existing labels/source roots. It is private compile data, not a manifest field
or a new capability grant. Unrestricted executable sites still receive a
checked empty value. Wrapper relocation moves the entire fact. Error messages
retain source/site, field and full cause; dependent errors add office,
provider/model and resolved boundary as applicable. No value-dependent raw
flag reconciliation or change to later refusal activation belongs here.

### D5.3. Existing representations and the fail-closed handoff

Decoding is not runnable admission. An explicit restriction must already be
represented by the selected serving path, or bundle compilation refuses with
the owning field and unsupported-representation cause. This implements SCM's
existing unrepresentable-restriction rule; it is not an exception to it.
The guard concerns typed declarations only, not unit 12's general authored-flag
campaign. Authored flags can never supply the missing representation.

| Effective declaration/path | Unit 2 admission boundary |
| --- | --- |
| Both fields unspecified | Preserve the existing local default and all independent authority checks. |
| Agent direct nonempty allow, no sandbox | Existing compose mapping applies to the effective clone, across every candidate. Preserve literal mapping values and existing native-alias refusal. |
| Inline direct allow, or direct explicit empty | Decode/narrow exactly, then refuse until exact lowering exists. Joining empty names into an empty flag value is not an empty-tool proof. |
| Agent hands and local allow, no sandbox | Preserve existing replacement semantics and strict local validation from D5.2; never add direct flags or reinterpret the list as authority inside the broker. |
| Agent with hands, actual Codex dispatch, harness gate | Only read-only matching the existing valid gate fragment can be admitted; missing fragment and open-gate authority refusals retain precedence. Workspace-write and danger-full-access conflict with the gate. |
| Agent with hands, actual Codex dispatch, harness work | Only workspace-write matching the existing work fragment can be admitted. Read-only would require replacing the writable fragment; danger-full-access would contradict it. Both refuse in this unit. |
| Agent with hands, actual Codex dispatch, boxed boundary | Only read-only matching the existing workspace fragment can be admitted, with the box and hands policy unchanged. Other explicit classes refuse; harness.work cannot substitute for missing workspace support. |
| Open work with hands; any no-hands sandbox; inline sandbox | No existing typed emission path supplies the requested class. Refuse, including danger-full-access: a presumed provider default is not a representation. |
| Claude/LaneTally/DSH, exec or opaque dispatch with explicit sandbox | No established mapping to these three classes in current data. Refuse; acceptEdits is not evidence of a Codex sandbox class. |

Recognize the actual driver, not the adapter's provider label. For the admitted
Codex cases, inspect the selected engine fragment through the existing public
protocol grammar: exactly one sandbox option with exactly the requested class.
The shipped workspace/gate fragments use `--sandbox read-only`; work uses
`--sandbox workspace-write`. Inspect the other command contributions for a
competing sandbox control or opaque configuration that could defeat this
check; uncertainty refuses typed admission. Apply that judgment to the selected
fragment itself as well as the authored contribution: a matching `--sandbox`
is insufficient beside `--full-auto`,
`--dangerously-bypass-approvals-and-sandbox`, or configuration under
`sandbox_mode` or `sandbox_workspace_write`. Use the existing parsed option
identities and configuration paths; refuse these competing or unestablished
effects without reconciling them or trusting argument order. Engine provenance
is not an exemption. Preserve established hands transport and effort settings;
rejecting every `-c` assignment would reject the matching supported fragments.
This completes the existing no-competing-control rule, not a new sandbox
class or unit 12 catalogue activation. Do not add another parser, infer
support from diagnostic prose, reconcile duplicates, or treat authored bytes
as engine provenance. The current harness path appends the selected fragment;
no new engine wiring is commissioned. Boxed host availability remains the
existing start-time obligation, not evidence obtained by decoding.

These admitted cases assert existing fragment selection and exact compile
facts only. They do not close cold/resume/final-command proof. Units 3–4 own
new lowering and transport: a guard may be removed only for a path whose
restriction reaches its command with preserved origin. If that needs both
units, keep that path refused until both land. All three enum values have
positive decoding/narrowing tests even when a particular runnable path refuses.
Compiled positive scenarios are conditional on representation, as SCM already
requires. There is no unresolved unit 2 design question or request to accept and discard
typed data.

### D5.4. Proof, migration and scope controls for unit 2

Keep production in agents.rs, agents/load.rs and bundle.rs, and tests in
agents/tests.rs and bundle/agent_tests.rs. Keep Candidate and public contracts
unchanged. Use the existing canonical Tree and a retained, once-canonicalized
bundle fixture root; new inline roles contain bytes inside their bundle.
No test reads .forge/, discovers a provider or starts a model.

Record baseline behavior before repair, then an independent compiling mutation
for every new test, its failing exact assertion and restoration/pass. Required
cases cover each malformed field/duplicate, empty versus omission, each class,
partial inheritance, ordered subsets, independent allow/sandbox widenings,
unknown/native mappings, fallback-only failures, unchanged hands semantics,
each table admission/refusal, every executable/container form and two sites
sharing an office in opposite traversal orders. Assert full effective facts,
selected fragments, source/field causes and unchanged holdings/native OFF.
A table's first failed row does not bind later rows. Existing strict readers
may already pass baseline; report that honestly and mutate their in-scope
caller/reader independently, never bundle/compose.rs to evade the budget.

The implementation visit must run the runtime suite, workspace tests, self
bundle compile, format, locked all-target/all-feature clippy, strict all-item
OpenSpec and diff checks. External exact coverage stays pending until actual
host/CI evidence exists. A fourth production file, another suite or pin edit
requires an inventoried split before editing. Later audits inspect these
records; they do not replace the owning unit's proof. Rollback this unit's
new admission if necessary; never fall back to unrestricted compilation.
Shipped migrations, broader sandbox support and new origins remain in their
assigned later units.

### D5.5. Unit 2 rerun council disposition, 2026-09-23

Run `build-decision-0065-slice-one-re-e332dc8d`, reviewed head `368bc34e`.
Read both current positions in full at `.forge/design/positions/robustness.md`
and `.forge/design/positions/simplicity.md`, and robustness's recorded
compile probe at `.forge/design/unit2-robustness-probe.json`. There is no
`returned_from`. All four commissioned commits remain ancestors; the review
startup failure is neither a code finding nor a review pass. This is an
amendment for demonstrated omissions, not a replacement specification/design.

| Current position claims | Disposition, evidence and owner |
| --- | --- |
| Both: retain shared strict decoding, exact presence/order, independent narrowing, private clone and per-site facts | Adopt. The implementation exists in the three named files; the specification and D5.2 already require these invariants. Reject storage consolidation, a new parser/module/registry, Candidate changes and public contracts: none addresses the observed gaps. |
| Both: keep whole-chain checks, hands replacement, native-alias refusal and independent realm/boundary authority | Adopt unchanged. Preserve unavailable-fallback validation, checked-unspecified versus unvisited facts, container refusal and constitutional error precedence. An empty dormant list does not disable hands or require otherwise-unused mappings. |
| Simplicity: zero production diff unless a behavior defect is demonstrated; robustness: repair the sandbox guard | Combine the conditional advice with the demonstrated defect. The probe admits typed ReadOnly beside bypass in authored argv and in hands.workspace, and beside authored full-auto or sandbox_workspace_write.network_access. Source inspection confirms expressed_sandbox ignores those recognized nodes. Reject an unconditional zero-diff conclusion or deferral to unit 12. Task 2.1.5 owns the narrow bundle.rs repair and bundle/agent_tests.rs regression. |
| Robustness: judge both engine and authored contributions, retaining supported config | Adopt the explicit D5.3 instruction. Check existing parsed nodes; refuse uncertain sandbox effects, never reconcile them or assume last-option priority. Reject trusting engine origin, a provider-version probe to decide priority, and blanket config rejection: existing hands and effort settings must keep their established meaning. |
| Both: full narrowing causes and independent row proofs are missing | Adopt the 368bc34e correction. Task 2.1.2 replaces field-only assertions with complete errors; 2.1.6 audits each claimed decoder, inline, executable/container and sandbox row. A first-row mutation failure proves no later assertion. Preserve sufficient existing experiments and supply only missing ones. |
| Both: strict bundle source proof needs a permitted seam; missing original reds must remain missing | Adopt. Investigate the bundle.rs caller or existing test seam without adding a decoder or editing compose.rs. If no valid compiling removal fits, stop with the exact split. Any 308a8a28 baseline experiment is retrospective. Retain the measured 17-test count and original history. |
| Both: retain D5.3's interim refusals and later-unit ownership | Adopt. Existing agent-direct nonempty mappings and matching Codex hands cases remain the only applicable paths; inline direct, direct empty and unsupported sandbox paths still refuse. Units 3–4 own lowering/transport, and later units own migration and final serving proofs. No grant, frozen surface, provider launch or release work enters this unit. |
| Both: final restored gates and external evidence remain obligations | Adopt. Fresh results belong to their tested revision; prior cargo-unavailable and prior gate-pass reports stay historical. Reopen 2.1.7 for the pending repair. Exact coverage, macOS and remote CI remain pending until observed; this design closes no implementation, proof or security hold. |

The regression belongs in the existing sandbox admission suite, using its
canonical temporary root and complete literal refusals. Observe the intended
baseline assertion before repairing the guard, then independently remove each
claimed enforcement, record the exact failure, restore and pass. Cover the
selected fragment and authored argv independently for each competing control,
with the matching boxed/gate/work positives, supported configuration and
standing error precedence retained. A compile-admission proof establishes no
provider escape or final-command behavior. Scope remains three production
files and two suites; an actual fourth-file requirement must be split first.

SCM's existing scenario “An existing sandbox fragment must match the typed
request” already requires no competing control. No ambiguity or earlier
specification fault was found, so no new scenario, semantic decision or
upstream return is justified. Decision 0066 remains proposed. The risk is an
incomplete sandbox-effect check; D5.3's explicit cases and independent proofs
address it without extending provider support. No data migration is needed;
retain refusal on unsupported paths and D5.4's rollback rule. There is no new
unit 2 open question; external evidence and later-unit questions retain their
owners. The accepted Rebuild units order is unchanged.

### D6. Refuse authored controls; validate declared and final meaning

Ruling 1 says “Nothing is merged.” The exhaustive supported refusal catalogue,
aliases and split/equal/attached forms are in the realm delta. A parsed authored
capability-bearing option refuses independent of its value and the realm grant.
Do not parse authored tool patterns into contributions, preserve authored deny
lists, or special-case compatible/empty/local-only lists. The known grammar
remains closed: unknown flags, unbounded config and opaque settings refuse.
DSH's single bound route-only overlay remains a non-capability input checked
by route_overlay before staging; web/plugin/profile and capability patches
are not that exception. LaneTally's wrapper and Claude child both participate.

The protocol grammar owns canonical options, supported forms, arity,
repeatability, subcommand and session/stdin positions and typed effects. One
parse serves extraction, selectors, resume eligibility and composition. Inert
values such as `--image resume` cannot become selectors in a later scan.
Diagnostics render a fixed cause, source/site/provider and normalized option
with position, bounded to 512 Unicode scalar values for the option/cause
portion. Never echo raw values, attached suffixes, joined assignments, secret
paths or arbitrary config key payloads. Unknown option labels are sanitized
and bounded; malformed positional payloads use a positional label.

At adapter load, parse every declared ON and OFF argv, even unused halves;
validate mappings, separators and supported restriction transports. Invalid
Codex managed arguments have no verbatim bypass. Explicit measured default ON
is a typed disposition with evidence; it is not an unexplained empty vector.
Concrete switches remain adapter data; declaration cannot extend the grammar.

The engine alone lowers typed local tools, realm-derived native controls and
hands. Distinguish no include restriction, an explicit empty restriction and
a nonempty hard limit. Never widen a hard limit by union; if the complete plan
cannot be represented, refuse. Managed patterns/separators must have bounded,
provider-equivalent meaning, or refuse. A generic --settings transport is not
proof that any JSON object enforces a restriction; require a supported bounded
transport preserving the plan, including permitted tools and denied powers.

Compile preflights the whole command shape. Immediately before each known-harness
serving process launch, parse the actual complete serialized command after engine prefixes,
wrapper settings, expansion, boundary/model/effort, session and stdin arguments.
Compare the effective capability state with the sealed plan: exact held/denied
powers, tool subset, restrictions and hands. Unknown, missing, extra or
contradictory effects refuse before spawn. This is ruling 2's “final command,
not an intermediate composer.” A round-trip preserving the same wrong string
is insufficient. Share this final composer/check with doctor. Return a private checked-command
value from the existing final builder; spawn consumes it without further argv
mutation. Any later edit requires a new check. Wrapper-child settings and DSH's
staged configuration retain their bound typed/pinned meaning in that assessment.
Availability/version probes and opaque custom drivers have different scope;
this check creates no capability guarantee for them. Do not add a new launch
framework, public schema or production module just to carry this private value.

Cold, actual eligible resume and rejected-rejoin cold replacement each use
that check. Preserve existing session/version/sandbox/effort/accounting
eligibility and refusal reasons. A cold replacement is not resume evidence.
Codex's measured OFF remains `-c web_search="disabled"`; only cold 0.154.0 is
live-measured. DSH/LaneTally retain independent unmeasured inventories.

### D7. Canonical containment and consumed identity

Ruling 3 says “It is not pinned and admitted.” The prior D7 argument that a
link under its own name may escape if content-pinned was an upstream
specification defect (R3/C7). Revoke it completely. Resolve the actual target
against the canonical declaring layer, then require containment, regular-file
kind, readability and a valid pin. Check both authored and resolved excluded
trees. A lexical walk, parent-step ban or content hash alone proves neither
containment nor identity of all consumed inputs. Contained links are permitted
only when every input/pin/consumption check holds; direct outward links and
symlink-plus-parent escapes refuse even with equal bytes.

Bind the read to that contained target using an owner-rooted, handle-based
resolution/read at the existing input boundary, or refuse when the supported
host cannot establish the binding. Check regular-file kind before reading;
read/hash/parse the verified buffer. An unchecked path reopen after canonical
comparison is rejected: a concurrent replacement could supply an outward or
nonregular file despite the earlier predicate. Keep contained symlink support
only where the bound read is provable. A controlled replacement test must
observe either the already bound contained file or refusal, never replacement
bytes. This obligation covers compile and subsequent charter consumption on
Linux/macOS; it is not a new watcher or generic filesystem service.

Policy composition reads and hashes one regular-file buffer, binds it to its
owner's file-map entry, and parses that buffer, including overridden ancestors.
If the map is assembled later, carry the consumed digest into the walk and
compare before sealing; never assume the walk necessarily pinned the input.
FIFOs/devices/directories refuse before a potentially blocking stream read.

At compile bind each charter to Layer or Library, original reference,
canonical target and existing expected digest. Preserve the selected library's
independent contained root, including external/nested libraries, without a
longest-prefix guess or fallback to a neighboring pin. Start, pinned-context
resume and common dispatch recheck owner, target and bytes. Equal-byte retargets
still refuse. Pass the verified text buffer after authored/context merging;
managed render_prompt never reopens a path or replaces failure with empty text.
Dialect instructions retain their own contained/pinned owner route. No new
identity inventory or manifest version is needed; unconsulted definitions stay
outside identity. All permitted consumed bytes remain pinned.

### D8. Doctor submits the whole plan

Ruling 4 requires the launch composer, not a per-capability assessment. Submit
all native ON/OFF dispositions, typed tools, hands, restrictions, candidate,
authored inert argv and command shape together. Report admission or the same
bounded refusal cause, with independent full-line expected test literals.
Do this in both native reporting and restriction/drop reporting. Absent,
scoped, empty, unused and subtracted grants cannot conceal conflicting OFFs.

Without a resolved seat, assess only a complete explicit adapter-level plan
and label that scope; never say every seat will launch. Unknown inventory,
measured unsupported OFF, invalid declaration, composition refusal and admitted
static control remain distinct. No model/server execution is required and
composition does not prove live enforcement.

### D9. Preserve foundational semantics

Keep additive contracts, realm v1–v5 empty grants, strict request source reads,
loaded-library lint, consulted identity, optional notice proofs and data-only
charters. Do not reinterpret historical channel versions or journal examples.
No new dependency is needed. Decision 0066 is proposed; this seat amends its
mechanism but cannot accept it. A requirement contradicted by new evidence
returns upstream; it is never hidden in an implementation exception.

### D10. Prove the actual boundary

Each rebuild unit adds or adjusts its owning suite, records intended baseline
failures for changed behavior and independent enforcement removals/restored
passes, and runs the applicable gates. Tests compile real realm/dialect/seat
fixtures before observing full final commands; fabricated Controls or resolver
argv cannot close final-launch proof. Separate cold/actual-resume restriction
proofs and their final-delivery removals remain owed. Canonicalize one retained
temporary root for writes and expectations on Linux and macOS.

Run cargo test --workspace, locked all-feature workspace tests, fmt, clippy,
self/verify compiles and strict OpenSpec on implementation candidates. Exact
coverage remains literal nonzero covered/total equality for source lines,
branches and logical functions. Run it on a capable host/CI outside the nested
workspace box; pending external results are not green. Remote CI must name the
final head. No leftover mutation, threshold reduction or stale report may discharge it.

## Risks / Trade-offs

- A decoder may accept data that the current command cannot express → D5.3
  keeps runnable admission refused until its lowering/transport is proven.
- Hands and local restrictions can be mistaken for the same policy → D5.2
  retains hands replacement and explicitly limits direct mapping checks.
- A sandbox label may appear compatible while a different fragment runs →
  D5.3 validates each candidate's actual selected fragment and refuses unknown
  representations; later final-command proof remains independently required.

- Refusal breaks shipped inline permissions → migrate every inventoried file
  before enabling refusal; prove preserved local limits and empty realm grants.
- Grammar/provider disagreement → bound supported syntax, reject ambiguous
  managed patterns and compare final meaning, not merely parse success.
- Rebase changes roster and version identities → measure all affected compiles
  and re-pin with reasons; historical pins remain facts of their old revisions.
- Typed migration needs provenance beyond hands → establish it first and test
  counterfeit authored fragments with identical bytes.
- Local gates cannot establish external coverage or macOS behavior → record
  those results as pending until the named host/head evidence exists.
- A canonical path can change before a later open → bind the checked target to
  the read handle or refuse; do not claim adversarial race safety from strings.
- A shared parser and serializer can share a mistake → independent literal
  expectations and scoped provider evidence remain required.

## Migration Plan

Inventory re-read at a84197cd using recursive searches of adapters/, recipes/,
agents/, extensions/ and additionally bundles/. Re-run after rebase. Files below
are future migration work, not edits in this visit.

| File | Authored site / required migration |
| --- | --- |
| recipes/fast/bundle.json | implement and review: remove --permission-mode/--allowedTools; typed allow cargo, git, python3, pytest, ls, rg, mkdir; pytest retains Bash(.venv/bin/pytest:*), and permission mapping retains acceptEdits. |
| recipes/node/bundle.json | implement and review: remove both flags; typed allow npm, npx, node, git, ls, rg, mkdir; adapter names must be declared. |
| recipes/preflight/bundle.json | reviewer: remove both flags; typed allow cargo, git, ls, rg. |
| bundles/verify/bundle.json | reviewer: remove the same inline flags; typed allow cargo, git, python3, pytest, ls, rg, gh-pr-view, gh-run-view; preserve the two gh subcommand patterns, not unrestricted gh. |
| recipes/standby/bundle.json | implement/review: move --sandbox to typed tools.sandbox: implement danger-full-access, review workspace-write. |
| recipes/review-first/bundle.json | Codex review: move --sandbox workspace-write to its typed restriction. |
| recipes/wager-harness/bundle.json | Codex implement: move --sandbox danger-full-access to its typed restriction. |
| adapters/claude.json, adapters/lanetally.json | driver permission-mode templates and generated tool mappings are engine-owned; identify that origin explicitly. Add local command mappings needed by typed migration, not native grants. |
| adapters/codex.json, adapters/dsh.json | native/hands/route declarations are engine data; validate them, not migrate them into authored allowlists. No inline recipe migration found here. |
| agents/researcher.json | prior typed wants migration survives; no inline capability-bearing argv found in any agents/*.json. Preserve local restrictions and DATA charter. |
| recipes/research-dsh/bundle.json and recipes/research-dsh/drivers/research-web.yml | only route metadata in the bound patch; retain and revalidate, never treat historical web comments as a grant. |
| extensions/dsh/plugin-cli-session/lib/startup.js and extensions/dsh/plugin-cli-session/README.md | profile strings are CLI definitions/examples, not recipe/agent launch declarations. No migration or extension-code edit; include in the inventory evidence so absence is explicit. |
| recipes/node/README.md, recipes/wager-harness/README.md | replace inline-flag advice with typed examples after migration. |
| docs/guides/recipe-authoring.md, docs/guides/agent-library.md, docs/guides/provider-adapters.md | replace outward-link/reconciliation advice after enforcement is built; no unsupported guarantee. |

Preserve the complete existing local restrictions; removing a flag and falling
back to unrestricted defaults is not migration. Unsupported mapping fails
closed. If typed restriction representation cannot meet an owning requirement,
return upstream with evidence before dependent units. No grandfathering flag
or filename exception is permitted. Rollback is an explicit operator decision
to revert the release, never a hidden reconciliation compatibility mode.

## Open questions

- **Supported nonempty restriction proof (task 9.1, unit 9): unresolved.**
  Shipped Claude restrictions are unsupported. A synthetic --settings payload
  is insufficient to meet the existing compiled cold/resume positive. Qualify
  one bounded transport with provider/version evidence before load/refusal/
  final-proof work depends on it. If none exists, return upstream with evidence
  and amend the owning requirement/scenarios before proceeding; do not accept
  arbitrary settings or mark 21.2 complete from hand-built Controls.
- **Unit 1 outstanding proof (1.1):** replay and the inventoried guard-order
  split are recorded in evidence.md; unit 1b has landed. Its pending external
  results remain pending. Unit 2 neither replays the branch nor substitutes
  its local checks for those results.
- **Host and provider measurements:** final-head Linux/macOS, external exact
  coverage and remote CI remain unobserved on the rebuilt candidate. DSH and
  LaneTally retain independent uncertainty and Codex live evidence stays cold
  0.154.0. These are proof obligations, not unanswered permission to reconcile.

The operator's four rulings have no unresolved design alternative. Omission
versus empty local permissions, origin ownership, diagnostic bounds and the
file-read race boundary are answered in D5–D7 and their owning scenarios.

## Rebuild units

Execute this single numbered order. Each unit is one independently
commissionable visit based on the preceding commit. Task IDs now follow the
unit/group number; each checkbox records its previous ID for historical
traceability. Substeps close only at the unit named here. Count shipped JSON
as production.
Every feature unit has at most three named production files; tests and measured
pins accompany its own change. Do not add a new module to evade that ceiling.
If its actual file/scope budget is exceeded, split the plan before implementation.
Retain adopted history; only unit 1 replays it. Record each unit's intended
baseline reds and independent removals/restored passes in the existing evidence
and owning suites, then run the applicable house gates. Obtain external exact
coverage before claiming fully green; pending results stay pending. Later audit
units check those records, not another broad implementation campaign.

Paths abbreviated below resolve under the last explicit crate src/tests or
repository docs root named in that unit; they name existing files, not new suites.
Every unit also updates this change's tasks.md/evidence.md with observed results.

1. **Rebase slice-0065-capabilities onto current origin/main, re-pin digests,
   keep every gate green.** Close 1.1. Capture current fetched main and replay
   every adopted commit including a84197cd, 3c5402be and the tasks amendment.
   The observed seven-commit advance includes #320's model-generation roster, #321 v0.11.0,
   #313/#326 DSH work and #315/#322/#323. Expected production conflicts:
   `adapters/claude.json`, `adapters/codex.json`, `adapters/lanetally.json`.
   Test/pin conflicts: `crates/brokkr-runtime/tests/witness_digests.rs`,
   `crates/brokkr-runtime/src/bundle/compose_tests.rs` and
   `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`. Preserve main's
   version/lock and roster moves; measure affected witness/compose digests
   because engine version participates in identity. Record SHA/replay mapping
   and every gate, including external exact coverage and supported-host checks.
   No feature repair here. A fourth production conflict requires an inventoried
   split before resolution; replaying main's committed files is not permission
   to hide extra manual repairs. Unavailable gates do not count as green.
   **1b. Bind the guard order in dsh_launch_with.** Close 1.2, the inventoried
   split of unit 1's X1 (evidence.md, "Review return, 2026-09-23"). The replay
   kept #313's `dsh_input_boundaries` and the slice's `composed_launch` and
   chose their order by hand; no test bound it. The operator ruled the order
   (operator-ruling-2026-09-23.md, addendum): the authority refusal wins, and
   the boundary check reads the composed argv. Production:
   `crates/brokkr-protocol/src/adapters.rs` (only `dsh_launch_with` and its
   comment). Tests: `adapters/tests.rs` in that same src root. Record the
   ruling as one native-control delta requirement with a scenario. Build the
   case that tells the orders apart (a native control beside a
   boundary-faulted argv, exact reasons) and a positive both guards admit;
   an order-swap mutation must fail it. Change production code only if the
   test shows it differs from the ruling. Unit 1's external gates stay with 1.1.
2. **Decode typed inline declarations.** Close 2.1. Production:
   `crates/brokkr-runtime/src/agents.rs`, `agents/load.rs`, `bundle.rs` in that
   same src root. Add D5 tools.allow/tools.sandbox decoding and strict local
   subset/empty/omission validation under existing realm/boundary authority.
   Tests: agents/tests.rs, bundle/agent_tests.rs; full invalid/widening causes.
3. **Lower typed tools and define private origins.** Close 3.1; advance 4.2.
   Production: `crates/brokkr-runtime/src/agents.rs`,
   `crates/brokkr-runtime/src/capabilities.rs`,
   `crates/brokkr-protocol/src/native_controls.rs`. Carry authored/template/
   local/hands/native segments and expected state before flattening; define
   fallible private decoding/reassembly, without a new public contract.
   Tests: runtime agents/tests.rs and protocol native_controls/tests.rs; exact
   mapped limits and identical-byte origin distinction. No refusal activation.
4. **Wire origins through runtime dispatch.** Close 4.1 and 4.2. Production:
   `crates/brokkr-runtime/src/engine.rs`, `bundle.rs` in the same src root.
   Replace SiteSpawn's two-array reconstruction; carry the selected candidate's
   segments through boundary, expansion and final input merging using unit 3's
   contract. Tests: engine/capability_tests.rs, engine/boundary_tests.rs and
   crates/brokkr-runtime/tests/capability_launch.rs. Prove reconstruction and
   override/missing record refusal; copied bytes remain authored. Final
   refusal proof stays open in 15.2.
5. **Supply local mappings and scaffold support.** Close 5.1 and 5.2.
   Production: `adapters/claude.json`, `adapters/lanetally.json`,
   `crates/brokkr-cli/src/init.rs`. Preserve acceptEdits templates and supply
   npm/npx/node plus narrow gh-pr-view/gh-run-view mappings. Tests:
   crates/brokkr-runtime/tests/library_data.rs, crates/brokkr-cli/tests/init_stacks.rs
   and init_doctor.rs there; generated typed restrictions add no grant.
6. **Recipe/agent migration: Claude recipes.** Close 6.1. Production data:
   `recipes/fast/bundle.json`, `recipes/node/bundle.json`,
   `recipes/preflight/bundle.json`. Replace inline lists/modes with typed tools,
   preserving every prefix, including .venv/bin/pytest. No agent JSON requires
   an inline migration at the inventoried head. Tests:
   crates/brokkr-runtime/tests/capability_launch.rs and measured witness/compose
   pins, exact limits/native OFF. This separate migration precedes refusal.
7. **Recipe migration: verify and Codex restrictions.** Close 7.1. Production:
   `bundles/verify/bundle.json`, `recipes/standby/bundle.json`,
   `recipes/review-first/bundle.json`. Preserve the two narrow gh prefixes and
   each sandbox class in Migration Plan. Tests: brokkr-runtime/tests/capability_launch.rs,
   brokkr-runtime/src/bundle/model_policy_tests.rs under crates/ and measured
   witness/compose pins. No boundary widening.
8. **Recipe migration: wager and inventory.** Close 8.1. Production:
   `recipes/wager-harness/bundle.json`; docs `recipes/node/README.md`,
   `recipes/wager-harness/README.md`. Preserve typed danger-full-access under
   existing authority; update examples. Re-audit adapters/recipes/agents/
   extensions/bundles and scaffolds. Tests: brokkr-runtime/tests/capability_launch.rs
   under crates/; record every file disposition. Newly discovered migrations
   get bounded visits before unit 12; no filename exemption.
9. **Qualify a supported nonempty restriction.** Close 9.1 only on evidence.
   No production edits. Files: evidence.md, the owning native/realm deltas and
   design Decisions; `crates/brokkr-runtime/tests/capability_launch.rs` for the
   fixture design/probe if useful. Establish bounded provider/version semantics
   and a real compilation path, not arbitrary --settings JSON. An impossible
   positive returns upstream before unit 11; no fabricated plan closes it.
10. **Bound the grammar and redact diagnostics.** Close 10.1–10.4. Production:
    `crates/brokkr-protocol/src/native_controls/grammar.rs`. Classify catalogue
    effects, all aliases/split/equals/attached forms and five Codex config forms;
    retain only needed bounded inert assignments. Bound managed lists/separators
    and final positions. Test grammar/native_controls suites with full redacted
    causes, newline/long sentinels and DSH route-only controls. Unit 12 activates
    authored refusal; this primitive admits no new opaque syntax.
11. **Validate both declared halves at load.** Close 11.1 and 11.2. Production:
    `crates/brokkr-runtime/src/agents/load.rs`,
    `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-protocol/src/native_controls.rs`. Parse unused ON/OFF,
    selection maps/separators and unit 9's qualified restriction representation.
    Remove Codex's verbatim bypass. Tests: runtime agents/tests.rs,
    capabilities/tests.rs and protocol native_controls/tests.rs; invalid unused
    halves and positive identity pins.
12. **Enable authored refusal and engine-only composition.** Close 12.1 and
    12.2; advance 15.2. Production: `crates/brokkr-protocol/src/native_controls.rs`,
    `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-runtime/src/bundle.rs`. Delete authored list folding and
    value-dependent admission; compose only typed engine contributions. Tests:
    protocol native_controls/tests.rs and brokkr-runtime/tests/capability_launch.rs;
    every harness/form, grant state, counterfeit origin, managed hard limit and
    migrated shipped positive.
13. **Build pure final assessment and share structural consumers.** Close 13.1
    and 13.2. Production: `crates/brokkr-protocol/src/native_controls.rs`,
    `native_controls/grammar.rs`, `adapters.rs` in the same src root. Finish the
    pure complete builder/checker, exact-state comparison and private checked
    command. Replace raw selector/extraction readers, including --image resume.
    Tests: adapters/tests.rs and native_controls/tests.rs; full independent
    state/refusal expectations. Integration remains open below.
14. **Integrate checked cold serving commands.** Close 14.1. Production:
    `crates/brokkr-protocol/src/adapters.rs`. After all expansions/prefixes,
    obtain and consume unit 13's checked value at Codex, Claude/LaneTally child
    and DSH cold serving seams. Tests: adapters/tests.rs,
    crates/brokkr-runtime/tests/capability_launch.rs; independent compiled
    commands, dropped OFF, changed separators, cross-origin duplicates and
    attempts to mutate after checking. No resume eligibility work here.
15. **Integrate eligible resume and replacement.** Close 15.1 and 15.2.
    Production: `crates/brokkr-protocol/src/adapters.rs`. Check each actual
    eligible resume and rejected-rejoin cold replacement independently; preserve
    session/version/sandbox/effort/accounting eligibility and private origins.
    Tests: adapters/tests.rs, crates/brokkr-runtime/tests/capability_launch.rs;
    exact session/stdin shape, selected fallback, malformed private records and
    ON/OFF/refusal expectations. Cold replacement is never resumed evidence.
16. **Bind canonical inputs and policy bytes.** Close 16.1, 16.2 and 16.3.
    Production: `crates/brokkr-runtime/src/bundle.rs`, `bundle/compose.rs` in
    that same src root. Refuse outward/excluded/nonregular/unpinned inputs,
    bind contained target to read handle or refuse, and bind policy parse/hash
    buffer to owner identity including overridden ancestors. Tests:
    bundle/compose_tests.rs; standalone/inherited escapes, FIFO, controlled
    path replacement, contained symlink positives and independent byte changes.
17. **Select charter owner and source at compile.** Close 17.1. Production:
    `crates/brokkr-runtime/src/bundle.rs`, `agents.rs`, `agents/load.rs` in that
    same src root. Carry owner/reference/canonical target/existing digest for
    each selected candidate; remove longest-prefix owner guesses. Tests:
    bundle/agent_tests.rs, agents/tests.rs; external/nested/overlapping owners,
    every selected site and fallback, no recipe escape reclassified as library.
18. **Consume the bound charter at dispatch and rendering.** Close 18.1, 18.2
    and 18.3. Production: `crates/brokkr-runtime/src/engine.rs`,
    `crates/brokkr-runtime/src/bundle.rs`,
    `crates/brokkr-protocol/src/adapters.rs`. Use units 16–17's verified read
    and selected owner; merge verified text last and never reopen at rendering.
    Tests: runtime engine/capability_tests.rs, engine/boundary_tests.rs and
    protocol adapters/tests.rs; equal-byte/changed/missing targets without
    recompile, selected prompt/DATA facts and controlled replacement refusal.
19. **Enforce charter integrity at start and pinned resume.** Close 19.1.
    Production: `crates/brokkr-runtime/src/engine.rs`, `bundle.rs` in the same
    src root. Apply existing start/resume identity doors to unit 17's complete
    selected owner binding and unit 16's verified reader. Tests: runtime
    engine/boundary_tests.rs, engine/capability_tests.rs and
    crates/brokkr-cli/tests/capability_verbs.rs; changed/excluded/equal-byte
    retarget/missing/restored inputs, mapped/unmapped pinned context. Neither
    dispatch proof nor recompile substitutes for these consumption checks.
20. **Audit compiled refusal and serving-shape matrix.** Close 20.1; advance 21.3.
    No production edits. Tests: `crates/brokkr-runtime/tests/capability_launch.rs`,
    `crates/brokkr-protocol/src/adapters/tests.rs` and
    `crates/brokkr-runtime/src/engine/capability_tests.rs`. Each supported
    harness/form/site/cold/resume/fallback row names a real compiled whole-command
    or full-refusal assertion and selected charter facts. Fill missing assertions
    in these suites; holdings-only/is_ok evidence closes no row. Restriction
    rows close only with unit 21; task 21.3's full matrix remains open until then.
21. **Prove compiled restrictions at cold and actual resume.** Close 21.1, 21.2
    and the remaining restriction rows of 21.3. No production edits. Tests:
    `crates/brokkr-runtime/tests/capability_launch.rs` and
    `crates/brokkr-protocol/src/adapters/tests.rs`. Use real realm/dialect/
    candidate resolution for managed Read/empty controls and unit 9's supported
    held nonempty restriction. Separate cold and eligible-resume expectations,
    structured manifest objects and session identity; hand-built Controls fail
    this obligation. Record independent final-delivery removals.
22. **Doctor submits the complete plan in both paths.** Close 22.1 and 22.2.
    Production: `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-cli/src/doctor.rs`. Consume units 13–15's same final composer;
    remove synthetic per-capability delivery. Tests: doctor/capability_tests.rs;
    interacting OFFs, include/deny conflict, admitted plan, every grant shape
    and labeled adapter-only scope, with independent full compile/report lines.
    Retain 0.19's no-provider/no-capability-server evidence.
23. **Audit launch enforcement removals.** Close 23.1. No permanent production
    edits. Files: brokkr-runtime/tests/capability_launch.rs, brokkr-protocol/src/adapters/tests.rs,
    brokkr-protocol/src/native_controls/tests.rs under crates/ and evidence.md. Check
    or run each missing isolated authored/load/final parse/state/ON/OFF/cold-resume
    restriction removal from units 10–15/21. Every intended assertion fails and
    passes after restoration; record revisions, never a build error as proof.
24. **Audit identity enforcement removals.** Close 24.1. No permanent production
    edits. Files: brokkr-runtime/src/bundle/compose_tests.rs,
    brokkr-runtime/src/engine/boundary_tests.rs, brokkr-cli/tests/capability_verbs.rs under
    crates/ and evidence.md. Check or run missing isolated containment/regular-
    policy binding/owner-target/read-binding/verified-buffer removals from units
    16–19; retain consulted/unconsulted controls. Restore each mutation and
    record its intended failure and pass.
25. **Audit proof history and portability.** Close 25.1, 25.2 and 25.3. No production
    edits. Files: brokkr-runtime/src/agents/tests.rs, brokkr-protocol/src/adapters/tests.rs,
    brokkr-runtime/tests/capability_launch.rs under crates/, evidence.md and tasks.md.
    Verify each baseline red/fix/removal/restored record and matrix assertion;
    earlier units must capture reds before changes. Retain one canonical TempDir
    root per fixture and obtain Linux/macOS evidence. Never invent history.
26. **Update guides, measured pins and scope audit.** Close 26.1–26.3. No
    production edits. Files: `docs/guides/recipe-authoring.md`, agent-library.md,
    provider-adapters.md in that guides root; brokkr-runtime/tests/witness_digests.rs,
    brokkr-runtime/src/bundle/compose_tests.rs under crates/, evidence.md and tasks.md.
    Replace old advice with implemented refusal/containment/whole-plan behavior
    and actual limits. Measure final self/verify/affected witness identities and
    append reasons. Audit frozen bytes, empty grants, scope and compiler pins.
27. **Final candidate gates and commit.** Close 27.1–27.5 only on observed
    success. Evidence/tasks and run-local records only, no production edits.
    Run fmt, locked all-target/all-feature clippy, all seven crate suites, both
    workspace suites, self/verify compiles, strict OpenSpec and diff check.
    Obtain final-head external exact coverage with all three nonzero equal
    counts and relevant Linux/macOS/remote CI evidence; both workflows and
    coverage must consume rust-nightly-version.txt. Commit restored work in
    repository style, never push. Record committed SHA/results in run-local
    evidence without changing the validated source head. Pending gates remain
    open. Task 28.1 awaits separate council judgment; no archive/publication
    or security clearance follows from this plan.
