# adapter-launch-evidence Specification

## Purpose
Give operators trustworthy cold/resumed evidence for every built-in model
adapter while retaining the seat record's privacy, accounting and acceptance
boundaries (decisions 0030 and 0034; proposed extension reserved as decision
0056).

## Requirements

### Requirement: LE1 Every model adapter reports actual launch outcomes

Codex, Claude, DSH and LaneTally SHALL implement the same launch semantics on
their model invocation paths. A launch checkpoint SHALL carry
`launch: cold` when the adapter actually takes the fresh-session path, and
`launch: resumed` only after provider evidence confirms rejoining the exact
offered root session. Passing a resume flag, preassigning a creation ID,
knowing an old handle, retaining edits, receiving historical transcript rows or
seeing a process exit zero SHALL NOT by itself prove a rejoin. A provider's
confirmed fresh creation remains cold even when the engine/adapter chose its
ID; assignment SHALL NOT be reported as captured session evidence before SR3's
provider confirmation.

Every invocation that becomes accepted with a known launch outcome SHALL
publish that outcome under LE3's ordering. An offered session that is declined
and results in a cold launch SHALL also carry a bounded refusal reason.
Absent an offer, a cold launch SHALL NOT invent a refusal. A resume whose
identity cannot be confirmed SHALL fail or remain indeterminate according to
the observed facts, never be labelled cold or resumed by guesswork or silently
executed again. Exec SHALL report no model launch.

#### Scenario: First invocation of each model adapter
- **WHEN** Codex, Claude, DSH and LaneTally each start model work through their fresh-session path without an offer
- **THEN** each publishes launch cold without a resume-refusal reason

#### Scenario: A gate starts fresh on re-entry
- **GIVEN** SR1 withholds a prior session because the invocation is gate class
- **WHEN** the model adapter's fresh-session invocation becomes accepted with a known launch outcome
- **THEN** it publishes launch cold without a resume-refusal reason, including for single gates and members or steps of composite gates
- **AND** no old gate session is selected through ambient continue or resume arguments

#### Scenario: Confirmed exact rejoin
- **WHEN** an adapter's measured provider evidence confirms the exact offered session before the invocation's first work is reported
- **THEN** its launch record says resumed and its session evidence identifies that root handle

#### Scenario: Requesting resume proves nothing alone
- **WHEN** the process receives a resume argument but dies or begins work without the confirmation required by its measured interface
- **THEN** no resumed fact is published, the uncertainty or failure is reported, and no internal cold replacement repeats that work

#### Scenario: A provider confirms an assigned cold identity
- **WHEN** a measured fresh-creation path assigns an ID and provider evidence confirms that exact new root before the invocation is accepted
- **THEN** its published launch is cold, its confirmed session evidence identifies that root, and later eligibility still requires SR2 and durable publication under LE3
- **AND** the matching creation ID never by itself produces launch resumed

#### Scenario: An unsupported offer is declined
- **WHEN** a model adapter receives an owned offer but its invocation cannot safely take it and the safe cold path starts
- **THEN** the launch is cold with a bounded reason; unsupported receipt is not silently indistinguishable from having no offer

#### Scenario: A rejected request followed by cold work
- **WHEN** a resume request is conclusively rejected before work and the single cold replacement starts
- **THEN** the invocation publishes cold with harness-refused evidence and never publishes an earlier speculative resumed row

#### Scenario: Deterministic execution has no model launch
- **WHEN** an exec or dialect validation step runs
- **THEN** it reports its existing lifecycle facts without a cold or resumed model-launch value

### Requirement: LE2 Launch evidence stays within a versioned closed vocabulary

Launch evidence SHALL contain bounded identifiers and established facts, never
commands, arguments, prompts, reasoning, tool output, credentials, raw rejected
handles or unrestricted refusal prose. Measured explanations SHALL live in the
adapter assessment and proposed decision 0056, not in seat accounting records.
The published and embedded contract bytes SHALL remain unchanged.

The v4 launch enum is exactly `cold | resumed`; its refusal enum is
`invalid-session-id | sandbox-unavailable | unsupported-sandbox |
incompatible-argv | harness-refused`, and its sandbox enum contains only
Codex's three admitted classes. Those names SHALL be used only where their
meaning accurately describes the observation. A provider permission mode
SHALL NOT be put in `sandbox`, and a version shall not acquire a private extra
value. If the required facts cannot be expressed honestly by the existing
fields, delivery SHALL first add a new bounded contract version beside the
old one, with matching embedding, version dispatch, compatibility and
append/export/verify tests. A semantic mismatch SHALL NOT be hidden by mapping
everything to a convenient existing refusal token (decision 0034).

If creation intent is persisted before provider confirmation, its representation
SHALL be distinguished from confirmed session facts in an admitted record
version. It SHALL NOT add a private field to frozen payloads or place an
unconfirmed assigned ID into captured-session fields. Design SHALL specify an
additive version first if no existing admitted representation can express that
distinction. Assignment does not relax identifier bounds or privacy rules.

#### Scenario: A bounded reason explains a safe decline
- **WHEN** an invalid handle, unavailable restriction, unsupported class, incompatible argument shape or measured harness rejection causes a cold launch
- **THEN** the record uses the corresponding truthful admitted token and the human explanation is in the reviewed adapter evidence

#### Scenario: Claude permission names are not Codex classes
- **WHEN** the adapter re-imposes a Claude permission mode
- **THEN** the mode is not emitted as a v4 sandbox value; the actual launch remains expressible without falsely assigning a Codex class

#### Scenario: A fact needs a new vocabulary
- **WHEN** council design establishes that a required launch fact cannot fit v4 without changing its meaning
- **THEN** the design specifies an additive version and its compatibility behavior before implementation, and all existing frozen bytes remain intact

#### Scenario: Persisting a creation request does not widen frozen records
- **WHEN** a design chooses to persist a preassigned creation ID before provider confirmation
- **THEN** it specifies an admitted representation of intent distinct from confirmed session evidence, using an additive version if necessary
- **AND** it neither silently adds a field to a frozen payload nor publishes the request as a confirmed session ID

#### Scenario: Private values fail at the fence
- **WHEN** a driver emits an unknown field, invalid enum, oversized identifier or private payload in its launch evidence
- **THEN** the store refuses the record before append, journals the attempt's failure through its existing refusal path, and diagnostics do not echo the rejected private value

#### Scenario: Engine boundary evidence remains authoritative
- **WHEN** a launch checkpoint also carries model and boundary facts
- **THEN** the existing engine boundary stamp and store validation continue to apply; a provider launch claim cannot change the realm's boundary

### Requirement: LE3 Launch reporting preserves first-work acceptance and refusal semantics

Launch and transcript-location rows alone SHALL NOT count as beginning work.
For adapters that classify pre-session refusal, pre-work launch facts SHALL be
held until actual work or a non-refusal ending establishes the existing
acceptance path. `Accepted` SHALL precede every emitted checkpoint, and held
facts SHALL be flushed in their observed order without a speculative resumed
classification.

A conclusively classified provider refusal before work and without delivery
SHALL retain a failed result with no `Accepted` and no checkpoints, so existing
failure-to-start and chain behavior do not change. Its failure reason SHALL
remain bounded and preserve the available refusal evidence. The absence of a
launch checkpoint on that path SHALL mean no durable launch checkpoint was
reported, not cold or resumed by inference. A kill while rows are held has the
same evidence limitation; documentation SHALL name this window. Preassigning
an ID SHALL NOT bypass withholding, flush a checkpoint early, reconstruct a
lost confirmation or make an unconfirmed start into a session-bearing record.

Demanding a launch checkpoint before acceptance is rejected for this path:
a checkpoint would move the refusal across the structural boundary and recreate
the shipped #219 defect. This exception concerns pre-work refusal or lost
evidence, not a blanket exemption for an adapter's normal cold and resumed work
(decision 0016 and shipped behavior described by proposed decision 0053).

#### Scenario: A real first turn flushes launch evidence
- **WHEN** a provider confirms its session, produces launch/location facts and then its first work checkpoint
- **THEN** the wire sends Accepted first, then the held confirmed facts in order, then the first work checkpoint

#### Scenario: Model refusal before work
- **WHEN** a provider reports its measured model/auth/quota refusal shape before work and delivers no result
- **THEN** the driver sends the bounded failed result with no Accepted and no checkpoint, even if it previously held a launch row

#### Scenario: A killed opening window
- **WHEN** the driver is killed while holding launch and session facts before the first work checkpoint
- **THEN** those facts are not fabricated from configuration on recovery, the absence is documented, and the watchdog kill does not become a provider failure to start

#### Scenario: Assigned identity does not bypass held confirmation
- **GIVEN** a cold invocation's assigned ID is already durable as creation intent and its provider confirmation is still held before work
- **WHEN** a conclusive pre-work provider refusal or a kill prevents those held facts from being published
- **THEN** no launch or confirmed-session checkpoint is fabricated from the assignment, and SR5 supplies no offer from that attempt
- **AND** the classified refusal retains its no-Accepted/no-checkpoint path, while a kill retains its existing watchdog or crash outcome

#### Scenario: An advisory is followed by real work
- **WHEN** an error-shaped provider event is followed by work or valid clean delivery
- **THEN** the actual session and launch facts are retained under the existing acceptance path rather than discarded as a refusal

#### Scenario: DSH cannot classify a refusal
- **WHEN** a DSH invocation supplies no measured machine-readable pre-session refusal shape
- **THEN** launch reporting does not invent that classifier from stderr, and its normal accepted cold or resumed invocation still reports launch

### Requirement: LE4 Resume does not recount old work as new activity

Launch evidence and served-model/effort evidence SHALL remain separate: a
requested pin SHALL NOT be reported as the served value without provider
evidence. A resumed invocation SHALL count only the current invocation's new
turns, tools, targets and reported usage. Replayed history, pre-existing
transcript bytes or repeated cumulative session totals SHALL NOT be counted
as new activity or additional billed usage.

The adapter SHALL establish a measured boundary between historical and current
events before enabling a resume shape. If it cannot distinguish a current
measurement from an old total, it SHALL omit that measurement under the existing
missing-evidence rules rather than invent zero, subtract a guessed baseline or
double-count. Inclusive input, cache-read subsets, cache writes, completion
deduplication and LaneTally's capture identity SHALL retain their established
meaning. This obligation concerns invocation evidence, not #222's transcript
reading or CLI/TUI derivation (decisions 0031, 0034 and 0035).

#### Scenario: A resumed stream replays old turns
- **GIVEN** a session has ten historical turns
- **WHEN** resume replays those turns and then reports two new turns
- **THEN** the new invocation records only the two new turns and their attributable usage, without adding historical tools or costs

#### Scenario: A cumulative usage total has no reliable baseline
- **WHEN** the resumed harness reports a session-lifetime total that cannot be attributed to the current invocation
- **THEN** that total is not journaled as newly billed usage, no synthetic zero is inserted, and the limitation is documented

#### Scenario: DSH reuses retained transcript storage
- **WHEN** the provider's measured resume form continues an owned retained session containing prior transcript data
- **THEN** only newly attributable invocation events reach the journal and existing retention, path boundaries and transcript caps remain unchanged

#### Scenario: LaneTally resumes through its wrapper
- **WHEN** a measured supported LaneTally invocation resumes and reports capture and model evidence
- **THEN** it retains LaneTally's capture marker and observed model/usage without substituting plain Claude or adding another session's ledger entries

### Requirement: LE5 Launch conformance covers every site and historical compatibility

Every emitted launch-bearing checkpoint and successful result SHALL pass the
same append-time, export and verification contracts as other seat evidence.
Conformance SHALL cover each built-in model adapter's no-offer cold launch,
safe work-site resume, declined offer and proven pre-work cold replacement
where supported, plus missing confirmation, malformed records and privacy
bounds. Gate-site tests SHALL prove no offer and fresh launch for every
admitted gate topology; they SHALL NOT assume that work-site resume eligibility
extends to judges.
Wire/site tests SHALL establish that a record belongs to the invocation that
actually received the offer; aggregate records SHALL NOT invent a member's
launch state.

Old journals with no launch field SHALL remain valid and unchanged, without
backfilling from argv, current declarations or another site's evidence.
Deterministic shims SHALL be labelled as protocol/accounting evidence and
SHALL NOT be cited as live provider enforcement proof (decision 0034).

#### Scenario: Each composite member keeps its launch
- **WHEN** one work-class panel member resumes and another starts cold, including inside a sequence
- **THEN** each member's tagged evidence retains its own launch, and the aggregate does not substitute one member's state for the other

#### Scenario: Historical launch absence
- **WHEN** an old valid Claude or DSH journal has no launch field
- **THEN** validation still accepts it and no new launch value is written into its history

#### Scenario: A shim passes without a live CLI
- **WHEN** deterministic provider shims prove argv, ordering, confirmation and record validation
- **THEN** those checks are reported as shim evidence and the separate installed-provider enforcement assessment remains explicitly measured or unmeasured

## Provenance

- `2026-09-09-226-session-resumption` — folded 2026-09-09
