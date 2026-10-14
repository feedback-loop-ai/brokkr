# operator-console-commands Specification

## Purpose
Let the operator console offer only the operator commands Brokkr would
admit, carry each press with its id and the head the operator saw, and
paint Brokkr's own outcome, refusal or reason for anything not available.

## Requirements

### Requirement: The view model names the commands Brokkr would admit

`RunView` SHALL carry the operator commands for the run as folded: retry
(with the phase it re-runs), stop, resume and conclude. Each SHALL be marked
admitted, or refused with the reason the verb would give, using the same
rules the verbs apply. `inspect --json` SHALL emit it.

#### Scenario: A parked run

- **WHEN** a run awaits the operator at security review
- **THEN** retry {security review} and stop are admitted, and resume is refused until a command is accepted

#### Scenario: A running run

- **WHEN** a run is running
- **THEN** retry is refused because the run is not awaiting the operator, and stop is admitted

### Requirement: The view model names the actions that are not available

`RunView` SHALL list the actions the console shows as disabled, each with a
fixed reason:

| Action | Shown when | Reason |
|---|---|---|
| `dismiss_finding` | an open residual finding exists | "Brokkr only lets you dismiss findings once a run has finished." |
| `talk_to_reviewer` | the run is parked at a review or gate step | "Reviews start fresh each time; your note can't reach the reviewer yet." |
| `hold` | the run is parked | "A parked run already waits for you; it holds nothing else back." |
| `answer_here` | the run carries clarify questions | "Not yet: Brokkr has no way to pass an answer to a run directly." |

#### Scenario: Finding on a parked run

- **WHEN** a parked run has a serious residual finding
- **THEN** `dismiss_finding` is listed with its reason, and no command for it exists

### Requirement: The console offers only admitted commands, with a reason

The decision card SHALL show the admitted commands as buttons named for
their effect, such as "Retry security review", "Stop the run", "Resume" and
"Close it". Each SHALL require a reason, which is passed to the verb. Refused
commands SHALL NOT be shown as buttons. Unavailable actions SHALL be shown
disabled, under "Not available yet", with their reasons.

#### Scenario: Retry without a reason

- **WHEN** the operator presses Retry with the reason field empty
- **THEN** nothing is sent, and the field asks for a reason

### Requirement: A command is idempotent and fenced by the head it was chosen at

Each command SHALL carry a command id and the run head the operator saw.

- If the head moved, the host SHALL answer "not sent", append nothing, and
  return the fresh view.
- A repeated command id SHALL return the first outcome.
- An unknown outcome SHALL be reconciled from the run's record before
  anything is resent.
- A retry SHALL be followed by resume.
- The card SHALL show one of four outcomes: done, not sent, refused (with
  Brokkr's text verbatim), or checking.

#### Scenario: The run changed first

- **WHEN** the head moved after the operator opened the card
- **THEN** the card shows "not sent: the run changed after you opened it" and the current state

#### Scenario: Close a run that is still driven

- **WHEN** conclude is refused because the journal moved beneath it
- **THEN** the card shows "refused" with Brokkr's refusal text, and nothing is retried automatically

## Provenance

- `2026-10-13-568-operator-console-slice-one` — folded 2026-10-14
