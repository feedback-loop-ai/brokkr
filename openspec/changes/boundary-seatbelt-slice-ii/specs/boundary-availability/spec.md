## MODIFIED Requirements

### Requirement: A boundary the machine cannot build refuses at start

Availability SHALL be judged only when the compiled bundle declares at
least one hands site, against the actual host and the supplied search path.
`namespace` retains the existing bubblewrap check and requires 0.10 or
newer for overlay binds. `seatbelt` requires macOS and a usable system
`sandbox-exec` available through that search path; a same-named executable
on Linux or Windows does not make Seatbelt available. A repository-planted
launcher does not establish system-tool availability. `container` remains
unbuilt as the following requirement states, whatever tools are found.
`harness` and `open` remain available without boundary tooling.

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
- **WHEN** macOS lookup finds a non-executable file, a repository-controlled lookalike or a system launcher that cannot apply the required policy
- **THEN** the boundary refuses with the tool or policy cause before user code, rather than accepting file existence as evidence

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

## REMOVED Requirements

### Requirement: A boundary this engine does not build refuses at start until its slice lands

**Reason**: Its slice-I inventory permanently describes Seatbelt and container
as unbuilt. Slice II replaces that inventory with an implemented Seatbelt path
and a continuing container refusal; preserving a scenario that refuses a
capable Mac solely for slice (ii) would contradict the commissioned behavior.

**Migration**: Use "Only container remains unbuilt after Seatbelt implementation"
and the modified host/tool availability requirement in this capability.
Realm declarations and compile-time identities remain valid; container's
engine-present and runtime-entry refusal scenarios remain covered.

### Requirement: doctor names the boundaries this machine offers

**Reason**: The old requirement and its Seatbelt warning scenario describe the
pre-slice-II engine even on a capable Mac. Doctor now diagnoses an implemented
Seatbelt boundary using host, tool and policy facts instead of promising a
future slice. This replacement retains diagnosis for the other boundaries.

**Migration**: Use "doctor reports the implemented boundaries and their actual
prerequisites" with the same single boundaries line and per-bundle hands line;
existing CLI options do not change. Users on missing-tool or wrong hosts get
an actionable refusal, while container still names slice (iii).

## ADDED Requirements

### Requirement: Only container remains unbuilt after Seatbelt implementation

The engine SHALL treat `seatbelt` as built only when both workspace and
boxed exec composition paths actually implement the hands policy described
by `seatbelt-execution`. Removing a refusal or generating profile strings
alone does not build a boundary. The implementation can be prepared for
native CI while Mac acceptance evidence remains explicitly pending; the
availability fact SHALL not be reported as completion of the whole slice.

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
- **THEN** both hands entry points have a real sandbox-exec execution path with policy enforcement and failure handling, and native tests are required before the overall slice is accepted

#### Scenario: container with either engine present still refuses
- **WHEN** a bundle with hands site `work` compiles under `container` and the path holds Docker, Podman or both
- **THEN** CLI and runtime entries refuse naming `container`, `work`, slice (iii) and the discovered engine, without appending a journal row or invoking a container

#### Scenario: A plain bundle under an unbuilt boundary starts
- **WHEN** a bundle with no hands site compiles under `container` and starts
- **THEN** the unbuilt-boundary fence does not refuse it

### Requirement: doctor reports the implemented boundaries and their actual prerequisites

`brokkr doctor` SHALL print one `boundaries` line using the same host/tool
availability rules as the start refusal: `namespace` with its bubblewrap
fact, `seatbelt` only on macOS with the usable system launcher and actual
implementation, and `harness` and `open` always. For an unavailable built
boundary it SHALL state the specific wrong-host, missing-tool or unusable-
tool reason. `container` SHALL be shown as not yet built, naming slice
(iii) and whether Docker/Podman is present as a readiness fact only.

With `--bundle`, doctor SHALL compile in the discovered realm and judge
that bundle's actual hands policy, naming affected sites and detected
unsupported binds/masks or git layouts. A healthy Seatbelt hands line
SHALL describe a Brokkr-enforced boundary, never the harness/open message
that no box is built. Doctor SHALL not run a seat or write a journal, and
its offering SHALL not substitute for the native security measurements.

#### Scenario: A Linux host with bubblewrap and Docker
- **WHEN** Linux has bubblewrap 0.11 and Docker, even with a Seatbelt lookalike on PATH
- **THEN** doctor offers namespace, harness and open, says Seatbelt requires macOS, and labels container unbuilt pending slice (iii) with Docker present

#### Scenario: A capable macOS host offers Seatbelt
- **WHEN** macOS has a usable system sandbox-exec and the actual Seatbelt implementation
- **THEN** doctor offers seatbelt, harness and open, and its Seatbelt bundle hands line reports an enforced boundary when the site's policy can be built

#### Scenario: Empty PATH and unusable tooling are diagnosed
- **WHEN** macOS has an empty supplied search path, or its located launcher cannot apply a sandbox
- **THEN** doctor distinguishes missing from unusable sandbox-exec and does not offer Seatbelt; harness/open remain offered and container remains unbuilt

#### Scenario: A particular bundle needs more than a launcher
- **WHEN** doctor examines a Seatbelt bundle whose binds, masks or git layout fail preparation checks
- **THEN** its hands line warns with the affected site and exact unsupported policy, even if the machine-level boundaries line finds the tool

#### Scenario: A harness bundle needs no Seatbelt or bubblewrap
- **WHEN** doctor examines a valid harness bundle on a host without either tool
- **THEN** the hands availability line stays healthy under harness and does not claim Brokkr enforcement
