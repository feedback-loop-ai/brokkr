## MODIFIED Requirements

### Requirement: Turn selection addresses the displayed sequence

Omitting `--turn` SHALL return the complete bounded projection. `--turn`
SHALL accept a positive unsigned 64-bit integer and refer to the one-based
position in the same projected sequence the TUI displays. For a readable
source, the projection, malformed-line and unrecognized-record counts and
both size caps SHALL be evaluated before selection. DSH header-version
admission, storage decoding and semantic refusal SHALL also precede selection:
`unsupported-format` SHALL return no turns for every
valid requested index, never `turn-not-retained` or an earlier readable turn.
A retained selection SHALL return one unchanged `Turn`, retain its original
position in text output and preserve all of the source's notices, including
truncation, malformed lines and unrecognized records. It SHALL NOT count only
assistant messages, checkpoint turns,
provider steps or raw JSONL lines. DSH packed members SHALL receive indices
only through the final chunks defined by `transcript-reading`'s coalescing
rule; the command SHALL not assign an index to each underlying member or
undo coalescing for selection. Equivalent packed and ordinary encodings can
therefore have different turn counts while retaining the same content and
citation outcomes. Indices address one bounded snapshot;
they are not durable message ids across refreshes. If canonical content
replaces fallback events/chunks, numbering SHALL be recomputed from the new
source-ordered projection, exactly as for the TUI.

Zero, negative, nonnumeric and overflowing turn arguments SHALL be usage
errors. A positive index beyond the retained turns SHALL return
`turn-not-retained`; when truncated, the diagnostic SHALL say the bounded
projection does not establish whether that turn exists later in the source.
The command SHALL not scan past the caps to satisfy a turn request.

#### Scenario: Selecting one turn agrees with the whole transcript
- **WHEN** a projected sequence contains user text, assistant reasoning, a tool call, a tool result and an answer, and the operator passes `--turn 4`
- **THEN** only the fourth displayed turn, the tool result, is returned with exactly the blocks present at that position in the whole-transcript read

#### Scenario: Source line numbers are not turn numbers
- **WHEN** metadata, skipped malformed lines and mirrored streaming events precede the first projected user message
- **THEN** `--turn 1` selects that user message, not a raw record or a journal accounting turn

#### Scenario: Selecting a retained turn keeps the cap notice
- **WHEN** the full projection is truncated after three retained turns and the operator selects turn two
- **THEN** the selected turn is complete, its original number is shown, and the result still carries the whole source's truncation notice

#### Scenario: A turn beyond the cap is not claimed absent from the file
- **WHEN** the projection is truncated after three turns and the operator selects turn four
- **THEN** the command fails with `turn-not-retained`, retains `truncated: true` and states that the requested turn is outside the retained prefix

#### Scenario: Turn selection preserves unknown-record diagnostics
- **WHEN** a readable source has two retained turns and three unrecognized records, and the operator requests turn two
- **THEN** the unchanged second turn is returned with `unrecognized_records: 3` and `unrecognized transcript records: 3`, just as in the whole read

#### Scenario: Turn indices follow assembly within each snapshot
- **WHEN** two displayed DSH chunk turns are replaced by one assembled message citing both chunks in their same recorded turn/step on a subsequent read
- **THEN** CLI and TUI use the new projection's one-based indices, and a requested index that was present only in the previous snapshot receives `turn-not-retained` instead of stale content

#### Scenario: Invalid or absent turns fail explicitly
- **WHEN** the operator passes turn zero, a negative or non-integer turn, or a positive index beyond a complete two-turn file
- **THEN** invalid syntax is a usage error, and the valid out-of-range index returns `turn-not-retained` without returning the last available turn instead

#### Scenario: A packed row's members have separate selectable turns
- **WHEN** a packed row's readable members at sequences 10, 11 and 12 are followed by a same-step readable assembly citing only member 11, with both budgets satisfied
- **THEN** the suppression gap leaves two separate chunk turns: `--turn 1` returns member 10's chunk, `--turn 2` returns member 12's chunk and `--turn 3` returns the assembly; the unsplit case instead follows the next scenario

#### Scenario: Packed selection addresses coalesced chunks
- **WHEN** a version-zero DSH snapshot contains an uncited `text-chunks` row with `seq0: 10`, `time0: 1000`, `data.turn: 1`, `data.step: 1`, `data.index: 0`, `data.dt: [-1, 5]` and `data.texts: ["a", "b", "c"]`, followed by an ordinary user message containing `q` at time 2000, with both budgets satisfied
- **THEN** whole text and JSON output contain exactly two turns: assistant chunk `abc` stamped `"1000"` and user message `q` stamped `"2000"`; `--turn 1` and `--turn 2` return those unchanged turns, and the TUI has the same two indices
- **AND** `--turn 3` exits one with `turn-not-retained` and no turns, retaining `truncated: false`, zero malformed/unrecognized counts and no notices; a different refusal is not equivalent
- **AND** an otherwise identical snapshot using three ordinary chunk rows instead retains four turns: `a` at `"1000"`, `b` at `"999"`, `c` at `"1004"`, then `q` at `"2000"`; ordinary index two selects `b` and index four selects `q`, so packed and ordinary index two deliberately differ

#### Scenario: Partial citations recompute the displayed indices
- **WHEN** a complete DSH snapshot has chunks at sequences 10, 11, 12 and 14 followed by a readable same-step assembly citing `[[10, 12]]`
- **THEN** turn one is the uncited chunk 14, turn two is the assembly and `--turn 3` returns `turn-not-retained`; CLI and TUI share that sequence whether the chunks were packed or unpacked

#### Scenario: Turn selection cannot bypass DSH semantic refusal
- **WHEN** an owned DSH file has readable early content followed within the source cap by an unknown required event or structurally invalid packed row, and the operator requests the whole transcript, `--turn 1` or `--turn 999`
- **THEN** every request exits one with `unsupported-format`, no turns and the same source metadata/counts/notices; neither the early turn nor the out-of-range request changes the refusal, even if display capping would have stopped before the offending row

#### Scenario: Structural truncation survives whole and selected JSON reads
- **WHEN** a DSH snapshot projects 10,000 one-byte tool-call turns with absent data and the operator requests whole JSON output or `--turn 7797`
- **THEN** the whole document contains exactly 7,797 turns, the selection contains the unchanged turn at index 7,797, and both carry `truncated: true` and `transcript truncated (size cap)`
- **AND** `--turn 7798` returns `turn-not-retained` with truncation; selection never serializes or retains the over-budget suffix
