## MODIFIED Requirements

### Requirement: Full-session information is truthful for its kind

The TUI SHALL render the shared `full_session` value specified by
`transcript-reading`, using exactly its kind/resolution table. The TUI SHALL
not independently choose a command or omit a non-null hint. A null value
SHALL render no convenience line; the common reference and specific
unavailability explanation SHALL remain visible. In particular, a valid
unresolved Codex reference always has the reader's explicit
`rollout unavailable` form with its recorded home, while unresolved DSH has no
hint.

The line SHALL be sanitized for terminal display, with paths/homes remaining
literal information rather than shell syntax. Displaying or opening it SHALL
execute nothing and SHALL not claim that live resumption, credentials or
sandbox re-imposition have been verified by reading a file. Proposed 0055
SHALL carry the explicit replacement of decision 0032 ruling 4's
command-construction binding stated in proposal S2; accepted decisions and
#226's independently owned launch behavior are not edited here.

The whole-transcript overlay SHALL include full-session information so a
long path clipped in the participant header remains readable. The CLI JSON
`full_session` SHALL carry the same informational text or null. The common
`transcript` fact SHALL remain visible independently of the convenience line.

#### Scenario: Each provider names only its own full session
- **WHEN** a readable Claude, Codex or DSH participant is selected
- **THEN** Claude shows its resume command, Codex shows its actual rollout path and Codex resume spelling with recorded home, and DSH shows its actual session file without a command

#### Scenario: A missing Codex rollout has no invented date
- **WHEN** a valid Codex reference has no local matching file
- **THEN** its common reference remains visible, the reader's non-null full-session line says the rollout is unavailable and names its Codex command and recorded home, and no guessed date path or Claude command is printed

#### Scenario: Codex hints do not inherit the Claude id guard
- **WHEN** a selected common Codex reference uses `0199mine` or a valid 65–80-character alphanumeric-or-dash thread id and its unique safe filename-matching rollout is readable without a session header
- **THEN** the TUI shows the shared confirmed-path Codex hint and its projected turns, allowing both doors for retained turns and the whole explanation for zero turns; CLI JSON and browser participant presentation retain that same hint, and the browser offers no Claude drill

#### Scenario: Rejected references stay visible without reading doors
- **WHEN** a selected common reference has an unsupported kind or missing home beside a stale legacy Claude id
- **THEN** the pane keeps its sanitized recorded reference and specific refusal, with no full-session hint, old turns or active reading door; JSON's preserved reference does not make the reference eligible for a TUI or browser read

#### Scenario: A shell fragment cannot form a convenience command
- **WHEN** a recorded id contains a semicolon, command substitution, leading hyphen or other invalid id syntax
- **THEN** neither TUI nor CLI constructs a resume command from it, and the read reports `invalid-reference`

#### Scenario: A displayed hint changes no execution boundary
- **WHEN** the operator opens the whole-transcript overlay of a Codex seat that ran with boxed hands
- **THEN** the hint is readable data only, no `codex` process starts, no sandbox setting changes and the recorded boundary remains the journal's original fact

#### Scenario: Agent-controlled path syntax stays inert in every TUI door
- **WHEN** a selected Codex or DSH source has a confirmed path or recorded home containing shell substitutions, backticks, quotes, whitespace or operator characters
- **THEN** the pane and whole-transcript overlay render the identical shared portable-display hint required by `transcript-reading`, preserve its reversible escapes after terminal sanitization, and expose no raw path/home fragment as shell syntax; opening either door starts no command

#### Scenario: Claude discovery refusals retain the shared hint
- **WHEN** a valid Claude reference has duplicate qualifying files, an exhausted discovery bound or only a symlink candidate below its canonical home
- **THEN** the TUI shows `ambiguous-source`, `discovery-limit` or `unsafe-path` respectively beside its common transcript fact and shared Claude full-session line, clears previous turns and offers no reading door into a former candidate

#### Scenario: Legacy Codex eligibility agrees across participant surfaces
- **WHEN** a pre-0032 Codex participant has only a legacy session id and no common reference
- **THEN** the TUI shows `no-reference` with no full-session line or active reading door, agreeing with the command, browser participant presentation and the participant's browser body route, none of which looks a Claude file up by that bare id
