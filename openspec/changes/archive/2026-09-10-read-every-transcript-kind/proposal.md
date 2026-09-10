# Change: Read every retained transcript kind (#222)

## Why

Decision 0032 makes every driver retain and name the operator's transcript,
but the TUI reads only Claude sessions: Codex rulings and DSH conversations
remain inaccessible behind their recorded references. The operator's
2026-09-09 commission prioritizes the complete transcript reader and CLI,
through specification, clarification, council design, tasks, analysis,
implementation and review.

## What Changes

- Derive Claude session JSONL, Codex rollout JSONL and DSH session JSONL
  into one ordered, serializable `Turn` shape in `brokkr-view`; keep file
  access in the local read layer. Preserve Claude's existing content
  projection and add Codex and DSH messages, readable reasoning, tool calls
  and tool outputs. Read recognized Codex event content and DSH chunks when
  their canonical content is absent, then replace proven echoes when the
  canonical record arrives; retain file order within each bounded snapshot.
  Decode DSH packed rows into their individual events and preserve fragment
  boundaries. Assembled messages replace only chunks proved by recorded
  citations, including stored citation ranges, within the same turn/step.
- Resolve the selected participant's recorded kind, locator and home to
  its own retained file. Distinguish missing, invalid, ambiguous and
  unreadable references. Admit DSH content only under an opening session
  header with numeric version zero; absent, nonnumeric or foreign versions
  return `unsupported-format` after safe unique ownership is established,
  retaining its path/hint and observed source truncation with zero diagnostic
  counts because event decoding never began. Count root candidates before
  version admission, so current/foreign candidates together remain ambiguous.
  Within admitted version-zero sources, unknown required events or invalid
  packed-row/citation encodings also refuse as `unsupported-format`, with no
  prose and with the confirmed path and bounded-source diagnostics retained;
  explicitly ignorable unknown events remain counted omissions. Never
  substitute another seat, a delegated DSH session or the newest file in a provider home. Codex uses the shipped
  whole-token filename identity without a required content header; its
  provider-specific id guard accepts the engine's existing language.
- Add `brokkr transcript --run <selector> --seat <label-or-key>` with
  optional one-based `--turn <n>` and `--json`, using the read surfaces'
  existing journal and realm selection and the TUI's same derivation.
  JSON preserves a present common reference's recorded fields even when the
  reader rejects it, separately from validation, resolved paths and hints.
- Make the TUI transcript pane, open-turn door and open-whole door work
  for all three readable kinds, including live file appearance/growth.
  Show the correct full-session location and measured command spelling:
  Claude's `claude --resume <id>`, Codex's rollout path and
  `codex exec resume <thread>`, and DSH's session file without inventing a
  resume command. These are display hints and execute nothing.
- Apply the existing 4,000,000-byte displayed-block budget to every kind,
  with bounded source reads and discovery and an explicit truncation
  notice shared by text, JSON and both TUI doors. Preserve journal privacy,
  transcript retention, terminal sanitization and accepted contracts. Count
  unrecognized records separately from malformed lines, using the closed Claude
  omission list in the reading delta so intentional omissions stay quiet and
  new record/block kinds remain visible as counted omissions. Retire both
  shipped Claude truncation suffixes, the TUI's
  ` — claude --resume carries the rest` and the browser's
  ` — resume the session for the rest`; every kind uses exactly
  `transcript truncated (size cap)`.
- **BREAKING**: require a leading hexadecimal character in Claude ids
  everywhere they are read or used in a convenience command, including the
  existing Claude browser drill and its client guard. Leading-hyphen ids that
  the old Claude guard admitted become invalid. Codex keeps the engine's
  existing 1–128 ASCII alphanumeric-or-dash language with an alphanumeric
  first character; the reader does not impose Claude's hex/64-character
  restriction or change the protocol's 80-character recorded-locator clamp.
- **BREAKING**: Claude lookup now refuses duplicate candidates
  (`ambiguous-source`), discovery beyond 10,000 entries (`discovery-limit`),
  and symlink project entries or transcript files below the canonical home
  (`unsafe-path` when no safe unique source exists), replacing first-match,
  unbounded enumeration and symlink following. These refusals apply to the
  TUI and both existing Claude HTTP routes; the routes return 404 for the new
  lookup refusals. The new 32 MiB source cap can truncate Claude content that
  the former displayed-text-only limit served in full. Proposed 0055 must
  name these compatibility costs, with their limits and migration guidance.
- Correct browser participant eligibility and convenience text through the
  shared Rust derivation. An explicit non-Claude legacy provider cannot
  trigger a Claude drill; every displayed `full_session` is the shared value.
  The id-only HTTP routes remain journal-independent Claude lookups. A
  participant whose recorded Claude home differs from the routes' local
  projects home keeps its shared hint but gets no drill into the wrong home.
  Losing an admitted drill's lookup admission, without any participant,
  reference or journal change, clears the page's cached body and growth
  watch and shows the current shared refusal instead of stale prose; a
  superseded in-flight response cannot restore it. A still-admitted source
  is instead redisplayed and rewatched from that fresh result, and the page
  re-checks its selected participant's presentation on a recurring occasion
  that needs no journal change, re-selection or other operator action, so
  growth survives a transport drop and a refusal clears itself once the
  ambiguity is resolved. This replaces the shipped page's reliance on
  `EventSource` automatic reconnection, which would resume a watch the
  shared lookup now refuses: browser growth recovery is preserved but
  follows a presentation round trip and that re-check cadence, bounded to
  one automatic watch opening per participant between re-checks. Admission
  is the presentation's own discovery-shaped state. It reports `unreadable`
  when directory I/O or the bounded DSH opening-header I/O/UTF-8 check prevents
  discovery from establishing a unique source; after a source is admitted,
  body unreadability remains a body outcome. A refused transcript body — the
  route reports read and lookup failures with one envelope — silences its
  source until the next re-check instead of driving a request
  loop. Every id-only request and growth watch additionally requires an
  eligible drill, so an admitted Codex, DSH or foreign-home source is
  displayed and re-checked without ever being drilled, and a successful
  body — including a readable zero-turn one — ends the deferred repair, so
  an admitted empty source is fetched once rather than once per re-check.
  Codex/DSH browser transcript bodies remain outside scope.
- Require reserved decision **0055**, with status **proposed**, for the
  new local reading, selection, output and limit semantics. The council
  design must author it and register it before implementation, explicitly
  proposing the replacement of decision 0032 ruling 4's command-construction
  binding stated in S2, the successor's identity/JSON resolutions in S8, and
  the returned DSH format and refusal resolutions in S9, and the DSH
  header-version admission and failure-state resolution in S11;
  only the operator can accept it. No semantic production change is made in
  this specification phase.

## Capabilities

### New Capabilities

- `transcript-reading`: local identity and ownership checks, per-kind
  content projection into ordered turns, bounded reads, diagnostic counts,
  deterministic full-session information and privacy.
- `transcript-command`: run/seat/turn selection, text and versioned JSON
  output, and explicit command failures for unavailable transcripts.
- `transcript-tui`: all-kind transcript navigation, live refresh,
  rendering the shared full-session information and notices.

### Modified Capabilities

None. The current standing capabilities concern named execution boundaries;
this change preserves their requirements and introduces transcript reading
as separate capabilities.

## Impact

Production work belongs in `crates/brokkr-view` and the CLI's local reader,
argument handling, rendering and TUI shell. Existing Claude browser drills
must consume the shared Claude derivation and keep their successful
`session_id`/`turns`/`truncated` response shape and supported Claude content.
The browser's selected-participant eligibility, shared hints, identifier
guard, lookup refusals, growth-watch recovery and truncation text change as
declared above. Browser
presentation metadata remains separate from existing inspect/seats/watch JSON
and contains no transcript prose in journal-derived models. Adding Codex/DSH
browser transcript routes is outside this commission.
Tests belong in the existing view, CLI, TUI and reader suites, with synthetic
files in test-owned
homes. The read-surfaces guide and proposed decision/index accompany the
implementation. No new production language or provider dependency is needed.

The base is shipped main `5bc8cf305aaef9af269866cbf83f094939691399`.
Issue #226 owns engine/adapter resumption and launch evidence; this change
neither edits those semantics nor assumes sibling changes. Frozen contracts,
`policy/phase-machine.json`, `policy/schemas/`, `reference/` and `fixtures/`
remain byte-for-byte unchanged. No journal or seat-record version changes
are needed for an operator-local transcript document.

## Decisions

### S1 — The full story stays in the phase machine

Triage already returned `design`; specify authors the proposal and deltas
only. Decision 0042 and the rendered dialect assign clarification, council
design, task breakdown and analysis to subsequent seats. Producing their
artifacts or claiming their judgments here would bypass the commissioned
phases. The change identifier is `read-every-transcript-kind` throughout;
this phase does not archive it or implement production code.

### S2 — Decision 0032 remains accepted; 0055 is required for new semantics

The issue's earlier suggestion of "no new decision" is declined because the
later operator commission and house rules explicitly require a proposed
decision for semantic changes. Decision 0032 still owns the common reference,
retention and paths-only journal law. Proposed 0055 supplements local reading
and explicitly proposes to supersede **only ruling 4's enforcement binding
for resume-command construction**, whose accepted text limits it to Claude.
The replacement binding to carry into 0055 is:

> The shared local transcript derivation constructs informational full-session
> lines by validated kind: `claude-session` names `claude --resume <id>`;
> `codex-thread` names `codex exec resume <thread>`, the recorded home and
> either the confirmed rollout path or explicit rollout unavailability;
> `dsh-session` names only a confirmed session file and no command. TUI,
> transcript CLI and browser participant presentation consume that same result.
> No other kind borrows either command, and displaying a hint executes nothing.

The common transcript cell, ownership, retention, journal privacy and decision
0030's same-seat/sandbox bindings remain unchanged. Accepted 0032 is not
edited or silently reinterpreted; this is a proposed replacement for the
operator to rule on.
The registry on the commissioned base ends at 0053. The controller reserves
0054 for DSH Git repair, 0055 for #222 and 0056 for #226; those reservations
are not evidence that either sibling has landed. The council must preserve
these reservations and register only proposed 0055 for this change.

### S3 — Evidence determines formats; model experiments are unnecessary

Repository evidence: `crates/brokkr-cli/src/ui.rs` defines `Turn`, Claude
lookup and the 4,000,000-byte block budget; `tui.rs::claude_session` gates the
pane and both doors. `crates/brokkr-protocol/src/adapters.rs` documents and
tests Codex `type`/`payload` rollout envelopes, identity-based discovery, and
DSH's retained root, depth-zero session header and assembled event stream.
Decisions 0013, 0014, 0030, 0032 and 0034 supply the read-only, ownership,
resume and privacy constraints. This is source evidence, not a live reader
measurement. The source and decision 0030 record the Codex resume spelling;
no hint here asserts that resumption is safe or available under a seat's
sandbox without #226's independently owned checks.

On 2026-09-09, the workspace's PATH exposes OpenSpec 1.12.0, but no `codex`,
`dsh`, `claude`, `cargo` or `rustup`. Installed provider help and full content
schemas cannot be re-measured in this seat. Council design must record
bounded installed help/source evidence for content variants and hint syntax,
or identify the precise missing proof for controller handoff. In particular,
the missing evidence is a versioned, redacted Codex rollout/source definition
showing `event_msg` content alone and beside `response_item` with their
message/call association, and a DSH source definition or retained step with
`assistant/chunk` payloads, turn/step association and its later assembled
message. Design must map those facts to the fallback scenarios; textual
equality or speculative payload pointers are not evidence of identity. This
does not justify omitting Codex/DSH reading, weakening acceptance, inventing formats,
changing global settings, or starting paid model experiments.

### S4 — Validation evidence is scoped to what ran

On the first specify visit,
`openspec validate read-every-transcript-kind --strict --no-interactive`
passed. `openspec status --change read-every-transcript-kind --json` reports
proposal and specs done, design ready and tasks blocked on design. These are
this phase's structural checks; clarification and analysis remain independent
judgments. Local
format, clippy, workspace tests and both bundle compilations were attempted
with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`: all were unavailable
because `cargo` is absent. The unchanged exact-coverage script was attempted
with those limits, `TMPDIR=/var/tmp` and
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it refused because `/var/tmp` is absent
in this box. These are pending checks, never passing evidence. Implementation
must extend the proving suites and run all commissioned commands. The
controller owns final host exact coverage with those variables, remote CI,
publication and integration; no seat can replace host boundary proof with
skipped tests inside a nested sandbox.

On the returned specify visit, the strict OpenSpec validation passed again
and status still reports only proposal/specs done. The same five Cargo
commands were attempted with both concurrency limits and remained unavailable
because `cargo` is absent. Exact coverage was attempted unchanged with
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; `mktemp` refused the
absent `/var/tmp` directory before any coverage proof ran. The command results
are recorded in `.forge/specify/read-every-transcript-kind-validation.json`.
These pending delivery checks are not prerequisites fabricated for drafting,
and drafting is not a claim of implemented or fully validated behavior.

On the second returned specify visit, strict OpenSpec validation passed;
proposal/specs remain the only completed artifacts. All five commissioned
Cargo commands were attempted with both concurrency limits and exited 127
because `cargo` is absent. Exact coverage exited 1 at `mktemp` because
`/var/tmp` is absent, with the required temporary root and boundary-evidence
flag supplied. This visit's separate evidence is
`.forge/specify/read-every-transcript-kind-second-return-validation.json`;
the earlier visit's record is preserved. Host proof remains pending.

On the successor adoption, strict OpenSpec validation and delta parsing
passed with all 20 requirements, and only proposal/specs report done.
All five commissioned Cargo commands were attempted with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2` and exited 127 (`cargo` absent).
Unchanged exact coverage was attempted with those limits,
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at
`mktemp` because `/var/tmp` is absent. Separate evidence is
`.forge/specify/read-every-transcript-kind-successor-validation.json`.
`git diff --check` passed; frozen files and accepted decisions are unchanged
from the commissioned base. Provider measurements, Rust delivery checks and
external host boundary proof remain pending; no skipped or unavailable check
is counted as passing evidence.

On the return from design at `6ece1ea`, strict OpenSpec validation and delta
parsing passed with all 20 requirements and 152 scenarios. All 125 prior
scenario names remain; 27 new scenarios answer U1/U2 and their CLI/TUI
consequences. Status reports proposal/specs/design present and tasks ready
by file existence, with planning and apply completion false; it does not
admit the historical returned design. All five commissioned Cargo commands
were attempted with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2` and exited
127 because Cargo is absent. The unchanged exact-coverage gate, with
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited 1 at
`mktemp` because that directory is absent. No Rust, coverage or boundary
proof ran. The command evidence is
`.forge/specify/read-every-transcript-kind-design-return-validation.json`;
whitespace, frozen-byte, accepted-decision and unchanged-coverage-script
checks passed. Final host and delivery proof remain pending with the
controller; the specification audit is not a replacement for those checks.

### S5 — Extra bounds are proposed policy, not provider measurements

The existing 4,000,000-byte block budget is preserved. The new 32 MiB source
budget bounds memory and input work even for ignored records; it leaves room
for JSON framing and metadata beyond the displayed-text budget. Six Codex
directory levels follow the existing adapter search bound. The 10,000-entry
discovery and 65,536-byte DSH first-record header bounds make a lookup finite;
they are proposed limits, not measured maxima of provider homes or headers.
A refusal at a limit stays explicit, and the retained original remains
untouched.
Council design must test these choices against installed format evidence and
return to this specification if a normal supported record cannot fit.

Selection, malformed-record handling and omission choices are answered in
the owning capability scenarios. Synthetic parser and filesystem cases must
be added under the existing crate test suites; the frozen evaluator fixtures
are never regenerated or repurposed for transcript examples.

### S6 — Return from clarify: eight findings answered in their owning scenarios

Adopted the existing change at `10f3c3c`; the earlier triage framing is not at
fault. All eight findings in
`.forge/clarify/read-every-transcript-kind-ambiguities.md` identify choices this
specification owed. The return amends the proposal first and then the three
deltas; no design or tasks artifact exists yet to revise. The owning
capabilities' `## Decisions` record reasons, and their scenarios bind the
observable answers:

| Finding | Resolution and owning scenario |
|---|---|
| 1 — Codex event-only content | Require fallback projection and replacement only for proven associations; reading: "A Codex event-only snapshot is readable" and "A late canonical record replaces its event fallback". |
| 2 — DSH chunks without an assembled step | Project readable chunks until the step is assembled; reading: "An interrupted DSH step retains its chunks"; TUI: "Assembly replaces chunks while the journal is unchanged". |
| 3 — Unrecognized content versus empty | Add `unrecognized_records` and a shared counted notice; reading: "Unrecognized records cannot masquerade as an empty session"; command: "JSON reports unknown records independently of empty and malformed". |
| 4 — 0032 ruling 4 | Explicitly propose its replacement binding in 0055, as S2 states; reading: "The proposed convenience binding remains kind-specific". |
| 5 — Unresolved full-session line | Reading owns deterministic strings/nulls for all kinds, consumed by both renderers; command: "A missing Codex rollout has a fixed full-session value". |
| 6 — Claude truncation suffix | Retire it on every surface; TUI: "Claude and Codex use the identical notice". |
| 7 — Leading-hyphen identifiers | Tighten the shared Claude Rust guard and the browser's copy together; reading: "The browser drill rejects a leading-hyphen id". |
| 8 — Invalid DSH depth | Deliberately require valid ownership evidence even where the old adapter coerced it to zero; reading: "A driver-folded invalid DSH depth is explicitly refused". |

These answers do not claim new installed-provider measurements or bypass the
independent clarification and analysis judgments. Design must return upstream
with evidence if a measured format cannot satisfy these behaviors.

### S7 — Second return from clarify: three findings resolved explicitly

Adopted `read-every-transcript-kind` at `604825d` and validated it before
amendment. The three findings in
`.forge/clarify/read-every-transcript-kind-ambiguities-second-pass.md` concern
this specification's own missing rules; no earlier triage artifact or
accepted decision needs repair. The proposal is revised first, then reading,
command and TUI deltas. Existing first-pass answers remain in force.

| Finding | Resolution and owning scenario |
|---|---|
| 1 — Claude's known omissions | Reading R9 enumerates uncounted record/block kinds and missing/blank fields; "The shipped Claude projection fixture has fixed diagnostic counts" fixes `skipped_lines: 1` and `unrecognized_records: 0`. Command and TUI scenarios preserve those counts on selection and both doors. |
| 2 — Browser eligibility and hint ownership | Reading R10 requires the shared Rust reference/hint result, suppresses non-Claude legacy drills and removes the holder sentence. "A legacy Codex participant never starts a Claude browser drill" states the page behavior and the independent direct endpoint's 200/404 behavior. Recorded custom homes cannot silently use the ambient home. |
| 3 — Claude lookup compatibility | Reading R11 adopts all three narrowings as explicitly `BREAKING`, fixes API/SSE admission at 404 for each new refusal, and states remediation without changing retained evidence. "Claude browser lookup refusals have fixed HTTP responses" covers all three classes; CLI/TUI scenarios preserve the specific refusal and shared Claude hint. |

The closed Claude omission list is a proposed classification policy, not a
claim of a measured live record census. Installed-provider content evidence
remains pending as S3 states. Council design must carry these classification,
browser and compatibility choices into proposed 0055; none silently rewrites
accepted 0032 or borrows #226's work.

### S8 — Successor adoption resolves the third clarify pass

Adopted the committed change at `f9abdc4` for successor run
`close-issue-222-a-transcript-rea-391bcdc7`; strict validation passed before
amendment. The three findings in
`.forge/clarify/read-every-transcript-kind-ambiguities-third-pass.md` are
specification defects, not defects in the commissioned scope or triage.
All eleven earlier answers in S6/S7 and R1–R11, C1–C4 and T1–T3 remain in
force. This amendment removes two unsupported Codex restrictions and closes
the JSON rule; it keeps the full #222 story and every later commissioned
phase. Proposal precedes the amended reading, command and TUI deltas.

| Finding | Resolution and owning scenario |
|---|---|
| 1 — Undefined Codex header gate | Reading R12 selects by the shipped filename-token rule and removes content-header gating entirely, including both absence and conflict vetoes. "A Codex rollout needs no session header" and "Codex payload ids do not select or veto a file" fix the outcomes; this is compatibility with shipped identity discovery, not an invented provider header. |
| 2 — Codex ids narrowed to Claude's language | Reading R13 keeps one language per kind: Codex matches the shipped 1–128 ASCII alphanumeric-or-dash guard, Claude keeps R7's tightened hex/64 rule. "Codex ids retain the engine's accepted language" covers non-hex and 65–80-character locators; CLI/TUI/browser participant scenarios retain the same path/hint result without a Claude drill. |
| 3 — Rejected common reference in JSON | Command C5 requires the recorded three string values, unchanged, even for every reference refusal, with `legacy: false`; null means no common reference and no valid legacy synthesis. "JSON retains every rejected common reference" pins all five refusal classes. |

The shipped adapter source and its tests settle Brokkr's compatibility
choices; they do not measure the current provider's complete id or record
vocabulary. This box still has no provider CLI or retained provider home.
S3's specific content-association and installed-help evidence and S5's bound
sizing remain obligations for council design, with controller handoff where
unmeasured. No model run is needed to settle the three rules above.

Council design must carry R12, R13 and C5 into proposed 0055 with enforcement
bindings to the shared reader and the CLI/TUI/browser regression scenarios.
It must distinguish the 128-character reader/engine guard from the unchanged
80-character built-in recording clamp, state that payload ids neither prove
nor veto filename identity, and keep rejected-reference JSON separate from
lookup permission. This is required content for that phase's decision draft,
not acceptance of 0055 or permission to alter #226's adapters. No design or
tasks artifact exists on this visit to amend or claim complete.

### S9 — Return from design: U1 and U2 answered in the owning requirements

Adopted `6ece1ea` and validated the existing change before amending it.
`returned_from` names the design's **upstream** result: U1 and U2 are faults
in this specification, not in triage, decision 0032 or #226. The proposal
is amended first, followed by reading, command and TUI deltas. The fourteen
clarification answers remain in force except the two precise DSH policies
now refined by new format evidence: R3's universal omission rule gains the
required-event refusal, and R2's step association requires actual citations.
The original scenario names and reasons remain traceable.

Both council positions were read in full. Adopt robustness's required-event
refusal and citation-specific suppression, combined with the chief's
persisted-row evidence. Reject simplicity's canonical-only reading and
removal of diagnostics: those cuts would discard the settled interrupted
content and format-drift answers. The chief's D1 reconciliation of the other
claims remains applicable, including D9's Codex completed-item support under
the existing response-item preference; neither finding reopens that policy.

| Finding | Answer and owning scenarios |
|---|---|
| U1 — Unknown required DSH events | Reading R14 distinguishes a safe counted omission from `unsupported-format`, pins whole-prefix counts and source-only truncation on refusal, and keeps I/O/UTF-8 precedence. “A required unknown DSH event refuses all prose” and the marker/collision scenarios bind it. Command C6 preserves the selected reference/path/hint and refuses every turn request; TUI T5 clears both doors on the same transition. |
| U2 — Physical rows and assembly citations | Reading R15 distinguishes physical rows from logical events, decodes text/reasoning/tool-argument packing without concatenating fragments, and defines ranged/empty/absent/partial citations. The packed/unpacked equivalence, malformed-row, range, scope and cap scenarios bind it. Command C7 and TUI T6 fix numbering, notices and selection invalidation after projection. |

The tagged DSH `dsh-v0.1.2-rc.1` [session types](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/core/session/src/types.ts),
[JSONL writer](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/session/session-persistence-jsonl/src/format.ts)
and [chunk codec](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.1.2-rc.1/packages/core/session/src/chunk-rows.ts)
were read as public primary source on 2026-09-09. They support these storage
and citation facts, not a live provider measurement. The direct range-codec
fetch was unavailable; the citation admission/set rules in R15 are explicit
Brokkr policy based on the writer's documented encoding and earlier-source
contract, not a claim to duplicate every provider replay validator. This box
still has no installed provider CLI or Cargo. S3's remaining Codex producer
association evidence and S5's proposed bounds are preserved, not fabricated.

The existing `design.md` is the historical council return at `6ece1ea`.
Its U1/U2 remedies are now answered here and in the deltas; its D2–D9
architecture remains applicable subject to these answers. Under S1 and the
dialect's declared artifact scope, this specify visit commits proposal/specs
only. The returned council must reconcile its admission/validation record
with R14/R15, finish its remaining provider evidence, and author/register
`docs/decisions/0055-read-every-transcript-kind.md` with `Status: proposed`
before implementation. That decision must carry these refusal-state,
physical-row/count/turn, timestamp, citation and audit-order rules with
bindings to projector, CLI/TUI and privacy tests. It must preserve S2/S8 and
all prior bindings. No proposed decision is accepted here, no task breakdown
exists to amend, and drafting does not claim design admission or delivery.

### S10 — Successor adoption preserves the repaired specification

Run `close-issue-222-a-transcript-rea-df38565b` adopts the committed change at
`3013388`, including the third-pass repair, fourth-pass clear record and
U1/U2 answers. Strict OpenSpec validation passed before amendment; the tree
was clean and descends from the commissioned base. This run's supplied
context contains no `returned_from` finding. Earlier quota failures are
provider failures, not clarification verdicts; neither they nor this
structural validation substitute for this run's independent judgments.

All prior answers remain in force. The existing council record and both
positions were read in full; S9's claim-by-claim reconciliation stands.
Design remains a historical upstream return awaiting reconciliation with
R14/R15, its remaining provider evidence and proposed 0055. Artifact presence
is not design admission. The registry still ends at 0053; 0054 and 0056
remain the controller's sibling reservations. This specify visit amends
only proposal and requirements; it does not author later phases' artifacts.

The controller's `.forge/controller-dsh-transcript-storage-interface.json`
capture, measured on 2026-09-09 at 08:10:13 UTC, is now available through the
workspace. Every captured source text matches its recorded SHA-256. Its
`@deepseek-ai/dsh-session` package metadata identifies version 0.1.2-rc.1.
The packed-row decoder confirms R15's stored shape, member boundaries and
time reconstruction. The captured `seq-ranges.js` closes S9's missing
range-codec source access: it confirms inclusive pairs but enforces strict
increasing order when a range is present. R15 deliberately uses set
membership instead and already disclaims execution replay. Preserve that
settled policy, document its reason against this source, and add the concrete
scenario “Citation entry order does not become transcript order”. Command
and TUI retain their existing shared-result, numbering and refresh rules.

This is inspection of controller-captured installed source, not a fresh
installed-provider probe or live transcript measurement. Codex association
proof and the remaining design mappings in S3/D9, S5's proposed bounds,
proposed decision 0055 and all delivery gates remain due. No sibling tree,
provider home, global setting, frozen file or accepted decision is changed.

Validation for this adoption: strict OpenSpec validation and delta parsing
pass with 20 requirements and 153 scenarios; all 152 earlier scenario names
and all three capability boundaries are preserved. The command/TUI deltas
and historical design remain byte-identical. Status reports artifacts by
presence with tasks ready and planning/apply completion false. Whitespace,
frozen-byte, accepted-decision and unchanged-coverage-script checks pass.
All five commissioned Cargo commands were attempted with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2` and exited 127 because Cargo
is absent. Exact coverage was attempted unchanged with those limits,
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at
`mktemp` because `/var/tmp` is absent. No Rust or host-boundary proof ran.
This visit's evidence is
`.forge/specify/read-every-transcript-kind-df38565b-validation.json` and
`.forge/specify/read-every-transcript-kind-df38565b-scenario-audit.json`;
prior evidence is preserved. Those delivery checks remain pending, not
passing, and this adoption claims no independent clarification verdict.

### S11 — Return from clarify: DSH header version is an admission rule

Adopted `read-every-transcript-kind` at `9170730` in the same commissioned
run. The one finding in `returned_from` is this specification's missing
DSH header-version policy. Triage, accepted decisions and the fourteen
prior clarification answers are not at fault. R14/R15 and S10's citation-set
policy remain in force; this amendment pins the format prerequisite they
had left implicit. Proposal precedes reading, command and TUI amendments.

The controller capture identified in S10 was rechecked: all 42 captured
texts match their recorded SHA-256. Both the session and persistence-JSONL
packages identify version 0.1.2-rc.1. In the captured session `lib/index.js`,
line 57 declares `SESSION_FORMAT_VERSION = 0`. In persistence-JSONL
`lib/index.js` (SHA-256
`dfd6cde28928996f2f44220d013359563c8d9bf6c9c904972e77256767eec385`),
lines 214–236 reject a foreign numeric version before the current header
shape check or event decoding; lines 38–79 show the physical header and its
replay validator. These are controller-captured installed-source facts,
not a fresh provider invocation, a live transcript measurement or evidence
that a future version has the same event meaning.

Adopt the version-zero decoding boundary. Reject version-agnostic decoding:
R14's refusal already recognizes that unknown semantics can invalidate
otherwise plausible content and associations. Do not convert an owned but
unsupported file to `not-found`, infer a missing version as zero, or choose
a current-version root over another owned root. Conversely, do not import
the provider's entire replay validator: the recorded root and the settled
first-record/depth predicate establish ownership, and this reader neither
replays a session nor consumes its execution metadata. R16 documents these
intentional distinctions, including the preserved omitted-depth allowance.

| Finding facet | Owning answer and scenario |
|---|---|
| Supported versions and header shapes | Reading R16 requires a first object of type `session`, settled root depth and numeric version zero for content admission; other header metadata does not affect an audit read. “DSH admits only the declared version-zero header shapes” and “Missing and mistyped DSH versions are not legacy zero” pin the boundary. |
| Absent or malformed header versus rejected version | Ownership discovery precedes version admission and never skips the first row. “An absent or malformed opening DSH header cannot borrow a later one” keeps `not-found` and null path/hint; a unique owned header with a rejected version instead keeps its path under `unsupported-format`. |
| Foreign version alone or beside current | “A foreign-version DSH root alone keeps its confirmed location” refuses content; “Current and foreign DSH roots are still ambiguous” forbids format-based ranking. Invalid/delegated candidates remain outside the root count under R8. |
| Precedence, counts, cap and hints | The usable bounded byte snapshot precedes header admission; rejected versions cause no event classification, zero counts and only observed source-cap truncation. Reading's collision scenarios, command C8 and TUI T7 make the whole/selected/error/refresh states identical. |

Required content for **proposed 0055**, to be authored and registered by the
returned council before implementation:

> DSH local content decoding admits numeric on-disk version zero only. Safe
> unique root ownership is established before format admission, independently
> of version; invalid/delegated roots cannot substitute and unsupported roots
> still participate in ambiguity. An absent, mistyped or foreign version on
> that unique root refuses as `unsupported-format` after bounded source
> I/O/UTF-8 validation, with the unchanged reference, confirmed path/hint,
> no turns, zero row counts and only observed source-cap truncation. No event
> decoder, association or display-cap decision runs under a rejected header.
> Other header metadata is outside this audit reader's admission policy;
> legacy omitted depth remains root depth, but omitted version is not zero.
> Current-version event/storage refusals retain R14/R15's whole-prefix counts.

Its enforcement bindings must name shared header-admission and ownership
matrix tests (including current/foreign roots in both enumeration orders),
source-failure/cap/count tests, CLI whole/selected text/JSON conformance and
TUI refusal/recovery tests, all using synthetic test-owned files. Preserve
S2/S8/S9/S10 and their existing bindings. No accepted decision, adapter or
frozen byte changes to implement this proposal's policy.

The existing `design.md` remains the historical upstream council record.
Its D3 ownership/safe-source architecture still applies; D4's unconditional
row classification now has R16's version-admission prerequisite. Its old
U1/U2 and admission record must be reconciled with S9–S11 on the returned
council visit, including the required proposed 0055. Under the dialect's
specified artifact scope, this seat commits only proposal/specs, not a
replacement council verdict or tasks. All later commissioned phases and
S3's remaining measured-association evidence remain due; this repair does
not claim independent clarification, design admission or delivery.

Validation of this header-version return: strict OpenSpec validation and
delta parsing pass with 20 requirements and 168 scenarios. All 153 prior
scenario names and bodies are unchanged; the 15 additions cover the returned
finding and its command/TUI consequences. The source-hash and scenario audits
are in `.forge/specify/read-every-transcript-kind-header-version-scenario-audit.json`.
Status reports proposal/specs/design by presence and tasks ready, with both
planning and apply completion false; the historical design is not thereby
admitted. Whitespace, frozen/accepted-decision/production bytes, historical
design and unchanged coverage gate checks pass.

All five commissioned Cargo commands were attempted with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`; each exited 127 because Cargo
is absent. Unchanged exact coverage was attempted with those limits,
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at
`mktemp` because `/var/tmp` is absent. No Rust or boundary proof ran.
Command evidence is in
`.forge/specify/read-every-transcript-kind-header-version-validation.json`.
Host proof and delivery checks remain pending with the controller; this
specification validation is not implementation evidence or a clarification
verdict. Earlier evidence files are preserved.

### S12 — Successor adoption grounds the browser's stale-content rule

Run `close-issue-222-a-transcript-rea-04d01419` adopts the committed change
at `8336e11`, including every earlier repair through the task breakdown.
Strict OpenSpec validation passed before amendment; the tree was clean and
descends from the commissioned base `5bc8cf3`. This run's context carries no
`returned_from` finding: triage ruled `design` and specify is its first seat.
Neither the earlier clarification-budget exhaustion nor the provider quota
failure is a verdict on this specification, and this structural validation
replaces no later phase's independent judgment.

All prior answers stay in force: S1–S11, R1–R16, C1–C8 and T1–T7, the closed
Claude omission list, Codex filename identity and id language, the rejected
reference echo, the DSH ownership/version/packed/citation rules, the proposed
bounds and the explicit limits of provider evidence in S3 and S5. The change
identifier remains `read-every-transcript-kind`. `docs/decisions/` still ends
at 0053 on this base, so reserved 0055 is free and the controller's 0054 and
0056 reservations are untouched.

The commission carries three open analysis findings from run
`close-issue-222-a-transcript-rea-df38565b`. A6 (task 6.7's absent-partner
wording) and A7's missing browser truncation task are work for the tasks
office; reading already retires that browser sentence, so A7 needs no new
rule. A5 is different in kind: the behavior it requires existed only in
council design D9, never in a requirement this office owns, which is why the
breakdown had nothing to derive it from. That omission is this
specification's fault, so the owning requirement now states the rule and one
scenario binds it.

| Amendment | Reason and owning place |
|---|---|
| Browser admission loss clears cached prose | Reading's browser requirement and R17 make the page close the watch without reconnecting, discard its cached and displayed turns, re-request the shared presentation and render the current refusal; a superseded response restores neither. "A closed growth stream cannot leave stale browser prose" pins the outcome and its recovery. |
| The retired browser suffix is named | The cap requirement now names the shipped `transcript truncated (size cap) — resume the session for the rest` literal beside the TUI suffix, so R6's settled answer has an exact target instead of an unnamed existing sentence. |

Evidence at base `5bc8cf3`: `crates/brokkr-cli/src/ui.html:757-782` keys
`transcriptCache` by session id, invalidates it only in the session stream's
`onmessage`, and installs no `onerror`, so a closed stream reconnects by
default and the previous body stays displayed; `ui.html:1118` closes the
session watch from the run stream without discarding that cache; and
`ui.html:814` holds the browser truncation literal named above. This is
shipped-source inspection, not a live browser, provider or transcript
measurement.

Neither amendment changes an accepted decision, a frozen byte, the command
or TUI delta, or the design and tasks artifacts this office does not own.
The returned council must reconcile its existing D9 clause with R17 and carry
the rule into proposed 0055; the tasks office still owes A5's client task and
proof, A6's split expected outcomes and A7's renderer task and assertion.
Under S1 and the rendered dialect, this visit commits proposal and specs only
and claims no clarification, design admission or delivery.

Validation of this adoption: strict validation and delta parsing pass with
20 requirements and 169 scenarios, each carrying scenarios. All 168 earlier
scenario names survive, and the command and TUI deltas are byte-identical.
`git diff --check` passes. `contracts/`, `policy/`, `reference/`, `fixtures/`,
`docs/decisions/`, `crates/`, `scripts/`, `bundles/` and `openspec/specs/`
are byte-identical to the commissioned base. All five commissioned Cargo
commands were attempted with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`
and exited 127 because Cargo is absent from this box, whose PATH holds only
`openspec`, `node`, `python3` and `git` and whose `HOME` holds no provider
home. Unchanged exact coverage was attempted with those limits,
`TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at
`mktemp` because `/var/tmp` does not exist here. No Rust, coverage or
boundary proof ran, and no unavailable check is counted as passing evidence.
This visit's record is
`.forge/specify/read-every-transcript-kind-04d01419-validation.json`;
earlier evidence files are preserved. Host proof, remote CI and delivery
remain the controller's pending work.

### S13 — Return from clarify: the browser's recovery branch and occasion

Adopted the committed change at `649a9df` and validated it before amending
it. `returned_from` names two open questions inside specify's own S12
amendment, both in the owning capability `transcript-reading`: triage, the
accepted decisions, the sibling issues and every prior answer are not at
fault. Both are settleable from shipped source in this repository, so
neither waits on S3's missing provider evidence. The proposal is amended
first, then the reading delta; the command and TUI deltas need no change
because these are browser-client rules this capability already owns.

| Finding | Resolution and owning scenario |
|---|---|
| 1 — The still-admitted branch of a closed watch | Reading R18 makes the page display the fresh result's turns and open one new growth watch while the participant is working, keeps the ban on reconnecting the closed stream, and bounds automatic recovery to one watch opening per participant between recurring re-checks, leaving the clearing rule's re-request uncapped because it carries no body and cannot outpace those openings. "A dropped stream on a still-admitted source resumes growth" pins the recovery and the bound. |
| 2 — The occasion for the next look | Reading R19 requires a recurring presentation re-check that needs no journal-head change, re-selection or other operator action and recurs at least as often as the page's existing runs poll, with an equivalence rule that makes an unchanged re-check invisible. "A persisting browser refusal is re-checked without the operator" pins the concluded-run case, and the closed-stream scenario's recovery clause now names that occasion instead of permitting a reopen. |

No normative sentence in the three deltas now leaves an outcome permissive;
the remaining `may` and `could` occurrences are descriptive prose in the
decision records and one hypothetical clause about an unrelated id-only
lookup. Browser growth recovery is preserved rather than lost, so the
declared compatibility note is a changed mechanism and cadence, not a
removed capability; the What Changes browser bullet records it beside
R11's costs. The evidence is shipped source at base
`5bc8cf3`, read in this worktree: `crates/brokkr-cli/src/ui.html:770-782`
(the session watch, with no `onerror` on `sessionSource`),
`ui.html:1117-1118` (the run stream's repaint and its watch close),
`ui.html:1127` with `ui.html:345-357` (the five-second interval repaints
only the runs list), `ui.html:818-821` (`loadDetail`'s two occasions),
`crates/brokkr-cli/src/ui.rs:429-461` and `ui.rs:464-490` (the transcript
and head poll loops) and `ui.rs:393` (the one-second `SSE_POLL`). This is
source inspection, not a live browser, provider or transcript measurement,
and it changes no accepted decision, frozen byte or production file.

The returned council must reconcile D9's single closure clause with R17,
R18 and R19 and carry all three into proposed 0055; the tasks office owes a
client task and proof for the reopen, its bound and the recurring re-check
beside A5's clearing work, with its A6 and A7 repairs unchanged. Under S1
and the rendered dialect this visit commits proposal and specs only and
claims no clarification verdict, design admission or delivery.

Validation of this return: strict OpenSpec validation passes
(`Change 'read-every-transcript-kind' is valid`, exit 0) with 20 ADDED
requirements and 171 scenarios, none without scenarios. All 169 earlier
scenario names survive and the two additions are the ones named above; the
command and TUI deltas are byte-identical to `649a9df`. `git diff --check`
passes, and `contracts/`, `policy/`, `reference/`, `fixtures/`,
`docs/decisions/`, `crates/`, `scripts/`, `bundles/` and `openspec/specs/`
are byte-identical to the commissioned base `5bc8cf3`. All five
commissioned Cargo commands were attempted with `CARGO_BUILD_JOBS=2` and
`RUST_TEST_THREADS=2` and exited 127 because Cargo is absent from this box,
whose PATH holds only `openspec`, `node`, `python3` and `git` and whose
`HOME` holds no provider home. The unchanged exact-coverage gate was
attempted with those limits, `TMPDIR=/var/tmp` and
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at `mktemp` because
`/var/tmp` does not exist here. No Rust, coverage or boundary proof ran and
no unavailable check is counted as passing evidence. This visit's record is
`.forge/specify/read-every-transcript-kind-recovery-return-validation.json`;
earlier evidence files are preserved. Host proof, remote CI and delivery
remain the controller's pending work.

### S14 — Return from clarify: admission, refusal and the one-opening budget

Adopted the committed change at `1e770f2` and validated it before amending
it. `returned_from` names two open questions, both inside specify's own S13
amendment and both in the owning capability `transcript-reading`: triage, the
accepted decisions, the sibling issues and every earlier answer stand. Both
are settleable from text already in this change and from shipped source in
this repository, so neither waits on S3's missing provider evidence. The
proposal is amended first, then the reading delta; the command and TUI deltas
need no change because these are browser-client rules this capability owns.

| Finding | Resolution and owning scenario |
|---|---|
| 1 — Is an unreadable source still "admitted", and what stops the body-refusal cycle | Reading R20 makes the branch predicate the presentation's own admission state, established by eligibility, identifier/home validation and safe unique discovery, and keeps post-admission `unreadable`, `unsupported-format` and a readable zero-turn source as body outcomes that never move admission or the equivalence tuple's reason. It defines a refused body request, forbids naming a reason the two identical 404 envelopes cannot distinguish, and gives the cycle its floor: a refusal silences that source's body and watch until the next re-check. "An unreadable admitted source is asked once per re-check" pins one refused request per interval. S15/R24 later distinguish discovery-stage `unreadable`, which prevents admission in the first place. |
| 2 — Does a re-check's own reopening spend the one-watch budget | Reading R21 deletes "Apart from a recurring re-check itself", so every automatic opening spends the single per-interval budget and each re-check restores it; "within the bound above" and the dropped-stream gloss now agree, and the tasks office's expected count is one. "A re-check's own reopening spends that interval's watch budget" pins the re-check-first trace beside the closure-first one. |

The bound paragraph's false premise is repaired with it: presentation
requests still need no separate cap, but the stated reason is now that each
follows a closure or a refusal, that at most one watch opens per interval and
so at most two can close in it, and that the first refusal silences further
body requests until the next re-check. Growth repaints on an open admitted
watch are excluded from the bound as the shipped growth mechanism, paced by
that stream's own poll. The equivalence rule keeps an unchanged re-check
invisible except for its two deferred repairs, a missing body and a missing
watch, and orders them so a watch is opened only after a successful body.

The evidence is text already in this change (the id-only route's shared 404
envelope for lookup and read failures, the `unreadable` and
`unsupported-format` states, R19's discovery-shaped costing) and shipped
source at base `5bc8cf3`, read in this worktree:
`crates/brokkr-cli/src/ui.html:783-796` (the body fetch and the existing
`no local transcript for this session` fallback with its checkpoint table),
`ui.html:757` and `ui.html:775-782` (the transcript cache dropped on each
growth event, which is why a growth repaint is a body request),
`crates/brokkr-cli/src/ui.rs:429-461` and `ui.rs:393` (the transcript stream
and its one-second `SSE_POLL`, which pace those repaints). This is source
inspection, not a live browser, provider or transcript measurement, and it
changes no accepted decision, frozen byte or production file.

The returned council must reconcile D9 with R17–R21 and carry all five into
proposed 0055; the tasks office owes client tasks and proofs for the refusal
floor, the single opening budget and the two deferred repairs, beside A5's
clearing work, with its A6 and A7 repairs unchanged. Under S1 and the
rendered dialect this visit commits proposal and specs only and claims no
clarification verdict, design admission or delivery.

Validation of this return: strict OpenSpec validation passes
(`Change 'read-every-transcript-kind' is valid`, exit 0) with 20 ADDED
requirements and 173 scenarios, none without scenarios. All 171 earlier
scenario names survive and the two additions are the ones named above; the
command and TUI deltas are byte-identical to `1e770f2`. `git diff --check`
passes, and `contracts/`, `policy/`, `reference/`, `fixtures/`,
`docs/decisions/`, `crates/`, `scripts/`, `bundles/` and `openspec/specs/`
are byte-identical to the commissioned base `5bc8cf3`. All five commissioned
Cargo commands were attempted with `CARGO_BUILD_JOBS=2` and
`RUST_TEST_THREADS=2` and exited 127 because Cargo is absent from this box,
whose PATH holds only `openspec`, `node`, `python3` and `git` and whose
`HOME` holds no provider home. The unchanged exact-coverage gate was
attempted with those limits, `TMPDIR=/var/tmp` and
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; it exited 1 at `mktemp` because
`/var/tmp` does not exist here. No Rust, coverage or boundary proof ran and
no unavailable check is counted as passing evidence. This visit's record is
`.forge/specify/read-every-transcript-kind-admission-return-validation.json`;
earlier evidence files are preserved. Host proof, remote CI and delivery
remain the controller's pending work.

### S15 — Successor adoption separates discovery and body unreadability

Adopted all committed work at `9298e8d`, including the checkpoint `73797a6`
repairs for drill eligibility and successful zero-turn bodies. Those repairs
remain correct: an admitted Codex/DSH or foreign-home source never drives a
Claude route, and a successful empty Claude body is not repeatedly fetched.
The latest F1 finding identifies an earlier fault in R20's broader statement,
not in either checkpoint repair.

R20 treated every `unreadable` result as a post-admission body outcome, but
the same capability already requires `unreadable` during shared discovery
when directory I/O or a bounded DSH opening-header I/O/UTF-8 failure prevents
the reader from establishing a unique owned source. Browser participant
presentation performs that bounded discovery for all kinds, including the
DSH header work needed to determine ownership, while this change deliberately
adds no Codex or DSH browser body route. Reading R24 therefore distinguishes
the stages: discovery-stage `unreadable` is a presentation unavailability
reason that closes admission; an I/O/UTF-8 failure after safe unique discovery
has admitted a source remains the body-level `unreadable` governed by R20's
refusal floor. The scenario "DSH discovery unreadability is a browser
presentation refusal" binds the otherwise missing DSH outcome and also pins
the equivalent directory-discovery case. This does not turn presentation into
a transcript-body projection or change its bounded recurring cost.

F2 and F3 are confirmed defects in the already-authored downstream
`tasks.md`, not faults in the proposal or capability deltas. The tasks office
must scope its no-mutation convention to the reader/system under test and
real operator evidence while retaining synthetic test-owned homes/files and
their append, shrink, disappearance and same-length rewrite fixtures. It must
also make the Rust 1.88 gate compile the new dev-only Boa harness and
controller tests with an all-targets or targeted test-compilation command;
the production-only `cargo check --workspace --locked` is insufficient.
Neither repair changes a product requirement or council choice. Under S1 and
the rendered dialect, this office records those downstream obligations but
does not edit or commit `tasks.md` during specification.
