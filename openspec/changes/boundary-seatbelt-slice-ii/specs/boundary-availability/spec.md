## MODIFIED Requirements

### Requirement: A boundary the machine cannot build refuses at start

Availability SHALL be judged only when the compiled bundle declares at
least one hands site, against the actual host and the supplied search path.
The activation requirement below governs whether Seatbelt is built at all;
positive availability and execution scenarios assume that prerequisite.
An incomplete implementation or any open SEATBELT-R1 through R4 native
evidence residual SHALL retain the unbuilt refusal, even when launcher readiness is reported separately.
`namespace` retains its existing PATH lookup and 0.10-or-newer check for
overlay binds without a new ownership, executable-bit or sandbox probe rule.
`container` remains unbuilt, whatever tools are found. `harness` and `open`
remain available without boundary tooling.

For built `seatbelt`, the host SHALL be macOS and the launcher SHALL be the
literal absolute `/usr/bin/sandbox-exec`. The supplied search path SHALL
contain a literal `/usr/bin` entry; an empty path or one containing only
other entries SHALL refuse, even if the system launcher exists elsewhere
in the process's environment. Relative entries, dot entries, alternate
spellings, symlinked directories and copied or same-named launchers SHALL
NOT establish this prerequisite. Other PATH entries SHALL never be executed
as a Seatbelt probe or launcher, including one before `/usr/bin`.

The fixed launcher and its ancestors through `/usr/bin` and `/usr` SHALL
be nonsymlink system entries owned by uid 0 and not writable by group or
others; the launcher SHALL be a regular file executable by the calling
user. This defines the trusted system location, not a search for any
root-owned file. The operator's OS installation is trusted; a hostile
repository or declared writable bind SHALL not supply or alias a writable
launcher. A failed metadata or protection check SHALL refuse by name.

Usable SHALL additionally require a real policy probe of that fixed
launcher, with a total five-second budget and bounded diagnostics. The
probe SHALL use trusted test operations, private disposable sentinels and
an engine-authored policy: one permitted read must return its nonce and
one prohibited read must fail without returning its nonce. Failure to
start, nonzero positive-control exit, successful prohibited read, timeout
or unexpected output SHALL refuse. Running `true` alone, file existence,
version output or a launcher stub that merely exits zero SHALL not pass.
The trusted operation SHALL be a purpose-built native helper or another
engine-controlled executable whose startup dependencies are part of the
bounded readiness probe; it SHALL NOT depend on undeclared interpreter,
shell-startup or repository runtime state. Each justified allowance SHALL be
followed by the same positive and denial controls.

Doctor and start SHALL share this rule; probes execute no repository code,
start no seat, and append no journal. A wrong host SHALL refuse without
executing any launcher or probe. The probe proves readiness only; the full
native adversarial suite remains mandatory.

A refusal SHALL name the boundary, failed prerequisite or unsupported
policy, observed host/tool facts, affected hands sites and decision 0046
ruling 2. CLI `run`, `resume` and `rerun`, and their runtime library entry
paths, SHALL refuse before appending any journal row or spawning a seat.
Resume may open the journal to read its pinned manifest but SHALL not write
on refusal. The low-level hands entry points SHALL also enforce their
boundary's prerequisites, including a tool or policy becoming unusable
between preflight and invocation. Nothing SHALL degrade to another
boundary or execute the payload after a failed preflight.

#### Scenario: namespace on an empty PATH refuses
- **WHEN** a bundle with hands site `work` compiles under `namespace` and the supplied search path is empty
- **THEN** start refuses naming `namespace`, bubblewrap, `work` and decision 0046 ruling 2

#### Scenario: seatbelt on an empty PATH refuses
- **WHEN** that bundle compiles under `seatbelt` on macOS and the supplied search path is empty
- **THEN** start refuses naming `seatbelt`, `sandbox-exec`, the missing-tool condition and `work`, without using an unrelated search path

#### Scenario: A wrong host cannot offer Seatbelt by naming a tool
- **WHEN** the same Seatbelt bundle is started on Linux or Windows with a same-named `sandbox-exec` file on the supplied path
- **THEN** it refuses naming the actual host, the macOS requirement and `work`, and the planted executable is not invoked

#### Scenario: A macOS host supplies the actual boundary
- **WHEN** a macOS host supplies the usable system `sandbox-exec`, the engine implements Seatbelt and the site's policy can be enforced
- **THEN** preflight admits the bundle and execution uses the Seatbelt path; the old slice (ii) unbuilt refusal does not fire

#### Scenario: An unusable or untrusted launcher is not offered
- **WHEN** macOS has only a repository-controlled lookalike on the supplied PATH, or the fixed system launcher is non-executable, a symlink, non-root-owned or writable by group/others
- **THEN** Seatbelt refuses naming `/usr/bin/sandbox-exec` and the missing literal PATH entry or failed trust check before user code; the lookalike is never invoked

#### Scenario: The system pin ignores an earlier lookalike
- **WHEN** a macOS supplied PATH is `<worktree>/bin:/usr/bin` and the worktree contains an executable named sandbox-exec
- **THEN** only `/usr/bin/sandbox-exec` is checked, probed and invoked, the planted marker stays absent, and a passing system probe admits an enforceable policy; `<worktree>/bin` alone or a symlink to `/usr/bin` alone refuses

#### Scenario: A no-op launcher fails the denial probe
- **WHEN** the readiness probe receives exit zero without executing its operations, or a launcher executes both sentinel reads without applying the policy
- **THEN** the missing expected nonce or successful forbidden read makes the result unusable and no payload starts; an actual native launcher must pass both controls within five seconds

#### Scenario: A crashing probe payload is not launcher readiness
- **WHEN** `/usr/bin/sandbox-exec` starts the readiness helper but the helper aborts before returning the permitted nonce, including the committed native helper exiting by signal 6 under the exact restrictive profile
- **THEN** readiness refuses with the startup status and bounded diagnostics; launcher existence, launchd registration, direct unboxed success or a labelled `allow default` diagnostic cannot make Seatbelt available

#### Scenario: Readiness does not widen namespace lookup
- **WHEN** existing namespace lookup and overlay-version fixtures run with their supplied PATHs
- **THEN** their previous results are unchanged; Seatbelt's fixed-path provenance checks and allow/deny probe are never applied to bwrap

#### Scenario: container on an empty PATH refuses
- **WHEN** a hands bundle compiles under `container` with no container engine on the supplied path
- **THEN** it refuses naming `container`, slice (iii), the missing Docker/Podman readiness fact and the hands sites

#### Scenario: harness and open pass on an empty PATH
- **WHEN** the hands bundle compiles under `harness` and under `open` with an empty search path
- **THEN** both pass boundary availability without changing their gate admission rules

#### Scenario: namespace passes with its tool
- **WHEN** the search path holds usable bubblewrap and the namespace bundle binds no overlay
- **THEN** namespace passes exactly as before

#### Scenario: An overlay bind still asks 0.10 of bubblewrap
- **WHEN** a namespace bundle declares an overlay with a bubblewrap older than 0.10
- **THEN** it refuses naming the affected site and `0.10 or newer`

#### Scenario: A plain bundle passes everywhere
- **WHEN** a bundle with no hands site compiles under any boundary with an empty search path
- **THEN** availability passes because no execution boundary is requested

#### Scenario: The three verbs refuse before the journal
- **WHEN** CLI run/resume/rerun or the equivalent runtime entry receives a Seatbelt hands bundle on a wrong host, without its tool, or with a detected unenforceable policy
- **THEN** no seat starts and no new journal row is written; resume's existing rows remain byte-identical, and the error identifies the relevant sites and reason

#### Scenario: A later tool failure stays closed
- **WHEN** preflight succeeds but the system tool disappears or rejects preparation before a workspace call or exec dispatch
- **THEN** that invocation fails naming Seatbelt and the cause, no payload marker appears, and no unboxed fallback runs

### Requirement: init's warning speaks the vocabulary

`brokkr init` SHALL keep naming the scaffolded hands seats and `namespace`
as the default requiring bubblewrap. On macOS its advice SHALL also name
`seatbelt` as the realm choice intended by 0046 slice (ii), with the same
activation and fixed-system-launcher facts used by doctor. When built and
ready it SHALL tell the operator to declare `boundary: seatbelt` in the
realm and check the scaffold with `brokkr doctor --bundle <dir>`. When
unbuilt or unavailable it SHALL name that condition and its cause without
promising the scaffold can run. It SHALL retain `harness` as an explicitly
unboxed alternative subject to its gate/adapter restrictions. On other
hosts the existing absent-bubblewrap advice SHALL remain. Init SHALL not
edit or select a realm boundary, infer a security ruling, or treat a
readiness probe as acceptance of the slice.

#### Scenario: init without bubblewrap
- **WHEN** init runs on a non-macOS host without bwrap on PATH
- **THEN** its warning names the scaffolded seats, namespace, bubblewrap, and harness as the realm alternative, and cites decision 0046

#### Scenario: init on macOS describes the available Seatbelt road
- **WHEN** init runs on macOS without bwrap and Seatbelt is built with a trusted launcher passing the readiness probe
- **THEN** its advice names Seatbelt as the macOS realm choice, the unchanged namespace default, the doctor bundle check and the unboxed harness alternative; the realm file is unchanged

#### Scenario: init does not recommend an unbuilt boundary as usable
- **WHEN** init runs on macOS while any required native residual remains open, or the host lacks a trusted usable launcher
- **THEN** its advice identifies unbuilt Seatbelt and the open residual or precise launcher cause; it does not promise the scaffold can run under Seatbelt or silently switch its boundary

## REMOVED Requirements

### Requirement: A boundary this engine does not build refuses at start until its slice lands

**Reason**: Its slice-I inventory permanently describes Seatbelt and container
as unbuilt. The target after slice II's activation replaces that inventory
with implemented Seatbelt and a continuing container refusal. Until the
activation conditions below hold, Seatbelt's unbuilt refusal is retained;
removing this requirement in a draft is not authority to lift that fence.

**Migration**: Use "Only container remains unbuilt after Seatbelt implementation"
and the modified host/tool availability requirement in this capability.
Realm declarations and compile-time identities remain valid; container's
engine-present and runtime-entry refusal scenarios remain covered.

### Requirement: doctor names the boundaries this machine offers

**Reason**: The old requirement and its Seatbelt warning scenario describe the
pre-slice-II engine even on a capable Mac. After activation doctor diagnoses
Seatbelt using host, tool and policy facts. Until then it retains the unbuilt
slice (ii) diagnosis. This replacement retains all other boundary diagnoses.

**Migration**: Use "doctor reports the implemented boundaries and their actual
prerequisites" with the same single boundaries line and per-bundle hands line;
existing CLI options do not change. Users on missing-tool or wrong hosts get
an actionable refusal, while container still names slice (iii).

## ADDED Requirements

### Requirement: Only container remains unbuilt after Seatbelt implementation

The engine SHALL treat `seatbelt` as built only when both workspace and
boxed exec paths implement the whole accepted hands policy described by
`seatbelt-execution`, including shipped masked overlays, conditional full-peer
Git protection and no surviving payload, and qualifying native macOS evidence
has closed SEATBELT-R1 through R4. The accepted 0046 addendum settles the
observables but supplies no implementation or evidence. Only the bounded R3
native feasibility probe may precede its proof; full dependent implementation
and activation remain fenced. A launcher probe, source inspection, mock, Linux
test or partially passing Mac suite SHALL NOT make Seatbelt built.
A native run that never proves payload startup is a failed startup measurement,
not a partially passing lifetime suite and not evidence that descendants were
contained.

Refusing a malformed or conflicting individual layout is required safety
behavior. Refusing all overlays, all masked binds, ordinary linked-worktree
Git or all commands that might fork SHALL NOT count as implementing those
features. A boundary supporting only a bind-free subset SHALL stay unbuilt
for this deliverable; removing the start fence is not its completion test.
If the operator commissions a reduced deliverable, its built status and
shipped declarations SHALL first be amended upstream with reasons and
corresponding scenarios. This draft chooses no such reduction.

`container` SHALL remain named, pinned and admitted by the compile-time
gate law but unbuilt in slice II. Every CLI and runtime start/resume/rerun
entry for a bundle with hands under `container` SHALL refuse before any
journal row or seat spawn, naming `container`, the hands sites, slice
(iii) of decision 0046 ruling 6 and the Docker/Podman readiness fact. A
present engine SHALL not bypass that refusal. No container execution path
or revived `driver.confine` wrapper belongs to this slice. A bundle without
hands SHALL not be refused merely for its realm's boundary word.

#### Scenario: Seatbelt becomes built through actual execution paths
- **WHEN** the implementation exposes Seatbelt as built
- **THEN** the overlay, mask, hooks-view/peer and lifetime prerequisites are resolved, both entry points implement their complete ruled policy through real sandbox-exec with failure handling, and the required Mac suite has passed before activation or overall slice acceptance

#### Scenario: Shipped overlay users cannot be refused into a built claim
- **GIVEN** a macOS host with a trusted usable system launcher and the unchanged self/verify verifier and review-agent declarations containing masked cargo overlays
- **WHEN** a candidate can execute a bind-free smoke test but cannot realize those declared overlays or masks
- **THEN** self and verify remain refused at start before any journal row, doctor reports Seatbelt unbuilt for slice (ii) with the unmet policy, and that candidate fails this slice's deliverable; a per-bind refusal does not make the boundary built

#### Scenario: Accepted semantics still require native evidence
- **WHEN** relocated snapshots, denied-read masks or private hooks are implemented without the required native proof, or detached survivors or harness-grade Seatbelt are proposed
- **THEN** Seatbelt activation remains fenced and the missing proof is recorded as an open residual; survivors or reduced grade require a new upstream ruling; no successful Seatbelt gate, peer marker or completion claim is produced

#### Scenario: A non-starting native payload proves no lifetime property
- **WHEN** the direct sandbox control aborts, launchd bootstrap fails, or a registered job has no externally observed ready state and reliable terminal facts
- **THEN** the R3 run stops before lifetime triggers, records startup separately with its exit and job state, leaves SEATBELT-R3 open and keeps the production fence intact

#### Scenario: container with either engine present still refuses
- **WHEN** a bundle with hands site `work` compiles under `container` and the path holds Docker, Podman or both
- **THEN** CLI and runtime entries refuse naming `container`, `work`, slice (iii) and the discovered engine, without appending a journal row or invoking a container

#### Scenario: A plain bundle under an unbuilt boundary starts
- **WHEN** a bundle with no hands site compiles under `container` and starts
- **THEN** the unbuilt-boundary fence does not refuse it

### Requirement: doctor reports the implemented boundaries and their actual prerequisites

`brokkr doctor` SHALL print one `boundaries` line using the same host, tool,
implementation and evidence rules as start: `namespace` with its bubblewrap
fact, activated `seatbelt` only on macOS after its usable system launcher and
SEATBELT-R1 through R4 are closed, and `harness` and `open` always. Before
Seatbelt activation it SHALL say not yet built, slice (ii), and list the open
residual identifiers and next native action; launcher readiness is a separate
fact and never an offer. For a closed implementation on the wrong host or with
an unavailable launcher it SHALL state that specific cause. `container` SHALL
remain not yet built for slice (iii), with Docker/Podman presence reported only
as readiness.

With `--bundle`, doctor SHALL compile in the discovered realm and judge
that bundle's actual hands policy, naming affected sites and detected
unsupported binds/masks or git layouts. A healthy Seatbelt hands line
SHALL describe a Brokkr-enforced boundary, never the harness/open message
that no box is built. Doctor SHALL not run a seat or write a journal, and
its offering SHALL not substitute for the native security measurements.
Doctor SHALL distinguish machine launcher readiness, feasibility-probe payload
startup and the lifetime verdict; a failure at one layer SHALL NOT be restated
as a fact about a later layer.

#### Scenario: A Linux host with bubblewrap and Docker
- **WHEN** Linux has bubblewrap 0.11 and Docker, even with a Seatbelt lookalike on PATH
- **THEN** doctor offers namespace, harness and open, says Seatbelt requires macOS, and labels container unbuilt pending slice (iii) with Docker present

#### Scenario: A capable macOS host offers Seatbelt
- **WHEN** macOS has a usable system sandbox-exec, both Seatbelt paths are implemented and SEATBELT-R1 through R4 are closed by qualifying native evidence
- **THEN** doctor offers seatbelt, harness and open, and its Seatbelt bundle hands line reports the enforced boundary when the site's policy can be built

#### Scenario: Empty PATH and unusable tooling are diagnosed
- **WHEN** macOS has an empty supplied search path, or its located launcher cannot apply a sandbox
- **THEN** doctor distinguishes missing from unusable sandbox-exec and does not offer Seatbelt; harness/open remain offered and container remains unbuilt

#### Scenario: A particular bundle needs more than a launcher
- **WHEN** doctor examines a Seatbelt bundle whose binds, masks or git layout fail preparation checks
- **THEN** its hands line warns with the affected site and exact unsupported policy, even if the machine-level boundaries line finds the tool

#### Scenario: A harness bundle needs no Seatbelt or bubblewrap
- **WHEN** doctor examines a valid harness bundle on a host without either tool
- **THEN** the hands availability line stays healthy under harness and does not claim Brokkr enforcement
