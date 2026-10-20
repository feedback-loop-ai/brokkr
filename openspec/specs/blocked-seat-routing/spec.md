# blocked-seat-routing Specification

## Purpose
Let a blocked seat say whether its environment or the work blocked it,
so an environment block parks for the operator to fix and retry that one
seat, and a work block routes as it always did (decision 0091).

## Requirements

### Requirement: A blocked seat names what blocked it

A seat that reports `blocked` MAY say what blocked it in its result's `inputs.blocked_by`, one of `environment` or `work`. A table SHALL read it only through the `blocked_by_in` condition, whose values the loader SHALL check against that vocabulary. A present value outside it SHALL park the evaluation as unreadable, and an absent value SHALL satisfy no condition.

#### Scenario: A table that lists a blocker outside the vocabulary

- **WHEN** a rule reads `blocked_by_in: ["signing"]`
- **THEN** the table is refused at load, naming the rule, the condition and the vocabulary `["environment", "work"]`

#### Scenario: A seat that reports a blocker outside the vocabulary

- **WHEN** a seat reports `blocked` with `inputs.blocked_by: "signing"` under a rule that reads `blocked_by_in`
- **THEN** the machine refuses to rule, and the run parks naming the value and the vocabulary

### Requirement: An environment block parks for the operator, and a work block routes as before

Every shipped table that rules a blocked implementer SHALL park an `environment` block with `IMPL-BLOCKED-ENVIRONMENT`, and SHALL stop a `work` block, and a block that names no blocker, hard at `IMPL-BLOCKED`. The park SHALL carry the seat's non-empty notes as its reason. The operator's `retry` followed by `resume` SHALL run that same seat again, with the earlier phases' results in its context, entering no earlier phase and spending none of the seat's attempts.

#### Scenario: An unreachable signing key

- **WHEN** the implementer reports `blocked` with `inputs.blocked_by: "environment"` and notes naming the unreachable signing agent
- **THEN** the run is `awaiting_operator` with the park reason `IMPL-BLOCKED-ENVIRONMENT for (implement, blocked): <the notes>`
- **AND WHEN** the operator retries and resumes it
- **THEN** the implementer runs again with intake's result in `context.results`, intake is not entered again, and the run completes under `max_attempts` 1

#### Scenario: A contradictory framing

- **WHEN** the implementer reports `blocked` with `inputs.blocked_by: "work"`, or with no `blocked_by`
- **THEN** the run is `stopped` at `IMPL-BLOCKED`, as it was before the blocker existed

## Provenance

- `2026-10-20-789-environment-blocks-park` — folded 2026-10-20
