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
  verification remain visibly pending. Complete every repository-local tracked
  task while the change is active and keep exact-head controller evidence as a
  mandatory handoff condition outside tracked checkboxes, so recording the
  result cannot change the head it validates. The non-SDD implementer is outside
  this progress rule; dialects continue to own task paths and formats.
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
- `sdd-progress-markers`: durable mid-phase progress, honest recovery and
  exact-head handoff evidence that never requires judges or controllers to edit
  an archived artifact.

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

The quota-successor commission adopts the completed F7 repair at `c56fa73`.
Its current triage framing, `.forge/tasks/226-resume-triage.md`, belongs to run
`close-issue-226-only-codex-resum-805ec715`. The prior clarification attempts
ended in provider quota failures, not clarification verdicts. This specify
visit validates the existing proposal and five deltas; it preserves settled
answers A–H, repairs F1–F7 and the full delivery minimum. Outstanding judged
phases and implementation remain required under the same change identifier.

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

These specification answers were recorded across the returns on 2026-09-09
and 2026-09-10. Answers A–H remain settled; I adopts the same reopened change
and is revised here with C to answer the latest clarification. Their observable
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
- **C — Progress is standing behavior; delivery particulars belong to the
  change.** PM1 applies to each current change's dialect task artifact. PM4
  retains requirement coverage, truthful verification, frozen-fixture
  protection and the distinction between tracked repository-local work and
  exact-head handoff evidence. Run-specific commands, resource limits and
  repository-local responsibilities below become explicit tracked tasks. A
  controller result whose subject is the immutable delivery commit or a later
  integrated head remains mandatory, but is stated as a non-checkbox handoff
  condition and recorded in the controller journal or evidence store keyed to
  that head. Its absence blocks the corresponding delivery, publication or
  closure claim; it does not leave an archived OpenSpec task unticked or cause a
  post-commit artifact edit. Requiring every controller responsibility to be a
  tracked task is rejected because exact-head proof would invalidate itself
  when its checkbox changed. Decision 0042's one-way fold makes either hidden
  standing commands or an unfinishable archived task a specification fault.
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

- **I — An unfinished delivery remains the same active change, but this
  change cannot be archived again under the current archive constitution.**
  The implementation commit archived `226-session-resumption` while ten
  required tasks remained honestly unchecked. Reopening that same identifier
  preserves the surviving design, task progress and implementation; it does
  not author a competing change. It did, however, remove the dated
  `2026-09-09-226-session-resumption` directory while leaving five append-only
  living provenance pointers to it.

  F12's exact probe disproves the prior idempotent-finalization claim. Restoring
  the 2026-09-09 archive byte-for-byte from `75ae68e` makes those pointers
  resolve but makes `openspec validate --archived --strict --no-interactive`
  fail on its truthful 92/102 task state. Omitting that archive leaves five
  dangling pointers. A normal archive now creates a new date-derived directory,
  so appending its required provenance does not repair the old pointers.
  Rewriting/removing the old lines violates decision 0042's append-only rule;
  checking the old tasks falsifies their completion; manually renaming the new
  archive or using `--skip-specs` bypasses the declared dialect operation or
  strands PM4.

  No downstream ordering can satisfy all of those accepted invariants. The
  change therefore remains active and archive readiness remains pending until
  the operator supplies a sanctioned recovery rule and compatible verifier
  behavior for a prematurely folded, reopened change. The minimum upstream
  mechanism must preserve the truthful incomplete historical fold, keep both
  provenance directions resolvable, permit a later all-ticked fold with its own
  actual archive identity, and make archived verification recognize that
  explicit supersession without treating incomplete work as complete. Because
  accepted decision 0042 currently says every folded change with an unticked
  task fails verification, only an operator-approved amendment plus supporting
  dialect/OpenSpec validation can authorize that state. Provider delivery,
  local gates and exact-head controller evidence remain required and unwaived.

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
latter erases a historical fact that was never the conflict. The four originally adopted capability deltas, answers A–H and repairs
F1–F6 retain their meaning. This is a specification amendment, not operator
acceptance of proposed 0056.

### F10 clarification return — exact-head evidence and the second archive

Both returned questions are adopted as defects in the earlier F10 account.
First, its claim that proposal answer C needed no amendment conflicted with
PM4's blanket rule that unavailable external proof stays an unticked task.
Answer C and PM4 now distinguish attainable pre-archive task evidence from
mandatory exact-head controller evidence. The latter is explicit in the
proposal and task prose but has no checkbox; its real result is recorded
outside the repository artifact and still gates every delivery or closure claim.

Second, answer I and Delivery obligations previously demanded both a manual
no-fold move and a fresh five-capability fold with another provenance append.
Neither is retained. OpenSpec probes provide the deciding evidence: the normal
archive operation against an all-ticked copy of the inherited, already-folded
change archived successfully with zero spec updates and unchanged living bytes;
changing the still-`ADDED` PM4 then made the same operation refuse the duplicate
requirement without changing files. The repaired delta therefore retains the
three already-folded progress additions as additions and moves the changed PM4
block under `MODIFIED Requirements`. The eventual normal archive can apply that
one repair, no-op the identical deltas and retain the existing provenance lines
once. `--skip-specs` would strand the repair, while a manual move would evade the
dialect operation. The downstream design and task visits must replace both old
instructions with this order; doing so here would cross the specification
office's artifact wall. Its archive-identity conclusion is superseded by F12.

### F12 specify return — upstream archive recovery is required

The latest analysis finding is adopted, and an exact scratch reproduction
establishes that specification prose alone cannot repair it. With every active
task ticked in the copy, OpenSpec 1.12.0 archived the undated change as
`2026-09-10-226-session-resumption`, applied only PM4 semantically and left the
other four deltas as no-ops. Restoring the exact historical archive from
`75ae68e` and appending the new directory's provenance made the trail resolve
in both directions. The mandatory archived validator then reported two archives
passed and one failed: the restored
`2026-09-09-226-session-resumption` retained its truthful ten incomplete tasks
(92/102). Removing or modifying that evidence would merely hide the original
premature fold.

The reasoned refusal is therefore owned here. The previous plan to retain one
pointer while producing a differently named archive is impossible; the proposed
two-archive repair is also impossible under decision 0042's unconditional
all-ticked archived verification. The operator must authorize a lossless
supersession state (or another equally truthful recovery) and corresponding
dialect/verifier support before this change may archive. Such a state must keep
the historical tasks unticked, keep their dated archive and existing pointers,
append the later archive identity normally, link the two folds explicitly and
allow `openspec validate --archived --strict --no-interactive` to distinguish a
superseded premature fold from an incomplete current delivery. Until then,
design and tasks must expose the upstream block and prohibit archive rather than
fabricate completion.

## Delivery obligations

This phase authors proposal and deltas in dialect order. Design, tasks,
clarification, analysis, full implementation and specification review remain
mandatory. Clarification answers belong in scenarios; analysis choices belong
in design's `## Decisions`. Council positions require explicit reconciliation,
and earlier artifact faults return upstream. After every tracked
repository-local task and progress edit is complete while the change is active,
the smith normally performs the archive effect after the final artifact task.
F12's upstream requirement currently blocks this change before that step.
No seat may restore an archived copy that fails mandatory verification, discard
the historical fold, rewrite its five provenance lines, preselect a dated name
the normal command will not produce or mark incomplete work complete. After an
operator-approved recovery rule and compatible verifier exist, revise this
proposal, PM4, design and tasks coherently before any archive attempt. Strict
archived validation and bidirectional provenance checks remain mandatory and
precede the delivery commit.

The smith's breakdown must carry these commission-specific obligations, with
separate unchecked tasks for measurements and validation attainable before the
final artifact operation. Exact delivery-head and integrated-head controller
results are mandatory non-checkbox handoff conditions, recorded outside the
tracked artifact after those heads exist. Extend
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

## Quota-successor specify validation — 2026-09-09

Adopted the committed proposal, five deltas, design and tasks. Read the current
triage framing, controller evidence index and dialect-rendered proposal/spec
instructions through the workspace hands. No new semantic choice or unresolved
specification defect was identified in this adoption check; the F7 amendment
and its dependent design/task coverage remain intact. Quota failures neither
settle clarification nor reduce AS1’s measured-provider delivery minimum.

- OpenSpec strict validation passed; its delta parser reads 19 ADDED
  requirements and one MODIFIED boundary-record requirement across five
  capabilities: **20 requirements / 123 scenarios**. Status reports all four
  planning artifacts done; this is artifact state, not a judged-loop verdict.
- Structural preservation checks passed: all four original deltas are
  byte-identical to `169e5b9`; all seven original boundary scenarios survive
  with only dispatch amended and the tagged 0.9.0/0.9.1 sentence unchanged.
  All eleven boundary scenarios have explicit task mappings. An in-memory
  replacement preserves the living capability’s other requirements and
  provenance. All 101 inherited task IDs plus 2.8 remain: **102 unchecked**.
- With `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`, attempts to spawn the five
  required Cargo commands failed with `ENOENT` because Cargo is absent.
  Format, clippy, workspace tests and both bundle compiles have no passing
  result from this visit. The unchanged exact-coverage script, also using
  `TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, exited **1** because
  `/var/tmp` is absent. Controller host coverage and boundary proof stay pending.
- CI, release admission and coverage still consume `rust-nightly-version.txt`.
  Frozen published/embedded contracts, policy, reference, fixtures, accepted
  decisions, production and living specs have no diff from the commissioned
  base. This visit records adoption/validation only in this proposal; the
  deltas, design, tasks and metadata retain their committed bytes.

No workflow runner, archive or provider probe was invoked. Independent
clarification, design reconciliation, tasks/analysis, proposed decision 0056,
implementation, verification and review remain delivery work. Source captures
are still interface evidence only; no live resume, enforcement or accounting
proof is claimed.

## Current successor specify return — F10 clarification, 2026-09-10

This visit adopts `226-session-resumption` and answers both ambiguities returned
from clarify in their earliest owning specification artifacts. Proposal answers
C and I, the Delivery obligations, and complete PM4 now separate tracked
repository-local closure from mandatory exact-head controller evidence and
select the normal archive operation for the returned fold. The other nineteen
requirements, their scenarios, provider minimum, ownership, work/gate split,
current-only accounting, v5 dispatch and answers A–H remain unchanged.

- `openspec validate 226-session-resumption --strict --no-interactive` passes.
  `openspec show 226-session-resumption --json --deltas-only` parses **20
  requirements / 125 scenarios**: 18 ADDED and two MODIFIED requirements across
  the same five capability deltas. PM4 is the only changed delta requirement;
  its complete original scenarios remain and two archive scenarios are added.
- An isolated probe copied the inherited active change and living truth, ticked
  every task in the copy, and ran the normal archive operation. With inherited
  deltas it archived and passed strict archived verification with zero spec
  updates and byte-identical living capabilities. With repaired PM4 represented
  as ADDED it refused the duplicate without changing files. With repaired PM4
  represented as MODIFIED it archived, passed strict archived verification and
  reported exactly one modified requirement; the other four living capability
  files were byte-identical and every capability retained exactly one provenance
  pointer for this change.
- The selected operation is therefore `openspec archive
  226-session-resumption --yes` after all tracked tasks are complete. A manual
  move is not the dialect operation, and `--skip-specs` would omit this PM4
  repair. Archived validation, the delivery commit and controller evidence
  follow without editing the archived task artifact.
- `openspec status --change 226-session-resumption --json` still reports the
  proposal, specs, design and tasks present. The existing design and task
  instructions are downstream artifacts and deliberately remain untouched by
  this specify office; their phase visits must adopt answers C/I and PM4 before
  implementation resumes. Current task truth remains 83 checked / 18 unchecked
  across 101 tracked tasks.
- `git diff --check` passes. Frozen contracts, policy, reference, fixtures,
  living capabilities, production code and provider declarations are untouched.
  `cargo fmt --all -- --check` was attempted with the commissioned job/thread
  limits and exited 127 because `cargo` is absent from this boxed workspace;
  no Rust validation is claimed by this specification visit. No archive,
  workflow runner, provider probe, push or merge was performed against the real
  change.

## Current successor specify return — F12, 2026-09-10

This visit adopts the existing `226-session-resumption` change and the latest
HIGH finding. It preserves A–H and every provider/runtime requirement, but
revises answer I and complete PM4 on new executable evidence. The result is
**upstream**, not a narrowed draft: accepted decision 0042 and its current
OpenSpec binding lack a truthful recovery state for the premature fold.

- `openspec validate 226-session-resumption --strict --no-interactive` passes,
  and status reports all planning artifacts complete. The five deltas still
  contain **20 requirements / 125 scenarios**.
- The exact scratch probe restored the historical archive from `75ae68e`,
  ticked only the copied active tasks, ran the normal archive operation and
  observed the new `2026-09-10-226-session-resumption` identity with only PM4
  semantically modified. After appending its five required provenance lines,
  both provenance directions resolved. Mandatory archived validation then
  failed only the restored historical archive at its truthful **92/102** state;
  the other two archives passed.
- The real tree still has five dangling 2026-09-09 provenance pointers because
  the reopened active change replaced that archive. Restoring it without an
  approved supersession rule would knowingly make archived verification fail,
  so no real archive or living-spec mutation was performed.
- The task record has **101 unique identifiers, 81 complete / 20 pending** after
  reopening 15.5 for the upstream recovery. `git diff --check` passes.
  `cargo` is unavailable in this boxed workspace, so the focused Rust
  provenance test did not run; no Rust, provider, host-coverage or delivery
  success is claimed.

Only proposal, the PM4 delta, design and tasks change. Production code, living
specs, frozen contracts/fixtures/policy/reference, accepted decisions, provider
settings and sibling worktrees remain untouched. No workflow runner, real
archive, push, merge, publication or issue action was invoked.
