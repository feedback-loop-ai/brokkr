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

The next executable work is therefore a bounded Gate A measurement repair and
profile diagnosis followed by a new exact-head startup run, not the lifetime
probe or full implementation. GitHub macOS CI is available through the
controller, so absence of a local Mac is not a host prerequisite. Production
remains Rust under `crates/`. Frozen contracts,
policy, reference material, and evaluator fixtures remain unchanged unless a
measured wire need later requires a new version beside a frozen file.
`container` remains unbuilt for slice III and `driver.confine` remains retired.

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
the external READY observation. Startup has its own typed verdict; an abort,
nonzero exit, crash-only job state, missing READY, or missing ordinary child
marks the lifetime matrix `not run`. Registration, one crashed run, a still
heartbeat, or a missing PID is never a zero-survivor result.

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

The next Gate A candidate repairs the measurement before it retries the native
host:

- each cell and repeated invocation owns a never-reused label, private root,
  plist, streams, and report record; pre-bootstrap and post-bootout
  `launchctl print` prove exact-label absence, and failed waits or cleanup are
  verdict facts rather than ignored hygiene;
- bootstrap, optional kickstart, loaded state, authenticated helper stages,
  `READY`, ordinary-child identity, payload return intent, terminal state,
  raw `launchctl print`, run/crash counters, bootout, and final absence remain
  separate bounded observations. Unknown or unparsable terminal state stays
  unknown and fails the cell; no counter or payload-authored intent is
  converted into an OS exit;
- the destructive launchd adapter has one explicit CI selection, so generic
  parallel workspace tests cannot run a second launchd sequence against the
  same per-user domain. Host-independent model tests remain in the ordinary
  workspace suite;
- authenticated stage telemetry brackets helper entry, private-directory
  setup, executable identity, ordinary-child spawn/observation/reap, `READY`,
  and return intent. Denial logging is captured when public runner facilities
  permit it. A differential changes one named operation/target allowance at a
  time and records the consumer and all credential, host-write, network,
  guard, and peer denial controls in a typed diagnostic collection that can
  never satisfy a startup cell; and
- all Unix ABI uses in the helper, including `getuid`, `getpgid`, `setsid`,
  and signal operations, are target-gated or have off-target refusing stubs.
  The shared verdict model still compiles and runs on Linux and Windows.

The identical-helper/argv comparison is structural: immutable executable,
mode, nonce protocol, and arguments must match, while a controller-generated
cell root is an explicit typed variable so isolation is not falsely described
as byte-identical absolute argv. No unchanged retry of `8c53dce`, broad grant,
or diagnostic result can pass Gate A.

#### Gate B — measure the transient launchd lease pair

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

The existing GitHub macOS runner is the native execution route. The concrete
next prerequisite is a committed Rust helper plus these measurement repairs,
followed by a controller-dispatched native run whose durable report is
preserved even when the test fails. The native probe is explicitly selected
once, separately from host-independent model tests, so a generic workspace
failure cannot prevent report preservation or accidentally turn a non-run
into green evidence.

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

1. reconcile proposal/specs/design/tasks and prepare host-independent
   refusal/planning tests while the activation fence stays closed;
2. repair helper portability and the isolated Gate A observation harness,
   commit that bounded candidate, and run only the standalone startup gate on
   the exact head;
3. after all four Gate A cells pass, repair any remaining Gate B adapter facts
   that are still asserted rather than observed, then run the R3 lifetime
   probe on the admitted helper/profile candidate;
4. if and only if both gates pass, implement the low-level planner, executor,
   both hands paths, and native cases behind the closed production fence;
5. close R1–R4 and all remaining native obligations on the implementation
   revision, and pass the host-independent validation suite on that same
   closed-fence revision;
6. flip only the single activation state to create the activation revision,
   then rerun the complete native and host-independent matrix on that exact
   revision, including run/resume/rerun, gates, records, doctor, and readouts;
   and
7. run format, clippy, all-features locked workspace tests, self/verify bundle
   compiles, release build where commissioned, and controller/CI exact coverage.

Each native result records candidate commit, macOS version and architecture,
literal launcher metadata, compiler, exact command, selected test names/count,
helper/profile digests, startup and lifetime verdicts, positive controls, exit
status, descendant identities where applicable, and durable logs/CI links. The
existing macOS runner supplies the required native candidate evidence. Evidence
records the architecture actually exercised; 0049 support remains arm64 and
x86_64, and any observed architecture-specific failure becomes a named residual
rather than inheriting another host's result. Remote CI, host exact coverage,
publication, integration, and closure remain controller-owned and pending until
their real results exist.

### Council reconciliation

The current robustness and simplicity positions both use exact-head native CI
`34441725835`; the older rows below retain the still-applicable reconciliation
of the earlier council. Their current claims are reconciled explicitly first:

| Current council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
| Both: S0 passed, S1 aborted under the exact profile, `allow default` is diagnostic only, launchd startup is not established, Gate B did not run, and the Windows helper link failed. | **Adopt.** | Controller findings and log lines 1114–1267 show those outcomes. Context and D3 record them without inferring a macOS version, denied operation, launchd verdict, or lifetime property. |
| Robustness: repair cell isolation, preserve raw lifecycle facts, and never synthesize terminal state. Simplicity: make the smallest launchd measurement repair and remove the double selection/race. | **Combine.** | D3 requires unique roots/labels and pre/post absence, separates bootstrap/print/exit/bootout facts, retains unknown as failure, and selects one native launchd driver. It does not add retries or a new job framework. |
| Robustness: authenticated helper stages and typed one-authority diagnostics are needed. Simplicity: diagnose one named allowance at a time and shrink back from the broad control. | **Combine.** | D3 adds bounded stage telemetry and a diagnostic-only typed collection. Each differential names its operation, target and consumer and reruns denial controls; `allow default` and broad file/Mach/service/network grants never enter the candidate. |
| Both: repair non-Darwin helper linkage without compiling away the shared model. | **Adopt.** | D3 target-gates every Unix ABI operation or provides refusing stubs, while shared verdict logic remains exercised on Linux and Windows. |
| Robustness: the current Gate B adapter contains false-pass paths that must be repaired. Simplicity: do not expand Gate B machinery before startup passes. | **Combine in dependency order.** | No Gate B work or execution precedes a passing exact-head Gate A. After Gate A, D3 requires the named adapter facts to become real observations before Gate B runs; evaluator mutation tests alone cannot establish native truth. |
| Robustness: preserve explicit production ownership and security invariants. Simplicity: defer D5–D13 production work, the crash reaper, capsule, and request supervisor until the feasibility gates justify consumers. | **Combine.** | D2 and D5–D13 remain architectural constraints so later tasks do not invent security semantics, but D3/D14 forbid their implementation before Gate B. D2 defers the reaper, D10 chooses direct protected paths first, and D11's cancellation seam is implemented only after feasibility. |
| Simplicity: do not add a sixth capability, wire field, general interpreter, new crate/framework, or unconditional dual-architecture gate. Robustness: preserve full R1–R4, evidence and activation obligations. | **Adopt both.** | The five deltas and accepted addendum already own the semantics. D4/D13 add no public framework or wire change; D14 records the architecture actually tested and blocks on observed architecture failures without fabricating duplicate evidence. No guarantee is removed. |
| Robustness: stale checked tasks/evidence must not overrule source and native results. Simplicity: the next artifact is one bounded repair commit, not production architecture. | **Adopt.** | This design supersedes its stale “repaired/pending” status. The downstream task/evidence visit must reopen or split the affected repair claims, preserve CI `34441725835`, and define the exact Gate A repair candidate before controller dispatch. |

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
| Robustness: require duplicate native evidence on both supported Mac architectures before activation. | **Reject as an unconditional activation gate.** | The accepted addendum and deltas require the actual host/OS/architecture and the existing native runner, not duplicate full matrices. D14 records the exercised architecture; 0049 still supports both, and any architecture-specific failure becomes a blocking residual rather than inheriting another host's evidence. |
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
  Gate A first, change only a measured named allowance, preserve all denial
  controls, and keep lifetime marked not run on failure.
- **[Public launchd cannot own detached descendants]** → Run D3 Gate B before
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

1. Keep the repaired two-job topology in the proposal, `seatbelt-execution`
   delta and this design coherent, and revalidate the five deltas. Existing
   Seatbelt realm declarations continue to compile and pin the word but refuse
   before journal writes.
2. Preserve native CI `34441725835` as failed Gate A evidence. Repair helper
   portability, unique launchd cell ownership, raw terminal observation,
   single native selection, and typed one-authority diagnostics in one bounded
   candidate. Dispatch only Gate A on that exact head. On failure, preserve
   the report, record the precise startup residual and stop with Gate B not
   run; the safe state is already-active `unbuilt: ii`.
3. After Gate A passes, repair any remaining Gate B false-pass paths named in
   D3 and run the lifetime matrix on the admitted helper/profile candidate. A
   failure preserves SEATBELT-R3 and stops dependent implementation.
4. After both gates pass, land the sealed planner/executor, engine-owned
   attempt state, native matrix, both hands paths, guides, and host-independent
   tests while
   the production activation fence remains closed. Close all native residuals
   and pass host-independent validation on that implementation revision.
5. Only then flip the single activation state to create the activation
   revision and rerun the complete native and host-independent matrix on that
   exact revision. Acceptance, integration, and a full-peer claim remain
   forbidden until that post-flip verification passes.
6. Refresh witness/compose pins only where measured identity changed. Add a
   contract version only if implementation measured a wire need. Do not edit
   frozen versions.

No stored-data migration or workspace version bump is presently required.
Omitted low-level boundary transport continues to mean namespace, whose
observable behavior remains unchanged. If a post-probe implementation or
native case fails, rollback is to leave/reinstate the single unbuilt state,
retain durable failure evidence, and remove only engine-owned private state
whose identity and quiescence are established. No persistent service needs
uninstallation. Container remains the separately commissioned slice III.

## Open Questions

There are no unanswered Seatbelt policy questions. The accepted 0046 addendum
settles R1, R2, R3's mandatory outcome, and conditional R4; the returned
clarification found no remaining semantic ambiguity.

Two empirical questions remain in strict order: after repairing the known
measurement and portability defects, can the committed native helper reach
authenticated READY and a real clean terminal state in every isolated D3 Gate
A cell under the bounded profile, and, only then, can the D2 two-job mechanism
use public unprivileged
per-user launchd facilities to end every `setsid`/double-fork descendant after
cancellation or supervisor `SIGKILL` while keeping its guard inaccessible to
the payload? CI `34433461814` answers only that the former Python payload did
not start; CI `34441725835` answers S0, fails S1, leaves S2/S3 unmeasured, and
does not answer lifetime.

These are not invitations to choose weaker semantics. The existing GitHub
macOS runner can answer them after the known Gate A defects are repaired; a
missing local Mac is not the blocker. A Gate A failure records a startup
residual and leaves
lifetime not run; a Gate B failure keeps SEATBELT-R3 open. Either keeps
Seatbelt unbuilt and returns any necessary semantic change in a focused
proposed decision.

## Implementation status

The committed `seatbelt-probe-helper` bin and fail-closed S0–S3 evaluator are
useful preparation, and native CI `34441725835` proved S0. That same run
invalidated the earlier “measurement repaired/native pending” status: S1 still
aborts, S2/S3 reuse state and yield order-dependent or unknown launchd facts,
the adapter synthesizes terminal results, two native selections can race, and
the helper fails Windows linkage. Gate B correctly remained not run. The
downstream task and evidence artifacts must therefore reopen or split their
affected checked claims rather than treating checkmarks as stronger evidence.
No production Seatbelt work is authorized. SEATBELT-R1–R4 remain open and
Seatbelt stays `unbuilt: ii` until a repaired exact-head Gate A passes, a
truthful Gate B passes, and the later integrated obligations pass through both
hands entry points.
