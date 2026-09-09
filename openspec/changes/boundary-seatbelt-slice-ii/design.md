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
Seatbelt run record. The activation state changes only after both hands paths
implement the complete policy, qualifying native evidence closes every
activation residual, and the activation revision reruns the required native
and host-independent suites. This combines the existing `Offer`, doctor, and
refusal surfaces with one fail-closed authority rather than adding a registry.

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
quiescence. The current `seatbelt-execution` delta instead says one transient
job is the lifetime domain and places the guard inside it. That earlier
topology is internally inconsistent with cleanup after `bootout`; treating
the shared bootstrap domain as the same domain only here would conceal the
owning-spec fault. **SEATBELT-SPEC-LIFETIME-TOPOLOGY** is therefore returned
upstream: amend that delta to the two-job pair (or another mechanism that keeps
the guard alive through teardown) before this design is adopted.

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
Overlays and execution capsule state belong to `SeatAttemptState`, so a second
workspace call or supervised MCP restart in the same attempt retains the map.
Another seat, member, resume, or rerun receives a different root and mapping.
Process-local `session_dir("serve")` cannot be the authority for persistent
state because provider restart/fallback would otherwise reset it.

### D6 — Filesystem authority is normalized before SBPL rendering

Preparation constructs a canonical authority graph after home expansion and
alias analysis. Its precedence is:

```text
protected or masked deny > read-only > read-write > default deny
```

The graph includes the worktree, expanded binds, masks, private roots,
execution capsule, result path, Git administration graph, and controlled
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
excluded without reading content and denied by the authority graph at both the
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

### D10 — Immutable execution inputs live in a verified capsule

Seatbelt has no `/runtime/bundle` mount. Before an exec dispatch, preparation
copies the exact engine executable and the declaring script directory/helpers
already covered by decision 0048's manifest/spawn pin into a protected private
capsule and verifies their bytes against the compiled file map before
publication. Seatbelt executes capsule paths. `{brokkr}` and the bundle-
relative script argument are rewritten to those paths only for the Seatbelt
dispatch; portable identity continues to name the declared files, not the
ephemeral capsule.

Copying is not protection of the originals. The authority graph independently
denies writes, unlink, rename, replacement, and alias access that could mutate
the original pinned engine, bundle, script, or helper inputs. A layout where
that protection conflicts with required worktree authority refuses before
spawn rather than relying on the capsule to hide a host mutation.

Helpers outside the existing pin retain decision 0048's limitation/refusal;
the capsule does not widen integrity scope. This avoids relying on fragile deny
carve-outs for inputs nested below a writable worktree or replaceable ancestor.

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
the boundary. Temporary profiles, private roots, capsule paths, snapshots, and
locator spellings do not enter portable identity. End-to-end native cases must
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
   revision;
5. flip the single activation state, then rerun the complete native matrix on
   that exact revision, including run/resume/rerun and records/readouts; and
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
| Simplicity: treat Seatbelt as one more argv builder with one shared spawn/wait/kill routine. | **Reject.** | Direct-child/group kill cannot satisfy `setsid` or supervisor death, and process-scoped state cannot preserve overlays. D2, D4, D5 and D11 make lifetime and ownership boundary-specific. |
| Robustness: separate preparation, seat-attempt state, call state, and teardown. | **Adopt.** | Current `box_argv`, `session_dir`, and child-kill paths conflate lifetimes; D2, D4 and D5 give each required owner. |
| Simplicity: use transient launchd state, not a resident daemon/XPC service. | **Adopt and strengthen.** | D2 chooses two transient jobs so the guard survives payload bootout; D3 requires public unprivileged proof before implementation. |
| Robustness: launchd/process-coalition names are hypotheses until a separate observer proves detach and supervisor-death behavior. | **Adopt.** | D3 includes positive and group-kill negative controls, stable identities, heartbeat quiet, job cleanup, and fail-closed stop conditions. |
| Simplicity: reuse `box_argv`'s policy work. | **Combine in part.** | D4 reuses parsed facts and bounds but rejects the argv as an oracle because it imports Linux mounts, `/dev/null`, `/runtime/bundle`, and namespace semantics. |
| Simplicity: plain-copy overlays; no FUSE/APFS path. Robustness: descriptor-relative coherent copying and engine-owned attempt state. | **Combine.** | D5 and D7 choose a private copy but add the link, mask, TOCTOU, publication, cleanup, and persistence defenses arbitrary overlays require. |
| Simplicity: profile denial plus copy exclusion is enough for masks. Robustness: identity/alias/overlap analysis is also required. | **Combine.** | D6 and D8 use denial/exclusion only after authority and inode analysis; unsafe layouts refuse. No masking filesystem is introduced. |
| Simplicity: private hooks plus profile denial; no Git shim or loader injection. Robustness: discover the complete Git administration graph and narrow allowed mutations. | **Combine.** | D9 treats private routing as convenience and the normalized profile as independent protection, with primary and linked-worktree adversaries. |
| Robustness: stage immutable inputs because Seatbelt has no `/runtime/bundle` and writable ancestors can undermine them. | **Adopt.** | D10 creates a verified capsule without widening decision 0048's pin. |
| Simplicity: reuse the existing availability table. Robustness: one evidence-gated activation authority must feed every verdict. | **Combine.** | D1 and D12 keep the table/readouts but derive their built state from one runtime authority; readiness remains separate. |
| Both: one fail-closed native target can cover both entry paths; missing/zero/skipped cases must fail. | **Adopt and strengthen.** | D14 uses a checked obligation matrix, actual revision/host evidence, and a final post-activation rerun without inventing a bespoke framework. |
| Robustness: require duplicate native evidence on both supported Mac architectures before activation. | **Reject as an unconditional activation gate.** | The accepted addendum and deltas require the actual host/OS/architecture and the existing native runner, not duplicate full matrices. D14 records the exercised architecture; 0049 still supports both, and any architecture-specific failure becomes a blocking residual rather than inheriting another host's evidence. |
| Simplicity: accept copy cost and mechanism risk rather than silently weaken guarantees. Robustness: quarantine uncertain state and return infeasibility upstream. | **Adopt.** | D2, D3 and the risks below stop on unproved quiescence. A failed native mechanism becomes a focused residual/proposed decision, never a reduced boundary. |

R1, R2, and conditional R4 are settled, and R3's no-survivor outcome remains
mandatory. However, **SEATBELT-SPEC-LIFETIME-TOPOLOGY** is an upstream design
finding owned by `specs/seatbelt-execution/spec.md`: its one-job/inside-guard
wording cannot supply a guard that survives payload `bootout` to prove
quiescence and clean state. The owning delta must adopt D2's separately owned
guard (or another coherent mechanism) before dependent artifacts can be called
coherent. This design phase therefore returns `upstream`. That correction
does not reopen the accepted R1–R4 policy or provide native proof. After the
delta is corrected, D3 remains the empirical feasibility gate; a native
failure then requires the exact residual and any focused `proposed` decision,
never a downstream relaxation.

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
  not the MCP process, own overlays and capsule state.
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

1. Return **SEATBELT-SPEC-LIFETIME-TOPOLOGY** to the owning
   `seatbelt-execution` delta and amend its one-job/inside-guard wording to the
   coherent lifetime pair, keeping every accepted observable unchanged.
2. Revalidate the five deltas, then land the reconciled design and dependent
   tasks without changing production availability. Existing Seatbelt realm
   declarations continue to compile and pin the word but refuse before journal
   writes.
3. Land the standalone native lifetime probe and obtain D3 evidence on the
   concrete macOS prerequisite. On failure, record the residual and stop; the
   safe rollback is already-active `unbuilt: ii`.
4. After a pass, land the sealed planner/executor, engine-owned attempt state,
   native matrix, both hands paths, guides, and host-independent tests while
   the production activation fence remains closed.
5. Close all native residuals on that candidate. Only then flip the single
   activation state and rerun the complete evidence matrix and repository
   validation on the exact activation revision.
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
