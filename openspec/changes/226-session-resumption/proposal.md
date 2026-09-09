# Change: Same-instance session resumption and durable progress (#226)

## Why

Eligible Claude and DSH retries lose their session's reasoning while retaining
partial edits, making the next smith reconstruct work it can misinterpret.
Issue #226 also exposes a recovery gap when safe resume is unavailable:
completed work needs a truthful task marker before the phase's final commit.

## What Changes

- Generalise decision 0030's offer across model work sites: single seats,
  panel members, sequence model steps and members inside sequence panels.
  Retries, phase re-entry and operator retry after a park reuse only the latest
  eligible session of the same run, site and adapter instance. Gate sites start
  fresh, including single-seat gates that the current code offers a session;
  decision 0042's fresh judging rule governs the work/gate distinction.
- Keep admission in the engine and provider-specific acceptance in the adapter,
  using negotiated resume messages. Prevent cross-site, cross-candidate and
  cross-instance reuse; deterministic exec steps hold no provider session.
- Assess Codex, Claude, DSH and LaneTally separately. Deliver working, measured
  resume for Claude's boxed workspace work shape and DSH's already-admitted
  headless work shape, preserving Codex support. Implement every measured safe
  shape; declare unsupported or unmeasured shapes honestly. Cold-only,
  declaration-gated preparation does not close #226. Re-impose current
  restrictions, hands and model/effort settings on every rejoin; inherited
  permissions and argument parsing do not prove safety.
- Report confirmed cold/resumed outcomes and bounded reasons for declined
  offers, preserving first-work acceptance and pre-session refusal semantics.
  Permit at most one proven pre-work cold replacement within the invocation's
  existing deadline, cancellation, attempt and chain bounds.
- Put dialect-independent progress timing and recovery in the shared SDD smith
  charter, applying to both OpenSpec and spec-kit. Persist completed, locally
  verified task progress before the next group, independently of commits.
  Reconcile markers with surviving edits on recovery; pending work and
  verification remain visibly pending. The non-SDD implementer is outside this
  progress rule; dialects continue to own task paths and formats.
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
receive eligible offers without checking the gate class; sequence model steps
and panel members do not. Fixing only the adapter switch would leave work sites
cold and extend a single-gate inconsistency into composite judging sites.

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
Adapter declarations, packaged equivalents, driver/provider guides and the SDD
smith charter must agree. The progress-rule source is
`agents/charters/implementer-sdd.md`; update its existing instruction tests and
measured charter identity in `crates/brokkr-runtime/tests/library_data.rs` and
relevant rendered-prompt tests. `agents/charters/implementer.md` and both dialects'
phase definitions, task instructions and packaged copies remain outside that
edit: their existing task formats already support checkboxes and progress notes.
Decision 0056 uses the established registry mechanism; the checked registry
still ends at 0053, and 0054/0055 remain reserved for other fires.

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

## Decisions

These are specification answers to clarification findings A-D, recorded on
2026-09-09. Their observable answers are scenarios in the owning deltas. They
are inputs to the council's `design.md` Decisions and proposed decision 0056,
not a claim that the operator has accepted a new decision.

- **A — Missing measurements are an incomplete delivery, not an exemption.**
  AS1 requires measured, enabled Claude boxed-workspace and DSH headless-work
  resume before #226 can be delivered. Common offer/launch plumbing and
  declaration-gated code can be prepared and committed with missing proof
  explicitly pending; they cannot be reported as full implementation or issue
  closure. This rejects the vacuous reading under which no measurements means
  no resume work. The issue requires avoiding cold respawns, while 0030 forbids
  guessing about enforcement. Unavailable probes satisfy neither obligation.
- **B — Work sites resume; judging sites start fresh.** SR1 excludes every
  gate site, including retry, re-entry and operator retry of single gates,
  gate panels and gate steps. Decision 0042 ruling 2 says the next judge is
  fresh and blind and reads prior answers from artifacts. That specific,
  later ruling resolves what 0030's work-seat example left unstated. Extending
  single-seat gate behavior to councils is rejected: current code is not
  authority to contradict fresh judging. A work-class council position or
  artifact author is still a work site; the compiled site class decides,
  never an office name or the presence of a later validator.
- **C — Progress is standing behavior; delivery commands belong to the change.**
  PM1 applies to each current change's dialect task artifact. PM4 retains
  requirement coverage, truthful verification and frozen-fixture protection.
  Run-specific commands, resource limits and controller responsibilities live
  below and must become explicit tasks, not be archived into capability truth.
  Decision 0042's one-way fold makes the old placement a specification fault.
- **D — The SDD office owns progress timing; the dialect owns its artifact.**
  PM3 places the rule once in `agents/charters/implementer-sdd.md`, with no
  framework paths in that charter, as 0042 ruling 6 requires. OpenSpec and
  spec-kit SDD smiths both inherit it. The non-SDD charter is excluded because
  it has no required dialect task artifact. Inventing an implement phase in
  either dialect or copying timing rules into their task templates is rejected:
  the existing charter already owns implement-time task completion, and those
  templates already own the required paths and formats.

## Delivery obligations

This phase authors proposal and deltas in dialect order. Design, tasks,
clarification, analysis, full implementation and specification review remain
mandatory. Clarification answers belong in scenarios; analysis choices belong
in design's `## Decisions`. Council positions require explicit reconciliation,
and earlier artifact faults return upstream. The smith performs the normal
archive/provenance operation as the final artifact task.

The smith's breakdown must carry these commission-specific obligations, with
separate unchecked tasks for unavailable measurements and validation. Extend
the existing Rust protocol, adapter, runtime, conformance and instruction suites
as applicable. Run the following with the commissioned limits:

```sh
export CARGO_BUILD_JOBS=2
export RUST_TEST_THREADS=2
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh
```

Exact coverage uses the unchanged gate and shared pinned compiler. A missing
tool, failed check or skipped nested boundary test is not passing proof. The
controller owns host validation, integration, completed-run publication, final
PR/CI/merge and issue closure. Work seats commit unsigned and never push; only
actual external results complete pending handoff checks. Do not start other
runs or alter global provider settings.

If the implementation seat also lacks installed provider binaries/source, it
must recheck and record its own availability. It must still prepare the Rust
engine/protocol offer and launch plumbing and deterministic tests, update
`adapters/{codex,claude,dsh,lanetally}.json` and any packaged representations
with explicit evidence status, and record the missing proof in the provider
guides and proposed `docs/decisions/0056-*.md`. Design must define a support
declaration that prevents unmeasured shapes from executing resume; adding that
declaration does not claim support. Provider-specific launch construction may
be prepared only from identified interface evidence, never invented from a
shim. With neither installed help/source nor a supplied dated measurement,
no speculative Claude/DSH resume argv is implemented; common plumbing still is.
Known historical Codex paths retain their measured scope.

The AS1 measurement/enablement tasks stay unchecked until dated controller host
evidence supplies CLI identity, exact invocation, same-root confirmation,
current restriction enforcement and current-only accounting for each required
shape. The controller receives the missing shapes and observations required;
no seat reaches outside the box for a provider home. Prepared but gated code is
partial work, not a lawful close for #226. A measured impossibility is reported
as a failed specification requirement for the normal return path, with its
reason; it cannot silently reduce the minimum or relax 0030's safety rule.

## Specification validation — 2026-09-09

The return pass re-ran the initial draft's checks and records their limits:

- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  Capability/requirement checks cover four deltas, 19 requirements and 96
  scenarios. Findings A-D are answered by this author in the owning scenarios;
  no later clarification or analysis verdict is claimed.
- Status reports proposal/specs done, design ready and tasks awaiting design;
  planning and implementation are not complete. Decision 0056 remains reserved
  for the proposed record preceding production semantic changes.
- All five required Cargo commands were attempted with `CARGO_BUILD_JOBS=2`
  and `RUST_TEST_THREADS=2` and exited 127 because `cargo` is unavailable.
  Formatting, clippy, workspace tests and both bundle compilations have no
  passing result from this seat.
- The unchanged exact-coverage script, with those limits, `TMPDIR=/var/tmp`
  and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited 1: temporary-directory
  creation failed because `/var/tmp` is absent in this box. No coverage or
  boundary proof was obtained; controller host validation remains pending.
- The availability check again resolves OpenSpec 1.12.0 but none of `codex`,
  `claude`, `dsh`, `claude-lanetally`, `cargo` or `rustc`. This return supplies
  no new installed-provider interface or enforcement evidence.
- `git diff --check` is clean. Frozen paths have no diff from the commissioned
  base; CI, release admission and coverage still consume
  `rust-nightly-version.txt`. Only this proposal and its four deltas change
  on this return; existing change metadata is retained.
