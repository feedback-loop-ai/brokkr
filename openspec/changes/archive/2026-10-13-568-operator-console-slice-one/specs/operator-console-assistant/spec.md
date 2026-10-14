## ADDED Requirements

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
