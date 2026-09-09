# Tasks: Same-instance session resumption and durable progress (#226)

Groups are the design's landing order (Migration Plan 1–5), which is the
order another smith executes them in: the proposed ruling before any
production semantic edit, the record version before anything emits its
fields, the site identity before the query that keys on it, the query
before the wire that carries it, the wire before the adapters that
receive it, the shared launch lifecycle before the four provider
planners, accounting after the planners that feed it, the charter after
the engine work it describes, then the prose, then the re-pins, then the
gates, then the fold.

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
      tokens; leave the `sandbox` enum at Codex's three classes — evidence / LE2.
- [ ] 2.2 Copy the same bytes to `crates/brokkr-store/src/seat-record.v5.schema.json`
      and add `SCHEMA_V5`/`CONTRACT_V5` beside their four siblings in
      `crates/brokkr-store/src/seat_record.rs` — evidence / LE2.
- [ ] 2.3 Add `SeatRecordVersion::V5` and dispatch it at the current
      **0.10.0** engine line in `SeatRecordVersion::of_engine`, following
      the newest-within-line precedent that already maps 0.9 to v4 and
      0.8 to v3; older and unparseable engines keep v1. No package
      version is bumped — evidence / LE2.
- [ ] 2.4 Extend the built-in conformance in `seat_record.rs` so
      `launch: resumed` requires `root_session`, a refusal reason appears
      only with `launch: cold`, and no added constraint invalidates a
      historical v4 row. Diagnostics name contract and JSON pointer and
      never echo the rejected value — evidence / LE2.
- [ ] 2.5 Use the one dispatch and validator at append, export, import
      verification and offline verification through
      `crates/brokkr-store/src/lib.rs`'s existing fence, so a private
      field, invalid enum or oversized identifier is refused before
      append and the attempt's failure is journaled through the existing
      path — evidence / LE2, evidence / LE5.
- [ ] 2.6 Tests in `crates/brokkr-store/src/tests.rs`: a v4 row and a v5
      row each validate against their own contract; an old 0.10.0 row
      with none of the new fields still validates under v5; `resumed`
      without `root_session` is refused; a refusal reason beside
      `resumed` is refused; an 81-character `id` and a flag-like `id` are
      refused; a Claude permission mode offered as `sandbox` is refused;
      export, import verification and offline verification agree with
      append — evidence / LE2, evidence / LE5.
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
      evidence. Missing data, a bare `true` or an absent assessment reads
      as `unmeasured` and enables nothing — safety / AS1.
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
      0.148.0 and **disabled** pending the 0.153.4 remeasurement of task
      10.1; Claude's boxed-workspace shape recorded with the 2.1.266
      interface capture and **disabled** pending 10.2; DSH's headless
      work shape **unmeasured** pending 10.3, with the hands deferral
      left where it is and named as a separate matter; LaneTally
      **unmeasured** pending 10.4 and never marked by analogy to Claude.
      `adapters/exec.json` gains no assessment — safety / AS1.
- [ ] 6.5 Update the scaffolded adapter text in
      `crates/brokkr-cli/src/init.rs` and its expectations in
      `crates/brokkr-cli/tests/init_stacks.rs` and
      `crates/brokkr-cli/tests/init_doctor.rs` so a scaffolded workspace
      declares the same shape — safety / AS1.
- [ ] 6.6 Tests in `crates/brokkr-runtime/src/agents/tests.rs`: each
      status parses; a bare `true`, a supported entry missing one of its
      four evidence references, and an assessment whose assessed version
      is absent are all refused; an adapter with no `resume` key loads
      and resolves to `unmeasured`; the digest moves when an assessment
      moves — safety / AS1.

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
      establishes. The launcher's TUI `--resume` is not forwarded to
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
      preferring the provider's turn or event cursor; for retained-file
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

## 10. Provider evidence — controller-supplied, unchecked until dated results exist

These four tasks are the gate on enablement (`safety / AS1`, answers A, F
and G). Each stays unchecked until dated controller host evidence records
CLI identity, exact invocation, same-root confirmation, current
restriction enforcement and current-only accounting for the named shape.
The seat rechecks its own availability and reads the existing captures
under `.forge/` before calling anything unmeasured; it reaches outside
the box for no provider source or home. Bounded probes use temporary test
data, change no global provider setting, read no unrelated session and
run no unnecessary model experiment.

- [ ] 10.1 Codex on installed **0.153.4** (or the actual installed
      replacement): `codex exec resume --help`, and a bounded cold/resume
      probe recording the exact invocation, the allowed argv, class
      re-imposition, same-root confirmation, current-only accounting and
      the pre-work rejection shape. 0030's 0.148.0 measurement stays
      historical regression scope — safety / AS1.
- [ ] 10.2 Claude **2.1.266**: root-opening semantics, the complete
      effective restrictions and their precedence — permission mode,
      `--tools ""`, strict MCP config, the boxed workspace fragment,
      including expiry of the old grant and operation of the new one —
      persistent identity, and current-only stream and accounting on
      resume. Record tool admission and actual filesystem effects, not a
      model's statement that it was blocked; one denied write does not
      prove permission binding, native-tool removal, MCP exclusion and
      grant renewal together — safety / AS1, safety / AS2.
- [ ] 10.3 DSH: controller captures of installed
      `@deepseek-ai/dsh-session`, `@deepseek-ai/dsh-agent`, the settings
      surface and the Cordis loader and extension exports that headless
      consumes, with package versions, exports, relevant implementation
      and hashes. Trace root load versus creation; a supported
      per-invocation configuration route that points headless at that
      root; replacement of restored model, settings and tool state by the
      current plan; and the followup's event interval. The allowed
      integration is Rust construction of supported declarative settings
      or patch data consumed by the installed CLI — no embedded
      JavaScript runner, no monkey-patched `agents.create`, no overridden
      UUID generation, no edited installed package and no TUI
      substitution. Stop when the supported route and its consumers
      establish success, or when those interfaces have been traced and
      the route requires a forbidden mechanism; in the latter case record
      the measured limitation and report **AS1 upstream**, naming
      `proposal.md` and `adapter-resume-safety` as owners — safety / AS1.
- [ ] 10.4 LaneTally: wrapper identity and forwarding, the underlying
      Claude version, root confirmation, capture attribution and the
      applicable restrictions. Unsupported hands stay unsupported — safety / AS1.

## 11. Enablement — one task per shape, unchecked until group 10 supplies its proof

- [ ] 11.1 Enable Codex's previously supported work shapes on the
      remeasured installed version, flip `adapters/codex.json` to
      `supported` with 10.1's evidence references, and land its live
      restriction, root and accounting assertions beside the shim tests.
      A Codex shape left disabled is incomplete delivery, not a lawful
      close — safety / AS1.
- [ ] 11.2 Enable Claude's boxed-workspace work shape under its
      already-supported boxed boundary with 10.2's evidence, and flip
      `adapters/claude.json` — safety / AS1, safety / AS2.
- [ ] 11.3 Enable DSH's already-admitted headless work shape through the
      route 10.3 establishes, and flip `adapters/dsh.json`, keeping the
      hands deferral untouched and admitting no new tool — safety / AS1,
      safety / AS2.
- [ ] 11.4 Enable or leave declared-unsupported LaneTally on 10.4's
      evidence, with its measured reason. Never mark it supported by
      analogy — safety / AS1.
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
- [ ] 15.6 Commit the completed work unsigned, in the repository's
      message style, with no push and no merge — progress / PM4.
- [ ] 15.7 Fold the change: run the dialect's archive operation, then
      append one provenance line — a list item naming the archived
      directory in backticks, an em dash, and `folded <YYYY-MM-DD>` — in
      the exact spelling `dialects/openspec/archive.md` gives, under the
      `## Provenance` heading at the end of
      `openspec/specs/site-session-resumption/spec.md`,
      `openspec/specs/adapter-resume-safety/spec.md`,
      `openspec/specs/adapter-launch-evidence/spec.md` and
      `openspec/specs/sdd-progress-markers/spec.md`, rewriting no
      existing line — progress / PM4.
- [ ] 15.8 Hand the final head and the evidence to the controller and
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
  breakdown holds **97** tasks in 15 groups.
- `openspec validate 226-session-resumption --strict --no-interactive`
  did **not** run: this seat's shell refuses the `openspec` binary, so
  the strict validator and the status readout have no result from here.
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

## Progress

Nothing is in progress: this file is the tasks phase's artifact and no
implementation has begun. Groups 10 and 11 depend on dated controller
provider evidence that does not exist in this worktree; groups 1–9 and
12–14 depend only on the repository. Group 15's coverage gate depends on
the host.
