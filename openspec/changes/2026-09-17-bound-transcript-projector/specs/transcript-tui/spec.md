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
as `brokkr transcript --turn`, using the final chunks defined by
`transcript-reading`'s packed coalescing rule rather than individual members.
Physical line numbers SHALL not become turn numbers.
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
- **WHEN** a readable DSH source contains three uncited nonempty text members in one packed row and three uncited nonempty reasoning members in another, within both budgets
- **THEN** the pane exposes two assistant turns under the shared coalescing rule; Enter on turn one opens the entire text chunk and Enter with no selection opens both chunks in order, agreeing with CLI whole and selected reads

#### Scenario: Structural truncation bounds the refreshed pane
- **WHEN** a working DSH seat's source grows to 10,000 ordinary one-byte tool-call turns with absent data
- **THEN** refresh receives exactly the shared 7,797 retained turns, shows the size-cap notice in the pane and both doors, and does not clone, compare or retain the excluded suffix in its transcript result


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
- **WHEN** a live DSH step first appends two readable ordinary chunk rows and a later refresh sees its complete assembled message citing both chunk sequences in that same turn/step without a journal-head change
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
- **WHEN** a working DSH source has readable chunk members 10, 11, 12 and 14, packed or ordinary, displayed under the shared coalescing rule, and later appends a same-step assembly citing `[[10, 12]]`
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

#### Scenario: Partial assembly replaces packed chunks without retaining stale text
- **WHEN** a working source initially shows a coalesced `abcd` packed chunk for member sequences 10 through 13, then appends a readable same-turn/step assembly citing `[11, 13]` without a journal-head change
- **THEN** refresh replaces the old chunk with the shared `a`, `c`, assembly projection, clears the old turn selection and closes its overlay before showing new indices; no cached `b` or `d` remains in either reading door
- **AND** CLI selection of that snapshot returns the same new turns and stamps
