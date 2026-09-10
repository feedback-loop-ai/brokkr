## Current operator disposition — 2026-09-09

R1–R4 are ruled by the accepted
[0046 addendum](../../../docs/decisions/0046-the-boundary-is-named.md#accepted-addendum--seatbelt-observables-and-proof-obligations).
Private overlay snapshots with explicit locators and denied-read masks are
permitted; no surviving payload remains mandatory; private Git hooks qualify
only with independent write protection and native adversarial evidence.

The operator explicitly requires demonstrated guarantees, with missing proof
recorded as residuals. See [the evidence inventory](evidence-residuals.md).
No native enforcement result is supplied by this acceptance. Seatbelt remains
unbuilt. This return reconciles the deltas to the accepted observables and names
a coherent two-job native lifetime candidate and proof gate. Only that bounded
feasibility probe may precede proof; full Seatbelt implementation remains
blocked until a real
macOS run demonstrates the mandatory no-survivor guarantee. No Linux, mock,
source-level or process-group-only result is native enforcement evidence.

Candidate `6a19a6f4ab9bd30b47537de1a649949cd1099d01` was measured by native
macOS CI run `34433461814` on macOS 26.6.2 arm64. It did not reach a lifetime
trigger: `/usr/bin/sandbox-exec` started `/usr/bin/python3` directly under the
experimental profile and the process aborted with `SIGABRT` and empty output;
the equivalent launchd payload job registered, ran once, recorded one
successive crash and was no longer running. Every payload heartbeat therefore
remained still. This identifies a startup failure, not its cause and not
evidence for or against launchd containment. SEATBELT-R3 remains open.

Successor candidate `8c53dcecaef414938b3abfb8911a77d9ec958f23` was measured by
native CI `34441725835` on the GitHub `macos-latest` arm64 runner. The
committed Rust helper digest `e925083d55a9c8b0` reached `READY`, identified
an ordinary child and exited cleanly in the direct unboxed control. The
identical helper and argv aborted with signal 6 before `READY` under the exact
Seatbelt profile; a separately labelled `allow default` diagnostic exited zero,
which localizes the difference to authority withheld by the exact profile but
does not identify or authorize an allowance. The launchd cells did not produce
a coherent control: across the two Gate A executions, an unboxed job once
reached `READY` but had no parseable run/crash facts and once failed bootstrap
with error 5, while the sandboxed job alternated between bootstrap failure and
a non-ready exit with no valid run/crash facts. Gate B was correctly not run.
The run therefore proves neither launchd startup nor lifetime containment; it
requires isolated launchd measurements and bounded profile diagnosis. The
successor repair commits the portability, label/root isolation, canonical-path,
parsed-terminal-state, staged-startup and one-authority diagnostic changes and
left the controller-dispatched exact-head Gate A run as its next prerequisite;
that completed run is reconciled below. No production Seatbelt path is
authorized by that repair.

Candidate `9f4c2c944cac217ccb8dc055971cc62614313ed4` was measured by native
CI `34449331270` on the GitHub `macos-latest` arm64 runner. The generic macOS
workspace and Windows workspace passed, and S0 again proved that the committed
helper, arguments, ordinary child and bounded stages work unboxed. S1 aborted
with signal 6 before the first stage under the exact restrictive profile. None
of the seven one-class diagnostics reached `READY`; only the forbidden
`allow default` control did, so the withheld operation and target remain
unidentified and no profile broadening is authorized. S2 is a launchd
observation refusal, not a payload result: its terminal output did not populate
the parser's required not-running shape. S3 recorded a not-running job with one
run and one crash but no authenticated startup fact. Gate B was correctly not
run, denial controls were unobservable because the helper never started, and
SEATBELT-R3 remains open. The report also exposes two probe measurement defects:
cell-private roots make raw instantiated profile digests differ even when the
authority template is identical, and an incomplete launchd terminal parse
currently erases separately observable readiness, stages and output. The next
candidate must compare a normalized profile template, preserve every launchd
fact independently, and identify the denied operation/target from native denial
evidence or a minimal staged syscall probe before changing the candidate.

Candidate `fa7ece587178a46baa66a7310e0546bfb87a0857` was measured by native
CI `34457208029` on the GitHub `macos-latest` arm64 runner. The root-inode read
`(allow file-read* (literal "/"))` moved S1 and S3 past the pre-stage abort:
both now reach `entry`, `payload-dir` and `executable`. The ordinary-child spawn
then fails with `EPERM`, and the helper exits 2 without `READY`. Every
one-class diagnostic fails the same way, and none of them grants any
`file-write*`. Only the forbidden `allow default` control starts. Credential
read, host write and loopback bind were observed denied; those are facts about
this candidate, not proof of R1–R4. No Seatbelt cell reached `READY`, so no
root-inode removal control was due. The empty `negative_controls` lists are that
designed outcome, not a missing record, and the predicate stays unproven as
load-bearing until a starting cell observes its removal. S2
reached `READY`, the exact stages and an ordinary child. Its raw terminal print
shows `state = not running`, `runs = 1` and `last exit code = 0`, and S3's shows
`last exit code = 2`. Neither prints a `successive crashes` line. The parser
refused both whole prints on that absent counter and erased the printed facts.
This exposes a specification inconsistency, not only a code defect: the
seatbelt-execution delta failed any omitted crash field, so as written no
launchd cell could ever pass. This visit reconciles the delta around the facts
launchd actually prints and names the discriminating child-spawn evidence the
next probe must collect. A later clarify visit scoped removal controls to
cells that reach `READY` and corrected the record of fa7's empty lists. Gate B
was correctly not run and SEATBELT-R3 remains open.

The next clarify visit, on `5dca1d0`, found that the record named a
removal-set completeness test with no source to check it against. No artifact
said which template rules were baseline and which were admitted by diagnosis,
and "justified baseline" was undefined. `8c53dce` had also added four
startup-motivated rules without recording why: `/private/var/tmp`, the helper
literal, `/dev/random` and `/dev/dtracehelper`. This visit enumerates every
template rule in an audited startup-rule ledger. It keeps what a hands element,
execution input or probe-harness need justifies and the one diagnosis-admitted
root-inode read. It withdraws or narrows every other fa7 rule. The candidate
therefore changes, and its startup is unmeasured until the controller's next
exact-head native Gate A.

The clarify visit on `353818d` accepted that ledger and found two gaps in it.
First, the normalization was not closed. A multi-operation `allow` form, an
action modifier, a compound filter or a non-`allow` top-level form was neither
a rule unit nor refused, so a parser could let an unlisted rule past the
equality. The delta now states a closed grammar. After the frame, every form
is a single-operation `allow` form of simple filters. Anything else fails and
is named. Second, the `/usr` read and exec units were baseline hands elements
wider than the `hands.rs` binds they cited. The box binds only listed `/usr`
children, not `/usr` itself. They are narrowed to one subpath per bind. The
check reads the binds from `box_argv` itself (the `a66b559` visit below
replaces that with the named `hands.rs` item). The process authority is listed
as seven units rather than called "exact-target". The narrowing changes the
candidate again, and its startup stays pending native Gate A.

The clarify visit on `d63cddf` found that the same visit had made the ledger
contradict itself. A toolchain unit had to target exactly its `--ro-bind-try`
source, while a widened correction clause let denial evidence replace a
toolchain unit with another resolved spelling. Keeping both kills the clause.
Relabelling the respelled unit as another hands element escapes the
`box_argv` check, which is fail-open. This visit rules that toolchain units
never respell. A toolchain denial under another spelling stays the named
startup residual `SEATBELT-R3-STARTUP-toolchain-respelling`, and changing it
needs a focused proposed decision. The relabelling escape is closed
mechanically. Every hands-element entry names one element of a closed set and
is tested against that element's anchor. No unit except a toolchain unit may
cover a bind source. No unit may target a bind's data-volume spelling or
contain `/System/Volumes/Data`. Only the system-library element keeps a
correction, now typed and bounded. The candidate's rule units do not change,
so its startup stays pending the same native Gate A.

The clarify visit on `a66b559` found that the check's own inputs were never
fixed. The toolchain anchor, the cover rule and the data-volume rule read "the
`--ro-bind-try` sources that `box_argv` renders". `box_argv` also renders
every declared `ro` bind, home-expanded, and the git common `config`. So a
declared `~/.rustup` could pass as a toolchain unit, and the verdict depended
on the host's home and git layout. The concrete cell root, payload root and
helper that normalization rewrites also came unchecked from the renderer the
check says it must not trust. This visit names the host-toolchain list as one
`hands.rs` item, `HOST_TOOLCHAIN_BINDS`, which `box_argv` iterates and the
check reads. Declared binds and the common `config` are never toolchain binds,
and the check takes no `HandsSpec`, home or `GitFacts`. The concrete roots,
helper and path-valued denial-control targets become typed check inputs,
validated for layout, canonical spelling and disjointness before any string is
normalized. The cover and data-volume rules, and a new rule that no unit
covers a denial-control target, also read each unit's concrete target. What a
string check cannot prove, such as a fresh observer-created root and
canonicalization equality, is a recorded observer duty. The candidate's rule
units do not change, so its startup stays pending the same native Gate A.

The clarify visit on `48b6f9d` accepted those inputs and found one asymmetry
the correction itself introduced. The cell root and helper had to be spelled
under `/private`, and the denial-control targets were listed in both
spellings. Host-toolchain sources, though, were compared only in their direct
spelling. `HOST_TOOLCHAIN_BINDS` carries six `/etc` sources, which macOS
resolves under `/private/etc`. So the cell root `/private/etc/ssl/brokkr-cell`
passed validation and received the payload write. A non-toolchain
`(subpath "/private/etc/ssl")` or `(literal "/private/etc/ld.so.cache")` also
passed the cover rule. That reopened, for those sources, the relabelling
escape the `d63cddf` visit had closed. The data-volume rule was also worded two
ways. This visit gives each host-toolchain source a fixed spelling set: its
direct spelling, plus the `/private` spelling when its first component is
`etc`, `var` or `tmp`. Cell-root validation and the cover rule compare with
every spelling, and the toolchain anchor still reads only the direct one. The
data-volume rule now refuses every unit on or under `/System/Volumes/Data`. The
candidate's rule units do not change, so its startup stays pending the same
native Gate A.

# Change: Seatbelt on macOS — decision 0046 slice (ii)

## Why

Slice I (PR #230, `7b53e92`) names and pins `seatbelt`, but this engine
still refuses it even when `sandbox-exec` exists. macOS needs a real
execution boundary for workspace calls and boxed exec seats, with the
security guarantees of decision 0043 and honest evidence of what was
measured, as accepted decision 0046 requires.

## What Changes

- Implement the `seatbelt` path for both the workspace MCP tool and whole
  boxed exec dispatches. Preserve the hands policy: cleared environment,
  private call HOME/tmp, controlled toolchain access, network denied unless
  declared, writable worktree, `ro`/`rw`/`overlay` binds, credential masks,
  protected git metadata, bounded output and process lifetime.
- Realize arbitrary overlays as seat-private snapshots at explicit replacement
  locators. Define generic transport, stable seat lifetime, isolation,
  later-source invisibility, link/mask handling and identity consequences.
  Redirect shipped Cargo and npm consumers without special-casing them as the
  whole feature.
- Make a present Seatbelt mask a denied read, never a claimed empty read.
  Preserve ordinary neighbors and host bytes through aliases, overlaps and
  mutation attempts; retain namespace's existing observable unchanged.
- Direct ordinary Git to an empty private hooks directory and deny host hook
  access. Full-peer status requires independent raw hook/config/routing write
  protection proven in primary and linked worktrees; an environment override
  or successful unsigned commit alone is insufficient.
- Before full implementation, probe the **per-invocation transient launchd
  lease pair**: a payload job using candidate job/process-coalition ownership
  and a separately launchd-owned liveness-guard job that survives payload
  `bootout`. It is a named candidate, not a claim: real macOS setsid,
  double-fork, timeout, cancellation and supervisor-SIGKILL adversaries must
  pass.
- Prove payload startup before exercising any lifetime trigger. Prefer a
  self-contained native Rust test helper under the exact experimental profile;
  if an interpreter is retained, use differential controls or denial/system
  evidence to identify each runtime allowance before adding it. Never widen
  the profile merely to make a payload start. Direct and launchd-owned startup
  controls must both reach an externally observed ready state before their
  observations enter the lifetime matrix.
- Give every launchd startup cell and repeated Gate A invocation unique job
  labels and private paths; prove the label absent before bootstrap and after
  bootout, and preserve raw bootstrap, print, exit and cleanup outcomes.
  Reused-label races, a missing required terminal fact (`state`, `runs` or
  `last exit code`) or inferred exits fail the startup measurement. An
  omitted `successive crashes` counter is recorded unknown with its raw
  sample, never synthesized as zero; a present nonzero counter fails.
  Every launchd job fact is read from the print's top-level dictionary, never
  from a nested coalition or other block and never by print order; a
  duplicated top-level key is unknown and fails the cell.
  Diagnostic broad profiles never become candidates.
- Split the helper's ordinary-child stage into stream-setup, spawn/exec and
  child-observed sub-stages with per-step OS errors. Discriminate a child-spawn
  refusal with a no-spawn `/dev/null` write cell, an inherited-stdio spawn, a
  null-stdio spawn and one literal `/dev/null` write-data diagnostic before any
  predicate is admitted. An admitted predicate is literal-scoped and has its own
  removal control. The removal set is exactly the diagnosis-admitted
  predicates. Every entry must be observed blocking on each Seatbelt startup
  cell that reaches `READY` before that cell can pass. Each removal is a direct
  `sandbox-exec` replay of the cell's own profile, S3 included. A cell without
  `READY` records its removals as not due and fails on its own startup facts.
- Account for every rule of the experimental template in one typed
  startup-rule ledger. A baseline entry names a hands element, an execution
  input or a probe-harness need. A diagnosis-admitted entry names its
  operation, single-object target, process, consumer and evidence, and has a
  removal entry. The justified baseline is exactly the baseline half. A
  host-independent check proves the rendered template's rule units equal the
  disjoint union of the baseline and the removal set. The check's grammar is
  closed: after the frame, every form is a single-operation `allow` form of
  simple filters, and a multi-operation form, modifier, compound filter,
  other top-level form or unparseable text fails by name. The ledger narrows
  the unfiltered process family to `process-fork` and six `process-exec`
  units (the helper and the `/usr/bin`, `/usr/libexec`, `/usr/local`, `/bin`
  and `/sbin` binds). It narrows `/usr` to one subpath per `hands.rs` bind,
  checked against the host-toolchain list that `hands.rs` names and
  `box_argv` iterates, and narrows `/System` and the cell root. Declared binds
  and the git common `config` are never toolchain binds. The check takes no
  `HandsSpec`, home or `GitFacts`. It validates its typed concrete cell root,
  payload root (`<cell-root>/payload`), inputs directory and helper before
  normalizing anything. The observer records the file-system facts a string
  check cannot prove.
  Toolchain units never respell: a toolchain denial under another spelling is
  the startup residual `SEATBELT-R3-STARTUP-toolchain-respelling`. Each
  hands-element entry names one element of a closed set and is tested against
  its anchor. Only a toolchain unit may cover a bind source, in any of the
  source's spellings, which include the `/private` spelling of an `/etc`
  source. No unit may target anything on or under `/System/Volumes/Data`.
  Only the system-library element keeps
  a typed, bounded correction. The ledger withdraws the self-signal,
  `/Library`, host tmp,
  `/dev/dtracehelper`, `sysctl-read` and `ipc-posix-shm` rules. A withdrawn
  rule returns only as a non-admitting restoration diagnostic followed by
  single-object attribution and a removal control.
- Compare the two Seatbelt startup profiles as one normalized authority
  template with typed substitutions for each private cell root and payload
  root. Record the concrete bytes and digest per cell, but do not mistake those
  required substitutions for authority drift.
- Diagnose startup at an operation and target boundary. Preserve native denial
  reports and use staged, minimal helpers or bounded combination controls when
  no single broad class suffices; no broad class enters the candidate merely
  because a diagnostic combination starts.
- Keep probe helpers portable: non-Darwin workspace builds must compile and
  link without unresolved Unix symbols even though native cases run on macOS.
- Thread the compiled realm boundary through runtime composition, both
  hands CLI verbs and adapter MCP configurations, entry refusals and doctor.
  Test the existing manifest, effect, seat-record and readout contracts
  against real Seatbelt dispatches. Handle the engine binary and declaring
  bundle layers without assuming Linux's `/runtime/bundle` mount exists.
- Add required behavioral tests on the existing macOS CI runner, alongside
  host-independent policy/composition tests and Linux regressions. Update
  the guides with the actual mechanism, restrictions, invocation examples
  and the evidence status; distinguish implemented code from measured
  acceptance of the whole slice.

This run owns slice II only. `container` stays unbuilt and refuses as
slice III, including when Docker or Podman exists; `driver.confine` stays
retired. The repository's default boundary, accepted decision statuses,
frozen contracts, production policy, reference material and evaluator
corpus are not rewritten. Any required wire change is an additive version.
No provider roster or global agent configuration change belongs here; the
controller selects the control library, which the 2026-09-10 operator override
set to the compiled `claude-flash` recipe. Delivery and all remote actions
remain the controller's; no nested Brokkr run is needed.

## Capabilities

### New Capabilities

- `seatbelt-execution`: Enforce the declared hands policy through real
  `sandbox-exec` execution, including bind and git adversaries, child
  lifecycle, pinned inputs, and the evidence required to accept slice II.

### Modified Capabilities

- `boundary-availability`: Activate Seatbelt only after both paths implement
  the accepted policy and every required native residual is closed; define the
  system-launcher probe, host/tool refusals and macOS init advice; retain
  slice III. Refusing the shipped overlay users is not this deliverable.
- `realm-boundary`: Keep compilation machine-independent and pin the
  realm's Seatbelt word while making the start refusal about availability.
- `gate-boundary-policy`: Route boxed model, exec and dialect sites through
  Seatbelt; retain the gate law and the documented unboxed script limits.
- `boundary-guides`: Describe the actual macOS boundary and its evidence,
  preserving platform classes and the distinctions between boundaries.

## Impact

The implementation reaches `crates/brokkr-protocol/src/hands.rs` and its
process/driver integration, `crates/brokkr-runtime/src/engine.rs`, agent
resolution and composition, `crates/brokkr-cli` hands commands and boundary
probes, their tests, `.github/workflows/ci.yml`, and the hands, quickstart,
provider, driver, journal and read-surface guides. The existing
`boundary-manifest-pin`, `boundary-record` and `boundary-readouts`
requirements remain authoritative; new scenarios exercise them end to end.
Witness and compose pins move only for measured identity changes, with the
reason recorded. No engine version bump is commissioned.

## Decisions
This visit adopts the committed `boundary-seatbelt-slice-ii` change at the
preserved starting HEAD `a66b559`, which is measured candidate `fa7ece5` plus
its specification reconciliations `098914c`, `c84502d`, `5dca1d0`, `353818d`,
`d63cddf` and `a66b559`; `225d2c7`, `8c53dce`, `9f4c2c9` and their
native measurements remain historical evidence, not checkout targets. It answers the
current findings in dependency order. The
accepted 0046 addendum supersedes the old R1, R2 and R4 questions; it does
not manufacture evidence. Only the proposal and five capability deltas are
authored in this specify phase. No workflow runner is invoked.

### Returned findings and current disposition

| Finding | Disposition and reason | Owning delta/scenario |
|---|---|---|
| R1 — overlay and shipped-library deliverable | **Adopt.** Every overlay uses a generic explicit replacement locator to a complete seat-private snapshot; later source changes are invisible, and locators are not portable identity. Cargo/npm redirects consume the generic map. | `seatbelt-execution`: Explicit locators cover arbitrary overlays |
| R2 — readable-empty masks | **Adopt.** Seatbelt uses a permission-class denied read with no content. It is documented as denial; namespace remains unchanged. | `seatbelt-execution`: Present Seatbelt masks deny reads |
| R3 — detached descendants | **Retain.** No surviving payload is mandatory. The per-invocation transient launchd lease pair uses a candidate payload job/process coalition plus a separately owned guard job; only native adversarial proof may establish feasibility. | `seatbelt-execution`: Native lifetime feasibility precedes full implementation |
| SEATBELT-SPEC-LIFETIME-TOPOLOGY | **Adopt.** The guard cannot be inside the payload job it must boot out. It is a separately launchd-owned job in the same per-user bootstrap domain, remains alive through payload teardown, establishes quiescence before cleanup, and then unregisters itself. This repairs the specification without claiming that public launchd can contain detached descendants. | `seatbelt-execution`: The guard remains outside the payload job |
| SEATBELT-R3-STARTUP — native CI `34433461814` | **Adopt as a failed prerequisite, not a lifetime verdict.** Candidate `6a19a6f` aborts direct sandboxed Python with `SIGABRT`; launchd records one crashed run and no heartbeat. The cause is not established. Diagnose startup with bounded controls before any trigger, survivor or quiescence observation is admissible. | `seatbelt-execution`: Payload startup is proved before lifetime is measured |
| SEATBELT-R3-STARTUP — native CI `34441725835` | **Adopt as a narrower failed prerequisite.** Candidate `8c53dce` proves the committed Rust helper and argv work directly unboxed, then reproduces signal 6 only under the exact restrictive profile. The `allow default` diagnostic narrows the layer but authorizes nothing. Gate B correctly remains not run. | `seatbelt-execution`: A broad diagnostic never becomes an admitted profile |
| Launchd startup measurement isolation | **Adopt as a probe defect.** S2/S3 results varied between the two Gate A runs and lacked reliable run/crash facts. Each cell and invocation therefore needs a unique label/root, pre-bootstrap absence, explicit kickstart/terminal observations, raw command evidence and confirmed post-bootout absence; inferred exits and label reuse cannot pass. | `seatbelt-execution`: Launchd startup cells are isolated and fully observed |
| Windows helper link failure | **Adopt as probe portability, not native security evidence.** Unconditional `getuid`/`getpgid` references break the Windows workspace test link. The helper must use target-correct implementations or be structurally unavailable off Unix without hiding shared decision logic. | `seatbelt-execution`: Probe support does not break non-Darwin validation |
| Native CI `34449331270` at `9f4c2c9` | **Adopt as a third failed startup prerequisite.** S0 and both generic workspaces pass; S1 aborts before stages, every one-class diagnostic fails, and only forbidden `allow default` starts. S2 is an observation refusal and S3 is one non-ready crash. No denial or lifetime fact is established. | `seatbelt-execution`: The third startup run still identifies no admissible authority |
| Cell-private profile comparison | **Adopt as a measurement defect.** Unique roots are required isolation, so their concrete profile literals and digests must differ. Compare a normalized policy template and typed root substitutions for equal authority while retaining each concrete profile as evidence. | `seatbelt-execution`: Cell-private roots are not profile drift |
| Lossless launchd observation | **Adopt as a measurement defect.** Failure to parse one terminal field must fail the cell but cannot erase separately observed `READY`, stages, child identity, raw lifecycle output or individual parsed fields. | `seatbelt-execution`: An unknown launchd field does not erase other facts |
| Exact startup operation | **Partly answered.** The pre-stage abort is named as the root-inode read and fa7 moved past it. The child-spawn refusal that follows is still unattributed. Broad combination success alone authorizes nothing. | `seatbelt-execution`: Startup diagnosis names operations and targets |
| Native CI `34457208029` at `fa7ece5` | **Adopt as a fourth failed Gate A prerequisite with measured progress.** S1 and S3 reach `executable`, then the child spawn fails with `EPERM` and exit 2. Three denial controls are observed denied. S2 reached `READY`, stages and a child. The report erased S2's and S3's printed terminal facts. Gate B was not run. | `seatbelt-execution`: Probe payload startup is established separately |
| Launchd crash counter omitted by macOS | **Adopt as a specification inconsistency.** The measured terminal print omits `successive crashes` for exit 0 and exit 2, so failing an omitted counter makes every launchd cell unpassable. `state`, `runs` and `last exit code` are required. An absent counter stays unknown with its raw sample and is never zero; a present nonzero counter fails. The pass still needs the helper's authenticated facts. | `seatbelt-execution`: An omitted crash counter stays unknown; Required launchd terminal facts still fail closed |
| Launchd print dictionary scope (clarify, fa7) | **Adopt top-level-only.** The fa7 S2 and S3 terminal prints repeat `state` and `active count` inside both coalition blocks, where they read `active` and 1 even for S3's never-spawned child. Job facts come only from the top-level dictionary of the single outermost block, independent of print order. Nested blocks never supply, complete or override a job fact. A missing top-level key stays unknown. A duplicated top-level key is unknown and fails the cell. Coalition fields are raw evidence only, never liveness, quiescence or survivor facts. First-match scanning, failing on any nested repeat and nested fallback are refuted. The fa7 samples become the parser's regression fixtures in the probe's test data. | `seatbelt-execution`: Launchd job facts come only from the top-level dictionary; The running fa7 sample is non-terminal; Nested or duplicated keys never manufacture a job fact |
| Child-spawn authority | **Adopt as the next discriminating measurement.** Every failing cell withheld `file-write*` outside the payload, while null stdio opens `/dev/null` for writing before exec. That is the leading hypothesis, not a conclusion. Per-sub-stage errors plus no-spawn, inherited-stdio, null-stdio and single literal diagnostic cells decide it or move attribution to the exec. | `seatbelt-execution`: Child-spawn refusal is localized to its sub-stage |
| Root-inode removal control unobserved | **Corrected by the removal-scope clarify answer below.** No fa7 Seatbelt cell reached `READY`, so no removal was due. The empty `negative_controls` lists on S1 and S3 are the designed not-due outcome, not a gap, and S0 and S2 are unboxed. The root-inode read stays unproven as load-bearing until a cell that reaches `READY` observes its removal blocking. On such a cell an empty record fails. | `seatbelt-execution`: A Seatbelt cell without an observed removal control fails |
| Removal-control scope (clarify, fa7) | **Adopt READY-only.** Removal controls are due on exactly the Seatbelt startup cells that reach a nonce-authenticated `READY`. Blocking means the stripped replay, from fresh payload state, reaches no `READY`. A non-`READY` cell records each removal as not due and fails on its own facts. The set is exactly the diagnosis-admitted predicates, verbatim as the template carries them; a host-independent test checks that set against the template. Each removal is a direct `/usr/bin/sandbox-exec` replay of the cell's own profile, S3 included, because launchd adds no Seatbelt authority; a launchd-only predicate is not admitted. Every-cell removal with a stage-relative "blocked" is refuted. Necessity is proven only against a candidate that starts, and a later predicate can change it. An earlier failure of a stripped non-starting run only shows that the predicate advances the stages, so recording it as load-bearing would fabricate proof. Cross-candidate stage progress stays diagnosis evidence. | `seatbelt-execution`: A non-starting Seatbelt cell owes no removal verdict; A launchd cell's removal replays the profile directly; The removal set is exactly the diagnosis-admitted predicates |
| Startup-rule ledger (clarify, `5dca1d0`) | **Adopt.** The removal-set completeness test had no classification source. Every template rule is now a normalized rule unit (one operation, at most one filter) with exactly one class in a typed ledger. The justified baseline is defined as the baseline half. The check parses the rendered template and requires its units to equal baseline ⊎ removal set, so an unlisted, doubly classified or missing unit fails. The subset-only check it replaces could not detect a missing diagnosis-admitted rule. | `seatbelt-execution`: The experimental startup template is an audited rule ledger; The template is exactly the ledger's disjoint union |
| The four `8c53dce` additions | **Dispose explicitly.** The helper literal is justified baseline as an execution input, and `/dev/random` as a hands element (the box's `--dev /dev` device set). `/private/var/tmp` contradicts the private per-call tmp and is withdrawn. `/dev/dtracehelper` is in no hands element and has no measurement behind it, because `8c53dce` still aborted on the root inode, so it is withdrawn. | `seatbelt-execution`: The four 8c53dce additions keep their disposition |
| Ledger normalization closure (clarify, `353818d`) | **Adopt a closed grammar.** A unit was defined only for single-operation forms, so a form with several operations, a `(with ...)` modifier, a `require-*` compound filter or a `trace`, `define`, `if` or `debug` form was neither a unit nor refused. A first-operation or skip-unknown parser would satisfy the text and pass an unlisted rule. After the frame, every top-level form must be `(allow OPERATION FILTER...)`, with exactly one operation and simple one-argument filters. A multi-filter form still normalizes to one unit per filter, because it grants wherever any filter matches. The check refuses and names a multi-operation form, which is not split into N×M units, so a grouping renderer cannot hide an operation. It also refuses and names any modifier, compound filter, other top-level form, comment, escape or duplicate unit. Splitting multi-operation forms was refuted: it would admit a rendering nobody writes and make a removal rewrite a form. An unknown simple filter needs no vocabulary, because the equality already refuses it as unlisted. | `seatbelt-execution`: A multi-operation form cannot hide an operation; A top-level form other than the frame and allow forms fails; Modifiers, compound filters and unparseable text fail |
| `/usr` baseline wider than its element (clarify, `353818d`) | **Narrow; the whole tree is refuted.** `hands.rs` creates an empty `/usr` and binds only `/usr/bin`, `/usr/lib`, `/usr/lib64`, `/usr/include`, `/usr/share`, `/usr/local` and `/usr/libexec`. `(subpath "/usr")` also covers `/usr/sbin`, `/usr/standalone` and the other unbound children, and no hands element names them. The same reasoning already narrowed `/System` and the cell root. Reads become one subpath per bind: `/usr/bin`, `/usr/lib`, `/usr/libexec`, `/usr/share`, `/usr/local`. Execs cover the program binds only: `/usr/bin`, `/usr/libexec`, `/usr/local`, `/bin`, `/sbin`. `/usr/include` has no unit, because nothing in the profile compiles C. `/usr/lib64`, `/lib` and `/lib64` have none either, because their macOS image is the system-library element. A hands-element unit may carry less than its element, never more. The check confirms each toolchain unit against the host-toolchain sources; the ledger anchor inputs row below fixes that set as the `HOST_TOOLCHAIN_BINDS` item rather than a rendered argv. "Exact-target process-exec units" was wrong for four subpath units. It is replaced by the explicit seven process units that the child-spawn prohibition reads against. The narrowing changes the candidate, and its startup is pending native Gate A. | `seatbelt-execution`: The toolchain baseline is no wider than the binds it names; The candidate's process authority is seven named units |
| Toolchain respelling vs the exact bind check (clarify, `d63cddf`) | **Adopt option 1: toolchain units never respell.** `d63cddf` required a toolchain unit to target exactly its `--ro-bind-try` source and also let a correction replace it with another resolved spelling. Both cannot hold. Keeping the exact check alone left the clause dead. Relabelling the unit as a non-toolchain element escaped the `box_argv` check, and "no wider than the element" had no mechanical test, which is fail-open. A toolchain denial under another spelling now keeps the unit's bind target, fails the cell and records the startup residual `SEATBELT-R3-STARTUP-toolchain-respelling`. Changing it needs a focused proposed decision. Option 2, a typed same-object spelling per toolchain entry, is refuted. The check cannot prove two spellings name one object without the host, so it would have to trust a recorded path. The probe also names no consumer for `/usr/local`, the bind current macOS firmlinks into the data volume, so the residual costs this probe no named need. The relabelling escape is closed by rules the check can decide. Each hands-element entry names one element of a closed set: toolchain, system library, writable worktree, device set or shell. Each is tested against its anchor. Only a toolchain unit may cover a `box_argv` bind source. No unit of either half may target `/System/Volumes/Data`, a bind's data-volume spelling or a path under one, and no `subpath` may contain it. (That closure held only for data-volume spellings. The `48b6f9d` row below extends the cover rule to each source's `/private` spelling and the data-volume rule to every path under the volume.) Filtered baseline units use `literal` or `subpath`. The system-library element alone keeps a correction. The correction is typed with the unit it replaces, the resolved spelling and the evidence, and it may not equal or contain a withdrawn or narrowed fa7 target. The candidate's units are unchanged. | `seatbelt-execution`: A respelled toolchain unit fails however it is recorded; A bind's image cannot be relabelled out of the toolchain check; A system-library correction is typed and bounded |
| Ledger anchor inputs (clarify, `a66b559`) | **Adopt a named item and typed, validated inputs.** The anchors read the `--ro-bind-try` sources `box_argv` renders, but `box_argv` also renders each declared `ro` bind, home-expanded, and the git common `config`. So `bundles/self`'s `~/.rustup` could anchor a "toolchain unit", and the verdict varied with the host's home and `GitFacts`. The host-toolchain list becomes one public `hands.rs` item, `HOST_TOOLCHAIN_BINDS`. `box_argv` iterates it with a byte-identical namespace argv, and the check reads it, never a rendered argv. That keeps `design.md`'s refusal of the argv as a policy oracle. A test binds the two: the rendered `--ro-bind-try` sources are exactly the item, the home-expanded declared `ro` binds and `<common>/config`. The named minimal invocation (no binds, no common directory, no bundle root) is refuted. It would still read an argv, depend on scratch paths and home-derived defaults, and let a future default bind enter the anchor silently. Declared binds and the common `config` are never toolchain binds. The check takes no `HandsSpec`, home or `GitFacts`, so its verdict cannot vary with them. The concrete cell root, payload root, inputs directory, helper and path-valued denial-control targets are typed inputs, validated before normalization: the layout `<cell-root>/{payload,inputs}`, absolute canonical spelling with no `/var`, `/tmp` or `/etc` symlink spelling, a cell root that is not `/` and is disjoint from every host-toolchain source, system-library target, device literal, control target and `/System/Volumes/Data`, and a helper disjoint from the cell root. The cover and data-volume rules, and a new rule that no unit covers a control target, read each unit's concrete target as well, so a placeholder hides nothing. The observer, not the string check, proves exclusive fresh creation, ownership and canonicalization equality, and the probe's silent uncanonical fallback is withdrawn. The candidate's rule units do not change. | `seatbelt-execution`: A declared read-only bind is not a toolchain bind; The git common config is not a toolchain bind; A git common directory under the payload root leaves the worktree unit valid; The check's verdict does not depend on HandsSpec, home or GitFacts; A mis-instantiated cell root fails before normalization; The payload and inputs layout is fixed; A non-canonical cell root or helper fails; A placeholder does not hide its concrete target; A unit that covers a denial-control target fails; The observer establishes what the string check cannot |
| Host-toolchain `/private` spelling (clarify, `48b6f9d`) | **Adopt a fixed spelling set per source and a whole-volume data rule.** `48b6f9d` required the cell root and helper in their `/private` spelling and listed control targets in both spellings. It compared host-toolchain sources only directly. For the six `/etc` sources in `HOST_TOOLCHAIN_BINDS` that is fail-open. The cell root `/private/etc/ssl/brokkr-cell` passed validation and received the payload write, and a non-toolchain `(subpath "/private/etc/ssl")` or `(literal "/private/etc/ld.so.cache")` passed the cover rule. That contradicted the no-respelling ruling and the requirement that protections hold through macOS path aliases. Each source now has a spelling set from one fixed, host-independent map: the direct spelling, plus the `/private`-prefixed spelling when the first component is `etc`, `var` or `tmp`. The canonical-spelling rule and the control-target list use the same map. Cell-root validation and the cover rule, in normalized and concrete form, compare with every spelling, and a refusal names the source and the spelling it matched. The toolchain anchor still reads only the direct spelling, so no grant widens and no toolchain unit changes. An `/etc` toolchain unit would match nothing on macOS, and admitting its `/private` spelling waits for the focused decision under `SEATBELT-R3-STARTUP-toolchain-respelling`. The data-volume rule had two readings: "`/System/Volumes/Data` joined with that source" gave `/System/Volumes/Data/etc/ssl`, not the real `/System/Volumes/Data/private/etc/ssl`, while the refusal list read as any path under the volume. It now refuses every unit on or under `/System/Volumes/Data` and every `subpath` that contains it, in both places. That subsumes each source's data-volume spelling, matches the cell-root clause and makes the helper scenario hold on its only reading. Excluding the `/etc` sources from the comparison is refuted: no reason permits a cell root or a non-toolchain grant under a toolchain bind's resolved image. Extending helper validation to toolchain and control targets is not adopted either. It would contradict the settled scenario in which such a helper passes validation and the concrete cover rule refuses its units. The candidate's rule units, the settled answers and the namespace argv are unchanged. | `seatbelt-execution`: A host-toolchain source is compared in its /private spelling; A toolchain source's /private spelling cannot enter under another class; Nothing on or under the data volume enters under any class |
| Historical template authority | **Withdraw or narrow; never grandfather.** No 0043 element justifies the unfiltered process family, self-signal, `/Library`, host `/private/tmp`, the whole cell root, `sysctl-read` or `ipc-posix-shm`. `/System` also contains `/System/Volumes/Data`. Each is narrowed to a justified unit or withdrawn, and a withdrawn unit re-enters only through a labelled restoration diagnostic, native single-object attribution and its own removal control. A `/System/Volumes/Data` credential-read denial control is added. | `seatbelt-execution`: A withdrawn unit returns only through the bounded experiment; The data-volume spelling of a credential stays denied |
| Probe measurement integrity | **Adopt every controller finding.** The negative control performs a real original-process-group kill without depending on the guard FIFO; guard liveness is sampled before unregister; peer registration is synchronized before an attempted attack; FIFO opening is nonblocking and bounded; killed holders are waited/reaped on all exits; each obligation has its own trigger; and guard/quiescence evidence is outside payload-writable state and covers every observed identity. | `seatbelt-execution`: The lifetime probe measures independent facts |
| R4 — hooks view and peer status | **Adopt conditionally.** Denied host hooks plus an empty private hooks directory may qualify as full peer only after independent raw hook/config/routing write protection passes native primary and linked-worktree adversaries. | `seatbelt-execution`: Private hooks satisfy the accepted view only with independent protection |
| R5 — system launcher | **Retain.** Only the literal trusted `/usr/bin/sandbox-exec` and a bounded real allow/deny probe establish launcher readiness; lookalikes never execute. | `boundary-availability`: The system pin ignores an earlier lookalike |
| R6 — exact coverage seam | **Retain.** Shared Rust decisions compile on Linux behind injected host/process facts; this is logical coverage, not Darwin enforcement. | `seatbelt-execution`: Linux exercises both policy arms without claiming native enforcement |
| R7 — init warning | **Retain.** macOS advice names namespace, Seatbelt's actual activation status and doctor, plus the explicit unboxed harness alternative. | `boundary-availability`: init on macOS describes the available Seatbelt road |
| Current-main gate scenario | **Adopt.** The modified gate requirement carries forward `An open work-class chain site asks no fragment`; slice II does not erase the accepted current capability while adding the startup fence. | `gate-boundary-policy`: An open work-class chain site asks no fragment |

### D1 — one generic private-overlay locator contract

Before the first payload for a seat attempt, each declared overlay is copied
to private storage without host hard links. `BROKKR_HANDS_OVERLAYS` is
ordered by the original `hands.binds` array; each overlay entry contains
exactly its zero-based original-array `index`, expanded absolute
`declared_path` and absolute `locator`, non-overlays are omitted, and no
overlays yields `[]`. The variable is engine-owned; the same mapping is
supplied to every call in that seat, and a new seat or attempt receives
different locators. The declared path, mode and masks remain
the identity inputs already pinned by the manifest; the ephemeral variable
value, scratch root and locator do not enter portable identity.

`CARGO_HOME` and `NPM_CONFIG_CACHE` point to their matching locator when
their declared bind is present. They do not replace the generic map. A
duplicate declared path, ambiguous well-known redirect, escaping link,
external hard-link alias, race or incoherent snapshot refuses before user
code. Internal links may be recreated only when their fully resolved targets
remain inside the same admitted snapshot. Masked targets and every admitted
alias are omitted from readable snapshot data and denied by policy. The
original overlay source is not granted merely because its locator is.

### D2 — denied-read masks are the Seatbelt observable

Opening a present masked entry under Seatbelt, by its admitted path or alias,
must fail with `EACCES` or `EPERM` and return no bytes. An absent mask stays
absent. Reads of ordinary neighbors remain usable, while read, unlink,
rewrite, rename and replacement attempts leave host bytes and directory
entries unchanged. This does not rewrite namespace's `/dev/null` behavior
and must never be documented as an empty read.

### D3 — native lifetime feasibility gates dependent implementation

The first implementation work is a bounded macOS spike of the
**per-invocation transient launchd lease pair** in an unprivileged per-user
bootstrap domain. The spike contains only the pair, native adversary and
observer helpers, and the minimum experimental policy needed to keep the guard
outside payload authority. One uniquely labelled payload job launches the
literal system `/usr/bin/sandbox-exec`; its job/process coalition is the
candidate containment domain. A second, separately launchd-owned guard job
observes a private
engine-liveness channel, inherits neither payload stdio nor payload-writable
state, and is not a member of the payload job it must terminate. Timeout and
cancellation ask the guard to `bootout` the payload job; liveness EOF triggers
the same path after abrupt supervisor death. The guard must establish payload
quiescence before private-state cleanup and then unregister itself. The spike
must show that ordinary children work and that `setsid` plus double-fork
descendants stop after timeout, cancellation and supervisor `SIGKILL`, with an
independently observed heartbeat quiet for one second and both transient job
labels absent.

Startup is a separate prerequisite. The exact profile and payload executable
must first pass an outside-sandbox control, a direct sandbox control and then
a launchd-owned control, each reporting an externally observed ready token and
ordinary-child identity. The preferred payload is a purpose-built native Rust
helper so interpreter startup is not confounded with lifetime. If an
interpreter is retained, each additional filesystem, IPC or service allowance
must be justified by staged differential controls or denial/system-log
evidence and kept no broader than the named dependency; denial controls must
still pass. A successful `allow default` diagnostic says only that the exact
restrictive profile withheld some required authority. It never supplies a
candidate profile, identifies the missing authority or admits a lifetime case.
A launchd registration, run count, crash count or missing heartbeat
does not establish that payload code ran.

Every launchd control uses a unique label and cell root, verifies the label is
absent before bootstrap, explicitly starts the job when bootstrap does not
itself prove execution, and captures the raw status and bounded output of
bootstrap, kickstart, print, payload exit and bootout. It observes the exact job
loaded, `READY`, ordered stages, ordinary child, terminal state, each available
run/crash/exit field and label absence as distinct facts. A missing field is
unknown, not a synthesized exit and not permission to discard the other facts;
a label must be absent after cleanup before the next cell starts. A stale label
or registration collision fails Gate A and is diagnosed before another native
candidate is dispatched. The required terminal facts are `state = not running`,
`runs` and a numeric `last exit code`, and the cell's exit is that printed code.
`successive crashes` is optional because macOS omits it from terminal jobs that
exited 0 and 2. When absent it stays unknown with its raw sample and is never
treated as zero; when present, a nonzero count fails. No launchd fact replaces
the helper's nonce-authenticated `READY`, stages and ordinary child. Each fact
comes only from the job's top-level dictionary. The fa7 prints repeat `state`
and `active count` inside the resource and jetsam coalition blocks, where they
read `active` and 1 even for S3's never-spawned child. Those nested entries
are raw evidence, never job, liveness or survivor facts. A missing top-level
key stays unknown despite a nested copy, and a duplicated top-level key is
unknown and fails the cell.

The helper records its ordinary-child stage as stream setup for each standard
stream, spawn/exec and child observed, each with its OS error. A Seatbelt child
spawn that fails while the unboxed control succeeds is discriminated under the
exact candidate by four cells: a no-spawn write-only `/dev/null` open, a spawn
with inherited or pre-opened stdio, a spawn with null stdio, and one literal
`(allow file-write-data (literal "/dev/null"))` diagnostic. None of these cells
passes startup. If none attributes the refusal, sub-stage and denial evidence
must name the exec-side operation and target before any predicate is proposed.
An admitted child-spawn predicate is literal-scoped and has its own removal
control, and every denial control reruns on the resulting candidate. Each
Seatbelt cell that reaches `READY` carries an observed blocking removal
control for every diagnosis-admitted predicate, and an empty record fails that
cell. A cell without `READY` owes no removal verdict. Each removal replays the
cell's own profile through direct `sandbox-exec`.

The experimental policy is one normalized template. Its cell root and payload
root are typed placeholders, instantiated separately for the isolated S1 and
S3 roots. Equality means equal normalized rules and substitutions in the same
positions, not equal concrete bytes or digests; every concrete profile is
still retained and hashed. Diagnosis first captures native Seatbelt denial
events with operation, target and responsible process. If those events are
unavailable or insufficient, committed minimal helpers bracket dynamic-loader,
pre-main, filesystem, child-spawn and clean-exit stages, and bounded monotonic
combinations determine the minimal jointly required predicates. Every proposed
predicate names its operation, narrow target and consumer and is rerun alone
against the justified baseline with credential, host-write, network,
guard and peer denial controls. A broad family or combination is diagnostic
only and never becomes the candidate without this per-predicate evidence.

The justified baseline is the baseline half of one typed startup-rule ledger,
which accounts for every rule of the template. The ledger's unit is one `allow`
form with one operation and at most one filter. The template's grouped
`file-read*` form therefore counts as one unit per filter, and a removal strips
exactly one unit. The grammar is closed. After the frame, every form is a
single-operation `allow` form of simple filters, and the check refuses and
names anything else. Baseline units trace to a 0043 hands element as `hands.rs`
realizes it, an execution input or a named probe-harness need. A hands-element
unit is never wider than its element, and names one element of a closed set
whose anchor the check tests. The toolchain units are one subpath per
`hands.rs` bind, checked against the `HOST_TOOLCHAIN_BINDS` item that
`box_argv` iterates, and they never respell. Declared binds and the git common
`config` are never toolchain binds, so the check's verdict does not vary with
the hands spec, home or git layout. Its concrete roots and helper are typed
inputs, validated before normalization. Only a toolchain unit may cover a
bind source, in its direct or its `/private` spelling, and no unit may target
anything on or under `/System/Volumes/Data`. A toolchain denial under another spelling is a named
startup residual that needs a proposed decision. Reads cover `/usr/bin`,
`/usr/lib`, `/usr/libexec`, `/usr/share`, `/usr/local`, `/bin` and `/sbin`,
and execs cover the program binds among them. `/System/Library` and the OS
cryptex, the helper, the typed inputs and payload roots, and the box's device
reads are baseline too. So is `process-fork`, the only unfiltered unit, and
with the six `process-exec` units it is the whole process authority. The root-inode read is the one
diagnosis-admitted unit, with removal entry `root-inode-read`. An attributed
`/dev/null` write-data literal joins that half with its own removal entry.
Every other fa7 unit is withdrawn or narrowed with a recorded reason. When the
candidate fails before `READY`, each withdrawn unit runs alone as a labelled
restoration diagnostic, and one cell restores all of them to reproduce fa7's
authority. None of these cells admits anything. Seatbelt denial events are
kept for every cell, or recorded unavailable.

Three alternatives were refuted. Labelling the historical template baseline
would claim a justification nobody recorded. Placing every historical rule in
the removal set would prove only necessity, never narrowness: an unfiltered
`sysctl-read` that blocks startup when stripped is still broad authority, and
the non-goals forbid it. Starting from an empty profile would discard
authority the hands policy itself grants, such as executing the helper that
`sandbox-exec` must run. `/dev/null` write-data stays diagnosis-admitted, the
stricter class the child-spawn answer ruled, even though the box's device set
could name it as baseline.

This is a hypothesis, not evidence. If launchd exposes only process-group
cleanup, requires private SPI, a privileged entitlement or global mutation,
permits the payload to signal or impersonate the guard or register an escape
job, or if any descendant survives, SEATBELT-R3 remains open and full
implementation stops. An observer outside both jobs may use PID identities,
heartbeats or `kqueue` to measure the experiment, but those observations are
not containment. A mock, Linux run, source argument, launcher smoke test or
original-group kill cannot pass.

The controller's measurement defects are specification failures, not
incidental cleanup details. The group-only negative control has no guard or
liveness-FIFO dependency, actually sends `SIGKILL` to the original process
group, and observes the detached identity continuing before cleanup. The
observer samples guard liveness after payload teardown and before unregister,
and the peer job reaches an observed ready state before the payload attempts
its attack. Cancellation, retained-pipe and parent-exit each execute their
distinct trigger. FIFO setup is nonblocking and bounded; every killed holder
is waited and reaped on success and error. Payload-writable heartbeats may
prove activity but may not attest guard survival, quiescence, cleanup ordering
or the complete survivor set; those facts come from the outside observer and
guard-private state. Harness cleanup happens only after verdict facts are
captured and is labeled cleanup, never containment.

### D4 — private hooks are conditional full-peer behavior

Ordinary Git is routed to an empty seat-private hooks directory and original
host hooks are denied. Routing is not the security boundary. Separate profile
enforcement must protect existing and absent hooks, `config`,
`config.worktree`, includes, effective `core.hooksPath`, and
gitdir/commondir routing against direct, alias, unlink, rename and replacement
attacks. Full-peer status follows only after both hands paths pass those
adversaries in primary and linked worktrees and later host Git executes no
planted sentinel. No harness-grade Seatbelt branch is authorized.

### D5 — evidence, activation and ownership

SEATBELT-R1 through R4 remain open evidence residuals. Launcher readiness,
code existence and accepted semantics close none of them. Seatbelt remains
unbuilt until both hands paths implement the whole policy and qualifying
native macOS evidence closes every activation residual. The R3 feasibility
gate precedes dependent profile, overlay, mask and Git implementation.

Each native result records candidate commit, macOS version/architecture,
system launcher, exact command, scenario names/counts, positive controls,
descendant identities where applicable, exit status and durable log/CI link.
Missing tool, an existing outer box, zero selected tests, skips and failures
are failures. The Linux controller can prepare shared policy tests but cannot
produce Mac enforcement evidence. Controller-owned exact coverage, remote CI,
publication, integration and closure remain pending until real results exist.

Native CI `34433461814` is retained as failed startup evidence for candidate
`6a19a6f4ab9bd30b47537de1a649949cd1099d01` on macOS 26.6.2 arm64. It closes
no lifetime or activation residual: direct sandboxed Python exited by
`SIGABRT`, each launchd payload showed one crashed run, and no heartbeat
preceded a trigger. The next admissible measurement first diagnoses and
demonstrates outside-sandbox, direct-sandbox and launchd startup with the same
helper and bounded policy, then runs the repaired obligation-specific lifetime
matrix. Git metadata and `/usr/include` setup repairs already present in the
runner are accepted prerequisites and are not reopened by this specification.

Native CI `34441725835` is retained as failed startup and probe-integrity
evidence for candidate `8c53dcecaef414938b3abfb8911a77d9ec958f23` on the
GitHub `macos-latest` arm64 runner. It proves the committed Rust helper works
unboxed and that the exact-profile direct path aborts with signal 6 before
`READY`; its passing broad diagnostic remains non-admitting. Its launchd facts
are not coherent enough to establish either launchd-owned control, and Gate B
was not run. The next candidate first repairs unique-label lifecycle
measurement and captures the host version, then bisects only named profile
authorities with denial controls intact. The same revision's Windows
`getuid`/`getpgid` link failure is a portability defect to repair before the
workspace suite can be evidence; it says nothing about native containment.

Native CI `34449331270` is retained as failed Gate A evidence for candidate
`9f4c2c944cac217ccb8dc055971cc62614313ed4` on the GitHub
`macos-latest` arm64 runner. The generic macOS and Windows workspaces pass. S0
reaches all exact stages with an ordinary child and clean exit; S1 reaches no
stage and exits by signal 6 under the exact profile; seven single-class
diagnostics also fail, while `allow default` alone starts and remains
non-admitting. S2 yields no parseable not-running launchd state and is an
observation refusal. S3 yields no `READY` or stages and reports not-running,
one run and one crash. Denial controls and Gate B are not observed. Raw digest
inequality between S1 and S3 is not authority drift because their required
private path literals differ. The next candidate must preserve per-fact
launchd observations, compare normalized policy authority, and obtain native
operation/target attribution before altering the exact profile or attempting
lifetime. No unchanged retry and no diagnostic default admission is allowed.

Native CI `34457208029` is retained as a fourth failed Gate A measurement for
candidate `fa7ece587178a46baa66a7310e0546bfb87a0857` on the GitHub
`macos-latest` arm64 runner. It records progress, not a pass. S1 and S3 reach
`executable` under the root-inode candidate. The ordinary-child spawn then fails
with `EPERM`, and the helper exits 2 without `READY`. All one-class diagnostics
fail at the same step, and `allow default` remains non-admitting. Credential
read, host write and loopback bind are observed denied for this candidate only.
No Seatbelt cell reached `READY`, so the root-inode removal was not due and
the predicate remains unproven as load-bearing. S2's printed terminal facts
(`not running`, one run, exit 0) and S3's (exit 2) are real observations that
the parser erased because `successive crashes` was absent. The successor keeps
the candidate's authority and helper otherwise unchanged. It splits and
discriminates the child-spawn step, parses the required launchd facts field by
field with the counter optional, and observes the removal control on every
Seatbelt cell that reaches `READY`. Gate B stays not run until Gate A passes.

Changing an accepted semantic needs a focused decision document with status
`proposed`; only the operator can accept it. Frozen contracts, policy,
reference and fixtures are untouched. A necessary wire change is additive.
A refusal-only or bind-free implementation is not this slice.
