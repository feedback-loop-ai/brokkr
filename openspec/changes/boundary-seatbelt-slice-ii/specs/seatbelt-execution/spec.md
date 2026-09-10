## Purpose

Execute workspace commands and boxed exec seats on macOS through a real
Seatbelt boundary, preserving declared hands policy and separating tested
security claims from pending measurements and proposed semantic rulings.

The positive execution scenarios define the commissioned target after
`boundary-availability`'s activation conditions hold. On this returned visit
R1–R4 have accepted observables under the 2026-09-09 addendum to 0046,
but native proof remains required; no partial implementation or refusal-only
path satisfies the target. The current unbuilt refusal remains in force.

## ADDED Requirements

### Requirement: Both hands entry points execute inside the declared Seatbelt boundary

For a site with hands compiled under `seatbelt`, every workspace MCP
command and the whole boxed exec dispatch SHALL execute under the real
macOS `sandbox-exec` policy, including their children. The provider harness
remains outside the boundary as trusted control-plane code. The same hands
spec SHALL have the same enforcement through either entry point.

`brokkr hands serve` and `brokkr hands exec` SHALL accept explicit boundary
transport from runtime composition, including both adapter MCP configuration
forms. Omission on these low-level commands SHALL retain `namespace` for
existing callers. This argument transports the compiled realm choice; it
is not a run-level override or a new `hands` field. A low-level request for
`harness`, `open`, `container` or an unknown word SHALL refuse rather than
serve an unconfined workspace tool. A missing/unusable launcher, unsupported
host, invalid policy or failed preparation SHALL execute no user command
and SHALL report the failed boundary and cause, with no fallback.

#### Scenario: Workspace and exec exercise the real launcher
- **WHEN** a macOS test sends an MCP workspace request to `hands serve` under `seatbelt`, then runs the equivalent command through `hands exec` under `seatbelt`
- **THEN** both execute through the system `sandbox-exec`, write an allowed worktree file and fail to read a planted outside secret; neither succeeds by substituting an unboxed command or a fake launcher

#### Scenario: The boundary survives both adapter configuration forms
- **WHEN** a model site under `seatbelt` receives the JSON MCP configuration or TOML server-arguments form
- **THEN** its server invocation carries `seatbelt` and its declared policy, and no workspace invocation defaults to `namespace` or uses the harness-only fragment

#### Scenario: A failed boundary executes no payload
- **WHEN** Seatbelt preparation or launch fails, or a low-level caller selects an unsupported boundary
- **THEN** the command's worktree marker is absent, the error names the cause, and neither exec nor MCP reports a successful command

### Requirement: Read and write grants expose only the declared filesystem authority

Seatbelt SHALL deny file-content reads outside the worktree, private call
state, protected execution inputs and the explicitly controlled readable
toolchain/system paths and binds. It SHALL allow worktree edits and the
result-file write while denying host writes outside declared `rw` paths;
private call state and overlay state are the only additional writable
scratch. Necessary system metadata, devices and service access SHALL be
enumerated and justified by the design and measured on macOS, without
turning broad home, temporary, developer or system directory trees into
unreviewed readable grants. Profile paths SHALL be treated as data, not
policy source.

Protections SHALL hold through canonical and symlink spellings, macOS path
aliases, nested/overlapping grants and attempts to replace protected files
or their parent directories. A conflict that cannot preserve all declared
restrictions SHALL refuse before user code. Permission granted for one
bind SHALL not uncover a mask or protected git/execution input in another.
This requirement concerns the filesystem authority exposed to commands;
it SHALL NOT be described as an empty root or Linux namespace isolation.

#### Scenario: Allowed worktree access and denied outside access coexist
- **GIVEN** disposable worktree and sibling directories containing distinct sentinel files
- **WHEN** both hands paths read and edit the worktree and try absolute-path reads and writes in the sibling and a planted operator home
- **THEN** worktree operations succeed, outside operations fail, and outside bytes remain unchanged

#### Scenario: Path spelling cannot widen the profile
- **WHEN** test roots contain spaces, quotes, backslashes, Unicode or profile-like text, and commands access permitted and forbidden paths through symlink aliases and `/var` or `/tmp` canonical spellings where present
- **THEN** permitted paths still resolve correctly or preparation explicitly refuses an unrepresentable path; no spelling injects a grant, exposes an outside sentinel or permits a protected write

#### Scenario: Alias and overlap attacks do not override protection
- **WHEN** a worktree symlink or an overlapping `rw` bind aliases a masked file, protected config or immutable execution input, including through a replaceable parent
- **THEN** the protected operation fails or the conflicting policy is refused before spawn, regardless of bind ordering; a host-side comparison finds no protected change

### Requirement: Commands start with the closed hands environment and private call state

Each workspace call SHALL start from an empty environment with only the
0043 hands entries: `PATH`, `HOME`, `USER`, `LOGNAME`, `TMPDIR`, `LANG`,
`LC_ALL`, `CI`, `DISABLE_AUTOUPDATER`, `DISABLE_TELEMETRY`, the engine-owned
`BROKKR_HANDS_BOX` marker, unsigned-commit git entries and resolved seat git
identity, plus the engine-owned `BROKKR_HANDS_OVERLAYS` locator map. The map
SHALL be a compact JSON array ordered by the original `hands.binds` array.
Each overlay entry SHALL contain exactly `index`, expanded absolute
`declared_path` and absolute `locator`, where `index` is that bind's zero-based
position in the original array; non-overlay binds are omitted and no overlays
yields `[]`.
`CARGO_HOME`, `RUSTUP_HOME` and `NPM_CONFIG_CACHE` SHALL be present only for
their declared binds; Cargo and npm overlay values SHALL equal the locator
from that same generic map. An exec dispatch SHALL receive the same policy,
with private HOME/tmp for that dispatch. HOME/tmp SHALL be fresh per call,
inaccessible to other seats and never the operator's directories; overlay
state and its locator map alone persist for the seat. The launch PATH SHALL
be composed from controlled system/toolchain paths and declared binds, never
inherited from arbitrary engine PATH entries or the working directory.
Interpreter startup SHALL not import undeclared environment or host startup
files. No other semantic difference is authorized; clearing the environment
alone SHALL never be offered as proof of filesystem denial.

#### Scenario: A planted parent environment does not reach commands
- **GIVEN** synthetic parent tokens, SSH agent variables, cloud credentials, loader variables, shell startup variables and a PATH containing a hostile fake interpreter
- **WHEN** the actual workspace and exec paths print their initial environment and run an ordinary tool
- **THEN** the allowed key set matches the declared table, none of the planted secrets or startup actions appears, and the hostile parent PATH entry is not selected

#### Scenario: Call scratch is fresh while overlay state persists
- **WHEN** one workspace call writes into HOME, TMPDIR and a declared overlay, and a second call in the same seat reads those locations
- **THEN** the second call has fresh HOME/tmp without the earlier scratch files and still sees the overlay write; another seat sees none of the first seat's private state

#### Scenario: Toolchain locators follow the declarations
- **WHEN** a seat declares cargo overlay and rustup read-only binds with credential masks, and another declares neither despite parent locator variables being set
- **THEN** the first can locate its admitted toolchain without revealing credentials; the second receives no cargo/rustup locator from the parent, and the real macOS test executes an admitted tool successfully

### Requirement: Network is denied unless the hands declaration grants it

An absent or false `hands.network` SHALL deny command network traffic,
including loopback, outbound connections and listening, for both hands
paths and all descendants. A true declaration SHALL permit the network
needed by the test command without widening filesystem or credential
access. Failed enforcement SHALL refuse; it SHALL never use the unboxed
best-effort network-prefix behavior. Any local IPC or service exception
needed for macOS execution SHALL be narrow, measured and unable to act as
an undeclared network, credential or host-write proxy.

#### Scenario: Network denial and grant have live positive controls
- **GIVEN** test-owned reachable TCP and UDP endpoints, including IPv6 where the test host supports it, with an outside positive control proving reachability
- **WHEN** the same clients run through workspace and exec with network absent, false and true
- **THEN** absent/false cannot exchange traffic or expose a listener, true exchanges the expected nonce, and neither case exposes the planted credential file; the result does not depend on public Internet availability

#### Scenario: Network grant is not a host-service grant
- **WHEN** commands try a test-owned outside Unix socket or other undeclared local service under each network policy
- **THEN** no request can use that service to read the outside sentinel or modify its host file; the design and native measurements name any required service exceptions

### Requirement: All declared bind modes and masks retain their observable contract

Seatbelt SHALL support arbitrary declared bind paths in `ro`, `rw` and
`overlay` modes, with home expansion and credential masks, rather than
special-casing a cargo cache as the entire bind implementation. `ro` SHALL
allow reads and reject mutations; `rw` SHALL permit declared host mutations
except protected or masked paths; `overlay` SHALL provide the seat-private
view required below. Missing optional `ro`/`rw` sources SHALL stay absent,
without host creation; an overlay source that cannot be prepared SHALL
refuse, without changing mode. Invalid mask targets or contradictory
bindings SHALL be diagnosed before user code when they cannot be enforced.

A masked file's host content SHALL be unavailable and its host bytes and
directory entries SHALL remain unchanged through symlinks, hard links,
overlapping binds and replacement attempts. Preparation SHALL not read or
copy a masked secret into private scratch. Under Seatbelt, opening a present
masked entry at every admitted direct or alias path SHALL fail with `EACCES`
or `EPERM` and return no content. An absent mask SHALL remain absent and SHALL
not create a host file. Ordinary unmasked neighbors SHALL remain usable. An
unsafe alias or overlap SHALL refuse before payload execution. This denied-read
observable SHALL be named as denial; namespace's existing mask behavior is
unchanged.

#### Scenario: Read-only and writable binds differ only as declared
- **WHEN** each hands path reads and modifies test-owned `ro` and `rw` binds outside the worktree
- **THEN** both reads succeed, the read-only write fails with unchanged host bytes, and the writable write reaches only the declared unmasked host file

#### Scenario: Credentials remain hidden under every bind mode
- **GIVEN** synthetic credential files masked in `ro`, `rw` and `overlay` binds, plus ordinary readable neighbors
- **WHEN** commands read the neighbors, read each credential by direct and alias paths, and try replacement, deletion and rewriting of the masked entries
- **THEN** neighbors remain usable, direct and alias reads of present masked entries fail with `EACCES` or `EPERM` and return no content, and all host credential bytes and directory entries remain unchanged

#### Scenario: Present Seatbelt masks deny reads
- **GIVEN** a present masked credential with nonempty synthetic bytes and an ordinary neighbor
- **WHEN** each real hands path opens the credential through its admitted direct and alias paths
- **THEN** every credential read fails with `EACCES` or `EPERM`, no credential byte is returned, the neighbor is readable, and a successful empty or content-bearing read fails this Seatbelt scenario

#### Scenario: Accepted denial still needs native proof
- **WHEN** a candidate claims denied-read masks from the accepted ruling, a generated profile, a mock or a Linux-only test
- **THEN** SEATBELT-R2 stays open and activation remains fenced until native direct, alias, overlap and mutation adversaries pass through both hands paths

#### Scenario: Missing masks do not create host files
- **WHEN** a declaration masks a file absent from the source, including after an earlier call created a same-named private overlay file
- **THEN** the host file stays absent and the next call cannot reveal host credential content through that name

#### Scenario: Unsupported binding semantics refuse visibly
- **WHEN** the implementation cannot preserve a bind or mask, including an alias or overlap conflict
- **THEN** it names the bind and unsupported operation before executing the payload; it neither changes `overlay` to `rw`/`ro` nor removes a mask

### Requirement: Overlay writes persist for the seat and never modify the host lower layer

Before the first payload of a seat attempt, each declared overlay SHALL be
materialized as one complete seat-private snapshot and exposed only through
the corresponding absolute locator in `BROKKR_HANDS_OVERLAYS`. The mapping
SHALL be stable across calls in that seat, distinct and inaccessible across
seats and attempts, and applicable to arbitrary declared paths. The declared
host path SHALL not become the writable view or an alternate route to the
snapshot. Creates, edits, deletions, renames and executable/cache updates at
the locator SHALL persist across calls in that seat and never change the host
source. Host changes after snapshot completion SHALL remain invisible. A new
attempt on resume or rerun SHALL start with a new snapshot. Cleanup SHALL wait
for all payload processes to end and remove private state without following a
seat-controlled link or synchronizing changes to the host.

Snapshot preparation SHALL copy bytes without retaining host hard links. It
SHALL recreate a symlink only when its fully resolved target remains within
the same admitted snapshot, recreate internal hard-link relationships using
private inodes, and refuse escaping, external, racing or ambiguous link and
overlap layouts before spawn. Masked entries and every alias to their host
identity SHALL be excluded from copied data and denied at their locator path.
A coherent snapshot failure SHALL execute no payload. `CARGO_HOME` and
`NPM_CONFIG_CACHE` SHALL select their matching generic locator; neither is a
special-case substitute for arbitrary overlays.

The manifest SHALL continue to pin the declared path, mode and masks. The
ephemeral locator map, scratch spelling and snapshot bytes SHALL not enter
portable bundle identity. Duplicate declared paths, two overlays competing
for one well-known redirect, or any mapping that is not one-to-one SHALL
refuse with the affected declarations. The shipped masked Cargo overlays and
the node recipe's npm overlay remain acceptance inputs.
#### Scenario: Explicit locators cover arbitrary overlays
- **GIVEN** Cargo, npm and an unrelated declared overlay whose lower sources contain ordinary files, internal links and masked entries
- **WHEN** each hands path reads `BROKKR_HANDS_OVERLAYS` and uses every returned locator
- **THEN** the array order, zero-based original bind indexes and three exact fields match the declarations, each locator exposes its safe private snapshot, Cargo/npm variables equal their matching locators, and no direct source or masked alias reveals host content

#### Scenario: Overlay mutation matrix survives the next call
- **WHEN** the outside test records the host before a first call creates, edits, deletes, renames and updates an executable at a locator, verifies that baseline remains unchanged, then changes the host itself and records a post-change baseline before a second call observes both
- **THEN** the second call sees the seat changes and its pre-change snapshot rather than the later host update, while comparison with both baselines proves neither payload call caused any host byte, name, link or mode change

#### Scenario: Seats and attempts never share overlay state
- **WHEN** another seat and a resumed or rerun attempt declare the same overlay after the first seat writes its snapshot
- **THEN** each receives a different inaccessible locator initialized from its own preparation-time source and sees none of the first seat's private writes

#### Scenario: Ephemeral locators do not change portable identity
- **WHEN** identical declarations compile or run with different scratch roots and locator spellings
- **THEN** manifest identity remains equal because it pins declared paths, modes and masks, while the runtime locator maps differ

#### Scenario: Seat exit and failure do not publish overlay changes
- **WHEN** a seat exits normally, times out, is cancelled or fails snapshot preparation
- **THEN** no overlay update reaches the host, cleanup waits for the no-survivor guarantee, and a new seat or attempt begins independently

### Requirement: Git metadata protection prevents host hook and configuration persistence

Before executing a command, the boundary SHALL identify the worktree's
actual git administration paths, including the common directory and linked
worktree directory. Git object, index, ref and log writes needed for normal
worktree operation SHALL remain possible, while host hook storage and
protected configuration SHALL not be mutable by the seat. Protection SHALL
cover existing and initially absent `config`/`config.worktree`, applicable
configuration include targets, effective `core.hooksPath` destinations
(including relative destinations), and gitdir/commondir indirections whose
replacement would redirect host Git to seat-written configuration or hooks.
A writable ancestor, worktree path, declared bind, symlink or hard-link
alias SHALL not bypass this protection. If the layout or an effective
configuration cannot be protected, the command SHALL refuse with a concrete
reason rather than expose the common git directory wholesale.

Setting `core.hooksPath` or signing configuration in the child environment
is useful for ordinary Git behavior but SHALL NOT be the protection against
raw filesystem writes or commands that clear/override those entries. The
mechanism SHALL prevent planting a program that the host's subsequent Git
invocation executes; preserving only the hooks/config files present at
preparation is insufficient if replacements or new files can bypass it.

#### Scenario: Direct and replacement attacks leave protected paths unchanged
- **WHEN** commands in primary and linked worktrees attempt file writes, `git config --local`/`--worktree`, creation of absent config, config-lock-and-rename, unlink/recreate, parent-directory rename and hook creation
- **THEN** no operation changes host protected bytes or routing metadata, permitted Git object/ref writes still work, and host-side verification includes directory entries as well as file hashes

#### Scenario: Effective hooks and includes are protected through aliases
- **GIVEN** disposable repositories with default hooks, relative and absolute `core.hooksPath`, worktree-specific configuration and configuration includes, with alias paths inside writable grants
- **WHEN** hostile commands try to modify hook bodies, include targets or the routing files by each spelling, including after clearing git environment overrides
- **THEN** no host Git invocation can be made to execute a planted sentinel program; unsupported layouts refuse before the command and identify the exact conflict

#### Scenario: A writable config parent is not an atomic-replacement loophole
- **WHEN** the command writes a new config or hooks directory in an allowed location and attempts to replace a protected target or its ancestor by rename or a link
- **THEN** protection holds at the destination and routing metadata, and a subsequent host-side Git check observes the original configuration and no planted hook

### Requirement: Git remains usable for unsigned commits in linked worktrees

Ordinary status, staging and unsigned commits SHALL work in both primary
and linked worktrees under the resolved seat identity. The host SHALL see
the intended object/ref update. Before each call, Brokkr SHALL supply an empty
seat-private hooks directory and route ordinary Git to it. The original and
effective host hooks paths and their aliases SHALL deny reads and writes. Host
hooks and signing programs SHALL not execute, and host hooks, configuration
and routing metadata SHALL remain unchanged. The private routing is ordinary
Git behavior, not the protection against hostile commands that clear or
override it; the independent filesystem enforcement above remains mandatory.
This accepted view qualifies as full peer only after native primary and linked-
worktree adversaries pass through both hands paths. A successful benign commit
or environment override alone SHALL not close SEATBELT-R4.

#### Scenario: Linked-worktree commit is visible and unsigned
- **GIVEN** a primary repository, a linked worktree outside it, synthetic identity, enabled signing with a sentinel signing program, and existing sentinel hooks
- **WHEN** each hands path stages and commits a worktree change
- **THEN** the host sees the expected author/committer, message and content on that worktree's ref, the commit has no signature, no hook/signing sentinel ran, and protected metadata is unchanged

#### Scenario: Private hooks satisfy the accepted view only with independent protection
- **GIVEN** primary and linked worktrees with sentinel hooks, signing, includes, relative and absolute hooks paths and writable aliases
- **WHEN** each hands path commits through the empty private hooks directory, reads original hooks, clears Git overrides, and attempts every direct, config, alias, unlink, rename and replacement attack
- **THEN** ordinary Git completes unsigned, every original hook read is denied, every raw hook/config/routing mutation fails, later host Git executes no sentinel, and host bytes and directory entries match; any gap leaves SEATBELT-R4 open and blocks peer activation

#### Scenario: No repository does not grant a git directory
- **WHEN** a workspace command runs in a non-repository directory
- **THEN** ordinary file operations work and no unrelated host git directory or configuration becomes readable or writable
### Requirement: Execution inputs stay controlled without Linux mount assumptions

The actual engine executable and the bundle layer owning an exec dispatch
SHALL be available read-only under Seatbelt, including scripts and helpers
outside the worktree and inherited declaring layers. Paths handed to the
child SHALL identify those intended inputs; no `/runtime/bundle` mount or
host path rewrite SHALL be assumed to exist without a real realization.
Supporting one binary or bundle SHALL not grant an entire parent home,
source checkout or installation directory. Writable worktree or bind
aliases SHALL not defeat protected-input restrictions. The result path
SHALL still name the seat's actual result file in its writable worktree.

The engine's launch tool and initial interpreter resolution SHALL come
from controlled paths, not a seat-planted same-named executable. This does
not pin every program a command chooses to run: arbitrary repository build
scripts remain hostile code confined by the boundary. Decision 0048's
script-directory integrity scope, unboxed inherited-PATH limitation and
re-walk/exec interval SHALL not be silently strengthened into claims about
all interpreter behavior or all host helpers.

#### Scenario: An inherited script outside the worktree runs its own helper
- **GIVEN** an inherited exec seat whose script and helper are in the declaring ancestor bundle outside the operated worktree, with a misleading sibling script in the worktree
- **WHEN** its compiled Seatbelt dispatch runs
- **THEN** the intended script/helper execute using the controlled engine binary, write the seat's valid result file, cannot modify their protected inputs, and do not read unrelated parent files

#### Scenario: A worktree containing its bundle remains useful
- **WHEN** the operated worktree contains the bundle, execution binary or symlink aliases of protected inputs
- **THEN** ordinary source and result-file writes still work while protected-input mutations fail or the precise incompatible layout is refused before execution; no broad grant erases protection

### Requirement: Native lifetime feasibility precedes full implementation

The named candidate is the **per-invocation transient launchd lease pair** in
an unprivileged per-user bootstrap domain. One uniquely labelled payload job
SHALL launch the literal system `/usr/bin/sandbox-exec` and payload; that
job's job/process coalition is the candidate payload lifetime domain. A second,
separately launchd-owned guard job SHALL observe a private engine-liveness
channel whose write end the payload SHALL NOT inherit. The guard SHALL NOT be
a member of the payload job it must terminate, inherit payload stdio or trust
payload-writable state. Timeout and explicit cancellation SHALL ask the guard
to `bootout` the payload job; liveness EOF SHALL trigger the same teardown
after abrupt engine/supervisor death. The guard SHALL remain alive through
payload teardown, establish payload-domain quiescence before private-state
cleanup, and then unregister itself. This is an unproven feasibility candidate,
not an enforcement claim. Before any dependent production Seatbelt profile,
bind, Git or runtime implementation proceeds, a narrowly scoped macOS probe
limited to the lease pair, native adversary and observer helpers, and the
minimum experimental policy needed to keep the guard outside payload authority
SHALL demonstrate that the candidate admits ordinary shell children and leaves
no setsid or double-fork descendant alive after timeout, cancellation or
engine/supervisor `SIGKILL`.

The observer SHALL live outside both jobs, record every descendant PID and
non-reusable start identity plus a moving heartbeat before the trigger, and
verify process absence within the five-second teardown bound, an unchanged
heartbeat for one further second, and disappearance of both transient job
labels. The required lifecycle SHALL be `prepared -> guard-registered ->
payload-started -> terminating -> payload-quiescent -> private-state-removed
-> guard-unregistered`. If quiescence cannot be established, the invocation
SHALL fail and leave private state quarantined for an engine-owned reaper that
re-establishes lease identity and quiescence; cleanup SHALL NOT trust or reuse
a stale PID, label or filename.

The evidence SHALL name candidate revision, macOS version and architecture,
the per-user bootstrap domain, both unique job labels and exact commands,
positive controls, trigger, exit statuses and durable logs. It SHALL also
demonstrate that payload code cannot signal or impersonate the guard, use its
private channel, boot out a peer invocation, or register an independently
surviving launchd job through authority exposed by the experimental profile.
If launchd provides only process-group cleanup, requires private SPI, a
privileged entitlement or global host mutation, loses a descendant, cannot
keep the guard separately owned through payload teardown, or cannot cover
supervisor death, the probe SHALL fail, keep SEATBELT-R3 open and stop
dependent implementation. PID polling, `kqueue`, source reasoning, mocks,
Linux execution and launcher smoke tests may assist observation but SHALL NOT
satisfy the guarantee.

#### Scenario: The named launchd candidate must survive the detach adversary
- **GIVEN** a native helper that proves its ordinary-child positive control, then forks, calls setsid, double-forks, ignores termination signals and reports identities while writing a heartbeat
- **WHEN** the isolated feasibility probe triggers timeout, cancellation and abrupt supervisor death in separate runs
- **THEN** guard-owned teardown ends every reported payload descendant inside the bound, both transient labels disappear in order, and the external observer sees one second of quiet; otherwise the probe fails with SEATBELT-R3 open and no dependent implementation is authorized

#### Scenario: The guard remains outside the payload job
- **GIVEN** the separately labelled guard job has established its private liveness channel and the payload job is running
- **WHEN** timeout, explicit cancellation or supervisor-liveness EOF causes the guard to boot out the payload job
- **THEN** the guard remains alive after payload bootout, proves payload-domain quiescence before cleanup, unregisters only after cleanup, and an observer outside both jobs verifies the ordering; a guard removed with the payload fails SEATBELT-SPEC-LIFETIME-TOPOLOGY and cannot authorize dependent implementation

#### Scenario: The payload cannot acquire guard or peer-job authority
- **GIVEN** the guard endpoint, labels and control state are engine-owned and payload code is confined by the experimental profile
- **WHEN** the payload attempts to signal or impersonate the guard, use its private channel, boot out a peer invocation or register an independently surviving launchd job
- **THEN** every attempt is denied while the ordinary-child control remains usable; any successful interference fails the probe, leaves SEATBELT-R3 open and stops dependent implementation

#### Scenario: Observation is not containment
- **WHEN** a candidate only polls descendant PIDs, watches them with `kqueue`, or signals the original process group
- **THEN** it cannot pass the supervisor-death case or activate Seatbelt, even if timeout returns and the directly observed child exits


### Requirement: Deadlines and teardown bound the entire command lifetime

The workspace deadline and the boxed exec seat's declared attempt deadline
SHALL bound the whole payload tree. Timeout, cancellation, server shutdown
and normal completion SHALL not leave descendants executing or holding
output pipes indefinitely. Tests SHALL include grandchildren, background
children after the immediate parent exits, ignored termination signals and
double-fork/session-detach attempts. The mechanism SHALL prevent those
operations or terminate their descendants within the same bound while
still supporting ordinary shell, build and Git children. Killing only the
immediate child or the initial process group is not sufficient. There is
no declared hands policy distinguishing commands that may detach: all
commands are arbitrary hostile code. A check over command text, a caller
promise not to daemonize, or refusing every forking command SHALL NOT
satisfy this requirement.

R3 is a feasibility prerequisite for the boundary as a whole. The transient
launchd lease pair SHALL remain isolated probe code until the preceding native
gate passes; a generic unsupported-policy refusal cannot make Seatbelt built.
No declaration distinguishes commands that may detach, and no surviving
payload is permitted. Any proposal to weaken that guarantee must return
upstream with the exact deadline, cancellation, supervisor-death and scratch
consequences. Private state SHALL not be reused or removed while payloads can
still mutate it. The mechanism SHALL not be described as PID, IPC, UTS or user
namespace isolation.

#### Scenario: Timeout stops descendants and returns within a fixed budget
- **WHEN** a native test uses a 300 ms workspace timeout and a one-second exec-seat deadline with descendants holding pipes and repeatedly writing a worktree heartbeat
- **THEN** each invocation reports timeout within its deadline plus five seconds, all observed descendants cease, and the heartbeat stays unchanged during a further one-second observation; an outer test watchdog fails and cleans up a hang rather than counting it as success

#### Scenario: Parent exit and cancellation leave no background payload
- **WHEN** the direct parent exits while background or detached descendants retain stdout/stderr, or the controller closes/cancels the seat, including abrupt supervisor death
- **THEN** the call/seat and its payload end within the same five-second teardown budget and no descendant keeps a pipe or continues host-visible worktree writes

#### Scenario: Session detachment is part of the ordinary command threat model
- **GIVEN** a native helper that forks, calls setsid in the child, forks again, reports its descendant identities to the outside test observer, ignores termination signals and writes a heartbeat while holding output pipes
- **WHEN** it runs under each hands path and the deadline expires or the supervisor is killed
- **THEN** every reported descendant is terminated or its creation was prevented, the heartbeat stops within the teardown bound and remains unchanged for one second, and the observer's watchdog cleans up any escape as a test failure; signaling only the original process group cannot pass through lost observations

#### Scenario: A group-only supervisor cannot activate Seatbelt
- **WHEN** a candidate terminates the initial process group but has no mechanism or native evidence covering the detachment helper
- **THEN** R3 remains an unmet boundary-wide prerequisite, Seatbelt stays unbuilt, and neither a timeout return nor a refusal of arbitrary commands counts as the no-survivor guarantee

### Requirement: Output stays bounded without corrupting driver or MCP transport

Workspace calls SHALL retain the existing request bounds: command length
1 through 16,384 bytes, timeout default 30,000 ms and accepted range 100
through 600,000 ms. Each captured stdout/stderr stream SHALL retain at most
262,144 payload bytes while continuing to drain, with explicit truncation
reporting and the exit/timeout outcome. Boxed exec SHALL preserve driver's
stdio and result-file contract, without buffering unbounded command or
diagnostic output in a supervisor. Captured diagnostics SHALL use the same
262,144-byte bound; oversized protocol data SHALL be handled as a bounded,
explicit failure rather than silently truncating a valid protocol frame.
The proposed semantic decision SHALL record any new limit visible to exec
callers. Exact formatting of normal existing output SHALL stay compatible.

#### Scenario: Noisy commands cannot grow capture without bound
- **WHEN** native workspace and exec payloads continuously write more than the capture budget to both streams and then finish or time out
- **THEN** capture remains bounded, truncation or an explicit bounded protocol failure is reported as appropriate, draining does not deadlock, and no successful driver result is forged by truncation

#### Scenario: Ordinary exec exits and result delivery are preserved
- **WHEN** a Seatbelt exec command returns a chosen nonzero exit and another writes a valid seat result through the driver
- **THEN** the exit is preserved and the valid result is accepted through the existing protocol; MCP invalid-argument requests execute no command

### Requirement: Seatbelt identity and evidence describe the path that actually ran

The existing realm, manifest, effect-boundary, seat-record and readout
contracts SHALL apply unchanged to the new path unless a necessary
extension is published as a new version beside frozen files. The manifest
SHALL pin `seatbelt` for the same site keys as hands; temporary profiles,
private directories and host-dependent discovery SHALL not silently enter
portable identity. Changing only the boundary SHALL change the digest;
repeated compilation with identical inputs SHALL not. Any moved witness or
compose pin SHALL come from measured compile/test output with its cause.

Effect entries, engine-stamped model records, seat inputs, prompts,
terminal/TUI/web rows, costs, compare and export SHALL agree on `seatbelt`
for Seatbelt sites. Delivery summaries SHALL preserve the existing
boxed/unboxed derivation for those sites. Failed starts or attempts SHALL
remain failures; a planned boundary in an attempt record is not evidence
that a successful sandbox execution occurred. Hands-less and historical
records retain their existing sentinel/absence behavior. A run with a
`harness` or `open` gate remains unboxed under the existing derivation.

The accepted 0046 addendum permits denied host-hook access plus an empty
private hooks directory, conditional on independent hook/config/routing
write protection and native primary/linked-worktree adversaries. These are
full-peer target scenarios; accepting the observable is not proving it.
Production start SHALL retain the unbuilt fence until all native obligations
pass. No harness-grade fallback is authorized.

#### Scenario: A real Seatbelt gate round-trips through the record
- **WHEN** a native Seatbelt exec gate succeeds and its journal is exported and verified
- **THEN** the manifest, effect entry, finishing checkpoint and successful model-bearing result name `seatbelt`; all model readouts agree, the run is not rendered unboxed on that gate's account, and driver-supplied false boundary stamps cannot replace the engine's word

#### Scenario: Unproven hooks protection cannot produce a peer gate
- **WHEN** the hooks mechanism lacks native protection evidence, even with a successful private-hooksPath commit experiment
- **THEN** production Seatbelt start refuses before a journal row or seat spawn; no successful Seatbelt record, hands-boxed prompt or delivery vouch is produced from that experiment, and R4 stays an open evidence residual

#### Scenario: Accepted hook semantics do not bypass the evidence gate
- **WHEN** a candidate implements the private hooks routing but has not passed the raw-write matrix on both worktree forms and hands paths
- **THEN** Seatbelt remains unbuilt and no peer record is emitted; the accepted policy answer is not counted as native protection evidence

#### Scenario: Compilation pins a word and not a scratch directory
- **WHEN** identical inputs compile twice with different temporary directories and then under `namespace` instead of `seatbelt`
- **THEN** the Seatbelt digests agree, the namespace digest differs, and frozen contract/corpus bytes remain unchanged

#### Scenario: A missing launcher is not successful boundary evidence
- **WHEN** an admitted Seatbelt attempt cannot execute its launcher
- **THEN** it is recorded and rendered as failed without a successful result or fabricated Mac measurement, and existing historical records still show absence when no boundary was recorded

### Requirement: Semantic choices and native evidence gate acceptance of the slice

Council design SHALL reconcile every position explicitly under `## Decisions`
and identify the concrete git, bind/mask, overlay, controlled-path and
process-lifetime mechanisms, alternatives and residual differences. New
semantic choices SHALL have a focused decision document with status
`proposed`. An observed difference from an accepted guarantee SHALL name
the precise operator question; absent a ruling the guarantee stands and
unsupported operations refuse. That refusal SHALL NOT discharge any
unimplemented required feature. The accepted rulings settle R1, R2 and R4,
but SEATBELT-R1 through R4 SHALL each close with qualifying native evidence
before the activation fence is lifted. A finding owned by an earlier artifact SHALL
be returned upstream and dependent artifacts kept coherent; a successful
OpenSpec syntax check SHALL not be presented as resolution of those facts.

Acceptance SHALL require behavioral/adversarial tests using actual
`sandbox-exec` on the existing macOS CI runner. Required native tests
SHALL fail if the system tool is missing/unusable, if the runner is already
boxed so the tests cannot execute, or if no native cases run. String/argv
checks and fake commands are useful unit tests but SHALL NOT count as Mac
measurements. The native suite SHALL cover every adversarial family above
through both entry paths, with disposable sentinels and positive controls,
without using real credentials, publishing, or starting a Brokkr delivery
run from inside a seat. Engine integration tests invoked by the controller
or CI outside a seat may drive disposable deterministic test runs.

Local preparation SHALL run applicable host-independent/Linux regressions,
`cargo test --workspace` (the all-features locked CI invocation also serves
this check), formatting, clippy, and compilation of `bundles/self` and
`bundles/verify` when tools are available. Exact coverage SHALL keep literal
nonzero 100% source-line/branch/function equality and its pinned compiler;
no exclusions, denominator changes or lowered gate can substitute for tests.
It SHALL be run by CI or the controller outside the workspace box, because
namespace tests cannot nest here. The design SHALL keep shared policy,
composition and lifecycle decisions
compiled and executable on Linux through injected host, path/probe and
process outcomes as recorded under Decisions below. No target-gated
Seatbelt production module, coverage attribute, name-based exclusion or
weakened denominator SHALL substitute for these tests. Actual Darwin
execution remains the native suite's separate obligation. Any required
native binding that cannot fit this seam SHALL be returned with its exact
coverage gap and a proposed additional native measurement before adopting
it; Linux coverage SHALL never be claimed as coverage of uncompiled code.

The evidence handoff SHALL identify the candidate commit, host OS/version
and architecture, system launcher, compiler, commands, test names/counts,
results and CI links/logs, plus every pending measurement and its owner.
Actual Mac measurements and host exact coverage SHALL stay explicitly
pending until their real results exist. Preparing code, tests and guides
on Linux does not make the overall slice complete. Evidence predating a
relevant change SHALL not be attributed to the final candidate.

#### Scenario: Hook protection needs evidence beyond its accepted view
- **WHEN** a candidate uses denied host-hook access and an empty private hooks directory, or cannot protect a linked-worktree hook/config alias
- **THEN** design cites the accepted 0046 observable, requires native independent write protection, and records any alias gap as an open blocking residual without reclassifying the boundary

#### Scenario: Linux exercises both policy arms without claiming native enforcement
- **WHEN** the candidate's host-independent tests run on Linux with supplied Linux/macOS/Windows host facts, trusted and untrusted paths, probe success/failure/timeout, and child completion/failure/timeout outcomes
- **THEN** they exercise the same compiled production policy, composition and lifecycle decisions used for native Seatbelt, including refusal and cleanup arms; the handoff labels this logical coverage and still requires actual sandbox-exec behavior on macOS

#### Scenario: The exact gate cannot be satisfied by hiding Seatbelt source
- **WHEN** coverage is prepared for the candidate
- **THEN** shared Seatbelt production logic is present in the Linux source denominator, the pinned script still demands all lines, branches and logical functions, and any native-only binding coverage is identified separately instead of counted as Linux-tested; a coverage failure or unavailable external run remains pending/failing, never passed by exclusion

#### Scenario: Mac evidence remains pending after Linux preparation
- **WHEN** implementation and Linux checks are prepared on this controller without any real Mac run
- **THEN** the handoff says Mac behavioral evidence is pending, gives the CI test invocation and expected cases for the controller, lists any unavailable local checks, and makes no overall slice-complete claim

#### Scenario: Required macOS tests cannot silently skip
- **WHEN** the required macOS CI suite has no usable `sandbox-exec`, is accidentally run inside an existing hands box, or selects zero Seatbelt tests
- **THEN** the CI check fails with the missing prerequisite or zero-test reason; no missing-tool return, ignore marker, fake tool or skipped success can satisfy acceptance

#### Scenario: The controller supplies final candidate evidence
- **WHEN** real native and exact-coverage runs exist for the candidate after the implementation is complete
- **THEN** their actual outcomes and links replace only the corresponding pending entries; failures remain actionable findings, and unrelated historical channel versions, journals and evidence are unchanged

## Decisions

- **R1 — generic private snapshot locators.** Adopt the explicit
  `BROKKR_HANDS_OVERLAYS` JSON mapping because arbitrary paths need one
  transport that is not a Cargo-only convention. Declaration order and the
  three-field shape are deterministic; locator spellings remain ephemeral.
  External/escaping links, host hard links and ambiguous mappings refuse
  because preserving them could bypass masks or publish writes.
- **R2 — denied reads.** Adopt `EACCES`/`EPERM` with no returned content
  for present Seatbelt masks. Refute the older readable-empty oracle because
  the accepted addendum replaces it for Seatbelt; namespace is untouched.
- **R3 and SEATBELT-SPEC-LIFETIME-TOPOLOGY — launchd lease pair, not a
  claim.** Adopt two separately launchd-owned jobs in one unprivileged per-user
  bootstrap domain: the payload job is the candidate job/process-coalition
  containment domain, while the guard job remains outside it, observes private
  engine liveness, boots out the payload, establishes quiescence, cleans up and
  then unregisters. Refute the earlier inside-guard topology because payload
  bootout would remove the component required to verify cleanup. Process
  groups are rejected because `setsid` leaves them; PID polling and `kqueue`
  are rejected as the guarantee because observation is not containment. A real
  macOS feasibility pass precedes dependent implementation.
- **R4 — conditional full peer.** Adopt denied original hooks plus an empty
  private hooks directory for ordinary Git. Refute routing alone as protection:
  independent raw-write enforcement and native primary/linked-worktree
  adversaries are required. No harness-grade branch is commissioned.
- **R6 — shared decisions and explicit host adapters.** Policy construction,
  locator/environment composition, readiness verdicts and cleanup routing
  SHALL be ordinary Rust compiled on Linux behind narrow fact adapters.
  Darwin-only calls require separate native evidence; target-gating the whole
  decision module or lowering exact coverage is refused.
