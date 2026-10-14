# operator-console-assistant Specification

## Purpose
Let DSH's model read Brokkr through read-only tools and suggest one command
Brokkr admits now, while only the operator presses commands; questions open
logged sessions whose answers cite the records they rest on.

## Requirements

### Requirement: The assistant reads and suggests; it never acts

The console's DSH model SHALL have read tools only: runs, inspect, queue and
transcript (through Brokkr's redaction). It MAY suggest one admitted command
for the open run. The suggestion SHALL render as a card naming the command,
with "only if the run hasn't changed since you read this", and a "Use this"
button. "Use this" fills the decision card. The operator still presses the
command.

#### Scenario: Suggesting a retry

- **WHEN** the operator asks whether a finding is real and the model recommends a fresh review
- **THEN** a "Suggested · Retry security review" card appears, and nothing is appended until the operator presses the command

#### Scenario: Suggesting something Brokkr lacks

- **WHEN** the model would recommend dismissing a finding on a parked run
- **THEN** no suggestion card is produced, and the answer states the action is not available yet and why

### Requirement: Two scopes, plain names

The all-realms panel SHALL be titled "Ask Brokkr" and read every realm. The
run panel SHALL be titled "Ask about this run" and read that run. Neither
SHALL be named Triage or Dispatcher.

#### Scenario: Why a queued run waits

- **WHEN** the operator asks in Ask Brokkr why a queued run is not starting
- **THEN** the answer cites the entry's admission verdict

### Requirement: Answers cite evidence and admit gaps

Every factual claim about a run SHALL link to the ruling, session, change or
record it came from. Missing data, such as a cost for dsh runs, SHALL be
stated as missing.

#### Scenario: No cost on record

- **WHEN** asked what a dsh retry would cost
- **THEN** the assistant says no cost is on record for dsh runs

### Requirement: DSH's model reads Brokkr through four read-only tools

The console SHALL register DSH model tools `brokkr_runs`, `brokkr_inspect`, `brokkr_queue` and `brokkr_transcript`, each running one Brokkr readout with the runner's bounds. Each SHALL refuse a run id or seat outside the command desk's grammar before starting a process. No tool SHALL run a verb that appends to a journal.

#### Scenario: A malformed run id

- **WHEN** the model calls `brokkr_inspect` with run `../etc`
- **THEN** the tool answers a refusal and no process starts

#### Scenario: A readout cut at its bound

- **WHEN** a readout exceeds the runner's byte bound
- **THEN** the tool's answer says it was cut

### Requirement: A suggestion is only for a command Brokkr admits now

`brokkr_suggest` SHALL answer with a suggestion card only when the named command is admitted for the run as read at that moment, carrying the head seq it was read at. Otherwise it SHALL answer with Brokkr's refusal or unavailable reason, and no card. The card SHALL read "Suggested · <command>", name the run by title, say "only if the run hasn't changed since you read this", and offer **Use this**, which fills the decision card. Nothing SHALL be appended until the operator presses the command.

#### Scenario: Suggesting a retry

- **WHEN** the model suggests retrying security review on a run parked there
- **THEN** a "Suggested · Retry security review" card appears, and the journal is unchanged until the operator presses Retry security review

#### Scenario: Suggesting something Brokkr lacks

- **WHEN** the model suggests dismissing a finding on a parked run
- **THEN** no card appears, and the answer gives the unavailable reason

### Requirement: Asking opens a logged session in the right directory

Ask Brokkr SHALL open a DSH session in the console's workspace. Ask about this run SHALL open one in the run's worktree when the view names it, else in the console's workspace. Each SHALL be prefilled with its scope, the tools and the instruction to cite records and state gaps, and then sent.

#### Scenario: Asking about a run

- **WHEN** the operator asks "why is this waiting?" in Ask about this run on a run whose view names worktree `/w/brokkr`
- **THEN** a session opens in `/w/brokkr` with the run's title and id in its prompt, and the question is sent

## Provenance

- `2026-10-13-568-operator-console-slice-one` — folded 2026-10-14
- `2026-10-14-568-operator-console-slice-two` — folded 2026-10-14
