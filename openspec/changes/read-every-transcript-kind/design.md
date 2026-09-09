## Context

Adopted change: **read-every-transcript-kind**, issue #222, at `4e815aa`,
descending from commissioned shipped main
`5bc8cf305aaef9af269866cbf83f094939691399`. See [proposal.md](proposal.md)
for motivation and the three capability deltas for the acceptance contract.

**Disposition: upstream.** This is a council design record, not a completed
design admission. Decisions D1–D9 below fix the architecture that remains
valid. U1–U2 identify requirements that must be amended before the provider
decoders and proposed decision 0055 can be completed coherently. They are
evidence-backed specification findings, not implementation discretion or
deferrable open questions. No tasks or production implementation is authored
on this visit.

The fourth clarification pass at `4e815aa` is clear. Its fourteen previous
answers remain intact: proposal S1–S8, reading R1–R13, command C1–C5 and TUI
T1–T4. In particular, Codex filename identity needs no header; the Codex
identifier language is its engine's own language; and a rejected common
reference is still echoed. None of those three settled findings is reopened.
There is no `returned_from` finding supplied for this design visit.

The shipped seams explain the change:

| Existing seam | Consequence for this design |
|---|---|
| `crates/brokkr-cli/src/ui.rs:167-316` owns `Block`, `Turn`, Claude lookup, parsing and the display cap. | Extract the content model/projector, preserve Claude's supported blocks, and replace the unbounded file read. |
| `crates/brokkr-cli/src/tui.rs:420-451` admits only Claude sessions; its transcript state and watch use a Claude id and length. | Replace eligibility and snapshot state, while retaining the keys and reading overlays. |
| `crates/brokkr-view/src/lib.rs:1959-1971` can retain a legacy flat id without the TUI's provenance guard; `ui.html:978-1010` trusts it. | Participant presentation must use the common local selection result, including recorded-home authority. |
| `crates/brokkr-protocol/src/adapters.rs:956-999,1351-1395,1795-1800` supplies discovery/identifier precedent; `transcript.rs:15,73` clamps recording at 80 characters. | Match the settled compatibility predicates without editing adapters or reconstructing clipped identifiers. |

Decisions 0004/0005, 0009, 0013/0014, 0030, 0032, 0034 and 0042 constrain
the design: deterministic refusal, Rust production, shared pure derivation,
read-only surfaces, same-seat ownership, retained originals, paths-only
journal facts, and separate specification/design/tasks judgments. The
decision registry still ends at 0053 on this worktree. Controller
reservations 0054, 0055 and 0056 remain as commissioned; their absence here
does not imply that sibling work landed.

### Evidence and its limits

Read both complete council positions:
`.forge/design/positions/robustness.md` and
`.forge/design/positions/simplicity.md`. They remain run-local evidence.
The simplicity position identifies its baseline as `f9abdc4`; the chief
reconciles its claims against the adopted `4e815aa` requirements.

The robustness seat reports installed Codex 0.153.4, DSH 0.1.2-rc.1, Claude
Code 2.1.266, and Codex resume help. Those are that seat's reported
measurements, not chief measurements. The chief's workspace exposes
OpenSpec/node/python3 but no codex/dsh/claude/cargo/rustup. All repository
reads and writes use the workspace tool. No provider home, global
configuration, sibling worktree or live model session was accessed.

To check the reported format claims, the chief read the following public
primary sources on 2026-09-09. These are tagged source observations, not
live rollout measurements or proof of an installed binary's exact build.
Unversioned master and failed fetches are not support claims.

| Source | Observed fact and design consequence |
|---|---|
| [Codex rust-v0.153.4 rollout persistence policy](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/rollout/src/policy.rs), `should_persist_event_msg` | Paginated history persists `ItemCompleted`; legacy user/agent/reasoning events have a different persistence policy. Completed items fit the existing Brokkr preference rule (D9). |
| [Codex rust-v0.153.4 item types](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/items.rs), `TurnItem` | Completed items contain typed user/agent messages, readable summary text and tool information. They are not merely lifecycle notifications. |
| [Codex rust-v0.153.4 protocol](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/protocol.rs), `ItemCompletedEvent`, `AgentMessageEvent`, `AgentReasoningEvent` | Completed items carry thread/turn/item identity. The two legacy text event structs have no message/item id. An identity rule cannot be invented for those mirrors. |
| [Codex rust-v0.153.4 response models](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/protocol/src/models.rs), `ResponseItem`, `ReasoningItemReasoningSummary` | Response messages carry ordered content; reasoning summaries carry text; function/custom calls retain argument/input strings and call ids. Raw/encrypted reasoning is not a summary. |
| [DSH dsh-v0.1.2-rc.1 session types](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/core/session/src/types.ts), `SessionEvent`, `SurfaceIntent` | Required unknown events and explicit source citations have semantics that the present requirements do not fully express (U1/U2). Timestamps are epoch milliseconds. |
| [DSH dsh-v0.1.2-rc.1 JSONL format](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/session/session-persistence-jsonl/src/format.ts), `eventLines`, `encodeProvenanceForStorage` | The writer can pack chunks and range-encodes source citations even when packing is disabled. In-memory event types alone do not describe the persisted bytes. |
| [DSH dsh-v0.1.2-rc.1 chunk rows](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/core/session/src/chunk-rows.ts), `ChunkRow`, `decodeStorageRecord` | Packed text/reasoning/tool-argument rows represent multiple events with preserved member boundaries. A row decoder must precede the event projector (U2). |

This evidence does not establish every mirrored Codex producer sequence,
every DSH content variant, or the limits as measured provider maxima.
Proposal S3's remaining precise evidence obligation is carried below; S5's
bounds remain proposed policy. No model experiment is needed.

## Goals / Non-Goals

**Goals:** one bounded, deterministic projection behind the transcript verb
and existing terminal doors; explicit reference, discovery, read and
projection states; preservation of the existing Claude content and browser
response; independently testable ownership and privacy boundaries; a design
whose provider claims identify their evidence.

**Non-Goals:** engine or adapter resumption/launch changes owned by #226,
provider execution, journal accounting changes, joining attempts or seats,
persistent transcript caches, new Codex/DSH browser body routes, binary/media
fetching, provider session reconstruction for execution, or edits to frozen
contracts, policy, reference or fixtures. Showing a provider hint supplies no
launch or sandbox evidence.

## Decisions

### D1 — Reconcile both positions without reducing the commissioned story

The council is resolved by claim, not by averaging the two designs.

| Council claim | Disposition and reason |
|---|---|
| Robustness: staged selection, safe opening, bounded snapshot, pure projection, shared presentation. | **Adopt.** These stages enforce the adopted ownership, cap and diagnostic requirements and expose testable failure boundaries. |
| Simplicity: reuse the existing Turn shape and keep the implementation small. | **Combine.** Preserve the three-field Turn/block wire shape, use two cohesive modules and closed enum dispatch; introduce no provider plugin framework. Rich result metadata is still required. |
| Simplicity: put every parser and I/O operation in one CLI module. | **Reject the placement.** The adopted reading requirement explicitly locates pure derivation in brokkr-view. The crate already depends on serde/serde_json; moving serializable structs adds no dependency cycle or new registry crate. I/O remains in CLI. |
| Simplicity cut 1: canonical-only records and deferred interrupted tails. | **Reject.** It removes R1/R2 and T1's settled behavior and contradicts the continuation's instruction to preserve answers. Missing association evidence goes upstream; it does not authorize dropping retained event/chunk content. |
| Simplicity cut 2: preserve first-match, unbounded, symlink-following Claude lookup. | **Reject.** R11 explicitly accepts the compatibility costs. Enumeration order cannot prove uniqueness, and a second discovery policy would defeat the shared ownership boundary. |
| Simplicity cut 3: remove unknown-record counts and the Claude omission table. | **Reject.** Malformed JSON and valid unknown content are different facts. R3/R9 are settled; silently dropping a future type would erase the diagnostic the requirements promise. U1 concerns DSH's required-event distinction, not removal of diagnostics. |
| Simplicity cut 4: coerce invalid DSH depth as the adapter does. | **Reject.** R8 deliberately refuses invalid ownership evidence. No adapter change is needed or authorized in #222. |
| Simplicity cut 5: drop the header byte bound or hide discovery exhaustion as unreadable. | **Reject.** Reading one line is not a byte bound; one line can be enormous. The declared limits and reason tokens remain observable policy under S5. |
| Simplicity cut 6: retain only one TUI requirement. | **Reject the reduction; retain the keys.** Snapshot replacement can change earlier turn indices even on file growth. Selection invalidation and notices in both doors need the existing explicit obligations. |
| Simplicity: a provenance filter in lib.rs replaces the browser requirement with no JavaScript changes. | **Reject as sufficient.** It fixes one symptom but does not suppress a stale flat id beside a rejected common reference, prove canonical home equality, or supply shared hints. Altering journal-derived compatibility fields is unnecessary for a separate local presentation result. |
| Both: exact recorded homes, no stitching/newest-file guess, per-kind ids, common cap and hint table, C5 echo. | **Adopt unchanged.** These have direct ownership and compatibility evidence. |
| Robustness: DSH required-unknown refusal. | **Adopt as upstream finding U1.** Installing a new refusal only in a parser would contradict the owning requirements. |
| Robustness: modern Codex completed items require a new canonical policy. | **Combine format support; reject the proposed upstream change.** The existing event fallback rule already gives them content visibility and a deterministic preference relative to proven response counterparts; D9 explains why a provider-native canonical label need not change Brokkr's rule. |
| Robustness: source citations suppress only proven DSH chunks. | **Combine with storage evidence in U2.** Citations are stronger than a step number; the persisted range/packed encoding must also be accounted for. |
| Robustness: emit no lossy path spelling; use platform-aware verified handles. | **Adopt the invariant.** A leaf-only no-follow flag is insufficient if an ancestor can change. D3 requires handles for the entire walk and explicit failure when a platform cannot enforce it. |

The simplicity position's delivery-cost concern is real. The remedy is to
finish the evidence/requirement corrections together and retain a small
implementation boundary, not to delete accepted answers. No fresh council
or unrelated fire is started.

### D2 — One pure content model and one local orchestration module

Create `crates/brokkr-view/src/transcript.rs` for serializable content
types, reference selection/lexical validation, record classification,
canonical association, display budgeting, notices and hint derivation.
Re-export the necessary types from the view crate. These functions accept
supplied values and snapshot facts; they have no environment, filesystem,
journal-write, provider, terminal or DOM access.

Create `crates/brokkr-cli/src/transcript.rs` for read-only journal/seat
resolution, effective local legacy home, filesystem discovery/opening,
bounded byte snapshots and command rendering. The TUI and HTTP shell call
this layer. Filesystem facts return to the pure model; the two crates do not
independently decide eligibility or full_session.

Use ordinary structs plus closed enums, not a trait/plugin registry. Keep
`Turn { role, ts, blocks }` and `Block { kind, text }` serialized exactly
as required, with equality available for refresh comparisons. Keep private
source identities beside candidates, never in that public Turn or RunView.
A local result contains selected reference, legacy flag, resolution,
confirmed path, projection/diagnostics, notices and full_session. The CLI's
document adds full run id, exact participant key and requested turn.

Do not flatten every failure into `Option` or infer failure from an empty
vector. A readable empty file, capped empty prefix and unavailable reference
are three distinct states. Selection precedes validation: a common reference
is copied exactly for C5 even when it cannot authorize any read.

The registry's 0055 reservation remains required. This upstream visit
defers its **completed document and registration** until U1–U2 are answered
in their requirements. Writing a nominally complete ruling that contradicts
them would disguise the fault. The returned council must author
`docs/decisions/0055-read-every-transcript-kind.md` with
`Status: proposed` and its index row before implementation, carrying D2–D8
and the repaired provider policies. It must explicitly propose to supersede
only 0032 ruling 4's command-construction binding, as proposal S2 requires.
Accepted decisions are not edited; 0054/0056 are not consumed.

### D3 — Selection and discovery produce a verified handle

Apply reading R7/R8/R11/R12/R13 exactly. In particular, validate the complete
recorded strings, distinguish the 128-character Codex language from the
unchanged 80-character recording clamp, and never search to repair a clipped
id. Resolve eligible legacy Claude provenance only when no common reference
exists. Supply the ambient legacy home as an explicit local input.

Canonicalize the recorded home once; the home itself may be a symlink under
R11. Open that canonical directory as the traversal root. Walk components
relative to held directory handles, refusing symlinks/reparse traversal
below the root, and admit only a regular leaf. Codex's sessions root is
directory depth zero; files in directories at depths zero through six are
eligible. DSH examines only its validated retained root and fixed two-level
project/session layout. Preserve the declared filename predicates.

The 10,000-entry counter spans the whole lookup, including unrelated
examined entries. A provisional match cannot short-circuit uniqueness.
Record candidate count and refusal evidence independently; do not flatten
directory iteration errors. Use the command requirement's reason
precedence: inability to establish uniqueness through I/O is unreadable,
limit exhaustion is discovery-limit, multiple qualifying files are
ambiguous-source, and unsafe candidates cannot supply content. Derive refusals from accumulated typed facts under that requirement, not
from whichever candidate the iterator yielded first. Tests must exercise
competing facts as well as each individual refusal.

Discovery and opening form one operation. Keep the selected open handle
and stable identity; compare discovery/open metadata and refuse replacement.
Check/open **ancestors as well as the leaf**. Unix handle-relative no-follow
directory opens and a nonblocking regular-file check must prevent FIFO
blocking and path swaps. Use an equivalent Windows reparse/handle check;
a path-only metadata check is not proof of that guarantee. Platform APIs
must be isolated in a small tested opening helper. If existing dependencies
cannot express this on a promised platform, return that concrete dependency
or platform decision before weakening the requirement.

Validate the DSH first record from the same safely opened candidate under
its 65,536-byte bound; selected-body reading later starts at byte zero of
that handle. Claude/Codex discovery reads no content. Reading never
reopens a previously checked pathname.

Confirmed paths must be losslessly representable as JSON strings. A
non-UTF-8 discovered path cannot be reported using replacement characters:
treat inability to represent the result as unreadable before path
confirmation, retaining only the validated reference's unresolved hint.
This is an output-encoding failure, not an invented identifier repair.
Do not create directories or substitute ambient provider homes.

### D4 — Snapshot first, associate second, cap displayed turns last

Read at most 33,554,432 source bytes and one probe byte from the verified
handle. Pass bytes plus whether EOF was reached to the pure projector.
The extra byte detects truncation only. At EOF a valid final record without
newline is complete; a final invalid non-newline JSON append is provisional.
Within an over-cap prefix, only terminated records are complete. Invalid
UTF-8 in consumed content is unreadable; a UTF-8/JSON fragment cut by the cap
is excluded as truncation. Complete malformed lines count separately.

Classify all complete retained physical records before final display
capping. Store lightweight candidate/source spans and association metadata,
not a second full JSON tree for every ignored record. Canonical preference
must see the whole bounded prefix and remove only proven fallback
representations. Then retain complete displayed turns until the first whose
final block text would exceed 4,000,000 UTF-8 bytes. Do not skip that turn to
include smaller successors. Counts are source diagnostics and survive
display capping and turn selection.

Use physical source position for order, with a provider-defined member
position only when a physical encoding contains multiple logical events
(U2). Do not sort by timestamps, ordinals, file mtime, turn number or map
iteration. Keep enough private source identity to distinguish pure append
from replacement; equality of rendered words alone cannot prove identity.

Claude extraction preserves its closed omission table, empty string
behavior, tool marker spelling and successful three-field HTTP response.
For Codex, summary extraction uses the measured text members, never a
guessed `summary_text` payload property or raw/encrypted reasoning.
Recorded tool argument strings remain strings; actual JSON values receive
deterministic JSON formatting, never command inference. Non-text media
becomes an inert omission block, with no fetch. Complete provider dispatch
tables remain gated by U1–U2 and S3's remaining association evidence.

### D5 — One result supplies reference states, hints and CLI output

Keep the reading delta's exact full_session table and JSON-string-literal
path/home quoting in one pure helper. Reference rejection yields null
path/hint, no turns, false truncation and zero diagnostics; the recorded
reference itself remains intact. Discovery failure cannot invent a path.
A validated Claude/Codex reference can retain the table's unresolved hint.
Read failure after safe path confirmation keeps that path and its hint
while returning no turns.

The new clap command uses inspect's read-only journal/world resolver.
Require run and seat; exact participant key wins, otherwise only a unique
exact label matches. Return all matching keys for an ambiguous label.
A panel parent has no implicit child transcript. Usage/run/seat errors
precede the transcript document and leave stdout empty.

Derive the full bounded projection before applying an optional positive
u64 turn index. Keep the original one-based position and all whole-source
metadata. Only a successful projection can produce turn-not-retained.
JSON serialization is direct from typed local data under
`brokkr.transcript/v1`; text and TUI reuse rendering helpers and the same
notice ordering. Terminal Safe applies to every untrusted display string;
JSON retains escaped original values. No explicit transcript body enters
inspect/seats/watch/export/dossier models.

The existing twelve unavailable tokens stay authoritative until the
owning command requirement resolves U1. A new semantic-refusal token must
not appear only in Rust or in this design's examples.

### D6 — Refresh replaces a snapshot and invalidates its subject atomically

Replace the TUI's Claude-only ask/result with a request identified by realm
journal, full run, participant key, and complete kind/home/locator.
Retain its resolved source identity with the snapshot. Only the selected
participant is read, at existing refresh opportunities; do not add a fleet
transcript cache or an independent background scanner.

Re-resolve active participants even without a journal-head or length change.
This catches first appearance, incomplete headers, same-size replacement,
disappearance and duplicate candidates. Perform a final read at conclusion,
then stop automatic polling; explicit refresh still resolves again.

Replace unavailable/readable state atomically. Preserve the cursor only
when identity is unchanged and the prior turns **and their source
identities** form an unchanged prefix. On replacement, removal, reordering,
shrink or identity change, clear the selected turn and close its overlay
before presenting new indices. Notices changing alone do not retarget
content. Empty successful projections still open their whole-session
explanation. Keep existing navigation and scrolling; number rendered turns
from one and include shared notices in both doors.

A file-length-only cache is rejected because growth can replace fallback
turns and equal-length files can contain different evidence.

### D7 — Browser participant presentation is a separate local request

Add a loopback GET presentation endpoint keyed by full run and encoded
participant key, separate from `/api/view/<id>`. Use URL component encoding
and validate decoded selectors server-side; no raw key interpolation.
Return only the common reference, eligibility, unavailable
token/explanation, shared hint and Claude drill id/home-eligibility facts.
Discard any content projection at this transport boundary; do not widen
RunView, inspect/watch JSON or the Claude body envelope.

The browser renders that result with textContent. The client hex guard is
only a second check on an already admitted Claude drill. Offer the id-only
route only when the effective Claude home is canonically the local projects
home. On a different or unprovable home, render the specified home
explanation and checkpoint fallback. Codex/DSH keep their shared hint and
fallback without Claude body requests.

Retain `/api/session/<id>` and `/sse/session/<id>` as independent local
Claude selectors with the specified 404 envelopes and successful shapes.
Each SSE poll redoes safe unique discovery. Lost admission closes the
stream, clears the client's cached body and triggers fresh presentation/API
resolution; it cannot keep stale prose or silently restart an old path.
Changing participant/reference cancels its prior watch and outstanding
request generation before applying any new response.

This is a small separate transport for existing required presentation
semantics, not a new general transcript browser API.

### D8 — Verify the shared boundaries, then judge the full story

Use the existing crate test suites with synthetic, test-owned homes.
Do not copy operator transcripts or edit frozen fixtures. Test the
requirement's observable outcome through the shared implementation, not
a parser-shaped mock repeated in each renderer.

| Layer | Required proof |
|---|---|
| Pure model/projector | Reference precedence and C5 serialization; complete per-kind id boundaries; Claude's exact fixture counts; source order and absent stamps; every supported family; proven and unproven associations; partial JSON, invalid UTF-8, both caps below/at/above; exact notices and hints. |
| Local filesystem | Custom-home authority, legacy provenance, unique vs duplicate sources, complete discovery bounds, DSH invalid/delegated headers, symlink/reparse ancestors and leaves, replacement races, FIFO/nonregular refusal, iteration/read failures and lossless path reporting. |
| CLI | Existing world/run precedence, ambiguous labels vs exact keys, parent/leaf ownership, positive-u64 parsing, selected turn equality, readable/empty/truncated/unavailable exit codes and stdout/stderr shapes. |
| TUI | All kinds in pane/both doors; zero-turn explanations; pure append vs canonical replacement; same-size source replacement; unavailable transitions; final/manual refresh; source identity and turn selection never drift. |
| HTTP/browser | Shared eligibility and hints; custom-home mismatch; legacy Codex and rejected common references never drill; unchanged successful Claude envelope; exact 404s and SSE closure/cache invalidation. |
| Privacy | Sentinel prose/tool data visible only in explicit local reads; journal count/hash and retained bytes unchanged; provider-spawn paths never invoked; no persistent body cache. |

U1–U2 must add their owning scenarios before this matrix becomes a final
task breakdown. Implementation must run format, clippy, workspace tests,
both bundles and the unchanged exact-coverage gate with
CARGO_BUILD_JOBS=2 and RUST_TEST_THREADS=2. Host exact coverage requires
TMPDIR=/var/tmp and BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 with the shared
rust-nightly-version.txt pin. Nested-sandbox skips are not host proof.

### D9 — Codex completed items fit the adopted preference rule

Adopt robustness's measured format support without its proposed new
canonical ranking. A completed TurnItem is substantive provider content,
but that description does not override Brokkr's explicitly chosen
response-item preference. The existing requirement already says how to
read a content-bearing event alone and how to replace a proven counterpart.
Calling every completed item lifecycle metadata would violate that rule;
recognizing it as event content satisfies it.

Decode recognized content within `event_msg` / `item_completed`, including
its typed user/agent messages, summary reasoning and tool information.
The nested TurnItem type spellings come from the tagged source, not from
the separate `codex exec --json` stdout format. Preserve the enclosing
record's source position and timestamp. When no complete recognized
response counterpart is proven in the bounded prefix, emit that content.
When one is proven, prefer the response at its own source position under
R1. File history mode is format context, never a new discovery identity
header or permission to omit all events.

Add tests for a completed-item-only ruling, legacy and completed-item
representations beside proven responses, incomplete/capped counterparts,
unassociated repeated text, and live canonical replacement. These are
concrete instances of the existing scenarios, not a changed canonical
policy or a reduced story. Command/TUI inherit the same ordering.

S3's remaining producer-backed association evidence is still due before
claiming duplicate-free support. The measured legacy agent-message and
reasoning event payloads lack ids; their type definitions alone do not
prove an unambiguous item position. The returned council must document a
tagged producer ordering proof or controller-supplied bounded redacted
paired example with that proof. Missing evidence does not prove an
association impossible, and inventing ids in test records proves nothing.
Keep unassociated content under R1. This is the existing named evidence
obligation, not a third new upstream specification finding. No adapter
or #226 resumption change is involved.

### U1 — Return DSH unknown-event policy to transcript-reading/command

**Owning fault:** reading “Partial records and read failures remain
distinguishable” and R3, plus command C1/its closed reason vocabulary,
require unknown valid records to remain a successful counted omission
without distinguishing DSH events whose omission is not declared safe.

**Evidence:** the tagged DSH SessionEvent contract above assigns omission
safety to `ignorable: true`; a required unknown event can affect later
interpretation. This is new provider evidence, not a re-raising of R3 merely
because the earlier box lacked a CLI. Although Brokkr is not reconstructing
execution history, it does interpret and suppress content based on event
relationships. A generic count cannot establish that an unknown event left
those relationships valid.

**Recommended upstream answer:** preserve counted omissions for Claude,
Codex and safely ignorable DSH unknown events. For a DSH event with an
unknown type and no valid ignorable marker, refuse the content projection
as `unsupported-format`, distinct from I/O/UTF-8 unreadable. Keep the
confirmed path/hint and selected reference, expose no turns, and make
whole/selected CLI reads exit one. Pin whether/how whole-prefix counts,
notices and source truncation are retained, including collisions with
partial/malformed input, before publishing the token.

The alternative is an explicitly specified raw, lossy DSH interpretation
that makes no reconstruction claim. It is not chosen here: it would need
its own truthful output/association semantics in the owning requirement
and proposed 0055. Importing every provider replay check is also not the
answer; the rule must target this reader's interpreted content.

**Required owner scenarios:** required unknown after visible content
returns no prose; the same unknown marked ignorable produces the counted
notice and retains recognized content; selection does not evade the
refusal; refreshing from readable to refused clears both TUI doors.
Amend proposal, reading and command, then dependent TUI scenarios and
0055. Preserve R3's distinction between malformed and valid unknown data.

### U2 — Return DSH physical records and citation scope to transcript-reading

**Owning fault:** “One transcript derivation” treats a displayed turn as
one content-bearing source record, while “DSH sessions expose assembled or
provisional content once” describes individual chunk events and step-based
replacement. It leaves the supported persisted packed representation and
its citation encoding unspecified, and can erase uncited chunks merely
because a readable assembly shares the step.

**Evidence:** the tagged JSONL writer above packs event batches and
range-encodes provenance. The chunk codec has text-chunks, reasoning-chunks
and tool-call-chunks rows, each preserving individual fragment boundaries.
The session type distinguishes an explicit empty citation from an absent
citation; sharing a step does not supply that missing evidence.

**Recommended upstream answer:** distinguish complete physical JSONL rows
from decoded logical events. Decode supported packed rows lazily within
the source prefix, retain member order and recorded time reconstruction,
then project each readable event as its own turn. Tool-argument fragments
remain fragments, never completed calls. Canonical assembly suppresses
only source events whose relationship is proved; a step pair scopes
citations rather than manufacturing them. Decode persisted citation
ranges without allocating an attacker-sized expanded range: match against
bounded observed event identities.

**Required owner scenarios:** packed and equivalent unpacked interrupted
text/reasoning yield identical turns; an incomplete/over-cap physical row
is never partly decoded; a malformed packed row has a pinned refusal/count
outcome; ranged citations suppress exactly their members; explicit empty,
absent and partial citations leave uncited readable chunks visible even
in the same step. Clarify whether unsupported content counts per physical
row or logical event (recommend physical row, preserving the current
at-most-once-per-source-record diagnostic). Pin cap/turn numbering after
decoding and test replacement selection invalidation through command/TUI.

Do not silently concatenate a packed row, count it as ordinary unknown
content, or switch to the provider's model-visible compacted surface.
Those alternatives would respectively change fragment boundaries, hide
retained words, or replace the commissioned audit order. Amend the owning
reading definitions/scenarios and all dependent numbering/count rules;
0055 must record the agreed distinction.

## Risks / Trade-offs

- **[Provider format drift]** → Pin measured supported encodings and
  association evidence; resolve U1–U2 before decoder admission. Do not
  replace a supported retained format with an empty success.
- **[Canonical suppression loses distinct prose]** → Require recorded
  identity or proved producer position, resolve association before display
  capping, and preserve unassociated content. Keep source identity private.
- **[Path swaps or same-size replacement]** → Traverse with verified
  handles and re-resolve snapshots; path metadata alone is insufficient.
- **[Bounded input still has allocation/refresh cost]** → Avoid whole-file
  JSON retention and expansion of citation ranges; read only the selected
  participant. The chosen limits bound work, not a measured latency promise.
- **[Claude compatibility tightening]** → Carry the declared identifier,
  uniqueness, discovery, symlink and source-cap costs into proposed 0055 and
  migration guidance. Do not conceal them as a refactor.
- **[More local metadata leaks prose]** → Keep the read result separate
  from journal-derived view models; the browser presentation endpoint
  serializes only metadata and every terminal string passes Safe.
- **[A hint is mistaken for verified resumption]** → Keep it inert,
  preserve recorded-home wording, and leave launch/sandbox proofs with
  controller evidence for #226.

## Migration Plan

1. Return U1–U2 to their owning requirements in dependency order:
   proposal, reading, command/TUI consequences. Preserve all earlier answers
   and add the new evidence-backed scenarios. Clarification remains the
   independent judgment; the chief does not manufacture a clear result.
2. Reconcile the returned design with those scenarios, finish the provider
   mapping and race-safe opening plan, then author/register only reserved
   proposed 0055. Its enforcement bindings must name the projector,
   filesystem, surface and privacy tests. No acceptance is claimed.
3. The tasks office decomposes the admitted design; analysis judges it.
   Implementation then extracts the Claude projector, adds the two
   provider decoders/resolver and command, connects TUI/browser, extends
   the proving suites and read-surfaces guide, and completes local checks.
4. No stored data migration is required. Readers consume retained files;
   they do not move, rewrite, backfill or delete them. Explain how an
   operator can inspect an original, resolve duplicate placement, fit
   discovery bounds, or replace an actual below-home symlink with an owned
   regular entry. There is no reader home override or automatic
   reorganization.
5. Before publication, controller integration owns shared-file overlap,
   full host coverage and review of the final head. After publication any
   structural change to brokkr.transcript/v1 needs its explicit version
   change. Rolling back the binary restores the previous reader, without
   changing retained bytes or journal references; the old Codex/DSH
   unreadability and older Claude lookup behavior would return.

No push, merge, issue closure, completed-run publication or new run belongs
to this seat.

## Open Questions

None is safely deferred as ordinary design discretion. U1–U2 and the
named S3 association evidence are admission blockers recorded under
Decisions. The implementation details that remain must be proved by the
tasks/analysis and validation phases; they are not permission to weaken
the current requirements.

## Validation of this visit

Read the dialect manifest and design/return instructions, then rendered
`openspec instructions design --change read-every-transcript-kind --json`.
It names design.md; no workflow runner was invoked. This upstream visit
commits only that declared artifact. Proposed 0055 is intentionally not
claimed complete, and tasks/implementation remain unauthored.

The five commissioned Cargo checks were attempted with both concurrency
limits and each was unavailable (exit 127: cargo absent). The unchanged
exact-coverage script, with TMPDIR=/var/tmp and
BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1, exited one at mktemp because /var/tmp is
absent in this box. No coverage or boundary proof ran. Command records are
in `.forge/design/read-every-transcript-kind-chief-validation.json`;
host proof remains pending.

Strict OpenSpec validation passed after authoring. Artifact status reports
proposal/specs/design present and tasks ready by file existence, while
planning and apply completion remain false. This structural status cannot
resolve U1–U2 or override this upstream result. The frozen directories and
all decision files remain unchanged against the commissioned base. Final
git scope/whitespace checks and structural output are recorded with the
visit evidence; no unavailable delivery check is counted as passing.
