# Tasks: Same-instance session resumption and durable progress (#226)

Current tasks visit, 2026-09-21: **the final targeted DSH composite R1–R4
repair**, run `dsh-composite-identity-issue-226-e291e076`. Adopt every commit
on `slice-dsh-composite-b` through design HEAD
`136f918305d4f75ccb5f6148527291323c0fa187`, including commissioned baseline
`81d7fe87`, proposal/specification `be8cd9d8`, the clean resolver port and
its narrowings. Proposal **AT**, AS1's five added scenarios and the current
D6/D10/D11 design precede this breakdown. No `returned_from` is supplied;
the commission carries chief run `124cca78`'s four findings. No earlier
artifact must change before an honest breakdown can be drafted.

Only **8.8(a)–(c)** and exactly **R1–R4** are commissioned. Execute the
unchecked repair clauses below in numbered order. The native cursor walk,
reached-cwd barrier, existing causes, fixed rc.2 inputs, sole producer and
sound earlier repairs are adopted. Prior proofs remain attributed history;
new guards, invocation carriage and pre-probe admission need their own tests
and removal/restoration evidence. This plan supersedes the previous
verification-only visit; dated accounts remain history.

All fourteen local addresses and their **5 complete / 9 pending** states are
preserved. All 101 change-wide addresses remain **84 complete / 17 pending**:
**115 numbered rows, 89 complete / 26 pending** in total. With 18 retained
historical checkbox rows, the file remains **133 checkbox rows, 105 complete /
28 pending**. No checkbox changes in this tasks phase. **8.8 stays unchecked**.
The five capability deltas have **20 requirements / 224 scenarios**; specify
already authored the five new scenarios.

Every local task serves **safety / AS1 — Resume support is measured per
adapter and execution shape**, in the
[safety delta](specs/adapter-resume-safety/spec.md#requirement-as1-resume-support-is-measured-per-adapter-and-execution-shape).
Scenario bindings appear below. Change-wide requirement citations and archive
ordering are preserved. This is a breakdown checkpoint, not implementation
completion, a council verdict or acceptance of debt.

Keep `contracts/`, `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/dsh/` and `docs/decisions/` byte-identical to the
adopted head; **0056 stays proposed**. Part **(d)**, **8.10**, **9.6**,
**10.6–10.8**, **11.1–11.4** and **groups 14–15** are excluded. No planner
completion, new crate/dependency, public resolver, YAML AST, identity format,
persisted field, production trial execution or installation change follows.
Production repair stays in the existing Rust protocol and doctor surfaces
and their tests. Use owned harmless fixtures and isolated child environments;
`.forge/` is evidence, never identity input.

Native macOS/Windows/MSRV, immutable Apple/env source pins and the retained-
Node absent-PATH positive/removals remain pending under their existing owners,
without becoming new work in this final repair. Capable-host exact coverage
and final-head remote CI retain their own pending results. The change is
already active, so no reopen is needed. The dialect archive/fold remains the
final whole-change operation in group 15; this partial slice performs none.

## Decisions

Adopt **AT** and **D10, Final R1–R4 council reconciliation**. R1 consumes
supported structural ASCII separation before scalar admission. R3 bounds raw
implicit block-key spelling, including quotes and pre-colon spaces, before
trimming or decoding. Neither normalizes identity data, changes the file cap
or expands YAML support. R2 carries selected invocation/native argv[0]
separately from canonical identity and qualifies selected env dispatch before
probing. R4 refuses an established env shebang with no nonblank program; the
old bare-env positive is withdrawn. Direct env availability is a separate case.

The pnpm fixes require shared protocol preparation before either version
probe. Retain admitted profile/pnpm data for composition; no caller-supplied
probe authority, doctor parser or second producer. Located-lock admission
failure blocks DSH and Node. Independent home/profile location failure keeps
the permitted version-only diagnostic and cannot yield an identity. D10's
state table governs both callers. Preparation is the wiring of R1/R3, not a
fifth repair. No behavioral ambiguity remains: AS1 records all five scenarios.

Preserve the resolver rule: equality with native is necessary — nothing is
selected that native would not execute — and not sufficient: a candidate in
the working directory (an empty PATH entry or the implicit cwd iteration glibc
produces after an oversized skip) is NEVER selected and NEVER skipped past.
Preserve an established native terminal cause at reached cwd; otherwise name
`the platform's search would fall into the working directory`. Earlier success,
native absent-PATH defaults and explicit direct paths retain their rules.

A removal is one independent compiling mutation, a nonzero executed test count,
and failure at the intended reason/identity/no-probe assertion. Record revision,
production line or behavior, exact command, test, failed assertion, restoration
and green rerun. Compile/setup errors, timeouts, unrelated failures, zero-test
filters and bare `is_err()` provide no proof. Restore each mutation before the
next. Separate native oracle markers from doctor markers; assert both DSH and
Node marker absence before report prose. Silent env failures also need injected
probe-call assertions. Controls execute and establish their intended version
or unchanged composite, so blanket refusal cannot pass.

Run crate suites sequentially, never `cargo test --workspace` or workspace-wide
coverage tests in the box. For the commissioned #255 Text-file-busy flake,
record both attempts and rerun that crate: its green rerun is green. Missing
Cargo is unavailable execution, not a test failure or upstream artifact fault.
The unchanged literal coverage gate belongs on a capable host because the box
cannot execute boundary namespace tests. Notes describe outcomes; they never
direct or waive a gate.

The operator authorized one final run of these four repairs. Independent
verify/full review owns its actual findings. Any remaining pnpm MEDIUM is
recorded for the operator's debt ruling, without further repair, self-granted
waiver or another automatic reforge assignment. No clause licenses work beyond
R1–R4.

### Execution order and requirement coverage

Preserved clauses constrain the work; they do not assign old mutations again.
Broad task owners remain unchecked for inherited pending evidence even after
their current repair clauses are delivered. Record partial delivery explicitly;
do not narrow full acceptance to earn a tick. AS1 names the requirement above.

| Order / task | Current work and evidence | Requirement and scenario binding |
|---|---|---|
| 1 — 8.8.1.1 | R2 invocation/identity carriage and selected env dispatch. | AS1 — Selected invocation is not replaced by its canonical target. |
| 2 — 8.8.1.2 | R2 alias, direct-env and invocation-sensitive controls; two removals. | AS1 — Selected invocation is not replaced by its canonical target; Doctor version and composite follow the same selected DSH installation. |
| 3 — 8.8.2.1 | R4 missing-program refusal; flip old admitted row. | AS1 — An env launcher without a nonblank program refuses before probing. |
| 4 — 8.8.2.2 | R4 producer/doctor refusals, bounded native evidence, env-sh control and removal. | AS1 — An env launcher without a nonblank program refuses before probing; Interpreter aliases cannot bypass the Node obstruction refusal. |
| 5 — 8.8.3.1 | R1/R3 pre-probe preparation, syntax guards, controls and guard/wiring removals. | AS1 — Flow-value separator padding cannot hide opening syntax; Implicit ignored block keys obey YAML lookahead; Implicit-key lookahead includes raw spelling and pre-colon separation. |
| 6 — 8.8.4.1 | Preserve scope-local YAML key comparison proof. | AS1 — Duplicate decoded pnpm package keys refuse before triple normalization. |
| 7–8 — 8.8.5.1–5.2 | Preserve sole-producer literal provenance/conformance. | AS1 — Plugin expectations are recorded from the sole producer. |
| 9 — 8.8.6.1 | Preserve terminal-safe cause rendering. | AS1 — A nonexistent override cannot inject terminal control bytes through doctor. |
| 10 — 8.8.7.1 | Preserve missing-manifest cause/later-hit control. | AS1 — Removing only the plugin manifest names the drifted file. |
| 11 — 8.8.8.1 | Consolidate assertions/removals; retain data and independent-unavailability controls. | AS1 — Doctor version and composite follow the same selected DSH installation; Executable selection and home availability are independent requirements; Digest acceptance is proved by removal without completing the planner. |
| 12 — 8.8.8.2 | Fresh fmt, clippy, seven sequential crate suites, both bundles, strict OpenSpec. | AS1 — Delivery evidence distinguishes executed positives from pending platforms. |
| 13 — 8.8.8.3 | Fresh committed-byte coverage: three integer pairs and capable-host gate. | AS1 — Fresh exact coverage cannot be replaced by retained perfect reports. |
| 14 — 8.8.8.4 | Commit delivered/pending account and hand off for independent verify/review. | AS1 — Digest acceptance is proved by removal without completing the planner; Delivery evidence distinguishes executed positives from pending platforms. |

## 8.8.1. Preserve selected invocation separately from identity — current R2

- [ ] 8.8.1.1 Extend existing private selection/retained seam values in
  `crates/brokkr-protocol/src/adapters/composite.rs`: keep candidate spelling
  and native argv[0] beside canonical file identity. Change
  `classify_in`/`selected_from` and consumers so
  `crates/brokkr-cli/src/doctor.rs` launches the already-selected invocation;
  core discovery and containment consume canonical identity. Preserve explicit
  path spelling and the same distinction in retained Node data where shared
  selection requires it. No new bare-name lookup, canonical-target execution
  with only argv[0] patched, fallback or trial execution is permitted.
  Factor the existing file-established env invocation check if needed and
  apply it at selected-executable admission as well as shebang admission.
  Searched `dsh -> /usr/bin/env` and an absolute `dsh` alias refuse before
  either probe, naming selected invocation and unestablished env dispatch.
  Direct `/usr/bin/env` retains its native availability control; never pass
  fabricated empty shebang arguments to establish it. Preserve cursor bytes/
  order, terminal/remembered causes, cwd barrier, defaults, override precedence
  and retained head/Node. Adopt resolver removals without replay. Immutable
  Apple source pins and native platform evidence remain this owner's pending
  inherited acceptance, outside the repair — safety / AS1: Resume support
  is measured per adapter and execution shape.

- [ ] 8.8.1.2 After 8.8.1.1, extend protocol selection tests, doctor unit
  seams and `crates/brokkr-cli/tests/doctor_dsh_selection.rs` with both R2
  alias forms and direct env under the same isolated child environment.
  Record each native alias result separately (prior uutils: exit 1, name
  mismatch, no stdout); assert producer-selection cause, no probe target and
  zero doctor DSH/Node calls/markers. A silent native failure cannot prove
  no doctor execution. Direct env's native version result is an availability
  control, never a readable DSH composite. Add an admitted argv/path-sensitive
  launcher control proving retained invocation spelling/argv[0], separate
  canonical identity and no second search; preserve valid launcher aliases.
  Independently remove selected-env qualification, then independently restore
  canonical execution: the first fails producer/doctor pre-probe refusal,
  the second the admitted control's invocation assertion. Restore and rerun
  focused tests green with nonzero counts. Existing paired native/doctor
  cursor controls remain intact; native macOS remains pending, not newly
  commissioned — safety / AS1: Resume support is measured per adapter and
  execution shape.

## 8.8.2. Refuse an env shebang without a program — current R4

- [ ] 8.8.2.1 After 8.8.1, change `env_program`'s successful absence for
  an empty or ASCII-space/tab-only argument tail to a cause-bearing refusal
  after interpreter identity/dispatch qualification. Name launcher, env
  interpreter and missing nonblank program; refuse before nested lookup or
  probe authority. Flip the bare `#!/usr/bin/env` row currently admitted in
  `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`
  (`composite/tests.rs`, adopted line 4719) to a named refusal alongside blank
  variants. Keep non-env interpreters and established `env sh` distinct from
  env-without-program; preserve measured `env node`, retained Node/head,
  option/assignment/CR/encoding refusals and direct env availability. Adopt
  earlier env identity/own-file-name guards and removals through `81d7fe87`.
  Immutable env source pins and missing native platform proof remain inherited
  debts — safety / AS1: Resume support is measured per adapter and execution
  shape.

- [ ] 8.8.2.2 After 8.8.2.1, test bare and spaces/tabs-only env shebangs
  through protocol selection/producer, injected doctor probes and the built
  doctor. Assert exact missing-program cause/context, no selected probe target
  and zero DSH/Node callbacks/markers. The owned `#!/usr/bin/env sh` control
  terminates normally; retain valid env-node and ordinary launcher controls.
  Native bare/blank reproductions use separate markers and an external
  process-group timeout with cleanup/reaping; record native outcomes. Doctor
  returns without starting the loop; a doctor timeout fails acceptance.
  Independently restore blank-tail `Ok(None)` admission: selection and callback
  tests fail the missing-program/no-probe assertions without spawning an
  unbounded loop. Restore exactly and rerun green. Preserve the complete
  differential matrix and independent inventory in
  `native_executable_resolution_matches_command_matrix` and
  `terminal_path_lengths_refuse_before_doctor_probe`; no cursor rewrite,
  matrix expansion or old-removal replay follows. Native macOS/Windows matrix,
  GetBinaryTypeW, Windows MSRV and source-pin cells retain their original
  pending acceptance — safety / AS1: Resume support is measured per adapter
  and execution shape.

## 8.8.3. Admit pnpm syntax before both probes — current R1 and R3

- [x] 8.8.3.1 After groups 1–2, implement these clauses in order in
  existing protocol/doctor code and tests. Each clause serves safety / AS1:
  Resume support is measured per adapter and execution shape.

  1. **Shared preparation.** Prepare the located profile and bounded pnpm
     observation in the protocol before authority for either DSH or Node
     version probing, for doctor and the standalone producer. Retain profile
     declarations, patchReload, raw lookup anchor, canonical boundary and
     admitted dependencies in a small privately constructed value. Composition
     consumes them once, without profile/lock rereads, caller-supplied digests
     or a doctor parser. Preparation computes no digest. Local exclusion names
     derive from retained declarations, but every declared component must still
     resolve by existing first-hit/containment rules and be measured before
     exclusions contribute to a successful identity. Located pnpm admission
     failure blocks both probes with named context/cause, no composite and no
     invented version. Preserve version-only reporting for independent home/
     profile-location failure; it grants no readable composite. Preserve
     declaration/warning context and no Node or fallback after a failed DSH
     version probe.
  2. **R1 separation.** At `split_flow_entry`, consume the entire admitted
     ASCII-space separator run before `flow_scalar` inspects opening syntax.
     Preserve quoted content, existing tab refusal, identity-bearing whitespace
     and the closed grammar. Through the sole producer and built doctor, test
     `engines: {node:  *missing}`, `{node:  &}` and `{node:  %bad}` with one,
     two and additional separator spaces. Assert pnpm, engines/member and
     malformed-opening cause, no composite and no DSH/Node probes. One-space
     and padded numeric controls (`22`) and valid quoted controls remain
     readable with the same control composite. Extend
     `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
     and `ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`;
     helpers expose separate DSH and Node markers.
  3. **R3 raw span.** At `pnpm_ignored_line`, bound the original implicit
     block-key slice before trimming separation or decoding its scalar. Count
     Unicode characters including quotes and pre-colon spaces; exclude
     indentation, colon and value. Maximum: 1,024. Through producer and built
     doctor cover 1,024/1,025 ASCII characters; 1,023/1,024 plus one space;
     and quoted keys with 1,022/1,023 content characters plus two delimiters.
     Every 1,025-character span refuses naming pnpm, peerDependencies and the
     implicit-key lookahead limit of 1,024, with no composite or either probe
     marker. Every 1,024-character span preserves the valid ignored-body
     composite. Include otherwise admitted multibyte keys to prove character
     rather than byte counting, and long scalar values to rule out a blanket
     line/value cap. Preserve the inclusive 8,388,608-byte file bound,
     scope-local decoded-key comparisons, ignored-body structure and identity
     serialization.
  4. **Independent proof.** Restore one-space consumption for R1; remove
     the raw-span guard for R3; separately move R3's guard after trimming.
     Each compiling mutation fails its producer and built-doctor reason/
     no-composite assertion, including padded-over-limit for trimming.
     Separately bypass admission before DSH and before Node: each fails a
     callback/marker assertion before report prose. Separately reopen retained
     pnpm/profile input after a version probe rewrites it: the retain-and-reuse
     assertion fails. Record each guard/wiring path and focused command,
     restore one mutation at a time, then rerun the same tests green beside
     valid controls.

  Coverage: AS1 **Flow-value separator padding cannot hide opening syntax**,
  **Implicit ignored block keys obey YAML lookahead**, **Implicit-key lookahead
  includes raw spelling and pre-colon separation**, and **Doctor version and
  composite follow the same selected DSH installation**. Preserve earlier
  ignored-syntax/parent/container, quote/control-byte, typed-scalar,
  Unicode-whitespace and measured-lock proof. No flow-key work, new parser
  finding, YAML dependency or grammar expansion is commissioned.

## 8.8.4. Preserve duplicate-key rejection — retained first-hold finding 4

- [x] 8.8.4.1 Track admitted decoded package headings separately from the
  existing complete-triple set, before exclusions and normalization. In
  producer-facing locks reject identical repetitions, conflicting records
  in both orders, quoted/unquoted equivalent keys, and repeated excluded
  local-tarball keys with `repeated package key` and the decoded key.
  Preserve legitimate equal-complete-triple deduplication from distinct
  records/across npm and pnpm, and retention of differing version/integrity
  triples. A duplicate document yields no readable partial digest. Remove
  the heading-set rejection and observe those named assertions fail; restore
  it and rerun positive and negative controls — safety / AS1: Resume support
  is measured per adapter and execution shape.

Coverage: AS1 **Duplicate decoded pnpm package keys refuse before triple
normalization**. Dependency: 8.8.3's retained scalar/heading decoding.

Extended on return, 2026-09-21 (review of run `124cca78`, R2): the same
singleton rule holds in every mapping scope the grammar admits without
reading — a flow map, an ignored block body at every depth, a package child
spelled twice — by DECODED key and per block, so one key in two sibling
blocks stays two keys. Each scope's guard is proved by its own removal
(D1–D3) in the dated account below; `resolution` keeps its own repeat
refusal. Extended again on the second return (R2, second sitting): keys are
compared as YAML compares them — the padding before a plain key's colon is
the separator's, and a plain key spelling a typed scalar is refused by its
cause before any comparison, in every admitted scope; proved by removals
D4–D5.

## 8.8.5. Preserve the sole-producer expectations — retained first-hold finding 5

- [x] 8.8.5.1 Replace the loop/hash oracle in
  `the_plugin_component_is_bytewise_path_order_and_fails_closed` with a
  literal recorded from the existing sole production producer. Record its
  producing revision and exact synthetic input: each of the six required
  files contains its relative path's bytes with no newline. The source bytes
  and producing call identify provenance; no prose/helper/generator computes
  another component or canonical serialization. Retain per-file input hashes,
  rc.2 measured pins, changed-byte assertions, each required-file removal and
  extra-entry refusals. Alter production path/line ordering in a compiling
  mutation and observe the pinned expectation fail, then restore and pass
  the focused component and canonical suites — safety / AS1: Resume support is
  measured per adapter and execution shape.

- [x] 8.8.5.2 Add D10's focused source-conformance assertion in the existing
  test surface. It detects restoration of the known competing serialization
  and concatenation-hash block without rejecting legitimate per-file hashing.
  Restore that exact test oracle in a compiling mutation: the conformance
  assertion must fail even if runtime digest equality still passes. Restore
  the literal-only expectation and observe the check pass. Record this
  bounded detection claim without inventing a general Rust analysis framework
  or treating two producer calls as a fixed oracle — safety / AS1: Resume
  support is measured per adapter and execution shape.

Coverage: AS1 **Plugin expectations are recorded from the sole producer**.
These first-hold clauses are adopted complete; preserve their literal provenance.

## 8.8.6. Preserve escaped diagnostics — retained first-hold S2

- [x] 8.8.6.1 Apply the existing `Safe` at final rendering of the unavailable
  binary and retained selection cause in `doctor.rs`, leaving lookup/probe
  inputs unescaped. A built-doctor test uses a nonexistent explicit override
  containing a newline and ANSI clear-screen sequence; assert the recognizable
  escaped spelling and meaningful cause, with no raw injected sequence or
  injected line in stdout. Normal report newlines remain valid. Remove safe
  rendering in a compiling control and fail the actual output assertion,
  then restore and pass; a Safe-helper unit test alone is insufficient
  — safety / AS1: Resume support is measured per adapter and execution shape.

Coverage: AS1 **A nonexistent override cannot inject terminal control bytes
through doctor**. Dependency: 8.8.1's unavailable-diagnostic cause carriage.

## 8.8.7. Preserve missing-manifest context — retained first-hold finding 7

- [x] 8.8.7.1 Enrich only exhausted `resolve_bundle` lookup with
  `bundle 'dsh-plugin-cli-session' does not resolve: no package.json found`
  (use the actual bundle name generally). Remove only the installed plugin
  `package.json` in an otherwise complete temporary layout with no later
  copy. Whole-producer and real doctor assertions must name both the bundle
  and `package.json`. Preserve true-absence continuation to a legitimate
  later hit; unreadable/canonicalization-error/outside first hits still stop
  and canonical containment/order stay unchanged. Remove filename context
  in a compiling mutation, observe the producer-facing reason assertion
  fail, restore and rerun the doctor and lookup controls — safety / AS1:
  Resume support is measured per adapter and execution shape.

Coverage: AS1 **Removing only the plugin manifest names the drifted file**.
Adopted complete; frozen extension bytes are never a removal target.

## 8.8.8. Prove the four repairs, validate committed bytes and record the handoff

- [ ] 8.8.8.1 After groups 1–7, consolidate new R1–R4 tests and independent
  removal records from 8.8.1–8.8.3. Record actual test names and nonzero counts;
  proposed extension points are not results. Verify all four built-doctor
  reproductions now end in named refusals before execution beside passing
  controls. Cover protocol producer, doctor callback seam and built binary;
  marker absence alone cannot close silent env dispatch. Verify profile/pnpm
  retain-and-reuse after probe-time replacement, independent home failure via
  `a_failed_home_seam_leaves_the_version_visible_beside_the_reason`, direct-env
  availability and no Node probe after failed DSH probing. Retain original-
  head, Node, hidden-lock and plugin-patch one-read controls; fixed measured
  inputs, containment, closed declaration/carriage, membership and doctor
  dispositions stay green. Audit every mutation restored and frozen/excluded
  bytes unchanged. Adopt earlier E/P/D and cursor proofs without replay. The
  actual absent-PATH retained-Node positive and its two removals remain pending
  under this address until their native prerequisite exists; shell failure
  or NotFound is no positive. Record inherited debts without adding them to
  R1–R4 — safety / AS1: Resume support is measured per adapter and execution
  shape.

- [ ] 8.8.8.2 On the restored candidate run
  `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
  Run `cargo test -p <crate> --all-features --locked` sequentially, in order:
  `brokkr-core`, `brokkr-store`, `brokkr-protocol`, `brokkr-runtime`,
  `brokkr-view`, `brokkr-bridge`, `brokkr-cli`. Every crate must pass; never
  run a boxed workspace test command. For #255 retain the Text-file-busy
  attempt and rerun that crate; its green rerun is green. Other or persistent
  failures remain actual failures, without inherited exemptions or unrelated
  roster/witness edits. Also run
  `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`,
  `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` and
  `openspec validate --all --strict`. Bind commands, target/libc/compiler,
  exit status, counts and failures to candidate revision/source state. Commit
  the restored candidate for 8.8.8.3; later source changes invalidate affected
  evidence and require fresh relevant checks. Unavailable tools or failures
  leave this row pending. Historical 2,193-pass validation is attributed
  preservation evidence, never this repair's result — safety / AS1: Resume
  support is measured per adapter and execution shape.

- [ ] 8.8.8.3 Collect fresh coverage on the committed restored candidate.
  Use unchanged `rust-nightly-version.txt`, a unique empty
  `CARGO_LLVM_COV_TARGET_DIR` and `cargo +<pin> llvm-cov clean --workspace`.
  If that prerequisite cannot execute, stop dependent collection/report steps
  and state all three current pairs unavailable. With boxed tools, instrument
  all seven crates sequentially with all features, `--locked` and branch
  coverage; retain profiles without intervening clean, produce full production
  JSON/LCOV and apply the unchanged script's harness/source-exclusion audit
  and exact integer checks. Record each deviation from the script, boundary-
  test failure and command status. The box cannot execute namespace boundary
  tests; this is preparation, not a literal gate pass. The controller runs
  unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` on a capable host/CI
  and records its committed revision and three numbers. Require nonzero exact
  equality for **source lines, branches and functions**, reporting each
  covered/total pair and preserving reports. Verify CI, release admission and
  script consume the same pin; change no gate, pin, exclusion or denominator.
  Explain denominator changes against chief `124cca78`'s fresh baseline:
  **32148/32324 lines, 5434/5448 branches, 3125/3135 functions** (failed equality).
  Its retained **32324/32324, 5448/5448, 3135/3135** recount is historical, not
  fresh. Older reports retain dates below. No copied profile, rounding, subset
  or hand-transcribed equivalent supplies the gate. Keep this row and final-
  head remote results pending until actual evidence exists — safety / AS1:
  Resume support is measured per adapter and execution shape.

- [ ] 8.8.8.4 Reconcile R1–R4 delivery with committed code, named refusal/
  control/removal/restoration results and fresh gates. Separate delivered
  clauses from inherited native/source-pin/Node debts, capable-host coverage,
  remote CI and excluded #226 work. Tick a task only when its entire acceptance
  is met; broad owners with inherited pending predicates stay open with current
  repair delivery recorded in prose. Preserve change-wide states, unchecked
  8.8 and proposed 0056. Audit staged paths for mutations or frozen/excluded
  changes, commit reviewed work and task account in the repository's message
  style, never push, then verify commit contents and clean status. Bind source
  evidence to that commit; distinguish any later evidence-only commit from the
  instrumented candidate without claiming a fresh execution. Hand concrete
  committed evidence to independent verify/full review; record their actual
  findings separately from this tasks/design work. The operator's one-run
  scope forbids another pnpm repair: any remaining pnpm MEDIUM is a tracked
  finding for the operator's debt ruling, not a reforge instruction or waiver.
  Earlier-artifact faults belong upstream. Do not claim whole-slice completion
  while native-proof, tests or literal coverage acceptance is unmet, or before
  finished tasks are ticked and work committed. A committed partial account
  names exactly what remains unproved. Keep the change active; archive/fold
  remains the final whole-change operation in group 15 — safety / AS1: Resume
  support is measured per adapter and execution shape.

Coverage: AS1 **Digest acceptance is proved by removal without completing the
planner**, **Delivery evidence distinguishes executed positives from pending
platforms**, **Fresh exact coverage cannot be replaced by retained perfect
reports**, **Executable selection and home availability are independent
requirements**, and the five R1–R4 scenarios cited above. Existing absent-PATH
retained-Node acceptance remains under 8.8.8.1. No pending native or remote
result is inferred from a source table, cross-build or old green run.

### Tasks-phase validation — final R1–R4 breakdown, 2026-09-21

This seat delivered only the dependency-ordered task repair over adopted
`136f918305d4f75ccb5f6148527291323c0fa187`. R2 invocation/identity precedes R4
argument admission; R1/R3 share retained pre-probe preparation; named controls,
independent removals, restored validation and the committed handoff follow.
Every local row cites AS1. No Rust, specification, design, decision, extension,
frozen file or checkbox changed, and no implementation/removal is claimed.

Observed validation:

| Check | This seat's result |
|---|---|
| `openspec validate --all --strict` | Exit 0: 15 passed, zero failed. Existing informational archive notices for missing living adapter-resume-safety/sdd-progress-markers targets remain whole-change archive concerns, outside this repair. |
| Numbered tasks and checkbox audit | All 115 numbered IDs/states and all 133 checkbox states match the adopted head; all five completed local task bodies remain byte-identical; 14/14 local rows cite AS1. |
| Capability inventory | Five deltas, 20 requirements, 224 scenarios; unchanged by this seat. |
| `git diff --check` and path scope | Clean; only this tasks artifact changed. |
| Format, clippy, seven crate suites sequentially, both bundle compiles | Each attempt could not start: Cargo is absent (recorded exit 127); zero tests executed. The 11 command records are in `.forge/tasks-final-r1-r4-2fdbcde5/local-checks.json`. |
| Toolchain agreement | CI, release admission and coverage script consume `rust-nightly-version.txt`, unchanged at `nightly-2026-09-05`. |
| Fresh exact coverage | Source lines: **unavailable**; branches: **unavailable**; functions: **unavailable**. Cargo is absent; no fresh instrumented report exists. The box also cannot execute boundary namespace tests. Capable-host literal gate and its three numbers remain controller evidence, not a retained-report substitution. |

All nine pending local rows remain open, including their inherited evidence
debts. Implementation, new regression/removal results, fresh Rust checks,
capable-host coverage and independent verify/review remain unproved. There is
no upstream behavioral/design gap preventing this breakdown. This checkpoint
records `drafted`, never implementation completion or permission to expand the
operator's four-finding scope. The change remains active and 0056 proposed.

### Implement — the final R1–R4 repair delivered, 2026-09-21

This implement seat (run `dsh-composite-identity-issue-226-e291e076`, no
`returned_from`) adopted `slice-dsh-composite-b` at
`88385af3`, clean at entry, every commit kept. It delivers exactly chief
`124cca78`'s R1–R4, proves each changed rule by removal, runs the gates and
records what remains. Environment: Linux x86_64 glibc host whose `/usr/bin/env`
is uutils; `cargo 1.98.0 (797e8a9bc 2026-08-05)` for fmt/clippy/tests/bundles
and `nightly-2026-09-05` with `cargo-llvm-cov 0.9.0` for coverage. Evidence
logs are under `.forge/impl-r1r4/`.

Commits, all signed, none pushed: **`336598b9`** (the repair, its tests and the
guide), **`aa1dbbb5`** (three coverage gaps the repair's own new code opened),
**`b2f723e1`** (built-doctor test rows only). `b2f723e1` is the code candidate
every number below binds to; the commit carrying this account changes no Rust.

**Delivered, by finding.**

- **R1.** `split_flow_entry` consumes the whole run of separator spaces before
  `flow_scalar` judges a value's opening, and an indicator opening is named
  (`…whose value opens with the YAML indicator '*'`). Tabs keep the document's
  own refusal; quoted values keep their bytes; an unclosed quoted member stays
  its own fault.
- **R3.** `pnpm_ignored_line` bounds the raw implicit-key span — quotes and
  pre-colon padding included — at 1,024 characters (`IMPLICIT_KEY_LOOKAHEAD`)
  before trimming or decoding. No line, value or file bound moved.
- **Shared preparation (R1/R3 wiring).** `DshPrepared` is constructed only by
  admission of the located profile and pnpm lock, holds the invocation
  privately and retains the profile declarations, anchor, boundary and admitted
  dependencies; `dsh_composite_prepared` composes from them without reopening
  either file. `DshUnprepared::Refused` (a located lock that fails admission)
  carries no invocation, so doctor reports `binary '…' selected and not probed`
  beside the refusal and declaration context, with no version and no recorded
  availability. `Unlocated` (home, profile or lock not found) keeps the
  version-only line. The standalone `dsh_composite` admits before its Node
  probe; `DshSeams::resolve` admits nothing, so the planner path reads once.
- **R2.** `DshInvocation` (candidate path, native `argv[0]`) rides beside the
  canonical identity through `Selected`, `DshNode` and the selection; doctor's
  `invocation_version` and the Node probe launch it. `env_dispatch` is factored
  out of `env_program` and asked of the selected executable in `classify_in`:
  searched and absolute `dsh -> /usr/bin/env` refuse at selection. Direct
  `/usr/bin/env` stays the availability control.
- **R4.** An established env with an empty or blank argument tail is a refusal
  naming launcher, interpreter and missing program; the admitted bare-env row
  in `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`
  is flipped beside four blank variants, with `env sh` the admitted control.

**Tests (executed, nonzero).** Protocol, new:
`the_selected_invocation_is_not_replaced_by_its_canonical_target`,
`the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained`;
extended: `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
(R1 3 openings × 4 paddings, R3 13-row span matrix, long-value controls), the
classifier table (R4) and the seams test (invocation carriage, no-home arm).
Doctor unit, new: `a_refused_pnpm_lock_stops_the_line_before_either_probe`
(probe/producer call counts), `a_selected_and_unprobed_dsh_is_not_reported_missing`,
`the_dsh_probe_runs_the_selected_invocation_and_not_its_canonical_target`;
`the_guide_documents_the_wording_the_classifier_emits` pins the two new guide
samples. Built doctor, new:
`a_dsh_alias_of_env_is_refused_and_an_admitted_alias_runs_as_selected`,
`an_env_launcher_without_a_program_is_refused_before_any_probe`; extended:
`ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`, whose
`node` shim now marks DSH and Node probes separately — every refusal row
asserts NO marker before the prose, every control asserts both.

**Native evidence, recorded and never counted** (`native-evidence.log`): both
R2 alias forms exit 1 with empty stdout (uutils name mismatch); bare and blank
R4 launchers did not exit within the 2-second process-group deadline and were
killed and reaped; `env sh` terminated with its version. Doctor returned the
named refusal for all four reproductions with zero markers; a silent env
leaves no marker, so R2/R4's no-probe half rests on the selection holding no
invocation (`DshUnselected`) and the panicking-probe callback test.

**Removals** — each one compiling mutation on the candidate, a nonzero focused
run failing at the intended assertion, restored before the next; the restored
files compared byte-identical to the pre-mutation snapshot (`cmp`). Run before
rustfmt and before `aa1dbbb5`, which touches none of the mutated lines.

| # | Mutation | Failing test and assertion |
|---|---|---|
| M1 (R1) | one-space consumption restored | producer: `{node:  *missing}` "was accepted"; built doctor: markers `["dsh","node"]` ≠ `[]`, line showing control composite `f742ba0e…` |
| M2 (R3) | raw-span guard disabled | producer: "expected a refusal" at the 1,025 row; built doctor: markers ≠ `[]` for the lookahead reason |
| M3 (R3) | guard moved after trimming | same two tests, failing at the padded-over-limit row (the plain 1,025 row still refuses) |
| M4 (R4) | blank tail `Ok(None)` restored | classifier: "expected a refusal" for the bare-env row; nothing spawned |
| M5 (R2) | selected-env qualification removed | protocol: "expected a refusal"; built doctor: line lacks the pre-probe cause |
| M6 (R2) | canonical path substituted as invocation | protocol: `DshInvocation` inequality; built doctor: version line prints `lib/launcher.sh` |
| M7 (wiring) | refused lock handed an invocation | doctor unit: call counts `(1, 0)` ≠ `(0, 0)`; built doctor: markers `["dsh"]` ≠ `[]` |
| M8 (wiring) | Node probe moved ahead of admission | protocol: Node probe count `1` ≠ `0` |
| M9 (wiring) | composition reopens its inputs | protocol: retain-and-reuse composite inequality |

**Gates on the candidate.**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| Seven crate suites, sequential, `--all-features --locked` | core 86, store 64, protocol 480 (2 ignored), runtime 534, view 243 (3 ignored), bridge 13, cli 780: **2,200 passed, 0 failed** on `336598b9`; protocol and the cli unit binary rerun green after `aa1dbbb5`, the built-doctor binary after `b2f723e1`, and all seven again under instrumentation on `b2f723e1` |
| #255 | one instrumented protocol attempt on `b2f723e1` failed 12: root `dsh_unsafe_stored_candidates_decline_instead_of_being_skipped` (a just-written version shim, injected composite, no code of this repair) plus 11 `PoisonError` siblings; attempt retained (`cov3-brokkr-protocol.log`), crate rerun green 380 + 99 (`cov3b-…`) |
| `compile --bundle bundles/self`, `bundles/verify` | both exit 0 |
| `openspec validate --all --strict` | **not executed**: the command is refused to this seat by permission. Pending; this account changes prose and one checkbox only |
| Frozen/excluded paths, manifests, pin | `contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`, `Cargo.toml`, `Cargo.lock`, `rust-nightly-version.txt`, `scripts/` byte-identical to `81d7fe87`; CI, release admission and the script read the one pin |

**Coverage on committed `b2f723e1`: source lines 32507/32507, branches
5466/5466, functions 3159/3159** — nonzero exact equality, harness-leak audit
`true`, no `coverage(off)`. Against chief `124cca78`'s fresh baseline
(32148/32324, 5434/5448, 3125/3135) the denominators grew by 183 lines, 18
branches and 24 functions, all of it this repair's production code in
`composite.rs` and `doctor.rs`; the first instrumented pass on `336598b9`
read 32507/32508, 5466/5468, 3158/3158, and `aa1dbbb5` answers those three
misses (one unreachable match guard removed, two arms driven). No namespace
boundary test skipped in these runs. This is a hand-executed equivalent and
NOT the literal gate; the deviations: the seat cannot run the script (command
substitution, `mktemp`, `awk` and env-prefixed commands are refused), so the
default `target/llvm-cov-target` was deleted and `llvm-cov clean --workspace`
run in place of a unique `CARGO_LLVM_COV_TARGET_DIR`; the seven crates were
instrumented sequentially with `--no-report --all-features --locked --branch`;
and the script's awk count was re-expressed in `jq` with the same first-comma
and file-plus-start-line rules. The unchanged
`TMPDIR=/tmp bash scripts/coverage-exact.sh` on a capable host, and its three
numbers, remain the controller's.

**States.** 8.8.3.1 is ticked: its whole acceptance is met and it carries no
inherited predicate. Local rows are now **6 complete / 8 pending**; change-wide
addresses **85 / 16**; numbered rows **90 / 25**; checkbox rows **106 / 27** —
superseding the tasks-phase counts above. 8.8.1.1–8.8.2.2 have their current
repair clauses delivered and stay open for their inherited Apple/env source
pins and native macOS/Windows proof; 8.8.8.1 for the absent-PATH retained-Node
positive and its removals; 8.8.8.2 for strict OpenSpec validation; 8.8.8.3 for
the capable-host literal gate; 8.8.8.4 for independent verify/review and
final-head remote CI. **8.8 stays unchecked.** Part (d), 8.10, 9.6, 10.6–10.8,
11.1–11.4 and groups 14–15 are untouched; 0056 stays proposed. One adjacent
fact is recorded and not repaired, being outside R1–R4: a QUOTED implicit key
followed by padding before its colon (`'k' : v`) is refused by the existing
closed grammar as "not a mapping entry", stricter than YAML. Any pnpm MEDIUM a
council still finds is the operator's debt ruling.

### Returned implement — the chief's three mediums on the R1–R4 repair, 2026-09-21

This implement seat (run `dsh-composite-identity-issue-226-e291e076`,
`returned_from` review, chief on `46142902`) adopted `slice-dsh-composite-b`
at `46142902`, clean at entry, every commit kept. It answers the chief's three
MEDIUM findings, all inside R1–R4, and nothing else. Environment as the
delivery above: Linux x86_64 glibc, uutils `/usr/bin/env`,
`cargo 1.98.0 (797e8a9bc 2026-08-05)`, `nightly-2026-09-05` with
`cargo-llvm-cov` for coverage. Logs are under `.forge/impl-r1r4-return/`.

One signed commit, not pushed: **`69fc8670`** — the code candidate every
number below binds to. The commit carrying this account changes no Rust.

**Delivered, by finding.**

- **Item 1, R3 incomplete (security).** The lookahead bound guarded
  `pnpm_ignored_line` alone. The `packages:` heading trimmed its pre-colon
  padding before reading the scalar, so `debug@2.6.9` and 1,014 spaces — a
  1,025-character span — kept the control composite with both probes run. The
  bound is now one function, `implicit_key_lookahead`, asked of the RAW span
  at both routes that admit a key of the document's choosing: the ignored
  bodies (unchanged behaviour) and the heading, before its trim and decode.
  The heading's refusal is `a package key that is an implicit key past YAML's
  implicit-key lookahead limit of 1,024 characters`. The two remaining key
  sites — top-level keys and package children — compare the whole spelled key
  against closed lists of short names, so an over-limit or padded spelling of
  one is already a named refusal; they are stated in the function's comment,
  not changed. No line, value or file bound moved.
- **Item 2, Windows test build.** The dangling-lock subcase of
  `the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained` is
  `#[cfg(unix)]`, and asserts its cause (`<lock>: <ENOENT>` as
  `DshUnprepared::Refused`) where it held a bare variant match. The admission
  and retention assertions around it run on every host.
  `cargo check -p brokkr-protocol --tests --all-features --locked --offline
  --target x86_64-pc-windows-msvc` — the chief's E0433 reproduction — now
  finishes; the same check of `brokkr-cli` cannot run here (`ring` and
  `libsqlite3-sys` build scripts need a Windows C toolchain) and its
  built-doctor test file is unix-only as before.
- **Item 3, env without `--version`.** Neither direct-env control requires
  `/usr/bin/env --version` to succeed. The producer test compares the selected
  invocation's status, stdout and stderr with native's under the same PATH.
  The built-doctor test reads native's answer first: where env prints a
  version the line is `ok dsh: <version>` with an unreadable composite, as
  before; where it does not, the line is `warn dsh: binary '/usr/bin/env' not
  found — ` with no selection cause, and the alias loop's
  env-version-absence assertion applies only where a version exists. The
  failed-probe arm is NOT executed on this host, whose env answers; whether
  Apple's env is even established as direct env by the resolver is native
  macOS evidence this seat does not have. Both stay pending with the inherited
  macOS proof.

**Tests (executed, nonzero).** Producer:
`missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
gains an 11-row heading matrix — plain, single- and double-quoted
`debug@2.6.9` padded to 1,024 (control composite) and 1,025 (refused), a
4,096 span, an unpadded long-version key at 1,024/1,025 plain and quoted — and
a control with 64 spaces AFTER the colon, which are not the key's span.
`the_pnpm_lock_is_admitted_before_any_probe_and_composed_as_retained` gains
the chief's reproduction as a third located refusal: `DshUnprepared::Refused`
by cause, zero Node probes, a selection holding nothing to probe. Built
doctor: `ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`
gains the plain and quoted 1,025 headings as refusal rows (markers `[]`
asserted before the prose, named reason, no control digest) and the 1,024
headings as controls (control digest, both probes).

**Removals** — each a compiling mutation on the candidate, a focused nonzero
run, restored before the next; `git diff --stat` after restoration showed the
pre-mutation change set and the focused tests reran green.

| # | Mutation | Failing test and assertion |
|---|---|---|
| N1 (R3 heading) | heading `implicit_key_lookahead` call removed | producer matrix: `expected a refusal` (tests.rs:116) at the first 1,025 row; admission test: `unwrap_err()` on an `Ok` `DshPrepared` holding `debug 2.6.9 sha512-DEBUG`; built doctor: markers `["dsh", "node"]` ≠ `[]` with the line `ok dsh: v22.23.2 … composite f742ba0e…` — the chief's reproduction, exactly |
| N2 (R3 body) | ignored-body call disabled, re-proving M2 after the refactor | the same two producer tests fail at the `peerDependencies` rows |

**Gates on committed `69fc8670`.**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| Seven crate suites, sequential, `--all-features --locked` | core 86, store 64, protocol 480 (2 ignored), runtime 534, view 243 (3 ignored), bridge 13, cli 780: **2,200 passed, 0 failed** |
| #255 | the first protocol attempt failed one test, `a_qualified_stream_json_launch_finishes_its_held_row_without_a_confirmation` (`adapters/tests.rs:5026`, the `unwrap` of a launch that spawns a just-written shim; no code of this repair). Its cause line was not retained. It passed alone and the full crate rerun is green 380 + 99 + 1 (`protocol-rerun.log`); the instrumented protocol run was green first time |
| `compile --bundle bundles/self`, `bundles/verify` | both exit 0 |
| `openspec validate --all --strict` | **not executed**: refused to this seat by permission, as before. Pending; this visit changes no spec delta, and this account is prose |
| Frozen/excluded paths, manifests, pin | `contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`, `Cargo.toml`, `Cargo.lock`, `rust-nightly-version.txt`, `scripts/` show no diff from `81d7fe87` |

**Coverage on committed `69fc8670`: source lines 32512/32512 (100%), branches
5464/5464 (100%), functions 3161/3161 (100%)** — nonzero exact equality,
harness-leak audit `true`, no `coverage(off)`. Against `b2f723e1`
(32507 / 5466 / 3159): +5 lines and +2 functions are `implicit_key_lookahead`
and the heading's `map_err` closure; −2 branches is the inline
`if …is_some()` becoming a `match` inside the shared function. This is again a
hand-executed equivalent and NOT the literal gate:
`bash scripts/coverage-exact.sh` is refused to the seat, so
`target/llvm-cov-target` was deleted and `llvm-cov clean --workspace` run in
place of a unique target directory, the seven crates were instrumented
sequentially with `--no-report --all-features --locked --branch`, the JSON and
LCOV reports were written to `target/coverage/`, and the script's awk count
was re-expressed in `jq` with the same first-comma and file-plus-start-line
rules. This box cannot create a namespace, so it cannot execute the boundary
proof; the unchanged script on a capable host, and its three numbers on these
bytes, remain the controller's.

**States.** No checkbox moves. 8.8.3.1 stays ticked: the chief showed its R3
clause was incomplete while ticked, and the heading route is what it lacked.
**8.8 stays unchecked.** Pending, unchanged in kind: native macOS proof
(now including the failed-probe arm above), strict OpenSpec validation, the
capable-host literal coverage gate, independent verify/review and final-head
remote CI. Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 are
untouched; 0056 stays proposed. Two adjacent facts are recorded and not
repaired, being outside R1–R4. First, flow-map keys — `engines: {…}` and the
`resolution` map — carry no lookahead bound here: YAML 1.2.2 bounds implicit
keys of block mappings and of single-pair flow-sequence entries (productions
154–155), not flow-mapping entries (§7.4.2), and this reader already refuses a
pair inside a flow sequence. A YAML 1.1-era scanner that applies its
simple-key limit in flow context as well may refuse a 1,025-character flow key
this reader admits; that was reasoned from the grammar and NOT executed,
because the seat is refused a Python interpreter. Second, the quoted-key
padding strictness recorded above stands. Any pnpm MEDIUM a council still
finds is the operator's debt ruling.

### Returned implement — review R1–R4 of the delivered port, 2026-09-21

This implement seat (run `dsh-composite-identity-issue-226-124cca78`, the
`REVIEW-REFORGE` return from the full council's chief, `gpt-6-astra` at
`xhigh`, reviewing `d120dd91`) adopted `slice-dsh-composite-b` at
`d120dd9190005614112f5e097f449d35a5b1070a`, clean at entry, every commit
kept. It fixes exactly the council's findings, proves each changed rule by
removal, reruns the gates and records what remains. Environment: Linux
x86_64 glibc host, unboxed; `cargo 1.98.0 (797e8a9bc 2026-08-05)` for
every gate, `nightly-2026-09-05` for the coverage steps; `/usr/bin/env`
resolves to `/usr/lib/cargo/bin/coreutils/env` (uutils installed as `env`)
and `/usr/bin/busybox` is present. Notes describe; they direct no gate.

**R1 (MEDIUM, security) — the other-name `env` invocation is refused, not
established.** The chief copied busybox to a file named `env`, hard-linked
it as `uu_env`, and the delivered `is_env`/`env_dispatch`/`env_invocation`
answered `Ok(true)`/`Named`/`Ok(())` while the layout natively exited 127,
`uu_env: applet not found`. The prefixed-name rule inferred WHICH utility is
installed as `env` from the reference's file name; the file does not carry
that fact, and uutils (runs `env` under `uu_env`), busybox and GNU's
single-binary `coreutils` (no applet under it) and a dedicated GNU/Apple
`env` (any name) are indistinguishable by device, inode, length, bytes or
name. `EnvDispatch`, `env_dispatch` and `env_invocation` are removed;
`env_program`'s same-file/other-name arm is one refusal — `is the
platform's env utility invoked under the name '<spelled>', a dispatch this
resolver does not establish without executing it` — and never `Ok(None)`.
The fourth hold's R8 positive (same-file `uu_env` reaching B on a uutils
host) is withdrawn as a requirement and kept as a recorded native fact:
fail-closed loading (decision 0004) rules over one host's success, and
trial execution stays forbidden. Native evidence, this host
(`an_env_argument_is_selected_as_the_kernel_hands_it_to_env`, protocol and
doctor, every spelling with its own native child and marker directory):

| Spelling | Native child (obstructed A/node; valid chain) | Resolver / doctor |
|---|---|---|
| `tools/uu_env`, hard link of the copy of `/usr/bin/env` (uutils) | ran B's node both times (`MARK:b-node`; doctor markers `["DSH_B_NODE_0.0.3"]`) | refused as unestablished; zero doctor markers, both times — recorded, not counted |
| `tools/env-alias`, `tools/link_env` symlinks; `tools/myenv` hard link | the utility's own refusals as before (security violation; argument dispatch) | refused as unestablished; zero markers |
| `tools/bb/env`, a stand-in INSTALLED AS `env` dispatching on `$0`'s basename (`env` → the platform's env, anything else → `applet … not found`, exit 127), injected as the reference | `bb/env`: A's obstruction natively, then `MARK:b-node`; `bb/uu_env` (hard link): exit 32512, stdout empty, `multicall: applet uu_env not found`, both times | `bb/env`: A's obstruction, then B's node selected and retained; `bb/uu_env`: refused as unestablished, both times |
| `tools/busybox/env`, the real busybox copied to `env`; `tools/busybox/uu_env`, its hard link (the chief's layout, run where `/usr/bin/busybox` exists; recorded `PENDING` otherwise) | `env /bin/echo ENV_SELECTED` → `ENV_SELECTED`, exit 0; `uu_env /bin/echo ENV_SELECTED` → exit 32512, stdout empty, `uu_env: applet not found`; through a launcher, `env` reached `MARK:b-node` and `uu_env` ran nothing | `env`: B's node selected and retained; `uu_env`: refused as unestablished |
| `/usr/bin/env`, `linked/env` symlink, `tools/env` copy; `impostor/env` | unchanged | unchanged (A's obstruction then B; refused as not the platform's env) |

**R2 (MEDIUM, security) — a repeated decoded key refuses in every admitted
mapping scope.** Three guards, each named by its scope: a flow map's keys
(`pnpm_ignored`: `the flow map '{node: '>=18', node: '>=20'}' with the
repeated key 'node'`; `'node'` and `node` are one decoded key), an ignored
block's keys (`Frame.keys`, the block's own set, checked in
`IgnoredBody::admit`: `the entry 'react' at 6 spaces, which repeats a key
of its block`, likewise `ms` in a `snapshots` body, a `snapshots` record
heading at 2 spaces, `autoInstallPeers` in `settings`), and a record's
children (`seen_children`, reset per heading: `a repeated package child
'cpu'`, `'peerDependencies'`); `resolution` keeps its own repeat refusal.
Nine producer vectors in
`missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
and the chief's four through the built doctor in
`ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`, each
asserted never to report the control's digest. Controls kept readable: one
key in two sibling blocks — `dependencies` under two importers, `cpu` in two
records, `optional` under two `peerDependenciesMeta` children — at the
digest of the same records without the ignored bodies (producer) and the
control's digest (doctor). No YAML dependency, grammar expansion, bound or
identity change.

**R3 (LOW)** — the 2026-09-21 gate row said `doctor_dsh_selection` (6); its
own log `.forge/v88b-test-cli.log:606-624` says `running 14 tests` /
`14 passed`. Corrected in place, with the correction named. **R4** — the
chief rejected the adversarial's closing instruction as untrusted panel
prose; nothing in it directed this seat, and this account directs nothing.

**Removal proofs** (each a compiling mutation of the enforcing line; the
named regression run with `cargo test -p brokkr-protocol --all-features
--locked --lib -- <name> --exact` and `cargo test -p brokkr-cli
--all-features --locked --test doctor_dsh_selection -- <name> --exact`; the
failed assertion quoted; exact restoration; both green again, 1 passed
each, before the next):

| Removal | Named regression → failed assertion |
|---|---|
| E5 same-file admission under another name restored (`env_program`, `(true, false) => {}`) | protocol `an_env_argument…` at `tests.rs:5532`: `tools/uu_env: no spelling bypasses D10` — left A's obstruction (the chain FOLLOWED through `uu_env`), right the unestablished refusal; doctor `an_env_argument…` at `doctor_dsh_selection.rs:1099`: the `uu_env` line carries A's obstruction, not the refusal (markers still zero only because A's node is obstructed). The `bb/uu_env` and busybox cells sit downstream of that first failure in the same test and admit under the same removal. |
| D1 flow-map key guard disabled (`!keys.insert(key.text()) && false`) | protocol `missing_pnpm…` at `tests.rs:111`: `…engines: {node: '>=18', node: '>=20'}\n" was accepted`; doctor `ignored_pnpm…` at `doctor_dsh_selection.rs:2397`: that lock reported `ok dsh … composite f742ba0e…` — the control's digest |
| D2 block key guard disabled (`key.is_some_and(…) && false`) | protocol: `…peerDependencies:\n      react: '>=16'\n      react: '>=17'\n" was accepted`; doctor: the same lock reported the control's composite |
| D3 package-child guard disabled (`!seen_children.insert(name) && false`) | protocol: `…cpu: [x64]\n    cpu: [arm64]\n" was accepted`; doctor: `a repeated package child 'cpu'` lock reported the control's composite |

`grep -rn REMOVAL crates/` is empty and `git diff --check` clean after the
last restoration. E1–E4 stand as history of the rule they proved; E1 and
E4 proved a rule this return removes, and E5 is their successor.

**Gates on the restored candidate.**

| Check | Actual outcome |
|---|---|
| `cargo fmt --all -- --check` | clean, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, exit 0 (`.forge/scratch/r124-clippy.log`) |
| `cargo test -p brokkr-core --all-features --locked` | ok: 86 passed, 0 failed |
| `cargo test -p brokkr-store --all-features --locked` | ok: 64 passed, 0 failed |
| `cargo test -p brokkr-protocol --all-features --locked` | first attempt **FAILED 316 / 61** — one `adapters::tests::a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled` panic, `could not invoke the agent CLI: Text file busy (os error 26)` (`adapters/tests.rs:2678`), then 60 sibling `adapters::tests` failures as `PoisonError` on the lock that panic poisoned — the #255 shape, reproduced UNBOXED; rerun of the same crate suite: **ok: 377 lib + 99 integration (2 ignored) + 1 doctest = 477 passed, 0 failed**, no `Text file busy` (`.forge/scratch/r124-test-protocol.log`, `…-rerun.log`) |
| `cargo test -p brokkr-runtime --all-features --locked` | ok: 534 passed, 0 failed |
| `cargo test -p brokkr-view --all-features --locked` | ok: 243 passed (3 ignored), 0 failed |
| `cargo test -p brokkr-bridge --all-features --locked` | ok: 13 passed, 0 failed |
| `cargo test -p brokkr-cli --all-features --locked` | ok: 463 lib + 312 across 31 binaries (`doctor_dsh_selection` 14) = 775 passed, 0 failed, no hang |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` / `bundles/verify` | both compile, exit 0 |
| `openspec validate --all --strict` | **NOT RUN from this seat**: refused (approval required) under the bare spelling, as for every prior seat. The last strict run on this change was the tasks seat's **15 / 0** on `c3f36568`; this return edits `tasks.md`, `design.md` and the AS1 delta's prose and existing scenarios only, adding no requirement, scenario or file. Pending on a seat that can launch it. |
| `bash scripts/coverage-exact.sh` | **NOT RUN literally**: refused. Its three cargo steps by hand on the pinned nightly, in order, into `.forge/scratch/r124-coverage/` (`llvm-cov clean --workspace`; `llvm-cov --workspace --all-features --locked --branch --json`, **68 binaries `test result: ok`, 0 failed, 0 `Text file busy`**, no `--ignore-run-fail`; `llvm-cov report --branch --lcov`); the script's `jq` harness-leak check on the JSON → `true`, **0 harness files among 53 `SF` records**; its LCOV rule through the retained transcription `.forge/lcovtool` (`cargo run --release --manifest-path …`). |

| Pass | Lines | Branches | Functions | Misses |
|------|-------|----------|-----------|--------|
| Fresh, on this return's bytes | **32,300 / 32,300 (100%)** | **5,440 / 5,440 (100%)** | **3,132 / 3,132 (100%)** | none |

The denominator moved from `25b40967`'s 32,322 / 5,440 / 3,139 by −22
lines and −7 functions: `env_dispatch`, `env_invocation`, `EnvDispatch` and
its derived `Debug`/`Clone`/`PartialEq` are gone, and the three key guards
and the one-arm refusal are added; every remaining record is hit. The
literal script, CI's `coverage-exact` job and final-head remote CI remain
the gate's own artifacts and are pending until they run.

**Dependent artifacts kept coherent.** AS1's requirement text now states
the `env`-name-only rule; the scenario *Interpreter aliases cannot bypass
the Node obstruction refusal* carries the answered ambiguity (the same-file
other-name spellings are refused, native outcomes recorded, the stand-in
and busybox counterexamples, E5) and *Duplicate decoded pnpm package keys
refuse before triple normalization* carries R2's scopes and controls; the
five deltas still hold **20 requirements / 219 scenarios**. Design D10 §4
records both answers as dated paragraphs superseding the prefixed-name rule
(the reconciliation rows above them are history). `## Decisions`, 8.8.2.1
and 8.8.4 in this file carry the same two rules. No checkbox changed: the
fourteen local addresses stay **5 complete / 9 pending**, the 101
change-wide identifiers **84 / 17**, the 133 rows **105 / 28**.

**Still pending, recorded and not claimed.** **8.8 stays unchecked**; part
**(d)**, **8.10**, **9.6**, 10.6–10.8, 11.1–11.4 and groups 14–15 untouched;
native **macOS/Windows** execution (matrix, doctor, `GetBinaryTypeW`,
Windows 1.88 MSRV); the immutable Apple and env source pins (this seat, like
the last, could fetch none: the source rules above are cited by function);
the absent-PATH Node positive and its two removals (no `node` on
`/bin:/usr/bin`); strict OpenSpec on this head; the literal coverage script;
final-head remote CI; independent verify and the council's second sitting on
this return. No `wrapper_digest` is declared, the DSH route stays disabled,
**0056 stays proposed**, and `contracts/`, `policy/phase-machine.json`,
`policy/schemas/`, `fixtures/`, `reference/`, `extensions/dsh/` and
`docs/decisions/` have no diff (`git diff --stat` over them is empty). The
active change is not archived. Nothing was pushed.

### Returned implement — the council's second sitting, R1/R2, 2026-09-21

This implement seat (run `dsh-composite-identity-issue-226-124cca78`, the
`REVIEW-REFORGE` return from the full council's chief, `gpt-6-astra` at
`xhigh`, reviewing `3a1df8d3`) adopted `slice-dsh-composite-b` at
`3a1df8d341468f9fea01f6c8047af7cb251f1120`, clean at entry, every commit
kept. It fixes exactly the council's two findings, proves each changed rule
by removal, reruns the gates and records what remains. Environment: Linux
x86_64 glibc host, unboxed; `cargo 1.98.0 (797e8a9bc 2026-08-05)` for every
gate, `nightly-2026-09-05` for the coverage steps; `/usr/bin/env` resolves
to `/usr/lib/cargo/bin/coreutils/env` (uutils installed as `env`) and
`/usr/bin/busybox` is present. Notes describe; they direct no gate.

**R1 (MEDIUM, security) — the name `env` is asked of the file that runs.**
The chief symlinked `links/env` to a copy of this host's `env` named
`tools/uu_env` (and `tools/ls`): spelled `env`, the platform's env by every
byte, and natively exit 1 with no stdout — uutils' `Security violation:
Requested utility `env` does not match executable name: …/tools/uu_env`,
the check of `argv[0]` against `/proc/self/exe`'s own name — while the
delivered `env_program` established it and doctor went on to its version
probe. `env_program` now resolves the interpreter to the file that runs
(`canonicalize`) and establishes the invocation only where that file is
itself named `env` or is the path the platform's reference resolves to;
otherwise one named refusal (`is the platform's env utility invoked under
the name 'env' but running as the file '<runs>', whose own name is not env,
a dispatch this resolver does not establish without executing it`), and an
interpreter whose path no longer resolves is refused by that cause on its
inspected metadata (`an_env_invocation_needs_the_file_that_runs`). Supported
platform-env links are preserved. Native evidence, this host (protocol and
doctor `an_env_argument_is_selected_as_the_kernel_hands_it_to_env`, every
spelling with its own native child and marker directory):

| Spelling | Native child (obstructed A/node; valid chain) | Resolver / doctor |
|---|---|---|
| `renamed/env -> tools/uu_env`, `renamed-ls/env -> tools/ls` (symlinks named `env` to the copy's hard links) | exit 1, stdout empty, `Security violation: Requested utility `env` does not match executable name: …/tools/uu_env` (resp. `…/tools/ls`), both times; doctor markers `[]` | refused naming the file that runs; zero doctor markers, both times |
| `viacopy/env -> tools/env` (symlink named `env` to the copy named `env`); `hard/env` (hard link named `env` of the copy) | `MARK:b-node` / `DSH_B_NODE_0.0.3` both times | A's obstruction, then B's node selected and retained; doctor `ok dsh: DSH_B_NODE_0.0.3`, one marker |
| `tools/bb-link/env -> bb/uu_env` (symlink named `env` to the installed-as-`env` stand-in's hard link, stand-in injected as the reference) | `MARK:b-node` — the stand-in dispatches on the name invoked | refused naming the file that runs: the implementations disagree, the native positive is recorded and not counted |
| `bbm/env -> bb/multicall` (the injected reference itself resolves to `multicall`; a symlink named `env` to it) | `MARK:b-node` | established: B's node selected and retained |
| `/usr/bin/env`, `linked/env`, `tools/env`; `uu_env`, `env-alias`, `link_env`, `myenv`; `impostor/env`; `bb/env`, `bb/uu_env`; busybox `env`/`uu_env` | unchanged | unchanged |

**R2 (MEDIUM, security) — keys compared as YAML compares them.** The
chief's built-doctor reproductions: `react: a` / `react : b`, `engines:
{11: 1, 0xB: 2}`, `true: a` / `True: b` and `settings` `null: a` / `~: b`
each reported the control's composite `f742ba0e…` where the independent
YAML parser refuses `DUPLICATE_KEY`; `{true: a, 'true': b}` was refused as
a repeat where it is a boolean beside a string. One decoder, `mapping_key`,
now serves the flow-map guard and the ignored-body guard: a plain key that
spells a typed scalar (`typed_plain_scalar`, the `integrity` field's own
closed lexical rule) is refused by its cause before any comparison (`the
flow map '{11: 1, 0xB: 2}' with the key '11', which is a number and not a
string`; `the key 'true', which is a boolean and not a string`; `the key
'null', which is a null and not a string`), and `pnpm_ignored_line` trims
the padding before a plain key's colon as the separator's before decoding
(`react :` beside `react:` → `the entry 'react' at 6 spaces, which repeats
a key of its block`). No YAML dependency, grammar expansion, bound or
identity change; the producer this grammar reads quotes any key that would
resolve to another type. Six producer vectors in
`missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
and the chief's five through the built doctor in
`ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`, each
asserted never to report the control's digest. Controls kept readable at
the control's digest in both: plain `react` beside quoted `'react '`, a
single padded `react :`, `{'true': a, 'True': b}` and `{'11': 1, '0xB': 2}`.

**Removal proofs** (each a compiling mutation of the enforcing line; the
named regression run with `cargo test -p brokkr-protocol --all-features
--locked --lib -- <path> --exact` and `cargo test -p brokkr-cli
--all-features --locked --test doctor_dsh_selection -- <name> --exact`; the
failed assertion quoted; exact restoration; both green again, 1 passed
each, before the next):

| Removal | Named regression → failed assertion |
|---|---|
| E6 admission on the invoked name alone (`env_program`, the file-that-runs condition prefixed `false &&`) | protocol `an_env_argument…` at `tests.rs:5632`: `…/renamed/env: no spelling bypasses D10` — left A's obstruction (the chain FOLLOWED through the renamed symlink), right the file-that-runs refusal; doctor `an_env_argument…` at `doctor_dsh_selection.rs:1127`: the `renamed/env` line carries A's obstruction, not the refusal |
| D4 typed-key refusal disabled (`mapping_key`, `typed_plain_scalar(text).filter(\|_\| false)`) | protocol `missing_pnpm…` at `tests.rs:111`: `…engines: {11: 1, 0xB: 2}\n" was accepted`; doctor `ignored_pnpm…` at `doctor_dsh_selection.rs:2466`: that lock reported `ok dsh … composite f742ba0e…` — the control's digest |
| D5 separator trim removed (`pnpm_ignored_line`, `let key = key;`) | protocol: `…peerDependencies:\n      react: '>=16'\n      react : '>=17'\n" was accepted`; doctor: the padded-`react` lock reported the control's composite |

`grep -rn REMOVAL crates/` is empty and `git diff --check` clean after the
last restoration; E5 and D1–D3 stand as the history of the rules they
proved, which this return refines rather than removes.

**Gates on the restored candidate.**

| Check | Actual outcome |
|---|---|
| `cargo fmt --all -- --check` | clean, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, exit 0 (`.forge/scratch/r124b-clippy.log`) |
| `cargo test -p brokkr-core --all-features --locked` | ok: 73 lib + 13 across 3 binaries = 86 passed, 0 failed |
| `cargo test -p brokkr-store --all-features --locked` | ok: 58 lib + 6 across 5 binaries = 64 passed, 0 failed |
| `cargo test -p brokkr-protocol --all-features --locked` | ok: 378 lib + 99 integration (2 ignored) + 1 doctest = 478 passed, 0 failed; one attempt, no `Text file busy` |
| `cargo test -p brokkr-runtime --all-features --locked` | ok: 441 lib + 93 across 22 binaries = 534 passed, 0 failed |
| `cargo test -p brokkr-view --all-features --locked` | ok: 243 passed (3 ignored), 0 failed |
| `cargo test -p brokkr-bridge --all-features --locked` | ok: 13 passed, 0 failed |
| `cargo test -p brokkr-cli --all-features --locked` | ok: 463 lib + 312 across 31 binaries (`doctor_dsh_selection` 14) = 775 passed, 0 failed, no hang |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` / `bundles/verify` | both compile, exit 0 |
| `openspec validate --all --strict` | **NOT RUN from this seat**: refused (approval required) under the bare spelling, as for every prior seat. This return edits `tasks.md`, `design.md` and the AS1 delta's requirement text and two existing scenarios only, adding no requirement, scenario or file; the deltas still hold **20 requirements / 219 scenarios**. Pending on a seat that can launch it. |
| `bash scripts/coverage-exact.sh` | **NOT RUN literally**: refused. Its steps by hand on the pinned nightly, in order, into `.forge/scratch/r124b-coverage/` (`git grep coverage(off)` empty; `llvm-cov clean --workspace`; `llvm-cov --workspace --all-features --locked --branch --json`, **68 binaries `test result: ok`, 0 failed, 0 `Text file busy`**, no `--ignore-run-fail`; `llvm-cov report --branch --lcov`); the script's `jq` harness-leak check on the JSON → `true`, **0 harness files among 53 `SF` records**; its LCOV rule through the retained transcription `.forge/lcovtool` (`cargo run --release --manifest-path …`). |

| Pass | Lines | Branches | Functions | Misses |
|------|-------|----------|-----------|--------|
| Fresh, on this return's bytes | **32,324 / 32,324 (100%)** | **5,448 / 5,448 (100%)** | **3,135 / 3,135 (100%)** | none |

The denominator moved from `3a1df8d3`'s 32,300 / 5,440 / 3,132 by +24
lines, +8 branches and +3 functions: `mapping_key`, the file-that-runs
resolution and refusal in `env_program`, and the trimmed key; every record
is hit. The chief's own fresh count on `3a1df8d3` (32,124 / 5,426 / 3,122
— not exact) was made with a unique target after an ETXTBSY-poisoned first
protocol run and a crate-scoped rerun; the recount of the retained r124
evidence was exact, and this pass is a fresh whole-workspace instrumented
run on this return's bytes. The literal script, CI's `coverage-exact` job
and final-head remote CI remain the gate's own artifacts, pending until
they run.

**Dependent artifacts kept coherent.** AS1's requirement text now states
the file-that-runs rule beside the `env`-name rule; the scenario
*Interpreter aliases cannot bypass the Node obstruction refusal* carries the
renamed-symlink refusal, the preserved links, the stand-in disagreement and
E6, and *Duplicate decoded pnpm package keys refuse before triple
normalization* carries the YAML-equality rule, its controls and D4–D5.
Design D10 §4 records both answers as dated paragraphs refining the
2026-09-21 return's rules. `## Decisions`, 8.8.2.1 and 8.8.4 in this file
carry the same two rules. No checkbox changed: the fourteen local addresses
stay **5 complete / 9 pending**, the 101 change-wide identifiers **84 /
17**, the 133 rows **105 / 28**.

**Still pending, recorded and not claimed.** **8.8 stays unchecked**; part
**(d)**, **8.10**, **9.6**, 10.6–10.8, 11.1–11.4 and groups 14–15 untouched;
native **macOS/Windows** execution (matrix, doctor, `GetBinaryTypeW`,
Windows 1.88 MSRV); the immutable Apple and env source pins (no source
access in this seat either: uutils' executable-name check and busybox's
`argv[0]` dispatch are cited from their native behaviour on this host and by
function); the absent-PATH Node positive and its two removals (no `node` on
`/bin:/usr/bin`); strict OpenSpec on this head; the literal coverage
script; final-head remote CI; independent verify and the council's third
sitting on this return. No `wrapper_digest` is declared, the DSH route
stays disabled, **0056 stays proposed**, and `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`, `reference/`,
`extensions/dsh/` and `docs/decisions/` have no diff. The active change is
not archived. Nothing was pushed.

### Implement visit — the delivered port confirmed as found, 2026-09-21

This implement seat (run `dsh-composite-identity-issue-226-124cca78`, no
`returned_from`) adopted `slice-dsh-composite-b` at
`42bb02108360e80976d462ba7263d091457bd272`, clean at entry, with the delivered
`417354ec` / `25b40967` and the adoption commits `3dfb3c5c` / `c3f36568` /
`42bb0210` as ancestors. Its commission was 8.8(a)–(c) only: confirm the
delivery on the committed bytes, change nothing unless a gate genuinely
failed there, and hand the port to independent verify and the full review
council. No gate failed, so **no Rust, test, capability, decision, compiler
pin or frozen byte changed**; this account is the visit's only edit, and
it changes no checkbox state.

Environment: Linux x86_64 glibc host, unboxed, stable `cargo 1.98.0` for
the suites and the pinned `nightly-2026-09-05` for coverage. The shell of
this seat refuses shell loops, `$?` expansion, `awk`, script launches and
`openspec`, so every command below was issued singly and the LCOV rule was
applied by the run-local Rust transcription under `.forge/lcovtool/` (the
security-hold visit's evaluator, rebuilt unchanged).

| Gate, on `42bb0210` | Result |
|---|---|
| `cargo fmt --all -- --check` | clean, exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, exit 0 |
| `git diff --check`; `git grep coverage(off)` under `crates/` | clean; no exclusion |
| `cargo test -p brokkr-core --all-features --locked` | ok: 73 lib + 13 across 3 binaries; 0 failed |
| `cargo test -p brokkr-store --all-features --locked` | ok: 58 lib + 6 across 5 binaries; 0 failed |
| `cargo test -p brokkr-protocol --all-features --locked` | ok: 377 lib, 99 integration (2 ignored), 1 doctest; 0 failed |
| `cargo test -p brokkr-runtime --all-features --locked` | ok: 441 lib + 93 across 22 binaries; 0 failed — the four roster cases recorded on 2026-09-20 pass on this head |
| `cargo test -p brokkr-view --all-features --locked` | ok: 243 lib (3 ignored); 0 failed |
| `cargo test -p brokkr-bridge --all-features --locked` | ok: 13; 0 failed |
| `cargo test -p brokkr-cli --all-features --locked` | ok: 463 lib + 312 across 28 binaries incl. `doctor_dsh_selection` (14 — this row first said 6, a miscount corrected on the 2026-09-21 return against `.forge/v88b-test-cli.log:606-624`, review R3); 0 failed; zero `Text file busy` — one attempt, no rerun needed |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles, exit 0 |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | compiles, exit 0 |
| `openspec validate --all --strict` | **NOT RUN from this seat**: `openspec`, its full path and `npx` each require an approval this non-interactive seat cannot grant. The tasks seat's run on `c3f36568` passed **15 / 0** the same day; `42bb0210` and this visit change only `tasks.md`. |
| `bash scripts/coverage-exact.sh` | **NOT RUN literally**: script launches are refused in this seat. The D11 equivalent below was run instead; the literal gate stays with a capable host / CI. |

The seven suites were run one crate at a time, in the framed order, never
workspace-wide; the crate totals (86 / 64 / 477 / 534 / 243 / 13 / 775 =
**2,192 passed, 0 failed**) equal the controller's unboxed measurement on
`25b40967` in `controller-branch-gates-2026-09-21.json`, now reproduced by
this seat on `42bb0210`.

Fresh exact coverage, the script's steps by hand on the pinned compiler,
in order: `cargo +nightly-2026-09-05 llvm-cov clean --workspace`; `cargo
+nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch
--json --output-path .forge/v88b-coverage.json` (68 test binaries, every
test run, **0 failed**, no `--ignore-run-fail`, exit 0); `cargo
+nightly-2026-09-05 llvm-cov report --branch --lcov --output-path
.forge/v88b-lcov.info`; the harness-exclusion check on the JSON (**0**
harness files among 53 `SF` records); the exact rule record by record.
One departure from the literal script, named: the instrumented target is
the default `target/llvm-cov-target` after the clean rather than a unique
`$CARGO_LLVM_COV_TARGET_DIR`, because this seat cannot set an environment
variable on a command; the clean step ran first, so no earlier profile
joined the merge.

| Pass | Lines | Branches | Functions | Misses |
|------|-------|----------|-----------|--------|
| Fresh, on `42bb0210` | **32,322 / 32,322 (100%)** | **5,440 / 5,440 (100%)** | **3,139 / 3,139 (100%)** | none |

All three integers equal the controller-attributed delivery measurement in
8.8.8.3 exactly; the production denominator did not move. Logs and reports
are run-local under `.forge/v88b-*`, untracked, read by no build or test.

Read on the delivered bytes, not re-derived: `glibc_walk` carries the
`p`/`subp` cursor pair, the oversized `continue` that leaves the cursor on
the colon and the late colon increment, so the bare-name candidate after
an oversized skip is constructed as glibc constructs it and refused by the
named cwd reason (`Origin::AfterOversizedSkip`); `musl_walk` steps past
the colon and constructs no such candidate; `apple_walk` separates
`execvP`'s warn-and-continue from `posix_spawnp`'s `ENAMETOOLONG` stop;
`refuse_working_directory` reads metadata only and preserves a native stop
cause (`ELOOP`, `ENAMETOOLONG`) over the cwd refusal. Nothing here is a
verdict — independent verify and the full review council own theirs
under 8.8.8.4.

**Checkbox states, unchanged, and why.** 8.8.8.2 stays open because
`openspec validate --all --strict` could not execute from this seat, which
its own text makes a pending ground. 8.8.8.3 stays open because a by-hand
equivalent does not supply the literal gate; its fresh integers are
recorded above for the capable-host run to confirm. 8.8.8.4 stays open
until verify and the council return their actual findings or a clean
verdict. 8.8.1.1, 8.8.1.2, 8.8.2.1, 8.8.2.2, 8.8.3.1 and 8.8.8.1 keep
their native macOS/Windows/MSRV, Apple/env source-pin and retained-Node
debts. The fourteen local addresses stay **5 complete / 9 pending**, the
101 change-wide identifiers **84 complete / 17 pending**, and the file's
133 checkbox rows **105 / 28**.

**Delivered, and confirmed as found:** 8.8(a)–(c) — the closed
`wrapper_digest` loader, the sole sealed composite producer over the fixed
D6 locators, and the once-resolved DSH doctor — with the fourth hold's
literal platform walks and the never-cwd rule, file-established env
invocations (E1–E4) and ignored-pnpm structure (P1–P3). **Still pending,
recorded and not claimed:** **8.8 stays unchecked**; part **(d)**, the
planner, untouched; **8.10**'s rejection-vector ledger; **9.6**;
10.6–10.8, 11.1–11.4 and groups 14–15; native **macOS** and **Windows**
execution (matrix, doctor, `GetBinaryTypeW`, Windows 1.88 MSRV); the
immutable Apple and env source pins; the absent-PATH Node positive and its
two removals; strict OpenSpec on this head from a seat that can launch it;
the literal coverage script on a capable host; final-head remote CI. No
`wrapper_digest` is declared, the DSH route stays disabled and unmeasured
for admission, **0056 stays proposed**, and `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`, `reference/`,
`extensions/dsh/` and `docs/decisions/` have no diff against `42bb0210`.
The active change is not archived. Nothing was pushed.

### Tasks-phase validation — delivered-port adoption, 2026-09-21

Entry HEAD `c3f365682c8b6f598744145889373ac3dec2a6d4`, clean at entry;
`417354ec` and `25b40967` are ancestors. Only proposal/design have changed
since the delivered code. Read both commissioned controller records, current
AS/AS1/D10/Open Questions, the rendered tasks/return instructions, delivered
source and task removal accounts. This amendment changes only `tasks.md`;
no Rust, test, capability, decision, compiler pin or frozen byte changes.

| Check | Observed outcome and provenance |
|---|---|
| Seven sequential crate-scoped all-feature locked suites | Each `cargo test -p <crate> --all-features --locked` was attempted separately, in order: core, store, protocol, runtime, view, bridge, CLI. Each exited **127**, Cargo unavailable, zero tests executed. No fresh Rust pass or test failure is claimed. |
| Format/clippy and self/verify bundle compiles | Each exact command in 8.8.8.2 exited **127**, Cargo unavailable. |
| Controller's unboxed checks on `25b40967` | **2,192 passed, 0 failed**: core 86, store 64, protocol 477, runtime 534, view 243, bridge 13, CLI 775; fmt and clippy clean. Inherited from `controller-branch-gates-2026-09-21.json`, not measured by this seat. |
| Strict OpenSpec | `openspec validate --all --strict` passed before and after the amendment: **15 passed, 0 failed**. Strict change validation and artifact status also passed. Existing informational archive-readiness messages are whole-change debt. |
| Fresh coverage preparation | Pin `nightly-2026-09-05`, unique run-local target directory; `cargo +nightly-2026-09-05 llvm-cov clean --workspace` exited **127** before instrumentation. Fresh lines/branches/functions are each **unavailable**. No workspace test invocation or literal script ran. |
| Retained coverage, independently recounted | The unchanged gate's own AWK program applied to `.forge/scratch/coverage/lcov.info` gives **32322/32322 lines, 5440/5440 branches, 3139/3139 functions**, with zero harness-source leaks in its JSON. This agrees with the controller-attributed delivery measurement; it is not fresh coverage or a literal-script pass. |
| Coverage source/pin checks | CI, release and the exact script consume `rust-nightly-version.txt`; tracked Rust contains no `coverage(off)` exclusion. No threshold, exclusion or denominator changed. |
| Task/scope audit | 115 numbered rows (89 checked / 26 open), including 14 local (5 / 9); 133 total checkbox rows (105 / 28), no duplicate numbered IDs, all states and dated history preserved. Each local task names AS1. All 20 requirements / 219 scenarios and delivered source/gate/frozen bytes remain unchanged; `git diff --check` is clean. |

Attempt logs and report hashes are under `.forge/tasks/validation-124cca78/`,
untracked run evidence. Retained reports were read, not replaced. #255's
historical ETXTBSY failure and green reruns remain accurately recorded below;
that history establishes no current port defect. Cargo's absence is an
execution limitation, not an earlier-artifact fault, so this phase can draft
the honest breakdown without claiming delivery verification.

The **tasks** result is `drafted`, with the existing change identifier. All
checkbox states stay unchanged. Independent verify/full review and fresh Rust /
capable-host exact results remain pending. **8.8 remains unchecked**; part
**(d)**, **8.10**, **9.6**, native **macOS/Windows** and the named source-pin /
retained-Node evidence remain pending. **0056 stays proposed**. No shipping,
whole-change completion, archive, push or broader #226 work is claimed.

### Implementation return — carry-overs of the fourth hold, 2026-09-21 (Europe/Sofia)

This implement seat (run `dsh-composite-identity-issue-226-5e82d607`, the
`IMPL-BROKEN-RETRY` return) adopted `slice-dsh-composite-b` at `417354ec`,
every commit kept, in worktree `brokkr-wt-dsh88b` on Linux x86_64
(`x86_64-unknown-linux-gnu`, glibc 2.42 as measured by the design seat;
`cargo 1.98.0 (797e8a9bc 2026-08-05)` ran every gate, `nightly-2026-09-05`
the coverage steps). It answers the gaps the previous visit named, in
their owning clauses, and records what still cannot run from a seat. It
describes outcomes and directs no gate.

**8.8.2.1 — the supported env invocation is established from the file.**
`env_program` no longer refuses every name but `env` outright. The
reference's canonical file decides what `env` is installed as
(`EnvDispatch`): a file NAMED `env` or a link to a multicall of another
name. Under a multicall link only the name `env` is established. Under a
file named `env`, `env_invocation` establishes an other-name invocation
exactly when the spelled name is the interpreter's OWN canonical file name
(a hard link or copy, never a renaming symlink) and spells `env` as a
prefixed utility name (`uu_env`, `gnu-env`), which uutils'
`src/bin/coreutils.rs` prefixed-utility dispatch reads as `env` and a
dedicated `env` ignores. Every other other-name spelling is its own named
refusal, never `Ok(None)`. Source references are cited by function in the
code; their revisions and line numbers are NOT pinned (no source access in
this seat). Design D10 §4 records the implemented rule.

Native evidence, this host — `/usr/bin/env` resolves to
`/usr/lib/cargo/bin/coreutils/env`, uutils installed under the name `env`
(`an_env_argument_is_selected_as_the_kernel_hands_it_to_env`, protocol
and doctor, every spelling with its own native child and the doctor's own
marker directory):

| Spelling of the same file | Native child (obstructed A/node; valid chain) | Resolver / doctor |
|---|---|---|
| `tools/uu_env`, hard link of the copy (the chief's R8 oracle) | ran B's node both times (`MARK:b-node`; doctor markers `["DSH_B_NODE_0.0.3"]`) | A's obstruction named, zero doctor markers; then B's node selected and retained, doctor `ok dsh: DSH_B_NODE_0.0.3` from exactly one probe |
| `tools/env-alias`, symlink | exit 1 both times: `Security violation: Requested utility `env-alias` does not match executable name: /usr/lib/cargo/bin/coreutils/env` | refused: `invoked under the name 'env-alias', which is not the name of the file that runs ('env')`; zero markers |
| `tools/link_env`, symlink spelled as a prefixed name | exit 1: the same security violation for `link_env` | refused by the own-name cause; zero markers — the cell only the own-name rule refuses |
| `tools/myenv`, hard link, no prefixed spelling | exit 1: `node: function/utility not found` (the utility fell to its argument dispatch) | refused: `does not spell env as a prefixed utility name`; zero markers |
| `/usr/bin/env`, `linked/env` symlink, `tools/env` copy | ran B's node | A's obstruction, then B's node (unchanged) |
| `impostor/env` | ran B's node through the impostor | refused as not the platform's env (unchanged) |

The multicall rule ran on this host through an injected reference: a
stand-in that dispatches on `$0`'s basename (`multi/multicall`, with
`multi/env` a symlink to it and `multi/uu_env` a hard link). Under the
name `env` the chain is followed — A's obstruction, then B's node, the
native child running B's node through it; under `uu_env` the resolver
refuses by the multicall cause and the native child answers `multicall:
applet uu_env not found` (exit 127). `env_invocation` is additionally
asserted arm by arm on files (`env_identity_is_the_file_and_never_a_name`
and the same test), including an interpreter that cannot be resolved.
Doctor's expectations are computed from what `/usr/bin/env` resolves to,
so a host whose `env` is a multicall link asserts the multicall arm
instead; no host records a pass it did not run.

**8.8.3.1 — ignored pnpm bodies keep their structure.** `pnpm_ignored_line`
now returns the admitted MEMBER (a sequence item, or a mapping entry with
whether it opens a block) and `IgnoredBody` carries a private stack of
open blocks — indentation and kind — through every ignored section body
and every block-form package child, reset when a section or child opens.
Three rules, each YAML's own and each a separate refusal: a member that
opens no block has no children (`the entry 'foo' nested below the scalar
entry 'react', which opens no block`; below a sequence item likewise); one
block's members are one kind (`the sequence item 'foo' at 6 spaces beside
mapping entries, which mixes mapping entries and sequence items in one
block`, and the reverse); a dedent returns to an open block (`the entry
'mid' at 4 spaces, which dedents to no open block`). Nothing reads a value,
no YAML dependency was added, the 8,388,608-byte bound and identity bytes
are untouched. New vectors in
`missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
(seven refusals: scalar parent under `peerDependencies` and in
`snapshots`, mixed siblings both ways, a child below a sequence item, two
dedents) and
`ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`
(four refusals through the built doctor, each asserted never to report the
control's digest); readable controls kept and added — the existing nested
`peerDependenciesMeta`/`snapshots` bodies, a null block key followed by
its sibling, a dedent back to an open block, a sequence block after a
mapping block, a body whose first line sits at four spaces, and a
`snapshots` body with a sibling record through doctor — all the control's
own digest.

**Removal proofs** (each a compiling mutation of the enforcing line, run
with `cargo test --no-fail-fast -p brokkr-protocol -p brokkr-cli
--all-features --locked -- <the two regressions' names>`, the failed
assertion quoted, exact restoration, the same command green again — 1
passed in each surface — before the next):

| Removal | Named regression → failed assertion |
|---|---|
| E1 blanket other-name refusal restored (`env_program`, the `(true, false)` arm returns the old refusal) | protocol `an_env_argument…` `tools/uu_env: no spelling bypasses D10`: left the blanket refusal, right A's obstruction; doctor `an_env_argument…` line 1124: the `uu_env` line carries the blanket refusal, not A's obstruction |
| E2 name recognition restored (`is_env … \|\| file_name == "env"`) | protocol: impostor cell, left A's obstruction through `impostor/env`, right `is named env but is not the platform's env utility '/usr/bin/env'`; doctor: the same impostor line |
| E3 own-name check removed (`own_name != spelled && false`) | protocol `tools/link_env: no spelling bypasses D10`: left A's obstruction (the chain FOLLOWED through a symlink native refuses), right the own-name refusal; doctor: the same `link_env` line |
| E4 prefixed-name check removed (`!prefixed && false`) | protocol `tools/myenv: no spelling bypasses D10`: left A's obstruction, right the prefixed-name refusal; doctor: the same `myenv` line |
| P1 parent-type check removed (`self.closed.as_ref().filter(\|_\| false)`) | protocol `missing_pnpm…`: the scalar-parent lock `…react: '>=16.8.0'\n        foo: bar\n` **was accepted**; doctor `ignored_pnpm…` line 2406: that lock reported `ok dsh … composite f742ba0e…` — the control's digest |
| P2 collection-kind check removed (`frame.block != block && false`) | protocol: the mixed lock `…react: '>=16.8.0'\n      - foo\n` was accepted; doctor: the same lock reported the control's composite |
| P3 dedent check weakened (`frame.indent <= indent`) | protocol: `…debug@2.6.9:\n      deep: 1\n    mid: 2\n` was accepted; doctor: the `mid` lock reported the control's composite |

`git diff --stat` on `composite.rs` is identical before and after every
restoration. The fourth-hold M1–M11 records above and the third hold's
records stand; nothing they cover was re-mutated here.

**Gates on the restored candidate.**

| Check | Actual outcome |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no output. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Exit 0, `Finished`, no warning. |
| `cargo test -p brokkr-core --all-features --locked` | 73 + 3 + 8 + 2 passed, 0 failed. |
| `cargo test -p brokkr-store --all-features --locked` | 58 + 1 + 1 + 1 + 1 + 2 passed, 0 failed. |
| `cargo test -p brokkr-protocol --all-features --locked` | 377 + 99 (2 ignored) + 1 passed, 0 failed; the native matrix passed. |
| `cargo test -p brokkr-runtime --all-features --locked` | Every binary `test result: ok` (441 in the lib), 0 failed. |
| `cargo test -p brokkr-view --all-features --locked` | 243 passed (3 ignored), 0 failed. |
| `cargo test -p brokkr-bridge --all-features --locked` | 13 passed, 0 failed. |
| `cargo test -p brokkr-cli --all-features --locked` | 463 + every integration suite passed (32 `test result: ok`, `doctor_dsh_selection` 14), 0 failed, no hang. |
| `cargo test --workspace` | First run: one panic, `hands::tests::the_network_prefix_is_eight_tokens_and_the_probe_asks_the_dispatchs_path` (`hands/tests.rs:1199`, a freshly planted `unshare` script spawned by the probe — outside this slice, the #255 shape); the test passed alone and the second full run had every binary `test result: ok`, 0 failed, no panic, no hang. Both recorded. |
| `cargo test --workspace --all-features --locked` | Every binary `test result: ok`, 0 failed, no panic, no hang. |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` / `bundles/verify` | Both compiled; the plan JSON printed (self: seats implement/intake/review/ship/verify; verify: review/verify). |
| `openspec validate --all --strict` | NOT EXECUTED: refused by the seat under both the bare and the absolute-path spelling. Pending. |
| `TMPDIR=/tmp bash scripts/coverage-exact.sh` | The literal script launch is refused by the seat under every spelling tried (`TMPDIR=… bash …`, `env TMPDIR=… bash …`). The script's own three cargo steps were run by hand on the restored bytes with the unchanged pin (`cargo +nightly-2026-09-05 llvm-cov clean --workspace`; `… llvm-cov --workspace --all-features --locked --branch --json --output-path …`; `… llvm-cov report --branch --lcov --output-path …`), its `jq` harness-leak check applied to the fresh JSON (`true`) and its LCOV rule applied through the retained transcription `.forge/lcovtool`. Reports under `.forge/scratch/coverage/` (`coverage.json`, `lcov.info`), outside the commit; 66 `test result: ok`, 0 `FAILED`, no `--ignore-run-fail`. **Lines 32322 / 32322 (100%), branches 5440 / 5440 (100%), functions 3139 / 3139 (100%)**, no uncovered record. The denominator moved from the previous visit's 32216 / 5424 / 3122 by this delivery's own production lines (`IgnoredBody`, `env_dispatch`, `env_invocation`). The literal script and CI's `coverage-exact` job on the final head remain the gate's own artifact and are pending until they run. |

The first `cargo test -p brokkr-protocol … composite::` baseline run of
this visit hit the #255 ETXTBSY race once
(`spawn_node_runtime_reads_one_version_line_and_refuses_the_rest`,
`Text file busy (os error 26)`); the rerun and every later run passed.

**Still pending, recorded and not claimed.** The immutable Apple
`exec.c`/`posix_spawn.c` revision pin and native macOS execution: this
seat's `curl`, `WebFetch` and the GitHub MCP (`list_tags`,
`get_file_contents`) were each refused, so the port still carries D10's
moving-`main` ranges; the source-cited env dispatch rules are likewise
unpinned. Native Windows matrix/doctor/`GetBinaryTypeW` and the Windows
1.88 MSRV build. The absent-PATH Node positive and its two removals:
`absent_path_node_identity_is_retained_by_the_composite` and
`absent_path_default_search_matches_native_dsh_and_node` both printed
`PENDING: no node on this host's default search path /bin:/usr/bin` and
asserted the named refusal. `openspec validate`, the literal coverage
script and remote CI on the final head. The post-probe head reopen and the
groups 4–7 removals keep their dated records. No local clause changes
state: 8.8.1.1/8.8.1.2 stay open on the Apple pin and native macOS,
8.8.2.1 on the pin and native platforms (its alias qualification is
delivered above), 8.8.2.2 on native platforms, 8.8.3.1's structure is
delivered above and its clause stays with 8.8.8.2's validation, 8.8.8.1
on the Node positive, 8.8.8.2 on `openspec validate`, 8.8.8.3 on the
literal script, 8.8.8.4 with them. Part (d), 8.10, 9.6, 10.6–10.8,
11.1–11.4, groups 14–15 were not touched; 8.8 stays unchecked; 0056 stays
proposed; `contracts/`, `policy/`, `fixtures/`, `reference/`,
`extensions/dsh/`, `docs/decisions/` are byte-identical (`git diff --stat`
over them is empty); no push.

### Implementation delivery — fourth security hold, 2026-09-21 (Europe/Sofia)

This implement seat (run `dsh-composite-identity-issue-226-5e82d607`) adopted
`slice-dsh-composite-b` at `afc3ddd0`, every commit kept, in worktree
`brokkr-wt-dsh88b`: Linux x86_64, target `x86_64-unknown-linux-gnu`, glibc
2.42 as the design seat measured it (this seat's `ldd`/`rustc` invocations
were refused; `cargo 1.98.0 (797e8a9bc 2026-08-05)` is the stable toolchain
that ran every gate), `nightly-2026-09-05` for coverage. The account
separates what ran from what remains pending; it describes outcomes and
directs no gate. It delivers 8.8(a)–(c)'s fourth-hold obligations for the
lookup (groups 8.8.1 and the lookup half of 8.8.2) and records the rest.

**Network in this seat.** `curl`, `WebFetch`, `gh api` and the GitHub MCP
were each refused, so `glibc-2.42/posix/execvpe.c`, Apple `gen/FreeBSD/exec.c`
and `sys/posix_spawn.c` could not be opened here. The glibc port cites the
line ranges design D10 §1 inspected (86–106, 107–119, 121–126, 134–158,
160–168) beside each corresponding block; the Apple port carries D10's
moving-`main` ranges (178–218, 262–297; 97–143, 170–195) and its immutable
revision pin is PENDING. Where the port's reading of the source could be
tested, the native oracle decided it (below).

**R1 — the loop is ported, not modelled.** `Search::find` no longer walks
`std::env::split_paths` with a per-component classification. It hands the
exact `PATH` bytes and the program bytes to `walk_search`, which dispatches
by library and operation to three literal ports: `glibc_walk` (the `p`/`subp`
cursor pair, `path_len = strnlen(path, PATH_MAX − 1) + 1`, `subp - p >=
path_len` with the final-component `break` and the early `continue` that
leaves `p` ON the colon so the next iteration constructs the bare name —
`Origin::AfterOversizedSkip` — and the `*subp++ == '\0'` increment at the
end of a completed iteration; the extra `/` for any nonempty entry, a
trailing `/` included), `musl_walk` (whose oversize branch is `if (!*z++)
break; continue;` — past the colon, no implicit cwd iteration), and
`apple_walk` (the `strsep` token walk, an empty token spelled `.`, the
`lp + ln + 2 > PATH_MAX` bound: `execvP` warns and continues, `posix_spawnp`
stops with ENAMETOOLONG). Every candidate carries its provenance
(`Origin::Entry`, `EmptyEntry { index }`, `AfterOversizedSkip { skipped }`).
`Construction`, `construction` and `glibc_path_len` are retired.

**R1 — the reconciled rule, in the code's words.** `Search::find` states it
verbatim: equality with native is necessary — nothing is selected that
native would not execute — and not sufficient: a candidate in the working
directory (an empty PATH entry, or the implicit cwd iteration glibc produces
after an oversized skip) is NEVER selected and NEVER skipped past. Never
cwd, otherwise native. `refuse_working_directory` reads the cwd candidate's
metadata only — no head, loader or interpreter inspection — and preserves a
native STOP there (glibc ELOOP on a cwd self-symlink, the kernel's
ENAMETOOLONG on a bare overlong name); every other outcome — a runnable
file, a missing one, a directory, a denial, Apple's continuable ELOOP —
refuses with `<name>: the platform's search would fall into the working
directory: PATH entry N is empty` or `…: glibc skips the N-byte component
and its next iteration is the empty entry it leaves the cursor on
(posix/execvpe.c 118–124, 168)`. Absent `PATH` stays native default-search
equality (`sh` positive preserved; `/bin:/usr/bin` on this host).

**R4 — the invented NAME_MAX refusal is gone for glibc.** The native oracle
decided the source reading this seat could not re-open: with `PATH` naming
only a MISSING directory, a 300-byte name is ENOENT to the native child (the
name was searched, not refused); under a file spelled as a directory it is
ENOTDIR; under an existing directory the kernel answers ENAMETOOLONG and
glibc stops; at the working directory (`PATH=""`) the kernel's ENAMETOOLONG
on the bare name is preserved as native's stop; with only an oversized
component nothing is constructed and the child reports whatever errno its
thread already had — proved by PLANTING ENOENT and then ENOTDIR with a
failing `metadata` call before each spawn and reading exactly the planted
value back, in both Command forms. `admit_program_name` keeps musl's
`k > NAME_MAX` refusal and refuses nothing by length on glibc.

**R5 — exhaustion keeps the last cause.** `Exhausted { denied, last,
attempted }`: the first remembered denial is reported as EACCES; otherwise
`'<name>' is not on PATH (the search ended at <candidate>: <cause>)` names
the last candidate and its own errno — ENOTDIR/20 after `nowhere:file`,
ENOENT/2 after `file:nowhere` — and a search that constructed nothing says
`(the search attempted no candidate: every component was skipped as longer
than the buffer the platform builds one in)`.

**R6 — the operation is carried.** `Operation::{Spawn, Exec}` on `Search`:
production's `select` is `Spawn` (Rust 1.88 `unix.rs` 417–423 takes
`posix_spawnp` for an unchanged environment), the explicit-`PATH` form and
`resolve_executable_in` are `Exec`, and `env_program` searches with
`search.for_env()` — always `Exec`, whatever the outer form. On glibc and
musl the operation changes nothing; on Apple it selects between the two
walks. `select_in` is now test-only; production has one entry, `select_as`.

**Native evidence, this host (glibc).**
`native_executable_resolution_matches_command_matrix`: 8 names × 52
layouts = 416 cells in both forms, plus 14 removal-control layouts in both
forms and 13 named controls — **997 oracles**, each counted against the
declared inventory; tally 334 equal selections, 126 NotFound parities, 250
terminal-error parities, 104 NUL refusals, 42 D10 loader exceptions and
**128 working-directory refusals**, the last recorded as their own kind and
never as equality. New layouts: 4095/4096/5000 × cwd runnable / self-symlink
/ absent (with the existing six no-cwd lengths kept); `A::B`, `A:`, `:B`,
`PATH=""` × the three cwd states; a 4096 skip followed by an explicit empty
entry; `A::B`/`A:` with a runnable A (earlier success wins); A padded with
slashes to 4092 and 4093 bytes with A/dsh present and absent (native
ENAMETOOLONG/36 at 4,096 bytes; the one-slash removal selects A when present,
B otherwise); a final 5000-byte component alone with cwd/dsh runnable
(nothing attempted). Per-cell glibc assertions: 255 → B; 256/300/4095 →
errno 36 both ways with the terminal-cause text and neither B nor cwd
named; 4096/5000 → native cwd marker / ELOOP 40 / B, resolver cwd reason /
`a symlink loop stops the lookup … (os error 40)` / cwd reason. Both forms
are compared on their own on every platform and additionally asserted
equal on glibc. Controls: overlong under existing / missing / file prefixes,
missing-then-existing, at cwd, no-candidate (planted ENOENT, then ENOTDIR),
explicit path, the 256-byte boundary, the 255-byte positive, the two
ordered exhaustion causes, the `sh` default-search positive.
`terminal_path_lengths_refuse_before_doctor_probe`: **43 cells** on the
built binary — 255/256/300; 4095/4096/5000 × three cwd states; the four
empty spellings × three cwd states; two earlier-A controls; three padded
spellings; and 14 same-fixture removals — each with an explicit-form oracle,
an inherited-form oracle (the test binary re-entered with the staged
environment, `Command::new("dsh")` unchanged) and a doctor run whose marker
directory is asserted EMPTY before its prose in every refused cell.
`absent_path_default_search_matches_native_dsh_and_node`: the present-empty
cell now asserts the cwd refusal and no marker with a cwd `dsh` decoy (the
`sh` decoy is removed for that cell — see the residual below).
`an_empty_path_entry_is_the_working_directory_and_is_refused` (renamed from
`…_is_the_current_directory`): every empty entry refused at its position,
`.` still selected as a nonempty component, absent `PATH` still the default
search. `the_lookup_rule_is_each_librarys_own_switch_arm_by_arm`: the
candidate SEQUENCE table for glibc, musl and both Apple operations (empty
entries, the extra slash, the implicit iteration, the final-oversize break,
Apple's skip-versus-stop), and the whole search under Apple `Spawn`/`Exec`
and musl. The classifier test's 4096/5000 cells now assert the cwd refusal
where native walks on to B, and the overlong name is asserted under all
three prefixes.

**Removal proofs** (each a compiling mutation of the enforcing line, the
focused commands from this file's preamble, the failed assertion quoted,
exact restoration, `cargo fmt --check` and the composite module green
again — 99 passed — before the next):

| Removal | Named regression → failed assertion |
|---|---|
| M1 direct advance to B after an oversized skip (`glibc_walk`, `p = subp + 1`) | matrix `n0-l12/inherited` `the resolver selected a file where the search reached cwd … native ran l12n0s1 [cwd/dsh], resolver Ok(…/b/dsh)`; doctor `4096-byte component, then B; cwd Runnable [cell]: doctor executed something where it must refuse` — markers `["DSH_B_LENGTH_SENTINEL_0.0.6"]`, native ran the cwd sentinel |
| M2 cwd barrier removed (`find`, `&& false`) | matrix `n0-l12/inherited` `… resolver Ok(…/cwd/dsh)`; doctor same cell — markers `["DSH_CWD_LENGTH_SENTINEL_0.0.7"]` |
| M3 pre-buffer skip removed (`glibc_walk`, `&& false`) | matrix `n0-l10/inherited` final-oversize cell: invented `metadata answers File name too long (os error 36)` where nothing is attempted; doctor `4096-byte component, then B; cwd Runnable [cell]: doctor names the cause: … would fall into the working directory` — replaced by ENAMETOOLONG |
| M4 ENAMETOOLONG made continuable (`step`, glibc) | matrix `n0-l9/inherited` `ENAMETOOLONG is named as the stop it is` (denial-then-terminal: the remembered EACCES was reported); doctor `256-byte component, then B [cell]: doctor executed something where it must refuse` — markers `["DSH_B_LENGTH_SENTINEL_0.0.6"]` |
| M5 ELOOP made continuable (`step`, glibc) | matrix `n0-l4/inherited` `the resolver selected a file where the child stopped … errno 40, resolver Ok(…/b/dsh)`; doctor `4096-byte component, then B; cwd Loop [cell]: doctor names the cause: dsh: a symlink loop stops the lookup` — cwd reason reported instead |
| M6 extra slash normalized (`glibc_walk`) | matrix `n0-l39/inherited` padded-A `the resolver selected a file where the child stopped … errno 36, resolver Ok(…/a/dsh)`; doctor `A padded to 4092 bytes … [cell]: doctor executed something where it must refuse` — markers `["DSH_A_EARLIER_SENTINEL_0.0.8"]` |
| M7 final non-denial cause erased (`find`) | matrix control `overlong-under-missing-prefix`: `'x…' is not on PATH` ≠ `… (the search ended at …/controls-nowhere/x…: No such file or directory (os error 2))`; doctor regression GREEN under this mutation (no exhaustion cell), recorded as such |
| M8 remembered EACCES erased (`find`) | matrix `n0-l8/inherited` `EACCES is named as the denial it is` (denial-then-miss reported the miss); classifier `'dsh' is not on PATH (the search ended at …/b/dsh …)` ≠ `'dsh' is not executable by this process on PATH: …/a/dsh …`; doctor GREEN (no denial cell) |
| M9 terminal precedence weakened (`find`, refused walked past after a denial) | matrix `n0-l9/inherited` `ENAMETOOLONG is named as the stop it is`; doctor GREEN (no such cell) |
| M10 Apple operations conflated (`apple_walk`, `Spawn` continues) | `the_lookup_rule_is_each_librarys_own_switch_arm_by_arm`: `[("B/dsh", Entry)]` ≠ `[("STOP x…/dsh", Entry)]` — table evidence only; native macOS pending |
| M11 invented glibc NAME_MAX refusal restored (`admit_program_name`) | matrix control `overlong-bare-name`: `'x…' is 300 bytes long, more than the 255 bytes NAME_MAX …` ≠ `…/controls/x…: metadata answers File name too long (os error 36) …`; classifier the same under `a/`; doctor GREEN (no overlong cell) |

After M1–M11 the by-hand coverage pass (below) named four lines and two
branches in the new resolver code that no test could reach — an exhaustion
arm no walk can produce, an `unreachable!` origin arm, and a redundant
pattern test inside the cwd refusal — and they were removed by refactor
(`stop_cause` shared by the ordinary and cwd candidates; the caller
computes how the search reached cwd; `(None, None)` exhaustion means no
candidate), not by exclusion. M2 and M7, whose enforcing lines that
refactor touched, were re-mutated and re-proved on the FINAL bytes with
the same failed assertions as above (matrix `n0-l12/inherited … resolver
Ok(…/cwd/dsh)` and doctor markers `["DSH_CWD_LENGTH_SENTINEL_0.0.7"]`;
matrix control `overlong-under-missing-prefix` and the classifier's
`'node --flag' is not on PATH (the search attempted no candidate …)` ≠
`… (the search ended at …/b/node --flag …)`), then restored, `cargo fmt
--check` clean and the composite module green (99 passed). M1, M3–M6 and
M8–M11 mutate lines the refactor did not touch.

Fixture-component removal is the separate positive control (`removed-inherited`
and `removed-explicit` cells, 14 layouts; the doctor's 14 `[removed]`
cells), each with a fresh native oracle running the same B — or A through
the fitting padded spelling. Not re-mutated here: the third hold's access,
Apple-default, FreeBSD, backslash, absent-as-empty, unconditional
absent-PATH, env-alias, pnpm and head/Node removals keep their dated
records; the Node positive's two removals stay pending with the positive.

**Gates on the restored candidate** (`git diff` after the last restoration
shows only the repair, four files):

| Check | Actual outcome |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no output. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Exit 0, `Finished`, no warning (one `type_complexity` in the new doctor test was fixed by a struct before the final run). |
| `cargo test -p brokkr-core --all-features --locked` | 73 + 3 + 8 + 2 passed, 0 failed. |
| `cargo test -p brokkr-store --all-features --locked` | 58 + 1 + 1 + 1 + 1 + 2 passed, 0 failed. |
| `cargo test -p brokkr-protocol --all-features --locked` | 377 + 99 (2 ignored) + 1 passed, 0 failed. |
| `cargo test -p brokkr-runtime --all-features --locked` | 441 + 6 + 1 + 6 + 2 + 3 + 6 + 5 + 2 + 3 + 7 + 3 + 3 + 2 + 3 + 13 + 2 + 7 + 6 + 3 passed, 0 failed. |
| `cargo test -p brokkr-view --all-features --locked` | 243 passed (3 ignored), 0 failed. |
| `cargo test -p brokkr-bridge --all-features --locked` | 13 passed, 0 failed. |
| `cargo test -p brokkr-cli --all-features --locked` (25-minute timeout) | 463 + every integration suite passed (`doctor_dsh_selection` 14), 0 failed, no hang. |
| `cargo test --workspace` (25-minute timeout) | First run: 16 `adapters::tests::*` failures in `brokkr-protocol`, all `could not invoke the agent CLI: Text file busy (os error 26)` — the #255 ETXTBSY race in adapter launch fixtures, outside this slice; second run: every binary `test result: ok`, 0 failed, no hang. Both recorded. |
| `cargo test --workspace --all-features --locked` (25-minute timeout) | Every binary `test result: ok`, 0 failed, no hang. |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` / `bundles/verify` | Both compiled; the plan JSON printed. |
| `openspec validate --all --strict` | NOT EXECUTED: the seat refuses the `openspec` and `npx` commands (approval denied). Pending. |
| `TMPDIR=/tmp bash scripts/coverage-exact.sh` | The literal script launch is refused by the seat (script files), as in the third hold. The script's own three cargo steps were run by hand on the restored bytes with the unchanged pin (`cargo +nightly-2026-09-05 llvm-cov clean --workspace`; `… llvm-cov --workspace --all-features --locked --branch --json`; `… llvm-cov report --branch --lcov`) and its LCOV rule applied through the retained transcription `.forge/lcovtool`, with the script's `jq` harness-leak check (`true`). Reports under `.forge/scratch/coverage/` (`coverage.json`, `lcov.info`, the three step logs), outside the commit. **Lines 32216 / 32216 (100%), branches 5424 / 5424 (100%), functions 3122 / 3122 (100%)** on the final committed bytes; 68 `test result: ok`, 0 `FAILED`, no `--ignore-run-fail`. The first pass on the pre-refactor bytes measured 32213 / 32217, 5430 / 5432, 3119 / 3119 — four dead lines and two dead branches in the new resolver, removed by the refactor named in the removal section, not by exclusion. The denominator moved from the third hold's 32069 / 5396 / 3108 by this delivery's own production lines. The literal script and CI's `coverage-exact` job on the final head remain the gate's own artifact and are pending until they run. |

**Residual, for the controller — not in this slice's scope.** Doctor's
OTHER provider probes are bare `Command::new(<declared binary>)` calls
(`tool_version`, `probe_providers`): under `PATH=""` the exec adapter's `sh`
probe executes a `sh` sitting in the working directory exactly as a native
child does. The reconciled cwd rule is applied to DSH and Node selection
only; the `absent_path_default_search…` present-empty cell removes its `sh`
decoy for that reason and proves the DSH selection with a `dsh` decoy.

**Pending, recorded and not claimed.** Native macOS execution and the
immutable Apple `exec.c`/`posix_spawn.c` revision pin (no network in this
seat); native Windows matrix/doctor/`GetBinaryTypeW` and the Windows 1.88
MSRV build; the absent-PATH Node positive and its two removals (no `node` on
this host's `/bin:/usr/bin`); `openspec validate`; the literal coverage
script; remote CI on the final head; 8.8.2.1's env-alias qualification and
8.8.3.1's pnpm container structure, which this visit did not touch. No local
clause changes state: 8.8.1.1 and 8.8.1.2 stay open on the Apple pin and
native macOS, 8.8.2.1/8.8.2.2 on their carry-overs and native platforms,
8.8.3.1 untouched, 8.8.8.1 on the Node positive, 8.8.8.2 on `openspec
validate`, 8.8.8.3 on the literal script, 8.8.8.4 with them. Part (d),
8.10, 9.6, 10.6–10.8, 11.1–11.4, groups 14–15 were not touched; 8.8 stays
unchecked; 0056 stays proposed; `contracts/`, `policy/`, `fixtures/`,
`reference/`, `extensions/dsh/`, `docs/decisions/` are byte-identical
(`git diff --stat` over them is empty); no push.

### Analyze-return repair — A1, 2026-09-21 (Europe/Sofia)

This return adopts `af0065a7` and all its branch ancestry, including
`314ed02b`, and answers the supplied analyze finding **A1 — LOW,
INCONSISTENCY** in its owning artifact. The opening inventory now labels
115 as numbered task rows (89 complete / 26 pending), distinct from the
complete file's 133 checkbox rows (105 complete / 28 pending). The difference
is 18 retained historical rows (16 complete / 2 pending). All checkbox rows,
task identifiers, ticks, requirement coverage, execution order and earlier
accounts remain unchanged. No upstream artifact amendment is needed.

Fresh strict validation of this change and `openspec validate --all --strict`
passed (15 items / 0 failures for all-item validation); the inventory,
historical-byte preservation, frozen/excluded scope and whitespace checks
passed. Format, clippy, all seven crate-scoped suites, both workspace suites,
and self/verify bundle compilation were attempted and each exited 127:
`cargo: command not found`. The unchanged exact-coverage command also exited
127 at line 33's clean step; no Rust test, fresh instrumented build or report
ran. Fresh covered/total lines, branches and functions are all **unavailable**
(`null` on each axis). Commands, statuses and logs are retained under
`.forge/tasks-a1-cf3c0ab9/`, outside the commit.

Delivered here: the A1 inventory-scope repair only. The breakdown remains
drafted; implementation, native/removal proofs and Rust/coverage gates remain
pending under their existing owners. 8.8 stays unchecked, 0056 stays proposed,
and the security hold remains open. Frozen/excluded surfaces are unchanged;
no archive or push occurred.

### Historical fourth-hold disposition — design return, 2026-09-20

This account supersedes conflicting completion descriptions in the dated
records below without rewriting those records. It describes the design seat's
work and evidence; implementation gates remain owned by the numbered clauses.

| Chief finding / retained work | Current disposition and owner |
|---|---|
| R1 HIGH / implicit cwd | Open in production. Design now specifies literal cursor control and ordered refusal; 8.8.1.1–1.2/8.8.2.2 own both forms, all nine cells, explicit-empty controls and independent advance/cwd removals. |
| R2 HIGH / extra slash | Open. Byte assembly, padded-directory cases and same-fixture one-slash controls bind both named regressions under 8.8.1.1–1.2/8.8.2.2. |
| R3 / ignored parent/container structure | Open. Existing line/scalar guards are adopted; 8.8.3.1 owns scalar-parent/mixed-kind refusals, lawful nested positives and separate structural removals. |
| R4 / invented long-name refusal | Open. 8.8.1.1/8.8.2.2 preserve native causes under missing, file and existing-directory prefixes plus direct-path control. |
| R5 / exhaustion cause | Open. 8.8.1.1/8.8.2.2 retain final cause/candidate separately from denial, with independent exact-cause and precedence removals. |
| R6 / Apple operation | Open; source evidence only. 8.8.1.1/8.8.2.1–2.2 distinguish outer forms, nested env exec and successful-stat denial. Native macOS remains pending. |
| R7 / unasserted oracle cells | Open. Both forms need completed identity/cause assertions and no markers; 8.8.1.2/8.8.2.2 retain an independent inventory. Logged/PENDING cells are not passes. |
| R8 / supported env alias | File comparison adopted; blanket non-env rejection is insufficient. 8.8.2.1 owns actual successful alias qualification, specific A obstruction and same-file removed-A positive. |
| R9 / F9 literal coverage | Pending. The chief's literal run exited 101 before reports. Its counts were unavailable; retained 31893/32069 lines, 5382/5396 branches, 3098/3108 functions and hand-transcribed perfect reports remain historical. 8.8.8.3 still lacks a fresh literal pass. |
| R10 / native and Node-positive evidence | Pending under 8.8.1.2/8.8.2.2/8.8.8.1. Native macOS/Windows/MSRV and actual retained-Node positive plus two removals are unproved here; final-head remote CI is separately pending. |
| R11 / independent removals | Open under 8.8.1.1/8.8.3.1/8.8.8.1: remembered EACCES, separate scalar/separation/Unicode guards and home-only premise each retain their own intended failing assertion. |
| Launcher head and groups 4–7 | Adopted implementation/evidence. Preserve both rewriting regressions and the retained observation; no completed group is reimplemented or newly certified by this return. |

Delivered here: revised design, explicit reconciliation of both council
positions and dependency corrections to the existing task clauses. No Rust
source, native sentinel or enforcement mutation changed or ran. No task tick
changed, and the security hold remains open for implementation and proof.

Fresh design-seat validation on Linux x86_64 / glibc 2.42, adopted source
`d9297641` plus these two documentation edits:

- Strict change validation and `openspec validate --all --strict` exited 0;
  all-item validation reports **15 passed / 0 failed**. Informational archive
  refusals for absent living `adapter-resume-safety` and `sdd-progress-markers`
  remain whole-change fold debt; this non-archiving return does not resolve it.
- Format, locked all-target/all-feature clippy, all seven crate-scoped suites,
  both workspace test forms and self/verify compilation were each attempted;
  every command exited **127**, `cargo: command not found`. No Rust test ran,
  and no compiler version or compiled target was available to record.
- The unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` was attempted
  directly. It exited **127** at its clean step, line 33, because Cargo is
  missing, before any fresh instrumented build or report. Fresh source lines:
  **unavailable**; branches: **unavailable**; functions: **unavailable**.
  No existing target report supplies these counts. The coverage script, CI
  and release workflow all still consume `rust-nightly-version.txt`.
- `git diff --check`, required-design-section checks, the derived
  **20 requirements / 219 scenarios** inventory and exact task-ID/state
  comparison passed. Dated task execution bytes below remain unchanged;
  no frozen/excluded file differs from the adopted head.

Exact commands and outputs are in
`.forge/design-fourth-hold-345a3e7e/validation.json` and adjacent logs, outside
the commit. These are blocked local gate attempts, not candidate passes.
Production repair/proof and fresh capable-host coverage remain outstanding;
(d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 remain excluded.
0056 stays proposed; 8.8 stays unchecked. No archive, push or remote result.

### Tasks-phase reconciliation and validation — fourth hold, 2026-09-21 (Europe/Sofia)

This tasks seat adopted `slice-dsh-composite-b` at `6b3c66b4`, including all
branch history and `314ed02b`. It read the rendered tasks/return dialect,
README, decisions 0004/0005/0009 and proposed 0056, both controller lookup
records, fourth-hold intake, the full R1–R11 chief finding, proposal AR/AS1,
D10 and the current production/test surfaces. The active dated change was
already open. No supplied `returned_from` added another finding, and no
upstream specification or design gap was found.

Delivered in this phase: an explicit ordered handoff for all fourteen local
tasks, requirement/scenario coverage, independent proof ownership, prospective
per-form cwd-cell inventory and exact focused commands for the two mandatory
regressions. The stale instruction to introduce six length layouts was
corrected: those layouts already exist in adopted `314ed02b` and need their
cwd and per-form comparisons extended. No new implementation, native oracle
execution, compiling mutation or task completion is claimed.

Fresh validation on adopted `6b3c66b4` plus this `tasks.md` edit, Linux x86_64 /
glibc 2.42:

| Check actually attempted | Observed result |
|---|---|
| `openspec validate 2026-09-09-226-session-resumption --strict` | Exit 0; valid. Existing informational notices concern the absent living safety/progress archive targets; this active partial slice performs no archive. |
| `openspec validate --all --strict` | Exit 0; **15 passed / 0 failed**. |
| `cargo fmt --all -- --check`; locked workspace/all-target/all-feature clippy with `-D warnings` | Each exited 127: `cargo: command not found`. |
| All seven crate-scoped all-feature locked suites; `cargo test --workspace`; all-feature locked workspace suite | Each exited 127 because Cargo is unavailable; **zero Rust tests executed**. |
| Locked self and verify bundle compilation | Each exited 127 because Cargo is unavailable; no bundle compiled. |
| Unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` | Exit 127 at line 33's clean command because Cargo is unavailable; no fresh instrumentation or report. |
| Fresh covered/total source lines; branches; functions | **Unavailable / unavailable; unavailable / unavailable; unavailable / unavailable** (`null` on all three axes), not zero or retained historical counts. |
| Task/requirement/history/scope audit and diff whitespace | Passed: 20 requirements / 219 scenarios; 14 local states unchanged (5 complete / 9 pending), 101 change-wide states unchanged (84 complete / 17 pending). Every local checkbox names safety / AS1. All third-hold and older delivery records remain byte-identical. |

Exact commands, statuses and output logs are recorded in
`.forge/tasks-fourth-hold-ce31d14f/validation.json` and adjacent files, outside
the commit. Cargo, rustc and rustup are unavailable through this seat's
workspace hands; no compiler/native target result is inferred. CI, release
admission and the unchanged exact-coverage script all still consume
`rust-nightly-version.txt`, whose value is `nightly-2026-09-05`.

Only `tasks.md` changed. No frozen contract, policy/schema, fixture, reference,
extension, decision, production or test byte moved. 0056 remains proposed and
8.8 remains unchecked. The breakdown is drafted; the security hold remains
open for loop/byte/cause, supported-env and pnpm-structure repair, paired native
and independent removal proofs, retained-Node evidence and the actual Rust
and fresh literal coverage gates. Native macOS/Windows/MSRV and final-head
remote CI remain pending. Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and
groups 14–15 remain outside this visit; no archive or push occurred.

### Historical implementation delivery — third security hold, 2026-09-20

This implement seat executed the breakdown above on `slice-dsh-composite-b`
(adopted through `6e8dd5b5`, every commit kept) in worktree `brokkr-wt-dsh88b`,
Linux x86_64, target `x86_64-unknown-linux-gnu` (glibc), stable cargo 1.98.0
for the gates and `nightly-2026-09-05` for coverage. The account separates
what ran from what remains pending; it describes outcomes and directs no
gate. Removal transcripts are in `.forge/third-hold-removals.md` and the gate
logs under `.forge/gate-*.log`.

**R1 — the oracle is the specification.** `Search::find` now has the
library's own stages in the library's order: the name is admitted before any
entry is read (`admit_program_name`: glibc/musl refuse more than `NAME_MAX`
with ENAMETOOLONG, `posix/execvpe.c` 92–106); each component is sized as the
library sizes it before a candidate is joined (`construction`: glibc/musl
skip a component of `path_len = strnlen(path, PATH_MAX − 1) + 1` bytes or
more, lines 112–119, with no errno; Apple's `posix_spawnp` stops when
directory + name + 2 exceed its 1024-byte buffer, D10's `sys/posix_spawn.c`
125–127); only a constructed candidate reaches the kernel, and only the
kernel's errno reaches the switch (`step`: glibc continues on EACCES
remembered, ENOENT, ESTALE, ENOTDIR, ENODEV, ETIMEDOUT, lines 136–158 and
165–168, and stops on everything else — ENAMETOOLONG, ELOOP, EIO, EINVAL
included; musl on EACCES, ENOENT, ENOTDIR; Apple on ELOOP, ENAMETOOLONG,
ENOENT, ENOTDIR with EACCES remembered, and any arm this seat could not pin
is a named limitation rather than a guess). `effective_exec_access` answers
the kernel's errno instead of a boolean; `lookup_failure` decides each
operation — metadata, access — on its own errno and names the operation in
the refusal. A terminal cause wins over an earlier denial (the refusal
returns at once); exhaustion reports the remembered EACCES. The compiled
target selects the library (`LIBRARY`); every rule is a pure table a Linux
run tests arm by arm, and `Search` carries the library so the whole search
under another library's rule is a plain test too. Metadata ENAMETOOLONG is
what the switch says it is — terminal on glibc — and never a stand-in for
the buffer skip.

**R2 — target rules are separate.** `default_search_of` keeps Apple's
`/usr/bin:/bin` (Apple `include/paths.h` 63), adds FreeBSD's own
`/sbin:/bin:/usr/sbin:/usr/bin:/usr/local/sbin:/usr/local/bin` (FreeBSD
`include/paths.h` 36–40), keeps glibc's `confstr` and musl's literal, and
answers `Unestablished` — an explicit refusal, no other target's literal —
for Android's bionic, the other BSDs, illumos and a Linux libc that is
neither glibc nor musl. The ELOOP rule follows the same table: glibc stops,
Apple continues, an unestablished library refuses. Apple's complete
`posix_spawnp` switch beyond the arms D10 pinned is NOT pinned by this seat
(no network in the seat; `exec.c`/`posix_spawn.c` could not be fetched), and
that limitation is spelled in the code as a refusal, not a fallback.

**R3 — env identity is the file.** `is_env(interpreter, metadata, reference)`
compares device and inode with `/usr/bin/env`, and for a copy compares equal
lengths and streaming bytes in 8 KiB buffers on the file that is open,
checked to be the file that was inspected. `env_program` admits only the
established invocation — the platform's env file under the name `env` — and
refuses, before any probe, a copy or link under another name (the dispatch a
multicall `env` makes on argv0 cannot be established without executing it)
and an impostor named `env`; neither is `Ok(None)` admission.

**R4 — ignored pnpm syntax is complete.** `pnpm_ignored` tracks member
position and quote state: `{}`, `[]` and one trailing comma remain valid,
leading/interior missing members and comma-only bodies refuse by position, a
flow map's members must be `key: value` entries and a flow sequence's must be
scalars (`split_flow_entry` keeps a `: ` inside a quoted scalar to the
scalar). `pnpm_ignored_line` admits every line under an ignored section and
under a block-form package child as a mapping entry or sequence item with a
scalar key and an admitted value, naming the section or child in the refusal;
the raw identity bytes, single read and 8,388,608-byte bound are unchanged.

**F6/R5 — retained head and Node.** The retained-head repair stands (renewed
by removal R9). The protocol companion
`absent_path_node_identity_is_retained_by_the_composite` compares native
`node -p process.execPath` with the private `Selected.node` and asserts the
composite consumes that field (a distinct retained runtime moves the `node`
line and the digest); the doctor Node arm additionally asserts a readable
composite. This host has no `node` on its default search (`/bin:/usr/bin`),
so both record the refusal arm and the POSITIVE with its two removals is
PENDING, not passed.

**R6 — every cell has its oracle.** The matrix moved to
`composite/tests/native_matrix.rs`: 8 names × 26 layouts on Linux (24
elsewhere) = 208 cells, each run in the INHERITED form (the parent stages the
layout's `PATH` in a child test binary's environment; `Command::new(name)` and
`resolve_executable` read the same environment, the `posix_spawnp` path) and
the EXPLICIT form (`PATH` set on the `Command`, `fork`/`execvp` per Rust 1.88
`unix.rs` 417–423; asserted equal on glibc, recorded elsewhere), plus a
removed-component oracle for the eight length/file/nonexistent layouts and
five named controls (overlong bare name, overlong explicit path, the NAME_MAX
boundary, a 255-byte positive, the `sh` default-search positive): 453 oracles
on this host. The child prints one `matrix-oracle:` line per completed native
outcome and the parent asserts that set equals the declared inventory;
unidentified output or an unsuccessful sentinel exit is a panic. The six
lengths are individual layouts, each asserted on glibc by its own expected
outcome (255/4096/5000 → B identity; 256/300/4095 → errno 36 and the
`metadata answers File name too long (os error 36), on which the platform's
lookup stops` refusal). Tally on this host: 79 equal selections, 39 NotFound
parities, 43 terminal-error parities, 26 NUL refusals, 21 D10 loader
exceptions recorded separately.

**Tests added or strengthened.** Protocol: `native_matrix::native_executable_resolution_matches_command_matrix`,
`the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`
(six length cells with native oracles and removed-component controls, the
overlong bare/explicit names with their oracles, the 255-byte positive),
`the_lookup_rule_is_each_librarys_own_switch_arm_by_arm` (every arm of every
library, construction arithmetic, name admission, `lookup_failure` by
operation, the whole search under Apple/musl/unestablished rules),
`env_identity_is_the_file_and_never_a_name`, the rewritten
`an_env_argument_is_selected_as_the_kernel_hands_it_to_env` (system env,
symlink named `env`, byte copy named `env`, `env-alias`, `uu_env` hard link,
impostor; obstructed and valid chains; native control each),
`each_platforms_absent_path_search_is_its_own_loaders_rule` (FreeBSD,
unestablished rows), `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`
(the five commissioned member vectors, mapping/sequence structure, body
lines under `peerDependencies`, `snapshots`, `importers`, `settings`, and the
readable body controls), `absent_path_node_identity_is_retained_by_the_composite`.
Built doctor (`doctor_dsh_selection.rs`): `terminal_path_lengths_refuse_before_doctor_probe`
(six lengths, per-length oracle and doctor marker directories, removed-component
control each), `apple_default_search_excludes_confstr_only_directories`
(macOS only; not run here), the rewritten env regression (six spellings,
markers checked before lines), the strengthened Node arm, the R4 vectors in
`ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor`.

**Removal proofs** (each a compiling mutation of the enforcing line, the
named regression, the failed assertion, exact restoration, green rerun;
transcripts in `.forge/third-hold-removals.md`):

| Removal | Named regression → failed assertion |
|---|---|
| R1 blanket ENAMETOOLONG continuation (`step`, glibc arm) | table test `step(Glibc, NAMETOOLONG)` `Continue` ≠ `Stop`; classifier `expected a refusal` at the 256-byte cell; matrix `n0-l14/inherited` `ENAMETOOLONG is named as the stop it is`; doctor `256: doctor executed B where native lookup stopped` — marker `["DSH_B_LENGTH_SENTINEL_0.0.6"]` |
| R2 pre-buffer skip removed (`construction`) | table `construction(Glibc, 5010, 5000, 3)` `Try` ≠ `Skip`; classifier 4096 cell refused where native ran B; matrix `n0-l15/inherited` `the resolver selects exactly the file the child ran` (cwd file after a 4096-byte skip); doctor `4096: doctor probed exactly B` — `[]` |
| R3 access errors erased (`effective_exec_access`) | `path_resolution_walks_past…` `Err(EACCES)` ≠ `Err(ENOENT)` |
| R4 terminal precedence erased (`find`, refused walked past) | classifier `expected a refusal`; matrix `n0-l8/inherited` `the resolver admitted the obstructed A`; doctor obstruction `doctor did not probe B in place of the obstructed A` and lengths `256: doctor executed B` |
| R5a FreeBSD default = Apple's | table `Literal("/usr/bin:/bin")` ≠ FreeBSD literal |
| R5b Apple ELOOP on glibc | table `step(Glibc, LOOP)` `Continue` ≠ `Stop`; classifier `expected a refusal`; matrix `n0-l9/inherited` `selected a file where the child stopped` (errno 40); doctor ELOOP arm probed B |
| R5c Apple default = confstr | table `macos` `Library` ≠ `Literal("/usr/bin:/bin")` |
| R6 absent PATH as empty entry (S1) | matrix `n0-l2/inherited` selected `cwd/dsh` where native found nothing; doctor `doctor executed the cwd dsh under an absent PATH` |
| R6b backslash separator (S1b) | matrix `n4-l0/inherited` selected `cwd/C:\Tools\dsh.exe`; doctor `doctor executed the cwd C:\Tools\dsh.exe with PATH None` |
| R7 basename env admission (spelled or canonical, alias followed) | protocol `expected a refusal` at `tools/uu_env` (launcher admitted); doctor `doctor probed nothing under "…/tools/uu_env"` — marker `["DSH_B_NODE_0.0.3"]` |
| R8a empty-member elision | protocol `[true,,false]` lock `was accepted`; doctor `cpu: [,x64]` reported the control's composite |
| R8b ignored-body bypass | protocol `snapshots … ms: '2.0.0` `was accepted`; doctor `peerDependencies` body reported the control's composite |
| R8c control-byte guard bypassed (renewed) | protocol NUL tarball `was accepted`; doctor NUL vector reported the control's composite |
| R9 post-probe reread (renewed) | protocol `expected a refusal` for the self-rewriting launcher; doctor `the observation reads the first line selection inspected` |
| R10 unconditional absent-PATH refusal | matrix `n1-l2/inherited` `Err("PATH is absent")` where native ran; `an_absent_path_is_a_named_refusal…` and the Node companion's refusal arm; doctor `the default-search sh, selected and silent` (the sh positive). Node positive half PENDING |
| R11 executable/home flattened | seams test `expected a refusal` at `resolve_with("dsh", None, [], None)` |
| R12 harness: one name's oracles omitted | matrix `every declared cell invoked its oracle and no other did; missing ["n3-l0/explicit", …]` |
| R13a unconditional native-image admission (renewed) | matrix `n0-l24/inherited` `admitted the obstructed A`; doctor `a native image with a missing loader:` A selected and probed |
| R13b one-level interpreter admission (renewed) | classifier retained node `None` ≠ `Some(…/b/node)`; matrix `n0-l24/inherited` admitted A; doctor `a script whose interpreter has a missing loader:` A probed |

Not re-mutated here: the second hold's scalar guards (quote closing,
indicator refusal, typed plain scalar, header/outer separation, NBSP
preservation) keep their dated records; the wrong-Node retention and the
absent-PATH refusal against the Node POSITIVE are pending with the positive.

**Gates on the restored candidate** (`git diff` after every removal shows
only the repair; the tree at the gate runs is the tree committed):

| Check | Actual outcome |
|---|---|
| `cargo fmt --all -- --check` | Exit 0, no output. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Exit 0, `Finished`, no warning. |
| `cargo test -p brokkr-core --all-features --locked` | 73 + 3 + 8 + 2 passed, 0 failed. |
| `cargo test -p brokkr-store --all-features --locked` | 58 + 1 + 1 + 1 + 1 + 2 passed, 0 failed. |
| `cargo test -p brokkr-protocol --all-features --locked` | 377 + 99 (2 ignored) + 1 passed, 0 failed. |
| `cargo test -p brokkr-runtime --all-features --locked` | 441 + 6 + 1 + 6 + 2 + 3 + 6 + 5 + 2 + 3 passed, 0 failed. |
| `cargo test -p brokkr-view --all-features --locked` | 243 passed (3 ignored), 0 failed. |
| `cargo test -p brokkr-bridge --all-features --locked` | 13 passed, 0 failed. |
| `cargo test -p brokkr-cli --all-features --locked` | 463 + integration suites (`doctor_dsh_selection` 14) passed, 0 failed. |
| `cargo test --workspace --all-features --locked` (15-minute timeout) | 75 `test result: ok` lines, 0 `FAILED`, no hang. |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` / `bundles/verify` | Both compiled; the plan JSON printed. |
| `openspec validate --all --strict` | NOT EXECUTED: the seat refuses the `openspec` and `npx` commands (approval denied). Pending. |
| `TMPDIR=/tmp bash scripts/coverage-exact.sh` | The literal script launch is refused by the seat (script files), and `/tmp` is a 31 GB tmpfs with 5.9 GB free, so the script's own steps were run by hand on the final bytes, unchanged pin and unchanged rule: `cargo +nightly-2026-09-05 llvm-cov clean --workspace`, then `cargo +nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch --json --output-path .forge/coverage-tmp/coverage.json` (fresh instrumentation in the default `target/llvm-cov-target` after the clean; 68 `test result: ok`, 0 `FAILED`, no `--ignore-run-fail`), then `cargo +nightly-2026-09-05 llvm-cov report --branch --lcov --output-path target/coverage/lcov.info`, the script's `jq` harness-leak check (`true`), and the script's LCOV rule — every `DA` and `BRDA` hit, every logical function by file + `FN` start line — transcribed in `.forge/lcovtool/src/main.rs` because the seat refuses `awk -f`. Reports preserved at `target/coverage/coverage-exact.json`, `target/coverage/lcov.info`, `target/coverage/coverage-summary.json`. **Lines 32069 / 32069 (100%), branches 5396 / 5396 (100%), functions 3108 / 3108 (100%)**, exit 0, on the candidate bytes committed below (working tree of `6e8dd5b5` plus this delivery). A first run on the pre-final bytes measured 32067/32069, 5393/5396, 3108/3108 — three pnpm branches in `pnpm_ignored`, `split_flow_entry` and `split_mapping_key` — closed by three vectors (`engines: {'a'b: 1}`, `os: ["a: b"]`, an unterminated quoted key in a `peerDependencies` body), not by exclusion. The denominator grew from the chief's 31848 lines / 5366 branches / 3088 functions by this delivery's own production lines; nothing was excluded. The literal script and CI's `coverage-exact` job on the final head remain the gate's own artifact and are pending until they run. |

**Pending, recorded and not claimed.** Native macOS execution (the six
lengths, ELOOP continuation, `apple_default_search_excludes_confstr_only_directories`,
the Mach-O loader), native Windows matrix/doctor/`GetBinaryTypeW` and the
Windows 1.88 MSRV build, the absent-PATH Node positive and its two removals,
Apple's complete `posix_spawnp` switch pinning, `openspec validate`, and
remote CI on the final head. No local clause changes state: 8.8.1.1 stays
open on the Apple pin, 8.8.1.2/8.8.2.2 on native macOS/Windows, 8.8.2.1 on
the same, 8.8.3.1 on the scalar-guard re-mutations, 8.8.8.1 on the Node
positive, 8.8.8.2 on `openspec validate`, 8.8.8.3 on the literal script,
8.8.8.4 with them. Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4, groups 14–15
were not touched; 8.8 stays unchecked; 0056 stays proposed; `contracts/`,
`policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`
are byte-identical; no push.

### Third-hold reconciliation of the prior delivery account — 2026-09-20

This is the current account, superseding conflicting claims in the dated
second-hold records below. Those records remain attributed history, not new
measurements or gate instructions. Chief run `09ec8d81` and its positions at
`.forge/results/9e0a2104-4a8a-43b0-a176-3a13b5183142-*` supply R1–R7;
this tasks seat performed no production mutation or native reproduction.

| Finding / earlier claim | Current disposition and task owner |
|---|---|
| R1 HIGH SECURITY / F1 continuation | Open. Withdraw the blanket ENAMETOOLONG and overlong-name ordinary-miss conclusions. The controller measured 5000/ENOENT/ENOTDIR; the chief measured the six boundaries. 8.8.1.1–1.2 and 8.8.2.2 require fresh independent oracles, exact cause/identity and no forbidden execution. The historical mutation removing all ENAMETOOLONG continuation does not prove the required split. |
| R2 / F2 defaults, prior F3 ELOOP | Open. Apple's constant stands; assigning it to every BSD and assigning Apple's ELOOP continuation to every non-Linux Unix do not. 8.8.1.1 separates target rules; 8.8.1.2 and 8.8.2.2 require native Apple discrimination and actual invocation/length/ELOOP proof. Linux table results certify no Apple/FreeBSD execution. |
| R3 SECURITY / F4 env aliases | Open. Canonical basename remains name recognition; the copied same-inode `env`/`uu_env` bypass needs actual file plus invocation identity and its removal under 8.8.2.1. A host utility rejecting an alias does not prove the required successful-alias control. |
| R4 SECURITY / F5 ignored syntax | Open. Existing control-byte/scalar guards stand, but empty members and skipped bodies remain unproved. 8.8.3.1 adds both independently, with positive locks and producer/doctor reason assertions. |
| Prior F6 retained launcher head | Adopt the implementation and dated evidence; preserve its head/Node carriers and rerun both self-rewrite controls and the independent reread removal under 8.8.2.1. Native Windows execution is not inferred from shared source. |
| R5 / F7 retained Node | Open. Withdraw launcher `process.execPath` as proof of the private retained Node. 8.8.8.1 requires that actual field, a readable composite and separate wrong-retention/absent-PATH removals. The host's prior NotFound outcome proves no Node positive. |
| R6 / F8 actual executions | Pending. 8.8.1.2, 8.8.2.2 and 8.8.8.1 require native macOS, native Windows matrix/doctor/GetBinaryTypeW and Windows 1.88 MSRV, and the actual Node positive/removals. None ran in this tasks phase. Each later result must name revision/target/compiler, not test presence or a passing skip. |
| R7 / F9 literal coverage | Pending. The chief's literal gate failed at 31672/31848, 5352/5366, 3078/3088. The prior hand-executed/retained perfect report remains historical, not gate completion. 8.8.8.3 requires the fresh unchanged script and actual three integer pairs on a capable host. |
| S1/S1b, separator/NBSP, executable/home and first-hold preservation | Remain binding. Groups 1–3 and 8.8.8.1 preserve the sound repairs and renew affected proofs; completed groups 4–7 remain byte-for-byte adopted with their dated evidence. No first-hold tick alone certifies a changed rule. |

### Tasks return validation — clarified third hold, 2026-09-20

This tasks seat adopts every commit through `76daef9d` and answers the
returned design's `drafted` result and its two informational archive notices.
The rendered tasks instructions, current Open Questions, AP/AQ, AS1, D10,
controller ENAMETOOLONG/S1/S1b/absent-PATH/ELOOP records, prior hold notes,
chief R1–R7 and historical F1–F9 delivery account support the existing scope.
Source inspection confirms the remaining lookup, env-name, ignored-syntax and
retained-Node proof gaps; it supplies no new runtime result.

Delivered only this planning return. The fourteen requirement-linked clauses
retain their identifiers/order and **5 complete / 9 pending** states; all
101 change-wide tasks retain **84 complete / 17 pending**, including unchecked
8.8. The lookup clause now establishes its oracle comparison/inventory and
focused boundary controls before lookup completion; 8.8.2.2 subsequently
completes the same matrix. Env admission explicitly binds D10's impostor and
unsupported-invocation controls to the existing named regressions. AQ's
command-ownership decision is recorded without copying commands back into AS1.
No new product ambiguity, scenario or earlier-artifact repair is required.

The current artifact audit in `.forge/tasks-return-84e6c869/` checks all
115 numbered checkbox occurrences (**89 checked / 26 unchecked**), every
local AS1 citation and ownership of all **31** current selection/security/
preservation scenarios. The five deltas remain **20 requirements / 207
scenarios**. It compares every tracked file against adopted `76daef9d`:
only this task artifact changes; the other **741** files, preservation groups
8.8.4–8.8.7, whole-change task bodies, groups 14–15 and historical delivery
accounts retain their bytes. Proposed 0056 and all frozen/excluded surfaces
remain unchanged.

Fresh validation, on Linux x86_64 with Cargo/rustc unavailable:

| Check | Actual outcome |
|---|---|
| `openspec validate --all --strict` | Exit 0; **15 passed / 0 failed**. |
| Strict active-change validation and artifact status | Exit 0; planning artifacts present. The two archive-target notices remain informational for this phase; archive readiness is not claimed. |
| `git diff --check` and artifact/state audit | Pass; only the declared task artifact changes and no checkbox state changes. |
| Format, locked all-target/all-feature clippy, seven separate locked all-feature crate suites, plain and locked all-feature workspace tests, self/verify bundle compiles | Each command attempted; each exits **127**, `cargo: command not found`. Zero Rust tests execute. |
| Unchanged `TMPDIR=/tmp bash scripts/coverage-exact.sh` | Fresh invocation exits **127** at line 33 during its Cargo clean step, before instrumentation or new reports. |
| Fresh source-line / branch / function coverage | **N/A/N/A / N/A/N/A / N/A/N/A**; all structured covered/total values and percentages are null. No old count is relabelled as a current result. |
| Native macOS, Windows/GetBinaryTypeW/MSRV, actual Node positives, removals and final-head remote CI | Not executed here; pending. |

Exact commands, exits, host/source state and output are retained under
`.forge/tasks-return-84e6c869/`. The immutable compiler-pin check confirms CI,
release admission and coverage consume `rust-nightly-version.txt`, still
`nightly-2026-09-05`. Missing Cargo prevents local Rust validation; the
namespace-dependent coverage additionally needs its prescribed capable host/CI.
Neither limitation changes the breakdown or waives delivery acceptance.

The tasks result is `drafted`, not implementation completion or security-hold
clearance. R1–R7 repairs and proofs, all green candidate suites and fresh
literal 100% coverage remain owed under the unchanged scope. The missing
living targets must be reconciled by their owning whole-change work before
archive, not by this partial return. This seat commits only `tasks.md`,
leaves the change active and makes no push or out-of-slice task change.

### Historical tasks-phase validation — third security hold, 2026-09-20

Delivered this requirement-linked breakdown and current evidence correction
only. D10's existing addresses are retained: three affected ticks reopened,
five preservation clauses remain checked, and nine local clauses await
implementation/proof. All 101 change-wide states remain 84/17, including
unchecked 8.8. No source, test, earlier artifact, decision or frozen byte changed.
The entire adopted branch history remains in place.

The artifact audit at `.forge/tasks-third-hold-efb3360b/artifact-audit.json`
checks the exact checkbox ID/order/state diff, all fourteen local AS1 citations
and an independent mapping of **31** current selection/security/preservation
scenarios to task owners. It finds **115** numbered checkbox occurrences
(**89 checked / 26 unchecked**) and only the three declared reopened ticks.
Groups 8.8.4–8.8.7 retain their exact bytes. The other **741** tracked files
are byte-identical to adopted `aeb92115`; global/excluded tasks retain their
states. The five deltas still contain 20 requirements and 207 scenarios.

Fresh checks/attempts are recorded under
`.forge/tasks-third-hold-efb3360b/`. `openspec validate --all --strict`
passes **15/15**, active-change strict validation passes, artifact status
reports the four planning artifacts present, and `git diff --check` passes.
Existing informational notices about missing archive targets remain
whole-change debt; no archive readiness or implementation success is inferred
from planning status. No build-affecting upstream gap was found.

Format, clippy, each of the seven crate suites, workspace tests and both
bundle compiles were attempted here and each exited **127: cargo: command
not found**. The unchanged literal
`TMPDIR=/tmp bash scripts/coverage-exact.sh` also exited **127** at line 33,
before instrumentation, for missing Cargo. Fresh coverage is **lines N/A/N/A,
branches N/A/N/A, functions N/A/N/A** (all percentages unavailable; structured
counts are null). No retained report is relabelled as current. Read-only
inspection confirms CI, release admission and the coverage script consume
the unchanged `nightly-2026-09-05` pin. No Rust gate, native oracle, provider
probe, removal proof or exact-coverage pass is claimed by this tasks phase.

The tasks outcome is `drafted`. Rust repairs, actual Node-positive/removal
proof, native macOS/Windows/MSRV, full green candidate suites and fresh
capable-host literal exact coverage remain implementation obligations; remote
final-head CI remains pending until observed. The security hold is not cleared.
Only this planning artifact is committed; there is no push, archive, provider
enablement or release action.

### Historical implementation delivery — second security hold, 2026-09-20

This implement seat executed the breakdown above on
`slice-dsh-composite-b` (adopted through `3e18f2c9`), in D10's order. The
account below separates what is delivered from what remains pending; it
describes outcomes and directs no gate.

**Finding 1 — the platform's lookup rule.** `resolve_executable_in` now
applies `std::process::Command`'s rule and nothing else. A NUL-bearing or
empty name refuses before any filesystem, home or search work. On Unix the
only path predicate is a literal `/` (`is_explicit_path`); a backslash,
drive spelling, extension or space is an ordinary filename byte and goes
through native search. The search context is captured once (`Search`): a
present `PATH` is split with its ordered empty entries; an absent `PATH`
consults the C library's default search path through a local read-only
`confstr(_CS_PATH)` binding (glibc and Apple libc report there what their
`execvp` searches; musl's literal is used on musl), never a hardcoded
host answer and never a subprocess. A no-match names which search failed
(`'dsh' is not on the default search path /bin:/usr/bin (PATH is absent)`
versus `'dsh' is not on PATH`). `selected_executable` consumes the
established selection through the same predicate, checks the file is still
a regular executable file and canonicalizes it once more; it performs no
second search. A Windows implementation of Rust's `resolve_exe` (child
`PATH` when the child environment changed, application directory, system
and Windows directories, parent `PATH`; `.exe` suffix rules; batch
dispatch refused; bounded PE admission) is written under `cfg(windows)`
and is **unexecuted on Windows** in this seat — see pending.

**Finding 4 — loading evidence, not metadata.** A new private module
`crates/brokkr-protocol/src/adapters/composite/image.rs` reads loading
declarations with every offset, count and range checked: ELF (class, byte
order, version, type, machine, bounded program-header table, load-segment
sizes, at most one well-formed NUL-terminated `PT_INTERP` read as the
kernel reads the C string), Mach-O (64-bit thin and universal, CPU type,
executable or `MH_DYLINKER`, bounded load commands, at most one
`LC_LOAD_DYLINKER`) and PE (signature, machine, executable-not-DLL,
optional-header magic and console/GUI subsystem). Every format parses on
every target by its magic — a Mach-O on Linux is a parsed image refused for
being another target's — so all three readers are live and unit-tested on
Linux with synthetic images (`composite/image/tests.rs`). `classify_in`
then establishes a candidate's prerequisite before admission: a `#!`
interpreter is followed as a candidate in its own right (device/inode
chain, loop and depth-4 refusals, the measured `env <program>` form
selecting the program under the same search, option languages refused), a
native image's loader must exist, be executable and parse as a loader of
the same format, and a head that is neither a script nor a native image is
refused by name. The refusal is D10's one named exception to native
equality; nothing is emulated or trial-executed. The matrix run recorded
one divergence between the first reader and the kernel — a NUL-padded
`PT_INTERP` — and the reader was aligned to the kernel's rule.

**Finding 2 — syntax before trimming.** One private helper,
`pnpm_separated`, decides `key:`/`key: value` separation consuming only the
grammar's ASCII space padding; the header, every top-level key, every
package child (read or ignored) and every flow field use it. Plain `: `
inside a flow scalar refuses as unsupported flow syntax in every field, the
ignored `tarball` included. `lockfileVersion:9.0`,
`resolution:{integrity: sha512-X}` and
`resolution: {integrity: sha512-X, tarball: x: y}` each refuse through the
sole producer by their own cause; the separated plain/quoted controls and a
colon-without-space URL still read to the control's composite.

**Finding 3 — identity bytes are never trimmed.** No `str::trim` remains on
any path feeding a scalar or key; blank-line detection is ASCII-space only.
U+00A0 (and U+2003) at either edge of a plain, single-quoted or
double-quoted integrity reaches `'debug@2.6.9': integrity carries
whitespace` through the sole producer, and a U+00A0 on a blank line, after
a section, beside a key or inside a flow map is a refusal of its own.

**Finding 5 — the failing premise.** The home-only premise is gone.
`dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one` asserts the
real selection against the real lookup, drives the four
executable/home combinations through `selected_from`/`resolved`, and runs
a real child with `HOME=/tmp`, `PATH=/usr/bin:/bin` and the three overrides
unset, where native lookup decides: on this host the child's
`Command::new("dsh")` is NotFound, selection refuses `'dsh' is not on
PATH`, and resolution refuses the same with the home present.

**Tests added or extended (all on the final bytes):**

| Surface | Tests |
|---|---|
| `composite/tests.rs` | `native_executable_resolution_matches_command_matrix` (child-process; 8 names × 14 layouts = 112 cells: 60 equal selections, 14 NotFound parities, 8 terminal-error parities, 14 NUL refusals, 16 D10 loader exceptions recorded separately, plus the `sh` default-search positive with a cwd decoy); `a_native_images_loader_is_read_as_the_kernel_reads_it`; `the_default_search_path_is_the_c_librarys_own_answer`; `an_absent_path_is_a_named_refusal_and_never_the_working_directory` (rewritten: `sh` positive by `/proc` identity, `dsh`/`node` asked of the host, `node` identity by `process.execPath` when present); `executable_resolution_walks_path_entries_and_refuses_a_miss` (backslash name searched and found); `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` (loader, env, chain, foreign-image arms); `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason` (header/outer/child/top-level/tarball vectors and separated controls); `pnpm_integrity_preserves_unicode_whitespace_for_refusal`; `dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one`; `the_producer_refuses_a_bare_executable_spelling`. |
| `composite/image/tests.rs` | Every ELF, Mach-O, universal and PE rule by name, the failing-device arms, and this test binary as the native positive. |
| `brokkr-cli/tests/doctor_dsh_selection.rs` | `absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel` (default-search reason, marker directory); `unix_backslash_names_follow_native_lookup_before_doctor_probe`; `absent_path_default_search_matches_native_dsh_and_node`; `an_obstructed_path_search_takes_the_explicit_safe_refusal` (native missing-loader image and interpreter-with-missing-loader cells, patched `PT_INTERP` on a copy of the built binary, native child runs B, doctor probes nothing). |

**Removal records** — each a compiling mutation applied alone, the named
test run with `cargo test -p <crate> --all-features --locked <name>`
(one test selected each time), the intended assertion failing, the exact
inverse edit, and a green rerun:

| # | Mutation (exact) | Named test and failing assertion | Restored rerun |
|---|---|---|---|
| R1 | `Search::capture` `None` arm: `entries: OsString::new()` (absent PATH as one empty entry) | `absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel` panicked at its first assertion, `doctor executed the cwd dsh under an absent PATH`, stdout `ok dsh: SECURITY_CWD_SENTINEL_9f3`; the matrix failed its `dsh`/absent-PATH cell (`expected a refusal`). | both `ok` |
| R2 | Unix `is_explicit_path`: `command.contains('/') \|\| command.contains('\\')` | `unix_backslash_names_follow_native_lookup_before_doctor_probe` panicked `doctor executed the cwd C:\Tools\dsh.exe with PATH None`, stdout `ok dsh: SECURITY_BACKSLASH_CWD_SENTINEL_9f3`; the matrix: `the resolver selected a file where the child found nothing: name "C:\\Tools\\dsh.exe", layout "cwd-only, PATH elsewhere", native NotFound, resolver Ok(.../cwd/C:\Tools\dsh.exe)`. | all 8 doctor-selection tests and the matrix `ok` |
| R3 | `resolve_executable_in`: `if path.is_none() && !is_explicit_path(command) { return Err("'{command}': PATH is absent") }` | `absent_path_default_search_matches_native_dsh_and_node` panicked `the default-search sh, selected and silent: warn dsh: binary 'sh' not found: ... 'sh': PATH is absent` while the native `sh` child had run; the matrix failed at `resolve_executable_in("sh", None).unwrap()`. | both `ok` |
| R4 | `native_obstruction`: `if len > 0 { return Ok(()) }` before inspection (unconditional native-image admission) | `an_obstructed_path_search_takes_the_explicit_safe_refusal` panicked on the missing-loader cell with the generic `warn dsh: binary '/…/a/dsh' not found — seats …`; the matrix: `the resolver admitted the obstructed A: … layout "A:B, A's interpreter has a missing loader", native Ok("c13s1")` (B ran). | both `ok` |
| R5 | `loading_obstruction`: interpreter recursion replaced by `Ok(())` (one-level metadata admission) | same two assertions as R4, on the interpreter-with-missing-loader cell. | both `ok` |
| R6 | `classify_in`: ELOOP and unprovable metadata arms collapsed to `Candidate::Passed` | `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` failed `expected a refusal` at the ELOOP assertion; doctor's obstruction test printed `ok dsh: DSH_B_SENTINEL_0.0.0-b` where the native child stopped with ELOOP; the matrix: `the resolver selected a file where the child stopped: … layout "A:B, A is a self-symlink (ELOOP)", native Err(FilesystemLoop), resolver Ok(…/b/dsh)`. | all three `ok` |
| R7a | header `Err(_) => rest.trim()` | `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`: `"lockfileVersion:9.0\n…" was accepted` | `ok` |
| R7b | package child `Err(_) => Separated::Inline(rest.trim())` | same test: `"… resolution:{integrity: sha512-X}\n" was accepted` | `ok` |
| R7c | flow field check without `\|\| text.contains(": ")` | same test: `"{integrity: sha512-X, tarball: x: y}" was accepted` | `ok` |
| R8 | flow field `pnpm_scalar(value.trim())` | `pnpm_integrity_preserves_unicode_whitespace_for_refusal`: `"{integrity: \u{a0}sha512-X}" was accepted` | `ok` |
| R9 | test premise `assert_eq!(DshSeams::resolve().is_ok(), dsh_home().is_some())` restored in the seams child control | `dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one`: `the inherited premise: a home is a resolution — left: false, right: true` under `HOME=/tmp`, `PATH=/usr/bin:/bin` | `ok` |

The production file was scanned for every mutation spelling after the
last restoration (none present) and the final gates below ran on the
restored bytes. The first hold's removal records remain dated history.

**Gates on the final candidate (this seat, Linux x86_64, cargo 1.98.0,
stable toolchain; coverage on the pinned `nightly-2026-09-05` with
cargo-llvm-cov 0.9.0):**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, 0 warnings |
| `cargo test -p <crate> --all-features --locked`, each of the seven crates separately | all green; `gpt_flash_shape`, roster and witness-digest suites pass (no inherited exemption claimed) |
| `cargo test --workspace --all-features --locked` | green: 74 test-binary results, 0 failed, no hang |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles |
| `git diff --check`; frozen surfaces (`contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`), `Cargo.toml`, `Cargo.lock` | clean; byte-identical |
| `openspec validate --all --strict` | **not run**: the seat sandbox refuses the `openspec` binary (direct and via `npx`); pending host validation |
| `bash scripts/coverage-exact.sh` | **the literal script could not be launched** (the seat sandbox refuses script files). The script's own steps were run by hand on the restored bytes with fresh instrumentation: `cargo +nightly-2026-09-05 llvm-cov clean --workspace`, then `cargo +nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch --json`, then `llvm-cov report --branch --lcov`, then the script's exact LCOV accounting (every DA and BRDA hit; source functions by file + start line) reproduced in a run-local tally. |

Fresh coverage integers on the final bytes: **lines 31684 / 31684 (100%),
branches 5338 / 5338 (100%), functions 3060 / 3060 (100%)**; no test-harness
source in the report. Against the historical 31166 / 5214 / 3008 the
denominators grew by 518 lines, 124 branches and 52 functions, all in the new
`composite/image.rs` reader and the resolver's platform rule, search
context, interpreter chain and loader evidence in `composite.rs`. An earlier
measurement on the same production bytes found 14 lines, 5 branches and 2
functions of `native_obstruction`'s loader arms unreached; the loader-arm
unit test closed them and the run above is the fresh measurement after it.
The literal script run on a capable host or CI remains the gate of record
and is recorded as **pending**.

**Pending, recorded and not claimed:**

- Native Windows execution of the matrix and the Windows lookup, and the
  Windows OS binary-type query D10 names beside the PE header check.
- Native macOS execution: the Mach-O reader and its `dyld` prerequisite are
  proved on synthetic images only; the two missing-loader doctor cells and
  the two matrix loader layouts print `PENDING` on non-Linux Unix.
- The absent-PATH native-positive for **Node**: this host keeps `node`
  under `~/.volta/bin`, not on the default search path, so the Node cell
  asserted the named refusal (`selects no 'node': … 'node' is not on the
  default search path … (PATH is absent)`) and printed `PENDING`; the
  `sh` positive is established by `/proc` identity.
- The Node selection preflighted for a `#!/usr/bin/env node` candidate is
  not retained into the later composite observation; the composite selects
  `node` again by the same rule under the same environment (same-host
  concurrent mutation remains an observation limit).
- `openspec validate --all --strict`, the literal coverage script and
  remote CI on the final head: host/CI evidence.

Clauses 8.8.1.1, 8.8.3.1 and 8.8.8.1 are ticked. 8.8.1.2, 8.8.2.1 and
8.8.2.2 stay open only for the pending native-platform and Node positives
above; 8.8.8.2 stays open because `openspec validate` could not run here;
8.8.8.3 stays open until the literal script result exists; 8.8.8.4 stays
open with them. Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15
were not touched; 8.8 stays unchecked; 0056 stays proposed; no push.

### Historical returned implement — review R1–R12 of the second-hold delivery, 2026-09-20

Run `dsh-composite-identity-issue-226-069caa79`, phase implement, returned
from review with `residual` (medium floor, twelve deduplicated findings) on
`1568af91`. Every commit on `slice-dsh-composite-b` was adopted; this visit
answers the findings in the review's own order of severity and records what
it could not establish from this seat. It describes outcomes and directs no
gate.

**R1 — ENOTDIR is walked past.** `classify_in` now records `ENOTDIR` beside
`ENOENT` as an entry the child's `execvp` walks past (`is_not_a_directory`,
asked by the platform's errno like the ELOOP check), so `PATH=<file>:B`
selects `B/dsh` exactly as the native child runs it; an explicit path with a
file for a component is still refused by that cause, because an override has
nowhere to walk to. The classifier test replaces its wrong assertion with the
native oracle (`Command::new("dsh")` under the same `PATH` succeeds) and the
continuation cell; the ENAMETOOLONG control keeps the "cannot be proved"
refusal for a failure the child does stop on.

**R2 — the `env` argument as the kernel hands it.** `binfmt_script` passes
everything after the interpreter, trailing spaces and tabs removed, as ONE
argument. `env_program` now selects that whole argument on Linux and
Android: `#!/usr/bin/env reviewed extra` looks up a program named
`reviewed extra` under the same search, so an obstructed `A/reviewed extra`
is D10's named refusal before any probe where the child walked it to B; a
one-word `node` with trailing blanks is still the measured form; `node
--flag` is the program `node --flag`, which no search holds, and refuses as
such on Linux. The other Unix kernels split the line into words, and there
a multi-word argument refuses by name rather than being guessed either way
(the classifier test asserts the platform-appropriate reason). New:
`an_env_argument_is_selected_as_the_kernel_hands_it_to_env` in both the
protocol suite (native oracle first) and the built-doctor suite (native
child runs `B/reviewed extra`; doctor refuses naming `A/reviewed extra`'s
missing interpreter, executes nothing, and never touches the `A/reviewed`
decoy).

**R3 — a terminal colon is the mapping indicator.** A plain flow scalar
that ENDS in `:` refuses as unsupported flow syntax, whatever the document
followed the colon with (padding, `,` or `}`), so `tarball: x: }`,
`tarball: x:}`, `integrity: sha512-X:` and `integrity: sha512-X: ` no longer
read as the control; a package heading whose key ends in `:` once its own
colon and padding are gone (`debug@2.6.9: :`, `debug@2.6.9::`) is "a package
key that is itself a mapping". Seven vectors added to
`missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`;
the separated controls and the colon-without-space URL still read to the
control's composite.

**R4 — the bounded Mach-O read.** `LC_LOAD_DYLINKER` needs twelve bytes;
the generic eight-byte bound did not establish the offset field, and the
`expect` behind it panicked on a 40-byte image. The command-specific read is
now bounded by the command's declared size before it is made, and the image
refuses as "a malformed Mach-O dynamic linker command". Covered in
`every_macho_rule_refuses_by_name` (the `cmdsize` 8 vector), through public
selection in `a_truncated_macho_dylinker_command_is_refused_rather_than_panicking`
(explicit path, search, and `selected_from`), and by the built doctor in
`a_truncated_macho_on_path_is_refused_by_name_without_a_panic` (exit code is
not 101, no `panicked` on stderr, nothing probed, the cause named).

**R5 — the Node selection is retained.** Selection now answers a `Selected
{ path, node }`: the `node` an `env node` line selected under the same
search is carried through `DshSeams::selected_from` into a new
`DshSeams.node`, and `dsh_composite` probes THAT file
(`spawn_node_runtime(retained)`), looking `node` up only when the selection
established none (a native image, another interpreter). A script whose
interpreter is the `env node` script retains the innermost selection. The
reproduction is the built-doctor test
`the_composite_probes_the_node_the_selection_retained`: `A/node` answers the
version probe and removes its own launcher, and doctor's line carries the
version beside `composite unreadable: … node --version: No such file or
directory` with `B/node` never executed (marker directory), where the second
lookup produced a readable composite through B. The protocol test
`the_composite_observes_the_node_the_selection_retained` drives the same
through the sole producer, and the seams test asserts the retained path
crosses `selected_from`. `DshSeams` gained the one runtime field the
observation needs; no stored declaration, contract or identity format moved.

**R6 — the matrix count and the ELF test.** The parent asserts the tally
against `MATRIX_LAYOUTS` (15 on Linux, 13 on the other Unixes) and the child
asserts it built exactly that many; the ELF loader test is gated to ELF
targets (`all(unix, not(target_vendor = "apple"))`); the classifier's
foreign-image vectors are chosen by `image::NATIVE` (the two formats this
target does not load, a synthetic ELF included), so a macOS run refuses a
Mach-O for nothing and an ELF for being another target's. No macOS
execution is claimed.

**R7 — PE admission.** The PE reader now requires this target's
optional-header magic (PE32+ on 64-bit targets, PE32 on 32-bit), 1 to 96
sections whose table lies inside the file, every section's raw-data range
inside the file, and a declared header size covering the headers and no
more than the file; `admit_windows` additionally asks the OS
(`GetBinaryTypeW`, read-only) and admits only this target's binary type.
Seven refusal vectors and the 96-section positive were added to
`a_pe_image_is_admitted_by_its_bounded_header`; `synthetic_pe()` is the
shared well-formed image the resolver tests plant as a foreign candidate
and as a foreign loader.

**R8 — Windows evidence, written and unexecuted here.**
`native_executable_resolution_matches_command_matrix_on_windows` crosses
the eight commissioned spellings (`.\dsh`, `..\dsh`, an owned absolute
`…\abs\dsh`, the literal `C:\Tools\dsh.exe`, `dsh.exe`, `my dsh`, a NUL
name and `dsh`) with eleven layouts (cwd-only, PATH directory plus cwd
decoy, absent PATH, present-empty PATH with and without a candidate,
leading/interior/trailing empty entries, and `A;B` with a non-image, a
directory and a working image at A) against real `Command::new(name)`
children in a child process holding the fixture cwd; the sentinels are
hard-linked copies of the test binary answering with their own image
path, so every cell asserts the canonical file the platform ran or its
exact failure. The literal drive path is asserted as the absent path a
runner has and recorded PENDING where an operator keeps an installation
there, never created or executed. A new built-doctor suite
`crates/brokkr-cli/tests/doctor_dsh_selection_windows.rs` (cwd image never
selected, native image on PATH selected as the child selects it,
existing-but-unrunnable entries refused by cause without a panic, batch
dispatch refused, override precedence). Both are `cfg(windows)`, compile
against the `x86_64-pc-windows-msvc` target here (protocol crate; the cli
crate's C dependencies need an MSVC toolchain this host lacks) and are
**unexecuted on Windows in this seat**: Windows CI on the final head is
the evidence, recorded pending.

**R9 — the denial is named.** `Candidate::Passed { why, denied }` records
whether an entry was walked past for `EACCES` (untraversable component,
not a regular file, not executable by this process) or for absence;
`Search::find` answers the first denial when nothing was admitted —
`'dsh' is not executable by this process on PATH: <candidate>: <why>` —
and the plain no-match otherwise, which is the child's PermissionDenied
versus NotFound. The later-executable-wins control, the first-denial
control, the directory-named-dsh control and the default-search spelling
are in the classifier test; the matrix gained the "A alone, A not
executable" layout and asserts the denial is named wherever the native
child reports PermissionDenied.

**R10 — Windows-target clippy.** `Candidate`, `is_symlink_loop` and
`resolve_executable_in` are Unix-only or test-only; `Native::is_loader_for`
is `cfg(any(unix, test))`; Windows `resolve_executable` goes through
`select_in(command, None)`, which is the same directory order an unchanged
child environment gets. Fresh `cargo clippy --target x86_64-pc-windows-msvc
-p brokkr-protocol --all-targets --all-features --locked` reports no
warning in `composite.rs`, `composite/image.rs`, `composite/image/tests.rs`
or `composite/tests.rs` (the remaining Windows test-build warnings are in
`secret/tests.rs`, `adapters/tests.rs`, `dsh_sandbox/tests.rs` and
`hands/tests.rs`, none touched by this slice and not charged by the review).

**R11 and R12.** The coverage record is below; this account describes and
does not instruct.

**Removal records** — each a compiling mutation applied ALONE in a detached
scratch worktree of `HEAD` under the ignored `.forge/mut`, synced to the
candidate's bytes by patch, the named test run with `cargo test -p <crate>
--all-features --locked <name>` there, the intended assertion failing, the
exact inverse edit applied, and the restored worktree verified
byte-identical to the candidate before the candidate's own green runs:

| # | Mutation (exact, in the scratch worktree) | Named test and failing assertion | Restored |
|---|---|---|---|
| M-R1 | `classify_in`: the `is_not_a_directory` arm removed, so ENOTDIR falls to "the lookup cannot be proved" | `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` panicked at `tests.rs:4683` on the `unwrap()` of the ENOTDIR continuation cell ("the resolver walks past ENOTDIR to B as the child did"), the native child having run B | inverse edit; `ok` in the candidate suites |
| M-R2 | `env_program`: the Linux program cut at its first blank (`&argument[..position(is_blank)]`) | protocol `an_env_argument_is_selected_as_the_kernel_hands_it_to_env`: `expected a refusal` (A's script admitted on `A/reviewed`); built-doctor `an_env_argument_is_selected_as_the_kernel_hands_it_to_env` at `doctor_dsh_selection.rs:809`: `doctor probed nothing` failed — doctor executed A's script and `env` ran `B/reviewed extra` | inverse edit; both `ok` |
| M-R3a | `pnpm_flow_map`: `\|\| text.ends_with(':')` removed | `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`: `"{integrity: sha512-X, tarball: x: }" was accepted` | inverse edit; `ok` |
| M-R3b | package heading: `\|\| key.ends_with(':')` removed | same test: the `debug@2.6.9: :` lock `was accepted` | inverse edit; `ok` |
| M-R4 | `image.rs`: the `cmdsize >= 12` bound removed, the bare `expect` on the offset restored | `every_macho_rule_refuses_by_name` and `a_truncated_macho_dylinker_command_is_refused_rather_than_panicking` both PANICKED inside the reader at `image.rs:478:48`; built-doctor `a_truncated_macho_on_path_is_refused_by_name_without_a_panic` at `:888`: `doctor panicked` — the doctor child's own panic at `image.rs:478:48`, exit 101 | inverse edit; all three `ok` |
| M-R5 | `spawn_node_runtime`: the retained runtime ignored (`retained.filter(\|_\| false)`), a second `node` lookup restored | protocol `the_composite_observes_the_node_the_selection_retained` failed at `tests.rs:5095` (the composite no longer observes A's runtime); built-doctor `the_composite_probes_the_node_the_selection_retained` at `:966`: `the version probe ran A's node once, and B's never` failed — the line carried a READABLE composite (`composite aad4eaec… plugin 8894f23e…`) through `B/node`, the reviewer's reproduction | inverse edit; both `ok` |
| M-R6 | `MATRIX_LAYOUTS` set to the old literal 14 | `native_executable_resolution_matches_command_matrix`: child `assert_eq!(layouts.len(), MATRIX_LAYOUTS)` — `left: 15, right: 14`; parent tally assertion at `tests.rs:2605` | inverse edit; `ok` |
| M-R7 | `pe()`: the section raw-range check replaced by `let _ = (raw_size, raw_at)` | `a_pe_image_is_admitted_by_its_bounded_header` at `image/tests.rs:972`: the "a PE section beyond the end of the file" vector was accepted (`unwrap_err` on `Ok`) | inverse edit; `ok` |
| M-R9 | `Search::find`: the denied arm collapsed into `Candidate::Passed { .. } => {}` | `path_resolution_walks_past_a_candidate_a_child_could_not_execute` at `tests.rs:1866`: `left: "…'dsh' is not on PATH"` vs `right: "…'dsh' is not executable by this process on PATH: …/first/dsh: is not executable by this process"`; the matrix at `tests.rs:3056`: `EACCES is named as the denial it is: … layout "A alone, A not executable", native Err(PermissionDenied), resolver Err("'dsh' is not on PATH")` | inverse edit; both `ok` |

R8 has no removal on this host (unexecuted Windows evidence); R10's proof is
the Windows-target clippy output; R11's is the coverage record. After the
last restoration the scratch worktree was reset and re-synced from the
candidate's final diff and `diff -rq` over both `crates/` trees found only
ignored scratch directories, no source difference.

**Gates on the final candidate (this seat, Linux x86_64, cargo 1.98.0
stable; coverage on the pinned `nightly-2026-09-05` with cargo-llvm-cov
0.9.0):**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, 0 warnings |
| `cargo clippy --target x86_64-pc-windows-msvc -p brokkr-protocol --all-targets --all-features --locked` | no warning in any file this slice touches (R10) |
| `cargo test -p <crate> --all-features --locked --no-fail-fast`, each of the seven crates | all green on the final bytes, including `gpt_flash_shape`, roster and witness-digest suites; `brokkr-protocol` 372 lib + 99 + 1; `brokkr-cli` 0 failed, `doctor_dsh_selection` 11 passed (was 8) |
| `cargo test --workspace --all-features --locked --no-fail-fast` | green, 0 failed; and green again inside the instrumented run below |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and `bundles/verify` | compile |
| `git diff --check`; frozen surfaces (`contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`), `Cargo.toml`, `Cargo.lock` | clean; byte-identical |
| `openspec validate --all --strict` | **not run**: the seat sandbox refuses the `openspec` binary; pending host validation (the last recorded strict pass on this change is the tasks seat's, 15/15, at `3e18f2c9`; under `openspec/` this visit changed only this account) |
| `bash scripts/coverage-exact.sh` | **the literal script could not be launched** (script launches are refused in this seat). Its steps were run by hand on the final bytes with fresh instrumentation: `cargo +nightly-2026-09-05 llvm-cov clean --workspace`; `cargo +nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch --json --output-path .forge/coverage-r/coverage-final.json` (every test run, 0 failed, no `--ignore-run-fail`); `llvm-cov report --branch --lcov`; then the script's LCOV rule — every `DA` and `BRDA` hit, every logical function by file + `FN` start line — transcribed in a run-local Rust tally under `.forge/lcovtool/` because the seat refuses `awk`. |

Fresh coverage integers on the final bytes: **lines 31750 / 31750 (100%),
branches 5354 / 5354 (100%), functions 3069 / 3069 (100%)**; 0 test-harness
sources in the report. Against the previous delivery's 31684 / 5338 / 3060
the denominators grew by 66 lines, 16 branches and 9 functions, all in the
resolver's denial tracking, the retained selection, the `env` argument
rule, the PE section and header checks and the Mach-O bound. A first
measurement on these bytes found 9 lines and 1 branch unreached: the
non-Linux arm of the `env` argument rule, compiled in by a runtime `cfg!`
and made a compile-time `#[cfg]`, and the zero-raw-size PE section branch,
covered by the `.bss`-like positive; the integers above are the fresh
measurement after those two edits. The literal script on a capable host
and the workflow's `coverage-exact` job on the final head remain the gate
of record and are recorded as **pending**. The reviewer's fresh gate
(31508/31684 with misses outside this slice) was a boxed run; every miss
it named lies in namespace-dependent paths this unboxed run reaches.

**Pending, recorded and not claimed:**

- Native Windows execution of the Windows matrix, the Windows doctor suite,
  the `GetBinaryTypeW` query and the Windows MSRV build: Windows CI on the
  final head. The cli crate's Windows cross-check could not run here (its
  C dependencies need an MSVC toolchain); the protocol crate's did.
- Native macOS execution: the Mach-O reader, the `dyld` prerequisite and
  the platform-split `env` argument refusal are proved on synthetic images
  and by source only; the two missing-loader doctor cells and the two
  matrix loader layouts print `PENDING` there.
- The absent-PATH native-positive for Node on a host keeping `node` on its
  default search path (this host keeps it under `~/.volta/bin`).
- `openspec validate --all --strict`, the literal coverage script and
  remote CI on the final head: host/CI evidence.

No local clause changes state: 8.8.1.2, 8.8.2.1 and 8.8.2.2 stay open on
the native-platform and Node positives above; 8.8.8.2 on `openspec
validate`; 8.8.8.3 on the literal script; 8.8.8.4 with them. Part (d),
8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 were not touched; 8.8
stays unchecked; 0056 stays proposed; no push.

### Historical returned implement — review F1–F9 of the second-hold delivery, 2026-09-20

Run `dsh-composite-identity-issue-226-09ec8d81`, phase implement, successor
to run `069caa79` (parked `REVIEW-SPEC-DEFECT-EXHAUSTED`). Every commit on
`slice-dsh-composite-b` was adopted, including the controller's
platform-qualified ELOOP correction at `af7d6378`, which this visit
implements rather than reopens. The nine surviving MEDIUM findings are
answered in the review's order. This account describes outcomes and directs
no gate.

**F1 — an overlong PATH component is walked past.** `classify_in` now
records `ENAMETOOLONG` beside `ENOENT`, `ENOTDIR` and `EACCES` as an entry
both searches walk past: glibc's `execvp` skips a `PATH` component longer
than the buffer it sized for the whole variable before any `execve`, and
Apple's `execvP` warns on the oversized candidate and continues. The
component establishes nothing about a candidate, so it is neither D10's
loading obstruction nor a denial, and a search that then admits nothing
answers NotFound. Measured here with a real `Command` oracle: `PATH` spelled
as 5,000 ASCII `x` bytes followed by a runnable `B` runs `B/dsh`, and so
does the resolver. The classifier regression replaces its inherited
"cannot be proved" assertion with that oracle, the removed-component
positive control, the overlong-NAME no-match and the explicit-path refusal;
the differential matrix gains a sixteenth layout, "overlong PATH component,
then B", crossed with all eight spellings.

**F2 — each platform's own absent-PATH search.** `default_search_of` is now
a per-platform table: glibc and Android read `confstr(_CS_PATH)`, because
their own `execvp` reads it; musl searches its literal
`/usr/local/bin:/bin:/usr/bin`; every BSD-derived target, Apple included,
searches `_PATH_DEFPATH`, `/usr/bin:/bin`. Apple's `confstr(_CS_PATH)`
answers the wider `USER_CS_PATH`, `/usr/bin:/bin:/usr/sbin:/sbin`, so asking
the library there would have put two system directories on a search Apple's
loader never walks, and the resolver could have selected or probed a system
executable native lookup would not select. The new named regression
`each_platforms_absent_path_search_is_its_own_loaders_rule` asserts the row
for macOS, iOS, tvOS, watchOS, FreeBSD, glibc, Android and musl and asserts
the distinction itself — `/usr/sbin` and `/sbin` are on the `confstr` answer
and not on the loader's — on whichever platform the suite runs, which an
`sh` positive shared by both searches cannot do. Native Apple execution of
this cell remains PENDING; the table is source evidence plus a Linux-run
regression, recorded as such.

**F3 — the platform-qualified ELOOP cell, implemented.**
`symlink_loop_candidate` is chosen at compile time: on Linux and Android a
loop STOPS the search by that named cause (glibc, measured 2026-09-20); on
every other Unix it is walked past, not denied, as Apple's `execvP` and
`posix_spawnp` walk past it. The classifier regression, the doctor
regression `an_obstructed_path_search_takes_the_explicit_safe_refusal` and
the matrix's `A:B, A is a self-symlink (ELOOP)` layout all assert the
RUNNING platform's own native control — the resolver refuses by cause where
the child stopped, and selects exactly the file the child ran where it
continued — never a fixed "Unix" outcome. An explicit override remains a
refusal on either platform, by the cause the search would have used, because
an override has no next entry. Native macOS execution of this cell remains
PENDING.

**F4 — the `env` an interpreter IS, not the name it wears (SECURITY).**
`is_env` now asks the FILE: an interpreter is `env` when the `#!` line
spells that basename OR when the file it canonically is has that name. An
`env-alias` symlinked to the same binary therefore reaches D10's refusal,
where basename-only recognition let the measured form through, admitted a
launcher whose `node` was missing and let doctor EXECUTE it. Both the
protocol and the built-doctor
`an_env_argument_is_selected_as_the_kernel_hands_it_to_env` now cross an
obstructed `A/node` and a runnable `B/node` with both spellings: the named
obstruction refusal is identical under each, doctor leaves no execution
marker under either, and the valid chain is a positive. The independent
native control is kept and recorded: reaching it proves native lookup
selected and loaded the launcher under that spelling, and where the
platform's own `env` ran a program it ran B's. This host's `env` is a
uutils multi-call binary that refuses to answer to `env-alias`
("Security violation: Requested utility `env-alias` does not match
executable name"), so the version half of the alias valid-chain positive is
PENDING here and prints as such; the refusal and no-probe halves are
established on both spellings.

**F5 — ignored pnpm values are admitted as SYNTAX (SECURITY).** Three
independent guards, each removed separately below. (a) YAML 1.2's own
character set (§5.1 `c-printable`) is asked of the WHOLE document beside the
existing tab and carriage-return refusals: a NUL, a BEL, an ESC, DEL, a C1
control or a plane-end noncharacter is not a value with an unusual byte in
it, it is a stream YAML cannot carry. (b) A recognized top-level scalar key
must carry a SCALAR: `pnpmfileChecksum: sha256-a: b` is the mapping YAML
opens there. (c) An ignored package child's inline value is admitted by
`pnpm_ignored`: a quoted scalar closes its quote, a flow collection closes
and does not nest and every member is a flow scalar, another indicator opens
syntax this grammar does not read, and a plain scalar is refused exactly
where YAML refuses one — the mapping it would open and the comment it would
start. The admitted dialect did NOT grow: a plain block scalar's free text
keeps its apostrophes, commas and brackets, `engines: {node: '>=18.12'}`,
`cpu: [x64, arm64]` and `hasBin: true` still read, and the resolution map's
own rules are unchanged, now expressed through a shared `flow_scalar`.
Every commissioned vector — plain tarball NUL and BEL, quoted tarball NUL,
checksum colon-space and NUL, the unterminated `deprecated` quote and the
unterminated `engines` flow map — refuses by pnpm, field and cause through
the sole producer and through a complete synthetic installation under the
BUILT doctor, and none of them reports the valid control's composite. The
separator, NBSP and duplicate-key cases are retained; the NUL package-key
vector now refuses one step earlier, at the character set, and a U+00A0 key
keeps the key's own scalar reason.

**F6 — the inspected launcher head is retained.** `Selected` and `DshSeams`
now carry the bounded head selection READ, and `first_line` reads those
bytes and opens nothing. Composition had asked the file again AFTER the
version probe had run it: a shell launcher that answers `v22.23.2` and
rewrites itself to the `env node` shebang was refused by the reading that
admitted it and admitted by the reading that followed. Reproduced here under
the restored reread as the reviewer's own digest,
`f742ba0ece3e4684adf8398132b688a2ca70d6397b1f29fcef7e2b3ea15eb80d`, and
refused on the candidate bytes. Two named regressions,
`the_composite_reuses_the_launcher_head_selection_inspected` (protocol) and
`the_composite_reuses_the_launcher_head_doctor_selected` (built doctor),
each carry the rewriting launcher, the otherwise identical non-rewriting
control and the valid `env node` positive. The Windows admission retains its
head from the same inspection, so the contract holds on both platforms.

**F7 — the absent-PATH Node positive asserts identity.** The installed core
launcher now prints `process.execPath`, so doctor's version field IS the
canonical identity of the runtime doctor selected, probed and retained — not
a version banner two installations can share. The positive asserts the
native child's SUCCESSFUL exit, the exact `process.execPath` it printed and
the canonical equality of the two. Under a scripted `node` shim the launcher
body is never read, so every other test is unaffected. This host keeps
`node` under `~/.volta/bin` and not on `/usr/bin:/bin`, so the native child
answers NotFound: the test asserts the named pre-probe refusal instead and
prints `PENDING: no node on this host's default search path`. The
discriminating removal — unconditional absent-PATH refusal failing the Node
positive while native Node still succeeds — therefore remains PENDING with
it, and no shell-only failure is offered in its place.

**F8 — final-candidate native evidence.** Not established, and not claimed.
Native Windows execution of the matrix, the Windows doctor suite and the
`GetBinaryTypeW` path, the Windows MSRV build, native macOS lookup and
loading controls (ELOOP continuation, `_PATH_DEFPATH`, the Mach-O reader and
its `dyld` prerequisite) and the actual default-search Node positive all
remain PENDING on CI for the final head. This seat is Linux x86_64 with
cargo 1.98.0 stable; no cross-compilation, source inspection, synthetic
image or passing skip is recorded as one of those executions.

**F9 — the fresh literal exact-coverage gate.** The literal
`bash scripts/coverage-exact.sh` cannot be LAUNCHED from this seat (script
launches are refused, as they were for the previous seat). Its exact steps
were run by hand on the final restored bytes: the `coverage(off)` refusal
(`git grep` finds none); every stale instrumentation directory removed
(`target/llvm-cov-target`, `target/llvm-cov`, `target/coverage`) so no
earlier instrumented executable can participate in the merge;
`cargo +nightly-2026-09-05 llvm-cov --workspace --all-features --locked
--branch --json` with every test run and none failing, no
`--ignore-run-fail`; `llvm-cov report --branch --lcov`; the harness-leak
check over the LCOV `SF:` records (0 matches); and the script's own LCOV
arithmetic — every `DA` and `BRDA` record, and every logical function by
file plus `FN` start line — transcribed into a run-local Rust tally under
`.forge/coverage-f9/tally/`, because this seat refuses `awk`. **Fresh
integers on the final bytes: lines 31848 / 31848, branches 5366 / 5366,
functions 3088 / 3088; literal 100% equality met.** A first measurement on
these bytes found 4 missed lines and 1 missed branch, all in this slice's own
new code — the two upper `c-printable` ranges, a valid quoted ignored
scalar, a quoted top-level checksum and the malformed-KEY half of a flow
member — and each was closed with a vector rather than a lowered gate; the
integers above are the fresh measurement after those vectors. The retained
reports are `.forge/coverage-f9/lcov-final.info` and
`.forge/coverage-f9/coverage-summary.json`.

  The review's box measured 31574 / 31750 lines, 5340 / 5354 branches and
  3059 / 3069 functions on the previous candidate and named the 176 missed
  lines as lying outside `composite.rs` and `composite/image.rs`, including
  namespace-dependent paths. This run is UNBOXED. The workspace box
  deliberately refuses to nest a namespace — the release configuration
  records exactly that for this gate ("Boundary tests require creating a
  namespace, which the box deliberately refuses to nest… record this check
  as pending… until that external result exists") — so a boxed run cannot
  reach the boundary and hands paths this one executes. That is an
  explanation of the difference, not a claim that a boxed run should pass:
  capable-host equality, the literal script and the workflow's
  `coverage-exact` job on the final head remain the gate of record and are
  recorded as **pending**. The gate was not lowered and no exclusion
  attribute exists.

**Removal proofs on the final candidate.** Each mutation compiled, ran to
its intended failing assertion, and was restored to the exact candidate
bytes with the named test green again.

| Mutation | Named test and its failure | Restoration |
|---|---|---|
| F1 | `classify_in`'s `ENAMETOOLONG` arm made unreachable (`&& false`) | `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`: the overlong-component positive `unwrap()`ed a `Config("xxx…/dsh: the lookup cannot be proved: File name too long (os error 36)")`; the matrix at `tests.rs:3151`: `the resolver selects exactly the file the child ran: … layout "overlong PATH component, then B" … native Ok("c11s0"), resolver Err(…)` | inverse edit; both `ok` |
| F2a | `default_search_of`'s BSD row set to `DefaultSearch::Library` | `each_platforms_absent_path_search_is_its_own_loaders_rule` at `tests.rs:1857`: `macos: left: Library, right: Literal("/usr/bin:/bin")` | inverse edit; `ok` |
| F2b | `BSD_DEFAULT_PATH` widened to the `confstr` answer `/usr/bin:/bin:/usr/sbin:/sbin` | same test, same line: `left: Literal("/usr/bin:/bin:/usr/sbin:/sbin")` | inverse edit; `ok` |
| F3 | the Linux arm of `symlink_loop_candidate` made to CONTINUE (the other platform's rule) | protocol classifier: `expected a refusal` at the ELOOP cell; built-doctor `an_obstructed_path_search_takes_the_explicit_safe_refusal` at `:693` | inverse edit; both `ok` |
| F4 | `is_env` reduced to the spelled basename | protocol `an_env_argument_is_selected_as_the_kernel_hands_it_to_env`: `expected a refusal` under the alias; built-doctor same test at `:957`: the alias spelling no longer names the obstruction | inverse edit; both `ok` |
| F5a | the document character-set guard made unreachable | protocol `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason`: `expected a refusal` on the plain tarball NUL; built-doctor `ignored_pnpm_values_are_admitted_as_syntax_through_the_built_doctor` at `:1561` | inverse edit; both `ok` |
| F5b | `pnpm_ignored` dropped from the ignored package child | the same two tests, failing on the unterminated `deprecated` quote | inverse edit; both `ok` |
| F5c | the top-level scalar's `plain_opens_a_mapping` check removed | the same protocol test, failing on `pnpmfileChecksum: sha256-a: b` | inverse edit; `ok` |
| F6 | `first_line` reopening the file (the post-probe reread) | protocol `the_composite_reuses_the_launcher_head_selection_inspected`: `expected a refusal`; built-doctor `the_composite_reuses_the_launcher_head_doctor_selected` at `:1462`, which reported the reviewer's own readable composite `f742ba0e…` through the rewritten launcher | inverse edit; both `ok` |

F7's discriminating Node removal, F8's native executions and F9's literal
script and capable-host equality have no removal on this host; they are
recorded as pending above and a mutation is not offered in their place.

**Gates on the final restored bytes (this seat, Linux x86_64, cargo 1.98.0
stable; coverage on the pinned `nightly-2026-09-05` with cargo-llvm-cov
0.9.0):**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean, 0 warnings |
| `cargo test -p <crate> --all-features --locked --no-fail-fast`, each of the seven crates separately | all green, 0 failed; `brokkr-protocol` 374 lib + 99 (2 ignored, the native macOS launchd probes) + 1; `brokkr-cli` 32 binaries green, `doctor_dsh_selection` 13 passed (was 11) |
| `cargo test --workspace --all-features --locked --no-fail-fast` | green, 75 binaries, 0 failed; see the flake note below |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and `bundles/verify` | both compile |
| exact coverage | **31848 / 31848 lines, 5366 / 5366 branches, 3088 / 3088 functions**, by the script's own rule on fresh instrumentation; the literal script could not be launched from this seat and remains pending as the gate of record |
| `git diff --check`; frozen surfaces (`contracts/`, `policy/`, `fixtures/`, `reference/`, `extensions/dsh/`, `docs/decisions/`), `Cargo.toml`, `Cargo.lock` | clean; byte-identical |
| `openspec validate --all --strict` | **not run**: this seat refuses the `openspec` binary and its `npx` install. Pending host validation; under `openspec/` this visit changed only this account |

**One flake, recorded not hidden.** On one of four workspace runs on the
final bytes, `hands::tests::the_network_prefix_is_eight_tokens_and_the_probe
_asks_the_dispatchs_path` failed at `hands/tests.rs:1199`. It plants an
`unshare` script and spawns it immediately, which is the freshly-staged-
executable race (#255); it passed alone and on the three other workspace
runs, including the instrumented coverage run. The test is in `hands` and
this slice touches nothing it reads. No exemption is claimed and no gate is
waived by this note; it is recorded so a reviewer who meets it knows it was
seen here too.

**Pending, recorded and not claimed:** every F8 native execution above;
F7's default-search Node positive and its discriminating removal; F2's and
F3's native Apple cells; `openspec validate --all --strict`; the literal
coverage script and capable-host/CI coverage equality; remote CI on the
final PR head.

No local clause changes state, and no global one: 8.8.1.2, 8.8.2.1 and
8.8.2.2 stay open on the native-platform and Node positives; 8.8.8.2 on
`openspec validate`; 8.8.8.3 on the literal script; 8.8.8.4 with them. All
101 change-wide identifiers retain their 84 complete / 17 pending states.
Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 were not
touched; 8.8 stays unchecked; 0056 stays proposed; nothing was pushed.

### Historical tasks-phase validation — second security hold, 2026-09-20

This seat delivered the requirement-linked breakdown for the five open findings.
It preserved all fourteen local addresses, retained five completed first-hold
clauses and reopened seven; nine local clauses now await implementation/proof.
All 101 change-wide identifiers retain their 84/17 states, including 8.8.
Production, tests, earlier specification/design artifacts and frozen surfaces
are unchanged. The dated records below remain their original observations,
not current completion claims or gate exceptions.

Fresh attempts are recorded in `.forge/tasks-second-hold-069caa79/gates.json`.
Format, clippy, each of the seven crate suites, workspace tests and self-bundle
compilation could not launch: **Cargo ENOENT, status 127** for each. Literal
`bash scripts/coverage-exact.sh` exited **127** at line 33 for missing Cargo,
before instrumentation. Fresh lines, branches and functions are each
**N/A covered / N/A total (N/A%)**; structured counts are null. Historical
31166/31166, 5214/5214 and 3008/3008 remain comparison data only. No Rust,
coverage, differential or removal pass is claimed by this planning visit.

`openspec validate --all --strict` passes: **15 passed, 0 failed**. Strict
active-change validation, artifact status and `git diff --check` also pass.
The existing informational archive-target notices remain whole-change debt;
this slice makes no archive-readiness claim. The byte/ledger audit confirms
only `tasks.md` changed, all other 738 tracked files are byte-identical,
all 133 checkbox addresses are retained (**105 checked / 28 unchecked**),
and only the seven named local ticks reopened. Each local task names AS1
and all current second-hold scenarios have explicit acceptance coverage.

The tasks result is `drafted`: implementation,
native Windows/default-Node evidence, all green suites and fresh host-capable
exact coverage remain pending. No archive, provider probe, enablement, push
or publication occurred.

### Historical tasks-phase validation — first security hold, 2026-09-20

The second-hold breakdown above supersedes this dated checkpoint's
absent-PATH rule, runtime-failure disposition and coverage acceptance.
Its attempts and counts remain historical observations only.

This seat delivered the ordered, requirement-linked repair breakdown and the
four precise outside-slice runtime dispositions. All 14 local clauses remain
unchecked; all 101 inherited task IDs and their 84/17 states are unchanged.
Only `tasks.md` is a tracked edit. The active change, seven repair findings,
implementation/removal work and full exact coverage remain pending.

Fresh attempts are recorded in `.forge/tasks-security-4437331e/gates.json`:

| Check | Actual result in this tasks seat |
|---|---|
| `openspec validate --all --strict` | Exit 0; 15 items passed, 0 failed. Existing informational archive-target and long-requirement notices remain. |
| `git diff --check` and requirement/ledger checks | Pass; every local clause names AS1, all nine security-hold scenarios have a task, inherited states and frozen paths are unchanged. |
| Format, all-target/all-feature locked clippy, seven separate crate suites and `bundles/self` compile | Each exits 127: `cargo: command not found`. No Rust pass is claimed. |
| Three target-specific runtime reproduction attempts | Each exits 127 for absent Cargo; the four failures above remain recorded evidence, not new execution results. |
| Literal `bash scripts/coverage-exact.sh` | Exit 127 at line 33, `cargo: command not found`, before instrumentation. |

Fresh coverage: **lines N/A covered / N/A total (N/A%), branches N/A / N/A
(N/A%), functions N/A / N/A (N/A%)**. The run-local `coverage.json` uses nulls.
The required result remains **100% lines / 100% branches / 100% functions**
with nonzero integer denominators and all touched production regions included.
No production line was edited in this planning visit; that does not turn the
unavailable full report into a passing gate. Cargo availability, capable-host
namespace execution and final-head external results remain unmet validation
prerequisites, not an earlier-artifact defect or a lowered threshold.

The validator still notes missing living targets for the modified
`adapter-resume-safety` and `sdd-progress-markers` deltas. Their archive/fold
reconciliation is excluded whole-change work; strict validation passes, and
this slice does not claim archive readiness.

This is a `drafted` tasks checkpoint. It does not clear the security hold or
complete task 8.8. No provider probe, archive, frozen/decision edit or push
occurred. The phase artifact is committed separately from ignored run-local
validation records and the mandatory result JSON.

Earlier DSH digest tasks commission, retained as dated history:

Current commission: **DSH COMPOSITE IDENTITY**, run
`dsh-composite-identity-issue-226-26def5a5`, phase tasks, adopts proposal
answer AK and AS1 at `9ccfc68d`, proposed decision 0056 and design `b2adb4f5`
on `slice-dsh-composite-b`; the commissioned baseline is `99fdbb0a`.
Only task 8.8's **(a), (b) and (c)** are commissioned, in D10's order.
Execute the [digest-only breakdown](#current-tasks-return--dsh-composite-digest-breakdown-2026-09-19)
below. Part (d), the `dsh_launch`/`dsh_launch_with` planner, is outside
this commission and must not be implemented, planned or changed.

Both predecessor parks were correct. The canonical local-tarball reinstall
closed the stale-lock/length gap; the raw-byte/layout and retained-hidden-lock
addenda close the missing-input gap. The complete retained npm lock, raw pnpm
lock, raw profile patch, measured triples, both bundle anchors and unchanged
six committed plugin files now support literal test-source ground truth.
Authoring may read the supplied `.forge/tasks/` records; builds and tests must
never read or include `.forge/`. Digests and counts cannot substitute for raw
inputs or full expected-value equality. Neither this tasks visit nor an
independent helper produces a plugin component or canonical composite.
No provider, registry or retained-home remeasurement is authorized.

Task **8.8 stays unchecked** after this slice: its acceptance still spans
planner part (d) and 8.10. The ledger remains **84 complete / 17 pending across
101 identifiers**. The 18 local digest checkboxes below are subordinate clauses
of 8.8, with no new change-wide task identifiers. The five deltas contain
**20 requirements / 178 scenarios**. Archive and folding into living truth
remain whole-change work outside this commission, as D10's migration states.

The adopted design result is `drafted`; no `returned_from` finding is supplied.
D10's dependent-artifact handoff governs this in-place repair: selected private
assessment proof, complete measured literals, full ordered expectations,
producer-derived pins, unreadable declaration context and paired doctor seam
controls. Source inspection confirms the loader grammar and canonical profile
boundary already exist; preserve and prove them while repairing the remaining
producer and doctor defects. Proposed 0056 remains `proposed`; DSH remains
disabled with no declared `wrapper_digest`. No earlier artifact needs revision
before an honest breakdown can be written. This is a drafted tasks checkpoint,
not delivered code, completed removal proof or a passing Rust/coverage report.

Earlier CODEX, END TO END task return, retained as dated history:

Current commission: **CODEX, END TO END**, run
`codex-end-to-end-issue-226-tasks-1bd3f94f`, phase tasks, adopts proposal AB
and specifications `d24189a8` and design `036390bf` over production `5ef4a842`
on `slice-codex-proof`. Only **10.5 and 11.1** are commissioned. Execute the
[Codex proof breakdown](#current-tasks-return--codex-proof-breakdown-2026-09-16)
in dependency order. It supersedes earlier instructions here to probe Codex,
retain 0.153.4 as current applicability, execute THE PROOFS again or close the
whole change. The supplied live proof, raw report and instrument were read
first and in full; no provider command or new probe is authorized.

All five live axes are recorded on **codex-cli 0.154.0**. The return below
cites each observation. **10.5 stays unchecked at this planning checkpoint**
until implementation's matching assertions and removal controls pass; its live
experiment is already done. **11.1 stays unchecked**: historical 10.1 does not
supply exact-0.154.0 interface evidence for effort configuration, the complete
safe passthrough list or stdin prompt positional `-`. Preservation of shipping
support and deterministic tests cannot discharge those missing observations.

The ledger remains **82 complete / 19 pending across 101 identifiers**; no
checkbox is added, removed or renumbered. The five deltas contain **20
requirements / 162 scenarios**. Preserve `work-site: supported`, both shipping
harness/inline no-hands coordinates, every refusal and affirmative confinement
marker. 10.6–10.8 and 11.2–11.4 remain unmeasured/pending; Passes C/D, archive,
re-folding the five withdrawn living specs, new `--worktree`/`--thread-source`
shapes and frozen-byte edits remain outside scope. No earlier artifact needs
amendment before this breakdown can be executed; no `returned_from` finding
was supplied beyond design's drafted handoff.

Earlier THE PROOFS task return, retained as dated execution history:

Current commission: **THE PROOFS**, run
`the-proofs-branch-integration-22-5533cd6b`, phase tasks, adopts proposal AA and
specifications `614abde`, design `80772ce` and production repairs `4daaa7d`.
The predecessor return in
`.forge/results/99cbdaee-9e9d-4fdc-b2f7-25cb2aa60def.json` was read first:
the canonical `Bundle.sites` family, exact drain-all relocation, authoring/final
ownership census, runtime readers and affirmative dispatch markers are delivered.
Their commits and proposed 0056 stand. No production rework is commissioned.

Execute the [THE PROOFS acceptance order](#current-tasks-visit--the-proofs-execution-order-2026-09-16)
under the existing numbered checkbox groups: preserve/measure before; connect
the existing test transports; prove all six gate decisions and their mutations;
cover the two reachable compiler refusals and remove only the unreachable
same-owner census tolerance; run local gates; measure after and commit evidence.
D10 resolves the test seam and coverage dispositions. No upstream choice remains.

Only these two items belong to this visit. The broader F2 campaign, selected-single
follow-up and historical repair instructions below are adopted history or wider
change acceptance. Their remaining proof does not expand AA. Keep the whole
family indivisible, absence refused, and namespace/boxed Codex refused;
affirmative markers alone do not qualify an incompatible coordinate. All six rows use the production gate, with
confirmed engine-offered retries for supported forms.

The ledger remains **82 complete / 19 pending across 101 identifiers**. Every
checkbox state is preserved; record clause progress and evidence without a
whole-change tick. In particular, 8.8, 8.10, 9.6 and 11.1–11.4 stay pending,
and 11.5 keeps its recorded completion. Passes C/D, archive, re-folding living
specs, frozen-byte edits and provider enablement remain excluded. Proposed 0056
stays `proposed`; the settled census simplification requires no semantic redesign.
The unchanged five deltas contain **20 requirements / 159 scenarios** (31
evidence, 64 safety, 11 boundary, 20 progress, 33 site).

Earlier operator-ruling tasks return, retained as dated execution history;
the current THE PROOFS acceptance order supersedes its instructions to repeat work:

Current commission: run `operator-ruling-slice-branch-int-bf5f6a32`, tasks
return, adopts design `78098d5`, the F1 task checkpoint `26f665e`, proposal
W/specifications `3ead8d4` and committed implementation through `2da94b0`.
D10 retains F1's composition repair; D11 now answers analyze F2's stale scenario
inventory. This visit confirms the breakdown against that return and names
its existing gate/no-hands and cold-replacement checks before implementation
resumes.
The operator ruled: **“keep
decision 0030's rejoin live, do not regress codex.”** This is the **operator
ruling and four lows only**. The ledger remains **82 complete / 19 pending**
across the same 101 identifiers; every checkbox state is inherited unchanged.

Execute the [operator-ruling breakdown](#current-tasks-visit--operator-ruling-breakdown-2026-09-15)
in its declared dependency order: preserve reports and measure before;
reconcile proposed 0056; add the shipped-Codex driver regression and observe it
fail; preserve Codex support in the declarations and prove the restoration and
all retained refusals; then verify DSH sequence zero, Codex refusal cause,
Codex cold selectors and DSH never-publish, in that order. Finish the prose,
measured re-pins, local gates, after measurement and unsigned evidence commit.
Every clause belongs to the existing requirement-linked checkboxes below;
none creates a new task identifier or completes a whole checkbox. Record each
actual control and remaining proof here before proceeding to the next clause.

Proposal W/AS1 and design D5/D10 resolve the draft conflict. Returned F1
requires preservation of BOTH shipping Codex work-site coordinates: the
engine-composed harness work seat (declaration `boundaries: ["harness"]`,
`hands: "none"`, actual input `boundary: harness` with no `hands` marker, and
the shipped `--sandbox workspace-write` argv) and the INLINE work seat main
also rejoins (`recipes/standby`, `recipes/wager-harness`: the author's own
`--sandbox danger-full-access`, reported as `boundary: not applicable` with
`hands: "none"`). The declaration names both, and the compiled bundle carries
each inline built-in model driver's adapter assessment so the engine supplies
it exactly as it does for an agent-resolved site.
Main's boxed work seat carries MCP `-c` settings its resume allow-list already
refuses; it stays cold. Use the composition bridge and independent status/hands
controls below. The scalar gate/loader and argv allow-list stay unchanged.
The decision, declarations, guide and executable preservation proof remain
implementation work at this tasks-return checkpoint. Historical
measurement-version drift alone
must no longer disable main's Codex rejoin; actual observed/origin identity,
boundary, hands, accounting and ownership mismatches still refuse. Claude,
DSH and LaneTally remain unmeasured and disabled. Full installed-version
remeasurement and 11.1 remain pending; preservation does not discharge them.

The earlier remediation breakdown and its cold-interval instructions below
are dated history, superseded by W and this breakdown. `2da94b0` settles the
second-selection reader-seam investigation; retain its guards and tests without
repeating that work. The inherited gate reports **30,015/30,015 lines,
5,036/5,036 branches, 2,861/2,861 functions** are prior evidence, never a fresh
before run. Report the unchanged gate's actual before/after results.

Passes C/D, new provider enablement, every 11.x completion, archive and
re-folding the five withdrawn living specs are outside this slice. Leave every
tick unchanged, including 8.8, 8.10, 9.6 and all 11.x. Implementation capacity
exhaustion is `oversized`, never `broken`; name remaining work with the ruling
first. This tasks phase uses its own `drafted`/`upstream` contract.

R5 remains an office boundary: position prose states findings and evidence;
it supplies no gate verdict, destination or authority of another office.

The current staffing ruling supersedes every historical assignment below:
Codex `gpt-6-astra` at `xhigh` holds triage, clarify, chief architect, analyze
and review chief; stable `deepseek-v4-flash` holds implementation, task planning
and every design/review position. No Claude seat is commissioned. This records
an operator ruling, not a recipe/configuration edit or provider evidence.

Earlier commission and execution history, retained for the whole change:

The 2026-09-12 operator ruling adopts all work at
`cf06034`, inherited through `b049224`, for
`current-successor-operator-rulin-eef1e666`. The change stays whole. This visit
adopts the inherited 82-complete/19-pending ledger and reopens only 13.1 on
D10's concrete guide evidence: the current ledger is **81 complete / 20 pending**
across the same 101 identifiers. No implementation/proof task is ticked here.
The council re-entry at `02e6771` preserves that count and all ticks; D6/D10
add the canonical-containment invariant and existing-task acceptance on new
source evidence, with the original loader lookup order retained.
Proposal P governs provider assignments: Astra (`gpt-6-astra`, `xhigh`) holds
the specified gates, chiefs and judges; DeepSeek Flash implements; Sonnet
retains the task planner and specified design/review positions. The historical
no-Codex instruction is superseded and no Fable/Opus pin is restored.

The September 12 controller Codex sandbox evidence makes 10.5's startup
precondition true; the resume measurement itself remains pending. This seat's
boxed version attempt returned command-not-found, while the supplied triage
read reports 0.153.4. Task 10.5 re-reads its exercised binary and reconciles
that result with accepted 0.148.0 and supplied 0.153.4 evidence. Earlier dated
host-blocker accounts below remain history, never current controller blockers.

Groups are the design's landing order (Migration Plan 1–6): the proposed
ruling before any production semantic edit, the record version before
anything emits its fields, the site identity before the query that keys
on it, the query before the wire that carries it, the wire before the
adapters that receive it, the shared launch lifecycle before the four
provider planners, accounting after the planners that feed it, the
charter after the engine work it describes, then the prose, then the
re-pins, then the local gates, final tracked reconciliation while the change
is active, pre-archive readiness as the last tracked task, the normal archive
operation as a non-checkbox artifact effect, the delivery commit action and
controller-owned evidence outside the tracked task state. On this dated return,
15.3 defers only D9's named archive-dependent assertion, with unique-match and
single-filter evidence; the post-task action runs the full unfiltered workspace
suite on archived bytes before the activation/delivery commit. Readiness records
that archived validation as pending, never as a completed full-suite gate.

Execution order is that order with one declared exception, because two
groups consume what group 10 produces. Group 10 is split at its own
seam. **10.1–10.4 are interface investigation**: they read installed
help, installed source and packages, the dated captures under
`.forge/`, and the supplied exact-version upstream checkouts, and they run
**before** group 8's provider-specific construction (8.5–8.9) and group 9's
accounting boundaries (9.1–9.3), whose argv, routes and cursors are their
output — 8.8's DSH route is 10.3's exact forward-pinned core/plugin-adaptation candidate,
and 9.1's current-work cursor is measured, not assumed.
**10.5–10.8 are live enforcement proof**: they need complete dated
provider evidence; supplied partial captures and September 12 Codex startup
evidence do not complete them. They run after
groups 8 and 9, and they gate group 11 and nothing else. Every task that
depends on one of the eight names it, and the `## Progress` foot of this
file records the dependency as it actually binds rather than by group
number.

The operator's 2026-09-10 ruling reversed `a86eca1`'s re-pin to core
0.1.0-rc.6 and reopened its four truth-repair tasks. The inherited work repairs
that pin: 1.1, 6.4 and 11.5 stay checked; 13.1's DSH correction is preserved,
but its Codex/Claude evidence correction is pending before remaining 8.8 work.
The following order describes the original repair dependency, not a request to
repeat the completed pair qualification, loader, locator or doctor work. The
correction adds one earlier evidence seam without renumbering settled task
identities: **10.7's live half runs first, then 1.1 -> 6.4 -> 11.5 -> 13.1,
then 8.8, then 10.7's recording step**. 10.7 first commits the repository-owned six-file adaptation of
`dsh-plugin-cli-session` 0.2.0 under `extensions/dsh/plugin-cli-session/`,
with its sibling `extensions/dsh/PROVENANCE.md` and the
`docs/guides/repository-layout.md` row, installs the latest official core
(`@deepseek-ai/dsh` 0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`,
or the release answer N1 resolves in its place) in a task-owned home, and
records the live cold/warm evidence with the composite's raw inputs, never a
composite digest, in `.forge/tasks/dsh-pair-qualification-015rc1.json`. Only
then can 1.1, 6.4,
11.5 and 13.1 honestly make proposed 0056, the disabled declaration and every
packaged/scaffolded representation, and the provider guide agree that the
forward-pinned core/adaptation route is selected but unmeasured, naming the
superseded `dsh-pair-qualification-010rc6.json` record as dated history. Task
8.8 then lands in design D10's order: the optional `wrapper_digest` loader
amendment, the composite-digest function and its `brokkr doctor` line, then
the Rust route against that fail-closed truth and the committed adaptation.
Once the function and doctor line exist, 10.7's recording step re-measures the
raw inputs, asks doctor for the plugin component and canonical composite over
the retained task-owned home and appends them to the same record. 8.8's
function is the only producer of those values. 10.7 stays unchecked until
that entry and Brokkr's matching assertions agree, and 11.3 still owns any
later enablement. This is an execution dependency across the
existing numbered groups, not a completed provider proof or a new task
group.

Every task names the requirement it serves as `<capability> /
<Requirement>`. The closing gates of group 15 serve every requirement of
the change and say so, because a gate is not a requirement of its own.
Capability short names below are `site` (`site-session-resumption`),
`safety` (`adapter-resume-safety`), `evidence` (`adapter-launch-evidence`),
`progress` (`sdd-progress-markers`), and `boundary` (`boundary-record`,
requirement **The seat record carries the boundary as seat-record/v4**).
Across the five capability deltas, AS1–AS3, PM4 and the F7 boundary
requirement are MODIFIED; the other fifteen requirements remain ADDED.
The current inventory agrees with D11 and the prior inventory recount:
20 requirements / 159 scenarios (evidence 31 + safety 64 + boundary 11 +
progress 20 + site 33). Dated lower counts below remain historical validation
records.

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
  Provider-specific construction spends the dated controller captures under
  `.forge/`, the exact upstream sources identified by
  `.forge/tasks/controller-dsh-upstream-discovery.json`, and nothing else
  (`safety / AS1`). `.forge/controller-provider-evidence-index.md` indexes the
  earlier host interface, four-file DSH headless and 40-file DSH
  session/agent/settings captures. The 2026-09-10 controller evidence
  (`.forge/tasks/controller-dsh-upstream-discovery.json`) adds official core
  0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e` and the selected
  CLI-session plugin as supported interface evidence, plus partial Claude
  observations; none of those source
  inspections or partial probes enables a shape by itself.
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

- [x] 1.1 Amend the existing proposed
      `docs/decisions/0056-same-instance-session-resumption.md`
      with `Status: proposed`, in the register of the neighbouring
      decisions: context from #226 and the measured cold/resumed table,
      the alternatives design D1–D9 rejected, and ten numbered rulings
      whose content is D10's table — work-site continuity with every gate
      fresh (1), same run/site/instance and local origin with no
      older-owner resurrection (2), provider-confirmed root and the two
      admitted identity origins (3), the negotiated one-use offer (4),
      measurement for new rejoins and preservation of currently shipping
      rejoins across historical version drift under proposal W and the
      operator's 2026-09-15 ruling, with actual applicability/origin identity,
      boundary, hands and accounting checks retained, and the latest DSH core
      (`@deepseek-ai/dsh` 0.1.5-rc.1 at
      `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
      resolves in its place) paired with the repository-owned six-file
      adaptation of `dsh-plugin-cli-session` 0.2.0 at
      `0f487e74c81ed102c6899440d9f5d65e8e9eabda`, pinned by the declared
      `wrapper_digest` written at enablement, no older core, selected but
      disabled pending its exact-root, restriction and multi-message
      accounting proof, with session integration held separate from hands, and
      recording the admission of that repository-owned plugin (answer M) as
      extension-boundary source Brokkr never builds, loads or executes,
      conditional on 10.7 and 11.3 (5),
      re-imposed
      current restrictions (6), one confirmed launch in the v5 vocabulary,
      manifest dispatch from 0.10.0 under the amended boundary-record
      requirement, unchanged boundary stamping and the first-work hold (7),
      one proven pre-work replacement inside the existing bounds (8), current-only accounting and narrow legacy
      compatibility (9), and progress persisted before the next group with
      every tracked edit finished active, D9's one named assertion deferred
      explicitly to archive validation, the normal archive operation last,
      the full unfiltered workspace suite required on archived bytes before
      the delivery commit, and exact-head controller evidence external to the
      task state (10).
      For THE PROOFS, adopt the landed rulings 1/6 clarification for the
      entire site-facts family and affirmative current confinement. Retain
      `Status: proposed` and accepted 0030's safe supported rejoin; no decision
      amendment is commissioned. Clause 5 runs the existing decision-index
      check; adoption does not attest runtime compliance or fresh validation.
      The remaining amendment instructions are inherited history/acceptance.
      Preserve the existing ruling text that remains true; amend ruling 10 and
      its enforcement binding rather than replacing the decision or implying
      operator acceptance. For the earlier operator-ruling slice, first amend
      ruling 5, its directly conflicting rejected alternative and consequence in
      D10's specified order, citing the operator's ruling. Retain the caution
      for never-supported shapes and dated evidence. The ordered breakdown
      below owns this pending amendment; the inherited tick does not attest
      its execution or a new live measurement. Replace the consequences
      paragraph's stale
      "future DSH route" ambiguity with the exact selected extension route,
      its conditional admission and the bounded historical 0.1.2-rc.1
      one-shot result; do not repeat that result as a global DSH limitation.
      Append one dated 2026-09-10 Consequences note naming the operator's
      ruling, the reversed 0.1.0-rc.6 pin and the superseded
      `dsh-pair-qualification-010rc6.json` record as history, without
      rewriting that record's own bytes.
      Cite decisions 0006, 0009, 0016, 0030, 0034, 0041, 0042, 0043, 0046 and
      shipped 0053 where each ruling stands on them — 0009 for ruling 5's
      admission of six committed non-Rust files under `extensions/`, which
      rests on its statement that Rust-only describes this repository, not the
      extension boundary — and quote no accepted decision into a different
      meaning. Verify the decision remains `proposed`, cites 0009, its ten
      rulings and consequences agree with design D9/D10/D12/D13 and AS1's DSH
      scenarios, and `cargo test -p brokkr-cli --test decisions_index` passes —
      safety / AS1, progress / PM4, boundary /
      The seat record carries the boundary as seat-record/v4.
- [x] 1.2 Append the registry row to `docs/decisions/README.md` in number
      order with status `proposed`, and run
      `cargo test -p brokkr-cli --test decisions_index` so the derived
      index test proves the row is present exactly once — safety / AS1.
- [x] 1.3 State in 0056 the two limits the design refuses to hide: the
      local fingerprint cannot authenticate a provider account or detect
      a swapped credential home (`site / SR5`), and a charter instruction
      is not an engine guarantee that a model obeys it
      (`progress / PM3`) — site / SR5.

## 2. Seat record v5 (design D4)

- [x] 2.1 Write `contracts/seat-record.v5.schema.json` as v4 plus the
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
- [x] 2.2 Copy the same bytes to `crates/brokkr-store/src/seat-record.v5.schema.json`
      and add `SCHEMA_V5`/`CONTRACT_V5` beside their four siblings in
      `crates/brokkr-store/src/seat_record.rs` — evidence / LE2.
- [x] 2.3 Add `SeatRecordVersion::V5` and dispatch it from **0.10.0**
      in `SeatRecordVersion::of_engine` under the F7 boundary-record
      amendment: 0.9.0 up to but excluding 0.10.0 selects v4, 0.8.0 up to
      but excluding 0.9.0 selects v3, and older/unparseable engines keep
      v1. Preserve the existing parsing convention and direct validation
      of frozen versions, including v2 which no engine string selects.
      No package version is bumped. Verify the dispatch matrix in 2.6
      through all four fences — evidence / LE2, boundary /
      The seat record carries the boundary as seat-record/v4.
- [x] 2.4 Extend the built-in conformance in `seat_record.rs`, scoping
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
      valid. Scope the refusal-reason-only-with-`cold` rule on the same
      stamp, for the same reason. v4 admits `launch` and
      `resume_refusal` independently
      (`contracts/seat-record.v4.schema.json`), `step` is a free
      bounded string, and the one validator behind
      `crates/brokkr-store/src/lib.rs`'s fence judges third-party driver
      checkpoints as well as built-in ones — so
      `{"step":"resume-declined","resume_refusal":"incompatible-argv"}`
      and a row carrying `launch: resumed` beside a reason are both
      valid v4 rows that an unconditional rule would newly refuse under
      v5, at append, export, import verification and offline
      verification. Reading the shipped adapter arms establishes what
      *those arms* write, never what every producer of a 0.10.0 row
      wrote, so it cannot license an unconditional constraint. A row
      carrying `site_ref` may therefore carry a refusal reason only
      beside `launch: cold`; an unstamped row keeps its v4 meaning. No
      added constraint may fail a historical v4 row or an unstamped
      0.10.0 row.
      Diagnostics name contract and JSON pointer and never echo the
      rejected value — evidence / LE2, evidence / LE5.
- [x] 2.5 Use the one dispatch and validator at append, export, import
      verification and offline verification through
      `crates/brokkr-store/src/lib.rs`'s existing fence, so a private
      field, invalid enum or oversized identifier is refused before
      append and the attempt's failure is journaled through the existing
      path. Both unconditional invariants — that every `resumed` launch
      *this change emits* carries confirmed root evidence, and that a
      refusal reason *this change emits* appears only beside a cold
      launch — are enforced where the records are produced, in group 7's
      lifecycle and proven by the conformance suites of 5.8 and 9.7, not
      by refusing rows another producer already wrote. Design D4 says
      the same in one line: new **built-in conformance** requires root
      evidence for `resumed` and a refusal only with `cold`, and no
      added constraint invalidates historical rows — evidence / LE2,
      evidence / LE5.
- [x] 2.6 Tests in `crates/brokkr-store/src/tests.rs`: a v4 row and a v5
      row each validate against their own contract; an old 0.10.0 row
      with none of the new fields still validates under v5; the
      historical regression — a 0.10.0 codex checkpoint with
      `launch: resumed`, no `root_session` and no site stamp, in the
      exact shape shipped `codex_started` writes — still validates under
      v5 and is neither rewritten nor backfilled; the same row carrying a
      `site_ref` stamp **is** refused; the two unstamped
      refusal-bearing compatibility rows a third-party driver may have
      written under v4 — a checkpoint carrying `resume_refusal` with no
      `launch` at all, and one carrying `launch: resumed` beside a
      refusal reason — still validate under v5, and each **is** refused
      once it carries a `site_ref` stamp; an 81-character `id` and a
      flag-like `id` are refused;
      a Claude permission mode offered as `sandbox` is refused; export,
      import verification and offline verification agree with append on
      every one of these. Extend the existing version-dispatch tests as well:
      manifest engines 0.7.9, 0.8.0, 0.8.99, 0.9.0, 0.9.1, 0.9.99,
      0.10.0, 0.10.1, 1.0.0 and an unparseable string select respectively
      v1, v3, v3, v4, v4, v4, v5, v5, v5 and v1 at all four fences.
      Keep the tagged 0.9.0/0.9.1 no-boundary historical example valid
      under v4; test that a v5-only field under a 0.9-line manifest is
      refused under v4 rather than selecting v5 from its presence.
      Keep the synthetic third-party counterexamples labelled as
      contract tests, not provider telemetry — evidence / LE2, evidence / LE5,
      boundary / The seat record carries the boundary as seat-record/v4.
- [x] 2.7 Pin the v1–v4 seat-record bytes in
      `crates/brokkr-runtime/tests/frozen_contracts.rs` if any are not
      pinned yet, and assert `contracts/seat-record.v5.schema.json` exists
      beside them with title `Forge seat record v5`, exactly as decision
      0046’s v4 landed beside v3. Register v5 beside v4 in
      `contracts/README.md`; retain the v4 row and the published and
      embedded v1–v4 bytes. Run the frozen-contract and embedded-copy
      tests and compare the frozen files to the commissioned base —
      evidence / LE2, boundary /
      The seat record carries the boundary as seat-record/v4.
- [x] 2.8 Extend the existing integration coverage in
      `crates/brokkr-runtime/src/engine/boundary_tests.rs` and store
      record tests for the amended requirement’s boundary scenarios
      under v5: boxed exec finishing/success records carry namespace;
      hands-less exec/model sites carry `not applicable`; the engine’s
      current word replaces a driver’s word on model-bearing records and
      drops it without a model; panel aggregates carry none while member
      markers and a sequence’s ending result/marker carry their site’s
      word. Cover a model-bearing launch checkpoint under 0.10.0 as well
      as the existing finishing paths. A record offered directly to the
      v4 or v5 store fence with `boundary: chroot` must append nothing
      and keep the bounded schema-path/attempt-failure behavior. Run
      these existing suites; name which cases need host boundary proof
      rather than crediting a nested-box skip — boundary /
      The seat record carries the boundary as seat-record/v4.

## 3. Structural site identity (design D2)

- [x] 3.1 Add private `SiteKey`, `InstanceKey` and `ConfirmedSession` to
      `crates/brokkr-runtime/src/engine.rs`. `SiteKey` carries the outer
      seat, the selected case (with explicit absence for an unselected
      body), the body kind and the tagged path — single, panel member,
      sequence model step, or step plus panel member — from distinct
      name and index components taken off the compiled body walk. Never
      split a display tag on `:` to recover ancestry; never case-fold a
      recovered name — site / SR1.
- [x] 3.2 Derive `site_ref` as canonical JSON through the existing
      `brokkr_core::canonical` SHA-256, domain-separated with
      `brokkr.resume-site/v1`, to 64 lowercase hex; the run journal
      supplies run scope — site / SR1.
- [x] 3.3 Build `InstanceKey` from this site's agent, provider, model,
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
- [x] 3.4 Compose a per-invocation site context — key, selected
      candidate, current class, current restrictions — where the actual
      site is composed, and carry it through single dispatch, `MemberRun`
      and sequence model dispatch. Panel workers hand the context key
      beside their checkpoints to the single journal writer for stamping.
      A reused completed step invokes no driver and gains no launch. Adopt
      `4daaa7d`'s canonical readers, marker replacement/clearing and selected
      single's executing `site_name`. Unknown confinement never becomes a known
      no-hands instance. THE PROOFS clauses 2–3 exercise the existing composition
      at the six commissioned decisions; no dispatch rework is planned —
      site / SR1, SR2; safety / AS1, AS2.
- [x] 3.5 Add the compile-time uniqueness check for the existing
      flattened `Site` addresses within their actual candidate and
      boundary lookup scopes in `crates/brokkr-runtime/src/bundle.rs`, so
      step `a:b`/member `c` and step `a`/member `b:c` cannot alias; an
      ambiguous bundle fails before spawn with a message naming both
      sites. Ordinary repeated member names under different steps stay
      valid, and chain progression and historical tag meanings do not
      move. Adopt the delivered authoring census before evidence insertion,
      wrapper destination/validator claims before writes and final global census,
      including cases/defaults, inline/agent leaves and factless owners. Keep
      one canonical `SiteFacts` table and distinct-owner refusals naming the full
      label and both owners. THE PROOFS clause 4 removes only unreachable
      same-owner census tolerance under D10's enumeration invariant, retaining
      legitimate same-owner fact merges and the tested collision refusal —
      site / SR1, SR2; safety / AS1, AS2.
- [x] 3.6 Tests in `crates/brokkr-runtime/src/bundle/tests.rs`: the
      colliding pair is refused at compile time; two panels each holding
      an `alpha` still compile; every bundle under `recipes/` and
      `bundles/` walks clean. THE PROOFS clauses 2–4 cover only AS1's six
      compiled decisions/exchanges, independent confinement controls and the
      three commissioned coverage sites. Adopt existing collision/relocation
      controls; the wider F2 live-rename/identity campaign is not this visit.
      Each new test needs a compiling mutation failure at its claimed decision
      or refusal and a restored pass; direct shebang fixtures are Unix-only —
      site / SR1, SR2; safety / AS1, AS2; evidence / LE2, LE5.
- [x] 3.7 Tests in `crates/brokkr-runtime/src/engine/resume_tests.rs`:
      the canonical key is stable across process runs and moves for each
      identity axis of 3.3 taken one at a time; two sites with the same
      display tag and different ancestry produce different `site_ref` —
      site / SR1, site / SR2.

## 4. The eligibility query and its four topologies (design D1, D3)

- [x] 4.1 Replace `seat_session`/`resume_offer` in `engine.rs` with a
      pure query over this run's durable evidence: require a work-class
      model site, an unchanged pinned manifest and `Store::started_here`;
      find the newest checkpoint carrying confirmed root evidence for
      this exact `site_ref`; resolve its effect and attempt to this run's
      requested seat and durable start; require the engine-stamped site
      and instance facts. Keep the existing manifest refusal and
      indeterminate recovery ahead of dispatch — site / SR2, site / SR5.
- [x] 4.2 Compare the newest session's `instance_ref` with the current
      instance and stop there: an incompatible or ambiguous newest owner
      denies the offer and the query never searches behind it for an
      older matching owner. A failed invocation that recorded no session
      supplies no replacement; a start carrying only an assigned creation
      ID is not session-bearing evidence — site / SR2.
- [x] 4.3 Stamp `site_ref` and `instance_ref` from the invoking context
      when the engine writes a checkpoint, overwriting or removing any
      driver-supplied value exactly as the boundary stamp already does.
      An aggregate record never establishes a member's session
      ownership — site / SR2, evidence / LE5.
- [x] 4.4 Offer the complete validated provider ID through `Body::Resume`
      at all four executing work topologies — the single seat, each panel
      member, each sequence model step and each member of a sequence
      panel — replacing the three `session_ref: None` call sites of
      `run_driver`. Pass the originating harness identity to the
      adapter's private start context for group 8's version check — site / SR1.
- [x] 4.5 Withhold the offer from every gate-class invocation, including
      the single gate that the shipped code offers one today: a single
      seat or panel uses its selected compiled class, a sequence step its
      own compiled class, and a panel member inherits its enclosing
      panel's class. An office name and the presence of a later gate step
      decide nothing (decision 0042 ruling 2) — site / SR1.
- [x] 4.6 Keep deterministic exec and dialect validation steps outside
      the capability: no offer, no negotiation, no model launch — site / SR1,
      evidence / LE1.
- [x] 4.7 Tests in `engine/resume_tests.rs` asserting the actual wire
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
- [x] 4.8 Gate tests in the same suite for all four gate topologies —
      single gate, gate-panel member, gate model step, member of a gate
      panel step — on retry, re-entry and operator retry, each proving no
      offer and a fresh launch; plus the corrected historical single-gate
      case, whose journal rows are left unchanged — site / SR1, evidence / LE5.
- [x] 4.9 Recovery tests in `engine/resume_tests.rs` and
      `crates/brokkr-runtime/tests/recovery.rs`: a fresh engine process
      derives the same offer from the same journal, bundle and origin; an
      effect whose execution is indeterminate stays parked and the
      existence of a resumable session causes no automatic retry or cold
      replacement — site / SR5.

## 5. The correlated one-use offer (design D5, protocol half)

- [x] 5.1 Advertise `resume` as offer receipt from all four model
      adapters in `AdapterKind::supports` in
      `crates/brokkr-protocol/src/adapters.rs`; exec advertises none.
      Receipt is not a claim that any offered session can be resumed — site / SR4.
- [x] 5.2 Replace `serve_io`'s uncorrelated `offered: Option<String>`
      with a `PendingOffer { effect_id, attempt_id, handle }` whose
      handle is size-bounded. A matching start consumes it exactly once;
      an empty pending state is an ordinary cold start — site / SR4.
- [x] 5.3 Poison the exchange on a duplicate offer, malformed
      correlation, wrong effect or attempt, a resume before negotiation
      or a malformed resume envelope: launch no provider, send a bounded
      protocol failure when the correlation is known, otherwise end the
      exchange without pretending a provider refused. Never repair a
      poisoned exchange into a cold execution. Cancellation, shutdown,
      EOF and a completed invocation discard pending state — site / SR4.
- [x] 5.4 Treat a correlated string that fails the provider ID grammar as
      a declined offer with `invalid-session-id`, whose cold substitute
      independently passes current policy — site / SR4, evidence / LE1.
- [x] 5.5 Keep driver protocol v1 and `run_attempt_resuming`'s ordering
      in `crates/brokkr-protocol/src/process.rs` unchanged; a driver that
      does not advertise receipt is sent only its normal start, and the
      handle never travels in the prompt, the rendered context or a
      policy input — site / SR4.
- [x] 5.6 Carry the current attempt's prompt, result destination and
      scoped hands configuration with every rejoin, so an old
      conversation's remembered result path or grant cannot select this
      attempt's output or permissions — site / SR4, safety / AS2.
- [x] 5.7 Tests in `crates/brokkr-protocol/src/process/tests.rs` and
      `crates/brokkr-protocol/src/adapters/tests.rs`: negotiation then
      one correlated offer then one start; a second start with no offer
      in front of it starts cold; an offer whose effect or attempt
      differs from the next start reaches no provider and survives to no
      later start; two conflicting offers before one start reach none;
      cancel, shutdown and EOF discard the pending offer; the private
      start context is not rendered into the prompt — site / SR4.
- [x] 5.8 Extend `crates/brokkr-cli/tests/driver_conformance.rs` with the
      receipt capability and the one-use exchange for every built-in
      model adapter — site / SR4, evidence / LE5.

## 6. Adapter support declarations (design D5, declaration half)

- [x] 6.1 Add the typed `resume` assessment to the adapter declaration in
      `crates/brokkr-runtime/src/agents.rs`: per named execution shape, a
      status of `unmeasured | unsupported | supported`, an assessed
      identity, the applicable classes, boundaries and hands mode, the
      evidence references, and the measured limitations. The assessed
      identity takes one of two explicit forms, because AS1 requires an
      honest unmeasured declaration to be writable *before* anything is
      measured (`Neither local nor supplied interface evidence exists`):
      a **measured** identity naming the assessed CLI or wrapper version
      and the installed version it applies to, or the explicit
      **unknown** identity carrying a bounded non-empty reason. A
      `supported` entry must carry a measured identity and name its
      interface, restriction, exact-root and current-accounting
      evidence — root evidence stating whether that root persists, and a
      wrapper shape qualified on its own wrapper rather than on what it
      wraps (design D5) — so an unknown identity can never be supported — which is
      what leaves 8.1 a pinned version to compare the observed one
      against wherever resume is enabled. An `unsupported` entry must
      carry its bounded measured reason. An `unmeasured` entry carries
      either form: unknown with its reason where nothing is measured
      yet, or a measured identity that does not qualify the installed
      version. Absence and malformation are two different outcomes and
      6.6 tests them as two: an **absent** `resume` key, or a shape the
      assessment does not name, resolves to `unmeasured`, loads, and
      enables nothing — the site compiles and invokes cold. Data that is
      **present and malformed** — a bare `true`, a status outside the
      three tokens, a supported entry missing a measured identity or one
      of its four evidence references, an unsupported entry with no
      measured reason, an assessment carrying neither identity form, an
      unknown identity with no reason — is not normalized to
      `unmeasured`; the loader refuses it (6.2), so an authoring error
      cannot pass as an honest "not yet measured". An explicitly unknown
      identity inside an otherwise well-formed `unmeasured` assessment
      is **not** malformed: it is that honest declaration, and treating
      it as an authoring error would make the preparatory declarations
      of 6.4 unwritable. Neither outcome ever enables resume
      (design D5) — safety / AS1.
- [x] 6.2 Validate that shape in the loader and let the existing adapter
      content digest pin it, so a declaration edit moves bundle identity
      as it does today. The shape stays closed and compact — the
      admission-relevant fields of 6.1 and nothing behind them: no
      evidence database, no probe DSL, no version-range resolver and no
      path by which a declaration enables itself (design D5, D10) — safety / AS1.
- [x] 6.3 Carry the selected assessment through `Candidate`/`SiteSpawn`
      into the driver's private context inside `Start.input`, separate
      from the rendered `context`, the phase inputs and the resume
      handle. Adopt `4daaa7d`'s one owned `SiteFacts` value containing
      `inline_resume`, `pin_drivers`, tri-state `hands`, agent record and driver
      evidence, relocated once by structural ancestry. Runtime readers share
      that authority; manifest maps remain projections. Keep the independent
      `resume_gate` refusal on missing/invalid markers. THE PROOFS clauses 2–3
      prove the six compiled decisions and supported exact-root launches;
      broader witness-invalidation proof is outside this visit. No wire type,
      public gate API or label fallback is added — safety /
      AS1, AS2; site / SR1, SR2, SR4; evidence / LE2, LE5.
- [x] 6.4 Retain this completed declaration history. For CODEX, END TO END,
      11.1 below owns the Codex-only 0.154.0 reconciliation; the following
      older version/scope instructions do not override AB/D10. No other
      provider declaration changes in this slice — safety / AS1.
      Write the assessments into `adapters/codex.json`,
      `adapters/claude.json`, `adapters/dsh.json` and
      `adapters/lanetally.json` with their honest status as of this
      change, each one loadable now under one of 6.1's two identity
      forms and none of them waiting on group 10. Under proposal W, preserve
      Codex `work-site` as **supported** because main already rejoins it under
      accepted 0030. After 1.1's proposed-decision amendment and 8.10's
      failing shipped-retry control, set `boundaries: ["harness"]` and
      `hands: "none"` as design D10's F1 return requires: the composed
      workspace-write work seat has no boxed-hands marker. Retain measured
      `version: 0.148.0`, applicable `applies_to: 0.153.4`, classes and
      existing evidence; preserve every existing limitations byte/order.
      Add D10's exact September 10 partial accounting reference and append
      its dated harness-scope/boxed-cold limitation. Cite the September 15
      operator ruling and pending 10.5 proof in the reason. Keep the scalar
      hands type, loader, `resume_gate`, `qualify`, engine markers and argv
      allow-list unchanged for that historical declaration correction. Y/D10
      now requires clause 3's engine-marker and adapter absence-guard changes;
      the scalar declaration grammar and argv allow-list still stand.
      The other three reasons explicitly
      say main does not perform their rejoin; preserve all their non-reason
      bytes, identities, evidence and scope. Claude's
      boxed-workspace shape **unmeasured** against the *measured* 2.1.266
      interface identity in
      `.forge/controller-host-provider-interface.json`, with the reason
      naming the enforcement proof 10.6 still owes; DSH's headless work shape
      **unmeasured** with the measured resolved-composite identity and a
      bounded reason naming the forward-pinned latest core (`@deepseek-ai/dsh`
      0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release
      answer N1 resolves in its place) paired with the repository-owned
      six-file adaptation of `dsh-plugin-cli-session` 0.2.0 at
      `0f487e74c81ed102c6899440d9f5d65e8e9eabda` under
      `extensions/dsh/plugin-cli-session/`, their supported `--session`
      extension route, and 10.7's still missing exact-root, restriction and
      current-accounting proof, with no `wrapper_digest` set until 11.3
      writes it. Repoint DSH's `evidence.interface` to
      `.forge/tasks/dsh-pair-qualification-015rc1.json`, or, while that record
      does not exist, to `.forge/tasks/controller-dsh-upstream-discovery.json`
      and `.forge/tasks/dsh-pair-incompatibility.json` with the qualification
      named pending, so no evidence member cites the superseded 0.1.0-rc.6
      record as current. Append one dated 2026-09-10 `limitations` entry naming the
      operator's ruling, the reversed 0.1.0-rc.6 pin and the superseded
      `dsh-pair-qualification-010rc6.json` record as history, keeping every
      earlier entry's bytes and order unchanged. Preserve the installed
      0.1.2-rc.1 result only as a version-bounded one-shot limitation, keep
      the separate hands deferral unchanged, and do not mark the selected
      route `unsupported` or `supported` before 10.7.
      LaneTally remains **unmeasured** with the *unknown* identity and a
      bounded reason naming the wrapper identity 10.4 owes and 10.8's
      proof, never marked by analogy to Claude. 10.1–10.4 refine these
      identities; group 11 still owns full qualification and new enablement.
      W preserves shipping Codex support before 10.5/11.1 finish and completes
      neither. Verify current preservation through 8.10's production
      composition bridge and real driver exchange, with independent
      disabled-status and boxed-hands controls and full boxed-MCP cold argv.
      Also run the existing `tests/roster.rs` shipped-shape test through
      `Adapters::load`: Codex's harness/none scope, distinct measured/applicable
      versions and four references, and the other three disabled dispositions.
      Require the driver retry to confirm `launch: resumed`; a status or
      composition assertion alone is insufficient. This inherited tick does
      not attest execution of the current amendment or its controls.
      `adapters/exec.json` gains no assessment. Verify the four shipped files
      load through the existing runtime assessment tests, DSH resolves disabled
      with its measured version identity (`version` and `applies_to` equal to
      the resolved core) and no `wrapper_digest`, its `evidence.interface`
      names the 015rc1 record or the discovery and incompatibility records with
      the qualification pending and never `dsh-pair-qualification-010rc6.json`,
      and no declaration claims a global DSH limitation or still names
      0.1.0-rc.6 as the selected route — safety / AS1.
- [x] 6.5 Update the scaffolded adapter text in
      `crates/brokkr-cli/src/init.rs` and its expectations in
      `crates/brokkr-cli/tests/init_stacks.rs` and
      `crates/brokkr-cli/tests/init_doctor.rs` so a scaffolded workspace
      declares the same shape — safety / AS1.
- [x] 6.6 Tests in `crates/brokkr-runtime/src/agents/tests.rs`, holding
      6.1's two outcomes apart: each of the three statuses parses; an
      adapter with **no** `resume` key loads, resolves to `unmeasured`,
      and its site compiles and invokes cold; an `unmeasured` assessment
      carrying the explicit unknown identity and its bounded reason —
      the shape 6.4 writes for LaneTally — loads, enables
      nothing and invokes cold; an `unmeasured` assessment carrying a
      measured identity that does not qualify the installed version —
      6.4's Claude and DSH shapes, DSH's without `wrapper_digest` —
      likewise loads and enables nothing (text amended in place for analyze
      A1; the tick stands because this case already loads DSH's pre-11.3
      form, and answer O's optional-member cases belong to 8.8); a **present** bare `true`, an unknown status token, a
      supported entry missing a measured identity, a supported entry
      missing one of its four evidence references, an unsupported entry
      with no measured reason, an assessment carrying neither identity
      form and an unknown identity with no reason are each a loader
      refusal naming the offending field, not a silent downgrade to
      `unmeasured`; the digest moves when an assessment moves — safety / AS1.

## 7. The launch lifecycle (design D7, evidence half)

- [x] 7.1 Model the invocation as `plan -> child spawned -> exact root
      confirmed -> current work -> terminal` in the shared part of
      `adapters.rs`, with explicit resume outcomes: confirmed,
      conclusively rejected before work, failed after
      confirmation or work, and uncertain — evidence / LE1.
- [x] 7.2 Publish `launch: cold` only when the adapter actually took the
      fresh-session path, and `launch: resumed` only after provider
      evidence confirms the exact offered root before first work. A
      resume flag, a preassigned creation ID, a known old handle,
      surviving edits, replayed transcript rows and an exit status of
      zero prove nothing on their own. A confirmed fresh creation stays
      cold even when the engine or adapter chose its ID — evidence / LE1.
- [x] 7.3 Emit a bounded refusal reason exactly when an offer was
      declined and the launch is cold; invent no refusal where no offer
      was made; label no unconfirmed resume cold or resumed by
      guesswork — evidence / LE1, evidence / LE2.
- [x] 7.4 Hold root and launch candidates before `run_seat`'s existing
      first-work boundary, then flush `Accepted`, then the held confirmed
      launch and location facts in observed order, then the first work
      checkpoint. Publish one final launch per executing model site, not
      one per internal spawn. New root evidence uses a pre-work lifecycle
      step that must not satisfy `begins_work` — evidence / LE3.
- [x] 7.5 Keep the classified pre-session refusal path exactly as shipped
      under proposed 0053: a conclusive provider refusal before work and
      without delivery retains a failed result with no `Accepted` and no
      checkpoints, including no launch row, and a kill inside the held
      window loses those rows — evidence / LE3.
- [x] 7.6 Permit one cold launch after a local decline, and one cold
      replacement after a provider-rejected resume, and only on measured
      machine session-rejection evidence establishing that no session
      opened, no turn or tool action ran and no result was delivered. A
      generic nonzero exit, stderr prose, missing telemetry, a dropped
      connection or elapsed time establishes nothing. Read the whole
      invocation outcome first: an error-shaped notice followed by work
      or clean delivery is not a rejection, and its delivered work is
      retained — safety / AS4.
- [x] 7.7 Clear the rejected child's held launch, root, locator,
      model/effort and accounting candidates before the single
      replacement, report that replacement as cold with `harness-refused`,
      and never recurse. A failed replacement reports its own outcome;
      if it is itself a classified pre-work refusal, 7.5's
      no-checkpoint exception applies — safety / AS4, evidence / LE1.
- [x] 7.8 Share one outer driver process, its process-tree watchdog and
      the original deadline across both launches and the version check of
      8.1: the replacement gets only the remaining budget, no new attempt
      ID, no new chain slot and no new timer. Cancellation or deadline
      termination prevents a later replacement from starting, and a
      watchdog kill keeps `deadline_killed` rather than becoming a
      failure to start because acceptance was held — safety / AS5.
- [x] 7.9 Tests in `crates/brokkr-protocol/src/adapters/tests.rs` driving
      shim sequences: confirmation then work; conclusive rejection then
      one replacement; an error notice followed by delivery; a different
      root than the one offered; a post-work failure; a failed
      replacement; a cancellation racing the replacement; a rejection
      arriving near the deadline; a watchdog kill before any checkpoint;
      and the held-row order `Accepted`, launch, location, first work — safety / AS4,
      safety / AS5, evidence / LE3.
- [x] 7.10 Tests that a kill inside the held window fabricates nothing on
      recovery and that group 4's query then offers nothing from that
      attempt — evidence / LE3, site / SR5.

## 8. Provider planners (design D6)

- [x] 8.1 Probe the selected executable's version once per invocation
      through its measured version interface, with bounded output inside
      the existing deadline, and compare the observed version with the
      pinned assessment and with the originating root's recorded
      `harness_version`. LaneTally additionally checks wrapper identity
      and the underlying Claude version through a measured interface; no
      LaneTally version command is invented. A missing, changed or
      unreadable identity disables resume with `unverified-harness`. The
      observed version is recorded, never the desired pin. No per-attempt
      model experiment runs — safety / AS1.
- [x] 8.2 Compose the restriction plan in the engine from the declaration
      argv, the model and effort, the generated current hands fragment,
      the result door and the scoped resources, preserving provenance
      before flattening, and compare the actual expanded fragment against
      that plan in the adapter. User passthrough that merely resembles
      generated MCP settings is not authorized by resemblance — safety / AS2,
      safety / AS3.
- [x] 8.3 Apply a measured resume allow-list with exact arity, duplicate
      and precedence checks. Competing selectors, forks,
      background, cloud or worktree launch, extra positional handles and
      unknown restriction or profile overrides are not forwarded. Apply
      the same selector protection to the cold and gate paths, so an
      ambient `--continue` is never forwarded after an offer is declined;
      an unsafe cold setting fails before provider work. A resume-only
      incompatibility may select a known safe cold spelling;
      cold-inadmissible settings refuse rather than drop a restriction — safety / AS3,
      safety / AS2.
- [x] 8.4 Refuse an offered handle carrying a flag-like prefix, control
      characters, path traversal, shell syntax or a value outside the
      measured grammar: it is never passed as a selector, never truncated
      into another handle and never echoed into the journal — safety / AS3,
      evidence / LE2.
- [x] 8.5 Retain the shipped Codex planner. CODEX, END TO END strengthens
      its existing argv/confirmation/refusal assertions under 11.1, without
      opening boxed MCP admission or repeating the earlier preservation work
      — safety / AS1, AS2, AS3; evidence / LE5.
      Codex: keep `codex exec resume --json`, the workdir through
      `current_dir`, `-c sandbox_mode=...`, the pinned effort and the safe
      passthrough of `codex_resume_blocker`, and admit exact
      engine-generated MCP fragments separately from arbitrary `-c`.
      Re-express the class and effort on every rejoin, including decision
      0030's safe class override, rather than inheriting the old
      thread's. The earlier MCP-admission instruction is whole-change debt,
      not part of this slice: main's complete boxed MCP argv already refuses,
      and D10 retains that cold behavior and the existing allow-list. Proposal
      W's current preservation and refusal controls run under 8.10 using the
      actual resolved/composed harness workspace-write argv and shipped
      harness/none assessment, with no new installed-provider claim — safety / AS1,
      safety / AS2, safety / AS3, evidence / LE5.
- [x] 8.6 Claude: build the print/stream-json resume path with exactly
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
- [x] 8.7 LaneTally: share Claude's parsing where the measurement covers
      it, keep the wrapper and its capture marker, gate the planner
      separately and never substitute plain Claude to make resume
      work — safety / AS1, evidence / LE4.
- [ ] 8.8 Current final targeted repair is only R1–R4 within (a)–(c),
      in the ordered local groups 8.8.1–8.8.8 above under proposal AT and
      the current D10 sitting. Historical closure claims do not discharge
      these four findings or erase inherited evidence debts. The remaining
      text records whole-change acceptance, including excluded (d), not this
      slice's execution order. This checkbox remains open.
      After 1.1, 6.4, 11.5 and 13.1 establish the corrected disabled
      truth and 10.7's live half commits the adaptation, implement 10.3's
      forward-pinned DSH route for N1's resolved official core,
      `@deepseek-ai/dsh` 0.1.5-rc.2 with registry integrity
      `sha512-8Xc8hCQHcIWRmTCVU/xZdp6/qMsWMeAd2ObChKDEsfhUPJFXx6H0lgeb1DxUMD86HZrrVN+1bCvn1ppjZ/fOxw==`,
      with the repository-owned six-file adaptation of
      `dsh-plugin-cli-session` 0.2.0 at
      `0f487e74c81ed102c6899440d9f5d65e8e9eabda`, committed as bytes under
      `extensions/dsh/plugin-cli-session/` (`package.json`, `lib/index.js`,
      `lib/startup.js`, `cordis.patch.yml`, `README.md`, `LICENSE`) with the
      sibling provenance note `extensions/dsh/PROVENANCE.md`, whose isolated
      live cold/warm qualification and corrected canonical-install erratum
      are recorded in
      `.forge/tasks/dsh-pair-qualification-015rc2.json`. The adaptation
      changes exactly one expression, `lib/index.js` line 253, from
      `agent.session.events` to `agent.session.snapshotEvents(firstSeq)`;
      every other byte is upstream-identical, nothing is rebuilt, and no
      source, lockfile, test, build configuration or CI file is vendored.
      The current operator-ruling slice adopts these bytes, the never-publish
      note and the settled second-selection reader seam unchanged. Its ordered
      low-4 clause below adds the still-owed all-six-digest assertion and control
      under 8.10; it neither re-qualifies this pair nor completes 8.8
      (safety / AS1, site / SR3).
      Keep the admitted headless profile and Rust-owned persistence/model/
      effort overlay; use the plugin's explicit `--new --output-format
      stream-json` cold form and `--session <owned-id> --output-format
      stream-json` warm form. Extend only the private engine/start context so
      DSH receives an owned target containing the provider ID, the
      already-recorded persistence locator and its recorded home. Preserve the
      delivered `ResumeTarget`/`OriginatingRoot` and `start_context` path in
      `crates/brokkr-runtime/src/engine/resume.rs`: read `transcript.home` from
      the same confirmed checkpoint as the root and locator and carry it as
      `owned_target.persistence_home` at both existing callers in `engine.rs`.
      Never borrow a missing field from another checkpoint, site or older owner;
      missing evidence stays missing for the DSH planner to decline. Verify the
      runtime unit/integration cases assigned in 8.10 after repairing R2's
      serialized checkpoint transport; change the inherited carrier only if
      a failing same-checkpoint case demonstrates a gap. These observations
      precede the consuming planner cases (site / SR2, site / SR3, site / SR5,
      safety / AS2).
      Leave `Body::Resume` and driver
      protocol v1 unchanged, and publish `root_session` plus the complete
      `transcript` locator atomically on the same stamped launch checkpoint.
      Resolve that locator beneath the admitted originating DSH home, require
      its bounded relative form, depth-zero header and exact ID, and decline
      truncation, ambiguity, `..` or symlink escape without scanning for or
      creating a substitute root. Land the digest work first, in design
      D10's order. (a) Amend the loader in
      `crates/brokkr-runtime/src/agents/load.rs`, whose measured branch
      originally admitted only `version` and `applies_to`: preserve the inherited
      implementation in which `ResumeIdentity::Measured` gains an optional
      `wrapper_digest`, the measured branch's closed key list admits it with seat record v5's 64-lowercase-hex grammar checked
      at load, the unknown branch still admits `unknown` alone, and the
      member travels with the selected assessment into the private start
      context. (b) Implement one Rust function in `brokkr-protocol`, beside
      the DSH planner, that computes the plugin component as the SHA-256 of
      the six `<relative path>\0<file SHA-256>\n` lines in bytewise path order
      over a plugin directory — a missing file, or an extra entry other than a
      nested `node_modules/` the dependency identity already covers, is
      unreadable and names the drifted file — and the canonical composite as
      the SHA-256 of design D6's fixed `<component>\0<value>\n` lines: `core`
      (name, version, registry integrity), `node`, one deduplicated,
      bytewise-sorted `dependency` line per lock-metadata
      `name version integrity` triple normalized from npm's hidden
      `node_modules/.package-lock.json` and pnpm `pnpm-lock.yaml`. Apply D6's
      complete npm path grammar: consume every `node_modules/<package>` group
      left to right, treating the slash within `@scope/name` as part of that
      package, then take only the final complete package spelling, and
      take version and integrity verbatim from that same entry. Ignore an
      optional `name` field; reject malformed paths and missing, mistyped or
      invalid version fields. Apply D6's one shared source-scalar rule to
      names, versions, integrities, Node output, profile values and every other
      scalar entering a line: reject empty, NUL, space, tab, CR and LF without
      trimming. The inherited npm-version predicate is only one use of this
      rule; the complete 8.10 rejection-vector ledger remains pending.
      Deduplicate only equal complete triples, retain different versions/integrities, then sort their value bytes. Exclude the
      core's own entry and the plugin's local-tarball entry (also the local
      `brokkr-dsh-resume-policy` entry only when its composed bytes enter the
      conditional extension component). Follow those lines with `plugin`,
      `plugin-patch`, `profile-patch`, one
      `profile-bundle` line per `dsh.profile.bundles` entry in declared order,
      `profile-patch-reload`, `home-patch` (the home-level
      `$DSH_HOME/cordis.patch.yml` SHA-256 or `absent`) and, only if D6's
      conditional extension is named in the profile's bundles, `extension`.
      That package is `brokkr-dsh-resume-policy`, authored only after a
      demonstrated missing hook under `extensions/dsh/resume-policy/`, with
      `package.json`, `index.js`, `cordis.patch.yml` and `LICENSE` and its
      separate section of `extensions/dsh/PROVENANCE.md` as D6 specifies.
      Resolve its installed directory through the same bundle lookup, require
      it inside the profile, and hash that exact installed set with the same
      file-line function; missing/extra entries, symlinks or unsafe resolution
      are unreadable. No repository provenance is read at runtime and no
      second digest producer is added. It reaches its inputs only
      through D6's locators: the canonical executable must be the core
      package's `bin.dsh` script with the `#!/usr/bin/env node` first line;
      the core lock is only `<core root>/node_modules/.package-lock.json`, whose
      core entry carries the package's own version, with no root-lock fallback;
      `node` is selected by AO/D10's native child lookup, including the
      native default search when PATH is absent; preflight its loader and
      reuse that exact selection for `node --version`. Only
      `<home>/profiles/headless/package.json` supplies the profile manifest,
      with no search over other profiles, yielding `bundles` (a non-empty
      string array) and `patchReload` (`live` or `startup`); each listed bundle resolves by the
      loader's order (the core package's lookup paths, Node's global folders,
      then the profile's). Canonicalize the complete profile directory once
      for containment when reading it, retaining the original profile lookup
      anchor separately so that order does not change. Compare each first-hit
      canonical bundle directory against the canonical core root or canonical
      profile boundary; the plugin and conditional extension must lie inside
      that same canonical profile. A boundary or candidate that cannot be
      canonicalized is unreadable; no raw-path fallback, string-prefix check
      or search past an outside first hit is permitted. Preserve the inherited
      two-path canonical-boundary correction and prove the symlinked-home case
      in this slice; the complete 8.10 containment ledger remains pending. The
      pnpm lock is read before allocation through D6's inclusive 8 MiB bounded,
      fail-closed line reader with no YAML crate; and the plugin
      lines are in bytewise path order. `cordis.yml`, the raw `package.json`,
      `pnpm-workspace.yaml`, `.env` layers, persisted state and the per-seat
      overlay never enter it. Another entry without a registry integrity, a
      value holding a NUL or newline, a layout outside those locators, or an
      unreadable component makes the identity unreadable. Read every
      identity-bearing source once per call, reuse the retained hidden-lock
      and plugin-patch bytes, and expose only `dsh_composite` as the production
      producer; private parsing/hashing and test seams do not become competing
      producers. (c) Extend `brokkr doctor`'s existing `dsh`
      line in `crates/brokkr-cli/src/doctor.rs` to report, through the
      adapter's own seam resolution, the composite's digest or its
      unreadable component and whether it equals, differs from or has no
      declared `wrapper_digest`. It is informational until a `supported`
      shape declares one, then a warning on difference or unreadability. It
      reads no credential or settings file, and the guide's doctor sample
      follows it; it spawns only the `dsh` and `node` version probes. The report
      describes the composite as read for launch, not lifetime integrity;
      D6's accepted later same-host patch-mutation residual adds no continuous
      verifier or exemption from the qualified-mode and current-restriction
      proof. (d) Complete the inherited `dsh_launch`/`dsh_launch_with` planner
      in this order, with the corresponding 8.10 cases beside each repair.
      On this successor, preserve the implemented guards and close the missing
      observations under 8.10; production changes require a demonstrated B
      failure. Adopt the controller's R1 reader separation unchanged. R3's
      staging observation is a private `#[cfg(test)]` thread-local counter
      at entry to `dsh_seat_overlay_in`, before either settings or patch
      creation; reset per synchronous planner call and calibrate on a positive
      plan. Keep the production staging location and planner signature.
      Pass B adopts (a)–(c); it adds no digest producer, public planner seam,
      dependency, generic argument grammar or live-provider qualification.
      First extract and validate the authorized DSH inputs: exactly one
      separate-value `--model <id>` where present, the shared effort splitter's
      existing separate or equals-joined form, and, where supplied, one exact
      `--patch <value>`.
      Refuse `--model=<id>`, duplicate/malformed/valueless model or effort
      controls, and effort without a model, before any route read, version
      probe, composite call or staging. Keep the shared splitter's behavior
      for other adapters. After extraction, refuse every residual argument,
      including selectors, profile/settings/output overrides, extra positional
      text, `--`, aliases, joined/clustered forms, `--from-default-profile` and
      unverified `--verbose`; the inherited selector-only deny-list is not the
      admission rule. Use fixed field/category diagnostics in model validation,
      `split_dsh_patch`, `dsh_control_conflict` and storage refusals; never echo
      an unknown option name, joined value, model, ID or path. Prove these
      refusals on cold, offered and disabled paths with private marker values
      absent from the errors (safety / AS3, evidence / LE2).
      Next retain D5/D6's delivered route binding and reader exactly as answers
      Q-S and AS3 specify; this task does not re-derive their byte-exact grammar.
      At every model-site start, cold, offered or `unmeasured`, independent of
      the resume gate, at both `start_context` call sites: bind the seat's
      single `--patch` value from the argv, the run's working directory and the compiled leaf
      layer's directory and manifest to a `files` member of that manifest,
      withholding the binding for an absolute path, `..` or symlink escape and
      any value the leaf layer does not carry as its own `files` member (an
      ancestor layer's aggregate digest does not bind, and the bundle-relative
      `./` spelling expands to an absolute path and receives no binding);
      carry the argv value and that member's
      64-lowercase-hex digest as the private `route_overlay` beside the
      assessment and owned target. The binding is the engine's alone: it is an
      input of `start_context` in `crates/brokkr-runtime/src/engine/resume.rs`,
      computed in `crates/brokkr-runtime/src/engine.rs` at the single-site call
      site in `run_driver` and at the panel-member call site where each
      `MemberRun` is composed, from the compiled manifest's `files` entry and
      never from a hash of the file the value resolves to, so a member edited
      since compilation still travels with the manifest's recorded digest and
      the adapter's digest comparison below is what refuses its bytes. In the
      adapter, read the file once from the
      working directory, require SHA-256 equality with the bound digest before
      any shape check, then apply AS3's closed, data-only line reader — no tag,
      anchor, alias, flow collection, block scalar, merge key or quoted scalar
      at any depth, refusal naming the offending depth — and its closed field
      set (`displayName`, `api`, `baseURL`, `compat`, `models`, `apiKeyEnv`) with
      `apiKeyEnv` required as an environment-variable name and `baseURL`, when
      present, the closed lowercase-`https` endpoint grammar excluding every
      credential-bearing position and character. Require the binding and its
      argv agreement before reading, digest equality before shape validation,
      and recognize those same once-read bytes before staging. Every other
      `--patch` refuses through the existing pre-work failure path, never
      forwarded or dropped (safety / AS3).
      Then close every disabled assessment gate before the version probe or
      composite producer, with or without an offer. Missing/malformed declared
      `identity.wrapper_digest` also prevents either observation. Only an
      enabled shape with a valid declared digest probes the selected binary's
      version once, bounded inside the existing deadline; only a matching
      `applies_to` reaches the sole Rust producer. Compare the recomputed
      canonical composite with the declared digest. On an offer compare both
      observations with `resume_context.originating_harness_version` and
      `originating_wrapper_digest` from the same confirmed root. Missing,
      mistyped, malformed, unreadable or mismatched evidence declines an offer
      as `unverified-harness`; a disabled assessment retains its own bounded
      refusal reason. Preserve the engine's `root_session.wrapper_digest` to
      `resume_context.originating_wrapper_digest` binding beside the originating
      version. Carry observed core version as `harness_version` and composite
      as `wrapper_digest` in the plan, separately from desired pins; their
      confirmed recording and assessment/instance association remain required.
      An origin mismatch follows the current observations; it does not imply
      zero producer calls (safety / AS1, site / SR5).
      Only after identity agreement admit the complete owned target: require
      string provider ID, locator and `persistence_home`, with the provider ID
      equal to the negotiated ID. Canonicalize the recorded and current admitted
      homes and require equality before retained-session reads; a canonical
      alias is equivalent, but a missing, unresolvable or different home declines
      as `unverified-harness`. Never switch homes to honour an offer, even when
      both homes contain the same ID and locator. Resolve the offered locator
      losslessly beneath that home and enforce the existing 80-character bound
      using Rust `chars`, not UTF-8 bytes. Do not truncate, rewrite separators
      or use lossy path conversion to obtain a different address; validate the
      planned locator before the shared `Transcript::record` clamp and before
      staging too, keeping that clamp unchanged for other producers.
      Require canonical containment of each project/session directory before
      enumerating it and of the selected regular `session.v3.jsonl` before opening
      it, within the owned root as well as the admitted home. A contained alias
      may pass; an escape into another root inside the same home must fail.
      Complete the exact-one valid depth-zero header check for the offered ID,
      refusing missing, malformed/truncated, unreadable or ambiguous evidence
      instead of skipping unsafe candidates or finding a substitute. Give the
      directory enumeration and existing pre-spawn header/sequence reads
      explicit finite DSH-local budgets and bounded IO; a size check after an
      unbounded allocation is insufficient. A truncated, invalid or unreadable
      boundary must decline, never become zero or a partial-prefix maximum.
      Preserve the inherited `firstSeq` convention and all current-event folds;
      B repairs admission reads only (safety / AS2, safety / AS3, site / SR3,
      site / SR5; design D6).
      Finally settle the persistence root and lossless locator, then fold the
      validated route ahead of the transcript/model/settings rows in
      `dsh_seat_overlay_in` and stage exactly one overlay. Qualified cold builds
      `--new --output-format stream-json`; qualified warm builds exactly
      `--session <owned-id> --output-format stream-json` on the originating
      root. Both retain the admitted `headless` profile, current workdir and
      Rust-owned model/effort/settings. A disabled gate or identity/storage
      mismatch builds the shipped `dsh --profile headless --patch <overlay>`
      cold command under the current home, with no `--new`, `--session` or
      `--output-format`, no rejoining target and no offerable root. Only a
      declined offer carries a refusal token; no-offer cold has none. The
      bound current route folds on qualified cold, warm and disabled/mismatched
      cold alike. No route byte or binding enters the composite, launch row or
      journal. Retain `confirms_from_locator: false`: a plan and stored header
      are not launch confirmation (safety / AS1, safety / AS2, safety / AS3,
      evidence / LE1, evidence / LE2).
      Preserve `1d21319`'s completed roster-comment correction: the route is
      the overlay's and the fetch grant is the composed profile's own (decision
      0044 ruling 5's erratum). Do not move the roster assertion, `bundle.json`,
      `research-web.yml`, compiled staffing or research-dsh witness digest.
      The remaining launch half of (d) belongs to Pass C, after B; preserve its
      inherited partial implementation without extending or crediting it here.
      Confirm the launched root before publishing: do not treat the
      plugin's request-derived `session_id` value by itself as
      confirmation. Require a valid prior depth-zero header retained at
      the resolved locator for the offered ID, the pinned plugin's
      post-`await agents.resume` init event read from the stream-json
      child on that selected persistence root, no fresh sibling
      root/session in the retained store, and new sequence activity past
      the recorded `firstSeq` in that same root, before the launch hold
      releases. Same-root nonce continuity is 10.7's probe-only
      model-recall device (design D6): it is never planted in a prompt or
      read at run time, and this run-time confirmation names only the
      mechanically observable header, init event, sibling-root and
      sequence facts above. A missing or different root, whether followed
      by a clean child exit
      or by a delivered result file, remains failed or indeterminate
      under D7: it publishes no `root_session`, no `transcript` locator
      and no launch row, and authorizes no cold replacement by itself.
      Never forward the launcher's TUI example, treat the retained
      directory as a provider handle, alter the live global pin/profile, add
      an SDK runner or admit hands. If 10.7 demonstrates that the documented
      setup or pre-work observation hook is still insufficient after the
      adaptation, return this task to pending and add only the narrow Cordis
      extension D6 permits through the documented API at
      `extensions/dsh/resume-policy/`, record its authorship, missing-fact probe,
      documented hook/core version, limited behavior, licence and exhaustive
      per-file digests in the sibling provenance note, include its installed
      set in the composite identity, then rerun 10.7; do not patch provider packages,
      fork the plugin a second time outside this adaptation or intercept
      UUIDs. Verify with the DSH planner/storage shim cases in 8.10 and 9.6;
      the route-overlay binding's engine cases in the runtime crate, which
      `adapters/tests.rs` cannot supply because the adapter receives only
      argv, a working directory and the private context: in
      `crates/brokkr-runtime/src/engine/resume.rs`'s test module, beside
      `the_private_context_carries_the_owned_target_and_originating_digest`
      and in its pattern, that `start_context` carries a supplied binding as
      `route_overlay` with exactly the argv value and digest it was handed
      and carries no `route_overlay` when none is supplied; and in
      `crates/brokkr-runtime/src/engine/resume_tests.rs`, the engine
      integration cases 8.10 assigns to that suite, read off the `Start.input`
      that suite's logging drivers actually received at both call sites, a
      single site and a panel member; also verify the originating-home
      propagation in that unit case and the runtime checkpoint/actual-Start
      cases named in 8.10, independently of the delivered route binding;
      the loader cases in `crates/brokkr-runtime/src/agents/tests.rs` (a
      measured identity without the member loads, a well-formed member loads
      and is carried, a malformed member or one beside `unknown` is refused
      naming the field, and the adapter content digest moves when the member
      moves); doctor cases in `crates/brokkr-cli/src/doctor/tests.rs` for a
      matching, differing, undeclared and unreadable composite; and the
      committed-bytes test that the adaptation directory holds exactly the
      six files, the function's per-file lines carry the provenance block's
      path and SHA-256 pairs, the adapted expression occurs exactly once, the
      recomputed delta digest equals the note's, and substituting the upstream
      expression back reproduces upstream `lib/index.js` SHA-256
      `a40b52b3891485821ad01b00c322006abee8a51a0d4a2ae4ddb8427a0183d99b`,
      before ticking. Full 8.8 remains pending through C/D; completing B
      alone never ticks it — safety / AS1, safety / AS2, safety / AS3, safety / AS4,
      site / SR2, site / SR3, site / SR5, evidence / LE1, evidence / LE2,
      evidence / LE3.
- [x] 8.9 Implement SR3's two identity origins: harvest the
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
- [ ] 8.10 This whole-change acceptance remains pending and is not a new
      implementation task in CODEX, END TO END. Reuse its landed Codex test
      seams under 11.1; do not repeat AA's production repairs or open C/D
      — safety / AS1, AS2; site / SR1; evidence / LE5.
      Earlier THE PROOFS scope: execute only AA/D10's two items in clauses 1–6
      below: six wrapped/unwrapped compiled Codex gate decisions and supported
      exact-root exchanges, then `bundle.rs`'s two reachable refusal tests and
      unreachable census-arm consolidation. Every new test needs an observed
      compiling mutation failure at its claimed assertion and a restored pass.
      Extend the existing runtime/protocol/CLI suites using the test-only seam;
      do not substitute fabricated roots, repaired markers or map assertions.
      A no-offer refusal is judged by the private production gate with actual
      composed facts. Namespace/boxed remains refused; preserve shipping
      harness/none and no-hands live controls. Directly executed shebang tests
      are Unix-only or use a real target-platform executable. Clauses 5–6 own
      fresh gates, coverage and the unsigned evidence commit. Keep this checkbox
      pending for inherited C/D work — safety / AS1, AS2; site / SR1, SR2,
      SR3, SR4; evidence / LE2, LE5; progress / PM4.
      The following provider-planner and operator-ruling breakdown is inherited
      acceptance/history, not additional work commissioned by this slice.
      Complete each provider-local planner guard and its tests in
      `crates/brokkr-protocol/src/adapters/tests.rs` from the captured grammar,
      and the engine's private-target and route-binding cases in the runtime
      suites below. Keep exact arity, duplicate and precedence checks for every
      authoritative restriction on cold and resume paths without a generic
      provider grammar. Preserve the completed Claude cases that independently
      refuse a second or last-wins permission mode, tools list, strictness/MCP
      document, allowed/disallowed-tools list (including aliases), model or
      effort control. The current operator-ruling breakdown executes only
      shipped-Codex harness rejoin preservation and its fail-closed controls:
      first the composition bridge in `engine/boundary_tests.rs` and the
      existing agent suites, then the shipped assessment/production-composed
      argv exchange in CLI `tests/driver_conformance.rs`, with separately
      observed disabled-status, boxed-hands, harness-fragment and boundary-mark
      mutation failures. The full resolved boxed MCP argv stays cold. Run the
      existing runtime `the_seat_input_names_the_boundary_and_the_marker_only_under_a_box`
      and `no_gate_topology_is_ever_offered_a_session` checks beside that bridge:
      absent configured hands supplies neither marker nor boundary, while a
      harness gate with hands still receives no offer (site / SR1, evidence / LE2).
      Then the actual DSH cold planner boundary, Codex refusal cause, Codex
      cold selector and six-file digest controls, in that order. For low 3,
      pair its selector mutation with the existing protocol adapter test
      `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`, verifying
      two children, no selector and the retained sandbox in the replacement
      argv, and one cold launch row (safety / AS3, AS4, evidence / LE3).
      It adopts the settled reader seam. Each clause names its tests,
      requirement and observed failure/pass;
      retain all inherited fixes and leave 8.10 unchecked. These clauses add
      acceptance detail under safety / AS1, AS2, AS3 and AS4, site / SR1, SR2,
      SR3 and SR5, and evidence / LE1, LE2, LE3, LE4 and LE5. The following
      Pass B breakdown is inherited acceptance,
      not work to repeat; C/D paragraphs remain pending and unscheduled.
      Verify each applicable case alongside its 8.8(d) implementation before
      advancing; the full 8.10 checkbox still awaits C/D.
      First repair R2 in `engine/resume_tests.rs::dsh_model_driver`: use
      `serde_json` to serialize one complete checkpoint-data JSONL row per
      declared invocation, including that invocation's ID, into a temporary
      data file beside the existing verdict file. The shim selects its indexed
      row and emits it as a `%s` argument in a fixed format; arbitrary payload
      bytes never enter shell source or the format string. A missing row fails
      explicitly, never repeats the last checkpoint. Declare enough rows for
      panel re-entry even when its verdict legitimately repeats. Exercise the
      bytes this same shim actually emits with a representative Windows home
      and quotes, percent signs, backslashes and an embedded newline; decode
      JSON and assert exact fields and one JSONL frame per checkpoint. Treat
      that home as data, without creating a Windows-shaped directory on POSIX
      or rewriting separators. Preserve and run both existing tests
      `an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` and
      `an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member` on
      their actual temporary homes. This is portable transport evidence;
      native Windows/macOS evidence stays pending controller CI (site / SR2,
      site / SR3, site / SR5, safety / AS2, evidence / LE2).
      Then prove the originating-home carrier in
      `crates/brokkr-runtime/src/engine/resume.rs`'s existing
      `the_private_context_carries_the_owned_target_and_originating_digest`
      test and checkpoint tests in `engine/resume_tests.rs`: provider ID,
      locator, home, version and digest describe the same confirmed checkpoint;
      use distinct old/new ID, locator, home, version and digest values plus
      another site's row, then absent/mistyped latest fields to expose mixing
      between `eligible_offer` and `originating_root`. A missing/mistyped home
      or locator never borrows from an older checkpoint or another site; an
      incompatible newest owner supplies no offer. Preserve origin-identity
      absence for planner refusal instead of filling it from another row.
      In `engine/resume_tests.rs`, use the existing
      bundle/logging-driver pattern to read actual `Start.input` at both
      production callers, a single site and a panel member: an offered start
      carries the exact five coordinates from its selected checkpoint, including
      `owned_target.persistence_home` and both originating identity members,
      and a no-offer start carries no owned target. Directly constructing both
      context objects in a unit test does not establish their journal
      association. Retain the two scans unless a failing case requires a B
      correction. Assert the private
      carrier is absent from rendered prompt/context and is not copied as
      `resume_context` or `owned_target` into launch evidence; retain the existing
      confirmed `root_session` and `transcript` fields. Preserve gate/no-offer
      behavior and Pass A's binding cases independently (site / SR2, site / SR3,
      site / SR5, safety / AS2, evidence / LE2).
      Before the protocol matrices, add D10's private test-only thread-local
      counter at `dsh_seat_overlay_in` entry, before settings or patch creation.
      Reset it for every synchronous `dsh_launch_with` call and require one
      on positive plans to calibrate it; every pre-observation control/route
      refusal requires zero. Keep the recording version shim and counted or
      panicking sole-producer closure. Use no process-global counter, directory
      scan or new planner signature. Origin/storage declines can legitimately
      observe identities and stage one safe cold plan; the zero rule does not
      apply to them (safety / AS1, safety / AS3, evidence / LE2).
      Then exercise `dsh_launch_with` in temporary homes for exact authorized
      model, effort and patch inputs and every competing residual category:
      duplicate/missing/invalid model and effort, equals-joined model, mixed
      effort spellings, effort without a model, bare/duplicate/odd patch,
      session/new/resume/list/profile/workdir/output/settings controls,
      `--from-default-profile`, `--verbose`, unknown option names, option
      terminators, positional text and short/joined/clustered forms. Retain
      both existing effort spellings as positive cases, with no guessed aliases.
      Exercise cold, offered and disabled paths. Pair bad controls with a route
      that would fail to read and require the control refusal first; record
      zero version calls, a never-called composite closure and no staged overlay
      or retained-root allocation on these failures. Include synthetic private
      markers in rejected option names, equals-joined values, malformed
      model/path/selector-control values and odd patch spellings; assert no
      diagnostic echoes them (safety / AS3, evidence / LE2).
      Complete the DSH route overlay's
      deterministic cases (answers Q-S), split by the crate that can observe
      each. The engine-side binding cases belong to the runtime engine suite,
      `crates/brokkr-runtime/src/engine/resume_tests.rs`, in the pattern of
      the private-context unit case
      `the_private_context_carries_the_owned_target_and_originating_digest`
      in `engine/resume.rs`: each builds a bundle through that suite's
      `bundle` helper, whose `dir` and `manifest.files` are what the engine
      binds against, with a site the binding rule applies to (a dsh site
      whose compiled command carries one `--patch`, design D5) and that
      suite's logging driver standing in for the provider process, plants the
      named file under the run's working directory, and reads the
      `resume_context.route_overlay` member off the `Start.input` the driver
      actually received — at both production `start_context` call sites, a
      single site and a panel member, and for the positive case on a cold
      start, an offered start and a start under an `unmeasured` assessment
      alike. (i) A valid leaf-manifest member carries the actual argv value
      and that member's compiled manifest digest. (ii) A same-shaped
      nonmember outside the layer, a working-directory shadow of the bundled
      path, an ancestor-layer file, a `..` component and an in-layer symlink
      whose target resolves outside the layer directory while remaining
      inside the working directory (present at compilation and therefore a
      `files` member), and the absolute path produced by `./` expansion each
      receive no binding: the context carries no `route_overlay`, at either
      call site. (iii) A member whose bytes changed
      after compilation cannot authorize its new bytes: the carried digest is
      the manifest's recorded value, not a hash of the resolved file, at
      either call site. The adapter suite, `adapters/tests.rs`, which receives
      only argv, a working directory and the private context, owns the
      remaining planner cases: call `dsh_launch_with`, reusing the existing
      `route_overlay.rs` reader vectors and the shipped `recipes/research-dsh`
      overlay as the positive vector on qualified cold, qualified warm and
      disabled cold, plus identity-mismatch cold. Use its shipped model/effort
      and compiled-file binding, asserting each planned command, exactly one
      `--patch`, one staging call, unchanged reasoning-level rows and the route
      rows before every Rust-owned persistence/model/settings row. Include
      declared-composite mismatch without an offer (no refusal token) and
      originating-identity mismatch with an offer (`unverified-harness`), with
      the version/producer counts appropriate to each comparison. The resulting
      overlay is the observation; the existing helper-only ordering test and
      synthetic `contains` assertions do not discharge it. Keep the composite,
      launch row and journal free of route bytes. Reuse and extend reader
      vectors for every named class below through all three planner paths.
      Run each negative on cold, offered and disabled planning, requiring a
      pre-staging refusal naming a depth, field or URL part and never a value:
      a second or bare `--patch`; an absolute, `..`, symlink-escaping,
      non-regular, oversized or non-UTF-8 path; an absent binding beside a
      present `--patch`, a binding without `--patch`, a disagreeing binding,
      and a bound digest the bytes read do not hash to — the two outcomes the
      engine cases hand the adapter, no binding for a nonmember, shadow,
      ancestor-layer file or `./` expansion and the manifest's digest against
      changed bytes, so each of those five refusals is proven end to end by
      the two suites together and never by adapter cases in place of engine
      ones; a
      Rust-owned or foreign row ID, a second entry or provider, a provider the
      seat did not pin, an absent model pin or one without a provider segment;
      each field outside the closed
      six-field set, a literal authentication header beside a valid
      `apiKeyEnv`, and a missing or non-name-shaped `apiKeyEnv`; each `baseURL`
      grammar breach (userinfo, query, fragment, percent-escape, backslash,
      whitespace, brackets, non-ASCII, empty segment, invalid host label or
      port, `http`, uppercase scheme, schemeless); tabs, control characters,
      document markers and executable or unrecognized syntax at any depth in
      either representation (a `!!js` or other tagged scalar, a `__jsExpr` mapping, a
      flow collection, anchor, alias, merge key or block/quoted scalar). Prove
      the binding and digest check run before any shape check: bytes invalid
      on both digest and shape axes must yield the digest refusal first, while
      a bound, digest-matching member with an invalid `baseURL` yields its
      grammar refusal. In every negative route-overlay planner vector assert
      zero entries to the calibrated stager and zero version/producer calls,
      with a fixed depth/field/URL-part reason and no synthetic private marker
      echoed. An error or absent retained directory alone does not establish
      that nothing was staged. Helper-reader success is not evidence for these
      three launch paths. Keep one resulting patch,
      unchanged reasoning levels and current route rows before Rust-owned rows
      on each positive path, including identity-mismatch cold. Retain the
      existing engine privacy assertions; B's plan is not an emitted launch
      (safety / AS1, safety / AS2, safety / AS3, evidence / LE2).
      Prove the full gate-before-probe matrix, with and without an offer:
      missing/unmeasured/unsupported assessments and missing accounting evidence
      (`unsupported-resume`), incompatible boundary/hands
      (`restrictions-unavailable`), missing/mistyped applicable identity and
      absent/mistyped/malformed declared digest (`unverified-harness`). Use a
      recording version shim and a panicking or counted composite closure;
      assert zero calls to both. A nonexistent executable alone does not prove
      no version attempt. Assert the complete shipped cold argv and overlay,
      not merely `stream_json == false`: no `--new`, `--session` or
      `--output-format`, no rejoining target or offerable-root claim, and a
      refusal token only when an offer was declined (safety / AS1, evidence / LE1).
      For the enabled path, independently exercise matching observations,
      absent/malformed/unreadable version output, version-command failure and
      version drift, producer error and canonical-composite mismatch. A matching
      version invokes the sole producer once; earlier failures invoke it zero
      times. On an offer with matching current observations, independently vary
      originating version and digest through missing, mistyped, malformed and
      different values; these declines may follow one producer call. Require
      `unverified-harness`, the exact shipped cold route under the current home
      and no offerable root on any failed qualification. No-offer mismatch has
      no refusal token. Observed version/digest must not be replaced by requested
      pins. Retain the producer's existing component-drift suites; extending the
      full composite/doctor/adaptation matrix below belongs to D (safety / AS1,
      site / SR5, evidence / LE1).
      Next prove owned storage at the planner boundary with otherwise matching
      identity: missing/mistyped provider ID, locator or home, ID disagreement,
      a missing/unresolvable/different home, and two homes holding the identical
      ID/locator must decline to cold in the current home without reading the
      other store. A symlinked spelling of the same canonical home is a positive
      case. Cover empty, absolute, traversal, non-directory, missing and escaping
      locators; offered and planned locator round-trip at 80/81 Rust characters,
      including R4's eligible 80-character locator whose UTF-8 encoding exceeds
      80 bytes, counting its path prefix. Drive that offer through warm planning
      and assert the exact original output locator, original root, warm argv and
      no replacement allocation; pass the planned value through the existing
      `Transcript::record` clamp and assert it remains unchanged, without a
      provider launch. Retain the 81-character refusals and an overlong value
      whose 80-character prefix names another valid root, which must never be
      selected by truncation. This positive case must fail if the admission
      bound is changed from `chars().count()` to byte length; preserve the
      correct production checks and shared clamp, with no mandatory refactor.
      No lossy conversion or separator rewrite repairs an address. Include
      project-directory, session-directory and `session.v3.jsonl` symlink escapes,
      including an escape to another root within the same home; contained aliases
      remain admissible. Require one matching valid depth-zero header; missing,
      nonregular, unreadable, malformed/truncated, delegated or ambiguous stored
      evidence cannot qualify. Exercise each finite enumeration/header/sequence
      budget at its admitted limit and beyond, read/iteration failure, malformed
      or truncated boundary data and a valid prefix followed by invalid data:
      decline rather than skip an unsafe candidate, use a partial maximum or
      default to zero. Assert no other store changes, history copies or
      substitute-root search; the sole safe cold plan may allocate its own fresh
      root, never reuse the refused one. Put synthetic private markers in
      invalid offered IDs and stored addresses and assert bounded storage
      diagnostics never echo them. These are pre-spawn admission cases;
      current-event folding and accounting remain D (safety / AS2, safety / AS3,
      site / SR3, site / SR5).
      Finish with exact qualified cold/warm argv and overlay assertions: cold
      `--new --output-format stream-json`, warm exactly `--session <owned-id>
      --output-format stream-json`, one `--patch`, admitted profile and current
      Rust-owned model/effort/settings, with warm selecting the original root
      rather than allocating a replacement. Preserve current-directory conformance
      and `confirms_from_locator: false`. A built-driver conformance case in
      `crates/brokkr-cli/tests/driver_conformance.rs` is added only if it proves
      a distinct planner observation; no child-confirmation acceptance is added
      in B (safety / AS1, safety / AS2, safety / AS3, evidence / LE1).
      Pass C, after B, owns the following DSH stream-json confirmation cases
      design D6 and D7 name; do not implement or credit them in B. Each uses a
      captured synthetic child transcript with no installed provider: a post-`await agents.resume`
      init event confirming the offered root, no fresh sibling
      root/session and new sequence activity past the recorded
      `firstSeq`, followed by current work, publishes
      `root_session` and the launch row only after that event, and a
      request-derived `session_id` alone never substitutes for it; the
      child exiting before any init event is failed or indeterminate,
      records no launch row and authorizes no replacement; an init event
      naming a different root followed by a clean exit, and the same
      mismatch followed by an otherwise valid delivered result file, each
      end failed or indeterminate without an accepted successful launch or
      a cold replacement; a nonzero exit carrying stderr prose but no
      measured machine-readable session-rejection shape performs no
      automatic cold replacement (AS4's unstructured-DSH-error case, LE3's
      cannot-classify-a-refusal case); cancellation or a deadline expiry
      while the launch hold is open ends the invocation without a
      fabricated launch or confirmed-session checkpoint and starts no
      replacement; and the one DSH-specific local-decline path, an
      unsupported offer, permits exactly one independently safe cold
      launch. Label these the DSH arm of LE1/LE3/AS4/D7 rather than a
      restatement of 7.9's or 9.7's generic cross-adapter coverage, which
      exercise no DSH child-process init event.
      Pass D owns completion of the remaining composite/containment/doctor/
      adaptation-bytes/retained-storage matrix; keep its passing cases without
      extending that matrix in B. Prove core, Node, dependency, plugin, patch,
      composed-profile or optional extension drift yields `unverified-harness`
      before provider work. Pin the canonical composite's byte form
      with one worked vector per lock dialect (the npm lockfile-3
      hidden lock and pnpm lockfile 9.0, as committed synthetic excerpts in the
      measured grammar). The npm vector includes top-level and nested
      unscoped names, scoped names under unscoped and scoped parents,
      scoped-parent/unscoped-terminal keys, no `name` fields and a conflicting
      optional `name` field, with versions taken from the selected entries.
      Include at least three package groups, for example
      `node_modules/a/node_modules/@parent/b/node_modules/@scope/child`, with
      version and integrity distinct from shallower `@scope/child` entries;
      consume every group and preserve only the terminal package in the triple.
      The measured lock reaches only two groups, so this is synthetic grammar
      coverage, not a claim of deeper measured provider trees. Also refuse
      `node_modules/a/extra/node_modules/@scope/child`, whose valid terminal
      package cannot excuse its malformed intermediate group. The hidden lock
      is the sole npm source: a missing hidden lock is unreadable even if a
      valid root `package-lock.json` is present. Assert exact normalized value
      bytes, equality with equivalent pnpm entries, complete-triple deduplication and
      retention of same-name entries with different versions or integrities.
      Empty, absolute, incomplete, traversal, backslash or trailing-separator
      keys and missing, non-string, empty or whitespace-bearing versions are
      unreadable. Add one vector for the plugin component's bytewise path
      order, the exclusion of the plugin's own tarball entry, and an equal
      composite for the same pair staged in two homes at different absolute
      paths and under different per-seat overlays. In the existing synthetic-home
      suite, also reach the same home through a symlinked ancestor and require
      equal plugin/composite values when the same bundles resolve. Exercise
      general bundle, plugin and synthetic conditional-extension containment
      against the canonical profile, and prove the original lookup anchor/order
      is retained. An unresolvable profile boundary, a symlink target outside
      the allowed canonical roots, a near-prefix sibling such as
      `headless-extra`, and an outside first hit with a later inside candidate
      must each stay unreadable; no fallback or weakened comparison cures the
      false refusal. These cases add no provider support or real extension.
      A profile bundle added,
      dropped or reordered, a changed `patchReload` and an added home-level
      `cordis.patch.yml` each move the composite, and a rewritten `cordis.yml`
      does not. A listed bundle resolving outside the core root and the
      profile (a same-named package in an ancestor `node_modules` or a global
      folder), an executable that is not the core's `env node` script, a
      missing or malformed `bundles` or `patchReload`, and each unrecognized
      pnpm construct (tab, comment, document marker, block-form, missing or
      repeated resolution, key without a version separator) make the identity
      unreadable. Every case builds its homes in temporary directories; no
      test reads `.forge/` or needs an installed provider. Cover bounded locator round-trip and refusal of truncation,
      ambiguity, traversal and symlink escape; a retained directory is never a
      handle. Beside these drift shims, keep 8.8's committed-bytes test pinning
      the repository-owned adaptation's exact six-file set against its
      provenance block. Cover the conditional extension with synthetic absent
      and present sets, exact four-file path order, changed bytes, missing or
      extra files, symlinks and resolution outside the profile; absence emits
      no extension line. If the extension is required, also compare its
      committed set to its own provenance block through the same function.
      No speculative extension is created merely to exercise these cases.
      Retain the exact resume argv and complete current class/model/
      effort cases for every other adapter, generated-fragment versus passthrough
      distinction, no ambient cold/gate continuation, nonpersistent refusal,
      identifier injection, unsupported hands and cold/resume inability to
      honour the class. Label these deterministic planner/storage shims rather
      than live DSH compatibility or enforcement evidence — safety / AS1,
      safety / AS2, safety / AS3, safety / AS4, site / SR2, site / SR3,
      site / SR5, evidence / LE1, evidence / LE2, evidence / LE3, evidence / LE5.
- [x] 8.11 Assignment tests: a confirmed assigned creation reports
      `launch: cold` with root evidence; an assigned ID echoed in a
      start, argv or configuration with unmeasured opening semantics
      produces no session-existence proof and populates no transcript or
      session field; provider evidence naming a different root than the
      assigned one is never recorded as confirmed; and a kill before
      durable confirmation leaves the next retry with no offer and a
      fresh cold identity — site / SR3, site / SR5, evidence / LE1.

## 9. Accounting and legacy compatibility (design D8)

- [x] 9.1 Establish a measured restored-history/current-work boundary in
      each enabled planner before its usage and tool folds run,
      preferring the provider's turn or event cursor. The cursor or
      interval each planner uses is the one 10.1–10.4 establish for it,
      which is why those four precede this group; a boundary is measured,
      never assumed, and a shape whose boundary is not established is not
      enabled. For retained-file
      accounting capture the owned pre-followup offset or sequence
      through the existing bounded path, and never treat a byte count as
      root confirmation — evidence / LE4.
- [x] 9.2 Count only the current invocation's new turns, tools, targets
      and reported usage, filtering replayed tools and targets too.
      Rotation, truncation, a missing cursor or uncertain attribution
      omits the affected measurement, or refuses the resume when no
      current-work boundary can be established at all; a lifetime total
      without an established baseline stays absent rather than guessed,
      subtracted or replaced by zero — evidence / LE4.
- [x] 9.3 Keep the inclusive input and cache-read subsets, cache writes,
      completion deduplication, served model and effort conventions and
      LaneTally's capture identity exactly as they are, and keep a
      rejected resume's unconfirmed usage out of the cold replacement's
      accounting — evidence / LE4.
- [x] 9.4 Never read historical transcript text to reconstruct a prompt
      or a line of reasoning; retain the existing transcript kinds,
      locators, homes, size caps, retention and path checks; leave #222's
      readers alone — site / SR3, evidence / LE4.
- [x] 9.5 Legacy lookup: an unambiguous local single-work-site Codex row
      may offer its thread through the established kind and flat-ID
      mapping, subject to every instance, manifest and origin check and
      the current adapter-version qualification. Legacy composites and
      Claude or DSH locator-only evidence start cold once to establish an
      explicit root; no colon ancestry is guessed and no DSH directory is
      equated with a session. A new row without confirmed root cannot
      fall back to legacy fields to evade confirmation — site / SR3,
      evidence / LE4.
- [ ] 9.6 After 8.8 and 8.10, extend the existing accounting and compatibility
      tests with the selected DSH route. The current operator-ruling slice's
      low-1 clause below verifies only the adopted sequence-zero fix
      and its actual planner-to-drain input (evidence / LE4). It is authorized
      before those whole-task dependencies finish, leaves 9.6 unticked and
      does not begin the remaining Pass D accounting work described here.
      In deterministic retained-store fixtures, a cold
      `--new` launch confirms a fresh root and atomically journals the
      provider ID, safe persistence locator and composite runner identity; a
      warm `--session` launch can use only that same root/locator, and missing,
      truncated, ambiguous or escaping storage starts cold without searching
      for a sibling. Exercise historical sequences followed by a
      multi-message/tool/retry current interval: fold only post-`firstSeq`
      output, tools and targets; deduplicate message usage; distinguish
      per-message from cumulative payloads; omit an unattributable numeric total
      rather than subtracting a guessed baseline or writing zero. Retain the
      generic ten-old/two-new stream case, LaneTally capture-only ledger,
      historical no-launch compatibility, unrewritten legacy Codex resume and
      cold migration of legacy composites and DSH-directory-only history.
      Build every fixture in temporary directories; none reads `.forge/`.
      Verify the focused adapter/runtime accounting suites and label fixtures as
      shim/store evidence, not proof of the upstream pair — evidence / LE4,
      evidence / LE5, site / SR3, site / SR5, safety / AS1.
- [x] 9.7 Close the shared terminal guard and prove launch conformance across
      every built-in adapter in `adapters/tests.rs` and
      `crates/brokkr-cli/tests/driver_conformance.rs`: a different or missing
      required root followed by a clean exit, and the same mismatch followed
      by an otherwise valid delivered result, each ends failed or indeterminate
      without an accepted successful seat, a guessed launch or a cold
      replacement; retain any delivered file only for diagnosis. Also cover
      the no-offer cold launch, the safe work-site resume where supported, the
      declined offer, the proven pre-work cold replacement, a missing
      confirmation, a malformed record and the privacy bound; one panel member
      resuming while another starts cold keeps each member's own launch and the
      aggregate substitutes neither; exec reports no model launch — safety /
      AS4, evidence / LE1, evidence / LE5.

## 10. Provider evidence — investigation first, then dated live proof

Group 10 is the gate on enablement (`safety / AS1`, answers A, F and G),
and its two halves do not block on the same thing.

**10.1–10.4 are investigation**: they read the installed interfaces,
dated captures and supplied exact-version upstream inspection. They run before
group 8's provider-specific construction and group 9's accounting boundaries,
whose argv, routes and cursors are their output. The selected DSH extension is
supported interface evidence; exact-pair compatibility and enforcement remain
10.7's proof work.

**10.5–10.8 are proof**, and each stays unchecked until dated controller
host evidence records CLI identity, exact invocation, same-root
confirmation, current restriction enforcement and current-only accounting
for the named shape. They gate group 11 and nothing else.

For CODEX, END TO END, 10.5 consumes the three supplied September 16 files
and the five-axis mapping below, without an installed-provider availability
check or any fresh measurement. The live proof already identifies 0.154.0.
10.1's historical tick stays historical; 11.1 names its outstanding current
interface obligations. The other provider tasks are outside this commission.
The following investigation/probe convention describes earlier and broader
work, not authorization to repeat this proof.

The seat rechecks its own availability before calling anything unmeasured
and reaches outside the box for no provider source or home, and spends the
captures named in `.forge/controller-provider-evidence-index.md`. Bounded
probes use temporary test data, change no global provider setting, read no
unrelated session and run no unnecessary model experiment. One probe may
cover several axes at once only where each axis has its own attributable
observation; a single denial or a single confirmation is never spread
across unrelated axes, and each proof task names the observation behind
every axis it claims (design D6). An interface that help or source
establishes still produces its construction, validation and shim work
while its proof is pending; interface evidence never enables a shape by
itself (`safety / AS1`).

- [x] 10.1 Preserve the historical Codex interface investigation on
      **0.153.4**. This tick itself supplied no exact-0.154.0 help/source
      evidence for effort configuration, every allowed safe passthrough option
      or stdin prompt positional `-`, and no new probe was commissioned for
      them. Those three deferred obligations are now DISCHARGED by the
      controller's hand-taken
      `.forge/tasks/controller-codex-interface-2026-09-17.json`, measured on
      the installed `codex-cli 0.154.0`, one recorded observation each:
      `/obligations/stdin_prompt_positional/evidence` (resume is
      `codex exec resume [OPTIONS] [SESSION_ID] [PROMPT]`, and the `PROMPT`
      argument itself documents "If `-` is used, read from stdin");
      `/obligations/effort_configuration/evidence` with its
      `/obligations/effort_configuration/control` (under `--strict-config`,
      `-c model_reasoning_effort=low` is accepted and the turn completes, while
      the deliberately misspelled `-c model_reasoning_effrot=low` is refused as
      an "unknown configuration field" — the control is what makes the
      acceptance mean the field is recognised rather than silently ignored);
      and `/obligations/allowed_safe_passthrough/resume_accepts` beside
      `/exec_accepts_but_resume_does_not` (the resume option surface
      enumerated, against the nine options `exec` takes and `resume` does not:
      `--sandbox`, `--cd`, `--add-dir`, `--approve-for-me`, `--color`,
      `--local-provider`, `--oss`, `--profile`, `--version`). The historical
      0.153.4 tick stays historical and is not relabelled. Parsing an option is
      not confinement: the record's own `caution` says so, and its
      `not_previously_supported` enumeration — `--worktree`, `--thread-source`,
      `--ephemeral`, `--ignore-rules`, `--ignore-user-config`,
      `--output-schema` — grants none of them anything. `--worktree` and
      `--thread-source` stay refused. The matching adapter assertions over this
      measured surface remain 11.1's work, and the two enumerated options the
      adapter's allow-list already admits are named as a residual in the
      2026-09-18 return. The completed historical investigation read
      `codex exec resume --help` and the
      installed help or source for `--json`, `-c sandbox_mode`, effort
      configuration, safe passthrough and the thread positional,
      recording the observed version and the allowed argv shape. 0030's
      0.148.0 measurement is historical regression scope, not current
      qualification. Prerequisite of 8.5 — safety / AS1.
- [x] 10.2 Claude interface on **2.1.266**: the resume selector and its
      arity, the restriction flags and their precedence — permission
      mode, model and effort, `--tools ""`, strict MCP config, the boxed
      workspace fragment — the persistence setting, the excluded
      selectors of 8.6, and the stream shape a resumed print invocation
      emits, from installed help and
      `.forge/controller-host-provider-interface.json`. Prerequisite of
      8.6 and 8.7 — safety / AS1, safety / AS2.
- [x] 10.3 DSH source and interface: retain the completed installed
      0.1.2-rc.1 trace as a bounded one-shot-entry result — its headless runner
      accepts only `task` and creates a random root, so the launcher's TUI
      `--resume` example cannot simply be forwarded — but never restate it as a
      global DSH limitation. Adopt
      `.forge/tasks/controller-dsh-upstream-discovery.json` and the inspected
      read-only checkouts: the official API exposes
      `ctx.agents.resume({ resumeSessionId, agentOptions, setup })` and continued
      persisted history, while `dsh-plugin-cli-session` 0.2.0 at
      `0f487e74c81ed102c6899440d9f5d65e8e9eabda` is the selected documented
      extension route with `--new`, explicit `--session`, current model
      selection and a pre-followup `firstSeq` interval. A live isolated probe
      (`.forge/tasks/dsh-pair-incompatibility.json`) shows core 0.1.5-rc.1 is
      incompatible with this plugin's unmodified bytes because it removed the
      `agent.session.events` accessor. The operator's 2026-09-10 ruling
      reverses the interim re-pin to the plugin's development core 0.1.0-rc.6
      (`.forge/tasks/dsh-pair-qualification-010rc6.json`, now dated history)
      and selects the latest core forward with a repository-owned adaptation
      instead. Installed `@deepseek-ai/dsh-session` 0.1.5-rc.1 declares
      `snapshotEvents(fromSeq?, toSeqExclusive?)` (`lib/types/index.d.ts:187`,
      SHA-256
      `ed327445b83ca8d699eb22991178362e4458172ba3c717a7a894f4907394fc37`) as a
      frozen `log.slice(fromSeq, toSeqExclusive)` (`lib/index.js:1107`,
      SHA-256
      `05e94f57d96e7979670a5b51024c8591572eb0051ce793613dbdec35cf2c47bf`); the
      same module defines `seq` as `log.length` and stamps every appended
      event with `seq: log.length`, so log index equals sequence and
      `snapshotEvents(firstSeq)` is exactly the log filtered to
      `seq >= firstSeq` — the plugin's lawful replacement for the removed
      accessor. `ownEvents()` is rejected because its boundary is fork
      lineage, and `eventAt()` because the folds need an interval. Record that
      its `session_id` is request-derived and its latest in-range usage is
      last-wins; these remain 10.7 independent-root confirmation and
      accounting questions. Reject the official SDK for this change because it
      would add a second runner where the supported CLI extension candidate
      exists. This checked task proves and selects the supported caller route
      and its replacement accessor only; it does not prove the resolved pair,
      restrictions, exact root or accounting and enables nothing. It is 8.8's
      prerequisite — safety / AS1.
- [x] 10.4 LaneTally interface: wrapper identity, what it forwards to
      Claude, how the underlying Claude version is read through a
      measured interface, and the capture marker's attribution point. No
      LaneTally version command is invented. Prerequisite of 8.7 — safety / AS1.
- [x] 10.5 Consume the completed Codex controller proof, first and in full:
      `.forge/tasks/controller-codex-proof-2026-09-16-live.json`,
      `.forge/tasks/controller-codex-proof-2026-09-16-raw-report.json` and
      `.forge/tools/codex-enforcement-probe.py` (read only; never execute it).
      Record one live-file citation per axis using the current tasks return
      below: exact demonstrated invocation/allowed argv; effective class and
      applicable fragment re-imposed across resume; exact same-root
      confirmation; current-only accounting; pre-work rejection shape.
      The exercised identity is 0.154.0, accepted 0030 measured 0.148.0,
      and the pre-reconciliation declaration applies to 0.153.4. Consume the
      attributable completed command output, including the allowed control,
      denied cold write, allowed bare-resume write and denied restored write;
      `EXIT=2` belongs to the denied write even when its reporting shell exits
      zero. The record, not a file's absence alone, establishes enforcement.
      Keep September 10/12 records as dated history only; the refuted
      September 16 partial supplies no qualification and is not cited.
      No provider version read, remeasurement, network lookup or new probe is
      authorized. The observed fragment is the no-boxed-hands sandbox
      override, not boxed MCP or all safe passthrough options. Tick after
      11.1's matching argv, root, refusal and current-usage assertions and
      their compiling removal controls pass and the return records that
      agreement. This assertion subset does not depend on ticking 11.1;
      its remaining 10.1 interface gap must not create a circular dependency.
      Boxed PATH absence does not invalidate the supplied proof. Verification:
      the five separately cited observations plus the named passing adapter
      tests and restored removal controls — safety / AS1, AS2, AS3, AS4;
      evidence / LE1, LE3, LE4, LE5; site / SR3.
- [x] 10.6 Finish Claude proof on the pinned 2.1.266 binary without
      repeating the supplied same-root and Read-grant observations. Preserve
      `controller-claude-root-probe.json`,
      `controller-claude-grant-probe.json`,
      `controller-claude-tool-removal-probe.json`,
      `controller-claude-mcp-probe.json` and
      `controller-claude-accounting-analysis.json` as partial evidence. Run one
      bounded disposable-fixture follow-up with successful cold native Write and
      MCP-call controls, then the same-root resume under the complete replacement
      boxed plan: explicitly attempt the expired Read grant, removed Write and
      removed MCP tool, prove their denial or lack of filesystem/server effect,
      and prove the new Read grant actually operates. Capture init tool/server
      admission, raw stream message/tool IDs, terminal totals and retained
      completed-message IDs so the exceptional empty-response/retry and
      visible-message/turn differences are attributed without a guessed baseline
      or zero. Also establish the complete filesystem boundary, flag precedence
      and persistent identity.
      **Done by hand** in `.forge/tasks/controller-claude-proof-2026-09-18.json`,
      completed 2026-09-19. Ten axes, each attributable to a call that
      demonstrably ran. The axis this record previously listed as unexercised —
      the exceptional empty-response/retry — was **induced deliberately** rather
      than waited for: an unknown model name forces the empty response, an
      unroutable endpoint forces the retry. The empty response retains an
      assistant message id for a turn that never reached the API, marks itself
      fabricated with `model: "<synthetic>"`, and reports every total as zero
      with `api_error_status` 404 — so attribution rests on the id and the
      status, exactly as this task demanded it must, never on a guessed baseline
      or a zero. The retry path carries its own typed `system/api_retry` event
      with `attempt`, `max_retries`, `retry_delay_ms` and a per-attempt uuid.
      The adapter normalization was READ against both, not assumed: `api_retry`
      falls through the `system`/`init`-only match so retries count no turn and
      no usage, and `<synthetic>` is rejected by `model_token`'s character
      grammar so a seat's real model pin survives. The binary pinned here,
      2.1.266, is no longer installed on the host at all; this exercises the
      installed 2.1.273, on the precedent 10.5 set for Codex. If only the controller can execute it, prepare
      and hand off an executable task-owned probe with these exact argv,
      fixture effects and assertions; do not cite worker-home EROFS as a
      controller blocker. Tick only when every missing axis has its own
      attributable observation and the Brokkr adapter normalization agrees —
      safety / AS1, safety / AS2, evidence / LE4.
- [x] 10.7 Qualify the forward-pinned DSH pair from 10.3 in an isolated
      worktree or task-owned profile, in two halves. **The live half runs
      first**, before 1.1, 6.4, 11.5 and 13.1: install the latest official
      core (`@deepseek-ai/dsh` 0.1.5-rc.1 at
      `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
      resolves in its place, re-verifying any reused task-owned install's
      hash), commit the repository-owned six-file adaptation of
      `dsh-plugin-cli-session` 0.2.0 at
      `0f487e74c81ed102c6899440d9f5d65e8e9eabda` under
      `extensions/dsh/plugin-cli-session/` with its provenance note and the
      `docs/guides/repository-layout.md` row, install the pair into the
      task-owned home's `headless` profile, the one the adapter launches,
      check the six installed files against the provenance block with
      `sha256sum -c` before any model call, and record the composite's raw
      inputs as design D6 lists them — the core package's name, version and registry integrity,
      the executable's real path relative to the core package with its first
      line, the Node runtime version, each lock file by home-relative path and
      SHA-256, the six installed files' SHA-256, the plugin's and profile's
      Cordis patch SHA-256, the profile manifest's `bundles` and
      `patchReload`, each listed bundle's resolved directory relative to the
      core root or the profile, and the home-level patch's SHA-256 or its
      absence — with the composed profile and extension protocol, but no
      plugin-component or composite digest. If D6's measured missing-hook
      condition applies, record `extensions/dsh/resume-policy/`'s separate
      provenance, installed four-file digests and resolved profile-relative
      directory as raw inputs too; verify them against the note before a
      model call. Obtain its component only in the recording step through
      8.8's function. Prove the live global DSH pin, profiles, credentials and other
      runs are unchanged. Keep the task-owned home in place.
      Write the result to `.forge/tasks/dsh-pair-qualification-015rc1.json`
      beside the superseded `.forge/tasks/dsh-pair-qualification-010rc6.json`,
      by that record's method: run a bounded `--new` cold invocation that
      persists and independently confirms its root plus a private nonce, then
      `--session <owned-root> --output-format stream-json` against the
      originating persistence root, both now exiting 0 with a `stream-json`
      result envelope once the adapted accessor folds. Confirmation requires
      the prior depth-zero header, the pinned plugin's post-`agents.resume`
      init event, same-root nonce continuity, no fresh sibling root/session
      and new sequence activity; the request-derived `session_id` echo alone
      is insufficient. Dump the composed profile and prove current headless
      runner, model, effort, working directory, persistence root and every
      applicable sandbox/tool/settings restriction win after restoration; use
      positive current controls and inverted removed-authority
      filesystem/server effects, and show an alternate selector/profile
      cannot override adapter-owned values. Exercise multi-message, tool and
      retry activity across `firstSeq`, compare raw sequence/message
      identities and determine whether usage is per-message or cumulative;
      historical output/tools/tokens must be absent and every numeric total
      attributable or omitted. **The recording step runs after 8.8's function
      and doctor line land**, even while 8.8's planner half is pending:
      re-measure the raw inputs and require them unchanged, run
      `brokkr doctor` with `BROKKR_DSH_BIN` and `DSH_HOME` pointed at the
      retained task-owned home, and append the reported plugin component and
      canonical composite to the same record as a dated entry, without
      editing the live record. 8.8's function is the only producer of those
      values; nothing hand-computes one. **The task itself stays unchecked**
      until that entry exists and Brokkr's matching adapter assertions
      agree. If a required policy or
      pre-work fact is unavailable even with the adapted accessor, identify
      the exact documented hook needed and return 8.8 to pending for D6's
      narrow digest-bound Cordis extension at
      `extensions/dsh/resume-policy/` with its separate provenance section,
      then rerun this proof;
      incompatibility of this exact route is the precise unmet AS1 condition,
      not a global limitation and not authority for SDK/TUI substitution or a
      second adaptation delta. If the resolved release genuinely exposes no
      lawful replacement for the removed accessor, do not downgrade and do
      not fabricate one: record the exact missing surface with the
      reproducing probe, name the residual and park with the precise
      upstream ask (module, symbol, version) instead of ticking. Verify both
      the upstream observation artifact and Brokkr's matching adapter/shim
      assertions before ticking — safety / AS1, safety / AS2, safety / AS3,
      site / SR3, evidence / LE4.
      **Recording step done by hand, 2026-09-21**, after run
      `dsh-composite-identity-issue-226-e291e076` completed: the seventeen raw
      inputs were re-measured on the retained home and are unchanged, and
      `brokkr doctor` from this branch's release build, with `BROKKR_DSH_BIN`
      and `DSH_HOME` on that home, reports plugin component
      `074d1b111148cd3f1770a5afc23e1589fbef61cc940c49385e97da8117e2eda5` and
      canonical composite
      `a64fcd6d048603ecb1767b229fa0fb6a30d9ae7cda92a47cdc82360d9ee3ddd1`. Both
      are appended as the dated entry `RECORDING_STEP_2026-09-21` in
      `.forge/tasks/dsh-pair-qualification-015rc2.json`, and both equal the
      values `composite/tests.rs` pins from the retained ground truth, so the
      adapter's assertions and the live measurement agree. The declared
      `wrapper_digest` remains 11.3's to write.
- [x] 10.8 LaneTally proof: wrapper forwarding, the underlying Claude
      version, root confirmation, capture attribution and the applicable
      restrictions on resume. Unsupported hands stay unsupported — safety / AS1.
      Recorded by hand in `.forge/tasks/controller-lanetally-proof-2026-09-19.json`
      against the shipping wrapper on the installed `2.1.273`, each axis with its
      own attributable observation and each refusal proved by removal: argv
      forwarding is verbatim and positional, with exactly `--settings <fragment>`
      inserted for a child named `claude` and nothing inserted for a child that is
      not; the underlying version answers through the wrapper unchanged; a cold
      run's persistence root is derived from the child's cwd and a `--resume`
      returns to that same root, proved by the restored private nonce rather than
      by the echoed session id; capture attribution is minted **per invocation**,
      so two resumes of one session carry two different `X-Run-Id` values and the
      transcript carries none of them; and the restrictions on resume are the
      wrapper's ownership of the per-session settings layer plus its typed
      refusals. Hands stay `unsupported` — forwarding is not confinement — and
      `adapters/lanetally.json`'s reason is corrected to say what was measured and
      what still is not, rather than claiming the pass-through is unmeasured.

## 11. Enablement — evidence-gated provider dispositions

- [ ] 11.1 Complete installed-version qualification of Codex's previously
      supported work shapes, using the ordered current tasks return below.
      First reconcile `adapters/codex.json`: preserve `supported`, class
      `work`, boundaries `harness` and `not applicable`, and `hands: none`;
      set both `identity.version` and `identity.applies_to` to 0.154.0. Retain
      0030's historical 0.148.0 measurement in evidence and append the dated
      0.153.4→0.154.0 reconciliation referencing the supplied live proof,
      preserving all five existing limitations byte-for-byte and in order.
      Explain in the reason that resume has no sandbox flag, so the class
      must travel through `-c sandbox_mode=<class>`. Reconcile only dependent
      shipped-assessment tests/shims, proposed 0056's current prose (status
      stays `proposed`) and the current provider guide; preserve independent
      synthetic versions and dated history. Verify the existing loader and
      shipped identity tests; measure every affected digest before re-pinning
      — safety / AS1; progress / PM4.
      Then extend existing protocol, runtime and real-driver conformance
      suites: all three sandbox input spellings produce exactly one quoted
      sandbox override paired with `-c`, and no sandbox flag on resume; the
      offered root is the provider-received and exactly confirmed root; no
      root or a different root never publishes a successful resume, even
      with clean exit and a delivered result. Independently assert identity
      (including originating-version) `unverified-harness`, boundary/hands
      and absent-marker `restrictions-unavailable`, and absent assessment/
      accounting `unsupported-resume`. Pin current invocation usage and
      preserve all existing selector, unknown-option and pre-work refusal
      behavior. Every claimed assertion requires its own observed compiling
      removal failure at the intended assertion, exact restoration and
      passing rerun. Stage executable shims beside their target, close them,
      then rename into place. Verify both shipping coordinates through the
      existing production composition/exchange seams; synthetic assessment
      tests alone cannot discharge preservation — safety / AS1, AS2, AS3,
      AS4; site / SR2, SR3, SR4; evidence / LE1, LE3, LE4, LE5.
      Complete local gates and persist evidence before claiming the assertion
      subset delivered. 10.5 is ticked on its recorded axes and the agreeing
      assertions; that tick is history, not an instruction. This entire 11.1
      needs 10.1's exact current-version interface acceptance as well, and
      that acceptance is a necessary condition, never a sufficient one. It is
      no longer the gap: `.forge/tasks/controller-codex-interface-2026-09-17.json`
      measures effort configuration, the resume option surface and the stdin
      prompt positional `-` on the installed 0.154.0, as 10.1 now records
      obligation by obligation. What 11.1 still owes is its own half — the
      adapter assertions over that measured surface and their compiling removal
      controls, ordered as clauses E1–E11 in the 2026-09-18 breakdown below —
      together with the declaration, 0056 and guide prose that still recite the
      discharged debt, and the witness/compose digests that text moves. Those
      clauses are this slice's substance, and they do not by themselves earn
      the tick: D10's 2026-09-18 sitting holds a blocking qualification
      residual open, because `CODEX_RESUME_VALUE_FLAGS` admits
      `--output-schema` and `CODEX_RESUME_BARE_FLAGS` admits `--ephemeral`
      while the September 17 record lists both under
      `not_previously_supported`, and no supplied observation establishes
      ephemeral persistence (`codex_launch` sets `persistent: true` on both
      arms) or output-schema safety on a resume. Widening or withdrawing
      either entry is a semantic move and the operator's word, not a seat's.
      So leave 11.1 unchecked, change the allow-list in neither direction,
      and name both missing qualifications in the return; preserving support
      does not complete it, the interface record alone does not complete it,
      and neither does a passing surface-membership assertion. Verification:
      the concrete evidence checklist and commands in the current tasks
      return, with any remaining obligation explicitly pending and no new
      provider probe — safety / AS1;
      evidence / LE5; progress / PM1, PM4.
      Earlier operator-ruling implementation history follows. Its previous
      version/disposition observations remain dated history, not the new pin:
      The 2026-09-15 operator ruling, **“keep decision 0030's rejoin live,
      do not regress codex,”** closes the cold interval recorded by the
      judging pass at `2da94b0`. The change must retain the rejoin main already
      performs across the historical 0.148.0/current-applicability 0.153.4 drift,
      while refusing actual identity, boundary, hands and accounting mismatch.
      At this tasks return's adopted head `78098d5`, the declaration was still
      unmeasured. Analyze F1 establishes that retaining `hands: boxed` would
      also refuse the shipping harness retry after a status flip. This
      implementation visit landed D10's amendment: `adapters/codex.json`
      `work-site` is now `supported`, scoped to
      `boundaries: ["harness", "not applicable"]` with
      `hands: "none"`, keeping the historical 0.148.0 measurement and 0.153.4
      applicability and adding the dated 0.153.4 current-accounting reference;
      proposed 0056 ruling 5 and the provider guide state the
      preservation/new-shape distinction. The judging pass at `b4dbfd5`
      returned F1: the engine-composed harness coordinate was preserved, but
      the INLINE Codex work seats main also rejoins (`recipes/standby`,
      `recipes/wager-harness`) still regressed, because
      `Bundle::compile_with` carried no assessment for a raw `driver.command`,
      so the engine supplied `assessment: null`, the gate declined an offered
      retry as `unsupported-resume`, and a cold launch recorded no qualified
      root. This return attaches each inline built-in model driver's adapter
      resume assessment to the compiled bundle and reads it in `site_plans`,
      names `not applicable` in the declared boundaries, and proves both the
      composition (`inline_resume` read off the shipped recipes) and the
      inline coordinate behaviorally through a real `brokkr driver codex`
      cold-then-resume exchange in `driver_conformance.rs`. Its controls:
      reverting only the shipped status to `unmeasured` made the inline cold
      launch record no qualified root and failed the retry; dropping only
      `not applicable` from the declared boundaries failed it cold; removing
      only the inline population or the engine fallback failed the composition
      and engine cases. The runtime boundary suite also gained the
      composition bridge (shipped `Adapters::load`, the production resolver and
      `compose_site` under harness, `seat_input`/`mark_hands`), and
      `crates/brokkr-cli/tests/driver_conformance.rs` drives a cold
      `brokkr driver codex` with the shipped assessment and the
      production-composed `--sandbox workspace-write` argv, then offers that
      root to a fresh driver and observes the production adapter's confirmed
      `launch: resumed` with the exact thread, `sandbox_mode="workspace-write"`
      and `model_reasoning_effort="xhigh"` re-expressed. Observed controls:
      reverting only the shipped status to `unmeasured` failed the retry with
      `resume_refusal: unsupported-resume`; reverting only declared hands to
      `boxed` failed it cold with `restrictions-unavailable`; suppressing the
      harness fragment append in `compose_site` and the boundary write in
      `mark_hands` each failed the bridge; every mutation was restored and the
      tests rerun green. The regression is removed on both the harness and inline
      coordinates and full 10.5/11.1
      qualification is still owed; this checkbox stays unchecked. The earlier
      cold/unsupported observation remains dated evidence, never an authorized
      current disposition.
- [ ] 11.2 Enable Claude's boxed-workspace work shape under its
      already-supported boxed boundary with 10.2's interface and 10.6's
      proof, and flip `adapters/claude.json` — safety / AS1, safety / AS2.
- [ ] 11.3 After 8.8, 8.10, 9.6 and 10.7 pass, enable DSH's
      already-admitted headless work shape only for the exact isolated
      resolved-core/repository-owned-adaptation composite identity — the
      latest official core (currently `@deepseek-ai/dsh` 0.1.5-rc.1 at
      `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
      resolves in its place) with the six-file adaptation of plugin 0.2.0
      under `extensions/dsh/plugin-cli-session/`. This is the only task that
      writes the measured `wrapper_digest`, copying the composite 10.7's
      recording step appended, which 8.8's loader amendment already admits;
      flip `adapters/dsh.json` and its
      packaged/scaffolded equivalents to `supported` with 10.3's interface
      and 10.7's compatibility, exact-root, restriction and
      current-accounting references, together with `wrapper_digest`; record
      the provider ID, persistence-locator and composite-digest limits in
      proposed 0056 and `docs/guides/provider-adapters.md`, including the
      digest a deployed home must reproduce and the doctor line that reports
      it. Before that edit, record design D6's end-to-end proof: drive the
      built `brokkr driver dsh` over driver-protocol v1 with `BROKKR_DSH_BIN`
      and `DSH_HOME` pointed at the retained task-owned home and the candidate
      `supported` assessment, carrying the appended composite, in the private
      start context. The exchange covers a cold `--new` that confirms a root, a
      `resume` offer for it that rejoins through `--session` with
      current-only evidence, and the same offer declined to the shipped cold
      invocation after a reversible composite drift (a home-level
      `cordis.patch.yml` added, then removed with the raw inputs
      re-measured) and with an unsafe locator. Append its input lines,
      emitted messages, the binary's SHA-256 and the Git head to the 015rc1
      record as a dated entry. The driver exchange starts no Brokkr run. The
      committed suite's stand-ins stay 8.10's and 9.6's hermetic shims; add
      no cargo test that reads `.forge/` and no ignored or environment-gated
      live test. Tick only when the recorded exchange and those hermetic cases
      both pass and doctor over the same home reports the written digest as
      equal. Keep 0.1.2-rc.1 one-shot, the superseded 0.1.0-rc.6 pin,
      SDK/TUI, global profiles, new trust/boundary and DSH hands disabled and
      named only as dated history. If 10.7 fails an admission condition,
      leave this task unchecked and report that exact AS1 gap upstream
      rather than flipping the declaration — safety / AS1, safety / AS2,
      safety / AS3, site / SR3, evidence / LE4.
      Three adaptation facts the 2026-09-15 judging pass read in the inert
      plugin, to be measured or closed before the flip rather than
      discovered by it: `lib/startup.js` lets `--workdir` set the sandbox
      root (a new session only, refused beside `--session`/`--resume`);
      `lib/index.js` resolves bare `--resume` to the NEWEST session of the
      cwd, which the driver must never send — it passes `--session <id>`
      only; and `package.json` pins `commander` at `^15.0.0` with no
      lockfile, so the composite digest 10.7 records must name the
      resolved dependency set, not the range.
- [ ] 11.4 Enable LaneTally only on 10.4's interface and 10.8's proof,
      or leave it declared-unsupported on 10.4's measured evidence with that
      reason. This is the operator's one lawful unsupported disposition;
      it does not waive Codex, Claude or DSH delivery. Never mark it supported
      by analogy — safety / AS1.
- [x] 11.5 Before 8.8, reconcile every shipped, packaged or scaffolded
      support representation with 6.4's corrected preparatory truth. DSH's
      forward-pinned latest core (`@deepseek-ai/dsh` 0.1.5-rc.1, or the
      release answer N1 resolves in its place) plus the repository-owned
      six-file adaptation of plugin 0.2.0 route is `unmeasured`, disabled and
      waiting on 10.7; its measured resolved-composite identity is visible
      with no `wrapper_digest` set, while the installed 0.1.2-rc.1 one-shot
      result and the superseded `dsh-pair-qualification-010rc6.json` record
      are only bounded, dated history — never rewritten and never restated
      as the selected route. Preserve every other shape's current status and
      the separate DSH hands deferral. Verify the declaration
      loader/scaffold expectations and a repository content audit agree and
      find no current declaration or packaged equivalent that still names
      0.1.0-rc.6 as the selected route or calls DSH globally unsupported;
      deterministic shims remain labelled as shims, not live proof —
      safety / AS1, evidence / LE5.

## 12. The SDD charter (design D9)

- [x] 12.1 Change `agents/charters/implementer-sdd.md` once: before a
      group, record it as in progress and name its focused acceptance
      checks; when a task's implementation and its required focused
      checks pass, persist its tick and concise evidence before the next
      group; partial implementation and failed or pending checks stay
      unchecked with a next action; group completion, workspace
      validation, commit and external delivery stay four separate facts.
      Name the dialect's task artifact generically — no framework path,
      no repository command (decision 0042 ruling 6) — progress / PM1,
      progress / PM3.
- [x] 12.2 Add the recovery clause to the same charter: on retry or
      re-entry, resumed or cold, read the change's current specification,
      design and task artifacts and the current worktree before
      continuing; reconcile ticks against surviving edits and cited
      verification; preserve work that still satisfies its task; return
      an invalidated task to pending with its reason; never erase partial
      uncommitted edits merely because the successor does not remember
      authoring them; never read another session's private transcript.
      Current evidence outranks memory — progress / PM2.
- [x] 12.3 Leave `agents/charters/implementer.md` unchanged, and leave
      `dialects/openspec.json`, `dialects/speckit.json`, their
      instruction directories and the copies under
      `crates/brokkr-cli/dialects/` with their existing phase maps and
      task formats: neither dialect gains an implement phase or a
      duplicate timing rule — progress / PM3.
- [x] 12.4 Re-record the charter digest in
      `crates/brokkr-runtime/tests/library_data.rs` from the test's own
      reported pair, and update the rendered-prompt expectations in
      `crates/brokkr-runtime/tests/sdd_shape.rs` and
      `crates/brokkr-runtime/tests/roster.rs` so both dialects' implement
      and return prompts carry progress-before-next-group, truthful
      verification and worktree reconciliation, with OpenSpec's numbered
      `tasks.md` checkboxes and spec-kit's phased rows each supplied by
      the dialect — progress / PM3.
- [x] 12.5 A deterministic recovery exercise in a temporary worktree: a
      completed group with recorded progress, a second group marked in
      progress, an interruption before any commit, and a successor
      reading group one's checked work and group two's incomplete status
      from the artifact rather than finding everything unchecked. The
      evidence identifies it as an exercise and an instruction check, not
      a claim that a live model always obeys — progress / PM1,
      progress / PM2.
- [x] 12.6 A test that a judge's rendered prompts grant no task edit: the
      review offices report findings to the owning artifact and leave the
      task file and the tree unchanged — progress / PM3.

## 13. Prose

- [x] 13.1 Before continuing 8.8, correct `docs/guides/provider-adapters.md` while
      preserving its offer, assessment and held-window guidance. The current
      adapter table must call the forward-pinned latest DSH core
      (`@deepseek-ai/dsh` 0.1.5-rc.1 at
      `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`, or the release answer N1
      resolves in its place) plus the repository-owned six-file adaptation of
      `dsh-plugin-cli-session` 0.2.0 under `extensions/dsh/plugin-cli-session/`
      route `unmeasured` and disabled pending 10.7's exact-root,
      independent-root, restriction and current-accounting proof;
      distinguish its documented `--session` extension interface from the
      version-bounded installed 0.1.2-rc.1 one-shot result, name the
      reversed 0.1.0-rc.6 pin and the superseded
      `dsh-pair-qualification-010rc6.json` record as dated history, and keep
      the hands plugin deferral independent. Also retain the supplied Claude
      observations as partial and the other declarations' current truth.
      Reopened on 2026-09-13 at `b049224`: preserve the completed DSH row,
      but replace the false claims that Codex has no accounting evidence on
      any version and that no resumed Claude behavior has been measured.
      Cite the September 10 Codex current-only accounting/root sample and the
      controller's partial Claude root/Read observations, with their version
      and enforcement/accounting limits; retain September 12 Codex startup
      enforcement as startup evidence only. Neither partial capture enables
      a shape or completes 10.5/10.6. This is the sole tick reversal on this visit.
      The current operator-ruling slice additionally replaces the guide's
      blanket measurement-expiry paragraph, Codex row and all-cold statement
      with W/D10's distinction: preserve main's harness/none workspace-write
      rejoin, keep never-supported shapes disabled and retain partial accounting
      evidence and full 10.5/11.1 debt. Explain that the full boxed MCP argv
      remains cold, now with `restrictions-unavailable` instead of main's
      `incompatible-argv`; the ruling enables no boxed rejoin. Scope the
      `ResumeIdentity::Measured.applies_to` comment in `agents.rs` likewise;
      do not change its type or validation. The earlier tick-reversal account
      above is history; this slice changes no tick. Verify the guide,
      proposed 0056, task 11.1 and all four declarations agree on the ruling.
      Verify the guide and declarations agree line by line on status,
      identities, evidence class and limitations, and that no current prose
      repeats the disproven global DSH limitation or the superseded
      0.1.0-rc.6 selection — safety / AS1, evidence / LE3, evidence / LE5.
- [x] 13.2 `docs/guides/driver-authoring.md`: how a third-party driver
      advertises receipt, what a correlated one-use offer looks like on
      the wire, and that not advertising it means never being handed a
      handle — site / SR4.
- [x] 13.3 `docs/guides/journal-and-verification.md`: seat-record v5, its
      new optional fields, its five added refusal tokens, the 0.10.0
      dispatch boundary across append, export, import verification and
      offline verification, and the guarantee that valid historical
      v1–v4 rows, including unstamped 0.10.0 and tagged 0.9.0/0.9.1
      no-boundary rows, stay valid and unrewritten. Check this account
      against D4 and the modified boundary-record requirement — evidence / LE2,
      evidence / LE5, boundary /
      The seat record carries the boundary as seat-record/v4.
- [x] 13.4 State the two honest limits in the guides as well as in 0056:
      the local origin check does not authenticate a provider account,
      and a charter instruction is not an engine guarantee — site / SR5,
      progress / PM3.

## 14. Re-pins

- [ ] 14.1 After every remaining code, declaration and charter change in
      groups 8–13, re-run the witness tests and re-record any moved digests in
      `crates/brokkr-runtime/tests/witness_digests.rs` from the tests'
      own reported left/right pairs — the adapter declarations of group 6
      and the charter of group 12 move every identity that consults
      them, and a declaration enabled in group 11 can move them again — and
      change no pin the tests did not report. W's bounded re-pin clause runs
      after all four declaration reasons settle, including prose-only edits:
      run the existing witness and compose tests, update only their observed
      digest pairs, and retain equality assertions. This does not complete
      14.1 while the whole change remains open. The successful pre-return run is
      historical evidence, not proof for the eventual final bytes — safety /
      AS1, progress / PM3.
- [ ] 14.2 After 14.1 and all other source changes, compile `bundles/self` and
      `bundles/verify` and reconcile any manifest digest the compiles report.
      The successful pre-return compiles do not cover later planner or
      declaration edits — safety / AS1.

## 15. Gates, pre-archive readiness, archive effect, commit and controller evidence

The commands below are this commission's, recorded here and not promoted
into capability truth (`progress / PM4`). Run them with
`CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

For CODEX, END TO END, run the local commands as slice validation in the
current Codex breakdown below, without claiming whole-change readiness or
ticking 15.x.
Use both unfiltered workspace test commands and the unchanged exact script
and test selection. Earlier pre-archive history grants no new filter, gate
edit, archive or spec fold to this slice; record each command's actual status
and scope. The whole-change closure and archive instructions below remain
outside the present commission.

- [ ] 15.1 After groups 8–14 are complete, run `cargo fmt --all -- --check` —
      every requirement of this change.
- [ ] 15.2 `cargo clippy --workspace --all-targets --all-features --locked
      -- -D warnings` — every requirement of this change.
- [ ] 15.3 Complete the pre-archive workspace run under design D9's F1
      ordering: `cargo test --workspace --all-features --locked -- --skip
      every_capability_names_the_archived_changes_that_wrote_it`.
      This single assertion requires this change's archived directory, which
      does not exist while the dated change is active again. First retain the
      test listing from `cargo test --workspace --all-features --locked --
      --list` and verify the skip substring matches exactly that one test in
      the provenance binary. Retain the successful run's output and require
      the sum of all `N filtered out` test-result counts to equal exactly one;
      no other filter or exclusion is permitted. Any extra match, extra
      filtered test or failure leaves this task pending. The other provenance
      direction runs, but its current pass is vacuous for this change's absent
      archive and does not prove its round trip. Tick only for the completed
      pre-archive scope and record the named assertion and full archived suite
      as pending archive validation. Dropping the skip before the fold should
      fail loudly; never weaken the assertion, annotate it ignored, accept an
      active directory as an archive, or widen the skip for another failure.
      The post-task action below must run the complete unfiltered suite on
      archived bytes before the activation/delivery commit — every requirement
      of this change, progress / PM1, progress / PM4.
- [ ] 15.4 `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
      and `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify`
      both pass, then run `cargo build --release --locked -p brokkr-cli` and
      require its own successful result on the same final active bytes. Tests,
      debug builds and bundle compiles do not substitute for the commissioned
      release-profile build — every requirement of this change, progress / PM4.
- [x] 15.5 Preserve the **completed original fold and dated same-change
      identity**. The five living capabilities already carry exactly one
      `2026-09-09-226-session-resumption` provenance line from commit
      `75ae68e`, and `boundary-record` retains its earlier
      `2026-09-06-boundary-named-slice-i` entry. Restore the returned active
      directory with `git mv openspec/changes/226-session-resumption
      openspec/changes/2026-09-09-226-session-resumption`; inspect installed
      OpenSpec 1.12.0's `ARCHIVE_DATE_PREFIX_PATTERN` branch and the supplied
      dated scratch probe. These establish that the normal archive operation
      preserves the already dated identity and existing provenance
      byte-for-byte. This completed task claims the real identity repair and
      evidence review only: scratch ticks/archive do not complete any real
      provider, Rust, host, archive or delivery task. Do not create a competing
      change, rename an archive after folding, spoof/drop the date, rewrite
      provenance or invoke `--skip-specs` — progress / PM4, boundary /
      The seat record carries the boundary as seat-record/v4.
- [ ] 15.6 After every task in groups 8–14 and gates 15.1–15.5 are
      complete, validate the **active** change
      strictly. Reconcile every earlier task tick and the final `## Progress`
      account against the worktree and its evidence, including 15.3's completed
      pre-archive scope and the still-pending single assertion/full archived
      suite; the deferred assertion is not a passing result. Stage only the
      intended delivery paths, and inspect the staged diff. Tick 15.6
      while the change is still active; at that point 15.7 must be the only
      unchecked tracked task and no ordinary artifact edit remains — progress /
      PM1, progress / PM4.
- [ ] 15.7 Establish **pre-archive readiness** while the change is active, as
      proposal answer I and design D9 require. After 15.6 passes, confirm every
      other tracked obligation and its evidence is complete, the active strict
      validation still passes, F12's dated same-change postconditions are
      present and verified, and the intended delivery paths and exact `openspec
      archive 2026-09-09-226-session-resumption --yes` postconditions
      below are selected and reviewed. Make the last tracked edit: tick 15.7 and set
      `## Progress` to the completed repository-local task truth while stating
      that archive, the named deferred assertion, the full unfiltered archived
      workspace suite and the delivery commit are still pending. The second
      provenance direction's pre-archive pass proves no round trip for this
      change. Select the full-suite command and failure/reopen behavior below;
      stage and inspect that final active diff. Once ticked, make no further tracked write before the archive
      effect. This tick attests readiness, not that archive already ran. If a
      readiness premise fails, reopen its owning task and remain active —
      progress / PM1, progress / PM4, boundary /
      The seat record carries the boundary as seat-record/v4.

**Post-task phase action — final archive effect and read-only verification.**
Once every tracked task is checked, run `openspec archive
2026-09-09-226-session-resumption --yes` as the normal dialect archive
operation; do not substitute a manual move or `--skip-specs`. Require the
archive to retain that exact dated identity, apply repaired AS1–AS3 and PM4
as replacements, treat the other sixteen identical requirements as no-ops, and
preserve all five existing
provenance sections byte-for-byte with one pointer each and retain
`boundary-record`'s earlier provenance and historical examples. Validate all
archived changes strictly, then run the complete
`cargo test --workspace --all-features --locked` with no filter or skip,
including both assertions in `crates/brokkr-cli/tests/provenance.rs`, and
inspect the complete staged diff, all read-only with respect to tracked
artifacts. Retain the full-suite command, result and output outside the
archived task file as archive-validation evidence. A fully passing unfiltered
workspace suite on these archived bytes is mandatory before the activation/
delivery commit; the pre-archive run is not its substitute. The scratch probe
is feasibility evidence only and does not satisfy this action. If archive completion is
uncertain, classify the active/archive namespaces, living truth, provenance,
index and `HEAD` before retrying. If the archive is partial or a check fails,
block the delivery commit, restore the same dated active authoritative change,
reopen the owning task and invalidate 15.7 before repairing; re-establish any
other affected gates before readiness. Never edit an archived task file in place
(`progress / PM1`, `progress / PM4`, boundary /
The seat record carries the boundary as seat-record/v4).

**Post-archive phase action — delivery commit.** Once the all-ticked change is
archived, staged and its strict validation and full unfiltered workspace suite
pass on the archived bytes, commit that exact tree
unsigned in the repository's message style, with no push or merge. Verify that
`HEAD` contains the checked archived task artifact and final progress state and
that `git status --short` is empty. If the commit fails, the phase action is not
complete and the head must not be handed off. Do not edit this file merely to
record the successful commit: that would make the tree dirty again. This is an
action after all tracked task checkboxes and the final artifact operation, not
a checkbox that circularly requires its own checked state to be present in the
commit (`progress / PM1`, `progress / PM4`).

**Post-commit controller handoff evidence — deliberately not tracked task
checkboxes.** Hand the final committed head and local evidence to the controller.
The controller records these results in its journal or other evidence outside
this committed task artifact, so observing them never changes the head they
validate:

- **Pending host exact coverage:** run `env TMPDIR=/var/tmp
  BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh` against
  that exact committed head with the gate unchanged. An in-box run or skipped
  boundary test is not this proof.
- **Pending integration and shipping evidence:** integrate the #222 shared-file
  overlap, require remote CI on the resulting final integrated head, publish the
  completed run, open and merge the PR, and close #226 only when those results
  exist. These remain controller-owned handoff conditions, not OpenSpec task
  completion state.

No post-commit controller result authorizes editing this archived artifact.

## Prior tasks-phase validation — 2026-09-09

Historical evidence from the tasks seats through `169e5b9`, before the
successor F7 amendment below. That work authored the breakdown only; no
production file, decision, contract or charter was touched.

- Coverage was self-checked by reading the four deltas: all **19**
  requirements — SR1–SR5, AS1–AS5, LE1–LE5, PM1–PM4 — are named by at
  least one task, and every task names the requirement it serves. The
  breakdown holds **101** tasks in 15 groups.
- `openspec validate 226-session-resumption --strict --no-interactive`
  did **not** run, on any of the three visits. The binary is on this
  seat's PATH (`/home/vyanakiev/.volta/bin/openspec`) — correcting the
  earlier reading that it was absent — but the sandbox refuses to
  execute it, so the strict validator and the status readout still have
  no result from here. Analyze ran it successfully on both of its
  visits, most recently against the artifacts as they stood before this
  second return's repairs, which touch task prose only: no task was
  added, removed or renumbered.
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
  stamped row that must still be refused. **That repair carried one
  false claim**, corrected in the second return below: it argued from
  the shipped adapter arms that the refusal-reason half of 2.4 could
  stay unconditional. The shipped arms are not the only producers the
  shared validator judges, so that half is now scoped on the same site
  stamp.
- **F2 (MEDIUM), 6.1 and 6.6.** The two tasks demanded different
  observable outcomes for the same input. Reconciled on the distinction
  the finding names, which is also design D5's: an *absent* `resume` key
  resolves to `unmeasured`, loads and compiles cold; *present malformed*
  data is a loader refusal naming the field, never a silent downgrade.
  Neither outcome enables resume. That distinction stands. **The list of
  what counts as malformed was wrong** in one entry, corrected in the
  second return below: it made an assessment with no assessed version a
  refusal, which would have refused the honest unmeasured declarations
  6.4 must write today.
- **F3 (MEDIUM), the header, group 10, 6.4, 8.8, 9.1, group 11 and the
  progress record.** Group 10 is split at its real seam: 10.1–10.4 are
  interface investigation, preparable here from installed help, source
  and the dated `.forge/` captures, and 10.5–10.8 are the blocked live
  proof. The stated execution order now runs the investigation before
  its consumers — 8.5–8.9 and 9.1–9.3 — and the proof after them, gating
  only group 11. Each dependent task names its prerequisite, and the
  progress record below states dependencies as they bind rather than by
  group number.
- **F4 (MEDIUM), then-numbered 15.6 and 15.7.** The commit preceded the fold, which
  would have left the archived change and the four promoted capability
  files outside the committed head, against decision 0042 ruling 6 and
  the design's Migration Plan 4-then-5. Swapped: fold, re-run the gate
  the fold moves — `crates/brokkr-cli/tests/provenance.rs` walks
  `openspec/specs` and the archive in both directions — then commit, then
  hand on that committed head.

The repair added four tasks (97 to 101) and moved no requirement citation:
all 19 requirements remain named.

## Return — analyze drift, second visit, 2026-09-09

Two findings, both owned by this file, both defects in the *previous
return's own repairs* rather than in anything the proposal, the four
deltas or the design says. Repaired here; again no other artifact was
edited, and no task was added or renumbered — the breakdown still holds
101 tasks in 15 groups and still names all 19 requirements.

- **F5 (HIGH), 2.4, 2.5, 2.6 and the F1 record.** The F1 repair scoped
  the resumed-requires-root rule on the site stamp and then argued that
  the second rule — a refusal reason only beside `cold` — could stay
  unconditional, because no shipped adapter arm emits a reason beside
  `resumed`. That is an inspection of the built-in arms, and the fence
  it constrains is not theirs alone: `crates/brokkr-store/src/lib.rs`
  runs one validator over every appended, exported and verified record,
  third-party driver checkpoints included, and v4 admits `launch` and
  `resume_refusal` independently
  (`contracts/seat-record.v4.schema.json:265-278`) over a free `step`
  string. So `{"step":"resume-declined","resume_refusal":"incompatible-argv"}`
  and `{"step":"harness-started","launch":"resumed","resume_refusal":"incompatible-argv"}`
  are valid v4 rows — synthetic contract counterexamples, not observed
  provider telemetry — and with 2.3 dispatching the whole 0.10.0 line to
  v5, an unconditional rule would newly refuse them at export, import
  verification and offline verification. Repaired the way the finding
  allows and the way D4 already words it: the fence rule is scoped on
  the 4.3 site stamp, the unconditional form binds this change's
  producers through group 7's lifecycle and the 5.8/9.7 conformance
  suites, 2.6 gains both unstamped refusal-bearing compatibility
  regressions and their stamped refusals, and the F1 record above no
  longer states the claim that failed.
- **F6 (MEDIUM), 6.1, 6.4, 6.6 and the progress record.** The F2 repair
  listed "an assessment with no assessed version" among the present-and-
  malformed data the loader refuses, while 6.4 requires four explicitly
  `unmeasured` assessments to be written *now* — and no wrapper
  measurement exists for LaneTally, whose identity 10.4 still owes, nor
  a route for DSH, which 10.3 still owes. The preparatory declaration
  would therefore have had to fail its own loader, invent an assessed
  version, or be omitted against 6.4 and the progress record's claim
  that group 6 is preparable. The absent-versus-malformed distinction is
  untouched; what was wrong was treating an honestly unknown identity as
  an authoring error. 6.1 now gives the assessed identity two explicit
  forms — a measured identity, or an unknown identity with a bounded
  reason — requires the measured form of every `supported` entry (so
  8.1 keeps a version to compare against), requires a bounded measured
  reason of every `unsupported` entry, and refuses an assessment
  carrying neither form or an unknown identity with no reason. 6.4 now
  says which form each of the four declarations takes and why it is
  writable today; 6.6 tests both loadable unmeasured shapes beside the
  refusals. This keeps AS1's `Neither local nor supplied interface
  evidence exists` scenario satisfiable and LaneTally independently
  assessed, and reopens no part of the enabled delivery minimum.

## Return — F7 specification repair, successor visit, 2026-09-09

The successor commission returns F7 to specify because the proposal omitted
Modified Capabilities while D4 and 2.3 selected v5 contrary to the living
boundary-record rule. The proposal and complete MODIFIED delta now own the
amendment; design D12 records the reason and corrects D10. This breakdown
carries that repaired dependency forward without reopening F1–F6. All prior
101 tasks and their identifiers remain; added task 2.8 makes **102** unchecked
delivery tasks in the same 15 groups. The five deltas contain **20**
requirements and **123** scenarios: the original 19/112 are unchanged and the
modified boundary requirement has seven retained scenarios (dispatch amended)
plus four additional cases.

Coverage of `boundary / The seat record carries the boundary as seat-record/v4`:

| Scenario | Tasks / verification |
|---|---|
| A boxed exec gate's record carries the word | 2.8 runtime boundary integration; 15.3 workspace proof and the post-commit controller host-coverage evidence. |
| A site without hands carries the sentinel | 2.8 runtime boundary integration. |
| The engine's word wins | 2.8 current-word overwrite and no-model removal. |
| A panel's aggregate carries none, a sequence's ending result its step's word | 2.8 composite markers/results. |
| A wrong word is refused at append | 2.5/2.8 direct v4/v5 refusal and attempt-failure path. |
| v4 dispatch is the 0.9 line | 2.3/2.6 matrix and tagged 0.9.0/0.9.1 compatibility. |
| The contract file is published beside the frozen ones | 2.2/2.7 retained v4 embedding, frozen pins and README row. |
| Every fence agrees at the version boundaries | 2.3/2.5/2.6 shared dispatch and v5-only refusal under v4. |
| Valid unstamped 0.10.0 history remains readable | 2.4–2.6 F1/F5 regressions with and without the site stamp. |
| v5 retains boundary authority and refusal behavior | 2.8 model-bearing launch stamping and direct invalid-word refusal. |
| v5 is published without changing any frozen version | 2.1/2.2/2.7 publication, embedding, frozen pins and README registration. |

Task 1.1 carries the amendment into proposed 0056 without accepting it; 13.3
keeps the guide consistent. Current tasks 15.5–15.7 and the post-task commit action preserve F4’s
fold/validate/commit order and include the fifth capability and its append-only
provenance. The
actual fold, implementation and provider/host validation remain delivery work.
The current specify validation is recorded in proposal.md, separately from
these tasks’ completion and from the historical phase evidence above.

## Tasks visit — design reconciliation, 2026-09-09

This run's design seat reconciled both current council positions and the newer
DSH source capture (`5d72a2c`) after clarification returned `clear`. That design
states no task amendment is needed and leaves the 102 identifiers intact; this
seat validated the breakdown against it rather than taking that statement on
trust. No requirement lost or gained a citation, no task was added, removed or
renumbered, and F1–F7 are not reopened. Four prose repairs carry the reconciled
design into the tasks that execute it:

- **Group 10's preamble and the conventions**: the controller's captures are
  now named through `.forge/controller-provider-evidence-index.md`, which
  indexes three, not the two the earlier text implied. D6's combined-probe rule
  is stated where the proof tasks read it: one probe may cover several axes only
  where each axis has its own attributable observation, and each proof task
  names the observation behind every axis it claims.
- **10.3**: design D6 has now read
  `.forge/controller-dsh-resume-source-interface.json` and narrowed this task
  from an open trace to one seam. The task names what that capture already
  establishes — `agents.resume`, `Config.agents[].resumeSessionId`/`resumeWith`,
  and the persistence-loading factory — and what it does not: captured
  `dsh-headless` still validates only `task`, mints a random agent ID and routes
  its followup there, so a restored second agent does not run the admitted task.
  The task's subject is that caller connection; an exported API, a mountable
  plugin or a resolvable key is not a route. Otherwise unchanged, including
  8.8's prerequisite, the forbidden mechanisms and the AS1 upstream return.
- **6.1 and 6.2**: root evidence states whether the root persists and a wrapper
  is qualified on its own wrapper, as reconciled D5 words it; the declaration
  shape stays closed and compact, with no evidence database, probe DSL,
  version-range resolver or self-enabling path behind it (D5, D10). 6.1's two
  identity forms and 6.6's absent-versus-malformed distinction are untouched.

Checked against the current artifacts on this visit: the five deltas hold **20**
requirements, every one cited by at least one task and every task naming what it
serves; the file holds **102** tasks with unique identifiers in 15 groups, all
unchecked; the design's references to tasks 2.3, 2.8, 3.5–3.6, 6.1/6.6, 8.1,
8.5–8.9, 9.1–9.3, 10.1–10.4, 10.3, 10.7 and current tasks 15.5–15.7 all
resolve, and the ordering they require — investigation before construction,
proof after it and gating only group 11, fold before the post-task commit — is
the ordering this file states.

`openspec validate --strict` did **not** run from this seat: the sandbox refused
both to execute the binary and to stat it outside the worktree, so this visit
has no strict-validator or status result of its own, as the earlier tasks visits
also recorded. The specify and design seats
recorded their own passing runs against these bytes; this visit changes task
prose only and adds no heading, checkbox or identifier. No Rust check was run
here — this phase writes no code, and a green suite is group 15's evidence, not
this artifact's. No provider CLI was probed and no provider evidence was created.
Frozen contracts, policy, reference, fixtures, production code, living specs and
the sibling fire's files have no diff from the commissioned base.

## Current tasks visit — council reconciliation, 2026-09-09

The design council reconciliation at `35fa9ae` supplies new implementation
evidence after the earlier task visit: Claude's planner rejects competing
conversation selectors but otherwise appends `extra` without the
restriction-by-restriction duplicate, arity and precedence checks required by
AS3, and a different-root `LaunchHold` outcome can suppress launch evidence
while still allowing the invocation's ordinary clean/result outcome to be
accepted. The design locates these gaps in existing tasks 8.10 and 9.7 rather
than adding or renumbering work.

This visit therefore returns **8.10** and **9.7** to pending and makes their
acceptance evidence explicit. Task 8.10 now requires the provider-local guard
and independently asserted Claude permission/tool/MCP/model/effort cases on
both cold and resume paths. Task 9.7 now requires the shared terminal guard and
the clean-exit and valid-result mismatch variants, with no accepted success,
guessed launch or replacement. Its citations now include `safety / AS4`, the
requirement whose different-session scenario owns that behavior. All other
identifiers, dependency order and citations remain intact; F1–F7 are not
reopened, and the five deltas still contain 20 requirements covered by the 102
tasks.

No implementation or frozen artifact is changed in this phase. The next smith
works 8.10 before 9.7, then resumes the already-open provider-proof and delivery
tasks in their stated order. The task truth is now **90 complete / 12 pending**.

## Progress

Current tasks return, 2026-09-20, THIRD DSH SECURITY HOLD: AP/AQ/AS1 and
adopted design `76daef9d` are reflected in the ordered local breakdown above.
The returned design/archive notices are answered in Decisions and the current
validation account. All 101 change-wide states remain **84 complete / 17
pending**; the fourteen local addresses remain **5 complete / 9 pending**.
This return changes no tick. No implementation, native oracle or removal was
executed by this tasks seat. Rust gates, native-platform/retained-Node proof,
fresh literal exact coverage and implementation delivery remain pending.
8.8 stays unchecked. The current validation account above records this seat's
artifact checks and tool failures; earlier progress entries are dated history.

### Implement — 2026-09-09, run `close-issue-226-only-codex-resum-805ec715`

**At the close of that implementation visit, 90 of 102 delivery tasks were
ticked.** Twelve stayed unchecked — the two
conformance repairs returned by the current tasks visit, the eight provider-proof
and enablement tasks, plus then-numbered 15.5 (host exact coverage) and
15.8 (controller handoff) — each with its reason below. The
predecessor's 92/10 count was correct for `75ae68e`; the design evidence at
`35fa9ae` invalidated the proof behind 8.10 and 9.7, so current evidence returns
them to pending as task 12.2 requires.

**This visit began inside the failure #226 describes.** The tree held
about five thousand uncommitted lines — the seat-record v5 contract, the
engine's `resume` module, the four provider planners, the launch
lifecycle, the adapter declarations, the charter and the guides — beside
a `## Progress` section reading *implementation has not begun* and 0 of
102 ticked. The predecessor died mid-phase with its session, and the
artifact it left said nothing about what it had done. Reconciled by
reading the code and running its checks rather than by trusting either
the artifact or a memory this seat does not have. No surviving edit was
discarded.

Repaired in this visit, none of them a change of plan: a `useless_format`
in the group 7 shim tests, and two group 3 helpers that had been appended
*after* `resume.rs`'s test module, which
`clippy::items_after_test_module` refuses — both formatting-level repairs
to the code the predecessor wrote. The third surfaced only at the fold:
`crates/brokkr-cli/tests/rename_guard.rs` keeps a list of contract titles
that living specs may quote as data, and the folded `boundary-record`
amendment quotes `Forge seat record v5` beside v4's. The title is v5's
real one — task 2.1 requires the v4 file's spelling — so the guard's data
list gains the row, exactly as it holds v4's, v9's and the two v1/v4
siblings. The guard's own test that a quoted title never licenses retired
prose covers the new row too. No requirement moved.

Green on this worktree, in the order group 15 names them: `cargo fmt
--all -- --check` (15.1), `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` (15.2), `cargo test --workspace
--all-features --locked` (15.3, 66 test binaries, 0 failures), both
bundle compiles (15.4, and 14.2's manifest digests needed no further
reconciliation), and after the fold, `cargo test -p brokkr-cli --test
provenance` (then-numbered 15.6) and the whole suite again.

**The fold (then-numbered 15.6) was performed by hand, because the sandbox refuses to
execute `openspec` as it refuses every other binary outside this
worktree.** It replicates what the archive operation does, checked
against the `2026-09-06-boundary-named-slice-i` precedent line by line:
the change directory moved under `openspec/changes/archive/` with `git
mv`; each ADDED delta copied to `openspec/specs/<capability>/spec.md`
with the title line added and `## ADDED Requirements` retitled
`## Requirements`; the MODIFIED `boundary-record` requirement replacing
its living predecessor in place, with that capability's other two
requirements and its `2026-09-06-boundary-named-slice-i` provenance line
untouched; and one appended provenance line per capability in the
spelling `dialects/openspec/archive.md` gives. The fold's own gate,
`crates/brokkr-cli/tests/provenance.rs`, walks both directions and
passes. `openspec validate --strict` has no result from this seat, as
every planning visit before it also recorded.

Unchecked, and why:

- **8.10 and 9.7, implementation conformance.** Current code inspection in the
  reconciled design shows that Claude still accepts duplicate or last-wins
  authoritative restriction controls through `extra`, and that a different
  resumed root can suppress a launch row without overriding an otherwise
  successful invocation result. The tasks now name the missing provider-local
  validation, shared terminal guard and distinguishing tests. No tasks-phase
  edit pretends those production fixes or checks have already run.
- **10.5–10.8, live provider proof, and 11.1–11.4, the enablement they
  gate.** This seat's sandbox refuses to *execute* any provider CLI.
  `claude`, `codex`, `dsh` and `claude-lanetally` all resolve on PATH —
  `/home/vyanakiev/.local/bin/claude`, `/home/vyanakiev/.volta/bin/codex`,
  `/home/vyanakiev/.volta/bin/dsh`,
  `/home/vyanakiev/.local/bin/claude-lanetally` — and every invocation of
  one, down to `--version`, is denied, as are reads outside this
  worktree. So no root-opening semantics, restriction enforcement or
  current-only accounting could be observed on any installed version, and
  under AS1 no declaration may flip on interface evidence alone. All four
  shapes therefore stay as group 6 wrote them: Codex and Claude
  `unmeasured` against measured identities that do not qualify the
  installed CLI, LaneTally `unmeasured` with the explicit unknown
  identity, DSH `unsupported` with its measured reason. **This is a
  missing measurement, not a measurement of absence**, and it is the one
  thing standing between this change and the commission's enabled
  delivery minimum. The engine, the wire, the planners, the argv and the
  refusal paths are built and proven against deterministic shims; what
  they wait on is a host that may run the CLIs.
- **10.3 is ticked, and it reports AS1 upstream.** The investigation half
  reached its stated end: the installed source establishes `agents.resume`,
  `Config.agents[].resumeSessionId`/`resumeWith` and the persistence-loading
  factory, and it establishes that captured `dsh-headless` validates only
  `task`, mints its own `session-${randomUUID()}` and applies its followup,
  `firstSeq` and summary to *that* agent. No supported declarative route
  connects the admitted headless caller to an owned root. Closing it needs
  a mechanism this change is forbidden: replacing the runner plugin,
  monkey-patching `agents.create`, overriding UUID generation, editing the
  installed package or substituting the TUI. **The exact upstream
  requirement** is therefore: `dsh-headless` must accept a session or
  resume identifier in its config schema and route its admitted task,
  followup and event interval to the restored agent rather than to a
  freshly created one. Recorded in `adapters/dsh.json` and
  `docs/guides/provider-adapters.md`; the shape is declared `unsupported`
  with that reason rather than narrowed away.
- **Then-current 15.8 (originally 15.5), now post-commit controller evidence,
  the exact coverage gate.** Not run, and not claimed. The
  sandbox refuses to execute `scripts/coverage-exact.sh` at all, and
  refuses the `TMPDIR`/`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` prefix the
  commission specifies with it. The gate's substance was attempted
  directly instead, three times, on the pinned `nightly-2026-09-05` with
  the installed `cargo-llvm-cov 0.9.0`: each run compiled and executed
  most of the workspace and then died inside its own instrumented target
  directory — twice on
  `target/llvm-cov-target/debug/build/brokkr-cli/<hash>/out/brokkr-<hash>`
  not existing when cargo went to run it, once on a dependency graph that
  could not be moved into place, all `os error 2`. The sanctioned script
  avoids this by pointing `CARGO_LLVM_COV_TARGET_DIR` at a fresh
  directory under `TMPDIR`, which is the environment variable this seat
  may not set. **Every failure was the harness, not a threshold.**

  A fourth attempt, after clearing `target/llvm-cov-target` by hand,
  completed. Its report was then evaluated by the gate's OWN arithmetic
  rather than by LLVM's summary percentages — which count compiler
  instantiations, not source lines, and are therefore not what the gate
  reads: `cargo llvm-cov report --branch --lcov` over the whole
  workspace has **zero `DA` records with no hits and zero `BRDA` records
  with no hits**. That is the gate's literal integer equality on lines
  and branches. Its function half follows: it collapses symbols by file
  and start line and asks for one covered instance, and a source
  function with none would have left uncovered lines, of which there are
  none.

  What that is NOT, and what stays unchecked: the gate's own run, on the
  host, with `TMPDIR=/var/tmp` and `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`,
  where the boundary tests create a namespace instead of being skipped
  inside a box. That result does not exist here and only the controller
  can produce it. A skipped boundary test would prove nothing either
  way, and the gate was not lowered or edited.
- **The then-current controller handoff task, later 15.9 and now post-commit
  controller evidence.** Host validation,
  remote CI on the final
  head, integration with the #222 fire's shared-file overlap, the PR, the
  merge, publication and the closing of #226. Their results do not exist.

## Return — analyze drift, finalization ordering, 2026-09-09

The first repair at `ac9c16e` correctly separated the already completed
one-time fold from the still-pending final re-archive, and correctly reopened
the re-pins and local gates whose earlier results cannot prove later code or
declaration edits. Its finalization order was nevertheless still defective.
This returned visit adopts both new findings and repairs only their earliest
owning artifact, this breakdown.

- **F8 (HIGH), then-current 15.5–15.9.** Host exact coverage is
  controller-owned validation of the final committed head under proposal
  Delivery obligations and design Migration Plan 5. It is no longer a
  prerequisite of re-archive or commit. The checked one-time fold is 15.5;
  then-current 15.6 required remaining implementation, re-pins and local gates
  15.1–15.4 before strict validation and final re-archive. The then-current
  15.8–15.9 placed controller evidence after the commit. That resolved the
  pre-final-tree coverage dependency, but F10 below establishes that those
  entries cannot remain checkboxes.
- **F9 (MEDIUM), then-current 15.7 and the post-task phase action.** A commit
  cannot truthfully be its own checkbox while also containing that checkbox's
  final tick and leaving a clean tree. The repair correctly made the unsigned
  delivery commit a post-task action and prohibited a post-commit edit merely
  to record it. F10 preserves that answer while correcting the artifact
  ordering and remaining post-commit checkboxes.

### F10 — Archived task closure and exact-head handoff

The latest HIGH finding is adopted on new evidence from the F8/F9 ordering.
Their repair left two distinct contradictions: 15.6 archived before 15.7's
tracked edit, and then-current 15.8–15.9 remained unchecked after archive.
Decision 0042 ruling 4 requires archived strict verification and makes an
unticked task fail that verification. Checking either controller item later
would mutate the exact committed head whose host coverage or remote CI it was
meant to prove. This is not a reopening of the settled separation between
commit and coverage; it is the consequence of applying the archived-task rule
to that repaired sequence.

Current 15.6 now completes reconciliation and every ordinary tracked edit while
the change is active. Current 15.7 makes its own tick and the final progress
account the last tracked artifact edit while active, then re-archives the change
as the final artifact operation. Archived validation and provenance checks are
read-only; a failure requires reopening the same change before any repair. The
delivery commit follows as an untracked phase action, preserving F9.

The former 15.8–15.9 obligations are not waived or narrowed. They are now
explicit post-commit controller handoff evidence with no checkboxes: sanctioned
host exact coverage validates the exact committed delivery head, while
integration, remote CI on the final integrated head, publication, PR, merge and
issue closure stay pending until the controller records their real results
outside this artifact. No such result may cause a post-commit edit here.

No proposal, delta, design, living capability, provenance, provider
declaration, production file, frozen artifact or sibling-fire file changes in
this return. Proposal answer I's remaining pre-archive delivery obligations are
the repository-local tracked tasks; its controller-owned external checks remain
post-commit handoff conditions. Design Migration Plan 4–5 already orders final
artifact operation, commit, then controller handoff, so the defect is confined
to this owning breakdown.

The current task truth is **83 complete / 18 pending across 101 tracked
tasks**. Pending tracked work is dependency-ordered: provider-local
conformance, shared terminal conformance, live provider proof, evidence-gated
enablement, refreshed pins and local gates, active-change reconciliation and
the final re-archive artifact operation. The post-task delivery commit and
controller evidence are deliberately outside the checkbox count.

Validation for this repair passed: `openspec validate
226-session-resumption --strict --no-interactive` accepts the active change,
`openspec status --change 226-session-resumption --json` reports planning
complete, and a structural check finds 101 unique task identifiers with the
stated 83/18 split. Every checkbox still names its served requirement; the
post-task action names PM1/PM4, and the five living capabilities retain exactly
one provenance line for this archive plus the earlier `boundary-record`
provenance. No Rust or provider test is required for this task-artifact-only
repair; their refreshed commands remain delivery tasks 15.1–15.4, and host
coverage remains mandatory controller evidence.

## Current tasks return — F10 dependency reconciliation, 2026-09-10

This tasks visit adopts the completed clarification and design repairs at
`e047f57` and `6af3486`. They supply new owning-artifact evidence after the
first F10 task repair: the returned archive must run the normal dialect
operation so it can apply the repaired PM4 requirement. No settled F1–F9
answer is reopened, and the former 15.8–15.9 exact-head obligations remain
mandatory non-checkbox controller evidence.

Task 1.1 is reopened because proposed 0056 ruling 10 still lacks the specified
exact-head separation and must retain `Status: proposed` without implying
operator acceptance. Checked task 15.5 now preserves the earlier fold while
explicitly permitting the required idempotent archive. Task 15.7 replaces the
stale manual move with `openspec archive 226-session-resumption --yes`, requires
only repaired PM4 to change while identical deltas no-op and provenance remains
singular, and makes failed or partial archived verification restore this same
active change before repair. It stays the final tracked artifact operation;
the delivery commit and exact-head controller proof remain read-only follow-ons.

The identifiers remain stable at **101 tasks in 15 groups**. Reopening 1.1
makes the truthful state **82 complete / 19 pending**: proposed-decision repair,
provider-local and terminal conformance, live provider proof, evidence-gated
enablement, refreshed pins and local gates, active reconciliation and the final
archive transaction. Host coverage, integration, remote CI, publication, merge
and issue closure remain mandatory controller evidence outside that count.

Validation on this visit passed: `openspec validate 226-session-resumption
--strict --no-interactive` accepts the active change, `openspec status --change
226-session-resumption --json` reports planning complete, and structural checks
find 101 unique identifiers with the stated 82/19 split. All 20 requirements and
125 scenarios retain task coverage, and every changed task names the requirement
it serves. No Rust or provider check is claimed for this task-artifact repair;
those checks remain dependency-ordered delivery work.

## Current tasks return — F11 pre-archive readiness, 2026-09-10

This returned visit adopts the current design reconciliation at `1f00910` and
repairs its only downstream inconsistency. The prior F10 repair correctly
removed exact-head host coverage and remote delivery evidence from tracked
checkboxes, but current task 15.7 still combined a readiness tick with the
archive operation and its archived checks. This supersedes only the prior
return records' current-tense description of 15.7; those records remain
historical evidence of their visits. Under `progress / PM1`, that tick
would be false until the operation completed; under decision 0042, no tracked
edit may then repair the archived artifact.

Task 15.7 is now only the attainable active-state readiness task specified by
design D9: all earlier repository-local work and active validation complete,
the final progress truth and tick written while active, and the intended archive
transaction reviewed. The normal `openspec archive` operation is the final
non-checkbox artifact effect, followed only by read-only archived validation
and provenance checks, the delivery commit, and controller evidence external to
the exact head it judges. The recovery text distinguishes a lost acknowledgement
from a partial fold and requires reopening the same authoritative change before
any repair. No F1–F10 answer, delivery minimum, provider proof, host gate or
controller obligation is narrowed or waived.

The identifiers and completion truth remain **101 tasks in 15 groups, 82
complete / 19 pending**. The changed task continues to serve `progress / PM1`,
`progress / PM4` and the modified boundary-record requirement; the archive
effect names the same requirements outside the checkbox count. All 20
requirements and 125 scenarios retain their existing implementation and
verification coverage.

Validation on this visit passed: `openspec validate
226-session-resumption --strict --no-interactive` accepts the active change,
`openspec status --change 226-session-resumption --json` reports planning
complete, and structural checks find 101 unique identifiers at the stated
82/19 split with no tracked 15.8–15.9. The five deltas contain the stated
20 requirements / 125 scenarios, and `git diff --check` passes. No Rust or
provider check is claimed for this task-artifact-only repair; those checks
remain dependency-ordered delivery work, and host coverage remains mandatory
controller evidence.

## Current tasks return — F12 dated same-change recovery, 2026-09-10

This return adopts proposal I/F12 and design D13 on the new controller evidence.
Task 15.5 records the actual history-preserving move back to the original active
identity `2026-09-09-226-session-resumption` plus inspection of the installed
archive rule and scratch feasibility probe. Decision 0042 already permits this
same-change reopen-and-refold path, so no supersession task, verifier exception
or accepted-decision amendment remains.

Tasks 15.6–15.7 and every implementation/provider/gate task remain pending
until their real work and evidence complete. The scratch copy's ticks and
archive complete none of them. The current truth is **82 complete / 19 pending
across 101 tracked tasks**. All 20 requirements / 125 scenarios retain coverage.
The eventual post-task action uses the normal dated archive command, preserves
the five existing provenance sections without duplicates, and requires strict
archived validation plus Rust bidirectional-provenance checks before commit.
Active strict validation passes under the dated identifier, OpenSpec status is
complete, the existing archive validates strictly, and `git diff --check` is
clean. Cargo is unavailable in the box; no real archive, Rust/provider proof,
host gate or shipping action is claimed here.

## Current tasks return — J release-binary gate, 2026-09-10

This returned visit adopts proposal answer J and the corresponding D9/D11
design reconciliation at `52a743c`. The earlier breakdown omitted the
commissioned repository-local release-profile proof even though it can run
before the final artifact operation. Pending task 15.4 now names
`cargo build --release --locked -p brokkr-cli` after both bundle compiles and
requires its own successful result on the same final active bytes; tests, debug
builds and bundle compiles cannot stand in for it. No task is added,
renumbered or ticked, and no release versioning, packaging or publication work
enters this change.

The identifiers and task truth remain **101 tasks in 15 groups, 82 complete /
19 pending**. Task 15.4 continues to serve every requirement of the change and
now explicitly serves `progress / PM4`'s per-commission command scenario. All
20 requirements / 125 scenarios retain coverage, while exact-head host
coverage, integration, remote CI, publication, merge and issue closure remain
mandatory non-checkbox controller evidence.

This tasks-only repair claims no Rust, provider, bundle or release-binary
result. Those checks remain pending in dependency order before active-change
reconciliation and pre-archive readiness.

## Implement — 2026-09-10, run `complete-issue-226-in-full-adopt-2b5ab3ff`

This visit completed the three repository-local tasks this change still had
open, each with its named focused check, and made no provider claim.

- **1.1** now carries the exact-head separation the F10 clarification
  required. Ruling 10 in proposed `docs/decisions/0056-*` states that every
  tracked repository-local obligation is finished and ticked before the
  normal archive operation, that the delivery commit is an untracked phase
  action, and that exact-head controller evidence — host exact coverage,
  remote CI, integration, publication, PR, merge and issue closure — is
  recorded outside the checked task state and this artifact, because
  checking it would mutate the head it judges. Its enforcement binding names
  design D9/D11's finalization order. `Status: proposed` is unchanged.
  Check: `cargo test -p brokkr-cli --test decisions_index --locked`.
- **8.10** gains `claude_restriction_conflict` in
  `crates/brokkr-protocol/src/adapters.rs`: each authoritative Claude
  restriction control (permission mode, tools list, strict-MCP config,
  MCP config, allowed/disallowed tools including the alias pair, model,
  effort) is refused when duplicated or when a value-taking spelling has no
  value, on both cold and resume paths, before any provider work. The
  existing exact-argv, class/model/effort, selector, version-drift,
  identifier, DSH and hands-refusal tests are unchanged and still pass.
- **9.7** gains `Invocation.launch`/`LaunchTerminal`: `run_seat` now fails
  an invocation whose rejoin was never confirmed, or confirmed a different
  root, even when the child exits zero and writes the result file, with no
  guessed launch and no replacement (`adapters.rs`, `adapters/tests.rs`),
  plus a codex wire-level conformance case in
  `crates/brokkr-cli/tests/driver_conformance.rs`.

**Still pending, and why no archive can yet run.** The live provider proofs
10.5–10.8 and the enablement 11.1–11.4 they gate cannot be produced from
this box, so 14.1–14.2, 15.1–15.4, 15.6, 15.7 and the final archive
transaction remain open:

- The root filesystem is mounted read-only; only this worktree and `/tmp`
  are writable. `$CODEX_HOME` (`~/.codex`) and `$CLAUDE_HOME`
  (`~/.claude`) therefore cannot persist anything, and `$DSH_HOME`
  (`~/.dsh`) cannot compose a profile.
- Codex: `codex exec` fails at `failed to initialize in-process app-server
  client: Read-only file system (os error 30)` before any session exists.
- Claude: a cold `2.1.266` print/stream-json run succeeds and names a
  session id, but `--resume <id>` answers `No conversation found with
  session ID: …` because the transcript home cannot be written. The
  installed symlink is 2.1.267 while the measured identity is 2.1.266.
- DSH: independently verified against the installed `0.1.2-rc.1` bytes
  (40/40 captured-source hashes match). `dsh-headless` accepts only `task`
  and always calls `agents.create` with `session-${randomUUID()}`; no
  supported declarative per-invocation route reaches a restored root, so
  10.7 has no route to prove and 11.3 has no shape to enable. The upstream
  capability is that `dsh-headless` must accept a session/resume identifier
  in its supported config schema and route its admitted task, event
  interval, summary and flush to the restored agent.
- LaneTally: `~/.local/bin/claude-lanetally` execs into
  `/home/vyanakiev/source/feedback-loop-ai/lanetally/rollout/session-wrapper.sh`,
  a sibling worktree this commission forbids touching, so 10.8/11.4 cannot
  be measured here and LaneTally stays declared-unsupported.

AS1's measured-impossibility path therefore applies: the delivery minimum
answer A requires is unmet, and the owning specification owns the return
with the measured reason. No adapter status was flipped, no unmeasured
shape was enabled, no provider setting or installed package was changed,
and no credential or private transcript was read. The focused Rust suites
above, `cargo fmt --all -- --check`, and the protocol/cli clippy runs pass
on this tree; the full host gates 15.1–15.4 and the archive effect are not
claimed.

## Current tasks return — exact DSH extension breakdown, 2026-09-10

This tasks visit adopts proposal answers K/L and current design D6/D10. It read
`controller-evidence-2026-09-10.md` and
`.forge/tasks/controller-dsh-upstream-discovery.json` first, then the five
supplied Claude probe/analysis artifacts and the rendered OpenSpec tasks
instructions. No open design question remains, so no upstream artifact must
change before implementation resumes.

The selected route changes three completed implementation claims. Tasks 8.8,
8.10 and 9.6 are therefore reopened for the exact isolated official DSH
0.1.5-rc.1/plugin 0.2.0 path, retained-root/composite-identity binding,
provider-local selector and restriction guards, and current-sequence
accounting/storage tests. Checked task 10.3 keeps its completed status because
the old one-shot trace remains valid and the supplied official/plugin inspection
now establishes the supported caller route; its text no longer reports the
installed 0.1.2-rc.1 entry as a global limitation. Pending 10.7 and 11.3 now
name the exact dependency graph, independent root confirmation, restriction
precedence, accounting qualification and evidence-gated enablement. Generic
9.1/9.2 and the shared terminal guard in 9.7 remain checked as design directs.

Pending 10.6 now consumes the controller's Claude evidence as partial instead
of repeating it. Its remaining bounded observation requires cold native-Write
and MCP positive controls; explicit same-root attempts at expired Read, removed
Write and removed MCP authority; a working replacement Read grant; the complete
filesystem/precedence result; and raw stream/transcript identity reconciliation
for the exceptional invisible-response/turn cases. If worker execution is
unavailable, the implementer supplies that exact executable task-owned probe to
the controller; worker-home EROFS is not a controller blocker.

The archive action is corrected for answer L: the dated normal archive replaces
AS1–AS3 and PM4, no-ops the other sixteen identical requirements and preserves
all five singular bidirectional provenance sections. No task identifier is
added or renumbered. The authoritative state is **82 complete / 19 pending
across 101 unique tasks**, with all 20 requirements / 125 scenarios covered.
Implementation proceeds in dependency order: 8.8, 8.10, 9.6; provider proofs
10.5–10.8; evidence-gated enablement 11.1–11.4; re-pins, local gates, active
reconciliation and readiness; then the normal archive effect, read-only
archived checks and delivery commit.

This visit changes only `tasks.md`. Strict active validation and OpenSpec status
pass, the structural count and requirement-citation audit pass, and
`git diff --check` is clean. It claims no provider qualification, Rust test,
release build, archive, publication, merge or issue closure.

## Current tasks return — fail-closed DSH truth ledger, 2026-09-10

This returned tasks visit answers current design D10's finding in dependency
order. The earlier tasks return correctly selected the exact official DSH
0.1.5-rc.1/plugin 0.2.0 route and reopened its Rust planner, guard and
accounting work, but it left four checked artifacts certifying the displaced
unsupported-route premise. Current evidence therefore invalidates the checks
behind 1.1, 6.4, 11.5 and 13.1 under PM2; their implementation is not preserved
as complete merely because the files still exist.

The breakdown reopens only those four task IDs. Task 1.1 corrects proposed 0056
while preserving `Status: proposed`; 6.4 writes the exact selected route as an
unknown-composite, `unmeasured`, disabled assessment; 11.5 reconciles every
shipped/packaged/scaffolded representation; and 13.1 makes the provider guide
state the same evidence boundary. Each keeps installed 0.1.2-rc.1 as a bounded
one-shot result, preserves the independent DSH hands deferral and refuses both
premature enablement and the disproven global limitation.

The executable dependency seam is now explicit:
**1.1 -> 6.4 -> 11.5 -> 13.1 -> 8.8 -> 8.10 -> 9.6 -> 10.5–10.8 ->
11.1–11.4 -> 14.1–15.7**. Checked 10.3 still owns the completed source/interface
selection; 10.7 still owns live compatibility, independent-root, restriction
and current-sequence/accounting qualification; 11.3 still owns evidence-gated
DSH enablement. The Claude task continues to consume the five supplied probes
as partial evidence and asks the controller only for their exact missing
combined observations, never for a repeat of proven continuity or Read-grant
replacement.

The current task truth is **78 complete / 23 pending across 101 unique tasks**.
All 20 requirements / 125 scenarios retain coverage and every task retains a
named requirement plus verification. The preceding 82/19 account remains
historical truth for its own visit and is superseded by this returned-design
correction. No provider proof, Rust implementation test, release build, archive,
publication, merge or issue closure is claimed here.

## Implement visit — DSH pair measured incompatible, 2026-09-10

This implement visit completed the four pre-8.8 truth repairs and then qualified
the exact selected DSH pair. Tasks **1.1, 6.4, 11.5 and 13.1 are now complete**:
proposed 0056 selects core 0.1.5-rc.1 at
`183f08e9c6dde7e36cd2318eaee70b0da08fb35e` with `dsh-plugin-cli-session` 0.2.0 at
`0f487e74c81ed102c6899440d9f5d65e8e9eabda` while remaining `Status: proposed`;
`adapters/dsh.json`, `docs/guides/provider-adapters.md` and every shipped or
packaged declaration call that shape `unmeasured`, disabled and waiting on 10.7;
and the installed 0.1.2-rc.1 one-shot result stays bounded history, never a
global limitation. Task truth is therefore **82 complete / 19 pending across the
same 101 unique tasks**.

Task 10.7's isolated qualification is measured and **fails the exact pair's
compatibility**. Under an isolated `DSH_HOME` the pair composes, loads, runs a
cold `--new` turn and a same-root warm `--session <owned-id>` turn, and recalls
the cold nonce — but then both invocations throw `dsh: events is not iterable`,
exit nonzero and emit no `stream-json` result envelope. Cause: plugin
`lib/index.js` reads `agent.session.events`, while core 0.1.5-rc.1's `Session`
no longer exposes a public `events` member (it now provides `eventAt()`,
`snapshotEvents()`, `ownEvents()` and `firstLiveSeq`). The static counter-check
confirms the API moved between generations: `@deepseek-ai/dsh-session`
0.1.0-rc.6 still has `get events()`, so the plugin's own dev pin is the
supported one. The live 0.1.0-rc.6 re-run did not finish in this visit; it is a
candidate re-pin, not a qualified route. Evidence:
`.forge/tasks/dsh-pair-qualification.json` and
`.forge/tasks/dsh-pair-incompatibility.json`. Per answer K this is the precise
unmet AS1 condition, not the disproven global DSH limitation; no package patch,
UUID interception, Cordis shim, SDK or TUI substitution was performed. Tasks
**8.8, 8.10, 9.6, 10.7 and 11.3 remain unchecked**, and the route needs an
owning specification/design re-rule (a plugin revision that supports core
0.1.5-rc.1, or an explicit re-pin to the plugin's supported core) before they
can proceed.

The other provider proofs advanced but stay partial. Codex 10.5
(`.forge/tasks/codex-proof-2026-09-10.json`) observed the 0.153.4 resume
grammar, same-thread cold/resume continuity, nonce recall, same-root, the
pre-work `thread/resume` rejection shape and per-invocation current-only
accounting; OS-level read-only enforcement stays unmeasured because the host
blocks unprivileged user namespaces and codex's `bwrap` cannot start. Claude
10.6 (`.forge/tasks/claude-proof-2026-09-10.json`) ran one bounded same-root
pair on pinned 2.1.266 with real cold Write and MCP controls, resumed denial of
the expired Read grant, the removed Write and the removed MCP tool, an operating
new Read grant, observed precedence and exact per-message accounting, but
Brokkr's adapter normalization is not yet cross-checked, so the task is not
ticked. LaneTally 10.8 remains unmeasured. No provider is enabled.

Local gates on the current active bytes: `cargo fmt --all -- --check` clean;
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
clean; both bundle compiles exit 0; and `cargo build --release --locked -p
brokkr-cli` exits 0. The workspace suite is green except the known transient
`provenance::every_capability_names_the_archived_changes_that_wrote_it`, which
fails only because this change is active and its five living capability pointers
resolve again after the final archive, and the nested-box
`machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_evidence`,
which requires a host that can open the box. The real archive, exact-head
controller coverage, integration, CI, publication, merge and issue closure
remain pending and unclaimed.

### 2026-09-10 — successor implement: DSH pair re-pin qualified

Task 10.3's candidate re-pin is now measured. Under an isolated task-owned
`DSH_HOME` (`.forge/dsh-qualify-010rc6/`), official core `@deepseek-ai/dsh`
0.1.0-rc.6 — the plugin's development pin, whose host modules resolve to
0.1.0-rc.8 and still expose `agent.session.events` — composes the `headless`
profile with `dsh-plugin-cli-session` 0.2.0 and completes both live turns: cold
`--new` and warm `--session <id>` each exit 0 with a `stream-json` result
envelope, the same `session-a95eacaf…` root, cold-nonce recall on the warm turn
and per-message usage, while the global 0.1.2-rc.1 pin, profiles and credentials
are byte-unchanged. Evidence:
`.forge/tasks/dsh-pair-qualification-010rc6.json`; the 0.1.5-rc.1 failure
remains bounded history in `.forge/tasks/dsh-pair-incompatibility.json`.
Proposal K, design D6, decision 0056, AS1, `adapters/dsh.json`, the provider
guide and tasks 1.1/6.4/10.3/10.7/11.3/11.5/13.1 now name core 0.1.0-rc.6.
The shape stays `unmeasured` and **8.8, 8.10, 9.6, 10.7 and 11.3 remain
unchecked**: consumer-side exact-root confirmation, restriction precedence after
restore, a multi-message/retry accounting boundary and the Rust route are still
owed. No provider is enabled and no task is ticked from the qualification alone.

## Current tasks visit — DSH forward-pin reconciliation, 2026-09-10

The operator's 2026-09-10 ruling reverses `a86eca1`'s re-pin of the DSH
session route to core 0.1.0-rc.6 and selects the latest official core
(`@deepseek-ai/dsh` 0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`,
or the release answer N1 resolves in its place) forward, paired with a
repository-owned six-file adaptation of `dsh-plugin-cli-session` 0.2.0 under
`extensions/dsh/plugin-cli-session/`. Design D6 already carries this reversal
(`d483c64`); this visit repairs the task breakdown to match it, since design
is the settled input this phase builds against and the design/tasks
mismatch left the wrong version pinned in eight tasks.

This visit reopened the four truth-repair tasks `a86eca1` had completed —
**1.1, 6.4, 11.5 and 13.1** — because their checked bytes still named the
superseded 0.1.0-rc.6 pin; the task truth returns from 82/19 to **78
complete / 23 pending across the same 101 unique tasks**. It amended checked
**10.3** in place, adding the discovered replacement accessor
(`snapshotEvents(fromSeq?, toSeqExclusive?)`, with its declaring/implementing
hashes and the index-equals-sequence reading) and turning the interim
0.1.0-rc.6 re-pin sentence into dated history rather than a live claim,
without unchecking it, per design's instruction to amend a settled discovery
task in place. It rewrote pending **8.8, 8.10, 9.6 (unchanged — already
version-agnostic), 10.7 and 11.3** for the forward pin: 8.8 now names the
committed six-file adaptation, its one-line accessor delta, the plugin
component and canonical composite functions, and the committed-bytes test;
10.7 is split into a live half that installs 0.1.5-rc.1, commits the
adaptation and records `.forge/tasks/dsh-pair-qualification-015rc1.json`
before 1.1/6.4/11.5/13.1 can honestly run, and a checkbox that still gates on
8.8's digest agreeing with that evidence, exactly as it gated on the
superseded record; 11.3 now enables only the resolved-core/adaptation
composite and is the sole task that writes `wrapper_digest`. The intro's
execution-order note and the group 10 preamble line naming 8.8's candidate
were updated to match. No task was added, removed or renumbered; the five
deltas still hold 20 requirements and every task still names the requirement
it serves.

`openspec validate --strict` was run from this seat against the current
change bytes; its exit status and output are recorded immediately below this
entry as the seat's own check, distinct from the specify/design seats'
earlier runs. No Rust check was run here — this phase writes no code, and a
green suite is group 15's evidence, not this artifact's. No provider CLI was
probed, no adaptation bytes were committed and no evidence file was written
by this visit: those are 10.7's live half and 8.8/8.10's implementation work,
not the task-breakdown repair. Frozen contracts, policy, reference, fixtures,
production code, living specs, decision 0056, `adapters/dsh.json`,
`docs/guides/provider-adapters.md` and the sibling fire's files have no diff
from this visit; they carry the superseded 0.1.0-rc.6 truth until the
reopened 1.1/6.4/11.5/13.1 correct them on the next implement visit, in the
order this file now states.

`openspec validate 2026-09-09-226-session-resumption --strict` exit 0:
`Change '2026-09-09-226-session-resumption' is valid`.

## Design return — analyze drift A1–A5, 2026-09-11

The design seat of run `current-successor-operator-rulin-b83add73` answered
the analyze return at `095dd21` and revised this file with design D5, D6 and
D10, because three findings sat here:

- **A1**: 6.4's verify clause now expects DSH's measured version identity with
  no `wrapper_digest`. Checked 6.6's text names LaneTally alone for the
  unknown form and Codex, Claude and DSH for the measured-but-unqualified
  form. Its tick stands, because that case already loads DSH's pre-11.3 form.
- **A2**: 10.7's live half records the composite's raw inputs and checks the
  installed files with `sha256sum -c`. Its recording step appends the
  doctor-reported plugin component and canonical composite after 8.8's
  function lands, and 8.8's function is their only producer. The preamble's
  order names that step.
- **A3**: 8.8 opens with the optional `wrapper_digest` loader amendment and
  its `agents/tests.rs` cases.
- **A4**: 8.8 names the declared-digest and originating-root comparisons and
  `originating_wrapper_digest`. 8.10 names the missing, malformed, mismatched
  and originating-root cases, the closed unmeasured gate, the per-dialect
  vectors and cross-home/overlay equality.
- **A5**: the conventions line names official core 0.1.5-rc.1 at `183f08e9`.

No identifier was added, removed or renumbered. The record still reads 78
complete / 23 pending across 101 tasks. No task was ticked and none reopened.

## Current tasks visit — breakdown confirmation, 2026-09-11

This visit belongs to run `current-successor-operator-rulin-b83add73` and
follows design's `bed1a95` (the analyze return above). It adopts HEAD
`bed1a95` and the dated change `2026-09-09-226-session-resumption` and checks
the task breakdown against that settled design input before implementation
resumes.

Independent re-derivation confirms 10.3's and 8.8's cited facts rather than
merely trusting them: reading the installed `.forge/dsh-qualify/core`
0.1.5-rc.1 tree's `@deepseek-ai/dsh-session` package directly reproduces both
declaring/implementing SHA-256 values 10.3 and 8.8 cite for
`snapshotEvents(fromSeq?, toSeqExclusive?)` (`lib/types/index.d.ts`,
`ed327445b83ca8d699eb22991178362e4458172ba3c717a7a894f4907394fc37`) and its
`log.slice(fromSeq, toSeqExclusive)` implementation (`lib/index.js`,
`05e94f57d96e7979670a5b51024c8591572eb0051ce793613dbdec35cf2c47bf`), and
confirms `SessionEvent`'s `{ type, seq, time, data }` shape matches the
`{ seq, type, data }` fields `summarize`/`collectUsage`/`countTurns` already
read, so 8.8's one-expression delta needs no signature adaptation beyond the
accessor name.

Reviewing every task against design D5/D6/D10 finds no further drift: 1.1,
6.4, 8.8, 8.10, 9.6, 10.3, 10.7, 11.3, 11.5 and 13.1 already state the
forward-pinned core, the `extensions/dsh/plugin-cli-session/` location, the
single-expression adaptation, the `wrapper_digest`/`originating_wrapper_digest`
loader and composite-function ordering ahead of 10.7's recording step, and the
0.1.0-rc.6 pin only as bounded, dated history — matching A1–A5 above. The
checkbox count is unchanged at 78 complete / 23 pending across the same 101
tasks, no identifier was added, removed or renumbered, and every task still
names the requirement it serves. No production code, contract, decision,
adapter declaration, doctor output, guide or evidence file was read as
authoritative or edited by this visit; the independent re-derivation above
used only the read-only task-owned install already present under
`.forge/dsh-qualify/`, no install, fetch or model call ran, and the global
DSH pin, profiles and credentials were not touched.

`openspec validate 2026-09-09-226-session-resumption --strict` exit 0:
`Change '2026-09-09-226-session-resumption' is valid`. This breakdown is
ready for the next implement visit in the order this file states: 10.7's
live half, then 1.1 → 6.4 → 11.5 → 13.1, then 8.8, then 10.7's recording
step, then 8.10, 9.6 and 11.3.

## Design return — analyze drift B1–B5, 2026-09-11

The design seat of run `current-successor-operator-rulin-b83add73` answered
the second analyze return at `6829e57` and revised this file with design D6
and D10, because the findings sat in the design and in five tasks here:

- **B1**: 8.8(b) adds the `profile-bundle`, `profile-patch-reload` and
  `home-patch` lines and the bundle-resolution rule. 8.10 adds the
  composition drift and shadowing cases. 10.7 installs into the `headless`
  profile and records the added raw inputs.
- **B2**: 8.8(b) names D6's locators: the canonical `env node` executable, the
  hidden core lock, `node` on the child `PATH`, the two manifest members, the
  bounded pnpm reader with no YAML crate, and bytewise path order. 8.10 pins
  the path-order vector and the reader's refusals.
- **B3**: 11.3 records D6's end-to-end driver exchange against the retained
  home and ticks only beside the hermetic shims. 8.10 and 9.6 state that no
  test reads `.forge/`.
- **B4**: 6.4 repoints `evidence.interface` and verifies it.
- **B5**: 1.1 cites 0009 and records answer M's conditional admission.

No identifier was added, removed or renumbered. The record still reads 78
complete / 23 pending across 101 tasks. No task was ticked and none reopened.

## Current tasks visit — B1–B5 breakdown confirmation, 2026-09-12

This visit belongs to run `current-successor-operator-rulin-b83add73` and
follows design's `85b743b` (the second analyze-drift return above). It
adopts HEAD `85b743b` and the dated change `2026-09-09-226-session-resumption`
and checks the task breakdown against that settled design input before
implementation resumes, as the dialect's return rule requires.

The design seat's own return already edited 1.1, 6.4, 8.8, 8.10, 9.6, 10.7
and 11.3 in place to carry B1–B5 (`profile-bundle`/`profile-patch-reload`/
`home-patch` lines and the bundle-resolution rule; the `env node` executable,
hidden core lock, child-`PATH` `node`, two-member profile manifest, bounded
pnpm reader and bytewise plugin-path-order locators; the end-to-end driver
exchange recorded as 11.3's proof with the "no test reads `.forge/`"
statement carried into 8.10 and 9.6; 6.4's `evidence.interface` repoint; and
1.1's citation of decision 0009 and answer M). This visit re-reads all seven
tasks in full against design D6 section-by-section and finds no further
drift: every locator, line name and ordering rule D6 states for the
composite appears in 8.8(b) and 8.10's vectors; 10.7's raw-input list and
its "recording step runs after 8.8's function" gate match D6's qualification
section; 11.3's driver-exchange description matches D6's "end-to-end proof"
section exactly, including the reversible `cordis.patch.yml` drift case and
the unsafe-locator case.

Independent re-derivation, not just trust of the design's citations,
confirms three of B1/B2's facts against the task-owned `.forge/dsh-qualify/core`
0.1.5-rc.1 install, which this visit only read: `dsh-package-manifest`
`lib/types/types.d.ts` hashes to
`5d9bdce33121eb6831d2981f3e91f5a932f7db58cea96f722a913b3be26cef93` and
declares `export type ProfilePatchReload = 'live' | 'startup'`, confirming
B1's typed-boolean correction; `@deepseek-ai/dsh`
`lib/profile-boot-Dk-7KqJc.js` hashes to
`8b79b5c70281f23153ecc1828ba9b6e7b48364cc71409c90f12bde4cc4d7cde0` and its
`composeProfile` applies bundle layers, then the profile's own patch, then
the home-level patch (documented as outranking the profile layer), then
`--patch` overlays, then telemetry — confirming D6's layering order line for
line; and `dsh-app-boot` `lib/index.js` hashes to
`d8fdfe41996a4fefcff63af4982787924b0ae651f24bb1402f775b70202529bb`, whose
`resolveBundleDir` tries `packageDirFromAnchor` against the install anchor
then the profile directory, each call walking `require.resolve.paths` —
Node's own ancestor-`node_modules`-then-global-folder order — confirming B2's
bundle-resolution locator.

The checkbox count is unchanged at 78 complete / 23 pending across the same
101 tasks (recounted directly against the file), no identifier was added,
removed or renumbered, and every task still names the requirement it serves.
No production code, contract, decision, adapter declaration, doctor output,
guide or evidence file was read as authoritative or edited by this visit;
the independent re-derivation above used only the read-only task-owned
install already present under `.forge/dsh-qualify/`, no install, fetch or
model call ran, and the global DSH pin, profiles and credentials were not
touched.

`openspec validate 2026-09-09-226-session-resumption --strict` exit 0:
`Change '2026-09-09-226-session-resumption' is valid`. This breakdown is
ready for the next implement visit in the order this file states: 10.7's
live half, then 1.1 → 6.4 → 11.5 → 13.1, then 8.8, then 10.7's recording
step, then 8.10, 9.6 and 11.3.

## Current specify adoption — operator ruling 2026-09-12

Run `current-successor-operator-rulin-eef1e666` adopts `cf06034` and the whole
change. The two new findings were answered first in design D6, then in AS1
scenarios, then in this breakdown. Tasks 8.8(b) and 8.10 now name npm terminal
package extraction, same-entry version/integrity, complete-triple sorting and
deduplication, nested/scoped vectors and malformed-input refusals. Task 8.8
names the conditional extension's location and provenance; 8.10 tests its
identity, and 10.7 records its raw inputs only if the measured need arises.
The first finding costs reproducibility, not a demonstrated safety bypass;
8.8 remains the only digest producer. The second supplies ownership without
commissioning a speculative extension. A1–A5 and B1–B5 remain settled.

Task 10.5 now consumes the September 12 host evidence: startup is measured
true, resumed enforcement/root/accounting are still pending, and the installed
version must be re-read and reconciled. The September 10 host-blocker account
is preserved above solely as history. Task 11.4 states the operator's existing
LaneTally exception explicitly; no provider disposition is completed here.
The current provider instruction supersedes the former quota restriction.

All 101 task identifiers and every checkbox retain their adopted state:
**78 complete / 23 pending**. The five deltas now contain **20 requirements /
141 scenarios**. The two new AS1 scenarios map to 8.8/8.10 and, conditionally,
10.7/11.3; the amended Codex scenario maps to 10.5/11.1. Existing truth repairs,
provider proof and enablement, re-pins, local gates and readiness remain
pending. This artifact-only commit is preparation, not activation, archive or
delivery. The controller owns exact-head coverage, CI, integration,
publication and closure, with real results pending outside this artifact.


## Council design reconciliation — 2026-09-13 (Europe/Sofia)

Run `current-successor-operator-rulin-eef1e666` adopts `05fd566` and all work
at `cf06034`. The chief read both fresh positions and reconciled them in design
D10 after verifying the hidden lock and the loader's live patch behavior.
The two commissioned D6 findings remain answered; A1–A5 and B1–B5 stay settled.

D6 was revised first, then its existing AS1 scenarios, then 8.8/8.10 here.
Task 8.8 makes left-to-right scoped group consumption, the hidden-lock-only
source and the single `headless` profile explicit. Task 8.10 adds a synthetic
three-group key with distinct version/integrity, a scoped-parent/unscoped-child
case, malformed intermediate-group refusal and a no-root-lock-fallback case.
No test reads the measured `.forge/` lock and no real extension is authored
for a synthetic test. Task 8.8's doctor account follows D6's launch-time limit;
AS1/AS2 qualification and restriction proof remain mandatory, with no continuous
verification mechanism added. D6's measured count is 335 paths containing a
scope and 333 scoped terminal packages, not 335 scoped terminal packages.

All 101 identifiers and ticks are unchanged: **78 complete / 23 pending**.
All 20 requirements and 141 scenarios remain covered. These are design and
acceptance-work clarifications, not completed implementation or provider proof.
The Codex startup precondition remains true; this seat's own boxed version
attempt returned 127 and supplies no host version. Task 10.5 still re-reads
its exercised binary. The four truth repairs, provider proofs and enablement,
re-pins, local gates and readiness remain pending in their adopted order.

## Current tasks visit — breakdown readiness confirmation, 2026-09-13

This visit belongs to run `current-successor-operator-rulin-eef1e666` and
holds the tasks phase after design's own `6baca70` return above. It adopts
HEAD `6baca70` and the whole dated change, re-reads the required evidence set
(the September 12 controller sandbox note superseding the September 10 host
blocker, both codex proof records, both DSH pair-measurement records, the
upstream-discovery record, both probe scripts, `adapters/dsh.json` and
proposed 0056) against D6, D10, all five deltas and this file, and checks the
breakdown rather than repeating design's own reconciliation.

Both commissioned findings are confirmed answered in the design before their
task clauses: the npm key-to-triple rule (terminal group, verbatim entry
version/integrity, no `name`-field fallback, hidden-lock-only source, three-
group and malformed-intermediate-group vectors) is stated in D6 and carried
into 8.8(b)'s canonicalization steps and 8.10's pinned vectors and the AS1
"npm nested and scoped keys" scenario; the conditional Cordis extension's
repository location (`extensions/dsh/resume-policy/`), four-file set and
provenance rule are stated in D6 and carried into 8.8's return-to-pending
clause, 8.10's synthetic-extension cases and the AS1 "measured missing hook"
scenario. Tasks 1.1, 6.4, 8.8, 8.10, 9.6, 10.7, 11.3 and 11.5 all name the
forward-pinned latest core (`@deepseek-ai/dsh` 0.1.5-rc.1 at `183f08e9`) with
the repository-owned six-file adaptation, and none names the superseded
0.1.0-rc.6 pin as current; the reversal is preserved only as dated history
in each artifact's own append-only ledger. Task 10.5 carries the September 12
host evidence (startup/enforcement precondition measured true; resumed
root/accounting/re-imposition still unmeasured; the 0.148.0/0.153.4
reconciliation left as 10.5's own work) without upgrading it into a resume
proof, and 11.4 states the operator's LaneTally exception without completing
any provider disposition here.

Ordering is unchanged and consistent across the file: 10.7's live half runs
before 1.1/6.4/11.5/13.1; those four then land before 8.8; 8.8's function and
doctor line land before 10.7's recording step; 8.10, 9.6 and 11.3 follow;
11.1, 11.2, 10.5, 10.6, 10.8 and 11.4 proceed independently on their own
provider's evidence; 14.1/14.2 follow every code and declaration change; 15.x
gates and pre-archive readiness are last. No task's stated prerequisite
contradicts another's.

Recounted directly against this file: **78 complete / 23 pending** across the
same 101 identifiers, none added, removed or renumbered. The five deltas
parse at **20 requirements / 141 scenarios**, matching design and proposal's
own counts. `openspec validate 2026-09-09-226-session-resumption --strict`
exits 0: `Change '2026-09-09-226-session-resumption' is valid`. This seat's
box has no `cargo` on `PATH`, consistent with every predecessor visit's own
report from this box; no Rust, bundle, coverage or release result is claimed
here. No production code, contract, decision, adapter declaration, doctor
output, guide or evidence file was written by this visit; only this planning
artifact was read and appended to. No task was ticked and none reopened.

This breakdown names, for every pending task, the requirement it serves and
the concrete artifact it produces, and needs no repair before implementation
resumes. `upstream` is not reported: no design, proposal or spec defect was
found. The next visit should implement in the stated order, starting with
10.7's live half.


## Design return — F1 archive-dependent test ordering, 2026-09-13

Run `current-successor-operator-rulin-eef1e666` adopts `663c84a` and all work
at `cf06034`. The returned HIGH finding belongs first to design D9: full
pre-archive 15.3 required an assertion whose archive directory cannot exist
until every tracked task is complete. The chief repaired D9, reconciled both
fresh positions in D10 and encoded the answer in PM4's existing normal-rearchive
scenario before amending this breakdown.

Task 15.3 now checks the pre-archive suite with exactly the one named assertion
deferred, verifies one matching listed test and one filtered test across the
run, and records that scope truthfully. Tasks 15.6/15.7 keep the deferred
assertion and full archived suite explicitly pending. The existing post-task
action now runs the full unfiltered workspace suite, including both provenance
directions, after normal archive and before delivery. A failure blocks commit
and reopens the same dated change and owning task, invalidating readiness
before repair. Task 1.1 carries that same order into its already-pending
proposed-0056 repair; no decision is accepted or edited by this seat.

Robustness's bounded-exclusion evidence, vacuous-pass distinction and loud
failure behavior are adopted. Its source-comment request is rejected with
reasons in D10: PM4 retains the durable observable rule and D9/tasks own the
commission's scheduling particulars. Simplicity's unchanged assertions,
artifact-only repair and no-new-mechanism constraints are adopted. No source,
charter, dialect instruction, test annotation, duplicate archive or new task
is needed. The two D6 findings and A1–A5/B1–B5 stay settled; no provider
requirement or proof is narrowed.

All 101 identifiers and ticks are unchanged: **78 complete / 23 pending**.
All 20 requirements and 141 scenarios retain coverage; the amended PM4 scenario
maps to 15.3, 15.6, 15.7 and the existing archive action. This is planning
preparation only. The retained provenance binary reproduces the named failure
and one-pass/one-filter result, but it is not a fresh Cargo/workspace pass or
task completion. Provider proofs, enablement, re-pins, local gates, readiness,
the real archive and controller evidence remain pending.

Strict active OpenSpec validation, status, delta parsing, structural checks and
`git diff --check` pass. Cargo is absent, so all six commissioned Rust commands
failed to launch with `ENOENT`; no local gate is discharged. The chief's
run-local evidence is under `.forge/design-chief-eef1e666-f1/`. Full archived
validation and the unfiltered suite remain mandatory before delivery, with
exact-head host coverage and remote results still owned by the controller.


## Current tasks visit — post-F1 breakdown reconfirmation, 2026-09-13

This visit belongs to run `current-successor-operator-rulin-eef1e666` under
the operator's 2026-09-12 ruling that the change stays whole with Astra
holding triage/clarify/chief/analyze/review and Codex capacity restored; it
adopts HEAD `72c8332` and all work at `cf06034`. It re-read the full required
evidence set — the September 12 controller sandbox note (superseding the
September 10 host blocker), both codex proof records, both DSH pair-
measurement records, the upstream-discovery record, both probe scripts,
`adapters/dsh.json`, proposed 0056, design D5/D6/D9/D10/D11/D12/D13, all five
deltas' AS1–AS3 scenarios and this file including every dated return section —
against the commission's restated instructions, none of which supplies new
facts beyond what `72c8332` already encodes.

The design return already answered at `72c8332` (F1, archive-dependent test
ordering) is reflected in this file exactly: task 15.3 defers only the one
named provenance assertion with unique-match/single-filter evidence, 15.6/15.7
keep that assertion and the full archived suite pending, and the post-task
archive action runs the complete unfiltered suite before delivery. The two
2026-09-12 D6 findings remain fully carried: the npm key-to-triple rule
(terminal-group extraction, verbatim entry version/integrity, no `name`-field
fallback, hidden-lock-only source, the three-group and malformed-intermediate-
group vectors) in 8.8(b) and 8.10, and the conditional Cordis extension's
`extensions/dsh/resume-policy/` location, four-file set and provenance rule in
8.8's return-to-pending clause and 8.10's synthetic cases. Task 10.5 still
carries the September 12 host evidence as a measured precondition only, never
upgraded into a resume proof; 11.4 still states the operator's LaneTally
exception without completing a disposition. A1–A5 and B1–B5 remain settled
and are not reopened.

Recounted directly against this file: **78 complete / 23 pending** across the
same 101 identifiers, unchanged. `openspec validate 2026-09-09-226-session-
resumption --strict` exits 0: `Change '2026-09-09-226-session-resumption' is
valid`. `git status --short` was clean before this visit's edit. This seat's
box has no `cargo`, `npx` or `npm`; only the installed `openspec` binary was
run, consistent with prior visits' own reports. No production code, contract,
decision, adapter declaration, doctor output, guide or evidence file was
written by this visit; only this planning artifact was read and appended to.
No task was ticked and none reopened.

The restated provider instruction (Astra at xhigh for triage/clarify/chief/
analyze/review-chief, DeepSeek Flash implementing, Sonnet holding the task
planner and sonnet-side review/design) governs seat assignment for this
change's remaining phases and requires no edit to this artifact. This
breakdown names, for every pending task, the requirement it serves and the
concrete artifact it produces, and needs no repair before implementation
resumes. `upstream` is not reported: no design, proposal or spec defect was
found on this re-read. The next visit should implement in the stated order,
starting with 10.7's live half.

## Implement visit — 2026-09-13, run `current-successor-operator-rulin-eef1e666`

This seat began with the task the previous visit named: 10.7's live half. It
committed the repository-owned adaptation of `dsh-plugin-cli-session` 0.2.0 at
`0f487e74c81ed102c6899440d9f5d65e8e9eabda` under
`extensions/dsh/plugin-cli-session/` (the six published files; `lib/index.js`
line 253 changes `agent.session.events` to
`agent.session.snapshotEvents(firstSeq)`, and substituting the upstream
expression back reproduces upstream `lib/index.js` `a40b52…`), added the
sibling `extensions/dsh/PROVENANCE.md` and the
`docs/guides/repository-layout.md` row, built an adapted tarball, installed the
pair into the task-owned `.forge/dsh-qualify-015rc1` home's `headless` profile
after `sha256sum -c` against the provenance block, and ran the bounded live
cold/warm probe. Result: cold and warm both exit 0, emit the plugin result
envelope, continue the same session root (`session-9992bd7c-…`), recall the
nonce and report per-message usage, with the global pin unchanged. The raw
composite inputs and the exchange are recorded in
`.forge/tasks/dsh-pair-qualification-015rc1.json`. The prior 0.1.5-rc.1
`events is not iterable` failure was the unadapted plugin; the adaptation fixes
it. 10.7 stays unchecked because its recording step and the
exact-root/restriction/accounting proof remain.

1.1, 6.4, 11.5 and 13.1 are completed: proposed 0056 ruling 5 now selects core
0.1.5-rc.1 plus the repository-owned adaptation, cites 0009 for the
extension-boundary admission, ruling 6 cites 0043, ruling 7 cites 0034, ruling
8 cites 0006, ruling 10 cites 0041 and carries the archive-dependent deferral,
and a dated 2026-09-10 consequence records the reversed 0.1.0-rc.6 pin;
`adapters/dsh.json` sets `version`/`applies_to` to 0.1.5-rc.1 with no
`wrapper_digest`, points `evidence.interface` at the 015rc1 record and appends
the dated reversal limitation while preserving every earlier entry; the
provider-adapters guide row agrees; the content audit found no current
declaration still selecting 0.1.0-rc.6 (only the preserved superseded
limitation entry, the separate hands deferral and upstream plugin bytes).
`cargo test -p brokkr-cli --test decisions_index` and the runtime agents suite
pass.

8.8(a) is implemented: `ResumeIdentity::Measured` carries the optional
`wrapper_digest`; the loader admits it in the measured branch with seat record
v5's 64-lowercase-hex grammar, refuses a malformed member or one beside
`unknown` by name, carries it into the closed driver data, and a new
`agents/tests.rs` case proves absence, presence, carriage, refusal and the
adapter-content-digest move. 8.8(b)'s deterministic core is implemented and
tested in `crates/brokkr-protocol/src/adapters/composite.rs`: the
bytewise-path plugin component, the npm key-to-triple rule and the bounded
pnpm line reader (the full D6 normalization vectors, the three-group and
malformed-intermediate cases, hidden-lock-only, deduplication and pnpm
equivalence), and the canonical composite's fixed line order with its
bundle-order, reload, home-patch and extension movements. `brokkr-protocol`
gains the workspace `sha2` and `hex` dependencies, and `Cargo.lock` records
exactly those two edges. 8.8 stays unchecked: (b)'s seam/locator layer (core
package, Node runtime, profile manifest, bundle resolution and home patch),
(c) the doctor line and (d) the planner route with the private DSH target are
not implemented, and 8.10/9.6 are unwritten, so no value is wired to a home
yet.

Recount: **82 complete / 19 pending** across the same 101 identifiers. Cargo
was present in this seat's box, so the focused Rust checks ran; the six
commissioned full gates, provider proofs 10.5/10.6/10.8, the DSH recording
step, enablement 11.1–11.4, re-pins and readiness remain pending. The next
visit should implement 8.8(b)'s seam/locator layer, then 8.8(c)–(d), 8.10 and
9.6.


## Specify re-entry progress reconciliation — 2026-09-13, inherited head `b049224`

Adopt all committed work through `b049224`, including `cf06034`, without a
split or re-authoring. D10 first revalidates both commissioned D6 answers and
the retained council's F1 reconciliation; the AS1 and PM4 scenarios stay
unchanged. A1–A5/B1–B5 remain settled. Returned triage's engine classification
is correct. This entry supersedes only the earlier progress claims about the
current implementation and count, preserving those dated records as history.

The inherited loader in 8.8(a), Rust composite/locator implementation in
8.8(b), and doctor report in 8.8(c) exist and are retained. The 015rc1 evidence
record now includes the recording step produced by that Rust doctor entry.
Those facts do not finish their enclosing tasks: `eligible_offer` still
returns a string, the private DSH provider-ID/persistence-locator target and
planner route remain to be implemented, and 8.10/9.6 acceptance plus the live
built-driver exchange and full 10.7 proof are pending. D6's existing npm
version rule also still needs complete whitespace rejection in 8.8(b) and
space/tab/CR rejection coverage in 8.10; the inherited empty/NUL/LF checks are
insufficient. No separate canonicalization or digest is introduced.

Tasks 1.1, 6.4 and 11.5 remain checked after their valid forward-pin repairs.
Only 13.1 is reopened: its DSH row is repaired but its Codex/Claude rows deny
supplied partial observations. Correct that bounded evidence account before
continuing the remaining 8.8 work; do not rerun or undo completed preparatory
steps merely to recreate the earlier landing order. All four declarations
remain disabled and no provider proof or enablement is completed by this
visit. Codex's September 12 startup precondition remains true, while 10.5
still owes an exercised-binary version read and full resume proof. This seat's
own boxed `codex --version` returned 127, not a controller version observation
or a renewed host sandbox blocker.

Current recount: **81 complete / 20 pending** across 101 unchanged IDs. Pending:
8.8, 8.10, 9.6, 10.5–10.8, 11.1–11.4, 13.1, 14.1–14.2, 15.1–15.4 and
15.6–15.7. Preserve Astra xhigh at the commissioned gates/chiefs/judges, Flash
implementation and the specified Sonnet positions. Re-pins, local gates,
archive readiness, full post-archive tests and activation remain pending;
exact-head host coverage and remote delivery remain controller handoff work.


This visit's strict OpenSpec and structural checks pass; the command records
are under `.forge/specify-cff27e9f/` and the consolidated validation account is
in [proposal.md](proposal.md#specify-re-entry-validation--2026-09-13-inherited-head-b049224).
Cargo is absent, so none of the six commissioned Cargo gates could launch.
No delivery checkbox is ticked on these specification checks, and exact
coverage plus remote final-head evidence remain pending with the controller.

## Council re-entry reconciliation — 2026-09-13, inherited head `02e6771`

The chief adopted returned clarify's `clear` result and read both fresh council
positions in full. Design D10 pins and explicitly reconciles their claims.
F1, both commissioned D6 answers and A1–A5/B1–B5 remain settled. The new
robustness observation is confirmed in `composite.rs`: three containment
checks compare canonical candidates against a raw profile boundary. A
synthetic filesystem observation demonstrates a legitimate symlinked home's
false refusal, not false admission or a Rust/provider pass.

D6 now requires one canonical profile containment boundary and canonical
candidate comparisons, without changing the loader's original lookup anchor
or first-hit order. The existing AS1 ordinary-runtime scenario follows, then
8.8(b)'s correction and 8.10's synthetic alias/escape/error cases here. The
Node observation shows why simply reusing a canonicalized directory as the
lookup anchor is insufficient: its ancestor search paths differ. No digest
line, npm grammar, new task or speculative extension follows; the one Rust
producer and existing synthetic-home suite remain the implementation owners.

All 101 IDs and ticks remain **81 complete / 20 pending**. Pending work remains
8.8, 8.10, 9.6, 10.5–10.8, 11.1–11.4, 13.1, 14.1–14.2, 15.1–15.4 and
15.6–15.7. Preserve the inherited loader/composite/doctor work and completed
truth repairs. Correct 13.1's partial-evidence prose before remaining 8.8 work;
finish the existing npm whitespace and new containment acceptance within
8.8/8.10 before claiming those tasks complete. The September 12 Codex startup
precondition remains true, and 10.5 still owes its own binary-version read and
full resume proof. No provider is enabled, proposed 0056 stays proposed, and
the full archived suite and controller evidence on the final commit remain mandatory.

This is dependent-artifact preparation, not implementation or activation.
[This visit's design validation](design.md#council-re-entry-validation--2026-09-13-inherited-head-02e6771)
records the actual checks and their limits. No tick is awarded for them.

## Current tasks visit — post-containment breakdown confirmation, 2026-09-13

This visit belongs to run `current-successor-operator-rulin-eef1e666` and
holds the tasks phase after design's own `ab3012a` council re-entry return
above, which reconciled the canonical DSH profile containment finding (a
symlinked profile ancestor producing a false refusal against a raw boundary).
It adopts HEAD `ab3012a` and the whole dated change, restated in this run's
commission as CURRENT SUCCESSOR with the change kept whole under Astra/
DeepSeek Flash/Sonnet seat assignment, and re-reads the commissioned evidence
set — the two D6 findings, A1–A5, B1–B5, F1, the September 12 controller
sandbox note, both codex proof records, the DSH pair-measurement and
upstream-discovery records, `adapters/dsh.json` and proposed 0056 — against
D6/D9/D10/D11 and this file.

The containment fix is confirmed already carried into this breakdown, not
merely into design: task 8.8(b) states canonicalizing the complete profile
directory once for containment while retaining the original raw lookup
anchor and order, comparing each first-hit canonical bundle directory against
the canonical core root or canonical profile boundary, refusing an
uncanonicalizable boundary or candidate, and it names the inherited raw
comparison's symlinked-home false refusal as the correction still pending
under this task. Task 8.10 adds the matching synthetic acceptance and refusal
vectors: a symlinked ancestor resolving the same contained bundles, an
unresolvable boundary, a symlink target outside the allowed canonical roots,
a near-prefix sibling (`headless-extra`), and an outside first hit with a
later inside candidate, each staying unreadable with no fallback. Both tasks
still cite `safety / AS1`, matching the amended AS1 scenario in
`specs/adapter-resume-safety/spec.md`. The two 2026-09-12 D6 findings (npm
key-to-triple rule, conditional Cordis extension location/provenance) and
A1–A5/B1–B5 remain settled and unre-derived; no new finding reopens them.

Recounted directly against this file: **81 complete / 20 pending** across the
same 101 identifiers — 8.8, 8.10, 9.6, 10.5–10.8, 11.1–11.4, 13.1, 14.1–14.2,
15.1–15.4 and 15.6–15.7 — none added, removed or renumbered. The five deltas
parse at **20 requirements / 141 scenarios** (`grep` count over
`specs/*/spec.md` matches design's own recount exactly).
`openspec validate 2026-09-09-226-session-resumption --strict` exits 0:
`Change '2026-09-09-226-session-resumption' is valid`. `git diff --check`
reports no whitespace errors. This seat's box has no `cargo`, `npm`, `npx` or
`codex` on `PATH`, consistent with every predecessor visit's own report from
this box; only the installed `openspec` binary was run, and no production
code, contract, decision, adapter declaration, doctor output, guide or
evidence file was written by this visit — only this planning artifact was
read and appended to. No task was ticked and none reopened.

Ordering is unchanged and still consistent: 10.7's live half already landed
(recorded above at the `current-successor-operator-rulin-eef1e666` implement
visit); the remaining order is 1.1/6.4/11.5/13.1's already-complete truth
repair (13.1 reopened for its Codex/Claude row correction per the specify
re-entry visit above), then 8.8's remaining seam/locator/doctor/planner work
including this visit's confirmed containment clause, then 8.10 and 9.6, then
10.5–10.8's provider proofs, 11.1–11.4's evidence-gated enablement (LaneTally
per 11.4's sole exception), 14.1–14.2's re-pins, and 15.x's gates and
pre-archive readiness last.

This breakdown names, for every pending task, the requirement it serves and
the concrete artifact it produces, and needs no repair before implementation
resumes. `upstream` is not reported: no design, proposal or spec defect was
found on this re-read; design's own containment fix at `ab3012a` was already
propagated into this file's task text and needed no further edit here. The
next visit should resume implementation at 13.1's reopened Codex/Claude row,
then 8.8's remaining work in the stated order.

## Implement visit — 2026-09-13, run `current-successor-operator-rulin-eef1e666` (continuation)

Adopted HEAD `1aa120b` and the whole change. This visit completed **13.1**
and advanced **8.8(b)/8.10**; nothing else was ticked.

**13.1 (ticked).** `docs/guides/provider-adapters.md` now replaces the two
false claims: the codex row cites the September 10 bounded cold/warm probe on
the installed **0.153.4** (same thread id re-announced, cold nonce recalled,
current-invocation-only usage) while keeping resumed restriction enforcement
unmeasured, and the claude row cites the controller captures and the
September 10 same-root probe (root continuity, expired Read grant renewed,
new Read grant operating, Write and MCP tools removed, init admission, raw
id/tool ids, reconciled accounting) while keeping the installed **2.1.270**
and the complete boundary/precedence unmeasured. The completed DSH row is
preserved verbatim. To satisfy 13.1's line-by-line declaration agreement,
`adapters/codex.json` and `adapters/claude.json` limitations/reason text were
corrected to the same account, and `adapters/dsh.json` line 69's stale
sentence — which still said the selected pair "pins ... 0.1.0-rc.6" — was
replaced with the repository-owned `snapshotEvents(firstSeq)` adaptation
truth; 0056 and the guide already carry the reversal. The declaration edits
moved the witness and compose pins, which were re-recorded from the tests'
own reported pairs: `WITNESSES` (9 entries, only `recipes/research-dsh`
unmoved) and `UNCOMPOSED` (4 entries). That re-pin is preparable work under
14.1, which stays unchecked because groups 8–13 still change.

**8.8(b)/8.10 (advanced, still unchecked).** `crates/brokkr-protocol/src/adapters/composite.rs`
now rejects an npm `version` carrying any whitespace, not only empty/NUL/LF
(D6's complete rejection), and canonicalizes the complete
`<home>/profiles/headless` directory once as the containment boundary while
retaining the raw lookup anchor: every first-hit canonical candidate is
compared against the canonical core root or canonical profile boundary, the
plugin and conditional extension must lie inside that same canonical profile,
and an uncanonicalizable boundary or candidate is unreadable. New
`composite/tests.rs` vectors cover space/tab/CR/NUL versions, a symlinked
home ancestor yielding equal plugin/composite values, a near-prefix sibling
compared by components not string prefix, a broken-symlink boundary, and an
outside first hit that is never skipped for a later inside candidate. The 17
focused composite tests pass. 8.8 stays unchecked because (d) the planner
route and the private DSH provider-ID/persistence-locator target, the
remaining seam/doctor acceptance and the built-driver exchange are not
implemented; 8.10 and 9.6 remain unwritten.

Recount: **82 complete / 19 pending** across the same 101 identifiers.
Pending: 8.8, 8.10, 9.6, 10.5–10.8, 11.1–11.4, 14.1–14.2, 15.1–15.4 and
15.6–15.7. `cargo fmt --all -- --check` is clean and the complete
`cargo test --workspace --all-features --locked` passes except three
`brokkr-cli` tests that spawn `git` and fail only under this harness's
malformed `GIT_CONFIG_COUNT` environment; with that variable unset the same
287-test binary is green. No provider proof, enablement, coverage, bundle,
release or archive step is claimed.

## Implement visit — 2026-09-13, run `current-successor-operator-rulin-eef1e666` (engine target plumbing)

Adopted HEAD `0ae143c` and the whole change. This visit implemented the
engine half of 8.8(d) and left every checkbox unchanged: **82 complete /
19 pending** across the same 101 identifiers.

`crates/brokkr-runtime/src/engine/resume.rs` gains `ResumeTarget` (the
provider ID plus the persistence locator read off the SAME confirmed
checkpoint) and `OriginatingRoot` (the offered row's `harness_version`, its
optional `wrapper_digest` and its locator). `eligible_offer` now returns
the owned target: the legacy path pairs its session with itself as the
locator, and a stamped row with no transcript reference still offers its
provider ID with no locator to rejoin. `originating_root` replaces
`originating_harness_version` and reads version, digest and locator from
one row rather than per field from whichever checkpoint happens to be
newest. `start_context` now carries `originating_wrapper_digest` and
`owned_target` beside the assessment.

`crates/brokkr-runtime/src/engine.rs`: `SitePlan.offer`/`MemberRun.offer`
are the owned target and `SitePlan.originating` is the `OriginatingRoot`.
Only `provider_id` crosses `run_attempt_resuming`'s `Body::Resume`; the
locator and originating digest ride the private `Start.input` context. No
wire, contract, store or declaration changes.

Tests: `resume::tests::the_private_context_carries_the_owned_target_and_originating_digest`
and the extended stamped-row case assert same-row locator pairing, the
optional wrapper digest and the locatorless decline. Task 8.8 stays
unchecked: the DSH adapter's `--new`/`--session` planner, its stream-json
confirmation and the atomic `root_session`+`transcript` launch row are not
implemented, and 8.10/9.6 remain unwritten.

**A concrete conflict the DSH planner must resolve.** Task 8.8(d) and
design D6 (lines 1007–1011) require the planner to reject a user
`--patch` on both paths, but `recipes/research-dsh/bundle.json` is the one
bundle whose dsh fetch grant IS a `--patch` overlay, pinned by
`crates/brokkr-runtime/tests/roster.rs`
`the_dsh_fetch_overlay_is_the_research_lanes_alone_and_its_role_is_the_charter`
and by the witness digests. Rejecting `--patch` outright would disable
that grant, so 8.8 must either consume and fold the seat patch into the
Rust-owned overlay or migrate the grant to a profile/bundle mechanism,
then reconcile the roster test and re-record the moved witness digests.
Recorded here because it is new concrete evidence, not a reopening of
A1–A5/B1–B5.

Local checks on this tree: `cargo fmt --all -- --check` clean; `cargo
clippy --workspace --all-targets --all-features --locked -- -D warnings`
clean; `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
exit 0. The complete workspace suite has exactly one failure,
`machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_evidence`,
which fails identically at HEAD with these edits stashed — a pre-existing
environment/harness failure, not this visit's. No provider proof,
enablement, coverage, release, archive or controller step is claimed.

## Current tasks visit — pass A route-overlay breakdown reconciliation, 2026-09-14

This visit belongs to run `current-successor-issue-226-pass-972adad6` (tasks
phase) under `.forge/tasks/current-successor-226-passes.md`'s scheduled
passes A–D and nothing else. It adopts HEAD `77574b06950405039f9036dd8b718a61df517218`,
including this run's design return that reconciled D5/D6/D10/D11 and proposed
0056 ruling 6 with the route-overlay rule answers Q, R and S already landed in
AS3 (specify commits `2f1b97b`, `ca2d145`, `7ac9852`; design commit `77574b0`).
That design return explicitly left tasks 8.8(d) and 8.10 stale against the
reconciled specification and named them as this phase's own work, together
with the `roster.rs` doc-comment correction. Read in full: the pass framing,
`.forge/tasks/controller-codex-sandbox-host-2026-09-12.md`,
`dsh-pair-qualification-015rc1.json`, `dsh-pair-incompatibility.json`,
`controller-dsh-upstream-discovery.json`, `adapters/dsh.json`, proposed 0056
including its dated 2026-09-14 route-overlay note, and the whole change
(proposal answers Q–S, all five deltas including the amended AS1–AS3
scenarios, design D5/D6/D10/D11 and this file).

**Repaired in this visit.** Task 8.8(d) now states the route overlay's four
mechanisms as build steps in the same order D6 specifies them — bind the
seat's single `--patch` to a `files` member of the compiled leaf layer at
both `start_context` call sites; read the file once and require SHA-256
equality before any shape check; apply AS3's closed data-only line reader and
closed six-field set with the `apiKeyEnv`/`baseURL` value grammars; fold the
validated rows ahead of the transcript/model/settings rows in
`dsh_seat_overlay_in` unconditionally on the cold, resume and closed/
`unmeasured`-gate paths — and now excepts exactly that one bound overlay from
the prior blanket `--patch` rejection sentence, matching AS2's and AS3's
amended scenarios. It also assigns the `roster.rs` doc-comment correction
(`the_dsh_fetch_overlay_is_the_research_lanes_alone_and_its_role_is_the_charter`
still calls the overlay "the fetch grant") to this task, without moving that
test's assertion, `bundle.json`, `research-web.yml` or the research-dsh
witness digest. Task 8.10 gains the deterministic route-overlay case list
answers Q–S require: the shipped overlay as the positive vector under cold,
offered and `unmeasured`; refusals for a second/bare `--patch`, an unsafe or
unbound/drifted path, a foreign row/provider/model pin, each field-set or
`apiKeyEnv`/`baseURL` grammar breach and executable syntax at any depth in
either representation — each naming a depth, field or URL part and never a
value, and proving the binding/digest check precedes the shape check. No
other task text moved: 9.6's accounting folds are unaffected because the
composite and its folds exclude the overlay's rows by construction, and D7/D8
(passes C and D) needed no new design decision per this run's design return.

Passes B, C and D need no further breakdown repair beyond this reconciliation:
8.8's remaining seam/planner work (the private DSH provider-ID/persistence-
locator target, the stream-json cold/warm construction and now the reconciled
route-overlay fold), 8.10's and 9.6's acceptance suites, and the launch/
accounting mechanics D7/D8 already specify, are all named with their concrete
artifacts and requirement citations in the existing task text. No task's
stated prerequisite contradicts another's, and the order — 8.8's remaining
work, then 8.10, then 9.6 — is unchanged.

Recounted directly against this file: **82 complete / 19 pending** across the
same 101 identifiers, none added, removed or renumbered. The five deltas
parse at **20 requirements / 150 scenarios**, matching this run's design
return's own recount (the specify phase's Q–S scenarios raised the AS1–AS3
count from 141). `openspec validate 2026-09-09-226-session-resumption
--strict` exits 0: `Change '2026-09-09-226-session-resumption' is valid`.
`git diff --check` reports no whitespace errors. This seat's box has no
`cargo` on `PATH`, consistent with every predecessor visit's own report from
this box; only the installed `openspec` binary was run. No production code,
contract, decision, adapter declaration, doctor output, guide or evidence
file was written by this visit — only this planning artifact was edited. No
task was ticked and none reopened.

This breakdown names, for every pending task, the requirement it serves and
the concrete artifact it produces, and needs no further repair before
implementation resumes. `upstream` is not reported: the design and
specification already own and coherently state the route-overlay rule: the
staleness was this file's own text, now reconciled. Passes E through K remain
out of this run's scope and are untouched. The next visit should implement
pass B starting from 8.8's remaining planner work, in the order this file
states.

## Current tasks return — D7 confirmation-exchange coverage repair, 2026-09-14

This visit belongs to run `current-successor-issue-226-pass-972adad6` (tasks
phase), returned from that run's own analyze visit
(transcript `a9ccdc00-0617-4b6f-b6e6-a532fec188a8`), which held pass A
complete but raised one MEDIUM coverage-gap finding against this file at HEAD
`77fcacbb41f6174cab04af5a6bb2d3246362b91e`: D10's own sitting ruling that
pass C is "D6's confirmation paragraph and D7" and that "their deterministic
cases belong to 8.10 and 9.6" was not carried into the breakdown. 8.8(d)
named the locator-side check (bounded relative form, depth-zero header, exact
ID) but never D6's later paragraph requiring the pinned plugin's post-`await
agents.resume` init event, same-root nonce continuity and new sequence
activity before the launch hold releases, and never stated that a bare
request-derived `session_id` is insufficient alone. 8.10's DSH list covered
argv, selectors, drift, digest and the new route overlay but carried no case
for that confirmation exchange itself, and 9.6 covers only D3's offer-time
storage decline (missing/truncated/ambiguous/escaping storage starts cold),
not D7's post-spawn missing-confirmation or different-root outcome. The
generic 7.9/9.7 cases were checked before DSH existed as a launch-confirming
shape and do not exercise a DSH child-process init event, so AS4's
"unstructured DSH error" and "different session" scenarios and LE1's
"requesting resume proves nothing alone" and LE3's "DSH cannot classify a
refusal" scenarios had no DSH-owned deterministic task case.

**Repaired in this visit, tasks-only, no tick.** 8.8(d) gains one paragraph,
placed after the version/composite gate paragraph and before the existing
forbidden-flag sentence, stating D6's confirmation rule as a build
obligation: reject the plugin's request-derived `session_id` alone; require
the retained depth-zero header for the offered ID, the pinned plugin's
post-`await agents.resume` init event from that root, same-root nonce
continuity and new sequence activity before the launch hold releases; and a
missing or different root, whether followed by a clean exit or a delivered
result file, stays failed or indeterminate under D7 and publishes no
`root_session`, `transcript` locator or launch row. 8.10 gains one
enumeration, placed after the route-overlay refusal cases and before the
canonical-composite byte-form vectors, naming six deterministic DSH
stream-json exchange cases built from a captured synthetic child transcript
with no installed provider: the confirming init-then-work positive vector;
child exit before any init (indeterminate/failed, no launch row, no
replacement); an init naming a different root followed by a clean exit, and
the same mismatch followed by a valid delivered result file (each
failed/indeterminate, no accepted success); a nonzero exit with stderr prose
but no measured machine-readable rejection shape (no automatic cold
replacement, AS4/LE3); cancellation or deadline expiry while the hold is open
(no fabricated launch, no replacement); and the one DSH-specific local-decline
path permitting exactly one safe cold launch. 8.10's and 8.8's closing
requirement citations gain `safety / AS4`, `evidence / LE1` and
`evidence / LE3` to match. 9.6 is left untouched: its store fixtures prove
D3's pre-spawn decline, a different artifact than D7's post-spawn confirmation
exchange that 8.10 now owns, and reopening it would exceed this finding's
demonstrated gap.

Recounted directly against this file: **82 complete / 19 pending**, the same
101 identifiers, none added, removed, renumbered or ticked.
`openspec validate 2026-09-09-226-session-resumption --strict` exits 0:
`Change '2026-09-09-226-session-resumption' is valid`. `git diff --check`
reports no whitespace errors. This seat's box has no `cargo` on `PATH`,
consistent with every predecessor tasks visit's own report; only the
installed `openspec` binary and `git` were run. No production code, contract,
decision, adapter declaration, doctor output, guide or evidence file was
written by this visit — only this planning artifact was edited.

This breakdown now carries every deterministic case D6's confirmation
paragraph and D7 require, under existing identifiers 8.8 and 8.10, and needs
no further repair before implementation resumes. `upstream` is not reported:
the defect was this file's own missing coverage, not a defect in design or
specification, and it is now closed. Passes B, C and D (this run's scheduled
scope) and passes E through K (out of scope) are otherwise unchanged from the
prior visit's reconciliation. The next visit should implement pass B starting
from 8.8's remaining planner work, in the order this file states.

## Current tasks return — nonce/run-time confirmation conflation repair, 2026-09-14

This visit belongs to run `current-successor-issue-226-pass-972adad6` (tasks
phase), returned from that run's own analyze visit (transcript
`2c99aeba-c098-4212-9e8a-24b6e7a94d22`), which held pass A complete but raised
one MEDIUM finding against this file at HEAD `d14c97b`: the previous
2026-09-14 repair (above) carried D6's probe-only "same-root nonce
continuity" phrase into 8.8(d)'s run-time launch-hold paragraph, into 8.10's
positive stream-json vector, and into 9.6's cold `--new` launch description,
each stating or implying that a nonce is confirmed at run time. Design D6
(`design.md:1171`) assigns nonce continuity to **10.7's probe** alongside "no
fresh sibling root/session" and "new sequence activity," and the operator's
pass C framing for this run separately names only "the depth-zero stored
header and the post-resume init" as run-time confirmation. A nonce is the
probe's model-recall device: proving it at run time would require planting a
token in the prompt and reading the model's recall on every invocation, a
per-attempt model experiment D5 forbids (`design.md:468`) and AS1 rejects as
an unnecessary model experiment, and it would put engine content in the
prompt that ruling 4 forbids. As written, 8.8(d) could not be implemented
without violating D5, and 9.6's "nonce" word described a live-model fact
inside what its own sentence calls a deterministic retained-store fixture.

**Repaired in this visit, tasks-only, no tick.** 8.8(d)'s launch-hold
paragraph now requires only the retained depth-zero header, the pinned
plugin's post-`await agents.resume` init event, no fresh sibling root/session
in the retained store, and new sequence activity past the recorded
`firstSeq` — all four mechanically observable from the retained root
directory and its log tail — and states explicitly that same-root nonce
continuity is 10.7's probe-only model-recall device, never planted or read
at run time. 8.10's positive stream-json vector drops "same-root nonce
continuity" from the run-time confirming event and states the same two
mechanical facts (no fresh sibling root/session, new sequence activity past
`firstSeq`) instead. 9.6's cold `--new` launch description drops "and nonce"
from what a deterministic retained-store fixture confirms, leaving "a fresh
root." Task 10.7 (`tasks.md:1316`, out of this run's scope) is untouched: it
already scopes same-root nonce continuity to its own live probe alongside the
header and init event, which is correct and needed no repair. Design D6 is
untouched: its own sentence already assigns nonce continuity to "the probe,"
and the conflation was this file's wording, not the design's.

Recounted directly against this file: **82 complete / 19 pending**, the same
101 identifiers, none added, removed, renumbered or ticked.
`openspec validate 2026-09-09-226-session-resumption --strict` exits 0:
`Change '2026-09-09-226-session-resumption' is valid`. `git diff --check`
reports no whitespace errors. No production code, contract, decision,
adapter declaration, doctor output, guide or evidence file was written by
this visit — only this planning artifact was edited.

This breakdown now states D6's run-time/probe nonce split coherently under
existing identifiers 8.8, 8.10 and 9.6, and needs no further repair before
implementation resumes. `upstream` is not reported: the defect was this
file's own wording, not a defect in design or specification, and it is now
closed. Passes B, C and D (this run's scheduled scope) and passes E through K
(out of scope) are otherwise unchanged from the prior visit's reconciliation.
The next visit should implement pass B starting from 8.8's remaining planner
work, in the order this file states.

## Current specify return — engine-side route-overlay verification ownership, 2026-09-14

This visit belongs to run `current-successor-issue-226-pass-c2d8f6ec`
(specify phase, the run's only seat). Its predecessor,
`current-successor-issue-226-pass-972adad6`, parked at
`ANALYZE-DRIFT-EXHAUSTED` after its final analyze visit raised one MEDIUM
coverage-gap finding against this file at HEAD `2e157c5`, owned by tasks.
This visit adopts all committed work at that head, including `d14c97b` and
`2e157c5`, under the unchanged `2026-09-09-226-session-resumption`
identifier; the predecessor's planning stands, its recount (20 requirements /
150 scenarios, 82 complete / 19 pending across 101 identifiers) is inherited,
and neither the framing, the installed-writer reads nor the qualification
reads are repeated. Read in full, through the workspace hands: the pass
framing and its A–D supplement, the September 12 controller Codex note, the
015rc1 qualification, incompatibility and upstream-discovery records,
`adapters/dsh.json`, proposed 0056, the proposal, all five deltas, design and
this file, plus `engine/resume.rs::start_context` with its test module, both
`resume_context` call sites in `engine.rs` and the fixtures of
`engine/resume_tests.rs`.

**The finding.** D5/D6 and AS3 place the route-overlay binding in the engine
at both `start_context` call sites, and AS3's scenario "An unbound or drifted
route overlay is refused" makes the engine-side outcome observable per case:
a same-shaped nonmember, a working-directory shadow, an ancestor-layer file,
the `./` expansion, a member whose bytes changed since compilation, and the
carried digest being the manifest's rather than a hash of the resolved file.
But 8.10 assigned every route-overlay case to `adapters/tests.rs`, and
8.8(d)'s verify clause named no runtime-crate suite. The adapter receives
only argv, a working directory and the private context: its suite can prove
an absent binding and a digest mismatch, but it cannot distinguish a
nonmember from a shadow from an ancestor-layer file, cannot show that the
engine withholds a binding for each, and cannot show that the engine carries
the manifest's digest rather than a hash of the file it resolved.

**Repaired in this visit, tasks-only, no tick.** 8.8(d) now states where the
binding lives and what it carries: an input of `start_context` in
`crates/brokkr-runtime/src/engine/resume.rs`, computed in
`crates/brokkr-runtime/src/engine.rs` at the single-site call site in
`run_driver` and at the panel-member call site where each `MemberRun` is
composed, from the compiled manifest's `files` entry and never from a hash of
the resolved file, so the adapter's digest comparison is what refuses edited
bytes. 8.8's verify clause gains the runtime crate's suites beside the
adapter, loader, doctor and committed-bytes cases: the unit case in
`engine/resume.rs`'s test module, in the pattern of
`the_private_context_carries_the_owned_target_and_originating_digest`, that
`start_context` carries a supplied binding exactly and carries none when none
is supplied; and the engine integration cases in
`crates/brokkr-runtime/src/engine/resume_tests.rs`, read off the `Start.input`
that suite's logging drivers actually received at both call sites. 8.10's
heading names both suites, and its route-overlay list is split by the crate
that can observe each case. The runtime suite owns, at both call sites: (i)
a valid leaf-manifest member carrying the actual argv value and that member's
compiled digest, for the positive case on cold, offered and `unmeasured`
starts alike; (ii) a same-shaped nonmember, a working-directory shadow, an
ancestor-layer file and the `./` expansion each receiving no binding; and
(iii) a member changed after compilation travelling with the manifest's
digest, not a hash of the resolved file. The adapter suite keeps every case
it can observe — the positive fold, arity and path refusals, an absent
binding beside a present `--patch`, a binding without `--patch`, a
disagreeing binding and a bound digest the read bytes do not hash to, the
row, provider, field-set, `apiKeyEnv`, `baseURL` and executable-syntax
refusals, and the binding-before-shape ordering — with the five engine-side
outcomes proven end to end by the two suites together and never by adapter
cases in place of engine ones. AS3, D5, D6, proposed 0056 and the requirement
citations are unchanged: the finding adds verification ownership to this
file, not a requirement or a design change. No case is dropped, no adapter
case stands in for an engine one, and no task moves between identifiers.

Recounted directly against this file: **82 complete / 19 pending**, the same
101 identifiers, none added, removed, renumbered or ticked. The five deltas
still parse at **20 requirements / 150 scenarios**.
`openspec validate 2026-09-09-226-session-resumption --strict` exits 0:
`Change '2026-09-09-226-session-resumption' is valid`. `git diff --check`
reports no whitespace errors. Cargo is absent from this seat's box, as every
predecessor planning visit in this feature recorded from this box, so format,
clippy, the workspace tests, both bundle compiles and the release build could
not launch here; they remain the implementing pass's obligation before its
final commit, and the engine cases named above are pass B's to write and run.
No production code, contract, decision, adapter declaration, doctor output,
guide, recipe, living specification or evidence file was written by this
visit — only this planning artifact and the proposal's dated visit record.
Evidence is under `.forge/specify-c2d8f6ec/`.

`upstream` is not reported: the specification and design already state the
engine-side binding and its observable outcomes; the defect was this file's
verification ownership, and it is now closed. Passes B, C and D (this run's
scheduled scope) are otherwise unchanged from the predecessor's
reconciliation; passes E through K remain unscheduled and untouched. The next
visit should implement pass B starting from 8.8's remaining planner work, in
the order this file states, writing the runtime engine cases beside the
binding they prove.

## Current tasks visit — 8.8(d) verb normalization, 2026-09-14

This visit belongs to run `current-successor-issue-226-pass-c2d8f6ec` (tasks
phase, the run's only seat). It adopts all committed work at `1064dca` on
this branch, including the predecessor's `d14c97b`/`2e157c5` and this run's
own `b1caf37` (specify) and `1064dca` (design), under the unchanged
`2026-09-09-226-session-resumption` identifier. The predecessor,
`current-successor-issue-226-pass-972adad6`, parked at
`ANALYZE-DRIFT-EXHAUSTED` with one MEDIUM coverage-gap finding owned by
tasks; `b1caf37` already answered its substance by naming the runtime engine
suite and its cases for the route-overlay binding at both `start_context`
call sites, and `1064dca` recorded that ownership in design D6/D10/D11/Risks
and ordered this visit's one residual: 8.8(d)'s engine-half verb still read
as an engine-side refusal, which AS3/D6 do not specify — the engine's outcome
for a non-bindable value is withholding the `route_overlay` member, never a
start failure, with the adapter's existing pre-work failure to start on
seeing a `--patch` beside an absent binding as the single refusal path.

Applied exactly the edit design ordered, at 8.8(d) only, under the existing
identifier, without a tick: "refusing an absolute path, `..` or symlink
escape and any value the leaf layer does not carry as its own `files`
member" becomes "withholding the binding for" those same values, and "the
bundle-relative `./` spelling is refused as absolute" becomes "the
bundle-relative `./` spelling expands to an absolute path and receives no
binding." No other byte in 8.8(d) or 8.10 moved; 8.10's crate-split case list
and 8.8(d)'s verify clause were already correct from `b1caf37` and are
unchanged.

Recounted directly against this file: **82 complete / 19 pending**, the same
101 identifiers, none added, removed, renumbered or ticked. `openspec
validate 2026-09-09-226-session-resumption --strict --no-interactive` exits
0: `Change '2026-09-09-226-session-resumption' is valid`. `git diff --check`
reports no whitespace errors. `cargo`/`rustc` are absent from this seat's
box, as every planning visit in this feature has recorded from this box; no
Rust, bundle or release-build result is claimed here, and none is owed by a
tasks-only verb normalization. No production code, contract, decision,
adapter declaration, doctor output, guide, recipe, living specification or
evidence file was written by this visit.

`upstream` is not reported: the specification and design already state the
engine-side binding and its uniform withholding outcome, and `b1caf37`
already placed the engine cases; the only residual was this file's own verb,
now normalized. Passes B, C and D (this run's scheduled scope) are otherwise
unchanged from the prior reconciliation; passes E through K remain
unscheduled and untouched. The next visit should implement pass B starting
from 8.8's remaining planner work, in the order this file states, writing the
runtime engine cases beside the binding they prove.


## Current tasks visit — Pass B breakdown reconciliation, 2026-09-14

Run `current-successor-issue-226-pass-838309ce`, tasks phase and its only seat,
adopts `31cd6fa`, specify `cf35cc7` and design `1a04751`. Read the full pass
framing, current B intake, predecessor's final journal result
`721fc65d-da69-4b95-9039-6f6469fadd91`, DSH declaration, proposed 0056,
proposal, five deltas, design D5/D6 and the current D10 reconciliation/open
questions, tasks 8.8/8.10/9.6 and finalization, and relevant Rust planner,
storage, private-context and test code through workspace hands. Read the
repository's OpenSpec tasks/return instructions and the rendered tasks
instructions. No `returned_from` finding is present in this visit; the current
design supplies the bounded implementation details under existing requirements.

Only the current framing, 8.8 and 8.10 descriptions and this progress entry
change. The breakdown now orders private originating-home propagation before
its consumer, then control validation and fixed diagnostics, delivered route
validation, gate/identity comparisons, bounded owned-storage reads, and exact
cold/warm command and overlay construction. Each step names its requirement
and corresponding verification. 8.10 assigns same-checkpoint and actual-Start
home cases to the runtime suites at both callers and planner outcomes to the
protocol suite. It requires recorded zero-call evidence only where ordering
prevents observations, retains legitimate observations before origin/storage
declines, and distinguishes route digest-before-shape refusal from a
matching-digest grammar refusal. D6's character bound and canonical homes,
selected-file containment and finite read budgets are explicit acceptance.

All 101 identifiers and ticks are unchanged: **82 complete / 19 pending**.
Only 8.8 and 8.10 task descriptions move; the other 99 descriptions, including
9.6 and group 15, and all earlier dated progress sections retain their bytes.
Pass A is adopted, including the roster-comment correction. C's confirmation
outcomes remain intact; C/D and E–K are unscheduled. 8.8, 8.10 and 9.6 stay
unticked, and this tasks draft claims no completed Pass B implementation.
The change stays whole under AS1/0030 with LaneTally's sole 11.4 exception;
DSH stays `unmeasured` and 0056 stays `proposed`.

Strict active OpenSpec validation, status/delta parsing, task/requirement
coverage and `git diff --check` pass. All 20 requirements / 150 scenarios and
their five delta files are unchanged. Structural evidence and command results
are under `.forge/tasks-838309ce/`. Production, declarations, decisions,
recipes, pins, frozen paths and living capabilities are unchanged from the
adopted head. No test is added for this tasks-only preparation edit.

With `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` and #282's environment workaround,
format, all-target/all-feature clippy, the workspace test listing and unfiltered
suite, both bundle compiles and the release build cannot launch: Cargo is
absent (`ENOENT`). No Rust gate passes here. They remain required before the
implementing pass's final commit, using
`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` for Git and validation processes.
The known machine-proof failure is not newly diagnosed or exempted; no skip was
used and the single archive-dependent assertion/full archived suite remain
pending under unchanged 15.3/15.7. Exact coverage remains pending controller
host evidence outside the box, with CI, release admission and coverage still
consuming `rust-nightly-version.txt`; remote results remain pending.

This is a `drafted` task-planning checkpoint, not implementation completion.
No earlier artifact must change to write an honest breakdown, so `upstream`
is not warranted. Commit this preparation unsigned in the repository style;
no provider probe, workflow runner, archive, task tick, push, merge, activation
or new Brokkr run occurs in this visit.

## Current implement visit — Pass B DSH planner, 2026-09-14

Run `current-successor-issue-226-pass-838309ce`, implement phase and its only
seat, adopts `3e01ca6`, design `1a04751` and the inherited planner at
`31cd6fa`. **Pass B alone**: the planner portion of 8.8(d) and its matching
8.10 cases. 8.8, 8.10 and 9.6 stay unticked — their whole acceptance still
depends on Pass C's child confirmation and Pass D's accounting,
deduplication and remaining matrix, which this visit does not begin.

Delivered, with its verification beside it:

- **Originating home.** `ResumeTarget` carries `persistence_home` read off
  the SAME confirmed checkpoint's `/transcript/home`; `start_context`
  publishes it in `owned_target`; `owned_dsh_root` requires a string provider
  ID, a non-empty locator and the recorded home, canonicalizes the recorded
  and admitted homes and requires equality before any retained read. Unit and
  integration cases read the actual `Start.input` at both production callers,
  a single site and a panel member, and prove a no-offer start carries none
  and an older row's home is never borrowed.
- **Admission rule and diagnostics.** After the one separate `--model <id>`,
  the shared effort splitter's two spellings and the one `--patch <value>`
  are extracted, every residual argument is refused before the route read,
  version probe, producer call or staging; joined model spellings and effort
  without a model refuse on the cold, offered and disabled paths; the fixed
  categories echo no option, model, ID or path.
- **Bounded pre-spawn reads.** The header line, the retained-root
  enumeration and the stored sequence each carry a finite DSH-local budget;
  every project/session directory and the selected `session.v3.jsonl` is
  canonicalized and confined to the retained root; an ambiguous, truncated,
  escaping, non-regular or over-budget boundary declines rather than
  selecting a substitute or a partial maximum.
- **Lossless locator.** The offered locator must round-trip and stay within
  the existing 80 Rust-character bound; an overlong value whose prefix names
  a valid root is never selected by truncation.

Verification on this tree, under `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` and
#282's `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`: `cargo fmt --all --
--check` clean; `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings` clean; `cargo test -p brokkr-protocol` 194/194;
`cargo test -p brokkr-runtime` 400/400. The unfiltered `cargo test
--workspace --all-features --locked --no-fail-fast` is 1417 passed / 2
failed: the pre-existing #282
`machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_evidence`
and task 15.3's archive-dependent
`every_capability_names_the_archived_changes_that_wrote_it`, which matches
exactly one test and stays deferred until archive validation. With that one
named pre-archive `--skip`, the workspace suite is 1417 passed / 1 failed,
the sole failure #282. Both bundles compile and the release binary builds.
`openspec validate
2026-09-09-226-session-resumption --strict` passes. One pre-existing
inconsistency was corrected en route: `bundle::compose_tests`' pinned
`recipes/triage` manifest digest disagreed with the measured value already
pinned in `tests/witness_digests.rs`; it now carries that measured value,
and the stale pin failed at the adopted head independently of this visit.
No checkbox, contract, policy, reference, fixture, declaration, recipe,
provider proof, activation or archive changes; DSH stays `unmeasured`,
0056 stays `proposed`, and Passes C–D and E–K are untouched.

## Current implement return — Pass B review residual repair, 2026-09-14

Returned implement answering the Pass B review residual at `86bae3d`
(findings B1–B4), still **Pass B alone**: the planner portion of 8.8(d) and
its matching 8.10 cases. 8.8, 8.10 and 9.6 stay unticked — their whole
acceptance still depends on Pass C's child confirmation and Pass D's
accounting and remaining matrix, neither of which this visit begins. The
predecessor visit's counts above measure `86bae3d`; this visit's are below.

- **B1 — selected storage generation.** The selected `0.1.5-rc.1` core sets
  `SESSION_FORMAT_VERSION = 3` (`@deepseek-ai/dsh-session/lib/index.js`),
  names generation 3 `session.v3.jsonl` (`dsh-session-format`), and the
  overlay's `compression: none` changes only the suffix
  (`dsh-session-persistence-jsonl`), so the retained plaintext artifact is
  `session.v3.jsonl`. `DSH_TRANSCRIPT` now names that generation, for the
  planner's owned-file lookup and the shipped route's cold fold alike; D6
  and 8.8(d)/8.10's basename literal reconcile to it.
  `a_dsh_warm_offer_reads_the_selected_storage_generation` writes the
  literal generation name rather than the shared constant, admits a warm
  plan, then proves a store holding only `session.jsonl` declines to the
  shipped cold route with no offerable root.
- **B2 — header versus event budgets.** `dsh_session_last_seq_with` reads
  the first row (the header) inside `DSH_HEADER_LIMIT` and every later whole
  event inside the file-sized budget, so an ordinary large user message or
  tool result — one serialized row — is no longer cut at the header bound.
  Over-budget event rows, a truncated final row and an over-budget file
  still decline. Both the reader vector and the planner-level warm case
  store a 4096-character event.
- **B3 — strict admission fields.** `dsh_stored_session_with` requires the
  selected core's non-negative safe-integer `delegationDepth`; a string,
  null, negative, fractional or missing depth declines instead of defaulting
  malformed storage to depth zero. `dsh_session_last_seq` treats the first
  row as the header and every later row as malformed unless it carries a
  non-negative integer `seq`; a JSON `null`, non-object, string, null,
  negative or sequence-less complete row declines instead of being skipped
  to a partial maximum.
- **B4 — planner acceptance matrix.** The closed-gate test now exercises
  absent, unmeasured and unsupported assessments, missing accounting
  evidence (`unsupported-resume`), a boundary mismatch and a hands mismatch
  (`restrictions-unavailable`), and missing/mistyped identity and
  absent/mistyped declared digest (`unverified-harness`), with and without
  an offer, asserting each exact refusal reason, the complete shipped cold
  argv and its transcript/model overlay. The residual-control test now runs
  the full control list on the disabled, offered and enabled paths,
  asserting zero producer calls, no version probe, no route read and no
  retained-root allocation.

Verification on this tree, under `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`
and #282's `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`: `cargo fmt --all
-- --check` clean; `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings` clean; `cargo test -p brokkr-protocol
--all-features` 199/199; `cargo test -p brokkr-cli --test driver_conformance`
13/13, including the DSH usage fold that now follows `session.v3.jsonl`.
With the named pre-archive `--skip
every_capability_names_the_archived_changes_that_wrote_it` (which
`-- --list` matches exactly once), the workspace suite is 1422 passed / 1
failed: the sole failure is the pre-existing #282
`machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_evidence`,
and `provenance`'s `every_archived_change_is_named_by_the_capabilities_it_promoted`
passes beside exactly one filtered test. Both bundles compile and the
release binary builds. `openspec validate
2026-09-09-226-session-resumption --strict` passes. No checkbox, contract,
policy, reference, fixture, declaration, recipe, provider proof, activation
or archive change; DSH stays `unmeasured`, 0056 stays `proposed`, and
Passes C–D and E–K are untouched.

### Controller repair, 2026-09-15 — R1, the shipped cold route's telemetry

The Pass B review chief raised R1 as a medium on `5a38fb9` and it is repaired
here by the controller, outside a seat, because it is a regression this change
introduced rather than work the pass had left to do. No checkbox moves; 8.8,
8.10 and 9.6 stay unticked.

B1 above reads, of the selected generation name, that "`DSH_TRANSCRIPT` now
names that generation, for the planner's owned-file lookup **and the shipped
route's cold fold alike**". That second half is withdrawn. Unifying the two
was the defect: a shipped cold launch — one the qualification gate disabled,
or one whose composite did not match the declared identity — runs whatever
core the host has installed, and that core may still write `session.jsonl`.
Moving the one shared name therefore cost every such launch its seat turns,
silently: the run succeeded, reported `model` and `effort` as `not reported`,
and carried no turn or token totals at all. `specs/adapter-resume-safety`
forbids exactly that, since a previously supported shape may not quietly lose
what it had.

The repair separates the two readers, which were never the same question:

- `find_dsh_transcript` — the seat's own telemetry discovery — now reads both
  generations through `DSH_TRANSCRIPT_NAMES`, the selected name first. It is
  still a fixed name list joined onto a known session directory, never a
  directory scan, so the race the original comment refuses is still refused.
- Strict warm admission is unchanged and still names `DSH_TRANSCRIPT` alone.
  A store holding only `session.jsonl` still declines the warm offer, and B1's
  `a_dsh_warm_offer_reads_the_selected_storage_generation` still proves it.

Both new tests write the two filenames as **literals**, so a later edit to
either constant cannot move what they claim, and both were proved by removal:
with the shipped name dropped from the list, `driver_conformance::the_shipped_
cold_transcript_name_keeps_its_seat_telemetry` fails with the regression's own
signature — succeeded, `model: not reported`, no seat turn — and
`both_shipped_generations_of_the_transcript_name_are_discovered` fails on the
`session.jsonl` leg. `the_selected_generation_is_preferred_when_a_session_
holds_both` pins the order for a freshly upgraded host.


## Current tasks visit — Pass B residual execution order, 2026-09-15

Run `current-successor-issue-226-pass-84698352`, tasks phase, adopts `f63c113`
and the inherited proposal U/design D10 drafts. The predecessor implementation
and chief readouts retained under `.forge/results/` supply the findings; no
independent journal replay is claimed. The controller's dated R1 repair was
read first and remains byte-identical. No `returned_from` finding is supplied
for this phase; R2–R5 are the commissioned residuals, already reconciled in
proposal U and D10's September 15 sitting.

Updated the existing 8.8/8.10 descriptions in execution order: R2's serialized,
indexed checkpoint transport and five-coordinate journal/actual-Start checks;
the calibrated private staging observation; R3's shipped-route positives and
full three-path refusal matrix; remaining gate/identity/storage observations;
and R4's admitted multibyte warm/output/clamp round trip. Each obligation
names its observing suite and served requirement. The inherited carrier and
planner are preserved unless a failing B case demonstrates a repair. The
R5 office boundary is explicit in the current commission. These edits carry
settled design choices into the breakdown; adding a new scenario, mechanism
or task catalogue is unwarranted because AS1–AS3/SR2/SR3/SR5 already own the
outcomes. No earlier artifact must change for an honest breakdown, so this
phase records `drafted`, not `upstream`.

Structural validation retains all 101 IDs/ticks, **82 complete / 19 pending**,
and requirement coverage for all 20 requirements / 150 scenarios. Only 8.8
and 8.10 task descriptions change; the other 99 tasks, C/D clauses, every
prior dated return and the controller repair retain their bytes. Proposal,
design, deltas, proposed 0056 and the unmeasured DSH declaration retain their
entry bytes. The real archive remains the whole change's final artifact
operation under 15.7; this Pass B tasks draft completes no implementation,
provider, C/D or delivery obligation and ticks nothing.

Strict active OpenSpec validation, status/delta parsing and whitespace checks
pass. With `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` and the #282 environment
workaround, the format, clippy, protocol/runtime suites, driver conformance,
workspace listing/tests, both bundle compiles and release build each exit 127
because Cargo is absent from this box. None passed; no new test exception is
introduced and no final commit is made before the commissioned gates. This
tasks draft remains uncommitted beside the inherited proposal/design drafts.
Evidence is under `.forge/tasks-84698352/`. Host exact coverage and native
Windows/macOS CI remain pending controller results; both workflows and the
unchanged coverage script consume `rust-nightly-version.txt`. No archive,
provider probe, push, merge or new Brokkr run occurred.

## Current implement — Pass B residual, 2026-09-15

Run `current-successor-issue-226-pass-84698352`, implement phase and its only
seat, adopts `f63c113` and the inherited proposal U / design D10 / tasks drafts.
**Pass B alone**: the planner portion of 8.8(d) and its matching 8.10 cases. The
controller's dated R1 repair is adopted closed and byte-identical; the two
reader questions are not reworked and the literal-filename regressions are
untouched. 8.8, 8.10 and 9.6 stay unticked — their whole acceptance still
depends on Pass C's child confirmation and Pass D's accounting and remaining
matrix, neither of which this visit begins.

Delivered, with its verification beside it:

- **R2 — portable checkpoint transport.** `dsh_model_driver` in
  `engine/resume_tests.rs` now serializes one complete checkpoint-data JSONL row
  per declared invocation with `serde_json`, including that invocation's own ID,
  into a data file beside the verdicts. The shim selects its indexed row and
  emits it as a `%s` argument in a fixed format: no payload byte enters shell
  source or a `printf` format, and a missing row fails the shim explicitly
  instead of repeating the last checkpoint. Both existing carriers
  (`an_offered_dsh_start_carries_the_recorded_home_at_the_single_site` and
  `..._at_the_panel_member`) run on their actual temporary homes and now also
  assert that provider ID, locator, home, originating harness version and
  wrapper digest come off the SAME confirmed checkpoint in the actual
  `Start.input`. `a_windows_shaped_dsh_home_survives_the_checkpoint_transport`
  runs the actual shim with backslashes, a percent sign, a quoted span and an
  embedded newline, and decodes one frame back unchanged;
  `a_missing_dsh_checkpoint_row_fails_the_shim_without_repeating` proves the
  explicit failure. `a_stamped_row_is_offered_only_to_its_own_site_owner_and_
  persistent_root` was extended with distinct old/new ID, locator, home,
  version and digest plus a mistyped newest version/digest, proving the split
  `eligible_offer`/`originating_root` scans read the same newest row and treat
  missing evidence as missing. This is portable transport evidence; native
  Windows/macOS results stay pending controller CI.
- **R3 — planner route evidence.** A private `#[cfg(test)]` thread-local staging
  counter at `dsh_seat_overlay_in` entry (before either the settings or patch
  creation) is reset per synchronous planner call and calibrated to one on each
  positive plan. `dsh_positive_planner_paths_fold_the_shipped_route_ahead_of_
  rust_owned_rows` drives the committed `recipes/research-dsh` route through
  qualified cold, qualified warm, disabled cold, declared-composite mismatch
  (no refusal token) and originating-identity mismatch (`unverified-harness`),
  asserting each planned command, exactly one `--patch`, one staging call,
  unchanged reasoning levels and the route rows ahead of every Rust-owned
  persistence/model/settings row. `dsh_route_grammar_matrix_refuses_before_
  staging_on_every_planner_path` runs fifteen grammar vectors through disabled,
  offered and enabled planning with zero staging, zero producer calls and no
  version probe; `dsh_route_binding_matrix_refuses_before_staging_on_every_
  planner_path` does the same for the absent/disagreeing/digest-mismatch
  relationships and proves the digest check precedes the shape check. The
  existing path-refusal and digest/shape cases now assert the calibrated
  zero-staging observation directly, rather than inferring it from an error.
- **R4 — admitted multibyte round trip.** `an_eighty_character_multibyte_dsh_
  locator_round_trips_through_warm_planning` plants an 80-Rust-character
  locator whose UTF-8 encoding is 144 bytes, drives warm planning, and asserts
  the exact output locator, the original root, the warm argv and one staging
  call; the planned value then passes the shared `Transcript::record` clamp
  unchanged. Proved by removal: replacing `resolve_dsh_root`'s
  `chars().count()` with `len()` fails this case, and the check was restored.

Verification on this tree, with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` and
#282's `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`: `cargo fmt --all --
--check` clean; `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings` clean; `cargo test -p brokkr-protocol --lib` 205/205;
`cargo test -p brokkr-runtime --lib` 402/402; `driver_conformance` and the R1
telemetry regressions pass. The unfiltered `cargo test --workspace
--all-features --locked --no-fail-fast` with only task 15.3's pre-archive
`--skip every_capability_names_the_archived_changes_that_wrote_it` (whose
`--list` output matches exactly one test) passes every target except the
pre-existing #282
`machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_
evidence`. Both bundles compile, the release binary builds, and
`openspec validate 2026-09-09-226-session-resumption --strict` passes. No
checkbox, contract, policy, reference, fixture, declaration, recipe, provider
proof, activation or archive change; DSH stays `unmeasured`, 0056 stays
`proposed`, exact coverage and remote results stay pending with the controller,
and Passes A and C–K are untouched.


## Current tasks visit — remediation breakdown, 2026-09-15

Historical record, superseded by proposal W and the operator-ruling breakdown
below. Its cold-interval disposition and request to rebuild the settled reader
seam are no longer instructions for the current slice.

Run `remediation-slice-branch-integra-1d3bbafc`, tasks phase, adopts `0f3f517`
and proposal V under the same dated change. No `returned_from` finding was
supplied. D10's remediation sitting settles the remaining design choices;
its Open Questions contain no ambiguity about what this slice builds. No
upstream artifact amendment is needed for this breakdown.

The following numbered clauses are execution order within the existing
checkboxes, not additional task identifiers. Each cites its owning requirement
and observable completion evidence. Preserve the 101-ID ledger and every tick;
write remediation evidence and pending work here without completing a task.

1. **Preserve the evidence and measure before editing production.** Existing
   8.10/validation obligations; safety / **AS1 Resume support is measured per
   adapter and execution shape**, evidence / **LE5 Launch conformance covers
   every site and historical compatibility**, progress / **PM4 Task completion
   does not stand in for delivery proof**. Read proposal V and D10's current
   sitting first. Preserve the saved JSON, LCOV and summary plus their hashes
   before any gate can overwrite them. D10's saved-report audit reproduced
   **30,014/30,014 lines, 5,036/5,036 branches, 2,860/2,861 functions**;
   those are inherited covered/total pairs, not this slice's before run.
   On those coordinates `owned_dsh_root` has 38 hits, the unreadable-sequence
   closure has two, and the second `dsh_session_file(...).map_err(...)`
   closure has zero across its instances. Reapply the literal script reduction
   and inspect JSON/LCOV function rows instead of inferring closure execution
   from line coverage. Keep the inherited sequence-guard mutation's provenance
   separate from the missing selection-arm control.

   In a toolchain-equipped workspace, choose and verify a writable disk-backed
   directory outside the repository for `TMPDIR`; do not use the host's filling
   `/tmp` tmpfs, assume the global directory is writable, or use the repository
   as temporary storage. Set `remediation_tmp` to that verified absolute path. Run
   `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0 TMPDIR="$remediation_tmp" bash scripts/coverage-exact.sh`
   before the production seam. Inherit #282's environment cleanup in all test
   children, including their Git initialization. Record the exact command,
   status, UTC time, source revision/tree and any diff, compiler pin/version,
   cargo-llvm-cov version, actual selected tests and binaries, profile identity,
   and hashes of the resulting raw JSON, LCOV and summary. Preserve raw profiles
   outside the repository before the script's EXIT cleanup if needed to make
   that chain inspectable; do not change test selection or counting to retain
   them. Separately run/list the requested
   `cargo test --tests --workspace --all-features --locked` selection, confirm
   the unreadable-sequence test executes, and record its actual protocol test
   count; the predecessor's 313 and the gate's selection are not assumptions.
   This fresh invocation must distinguish candidate-bound output from stale
   output and attribute the uncovered closure. A tool failure supplies no new
   coverage integers. Preserve the script, workflow pins, production denominator,
   nonzero equality, all guards and all tests; add no exclusions or coverage
   attributes. New evidence requiring a different mechanism returns to its
   owning design/specification before implementation continues.

2. **Exercise the retained second file selection.** Existing 8.8(d)/8.10;
   site / **SR3 The offered handle names the provider's own session**,
   safety / **AS1**, evidence / **LE2 Launch evidence stays within a versioned
   closed vocabulary**. After the baseline, inject an outer file-selector
   parameter into the existing private `owned_dsh_root`; the production
   caller passes the real `dsh_session_file`. Retain initial `resolve_dsh_root`,
   second selection/error mapping and sequence guard. Adapt existing direct
   test callers to the real selector. Add no wrapper, public API, dispatch
   input, ambient switch or production test branch.

   Extend `crates/brokkr-protocol/src/adapters/tests.rs` with a deterministic
   store-drift case: a contained root with matching depth-zero header and
   readable sequence first resolves successfully; the injected outer selector
   changes that temporary header's ID, then calls the real selector. Assert
   exactly one injected call, `unverified-harness` and the real bounded
   selection reason. Keep the sequence independently readable so it cannot
   conceal a missing selection refusal. The stable-store control returns the
   exact root and stored boundary. Preserve and run
   `owned_dsh_root_refuses_a_store_whose_sequence_cannot_be_read` separately.
   Prove the new refusal test by temporarily bypassing the production outer
   selection failure with the test's known retained file: the readable sequence
   must now yield success and fail the refusal assertion at runtime. Restore
   the exact expression and rerun successfully. A diagnostic rename, altered
   test input or compilation failure does not prove this arm. Every added
   behavioral test needs a compiling behavior mutation and restored pass.
   Verify focused owned-root tests before the next clause; final coverage
   must show the real outer error closure hit through the unchanged reduction.

3. **Connect the cold plan to the sequence-zero drain.** Existing 8.10 and
   the bounded part of 9.6; evidence / **LE4 Resume does not recount old work
   as new activity**, scenarios **A cold DSH transcript begins at sequence
   zero** and **A rejoin has a stored historical boundary of zero**. Adopt
   the existing `Option` fix. Extend the existing DSH planner cases in
   `adapters/tests.rs` to assert `DshLaunch.first_seq == None` for shipped
   disabled cold and qualified cold, then feed each actual plan boundary into
   `drain_dsh_transcript` over a temporary real transcript beginning at zero.
   Warm planning from a stored zero returns `Some(0)` and the same drain
   excludes historical events at or below zero while folding later events.
   Preserve the unsequenced-row behavior and the existing fold/drain regression
   `a_cold_dsh_launch_folds_its_first_event_and_a_warm_one_folds_past_the_boundary`.
   Temporarily restore `None`-as-zero filtering and observe that regression
   fail; restore/pass. Separately change the cold planner seed to `Some(0)`
   and observe the plan-to-drain case fail; restore/pass. Demonstrate the warm
   stored-zero control fails if its boundary is discarded, then restore/pass.
   Verify the focused planner and telemetry cases before proceeding. These are
   retained-file/shim controls, not live qualification or Pass D accounting;
   neither the plugin's first-current-sequence convention nor its bytes change.

4. **Verify the adopted Codex refusal ordering and selector boundary.**
   Existing 8.10; evidence / **LE2**, scenario **The closed shape gate is the
   cause of a Codex decline**; safety / **AS3 Invocation settings cannot
   redirect or weaken a resume**, scenario **A bundle supplies a Codex resume
   selector on a cold launch**. Run the inherited
   `a_closed_gate_names_its_own_reason_ahead_of_the_seat_s_local_checks`.
   Temporarily restore local ID/sandbox/compatible-argv checks ahead of the
   gate; the unmeasured/no-sandbox offer must fail with `sandbox-unavailable`
   instead of `unsupported-resume`, then restore/pass. Retain boundary/hands
   mismatch, byte-identical cold argv, absent observed version on gate decline,
   enabled local-defect and no-offer controls. No offer has no refusal; enabled
   no-offer planning may still observe a version, whereas disabled planning
   does not. The selector guard precedes all of these safe-cold decisions.

   Run `a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`
   through planner/invocation, covering cold and offered paths and a disabled
   shape. Temporarily bypass the production selector guard and observe its
   assertion fail at runtime; restore/pass. Observe refusal before provider
   work, no echoed handle and unchanged selector-free cold argv. At `invoke`'s
   existing Codex replacement call, document that `codex_cold` receives the
   same immutable `extra` already validated by successful `codex_launch`.
   Verify that call path and the focused Codex tests; do not duplicate the
   guard in the builder, drop argv, widen grammar or alter provider enablement.

5. **Pin the committed plugin set without changing it.** Existing 8.8/8.10;
   safety / **AS1**, scenarios **The adaptation is compared as the published
   file set** and **The adapted package retains upstream publication metadata**.
   Read and adopt `extensions/dsh/PROVENANCE.md`'s never-publish rule, outside
   the runtime digest. In `adapters/composite/tests.rs`, extend
   `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`
   to compare the complete six-path digest map with the committed-adapted
   values recorded in PROVENANCE. Retain exact filenames and the upstream
   one-expression reversal proof. To prove the manifest assertion, copy the
   six-file set to an isolated temporary directory, change one manifest byte
   there, temporarily point the test at that copy and observe the digest
   assertion fail. Restore the test and rerun against unchanged committed
   bytes. Never mutate the repository plugin as a control. Verify its six
   hashes again and run the composite suite. A digest test detects drift;
   never-publish remains an operator rule. Add no `private: true`, metadata,
   publisher, plugin delta, component/composite producer or registry probe.

6. **Validate the slice, measure after and commit its evidence.** Existing
   validation obligations serve **every requirement of this change**, with
   evidence / **LE5** and progress / **PM4** governing claims. After all mutation
   defects are restored, run the affected protocol/runtime crate suites and
   CLI driver conformance, `cargo test --workspace`,
   `cargo test --workspace --all-features --locked`, and the requested
   `cargo test --tests --workspace --all-features --locked`. Run
   `cargo fmt --all -- --check` and
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   compile `bundles/self` and `bundles/verify` with
   `cargo run --locked -p brokkr-cli -- compile --bundle <bundle>` and run
   `cargo build --release --locked -p brokkr-cli`. Validate the active change
   with `openspec validate 2026-09-09-226-session-resumption --strict --no-interactive`
   and `openspec validate --all --strict --no-interactive`. Inherit the Git
   cleanup and existing `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2` resource bounds.

   Repeat clause 1's unchanged gate with a verified external disk-backed
   `TMPDIR` and retain its full provenance. Report actual before/after
   covered/total line, branch and function integers and exit statuses; trace
   the newly hit second-selection closure and the added compiled instances
   through the literal reduction. Use the measured denominator, never an
   estimate or a manufactured hit. CI, release admission and local coverage
   retain `rust-nightly-version.txt` (`nightly-2026-09-05`). Record skipped
   namespace boundary evidence separately: #286 prevents in-box equality,
   and the final host measurement is the controller's to take. A skipped or
   failed check is not green; no new filter, lowered gate, archive or spec
   fold is authorized to change its result. Record inherited results as
   inherited and fresh results as fresh. Host equality, remote CI and the
   operator's Codex cold-interval ruling remain pending until they exist.

   Before the unsigned commit, reconcile each control's temporary diff,
   runtime failure and restored pass, inspection of the final diff, actual
   local-check results and remaining controller work. Confirm that every
   original checkbox/tick and the plugin, declarations, proposed 0056,
   frozen surfaces and withdrawn living specs retain their adopted state.
   Commit only the completed authorized slice and its evidence with the
   repository's message style using
   `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0 git -c commit.gpgsign=false commit ...`.
   Record the commit identity; never push, merge, publish, start another run,
   claim a provider ruling or archive this open change. This is no claim of
   whole-task or whole-feature completion.


### Tasks-phase validation and handoff

Read the rendered tasks instructions, dialect tasks/return instructions,
proposal V and owning scenarios, D10's current sitting and Open Questions,
proposed 0056, the relevant source/tests, PROVENANCE and adoption history
through workspace hands. The breakdown preserves the chosen design and all
settled repairs. It changes the current scope and existing 8.8/8.10/9.6
acceptance descriptions; the other 98 checkbox descriptions and every prior
dated progress account retain their bytes. Each ordered clause names its
requirement, observing suite, mutation control and completion evidence.

Strict active OpenSpec validation and `openspec validate --all --strict
--no-interactive` pass: **14 passed, 0 failed**. Status reports planning
artifacts present; it is not implementation or delivery completion. The
existing informational archive notices for the absent living safety/progress
specs remain outside this slice. Whitespace, task-identity and requirement
citation checks pass: **101 IDs, 82 complete / 19 pending**, with all
**20 requirements / 155 scenarios** preserved. No tick changes. Only this
tasks artifact has a tracked diff; source, tests, other artifacts, proposed
0056, plugin bytes, declarations, living specs and frozen surfaces retain
entry hashes.

Format, all-target/all-feature clippy, protocol/runtime suites, CLI driver
conformance, `cargo test --workspace`, locked all-feature workspace tests,
the separately requested `--tests` selection, both bundle compiles and the
release build each exit **127** because Cargo is absent from this seat.
They used #282's Git-environment workaround and the existing resource bounds;
none is reported green. The gate was not launched and its three inherited
reports retain their hashes. There is **no fresh before/after or in-box
coverage measurement** from this tasks visit. D10's inherited
**30,014/30,014 lines, 5,036/5,036 branches, 2,860/2,861 functions** remain
saved-report evidence only. Implementation must execute the controls and
fresh measurements above; #286's final host measurement is the controller's
to take, pending actual results.

Audit and command logs are under `.forge/tasks-remediation-1d3bbafc/`.
This unsigned tasks checkpoint claims a drafted breakdown only. Implementation,
Rust gates, host equality, the operator's Codex cold-interval ruling and remote
results remain pending. No provider probe, archive, spec fold, push, merge,
publication or new Brokkr run occurred.

## Current tasks visit — operator-ruling breakdown, 2026-09-15

Run `operator-ruling-slice-branch-int-bf5f6a32` adopts returned design
`78098d5`, task checkpoint `26f665e` and implementation `2da94b0`. The earlier
F1 tasks return reconciled 6.4, 8.5, 8.10, 11.1 and 13.1 with design `15d6a38`.
D11's F2 inventory correction now agrees with the unchanged five deltas and
this breakdown. D10's council reconciliation retains that plan; this visit
names its existing gate/no-hands and cold-replacement checks explicitly under
8.10 and clauses 3/6. Proposal W/AS1/LE5 stand: no earlier requirement is
defective. D10 records the resolved composition scenario under `## Decisions`;
its Open Questions defer no choice.
Proposed 0056's amendment remains the first semantic implementation obligation
within this same open change.

The supplied `returned_from` reports a drafted design and two informational
archive-target notices: living `adapter-resume-safety` and
`sdd-progress-markers` targets are absent. The operator excludes archive and
re-folding the withdrawn living specs from this slice. Retain the deltas and
notices; they do not prevent this active implementation breakdown or require
an upstream change to the preservation requirements.

These numbered clauses order the pending acceptance within existing checkbox
IDs; they are not additional tasks or permission to tick any whole-change
checkbox. The 82 checked tasks record adopted work, not proof of the new
amendments. Record execution, temporary mutation diffs, commands, observed
runtime failures, exact restorations and passing reruns here as each clause
finishes. Stop dependent work on an unexplained failure; retain inherited
repairs and fix only a gap the commissioned control demonstrates.

1. **Preserve reports and take the fresh before measurement.** Existing
   8.10 and group 15; safety / **AS1 Resume support is measured per adapter
   and execution shape**, evidence / **LE5 Launch conformance covers every
   site and historical compatibility**, progress / **PM4 Task completion does
   not stand in for delivery proof**. Read W, D5/D10 and
   `extensions/dsh/PROVENANCE.md` before editing. Preserve and hash the saved
   `target/coverage/{coverage-exact.json,lcov.info,coverage-summary.json}`
   outside the output location before the gate can overwrite them. The
   inherited **30,015/30,015 lines, 5,036/5,036 branches, 2,861/2,861 functions**
   is a prior report, not this slice's fresh baseline. Do not repeat the
   second-selection diagnosis or rebuild its settled reader seam.

   Verify a writable disk-backed directory outside the repository and set
   `ruling_tmp` to its absolute path; never use the filling `/tmp` tmpfs,
   assume global `TMPDIR` is writable, or place coverage scratch in the repo.
   Run `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0 TMPDIR="$ruling_tmp" bash scripts/coverage-exact.sh`
   before semantic/source changes. Inherit #282's cleanup in every Git call
   and parent test command so child `git init` calls are corrected too.
   Record UTC time, exact command and exit status, revision/tree and changed
   bytes, the compiler pin and actual compiler/cargo-llvm-cov versions, and
   JSON/LCOV/summary hashes with the gate's covered/total line, branch and
   function integers. Preserve raw profiles outside the repo before cleanup
   if needed to connect reports to binaries; never change counting or test
   selection. If the compiler or suitable scratch is absent, record the exact
   unavailable prerequisite and pending measurement, not fabricated integers.

2. **Amend the proposed rule, then prove the shipping regression before
   correcting the declaration.** Existing 1.1, 8.5/8.10 and 9.7; safety /
   **AS1**, **AS2 Every resume re-imposes the current effective restrictions**;
   evidence / **LE1 Every model adapter reports actual launch outcomes** and
   **LE5**, scenario **The shipping Codex declaration actually rejoins a retry**.
   Amend proposed 0056 ruling 5's title/paragraph, the directly conflicting
   rejected alternative and current consequence exactly as D10 specifies:
   preserve the rejoin main ships across historical measurement drift while
   retaining qualification for never-supported shapes and every real mismatch
   refusal. Cite the operator's ruling verbatim; keep `Status: proposed`,
   accepted 0030 and dated evidence unchanged. Verify the decision and registry
   with `cargo test -p brokkr-cli --test decisions_index` and check its account
   against W/AS1 and D5/D10 before the declaration edit.

   First add D10's composition bridge in the existing runtime boundary/agent
   suites: load shipped Codex through `Adapters::load`, resolve a work agent
   with hands under harness, compose its `SiteSpawn`, and obtain the work input
   through `seat_input`/`mark_hands`. Assert `boundary: harness`, no `hands`
   key, the shipped `--sandbox workspace-write`, current model/effort and no
   workspace MCP fragment. Assert the declaration's scope matches those facts.
   Pair boxed resolution with its full MCP-bearing workspace argv and boxed
   input facts, without spawning a nested namespace. Existing synthetic
   boundary fixtures alone cannot prove this shipped composition.

   In `crates/brokkr-cli/tests/driver_conformance.rs`, add D10's deterministic
   cold-then-retry exchange with test-owned homes. Feed the actual shipped
   `adapters/codex.json` resume map to `/resume_context/assessment`; no
   positive status override or `enabled_assessment`. Use the production
   resolver and `compose_site` APIs to obtain the harness argv, with the exact
   input facts pinned above. The CLI shim answers `--version` as
   `codex-cli 0.153.4`, captures argv and confirms a root before current work.
   Establish the cold root through the production driver; a fresh driver
   receives `hello`, correctly correlated `resume`, `start`, `shutdown`.
   Assert `exec resume`, exactly the offered thread, re-expressed
   `sandbox_mode="workspace-write"` and effort, current prompt/result
   destination, and the production adapter's confirmed `launch: resumed`
   without an unsupported refusal. The shim emits provider events, not launch
   evidence. Run this driver case separately from the composition assertions:
   neither a declaration-scope assertion nor an assertion of the cold launch's
   gate-dependent `root_session` may abort before the retry's argv/launch
   assertion. Carry only origin fields the cold driver actually reports; never
   synthesize missing version evidence. Assert the complete cold and resumed
   roots as well on the restored supported path. Run the behavioral test
   against the still-disabled declaration
   and retain its runtime assertion failure. A compile error, bridge-only
   mismatch or status-only assertion is not this launch control. This binds
   LE5 to D10's scenario **The engine-composed harness work seat keeps main's
   rejoin**, closing F1's composition gap.

3. **Preserve shipping support and prove every retained admission fence.**
   Existing 6.1/6.4–6.6, 8.1/8.5/8.10, 9.5/9.7; safety / **AS1**, **AS2**;
   site / **SR1 Every model work site receives its own eligible offer**,
   **SR2 An offer preserves all established instance identity checks**, **SR3 The
   offered handle names the provider's own session**, **SR5 Session recovery uses durable
   evidence without widening crash recovery**; evidence / **LE2 Launch evidence
   stays within a versioned closed vocabulary**, **LE5**. Set only Codex
   `work-site` to `supported`, `boundaries: ["harness", "not applicable"]` and
   `hands: "none"` — `harness` for the engine-composed work agent, `not
   applicable` for the author-written INLINE work seat whose own argv carries
   its `--sandbox` class and no Brokkr boundary (D10's second coordinate);
   retain measured 0.148.0, applicable 0.153.4, classes, existing evidence and
   every existing limitations byte/order. Add D10's exact partial September 10
   accounting reference and append its dated limitation naming the preserved
   harness and inline coordinates, main's boxed MCP-argv refusal and remaining
   proof debt.
   The boxed cold cause is now `restrictions-unavailable` instead of main's
   `incompatible-argv`; explain it without claiming a new rejoin. The reason
   cites the September 15 ruling, harness preservation and full proof owed.
   Keep the scalar hands type, loader, gate, qualification, allow-list and
   engine markers unchanged. Update only the other three reasons to state why
   main does not perform their rejoin: Claude's incomplete installed 2.1.270 restriction/
   boundary/precedence proof while retaining declared 2.1.266/2.1.266;
   DSH's incomplete exact-root/restriction/multi-message/composite proof;
   LaneTally's unknown wrapper identity/forwarding. Preserve their statuses,
   identities, evidence, scopes and limitations; add no DSH `wrapper_digest`.

   Run clause 2 to green. Temporarily set the actual shipped Codex status
   back to `unmeasured`, rebuild and observe that same behavioral assertion
   fail on the cold/unsupported launch; restore exact bytes and pass. Then
   independently revert only declared hands to `boxed`; the same driver test
   must fail cold/`restrictions-unavailable`, reproducing F1. Restore/pass.
   Calibrate the bridge by suppressing the harness fragment append in
   `compose_site`, then separately the boundary write in `mark_hands`, each
   with an observed corresponding assertion failure and exact restoration/pass.
   Record all controls separately; deleting accounting replaces none of them.
   In the existing runtime `tests/roster.rs` shipped-shape case, load the four
   real declarations through `Adapters::load`, asserting those dispositions,
   Codex's harness/inline/none scope, distinct measured/applicable versions
   and four references, and DSH's absent digest. This is secondary loader
   evidence, not the rejoin proof. Run the boxed negative with its complete
   resolved workspace MCP argv; require cold `restrictions-unavailable` and
   no version probe under the narrowed declaration. Preserve the independent
   existing `-c` allow-list refusal; no boxed or no-hands rejoin is newly
   enabled.
   Run the existing runtime boundary case
   `the_seat_input_names_the_boundary_and_the_marker_only_under_a_box` and
   `engine/resume_tests.rs::no_gate_topology_is_ever_offered_a_session` beside
   the bridge. A site without configured hands has neither field and therefore
   reads the `not applicable` boundary; that coordinate is refused for every
   shape the declaration leaves unmeasured, while Codex's inline work seat
   preserves it, and a harness gate with hands still receives no offer under
   SR1. Absent `hands` alone establishes neither work eligibility nor a
   supported boundary. Reuse these assertions; do not add a duplicate topology
   suite or fabricate a marker.
   Exercise all three other shipped assessments with otherwise eligible offers
   through the existing planner suites and require `unsupported-resume`
   without a version probe.

   Pair the positive with existing or narrowly extended tests for missing/
   unreadable observed identity, observed 0.148.0 or a third version, recorded
   originating-version mismatch, boundary/hands mismatch, absent accounting
   and different confirmed root. A scope/applicability mismatch makes evidence
   inapplicable; neither a prose-evidence parser nor a new gate predicate is
   required. The missing-accounting case must remain closed even after status
   preservation. Run the runtime `engine/resume_tests.rs` suite for site,
   instance, local origin, manifest, latest-owner, legacy-offer and fresh-gate
   fences alongside the actual driver test. Preserve its narrow legacy mapping
   without inventing historical identity fields. For every new behavioral
   test, break its exact production predicate with a compiling mutation,
   observe the claimed assertion fail, restore and pass. Do not introduce a
   bypass, fourth status, public engine seam or duplicate topology suite.

4. **Low 1 — control the adopted DSH sequence-zero fix.** Existing 8.10 and
   only the bounded cold-boundary acceptance in 9.6; evidence / **LE4 Resume
   does not recount old work as new activity**, scenarios **A cold DSH
   transcript begins at sequence zero** and **A rejoin has a stored historical
   boundary of zero**. Run
   `a_cold_dsh_launch_folds_its_first_event_and_a_warm_one_folds_past_the_boundary`
   and `the_planned_dsh_fold_boundary_reaches_the_transcript_drain` in the
   protocol adapter suite. Before any further fix, temporarily restore
   `None`-as-zero filtering in `fold_dsh_event`; observe the cold-zero assertion
   fail, restore and pass. Separately restore the cold planner seed `Some(0)`;
   the actual plan-to-drain case must fail, then restore/pass. Put each folded
   turn-count assertion before its `first_seq` field assertion: the inherited
   test checks the field first and would otherwise stop before the drain.
   Extend that same plan-to-drain test to the shipped-disabled cold route,
   which its current qualified-cold/warm cases do not exercise. Require both
   cold routes to count the sequence-zero event, and the warm route to exclude
   stored zero. The warm-boundary mutation must likewise fail the folded count,
   not merely its field read-back. Keep cold `None`
   through the drain for both qualified and shipped-disabled routes, warm
   `Some(last_stored_seq)` including zero, no-sequence behavior and exclusion
   at/below the warm boundary. Calibrate any new warm assertion by discarding
   that boundary and observing failure before restoration. Preserve the
   plugin's inclusive first-current sequence and Rust's exclusive last-stored
   boundary as distinct conventions. This does not begin Pass D accounting.

5. **Low 2 — control Codex's refusal cause.** Existing 8.10; evidence /
   **LE2**, scenario **The closed shape gate is the cause of a Codex decline**.
   Run `a_closed_gate_names_its_own_reason_ahead_of_the_seat_s_local_checks`.
   Temporarily move the gate-decline block below the sandbox split; observe
   `sandbox-unavailable` fail the expected `unsupported-resume` assertion for
   an unmeasured/no-sandbox offer, then restore/pass. Keep selector refusal
   first, gate refusal before ID/sandbox/compatible-argv checks, no probe on a
   disabled gate, safe unchanged cold argv, boundary/hands refusal and enabled
   local-defect controls. No offer has no refusal; an enabled cold invocation
   may still probe identity. Extend only if an observable gap remains.

6. **Low 3 — control the cold selector refusal.** Existing 8.3/8.10;
   safety / **AS3 Invocation settings cannot redirect or weaken a resume**,
   **AS4 A refused resume permits only one proven pre-work cold replacement**;
   evidence / **LE3 Launch reporting preserves first-work acceptance and refusal semantics**;
   scenario **A bundle supplies a Codex resume selector on a cold launch**.
   Run `a_codex_seat_argv_that_selects_a_session_is_refused_on_the_cold_path_too`
   across no-offer, offered and disabled-shape paths. Temporarily bypass
   `codex_selector_conflict`'s guard, observe the assertion fail, restore and
   pass. Verify refusal before provider work and no echoed handle, including
   bundle `resume <id>` after the driver's `--`. Retain conservative refusal
   of the `resume` word in value positions. Add only the planned comment at
   `invoke`'s direct replacement `codex_cold` call: successful `codex_launch`
   has already validated the same immutable arguments. Pair this control with
   `adapters/tests.rs::a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`:
   its two-child exchange already asserts no `resume` in the replacement argv,
   the retained sandbox class and one cold launch row. Run it with the selector
   case; do not add a test that expects a forbidden selector to reach replacement,
   since the first guard must reject it. No duplicated builder guard, silent
   stripping or provider exception is justified by this design.

7. **Low 4 — pin the unchanged extension bytes and retain never-publish.**
   Existing 8.8/8.10; safety / **AS1**, scenarios **The adaptation is compared
   as the published file set** and **The adapted package retains upstream
   publication metadata**. Adopt the prohibition already in the sibling
   `extensions/dsh/PROVENANCE.md`; change nothing in the six-file package.
   Extend `adapters/composite/tests.rs`'s
   `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`
   to compare the full returned digest map with the six committed-adapted
   digests from PROVENANCE, preserving the exact file-set and reverse-substituted
   upstream JavaScript checks. Copy the set to isolated temporary storage,
   change one manifest byte in that copy, temporarily point the test there and
   observe the digest assertion fail. Restore the test and pass on the real
   unchanged set; verify all six file hashes against PROVENANCE directly and
   run the composite suite. This detects byte drift; the documentation owns
   never-publish. No `private: true`, npm query, publisher or second plugin
   delta is authorized.

8. **Reconcile current prose and measured identities.** Existing 11.1's
   account only, 13.1 and 14.1/14.2; safety / **AS1**, evidence / **LE5**,
   progress / **PM2 Recovery reconciles the marker with surviving work**.
   After clause 3 passes, update 11.1's current account to the actual rejoin
   result and controls, retaining full 10.5/11.1 debt and its unchecked state.
   Align the provider guide's blanket expiry paragraph, Codex row and all-cold
   statement, the `ResumeIdentity::Measured.applies_to` comment and only
   necessary packaged/scaffolded consumers with the four dispositions. Name
   the preserved harness and inline coordinates (both with hands `none`) and
   the boxed coordinate's continued cold launch and changed refusal token; do
   not imply all Codex boundaries are enabled.
   Preserve dated captures and limitations; no live enforcement claim follows from shims or prose.
   After all declaration bytes settle, run the existing witness and compose
   tests and update only their measured left/right digest pairs. Declaration
   reason edits can change any consuming identity, including Claude-only
   bundles; retain the witness equality assertions. Compile both bundles on
   these final bytes. This bounded re-pin completes none of group 14's
   whole-change tasks and grants no cross-instance resumption.

9. **Validate, measure after and commit the slice's evidence.** Existing
   group 15 obligations serve **every requirement of this change**, with
   evidence / **LE5** and progress / **PM4** governing claims. Restore every
   mutation before final validation. Run affected protocol/runtime suites and
   CLI driver conformance, `cargo test --workspace`,
   `cargo test --workspace --all-features --locked`,
   `cargo fmt --all -- --check`, and
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
   Run both `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
   and the corresponding `bundles/verify` command; retain the inherited
   `cargo build --release --locked -p brokkr-cli` check as a separate result.
   Run `openspec validate 2026-09-09-226-session-resumption --strict --no-interactive`
   and `openspec validate --all --strict --no-interactive`. Use #282's cleanup
   with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`. No archive-related skip or
   weakened provenance assertion is authorized for this slice.

   Repeat clause 1's unchanged exact gate and preserve the same provenance
   fields. Report its own actual before/after covered/total pairs and exit
   statuses; never substitute saved integers or predict a denominator. Keep
   CI, release admission and coverage on `rust-nightly-version.txt`; change
   no gate, thresholds, test selection, conditional-compilation exclusions or
   `coverage(off)`. Report actual in-box counts and skipped namespace evidence:
   #286 leaves host equality to the controller, pending its actual result.
   Compiler/scratch absence supplies no new measurement. Record failed or
   unavailable local checks honestly, never green. Remote CI, integration,
   publication and closure remain pending external results, not local claims.

   Verify the final diff contains only this slice, every original checkbox
   state/ID is unchanged, no mutation remains, plugin/frozen/living-spec bytes
   retain their adopted hashes, and proposed 0056 remains proposed. Record
   completed controls and residual proof here, then commit the completed
   authorized work unsigned in repository style with
   `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0 git -c commit.gpgsign=false commit ...`.
   Record the commit identity. Never push, merge, publish, archive, fold living
   specs or start another Brokkr run. This slice's evidence cannot close a
   whole task, new enablement or the open change.


### Implementation record — operator ruling and four lows, 2026-09-15

Executed against the adopted head `21f4ac2` with the pinned toolchain and
#282's Git cleanup. No checkbox was ticked; the ledger stays 82 checked / 19
pending across the same 101 identifiers.

**The ruling.** `adapters/codex.json` `work-site` is `supported`, scoped to
`boundaries: ["harness"]` with `hands: "none"`, retaining the historical
0.148.0 measurement and 0.153.4 applicability and adding the dated 0.153.4
current-accounting reference. Proposed 0056 ruling 5, its rejected
alternative, its consequence list, the `ResumeIdentity::Measured` comment and
`docs/guides/provider-adapters.md` carry the preservation/new-shape
distinction. The Claude, DSH and LaneTally declarations changed reason prose
only and stay `unmeasured`; none was enabled.

Proof: `engine::boundary_tests::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`
loads the shipped adapter and library, resolves the harness work seat and
composes the real argv (`--sandbox workspace-write`, no MCP fragment) and the
`boundary: harness`/no-hands input;
`driver_conformance::the_shipped_codex_harness_work_seat_rejoins_its_retry`
feeds the shipped assessment and composed argv through a cold driver and then
a fresh retry, and observes `launch: resumed` with the exact thread and
re-expressed `sandbox_mode="workspace-write"`/`model_reasoning_effort="xhigh"`.
Controls: status back to `unmeasured` failed the retry with
`resume_refusal: unsupported-resume`; declared hands back to `boxed` failed it
cold with `restrictions-unavailable`; suppressing `compose_site`'s harness
fragment or `mark_hands`'s boundary write each failed the bridge. Every
mutation was restored and rerun green.

**Low 1.** `the_planned_dsh_fold_boundary_reaches_the_transcript_drain` now
asserts the folded count before the plan field and adds the shipped-disabled
cold route. Controls: `None`-as-zero filtering dropped the cold seq-0 event
(`[Some(1), Some(2)]` vs `[Some(0), Some(1), Some(2)]`); the cold seed
`Some(0)` failed the plan-to-drain count (1 vs 2). Restored.

**Low 2.** The closed-gate test failed `sandbox-unavailable` against
`unsupported-resume` when the gate decline was moved below the sandbox split.
Restored.

**Low 3.** The cold-selector test failed (guard returned `None`) with the
selector guard bypassed. Restored.

**Low 4.** `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`
now compares the full returned digest map with PROVENANCE's six adapted
digests; a one-byte `package.json` change in an isolated copy failed the
assertion. `extensions/dsh/PROVENANCE.md` already carries the never-publish
rule and no plugin byte changed.

**Gates (in-box).** `cargo fmt --all -- --check`, all-target/all-feature
locked clippy, `cargo test --workspace`, `cargo test --workspace
--all-features --locked`, both bundle compiles, `cargo build --release
--locked -p brokkr-cli` and strict active/all OpenSpec validation pass. The
adapter edits moved the measured compose/witness pins, which were updated from
the tests' own reported left/right pairs only. The unchanged exact gate reports
before and after **29,844/30,015 lines, 5,023/5,036 branches, 2,851/2,861
functions**; the shortfall is the known #286 in-box boundary-skip, and host
equality remains the controller's. Remote CI, integration, publication and
closure remain pending external results.


### Implementation return — F1 inline declaration identity, 2026-09-15

Answered the judging pass at `81df715` without ticking a checkbox; the
ledger stays 82 checked / 19 pending across the same 101 identifiers.

**F1.** `Bundle::compile_with` now pins the adapter declaration each inline
built-in model driver's resume assessment was read from, beside the gate's
authorising digest and the effort exemption, in the manifest's `drivers`
map. `enforce_model_pins` returns the assessment map for the engine and a
separate witness map merged into `drivers`, so an exemption and a consumed
assessment stay distinct facts while both move identity. A valid edit to
Codex's restrictions evidence moves
`recipes/{fast,node,night-shift,wager-harness,research-dsh}`; the recorded
compose/witness digests were re-pinned from the tests' own reported
`left`/`right` pairs only. The `drivers`-as-gates invariant tests were
reconciled: `witness_digests` now names every inline consult, and
`wager_parity` compares the judging seats after lifting the overridden
`implement` seat.

Proof:
`engine::resume_tests::an_edited_inline_resume_declaration_moves_identity_and_refuses_the_old_root`
compiles the shipped `recipes/standby` twice with one declaration edit, drives
a cold Codex-shaped run to a parked confirmed root, and resumes it under the
unchanged bundle (rejoins `[None, Some("implement-1")]`) and under the edited
one (`EngineError::ManifestMismatch`, so the old root is not reused). Control:
removing `pin_drivers.extend(resume_witness)` failed the pin assertion;
restored and rerun green.

**F2.** The active operator-ruling breakdown's clause 3 now names
`boundaries: ["harness", "not applicable"]`, says which coordinate each word
is, and states that `not applicable` is refused for every shape the
declaration leaves unmeasured while Codex's inline work seat preserves it.
Clause 8's prose reconciliation names both preserved coordinates beside the
boxed coordinate's continued cold launch. The dated implementation records
above stay as history.

**Gates (in-box).** fmt, all-target/all-feature locked clippy,
`cargo test --workspace`, `cargo test --workspace --all-features --locked`,
both bundle compiles, `cargo build --release --locked -p brokkr-cli` and
strict `openspec validate 2026-09-09-226-session-resumption --strict` pass.
The exact gate before this return measured **30,021/29,850 lines,
5,038/5,025 branches, 2,863/2,853 functions**; after, **30,036/29,865 lines,
5,038/5,025 branches, 2,863/2,853 functions** — every added production line
covered and the deficit unchanged at the known #286 in-box boundary skip.
Host equality and remote CI remain the controller's.


### Prior tasks-phase validation and handoff — operator ruling (`c45ee20`)

Read the rendered tasks instructions and dialect tasks/return instructions,
proposal W and owning scenarios, D5/D10 and Open Questions, proposed 0056,
accepted 0030, relevant source/tests and PROVENANCE through workspace hands.
The breakdown implements the design's dependency choices without changing an
upstream artifact: the proposed-decision amendment and declaration/runtime
proof belong to implementation. No `returned_from` finding or unresolved
upstream question accompanies this visit.

Strict active OpenSpec validation passes; repository-wide strict validation
reports **14 passed, 0 failed**. Status confirms planning artifacts exist,
not that implementation is complete. The informational archive-target notices
remain outside this slice. Whitespace and artifact audits pass: **101 task
IDs, 82 checked / 19 pending**, every state unchanged and every task retaining
its requirement citation; **20 requirements / 158 scenarios** retain their
bytes. Only `tasks.md` has a tracked diff. Source, tests, proposed 0056,
declarations, plugin, living specifications, frozen surfaces and all other
tracked artifacts retain their entry hashes.

Format, locked all-target/all-feature clippy, protocol/runtime suites, CLI
driver conformance and decision-index tests, `cargo test --workspace`, locked
all-feature workspace tests, both bundle compiles and the release build were
attempted with #282's environment correction and the recorded resource bounds.
All **11 commands exited 127**: Cargo is absent from this seat. No Rust test or
mutation was executed and no Rust gate is reported green. The coverage script
was not launched: Cargo is absent and the global `TMPDIR` is `/tmp`, which the
commission excludes. There are **no fresh before/after or in-box coverage
counts** from this tasks visit. The saved report remains byte-unchanged at
**30,015/30,015 lines, 5,036/5,036 branches, 2,861/2,861 functions**; these are
inherited integers only. CI, release admission and the unchanged exact gate
all still consume `rust-nightly-version.txt`, pinned to `nightly-2026-09-05`.

Entry hashes, artifact audit and individual Rust command outcomes are retained
under `.forge/tasks-operator-ruling-bf5f6a32/`. This unsigned checkpoint drafts
the task breakdown only. Implementation must execute the ordered amendments,
controls and fresh gates; #286's host equality and final remote results remain
pending the controller. The operator ruling itself is resolved. No provider
qualification, checkbox completion, archive, living-spec fold, push, merge,
publication or additional Brokkr run is claimed.

### Design-return reconciliation — analyze F1, 2026-09-15

This return revised owning D10 first, then the current task instructions and
11.1's pending account. The breakdown now requires harness/none declaration
scope, full production composition, independent status/hands and composition
controls, and a boxed cold negative with its complete MCP argv. The four lows
retain their order. No requirement or task ID/tick changed; all 101 IDs remain
82 checked / 19 pending. Implementation has not run these new proofs.

Strict active and all-item OpenSpec validation pass (14 items, zero failures);
whitespace and artifact audits pass. The design's F1 validation section records
the nine Rust commands unavailable at exit 127 because Cargo is absent, and
no fresh before/after or in-box coverage. The inherited gate counts are not
new results. This return claims a coherent design and dependent breakdown only;
implementation, mutation controls, local Rust gates and controller host/remote
results remain pending. Audit: `.forge/design/operator-ruling-f1-bf5f6a32/`.


### Tasks-return reconciliation and validation — adopted design `15d6a38`

This tasks visit read the returned D10 and its resolved composition scenario,
Open Questions, proposal W, AS1/AS2/AS3 and LE2/LE4/LE5, the rendered tasks
instructions, the dialect's tasks/return instructions, proposed 0056,
accepted 0030 and the relevant production composition, planner and test code.
The operator ruling and design are settled; no upstream amendment is needed
before this breakdown can be executed. The supplied archive-target notices
remain informational for this active slice, as recorded above.

The dependency repair is in the existing numbered checkbox descriptions:

- **6.4 — safety / AS1:** removed the stale instruction to retain Codex's old
  scope. It now requires D10's harness/none declaration, exact dated accounting
  reference and appended limitation after the decision amendment and failing
  behavioral control, with other providers' reason-only changes retained.
- **8.5 and 8.10 — safety / AS1, AS2, AS3; evidence / LE5:** reconciled the
  composition bridge, real driver exchange and independent status/hands
  controls. The earlier MCP-admission instruction remains whole-change debt;
  it does not authorize widening the allow-list in this slice. Separate test
  filters and assertion order ensure the disabled-shape control reaches the
  retry's argv/launch assertion instead of stopping at declaration scope or
  the cold launch's missing versioned root.
- **8.10 and bounded 9.6 acceptance — evidence / LE4:** the current
  plan-to-drain test asserts `first_seq` before calling the drain and covers
  qualified cold/warm only. The low-1 clause now schedules the shipped-disabled
  cold case and puts folded-count assertions first, so the prescribed seed and
  warm-boundary mutations must demonstrate dropped or recounted events.
  These are test obligations, not a new production design or completed proof.
- **11.1's account and 13.1 — safety / AS1; evidence / LE5:** retained the
  pending runtime repair at the adopted head and the full qualification debt;
  aligned guide work with harness preservation and boxed cold behavior.
  Every tick remains unchanged. The four lows keep their commissioned order.

Strict active validation passes; strict all-item validation reports **14
passed, 0 failed**. Whitespace, tracked-scope and requirement-citation audits
pass: **101 task IDs, 82 checked / 19 pending**, every ID/state unchanged,
all **20 requirements / 158 scenarios** retain their bytes, and every numbered
checkbox retains its requirement citation. Only `tasks.md` changed; the other
**727 tracked files**, including design, proposal, specs, proposed 0056,
production/tests, declarations, six plugin files, living specs and frozen
surfaces, retain their entry hashes.

All **11 attempted Rust commands exited 127**, reporting Cargo absent: format,
all-target/all-feature locked clippy, protocol/runtime suites, CLI driver
conformance and decisions-index tests, both workspace test modes, both bundle
compiles and release build. Commands use #282's environment cleanup and the
recorded resource bounds. No Rust test or mutation ran, and no Rust gate is
reported green. The exact gate was not launched without its compiler; this
visit supplies **no fresh before/after or in-box coverage counts**. Saved
reports retain their entry hashes and inherited **30,015/30,015 lines,
5,036/5,036 branches, 2,861/2,861 functions**. Those are prior gate results,
not a measurement of this draft. CI/release coverage and the local exact gate
still consume the unchanged `nightly-2026-09-05` pin.

Audit records and exact command results are under
`.forge/tasks-operator-ruling-f1-bf5f6a32/`. This checkpoint drafts the task
artifact only. Implementation must execute the ordered decision/declaration
amendments, behavioral controls, measured re-pins and local gates; host equality
under #286 and final remote results remain pending with the controller.
The change remains open. No task completion, provider qualification or runtime
repair is claimed by this tasks return.


### Tasks-return validation — adopted design `78098d5`, analyze F2

The returned design resolves F2 in D11: the unchanged five deltas contain
**20 requirements / 158 scenarios**, derived as 31 + 63 + 11 + 20 + 33.
No scenario is missing and Open Questions defer no implementation choice.
This tasks-only return adopts that correction, preserves F1's composition
proof and makes D10's existing no-hands/gate and cold-replacement test pairings
explicit in 8.10 and clauses 3/6, with their requirement citations. It adds
no test, task identifier or production design. The operator ruling remains
first, followed by the four lows in their commissioned order.

No `upstream` amendment is needed: the earlier artifacts already settle the
ruling and the current inventory. The two supplied archive-target notices
remain informational while this change is open; neither a living-spec fold
nor changing delta operations is authorized in this slice. Proposed 0056,
the declaration repair and all prescribed runtime controls remain pending.

Strict active validation passes; strict all-item validation reports **14
passed, 0 failed**. Whitespace, task-citation and tracked-byte audits pass:
all **101 task IDs / 82 checked / 19 pending** retain their states, only
`tasks.md` changes, and the other **727 tracked files** retain their entry
hashes. The six plugin digests still match PROVENANCE. No task is ticked.

All **11 attempted Rust commands exited 127** because Cargo is absent: format,
locked all-target/all-feature clippy, protocol/runtime suites, CLI conformance,
decisions index, both workspace test modes, both bundle compiles and release
build. They inherit #282's Git-environment correction and the resource bounds.
No Rust test or mutation ran, and no Rust gate is claimed green. The exact gate
was not launched without its compiler: **no fresh before/after or in-box
coverage counts** exist for this return. The saved reports retain their hashes
and prior **30,015/30,015 lines, 5,036/5,036 branches, 2,861/2,861 functions**;
those are inherited evidence only. The shared compiler pin remains unchanged.
Implementation owes the measured controls and local gates; #286 host equality
and final-head remote evidence remain pending with the controller.

Readouts and audits are retained in `.forge/tasks-operator-ruling-f2-bf5f6a32/`.
This is an unsigned task-draft checkpoint, with implementation completion,
provider qualification and archive unclaimed. No push, merge or new run occurs.

## Current tasks visit — THE PROOFS execution order, 2026-09-16

This tasks visit adopts proposal AA, AS1/AS2 and LE5 at `614abde`, and D10's
THE PROOFS reconciliation at `80772ce`, over production `4daaa7d`. The supplied
run context has design's `drafted` result and no named `returned_from` finding.
The predecessor return `.forge/results/99cbdaee-9e9d-4fdc-b2f7-25cb2aa60def.json`
was read first. Its delivered family/census/dispatch repairs and four observed,
restored removal controls stand. D10 answers the proof seam and all three
coverage dispositions; no earlier artifact fault requires `upstream`.

These six acceptance clauses refine the existing numbered checkbox groups;
they introduce no task identifiers or ticks. They supersede the prior six-clause
whole-family work list for this visit. Each names the owning tasks and
requirements. Execute item 1 before item 2, recording actual clause progress
and evidence before continuing. Historical ticks do not attest these new
proofs; the ledger remains 101 IDs, 82 checked / 19 pending. The broader F2
collision/rename/identity campaign, selected-single follow-up, C/D, every
whole-change tick, archive and living-spec fold remain outside this commission.

1. **Preserve the baseline and establish measurement prerequisites.** Existing
   1.1 and group 15 — safety / AS1; evidence / LE5; progress / PM1, PM4.
   Record entry revision, clean/dirty state, frozen/living-spec and task-state
   hashes, and preserve existing coverage reports before another run overwrites
   them. Retain proposed 0056 unchanged; its family and affirmative-confinement
   decisions are adopted. Before any test or source edit, run the unchanged exact
   gate using clause 6's compiler/scratch rules, or name the unavailable
   prerequisite. The supplied host baseline at `4daaa7d` is **30,269/30,275
   lines, 5,039/5,040 branches, 2,890/2,890 functions**, covered/total; it is
   not a new in-box run. Verify the baseline record identifies environment,
   revision and all three measures separately.

2. **Connect the existing test transports to the production exchange.** Existing
   3.6, 6.3 and 8.10 — site / SR1, SR2, SR3, SR4; safety / AS1, AS2;
   evidence / LE2, LE5. Follow settled D10: one temporary-realm builder
   parameterizes the three inline Codex forms and wrapping. Use
   `Bundle::compile_with_realm`, shipped dialect/adapter declarations and valid
   author-pinned work argv. Keep the compiled wrapper, complete command,
   canonical site facts and instance identity intact. Reuse the capture patterns
   in `engine/resume_tests.rs` (`received`, `route_start`) and CLI
   `tests/driver_conformance.rs` (`make_shim`, `drive_codex`, `launch_row`), sharing
   ordinary test modules where needed. Add only development dependencies on
   existing workspace crates if required for protocol-unit access to compiler,
   engine and store; no production hook, public gate API or registry dependency.

   Forward the actual engine cold Start through `brokkr driver codex` with the
   existing deterministic provider shim. Return the adapter's real accepted,
   checkpoint and result messages through normal engine recording; the provider
   announces the cold root before work and the engine durably stamps it. The
   existing `driver`/`model_driver` helpers' invented roots cannot substitute.
   Retain the same provider home/origin and instance across cold and retry:
   `drive_codex` currently creates fresh homes per call, so shared test support
   must own those homes for the pair. Capture the engine's retry Resume and
   Start, then forward them unchanged to a fresh real adapter process. Never
   inject `resume_context`, a test-selected offer, repaired markers or shortened
   argv. Observe the model step before the boxed validator; keep that step
   compiled, without requiring namespace creation or validator completion.

   Keep direct no-offer assertions in `crates/brokkr-protocol/src/adapters/tests.rs`,
   where the same private production `resume_gate` is accessible. Shared support
   supplies the unchanged engine-composed `Start.input`; do not copy the
   predicate or return an expected token from a helper. Verify this bridge
   with clause 3's cold/root/retry and private-gate assertions; helper construction
   alone is not proof. Every added launch is Unix-only when it executes a shebang
   file directly, or uses a real target-platform executable. The CLI conformance
   suite already has `#![cfg(unix)]`; do not assume unconditional tests can execute
   scripts or Windows supplies `sh`, and keep production `Command` semantics.

3. **Close item 1 at all six decisions, with observed mutations.** Existing
   3.6, 6.3 and 8.10 — safety / AS1, AS2; site / SR1, SR2, SR3, SR4;
   evidence / LE2, LE5. With clause 2's one builder, execute this matrix; assert
   the actual gate result for every row, not only maps or offer presence.

   | Compiled form | Required decision and exchange |
   |---|---|
   | Wrapped no-hands single, `verify:checks` | Enabled; provider-confirmed cold root, engine's exact same-root retry, confirmed `launch: resumed`. |
   | Wrapped hands-bearing member, `verify:checks:alpha` | Namespace/boxed gives `Disabled("restrictions-unavailable")`; no resumed launch. |
   | Wrapped no-hands member, `verify:checks:x` | Enabled and confirmed same-root retry beside hands-bearing `checks:x`, executing at `verify:checks:checks:x`. |
   | Unwrapped no-hands single, `verify` | The same enabled, confirmed retry. |
   | Unwrapped hands-bearing member, `verify:alpha` | The same confinement refusal and token. |
   | Unwrapped no-hands member, `verify:x` | The same live retry beside its own hands-bearing `checks:x` sibling. |

   For live rows assert the durable cold root equals the engine offer, provider
   argv uses `exec resume` with that root and current sandbox/effort, confirmation
   precedes work, and the launch names that exact root as `resumed` without
   `resume_refusal`. An unconfirmed root must never produce `resumed`. For a
   refused row assert the private production gate's exact token on the captured
   input, even when no eligible root exists. An actual offered fallback also
   asserts its cold launch and token; a no-offer cold exchange carries no
   invented refusal record. Namespace/boxed remains refused even with both
   strings present: applicable affirmative markers are necessary, not sufficient.
   Keep the existing shipping harness/none live control under its own composition;
   never relabel a boxed member or broaden its assessment. Report any safely
   refused coordinate explicitly. This is deterministic protocol evidence,
   not new installed-provider qualification.

   Preserve `Unknown` / `NoHands` / `Hands`. Keep the independent adapter matrix
   against otherwise supported/accounted evidence: boundary missing, hands
   missing, both missing, null, non-string and unrecognized values each decline
   `restrictions-unavailable`. Absent/unsupported assessment or absent accounting
   instead declines `unsupported-resume`. Exercise unresolved/unknown executing
   confinement through real composition without borrowing parent/sibling markers;
   altered-input unit controls remain separate from the six unchanged captures.
   Retain the existing member `checks`, unrelated literal `verify:foo`, harness,
   identity, argv, accounting, fresh-gate and cold-replacement controls. Their
   retention does not commission the predecessor's wider F2 live-rename campaign.

   For **every new test**, record the exact compiling mutation, named assertion
   failure and intended cause, restore the bytes, and record the passing rerun
   before continuing. Apply D10's bindings: disable whole relocation for supported
   wrapped rejoins; lose executing hands/marking for the no-hands mirror; falsely
   mark a hands-bearing member as known no-hands to break its direct gate-refusal
   assertion; delete the independent absence refusal to break otherwise supported
   unknown-marker controls. Common marker/assessment mutations must also fail
   unwrapped controls; relocation-only mutations cannot prove them. Include the
   missing-assessment token and confirmation controls when newly tested. Existing
   argv guards may keep an erroneously enabled boxed gate cold, so the direct
   decision assertion is essential; do not weaken another guard to force `resumed`.
   Map panics, compiler errors, earlier fixture failures or masked mutations do
   not count. Temporary partial-family mutations are isolated experiments only:
   restore them completely and never commit partial relocation. Preserve the
   four predecessor controls as dated evidence, not substitutes for new proofs.

4. **Close item 2 at the three adopted source locations.** Existing 3.5, 3.6
   and 8.10 — site / SR1, SR2; safety / AS1, AS2; evidence / LE5. Only after
   item 1, extend the existing bundle/selected-agent suites and apply D10's
   settled dispositions. Line numbers identify `4daaa7d`, not permanent addresses.

   - `assemble`, lines 1528–1531: compile dialect verify with legal
     `Select { cases: {}, default: Single }`, ordinary role/driver and results
     on the outer seat. The single default passes `selected(None)` so the outer
     Select reaches the second refusal. Assert `dialect verify currently
     requires a single or panel verify seat`. Keep both current guards and the
     existing sequence refusal. Mutate this reached diagnostic, observe the
     named assertion fail, restore and rerun; do not claim that changing a
     diagnostic removes all backstops or establishes future selector policy.
   - `parse_selected_body`, line 3042: put `{"agent":""}` in a legal named
     strategy case, with a valid default and ordinary library/adapter setup;
     keep results on the outer seat. Assert both the selector-qualified seat
     and `agent must be a non-empty string`. Mutate the reached diagnostic/error
     propagation so this assertion fails, restore and rerun. An earlier
     unknown-key/load failure or arbitrary compile error does not cover this
     continuation; retain the resolver's refusal.
   - `owner_index`, line 1780 and branch at 1770: remove only the unreachable
     same-owner tolerance by replacing guarded `Some(first)` plus `Some(_)`
     with one existing-label refusal and insertion on absence. Record D10's
     invariant beside the simplification: unique phase keys; unique case keys
     from `SELECT_STRATEGIES`, which excludes `default`; no nested selection;
     each body emits its single once or members/steps with distinct enumerated
     indices in `SiteKey`. The census cannot revisit an equal owner. Different
     structural keys can flatten to the same label and must still refuse with
     the label and both owners. Same-owner **fact merges** remain legal,
     separately from structural enumeration. Preserve authoring/final censuses
     and wrapper destination claims. If a legitimate parsed counterexample
     disproves the invariant, cover it and reconcile that evidence with D10
     instead of silently rejecting it. Verify with existing
     `a_literal_phase_that_aliases_a_selected_case_is_refused_globally` and its
     rename-only compile control. Temporarily disable this refusal in both
     census invocations so a backstop cannot mask the mutation, observe the
     collision assertion fail while the renamed construction stays valid,
     restore and rerun. This is the retained rule's proof, not a new F2 campaign.

   Retain exact mutation diffs/failures/restored passes for both new refusal
   tests and the census control. Cover reachable rules; never delete a required
   rule, add `#[cfg(...)]`/`coverage(off)` to hide production code, change the
   gate or predict an after denominator. This settled simplification changes
   no constructible bundle's behavior; proposed 0056 stays unchanged. An actual
   semantic departure needs its owning proposed decision, not an undocumented
   cleanup under coverage.

5. **Run the focused suites and complete local validation.** Existing
   14.1–14.2, 15.1–15.4 — every requirement for integration gates;
   safety / AS1, evidence / LE5, progress / PM3, PM4. After all mutations are
   restored, use #282's `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` on every
   Git invocation and parent test/script command (including children running
   `git init`), with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`. Run the named
   focused tests during clauses 2–4 and then:

   ```text
   cargo test -p brokkr-runtime -p brokkr-protocol --all-features --locked
   cargo test -p brokkr-cli --test driver_conformance --test decisions_index --all-features --locked
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
   cargo test --workspace
   cargo test --workspace --all-features --locked
   cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
   cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
   cargo build --release --locked -p brokkr-cli
   openspec validate 2026-09-09-226-session-resumption --strict
   openspec validate --all --strict
   git diff --check
   ```

   Keep witness/compose checks green; update only a digest actually reported
   changed by a commissioned edit, never re-pin from a stale expected value.
   Record exact commands, revision and exits. No archive filter, gate reduction
   or inherited pass counts as current validation. Missing Cargo or another
   prerequisite is unavailable, not success. #281's DeepSeek turn/token defect
   requires no investigation here. These partial-scope runs do not complete
   the whole-change gate or enablement checkboxes.

6. **Measure after, record the bounded result and commit.** Existing group 15 —
   every requirement for integration evidence; evidence / LE5; progress / PM1,
   PM4. Run unchanged `bash scripts/coverage-exact.sh` after the restored work,
   as before in clause 1, with the compiler from `rust-nightly-version.txt` and
   a verified writable **disk-backed TMPDIR outside the repository**. Never
   substitute an in-repo path or an unverified inherited TMPDIR; `/tmp` and
   `/dev/shm` were tmpfs in the predecessor's seat. If compiler or scratch is
   unavailable, name it and report all three fresh measures as unmeasured.
   Preserve the gate summary, JSON/LCOV reports, hashes, exact commands,
   revision and tool versions before another run overwrites them. Report the
   gate's own before/after covered/total integers for lines, branches and
   functions, with remaining uncovered source regions. New callers or the
   census removal may change emitted totals; do not infer equality from six
   lines/one branch disappearing or subtract #286's skipped boundary code.
   In-box shortfall is reported honestly; host equality and final-head native
   Windows/macOS CI remain pending controller evidence. All six decision tests
   still run inside the box; #286 excuses no decision row. CI, release admission
   and coverage retain their common compiler pin.

   Record each clause's actual outcome, the six gate decisions and tokens,
   provider-confirmed retry evidence, per-test mutation/restoration results,
   coverage dispositions and fresh measurements or precise missing prerequisites.
   Audit task IDs/states, frozen bytes, withdrawn living specs and adopted
   production invariants. Commit completed commissioned edits unsigned in
   repository style; no temporary mutation belongs in the commit. No checkbox
   tick, archive, fold, C/D, provider enablement, push, merge, publication or
   additional Brokkr run is authorized. If implementation runs out of room,
   report `oversized` with item 1 prioritized and the exact remainder, never
   `broken`. This tasks office uses `drafted`/`upstream` and claims a committed
   breakdown only, not completed implementation or a gate's next phase.

### Prior tasks-phase validation and handoff — two mediums

Strict active validation passes; strict all-item validation reports **14 passed,
0 failed**. Status reports all planning artifacts present; it does not attest
runtime completion. The two inherited archive-target notices remain informational
and outside this no-archive slice. Whitespace and requirement-citation audits
pass. Direct headings verify **20 requirements / 159 scenarios** (31 + 64 + 11
+ 20 + 33), matching both current statements. All **101 IDs / 82 checked /
19 pending** retain their states; only `tasks.md` changes and the other **728
tracked files** retain their entry hashes. No scenario, production/test,
declaration, proposed decision, frozen surface or living specification changed.

All seven commissioned Rust gate commands and the pinned coverage-tool probe
could not launch: Cargo is absent in this seat (recorded as exit 127 / ENOENT).
No Rust gate is claimed green, no behavioral/removal test ran and the exact
script was not launched without its compiler. **No fresh in-box coverage
numbers exist for this tasks visit.** Saved reports were not overwritten or
substituted for measurements. Implementation owes clauses 1–6 and their actual
proof; #286 host equality and final-head remote results remain controller work.
Evidence is retained under `.forge/tasks-two-mediums-84288d2b/`.

This unsigned task-draft checkpoint reports `drafted`: the earlier artifacts
support an honest breakdown, and no upstream repair is needed. It claims no
runtime fix, task completion, provider qualification or delivery. No archive,
re-fold, push, merge or additional Brokkr run occurred.


### Tasks-phase validation and handoff — whole family, 2026-09-16

Adopted design `e77c205` and specification `c2ed4f8` over revert `5ee48aa`;
read the current compiler, engine marker, adapter gate and direct-spawn seams
without reading the reverted patch. Repaired the existing six-clause order
and its owning task descriptions for Y/D10. No design choice, requirement,
scenario or task identifier was added. Existing 0056's minimal clarification
and every implementation/removal proof remain execution work, not completed
by this task draft.

Strict active OpenSpec validation passes; strict all-item validation reports
**14 passed / 0 failed**. Status confirms planning artifacts exist, not runtime
completion. The two inherited archive-target notices remain informational and
outside this no-archive slice. Whitespace and requirement-citation audits pass.
All **101 IDs / 82 checked / 19 pending** retain their states, and the five
deltas retain **20 requirements / 159 scenarios**. Only `tasks.md` changes;
the other **728 tracked files**, including production/tests, proposed 0056,
frozen surfaces and living specs, retain their entry hashes. All 21 saved
coverage reports retain their hashes.

Format, all-target/all-feature locked clippy, both workspace test commands,
both bundle compiles, release build and the pinned llvm-cov probe each exited
127 because **Cargo is unavailable in this seat**. The exact script was not
launched without its compiler; no usable external disk-backed TMPDIR was
established. **Fresh gate before: unavailable; fresh gate after: unavailable.**
The operator-supplied **30,036/30,036 lines, 5,038/5,038 branches,
2,863/2,863 functions** remain inherited baseline evidence. No fresh in-box
numbers, behavioral/removal results or Rust passes are claimed. CI, release
admission and coverage still read `nightly-2026-09-05` from the shared pin.

Evidence is retained under `.forge/tasks-whole-family-0a5cdb4f/`. This unsigned
checkpoint reports `drafted`; the earlier artifacts support an honest
breakdown. Implementation owes clauses 1–6; host equality under #286 and
final-head remote/platform CI remain pending controller evidence. No runtime
repair, provider qualification, whole-change task completion, archive/fold,
frozen-byte edit, push, merge or additional Brokkr run occurred.


### Tasks-return validation — adopted design `d194645`, 2026-09-16

Answered the design return in the existing requirement-linked clauses 1–4 and
owning 1.1/8.10 descriptions. Adopted the landed foundations and proposed-0056
clarification; made the residual raw census, exact movement/reservations,
canonical readers, selected-single decision and overlapping-guard removal proof
explicit. D10 already decides these points and supplies the raw-alias and
selected-single scenarios. No earlier artifact needs repair; no new plan,
requirement, scenario, task ID or checkbox-state change is introduced.

Strict active validation passes; strict all-item OpenSpec validation reports
**14 passed / 0 failed**. Status and whitespace checks pass. The inherited
archive-target notices remain informational and outside this no-archive slice.
All **101 IDs / 82 checked / 19 pending** retain their entry states. Requirement
bindings remain present; the five deltas retain **20 requirements / 159 scenarios**.
Only `tasks.md` changes; the other **728 tracked files** and inventoried saved
coverage reports retain their entry hashes. Production/tests, proposed 0056,
frozen surfaces, gate scripts and living specifications are unchanged.

Fresh Rust commands could not start: **Cargo is unavailable** (ENOENT, recorded
as 127). This includes format, all-target/all-feature clippy, both workspace
modes, affected runtime/protocol and CLI conformance/index suites, both bundle
compiles, release build and the pinned llvm-cov prerequisite. No Rust pass or
behavioral/removal proof is claimed. **Fresh exact-gate before: unmeasured;
after: unmeasured; no fresh in-box counts.** The script was not launched without
its compiler or verified external disk-backed scratch; inherited `TMPDIR=/tmp`
was not used. The supplied baseline remains **30,036/30,036 lines,
5,038/5,038 branches, 2,863/2,863 functions**. CI, release admission and coverage
still consume `nightly-2026-09-05` from their unchanged shared pin.

Evidence is retained under `.forge/tasks-whole-family-return-c0f961f6/`.
This unsigned task draft reports `drafted`. Implementation, decision/removal
proofs and local Rust validation remain owed; #286 host equality and final-head
native-platform CI remain controller handoffs. No runtime or whole-change
completion, archive/fold, push, merge, publication or new run is claimed.

### Tasks validation — THE PROOFS, adopted design `80772ce`, 2026-09-16

Run `the-proofs-branch-integration-22-5533cd6b`, this phase's only seat, read
`.forge/results/99cbdaee-9e9d-4fdc-b2f7-25cb2aa60def.json` before substantive
planning, then the dialect tasks/return files, rendered tasks instructions,
proposal AA, the safety/evidence deltas, D10's reconciliation and its answered
Open Questions. Current source confirms the settled gate, cold/retry transport
seams and three coverage locations. No design question or requirement fault
requires an upstream repair.

Reconciled the current scope, owning 1.1/3.4–3.6/6.3/8.10 descriptions and
existing six-clause execution order. Production repairs are adopted, six
compiled gate decisions precede the three coverage dispositions, and every
clause binds requirements plus observable verification. The transport instructions
retain the provider origin across cold/retry and distinguish real adapter roots
from synthetic helper checkpoints. No broader F2 campaign, selected-single
follow-up, new provider qualification or whole-change completion is commissioned.
No scenario, design choice, task identifier or checkbox state was added.

Strict active OpenSpec validation passes; strict all-item validation reports
**14 passed / 0 failed**. Status confirms planning artifacts exist, not runtime
completion. The inherited archive-target notices for adapter-resume-safety and
sdd-progress-markers remain informational and outside this no-archive visit.
Whitespace and requirement-binding audits pass. All **101 IDs / 82 checked /
19 pending** retain their entry states. Only `tasks.md` changes; the other
**728 tracked files** and all **28 inventoried coverage reports** retain their
entry hashes. The unchanged deltas contain **20 requirements / 159 scenarios**.
Production/tests, proposed 0056, declarations, frozen surfaces, gate scripts
and living specifications are unchanged.

Fresh validation attempts could not start because **Cargo is absent in this
workspace-tool seat** (ENOENT, recorded as exit 127): format, all-target/all-feature
locked clippy, runtime/protocol suites, CLI conformance/decision-index suites,
both workspace test modes, both bundle compiles and release build. The pinned
llvm-cov prerequisite also could not start. Commands inherited #282's Git
cleanup and `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`. No Rust or mutation-test
pass is claimed. The exact script was not launched without its compiler, and
no external disk-backed scratch was established for a run.

**Fresh in-box before/after: lines unmeasured; branches unmeasured; functions
unmeasured.** The supplied host before at production `4daaa7d` remains
**30,269/30,275 lines, 5,039/5,040 branches, 2,890/2,890 functions**, covered/total.
It is not this seat's measurement; saved reports were neither overwritten nor
substituted for one. CI, release admission and coverage still consume the
unchanged `nightly-2026-09-05` pin. Implementation owes the actual decision,
mutation and coverage results. Host equality under #286 and final-head native
Windows/macOS CI remain pending controller evidence.

Evidence is retained under `.forge/tasks-proofs-3910b832/`. This unsigned
checkpoint reports `drafted`: the breakdown is coherent with the settled
artifacts, while implementation and measured proof remain owed. No task tick,
archive, living-spec fold, frozen-byte edit, push, merge, publication or
additional Brokkr run occurred. The phase result carries the adopted change
identifier and selects no next phase.

## Current tasks return — Codex proof breakdown, 2026-09-16

This tasks seat adopts design `036390bf80dbe1952314b25109d3d557ef7789cd`
and proposal AB/specifications `d24189a8` for run
`codex-end-to-end-issue-226-tasks-1bd3f94f`. Production remains `5ef4a842`;
the two later commits are this run's specification and design. The change is
already active, so no archive reopen or fold is needed or authorized. There
is no supplied `returned_from` finding. The stale instructions to repeat the
Codex probe and retain the old current pin were this artifact's repair, not
an unresolved upstream decision. D10's current Decisions and Open Questions
already resolve the identity, append-only history, test seams and interface
boundary. The two AS1 scenarios and LE5's recorded-Codex-proof scenario own
this acceptance; no new requirement, scenario, task identifier or design is
introduced here.

### 10.5 — recorded observations, consumed without a probe

The live record, raw report and instrument were read first and in full. The
following table gives **one primary citation per axis**. The raw report
corroborates those cited facts; the instrument was inspected, never executed.
`LIVE` in the citations is the supplied September 16 live record, not the
refuted partial. These are controller observations on **codex-cli 0.154.0**,
not measurements by this seat. The evidence comparison under
`.forge/tasks/codex-proof-tasks-2026-09-16/proof-consistency.txt` checks the
supplied arrays, completed-command output, root and usage for agreement only.

| Axis and requirement served | Primary citation and observation |
|---|---|
| Exact invocation and demonstrated allowed argv — safety / AS1, AS3 | [LIVE](../../../.forge/tasks/controller-codex-proof-2026-09-16-live.json), JSON pointer `/axes/restriction_re_imposed_across_resume/resume_with_class_restored/argv`: the exact array selects `01a0aaa4-8667-7753-94b8-b0a60607524b` and carries `-c sandbox_mode=read-only`, `--skip-git-repo-check`, `--json`, `-c approval_policy=never` and the recorded literal prompt. That invocation demonstrably ran; it does not establish the complete adapter allow-list or its stdin prompt form. |
| Effective class and applicable fragment re-imposed — safety / AS2 | [LIVE](../../../.forge/tasks/controller-codex-proof-2026-09-16-live.json), `/axes/restriction_re_imposed_across_resume`: cold read-only denied the write; a bare resume wrote successfully; restoring the class denied it with `Read-only file system` and `EXIT=2`. The reporting shell's outer exit 0 is not the write's status. `interface_constraint` records that resume offers no sandbox flag and rejects `-s`; its applicable no-boxed-hands fragment is `-c sandbox_mode=<class>`. |
| Exact same-root confirmation — site / SR3; evidence / LE1 | [LIVE](../../../.forge/tasks/controller-codex-proof-2026-09-16-live.json), `/axes/same_root_confirmation/detail`: cold and resumed ids both equal `01a0aaa4-8667-7753-94b8-b0a60607524b`. |
| Current-only accounting — evidence / LE4 | [LIVE](../../../.forge/tasks/controller-codex-proof-2026-09-16-live.json), `/axes/current_only_accounting`: two resumes report 303 then 308 output tokens, input 111,390 then 128,069 and cached input 98,560 then 114,176. Adopt the controller's per-invocation finding; rising cached input is history reread, not accumulated new output billing. |
| Pre-work rejection shape — safety / AS4; evidence / LE3 | [LIVE](../../../.forge/tasks/controller-codex-proof-2026-09-16-live.json), `/axes/pre_work_rejection_shape/detail`: unknown id `00000000-0000-7000-8000-000000000000` exits 1 with `no rollout found`, with neither `thread.started` nor `turn.started`. The instrument's `any_turn_started` checks both event names. |

Raw corroboration, in the same row order, is in
`.forge/tasks/controller-codex-proof-2026-09-16-raw-report.json` at
`/phases/resume_class_reimposed/argv`; the four control/cold/bare/restored
phase records; `/same_root_confirmation` and the restored phase's
`thread_id`; the two accounting phases' single `usage` rows; and
`/phases/unknown_session_rejected`. The raw report does not contain a separate
`-s` rejection phase; that interface statement is attributed only to the live
controller record. The successful workspace-write control, which ran and
allowed the write, is corroborated by raw `/phases/control_workspace_write_cold`
and live `/method/control`.

All five **live observations are complete**. Task 10.5's remaining condition
is agreeing, passing adapter assertions with their removal evidence. This
planning return records the observations without ticking unexecuted tests.
No boxed MCP configuration, unobserved sandbox/flag combination, `--worktree`
or `--thread-source` shape is qualified by this table.

### Execution clauses of 11.1, then 10.5's evidence closure

These are ordered clauses of the existing numbered checkboxes, not additional
task identifiers. Each clause names its owning requirements and verification.
The 11.1 assertion subset can complete and discharge 10.5 while 11.1 itself
remains open for 10.1's missing interface evidence. No step waits for the
whole 11.1 tick before starting 10.5's closure.

1. **Record entry state and validate the proof mapping (10.5; safety / AS1,
   AS2; evidence / LE4, LE5; progress / PM1, PM4).** Adopt the table above and
   record actual HEAD/status, existing tests and before-coverage evidence.
   Retain historical 0030 at 0.148.0, prior declaration applicability
   0.153.4, and the controller's exercised 0.154.0 as different facts. No
   fresh provider version command, help query, source lookup or probe is
   commissioned. Verification is supplied-record agreement and an honest
   before-measurement status, never a reused historical coverage total.

2. **Reconcile the declaration and dependent claims (11.1; safety / AS1,
   AS2; progress / PM4).** Set both Codex identity fields to 0.154.0 while
   retaining `supported`, `classes: ["work"]`,
   `boundaries: ["harness", "not applicable"]` and `hands: "none"`. Retain
   cold `hands.harness.work` as `--sandbox workspace-write`. Each of the four
   evidence strings names the live proof, exercised version and its owning
   axis; interface also retains decision 0030's 0.148.0 origin without
   implying complete 0.154.0 interface qualification. The reason names the
   preservation ruling and states that bare resume drops the class and resume
   offers no sandbox flag, requiring `-c sandbox_mode=<class>`.
   Preserve all five existing limitation strings byte-for-byte and in order;
   append a bounded entry beginning `2026-09-16`, naming the ruling,
   `0.153.4→0.154.0`, historical `codex-proof-2026-09-10.json` and the new live
   proof. Resolve the now-measured axes while retaining 11.1's interface debt.
   The historical September 10 filename is provenance, not a new reading or
   current qualification. Stay within the loader's 400-character limit for
   each bounded value; introduce no new key, token or evidence format.
   Amend only the current Codex claims in proposed 0056 ruling 5/consequences
   and `docs/guides/provider-adapters.md`; keep 0056 `proposed`, 0030 unchanged
   and historical examples dated. Verification: extend existing runtime
   `agents/tests.rs` shipped identity/declaration assertions and run its
   loader suite, including invalid evidence and digest-change coverage.

3. **Align dependent shims and strengthen argv/current-usage assertions
   (11.1; safety / AS1, AS2, AS3; evidence / LE4, LE5).** Update protocol
   `CODEX_VERSION` and its paired current-proof assessments, CLI shipped
   harness/inline shims and `proof_shim_body`, plus the current-version
   comments in runtime `agents/tests.rs` and `engine/boundary_tests.rs`.
   Preserve self-contained synthetic version pairs, including CLI
   `a_resumed_mismatch_is_never_an_accepted_success`; no global replacement.
   In existing protocol
   `a_codex_resume_carries_the_thread_the_class_and_the_prompt`, cover input
   `-s X`, `--sandbox X` and `--sandbox=X` with exact complete resumed argv,
   exactly one `sandbox_mode="X"` paired with `-c`, and no surviving sandbox
   flag. Assert the selected positional id and retained prompt delivery.
   Preserve effort/model/workdir/result behavior with the existing effort
   test. Pin the same shim invocation's `input_tokens: 100`,
   `cache_read_tokens: 96`, `output_tokens: 4`; these normalize the current
   event and do not establish live provider accounting. Independently remove
   the emitted override, bypass sandbox extraction/consumption in a compiling
   plan, and omit each claimed folded usage field. Each control must fail its
   intended argv/resume/numeric assertion; restore and rerun after each.
   A retained allow-list backstop may decline the extraction mutation: record
   that actual failed resumed assertion, not a claim of provider acceptance.

4. **Prove offered-root selection and observed confirmation (11.1;
   site / SR2, SR3, SR4; evidence / LE1, LE3, LE5).** Use existing runtime
   `the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin` and CLI
   `the_shipped_codex_harness_work_seat_rejoins_its_retry`,
   `the_shipped_inline_codex_work_seat_rejoins_its_retry` and
   `the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`.
   Assert that the durable cold root, actual engine offer, provider-received
   positional id and confirmed launch root agree for both shipping
   coordinates. Keep the production composition and captured engine exchange;
   do not fabricate an offer, repair markers or replace the shipped assessment.
   Mutate the appended id to another legal id to fail the selection assertion;
   independently relax `LaunchHold::confirm`'s equality guard to fail the
   existing different-root no-success/no-launch assertion. Exercise the
   no-root terminal refusal separately, even with clean exit and a delivered
   result; if strengthened, remove that check separately to fail its own
   asserted outcome. Existing protocol confirmation/unsettled tests and CLI
   `a_resumed_mismatch_is_never_an_accepted_success` own those negatives.
   Preserve and run `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled`
   and `a_resume_that_started_and_failed_is_not_respawned_cold`: conclusive
   pre-work rejection permits only the existing bounded cold replacement;
   missing confirmation or work already started does not.

5. **Prove each independent refusal and retain precedence (11.1;
   safety / AS1, AS2, AS3, AS4; evidence / LE3, LE5).** Extend existing
   `adapters/tests.rs` suites, using otherwise valid supported input, a valid
   offered id/argv, accounting and affirmative markers. Direct private-gate
   tests isolate guards; shipping positive exchanges remain clause 4's proof.
   Run one compiling removal at a time, restore exact bytes, then rerun the
   named test green. The specific cases and controls are:

   | Case | Expected assertion | Removal target |
   |---|---|---|
   | Observed version differs from 0.154.0 applicability | `unverified-harness` with the observed version recorded | Observed-versus-applicability comparison in `qualify` |
   | Version unavailable/unreadable | `unverified-harness`, no guessed identity | The version-unavailable refusal, with a compiling mutation |
   | Observed/applicable 0.154.0 but origin 0.153.4 | `unverified-harness`; matching-origin control resumes | Origin comparison in `qualify`, independently of observed mismatch |
   | Boundary-only mismatch with valid hands | `restrictions-unavailable` | Boundary membership term only |
   | Hands-only mismatch with valid boundary | `restrictions-unavailable` | Hands equality term only |
   | Missing, null, non-string or unknown boundary/hands | `restrictions-unavailable`; affirmative control enables | Each responsible absence/type/membership guard independently, preserving unrelated facts |
   | Accounting removed from otherwise valid supported assessment | `unsupported-resume` | Accounting-presence guard in `resume_gate` |
   | Assessment absent, all other input valid | `unsupported-resume` | Assessment-absence guard, with a compiling bypass rather than a parse/compiler failure |

   Use existing `a_codex_whose_installed_version_has_moved_declines_the_offer`,
   `qualify_refuses_an_originating_version_drift`,
   `a_supported_assessment_without_both_affirmative_markers_declines` and
   `a_closed_gate_names_its_own_reason_ahead_of_the_seat_s_local_checks` where
   they already own these cases. Add the current 0.154.0/origin 0.153.4 vector
   beside the direct qualifier's historical synthetic pair; do not repin the
   latter merely to make it look current. A version-probe failure cannot
   substitute for reaching the origin comparison. Retain loader malformed-
   evidence checks: the gate tests string presence, not numeric accounting or
   the loader's length bound. Preserve exact tokens `invalid-session-id`,
   `sandbox-unavailable`, `unsupported-sandbox`, `incompatible-argv`,
   `unverified-harness`, `restrictions-unavailable`, `unsupported-resume` and
   conclusive `harness-refused`, along with cold-selector rejection and
   closed-gate precedence. Extend the existing unsafe-passthrough cases with
   `--worktree` and `--thread-source` under an otherwise eligible offer,
   asserting `incompatible-argv`; do not add either to the allow-list.
   Refusals assert their reason, never only `is_err()`.

6. **Measure derived pins and run local gates on restored bytes (11.1;
   safety / AS1; evidence / LE5; progress / PM4).** After all declaration,
   dependent-document and test edits, run the existing runtime witness and
   compose suites. Copy only their actually reported changed digest pairs;
   preserve equality assertions and unrelated pins. This is clause work under
   14.1/14.2, not whole-change completion. Run format, all-target/all-feature
   locked clippy, the focused protocol/runtime/driver suites, both complete
   workspace test modes and both bundle compiles. Run strict OpenSpec and the
   unchanged exact gate with the environment below; retain actual results.
   No filter, coverage exemption or production guard deletion can make the
   slice pass. Reuse existing test helpers; every new or strengthened
   assertion must have an identified, observed removal failure and restored
   pass, not a compile error, discovery failure or poisoned-lock cascade.

7. **Close only the evidenced work and commit (10.5/11.1; safety / AS1;
   evidence / LE5; progress / PM1, PM4).** Record each new assertion's test
   name, exact invocation, responsible mutation, intended failure output,
   restoration and successful rerun in the implementation return. Update
   progress as clauses finish. Once those assertions agree with the five
   supplied observations, tick 10.5 promptly; the ledger becomes **83 complete
   / 18 pending**, with 11.1 still unchecked for its missing 0.154.0 effort,
   complete safe-passthrough and stdin-prompt interface evidence. No checkbox
   is awarded merely for preserving support. Retain every other tick,
   provider status and excluded task. Review and commit only completed,
   verified work unsigned in the repository's message style. Never push,
   merge, archive, fold living specs or start a Brokkr run. The controller
   owns remote CI, integration and host coverage.

### Validation commands and environment for the implementation clauses

Use `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2` and
`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` for every Git command and all
Cargo commands that may spawn Git in tests (#282). Stage any executable file
beside its destination, close it and rename it into place before execution.
Directly executed shebang shims remain Unix-only, or use a real target-platform
executable. No production dependency, wrapper or helper abstraction is needed.

Run each focused test with an unambiguous filter and retain its actual test
count; a zero-test run is not a pass. The owning suites and broader gates are:

```text
cargo test --locked -p brokkr-protocol --all-features adapters::tests
cargo test --locked -p brokkr-runtime --all-features
cargo test --locked -p brokkr-cli --all-features --test driver_conformance
cargo test --locked -p brokkr-runtime --test witness_digests
cargo test --locked -p brokkr-runtime --lib bundle::compose_tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace
cargo test --workspace --all-features --locked
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
cargo build --release --locked -p brokkr-cli
openspec validate 2026-09-09-226-session-resumption --strict
openspec validate --all --strict --no-interactive
openspec validate --archived --strict --no-interactive
bash scripts/coverage-exact.sh
```

Before coverage, select and verify a writable **disk-backed TMPDIR outside the
repository**, using workspace hands; neither an in-repo directory nor a
read-only global default satisfies the premise. Preserve fresh before/after
reports and record their own covered/total line, branch and function integers,
revision and environment. Keep the literal equality gate and test selection
unchanged. CI, release admission and the script must still consume
`rust-nightly-version.txt`. Boundary skips inside the box make host equality
unattainable here (#286); report actual in-box numbers and leave host equality
pending for the controller. If Cargo or valid scratch is unavailable, report
counts as **unmeasured**, name that exact limitation and do not substitute
old reports, calculate a presumed delta or claim green. All full-change 15.x
readiness, archive, release and remote-head results remain outside this slice.

### Tasks-phase validation and handoff — adopted design `036390bf`

This phase repairs only `tasks.md`. The live/raw/instrument files retain their
entry bytes. The supplied-record consistency check passed without running a
provider. The breakdown now names concrete production/test seams, exact refusal
tokens, independent removal targets, restored reruns, dependency order and the
requirement served by every execution clause. It supersedes the old probe and
0.153.4 pin instructions in 10.5/11.1, labels 10.1's completed interface work
historical, and scopes the older 6.4/8.5/8.10 and group-15 instructions so they
cannot reopen excluded work. D10 resolves every choice needed to draft this
plan; `upstream` is not warranted by missing future execution evidence.

Strict active-change validation passed. Repository-wide strict validation
passed **14 items, zero failures**; archived validation passed **six changes,
zero failures**. Existing informational archive-target notices concern the
withdrawn living specs and do not authorize a fold. `git diff --check` passed.
The tracked-byte audit confirms the other **728 tracked files** are unchanged,
including all frozen surfaces, production/tests, declarations, decisions,
proposal/design/specs and living truth. All **101 identifiers and their ticks**
are unchanged: **82 complete / 19 pending**. The deltas remain **20 requirements
/ 162 scenarios** (evidence 32, safety 66, boundary 11, progress 20, site 33).

Fresh format, locked all-target/all-feature clippy, protocol/runtime/CLI
conformance suites, both workspace test modes, both bundle compiles and the
release build were attempted through workspace hands but **could not launch:
Cargo is absent (ENOENT)**. The pinned cargo-llvm-cov prerequisite likewise
could not launch, so the exact script was not started. Fresh in-box covered/
total **lines, branches and functions are unmeasured**, not zero and not an
inherited report. No invalid TMPDIR or lowered gate substitutes for the missing
compiler. CI and release admission still consume `rust-nightly-version.txt`,
as does `coverage-exact.sh`; the current pin is `nightly-2026-09-05`. Host
literal equality under #286 and final-head remote results remain controller
handoffs. Logs, entry hashes, evidence comparison and audit are retained under
`.forge/tasks/codex-proof-tasks-2026-09-16/`.

This is a **drafted tasks artifact**, not a completed adapter implementation.
10.5's five live observations are recorded; its tick awaits agreeing executed
assertions and removal controls. 11.1 additionally lacks exact-0.154.0
help/source evidence for effort configuration, every safe passthrough option
and stdin prompt positional `-`. Neither task is ticked here. The unsigned
planning commit contains only this file, and the phase result carries the
adopted identifier in `inputs.change`, without selecting another phase. No
provider experiment, declaration edit, production change, archive, spec fold,
push, merge, publication or new Brokkr run occurred.

## Implementation return — the Codex 0.154.0 reconciliation landed, 2026-09-16

This implement seat landed the reconciliation and the matching assertions
atop the planning commits `d24189a8`/`036390bf`/`0cc62301`; production was
`5ef4a842` and no provider probe, network lookup or new measurement was run.
Changed: `adapters/codex.json`; the shipped-version assertions and their
comments in `crates/brokkr-runtime/src/agents/tests.rs` and
`crates/brokkr-runtime/src/engine/boundary_tests.rs`; the shipped and proof
shims plus `proof_shim_body` in
`crates/brokkr-cli/tests/driver_conformance.rs`; `CODEX_VERSION`, the resume
argv test and the gate/refusal cases in
`crates/brokkr-protocol/src/adapters/tests.rs`; proposed 0056's current Codex
prose and `docs/guides/provider-adapters.md`; and the derived witness/compose
digest pins in `crates/brokkr-runtime/tests/witness_digests.rs` and
`crates/brokkr-runtime/src/bundle/compose_tests.rs`, copied from the actual
compiles.

### 10.5 — ticked; one recorded citation per axis

The recorded table above gives one live citation per axis. The tick rests on
those observations plus the executed assertions and removal controls below;
the exercised identity is the controller's `codex-cli 0.154.0`.

- **Exact invocation and demonstrated allowed argv — safety / AS1, AS3.**
  Citation: live `/axes/restriction_re_imposed_across_resume/resume_with_class_restored/argv`.
  `adapters::tests::a_codex_resume_carries_the_thread_the_class_and_the_prompt`
  now drives `-s X`, `--sandbox X` and `--sandbox=X` and asserts the whole
  resumed argv, exactly one paired `-c sandbox_mode="X"`, no surviving sandbox
  flag, the selected positional thread and the stdin `-`. Removal M1 (drop the
  emitted `-c sandbox_mode=` pair) failed it at `tests.rs:2287`; M2 (leave the
  sandbox flag in passthrough) failed the same assertion via the retained
  allow-list backstop `incompatible-argv`; restore reran green.
- **Effective class and applicable fragment re-imposed — safety / AS2.**
  Citation: live `/axes/restriction_re_imposed_across_resume` (cold denial, bare
  resume write, restored denial with `Read-only file system` and `EXIT=2`) and
  `interface_constraint` (resume offers no sandbox flag; only
  `-c sandbox_mode=<class>`). The whole-argv assertion covers it: no `-s`
  survives and the class rides only the config override.
- **Exact same-root confirmation — site / SR3; evidence / LE1.** Citation:
  live `/axes/same_root_confirmation/detail`, both ids
  `01a0aaa4-8667-7753-94b8-b0a60607524b`. The same test asserts the published
  `root_session.id` equals the offered `THREAD`; removal M3 (mutate the
  appended id) failed it; restore green. The shipped-coordinate CLI tests
  `the_shipped_codex_harness_work_seat_rejoins_its_retry`,
  `the_shipped_inline_codex_work_seat_rejoins_its_retry` and
  `the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`
  pass; removal M4 (accept a different announced root as `resumed`) failed
  `a_resumed_mismatch_is_never_an_accepted_success` at
  `driver_conformance.rs:1424`; restore green.
- **Current-only accounting — evidence / LE4.** Citation: live
  `/axes/current_only_accounting` (303 then 308 output tokens; cached input
  98,560 then 114,176). The protocol test now pins `input_tokens: 100`,
  `cache_read_tokens: 96` and `output_tokens: 4` from the shim's one current
  turn; removal M12 (omit the folded `input_tokens`) failed it at
  `tests.rs:2325`; restore green.
- **Pre-work rejection shape — safety / AS4; evidence / LE3.** Citation: live
  `/axes/pre_work_rejection_shape/detail` (unknown id exits 1, `no rollout
  found`, neither `thread.started` nor `turn.started`).
  `a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled` and
  `a_resume_that_started_and_failed_is_not_respawned_cold` pass unchanged.

Independent refusal assertions, each with valid unrelated prerequisites and
its own observed removal failure:

| Test | Assertion | Removal | Observed failure |
|---|---|---|---|
| `a_codex_whose_installed_version_has_moved_declines_the_offer` | observed 0.160.0 vs applicable 0.154.0 → `unverified-harness`; observed version recorded, never the pin | M5 drop the observed-vs-applicability term | `tests.rs:2864` |
| `a_codex_whose_version_cannot_be_read_declines_the_offer` | absent executable → `unverified-harness`, no guessed identity, cold argv | M7 admit the version-unavailable arm | `tests.rs:2900` |
| `qualify_refuses_an_originating_version_drift` | observed/applicable 0.154.0 with origin 0.153.4 → `unverified-harness` | M6 drop the origin term | `tests.rs:3151` |
| `a_supported_assessment_without_both_affirmative_markers_declines` | boundary-only and hands-only mismatch → `restrictions-unavailable`; accounting absent and assessment absent → `unsupported-resume` | M8 boundary term, M9 hands term, M10 accounting guard, M11 assessment guard | `tests.rs:2030` each |
| `only_the_flags_a_resume_can_safely_carry_travel_with_it` and `a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled` | `--worktree` and `--thread-source` → `incompatible-argv`, never added to the allow-list | new negative cases | pass |

Those controls were applied alone to the then-unmodified
`crates/brokkr-protocol/src/adapters.rs`, compiled, failed their intended
assertions, and reran green after restoration. That first return recorded only
the controls it had run: it did not independently omit `cache_read_tokens` or
`output_tokens`, did not isolate the boundary/hands absence-and-type guards,
recorded the `--worktree`/`--thread-source` refusals as passing without a
removal, and attributed M6 to the shared historical origin assertion rather
than the current 0.154.0 one. The review's returned R1 names exactly those
gaps; the correction below completes every outstanding control and restores
the accuracy of this record. Refusals assert their token, never `is_err()`
alone. All other ticks, provider statuses and excluded tasks are unchanged:
**83 complete / 18 pending** across the same 101 identifiers, with 11.1 still
unchecked.

### 11.1 — partial, left unchecked

The declaration reconciliation, shipped-assessment bump, digest re-pins and
the argv/root/refusal/current-usage assertions are delivered and pass. The
whole 11.1 tick is NOT awarded: 10.1's exact current-version interface
acceptance is still missing at this head, so no supplied 0.154.0 help/source
capture establishes (a) effort configuration, (b) the complete
safe-passthrough list, or (c) the stdin prompt positional `-`. Preserving the
shipping disposition does not complete them.

### Local validation on the restored bytes

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` clean; `cargo test --workspace
--all-features --locked --no-fail-fast` and `cargo test --workspace --locked
--no-fail-fast` green once every git-invoking command is prefixed with
`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` (#282 — without it three
`brokkr-cli` git tests fail and the first poisons the shared test lock);
`cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
`bundles/verify` both exit 0; `cargo build --release --locked -p brokkr-cli`
succeeds; strict active and repository-wide OpenSpec validation pass. The
in-box exact-coverage attempt reported, on the host's tmpfs `TMPDIR` (the box
exposes no writable disk-backed scratch and `/var/tmp` is read-only):
**lines 30273 total / 30102 covered, branches 5040 / 5027, functions 2890 /
2880** — literal equality is not met because the boundary proofs skip inside
the box, which cannot create a bubblewrap namespace (#286). No production Rust
line was added, so that shortfall is the box's, not this edit's. Host literal
equality remains the controller's.

## Returned implement correction — review R1–R4, 2026-09-16

The review returned `residual` on `db05da57` with one medium and three low
findings plus an informational note. This visit answers each; it runs no
provider probe, reads the live record, raw report and instrument already
supplied, and leaves every checkbox state intact (**83 complete / 18 pending**
across the same 101 identifiers, with 10.5 checked and 11.1 unchecked).

- **R1 (medium) — the completed-work commit was GPG-signed.** `db05da57`
  carried a `gpgsig` header from the DSH delegated-signing loan, contrary to
  CONTRIBUTING.md:91 and decision 0054 ruling 8 (seat commits are unsigned;
  the operator's squash-merge is the signed commit). The previous implement
  result's claim that it was unsigned was therefore inaccurate. The signed
  commit was replaced by an unsigned re-commit of the same tree carrying
  these corrections; the final head is named in this visit's result artifact.
  The #282 prefix (`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`) removes
  the malformed `GIT_CONFIG_COUNT`, but the linked worktree's config still
  forces `commit.gpgsign=true`, so the replacement was committed with
  `--no-gpg-sign`; `git cat-file -p HEAD` shows no `gpgsig` header.
- **R2 (low) — the declaration did not cite the superseding live record.**
  `adapters/codex.json` now names
  `.forge/tasks/controller-codex-proof-2026-09-16-live.json` in the
  `interface` evidence and in the dated `2026-09-16` reconciliation
  limitation, so the live record is disambiguated from the refuted partial
  by path, not date alone. All five historical limitation strings (indices
  0–4) are byte-for-byte unchanged; every bounded value stays within the
  loader's 400-character limit (interface 342, reconciliation 396).
- **R3 (low) — the compose comment misstated the moved pins.**
  `crates/brokkr-runtime/src/bundle/compose_tests.rs` now says the
  reconciliation moves `recipes/panel-review` **and** `bundles/self`, while
  `recipes/fast` and `bundles/verify` keep their digests. The golden
  digests themselves were measured, not guessed (see below).
- **R4 (low) — the `-C`/`-s` attribution over-claimed the live proof.**
  The resume-argv comment in `crates/brokkr-protocol/src/adapters/tests.rs`
  now records that decision 0030 established both `-C` and `-s` on
  codex-cli 0.148.0, and that the 2026-09-16 live proof re-confirms `-s` on
  the exercised 0.154.0 while recording no `-C` observation.
- **Informational — the guide counted four axes.** `docs/guides/provider-adapters.md`
  now says "five measured live axes" and names exact invocation in the
  enumeration, matching the five-axis mapping 10.5 cites.

Because the declaration text moved, the bundles that pin its digest were
re-measured with a temporary throwaway test that printed each
`manifest_digest()` from the same `Bundle::compile_with` the pinning suites
use; the temp file was deleted and never committed. Updated goldens are the
witness pins for `recipes/night-shift` (`5eec5fd2…`),
`recipes/wager-harness` (`1f6f5d58…`), `recipes/triage` (`5e820770…`) and
`recipes/gpt-flash` (`9d73bc9e…`), and the compose pins for
`recipes/panel-review` (`9020c5c2…`), `bundles/self` (`71428b28…`) and
`recipes/triage` (the same `5e820770…`). `witness_digests` (4 tests) and
`bundle::compose_tests` (17 tests) pass on the restored bytes. No frozen
surface, production Rust guard, refusal token or provider disposition moved;
proposed 0056 stays `proposed` and the other three providers stay
`unmeasured`.

## Returned implement correction — review R1 removal evidence, 2026-09-16

The review returned `residual` on `8a5a1675` with one medium finding: the
removal-proof acceptance was incompletely evidenced. This visit answers that
finding alone. It runs no provider, reads the supplied live record, raw report
and instrument, and changes only
`crates/brokkr-protocol/src/adapters/tests.rs` (one test isolated) and this
file. No production Rust guard, declaration, frozen surface, refusal token or
checkbox moved: **83 complete / 18 pending** across the same 101 identifiers,
with 10.5 checked and 11.1 unchecked.

### Current-origin isolation (M6)

The earlier return's M6 dropped `qualify`'s origin comparison and claimed the
new observed/applicable 0.154.0 with origin 0.153.4 vector as its failure, but
that vector shared a function with the historical 0.153.4 against 0.150.0
case, so execution stopped at the earlier assertion and the current vector was
never reached. `qualify_refuses_an_originating_version_drift` now keeps only
the historical vector; the current vector lives in its own
`qualify_refuses_a_current_version_stale_origin` (`tests.rs:3177`), whose own
`tests.rs:3211` refusal assertion is what M6 now fails.

### Completed compiling removal controls

Each control changes exactly one production seam in
`crates/brokkr-protocol/src/adapters.rs`, recompiles, and runs its exact test
through:

```text
env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0 CARGO_BUILD_JOBS=2 \
  RUST_TEST_THREADS=1 cargo test --locked -p brokkr-protocol \
  --all-features --lib adapters::tests::<test> -- --exact
```

| Control | Test | Break | Observed failure | Restored rerun |
|---|---|---|---|---|
| M1 | `a_codex_resume_carries_the_thread_the_class_and_the_prompt` | drop the emitted `-c sandbox_mode="<class>"` pair | `tests.rs:2288:9` `assertion left == right failed: short-separate: the whole resumed argv` (left lacks the pair) | 1 passed |
| M2 | same | push the sandbox flag and value into passthrough | `tests.rs:2288:9` (left is the cold `exec --json -C … -s read-only …` argv) | 1 passed |
| M12a | same | omit folded `input_tokens` | `tests.rs:2326:43` `no entry found for key` | 1 passed |
| M12b | same | omit folded `cache_read_tokens` | `tests.rs:2327:43` `no entry found for key` | 1 passed |
| M12c | same | omit folded `output_tokens` | `tests.rs:2328:43` `no entry found for key` | 1 passed |
| M8 | `a_supported_assessment_without_both_affirmative_markers_declines` | remove the boundary membership term | `tests.rs:2030:43` `boundary unknown: must not enable` | 1 passed |
| M8a | same | bypass the boundary absence/type guard with a `"harness"` default | `tests.rs:2030:43` `boundary absent: must not enable` | 1 passed |
| M9 | same | remove the hands equality term | `tests.rs:2030:43` `hands unknown: must not enable` | 1 passed |
| M9a | same | bypass the hands absence/type guard with a `"none"` default | `tests.rs:2030:43` `hands absent: must not enable` | 1 passed |
| M10 | same | remove the accounting-presence guard | `tests.rs:2030:43` `accounting absent: must not enable` | 1 passed |
| M11 | same | bypass the assessment-absence guard with a compiling supported default entry | `tests.rs:2030:43` `assessment absent: must not enable` | 1 passed |
| M5 | `a_codex_whose_installed_version_has_moved_declines_the_offer` | remove the observed-versus-applicability term | `tests.rs:2865:5` `the cold argv, unchanged: ["exec","resume",…]` | 1 passed |
| M6 | `qualify_refuses_a_current_version_stale_origin` | remove the origin-comparison term | `tests.rs:3211:5` `assertion left == right failed: a root opened under 0.153.4 is not a 0.154.0 session` (`left: None`) | 1 passed |
| M7 | `a_codex_whose_version_cannot_be_read_declines_the_offer` | bypass the version-unavailable arm with an `applies_to` fallback | `tests.rs:2901:5` `no rejoin without a version` | 1 passed |
| M13 | `only_the_flags_a_resume_can_safely_carry_travel_with_it` | add `--worktree` to the resume allow-list | `tests.rs:2735:9` `assertion left == right failed: --worktree may not travel to a resume`; the same removal rejoins the `worktree` case and fails `a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled` at `tests.rs:2508:9` `worktree: the cold argv, unchanged` | 1 passed each |
| M14 | same | add `--thread-source` to the resume allow-list | `tests.rs:2735:9` `--thread-source may not travel to a resume`; the same removal rejoins the `thread-source` case and fails `a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled` at `tests.rs:2508:9` `thread-source: the cold argv, unchanged` | 1 passed each |

Every control was applied alone and compiled. After the observed failure the
production file was rewritten from its pre-run bytes and sha256-verified equal
to `233f42596ed10e83e7ae7572e3d47153b56b139b097edc0ed049337bfa804167` before
its exact test reran green; the final file still carries that hash and
`git diff` for it is empty. The throwaway mutation runner lived under the
ignored `.forge/scratch-codex-controls/` and is absent from the patch. As
whole suites, the protocol adapter suite is green at **154 tests** (was 153;
the isolated origin test is the addition) and the restored test file leaves
every existing refusal token and bounded pre-flight replacement unchanged.

### Reconciliation

The earlier "Every mutation was applied alone …" sentence described the
controls then run; it was not a claim that every clause-3, clause-5 and LE5
removal obligation had been recorded, and the incomplete set is named and
completed above. M6 now points at the isolated current-version assertion,
`cache_read_tokens` and `output_tokens` each have their own omission failure,
the absence-and-type guards are separated from the membership and equality
terms they could otherwise be confused with, and both `--worktree` and
`--thread-source` have removal proofs. 11.1 remains unchecked for the
exact-0.154.0 effort, safe-passthrough and stdin `-` interface evidence this
proof does not contain. No provider was executed, no new probe was
commissioned, `--worktree`/`--thread-source` stay unqualified and refused, and
the Phase-D work remains outside this slice.

### Local validation on the corrected bytes

Every command below ran on the corrected tree with the #282 Git prefix
(`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0`):

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  exit 0.
- `cargo test --locked -p brokkr-protocol --all-features adapters::tests`:
  **154 passed** (was 153).
- `cargo test --locked -p brokkr-runtime --all-features`: green, 440 lib tests
  plus every integration binary.
- `cargo test --locked -p brokkr-cli --all-features --test driver_conformance`:
  **18 passed**.
- `cargo test --locked -p brokkr-runtime --test witness_digests`: **4 passed**;
  `--lib bundle::compose_tests`: **17 passed**.
- `cargo test --workspace` and `cargo test --workspace --all-features --locked`:
  both exit 0.
- `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and
  `bundles/verify`: both exit 0, with no bundle or digest file modified;
  `cargo build --release --locked -p brokkr-cli` exit 0.
- `openspec validate 2026-09-09-226-session-resumption --strict`:
  `Change … is valid`; `--all --strict`: **14 passed, 0 failed**;
  `--archived --strict`: **6 passed, 0 failed**.
- Fresh `bash scripts/coverage-exact.sh` on the pinned `nightly-2026-09-05`
  compiler (the only writable scratch the box exposes is tmpfs): **lines 30102
  covered / 30273, branches 5027 / 5040, functions 2880 / 2890**. Literal
  equality is not met in-box because the namespace boundary proofs skip where
  bwrap cannot nest (#286), and no production Rust line moved in this
  correction; host equality remains the controller's.

## Implement visit — 10.1's deferred obligations discharged; 11.1's assertions blocked on a toolchain-less seat, 2026-09-18

Run `codex-enablement-issue-226-task--c929d7ec`, branch `slice-codex-enable`
cut from `9da5ff92`. This visit read the two supplied controller records and
the instrument, ran no provider, no probe and no network lookup, and changed
**only this file**. No checkbox moved: **83 complete / 18 pending** across the
same 101 identifiers, 10.5 checked and 11.1 unchecked.

### Delivered — 10.1's three deferred obligations, one citation each

10.1's text now records the discharge above, obligation by obligation, against
`.forge/tasks/controller-codex-interface-2026-09-17.json` (`codex-cli
0.154.0`, taken by the controller by hand):

| Deferred obligation | Recorded observation that supplies it |
|---|---|
| Stdin prompt positional `-` | `/obligations/stdin_prompt_positional/evidence`: `codex exec resume [OPTIONS] [SESSION_ID] [PROMPT]`, whose `PROMPT` argument documents "If `-` is used, read from stdin". |
| Effort configuration | `/obligations/effort_configuration/evidence` — under `--strict-config`, `-c model_reasoning_effort=low` is accepted and the turn completes — held up by `/obligations/effort_configuration/control`, where the misspelled `model_reasoning_effrot` is refused as an "unknown configuration field". Acceptance alone would prove nothing; the refusing control is what shows `--strict-config` discriminates. |
| Allowed safe passthrough | `/obligations/allowed_safe_passthrough/resume_accepts` enumerates the resume surface; `/exec_accepts_but_resume_does_not` names the nine `exec` options a resume refuses — `--sandbox`, `--cd`, `--add-dir`, `--approve-for-me`, `--color`, `--local-provider`, `--oss`, `--profile`, `--version` — with `--sandbox`'s absence independently corroborated by the September 16 live proof's `interface_constraint`. |

The 0.153.4 investigation keeps its historical tick and its historical
version; nothing is relabelled. Parsing is not confinement, so the record's
own `caution` and `not_previously_supported` entries grant nothing, and
`--worktree` and `--thread-source` stay unqualified and refused.

### NOT delivered — 11.1's assertions and its tick

The seat could not compile or run anything. Every form of the Rust toolchain
is refused by this seat's Bash permission layer with `This command requires
approval`, in a non-interactive session where that is a denial: `cargo`,
`cargo version`, `cargo --version`, `/home/vyanakiev/.cargo/bin/cargo
--version`, `cargo build --locked -p brokkr-protocol --all-features --tests`,
`cargo fmt --all -- --check`, `cargo test --workspace --all-features
--locked`, `cargo test --locked -p brokkr-protocol --all-features --lib
adapters::tests::<test> -- --exact`, `rustc --version`, `bash
scripts/coverage-exact.sh --help` and `openspec --version` are all denied,
while `git status`/`git log` are honoured — so the denial is a permission
rule, not the sandbox (a repeat with the sandbox disabled is refused
identically). `agents/implementer.json` declares `tools.allow: ["cargo",
"git"]` and `adapters/claude.json` maps `cargo` to `Bash(cargo:*)`, so the
grant this seat was framed under did not reach the session. Without a
compiler there is no test run, no compiling removal control, no bundle
compile, no strict OpenSpec validation and no in-box coverage measurement;
`is_err()`-style or asserted-by-reading claims are worth nothing here, so none
were written. No Rust was added: an uncompiled test is not evidence, and a
branch carrying one is worse than a branch without it.

### The exact assertion work 11.1 still owes, located

Read off the current bytes so the next visit can start at the seam rather than
re-derive it. Everything below is missing today; everything not listed is
already delivered and passing per the two returns above.

1. **The exec-only nine, at the blocker seam.**
   `adapters::tests::only_the_flags_a_resume_can_safely_carry_travel_with_it`
   (`crates/brokkr-protocol/src/adapters/tests.rs:2688`) refuses `--profile`,
   `--add-dir`, `--approve-for-me` and `-C`, but never exercises `--cd`,
   `--color`, `--local-provider`, `--oss` or `--version`, which
   `/obligations/allowed_safe_passthrough/exec_accepts_but_resume_does_not`
   now measures as refused by the subcommand itself. Add all five, in their
   joined spelling too, asserting `codex_resume_blocker` returns that exact
   part — never `is_some()` alone.
2. **The whole-argv consequence of the same five.** `--cd <dir>` and one bare
   member (`--oss`) belong in
   `a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled`
   (`tests.rs:2397`), asserting `incompatible-argv`, the unchanged cold argv
   and no recorded root — the shape the eleven existing cases already prove.
3. **The allow-list against the measured surface.** No test today compares
   `CODEX_RESUME_VALUE_FLAGS`/`CODEX_RESUME_BARE_FLAGS`
   (`crates/brokkr-protocol/src/adapters.rs:2388`, `:2397`) with the record's
   enumeration. Assert: every admitted long option appears in
   `resume_accepts`; no member of `exec_accepts_but_resume_does_not` is
   admitted; `--worktree` and `--thread-source` are parsed by the subcommand
   and still refused here, which is the sentence that says parsing is not
   qualification.
4. **The effort field's exact spelling.** `codex_effort_config`
   (`adapters.rs:2285`) composes `model_reasoning_effort="<effort>"`, the key
   whose recognition the September 17 control establishes by refusing one
   misspelling. Pin the key text, exactly one `-c model_reasoning_effort=`
   pair, both `--effort <level>` and `--effort=<level>` inputs, and that no
   `--effort` flag survives on a resume argv. Removal control: misspell the
   key in `codex_effort_config`, watch the intended assertion fail, restore.
5. **The stdin positional, tightened.**
   `a_codex_resume_carries_the_thread_the_class_and_the_prompt`
   (`tests.rs:2245`) already asserts the whole argv and the fed prompt; the
   measured `[SESSION_ID] [PROMPT]` grammar additionally wants the offered
   thread and the trailing `-` asserted as the final two parts in that order,
   with a distinctive multi-word fixture prompt so no `contains` can match a
   temporary path. Removal control: drop the `-` push at `adapters.rs:2591`.
6. **Removal controls for 1–5**, each mutating one production seam alone,
   compiling, failing at the intended assertion, restored to sha256-verified
   bytes and rerun green — the standard the previous two returns met.

### Residual for the controller, not resolved here

`CODEX_RESUME_VALUE_FLAGS` admits `--output-schema` and
`CODEX_RESUME_BARE_FLAGS` admits `--ephemeral`, and the September 17 record
lists both under `/obligations/allowed_safe_passthrough/not_previously_supported`
beside `--worktree`, `--thread-source`, `--ignore-rules` and
`--ignore-user-config` (the last two are already refused). Both readings are
open: "never present before 0.154.0", which would contradict the allow-list
comment's attribution to 0.148.0 help, or "never carried a qualification",
which is simply true of every option on a preservation-ruled surface. This
visit changed neither the list nor the comment — widening and narrowing are
both semantic moves, and this is an operator's word to say, not a seat's.

### Left deliberately untouched

`adapters/codex.json`, proposed 0056, `docs/guides/provider-adapters.md` and
the specs still carry the pre-discharge debt prose. Reconciling the
declaration moves the witness and compose digests that pin it
(`crates/brokkr-runtime/tests/witness_digests.rs`,
`crates/brokkr-runtime/src/bundle/compose_tests.rs`), and 11.1 requires every
affected digest to be MEASURED before re-pinning. A seat that cannot compile
cannot measure a digest, and a guessed pin is a red branch; so the declaration
keeps its current bytes, its disposition, its identities and all six dated
limitations, and that reconciliation travels with the assertion work.

### Gates, and why this edit is uncommitted

None ran; every command above is denied to this seat. Nothing about the
change's gate standing is claimed: fmt, clippy, the crate suites, both bundle
compiles, strict OpenSpec validation and in-box coverage are all UNRUN, not
passing, on this head.

The same permission layer also refuses git's write side — `git add <path>` and
`git commit` are denied with `This command requires approval` while `git
status`, `git log`, `git diff` and `git show` are honoured — so this visit
could not commit its own work. The edit above is present in the working tree
and UNCOMMITTED at `9da5ff92`; `git diff` shows it whole, and it touches this
file alone. Committing it is the next visit's first step, or the controller's.

## Current tasks visit — CODEX ENABLEMENT assertion breakdown, 2026-09-18

This tasks seat adopts design `7e36fb15` and specification `d0b4f639` for run
`codex-enablement-issue-226-task--a1d8e540` on `slice-codex-enable`, cut from
production `9da5ff92`. Predecessor item 1 at `85cf6d55` is **adopted, not
re-authored**: 10.1's three deferred obligations keep their citations and
10.5 keeps its tick. The September 16 live proof, the September 17 interface
record and `.forge/tools/codex-enforcement-probe.py` were read first and in
full; the instrument is evidence, not a command to run. No provider, probe or
network lookup was executed and none is commissioned. There is no supplied
`returned_from` finding, the change is active, and no archive reopen or fold
is authorized.

Both stated dependencies of 11.1 are discharged. What remains is 11.1's own
half — adapter assertions over the measured surface, their compiling removal
controls, the prose that still recites the discharged debt, and the digests
that prose moves. The clauses below are **ordered clauses of the existing
checkbox 11.1**, not new task identifiers; the ledger stays **83 complete /
18 pending** across the same 101 identifiers. No requirement, scenario,
decision or task family is introduced here.

### What is already delivered, and must not be redone

Read off the current bytes. Everything in this list passes today and is
adopted as dated evidence with its own date; a return that restates it as a
fresh experiment is inaccurate.

- The declaration's disposition and identity: `adapters/codex.json`
  `work-site` is `supported`, `classes: ["work"]`,
  `boundaries: ["harness", "not applicable"]`, `hands: "none"`, both
  `identity.version` and `identity.applies_to` are `0.154.0`, and six dated
  limitations stand in order. None of that is this slice's to change.
- The three-spelling sandbox argv assertion, exact-root confirmation,
  current-only usage pins and every independent refusal token, with removal
  controls **M1–M14** recorded in the two 2026-09-16 returns above.
- `--worktree` and `--thread-source` refusal in their **bare** spellings at
  both the blocker seam (`tests.rs:2726-2727`) and the launch seam
  (`tests.rs:2462-2471`), with M13/M14 as their controls. Those controls
  admitted each name to `CODEX_RESUME_BARE_FLAGS` and are valid for exactly
  the dangling shape they exercised. They say nothing about a value-bearing
  spelling: a name in `CODEX_RESUME_VALUE_FLAGS` with nothing after it is
  still refused, so a dangling case passes under that admission and cannot
  detect it. E2, E3 and E4 close that hole; this bullet is not a claim that
  it is closed today.

### Ordered clauses E1–E11

**E1. Record entry state and adopt both dependency records — 11.1;
evidence / LE5; progress / PM1, PM4.** Before any edit, record the actual
HEAD, the checkbox ledger, and the before-state test counts of every suite
this slice touches (`adapters::tests` 154, `driver_conformance` 18,
`witness_digests` 4, `bundle::compose_tests` 17 as last reported) by running
them, not by quoting this line. Confirm a writable **disk-backed TMPDIR
outside the repository** before the first Cargo command; an in-repo TMPDIR
breaks the ledger tests and the box's global default may be read-only or
tmpfs. Verification: an honest before-measurement, never a reused historical
total; a filter that discovers zero tests is not a pass.

**E2. The nine exec-only options and the two new shapes at the blocker seam
— 11.1; safety / AS1, AS3; evidence / LE5.** In
`adapters::tests::only_the_flags_a_resume_can_safely_carry_travel_with_it`
(`crates/brokkr-protocol/src/adapters/tests.rs:2688`) the refused list
exercises `--profile`, `--add-dir`, `--approve-for-me` and `-C`, but never
`--cd`, `--color`, `--local-provider`, `--oss` or `--version`, which
`/obligations/allowed_safe_passthrough/exec_accepts_but_resume_does_not`
measures as refused by the subcommand itself. Add all five, in their separate
and joined spellings, asserting `codex_resume_blocker` returns
`Some(<that exact part>)` — the exact string, never `is_some()` and never
`is_err()`. Removal control, one option at a time: admit that option to the
list **matching the constructed case's arity**, correct the array length so
it compiles, observe the exact `<option> may not travel to a resume`
assertion fail, restore sha256-verified bytes and rerun the named test. The
supplied record enumerates option NAMES, not arities; where it does not
document a value, construct the bare shape and mutate
`CODEX_RESUME_BARE_FLAGS`, and say in the return that no provider arity is
claimed. A bare-list mutation cannot prove value consumption, and a
value-list mutation beside a dangling flag proves nothing either.

Then the two shapes NEW in 0.154.0, in the spellings no recorded control has
reached. The refused list exercises `--worktree` and `--thread-source` only as
dangling flags (`tests.rs:2726-2727`), and a dangling flag is refused by the
value branch (`adapters.rs:2429-2432`) exactly as it is by the fall-through —
so M13/M14 falsify a bare-list admission and nothing else. These cases cannot
ride the existing single-part loop (`tests.rs:2735-2739`), whose body appends
one part to `["--model", "sol"]`; add them beside it, as their own
`assert_eq!` calls or a second loop over `(Vec<&str>, &str)` pairs. Four
cases, each preceded by `--model sol` so the value branch is exercised first:
`["--thread-source", "github"]` and `["--worktree", "/distinctive-worktree"]`,
each asserting `Some(<the flag NAME>)`, which is the part the fall-through
returns; and `"--thread-source=github"` and `"--worktree=/distinctive-worktree"`,
each asserting `Some(<the WHOLE joined part>)`, because `split_once('=')`
(`adapters.rs:2434-2437`) refuses the part entire when the name is not
admitted. Removal control, one name at a time: admit `--thread-source` to
`CODEX_RESUME_VALUE_FLAGS` (arity 7 -> 8, `adapters.rs:2388`), observe BOTH of
its value-bearing cases fail, and record in the same breath that the dangling
case **still passes** — that passing dangling case is the evidence the adopted
control was blind. Repeat for `--worktree`. Restore sha256-verified bytes and
rerun the named test after each. Constructing a value shape asserts what this
adapter composes, not a provider arity: the September 17 record enumerates
names, and the return says so.

**E3. The whole-argv consequence of the same options — 11.1; safety / AS1,
AS2, AS3; evidence / LE3, LE5.** E2's seam observes one thing: the value
`codex_resume_blocker` returns. The launch token, the unchanged cold argv and
the absent root are observable only here, so every option E2 refuses at the raw
seam also earns a case in
`a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled`
(`tests.rs:2397`, eleven cases today), which exercises `--profile` alone of the
nine (`:2439-2443`). Adopted design `7e36fb15` (`design.md:3543-3544`) requires
launch-level cases for all eight non-sandbox exec-only options and for the
value-bearing new shape. Construct these nine, each carrying
`--sandbox read-only` first so the class path is satisfied and only passthrough
admission fails:

| case | parts after `--sandbox read-only` | constructed arity | list its control mutates |
|---|---|---|---|
| `workdir` | `--cd`, `/distinctive-workdir` | value | `CODEX_RESUME_VALUE_FLAGS` |
| `added-dir` | `--add-dir`, `/distinctive-added-dir` | value | `CODEX_RESUME_VALUE_FLAGS` |
| `colored` | `--color`, `never` | value | `CODEX_RESUME_VALUE_FLAGS` |
| `local-provider` | `--local-provider`, `distinctive-provider` | value | `CODEX_RESUME_VALUE_FLAGS` |
| `oss` | `--oss` | bare | `CODEX_RESUME_BARE_FLAGS` |
| `approving` | `--approve-for-me` | bare | `CODEX_RESUME_BARE_FLAGS` |
| `versioned` | `--version` | bare | `CODEX_RESUME_BARE_FLAGS` |
| `thread-source-value` | `--thread-source`, `github` | value | `CODEX_RESUME_VALUE_FLAGS` |
| `thread-source-joined` | `--thread-source=github` | joined value | `CODEX_RESUME_VALUE_FLAGS` |

`--profile` keeps its existing `profiled` case and is not duplicated;
`--sandbox`, the ninth exec-only name, is deliberately absent — see below. The
loop body already observes everything each case needs: the whole cold argv by
equality (`exec --json -C <workdir>` then the seat's own parts in order,
`:2501-2508`), exactly one launch row (`:2510`), `launch: cold` (`:2511`), the
exact `resume_refusal` string (`:2512-2516`) and absent `root_session`
(`:2517-2521`). So the work here is the cases and the arity, not new
assertions: every new row declares `incompatible-argv` and inherits those five
observations. Widen the `[(&str, Vec<&str>, &str); 11]` arity to **20** — or to
whatever the executor actually constructs, counted off the array rather than
copied from this line. Do **not** add `--sandbox` itself here: at the launch
seam a declared class is translated, not refused, and its absence from the
resume surface is already proved by the argv test's no-sandbox-flag assertion.
Removal control per case, one at a time: admit that case's option to the list
its row names, correct the array length so it compiles, observe THAT case's
assertion fail — for an admitted value flag the passthrough becomes admissible,
the launch comes back `resumed` and both the `incompatible-argv` and the
cold-argv assertions go, which is the intended falsification — then restore
sha256-verified bytes and rerun. A bare-list admission cannot falsify a
value-shaped case: the flag continues, its value lands as a bare word and the
case is refused for a second reason, so the control proves nothing. Arity is
constructed, not claimed: the September 17 record enumerates NAMES, and the
return says no provider arity is asserted.

**E4. The allow-list measured against the surface — 11.1; safety / AS1, AS3;
evidence / LE5.** No test today compares `CODEX_RESUME_VALUE_FLAGS`
(`crates/brokkr-protocol/src/adapters.rs:2388`) and `CODEX_RESUME_BARE_FLAGS`
(`:2397`) with the measured enumeration. Inside the existing blocker test —
no new file, module, helper or production data file — put a **test-local
literal** of the September 17 long-option surface, with
`.forge/tasks/controller-codex-interface-2026-09-17.json` and its
`/obligations/allowed_safe_passthrough` pointers cited in the doc comment;
tests never read `.forge/` at run time. Assert **subset and disjointness
only**: every admitted long option occurs in `resume_accepts`; no member of
`exec_accepts_but_resume_does_not` is admitted; `--worktree` and
`--thread-source` occur in `resume_accepts` and are still refused by the
blocker — the sentence that says parsing is not qualification. Pin the
existing short aliases (`-m`, `-i`, `-o`, `-C`) as this repository's
spellings and invent no measured alias the record does not contain. Assert no
equality with the surface: equality would authorize `--config`, `--enable`,
`--disable`, `--last`, `--all`, `--ignore-rules`, `--ignore-user-config` and
`--dangerously-bypass-approvals-and-sandbox`. Removal controls: for the
subset direction admit one option absent from `resume_accepts`; for
disjointness admit `--sandbox`; each alone, each failing its own assertion,
each restored. Membership is a surface check and qualifies nothing — see
E11. In one direction it is worse than inert, and the test must say which:
`--worktree` and `--thread-source` ARE members of `resume_accepts`, so the
subset direction admits them and the disjointness direction — which
quantifies over `exec_accepts_but_resume_does_not` — never sees them.
Admitting either to `CODEX_RESUME_VALUE_FLAGS` leaves both directions
passing. Add therefore one assertion that is not blind: neither name occurs
in `CODEX_RESUME_VALUE_FLAGS` nor in `CODEX_RESUME_BARE_FLAGS`, asserted
directly against the two constants. Its own control is the same mutation E2
runs — admit `--thread-source` to the value list — and both this
non-membership assertion and E2's two value-bearing cases must fail under
it; record both failures, restore, rerun. State in the return that the
refusal of these two rests on E2's value-bearing blocker cases and E3's
launch cases, and that a passing membership assertion establishes nothing
whatever about them.

**E5. The effort field's exact spelling — 11.1; safety / AS1; evidence /
LE5.** `a_codex_resume_re_expresses_the_effort_pin_as_a_config_override`
(`tests.rs:2340`) today proves the pair exists by `.any()` and slices the
first five parts. `split_effort` (`adapters.rs:2301`) accepts both
`--effort <level>` and `--effort=<level>`, so drive both inputs and assert,
for each: the whole expected resume argv as a literal vector; exactly one
`["-c", "model_reasoning_effort=\"high\""]` pair by count; the literal key
text; and that no `--effort` or `--effort=` part survives. Build the expected
literal by hand — never by calling `codex_effort_config` (`adapters.rs:2285`),
which would assert the formatter against itself. Removal controls, each
alone: misspell the key inside `codex_effort_config` (`model_reasoning_effrot`,
the spelling the controller's `--strict-config` control proves the provider
refuses); omit the emitted pair; and let the author's `--effort` flag survive
into passthrough. If the whole-argv literal catches a mutation before the
count or absence assertion runs, record **that actual failure** and do not
claim the later assertion was reached.

**E6. The stdin positional and the working directory — 11.1; safety / AS1;
site / SR3; evidence / LE1, LE5.**
`a_codex_resume_carries_the_thread_the_class_and_the_prompt`
(`tests.rs:2245`) already asserts the whole argv ending `[THREAD, "-"]` and
the fed prompt. The measured `[SESSION_ID] [PROMPT]` grammar wants that
stated directly: assert the final two parts are exactly the offered thread
then `-`, in that order, and that exactly one `-` part and no other
positional occur. Replace the `"the prompt"` fixture with a **distinctive
multi-word prompt** and compare stdin with `assert_eq!` byte-for-byte; a one-
or two-character fixture under `contains` matches a temporary path on some
platform and passes vacuously everywhere else. Add the cwd observation
(design FM11): the resume argv emits no `-C`, so the working directory rests
entirely on `Command::current_dir(workdir)` in `invoke_codex`
(`adapters.rs:2866`). Strengthen `codex_shim` (`tests.rs:2138`) to record its
own `pwd` beside the argv it already logs one part per line, and assert it
equals the fixture workdir, which is a tempdir and therefore differs from the
runner's cwd. Removal controls, each alone: drop the trailing `-` push
(`adapters.rs:2591`); substitute the appended positional with another legal
id; omit the stdin write; remove the `current_dir` call.

**E7. Both shipping coordinates, each bound to the class IT declares — 11.1;
safety / AS1, AS2; site / SR2, SR3, SR4; evidence / LE1, LE5.** The shipped
harness and inline tests (`crates/brokkr-cli/tests/driver_conformance.rs:1451`,
`:1623`, `:2160` and `proof_shim_body` at `:2055`, with runtime
`engine::boundary_tests::the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`
at `boundary_tests.rs:1695`) log their argv as `"$*"`, space-joined, which
cannot reconstruct element boundaries — a `contains` over that text cannot
exclude a competing flag. Add a boundary-preserving companion log
(`for a in "$@"; do printf '%s\n' "$a" >> <log>.parts; done`) **beside** the
existing `$*` line rather than replacing it, so no existing assertion changes
meaning, and assert over the parts:

- exactly one `sandbox_mode=` part in the whole resume argv, immediately
  preceded by its own `-c`, and its text equal to the class **that coordinate
  declares**, not one literal imposed on both. The harness coordinate composes
  `--sandbox workspace-write` — the shipped `adapters/codex.json`
  `hands.harness.work` fragment, already asserted as the last two parts of the
  composed spawn argv at `driver_conformance.rs:1500-1505` and
  `boundary_tests.rs:1758-1763` — so its expected pair is
  `["-c", "sandbox_mode=\"workspace-write\""]`. The inline coordinate declares
  `--sandbox danger-full-access` in the argv `recipes/standby` and
  `recipes/wager-harness` ship (`driver_conformance.rs:1647-1648`), and the
  compiled live proof already asserts `sandbox_mode="danger-full-access"` at
  `:2227`, so its expected pair is
  `["-c", "sandbox_mode=\"danger-full-access\""]` at `:1623` and at all four
  shapes of `:2160`. AS1 and AS2 preserve the class the seat declared;
  prescribing `workspace-write` for both coordinates would assert a class the
  inline coordinate never declares, and the only way to make that assertion
  pass would be to edit the shipped inline argv — a silent change to what
  ships, dressed as a test repair;
- the `-c` / `model_reasoning_effort="xhigh"` pair, adjacent and exactly once,
  at both coordinates. Here the literal genuinely is shared, and for two
  separate reasons that must each be cited: the harness lane's effort comes
  from the shipped `agents/reviewer.json` `efforts.astra`, the inline lane's
  from the literal `--effort xhigh` at `driver_conformance.rs:1645-1646`;
- no `-s`, `--sandbox` or `--sandbox=` part anywhere in the resume argv;
- the final two parts equal to the confirmed root and `-`.

Read each expected value off the coordinate's own declared parts and pin the
literal beside it, so that a shipped change fails the test rather than passing
under a silently re-derived expectation. Keep the shipped assessment, the
production composition bridge and the captured engine exchange as they are;
fabricate no offer and repair no marker. Removal controls: break each
strengthened emission at its production seam, one at a time; and additionally,
to prove the class binding is load-bearing rather than a presence check, mutate
the declared class at each coordinate separately — inline, change
`danger-full-access` to `workspace-write` in the driver fixture at `:1647-1648`
and observe the exact-pair assertion fail; harness, change
`hands.harness.work` in `adapters/codex.json` to `read-only` and observe both
the composed-argv assertion and the resume-pair assertion fail. The harness control touches shipped data: restore it by sha256 and confirm
no other byte of the declaration moved, and expect it to be loud — the
composed class participates in bundle identity, so a witness or compose
digest assertion may fail ahead of the resume-pair assertion. If it does,
record THAT actual failure, then run the narrower substitute control in its
place: put the other coordinate's class in this test's expected literal and
observe the exact-pair assertion fail. Say which of the two controls
actually ran, and never claim the one that did not. Shims are staged beside their target, closed, then
renamed into place before execution (#255); Git-invoking children inherit the
#282 cleanup.

**E8. Keep adopted controls separate from fresh ones — evidence / LE5.**
M1–M14 stand as dated evidence with their recorded failures and line numbers.
The return presents this visit's experiments in their own table and does not
restate an adopted control as new. Where E2–E7 strengthen a test an adopted
control ran against, re-run that control if the strengthening could mask it,
and say which ones were re-run and which were adopted unchanged. M13 and M14
carry a further scope limit that the return must state where it repeats them:
each admitted a new-in-0.154.0 name to `CODEX_RESUME_BARE_FLAGS` against a
dangling case, so each is evidence for the bare shape alone. The value-bearing
shapes of `--worktree` and `--thread-source` have no adopted control at all;
theirs are fresh in E2, E3 and E4 and belong in this visit's own table.

**E9. Declaration, prose and measured digests — 11.1; safety / AS1;
evidence / LE5; progress / PM4.** Only after E2–E7 pass. `adapters/codex.json`
`evidence.interface` (342 characters today) still ends "does not qualify
effort configuration, safe passthrough or the stdin '-' positional", and
`reason` (391) still ends "11.1's interface debt remains"; both are now false.
Correct them to name
`.forge/tasks/controller-codex-interface-2026-09-17.json` as the record that
supplies those three observations and to state the remaining qualification
accurately (E11). Append exactly **one** dated `2026-09-17` limitation naming
that record and that parser acceptance is not confinement. Preserve
`supported`, both `0.154.0` fields, `classes`, `boundaries`, `hands`, the cold
fragments, the four live-axis evidence strings and all six existing limitation
strings byte-for-byte and in order; every bounded value stays within the
loader's 400-character limit — re-measure each after editing rather than
trusting these counts. Reconcile the same distinction in proposed decision
0056 ruling 5 and its consequences
(`docs/decisions/0056-same-instance-session-resumption.md:257`, `:549`) and in
the Codex row of `docs/guides/provider-adapters.md:304`; 0056 stays
`proposed`, accepted 0030 is unchanged, and dated examples keep their dates.
Because the declaration's text participates in bundle identity, **measure**
the pins it moves and copy only the pairs actually reported changed:
`crates/brokkr-runtime/tests/witness_digests.rs` (`recipes/night-shift`,
`recipes/wager-harness`, `recipes/triage`, `recipes/gpt-flash`) and
`crates/brokkr-runtime/src/bundle/compose_tests.rs` (`recipes/panel-review`,
`bundles/self`, `recipes/triage`). A guessed pin is a red branch. Any
throwaway measuring test lives under ignored `.forge/` scratch and is absent
from the patch. This is clause work under 11.1, not group 14 or 15.

**E10. Local gates on restored bytes — evidence / LE5; progress / PM4.**
Prefix every Git command and every Cargo command whose tests spawn Git with
`env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` (#282); use
`CARGO_BUILD_JOBS=2` and a bounded `RUST_TEST_THREADS`. Run each focused suite
with an unambiguous filter and retain its actual test count:

```text
cargo test --locked -p brokkr-protocol --all-features adapters::tests
cargo test --locked -p brokkr-runtime --all-features
cargo test --locked -p brokkr-cli --all-features --test driver_conformance
cargo test --locked -p brokkr-runtime --test witness_digests
cargo test --locked -p brokkr-runtime --lib bundle::compose_tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace
cargo test --workspace --all-features --locked
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
cargo build --release --locked -p brokkr-cli
openspec validate 2026-09-09-226-session-resumption --strict
openspec validate --all --strict --no-interactive
openspec validate --archived --strict --no-interactive
bash scripts/coverage-exact.sh
```

The exact gate demands literal equality on lines, branches and functions, so
every production line added is a line it will demand covered; this slice adds
test lines and prose, and the gate excludes test source. Report the actual
in-box covered/total integers for lines, branches and functions with their
revision and environment. In-box literal equality is unattainable because the
boundary proofs skip where bwrap cannot nest (#286); that shortfall is the
box's, not the edit's, and host equality is the controller's handoff. If a
toolchain or valid scratch is unavailable, report the counts as **unmeasured**,
name that exact limitation, and substitute neither an old report nor a
presumed delta.

**E11. The honest acceptance account, and the commit — 11.1; evidence / LE5;
progress / PM1, PM4.** **11.1 stays unchecked.** Record the delivered subset
and name both missing qualifications exactly:

| Missing qualification | Why nothing supplied closes it |
|---|---|
| `--ephemeral`, admitted by `CODEX_RESUME_BARE_FLAGS` | The September 17 record lists it under `not_previously_supported`, and `codex_launch` sets `persistent: true` on both arms. No supplied observation establishes the ephemeral shape's persistence, restriction, root or current-accounting behaviour. |
| `--output-schema`, admitted by `CODEX_RESUME_VALUE_FLAGS` | Listed the same way; no supplied observation establishes that a schema argument is safe on a resume. |

Change the allow-list in neither direction: removing either entry or
inferring `persistent: false` decides unresolved semantics, and widening it
decides them the other way. Both are an operator's word, and this visit says
neither. A passing E4 membership assertion does not resolve it, and
preserving the shipping disposition does not complete an enablement task.
Do not tick, start or plan 10.6, 10.7, 10.8, 11.2, 11.3, 11.4, 8.8, 8.10,
9.6, passes C and D, or groups 14 and 15. Record each new assertion's test
name, exact invocation, responsible mutation, intended failure output,
restoration and passing rerun; keep logs under task-owned `.forge/` storage.
Commit only completed, verified work **unsigned** in the repository's message
style — the linked worktree forces `commit.gpgsign=true`, so `--no-gpg-sign`
is required (decision 0054 ruling 8). Never push, merge, archive, fold living
specs, publish or start a Brokkr run; the controller owns remote CI, host
coverage, integration and closure.

### Decisions

- **The `$*` shim logs gain a companion, not a replacement.** Rewriting the
  existing space-joined log would silently change what every current
  assertion over it means. A `<log>.parts` file written one argv element per
  line adds the boundary evidence E7 needs while leaving the old assertions
  measuring exactly what they measured before.
- **The surface literal lives in the test, not in a data file.** Design D10
  admits no new production data file, and a test that reads `.forge/` at run
  time would make a hermetic suite depend on run-local storage. The pointer
  into the controller record is cited in the doc comment, where provenance
  belongs.
- **Subset, never equality, against the measured surface.** The measured
  surface contains options the adapter refuses on purpose. Asserting equality
  would turn a safety allow-list into a mirror of whatever the provider
  parses — the opposite of failing closed.
- **`--sandbox` is an E2/E4 case, not an E3 case.** At the blocker seam it is
  a refused name; at the launch seam a declared class is translated into
  `-c sandbox_mode=<class>`, so adding it to E3's refusal cases would assert
  the wrong outcome.
- **Arity is constructed, not claimed.** The record enumerates names. Each E2
  case declares the shape it constructs and mutates the list of that arity, so
  no unmeasured provider arity is asserted anywhere.
- **Each shipping coordinate is bound to the class it declares** (answers the
  2026-09-18 analyze finding F1). An earlier draft of E7 prescribed
  `sandbox_mode="workspace-write"` for the harness and the inline coordinates
  alike. The inline coordinate declares `danger-full-access`
  (`driver_conformance.rs:1647-1648`) and its compiled proof asserts exactly
  that (`:2227`); AS1/AS2 preserve the seat's own class rather than normalising
  it. One literal across both coordinates asserts the wrong thing at one of
  them, and the cheapest way to make it green is to edit what ships.
- **A dangling flag cannot falsify a value-list admission** (answers F3). A
  name admitted to `CODEX_RESUME_VALUE_FLAGS` with nothing after it is still
  refused (`adapters.rs:2429-2432`), so the recorded `--worktree` and
  `--thread-source` cases pass under the very mutation they are supposed to
  catch. Membership against the measured surface is blind in the same place,
  because both names occur in `resume_accepts`. Hence the value-bearing
  separate and joined cases in E2, the launch cases in E3, and the direct
  non-membership assertion in E4 — three seams, because no one of them covers
  the shape alone.
- **Both seams, for every exec-only option** (answers F2). The blocker seam
  observes a returned string; only the launch seam observes `incompatible-argv`,
  the unchanged cold argv and the absent root. Design `7e36fb15` names all
  eight non-sandbox exec-only options at the launch seam, and E3 now carries
  them; refusing an option at the raw seam alone leaves the consequence the
  engine actually journals unobserved.

### Tasks-phase validation and handoff — adopted design `7e36fb15`

This phase repairs only `tasks.md`: the stale sentence under 11.1 that read as
though passing assertions would earn the tick, and this breakdown. The
controller records, the instrument, every declaration, production file, test,
decision, frozen surface and the other four artifacts of this change keep
their entry bytes. No checkbox moved: **83 complete / 18 pending** across the
same 101 identifiers, 10.1 and 10.5 checked and 11.1 unchecked.

Fresh Cargo gates **could not be attempted**: `cargo` is absent from this box
(`cargo --version` exits 127, `command not found`; `PATH` names
`/home/vyanakiev/.cargo/bin`, which the box does not mount, and neither
`rustc` nor `rustup` is present). That is ENOENT, not the permission denial
the 2026-09-18 implement visit met, and it was reported immediately. In
consequence fmt, clippy, the crate suites, both bundle compiles, the release
build and `scripts/coverage-exact.sh` are **UNRUN** on this head, and fresh
in-box covered/total lines, branches and functions are **unmeasured** — not
zero, and not an inherited report. Nothing about this head's gate standing is
claimed; no production or test byte moved in this phase to disturb it.
`/var/tmp` does not exist in this box and `TMPDIR` is `/tmp` on tmpfs, so
E1's disk-backed scratch premise must be satisfied by the seat that compiles.

`openspec` 1.12.0 is present and ran on the edited bytes: strict validation of
`2026-09-09-226-session-resumption` reports `Change … is valid`;
repository-wide strict validation passes **14 items, zero failures**; archived
strict validation passes **six changes, zero failures**. The existing
informational archive-target notices concern the withdrawn living specs and
authorize no fold. The five deltas remain **20 requirements / 165 scenarios**
(`adapter-launch-evidence` 32 + `adapter-resume-safety` 69 +
`boundary-record` 11 + `sdd-progress-markers` 20 + `site-session-resumption`
33), counted on the current bytes; this visit adds none.

This is a **drafted tasks artifact**, not an implementation. 11.1's assertion
work is specified, ordered and bound to named seams, existing tests and
falsifying controls; it is not executed here. 11.1 remains unchecked and, per
E11, is expected to remain unchecked after the clauses pass, for the two
qualifications the supplied evidence does not establish. No provider
experiment, declaration edit, production change, archive, spec fold, push,
merge or new Brokkr run occurred.

### Tasks-phase second sitting — the three analyze findings answered

Returned from analyze at `c54c8601` with `result: drift`, `drift_in: tasks`,
three findings all owned by this artifact and no earlier artifact fault
established. Each is answered in the clause that carried it, in dependency
order, and recorded under `### Decisions` above; no requirement, scenario,
decision, task identifier or checkbox moved, and the ledger stays **83
complete / 18 pending** across the same 101 identifiers with 10.1 and 10.5
checked and 11.1 unchecked.

| Finding | Where it was wrong | What the artifact now orders |
|---|---|---|
| F1, inconsistency, E7 | It prescribed `sandbox_mode="workspace-write"` for the harness AND the inline shipping coordinates. The inline coordinate declares `danger-full-access` (`driver_conformance.rs:1647-1648`) and its compiled proof asserts exactly that (`:2227`); AS1/AS2 preserve the declared class. | E7 binds the expected `-c` pair to each coordinate's own declared class — `workspace-write` at the harness seam (`:1500-1505`, `boundary_tests.rs:1758-1763`), `danger-full-access` at `:1623` and all four shapes of `:2160` — and adds a per-coordinate declared-class mutation as the control that proves the binding is exact rather than a presence check. The `xhigh` effort literal is shared, and E7 now cites the separate source at each coordinate instead of assuming one. |
| F2, coverage gap, E3 | It added launch cases for `--cd` and `--oss` only, leaving `--add-dir`, `--approve-for-me`, `--color`, `--local-provider` and `--version` with no launch-level case, against `design.md:3543`. E2's seam observes only the blocker's return value. | E3 now tables nine launch cases with their constructed arity and the list each removal control mutates, names the five that were missing, states that the loop body already supplies the five observations each case needs (`tests.rs:2501-2521`), widens the array arity to 20, and keeps `--sandbox` out with its reason. |
| F3, coverage gap, E4/E8 | It adopted M14's bare `--thread-source` proof as if it covered the name. It does not: a value-list admission leaves a dangling flag refused (`adapters.rs:2429-2432`), and surface membership is blind because both new names occur in `resume_accepts`. | E2 adds separate and joined value-bearing cases for `--thread-source` and `--worktree` with their exact expected parts, and a control that must record the dangling case still passing. E3 adds the two `--thread-source` launch cases. E4 adds a direct non-membership assertion against both constants with its own control. E8 and the adopted-evidence bullet now scope M13/M14 to the bare shape they exercised. |

`openspec` 1.12.0 ran again on the edited bytes: strict validation of
`2026-09-09-226-session-resumption` reports `Change … is valid`;
repository-wide strict validation passes **14 items, zero failures**; archived
strict validation passes **six changes, zero failures**. The five deltas remain
**20 requirements / 165 scenarios**; this sitting adds none.

Cargo remains **absent from this box**: `cargo --version` exits 127,
`command not found`, and neither `rustc` nor `rustup` is present. That is
ENOENT, not the permission denial the 2026-09-18 implement visit met, and it is
reported here rather than worked around. fmt, clippy, the crate suites, both
bundle compiles, the release build and `scripts/coverage-exact.sh` are
therefore **UNRUN** on this head and in-box covered/total lines, branches and
functions are **unmeasured** — not zero and not an inherited report. No
production or test byte moved in this sitting to disturb that standing.
`/var/tmp` still does not exist here and `TMPDIR` is `/tmp` on tmpfs, so E1's
disk-backed scratch premise remains the compiling seat's to satisfy.

## Implement visit — E1–E11 delivered on a seat with a toolchain; 11.1 still unchecked, 2026-09-18

Entered at `5732233a` on `slice-codex-enable`. This seat has `cargo` and
`git`, so the 2026-09-18 permission blocker and the toolchain-less tasks
sitting above are both behind us: every claim below rests on a command that
ran. Landed as `8271d5e4` (E2–E6), `2dcd7426` (E7) and `88085c7f` (E9), all
unsigned. The ledger stays **83 complete / 18 pending** across the same 101
identifiers, 10.1 and 10.5 checked and **11.1 unchecked**. No requirement,
scenario, decision or task identifier moved.

### E1 — entry state, measured rather than quoted

`adapters::tests` **154**, `driver_conformance` **18**, `witness_digests`
**4**, `bundle::compose_tests` **17**, each by running it. Those totals are
unchanged at exit: this slice strengthens existing tests rather than adding
new ones, as ordered.

E1's scratch premise could not be satisfied and did not need to be. This
seat's file writes are confined to the worktree, so a disk-backed `TMPDIR`
outside the repository cannot be created; `TMPDIR` is `/tmp`, tmpfs, with
~30G free. The default was writable and every suite that builds tempdirs
passed under it, the ledger tests included. **No in-repo `TMPDIR` was used.**
Separately, `env` and env-assignment prefixes are refused by this seat, so
E10's `env -u GIT_CONFIG_COUNT -u GIT_CONFIG_VALUE_0` prefix could not be
applied — and was not needed: #282 is a defect of the installed DSH core,
and bare `git status`, `git add` and `git commit` all work cleanly here.

### This visit's controls, kept separate from the adopted ones

M1–M14 stand as dated evidence and are **not** restated as new. M13 and M14
are scoped, where they bear on this work, to the **bare** shape each
exercised: each admitted a new-in-0.154.0 name to `CODEX_RESUME_BARE_FLAGS`
against a dangling case, and says nothing about a value-bearing one. The
thirty controls below are this visit's own. Each was run alone; after each
group `adapters.rs` and `adapters/codex.json` were confirmed **byte-identical
to HEAD** with `git diff`, which is a stronger restoration check than a
self-computed digest.

| # | Mutation | Observed failure |
|---|---|---|
| N1 | `--cd` → `CODEX_RESUME_BARE_FLAGS` (4→5) | `--cd may not travel to a resume`, `left: None right: Some("--cd")` (`tests.rs:2933`) |
| N2 | `--color` → bare list | same assertion, `--color` |
| N3 | `--local-provider` → bare list | same assertion, `--local-provider` |
| N4 | `--oss` → bare list | same assertion, `--oss` |
| N5 | `--version` → bare list | same assertion, `--version` |
| N6 | `--thread-source` → `CODEX_RESUME_VALUE_FLAGS` (7→8) | `--thread-source may not travel to a resume in a value-bearing spelling` (`tests.rs:2971`). **The dangling case at `:2933` passed under this same admission** — the evidence that the adopted control was blind. |
| N7 | `--worktree` → value list (7→8) | same assertion, `--worktree`; its dangling case also still passed |
| N8 | `--absent-from-the-measured-surface` → bare list | `… is admitted to a resume but is absent from the measured resume surface` (`tests.rs:3050`) |
| N9 | `--sandbox` → bare list | `--sandbox is refused by \`codex exec resume\` itself and must not be admitted` (`tests.rs:3040`) |
| N10 | `--thread-source` → value list, with the two value-bearing `--thread-source` cases removed | `--thread-source is parsed by the resume subcommand and is still not admitted to one: parsing is not qualification` (`tests.rs:3078`) |
| N11 | `--cd` → value list | `workdir: the cold argv, unchanged` (`tests.rs:2687`); the resume argv carried `--cd /distinctive-workdir` across |
| N12 | `--add-dir` → value list | `added-dir: the cold argv, unchanged` |
| N13 | `--color` → value list | `colored: the cold argv, unchanged` |
| N14 | `--local-provider` → value list | `local-provider: the cold argv, unchanged` |
| N15 | `--thread-source` → value list | `thread-source-value: the cold argv, unchanged`; the dangling `thread-source` launch case still passed |
| N16 | `--oss` → bare list | `oss: the cold argv, unchanged` |
| N17 | `--approve-for-me` → bare list | `approving: the cold argv, unchanged` |
| N18 | `--version` → bare list | `versioned: the cold argv, unchanged` |
| N19 | `codex_effort_config` key → `model_reasoning_effrot` | `separate: the whole resumed argv` (`tests.rs:2461`) |
| N20 | omit the emitted `-c`/effort pair | same assertion |
| N21 | let the author's `--effort` survive into passthrough | same assertion; the whole rejoin collapsed to a **cold spawn**, because the surviving flag reached the blocker |
| N22 | drop the trailing `-` push | `short-separate: the whole resumed argv` (`tests.rs:2303`) |
| N23 | substitute another legal id for the appended positional | same assertion |
| N24 | omit the stdin write in `invoke_codex` | `short-separate: the complete prompt arrives on stdin`, `left: ""` (`tests.rs:2351`) |
| N25 | remove `Command::current_dir` | `short-separate: the rejoined seat runs in the workdir it was given` (`tests.rs:2362`); `left` was the runner's own cwd, which confirms the fixture workdir genuinely differs |
| N26 | inline coordinate's declared class → `workspace-write` in the shipped recipe fixture | reached the **pre-existing** launch-row assertion at `driver_conformance.rs:1743`; E7's new assertion was **not** reached |
| N27 | substitute for N26: the other coordinate's class in the inline expected literal | `shipped inline work seat: sandbox_mode`, `left danger-full-access right workspace-write` |
| N28 | `hands.harness.work` → `read-only` in `adapters/codex.json` | reached the **pre-existing** composed-argv assertion at `driver_conformance.rs:1500`; E7's new assertion was **not** reached. Shipped data restored and confirmed unchanged by `git diff`. |
| N29 | substitute for N28: the other coordinate's class in the harness expected literal | `shipped harness work seat: sandbox_mode`, `left workspace-write right danger-full-access` |
| N30 | production seam: `-c` reordered away from `sandbox_mode=` in `codex_launch` | `shipped harness work seat: sandbox_mode is paired with its own -c` (`driver_conformance.rs:1825`). **The pre-existing `$*` `contains` assertion passed under this mutation** — the concrete demonstration that a space-joined log cannot exclude a competing flag. |

Three honest negatives, recorded rather than papered over:

- **E5's follow-on assertions were never reached.** All three of N19–N21
  failed at the whole-argv literal first. The pair-count, literal-key and
  no-surviving-`--effort` assertions are therefore subsumed by that equality
  for every production mutation available; they stand as clearer diagnostics,
  not as independently controlled claims.
- **E4's disjointness direction had no control until the order changed.**
  The two measured lists are disjoint by construction, so admitting any
  exec-only name falsifies the subset direction too and the subset assertion
  always fired first. Disjointness is now asserted **before** subset, which
  gives each direction its own falsifying mutation (N9 and N8). This is
  recorded as a repair the control found, not as something that was already
  true.
- **E7's two declared-class mutations reached earlier assertions** (N26,
  N28), exactly as E7 warned they might. The narrower substitutes it
  authorises were run in their place (N27, N29) and are what proves the class
  binding is exact rather than a presence check. Both facts are stated; the
  control that did not run is not claimed.

### E10 — gates on restored bytes

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS, no diagnostics |
| `cargo test --workspace --all-features --locked` | PASS, **73 test targets, zero failures** |
| `cargo test --locked -p brokkr-protocol --all-features adapters::tests` | PASS, 154 |
| `cargo test --locked -p brokkr-cli --all-features --test driver_conformance` | PASS, 18 |
| `cargo test --locked -p brokkr-runtime --test witness_digests` | PASS, 4 |
| `cargo test --locked -p brokkr-runtime --lib bundle::compose_tests` | PASS, 17 |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | PASS |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | PASS |
| `cargo build --release --locked -p brokkr-cli` | PASS |
| `openspec validate … --strict` | **UNRUN** |
| `bash scripts/coverage-exact.sh` | **UNRUN** |

The last two are a **grant** limitation of this seat, not #286 and not a
property of the edit: this seat's Bash allow-list is `cargo` and `git`, and
both `openspec` and `bash` come back `This command requires approval`. In-box
covered/total **lines, branches and functions are unmeasured** — not zero,
and not an inherited report. Strict OpenSpec validation of this file's own
edit is likewise owed. Both belong to the controller alongside the host
coverage measurement. Every cargo-shaped gate ran and passed.

### E9 — the pins, measured

Only the pairs actually reported changed were copied: `recipes/panel-review`,
`bundles/self` and `recipes/triage` in `compose_tests.rs`, and
`recipes/night-shift`, `recipes/wager-harness`, `recipes/triage` and
`recipes/gpt-flash` in `witness_digests.rs`. `recipes/triage` measured the
same value in both files, which is the cross-check that these are real
compiles rather than transcriptions. No pin was guessed.

### E11 — what 11.1 still owes, and why this is not a tick

**11.1 stays unchecked.** Its assertion half is delivered and its prose debt
is retired, and neither completes it.

| Missing qualification | Why nothing supplied closes it |
|---|---|
| `--ephemeral`, admitted by `CODEX_RESUME_BARE_FLAGS` | The September 17 record lists it under `not_previously_supported`, and `codex_launch` sets `persistent: true` on both arms. No supplied observation establishes the ephemeral shape's persistence, restriction, root or current-accounting behaviour. |
| `--output-schema`, admitted by `CODEX_RESUME_VALUE_FLAGS` | Listed the same way; no supplied observation establishes that a schema argument is safe on a resume. |

The allow-list was changed in **neither** direction. Removing either entry or
inferring `persistent: false` decides an unresolved semantic; widening it
decides the same semantic the other way. Both are the operator's word, and
this visit says neither. E4's membership assertion does not resolve it —
that was the point of adding the non-membership assertion beside it — and
preserving the shipping disposition does not complete an enablement task.

Nothing outside this commission was ticked, started or planned: 10.6, 10.7,
10.8, 11.2, 11.3, 11.4, 8.8, 8.10, 9.6, passes C and D and groups 14 and 15
are untouched. No provider was re-measured; every Codex fact cited here comes
from the two controller records. No push, merge, archive or Brokkr run.

## Returned implement correction — review R1, invocation ownership, 2026-09-18

The review of `85cf6d55..ecac7a05` returned one medium, R1: the codex shims in
`driver_conformance.rs` appended a marker line and then **each argument
separately** to one shared `argv.parts`. A panel runs its members
concurrently against that same shim — `NoHandsMember` seats `x` beside
`checks:x` — so two live invocations' writes interleave, and a reader that
splits the shared file on the marker hands one invocation's parts to another
or truncates one. The exact argv assertions downstream were therefore
nondeterministic. R2 and R3 are answered below without a code move.

### The defect, reproduced before it was repaired

The mechanism is not theoretical here. The recording preamble was restored to
the reviewed shared-append-with-marker form and the new ownership test run
against it, which failed on the first attempt:

```
assertion `left == right` failed: each invocation owns one whole record of its own argv
  left: [[], ["…invocation-00-part-36", …, "…invocation-00-part-42",
              "…invocation-02-part-00", "…invocation-00-part-43",
              "…invocation-02-part-01", …
```

One marker-delimited record holds invocation 00's parts braided with
invocation 02's, and an **empty** record appears beside it — the mixing and
the truncation R1 named, on Linux, at this workload. The reviewed code was
then replaced by the repair and the same test passed.

### The repair — a record per invocation, not a separator in a shared one

`record_argv_snippet` is now the single preamble all three codex shims carry
(`driver_conformance.rs`), and it writes this invocation's argv into a file of
its own under `argv.parts.d`:

- the file is named for the writing shell's `$$`, which **two live
  invocations cannot share**;
- an `-e` probe walks past a name an already-exited pid used, so ownership
  survives pid reuse as well;
- an empty `: >` claims the name before the first part is written, so an
  invocation with no argv still owns its record instead of yielding the name;
- the `$*` line still goes to `argv.log` beside it, so every assertion already
  made over that line keeps measuring exactly what it measured.

`resume_parts` now reads that directory and returns a **whole** record. It
additionally requires every resume record present to agree, so which one is
read cannot decide what the caller asserts: a run that produced two different
resume argvs fails by name rather than silently asserting whichever the
directory listed first.

Two incidental repairs travelled with the shared preamble, both in the same
family the review's own `shell_quote` note opened: the two shipping-coordinate
shims spliced their log paths **unquoted** (`argv_log.display()`), so a
`TMPDIR` containing a space would have split the redirection and lost the log.
They now use the quoted form the proof shim already used.

The concurrent panel is retained, and so is every exact argv assertion. No
production source was touched by this correction — the diff is confined to
`crates/brokkr-cli/tests/driver_conformance.rs`.

### Controls for this correction

Each mutation was run alone and the bytes restored and rerun afterwards.

| # | Mutation | Observed failure |
|---|---|---|
| P1 | the preamble returned to the reviewed shared file + marker line, reader split on the marker | `each invocation owns one whole record of its own argv`, with invocation 00's and 02's parts interleaved in one record and an empty record beside it. The panic reported `driver_conformance.rs:1897` because the mutation was three lines shorter than the restored bytes; the assertion is the one at `:1901` today |
| P2 | compiled-proof expected class `danger-full-access` → `workspace-write` | `compiled live inline Single wrapped=true: sandbox_mode`, `left "sandbox_mode=\"danger-full-access\"" right "sandbox_mode=\"workspace-write\""`, over the whole 11-part record `["exec","resume","--json","-c","sandbox_mode=\"danger-full-access\"","-c","model_reasoning_effort=\"xhigh\"","--model","gpt-6-astra","0199aaaa-…","-"]` |

P2 is the control that the new reader is **not** vacuous: it shows the record
`resume_parts` hands to `assert_resume_argv` is the real, complete resume argv
and not a fragment that would satisfy a weaker check.

### R2 and R3

**R2 stands, unmoved.** `--output-schema` and `--ephemeral` remain admitted,
and the allow-list is changed in neither direction. The two missing
qualifications are exactly as tabled under E11 above, and **11.1 stays
unchecked** for that reason. This correction does not touch them and does not
claim to discharge them. **R3** asks nothing of the tree.

### Gates on the corrected bytes

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS, no diagnostics |
| `cargo test --workspace --all-features --locked` | PASS, 73 test targets, zero failures, no ETXTBSY on this run |
| `cargo test --locked -p brokkr-cli --all-features --test driver_conformance` | PASS, **19** (the ownership test is the new one) |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | PASS |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | PASS |
| `openspec validate … --strict` | **UNRUN** |
| `bash scripts/coverage-exact.sh` | **UNRUN** |

The last two are the same **grant** limitation the previous visit recorded and
not #286: this seat's Bash allow-list is `cargo` and `git`, and both
`openspec` and `bash` return `This command requires approval`. In-box
covered/total lines, branches and functions are **unmeasured**. Both remain
the controller's, alongside the host coverage measurement. No production
source, no pin and no digest moved, so no re-pin was owed: the edit is
confined to a test harness source, which `scripts/coverage-exact.sh` excludes
from the report by path.

Nothing outside the commission was ticked, started or planned. No provider was
re-measured. No push, merge, archive or Brokkr run.

## Current tasks return — DSH composite digest breakdown, 2026-09-19

This is the executable breakdown for run
`dsh-composite-identity-issue-226-26def5a5`, repaired against AK, AS1 and
current design `b2adb4f5` (D6 and D10's retained-input sitting). It replaces
the parked predecessor's synthetic-fixture narrowing and its incomplete
unreadable disposition; all other adopted obligations remain. Work the groups
in order and tick each local checkbox only after its implementation, focused
test and compiling removal control pass. These 18 clauses of pending 8.8
create no change-wide task identifier and never authorize its checkbox.
Every clause serves **safety / AS1 — Resume support is measured per adapter
and execution shape**, in `specs/adapter-resume-safety/spec.md`.

The implementation may edit only the named Rust/guide surfaces, their directly
affected tests, this progress account and pins actually moved by those edits.
Add no dependency, subcommand, contract or general reader framework. Preserve
`extensions/dsh/`, frozen contracts, `policy/phase-machine.json`,
`policy/schemas/`, `fixtures/`, `reference/`, adapter declarations and provider
evidence. Do not change planner production behavior, execute a live provider
or registry measurement, write a declaration digest, perform 10.7's retained-
home recording, or start 8.8(d), 8.10, 9.6, 10.6–10.8, 11.1–11.4, 14 or 15.
Hermetic DSH/Node version sentinels are diagnostic test probes, not provider
qualification. The unchanged archive operation remains pending with the whole
change; this partial slice cannot honestly fold its unfinished deltas.

### 1. Preserve and prove the closed loader — 8.8(a)

- [x] Preserve the existing `ResumeIdentity::Measured.wrapper_digest:
  Option<String>` and measured closed key set in
  `crates/brokkr-runtime/src/agents/load.rs`: absent remains valid; present
  is exactly 64 lowercase ASCII hexadecimal characters, without trimming or
  normalization; `Unknown` admits `unknown` alone. Preserve the exact member
  through selection into the private assessment, independently of
  `originating_wrapper_digest`; changing the declaration member moves the
  adapter content digest. Add no store, wire, contract or planner field —
  safety / AS1.

- [x] Strengthen
  `agents::tests::the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name`
  and `an_edited_resume_assessment_moves_the_adapter_digest` with exact
  presence/omission, changed-digest and field-plus-grammar/type reasons for
  uppercase, short, long, non-hex, mistyped and unknown-beside-digest cases.
  Extend `engine::resume_tests::an_offered_dsh_start_carries_the_recorded_home_at_the_single_site`
  and `an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member`
  to assert the selected declaration's exact member and omission in the
  actual `Start.input.resume_context.assessment`, on their real start paths.
  Serialization alone or the originating checkpoint digest is insufficient.
  Mutate admission, grammar, declaration identity and selected-value carriage
  separately; record each exact assertion failure, restore and rerun the
  owning runtime test. A bare `is_err()` or generic failure is no proof —
  safety / AS1.

### 2. Seal the sole producer and its byte readers — 8.8(b)

- [x] Make `dsh_composite` the only production entry that produces either
  digest, retaining `crates/brokkr-protocol/src/adapters/composite.rs` beside
  the DSH planner. Remove public re-exports of `canonical_composite`,
  `plugin_component`, `plugin_file_digests`, `npm_dependencies`,
  `pnpm_dependencies` and the injected producer from `adapters.rs`; keep
  parsing, hashing and injection private. Expose only necessary seams,
  observation read access and error surface. Callers cannot construct an
  observation from precomputed digests; synthetic construction is test-only.
  Remove competing test serializers as well as production alternatives;
  per-input/file SHA-256 checks remain legitimate. No digest newtype or
  public error hierarchy. Review visibility and callers, compile affected
  suites, and use a compiling substitution/removal of the sole observation's
  emitted component in the existing canonical-order test to fail its
  producer-derived pinned-output assertion (group 3 later adds measured pins);
  a visibility/compiler error does not count as behavioral removal evidence —
  safety / AS1.

- [x] Retain every identity-bearing source once per call: the selected core
  manifest from discovery, the hidden npm lock for both core and dependencies,
  and each plugin file, reusing its retained `cordis.patch.yml` digest for
  `plugin-patch`. Do not reopen profile, lock or patch sources for another use.
  Extend private injected-reader count/changed-second-read tests within the
  existing composite suite. Reintroduce each concrete manifest, hidden-lock
  and plugin-patch reopening separately; assert the exact read count and
  consistent retained component, observe failure, restore and rerun. This
  proves one-pass consistency, not an atomic filesystem snapshot — safety / AS1.

- [x] Replace lossy walking with exact path membership: the six regular,
  non-symlink files, their necessary `lib/` ancestor and, as the sole extra,
  a direct real non-symlink `<plugin>/node_modules/` directory. Check metadata
  before skipping that directory; never traverse it. Reject other directories
  even if empty, deeper `node_modules/`, symlinks, special entries,
  non-UTF-8 names and unreadable/missing/extra files with a safely rendered
  responsible path and reason. Hash six `<relative path>\0<file SHA-256>\n`
  lines in bytewise UTF-8 slash-path order. Extend
  `the_plugin_component_is_bytewise_path_order_and_fails_closed`,
  `the_plugin_component_names_a_missing_expected_file` and walk-error tests;
  clear the extra-file fixture before deleting a required file so missing
  and extra claims fail independently. Remove each membership, metadata,
  exact-path or regular-file guard and each relevant emitted file line in
  compiling controls; assert its own path/reason or pinned digest failure,
  restore and rerun. Never mutate committed extension bytes — safety / AS1.

- [x] Apply one source-scalar rule before concatenation, rejecting empty,
  NUL and every whitespace character, including space, tab, CR and LF, in
  names, versions, integrities, core fields, Node's extracted version and
  profile bundle/reload values. Intentional spaces between triple fields are
  added only afterward. Read `node --version` as exactly one non-empty record
  with at most its one terminator; do not `trim()` banners or remove a
  forbidden CR from the executable shebang. Extend
  `the_component_gate_refuses_each_forbidden_byte`,
  `npm_versions_with_any_whitespace_are_unreadable`,
  `spawn_node_runtime_reads_one_version_line_and_refuses_the_rest` and
  core/shebang cases with exact field/component reasons. Remove each delivered
  rejection separately, with all preceding inputs valid, observe the intended
  assertion fail, restore and rerun. These focused guards do not complete any
  of 8.10's separately owned rejection-vector ledger — safety / AS1.

- [x] Normalize npm from every complete left-to-right
  `node_modules/<package>` group in the retained hidden lock, preserving
  scoped spellings and taking only the terminal name and that entry's verbatim
  string version/integrity; ignore optional `name`. Refuse malformed ancestor
  or terminal groups and invalid/missing/mistyped fields with their reasons.
  Exclude only the exact core key and applicable plugin/conditional-extension
  local-tarball records whose bytes enter components; preserve same-name
  registry entries. Deduplicate only equal complete triples and sort value
  bytes across both locks. Extend existing terminal, three-group, malformed,
  distinct-triple and hidden-lock cases. Remove terminal extraction, optional-
  name independence, exact-record exclusion and complete-triple deduplication
  separately; record each exact triple/refusal failure and restored pass —
  safety / AS1.

- [x] Replace the unbounded pnpm text read with the no-YAML fail-closed raw
  reader: inclusive 8,388,608 bytes, consuming at most 8,388,609 before
  UTF-8 conversion. Trust no metadata size; add no npm, line, line-length or
  entry-count cap. Preserve the closed lockfile-9 state machine over the
  measured header, sections, quoting, keys, child vocabulary and single
  flow-form registry resolution. Refuse tabs, CR, comments, document markers,
  unknown structure, malformed quoting/keys, block/repeated resolution and
  missing integrity by exact reason. Extend
  `pnpm_locks_reject_every_unrecognized_construct` and add accepted-blank-line
  padding to the exact limit, one byte over, and read-count/race controls.
  Assert exact-limit grammar entry, at most limit+1 bytes consumed and
  `pnpm lock exceeds 8388608-byte limit` before normalization. Remove the
  bounded read, overflow guard, inclusive comparison and a closed-grammar arm
  separately; record exact failures, restore and rerun — safety / AS1.

### 3. Compose the fixed identity from fixed locators — 8.8(b)

- [x] Preserve only D6's locator chain: canonical resolved executable, nearest
  `@deepseek-ai/dsh` package, exact `bin.dsh` at
  `node_modules/@deepseek-ai/dsh/lib/bin.js` with exact shebang, hidden
  `<core root>/node_modules/.package-lock.json`, first executable `node` on
  the child PATH, and `<home>/profiles/headless/package.json` plus its
  bounded pnpm lock. Require the core's own hidden-lock version to match its
  package. Node PATH selection must skip an ordinary non-executable file,
  without another runtime guess or resolver process. Distinguish absent
  lookup candidates from failed metadata; a known candidate's failure cannot
  authorize another hit. Extend executable/core/profile/layout and seam tests
  with exact locator/component/reason assertions, including missing hidden
  lock beside a plausible root lock and missing headless beside another
  profile. Remove each delivered locator/executable/failure guard in isolation,
  record its exact failed assertion, restore and rerun — safety / AS1.

- [x] Preserve the existing `Profile { dir, canonical }` correction: original
  profile directory as lookup anchor, full directory canonicalized once as
  containment boundary. Keep core lookup paths before profile paths and
  judge each first-hit canonical candidate by components against canonical
  core or profile, confining plugin/conditional extension to profile. Refuse
  uncanonicalizable boundary/candidate, near-prefix siblings and outside
  first hits, without raw fallback or continued search. Strengthen
  `the_dsh_composite_accepts_a_symlinked_home_ancestor`,
  `containment_compares_canonical_components_not_string_prefixes` and
  `an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one`.
  Keep earlier bundles valid to reach the intended plugin refusal. Assert
  alias component/canonical equality and exact offending path/reason;
  substitute raw boundary, string containment, altered anchor or later-hit
  search separately, observe the owning assertion fail, restore and rerun.
  Do not claim a new implementation of the inherited symlinked-home fix or
  completion of 8.10 — safety / AS1.

- [x] Serialize D6's exact `<component>\0<value>\n` stream: `core`, `node`,
  deduplicated bytewise-sorted `dependency` values, `plugin`, `plugin-patch`,
  `profile-patch`, declared-order `profile-bundle` values,
  `profile-patch-reload`, `home-patch`, then `extension` only when
  `brokkr-dsh-resume-policy` is named. Use the same private file computation
  for its existing four-file set; author no extension. Home patch is its byte
  digest or literal `absent` only for true absence: a dangling symlink,
  known-present unreadable entry or read/metadata failure is `home-patch`
  unreadable with safe locator/reason. Attribute hidden-lock/profile/patch
  failures to their own components. Exclude absolute locations, `cordis.yml`,
  raw profile manifest bytes, `pnpm-workspace.yaml`, env layers, credentials,
  settings, state and per-seat overlays. Extend the canonical-order, locator-
  movement and conditional-extension tests using producer-derived pins.
  Remove each emitted element/order, relevant exclusion and absence-versus-
  failure guard separately; record exact digest/movement/component/reason
  failures, restore and rerun. No parallel test-side composite serializer —
  safety / AS1.

- [x] Replace the synthetic rc.2 fixture clause with complete literal measured
  ground truth in `crates/brokkr-protocol/src/adapters/composite/tests.rs`.
  At authoring time copy the full hidden lock from
  `.forge/tasks/dsh-core-package-lock-015rc2.json`, full pnpm/profile-patch
  strings and layout from the qualification addenda, and measured expected
  triples (the derived-list record is an expectation only). Preserve all bytes,
  line endings and final newlines; never reserialize, truncate, compress or
  regenerate them. Build/test code never reads or includes `.forge/`.
  Assert raw npm 311,184 bytes / SHA-256
  `b84bac2d866224a997be29811dc71bde6013dbc6e2adf8c1e77523e6f05a3847`,
  pnpm 1,982 bytes / 57 lines / longest 186 / SHA-256
  `4708752f0463211bf25d470fc26befa49748707b9c12fae7b4f2544e02b21055`,
  and profile patch 217 bytes / SHA-256
  `ef189a8c27db6d63930aa3046a3040482e952eafcb7487c644d508e8d461f027`.
  Materialize temporary fixed locators: core rc.2 and exact measured integrity,
  measured executable/shebang, Node `v22.23.2`, built-in base and headless
  bundles under CORE at their measured scoped paths/rc.2 versions, and plugin
  under PROFILE at `node_modules/dsh-plugin-cli-session`, version 0.2.0.
  Copy the six unchanged committed plugin files; assert all six measured
  digests, exact set, one adapted expression, upstream substitution and delta
  digest in `the_committed_plugin_set_is_the_six_files_and_the_one_expression_delta`.
  Assert both first-hit anchors, declared bundle order, `startup`, plugin patch
  `84745a1bb00d773acf2e5ab5e32dc42825ffe164100ba469375dcabbbd5f9dab`,
  measured profile patch and absent home patch; canonical plugin has no nested
  `node_modules/`. Keep direct dependency-directory tolerance and synthetic
  rejection/extension cases separate. Assert full ordered values alongside
  522 integrity-bearing npm entries, 521 after exact-core exclusion,
  501 complete triples / 489 names retaining all twelve differing-name sets,
  four exact pnpm triples, three exact overlaps and 502 combined values.
  Bind all three real npm path cases in AS1/D6 to their full terminal names,
  same-entry versions and measured integrities. The producer reads locks,
  never the derived list. Invoke only the sole producer to obtain expected
  plugin/composite digests, pin those outputs, then prove them against the
  independent inputs, component/value assertions and compiling removals;
  self-equality and counts alone are insufficient. Collapse by name, omit a
  differing triple, remove exclusions, take an ancestor name, move a built-in
  to the wrong anchor and remove a serialized component/order in separate
  controls, each failing its exact owning assertion before restoration.
  Corrupt only copied fixture bytes as additional negative controls, never
  `extensions/dsh/`; no fixture result claims 10.7 recording — safety / AS1.

### 4. Make doctor observe the same resolved installation — 8.8(c)

- [x] Refactor only the DSH branch in `crates/brokkr-cli/src/doctor.rs` to
  resolve adapter seams once, version-probe the selected executable and pass
  the same seams to `dsh_composite`. Preserve non-DSH probing. In hermetic
  subprocess provider-line tests give primary override `BROKKR_DSH_BIN`,
  legacy `FORGE_DSH_BIN` and PATH installations distinguishable versions and
  readable component inputs; assert BOTH version and corresponding producer-
  derived composite when both overrides, only legacy, and neither are set.
  Prove a failed selected version probe does not retry a usable PATH decoy.
  Preserve the selected binary and successful version if the home seam fails;
  report the named seam/component failure without a second resolution or a
  fabricated missing binary. Assert one seam selection and exactly the
  selected `dsh --version` and first child-PATH `node --version` probes when
  reached, with no credential/settings/env-layer/session/state/overlay read
  or resolver/model process. Revert each observation separately to bare-name
  probing, weaken precedence/failure handling and process/read limits in
  isolated compiling controls; record which paired assertion fails, restore
  and rerun — safety / AS1.

- [x] Implement D10's full truth table in the composite suffix. Readable
  output includes canonical and plugin digests and exactly one of
  `matches the declared wrapper_digest`,
  `differs from the declared wrapper_digest <digest>`, or
  `no declared wrapper_digest`. Unreadable output retains the safe responsible
  component/reason AND either `no declared wrapper_digest` or
  `declared wrapper_digest <digest>; comparison unavailable`; it never
  invents a match or difference. Composite warning requires a supported shape,
  a declared digest, and difference or unreadability; all other composite
  states are informational. Version-availability warnings remain independent,
  and an observed version survives a composite error. Extend
  `the_dsh_composite_detail_reports_each_disposition`,
  `doctor_appends_the_dsh_composite_detail_to_the_provider_line`,
  `doctor_warns_when_the_dsh_composite_differs_from_a_declared_digest` and
  real-seam tests with exact strings, reasons and terminal sanitization for
  every table row. Remove each classification, warning, declaration-context,
  version-retention or sanitization arm separately; record its exact failed
  assertion, restore and rerun — safety / AS1.

- [x] Update `docs/guides/provider-adapters.md` with the emitted rc.2 doctor
  vocabulary and producer-derived measured-fixture output, explicitly
  attributed to that fixture. Cover readable match/difference/undeclared and
  unreadable declaration-presence/comparison-unavailable wording. Never
  relabel the inherited synthetic rc.1 sample as measured, add a declaration
  digest, alter qualification evidence or imply lifetime integrity/10.7
  completion. Retain a guide/content assertion against actual rendered
  output; remove or change the responsible production wording, record its
  exact failed sample assertion, restore and rerun — safety / AS1.

### 5. Verify every delivered claim and preserve the slice boundary

- [x] For every delivered substantive claim above, append a removal-proof row
  here naming the production mutation, exact test, exact assertion/reason
  that failed, exact restoration and passing focused rerun. Perform compiling
  mutations one at a time and undo them before the next. Replace touched bare
  `is_err()` assertions with the responsible variant/component, reason and
  drifted path. Never credit a compile/setup failure, earlier unrelated
  assertion, self-equality or coverage percentage as removal evidence —
  safety / AS1.

  *Untied 2026-09-19, council return F8.* It was ticked while the account
  below it named four controls that did not exist. Fifteen rows (R1–R15)
  are now recorded, the closed-grammar arms are removed one at a time and
  every touched bare `is_err()` carries its reason — but two controls the
  clause asks for still do not exist, and are named in the account rather
  than counted as delivered: the plugin `cordis.patch.yml` reread, which
  no assertion can distinguish because reopening an unchanged file yields
  the same digest; and removing the bounded reader's `take`, whose harm is
  an unterminated read rather than a failing assertion.

  *Re-tied 2026-09-19, second council return.* Both missing controls now
  exist and fail by assertion (R24 and R25 in the second return's table
  below): a listing that rewrites the patch the instant the walk has
  hashed it separates one read from two, and a counting reader over a
  FINITE source makes a lost `take` a number rather than an exhaustion.
  The remaining checked clauses' named controls are rows R16–R49 and the
  five-test B19 re-run; the last bare `is_err()` in the composite suite
  carries its reason.

- [ ] On final restored bytes run
  `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
  `cargo test -p brokkr-runtime --all-features --locked`, then equivalent
  separate `-p brokkr-protocol` and `-p brokkr-cli` runs;
  `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`;
  `bash scripts/coverage-exact.sh` on a capable host/CI with its pinned
  compiler and namespace prerequisite; and `openspec validate --all --strict`.
  Do not substitute the potentially deadlocking workspace-wide CLI test run
  for crate-scoped tests. Keep the exact gate's literal 100% line/branch/
  function equality and all exclusions unchanged. Record actual commands and
  results; a missing tool or boxed namespace refusal leaves its gate pending,
  not green. Keep this checkbox open until the required fresh external result
  exists; older counts never supply it — safety / AS1.

- [ ] After every local implementation/proof/gate clause passes, tick its
  checkbox and record exactly what (a)–(c) delivered. Keep 8.8(d), all 8.10
  cases, 10.7's retained-home doctor recording, declaration pin, enablement,
  archive and remaining whole-change acceptance pending. Leave global 8.8
  unchecked and the 84-complete/17-pending ledger unchanged. Commit verified
  work unsigned in repository style and never push; record any unrun gate
  without an unconditional delivery claim. This delivery-account clause is
  finished only with the verified work committed — safety / AS1.

  *Untied 2026-09-19, council return F8.* Its own precondition is "after
  every local implementation/proof/gate clause passes", and the gate
  clause above it is open: the exact-coverage gate has produced no report
  on any bytes at this HEAD. The account is written and the work is
  committed; the tick waits on the gate, not on the account.

### Tasks return validation — adopted retained-input design, 2026-09-19

This tasks seat read the rendered tasks/return/archive dialect, adopted AK,
AS1 and D6/D10 at `b2adb4f5`, and inspected the loader, runtime single/panel
start tests, composite producer/tests and doctor provider-line/tests. The
returned design supplies no unresolved upstream finding. Its dependency-order
handoff is answered in place: group 1 now proves actual selected private
carriage; groups 2–3 preserve the sole producer and use complete measured
literals and ordered expectations; group 4 retains declaration context on
unreadability and proves both observations through each binary seam. Design
choices and their scenarios remain owned by `design.md` under `## Decisions`;
this breakdown adopts them without a new decision or specification rewrite.

Authoring-only input checks reproduce the full hidden-lock, pnpm-lock and
profile-patch byte lengths and SHA-256 values named above, pnpm's 57 lines and
186-byte longest line, and all six committed plugin hashes. Parsing the full
retained npm lock independently for expectation checking agrees with every
ordered triple in the supplied cross-check: 522 integrity-bearing entries,
521 after the exact core exclusion, 501 complete triples, 489 names and twelve
names with differing triples. The four supplied pnpm triples overlap in three
complete values, yielding 502 combined values. These checks establish that the
inputs needed by the breakdown exist; they neither produce a component or
composite nor execute the future Rust fixture or prove its behavior.

`openspec validate --all --strict` passes **15 items / 0 failures** and
`git diff --check` passes. An artifact audit confirms five numbered groups,
**18 unchecked local clauses**, each naming safety / AS1, and the unchanged
**101 global identifiers / 84 checked / 17 pending**, including 8.8. The five
deltas count **20 requirements / 178 scenarios** (evidence 32, safety 82,
boundary 11, progress 20, site 33). The earlier 174-scenario account is
superseded for this commission. OpenSpec's informational archive refusals for
the withdrawn living safety/progress targets remain whole-change history;
no archive or fold is performed or newly planned by this slice.

The exact format/clippy commands, all three separate locked all-feature crate
test commands and self-bundle compile listed in group 5 were attempted: each
could not launch because **Cargo is absent (ENOENT, exit 127)**. The exact
coverage script also exits 127 at its first Cargo invocation, before any
coverage measurement. Attempt logs are retained in
`.forge/tasks-26def5a5/gate-attempts.json`, never as runtime/build inputs.
No fresh Rust pass, coverage count or removal outcome is claimed. Literal
100% line/branch/function coverage remains pending capable host/CI evidence,
with the pinned compiler and namespace prerequisite unchanged.

Only `tasks.md` changes in this tasks commit. The adopted proposal, deltas,
design and proposed 0056, production/tests, guide, declarations, frozen paths
and extension provenance bytes retain their committed content. This is a
**drafted tasks checkpoint**, not implementation completion. Every local
implementation checkbox and global 8.8 stays unchecked; (d), all 8.10 cases,
10.7 recording, declaration pin, enablement and whole-change acceptance remain
pending. No provider probe, install, planner change, task tick, push, release
or new Brokkr run occurred. No earlier artifact needs repair, so the phase
result is `drafted`, with the adopted change identifier in `inputs.change`.

## Implement visit — 8.8(a), (b) and (c) delivered; (d), 8.10 and the exact gate still pending, 2026-09-19

Run `dsh-composite-identity-issue-226-26def5a5`, on `slice-dsh-composite-b`
above `310c5864`. This visit delivered only the loader, the sole Rust
composite producer and the DSH doctor report, in D10's order. **Task 8.8 stays
unchecked**: its acceptance also spans part (d)'s planner and 8.10's
rejection-vector ledger, neither of which this visit touched. Fourteen of the
breakdown's eighteen local clauses are ticked above; the four that are not are
named below with exactly what they still owe.

### (a) The closed loader, and the carriage nobody had read

`ResumeIdentity::Measured.wrapper_digest` and its 64-lowercase-hex check were
already at this HEAD and are preserved unchanged. What was missing was the
proof that the DECLARED member reaches the driver — the inherited tests read
the originating checkpoint's digest, which is a different fact.

`engine::resume_tests::a_declared_wrapper_digest_reaches_the_private_start_context`
loads the shipped `adapters/dsh.json` through the production loader with the
member added and then removed, runs a real seat, and reads
`Start.input.resume_context.assessment["headless-work"].identity`. The two
named start tests now carry a selected declaration of their own, so BOTH
production `start_context` call sites — `run_driver` and the panel member —
are read: `an_offered_dsh_start_carries_the_recorded_home_at_the_single_site`
asserts `c…`, `…_at_the_panel_member` asserts `d…`, each distinct from the
originating `a…`. Omission stays an omission rather than a null. No store,
wire, contract or v5 member was added.

### (b) One producer, sealed, over fixed locators

`adapters.rs` now re-exports only `dsh_composite`, `DshSeams`, `DshComposite`
and `CompositeError`. `canonical_composite`, `plugin_component`,
`plugin_file_digests`, `npm_dependencies`, `pnpm_dependencies`,
`spawn_node_runtime`, `dsh_composite_with`, `NodeRuntime`, `PLUGIN_FILES` and
`EXTENSION_FILES` are private to the module. The planner's two call sites are
unchanged.

Delivered behaviour, each a change from the inherited bytes:

- **Exact membership walking.** The walker admits the six declared files,
  their `lib/` ancestor and — as the one extra-entry exception — a DIRECT,
  real, non-symlink `node_modules/` directory, granted on metadata already
  read and never traversed. A deeper `node_modules/`, an empty or stray
  directory, a symlink, a special entry, a non-UTF-8 name and an unreadable
  or missing file are each refusals naming their own relative path. The
  inherited `to_string_lossy` merge is gone: a name that is not UTF-8 is
  refused rather than mapped onto a spelling that might collide with a
  declared one.
- **Exact-record exclusions.** The core leaves by its exact hidden-lock KEY
  and the plugin by its local `file:` record, so a same-named registry entry
  and a second version of the same package are both retained. The inherited
  name-wide list would have erased them.
- **One scalar rule.** Empty, NUL and EVERY whitespace character — space,
  tab, CR and LF — are refused at the source, before concatenation, naming
  the responsible component. `node --version` is read as one record with at
  most one terminator; the inherited `trim()` would have repaired a padded
  banner into a plausible version.
- **A bounded pnpm reader.** `read_pnpm` takes at most 8,388,609 bytes before
  any unbounded allocation and trusts no metadata size.
- **Components named for themselves.** An unreadable profile or home patch is
  a `profile-patch`/`home-patch` refusal, not plugin drift. A dangling
  home-patch symlink is unreadable; only a true missing path yields `absent`.
- **One read per identity-bearing source.** `resolve_core` retains the
  manifest from discovery and parses the hidden lock once; `npm_dependencies`
  consumes that retained value and has no path to the filesystem at all. The
  plugin's `cordis.patch.yml` digest is taken from the component walk and
  reused as `plugin-patch`.

**The measured fixture.** `composite/tests.rs` now embeds the complete
measured inputs as literal Rust constants — the 311,184-byte hidden npm lock
(SHA-256 `b84bac2d…`), the 1,982-byte pnpm lock (`4708752f…`, 57 lines,
longest 186) and the 217-byte profile patch (`ef189a8c…`) — materialized at
D6's locators beside the six unchanged committed plugin files. Nothing reads
`.forge/` at build or run time. The fixture asserts the rc.2 core name,
version and measured integrity, `node_modules/@deepseek-ai/dsh/lib/bin.js`
with its exact shebang, Node `v22.23.2`, `startup`, the measured profile and
plugin patches, absent home patch, the declared bundle order, and both real
anchors: `@deepseek-ai/dsh-base` and `@deepseek-ai/dsh-headless` under the
CORE root, `dsh-plugin-cli-session` under the PROFILE.

The dependency set is asserted as complete ORDERED VALUES, not counts: 501
distinct complete npm triples over 489 names, four pnpm triples with their
three exact overlaps, and the 502 combined values. All three supplied
path-grammar cases bind to their terminal spellings with that entry's own
version and integrity. The producer's own output is then pinned:

| Value | Producer output over the measured inputs |
|---|---|
| plugin component | `074d1b111148cd3f1770a5afc23e1589fbef61cc940c49385e97da8117e2eda5` |
| canonical composite | `a64fcd6d048603ecb1767b229fa0fb6a30d9ae7cda92a47cdc82360d9ee3ddd1` |

These are FIXTURE results. They are not 10.7's retained-home recording and
they authorize no declaration.

### (c) One resolved installation, in doctor

The commissioned defect is closed. `probe_providers` no longer probes the
bare `adapter.binary` for DSH: `dsh_provider_line` resolves the seam once
through `DshSeams::selected`, probes `--version` on THAT executable and hands
the same resolution to `dsh_composite`. A selected executable that does not
answer is reported missing by its own name, no PATH decoy is tried, and the
producer is not called at all. A failed `$DSH_HOME` is a named composite
failure that leaves the observed version visible. Non-DSH providers are
untouched.

`composite_detail` now implements D10's full table: an unreadable composite
keeps its declaration context (`no declared wrapper_digest`, or
`declared wrapper_digest <digest>; comparison unavailable`) and asserts
neither a match nor a difference, and its reason goes through `Safe`. The
guide sample carries the producer-derived measured-fixture digests, and two
tests hold it to the emitted vocabulary rather than to a transcription.

### Compiling removal controls

Each mutation was applied alone, its focused test run, then restored. All
gates below ran on the restored bytes.

| # | Production mutation | Test | Assertion that failed |
|---|---|---|---|
| A1 | `agents.rs` `ResumeIdentity::value` drops the `wrapper_digest` member | `a_declared_wrapper_digest_reaches_the_private_start_context` | "the exact declared member reaches the private start context": `Null` vs `aaaa…` |
| A2 | `load.rs` `is_lower_hex_64` accepts `is_ascii_hexdigit` | `the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name` | the uppercase case loads instead of refusing (`unwrap_err` on `Ok`) |
| A3 | `load.rs` measured key list drops `wrapper_digest` | same | the well-formed member is refused: "unknown key 'wrapper_digest'" |
| A4 | same as A1 | same | `shape.identity.value()["wrapper_digest"]`: `Null` vs the digest |
| B1 | `walk` grants the `node_modules` exception at any depth | `the_plugin_component_is_bytewise_path_order_and_fails_closed` | "unexpected directory 'lib/node_modules'" no longer refuses |
| B2 | `walk` uses `to_string_lossy` for the entry name | `the_plugin_walk_refuses_a_name_that_is_not_utf8` | reason became "unexpected entry 'LICENSE\u{fffd}'" |
| B3 | `walk` drops the expected-membership guard | `the_plugin_component_is_bytewise_path_order_and_fails_closed` | "unexpected entry 'extra.txt'" no longer refuses |
| B4 | `walk` admits any directory | `only_an_ancestor_of_a_declared_file_is_a_walkable_directory` | "unexpected directory 'empty'" no longer refuses |
| B5 | `npm_dependencies` excludes by NAME | `npm_exclusions_name_exact_records_and_keep_same_named_registry_ones` | `[]` vs the retained `@deepseek-ai/dsh 0.1.4` and registry `dsh-plugin-cli-session` |
| B6 | `npm_dependencies` deduplicates by name | `the_measured_locks_yield_the_complete_ordered_dependency_set` | "521 records deduplicate to 501": `489` vs `501` |
| B7 | `npm_name` returns the FIRST group | `the_measured_npm_keys_bind_to_their_terminal_spellings` | `@aws-sdk/credential-provider-http` vs `@smithy/node-http-handler` |
| B8 | `read_pnpm` bound made exclusive (`>=`) | `the_pnpm_reader_is_bounded_inclusively_at_the_limit` | the exact-limit file no longer reaches grammar validation |
| B10 | `scalar_reason` checks only `\n` | `npm_versions_with_any_whitespace_are_unreadable` | `" 1.0.0"` produced a triple instead of refusing |
| B11 | `spawn_node_runtime_at` restores `trim()` | `spawn_node_runtime_reads_one_version_line_and_refuses_the_rest` | the padded banner was repaired to `v1.2.3` |
| B12 | `home_patch` maps any metadata failure to `absent` | `a_dangling_home_patch_symlink_is_unreadable_rather_than_absent`, `a_present_unreadable_home_patch_is_not_absence` | `"absent"` returned for a failed observation |
| B13 | `patch_digest` raises `Component` | `an_unreadable_profile_patch_is_not_a_plugin_component_failure` | the profile patch failed as "plugin component is unreadable" |
| B14 | `resolve_bundle` continues past an outside first hit | `an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one` | the later inside candidate was used instead of refusing |
| B15 | `read_profile` keeps the RAW directory as the containment boundary | `the_dsh_composite_accepts_a_symlinked_home_ancestor` | the symlinked home's legitimate bundle was refused — the inherited false refusal, reproduced |
| B16 | `resolve_core_reading` reopens the selected manifest | `the_core_manifest_and_hidden_lock_are_read_once_and_retained` | "the selected core manifest is opened once": `2` vs `1` |
| B17 | `canonical_composite` drops the `home-patch` line | `the_measured_install_yields_…_pinned_digests`, `the_measured_composite_moves_with_every_component_it_names` | the pinned canonical digest moved, and the home-patch movement vanished |
| B18 | `canonical_composite` sorts the bundle lines | `the_measured_composite_moves_with_every_component_it_names` | the reordered declaration no longer moved the composite |
| B19 | `resolve_bundle` searches the profile before the core | four composite tests | each containment refusal became a success |
| C1 | `dsh_provider_line_with` probes `adapter.binary` | `the_dsh_version_and_composite_come_from_one_resolved_installation`, `the_dsh_seam_precedence_moves_the_version_and_the_composite_together` | "the version probe reads the SELECTED executable": `["dsh"]` vs the sentinel |
| C2 | the unreadable arm drops its declaration context | `the_dsh_composite_detail_reports_each_disposition` | `(no declared wrapper_digest)` vs `(declared wrapper_digest …; comparison unavailable)` |
| C3 | the unreadable reason skips `Safe` | same | the escape sequence reached the rendered line |
| C4 | the composite is computed for an unanswering binary | `a_failed_selected_executable_is_never_retried_against_a_path_decoy` | the injected producer's "not computed" panic fired |
| C5 | a failed home seam returns no version | `a_failed_home_seam_leaves_the_version_visible_beside_the_reason` | `None` vs `Some("0.1.5-rc.2")` |
| C6 | `composite_detail` changes its readable wording | `the_guide_documents_the_wording_the_classifier_emits` | the guide sample no longer matched the emitted string |

One control found a defect in this visit's OWN test rather than in
production: the hermetic seam-precedence case first passed under C1 because
its child filter used a bare function name, and a libtest filter that matches
nothing exits zero. The child now runs the full `doctor::tests::…` path and
the parent asserts the child actually ran a case before reading its status.

### Gates, on the restored final bytes

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-protocol --all-features --locked` | 338 + 99 + 1 pass, 0 fail |
| `cargo test -p brokkr-cli --all-features --locked` | pass, 0 fail |
| `cargo test -p brokkr-runtime --all-features --locked` | pass except three PRE-EXISTING failures (below) |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles |
| `bash scripts/coverage-exact.sh` | **PENDING** — see below |
| `openspec validate --all --strict` | **PENDING** — see below |

**Three pre-existing runtime failures, not this slice's.** `gpt_flash_shape`
(2), `roster` (1) and `witness_digests` (1) fail at this HEAD before any edit
of this visit. Verified by running them in a clean worktree at `310c5864`:
byte-identical failures, including `recipes/gpt-flash manifest digest moved`
`20b1ba15…` vs `2b6623f1…`. They follow commit `99fdbb0a`, which pinned every
gate, chief and council office to astra without moving the recipe's pinned
manifest digest or the panel-diversity expectations. Repairing them belongs to
that commit's owner, not to 8.8(a)–(c).

**The exact-coverage gate is pending, for two separate reasons.** First,
`bash scripts/coverage-exact.sh` cannot be launched from this seat at all —
script execution is refused, as `cargo`/`openspec` were for the tasks seat.
Second, and independently: the gate's instrumented run is
`cargo llvm-cov --workspace`, and `cargo-llvm-cov` abandons report generation
when the test run fails. While those three pre-existing failures stand, the
gate cannot produce a report on ANY bytes at this HEAD, this slice's included.
Neither reason is a pass, and none is inferred.

What WAS measured, with the pinned `nightly-2026-09-05` compiler, is a
crate-scoped instrumented run over the three touched crates' library suites
(`cargo +nightly-2026-09-05 llvm-cov --lib -p brokkr-protocol -p brokkr-cli
-p brokkr-runtime --all-features --locked --branch`):
`crates/brokkr-protocol/src/adapters/composite.rs` reports **FNF 100 / FNH
100, BRF 174 / BRH 174, LF 844 / LH 844** — no uncovered line, branch or
function. In `doctor.rs` the only zero-hit records are pre-existing paths the
crate's INTEGRATION tests cover and this scoped run excludes (`unbuilt`, the
boxed-probe error arm and the public `doctor` entry point); the whole
171–300 region this visit rewrote is covered. This is evidence about the new
lines, NOT a substitute for the gate, which stays red-and-unrun above.

`openspec validate --all --strict` could not be launched from this seat
either. The only artifact this visit changed under `openspec/` is this
account and fourteen checkboxes inside an existing section of `tasks.md`; no
delta, requirement or scenario was touched, and the whole-change ledger is
unchanged at **101 global identifiers / 84 checked / 17 pending**, 8.8 among
the pending.

### What the four unticked local clauses still owe

- **Retain every identity-bearing source once per call.** The manifest and
  the hidden lock are delivered and counted (B16); `npm_dependencies` takes
  the retained `&Value` and cannot reopen. The plugin `cordis.patch.yml`
  reuse is delivered and asserted by value identity, but it has **no
  behavioural removal control**: reopening an unchanged file yields the same
  digest, so no assertion can distinguish it. A counting seam for the
  component walk's file reads would close this; it is not built.
- **The bounded pnpm reader.** The inclusive comparison is proved (B8) and
  `the_pnpm_bound_stops_an_endless_source_that_reports_no_size` shows the
  bound is applied to bytes CONSUMED, using `/dev/zero`. But removing the
  `take` itself does not fail an assertion — it makes that test consume
  memory without terminating, which is the harm the bound prevents rather
  than a failing assertion, and it was **not executed** so as not to exhaust
  the host. The closed-grammar arms were not removed one at a time.
- **D6's locator chain.** Preserved and covered by the existing executable,
  core, profile and layout tests, but this visit ran **no removal control**
  on the shebang, `bin.dsh`, core-root or hidden-lock-vs-root-lock guards.
  The clause asks for each in isolation.
- **Gates on final restored bytes.** Pending the exact-coverage gate and
  `openspec validate`, for the reasons above.

### Explicitly not done, and still pending

Part **(d)**'s planner behaviour was not started, planned or touched, and no
planner call site changed. **8.10**'s complete whitespace and
rejection-vector ledger, **9.6**, **10.6–10.8** (including 10.7's
retained-home doctor recording), **11.1–11.4** and groups **14** and **15**
remain their owners'. No `wrapper_digest` was declared in adapter data, no
shape moved to `supported`, and the DSH route stays disabled and unmeasured
for admission. Proposed decision 0056 keeps its `proposed` status. The frozen
contracts, `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/` and every byte under `extensions/dsh/` are untouched. No
provider was installed, probed for qualification, or re-measured; the
supplied qualification was adopted as given. Nothing was pushed.

One residual observed but deliberately left alone: the provider-adapters
guide's adapter-shape table still describes the selected core as **0.1.5-rc.1**
while the qualification and this fixture are rc.2. That row is the route
pin's, not 8.8(a)–(c)'s, and correcting it here would edit a version
reference this commission does not own.

### Returned implement — council return of 2026-09-19, findings F1–F12

This visit owns the review return on `29eaf8ff`, not a fresh reading of
8.8. Every production change below answers a named finding; nothing else
in (a)–(c) was reopened. **8.8 stays unchecked**, part (d) was not
started, and the 8.10 rejection-vector ledger remains its owner's.

#### What each finding cost, in production

**F1 — the pnpm package key.** `pnpm_package` now crosses the whole key
through `scalar_reason` and the package-name grammar BEFORE any exclusion
or serialization. `a b@1` and `a@b 1` split into different name/version
pairs that serialized to the identical line `a b 1 <integrity>`; the
whole-key scalar rule is what separates them. A NUL or control byte in
either half no longer reaches the hashed stream.

**F2 — the lockfile-9 grammar is closed.** `pnpm_dependencies` is a state
machine over a named set of top-level keys (`PNPM_SECTIONS`), a named set
of `packages:` children (`PNPM_PACKAGE_CHILDREN`) and a named set of
resolution fields (`PNPM_RESOLUTION_KEYS`); indentation must be even and
must belong to a recognized state; a document carrying an unrecognized key
is unreadable rather than partially read. `pnpm_scalar` unwraps quotes
EXACTLY — `trim_matches` repaired `'sha512-X` into `sha512-X` — and
`pnpm_flow_map` matches field keys WHOLE, where the substring search read
`xintegrity:` and `fakeintegrity:` as `integrity:` and let a changed
actual integrity leave identity unmoved. A carriage return anywhere is a
refusal, because `str::lines` read a CRLF document as a Unix one. A
document with no `packages:` section refuses instead of answering with an
empty dependency set.

*Its scope, stated rather than implied.* The grammar is closed over the
document's shape, over every byte identity is taken from, and over the
`packages:` records themselves. The BODIES of the recognized sections
that carry no identity — `importers`, `snapshots` and their kin — are
skipped by design: only an indent-0 key opens a section and only
`packages` admits a record, so nothing under them can reach a triple.

**F3 — a bundle candidate is inspected, not guessed at.** `resolve_bundle`
replaced `Path::is_file` with fallible metadata: only `NotFound`
continues the search; a permission failure, a symlink loop or a
`package.json` that is not a regular file is unreadable and names the
locator and reason. `is_file` answered `false` for a failure exactly as
it answers `false` for absence, which authorized describing a different
installed copy further down the chain.

**F4 — one resolution, the child's own rules.** `is_executable_file`
requires a regular file carrying an execute bit, and an EMPTY `PATH`
entry names the working directory — both what a spawning child does.
`DshSeams::selected` resolves the declared name ONCE and returns the
resolved path, so doctor's probe target and the producer's input are the
same string; a name that resolves to nothing keeps its declared spelling
so the report still names what was looked for. `DshSeams::selected_from`
is the injected seam that makes each arm a test rather than a fact about
the host's installed `dsh`.

**F5 — the executable's two requirements, separately.** `resolve_core`
now also requires the canonical executable to BE `<core>/lib/bin.js`,
independently of the manifest agreeing with it, and `first_line` no
longer strips a carriage return — `#!/usr/bin/env node\r` is not the
first line D6 measured, and the kernel cannot execute it.

**F6 — the observation is sealed.** Every `DshComposite` member is
private to the producer's module; `canonical()` and `plugin()` are the
read access production needs; `DshComposite::synthetic` is `cfg(test)`.
The CLI's `composite_identity` unit test, which assembled an observation
field by field from chosen strings, is gone — its mapping is asserted
against a REAL observation in the doctor child instead.

**F9 — each failure names its own component.** `CompositeError::Component`
carries the component name, so extension drift reads `extension component
is unreadable` and no longer sends an operator to the plugin directory.
`read_json` returns a reason and each caller names the component: the
hidden npm lock's failures are the NPM LOCK's, not "the DSH layout".

**F10** the guide test provokes the bound with a finite oversized file
rather than `/dev/zero`, which exists only on Unix. **F11** the seams
test asserts the ADAPTER's configured selection rather than the literal
`dsh`, and each arm of the selection is now driven through
`selected_from`; this seat reproduced the reported failure under a
configured `BROKKR_DSH_BIN` before fixing it. **F12** is a run defect
about a member's note, with no source to answer.

#### F7 and F8 — the proofs

The doctor precedence control is now hermetic in every case. Its children
carry a `PATH` holding a scripted `node` and a `dsh` symlinked into a
third installation, so the producer never depends on this machine's
runtime and the `neither` case selects a KNOWN install instead of
whatever the host has — the conditional arm that let a failing child pass
is gone. The version probe reads the version out of the SELECTED
installation's own `package.json` rather than returning a constant, and
the child computes the composite of BOTH the chosen and the rejected
install through the real producer and asserts the reported suffix equals
the chosen one's exactly, digest and plugin component. The companion test
records `seams.executable` as well as the home. The real
`dsh_provider_line` — production's own seam resolver and producer closure
— runs in the same child and is asserted against the same expected line.

Bare `is_err()` is gone from the composite refusal suites: the two pnpm
construct tests are tables of (input, exact reason) — every closed-grammar
arm carries its own vector — and
`read_json_and_first_line_report_io_and_encoding_failures` asserts
reasons rather than error-ness. The loader's malformed-member table asserts the
RESPONSIBLE reason per case — the grammar refusal quotes the offending
value, the mistyped cases fail the string rule — where
`contains("wrapper_digest")` could not tell them apart.

#### Removal proofs, R1–R15

Each row is one compiling mutation, undone before the next. Every failure
is an ASSERTION, never a compile or setup error; `expected a refusal` is
the `refused()` helper's panic, which fires when a mutated reader ACCEPTS
a document it must refuse.

| # | Finding | Production mutation | Test | Failure |
|---|---------|---------------------|------|---------|
| R1 | F1 | drop the `scalar_reason(key)` check in `pnpm_package` | `two_package_keys_that_serialized_to_one_line_are_both_refused` | `left: "…'a b@1': 'a b' is not a package name"` vs `right: "…the key carries whitespace"` |
| R2 | F2 | `pnpm_flow_map` matches keys by substring again | `pnpm_locks_reject_every_unrecognized_construct` | `expected a refusal` — `xintegrity` parses |
| R3 | F2 | drop the carriage-return refusal | same | `expected a refusal` on the CRLF document |
| R4 | F2 | `pnpm_scalar` back to `trim_matches` | same | `expected a refusal` on the unterminated scalar |
| R5 | F2 | drop the `PNPM_SECTIONS` refusal | same | `expected a refusal` on `rogue:` |
| R6 | F3 | restore `is_file()` ahead of the metadata inspection | `a_bundle_candidate_that_cannot_be_inspected_stops_the_search` | `expected a refusal` — the sealed candidate is skipped for the profile copy |
| R7 | F4 | `is_executable_file` back to `metadata.is_file()` | `path_resolution_walks_past_a_candidate_a_child_could_not_execute` | `left: "…/first/dsh"` vs `right: "…/second/dsh"` |
| R8 | F4 | skip the empty `PATH` entry | `an_empty_path_entry_is_the_current_directory` | child: `Config("'mytool' is not on PATH")`; parent: the child's exit assertion |
| R9 | F4 | `selected` returns the declared name unresolved | `the_dsh_seam_precedence_moves_the_version_and_the_composite_together` | `case neither: the seam's choice`, `left: "dsh"` vs the resolved path |
| R10 | F5 | `measured_bin` aliased to the canonical executable | `the_core_executable_must_be_lib_bin_js_with_the_exact_shebang` | `expected a refusal` — `lib/other.js` passes |
| R11 | F5 | restore the CR strip in `first_line` | same | `expected a refusal` — the CRLF shebang passes |
| R12 | F9 | hidden lock mapped to `Config` | `an_unreadable_npm_lock_and_extension_are_named_by_their_own_component` | `the DSH layout is unreadable: …/.package-lock.json: not JSON: …` |
| R13 | F9 | extension walk named `"plugin"` | same | `left: "plugin component is unreadable: missing expected file 'index.js'"` vs `right: "extension component…"` |
| R14 | F8 | loader grammar reason reduced to `is not acceptable` | `the_optional_wrapper_digest_member_loads_carries_and_is_refused_by_name` | the refusal no longer contains the grammar sentence |
| R15 | F4/F7 | `doctor.rs` probes `adapter.binary` again — the measured 2026-09-19 defect | `the_dsh_seam_precedence_moves_the_version_and_the_composite_together` | `case both: the version probe reads the selected executable, once`, `left: ["dsh"]` vs the selected path |

**F6 is a visibility change, and visibility is not behavioural removal
evidence.** Sealing the members makes external construction a compiler
error; this account does not credit that as a proof. What IS asserted is
that the only remaining constructor outside the module is `cfg(test)`,
and that the CLI's field-by-field assembly is gone.

#### Gates, on final restored bytes

| gate | result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-protocol --all-features --locked` | 344 + 99 + 1 pass, 0 fail |
| `cargo test -p brokkr-cli --all-features --locked` | 30 binaries, all ok (461 lib) |
| `cargo test -p brokkr-runtime --all-features --locked` | 441 lib pass; **3 pre-existing integration failures**, below |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles |
| `bash scripts/coverage-exact.sh` | **PENDING** — below |
| `openspec validate --all --strict` | **PENDING** — this seat's shell refuses `openspec` |

**The three runtime failures are not this slice's.** `gpt_flash_shape`
(2) and `roster` (1) fail on panel diversity and office pinning —
`analyze:judge pins the wrong model, left: Some("astra"), right:
Some("sol")` and `chore panel lost its GPT/Flash diversity, left:
{"codex"}, right: {"codex", "dsh"}`. They follow commit `99fdbb0a`, which
pinned every gate, chief and council office to astra. No file this visit
touched is read by them.

**The exact-coverage gate is pending, and for the same reason as before.**
The gate's instrumented run is `cargo llvm-cov --workspace`, and
`cargo-llvm-cov` abandons report generation when the test run fails;
while those three failures stand it can produce a report on NO bytes at
this HEAD, this slice's included. That is not a pass and none is inferred.

What WAS measured, with the pinned `nightly-2026-09-05`, is an
all-target instrumented run of the two crates this visit changed in
production (`cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli
-p brokkr-protocol --all-features --locked --branch`), evaluated by the
gate's OWN counting rule — every `DA` and `BRDA` record hit:

- `crates/brokkr-protocol/src/adapters/composite.rs`: **FNF 115 / FNH
  115, BRF 214 / BRH 214**, and no zero-hit `DA` record.
- `crates/brokkr-cli/src/doctor.rs`: no zero-hit `DA` or `BRDA` record.
- `crates/brokkr-runtime/src/agents/load.rs`, in the crate-scoped lib
  run: no zero-hit `DA` or `BRDA` record.

Three uncovered records the first measurement found were CLOSED rather
than excused: the two `pnpm_scalar` quote arms gained refusal vectors,
and `dsh_provider_line`'s producer closure gained the real-entry
assertion in the doctor child. One of them was not a missing test but a
missing possibility — the block-depth guard `Some(open) if indent > open`
could not be falsified by any input, because a block opens only at four
spaces and only a line at six or more consults it. An unreachable guard
is a claim the code cannot keep, so the state became a flag. This is
evidence about the changed lines; it is NOT the gate, which stays unrun.

**One flake was found and closed.** Two instrumented runs failed
intermittently in `brokkr-protocol --lib` immediately after a rebuild. A
test that re-executes its own binary can reach `exec` while another
thread still holds a write descriptor to a file it staged, and the kernel
answers `Text file busy` (#255). Both re-execution sites now retry that
one error and nothing else. It would have reached CI as an unexplained
red.

#### Still not delivered, and named

- **Part (d)**, the planner, was not started, planned or touched. The
  only planner-adjacent edits are mechanical: `value.canonical` became
  `value.canonical()` at `adapters.rs:3547`, and `synthetic_dsh_composite`
  now calls the producer's test-only constructor. No decline, gate or
  route behaviour changed.
- **8.10**'s whitespace and rejection-vector ledger; **9.6**;
  **10.6–10.8**; **11.1–11.4**; groups **14** and **15**.
- The two removal controls named at the unticked clause above: the plugin
  patch reread and the bounded reader's `take`.
- The **core-root** and **hidden-lock-vs-root-lock** guards still have no
  isolated removal control of their own; the shebang, `bin.dsh`,
  `lib/bin.js` and hidden-lock attribution guards now do.
- `openspec validate --all --strict` and the **exact-coverage gate**.
- No `wrapper_digest` was declared, no shape moved to `supported`, the
  DSH route stays disabled and unmeasured, and decision 0056 keeps its
  `proposed` status. The frozen contracts, `policy/phase-machine.json`,
  `policy/schemas/`, `fixtures/`, `reference/` and every byte under
  `extensions/dsh/` are untouched. Nothing was pushed.

### Returned implement — second council return of 2026-09-19, findings 1–5

This visit owns the review return on `75b5517c` (run
`dsh-composite-identity-issue-226-26def5a5`, review by `gpt-6-astra`):
five deduplicated findings, two of them HIGH on specification compliance
and one HIGH on proof completeness. Every production change below answers
a named finding. **8.8 stays unchecked**, part (d) was not started, and
8.10's rejection-vector ledger remains its owner's. The working tree this
visit inherited already carried an uncommitted first answer to findings
1, 2, 3 and 5 and part of 4; it was read, kept where it was right, and
corrected where it was not — its reader had lost the bounded `take`
outright, which is the hazard finding 4 named, not the fix.

#### What each finding cost, in production

**Finding 1 — a document key is a singleton and its form is fixed.**
`pnpm_dependencies` tracks every top-level key it has spelled; a second
spelling of any of them is `a repeated top-level key '<name>'`, whether the
second is `packages: null`, `packages: {}`, a second block-form `packages:`
or a late `lockfileVersion: '8.0'`. The form is required of the KEY, not
read off the line: `lockfileVersion`, `packageExtensionsChecksum` and
`pnpmfileChecksum` carry a scalar, which is parsed even though it is not
read, and every other recognized key opens a block, so `packages: null`
with no earlier block is `an inline value on top-level section 'packages'`
rather than an empty package set. The inherited reader decided the form
from the line and kept the first block's triples as the document's answer
— an ambiguous lock reporting an unchanged identity.

**Finding 2 — a scalar is one of three forms, never YAML syntax.**
`pnpm_scalar` admits a single-quoted scalar (no inner `'`), a double-quoted
scalar with NO backslash at all, and a plain scalar that carries no quote,
no `#` and no opening indicator from the YAML set (`- ? : , [ ] { } # & *
! | > ' " % @ \``). `*undefined`, `[sha512-X]`, `&anchor`, `!!str`,
`"sha512-A"` and `"a\tb"` had each entered identity as their own
spelling, so an alias target or a sequence member could change under a
digest that did not, and an escaped tab defeated the whitespace rule.
`PNPM_RESOLUTION_KEYS` is D6's vocabulary and nothing more — `integrity`
with an optional `tarball`; `commit`, `directory`, `path`, `registry`,
`repo` and `type` are refusals, where the inherited list named them and
then ignored them, reading a git or directory resolution as a registry
one. Every field of a resolution flow map is a singleton, not only the one
read.

**Finding 3 — executability is the kernel's answer for THIS process.**
`is_executable_file` asks `faccessat(AT_EACCESS)` through `rustix`, which
is the check `execve` itself makes, instead of `mode & 0o111 != 0`, which
asks whether ANYONE may execute the file. A mode-0641 candidate carries an
execute bit for others and none for its owner, so the inherited test said
yes while the owner's child got `EACCES` and walked on — the two-installs
disagreement 8.8(c) exists to close, reached by permission rather than by
absence. The regression case asserts whichever answer the running identity
is entitled to (POSIX grants a privileged process `X_OK` on any file with
an execute bit) and detects privilege with a mode-0000 file rather than
skipping.

**Finding 4 — the proofs that did not exist.** Each is named in the table
below: the plugin-patch reread (R24), the bounded reader's `take` (R25),
the core-root guard (R26), the hidden-lock-versus-root-lock pair (R27,
R28), a composition-level hidden-lock reread (R29), string-prefix
containment (R30), the altered lookup anchor (R31), every serialized
element and the order and the deduplication (R32–R42), an omitted
differing triple (R43), a built-in bundle at the wrong anchor (R44), each
classifier and warning arm (R45–R48) and the real entry's reason (R49).
B19 is re-run and its "four composite tests" are now five, named. The
last bare `is_err()` in the composite suite
(`resolve_core_refuses_an_executable_with_no_dsh_ancestor`) asserts the
canonical path and the package looked for; the three containment refusals
assert the exact outside path rather than a prefix; and the two new
refusal tables name an ACCEPTED vector in their panic
(`refused_vector`), so a control that lets one through reports which.

Two production seams were added for these proofs, both injected and
test-only in their injected form: `read_pnpm_from(path, source)` beneath
`read_pnpm`, so a counting reader can report consumption; and
`dsh_composite_reading` now takes the JSON reader as well as the directory
reader, so a count taken across the WHOLE observation can see a reopen
made after `resolve_core` has returned, which a count inside it could not.

**Finding 5 — the producer closure is reached by a real failure.** The
seam-precedence child gained an `unreadable` case: the real
`dsh_provider_line` over a selected installation whose `DSH_HOME` holds no
`profiles/headless`. The version probe still answers for that executable
and the producer's refusal reaches the line through production's own
`map_err`, asserted by the component's reason, the declaration context and
the absence of a warning. The report this visit measured records that
closure's line (`doctor.rs:213`) hit 4 times.

#### Decisions

- **`rustix` becomes a direct dependency of `brokkr-protocol`.** The
  breakdown says "add no dependency"; finding 3 asks for the child's own
  executability semantics, and the standard library exposes no
  `faccessat`. `rustix` 1.1.4 is already a direct dependency of
  `brokkr-cli` under decision 0055's narrow exception for the transcript
  reader, so this extends an edge to a version already in `Cargo.lock` —
  no registry version moves and no new crate enters the graph. The
  alternative, a hand-rolled effective-identity check over `metadata`,
  would have re-implemented the kernel's rule (owner, group, supplementary
  groups, capabilities) and been wrong in exactly the cases the finding
  is about.
- **The `/dev/zero` control is gone, not kept beside the counting one.**
  A control whose failure mode is exhausting the host is not a control;
  the counting reader proves the same fact (the bound applies to bytes
  CONSUMED, not to a metadata size) with a finite source, and the
  file-backed path is still exercised by
  `the_pnpm_reader_is_bounded_inclusively_at_the_limit`.
- **The extension line's isolated control lives in the serializer test
  alone.** At the producer level a listed extension also moves the
  `profile-bundle` lines, so the two producer-level extension tests cannot
  tell a dropped `extension` line from a present one; the named serializer
  row can, and did (R40).

#### Removal proofs, R16–R49

Each row is one compiling mutation, applied alone, its focused test run,
then undone before the next. Every failure is an ASSERTION or the named
`was accepted` panic of `refused_vector`; none is a compile or setup
error. The seat ran as uid 1000, so R23's expected selection is the
unprivileged one. Line numbers are the test file's at the moment of the
run.

| # | Finding | Production mutation | Test | Failure |
|---|---------|---------------------|------|---------|
| R16 | 1 | `pnpm_dependencies` records a top-level key without refusing a repeat | `a_pnpm_document_key_is_a_singleton_and_its_form_is_fixed` | `"packages: null"`: `left: "…an inline value on top-level section 'packages'"` vs `right: "…a repeated top-level key 'packages'"` |
| R17 | 1 | the inherited line-based form decision (`section = value.is_empty().then_some(name)`) replaces the inline-value refusal | same | `"lockfileVersion: '9.0'\npackages: null\n" was accepted` |
| R18 | 1 | a scalar-valued top-level key is no longer parsed | same | `pnpmfileChecksum:` opening a block: `left: "…a child line outside every section"` vs `right: "…a malformed scalar for top-level key 'pnpmfileChecksum'"` |
| R19 | 2 | `pnpm_scalar` admits an opening indicator | `a_pnpm_scalar_is_one_of_three_forms_and_never_yaml_syntax` | `"*undefined" was accepted` |
| R20 | 2 | a double-quoted scalar admits a backslash | same | `"\"sha512-\\u0041\"" was accepted` |
| R21 | 2 | a plain scalar admits `#` | same | `"sha512-X #note"`: `left: "…'debug@2.6.9': integrity carries whitespace"` vs `right: "…a malformed resolution flow map"` |
| R22 | 2 | `PNPM_RESOLUTION_KEYS` back to the inherited eight | same | `"directory: /tmp/x" was accepted` |
| R23 | 3 | `is_executable_file` back to `mode & 0o111 != 0` | `path_resolution_walks_past_a_candidate_a_child_could_not_execute` | "the candidate is taken exactly when THIS process could execute it": `left: "…/other-only/dsh"` vs `right: "…/second/dsh"` |
| R24 | 4 | `dsh_composite_reading` reopens `cordis.patch.yml` for `plugin-patch` | `the_plugin_patch_cannot_be_a_second_read_of_a_changed_file` | "plugin-patch is the walk's observation, not a later reopen": `left: "05fefe2e…"` vs `right: "66e6d923…"` |
| R25 | 4 | `read_pnpm_from` loses its `take` | `the_pnpm_reader_consumes_at_most_one_byte_past_the_limit` | "exactly one byte past the limit is consumed, of a source holding 4096 more": `left: 8392704` vs `right: 8388609`, in 0.01 s |
| R26 | 4 | the core-root guard is removed | `resolve_core_refuses_a_manifest_that_does_not_match_its_binary_or_scope` | `left: "npm lock is unreadable: …/core/node_modules/.package-lock.json: No such file or directory"` vs `right: "the DSH layout is unreadable: core package is not at <core root>/node_modules/@deepseek-ai/dsh"` |
| R27 | 4 | the hidden lock falls back to `<core root>/package-lock.json` | `the_root_package_lock_is_never_read_beside_or_instead_of_the_hidden_one` | `left: "the DSH layout is unreadable: core lock version 9.9.9 differs from package version 0.1.5-rc.2"` vs `right: "npm lock is unreadable: …/.package-lock.json: No such file or directory"` |
| R28 | 4 | the root lock is preferred over the hidden one | same | "the core line comes from the hidden lock, not the root one": `left: "@deepseek-ai/dsh 0.1.5-rc.2 sha512-DECOY"` vs `right: "… sha512-CORE"` |
| R29 | 4 | the hidden lock is reopened at the composition level for the triples | `the_core_manifest_and_hidden_lock_are_read_once_and_retained` | "the whole observation opens the hidden lock once": `left: 2` vs `right: 1` |
| R30 | 4 | `resolve_bundle` decides containment by string prefix | `containment_compares_canonical_components_not_string_prefixes` | `left: "…the plugin resolves outside the profile"` vs `right: "…bundle 'dsh-plugin-cli-session' resolves outside the core root and the profile (…/headless-extra/dsh-plugin-cli-session)"` — the sibling passed the lookup and only the later plugin guard caught it |
| R31 | 4 | the canonical profile directory is used as the lookup anchor | `the_dsh_composite_accepts_a_symlinked_home_ancestor` | "the search walked the RAW anchor's ancestry and stopped at its outside first hit": `left: "…bundle '@deepseek-ai/dsh-base' does not resolve"` vs `right: "…resolves outside … (…/aliases/node_modules/@deepseek-ai/dsh-base)"` |
| B19 | 4 | `resolve_bundle` searches the profile before the core (re-run) | `a_bundle_candidate_that_cannot_be_inspected_stops_the_search`, `an_outside_first_bundle_hit_is_not_skipped_for_a_later_inside_one`, `containment_compares_canonical_components_not_string_prefixes`, `the_dsh_composite_refuses_a_layout_outside_the_locators`, `the_plugin_and_extension_must_resolve_inside_the_profile` | each containment refusal became a success: `expected a refusal` ×3, `unwrap_err()` on an `Ok` ×2 |
| R32 | 4 | no `core` line | `the_canonical_composite_orders_lines_and_moves_with_its_inputs`; `the_measured_install_yields_its_recorded_components_and_pinned_digests` | "the `core` line moves the composite" (equal digests); the pinned canonical `a64fcd6d…` became `47f6eef9…` |
| R33 | 4 | no `node` line | same | "the `node` line moves the composite"; pinned became `4680b3fc…` |
| R34 | 4 | no `dependency` lines | same | "the `dependency (from npm)` line moves the composite"; pinned became `1dfe2e14…` |
| R35 | 4 | no `plugin` line | same | "the `plugin` line moves the composite"; pinned became `ed0e9bb4…` |
| R36 | 4 | no `plugin-patch` line | same | "the `plugin-patch` line moves the composite"; pinned became `cefdb493…` |
| R37 | 4 | no `profile-patch` line | same, and `the_measured_composite_moves_with_every_component_it_names` | "the `profile-patch` line moves the composite"; pinned became `9c597ad2…`; "the profile patch moves the measured composite" |
| R38 | 4 | no `profile-bundle` lines | same three | "the `profile-bundle (declared order)` line moves the composite"; pinned became `d7c16143…`; "the declared bundle order moves the measured composite" |
| R39 | 4 | no `profile-patch-reload` line | same three | "the `profile-patch-reload` line moves the composite"; pinned became `0ef4446f…`; "the reload mode moves the measured composite" |
| R40 | 4 | no `extension` line | `the_canonical_composite_orders_lines_and_moves_with_its_inputs` | "the `extension` line moves the composite" — the two producer-level extension tests stayed green, see Decisions |
| R41 | 4 | dependencies concatenated without deduplication | serializer test; pinned digest | "an equal complete triple from both locks is ONE dependency line"; pinned became `3d085ff8…` (the measured locks share three triples) |
| R42 | 4 | `node` serialized ahead of `core` | `the_measured_install_yields_its_recorded_components_and_pinned_digests` | "the canonical composite over the measured inputs is the producer's pinned output": `15638ea6…` vs `a64fcd6d…` — order is caught by the pin alone, as every element still moves |
| R43 | 4 | `npm_dependencies` omits EVERY record of `@smithy/node-http-handler` 4.7.3, one of that name's two versions | `the_measured_locks_yield_the_complete_ordered_dependency_set`; pinned digest | "521 records deduplicate to 501 distinct complete triples": `left: 500` vs `right: 501`; pinned became `9d77441c…` |
| R44 | 4 | FIXTURE: `@deepseek-ai/dsh-base` materialized under the profile instead of the core | `the_measured_install_yields_its_recorded_components_and_pinned_digests` | "@deepseek-ai/dsh-base resolves at the core anchor": `left: false` vs `right: true` |
| R45 | 4 | `composite_detail` loses its `matches` arm | `the_dsh_composite_detail_reports_each_disposition` | the equal digest rendered `differs from the declared wrapper_digest aaaa…` |
| R46 | 4 | a readable difference warns without a `supported` shape | same | `assertion failed: !composite_detail(Some(&digest), false, Ok((&other, "plugin"))).0` |
| R47 | 4 | an unreadable composite warns without a `supported` shape | same | `assertion failed: !warning` at the unsupported-with-declaration row |
| R48 | 4 | an unreadable composite warns with no declared digest | same | `assertion failed: !warning` at the supported-without-declaration row |
| R49 | 5 | `dsh_provider_line`'s closure drops the producer's reason (`"composite unreadable"`) | `the_dsh_seam_precedence_moves_the_version_and_the_composite_together`, case `unreadable` | "the line carries the component's own reason and the declaration context": `left: "composite unreadable: composite unreadable (no declared wrapper_digest)"` vs `right: "composite unreadable: the DSH layout is unreadable: …/broken-home/profiles/headless: No such file or directory (os error 2) (no declared wrapper_digest)"` |

**One control had to be redone before it fired, and the reason is worth
keeping.** R43's first form omitted a single nested record
(`node_modules/@aws-sdk/credential-provider-http/node_modules/@smithy/node-http-handler`)
and no assertion moved: that record's complete triple is carried by
another record too, and equal complete triples deduplicate by design. An
omitted DIFFERING triple is one whose every record is gone, which is what
the second form does. R30 shows the same kind of thing from the other
side: the string-prefix lookup admitted the sibling, and the later
plugin-inside-the-profile guard refused it anyway with a different
reason. That guard is defence in depth, not the containment rule; the
tightened assertion tells the two apart.

#### Gates, on final restored bytes

| gate | result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `git diff --check` | clean |
| `cargo test -p brokkr-protocol --all-features --locked` | 348 + 99 (2 ignored) + 1 pass, 0 fail |
| `cargo test -p brokkr-cli --all-features --locked` | 30 binaries, all ok (461 lib), 0 fail |
| `cargo test -p brokkr-runtime --all-features --locked --no-fail-fast` | 441 lib pass, 21 binaries ok; **the same four pre-existing failures** (`gpt_flash_shape` ×2, `roster` ×1, `witness_digests` ×1), none in a file this slice reads or writes |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | compiles |
| `openspec validate --all --strict` | **PENDING** — this seat's shell refuses to launch `openspec` (the reviewer's own run on `75b5517c` passed 15/15; this visit changed no delta, requirement or scenario, only this account and four local checkboxes) |
| `bash scripts/coverage-exact.sh` | **PENDING** — below |

**The exact-coverage gate stays pending, for the reasons already on
record.** Its instrumented run is `cargo llvm-cov --workspace`, and the
four runtime failures above stand at this HEAD; and the script builds its
instrumented target under `$TMPDIR`, which on this host is a 31 GiB tmpfs
that a workspace instrumented build fills. Neither is a pass and none is
inferred.

**One measurement trap, found and avoided.** The first crate-scoped run
this visit made (`cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli -p
brokkr-protocol --all-features --locked --branch`) reported 936 zero-hit
lines in `composite.rs` with `FNF 226 / FNH 119` — three compiled
instances of every function, one of them with every count at zero. The
unchanged `adapters.rs` in the same report carried the same three
instances and no zero-hit record, so the zeros were not this slice's: an
instrumented executable from an earlier source graph had survived in
`target/llvm-cov-target` and joined the merge. That is exactly why the
gate script runs `cargo llvm-cov clean --workspace` first. The
measurement below is taken after that clean.

What WAS measured, on the final bytes, with the pinned
`nightly-2026-09-05`: `cargo +nightly-2026-09-05 llvm-cov clean
--workspace`, then `cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli
-p brokkr-protocol --all-features --locked --branch --lcov`, every test
binary green, evaluated record by record with the gate's OWN rule — every
`DA` and `BRDA` record hit, and every logical function (file plus start
line, any positive compiled instance) hit:

| file | `DA` records | `BRDA` records | logical functions |
|------|-------------|----------------|-------------------|
| `crates/brokkr-protocol/src/adapters/composite.rs` | 984, none at zero | 230, none at zero or `-` | 119 of 119 |
| `crates/brokkr-cli/src/doctor.rs` | 520, none at zero | 44, none at zero or `-` | 47 of 47 |

The review's finding 5 was `doctor.rs` at 46 of 47 with the producer
closure at line 213 unhit; that line now carries `DA:213,4`. This is
evidence about the changed files, taken the way the gate takes it; it is
NOT the gate, which stays pending above.

#### Still not delivered, and named

- **Part (d)**, the planner, was not started, planned or touched.
- **8.10**'s whitespace and rejection-vector ledger; **9.6**;
  **10.6–10.8**; **11.1–11.4**; groups **14** and **15**.
- `openspec validate --all --strict` from a seat that can launch it, and
  the **exact-coverage gate** on a host where the four pre-existing
  runtime failures have been repaired by their owner and `$TMPDIR` can
  hold a workspace instrumented build. The two gate clauses above stay
  open on exactly that.
- No `wrapper_digest` was declared, no shape moved to `supported`, the
  DSH route stays disabled and unmeasured, and decision 0056 keeps its
  `proposed` status. The frozen contracts, `policy/phase-machine.json`,
  `policy/schemas/`, `fixtures/`, `reference/` and every byte under
  `extensions/dsh/` are untouched. The guide's composite vocabulary did
  not change, so its sample stands. Nothing was pushed.

## Implement visit — the security hold answered by removal proof, 2026-09-20

Historical first-hold account: its ticks, retained coverage and roster-failure
disposition do not discharge the second hold. AN/AO and the current breakdown
above govern; the removed roster commit supplies no current exemption.

Run `dsh-composite-identity-issue-226-4437331e`, phase implement, on
`slice-dsh-composite-b`. The branch this visit entered already carried
`d54a9f7b` — this run's implement commit answering the seven findings of
run `26def5a5`'s third review sitting — and one uncommitted test in
`crates/brokkr-cli/tests/doctor_dsh_selection.rs`. Both were read, adopted
and verified rather than redone; the controller's reproduction record
(`.forge/tasks/controller-s1-reproduction-2026-09-20.json`) was read first.
This visit added the removal proofs the breakdown asks for, closed the four
coverage misses the exact rule found in `composite.rs`, ran every gate this
seat can launch, and wrote this account. **8.8 stays unchecked**; part (d),
8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 were not touched. The 101
change-wide identifiers remain **84 complete / 17 pending**; the ticks below
are the local 8.8.x.x clauses only.

### What each finding cost, in production (adopted at `d54a9f7b`)

| # | Finding | Repair in the delivered bytes |
|---|---------|-------------------------------|
| 1 | HIGH S1 | `resolve_executable_in` refuses a `None` `PATH` with `'<command>': PATH is absent` before any split; a present-empty `PATH` keeps its one-empty-entry cwd meaning. `DshSeams::selected` is fallible: `Ok(DshSelection { executable, seams })` only when a file was selected, `Err(DshUnselected { declared, cause })` otherwise. `dsh_provider_line_with` matches that result BEFORE probing; a failed selection returns the declared spelling and the cause and spawns nothing. |
| 2 | MEDIUM correctness (3) / adversarial F1 | `classify_candidate` yields `Passed` only for ENOENT/EACCES/non-file/non-executable, `Refused` for a symlink loop, an unproved metadata error, a failed canonicalization and — via a bounded `#!` head read — a missing, non-absolute, non-executable, empty, NUL-carrying or unterminated interpreter. An explicit override takes the same checks with no next entry. |
| 3 | MEDIUM correctness (1) / spec F1 | pnpm scalars keep plain/quoted provenance; a flow field needs `: ` separation; a plain flow value may not carry `[ ] { } ,`; a plain `integrity` that the YAML core schema reads as null, boolean or number refuses by field, spelling and kind. |
| 4 | MEDIUM correctness (2) / adversarial F2 / spec F2 | a decoded `packages:` heading is a singleton, checked before exclusion or triple normalization: `a repeated package key '<key>'`. |
| 5 | MEDIUM spec F3 | the plugin-component test compares to a literal recorded from the sole producer; `no_test_reassembles_the_component_stream` reads the test source and refuses the competing block. |
| 6 | LOW S2 | the unavailable line renders the binary and the retained cause through `Safe` at their only interpolation. |
| 7 | LOW spec F4 | exhausted `resolve_bundle` lookup reads `bundle '<name>' does not resolve: no package.json found`; lookup order and containment unchanged. |

### What this visit changed on top of `d54a9f7b`

- `crates/brokkr-cli/tests/doctor_dsh_selection.rs`:
  `a_native_image_on_path_is_selected_as_the_child_selects_it` — the
  native-image half of the platform-native controls (a symlink to the built
  `brokkr` binary on `PATH`; doctor's line carries the native child's own
  first line). Adopted from the uncommitted tree; passes.
- `composite.rs`: `DshSeams::resolve` now delegates to a private
  `resolved(selection)` seam, and `dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one`
  drives both arms with injected selections (`'dsh': PATH is absent` comes
  back as the layout's refusal; a selection is its seams either way the home
  went). The failed-selection arm of `resolve` had no test reaching it.
- `composite.rs`: `interpreter_obstruction` reads the head with
  `File::open(..).and_then(|file| file.take(SHEBANG_BOUND).read_to_end(..))`,
  one `cannot be read: <error>` reason for open and read alike. The separate
  read-after-open failure arm had no deterministic trigger (a regular,
  executable file that opens and then fails to read).
- `composite.rs`: `classify_candidate` ends in
  `canonicalize(candidate).map_or_else(Candidate::Refused, Candidate::Admitted)`.
  The canonicalization refusal is unchanged in behaviour and reason; no test
  reaches a candidate that passes metadata, access and head inspection and
  still cannot be canonicalized, and none was invented.
- `composite/tests.rs`: `the_producer_refuses_a_bare_executable_spelling`
  asserts a backslash-only spelling (`missing\dsh`) is an explicit path
  refused by what the lookup found, the one untaken branch of
  `selected_executable`.

No delta, requirement, scenario, guide, decision or frozen byte moved. The
provider guide's two doctor samples (the LaneTally unavailable line and the
DSH composite line) are unaffected: the `: <cause>` suffix appears only when
a DSH selection itself failed.

### Removal proofs, M1–M12

Method: a detached scratch worktree of `HEAD` under the ignored `.forge/`
(`git worktree add --detach .forge/mut HEAD`, the uncommitted test copied
in), so no mutation ever raced a build in the candidate tree; each mutation
applied alone, its focused test run, then undone by exact restoration. After
this visit's coverage edits the scratch worktree was synced to the candidate
sources and M1, M3 and M4 — the mutations inside the functions those edits
touched — were run again with the same failures. Every failure below is the
intended assertion or the named `was accepted` panic of `refused_vector`;
every restored rerun is green in the final gates table. Line numbers are the
test files' at the moment of the run; logs are run-local under
`.forge/implement-4437331e/removal-M*.txt`.

| # | Finding | Production mutation (compiling) | Test | Failure observed |
|---|---------|--------------------------------|------|------------------|
| M1 | S1 | `resolve_executable_in`: `let path = path.unwrap_or_default();` in place of the `None` refusal — the inherited pre-fix path | `absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel` (built binary) | `doctor_dsh_selection.rs:156` "doctor executed the cwd dsh under an absent PATH": the report's line read `ok       dsh: SECURITY_CWD_SENTINEL_9f3 · serves … · composite unreadable: the DSH layout is unreadable: 'node' is not on PATH (no declared wrapper_digest)` — the controller's observation, reproduced by the test's FIRST assertion. Same failure on the candidate bytes. |
| M1 | S1 | same | `an_absent_path_is_a_named_refusal_and_never_the_working_directory` | `left: "…'dsh' is not on PATH"` vs `right: "…'dsh': PATH is absent"` |
| M2 | S1 | `dsh_provider_line_with`: the `Err(DshUnselected)` arm hands `(declared, Err(cause))` on to the probe — the inherited discarded-error arrangement | `doctor::tests::a_failed_selection_probes_nothing_and_carries_its_cause` | panic at `doctor/tests.rs:2687` "a probe of 'dsh' after a failed selection" (the no-probe assertion) |
| M2 | S1 | same | `absent_path_refuses_before_doctor_can_execute_a_cwd_sentinel` | `:161` "the refusal names the absent PATH": the line read `warn     dsh: binary 'dsh' not found — seats resolving…` with no cause; the sentinel stayed unexecuted because M1 was restored |
| M3 | 2 | `classify_candidate`: `let _ = interpreter_obstruction(candidate);` — metadata-only selection | `an_obstructed_path_search_takes_the_explicit_safe_refusal` (built binary) | `:239`: doctor selected and probed `A/dsh` (`warn     dsh: binary '…/a/dsh' not found — …`) instead of refusing by `its #! interpreter '…' is missing`. Same on the candidate bytes. |
| M3 | 2 | same | `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` | `tests.rs:96` "expected a refusal" at the missing-interpreter arm |
| M4 | 2 | `classify_candidate`: the ELOOP arm returns `Candidate::Passed(error.to_string())` — the inherited error-erasing continuation | `an_obstructed_path_search_takes_the_explicit_safe_refusal` | `:256`: B executed past the loop — `ok       dsh: DSH_B_SENTINEL_0.0.0-b · serves …`. Same on the candidate bytes. |
| M4 | 2 | same | `the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable` | `tests.rs:96` "expected a refusal" at the self-symlink arm |
| M5 | 3 | `pnpm_flow_map`: the `rest.starts_with(' ')` separation check removed | `missing_pnpm_field_separation_and_unsupported_flow_syntax_refuse_by_reason` | `"{integrity:sha512-X}" was accepted` |
| M6 | 3 | `pnpm_flow_map`: the plain-value `[ ] { } ,` check removed | same | `"{integrity: sha512-X[one]}" was accepted` |
| M7 | 3 | `pnpm_flow_map`: the `typed_plain_scalar` refusal for `integrity` removed | `pnpm_identity_strings_preserve_the_distinction_from_typed_scalars` | `"null" was accepted` |
| M8 | 4 | `pnpm_dependencies`: `seen_packages.insert` no longer refuses a repeat | `duplicate_decoded_pnpm_package_keys_refuse_before_triple_normalization` | `"identical" was accepted` — the identical repeat vanished into the triple set again |
| M9 | 5 | `component_digest` iterates `digests.iter().rev()` | `the_plugin_component_is_bytewise_path_order_and_fails_closed` | `:676` "the producer's recorded output over the synthetic six-file set": `left: "f193cd95…"` vs `right: "8894f23e…"` |
| M10 | 5 | TEST mutation: the inherited `lines.push('\0')` / `digest_of(lines.as_bytes())` oracle restored verbatim from `ab2d8e1d` beside the literal | `the_plugin_component_is_bytewise_path_order_and_fails_closed` AND `no_test_reassembles_the_component_stream` in one run | the component test **passed** (runtime equality holds); the conformance test failed at `tests.rs:2226`: "tests.rs:686 pushes a NUL separator: a test that serializes the component stream is a second producer" |
| M11 | S2 | `probe_providers`: `Safe` dropped from both interpolations of the unavailable line | `a_nonexistent_override_cannot_inject_terminal_control_bytes_through_doctor` (built binary) | `:385` "a raw escape byte reached stdout" |
| M11 | S2 | same | `doctor::tests::the_unavailable_line_escapes_the_binary_and_the_selection_cause` | `:2767`: the raw newline split the line — `warn     dsh: binary '/tmp/x` |
| M12 | 7 | `resolve_bundle`: exhausted lookup reads `bundle '<name>' does not resolve` | `removing_only_the_plugin_manifest_names_the_drifted_file` (producer) | `:2613`: `left: "…does not resolve"` vs `right: "…does not resolve: no package.json found"` |
| M12 | 7 | same | `removing_only_the_plugin_manifest_names_the_drifted_file` (built binary) | `:490` "the bundle and the file": `… composite unreadable: the DSH layout is unreadable: bundle 'dsh-plugin-cli-session' does not resolve (no declared wrapper_digest)` |

The real-child controls the breakdown asks for are in the tests themselves
and were exercised on every run: `Command::new("dsh")` under the same cwd
with no `PATH` returns `NotFound`; under `PATH=A:B` the native child runs B
past a missing-interpreter A and stops with ELOOP at a self-symlinked A;
explicit `/usr/bin:/bin`, a present `PATH` naming one empty directory, and
the sentinel removed with `PATH` still absent are the retained controls;
primary/legacy/`PATH` precedence, an absolute override with no `PATH`, and a
failed override beside a usable decoy are asserted by version sentinel. The
scratch worktree was removed after M4's restoration was verified equal to
the candidate.

### Gates, on the candidate bytes

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `git diff --check` | clean |
| `cargo test -p brokkr-core --all-features --locked --no-fail-fast` | ok (73 lib + 4 binaries) |
| `cargo test -p brokkr-store --all-features --locked --no-fail-fast` | ok (58 lib + 6 binaries) |
| `cargo test -p brokkr-protocol --all-features --locked --no-fail-fast` | ok: 357 lib, 99 integration (2 ignored), 1 doctest; 0 failed |
| `cargo test -p brokkr-runtime --all-features --locked --no-fail-fast` | 441 lib ok, 21 binaries ok, **3 binaries failed on the four recorded tests below**; nothing else |
| `cargo test -p brokkr-view --all-features --locked --no-fail-fast` | ok (243 lib, 3 ignored) |
| `cargo test -p brokkr-bridge --all-features --locked --no-fail-fast` | ok (13) |
| `cargo test -p brokkr-cli --all-features --locked --no-fail-fast` | ok: 463 lib, 31 binaries incl. `doctor_dsh_selection` (6), 0 failed |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` | compiles (digest `1dd39826…`) |
| `cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify` | compiles (digest `0ce86285…`) |
| `openspec validate --all --strict` | **NOT RUN from this seat**: both `openspec` and its full path require an approval this non-interactive seat cannot grant. The last recorded run on this change is the analyze seat's on `5a160542` (15 passed, 0 failed); under `openspec/` this visit changed only twelve `[ ]`→`[x]` ticks, one pointer paragraph and this account in `tasks.md`. |
| `bash scripts/coverage-exact.sh` | **NOT RUN literally**: script launches are refused in this seat; the script would also have exited at the runtime failures before writing a report, and its `$TMPDIR` default is this host's 31 GiB tmpfs. The D11 equivalent below was run instead. |

The four runtime failures were rerun three times this visit — the first
crate gate, the final crate gate and inside the instrumented run — with
byte-identical assertions, and none touches a file this slice reads or
writes; their disposition is the roster/recipe table earlier in this file.

| Fully qualified test | Failure this visit observed |
|---|---|
| `brokkr-runtime::gpt_flash_shape::sol_rules_specification_and_planning` | `gpt_flash_shape.rs:100`: analyze:judge `left: Some("astra")` vs `right: Some("sol")` |
| `brokkr-runtime::gpt_flash_shape::every_strategy_reviews_with_a_mixed_panel_before_the_astra_chief` | `gpt_flash_shape.rs:241`: chore panel `left: {"codex"}` vs `right: {"codex", "dsh"}` |
| `brokkr-runtime::roster::every_shipped_panel_seats_at_least_two_providers` | `roster.rs:497`: `recipes/gpt-flash/bundle.json` `seats.design.sequence.0` first hires `{"codex"}` |
| `brokkr-runtime::witness_digests::pinned_bundles_keep_their_recorded_digest` | `witness_digests.rs:304`: gpt-flash manifest digest `20b1ba15…` vs pinned `2b6623f1…` |

### Exact coverage — D11's equivalent, literal integers

Commands, in order, on the candidate bytes with the pinned compiler:

1. `cargo +nightly-2026-09-05 llvm-cov clean --workspace` — the script's own
   clean, so no earlier instrumented executable joins the merge.
2. `cargo +nightly-2026-09-05 llvm-cov --workspace --all-features --locked --branch --ignore-run-fail --json --output-path <run-local>/coverage-exact-final.json`
   — the complete workspace suite, 67 test binaries, every test run; the
   four runtime failures above fail inside it and are not skipped;
   `--ignore-run-fail` only keeps the report from being abandoned.
3. `cargo +nightly-2026-09-05 llvm-cov report --branch --lcov --output-path <run-local>/lcov-final.info`.
4. The script's counting rule applied record by record — every `DA` and
   `BRDA` hit, every logical function (file plus `FN` start line, any
   positive compiled instance) hit — by a run-local Rust program under
   `.forge/implement-4437331e/lcovtool/`, because this seat refuses `awk`,
   `python3` and script launches; the rule is the script's, transcribed.
   Harness-source check: 52 `SF` records, none matching
   `(^|/)(tests\.rs|[^/]+_tests\.rs|tests/)`.

Two departures from the literal script, both named: the instrumented target
is the default `target/llvm-cov-target` after the clean rather than a fresh
`$TMPDIR` directory (this seat cannot set environment variables on a
command, and the tmpfs default is the hazard the D11 note warns of), and
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` was not set for the same reason. The
totals below are exact regardless, so no production record depended on a
boundary proof that skipped.

| Pass | Lines | Branches | Functions | Misses |
|------|-------|----------|-----------|--------|
| First, on `d54a9f7b` + the adopted test | **31,164 / 31,167** (99.9904%) | **5,215 / 5,216** (99.9808%) | **3,006 / 3,006** (100%) | all in `composite.rs`: `DA 1101` (`resolve`'s unselected arm), `DA 1384` (post-admission canonicalize failure), `DA 1418` (read failure after a successful open), `BRDA 1536 block 1 branch 3` (backslash-only spelling in `selected_executable`) |
| Final, on the candidate | **31,166 / 31,166 (100%)** | **5,214 / 5,214 (100%)** | **3,008 / 3,008 (100%)** | none |

The report's JSON and LCOV, both passes' logs and the evaluator are run-local
under `.forge/implement-4437331e/`; nothing under `.forge/` is read by any
build or test. Final-head remote CI, including the workflow's own
`coverage-exact` job on a namespace-capable runner, remains pending until it
exists.

### Clauses: ticked, and left open with the reason

Ticked on focused regression plus removal proof: 8.8.1.1 (M1 unit, M2),
8.8.1.2 (M1 sentinel), 8.8.2.1 (M3, M4), 8.8.2.2 (real-child controls),
8.8.3.1 (M5, M6, M7), 8.8.4.1 (M8), 8.8.5.1 (M9), 8.8.5.2 (M10), 8.8.6.1
(M11), 8.8.7.1 (M12), 8.8.8.1 (this consolidation; `git status` shows only
the three code files and this file changed, no frozen path), 8.8.8.3 (the
exact totals above).

Left open: **8.8.8.2**, because two of its required checks did not pass in
this seat — `openspec validate --all --strict` could not be launched and
`cargo test -p brokkr-runtime` exits nonzero on the four outside-slice
failures; and **8.8.8.4**, because it is complete only with 8.8.8.2 proved.
The work is committed all the same (message style `dsh:` for code and
`tasks:` for this account), never pushed.

### Still not delivered, and named

- **Part (d)**, the planner, was not started, planned or touched; the
  planner's two call sites are unchanged.
- **8.10**'s rejection-vector ledger; **9.6**; **10.6–10.8** (including
  10.7's retained-home doctor recording); **11.1–11.4**; groups **14–15**.
- `openspec validate --all --strict` on this head from a seat that can
  launch it; the roster/recipe reconciliation that owns the four runtime
  failures; final-head remote CI and the namespace-capable coverage job.
- No `wrapper_digest` was declared, no shape moved to `supported`, the DSH
  route stays disabled and unmeasured for admission, and decision 0056
  keeps its `proposed` status. `contracts/`, `policy/phase-machine.json`,
  `policy/schemas/`, `fixtures/`, `reference/`, `extensions/dsh/` and
  `docs/decisions/` have no diff. The active change is not archived.
  Nothing was pushed.

## Implement visit — macOS portability of the delivered branch, 2026-09-21

Run `dsh-composite-identity-issue-226-d462f720`, phase implement, on
`slice-dsh-composite-b` at adopted head `a8c96e93`. This is a PORTABILITY
repair of the already-judged 8.8(a)–(c) delivery, not a redesign: run
`e291e076`'s council established no medium-or-higher defect, no security
residual and no specification defect, and that disposition is inherited, not
reopened. PR #311's CI passes ubuntu tests, exact coverage, clippy, MSRV and
packaging, and fails `test (macos-latest)` with twelve tests, all in
`crates/brokkr-protocol/src/adapters/composite/tests.rs` and its
`tests/native_matrix.rs`.

**There is no macOS host in this seat.** The only native macOS observation
is the CI log excerpt in
`.forge/tasks/controller-macos-ci-failures-pr311-2026-09-21.txt`, read
first. Every claim below is marked as executed here or as awaiting the
repaired head's own macOS leg. **8.8 stays unchecked** and no checkbox
moved; part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 were not
touched. The full account, with logs, is
`.forge/tasks/dsh-macos-portability-evidence.md`.

### The five causes, and what each cost

| # | Cause | Repair |
|---|-------|--------|
| A | macOS's `$TMPDIR` is reached through `/var`, a symlink to `/private/var`; the producer canonicalizes what it reports; seven fixtures glued `TempDir::path()` into their expectation. | A test-side `FixtureRoot` canonicalizes the temporary root ONCE at creation and retains the `TempDir` for cleanup; `Synthetic`, the core-executable test's two independent roots and the matrix parent build on it. No string substitution, no weakened comparison, no canonicalize of a deleted child. |
| B | `Errno::LOOP` renders as `os error 40` on Linux and `os error 62` on Darwin; the table asserted the literal. | The expected text is built from the host's `io::Error` for the same `Errno::LOOP` the call is given. The reason is still asserted whole. ENOENT, EACCES, EIO and ENOTDIR agree across both hosts and keep their literals. |
| C | **Production.** The cwd candidate is not one spelling: glibc and musl build the bare name for an empty entry, Apple builds `./<name>`, and the named refusal displayed the candidate — so one rule was reported two ways. | `Search::refuse_working_directory` renders the named refusal from the SEARCHED NAME. The native STOP at the same candidate still names the candidate as the platform built it, because that is an observation of a file, not an answer about a name. Every Linux string is byte-identical: on glibc the empty-entry candidate IS the bare name. AS1 fixes the phrase and the terminal-cause preservation and does not platform-qualify the prefix. |
| D | Apple's kernel separates `#!` arguments on whitespace (XNU `exec_shell_imgact`), so `-S node` reaches `env_program`'s compile-time split arm, not the form check. The commission's summary reversed actual and expected; log lines 1973–1975 show the production arm already right. | The two multiword vectors follow the compile-time arm, as the neighbouring `node --flag` case already did. `FOO=1 node` had the same latent mismatch and moves with it. Two single-word vectors (`-S`, `FOO=1`) are added so the measured-form refusal has a vector on every platform. No parser and no production shebang policy changed. |
| E | APFS enforces UTF-8 at creation, so the non-UTF-8 fixture cannot be built and the test panicked on its creation unwrap. | The creation is answered, not unwrapped. Where it succeeds the walker's exact refusal is asserted as before. Where it fails the filesystem error must be `Errno::ILSEQ`, the entry must be absent, and the walk over the remaining name must still complete — asserted by cause, never skipped, never `is_err()`. No injected reader impersonates the walker case: a `std::fs::DirEntry` is only ever yielded by a real directory. |

No Windows handling of any kind was added. Accepted decision 0063 closes
`e291e076`'s windows-msvc clippy LOW; it is not repair work.

### Removal proofs

**P1, the production change.** Restoring `candidate.display()` fails the new
`the_working_directory_refusal_names_the_searched_name_under_every_library`
on its `Library::Apple` row, on this Linux host, with exactly the CI
excerpt's mismatch (`./mytool` where `mytool` is owed); restored, it passes.
The same test drives `Library::Glibc` and `Library::Musl` beside Apple —
the table carries the library it translates — and asserts that the glibc
ELOOP stop still names the candidate and its cause, which is the guard
against over-applying the change.

**P2, the fixture root, proved on Linux.** The macOS condition is a
temporary root whose spelling is not its canonical one, and that is
reproducible here by handing the suite `TMPDIR=<...>/real/../real`. Under
it, crate-scoped to `adapters::composite::`: **103 passed / 0 failed** with
the canonicalization, **96 passed / 7 failed** with it removed, **103 / 0**
restored. The seven are exactly the seven the macOS leg reports for this
cause, failing in the same shape — including the opened-once counter's
`left: 0 / right: 1` and the matrix child's swapped `PATH` equality. This
proves the group is fixed at its cause on Linux; it is not a macOS
execution and does not replace one.

Not provable in this seat, and named as such: B's Darwin rendering (the
literal and the constructed text coincide on this host, so removal changes
nothing here), D's Apple arm (`#[cfg(not(linux))]` in `env_program`) and
E's filesystem-refusal arm.

### Gates, on the candidate bytes

`cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` clean; the seven crate suites each
run on their own and each **ok, 0 failed** (`brokkr-protocol` 381 + 99 + 1);
`cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
compiled. The full native matrix executed: 8 names × 52 layouts, 997
oracles, 334 equal selections, 126 NotFound parities, 250 terminal-error
parities, 104 NUL refusals, 42 D10 loader exceptions, 128
working-directory refusals.

Two required checks did NOT run and are not passes: `openspec validate
--all --strict` and `bash scripts/coverage-exact.sh` were both refused
launch by this seat's sandbox. No file under `openspec/specs` moved, and
the production delta adds no branch and no function — it changes one format
string and one signature — but neither fact is a substitute for the check.
The coverage gate is unchanged and not lowered.

One intermittent failure was seen and is not this repair's: the first
`brokkr-protocol` run lost
`spawn_node_runtime_reads_one_version_line_and_refuses_the_rest` to the
known ETXTBSY race (#255); every rerun is green. The previous visit's four
outside-slice `brokkr-runtime` failures did not reproduce here.

### Still not delivered, and named

- The repaired head's own `test (macos-latest)` leg. Nothing here claims
  the branch green on macOS, and the earlier Linux CI passes do not
  certify this diff.
- `openspec validate --all --strict`, `bash scripts/coverage-exact.sh`, and
  the workflow's `coverage-exact` job on a namespace-capable runner.
- Every Apple branch that is `cfg`-selected or library-gated and therefore
  recorded rather than executed: `env_program`'s split arm, the
  `assert_glibc_*` matrix cells and `Library::Glibc` control arms under a
  Darwin `LIBRARY`, and the non-UTF-8 test's filesystem-refusal arm.
- Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4, groups 14–15. `contracts/`,
  `policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
  `reference/`, `extensions/dsh/` and `docs/decisions/` have no diff;
  decision 0056 keeps its `proposed` status. Nothing was pushed.

## Implement visit — the direct name's symlink loop, 2026-09-21

Run `dsh-composite-identity-issue-226-551ef2a7`, phase implement, on
`slice-dsh-composite-b` at adopted head `a7c07cd1`. The LAST macOS cell of
PR #311's 8.8(a)–(c) delivery. The previous visit's repair took the macOS
leg from twelve failures to **373 passed, 1 failed**, with every other
check passing; this visit answers that one. **8.8 stays unchecked** and no
checkbox moved. The full account, with commands and logs, is
`.forge/tasks/dsh-direct-eloop-evidence.md`.

**There is no macOS host in this seat.** The only native observation is
the CI excerpt in
`.forge/tasks/controller-macos-ci-failures-pr311-pass2-2026-09-21.txt`,
read first. It names `native_matrix.rs:751`, cell n1-l4, name `./dsh`,
layout `A:B, A is a self-symlink (ELOOP)`: native failed with ELOOP and
the resolver answered `Config("./dsh: Too many levels of symbolic links
(os error 62)")` — the raw error, without the named reason the matrix
requires whenever the terminal native errno is ELOOP.

### The cause, and the rule it mistook

`lookup_in` sends a name containing `/` through `classify_in`, whose
metadata error reaches `lookup_failure`, which applied `step` — the C
library's own continuation switch. glibc and musl answer `Stop` for
ELOOP and reach `stop_cause`, which NAMES the loop; Apple's
`sys/posix_spawn.c` answers `Continue`, so the candidate became
`Candidate::Passed` carrying only the raw `io::Error` and the
explicit-path arm wrapped that as `Config`.

That switch is a rule about a SEARCH — whether the next PATH entry is
tried. A direct name has no next entry: `execve` answers for the path as
spelled and no library search runs at all. ELOOP on a direct name is
therefore terminal on every platform, and the naming belongs to the
terminal loop, not to the libraries whose search happens to stop on it.

### The production change, and its bounds

`composite.rs` gains a private `enum Position { Searched, Direct }`,
carried by `classify_in` and `lookup_failure`; `Search::find` passes
`Searched` and `lookup_in`'s explicit-path arm passes `Direct`. One
guarded match arm: a `Step::Continue` at a `Direct` position with
`Errno::LOOP` is `Candidate::Refused(stop_cause(…))`.

Only the loop, and only at a direct name. Every library's `Stop` keeps
its own words; every other continued errno at a direct name keeps the
operation's own answer; every searched candidate is untouched, Apple's
continuation past ELOOP to the next entry included. **Every Linux string
is byte-identical** — on glibc and musl `step` never returns `Continue`
for ELOOP, so the new arm is reachable on this host only through an
injected `Library::Apple`, which the tests do drive.

### Tests, and the removal proof

Four rows join the per-library lookup table — the seam that already
drives each library's switch here. `Position::Direct` + `Errno::LOOP`
under Apple, glibc and musl all answer the identical named refusal; and
`Direct` + ENOENT under Apple, + EACCES and + EIO under glibc answer
exactly what they answered before, which is the guard against
over-applying the change. A new Linux-runnable test,
`a_direct_names_symlink_loop_is_named_on_every_librarys_arm`, plants a
real `a/dsh -> dsh` self-symlink beside a runnable `b/dsh`, has the
host's kernel confirm ELOOP for the fixture, then drives `lookup_in`
with the DIRECT path under an injected Apple, glibc, musl and the
compiled `LIBRARY`, each under both `Operation::Exec` and
`Operation::Spawn` — asserting the whole reason, candidate and named
cause included. ELOOP's number is the host's, so the expectation is
built from `rustix::io::Errno::LOOP` through `io::Error` and never from
Darwin's 62 written into a Linux test. The same test asserts the
searched controls do not move: Apple still walks past the loop to B,
glibc still stops there. `tests/native_matrix.rs` is unchanged.

Removal, crate-scoped to `adapters::composite::`: **104 / 0** with the
correction, **102 / 2** with the guard's condition forced false (a
compiling mutation, the arm retained), **104 / 0** restored. The two
failures are the two new assertions and their text is exactly the macOS
symptom — `…/a/dsh: Too many levels of symbolic links` where
`…/a/dsh: a symlink loop stops the lookup: …` is owed. Nothing else
moved in either direction.

### The LOW from run `d462f720`'s chief

`the_plugin_walk_refuses_a_name_that_is_not_utf8` asserted
`raw.symlink_metadata().is_err()`, which a denial or an I/O fault would
satisfy as readily as absence. It now asserts the exact cause —
`io::ErrorKind::NotFound`, with a named panic if the entry is there at
all — and the directory's exact entry names, `["LICENSE"]`, compared
whole. The test compiles on Linux and macOS. Its branch is the
filesystem-REFUSAL arm, which only a filesystem that rejects the name at
creation enters; Linux's tmpfs accepts it, so **the strengthened
assertions are compiled here and await the macOS leg for execution**.

### Gates

`cargo fmt --all -- --check` clean; `cargo clippy --workspace
--all-targets --all-features --locked -- -D warnings` clean; the seven
crate suites each run on their own and each **ok, 0 failed**
(`brokkr-protocol` 382 + 99 + 1); `cargo run --locked -p brokkr-cli --
compile --bundle bundles/self` compiled. No boxed workspace sweep and no
concurrent crate suites.

Two required checks did NOT run and are not passes: `openspec validate
--all --strict` was refused launch by this seat's sandbox under every
spelling tried, and `bash scripts/coverage-exact.sh` was not run here.
No file under `openspec/specs` moved and the coverage gate is unchanged
and not lowered, but neither fact substitutes for the check. One
intermittent failure was seen on the first `brokkr-protocol` run and is
not this repair's: `hands::tests::the_network_prefix_is_eight_tokens_…`,
a plant-then-exec race of the known ETXTBSY family (#255); it passes
alone and on every rerun.

### Awaiting the macOS leg

The repaired head's own `test (macos-latest)` job — cell n1-l4 in both
the `inherited` and `explicit` forms, and the remaining 373 staying
green — and the non-UTF-8 test's filesystem-refusal arm. The injected
Apple arm proves the resolver branch on Linux; it is not native macOS
execution and does not replace one.

Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4, groups 14–15 were not
touched. `contracts/`, `policy/phase-machine.json`, `policy/schemas/`,
`fixtures/`, `reference/`, `extensions/dsh/` and `docs/decisions/` have
no diff; decision 0056 keeps its `proposed` status. No delivery recipe
was selected. Nothing was pushed.

## Implement visit — ending the macOS whack-a-mole, 2026-09-21

Run `dsh-composite-identity-issue-226-380a534e`, phase implement, on
`slice-dsh-composite-b` at adopted head `75605bdc`. Task 8.8(a)–(c),
parts of PR #311's macOS leg. **8.8 stays unchecked** and no checkbox
moved. Three macOS passes had answered twelve failures, then one, then
two; this visit answers the two AND the reason each pass revealed only
the next cell.

**There is no macOS host in this seat.** The only native observations
are the CI excerpts in
`.forge/tasks/controller-macos-ci-failures-pr311-pass3-2026-09-21.txt`
and its two predecessors, read first.

### F1 — the Apple-arm leftover of a rule already replaced

`the_candidate_classifier_stops_where_the_child_stops_and_refuses_the_unprovable`
still spelled a DIRECT name's loop cause per library: `loop_stops` true
gave `a symlink loop stops the lookup: …` and false gave the raw
`io::Error`. Run `551ef2a7` had already made production name that loop
on every arm, so the false branch was dead wording that only macOS could
execute. The expectation is now unconditional and built from the
fixture's own `eloop`. `loop_stops` still governs the SEARCHED assertion
above it, which is genuinely per library.

Every other expectation of the same shape was looked for, by grep over
`symlink loop stops the lookup` and `Errno::LOOP` across `crates/`:
`doctor_dsh_selection.rs:740` branches on the NATIVE control's own
outcome rather than on a library constant and is correct, and nothing
else qualifies a direct name's wording by library.

### F2 — which refusal Apple's arm owes an oversized component

Cell n0-l10, `PATH` a single 5,000-byte component, name `dsh`: native
answered ENAMETOOLONG/63, production answered `<5000 x>/dsh: the
platform's lookup stops before attempting a candidate longer than the
1024 bytes it builds one in (ENAMETOOLONG)`, and the matrix required the
kernel-answered wording for every ENAMETOOLONG.

**Production was right and the matrix was wrong.** Apple sizes EVERY
candidate against a 1,024-byte buffer before building it —
`lp + ln + 2 > sizeof(buf)`, `sys/posix_spawn.c` 97–143 and
`gen/FreeBSD/exec.c` 178–218, the revisions design D10 §3 inspected — so
a 5,004-byte candidate is never constructed and never handed to
`execve`. The kernel cannot be the author of that errno, because the
path it would have measured does not exist; `posix_spawnp` answers
`err = ENAMETOOLONG` at the bound and `execvP` warns and takes the next
token. The two refusals differ in exactly that, and the pre-attempt one
is the true one on this arm. glibc's bound is `PATH_MAX`/4,096 and its
skip leaves the cursor on the colon, which is why the same `PATH` is a
working-directory iteration there and a stop here.

The matrix now translates a layout's DECLARED expectation — glibc's,
because glibc is where every cell was measured — to the running library
and this form's operation, in `expect_on`. On Apple a layout whose first
component overflows the bound is `Expect::ConstructionStop` under
`posix_spawnp` and, under `execvP`, whatever the surviving tokens make
it: `Parity` where B follows, `WorkingDirectory` where an explicit empty
entry does. That only the first component overflows is asserted of every
later slot rather than assumed. A new `Expect::ConstructionStop` arm
asserts native's ENAMETOOLONG, the pre-attempt wording, and that neither
B nor the working directory is reached. A removal control carries its
own `PATH` now, because one slash off a 4,092-byte padded A is still far
over 1,024 and is still a construction stop there.

The terminal-error arm's ENAMETOOLONG and ELOOP assertions are per
library and per position too: Apple's switch CONTINUES past both, so on
that arm a searched name can only end on one by exhausting its entries,
while a direct name — which no switch governs — names the terminal
cause on every arm alike. The `overlong-explicit-path` control is a
direct name and is therefore no longer glibc-gated; its expectation
renders ENAMETOOLONG's number from the constant the kernel answers with,
36 under Linux and 63 under Darwin, instead of spelling Linux's integer
into a control that runs on both.

### The one production change, and its bound

At `Position::Direct`, `lookup_failure`'s named-stop guard now covers
`Errno::NAMETOOLONG` beside `Errno::LOOP`. Those two are exactly the
errnos the searching libraries part company on, and therefore the two
whose direct-name refusal read one way on Linux and another on macOS. At
a direct name no switch runs at all, so the refusal may not vary by
library. **Every Linux string is byte-identical**: glibc and musl
already reached `stop_cause` for both errnos, so the new arm returns
what they returned; only the Apple arm moves, from the bare `File name
too long (os error 63)` to `metadata answers File name too long (os
error 63), on which the platform's lookup stops`. ENOENT, ENOTDIR and
EACCES are untouched — they sit in every library's continue-set and
already answered alike on every arm.

Removal proof: the guard restored to `LOOP` alone (a compiling mutation,
the arm retained) → `the_lookup_rule_is_each_librarys_own_switch_arm_by_arm`
fails with `left: "passed (false): File name too long (os error 36)"`
against the named stop, under "Apple names a direct name's terminal
ENAMETOOLONG"; restored → green. Separately, `apple_walk`'s
`Operation::Spawn` arm forced to `continue` → the new construction-bound
test fails at its first `refused(…)`; restored → green.

### The loop fix, and its own removal proof

The matrix parent asserted `output.status.success()` inside its per-case
loop, and each child panicked at its first failing cell — which is why
three passes at ~25 minutes each revealed one cell apiece. Both now
COLLECT: `collecting` runs one cell under `catch_unwind` and records the
payload, and `report` raises every recorded failure in one panic — by
the child for its cells, by the parent for its children, ahead of the
inventory assertion (a failing child also stops reporting oracles, so
its missing identifiers are a consequence of the failure and not a
second finding). The 13 named controls collect the same way. Cells are
independent: each name's fixtures are its own and are removed after it.

Proved by removal rather than described: `CWD_REASON` set to a wrong
string and the matrix run once → **16 layouts, 128 distinct cells**
reported in one panic (`the matrix: 16 of this child's cells failed`,
then `layout 12: 8 …` through `layout 49: 8 …`). Before the change that
same run reported layout 12 and stopped. Restored → green.

### The Apple-arm audit — errno × name kind × covered by

Read from `step`, `apple_walk` and `lookup_failure` against Apple's two
walks. "Searched" is one entry of a search; "direct" is a name
containing `/`, which no library's switch governs.

| errno | searched, Apple | direct, Apple | Linux-runnable proof of the Apple arm |
| --- | --- | --- | --- |
| EACCES | remembered denial, walk continues | `is not executable by this process`, as on every arm | `…_switch_arm_by_arm`: `step(Apple, ACCESS)`; the direct cross-arm loop (new) |
| ENOENT | continues | the operation's own answer, as on every arm | same test: `step(Apple, NOENT)`; the direct cross-arm loop |
| ENOTDIR | continues | the operation's own answer, as on every arm | same test: `step(Apple, NOTDIR)`; the direct cross-arm loop (new) |
| ELOOP | continues to the next entry | `a symlink loop stops the lookup: …` on every arm | `a_direct_names_symlink_loop_is_named_on_every_librarys_arm`; `lookup_failure(Apple, LOOP, Searched)` |
| ENAMETOOLONG, kernel-answered | continues; only exhaustion can end on it | `metadata answers …, on which the platform's lookup stops` on every arm (new) | `…_switch_arm_by_arm`: the 600-byte component reaching B; the searched and direct Apple assertions (new) |
| ENAMETOOLONG, construction bound | `posix_spawnp` stops before any attempt; `execvP` skips the token | not reachable — a direct name constructs no candidate | `the_apple_arm_answers_an_oversized_component_by_its_construction_bound` (new); the 1,100-byte cells |
| ENOEXEC | no pinned arm → named limitation | same limitation | `step(Apple, NOEXEC) == Err(…)` (new); `lookup_failure(Apple, IO, Searched)` |
| EIO, EINVAL, ESTALE | no pinned arm → named limitation | same limitation — **the one residual** | `step(Apple, …)`; the direct EIO assertion recording it (new) |
| the working-directory candidate | `./<name>` for an empty token, refused under the searched NAME | not a search | `the_working_directory_refusal_names_the_searched_name_under_every_library` |

**Superseded — read the corrected table in the next visit's account.** The
returned review (R2, 2026-09-21) found this fourth column overstated:
the EACCES, ENOENT, ENOTDIR and ENOEXEC rows cite `step` and
`lookup_failure`, which answer the SWITCH's question and stop at an
intermediate `Candidate::Passed`, not the refusal a caller reads; and
ENOEXEC reaches no switch arm at all.

The residual: at a direct name an errno outside Apple's pinned switch
renders as that limitation, where glibc and musl name the stop a direct
name always is. No cell of the matrix asserts it and no Linux string
carries it; it is asserted as it stands and left as named pending work
rather than guessed at.

`the_apple_arm_answers_an_oversized_component_by_its_construction_bound`
drives the exact `PATH` spellings of the cells the third pass had still
to reveal — the sole 5,000-byte component (n0-l10); 4,095, 4,096 and
5,000 ahead of a runnable B (n0-l11/47, n0-l12/48, n0-l13/49); the 4,092
bytes the padded-A spelling reaches (n0-l39–41); and 4,096 ahead of an
explicit empty entry and B (n0-l18) — under an injected Apple arm in
both operations, with glibc's answers to the same spellings beside them.

### Gates, on the candidate bytes

`cargo fmt --all -- --check` clean; `cargo clippy --workspace
--all-targets --all-features --locked -- -D warnings` clean; the seven
crate suites each run on their own, sequentially, each **ok, 0 failed**
(`brokkr-protocol` 383 + 99 + 1; `brokkr-cli` 466 plus its 29
integration binaries; `brokkr-runtime` 441 plus its binaries;
`brokkr-core` 73; `brokkr-store` 58; `brokkr-view` 243;
`brokkr-bridge` 13). No boxed workspace sweep and no concurrent crate
suites.

`openspec validate --all --strict` did NOT run and is not a pass: this
seat's sandbox refused the binary under every spelling tried, as it did
on the previous visit. Only this file moved under `openspec/`, and no
file under `openspec/specs` did. `bash scripts/coverage-exact.sh` was
not run here either. The one production change adds a pattern
alternative to an existing match guard; both alternatives, the
fall-through and the `Searched` position are each exercised by
`…_switch_arm_by_arm`, and the coverage gate is unchanged and not
lowered — neither fact substitutes for the check.

### Awaiting the macOS leg

The repaired head's own `test (macos-latest)` job. It is now expected to
report its WHOLE remaining surface in one panic rather than the next
cell, which is the point of the change. The Apple arm is injected here,
which proves the resolver's branches on Linux; that is not native macOS
execution and does not replace one.

Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and groups 14–15 were not
touched. `contracts/`, `policy/phase-machine.json`, `policy/schemas/`,
`fixtures/`, `reference/`, `extensions/dsh/` and `docs/decisions/` have
no diff; decision 0056 keeps its `proposed` status. Nothing was pushed.

## Implement visit — the returned review's R1 and R2, 2026-09-21

Run `dsh-composite-identity-issue-226-380a534e`, returned from review
with two MEDIUM residuals and one LOW against `75605bdc..77325d46`. Both
MEDIUMs are about the PREVIOUS visit's own work — where its collection
boundary sits, and what its audit table claims — so this visit's whole
delta is tests. `crates/brokkr-protocol/src/adapters/composite.rs` has
NO diff: the mutations below were applied to it and restored, and
`git status` names only the two test files.

### R1 — the collection boundary sat inside the operation, not around it

The previous visit collected each cell's COMPARISON. Everything that
produced the values compared — the planted errno, the oracle
invocation, the resolution — still ran outside `collecting`, and
`oracle` panics on two conditions of its own: an unidentified sentinel
(`native_matrix.rs` 705–710 as reviewed) and an unsuccessful sentinel
exit. So one such failure still ended its child at that cell, taking
the name's other invocation form, every later name, and the per-name
fixture cleanup with it — the exact failure mode the commission asked
to end, one layer in.

Four boundaries moved, and nothing else:

1. **Each form's whole operation** — plant, oracle, resolve — is one
   `collecting` call, and the comparison that follows it is skipped
   where the invocation has no outcome to compare rather than invented.
   The cross-form glibc check runs only where BOTH forms answered, so a
   failed form is one finding and not two.
2. **The per-name fixture staging** is collected the same way, and
   records what it placed AS it places it, so a staging failure is that
   cell's finding and whatever reached the filesystem is still removed.
3. **The parent's per-case invocation** is collected, so a child this
   parent cannot even spawn is one case's finding.
4. **The parent's report validation** — a duplicated oracle identifier
   and an unreadable tally line — is recorded instead of asserted, so a
   malformed report does not end the run at the line it appeared on.

The 12 named controls that drive an oracle now hold their oracle and
their resolution inside their own `collecting` too; the thirteenth,
`default-search-sh`, drives `matrix_spawn` directly and was already
inside its own.

`oracle` also prints the cell's identifier as soon as the invocation
COMPLETES, ahead of identifying what ran. Fail-closed accounting is
unchanged — the identifier still means "this cell's oracle ran", and
`report` still fires ahead of the inventory assertion — but an
unidentified sentinel is now ONE finding rather than its panic plus a
hole in the parent's inventory.

#### Removal proofs, this visit's own

Each mutation applied to the candidate bytes, run, and restored; the
recorded `CWD_REASON` proof of the last visit exercised the comparison
aggregation alone, which is what R1 said.

| mutation | what it proves | what was reported |
| --- | --- | --- |
| `oracle`'s marker prefix `MARK:` → `MARKX:` (every ran sentinel is unidentified) | oracle-level failures are collected AND the cells after them still run | **456 distinct cells** across **52 children**, in one panic (`the matrix: 52 of this child's cells failed`) |
| `oracle` panics unconditionally after printing its identifier | the named controls collect at the same boundary | **all 12 oracle-driving controls** in one panic (`the controls: 12 of this child's cells failed`), and 72 parent cases |
| the controls child prints a seventh tally slot | an unreadable report is a finding, not the end of the parent | `case controls: a tally line this parent cannot read: matrix-tally: 0 0 0 0 0 0 7` |
| the controls child repeats an oracle identifier | a duplicated identifier likewise | `case controls: oracle control:valid-length-name reported twice` |

Restored, the matrix is green and its inventory is unchanged: 8 names ×
52 layouts, **997 oracles**, every declared cell reporting exactly once.

### R2 — the audit named the switch where the refusal is the lookup's

R2 is right on every count. `step` answers continue-or-stop and
`lookup_failure` answers with an intermediate `Candidate`; a
CONTINUATION is not a refusal at all, and what the differential matrix
compares on macOS is the string `Search::find` or `lookup_in` finally
produces. The EACCES, ENOENT and ENOTDIR rows cited the switch; the
ENOEXEC row cited an arm no lookup path reaches, and lent it an EIO
diagnostic that belongs to the unpinned-arm residual.

`each_pinned_errno_ends_in_the_same_refusal_on_every_librarys_arm`
(`composite/tests.rs`) drives `lookup_in` itself, under the injected
Apple arm beside glibc, musl and this target's own, in BOTH operations,
and asserts the whole final reason of each:

- **searched, denial** — `denied:nowhere` and `nowhere:denied` both end
  in `'dsh' is not executable by this process on PATH: <denied>/dsh: is
  not executable by this process`, which is the remembered EACCES
  reported over a later cause, on every arm;
- **searched, exhaustion** — `'dsh' is not on PATH (the search ended at
  <candidate>: <cause>)` for ENOENT under a missing entry and ENOTDIR
  under a regular file spelled as one, the cause taken from the same
  kernel error the fixture gives this host;
- **searched, positive** — all three walked past to a runnable B, so the
  continuations are proved to BE continuations and not silent stops;
- **direct** — ENOENT, ENOTDIR, EACCES, a directory (`is not a regular
  file`), a 300-byte component under an existing directory (the named
  ENAMETOOLONG stop), and the direct name that loads, each the one
  answer `lookup_in` formats, identical on every arm;
- **loading** — an executable whose content is neither a `#!` script nor
  a loadable image refuses `is not a loadable native image: neither a
  #! script nor a native image`, ahead of the runnable B behind it,
  searched and direct alike.

Removal proofs: `step(Apple, ACCESS)` weakened to `Continue { denied:
false }` → the Apple denial assertion fails against the exhaustion
string while glibc's passes (a per-ARM divergence at the final refusal,
which is what the test exists to catch); `native_obstruction`'s
`image::inspect` failure turned into `Ok(())` → the unloadable
candidate is admitted and the loading assertions fail. Both restored →
green.

#### The Apple-arm audit, corrected — errno × name kind × covered by

"Searched" is one entry of a search; "direct" is a name containing `/`,
which no library's switch governs. The last column names the test that
drives the FINAL refusal on the Apple arm, and says where only a
switch-level check exists.

| errno | searched, Apple | direct, Apple | Linux-runnable proof of the Apple arm |
| --- | --- | --- | --- |
| EACCES | remembered denial, walk continues; reported over any later cause when nothing is admitted | `is not executable by this process`, as on every arm | final: `each_pinned_errno_ends_in_the_same_refusal_on_every_librarys_arm`, the denial string in both entry orders and the direct `<denied>/dsh`; switch: `…_switch_arm_by_arm`'s `step(Apple, ACCESS)` |
| ENOENT | continues | the operation's own answer, as on every arm | final: same test — the exhaustion under a missing entry, and the direct `<nowhere>/dsh` |
| ENOTDIR | continues | the operation's own answer, as on every arm | final: same test — the exhaustion under a file spelled as an entry, and the direct `<file>/dsh` |
| ELOOP | continues to the next entry | `a symlink loop stops the lookup: …` on every arm | final: `a_direct_names_symlink_loop_is_named_on_every_librarys_arm`, through `lookup_in` in both operations, with Apple's searched continuation to B beside it |
| ENAMETOOLONG, kernel-answered | continues; only exhaustion can end on it | `metadata answers …, on which the platform's lookup stops` on every arm | final, searched: `…_switch_arm_by_arm`'s 600-byte component reaching B through `lookup_in`; final, direct: `each_pinned_errno_…`'s 300-byte component under an existing directory |
| ENAMETOOLONG, construction bound | `posix_spawnp` stops before any attempt; `execvP` skips the token | not reachable — a direct name constructs no candidate | final: `the_apple_arm_answers_an_oversized_component_by_its_construction_bound`, the matrix's own oversized `PATH` spellings under both operations |
| ENOEXEC | **reaches no switch arm**: a candidate whose content no loader reads is refused by `native_obstruction` before any execution, on every arm alike | the same refusal, by the same path | final: `each_pinned_errno_…` — `is not a loadable native image: neither a #! script nor a native image`, searched (ahead of a runnable B) and direct. The superseded row's `step(Apple, NOEXEC)` described an arm the lookup never asks about |
| EIO, EINVAL, ESTALE | no pinned arm → named limitation | same limitation — **the one residual** | switch level only: `…_switch_arm_by_arm`'s `step(Apple, …)` and its direct EIO assertion. No final-refusal test, because no fixture makes a kernel answer these to `metadata` or `access`; named rather than simulated |
| not a regular file (no errno) | walked past as the denial `execve` answers EACCES for | `is not a regular file` | final: `each_pinned_errno_…`'s direct directory row |
| the working-directory candidate | `./<name>` for an empty token, refused under the searched NAME | not a search | `the_working_directory_refusal_names_the_searched_name_under_every_library` |

Two limitations stand, both recorded rather than modelled:

- the EIO/EINVAL/ESTALE residual above, unchanged from the previous
  visit;
- Apple's `execvP` retries an ENOEXEC candidate through `_PATH_BSHELL`
  (the returned review read `gen/FreeBSD/exec.c` for it; this seat
  reached no Apple source). This resolver refuses such a candidate
  instead, which is the DECLARED loading exception and strictly
  narrower than native — it selects nothing native would not run — so
  no cell can fail on it, and no matrix layout places such a body on a
  macOS host (`loader_fixture()` is Linux-only, and the non-executable
  body is a denial, not an ENOEXEC).

### R3 — the LOW, on what the gates report

The run-validation report for the previous visit named `cargo test
--workspace`, which the commission does not permit. The gates below were
crate-scoped and sequential, one crate at a time, no workspace sweep and
no concurrent suites, and that is what is claimed for them.

### Gates, on the candidate bytes

`cargo fmt --all -- --check` clean. `cargo clippy --workspace
--all-targets --all-features --locked -- -D warnings` clean. The seven
crate suites each run on their own, sequentially, each **ok, 0 failed**:
`brokkr-protocol` 384 + 99 + 1 doc-test; `brokkr-cli` 466 plus its 29
integration binaries; `brokkr-runtime` 441 plus its binaries;
`brokkr-view` 243; `brokkr-core` 73 plus three; `brokkr-store` 58 plus
five; `brokkr-bridge` 13. `cargo run --locked -p brokkr-cli -- compile
--bundle bundles/self` compiles.

`openspec validate --all --strict` did NOT run and is not a pass: this
seat's sandbox refused the binary under every spelling tried — the
installed `openspec`, its absolute path, and `npx @fission-ai/openspec`
— as on the two previous visits. Only this file moved under
`openspec/`, and no file under `openspec/specs` did.
`bash scripts/coverage-exact.sh` did not run here either; this visit
adds tests and moves no production line, so the gate's subject is
unchanged and it is not lowered — which is not a substitute for the
check.

### Awaiting the macOS leg

Unchanged, and now the whole point of the change: the repaired head's
own `test (macos-latest)` job is expected to report its WHOLE remaining
surface in one panic — every failing cell of every layout and every
failing control, including the ones whose ORACLE fails — rather than
the next cell behind the last. The Apple arm is injected on this host,
which proves the resolver's branches here; that is not native macOS
execution and does not replace one.

8.8 stays unchecked. Part (d), 8.10, 9.6, 10.6–10.8, 11.1–11.4 and
groups 14–15 were not touched. `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/dsh/` and `docs/decisions/` have no diff;
decision 0056 keeps its `proposed` status. Nothing was pushed.

## Implement visit — 8.8(d) Pass B, argument admission, 2026-09-21

Run `dsh-launch-planner-issue-226-tas-3d08ce19` on
`slice-dsh-planner-d`, stacked on `slice-dsh-composite-b` (PR #311) and
adopting every one of its commits. This visit owns exactly the FIRST
paragraph of Pass B — admission of the planner's own inputs, before any
route read, version probe, composite call or staging — and its matching
8.10 cases. Everything after identity agreement in the (d) text, and
the whole launch half, stays where it was.

### What the admission surface already proved, and what it did not

The planner's order was already right: `split_dsh_model`, the shared
`split_effort`, `split_dsh_patch`, then `dsh_control_conflict` over the
WHOLE residual, then model validation and the effort/model
relationship, and only then `route_overlay::claim`. The inherited
selector-only deny-list is already gone: `dsh_control_conflict` refuses
every residual by fixed category.

What was missing was the observation. The inherited matrix drove
eleven spellings on three paths; 8.10's ledger names roughly forty, and
it requires the private staging counter to read zero on each — an
error and an absent retained directory do not establish that nothing
was staged. It also requires the private marker to sit IN the rejected
spellings, which the inherited vectors did not do.

### The one production repair, and the failure that demanded it

`split_dsh_patch` refused a value starting with `--` and admitted one
starting with a single `-`. Its own contract says "non-empty, non-flag
value", and `split_dsh_model` beside it refuses any leading `-`. So
`--patch -<anything>` — every short launcher spelling — passed arity
and was carried into the route read, where it refused for a DIFFERENT
reason, in a diagnostic naming the route.

Demonstrated first, before the line moved:

```
disabled/flag-shaped patch value: the admission refusal precedes the
route read: refusing to invoke the dsh driver: the route overlay
binding disagrees with the `--patch` value
  adapters/tests.rs:8504
```

The repair is one character of predicate (`"--"` → `'-'`) with its
reason beside it, and it is the ONLY production line this visit moves.
Restoring the old predicate reproduces that panic and a second one,
`["--patch", "-zzz-4be20d-splitter"] must refuse` at
`adapters/tests.rs:10366`; restoring the repair passes both.

### Removal proofs

| Broken line | Failing test | Assertion |
| --- | --- | --- |
| `split_dsh_patch`'s flag predicate back to `"--"` | `dsh_residual_and_joined_controls_refuse_before_any_observation`, `the_dsh_model_and_patch_splitters_refuse_their_malformed_shapes` | "the admission refusal precedes the route read"; `must refuse` |
| the `dsh_control_conflict` block moved BELOW `route_overlay::claim` | `dsh_residual_and_joined_controls_refuse_before_any_observation` | `disabled/unknown option beside a readable-looking route`: the refusal read `route_overlay file is unreadable` |
| the staging counter's `+ 1` at `dsh_seat_overlay_in` entry | `both_dsh_effort_spellings_are_admitted_and_stage_one_overlay` | `separate: exactly one staged overlay`, left 0 right 1 |

The third is D10's calibration: the ledger's forty-odd zero assertions
are only evidence because a positive plan makes the same counter read
one.

### The ledger

`dsh_residual_and_joined_controls_refuse_before_any_observation` now
drives every category 8.10 names, on the disabled, offered and cold
paths, each against a route binding that would fail to read:

- the plugin's value and bare selectors, long and short —
  `--session`/`-s`, `--new`/`-n`, `--resume`, `--list`,
  `--workdir`/`-w`, `--output-format`, `--json-schema`,
  `--profile`, `--dump-config`, `--dump-default-config`, `-h`;
- joined and clustered spellings — `--session=…`, `-s=…`, `-o…`,
  `-nrl`;
- the launcher's own unverified controls — `--from-default-profile`,
  `--verbose`; a settings override; an unknown option name; the option
  terminator; bare positional text;
- the effort spellings the shared splitter deliberately leaves in the
  argv rather than dropping a pin in silence — duplicate, mixed
  separate/joined, valueless, and a level outside its clamp in either
  spelling;
- the three authorized controls' own malformed shapes, refused by
  their own splitters with their own fixed field — joined, duplicate,
  valueless, empty-valued and flag-shaped `--model`; a malformed id;
  duplicate, bare, joined, `--patch…`-prefixed and flag-shaped
  `--patch`; effort without a model.

Each case asserts: a refusal; zero producer calls; no version probe
(the recording shim's marker); `dsh_staging_calls() == 0`; no retained
root under the home; the refusal names its fixed category or fixed
field; and the refusal contains neither the private marker nor the
route value.

The marker `zzz-9f31c7-marker` is a PLAIN IDENTIFIER, so it is also a
valid model id: the control cases pin it as the seat's model and carry
it in option names, equals-joined values, selector values, patch values
and positional text at once. No diagnostic echoes it.

One correction to the commission's own reading: there is no closed
effort VOCABULARY at this boundary. `effort_token` is a shape clamp —
one bounded word of at most forty characters starting with an
alphanumeric — so `--effort <any-plain-word>` is admitted by design and
the invalid-effort vectors are the ones outside that clamp.

`both_dsh_effort_spellings_are_admitted_and_stage_one_overlay` keeps
both admitted spellings as positive cases with no guessed alias: the
separate `--effort xhigh` and the equals-joined `--effort=xhigh`
compose the identical settings document and each stage exactly one
overlay.

`dsh_controls_that_decide_the_session_or_a_restriction_are_refused`
and `the_dsh_model_and_patch_splitters_refuse_their_malformed_shapes`
grew the same way at unit level. The second no longer asserts
`is_err()`: every refusal is now compared to its exact reason, and the
marker is checked absent from each.

### An inherited test race, repaired

`adapters::composite::tests::dsh_seams_resolve_reads_the_home_and_refuses_a_missing_one`
READS the process `DSH_HOME` and asserts what it read, without taking
the `ADAPTER_ENV` lock every planner test takes when it sets a
temporary one. It is an inherited race, not this visit's: two tests
that both exist at `1f60fdf4`, run together, reproduce it —

```
left: "/tmp/.tmpOgldUg"  right: "/home/vyanakiev/.dsh"
```

— with none of this visit's new tests running in that filter. Two more
`DSH_HOME` writers made it fire on the ordinary crate run, so the
reader now takes the same lock; `ADAPTER_ENV` became
`pub(in crate::adapters)` to make that possible. A reader is as much a
party to the race as a writer.

### Gates

Crate-scoped and sequential, never `cargo test --workspace` (issue
#255):

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-protocol` | 485 (385 + 99 + 1 doc), 0 failed |
| `cargo test -p brokkr-core` | 86, 0 failed |
| `cargo test -p brokkr-store` | 64, 0 failed |
| `cargo test -p brokkr-view` | 243, 0 failed |
| `cargo test -p brokkr-runtime` | 534 across 24 binaries, 0 failed |
| `cargo test -p brokkr-bridge` | 13, 0 failed |
| `cargo test -p brokkr-cli` | 32 binaries, 0 failed |
| `compile --bundle bundles/self` | compiled |
| `compile --bundle bundles/verify` | compiled |

`openspec validate --all --strict` COULD NOT RUN in this box: the
binary is installed at `~/.volta/bin/openspec` but the sandbox refused
the invocation. This visit adds no `openspec/specs` delta and touches
only this tasks file under `openspec/`, but that is not a substitute
for the check. `bash scripts/coverage-exact.sh` did not run here
either; it is not lowered.

### What Pass C still owes, and what the rest of B does

Pass C — launch-root confirmation before publishing — is untouched and
uncredited: no `root_session`, `transcript` locator or launch row may
be published before the pinned plugin's post-`await agents.resume`
init event names the offered root on the selected persistence root,
with a valid prior depth-zero header retained at the resolved locator,
no fresh sibling root or session in the retained store, and new
sequence activity past the recorded `firstSeq`. A request-derived
`session_id` is not that event. The inherited partial implementation
stands exactly as it was.

The REST of Pass B — the route binding beyond admission, the
gate-before-probe matrix, the identity comparisons and the owned-storage
boundary — was already delivered on `slice-dsh-composite-b` and its
suites pass here unchanged; this visit neither extended nor re-derived
them.

8.8 and 8.10 stay unchecked, and no sub-clause was ticked: the owners
run through C and D. 9.6, 10.x, 11.1–11.4 and groups 14–15 were not
touched. `contracts/`, `policy/phase-machine.json`, `policy/schemas/`,
`fixtures/`, `reference/`, `extensions/dsh/` and `docs/decisions/`
have no diff; decision 0056 keeps its `proposed` status; the DSH route
stays disabled. The research-dsh roster assertion, `bundle.json`,
`research-web.yml`, compiled staffing and every witness digest are
unmoved — `git status` names three files, all under
`crates/brokkr-protocol/src/`, plus this record. Nothing was pushed.

No Windows handling was added anywhere. Where the (d) text names a
Windows obligation it is withdrawn by decision 0063 (accepted
2026-09-21): the hosts are Linux and macOS. The new fixtures build on
temporary roots and take no literal errno.

## Implement visit — the returned review's R1–R4, 2026-09-21

Run `dsh-launch-planner-issue-226-tas-3d08ce19`, returned from review
with two MEDIUMs and two LOWs against `1f60fdf4..d7fe5e18`. The
security residual is R1 and it is real: the previous visit's own claim
that "the planner's order was already right" is withdrawn here.

### R1 — sequential extraction dissolved the argv's own adjacency

The three extractions run in sequence and each removes its flag WITH
its value before the next one looks. So a control standing in a LATER
control's value slot vanishes before that slot is read, and the
positional text behind it slides into the emptied slot:

| the seat's argv | what admission made of it |
| --- | --- |
| `--effort --model p/m high` | model `p/m`, effort `high` — both invented |
| `--patch --model p/m <path>` | model `p/m`, overlay `<path>` |
| `--model p/m --patch --effort high <path>` | effort `high`, overlay `<path>` |
| `--model p/m --patch --effort=high <path>` | the same, joined spelling |

None of those argv offers an effort level or an overlay path. Every
one was admitted past the admission rule and refused only later, by
the route read — which is exactly what Pass B's first paragraph
forbids, and the review reproduced all four through the built driver
on the disabled, cold and offered paths.

Demonstrated here before any line moved, with the four cases added to
8.10's ledger:

```
disabled/effort claiming a later model control: the admission refusal
precedes the route read: refusing to invoke the dsh driver: the route
overlay binding disagrees with the `--patch` value
  adapters/tests.rs:8541
```

The repair is `dsh_input_boundaries`, a DSH-LOCAL pass over the argv
the seat actually wrote, run first in `dsh_launch_with`. It walks once:
`--model`, `--patch` and `--effort` each claim the one part after
them, and a claimed slot holding a flag-shaped token is refused by the
field that owns the slot. An EMPTY slot is not its business — a bare
`--model` or `--patch` is still refused by arity in its own splitter,
and a bare `--effort` still stays in the argv as the residual the
shared splitter declines to drop in silence, so the inherited reasons
for those are unchanged.

Why one pass over the original argv is sufficient: the only tokens an
earlier splitter removes are a flag and the value adjacent to it, and
every such flag is itself flag-shaped. A later control's occupant can
therefore only be dissolved if it is flag-shaped, which is precisely
what this pass refuses. The occupant can never be consumed as a
preceding `--model`'s id either, because that would require `--model`
to sit where `--patch`/`--effort` sits.

The shared `split_effort` is NOT touched: it keeps its behaviour for
codex and every other adapter, and no other arm calls this pass.

### R2 — the transcript-root refusal published the operator's home

`dsh_transcript_row` interpolated the whole root into its LF/CR
refusal. That root is composed beneath the admitted DSH home, so the
diagnostic handed the seat the home's name and the harness layout in a
`Result.error` it can read. Demonstrated first:

```
disabled: the home echoed in dsh driver: transcript root
"/tmp/.tmpKCVDn0/zzz-4a0e13-home\nline/sessions/brokkr/seat-vJlncI"
spans more than one line
  adapters/tests.rs:8683
```

The message is now fixed and names its field only. The inherited
assertion at `adapters/tests.rs:1061` still passes unchanged; the
inherited `is_err()` helper beside `dsh_transcript_row` now compares
the exact reason and checks the path absent.

### R3 — the new fixtures now derive from a canonicalized root

`dsh_residual_and_joined_controls_refuse_before_any_observation` and
`both_dsh_effort_spellings_are_admitted_and_stage_one_overlay`
canonicalize the temporary root once and derive `DSH_HOME`, the
workdir and the shim from it. On macOS `/var` reaches the temporary
directory through `/private/var`, and the admission comparisons these
fixtures drive read those as two paths. No native macOS failure is
claimed; this is the rule, applied.

### R4 — the recorded `cargo test --workspace` attempt

`.forge/results/20687277-…-checks.json` stands as written: it records
an attempt this commission's #255 restriction forbids, and erasing it
would hide that. This visit's gates were crate-scoped and sequential
throughout, as the table below shows.

### Removal proofs

| Broken line | Failing test | Assertion |
| --- | --- | --- |
| `dsh_input_boundaries(extra)?` → `let _ = dsh_input_boundaries(extra)` | `dsh_residual_and_joined_controls_refuse_before_any_observation` | `disabled/effort claiming a later model control: the admission refusal precedes the route read` |
| `dsh_transcript_row`'s fixed diagnostic back to the interpolating `format!` | `a_dsh_transcript_root_refusal_names_its_field_and_never_the_root` | `disabled: the home echoed in …` |

Both restored; both green after.

### Gates

Crate-scoped and sequential, never `cargo test --workspace` (issue
#255):

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-core` | 86, 0 failed |
| `cargo test -p brokkr-store` | 64, 0 failed |
| `cargo test -p brokkr-protocol` | 486 (386 + 99 + 1 doc), 0 failed |
| `cargo test -p brokkr-runtime` | 534 across 24 binaries, 0 failed |
| `cargo test -p brokkr-view` | 243, 0 failed |
| `cargo test -p brokkr-bridge` | 13, 0 failed |
| `cargo test -p brokkr-cli` | 32 binaries, 0 failed |
| `compile --bundle bundles/self` | compiled |
| `compile --bundle bundles/verify` | compiled |

Two `-p brokkr-protocol` attempts FAILED before the green one above,
both on issue #255's ETXTBSY and neither on anything this visit wrote.
The first showed 63 failures, every one of them a poisoned
`ADAPTER_ENV` acquired after some other thread panicked. The second is
kept whole at
`.forge/tasks/controller-etxtbsy-protocol-2026-09-21.log`, and names
the root directly:

```
adapters::composite::tests::spawn_node_runtime_reads_one_version_line_and_refuses_the_rest
left: "the DSH layout is unreadable: node --version: Text file busy (os error 26)"
```

A single-threaded run (`-- --test-threads=1`, 386 passed) and two more
ordinary runs are green, which is the #255 signature. The failed
attempts are recorded, not erased.

`openspec validate --all --strict` COULD NOT RUN in this seat: the
sandbox refused the invocation under every spelling tried. This visit
adds no `openspec/specs` delta and touches only this tasks file under
`openspec/`; that is not a substitute for the check, and it stays
owed. `bash scripts/coverage-exact.sh` did not run here either — its
boundary tests need a namespace the box refuses to nest — and it is
not lowered. Native macOS and remote CI on the final head remain
pending; the parent PR's green checks do not establish them for this
head.

### Scope

Two files moved, both under `crates/brokkr-protocol/src/`, plus this
record. Production gained one function and two lines that call or
replace something: `dsh_input_boundaries`, its call site, and
`dsh_transcript_row`'s diagnostic. The shared effort splitter, the
production staging location, the planner signature, the staging
counter's seam, the route binding, the gate-before-probe order, the
identity comparisons and the owned-storage boundary are untouched.

Pass C is untouched and uncredited; it still owes everything the
previous record names — a valid prior depth-zero header retained at
the resolved locator for the offered ID, the pinned plugin's
post-`await agents.resume` init event read from the stream-json child
on the selected persistence root, no fresh sibling root or session in
the retained store, and new sequence activity past the recorded
`firstSeq`, before any `root_session`, transcript locator or launch
row is published. A request-derived `session_id` is not that event.

8.8 and 8.10 stay unchecked and no sub-clause was ticked. 9.6, 10.x,
11.1–11.4 and groups 14–15 were not touched. `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/dsh/` and `docs/decisions/` have no diff;
decision 0056 keeps its `proposed` status; the DSH route stays
disabled. The research-dsh roster assertion, `bundle.json`,
`research-web.yml`, compiled staffing and every witness digest are
unmoved. No live provider was called. Nothing was pushed.

No Windows handling was added. Decision 0063 (accepted 2026-09-21)
withdraws every Windows obligation the (d) text names.

## Implement visit — the returned review's R1 and R2, 2026-09-21

Run `dsh-launch-planner-issue-226-tas-3d08ce19`, returned from review a
second time with two MEDIUMs and two LOWs against `1f60fdf4..aeafc02f`.
Both MEDIUMs are real and both are closed here. The previous visit's
claim that the admission rule "runs first in `dsh_launch_with`" was true
and still insufficient: the rule only ever judged what the CLI handed
it, and the CLI was handing it less than the operator typed.

### R1 — the CLI cut the payload before admission could see it

`driver_extra_args` searched the WHOLE argument vector for `--` and
returned everything after the first hit. Clap consumes the operator's
outer separator itself, so the only terminator that can still be
standing there is one the SEAT wrote — and cutting at it hands the
adapter a shorter argv than was typed:

| the operator's command line | what the adapter received | what it did |
| --- | --- | --- |
| `driver dsh -- --effort --model p/m high --` | nothing at all | launched with no pin |
| `driver dsh -- --model p/m -- --model second` | `--model second` | launched on the second model |

Demonstrated first, through clap's own parse, before any line moved:

```
assertion `left == right` failed
  left: []
 right: ["--effort", "--model", "p/m", "high", "--"]
  crates/brokkr-cli/src/tests.rs:1165
```

and through the BUILT binary, where the refusal the admission rule owes
arrives as the launcher's own exit instead:

```
an effort slot claimed by a later model control, trailing terminator:
the fixed field: agent CLI exited 1
  crates/brokkr-cli/tests/driver_conformance.rs:324
```

The repair is the hands box's rule, for the reason its own comment
already gives — a command may carry its own `--`: only a LEADING
separator is the CLI's, and nothing else is dropped. Admission belongs
to the driver, and `dsh_control_conflict` already refuses a terminator
as a residual, so the complete payload now reaches a rule that judges
it.

The two argv above joined 8.10's admission ledger as cases on all three
planner paths, refused by `--effort`'s slot and by `--model`'s duplicate
arity, with zero probes, zero producer calls, zero staging and no
retained root. The built-driver case asserts the same refusal plus two
observations an error alone cannot supply: the shim's launch marker was
never written and the admitted home grew no `sessions` directory. No
other adapter's payload handling changed — the leading separator is
still consumed for all five.

### R2 — the DSH storage refusals published the path they tried

Every place this driver allocates storage allocates it beneath the
operator's own layout: the seat overlay and its settings document under
`TMPDIR`, the retained transcript root under the admitted DSH home.
`tempfile` reports both halves of a failure — the allocation AND a
failed write on its own handle — as `<errno> at path "<path>"`, and the
shared `io_context` interpolated the whole thing into an error the seat
reads back. The settings row echoed its path outright.

Demonstrated first, at each site:

```
the path echoed in could not stage the dsh seat overlay: No such file
or directory (os error 2) at path
"/tmp/.tmpVtKQBs/zzz-7c02be-store-absent/brokkr-dsh-privacy-CPoLwv"

the path echoed in could not write the dsh seat overlay: Bad file
descriptor (os error 9) at path
"/tmp/.tmpiFSXb3/zzz-7c02be-store-Wd2bhP"

disabled: the home echoed in could not stage the dsh session transcript
root: Permission denied (os error 13) at path
"/tmp/.tmphpxrrF/zzz-1e84fa-retained/sessions/brokkr/seat-U38UfZ"
```

The write halves were assumed safe when this visit began — the host
reports a write on an open descriptor — and the test disproved that
assumption before any of them was changed. That is why all five sites
moved and not two.

The repair is `dsh_storage_context`, DSH-local: the same categories,
followed by the errno text the HOST words the failure with and nothing
else — the raw OS error where the host gave one, else the kind's own
words, which is what a wrapped error keeps. No literal errno number is
written anywhere, and the vocabulary and storage ownership semantics are
unchanged. `dsh_settings_row` now names its field exactly as
`dsh_transcript_row` does.

The retained-root case is proved under otherwise valid planner inputs on
the cold, offered and disabled paths, which is where the home is really
at stake; it follows the identity observations, so the pre-observation
zero-call rule is not applied to it. The inherited bare `is_err()` at
`adapters/tests.rs:1159` now compares the reason and checks the path
absent.

### R3 — the LOW on the recorded workspace run

`.forge/results/442a91e5-…-checks.json` stands as written. This visit's
gates were crate-scoped and sequential throughout; `cargo test
--workspace` was never run.

### P1 — the LOW on instruction-bearing panel prose

Recorded as the review recorded it: panel notes carry no authority here
and reduced nothing. The two MEDIUMs were answered on their source
evidence, not on their prose.

### Removal proofs

Each mutation compiled, ran alone, failed at the named assertion with a
nonzero test count, and was restored green before the next.

| Broken line | Failing test | Assertion |
| --- | --- | --- |
| `driver_extra_args`'s leading-separator match → back to `position("--")` | `the_driver_payload_keeps_every_argument_past_the_leading_separator` | `left: []` against the whole payload |
| the same line | `the_dsh_admission_rule_reads_the_whole_payload_the_command_line_carried` | `the fixed field: agent CLI exited 1` |
| overlay allocation `dsh_storage_context` → `io_context` | `dsh_storage_refusals_name_their_field_and_never_the_path_they_tried` | `the path echoed in could not stage the dsh seat overlay` |
| settings allocation `dsh_storage_context` → `io_context` | the same | `the path echoed in could not stage the dsh seat settings` |
| transcript-root `dsh_storage_context` → `io_context` | the same | `the path echoed in could not stage the dsh session transcript root` |
| the same line | `a_dsh_retained_root_refusal_names_its_field_and_never_the_home` | `disabled: the home echoed in …` |
| overlay write `dsh_storage_context` → `io_context` | `dsh_storage_refusals_name_their_field_and_never_the_path_they_tried` | `the path echoed in could not write the dsh seat overlay` |
| settings write `dsh_storage_context` → `io_context` | the same | `the path echoed in could not write the dsh seat settings` |
| `dsh_settings_row`'s fixed diagnostic → back to the interpolating `format!` | the same | `the path echoed in dsh driver: settings path "…"` |
| the same line | `dsh_effort_rides_the_seat_settings_document_and_needs_a_model_beside_it` | `"/tmp/a\nb": dsh driver: settings path "…"` |

Each new privacy assertion is guarded by its own source-error check: the
test asserts the `tempfile` error IS path-bearing before asserting the
driver's rendering is not, so none of them can pass vacuously.

### Gates

Crate-scoped and sequential, never `cargo test --workspace` (issue
#255):

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-core` | 86, 0 failed |
| `cargo test -p brokkr-store` | 64, 0 failed |
| `cargo test -p brokkr-protocol` | 488 (388 + 99 + 1 doc), 0 failed |
| `cargo test -p brokkr-runtime` | 24 binaries, 0 failed |
| `cargo test -p brokkr-view` | 243, 0 failed |
| `cargo test -p brokkr-bridge` | 13, 0 failed |
| `cargo test -p brokkr-cli` | 32 binaries, 0 failed |
| `compile --bundle bundles/self` | compiled |
| `compile --bundle bundles/verify` | compiled |

No ETXTBSY attempt was needed on this candidate: every crate suite above
passed on its first run. The gate log is kept at
`.forge/tasks/controller-planner-d-passb-2026-09-21.md`.

`openspec validate --all --strict` COULD NOT RUN in this seat again. The
binary is installed (`/usr/local/bin/openspec` → `@fission-ai/openspec`)
and the sandbox refused every spelling of the invocation, including
`node` on the package's own entry point. This visit adds no
`openspec/specs` delta and touches only this tasks file under
`openspec/`; that is not a substitute, and the check stays owed. `bash
scripts/coverage-exact.sh` did not run here either — its boundary tests
need a namespace the box refuses to nest — and it is not lowered. Native
macOS and remote CI on the final head remain pending; the parent PR's
green checks do not establish them for this head.

### Scope

Four files moved, plus this record. Production gained one function and
changed six lines: `dsh_storage_context` and the five sites that call
it, `dsh_settings_row`'s diagnostic, and `driver_extra_args`'s separator
rule. The shared `io_context` keeps every other caller. The shared
effort splitter, `dsh_input_boundaries`, the production staging
location, the planner signature, the staging counter's seam, the route
binding, the gate-before-probe order, the identity comparisons and the
owned-storage boundary are untouched.

Pass C is untouched and uncredited; it still owes exactly what the
previous record names — a valid prior depth-zero header retained at the
resolved locator for the offered ID, the pinned plugin's
post-`await agents.resume` init event read from the stream-json child on
the selected persistence root, no fresh sibling root or session in the
retained store, and new sequence activity past the recorded `firstSeq`,
before any `root_session`, transcript locator or launch row is
published. A request-derived `session_id` is not that event, and
`confirms_from_locator: false` stays.

8.8 and 8.10 stay unchecked and no sub-clause was ticked. 9.6, 10.x,
11.1–11.4 and groups 14–15 were not touched. `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/dsh/` and `docs/decisions/` have no diff;
decision 0056 keeps its `proposed` status; the DSH route stays disabled.
The research-dsh roster assertion, `bundle.json`, `research-web.yml`,
compiled staffing and every witness digest are unmoved. No live provider
was called. Nothing was pushed.

No Windows handling was added. Decision 0063 (accepted 2026-09-21)
withdraws every Windows obligation the (d) text names, including the
native lookup oracle, GetBinaryTypeW, the native matrix, Windows MSRV
and the Windows-specific transport obligation.

## Implement visit — the surviving terminator MEDIUM, 2026-09-21

Run `dsh-launch-planner-issue-226-tas-10abd37c`, on `slice-dsh-planner-d`
at `d65f0784`. The previous run
(`dsh-launch-planner-issue-226-tas-3d08ce19`) parked
REVIEW-REFORGE-EXHAUSTED-MEDIUM on one finding and stays as it is for the
operator; this visit repairs that finding and the LOW beside it. Every
existing commit is adopted.

### R1 — the repair the previous visit made was half of one

The previous visit stopped `driver_extra_args` from searching the WHOLE
vector, and kept the deletion of a LEADING `--`. But clap consumes the
operator's outer separator ITSELF. Nothing the CLI receives is that
separator — so the surviving deletion was not a boundary at all, it was a
normalisation, and it swallowed exactly the token the DSH admission rule
exists to refuse:

| the operator's command line | what admission received | what it did |
| --- | --- | --- |
| `brokkr driver dsh -- --` | nothing at all | launched, staged, retained a root |
| `brokkr driver dsh -- -- --model p/m` | `--model p/m` | launched on that model |

The rule, not the instance: after clap parsing the DSH adapter's
admission sees every payload token exactly as given — leading, interior
and trailing alike. An adapter can only refuse what it receives.

Every separator site in `brokkr-cli` was audited and each ruled on its
own:

| Site | What it does | Ruling |
| --- | --- | --- |
| `lib.rs` `driver_extra_args` | drops one LEADING `--` from a driver payload | kept for claude, lanetally, codex, exec; DSH opts out |
| `lib.rs` `Cmd::Driver` dispatch | called it for all five | now calls `driver_payload(kind, args)` |
| `lib.rs` `HandsCommand::Exec` | drops one LEADING `--` from a boxed command | untouched — a separate command boundary |

`git grep '"--"' -- crates/brokkr-cli/src` returns seven hits and no
eighth site: the two removals above, four `init.rs` scaffold strings that
WRITE a `--` into a generated recipe, and `recipes.rs`'s `git clone`
terminator.

The opt-out is `driver_payload`, a two-arm match on `AdapterKind`. The
shared helper is untouched and still owns the other four, so their
behaviour cannot move under a DSH repair — and it did not move: their own
suites ran green, including `conformance_across_all_builtin_adapters`,
the three codex rejoin proofs and every adapter's refusal shape (22
passed in `driver_conformance`, 0 failed). That is executed evidence, not
an assertion in prose.

### The built-CLI proof

`a_residual_terminator_refuses_on_every_dsh_path_wherever_it_stands`
drives the BUILT binary 18 times: six payloads on each of disabled,
enabled-cold and offered. Bare `--`; a leading `--` before an admissible
pin; an interior `--`; a trailing `--`; and the two route-ordering cases
below. Each asserts the fixed option-terminator reason — never
`is_err()`, never a status — and asserts the diagnostic echoes none of
the model id, the route value, the offered session id, any `--` spelling
at all, the workdir, the DSH home, the operator HOME or the shim path.

Zero provider observations in all 18, each one an owned-shim or
filesystem fact an error string cannot supply: no `--version` probe, no
child, no staged overlay (the shim copies whatever `--patch` hands it),
no retained root under `$DSH_HOME/sessions/brokkr`, no checkpoint.

The last two payloads bind `resume_context.route_overlay` to a file that
is never created, so an argv that reaches route resolution refuses in the
route's own words. Both also assert the error never says `route_overlay`:
the terminator refuses first from either position.

`the_dsh_launch_paths_the_terminator_proof_runs_on_are_the_ones_it_names`
proves the fixtures are what they claim, each path identified by an
observation only that path produces: disabled probes no version at all;
an open gate probes it exactly once and ships `cold`, because the
declared wrapper digest is not this host's composite; an open gate handed
a session back declines it as `resume_refusal: unverified-harness`. All
three launch a child, stage the overlay and retain one root. Without this
test the 18 refusals could have passed over a fixture that refused for
some unrelated reason.

### R2 — the fixture's temporary roots

`canonical_root()` returns one `tempfile::TempDir` and its canonical
path; `DshFixture` keeps the owner beside the path and derives the
workdir, shim, logs, `DSH_HOME` and operator `HOME` from the canonical
form, once each. macOS resolves the system temporary directory through
`/var`, a symlink to `/private/var`, so a raw root and the path the
driver canonicalizes are two strings for one directory. The old
`drive_dsh` helper, which derived from raw roots, is gone; the retained
`the_dsh_admission_rule_reads_the_whole_payload_…` keeps both its cases
and its fixed-field assertions on the new fixture, with the full
zero-observation set added. Executed on Linux only; no native macOS
execution occurred and none is claimed.

### Removal proofs

The deletion was restored (`AdapterKind::Dsh => driver_extra_args(args)`),
each test run alone, then the fix restored and every test re-run green.
The final tree carries the fix.

| Broken line | Failing test | Assertion |
| --- | --- | --- |
| `driver_payload`'s DSH arm → back to the shared helper | `the_driver_payload_keeps_every_argument_the_dsh_command_line_carried` | `tests.rs:1186` — `left: []` against `right: ["--"]` |
| the same line | `a_residual_terminator_refuses_on_every_dsh_path_wherever_it_stands` | `driver_conformance.rs:370` — `Disabled/a bare terminator alone: the fixed option-terminator reason: agent CLI exited 1` |
| the same line, with the route case moved to the head of the list | the same | the same assertion — `Disabled/a leading terminator with a bound, absent route overlay: … refusing to invoke the dsh driver: route_overlay file is unreadable` |

`agent CLI exited 1` is the shim's own exit: the payload was ADMITTED and
a child ran. The third row is the reviewer's ordering finding reproduced
exactly — a leading terminator reaching route-file resolution.

### Gates

Crate-scoped and sequential, never `cargo test --workspace` (issue #255:
the verify exec is the controller's and is not a seat's choice).

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `cargo test -p brokkr-core` | 86, 0 failed |
| `cargo test -p brokkr-store` | 64, 0 failed |
| `cargo test -p brokkr-protocol` | 488 (388 + 99 + 1 doc), 0 failed |
| `cargo test -p brokkr-runtime` | 24 binaries, 0 failed |
| `cargo test -p brokkr-view` | 243, 0 failed |
| `cargo test -p brokkr-bridge` | 13, 0 failed |
| `cargo test -p brokkr-cli` | 32 binaries, 0 failed |
| `compile --bundle bundles/self` | compiled |
| `compile --bundle bundles/verify` | compiled |

No ETXTBSY attempt was needed: every crate suite passed on its first run.
The evidence log is kept at
`.forge/tasks/dsh-pass-b-terminator-delivery.md`.

`openspec validate --all --strict` COULD NOT RUN in this seat, as on the
previous visits: every spelling was refused, and the binary's own
directory is outside the seat's allowed paths. This visit adds no
`openspec/specs` delta and touches only this tasks file under
`openspec/`; that is not a substitute and the check stays owed. `bash
scripts/coverage-exact.sh` did not run here either — its boundary tests
need a namespace the box refuses to nest — and it is not lowered. Native
macOS and remote CI on the final head remain pending.

### Scope

Three files moved, plus this record. Production changed by one function:
`driver_payload`, and the one dispatch line that now calls it.
`driver_extra_args` keeps its body and its four callers' behaviour. The
protocol crate has no diff at all: `dsh_control_conflict` already
categorised a residual `--` as `the option terminator` before any route
read, version probe, composite call or staging, and this visit only made
sure the token reaches it.

Pass C is untouched and uncredited. It still owes, before any
`root_session`, transcript locator or launch row is published: a valid
prior depth-zero header retained at the resolved locator for the offered
ID; the pinned plugin's post-`await agents.resume` init event read from
the stream-json child on the selected persistence root; no fresh sibling
root or session in the retained store; and new sequence activity past the
recorded `firstSeq`. A request-derived `session_id` is not that event,
and `confirms_from_locator: false` stays.

8.8 and 8.10 stay unchecked and no sub-clause was ticked. 9.6, 10.x, 11.x
and groups 14–15 were not touched. `contracts/`,
`policy/phase-machine.json`, `policy/schemas/`, `fixtures/`,
`reference/`, `extensions/dsh/` and `docs/decisions/` have no diff;
decision 0056 keeps its `proposed` status; the DSH route stays disabled.
No live provider was called. No Windows handling was added (decision
0063). Nothing was pushed.
