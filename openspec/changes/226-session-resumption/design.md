## Context

This design adopts the existing **226-session-resumption** change, reopened at
`667086a` over its implementation commit `75ae68e`, while retaining the
commissioned shipped base `5bc8cf305aaef9af269866cbf83f094939691399`, answers
A–H, task repairs F1–F6 and F7's committed specification repair at `c56fa73`.
See [proposal.md](proposal.md) for motivation and scope. The current run,
`close-issue-226-only-codex-resum-dafafa41`, has returned `clear` from
clarification; its quota-failed predecessors supplied no clarification verdict.
This is a council reconciliation of the adopted design, not a new change or a
second production enactment.

Both current council positions were read in full:
`.forge/design/positions/robustness.md` and
`.forge/design/positions/simplicity.md`. D10 records their current claims and
evidence-based dispositions. They agree on additive v5, structural identity,
the full measured provider minimum and reuse of the existing resume/hold seams.
Their remaining tension is implementation scale: robustness asks that two
fail-closed outcomes be made unmistakable, while simplicity correctly argues
that each fits the existing provider-specific planner and first-work hold. No
`returned_from` finding accompanies this visit; D12 preserves the resolved F7
return and its dependent obligations. Controller integration of PR250 and #222
remains separate.

The implementation at this head already contains the reusable architecture:
`engine/resume.rs::{eligible_offer, SiteContext, InstanceKey,
ConfirmedSession}`, `Store::started_here`, negotiated `Body::Resume` with a
correlated `PendingOffer`, provider-specific planners, `LaunchHold`, and
`SeatRecordVersion::of_engine`. Its task record truthfully shows **92 of 102**
complete. The ten open tasks are live provider proof 10.5–10.8, the dependent
enablements 11.1–11.4, host exact coverage 15.5 and controller handoff 15.8.
Codex and Claude remain `unmeasured`, LaneTally remains independently
`unmeasured`, and DSH is `unsupported`; none is delivery evidence.

Current code inspection also confirms the two council seams. A different-root
`LaunchHold` outcome suppresses a launch row, but the adapter invocation can
still return its ordinary exit/result outcome; D7 requires that mismatch to
prevent successful acceptance. Claude rejects competing conversation selectors,
but otherwise appends `extra` wholesale; D5 requires adapter-specific duplicate,
arity and precedence checks for every authoritative restriction before the
shape can be enabled. These are downstream conformance gaps already owned by
D5/D7/D11 and the existing tasks, not grounds for a second lifecycle subsystem
or a generic provider grammar.

Read: README, house rules, decisions 0004/0005/0009/0030/0034/0042, applicable
0043/0046 rulings and shipped 0053 behavior; runtime dispatch/ownership,
protocol and adapter code; adapter declarations; seat-record v4 and store
dispatch; SDD charter; the five deltas, surviving tasks and specification
history. The registry carries proposed 0056. The commission reserves 0054/0055
for other fires and **0056** for this change.

The controller evidence index now names three captures. The two proposal
capture hashes were rechecked, along with the newer
`.forge/controller-dsh-resume-source-interface.json`, captured
**2026-09-09T07:43:14.783176+00:00**, SHA-256
`62aba46649b6919117e3f4882ccb63f19aa87445274f6821883997e682c56d95`.
All 40 contained source hashes match their captured text. D6 uses the concrete
agent-resume and declarative configuration interfaces to identify the missing
headless caller route; it does not claim live enforcement. The older four-file
headless capture's unexamined-extension limit describes that capture, not the
completed 40-file source trace.

Claude's captured help says background resume can copy a running session,
invalid settings can be ignored in print mode, and a recorded system prompt
can survive later launch text. These facts support restricting execution shape
and keeping the current task in the current invocation's user input. Help and
installed source are interface evidence only. On this visit, the exact bare
`codex`, `claude`, `dsh`, `claude-lanetally`, `cargo` and `rustc` version
commands each returned command-not-found inside the workspace hands. No live
provider behavior or Rust validation is claimed from this design seat.

## Goals / Non-Goals

**Goals:** Derive one owned offer per executing work site from existing durable
attempt evidence; reconstruct current restrictions; record one actual launch;
preserve bounded recovery and truthful progress across process death. Separate
engine eligibility from provider planning and from acceptance/accounting.

**Non-Goals:** No policy input, reducer state, session database, provider workflow
runner, transcript reader, CLI/TUI derivation or cross-provider memory transfer.
No global settings, provider installation patch, non-Rust production runner,
new DSH hands admission, boundary, release work or sibling-worktree change.
The controller integrates #222 overlap; this design uses the commissioned base
and supplied controller evidence.

## Decisions

### D1 — Preserve the adopted answers and the full minimum

Proposal answers A–H stand. Work/gate class comes from the selected compiled
invocation, including a step's own class and a panel member's inherited class.
Single gates lose their old offer path under SR1 and accepted 0042's fresh-judge
rule. Work authors and council positions remain eligible even when a later
step judges their output. Exec and dialect validators never negotiate model
resume or emit model launch.

AS1's minimum remains measured Claude boxed-workspace resume, DSH's admitted
headless work resume, and preserved Codex work shapes after current-version
remeasurement. Cold reporting is preparation, not completion. LaneTally is
assessed independently; implement its measured safe admitted shapes without
inferring wrapper support from Claude.

**Alternative rejected:** declaring DSH unsupported from its headless startup
alone and treating launch reporting as closure contradicts AS1 and answer F.
Both current positions reject that reduction. The complete installed source
trace establishes lower session interfaces but also establishes that the
headless caller still creates a fresh agent and cannot direct its admitted task
to the configured restored one. D6 records the exact required upstream caller
seam. That measured unsupported state blocks full delivery; it does not
authorize a narrower downstream claim of success.

**Bindings:** SR1/AS1; class-aware runtime dispatch and required provider proof.
No earlier specification fault is established by the council's size objections.

### D2 — Structural site identity, reusing existing dispatch

Introduce private Rust `SiteKey`, `InstanceKey` and `ConfirmedSession` types,
separating ancestry, owner and provider identity. `SiteKey` contains the outer
seat, selected case (explicit absence for an unselected body), body kind and
a tagged path: single, panel member, sequence model step, or step plus panel
member. Use distinct name/index components from the compiled body walk; never
split a colon string to recover ancestry. Names are exact, not case-folded on
recovery. The pinned bundle fixes ordering.

Canonical JSON plus existing SHA-256, domain-separated with
`brokkr.resume-site/v1`, yields a 64-lowercase-hex `site_ref`. The enclosing run
journal supplies run scope. The manifest/selected body permits reconstruction
of the key; accounting need not copy the entire structured name.

Build a per-invocation context when the actual site is composed, with its key,
selected candidate, current class and restrictions. Single dispatch, `MemberRun`
and sequence model dispatch carry this context and their own offer. Panel
workers pass the context key beside checkpoints to the single journal writer
for stamping. A reused completed step does not invoke a driver or gain a launch.

Retain display tags for compatibility, but never use them as resume keys or
compare whole-panel provenance to one member's owner. Candidate/boundary lookup
must resolve to a unique structural site before its context is computed. The
current flattened maps cannot distinguish, for example, step `a:b`/member `c`
from step `a`/member `b:c`. Add a compile-time uniqueness check for the existing
flattened addresses within their actual lookup scopes: ambiguous bundles fail
before spawn. This prevents aliasing at the existing lookup boundary without
changing chain progression or historical tag meanings. Ordinary repeated member
names under different steps remain valid. Key the new resume map by `SiteKey`.

`InstanceKey` contains this site's agent/provider/model/effort and chain index,
normalized driver identity and pinned unexpanded command-template digest,
adapter digest, engine version, pinned bundle identity, class, boundary and
hands declaration identity. Inline model commands use their pinned command and
compiled model/effort facts; missing required identity means no offer. Hash
with `brokkr.resume-instance/v1` to `instance_ref`. A sibling changing candidate
within its existing pinned chain does not change this key; a changed bundle
still fails the broad manifest fence. Exclude ephemeral result paths, expanded
MCP filenames and capability tokens from comparison: their declaration/scope
is pinned, while each invocation reconstructs current values. No resolved
credential, provider-home contents or private argv enters identity evidence.

**Alternatives:** both current positions support private structural types and
equality digests while retaining display tags and existing dispatch. Keep that
combination. Simplicity cut 6 would omit collision rejection unless an existing
lookup cannot distinguish the sites; that condition is present:
`engine.rs::invocation_sites` flattens the path, `select_candidates` inserts it
into a `BTreeMap`, and `argv_for` selects by that same key. `site_boundary`
also consults the flattened label in `bundle.hands`. A collision can select
another site's candidate or hands identity before the resume hash is built.
Retain the scoped uniqueness check and tasks 3.5–3.6, without a general naming
restriction or a public addressing API.

Decline robustness's additional `effect/started` site table: D3's stamped
confirmed checkpoint supplies the required durable association, and the
existing attempt/start/manifest link supplies invocation provenance. The query
must validate that link; omitting the duplicate table does not permit trusting
a provider stamp. No new start association or effect-start contract version
is needed.

**Bindings:** SR1/SR2/SR5/LE5; composition, ambiguity check, checkpoint stamps,
canonical-key tests and topology/identity tests.

### D3 — Confirmed checkpoints are the durable session authority

Replace the untyped locator query with a pure query over this run's evidence:

1. Require a work model site, unchanged manifest and `started_here`. Preserve
   existing manifest refusal and indeterminate recovery ahead of dispatch.
2. Find the newest checkpoint with confirmed root evidence for this exact
   `site_ref`. Resolve its effect/attempt to this run's requested seat and
   durable start. Require engine-stamped site/instance facts.
3. Compare that newest session's `instance_ref` with the current instance.
   An incompatible or ambiguous newest owner denies the offer; never search
   behind it for an older matching owner. A failure without confirmed root
   supplies no replacement. A nonpersistent root is not offered.
4. Offer the complete validated provider ID through `Body::Resume`. Pass its
   observed harness identity to the adapter's private current-start context
   for D5's version check. A locator/configured UUID never proves existence.

The engine overwrites/removes driver-supplied `site_ref` and `instance_ref`
using the invoking context, as it does boundary. Aggregates never establish
session ownership. Confirmed root evidence is separate from `transcript`;
DSH retains its directory locator. Delegated sessions cannot replace the root.
After the root is latched, a different root is a mismatch, not last-writer-wins.

Use provider-generated identity for known Claude/Codex paths. Preserve answer
H's fresh assignment mechanism when a measured creation interface requires it,
including a potential DSH integration. Assignment is invocation-local intent
held in memory; this design adds **no durable pre-spawn creation-intent field**.
Provider confirmation must exactly match the assigned ID before root evidence
is emitted. A kill loses unconfirmed intent; the next authorized cold creation
chooses a fresh value. If implementation needs durable creation intent, return
to this design for its representation first, without widening a start payload.

**Alternatives:** both current positions support harvesting known provider
roots and conditional fresh assignment under SR3/H. Retain both mechanisms
with the same confirmation threshold, choosing assignment only where the
measured route needs it. Adopt simplicity's refusal of speculative durable
creation intent; robustness's conditional start extension is unnecessary when
no such intent is persisted. No private-transcript scan, ambient latest
selection, registry or automatic backfill is introduced.

**Bindings:** SR2–SR5/LE1/LE3; pure lookup, stamps, root latch, assignment,
kill-window and fresh-process recovery tests.

### D4 — Add seat-record v5 for facts v4 cannot state

The gap is not Claude's permission-mode name. v4's `session_id` can carry an ID,
but it does not encode a confirmed root's role, persistence or CLI version;
there is no structural site/instance stamp. Its closed refusal enum also cannot
honestly describe cold because installed 0.153.4 has only 0.148.0 resume proof,
or because headless support is unmeasured. Neither is an invalid ID or a harness
rejection. These gaps follow from v4/source inspection and answers F/G/H.

Add `contracts/seat-record.v5.schema.json` and the matching embedded store copy.
Published and embedded v1–v4 bytes stay unchanged. v5 is a superset of v4; add
only optional checkpoint properties:

| Property | Meaning and bound |
|---|---|
| `site_ref` | Engine stamp: 64 lowercase hex characters, D2's site digest. |
| `instance_ref` | Engine stamp: 64 lowercase hex characters, D2's owner digest. |
| `root_session` | Closed object, constructed only after measured provider-root confirmation; fields below. |
| `root_session.kind` | `codex-thread`, `claude-session` or `dsh-session`; LaneTally retains Claude's kind. |
| `root_session.id` | Complete ID, 1–80 ASCII characters, alphanumeric first and then alphanumeric/underscore/hyphen. Provider grammar is narrower; never truncate. |
| `root_session.harness_version` | Observed normalized CLI version, 1–80 ASCII alphanumeric/dot/underscore/plus/hyphen characters, starting alphanumeric. Never substitute the requested version. |
| `root_session.wrapper_digest` | Optional 64-lowercase-hex digest of measured wrapper source when applicable; absent for plain Claude. |
| `root_session.persistent` | Boolean from established current-shape persistence semantics. False roots establish the newest owner but cannot be offered. |

The 80-character ceiling retains the existing ID budget. Captured Claude
creation grammar is UUID; DSH's creation spelling is `session-` plus UUID;
Codex's existing grammar is bounded at 80. These do not prove resume selection
or root confirmation. A supported selector that cannot fit losslessly requires
a design return before admission, never clamping or hiding it in `home`.
Unknown root/persistence/version leaves `root_session` absent even when a known
fresh launch reports cold. Existing session/transcript fields keep their
meanings; new offers use explicit root evidence.

Keep `launch: cold | resumed`. Extend only v5's refusal enum with
`unsupported-resume`, `unverified-harness`, `restrictions-unavailable`,
`instance-changed` and `nonpersistent-session`. Retain the five existing tokens
for actual malformed IDs, missing/unsupported Codex sandbox, incompatible argv
or measured harness rejection. New tokens distinguish unsupported interface,
missing/stale proof, unavailable non-Codex restrictions, detectable owner/client
drift and nonpersistence. A cold launch without an offer has no refusal. Human
reasons/commands belong in reviewed support evidence, not accounting. New
built-in conformance requires root evidence for resumed and refusal only with
cold; do not add constraints that invalidate historical v4 rows.

The proposal now declares `boundary-record` modified, and its complete
MODIFIED requirement, **The seat record carries the boundary as seat-record/v4**,
authorizes this dispatch in addition to LE2’s additive vocabulary. Dispatch v5
from **0.10.0** in `SeatRecordVersion::of_engine`, following the
newest-within-line precedent: 0.9.0 up to but excluding 0.10.0 selects v4,
0.8.0 up to but excluding 0.9.0 selects v3, and older/unparseable engines select
v1. Keep existing parsing and direct v1–v4 validation. No package version bump.
Old 0.10.0 rows without new fields remain valid under v5; absence
is not confirmation. Use the same dispatch/validator at append, export, import
verification and offline verification; diagnostics name contract/path, never
values. Include direct v4/v5 tests and validate engine stamps at the fence.

**Alternatives:** both current positions now accept the minimal additive v5
and F7's boundary-record amendment. Retain the fixed D4 fields and five new
refusal tokens; reject robustness's suggestion to defer their names until the
provider plans, because the adopted tasks and versioned fence already depend
on this vocabulary. A newly measured fact that cannot fit returns here first.
Omit Claude modes from `sandbox`: boundary remains engine authority and
restrictions remain pinned configuration. No new event type, event schema,
database schema, per-record version marker or effect-start extension is needed.

**Bindings:** LE2/LE5/SR2/SR3; schema dispatch/fence, embedded/frozen-byte tests,
built-in conformance and stamp tests; `boundary-record` / **The seat record
carries the boundary as seat-record/v4** for shared dispatch, preserved boundary
stamping and historical compatibility. F1/F5’s task repairs remain binding:
the store’s new resumed-requires-root and refusal-only-with-cold conditions
apply only to `site_ref`-stamped rows, while the unconditional form is required
of this change’s producers. Existing valid unstamped 0.10.0 rows, including
third-party refusal-bearing shapes, stay valid at all four fences.

### D5 — Receive once; admit only a measured current invocation

All four built-in model adapters advertise `resume` as *offer receipt*; exec
advertises none. Keep driver protocol v1 and `run_attempt_resuming` ordering.
Third-party drivers receive handles only if they advertise receipt, never by
prompt or another field.

Replace `serve_io`'s string with `PendingOffer`: effect ID, attempt ID and a
size-bounded candidate handle. Empty means ordinary cold. A matching start
consumes one offer once. Duplicate offers, malformed correlation, wrong
effect/attempt, resume before negotiation or malformed resume envelopes poison
that exchange and launch no provider. Send bounded protocol failure if the
correlation is known, then end the exchange; otherwise terminate without
pretending a provider refused. Never repair it into cold execution.
Cancellation, shutdown, EOF and completed invocation discard pending state.
A correlated string that fails provider ID grammar is instead a declined offer
(`invalid-session-id`); any cold substitute independently passes current policy.

Extend adapter declarations with a typed `resume` assessment: explicit
`unmeasured`, `unsupported` or `supported` status per named execution shape,
assessed identity, applicable classes/boundaries/hands mode, evidence
references and measured limitations. Supported entries name interface,
restriction, exact-root and current-accounting evidence; root evidence includes
persistence, and wrapper evidence remains independently qualified where used.
Keep F2/F6's exact distinction from tasks 6.1/6.6: identity is either measured,
with the assessed/applicable version, or explicitly unknown with a bounded
non-empty reason. Only measured identity can support enablement; unsupported
requires a measured reason. Unmeasured admits either identity form. An absent
assessment or unnamed shape loads as unmeasured and enables nothing; present
malformed data, bare true, neither identity form or unknown without a reason
is a loader refusal. Honest unknown identity is not malformed. The existing
adapter content digest pins this compact closed shape; no evidence database,
probe DSL or version-range resolver is introduced. Carry the selected assessment
through `Candidate`/`SiteSpawn` into private driver context in `Start.input`, separate
from rendered `context`, phase inputs and the resume handle. Missing assessment
for inline/standalone invocation means unmeasured, not implicit support. The
existing start-input object carries this context; no wire type is added.

Probe the selected executable's version using its *measured* version interface
once per invocation, with bounded output inside the existing total deadline.
Compare observed version with pinned support and the originating root.
LaneTally additionally checks wrapper identity and underlying Claude version
through a measured interface; do not invent a LaneTally version command.
Missing, changed or unreadable identity disables resume with a bounded reason.
No per-attempt model experiment runs. Version change requires new dated proof;
historical Codex acceptance does not authorize later versions. Record the
observed version, not the desired pin.

Parse restrictions from an engine-composed plan: declaration argv, model/effort,
generated current hands fragment, result door and scoped resources. Preserve
provenance before flattening; compare the actual expanded fragment with this
plan. User passthrough resembling generated MCP settings is not authorized
merely by resemblance. Private context is driver-only, never model authority,
policy input, checkpoint content or a copy of private argv into the journal.

Use a measured resume allow-list with exact arity, duplicate and precedence
checks. Competing selectors, forks, background/cloud/worktree launch, extra
positional handles and unknown restriction/profile overrides are not forwarded.
Apply selector protection to cold/gate paths too: unsafe cold settings fail
before provider work, instead of forwarding `--continue` after declining an
offer. Do not silently strip persistence or restrictions. A resume-only
incompatibility may select a known safe cold spelling; cold-inadmissible settings
refuse. Retain non-provider configuration-failure treatment rather than creating
new model-chain fallback for argument validation.

**Alternatives:** both positions support reusing the protocol with correlated
one-use receipt, separate version qualification and closed provider planners.
Retain argument provenance because the composed hands fragment has authority
that lookalike passthrough lacks. Reject raw cold-argv-plus-resume and a
selector-only deny-list: captured Claude help documents copy-producing
background behavior and ignored invalid print settings. Equal flag lists do
not prove enforcement. Both positions now accept answer G's current-version
qualification; the original council's enabled-until-contradicted alternative
remains rejected, not an outstanding dispute.

**Bindings:** SR4/SR5/AS1–AS3; loading/packaging, private-context rendering,
protocol abuse, argv/precedence and current-version mismatch tests.

### D6 — Provider planners spend interface evidence without inventing behavior

Share orchestration/folds where identical; each adapter owns its launch planner
and measured confirmation/accounting rules.

| Adapter | Construction decision | Proof still required before enabling |
|---|---|---|
| Codex | Retain explicit `codex exec resume --json`, workdir through `current_dir`, `-c sandbox_mode=...`, effort and safe passthrough. Admit exact engine-generated MCP fragments separately from arbitrary `-c`. | Current 0.153.4 resume help, class/boxed-fragment enforcement, exact root, current events and pre-work rejection semantics. 0030's 0.148.0 measurement remains historical regression scope. |
| Claude | Known print/stream-json path with exactly `--resume <owned-id>` and the current restriction plan: permission mode, model/effort, `--tools ""`, strict MCP config, current MCP document and allowed workspace tool where boxed. Current prompt stays on stdin. | Root-opening semantics, complete effective restrictions/precedence, persistent identity and current-only stream/accounting on 2.1.266. |
| LaneTally | Share Claude parsing where measured; keep wrapper and capture marker, with separately gated planner. Never substitute plain Claude. | Wrapper identity/forwarding, underlying version, root, capture attribution and applicable restrictions. Unsupported hands remain unsupported. |
| DSH | Keep the admitted headless profile, Rust-owned per-invocation overlay, model/effort settings and retained scope. Installed 0.1.2-rc.1 has no supported caller route from its `{task}` headless config to a restored agent, so this shape remains disabled without inventing one. | A later supported `dsh-headless` interface must accept a session/resume identifier and route the admitted task, followup and event interval to that restored root; then exact-root, restriction and current-event proof is required before enablement. |

Claude creates with its harvested provider ID by default. Bare `-r`, continue,
fork, user `--session-id`, `--from-pr`, `--teleport`, background/cloud selectors
and replacement workdirs are excluded. Do not depend on new `--system-prompt`
text overriding a remembered snapshot; captured help says it may not. Keep
the current task/result instruction on the current user-input path.
`--restricted` is not blindly appended: help rejects `bypassPermissions`, and
its relation to the current class must be measured if used. Preserve an
explicit no-persistence setting; such a root cannot become a persistent offer.

The completed DSH interface trace is conclusive for the installed package. In
captured `@deepseek-ai/dsh-agent/lib/index.js:557`, `agents.resume(options)`
delegates to the registered factory. `dsh-agent-loop/lib/index.js:1049` admits
`Config.agents[].resumeSessionId`; its constructor at :1096–1119 calls
`resumeWith` when persistence becomes available. At :1349–1378 the factory
loads through `sessionPersistence.prepare` and publishes with source `resume`.
These are real lower-level source interfaces, not an invented configuration.
However, `dsh-headless/lib/index.js` validates only `task`, calls
`agents.create` with a random ID, and applies its followup, `firstSeq` and
summary to that newly created agent. Restoring a second configured agent does
not route the admitted headless task to it. Task 10.3's bounded trace therefore
correctly records installed `dsh-headless` 0.1.2-rc.1 as unsupported; task 10.7
cannot supply same-root live proof over a caller route this package lacks.

The exact upstream requirement is now fixed: `dsh-headless` must accept a
session or resume identifier in its supported per-invocation configuration
schema and route its admitted task, followup and event interval to the restored
agent instead of the `session-${randomUUID()}` agent it currently mints. That
surface must also permit the current model/settings/restriction plan to replace
restored state. Lower-level `agents.resume`, configured-agent restoration and
the TUI selector do not satisfy this headless caller requirement.

The allowed integration is Rust construction of supported declarative
settings/patch data consumed by the installed CLI. Do not embed a new JavaScript
runner in that data, monkey-patch `agents.create`, override UUID generation,
edit installed packages or substitute TUI. An exported Session API alone does
not make it callable through the admitted headless route. A later installed
version may satisfy the exact requirement through supported declarative data;
if so, re-open construction and live proof from that interface. Until then,
retain the measured limitation and report **AS1 upstream** at delivery, naming
proposal and `adapter-resume-safety` as owners, without patching the provider or
narrowing the minimum.

A future supported route must reuse the same retained root without copying
history. Current-only accounting may observe new bytes/provider sequences
through the existing owned, bounded path; it cannot load old transcripts to
manufacture context. Preserve read caps and path checks. This design invents no
DSH config keys, confirmation events or rejection codes. Unimplemented or
disabled DSH support cannot satisfy the minimum.

Use a bounded cold/resume probe per materially different required shape,
combining root, current-scope access and accounting observations where possible.
Test permitted current workspace action and prohibited native/ambient MCP or
out-of-scope action as applicable, including expiry of old hands grants and
operation of the new grant. Source may establish enforcement for an axis;
name the observation supporting each axis. Record tool admission and actual
filesystem effects, not a model's statement that it was blocked. One denied
write alone does not prove permission binding, native-tool removal, MCP
exclusion and grant renewal.
Do not test a cross-product of unrelated flags or repeatedly run models.

**Alternatives:** both current positions support four small provider planners,
shared folds where behavior matches, and bounded proof for each materially
different enabled shape. Adopt that instead of a generic provider framework.
Reject help-as-enforcement, a blanket ban on generated Codex configuration and
one denial as proof of unrelated restriction axes. A combined probe may satisfy
several axes only when each has an attributable observation. Missing live proof
remains explicit, never filled by synthetic telemetry.

Preserve F3's ordering: interface tasks 10.1–10.4 precede the provider-specific
construction in 8.5–8.9 and accounting in 9.1–9.3; live proof 10.5–10.8 follows
that construction and gates enablement in group 11. The evidence gate does not
block preparable common plumbing or honest unmeasured declarations.

**Bindings:** AS1–AS3/SR3/LE1/LE4; planner tests from captured grammar plus
separate installed-provider restriction/root/accounting evidence.

### D7 — Confirm before publishing; replace only before proven work

Invocation state is distinct from wire acceptance:

`plan -> child spawned -> exact root confirmed -> current work -> terminal`

A safe fresh plan gains a candidate cold outcome only after successful spawn.
Resume has no launch fact until the provider confirms the exact offered root.
Provider-specific observation confirms the outcome; a flag or exit zero cannot.
Current work before required confirmation, or a different root, ends that
resume as failed/uncertain, with no guessed launch and no replacement. Hold
root/launch candidates before `run_seat`'s existing first-work boundary.

Flush Accepted, then held confirmed launch/location facts in observed order,
then first work. Publish one final launch per executing model site, not per
internal spawn. Known fresh paths can report cold without root evidence, but
supply no resumable root. Exec emits none. Non-refusal endings keep the existing
acceptance path. Classified provider refusal before work/delivery keeps no
Accepted and no checkpoints, including no finishing/launch row. A kill in that
window loses held rows, as proposed 0053 documents. New root evidence uses a
pre-work lifecycle step; it must not accidentally satisfy `begins_work`.

Represent resume outcomes explicitly: confirmed, conclusively rejected before
work, failed after confirmation/work, or uncertain. Only measured machine
*session rejection* with no root, work, current result or uncertainty authorizes
one cold replacement. Read the entire invocation outcome before spending that
permission: an error notice followed by work/clean delivery is not rejection.
Keep delivered work/result files under the existing rules rather than discarding
them because of an advisory; missing required exact-root confirmation still
makes the attempt failed/indeterminate, never a successful guessed rejoin.
Model/auth/quota refusal stays the separate shipped classifier.

A local decline permits at most one independently safe cold launch. A conclusive
session rejection clears that child's held launch, root/locator, model/effort
and accounting candidates before one cold replacement with `harness-refused`.
No recursive fallback. A failed replacement reports its outcome. If it is itself a classified pre-work
provider refusal, the no-checkpoint exception applies and the bounded failure
preserves available refusal tokens; a launch row cannot be forced through it.

Keep one outer driver process and its process-tree watchdog/deadline across
both launches and version checks. Replacement gets only remaining time; no
new attempt ID, chain slot or timer. Current production cancellation terminates
the owned process tree; `serve_io` does not asynchronously read `Body::Cancel`
during synchronous invocation. Preserve that actual bound and prevent a later
replacement after cancellation rather than claiming cooperative cancellation.
A watchdog kill retains `deadline_killed` and never becomes failure-to-start
just because acceptance was held. Engine indeterminate recovery stays parked.

**Alternatives:** adopt robustness's explicit states and reject speculative
resumed followed by corrective cold rows. Combine simplicity Cut E's
provider-specific rejection proof with a shared one-replacement controller;
AS4 does not demand a fabricated DSH classifier. Reject generic nonzero/no-locator
fallback: Codex's old predicate needs current session-rejection proof to satisfy
the stronger AS4 contract.

**Bindings:** AS4/AS5/LE1/LE3/SR5; outcome sequences, wire ordering, replacement
count, deadline/cancellation and crash-recovery tests.

### D8 — Current-only accounting, privacy and historical compatibility

Each enabled planner establishes a measured restored-history/current-work
boundary before feeding usage/tool folds. Prefer provider turn/event cursors.
For retained-file accounting, capture the owned pre-followup offset/sequence
through the existing bounded path; byte count is not root confirmation.
Rotation, truncation, missing cursor or uncertain attribution omits affected
measurements, or refuses resume if no current-work boundary can be established.
Never read historical transcript text to reconstruct a prompt or reasoning.

Keep inclusive input/cache subsets, cache writes, completion deduplication and
served model/effort conventions. Filter replayed tools/targets too. Lifetime
totals without an established baseline remain absent, never guessed/subtracted
heuristically or replaced by zero. LaneTally retains current wrapper capture
identity. Rejected resume contributes no unconfirmed usage to cold replacement.

Legacy unambiguous local single-work-site Codex rows may offer their thread
through the established kind/flat-ID mapping, subject to all instance,
manifest/origin checks and current adapter-version qualification. Legacy
composites and Claude/DSH locator-only evidence start cold once to establish
explicit roots: no guessed colon ancestry or DSH directory equivalence. A new
row without confirmed root cannot fall back to its legacy fields to evade
confirmation. Historical absent launch stays absent; no events/fixtures are
rewritten. Engine pin mismatch remains authoritative even for legacy Codex.

**Alternatives:** adopt robustness's current-event boundary and narrow migration;
retain existing harvest/folds where measured semantics suffice. Reject the
claim that every locator proves an owned root. #222's readers/readouts remain
outside this work.

**Bindings:** LE4/LE5/SR3/SR5; replay/current-window, missing evidence, legacy
lookup, schema compatibility and frozen corpus tests.

### D9 — Progress persists independently of commits

Change `agents/charters/implementer-sdd.md` once for PM1–PM3: before a group,
record it in progress and name its focused acceptance checks; when a task's
implementation and required focused checks pass, persist its tick and concise
evidence before the next group. Partial implementation and failed/pending checks
stay unchecked with a next action. Group completion, workspace validation,
commit and external delivery remain separate facts.

On retry/re-entry, resumed or cold, read current dialect artifacts/worktree,
reconcile ticks with surviving edits/checks, preserve valid partial work and
reopen invalidated completion with reasons. Current evidence outranks memory.
No transcript recovery. The charter names the task artifact generically;
OpenSpec/spec-kit keep existing locations, task formats and phase maps. The
non-SDD charter does not change and judges remain read-only.

**Alternatives:** adopt simplicity's instruction home and robustness's
in-progress/completed distinction already required by PM1. Two timing clauses
alone omit partial status, verification and discrepancy recovery. Reject a
journal progress store, dialect implement phase or claim that prompts guarantee
obedience. Test rendered instructions/packaged identity and demonstrate an
interruption before commit in a temporary worktree; judges decide completion.

**Bindings:** PM1–PM4; `library_data.rs` charter/roster/identity tests, both
dialects' implement/return prompt tests and a deterministic recovery exercise.

### D10 — Proposed 0056 and explicit council reconciliation

Before production semantic changes, the smith adds
`docs/decisions/0056-same-instance-session-resumption.md` and its registry row,
with **Status: proposed**. Only the operator accepts it. Context/alternatives
cite this design and preserve historical accepted decision text. Required
numbered rulings and enforcement bindings are:

| Ruling | Required content | Enforcement binding |
|---|---|---|
| 1 | Work-site continuity, all gates fresh, every model topology, exec excluded. | D1/D2 runtime class/topology tests. |
| 2 | Same run/site/instance and local origin; newest owner, no older-owner resurrection. | D2/D3 pure query, stamps, manifest/origin tests. |
| 3 | Provider-confirmed root; generated or fresh assigned ID; intent proves nothing. | D3/D4 root latch, fence, assignment/kill-window tests. |
| 4 | Negotiated, correlated, one-use offer in existing wire vocabulary. | D5 protocol/conformance tests. |
| 5 | Required measured provider shapes; current-version qualification; DSH session integration separate from hands. | D5/D6 loader, runtime identity check, dated provider evidence. |
| 6 | Re-impose current restrictions, model/effort, grant and result door; no alternate selectors. | D5/D6 composition and enforcement proof. |
| 7 | One confirmed launch, additive v5 vocabulary and manifest dispatch from 0.10.0 under the amended boundary-record requirement, preserved boundary stamping, first-work hold and privacy fence. | D4/D7 shared append/export/import/offline dispatch, frozen-byte, historical-compatibility and conformance/acceptance tests. |
| 8 | One proven pre-work replacement within deadline/cancellation/chain bounds. | D7 outcome/watchdog tests. |
| 9 | Current-only accounting, unchanged transcript/privacy limits, narrow legacy compatibility. | D8 accounting/export/verify/legacy tests. |
| 10 | Persist truthful progress before next group; reconcile independently of commit. | D9 instruction/identity tests. Actual completion is judgment guidance, not an automatic guarantee. |

The current sitting's sources are the two run-local positions named in Context,
recorded here so the reasoning survives their replacement on a future visit:

- Robustness SHA-256: `962804144359ac49bdfab49657d2579037db1713f0233574ee3a92d5646233cc`.
- Simplicity SHA-256: `2e63945c0588a5b76decb310d3fb6bd258ca1eb0362e187907b1bc1550677341`.

This table replaces the original council's Cut A–E/R1–R6 attribution. The
rejected mechanisms retain their reasons in D1–D9; the current simplicity
position does not advocate cold-only delivery, v4-only evidence, flat resume
identity, harvest-only syntax or unqualified newer Codex enablement.

| Current position / claim | Disposition and evidence |
|---|---|
| Both: the full Claude boxed, DSH headless and remeasured Codex minimum; all work topologies and fresh gates. | Adopt D1/D6. AS1/SR1 and accepted 0042 define the minimum. A support declaration or quota failure cannot narrow it. |
| Robustness: typed structural ownership; simplicity §1: private structural values, equality digests and one pure reverse scan. | Combine D2/D3. The pre-change `Site` and `seat_session` lost ancestry/root kind; the implemented `SiteKey`, `InstanceKey`, checkpoint stamp and existing journal/manifest/origin guards close that gap without a second store. Keep per-site candidate projection so sibling chain movement does not change the owner. |
| Robustness: effect-start site table and numbered extension; simplicity §1/cut 1: confirmed checkpoints suffice. | Adopt the checkpoint association, reject the duplicate start table. Engine stamps plus the validated attempt/start link prove the originating site and instance without another store, reducer field or migration (D3). |
| Both: newest incompatible/nonpersistent owner stops the query; no resurrection or aggregate/child borrowing. | Adopt D3/D8. Ownership is checked after choosing the latest confirmed root, with the narrow existing single-Codex compatibility path only. |
| Simplicity cut 6: omit compile-time flat-tag rejection unless an existing lookup cannot distinguish sites. | Retain D2's scoped rejection: `Selection`, `argv_for` and `bundle.hands` actually use flattened keys. This meets the position's stated exception; tasks 3.5–3.6 remain necessary. No broad naming ban follows. |
| Robustness: correlated pending state; simplicity §2/cut 4: repair the existing wire without protocol v2. | Combine D5. The pre-change `serve_io` overwrote an uncorrelated string; the implemented `PendingOffer` spends `Body::Resume`'s existing correlation IDs, poisons invalid exchanges and consumes once. No new phase or policy input. |
| Both: smallest additive v5, F7's single manifest-derived dispatch and historical compatibility. | Adopt D4/D12 at all four fences. F1/F5's conditions apply only to stamped new rows at the store, while every current producer obeys them. Boundary remains engine-stamped; published/embedded v1–v4 and tagged history stay frozen. |
| Robustness: settle refusal names with provider plans; simplicity §3: retain five selected tokens and no extra permission/argv/history fields. | Retain D4's already-selected bounded vocabulary. Provider-specific semantics must fit truthfully or return to design; no downstream private token is allowed. |
| Both: provider-generated roots by default, fresh assignment only through a measured creation interface; simplicity cut 2: no durable intent. | Adopt D3. Unconfirmed assignment stays in memory, never in captured-session fields; the accepted pre-confirmation crash window needs no new durable protocol. |
| Robustness: detailed support assessments and independent restriction proof; simplicity §4/cut 3: compact pinned data and four explicit planners. | Combine D5/D6. Keep only admission-relevant closed assessment data, including F2/F6's loadable unknown form. Retain separate evidence for interface, restrictions, root/persistence and accounting, including wrapper identity where applicable; no probe DSL, automatic enablement or generic planner language. |
| Both: provider-specific selectors, current restriction reconstruction and cold/gate selector protection. | Adopt D5/D6. Generated hands provenance and captured CLI precedence require exact parsing; a cold fallback cannot carry ambient continue or competing settings. LaneTally cannot inherit Claude qualification or substitute plain Claude. |
| Robustness: Claude's selector-only guard does not reject duplicate or last-wins permission/tool/MCP controls; simplicity §4/cut 3: keep explicit provider planners, not a generic grammar engine. | Combine under D5/D6. Each planner performs closed, measured arity, duplicate and precedence validation for every authoritative restriction. For Claude, a second permission mode, tools list, strictness/MCP document, allowed-tools list, model or effort control is a pre-work refusal on cold and resume paths; merely constructing warm argv as cold argv plus `--resume` is insufficient. This is an adapter-local parser and test matrix, not a cross-provider DSL. |
| Both: use DSH's installed declarative/session route only if it reaches the admitted headless caller. | Adopt D6's completed bounded trace and exact upstream condition. Configured restoration exists, but installed 0.1.2-rc.1 cannot route the admitted headless task to that agent. Retain `unsupported`, keep 10.7/11.3 incomplete, and require the named upstream caller seam before proof or enablement. |
| Robustness: explicit launch state and no speculative resumed row; simplicity §5: extend the existing first-work hold. | Combine D7. The implemented `LaunchHold` supplies confirmation ordering without a second public lifecycle type. Retain shipped 0053 refusal and kill-window behavior, and close the distinct terminal-mismatch seam in the next row. |
| Robustness: a different-root confirmation currently suppresses the launch row but can still return an ordinary successful invocation; simplicity §5/cut 2: one local guard is enough, with no new lifecycle subsystem. | Combine under D7. Keep the private explicit outcome already represented by `LaunchHold`, and make mismatch/required-confirmation absence override the ordinary success path to failed or indeterminate while preserving any delivered file for diagnosis. It publishes no guessed launch, authorizes no cold replacement and cannot produce an accepted successful seat. Test clean exit and valid-result variants, not only launch-row absence. |
| Both: one conclusively rejected pre-work replacement under the original deadline/process tree. | Adopt D7. No generic retry engine or asynchronous cancel protocol; `serve_io` invokes synchronously and the runtime watchdog owns process termination. Stderr, an advisory or nonzero exit alone cannot authorize duplicate work. |
| Both: measured current-event cursor, private transcript boundaries and narrow legacy migration. | Adopt D8. Replayed turns/tools/totals are not new accounting; omit unattributable measurements. No transcript seeding, reader changes, backfill or second provider-session ledger. |
| Robustness: in-progress/completed recovery; simplicity §6: one SDD charter edit. | Combine D9. PM1–PM3 already require those states in the dialect task artifact. Keep focused evidence before the next group, reconcile surviving edits on either warm or cold recovery, and leave judges read-only. No engine progress service; tests prove distribution and an interruption exercise, not model obedience. |
| Robustness: full verification axes; simplicity cut 7: table-driven coverage and bounded probes. | Combine D11. Distinguishable assertions cover 20 requirements / 123 scenarios without one test function per scenario or a provider-flag cross-product. Live evidence remains separate from shims. |
| Simplicity: compress the inherited breakdown to roughly seven groups. | Decline renumbering or deleting the surviving tasks. The current file has 102 tasks after F7 added 2.8, rather than the position's inherited count of 101. F1–F6 repaired concrete dependencies and proof obligations. Coherent work groups and parameterized tests can reduce execution overhead while retaining stable IDs, unchecked evidence tasks and F4/F7's five-capability fold/provenance order. |
| Both: local origin is not account authentication, LaneTally can remain unmeasured/cold, and unsupported required shapes block delivery. | Adopt D5/D6/D8 and Risks. Detectable owner rejection fails closed; do not inspect credentials to invent a stronger guarantee. LaneTally's truthful evidence is still required. |
| Both: keep scope within #226; simplicity cut 8 rejects release work, #222 readers, provider patches and new hands/boundaries. | Adopt the existing Non-Goals. No sibling tree or assumed PR250 integration; controller evidence and later integration remain the only coordination path. |

No new specification or task-breakdown defect is established by these current
positions. F7's upstream defect has already been repaired at its owner; its
consequences remain binding below. Existing AS3, LE1 and tasks 8.10/9.7 already
own the two newly highlighted conformance gaps, so this design seat does not
duplicate or renumber work. The downstream seat must reconcile their checked
markers against the implementation and re-open them with reasons if the required
tests and behavior are absent. The ten evidence/delivery tasks remain unchecked.

### D11 — Verify transitions and trace every requirement

Extend existing Rust suites with deterministic provider shims for behavior
under Brokkr's control and separate dated proof for provider behavior. The
smith's numbered tasks name requirements and trace every existing scenario.
The current five deltas contain 20 requirements and 123 scenarios; the 102
surviving task IDs keep their coverage and evidence dependencies. A parameterized
test may cover several scenarios only when it exercises their independent
outcomes. The topology, ownership, wire, record, provider and recovery axes in
both positions are acceptance dimensions, not a demand for a new test function
per prose scenario. Do not write tests that merely echo implementation or
modify frozen evaluator fixtures.

| Requirements | Suite and distinguishing evidence |
|---|---|
| SR1/SR2 | Runtime `resume_tests`, agent/panel/sequence tests: four work/gate topologies, repeated labels, collision refusal, case switch, per-member chain change, latest incompatible owner, all identity axes, import/local origin and manifest mismatch. Assert actual wire offers and absence. |
| SR3/SR5 | Runtime/provider tests: generated root, assigned creation/confirmation if implemented, child distinction, DSH directory mismatch, unsafe/oversized IDs, held-window death, park/fresh-engine retry, indeterminate non-reexecution, legacy Codex and composite cold migration. |
| SR4 | Protocol `process/tests.rs`, adapter loop, CLI conformance: negotiation, effect/attempt mismatch, duplicate/malformed offers, two starts, cancel/shutdown/EOF, current result door, private context not rendered. |
| AS1/AS2/AS3 | Declaration/packaging and planner tests: captured argv, current class/model/effort, generated fragment versus passthrough, duplicate and last-wins permission/tool/MCP/model/effort controls rejected on cold and resume paths, no ambient cold/gate continuation, nonpersistence and changed CLI/wrapper. Separate installed enforcement/root/accounting observations for every enabled shape. |
| AS4/AS5/LE3 | Adapter/process/runtime sequences: confirmation, conclusive rejection, error then work/delivery, different/missing root followed by clean exit or a valid result still ending failed/indeterminate without an accepted success, post-work failure, failed replacement, watchdog/deadline/cancellation race, classified refusal without Accepted/checkpoints and held-row order. |
| LE1/LE2/LE5 | Every built-in: cold/no offer, supported resume, decline/replacement, exec absence and independent member launch. Validate emitted checkpoints/results at the store; refused append writes nothing; export/import/offline verify agree; v1–v4 compatibility and embedded-byte pins. |
| boundary-record / The seat record carries the boundary as seat-record/v4 | Store version/record tests and runtime `engine/boundary_tests.rs`: all four fences agree at 0.8/0.9/0.10 boundaries and later versions, v5-only fields fail under v4, unstamped historical 0.10.0 rows stay valid, stamped violations fail, the tagged 0.9.0/0.9.1 example and every boundary-stamping scenario remain intact. Published/embedded v1–v4 bytes stay pinned beside v5. |
| LE4 | Only current turns/tools/targets/usage, replay, unknown baseline, rotated/truncated source, completion deduplication and LaneTally capture. Retain transcript caps. |
| PM1/PM2/PM3/PM4 | Both dialects' SDD instruction/rendering/identity suites and returns; completed uncommitted group plus interrupted partial group; missing-edit/failed-check reconciliation; pending workspace/external proof; judges do not mutate. |

Run the proposal's exact commands with `CARGO_BUILD_JOBS=2` and
`RUST_TEST_THREADS=2`: format, clippy with all targets/features and warnings as
errors, workspace tests with all features/locked, and both bundle compiles.
Run the unchanged exact-coverage gate on the host with `TMPDIR=/var/tmp` and
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`. CI, release admission and coverage continue
to consume `rust-nightly-version.txt`. A nested-box skip or missing tool is
pending proof, never a clean gate. No new runs, push, merge, publication or
issue closure is authorized to this seat.

### D12 — F7 repairs the missing boundary-record amendment

Adopt the third analysis finding. The earliest defect was the proposal’s claim
that no existing capability changed, together with the missing delta for the
standing v4 append/dispatch requirement. D10’s prior “no upstream amendment”
claim was too broad: permission to add v5 under LE2 never replaced the living
rule that all engines at or after 0.9.0 select v4. The same 0.10.0 manifest
could not satisfy both that rule and D4.

The proposal and complete MODIFIED boundary-record requirement now reconcile
that conflict before D4, the tasks or production code spend v5. This is the
needed upstream specification repair, performed by the specify owner; no
unresolved triage fault or further upstream requirement is established by F7.
Retain the requirement’s exact title for OpenSpec replacement, all original
boundary scenarios, frozen v1–v4 bytes and the tagged 0.9.0/0.9.1 no-boundary
example. F1/F5’s historical/current-producer distinction remains unchanged, as
do A–H and F2–F4/F6. No provider measurement or enablement follows from F7.

**Alternatives rejected:** changing only implementation tasks leaves the
standing dispatch inconsistent; removing the historical tagged example attacks
the wrong sentence; an ADDED requirement elsewhere leaves the v4 rule standing.
The complete MODIFIED block is required by OpenSpec’s fold semantics.

**Bindings:** D4/D10/D11 and tasks 1.1, 2.3–2.8, 13.3 and 15.6–15.7. The smith
folds all five deltas, preserves boundary-record’s existing provenance and
appends exactly one new entry alongside the four new capability entries.
Revalidate the archive and bidirectional provenance before the delivery commit.
The original specify return corrected the earlier council's Cut A/R4 record
on that evidence. The current council independently accepts the repaired
amendment; D10 records this sitting's dispositions without reopening F7.

## Risks / Trade-offs

- [Required provider evidence is incomplete] → Proceed with known construction
  and tests; keep separate evidence/enablement tasks pending. Unsupported
  declarations do not complete AS1. Measured impossibility returns upstream.
- [Version drift silently changes restrictions] → Compare observed identity to
  pinned evidence, reconstruct the current plan and remeasure changed shapes.
  Prepared Codex disablement blocks delivery until support is re-established.
- [Legacy identity cannot prove composite ancestry] → Cold once and establish
  explicit root evidence; keep old history read-only without relaxing ownership.
- [Names exceed display/address conventions] → Hash exact components for resume,
  validate legacy lookup uniqueness, retain record bounds. Refuse unrepresentable
  evidence; wider addressing defects return to their owning design.
- [Death during pre-work hold loses session facts] → Retain/document the window;
  lost confirmation and creation intent create no offer. Worktree progress
  remains useful independently of session continuity.
- [Undetectable same-host credential/provider-home change] → Preserve local-origin
  checks without claiming account authentication. Fail closed on detectable
  owner rejection; never inspect unrelated homes or credentials for extra proof.
- [v5 costs maintenance; older binaries cannot verify its fields] → Limit additions
  to required facts, share dispatch at all fences, test old rows under v5 and
  retain the matching verifier for new journals.
- [A model ignores markers or trusts stale memory] → Explicit durable recovery
  instructions plus review; tests prove distribution/exercise, not obedience.

## Migration Plan

1. The smith adopts this design for numbered requirement-linked tasks with
   separate unchecked evidence/enablement/delivery obligations. Tasks validation
   and the normal analyze-to-zero loop precede implementation.
2. Add proposed 0056/registry, then v5 contract/embedding/dispatch and tests before
   emitting new fields. Accepted decisions and existing frozen bytes do not move.
3. Add structural contexts/stamps, correlated offers and provider planning/
   confirmation. Enable measured shapes only; keep declarations, packaged copies,
   guides and instruction identities coherent. No private-run migration or global
   provider settings change.
4. Obtain controller evidence and deliver Claude and preserved Codex support.
   DSH support additionally waits for an installed `dsh-headless` release exposing
   D6's exact caller seam; then prove and enable it without changing the minimum.
   Resolve that upstream dependency before claiming completion. Finish tests,
   house validation and specification review. The smith performs the declared
   archive/provenance operation as its final artifact task, folding four new
   capabilities and the modified `boundary-record`, retaining its existing
   provenance and appending this change’s entry. Revalidate the folded change
   and bidirectional provenance before the commit; this design visit does
   not archive.
5. Commit unsigned in repository style. Hand final head/evidence to the controller
   for integration, host exact coverage, remote validation, publication and issue
   closure. External steps stay pending until results exist.

Rollback reverts code/declarations for new runs, never journals. Retain v5
validation for already-written v5 evidence. An active pinned run cannot silently
switch to a reverted adapter/engine; retain manifest mismatch/park behavior and
leave a new-run decision to the operator/controller. Historical version/channel
facts are not current references to replace.

## Open Questions

No scope or safety ruling is left to implementation discretion. D6's missing
Claude/Codex/LaneTally observations are required evidence tasks with explicit
admission/return conditions, not optional questions or claims of support. DSH's
installed headless caller gap is answered by the exact upstream requirement in
D6; it is not an invitation to invent a local route. Evidence requiring
different ancestry, record representation, execution architecture or a smaller
provider minimum returns to the earliest owning artifact.

## Prior design-phase validation

Historical evidence from the original design pass (`a1624a6`), before tasks
and the successor F7 repair. These results are artifact validation, not
implementation, analysis, provider enforcement or full-delivery success.

On 2026-09-09:

- Read `openspec instructions design --change 226-session-resumption --json`
  through the workspace hands; no workflow runner invoked. Adopted proposal
  and deltas were preserved, and only this phase's `design.md` was authored.
- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  Status reports proposal/specs/design done, tasks ready, planning incomplete.
  The requirement-reference check found all 19 requirements in the design;
  the existing 112 scenarios remain intact. This is not a judged analysis or
  implementation-test verdict.
- All five requested Cargo command launches were attempted with
  `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`; the executable is absent, so format,
  clippy, workspace tests and both bundle compiles did not run. No passing
  Rust check is claimed by this seat.
- The unchanged exact-coverage script was invoked with those limits plus
  `TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 because
  `/var/tmp` is absent and `mktemp` could not create its directory. No coverage
  or boundary proof was obtained; controller host validation remains pending.
- Both controller-capture hashes match the proposal. CI, release admission and
  the coverage script still consume `rust-nightly-version.txt`. Frozen paths
  have no diff from the commissioned base. Proposed 0056 remains reserved for
  the smith's decision document before production semantic edits.


## Successor specify reconciliation — F7

D4/D10/D11, new D12 and the migration/fold obligations are reconciled with the
fifth delta. Current artifact-validation results and pending Rust/host proof
are recorded in [proposal.md](proposal.md#successor-specify-validation--f7-2026-09-09),
separately from the original council’s historical validation above. The
successor’s independent judged phases and full delivery remain unclaimed.

## Quota-successor design validation — 2026-09-09

Adopted `226-session-resumption` after this run's clear clarification verdict.
Read the dialect's `design`/`return` instructions and
`openspec instructions design --change 226-session-resumption --json` through
the workspace hands. D1–D10 reconcile both current positions, retaining A–H,
F1–F7, the full provider minimum and proposed 0056's ten planned rulings.
D6 records the newer DSH source interfaces and the still-unestablished headless
caller connection. No workflow runner, archive or live provider probe was run.

- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  Status reports all four planning artifacts done. This is artifact state,
  not a tasks/analysis, implementation, verification or review verdict.
- The OpenSpec delta parser reads 19 ADDED requirements and one MODIFIED
  boundary-record requirement: 20 requirements / 123 scenarios. The five
  deltas, proposal, tasks and change metadata are byte-identical to this
  visit's starting HEAD. All requirements are cited by design and tasks;
  all 102 unique task IDs remain unchecked. D1–D12 and every dialect-required
  design section remain present. No dependent task amendment is needed for
  this reconciliation; the later tasks seat still validates its breakdown.
- With `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`, the five commissioned Cargo
  command launches failed with `ENOENT`: Cargo is absent. Format, clippy,
  workspace tests and both bundle compiles have no passing result here.
- The unchanged coverage script, with the same limits plus `TMPDIR=/var/tmp`
  and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited 1 because `/var/tmp` is
  absent and `mktemp` failed. Host exact coverage and boundary proof remain
  pending. CI, release admission and coverage still read the same
  `rust-nightly-version.txt`; no skipped boundary test is credited as proof.
- `git diff --check` passed. Only this phase's `design.md` changed. Published
  and embedded frozen contracts, production policy, reference, fixtures,
  accepted decisions, production code, adapters/charters and living specs
  match the commissioned base. The three capture hashes were rechecked;
  source evidence and synthetic tests supply no live resume qualification.

## Current council reconciliation validation — 2026-09-09

This visit read the dialect's repository instructions and
`openspec instructions design --change 226-session-resumption --json` through
the workspace hands; it did not invoke a workflow runner. Both current council
positions were read in full and pinned above by their rechecked hashes. D10
adopts or combines every claim that survives repository evidence and rejects
the extra session store, public lifecycle vocabulary, generic provider grammar,
evidence database, protocol version and task renumbering with reasons.

- `openspec validate 226-session-resumption --strict --no-interactive` passes,
  and `openspec status --change 226-session-resumption --json` reports all four
  planning artifacts complete.
- The task record remains 92 complete / 10 pending. This design records the two
  implementation conformance seams for downstream reconciliation without
  changing a checked marker in the wrong phase.
- Exact bare version invocations of `codex`, `claude`, `dsh`,
  `claude-lanetally`, `cargo` and `rustc` each returned command-not-found in the
  workspace hands. The required provider proofs and Rust checks therefore did
  not run here; supplied interface captures and deterministic shims are not
  relabelled as live enforcement.
- The exact host coverage command,
  `env TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh`,
  exited 1 because the boxed workspace has no `/var/tmp`. It supplies no host
  boundary proof, and task 15.5 remains pending.
- `git diff --check` passes. This visit changes only `design.md`; frozen v1–v4
  contracts, fixtures, production policy and reference bytes remain untouched.
  No archive, provider setting, sibling tree, push, merge or issue state changed.

The reconciled design is drafted. Full delivery remains blocked on the exact
DSH upstream caller seam and on the pending provider/host evidence; neither is
converted into a smaller requirement.
