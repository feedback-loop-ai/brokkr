## Purpose

Leave a truthful, durable account of completed and pending work when a smith
is interrupted before committing, so either a resumed or a cold successor can
recover safely (decisions 0041 and 0042; progress semantics reserved for
proposed decision 0056).

## ADDED Requirements

### Requirement: PM1 Task progress is persisted when work is completed

The implementing SDD smith SHALL update the current change's task artifact,
as named by the realm's dialect, as each task or coherent task group finishes,
before starting the next group and without waiting for a commit or phase
completion. For every OpenSpec change the marker SHALL live in that change's
`tasks.md`, using numbered task checkboxes and concise progress notes alongside
them or in a `## Progress` section. A spec-kit change SHALL use its own phased
task rows and progress notes in its dialect-declared task artifact. The marker
SHALL survive the seat process ending because it is written to the worktree,
not held only in conversation, a final result or an unexecuted commit plan.

A checked task SHALL mean its stated acceptance work and required task-level
verification have completed. A group SHALL not be marked complete while any
of its required tasks remain incomplete. Completed implementation with pending
verification SHALL stay unchecked, with a note identifying the implemented
portion, remaining check and next action. Before beginning a group, the smith
SHALL mark which group is in progress so an interruption does not make partial
edits appear unexplained.

The marker SHALL distinguish task completion, verification, uncommitted edits
and whole-story completion. It SHALL contain concise work/evidence references,
not copied provider transcripts, credentials or private reasoning.

#### Scenario: Progress applies to the next change as well
- **WHEN** an SDD smith implements any subsequent OpenSpec change
- **THEN** it persists progress in that change's `tasks.md` as groups finish, with the same checkbox meaning and recovery rules; the rule is not limited to the change that introduced this capability

#### Scenario: A group completes before any commit
- **GIVEN** a group has completed its implementation and required focused checks
- **WHEN** the smith is about to start the next group
- **THEN** the completed tasks are already checked in the persisted task artifact and their evidence is named, even though their edits remain uncommitted

#### Scenario: Interruption before commit
- **GIVEN** group one is complete and recorded, group two is marked in progress, and no phase commit has occurred
- **WHEN** the seat dies during group two and its successor starts cold
- **THEN** the successor reads group one's checked work and verification references plus group two's incomplete status from the worktree instead of finding every task unchecked

#### Scenario: Implementation done but tests pending
- **WHEN** code for a task is written but its required verification has not run or has failed
- **THEN** the task remains unchecked and the progress note distinguishes the implemented portion from the pending or failed verification

#### Scenario: Partial group completion
- **WHEN** two tasks in a group pass their acceptance checks but a third is still being edited
- **THEN** the two completed tasks can be checked, the third remains unchecked, and the group does not claim completion

### Requirement: PM2 Recovery reconciles the marker with surviving work

On retry or re-entry, whether resumed or cold, the SDD smith SHALL read the
current change's specification, design/plan and task artifacts as named by the
realm's dialect, including the proposal where present, and the current worktree
state before continuing implementation. It SHALL reconcile completed markers
against the surviving edits and cited verification, preserving work that still
satisfies the task and investigating partial or conflicting changes.

A checkbox SHALL not by itself prove that code or tests still satisfy a
requirement. Missing edits, failed checks or a changed dependency that
invalidates prior completion SHALL return the affected task to pending with a
reason in the task artifact. Still-valid tasks SHALL not be blindly restarted,
and partial uncommitted edits SHALL not be erased merely because the successor
does not remember authoring them. Recovery SHALL not read another session's
private transcript to reconstruct progress.

#### Scenario: A cold successor finds valid completed work
- **WHEN** completed markers match surviving edits and applicable verification evidence
- **THEN** the successor preserves those edits and continues from the recorded pending work, without treating uncommitted status alone as a reason to redo them

#### Scenario: A checkbox outlives its implementation
- **WHEN** a task is checked but its required edit is missing or its relevant verification fails
- **THEN** the successor records the discrepancy, returns the task to pending and repairs or re-verifies it before checking it again

#### Scenario: A resumed thread remembers a different state
- **WHEN** conversation memory conflicts with the current worktree, artifacts or recorded checks
- **THEN** the smith reconciles against the current artifacts and surviving evidence rather than trusting its remembered completion

#### Scenario: A return changes an earlier requirement
- **WHEN** a judged finding changes the proposal, requirement or design on which checked work depends
- **THEN** affected task completion and progress notes are reconciled in dependency order, and an earlier artifact fault is reported upstream instead of hidden in implementation

### Requirement: PM3 Progress remains the smith's artifact and judges remain read-only

The shared SDD smith charter SHALL carry the update timing, checkbox meaning
and recovery behavior of PM1 and PM2 into implementation and return prompts.
The rule SHALL apply to SDD smiths using either shipped dialect. The charter
SHALL name the realm's dialect task artifact generically, with no framework
paths or repository commands; the dialect SHALL continue to own the artifact's
location, task syntax and artifact-phase instructions. No competing progress
store, journal payload or dialect implement phase SHALL be introduced.

The source charter and any distributed copy SHALL carry the same rule.
Existing source and packaged dialect instructions SHALL keep their matching
paths and formats. The non-SDD implementer charter SHALL not acquire a dialect
task-artifact obligation from this capability. Restricting progress timing to
OpenSpec or duplicating it in per-dialect task templates is rejected because
the SDD charter already owns implement-time completion and the dialects own
artifact conventions (decision 0042 ruling 6).

Judges SHALL inspect progress and report findings without checking tasks,
editing notes or changing the tree. Instruction/rendering/packaging consistency
SHALL be checked deterministically; whether a task's actual work is complete
SHALL remain a matter for verification and judgment. A prompt instruction SHALL
not be represented as an automatic engine guarantee (decisions 0041 and 0042).

#### Scenario: The shared SDD charter is the instruction home
- **WHEN** this progress rule is authored in the library
- **THEN** `agents/charters/implementer-sdd.md` changes to state PM1/PM2's timing, checkbox meaning and recovery generically, and its existing instruction/identity and rendered-prompt tests are updated
- **AND** `agents/charters/implementer.md` is unchanged because its non-SDD office requires no dialect task artifact
- **AND** `dialects/openspec.json`, `dialects/speckit.json`, their instruction directories and their copies under `crates/brokkr-cli/dialects/` retain the existing phase maps and task formats; neither dialect gains an implement phase or a duplicate timing rule

#### Scenario: Both dialects render the rule
- **WHEN** the shared SDD charter renders an implementation or return prompt with OpenSpec and again with spec-kit
- **THEN** both prompts require progress before the next group, truthful verification and reconciliation with the surviving worktree
- **AND** OpenSpec still uses its change's numbered `tasks.md` checkboxes and spec-kit its phased task rows, with each location supplied by the corresponding dialect rather than hard-coded in the charter

#### Scenario: A judge finds a false marker
- **WHEN** verification or review finds a checked task whose claimed work is incomplete
- **THEN** the judge reports a finding to its owning artifact and leaves the task file and worktree unchanged

#### Scenario: Progress timing is demonstrated without a provider session
- **WHEN** delivery exercises a temporary worktree with a completed group, recorded progress and interruption before commit
- **THEN** the persisted artifact demonstrates PM1 and PM2's recovery behavior, and the evidence identifies the exercise and instruction checks without claiming a live model always obeys them

## MODIFIED Requirements

### Requirement: PM4 Task completion does not stand in for delivery proof

The breakdown SHALL link each task to the named requirement it serves, and
the implementation/review evidence SHALL cover the corresponding scenarios or
acceptance criteria. Required tests SHALL extend the applicable existing suite;
fixture inputs SHALL be temporary or additive test data without modifying a
frozen evaluator corpus.

A checked task SHALL NOT replace any artifact validation, judged phase,
implementation check or completion obligation required by the selected dialect,
recipe and realm house rules. Findings SHALL be resolved in their owning
artifacts; a completed group or specification draft SHALL NOT claim that later
phases have passed. A missing tool, failed check or skipped required check
SHALL NOT count as passing evidence. A tracked task whose required proof can be
obtained before the final artifact operation SHALL remain pending until that
proof exists.

A mandatory external result whose subject is the immutable delivery commit or a
later integrated head SHALL instead be explicit non-checkbox handoff evidence.
It SHALL be recorded by the responsible controller in a journal or evidence
store outside the tracked change artifact and keyed to the exact head it judges;
recording the result SHALL NOT mutate that head. Until the real result exists,
the corresponding host-validation, integration, publication, merge or closure
claim SHALL remain pending. Its absence SHALL NOT leave an archived task
unticked after every repository-local tracked obligation is complete.

All tracked task and progress edits SHALL finish while an OpenSpec change is
active, and the dialect's archive operation SHALL remain the final artifact
operation. Archived validation and later exact-head evidence SHALL be read-only
with respect to that artifact. A failed archived check SHALL require reopening
the same change before repair; later controller evidence SHALL NOT be written
back into an archived task file.

Per-commission commands, resource limits, signing instructions and handoff
owners SHALL be recorded in the change's planning artifacts and task breakdown
(the proposal and tasks under OpenSpec), not promoted as standing capability
requirements. Repository-local obligations SHALL be tracked as tasks. Exact-head
controller obligations SHALL be stated explicitly in non-checkbox handoff prose
and distinguished from completed task preparation without redefining or waiving
either kind of evidence.

#### Scenario: Group checks pass while workspace validation is pending
- **WHEN** a completed task's focused verification passes but the full workspace suite has not run
- **THEN** its task marker can describe that local completion while the separate workspace-validation task remains pending

#### Scenario: Required proof cannot run locally
- **WHEN** the current environment cannot execute a required check
- **THEN** the task artifact names the missing evidence and leaves its verification pending, without weakening the requirement or treating a skipped check as a pass

#### Scenario: Specification has passed its validator
- **WHEN** proposal and capability deltas pass OpenSpec strict validation
- **THEN** the specify artifact is drafted, while design, tasks, judged loops, implementation and delivery remain unclaimed

#### Scenario: Run-specific instructions do not become standing truth
- **GIVEN** a commission chooses particular build limits, commit signing and an owner for external validation
- **WHEN** the task breakdown is authored and the capability is later promoted
- **THEN** the concrete commands and responsibilities are preserved in that change's planning artifacts and tasks, while the promoted capability retains only the progress, coverage and truthful-evidence rules
- **AND** a later commission can choose different delivery settings without amending this capability

#### Scenario: External results do not yet exist
- **GIVEN** every repository-local tracked obligation is complete and the dialect requires every archived task checkbox to be ticked
- **WHEN** host coverage, integration, remote CI, publication, merge or closure must judge an immutable delivery or integrated head and its real result does not yet exist
- **THEN** the change archives with all tracked tasks complete while that mandatory condition remains explicitly pending as non-checkbox controller evidence keyed to the head outside the tracked artifact
- **AND** neither the archived verification, a checked implementation task nor a local commit is reported as the missing external result

#### Scenario: An unchanged returned fold closes idempotently
- **GIVEN** an OpenSpec change was folded once, retains exactly one provenance pointer in each affected living capability, and is reopened only to complete repository-local tracked work without changing any delta
- **WHEN** all tracked tasks are completed while active and the normal dialect archive operation runs as the final artifact operation
- **THEN** it moves the same change to the archive with zero specification updates, leaves living capability bytes and provenance pointers unchanged, and permits strict archived verification of the all-ticked task artifact

#### Scenario: A returned specification repair updates only its owner
- **GIVEN** an already-folded OpenSpec change is reopened and a returned finding changes one requirement while its other deltas still equal living truth
- **WHEN** the repaired requirement is represented as MODIFIED and the normal dialect archive operation runs after every tracked task is complete
- **THEN** it replaces that living requirement, treats the identical deltas as no-ops, retains the existing provenance pointer exactly once in every affected capability, and makes no post-archive task edit
