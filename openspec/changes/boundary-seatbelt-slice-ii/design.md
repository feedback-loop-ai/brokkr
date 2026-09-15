# Design: Seatbelt on macOS — decision 0046 slice II

## Context

This design adopts the existing `boundary-seatbelt-slice-ii` change. The
evidence inventory first committed at `225d2c7` records the absence of native
proof, but that commit is historical rather than the recovery base. The
predecessor's work was preserved at `da12b3c`, the controller probe repairs
continue through `40e2ab8`, `6a19a6f`, `8c53dce`, `9f4c2c9` and `fa7ece5`, the
specification visits continue through `d52cbd7`, and this visit uses the
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

Native CI `34457208029` then measured candidate `fa7ece5`, which carried the
root-inode read. S1 and S3 moved past the pre-stage abort to `entry`,
`payload-dir` and `executable`, then the ordinary-child spawn failed with
`EPERM` and the helper exited 2 without `READY`. The credential-read,
host-write and loopback-bind denial controls were observed denied for that
candidate. S2 reached `READY`, the exact stages and an ordinary child. Its raw
terminal print read `state = not running`, `runs = 1` and
`last exit code = 0`; S3's read `runs = 1` and `last exit code = 2`. Neither
print carried `successive crashes`, and the parser refused both whole prints
on that absent counter, erasing the printed facts. No Seatbelt cell reached
`READY`, so no removal control was due, and Gate B correctly did not run.

The specification visits after that run (`098914c` through `d52cbd7`) settle
how the next candidate is built and judged:

- launchd facts come only from a print's top-level dictionary, and the crash
  counter is optional;
- removal controls are due only on cells that reach `READY`;
- the child spawn is split into sub-stages with four discriminating cells;
- every template rule is a rule unit in one typed startup-rule ledger;
- every baseline unit passes a two-part anchor, an exact operation name and a
  target test, and a `process-*` unit of either half outside the seven process
  units fails (`d1d714d`);
- the helper runs and re-executes only at its validated, staged `<helper>`
  spelling, and an exec-side refusal of it under another spelling fails the
  cell as a named residual rather than correcting anything (`d52cbd7`).

A host-independent function checks that ledger over fixed inputs: the
`HOST_TOOLCHAIN_BINDS` item in `hands.rs`, and the observer's validated cell
root, payload root, inputs directory, helper and denial-control targets. The
next executable work is therefore that ledger candidate and its probe repairs,
followed by a new exact-head Gate A run—not lifetime or production
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
head and actual checked-out SHA, literal executable path (the staged
`<helper>`), executable bytes and the build digest they equal, mode,
architecture, nonce protocol, structural argv and resolved dynamic
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
root substitutions change their concrete bytes. And launchd parsing discards
separately observable facts when one terminal shape is missing.

The named cause of the pre-stage abort is now established from the retained
matrix and the public Seatbelt record. The exact candidate profile denied a read
of the filesystem-root inode `/`; macOS `dyld` reads that inode while
initialising a dynamically linked process, so Seatbelt failed closed with
`SIGABRT` before the helper could record its first stage. The seven one-class
diagnostics did not restore startup because none supplied a root-inode
`file-read-data` grant: that profile's `(subpath "/usr")`, `(subpath "/bin")`
and other subpaths do not cover `/`, and `(allow file-read-metadata)` covers only
metadata. The labelled `allow default` control started, which is the positive
contrast for exactly this layer and authorizes nothing.

The `fa7ece5` candidate therefore:

- grants `(allow file-read* (literal "/"))` in the candidate profile as the
  minimal-aperture root read — the root inode only, never `(subpath "/")` — and
  keeps `deny default`, no `mach-lookup` and the payload-only write grant;
- proves the rule is load-bearing by removal: it strips exactly that rule from
  the exact candidate and requires the identical payload to fail closed before a
  Seatbelt startup cell may pass. A removal that still starts, or that cannot be
  observed, fails the cell; the stripped profile never enters the candidate;
- normalizes cross-cell profile identity by substituting the typed private cell
  root with the structural `ROOT_TOKEN` before hashing, so isolated cells
  compare their policy rather than their private path while a real rule change
  still changes the digest; and
- retains a bounded sequence of distinct raw `launchctl print` samples with
  exit status, stdout and stderr, reports the last parseable state when no
  terminal state is reached, and refuses with the last raw sample. A reaped label
  is recorded as reaped and a missing field is never synthesized into an exit.

Native CI `34457208029` for `fa7ece5` is the fourth failed Gate A result and
the first with measured progress. With the root-inode read granted, S1 and S3
reached `entry`, `payload-dir` and `executable`; the ordinary-child spawn then
failed with `EPERM` and the helper exited 2 without `READY`. Every one-class
diagnostic failed identically at the child spawn, none of them granted any
`file-write*`, and only the labelled `allow default` control started. The
credential-read, host-write and loopback-bind denial controls were observed
denied; they describe that candidate only and close none of R1–R4. S2 reached
`READY`, the exact stages and an ordinary child. The raw terminal prints show
S2 at `state = not running`, `runs = 1`, `last exit code = 0` and S3 at
`runs = 1`, `last exit code = 2`, neither with a `successive crashes` line.
The parser refused each whole print on that absent counter and the report
erased the printed facts; that is a measurement defect, not a launchd verdict.
No Seatbelt cell reached `READY`, so no removal control was due and the
root-inode read is not yet proven load-bearing. Gate B correctly did not run.

The contingency list this design carried for the `fa7ece5` retry is now
disposed of. The structured template with typed substitutions is realized by
the ledger renderer below, the bounded denial window by per-cell denial events,
and field-wise launchd observation by the top-level parser. An uncertain
bootstrap, terminal or bootout state fails its Gate A cell as a measurement
failure, and the next cell takes a new label and root; sealed quarantine for a
production reaper stays with D2. System-binary brackets, first-instruction
helper modes, monotonic combination search and observer-created denial
sentinels are not built for the next candidate: fa7 shows the helper executes
its own code through `executable` under a deny-default profile, the existing
stage ladder plus the child sub-stages brackets loader, first write, executable
lookup, child spawn and clean exit, and the restoration cells below
discriminate a regression that the narrower ledger might introduce. They return
only if denial events and restoration cells both fail to attribute a refusal,
and then only through the specification's bounded rules.

Each cell and repeated invocation continues to own a never-reused label, private
root, plist, streams and report. The destructive launchd adapter has one
explicit CI selection, and Unix ABI calls remain target-gated so the shared
model continues to build on Linux and Windows. The identical-helper/argv
comparison remains structural: immutable executable, mode, architecture, nonce
protocol and arguments must match, while typed private-root substitutions may
differ. No unchanged retry of `9f4c2c9` or `fa7ece5`, broad grant, diagnostic
result, missing denial log or parser refusal can pass Gate A.

#### The next Gate A candidate is an audited rule ledger over fixed inputs

The specification now fixes what the next candidate carries and how a
host-independent check judges it (`seatbelt-execution`, "The experimental
startup template is an audited rule ledger"). This design adds no policy to
it. It decides the implementation shape, which is probe-only apart from one
constant in `hands.rs`.

**One host-toolchain list.** `hands.rs` gains
`pub const HOST_TOOLCHAIN_BINDS: &[&str]`, holding the seventeen literals of
the inline array `box_argv` iterates today, in the same order, with the
`/usr/include` comment beside its entry. `box_argv` iterates the constant and
the inline array is deleted, so there is one list rather than two kept equal
by a test. This is the only production-file edit before native proof. It adds
no function, branch or error path, so the exact-coverage denominator does not
grow, and the namespace argv stays byte-identical. The declared-bind loop and
the git common `config` bind are untouched. A host-independent test renders
`box_argv` over the scenario's varied `HandsSpec`, home and `GitFacts` values
and requires its `--ro-bind-try` sources to be exactly the constant, the
home-expanded declared `ro` binds and `<common>/config`. The existing namespace
argv tests pin the byte identity.

- *Rejected: a parallel constant pinned to the inline array by a test.* It
  fails only when that test runs, after the drift exists.
- *Rejected: a named minimal `box_argv` invocation.* The proposal refutes it:
  the check would still read an argv, depend on scratch paths and home-derived
  defaults, and admit a future default bind silently.

**The ledger is the renderer's only input.** A typed `STARTUP_RULE_LEDGER`
constant sits in the probe's shared model (`tests/seatbelt_probe/mod.rs`)
beside the removal set. Each entry is one normalized rule unit (one operation
and at most one simple filter, over the placeholders `<cell-root>`,
`<payload-root>` and `<helper>`) plus exactly one class:

- `Baseline { kind, justification }`, where `kind` is a hands element (toolchain
  naming its bind, system library with an optional typed correction, writable
  worktree, device set or shell), an execution input or a probe-harness need;
- `DiagnosisAdmitted { operation, target, process, consumer, evidence,
  removal }`, where `removal` names one entry of the removal set.

The removal set stays the existing `STARTUP_NEGATIVE_ALLOWANCES`, now one
normalized unit per entry, so a replay strips exactly one unit. The fa7
dispositions are a third typed table that maps every fa7 unit to "carried" or
to its withdrawal or narrowing reason; the fa7 template stays verbatim in the
probe's test data.

The candidate template is rendered from the ledger: the frame, then one
`allow` form per unit in ledger order. The concrete profile substitutes the
observer's validated values. A removal replay renders the ledger without its
one unit, a restoration diagnostic renders it plus one fa7-form unit, and the
all-restored cell renders it plus every such unit. The fa7 literal
`sandbox_profile` leaves the native path. The check always parses the
concrete text the renderer produced, so a renderer that groups, escapes, drops
or duplicates a unit is caught by the parse rather than trusted. Cross-cell
identity is the digest of the normalized template over all three placeholders,
which supersedes fa7's cell-root-only token.

- *Rejected: keep a hand-written profile literal and assert it parses to the
  ledger.* It is a smaller diff, but it keeps two authority sources pinned by
  a test, the drift class this change removes from the toolchain list, and
  the specification names the ledger the only source of the candidate's
  authority. Reviewers still see the candidate's bytes, because every report
  retains the normalized template and each concrete profile.
- *Rejected: a data-file ledger or a general SBPL parser.* A data file's drift
  is invisible to the compiler, and the grammar is closed, so a hand-written
  tokenizer for exactly the admitted forms refuses everything else by
  construction.

**The check is pure and reports every refusal in a fixed order.** The check
is a function in the shared model over the concrete profile text, the ledger,
the removal set, `HOST_TOOLCHAIN_BINDS` (read directly, with no parameter for
it, so no caller can hand it a copy) and a typed
`CheckInputs { cell_root, payload_root, inputs_dir, helper }`. The
denial-control targets are derived from the shared control constants below
through the same spelling map. It performs only string and path-component
work and never touches the file system. It runs in three stages:

1. validate the concrete inputs against every input rule, in the order the
   specification lists them, and stop before any rewrite if one fails;
2. parse the closed grammar and normalize, longest validated path first;
3. judge each unit's class and its two-part anchor, apply the process rule
   to every unit of both halves, apply the cover, data-volume and
   control-target rules to its normalized and concrete forms, and compare the
   units with the disjoint union of the baseline and the removal set.

Stage 1 returns a private `ValidatedInputs` value that only stage 1
constructs and that the normalizer takes as its only source of concrete
roots, so normalizing an unvalidated input does not compile. It is one
struct, not a public type hierarchy. The check returns the checked template
or a `Vec<CheckRefusal>`. `CheckRefusal` is an enum with one variant for each
refusal the specification lists, carrying the input or unit and what it found
(for example the source `/etc/ssl` and the spelling `/private/etc/ssl` a cell
root lies under). A stage that refuses ends the check, so no unit is judged
from unvalidated inputs or unparsed text. Stage 1 reports input refusals in
the specification's rule order, stage 2 reports grammar refusals in text
order, and stage 3 reports unit refusals in template order and then missing
ledger or removal entries in ledger order. Within one unit, refusals follow
the order of the specification's refusal list, so a toolchain `process-exec`
on `/usr/lib` reports its failed operation anchor and then the process rule.
The same inputs therefore always produce the same report. One
`spellings(path)` map yields the direct spelling plus the `/private` spelling
for a first component of
`etc`, `var` or `tmp`. Validation, the cover rule and the control-target
list all use it; the toolchain anchor reads only the direct spelling.

- *Rejected: `bool` or formatted-string refusals.* A later edit can soften a
  message until it no longer names the input, and nothing makes a new refusal
  case testable. The enum makes each variant a named test.
- *Rejected: a first-refusal priority order.* A cell root that is both
  non-canonical and on the data volume should report both. The
  specification's exec-and-fork scenario also requires one unit to be named
  under its failed anchor and under the process rule, which a first-only
  report cannot do. Reporting every refusal in a fixed order is as
  deterministic and loses nothing.
- *Rejected: a separate layout-refusal enum beside `CheckRefusal`.* One enum
  in one fixed order covers all three stages. Two enums would need a merge
  order that the specification does not state.
- *Rejected: canonicalizing inside the check.* It would make the
  host-independent verdict depend on the host, and it would erase the split
  between what the string check proves and what the observer proves.
- *Rejected: reusing the unbuilt production D6 normalizer.* It would tie a
  probe precondition to semantics nobody has measured.

**Anchors live in the check, keyed by kind, and match operations by exact
name.** A ledger entry records its class and its element or kind. It never
carries its own anchor, because an entry that carried one could widen the
anchor in the same edit that widened the unit. The check maps each baseline
element or kind to one fixed anchor with an operation part and a target part:

| Element or kind | Admitted operations | Target test |
| --- | --- | --- |
| toolchain | `file-read*`; `process-exec` only when the named bind is a program bind | exactly the named bind's path in its direct spelling, which must be a `HOST_TOOLCHAIN_BINDS` source and not a declared bind or `<common>/config` |
| system library | `file-read*` | `/System/Library`, `/System/Volumes/Preboot/Cryptexes/OS` or a recorded correction of one |
| writable worktree | `file-write*` | exactly `<payload-root>` |
| device set | `file-read*` | exactly one of the literals `/dev/null`, `/dev/urandom` and `/dev/random` |
| shell | `process-fork` | none; it is the only unfiltered unit |
| execution input | `file-read*` or `process-exec` | exactly `(literal "<helper>")` |
| probe-harness need | `file-read*` | exactly `(subpath "<cell-root>/inputs")` or `(subpath "<payload-root>")` |

An operation matches by byte equality of its token. No anchor has family or
prefix logic, so `file-write*` admits neither `file-write-data` nor any other
member, and a unit spelled with a member name fails its anchor rather than
reading as covered. `PROGRAM_BINDS` is a fixed five-entry constant beside the
ledger: `/usr/bin`, `/usr/libexec`, `/usr/local`, `/bin` and `/sbin`. The check
confirms that each is a `HOST_TOOLCHAIN_BINDS` source and refuses by name
otherwise, so the program list cannot admit an exec on a path the box does
not bind.

A separate process rule runs over every unit of both halves. A unit whose
operation begins with `process-` must equal one of the seven `PROCESS_UNITS`,
compared as whole normalized units. The prefix appears only in this rule,
where it can only refuse. A diagnosis-admitted unit therefore never adds
process authority. That half carries no element anchor. Its bound is the
single-object filter, the recorded evidence and a removal entry that a
`READY` cell must observe blocking, plus the process, cover, data-volume and
control-target rules that every unit meets. Because the device-set anchor
admits only `file-read*` and no other anchor admits a write outside
`<payload-root>`, an attributed `/dev/null` write can never be baseline. It
enters only as the diagnosis-admitted literal with its own removal entry.

`CheckRefusal` carries one variant each for an operation that fails its
anchor and a target that fails its anchor (both naming the unit and the
element or kind), a `process-*` unit outside the seven, and a program bind
that is not a host-toolchain source. Every candidate unit passes its anchor,
so the anchors change no candidate unit.

- *Rejected: matching an operation by family prefix.* The specification
  refutes it. The worktree's `file-write*` would admit `file-write-data`, and
  the check would accept operation names nobody listed.
- *Rejected: the process rule as an anchor on the baseline half only.* A
  diagnosis-admitted `process-exec` would then add process authority through
  the half that has no element anchor.

Falsification tests exercise both anchor parts. For the target part they use
ledger entries that name binds the constant lacks, such as the home-expanded
`~/.rustup`, `<common>/config` and `/opt/homebrew`. For the operation part they
use a baseline `/dev/null` write, a `file-write*` on `/usr/local` or
`/System/Library`, an exec on the non-program sources `/usr/lib` and
`/usr/share`, a `process-fork` with a filter, an exec on `<payload-root>`, the
execution-input and probe-harness cases, and member-named operations. Each
fails with its named variant, and every candidate unit passes. A new constant
entry adds no unit and grants nothing, because the ledger is explicit and a
bind without a unit only narrows. It only tightens the cover rule and
cell-root validation. The fail-closed direction is the anchor: a toolchain
unit that names a bind the constant does not list fails.

**Denial-control targets share one source with the helper.** The helper is a
separate binary target (`[[bin]] seatbelt-probe-helper`) and cannot import the
shared model. Today it hard-codes `/etc/passwd`, `/etc/hosts` and
`/private/tmp/brokkr-probe-denial-write` inside its attack functions. A small
`tests/seatbelt_probe/controls.rs` holds the paths the credential-read,
data-volume credential-read (`/System/Volumes/Data/private/etc/passwd`) and
host-write controls open. The helper includes it through `#[path]`, the shared
model declares it as a module, and the check derives its control-target input
from the same constants. That is the one new file. `/etc/passwd` is the only
credential target with a data-volume spelling and a data-volume control. The
check's control-target list is `spellings()` of the three direct targets plus
that data-volume path as its own entry. `/etc/hosts` gets no data-volume
entry, because the data-volume rule already refuses every unit on the volume.

- *Rejected: copying the literals into the check.* A control could then
  change its target while the check still refuses the old path, which is the
  same drift class one file over.
- *Rejected: passing targets through the helper's argv.* It would change the
  argv structure the cells compare and add protocol for a fixed fact.

**Observer duties are native, ordered and without fallback.** In `native.rs`
the observer:

1. canonicalizes the host temporary base once;
2. creates the per-run probe root under it with an exclusive owner-only
   create (`DirBuilder` with mode `0o700`, never `create_dir_all`);
3. stages `<helper>`: it creates `<probe-root>/bin` the same way, opens
   `<probe-root>/bin/seatbelt-probe-helper` with an exclusive `create_new`
   open, copies the bytes of the committed build
   (`CARGO_BIN_EXE_seatbelt-probe-helper`) into it and makes it executable,
   then confirms a regular file owned by the invoking user with exactly one
   link whose digest equals the build's. The report records both digests;
4. creates each cell root with the same exclusive owner-only create, then
   `payload` and `inputs` inside it;
5. confirms each is a directory owned by the invoking user;
6. canonicalizes the cell root and the staged helper and requires exactly the
   input spelling.

Creation always precedes canonicalization, which compares and never
substitutes. The silent `unwrap_or(raw)` fallback at `native.rs:252` is
deleted. The helper is staged once per run and outside every cell root, so
every cell, replay and diagnostic of one run starts the identical staged bytes
at one spelling. S0 runs that same staged spelling unboxed, so a staging
defect fails S0 and is never attributed to Seatbelt. Cargo keeps the build
under a second hard-link name in `deps/`, so the build file itself is never
`<helper>`. Only then does the observer build `CheckInputs`, run the check
over the cell's concrete candidate profile and invoke `sandbox-exec`. A
failed duty or refusal fails the cell before any payload runs, and the report
names it. Removal replays and diagnostics run only from a cell whose inputs
passed.

- *Rejected: canonicalize, then create.* A path planted between the two steps
  would redirect every later bind.
- *Rejected: using the build file as `<helper>`.* Its second hard-link name is
  a respelling of the same file that no string rule can tie to the helper.
- *Rejected: staging with `fs::copy`.* It truncates an existing file instead
  of failing, so it cannot give the exclusive creation the duty requires.

**The helper re-executes through its launch argument.** The observer passes
`--helper <helper>` in the argv `sandbox-exec` runs, in the launchd job's
program arguments and in S0's direct exec, and the helper passes it on to
every mode it execs. Every mode reads it before its first stage. Without it,
the helper exits 2 with a named message and records no stage. Each of the
seven `current_exe()` sites in `helper.rs` (the startup mode's ordinary
child, five payload attack cases and the `detach-child` role) execs that
value instead, and the helper never resolves its own path. The `executable`
stage therefore records that the launch argument was read, not that a
`current_exe` call returned. That is one more reason fa7's `executable`
progress does not carry over. The argument is the typed `<helper>`
substitution in the structural argv comparison, so the cells still compare
as identical.

- *Rejected: `current_exe` or `argv[0]`.* Either can return a spelling the
  check never validated, such as the second hard-link name, a relative path or
  a resolved data-volume spelling.

**The child spawn becomes observable.** The helper pre-opens each ordinary
child stream itself (stdin read-only, stdout and stderr write-only on
`/dev/null`) and records `child-stdin`, `child-stdout` and `child-stderr`. It
then spawns with those handles (`child-spawn`) and observes the child
(`child-observed`), recording the sub-stage reached and the raw OS error on
failure. Today `Stdio::null()` hides the opens inside `Command::spawn`, so the
`EPERM` could come from either the open or the exec. When a Seatbelt cell
fails at a child sub-stage while S0 spawns, four labelled cells run as direct
replays from that cell's root under the exact candidate:

- open `/dev/null` write-only with no spawn;
- spawn with inherited stdio;
- spawn with null stdio;
- the ordinary mode plus exactly
  `(allow file-write-data (literal "/dev/null"))`.

None passes startup. An attribution enters the ledger only as the
specification's diagnosis-admitted literal with its own removal entry. An
attribution to a `process-*` operation outside the seven process units stays
evidence. An exec-side refusal of the helper under another resolved spelling
fails the cell on its own startup facts and records
`SEATBELT-R3-STARTUP-helper-respelling` with its denial evidence. It admits
nothing in either half and changes neither `<helper>` nor the ledger. Any
other spelling of the helper waits for a focused proposed decision.

**Withdrawal regressions are discriminated, not assumed.** When a Seatbelt
cell of the ledger candidate fails before `READY`, each withdrawn or narrowed
fa7 unit runs alone, in its fa7 form, as a labelled restoration diagnostic,
and one more cell restores all of them. Every diagnostic, the seven retained
one-class differentials and `allow default` included, is a direct
`sandbox-exec` replay from the failing cell's validated root. No launchd
label is bootstrapped for one, because launchd adds no Seatbelt authority. The
seven differentials are kept unchanged and authorize nothing.

**Each Seatbelt cell keeps its denial events.** After each Seatbelt cell,
replay or diagnostic, one bounded `/usr/bin/log show` invocation over the
cell's time window, filtered to Sandbox events for its responsible process
IDs and executable, keeps at most a fixed count and size of raw events. Its
exit status is kept too. If the host refuses or returns nothing, that is
recorded as unavailable or empty, never as proof that an operation was
allowed. No resident collector or `log stream` supervisor is added.

**Launchd facts and removal verdicts follow the settled answers.** A
brace-depth scanner over the single outermost block replaces the
all-or-nothing `parse_launchd_print`. It gives each field an independent
parsed or unknown value, raw evidence for nested entries under their block
path, and unknown for duplicated top-level keys. `state`, `runs` and
`last exit code` are required, and `runs` must be exactly one completed run;
`successive crashes`, a terminating signal and
`active count` are optional, and a present nonzero crash counter or a present
terminating signal fails the cell. The launchd exit classifier no longer infers
"clean" from a zero crash counter, which may be absent. The three fa7 raw
samples become verbatim regression data in the probe's test data, not in
`fixtures/`. `StartupNegativeControl` records each removal as not due (the
cell reached no `READY`), observed blocking, observed not blocking, or
missing. Only a `READY` cell owes observed-blocking controls, and blocking
means the stripped replay reached no nonce-authenticated `READY` from fresh
payload state.

The ledger candidate's startup, denial controls and removal controls are
unmeasured until the controller's exact-head Gate A run. No fa7 stage
progress, denial or not-due removal carries over.

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
ledger candidate and the probe repairs above, followed by a
controller-dispatched Gate A run on that materially changed exact head. Its
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

The first such extracted fact is `HOST_TOOLCHAIN_BINDS` (D3). `box_argv`
iterates it, and the probe's startup-rule check reads the item itself, never
an argv. This keeps the rule above: the check takes the shared list, not
`box_argv`'s mount semantics, and no `HandsSpec`, home or `GitFacts` value
reaches its verdict. A binding test keeps the argv honest: its
`--ro-bind-try` sources are exactly the item, the home-expanded declared `ro`
binds and `<common>/config`. Declared binds and the common `config` are never
toolchain binds.

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

When this normalizer is built, it takes its controlled host-toolchain paths
from the same `HOST_TOOLCHAIN_BINDS` item, so the namespace box, the probe and
production read one list. It does not reuse the probe's startup-rule check,
which is a string-level probe precondition over a fixed experimental
template, not production authority analysis.

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
2. implement only the bounded Gate A candidate D3 names: the
   `HOST_TOOLCHAIN_BINDS` item, the typed startup-rule ledger and its
   renderer, the pure check over validated inputs, shared control targets,
   ordered observer duties, child-spawn sub-stages and discriminating cells,
   restoration diagnostics, per-cell denial events, the top-level launchd
   scanner and the READY-only removal record;
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

The current positions use HEAD `d52cbd7` and native CI `34457208029` at
candidate `fa7ece5`. Both confirm that the specification answers the 366 gap
in `48b6f9d` through `d52cbd7`. Both accept the accepted boundary and every
settled answer. They differ only on how much structure the implementation
carries around the fixed inputs. Every current claim is disposed of here. The
`98984dd` table, the `70bdb1b` table and the older table below remain
historical reconciliation where they do not conflict.

| Current council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
| Simplicity: the 366 gap is answered at HEAD: `HOST_TOOLCHAIN_BINDS`, typed inputs validated before normalization, the `d1d714d` anchors and the 366 scenarios. Ratify it and add no policy. | **Adopt, after checking it.** | The `seatbelt-execution` delta names the one item `box_argv` iterates. It excludes declared `ro`, `rw` and overlay binds and `<common>/config`, and the check takes no `HandsSpec`, home or `GitFacts`. It types and validates the concrete roots and helper, and it carries every scenario the 366 record asked for. This visit adds no policy and reopens no settled answer. |
| Simplicity: `d52cbd7` fixed the old `409-412` correction clause, and a sweep finds no other unsatisfiable clause. | **Adopt; this design removes the lag that remained.** | The delta now fails an exec-side helper respelling closed. Before this visit, the design contradicted that in three places. The observer duty accepted any regular helper file, the child-spawn paragraph did not exclude a process or respelling attribution, and D3 did not state the operation anchors. All three are rewritten above, and the risk list names the residual. No clause of the delta is at fault, so nothing returns upstream. |
| Robustness: `HOST_TOOLCHAIN_BINDS` is the only list, and `box_argv` iterates it. Simplicity: a pure extraction is the whole production change. | **Adopt; unchanged from `80d9736`.** | There is no second list to drift from. |
| Robustness: the denial-control targets need one source shared with the helper's attacks. | **Adopt; unchanged.** | `controls.rs` also carries `d1d714d`'s single data-volume control, and the check's control-target list derives from it. |
| Robustness: a pure, file-system-free check with a typed refusal, a specified priority order and a `ValidatedLayout` result. Simplicity: one ordered `Vec<CheckRefusal>` and no validated-layout type hierarchy. | **Combine.** | One enum and every refusal in a fixed order, now including the order within one unit, which the exec-and-fork scenario needs. One private `ValidatedInputs` struct means that normalizing an unvalidated input does not compile. There is no second enum and no public hierarchy. |
| Robustness: exclusive create, then canonicalize. | **Adopt and extend.** | The staged helper copy from `d52cbd7` is created with `create_new` before it is canonicalized, like the roots. |
| Robustness: a test proving that an unaccounted `HOST_TOOLCHAIN_BINDS` entry fails the ledger equality, and a structural link between the spelling map and the constant. | **Reject the equality test again; adopt a smaller link.** | The specification says a bind with no unit only narrows, so that test would assert something false. The spelling map is derived from each path, not kept as a parallel table, as simplicity argues. The one link the specification requires is that each of the five program binds is a `HOST_TOOLCHAIN_BINDS` source, and the check confirms it. |
| Robustness: host-independent tests use synthetic paths that do not exist on the test host, and the observer's file-system duties are a separate native function, not a `cfg` branch inside the check. | **Adopt.** | The check's tests use the specification's macOS paths (`/private/var/folders/xy/T/...` and `/Users/runner/...`), which do not exist on the Linux controller. A file-system call slipped into the check would change its result there. The duties stay in `native.rs`. |
| Simplicity: D3 does not yet state `d1d714d`'s operation anchors or their refusals. | **Adopt.** | D3 now tables every anchor and keeps the anchors in the check, not in ledger data. Operations match by exact name, the check confirms the program binds, and the process rule runs over both halves. |
| Simplicity: the helper re-executes through its validated spelling, and a respelling refusal is a named residual. | **Adopt, naming the mechanism.** | D3 stages one single-link copy per run, passes `--helper`, replaces all seven `current_exe()` sites and records `SEATBELT-R3-STARTUP-helper-respelling`. No override, recorded alternative or data-volume helper grant is added. |
| Simplicity: cut a toolchain-source type system, a general SBPL parser, a denial-capture subsystem, a dual-architecture Gate A and pre-Gate-A production machinery. | **Adopt; unchanged.** | D3 and D14 already keep that order. |
| Simplicity: reconcile `design.md`, `tasks.md` and `evidence-residuals.md` in dependency order. | **Adopt for this artifact; the rest is downstream.** | This design is reconciled to `d52cbd7`. The tasks visit must add these to tasks 1.20–1.23: the operation-anchor, process-rule and helper-respelling scenarios as tests, the staged-copy duty and the `--helper` re-exec. `evidence-residuals.md` gains no row, because no native run has happened since `34457208029`. A helper-respelling residual gets a row only if a native cell records one. |

The `98984dd` positions below were reconciled when `80d9736` first designed the
ledger candidate.

| `98984dd` council claim | Disposition | Evidence and resulting design |
| --- | --- | --- |
| Simplicity: the clarify gap is already closed by `48b6f9d` and `98984dd`; ratify it without re-elaboration and add no policy. | **Adopt.** | The specification fixes the source set, the typed inputs, validation before normalization and the adversarial scenarios. D3's ledger subsection decides only structure. No toolchain-source type system, new crate or vocabulary is added. |
| Robustness: `HOST_TOOLCHAIN_BINDS` must be the only list, iterated by `box_argv` directly, not a parallel array pinned by a test. Simplicity: a pure const extraction with a byte-identical argv is the whole production change. | **Adopt both; they agree.** | D3 deletes the inline array. The one production edit adds no branch, and the binding test plus the existing namespace argv tests prove the item and the byte identity. |
| Robustness: the denial-control targets need the same single source; the helper hard-codes them today. Simplicity: no new file or module. | **Adopt robustness; simplicity yields on one file.** | `helper.rs` is a separate `[[bin]]` target and cannot import the shared model, so one `controls.rs` included by both is the smallest single source. Copying literals repeats the drift class the clarify record closed. |
| Robustness: validation is a pure, file-system-free function with a typed refusal enum and a specified priority order. Simplicity: `validate(...) -> Result<(), CheckRefusal>` over path components. | **Combine.** | Pure function and typed `CheckRefusal` enum are adopted. A first-only priority is rejected in favour of every refusal in a fixed order, which is as deterministic and reports a doubly broken input completely. |
| Robustness: exclusive create before canonicalize, never the reverse. | **Adopt.** | The observer canonicalizes only the pre-existing temporary base, creates the probe and cell roots exclusively and owner-only, then canonicalizes to compare, with no fallback. The `unwrap_or(raw)` fallback is deleted. |
| Robustness: a test must prove that adding an unaccounted `HOST_TOOLCHAIN_BINDS` entry fails the ledger equality. Simplicity risk 3 says the same equality fails closed on a new bind. | **Reject as contrary to the specification.** | The equality compares the rendered template with the ledger, not with the constant. The specification names binds that carry no unit (`/usr/include`, `/usr/lib64`, `/lib`, `/lib64`, the `/etc` binds) and says that only narrows authority. A new entry therefore grants nothing and only tightens the cover rule and validation. The fail-closed direction the tests prove is the anchor: a toolchain unit that names a bind the constant lacks fails. |
| Robustness: the pure check and the observer's file-system duties are two functions with two test strategies. Simplicity: keep observer duties in `native.rs`, apart from the check. | **Adopt.** | The check lives in the shared model and is tested on Linux with synthetic paths that do not exist. The duties are native-only and are named in the cell report. |
| Simplicity: keep the fa7 literal and assert it parses to the ledger, or render from the ledger; either is acceptable. | **Render from the ledger.** | One authority source, like the toolchain list. Removal, restoration and all-restored profiles become ledger operations, and the parse still judges the rendered text. |
| Simplicity: a typed const ledger in the shared model, a hand-written closed-grammar tokenizer, no data file or general SBPL parser, no reuse of D6. | **Adopt.** | The closed grammar refuses everything outside it, and D6 stays unbuilt. |
| Simplicity: one bounded `log show` or `log stream` invocation for denial capture, not a collector daemon. | **Adopt, choosing `log show`.** | A post-cell `log show` over the cell's window needs no process kept alive across the cell. Unavailability is recorded and never read as permission. |
| Simplicity: the launchd parser is field-wise over the top-level dictionary, the exit classifier stops inferring clean from an absent counter, and `blocked` counts only no-`READY`. | **Adopt.** | These are the settled answers. D3 names the scanner, the optional fields and the four-way removal record. |
| Simplicity: no Gate B apparatus before Gate A and B0, B0 before B1, no production D5–D13 before native proof, one architecture for Gate A. | **Adopt; already in D3 and D14.** | No change to the existing gate order or to the both-architectures activation requirement. |
| Simplicity: `design.md`, `tasks.md` and `evidence-residuals.md` lag the fa7 run and the ledger. | **Adopt for this artifact; the rest is downstream.** | This design records fa7 and the ledger candidate. The tasks and evidence visit `a633f13` then appended the `34457208029` evidence row, without rewriting earlier rows, and added the ledger candidate as tasks 1.18–1.28. |

The `70bdb1b` positions below were reconciled at native CI `34449331270`.

| `70bdb1b` council claim | Disposition | Evidence and resulting design |
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

- **[The narrower ledger candidate regresses below fa7's stages]** → The
  withdrawn and narrowed fa7 units run as single restoration cells plus one
  all-restored cell, and denial events name the operation. A restored unit
  never re-enters in its historical form; only a single-object
  diagnosis-admitted entry with its own removal control may.
- **[The child spawn stays unattributed]** → The sub-stages and four
  discriminating cells either name the refused open or exec or record a
  precise residual. Seatbelt stays `unbuilt: ii` and no `/dev` subpath, broad
  write or wider process grant is tried.
- **[A toolchain bind is refused under another resolved spelling]** → The
  cell fails and records `SEATBELT-R3-STARTUP-toolchain-respelling`. Changing
  a toolchain unit's target waits for a focused proposed decision.
- **[The helper is refused under another resolved spelling]** → The cell
  fails on its own startup facts and records
  `SEATBELT-R3-STARTUP-helper-respelling`. Neither `<helper>` nor the ledger
  changes, and another spelling waits for a focused proposed decision. The
  staged single-link copy and the `--helper` re-exec leave the helper no
  spelling but its validated one to be exec'd by.
- **[The staged copy does not start where the build would]** → S0 starts the
  same staged spelling unboxed. A lost execute mode, a digest mismatch or a
  host rejection of the copy therefore fails S0 and is recorded as a staging
  defect, not as a Seatbelt verdict. The build's hard-linked file is never
  substituted.
- **[An exact-name anchor refuses a member operation the payload needs]** →
  Once native evidence attributes it, a member such as `file-read-metadata`
  on one object enters only as a diagnosis-admitted single-object unit with
  its own removal entry. Refusal is the intended direction, and widening an
  anchor's operations needs the delta to change.
- **[The fixed spelling map misses a future macOS alias]** → The data-volume
  rule and the canonical-spelling rule still bound it. A newly observed alias
  becomes a named residual, never a host lookup that would make the verdict
  host-dependent.
- **[The string check refuses a profile that would behave correctly]** →
  Fail-closed is the intended direction. The defect this change fixes is the
  opposite one, a declared bind or `/Users/runner` passing as an anchor.
- **[A same-user host process swaps an ancestor after the observer's
  checks]** → The probe root is fresh, exclusive and owner-only, and the
  observer's canonicalization compares rather than substitutes. The probe
  claims no defence against a concurrent host process running as the same
  user outside the payload, which shares the observer's own trust.
- **[The `hands.rs` extraction moves exact coverage or the namespace
  argv]** → It is a loop-source substitution with no new branch, pinned by the
  existing byte-exact argv tests and the new binding test. The controller's
  external exact-coverage run measures it.
- **[The exact payload cannot start under the bounded profile]** → Run D3
  Gate A first, require operation/target attribution and necessity/sufficiency
  evidence before changing one narrow predicate, preserve all denial controls,
  and keep lifetime marked not run on failure.
- **[Native denial logs are unavailable or incomplete]** → Preserve the
  `log show` status and raw bounded output, then rely on the stage ladder,
  the child sub-stages and the restoration cells. System-binary and
  first-instruction brackets return only if those also fail to attribute a
  refusal. An absent denial record stays unknown and never authorizes a
  guessed allowance.
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
2. Preserve native CI `34457208029` at `fa7ece5` as the fourth failed Gate A
   observation. `a633f13` appended its evidence row and added the ledger
   candidate as tasks 1.18–1.28, without rewriting the `6a19a6f`, `8c53dce`
   and `9f4c2c9` evidence or reading S2's and S3's erased prints as launchd
   verdicts. Before implementation, the tasks visit adds the `d1d714d` and
   `d52cbd7` scenarios, the staged-copy duty and the `--helper` re-exec to
   tasks 1.20–1.23.
3. Implement the ledger candidate from D3 while Seatbelt remains
   `unbuilt: ii`:
   - the `HOST_TOOLCHAIN_BINDS` extraction;
   - the typed ledger, the fa7 dispositions and the renderer;
   - the pure check with its typed inputs and refusals, its two-part
     anchors, the program-bind confirmation and the process rule over both
     halves;
   - the shared control constants;
   - the ordered observer duties, the staged single-link helper and the
     `<cell-root>/{inputs,payload}` layout;
   - the `--helper` launch argument in place of every `current_exe()` site;
   - the child sub-stages and discriminating cells;
   - the restoration diagnostics and per-cell denial events;
   - the top-level launchd scanner and the four-way removal record.

   Add the host-independent tests the specification's scenarios name, pass
   every local gate, and commit that materially changed probe.
4. Have the controller dispatch Gate A alone on that exact head and preserve
   the complete report. A failed or unknown cell appends the precise startup
   residual and returns to step 3 with the attribution it produced. It never
   runs lifetime, reruns an unchanged candidate or broadens the admitted
   profile.
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
settles R1, R2, R3's mandatory outcome and conditional R4. The specification
visits through `d52cbd7` settle the startup-rule ledger, its closed grammar,
the per-bind `/usr` narrowing, the no-respelling rule, the fixed toolchain
source set, the validated check inputs, the two-part operation and target
anchors with the process rule in both halves, and the helper's single staged
spelling. None may be reopened to explain away a failed probe.

Four empirical questions remain, in strict order:

1. Does the ledger candidate start at all? It is narrower than fa7's
   template, so it may stop before fa7's `executable` stage, which now
   records the launch argument read rather than a `current_exe` call. The
   restoration cells and denial events are what attribute such a regression,
   and S0 separates a staging defect from a Seatbelt refusal.
2. Which operation and target does the child spawn need? Leading hypothesis:
   the null-stdio write open of `/dev/null`, which no fa7 profile or
   diagnostic granted. The sub-stages and four discriminating cells decide it
   or move attribution to the exec. The answer is the diagnosis-admitted
   literal the specification names or a precise residual.
3. With a candidate that reaches `READY`, is each diagnosis-admitted
   predicate, the root-inode read included, load-bearing by removal? Does the
   top-level launchd parse give S2 and S3 their truthful terminal facts?
4. Only after Gate A passes: does the public unprivileged per-user launchd
   payload-job domain end every `setsid` and double-fork descendant after
   timeout, cancellation and supervisor death, while the separately owned
   guard and protected peers survive?

CI `34457208029` answers none of those positively. It shows the root-inode read
advances the fa7 template to `executable`, three denial controls deny on that
candidate, S2 starts under launchd, and the parser defect erased real facts.
The available arm64 CI route can run the next candidate, so no local Mac is
missing. A Gate A failure is recorded precisely and leaves lifetime not run. A
B0 or B1 failure rejects the D2 candidate and keeps SEATBELT-R3 open. Any
request to weaken accepted semantics, respell a toolchain unit or the helper,
or narrow decision 0049's platform scope returns as a focused `proposed`
decision rather than a downstream workaround.

## Implementation status

The current committed head preserves the probe work through `fa7ece5`, the
fourth native report, the specification visits that settle the ledger
candidate and the implementation of tasks 1.18–1.27. It contains no production
Seatbelt implementation. Seatbelt remains `unbuilt: ii`; container remains
`unbuilt: iii`.

The committed probe has demonstrated preparation:

- one portable Rust helper with authenticated stages;
- a fail-closed four-cell model with isolated roots and labels;
- target-gated Unix ABI and a single explicitly selected native test;
- the root-inode read, which moved S1 and S3 to `executable`;
- bounded raw `launchctl print` retention;
- host-independent evaluator tests.

Native CI `34457208029` confirmed that progress and three denial controls on
that candidate. It also showed that the committed parser erases printed
terminal facts when the crash counter is absent, and that the ordinary-child
spawn is refused. No later native run exists, so the ledger candidate's startup
is unmeasured.

The D3 ledger candidate is now implemented:

- `hands.rs` exposes `HOST_TOOLCHAIN_BINDS`, and `box_argv` iterates it so
  there is one list and the namespace argv is byte-identical;
- `controls.rs` is the one source of the path-valued denial-control targets
  that both the helper and the check read;
- `ledger.rs` carries the typed 23-entry baseline plus one diagnosis-admitted
  entry, `PROGRAM_BINDS`, the fa7 disposition table and the pure check, which
  validates the concrete inputs before normalization and returns an ordered
  `Vec<CheckRefusal>` of named variants, including a failed operation anchor
  and a failed target anchor, a `process-*` unit outside the seven and a
  program bind that is not a host-toolchain source;
- `native.rs` stages the helper with exclusive owner-only creates and no
  canonicalization fallback, passes the validated `--helper` spelling, runs
  the check over the concrete profile before `sandbox-exec`, and carries the
  child-spawn sub-stages, the four discriminating replays, the restoration
  diagnostics, the bounded `log show` collection and the top-level launchd
  scanner with a four-way removal record.

The host-independent probe suite passes (95 tests, two native-only ignored),
and the ledger candidate's Gate A startup, denial controls and removal
controls are still pending the controller's exact-head native run. Tasks
1.28–1.33 (native Gate A, B0 and B1) remain open. No production planner,
lifetime executor, overlay, mask, Git, runtime transport, activation edit or
full-peer claim is authorized until Gate A and the complete B1 feasibility
matrix pass. Exact coverage, native dispatch, final remote CI, publication,
integration and closure remain controller-owned.
