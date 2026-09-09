## Purpose

Make each retained Claude, Codex and DSH transcript readable through the
existing terminal transcript pane and its two reading doors, with the same
identity, content, limits and full-session information as the transcript CLI.

## ADDED Requirements

### Requirement: Every readable kind reaches the pane and both doors

Selecting a participant in the TUI SHALL resolve its common reference using
`transcript-reading`, regardless of whether it is a Claude session, Codex
thread or DSH session. The transcript pane SHALL display the shared ordered
turns. Existing turn navigation, Enter to open a selected turn, Esc to clear
the turn selection, and Enter with no selected turn to open the whole
transcript SHALL retain their current interaction. The reading overlay SHALL
include every retained block of the selected turn or entire transcript,
including reasoning, tool arguments and output in the new projections, with
normal scrolling and the existing terminal sanitization.

The pane and overlay SHALL visibly number turns from one in the same sequence
as `brokkr transcript --turn`. Preview clipping due to terminal space SHALL
not remove content from either reading door. Transcript selection SHALL not
change the journal checkpoint selection or reinterpret the seat's accounting.
A participant with an unavailable reference SHALL show the common transcript
fact and its specific unavailability explanation, with no stale prose or
active door into another participant's content. A readable zero-turn result
SHALL still permit opening the whole transcript's empty/capped explanation.

#### Scenario: Codex content reaches each existing door
- **WHEN** a Codex participant's retained file contains user text, reasoning, a tool call, its output and an assistant ruling
- **THEN** its pane lists the projected turns; selecting the ruling and pressing Enter opens it in full, and clearing the selection and pressing Enter opens the whole bounded transcript

#### Scenario: DSH uses the same navigation
- **WHEN** a DSH participant's depth-zero session has multiple assembled messages and tool output
- **THEN** the same keys browse and open those turns without a Claude session id or a provider invocation

#### Scenario: A long turn is clipped only in the preview
- **WHEN** one retained turn exceeds the pane's visible height but remains within the shared budgets
- **THEN** the open-turn and open-whole overlays contain its entire retained content and allow it to be scrolled

#### Scenario: Terminal transcript indices agree with the CLI
- **WHEN** the TUI highlights displayed turn three of a stable transcript snapshot
- **THEN** `brokkr transcript --run <id> --seat <key> --turn 3` returns the same role, timestamp and ordered blocks

#### Scenario: An unavailable seat cannot reuse the previous seat's pane
- **WHEN** the operator moves from a readable Claude seat to an unavailable Codex, DSH or none-kind participant
- **THEN** the pane clears the previous turns, explains the selected reference's unavailability and neither reading door exposes the old seat's prose

### Requirement: Full-session information is truthful for its kind

The TUI and transcript command SHALL derive their informational full-session
line from the same validated reference and resolved local path:

| Kind | Information |
|---|---|
| `claude-session` | `full session: claude --resume <id>` for a valid reference, preserving the existing convenience. |
| `codex-thread` | `full session:` followed by the confirmed rollout path and `codex exec resume <thread>`; the recorded Codex home is also identified. |
| `dsh-session` | `full session:` followed by the confirmed depth-zero `session.jsonl` path; no resume command. |
| `none`, missing or invalid reference | No fabricated path or command. |

If a valid Claude or Codex reference exists but the source cannot be resolved,
the supported command spelling can remain informational, but a Codex line
SHALL explicitly say the rollout is unavailable and SHALL not invent a dated
path. DSH without a resolved file SHALL have no full-session hint beyond the
common reference and unavailability explanation. The new Codex/DSH behavior
SHALL never produce a Claude resume command.

Every command-bearing id SHALL pass the reading capability's id validation;
no raw locator, home or path SHALL be interpolated as shell syntax. Paths and
homes SHALL be presented as literal informational data with terminal control
characters sanitized, not as commands. Displaying or opening these lines
SHALL execute nothing. The hint SHALL not claim that live resumption,
credentials or sandbox re-imposition have been verified by reading a file.
The command syntax is grounded in decision 0030's recorded measurement and
must be checked against installed help/source during design when available;
no unmeasured flags, provider launch behavior or DSH resume syntax is added.

The whole-transcript overlay SHALL include full-session information so a
long path clipped in the participant header remains readable. The CLI JSON
`full_session` SHALL carry the same informational text or null. The common
`transcript` fact SHALL remain visible independently of the convenience line.

#### Scenario: Each provider names only its own full session
- **WHEN** a readable Claude, Codex or DSH participant is selected
- **THEN** Claude shows its resume command, Codex shows its actual rollout path and Codex resume spelling with recorded home, and DSH shows its actual session file without a command

#### Scenario: A missing Codex rollout has no invented date
- **WHEN** a valid Codex reference has no local matching file
- **THEN** its common reference remains visible, any Codex resume hint says the rollout is unavailable, and no guessed date path or Claude command is printed

#### Scenario: A shell fragment cannot form a convenience command
- **WHEN** a recorded id contains a semicolon, command substitution, leading hyphen or other invalid id syntax
- **THEN** neither TUI nor CLI constructs a resume command from it, and the read reports `invalid-reference`

#### Scenario: A displayed hint changes no execution boundary
- **WHEN** the operator opens the whole-transcript overlay of a Codex seat that ran with boxed hands
- **THEN** the hint is readable data only, no `codex` process starts, no sandbox setting changes and the recorded boundary remains the journal's original fact

### Requirement: Notices survive every reading surface

The shared `transcript truncated (size cap)` notice SHALL appear in the pane,
open-whole overlay, open-turn overlay and text command whenever the source
projection was truncated, including a selected turn that itself fit in full.
The JSON document SHALL carry the same notice in `notices` and
`truncated: true`. A truncated zero-turn projection SHALL show the notice and
allow its whole-transcript explanation to be opened. A malformed-line notice
SHALL likewise accompany the shared `skipped_lines` count, without exposing
the malformed payload. A missing source SHALL say unavailable rather than
truncated or empty.

Notices SHALL be independent of provider-specific hints. No universal notice
SHALL claim that `claude --resume` carries the rest of a Codex or DSH session.
The full-session location, when known, SHALL remain available beside the
bounded readout so the operator can inspect the retained original separately.

#### Scenario: The whole door cannot hide the cap
- **WHEN** a transcript of any readable kind exceeds a shared source or display budget
- **THEN** the pane and whole-transcript overlay both show the truncation notice and the appropriate full-session information

#### Scenario: The selected door cannot hide the source cap
- **WHEN** a retained turn is selected from a truncated transcript and opened
- **THEN** the complete retained turn and the source's truncation notice are both present, matching command output for that turn

#### Scenario: No retained turns still leaves a readable explanation
- **WHEN** an oversized first turn leaves an empty retained prefix
- **THEN** the pane says the transcript was truncated, Enter with no turn selection opens that explanation and any full-session hint, and the UI does not claim the transcript is missing

### Requirement: Live refresh follows the selected reference

While the selected participant is working, the TUI SHALL recheck local
transcript availability at its existing refresh opportunities even when the
journal sequence has not advanced. A not-yet-created file or incomplete
header SHALL be retried; a grown file SHALL be re-derived under the shared
caps. The first file appearance SHALL become visible without leaving and
re-entering the participant view. File shrinkage, disappearance, reference
replacement or participant/run/realm selection changes SHALL invalidate
stale content and turn selection before a new result is displayed.

The refresh identity SHALL include the selected run and participant plus
kind, home and locator, not only an id that two providers can share. A final
refresh SHALL occur when the participant concludes; automatic file-growth
polling then stops as today. Explicit refresh SHALL still re-resolve a
concluded participant's reference. Every refresh SHALL preserve bounded
reads and read-only journal/provider access.

#### Scenario: An announced thread whose rollout arrives late is readable
- **WHEN** a working Codex seat has journaled its reference before its rollout exists, and the valid file appears without another journal event
- **THEN** a subsequent existing TUI refresh finds and displays it without a navigation round trip

#### Scenario: DSH appends prose between checkpoints
- **WHEN** the selected DSH session gains a complete assembled message while the journal head stays the same
- **THEN** a subsequent active-seat refresh adds the projected turn exactly once and preserves source order

#### Scenario: A replacement reference cannot keep a stale cursor
- **WHEN** the selected participant's retry records a different kind, home or locator, or its source shrinks or disappears
- **THEN** the previous content and turn selection are invalidated, and the new bounded read or specific unavailable state is displayed

#### Scenario: Concluded sessions remain manually refreshable
- **WHEN** a working participant concludes and the operator later requests an explicit refresh
- **THEN** the conclusion received a final transcript read, idle automatic growth polling has stopped and the explicit refresh rechecks the same recorded identity
