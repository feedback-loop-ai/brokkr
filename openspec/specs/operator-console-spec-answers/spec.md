# operator-console-spec-answers Specification

## Purpose
Let the operator answer a clarify park through the run's own spec: the
console opens the run's worktree with the clarifier's questions as a draft
and writes nothing on the operator's behalf.

## Requirements

### Requirement: "Open the spec" opens the run's worktree with the questions

On a clarify card, "Open the spec" SHALL open a DSH session in the run's worktree, open the Files tab rooted there, and prefill, without sending, the clarifier's questions with "answer these in the spec, then Retry clarify". It SHALL write nothing. Without a worktree in the view it SHALL be disabled with "no worktree recorded for this run".

#### Scenario: A clarify park with a worktree

- **WHEN** the operator presses Open the spec on a run parked by clarify with worktree `/w/brokkr`
- **THEN** a session opens in `/w/brokkr`, the Files tab shows that tree, and the composer holds the questions unsent

#### Scenario: No worktree

- **WHEN** the view names no worktree
- **THEN** Open the spec is disabled with "no worktree recorded for this run"

## Provenance

- `2026-10-14-568-operator-console-slice-two` — folded 2026-10-14
