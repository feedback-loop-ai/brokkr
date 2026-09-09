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
a native lifetime candidate and proof gate. Only that bounded feasibility probe
may precede proof; full Seatbelt implementation remains blocked until a real
macOS run demonstrates the mandatory no-survivor guarantee. No Linux, mock,
source-level or process-group-only result is native enforcement evidence.

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
- Before full implementation, probe a transient per-invocation `launchd` job
  using job/process-coalition ownership, `bootout` and an engine-liveness
  guard. It is a named candidate, not a claim: real macOS setsid, double-fork,
  timeout, cancellation and supervisor-SIGKILL adversaries must pass.
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
controller already selected the Codex-led control library. Delivery and
all remote actions remain the controller's; no nested Brokkr run is needed.

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
This visit adopts the committed `boundary-seatbelt-slice-ii` change at
`225d2c7` and answers the returned findings in dependency order. The
accepted 0046 addendum supersedes the old R1, R2 and R4 questions; it does
not manufacture evidence. Only the proposal and five capability deltas are
authored in this specify phase. No workflow runner is invoked.

### Returned findings and current disposition

| Finding | Disposition and reason | Owning delta/scenario |
|---|---|---|
| R1 — overlay and shipped-library deliverable | **Adopt.** Every overlay uses a generic explicit replacement locator to a complete seat-private snapshot; later source changes are invisible, and locators are not portable identity. Cargo/npm redirects consume the generic map. | `seatbelt-execution`: Explicit locators cover arbitrary overlays |
| R2 — readable-empty masks | **Adopt.** Seatbelt uses a permission-class denied read with no content. It is documented as denial; namespace remains unchanged. | `seatbelt-execution`: Present Seatbelt masks deny reads |
| R3 — detached descendants | **Retain.** No surviving payload is mandatory. A transient `launchd` job using job/process-coalition ownership, `bootout` and an engine-liveness guard is the named candidate; only native adversarial proof may establish feasibility. | `seatbelt-execution`: Native lifetime feasibility precedes full implementation |
| R4 — hooks view and peer status | **Adopt conditionally.** Denied host hooks plus an empty private hooks directory may qualify as full peer only after independent raw hook/config/routing write protection passes native primary and linked-worktree adversaries. | `seatbelt-execution`: Private hooks satisfy the accepted view only with independent protection |
| R5 — system launcher | **Retain.** Only the literal trusted `/usr/bin/sandbox-exec` and a bounded real allow/deny probe establish launcher readiness; lookalikes never execute. | `boundary-availability`: The system pin ignores an earlier lookalike |
| R6 — exact coverage seam | **Retain.** Shared Rust decisions compile on Linux behind injected host/process facts; this is logical coverage, not Darwin enforcement. | `seatbelt-execution`: Linux exercises both policy arms without claiming native enforcement |
| R7 — init warning | **Retain.** macOS advice names namespace, Seatbelt's actual activation status and doctor, plus the explicit unboxed harness alternative. | `boundary-availability`: init on macOS describes the available Seatbelt road |

### D1 — one generic private-overlay locator contract

Before the first payload for a seat attempt, each declared overlay is copied
to private storage without host hard links. `BROKKR_HANDS_OVERLAYS` is
ordered by the original `hands.binds` array; each overlay entry contains
exactly its zero-based original-array `index`, `declared_path` and absolute
`locator`, non-overlays are omitted, and no overlays yields `[]`. The variable
is engine-owned; the same mapping is supplied to every call in that seat, and
a new seat or attempt receives different locators. The declared path, mode and masks remain
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

The first implementation work is a bounded macOS spike of a transient
per-invocation `launchd` job whose job/process-coalition ownership is the
candidate containment domain. Timeout and cancellation use `bootout`; an
engine-liveness guard must trigger the same teardown after abrupt supervisor
death. The spike must show that ordinary children work and that `setsid`
plus double-fork descendants stop after timeout, cancellation and supervisor
`SIGKILL`, with an independently observed heartbeat quiet for one second.

This is a hypothesis, not evidence. If launchd exposes only process-group
cleanup, requires private SPI, a privileged entitlement or global mutation,
or any descendant survives, SEATBELT-R3 remains open and full implementation
stops. PID polling and `kqueue` may observe the experiment but cannot be the
guarantee because their watcher dies with the supervisor. A mock, Linux run,
source argument, launcher smoke test or original-group kill cannot pass.

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

Changing an accepted semantic needs a focused decision document with status
`proposed`; only the operator can accept it. Frozen contracts, policy,
reference and fixtures are untouched. A necessary wire change is additive.
A refusal-only or bind-free implementation is not this slice.
