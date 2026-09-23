# Decision 0065, slice one — refuse, never reconcile

Status: proposed design; decision 0066 remains proposed.
Adopted: every commit through 44430402 on slice-0065-capabilities.
Authority: [the complete operator ruling](operator-ruling-2026-09-23.md).
This visit authors documents only. No behavioral finding or security hold is closed.

## Context

See proposal.md for the four governing changes. This revision adopts the first
and second chief rulings and reads all three completed third-council positions
in `.forge/tasks/council-positions-80bfd784.md`. The third adversarial position
was blocked twice; no position or chief judgment is invented for it. The run
context has no returned_from. The named ruling supplies the return obligations.

Local origin/main is 072cdd9b, seven commits beyond the shared branch ancestry:
#313, #315, #320 (model generations), #321 (v0.11.0), #322, #323 and #326.
The rebase is future unit 1, not work performed by this documentation visit.
Production remains Rust under crates/. Hosts remain Linux and macOS (0063).

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

Compile preflights the whole command shape. Immediately before EVERY process
launch, parse the actual complete serialized command after engine prefixes,
wrapper settings, expansion, boundary/model/effort, session and stdin arguments.
Compare the effective capability state with the sealed plan: exact held/denied
powers, tool subset, restrictions and hands. Unknown, missing, extra or
contradictory effects refuse before spawn. This is ruling 2's “final command,
not an intermediate composer.” A round-trip preserving the same wrong string
is insufficient. Share this final composer/check with doctor.

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
final head. No mutation, threshold reduction or stale report may discharge it.

## Risks / Trade-offs

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

## Migration Plan

Inventory read at 44430402 using recursive searches of adapters/, recipes/,
agents/, extensions/ and additionally bundles/. Re-run after rebase. Files below
are future migration work, not edits in this visit.

| File | Authored site / required migration |
| --- | --- |
| recipes/fast/bundle.json | implement and review: remove --permission-mode/--allowedTools; typed allow cargo, git, python3, pytest, ls, rg, mkdir. |
| recipes/node/bundle.json | implement and review: remove both flags; typed allow npm, npx, node, git, ls, rg, mkdir; adapter names must be declared. |
| recipes/preflight/bundle.json | reviewer: remove both flags; typed allow cargo, git, ls, rg. |
| bundles/verify/bundle.json | reviewer: remove the same inline flags; typed allow cargo, git, python3, pytest, ls, rg, gh-pr-view, gh-run-view; preserve the two gh subcommand patterns, not unrestricted gh. |
| recipes/standby/bundle.json | implement/review: move --sandbox values to checked typed local sandbox restrictions. |
| recipes/review-first/bundle.json | Codex review: move --sandbox to its typed restriction. |
| recipes/wager-harness/bundle.json | Codex implement: move --sandbox to its typed restriction. |
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

## Rebuild units

Execute this single numbered order. Each unit is one independently
commissionable visit based on the previous unit's committed result; no parallel
order is implied. Each names its production files, owning tests and tasks it
closes. Unlisted production work requires splitting the commission before
implementation: at most three production files per feature unit, counting
shipped runtime data too. Tests, measured digest pins and evidence/tasks updates
accompany the owning unit. All units retain every adopted change; unit 1 alone
replays history. Every unit runs applicable tests, workspace tests, fmt,
clippy, self compile and strict validation; obtain external exact coverage
before claiming that unit fully green. A pending gate is recorded pending.

1. **Rebase slice-0065-capabilities onto current origin/main, re-pin digests,
   keep every gate green.** Close 0.4. Capture the fetched main SHA (local
   observation 072cdd9b), replay all adopted slice commits including this docs
   revision, retaining the seven main commits named in Context. Expected
   reconciliation: `adapters/claude.json`, `adapters/codex.json`,
   `adapters/lanetally.json`; tests/pins in
   `crates/brokkr-runtime/tests/witness_digests.rs`,
   `crates/brokkr-runtime/src/bundle/compose_tests.rs` and
   `crates/brokkr-runtime/src/bundle/model_policy_tests.rs`. Preserve #320's
   roster and #321's version/lock changes, plus #313/#326's DSH behavior; no
   new feature repair in this unit. Measure every affected witness/compose
   digest; engine-version identity matters. Run the whole gate set, including
   external exact coverage and Linux/macOS checks, record the new head and
   mappings of adopted commits. Do not count a pending result as green. If
   conflict resolution needs additional production edits beyond these three,
   return the expanded conflict inventory for a split before resolving it.
2. **Typed declaration decoding.** Close 3.14. Production:
   `crates/brokkr-runtime/src/agents.rs`, `crates/brokkr-runtime/src/agents/load.rs`,
   `crates/brokkr-runtime/src/bundle.rs`. Add D5's typed inline local allow/sandbox
   representation and strict validation, preserving existing agent semantics,
   realm/boundary limits and no widening by a seat. Tests: agents/tests.rs and
   bundle/agent_tests.rs; full unsupported and widening causes. No raw flag
   exception is added.
3. **Typed lowering and origin carriage.** Close 3.15. Production:
   `crates/brokkr-runtime/src/agents.rs`, `crates/brokkr-runtime/src/capabilities.rs`,
   `crates/brokkr-protocol/src/native_controls.rs`. Lower typed restrictions;
   separate adapter template, authored, local permission, hands and native
   origins before flattening. Tests: agents/tests.rs, native_controls/tests.rs,
   runtime/tests/capability_launch.rs; identical authored bytes cannot inherit
   engine provenance. Keep the new support usable before refusal is enabled.
4. **Shipped mapping and scaffold support.** Close 3.16 and 8.2. Production:
   `adapters/claude.json`, `adapters/lanetally.json`,
   `crates/brokkr-cli/src/init.rs`. Supply all local names required by migration,
   including npm/npx/node and the narrow gh-pr-view/gh-run-view mappings, and
   preserve engine-owned permission templates.
   Tests: library_data.rs, init_stacks.rs, init_doctor.rs; generated no-grant
   scaffolds and typed lists yield the expected limits, never native grants.
5. **Recipe/agent migration: Claude recipes.** Close 3.17. Runtime data only:
   `recipes/fast/bundle.json`, `recipes/node/bundle.json`,
   `recipes/preflight/bundle.json`. Move every listed site's inline flags to
   typed declarations. Agents already using typed permissions retain them;
   no agent file needs an inline-flag migration at the inventoried head.
   Tests: capability_launch.rs and witness/compose pins; exact local limits
   and native OFF, including preflight's reviewer. This is its own migration
   visit, before any refusal lands.
6. **Recipe migration: witness and Codex restrictions.** Close 3.18. Data:
   `bundles/verify/bundle.json`, `recipes/standby/bundle.json`,
   `recipes/review-first/bundle.json`. Preserve verify's typed local list and
   Codex's checked sandbox restrictions. Tests: capability_launch.rs,
   model_policy_tests.rs, witness/compose pins; no broadened boundary or grants.
7. **Recipe migration: remaining wager and inventory.** Close 3.19. Data:
   `recipes/wager-harness/bundle.json`; docs `recipes/node/README.md` and
   `recipes/wager-harness/README.md`. Re-run the whole inventory from Migration
   Plan, record all agents/adapters/extensions dispositions and prove every
   shipped migrated recipe. Any newly found runtime file starts a separately
   bounded migration visit before unit 11, never an exemption.
8. **Qualify the supported restriction proof.** Close 0.3. No production edit:
   inspect supported --settings semantics and establish a bounded nonempty
   restriction transport satisfying exact planned state, with provider/version
   evidence and production-compiled fixture design. Update evidence.md and
   runtime/tests/capability_launch.rs as needed. A settings-shaped JSON string
   alone proves no restriction. If no supported transport meets the owning
   positive requirement, report upstream before units 9–13 depend on it.
9. **Grammar and bounded diagnostics.** Close 3.9–3.12. Production:
   `crates/brokkr-protocol/src/native_controls/grammar.rs`. Model the catalogue,
   bounded config effects, engine final command/subcommand positions, aliases,
   all five Codex forms, and value-redacted errors. Validate managed list
   syntax/separators without a permissive splitter. Tests:
   native_controls/tests.rs and grammar tests; every harness/form plus inert
   values. This establishes the primitive; unit 11 activates authored refusal.
10. **Validate declared ON and OFF at load.** Close 3.3 and 4.4. Production:
    `crates/brokkr-runtime/src/agents/load.rs`,
    `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-protocol/src/native_controls.rs`. Parse both declared halves,
    selections and unit 8's transport before use; unknown Codex argv and bad
    separators refuse even when unused. Tests: agents/tests.rs,
    capabilities/tests.rs, native_controls/tests.rs; valid controls remain
    positive and restriction JSON remains identity-bound.
11. **Enable authored refusal and engine-only composition.** Close 4.3,
    4.6 and 4.7. Production: `crates/brokkr-protocol/src/native_controls.rs`,
    `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-runtime/src/bundle.rs`. Delete authored-list folding/admission
    reconciliation; refuse the option regardless of value/grant. Keep exact
    engine restrictions and private provenance across compile entry points.
    Tests: native_controls/tests.rs and capability_launch.rs, complete bounded
    causes for every form/harness and migrated shipped positives.
12. **Final command check and shared consumers.** Close 3.13, 7.1 and 7.2.
    Production: `crates/brokkr-protocol/src/adapters.rs`,
    `crates/brokkr-protocol/src/native_controls.rs`,
    `crates/brokkr-protocol/src/native_controls/grammar.rs`. Replace raw managed
    readers; parse actual complete serialized cold/resume/replacement commands
    and compare exact planned state immediately before spawn, including
    LaneTally child and DSH. Export the same pure complete composer for doctor.
    Tests: adapters/tests.rs and capability_launch.rs; prefix duplicates,
    separators, selectors and mutated final state refuse before provider work.
13. **Canonical input and policy identity.** Close 5.1, 5.2 and 6.1. Production:
    `crates/brokkr-runtime/src/bundle.rs`,
    `crates/brokkr-runtime/src/bundle/compose.rs`. Refuse outward/nonregular
    inputs, bind policy read/hash/parse to the owner pin. Tests:
    bundle/compose_tests.rs; direct outward links, symlink-parent escape,
    FIFO policy, standalone/inherited variants, contained positive controls.
14. **Selected charter owner binding.** Close 6.2. Production:
    `crates/brokkr-runtime/src/bundle.rs`, `crates/brokkr-runtime/src/agents.rs`,
    `crates/brokkr-runtime/src/agents/load.rs`. Carry owner/original reference/
    canonical target/existing digest through all selected sites/candidates;
    remove longest-prefix guessing. Tests: bundle/agent_tests.rs,
    agents/tests.rs; external/nested/overlapping owners and fallback.
15. **Consume the owner-bound charter.** Close 6.3–6.5 and 8.1. Production:
    `crates/brokkr-runtime/src/engine.rs`, `crates/brokkr-runtime/src/bundle.rs`,
    `crates/brokkr-protocol/src/adapters.rs`. Enforce owner/target/bytes at
    start, pinned resume and dispatch; hand verified text to rendering.
    Tests: engine/boundary_tests.rs, engine/capability_tests.rs,
    cli/tests/capability_verbs.rs, adapters/tests.rs. Equal-byte retargets
    refuse without recompilation; all serving paths consume only verified text.
16. **Compiled launch proof matrix.** Close 7.3–7.6. No production edits:
    `crates/brokkr-runtime/tests/capability_launch.rs`,
    `crates/brokkr-protocol/src/adapters/tests.rs`,
    `crates/brokkr-runtime/src/engine/capability_tests.rs`. Compile real
    fixtures for every supported serving shape; assert complete independent
    literals or full refusals. Separate held nonempty restriction cold and
    actual eligible-resume tests, manifest object and session assertions.
17. **Whole-plan doctor.** Close 8.3 and 8.4. Production:
    `crates/brokkr-runtime/src/capabilities.rs`,
    `crates/brokkr-cli/src/doctor.rs`. Pass complete plans to unit 12's composer
    in both reporting paths. Tests: doctor/capability_tests.rs; combined OFF
    conflicts, all grant shapes, scoped adapter-only statements and independent
    matching compile/report literals. Retain 8.5's no-server/MCP report evidence.
18. **Launch enforcement removals.** Close 9.2. Tests/evidence:
    capability_launch.rs, adapters/tests.rs, native_controls/tests.rs and
    evidence.md. Independently remove authored refusal, load parsing, final
    parsing/state equality, cold/resume restriction delivery and ON/OFF
    protections; record intended failures then restore every mutation. No
    permanent production edit; keep the experiments individually bounded.
19. **Identity enforcement removals.** Close 9.3. Tests/evidence:
    bundle/compose_tests.rs, engine/boundary_tests.rs, cli/tests/capability_verbs.rs,
    evidence.md. Independently remove containment, regular-file/policy binding,
    owner/target checks and verified-buffer delivery; restore each. Retain
    consulted/unconsulted and grant/restriction identity controls.
20. **Proof and portability audit.** Close 0.2, 3.6 and 9.4. Tests/evidence:
    agents/tests.rs, adapters/tests.rs, capability_launch.rs, evidence.md and
    tasks.md. Audit the baseline/fix/removal/restored revision per case, every
    real matrix assertion and canonical fixture roots. Earlier units record
    their reds before changing enforcement; this unit verifies completeness,
    never invents a pre-fix observation. Obtain Linux/macOS results.
21. **Guides, measured pins and scope audit.** Close 10.1–10.3. No production
    edits: docs/guides/recipe-authoring.md, agent-library.md, provider-adapters.md, witness_digests.rs,
    bundle/compose_tests.rs, evidence.md and tasks.md. Correct outdated
    outward-link advice, document refusal and migration, compile actual final
    self/verify/all affected witnesses and re-pin with reasons. Check frozen
    bytes and unchanged empty realm grants; preserve historical release facts.
22. **Final candidate gates and commit.** Close 11.1–11.5 only on observed
    success. Evidence/tasks and run-local records only; no production edit.
    Run fmt, locked all-target/all-feature clippy, all seven crate suites,
    both workspace suites, self/verify compiles, strict OpenSpec and diff check.
    Obtain final-head external exact coverage (all three counts) and relevant
    Linux/macOS/remote CI results. Verify both workflows and coverage consume
    rust-nightly-version.txt. Commit in repository style, never push; record
    final SHA without creating a different tracked source head. Missing gates
    remain pending. Task 12.1 remains open for separate council re-judgment;
    this plan neither clears the hold nor authorizes archive/publication.
