# Design: Seatbelt on macOS — decision 0046 slice II

## Context

This design adopts the existing `boundary-seatbelt-slice-ii` change. The
evidence inventory committed at `225d2c7` records the absence of native proof,
the accepted decision 0046 addendum at `c966ef3` settles R1–R4's semantics,
and `7e79b43` reconciles the proposal and all five capability deltas to that
ruling. See [the proposal](proposal.md#why) for the motivation and
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

This controller is Linux and has no native Mac evidence. The first executable
Seatbelt work is therefore a bounded native lifetime feasibility probe, not
the full implementation. Production remains Rust under `crates/`. Frozen
contracts, policy, reference material, and evaluator fixtures remain unchanged
unless a measured wire need later requires a new version beside a frozen file.
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
   payload job;
4. the guard establishes payload-domain quiescence before private-state
   cleanup, then unregisters itself.

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

### D3 — Native R3 feasibility is the first implementation gate

Before production profiles, overlays, masks, Git protection, or runtime
composition, a standalone macOS probe exercises only the lifetime pair and the
minimum experimental Seatbelt policy needed to isolate the guard. It runs
outside any existing Brokkr/Seatbelt box and uses disposable state.

The probe has separate cases for ordinary child completion, timeout, explicit
cancellation, direct-parent normal exit with a background child, MCP-server
death, exec-wrapper death, and engine/supervisor `SIGKILL`. Each adversarial
case uses a native helper that calls `setsid`, double-forks, ignores ordinary
termination, records every PID plus a non-reusable start identity, retains
output pipes, and writes a heartbeat. An observer outside both jobs must:

- establish that the ordinary-child positive control works and that the
  adversarial heartbeat advances before the trigger;
- observe every recorded identity disappear within five seconds;
- observe the heartbeat unchanged for a further second;
- observe both transient job labels disappear; and
- prove scratch removal occurred only after payload quiescence.

A negative control must show the same helper surviving an original-process-
group-only kill; otherwise the test did not exercise the required escape. The
probe records candidate revision, `sw_vers`, architecture, launcher metadata,
bootstrap domain, exact command, selected case count, identities, triggers,
exit statuses, cleanup observations, and a durable log/CI link.

The probe fails on zero cases, a skip, an outer box, missing/unusable public
facilities, sudo/private-SPI/entitlement/persistent-configuration needs, a
surviving descendant or label, payload interference with the guard, cleanup
before quiescence, or evidence lost with the supervisor. Any failure keeps
SEATBELT-R3 open and stops all dependent implementation. It is recorded as a
residual, not translated into a weaker guarantee.

The concrete missing prerequisite is a real, unboxed macOS host with
`/usr/bin/sandbox-exec` and public per-user `launchctl` facilities that permit
uniquely labelled transient jobs without installation or privilege. It must
permit an outside observer to kill the supervisor and retain the evidence just
listed. This Linux controller cannot supply that proof.

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
2. commit and run only the standalone R3 native probe;
3. if and only if it passes, implement the low-level planner, executor, both
   hands paths, and native cases behind the closed production fence;
4. close R1–R4 and all remaining native obligations on the implementation
   revision, and pass the host-independent validation suite on that same
   closed-fence revision;
5. flip only the single activation state to create the activation revision,
   then rerun the complete native and host-independent matrix on that exact
   revision, including run/resume/rerun, gates, records, doctor, and readouts;
   and
6. run format, clippy, all-features locked workspace tests, self/verify bundle
   compiles, release build where commissioned, and controller/CI exact coverage.

Each native result records candidate commit, macOS version and architecture,
literal launcher metadata, compiler, exact command, selected test names/count,
positive controls, exit status, descendant identities where applicable, and
durable logs/CI links. The existing macOS runner supplies the required native
candidate evidence. Evidence records the architecture actually exercised;
0049 support remains arm64 and x86_64, and any observed architecture-specific
failure becomes a named residual rather than inheriting another host's result.
Remote CI, host exact coverage, publication, integration, and closure remain
controller-owned and pending until their real results exist.

### Council reconciliation

| Council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
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

- **[Public launchd cannot own detached descendants]** → Run D3 before all
  dependent implementation; preserve SEATBELT-R3 and the production refusal on
  any failure.
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
- **[Refactoring regresses namespace/harness/open]** → Retain namespace helpers
  and byte-exact argv tests, plus existing gate, record, pinned-script, and
  container-refusal regressions.

## Migration Plan

1. Keep the repaired two-job topology in the proposal, `seatbelt-execution`
   delta and this design coherent, and revalidate the five deltas. Existing
   Seatbelt realm declarations continue to compile and pin the word but refuse
   before journal writes.
2. Land the standalone native lifetime probe and obtain D3 evidence on the
   concrete macOS prerequisite. On failure, record the residual and stop; the
   safe rollback is already-active `unbuilt: ii`.
3. After a pass, land the sealed planner/executor, engine-owned attempt state,
   native matrix, both hands paths, guides, and host-independent tests while
   the production activation fence remains closed. Close all native residuals
   and pass host-independent validation on that implementation revision.
4. Only then flip the single activation state to create the activation
   revision and rerun the complete native and host-independent matrix on that
   exact revision. Acceptance, integration, and a full-peer claim remain
   forbidden until that post-flip verification passes.
5. Refresh witness/compose pins only where measured identity changed. Add a
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

One empirical feasibility question gates all subsequent implementation: can
the D2 two-job mechanism, using only public unprivileged per-user launchd
facilities, establish a non-PID-reuse-prone empty payload domain after
`setsid`/double-fork detachment, cancellation, and supervisor `SIGKILL`,
while keeping its guard inaccessible to the payload? This is not an invitation
to choose weaker semantics. It is answered only by D3's real, unboxed macOS
probe. On this Linux controller it remains unmeasured because the required
macOS host is absent. A failure keeps SEATBELT-R3 open, keeps Seatbelt unbuilt,
and returns any necessary semantic change in a focused proposed decision.
