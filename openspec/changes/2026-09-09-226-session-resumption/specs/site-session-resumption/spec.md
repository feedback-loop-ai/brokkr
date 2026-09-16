## Purpose

Keep each model work site's memory with the adapter instance that opened it
across retries and re-entry, without transferring sessions or carrying a prior
judgment into a fresh gate invocation (decisions 0030, 0041 and 0042; proposed
extension reserved as decision 0056).

## ADDED Requirements

### Requirement: SR1 Every model work site receives its own eligible offer

The engine SHALL offer the latest durably recorded provider session, subject
to SR2, to each work-class model invocation site on retry and phase re-entry,
including operator retry after a park in a new engine process. A site SHALL
distinguish its run, outer seat, selected body and full member/step path: a
single seat, a panel member, a sequence model step, or a member of a sequence
panel. A label reused under a different parent SHALL NOT identify the same
site. Deterministic exec and dialect validation steps SHALL NOT receive
provider session offers.

Every execution-site evidence address SHALL have one structural owner across
the whole compiled bundle: phases, selected cases and defaults, steps,
members, wrapper-created sites and injected validators. Ownership SHALL be
checked before evidence collection can combine distinct owners and again
where wrapping creates executing addresses. Compilation SHALL refuse distinct
owners of one lookup address, naming both owners and the address, before
publishing an ambiguous bundle or dispatching work. This rule SHALL include
owners with no resume assessment or no populated evidence entry and
synthetic deterministic sites. Legitimate merges of evidence for the same
owner SHALL remain valid; equality of evidence from different owners SHALL
NOT make their alias legitimate.

Every gate-class model invocation SHALL start without a session offer, even
when its previous owned session would pass SR2. This applies to a single gate,
each member of a gate panel, a gate model step and each member of a gate panel
step, on retry, re-entry and operator retry alike. A single seat or panel uses
its selected compiled class; sequence steps use their own compiled class and
members inherit their enclosing panel's class. Office names and a later gate
step SHALL NOT make a work-class author or position a gate.

Extending the existing single-seat gate resume to other judges is rejected:
decision 0042 ruling 2 requires the next judge to be fresh and blind, reading
recorded answers from the artifacts. That later, specific judging rule bounds
0030's work-session continuity. The single-gate offer is removed as well; a
prior judging session is not authority for the next verdict. Judges still
receive their normally rendered current inputs and artifacts, never a prior
provider transcript or a synthesized continuation prompt.

Offer selection SHALL be per site, not per panel or sequence aggregate.
Composite execution order, join rules and completed-step reuse SHALL remain
unchanged: this requirement governs invocations that actually run. Gate
admission, immutability and existing park/fallback rules SHALL remain in force.

#### Scenario: A single site's retry and re-entry
- **GIVEN** a local run whose work seat records session A and later session B under the same eligible instance
- **WHEN** the seat retries after A and is later re-entered after B
- **THEN** the wire offers A to the retry and B to the re-entry, and offers nothing to the original cold invocation

#### Scenario: Independent panel members
- **GIVEN** work-class panel members alpha and beta record different sessions
- **WHEN** the panel invokes those members again
- **THEN** each receives only its own eligible session, regardless of which member most recently checkpointed

#### Scenario: Model steps and a nested panel
- **GIVEN** a sequence contains a work-class model author, a deterministic gate validator, and a work-class panel with alpha and beta
- **WHEN** those sites run again after retry or re-entry
- **THEN** the author and each panel member receive their respective eligible sessions and the validator receives none

#### Scenario: Repeated labels do not alias
- **GIVEN** two work-class sequence panels each have a member called alpha, and another work-class seat also has an alpha member
- **WHEN** compilation validates ownership and any alpha member retries
- **THEN** distinct complete structural addresses remain valid and sessions or confinement evidence belonging to another site are never offered or applied to it
- **AND** each otherwise valid collision below is refused at compilation with the shared address and both structural owners, irrespective of whether both owners populate an assessment or other evidence map

| Collision | Distinct owners sharing one lookup address |
|---|---|
| Raw cross-phase | Codex at phase `work`, selected case `chore`, and another provider without Codex work-site support at literal phase `work:chore`. |
| Raw cross-selector | Codex at phase `work`, case `chore`, step `engine:stage`, and the other provider at phase `work:chore`, case `engine`, step `stage`; both selectors carry valid defaults and pins. |
| Default and literal phase | A selected default's model site and a distinct literal phase spelling that site's full lookup address. |
| Wrapped single and literal exec phase | A no-hands Codex verify seat executing at `verify:checks` and a literal `verify:checks` exec phase carrying hands but no inline assessment. |
| Wrapped member and literal phase | A verify panel member executing at `verify:checks:alpha` and a distinct literal phase with that address, including an owner without resume evidence. |
| Injected validator and literal Codex phase | The deterministic validator at `verify:dialect-verify` and a no-hands Codex phase literally named `verify:dialect-verify`; the validator's hands and driver evidence cannot belong to that Codex site. |

- **AND** reversing declaration order does not change collision refusal; a raw ownership collision is refused before evidence collection even if wrapping would later separate its addresses, and wrapper-created ownership is checked when those executing addresses are claimed
- **AND** renaming only the conflicting outer phase in each construction permits compilation and the supported Codex site's confirmed retry reaches `launch: resumed` through the production gate with its own restrictions and declaration identity
- **AND** the validator and every gate-class site still receive no offer; ownership reservation grants no resume capability
- **AND** members `x` and `checks:x` are not themselves a collision: their complete families move to distinct executing addresses together, with the no-hands site's live retry and the hands-bearing site's refusal proved under AS1
- **AND** same-owner effort/resume witness merges remain valid, while removing collision enforcement makes the corresponding refusal assertion fail rather than an unrelated strategy, pin or vocabulary check

#### Scenario: A sibling changes candidate
- **GIVEN** alpha's instance is unchanged but beta's selected model changes within the same pinned work-class panel
- **WHEN** both members are invoked
- **THEN** alpha remains eligible for its own session, while beta never receives the session opened by its previous candidate

#### Scenario: A judge returns to a revised artifact
- **GIVEN** a gate's previous invocation recorded an owned session and the author has revised the artifact to answer its finding
- **WHEN** that single gate, gate-panel member, gate model step or nested gate-panel member runs again, including after operator retry
- **THEN** the engine sends no resume offer and the built-in adapter takes its fresh-session path under the same current gate restrictions
- **AND** the judge reads the current artifact and its recorded answers afresh; its old provider conversation is not carried into the new session and a previous verdict does not substitute for current judgment
- **AND** the accepted launch is cold with no resume-refusal reason because no offer was made

#### Scenario: A work author and its judging step have different eligibility
- **GIVEN** a sequence includes work-class council positions, a work-class chief author and a gate-class judge or validator
- **WHEN** the sequence invokes them again
- **THEN** each work model site can receive its own eligible session even though the sequence contains a gate, and each gate model site receives none
- **AND** a deterministic validator has no model launch or session at all

#### Scenario: Historical single-gate behavior is corrected
- **GIVEN** a local single-seat Codex gate has sufficient legacy session and ownership evidence for the old single-seat offer path
- **WHEN** it retries under the new eligibility rule
- **THEN** no session is offered, just as for every composite gate; historical journal rows are left unchanged

### Requirement: SR2 An offer preserves all established instance identity checks

An offer SHALL require the same run, site, selected agent/provider/model/effort,
adapter declaration pin, driver identity, engine identity and pinned bundle
that governed the session's originating invocation. Boundary and hands pins
SHALL continue to participate in bundle identity. The local installation's
existing origin check SHALL also hold. Missing, conflicting or ambiguous
ownership evidence SHALL yield no offer, never a guessed match.

The engine SHALL consider the latest session-bearing invocation of that site
and then check its ownership. It SHALL NOT search behind an incompatible
latest owner to resurrect an older candidate's session. A failure that records
no session SHALL NOT invent a replacement handle. A start that contains only
an assigned creation ID SHALL NOT count as session-bearing evidence or replace
the last provider-confirmed session. Eligibility SHALL remain a
derivation of durable evidence and local origin, not a new policy input or a
model's claim about its identity (decisions 0004, 0007 and 0030).

#### Scenario: Chain fallback and subsequent re-entry
- **GIVEN** the second chain candidate opened the site's latest session after the first failed to start
- **WHEN** a new effect selects the first candidate again
- **THEN** it receives no handle from the second candidate, even if older evidence also mentions the first candidate

#### Scenario: A changed identity denies the offer
- **WHEN** the selected provider, model, effort, agent, driver, adapter digest, engine identity, or pinned bundle differs from the originating invocation
- **THEN** no session is offered; any existing manifest-mismatch refusal remains in force
- **AND** changing only the copied Codex declaration with another provider still present invalidates the old pinned root at the actual retry decision, including wrapped sites; a map entry or whole-manifest digest assertion alone does not establish the refusal

#### Scenario: An imported or unverifiable origin
- **WHEN** the same event history is imported, copied to another installation, or lacks verifiable local origin
- **THEN** no prior session is offered, including on later retries of the imported run

#### Scenario: Partial ownership facts
- **WHEN** a composite checkpoint lacks an unambiguous site tag or its originating start evidence cannot be matched
- **THEN** the engine withholds the offer rather than borrowing an aggregate or sibling identity

#### Scenario: A failure supplied no replacement session
- **GIVEN** the site already has a durably recorded session and an intervening failed invocation records none
- **WHEN** the site is retried
- **THEN** only the last recorded session is considered, and all current instance checks still have to pass

### Requirement: SR3 The offered handle names the provider's own session

A resumable handle SHALL be the complete, validated identifier or selector
of the site's provider-confirmed root session. Two identity origins are admitted:
a provider-generated identifier captured when the root opens, or a fresh
identifier assigned by the owning engine/adapter through a measured provider
creation interface. Each supported path SHALL identify which mechanism it uses.
Assignment SHALL be specific to that site's cold creation, never an arbitrary
user selector, an ID borrowed from another session or a way to create a copy.
A resumed invocation SHALL select the existing confirmed root, not assign a new
identity, recreate the session or fork it. A shared open/create interface can
qualify only when measured behavior loads and continues the exact existing root;
accepting a caller-supplied ID alone SHALL NOT establish that behavior.

For either origin, eligibility SHALL require measured provider evidence that
the root actually opened under the originating site's instance, and that
evidence SHALL be durably recorded under adapter-launch-evidence LE3. For an
assigned ID, the provider-confirmed root SHALL match the assigned value exactly.
An assigned ID in configuration, argv or a durable pre-spawn start is creation
intent only: it SHALL NOT prove that the session exists or be published as a
captured session fact. A configuration echo without measured root-opening
semantics SHALL NOT count as confirmation. Assignment alone granting resume
eligibility is rejected because the provider might never have created a session
(decision 0030 ruling 4).

A handle SHALL NOT be derived by truncating an identifier, selecting an ambient
latest session, or treating a transcript storage directory as a provider handle
without measured equivalence. Delegated child sessions SHALL NOT replace the
root session. Persisted creation intent, if used, SHALL remain distinguishable
from confirmed session evidence within the admitted versioned record vocabulary;
no frozen record acquires a new field implicitly.

Existing transcript kinds, locators, homes, size caps, retention and privacy
boundaries SHALL be preserved. Resume SHALL NOT fork or copy a session, seed
a fresh prompt from its transcript, search another site's private transcript,
or replace #222's transcript readers. Legacy evidence SHALL remain readable;
it SHALL admit an offer only when its kind and ownership establish the exact
provider handle under a measured mapping (decisions 0030, 0032 and 0034).

#### Scenario: The provider generates the root identity
- **WHEN** a cold invocation receives a provider-generated root ID with measured opening semantics and its confirmed session evidence reaches the durable journal
- **THEN** that complete handle can be offered on an eligible retry after all SR2 checks pass; no preassignment is required

#### Scenario: The owning instance assigns a fresh creation identity
- **GIVEN** a measured provider creation interface accepts an engine/adapter-assigned fresh ID for this work site
- **WHEN** provider evidence confirms that exact root opened and that confirmation becomes durable under LE3
- **THEN** the handle is eligible under the same SR2 ownership rules as a provider-generated ID, and that creation's launch remains cold
- **AND** a later resume rejoins that confirmed root through a measured resume interface, including a shared open/create API only if it demonstrably loads that existing root; it neither forks nor creates another session with the same text

#### Scenario: Assignment is echoed without confirmed creation
- **WHEN** a generated ID appears in a start, argv, configuration echo or an event whose root-opening semantics are unmeasured
- **THEN** it supplies no session-existence proof and no new eligible handle, even if the request was durably recorded
- **AND** existing transcript fields and session IDs are not populated from that assignment as if the provider had confirmed them

#### Scenario: The provider disagrees with the assigned identity
- **WHEN** a cold creation request assigns one ID but provider evidence identifies a different root
- **THEN** the assigned ID is never offered or recorded as confirmed, and the mismatch follows the observed failure or uncertainty path without relabelling the different root as the requested session

#### Scenario: DSH locator and provider identifier differ
- **GIVEN** a DSH invocation records a retained directory locator and a distinct root provider session identifier
- **WHEN** its retry offers a session
- **THEN** the validated provider selector is offered and the directory remains transcript evidence; the directory is not guessed to be the resume identifier

#### Scenario: Only a DSH directory survived
- **WHEN** historical evidence contains only a DSH transcript directory and no measured mapping to a provider session
- **THEN** the retry starts without an offer and no private transcript scan invents one

#### Scenario: A delegated session is announced
- **WHEN** the harness announces a child session after its root session
- **THEN** the next eligible offer still identifies the site's root session

#### Scenario: A legacy Codex handle is sufficient
- **GIVEN** an old local single-work-seat checkpoint supplies a valid Codex thread identifier in its established flat or typed locator form and all ownership checks pass
- **WHEN** that site retries
- **THEN** it remains eligible for the exact thread, without rewriting the historical checkpoint

#### Scenario: A handle exceeds its admitted representation
- **WHEN** an identifier contains unsafe selectors or cannot fit its bounded contract field losslessly
- **THEN** it is not used as a resumable handle and is never truncated into a different valid-looking identifier

### Requirement: SR4 A negotiated offer belongs to exactly one attempt

The engine SHALL deliver an eligible handle using the protocol's negotiated
resume message before the corresponding start, never through the seat prompt
or policy input. Built-in model adapters SHALL be able to receive and decide
an offer, including declining an invocation whose safe resume path is
unsupported. Negotiating that capability SHALL NOT claim that every offered
session can be resumed. Drivers that do not negotiate receipt SHALL receive
no handle, and exec SHALL remain outside this capability.

The adapter SHALL correlate an offer with its effect and attempt, consume it
at most once, and discard it on cancellation, shutdown or rejection. A missing,
malformed, mismatched or ambiguous duplicate offer SHALL NOT become a later
start's session. The current attempt's prompt, result destination and scoped
hands configuration SHALL accompany a rejoin; old conversation instructions
SHALL NOT select another attempt's output or permissions.

#### Scenario: Negotiation distinguishes receipt from safe support
- **WHEN** a built-in model adapter receives an eligible offer but cannot impose the invocation's restrictions on resume
- **THEN** it declines the offer and uses the safe cold path with the launch evidence required by adapter-launch-evidence

#### Scenario: An unadvertised driver
- **WHEN** a third-party driver advertises no resume capability
- **THEN** the protocol sends only its normal start and never includes the private handle elsewhere

#### Scenario: Correlated one-use delivery
- **WHEN** a matching resume message precedes one start and a second start follows without an offer
- **THEN** only the first start can use that session

#### Scenario: A stale or duplicate message
- **WHEN** the offer's effect or attempt differs from the next start, or conflicting offers precede that start
- **THEN** none of those handles reaches the provider and none remains available to a later start

#### Scenario: A resumed seat writes the current result
- **GIVEN** the conversation remembers a previous attempt's result path and boxed tool configuration
- **WHEN** an eligible retry resumes it
- **THEN** its new instruction names the current result destination and it receives the current scoped hands configuration rather than reusing the old grant

### Requirement: SR5 Session recovery uses durable evidence without widening crash recovery

A fresh engine process SHALL derive the same eligible offer from the same
journal, pinned bundle and local origin as the prior process. It SHALL NOT
depend on in-memory adapter state or inspect another installation's settings.
A crash after effect start without a determinate terminal outcome SHALL retain
existing indeterminate parking; resume support SHALL NOT authorize automatic
re-execution of that effect (decisions 0006 and 0030).

SR3's two identity mechanisms SHALL have the same durability threshold. An ID
assigned before spawn, even if persisted as creation intent, SHALL NOT survive
an interruption as an offer unless provider confirmation also became durable
under LE3. Recovery SHALL NOT probe a guessed-to-exist session or reconstruct
lost provider confirmation from the start payload. A later authorized retry
without any confirmed prior session SHALL create a fresh session; a creation
assignment from the interrupted attempt SHALL NOT be reused as a resume selector.
An older confirmed session, if any, remains subject to SR2's unchanged checks.

Provider ownership checks SHALL remain the adapter's last line of defense:
an unrecognised handle or detectable credential/client change SHALL not be
resumed. The existing local fingerprint cannot establish undetectable token or
provider-home changes; documentation SHALL state that limitation without
claiming the engine authenticates provider session ownership.

#### Scenario: Operator retry in a new engine process
- **GIVEN** a local run parks after a session-bearing attempt
- **WHEN** the operator retries with the same pinned instance in a fresh process
- **THEN** every invoked work-class model site derives its own eligible offer from the durable record, while gate sites still receive no offer

#### Scenario: Interruption before durable session evidence
- **WHEN** an initial invocation is killed after the provider announces a generated or preassigned root handle but before held pre-work confirmation rows reach the journal
- **THEN** a later invocation authorized under the existing retry rules has no offer from that attempt and follows the safe cold path; memory-only evidence is not reconstructed

#### Scenario: An assigned ID is durable before the provider opens
- **GIVEN** a site's initial cold invocation has an assigned ID persisted as creation intent before spawn and no earlier confirmed session
- **WHEN** it is killed before spawn or before provider confirmation becomes durable
- **THEN** existing crash/indeterminate parking still applies, and a later authorized retry receives no offer from the assignment
- **AND** that retry uses a fresh cold creation identity, not the abandoned ID as a resume selector; durable intent does not prove existence

#### Scenario: An assigned root is confirmed durably before interruption
- **GIVEN** the provider confirmed a site's assigned root ID and the confirmation reached the journal under LE3 before interruption
- **WHEN** a later retry or re-entry is authorized under the existing execution rules
- **THEN** it derives the same owned offer as for a provider-generated root, subject to SR2, without treating session evidence as authority to re-execute an indeterminate effect

#### Scenario: Uncertain execution remains parked
- **WHEN** the engine recovers an effect whose execution is indeterminate
- **THEN** the existence of a resumable session does not cause automatic retry or cold replacement

#### Scenario: The provider no longer recognises the owner
- **WHEN** the engine's local checks pass but the adapter detects an incompatible client context or the provider rejects the owned handle
- **THEN** the adapter does not rejoin it and follows only the bounded refusal behavior specified by adapter-resume-safety
