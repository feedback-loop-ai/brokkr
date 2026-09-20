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
or the release that resolution at qualification time selects in its place. The
2026-09-19 resolution exercised that rule and selected `@deepseek-ai/dsh`
0.1.5-rc.2 with registry integrity
`sha512-8Xc8hCQHcIWRmTCVU/xZdp6/qMsWMeAd2ObChKDEsfhUPJFXx6H0lgeb1DxUMD86HZrrVN+1bCvn1ppjZ/fOxw==`.
It SHALL be qualified together with a repository-owned adaptation of the `dsh-plugin-cli-session`
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

Executable resolution SHALL follow the running platform's native program lookup
rule for the same program name, cwd and child environment. On Unix a program
is a path if and only if its spelling contains `/`; backslash is an ordinary
filename byte. A drive-like spelling, extension or space SHALL NOT make a
bare Unix name a direct path. Windows SHALL use Windows' native rule, including
its native path and executable-name treatment, with no inferred Unix behavior.
A program name containing NUL SHALL refuse with NUL named before lookup or any
probe. This discipline SHALL apply to both DSH and Node, including explicit
overrides and the already-selected executable passed to the composite producer.

Bare-name Unix lookup SHALL distinguish absent PATH from a present PATH with
an empty entry. Absent PATH SHALL refuse with `PATH is absent` before any DSH
or Node version probe and SHALL NOT become cwd lookup. Explicit empty entries
SHALL retain their native cwd meaning at their position in the search. A
successful selection SHALL identify exactly the executable native
`std::process::Command::new(name)` runs under the same conditions; native
NotFound SHALL NOT become an executable selection. Ordinary provable positive
cases SHALL NOT be silently refused or substituted. The comparison SHALL use
real native children with distinct harmless sentinel identities, not metadata,
permissions, a manifest version or injected success as a lookup oracle.

Where the native outcome cannot be established without executing a candidate,
resolution SHALL take D10's pre-probe refusal with the obstructing candidate
and specific cause named, before probing that executable. A DSH selection
refusal SHALL prevent both DSH and Node probes; Node selection likewise SHALL
refuse before its own probe. This refusal SHALL be reported distinctly from native NotFound and SHALL NOT count as an equality
pass when a native control runs a later candidate. Metadata/access success,
a readable shebang interpreter, or a non-shebang native image alone SHALL NOT
prove loader success. Missing interpreters, interpreter chains with missing
loaders and native images with missing dynamic loaders SHALL NOT authorize
selecting the obstructing entry or silently guessing a later entry. A terminal
native lookup error SHALL preserve its cause and never authorize continuation.
Resolution SHALL introduce no candidate trial execution or resolver subprocess.
Explicit binary overrides SHALL retain precedence and no fallback to another
installation; a platform-native explicit path does not require PATH to select
that executable. A successful selection SHALL be reused for the version and
composite observations without retrying the original bare name.

One Rust function beside the DSH planner SHALL be the only producer of the
plugin component and canonical composite. For the plugin component it SHALL
read exactly `LICENSE`, `README.md`, `cordis.patch.yml`, `lib/index.js`,
`lib/startup.js` and `package.json` beneath the installed plugin directory,
sort those relative path bytes, serialize each as `<relative
path>\0<file SHA-256>\n`, and SHA-256 the concatenation. A missing file, a
symlink or an extra entry SHALL make the component unreadable and name the
drifted path. The sole exception is a direct, real, non-symlink
`node_modules/` directory beneath the plugin root, whose packages already
enter dependency identity; the producer SHALL neither hash its entries as
plugin files nor silently ignore any other entry, including an empty directory
or a deeper `node_modules/` under an unexpected directory. Unrepresentable
path names and special entries SHALL be reason-bearing unreadable outcomes;
lossy path conversion SHALL NOT merge distinct entries.

The same function SHALL serialize the composite as fixed `<component>\0<value>\n`
lines in this order: `core` with the core name, version and registry integrity;
`node`; one `dependency` for each normalized lock-metadata name/version/integrity
triple; `plugin`; `plugin-patch`; `profile-patch`; one `profile-bundle` for each
manifest bundle in declared order; `profile-patch-reload`; `home-patch` with the
home-level `cordis.patch.yml` SHA-256 or `absent`; and `extension` only when the
conditional extension is named among those bundles. Dependency triples SHALL
be deduplicated only when all three complete values are equal and SHALL then be
sorted by their serialized value bytes. Different versions or integrities SHALL
remain distinct. Only the core's exact own hidden-lock entry and the plugin's
local-tarball entry SHALL be excluded; the conditional extension's local entry
SHALL be excluded only when its installed bytes supply the `extension` line.
Same-named registry entries SHALL remain dependency inputs. No other component,
including `cordis.yml`, raw profile `package.json`, `pnpm-workspace.yaml`, any
`.env` layer, persisted state or per-seat overlay, SHALL enter the composite.
Any lock entry without registry integrity and any source field that is empty,
mistyped, contains NUL or LF, or contains whitespace including space, tab or CR
SHALL make the identity unreadable with the responsible component named.
One returned observation SHALL derive repeated uses of each identity-bearing
source from the same read, including the hidden lock and plugin patch; it
SHALL NOT contradict itself by reopening a source within that observation.
This requirement adds no atomic snapshot or continuous verification guarantee.

For npm metadata the producer SHALL read only the core root's
`node_modules/.package-lock.json`; it SHALL NOT fall back to a root lock. It
SHALL consume every complete `node_modules/<package>` group in a lock key from
left to right, treating the slash within `@scope/name` as part of one package,
then use only the final complete package spelling and the version and integrity
from that same entry. It SHALL ignore an optional `name` field and reject a
malformed intermediate or terminal group, and a missing, mistyped or invalid
version. The pnpm metadata SHALL come only from the headless profile's
`pnpm-lock.yaml`, parsed by the fail-closed line reader without a YAML crate.
The raw file SHALL be at most 8,388,608 bytes. The reader SHALL consume at most
8,388,609 bytes so a file that grows after a metadata check cannot bypass the
bound. Exactly 8,388,608 bytes SHALL reach grammar validation; any additional
byte SHALL refuse before dependency normalization with `pnpm lock exceeds
8388608-byte limit`. The total byte cap is the sole size bound; there is no
separate line, line-length or entry-count limit. Equivalent npm and pnpm triples
SHALL produce identical value bytes.

The admitted pnpm grammar SHALL preserve scalar types and field separation.
The lockfile header, outer mappings and inner resolution fields SHALL validate
their admitted separators before removing structural bytes. Missing separation
in `lockfileVersion:9.0` or `resolution:{integrity: sha512-X}` SHALL refuse.
Every plain flow field, including an ignored field such as `tarball`, SHALL
refuse colon-space syntax such as `tarball: x: y`; an ignored value SHALL NOT
hide a malformed document. An identity-bearing string field SHALL NOT accept
an unquoted null, boolean or number as a string, or repair missing colon
separation or unsupported flow punctuation into a value. Its refusal SHALL
name the pnpm component and syntax or type cause. Properly separated plain or
quoted version-9 headers and admitted quoted/plain field controls SHALL remain
readable, including supported colon-without-space URL values.

Only grammar-admitted structural ASCII separation outside scalar data may be
consumed as formatting. Identity-bearing scalar bytes SHALL NOT be trimmed or
normalized before validation, whether plain or quoted. Leading or trailing
U+00A0 in plain integrity SHALL reach the identity-whitespace refusal, just as
it does within quotes, with `integrity carries whitespace` named. Unicode
whitespace SHALL NOT become structural padding or be erased before quote
recognition; no whitespace-bearing identity can acquire the valid control's
readable composite. Existing empty, NUL and all-whitespace refusals remain.

Each decoded mapping key within `packages` SHALL occur at most once, including
excluded local records. A repeated key SHALL refuse with that decoded key named
before record exclusion or legitimate complete-triple deduplication; order,
identical values and different quoted spellings SHALL NOT cure the repetition.

The canonical executable SHALL be the selected core package's `bin.dsh`, whose
resolved relative path is `node_modules/@deepseek-ai/dsh/lib/bin.js` and whose
first line is exactly `#!/usr/bin/env node`. The selected core's own hidden-lock
entry SHALL supply its version and match the core package's version. `node`
SHALL identify the runtime native child lookup would execute on the child
PATH, subject to the same named pre-probe refusal for unprovable selection,
and be observed through `node --version` as one non-empty record with at most
its single output terminator; trimming SHALL NOT repair whitespace inside the version value. Only
`<home>/profiles/headless/package.json` SHALL supply the non-empty string-array
`bundles` and `patchReload` (`live` or `startup`); no other profile is searched.
Bundle resolution SHALL preserve the provider loader's order. The producer
SHALL canonicalize the complete profile directory once for containment while
retaining the original profile path as the lookup anchor. It SHALL judge each
first-hit canonical bundle directory against the canonical core root or
canonical profile boundary, with the plugin and conditional extension inside
the profile. A boundary or candidate that cannot be canonicalized, or a first
hit outside those roots, SHALL be unreadable. It SHALL NOT compare raw paths,
use string prefixes, fall back after canonicalization failure or search past an
outside first hit. A symlinked home that resolves to the same contained profile
SHALL produce the same identity.

The existing `brokkr doctor` DSH line SHALL resolve the adapter's executable
and home seams once and use that same resolution for both the DSH version probe
and the sole composite producer. The executable precedence SHALL be
`BROKKR_DSH_BIN`, then `FORGE_DSH_BIN`, then `dsh` on PATH, with the adapter's
existing home resolution. It SHALL NOT pair a PATH version with an
override-selected composite or silently retry another installation when the
selected executable fails. It SHALL report the canonical digest and plugin
component, or the named unreadable component, and state whether the canonical
digest equals, differs from or has no declared `wrapper_digest`. The result is
informational while no `supported` shape declares a digest. Once a supported
shape declares one, difference or unreadability SHALL be a warning. This probe
SHALL read no credential or settings file and SHALL spawn only the existing
`dsh` and `node` version probes. An executable-selection refusal SHALL be
reported by reason before probing any candidate, including for an unmeasured
shape with no declared digest. Home availability SHALL NOT establish executable
selection. Combined seam resolution succeeds only when both executable
selection and home resolution succeed. An independently safe selected executable
may still report its version beside a home/composite refusal; failed selection
SHALL provide no probe target even when a home exists. Every rendered
selected-binary value and unreadable reason SHALL escape terminal control
characters, including on the binary-not-found path, without injecting a new
line or terminal command. If the required plugin manifest is absent and no
candidate resolves under the existing lookup order, the refusal SHALL identify
both the plugin bundle and `package.json`; a later legitimate candidate remains
eligible when the earlier manifest is truly absent.

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

A previously supported, currently shipping work-site rejoin SHALL remain live
across a difference between its historical measurement version and the
applicable installed version. The operator's 2026-09-15 ruling, “keep decision
0030's rejoin live, do not regress codex,” amends this change's earlier draft
requirement to disable historically supported Codex during remeasurement.
Codex `work-site` is preservation of a rejoin main already performs under
accepted decision 0030. Its declaration SHALL describe that disposition and
ruling and distinguish accepted 0030's historical 0.148.0 measurement, the
previous 0.153.4 applicability and the exercised 0.154.0 binary in the
September 16 controller proof. The current measured identity and applicability
SHALL name 0.154.0, with a dated 0.153.4→0.154.0 reconciliation citing
`.forge/tasks/controller-codex-proof-2026-09-16-live.json`. Its interface,
restriction, exact-root and current-accounting evidence SHALL identify each
observation's version and scope. Earlier limitations SHALL remain dated
history, with the new entry explicitly resolving the now-measured resume axes.
The reason SHALL state that `codex exec resume` offers no sandbox flag and
therefore the adapter re-imposes the class through `-c sandbox_mode=<class>`.
Neither this reconciliation nor the historical evidence SHALL be described as
complete qualification of unobserved 0.154.0 interface options or new shapes.
The preserved shape SHALL name BOTH work-seat coordinates main rejoins: the
engine-composed `harness` argv (`--sandbox workspace-write`, explicitly no boxed hands)
AND the author-written INLINE argv (`boundary: not applicable`, the command's
own `--sandbox` class). The compiler SHALL carry each inline built-in model
driver's adapter resume assessment into the engine so the driver judges an
inline site exactly as an agent-resolved one; a custom no-hands driver no
declaration names still carries none and cannot rejoin. The resume SHALL still
re-impose the inline argv's sandbox class, and a missing or unsupported class
SHALL refuse cold. Wrapping SHALL preserve the executing site's assessment,
adapter declaration pins, hands, agent record and driver evidence as one
indivisible owned family, including panel members. Evidence SHALL neither
remain at an old coordinate nor answer for a sibling. Unwrapped forms SHALL
retain their existing supported or refused behavior.
Complete installed-version qualification remains a delivery obligation. The
September 16 live proof and September 17 interface record SHALL be consumed
without repetition. Task 10.1's three deferred interface observations and
completed 10.5 SHALL be adopted from `85cf6d55`, not re-authored. The supplied
stdin grammar, strict-config effort-key control and enumerated option surface
SHALL inform the matching adapter assertions. Parser acceptance SHALL NOT
qualify confinement or widen the allow-list. Preserving shipping behavior or
recording dependency completion SHALL NOT complete 11.1 without its whole
acceptance, compiling removal proofs and local validation. Current evidence
claims SHALL distinguish that discharged interface evidence from remaining
adapter proof; historical limitations SHALL retain their dates and bytes.

An unmeasured rejoin main does not perform SHALL remain disabled during
preparation. A version change SHALL invalidate that new shape's prior-version
qualification until its interface, current restriction enforcement, exact-root
confirmation and current-only accounting are established for the applicable
version. Under this preservation ruling, the Claude `boxed-workspace`,
DSH `headless-work` and LaneTally
`wrapper-work-site` declarations SHALL explicitly identify themselves as this
case and retain their unmeasured status, identities, evidence and scope. Claude's
partial probes do not establish complete restriction enforcement; DSH's live
isolated pair qualifies core/plugin continuity, current restriction precedence
and attributable per-message accounting but still lacks completed digest
recording, matching adapter/shim assertions, planner acceptance and end-to-end
admission; LaneTally lacks independent wrapper qualification. This ruling
enables none of these three rejoins.

Preservation SHALL NOT bypass identity, boundary, hands or accounting checks.
The observed executable identity SHALL still match declared applicability and,
when recorded, the originating root's identity. Missing or unreadable required
identity and evidence, a mismatch in those identities, inapplicable boundary or
hands, and absent or inapplicable current-accounting evidence SHALL still refuse
rejoin. The engine SHALL supply affirmative boundary and hands facts for the
actual executing site, including explicit `not applicable` / `none` only
when that owner's declaration has been resolved and establishes no Brokkr
hands. An unregistered owner or unresolved hands state SHALL remain unknown;
registration alone SHALL NOT establish no-hands state. Composition SHALL
replace or clear earlier confinement markers so an enclosing or sibling input
cannot supply this invocation's evidence. Independently of compilation or
relocation, the adapter SHALL treat a missing, null or unreadable boundary or
hands marker as unknown confinement, never as those affirmative values. With otherwise
supported evidence, unknown or mismatched confinement SHALL decline as
`restrictions-unavailable`; absent or unsupported assessment SHALL decline as
`unsupported-resume`. Neither case SHALL enable a work-site assessment.
Affirmative markers SHALL be necessary, not sufficient: both SHALL describe
this executing site's actual confinement and match the supported assessment.
An affirmative namespace/boxed pair SHALL NOT enable a Codex shape that does
not support it. A live supported control SHALL use its own correctly composed
facts, never replacement no-hands markers for a hands-bearing invocation.
Same-site/instance ownership, local origin, exact-root confirmation,
current restriction re-imposition and fresh gates remain binding. A historical
measurement-version difference alone SHALL NOT cause a preserved shipping shape
to be disabled. A shape whose applicable version still matches needs no repeated
live experiment solely because another attempt begins.

Unavailable enforcement evidence for a new, never-supported rejoin SHALL keep
that shape disabled and the requirement incomplete. Missing new measurements
alone SHALL NOT withdraw the preserved shipping Codex shape; its required
existing evidence and invocation checks still apply. Common plumbing or
provider-specific code
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
pin and distinguish preservation from new enablement, with assessed and
applicable versions labelled separately. Its `limitations` list SHALL be the
append-only dated ledger. Existing entries SHALL keep their bytes and order. A reversal SHALL be appended as a
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
- **GIVEN** 10.7's qualification of the adapted pair on resolved core 0.1.5-rc.2 passes and the delivered Rust canonicalization computes the qualified composite's digest over the task-owned home
- **WHEN** 11.3 enables the DSH `headless-work` shape
- **THEN** the same declaration edit that sets `supported` writes that digest as `identity.wrapper_digest` beside `version` and `applies_to` `0.1.5-rc.2`, in `adapters/dsh.json` and its packaged or scaffolded equivalents
- **AND** a later DSH seat compares its probed version with `applies_to` and its recomputed composite with that digest, and on an offer also compares both with the values the originating root recorded; a confirmed root records the observed digest in `root_session.wrapper_digest`
- **AND** the digest does not change when the same composite is deployed in another home or when the per-seat overlay differs

#### Scenario: The canonical rc.2 fixture reproduces the measured locator set end to end
- **GIVEN** the retained canonical installation uses core `@deepseek-ai/dsh` 0.1.5-rc.2 with integrity `sha512-8Xc8hCQHcIWRmTCVU/xZdp6/qMsWMeAd2ObChKDEsfhUPJFXx6H0lgeb1DxUMD86HZrrVN+1bCvn1ppjZ/fOxw==`, executable `node_modules/@deepseek-ai/dsh/lib/bin.js` beginning `#!/usr/bin/env node`, Node `v22.23.2`, and the six installed plugin files whose bytes reproduce the preinstall record's SHA-256 values
- **AND** its profile declares bundles `@deepseek-ai/dsh-base`, `@deepseek-ai/dsh-headless`, `dsh-plugin-cli-session` in that order with `patchReload: startup`, uses the measured 217-byte profile patch and has no home-level patch
- **AND** the first two bundles resolve under the core root at `node_modules/@deepseek-ai/dsh-base` and `node_modules/@deepseek-ai/dsh-headless`, each at 0.1.5-rc.2, while the plugin resolves under the profile at `node_modules/dsh-plugin-cli-session`, at 0.2.0, as a real directory with no nested `node_modules/`
- **AND** the fixture retains the canonical reinstall's byte-exact 311,184-byte hidden npm lock with SHA-256 `b84bac2d866224a997be29811dc71bde6013dbc6e2adf8c1e77523e6f05a3847`, the 1,982-byte pnpm lock with SHA-256 `4708752f0463211bf25d470fc26befa49748707b9c12fae7b4f2544e02b21055`, and the profile-patch bytes that reproduce SHA-256 `ef189a8c27db6d63930aa3046a3040482e952eafcb7487c644d508e8d461f027`; the earlier 277-byte pnpm lock with SHA-256 `54265d3b5db4b7368bccd8ddf26c5a1ca68f308016d0cd0f0b21660e89d1c8e0` describes the superseded `link:` install and is not fixture ground truth
- **AND** authoring copies the full measured lock and patch bytes and measured expectations into literal test-source constants, preserving line endings and final newlines, then tests materialize the fixed locators under temporary roots using the unchanged committed plugin files
- **WHEN** the sole Rust producer reads those files and directories through D6's fixed locators
- **THEN** it reproduces the retained installation's complete normalized dependency values, first-hit bundle resolutions at both anchors, plugin and patch components and canonical composite from that fixture
- **AND** tests neither read `.forge/` at runtime nor include it as a build input; they do not regenerate the measured bytes, replace the full lock with excerpts, or substitute a derived triple list for the producer's lock input
- **AND** the expected component and composite are recorded only from that producer; no test helper, fixture generator, prose calculation or synthetic value is a second producer
- **AND** a separate layout case admits a direct, real, non-symlink plugin `node_modules/` directory as the sole extra entry, matching the earlier measured working shape without claiming it exists in the corrected canonical tree

#### Scenario: Hashes and dimensions do not substitute for measured fixture inputs
- **GIVEN** a handoff retains only lock hashes and dimensions and the profile-patch hash, but omits lock bodies, complete normalized dependency triples, resolved bundle targets or profile-patch bytes
- **WHEN** the task 8.8(a)–(c) implementation handoff is evaluated
- **THEN** the measured fixture requirement is unsatisfied because the sole producer cannot reproduce the retained dependency set, bundle resolution, profile-patch component or canonical composite from those observations
- **AND** synthetic lock entries, bundle directories or profile-patch bytes may prove isolated grammar and rejection behavior but SHALL NOT be asserted as the canonical rc.2 fixture or as its composite ground truth
- **AND** the controller SHALL retain the canonical locator bytes and resolution layout, or an equivalent complete ground-truth fixture, before design, tasks or implementation resumes; no provider or retained-home remeasurement is delegated to a boxed seat
- **AND** task 8.8 remains unchecked, including after its later digest implementation, until part (d) and the owned 8.10 cases also complete

#### Scenario: Retained raw bytes close the evidence parks without runtime evidence access
- **GIVEN** the canonical reinstall's raw-byte/layout addendum retains the complete pnpm lock, profile patch, four dependency triples and two-anchor bundle layout, and the retained-hidden-lock addendum points to the complete 311,184-byte npm lock with its recorded SHA-256
- **WHEN** authoring verifies those input bytes and embeds them and their measured expectations as literal test-source constants
- **THEN** the missing-length and missing-input blockers are resolved without a provider, registry or retained-home remeasurement, and the measured rc.2 fixture requirement remains unchanged
- **AND** a derived dependency list is only an independent expectation for comparison; the sole producer still reads the complete hidden lock through its declared locator
- **AND** the earlier hash-only fixture narrowing is superseded in the dependent design and task breakdown before implementation; the existing producer is the only source of either digest, and the declaration remains disabled without a `wrapper_digest`
- **AND** this closes evidence availability only: Rust assertions, compiling removal proofs, local gates, external exact coverage, 10.7's retained-home recording, part (d), 8.10 and the 8.8 checkbox remain pending

#### Scenario: The measured dependency set distinguishes complete triples from package names
- **GIVEN** the literal canonical hidden lock has 522 entries, all carrying integrity, including exact key `node_modules/@deepseek-ai/dsh` at 0.1.5-rc.2 with the selected core's integrity
- **AND** the literal pnpm lock contains the four registry triples below and the excluded local entry `dsh-plugin-cli-session@file:../../../pack/dsh-plugin-cli-session-0.2.0.tgz`
- **WHEN** the sole producer normalizes both locks, excludes only the exact core and local-tarball records, deduplicates equal complete triples and sorts their value bytes
- **THEN** the npm input contributes 521 post-exclusion entries, 501 unique complete triples and 489 distinct names, preserving all twelve names whose versions or integrities differ
- **AND** pnpm contributes exactly the four complete triples below, three equal to npm triples, yielding exactly 502 combined dependency values in bytewise order
- **AND** full ordered-value equality with the embedded measured expectations is asserted alongside counts; neither 521 undeduplicated values nor 489 name-deduplicated values satisfies the claim, and no core or local-tarball record leaks into the dependency lines

| pnpm package | Version | Registry integrity |
|---|---|---|
| `@deepseek-ai/cosmokit` | `1.8.3` | `sha512-qBo+ronVM6Eu2WNVJXi8JcMiqZ19T9BRIpV+5qJUFPXjGH/Z0QKcQMC/IZJ7L394YTOtJgcovbk9qP0w2GsBXQ==` |
| `@deepseek-ai/schemastery` | `3.18.1` | `sha512-Qn0FCSwCQnpnj6SB31I6i2sIKgKWnkbJM8O0EU91Gv2UsYVvtZTl6IA0sCwk2e2MZf5S8w5hpq9QkeVvK9qwxg==` |
| `@standard-schema/spec` | `1.1.0` | `sha512-l2aFy5jALhniG5HgqrD6jXLi/rUWrKvqN/qJx6yoJsgKhblVd+iqqU4RCXavm/jPityDo5TCvKMnpjKnOriy0w==` |
| `commander` | `15.0.0` | `sha512-z67u4ZhzCL/Tydu1lJARtEZYWbWaN7oYLHbsuzocr6y4N6WZAagG3RQ4FW61V1/0+jImpj293XfrcYnd1qxtPg==` |

#### Scenario: The rc.2 qualification already proves restriction precedence after restoration
- **GIVEN** the live rc.2 cold/warm qualification restored the cold session's private nonce and the complete plugin CLI exposes no model, effort, sandbox, tool or persistence-root override
- **AND** a current model patch on resume selected `deepseek-v4-pro`, removing that patch restored the profile's `deepseek-flash`, and a deliberately invalid current model reached the provider's model-name refusal
- **AND** resumed `--workdir` was refused before launch without creating another root, while an unknown profile selector was refused without falling back
- **WHEN** the DSH evidence account is evaluated for task 8.8's digest slice and task 10.7's remaining acceptance
- **THEN** current restriction precedence is measured for the qualified rc.2 pair and is consumed without another provider probe
- **AND** task 10.7 still owes the post-8.8 doctor recording and matching Brokkr adapter/shim assertions; this evidence neither completes 10.7 nor authorizes planner behavior, 8.10 cases or the 8.8 checkbox

#### Scenario: A stale proposed decision blocks the rc.2 digest implementation
- **GIVEN** AS1, D6, task 8.8 and the controller records select `@deepseek-ai/dsh` 0.1.5-rc.2 with its recorded registry integrity and measured continuity, restriction precedence and current-sequence accounting
- **AND** proposed decision 0056 ruling 5 and its consequence still select rc.1 and describe those measured rc.2 facts as outstanding
- **WHEN** the task 8.8(a)–(c) implementation handoff is evaluated
- **THEN** proposed 0056 does not yet agree with the evidence and SHALL NOT be treated as the governing rc.2 decision text
- **AND** the decision-owning upstream office SHALL amend ruling 5 and its consequence to record rc.2 and the measured facts, preserve rc.1 as dated history, retain `Status: proposed`, and reconcile D10 before implementation proceeds
- **AND** the amendment SHALL keep digest recording, matching adapter/shim assertions, planner admission and end-to-end enablement pending and SHALL require no provider remeasurement

#### Scenario: The amended proposed decision reopens only the digest slice
- **GIVEN** proposed decision 0056 carries its dated 2026-09-19 note selecting `@deepseek-ai/dsh@0.1.5-rc.2` with the recorded registry integrity, preserving rc.1 as history and consuming the measured continuity, restriction-precedence and current-sequence-accounting facts
- **AND** D10 withdraws its earlier claim that this amendment already existed and agrees that the declared composite remains unproved, the route remains `unmeasured` and disabled, no `wrapper_digest` is set, and decision 0056 remains `proposed`
- **WHEN** the returned task 8.8(a)–(c) implementation handoff is evaluated on that amended head
- **THEN** the stale-decision blocker is resolved and the loader, sole digest producer and doctor work SHALL proceed without repeating the registry, provider or retained-home measurements
- **AND** planner behavior, 8.10's remaining cases, the retained-home doctor recording, the declaration pin, end-to-end enablement and the 8.8 checkbox SHALL remain pending

#### Scenario: The pnpm reader has one exact raw-byte bound
- **GIVEN** otherwise valid lockfile-9.0 bytes padded with grammar-accepted blank lines to exactly 8,388,608 bytes, and the same bytes followed by one additional byte
- **WHEN** the sole producer reads each `pnpm-lock.yaml`
- **THEN** the exact-boundary file reaches dependency parsing, while the 8,388,609-byte file is refused before normalization with `pnpm lock exceeds 8388608-byte limit`
- **AND** the read itself consumes no more than 8,388,609 bytes, a metadata race cannot admit a larger file, and no separate line, line-length or entry-count limit changes the outcome
- **AND** the controller's byte-exact measurements show that the corrected 1,982-byte rc.2 pnpm lock and 311,184-byte hidden npm lock fit the bound, closing the earlier live-size evidence return without changing the cap

#### Scenario: The loader admits a measured composite pin only
- **GIVEN** otherwise valid measured and unknown resume identities
- **WHEN** their declarations are loaded and the selected assessment is carried into a private start context
- **THEN** a measured identity may omit `wrapper_digest` or carry exactly 64 lowercase hexadecimal characters, and the carried assessment preserves that exact optional member
- **AND** uppercase, short, long or non-hexadecimal values are refused at load with the `wrapper_digest` grammar named, while the unknown identity refuses `wrapper_digest` as an unknown key and still admits `unknown` alone

#### Scenario: Lock whitespace and malformed package groups are unreadable
- **GIVEN** otherwise valid npm or pnpm lock metadata whose package, version or integrity field is separately changed to contain a space, tab, carriage return, NUL or line feed, or whose nested npm key has a malformed intermediate package group before a valid terminal group
- **WHEN** the composite producer normalizes dependencies
- **THEN** every variant is unreadable with the dependency field or malformed path named, before any digest can be compared
- **AND** an assertion of `is_err()` alone does not prove the refusal; the test asserts the exact reason, while the complete 8.10 rejection-vector ledger remains pending its own task

#### Scenario: Doctor reports every declared-digest disposition through DSH seams
- **GIVEN** the DSH binary is available and the adapter's seam resolution yields either a readable composite or a named unreadable component
- **WHEN** `brokkr doctor` renders the existing `dsh` line for an unmeasured shape with no digest, a supported shape with an equal digest, a supported shape with a different digest, and a supported shape whose composite is unreadable
- **THEN** the line respectively reports `no declared wrapper_digest`, a match, a difference naming the declaration, or `composite unreadable` naming the component
- **AND** the first two are informational, the latter two warn only for the supported declared shape, and no credential or settings file is read
- **AND** only the DSH and Node version probes are spawned and the guide's doctor sample uses the same wording

#### Scenario: Doctor version and composite follow the same selected DSH installation
- **GIVEN** primary-override, legacy-override and PATH installations have distinguishable sentinel versions and distinct readable composite inputs
- **WHEN** the primary and legacy overrides are both set, then only the legacy override is set, then neither is set
- **THEN** the DSH report's version and composite both describe respectively the primary, legacy and PATH installation, using the same resolved home and the child PATH's Node version
- **AND** the test asserts both the reported version and the corresponding producer-derived composite on each real provider-line path; changing just one half back to the bare binary makes that paired assertion fail
- **AND** if the selected executable's version probe fails while a PATH binary remains available, doctor reports the selected failure without silently reporting the other installation
- **AND** no credential/settings read or subprocess beyond the selected DSH and Node version probes is needed for this diagnostic, and the guide sample follows the same wording

#### Scenario: Absent PATH refuses before doctor can execute a cwd sentinel
- **GIVEN** a real executable `dsh` in a temporary cwd prints `SECURITY_CWD_SENTINEL_9f3`, no DSH binary override is set, the child environment has no PATH, and the shipped DSH declaration is unmeasured with no `wrapper_digest`
- **WHEN** the built doctor and a native Rust `Command::new("dsh").arg("--version")` control are invoked with the same cwd and environment
- **THEN** doctor reports a DSH selection refusal containing `PATH is absent`, its output does not contain the sentinel, and the controlled native child returns `NotFound`
- **AND** with PATH explicitly `/usr/bin:/bin` and no DSH installed there the sentinel remains unexecuted; removing cwd `dsh` while keeping PATH absent still produces the named absent-PATH refusal
- **AND** with the sentinel executable present, the same doctor regression test fails specifically at its no-sentinel assertion on the adopted pre-fix execution path; after exact restoration of the repair it passes both that assertion and the `PATH is absent` reason assertion
- **AND** removing only the fixture or supplying explicit PATH is a separate control, not a substitute for that production regression proof; failure only at a newly required reason assertion does not prove sentinel execution
- **AND** environment changes are confined to child processes and no installed provider, global home or frozen fixture supplies this test

#### Scenario: Unix backslashes never confer direct-path authority
- **GIVEN** a temporary Unix cwd contains an executable literally named `C:\Tools\dsh.exe` printing `SECURITY_BACKSLASH_CWD_SENTINEL_9f3`, the primary DSH override spells that exact name, and the shipped declaration is unmeasured
- **WHEN** the built doctor and native `Command::new(name)` control use that cwd first with PATH absent and then with PATH set to directories containing no such file
- **THEN** native lookup returns NotFound in both layouts, doctor refuses before any probe, and neither doctor output nor an execution marker records the cwd sentinel
- **AND** the absent-PATH refusal names `PATH is absent`; the populated-PATH refusal identifies the unsuccessful lookup without converting the backslashes to separators
- **AND** placing a distinct file with that literal name in a PATH directory selects that native PATH identity, while a `/`-containing explicit path to the cwd file exercises the separate direct-path control
- **AND** removing the cwd fixture is a separate negative control; restoring backslash-as-separator classification must make the present-fixture no-execution assertion and differential matrix fail, then exact restoration passes both

#### Scenario: Program lookup is proved against the platform by a complete differential matrix
- **GIVEN** isolated layouts with distinct harmless sentinel identities for every candidate, and the following program-name and layout axes
- **WHEN** every name is crossed with every layout and a real native `Command::new(name)` child is compared with resolution under identical cwd and environment
- **THEN** each ordinary successful cell identifies exactly the file native lookup ran, and every native-NotFound cell refuses without a probe target; success booleans or matching generic version strings cannot prove selected identity
- **AND** unprovable obstruction cells record the actual native result separately and assert the named D10 refusal with zero resolver/doctor probes; they are not reported as native NotFound or as equality passes
- **AND** a NUL-bearing name is refused with NUL named before any filesystem lookup or probe, and the native control returns invalid input without executing a sentinel
- **AND** other terminal native errors retain a cause-bearing refusal and never authorize a later candidate
- **AND** the same table executes on Windows with native executable sentinels and native fixture paths; names or layouts the platform cannot admit retain their observed native refusal rather than being relabelled, omitted or claimed from Unix evidence
- **AND** slash-containing explicit paths still occupy every matrix cell and preserve their native direct-path behavior regardless of which PATH layout surrounds them; ordinary successful controls prevent blanket refusal from satisfying the table

| Axis | Required members |
|---|---|
| Program name | `dsh`; `./dsh`; `../dsh`; an absolute temporary path corresponding to `/abs/dsh`; literal `C:\Tools\dsh.exe`; `dsh.exe`; a name containing a space; a name containing NUL |
| Layout / environment | file in cwd with PATH pointing elsewhere; file in a PATH directory; PATH absent with cwd file present; PATH containing an empty entry; PATH A:B with obstructed A and runnable B |
| Empty-entry variants | present-empty PATH, leading empty entry, interior empty entry and trailing empty entry, with distinguishable competing candidates |
| Obstruction variants | missing shebang interpreter; executable interpreter whose own loader is missing; native image with missing dynamic loader; retain the terminal self-symlink and ordinary non-executable continuation controls |
| Platform | native Unix execution and native Windows execution; platform-specific path syntax and loader fixtures follow that platform's rules |

#### Scenario: Explicit empty PATH entries and explicit overrides retain their meaning
- **GIVEN** controlled executable names in cwd and a later PATH directory, plus distinct primary and legacy explicit override paths
- **WHEN** PATH is present but empty, or contains an explicit empty entry among its directories
- **THEN** cwd lookup has its native meaning at that entry's position and is not classified as absent PATH; doctor either describes the native-selected installation or gives a named safe refusal
- **AND** primary then legacy override precedence remains unchanged, an absolute explicit override can be selected with PATH absent, and a failed explicit override never falls back to a PATH decoy
- **AND** version and composite never describe different installations, including when a safe version probe succeeds but home resolution fails

#### Scenario: Bare-name lookup preserves native continuation and terminal errors
- **GIVEN** PATH is A:B, neither binary override is set, and B/dsh is a working executable with a distinguishable sentinel identity
- **WHEN** A/dsh separately has a nonexistent shebang interpreter, an executable interpreter whose own loader is missing, or a missing native dynamic loader
- **THEN** real native controls record the B identity they execute, while doctor takes D10's named pre-probe refusal identifying A and the unproved interpreter/loader cause, without executing either candidate
- **AND** the refusal is not a generic selected-binary NotFound after probing A, and is not reported as equality with the native B result
- **AND** replacing A with a self-referential symlink makes the Unix native control fail with ELOOP and resolution refuse by that cause without probing B
- **AND** an ordinary non-executable A followed by runnable B remains a positive continuation control, and supported explicit native-image/interpreter cases identify exactly what their native controls run
- **AND** restoring unconditional native-image admission or one-level interpreter metadata admission fails the relevant named-cause/no-probe assertion; restoring error-erasing continuation fails the terminal-error assertion

#### Scenario: Missing pnpm field separation and unsupported flow syntax refuse by reason
- **GIVEN** an otherwise readable installed composite separately changes the lock header to `lockfileVersion:9.0`, the package child to `resolution:{integrity: sha512-X}`, or the resolution to `resolution: {integrity: sha512-X, tarball: x: y}`
- **WHEN** the sole producer reads each lock through the profile locator
- **THEN** each input refuses before normalization, naming the pnpm component and respectively the header separation, outer resolution separation or tarball unsupported-flow cause; none produces the valid control's readable composite
- **AND** the original `integrity:sha512-X` and plain `integrity: sha512-X[one]` cases continue to refuse by their specific missing-separation or unsupported-flow cause
- **AND** properly separated plain and quoted version-9 headers, `resolution: {integrity: sha512-X}`, supported quoted field controls and supported colon-without-space URL values remain readable
- **AND** separately removing header separation, outer separation or plain colon-space syntax protection fails its producer-facing reason-bearing regression, with each exact restoration passing; an inner-integrity-only check cannot discharge the outer or ignored-field cases

#### Scenario: Plain and quoted integrity retain their Unicode whitespace bytes
- **GIVEN** the same readable installed composite separately adds U+00A0 before or after plain `sha512-X`, or before or after that value inside each supported quote form
- **WHEN** the sole producer reads each otherwise valid pnpm lock
- **THEN** every variant refuses with `pnpm lock`, the package and `integrity carries whitespace` named, and none yields the unpadded control's composite
- **AND** grammar-admitted ASCII spacing outside the scalar and unpadded plain/quoted controls remain readable; U+00A0 outside a quote is not converted into admitted padding
- **AND** restoring Unicode trimming before scalar validation makes the plain-edge regression fail, while exact restoration of identity-byte preservation passes both plain and quoted cases

#### Scenario: Executable selection and home availability are independent requirements
- **GIVEN** isolated deterministic combinations of a safely selectable executable or failed selection, and an available home or missing home
- **WHEN** the adapter resolves its seams and doctor reports that observation
- **THEN** combined seam resolution succeeds only with both a selected executable and a home; a failed selection with a home still refuses by selection cause and never probes
- **AND** a safely selected executable with no home can report its version beside the named home/composite refusal, preserving the same selected identity
- **AND** under child HOME=/tmp, PATH=/usr/bin:/bin with no DSH there and DSH_HOME, BROKKR_DSH_BIN and FORGE_DSH_BIN unset, the test asserts the selection refusal instead of equating home presence with success
- **AND** assertions are isolated from the test runner's installed providers and environment; restoring the home-only success assumption fails the commissioned control without weakening production selection

#### Scenario: Pnpm identity strings preserve the distinction from typed scalars
- **GIVEN** an otherwise readable composite with `resolution.integrity` separately set to plain `null`, `~`, `true` or `42`, in each admitted block or flow representation
- **WHEN** the sole producer reads the lock
- **THEN** each plain typed scalar refuses with the pnpm component, integrity field and non-string or unsupported-scalar cause named
- **AND** the corresponding supported quoted string is a readable string control, including `'null'`; plain null cannot share its readable identity
- **AND** admitted lockfile-version syntax, including its existing plain and quoted version forms, still passes, and restoring string coercion fails the reason-bearing controls

#### Scenario: Duplicate decoded pnpm package keys refuse before triple normalization
- **GIVEN** a valid pnpm package record followed by an identical record, a conflicting-integrity record in either order, or an equivalent quoted/unquoted spelling of the same decoded package key
- **WHEN** the sole producer reads each installed lock, including a repeated local-tarball key otherwise excluded from dependency identity
- **THEN** every repetition refuses with a repeated-package-key reason naming the decoded key, before exclusions or complete-triple deduplication can hide it
- **AND** distinct valid records across the two locks still deduplicate equal complete triples and retain triples with different versions or integrities
- **AND** removing duplicate-key rejection makes the repeated-record assertion fail; reversed conflicting records producing an equal digest is evidence of the defect, not a positive deduplication control

#### Scenario: Plugin expectations are recorded from the sole producer
- **GIVEN** an exact plugin input set with its source bytes identified and a fixed expected component recorded from the sole producer at an identified revision
- **WHEN** the plugin component and canonical composite assertions run for measured or synthetic layouts
- **THEN** expectations are literals with that provenance, and no test helper, fixture generator or prose calculation reconstructs either serialization or hashes its concatenation as a competing oracle
- **AND** independent per-file input hashes remain permitted, while changed file bytes, removed required files and altered production path ordering still fail their named behavioral or compiling-removal assertions
- **AND** restoration of the independent test serializer fails an explicit source-conformance check; comparing two calls to the producer alone does not establish the expected byte format

#### Scenario: A nonexistent override cannot inject terminal control bytes through doctor
- **GIVEN** a nonexistent explicit DSH binary override contains a newline followed by ANSI clear-screen bytes
- **WHEN** the built doctor renders its binary-not-found diagnostic
- **THEN** the selected value is escaped using the established terminal-safe convention and remains recognizable, while its raw newline and ANSI sequence cannot create an injected line or terminal command in stdout
- **AND** the test asserts the escaped value and absence of the raw injected sequence; removing safe rendering fails that assertion

#### Scenario: Removing only the plugin manifest names the drifted file
- **GIVEN** a complete temporary installed plugin layout with only `package.json` removed and no later resolving copy of that bundle
- **WHEN** the sole producer reads the installed composite and doctor reports its refusal
- **THEN** the unreadable reason names both `dsh-plugin-cli-session` and `package.json`, rather than only saying the bundle does not resolve
- **AND** a positive control with a truly absent earlier manifest and a valid later hit retains the established lookup order; an unreadable or outside first hit is still a refusal and never falls through
- **AND** removing the missing-filename context makes the producer-facing reason assertion fail while restoring it passes; a private file-set helper alone does not prove the installed lookup path

#### Scenario: Digest acceptance is proved by removal without completing the planner
- **GIVEN** tests for the loader grammar and exact selected-assessment carriage into the private start context, six-file membership and bytes, complete dependency parsing and whitespace, fixed locators and exclusions, canonical containment, the measured rc.2 fixture, every doctor disposition, the paired version/composite seam assertions, the first hold's seven obligations and all five second-hold findings covered above
- **WHEN** each responsible production check or emitted element is removed in a compiling mutation and then exactly restored
- **THEN** the named test fails at the exact claimed assertion, including the drifted file, component or refusal reason, and its restored rerun passes
- **AND** a compilation failure, unrelated earlier failure, bare `is_err()`, count without value equality or composite compared only with itself is not removal evidence
- **AND** delivery additionally requires every crate suite green and a fresh exact coverage run with actual nonzero covered/total line, branch and function equality; retained instrumentation and the removed roster commit's historical failures supply no current exemption
- **AND** those proofs deliver task 8.8(a)–(c) only; `dsh_launch`/`dsh_launch_with`, planner production behavior, 8.10's remaining cases and the 8.8 checkbox remain pending

#### Scenario: npm nested and scoped keys produce reproducible dependency values
- **GIVEN** a hidden npm lock with no `name` fields, including `node_modules/debug`, nested `node_modules/parent/node_modules/debug`, scoped-parent/unscoped-child entries and `node_modules/a/node_modules/@parent/b/node_modules/@scope/child` with version and integrity distinct from shallower `@scope/child` entries
- **AND** the full measured rc.2 lock retains `node_modules/@aws-sdk/credential-provider-http/node_modules/@smithy/node-http-handler`, `node_modules/@aws-sdk/credential-provider-sso/node_modules/@aws-sdk/token-providers` and `node_modules/@anthropic-ai/sdk`
- **WHEN** qualification and runtime compute the same composite through the delivered Rust canonicalization
- **THEN** those measured keys produce respectively `@smithy/node-http-handler`, `@aws-sdk/token-providers` and `@anthropic-ai/sdk` with the exact version and integrity from each entry
- **AND** the key is parsed left to right through every package group, preserving the slash inside each scoped package; each dependency value uses only the terminal package name and that entry's exact `version` and `integrity`, joined by single ASCII spaces, with no parent path or optional `name` field supplying a value
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
- **GIVEN** main already rejoins this Codex work-site shape under accepted decision 0030, its historical measurement identifies codex-cli 0.148.0, its previous applicability was 0.153.4, and the September 16 controller proof exercises installed 0.154.0
- **WHEN** the declaration adopts that proof and an eligible same-instance work-seat retry offers its owned thread with matching current identity, boundary, hands and current-accounting evidence
- **THEN** the current measured identity and applicability both name 0.154.0 and the adapter rejoins that exact thread with current restrictions re-expressed, reporting `launch: resumed` only on exact provider confirmation before work
- **AND** its declaration retains the 2026-09-15 preservation ruling, the historical 0.148.0 evidence, and a dated 0.153.4→0.154.0 reconciliation referencing the new proof; `supported`, class `work`, boundaries `harness` and `not applicable`, and `hands: none` remain its scope
- **AND** an executable or originating root whose recorded identity differs from 0.154.0 still refuses as `unverified-harness`; retaining historical evidence does not admit historical identities
- **AND** the old cold-only and partial limitations keep their dates and the new reconciliation resolves their measured resume axes without claiming that task 11.1 or complete current-version interface qualification follows

#### Scenario: Supplied Codex proof establishes the live resume axes
- **GIVEN** the September 16 live record and raw report identify codex-cli 0.154.0 and a successful workspace-write cold control, followed by attributable completed command output
- **WHEN** the existing Codex live-proof task is reconciled with adapter assertions that agree with those observations
- **THEN** its return cites each of the five axes separately: exact demonstrated resume argv; effective class and applicable sandbox fragment re-imposed; exact same-root confirmation; current-only accounting; and pre-work rejection
- **AND** the recorded cold read-only denial, successful bare-resume write and restored-class denial establish the need for `-c sandbox_mode`; the denied write reports `EXIT=2` and `Read-only file system` even though its reporting shell exits zero
- **AND** the root is exactly `01a0aaa4-8667-7753-94b8-b0a60607524b`, the two resumed output counts are 303 and 308 for separate invocations, and the unknown-session rejection is exit 1 with the provider's named cause and neither thread nor turn start
- **AND** task 10.5 completes on that cited evidence and agreeing assertions without repeating the provider experiment; the no-boxed-hands sandbox override does not qualify a boxed MCP fragment or every allowed passthrough option

#### Scenario: Supplied interface evidence completes dependencies but not enablement
- **GIVEN** task 10.5 is complete and `85cf6d55` records 10.1's three deferred observations from the September 17 controller record on codex-cli 0.154.0
- **WHEN** task 11.1's full installed-version acceptance is evaluated
- **THEN** those dependencies and their exact citations are adopted without remeasurement, and 11.1 completes only if its matching adapter assertions, compiling removal proofs, coherent evidence claims and required local validation are all established
- **AND** otherwise 11.1 stays unchecked with the exact outstanding obligations, without repeating the obsolete claim that the three interface observations are absent
- **AND** the shipping disposition, both 0.154.0 identity fields and every refusal check remain intact; dependency completion grants neither a new shape nor a task tick

#### Scenario: Parsed options have conflicting qualification provenance
- **GIVEN** the September 17 record lists `--ephemeral` and `--output-schema` as not previously supported while the inherited adapter already allows them with historical interface attribution
- **WHEN** the bounded 11.1 qualification accounts for that discrepancy without a new provider measurement or human ruling
- **THEN** it records the unresolved qualification separately from parser availability, claims no new confinement proof and makes no semantic expansion or withdrawal of those inherited entries
- **AND** if that residual prevents full acceptance, 11.1 remains unchecked and names the missing evidence; neither preservation nor a passing surface-membership assertion resolves the discrepancy by itself

#### Scenario: The preserved inline Codex coordinate is judged by its adapter
- **GIVEN** inline work-class Codex sites compiled with the shipped assessment and author-pinned argv, including the raw commands in `recipes/standby` and `recipes/wager-harness`, and the following verify single/panel forms
- **WHEN** the production engine composes each cold invocation and, for each supported form, its eligible same-instance retry, preserving the real dialect wrapper where present, and the adapter judges that executing site's own facts
- **THEN** the decisions and exchanges match every row below; the assessment, declaration pins, hands, agent record and driver evidence retain the same structural owner throughout

| Form | Executing site | Required decision and exchange |
|---|---|---|
| No-hands single, wrapped | `verify:checks` | Explicit `boundary: not applicable`, `hands: none`; rejoin the cold invocation's confirmed root with current sandbox/effort re-expressed and confirmed `launch: resumed`. |
| Hands-bearing inline panel member, wrapped under namespace | `verify:checks:alpha` | Its own namespace boundary and boxed hands remain effective in composition; the production shape gate returns `restrictions-unavailable` and never enables a resumed work-site launch. |
| No-hands inline panel member, wrapped | `verify:checks:x` | Explicit no-hands facts; the supported confirmed retry rejoins even beside a hands-bearing member named `checks:x`, whose facts belong to `verify:checks:checks:x`. |
| No-hands single, unwrapped | `verify` | The same supported confirmed retry remains live with its own explicit facts and current restrictions. |
| Hands-bearing inline panel member, unwrapped under namespace | `verify:alpha` | The same namespace/boxed shape remains refused by the production shape gate as `restrictions-unavailable` with its hands effective. |
| No-hands inline panel member, unwrapped | `verify:x` | The supported confirmed retry remains live; a hands-bearing sibling `checks:x` supplies none of this site's facts. |

- **AND** every row asserts the production provider gate's enabled or refused decision and the refused row's exact token; assertions about the compiled facts alone do not discharge any row
- **AND** for the supported rows the cold invocation records the qualified root, the engine offers that exact root and the provider confirms it before current work; removing the assessment or required accounting evidence produces `unsupported-resume`, removing either confinement marker produces `restrictions-unavailable`, and removing the declared boundary's support refuses the retry
- **AND** a hands-bearing member with unknown or inapplicable confinement refuses `restrictions-unavailable` in both wrapped and unwrapped forms; even affirmative `namespace` / `boxed` markers retain that refusal under the shipped shape, while an actually supported control with both correct, applicable markers retains live rejoin without relabelling the refused member as no-hands
- **AND** the engine-composed supported `harness` coordinate also retains affirmative boundary/hands facts and its live rejoin; preserving inline sites does not regress that coordinate
- **AND** a member called `checks` retains its complete path, and an independent literal `verify:foo` phase retains its own facts instead of moving by label prefix
- **AND** deterministic validators and gate-class sites receive no offer, and no harness replaces missing compiled facts to obtain a passing launch
- **AND** refused forms are judged from their real engine-composed confinement facts even when no eligible root exists; a no-offer cold launch invents no refusal record, while an offered safe cold fallback records the gate's bounded refusal under LE2

#### Scenario: The other shipped declarations describe unmeasured new rejoins
- **GIVEN** main does not perform the Claude boxed-workspace, DSH headless-work or LaneTally wrapper-work-site rejoin, and each declaration is unmeasured
- **WHEN** the operator ruling preserves Codex's shipping rejoin and an otherwise eligible offer reaches one of these three shapes
- **THEN** that shape still declines with `unsupported-resume`; its declaration explains that it is a new unmeasured rejoin, with the provider-specific missing proof, and retains its status, identity, evidence and execution scope
- **AND** DSH gains no wrapper digest or session-selection invocation, and neither partial Claude/DSH observations nor plain Claude evidence qualify LaneTally

#### Scenario: Preserved Codex support still refuses mismatched evidence
- **GIVEN** Codex's shipping work-site shape is supported under the operator ruling
- **WHEN** the observed executable identity is missing or unreadable, differs from declared applicability or the originating root's recorded identity, the invocation's boundary or hands do not match, or required current-accounting evidence is absent or inapplicable
- **THEN** the invocation does not rejoin and any permitted safe cold launch reports the corresponding bounded refusal under LE2
- **AND** missing, unreadable or drifted executable identity, or a mismatch with a recorded originating identity, reports `unverified-harness`; a boundary/hands mismatch or absent confinement marker reports `restrictions-unavailable`; an absent or unsupported assessment or absent accounting evidence reports `unsupported-resume`, with the existing closed-gate precedence
- **AND** independently testing missing boundary alone, missing hands alone, both missing, null markers, non-string markers and markers outside the admitted vocabulary against an otherwise supported assessment always declines as `restrictions-unavailable`, before any resume launch; the adapter never interprets missing evidence as `not applicable` / `none`
- **AND** an unregistered executing owner or unresolved hands state remains unknown even when an enclosing or sibling input previously carried affirmative markers; production composition clears or replaces those facts and the otherwise supported assessment declines `restrictions-unavailable`, while a positively resolved no-hands control retains its live rejoin
- **AND** an absent or unsupported assessment, or absent required accounting evidence, still declines `unsupported-resume`; an assessment alone or a sibling's evidence never establishes the executing site's confinement
- **AND** a different site, instance or unverifiable local origin supplies no offer under SR2; a provider that fails to confirm the exact offered root never yields `launch: resumed`

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
- **GIVEN** common plumbing is prepared but a required new rejoin that main does not perform is disabled for missing or outdated evidence
- **WHEN** dated installed-provider evidence establishes the required interface, exact-session confirmation, all current restrictions and current-only accounting
- **THEN** the corresponding measured path is implemented and enabled, its declarations and documentation agree, and its measurement and implementation tasks complete only with cited verification
- **AND** remaining unmeasured new rejoins stay disabled; deterministic shim success alone cannot enable them, while preservation of shipping Codex remains a separate obligation

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
satisfies the same current restrictions. Affirmative confinement facts SHALL
come from this invocation's owned site; missing markers SHALL NOT establish
that no boundary or hands applies, even if an assessment reaches the adapter
through an incomplete relocation. If the cold path is itself inadmissible,
it SHALL refuse under the existing boundary/adapter rules instead of dropping
a restriction. Resume SHALL NOT promote trust, add a boundary backend, admit
an unsupported hands shape or change which adapters can hold a gate.

#### Scenario: Codex re-imposes its sandbox and effort
- **WHEN** a Codex invocation with a supported explicit class and effort rejoins its own thread in either shipping work-site coordinate (`harness` or inline `not applicable`, both with `hands: none`)
- **THEN** the production-composed resume argv carries the effective class through `-c sandbox_mode=<class>` (including the adapter's TOML quoting), contains no `-s`, `--sandbox` or `--sandbox=...`, and selects the exact offered root
- **AND** either `--effort <level>` or `--effort=<level>` becomes exactly one `-c` pair with the exact key `model_reasoning_effort` and the adapter's quoted level, with no `--effort` spelling surviving on resume; current model, workdir and result delivery remain effective
- **AND** the September 17 strict-config acceptance of the real key and rejection of `model_reasoning_effrot` establish field recognition; they do not measure every effort value or prove that a resume inherits effort
- **AND** the declaration explains why the override is necessary: the bare resume drops the class and the resume subcommand offers no sandbox flag

#### Scenario: A Codex resume reads the current prompt from stdin
- **GIVEN** the measured resume grammar is `[SESSION_ID] [PROMPT]` and the documented prompt positional `-` reads stdin
- **WHEN** an eligible Codex invocation resumes its owned thread
- **THEN** the final two argv parts are exactly the offered thread followed by `-`, and the current invocation's distinctive multi-word prompt reaches provider stdin byte-for-byte
- **AND** no prompt text, alternate handle or extra positional argument displaces either position

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
- **AND** the new 0.154.0 `--worktree` and `--thread-source` shapes remain unqualified; on an otherwise eligible supported offer their unadmitted passthrough still declines as `incompatible-argv`, without weakening any earlier gate refusal

#### Scenario: The measured exec-only surface cannot travel to resume
- **GIVEN** an otherwise eligible Codex offer and the September 17 record's exec-only set `--sandbox`, `--cd`, `--add-dir`, `--approve-for-me`, `--color`, `--local-provider`, `--oss`, `--profile` and `--version`
- **WHEN** the adapter composes the invocation from the seat's options
- **THEN** no member of that set reaches a resume argv; the declared sandbox is translated only through AS2, while every other member declines as `incompatible-argv`, retaining the safe cold argv and recording no resumed root
- **AND** separate and joined value spellings and bare options retain the same refusal; the argument-check assertion identifies the exact offending part rather than merely observing an error
- **AND** every admitted long passthrough option belongs to the measured resume surface, but membership alone never admits a new option or qualifies its restrictions
- **AND** `--worktree` and `--thread-source` are explicitly parsed by the provider and still refused by the adapter on an otherwise eligible offer; earlier gate refusal precedence remains unchanged

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
