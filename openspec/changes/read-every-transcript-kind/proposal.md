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
  and tool outputs without duplicating streaming echoes.
- Resolve the selected participant's recorded kind, locator and home to
  its own retained file. Distinguish missing, invalid, ambiguous and
  unreadable references; never substitute another seat, a delegated DSH
  session or the newest file in a provider home.
- Add `brokkr transcript --run <selector> --seat <label-or-key>` with
  optional one-based `--turn <n>` and `--json`, using the read surfaces'
  existing journal and realm selection and the TUI's same derivation.
- Make the TUI transcript pane, open-turn door and open-whole door work
  for all three readable kinds, including live file appearance/growth.
  Show the correct full-session location and measured command spelling:
  Claude's `claude --resume <id>`, Codex's rollout path and
  `codex exec resume <thread>`, and DSH's session file without inventing a
  resume command. These are display hints and execute nothing.
- Apply the existing 4,000,000-byte displayed-block budget to every kind,
  with bounded source reads and discovery and an explicit truncation
  notice shared by text, JSON and both TUI doors. Preserve journal privacy,
  transcript retention, terminal sanitization and accepted contracts.
- Require reserved decision **0055**, with status **proposed**, for the
  new local reading, selection, output and limit semantics. The council
  design must author it and register it before implementation; only the
  operator can accept it. No semantic production change is made in this
  specification phase.

## Capabilities

### New Capabilities

- `transcript-reading`: local identity and ownership checks, per-kind
  content projection into ordered turns, bounded reads and privacy.
- `transcript-command`: run/seat/turn selection, text and versioned JSON
  output, and explicit command failures for unavailable transcripts.
- `transcript-tui`: all-kind transcript navigation, live refresh,
  truthful full-session hints and consistent truncation notices.

### Modified Capabilities

None. The current standing capabilities concern named execution boundaries;
this change preserves their requirements and introduces transcript reading
as separate capabilities.

## Impact

Production work belongs in `crates/brokkr-view` and the CLI's local reader,
argument handling, rendering and TUI shell. Existing Claude browser drills
must consume the shared Claude derivation and remain compatible; adding
Codex/DSH browser routes is outside this commission. Tests belong in the
existing view, CLI, TUI and reader suites, with synthetic files in test-owned
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
retention and paths-only journal law; 0055 supplements it for local reading.
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
or identify the precise missing proof for controller handoff. This does not
justify omitting Codex/DSH reading, weakening acceptance, inventing formats,
changing global settings, or starting paid model experiments.

### S4 — Validation evidence is scoped to what ran

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

### S5 — Extra bounds are proposed policy, not provider measurements

The existing 4,000,000-byte block budget is preserved. The new 32 MiB source
budget bounds memory and input work even for ignored records; it leaves room
for JSON framing and metadata beyond the displayed-text budget. Six Codex
directory levels follow the existing adapter search bound. The 10,000-entry
discovery and 65,536-byte header bounds make a lookup finite; they are
proposed limits, not measured maxima of provider homes or headers. A refusal
at a limit stays explicit, and the retained original remains untouched.
Council design must test these choices against installed format evidence and
return to this specification if a normal supported record cannot fit.

Selection, malformed-record handling and omission choices are answered in
the owning capability scenarios. Synthetic parser and filesystem cases must
be added under the existing crate test suites; the frozen evaluator fixtures
are never regenerated or repurposed for transcript examples.
