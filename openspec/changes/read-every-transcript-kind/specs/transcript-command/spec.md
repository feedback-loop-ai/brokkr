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
malformed-line and unrecognized-record counts and both size caps SHALL be
evaluated before selection.
A retained selection SHALL return one unchanged `Turn`, retain its original
position in text output and preserve all of the source's notices, including
truncation, malformed lines and unrecognized records. It SHALL NOT count only
assistant messages, checkpoint turns,
provider steps or raw JSONL lines. Indices address one bounded snapshot;
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
- **WHEN** two displayed DSH chunk turns are replaced by one assembled message on a subsequent read
- **THEN** CLI and TUI use the new projection's one-based indices, and a requested index that was present only in the previous snapshot receives `turn-not-retained` instead of stale content

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
| `skipped_lines` | Nonnegative integer count of complete malformed JSON lines in the bounded source snapshot, before display capping and turn selection; zero before any read. |
| `unrecognized_records` | Nonnegative integer count of complete valid JSON records with unsupported types/envelopes/content variants, once per record, as defined by `transcript-reading`; zero before any read. |
| `notices` | Ordered shared strings: exact truncation notice, malformed-line count, unrecognized-record count; include only those whose condition holds. |
| `unavailable` | Null on a readable result, otherwise the reason token defined below. |
| `full_session` | Exact informational string or null required by the `transcript-reading` kind/resolution table; independent of TUI rendering. |

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

#### Scenario: JSON reports unknown records independently of empty and malformed
- **WHEN** an owned file contains a recognized header, five unrecognized complete records and no readable turns or malformed lines
- **THEN** JSON has `turns: []`, `unavailable: null`, `truncated: false`, `skipped_lines: 0`, `unrecognized_records: 5` and `notices: ["unrecognized transcript records: 5"]`; the text read says `no readable turns` with that same notice and exits zero

#### Scenario: Claude omission counts survive CLI turn selection
- **WHEN** the owned Claude source is the shipped projection fixture identified in `transcript-reading`, and the operator requests its whole transcript or `--turn 2`
- **THEN** JSON preserves `skipped_lines: 1`, `unrecognized_records: 0` and `notices: ["malformed transcript lines skipped: 1"]` in either read; the whole read returns the same two turns and the selected read returns only the unchanged second turn, with no notice for intentionally omitted content

#### Scenario: Claude lookup restrictions keep their exact CLI reasons
- **WHEN** a selected valid Claude reference with id `abcd-1234` has duplicate qualifying files, exceeds the discovery-entry bound, or has only a symlink candidate below its canonical home
- **THEN** JSON exits one with `unavailable` equal to `ambiguous-source`, `discovery-limit` or `unsafe-path` respectively, `path: null`, `turns: []`, zero diagnostic counts and `full_session: "full session: claude --resume abcd-1234"`; these are lookup failures, not empty or malformed transcripts

#### Scenario: A legacy Codex id remains unavailable in the command
- **WHEN** the selected participant has explicit Codex provenance, no common reference and only legacy `session_id: "abcd-1234"`, even with a matching local Claude file
- **THEN** JSON exits one with `unavailable: "no-reference"`, `transcript: null`, `legacy: false`, `path: null` and `full_session: null`, matching the TUI and browser participant eligibility; it returns no turns and reads no Claude file

#### Scenario: A missing Codex rollout has a fixed full-session value
- **WHEN** a participant's valid reference is `{"kind":"codex-thread","locator":"019c-222a","home":"/retained/codex"}` and lookup finds no matching file
- **THEN** JSON has `path: null`, `unavailable: "not-found"` and `full_session: "full session: rollout unavailable; codex exec resume 019c-222a; home: \"/retained/codex\""`, exactly, with empty turns and zero diagnostic counts; the command exits one

#### Scenario: Missing Claude and DSH files have distinct fixed hints
- **WHEN** valid Claude id `abcd-1234` and a valid DSH reference each lack a matching local file
- **THEN** each JSON read reports `not-found` and a null path, Claude has `full_session: "full session: claude --resume abcd-1234"`, and DSH has `full_session: null`

#### Scenario: Invalid DSH ownership has no fabricated path
- **WHEN** the only DSH candidate has a session header with a nonnumeric delegation depth
- **THEN** JSON carries `unavailable: "not-found"`, `path: null` and `full_session: null`, stderr says `no valid depth-zero DSH session header`, and no content from that candidate is returned

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
A positive `unrecognized_records` SHALL append its shared counted notice to
the explanation, never make unsupported content look like an empty recording.
Terminal control characters SHALL be sanitized in content, references,
paths, hints and diagnostics. Tool arguments/results SHALL remain readable
text and SHALL never be executed.

A readable result, including empty, skipped-line, unrecognized-record or
truncated results, SHALL exit zero unless a requested turn was not retained. Once run and participant
are selected, unavailability SHALL be one of `no-reference`, `none`,
`unsupported-kind`, `unannounced`, `missing-home`, `invalid-reference`,
`unsafe-path`, `not-found`, `ambiguous-source`, `discovery-limit`,
`unreadable`, or `turn-not-retained`. It SHALL exit one, carry no transcript
turns, and provide a sanitized explanation on stderr. With `--json` it SHALL
also emit the document above with the corresponding reason on stdout;
without `--json` the unavailable read SHALL emit no transcript body on stdout.
Known path, cap, both diagnostic counts and shared notices SHALL remain
available in an error document; a read failure SHALL still return no turns.

Reference failure precedence SHALL be: absent/ineligible common or legacy
reference candidate,
`none`, unsupported kind, empty locator, missing home, malformed reference,
then discovery/read failure. Unsafe candidate paths SHALL return
`unsafe-path` when no safe unique source is established, and a discovery
limit SHALL take precedence over a provisional unique candidate. The reader
SHALL report `unreadable` if an I/O failure prevents establishing a unique
answer. `turn-not-retained` SHALL apply only after a readable projection. Eligible
legacy provenance with a nonempty invalid id SHALL follow the reading
capability's `invalid-reference` rule instead of being treated as absent.
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

#### Scenario: All active notices keep the same order
- **WHEN** a readable source exceeds a size budget and its bounded snapshot contains one malformed line and two unrecognized records
- **THEN** text and JSON both carry notices in this order: `transcript truncated (size cap)`, `malformed transcript lines skipped: 1`, `unrecognized transcript records: 2`, without provider suffixes or quoted payloads

#### Scenario: Unknown selection cannot fall back to another seat
- **WHEN** `--seat` matches no participant or is an ambiguous label
- **THEN** either output mode fails before file lookup, leaves stdout empty and names the selection problem on stderr without reading another seat's prose

## Decisions

### C1 / clarification 3 — Diagnostic counts survive selection and errors

Add `unrecognized_records` to the not-yet-published `brokkr.transcript/v1`
document now, alongside `skipped_lines`. Whole and selected reads share both
counts and notices; neither turn selection nor a zero-turn projection hides
format drift. Unknown but valid JSON is a counted omission and remains a
successful bounded read, distinct from source unavailability. A future
structural change after publication requires a new document version.

### C2 / clarifications 4–6 — JSON consumes the reader's exact hint and notices

The reading capability, not the terminal UI, owns `full_session` and its
string/null table. The unresolved Codex scenario fixes the exact value and
shows that an informational command does not turn `not-found` into success.
Malformed, unknown-record and truncation notices also have one owner and
fixed ordering. Claude's convenience remains in the hint, never a suffix
that alters the common truncation string.

### C3 / clarifications 1–2 — Display indices are snapshot positions

Canonical records can replace streaming fallbacks when a file grows. Keeping
an old index attached to new content would misidentify the requested turn.
Each invocation selects only after the shared current projection is complete;
no durable message identity or frozen index across file changes is promised.

### C4 / second-pass clarifications 1–3 — Shared classifications remain observable

The command inherits the reading capability's closed Claude omission list,
browser-independent participant eligibility and precise discovery reasons.
The new selection scenario fixes the shipped Claude fixture at `(1, 0)`
diagnostic counts without widening the successful browser envelope. Lookup
refusals retain the reader's non-null Claude hint and zero counts before a
source is read. A direct id-only browser request is a different selector;
its local Claude result cannot make a legacy Codex participant eligible.
No second JSON schema or independent hint/omission rule is introduced.
