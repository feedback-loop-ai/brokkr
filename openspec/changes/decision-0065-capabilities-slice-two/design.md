# Decision 0065 slice two — broker, gates and citable calls

Status: proposed design for operator ruling; documents only.
Change: decision-0065-capabilities-slice-two.
Base checked: 2a23488b19f6dea271264f6a85b40d46a84af39e, 2026-10-03.

## Context

See [proposal.md](proposal.md) for motivation and adoption. Read the local
dialect file and all specify/clarify/design/tasks/analyze/return instructions,
then the rendered OpenSpec artifact instructions, without invoking a workflow
runner. The two operator maps were read whole, as were the commissioned
decisions and slice-one proposal/design/specs/operator rulings. D4's ordering
and D11's deferral are constraints, not opportunities to redesign native power.

The current change adopts twelve staged files. There are no position files
under .forge for this council and no returned_from in the supplied context.
This design therefore records a source-based synthesis, not a council vote;
the engine's later seats still judge it. No delegated positions are invented.

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
| protocol/adapters.rs:687–704; cli/tests/machine_proof.rs:3141–3184 | Exactly one plaintext injector; move and reuse it, preserving the proof's cardinality. |
| runtime/bundle.rs:4352–4362; contracts/tool-dialect.v1.schema.json | Binding minimum is an existing policy fact; MCP version/connection/secrets/retained already have a frozen schema. |
| agents/charters/researcher.md:20–30 | Existing prose names both capabilities and the exact DATA clause in one paragraph. |

The current GitHub API list and complete changed-file pages were checked for
all six open PRs on 2026-10-03: #404, #452, #460, #486, #494 and #500.
Their titles/bodies/file paths/patches contain no 0077 claim. The index gap
table reserves 0072/0074/0075 and 0076 exists; retain proposed 0077. This is
an observation, not a remote reservation. The commission's agreement that
#487 takes realms v7 supersedes PR #494's historical v6 wording.

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

### D1. Adopt and reconcile the available claims

| Claim / source | Disposition and evidence |
| --- | --- |
| R1 and map 1: harness owns the child, final proof is singleton | Adopt. Proposed 0077 amends the words "launched by the engine"; use one engine-configured broker child per holding, never an engine sibling outside #403. |
| R2 and map 1: existing box channel only | Adopt namespace/workspace-hands/network-false checks. A wanted request cannot excuse a hard site refusal; provider carriage still follows 0065 ruling 5. |
| R3 and first draft: Codex isolation unproven | Adopt measurement-first candidates. Reject choosing a private Codex home or table replacement from source inspection alone. |
| R4 and map 2: no artifact store and only engine writes journal | Combine protected broker ledgers with engine publication/fold. Reject a writable worktree ledger or broker SQLite writer. |
| R5: one PR per narrow unit, inert until enabling | Adopt. Interpret inert as no compiled MCP execution. U1–U4 can independently correct strictness, panics, native gates and native recording. |
| Draft: realms v7 veto | Amend proposal/SC2/CR1 first: use the next realms version after v7, preserving #487's fields. |
| Draft: tool-dialect v2 mandatory | Reject under 0071 ruling 6. Frozen v1 already has every needed field. Preserve its syntax and distinguish valid-but-unexecutable URL/reference connections through SC1 scenarios. |
| Draft: local signing/format/typos must succeed here | Amend SD4 under the second commission: unsigned local commit, operator format/typos and squash signing outside the box. First-visit failures remain historical. |
| Draft: retained data is always actually delivered | Refine CR2 before design: durable child outcome is not proof of harness receipt across a crash. Normal delivery uses exactly the persisted masked data; the crash scenario names the limit. |
| Draft: U6 proxy precedes U8 ledger | Split: U6 includes minimum durable writer, otherwise forwarding violates MB3/CR3. U8 adds staging, publication, engine fold/recovery and inspect. |
| Slice-one D4 / D11 | Adopt validation before compatibility and independent native OFF; reject a broker-specific nonempty restriction bypass. |
| Council positions | None supplied or present. No assent, dissent, severity average or missing-position clearance is recorded. |

Clarification is encoded in the owning scenarios, not a separate ambiguous
FAQ. These corrections amend this adopted draft's proposed choices; they do
not change a ruled upstream artifact. If later evidence requires weakening
R1–R5, D4 or D11, return upstream at that owner instead of changing a task.

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
new evidence or refusal. Measurement can close rows through honest refusal;
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
   write confinement protecting evidence. Requires and
   wants both refuse on failure.
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
8. Seal the complete outcome; preflight the whole serving command and repeat
   its independent final assessment immediately before the actual spawn.

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
| Seat record | v6 attribution group capability/dialect/tool/call_id/call_state, optional response_sha256 on succeeded/failed only, and conditional turn requirement from SC4. Old rows remain valid. |
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
elicitation or subscriptions; reject their requests, ignore irrelevant
notifications safely, and implement cancellation locally with no new power.
Error responses use fixed safe causes; never echo raw child stderr/arguments.

Each valid tools/call gets an internal sequence unrelated to JSON-RPC ID.
Append and fsync Started before forwarding; append/fsync Terminal after
masking/staging and before delivering. Local denials get Refused with no
child invocation or digest. Serialization uses one active call, while the
reader continues handling cancellation and bounded protocol frames.
Clock/deadline/process observations stay in the shell; tests inject clock
facts and closure seams, not mock-only traits (0071 rulings 1, 2).

The child gets a clean base environment containing a trusted executable
search path, locale and engine-created private HOME/TMPDIR, plus the declared
store bindings through the shared injector. No inherited harness credential,
LD variables or Brokkr control variables are copied. Driver spawn removes all
dialect secret names from the harness and broker inherited environments;
U0-measured authentication-name collisions take MB4's exact compatibility
refusal. The real child resolves only the operator store, never ambient values. No shell is inserted;
the operator's dialect can explicitly choose its own executable, which is
trusted operator code, not seat-provided argv. Version checking is an
implementation-version assertion, not a hash attestation of that executable.

Decode complete bounded protocol messages before masking all text, including
object keys and nested encoded content. Canonicalize keys in UTF-8 byte order,
retain array order, use compact serde-compatible JSON number/string encoding
and no trailing newline. Reject duplicate keys both before and after masking.
Reattach the original correlation envelope only to the masked response.
Stderr is drained with bounded retained tail and a rolling masking window
large enough for each known encoding; no raw diagnostic reaches a log.
A transformed secret outside 0012's known encodings remains its documented
limit. Shared injector extraction retains exactly one production accessor.

### D6. Protected evidence and ownership

Use an engine-resolved host state directory
HOME/.local/state/brokkr/capabilities/<repo-sha256>/<run>/<attempt>/ outside
the operated tree. Resolve HOME only in the host shell and pass the path;
core/view read no environment. repo-sha256 is over the canonical operated
repository path bytes, not a model string. Refuse a state root or secret store
inside writable seat reach, including custom binds/aliases, rather than
moving them silently. A store in the operated worktree, including the default
.forge/secrets.env when that tree is exposed, takes MB4's exact store refusal;
use the existing operator --secrets-file option for an inaccessible store.
Parent directories are owner-only, created/read through
owner-rooted no-follow handles, with exclusive files and an active owner lock.
The attempt inventory pins each plan digest and broker identity; the engine
durably creates it before spawn. An engine restart derives the same root from
recorded run/attempt identity and verifies it before reading. Missing host
state parks; no reconstruction from a workspace lookalike.

An inventory tracks Prepared, Started and settled launch evidence from the
engine's process observations. Per-broker files comprise immutable plan,
append-only ledger and response staging. A broker cannot choose paths in a
request or fresh grants. Access mode alone is not the defense: hands exposes
none of the private root and rejects overlaps/aliases; native model tools
must retain the previously proved read-only write boundary. A harness with
unproved native write containment cannot hold MCP. This does not claim that
Codex's native reads cannot see host files (0043's recorded residual stands).

Published content stays exactly at the operated repository's
.forge/artifacts/sha256/<hex>. Before spawning any MCP-holding site, create
and pin the artifact directory then mount it read-only over the writable
worktree view after user binds. Mask aliases and reject writable/overlay
binds exposing another route to the protected objects. Pin ancestors by
handles; reject changed roots, symlink/nonregular targets and multiply
linked existing artifacts. An ancestor replacement cannot redirect the
engine's bound write; detected drift refuses further publication/folding.
Mount/read-only failure refuses launch, never falls back to worktree writes.
U8a's real namespace tests must prove ancestor rename, duplicate bind,
symlink and hardlink attempts alongside legitimate worktree writes.
A path-string comparison or chmod-only test cannot enable U9.

For retention, write canonical masked bytes to an exclusive staging file,
fsync it and its directory, then reference its digest from durable terminal
metadata. Engine verifies owner, regular kind, length, digest and canonical
bytes using the same bound handle, writes a same-filesystem temporary artifact,
fsyncs and atomically publishes without replacement, then fsyncs the parent.
An existing identical digest is reused only after verification. The digest
checkpoint comes last. Publication failure prevents successful result admission.
A broker never writes the final artifact store or opens forge.db.

The current bounded trust model is the operator-selected real server outside
the box. This is not a generic filesystem/network sandbox for malicious
operator binaries. A seat has no path to alter the broker plan, choose another
server or leak secrets via permitted protocol output; #403 residuals and the
known-transformation limit remain named. Widening reach transport is D11's
later slice, not invented here.

### D7. Ledger and crash semantics

Use closed Opened, Started, Terminal and Closed variants. Every record has
a contiguous record_seq; calls additionally have call_seq beginning at one.
Owner contains the run/effect/attempt/site/instance/server/capability/dialect
pin, or an immutable header reference checked against that same inventory.
A schema/version marker is private and fixed. An ungranted syntactically valid
tool may occur only in a refused call. No arguments, request URLs, command,
response body or arbitrary child text is journal metadata.

Limits: 4 KiB per metadata record, 64 MiB per ledger, at most 4,096 accepted
calls per broker attempt; reserve space for each accepted call's terminal
before forwarding. At a limit, terminate the broker session with
"broker ledger exceeds the attempt limit" before accepting another call.
Stop reading new request frames while enough reserved capacity still exists
for every decoded valid request's records, including concurrency and tool
refusals. Unread bytes after session closure are no accepted capability call;
no decoded valid call is silently dropped. Reader memory is bounded
per record; retained data has MB3's separate 8 MiB limit.

Derive call_id as a bounded lowercase SHA-256 identifier over a canonical
typed tuple: native = attempt/site/instance/provider/measured call identity;
broker = attempt/site/instance/server/call_seq. Prefix n- or m- separates the
domains. No Debug encoding, shared mutable ordinal across panel sites or
truncation. Native events with no reliable measured ID fail attribution;
a measured one-event format may use its per-attempt ordinal. U4 needs the
format measurement but not U1 strictness implementation.

Runtime's common fold validates the inventory and every complete record.
Journal deduplication keys are call_id plus Started or Terminal, with exact
typed payload comparison; succeeded versus failed is a conflicting terminal,
not a new stage. The journal, not a local cursor, decides what already landed.
Native Observed rows are one per call and never claim completion. Broker
harness telemetry is diagnostic only and cannot create duplicate uses.
Capability lifecycle checkpoints bypass progress coalescing/rate-drop logic;
ordinary usage accounting keeps its current behavior.

| Failure window | Recovery and result admission |
| --- | --- |
| Before Started is durable | No forwarding. Failed persistence returns CR3's recording cause. |
| After Started, before child response/terminal | Settle the owned process tree, record Interrupted without digest, never replay the child call. External action may have happened. |
| After staging, before terminal | Orphan bytes prove no result; Interrupted. No artifact digest inferred from the file. |
| After terminal, before harness delivery | Record/publish child outcome and prepared data; do not claim model receipt or automatically replay. |
| After publication, before checkpoint | Verify/reuse immutable artifact, append the absent stage once. |
| After checkpoint, before local cursor | Match committed stage exactly and append nothing. |
| Partial tail or malformed/gapped/conflicting inventory/ledger | Preserve verified prior records, mark unresolved calls interrupted only after settlement, and fail/park through existing failed/indeterminate paths with CR3/CR4 causes. Never admit success. |

Fold available ledgers during checkpoint drain and fold them all again after
process settlement, before single/panel/step terminal results, stop/timeout
completion or restarted-attempt settlement. Missing expected ledger differs
from a process proven never to have started. A normal zero-call session has
Opened/Closed; a missing file is not an empty ledger. An unexpected ledger
is an integrity failure. The engine remains the sole journal writer, using
the existing fenced append (0029), and does not choose a next phase itself.

### D8. Charter check, native attribution and inspect

GP2 owns deterministic paragraph semantics: requested names with safe-name
boundaries, normalized wrapped whitespace, exact DATA clause, no headings or
fenced code satisfying prose. Scan verified bytes once for each loaded office
and inline requester. Test fences, CRLF/blank lines, repeated paragraphs and
name prefixes. Do not add an NLP classifier or claim it controls model behavior.

Normalize observations at the harness edge before the existing 80-character
display clamp. Use selected sealed holdings for attribution; native inventory
alone describes known tools but grants none. Reject ambiguous reverse mapping
at compile and unheld known calls at runtime. Engine-owned site/instance/
boundary stamps are applied together. Missing MCP telemetry fields limit
diagnostics but cannot erase the authoritative broker ledger; missing native
identity blocks a completeness claim with CC2's exact cause.

One pure brokkr-view projection groups call stages and exposes attribution
and optional digest. CLI/TUI/browser can paint that common fact without
re-deriving capability ownership. No new comparison behavior is introduced.
Inspect adds --capability-call <call_id> to its existing run selector; it finds
exactly one journal-owned call, selects the terminal digest, and opens the
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

U1, U2, U3 and U4 have no dependency on one another. U1/U4 require their U0
evidence; U2/U3 do not. The listed order serializes their merges and conflicts,
not their logic. Later feature units explicitly join their outputs. U5c's
grant extraction and U6a's injector extraction also have independent roots.

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

Document validation observed on 2026-10-03: unstaged and staged
git diff --check both exited 0; openspec validate --all --strict exited 0
with 20 passed, 0 failed. Its requirement-length notices are informational.
The document audit found 26 requirements, each with scenarios (87 total),
49 ordered PR rows with at most three production paths each, and 98 unique
unchecked tasks covering every requirement. R1–R5 are byte-identical to the
adopted staged record. Only the fourteen commissioned Markdown artifacts
are in the document commit set; runtime proofs remain future work.

### D11. Clarification and analysis disposition

After proposal/deltas and before tasks, the following ambiguities are settled
in scenarios: realm version ownership (SC2), v1 execution compatibility (SC1),
hard R2 versus wants (MB2), gate precedence (GP1), paragraph identity (GP2),
record-versus-receipt crash gap (CR2), deduplication and unexpected ledgers
(CC2/CR4), bounded MCP (MB3), and the second-visit gate/signing change (SD4).

| Analyze category | Finding and disposition at earliest owner |
| --- | --- |
| Inconsistency, high if implemented | Draft's worktree evidence protection was only a promise. CR2 already required protection; D6 names host state, immutable publication, mount/alias checks and U8a/U9b proof. No runtime closure is claimed. 0071 rulings 1, 9. |
| Underspecification, medium | Delivery/retention atomicity: corrected CR2 to distinguish prepared bytes from receipt, with a crash scenario; D7 and tasks follow. 0071 ruling 3. |
| Unused surface, low | Draft-only tool-dialect v2 had no additional data consumer. Removed in proposal and SC1; retain frozen v1 with typed execution refusals. 0071 ruling 6. |
| Inconsistency, medium | Realm v7 collision: corrected proposal/SC2/CR1 to the next realms version after v7, preserving #487. 0071 rulings 3, 5. |
| Coverage gap, medium | U6 cannot forward before logging and U7 spans more than three files. Split minimal ledger before proxy, then retention/fold; explicit files and task owners below. 0071 rulings 4, 9. |
| Ambiguity, medium | Broker turn correlation is not guaranteed by a harness. SC4 permits only attributed broker rows to omit turn; D7 never invents usage/turns and U4b tests the conditional schema. 0071 ruling 3. |
| House alignment / deterministic gates | Format, OpenSpec structure and whitespace findings belong to their gates (info, 0071 ruling 11); planned runtime gates remain pending. No claimed green runtime from this document. |
| Duplication / traceability | Requirement semantics live in deltas; design explains choices; numbered tasks reference every requirement and one unit. No copied agent or recipe, no parallel policy implementation. 0071 rulings 2, 5, 7, 10. |

These are answered drafting findings, not a security review of an implemented
broker. No unresolved contradiction remains in the planned scope; U0 and the
implementation proofs remain open prerequisites. If evidence makes R2 or
protected evidence infeasible for a harness, keep it refused; never downgrade
a high refusal finding to an informational gate result.

## Risks / Trade-offs

- Harness config precedence may defeat isolation → U0 controls admission,
  and SI2 refuses affected sites even if this makes an existing recipe unusable.
- A version string is weaker than an executable digest → name that limit;
  dialect bytes/version remain pinned, and this slice adds no installer.
- Protected artifacts live under a writable repository ancestor → bound
  handles, read-only mount overlays, alias refusal and adversarial real
  namespace proofs precede enablement.
- Process groups are not cgroups → reuse #403, retain #472/macOS residuals;
  namespace activation requires real Linux cleanup evidence.
- Exact recording costs a sync before each external call → pay it, with
  bounded serial calls and explicit failure; no asynchronous unsafe queue.
- Response persistence and receipt cannot commit atomically → distinguish
  child outcome from receipt, and never replay an uncertain external effect.
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
stdio grants with empty restrictions.

Rollback before U9 simply leaves the MCP fence. After U9, an emergency
rollback restores the compile refusal and refuses new starts; it does not
erase ledgers/artifacts, reinterpret an old manifest, replay external calls
or revert native strictness/gate protections. Existing in-flight operations
are settled through the same journal/cleanup path, not left detached.

## Open Questions

Only measurements and external results remain: which of D2's named candidates
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
| `crates/brokkr-protocol/src/native_controls/mcp.rs` | U1a, U1c, U6c, U6d, U7a, U7d, U7e, U8a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/agents.rs` (1671) | U1b, U1f, U3b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/load.rs` (1334) | U1b, U3b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/agents/mcp.rs` | U1b, U7b | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters.rs` (7268) | U1c, U4c, U6a, U7d, U7e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/adapters/mcp.rs` | U1c, U1g, U7d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/claude.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/codex.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/lanetally.json` | U1d | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/dsh.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `adapters/exec.json` | U1e | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle.rs` (7815) | U1f, U3a, U3c, U4d | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/bundle/mcp.rs` | U1f, U7b, U9b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/engine.rs` (4862) | U1g, U4e, U7c, U8c, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/capabilities.rs` (2369) | U2, U3a, U4d, U5a, U5b, U5e, U5f, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/binding.rs` | U2, U5b, U5e, U9a, U9b | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-cli/src/doctor.rs` (1567) | U2, U9a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/gates.rs` | U3a | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/agents/charter_data.rs` | U3b, U3c | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-runtime/src/bundle/charters.rs` | U3c | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-store/src/seat_record.rs` (880) | U4a, U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat_record/validation.rs` | U4a | v6 reserved for this slice; preserve append/export/verify dispatch |
| `contracts/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-store/src/seat-record.v6.schema.json` | U4b | v6 reserved for this slice; preserve append/export/verify dispatch |
| `crates/brokkr-protocol/src/adapters/capability_calls.rs` | U4c | #347/#348 harness splits; #467 strictness and #500 probe evidence |
| `crates/brokkr-protocol/src/lib.rs` | U4c, U6a | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-runtime/src/capabilities/attribution.rs` | U4d | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/engine/capability_calls.rs` | U4e, U4f, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/checkpoints.rs` | U4e, U8d | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/sequence.rs` | U4f, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/resume.rs` | U4f | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-view/src/lib.rs` (2767) | U4g, U8f | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-view/src/capability_calls.rs` | U4g, U8f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/dialect.rs` | U5a, U5b, U5e | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-core/src/realms.rs` (902) | U5c, U5d | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `crates/brokkr-core/src/realms/grants.rs` | U5c, U5d | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/realms.v<N>.schema.json` | U5d | #487/v7; allocate the next realms version after v7 without editing old bytes |
| `contracts/run-manifest.v12.schema.json` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-runtime/src/capabilities/manifest.rs` | U5f | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-protocol/src/secret_spawn.rs` | U6a | 0012 single-injector proof; no second plaintext accessor |
| `crates/brokkr-cli/src/cli_args.rs` | U6b, U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/lib.rs` (1918) | U6b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker.rs` | U6b, U6c, U6e | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/session.rs` | U6c, U6d, U6e, U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs` | U6d, U8b | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/broker/rpc.rs` | U6e, U6f | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-cli/src/broker/output.rs` | U6f, U8b | New module; keep protocol types shared, no duplicate policy |
| `crates/brokkr-protocol/src/hands.rs` (1276) | U7a, U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/broker.rs` | U7c, U8c, U8d, U8e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/marks.rs` | U7c, U7e | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-protocol/src/hands/evidence.rs` | U8a | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-runtime/src/engine/artifacts.rs` | U8c | #403/#415 cleanup and ongoing runtime splits; preserve owner handles |
| `crates/brokkr-cli/src/verbs/readouts.rs` | U8g | Recheck concurrent main edits and module registration before the row |
| `crates/brokkr-cli/src/verbs/capability_artifact.rs` | U8g | 0065 follow-up refactors; identity and gate ordering must survive |
| `crates/brokkr-cli/src/doctor/capabilities.rs` | U9a | 0065 follow-up refactors; identity and gate ordering must survive |

Also coordinate `crates/brokkr-runtime/tests/witness_digests.rs`,
`crates/brokkr-runtime/src/bundle/compose_tests.rs` (every identity-changing
row), `crates/brokkr-runtime/tests/frozen_contracts.rs` (U4b/U5d/U5f/U10a),
`crates/brokkr-cli/tests/machine_proof.rs` (U6a), the exact owning suites
listed with each unit, and `contracts/README.md` plus U9c's guides.
The current `store/lib.rs` append fence, `protocol/secret.rs`,
`protocol/process/tree.rs` and `cli/render.rs` are review seams with no
planned production edits: shared consumers suffice. If they actually need
editing, budget a new split before touching them. Their invariants remain
covered by store, secret, cleanup and readout proofs.

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
as **the next realms version after v7** at U5d, including v7's fields. It is
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
| U5a | U2 | Extract dialect loading; 20.1–20.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/dialect.rs` |
| U5b | U5a | Retain typed MCP connection and policy; 21.1–21.2 | `crates/brokkr-runtime/src/capabilities/dialect.rs`; `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U5c | Independent | Extract version-aware realm grants; 22.1–22.2 | `crates/brokkr-core/src/realms.rs`; `crates/brokkr-core/src/realms/grants.rs` |
| U5d | U5c | Mint the retention-veto realm version; 23.1–23.2 | `contracts/realms.v<N>.schema.json`; `crates/brokkr-core/src/realms.rs`; `crates/brokkr-core/src/realms/grants.rs` |
| U5e | U5b, U5d | Bind reservation and effective retention; 24.1–24.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/dialect.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U5f | U5e | Publish manifest v12 with its live native consumer; 25.1–25.2 | `contracts/run-manifest.v12.schema.json`; `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/manifest.rs` |
| U6a | Independent | Share the one secret injector; 26.1–26.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/secret_spawn.rs`; `crates/brokkr-protocol/src/lib.rs` |
| U6b | U5f, U6a | Introduce the broker command as a closed handler; 27.1–27.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/lib.rs`; `crates/brokkr-cli/src/broker.rs` |
| U6c | U6b, U1a | Define and consume the bound plan; 28.1–28.2 | `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6d | U6c | Establish durable ledger records before calls; 29.1–29.2 | `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs`; `crates/brokkr-cli/src/broker/session.rs` |
| U6e | U6d | Serve the filtered protocol; 30.1–30.2 | `crates/brokkr-cli/src/broker.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/rpc.rs` |
| U6f | U6e | Mask and canonicalize all output; 31.1–31.2 | `crates/brokkr-cli/src/broker/rpc.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-cli/src/broker/output.rs` |
| U6g | U6f | Prove attempt cleanup; 32.1–32.2 | None |
| U7a | U1g, U5f, U6g | Represent the complete server set; 33.1–33.2 | `crates/brokkr-protocol/src/native_controls.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs`; `crates/brokkr-protocol/src/hands.rs` |
| U7b | U7a | Consume adapter carriage and selected holdings; 34.1–34.2 | `crates/brokkr-runtime/src/agents.rs`; `crates/brokkr-runtime/src/agents/mcp.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U7c | U7b | Provision protected per-attempt plans; 35.1–35.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/marks.rs` |
| U7d | U7c | Deliver checked multi-server configurations; 36.1–36.2 | `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/adapters/mcp.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U7e | U7d | Render selected capability discovery; 37.1–37.2 | `crates/brokkr-runtime/src/engine/marks.rs`; `crates/brokkr-protocol/src/adapters.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8a | U7e | Protect artifact paths in workspace hands; 38.1–38.2 | `crates/brokkr-protocol/src/hands.rs`; `crates/brokkr-protocol/src/hands/evidence.rs`; `crates/brokkr-protocol/src/native_controls/mcp.rs` |
| U8b | U8a | Stage retained responses before delivery; 39.1–39.2 | `crates/brokkr-cli/src/broker/output.rs`; `crates/brokkr-cli/src/broker/session.rs`; `crates/brokkr-protocol/src/native_controls/mcp/ledger.rs` |
| U8c | U8b | Publish verified content-addressed artifacts; 40.1–40.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/artifacts.rs` |
| U8d | U8c, U4g | Fold ledger stages through the append fence; 41.1–41.2 | `crates/brokkr-runtime/src/engine/broker.rs`; `crates/brokkr-runtime/src/engine/capability_calls.rs`; `crates/brokkr-runtime/src/engine/checkpoints.rs` |
| U8e | U8d | Settle and recover every attempt's evidence; 42.1–42.2 | `crates/brokkr-runtime/src/engine.rs`; `crates/brokkr-runtime/src/engine/sequence.rs`; `crates/brokkr-runtime/src/engine/broker.rs` |
| U8f | U8e | Expose retained evidence in the pure view; 43.1–43.2 | `crates/brokkr-view/src/capability_calls.rs`; `crates/brokkr-view/src/lib.rs` |
| U8g | U8f | Open a cited artifact through inspect; 44.1–44.2 | `crates/brokkr-cli/src/cli_args.rs`; `crates/brokkr-cli/src/verbs/readouts.rs`; `crates/brokkr-cli/src/verbs/capability_artifact.rs` |
| U9a | U8g, U2, U3c | Prepare whole-plan MCP doctor reporting; 45.1–45.2 | `crates/brokkr-cli/src/doctor.rs`; `crates/brokkr-cli/src/doctor/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs` |
| U9b | U9a | Enable the proved namespace path; 46.1–46.2 | `crates/brokkr-runtime/src/capabilities.rs`; `crates/brokkr-runtime/src/capabilities/binding.rs`; `crates/brokkr-runtime/src/bundle/mcp.rs` |
| U9c | U9b | Publish implemented scope and contract guidance; 47.1–47.2 | None |
| U10a | U9c | Audit every removal proof and scope; 48.1–48.2 | None |
| U10b | U10a | Run final candidate gates and hand off; 49.1–49.2 | None |

### U0 — Measure isolation and telemetry

Execute D2's complete positive/sentinel matrix; record raw redacted observations and adapter-specific verdicts, including native call identifiers. This PR changes evidence only.

Closes tasks 1.1 and 1.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: A reviewer can reproduce every supported row; missing rows are unmeasured and cannot license U1/U4.

Evidence/documents only; do not add behavior-mirroring tests for this row.

Documents/evidence: `docs/evidence/adapters/slice-two-mcp-isolation.md`, `docs/evidence/adapters/slice-two-mcp-observations.json`.

### U1a — Extract existing MCP transport checks

Move the current single-server transport/parser checks into the named module, retaining production callers and exact behavior; create room under the existing file baseline.

Closes tasks 2.1 and 2.2; requirements [SI2](specs/strict-mcp-isolation/spec.md), [MB1](specs/mcp-capability-broker/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Unchanged hands-only exact-state and authored-option tests, with no added server acceptance.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U1b — Type adapter MCP facts

Extract McpSupport and its loader into agents/mcp.rs; represent measured, unsupported and unmeasured isolation by invocation shape and carriage separately. Consume the type at load; legacy servers never grant authority.

Closes tasks 3.1 and 3.2; requirements [SI1](specs/strict-mcp-isolation/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Strict closed decoding, old unsupported data and independent wrapper evidence have exact variants.

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

Factor the existing charter-byte read through its verified binding and check every loaded requesting office, including unseated/subtracted asks. Use the bounded paragraph scanner.

Closes tasks 11.1 and 11.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Existing researcher prose passes; absent, nearby, repeated, fenced and substring-only names refuse with exact causes.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U3c — Check inline requester charters

Extract inline verified-charter consumption into bundle/charters.rs and use the same scanner for every executable inline body; no new reopening of unchecked paths.

Closes tasks 12.1 and 12.2; requirements [GP2](specs/gate-capability-policy/spec.md).
Proof: Inline, selected/inherited/member/step cases, preserved dispatch pins and fixed DATA reminder; no duplicated scanner.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U4a — Make room for additive record validation

Extract existing validation functions into a consumed child module; preserve dispatch and export/verify behavior.

Closes tasks 13.1 and 13.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Historical version and exact refusal tests stay green; the oversized parent shrinks.

Owning tests: `crates/brokkr-store/src/tests.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

### U4b — Publish and consume seat-record v6

Add the schema and its exact embedded copy, select the engine-line boundary on then-current main, and use v6 at append/export/verify. Older valid rows remain valid.

Closes tasks 14.1 and 14.2; requirements [CC3](specs/capability-call-checkpoints/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Frozen-byte pins, embedded equality and each field/state/dependency boundary are exact; no package version bump solely for dispatch.

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

Route step and nested-member observations through the same stamp, preserve session eligibility and ensure cold replacement has a new attempt identity.

Closes tasks 18.1 and 18.2; requirements [CC1](specs/capability-call-checkpoints/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Separate actual resumed and cold replacement tests, sibling ownership and repeated-call IDs.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U4g — Derive call evidence once

Extract checkpoint-derived call facts into a pure view module, consumed by the existing view. One call ID counts once; historical absence is unrecorded.

Closes tasks 19.1 and 19.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CC3](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Literal view values for native observed and synthetic broker stages, no I/O or grant admission in view.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U5a — Extract dialect loading

Move the current dialect edge loader and contained read use to a consumed module without changing v1 acceptance or the MCP fence.

Closes tasks 20.1 and 20.2; requirements [SC1](specs/slice-two-contracts/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: Existing schema/duplicate/containment and refusal tests prove extraction parity.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U5b — Retain typed MCP connection and policy

Carry typed v1 connection, version, names, retained, egress and sends. Use exhaustive kind and retention variants; reject runtime use of references/URL only through the specified compatibility causes after enablement.

Closes tasks 21.1 and 21.2; requirements [SC1](specs/slice-two-contracts/spec.md), [MB4](specs/mcp-capability-broker/spec.md).
Proof: Exact field retention/digests, no process/store read, old native data and pre-U9 refusal intact.

Owning tests: `crates/brokkr-runtime/src/agents/tests.rs`, `crates/brokkr-runtime/src/capabilities/tests.rs`, `crates/brokkr-runtime/src/bundle/agent_tests.rs`.

### U5c — Extract version-aware realm grants

Extract current grant parsing/serialization and reserved-key selection, retaining its v6 behavior and any landed v7 fields. Typed errors keep old text.

Closes tasks 22.1 and 22.2; requirements [SC2](specs/slice-two-contracts/spec.md).
Proof: Legacy realm round trips, empty/omitted lists and restriction identity remain exact.

Owning tests: `crates/brokkr-core/tests/realms.rs`.

### U5d — Mint the retention-veto realm version

N denotes the next realms version after v7, allocated against main at this PR; preserve #487's v7 fields. Decode retain false as Veto only in that new version; prior spellings stay restrictions.

Closes tasks 23.1 and 23.2; requirements [SC2](specs/slice-two-contracts/spec.md), [CR1](specs/capability-response-retention/spec.md).
Proof: New/old round trips, bad veto values and provisional-office compatibility; frozen pins do not move.

Owning tests: `crates/brokkr-core/tests/realms.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

Documents/evidence: `contracts/README.md`.

### U5e — Bind reservation and effective retention

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

Move bind_environment to the named shared module and keep the single expose_for_spawn production call there; existing harness spawns consume it immediately. New errors are typed.

Closes tasks 26.1 and 26.2; requirements [MB4](specs/mcp-capability-broker/spec.md).
Proof: Existing secret machine proof is updated to the one new location, not weakened; no second accessor or environment fallback.

Owning tests: `crates/brokkr-cli/tests/machine_proof.rs`, `crates/brokkr-protocol/src/adapters/tests.rs`.

### U6b — Introduce the broker command as a closed handler

Add Cmd plus handler for broker serve with bounded engine-plan locator/digest arguments. An unbound manual invocation refuses; no raw server argv, grants or secret values are CLI options. Move dispatch code out of the oversized CLI parent.

Closes tasks 27.1 and 27.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md).
Proof: Exact CLI parsing, bound-plan refusal and unchanged commands; compile still refuses MCP.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6c — Define and consume the bound plan

Define the shared typed BrokerPlan at the private protocol edge and implement owner-rooted reads and direct child spawn in session.rs. Only engine inventory-bound plans pass, with sanitized base environment and shared injector.

Closes tasks 28.1 and 28.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [MB4](specs/mcp-capability-broker/spec.md), [MB5](specs/mcp-capability-broker/spec.md).
Proof: Fake child version/init, egress clearance, missing/denied/store-only names and no-argv leaks; process group is inherited.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6d — Establish durable ledger records before calls

Define the shared closed ledger vocabulary and bounded writer; session consumes it for Opened/Closed lifecycle. This moves minimal ledger durability earlier than U8 because MB3 cannot forward an unrecorded call.

Closes tasks 29.1 and 29.2; requirements [CR3](specs/capability-response-retention/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Owner binding, exclusive open, durable opened/closed and write/sync failure controls; no journal writer.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6e — Serve the filtered protocol

Implement MB3's protocol subset and bounds, complete filtered catalog and per-call Started/Terminal logging through the shared ledger. No unsafe passthrough, shell injection or background daemon.

Closes tasks 30.1 and 30.2; requirements [MB3](specs/mcp-capability-broker/spec.md), [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md).
Proof: Fake-server allow/deny/pagination/version/method/cancellation/limit matrix; record failure precedes external action.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6f — Mask and canonicalize all output

Factor bounded response/stderr masking and canonical serialization, including textual keys and common encodings. Retained staging remains refused until U8 storage is wired; no retained response can be forwarded without its durable evidence.

Closes tasks 31.1 and 31.2; requirements [MB4](specs/mcp-capability-broker/spec.md), [CR2](specs/capability-response-retention/spec.md).
Proof: Literal/encoded/split-chunk leak scan across every output, invalid-after-masking protocol identities and exact masked data bytes.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U6g — Prove attempt cleanup

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

Extract broker provisioning into engine/broker.rs. Write durable inventory and per-capability plan/ledger roots outside seat-writable reach, then seal their digest/owner in selected spawn inputs.

Closes tasks 35.1 and 35.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [MB3](specs/mcp-capability-broker/spec.md), [CR3](specs/capability-response-retention/spec.md).
Proof: Plan substitution, missing inventory, pre-start failure and every site/attempt identity are exact; no live compile escape.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7d — Deliver checked multi-server configurations

Wire the expanded server set and dialect-secret environment removals into each U0-supported builder and final consumption point; unsupported measured carriers refuse. No modifications after checked command creation.

Closes tasks 36.1 and 36.2; requirements [MB1](specs/mcp-capability-broker/spec.md), [SI2](specs/strict-mcp-isolation/spec.md).
Proof: Actual composed cold/resume/replacement commands match independent literal server intent, and mutations are refused at the final serving boundary.

Owning tests: `crates/brokkr-protocol/src/adapters/tests.rs`, `crates/brokkr-protocol/src/native_controls/tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U7e — Render selected capability discovery

Reuse the measured discovery identifier from adapter hands.notice, since eligible MCP sites already have hands; render fixed capability/tool guidance per selected attempt outside the requested digest.

Closes tasks 37.1 and 37.2; requirements [SD3](specs/slice-two-delivery/spec.md), [CC2](specs/capability-call-checkpoints/spec.md).
Proof: Codex deferred tool notice, fallback clearing, optional drop and hostile prose tests; hands notice stays byte-coherent.

Owning tests: `crates/brokkr-runtime/src/engine/capability_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`, `crates/brokkr-runtime/tests/capability_launch.rs`.

### U8a — Protect artifact paths in workspace hands

Carry engine-only protected storage facts, overlay the artifacts directory read-only after writable worktree binds, and reject every conflicting writable/overlay alias by canonical owner identity. No new authored HandsSpec key.

Closes tasks 38.1 and 38.2; requirements [CR2](specs/capability-response-retention/spec.md), [MB3](specs/mcp-capability-broker/spec.md).
Proof: Real namespace writes/ancestor renames/aliases/overlap controls cannot change evidence; mount failure refuses, never omits the protection.

Owning tests: `crates/brokkr-protocol/src/hands/tests.rs`, `crates/brokkr-runtime/src/engine/boundary_tests.rs`.

### U8b — Stage retained responses before delivery

Stage only opted-in masked canonical bytes in the protected attempt root, fsync/rename before durable terminal metadata and delivery; metadata-only veto writes no body.

Closes tasks 39.1 and 39.2; requirements [CR1](specs/capability-response-retention/spec.md), [CR2](specs/capability-response-retention/spec.md), [CR3](specs/capability-response-retention/spec.md).
Proof: Four retention outcomes and failure injection at stage/sync/terminal/delivery; error bodies retained, local failures have no digest.

Owning tests: `crates/brokkr-cli/tests/capability_broker.rs`.

### U8c — Publish verified content-addressed artifacts

Use owner-rooted no-follow reads, verify staged bytes and atomically publish immutable digest paths under the operated repository; retain handles across checks.

Closes tasks 40.1 and 40.2; requirements [CR2](specs/capability-response-retention/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Existing-good reuse, collision/mismatch, nonregular/symlink/hardlink and concurrent replacement controls, exact missing/corrupt causes.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8d — Fold ledger stages through the append fence

Consume shared typed ledger records, compare sealed ownership/tools, publish before digest checkpoints and deduplicate stages from committed journal facts. Keep capability lifecycle rows out of lossy progress coalescing.

Closes tasks 41.1 and 41.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md).
Proof: Started/terminal/refused/failed/interrupted, duplicate/conflict/gap/partial/unexpected/missing cases and one-use view counts.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`.

### U8e — Settle and recover every attempt's evidence

Fold after owned processes settle and before any successful ordinary/panel/step result is admitted, including stop/timeout/engine restart paths. Failed folds use existing failed/indeterminate transitions.

Closes tasks 42.1 and 42.2; requirements [CR3](specs/capability-response-retention/spec.md), [CR4](specs/capability-response-retention/spec.md), [MB5](specs/mcp-capability-broker/spec.md).
Proof: Crash at each durability window, panel/sequence/fallback ownership, no automatic child replay, restart idempotence from journal.

Owning tests: `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-runtime/src/engine/cleanup_tests.rs`, `crates/brokkr-runtime/src/engine/resume_tests.rs`.

### U8f — Expose retained evidence in the pure view

Extend the already consumed call projection with digest/provenance and terminal selection, without reading paths or deriving grants.

Closes tasks 43.1 and 43.2; requirements [CC2](specs/capability-call-checkpoints/spec.md), [CR5](specs/capability-response-retention/spec.md).
Proof: Unrecorded/veto-unknown/recorded distinctions, duplicate stages and shared display data values.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U8g — Open a cited artifact through inspect

Add inspect --capability-call <call_id>; read the selected run's derived call, use shared runtime artifact reader and emit verified bytes/provenance. Register the handler as a submodule of readouts.rs so no fourth file is needed.

Closes tasks 44.1 and 44.2; requirements [CR5](specs/capability-response-retention/spec.md).
Proof: Complete exact bytes and missing/no-digest/corrupt/path/foreign-run refusal tests; no refetch and no view I/O.

Owning tests: `crates/brokkr-view/src/tests.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`.

### U9a — Prepare whole-plan MCP doctor reporting

Extract capability reporting into the named module and use the shared complete planner for native/MCP metadata; preserve pre-U9 public compile refusal until U9b.

Closes tasks 45.1 and 45.2; requirements [SC5](specs/slice-two-contracts/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Grant versus holding and retention facts with static scope, native denial and exact no-spawn assertions.

Owning tests: `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U9b — Enable the proved namespace path

Lift only SD3's realm-wide unbuilt-kind fence; activate namespace/hands/network, class, strictness, carriage, connection, secret-clearance and D11 rules in D3 order. Every prior unit must be accepted and proved.

Closes tasks 46.1 and 46.2; requirements [MB2](specs/mcp-capability-broker/spec.md), [SD3](specs/slice-two-delivery/spec.md).
Proof: Real compile/launch through engine, deterministic harness fixture, actual broker and fake dialect; independent denial, secret, retention, tamper, cold/fallback/member/step controls. No hand-built holding substitutes.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-cli/src/doctor/capability_tests.rs`.

### U9c — Publish implemented scope and contract guidance

Describe operator grant/veto migration, strictness refusals, secret-store-only binding, namespace/stdio/empty restrictions and checkpoint/artifact inspection with measured limitations.

Closes tasks 47.1 and 47.2; requirements [SD3](specs/slice-two-delivery/spec.md), [SC1](specs/slice-two-contracts/spec.md), [SC2](specs/slice-two-contracts/spec.md), [SC3](specs/slice-two-contracts/spec.md), [SC4](specs/slice-two-contracts/spec.md).
Proof: Examples agree with real compile/report evidence; no MCP server or realm grant is shipped.

Evidence/documents only; do not add behavior-mirroring tests for this row.

Documents/evidence: `docs/guides/agent-library.md`, `docs/guides/provider-adapters.md`, `docs/guides/recipe-authoring.md`, `docs/guides/secrets.md`, `docs/security-model.md`, `docs/status.md`, `docs/reference/cli.md`, `contracts/README.md`.

### U10a — Audit every removal proof and scope

Audit requirement/task/test mapping, each compiling removal/restored pass, no new suppression/clone/unused API and frozen bytes against each unit's main. Repair a missing proof in its assigned suite, not by declaring it proved.

Closes tasks 48.1 and 48.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD2](specs/slice-two-delivery/spec.md).
Proof: All 26 requirement IDs covered; typed refusal text pins once per module; no frozen fixture regeneration or recording gaps.

Owning tests: `crates/brokkr-runtime/tests/capability_broker_launch.rs`, `crates/brokkr-runtime/tests/capability_ledger.rs`, `crates/brokkr-cli/tests/capability_broker.rs`, `crates/brokkr-cli/tests/capability_artifacts.rs`, `crates/brokkr-runtime/tests/frozen_contracts.rs`.

### U10b — Run final candidate gates and hand off

Run the complete D10 validation set on the restored final candidate, obtain external exact coverage and Linux/macOS/remote results naming its head; keep missing results pending.

Closes tasks 49.1 and 49.2; requirements [SD4](specs/slice-two-delivery/spec.md), [SD1](specs/slice-two-delivery/spec.md).
Proof: Literal nonzero covered/total equality for lines/branches/functions, pinned compiler agreement, self/verify compiles, measured identities and signed merge-queue delivery; never push from a seat.

Owning tests: `crates/brokkr-runtime/tests/witness_digests.rs`, `crates/brokkr-runtime/src/bundle/compose_tests.rs`.
