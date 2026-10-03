# Decision 0065 slice two — broker, gates and citable calls

Status: proposed design for operator ruling; documents only.
Change: decision-0065-capabilities-slice-two.
Source base: 2a23488b19f6dea271264f6a85b40d46a84af39e.
Council checked HEAD: 1c96ca2c37908e7900cbdc2a402331b051d85513, 2026-10-03.
Run: build-decision-0065-slice-two-sp-cd8dd603.

## Context

See [proposal.md](proposal.md) for motivation and adoption. Read the local
dialect file and all specify/clarify/design/tasks/analyze/return instructions,
then the rendered OpenSpec artifact instructions, without invoking a workflow
runner. The two operator maps were read whole, as were the commissioned
decisions and slice-one proposal/design/specs/operator rulings. D4's ordering
and D11's deferral are constraints, not opportunities to redesign native power.

This design visit adopts the committed change at 1c96ca2c. The supplied
returned_from is clarify's clear review of the adopted R-G/R-H repairs;
there is no outstanding clarification finding to disguise downstream. Both
current positions were read whole at .forge/design/positions/robustness.md
and simplicity.md. D1 reconciles their claims against source, and D11 records
the current analysis. R-I makes existing MB3 startup protection concrete;
its scenario precedes this design and tasks. Historical upstream returns and
their owning reviews remain identified by commit below. Nothing here accepts
0077, closes implementation or invents a harness measurement.

The following evidence was rechecked at this base. Paths use the crate name
plus src unless prefixed with contracts; line facts are starting evidence,
not promised locations after another thread merges.

| Checked source | Design consequence |
| --- | --- |
| runtime/capabilities.rs:234–242, :477–490, :621–716 | SiteAsks lacks class; ToolDialect discards the four MCP fields. Type the edge and carry site facts. |
| runtime/capabilities.rs:1436–1526, :1626; cli/doctor.rs:999–1001 | Validate every grant before the unbuilt-kind fence; remove native-binding index/expect assumptions before enablement. |
| core/realms.rs:403–517 | Three reserved keys today; reserve retain by realm version, never globally reinterpret old restrictions. |
| runtime/bundle.rs:6054–6091 | Class is consulted only for a harness fragment today; capability policy must receive canonical class on every site. |
| runtime/engine.rs:4434–4512; protocol/hands.rs:1119–1142 | Harness launches hands. A broker belongs in that config and attempt tree. |
| protocol/native_controls.rs:2433–2456 | Final proof admits only the current transport; adding flags alone will refuse. |
| protocol/adapters.rs:1603–1620, :1836–1891 | Names are clamped and Codex emits only item type; normalize/correlate before display conversion. |
| runtime/engine.rs:2091–2107, :2394–2406; engine/sequence.rs:395–416, :616 | Single/panel sinks and step dispatch/settlement all need the same attribution and evidence barrier. |
| store/lib.rs:316–329; store/seat_record.rs:57–115 | Append and offline reading share engine-line contract dispatch; v6 must be additive on valid old records. |
| protocol/adapters.rs:687–704; protocol/secret.rs:20–30, :105–137; cli/tests/machine_proof.rs:3141–3184 | One injector overwrites existing entries; HOME/TMPDIR remain globally valid. Protect broker-owned keys before lookup, and preserve injector cardinality. |
| runtime/engine/marks.rs:25–55, :128–155; capabilities.rs:1314–1370; protocol/adapters.rs:226–269, :398–425 | Selection, stale-notice clearing and capability rendering already exist. Derive discovery from that selected intent in U7c/U7d. |
| runtime/bundle.rs:4352–4362; contracts/tool-dialect.v1.schema.json | Binding minimum is an existing policy fact; MCP version/connection/secrets/retained already have a frozen schema. |
| agents/charters/researcher.md:20–30 | Existing prose names both capabilities and the exact DATA clause in one paragraph. |

The adopting office recorded, and the earlier specify revisit repeated, a GitHub API
and complete changed-file check for all six open PRs on 2026-10-03: #404, #452, #460, #486, #494 and #500.
Their titles/bodies/file paths/patches contain no 0077 claim. The index gap
table reserves 0072/0074/0075 and 0076 exists; retain proposed 0077. This is
an observation, not a remote reservation. The commission's agreement that
#487 takes realms v7 supersedes PR #494's historical v6 wording.
This council preserves that dated observation. Its fresh gh query could not
authenticate (GH_TOKEN absent), and the public API hostname could not resolve
in the box. It does not re-certify open PRs or reserve 0077 remotely. The local
index and amendment pointers still name proposed 0077; recheck concurrent
claims before its implementation PR.

## Goals / Non-Goals

A broker launch must be reconstructible from one sealed selected holding.
All effects remain above pure core/view (0071 ruling 1); no second policy
resolver, general plugin registry or trait-object catalog (rulings 2, 5, 10).
Preserve the current native denial and private-origin final-command proof.
Every future unit must have a production consumer, bounded files and binding
tests (rulings 4, 6, 9).

No URL client, native response retention, nonempty restriction transport,
server sandbox, whole-harness box, new provider, new shipped MCP server or
capability grant, comparison/slice-three work, garbage collector or release.
R2 does not build macOS namespace support; macOS keeps its exact refusal.
No production/test/schema/data edit or live U0 measurement occurs in this visit.

## Decisions

### D1. Current council reconciliation at 1c96ca2c

Both positions in the supplied roster were read in full. Their pass results
mean advice was produced, not that the design or runtime is accepted. Adopt,
combine or reject the claims below on their checked evidence, not by vote.

| Claim / source | Disposition, evidence and owner |
| --- | --- |
| Both: preserve R1–R5, D4/D11 and the adopted repairs | Adopt. engine.rs:4434 and hands.rs:1119 establish harness launch; the singleton final check needs an independently sealed complete set. Proposed 0077 changes only the launch letter of 0065 ruling 6. Namespace-only admission, realm authority, engine-only journal and U9b enablement stand (0071 rulings 1–3, 5, 8, 10). |
| Robustness R-I: injection can replace protected HOME/TMPDIR | Adopt under existing MB3. adapters.rs:687–704 overwrites names; secret.rs:20–30,105–137 permits both. One consumed base-environment builder owns fixed values and their reserved set, rejects collisions before store lookup/spawn, and forbids later replacement. Reject trusting current values, version pins, global name/schema edits or a second injector. D5/D6 and U6c/U6f/U9b bind each collision separately; no extra production path (rulings 2, 3, 5, 8–10; high security if bypassed). |
| Robustness 1 and simplicity's bounded session: isolation facts differ | Combine unchanged at SI1/SI2/MB2 and D2/D3. Codex's own adapter says ambient exclusion is unmeasured. Carriage, ambient strictness, native write containment and store/process read isolation stay separate; retain all positive sentinels and invocation shapes. Reject guessed Codex precedence, borrowed wrapper evidence or a roster exemption when self cannot compile (rulings 2, 3, 8, 9). |
| Robustness 2 and simplicity's one authority derivation | Adopt selected holding plus independent final equality, canonical gate class/stable office and independent native OFF. marks.rs selects one provider/model; realm reaches() cannot prove explicit gate egress scope. Preserve GP1/GP2's writes-first rule and one same-paragraph DATA declaration. No fallback union, argv-derived authority or equal-byte authored provenance (rulings 2, 3, 5, 8, 10). |
| Robustness 3 and simplicity's private durability | Combine and preserve R-G at MB3/MB5/CR3/CR4, D7. process.rs:499–520 trusts driver status; checkpoints.rs:169–248 offers no append receipt. Keep latched Clean/Failed closure, independent engine judgment, durable acceptance, reserved closure space, confirmed fenced append and full-payload deduplication. Retain 0/1/4,096-call controls and native loss/stranding refusals; no live scanner, public lifecycle, cursor or uncertain replay (rulings 3, 5, 7–9). |
| Robustness 4 and simplicity's one prepared buffer | Combine and preserve R-H at MB4/CR2, D5/D7. The shared masker leaves numeric scalars unchanged, including exact 876543210. Retain the exact unsafe-masking refusal independently of precision validation and retention policy, one masked canonical buffer, shared encoding definitions and fixed attempt shares. Persistence proves prepared output, not receipt; no coercion, truncation, silent downgrade or quota reset (rulings 3, 5, 8, 9). |
| Robustness 5 and simplicity's accepted custody cost | Adopt CR2 and D6 unchanged. Writable hands mounts expose sibling/historical evidence; a digest does not establish custody. Keep private bound handles, all-writer U8a2, read-only publication, alias checks, writer lease through settlement, older-engine quiescing and verify-before-reuse/inspect. Accept same-root serialization; distinct roots still need alias-safe write reach (rulings 1, 3, 7–10). |
| Simplicity: only required contracts and one read model | Adopt frozen tool-dialect v1, seat-record v6, manifest v12 and the next realms version after v7, with historical meaning intact. Keep attribution in view, inspect in readouts.rs and the shared injector in secret.rs; its cardinality proof must now inspect that module. D4/D8 and U4/U5/U6a/U8g own these choices (rulings 1, 3–8, 10). |
| Simplicity: absorb standalone discovery U7e | Adopt. marks.rs already selects holdings/clears notice; adapters.rs already renders the fixed notice/capability contract. U7c carries and clears only selected server/tool/discovery identifiers as a projection of configuration intent. U7d renders them and closes tasks 37.1–37.2 with 36.1–36.2; U8a depends on U7d. Retain both runtime and protocol suites, separate selection/rendering removals, unchanged requested digest and native/no-MCP prompts. Reject another registry, full plan in the prompt or string-key state beyond the edge (rulings 3–6, 10). |
| Both: bounded units and real consumers | Adopt 43 PRs and all 100 tasks. Keep U4a/U4b and U8a/U8a2 separate: their unions exceed three production files and the latter proves all writers. Keep U6's minimal ledger before forwarding and the public command's own incomplete-serving/retention refusals; the compile fence cannot protect manual CLI calls. U1–U4 remain independent as D9 states (rulings 4, 6, 9). |
| Simplicity's exclusions; robustness's bounded extension seam | Reject URL execution, reconnect/retry/pooling, extra MCP methods, nonempty restriction transport, installers/attestation/interpreter analyzers, another box, dynamic quota/index/GC services, new discovery prose/registries, duplicate serializers/accessors and native response capture. They add authority, lifetime or public vocabulary without a commissioned consumer. Typed stdio data and existing process seams suffice (rulings 1–3, 5, 6, 8, 10). |
| Simplicity: shorten history and repeated gate prose | Adopt one current reconciliation table, compact dated history below and in D10/D11, and one shared task-verification duty. Preserve commit identities, exact prior outcomes, all task IDs and proof-specific controls. No retrospective artifact or relabelled historical result (ruling 5). |
| Both: costs and proof still owed | Accept serial/fsynced calls, delayed settled display, unused static quota, conservative masking and unsupported harness shapes. Preserve the named #403/0012 residuals and operator-installed-code boundary. Retain real compile→launch→broker→journal→inspect and independent removal controls for every refusal; external gates and operator acceptance remain pending. Gate-owned findings are informational under ruling 11; security refusals stay rated under rulings 3, 8, 9. |

### D1.1. Historical council record

The complete earlier reconciliations remain in committed design versions
7aa9e9ff and b310e094. Their owning specify reviews are 7f36e922 and 1c96ca2c.
They established secret-read/startup and all-writer protection, settled
journal evidence, correlation/budgets, one DATA declaration, existing canonical
and launch ownership, then R-G/R-H. The bounded U5/U6/U9/U10 combinations and
secret/readouts module reuse remain in this plan. D11 records the original
upstream reasons and their adoption; those outcomes are not this council's.
R1–R5 remain verbatim and only the operator can accept proposed 0077.

### D2. U0 is an experiment with an explicit decision rule

U0 uses disposable host-side configurations, fake sentinel MCP servers and
real installed harnesses on Linux/macOS. It changes evidence only. Capture
binary version, host, cold/eligible-resume/replacement shape, effective config
sources, exact argv/environment names (redacted), server lifecycle log,
tool listing/discovery/call attempts, native call events and limitations.
No production default is changed by this experiment.

| Harness | Candidates and distinct measurements |
| --- | --- |
| Claude | Strict MCP flag plus explicit empty, hands-only and engine-sentinel config. Plant user/project/plugin ambient sentinels; inspect starts, lists and denied direct calls. |
| LaneTally | The actual wrapper and child, with the same sentinel matrix. Neither Claude's result nor successful argv forwarding qualifies the wrapper. |
| Codex | (a) a private engine-owned config/home containing only needed auth references and session/model configuration; (b) replacement of the complete MCP table using supported config precedence. Measure user, project, managed and plugin sources, auth functionality, normal model/effort, session storage/rejoin, deferred discovery, and native/MCP event call identities independently. |
| dsh | Existing profile/plugin configuration versus an engine-only profile candidate; separately measure ambient exclusion and engine-server loading. No new plugin is built here. |
| exec | No model MCP surface. Record inapplicable, not measured strictness. |

The positive control must start/discover/call each planted ambient sentinel.
A candidate succeeds only if all those ambient servers neither start nor
appear nor answer while the engine sentinel works. A model voluntarily
ignoring a server is not exclusion evidence. Run with and without hands and
on each currently supported serving shape; do not qualify a new resume
shape. Existing native OFF must survive both candidates.

Choose a mechanism only after its row passes. If both pass, prefer the one
that isolates configuration without copying credentials/session data, with
fewer independent precedence assumptions. If neither passes, declare the
shape unmeasured/unsupported with SI2's exact cause; no fallback to ambient
config. U0 also captures the exact native/MCP call identifiers and repeated
start/completion semantics required by U4. A changed harness version needs
new evidence or refusal. Measure native resumed-history replay versus new calls.
For secret-bearing use, plant noncredential canaries in the selected store and
child environment, prove the positive controls readable without isolation,
then probe workspace hands and every model-native read/process surface on each
shape. Protected mode bits and a write-denial flag are not a passing result.
Record store-read, process-read, native-write and ambient-MCP results separately;
MB2 refuses secret-bearing holdings unless both read channels are excluded.
Secret-free holdings can pass other protections without this qualification.
Measurement can close rows through honest refusal;
admitting additional shapes or inventing a third mechanism requires updating
this plan before dependent implementation. #500 can supply evidence only
when its actual observations meet this matrix; its verdict is not authority.

### D3. One admission order, two kinds of refusal

Use typed SiteClass, capability classes, BindingKind and compatibility causes.
No string vocabulary or Value is carried into the new policy state (0071
rulings 2, 3, 8). Preserve the existing constitutional checks that precede
capability resolution. New ordering is:

1. Strictly load original source bytes, all consulted definitions and every
   selected-realm grant/dialect; schema/duplicate/containment/reserved-key
   faults are unconditional. Loaded-library and inline DATA lint uses verified
   charter bytes, including asks later subtracted.
2. Until U9b, return the exact existing realm-wide MCP unbuilt-kind cause
   after grant validation, even for unused or offices-empty MCP grants.
   U2 tests non-native seams directly; it adds no compile bypass.
3. Per remaining site/candidate: subtraction, grant presence, office reach,
   nonempty tool set. An unused valid grant is pinned and inactive after U9.
4. For an applicable MCP request, MB2's hard checks in order: namespace,
   workspace hands, network false, recognized model channel, proved native-tool
   write confinement protecting evidence, then measured store/child-process
   read isolation for secret-bearing dialects. Requires and wants both refuse
   on failure; secret-free holdings skip only the last check.
5. GP1 for both kinds: writes prohibition before explicit-office egress.
   Requires refuses; wants loses the whole holding with its exact notice.
6. Provider/native or MCP carriage, representable identity, connection and
   secret clearance, then empty-only restrictions. URL and argv references
   get SC1 causes. MCP nonempty restriction cause is exactly
   "MCP nonempty restrictions are deferred to the restriction-transport slice",
   through GP1's required/optional forms. Never discard only the restriction.
7. Independently prove every known unheld native power OFF and every model
   shape strictly isolated, even after drops, scope exclusions and subtraction.
   Missing measured isolation is not a compatibility drop.
8. Seal the complete outcome; establish D6's managed-writer protection before
   any writer dispatch. Recheck secret-store reach, protected executable/startup
   inputs and aggregate reservations before broker spawn. Preflight the complete
   command and repeat independent final assessment immediately before spawn.
   Runtime drift is a hard launch failure, never a late optional drop.

The adapter's former dead mcp key becomes carriage data, separate from
strictness. A provider can exclude all ambient servers but be unable to load
a granted server. That distinction permits a wants drop only if the empty
or hands-only strict plan and native OFF remain provable.

U2 changes binding access to exhaustive native/MCP/reserved-hands variants.
An absent native binding returns a typed missing-binding cause; it never
indexes, expects, invents an empty provider or treats MCP as native.
The public compile path remains fenced until all later dependencies land.

### D4. Typed contracts and version ownership

SC1–SC5 own public observables. New internal types carry Connection::Stdio
or Url, RetentionDisposition::Inherit or Veto, implementation kind and call
state. Closed serde shapes deny unknown fields. Arbitrary MCP payload/schema
JSON exists only in the bounded parsing/forwarding edge; durable authority,
ledger and journal projections are typed. Typed errors preserve exact text;
existing legacy String APIs can render a typed error at their outer adapter
without adding new Result<_, String> functions.

| Surface | Planned contract / consumer |
| --- | --- |
| Tool dialect | Reuse frozen v1 and its existing embedded copy. Retain four discarded fields, same bound bytes/digest. Valid declared argv references remain data but are unexecutable in this slice. |
| Realm | The next realms version after v7. Only false is legal for reserved retain; absent means inherit. Version-aware reserved keys keep a v6/v7 retain spelling as a dialect restriction. Include every v7 provisional-office field. |
| Manifest | v12 implementation union plus declared/disposition/effective retention per candidate. Native has declared/effective false. No transient plan path, PID, resolved secret or call ID is identity data. |
| Seat record | v6 attribution group capability/dialect/tool/call_id/call_state, optional response_sha256 on succeeded/failed only, one settled broker row (no public started), and conditional turn requirement from SC4. Old rows remain valid. |
| Private transport | Shared typed broker plan and ledger records within protocol's MCP module; consumed by CLI broker and runtime engine. No new public driver-protocol version or policy input. |

The v6 store dispatch boundary follows its existing engine-line convention:
select the development line actually on main when U4b lands, document why
its additive acceptance preserves older rows in that line, and test adjacent
lines. This does not require a version bump. Add new schema files and exact
embedded copies with their consumers in one PR; never edit old versions.
Manifest v12 has no existing embedded manifest copy to duplicate: its
production projection and contract validation are the consumer.

Restriction schemas remain validated/digested edge data, with no network
schema retrieval. Reserved-key traversal must receive the realm version;
hard-coding retain into the global old GRANT_KEYS would corrupt v6 semantics.
Changed declarations still move identity even when effective retention does
not; secret rotation does not. Old runs read honestly and cannot acquire new
MCP authority on resume.

### D5. Broker boundary and protocol

The tree is engine → driver → harness → broker → dialect server, with hands
a sibling of the broker under the harness. The engine writes one entry
cap-<capability> per selected holding beside brokkr. It seals executable,
ordered arguments, plan digest/owner, exact tool identifiers and strictness
before serialization. A separate final parser derives actual effective
configuration and compares against that intent. Equal authored bytes never
gain engine origin. No post-check mutation is allowed. Protected environment removals are also
part of the checked spawn facts, not a later unverified mutation.

The private plan includes run/effect/attempt/site/instance, server/capability,
dialect and definition digests, version, connection, admitted tools, empty
restrictions, retention disposition, store locator and binding minimum.
Only names and locators, never secrets, reach it. The broker opens only this
engine-owned inventory-bound plan; a locator by itself is no authority.
A new Cmd variant and handler follows 0071 ruling 10. Modules are ordinary
Rust modules and enums; no new trait or broker daemon.

Use the MCP 2025-06-18 stdio framing and lifecycle, whose standard describes
newline-delimited JSON-RPC and initialization/version negotiation.
See the [transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)
and [lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
contracts. MB3 deliberately admits a bounded subset, not every optional
feature. The [tools contract](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)
distinguishes JSON-RPC errors from tool results with isError; both become
failed terminal outcomes when the child returns them.

MB3 fixes literal request/response/depth/catalog/time/concurrency bounds.
Reject unsupported versions rather than echoing an arbitrary version as the
current hands server does. Initialize the real child, compare serverInfo
version with the dialect pin, enumerate every catalog page, reject repeated
cursors/duplicate names, verify every granted tool exists, then expose only
that subset. Preserve each admitted tool's schema as DATA. Dynamic catalogs
never widen authority. Do not advertise sampling, roots, prompts, resources,
elicitation or subscriptions; reject their requests, bound irrelevant
notifications by MB3's 1 MiB per-operation allowance, and implement cancellation locally with no new power.
Error responses use fixed safe causes; never echo raw child stderr/arguments.

Responses match an outstanding typed request ID, expected method and session
phase. String and numeric IDs differ; wrong, unsolicited, duplicate or late
responses take MB3's protocol cause. The private call sequence never copies a
raw ID into journal identity. Initialization/list/call deadlines are absolute;
progress/ping traffic never resets them. Timeout or cancellation ends an
uncertain session; no reconnect or replay of an external action fills a gap.

Each valid tools/call gets an internal sequence unrelated to JSON-RPC ID.
Append and fsync Started before forwarding; append/fsync Terminal after
masking/staging and before delivering. Local denials get Refused with no
child invocation or digest. Serialization uses one active call, while the
reader continues handling cancellation and bounded protocol frames.
Clock/deadline/process observations stay in the shell; tests inject clock
facts and closure seams, not mock-only traits (0071 rulings 1, 2).

The child gets a clean base environment containing a trusted executable
search path, locale and engine-created private HOME/TMPDIR, plus admitted
store bindings through the shared injector. A single consumed builder in
broker/session.rs owns both fixed startup values and their reserved-name set.
Before any store lookup or spawn, compare the bound plan's binding names with
that set; any collision, including HOME/TMPDIR, takes MB3's existing
"MCP server startup inputs are not protected from seat writes" cause. Names
are rejected even with currently benign values: secret rotation does not
change dialect identity. No later assignment may replace a fixed value.
This is broker-specific validation; U6a preserves the shared name grammar,
frozen schema and single plaintext injector. Other startup-loading environment
variables still need MB3's qualified protected arrangement; reserving two
names is not proof about arbitrary installed server code. No inherited harness credential,
LD variables or Brokkr control variables are copied. Driver spawn removes all
dialect secret names from the harness and broker inherited environments;
U0-measured authentication-name collisions take MB4's exact compatibility
refusal. The real child resolves only the operator store, never ambient values. No shell is inserted;
the operator's dialect chooses trusted installed code outside writable seat
reach. Before secret lookup, use a private cwd, resolve the executable and
bind its protected ancestry; reject repository executables, replaceable script,
config, package or plugin inputs. The operator must provide a startup arrangement
whose inputs cannot resolve through seat-writable paths, or MB3 refuses it.
An argument-string heuristic is not that proof. process.rs otherwise inherits
the operated workdir; changing only HOME leaves this hole open. Version checking
is an implementation assertion after execution, not executable authentication.
No generic interpreter analyzer, installer or server sandbox is added.

Bind R-I with separate HOME and TMPDIR cases: a protected installed fake server
would read a marker from the chosen startup directory, but each colliding
plan refuses with zero store lookups and zero child starts. Include benign
values and a rotation without changed dialect bytes. A DOCS_TOKEN control
reaches the child and observes the exact engine-private HOME/TMPDIR. U6c owns
pre-secret admission; U6f proves the final spawn; U9b exercises the integrated
path. Removing only the broker reserved-name check must independently fail
each collision; removing environment clearing proves a different property.

Decode complete bounded protocol messages before masking all text, including
object keys and known encodings in textual content. Use existing
canonical::to_bytes and sha256_bytes once on the masked result/error object;
reuse that buffer for staging and delivery data. Before serialization, reject
numbers whose exact JSON value would change under existing numeric conversion
(large integers and precise decimals need negative controls). This bounded edge
guard is not a second canonicalization implementation. Reject duplicate keys
both before and after masking; do not parse or normalize embedded documents.
Reattach the original correlation envelope only to the masked response.
Stderr is drained with bounded masked tail and the existing rolling byte
window large enough for each known encoding, before any lossy UTF-8 conversion;
no raw diagnostic reaches a log. protocol/secret.rs:572–580 explains the
stream-overlap invariant; split multibyte/encoded canaries test its reuse.
A transformed secret outside 0012's known encodings remains its documented
limit. The shared injector moves into existing protocol/secret.rs, with one production
accessor invocation in total. U6a's machine proof scans that module too and
separates the method definition from actual calls; it must reject a second
invocation there as well as in another module. Existing harness callers consume
the extracted helper immediately; no secret_spawn module or lib registration.

After ordinary string/key redaction, check bounded prepared output and its
correlation/structural fields using the existing masking primitives/encoding
catalogue. A known occurrence that cannot be removed while preserving shape
returns MB4's "MCP response cannot be safely masked". A forwarded call records
Failed with no digest and stages/delivers no unsafe bytes; safe local error
output carries only that cause. Unsafe correlation ends the session without
an unsafe frame. Neither converting a number to text nor changing its value
is permitted. This check is distinct from numeric precision validation and
runs even under a retention veto. Legacy mask_json semantics stay unchanged;
no second serializer, encoding table or plaintext accessor is introduced.

### D6. Protected evidence and ownership

Use the engine-resolved host root
HOME/.local/state/brokkr/capabilities/<repo-sha256>/<run>/<attempt>/ outside
the operated tree. The shell resolves paths; core/view receive typed facts.
Hash the canonical operated worktree path, not the common Git directory or a
model string. Use owner-rooted no-follow handles, owner-only parents and
exclusive files. The sealed broker-plan inventory pins each plan and expected
ledger, including disjoint retention shares and possible fallback slots, before
launch. Existing engine process observations establish proven never-started
cases; uncertain missing evidence refuses. Do not add another independently
editable capability inventory or Prepared/Started/Settled launch model.
Evidence survives engine restart; hands Session's delete-on-drop scratch does
not satisfy that lifetime. D5's builder-owned startup values are never resolved
from secret bindings; the host evidence root and child private directories
retain their engine-bound ownership after the actual environment is assembled. A missing/private-root mismatch parks, never rebuilds
from an untrusted worktree lookalike.

Private plan, ledger and staging roots must be outside every seat's writable
reach. Store secrecy is a separate obligation: hands must not read the store,
and native tools must have U0 proof excluding both store and child process
secret reads. The ordinary .forge/secrets.env path is unsafe when exposed by
hands; an operator can select an inaccessible --secrets-file, but relocating a
store alone does not prove native-read isolation. Native read-only mode only
proves write containment. A secret-bearing shape that cannot qualify takes
MB2's hard refusal. No whole-harness box is invented to waive that result.

Published bytes stay at .forge/artifacts/sha256/<hex>. U8a establishes a
read-only overlay after all user binds, blocks writable aliases/overlaps, pins
ancestors and rejects symlinks, nonregular/multiply-linked artifacts and root
drift. Atomic publication uses bound handles, same-filesystem exclusive temp,
fsync, no replacement and parent sync; existing matching bytes are verified
before reuse. The journal digest is appended only after verified publication.
The broker writes only private staging, never the artifact root or forge.db.

U8a2 applies this protection to ALL Brokkr-managed writers sharing that
canonical worktree, including zero-grant panel siblings, sequence/fallback
sites, boxed exec and future runs with old artifacts. The checked common
spawn_site door (engine.rs:3192 onward) covers every driver-bearing site; the
protection plan must reach composition before the final check and be checked
again there. Protect the root before the first writer, not when its first
retaining member is reached. Existing artifacts alone require protection.
A site unable to honor it takes CR2's managed-writer refusal before dispatch.

Serialize managed runs sharing that root with one exclusive host-side writer
lease held across the run's owned-process lifetime, not one lock per member.
Panel members of the same admitted run share its protection; separate worktrees
retain parallelism. All current-version runs take this lease, even with no MCP
holding/artifact yet, so a retaining run cannot race an already active writer.
Use the existing process/attempt facts to recover abandoned ownership; an
uncertain surviving writer refuses rather than treating lock release as process
settlement. No daemon or general lock service is introduced. U9 migration must
quiesce older engines that cannot participate; arbitrary operator host writes
remain outside this managed-seat guarantee. The observable serialization cost
and refusal of unsafe writer shapes are accepted for evidence integrity.

U8a/U8a2 real namespace proofs must include a retaining member plus a zero-grant
attacker, boxed exec, an already-running writer, historical artifacts, ancestor
replacement, aliases and hardlinks alongside legitimate worktree writes. These
are a distinct dispatch/coordination PR; a producer-only mount test cannot close
CR2. Read/write protections never claim a sandbox for malicious operator-selected
server binaries. #403 and 0012's transformed-secret residuals remain named.

### D7. Ledger and crash semantics

CR3 owns the literal bounds, quota accounting and lifecycle. Use closed typed
Opened, Started, Terminal and Closed variants. Every record has a contiguous
record_seq and fixed owner; calls additionally have the next call_seq and tool.
A header reference may avoid repeating the immutable owner. No arguments, raw
JSON-RPC IDs, response bodies or arbitrary child text become durable metadata.
The broker exclusively opens a plan once: reconnect refuses rather than resetting
sequence. Reserve Started/Terminal/Closed capacity before accepting a frame.
Validate request shape and bounded tool identity at the edge, then fsync Started:
that is acceptance. Tool, active-call and retention-budget denials occur after
it and have private Started/Terminal, with no child forwarding. At ledger
capacity stop reading; no fictitious checkpoint describes unread input. Failed
Started persistence blocks successful completion and forwarding; recovery uses
only the verified prefix, including a complete record whose acknowledgement was
lost. CC2 and SC4 pin these cases without clamping an invalid identity.

| Ledger state | Accepted next record | Otherwise |
| --- | --- | --- |
| Empty | Exactly one owner-bound Opened | Lifecycle refusal |
| Open | Started with next call_seq; or Closed with no unresolved calls | Lifecycle refusal |
| Calls begun | Terminal for the matching begun call/tool; additional Started only for an already decoded bounded local refusal | Lifecycle or specific owner/tool/conflict refusal |
| Closed | End of file only | Lifecycle refusal |

Opened has record_seq 1. Terminal cannot precede Started, match another owner,
or occur twice in one ledger; conflicting duplicates have their specific cause.
Closed requires every accepted call terminal and carries SessionDisposition::Clean
or Failed { cause: BrokerSessionFailure }, a closed typed safe-cause vocabulary.
Latch the first fatal initialization/version/protocol/response-limit/timeout,
unsafe-correlation or ledger-limit cause. Orderly EOF and successful harness
output cannot clear it. Ordinary child tool errors and recorded local denials
alone leave session disposition clean; their call outcomes remain unchanged.
Reserve and sync closure before normal exit. Failed persistence preserves only
the verified prefix and its existing missing/partial/lifecycle cause, without
inventing a specific I/O diagnosis during recovery. Missing Closed is a lifecycle
failure after process settlement, even if zero calls were observed. An incomplete
call still yields interrupted evidence; it cannot certify a successful session.
A partial live append is never inspected as corruption: folding is settlement-only.

Reserve retention before forwarding using CR3's 256 MiB attempt-wide shares
and 8 MiB per-call reservation. Include possible fallback slots up front; unused
shares are not dynamically redistributed, and reconnect never creates quota.
Charge actual complete bytes only after durable staging and retain the charge
through settlement, including duplicate content. Staging and publication copies
stay within 512 MiB of new response data. Share exhaustion refuses before external
work. Historical content is not evicted and quota failure never silently disables
retention. This bounded static allocation avoids another service or allocator.

Call identity is SHA-256 over an existing canonical typed tuple: native =
attempt/site/instance/provider/measured new-call identity; broker =
attempt/site/instance/server/call_seq. Prefix n- or m- separates domains.
No Debug encoding or truncation. U0 must distinguish resumed historical events
from new activity; history is not restamped as a current call. A measured
single-event native format may use a per-attempt ordinal; otherwise missing
correlation refuses. MCP holding of an abstract capability never enables the
same-named native power: only a selected native binding can do that.

Private Started is fsynced before forwarding; masked staging and Terminal are
durable before delivery. After owned-process settlement, project exactly one
public checkpoint per accepted call: succeeded, failed, refused or interrupted.
Native observed rows remain observations without completion claims. Broker
telemetry is diagnostic only. The pure view reads one outcome directly, without
reassembling public stages. The cost is delayed authoritative MCP visibility.

The engine iterates verified ledger records from disk with bounded memory at
the existing checkpoint settlement barrier. Use a fenced append returning an
actual commit result; do not send broker rows through Checkpoints::offer's
lossy/held progress queue. Before retry, compare committed call_id and full typed
payload; confirmed identical rows skip, conflicts refuse. Existing append
patiences govern contention and failed/indeterminate settlement. Durable ledger
position is not an authoritative cursor; restart reconstructs from the journal.
No terminal attempt result of any kind bypasses this barrier. It also judges
validated session dispositions, before admitting any harness result. A Failed
session overrides harness success using existing failed/indeterminate handling
and the exact recorded cause. Clean closure is necessary, not sufficient for
success. Zero-call failures affect the attempt without inventing a tool row;
no new public schema field or phase-machine input is needed.

The deterministic harness fixture must deliberately ignore broker failures and
report success for wrong version before calls, protocol failure after one
terminal call, and exhaustion after exactly 4,096 calls. Assert the respective
fixed cause and 0/1/4,096 checkpoint counts, plus healthy zero-call and ordinary
tool-error/denial controls. Removing only the engine disposition judgment must
fail them; process cleanup removal is separate evidence. Restart rechecks the
durable outcome and never repeats the external action.

| Failure window | Recovery and result admission |
| --- | --- |
| Before durable Started | No forwarding; persistence failure blocks the call and successful attempt. |
| After Started without durable Terminal | Settle processes, append one interrupted row without digest; an external effect may have happened. |
| Staging without Terminal | Orphan bytes prove no result, no inferred digest. |
| Terminal before harness delivery | Publish the child's prepared outcome, not a claim of receipt; no replay. |
| Publication before checkpoint | Verify/reuse artifact, append one absent settled row. |
| Journal commit before acknowledgement | Full payload match skips duplicate; no cursor recovery is needed. |
| Lock outlasts append patiences | Preserve disk evidence, take existing failed/indeterminate settlement, never admit clean completion. |
| Valid Closed with Failed disposition | Preserve exact safe cause and call rows; reject harness success, including zero calls; no synthetic tool checkpoint. |
| Invalid owner/order/gap/partial tail/missing ledger | Keep verified prior calls; interrupt unresolved calls only after settlement; retain the exact CR3/CR4 cause and block success. |

The same barrier covers single seats, all panel members, sequence steps,
stop/timeout/failure and engine restart. An unexpected ledger is an integrity
failure; a missing ledger differs from a proven never-started process. No
child effect is replayed to repair evidence. The engine alone writes journal
rows and the existing phase machine alone chooses the next phase.

### D8. Charter check, native attribution and inspect

GP2 owns deterministic paragraph semantics: requested names with safe-name
boundaries, normalized wrapped whitespace, exact DATA clause, no headings or
fenced code satisfying prose. One qualifying declaration paragraph per requested
capability suffices; later references pass without repeating the clause. Scan
verified bytes once for each loaded office
and inline requester. Test fences, CRLF/blank lines, repeated paragraphs and
name prefixes. Do not add an NLP classifier or claim it controls model behavior.

Normalize observations at the harness edge before the existing 80-character
display clamp. Use selected sealed holdings for attribution; native inventory
alone describes known tools but grants none. Reject ambiguous reverse mapping
at compile and unheld known calls at runtime. Engine-owned site/instance/
boundary stamps are applied together. Missing MCP telemetry fields limit
diagnostics but cannot erase the authoritative broker ledger; missing native
identity blocks a completeness claim with CC2's exact cause.

For discovery, U7c projects only this selected intent's server/tool identifiers
and measured adapter hands.notice discovery identifier into the existing
spawn-mark path. Clear stale primary/previous holdings on every selection,
including wanted drops. Do not reconstruct them from emitted argv or expose
the plan, secret-store or ledger locator. U7d parses the new typed edge once
and renders fixed guidance in the existing capability contract; the workspace
notice's contract stays unchanged. Configuration consumes the selected facts
before rendering lands. Separate selection and rendering tests/removals bind
Codex deferred discovery, fallback clearing, hostile prose and no-MCP/native
controls, all outside the requested-effect digest. No new notice registry or
parallel prompt carrier is needed (0071 rulings 3, 5, 6).

One pure brokkr-view projection exposes each settled call's attribution
and optional digest. CLI/TUI/browser can paint that common fact without
re-deriving capability ownership. No new comparison behavior is introduced.
The thin selection/printing handler stays in cli/verbs/readouts.rs beside
inspect; filesystem verification remains in runtime and attribution in view.
Native lost/stranded checkpoint failures remain enforced independently; an MCP
ledger cannot reconstruct missing native observations.
Inspect adds --capability-call <call_id> to its existing run selector; it finds
exactly one journal-owned call, selects that checkpoint's digest, and opens the
verified bytes via the runtime artifact reader. With --json it emits a
provenance envelope plus the verified canonical response; otherwise prints
provenance and those bytes. Unknown call selection says
"capability call is not recorded in this run". SC4/CR5 own absent/missing/
corrupt causes; no caller-supplied path or fresh server response is accepted.

### D9. Unit boundaries, consumers and merge safety

The last section is the only executable merge order. Each row is one PR,
not an objective with hidden follow-up edits. Shipped JSON, a new schema,
its embedded copy, module registration, extraction source and destination
all count. Tests and report-only evidence are separate. A newly factored
module is used by an existing production caller in the same PR; no unused
public staging API is added. The broker command is a consumer even while
realm compilation remains fenced; it still requires a bound engine plan.
U6b–U6e implement consumed parsing/validation and bounded private session seams,
but the public handler refuses before spawn until masking, ledger and admission
protections are complete in U6f. A directly invoked CLI cannot bypass this just
because realm compilation is fenced. Keep incomplete session seams private;
no exported “not implemented” scaffolding counts as a production consumer.

U1, U2, U3 and U4 have no dependency on one another. U1/U4 require their U0
evidence; U2/U3 do not. The listed order serializes their merges and conflicts,
not their logic. Later feature units explicitly join their outputs. U5c's
versioned grant change and U6a's injector extraction also have independent roots.

The previously adopted U5/U6/U9/U10 combinations and secret/readouts module
reuse remain. This council folds discovery selection into U7c and rendering
into U7d, which already own the necessary production files. That removes U7e
and leaves 43 PRs with all 100 task IDs and every proof retained. U8a now
depends on U7d. U5c precedes U5a; U4a/U4b and U8a/U8a2 remain separate for
their file/proof boundaries. Existing size/clone/consumer gates retain
authority; no exemption or fourth production file is implied.

New production modules stay within 800 lines; new test modules within 2,000.
Existing oversized files shrink or remain at/below their committed baseline,
with sufficient extraction included in the row before adding behavior.
No new suppression is authorized. The row cannot grow a fourth production
file just because a constructor or registry needs updating: amend the row
into another ordered PR first, retaining its task ownership and all proofs.
Likewise if a U0-qualified mechanism needs files beyond U1's inventory,
split that inventory before implementing it. No partial path is enabled
to satisfy a file limit. A strictness result that makes bundles/self
unseatable cannot waive its compile gate: return upstream for an operator
roster decision before merging U1; do not add a guessed harness exemption.

New tests use the named owning suites. Large old test files may relocate
existing cases into a test-only child file in that same suite to make room,
without production registration changes outside the row. Every actual
test-file move is listed in that unit's evidence and cannot change coverage.
Shared builders and unwind-safe environment guards are mandatory. New
integration suites are named explicitly below; no frozen fixtures change.

### D10. Validation and honest handoff

Every implementation row records requirement IDs, exact assertion/variant,
compiling behavior-removal mutation, intended failure, restored pass and
tested commit. Pin each module's refusal text once and otherwise assert typed
variants; a first failing table row proves no later row. Shared helper tests
do not replace the real compile→launch→broker→fold test at U9b.

Every row owes cargo test --workspace, self compilation, formatting, clippy
and exact coverage. Final candidate also runs locked all-feature tests,
locked all-target/all-feature clippy with -D warnings, self and verify
compiles, strict OpenSpec and diff checks. Namespace and exact coverage run
on a capable host outside the nested box; literal nonzero equality for lines,
branches and logical functions is unchanged. Linux/macOS results and remote
CI name the final head. CI/release/local coverage consume the same
rust-nightly-version.txt. Pending results are never substituted with a
previous head or a docs pass.

Witness/compose pins are remeasured in every row that changes their inputs,
with the reason appended; no invented digest and no historical channel
replacement. The shared pin files are tests, not an exception to source
budgets. Unit records update this change's tasks and later implementation
evidence; this visit does not create a fabricated proof ledger.

This visit's checks are staged/unstaged git diff --check and
openspec validate --all --strict. The operator runs cargo fmt and typos
outside the box and signs the single squash before pushing. The document
commit is plain unsigned git commit. No push, PR creation, archive or
decision acceptance is performed here.

Historical document results, all dated 2026-10-03, keep their original scope.
Each recorded both diff checks and strict OpenSpec with 20 passed, zero failed;
length/archive notices were informational. None executed U0 or runtime proof.

| Committed visit | Recorded static audit and disposition |
| --- | --- |
| 7aa9e9ff council | 26 requirements, 97 scenarios, 48 PRs, 100 open tasks; eleven-file repair; upstream for owning specification review. |
| 7f36e922 specify | 26 requirements, 100 scenarios, 48 PRs, 100 open tasks, 56 production paths; eleven-file adoption, no runtime closure. |
| b310e094 council | 26 requirements, 106 scenarios, 44 PRs, 100 open tasks, 55 production paths, 521 local links/anchors; seven-file repair; upstream for R-H owning review. |
| 1c96ca2c specify | 26 requirements, 107 scenarios, 44 PRs, 100 open tasks, 55 production paths, 521 local links/anchors; seven-file adoption of R-G/R-H. |

Current council document validation, 2026-10-03, based on 1c96ca2c: both
unstaged and cached git diff --check pass; openspec validate --all --strict
passes 20 items with zero failures and existing informational notices. The
static audit finds 26 unchanged requirements, 108 scenarios, 43 ordered PRs,
100 preserved unique open task IDs and at most three production files per PR.
All 55 production paths match Hot files ownership; task dependencies, detailed
closures and 530 local Markdown links/anchors resolve. Exactly six commissioned
Markdown artifacts change. Production/test/frozen bytes, the verbatim R1–R5
record, index row and 0065 amendment pointer remain unchanged. Final staged
checks repeat after this record; no Rust or live harness test was executed.
U0, implementation/removal proofs, external format/typos and runtime/host gates,
operator acceptance and squash signing remain pending.

### D11. Current return and analysis disposition

The supplied returned_from is clarify's clear review of the adopted change at
1c96ca2c. Preserve its R-G/R-H answers and independent proofs. Both current
positions report no new earlier-owner defect. R-I uses MB3's existing duty to
protect startup inputs before secrets, including an unprovable arrangement's
exact cause; its scenario makes the chosen construction testable. It neither
changes global secret grammar nor weakens a requirement. The discovery cut
changes delivery ownership only. No upstream return is warranted by these
facts; a future incompatible measurement still returns to its earliest owner.

| Analyze category | Judgment, owner and consequence |
| --- | --- |
| Duplication | Remove the separate discovery PR and repeated gate/history prose. U7c selects once; U7d renders; D5's single builder owns fixed values and reserved keys. Preserve shared injector/canonicalizer and pure view (0071 rulings 1, 3–7). |
| Ambiguity | MB3 now explicitly answers secret-name collisions before lookup, including benign values/rotation. Existing exact startup cause and one builder settle construction without another public vocabulary. R-G/R-H remain answered at MB3–MB5/CR2–CR4 (rulings 2, 3, 8, 9). |
| Underspecification | No new unresolved choice changes the build. D2 leaves mechanisms to U0's specified measurements, with refusal on absent proof. Runtime enforcement remains a proof obligation, not a claimed result (rulings 3, 8, 9). |
| House alignment | Keep three-production-file units, consumed extractions, native OFF, typed edges/errors, no new traits or suppressions, no copied recipe/agent and no Windows surface. Frozen versions and the next realms version after v7 remain coherent (rulings 1–10). |
| Coverage | U6c/U6f/U9b own each R-I removal independently; U7c/U7d retain selection/rendering and all discovery suites. R-G's engine check and R-H's scalar check stay independent of cleanup/precision. All 26 requirements and 100 open task IDs retain owners; D10 records the inventory audit (ruling 9). |
| Inconsistency | Current context, 43-row order, task groups 35–38 and Hot files agree. Historical 44-PR counts are explicitly dated. No scope, contract number, gate promise or enablement moved (rulings 3–6). |
| Severity / gates | A bypass of R-I, R-G/R-H, strictness or custody would be high security because it weakens a refusal (rulings 3, 8, 9). This is design prevention, not a reproduced implemented exploit. Whitespace, format, file/clone and exact-coverage findings belong to their gates and are informational (ruling 11). |

The design result is **drafted**, with inputs.change
`decision-0065-capabilities-slice-two`. This disposition accepts no decision,
closes no implementation task and selects no next phase.

### D11.1. Historical upstream returns

7aa9e9ff returned requirement gaps in secret-read/startup isolation (MB2–MB4),
all-writer custody (CR2), durable/confirmed settlement (CR3/CR4), correlation
and aggregate bounds (MB3/CR3), plus excess DATA/public-lifecycle requirements
(GP2/CC2/SC4). 7f36e922 adopted them and clarified durable acceptance and the
bounded request identity at CC2/CR3/MB3/SC4. These corrected earlier owners;
they were not downstream implementation exceptions.

b310e094 returned upstream because R-H exposed a specification conflict:
shape-preserved secret-free output had no safe outcome for the shared masker's
numeric residual. R-G also exposed the missing durable failed-session judgment.
MB4/CR2 and MB3/MB5/CR3/CR4 were repaired first, then 0077, design and tasks.
Leaving either fail-open would be high security under 0071 rulings 3, 8, 9.

### D11.2. Historical owning review at 1c96ca2c

That specify visit adopted b310e094 after checking secret.rs:637–668,
secrets.md:50–52, process.rs:499–520 and checkpoints.rs:169–248. R-H keeps
"MCP response cannot be safely masked", no unsafe delivered/staged body and
no digest for the failed forwarded call; scalar refusal is independent of
numeric precision and applies under retention veto. R-G keeps typed failed
closure and engine judgment even after a successful harness result, with
0/1/4,096-call controls and an independent engine-check removal. The result
was drafted and 0077 remained proposed. The current clear clarification
covers that adoption; neither review is an executed implementation proof.

## Risks / Trade-offs

- Harness config precedence may defeat isolation → U0 controls admission,
  and SI2 refuses affected sites even if this makes an existing recipe unusable.
- Secret-bearing native reads may be impossible to isolate → refuse that
  holding; a secret-free control may still qualify. U0 decides, not mode bits.
- Version reporting happens after code executes → require protected startup
  inputs and private cwd; the operator installation is still trusted code.
- Protected artifacts live under a writable repository ancestor → bound
  handles, read-only mount overlays, alias refusal and adversarial real
  namespace proofs for every managed writer precede enablement. Same-root runs
  serialize, and an unsafe zero-grant writer may now refuse.
- Process groups are not cgroups → reuse #403, retain #472/macOS residuals;
  namespace activation requires real Linux cleanup evidence.
- Exact recording costs a sync before each external call → pay it, with
  bounded serial calls and explicit failure; no asynchronous unsafe queue.
- Response persistence and receipt cannot commit atomically → distinguish
  child outcome from receipt, and never replay an uncertain external effect.
- Authoritative broker calls appear only at settlement → retain durable private
  evidence and diagnostics without a second public lifecycle.
- Fixed retention shares can refuse a busy broker while another share is idle →
  accept this predictable bound rather than redistribute authority during a call.
- Shape-preserving masking can refuse a valid JSON response → accept the
  bounded refusal instead of exposing a scalar secret or changing evidence.
- Harness success can hide child failure → persist and judge session outcome;
  process cleanup alone proves neither healthy closure nor successful calls.
- Many source seams are oversized → extract with their consumers first;
  each split is budgeted rather than hidden in one broad wiring PR.

## Migration Plan

Operator rules on this proposed change and 0077 first. Every listed PR then
starts from current main, updates changed path/number facts, passes its gates,
is signed and lands through the merge queue. No long-lived implementation
branch is assembled by this document run.

U1 can intentionally refuse previously ambient-configured seats; guides and
doctor must not call unmeasured configurations safe. U3 narrows gate native
holdings, so realms that intend reviewer egress list its office explicitly.
The repository's own grants remain empty. U4 adds record acceptance/attribution
without rewriting history. U5 adds the next realms version after v7 and v12
identity, without editing frozen contracts. U9b alone enables admitted v1
stdio grants with empty restrictions. Before that migration, quiesce older
Brokkr engines writing the same worktree; their launches cannot honor the new
managed-writer lease. New runs protect existing artifacts even without grants.

Rollback before U9 simply leaves the MCP fence. After U9, an emergency
rollback restores the compile refusal and refuses new starts; it does not
erase ledgers/artifacts, reinterpret an old manifest, replay external calls
or revert native strictness/gate protections. Existing in-flight operations
are settled through the same journal/cleanup path, not left detached.

## Open Questions

The supplied clarify result is clear, and D11 retains the adopted R-G/R-H
answers. R-I's existing startup duty now has a concrete scenario and proof
owners; operator acceptance remains required. The technical
unknowns are measurements and external results: which of D2's named candidates
qualifies each installed harness/shape; exact observed call identifiers; and
host/CI evidence on each final unit head. Their failure outcome is already
specified as refusal/pending. No unmeasured flag, config-precedence choice,
decision acceptance, protection exemption or new boundary is assumed.

## Hot files

The unit table below is the authoritative complete path inventory. The following
collision map is for concurrent main work; new modules are explicitly included.
At each PR recheck these paths and baseline counts against main.

| File / baseline lines where oversized | Units | Coordination |
| --- | --- | --- |
| `crates/brokkr-protocol/src/native_controls.rs` (4539) | U1a, U1g, U7a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-protocol/src/native_controls/mcp.rs` | U1a, U1c, U6c, U6d, U7a, U7d, U8a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/agents.rs` (1671) | U1b, U1f, U3b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/load.rs` (1334) | U1b, U3b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/mcp.rs` | U1b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters.rs` (7268) | U1c, U4c, U6a, U7d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters/mcp.rs` | U1c, U1g, U7d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/claude.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/codex.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/lanetally.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/dsh.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/exec.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle.rs` (7815) | U1f, U3a, U3c, U4d | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/bundle/mcp.rs` | U1f, U7b, U9b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/engine.rs` (4862) | U1g, U4e, U7c, U8a2, U8c, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/capabilities.rs` (2369) | U2, U3a, U4d, U5a, U5f, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/binding.rs` | U2, U5a, U9a, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-cli/src/doctor.rs` (1567) | U2, U9a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/gates.rs` | U3a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/agents/charter_data.rs` | U3b, U3c | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle/charters.rs` | U3c | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-store/src/seat_record.rs` (880) | U4a, U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat_record/validation.rs` | U4a | v6 reserved for this slice; preserve append/export/verify dispatch |
| `contracts/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-protocol/src/adapters/capability_calls.rs` | U4c | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/lib.rs` | U4c | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/attribution.rs` | U4d | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/engine/capability_calls.rs` | U4e, U4f, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/checkpoints.rs` | U4e, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/sequence.rs` | U4f, U8a2, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/resume.rs` | U4f | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-view/src/lib.rs` (2767) | U4g, U8f | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-view/src/capability_calls.rs` | U4g, U8f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/dialect.rs` | U5a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-core/src/realms.rs` (902) | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `crates/brokkr-core/src/realms/grants.rs` | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/realms.v<N>.schema.json` | U5c | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/run-manifest.v12.schema.json` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/manifest.rs` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-protocol/src/secret.rs` | U6a | 0012 single-injector proof; no second plaintext accessor |
| `crates/brokkr-cli/src/cli_args.rs` | U6b, U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/lib.rs` (1918) | U6b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker.rs` | U6b, U6c, U6e | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/session.rs` | U6c, U6d, U6e, U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs` | U6d, U8b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker/rpc.rs` | U6e, U6f | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/output.rs` | U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/hands.rs` (1276) | U7a, U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/broker.rs` | U7c, U8a2, U8c, U8d, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/marks.rs` | U7c | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-protocol/src/hands/evidence.rs` | U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/artifacts.rs` | U8c | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-cli/src/verbs/readouts.rs` | U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/doctor/capabilities.rs` | U9a | 0065 follow-up refactors; identity and gate ordering must survive |
Also coordinate `crates/brokkr-runtime/tests/witness_digests.rs`,
`crates/brokkr-runtime/src/bundle/compose_tests.rs` (every identity-changing
row), `crates/brokkr-runtime/tests/frozen_contracts.rs` (U4b/U5c/U5f/U10a),
`crates/brokkr-cli/tests/machine_proof.rs` (U6a), the exact owning suites
listed with each unit, and `contracts/README.md` plus U9b's guides.
The current `store/lib.rs` append fence, `protocol/process/tree.rs` and `cli/render.rs` are review seams with no
planned production edits: shared consumers suffice. If they actually need
editing, budget a new split before touching them. Their invariants remain
covered by store, secret, cleanup and readout proofs.

The current document-only collision set is this change's proposal/deltas/design/
tasks and `docs/decisions/0077-the-capability-broker-is-a-harness-child.md`.
The already adopted index row in `docs/decisions/README.md` and reciprocal
pointer in `docs/decisions/0065-capabilities-are-the-realms-to-grant.md` remain
part of the commission and must be preserved. R1–R5 remain verbatim.

## Slice two units

One row is one PR, from main, signed and through the merge queue after the
operator rules. Follow this order. Dependencies are semantic prerequisites,
not simply the preceding row; the independence of U1–U4 is explicit in D9.
No row through U9a lifts the MCP compile fence. U9b is the sole enabling PR.
The lettered splits are required by the checked seams: record schema plus
embedded copy already consumes two files; multi-server wiring crosses runtime,
protocol and adapter data; storage/folding/inspect crosses CLI, protocol,
runtime and pure view. Minimal ledger writing is deliberately pulled into U6.

`contracts/realms.v<N>.schema.json` denotes one exact future file allocated
as **the next realms version after v7** at U5c, including v7's fields. It is
the only intentionally deferred filename number. Every other production path
below is literal. New paths are proposed modules/schemas, not files created
in this docs visit. Constructors/registration are counted in their row.
Test files new to this plan include core/tests/realms.rs, CLI capability_broker
and capability_artifacts, and runtime capability_ledger/capability_broker_launch.

Each row closes exactly its numbered tasks, including tests/mutations and
gate evidence. Test lists below are the allowed owning suites; every row
also updates this change's tasks and later implementation evidence. Measured
witness/compose pins accompany only rows that change their inputs.

| PR | Dependencies | Objective and task IDs | Production files (maximum three) |
| --- | --- | --- | --- |
| U0 | Independent | Measure isolation and telemetry; 1.1–1.2 | None |
| U1a | U0 | Extract existing MCP transport checks; 2.1–2.2 | `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U1b | U1a | Type adapter MCP facts; 3.1–3.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load.rs`; `crates/brokkr-runtime/src/agents/mcp.rs` |
| U1c | U1b | Build isolated serving configurations; 4.1–4.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U1d | U1c | Record Claude, Codex and LaneTally declarations; 5.1–5.2 | `adapters/claude.json`; `adapters/codex.json`; `adapters/lanetally.json` |
| U1e | U1d | Record dsh and exec declarations; 6.1–6.2 | `adapters/dsh.json`; `adapters/exec.json` |
| U1f | U1e | Thread independent strict intent; 7.1–7.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/bundle.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U1g | U1f | Seal and enforce every launch; 8.1–8.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs` |
| U2 | Independent | Remove both native-binding panics; 9.1–9.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-cli/src/doctor.rs` |
| U3a | Independent | Apply gate classes; 10.1–10.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/gates.rs`; `crates/brokkr-runtime/src/bundle.rs` |
| U3b | U3a | Check loaded office charters; 11.1–11.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/load.rs`; `crates/brokkr-runtime/src/agents/charter_data.rs` |
| U3c | U3b | Check inline requester charters; 12.1–12.2 | `crates/brokkr-runtime/src/bundle.rs`; `crates/brokkr-runtime/src/bundle/charters.rs`; `crates/brokkr-runtime/src/agents/charter_data.rs` |
| U4a | U0 | Make room for additive record validation; 13.1–13.2 | `crates/brokkr-store/src/seat_record.rs`; `crates/brokkr-store/src/seat_record/validation.rs` |
| U4b | U4a | Publish and consume seat-record v6; 14.1–14.2 | `contracts/seat-record.v6.schema.json`; `crates/brokkr-store/src/seat-record.v6.schema.json`; `crates/brokkr-store/src/seat_record.rs` |
| U4c | U4b | Normalize harness call observations; 15.1–15.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/capability_calls.rs`; `crates/brokkr-protocol/src/lib.rs` |
| U4d | U4c | Bind attribution to compiled holdings; 16.1–16.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/attribution.rs`; `crates/brokkr-runtime/src/bundle.rs` |
| U4e | U4d | Stamp single and panel calls; 17.1–17.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U4f | U4e | Bind sequence and resumed observations; 18.1–18.2 | `crates/brokkr-runtime/src/engine/sequence.rs`; `crates/brokkr-runtime/src/engine/resume.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs` |
| U4g | U4f | Derive call evidence once; 19.1–19.2 | `crates/brokkr-view/src/lib.rs`; `crates/brokkr-view/src/capability_calls.rs` |
| U5c | Independent | Extract grants and mint the versioned veto; 22.1–23.2 | `crates/brokkr-core/src/realms.rs`; `crates/brokkr-core/src/realms/grants.rs`; `contracts/realms.v<N>.schema.json` |
| U5a | U2, U5c | Extract typed dialect policy and bind retention; 20.1–21.2, 24.1–24.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/dialect.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U5f | U5a | Publish manifest v12 with its live native consumer; 25.1–25.2 | `contracts/run-manifest.v12.schema.json`; `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/manifest.rs` |
| U6a | Independent | Share the one secret injector; 26.1–26.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/secret.rs` |
| U6b | U5f, U6a | Introduce the broker command as a closed handler; 27.1–27.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/lib.rs`; `crates/brokkr-cli/src/broker.rs` |
| U6c | U6b, U1a | Define and consume the bound plan; 28.1–28.2 | `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6d | U6c | Establish durable ledger records before calls; 29.1–29.2 | `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6e | U6d | Serve the filtered protocol; 30.1–30.2 | `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/rpc.rs` |
| U6f | U6e | Mask output and prove complete session cleanup; 31.1–32.2 | `crates/brokkr-cli/src/broker/rpc.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/output.rs` |
| U7a | U1g, U5f, U6f | Represent the complete server set; 33.1–33.2 | `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-protocol/src/hands.rs` |
| U7b | U7a | Consume adapter carriage and selected holdings; 34.1–34.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/mcp.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U7c | U7b | Provision protected per-attempt plans; 35.1–35.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/marks.rs` |
| U7d | U7c | Deliver checked configurations and selected discovery; 36.1–37.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8a | U7d | Protect artifact paths in workspace hands; 38.1–38.2 | `crates/brokkr-protocol/src/hands.rs`; `crates/brokkr-protocol/src/hands/evidence.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8a2 | U8a | Protect every managed writer before dispatch; 38.3–38.4 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/sequence.rs` |
| U8b | U8a2 | Stage retained responses before delivery; 39.1–39.2 | `crates/brokkr-cli/src/broker/output.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs` |
| U8c | U8b | Publish verified content-addressed artifacts; 40.1–40.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/artifacts.rs` |
| U8d | U8c, U4g | Fold settled calls through confirmed append; 41.1–41.2 | `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U8e | U8d | Settle and recover every attempt's evidence; 42.1–42.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/sequence.rs`; `crates/brokkr-runtime/src/engine/broker.rs` |
| U8f | U8e | Expose retained evidence in the pure view; 43.1–43.2 | `crates/brokkr-view/src/capability_calls.rs`; `crates/brokkr-view/src/lib.rs` |
| U8g | U8f | Open a cited artifact through inspect; 44.1–44.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/verbs/readouts.rs` |
| U9a | U8g, U2, U3c | Prepare whole-plan MCP doctor reporting; 45.1–45.2 | `crates/brokkr-cli/src/doctor.rs`; `crates/brokkr-cli/src/doctor/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U9b | U9a | Enable the proved namespace path with guides; 46.1–47.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U10a | U9b | Audit removals, then validate and hand off; 48.1–49.2 | None |

### U0 — Measure isolation and telemetry

Execute D2's per-harness ambient sentinels, strict config, Codex discovery/event and dsh loading matrix, including native historical-replay detection and separate store/process canary read-isolation controls. Record adapter evidence only; no code changes.

Closes tasks 1.1 and 1.2; requirements [MB2](specs/mcp-capability-broker/spec.md), [SI1](specs/strict-mcp-isolation/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: Positive controls and each cold/resume/replacement shape are reproducible; missing read isolation refuses secret-bearing holdings, while secret-free eligibility is assessed separately.

Evidence/documents only; do not add behavior-mirroring tests for this row.

Documents/evidence: `docs/evidence/adapters/slice-two-mcp-isolation.md`, `docs/evidence/adapters/slice-two-mcp-observations.json`.

### U1a — Extract existing MCP transport checks

Move the current single-server transport/parser checks into the named module, retaining production callers and exact behavior; create room under the existing file baseline.

Closes tasks 2.1 and 2.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Unchanged hands-only exact-state and authored-option tests, with no added server acceptance.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U1b — Type adapter MCP facts

Extract McpSupport and its loader into agents/mcp.rs; type measured/unsupported/unmeasured ambient isolation, native-write confinement and store/process read isolation separately by invocation shape. Consume at load; legacy server maps grant no authority.

Closes tasks 3.1 and 3.2; requirements [MB2](specs/mcp-capability-broker/spec.md), [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Closed decoding, absent evidence and wrapper-specific results have exact variants; a read-only flag never supplies a secret-read proof.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1c — Build isolated serving configurations

Factor existing config assembly into adapters/mcp.rs and implement only U0-qualified isolation shapes. Typed engine input crosses the private serving edge; the current no-broker plan is empty or hands-only.

Closes tasks 4.1 and 4.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md).
Proof: Exact cold/resume/replacement configuration, auth/session controls and missing evidence refusals; the module is used by existing launch builders.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U1d — Record Claude, Codex and LaneTally declarations

Populate declarations from U0 with exact evidence scope; unsupported/unmeasured is a valid outcome, never guessed support.

Closes tasks 5.1 and 5.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Adapter-load and whole-file identity tests pin only observed facts.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1e — Record dsh and exec declarations

Record dsh's separate exclusion/carriage verdict and exec's inapplicable model surface. No plugin or harness is added.

Closes tasks 6.1 and 6.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: No inheritance from Claude, no fabricated dsh hands support, and exec remains a script path.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1f — Thread independent strict intent

Extract capability-relevant candidate composition from bundle into bundle/mcp.rs. Carry strict empty/hands intent through candidates and inline site facts independently of emitted config.

Closes tasks 7.1 and 7.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md).
Proof: Primary/fallback and nested sites retain distinct intended sets; equality of authored bytes never supplies origin.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U1g — Seal and enforce every launch

Bind U1f facts at dispatch and consume the final checked isolated configuration at all serving builders. Mandatory strict admission is activated with this complete path, including no-ask sites. Shrink engine composition by using existing extracted helpers.

Closes tasks 8.1 and 8.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: All SI2 shapes and final isolation removal fail exactly; every shipped compile either passes measured support or reports its exact unmeasured refusal, never a filename exemption.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U2 — Remove both native-binding panics

Extract kind-specific binding projection and replace the indexed native binding and doctor expect with exhaustive typed outcomes. Native behavior and the public MCP fence stay unchanged.

Closes tasks 9.1 and 9.2; requirements [SC5](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Direct non-native seam fixtures prove both panic sites fixed; native controls and the old compile refusal are exact.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`, `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U3a — Apply gate classes

Thread canonical executable SeatClass and stable office once; extract the common pure class check. Writes precedes egress; D4 scope and independent native OFF remain.

Closes tasks 10.1 and 10.2; requirements [GP1](specs/gate-capability-policy/spec.md).
Proof: Exact native reads/writes/egress requires/wants, all nested sites, subtraction and fallback tests; helper-level MCP cases do not bypass compile.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U3b — Check loaded office charters

Apply the shared deterministic DATA checker to verified loaded-office charters. Each requested capability needs one qualifying declaration paragraph; later references need no repeated clause, including for dropped or subtracted asks.

Closes tasks 11.1 and 11.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Existing multi-capability researcher paragraph and later references pass; missing/deferred/fenced clauses and prefix collisions fail with the exact owning capability.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U3c — Check inline requester charters

Reuse the same DATA checker for verified inline requesters and all executable site forms; do not duplicate the paragraph grammar or derive permission from the reminder.

Closes tasks 12.1 and 12.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Inline declaration and later-reference positives, missing-clause negatives, verified pin drift and every nested site shape bind the same checker.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U4a — Make room for additive record validation

Extract existing validation functions into a consumed child module; preserve dispatch and export/verify behavior.

Closes tasks 13.1 and 13.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Historical version and exact refusal tests stay green; the oversized parent shrinks.

Owning tests: `crates/brokkr-store/src/tests.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

### U4b — Publish and consume seat-record v6

Add the public/embedded v6 schemas and consume them in version dispatch. Admit one native observed or broker settled attribution group; public started is invalid. Preserve old-shaped rows and conditional broker turn absence.

Closes tasks 14.1 and 14.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Exact old/new boundary cases, full group dependencies, digest-state restrictions, no invented turn and explicit started rejection; embedded bytes match their source.

Owning tests: `crates/brokkr-store/src/tests.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Documents/evidence: `contracts/README.md`.

### U4c — Normalize harness call observations

Extract telemetry normalization at the harness edge; parse measured call identifiers/server/tool fields and deduplicate native start/completion without carrying raw Value state. Carry typed observations through the existing opaque driver data edge.

Closes tasks 15.1 and 15.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Claude/Codex/dsh measured format fixtures, duplicate and missing identity controls; unrelated ordinary checkpoints unchanged.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4d — Bind attribution to compiled holdings

Build a typed reverse attribution index from each selected native holding and adapter inventory; compile-refuse ambiguous or unrepresentable names. Extract existing projection logic to keep parents below baseline.

Closes tasks 16.1 and 16.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Exact selected dialect/tool, long-name and ambiguous-map cases, no substring matching or inventory-only grant.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U4e — Stamp single and panel calls

Extract common checkpoint attribution beside current boundary/site stamps. Erase untrusted authority fields, assign attempt-owned native IDs and keep local calls unattributed; fail known-unheld observations.

Closes tasks 17.1 and 17.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md).
Proof: Native compile-to-journal proofs for ordinary, inline, fallback and panel calls; spoofed stamps cannot survive.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4f — Bind sequence and resumed observations

Apply selected-site attribution to sequence and resumed observations, using U0's measured new-call identities and ignoring replayed historical activity. Never borrow another fallback holding.

Closes tasks 18.1 and 18.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Fresh repeated calls stay distinct, historical replay makes no new use, start/completion deduplicate, and changed/missing native identity refuses rather than guessing.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4g — Derive call evidence once

Consume a pure call projection in existing view construction, representing native observations and the new settled broker states without stage grouping or grant lookup.

Closes tasks 19.1 and 19.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Exact observed/succeeded/failed/refused/interrupted values and honest historical absence; no view I/O, clock or authority decision.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U5c — Extract grants and mint the versioned veto

Combine former U5c/U5d within their three-file union. Extract version-aware grant parsing and reserve retain false only in the next realms version after v7, allocated on main in this PR. Preserve all #487 fields, old restriction meanings and existing error text.

Closes tasks 22.1, 22.2, 23.1 and 23.2; requirements [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md).
Proof: Old/new exact round trips, omission/false and invalid veto values, provisional-office compatibility and unchanged frozen pins.

Owning tests: `crates/brokkr-core/tests/realms.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Documents/evidence: `contracts/README.md`.

### U5a — Extract typed dialect policy and bind retention

Combine former U5a/U5b within their three-file union. Extract the consumed dialect loader while retaining typed v1 connection, version, secret names, retained, egress and sends. Preserve frozen acceptance and the MCP compile fence; URL/argv-reference execution receives only the specified compatibility causes after enablement.

Closes tasks 20.1, 20.2, 21.1 and 21.2; requirements [SC1](specs/slice-two-contracts/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Exact retained fields/digests, duplicate/schema/containment parity, no store/process effects and unchanged native/fenced behavior.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

Make dialect restriction-reservation checks use the grant's version, carry inherit/veto into the typed holding and preserve D11. Bound identifiers and egress minimum without granting secret clearance.

Closes tasks 24.1 and 24.2; requirements [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md), [MB2](specs/mcp-capability-broker/spec.md).
Proof: Four retention outcomes, legacy retain as restriction, reserved collisions through refs/composition and unchanged inactive grants.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U5f — Publish manifest v12 with its live native consumer

Extract manifest projections and emit v12 native implementation/retention plus the typed MCP shape while MCP still refuses compilation. Keep every consulted and inactive grant identity fact.

Closes tasks 25.1 and 25.2; requirements [SC3](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md).
Proof: Real native compiles validate v12; internal MCP projection is typed; independent identity changes and old manifest reads are exact.

Owning tests: `crates/brokkr-runtime/tests/frozen_contracts.rs`, `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

Documents/evidence: `contracts/README.md`.

### U6a — Share the one secret injector

Move bind_environment into existing protocol/secret.rs with a narrow typed error; existing harness spawns consume it immediately and location comments follow it. Keep exactly one expose_for_spawn production invocation, counting secret.rs too. No new module or lib registration.

Closes tasks 26.1 and 26.2; requirements [MB4](specs/mcp-capability-broker/spec.md).
Proof: The machine proof counts actual accessor calls across all production modules including secret.rs, distinguishes the method definition, and asserts the one injector location. Adding a second call inside secret.rs and separately outside it must fail; existing leak scans and safe diagnostic text remain bound. No environment fallback.

Owning tests: `crates/brokkr-cli/tests/machine_proof.rs`, `crates/brokkr-protocol/src/adapters/tests.rs`.

### U6b — Introduce the broker command as a closed handler

Add Cmd plus handler for broker serve with bounded engine-plan locator/digest arguments. An unbound manual invocation refuses; no raw server argv, grants or secret values are CLI options. Move dispatch code out of the oversized CLI parent.

Closes tasks 27.1 and 27.2; requirements [SD3](specs/slice-two-delivery/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md).
Proof: Exact CLI parsing, bound-plan refusal and unchanged commands; compile still refuses MCP.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c — Define and consume the bound plan

Define and consume the identity-bound BrokerPlan with owner-rooted reads, protected executable/startup inputs, private cwd, sanitized environment and shared injector. D5's consumed builder owns fixed startup values and reserved names; reject binding collisions before any store lookup or spawn, with no global grammar change. Keep the public path before spawn closed until U6f completes its protections; no exposed unconsumed helper.

Closes tasks 28.1 and 28.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md).
Proof: Protected installed fake positive; repository executable, replaceable script/config/plugin/ancestor, missing/denied/store-only bindings and direct unsafe invocation refuse before secret disclosure or spawn. Separate HOME/TMPDIR collisions, benign values and rotation refuse with MB3's exact cause, zero lookups and zero starts; DOCS_TOKEN preserves the exact private directories. Each collision binds removal of the broker check independently of ambient clearing.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6d — Establish durable ledger records before calls

Define and consume the shared closed ledger variants and durable writer, with exclusive single plan lifetime, contiguous record/call sequences and reserved terminal/closure capacity. Closed carries Clean or Failed with a latched typed safe cause, synced before normal exit; EOF cannot clear it. Local refusals also have private Started/Terminal.

Closes tasks 29.1 and 29.2; requirements [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Missing/duplicate Opened, Terminal-before-Started, post-Closed records, missing closure, restart of the same plan and each write/sync failure take exact variants; no journal writer. At 4,096 calls the next frame stays unread and closure carries the ledger-limit failure. Wrong-version zero-call and fatal-protocol after-call sessions retain their first cause; healthy zero-call and recoverable call errors may close Clean. Closure write/sync failure preserves verified-prefix recovery, never invented success. Durable local refusals each have one Started/Terminal pair; failed Started persistence admits neither forwarding nor successful completion.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6e — Serve the filtered protocol

Implement the bounded filtered protocol with private Started/Terminal records. Correlate typed response IDs to method/session state, fix absolute deadlines and bound non-response traffic. Fatal protocol/init/version/limit/timeout outcomes latch CR3 session failure; ordinary tool errors and local denials remain call outcomes. Public serving remains closed until U6f; no unsafe intermediate proxy.

Closes tasks 30.1 and 30.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Allow/deny/catalog/version/method controls; wrong-type/late/duplicate/phase IDs, endless progress, cancellation, concurrent calls and persistence failures never forward an unrecorded or uncertain retry. Invalid request shapes/vocabulary and 257-byte names refuse before acceptance; a valid 256-byte ungranted name records one exact denial without truncation.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6f — Mask output and prove complete session cleanup

Complete the serving protections in session.rs using existing secret masking and canonical byte/hash functions at the edge. Share one masked buffer, reject duplicate keys and numeric value changes, and independently refuse unsafe scalar/structural secret occurrences before staging or delivery with MB4's exact cause. Keep legacy masker semantics and shared encodings. Drain stderr with raw-byte overlap before lossy decoding. Only then can the bound public session serve; retained plans still refuse until U8b.

Closes tasks 31.1 and 31.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md).
Proof: Literal/encoded/split/multibyte leak scans, digits-only scalar refusal versus text-redaction/unrelated-number controls, masking-created key collisions, and unsafe-correlation failure without raw frames. Assert failed forwarded call with no digest and no unsafe body for retention on/off/veto; the refusal itself passes leak scans. Remove the scalar check independently of numeric-precision validation; direct command cannot bypass plan/ledger/startup/masking protections, and a retained plan never silently degrades. Repeat R-I's independent HOME/TMPDIR collision removals at the final public spawn boundary, including zero lookup/start and normal-binding/private-directory controls.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

Exercise existing #403 process ownership with fake servers that block, fail init and die mid-call. Make no production tree rewrite.

Closes tasks 32.1 and 32.2; requirements [MB5](specs/mcp-capability-broker/spec.md), [SD4](specs/slice-two-delivery/spec.md).
Proof: Linux real-process positive and cancellation/timeout tests; report existing subreaper/cgroup residuals without claiming them fixed.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-runtime/src/engine/cleanup_tests.rs`.

### U7a — Represent the complete server set

Move hands config helpers into the common typed MCP builder and replace singleton transport intent with exact named server intent. Parse final config independently; preserve authored provenance and native OFF.

Closes tasks 33.1 and 33.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Zero/hands/three-server exact positives and independent missing/extra/changed/counterfeit negatives.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7b — Consume adapter carriage and selected holdings

Read the formerly dead mcp support at each selected candidate and materialize the engine-owned empty/hands/broker contributions from typed holdings; unsupported carriage keeps requires/wants semantics.

Closes tasks 34.1 and 34.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: No union of fallbacks, dead server maps give no authority, unrepresentable cap names refuse.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U7c — Provision protected per-attempt plans

Provision the single sealed plan inventory and private roots before launch, with complete owner identity and disjoint fixed retention shares including permitted fallback slots. Reuse existing launch facts; do not add a duplicate capability inventory or launch state machine. In marks.rs project and clear selected server/tool/discovery identifiers from the same intent consumed by configuration, using existing adapter hands.notice facts. Carry no plan or storage locator into prompt rendering; parse new edge data once into types.

Closes tasks 35.1 and 35.2; requirements [SD3](specs/slice-two-delivery/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [CR3](specs/capability-response-retention/spec.md).
Proof: Substitution/missing inventory/pre-start uncertainty refuse exactly; simultaneous slots cannot share quota, oversized allocation refuses before launch, and resume cannot create fresh budget. Bind selected-fact construction and clearing independently: fallback and wanted drop remove stale facts, original requested digest and native/no-MCP behavior stay unchanged, and prompt facts contain no secret-store/ledger locator.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7d — Deliver checked configurations and selected discovery

Wire the expanded server set and dialect-secret environment removals into each U0-supported builder and final consumption point; unsupported measured carriers refuse. No modifications after checked command creation. Render fixed capability/tool discovery guidance from U7c's typed selected identifiers in the existing capability contract, reusing the measured hands.notice discovery identifier. No provider-name branch, new notice registry or parallel authority.

Closes tasks 36.1, 36.2, 37.1 and 37.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md), [SD3](specs/slice-two-delivery/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Actual composed cold/resume/replacement commands match independent literal server intent, and mutations refuse at the final serving boundary. Separately bind rendering and selection for Codex deferred discovery, fallback clearing, wanted drop and hostile prose; preserve hands notice, native/no-MCP prompts and requested-effect digest. No complete plan or storage locator reaches the renderer.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U8a — Protect artifact paths in workspace hands

Carry engine-only protected storage facts, overlay the artifacts directory read-only after writable worktree binds, and reject every conflicting writable/overlay alias by canonical owner identity. No new authored HandsSpec key.

Closes tasks 38.1 and 38.2; requirements [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md).
Proof: Real namespace writes/ancestor renames/aliases/overlap controls cannot change evidence; mount failure refuses, never omits the protection.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U8a2 — Protect every managed writer before dispatch

Carry the protected root to every writer before composition and recheck at the common dispatch door. Hold one exclusive canonical-worktree writer lease per run through owned-process settlement, including no-grant runs; protect historical artifacts and refuse unsafe or uncertain concurrent writers. No new authored protection key or daemon.

Closes tasks 38.3 and 38.4; requirements [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md).
Proof: A retaining panel member plus zero-grant attacker and boxed exec cannot replace root/digest; an already-running writer refuses, abandoned ownership cannot outlive cleanup, later no-grant runs preserve old evidence, and separate worktrees remain independent.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8b — Stage retained responses before delivery

Stage only opted-in masked bytes under the sealed disjoint attempt share; reserve 8 MiB before forwarding, charge actual durable bytes and retain completed charges through settlement. Fsync staged content before Terminal/delivery; veto never writes a body. MB4's unsafe-output refusal occurs before staging, regardless of retention disposition.

Closes tasks 39.1 and 39.2; requirements [CR1](specs/capability-response-retention/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Four retention outcomes, multi-broker/fallback exhaustion and exact-bound controls, no same-digest quota reset, stage/sync/terminal/delivery failures, and no silent metadata downgrade; an unsafe scalar response yields the exact failed call without staged/published bytes or digest.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U8c — Publish verified content-addressed artifacts

Use owner-rooted no-follow reads, verify staged bytes and atomically publish immutable digest paths under the operated repository; retain handles across checks.

Closes tasks 40.1 and 40.2; requirements [CR2](specs/capability-response-retention/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Existing-good reuse, collision/mismatch, nonregular/symlink/hardlink and concurrent replacement controls, exact missing/corrupt causes.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8d — Fold settled calls through confirmed append

After process settlement, validate the private lifecycle and project one checkpoint per accepted call through commit-confirmed fenced append. Stream from disk with bounded memory; the committed journal and full call payload determine deduplication. Bypass the lossy held-event queue. Validate Closed disposition independently of call completeness; a latched Failed cause overrides harness success without a synthetic call.

Closes tasks 41.1 and 41.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md).
Proof: Journal lock across repeated attempts, exhaustion, commit-before-ack crash, duplicate/conflict/gap/partial/missing/unexpected/lifecycle faults, and interrupted calls each have exact counts/outcomes and no replay. A capacity-ending ledger yields exactly 4,096 checkpoints and no invented 4,097th; tool/active-call/budget refusals each yield one refused checkpoint. Wrong-version, post-call fatal protocol and capacity-ending sessions preserve their exact failed-attempt causes despite complete ledgers; healthy zero-call and recoverable-error controls can succeed.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8e — Settle and recover every attempt's evidence

Fold only after owned processes settle and before every ordinary/panel/step terminal result, including failed, cancelled, timed-out and engine-restart paths. Reuse existing append patiences and failed/indeterminate transitions; preserve disk evidence on failure.

Closes tasks 42.1 and 42.2; requirements [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [MB5](specs/mcp-capability-broker/spec.md).
Proof: Live half-record causes no premature corruption finding; settled half-record fails exactly, every terminal route meets the barrier, and restart appends each call once without replaying external work. A deterministic harness deliberately reports success after fatal broker failure; assert CR4's exact 0/1/4,096 call counts and causes, and catch an independent compiling removal of the engine disposition check. Preserve native lost/stranded failure handling too.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-runtime/src/engine/cleanup_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`.

### U8f — Expose retained evidence in the pure view

Extend the consumed pure call projection with the settled checkpoint's digest/provenance, without stage assembly, path reads or derived grants.

Closes tasks 43.1 and 43.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Exact historical-unrecorded/no-digest/recorded distinctions and shared display values for every settled outcome; no inferred retention veto.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U8g — Open a cited artifact through inspect

Add inspect --capability-call <call_id>; read the selected run's derived call, use shared runtime artifact reader and emit verified bytes/provenance. Keep the thin selection/printing helper in existing readouts.rs beside inspect; runtime owns file verification and view owns provenance, with no new capability_artifact module.

Closes tasks 44.1 and 44.2; requirements [CR5](specs/capability-response-retention/spec.md).
Proof: Complete exact bytes and missing/no-digest/corrupt/path/foreign-run refusal tests; no refetch and no view I/O.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U9a — Prepare whole-plan MCP doctor reporting

Extract capability reporting into the named module and use the shared complete planner for native/MCP metadata; preserve pre-U9 public compile refusal until U9b.

Closes tasks 45.1 and 45.2; requirements [SC5](specs/slice-two-contracts/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Grant versus holding and retention facts with static scope, native denial and exact no-spawn assertions.

Owning tests: `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U9b — Enable the proved namespace path with guides

Lift only the global MCP compile fence after all prior proofs, activating D3's namespace, gate, strictness, carriage, secret-read, startup, evidence and D11 rules. Quiesce older same-worktree engines before enabling managed-writer coordination.

Closes tasks 46.1 and 46.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB2](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Real compile/launch/broker/fold/inspect with fake dialect; same-name MCP holding keeps native power OFF, zero-grant sibling and exec cannot alter evidence, unsafe secret reads refuse, and quota/recovery/cold/fallback/member/step cases bind. Include the success-reporting harness after zero-call version failure, post-call fatal protocol and 4,096-call exhaustion, plus scalar-secret refusal on the real retention/inspect path; pin exact causes, counts and absent unsafe bodies.
Add R-I's separate HOME/TMPDIR collisions through real compile and selected launch, asserting the startup cause, zero store lookup/child start, and normal DOCS_TOKEN/private-directory control. Independent removal of reserved-name validation must fail.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-cli/src/doctor/capability_tests.rs`.

Publish grant/veto migration and measured namespace/stdio/empty-restriction limits, secret-read refusal, protected startup requirements, delayed settled checkpoints, fixed retention budgets, historical evidence protection and same-root run serialization. Explain durable session failures, the broker's scalar/structural masking refusal and its fixed startup-key collisions; distinguish broker validation from the unchanged shared secret grammar and legacy mask_json semantics.

Closes tasks 47.1 and 47.2; requirements [SD3](specs/slice-two-delivery/spec.md), [SC1](specs/slice-two-contracts/spec.md), [SC2](specs/slice-two-contracts/spec.md), [SC3](specs/slice-two-contracts/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Guides agree with actual compile/report/inspect evidence and explain quiescing old writers; no new realm grant, MCP server or unmeasured support claim is shipped.

Guide edits require coherence with the enabling proof, not behavior-mirroring tests.

Documents/evidence: `docs/guides/agent-library.md`, `docs/guides/provider-adapters.md`, `docs/guides/recipe-authoring.md`, `docs/guides/secrets.md`, `docs/security-model.md`, `docs/status.md`, `docs/reference/cli.md`, `contracts/README.md`.

### U10a — Audit removals, then validate and hand off

Audit requirement/task/test mapping, each compiling removal/restored pass, no new suppression/clone/unused API and frozen bytes against each unit's main. Repair a missing proof in its assigned suite, not by declaring it proved.

Closes tasks 48.1 and 48.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: All 26 requirement IDs covered; typed refusal text pins once per module; no frozen fixture regeneration or recording gaps.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Run the complete D10 validation set on the restored final candidate, obtain external exact coverage and Linux/macOS/remote results naming its head; keep missing results pending.

Closes tasks 49.1 and 49.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: Literal nonzero covered/total equality for lines/branches/functions, pinned compiler agreement, self/verify compiles, measured identities and signed merge-queue delivery; never push from a seat.

Owning tests: `crates/brokkr-runtime/tests/witness_digests.rs`, `crates/brokkr-runtime/src/bundle/compose_tests.rs`.
