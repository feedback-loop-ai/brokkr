## ADDED Requirements

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
