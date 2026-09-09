## Purpose

Expose the same local transcript the TUI reads through a scriptable command,
with unambiguous run, participant and turn selection, safe terminal text,
versioned JSON and explicit failures when retained content is unavailable.

## ADDED Requirements

### Requirement: The transcript command selects one run and participant

`brokkr transcript` SHALL require `--run <selector>` and
`--seat <label-or-participant-key>`, accept optional `--turn <n>` and `--json`,
and support `--realms` and `--db` with the same read-only journal resolution
and precedence as `inspect`. Run selection SHALL use the existing full-id,
unique-prefix and `latest` semantics, including refusal of ambiguous matches
across realms. The command SHALL NOT create a missing journal.

Within the selected run, an exact participant key SHALL win. Otherwise an
exact participant label SHALL select only when unique. Prefix or fuzzy seat
matching SHALL not occur. An unknown seat SHALL fail; a label shared by
multiple phase visits SHALL fail with the matching participant keys so the
operator can choose. A panel/sequence parent SHALL read only its own common
reference if any; it SHALL not aggregate or borrow a member transcript.
Selecting a leaf key SHALL select only that member/step's reference. The
command SHALL not launch, retry or resume a run or provider.

#### Scenario: The operator reads a uniquely named seat
- **WHEN** `brokkr transcript --run latest --seat review:chief` identifies one run and one participant label
- **THEN** the command resolves that participant's recorded transcript and renders its retained turns without a provider invocation or journal write

#### Scenario: A repeated visit requires the participant key
- **WHEN** a run has two participants labeled `implement` from different effect visits and the operator passes `--seat implement`
- **THEN** the command fails, lists both participant keys and displays neither transcript; passing either exact key selects that visit

#### Scenario: A parent does not inherit its child's prose
- **WHEN** a panel parent has no transcript but a member has one
- **THEN** selecting the parent reports `no-reference`, while selecting the member's exact key reads that member's file

#### Scenario: Run selection follows the existing world
- **WHEN** the same run prefix matches several realm journals, or `--db` is provided beside a realms map
- **THEN** ambiguous cross-realm matches fail by the existing resolver's rule, and an explicit database has the same precedence it has for inspect

#### Scenario: Missing arguments and databases write nothing
- **WHEN** a required selector is omitted or the selected journal does not exist
- **THEN** the command fails without creating a database, a run or a transcript directory

### Requirement: Turn selection addresses the displayed sequence

Omitting `--turn` SHALL return the complete bounded projection. `--turn`
SHALL accept a positive unsigned 64-bit integer and refer to the one-based
position in the same projected sequence the TUI displays. The projection,
malformed-line count and both size caps SHALL be evaluated before selection.
A retained selection SHALL return one unchanged `Turn`, retain its original
position in text output and preserve the source's truncation and malformed
line notices. It SHALL NOT count only assistant messages, checkpoint turns,
provider steps or raw JSONL lines.

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

#### Scenario: Invalid or absent turns fail explicitly
- **WHEN** the operator passes turn zero, a negative or non-integer turn, or a positive index beyond a complete two-turn file
- **THEN** invalid syntax is a usage error, and the valid out-of-range index returns `turn-not-retained` without returning the last available turn instead

### Requirement: JSON exposes a distinct local transcript document

`--json` SHALL emit one JSON object with `schema: "brokkr.transcript/v1"`
and the following members, present even when null or empty:

| Member | Meaning |
|---|---|
| `run_id` | Resolved full run identifier. |
| `seat` | Resolved participant key, never only its possibly repeated label. |
| `transcript` | Effective `{kind, locator, home}` reference, or null when no reference can be derived. |
| `legacy` | Boolean indicating a synthesized legacy Claude reference. |
| `path` | Confirmed local source path, or null when no owned file was resolved. |
| `turn` | Requested one-based index, or null for the whole transcript. |
| `turns` | Ordered shared `Turn` values, filtered to one when requested. |
| `truncated` | Boolean for source/display truncation before turn selection. |
| `skipped_lines` | Count of complete malformed JSON lines encountered in the bounded read. |
| `notices` | Strings for truncation and skipped lines; truncation uses the shared notice verbatim. |
| `unavailable` | Null on a readable result, otherwise the reason token defined below. |
| `full_session` | Kind-specific informational line from `transcript-tui`, or null when no safe hint can be formed. |

This document SHALL be an explicit local read result, not a journal record,
export format, or addition of prose to `RunView`. It SHALL NOT change the
existing inspect/seats JSON shape or view version solely to introduce this
new command. JSON strings SHALL retain content using JSON escaping, without
ANSI decoration, terminal control execution or prose scraped from rendered
text. Structural changes to this new public document SHALL require an
explicit version change in its schema identifier.

#### Scenario: Scripts receive the selected identity and shared turns
- **WHEN** `brokkr transcript --run <prefix> --seat <label> --json` resolves a readable reference
- **THEN** one JSON document contains the full run id, exact participant key, common reference, confirmed path, shared turn objects and explicit truncation state under `brokkr.transcript/v1`

#### Scenario: JSON distinguishes an empty transcript from unavailability
- **WHEN** one read resolves an empty valid file and another resolves a missing file
- **THEN** both carry an empty `turns` array, but the first has `unavailable: null` and the second `unavailable: "not-found"` with no fabricated source path

#### Scenario: Legacy synthesis is visible
- **WHEN** the command uses the guarded pre-0032 Claude fallback
- **THEN** `legacy` is true and `transcript` carries the effective Claude reference used for that lookup; common-reference reads have `legacy: false`

#### Scenario: Terminal controls survive only as JSON data
- **WHEN** a turn contains an escape sequence, quotes and newline characters
- **THEN** the JSON parses back to that turn's content using JSON escapes, with no injected terminal formatting or second JSON document

### Requirement: Text output and errors report the same bounded result

Default text output SHALL identify the run, participant, transcript kind
and confirmed source when available, then render one-based turn numbers,
roles, recorded stamps and every retained block in order. It SHALL use the
same kind-specific full-session hint and notices as the TUI. A valid empty
projection SHALL say `no readable turns`; a truncated zero-turn projection
SHALL say it was truncated instead of implying that the session was empty.
Terminal control characters SHALL be sanitized in content, references,
paths, hints and diagnostics. Tool arguments/results SHALL remain readable
text and SHALL never be executed.

A readable result, including empty, skipped-line or truncated results, SHALL
exit zero unless a requested turn was not retained. Once run and participant
are selected, unavailability SHALL be one of `no-reference`, `none`,
`unsupported-kind`, `unannounced`, `missing-home`, `invalid-reference`,
`unsafe-path`, `not-found`, `ambiguous-source`, `discovery-limit`,
`unreadable`, or `turn-not-retained`. It SHALL exit one, carry no transcript
turns, and provide a sanitized explanation on stderr. With `--json` it SHALL
also emit the document above with the corresponding reason on stdout;
without `--json` the unavailable read SHALL emit no transcript body on stdout.
Known path and cap/skip facts SHALL remain available in an error document.

Reference failure precedence SHALL be: absent/ineligible reference,
`none`, unsupported kind, empty locator, missing home, malformed reference,
then discovery/read failure. Unsafe candidate paths SHALL return
`unsafe-path` when no safe unique source is established, and a discovery
limit SHALL take precedence over a provisional unique candidate. The reader
SHALL report `unreadable` if an I/O failure prevents establishing a unique
answer. `turn-not-retained` SHALL apply only after a readable projection.
Usage, run selection and participant selection failures occur before a
transcript document exists: they SHALL exit nonzero with safe stderr and
empty stdout, including when `--json` was requested.

#### Scenario: Text and JSON agree on a missing local transcript
- **WHEN** a selected participant records a valid kind/id/home but the retained file is missing
- **THEN** either command mode exits one with a `not-found` explanation, and JSON mode emits an unavailable document rather than an empty successful transcript

#### Scenario: Explicit none is not an error in discovery
- **WHEN** the selected participant's common reference has kind `none` and empty locator/home
- **THEN** the command exits one with reason `none`, without searching a provider home or reporting the empty locator as an invented session

#### Scenario: Malformed lines and truncation do not turn readable content into failure
- **WHEN** a readable file has a skipped malformed line and a valid prefix exceeding a size budget
- **THEN** whole-transcript text and JSON both exit zero, expose the retained prefix and carry the malformed-line and truncation notices

#### Scenario: Unknown selection cannot fall back to another seat
- **WHEN** `--seat` matches no participant or is an ambiguous label
- **THEN** either output mode fails before file lookup, leaves stdout empty and names the selection problem on stderr without reading another seat's prose
