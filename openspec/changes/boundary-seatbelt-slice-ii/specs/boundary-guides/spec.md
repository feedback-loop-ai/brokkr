## MODIFIED Requirements

### Requirement: The guides document the boundary and never lose a section
The guides SHALL be updated as follows, each section kept and amended,
none removed: `docs/guides/provider-adapters.md`'s Hands section
documents `hands.harness` with its `gate`, `work` and `result` members,
the `{result_path}` token and the two workspace tokens refused there,
the three-shape convention, codex's fragments and its `last-message`
door — the fragment ruling 4's own word and the door the tool's
documented capture, with the measurement of the capture under the
read-only class recorded when made and named as the operator's and
pending until then — and, per claude member, either the measurement,
the claude version and what the mode denies and allows, or that the
member is undeclared pending the operator's measurement, with the
candidates and the recipe; and its doctor section names the
`boundaries` line, actual host/tool availability and the distinction between
an implemented boundary and pending native acceptance evidence;
`docs/guides/recipe-authoring.md`'s `hands` row stops saying Linux only
and says the boundary lives in the realm and that under `harness` and
`open` a bind's `mask` is declared and not enforced and that clearing
the environment confines nothing on disk, an unboxed script reaching
any host path the operator's uid may read (design DD10), and its
`driver.confine` row
says the field is refused and points at decision 0046 ruling 5;
`docs/guides/quickstart.md`'s platform paragraph says that macOS is
supported on arm64 and x86_64 (0049), that a macOS realm declares
`boundary: seatbelt` after its complete policy and peer prerequisites are
resolved and implemented, and how doctor diagnoses the fixed trusted
`/usr/bin/sandbox-exec`, its required literal `/usr/bin` PATH entry and its
real allow/deny readiness probe. While activation is unresolved it states
that Seatbelt is unbuilt for slice (ii). During preparation it names the
pending Mac measurement and does not claim slice II complete. Its init
advice follows the same macOS status and never silently selects a boundary.
Windows remains best-effort under
`harness` or `open`, Linux/WSL2 keeps `namespace`, absence still resolves
`namespace`, and container refuses until slice (iii). The paragraph keeps
the harness alternative and its *unboxed* meaning, provider-fragment and
pinned-script limitations, and names the shipped bundles that compile or
refuse under that boundary using the current compile inventory and each
actual first refusal. The inventory is measured from the current shipped
library, not copied from slice I's historical nine/thirteen counts; the
existing pin test is the record of the current inventory;
`docs/guides/journal-and-verification.md` gains the unboxed rendering
and what `boundary` on a record and on `effect/started` means;
`docs/guides/read-surfaces.md`'s seats-table example is refreshed from
the renderer's header line, which already carries `model`, and shows the
`boundary` column beside it, and its verb list gains `brokkr seats`
beside `inspect`, its `--json` named as `inspect`'s own view model;
`docs/guides/quickstart.md`'s `rerun` line says the
rerun compiles in the discovered realm as `run` does; no guide states
that the network was off under `harness` or `open` — the prefix is
described as a narrowing the engine attempts on Linux (design DD15);
`docs/guides/repository-layout.md` names `boundary` beside
`house` and `dialect` in the `realms.json` row and the new contract
files in the `contracts/` row; `docs/guides/driver-authoring.md` and
`ARCHITECTURE.md` stop describing the `docker run` wrapper as a trust
class and point at the boundary, and driver-authoring's opening
paragraph, which says the engine runs the shipped verifier and shipper
through `brokkr hands exec`, says so of `namespace` and `seatbelt`,
explains their distinct path handling, and adds that under
`harness` and `open` the same `exec` dispatch runs with no verb of
Brokkr's around it, in a fixed environment, the network narrowed on Linux
where `unshare` permits and never stated as off, that the script's
containing directory and descendants are re-walked
against the declaring layer's compiled file map at every unboxed spawn
and a changed, missing or added file refuses the dispatch (0048), while
helpers outside that directory and the interval between the re-walk and
exec remain outside the integrity check. It preserves 0049's statement
that inherited PATH leaves the interpreter unpinned under harness/open,
and that the `hands` subcommand gains no
verb for it; the two blueprint pages that still present the container trust class follow the same
way, every section kept — `docs/extension-model.md`'s seat-field table,
whose `trust` row says the tier "decides what the engine mounts into
the sandbox", says the wall itself is the realm's `boundary` (decision
0046) and that the tier decides what is mounted inside it, and
`docs/target-architecture.md`'s runner table, whose `policy-confined`
row is an OCI container with a pinned digest, points at decision 0046's
`container` boundary — declared by the realm, refused at start until
slice (iii) measures it — and whose `public-evidence-only` row names
the same boundary for its container form, each page's status line
untouched; and `contracts/README.md` gains rows
and a paragraph for `realms.v4`, `run-manifest.v9`, `seat-record.v4`
and `effect-boundary.v1` in the style of the rows before them.

#### Scenario: provider-adapters documents hands.harness
- **WHEN** `docs/guides/provider-adapters.md` is read
- **THEN** its Hands section names `hands.harness`, `gate`, `work`, `result`, `{result_path}`, the codex fragments and door with the door's measurement recorded or named as pending and the operator's, and, per claude member, the claude version it was measured against or that it is undeclared pending the operator's measurement; and its doctor section names the `boundaries` line

#### Scenario: recipe-authoring points the two rows at the realm and at 0046
- **WHEN** the site vocabulary table is read
- **THEN** the `hands` row says the boundary is the realm's, that a mask is not enforced under `harness` and `open`, that clearing the environment confines nothing on disk, and no longer says Linux only, and the `driver.confine` row says the field is refused under decision 0046 ruling 5

#### Scenario: quickstart's platform paragraph
- **WHEN** the quickstart's platform paragraph is read
- **THEN** it identifies supported macOS with Seatbelt's actual activation/implementation status and any pending native measurement, best-effort Windows with harness/open, namespace as the default on Linux/WSL2, and container as unbuilt slice (iii); it keeps the harness alternative with its unboxed meaning and the actual measured shipped-bundle inventory and refusals

#### Scenario: journal-and-verification and read-surfaces show the rendering
- **WHEN** the two guides are read
- **THEN** one explains the unboxed rendering and the record's `boundary`, and the other's seats table carries a `boundary` column beside `model` and its verb list names `brokkr seats`

#### Scenario: The layout, driver and architecture pages follow
- **WHEN** `repository-layout.md`, `driver-authoring.md` and `ARCHITECTURE.md` are read
- **THEN** the realm row names `boundary`, the contracts row names the four new files, no page describes `driver.confine` as a working trust class, and driver-authoring's opening paragraph qualifies `brokkr hands exec` with namespace and Seatbelt, explains their actual input paths, describes the unboxed dispatch and script-directory integrity check with its interpreter and timing limitations, and names no new unboxed verb

#### Scenario: The blueprint pages point at the boundary
- **WHEN** `docs/extension-model.md` and `docs/target-architecture.md` are read
- **THEN** the seat-field `trust` row names the realm's `boundary` as the wall and decision 0046, the runner table's `policy-confined` row names the `container` boundary and slice (iii) in place of a working OCI wrapper, no row is removed, and each page keeps every section and its status line

#### Scenario: The contracts README lists the four files
- **WHEN** `contracts/README.md` is read
- **THEN** it carries rows for `realms.v4`, `run-manifest.v9`, `seat-record.v4` and `effect-boundary.v1`, and its extension-schema paragraph names `effect/started.boundary` among the fields `fold` never reads

#### Scenario: Current inventories replace obsolete slice-I counts with evidence
- **WHEN** a guide's harness inventory is refreshed alongside the Seatbelt platform text
- **THEN** the counts, names and refusal reasons match the current compile pin's measured output, the reason for any changed inventory is recorded, and historical examples and archived slice-I evidence are not rewritten as current results

## ADDED Requirements

### Requirement: Seatbelt documentation names its mechanism and evidence limits

Hands CLI help, the MCP tool description, relevant model prompts, the
provider/recipe/driver guides and architecture documentation SHALL describe
Seatbelt's actual filesystem, network and process restrictions. They SHALL
not claim an empty root, hidden host process namespace, Linux UID/capability
mapping, PID/IPC/UTS isolation or mount remapping that Seatbelt does not
provide. The guides SHALL distinguish the provider harness outside the
boundary from commands using workspace hands, retaining decision 0043's
Codex native read-only shell limitation instead of claiming the harness
cannot read any host credential.

The guides SHALL show the realm declaration and both hands entry points and
explain private per-call HOME/tmp versus seat-scoped overlay state. For every
overlay they SHALL document the `BROKKR_HANDS_OVERLAYS` JSON array's
`index`, `declared_path` and replacement `locator`, stable seat lifetime,
new seat/attempt isolation, later-source invisibility, and exclusion from
portable identity. Cargo and npm variables SHALL be described as consumers
of the same generic mapping. The original path is not the writable view.

They SHALL describe Seatbelt masks as `EACCES`/`EPERM` denied reads with no
content, not readable-empty; ordinary neighbors remain usable and namespace
keeps its current behavior. Git documentation SHALL say that original host
hooks are denied while ordinary Git uses an empty private hooks directory,
and that only independent raw hook/config/routing write protection proven
in primary and linked worktrees permits full-peer status. It SHALL also cover
controlled toolchain/SDK paths, `ro`/`rw`, network off/on, output/deadline
limits, unsupported-layout refusals and both entry paths.

The process section SHALL name the **per-invocation transient launchd lease
pair** as the unproven R3 candidate: one payload job whose job/process
coalition is the candidate containment domain, plus a separately
launchd-owned guard job in the same per-user bootstrap domain. It SHALL state
that the guard is not a member of the payload job it must `bootout`, survives
payload teardown and engine-liveness EOF, establishes quiescence before
cleanup, and then unregisters itself. It SHALL state that process-group kill,
PID polling, `kqueue`, source reasoning, mocks and Linux runs are not the
guarantee, and that full implementation cannot proceed until real macOS
setsid/double-fork timeout, cancellation and supervisor-death adversaries
pass. The network explanation SHALL distinguish shared macOS loopback from a
private Linux network namespace; a host loopback allowance is not isolation.

The evidence guide SHALL preserve native CI `34433461814` for candidate
`6a19a6f4ab9bd30b47537de1a649949cd1099d01` as a startup failure on macOS
26.6.2 arm64: direct sandboxed `/usr/bin/python3` ended in `SIGABRT` with
empty output, the launchd payload registered but recorded one crashed run, and
no heartbeat reached any lifetime trigger. It SHALL explicitly say that this
establishes neither survival nor containment and does not identify the startup
cause. The next action SHALL name staged outside-box, direct-sandbox and
launchd-owned native-helper controls plus repaired lifetime observations,
without reopening the already repaired git-metadata or `/usr/include` runner
prerequisites.

It SHALL also preserve native CI `34441725835` for candidate
`8c53dcecaef414938b3abfb8911a77d9ec958f23` as a second failed startup
measurement on the GitHub `macos-latest` arm64 runner. It SHALL distinguish
the direct unboxed Rust-helper pass, exact-profile signal 6 before `READY`,
and labelled non-passing `allow default` diagnostic. It SHALL report the
launchd results as incoherent measurement evidence—alternating bootstrap
failure and missing terminal counters across repeated Gate A execution—not as
a launchd or lifetime verdict. The next action SHALL require unique labels and
roots, pre/post label-absence checks, raw bootstrap/kickstart/print/bootout
status and bounded output, a recorded host version, and one-authority-at-a-time
profile diagnosis. The Windows helper link failure SHALL be reported as a
portability repair, never as macOS enforcement evidence.

It SHALL preserve native CI `34449331270` for candidate
`9f4c2c944cac217ccb8dc055971cc62614313ed4` as a third failed Gate A
measurement. It SHALL record that generic macOS and Windows workspaces and S0
passed, while S1 aborted before stages, seven single-class diagnostics failed,
and only the non-admitting `allow default` control started. It SHALL describe
S2 as an observation/parsing refusal, S3 as one non-ready crashed run, denial
controls as unobserved and Gate B as not run. The next action SHALL name
normalized profile-template comparison, field-wise lossless launchd evidence
and native operation/target attribution; it SHALL NOT request an unchanged
retry or present concrete private-root profile digests as authority drift.

It SHALL preserve native CI `34457208029` for candidate
`fa7ece587178a46baa66a7310e0546bfb87a0857` as a fourth failed Gate A
measurement. It SHALL record that the root-inode read moved S1 and S3 past the
pre-stage abort to `executable`, and that the ordinary-child spawn then failed
with `EPERM` and exit 2 without `READY`. It SHALL record that every one-class
diagnostic failed the same way and only the non-admitting `allow default`
control started. It SHALL record that the credential-read, host-write and
loopback-bind denial controls were observed denied, without closing any R1–R4
residual. It SHALL describe S2's `READY`, stages and ordinary child together
with its printed `not running`, one run and exit 0, and S3's printed exit 2. It
SHALL report the omitted crash counter as unknown, never as zero; the report's
erasure of those printed facts is a parser defect, not a launchd verdict. It
SHALL state that the root-inode removal control was not observed and that Gate
B did not run. The next action SHALL name child-spawn sub-stage attribution and
field-wise launchd parsing. It SHALL NOT request an unchanged retry or present
a broad diagnostic as the cure.

While any SEATBELT-R1 through R4 evidence remains open, examples SHALL be
labeled accepted target behavior and Seatbelt SHALL be described as unbuilt.
Refusing shipped overlays is an unmet deliverable, not an optional limitation.
An implementation-only Linux handoff SHALL give the exact native invocation
and expected cases without inventing a success count. The accepted addendum
SHALL be cited without implying evidence. A new semantic decision remains
`proposed` until the operator rules; frozen contracts change only additively,
and historical examples and channel versions retain their meaning.

#### Scenario: A Mac operator can tell what the boundary provides
- **WHEN** the operator reads the hands description and macOS guide examples
- **THEN** they see the actual policy, locator/mask/hooks/lifetime observables, activation status and diagnosis, with no claim of Linux namespaces or a nonexistent `/runtime/bundle` mount

#### Scenario: Native Codex tools are not conflated with workspace hands
- **WHEN** the guide describes the Codex workspace adapter under Seatbelt
- **THEN** it states that Brokkr confines workspace-tool commands, the provider harness remains outside, and the native read-only shell limitation recorded by 0043 still applies

#### Scenario: Preparation and measurement have different evidence
- **WHEN** the guide or handoff is written before native macOS and host exact-coverage runs exist for the candidate
- **THEN** those checks are pending with controller-owned invocations and required outcomes, no activation or completion is asserted, and later evidence is attributed to its actual candidate and host

#### Scenario: A failed startup run is reported at the layer it measured
- **WHEN** the guide reports native CI `34433461814`
- **THEN** it names candidate, host, direct `SIGABRT`, launchd crash state and absent ready signal, says no lifetime case ran, and directs the next run to prove startup before triggering teardown

#### Scenario: The successor startup run preserves each distinct fact
- **WHEN** the guide reports native CI `34441725835`
- **THEN** it names candidate, runner/architecture, missing recorded host version, direct-unboxed success, exact-profile signal 6, non-admitting broad diagnostic, incoherent launchd observations, Gate B not run and the Windows portability failure without turning any of them into a containment verdict

#### Scenario: The third startup run names measurement defects
- **WHEN** the guide reports native CI `34449331270`
- **THEN** it separates S0 and workspace controls from S1's signal 6, the non-admitting diagnostics, S2's parsing refusal and S3's non-ready crash; it says denial controls and lifetime were not observed and directs the next candidate to normalized-policy, lossless-launchd and operation-level diagnosis

#### Scenario: The fourth startup run separates progress from proof
- **WHEN** the guide reports native CI `34457208029`
- **THEN** it names the progress to `executable`, the child-spawn `EPERM`, the three observed denials, S2's printed clean terminal facts with an unknown crash counter, S3's printed exit 2, the unobserved removal control and Gate B not run, and calls none of startup, launchd containment or any R1–R4 residual proven

#### Scenario: Accepted observables and proof status stay adjacent
- **WHEN** documentation describes private locator snapshots, denied masks or the private-hooks view
- **THEN** it cites the accepted addendum, states the exact observable and native evidence status, and preserves the unbuilt refusal until every guarantee is demonstrated

#### Scenario: Missing native proof is not a working macOS example
- **WHEN** Seatbelt has only a launcher, bind-free experiment, generated profile, Linux reasoning or an unproved launchd candidate
- **THEN** the guide says slice (ii) remains unbuilt, lists open residuals and the next native action, and never presents shipped bundles or full-peer gates as runnable

#### Scenario: Init advice and launcher diagnosis agree
- **WHEN** the macOS setup instructions explain init and doctor
- **THEN** they name the fixed launcher, literal `/usr/bin` PATH entry, readiness probe, open evidence residuals and actual activation status; harness remains explicitly unboxed and init never writes the realm choice
