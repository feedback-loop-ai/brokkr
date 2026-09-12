# 0056 — A work site rejoins its own session, whatever its harness, and a launch says which it was

Status: proposed
Date: 2026-09-09

## Context

Decision 0030 ruled that a retry or a re-entry of a seat rejoins its own
thread, with the sandbox class re-imposed because `codex exec resume`
drops it. It was written from a codex measurement and only the codex arm
was ever wired. Measured on the canonical journal on 2026-09-06:

| Harness | cold | resumed |
|---|---|---|
| codex | 47 | 19 |
| claude | reports no launch field at all | — |
| dsh | reports no launch field at all | — |

Both other CLIs advertise a resume interface. `claude --resume
<session-id>` is in the installed 2.1.266 help, beside `-c/--continue`,
`--session-id <uuid>` and `--fork-session`; the dsh launcher's own help
shows `--resume <session>` in a TUI-profile example. The engine's three
`run_driver` call sites pass `session_ref: None`, and
`AdapterKind::Codex => vec!["resume"]` is the only arm that knows the
verb.

What a cold re-spawn costs was measured on run
`decision-0046-enactment-slice-i--165601b4`. Its implement phase ran 340
turns to a peak of 685k input tokens and died mid-session on the
account's model limit (issue #219), leaving 25 files changed, +2138/-546,
uncommitted. The retry spawned a fresh session at turn 1 with ~10k input.
The edits survived; the reasoning did not. The new smith walked into 25
modified files it had no memory of writing, with `tasks.md` still reading
0 of 124 ticked, and had to re-derive from the diff what its predecessor
had done. That is the same discovery paid for twice, and it is a
correctness hazard before it is a cost one: a smith that misreads a
partial edit can undo it.

Two other rulings bound this one. Decision 0042 ruling 2 says the next
judge is **fresh and blind** and reads prior answers from the artifacts,
which is later and more specific than 0030's work-seat example — so
generalising 0030 to every model site does not generalise it to gates.
And decision 0053 shipped the pre-session refusal path, which holds the
launch row until a checkpoint proves work began; a launch fact that
overran that hold would put a refused attempt on decision 0016's
mid-session side.

Alternatives weighed, each rejected on evidence rather than taste:

- **Fix only the adapter switch — advertise `resume` from every arm and
  leave the engine alone.** Rejected: the three `None` call sites are
  panel members and sequence model steps, so the switch would leave every
  composite work site cold. It would also carry the single-seat gate's
  existing offer into councils, extending an inconsistency with 0042
  ruling 2 rather than closing it.
- **Extend the existing single-seat offer to every site, gates
  included.** Rejected by 0042 ruling 2, above. Current code is not
  authority to contradict a later ruling; the single gate's offer is
  removed rather than copied.
- **Keep the flat `<step>:<member>` display tag as the resume key.**
  Rejected: `Site = Option<String>` and `tag_member` flatten ancestry, so
  step `a:b`/member `c` and step `a`/member `b:c` are the same string.
  Two different sites sharing one key is a session handed to the wrong
  model. Ancestry is taken from the compiled body walk instead, and the
  flattened lookup gets a compile-time uniqueness check because
  `select_candidates`, `argv_for` and `bundle.hands` already key on it.
- **Compare the whole provenance array, as `resume_offer` does today.**
  Rejected: a sibling member's chain fallback moves that array and would
  deny an unrelated member's own session. Ownership is a per-site digest.
- **Search behind an incompatible newest owner for an older matching
  one.** Rejected: it resurrects a session the site has since moved past,
  and the search is unbounded in exactly the direction that fails open.
- **Treat an assigned session id as evidence that the session exists.**
  Rejected: the provider might never have created one. 0030 ruling 4's
  reasoning applies unchanged to an id the engine chose.
- **Trust a flag, an exit status of zero, surviving edits or a replayed
  transcript as proof that a rejoin happened.** Rejected: none of them
  distinguishes a rejoin from a fresh session that ran the same prompt.
  Only provider evidence naming the exact offered root does.
- **Enable a shape from its CLI help.** Rejected: 0030's own measurement
  found a thread opened `-s read-only` writing files on a bare resume.
  Equal flag lists do not prove enforcement.
- **Keep a shape enabled until something contradicts it.** Rejected for
  the same reason, one version later: the original measurement was
  0.148.0 and the installed CLI is 0.153.4. Version drift disables a
  shape until it is remeasured.
- **Add the new facts to `seat-record.v4`.** Rejected by house rule: a
  contract change lands as a new numbered file beside the old one. v5 is
  additive, and its two new conditions are scoped so no valid v4 or
  unstamped 0.10.0 row is newly refused.
- **Put the progress-marker rule in each dialect's task template.**
  Rejected by 0042 ruling 6: the SDD charter already owns implement-time
  task completion, and the templates already own the paths and formats.
  One charter edit, inherited by both dialects.
- **Ship cold-only reporting and call #226 closed.** Rejected: the issue
  is that claude and dsh re-spawn cold. A `launch` field that only ever
  says `cold` reports the defect rather than fixing it.

## Rulings

1. **A work site's continuity is its own; every gate is fresh.** The
   engine offers the latest durably recorded session of *this* run,
   *this* site and *this* adapter instance to each work-class model
   invocation on retry, phase re-entry and operator retry after a park,
   at all four executing topologies: the single seat, each panel member,
   each sequence model step and each member of a sequence panel. A site
   is the run, the outer seat, the selected case, the body kind and the
   full member/step path, taken from the compiled walk and never from a
   display tag split on `:`. A label reused under a different parent is a
   different site.

   Every gate-class invocation starts without an offer — the single gate
   that the shipped code offers one today included — because decision
   0042 ruling 2 requires the next judge to be fresh and blind. A single
   seat or panel uses its selected compiled class, a sequence step its
   own, a panel member its enclosing panel's; an office name and the
   presence of a later gate step decide nothing. Deterministic exec and
   dialect validation steps stay outside the capability entirely: no
   offer, no negotiation, no model launch.

   **Enforcement binding:** `SiteKey` and the per-invocation site context
   in `crates/brokkr-runtime/src/engine.rs`, offered at all four
   topologies; the compile-time uniqueness check in
   `crates/brokkr-runtime/src/bundle.rs`;
   `crates/brokkr-runtime/src/engine/resume_tests.rs` asserts the actual
   wire offers and their absence for every work and gate topology.

2. **The same run, site, instance and local origin, and the newest owner
   is the only one asked.** An offer requires the same selected
   agent, provider, model, effort and chain index, the same normalized
   driver identity and pinned command template, the same adapter
   declaration digest, engine identity and pinned bundle, the same class,
   boundary and hands declaration, and the local installation's existing
   origin check. The engine takes the *latest* session-bearing invocation
   of that site and then checks its ownership: an incompatible or
   ambiguous newest owner denies the offer, and the query never searches
   behind it for an older matching owner. A failure that recorded no
   session supplies no replacement. Ephemeral result paths, expanded MCP
   filenames and capability tokens are excluded from that identity —
   their declaration is pinned while each invocation reconstructs the
   value — and no resolved credential, provider-home content or private
   argv enters it.

   **Enforcement binding:** `InstanceKey` and the pure eligibility query
   in `engine.rs`, over this run's durable evidence, `Store::started_here`
   and the existing manifest fence; the identity-axis and
   newest-owner tests in `resume_tests.rs`.

3. **A handle is a provider-confirmed root, and intent is not
   existence.** Two identity origins are admitted: an identifier the
   provider generated when the root opened, or a fresh identifier the
   owning engine or adapter assigned through a *measured* provider
   creation interface. Either way, eligibility requires provider evidence
   that the root actually opened under the originating site's instance,
   durably recorded under ruling 7's vocabulary, and for an assigned id
   the confirmed root must match the assigned value exactly. An id in a
   start payload, an argv or a configuration echo is creation intent
   only; a kill before durable confirmation leaves no offer, and the next
   authorized cold creation chooses a fresh value. Assignment is held in
   memory for the invocation: this decision persists no pre-spawn
   creation-intent field. Once a root is latched, a different root is a
   mismatch and not a last-write-wins. A delegated child session never
   replaces the root, a transcript storage directory is not a handle
   without measured equivalence, and an identifier that cannot fit its
   bounded field losslessly is not used rather than truncated.

   **Enforcement binding:** `ConfirmedSession` and the root latch in
   `crates/brokkr-protocol/src/adapters.rs`; the v5 store fence; the
   assignment, mismatch and kill-window tests in
   `crates/brokkr-protocol/src/adapters/tests.rs` and `resume_tests.rs`.

4. **The offer is negotiated, correlated and used once.** The handle
   travels on the protocol's existing `resume` message, before the start
   it belongs to, and never in the prompt, the rendered context or a
   policy input. All four model adapters advertise `resume` as *offer
   receipt*; exec advertises none, and a third-party driver that does not
   advertise it is never handed one. Receipt is not a claim that any
   offered session can be resumed. The pending offer carries its effect
   and attempt: a matching start consumes it exactly once, and a
   duplicate offer, a malformed correlation, a wrong effect or attempt, a
   resume before negotiation or a malformed envelope poisons that
   exchange — no provider is launched and the exchange is never repaired
   into a cold execution. Cancellation, shutdown, EOF and a completed
   invocation discard pending state. The current attempt's prompt, result
   destination and scoped hands configuration accompany every rejoin, so
   an old conversation's remembered result path or grant cannot select
   this attempt's output or permissions.

   **Enforcement binding:** `PendingOffer` in `adapters.rs`'s `serve_io`
   and the unchanged `run_attempt_resuming` ordering in
   `crates/brokkr-protocol/src/process.rs`;
   `crates/brokkr-protocol/src/process/tests.rs`, `adapters/tests.rs` and
   `crates/brokkr-cli/tests/driver_conformance.rs`.

5. **A shape is enabled by measurement of the installed harness, and the
   measurement expires with its version.** Each adapter declares, per
   named execution shape, a status of `unmeasured`, `unsupported` or
   `supported`, an assessed identity that is either a measured version or
   an explicitly unknown one with a bounded reason, the applicable
   classes, boundaries and hands mode, its evidence references and its
   measured limitations. Only a measured identity can support
   enablement, and a `supported` entry names its interface, restriction,
   exact-root and current-accounting evidence — root evidence stating
   whether that root persists, and a wrapper qualified on its own wrapper
   rather than on what it wraps. An absent assessment or an unnamed shape
   loads as `unmeasured` and enables nothing; present malformed data is a
   loader refusal, never a silent downgrade to honest ignorance.

   The selected executable's version is probed once per invocation
   through its measured version interface and compared with the pinned
   assessment and with the originating root's recorded version; a
   missing, changed or unreadable identity disables resume with
   `unverified-harness`. Historical acceptance of an earlier version is
   history and regression scope, not a grant for a later one: 0030's
   0.148.0 codex measurement does not enable installed 0.153.4.

   DSH's session integration is selected as one exact route: the latest
   official core **0.1.5-rc.1** (`@deepseek-ai/dsh@0.1.5-rc.1` at
   `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
   resolves in its place) together with the repository-owned six-file
   adaptation of community plugin `dsh-plugin-cli-session` **0.2.0** at
   `0f487e74c81ed102c6899440d9f5d65e8e9eabda`, committed under
   `extensions/dsh/plugin-cli-session/` with its sibling
   `extensions/dsh/PROVENANCE.md`, installed and exercised only in
   worktree- or task-owned storage through the documented extension and
   `agents.resume` APIs. Decision 0009's extension boundary admits those six
   language-neutral files: Rust-only describes this repository's production
   crates, not the extension boundary, and Brokkr neither builds, loads nor
   executes them. An isolated live cold+warm qualification (2026-09-12)
   establishes composition, a result envelope, one continued root, nonce
   continuity and per-message usage with the global pin unchanged; the route
   is still **unmeasured** for admission because exact-root and
   independent-root confirmation, restriction precedence, a multi-message
   current-sequence accounting boundary and the declared composite digest are
   unproven, so the declaration stays disabled and sets no `wrapper_digest`
   until the qualification completes. The removed `agent.session.events`
   accessor was a measured incompatibility of the unadapted plugin on
   0.1.5-rc.1, fixed by the adaptation's one-expression substitution of
   `snapshotEvents(firstSeq)`; it is never a global DSH or extension
   limitation. The request-derived `session_id` echo and the post-`firstSeq`
   last-wins usage selection are interface evidence only: they neither
   confirm the root nor attribute a multi-call total. The installed
   0.1.2-rc.1 headless runner remains bounded history — its one-shot entry
   mints a fresh root and cannot forward the launcher's TUI example. This
   session-selection work stays separate from the hands/tools plugin the
   adapter defers, preserves the admitted headless profile, and admits no
   second runner. No different core or plugin revision inherits this
   evidence, and a measured incompatibility of this exact route is reported
   as the exact unmet requirement rather than a narrowed minimum or a
   permitted TUI, SDK or package-patch substitution.

   **Enforcement binding:** the typed `resume` assessment and its loader
   in `crates/brokkr-runtime/src/agents.rs`, pinned by the existing
   adapter content digest; the per-invocation version check in
   `adapters.rs`; `crates/brokkr-runtime/src/agents/tests.rs`;
   `adapters/{codex,claude,dsh,lanetally}.json` and their packaged
   copies; `docs/guides/provider-adapters.md`.

6. **The current restrictions are re-imposed, never inherited.** Every
   rejoin carries this invocation's own class, model and effort, its
   generated hands fragment, its result door and its scoped resources,
   composed in the engine with their provenance intact and compared
   against the actually expanded fragment in the adapter. The hands
   fragment is decision 0043's single tool; a rejoin re-imposes it and
   never inherits the previous grant. User
   passthrough that merely *resembles* a generated MCP setting is not
   authorized by the resemblance. A measured allow-list with exact arity,
   duplicate and precedence checks decides what may travel: competing
   selectors, forks, background, cloud or worktree launch, extra
   positional handles and unknown restriction or profile overrides are
   not forwarded, and the same selector protection applies to the cold
   and gate paths so an ambient `--continue` is never forwarded after an
   offer is declined. A resume-only incompatibility may select a known
   safe cold spelling; a cold-inadmissible setting refuses before
   provider work rather than dropping a restriction. This is 0030 ruling
   2 generalised: a class that cannot be re-expressed is a cold spawn
   with the reason journaled, never a quiet escalation.

   **Enforcement binding:** the restriction plan composed in `engine.rs`
   and compared in each planner in `adapters.rs`; the planner tests in
   `adapters/tests.rs` built from the captured grammars.

7. **One confirmed launch per executing model site, in v5's vocabulary,
   dispatched from 0.10.0.** `launch: resumed` is published only after
   provider evidence confirms the exact offered root before first work;
   `launch: cold` only when the adapter actually took the fresh-session
   path. A resume flag, a preassigned id, a known old handle, surviving
   edits, replayed transcript rows and an exit status of zero prove
   nothing on their own, and a confirmed fresh creation stays cold even
   when the engine chose its id. A bounded refusal reason is emitted
   exactly when an offer was declined and the launch is cold; no refusal
   is invented where no offer was made.

   Those facts ride `contracts/seat-record.v5.schema.json`, additive on
   v4 — decision 0034's seat record is a contract, and this is a new
   numbered version beside the frozen ones: `site_ref`, `instance_ref` and the closed `root_session` object,
   with five refusal tokens beside v4's five. The store selects v5 for
   the 0.10 engine line and later, v4 for the 0.9 line, v3 for the 0.8
   line and v1 for older or unparseable engines, identically at append,
   export, import verification and offline verification — the amendment
   this decision makes to the `boundary-record` capability's standing
   dispatch requirement. Published and embedded v1–v4 bytes do not move,
   boundary stamping is unchanged, and no added constraint refuses a
   valid historical row: v5's two new conditions are scoped on the
   `site_ref` stamp, the one within-row fact only an engine enacting this
   decision writes, while the unconditional form binds this change's
   producers in the launch lifecycle. Decision 0053's first-work hold
   stands: root and launch candidates are held until `run_seat`'s
   existing boundary, a classified pre-session refusal keeps no
   `Accepted` and no checkpoints, and a kill inside that window loses
   those rows and therefore offers nothing.

   **Enforcement binding:** `contracts/seat-record.v5.schema.json` and
   its embedded copy; `SeatRecordVersion::V5` and `of_engine` in
   `crates/brokkr-store/src/seat_record.rs`, used at all four fences
   through `crates/brokkr-store/src/lib.rs`; the launch lifecycle and
   held-row order in `adapters.rs`;
   `crates/brokkr-runtime/tests/frozen_contracts.rs`.

8. **One proven pre-work replacement, inside the bounds that already
   exist.** Decision 0006's bounded attempt is the frame: one replacement,
   one deadline. A local decline permits one independently safe cold
   launch. A provider-rejected resume permits one cold replacement, reported cold
   with `harness-refused`, and only on measured machine session-rejection
   evidence establishing that no session opened, no turn or tool action
   ran and no result was delivered. A generic nonzero exit, stderr prose,
   missing telemetry, a dropped connection or elapsed time establishes
   nothing, and the whole invocation outcome is read first: an
   error-shaped notice followed by work or clean delivery is not a
   rejection, and its delivered work is retained. The rejected child's
   held launch, root, locator, model/effort and accounting candidates are
   cleared before the replacement, and there is no recursion. One outer
   driver process, one process-tree watchdog and the original deadline
   span both launches and the version check: the replacement gets only
   the remaining budget, no new attempt id, no new chain slot and no new
   timer. Cancellation or deadline termination prevents a later
   replacement from starting, and a watchdog kill keeps `deadline_killed`
   rather than becoming a failure to start because acceptance was held
   (decision 0053 ruling 5).

   **Enforcement binding:** the resume outcome states and the single
   replacement in `adapters.rs`; the shim sequences in `adapters/tests.rs`
   and the deadline/cancellation tests in `process/tests.rs`.

9. **Accounting is current-only, and legacy compatibility is narrow.**
   Each enabled planner establishes a measured restored-history /
   current-work boundary — the provider's own turn or event cursor where
   it has one — before its usage and tool folds run, and a shape whose
   boundary cannot be established is not enabled. Only this invocation's
   new turns, tools, targets and reported usage are counted; replayed
   tools and targets are filtered too. Rotation, truncation, a missing
   cursor or uncertain attribution omits the affected measurement, or
   refuses the resume when no boundary can be established at all: a
   lifetime total without an established baseline stays absent rather
   than being guessed, subtracted or replaced by zero. The inclusive
   input and cache-read subsets, cache writes, completion deduplication,
   served model and effort conventions and LaneTally's capture identity
   are unchanged, and a rejected resume's unconfirmed usage stays out of
   the cold replacement's accounting. Historical transcript text is never
   read to reconstruct a prompt or a line of reasoning; transcript kinds,
   locators, homes, size caps, retention and path checks are preserved,
   and issue #222's readers are untouched. An unambiguous local
   single-work-site codex row may still offer its thread through the
   established kind and flat-id mapping, subject to every instance,
   manifest and origin check and to ruling 5's version qualification;
   legacy composites and claude or dsh locator-only evidence start cold
   once to establish an explicit root. A new row without a confirmed root
   cannot fall back to legacy fields to evade confirmation.

   **Enforcement binding:** the per-planner current-work boundary and the
   folds in `adapters.rs`; the legacy lookup in `engine.rs`; the replay,
   unknown-baseline and legacy tests in `adapters/tests.rs` and
   `resume_tests.rs`.

10. **Progress is persisted before the next group, not saved for the
    commit.** Resume is the fix when the harness allows it; when it does
    not, a seat that dies mid-phase must still leave a record of where
    the work stood. The SDD smith's charter — one edit, in
    `agents/charters/implementer-sdd.md`, inherited by both dialects —
    requires a group to be recorded in progress before its first edit
    with its focused acceptance checks named, and each task's tick and
    concise evidence to be persisted once its implementation and those
    checks pass, before the next group. Partial implementation and failed
    or pending checks stay unchecked with a next action, and group
    completion, workspace validation, commit and external delivery remain
    four separate facts. On retry or re-entry, warm or cold, the smith
    reads the change's current artifacts and the current worktree before
    continuing, reconciles ticks against surviving edits and cited
    verification, preserves work that still satisfies its task, returns
    an invalidated task to pending with its reason, and never erases
    partial uncommitted edits merely because it does not remember
    authoring them. Current evidence outranks memory, and no seat reads
    another session's private transcript. Decision 0041 binds the same
    boundary from the office side: judges never fix the work they judge,
    so no judge gains permission to edit the task artifact. The charter names the task
    artifact generically; the dialects keep their own paths, formats and
    phase maps, and judges gain no permission to edit them.

    Progress and delivery are two separate facts, and the artifact
    records the first while the change is still active. Every tracked
    repository-local obligation is finished and its tick persisted before
    the normal archive operation, which is the final tracked artifact
    effect; the delivery commit follows it as an untracked phase action.
    One pre-archive provenance assertion — the check that every capability
    names the archived change that wrote it — cannot pass while the change
    is still active, so the readiness gate defers exactly that named
    assertion with unique-match and single-filter evidence, and the full
    unfiltered workspace suite runs read-only on the archived bytes after
    the archive effect and before the delivery commit; readiness records
    that archived validation as pending until its result exists, never as a
    completed full-suite gate.
    Exact-head evidence whose subject is the immutable delivery commit or
    a later integrated head — the host exact-coverage gate, remote CI on
    the final head, integration, publication, PR, merge and issue closure
    — is controller evidence recorded outside the checked task state and
    outside this artifact, because checking it would mutate the head it
    judges. Its absence gates the corresponding delivery or closure
    claim; it never leaves an archived task unticked or licenses a
    post-commit edit of this artifact.

    **Enforcement binding:** `agents/charters/implementer-sdd.md`; the
    charter identity in `crates/brokkr-runtime/tests/library_data.rs` and
    the rendered-prompt expectations in
    `crates/brokkr-runtime/tests/sdd_shape.rs` and
    `crates/brokkr-runtime/tests/roster.rs`; the deterministic recovery
    exercise and the judge-read-only test beside them; and the
    finalization order in `design.md` D9/D11, whose post-task archive,
    provenance and commit steps are deliberately not checkboxes.

## Consequences

- The engine's three `session_ref: None` call sites become four
  topologies that each derive their own offer, and the single seat's
  offer stops depending on a gate check it never made.
- A retry of a work seat under a measured, enabled shape rejoins its own
  reasoning instead of re-deriving it from the diff. Under an unmeasured
  or unsupported shape it spawns cold **and says so**, with the reason in
  the record — which is the part of #226 that no measurement can be
  waiting on.
- Every model adapter reports `launch`, so the operator can see whether a
  retry was warm. A run whose journal shows only `cold` is now evidence
  about the adapters rather than silence.
- Seat records written by the 0.10 line and later are judged under v5.
  Nothing already written is rewritten, back-filled or newly refused, and
  older binaries cannot verify v5's fields — the ordinary price of a new
  contract version, paid the way every version before it paid it.
- **Two limits this decision refuses to hide.** The local origin
  fingerprint (`Store::started_here`) tells the engine that a run was
  created on this machine and account; it cannot authenticate a provider
  account or detect a credential home swapped underneath it. The engine
  fails closed on a detectable owner rejection and does not inspect
  credentials to invent a stronger guarantee. And ruling 10 is a charter
  instruction, not an engine guarantee: the tests prove that both
  dialects' smiths are told, and that an interrupted exercise recovers
  from what is on disk. Whether a live model obeys is judgment's to
  check, and the guides say so.
- **The selected DSH route and its bound.** Ruling 5 selects the latest
  official core 0.1.5-rc.1 (`@deepseek-ai/dsh@0.1.5-rc.1` at
  `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
  resolves in its place) with the repository-owned six-file adaptation of
  `dsh-plugin-cli-session` 0.2.0 at
  `0f487e74c81ed102c6899440d9f5d65e8e9eabda` under
  `extensions/dsh/plugin-cli-session/`, as the supported extension route
  from the headless caller to an owned root, and keeps its declaration
  disabled until that exact route's exact-root rejoin, restriction
  precedence, current-sequence accounting and declared composite digest are
  measured. An isolated live cold+warm qualification (2026-09-12) proved
  composition, a result envelope, one continued root, nonce continuity and
  per-message usage, but enables nothing. The installed 0.1.2-rc.1 one-shot
  result is bounded history for that entry only and is never restated as a
  global DSH limitation; the unadapted plugin's loss of
  `agent.session.events` on 0.1.5-rc.1 was a measured pair incompatibility,
  not a global one. Whether a harness other than these four is ever
  admitted, and on what evidence, stays the question ruling 5 already
  answers for any adapter.
- **2026-09-10 — the operator reversed the 0.1.0-rc.6 pin.** The ruling
  that selected the plugin's own supported core generation 0.1.0-rc.6 is
  reversed: the forward-pinned latest core with the repository-owned
  adaptation at `0f487e74` is the selected route, and the superseded
  `.forge/tasks/dsh-pair-qualification-010rc6.json` record and its
  0.1.0-rc.6 measurement are dated history only. This note is appended;
  the earlier ruling and consequence text it supersedes is preserved as
  history rather than rewritten.
