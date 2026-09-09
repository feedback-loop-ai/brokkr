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
  headless work shape, preserving Codex work-site coverage after installed-version
  remeasurement. DSH headless session-selection investigation and integration
  through supported settings/extension interfaces are in scope; the deferred
  hands/tools plugin remains separate. Implement every measured safe shape;
  declare unsupported or unmeasured shapes honestly. Cold-only,
  declaration-gated preparation does not close #226. Re-impose current
  restrictions, hands and model/effort settings on every rejoin; inherited
  permissions and argument parsing do not prove safety.
- Report confirmed cold/resumed outcomes and bounded reasons for declined
  offers, preserving first-work acceptance and pre-session refusal semantics.
  Permit at most one proven pre-work cold replacement within the invocation's
  existing deadline, cancellation, attempt and chain bounds.
- Add seat-record v5 beside frozen v1–v4 for the confirmed root, instance,
  harness identity and bounded launch facts selected by design D4. Amend the
  existing `boundary-record` append/version-dispatch requirement: v5 from
  0.10.0, v4 for the 0.9 line, v3 for the 0.8 line, and v1 for earlier or
  unparseable engines, identically at append, export, import verification and
  offline verification. Preserve boundary stamping and valid historical rows.
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

- `boundary-record`: amend the complete requirement “The seat record carries
  the boundary as seat-record/v4” so its append fence and manifest dispatch
  select additive v5 from 0.10.0. Preserve its boundary-stamping rule, unchanged
  scenarios and historical 0.9.0/0.9.1 compatibility example.

Existing realm boundary, manifest and gate requirements remain authoritative.
The boundary-record delta changes version selection without changing which
boundary the engine stamps or rewriting frozen contract bytes.

## Evidence and scope

Source: [issue #226](https://github.com/feedback-loop-ai/brokkr/issues/226),
the operator's 2026-09-09 commission and triage's
`.forge/tasks/226-session-resumption.md`. This change adopts that framing's
identifier and starts from shipped main
`5bc8cf305aaef9af269866cbf83f094939691399`. The successor commission and
`.forge/tasks/226-session-resumption-successor.md` adopt the surviving planning
artifacts at `169e5b94e50f0f46c04a2589f90350222ab43ff5` and return only F7 for
specification repair. Answers A–H and task repairs F1–F6 remain settled.
The separately shipped PR250 is controller-owned integration; this return
neither merges it nor assumes its changes are present.

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
`claude-lanetally`, `cargo` or `rustc` on PATH. That local limit does not erase
supplied host evidence. Two controller captures, read on this return, establish
current interface facts without establishing safe resume:

- `.forge/controller-host-provider-interface.json`, measured
  **2026-09-09T06:19:17Z**, records Claude **2.1.266** at
  `/home/vyanakiev/.local/bin/claude`, **codex-cli 0.153.4** and DSH
  **0.1.2-rc.1** at their recorded Volta paths, with successful version/help
  results. SHA-256:
  `f61a65b3128a0655cbb3eb0ccef35990438a1a936ed9b14b1ba598c0e94750db`.
  Claude lists `-r, --resume [value]`: the value is optional and omission
  opens an interactive picker. It also lists `-c/--continue`,
  `--session-id <uuid>`, `--fork-session`, `--permission-mode`,
  `--mcp-config`, `--strict-mcp-config`, `--allowedTools`, `--tools`
  (`""` disables native tools), `--no-session-persistence`, and
  `--print`/`--output-format`. These identify argv work that is now owed;
  session assignment and flag parsing alone prove neither creation nor safe
  continuation. Codex's capture is top-level help, not `exec resume` help.
  DSH's launcher shows `--resume <session>` only in a TUI-profile example.
- `.forge/controller-dsh-headless-source-interface.json`, measured
  **2026-09-09T06:29:10Z**, records `dsh --profile headless --help` and
  four hashed files from installed `@deepseek-ai/dsh-headless` 0.1.2-rc.1:
  `package.json`, `lib/startup.js`, `lib/index.js`, `cordis.patch.yml`.
  Capture SHA-256:
  `2750a127c6aa0fbe3d2a8a3195f0427c1ba07e7bc4ec370e3ea8d74cd2ff2206`.
  Headless startup accepts only task/help; the runner unconditionally calls
  `agents.create` with `session-${randomUUID()}`. Its runner configuration
  supplies a task, not a session selector. This disproves simply forwarding
  the launcher's TUI example to headless. It does not establish that supported
  session/settings/extension interfaces cannot rejoin: the capture explicitly
  leaves those interfaces for investigation.

Neither capture measures resumed restriction enforcement, exact-root rejoin,
current-only accounting or a safe pre-work rejection classifier. The complete
Claude boxed fragment and the DSH supported session route still need that
proof. LaneTally has no supplied wrapper measurement. Codex's accepted 0030
resume measurements identify **0.148.0**, while its separate shipped effort
comment cites **0.153.0**; neither is a resume measurement of installed
**0.153.4**. Installed-version remeasurement is required before delivery, per
answer G. Deterministic shims remain distinct from provider enforcement proof.

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
Design D4 selects additive v5 with matching embedding, version selection and
compatibility tests. LE2 authorizes the additive vocabulary; the modified
`boundary-record` requirement authorizes its append/version dispatch. Existing
published and embedded v1–v4 contracts, production policy, schemas, reference
and fixtures stay frozen. New v5 preserves valid unstamped 0.10.0 records,
including historical resumed rows without root evidence and third-party
refusal-bearing rows; current stamped rows obey the new producer invariants.

Issue #222 owns transcript reading and CLI/TUI derivation. This issue owns
session plumbing, launch evidence and progress markers; transcript caps and
privacy boundaries remain intact. Shared-file overlap goes to the controller,
without accessing its sibling worktree. Quota repair, DSH Git metadata repair,
trust promotion, new boundaries, global provider settings and release work
are outside this change.

## Decisions

These specification answers were recorded on 2026-09-09. Answers A-D are
retained from the prior return; E-H answer every new finding in the second
`clarify` return, incorporating the controller evidence above. Their observable
answers are scenarios in the owning deltas. The council must carry these
choices into `design.md` Decisions and the numbered rulings of proposed
0056; this is not a claim of operator acceptance.

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

- **E — Supplied interface evidence creates work; enforcement still gates use.**
  The stale claim that no provider interface evidence exists is withdrawn.
  Claude's captured explicit-handle and restriction flags must inform its
  launch construction and deterministic tests; a missing enforcement probe
  no longer excuses that argv work. AS1 separates interface availability from
  enablement. AS3 explicitly forbids bare `-r/--resume` pickers, continue,
  forks and conflicting `--session-id` assignments. Unsupported DSH headless
  syntax is not inferred from the launcher's TUI example.
- **F — The DSH headless minimum stands, with session extensions in scope.**
  Choose finding F's first reading: investigate installed `dsh-headless`,
  `dsh-session` and the agent/settings interfaces and implement a supported
  headless session-selection route if that evidence establishes one. This is
  part of #226, including a per-invocation settings/extension integration where
  supported, not the deferred plugin that replaces native tools with hands.
  The required 0056 ruling must preserve that distinction and the minimum.
  Reducing the minimum to Claude or treating the absent headless flag as proof
  of impossibility is rejected: only the startup/runner surface has been
  examined. No TUI substitution, new non-Rust production runner, provider
  installation patch or new hands admission is authorized. If the supported
  interfaces cannot meet the minimum within those rules, AS1 fails and returns
  with the measured reason; investigation is owed before such a conclusion.
- **G — Version drift requires remeasurement before delivery.** Historical
  evidence is retained as history and regression scope, not a perpetual grant
  for later provider versions. A changed installed CLI/wrapper version keeps
  its affected resume shape disabled until its interface, effective current
  restrictions, exact-root confirmation and accounting are re-established.
  For installed codex-cli 0.153.4, 0030's 0.148.0 evidence is insufficient to
  enable resume. Preserving Codex support means completing that remeasurement
  and retaining the previously supported work shapes in the delivered change;
  disabling them pending proof is incomplete preparation. The contrary
  enabled-until-contradicted rule is rejected because the original measurement
  itself found silently dropped restrictions. Proposed 0056 must state this
  qualification explicitly, without rewriting accepted 0030's historical facts.
- **H — A requested identity is not evidence that a session exists.** SR3
  permits either a provider-generated root handle or a fresh ID assigned by
  the owning engine/adapter through a measured provider creation interface.
  Both require provider evidence confirming the actual root and its owner,
  durably recorded under LE3, before SR2 admits a later offer. A preassigned
  UUID in a start payload is only intent, even if durable before spawn; a kill
  before durable provider confirmation leaves no offer from that attempt.
  AS3 bars arbitrary assignments that redirect an offer and `--fork-session`;
  SR5 and LE1/LE3 apply the same confirmation window to both mechanisms.
  This rejects both harvest-only syntax restrictions and treating assignment
  as proof of existence. Proposed 0056 must state this ownership/existence rule;
  any new record representation must obey LE2's additive-version requirement.

### F7 — Amend the standing append and dispatch requirement

The third analysis finding is adopted on its new evidence: the living
`boundary-record` requirement selects v4 for every engine at or after 0.9.0,
while design D4 selects v5 from 0.10.0. The former “None” under Modified
Capabilities and D10’s claim that no upstream amendment was needed were
incorrect. LE2 permits an additive contract but cannot amend another
capability’s standing dispatch rule.

This specify return repairs the earliest owning artifacts: the scope declaration
above and a complete MODIFIED boundary-record requirement, with explicit
version/fence, frozen-byte and historical-compatibility scenarios. Design D4,
D10 and D11, task coverage, and archive/provenance obligations follow that
amendment. Changing only downstream dispatch or deleting the tagged 0.9.0/0.9.1
example is rejected: the former leaves contradictory requirements and the
latter erases a historical fact that was never the conflict. The four adopted
ADDED deltas, answers A–H and repairs F1–F6 retain their meaning. This is a
specification amendment, not operator acceptance of proposed 0056.

## Delivery obligations

This phase authors proposal and deltas in dialect order. Design, tasks,
clarification, analysis, full implementation and specification review remain
mandatory. Clarification answers belong in scenarios; analysis choices belong
in design's `## Decisions`. Council positions require explicit reconciliation,
and earlier artifact faults return upstream. The smith performs the normal
archive/provenance operation as the final artifact task, folding all five
capability deltas: four new capabilities and the modified `boundary-record`.
Keep boundary-record’s existing provenance and append this change’s entry;
validate the folded artifacts and bidirectional provenance before committing
the complete delivery.

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

The implementation seat must recheck its own availability and read the supplied
controller captures before calling anything unmeasured. It must prepare Rust
engine/protocol offer and launch plumbing and deterministic tests, update
`adapters/{codex,claude,dsh,lanetally}.json` and packaged representations with
explicit evidence status, and record proof and limitations in the provider
guides and proposed `docs/decisions/0056-*.md`. Provider-specific construction
uses identified help/source; unmeasured enforcement keeps resume disabled,
without cancelling interface work that the evidence supports.

The council design and smith's breakdown must include these separate evidence
and implementation obligations:

- Use Claude 2.1.266's captured interface to construct and test explicit-handle
  resume and the complete current boxed fragment, selector validation, root
  confirmation and current-event accounting. A shim proves the chosen protocol
  cases, not the as-yet-unmeasured provider confirmation or enforcement behavior.
- Investigate installed `@deepseek-ai/dsh-headless`, `@deepseek-ai/dsh-session`
  and their agent/settings/extension dependencies through controller-supplied
  source with versions and hashes. Establish whether a supported per-invocation
  route can select/load the existing root session instead of minting a new one,
  retain model/effort and transcript scope, and separate historical events from
  this followup. Implement the supported route when established. Do not invent
  `dsh --profile headless --resume`; the supplied startup has no such flag.
  Source/session-extension investigation and integration are in scope despite
  the separate hands/tools-plugin deferral in `adapters/dsh.json`.
- Obtain current `codex exec resume --help` and a bounded sandbox-enforcement
  cold/resume probe from the controller on 0.153.4 (or the actual installed
  replacement), recording exact invocation, allowed argv, class re-imposition,
  same-root confirmation and current-only accounting. The historical 0030
  work shapes remain delivery obligations; new shapes need their own proof.
- Test both SR3 identity mechanisms if implemented, including an assigned ID
  persisted before spawn, a kill before durable confirmation and a successful
  confirmed creation. Configuration alone must never produce an offer or a
  resumed fact. Design must specify the representation without editing frozen
  contracts or publishing an unconfirmed value as session evidence.

The AS1 measurement/enablement tasks stay unchecked until dated controller host
evidence supplies CLI identity, exact invocation, same-root confirmation,
current restriction enforcement and current-only accounting for each required
shape, including Codex after version drift. The controller receives the missing
observations above and retains the cited raw captures with delivery evidence;
no seat reaches outside the box for provider source or homes. Prepared but gated
code is partial work, not a lawful close for #226. A measured impossibility is
reported as a failed specification requirement for the normal return path,
with its reason; it cannot silently reduce the minimum or relax 0030's safety.

## Prior specification validation — 2026-09-09

Historical results from the specify return committed as `2f45cc9`, before the
surviving design/tasks and this F7 amendment:

- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  The four deltas contain **19 requirements and 112 scenarios**, each with
  WHEN/THEN outcomes. Findings E-H are answered in the owning scenarios and
  the Decisions above; prior answers A-D remain coherent. No later judged
  clarification or analysis verdict is claimed.
- Status reports proposal/specs done, design ready and tasks awaiting design;
  planning and implementation are not complete. Decision 0056 remains reserved
  for the proposed record preceding production semantic changes. No design,
  task breakdown or decision document is authored by this specify seat.
- All five required Cargo commands were attempted with `CARGO_BUILD_JOBS=2`
  and `RUST_TEST_THREADS=2` and exited 127 because `cargo` is unavailable.
  Formatting, clippy, workspace tests and both bundle compilations have no
  passing result from this seat.
- The unchanged exact-coverage script, with those limits, `TMPDIR=/var/tmp`
  and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited 1: temporary-directory
  creation failed because `/var/tmp` is absent in this box. No coverage or
  boundary proof was obtained; controller host validation remains pending.
- The availability check resolves OpenSpec 1.12.0 but none of `codex`,
  `claude`, `dsh`, `claude-lanetally`, `cargo` or `rustc`. This pass
  incorporates the two dated controller interface/source captures, whose
  hashes match the evidence section. It performs no live provider enforcement
  probe; the identified remaining observations are explicit delivery obligations.
- `git diff --check` is clean. Frozen paths have no diff from the commissioned
  base; CI, release admission and coverage still consume
  `rust-nightly-version.txt`. This return changes the proposal and three
  affected deltas; the progress delta and existing change metadata are retained.


## Successor specify validation — F7, 2026-09-09

Adopted `226-session-resumption` and repaired F7 in dependency order: proposal,
complete MODIFIED boundary-record delta, then the affected design and tasks.
Read the dialect’s instructions through the workspace hands, including the
OpenSpec-rendered artifact instructions; no workflow runner or archive was run.
Both retained council positions were read in full and the affected D10 claim
was reconciled under design’s Decisions (D12). Answers A–H and task repairs
F1–F6 remain settled; no provider minimum was reduced.

- `openspec validate 226-session-resumption --strict --no-interactive` passed.
  `openspec show 226-session-resumption --json --deltas-only` parses 19 ADDED
  requirements and one MODIFIED boundary-record requirement. All five declared
  capabilities have their deltas: **20 requirements / 123 scenarios**.
- `openspec status --change 226-session-resumption --json` reports all four
  planning artifacts done and planning complete. This reports artifact state;
  this author does not claim the successor’s independent clarification,
  analysis, implementation, verification or review verdicts.
- A structural preservation/coverage check passed: the original four deltas
  are byte-identical to the inherited HEAD; the modified requirement retains
  all seven original scenarios, with only dispatch amended, and its tagged
  0.9.0/0.9.1 compatibility sentence is unchanged. Four new scenarios cover
  all-fence dispatch, historical 0.10.0 rows, v5 boundary authority and frozen
  versions. An in-memory replacement preserves the two other standing
  requirements and existing provenance; the living spec was not edited.
- All 20 requirements have task citations. All 101 inherited task identifiers
  remain, with only 2.8 added for boundary integration regressions; **102 tasks
  remain unchecked**. The new requirement’s 11 scenarios are mapped explicitly
  in tasks.md. Archive/commit tasks now cover all five capabilities and retain
  boundary-record’s earlier provenance entry.
- All five required Cargo commands above were attempted with
  `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` and exited **127** because Cargo is
  absent in this box. Format, clippy, workspace tests and both bundle compiles
  have no passing result from this seat. No implementation test is claimed.
- The unchanged exact-coverage script, under the same limits plus
  `TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited **1** because
  `/var/tmp` is absent and `mktemp` could not create its directory. Host exact
  coverage and boundary proof remain pending; the gate was not weakened.
- `git diff --check` passed. Frozen contracts (published and embedded),
  production policy, reference, fixtures and accepted decisions have no diff
  from the commissioned base. CI, release admission and coverage still consume
  `rust-nightly-version.txt`. No production, living-spec, sibling or provider
  installation file was changed. Decision 0056 remains reserved/proposed work
  for task 1.1 before production semantics; no decision was accepted here.
