# Design: Seatbelt on macOS — decision 0046 slice II

## Context

This design adopts the existing `boundary-seatbelt-slice-ii` change. The
evidence inventory first committed at `225d2c7` records the absence of native
proof, but that commit is historical rather than the recovery base. The
predecessor's work was preserved at `da12b3c`, the controller probe repairs
continue through `40e2ab8`, `6a19a6f`, and `8c53dce`, and this visit uses the
preserved current HEAD rather than reconstructing any of them. Historical
`225d2c7` is evidence ancestry, not a recovery target. The accepted decision
0046 addendum at `c966ef3` settles R1–R4's semantics, and `7e79b43`
reconciles the proposal and all five capability deltas to that ruling. See
[the proposal](proposal.md#why) for the motivation and
[the residual inventory](evidence-residuals.md) for evidence status. The audit
is historical evidence about its inspected revision; this design does not
rewrite it.

The implementation is still deliberately absent:

- `crates/brokkr-runtime/src/engine.rs::built_boundary` returns `Err("ii")`
  for Seatbelt;
- `crates/brokkr-cli/src/boundary.rs` reports `Offer::Unbuilt`, while its tool
  lookup is only a readiness fact;
- `crates/brokkr-protocol/src/hands.rs` constructs only the namespace path;
- workspace timeout and `DriverProcess` deadline handling kill a direct child,
  not a native lifetime domain containing detached descendants;
- `session_dir("serve")` is owned by one MCP process rather than an engine
  seat attempt; and
- `git_facts` discovers only the common directory and identity, not the Git
  administration graph required for primary and linked worktrees.

The accepted addendum makes private replacement-locator overlay snapshots and
permission-denied masks valid Seatbelt observables. It preserves mandatory
no-surviving-payload behavior and permits private hooks to count as full-peer
behavior only after independent hook/configuration/routing write protection is
demonstrated. Acceptance supplies no enforcement evidence: SEATBELT-R1 through
R4 remain open and Seatbelt remains unbuilt. The five deltas are the
requirements for this design; no Linux, mock, generated-profile, source-level,
or process-group result closes them.

Native CI `34433461814` supplies one real macOS observation for candidate
`6a19a6f`: direct sandboxed Python aborted with `SIGABRT`, the equivalent
launchd job registered but crashed once, and no payload heartbeat advanced.
That is failed startup evidence, not a lifetime result and not the cause of the
abort. Native CI `34441725835` then measured candidate `8c53dce`: the committed
Rust helper passed S0 direct-unboxed, the identical helper and argv aborted on
signal 6 before `READY` in S1 under the exact profile, and the separately
labelled `allow default` diagnostic exited zero. The diagnostic identifies only
the restrictive-profile layer and authorizes no allowance. S2/S3 alternated
between bootstrap error 5 and incomplete launchd facts while their mutable
roots/labels and test selections were not isolated; absent counters were also
converted into invented exit facts. Gate B was correctly not run. The same
candidate failed Windows linking through unconditional `getuid`/`getpgid`
references. The controller log records the GitHub `macos-latest` arm64 runner
but no `sw_vers` value, so no macOS version is inferred.

Native CI `34449331270` then measured candidate `9f4c2c9` after those
repairs. The generic macOS and Windows workspaces passed, and S0 again reached
the exact stages, ordinary child and clean exit. S1 still aborted on signal 6
before the first authenticated stage; all seven single-class diagnostics did
the same, while only the forbidden `allow default` control started. S2 is an
observation/parsing refusal because the adapter found no parseable not-running
shape, not proof that the payload did or did not execute. S3 reached no stage
or `READY` and ended not running with one run and one crash. Its concrete
profile digest differs from S1 because isolated cells have different private
roots; that is a profile-comparison defect, not evidence of authority drift.
The denial controls did not execute and Gate B correctly did not run.

The next executable work is therefore one bounded, materially changed Gate A
candidate that attributes pre-entry startup and preserves lossless launchd
facts, followed by a new exact-head startup run—not lifetime or production
implementation. GitHub macOS CI is available through the controller, so absence
of a local Mac is not a host prerequisite. Production remains Rust under
`crates/`. Frozen contracts, policy, reference material, and evaluator
fixtures remain unchanged unless a measured wire need later requires a new
version beside a frozen file. `container` remains unbuilt for slice III and
`driver.confine` remains retired.

## Goals / Non-Goals

**Goals:**

- Use one real macOS Seatbelt boundary for both workspace MCP commands and
  boxed exec dispatches after, and only after, all activation evidence passes.
- Preserve the hands policy: closed environment, fresh private per-call HOME
  and temporary state, controlled executable/toolchain paths, declared
  filesystem authority, network denied by default, bounded output and
  deadlines, and protected execution inputs.
- Realize arbitrary overlays through stable seat-private replacement locators,
  with cross-call persistence, cross-seat/attempt isolation, later-source
  invisibility, safe link/mask handling, and no host publication.
- Make present Seatbelt masks return `EACCES` or `EPERM` with no content through
  direct and admitted alias paths without changing namespace behavior.
- Keep ordinary unsigned Git usable while independently protecting hooks,
  configuration, includes, routing, and linked-worktree administration.
- Name and prove a public native lifetime mechanism against `setsid`,
  double-fork, cancellation, and supervisor-death adversaries before dependent
  Seatbelt implementation.
- Keep shared planning, composition, refusal, and cleanup decisions compiled
  and exactly covered on Linux while treating Darwin enforcement as separate
  native evidence.

**Non-Goals:**

- No container implementation, boundary vocabulary change, revived
  `driver.confine`, public boundary plugin API, new crate, or release work.
- No Linux namespace, empty-root, PID/IPC/UTS/cgroup, or mount-remapping claim
  for Seatbelt.
- No bind-free, mask-free, Git-free, or harness-grade fallback and no activation
  based on a readiness probe, smoke test, successful commit, or generated SBPL.
- No resident daemon/XPC installation, persistent launchd configuration, sudo,
  private SPI, entitlement, FUSE layer, Git shim, dynamic-loader interception,
  or process-tree walker treated as containment.
- No global provider/configuration change, sibling-worktree mutation, nested
  Brokkr run, publication, push, integration, or merge.

## Decisions

### D1 — One closed activation state owns every production verdict

The runtime's boundary build status is the single authority from which
`built_boundary`, CLI offers/doctor, and run/resume/rerun refusal derive.
Seatbelt stays `unbuilt: ii` throughout probe and implementation work. Readiness
may separately report host and launcher facts but cannot turn that state into
`Offered`. There is no environment flag, hidden option, test feature, or
all-features switch that activates an unproved release binary.

Low-level tests may reach explicitly selected experimental Seatbelt components
while the production fence is closed. They cannot append a successful
Seatbelt run record. D14 owns the canonical activation order and distinguishes
two evidence-bearing revisions. The implementation revision keeps Seatbelt
`unbuilt: ii`; qualifying native evidence must close every activation residual
and host-independent validation must pass on that revision. That evidence
authorizes only creation of a narrow activation candidate by flipping the
single build-state authority. The complete native and host-independent matrix
then reruns on that exact activation revision, including ordinary
run/resume/rerun, gates, records, doctor, and readouts. Only the post-flip pass
authorizes acceptance, integration, or a full-peer claim. Failure leaves or
reinstates the unbuilt verdict and records the exact residual; evidence from
the implementation revision cannot bless a failing activation revision. This
combines the existing `Offer`, doctor, and refusal surfaces with one fail-closed
authority rather than adding a registry.

### D2 — The native candidate is a transient launchd lifetime pair

The named candidate is a **per-invocation transient launchd lease** in an
unprivileged per-user bootstrap domain:

1. a uniquely labelled payload job launches the literal
   `/usr/bin/sandbox-exec` and the payload;
2. a separately launchd-owned Rust lease-guard job observes a private
   engine-liveness channel;
3. normal completion, timeout, explicit cancellation, MCP/server loss, exec
   wrapper loss, or engine/supervisor EOF asks the guard to `bootout` the
   payload job and report the public operation's result over guard-private
   control state;
4. the guard treats successful `bootout` plus payload-job absence as the
   candidate quiescence event but cannot infer it from silence; while the guard
   remains registered, the outside observer corroborates every stable identity
   and the quiet window, then releases cleanup and self-unregistration.

The guard belongs to the per-invocation launchd mechanism and the same user
bootstrap domain, but not to the payload job it must boot out. A guard killed
with that payload could neither handle supervisor death nor establish
quiescence. The `seatbelt-execution` delta now says the same: commit `02082a4`
replaced its earlier one-job/inside-guard wording with the separately owned
two-job pair and added scenarios for guard survival and payload interference.
That answers **SEATBELT-SPEC-LIFETIME-TOPOLOGY** at the specification layer,
so this design adopts the repaired topology. It does not establish that public
launchd can contain detached descendants; only D3's real unboxed macOS probe
can supply that feasibility evidence.

The proposed guard inherits neither payload stdio nor payload-writable state.
Unique labels, owner-only roots/endpoints, and the experimental profile must
prevent payload signalling or impersonation of the guard, cross-invocation
`bootout`, and payload registration of an escape job before this topology can
pass the native probe.

This topology is a hypothesis, not proof that public launchd APIs contain a
`setsid` or double-fork descendant. Apple's launchd documentation describes
`AbandonProcessGroup` in process-group terms and tells launchd jobs not to
daemonize; XNU's coalition implementation does not establish an unprivileged
public membership API. Those facts explain the probe but do not pass it.
Process-group kill, a process walker, PID polling, or `kqueue` may observe the
experiment and clean a failed test, but none is the production guarantee.

The lifecycle is explicit and idempotent:

```text
prepared -> guard-registered -> payload-started -> terminating
         -> payload-quiescent -> private-state-removed -> unregistered
```

If quiescence cannot be established, execution fails and cleanup stops. The
private root is quarantined for an engine-owned reaper that must re-establish
the lease identity and quiescence; it never trusts a stale PID or filename.
This records the recovery invariant already required by the delta; it does not
authorize building a production crash reaper before D3 proves the lifetime
mechanism that such a reaper would have to recover.

### D3 — Native startup and R3 feasibility are separate implementation gates

Before production profiles, overlays, masks, Git protection, or runtime
composition, a standalone macOS probe exercises only the native helper,
lifetime pair, outside observer, and minimum experimental Seatbelt policy
needed to isolate the guard. It runs outside any existing Brokkr/Seatbelt box
and uses disposable state. One committed test-support Rust executable provides
the payload, guard, and supervisor modes; the feasibility path has no
repository-script or general-purpose interpreter dependency.

#### Gate A — admit the exact payload before measuring lifetime

For the exact helper bytes and candidate profile, the probe records a four-cell
startup matrix:

| Cell | Launch owner | Seatbelt | Required observation |
| --- | --- | --- | --- |
| S0 | outside observer | off | helper reaches nonce-authenticated READY, identifies an ordinary child, and exits as directed |
| S1 | outside observer | exact candidate profile | the identical helper and argv reach the same READY and clean exit |
| S2 | transient launchd payload job | off | the launchd-owned helper reaches the same READY and clean exit |
| S3 | transient launchd payload job | exact candidate profile | the identical job used by the lifetime matrix reaches the same READY and clean exit |

S0, S1, and S3 are the specification's three admission stages. S2 is the
bounded differential control that separates launchd startup from its
composition with Seatbelt. All four must pass before any lifetime trigger.
Each cell records exit status or signal, launchd run/crash state where
applicable, helper and profile digests, exact argv, bounded stdout/stderr, and
the external READY observation. Candidate identity also includes the intended
head and actual checked-out SHA, literal executable path, executable bytes,
mode, architecture, nonce protocol, structural argv and resolved dynamic
dependencies; a pull-request merge SHA is never silently reported as the
candidate head. Startup has its own typed verdict; an abort, nonzero exit,
crash-only job state, missing READY, or missing ordinary child marks the
lifetime matrix `not run`. Registration, one crashed run, a still heartbeat,
or a missing PID is never a zero-survivor result.

Native CI `34433461814` on macOS 26.6.2 arm64 is retained as
`SEATBELT-R3-STARTUP` failure evidence for candidate `6a19a6f`: direct
`/usr/bin/sandbox-exec ... /usr/bin/python3` ended on `SIGABRT` with empty
output, the launchd job registered and recorded one successive crash, and no
heartbeat advanced. It establishes neither the cause nor a lifetime verdict.
The committed Rust helper removes the interpreter as a confounder. If S1 or S3
still fails, diagnosis changes one allowance at a time and names its consumer
with positive and negative controls. A temporary `allow default` profile may
be a labelled diagnostic control, never a passing candidate or a source of
production authority. Broad file, Mach/IPC, service, temporary-directory, or
network grants made merely until startup succeeds are rejected. Existing Git
metadata and `/usr/include` runner repairs are prerequisites already fixed,
not blockers to rediscover.

Native CI `34441725835` for `8c53dce` is retained as the successor failed Gate
A result. S0 proves helper digest `e925083d55a9c8b0` can reach authenticated
`READY`, identify an ordinary child, and return zero when run directly
unboxed. S1 proves that the exact-profile composition, digest
`7af92df5b001453c` in the startup selection, aborts the identical helper and
argv on signal 6 before `READY`; it does not identify the denied operation.
The labelled `allow default` side run is diagnostic only. Repeated S2/S3 facts
are inadmissible because the two native test selections reused
`startup/cell`, plist/output paths and launchd state, alternated bootstrap
error 5 with missing run/crash fields, and reported a synthesized `exit 1`.
The log does not record `sw_vers`. Gate B remained not run, and every R3
lifetime property remains open.

Native CI `34449331270` for `9f4c2c9` is the third failed Gate A result.
The generic macOS and Windows suites passed. S0 reached the exact ordered stages
`entry`, `payload-dir`, `executable`, `child`, `ready`,
`return-clean`, identified an ordinary child and exited zero. S1 reached no
stage and died on signal 6 under the exact profile; all seven one-class
diagnostics also died on signal 6, and only the labelled `allow default`
control started. That control authorizes nothing and the missing operation and
target remain unidentified. S2 produced no parseable not-running launchd state,
so it is an observation refusal rather than an execution result. S3 produced no
authenticated stage or `READY`, then reported not running with one run and one
crash. The three denial controls were unobserved because the helper never
started. Gate B correctly remained not run.

The third run also disproves two claims in the previous repair status. Raw S1
and S3 profile digests are not an equality oracle because required cell-private
root substitutions change their concrete bytes. And launchd parsing still
discards separately observable facts when one terminal shape is missing. The
next Gate A candidate therefore repairs attribution and observation before
retrying the native host:

- represent the experimental policy as a structured, versioned template with
  normalized rules and typed `cell_root`, `payload_root`, and
  `helper_path` substitutions. Compare rule identity and placeholder
  positions, retain every concrete profile and digest, and round-trip the same
  serializer that supplies `sandbox-exec`; post-render string replacement is
  not an authority comparison;
- capture a bounded native Seatbelt denial window keyed by timestamps, process
  identity, executable and nonce. Preserve raw unified-log output, collector
  command status, truncation and unavailability. A missing log is unknown, not
  evidence that no denial occurred;
- run `/usr/bin/true` and `/bin/echo` through the identical launcher and
  exact profile as diagnostic system-binary brackets. Pair each with an
  unboxed positive control. Their success localizes the abort but cannot satisfy
  S1 or add authority;
- add committed diagnostic helper modes that distinguish immediate exit,
  pre-main/first-user-instruction, argument collection, executable lookup,
  payload-state open/write, child spawn/wait and clean return. The first marker
  uses an observer-owned inherited endpoint and a raw, allocation-free write
  before argument parsing or filesystem traversal, so an absent marker
  distinguishes pre-main failure from later helper code;
- if denial attribution remains incomplete, test only bounded monotonic
  combinations as labelled search evidence. Each proposed production
  predicate must then be shown necessary and sufficient against the last
  justified baseline, with its operation, narrow target, responsible process
  and consumer named. No broad family, combination or `allow default` result
  enters the candidate;
- make launchd evidence append-only and field-wise. Preserve every command's
  argv, status/signal, bounded stdout/stderr and monotonic time, plus separate
  loaded, PID, run, crash, last-exit, `READY`, stage, child, return-intent,
  bootout and absence observations. An unknown field fails the cell without
  erasing independent facts or synthesizing an exit;
- replace ambient or fixed denial controls with nonce-bearing observer-created
  sentinels: a known-readable nonempty outside secret, a unique absent outside
  write target with an independently writable parent, and an owned loopback
  endpoint. `NotFound`, no attempted operation or missing after-state is
  unknown, not denial; and
- on uncertain bootstrap, terminal or bootout state, seal the failing verdict
  and quarantine the never-reused root with the label, identities, concrete
  inputs, raw commands and cleanup attempts. Later harness cleanup is separately
  labelled and never counts as containment; no recursive deletion or root reuse
  occurs until exact identities and quiescence are established.

Each cell and repeated invocation continues to own a never-reused label, private
root, plist, streams and report. The destructive launchd adapter has one
explicit CI selection, and Unix ABI calls remain target-gated so the shared
model continues to build on Linux and Windows. The identical-helper/argv
comparison remains structural: immutable executable, mode, architecture, nonce
protocol and arguments must match, while typed private-root substitutions may
differ. No unchanged retry of `9f4c2c9`, broad grant, diagnostic result,
missing denial log or parser refusal can pass Gate A.

#### Gate B — measure the transient launchd lease pair

After Gate A passes, a minimal **B0 rejection screen** may answer the cheapest
mechanism question first: whether public `launchctl bootout` of the payload job
actually ends one authenticated `setsid`/double-fork descendant while a second,
independently registered guard remains alive to issue and report that operation.
B0 records non-reusable identities, an advancing heartbeat before teardown, the
public operation result and the independent after-state. Failure rejects D2
immediately and leaves SEATBELT-R3 open. Success only justifies completing B1;
it is not the accepted R3 feasibility proof, cannot authorize production work,
and cannot replace any trigger, transport, peer or cleanup case below.

Only after Gate A passes **and the native adapter derives every Gate B fact
from an observation rather than assigning success constants**, separate cases
exercise ordinary child completion,
timeout, live explicit cancellation, direct-parent normal exit with a
background child, MCP-server death, exec-wrapper death, engine/supervisor
`SIGKILL`, `setsid`, double-fork, ignored signals, retained output, guard
interference, peer `bootout`, and escape-job registration. Each case performs
the trigger it names rather than relabelling one detach routine.

The probe physically separates owner-only observer/guard control state from
payload-writable state and from immutable launch inputs. The payload may report
READY, the descendants it deliberately creates, and a moving heartbeat. It
cannot write the durable event ledger, guard-liveness evidence, peer state,
quiescence, cleanup ordering, helper/profile inputs, or final verdict. The
guard reports its public `bootout` result and waits at a protected barrier.
While the guard is still registered, the outside observer must:

- establish that the ordinary-child positive control works and that each
  adversarial heartbeat advances before its distinct trigger;
- compare every reported PID with a public non-reusable start identity and
  observe the same identities disappear within five seconds;
- observe the heartbeat unchanged for a further second;
- confirm payload-job absence, guard survival, and protected peer state; and
- only then authorize private-state cleanup and guard self-unregistration.

The observer finally proves cleanup followed quiescence and both transient
labels disappeared in order. Complete case-specific lifecycle sequences are
matched exactly; an intended-event prefix is not a pass. Stable identities and
job observations measure the known helper lineage, but they are not a second
containment mechanism. Passing also requires the public launchd
job/process-coalition ownership and `bootout` semantics to be the causal
boundary. A process walker, PID polling, `kqueue`, heartbeat silence, or
harness cleanup cannot substitute for that ownership claim.

The process-group negative control is independent of the guard and liveness
channel. It records an original process group and detached descendant identity,
checks that the target group is neither the observer's nor the CI runner's,
sends a real `SIGKILL` to that group, and proves the detached identity and
heartbeat remain live through the quiet window before explicit harness
cleanup. Control channels use bounded nonblocking or pollable opens; every
holder, helper, supervisor, and child is waited or reaped on every success and
error path. The retained-output case uses the transport production intends to
drain rather than launchd files presented as pipes. Peer and interference
cases synchronize the target to READY and durably record the attempted attack
before observing its denial.

Before Gate B can run, the bounded native adapter must also replace the current
blocking FIFO-open thread with a genuinely nonblocking or pollable open;
record and compare non-reusable start identities rather than accepting any
`ps` row; maintain a complete child/holder ownership-and-wait ledger; execute
the distinct trigger each case names; make the supervisor-loss topology match
the proposed production liveness owner with no leaked writer; exercise the
intended pipe/spool transport in retained-output; derive timestamped lifecycle
events from authenticated or external observations; treat heartbeat quiet only
as corroboration after the causal domain is empty; validate and clean the real
process-group negative control; and give guard/peer attacks READY,
authenticated attempt records, public API outcomes, and independent
after-state. Injected evaluator tests that reject false facts remain useful,
but they are not native evidence that the adapter honestly observed a true
fact.

The probe fails on zero cases, a skip, an outer box, missing/unusable public
facilities, sudo/private-SPI/entitlement/persistent-configuration needs, a
surviving descendant or label, payload interference with the guard, cleanup
before quiescence, an abandoned helper/thread, or evidence lost with the
supervisor. Any failure keeps SEATBELT-R3 open and stops all dependent
implementation. It is recorded as a residual, not translated into a weaker
guarantee.

The existing GitHub macOS runner is the native execution route. The helper is
already committed and works unboxed; the concrete next prerequisite is the
bounded Gate A discriminator and lossless-observation repair above, followed by
a controller-dispatched Gate A run on that materially changed exact head. Its
durable report is preserved even when the test fails. The native probe remains
explicitly selected once, separately from host-independent model tests, so a
generic workspace failure cannot prevent report preservation or accidentally
turn a non-run into green evidence.

### D4 — A small sealed executor shares facts, not Linux semantics

Implementation uses internal sealed Rust plans rather than a public trait or a
generic boundary framework:

```text
PreparedHands
  common parsed HandsSpec + normalized policy facts
  Namespace(existing plan)
  Seatbelt(SeatbeltPlan)

SeatAttemptState
  run/effect/attempt/site/member identity + private root + overlay map

CallState
  fresh HOME/tmp/profile + lifetime lease + child/pipes/outcome
```

At most one private Seatbelt-focused module is added in the existing protocol
crate, with narrow Darwin fact/operation adapters. Existing `HandsSpec`, bind
types, limits, bounded rendering, `Executed`, composition, doctor, and refusal
surfaces are reused. Existing namespace-specific helpers and signatures remain
available for byte-for-byte namespace regression tests; a new internal
boundary dispatcher selects the Seatbelt plan when the CLI/runtime transports
that compiled word.

`box_argv` is not the abstract policy oracle. It embeds `/runtime`, Linux mount
ordering, `/dev/null` masks, and namespace process/network semantics. Common
policy facts are deliberately extracted before two distinct renderers and
lifecycle adapters. Namespace rendering stays byte-identical. Seatbelt never
inherits a claim merely because namespace happens to implement it.

### D5 — The engine owns seat-attempt state

The engine allocates an opaque state handle keyed by run, effect, attempt,
site, and member before composition. It carries that handle through both JSON
and TOML MCP configuration and the boxed exec path. A caller cannot select an
arbitrary host directory. The root is exclusive, owner-only, outside the
worktree, every bind, Git administration paths, and every other seat root, and
is checked through macOS aliases such as `/tmp` and `/private/tmp`.

Fresh per-call HOME, TMPDIR, profile, and pipes belong to `CallState`.
Overlays belong to `SeatAttemptState`, so a second workspace call or
supervised MCP restart in the same attempt retains the map.
Another seat, member, resume, or rerun receives a different root and mapping.
Process-local `session_dir("serve")` cannot be the authority for persistent
state because provider restart/fallback would otherwise reset it.

### D6 — Filesystem authority is normalized before SBPL rendering

Preparation constructs one internal normalized authority plan after home
expansion and alias analysis. This is not a public graph framework or a second
policy language; it is the checked input to the Seatbelt renderer. Its
precedence is:

```text
protected or masked deny > read-only > read-write > default deny
```

The plan includes the worktree, expanded binds, masks, private roots, result
path, Git administration graph, execution inputs, and controlled
system/toolchain paths. Equal and nested paths, symlink aliases, hard-link
identities, `/var`/`/private/var` and `/tmp` aliases, case-folding and Unicode-
normalization collisions, and replaceable ancestors take the strictest
authority or refuse before payload. Declaration order never uncovers a deny.

One data-safe path literal encoder or supported Seatbelt parameter mechanism
renders SBPL. NUL or unrepresentable paths refuse. Profiles are exclusively
created beneath protected call state and are never payload-writable. The
profile is default-deny. Every file, device, Mach-service, Unix-socket,
resolver, SDK, certificate, or other system exception has a named consumer,
a native positive control, and a negative test showing it cannot proxy an
undeclared file, credential, host write, or network operation. Broad home,
developer, temporary, or system-tree grants are not compatibility fallbacks.

### D7 — R1 uses defensive private copies and a generic locator ABI

Before the first payload in an attempt, every `BindMode::Overlay` is copied to
a complete private snapshot. `BROKKR_HANDS_OVERLAYS` is engine-owned compact
JSON, in original `hands.binds` order, with exactly these fields per overlay:

```json
{"index":0,"declared_path":"/expanded/source","locator":"/private/snapshot"}
```

Non-overlays are omitted and no overlays yields `[]`. The mapping remains
stable across calls in one seat attempt and differs across seats/attempts. It
is transport, not authority or portable identity. Manifest identity continues
to pin declared path, mode, and masks. `CARGO_HOME` and `NPM_CONFIG_CACHE` are
derived consumers of matching generic locators, never substitutes for generic
overlay support.

The copier performs a descriptor-relative, no-follow inventory/copy/
revalidation into staging and atomically publishes only a coherent snapshot.
It never opens a masked file; copies bytes without host hard links; recreates
hard-link relationships with private inodes only when all links are internal;
recreates symlinks only when their fully resolved targets remain inside the
same admitted unmasked snapshot; and refuses escaping, external, changing,
special-file, duplicate, ambiguous, or incoherent layouts. Metadata needed by
the shipped Cargo/npm users is named and tested rather than silently dropped.

Cleanup is descriptor-rooted, follows no payload-created link, starts only
after quiescence, and never synchronizes back. Plain copies are chosen over
FUSE/mount overlays and APFS cloning: the extra mechanisms add prerequisites,
and clone optimization is not admitted without later native proof of identical
isolation and metadata behavior. Same-path emulation is neither required nor
claimed under the accepted addendum.

### D8 — R2 combines snapshot exclusion with identity-aware profile denial

A present masked entry and every admitted alias to its host identity are
excluded without reading content and denied by the authority plan at both the
host and snapshot spellings. Either copy exclusion or string-path denial alone
is insufficient. An unsafe alias that cannot be enumerated across admitted
trees causes pre-spawn refusal.

Native cases cover direct and alias open, truncate, unlink, link, symlink,
rename, rename-over, directory replacement, and lockfile-then-rename. Present
mask reads return `EACCES` or `EPERM` and no bytes; an absent mask stays absent;
ordinary neighbors remain usable. Host verification compares bytes, type,
mode, link identity, and parent entries. Namespace's existing `/dev/null`
observable stays unchanged, and guides never call Seatbelt denial an empty
read.

### D9 — R4 uses a complete Git administration graph

Preparation discovers and freezes a bounded graph using a controlled absolute
Git executable, a cleared discovery environment, bounded/NUL-safe output, and
spawn-time revalidation. It includes:

- the worktree `.git` file/directory and routing content;
- the per-worktree Git directory, common directory, and `gitdir`/`commondir`
  links;
- present and absent `config`/`config.worktree` plus lock/rename destinations;
- effective `include`/`includeIf` origins and targets;
- default, relative, and absolute effective `core.hooksPath` destinations;
- hook parents and absent hook names; and
- only object, ref, log, HEAD, index, and lock paths needed by the demonstrated
  ordinary commit.

Recursive, cyclic, conditionally unstable, replaceable, writable-hook, or
otherwise unprovable layouts refuse before spawn. Ordinary Git receives an
empty private hooks directory, seat identity, and unsigned-commit settings.
Those are usability routing only. The normalized profile independently denies
raw reads of host hooks and every write or replacement of host hooks,
configuration, includes, and routing, even after the payload clears environment
entries or invokes `git -c`. It permits immutable configuration/routing reads
and the narrow mutations required for normal Git operation.

Full-peer status requires real primary and linked-worktree cases through both
hands paths: direct writes, config commands, initially absent files, locks,
unlink/recreate, rename/replace, writable aliases, relative/absolute hook
paths, and later host-side Git sentinel checks. A benign unsigned commit is a
positive control, not proof. Custom Git wrappers, patched Git, and dynamic-
loader injection are rejected because they add bypass surfaces without being
the boundary.

### D10 — Immutable execution inputs use protected paths or refuse

Seatbelt has no `/runtime/bundle` mount. Before an exec dispatch, preparation
resolves the exact engine executable and the declaring script directory/helpers
already covered by decision 0048's manifest/spawn pin, revalidates the compiled
file map at the existing spawn boundary, and grants read/execute only to those
controlled absolute paths. The authority plan independently denies writes,
unlink, rename, replacement, and admitted aliases of the engine, bundle,
script, and helper inputs. `{brokkr}` and the bundle-relative script continue
to identify the intended pinned inputs; portable identity remains the declared
files rather than a temporary host spelling.

If an input is beneath a required writable grant, has a replaceable ancestor,
or otherwise cannot retain both worktree usability and input protection, the
exact incompatible layout refuses before spawn. This is the delta's explicit
safe outcome for a worktree containing its bundle. A verified private capsule
is not selected pre-emptively: copying executable metadata and rewriting paths
would add an unmeasured mechanism before the native probe. If post-R3 native
adversaries demonstrate that direct protection cannot serve a required
non-conflicting shipped layout, return to design and add a capsule only with
byte/metadata verification and native positive controls.

Helpers outside decision 0048's existing pin retain its limitation/refusal;
direct protected paths do not widen integrity scope.

### D11 — Environment, network, output, and cancellation are independent

The payload environment is constructed from empty and literal-key-set tested.
It contains only the accepted hands entries, engine-owned box marker and
overlay map, seat Git identity/unsigned settings, and toolchain variables for
declared binds. It inherits no loader, shell-startup, proxy, credential, agent,
arbitrary PATH, or launchd variable. Launch, shell, engine, and discovery
programs use controlled absolute identities; controlled PATH is payload
convenience, not launcher authority.

`network: false` denies IPv4, IPv6, TCP, UDP, listening, and loopback for the
whole payload lifetime domain. `network: true` changes network authority only;
it does not grant filesystem, credential, arbitrary Mach-service, or Unix-
socket access. Native tests use owned endpoints and nonces rather than the
public Internet and explicitly characterize shared macOS loopback rather than
claiming a private Linux network namespace.

Completion means the native payload domain is quiescent, not that its direct
launcher exited. Every exit path requests teardown, proves quiescence, finishes
bounded pipe drains, reports the outcome, and only then cleans private state.
Workspace streams retain the 262,144-byte bound while draining excess.
Oversized exec protocol frames fail explicitly rather than being truncated
into apparently valid JSON. An independent outer watchdog turns stuck
teardown/drain into a failed native case.

MCP cancellation, stdin/server loss, engine stop, `DriverProcess` deadline,
exec-wrapper loss, and OS termination are named controller events. The current
synchronous MCP loop cannot process a cancellation notification while blocked
inside execution, so Seatbelt adds a narrow request supervisor and cancellation
channel. Namespace behavior stays unchanged. The lease guard's liveness EOF
covers wrapper/engine death independently of the dying process.

### D12 — Boundary transport extends existing runtime and CLI seams

`hands serve` and `hands exec` gain explicit internal boundary and attempt-
state transport. Both adapter MCP forms carry the same compiled realm word and
opaque state handle. Omission preserves namespace for existing low-level
callers; explicit `harness`, `open`, `container`, or unknown values refuse
rather than run unconfined. Model harnesses stay outside the boundary with the
engine environment they require, while every workspace command and whole exec
dispatch enters the same Seatbelt planner.

Existing `hands_command`, `compose_site`, site/panel/sequence/dialect shapes,
doctor, init advice, and the three start verbs are extended rather than
replaced. A built Seatbelt requires macOS, the literal trusted
`/usr/bin/sandbox-exec`, a literal `/usr/bin` entry in the supplied PATH,
system ownership/mode/ancestor checks, and the bounded engine-authored
allow/deny readiness probe. Wrong-host checks happen before invoking a planted
lookalike. Low-level launch rechecks readiness because preflight can go stale.
No failure falls back to another boundary or appends a success record.

The readiness probe executes no repository code and closes no R1–R4 residual.
Before activation, doctor reports Seatbelt as not yet built, lists open
residual IDs and the next native action, and reports launcher readiness only as
a separate fact. Container remains unbuilt whatever engine is present.

### D13 — Existing identity and record contracts remain authoritative

No new wire version is currently justified. Slice I already pins and records
the boundary. Temporary profiles, private roots, snapshots, and locator
spellings do not enter portable identity. End-to-end native cases must
still prove manifest, `effect/started`, seat record, export/verification, and
all readouts agree on the boundary that ran, and that an unproved or failed
attempt never emits a successful full-peer marker.

If implementation measures a missing field or changed wire meaning, work
returns to design and adds a new version beside the frozen file with migration
tests. Witness and compose pins move only from measured identity changes, with
their cause recorded. No frozen byte is edited pre-emptively.

### D14 — Evidence is a checked obligation matrix and gates activation order

A checked-in native obligation inventory makes both hands entry points a
dimension of each applicable filesystem, environment, network, overlay, mask,
Git, input, lifetime, output, record, and refusal case. The native target fails
on a missing/unusable launcher, outer box, skipped/ignored/filtered obligation,
zero selected cases, missing positive control, or candidate/evidence revision
mismatch. One test target and a small Rust adversary helper are sufficient; a
new test framework is not.

The dependency order is fixed:

1. keep proposal/specs/design/tasks/evidence coherent and prepare
   host-independent refusal and evidence-model tests while the activation fence
   stays closed;
2. implement only the bounded Gate A discriminator: structured
   template/substitution identity, native denial capture, system-binary
   brackets, a first-instruction marker, append-only field-wise launchd
   evidence, synthetic denial positives and quarantine-on-unknown;
3. commit that materially changed candidate and have the controller run Gate A
   alone on the exact head. Any red or unknown S0–S3 cell records the precise
   residual and returns to step 2 without running lifetime or broadening policy;
4. after Gate A passes, use B0 only as an early rejection screen. If B0 passes,
   repair every remaining B1 adapter fact that is still assigned rather than
   observed and run the complete R3 lifetime matrix; only a truthful B1 pass
   permits dependent implementation;
5. implement the low-level planner, executor, both hands paths and native cases
   behind the closed production fence;
6. close R1–R4 and all remaining native obligations on the implementation
   revision, including complete supported-platform evidence, and pass the
   host-independent validation suite on that same closed-fence revision;
7. flip only the single activation state to create the activation revision,
   then rerun the complete native and host-independent matrix on that exact
   revision, including run/resume/rerun, gates, records, doctor and readouts;
   and
8. run format, clippy, all-features locked workspace tests, self/verify bundle
   compiles, release build where commissioned, and controller/CI exact coverage.

Each native result records candidate commit, macOS version and architecture,
literal launcher metadata, compiler, exact command, selected test names/count,
helper/profile digests, startup and lifetime verdicts, positive controls, exit
status, descendant identities where applicable, and durable logs/CI links. The
existing macOS CI route supplies the immediate Gate A candidate evidence.
Decision 0049 promises the landed Seatbelt boundary on both supported macOS
arm64 and x86_64. Therefore the complete pre-activation and post-activation
matrix must pass on both architectures, or an explicit operator-accepted
narrowing must amend that promise. Evidence never transfers between
architectures; a missing or failed architecture remains a blocking residual.
Remote CI, host exact coverage,
publication, integration, and closure remain controller-owned and pending until
their real results exist.

### Council reconciliation

Both current positions use HEAD `70bdb1b` and native CI `34449331270` at
candidate `9f4c2c9`. The older table below remains historical reconciliation
where it does not conflict with the current evidence. Every current claim is
disposed here before those retained rows:

| Current council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
| Both: generic macOS/Windows and S0 pass; S1 aborts before stages; all seven single-class diagnostics fail; only forbidden `allow default` starts; S2 is an observation refusal; S3 is one non-ready crash; denials and Gate B are unobserved. | **Adopt.** | The controller findings and durable report state exactly those facts. Context and D3 retain them without inferring the denied operation, nonexecution, containment or a lifetime verdict. |
| Robustness: compare structured template identity and retain concrete profiles. Simplicity: normalized-template comparison is the smallest repair because private roots necessarily differ. | **Combine.** | D3 uses one serializer, typed substitution roles, normalized rule identity, concrete bytes/digests and a round-trip check. It rejects both raw-digest equality and dropping profile identity. |
| Robustness: collect native denial evidence and use a complete bounded pre-entry ladder. Simplicity: first use denial capture, system binaries and a first-instruction marker, with at most one later narrow diagnostic. | **Combine the order; reject the numeric cap.** | D3 orders denial capture, `true`/`echo`, raw first-instruction marking and committed staged modes before any combination. Because all seven single-class probes failed and the specification permits bounded joint search, an arbitrary one-predicate cap could leave a jointly required operation unattributed. Combinations remain diagnostic; each promoted predicate still needs operation/target/process/consumer necessity and sufficiency, so no guessed broad allowance is admitted. |
| Robustness: launchd evidence must be append-only, field-wise and retain command outcomes. Simplicity: fix field-wise parsing in place without a retry framework or second launcher. | **Combine.** | D3 preserves every independent command and job fact, leaves unknown fields unknown and failing, and adds no automatic retry or alternative launcher. S2 cannot erase observed facts or become proof of nonexecution. |
| Robustness: uncertain cleanup must quarantine evidence; denial controls need nonce-bearing positive controls. Simplicity: keep the next candidate bounded and stop honestly if diagnosis remains unavailable. | **Adopt both.** | Quarantine and synthetic sentinels repair false evidence paths but add no production architecture. An unavailable denial stream or unresolved abort is recorded as a precise residual, Gate B stays not run, and Seatbelt remains unbuilt. |
| Simplicity: run a minimal B0 `bootout`/detached-child/independent-guard experiment before building the whole matrix. Robustness: B0 must never become a new feasibility gate because only the complete causal matrix satisfies R3. | **Combine as rejection screen only.** | D3 lets B0 reject D2 cheaply after Gate A. A green B0 authorizes only B1 preparation; production remains blocked until the complete timeout/cancellation/supervisor/transport/interference/quiescence matrix passes. |
| Robustness: retain D5–D13 ownership and security decisions. Simplicity: freeze their implementation until startup and lifetime feasibility pass. | **Combine.** | The decisions remain explicit so downstream work cannot invent overlay, mask, Git, input, transport or cancellation semantics. D3/D14 prohibit implementing them before a truthful B1 pass. |
| Both: no sixth capability, new wire field, interpreter, crate, public framework, blind retry or rediscovery of Git-metadata and `/usr/include` repairs. | **Adopt.** | The existing five deltas and private Rust seams cover the accepted semantics; D3 changes only measurement until native feasibility exists. |
| Robustness: require native activation evidence on both supported macOS architectures. Simplicity: exercise the available arm64 runner first and record other architecture gaps. | **Adopt the sequencing from simplicity and the activation gate from robustness.** | The immediate Gate A iteration uses the available arm64 route. Decision 0049 nevertheless promises Seatbelt on macOS arm64 and x86_64, so D14 blocks activation until the complete matrix passes on both or the operator accepts a focused platform narrowing. |
| Both: stale design/tasks/evidence cannot overrule native evidence; the next commit is a bounded changed probe, not production code. | **Adopt.** | This design supersedes the stale pending-run status with CI `34449331270`. The dependent tasks/evidence visit must reopen the disproved claims, append the third run without rewriting history and name the exact repair and redispatch. |

| Council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
| Both native-recovery positions: CI `34433461814` failed payload startup and says nothing about lifetime; replace Python with one committed Rust helper. | **Adopt.** | The log shows direct `SIGABRT`, one crashed launchd run, and no heartbeat. Context and D3 preserve that result as `SEATBELT-R3-STARTUP`, require a native helper, and make every lifetime case not run until startup passes. |
| Robustness: use a four-cell direct/launchd × unboxed/Seatbelt matrix. Simplicity: keep the specification's three-stage gate. | **Combine, retaining the diagnostic cell.** | D3 keeps the three specified admission stages and adds unboxed launchd startup as S2. Requiring all four distinguishes helper, profile, launchd, and composed startup failures at the cost of one bounded control, without widening production policy. |
| Both: separate payload-writable state from guard/observer evidence and sample guard survival before unregister. Robustness adds a protected observer barrier; simplicity keeps the guard minimal. | **Combine.** | D2 and D3 limit the guard to liveness, public `bootout`, protected reporting, cleanup, and self-unregister. The outside observer owns stable-identity, quiet-window, peer, ordering, and final verdict facts and releases cleanup only after corroborating quiescence. |
| Robustness: helper PIDs alone cannot prove an arbitrary empty domain. Simplicity: use `(pid, start identity)` over the deliberately closed helper lineage and do not add coalition/`kqueue` machinery. | **Combine, distinguishing observation from containment.** | D3 requires stable identities for every reported helper descendant and public launchd job/coalition ownership as the causal mechanism. It rejects polling or `kqueue` as containment and adds no second process-discovery mechanism. |
| Both: repair the seven controller findings as measurement defects, with obligation-specific triggers and cleanup after verdict. | **Adopt and strengthen with source evidence.** | D3 requires a real group kill, pre-unregister guard observation, pre-attack peer readiness, bounded nonblocking channels, complete reaping, distinct triggers, payload-inaccessible evidence, and exact case lifecycles rather than accepted prefixes. |
| Robustness: isolate the native target and preserve failure evidence unconditionally. Simplicity: reuse the existing macOS CI route rather than inventing a host or framework. | **Combine.** | D3 and D14 reuse the existing controller-dispatched runner but select the destructive native probe once, apart from model tests, and preserve its durable report even on failure. |
| Simplicity: reuse `HandsSpec`, bind types, limits, `Offer`, engine composition, doctor, records, and readouts. | **Adopt.** | Slice I already owns those public seams; D1, D4 and D12 extend them instead of rebuilding them. |
| Both: avoid a new crate, public trait/plugin system, vocabulary, CLI workflow, or pre-emptive contract. | **Adopt.** | D4 uses sealed internal types and at most one private module; D13 keeps existing wire versions unless measurement proves a gap. |
| Simplicity: keep one private Seatbelt module and a small boundary-selected launcher. | **Adopt with a lifetime constraint.** | D4 uses at most one private module and a dispatcher, but D2 and D5 keep native lifetime and attempt ownership boundary-specific because direct-child/group kill cannot satisfy `setsid`, supervisor death, or overlay persistence. |
| Simplicity: collapse D5–D13 and repeated pre-probe production detail into a short invariants-and-seams section. | **Reject the collapse; adopt the dependency constraint.** | The accepted deltas and design dialect require explicit ownership, mechanism, alternatives, and refusal choices for overlays, masks, Git, inputs, cancellation, transport, and records; removing them would leave downstream tasks to invent security semantics. D3 and D14 nevertheless prohibit implementing any dependent mechanism before the native lifetime probe passes, and observable scenarios remain owned by the deltas. |
| Robustness: separate preparation, seat-attempt state, call state, and teardown. | **Adopt.** | Current `box_argv`, `session_dir`, and child-kill paths conflate lifetimes; D2, D4 and D5 give each required owner. |
| Simplicity: use transient launchd state, not a resident daemon/XPC service. | **Adopt and strengthen.** | D2 chooses two transient jobs so the guard survives payload bootout; D3 requires public unprivileged proof before implementation. |
| Robustness: launchd/process-coalition names are hypotheses until a separate observer proves detach and supervisor-death behavior. | **Adopt.** | D3 includes positive and group-kill negative controls, stable identities, heartbeat quiet, job cleanup, and fail-closed stop conditions. |
| Simplicity: reuse `box_argv`'s policy work. | **Combine in part.** | D4 reuses parsed facts and bounds but rejects the argv as an oracle because it imports Linux mounts, `/dev/null`, `/runtime/bundle`, and namespace semantics. |
| Simplicity: plain-copy overlays; no FUSE/APFS path. Robustness: descriptor-relative coherent copying and engine-owned attempt state. | **Combine.** | D5 and D7 choose a private copy but add the link, mask, TOCTOU, publication, cleanup, and persistence defenses arbitrary overlays require. |
| Simplicity: profile denial plus copy exclusion is enough for masks. Robustness: identity/alias/overlap analysis is also required. | **Combine.** | D6 and D8 use denial/exclusion only after authority and inode analysis; unsafe layouts refuse. No masking filesystem is introduced. |
| Simplicity: avoid a general authority-graph framework. Robustness: normalize aliases, overlaps, and deny precedence before rendering. | **Combine, favoring a focused internal plan.** | D6 retains the required normalization and conflict detector but makes it one private renderer input, not a public graph abstraction or second policy language. |
| Simplicity: preserve only metadata demonstrated by Cargo/npm and refuse unsupported forms. Robustness: never silently lose ACL/xattr/sparse/dataless semantics. | **Combine.** | D7 names and tests metadata required by shipped users; every additional form is preserved or explicitly refused rather than receiving a speculative matrix. |
| Simplicity: private hooks plus profile denial; no Git shim or loader injection. Robustness: discover the complete Git administration graph and narrow allowed mutations. | **Combine.** | D9 treats private routing as convenience and the normalized profile as independent protection, with primary and linked-worktree adversaries. |
| Simplicity: protect and revalidate exact inputs directly, refusing conflicts by default. Robustness: a capsule can avoid mutable input spellings. | **Combine, favoring the smaller first mechanism.** | D10 uses direct read/execute grants plus independent mutation denial and pre-spawn refusal, which the delta explicitly permits. A verified capsule is reconsidered only if post-R3 native evidence shows a required non-conflicting shipped layout cannot work directly. |
| Simplicity: reuse the existing availability table. Robustness: one evidence-gated activation authority must feed every verdict. | **Combine.** | D1 and D12 keep the table/readouts but derive their built state from one runtime authority; readiness remains separate. |
| Both returned positions: implementation-revision evidence authorizes the activation edit; activation-revision evidence authorizes acceptance, and one canonical order must state both verdicts. | **Combine and adopt.** | D1 now names the two evidence-bearing revisions and their distinct verdicts; D14 owns the numbered sequence, including host-independent validation on both revisions, while the migration plan follows it. This removes the circular pre-flip demand without allowing stale evidence to bless the admission change. |
| Simplicity: do not build crash recovery before proving the lease. Robustness: uncertain state must be quarantined and reaped only after identity and quiescence are re-established. | **Combine in dependency order.** | D2 preserves quarantine/recovery invariants required by the delta but explicitly defers any production reaper until D3 proves the native domain. |
| Simplicity: use liveness EOF for controller loss and add an MCP supervisor only if cancellation is observable. Robustness: explicit cancellation cannot remain prose. | **Reject deferring the cancellation seam.** | The delta distinguishes explicit cancellation from server/wrapper loss, and the current synchronous loop cannot observe it while blocked. D11 therefore retains a narrow request supervisor/channel, implemented only after D3 passes. |
| Both: one fail-closed native target can cover both entry paths; missing/zero/skipped cases must fail. | **Adopt and strengthen.** | D14 uses a checked obligation matrix, actual revision/host evidence, and a final post-activation rerun without inventing a bespoke framework. |
| Robustness: require duplicate native evidence on both supported Mac architectures before activation. | **Adopt for activation, sequence after the available arm64 feasibility run.** | Decision 0049 promises boxed boundaries as they land on supported macOS arm64 and x86_64. D14 therefore requires complete evidence on both before activation, unless the operator accepts a focused narrowing; no architecture inherits another's result. |
| Simplicity: accept copy cost and mechanism risk rather than silently weaken guarantees. Robustness: quarantine uncertain state and return infeasibility upstream. | **Adopt.** | D2, D3 and the risks below stop on unproved quiescence. A failed native mechanism becomes a focused residual/proposed decision, never a reduced boundary. |

R1, R2, and conditional R4 are settled, and R3's no-survivor outcome remains
mandatory. **SEATBELT-SPEC-LIFETIME-TOPOLOGY** is resolved by the repaired
`specs/seatbelt-execution/spec.md`: it assigns containment to the payload
job/process coalition and keeps the separately launchd-owned guard alive
through payload `bootout`, quiescence and cleanup. This design adopts that
topology and no longer returns the finding upstream. The repair does not reopen
the accepted R1–R4 policy or provide native proof. D3 remains the empirical
feasibility gate; a native failure requires the exact residual and any focused
`proposed` decision, never a downstream relaxation.

## Risks / Trade-offs

- **[The exact payload cannot start under the bounded profile]** → Run D3
  Gate A first, require operation/target attribution and necessity/sufficiency
  evidence before changing one narrow predicate, preserve all denial controls,
  and keep lifetime marked not run on failure.
- **[Native denial logs are unavailable or incomplete]** → Preserve collector
  status and raw bounded output, then use system-binary and first-instruction
  brackets; absence of a denial record stays unknown and never authorizes a
  guessed allowance.
- **[System controls start while the Rust helper does not]** → Localize the
  defect to helper/runtime startup with committed staged modes; a passing
  `true` or `echo` remains diagnostic rather than admission evidence.
- **[A launchd parser change erases a real fact]** → Store command results and
  job fields append-only, fail unknown fields independently, and retain raw
  text; never infer execution, nonexecution or exit from a missing shape.
- **[Uncertain cleanup destroys evidence or races a survivor]** → Seal the
  failure and quarantine the never-reused root; separately labelled harness
  cleanup cannot become containment evidence.
- **[Public launchd cannot own detached descendants]** → Use B0 only to reject
  the candidate cheaply, then require the complete D3 Gate B matrix before
  all dependent implementation; preserve SEATBELT-R3 and the production refusal
  on any failure.
- **[Payload-writable or harness cleanup evidence creates a false pass]** →
  Separate state roots and roles, record complete verdict facts before cleanup,
  require exact lifecycles, and preserve the report even on CI failure.
- **[The guard can be killed, impersonated, or leave registered state]** → Use
  distinct launchd ownership, private identities/endpoints, interference cases,
  and require both labels to disappear before the probe passes.
- **[Seatbelt matching or clause order uncovers a deny]** → Normalize authority
  independently of profile order and require native mask/Git/overlap tests;
  refuse ambiguous layouts.
- **[Necessary Mach/socket/system allowances become proxies]** → Admit only a
  named consumer with positive and negative controls; an unavoidably broad
  exception blocks activation.
- **[Private snapshot copying is slow or races the source]** → Accept the copy
  cost for correctness, stage atomically, revalidate identities/metadata, and
  refuse an incoherent snapshot; do not optimize with unproved cloning.
- **[Some Git layouts cannot be safely modeled]** → Diagnose the exact graph
  conflict before payload instead of granting the common directory broadly;
  refusal is safe but does not make refusal of all shipped layouts complete.
- **[MCP restart or fallback loses state]** → Make the engine's attempt handle,
  not the MCP process, own overlay state.
- **[A survivor holds pipes or private state]** → Prove domain quiescence before
  drain/cleanup, fail an outer watchdog, and quarantine uncertain state.
- **[Host-independent coverage hides Darwin behavior]** → Keep decisions in
  portable Rust behind injected facts and test all logical arms on Linux, while
  separately requiring real `/usr/bin/sandbox-exec` evidence.
- **[Native evidence is stale or from a different candidate]** → Record exact
  revision/host/case data and rerun the full matrix after the activation flip.
- **[Reused launchd state or invented counters creates a false startup
  verdict]** → Give every cell unique state, preserve raw command facts, fail
  unknown terminal state, and select the launchd adapter once.
- **[A Darwin-only helper breaks another supported host]** → Target-gate each
  ABI operation while keeping the shared evaluator compiled and tested on all
  workspace platforms; portability failure blocks candidate reuse but is not
  macOS containment evidence.
- **[Refactoring regresses namespace/harness/open]** → Retain namespace helpers
  and byte-exact argv tests, plus existing gate, record, pinned-script, and
  container-refusal regressions.

## Migration Plan

1. Keep the accepted two-job topology, the five reconciled deltas and this
   design coherent. Existing Seatbelt realm declarations continue to compile
   and pin the word but refuse before journal writes.
2. Preserve native CI `34449331270` at `9f4c2c9` as the third failed Gate A
   observation. Update the dependent task and residual inventory without
   rewriting the older `6a19a6f` and `8c53dce` evidence or treating S2 as
   execution.
3. Implement one bounded Gate A discriminator with structured profile identity,
   native denial capture, system-binary brackets, first-instruction/staged
   helper modes, append-only launchd observations, synthetic denial controls
   and quarantine-on-unknown. Validate and commit that materially changed probe
   while Seatbelt remains `unbuilt: ii`.
4. Have the controller dispatch Gate A alone on that exact head and preserve the
   complete report. A failed or unknown cell appends the precise startup
   residual and returns to step 3; it never runs lifetime or broadens the
   admitted profile.
5. After Gate A passes, run B0 only as an early mechanism-rejection screen, then
   repair all remaining assigned-success paths and run the complete B1 lifetime
   matrix. Only a truthful B1 pass permits the sealed planner/executor,
   engine-owned attempt state, overlays, masks, Git protection, both hands paths
   and guides to be implemented behind the closed fence.
6. Close all native and host-independent obligations on the closed-fence
   implementation revision. Before activation, obtain the complete required
   matrix on both decision 0049 supported macOS architectures, or an explicit
   operator-accepted narrowing.
7. Flip only the single activation authority, then rerun the complete native and
   host-independent matrix on that exact activation revision. Acceptance,
   integration and full-peer claims remain forbidden until the post-flip
   evidence passes.
8. Refresh witness/compose pins only where measured identity changed. Add a
   contract version only if implementation measures a wire need; never edit a
   frozen version.

No stored-data migration or workspace version bump is presently required.
Omitted low-level boundary transport continues to mean namespace, whose
observable behavior remains unchanged. If a post-probe implementation or
native case fails, rollback is to leave/reinstate the single unbuilt state,
retain durable failure evidence, and remove only engine-owned private state
whose identity and quiescence are established. No persistent service needs
uninstallation. Container remains the separately commissioned slice III.

## Open Questions

There are no unanswered Seatbelt policy questions. The accepted 0046 addendum
settles R1, R2, R3's mandatory outcome and conditional R4; none may be reopened
to explain away a failed probe.

Three empirical questions remain in strict order:

1. Which exact operation and target withheld by the bounded profile causes the
   helper to abort before its first authenticated file stage, and can a
   least-authority predicate be shown necessary and sufficient while every
   denial control still passes?
2. Can the launchd observer preserve a truthful field-wise S2/S3 lifecycle on
   the current macOS format, including command status, partial fields, clean
   terminal evidence and quarantine after uncertainty?
3. Only after both startup questions pass, does the public unprivileged
   per-user launchd payload-job domain causally end all `setsid`/double-fork
   descendants after timeout, cancellation and supervisor death while the
   separately owned guard and protected peers survive?

CI `34449331270` answers none of those positively. It proves the helper works
unboxed, the exact profile still prevents startup, S2 is unparseable and S3 is a
non-ready crashed run. The available arm64 CI route can run the next bounded
candidate; no local Mac is missing. An unavailable denial stream or a Gate A
failure is recorded precisely and leaves lifetime not run. A B0 or B1 failure
rejects the D2 candidate and keeps SEATBELT-R3 open. Any request to weaken
accepted semantics or decision 0049 platform scope returns as a focused
`proposed` decision rather than a downstream workaround.

## Implementation status

Current HEAD `70bdb1b` preserves the probe work through `9f4c2c9` and the
third native report, but contains no production Seatbelt implementation.
Seatbelt remains `unbuilt: ii`; container remains `unbuilt: iii`.

The committed probe has valuable, demonstrated preparation: one portable Rust
helper, a fail-closed four-cell model, exact authenticated S0 stages, isolated
roots and labels, target-gated Unix ABI, a single explicitly selected native
test, and host-independent evaluator tests. Native CI `34449331270` confirms
the Windows portability repair and the generic macOS suite.

The same run invalidates the prior claim that Gate A merely awaited dispatch.
S1 still aborts on signal 6 before any file-backed stage; seven one-class
diagnostics do not discriminate it; only non-admitting `allow default`
starts. S2's all-or-nothing parse loses its terminal observation, S3 is one
non-ready crashed run, raw profile-digest equality is invalid for distinct
private roots, denial controls never execute and Gate B remains not run.
Neither R3 feasibility nor any R1–R4 enforcement residual is closed.

The concrete next implementation unit is only the D3 Gate A discriminator and
lossless observer described above. It must be committed as a materially changed
probe, with dependent tasks and `evidence-residuals.md` reconciled, before the
controller dispatches another exact-head Gate A run. No production planner,
lifetime executor, overlay, mask, Git, runtime transport, activation edit or
full-peer claim is authorized until Gate A and the complete B1 feasibility
matrix pass. Exact coverage, native dispatch, final remote CI, publication,
integration and closure remain controller-owned.
