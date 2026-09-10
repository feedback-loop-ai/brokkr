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

### Requirement: Probe payload startup is established separately

Before any native lifetime trigger, the exact payload executable SHALL pass
three staged controls: outside Seatbelt, under direct
`/usr/bin/sandbox-exec` with the exact experimental profile, and as the
launchd-owned payload job with the identical executable, argument structure and
normalized profile authority. Each isolated cell SHALL instantiate typed cell-
root and payload-root placeholders in the same template positions; those
required path values and the resulting concrete profile digests MAY differ.
The observer SHALL retain both the normalized template and every concrete
profile and digest.
Each stage SHALL reach an externally observed ready state, identify an ordinary
child and exit cleanly when directed. The preferred payload is a committed,
purpose-built native Rust helper with no repository-script or general-purpose
interpreter dependency. If an interpreter is retained for diagnosis, staged
differential controls or denial/system-log evidence SHALL identify each
required operation, narrow target, responsible process and consumer before a
bounded predicate is added. Native Seatbelt denial events are preferred. When
they are unavailable or insufficient, committed minimal helpers SHALL bracket
dynamic-loader, pre-main, first-write, executable lookup, child-spawn and clean-
exit behavior; bounded monotonic combinations MAY reveal jointly required
predicates but remain diagnostic. Each proposed predicate SHALL be rerun alone
against the justified baseline, which is the baseline half of the startup-rule
ledger defined by the next requirement. Every added predicate SHALL preserve
negative controls over guard/peer authority, credentials, host writes and
undeclared network access. Broadening an operation class merely until startup
succeeds is forbidden, and a passing broad combination SHALL NOT be copied into
the candidate without independent operation/target evidence for every member.
A predicate the candidate requires SHALL additionally be proven by removal:
replaying the exact candidate with that one predicate stripped SHALL fail the
payload closed before the startup verdict can pass, and a removal that still
starts or cannot be observed SHALL fail the cell. The stripped profile is
evidence only and SHALL NOT enter the candidate. The predicates that need a
removal control are exactly the diagnosis-admitted entries of that ledger. The
probe's bounded removal set SHALL list each of them verbatim as one normalized
rule unit. Baseline ledger entries are bounded by their recorded justification
and by the denial controls, not by removal. A host-independent test SHALL prove
that the normalized template's rule units equal the disjoint union of the
baseline entries and the removal set. An unlisted unit, a unit in both halves
or a listed unit the template lacks therefore fails before any native run. At
runtime, a set entry the concrete profile does not carry records an unobserved
removal.
Removal controls are due on exactly the Seatbelt startup cells that reach a
nonce-authenticated `READY`. Such a cell SHALL carry an observed removal
control for every entry in the set. An empty, missing or unobserved record
SHALL fail that cell whatever its positive result. A removal is blocking only
when the stripped replay reaches no nonce-authenticated `READY`, from fresh
payload state. A Seatbelt cell that reaches no `READY` already fails startup.
It SHALL record each removal as not due. That record is never observed,
satisfied or blocking, and never a missing-record gap. No removal verdict,
stage-relative or otherwise, SHALL be drawn from a candidate that does not
start. Load-bearing status is a property of a starting candidate. A
predicate's necessity is unproven until a cell that reaches `READY` observes
its removal blocking. A later predicate may change that necessity, so the
removal replays on each new candidate that reaches `READY`. Different
candidates' stage progress, such as the pre-stage abort at `9f4c2c9` against
`executable` at `fa7ece5`, is diagnosis evidence that may name a predicate. It
is never a removal control. Each removal runs through direct
`/usr/bin/sandbox-exec` with the identical helper and argv. It uses that cell's
own concrete candidate profile and private root, with that one predicate
stripped. A launchd cell such as S3 does not bootstrap its removal as a launchd
job, because launchd ownership adds no Seatbelt authority and the profile is the
only layer the removal varies. A predicate whose named consumer exists only
under launchd ownership cannot be proven by a direct replay. It SHALL NOT enter
the candidate under this change.
The helper's ordinary-child stage SHALL be split into bounded, ordered sub-
stages: standard-stream setup for each of stdin, stdout and stderr, then the
spawn/exec call, then the child observed. A failure SHALL record the sub-stage it
reached and its OS error, so a refused step is named rather than inferred from
the stage that follows. When a Seatbelt cell fails at child spawn while the
unboxed control spawns the identical child, the discriminating evidence SHALL
include, under the exact candidate and identical helper, a cell that opens
`/dev/null` write-only without spawning, a cell that spawns with inherited or
pre-opened stdio, a cell that spawns with null stdio, and the exact candidate
plus exactly one named literal-scoped diagnostic
`(allow file-write-data (literal "/dev/null"))`. These cells are evidence and
SHALL NOT pass a startup verdict. If none of them attributes the refusal, the
spawn/exec sub-stage error and native denial evidence SHALL name the exec-side
operation and target, such as a `process-exec` or file read of the resolved
helper path, before any predicate is proposed. The ledger's baseline already
names the helper as an execution input, so an exec-side refusal of the helper
under another spelling corrects that input's canonical spelling rather than
admitting a new grant. A child-spawn predicate enters the candidate only when
this evidence attributes the refusal to it, only in literal-scoped form and
only as a diagnosis-admitted ledger entry with its own removal control; every
denial control SHALL rerun on the resulting candidate. The spawn SHALL NOT be
cured by a `/dev` subpath, a broad `file-write*`, any process unit other than
the seven the ledger lists (`process-fork` and six `process-exec` units), or
any Mach/IPC, service or network grant.
A separately labelled `allow default` run MAY diagnose that the restrictive
profile is the differing
layer, but it SHALL never be a candidate observation or authorize an allowance.

Startup and lifetime SHALL have separate verdicts. An abort, signal, nonzero
exit, crash-only launchd state or absent ready token fails startup and marks
every lifetime case not run. Registration, run/crash counters, empty output or
a still heartbeat do not establish that payload code executed and SHALL NOT
establish or reject containment.

Every launchd startup cell and repeated Gate A invocation SHALL have a unique
job label and private root. The observer SHALL prove the exact label absent
before bootstrap, preserve raw bounded stdout/stderr and status separately for
bootstrap, any explicit kickstart, `print`, payload exit and `bootout`, and
prove the label absent again before another cell runs. Loaded state, `READY`,
ordered stages, ordinary-child identity, terminal state, each available
run/crash/exit field and cleanup are distinct observations. Parsing SHALL be
field-wise and version-tolerant, and each field SHALL be parsed independently of
every other. The launchd terminal facts a startup cell requires are exactly
`state = not running`, a numeric `runs` and a numeric `last exit code`; the value
`last exit code = (never exited)` is a non-terminal observation, never an exit.
Measured `launchctl print` output on macOS arm64 omits `successive crashes` from
a terminal job that exited 0 or 2, so the crash counter is an optional fact.
When absent it SHALL be recorded as unknown with the raw sample that omitted it,
SHALL NOT be synthesized as zero and SHALL NOT by itself fail the cell. When
present it SHALL be recorded, and any nonzero value SHALL fail the cell. A
printed terminating-signal field is likewise recorded when present and fails
the cell. A missing or unparsable required field SHALL remain unknown and fail
the cell, but SHALL NOT erase independently observed facts or be converted into
a synthesized nonzero or clean exit. A launchd cell's exit fact SHALL be the
observed `last exit code`, never the helper's intent. A launchd cell passes
startup only when `runs = 1`, that exit equals the directed exit and the helper
independently supplies the nonce-authenticated `READY`, exact stages and
ordinary child; no launchd field substitutes for those helper facts. The
evidence SHALL keep
the raw `launchctl` text that caused each parsed or unknown value. An order-
dependent result, stale registration or failed bootstrap is a measurement
failure, not a payload or lifetime verdict.

Every job fact the probe derives from a `launchctl print` sample, in Gate A or
Gate B, SHALL come from the job's top-level dictionary. That dictionary holds
the entries at brace depth one inside the sample's single outermost
`<service-target> = {` block. An entry inside any nested `key = {` block, such
as `arguments`, an environment block, `resource coalition` or
`jetsam coalition`, belongs to that block. Nested blocks SHALL NOT supply,
complete or override a job fact. A top-level key that is missing stays unknown
even when a nested block carries the same key. The measured fa7 prints show
why: they repeat `state = active` and `active count = 1` inside both coalition
blocks while the job's own top level reads `state = not running` and
`active count = 0`. A key
that appears more than once at top level SHALL make that fact unknown, whether
or not the values agree. A duplicated required fact fails the cell, and so
does a duplicated crash counter or terminating signal. A sample
with no outermost block, more than one outermost block or unbalanced braces
has no top-level dictionary, so every fact it would supply is unknown. The
parse SHALL NOT depend on the order of keys or blocks: a first-match scan that
is correct only because launchd prints the top-level `state` before the
coalition blocks does not conform. Nested entries SHALL be recorded as raw
evidence attributed to their block path. A coalition's `state` or
`active count` SHALL NOT be treated as job state, liveness, quiescence or
survivor evidence, and it neither passes nor fails a cell by itself. A
top-level `active count` is recorded when present; it is not a required
terminal fact and never replaces the external observer's survivor set. The
retained fa7 raw samples (S2 while running, S2 terminal and S3 terminal) are
copied verbatim into the probe's own test data, not into the frozen
`fixtures/` corpus, and serve as the parser's regression fixtures.

Native CI `34433461814` at candidate
`6a19a6f4ab9bd30b47537de1a649949cd1099d01` is retained as a failed startup
measurement on macOS 26.6.2 arm64: direct sandboxed `/usr/bin/python3`
aborted with `SIGABRT` and empty output, while the launchd payload registered,
ran once, recorded one successive crash and produced no heartbeat. The cause is
not established. No lifetime observation from that run is admissible and
SEATBELT-R3 remains open.

Native CI `34441725835` at candidate
`8c53dcecaef414938b3abfb8911a77d9ec958f23` is retained as a second failed
startup measurement on the GitHub `macos-latest` arm64 runner; the log does
not record the macOS version. The committed native helper digest
`e925083d55a9c8b0` passed direct unboxed startup, then the identical helper and
argv aborted with signal 6 before `READY` under the exact profile. A labelled
`allow default` diagnostic exited zero and authorizes nothing. Repeated
launchd cells alternated between bootstrap error 5 and observations with
missing run/crash facts, so neither launchd control is established. Gate B was
not run; the startup cause and all lifetime properties remain unresolved.

Native CI `34449331270` at candidate
`9f4c2c944cac217ccb8dc055971cc62614313ed4` is retained as a third failed
Gate A measurement on the GitHub `macos-latest` arm64 runner. Generic macOS
and Windows workspace tests passed. S0 reached the exact stages, identified an
ordinary child and exited cleanly. S1 reached no stage and exited by signal 6;
all seven single-class differentials also failed, while only the labelled
`allow default` diagnostic started. S2 produced no parseable not-running state
and is an observation refusal, not proof of execution or nonexecution. S3
produced no `READY` or stages and reported not-running, one run and one crash.
Denial controls were unobserved and Gate B was not run. The pre-stage abort is
attributed to the missing root-inode read: the profile granted subpath reads but
not `(literal "/")`, which `dyld` needs while initialising a dynamically linked
process, and the labelled `allow default` control is the positive contrast for
exactly that layer. The report's S1/S3 raw profile-digest mismatch is also not
authority drift: unique private roots require different literal paths, so the
adapter compares structural profile identity and strips the required predicate
to prove it is load-bearing before the next native candidate.

Native CI `34457208029` at candidate
`fa7ece587178a46baa66a7310e0546bfb87a0857` is retained as a fourth failed
Gate A measurement on the GitHub `macos-latest` arm64 runner. With the root-inode
read granted, S1 and S3 reach `entry`, `payload-dir` and `executable`; the
ordinary-child spawn then fails with `EPERM` and the helper exits 2 without
`READY`. This is progress past the pre-stage abort, not a startup pass. Every
one-class diagnostic fails identically at child spawn, and none of them grants
any `file-write*`; only the non-admitting `allow default` control starts. The
credential-read, host-write and loopback-bind denial controls were observed
denied. They establish those three denials for this candidate only and close
none of R1–R4. No Seatbelt cell reached `READY`, so no removal control was
due. The empty `negative_controls` lists on S1 and S3 are that designed
not-due outcome, not a missing record. The unboxed S0 and S2 carry none by
construction. The root-inode read is therefore not yet proven load-bearing, and
it stays unproven until a Seatbelt cell that reaches `READY` observes its
removal blocking. S2 reached `READY`, the exact stages and an ordinary child. Its
raw terminal print shows `state = not running`, `runs = 1`, `last exit code = 0`
and no `successive crashes` line. S3's raw print shows `runs = 1` and
`last exit code = 2`, also with no crash line. The parser refused each whole
print on the absent counter, and the report erased those printed facts as
`runs=None` and `did not run`. That is a measurement defect, not a launchd
verdict. Gate B was correctly not run.

#### Scenario: Probe startup is established before lifetime triggers
- **GIVEN** the exact helper, argv and experimental profile intended for the lifetime matrix
- **WHEN** the outside-box, direct-sandbox and launchd-owned controls run in order
- **THEN** all three reach an externally observed ready state, demonstrate an ordinary child and exit as directed before any teardown trigger begins

#### Scenario: A non-starting payload is not lifetime evidence
- **WHEN** a direct sandbox payload aborts or a launchd job records a crashed run without the external ready observation
- **THEN** evidence records candidate, host, command, policy, status or signal, bounded output and launchd state, marks lifetime cases not run and leaves R3 open without a containment verdict

#### Scenario: A broad diagnostic never becomes an admitted profile
- **WHEN** the exact profile aborts before `READY` but a separately labelled `allow default` run of the identical helper and argv exits cleanly
- **THEN** evidence records only that the exact profile withheld some required authority, keeps every exact-profile cell failing, adds no broad allowance and requires operation/target attribution plus preserved denial controls before retry

#### Scenario: Startup diagnosis names operations and targets
- **WHEN** no single broad-class differential starts the helper and a bounded combination or minimal staged helper advances farther
- **THEN** the evidence identifies each newly required operation, narrow target, process and consumer from native denial or stage evidence, reruns each predicate against the justified baseline, and refuses to promote the broad family or combination itself

#### Scenario: Cell-private roots are not profile drift
- **WHEN** S1 and S3 instantiate the same normalized policy template with their distinct private cell and payload roots
- **THEN** the authority comparison passes only when the normalized rules and placeholder positions agree, while both concrete profiles, substitutions and different digests remain recorded; raw digest equality is neither required nor claimed

#### Scenario: Launchd startup cells are isolated and fully observed
- **WHEN** Gate A executes or repeats a direct/launchd and profile-off/on matrix
- **THEN** every launchd cell uses a never-reused label/root, proves absence before bootstrap and after cleanup, preserves each command's raw bounded status/output and reports missing job fields as unknown; a bootstrap error, stale label, inferred exit or result that changes with cell order fails Gate A

#### Scenario: An unknown launchd field does not erase other facts
- **WHEN** a launchd cell reaches `READY`, writes some ordered stages or produces bounded output but its terminal print omits or changes a required state, run or exit field
- **THEN** that field stays unknown and the cell fails, while every independently observed fact and the raw lifecycle command evidence remain present in the report

#### Scenario: An omitted crash counter stays unknown
- **WHEN** a terminal `launchctl print` shows `state = not running`, `runs = 1` and `last exit code = 0` with no `successive crashes` line, as the fa7 S2 print did
- **THEN** the cell records the state, one run and exit 0 as parsed facts and the crash count as unknown with its raw sample; no zero is synthesized, the absent counter alone does not fail the cell, and the same print with `last exit code = 2`, as in fa7 S3, records exit 2 and fails a clean-directed cell

#### Scenario: Required launchd terminal facts still fail closed
- **WHEN** a terminal print omits or cannot parse `state`, `runs` or `last exit code`, shows `last exit code = (never exited)`, shows a nonzero crash count or a terminating signal, or shows a run count other than one
- **THEN** that fact stays unknown or failing and the launchd cell fails, while every other parsed field, helper stage and raw sample remains in the report

#### Scenario: Launchd job facts come only from the top-level dictionary
- **GIVEN** the verbatim fa7 S2 terminal sample and S3 terminal sample, each with top-level `state = not running`, `active count = 0` and `runs = 1`, and `state = active` and `active count = 1` inside both coalition blocks
- **WHEN** the probe parses them
- **THEN** S2 yields state `not running`, one run, exit 0 and an unknown crash counter, and S3 yields the same with exit 2; both record top-level `active count = 0`, and each coalition's `state = active` and `active count = 1` is kept only as raw evidence under its block path, with no job fact taken from it and no duplicate declared; S3's coalition `active count = 1` does not count as a live or surviving payload even though its child never spawned

#### Scenario: The running fa7 sample is non-terminal
- **WHEN** the probe parses the verbatim fa7 S2 sample taken while the job ran, with top-level `state = running`, `active count = 1`, `runs = 1` and `last exit code = (never exited)`
- **THEN** it records a running, non-terminal observation with no exit fact and continues sampling; it does not treat that sample as terminal or clean

#### Scenario: Nested or duplicated keys never manufacture a job fact
- **WHEN** a sample lacks a top-level `state`, `runs` or `last exit code` while a nested block carries that key, repeats one of those keys at top level, puts the coalition blocks before the top-level keys, or has no single balanced outermost block
- **THEN** a missing or duplicated fact stays unknown and fails the cell, a reordered but otherwise well-formed sample parses to the same facts as the original order, no nested value is promoted to a job fact, and the raw sample and every other independently parsed fact remain in the report

#### Scenario: A launchd print never substitutes for helper facts
- **WHEN** a launchd print shows a clean terminal run but the helper's nonce-authenticated `READY`, exact stages or ordinary child is absent
- **THEN** the cell fails startup, and its exit fact comes only from the observed `last exit code`, never from the helper's directed intent

#### Scenario: Startup repair preserves least authority
- **WHEN** a staged control identifies a startup dependency
- **THEN** only the named dependency is added, all denial controls rerun, and a success caused only by broad file, Mach/IPC, service or network authority is rejected

#### Scenario: A required startup predicate is proven by removal
- **WHEN** a Seatbelt cell whose candidate grants a minimal predicate, such as the root-inode read `(literal "/")` that lets a dynamically linked payload initialise, reaches `READY`
- **THEN** the exact candidate with exactly that predicate stripped fails the identical payload closed before `READY`, the removal observation is recorded as blocking, and a removal that still starts the payload fails the cell


#### Scenario: The dynamic-loader root read is the named pre-stage predicate
- **WHEN** a dynamically linked payload aborts with `SIGABRT` before its first stage under a deny-default profile whose subpath reads do not cover the filesystem root
- **THEN** evidence names the root-inode `file-read-data` as the required predicate, grants only `(allow file-read* (literal "/"))` and never recursive `(subpath "/")`, and keeps `allow default` a non-admitting diagnostic

#### Scenario: A Seatbelt cell without an observed removal control fails
- **WHEN** a Seatbelt startup cell reaches a nonce-authenticated `READY` but its removal-control record is empty, missing, or lacks an observed blocking control for an entry in the removal set
- **THEN** the cell fails as unproven, whatever its positive result

#### Scenario: A non-starting Seatbelt cell owes no removal verdict
- **GIVEN** fa7's S1 and S3, which reach `entry`, `payload-dir` and `executable` and then fail the ordinary-child spawn with `EPERM` and no `READY`
- **WHEN** the probe reports their removal controls
- **THEN** each set entry reads not due because the cell reached no `READY`; the cell fails on its own startup facts; no stripped replay, stripped-run stages or cross-candidate contrast is recorded as blocking, satisfied or load-bearing; and the root-inode read stays unproven

#### Scenario: A stripped replay that reaches READY is not blocking
- **WHEN** a removal control's stripped replay reaches a nonce-authenticated `READY` and then exits nonzero, exits by signal or never records `return-clean`
- **THEN** the control is observed and not blocking and the cell fails; only a stripped replay that reaches no `READY` from fresh payload state is blocking, and neither a clean exit nor a failed later stage enters the blocking verdict

#### Scenario: A launchd cell's removal replays the profile directly
- **WHEN** S3 reaches `READY` as a launchd-owned job
- **THEN** each removal runs through direct `/usr/bin/sandbox-exec` with S3's own concrete profile minus that one predicate, from S3's private root with fresh payload state and the identical helper and argv, and no new launchd label is bootstrapped for it; a predicate whose consumer exists only under launchd ownership is not admitted

#### Scenario: The removal set is exactly the diagnosis-admitted predicates
- **WHEN** a host-independent probe test compares the removal set with the rendered candidate template and the startup-rule ledger
- **THEN** it parses the template into rule units and requires them to equal the disjoint union of the baseline entries and the removal set; every set entry appears verbatim as a diagnosis-admitted unit, baseline entries carry no removal control, and an unlisted, doubly classified or missing unit fails the test

#### Scenario: Child-spawn refusal is localized to its sub-stage
- **WHEN** a Seatbelt cell reaches `executable` and the ordinary-child spawn fails while the unboxed control spawns the identical child
- **THEN** evidence names the failing sub-stage (stream setup for a named stream, spawn/exec or child observation) with its OS error, and the no-spawn `/dev/null` write, inherited-stdio, null-stdio and single literal `/dev/null` write-data cells separate the stdio hypothesis from an exec-side refusal without any of them passing a startup verdict

#### Scenario: A named child-spawn predicate is literal-scoped and removable
- **WHEN** the discriminating cells attribute the child-spawn refusal to one operation and target
- **THEN** only that literal-scoped predicate enters the candidate, its own removal control strips exactly it and observes the identical payload fail closed, every denial control reruns, and no `/dev` subpath, broad `file-write*` or wider process, Mach/IPC, service or network grant is admitted

### Requirement: The experimental startup template is an audited rule ledger

One committed, typed startup-rule ledger SHALL account for every rule of the
experimental startup template, which Gate A and Gate B share. It lives beside
the removal set in the probe's shared model and is the only source of the
candidate's authority.

The ledger's unit of account is the **rule unit**: one `allow` form with one
operation and at most one filter. Units are written in the normalized template,
where the typed placeholders `<cell-root>`, `<payload-root>` and `<helper>`
replace the concrete cell root, payload root and helper path. A string equal to
one of those concrete paths, or beginning with it and then `/`, takes its
placeholder, longest path first. No other string is rewritten. `<helper>` SHALL
be instantiated with the helper's canonical path, because Seatbelt matches
resolved paths.

The normalization is closed. The template SHALL open with exactly
`(version 1)` and `(deny default)`. That frame is not a unit. After it, every
top-level form SHALL be an `allow` form written `(allow OPERATION FILTER...)`:
exactly one operation, a bare symbol in the first position, then zero or more
filters. A filter SHALL be simple: one filter name and exactly one argument,
which is a string literal, or the symbol `self` in `(target self)`. A form
with several filters normalizes into one unit per filter, because the form
grants its operation wherever any one of its filters matches. An unfiltered
form is one unit. The removal set uses the same unit, so a removal strips
exactly one unit even when the rendered template groups units in one form.
Nothing else normalizes. The check SHALL fail, and name what it found, for
each of these:

- an `allow` form with more than one operation, naming each operation after
  the first. Such a form is refused rather than split, and the template writes
  one operation per form;
- an action modifier such as `(with report)` anywhere in a form, naming the
  modifier;
- a compound filter (`require-any`, `require-all` or `require-not`), or any
  filter whose argument is not one string or `self`, naming the filter;
- any top-level form other than the frame and `allow` forms, naming the form.
  This covers `deny`, `allow default`, a second `version`, `import`, `param`,
  `define`, `if`, `debug`, `trace` and a bare atom;
- text the parser cannot normalize: an unbalanced parenthesis, a comment, a
  string that needs an escape sequence, or any token outside this grammar;
- the same unit rendered twice.

A parser that keeps only a form's first operation, skips a form or filter it
does not recognize, or unescapes a string does not satisfy this check. Filter
names outside the ledger need no separate vocabulary: a simple filter the
ledger does not list is an unlisted unit and fails the equality.

Every rule unit SHALL carry exactly one class.

- A **baseline** entry records a justification kind and a concrete
  justification. The kind is one of three:
  - a **hands element**, the Seatbelt image of authority that decision 0043,
    as `hands.rs` realizes it, already gives every boxed command;
  - an **execution input**, the exact payload executable or a typed probe root;
  - a **probe-harness need**, a named file or process the probe itself reads,
    writes or runs.

  A rule is not baseline because an earlier template carried it. A baseline
  entry claims no measured necessity.
- A **diagnosis-admitted** entry records its operation, narrow target,
  responsible process, consumer and the native evidence that admitted it. It
  names exactly one removal-set entry. Its filter SHALL name a single object: a
  `literal`; one named sysctl, IPC object or service (`sysctl-name`,
  `ipc-posix-name` or `global-name`); or `(target self)` for `signal`. An
  unfiltered unit, or a `subpath`, `prefix` or `regex` filter, SHALL NOT be
  diagnosis-admitted.

A hands-element entry SHALL be no wider than the image it names. It MAY carry
less than the whole element, never more. A **toolchain unit** images a
toolchain bind at the same path. It SHALL name that one bind and target exactly
that path. The check SHALL confirm that each named bind is a `--ro-bind-try`
source in the argv that `hands.rs` `box_argv` renders, read from that function
rather than from a copied list. A parent directory of several binds is not the
image of those binds, because it also covers every child the box does not
bind. The system-library units image the Linux library binds at macOS paths.
They are recorded as that one element with their own targets, not as
toolchain units.

The **justified baseline** is the baseline half of this ledger and nothing
else. The candidate's rule units SHALL equal the disjoint union of the baseline
entries and the removal set. `(allow process-fork)` is the only unfiltered
unit, because fork has no target. Any other unfiltered `allow` fails the check.

The candidate's process authority is exactly `(allow process-fork)` and six
`process-exec` units: the `<helper>` literal and one subpath each for
`/usr/bin`, `/usr/libexec`, `/usr/local`, `/bin` and `/sbin`. No other
`process-*` operation, and no `process-exec` unit on another target, is in the
candidate. Wherever this delta forbids process authority wider than the
candidate names, it reads against these seven units.

The candidate this change prepares carries these baseline entries:

| Rule unit | Kind | Justification |
|---|---|---|
| `(allow process-fork)` | hands element | 0043 ruling 1 runs each call as `bash -lc`, whose commands fork children. The probe's consumers are the ordinary child and the `setsid` and double-fork descendants. |
| `(allow process-exec (literal "<helper>"))` | execution input | `sandbox-exec` executes the exact helper under the profile. The helper re-executes itself for its child and descendant roles. |
| `(allow process-exec (subpath "/usr/bin"))` | hands element | The `/usr/bin` toolchain bind, whose programs 0043 ruling 1 lets `bash -lc` run. |
| `(allow process-exec (subpath "/usr/libexec"))` | hands element | The `/usr/libexec` toolchain bind, which holds helper programs the toolchain runs. |
| `(allow process-exec (subpath "/usr/local"))` | hands element | The `/usr/local` toolchain bind, where host-installed programs live. |
| `(allow process-exec (subpath "/bin"))` | hands element | The `/bin` toolchain bind. Gate B's escape, guard and peer adversaries run the real `/bin/launchctl`, so their denials measure launchd authority rather than a refused exec. |
| `(allow process-exec (subpath "/sbin"))` | hands element | The `/sbin` toolchain bind. |
| `(allow file-read* (subpath "/usr/bin"))` | hands element | The `/usr/bin` toolchain bind, read-only. |
| `(allow file-read* (subpath "/usr/lib"))` | hands element | The `/usr/lib` toolchain bind, read-only. It holds `dyld`. |
| `(allow file-read* (subpath "/usr/libexec"))` | hands element | The `/usr/libexec` toolchain bind, read-only. |
| `(allow file-read* (subpath "/usr/share"))` | hands element | The `/usr/share` toolchain bind, read-only. |
| `(allow file-read* (subpath "/usr/local"))` | hands element | The `/usr/local` toolchain bind, read-only. |
| `(allow file-read* (subpath "/bin"))` | hands element | The `/bin` toolchain bind. |
| `(allow file-read* (subpath "/sbin"))` | hands element | The `/sbin` toolchain bind. |
| `(allow file-read* (subpath "/System/Library"))` | hands element | macOS system libraries and frameworks, the image of the `/lib`, `/lib64` and `/usr/lib` binds. |
| `(allow file-read* (subpath "/System/Volumes/Preboot/Cryptexes/OS"))` | hands element | The OS cryptex, where current macOS keeps the dyld shared cache. It is the same element. |
| `(allow file-read* (literal "<helper>"))` | execution input | The committed helper bytes, which no other unit covers. |
| `(allow file-read* (subpath "<cell-root>/inputs"))` | probe-harness need | Immutable launch inputs, such as the guard and peer labels the denial adversaries target. |
| `(allow file-read* (subpath "<payload-root>"))` | probe-harness need | The payload's own stages, ready token, identities and heartbeat. |
| `(allow file-write* (subpath "<payload-root>"))` | hands element | The writable worktree's image, and the only writable location. It holds the payload state above. |
| `(allow file-read* (literal "/dev/null"))` | hands element | The box's private device set (`hands.rs` `--dev /dev`). It is also the ordinary child's null stdin. |
| `(allow file-read* (literal "/dev/urandom"))` | hands element | The same device set. |
| `(allow file-read* (literal "/dev/random"))` | hands element | The same device set. |

The toolchain units are one subpath per `hands.rs` bind, never the parent
`/usr`. The box creates an empty `/usr` and binds only its listed children, so
`(subpath "/usr")` would also cover `/usr/sbin`, `/usr/standalone` and every
other child the box leaves out. The library and data binds, `/usr/lib` and
`/usr/share`, carry read units and no exec unit, because the probe runs no
program from them. These `hands.rs` binds carry no unit, which only narrows
authority:

- `/usr/include`, because nothing inside the profile compiles C. A C
  toolchain on macOS reads its headers from the SDK that a production profile
  declares.
- `/usr/lib64`, `/lib` and `/lib64`, the Linux library paths. Their macOS
  image is the system-library element above.
- the `/etc` binds and the `passwd`, `group`, `hosts` and `nsswitch.conf`
  files, because the probe reads none of them.

It carries one diagnosis-admitted entry:

| Rule unit | Operation and target | Process and consumer | Evidence | Removal entry |
|---|---|---|---|---|
| `(allow file-read* (literal "/"))` | `file-read-data` of the root inode only | the helper, in `dyld` process initialisation | Without it, `9f4c2c9` aborted before any stage. With it, `fa7ece5` reached `executable`. | `root-inode-read` |

Its necessity stays unproven until a Seatbelt cell that reaches `READY`
observes its removal blocking. A literal `(allow file-write-data (literal
"/dev/null"))` that the child-spawn cells attribute SHALL enter this half with
its own removal entry. It does not enter the baseline, even though the box's
device set could name it, because the child-spawn attribution owns it. Until
then the candidate carries no `/dev/null` write.

Every unit of the measured fa7 template, `fa7ece5` `sandbox_profile`, SHALL
keep a recorded disposition. The units kept unchanged are the root-inode
read, the `/bin` and `/sbin` reads, the helper read, the three device reads and
the payload write. The units withdrawn or narrowed are:

| fa7 unit | Disposition | Reason |
|---|---|---|
| `(allow process*)` | Narrowed to `process-fork` and the six `process-exec` units listed above. | The family also carries process-information and code-signing operations on other processes. The hands box unshares pid, and no consumer needs them. |
| `(allow file-read* (subpath "/usr"))` | Narrowed to one subpath per `hands.rs` `/usr` bind: `/usr/bin`, `/usr/lib`, `/usr/libexec`, `/usr/share` and `/usr/local`. | The parent also covers `/usr/sbin`, `/usr/standalone` and every other child the box does not bind. |
| `(allow signal (target self))` | Withdrawn. | No consumer is named, and no measurement shows it is needed. |
| `(allow file-read* (subpath "/System"))` | Narrowed to `/System/Library` and the OS cryptex. | `/System` also holds `/System/Volumes/Data`, the data volume that also spells `/Users` and `/private`. No hands element grants it. |
| `(allow file-read* (subpath "/Library"))` | Withdrawn. | Host-wide preferences and keychain directories are not a hands element, and the probe names no consumer. Production toolchain paths come from the production profile's declarations, not from this template. |
| `(allow file-read* (subpath "/private/tmp"))` | Withdrawn. | Host tmp contradicts the private per-call `/tmp` of 0043 ruling 1, and no consumer is named. |
| `(allow file-read* (subpath "/private/var/tmp"))` | Withdrawn. | The same contradiction. |
| `(allow file-read* (subpath "<cell-root>"))` | Narrowed to `<cell-root>/inputs` and `<payload-root>`. | Guard and observer state needs no payload read. |
| `(allow file-read* (literal "/dev/dtracehelper"))` | Withdrawn. | It is not in the box device set, and no measurement shows it is needed. |
| `(allow sysctl-read)` | Withdrawn. | Unfiltered, it covers the process-table and process-argument sysctls that describe other processes. |
| `(allow ipc-posix-shm)` | Withdrawn. | Unfiltered, it opens shared memory with any same-user process. The hands box unshares ipc. |

The four units that `8c53dce` added when it replaced the aborting Python
payload of `6a19a6f` are disposed of as follows. The same exact profile still
aborted at `8c53dce` in CI `34441725835`, so no measurement shows any of them
needed:

- `(subpath "/private/var/tmp")` is withdrawn.
- `(literal "<helper>")` is justified baseline, as an execution input.
- `(literal "/dev/random")` is justified baseline, as a hands element.
- `(literal "/dev/dtracehelper")` is withdrawn.

A withdrawn unit, and the part of a narrowed unit that its narrowing dropped,
carries no authority. It returns only through the bounded native experiment.
When a Seatbelt startup cell of the candidate fails before `READY`, each
withdrawn or narrowed fa7 unit SHALL run as its own labelled restoration
diagnostic, meaning the exact candidate plus that one unit in its fa7 form. One
further labelled cell SHALL restore every such unit at once. It reproduces
fa7's authority, which separates a regression caused by the withdrawal from the
measured child-spawn refusal. These cells are diagnostics. They never pass
startup and never admit authority. If one advances the stages, native denial
evidence SHALL name the operation and a single-object target before a
diagnosis-admitted entry with its own removal control enters the ledger, and
every denial control SHALL rerun. The unfiltered process family, unfiltered
`sysctl-read`, unfiltered `ipc-posix-shm`, host tmp reads, `(subpath
"/System")`, `(subpath "/usr")` and the whole `<cell-root>` SHALL NOT re-enter
in their historical form. Denial evidence may show a toolchain bind or the
system-library element under another resolved spelling. That
spelling, no wider than the element, then replaces its baseline unit, recorded
with its evidence as a correction of the same justification.

Each Seatbelt startup cell, removal replay and diagnostic SHALL keep the
bounded native Sandbox denial events that the unprivileged observer can read
for its responsible processes during that cell. If the host does not make them
available, the cell SHALL record them as unavailable. An absent event never
proves an operation allowed or unneeded. The startup denial controls SHALL add
a read of the credential target through its `/System/Volumes/Data` spelling,
and the candidate SHALL deny it as it denies the direct spelling.

The check SHALL be a host-independent function over the rendered template text,
the ledger and the removal set. It SHALL parse the rendered profile rather than
trust whatever renders it. It SHALL refuse each of these:

- a unit in neither half, or a unit in both halves;
- a ledger or removal entry the template lacks;
- a missing or altered frame, an extra `deny` or an `allow default`;
- any form, filter or text outside the closed normalization above, or a unit
  rendered twice;
- an unfiltered unit other than `process-fork`;
- a baseline entry without a justification of a named kind;
- a toolchain unit whose target is not exactly the one `--ro-bind-try` source
  it names in the argv that `box_argv` renders;
- a diagnosis-admitted entry that lacks its operation, target, process,
  consumer, evidence or removal entry, or that uses a filter wider than a
  single object;
- a removal entry whose unit is not diagnosis-admitted.

The fa7 template stays in the probe's test data verbatim. The check SHALL
prove that each of its units is carried by the ledger or has a withdrawal
record with its reason, and that each of the four `8c53dce` units keeps the
disposition above.

The ledger changes the candidate, and so does its per-bind `/usr` narrowing.
The candidate's startup is unmeasured until the
controller's native Gate A runs on the exact head that carries it. fa7's
progress to `executable` describes the fa7 template, not this one. No stage
progress, denial result or removal verdict carries over from fa7 to the new
candidate.

#### Scenario: The template is exactly the ledger's disjoint union
- **GIVEN** the rendered candidate template, the startup-rule ledger and the removal set
- **WHEN** the host-independent check parses the template into normalized rule units
- **THEN** every form after the frame is a single-operation `allow` form of simple filters, the units equal the baseline entries plus the removal set with no overlap, the frame is exactly `(version 1)` then `(deny default)`, and `(allow process-fork)` is the only unfiltered unit

#### Scenario: A multi-operation form cannot hide an operation
- **GIVEN** a template whose listed payload write is rendered as `(allow file-write* process-exec (subpath "<payload-root>"))`
- **WHEN** the check parses it
- **THEN** the check fails and names `process-exec` as the hidden operation; it neither keeps the first operation nor splits the form into units, so the extra operation never reaches the equality as a listed unit

#### Scenario: A top-level form other than the frame and allow forms fails
- **WHEN** the template carries, after the frame, a `(trace ...)`, `(define ...)`, `(if ...)`, `(debug ...)`, `(import ...)` or `(param ...)` form, a `deny` form, `(allow default)`, a second `(version 1)` or a bare atom
- **THEN** the check fails and names that form; no top-level form is skipped as not a rule

#### Scenario: Modifiers, compound filters and unparseable text fail
- **WHEN** a form carries an action modifier such as `(with report)`, a `require-any`, `require-all` or `require-not` filter, or a filter with no argument, two arguments or a list argument, or when the text holds a comment, an unbalanced parenthesis, a string needing an escape sequence, or one unit rendered twice
- **THEN** the check fails and names the modifier, filter, text or duplicated unit, and no unit is taken from the unparsed remainder

#### Scenario: The toolchain baseline is no wider than the binds it names
- **GIVEN** the `--ro-bind-try` sources that `hands.rs` `box_argv` renders
- **WHEN** the check reads the ledger's toolchain units
- **THEN** each unit names one of those binds and targets exactly it; `(subpath "/usr")` for read or exec is an unlisted unit and fails; `/usr/include`, `/usr/lib64`, `/lib` and `/lib64` carry no unit; and a unit whose named bind `box_argv` no longer renders fails

#### Scenario: The candidate's process authority is seven named units
- **WHEN** the check reads the candidate's process units
- **THEN** they are exactly `(allow process-fork)` and the `process-exec` units for `<helper>`, `/usr/bin`, `/usr/libexec`, `/usr/local`, `/bin` and `/sbin`; any other `process-*` unit, including `process-exec` on `/usr`, `/usr/lib` or `/usr/share`, fails as unlisted; and the child-spawn prohibition on wider process authority reads against these seven

#### Scenario: An unlisted or doubly classified unit fails
- **WHEN** the check receives a template carrying one extra unit that is in neither half, such as `(allow sysctl-read)` or `(allow file-write-data (literal "/dev/null"))` without its ledger entry, or a ledger that lists one unit both as baseline and in the removal set
- **THEN** the check fails and names the unit, and no multi-filter grouping hides an extra filter inside a listed form

#### Scenario: A listed unit the template lacks fails
- **WHEN** a baseline entry or a removal entry names a unit that the rendered template does not carry
- **THEN** the check fails before any native run, and the adapter never reports that entry's removal as observed

#### Scenario: Every ledger entry carries its typed justification
- **WHEN** a baseline entry has an empty justification or a kind other than hands element, execution input or probe-harness need, or a diagnosis-admitted entry lacks its operation, narrow target, responsible process, consumer, evidence or removal entry
- **THEN** the check fails; a historical presence in an earlier template is not a justification

#### Scenario: A diagnosis-admitted unit names a single object
- **WHEN** a diagnosis-admitted entry is unfiltered or uses a `subpath`, `prefix` or `regex` filter
- **THEN** the check refuses it, whatever a restoration or one-class diagnostic observed

#### Scenario: The four 8c53dce additions keep their disposition
- **WHEN** the check reads the ledger and the retained fa7 template
- **THEN** `(literal "<helper>")` and `(literal "/dev/random")` are baseline with their recorded kinds, `(subpath "/private/var/tmp")` and `(literal "/dev/dtracehelper")` are absent from the candidate with withdrawal records, and every other fa7 unit is carried by the ledger or has a withdrawal record

#### Scenario: A withdrawn unit returns only through the bounded experiment
- **WHEN** a Seatbelt startup cell of the ledger candidate fails before `READY` and the restoration diagnostic for one withdrawn or narrowed fa7 unit, or the all-restored fa7-authority cell, advances the stages
- **THEN** those cells stay non-passing and admit nothing, native denial evidence must name an operation and single-object target before a diagnosis-admitted entry with its own removal control enters, every denial control reruns, and no withdrawn unfiltered family, host tmp read, `(subpath "/System")`, `(subpath "/usr")` or whole-cell-root read returns

#### Scenario: An attributed /dev/null write is diagnosis-admitted
- **WHEN** the child-spawn cells attribute the refusal to the null-stdio write open of `/dev/null`
- **THEN** exactly `(allow file-write-data (literal "/dev/null"))` enters the diagnosis-admitted half with its own removal entry, the equality check passes only with both, and its necessity stays unproven until a cell that reaches `READY` observes that removal blocking

#### Scenario: The data-volume spelling of a credential stays denied
- **WHEN** a denial control reads the credential target through `/System/Volumes/Data`
- **THEN** the read fails without returning bytes on every Seatbelt cell that runs the controls, exactly as the direct spelling does

#### Scenario: The ledger candidate's startup is pending native measurement
- **WHEN** the ledger candidate is prepared on the Linux controller
- **THEN** its Gate A startup, denial controls and removal controls are recorded as pending for the controller's exact-head native run, and fa7's stage progress, denials and not-due removals are not attributed to it

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

Verdict facts SHALL be captured before harness cleanup. Guard survival SHALL
be observed after payload teardown and before guard unregister. Quiescence,
guard state, cleanup ordering and the complete survivor set SHALL come from
the external observer or guard-private state that the payload cannot write; a
payload-writable heartbeat or marker may prove activity only. A guard or
payload job fact read from `launchctl print` follows the top-level dictionary
rule of the startup requirement. A coalition block's `state` or `active count`
SHALL NOT establish guard survival, payload quiescence or the survivor set.
All observed
children and helper processes, including a killed supervisor-liveness holder,
SHALL be waited or reaped on every success and error path after their verdict
facts have been preserved.

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

Each named obligation SHALL exercise its own trigger rather than reuse one
detach routine and infer the label: timeout crosses the configured deadline;
cancellation delivers a live supervisor request; supervisor death kills the
sole liveness writer and waits/reaps it; retained-pipes proves the detached
child actually holds the payload pipe; and parent-exit observes the direct
parent exit before teardown. The peer target SHALL be registered and observed
ready before the payload attempts `bootout`, and an outside record SHALL prove
the attempt occurred and the peer remained alive. Liveness-channel setup SHALL
use a bounded nonblocking or pollable open with both endpoints accounted for;
a timed-out blocked thread is a probe failure and is never abandoned.

The process-group negative control SHALL be independent of the guard and its
liveness FIFO. It SHALL launch the detach helper, record the original process
group and detached descendant identity, send a real `SIGKILL` to only the
original process group, and observe the detached descendant and heartbeat
still live before explicit harness cleanup. It does not pass from an assumed
group kill or a zero-survivor default.

#### Scenario: The named launchd candidate must survive the detach adversary
- **GIVEN** a native helper that proves its ordinary-child positive control, then forks, calls setsid, double-forks, ignores termination signals and reports identities while writing a heartbeat
- **WHEN** the isolated feasibility probe triggers timeout, cancellation and abrupt supervisor death in separate runs
- **THEN** guard-owned teardown ends every reported payload descendant inside the bound, both transient labels disappear in order, and the external observer sees one second of quiet; otherwise the probe fails with SEATBELT-R3 open and no dependent implementation is authorized

#### Scenario: The guard remains outside the payload job
- **GIVEN** the separately labelled guard job has established its private liveness channel and the payload job is running
- **WHEN** timeout, explicit cancellation or supervisor-liveness EOF causes the guard to boot out the payload job
- **THEN** the guard remains alive after payload bootout, proves payload-domain quiescence before cleanup, unregisters only after cleanup, and an observer outside both jobs verifies the ordering; a guard removed with the payload fails SEATBELT-SPEC-LIFETIME-TOPOLOGY and cannot authorize dependent implementation

#### Scenario: The payload cannot acquire guard or peer-job authority
- **GIVEN** the guard endpoint, labels and control state are engine-owned, and the peer job is registered and externally observed ready before payload code is confined by the experimental profile
- **WHEN** the payload attempts to signal or impersonate the guard, use its private channel, boot out a peer invocation or register an independently surviving launchd job
- **THEN** an outside record proves each attempt occurred, every attempt is denied, the guard is observed live after payload teardown but before unregister and the peer remains live; any missing synchronization or successful interference fails the probe, leaves SEATBELT-R3 open and stops dependent implementation

#### Scenario: The group-kill negative control performs the weaker operation
- **GIVEN** a detached helper whose original process group and non-reusable descendant identity are externally recorded without a guard or liveness FIFO
- **WHEN** the probe sends `SIGKILL` to that original process group
- **THEN** the detached descendant and heartbeat remain live until the outside observer records them, after which harness cleanup kills and reaps them; an assumed kill, blocked FIFO or zero-survivor result fails the negative control

#### Scenario: Every lifetime label has a distinct observed trigger
- **WHEN** cancellation, supervisor death, retained pipes and parent exit are measured
- **THEN** the observer records respectively a live cancellation request, the killed-and-reaped sole liveness writer, a descendant retaining the actual output pipe, and direct-parent exit before teardown; running one generic detach path under four names cannot satisfy the matrix

#### Scenario: Payload markers cannot attest guard or quiescence
- **WHEN** the adversary can write its heartbeat and other payload-root files
- **THEN** those files establish only pre-trigger activity, while guard liveness, survivor identities, label state and cleanup ordering are independently observed before unregister and before harness cleanup

#### Scenario: Probe channels and helpers terminate on every path
- **WHEN** liveness setup, a case trigger or observation fails or times out
- **THEN** no blocked FIFO-opening thread is abandoned, every killed holder or helper is waited or reaped, durable diagnostics precede cleanup, and harness cleanup is reported separately from containment

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
execution remains the native suite's separate obligation. Probe-support
binaries included in workspace targets SHALL compile and link on every
supported workspace host; Unix system calls SHALL use target-correct
implementations or be structurally excluded from unsupported targets without
hiding the shared verdict model. A non-Darwin link failure is a test-harness
defect and no native enforcement evidence. Any required
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

#### Scenario: Probe support does not break non-Darwin validation
- **WHEN** the workspace suite compiles the native helper on Linux or Windows
- **THEN** it links without unresolved Unix symbols and selects no native macOS case; platform gating does not remove the shared startup/lifetime verdict tests or turn their simulated pass into native evidence

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
  SHALL be ordinary Rust compiled on Linux behind narrow fact adapters. Probe
  helpers SHALL link on supported non-Darwin workspace hosts without resolving
  Unix calls there. Darwin-only calls require separate native evidence;
  target-gating the whole decision module or lowering exact coverage is
  refused.
- **SEATBELT-R3-STARTUP at `8c53dce` — narrow the fault, do not widen the
  boundary.** Adopt direct-unboxed helper success and exact-profile signal 6 as
  evidence that the restrictive profile is the differing layer. Reject the
  passing `allow default` diagnostic as admission because it demonstrates no
  bounded authority. Reject the launchd observations as a control because
  repeated cells reused lifecycle state and returned bootstrap errors or
  missing terminal facts. Unique-label isolation, raw lifecycle evidence and
  one-authority profile diagnosis precede another lifetime attempt.
- **SEATBELT-R3-STARTUP at `9f4c2c9` — repair observation before another
  native dispatch.** Adopt S0 and generic workspace passes as controls and S1's
  signal 6 plus S3's non-ready crash as startup failures. Reject the S2 parser
  refusal as proof that the payload did or did not run, and reject raw S1/S3
  profile-digest equality because private roots are intentionally distinct.
  Seven failing single-class diagnostics identify no admissible allowance;
  `allow default` still authorizes none. The successor must retain independent
  launchd facts, compare normalized authority and name the denied operation,
  target and consumer before changing the profile. Lifetime remains not run.
- **SEATBELT-R3-STARTUP at `fa7ece5`: name the child-spawn step and parse what
  launchd prints.** Adopt S1 and S3 reaching `executable` as evidence that the
  root-inode read cleared the pre-stage abort, and adopt the three observed
  denials as facts about that candidate only. Reject every one-class diagnostic
  and `allow default` as admission. Treat `/dev/null` write access for null
  stdio as the leading, unproven hypothesis. Every failing cell withheld all
  `file-write*` outside the payload, while Rust's null stdio opens `/dev/null`
  for writing in the parent before exec. Sub-stage evidence discriminates the
  hypothesis; nothing assumes it. Adopt `state`, `runs` and `last exit code` as
  the required launchd terminal facts and the crash counter as optional,
  because the measured print omits it for exit 0 and for exit 2. Refute
  requiring the counter, because then no launchd cell could ever pass. Refute
  synthesizing zero, because that fabricates a fact. This is probe observation
  semantics, not an accepted-addendum guarantee: a pass still depends on the
  helper's authenticated facts, so no proposed decision is needed. No fa7
  Seatbelt cell reached `READY`, so no removal control was due. The root-inode
  read remains unproven, and its removal blocks every cell that later reaches
  `READY` until observed.
- **SEATBELT-R3-STARTUP at `fa7ece5`: which cells owe a removal control
  (clarify).** Answer: exactly the Seatbelt startup cells that reach a
  nonce-authenticated `READY`. Blocking means the stripped replay reaches no
  `READY`. Refute requiring removal on every cell with a stage-relative
  criterion. A removal proves necessity only against a candidate that starts,
  and a later predicate can change that necessity. A stripped run that fails
  earlier than a failing candidate only shows that the predicate advances the
  stages. Recording that as load-bearing would be fabricated proof. It also
  adds no gate, because a non-`READY` cell already fails. Refute reading
  "blocked" as "no `READY`" on a non-starting candidate, because that verdict is
  vacuous. The removal set is exactly the diagnosis-admitted predicates. Each
  removal is a direct `/usr/bin/sandbox-exec` replay of the cell's own profile,
  including for S3, because launchd adds no Seatbelt authority. A
  launchd-only predicate is therefore not admissible under this change. This
  is probe observation semantics, not an accepted-addendum guarantee, so no
  proposed decision is needed.
- **SEATBELT-R3-STARTUP at `fa7ece5`: which dictionary supplies launchd facts
  (clarify).** Answer: only the job's top-level dictionary. The fa7 S2 and S3
  terminal prints carry three `state =` lines and three `active count =`
  lines, and the nested coalition copies read `active` and 1 even for S3, whose child
  never spawned. Refute first-match scanning, because it conforms only by print
  order. Refute failing on any repeated key anywhere in the sample, because
  every measured print repeats keys in nested blocks and no launchd cell could
  pass. Refute nested fallback for a missing top-level key, because that turns
  a coalition fact into a job fact. A duplicated top-level key stays unknown and
  fails closed, since launchd prints none and a duplicate would be an
  unattributable observation. Coalition fields are raw evidence only and never
  establish liveness, quiescence or survivors, which come from the external
  observer. This is probe observation semantics, not an accepted-addendum
  guarantee, so no proposed decision is needed.
