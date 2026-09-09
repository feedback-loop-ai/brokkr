## Purpose

Leave a truthful, durable account of completed and pending work when a smith
is interrupted before committing, so either a resumed or a cold successor can
recover safely (decisions 0041 and 0042; progress semantics reserved for
proposed decision 0056).

## ADDED Requirements

### Requirement: PM1 Task progress is persisted when work is completed

The implementing smith SHALL update the dialect's task artifact as each task
or coherent task group finishes, before starting the next group and without
waiting for a commit or phase completion. In this OpenSpec change the marker
SHALL live in `tasks.md`, using its numbered task checkboxes and concise
progress notes alongside them or in a `## Progress` section. It SHALL survive
the seat process ending because it is written to the worktree, not held only
in conversation, a final result or an unexecuted commit plan.

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

On retry or re-entry, whether resumed or cold, the smith SHALL read the
proposal, requirements, design, tasks and current worktree state before
continuing implementation. It SHALL reconcile completed markers against the
surviving edits and cited verification, preserving work that still satisfies
the task and investigating partial or conflicting changes.

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

The smith's rendered instructions SHALL explicitly carry the update timing,
checkbox meaning and recovery behavior of PM1 and PM2. Changed source and
packaged charter/dialect instructions SHALL remain consistent. The instruction
SHALL use the realm's dialect task artifact rather than introduce a competing
progress store or journal payload.

Judges SHALL inspect progress and report findings without checking tasks,
editing notes or changing the tree. Proposed decision 0056 SHALL distinguish
deterministic enforcement of instruction/packaging consistency from judgment
about whether a task's work is actually complete; a prompt instruction SHALL
not be represented as an automatic engine guarantee (decisions 0041 and 0042).

#### Scenario: A delivered smith prompt
- **WHEN** the source instructions and any packaged form render a smith's implementation or return prompt
- **THEN** both state that progress is persisted before the next group, verification must be truthful and recovery reconciles with the surviving worktree

#### Scenario: A judge finds a false marker
- **WHEN** verification or review finds a checked task whose claimed work is incomplete
- **THEN** the judge reports a finding to its owning artifact and leaves the task file and worktree unchanged

#### Scenario: Progress timing is demonstrated without a provider session
- **WHEN** delivery exercises a temporary worktree with a completed group, recorded progress and interruption before commit
- **THEN** the persisted artifact demonstrates PM1 and PM2's recovery behavior, and the evidence identifies the exercise and instruction checks without claiming a live model always obeys them

### Requirement: PM4 Task completion does not stand in for delivery proof

The breakdown SHALL link each task to the named capability requirement it
serves, and the implementation/review evidence SHALL cover the corresponding
scenarios. Required tests SHALL extend the existing Rust protocol, adapter,
runtime, conformance and artifact-instruction suites where applicable; fixture
inputs SHALL be temporary or additive test data without changing the frozen
evaluator corpus.

Whole-story completion SHALL require the mandatory OpenSpec artifact,
clarification, analysis, implementation and specification-review phases, with
findings resolved in their owning artifacts and the normal archive/provenance
operation performed by the smith. A completed group or specification draft
SHALL not claim that later phases have passed.

Validation SHALL use `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2` for:
`cargo fmt --all -- --check`;
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
`cargo test --workspace --all-features --locked`; and both
`cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` and the
same command for `bundles/verify`. Exact coverage SHALL use the unchanged
gate and the shared pinned compiler; required host evidence SHALL use
`TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh`
with the same job limits. A missing tool, failed check or skipped nested
boundary test SHALL not count as passing.

The delivery record SHALL distinguish completed local preparation from
pending controller-owned host validation, integration, publication, final
PR/CI/merge and issue closure. The smith SHALL commit completed work unsigned
in repository style and SHALL NOT push, merge or close the issue. Only actual
external evidence SHALL complete a pending handoff check.

#### Scenario: Group checks pass while workspace validation is pending
- **WHEN** a completed task's focused verification passes but the full workspace suite has not run
- **THEN** its task marker can describe that local completion while the separate workspace-validation task remains pending

#### Scenario: Coverage runs in a nested sandbox
- **WHEN** local testing cannot create the required namespace or cannot run the exact-coverage toolchain
- **THEN** the record names the limitation and keeps host exact coverage pending, without lowering the gate or claiming skipped boundary tests prove it

#### Scenario: Specification has passed its validator
- **WHEN** proposal and capability deltas pass OpenSpec strict validation
- **THEN** the specify artifact is drafted, while design, tasks, judged loops, implementation and delivery remain unclaimed

#### Scenario: Controller results do not yet exist
- **WHEN** the implementation has been locally prepared and committed but host proof, final CI or publication has no result
- **THEN** those handoff steps remain pending and neither a checked task nor a local commit is reported as remote completion
