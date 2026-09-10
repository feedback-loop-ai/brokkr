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
  remeasurement. Qualify an isolated exact-version DSH route using the latest
  official core release, `@deepseek-ai/dsh` 0.1.5-rc.1 at
  `183f08e9c6dde7e36cd2318eaee70b0da08fb35e` (or a newer release published by
  qualification time, recorded as such), with a repository-owned adaptation of
  `dsh-plugin-cli-session` 0.2.0 at
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda`. The adaptation's only delta moves
  the plugin's post-turn fold onto that core's public session-event accessor;
  it runs through the documented extension and `agents.resume` APIs and is
  pinned by digest. No older core is selected. The deferred hands/tools plugin
  remains separate. Implement every measured safe shape; declare unsupported or
  unmeasured shapes honestly. Cold-only, declaration-gated preparation does not
  close #226. Re-impose current restrictions, hands and model/effort settings on
  every rejoin; inherited permissions and argument parsing do not prove safety.
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
`.forge/tasks/226-session-resumption.md`. This change adopts that framing and
retains its original archive identity
`2026-09-09-226-session-resumption`; it starts from shipped main
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

- `.forge/tasks/controller-dsh-upstream-discovery.json`,
  `controller-evidence-2026-09-10.md`,
  `.forge/tasks/dsh-pair-incompatibility.json` and
  `.forge/tasks/dsh-pair-qualification-010rc6.json` correct that limited
  conclusion. The official API exposes explicit persisted identity through
  `ctx.agents.resume({ resumeSessionId, agentOptions, setup })` and its agent
  loop. Source inspection of the community `dsh-plugin-cli-session` 0.2.0 at
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda` confirms a real extension using
  `agents.resume`, an explicit `--session` selector, current model selection
  and a pre-followup `firstSeq` boundary for output/usage collection. On the
  latest core, **0.1.5-rc.1**, a live isolated probe of the unmodified plugin
  runs both model turns into one retained root and recalls the nonce, then
  throws `dsh: events is not iterable` in the plugin's post-turn fold (built
  `lib/index.js:253`, `agent.session.events`) and emits no result envelope.
  A later seat re-pinned to core **0.1.0-rc.6**, the plugin's development
  matrix, whose host modules still expose that accessor, and measured a
  compatible cold+warm pair there. The operator's 2026-09-10 ruling reverses
  that downgrade (answer M). The 0.1.0-rc.6 measurement remains dated history
  and transfers no evidence to 0.1.5-rc.1. The plugin's emitted `session_id`
  copies the normalized request rather than independently comparing the
  returned session, and its latest in-range message usage is last-wins rather
  than proven noncumulative.
- This specify visit read, without executing, the task-owned 0.1.5-rc.1
  installation under `.forge/dsh-qualify/core`. The core `package.json`
  (sha256 `8da881a5aa7d371dc1244ebb7acb8dfa9ac5fafce50404b31185d89cf152763a`)
  and each host module the plugin or headless bundle resolves — `dsh-session`,
  `dsh-agent`, `dsh-llm`, `dsh-cmdline`, `dsh-headless`, `dsh-base` and
  `dsh-agent-loop` — are 0.1.5-rc.1. The declared type surface of
  `@deepseek-ai/dsh-session` (`lib/types/index.d.ts:187`, sha256
  `ed327445b83ca8d699eb22991178362e4458172ba3c717a7a894f4907394fc37`) exposes
  `Session.snapshotEvents(fromSeq?, toSeqExclusive?)`, a frozen snapshot of
  the half-open sequence range, alongside `eventAt(seq)`, `ownEvents()`,
  `firstLiveSeq` and `get seq()`, and declares no `events` member. Its
  implementation (`lib/index.js:1107`, sha256
  `05e94f57d96e7979670a5b51024c8591572eb0051ce793613dbdec35cf2c47bf`) slices
  the same private append-only log the removed getter exposed. The plugin's
  `firstSeq` is `agent.session.seq` captured after `agents.resume` settles and
  before the follow-up (built `lib/index.js:235`), and all three folds skip
  earlier sequences; `snapshotEvents(firstSeq)` therefore yields exactly the
  interval they consume. This is a declared, lawful replacement for a
  one-accessor adaptation; its live behavior on the adapted pair is still
  unmeasured. The same type surface records a failed or retried model attempt
  as `assistant/attempt`, whose declared data carries no `usage` field, which
  bears directly on task 10.7's retry-accounting boundary.
- The registry document cached by that isolated install at
  2026-09-10T11:30:55Z tags `latest` and `next` as 0.1.5-rc.1 (published
  2026-09-10T03:12:53Z) and `alpha` as 0.1.5-alpha.2. The live registry is
  unreachable from this box, so the qualifying seat re-resolves the tags at
  run time and records the result.
- The four `controller-claude-*-probe.json` artifacts and accounting analysis dated
  **2026-09-10** establish same-root continuity on Claude **2.1.266**, observed
  Read-grant replacement, and partial native-tool and MCP removal. Three sample
  pairs reconcile token usage to current provider message IDs. The Read-grant
  and MCP resumes have exceptional visible-message/turn relationships, and the
  MCP resume's total usage does not reconcile to its one visible completed
  message. These are partial observations, not full boxed-fragment admission.

The older captures alone measure none of resumed restriction enforcement,
exact-root rejoin, current-only accounting or a safe pre-work rejection
classifier. The later Claude probes resolve only the observations stated above;
the complete boxed fragment and exceptional accounting attribution remain to be
proved. The selected DSH route still requires isolated dependency compatibility,
exact-root, restriction-precedence and current-sequence accounting proof before
enablement. LaneTally has no supplied wrapper measurement. Codex's accepted 0030
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

The DSH dependency/profile used for qualification is installed only under this
worktree or task-owned temporary storage and is pinned to the core release
above and to the repository-owned plugin adaptation's upstream commit and byte
digests, with its resolved dependency identity recorded. It does not alter the
live global DSH pin, profiles, credentials or other runs. Brokkr's production
integration remains Rust under `crates/`; loading a provider plugin through
DSH's documented extension API does not authorize a second Brokkr runner, a
provider-package patch or UUID interception. The adaptation is
extension-boundary source, the side decision 0009 leaves language-neutral. It
is tracked at a location design D6 names, outside `crates/` and every frozen
tree, and keeps the upstream MIT licence and provenance. It runs only inside
DSH's plugin loader and is never built into, loaded by or executed by Brokkr.
Its bytes join the pinned composite identity; they do not become a Brokkr
runtime.

## Decisions

These specification answers were recorded across the returns on 2026-09-09
and 2026-09-10. Answers A–K remain settled; L corrects the delta operation for
their new semantics without reopening them, and M applies the operator's
2026-09-10 ruling to K's pinned core without reopening K's route. Their
observable answers are scenarios in the owning deltas. The council must carry
these choices into `design.md` Decisions and the numbered rulings of proposed
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
  Choose finding F's first reading: investigate DSH core, `dsh-headless`,
  `dsh-session` and the agent/settings extension interfaces and implement a
  supported headless session-selection route when evidence establishes one.
  Answer K now identifies the exact route to qualify. This is part of #226,
  including per-invocation session-extension integration, not the deferred
  plugin that replaces native tools with hands.
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

- **I — An unfinished delivery remains the same dated active change, and the
  normal archive operation preserves that identity.** The implementation commit
  prematurely archived `2026-09-09-226-session-resumption` while ten required
  tasks remained honestly unchecked. Decision 0042 explicitly permits a
  returned smith to reopen the archived change with `git mv`, continue that
  same change and fold it again. The earlier reopening incorrectly dropped the
  date prefix and thereby created F12's apparent two-archive conflict. Restoring
  the active directory to `2026-09-09-226-session-resumption` preserves the
  original change identity, surviving design, truthful current task progress
  and implementation; it is not a new or competing change.

  Installed OpenSpec 1.12.0 settles the archive-name behavior. Its
  `ARCHIVE_DATE_PREFIX_PATTERN` branch preserves a `changeName` that already
  begins with `YYYY-MM-DD-`, specifically so a restored change keeps its
  original identity on a later archive date. The controller's isolated probe
  copied the current tree, used the dated active identifier, ticked tasks only
  in that scratch copy, and observed active strict validation, the normal
  archive command and strict archived validation all exit zero. The archive
  remained `2026-09-09-226-session-resumption`; at that time only PM4 changed
  semantically, and all five existing provenance sections remained byte-for-byte
  unchanged with their single pointers resolving in both directions. That probe
  predates K's material AS1–AS3 amendments, so it proves identity preservation
  but not the current fold; L supplies the corrected operation and evidence.

  The real change therefore remains active and its real tasks remain at their
  truthful states until their work and evidence complete. After every tracked
  repository-local task is checked, the smith runs `openspec archive
  2026-09-09-226-session-resumption --yes` as the normal final artifact
  operation. This is neither a manual post-archive rename, date spoof,
  supersession exception, `--skip-specs` nor waiver; it is decision 0042's
  existing reopen-and-refold path exercised with the original dated identity.
  Provider delivery, local gates, archived validation, Rust bidirectional
  provenance checks and exact-head controller evidence remain required and
  unwaived.

- **J — The commissioned release-binary build is tracked repository-local
  proof.** The supplied release configuration and the adopted intake both
  require `cargo build --release --locked -p brokkr-cli`. It can be completed
  against the active worktree before the final artifact operation, so PM4
  classifies it as a repository-local pre-archive obligation: the proposal
  names it, and the downstream design and task breakdown must give it an
  explicit gate whose checkbox stays pending until the command really passes.
  Treating this as inapplicable merely because #226 is not itself a versioned
  release is rejected; the realm supplied the command as part of every
  change's validation, and neither the commission nor an accepted decision
  provides an exemption. This does not turn remote CI or final-head evidence
  into tracked work: those results can exist only after their immutable
  subject heads and remain controller-owned non-checkbox handoff conditions.

- **K — Qualify the pinned DSH core/plugin extension route; do not preserve the
  disproven global blocker.** Official core 0.1.5-rc.1 at
  `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, the latest release (answer M),
  supplies explicit persisted-root resume in core, and
  `dsh-plugin-cli-session` 0.2.0 at
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda` is the selected headless CLI
  extension, carried as a repository-owned adaptation. Its inspected source
  calls the official `agents.resume` API with an explicit session, selects the
  current model and bounds output and usage from the current sequence. This is
  a documented extension mechanism, not a patched installed package, UUID
  interception or a guessed TUI flag. Core 0.1.5-rc.1 removed the
  `agent.session.events` accessor the upstream plugin reads (`dsh: events is
  not iterable`). That is a measured pair incompatibility, not a global DSH or
  extension limitation, and M resolves it forward through the core's declared
  `snapshotEvents` accessor instead of selecting an older core.

  The source also bounds what can be claimed before the probe: the plugin's
  printed `session_id` is derived from the request, and `collectUsage` chooses
  the latest assistant usage after `firstSeq` without establishing whether a
  multi-call payload is attributable. Official core opens persisted state by
  the exact requested ID and tests continued history. On 0.1.5-rc.1 the
  unmodified plugin already ran both turns, continued one root and recalled the
  nonce before its fold failed. The earlier 0.1.0-rc.6 cold+warm measurement
  is dated history for that superseded pin and qualifies nothing on
  0.1.5-rc.1. Compatibility of the adapted pair, a successful result envelope,
  independent root confirmation, restriction precedence and multi-message
  attribution are focused proof work on the selected pair, not grounds to
  reinstate the old global limitation.

  The selected route is installed and exercised only in an isolated worktree or
  task-owned profile with its complete dependency identity recorded. Admission
  still requires proof that the adapted plugin, identified by its upstream
  commit and delta digests, loads compatibly on that exact core release,
  rejoins the offered provider-confirmed root, preserves the admitted headless
  shape and current model/effort and restriction precedence, and emits only
  current-sequence output and attributable usage. Until those observations and
  the Rust adapter path agree, DSH remains disabled and AS1 incomplete. The
  official SDK is not selected for this change because it would introduce a
  separate production runner/control surface where the documented CLI extension
  path already exists; an incompatibility returns as the precise unmet AS1
  requirement rather than authorizing an unreviewed substitution or a
  downgrade.

  The controller's Claude 2.1.266 probes likewise amend evidence status, not the
  rule: same-root continuity and observed replacement restrictions count toward
  qualification, while partial native-tool/MCP evidence and unreconciled
  exceptional turn/usage cases keep full boxed admission pending. Neither a
  worker-home write error nor these partial observations is a controller
  blocker or a completed proof task.

- **L — Fold the amended existing safety rules as MODIFIED requirements.** The
  living `adapter-resume-safety` capability already contains AS1–AS5 with this
  change's singular provenance. K materially amends the complete AS1 rule and
  adds obligations to existing AS2 and AS3 scenarios. Keeping those three
  requirements under `ADDED` would misstate the operation and cause the normal
  archive to refuse the already-existing AS1; strict active validation alone
  does not detect that collision. AS1, AS2 and AS3 are therefore complete
  `MODIFIED Requirements`, while byte-identical AS4 and AS5 retain their
  historical `ADDED` classification. PM4 remains MODIFIED from F10.

  The eventual normal dated archive must apply the complete AS1–AS3 and PM4
  replacements, no-op the other sixteen unchanged requirements, preserve all
  five existing provenance sections byte-for-byte and singularly, and pass
  strict archived plus bidirectional-provenance verification. PM4's normal
  rearchive scenario owns this observable answer. Retaining AS1–AS3 as ADDED
  merely because they originated in this change is rejected: after the earlier
  fold, the living capability—not historical origin—determines the operation.

- **M — Operator ruling 2026-09-10: pin DSH forward to its latest release and
  adapt the plugin in a repository-owned form; never downgrade.** The ruling
  reverses `a86eca1`, which re-pinned the route to core 0.1.0-rc.6 because the
  plugin's development matrix pins it. The selected core is the latest
  published release, `@deepseek-ai/dsh` 0.1.5-rc.1 at
  `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, tagged `latest` and `next`. The
  qualifying seat re-resolves the tags at run time. If a newer release has been
  published, it is selected instead, and the evidence names the exact version
  and how it was resolved. Evidence never transfers between core versions in
  either direction. The 0.1.0-rc.6 measurement remains dated history, and
  records of it gain a new dated reversal entry instead of being rewritten.

  Discovery answers the blocker forward. The removed getter's declared public
  replacement is `Session.snapshotEvents(fromSeq?, toSeqExclusive?)` in
  `@deepseek-ai/dsh-session` 0.1.5-rc.1, which slices the same append-only log.
  The plugin fixes `firstSeq` after `agents.resume` settles and before its
  follow-up, and its three folds already skip earlier sequences, so reading
  `snapshotEvents(firstSeq)` preserves the upstream semantics exactly.
  `ownEvents()` is rejected as the replacement because its boundary is fork
  lineage, not the current invocation. `firstLiveSeq` marks the constructor
  seed, which the plugin's own `firstSeq` already follows. The adaptation is
  therefore a repository-owned copy of the plugin at upstream commit
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda` whose only delta is that
  accessor, in its source and built bytes, reviewable line for line against the
  upstream. Its identity is the upstream commit plus the digests of the delta
  and of the installed bytes. The composite identity and assessment admit that
  digest, never the unchanged package name or version. It keeps the upstream
  CLI surface: `--new`, `--session <id>`, `--output-format stream-json`, the
  `agents.resume` path and the `firstSeq` interval.

  The commission asks the adaptation to keep "its presentAs/restrict/guard/
  register surface and the one boxed tool". The upstream bytes answer that
  phrase rather than bear it out. The session plugin at `0f487e74` provides a
  `cli-startup` service and one Cordis `cli-runner` injecting
  `agentDefaultModel`, `agents`, `sessions` and `sessionPersistence`. It
  registers no tool and has no presentAs/restrict/guard/register surface. That
  surface and the one boxed tool belong to the deferred hands/tools plugin that
  decision 0043 names and `adapters/dsh.json` still defers. The adaptation
  keeps exactly the upstream surface, adds, widens or admits no tool, and
  leaves DSH hands unsupported and the hands deferral text unchanged.

  A repository-owned plugin is a semantic change to D6. D6 permitted an
  adapter-owned Cordis extension only beside the maintained plugin, for a
  missing policy or pre-work signal. This fault is inside the runner's own
  fold, so an extension beside it cannot cure it without becoming a second
  runner. Design D6 must name the adaptation's location, build, provenance and
  digest inputs. D10 and proposed decision 0056 ruling 5 must record the
  admission of a repository-owned plugin; only the operator accepts 0056.
  Rejected alternatives: re-pinning any older core (ruled out by the
  operator); patching the installed package in place, or a runtime shim that
  restores a `Session.events` member (an in-place patch and a fabricated
  surface); the SDK runner (K); and taking the probe's decoded session log as
  the route's result in place of the plugin's envelope (probe tooling, not a
  runner). If the live qualification shows the declared accessor cannot supply
  the fold's interval, the seat records the missing module, symbol and version
  with the reproducing probe, names a residual and parks with that upstream ask
  for DeepSeek. It neither downgrades nor claims a global DSH limitation.
  Discovery finds a lawful replacement, so that branch is a scenario, not the
  plan.

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
office's artifact wall. Its archive-identity conclusion is corrected by F12.

### F12 specify return — preserve the dated identity on normal rearchive

The finding is accepted as an identity error in the earlier reopening, while
its proposed constitutional remedy is rejected on new controller evidence.
Installed OpenSpec 1.12.0 `archive.js` lines 1115–1127 test
`ARCHIVE_DATE_PREFIX_PATTERN` before choosing the destination: an already dated
`changeName` is retained instead of receiving the current date. Decision 0042
already permits reopening an archived change with `git mv` and folding that
same change again. Together these facts provide the supported recovery path:
restore the active directory to
`2026-09-09-226-session-resumption`, retain that identity in every active
command and artifact, finish the real work, then invoke the normal archive
operation on that dated identifier.

The supplied scratch probe establishes mechanism feasibility without claiming
real completion. After tasks were ticked only in the copy, `openspec validate
2026-09-09-226-session-resumption --strict --no-interactive`, `openspec archive
2026-09-09-226-session-resumption --yes` and strict archived validation all
passed. The resulting archive kept the exact historical identity; the five
existing provenance sections stayed byte-for-byte unchanged and their pointers
resolved in both directions. The earlier probe's differently dated second
archive was an artifact of having first removed the original prefix, not an
OpenSpec or decision 0042 limitation.

No supersession state, verifier exception or accepted-rule amendment is needed
or authorized. This repair does not restore a separate incomplete historical
archive, rewrite provenance, tick historical work, select an archive name after
the operation or skip PM4's fold. Historical commits remain the truthful record
of the premature fold. The active task artifact is the continuing truth for the
same change, and normal final archive remains blocked only by genuinely pending
tasks and gates.

## Delivery obligations

This phase authors proposal and deltas in dialect order. Design, tasks,
clarification, analysis, full implementation and specification review remain
mandatory. Clarification answers belong in scenarios; analysis choices belong
in design's `## Decisions`. Council positions require explicit reconciliation,
and earlier artifact faults return upstream. This change has been restored to
its original active identity `2026-09-09-226-session-resumption`. After every
tracked repository-local task and progress edit is complete while it is active,
the smith performs `openspec archive
2026-09-09-226-session-resumption --yes` after the final artifact task. The
normal operation preserves the date-prefixed identity, applies repaired PM4,
applies the complete amended AS1–AS3 requirements, no-ops the other sixteen
unchanged requirements and retains each of the five existing provenance
pointers exactly once. It is not replaced by a manual archive move or
`--skip-specs`. Strict archived validation and Rust bidirectional-provenance
checks remain mandatory and precede the delivery commit.

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
cargo build --release --locked -p brokkr-cli
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
- Use an isolated dependency/profile installation of official DSH 0.1.5-rc.1
  at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the newer release that
  run-time registry resolution finds and records, with the repository-owned
  adaptation of `dsh-plugin-cli-session` 0.2.0 at
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda` (answer M). First record the
  discovery: module, symbol, signature, semantics against `firstSeq`, and the
  bytes and hashes that declare the replacement accessor. Then record the
  resolved dependency graph, Node runtime, adaptation provenance and digests,
  Cordis patch and composed-profile hashes. Prove the adapted plugin loads
  through the documented API without altering the live DSH installation,
  profiles or credentials, snapshotting them before and after. Never re-pin an
  older core; if no lawful replacement exists, record the missing surface with
  its probe and park with the upstream ask. Write the live evidence to
  `.forge/tasks/dsh-pair-qualification-015rc1.json`, beside the superseded
  0.1.0-rc.6 file. Qualifying and implementing seats use Claude and Flash only,
  with no Codex/OpenAI model call or fallback.
  Qualify a bounded cold/resume pair in which cold work creates and confirms a
  persisted root and private nonce, then `--session <owned-root>` rejoins that
  exact root and recalls it while using the current invocation's headless
  profile and model/effort. Verify that an alternate selector/profile cannot
  override the adapter-owned values. Compare the adapted plugin's `firstSeq`
  boundary, raw sequence and message identities and usage across a
  multi-message, tool and retried-attempt interval after historical sequences,
  so old output/tools/tokens are absent and every reported current total is
  attributable. Omission is required where it is not, including for retried
  attempts whose declared events carry no usage. Implement the Rust route
  against D6's private provider-ID and persistence-locator target, and enable
  it only after those observations pass.
  Do not invent `dsh --profile headless --resume`, patch a package, intercept a
  UUID, admit hands, or substitute TUI/SDK execution. The deferred hands/tools
  plugin in `adapters/dsh.json` remains separate.
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

## Prior successor specify return — F10 clarification, 2026-09-10

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

## Current successor specify return — F12 resolution, 2026-09-10

This visit adopts committed work at `7178895`, restores the existing change to
its original active identity `2026-09-09-226-session-resumption`, and answers
F12 without narrowing any provider/runtime requirement or amending accepted
decision 0042. Installed OpenSpec 1.12.0 preserves an already date-prefixed
`changeName`; the supplied dated scratch probe confirms that the normal archive
later retains this exact identity, applies only PM4 semantically, preserves all
five provenance sections byte-for-byte and passes strict archived validation.

The five deltas retain **20 requirements / 125 scenarios**. PM4's two F12
scenarios now specify the dated same-change return and normal identity-preserving
rearchive rather than an unsupported supersession state. The actual
history-preserving `git mv` and evidence review complete task 15.5; the scratch
copy completes no other real task. The task record therefore contains **101
unique identifiers, 82 complete / 19 pending**. Provider measurements and
enablement, Rust/local gates, pre-archive readiness, the real archive, host
coverage and controller shipping evidence all remain pending and unwaived.

Current active strict validation passes under the dated identifier, OpenSpec
status reports all planning artifacts complete, the existing archived change
validates strictly, and `git diff --check` is clean. Cargo is unavailable in
this boxed workspace, so no Rust gate is claimed.

Only the existing change artifacts move or change. Production code, living
specs, frozen contracts/fixtures/policy/reference, accepted decisions, provider
settings and sibling worktrees remain untouched. No workflow runner, real
archive, push, merge, publication or issue action was invoked.

## Current specify return — J release-binary gate, 2026-09-10

This returned visit adopts the dated change and accepts the clarifier's sole
finding. Proposal decision J now classifies
`cargo build --release --locked -p brokkr-cli` as an explicit
repository-local pre-archive validation obligation. The commissioned command is
listed with the other local gates; the later design and task phases must carry
it into their own gate account before implementation can become archive-ready.
Remote CI and exact-head controller results retain their separate non-checkbox
handoff status.

PM4's existing per-commission scenario now distinguishes every attainable local
command, including a commissioned release-binary build, from later exact-head
evidence. No requirement or scenario is added or removed, no task is ticked,
and answers A–I and repairs F1–F12 retain their settled meaning. The real
archive, implementation/provider evidence, Rust gates, host coverage and
shipping actions remain pending.

Strict active validation and OpenSpec status pass under the dated identifier;
the parser and source headings both remain at **20 requirements / 125
scenarios**. The existing archived change validates strictly and
`git diff --check` is clean. Cargo remains unavailable in this box, so this
specification return claims no Rust or release-binary result.

## Current successor specify return — upstream DSH route, 2026-09-10

This visit adopts HEAD `80f7a32` and the dated change
`2026-09-09-226-session-resumption`. Decision K corrects the earlier
installed-entry-only DSH conclusion by selecting official core 0.1.5-rc.1 at
`183f08e9c6dde7e36cd2318eaee70b0da08fb35e` with
`dsh-plugin-cli-session` 0.2.0 at
`0f487e74c81ed102c6899440d9f5d65e8e9eabda` as the exact isolated
headless route to qualify. It records why the official SDK is not selected,
keeps Brokkr production Rust-only, and forbids global-pin/profile changes,
package patches, UUID interception, TUI substitution and hands admission.

AS1 and its existing scenarios now require dependency/API compatibility,
provider-confirmed exact-root rejoin, current headless model/effort and
restriction precedence, and sequence-bounded output/accounting before DSH can
be enabled. The supplied Claude 2.1.266 root, grant, tool, MCP and accounting
evidence is recorded as partial: it advances qualification without completing
the full boxed restriction or exceptional-accounting proof. No task is ticked
by this specification visit, and Codex and LaneTally obligations remain intact.

Strict active validation passes and the parser and headings retain **20
requirements / 125 scenarios** across all five deltas. OpenSpec status reports
all planning artifacts present; strict archived validation and `git diff
--check` pass. Frozen contracts, fixtures, policy, reference and living specs
are unchanged. Cargo and Rust remain unavailable in the box, so no Rust gate is
claimed. No workflow runner, archive, provider probe, global provider change,
push, publication or issue action was invoked.

## Current successor specify return — AS1–AS3 archive operation, 2026-09-10

This visit adopts the clarifier's finding. Decision L corrects the earliest
owning artifacts: AS1, AS2 and AS3 are complete MODIFIED requirements because
their living counterparts already exist and the upstream DSH/partial Claude
evidence changed their semantics. AS4 and AS5 remain unchanged historical
ADDED requirements. PM4's existing normal-rearchive scenario now encodes the
same answer without adding or removing a scenario. The current archive
postcondition is therefore four replacements—AS1–AS3 and PM4—with the other
sixteen requirements treated as no-ops and all provenance preserved once.

This correction does not reopen the selected DSH route, provider minima,
restriction boundaries, ownership, accounting, progress, release-build or
dated-identity decisions. The deterministic record that called the active
delta clear while warning that archive would refuse AS1 is rejected as clean
archive evidence; strict active validation alone cannot prove foldability.

The corrected five deltas retain **20 requirements / 125 scenarios**, now
classified as **15 ADDED and five MODIFIED requirements**. In a fresh isolated
copy, active strict validation, the normal dated archive and strict archived
validation all exited zero with no refusal warning. OpenSpec reported exactly
three modified safety requirements and one modified progress requirement; only
those two living capability files changed. AS4–AS5 remained byte-identical,
every other living capability was unchanged, and all five provenance sections
remained byte-identical, singular and bidirectional. The real archive, tasks,
provider proof, Rust gates and delivery remain pending; this simulation proves
only the corrected fold mechanism.

## Current successor implement return — DSH pair re-pin, 2026-09-10

This visit answers the returned implement finding that the exact
0.1.5-rc.1/plugin-0.2.0 pair is incompatible. That incompatibility is real and
measured, not the old global DSH limitation: the plugin reads
`agent.session.events`, which core 0.1.5-rc.1 removed. The plugin's own
development matrix pins **0.1.0-rc.6**, whose host modules resolve to
0.1.0-rc.8 and retain the accessor. A task-owned isolated installation under
`.forge/dsh-qualify-010rc6/` composes the `headless` profile through the
plugin's `cordis.patch.yml` and completes a live cold `--new` plus warm
`--session` pair: both exit zero with `stream-json` result envelopes, the same
`session-a95eacaf…` root, nonce recall on the warm turn and per-message usage,
with the global `dsh 0.1.2-rc.1` pin, profiles and credentials byte-unchanged.
Evidence is `.forge/tasks/dsh-pair-qualification-010rc6.json`; the earlier
0.1.5-rc.1 failure remains bounded history in
`.forge/tasks/dsh-pair-incompatibility.json`.

Decision K therefore selects official core **0.1.0-rc.6** with
`dsh-plugin-cli-session` 0.2.0 at
`0f487e74c81ed102c6899440d9f5d65e8e9eabda` as the exact pair to implement and
qualify, and the AS1 requirement, the `headless-work` assessment, the provider
guide and decision 0056 record that pin. The DSH shape remains `unmeasured`
and disabled: `--session` still needs consumer-side exact-root confirmation,
restriction precedence after restore and a multi-message/retry current-sequence
accounting boundary, and the Rust route (tasks 8.8/8.10/9.6) is unimplemented
until those observations and the adapter path agree. No task is ticked from the
qualification alone, and Codex, Claude and LaneTally obligations remain
unchanged.

## Current successor specify return — DSH forward pin, 2026-09-10

This visit belongs to run `current-successor-operator-rulin-b83add73`. It
adopts every commit at HEAD `a86eca1` and the dated change
`2026-09-09-226-session-resumption`, and applies the operator's 2026-09-10
ruling as answer M. The DSH route pins the latest core, 0.1.5-rc.1 at
`183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, and `a86eca1`'s re-pin to
0.1.0-rc.6 is reversed. What Changes, Evidence, Impact, K and Delivery
obligations now name that core with a repository-owned, digest-pinned
adaptation of `dsh-plugin-cli-session` 0.2.0. AS1 states the forward pin, the
adaptation's single-delta and identity rules, append-only pin history, and
missing-surface handling. Four new AS1 scenarios encode the removed accessor,
the no-replacement park, run-time version resolution and the multi-message,
tool and retry accounting boundary of task 10.7. AS2 extends its
composite-identity clause to the adaptation's bytes. The dated `DSH pair
re-pin` record above remains the history of that superseded visit.

Discovery was read-only, in the task-owned `.forge/dsh-qualify/core`
installation and the plugin checkout beside it. It found the declared
replacement accessor recorded under Evidence and answer M. Nothing was
executed, installed or fetched; the live registry was unreachable, so the
cached registry document supplies the dist-tag reading. No model call, live
probe, global provider change, workflow runner or real archive ran, and the
global DSH pin, profiles and credentials were not touched.

These downstream artifacts owe the reversal and stay outside this office's
commit:

- design D6 (the adaptation's location, build, provenance and digest inputs),
  a dated design reconciliation section and D10 row 5;
- proposed decision 0056 ruling 5 and Consequences, still `proposed`;
- `adapters/dsh.json` identity, interface, reason and limitations, with a
  dated reversal entry that appends to the 0.1.0-rc.6 facts rather than
  rewriting them, and any packaged copy the loader tests pin;
- `docs/guides/provider-adapters.md`;
- tasks 1.1, 6.4, 10.3, 10.7, 11.3, 11.5 and 13.1, each reopened or re-ticked
  truthfully;
- the Rust route in 8.8, 8.10 and 9.6.

Task 11.3 is enabled only on measured
`.forge/tasks/dsh-pair-qualification-015rc1.json` evidence. Codex 10.5, Claude
10.6 and LaneTally 10.8 keep their recorded state, as do 11.1, 11.2 and 11.4.

Strict active validation passes. The parser and source headings agree at
**20 requirements / 129 scenarios**, still **15 ADDED and five MODIFIED**.
In a scratch copy with tasks ticked only there, the normal dated archive
modified exactly AS1–AS3 and PM4 and changed only those two living files. All
five provenance sections stayed byte-identical, and strict validation of the
twelve living specs passed. `git diff --check` is clean. Cargo is not on this
box's PATH, so this return claims no Rust, bundle or release-binary result.
