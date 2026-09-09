## Purpose

Allow each model adapter to rejoin its own session only when the current
invocation's restrictions can be re-imposed, with measured provider behavior
and bounded cold recovery (decision 0030; proposed extension reserved as
decision 0056).

## ADDED Requirements

### Requirement: AS1 Resume support is measured per adapter and execution shape

Codex, Claude, DSH and LaneTally SHALL each have an explicit, reviewable support
assessment. It SHALL identify the installed CLI or wrapper version and source,
actual invocation form, root session selection, settings precedence, relevant
boundary/class, and the evidence establishing restriction enforcement and
resume confirmation. The assessment SHALL distinguish installed help/source,
deterministic shim observations and live enforcement probes, with date and
bounded reproducible observations. Help that lists a flag proves its interface,
not its enforcement.

Each measured safe supported shape SHALL be implemented. Unsupported shapes
SHALL be declared with their measured reason, and unavailable or unmeasured
evidence SHALL be labelled as such. An adapter SHALL NOT enable a path on an
assumption about inherited settings, treat a missing measurement as a measured
CLI defect, or leave a measured safe path cold merely to avoid the work.
Support declarations and packaged equivalents SHALL agree with the behavior;
guides and proposed decision 0056 SHALL record limitations and evidence.

Measurements SHALL use installed help/source first and only necessary bounded
probes with temporary test data. They SHALL NOT change global provider
settings, read unrelated sessions, invent CLI syntax or telemetry, or use an
unnecessary model experiment. Existing accepted measurements remain identified
as historical rather than relabelled as current probes.

#### Scenario: An installed help flag without enforcement proof
- **WHEN** the installed Claude help lists explicit resume and permission controls but no observation establishes that the complete restriction set binds on resume
- **THEN** the interface is recorded as available and enforcement as unmeasured; the affected resume shape is not enabled

#### Scenario: Evidence establishes a safe shape
- **WHEN** versioned interface and effective enforcement evidence establish safe rejoin for one adapter and boundary/class combination
- **THEN** that combination takes eligible offers and its declaration, tests and documentation describe the measured support

#### Scenario: The provider is unavailable in the box
- **WHEN** the authoring environment exposes no installed provider binary or source
- **THEN** the artifacts name the missing measurement and the later evidence obligation, and claim neither a successful probe nor a provider limitation

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
different session. No argument or rejected private value SHALL be copied into
launch evidence (decision 0034).

#### Scenario: Claude continue is not a selector for this seat
- **WHEN** passthrough requests Claude's continue/latest-session behavior or a different explicit session
- **THEN** it cannot override the exact engine offer, and any cold path cannot silently perform that ambient continuation

#### Scenario: Codex unsafe passthrough remains blocked
- **WHEN** passthrough contains an alternate selector, extra positional handle, sandbox bypass or conflicting configuration
- **THEN** the resume is not invoked and the refusal remains bounded; 0030's allow-list protections are preserved

#### Scenario: Generated Codex hands are measured explicitly
- **WHEN** the current boxed Codex invocation contains engine-composed MCP configuration alongside its sandbox class
- **THEN** the adapter either carries the complete measured safe fragment on resume or declares the specific unsupported shape, without admitting arbitrary configuration overrides

#### Scenario: A DSH profile or wrapper can replace session selection
- **WHEN** user passthrough or a wrapper setting would override the headless profile, selected session, model/effort overlay or restriction set
- **THEN** the resume is declined and the safe cold or refusal outcome names a bounded reason

#### Scenario: Identifier injection
- **WHEN** an offered handle contains a flag-like prefix, control characters, path traversal, shell syntax or an overlong value outside the measured grammar
- **THEN** it is never passed to the CLI as a selector, never truncated into another handle, and never echoed into the journal

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
