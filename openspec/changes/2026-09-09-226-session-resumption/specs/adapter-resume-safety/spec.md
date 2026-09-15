## Purpose

Allow each model adapter to rejoin its own session only when the current
invocation's restrictions can be re-imposed, with measured provider behavior
and bounded cold recovery (decision 0030; proposed extension reserved as
decision 0056).

## MODIFIED Requirements

### Requirement: AS1 Resume support is measured per adapter and execution shape

Codex, Claude, DSH and LaneTally SHALL each have an explicit, reviewable support
assessment. It SHALL identify the CLI or wrapper version and source actually
assessed, the installed version to which it applies, actual invocation form,
root session selection, settings precedence, relevant boundary/class, and the
evidence establishing restriction enforcement and resume confirmation. The
assessment SHALL distinguish dated installed help/source, deterministic shim
observations and live enforcement probes. Help that lists a flag proves its
interface, not its enforcement. Supplied dated controller evidence SHALL be
used even when the implementation seat cannot access the binary itself.

Each measured safe supported work-site shape SHALL be implemented. At minimum,
Claude SHALL take eligible work-site offers with its complete shipped boxed
workspace hands fragment under an already-supported boxed boundary, and DSH
SHALL take eligible offers in its already-admitted headless work shape. Each
minimum requires dated provider evidence of exact root-session rejoin, current
restriction enforcement and current-invocation accounting, alongside the
implementation and deterministic tests. LaneTally SHALL be assessed independently.

The DSH minimum SHALL qualify the latest official core release,
`@deepseek-ai/dsh` 0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e`,
or the release that resolution at qualification time selects in its place,
together with a repository-owned adaptation of the `dsh-plugin-cli-session`
0.2.0 extension at `0f487e74c81ed102c6899440d9f5d65e8e9eabda`. Resolution SHALL
happen once, before the qualifying seat installs or verifies the core it
measures, and SHALL select the version the registry's `latest` dist-tag names.
A version named only by another tag, such as `next` or `alpha`, SHALL be
recorded and SHALL NOT be selected without an operator ruling, even when its
semver precedence or publish time is higher. A `latest` tag that names a
version of lower precedence than 0.1.5-rc.1 SHALL NOT select it: 0.1.5-rc.1
stays selected and the tag state is recorded. When the live registry is
unreachable, resolution SHALL apply the same rule to the registry document
cached by the task-owned installation, SHALL record the resolution as cached
with that document's fetch time and the failed live attempt, and SHALL NOT
claim that no newer release existed. The assessment SHALL record the exact core
version, the dist-tags, versions and publish times it read, how and when it was
resolved, and the complete resolved dependency identity. It SHALL establish
that this exact pair loads through DSH's documented extension and
`agents.resume` APIs.

The adaptation SHALL be the plugin's published package file set at the
upstream commit (`package.json`, `lib/index.js`, `lib/startup.js`,
`cordis.patch.yml`, `README.md` and `LICENSE`), committed as those bytes and
not rebuilt. Its delta SHALL be measured as the difference between that file
set and the same files at the upstream commit. The delta SHALL consist only of
the `lib/index.js` expression that reads the post-turn event interval, which
uses the core's declared public session-event accessor in place of the removed
`agent.session.events`. The upstream TypeScript source, tests, build
configuration, lockfiles and CI files SHALL NOT be vendored. A provenance note
SHALL record the upstream commit, the upstream source line the delta
corresponds to, the delta and the digests; it SHALL live outside the file set
and is not part of the delta. No Node or TypeScript build SHALL run for the
adaptation, so no build toolchain joins its identity, and the upstream
typecheck and test scripts need not pass against their 0.1.0-rc.6 development
dependencies. The adaptation SHALL keep the upstream package manifest and CLI
surface (`--new`, `--session <id>`, `--output-format stream-json`, the
`agents.resume` path and the `firstSeq` interval) and SHALL add, widen or admit
no tool. Its upstream commit, delta digest and per-file digests SHALL be
recorded and SHALL join the composite identity that admits the route. The bytes
installed into a DSH profile SHALL match them. It is extension-boundary source
this repository owns and pins, which Brokkr SHALL NOT build, install, load or
execute outside DSH's plugin loader.

The adapted package SHALL never be published to a registry from its provenance
directory. The sibling provenance note SHALL state that prohibition and
identify the retained upstream manifest fields as provenance, not a grant to
publish under that identity. Adding `private: true` to the pinned manifest is
rejected for this adaptation because it would introduce a second byte delta
and invalidate the qualified file-set proof; the rule belongs outside that set.

No older core SHALL be selected to avoid adapting to the latest one. Evidence
measured on another core, including the superseded 0.1.0-rc.6 pin, SHALL NOT
transfer, and a different resolved core, plugin revision or adaptation digest
SHALL NOT inherit evidence. Core 0.1.5-rc.1's removal of the accessor the
upstream plugin reads is a measured pair incompatibility and SHALL NOT be
restated as a global DSH or extension limitation. If the selected core exposes
no lawful replacement for a surface the plugin needs, the assessment SHALL
record the missing module, symbol and version with the probe that proves it,
keep the shape disabled and report the unmet requirement with that upstream
ask. It SHALL NOT fabricate the surface. Enablement SHALL additionally
establish that `--session` selects the owned provider-confirmed root, the
invocation remains in the admitted headless shape, current model/effort and
restriction settings take precedence, and the adapted plugin's sequence
boundary yields only current output and attributable usage.

The installed 0.1.2-rc.1 headless runner's lack of a selector establishes only
that its one-shot entry cannot forward the TUI example. It SHALL NOT be reported
as a global DSH or extension limitation. Qualifying and implementing seats SHALL
install and exercise the selected core/plugin pair only under the worktree or
task-owned storage, and SHALL leave the live global DSH installation, its
profiles and credentials, and other runs byte-unchanged, proven by snapshots
taken before and after.

At run time the DSH adapter SHALL resolve its executable through its existing
seam (`BROKKR_DSH_BIN`, then `FORGE_DSH_BIN`, then `dsh` on PATH) and its DSH
home as the shipped driver already does (`$DSH_HOME` when set and non-empty,
otherwise `$HOME/.dsh`), and SHALL launch that home's admitted `headless`
profile. Before provider work it SHALL recompute the composite identity of that
executable, the Node runtime and resolved dependencies it loads, the installed
plugin bytes, the Cordis patch and the composed `headless` profile, and compare
it with the qualified composite. Only a match SHALL take an eligible offer or
construct the plugin's `--new` and `--session` launches. A mismatch or an
unreadable identity SHALL decline any offer as `unverified-harness`, run the
shipped cold invocation unchanged and record no offerable root.

The qualified composite SHALL be the value of an optional `wrapper_digest`
member of the declaration's measured identity form, beside `version` and
`applies_to`. It SHALL use seat record v5's `root_session.wrapper_digest`
grammar, 64 lowercase hexadecimal characters. The loader SHALL admit the member
only in the measured form and SHALL refuse it beside `unknown`, and a shape
without it SHALL remain loadable. `applies_to` SHALL remain the version the
`--version` probe compares. The DSH planner SHALL treat a `supported` shape
without the member as `unverified-harness`. The composite recompute SHALL run
only where the gate is already open, like the version probe. Where it runs,
the planner SHALL compare the probed version with `applies_to` and the
recomputed composite with the declared `wrapper_digest`. On an offer it SHALL
also compare both with the values the originating root recorded. A confirmed
root SHALL record the observed digest in `root_session.wrapper_digest`. The
canonical composite SHALL NOT depend on the absolute locations of the
executable or home, or on the per-seat overlay Brokkr stages. The reference
SHALL be computed by the delivered Rust canonicalization over the qualified
composite, recorded with the qualification evidence and written into the
declaration and its packaged or scaffolded equivalents in the same edit that
enables the shape. Before that edit the shape stays unenabled and its gate
closes before any probe. Every DSH seat then runs the shipped cold invocation
unchanged, whatever its home holds. Re-qualifying a different composite SHALL
change `version`, `applies_to`, `wrapper_digest` and `evidence` together in the
declaration, and SHALL NOT require a code change. A constant compiled into
Brokkr, an originating root's recorded digest, or a qualification file read at
run time SHALL NOT serve as the reference. Brokkr SHALL
NOT install, compose, update or remove a DSH package, plugin or profile, and
SHALL add no configuration surface for doing so; the per-seat model, effort and
persistence overlay stays the only file it stages for DSH. Deploying the
qualified pair into a DSH home that ordinary runs resolve is an operator action
outside Brokkr, recorded as an operator ruling. It does not gate this change's
delivery and is not a hidden precondition of it. The qualification and the Rust
route's end-to-end proof SHALL reach the task-owned installation through the
same seams, so they exercise the adapter's actual resolution and argv.

A conditional adapter-owned Cordis extension for a demonstrated missing
policy or pre-work observation SHALL have a declared repository location and
provenance tied to that measurement and the documented hook it uses. Its
committed runtime file set, licence and per-file digests SHALL be recorded
outside the hashed file set; the installed bytes SHALL match them and SHALL
join the same composite through the delivered Rust canonicalization. No such
extension SHALL be authored or composed merely because it is permitted. It
SHALL NOT expand the session plugin's single-expression adaptation, add a
second runner or admit more tools.

This session-selection work is independent of the deferred plugin
for replacing native tools with boxed hands. It SHALL preserve Rust-only Brokkr
production, the admitted headless profile, existing trust and boundary
restrictions, and per-invocation settings; it SHALL NOT patch provider packages,
intercept UUID generation, introduce a second non-Rust Brokkr runner, substitute
TUI or SDK execution, or admit DSH hands. Installing the digest-pinned,
repository-owned adaptation into the isolated profile is not a patch of an
installed provider package. Selecting an owned session through a documented
provider extension does not replace or authorize more tools (decisions 0009 and
0030).

Previously supported Codex work-site shapes SHALL remain delivery requirements,
subject to installed-version remeasurement. A CLI/wrapper version change SHALL
invalidate the prior version's enablement evidence for the affected resume
shapes until their interface, current restriction enforcement, exact-root
confirmation and current-only accounting are re-established on the installed
version. Unmeasured shapes SHALL remain disabled during preparation, including
historically supported Codex shapes after version drift. Such a disablement
SHALL remain incomplete delivery, not satisfy the Codex preservation obligation.
A shape whose assessed version still matches needs no repeated live experiment
solely because another attempt begins. Historical acceptance is not a perpetual
grant for a changed binary: the enabled-until-contradicted interpretation is
rejected because 0030 measured a silent loss of restrictions on resume.

Unavailable enforcement evidence SHALL keep the affected resume shape disabled
and the requirement incomplete. Common plumbing or provider-specific code
prepared behind a disabled declaration SHALL NOT count as supported resume or
full feature delivery. Provider-specific construction SHALL use identified
interface evidence and SHALL NOT invent argv or provider event semantics. Known
interface evidence SHALL produce the corresponding launch-construction,
validation and deterministic-test work even when enforcement is still pending;
missing local binaries SHALL NOT erase supplied interface evidence or excuse
that work. Missing confirmation/accounting observations SHALL be tracked
separately from known CLI grammar. Common offer/launch plumbing and its tests
SHALL still be implemented when no provider-specific interface is available.

A measured inability to satisfy a minimum SHALL be reported as a failed
requirement for return to the owning specification, not silently converted into
completion or permission to weaken restrictions. The cold-only interpretation
is rejected because it preserves the work-session loss this feature must fix.
Unsupported shapes SHALL be declared with their measured reason; unavailable
or unmeasured evidence SHALL be labelled as such. An adapter SHALL NOT enable a
path on an assumption about inherited settings, treat missing evidence as a
measured CLI defect, or leave a measured safe path cold to avoid implementation.
Support declarations, packaged equivalents, guides and proposed decision 0056
SHALL agree on evidence, scope, version qualification and limitations.

Measurements SHALL use installed help/source first and only necessary bounded
probes with temporary test data. They SHALL NOT change global provider settings,
read unrelated sessions, invent CLI syntax or telemetry, or use an unnecessary
model experiment. Existing accepted measurements remain identified as historical
rather than relabelled as current probes. A superseded measured pin SHALL remain
recorded as dated history; its reversal SHALL be added as a new dated entry and
SHALL NOT rewrite the earlier measurement. In an adapter declaration, a resume
shape's `status`, `identity`, `evidence` and `reason` SHALL describe the current
pin, and its `limitations` list SHALL be the append-only dated ledger. Existing
entries SHALL keep their bytes and order. A reversal SHALL be appended as a
string that begins with its ISO date and names the ruling, the reversed pin and
the superseded evidence file. The closed resume shape SHALL gain no history
field for this purpose. A superseded entry followed by its dated reversal is
read as history, not as a current claim.

#### Scenario: An installed help flag without enforcement proof
- **WHEN** installed Claude help lists explicit resume and permission controls but no observation establishes that the complete restriction set binds on resume
- **THEN** the interface is recorded as available, its construction and validation work proceeds from that evidence, and enforcement remains unmeasured so the affected resume shape is not enabled

#### Scenario: Supplied interface evidence while the seat lacks the CLI
- **GIVEN** the implementation seat has no provider binary on PATH but dated controller captures identify Claude 2.1.266's interface, same-root continuity, Read-grant replacement and partial native-tool/MCP removal
- **WHEN** it prepares the adapter
- **THEN** it uses each positive observation for explicit-handle construction, restriction planning and deterministic validation without relabelling partial evidence as full admission
- **AND** explicit removed-tool/MCP enforcement, the complete filesystem boundary and exceptional visible-message/turn/usage attribution remain separate missing observations; the absence of a local executable or a worker-home write failure is not a controller blocker

#### Scenario: Headless startup lacks a resume selector
- **GIVEN** installed DSH 0.1.2-rc.1's one-shot headless runner mints a fresh root, while official core 0.1.5-rc.1 at `183f08e9c6dde7e36cd2318eaee70b0da08fb35e` and a repository-owned adaptation of `dsh-plugin-cli-session` 0.2.0 at `0f487e74c81ed102c6899440d9f5d65e8e9eabda` expose explicit resume through the documented extension API
- **WHEN** the DSH headless minimum is qualified and implemented
- **THEN** an isolated installation records and verifies that exact resolved pair, including the adaptation's upstream commit and byte digests, and `--session <owned-root>` demonstrably rejoins the provider-confirmed root under the current headless profile, model/effort and restriction settings
- **AND** the plugin's request-derived `session_id` echo and post-`firstSeq` last-wins usage selection are treated as interface evidence only; independent root identity and attributable current-usage observations are required
- **AND** the plugin's sequence boundary demonstrably excludes historical output, tools and usage from current invocation evidence; any unattributable total remains absent and keeps the shape disabled where the boundary itself is uncertain
- **AND** the live global DSH pin and profiles remain unchanged, the deferred hands/tools plugin does not exclude session integration, and no TUI flag, SDK runner, package patch, UUID interception or unsupported hands shape substitutes for the selected route
- **AND** incompatibility or failed enforcement is reported as the exact unmet AS1 requirement; the older one-shot limitation is never repeated as a global DSH limitation

#### Scenario: The latest DSH core removed an accessor the plugin reads
- **GIVEN** the unmodified plugin runs both model turns on core 0.1.5-rc.1 and then fails its post-turn fold with `dsh: events is not iterable`, and that core's `@deepseek-ai/dsh-session` declares `Session.snapshotEvents(fromSeq?, toSeqExclusive?)` and no `events` member
- **WHEN** the DSH route is adapted and qualified
- **THEN** the repository-owned adaptation reads the interval from `firstSeq` onward through that declared accessor, and this accessor change is its only delta from the upstream plugin
- **AND** the evidence names the module, symbol, signature and relation to `firstSeq`, with the hashes of the bytes that declare and implement it
- **AND** the route is not re-pinned to 0.1.0-rc.6 or any older core, the installed package is not patched, and no runtime shim restores the removed member
- **AND** any change to the adaptation's bytes gives `unverified-harness` before provider work, exactly as a core change does

#### Scenario: The selected DSH core offers no lawful replacement
- **GIVEN** the selected core's declared surface cannot supply a fact the plugin's fold needs, and a bounded probe reproduces the gap
- **WHEN** the DSH route is assessed
- **THEN** the shape stays disabled, and the record names the missing module, symbol and version, the reproducing probe and the residual, with that upstream ask for the provider
- **AND** no older core, fabricated accessor, installed-package patch or global DSH limitation claim fills the gap

#### Scenario: A newer DSH core is published before qualification
- **GIVEN** registry resolution at qualification time finds that the `latest` dist-tag names a published core release newer than 0.1.5-rc.1
- **WHEN** the DSH pair is qualified
- **THEN** that release is selected and recorded with the dist-tags, versions, publish times and resolution time that selected it
- **AND** discovery and live evidence are repeated against its exact bytes; nothing measured on 0.1.5-rc.1 transfers to it by version proximity
- **AND** a release published after that resolution does not reopen the completed qualification; an installed core that differs from the qualified one is version drift that declines offers as `unverified-harness` until it is qualified

#### Scenario: Only a prerelease channel names a newer DSH core
- **GIVEN** `latest` names 0.1.5-rc.1 while `next` or `alpha` names a different version, whether its semver precedence or its publish time is higher
- **WHEN** resolution selects the core to qualify
- **THEN** the version `latest` names is selected and every other tag's version is recorded as unselected
- **AND** selecting another channel's version requires an operator ruling, and a `latest` tag moved back to an older version selects nothing older than 0.1.5-rc.1

#### Scenario: The registry is unreachable at qualification time
- **GIVEN** the qualifying seat cannot reach the live registry and the task-owned installation holds a cached registry document
- **WHEN** the core version is resolved
- **THEN** the version the cached document's `latest` names is selected under the same rule, and the record marks the resolution as cached with the document's fetch time and the failed live attempt
- **AND** the record does not claim that no newer release existed

#### Scenario: The adaptation is compared as the published file set
- **GIVEN** the upstream plugin at `0f487e74c81ed102c6899440d9f5d65e8e9eabda` publishes `package.json`, `lib/index.js`, `lib/startup.js`, `cordis.patch.yml`, `README.md` and `LICENSE`, and its development dependencies pin core 0.1.0-rc.6 types that declare no `snapshotEvents`
- **WHEN** the repository-owned adaptation is committed and reviewed
- **THEN** those six files are committed as bytes, five byte-identical to upstream and `lib/index.js` differing only in the accessor expression, with no rebuilt artifact, vendored source, lockfile or manifest change
- **AND** the provenance note lives outside the file set, cites the corresponding upstream source line and records the upstream commit, delta digest and per-file digests
- **AND** no Node or TypeScript toolchain runs for it or joins the composite identity, and upstream's typecheck and test scripts are not gates
- **AND** any second difference in the file set, including in the manifest, means the bytes are not the qualified adaptation

#### Scenario: The adapted package retains upstream publication metadata
- **GIVEN** the pinned six-file adaptation retains the upstream package name, version, author and repository and its manifest has no `private: true`
- **WHEN** the adaptation's provenance and deployment instructions are reviewed
- **THEN** the sibling provenance note explicitly prohibits publishing this directory to npm or any registry and distinguishes operator deployment into a DSH profile from publication
- **AND** the six pinned files retain their qualified bytes; changing the manifest to encode that prohibition is refused because it invalidates the digest proof

#### Scenario: An enabled DSH shape at ordinary run time
- **GIVEN** the DSH `headless-work` shape is enabled on the qualified composite
- **WHEN** an eligible DSH site launches through `BROKKR_DSH_BIN`, `FORGE_DSH_BIN` or `dsh` on PATH, with its DSH home resolved from `$DSH_HOME` or `$HOME/.dsh`
- **THEN** the adapter recomputes that executable's and home's composite identity before provider work, and takes the offer through `--session <owned-root>` on the `headless` profile only when it matches the `wrapper_digest` the declaration's measured identity pins
- **AND** Brokkr installs, composes, updates and removes no DSH package, plugin or profile, and stages only its per-seat overlay
- **AND** the qualification and the end-to-end proof reach the task-owned installation through these same seams, while snapshots show the global DSH installation, profiles and credentials byte-unchanged
- **AND** the end-to-end proof is a recorded exchange with the built DSH driver over that installation, while the committed test suite proves the same decisions over synthetic homes and reads no provider installation
- **AND** the composite match concerns the inputs read for launch and only the qualified patch-reload mode can match; it does not attest to later mid-invocation patch changes or replace AS2's current-restriction proof
- **AND** resolving that same home through a symlinked ancestor does not itself change the composite or cause a refusal when the loader selects the same contained bundles; an unresolvable profile boundary or a first bundle target outside its allowed canonical roots makes the identity unreadable, without falling back to a later candidate

#### Scenario: The resolved DSH home lacks the qualified composite
- **GIVEN** the resolved executable and home are the global 0.1.2-rc.1 installation with its plugin-free `headless` profile, or any composite whose core, Node, dependency, plugin, patch or profile identity differs from the qualified one, including a profile bundle added, dropped or reordered, a different patch-reload mode, a home-level patch layer the qualified composite lacked, or a listed bundle that resolves outside the core installation and the profile
- **WHEN** an eligible DSH offer arrives or a cold DSH seat starts
- **THEN** any offer is declined as `unverified-harness`, the shipped cold invocation runs unchanged and no offerable root is recorded
- **AND** nothing is installed into that home, and whether the operator deploys the pair there is recorded as an operator ruling rather than assumed

#### Scenario: The declaration pins the qualified composite
- **GIVEN** 10.7's qualification of the adapted pair on core 0.1.5-rc.1 passes and the delivered Rust canonicalization computes the qualified composite's digest over the task-owned home
- **WHEN** 11.3 enables the DSH `headless-work` shape
- **THEN** the same declaration edit that sets `supported` writes that digest as `identity.wrapper_digest` beside `version` and `applies_to` `0.1.5-rc.1`, in `adapters/dsh.json` and its packaged or scaffolded equivalents
- **AND** a later DSH seat compares its probed version with `applies_to` and its recomputed composite with that digest, and on an offer also compares both with the values the originating root recorded; a confirmed root records the observed digest in `root_session.wrapper_digest`
- **AND** the digest does not change when the same composite is deployed in another home or when the per-seat overlay differs

#### Scenario: npm nested and scoped keys produce reproducible dependency values
- **GIVEN** a hidden npm lock with no `name` fields, including `node_modules/debug`, nested `node_modules/parent/node_modules/debug`, scoped-parent/unscoped-child entries and `node_modules/a/node_modules/@parent/b/node_modules/@scope/child` with version and integrity distinct from shallower `@scope/child` entries
- **WHEN** qualification and runtime compute the same composite through the delivered Rust canonicalization
- **THEN** the key is parsed left to right through every package group, preserving the slash inside each scoped package; each dependency value uses only the terminal package name and that entry's exact `version` and `integrity`, joined by single ASCII spaces, with no parent path or optional `name` field supplying a value
- **AND** the hidden lock is the only npm source, without a root-lock fallback, and three or more package groups obey the same rule as shallow entries
- **AND** identical complete triples produce one dependency line, different versions or integrities remain distinct, and equivalent npm and pnpm entries yield identical value bytes before bytewise sorting
- **AND** malformed paths, including a malformed intermediate group before a valid terminal package, or missing, mistyped or invalid version fields make the identity unreadable and an offer declines as `unverified-harness`; the name is never guessed from a URL or another entry

#### Scenario: A measured missing hook requires the conditional Cordis extension
- **GIVEN** a dated probe demonstrates a missing policy or pre-work fact on the adapted DSH route and a documented Cordis hook can supply that fact
- **WHEN** the conditional extension is prepared at its design-declared repository location
- **THEN** its separate provenance identifies repository authorship, the measured gap, documented hook and core version, limited behavior, licence and complete runtime file digests, and its installed file set contributes the extension component to the qualified composite
- **AND** missing or changed installed bytes or unsafe resolution cannot match that qualification; a changed composite requires re-qualification before enablement
- **AND** without the demonstrated gap no extension is authored or composed, and neither the session plugin's one-expression delta nor the admitted tool surface is widened

#### Scenario: Before enablement a home holding the pair still runs cold
- **GIVEN** the DSH shape is still `unmeasured`, its identity carries no `wrapper_digest`, and the resolved home holds the qualified pair
- **WHEN** a DSH seat starts cold or an eligible DSH offer arrives
- **THEN** any offer is declined as `unsupported-resume` and the shipped cold invocation runs unchanged
- **AND** no version probe or composite recompute runs, no `--new` or `--session` is built and no offerable root is recorded
- **AND** a route overlay the seat carries is validated and folded into that shipped cold invocation exactly as on an enabled shape; the closed gate changes nothing about which `--patch` is admitted

#### Scenario: A supported DSH identity lacks a valid composite digest
- **GIVEN** a DSH assessment marks the shape `supported` but its measured identity has no `wrapper_digest`, or the private start context carries one that does not fit the grammar
- **WHEN** a DSH seat starts cold or with an offer
- **THEN** the gate reads `unverified-harness`, any offer is declined and the shipped cold invocation runs unchanged
- **AND** a declaration that places `wrapper_digest` beside `unknown`, or gives it outside the 64-lowercase-hex grammar, is refused by the loader

#### Scenario: A different DSH composite is re-qualified
- **GIVEN** a newer core, another plugin revision or adaptation, or a different Node runtime, dependency graph or profile is qualified in place of the enabled composite
- **WHEN** its evidence is adopted
- **THEN** one declaration edit changes `version`, `applies_to`, `wrapper_digest` and `evidence` together, with no code change, and the adapter content digest and `instance_ref` move
- **AND** no root opened under the earlier composite is offered, and any that reaches the planner is declined as `unverified-harness` by the per-root comparison

#### Scenario: A superseded DSH pin is reversed in the declaration
- **GIVEN** `adapters/dsh.json` records the 0.1.0-rc.6 pin in its `headless-work` shape, whose closed resume shape has no history field
- **WHEN** the 2026-09-10 ruling's reversal to 0.1.5-rc.1 is recorded
- **THEN** `identity`, `evidence` and `reason` describe the 0.1.5-rc.1 pin, every existing `limitations` entry keeps its bytes and order, and a new entry beginning `2026-09-10` names the ruling, the reversed 0.1.0-rc.6 pin and the superseded `dsh-pair-qualification-010rc6.json`
- **AND** the loader and the closed shape gain no field, and the `hands.unsupported` text is unchanged

#### Scenario: A DSH current interval holds several messages, a tool call or a retried attempt
- **GIVEN** a resumed DSH invocation whose interval from `firstSeq` onward contains more than one assistant message, a tool call and result, or a failed and retried model attempt, after historical sequences in the same root
- **WHEN** its output and usage are folded for the invocation's evidence
- **THEN** only events at or after `firstSeq` contribute, usage is taken per assistant message with each message counted once, and raw sequence and message identities support every contribution
- **AND** a numeric total is reported only when every contributing message and attempt is attributable; otherwise it is omitted, and the plugin's last-wins usage is never reported as the total of a multi-message interval

#### Scenario: An installed version differs from historical Codex evidence
- **GIVEN** the accepted resume measurement identifies codex-cli 0.148.0 and supplied installed-version evidence identifies 0.153.4
- **WHEN** the new adapter's support is assessed for delivery
- **THEN** the exercised binary's version is re-read and reconciled with those dated measurements, 0.148.0 remains historical evidence, and resume on the installed version stays disabled until current resume-subcommand interface and bounded restriction, root-confirmation and accounting evidence qualify that shape
- **AND** supplied later evidence that the host starts the sandbox and enforces a cold read-only class supersedes a prior startup blocker, but does not establish restriction re-imposition, exact root or current accounting across resume
- **AND** preserving Codex support requires completing that remeasurement and delivering its previously supported work shapes; a disabled regression is not full delivery

#### Scenario: The measured installed version still applies
- **GIVEN** a shape has applicable installed-version interface, restriction, confirmation and accounting evidence
- **WHEN** another eligible attempt uses that same assessed version and shape
- **THEN** it can resume without another live experiment solely for the new attempt, while re-imposing the current restrictions and performing the same ownership and confirmation checks

#### Scenario: Evidence establishes a safe shape
- **WHEN** versioned interface and effective enforcement evidence establish safe rejoin for one adapter and boundary/class combination
- **THEN** that combination takes eligible offers and its declaration, tests and documentation describe the measured support

#### Scenario: Neither local nor supplied interface evidence exists
- **GIVEN** the implementation seat's availability check finds no installed provider binary/source and no supplied dated interface measurement for a provider's new path
- **WHEN** it prepares the change without that evidence
- **THEN** common session/launch plumbing and deterministic tests, support declarations and packaged equivalents, guides and the proposed decision record carry the preparable change without invented provider argv
- **AND** the affected declaration says evidence is unmeasured and resume is disabled; measurement and enablement remain unchecked, and that preparation cannot be reported as full delivery

#### Scenario: Host evidence later establishes the missing safe path
- **GIVEN** common plumbing is prepared but a required resume shape is disabled for missing or outdated evidence
- **WHEN** dated installed-provider evidence establishes the required interface, exact-session confirmation, all current restrictions and current-only accounting
- **THEN** the corresponding measured path is implemented and enabled, its declarations and documentation agree, and its measurement and implementation tasks complete only with cited verification
- **AND** remaining unmeasured shapes stay disabled; deterministic shim success alone cannot enable them

#### Scenario: Measurements rule out a required minimum
- **WHEN** measured provider behavior, including investigation of supported DSH headless session-extension interfaces where applicable, rules out a safe path for a required Claude, DSH or preserved Codex work shape
- **THEN** the adapter remains cold with truthful evidence, and delivery reports the unmet AS1 requirement for return to the specification with the measured reason instead of claiming that a safe decline closes the feature

#### Scenario: Wrapper evidence is independent
- **WHEN** Claude's resume path is measured but LaneTally's actual wrapper forwarding and capture behavior are not
- **THEN** LaneTally is not marked supported by analogy and never substitutes plain Claude to make resume work

### Requirement: AS2 Every resume re-imposes the current effective restrictions

A resume SHALL re-express the current seat's declared sandbox or permission
class, permitted tools, MCP configuration, boxed hands fragment and selected
boundary, wherever each applies. It SHALL also retain the selected model,
effort, workdir and result-delivery mode. Temporary grants and result paths
SHALL be those of the new invocation; persistent conversation state SHALL NOT
restore stale grants or add authority (decisions 0030, 0043 and 0046).

If the adapter cannot establish and express all applicable restrictions on the
resume path, it SHALL decline the offer and attempt only a cold path that
satisfies the same current restrictions. If the cold path is itself inadmissible,
it SHALL refuse under the existing boundary/adapter rules instead of dropping
a restriction. Resume SHALL NOT promote trust, add a boundary backend, admit
an unsupported hands shape or change which adapters can hold a gate.

#### Scenario: Codex re-imposes its sandbox and effort
- **WHEN** a Codex invocation with a supported explicit class and effort rejoins its own thread
- **THEN** the effective class and effort are re-expressed through the measured resume interface, including 0030's safe class override, rather than inherited from the old thread

#### Scenario: Claude re-imposes the entire boxed fragment
- **WHEN** Claude resumes a site whose current restrictions include permission mode, MCP configuration, allowed tools and boxed workspace hands
- **THEN** evidence establishes that the current permission mode and complete fragment bind, including removal of native tools and exclusion of ambient MCP servers where required
- **AND** failure to establish any one of those restrictions makes that shape cold with a reason

#### Scenario: Current hands configuration replaces the prior grant
- **GIVEN** the previous attempt's hands configuration contains a now-expired per-attempt capability or result path
- **WHEN** a retry resumes
- **THEN** only the new attempt's scoped configuration is effective, and the previous capability is not an alternative way to operate

#### Scenario: DSH remains in its declared headless shape
- **WHEN** an eligible DSH site resumes under a measured supported path
- **THEN** headless operation, the pinned model/effort overlay and the owned session/transcript relationship remain effective, without falling into an interactive or ambient profile
- **AND** the resolved home's `headless` profile verifies as the qualified composite, the declared `wrapper_digest`, before this invocation; a different core, plugin, profile or selector, or plugin bytes that differ from the pinned repository-owned adaptation, declines the offer instead of being inherited
- **AND** the seat's route overlay, where the bundle carries one, is validated and folded ahead of the Rust-owned rows on the rejoin exactly as on the cold launch; it is a current-invocation setting, never a restored one

#### Scenario: Unsupported hands still refuse
- **WHEN** a DSH or LaneTally site requests a hands shape its declaration does not support
- **THEN** resume support does not admit that site, and the existing refusal is retained

#### Scenario: A cold substitute cannot honour the class
- **WHEN** neither resume nor cold invocation can satisfy the current restrictions
- **THEN** the invocation is refused; it never proceeds with fewer restrictions

### Requirement: AS3 Invocation settings cannot redirect or weaken a resume

The adapter SHALL validate the exact provider handle and all settings carried
to resume using the measured provider grammar and precedence. Alternate or
ambient session selectors, forks, extra positional handles, unsafe identifiers,
conflicting class/tool/MCP/profile settings and unverified passthrough
SHALL NOT redirect an engine offer or override its restrictions.

Engine-composed hands and model/effort settings SHALL be distinguished from
arbitrary passthrough by their authorized shape. A measured safe generated
hands fragment SHALL be supported as such; a broad ban that makes every boxed
site cold is not an implementation of that supported shape. Cold fallback
SHALL NOT pass through an ambient resume selector and silently rejoin a
different session. Codex SHALL refuse bundle-authored `resume` selectors in
driver passthrough before any provider work, including on an initial cold
launch and when the shape is unmeasured. The selector SHALL neither be
forwarded nor silently removed to make the invocation acceptable. No argument
or rejected private value SHALL be copied into launch evidence (decision 0034).

Claude's optional-value `-r/--resume` SHALL always receive the complete validated
owned root ID when selected by the adapter. Bare/empty resume, interactive
picker/search selection, `-c/--continue` and `--fork-session` SHALL NOT be used.
A generated `--session-id` is permitted only for a measured fresh-creation path
under SR3; arbitrary passthrough SHALL NOT supply it, combine it with resume to
redirect an offer or preserve an abandoned creation request as a fake rejoin.
A setting such as `--no-session-persistence` SHALL NOT be silently removed to
make a nonpersistent invocation resumable. Its measured effect on the shape
SHALL be respected and reflected in support and session-eligibility evidence.

DSH's launcher has one override channel, `--patch <overlay>`, and Brokkr's
per-seat persistence/model/effort overlay already rides it. A seat's own
`--patch` SHALL be admitted in exactly one authorized shape, the **route
overlay**: an operator-ruled file of the compiled bundle, read from the seat's
working directory, whose rows state only the provider route of the pinned
model (decision 0044 ruling 5 and its erratum of 2026-09-04; the shipped
instance is `recipes/research-dsh/drivers/research-web.yml`). The adapter
SHALL validate the overlay by its measured shape before provider work, on the
cold and the resume path alike, and SHALL fold its validated bytes into the
per-seat overlay ahead of the Rust-owned rows, so the launcher receives one
`--patch` and the persistence, model and settings rows Brokkr writes apply
last. A route overlay SHALL be the only `--patch` in the seat's settings.

Authorization is a binding to the compiled bundle, not a path or a shape. At
every start of a model site, cold and resume alike and independent of the
resume gate, the engine SHALL bind the seat's single `--patch` value: the
value SHALL resolve, relative to the seat's working directory and without an
absolute path, `..` component or symlink escape, to a regular file inside the
compiled bundle's own layer directory, and that file SHALL be a `files`
member of the compiled manifest, whose recorded SHA-256 the run's witness
identity already pins. The engine SHALL carry that binding in the private
start context beside the assessment, as the argv value and the manifest's
64-lowercase-hex digest, and nowhere else. Before staging or provider work
the adapter SHALL read the named file exactly once from the seat's working
directory, SHALL require the SHA-256 of the bytes it read to equal the bound
digest, and only then SHALL validate those same bytes by shape and fold
them. A `--patch` with no binding, a binding with no `--patch` or whose value
differs from the argv, a file that is not the bound member (a same-shaped
file outside the bundle's layer, or a working-directory shadow of the
bundled path), or bytes whose digest differs from the compiled one (a file
changed since compilation) SHALL refuse the invocation before staging or
provider work on both paths; no unbound byte is staged, forwarded or folded.
A file of an ancestor layer is recorded only through that ancestor's
manifest digest and is therefore not bindable under this rule, and the
bundle-relative `./` spelling expands to an absolute path and is refused as
one; admitting either is a recorded amendment, never an implementation
choice. The binding never enters the prompt, the launch row or the journal.

The admitted shape is a closed, data-only grammar, and its reader is a
bounded line reader of the pnpm reader's discipline, never a YAML
implementation. dsh parses a patch file with a schema that turns a `!!js`
tagged scalar into an expression node, and its loader evaluates any mapping
that holds a `__jsExpr` key when the entry activates, so a deeper line
carried verbatim could carry executable syntax; the reader therefore carries
no line it has not recognized, at any depth. A route overlay SHALL be
bounded UTF-8 text without tabs, control characters or document markers;
SHALL consist only of full-line comments, blank lines and block-form lines of
the forms `<key>: <value>`, `<key>:` and the sequence item `- <key>: <value>`,
indented two spaces per depth; every key SHALL be a plain identifier that
begins with an ASCII letter and continues with ASCII letters, digits, `_` or
`-`, so `__jsExpr`, `<<`, quoted and flow keys are refused; every value SHALL
be a non-empty plain unquoted scalar that begins with none of `!`, `&`, `*`,
`{`, `[`, `|`, `>`, `%`, `@`, `` ` ``, `"`, `'`, `?`, `-`, `:` or `,` and
carries no ` #`; and no tag, anchor, alias, flow collection, block scalar,
merge key or quoted scalar SHALL be admitted at any depth. Within that
grammar the overlay SHALL hold exactly one top-level entry, whose `id` is the
provider-catalogue row the shipped overlay names (`llm-pi-ai`), whose only
member is `config`, whose `config` holds only `providers`, which SHALL define
exactly one provider key, equal to the provider segment of the seat's pinned
`<provider>/<id>` model. The provider mapping SHALL hold only fields of the
closed permitted set the shipped overlay uses, each at most once:
`displayName`, `api`, `baseURL`, `compat`, `models` and the credential
reference `apiKeyEnv`. `apiKeyEnv` SHALL be present and its value SHALL be an
environment-variable name (an ASCII letter or `_` followed by ASCII letters,
digits or `_`), which Brokkr never resolves, forwards or echoes (decision
0012). `baseURL`, when present, SHALL be an endpoint of the closed grammar
`https://<host>[:<port>][/<segment>...]`: the scheme is exactly the
lowercase `https`; `<host>` is one or more labels separated by `.`, each of
one or more ASCII letters, digits or `-`, neither beginning nor ending with
`-`; `<port>` is one to five ASCII digits; each `<segment>` is one or more
of ASCII letters, digits, `-`, `.`, `_` or `~`; and nothing else appears.
The grammar therefore refuses every position in which a URL carries a
credential and every character that could introduce one: userinfo (`@`), a
query (`?`), a fragment (`#`), a percent-escape (`%`), a backslash,
whitespace, brackets, a non-ASCII byte, an empty segment (a trailing or
doubled `/`), a scheme other than lowercase `https` (including `http`, which
would carry the resolved key in clear) and a value with no scheme. The
shipped Model Studio endpoint,
`https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1`,
lies inside it. Of the permitted fields, `baseURL` alone names a network
location and `apiKeyEnv` alone names a credential; `models` is pinned to the
seat's model, and the remaining values are labels or enumerations that dsh
itself validates and that reach no credential position. `compat` SHALL hold
only `<key>: <value>` pairs. `models` SHALL hold exactly one item, whose `id`
equals the id segment of the pinned model and which holds only `id` and
`reasoningEfforts`; `reasoningEfforts` SHALL hold one or more
`<level>: <wire>` pairs, so the shipped `low`, `medium` and `xhigh` levels
survive unchanged. The credential rule is that closed set
together with the value grammars of `apiKeyEnv` and `baseURL`, not the
absence of one named field: `apiKey`, `headers` (a literal `Authorization`
or `x-api-key` value is a credential even beside a valid `apiKeyEnv`),
`modelOverrides`, `reasoning`, transport, timeout, retry and every other
field dsh's provider profile would accept refuse the invocation, and so does
a `baseURL` outside the endpoint grammar, whatever its value carries. A
route that needs a further field or a wider endpoint form is a recorded
amendment of this grammar, never a relaxation of the reader.

Anything else offered as `--patch` — a second file, a bare `--patch`, a row
naming the persistence, model, settings, runner, session or tool rows or any
other id, more than one entry or provider, a provider other than the pinned
one, a route beside a model pin with no provider segment or no model pin, a
credential value in any field, a `baseURL` outside the endpoint grammar,
executable or unrecognized syntax at any depth, an unbound or drifted file,
or a file that cannot be read within the bound — SHALL refuse the invocation
before provider work on both paths, SHALL NOT be forwarded to the launcher
and SHALL NOT be dropped silently; the bounded reason names the depth or
field, never a value. The fetch grant is
the composed `headless` profile's own and enters the composite through its
`profile-bundle` lines; no overlay row grants or revokes it. The folded route
rows are part of the per-seat overlay the composite excludes, their
provenance is the bound manifest digest the bundle digest already covers,
and no route byte enters launch evidence. On a resume the route overlay is
re-imposed from the current bundle, never inherited from the persisted
session, and its admission is independent of the resume gate: an
`unmeasured` shape binds, verifies and folds it into the shipped cold
invocation exactly as an enabled shape does.

#### Scenario: Claude continue is not a selector for this seat
- **WHEN** passthrough requests Claude's continue/latest-session behavior or a different explicit session
- **THEN** it cannot override the exact engine offer, and any cold path cannot silently perform that ambient continuation

#### Scenario: A bare Claude resume opens a picker
- **WHEN** invocation settings contain bare `-r` or `--resume`, an empty value or a picker search term instead of the complete validated owned ID
- **THEN** no ambient selection or interactive picker is launched, and any allowed cold fallback excludes that selector
- **AND** the adapter's supported resume form always supplies the exact engine-offered ID explicitly

#### Scenario: A Claude fork or reassignment redirects identity
- **WHEN** passthrough requests `--fork-session` or supplies `--session-id` to replace or compete with the owned identity
- **THEN** neither a resume nor a cold substitute forwards those controls; only the adapter's measured SR3 cold-creation path can assign a fresh ID
- **AND** an engine-assigned creation ID is never evidence of rejoining the offered root

#### Scenario: Session persistence is disabled
- **WHEN** a site's measured provider settings disable session persistence and therefore prevent future resume
- **THEN** the setting is retained, that shape is declared nonresumable with the measured reason, and a reported or preassigned ID alone cannot override the limitation

#### Scenario: Codex unsafe passthrough remains blocked
- **WHEN** passthrough contains an alternate selector, extra positional handle, sandbox bypass or conflicting configuration
- **THEN** the resume is not invoked and the refusal remains bounded; 0030's allow-list protections are preserved

#### Scenario: A bundle supplies a Codex resume selector on a cold launch
- **GIVEN** driver passthrough after the adapter's `--` contains `resume <id>`, with or without an engine offer, including when the shape is unmeasured
- **WHEN** the Codex adapter prepares the invocation
- **THEN** it refuses before any provider work instead of forwarding or silently dropping the selector, and its bounded diagnostic names the selector without echoing the handle
- **AND** a cold invocation without that selector retains its existing cold arguments and, absent an offer, has no resume-refusal reason

#### Scenario: Generated Codex hands are measured explicitly
- **WHEN** the current boxed Codex invocation contains engine-composed MCP configuration alongside its sandbox class
- **THEN** the adapter either carries the complete measured safe fragment on resume or declares the specific unsupported shape, without admitting arbitrary configuration overrides

#### Scenario: A DSH profile or wrapper can replace session selection
- **WHEN** user passthrough or a wrapper setting would override the headless profile, selected session, model/effort overlay or restriction set
- **THEN** the resume is declined and the safe cold or refusal outcome names a bounded reason
- **AND** only the adapter-owned `--session <owned-root>` value reaches the selected pinned extension route; a user-supplied selector never competes with it
- **AND** a `--patch` other than the admitted route overlay is such an override: it is refused before provider work on the cold and the resume path, neither forwarded nor dropped

#### Scenario: The research route overlay is admitted by its shape and folded
- **GIVEN** a DSH seat pinned `--model dashscope/qwen3.8-max --effort xhigh --patch recipes/research-dsh/drivers/research-web.yml`, as `recipes/research-dsh` ships it, whose overlay's one entry is the `llm-pi-ai` row defining the single provider `dashscope` by `apiKeyEnv`, at its `https` Model Studio endpoint, with its model's declared reasoning levels
- **WHEN** the seat launches cold, or rejoins its owned root through `--session <owned-id>` on a supported shape
- **THEN** the engine binds the value to the member `drivers/research-web.yml` of the compiled `recipes/research-dsh` layer and carries the argv value and that member's manifest digest in the private start context; the adapter reads the file once from the seat's working directory, requires the SHA-256 of the bytes it read to equal the bound digest, validates those bytes by the closed grammar before provider work, and folds them into the per-seat overlay ahead of the persistence, model and settings rows Brokkr writes, so the launcher receives exactly one `--patch` and the Rust-owned rows apply last
- **AND** on a rejoin the route rows are the current bundle's, re-imposed like the model and effort; nothing the persisted session remembers supplies them
- **AND** an `unmeasured` shape folds the overlay into the shipped cold invocation the same way, still without a version probe, a composite recompute, `--new` or `--session`
- **AND** the composite, the launch row and the journal carry no route byte or binding; the file's provenance is the bound manifest digest the bundle digest already covers, and the fetch grant stays the composed profile's own

#### Scenario: A patch names a Rust-owned or selector row
- **WHEN** a seat's `--patch` file carries an entry whose `id` is `session-persistence-jsonl`, `agent-default-model`, `settings`, the pinned plugin's runner row, a tool row or any id other than the route row, or carries more than one entry
- **THEN** the invocation is refused before provider work on the cold and the resume path, the file is not forwarded to the launcher, no row of it is dropped to make the launch admissible, and no launch row or offerable root is recorded
- **AND** the refusal is the adapter's existing pre-work failure to start; it is not a resume decline, not a cold replacement and not a reason to inherit the persisted session's settings

#### Scenario: A patch competes by arity or path
- **WHEN** a seat's settings carry two `--patch` controls, a bare `--patch` with no value, or a value that is an absolute path, contains a `..` component, resolves through a symlink to a file outside the seat's working directory, is not a regular file, exceeds the reader's byte bound, is not UTF-8, or carries a tab, control character, document marker or an unrecognized construct at any depth
- **THEN** the invocation is refused before provider work on both paths with a bounded reason, and no partial overlay is staged

#### Scenario: A route overlay names a provider the seat did not pin
- **WHEN** the overlay defines a provider key other than the provider segment of the pinned `<provider>/<id>` model, defines two provider keys, or accompanies a model pin with no provider segment or no model pin at all
- **THEN** the invocation is refused before provider work; an overlay cannot redefine the profile's default route or a route the seat does not use

#### Scenario: A route overlay carries a credential value or a field outside the permitted set
- **WHEN** an overlay's provider carries an inline `apiKey` value, a `headers` mapping whose `Authorization` or `x-api-key` value is a literal bearer token even though a valid `apiKeyEnv` is present beside it, an `apiKeyEnv` whose value is not an environment-variable name, no `apiKeyEnv` at all, or any field outside `displayName`, `api`, `baseURL`, `compat`, `models` and `apiKeyEnv` — including `modelOverrides`, `reasoning`, transport, timeout and retry fields dsh would accept
- **THEN** the invocation is refused before provider work on the cold, the resume and the disabled-gate path, no value is staged, forwarded, resolved or echoed, and the bounded reason names the field, never its value; a route names its credential by environment variable only, and the closed set together with the `apiKeyEnv` and `baseURL` value grammars, not one named field, enforces that rule

#### Scenario: A route overlay's baseURL carries a credential or leaves the endpoint grammar
- **GIVEN** an overlay that equals the shipped route except in its `baseURL`, beside an unchanged valid `apiKeyEnv`
- **WHEN** that value carries URL userinfo holding a synthetic credential, a query such as `?api_key=` with a synthetic value, a fragment, a percent-escape, a backslash or whitespace, a bracketed address, an empty path segment, a scheme other than lowercase `https` (including `http` and `HTTPS`) or no scheme at all
- **THEN** the invocation is refused before staging or provider work on the cold, the resume and the disabled-gate path alike; no byte of the value is staged, forwarded, resolved or echoed, and the bounded reason names the field and the URL part that broke the grammar (scheme, authority, path, query or fragment), never the value
- **AND** the shipped endpoint `https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1` passes the same check and folds, so one grammar decides the positive case and every refusal
- **AND** the binding to the compiled bundle does not stand in for this check: a bound, digest-matching member whose `baseURL` fails the grammar refuses the same way, because the binding authorizes bytes and the grammar decides what those bytes may carry

#### Scenario: A route overlay carries executable syntax
- **WHEN** an overlay carries a `!!js` or any other tagged scalar at any depth (for example under `displayName` or `baseURL`), a mapping with a `__jsExpr` key at any depth, or a flow mapping or sequence, anchor, alias, merge key, block scalar or quoted scalar
- **THEN** the invocation is refused before staging or provider work on the cold, the resume and the disabled-gate path alike; nothing is parsed as YAML, evaluated or folded, and the shipped reasoning-level mapping is untouched by the rule because its keys and values are plain
- **AND** the refusal names the offending depth, never the text

#### Scenario: The route overlay is bound to the compiled bundle
- **GIVEN** a compiled bundle whose seat names one `--patch` whose value resolves under the seat's working directory to a regular file inside the bundle's own layer, recorded in the manifest's `files`
- **WHEN** the seat starts cold, with an offer, or while the shape is `unmeasured`
- **THEN** the private start context carries the binding as the argv value and the member's 64-lowercase-hex manifest digest, the adapter's single read of the file hashes to that digest before the shape check, and the fold proceeds only then
- **AND** the binding rides beside the assessment and the owned target, never in the prompt, the launch row or the journal

#### Scenario: An unbound or drifted route overlay is refused
- **WHEN** a seat's `--patch` names a same-shaped file outside the compiled bundle's layer, a working-directory path that resolves to a file other than the bound member (a shadow of the bundled path), a bundle member whose bytes differ from the digest recorded at compilation, a file of an ancestor layer, or a bundle-relative `./` spelling expanded to an absolute path; or the private start context carries no binding for a present `--patch`, a binding for an absent one, or a binding whose value differs from the argv
- **THEN** the invocation is refused before staging or provider work on the cold, the resume and the disabled-gate path, no byte of the file is staged, forwarded or folded, no launch row or offerable root is recorded, and the refusal is the adapter's existing pre-work failure to start

#### Scenario: Identifier injection
- **WHEN** an offered handle contains a flag-like prefix, control characters, path traversal, shell syntax or an overlong value outside the measured grammar
- **THEN** it is never passed to the CLI as a selector, never truncated into another handle, and never echoed into the journal

## ADDED Requirements

### Requirement: AS4 A refused resume permits only one proven pre-work cold replacement

For one model-site invocation, a declined offer SHALL cause at most one safe
cold launch, and a provider-rejected resume SHALL permit at most one safe cold
replacement. The latter SHALL require provider-specific measured machine
evidence establishing that the resume did not open a working session, perform
a turn/tool action or deliver the current attempt's result. A generic nonzero
exit, stderr prose, missing telemetry, connection loss or elapsed time SHALL
NOT establish that fact.

Once the resume has confirmed its session, begun work, delivered a result, or
left execution uncertain, the adapter SHALL NOT execute the seat again as
internal cold recovery. Determinate failures and indeterminate outcomes SHALL
follow their existing engine rules. A cold replacement that fails SHALL be
reported without recursive resume/cold attempts. Session rejection SHALL NOT
be confused with the separate provider model/auth/quota refusal rules already
shipped with proposed decision 0053.

#### Scenario: Unknown or expired session is conclusively rejected
- **WHEN** an adapter observes its measured pre-work session-rejection shape for the offered handle
- **THEN** it performs at most one cold replacement and reports that launch as cold with harness-refused evidence
- **AND** the rejected request is not counted as a confirmed resumed session

#### Scenario: An unstructured DSH error
- **WHEN** DSH exits nonzero with stderr text but its measured interface cannot establish a pre-work session rejection
- **THEN** the adapter performs no automatic cold replacement based on that text

#### Scenario: Work precedes failure
- **WHEN** a resumed invocation reports a turn or tool action and then fails
- **THEN** it is a mid-session failure and no internal cold replacement runs

#### Scenario: The provider reports a different session
- **WHEN** a resume request yields a session identifier different from the offered root handle
- **THEN** the adapter does not claim resumed or start another invocation; it reports the mismatch as failure or uncertainty according to the observed execution facts

#### Scenario: A refusal notice is followed by delivery
- **WHEN** a provider emits an error-shaped notice but subsequently works or delivers the current attempt's valid result with a clean exit
- **THEN** the delivered work is retained under the existing acceptance rules and the adapter does not run a cold replacement

#### Scenario: The replacement also fails
- **WHEN** the single cold replacement fails or is rejected
- **THEN** its observed outcome returns to the engine with the refusal evidence, and no further provider launch occurs inside that invocation

### Requirement: AS5 Resume and replacement share the existing execution bounds

A resume and its one permitted cold replacement SHALL share the original
invocation's deadline and cancellation scope; fallback SHALL NOT reset its
time budget or exceed existing attempt and chain bounds. Cancellation or
deadline termination SHALL prevent a later replacement from starting and
terminate the owned child work according to the existing process discipline.
A watchdog kill SHALL NOT be reclassified as a failure to start solely because
acceptance or checkpoints were withheld (decisions 0006 and 0016; shipped
behavior described by proposed decision 0053).

#### Scenario: The refusal used most of the deadline
- **WHEN** a proven session rejection arrives near the end of the invocation's deadline
- **THEN** the cold replacement receives only the remaining budget and cannot obtain a new full deadline

#### Scenario: Cancellation races the replacement
- **WHEN** cancellation or deadline termination arrives after resume rejection but before cold spawn
- **THEN** no replacement starts, and the termination outcome is preserved

#### Scenario: A provider stalls before its first turn
- **WHEN** the watchdog kills a stalled resume before accepted or any work checkpoint
- **THEN** the adapter does not replace it cold and the engine does not classify that kill as a provider failure to start

#### Scenario: The work chain and gate rules remain distinct
- **WHEN** the invocation ends in a classified provider refusal or in a mid-session failure
- **THEN** existing bounded candidate fallback applies only to eligible work-site failures to start, while gate immutability and the gate's existing park rules remain in force
