## Purpose

Keep the memory of each model invocation with the adapter instance that opened
it, across retries and re-entry, without transferring a session to another
site, candidate or installation (decision 0030; proposed extension reserved
as decision 0056).

## ADDED Requirements

### Requirement: SR1 Every model invocation site receives its own eligible offer

The engine SHALL offer the latest durably recorded provider session, subject to SR2,
to each model invocation site on retry and phase re-entry, including operator
retry after a park in a new engine process. A site SHALL distinguish its run,
outer seat, selected body and full member/step path: a single seat, a panel
member, a sequence model step, or a member of a sequence panel. A label reused
under a different parent SHALL NOT identify the same site. Deterministic exec
and dialect validation steps SHALL NOT receive provider session offers.

Offer selection SHALL be per site, not per panel or sequence aggregate.
Composite execution order, join rules, completed-step reuse and gate policy
SHALL remain unchanged: this requirement governs invocations that actually run.

#### Scenario: A single site's retry and re-entry
- **GIVEN** a local run whose work seat records session A and later session B under the same eligible instance
- **WHEN** the seat retries after A and is later re-entered after B
- **THEN** the wire offers A to the retry and B to the re-entry, and offers nothing to the original cold invocation

#### Scenario: Independent panel members
- **GIVEN** panel members alpha and beta record different sessions
- **WHEN** the panel invokes those members again
- **THEN** each receives only its own eligible session, regardless of which member most recently checkpointed

#### Scenario: Model steps and a nested panel
- **GIVEN** a sequence contains a model author, a deterministic validator, and a panel with alpha and beta
- **WHEN** those sites run again after retry or re-entry
- **THEN** the author and each panel member receive their respective eligible sessions and the validator receives none

#### Scenario: Repeated labels do not alias
- **GIVEN** two sequence panels each have a member called alpha, and another seat also has an alpha member
- **WHEN** any alpha member retries
- **THEN** sessions belonging to the other full site paths are never offered to it

#### Scenario: A sibling changes candidate
- **GIVEN** alpha's instance is unchanged but beta's selected model changes within the same pinned panel
- **WHEN** both members are invoked
- **THEN** alpha remains eligible for its own session, while beta never receives the session opened by its previous candidate

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
no session SHALL NOT invent a replacement handle. Eligibility SHALL remain a
derivation of durable evidence and local origin, not a new policy input or a
model's claim about its identity (decisions 0004, 0007 and 0030).

#### Scenario: Chain fallback and subsequent re-entry
- **GIVEN** the second chain candidate opened the site's latest session after the first failed to start
- **WHEN** a new effect selects the first candidate again
- **THEN** it receives no handle from the second candidate, even if older evidence also mentions the first candidate

#### Scenario: A changed identity denies the offer
- **WHEN** the selected provider, model, effort, agent, driver, adapter digest, engine identity, or pinned bundle differs from the originating invocation
- **THEN** no session is offered; any existing manifest-mismatch refusal remains in force

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
captured for the site's root provider session. It SHALL NOT be derived by
truncating an identifier, selecting an ambient latest session, or treating a
transcript storage directory as a provider handle without measured equivalence.
Delegated child sessions SHALL NOT replace the root session.

Existing transcript kinds, locators, homes, size caps, retention and privacy
boundaries SHALL be preserved. Resume SHALL NOT fork or copy a session, seed
a fresh prompt from its transcript, search another site's private transcript,
or replace #222's transcript readers. Legacy evidence SHALL remain readable;
it SHALL admit an offer only when its kind and ownership establish the exact
provider handle under a measured mapping (decisions 0030, 0032 and 0034).

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
- **GIVEN** an old local single-seat checkpoint supplies a valid Codex thread identifier in its established flat or typed locator form and all ownership checks pass
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

Provider ownership checks SHALL remain the adapter's last line of defense:
an unrecognised handle or detectable credential/client change SHALL not be
resumed. The existing local fingerprint cannot establish undetectable token or
provider-home changes; documentation SHALL state that limitation without
claiming the engine authenticates provider session ownership.

#### Scenario: Operator retry in a new engine process
- **GIVEN** a local run parks after a session-bearing attempt
- **WHEN** the operator retries with the same pinned instance in a fresh process
- **THEN** every invoked model site derives its own eligible offer from the durable record

#### Scenario: Interruption before durable session evidence
- **WHEN** an initial invocation is killed after the provider announces a handle but before held pre-work rows reach the journal
- **THEN** the next invocation has no offer from that attempt and follows the safe cold path; memory-only evidence is not reconstructed

#### Scenario: Uncertain execution remains parked
- **WHEN** the engine recovers an effect whose execution is indeterminate
- **THEN** the existence of a resumable session does not cause automatic retry or cold replacement

#### Scenario: The provider no longer recognises the owner
- **WHEN** the engine's local checks pass but the adapter detects an incompatible client context or the provider rejects the owned handle
- **THEN** the adapter does not rejoin it and follows only the bounded refusal behavior specified by adapter-resume-safety
