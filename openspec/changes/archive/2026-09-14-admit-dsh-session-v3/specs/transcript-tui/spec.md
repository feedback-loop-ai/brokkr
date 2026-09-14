## MODIFIED Requirements

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
as `brokkr transcript --turn`, including separate readable logical members of
one DSH packed row. Physical line numbers SHALL not become turn numbers.
Preview clipping due to terminal space SHALL not remove content from either reading door. Transcript selection SHALL not
change the journal checkpoint selection or reinterpret the seat's accounting.
A participant with an unavailable reference or source SHALL show the common
transcript fact and its specific unavailability explanation, with no stale
prose or active reading door. In particular, DSH `unsupported-format` SHALL
show the shared `DSH transcript format is not supported` explanation, retained
path/hint and source diagnostics, while clearing every old turn and closing
any open transcript overlay. Header-version refusal SHALL use R16's zero
row counts and only observed source-cap notice; event/storage refusal under
any admitted version SHALL keep R14's complete-prefix counts. A positive
diagnostic count or source-cap flag
SHALL not make a refused projection readable. A readable zero-turn result
SHALL still permit opening the whole transcript's empty/capped explanation
and any malformed-line or unrecognized-record notices.

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

#### Scenario: A packed row is readable through individual turns and the whole door
- **WHEN** a readable DSH source contains three text fragments in one packed row and three reasoning fragments in another, within both budgets
- **THEN** the pane exposes six separate assistant turns with the shared reconstructed stamps; Enter on turn three opens only its third text fragment and Enter with no selection opens all six in member order, agreeing with CLI whole and selected reads
