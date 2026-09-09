# Tasks: Same-instance session resumption and durable progress (#226)

Groups are the design's landing order (Migration Plan 1–5): the proposed
ruling before any production semantic edit, the record version before
anything emits its fields, the site identity before the query that keys
on it, the query before the wire that carries it, the wire before the
adapters that receive it, the shared launch lifecycle before the four
provider planners, accounting after the planners that feed it, the
charter after the engine work it describes, then the prose, then the
re-pins, then the gates, then the fold.

Execution order is that order with one declared exception, because two
groups consume what group 10 produces. Group 10 is split at its own
seam. **10.1–10.4 are interface investigation**: they read installed
help, installed source and packages, and the dated captures under
`.forge/`, and they run **before** group 8's provider-specific
construction (8.5–8.9) and group 9's accounting boundaries (9.1–9.3),
whose argv, routes and cursors are their output — 8.8's DSH route is
10.3's, and 9.1's current-work cursor is measured, not assumed.
**10.5–10.8 are live enforcement proof**: they need dated controller
host evidence that does not exist in this worktree, they run after
groups 8 and 9, and they gate group 11 and nothing else. Every task that
depends on one of the eight names it, and the `## Progress` foot of this
file records the dependency as it actually binds rather than by group
number.

Every task names the requirement it serves as `<capability> /
<Requirement>`. The closing gates of group 15 serve every requirement of
the change and say so, because a gate is not a requirement of its own.
Capability short names below are `site` (`site-session-resumption`),
`safety` (`adapter-resume-safety`), `evidence` (`adapter-launch-evidence`)
and `progress` (`sdd-progress-markers`).

Conventions binding on every task, restated once rather than per task:

- No frozen byte moves. `contracts/seat-record.v1`–`v4`,
  `contracts/realms.v1`–`v4`, `run-manifest.v1`–`v9`,
  `effect-provenance.v1`, `driver-protocol.v1`, `policy/phase-machine.json`,
  `policy/schemas/`, `reference/` and `fixtures/evaluator/corpus.ndjson`
  are read-only; a contract change is a new numbered file beside the old
  one (house rules; decision 0034).
- Tests are written with the code they prove, in the crate that holds it.
  Extend the existing suites named in each task rather than opening new
  files where one already covers the seam (`safety / AS1`,
  `progress / PM4`).
- Deterministic provider shims prove argv, ordering, correlation and
  record validation. They are labelled shim evidence and never cited as
  live provider enforcement (`evidence / LE5`).
- No invented CLI syntax, provider event name, telemetry or config key.
  Provider-specific construction spends the dated controller captures
  under `.forge/` and nothing else (`safety / AS1`).
- No push, no merge, no new run, no issue closure, no global provider
  settings change. Commits are unsigned and in the repository's message
  style.

Progress under `progress / PM1` is kept in the `## Progress` section at
the foot of this file: mark the group in progress before its first edit,
tick each task when its implementation and its named focused check both
pass, and leave a note when implementation is done but verification is
pending. The tick is written to the worktree as the group finishes, not
saved for the phase commit.

## 1. The proposed ruling (design D10)

- [ ] 1.1 Write `docs/decisions/0056-same-instance-session-resumption.md`
      with `Status: proposed`, in the register of the neighbouring
      decisions: context from #226 and the measured cold/resumed table,
      the alternatives design D1–D9 rejected, and ten numbered rulings
      whose content is D10's table — work-site continuity with every gate
      fresh (1), same run/site/instance and local origin with no
      older-owner resurrection (2), provider-confirmed root and the two
      admitted identity origins (3), the negotiated one-use offer (4),
      measured provider shapes with current-version qualification and DSH
      session integration held separate from hands (5), re-imposed
      current restrictions (6), one confirmed launch in the v5 vocabulary
      with the first-work hold (7), one proven pre-work replacement inside
      the existing bounds (8), current-only accounting and narrow legacy
      compatibility (9), and progress persisted before the next group
      (10). Cite decisions 0006, 0016, 0030, 0034, 0041, 0042, 0043,
      0046 and shipped 0053 where each ruling stands on them, and quote
      no accepted decision into a different meaning — safety / AS1.
- [ ] 1.2 Append the registry row to `docs/decisions/README.md` in number
      order with status `proposed`, and run
      `cargo test -p brokkr-cli --test decisions_index` so the derived
      index test proves the row is present exactly once — safety / AS1.
- [ ] 1.3 State in 0056 the two limits the design refuses to hide: the
      local fingerprint cannot authenticate a provider account or detect
      a swapped credential home (`site / SR5`), and a charter instruction
      is not an engine guarantee that a model obeys it
      (`progress / PM3`) — site / SR5.

## 2. Seat record v5 (design D4)

- [ ] 2.1 Write `contracts/seat-record.v5.schema.json` as v4 plus the
      optional checkpoint properties of D4's table — `site_ref` and
      `instance_ref` (64 lowercase hex), and the closed `root_session`
      object with `kind` (`codex-thread | claude-session | dsh-session`),
      `id` (1–80 ASCII, alphanumeric first then alphanumeric, underscore
      or hyphen), `harness_version` (1–80 ASCII alphanumeric, dot,
      underscore, plus or hyphen, alphanumeric first), optional
      `wrapper_digest` (64 lowercase hex) and `persistent` — with title
      and schema constant following the v4 file's spelling. Keep
      `launch` exactly `cold | resumed`; extend the refusal enum with
      `unsupported-resume`, `unverified-harness`, `restrictions-unavailable`,
      `instance-changed` and `nonpersistent-session` beside the five v4
      tokens; leave the `sandbox` enum at Codex's three classes. The
      published v5 file adds no unconditional requirement that a v4-valid
      row could fail: every added property is optional and every added
      enum member widens (design D4's superset rule) — evidence / LE2.
- [ ] 2.2 Copy the same bytes to `crates/brokkr-store/src/seat-record.v5.schema.json`
      and add `SCHEMA_V5`/`CONTRACT_V5` beside their four siblings in
      `crates/brokkr-store/src/seat_record.rs` — evidence / LE2.
- [ ] 2.3 Add `SeatRecordVersion::V5` and dispatch it at the current
      **0.10.0** engine line in `SeatRecordVersion::of_engine`, following
      the newest-within-line precedent that already maps 0.9 to v4 and
      0.8 to v3; older and unparseable engines keep v1. No package
      version is bumped — evidence / LE2.
- [ ] 2.4 Extend the built-in conformance in `seat_record.rs`, scoping
      the one constraint that history cannot satisfy. 2.3 sends the whole
      **0.10.0** line to v5, and `version_of` derives that version from
      the run's engine string, so v5 also judges rows the shipped 0.10.0
      engine already wrote — and shipped `codex_started`
      (`crates/brokkr-protocol/src/adapters.rs`) writes `launch: resumed`
      with no `root_session`. An unconditional resumed-requires-root rule
      would therefore refuse valid history at append, export, import
      verification and offline verification, against design D4's superset
      rule and `evidence / LE5`. Scope it on the one admitted within-row
      fact that only this change's engine writes — the 4.3 site stamp: a
      row carrying `site_ref` with `launch: resumed` requires
      `root_session`; an unstamped row keeps its v4 meaning and stays
      valid. The refusal-reason-only-with-`cold` rule is unconditional,
      because no shipped arm emits a reason beside `resumed` (the codex
      resume path sets none), so it invalidates no history. No other
      added constraint may fail a historical v4 or 0.10.0 row.
      Diagnostics name contract and JSON pointer and never echo the
      rejected value — evidence / LE2, evidence / LE5.
- [ ] 2.5 Use the one dispatch and validator at append, export, import
      verification and offline verification through
      `crates/brokkr-store/src/lib.rs`'s existing fence, so a private
      field, invalid enum or oversized identifier is refused before
      append and the attempt's failure is journaled through the existing
      path. The unconditional invariant — that every `resumed` launch
      *this change emits* carries confirmed root evidence — is enforced
      where the records are produced, in group 7's lifecycle and proven
      by the conformance suites of 5.8 and 9.7, not by refusing rows the
      shipped engine already wrote — evidence / LE2, evidence / LE5.
- [ ] 2.6 Tests in `crates/brokkr-store/src/tests.rs`: a v4 row and a v5
      row each validate against their own contract; an old 0.10.0 row
      with none of the new fields still validates under v5; the
      historical regression — a 0.10.0 codex checkpoint with
      `launch: resumed`, no `root_session` and no site stamp, in the
      exact shape shipped `codex_started` writes — still validates under
      v5 and is neither rewritten nor backfilled; the same row carrying a
      `site_ref` stamp **is** refused; a refusal reason beside `resumed`
      is refused; an 81-character `id` and a flag-like `id` are refused;
      a Claude permission mode offered as `sandbox` is refused; export,
      import verification and offline verification agree with append on
      every one of these — evidence / LE2, evidence / LE5.
- [ ] 2.7 Pin the v1–v4 seat-record bytes in
      `crates/brokkr-runtime/tests/frozen_contracts.rs` if any are not
      pinned yet, and assert `contracts/seat-record.v5.schema.json` exists
      beside them, exactly as decision 0046's v4 landed beside v3 — evidence / LE2.

## 3. Structural site identity (design D2)

- [ ] 3.1 Add private `SiteKey`, `InstanceKey` and `ConfirmedSession` to
      `crates/brokkr-runtime/src/engine.rs`. `SiteKey` carries the outer
      seat, the selected case (with explicit absence for an unselected
      body), the body kind and the tagged path — single, panel member,
      sequence model step, or step plus panel member — from distinct
      name and index components taken off the compiled body walk. Never
      split a display tag on `:` to recover ancestry; never case-fold a
      recovered name — site / SR1.
- [ ] 3.2 Derive `site_ref` as canonical JSON through the existing
      `brokkr_core::canonical` SHA-256, domain-separated with
      `brokkr.resume-site/v1`, to 64 lowercase hex; the run journal
      supplies run scope — site / SR1.
- [ ] 3.3 Build `InstanceKey` from this site's agent, provider, model,
      effort and chain index, the normalized driver identity, the pinned
      unexpanded command-template digest, the adapter digest, the engine
      version, the pinned bundle identity, the class, the boundary and
      the hands declaration identity; hash it with
      `brokkr.resume-instance/v1` to `instance_ref`. Exclude ephemeral
      result paths, expanded MCP filenames and capability tokens, whose
      declaration is pinned while each invocation reconstructs the value.
      No resolved credential, provider-home content or private argv
      enters this identity. Missing required identity yields no key and
      therefore no offer — site / SR2.
- [ ] 3.4 Compose a per-invocation site context — key, selected
      candidate, current class, current restrictions — where the actual
      site is composed, and carry it through single dispatch, `MemberRun`
      and sequence model dispatch. Panel workers hand the context key
      beside their checkpoints to the single journal writer for stamping.
      A reused completed step invokes no driver and gains no launch — site / SR1.
- [ ] 3.5 Add the compile-time uniqueness check for the existing
      flattened `Site` addresses within their actual candidate and
      boundary lookup scopes in `crates/brokkr-runtime/src/bundle.rs`, so
      step `a:b`/member `c` and step `a`/member `b:c` cannot alias; an
      ambiguous bundle fails before spawn with a message naming both
      sites. Ordinary repeated member names under different steps stay
      valid, and chain progression and historical tag meanings do not
      move — site / SR2.
- [ ] 3.6 Tests in `crates/brokkr-runtime/src/bundle/tests.rs`: the
      colliding pair is refused at compile time; two panels each holding
      an `alpha` still compile; every bundle under `recipes/` and
      `bundles/` walks clean — site / SR2.
- [ ] 3.7 Tests in `crates/brokkr-runtime/src/engine/resume_tests.rs`:
      the canonical key is stable across process runs and moves for each
      identity axis of 3.3 taken one at a time; two sites with the same
      display tag and different ancestry produce different `site_ref` —
      site / SR1, site / SR2.

## 4. The eligibility query and its four topologies (design D1, D3)

- [ ] 4.1 Replace `seat_session`/`resume_offer` in `engine.rs` with a
      pure query over this run's durable evidence: require a work-class
      model site, an unchanged pinned manifest and `Store::started_here`;
      find the newest checkpoint carrying confirmed root evidence for
      this exact `site_ref`; resolve its effect and attempt to this run's
      requested seat and durable start; require the engine-stamped site
      and instance facts. Keep the existing manifest refusal and
      indeterminate recovery ahead of dispatch — site / SR2, site / SR5.
- [ ] 4.2 Compare the newest session's `instance_ref` with the current
      instance and stop there: an incompatible or ambiguous newest owner
      denies the offer and the query never searches behind it for an
      older matching owner. A failed invocation that recorded no session
      supplies no replacement; a start carrying only an assigned creation
      ID is not session-bearing evidence — site / SR2.
- [ ] 4.3 Stamp `site_ref` and `instance_ref` from the invoking context
      when the engine writes a checkpoint, overwriting or removing any
      driver-supplied value exactly as the boundary stamp already does.
      An aggregate record never establishes a member's session
      ownership — site / SR2, evidence / LE5.
- [ ] 4.4 Offer the complete validated provider ID through `Body::Resume`
      at all four executing work topologies — the single seat, each panel
      member, each sequence model step and each member of a sequence
      panel — replacing the three `session_ref: None` call sites of
      `run_driver`. Pass the originating harness identity to the
      adapter's private start context for group 8's version check — site / SR1.
- [ ] 4.5 Withhold the offer from every gate-class invocation, including
      the single gate that the shipped code offers one today: a single
      seat or panel uses its selected compiled class, a sequence step its
      own compiled class, and a panel member inherits its enclosing
      panel's class. An office name and the presence of a later gate step
      decide nothing (decision 0042 ruling 2) — site / SR1.
- [ ] 4.6 Keep deterministic exec and dialect validation steps outside
      the capability: no offer, no negotiation, no model launch — site / SR1,
      evidence / LE1.
- [ ] 4.7 Tests in `engine/resume_tests.rs` asserting the actual wire
      offers and their absence: a single site's retry then re-entry gets
      A then B and the first cold invocation gets nothing; two panel
      members each get only their own; a sequence with a work author, a
      deterministic validator and a work panel offers the author and both
      members and nothing to the validator; repeated `alpha` labels under
      different parents never cross; a sibling's candidate change denies
      only that sibling; a chain fallback's session is not handed to the
      first candidate on re-selection; a changed provider, model, effort,
      agent, driver, adapter digest, engine identity or pinned bundle
      denies the offer; an imported or unverifiable origin denies it on
      every later retry; a composite checkpoint without an unambiguous
      site stamp denies it — site / SR1, site / SR2.
- [ ] 4.8 Gate tests in the same suite for all four gate topologies —
      single gate, gate-panel member, gate model step, member of a gate
      panel step — on retry, re-entry and operator retry, each proving no
      offer and a fresh launch; plus the corrected historical single-gate
      case, whose journal rows are left unchanged — site / SR1, evidence / LE5.
- [ ] 4.9 Recovery tests in `engine/resume_tests.rs` and
      `crates/brokkr-runtime/tests/recovery.rs`: a fresh engine process
      derives the same offer from the same journal, bundle and origin; an
      effect whose execution is indeterminate stays parked and the
      existence of a resumable session causes no automatic retry or cold
      replacement — site / SR5.

## 5. The correlated one-use offer (design D5, protocol half)

- [ ] 5.1 Advertise `resume` as offer receipt from all four model
      adapters in `AdapterKind::supports` in
      `crates/brokkr-protocol/src/adapters.rs`; exec advertises none.
      Receipt is not a claim that any offered session can be resumed — site / SR4.
- [ ] 5.2 Replace `serve_io`'s uncorrelated `offered: Option<String>`
      with a `PendingOffer { effect_id, attempt_id, handle }` whose
      handle is size-bounded. A matching start consumes it exactly once;
      an empty pending state is an ordinary cold start — site / SR4.
- [ ] 5.3 Poison the exchange on a duplicate offer, malformed
      correlation, wrong effect or attempt, a resume before negotiation
      or a malformed resume envelope: launch no provider, send a bounded
      protocol failure when the correlation is known, otherwise end the
      exchange without pretending a provider refused. Never repair a
      poisoned exchange into a cold execution. Cancellation, shutdown,
      EOF and a completed invocation discard pending state — site / SR4.
- [ ] 5.4 Treat a correlated string that fails the provider ID grammar as
      a declined offer with `invalid-session-id`, whose cold substitute
      independently passes current policy — site / SR4, evidence / LE1.
- [ ] 5.5 Keep driver protocol v1 and `run_attempt_resuming`'s ordering
      in `crates/brokkr-protocol/src/process.rs` unchanged; a driver that
      does not advertise receipt is sent only its normal start, and the
      handle never travels in the prompt, the rendered context or a
      policy input — site / SR4.
- [ ] 5.6 Carry the current attempt's prompt, result destination and
      scoped hands configuration with every rejoin, so an old
      conversation's remembered result path or grant cannot select this
      attempt's output or permissions — site / SR4, safety / AS2.
- [ ] 5.7 Tests in `crates/brokkr-protocol/src/process/tests.rs` and
      `crates/brokkr-protocol/src/adapters/tests.rs`: negotiation then
      one correlated offer then one start; a second start with no offer
      in front of it starts cold; an offer whose effect or attempt
      differs from the next start reaches no provider and survives to no
      later start; two conflicting offers before one start reach none;
      cancel, shutdown and EOF discard the pending offer; the private
      start context is not rendered into the prompt — site / SR4.
- [ ] 5.8 Extend `crates/brokkr-cli/tests/driver_conformance.rs` with the
      receipt capability and the one-use exchange for every built-in
      model adapter — site / SR4, evidence / LE5.

## 6. Adapter support declarations (design D5, declaration half)

- [ ] 6.1 Add the typed `resume` assessment to the adapter declaration in
      `crates/brokkr-runtime/src/agents.rs`: per named execution shape, a
      status of `unmeasured | unsupported | supported`, the assessed CLI
      or wrapper version and the installed version it applies to, the
      applicable classes, boundaries and hands mode, the evidence
      references, and the measured limitations. A supported entry must
      name its interface, restriction, exact-root and current-accounting
      evidence. Absence and malformation are two different outcomes and
      6.6 tests them as two: an **absent** `resume` key, or a shape the
      assessment does not name, resolves to `unmeasured`, loads, and
      enables nothing — the site compiles and invokes cold. Data that is
      **present and malformed** — a bare `true`, a status outside the
      three tokens, a supported entry missing one of its four evidence
      references, an assessment with no assessed version — is not
      normalized to `unmeasured`; the loader refuses it (6.2), so an
      authoring error cannot pass as an honest "not yet measured".
      Neither outcome ever enables resume (design D5) — safety / AS1.
- [ ] 6.2 Validate that shape in the loader and let the existing adapter
      content digest pin it, so a declaration edit moves bundle identity
      as it does today — safety / AS1.
- [ ] 6.3 Carry the selected assessment through `Candidate`/`SiteSpawn`
      into the driver's private context inside `Start.input`, separate
      from the rendered `context`, the phase inputs and the resume
      handle. No new wire type is added — safety / AS1, site / SR4.
- [ ] 6.4 Write the assessments into `adapters/codex.json`,
      `adapters/claude.json`, `adapters/dsh.json` and
      `adapters/lanetally.json` with their honest status as of this
      change: Codex's historically supported work shapes recorded against
      0.148.0 and **disabled** pending 10.5's remeasurement on installed
      0.153.4; Claude's boxed-workspace shape recorded with the 2.1.266
      interface capture and **disabled** pending 10.6; DSH's headless
      work shape **unmeasured** pending 10.3's route and 10.7's proof,
      with the hands deferral left where it is and named as a separate
      matter; LaneTally **unmeasured** pending 10.8 and never marked by
      analogy to Claude.
      `adapters/exec.json` gains no assessment — safety / AS1.
- [ ] 6.5 Update the scaffolded adapter text in
      `crates/brokkr-cli/src/init.rs` and its expectations in
      `crates/brokkr-cli/tests/init_stacks.rs` and
      `crates/brokkr-cli/tests/init_doctor.rs` so a scaffolded workspace
      declares the same shape — safety / AS1.
- [ ] 6.6 Tests in `crates/brokkr-runtime/src/agents/tests.rs`, holding
      6.1's two outcomes apart: each of the three statuses parses; an
      adapter with **no** `resume` key loads, resolves to `unmeasured`,
      and its site compiles and invokes cold; a **present** bare `true`,
      an unknown status token, a supported entry missing one of its four
      evidence references and an assessment whose assessed version is
      absent are each a loader refusal naming the offending field, not a
      silent downgrade to `unmeasured`; the digest moves when an
      assessment moves — safety / AS1.

## 7. The launch lifecycle (design D7, evidence half)

- [ ] 7.1 Model the invocation as `plan -> child spawned -> exact root
      confirmed -> current work -> terminal` in the shared part of
      `adapters.rs`, with explicit resume outcomes: confirmed,
      conclusively rejected before work, failed after
      confirmation or work, and uncertain — evidence / LE1.
- [ ] 7.2 Publish `launch: cold` only when the adapter actually took the
      fresh-session path, and `launch: resumed` only after provider
      evidence confirms the exact offered root before first work. A
      resume flag, a preassigned creation ID, a known old handle,
      surviving edits, replayed transcript rows and an exit status of
      zero prove nothing on their own. A confirmed fresh creation stays
      cold even when the engine or adapter chose its ID — evidence / LE1.
- [ ] 7.3 Emit a bounded refusal reason exactly when an offer was
      declined and the launch is cold; invent no refusal where no offer
      was made; label no unconfirmed resume cold or resumed by
      guesswork — evidence / LE1, evidence / LE2.
- [ ] 7.4 Hold root and launch candidates before `run_seat`'s existing
      first-work boundary, then flush `Accepted`, then the held confirmed
      launch and location facts in observed order, then the first work
      checkpoint. Publish one final launch per executing model site, not
      one per internal spawn. New root evidence uses a pre-work lifecycle
      step that must not satisfy `begins_work` — evidence / LE3.
- [ ] 7.5 Keep the classified pre-session refusal path exactly as shipped
      under proposed 0053: a conclusive provider refusal before work and
      without delivery retains a failed result with no `Accepted` and no
      checkpoints, including no launch row, and a kill inside the held
      window loses those rows — evidence / LE3.
- [ ] 7.6 Permit one cold launch after a local decline, and one cold
      replacement after a provider-rejected resume, and only on measured
      machine session-rejection evidence establishing that no session
      opened, no turn or tool action ran and no result was delivered. A
      generic nonzero exit, stderr prose, missing telemetry, a dropped
      connection or elapsed time establishes nothing. Read the whole
      invocation outcome first: an error-shaped notice followed by work
      or clean delivery is not a rejection, and its delivered work is
      retained — safety / AS4.
- [ ] 7.7 Clear the rejected child's held launch, root, locator,
      model/effort and accounting candidates before the single
      replacement, report that replacement as cold with `harness-refused`,
      and never recurse. A failed replacement reports its own outcome;
      if it is itself a classified pre-work refusal, 7.5's
      no-checkpoint exception applies — safety / AS4, evidence / LE1.
- [ ] 7.8 Share one outer driver process, its process-tree watchdog and
      the original deadline across both launches and the version check of
      8.1: the replacement gets only the remaining budget, no new attempt
      ID, no new chain slot and no new timer. Cancellation or deadline
      termination prevents a later replacement from starting, and a
      watchdog kill keeps `deadline_killed` rather than becoming a
      failure to start because acceptance was held — safety / AS5.
- [ ] 7.9 Tests in `crates/brokkr-protocol/src/adapters/tests.rs` driving
      shim sequences: confirmation then work; conclusive rejection then
      one replacement; an error notice followed by delivery; a different
      root than the one offered; a post-work failure; a failed
      replacement; a cancellation racing the replacement; a rejection
      arriving near the deadline; a watchdog kill before any checkpoint;
      and the held-row order `Accepted`, launch, location, first work — safety / AS4,
      safety / AS5, evidence / LE3.
- [ ] 7.10 Tests that a kill inside the held window fabricates nothing on
      recovery and that group 4's query then offers nothing from that
      attempt — evidence / LE3, site / SR5.

## 8. Provider planners (design D6)

- [ ] 8.1 Probe the selected executable's version once per invocation
      through its measured version interface, with bounded output inside
      the existing deadline, and compare the observed version with the
      pinned assessment and with the originating root's recorded
      `harness_version`. LaneTally additionally checks wrapper identity
      and the underlying Claude version through a measured interface; no
      LaneTally version command is invented. A missing, changed or
      unreadable identity disables resume with `unverified-harness`. The
      observed version is recorded, never the desired pin. No per-attempt
      model experiment runs — safety / AS1.
- [ ] 8.2 Compose the restriction plan in the engine from the declaration
      argv, the model and effort, the generated current hands fragment,
      the result door and the scoped resources, preserving provenance
      before flattening, and compare the actual expanded fragment against
      that plan in the adapter. User passthrough that merely resembles
      generated MCP settings is not authorized by resemblance — safety / AS2,
      safety / AS3.
- [ ] 8.3 Apply a measured resume allow-list with exact arity, duplicate
      and precedence checks. Competing selectors, forks,
      background, cloud or worktree launch, extra positional handles and
      unknown restriction or profile overrides are not forwarded. Apply
      the same selector protection to the cold and gate paths, so an
      ambient `--continue` is never forwarded after an offer is declined;
      an unsafe cold setting fails before provider work. A resume-only
      incompatibility may select a known safe cold spelling;
      cold-inadmissible settings refuse rather than drop a restriction — safety / AS3,
      safety / AS2.
- [ ] 8.4 Refuse an offered handle carrying a flag-like prefix, control
      characters, path traversal, shell syntax or a value outside the
      measured grammar: it is never passed as a selector, never truncated
      into another handle and never echoed into the journal — safety / AS3,
      evidence / LE2.
- [ ] 8.5 Codex: keep `codex exec resume --json`, the workdir through
      `current_dir`, `-c sandbox_mode=...`, the pinned effort and the safe
      passthrough of `codex_resume_blocker`, and admit exact
      engine-generated MCP fragments separately from arbitrary `-c`.
      Re-express the class and effort on every rejoin, including decision
      0030's safe class override, rather than inheriting the old
      thread's — safety / AS2, safety / AS3.
- [ ] 8.6 Claude: build the print/stream-json resume path with exactly
      `--resume <owned-id>` and the current restriction plan — permission
      mode, model and effort, `--tools ""`, strict MCP config, the
      current MCP document and the allowed workspace tool where boxed —
      with the current prompt on stdin, from the 2.1.266 interface in
      `.forge/controller-host-provider-interface.json`. Bare `-r` or
      `--resume`, an empty value, a picker search term, `-c/--continue`,
      `--fork-session`, a user `--session-id`, `--from-pr`, `--teleport`,
      background and cloud selectors and a replacement workdir are all
      excluded. Do not rely on new `--system-prompt` text overriding a
      remembered snapshot; keep the current task and result instruction
      on the current user-input path. Do not append `--restricted`
      blindly. Retain an explicit no-persistence setting and declare that
      shape nonresumable with `nonpersistent-session` — safety / AS2,
      safety / AS3.
- [ ] 8.7 LaneTally: share Claude's parsing where the measurement covers
      it, keep the wrapper and its capture marker, gate the planner
      separately and never substitute plain Claude to make resume
      work — safety / AS1, evidence / LE4.
- [ ] 8.8 DSH: keep the admitted headless profile, the Rust-owned
      per-invocation overlay, the model and effort settings and the
      retained transcript scope, and add owned-session selection only
      through the supported settings or extension route that task 10.3
      establishes — 10.3 is this task's prerequisite, and if it reports
      AS1 upstream this task does not proceed on a guessed route. The
      launcher's TUI `--resume` is not forwarded to
      headless, and the retained directory is not treated as a provider
      handle. An unsupported hands shape stays refused — safety / AS1,
      safety / AS2, safety / AS3.
- [ ] 8.9 Implement SR3's two identity origins: harvest the
      provider-generated root for the known Claude and Codex paths, and
      support a fresh engine- or adapter-assigned creation ID only where
      a measured creation interface requires one. Assignment is
      invocation-local intent held in memory — this change adds no
      durable pre-spawn creation-intent field — and provider confirmation
      must match the assigned value exactly before root evidence is
      emitted. A kill loses unconfirmed intent and the next authorized
      cold creation chooses a fresh value. Latch the root: after it is
      set, a different root is a mismatch, not a last write wins. A
      delegated child session never replaces the root. If implementation
      turns out to need durable intent, return to design for its
      representation rather than widening a start payload — site / SR3,
      site / SR5.
- [ ] 8.10 Planner tests in `adapters/tests.rs` built from the captured
      grammar: the exact resume argv per adapter; the complete current
      class, model and effort on the resume path; the generated fragment
      admitted where passthrough of the same shape is not; a settings
      conflict declined; no ambient continuation on the cold or gate
      path; a nonpersistent shape declared nonresumable; a changed CLI or
      wrapper version disabling resume; identifier injection refused;
      DSH's retained directory not offered as a handle; a DSH profile or
      wrapper override declining the resume with a bounded reason; an
      unsupported hands request still refused; and neither resume nor
      cold able to honour the class ending in refusal — safety / AS2,
      safety / AS3, site / SR3.
- [ ] 8.11 Assignment tests: a confirmed assigned creation reports
      `launch: cold` with root evidence; an assigned ID echoed in a
      start, argv or configuration with unmeasured opening semantics
      produces no session-existence proof and populates no transcript or
      session field; provider evidence naming a different root than the
      assigned one is never recorded as confirmed; and a kill before
      durable confirmation leaves the next retry with no offer and a
      fresh cold identity — site / SR3, site / SR5, evidence / LE1.

## 9. Accounting and legacy compatibility (design D8)

- [ ] 9.1 Establish a measured restored-history/current-work boundary in
      each enabled planner before its usage and tool folds run,
      preferring the provider's turn or event cursor. The cursor or
      interval each planner uses is the one 10.1–10.4 establish for it,
      which is why those four precede this group; a boundary is measured,
      never assumed, and a shape whose boundary is not established is not
      enabled. For retained-file
      accounting capture the owned pre-followup offset or sequence
      through the existing bounded path, and never treat a byte count as
      root confirmation — evidence / LE4.
- [ ] 9.2 Count only the current invocation's new turns, tools, targets
      and reported usage, filtering replayed tools and targets too.
      Rotation, truncation, a missing cursor or uncertain attribution
      omits the affected measurement, or refuses the resume when no
      current-work boundary can be established at all; a lifetime total
      without an established baseline stays absent rather than guessed,
      subtracted or replaced by zero — evidence / LE4.
- [ ] 9.3 Keep the inclusive input and cache-read subsets, cache writes,
      completion deduplication, served model and effort conventions and
      LaneTally's capture identity exactly as they are, and keep a
      rejected resume's unconfirmed usage out of the cold replacement's
      accounting — evidence / LE4.
- [ ] 9.4 Never read historical transcript text to reconstruct a prompt
      or a line of reasoning; retain the existing transcript kinds,
      locators, homes, size caps, retention and path checks; leave #222's
      readers alone — site / SR3, evidence / LE4.
- [ ] 9.5 Legacy lookup: an unambiguous local single-work-site Codex row
      may offer its thread through the established kind and flat-ID
      mapping, subject to every instance, manifest and origin check and
      the current adapter-version qualification. Legacy composites and
      Claude or DSH locator-only evidence start cold once to establish an
      explicit root; no colon ancestry is guessed and no DSH directory is
      equated with a session. A new row without confirmed root cannot
      fall back to legacy fields to evade confirmation — site / SR3,
      evidence / LE4.
- [ ] 9.6 Tests: a replayed stream of ten historical turns followed by
      two new ones records two; an unattributable lifetime total is
      omitted with the limitation documented; a DSH followup over
      retained storage journals only new events; a LaneTally resume keeps
      its capture marker and adds no other ledger entry; a historical
      journal with no launch field still validates and gains no
      backfilled value; a legacy Codex row resumes without rewriting the
      checkpoint; a legacy composite and a DSH-directory-only history
      each start cold — evidence / LE4, evidence / LE5.
- [ ] 9.7 Launch conformance across every built-in adapter in
      `adapters/tests.rs` and `crates/brokkr-cli/tests/driver_conformance.rs`:
      the no-offer cold launch, the safe work-site resume where
      supported, the declined offer, the proven pre-work cold
      replacement, a missing confirmation, a malformed record and the
      privacy bound; one panel member resuming while another starts cold
      keeps each member's own launch and the aggregate substitutes
      neither; exec reports no model launch — evidence / LE1, evidence / LE5.

## 10. Provider evidence — investigation first, then dated live proof

Group 10 is the gate on enablement (`safety / AS1`, answers A, F and G),
and its two halves do not block on the same thing.

**10.1–10.4 are investigation**, preparable in this worktree now: they
read installed CLI help, installed source and packages, and the existing
dated captures under `.forge/`. They run before group 8's
provider-specific construction and group 9's accounting boundaries, whose
argv, routes and cursors are their output.

**10.5–10.8 are proof**, and each stays unchecked until dated controller
host evidence records CLI identity, exact invocation, same-root
confirmation, current restriction enforcement and current-only accounting
for the named shape. They gate group 11 and nothing else.

The seat rechecks its own availability before calling anything unmeasured
and reaches outside the box for no provider source or home. Bounded probes
use temporary test data, change no global provider setting, read no
unrelated session and run no unnecessary model experiment. An interface
that help or source establishes still produces its construction,
validation and shim work while its proof is pending; interface evidence
never enables a shape by itself (`safety / AS1`).

- [ ] 10.1 Codex interface on installed **0.153.4** (or the actual
      installed replacement): `codex exec resume --help` and the
      installed help or source for `--json`, `-c sandbox_mode`, effort
      configuration, safe passthrough and the thread positional,
      recording the observed version and the allowed argv shape. 0030's
      0.148.0 measurement is historical regression scope, not current
      qualification. Prerequisite of 8.5 — safety / AS1.
- [ ] 10.2 Claude interface on **2.1.266**: the resume selector and its
      arity, the restriction flags and their precedence — permission
      mode, model and effort, `--tools ""`, strict MCP config, the boxed
      workspace fragment — the persistence setting, the excluded
      selectors of 8.6, and the stream shape a resumed print invocation
      emits, from installed help and
      `.forge/controller-host-provider-interface.json`. Prerequisite of
      8.6 and 8.7 — safety / AS1, safety / AS2.
- [ ] 10.3 DSH source and interface: installed
      `@deepseek-ai/dsh-session`, `@deepseek-ai/dsh-agent`, the settings
      surface and the Cordis loader and extension exports that headless
      consumes, with package versions, exports, relevant implementation
      and hashes, spending the controller captures where they reach.
      Trace root load versus creation; a supported per-invocation
      configuration route that points headless at that root; replacement
      of restored model, settings and tool state by the current plan; and
      the followup's event interval. The allowed integration is Rust
      construction of supported declarative settings or patch data
      consumed by the installed CLI — no embedded JavaScript runner, no
      monkey-patched `agents.create`, no overridden UUID generation, no
      edited installed package and no TUI substitution. This task is
      8.8's prerequisite: no DSH session selection is written before it
      names the route. Stop when the supported route and its consumers
      establish success, or when those interfaces have been traced and
      the route requires a forbidden mechanism; in the latter case record
      the measured limitation and report **AS1 upstream**, naming
      `proposal.md` and `adapter-resume-safety` as owners — safety / AS1.
- [ ] 10.4 LaneTally interface: wrapper identity, what it forwards to
      Claude, how the underlying Claude version is read through a
      measured interface, and the capture marker's attribution point. No
      LaneTally version command is invented. Prerequisite of 8.7 — safety / AS1.
- [ ] 10.5 Codex proof: a bounded cold/resume probe on the installed
      version recording the exact invocation, class re-imposition,
      same-root confirmation, current-only accounting and the pre-work
      rejection shape — safety / AS1.
- [ ] 10.6 Claude proof on 2.1.266: root-opening semantics, the complete
      effective restrictions and their precedence, including expiry of
      the old grant and operation of the new one, persistent identity,
      and current-only stream and accounting on resume. Record tool
      admission and actual filesystem effects, not a model's statement
      that it was blocked; one denied write does not prove permission
      binding, native-tool removal, MCP exclusion and grant renewal
      together — safety / AS1, safety / AS2.
- [ ] 10.7 DSH proof over 10.3's route: same-root rejoin, replacement of
      the restored model, settings and tool state by the current plan,
      the admitted headless profile and boundary preserved, and
      current-only accounting from the measured event interval — safety / AS1,
      safety / AS2.
- [ ] 10.8 LaneTally proof: wrapper forwarding, the underlying Claude
      version, root confirmation, capture attribution and the applicable
      restrictions on resume. Unsupported hands stay unsupported — safety / AS1.

## 11. Enablement — one task per shape, unchecked until 10.5–10.8 supply their proof

- [ ] 11.1 Enable Codex's previously supported work shapes on the
      remeasured installed version, flip `adapters/codex.json` to
      `supported` with 10.1's interface and 10.5's proof references, and land its live
      restriction, root and accounting assertions beside the shim tests.
      A Codex shape left disabled is incomplete delivery, not a lawful
      close — safety / AS1.
- [ ] 11.2 Enable Claude's boxed-workspace work shape under its
      already-supported boxed boundary with 10.2's interface and 10.6's
      proof, and flip `adapters/claude.json` — safety / AS1, safety / AS2.
- [ ] 11.3 Enable DSH's already-admitted headless work shape through the
      route 10.3 establishes, on 10.7's proof, and flip
      `adapters/dsh.json`, keeping the hands deferral untouched and
      admitting no new tool — safety / AS1, safety / AS2.
- [ ] 11.4 Enable or leave declared-unsupported LaneTally on 10.4's
      interface and 10.8's proof, with its measured reason. Never mark it
      supported by analogy — safety / AS1.
- [ ] 11.5 Record in each declaration and in
      `docs/guides/provider-adapters.md` which shapes remain unmeasured
      and disabled, with their reasons; a disabled shape and a shim-only
      proof are both labelled as such — safety / AS1, evidence / LE5.

## 12. The SDD charter (design D9)

- [ ] 12.1 Change `agents/charters/implementer-sdd.md` once: before a
      group, record it as in progress and name its focused acceptance
      checks; when a task's implementation and its required focused
      checks pass, persist its tick and concise evidence before the next
      group; partial implementation and failed or pending checks stay
      unchecked with a next action; group completion, workspace
      validation, commit and external delivery stay four separate facts.
      Name the dialect's task artifact generically — no framework path,
      no repository command (decision 0042 ruling 6) — progress / PM1,
      progress / PM3.
- [ ] 12.2 Add the recovery clause to the same charter: on retry or
      re-entry, resumed or cold, read the change's current specification,
      design and task artifacts and the current worktree before
      continuing; reconcile ticks against surviving edits and cited
      verification; preserve work that still satisfies its task; return
      an invalidated task to pending with its reason; never erase partial
      uncommitted edits merely because the successor does not remember
      authoring them; never read another session's private transcript.
      Current evidence outranks memory — progress / PM2.
- [ ] 12.3 Leave `agents/charters/implementer.md` unchanged, and leave
      `dialects/openspec.json`, `dialects/speckit.json`, their
      instruction directories and the copies under
      `crates/brokkr-cli/dialects/` with their existing phase maps and
      task formats: neither dialect gains an implement phase or a
      duplicate timing rule — progress / PM3.
- [ ] 12.4 Re-record the charter digest in
      `crates/brokkr-runtime/tests/library_data.rs` from the test's own
      reported pair, and update the rendered-prompt expectations in
      `crates/brokkr-runtime/tests/sdd_shape.rs` and
      `crates/brokkr-runtime/tests/roster.rs` so both dialects' implement
      and return prompts carry progress-before-next-group, truthful
      verification and worktree reconciliation, with OpenSpec's numbered
      `tasks.md` checkboxes and spec-kit's phased rows each supplied by
      the dialect — progress / PM3.
- [ ] 12.5 A deterministic recovery exercise in a temporary worktree: a
      completed group with recorded progress, a second group marked in
      progress, an interruption before any commit, and a successor
      reading group one's checked work and group two's incomplete status
      from the artifact rather than finding everything unchecked. The
      evidence identifies it as an exercise and an instruction check, not
      a claim that a live model always obeys — progress / PM1,
      progress / PM2.
- [ ] 12.6 A test that a judge's rendered prompts grant no task edit: the
      review offices report findings to the owning artifact and leave the
      task file and the tree unchanged — progress / PM3.

## 13. Prose

- [ ] 13.1 `docs/guides/provider-adapters.md`: the offer, the assessment
      shape, what each adapter's declaration says today, the measured
      limitations, and the held-window limitation that a kill before
      first work leaves no session evidence — safety / AS1, evidence / LE3.
- [ ] 13.2 `docs/guides/driver-authoring.md`: how a third-party driver
      advertises receipt, what a correlated one-use offer looks like on
      the wire, and that not advertising it means never being handed a
      handle — site / SR4.
- [ ] 13.3 `docs/guides/journal-and-verification.md`: seat-record v5, its
      new optional fields, its five added refusal tokens, the 0.10.0
      dispatch line and the guarantee that v1–v4 journals stay valid and
      unrewritten — evidence / LE2, evidence / LE5.
- [ ] 13.4 State the two honest limits in the guides as well as in 0056:
      the local origin check does not authenticate a provider account,
      and a charter instruction is not an engine guarantee — site / SR5,
      progress / PM3.

## 14. Re-pins

- [ ] 14.1 Re-record the moved witness digests in
      `crates/brokkr-runtime/tests/witness_digests.rs` from the tests'
      own reported left/right pairs — the adapter declarations of group 6
      and the charter of group 12 move every identity that consults
      them — and change no pin the tests did not report — safety / AS1,
      progress / PM3.
- [ ] 14.2 Compile `bundles/self` and `bundles/verify` and reconcile any
      manifest digest the compile reports — safety / AS1.

## 15. Gates, commit and the fold

The commands below are this commission's, recorded here and not promoted
into capability truth (`progress / PM4`). Run them with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

- [ ] 15.1 `cargo fmt --all -- --check` — every requirement of this change.
- [ ] 15.2 `cargo clippy --workspace --all-targets --all-features --locked
      -- -D warnings` — every requirement of this change.
- [ ] 15.3 `cargo test --workspace --all-features --locked` — every
      requirement of this change.
- [ ] 15.4 `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
      and `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify`
      — every requirement of this change.
- [ ] 15.5 `TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash
      scripts/coverage-exact.sh`, on the host and with the gate
      unchanged. Inside a nested sandbox this is pending host proof, not
      a pass, and skipped boundary tests prove nothing — every
      requirement of this change.
- [ ] 15.6 Fold the change **before** the commit, so the moved change and
      the four promoted capability files land in the same head as the
      code they describe (decision 0042 ruling 6, design Migration Plan 4
      then 5): run the dialect's archive operation, then append one
      provenance line — a list item naming the archived directory in
      backticks, an em dash, and `folded <YYYY-MM-DD>` — in the exact
      spelling `dialects/openspec/archive.md` gives, under the
      `## Provenance` heading at the end of
      `openspec/specs/site-session-resumption/spec.md`,
      `openspec/specs/adapter-resume-safety/spec.md`,
      `openspec/specs/adapter-launch-evidence/spec.md` and
      `openspec/specs/sdd-progress-markers/spec.md`, rewriting no
      existing line. The fold is a gated change, not a bookkeeping step:
      `crates/brokkr-cli/tests/provenance.rs` walks `openspec/specs` and
      `openspec/changes/archive` in both directions, so re-run 15.3 (or
      at least that test) after the fold and before the commit — progress / PM4.
- [ ] 15.7 Commit the completed work unsigned, in the repository's
      message style, with no push and no merge. This is the last write:
      it carries the code, the tests, the proposed decision, the archived
      change, the four promoted capability files with their provenance
      lines, and the final state of this file's ticks and `## Progress`
      section, so the head handed on in 15.8 is the committed one and
      nothing of the fold is left in the worktree — progress / PM4.
- [ ] 15.8 Hand that committed head and the evidence to the controller and
      leave pending, because their results do not exist yet: host
      validation, remote CI, integration with the #222 fire's
      shared-file overlap, completed-run publication, the PR, the merge
      and the closing of #226 — progress / PM4.

## Tasks-phase validation — 2026-09-09

This section records what this seat did and did not establish. It authors
the breakdown only; no production file, decision, contract or charter was
touched.

- Coverage was self-checked by reading the four deltas: all **19**
  requirements — SR1–SR5, AS1–AS5, LE1–LE5, PM1–PM4 — are named by at
  least one task, and every task names the requirement it serves. The
  breakdown holds **101** tasks in 15 groups.
- `openspec validate 226-session-resumption --strict --no-interactive`
  did **not** run, on the first visit or on the return: this seat's shell
  refuses the `openspec` binary, so the strict validator and the status
  readout have no result from here. Analyze reports running it
  successfully against the artifacts as they stood before this return's
  repairs, which touch task prose and numbering only.
  The specify and design passes recorded their own passing runs; this
  file's shape follows `dialects/openspec/tasks.md` and the archived
  `2026-09-06-boundary-named-slice-i/tasks.md` precedent.
- Unlike the specify and design seats, this seat **does** have `cargo`
  (1.98.0) on PATH. That corrects those artifacts' environment note for
  the implementation seat, which should recheck its own availability
  before relying on either statement. No Rust check was run here, because
  this phase changes no code and a green suite is group 15's evidence,
  not this artifact's.
- No provider CLI was probed and no provider evidence was created. The
  two dated controller captures under `.forge/` are cited by groups 8 and
  10 exactly as the proposal and design identify them.
- Frozen paths, contracts, policy, reference and fixtures have no diff
  from the commissioned base.

## Return — analyze drift, 2026-09-09

The breakdown came back with four findings, all first introduced here, and
all four are repaired in this file. No proposal, delta or design artifact
was edited: analyze established no fault in them, and each repair holds
the tasks to what those artifacts already say.

- **F1 (HIGH), 2.1, 2.4, 2.5 and 2.6.** The old 2.4 required the store to
  refuse any `resumed` row without `root_session`, while 2.3 sends the
  whole 0.10.0 line to v5 — and shipped `codex_started`
  (`crates/brokkr-protocol/src/adapters.rs:1870`) writes exactly that
  row, judged by whichever version the run's engine string selects. That
  rule would have refused valid shipped history at append, export, import
  verification and offline verification, against design D4's superset
  rule and `evidence / LE5`. Repaired both ways the finding allows: the
  fence rule is scoped on the 4.3 site stamp, the one within-row fact
  only this change's engine writes, so unstamped history keeps its
  meaning; and the unconditional new-producer invariant moves to where
  records are produced, group 7's lifecycle with the 5.8 and 9.7
  conformance suites. 2.6 gains the historical 0.10.0
  resumed-without-root regression the finding asked for, alongside the
  stamped row that must still be refused. Verified while repairing: no
  shipped arm emits a refusal reason beside `resumed`, so that half of
  2.4 stays unconditional and invalidates nothing.
- **F2 (MEDIUM), 6.1 and 6.6.** The two tasks demanded different
  observable outcomes for the same input. Reconciled on the distinction
  the finding names, which is also design D5's: an *absent* `resume` key
  resolves to `unmeasured`, loads and compiles cold; *present malformed*
  data is a loader refusal naming the field, never a silent downgrade.
  Neither outcome enables resume.
- **F3 (MEDIUM), the header, group 10, 6.4, 8.8, 9.1, group 11 and the
  progress record.** Group 10 is split at its real seam: 10.1–10.4 are
  interface investigation, preparable here from installed help, source
  and the dated `.forge/` captures, and 10.5–10.8 are the blocked live
  proof. The stated execution order now runs the investigation before
  its consumers — 8.5–8.9 and 9.1–9.3 — and the proof after them, gating
  only group 11. Each dependent task names its prerequisite, and the
  progress record below states dependencies as they bind rather than by
  group number.
- **F4 (MEDIUM), 15.6 and 15.7.** The commit preceded the fold, which
  would have left the archived change and the four promoted capability
  files outside the committed head, against decision 0042 ruling 6 and
  the design's Migration Plan 4-then-5. Swapped: fold, re-run the gate
  the fold moves — `crates/brokkr-cli/tests/provenance.rs` walks
  `openspec/specs` and the archive in both directions — then commit, then
  hand on that committed head.

The repair added four tasks (97 to 101) and moved no requirement citation:
all 19 requirements remain named.

## Progress

Nothing is in progress: this file is the tasks phase's artifact and no
implementation has begun. Prerequisites, stated as they bind rather than
by group number:

- **Preparable from the repository alone**: groups 1–7, tasks 8.1–8.4,
  and groups 12, 13 and 14, plus the investigation tasks 10.1–10.4 to
  whatever depth installed help, installed source and the dated `.forge/`
  captures reach.
- **Blocked on that investigation**: 8.5–8.9 each need their provider's
  interface from 10.1–10.4, and 8.8 in particular is not written before
  10.3 names DSH's supported route; 9.1–9.3 need the measured
  current-work boundary from the same four tasks. 8.10, 8.11 and 9.6
  follow their subjects.
- **Blocked on dated controller host evidence**: 10.5–10.8, and group 11
  entirely, which flips no declaration without them. Group 6 is *not*
  blocked: 6.4 records the honest disabled or unmeasured status
  meanwhile, which is what makes the declarations landable now.
- **Blocked on the host**: 15.5's coverage gate. Inside a nested sandbox
  it is pending host proof, and skipped boundary tests prove nothing.
- **The controller's, not this fire's**: everything named in 15.8, whose
  results do not exist yet.
