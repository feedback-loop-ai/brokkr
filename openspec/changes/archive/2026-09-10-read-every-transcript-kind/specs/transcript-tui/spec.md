## Purpose

Make each retained Claude, Codex and DSH transcript readable through the
existing terminal transcript pane and its two reading doors, with the same
identity, content, limits and full-session information as the transcript CLI.

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
row counts and only observed source-cap notice; version-zero event/storage
refusal SHALL keep R14's complete-prefix counts. A positive diagnostic count
or source-cap flag
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

#### Scenario: Claude discovery refusals retain the shared hint
- **WHEN** a valid Claude reference has duplicate qualifying files, an exhausted discovery bound or only a symlink candidate below its canonical home
- **THEN** the TUI shows `ambiguous-source`, `discovery-limit` or `unsafe-path` respectively beside its common transcript fact and shared Claude full-session line, clears previous turns and offers no reading door into a former candidate

#### Scenario: Legacy Codex eligibility agrees across participant surfaces
- **WHEN** a pre-0032 Codex participant has only a legacy session id and no common reference
- **THEN** the TUI shows `no-reference` with no full-session line or active reading door, agreeing with the command and browser participant presentation even if a direct id-only browser lookup could find an unrelated Claude file

### Requirement: Notices survive every reading surface

The shared `transcript truncated (size cap)` notice SHALL appear in the pane,
open-whole overlay, open-turn overlay and text command whenever the source
projection was truncated, including a selected turn that itself fit in full.
The JSON document SHALL carry the same notice in `notices` and
`truncated: true`. A readable truncated zero-turn projection SHALL show the
notice and allow its whole-transcript explanation to be opened. An unavailable DSH
snapshot SHALL show any retained source-cap/count notices in its pane beside
the refusal/path/hint, with both reading doors disabled; notices SHALL not
reopen an overlay or expose the previous snapshot. A malformed-line notice
SHALL likewise accompany the shared `skipped_lines` count, and the shared
unrecognized-record notice SHALL accompany a positive `unrecognized_records`
count, in the reader's fixed notice order without exposing unknown or malformed
payloads. A missing source SHALL say unavailable rather than truncated or empty.

Notices SHALL be independent of provider-specific hints. The truncation
notice SHALL be exactly `transcript truncated (size cap)` for Claude, Codex
and DSH. Claude's shipped ` — claude --resume carries the rest` suffix is
retired; its separate full-session line retains the convenience. No suffix
or provider command SHALL be added to the shared notice.
The full-session location, when known, SHALL remain available beside the
bounded readout so the operator can inspect the retained original separately.

#### Scenario: The whole door cannot hide the cap
- **WHEN** a transcript of any readable kind exceeds a shared source or display budget
- **THEN** the pane and whole-transcript overlay both show the truncation notice and the appropriate full-session information

#### Scenario: Claude and Codex use the identical notice
- **WHEN** a readable Claude transcript and a readable Codex transcript are each truncated
- **THEN** each pane and each reading door prints exactly `transcript truncated (size cap)`, matching text/JSON; Claude's separate full-session line still names `claude --resume <id>`, and no notice contains the old suffix

#### Scenario: Unknown content remains distinguishable in the pane and doors
- **WHEN** a readable file contains only recognized metadata and three unrecognized records, with top-level `ignorable: true` on unknown DSH events
- **THEN** the pane shows `no readable turns` and `unrecognized transcript records: 3`, and Enter opens the same explanation with the available full-session information

#### Scenario: A selected readable turn retains the unknown-record notice
- **WHEN** a file contains readable turns and unrecognized records and the operator opens a selected turn
- **THEN** the selected overlay, whole overlay and pane carry the same unrecognized-record count notice as CLI selection, without including unknown payloads

#### Scenario: Known Claude omissions do not invent an unknown-record notice
- **WHEN** the selected Claude source is the shipped projection fixture identified in `transcript-reading` and the operator opens its second turn or the whole transcript
- **THEN** the pane and both doors retain the shared two-turn projection or selected turn as appropriate and show only `malformed transcript lines skipped: 1`; `unrecognized_records` remains zero despite its omitted summary, absent message, blank/missing text, tool arguments and thinking

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
DSH first-record header SHALL be retried; Codex SHALL not wait for any
header after its filename establishes eligibility. A grown file SHALL be
re-derived under the shared caps, including event-only Codex content and
unassembled readable DSH chunks. A refresh SHALL replace the previous bounded projection, not append
its turns. Canonical replacements SHALL remove only associated fallbacks,
using the reading capability's snapshot ordering. DSH SHALL decode complete
packed rows and suppress only citation-proved chunks of the assembly's same
turn/step; absent, empty or partial citations SHALL leave uncited fragments
visible. Refresh SHALL replace a formerly readable snapshot with an
`unsupported-format` result atomically before presenting any new indices,
including when the unknown required event was appended without a journal
change or the opening header's version changed without a file-length change.
Every refresh SHALL re-evaluate DSH ownership and opening-header admission;
adding a second root of another version SHALL produce the shared ambiguity
state, never a preference for the supported format. Active refresh and
final/manual reads SHALL continue to recheck that reference under the same
bounds after refusal. If replacement removes,
changes or reorders any previously displayed turn, the turn selection SHALL
be cleared before the new projection is displayed, even when the file only
grew. Any open transcript overlay SHALL close before displaying a projection
that replaced, removed or reordered an earlier turn; it SHALL not silently
retarget a selection to a different turn. The first file appearance SHALL
become visible without leaving and re-entering the participant view. File shrinkage, disappearance, reference
replacement or participant/run/realm selection changes SHALL invalidate
stale content and turn selection before a new result is displayed.

The refresh identity SHALL include the selected run and participant plus
kind, home and locator, not only an id that two providers can share. A final
refresh SHALL occur when the participant concludes; automatic file-growth
polling then stops as today. Explicit refresh SHALL still re-resolve a
concluded participant's reference. Every refresh SHALL preserve bounded
reads and read-only journal/provider access.

#### Scenario: An announced thread whose rollout arrives late is readable
- **WHEN** a working Codex seat has journaled its reference before its rollout exists, and a unique safe filename-matching file appears with recognized readable content but no session header, without another journal event
- **THEN** a subsequent existing TUI refresh finds and displays it without a navigation round trip or waiting for a header record

#### Scenario: DSH appends prose between checkpoints
- **WHEN** the selected DSH session gains a complete assembled message while the journal head stays the same
- **THEN** a subsequent active-seat refresh adds the projected turn exactly once and preserves source order

#### Scenario: Assembly replaces chunks while the journal is unchanged
- **WHEN** a live DSH step first appends two readable chunks and a later refresh sees its complete assembled message citing both chunk sequences in that same turn/step without a journal-head change
- **THEN** the first refresh shows both chunk turns and both reading doors can open them; the later refresh replaces them with the single assembled turn, clears any prior turn selection and closes an open transcript overlay before showing the replacement

#### Scenario: Codex canonical arrival replaces only its associated fallback
- **WHEN** a live Codex snapshot shows an event-only assistant ruling and the grown file later includes its canonical item plus another unassociated content event
- **THEN** refresh displays the canonical ruling once at its source position, retains the other event, and clears the old turn selection before displaying the new indices

#### Scenario: A replacement reference cannot keep a stale cursor
- **WHEN** the selected participant's retry records a different kind, home or locator, or its source shrinks or disappears
- **THEN** the previous content and turn selection are invalidated, and the new bounded read or specific unavailable state is displayed

#### Scenario: Concluded sessions remain manually refreshable
- **WHEN** a working participant concludes and the operator later requests an explicit refresh
- **THEN** the conclusion received a final transcript read, idle automatic growth polling has stopped and the explicit refresh rechecks the same recorded identity

#### Scenario: A required unknown DSH append clears both reading doors
- **WHEN** a readable working DSH participant has a selected turn or open overlay and a refresh observes a new unknown required event in the bounded source without a journal-head change
- **THEN** that refresh atomically clears all turns and the selection, closes the overlay and shows `unsupported-format`, `DSH transcript format is not supported`, the confirmed path/hint and shared counts/notices; neither door reveals the previous prose, and subsequent active/final/manual refreshes remain bounded reads of the same reference

#### Scenario: An ignorable DSH append retains readable content and its notice
- **WHEN** the same readable source instead appends an unknown event with top-level `ignorable: true`
- **THEN** refresh retains its recognized turns and shows the increased physical-row unrecognized count in the pane and both doors, without treating the safely omitted event as semantic refusal

#### Scenario: Ranged partial assembly invalidates old indices without losing uncited chunks
- **WHEN** a working DSH source has displayed chunks 10, 11, 12 and 14, packed or ordinary, and later appends a same-step assembly citing `[[10, 12]]`
- **THEN** refresh retains uncited chunk 14 as turn one and the assembly as turn two, clears any old selection and closes its overlay before showing those indices; CLI selection for the new snapshot yields the same turns

#### Scenario: An absent assembly citation preserves earlier fragments
- **WHEN** a readable DSH source appends an assembly with absent or empty citations and changes no earlier event
- **THEN** refresh keeps every existing fragment and adds the assembly at its own position, preserving a prior selection under the unchanged-prefix rule; it does not treat the assembly's shared step as permission to remove fragments

#### Scenario: Refused storage cannot reopen a capped empty explanation
- **WHEN** a DSH snapshot has a source-cap flag and a complete invalid packed row or required unknown event within the prefix
- **THEN** the pane shows the shared `unsupported-format` refusal, source-cap notice and physical-row counts with its confirmed path/hint, but no turn or whole-session door is active; it does not offer the readable-zero-turn behavior reserved for a successful capped projection

#### Scenario: DSH header-version refusal clears and later recovers the pane
- **WHEN** a working DSH participant has readable turns and an open overlay, and an external file update changes its owned opening header from version 0 to version 1 without changing file length or the journal head
- **THEN** the next refresh clears all turns and selection, closes both doors and shows `unsupported-format`, `DSH transcript format is not supported`, the confirmed path/hint, zero counts and only an observed source-cap notice; it does not reuse the earlier version admission or treat the result as a readable empty projection
- **AND** if a subsequent bounded refresh sees the unique file restored externally to an admissible version-zero snapshot, the pane derives its current turns afresh and the existing doors become usable without reviving the old selection or overlay; no provider invocation or journal write occurs

#### Scenario: A foreign-version DSH root cannot leave a stale supported pane
- **WHEN** a working DSH participant's unique version-zero file was readable but an additional safe depth-zero version-one candidate appears before a refresh, with discovery otherwise successful
- **THEN** the pane changes to `ambiguous-source`, keeps the common reference with null path/hint, zero counts and no truncation/notices, clears all old turns and selection and closes both doors; it cannot retain the previous file by preferring its supported version

## Decisions

### T1 / clarifications 1–2 — Growth can replace displayed turns

A file can grow while its new assembled/canonical records replace earlier
fallbacks. Refreshing only by appending turns would duplicate content; keeping
the cursor on its old index could open a different message. Re-derive the
bounded snapshot and clear turn selection on replacement. Ordinary appends
that leave existing turns unchanged keep their navigation behavior. These
rules do not alter the journal's checkpoint selection or provider accounting.

### T2 / clarifications 3–6 — Render the reader's facts

`transcript-reading` owns full-session values and notices; the TUI renders
them in the pane and doors. Permissive unresolved hints and Claude-only
truncation suffixes are removed. A readable zero-turn file with safely
omittable unsupported records has an explanation the operator can open, not an apparently empty session. R14/T5
now distinguish a refused DSH snapshot: it retains diagnostics in the pane
and offers no reading door.
The explicit 0055 amendment in proposal S2 is required for the additional
Codex convenience; this rendering authorizes no provider execution.

### T3 / second-pass clarifications 1–3 — Compatibility refusals and omissions stay shared

A known Claude omission is not an unrecognized record: the pane and both
doors use the reading capability's exact fixture counts, including after
turn selection. Conversely a new Claude kind keeps the existing shared
unknown-record notice. The TUI accepts the newly declared breaking lookup
restrictions instead of preserving the old first candidate or stale prose.
Its precise reason and Claude hint agree with browser participant
presentation, whose direct id-only compatibility routes remain separately
scoped. All these values come from the reader; the TUI adds no classification.

### T4 / third-pass clarifications 1–3 — No extra TUI eligibility gate

R12/R13 own Codex filename identity and its id language. The TUI neither
waits for an undefined header nor applies Claude's guard to a Codex hint;
late file appearance remains readable on an existing refresh opportunity.
C5 preserves rejected references as JSON facts, which does not reopen any
reading door or authorize legacy fallback. The new scenarios make these
consequences visible while preserving T1–T3's navigation, diagnostic,
compatibility and stale-content rules. Proposed 0055 must bind these shared
results to TUI and browser participant regression tests, without a provider
execution or change to #226's launch behavior.

### T5 / design return U1 — Refusal invalidates content even when the path survives

Adopt R14/C6. A retained path or positive count is metadata, not permission to
keep old prose. Atomically replace a readable snapshot with the shared
unsupported-format state, clear selection and close both doors. Preserve the
path/hint and source diagnostics in the pane, including source truncation,
without confusing a refused projection with a readable empty one. An
ignorable unknown event keeps the existing readable notice behavior. Proposed
0055 must bind these distinctions to refresh and overlay-state tests.

### T6 / design return U2 — Navigation follows decoded and cited content

Adopt the exact same logical-event sequence as the command, including each
packed fragment's timestamp. Reject row-level concatenation and step-wide
replacement: both would make the terminal select different evidence from
R15/C7. Positive citations remove only their proven chunks; any such change
still invokes T1's selection/overlay invalidation. Empty or absent citations
can be a pure append and cannot retarget a cursor by deleting earlier turns.
Proposed 0055 must bind packed/ordinary equivalence, ranged partial assembly,
refusal diagnostics and existing cap behavior to the shared TUI/CLI tests.

### T7 / returned clarification — A remembered version cannot admit a new snapshot

Adopt R16/C8 without a TUI-specific format gate. Recheck opening-header
admission on every bounded refresh, even when the reference, source length
and journal head did not change. Header refusal retains the known location
with zero counts; ambiguous ownership removes that location. Both states
clear all stale prose and reading doors. A later admitted snapshot recovers
through ordinary fresh projection, not by restoring a cached selection.
Proposed 0055 must bind the same-size version transition, source-only notice,
recovery and current/foreign-candidate cases to the existing refresh and
overlay-state suite. These reads execute no provider or repair operation.
