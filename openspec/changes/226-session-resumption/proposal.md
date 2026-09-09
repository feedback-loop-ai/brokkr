# Change: Same-instance session resumption and durable progress (#226)

## Why

Eligible Claude and DSH retries lose their session's reasoning while retaining
partial edits, making the next smith reconstruct work it can misinterpret.
Issue #226 also exposes a recovery gap when safe resume is unavailable:
completed work needs a truthful task marker before the phase's final commit.

## What Changes

- Generalise decision 0030's offer to every model invocation site: single seats,
  panel members, sequence model steps and members inside sequence panels.
  Retries, phase re-entry and operator retry after a park reuse only the latest
  eligible session of the same run, site and adapter instance.
- Keep admission in the engine and provider-specific acceptance in the adapter,
  using negotiated resume messages. Prevent cross-site, cross-candidate and
  cross-instance reuse; deterministic exec steps hold no provider session.
- Assess Codex, Claude, DSH and LaneTally separately. Implement measured safe
  paths and declare unsupported or unmeasured shapes with their evidence.
  Re-impose current restrictions, hands and model/effort settings on every
  rejoin; inherited permissions and argument parsing do not prove safety.
- Report confirmed cold/resumed outcomes and bounded reasons for declined
  offers, preserving first-work acceptance and pre-session refusal semantics.
  Permit at most one proven pre-work cold replacement within the invocation's
  existing deadline, cancellation, attempt and chain bounds.
- Persist completed, locally verified task progress before the next group,
  independently of commits. Reconcile markers with surviving edits on recovery;
  pending work and verification remain visibly pending.
- Require proposed decision **0056**, with alternatives, numbered rulings and
  enforcement bindings, including progress timing. Its document and registry
  entry must precede production semantic changes; only the operator accepts it.
  The council must settle its design and the smith must include its enactment
  in the requirement-linked breakdown.

## Capabilities

### New Capabilities

- `site-session-resumption`: durable site-specific offer eligibility and
  one-use protocol delivery without transferring session ownership.
- `adapter-resume-safety`: measured support, restriction re-imposition,
  provider handle validation and bounded refusal fallback.
- `adapter-launch-evidence`: confirmed launch facts, refusal timing, privacy,
  compatibility and usage accounting.
- `sdd-progress-markers`: durable mid-phase progress and honest recovery
  without permitting judges to edit artifacts.

### Modified Capabilities

None. Existing realm boundary, manifest, gate and boundary-record requirements
remain authoritative; these deltas add resume and recovery behavior.

## Evidence and scope

Source: [issue #226](https://github.com/feedback-loop-ai/brokkr/issues/226),
the operator's 2026-09-09 commission and triage's
`.forge/tasks/226-session-resumption.md`. This change adopts that framing's
identifier and starts from shipped main
`5bc8cf305aaef9af269866cbf83f094939691399`.

The issue's three-`None` description predates this base. Inspection of
`engine.rs` and `engine/resume_tests.rs` confirms that single seats already
receive eligible offers; sequence model steps and panel members do not.
Fixing only the adapter switch would leave those sites cold.

Only Codex advertises resume and emits `launch` in `adapters.rs`. Claude and
LaneTally invoke their shared stream cold, and DSH creates a retained transcript
directory not established as its CLI's resume identifier. `process.rs` sends
offers only after negotiation. The deltas therefore separate offer receipt,
safe acceptance and confirmation, and distinguish provider handles from
transcript locators.

Decision 0030 supplies historical Codex measurements. Decision 0053 remains
proposed, but its refusal behavior shipped in `4624393` and must be preserved.
The issue's 47 cold/19 resumed Codex launches and interrupted 340-turn Claude
session are supplied historical evidence, not repeated measurements or targets.

This box resolves OpenSpec 1.12.0 but none of `codex`, `claude`, `dsh`,
`claude-lanetally`, `cargo` or `rustc` on PATH. Current provider interface
and enforcement facts remain unmeasured. Later evidence must distinguish
installed help/source, deterministic shims and necessary bounded enforcement
probes. Missing evidence proves neither safe support nor a CLI defect.

## Impact

Production changes belong in Rust under `crates/`: runtime dispatch,
protocol invocation/negotiation and their existing test/conformance suites.
Adapter declarations, packaged equivalents, driver/provider guides and smith
instructions must agree. Decision 0056 uses the established registry mechanism;
0054 and 0055 remain reserved for other fires.

Seat-record v4 admits two launch values, five refusal tokens and three Codex
sandbox classes, not arbitrary provider permission names or explanations.
Design must use these fields truthfully or specify an additive contract version,
embedding, version selection and compatibility tests. Existing contracts,
production policy, schemas, reference and fixtures stay frozen.

Issue #222 owns transcript reading and CLI/TUI derivation. This issue owns
session plumbing, launch evidence and progress markers; transcript caps and
privacy boundaries remain intact. Shared-file overlap goes to the controller,
without accessing its sibling worktree. Quota repair, DSH Git metadata repair,
trust promotion, new boundaries, global provider settings and release work
are outside this change.

## Delivery obligations

This phase authors proposal and deltas in dialect order. Design, tasks,
clarification, analysis, full implementation and specification review remain
mandatory. Clarification answers belong in scenarios; analysis choices belong
in design's `## Decisions`. Council positions require explicit reconciliation,
and earlier artifact faults return upstream. The smith performs the normal
archive/provenance operation as the final artifact task.

PM4 specifies the required Rust checks, both bundle compilations and unchanged
exact-coverage gate, using `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.
Host proof requires `TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`;
nested-sandbox skips do not prove it. The controller owns host validation,
integration, completed-run publication, final PR/CI/merge and issue closure.
Work seats commit unsigned and never push; external results stay pending until
their evidence exists.

## Specification validation — 2026-09-09

- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  Status reports proposal/specs done, design ready and tasks awaiting design;
  planning and implementation are not complete.
- All five required Cargo commands were attempted with the commissioned job
  limits and exited 127: `cargo` is unavailable. Formatting, clippy, tests and
  both bundle compilations have no passing result from this seat.
- The unchanged exact-coverage script, with the required job limits,
  `TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited 1:
  temporary-directory creation failed because `/var/tmp` is absent in this box.
  No coverage/boundary proof was obtained; host validation remains pending.
- Frozen paths have no diff from the base. CI, release admission and coverage
  still consume `rust-nightly-version.txt`. The authored artifacts are change
  metadata, this proposal and four deltas.
