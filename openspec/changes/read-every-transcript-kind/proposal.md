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
  unreadable references. Refuse DSH projection as `unsupported-format` for
  unknown required events or invalid packed-row/citation encodings, with no
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
  new record/block kinds remain visible as counted omissions. Retire Claude's
  truncation suffix; every kind uses exactly `transcript truncated (size cap)`.
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
  Codex/DSH browser transcript bodies remain outside scope.
- Require reserved decision **0055**, with status **proposed**, for the
  new local reading, selection, output and limit semantics. The council
  design must author it and register it before implementation, explicitly
  proposing the replacement of decision 0032 ruling 4's command-construction
  binding stated in S2, the successor's identity/JSON resolutions in S8, and
  the returned DSH format and refusal resolutions in S9;
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
guard, lookup refusals and truncation text change as declared above. Browser
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
