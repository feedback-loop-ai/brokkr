## Purpose

Expose the same local transcript the TUI reads through a scriptable command,
with unambiguous run, participant and turn selection, safe terminal text,
versioned JSON and explicit failures when retained content is unavailable.

## MODIFIED Requirements

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
provider steps or raw JSONL lines. Multiple readable members of one DSH
packed row SHALL receive separate indices; equivalent packed and ordinary
event encodings SHALL have the same turns and indices when both complete
sources fit the input budget. Indices address one bounded snapshot;
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
- **WHEN** a supported DSH text-chunks row yields three readable fragments, followed by an ordinary user message, with both budgets satisfied
- **THEN** `--turn 2` returns only the second fragment with its reconstructed stamp, `--turn 4` returns the user message, and equivalent ordinary chunk rows yield exactly those same indices and turns

#### Scenario: Partial citations recompute the displayed indices
- **WHEN** a complete DSH snapshot has chunks at sequences 10, 11, 12 and 14 followed by a readable same-step assembly citing `[[10, 12]]`
- **THEN** turn one is the uncited chunk 14, turn two is the assembly and `--turn 3` returns `turn-not-retained`; CLI and TUI share that sequence whether the chunks were packed or unpacked

#### Scenario: Turn selection cannot bypass DSH semantic refusal
- **WHEN** an owned DSH file has readable early content followed within the source cap by an unknown required event or structurally invalid packed row, and the operator requests the whole transcript, `--turn 1` or `--turn 999`
- **THEN** every request exits one with `unsupported-format`, no turns and the same source metadata/counts/notices; neither the early turn nor the out-of-range request changes the refusal, even if display capping would have stopped before the offending row

### Requirement: JSON exposes a distinct local transcript document

`--json` SHALL emit one JSON object with `schema: "brokkr.transcript/v1"`
and the following members, present even when null or empty:

| Member | Meaning |
|---|---|
| `run_id` | Resolved full run identifier. |
| `seat` | Resolved participant key, never only its possibly repeated label. |
| `transcript` | Selected common `{kind, locator, home}` with its three recorded string values unchanged, even when rejected as `none`, `unsupported-kind`, `unannounced`, `missing-home` or `invalid-reference`; when no common reference exists, the valid synthesized legacy Claude reference, or null if none can be synthesized. |
| `legacy` | Boolean indicating a synthesized legacy Claude reference. |
| `path` | Confirmed local source path, or null when no owned file was resolved. |
| `turn` | Requested one-based index, or null for the whole transcript. |
| `turns` | Ordered shared `Turn` values, filtered to one when requested. |
| `truncated` | Boolean for source/display truncation before turn selection; on `unsupported-format` or a failed source snapshot, only source truncation already established by the cap probe is retained. |
| `skipped_lines` | Nonnegative integer count of complete malformed JSON physical lines in the usable bounded snapshot admitted for decoding, before display capping and turn selection; zero if no usable snapshot was acquired or DSH header-version admission failed. A structurally invalid valid-JSON packed row is not a malformed JSON line. |
| `unrecognized_records` | Nonnegative integer count of complete valid JSON physical rows with unsupported types/envelopes/content or invalid DSH packed/citation encodings, once per row even if it has many members; includes event/storage-refusal rows under `transcript-reading`, and is zero if no usable snapshot was acquired or DSH header-version admission failed. The rejected opening header is not a counted row. |
| `notices` | Ordered shared strings: exact truncation notice, malformed-line count, unrecognized-record count; include only those whose condition holds. |
| `unavailable` | Null on a readable result, otherwise the reason token defined below. |
| `full_session` | Exact informational string or null required by the `transcript-reading` kind/resolution table; independent of TUI rendering. |

This document SHALL be an explicit local read result, not a journal record,
export format, or addition of prose to `RunView`. It SHALL NOT change the
existing inspect/seats JSON shape or view version solely to introduce this
new command. A present common reference SHALL be selected before validation
and echoed in the result with `legacy: false`. Refusal or path
canonicalization SHALL NOT normalize, reclamp, replace or null its recorded
fields. JSON escaping preserves these string values as data and does not
authorize a lookup or command. Only absence of a common reference together with failure to derive
a valid eligible legacy Claude reference SHALL make `transcript` null.
These document rules apply to common references admitted by the selected
run's seat-record contract. The frozen `seat-record.v1` through
`seat-record.v4` contracts admit only `claude-session`, `codex-thread`,
`dsh-session` and `none`; their append and verification fence SHALL reject an
unknown kind before it can become a selected participant. The current command
therefore SHALL NOT promise an `unsupported-kind` document from a valid
persisted run. The pure reader's defensive handling of a directly supplied
unknown string remains governed by `transcript-reading` and does not weaken
that journal fence.
JSON strings SHALL retain content using JSON escaping, without
ANSI decoration, terminal control execution or prose scraped from rendered
text. Structural changes to this new public document SHALL require an
explicit version change in its schema identifier.

#### Scenario: Scripts receive the selected identity and shared turns
- **WHEN** `brokkr transcript --run <prefix> --seat <label> --json` resolves a readable reference
- **THEN** one JSON document contains the full run id, exact participant key, common reference, confirmed path, shared turn objects and explicit truncation state under `brokkr.transcript/v1`

#### Scenario: JSON retains every rejected common reference
- **WHEN** a selected participant has one of the following common references, including when a stale valid legacy Claude id is also present
- **THEN** JSON exits one with `transcript` equal to the input's three string values, `legacy: false`, the listed `unavailable` reason, null `path` and `full_session`, `turns: []`, `truncated: false`, zero diagnostic counts and no notices; it performs no file or legacy lookup

| Recorded common reference | `unavailable` |
|---|---|
| `{"kind":"none","locator":"","home":""}` | `none` |
| `{"kind":"codex-thread","locator":"","home":"/retained/codex"}` | `unannounced` |
| `{"kind":"codex-thread","locator":"0199mine","home":""}` | `missing-home` |
| `{"kind":"codex-thread","locator":"-abc","home":"/retained/codex"}` | `invalid-reference` |

#### Scenario: The frozen seat-record vocabulary fences an unsupported kind before the command
- **WHEN** a seat record governed by frozen version 1, 2, 3 or 4 attempts to carry `{"kind":"future-session","locator":"opaque-222","home":"/retained/future"}`
- **THEN** journal append or verification rejects that seat record before it can become the command's selected run and participant, so `brokkr transcript` constructs no `brokkr.transcript/v1` document from it
- **AND** a pure-view caller that directly supplies those same three strings still receives the unchanged reference with `unsupported-kind` under `transcript-reading`; the view fold preserves an unknown kind and is not the contract fence

#### Scenario: Codex JSON does not impose a header or Claude id guard
- **WHEN** a common reference names `0199mine` under `/retained/codex` and the sole safe matching file is `/retained/codex/sessions/rollout-0199mine.jsonl` containing only recognized `turn_context` metadata and no session header
- **THEN** JSON exits zero with that unchanged reference, `legacy: false`, the confirmed path, `unavailable: null`, zero turns and diagnostic counts, no truncation and the shared confirmed-path Codex full-session string; no Claude lookup occurs

#### Scenario: JSON distinguishes an empty transcript from unavailability
- **WHEN** one read resolves an empty valid file and another resolves a missing file
- **THEN** both carry an empty `turns` array, but the first has `unavailable: null` and the second `unavailable: "not-found"` with no fabricated source path

#### Scenario: JSON reports unknown records independently of empty and malformed
- **WHEN** an owned file contains a recognized header, five unrecognized complete records (with top-level `ignorable: true` for unknown DSH events) and no readable turns or malformed lines
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

#### Scenario: DSH format refusal has a complete JSON state
- **WHEN** an owned DSH source with a valid common reference and safely confirmed path has the capped `(skipped_lines: 2, unrecognized_records: 2)` required-unknown snapshot specified in transcript-reading
- **THEN** whole JSON exits one with the recorded reference unchanged, `legacy: false`, the confirmed path and shared DSH path hint, `turn: null`, `turns: []`, `unavailable: "unsupported-format"`, `truncated: true`, those same counts and exactly `["transcript truncated (size cap)", "malformed transcript lines skipped: 2", "unrecognized transcript records: 2"]`; a valid turn request changes only `turn` to the requested index
- **AND** text mode exits one with empty stdout and a sanitized stderr explanation naming `unsupported-format`, `DSH transcript format is not supported` and those same notices, without earlier or rejected prose

#### Scenario: Rejected DSH header versions have a fixed command document
- **WHEN** the selected common reference is `{"kind":"dsh-session","locator":"sessions/brokkr/seat-222","home":"/retained/dsh"}` and its unique safe file `/retained/dsh/sessions/brokkr/seat-222/project/root/session.jsonl` has an owned header with foreign, missing or mistyped version and a usable source below the source cap, including malformed or readable-looking later rows
- **THEN** JSON exits one with the resolved run/key, that unchanged `transcript`, `legacy: false`, the confirmed `path`, `full_session: "full session: \"/retained/dsh/sessions/brokkr/seat-222/project/root/session.jsonl\""`, `turn: null`, `turns: []`, `unavailable: "unsupported-format"`, `truncated: false`, zero diagnostic counts and `notices: []`
- **AND** `--turn 1` or `--turn 999` changes only `turn` to the requested index; measured source overflow instead sets `truncated: true` and adds only `transcript truncated (size cap)`, with counts still zero; text mode exits one with empty stdout and sanitized stderr naming the same refusal and `DSH transcript format is not supported`, without header or event payloads

#### Scenario: DSH format support cannot select between root candidates
- **WHEN** the same selected DSH reference has safe version-zero and version-one root candidates, with discovery otherwise successful
- **THEN** whole and selected JSON reads exit one with `ambiguous-source`, the unchanged reference and `legacy: false`, null path/hint, no turns, zero counts, `truncated: false` and no notices; text mode has empty stdout and an ambiguity explanation, and neither mode returns the supported root or reports only the foreign root's format refusal

#### Scenario: Packed encoding does not multiply diagnostic rows
- **WHEN** a valid DSH packed row with three readable members is followed by one valid-JSON but structurally invalid packed row
- **THEN** JSON returns `unsupported-format`, no turns, `skipped_lines: 0` and `unrecognized_records: 1`, with the confirmed path/hint retained; neither three valid members nor partially decoded members of the invalid row multiply the count

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

A readable result, including empty, skipped-line, safely omitted
unrecognized-record or truncated results, SHALL exit zero unless a requested
turn was not retained. A DSH semantic refusal is not a readable result even
when its diagnostic counts are positive. Once run and participant
are selected, unavailability SHALL be one of `no-reference`, `none`,
`unsupported-kind`, `unannounced`, `missing-home`, `invalid-reference`,
`unsafe-path`, `not-found`, `ambiguous-source`, `discovery-limit`,
`unreadable`, `unsupported-format`, or `turn-not-retained`. It SHALL exit one,
carry no transcript turns, and provide a sanitized explanation on stderr. With `--json` it SHALL
also emit the document above with the corresponding reason on stdout;
without `--json` the unavailable read SHALL emit no transcript body on stdout.
Known path, cap, both diagnostic counts and shared notices SHALL remain
available in an error document according to the reading capability's failure
state rules; a failed read SHALL still return no turns. On DSH
`unsupported-format`, the document SHALL retain the selected reference,
legacy flag, confirmed path and full-session hint. On header-version
refusal both counts SHALL be zero, since no event classification begins;
on event/storage refusal after header admission both counts SHALL cover all
complete physical rows in the usable prefix. In either case, `truncated`
SHALL mean source-cap truncation only. The explanation SHALL be
`DSH transcript format is not supported`, beside the reason token and shared
notices, without quoting an unknown event or invalid storage payload.

Reference failure precedence SHALL be: absent/ineligible common or legacy
reference candidate,
`none`, unsupported kind, empty locator, missing home, malformed reference,
then discovery/read failure. Unsafe candidate paths SHALL return
`unsafe-path` when no safe unique source is established, and a discovery
limit SHALL take precedence over a provisional unique candidate. The reader
SHALL report `unreadable` if an I/O failure prevents establishing a unique
answer. DSH candidate uniqueness SHALL be evaluated independently of header
version, and source I/O/UTF-8 failure SHALL precede DSH header-version or
event/storage refusal. With a usable bounded snapshot, rejected header
admission SHALL precede event classification; either `unsupported-format`
case SHALL precede display capping and turn selection. `turn-not-retained`
SHALL apply only after a readable
projection. Eligible legacy provenance with a nonempty invalid id SHALL follow the reading
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
format drift. Unknown but valid JSON is counted separately from malformed
JSON. R14/C6 refine the original universal-success rule on new DSH evidence: only safely
omittable unknown DSH events remain successful; required unknown events and
invalid packed/citation encodings return `unsupported-format`. A future
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

### C5 / third-pass clarification 3 — Reference presence survives rejection

Adopt the finding: "effective" left validation failures free to erase a
present common reference. Preserve exactly its recorded three string values
for every failure and keep `legacy: false`. Nulling it is rejected because
it makes an unsupported or broken reference indistinguishable from no
reference, while C1 already commits this document to versioned behavior.
Reference echo is local diagnostic data, not proof of validity: lookup and
full-session construction still stop under the reading capability's guard
and reason precedence. The five-row scenario pins each reference refusal
without changing the older null/synthesized-legacy scenarios.

The dependent Codex scenario also consumes R12/R13: a filename-matching
non-hex id without a header resolves through the same reader as the TUI.
There is no command-specific discovery or identifier language. Proposed 0055
must record this JSON presence/null rule with serialization/refusal tests as
its enforcement binding; existing inspect/seats JSON, journal schemas and
frozen contracts gain no fields.

### C6 / design return U1 — A counted row can also refuse the read

Adopt the semantic-refusal token and its complete error state before this
public document is implemented. Counts describe physical input evidence;
they do not grant permission to return prose. Both CLI modes inherit R14's
source-failure precedence, whole-prefix counts and source-only truncation.
Reject serving an early requested turn, reporting out-of-range first, or
returning a stale previous projection after required-unknown refusal. C5's
reference-presence rule and the reader's confirmed path/hint remain intact.
Proposed 0055 must bind the whole/selected/text/JSON refusal cases without
widening inspect/seats JSON or introducing transcript telemetry.

### C7 / design return U2 — Turn and diagnostic units are deliberately different

Adopt R15's decoded logical-event sequence for selection, with physical rows
as the diagnostic unit. A packed row can yield several separately selectable
fragments; choosing a turn still cannot bypass either source or display cap.
Citation-specific suppression can reduce or reorder the displayed prefix on
a later invocation, so C3 remains authoritative. The new paired-encoding,
partial-citation and invalid-row scenarios bind numbering/counts to the
shared result instead of a second CLI decoder. Proposed 0055 must name these
observable units and the CLI/TUI equivalence tests that enforce them.

### C8 / returned clarification — Header admission precedes diagnostic classification

Adopt R16's two distinct causes of `unsupported-format` in the existing
versioned document. Rejected DSH header admission retains the confirmed
path/hint but has zero counts because event interpretation has not begun;
event/storage refusal inside admitted version zero keeps C6's whole-prefix
counts. Both retain only measured source truncation and precede every valid
turn request. The command neither chooses a familiar version to resolve
ambiguous ownership nor changes a rejected common reference to null.
The two new scenarios bind JSON/text, whole/selected reads and source-cap
state to the reader's answer. Proposed 0055 must state and test these cases
without adding fields or changing the closed unavailable vocabulary.
