## MODIFIED Requirements

### Requirement: The console offers only admitted commands, with a reason

The run's graph SHALL carry the control plane: Stop, Resume and Close, and Retry when Brokkr admits it, each named for its effect ("↻ Retry security", "■ Stop", "▶ Resume", "✓ Close"). A command Brokkr refuses SHALL be shown disabled with Brokkr's refusal as its reason; Retry SHALL be shown only when admitted. Pressing a command SHALL open a reason bar; the command SHALL be sent only with a reason, which is passed to the verb. Unavailable actions SHALL be listed, disabled, under "Not available yet", with their reasons.

#### Scenario: Retry without a reason

- **WHEN** the operator presses Retry and leaves the reason empty
- **THEN** nothing is sent, and the reason bar's send button stays disabled

#### Scenario: A refused command

- **WHEN** Brokkr refuses Resume on the open run
- **THEN** the Resume button is disabled and its reason is Brokkr's refusal
