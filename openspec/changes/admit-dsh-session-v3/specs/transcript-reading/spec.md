## MODIFIED Requirements

### Requirement: Discovery identifies one owned local file

Claude discovery SHALL search immediate project directories of the recorded
projects home for the exact `<session-id>.jsonl` filename. Codex discovery
SHALL search only below `<recorded-home>/sessions`, through at most six
nested directory levels, for a `rollout-*.jsonl` filename containing the
entire recorded thread id as a case-sensitive whole token. Whole token
means that each adjacent filename character, when present, is not ASCII
alphanumeric; filename ends also delimit the token. Hyphens are delimiters,
as in shipped discovery. A substring adjacent to an ASCII alphanumeric
character SHALL not qualify, and modification time SHALL never establish
identity.

For Codex, that filename token within the recorded scope SHALL be the sole
identity predicate, subject to the shared uniqueness, bounds and safe-file
rules. Codex discovery SHALL NOT read transcript content or require a
session/header record, type, field or record position. An absent, incomplete, unknown or
conflicting content header SHALL neither disqualify a filename match nor
qualify a filename that does not match. In particular, a
`payload.thread_id` in an `event_msg` whose `payload.type` is
`thread_settings_applied` SHALL not select or veto a file, wherever it occurs.
Such records remain subject to the content projection and diagnostic rules
after file selection. An empty file with a unique eligible name SHALL be a
readable zero-turn source; read errors still return `unreadable`. This
preserves the shipped Codex filename-identity rule and removes the earlier
proposal's unmeasured content-header gate; it does not claim that arbitrary
file contents independently authenticate a session.

DSH discovery SHALL search only the retained root at
`<recorded-home>/<locator>`, using its project/session directory layout and
a closed, ordered set of session filenames: `session.v3.jsonl`, then
`session.jsonl`. A session directory SHALL yield at most one candidate, the
first name in that order carrying a valid header, so a directory holding
both admits the newer and never becomes ambiguous with itself. A name
outside the set SHALL NOT be read. The set is versioned because the DSH core
versions the file: 0.1.5-rc.1 writes `session.v3.jsonl` where earlier cores
wrote `session.jsonl`, and a reader that knows only one name loses exactly
the newest evidence. Its first JSONL record SHALL be a `session` header
with `delegationDepth` equal to zero; an omitted depth SHALL retain the
existing legacy meaning of zero. An invalid depth or a positive delegation
depth SHALL not match. Valid zero means a JSON unsigned-integer zero;
null, booleans, strings, negative numbers and floating-point numbers are
invalid, not evidence of a root session. This reader is deliberately
stricter than the shipped driver's non-u64-to-zero fallback. After discovery
completes without a higher-priority failure, if no eligible match exists and
at least one session header has invalid depth, the reader SHALL return
`not-found` with the explanation
`no valid depth-zero DSH session header`, without echoing the invalid value,
showing its prose or fabricating a full-session path. A separate qualifying
root session remains eligible; invalid/delegated candidates do not displace it.
Other roots, compressed files and delegated sessions SHALL not be substitutes for the seat's plain JSONL file.

DSH ownership discovery and content-format admission SHALL be separate.
For ownership, the entire header shape SHALL be a first-record JSON object
whose `type` is exactly `session` and whose depth satisfies the preceding
rule. A missing, empty, incomplete or malformed first record, a non-object,
or another type SHALL not qualify; discovery SHALL never skip it to find a
later header. Completeness SHALL follow the shared final-line rule: a valid
JSON header at EOF needs no trailing newline. The `version` field SHALL
neither qualify nor veto ownership. Every safe depth-zero candidate SHALL
participate in uniqueness regardless of its version; a supported version
SHALL not outrank a foreign, missing or mistyped one. Before a unique owned
source is confirmed, failures retain null path and DSH full-session hint,
no turns, zero counts, `truncated: false` and no notices. In particular,
invalid-depth/delegated headers keep the preceding outcomes even when their
version is foreign. Header read I/O or invalid UTF-8 that prevents a unique
answer SHALL return `unreadable`, not a guessed ownership result.

Discovery SHALL inspect at most 10,000 directory entries per lookup and
read at most 65,536 bytes of each DSH candidate's first-record header.
Claude and Codex discovery SHALL read no transcript content; their selected
source reads use the separate 32 MiB budget below, with no 65,536-byte
opening-record gate. Exhausting discovery limits SHALL return
`discovery-limit`, not the first candidate found. Exactly
one qualifying file SHALL be required; several qualifying files SHALL return
`ambiguous-source` regardless of enumeration order. No match SHALL return
`not-found`. A file not yet created or a DSH first-record header not yet
complete can become available on a later read. These bounds apply equally
to CLI lookup, TUI refresh and the existing Claude browser routes. For
Claude this deliberately replaces the shipped first matching file, unbounded
project enumeration and symlink following with unique, bounded, safe
discovery. These are breaking lookup
changes proposed for 0055, not implied compatibility with the old reader.

After safe unique DSH ownership and a usable bounded UTF-8 source snapshot
are established, content admission SHALL require an opening header whose
top-level `version` is a JSON number in the admitted version set. That set
is closed and is exactly `{0, 3}`. Numeric zero spellings such as `0`,
`0.0`, `0e0` and `-0` SHALL denote version zero. Numeric three spellings
SHALL denote version three only when the parsed value is three and the
recorded token, with its decimal point and leading and trailing zeros
removed, spells exactly `3`, as `3`, `3.0` and `3e0` do; `3.1`, `3e1`,
`-3` and a token whose parsed value rounds to three but whose recorded
digits are not `3` SHALL not. Exactness SHALL be judged on the recorded
top-level `version` token as well as its parsed value, for zero and for
three alike. Admitting three widens that recorded-token version check
from one admitted integer to two and touches no other field: an ordinary
event's `time` and a packed row's `time0` keep the exactness the content
rules below already require of them under either version, and no
citation, marker or other event field is judged on its recorded token
under either version. Strings, booleans and null SHALL not be coerced. Missing `version`, any
nonnumeric value and any numeric value outside the set, including 1, 2 and
4, SHALL return `unsupported-format` with
`DSH transcript format is not supported`. There is no legacy default version
or automatic migration. An omitted depth still means zero under the settled
ownership rule; a missing version does not.

Each admitted version is admitted because its writer was read, and the
admitting read is cited from the change that admitted it. Version zero: the
0.1.2-rc.1 session and persistence-JSONL packages captured for #222
declare `SESSION_FORMAT_VERSION = 0`, write that header, refuse a foreign
version before decoding, and supply the event catalogue, packed-row and
citation types behind the version-zero rules below. Version three: the
`@deepseek-ai/dsh` 0.1.5-rc.1 core installed on the fleet's host, whose
session packages at 0.1.5-rc.2 declare `SESSION_FORMAT_VERSION = 3` and a
format catalogue with current version three, codecs zero through three and
lossless migrations between them, read for #279 and cited by file, digest
and line in that change. That read reached the session package's format
constant, event catalogue, event map, surface contract and citation
grammar and the catalogue and migration packages. It did not reach the
message and block definitions the event map imports for the payloads of
`user/message`, `assistant/message` and `tool/result`, the persistence
package's row kinds and its append and replace paths, or the seed and
fork path that writes a seeded session's inherited prefix. Under version
three those three payloads are therefore refused row by row by the content
rules below, a replacement copy and an inherited identity decide nothing,
and `tool/call`, whose fields the event map carries on the event itself,
is the one content kind that projects. Versions 1 and 2 are not admitted:
no core of this fleet wrote them to disk under an admitted name and their
codecs were not read. A version SHALL join the set only through a change
that records, from the writer's own source, its package identity and
files with digests; the record vocabulary it emits; the message and block
definitions behind every payload it projects, mapped field by field; how
streaming fragments persist; the meaning of every header field it adds;
the meaning of `surfaceOp` and `sourceEventSeqs` on every row that
carries them, including whether replacement copies are persisted as rows;
the identity of a seeded session's inherited rows; a disposition for every
record type; and whether every admitted-version meaning is preserved or a
per-version projection is required. A versioned filename, a header number
and the resemblance of sampled rows to admitted shapes are not admission
evidence, and a part of a writer that was not read SHALL stay refused row
by row rather than admitted with its version. The filename and the header
version are independent facts: `session.v3.jsonl` carrying a version-zero
header is readable under the version-zero rules, `session.jsonl` carrying
version three is readable under the version-three rules, and either name
carrying a version outside the set is refused; neither infers the other.

Other header fields, including `id`, `createdAt`, `cwd`, `parentSession`,
`seedLength`, `isSeeded`, `origin`, `agentPreset` and old execution-policy
fields, SHALL not qualify or veto this audit read, even when absent or
mistyped. In particular `isSeeded`, which the version-three writer records
as a boolean meaning the session carries a fork-inherited event prefix,
SHALL neither establish nor remove ownership, SHALL not alter the
delegation-depth rule and SHALL not supply admission, whatever its JSON
shape, including null and absent; no provenance or delegation claim rests
on it under either version. Whether a seeded session's inherited prefix
keeps the originating `seq`, `turn`, `step` and call identities of its
rows was not read for version three, so no version-three association
rests on those identities either: the content rules below apply no
embedded tool association under version three, and `isSeeded` changes no
row's classification. Extra fields SHALL be ignored, never
interpreted as an event, identity, path, timestamp or execution policy.
This is the complete reader header-admission shape; it is deliberately not
DSH's execution-replay header validator.

A rejected header version SHALL preserve the selected common reference with
`legacy: false`, the safely confirmed path and its shared DSH `full_session`,
return no turns and zero `skipped_lines` and `unrecognized_records`. No
subsequent row SHALL be JSON-classified, decoded as a version-zero event or
packed row, associated, or charged to the display budget. The rejected
header itself SHALL not contribute either row count. `truncated` SHALL
retain only source truncation established by the bounded read's extra-byte
probe, and `notices` SHALL contain only `transcript truncated (size cap)`
when that flag is true; otherwise notices SHALL be empty. Counts are zero
because format admission failed, not a claim that the remaining bytes are
valid, understood or empty. The entire source snapshot remains subject to
I/O/UTF-8 validation and the source cap before this refusal; source failure
SHALL outrank header refusal with the existing `unreadable` state. Discovery
failures SHALL likewise precede header admission. Header refusal SHALL
precede event/storage diagnostics, display capping and all valid turn
requests; neither early readable-looking text nor `ignorable: true` can
bypass it. The current snapshot's opening header SHALL be checked on every
read, including refresh; an old admission or a later `session` record cannot
supply a missing or supported version for it.

An admitted opening header SHALL be a quiet record, and the version it
carries SHALL select the vocabulary under which every later row of that
file is classified. Subsequent `session`-typed rows SHALL be unknown event
envelopes under the DSH ignorable rule, not additional format headers or
version switches. R14/R15's event, packed-row and citation admission rules
and whole-prefix diagnostic counts SHALL apply only after admitted-version
header admission, under that version's vocabulary. Elsewhere in these
deltas, a recognized DSH header or readable DSH source means one admitted
under these rules; owned location alone does not establish a readable format.

For the id-only Claude API, every shared reference/discovery/read failure
SHALL return HTTP 404 with the existing JSON error envelope, with no turns.
An invalid id SHALL use `{"error":"session not found"}`; a valid id with a
lookup/read failure SHALL use `{"error":"transcript not found"}`. A readable
empty, counted-omission or truncated projection SHALL still return HTTP 200
with only `session_id`, `turns` and `truncated`.

Claude SSE admission SHALL apply the same identifier and discovery rules.
Any refusal, including `ambiguous-source`, `discovery-limit` and `unsafe-path`,
SHALL return HTTP 404 with `{"error":"transcript not found"}` before an
event-stream header or event is written. For an admitted source the existing
size-event/heartbeat shape stays unchanged. Each subsequent poll SHALL
revalidate safe unique discovery; if that fails, the stream SHALL close
without reporting further sizes from the previous candidate. HTTP status
cannot change after stream admission; closing does not emit a second 404
response. SSE remains a growth notification, not a new transcript body API.

#### Scenario: Claude browser lookup refusals have fixed HTTP responses
- **WHEN** a valid Claude id has either two qualifying project files, a search that needs more than 10,000 entries to establish uniqueness, or only a matching symlink project/file below the canonical home
- **THEN** the shared lookup returns `ambiguous-source`, `discovery-limit` or `unsafe-path` respectively; `/api/session/<id>` and a new `/sse/session/<id>` request each return HTTP 404 with `{"error":"transcript not found"}`, no turns and no event-stream header
- **AND** the participant page and TUI keep the reader's shared Claude hint beside the specific unavailable explanation; no surface selects the former first candidate or follows the symlink

#### Scenario: An admitted Claude growth stream loses its unique source
- **WHEN** a growth stream has begun for one safe Claude file and another qualifying file appears, discovery exceeds its bound, or the selected file/project is replaced by a symlink before a later poll
- **THEN** that poll closes the stream without reporting another size from the old candidate; a new stream request receives HTTP 404 and a new API read supplies no transcript prose

#### Scenario: Concurrent Codex threads do not share a result
- **WHEN** the selected thread is `0199mine` and a dated sessions tree contains `rollout-0199mine.jsonl`, a newer `rollout-0199other.jsonl`, and `rollout-0199mineX.jsonl`
- **THEN** only `rollout-0199mine.jsonl` is eligible, independent of payload ids, timestamps and directory order; the trailing ASCII `X` makes the third filename a fragment match

#### Scenario: A Codex rollout needs no session header
- **WHEN** the unique safe `rollout-0199other.jsonl` in the recorded sessions tree contains only the shipped `turn_context` fixture from `crates/brokkr-protocol/src/adapters/tests.rs:1183-1186` at base `5bc8cf3`, with no header or thread-identifying record
- **THEN** selecting `0199other` resolves that path, returns a readable zero-turn result with zero diagnostic counts and its confirmed-path Codex hint, rather than `not-found`; subsequent recognized readable messages project without waiting for a header

#### Scenario: Codex payload ids do not select or veto a file
- **WHEN** the selected thread is `0199mine`, its uniquely named safe rollout contains a `thread_settings_applied` event naming another thread, and an unrelated rollout instead has that event naming `0199mine`
- **THEN** the filename-matching file remains the only eligible source and its recognized content is projected; the unrelated filename remains ineligible, regardless of the event's position, absence or payload id

#### Scenario: Codex content uses the source cap rather than a header cap
- **WHEN** a unique safe filename-matching Codex rollout begins with a complete recognized readable record larger than 65,536 bytes but fits both the 32 MiB source and 4,000,000-byte display budgets
- **THEN** the reader projects that record without a discovery-limit failure, extra header scan or invented truncation; a partial opening append is handled by the shared partial-record rule and does not make filename identity unavailable

#### Scenario: A DSH child cannot replace its parent
- **WHEN** the recorded DSH root contains one depth-zero session and a newer depth-one delegated session
- **THEN** only the depth-zero session file supplies turns and the full-session path

#### Scenario: A legacy DSH header omits the depth
- **WHEN** the only otherwise qualifying DSH file starts with a session header lacking `delegationDepth`
- **THEN** it is eligible as depth zero, while negative, string and nonzero depths are not eligible

#### Scenario: A driver-folded invalid DSH depth is explicitly refused
- **WHEN** a participant records a DSH root whose only session header has `delegationDepth: "zero"`, even if controller evidence shows the shipped driver already folded that file
- **THEN** the reader returns `not-found` with `no valid depth-zero DSH session header`, no turns, no confirmed path and no full-session hint; it does not coerce the value or alter the adapter

#### Scenario: DSH admits only the declared version-zero header shapes
- **WHEN** a unique safe DSH source begins with `{"type":"session","version":0,"delegationDepth":0}`, or the equivalent numeric version `0.0`, `0e0` or `-0`, with depth zero or omitted depth, followed by supported events within both caps
- **THEN** each source is admitted and projects those events with its confirmed path/hint and zero diagnostic counts; absent or mistyped unused header metadata adds no gate or notice, and a file containing only such a header at EOF without a newline is a readable zero-turn source

#### Scenario: An absent or malformed opening DSH header cannot borrow a later one
- **WHEN** the only safe candidate is empty, has a malformed or incomplete first JSON row, starts with a non-object JSON value, or starts with an object whose type is not `session`, even if a valid version-zero session header appears later, with discovery otherwise complete within its bounds and no I/O/UTF-8 failure
- **THEN** discovery returns `not-found` with null path/hint, no turns, zero counts, no truncation and no notices; it does not treat a later row as the opening header or label an incomplete opening append as unsupported format

#### Scenario: Missing and mistyped DSH versions are not legacy zero
- **WHEN** a unique safe candidate starts with a `session` object whose depth is zero or omitted, but whose version is absent, null, false, `"0"`, `"3"`, an array or an object, and its bounded source is usable without source truncation
- **THEN** ownership resolves its path, but content admission returns `unsupported-format`, `DSH transcript format is not supported`, the unchanged reference and confirmed path/hint, no turns, zero counts, `truncated: false` and no notices; no version is synthesized and an ignorable marker cannot make the header readable

#### Scenario: A foreign-version DSH root alone keeps its confirmed location
- **WHEN** the unique safe candidate's opening `session` object has depth zero and version 1, 2, 4, -1, -3, 0.5, 3.1, 3e1, `2.9999999999999999` or `3.0000000000000001`, with a usable bounded source below the source cap, including if unused current-version header metadata is missing or malformed
- **THEN** the reader returns `unsupported-format` with its unchanged reference, `legacy: false`, confirmed path and DSH path hint, no turns, zero diagnostic counts, `truncated: false` and no notices; it neither returns `not-found` nor decodes the apparently familiar later events, and a token whose parsed value rounds to an admitted integer is refused on its recorded digits

#### Scenario: DSH admits only the declared version-three header shapes
- **WHEN** a unique safe DSH source begins with `{"type":"session","version":3,"delegationDepth":0,"isSeeded":false}`, or the equivalent numeric version `3.0` or `3e0`, with depth zero or omitted depth, followed by version-three quiet rows and one `tool/call` row within both caps
- **THEN** each source is admitted under the version-three vocabulary and projects that call once with its confirmed path/hint and zero diagnostic counts; absent or mistyped unused header metadata adds no gate or notice, and a file containing only such a header at EOF without a newline is a readable zero-turn source

#### Scenario: A version-3 session is located, its measured rows project and its message rows refuse
- **WHEN** one recorded root's unique safe DSH candidate is `session.v3.jsonl` whose opening row is `{"type":"session","version":3,"delegationDepth":0,"isSeeded":false}`, followed by complete rows typed `system/message`, `todo/write`, `turn/end` and one `tool/call`, and another root's `session.v3.jsonl` holds the same rows followed by a readable-looking `user/message`, the call's `tool/result` and an `assistant/message` carrying `surfaceOp: "append"`, `stream` and `usage`, both within both caps, with discovery complete and no I/O or UTF-8 failure
- **THEN** discovery resolves each path; the first read projects the call once with zero diagnostic counts and no notice, the three quiet rows supplying no content; the second returns `unsupported-format` with `DSH transcript format is not supported`, its confirmed path/hint, no turns, `unrecognized_records: 3`, `skipped_lines: 0` and exactly `unrecognized transcript records: 3`, because version-three message payloads are refused until their definitions are read
- **AND** CLI text, CLI JSON and the TUI show that same projection and that same refusal, `--turn 1` reaches the call in the first, and the second keeps its notices with both reading doors disabled

#### Scenario: The filename and the header version are independent facts
- **WHEN** one recorded root's only session directory holds `session.jsonl` whose opening header has version 3 and depth zero followed by a `tool/call`, another's holds `session.v3.jsonl` whose opening header has version 0 and depth zero followed by a readable `user/message`, and a third's holds `session.v3.jsonl` whose opening header has version 4 and depth zero
- **THEN** the first projects its call under the version-three vocabulary and the second its message under the version-zero vocabulary, each with zero counts and no notice, and the third returns `unsupported-format` with its confirmed path and zero counts; neither name infers a version and neither version infers a name

#### Scenario: The versioned name beside the plain name is decisive, whatever its version
- **WHEN** one session directory holds `session.v3.jsonl` with a version-3 header followed by a `tool/call` and `session.jsonl` with a version-zero header followed by a readable `user/message`, and another directory holds `session.v3.jsonl` with a version-4 header beside `session.jsonl` with a version-zero header and a readable `user/message`
- **THEN** the first directory reads `session.v3.jsonl` and projects its call under the version-three vocabulary, never the plain name's message, and the second returns `unsupported-format` with the confirmed `session.v3.jsonl` path, zero counts and no turns; neither directory is ambiguous with itself, and the plain name is never read once the versioned name carries a valid session header, because falling through would present superseded evidence as the seat's newest transcript

#### Scenario: A seeded header changes neither ownership nor admission
- **WHEN** the unique safe candidate's opening header has depth zero or omitted depth and `isSeeded` true, false, a string, an object, null or absent, followed by a readable `user/message` under version 0 and by a `tool/call` under version 3
- **THEN** every source is admitted and projects that row with zero counts and no notice, no seeded value adds a gate, a notice, a claim about provenance or a change to any row's classification, and the reader claims nothing about an inherited prefix
- **AND** when the only candidate's header has version 3, `isSeeded: true` and `delegationDepth: 1`, it is not a candidate and the result is `not-found` with null path and hint; the depth rule governs ownership and `isSeeded` does not

#### Scenario: Version zero already rules the three sampled types
- **WHEN** a unique safe candidate with a version-zero header contains, in order, a `todo/write` row, a `turn/end` row, a readable `user/message` and a `system/message` row without an `ignorable` marker, and a second otherwise identical file whose `system/message` row carries top-level `ignorable: true`
- **THEN** the first read returns `unsupported-format` with no turns and one unrecognized record, because `system/message` is outside the captured 0.1.2-rc.1 catalogue and is a required unknown under version zero; the second projects the user message once with one unrecognized record and its notice
- **AND** in both files `todo/write` and `turn/end` are recognized quiet under that catalogue and count in neither diagnostic; their absence from version-zero samples is a sampling fact, not a version difference

#### Scenario: Current and foreign DSH roots are still ambiguous
- **WHEN** the retained scope contains two safe root candidates, one with version zero and one with version 1, absent version or mistyped version, with discovery otherwise complete
- **THEN** either enumeration order yields `ambiguous-source`, null path/hint, no turns, zero counts, `truncated: false` and no notices; supporting one format cannot select that root, and two foreign-version root candidates produce the same ambiguity

#### Scenario: Header versions cannot override DSH depth ownership
- **WHEN** one version-zero root is accompanied only by foreign-version candidates with positive or invalid delegation depths
- **THEN** the root remains the unique candidate and its supported content is readable, with no diagnostic contribution from the rejected candidates
- **AND** if only a foreign-version candidate with invalid depth remains, the result is `not-found` with `no valid depth-zero DSH session header`, null path/hint, no turns, zero counts and no truncation/notices; foreign version cannot outrank the ownership refusal

#### Scenario: DSH discovery failures precede version admission
- **WHEN** a foreign-version root candidate is observed but completing discovery then exceeds the entry/header bound, or an I/O or UTF-8 failure prevents establishing uniqueness
- **THEN** the result is respectively `discovery-limit` or `unreadable`, with null path/hint, no turns, zero counts and no truncation/notices; a provisional unsupported candidate cannot replace that discovery failure

#### Scenario: Multiple candidates are not merged or ranked
- **WHEN** the recorded search scope contains two qualifying Claude files, two matching Codex rollouts or two depth-zero DSH sessions
- **THEN** the reader reports `ambiguous-source`, displays no transcript prose and does not choose by recency or filesystem enumeration order

#### Scenario: Discovery is bounded even in an unrelated large home
- **WHEN** establishing a unique candidate would exceed 10,000 examined entries or requires a DSH first-record header exceeding 65,536 bytes
- **THEN** discovery reports `discovery-limit` and performs no unbounded scan or header allocation


### Requirement: DSH sessions expose assembled or provisional content once

The rules of this requirement are the version-zero rules, measured on the
captured 0.1.2-rc.1 writer, except where a paragraph names version three;
the version-three paragraphs below say which rows version three projects,
which it refuses and why, and what read lifts each refusal.

DSH SHALL read retained `user/message`, assembled `assistant/message`,
`tool/call` and `tool/result` events in source order, including readable
reasoning in assembled messages. It SHALL support ordinary event rows and
`text-chunks`, `reasoning-chunks` and `tool-call-chunks` storage rows, including
files mixing both encodings. Packed rows SHALL decode before event projection;
they SHALL not be classified as unknown events or joined into one message.
The supported packed representation is:

| Row | Logical members and recorded fields |
|---|---|
| `text-chunks` | `data.texts[k]` supplies one `assistant/chunk` with `chunk.type: "text-delta"`, its unchanged text and `data.index`. |
| `reasoning-chunks` | `data.texts[k]` supplies one `assistant/chunk` with `chunk.type: "reasoning-delta"`, its unchanged text and `data.index`. |
| `tool-call-chunks` | `data.args[k]` supplies one `assistant/chunk` with `chunk.type: "tool-call-delta"`, `chunk.argumentsDelta` equal to that raw fragment, and `chunk.index`, `chunk.id` and optional `chunk.name` from the corresponding data fields. It is not a completed call. |

The envelope SHALL contain exactly `type`, `seq0`, `time0`, `data`.
Text/reasoning data SHALL contain exactly `turn`, `step`, `index`, `dt`,
`texts`; tool data SHALL contain exactly `turn`, `step`, `index`, `dt`,
`args`, `id` and optional `name`. The three position fields SHALL be numeric;
`texts`/`args` SHALL be nonempty arrays of strings, and `id` and a present
`name` SHALL be strings. `dt` SHALL be an array with one fewer member than
the payload array. `seq0` SHALL be a nonnegative safe integer, excluding
negative zero. `time0` and every `dt` member SHALL be signed safe integers. Here safe integer
means an exact integer of magnitude at most 9,007,199,254,740,991. For zero-based
member k, sequence SHALL be `seq0 + k`, and time SHALL be `time0` plus the
first k gaps. Every member SHALL keep the stored `data.turn` and `data.step`;
its chunk index SHALL be the stored `data.index`. Every reconstructed
sequence and time SHALL remain in its respective safe range. Negative time
gaps SHALL preserve backwards clock movement, never reorder members. DSH `Turn.ts` SHALL be the event's signed base-ten epoch
millisecond integer string, without an exponent, timezone conversion or
padding; zero SHALL render as `"0"`, including a time recorded as negative
zero. An ordinary DSH event's time SHALL be interpreted only when it is a
signed safe integer; a missing or invalid value SHALL use an empty string.
Packed-row validation SHALL complete for the entire physical row before any
member can supply content or association. A valid JSON row violating this
encoding SHALL cause `unsupported-format` and count once in
`unrecognized_records`, never `skipped_lines`, even if early members are valid.

Each recognized readable `assistant/chunk` SHALL project as its own assistant
turn unless a complete readable assembly in the bounded snapshot proves it
is a source. Text/reasoning deltas retain each fragment as recorded, including
whitespace; an empty string supplies no block or turn. Tool-argument fragments,
block boundaries, usage and finish chunks SHALL remain recognized quiet
omissions, not assembled calls. Interrupted or live steps need no step-end or
assistant message before retained text/reasoning can be read.

Top-level `sourceEventSeqs` on a recognized `user/message`,
`assistant/message` or `tool/result` SHALL accept an array of individual
sequence integers and inclusive two-integer `[start, end]` ranges. Values
SHALL be nonnegative safe integers excluding negative zero, range start
SHALL not exceed end, and every cited value SHALL be less than the owning
event's sequence. A nonempty list SHALL require that owning sequence to be a
nonnegative safe integer excluding negative zero. An assembled
`assistant/message` SHALL suppress only earlier readable chunks it cites
within the same recorded `(data.turn, data.step)`; citations on a user message
or tool result SHALL not suppress chunks or replay a surface replacement.
A present invalid field (including null, a non-array, an invalid range, an
invalid owning sequence for a nonempty list, or a self/future reference)
SHALL cause `unsupported-format` and count its physical row once as
unrecognized. An absent field records no
association; `[]` explicitly cites no events. Neither SHALL suppress chunks.
A partial citation list SHALL suppress only its proved subset. Duplicate,
overlapping or out-of-order valid citation entries SHALL be treated as set
membership; they SHALL neither duplicate nor reorder content. These are
reader admission rules, not a claim to reproduce DSH's execution replay.

A citation proves a chunk only when exactly one earlier logical event has
that sequence in this file and its recorded turn/step pair matches the
assembly. A missing, ambiguous, cross-step or non-chunk target SHALL cause
no chunk suppression; gaps SHALL not be filled or searched outside the
bounded snapshot. A shared step alone, text equality, a timestamp, adjacency
or an inferred boundary SHALL not supply a missing citation. An assembly
occupies its own source position and time, including one marked interrupted;
uncited chunks and other steps' content retain their positions. An assembly
outside the source prefix or without readable projected blocks SHALL not
suppress chunks. Display capping SHALL not resurrect suppressed chunks.

Citation ranges SHALL be tested against bounded observed event identities
without expanding the integer interval or allocating space proportional to
its width. Packed decoding work and storage SHALL be bounded by actual
members of complete rows in the source prefix; no lookup, synthetic event
or memory allocation SHALL depend on an unobserved sequence gap. The source
cap counts encoded physical bytes, while the display cap and CLI/TUI turn
indices count final logical-event projections. An incomplete or source-cap
fragment of a packed row SHALL supply no members or associations. A complete
row can supply several turns, and the display cap can stop between members
without changing their individual text or numbering.

Dedicated call/result events SHALL own a duplicated embedded tool block only
on a proved matching recorded call id and turn/step; an unassociated embedded
complete call/result remains visible under the absent-partner rule. Message
text and reasoning blocks SHALL preserve their recorded order. Calls and
outputs SHALL retain names, identifiers, arguments and textual results under
the same inert-content rules as Codex. Request/context copies and recognized
lifecycle/accounting records SHALL supply no content. The read SHALL remain
an audit of retained events: `surfaceOp` compaction/replacement SHALL not
remove or reorder earlier messages, tools or uncited chunks. It SHALL not
substitute DSH's model-visible surface for the requested transcript.

Under version three the writer persists no fragment rows: its catalogue
has no `assistant/chunk`, a completed step is one `assistant/message`, and
a step cancelled mid-stream is one `assistant/message` with
`interrupted: true` whose message is the delivered prefix the writer
finalized. The rule that fragments are never concatenated is unchanged
and, under version three, has nothing to concatenate: the reader SHALL
assemble nothing under either version. `interrupted`, the embedded
`stream` and `usage` on `assistant/message` SHALL never be expanded into
fragments, duplicated as turns or rendered as prose, under any admission
of that row. `assistant/attempt`, a version-three record carrying a
stream with no surface message, SHALL be a counted omission: recognized,
contributing no content, adding one to `unrecognized_records`, and leaving
the read available. The text streamed in a failed or retried attempt is
therefore not part of the version-three audit read; under version zero
such deltas were chunk rows. `assistant/chunk` and the `text-chunks`,
`reasoning-chunks` and `tool-call-chunks` storage rows are version-zero
evidence: under a version-three header each SHALL be a required unknown
row, refused unless it carries top-level `ignorable: true`, and never
decoded by the version-zero fragment or packed-row path.

The message and block definitions behind version-three payloads were not
read. The admitting read cites the event map's envelope fields, content in
`data` for `user/message` and `data.message` for `assistant/message` and
`tool/result`, and not the message and block types that map imports,
whereas the version-zero projection of `text`, `reasoning`, `tool-call`,
`tool-result` and `image` blocks and of string content rests on the
0.1.2-rc.1 message and block definitions read for #222. A block name the
version-zero parser recognizes is not evidence that the same name carries
the same fields or meaning under version three, and counting an unknown
block does not measure a familiar one. Under version three a
`user/message`, `assistant/message` or `tool/result` row SHALL therefore
be a refused row: recognized as a content kind of the version-three
vocabulary, projected nowhere, causing `unsupported-format` and counting
once as unrecognized, whatever its payload carries, including string
content, familiar block names, `interrupted`, `stream`, `usage`,
`surfaceOp` or citations, and regardless of a top-level `ignorable`
marker, because that marker licenses omitting an operational record the
reader does not know, never dropping conversation content it cannot read.
No block of such a row SHALL be projected, suppressed, cited or
associated. `tool/call` under version three SHALL project as under version
zero, from the `callId`, `name` and `arguments` the event map carries on
the event itself; its recorded `turn` and `step` are read only for
association, which has no admitted version-three counterpart. This refusal
is lifted only by a change that records, from the installed writer's
source by package, file, digest and line, the version-three message and
block definitions the event map imports, maps every consumed nested field
(`content` as array or string, `text.text`, `reasoning.text`,
`tool-call.id`, `tool-call.name`, `tool-call.arguments`,
`tool-result.toolCallId`, `tool-result.content`, `tool-result.text`,
`image`) and every other block variant to its version-three meaning and
disposition, and pins each with synthetic scenarios naming exact counts
and any suppression.

The citation grammar above is the grammar the version-three writer emits,
byte-identical in its source, so `sourceEventSeqs` SHALL be validated
identically under both versions wherever it is validated; under version
three that validation reaches no row until the payload refusal above is
lifted, and suppression, which only ever removes cited earlier chunks,
would still remove nothing because no chunk row exists under version
three. `surfaceOp`, an append or replace marker on the writer's four
surface-eligible types, SHALL never remove, reorder or replace an earlier
row: the reader SHALL not replay the writer's model-visible surface, whose
own contract calls it the wrong source for a human transcript. Whether the
writer persists a replacement copy, a model-only row whose `surfaceOp` has
`op: replace`, as a message row of the session file, what such a row
carries, and how an append-origin row is told from it were not read,
because the persistence package and the append and replace paths were not
read. Keeping earlier rows is measured; excluding replacement copies is
not; they are separate operations. Until that path is read, a replacement
copy can reach the reader only as a version-three message row, which the
payload rule refuses, or as a `system/message`, which is quiet, and the
reader SHALL neither display nor suppress a version-three row on
`surfaceOp`. The change that lifts the payload refusal SHALL also rule,
from the persistence and surface source, whether replacement copies are
persisted and whether each displays, is omitted or refuses, with synthetic
append-plus-replacement and compaction scenarios pinning displayed rows,
order, duplicates and diagnostics. Under version zero the compaction and
replacement rules above are unchanged.

Dedicated call/result ownership of an embedded block, above, rests on a
unique recorded call id and turn/step. Whether a seeded version-three
session's inherited prefix keeps the originating `seq`, `turn`, `step` and
call identities of its rows, or remaps or reuses them, was not read.
Under version three that association SHALL not be applied: no message or
`tool/result` row projects, and a `tool/call` row has no embedded copy to
own. The change that lifts the payload refusal SHALL read the seed and
fork path and pin seeded scenarios in which inherited and newly emitted
embedded and dedicated calls and results suppress a genuine duplicate once
and keep unrelated or ambiguous identities, or SHALL leave the association
unapplied under version three.

Version-three admission changes nothing about time. An ordinary
version-three event's `time` keeps the signed-safe-integer rule above and
its existing exactness: a nonzero literal whose parsed value collapses to
zero is not an integer millisecond count, so it is invalid and renders an
empty stamp under either version, and a packed `time0` spelled that way
keeps its version-zero refusal. The recorded-token exactness checks of
this reader judge the header `version`, an ordinary event's `time` and a
packed row's `time0`, and no other field, under either version.

#### Scenario: A DSH step is assembled once
- **WHEN** a session has a user message, several assistant chunks, an assembled assistant message containing text and reasoning and citing every one of those same-step chunks, a tool call, its output and another assembled answer
- **THEN** the user message, assembled blocks, call, result and answer appear once in source order; the chunks do not duplicate the assistant's text

#### Scenario: An interrupted DSH step retains its chunks
- **WHEN** an owned session ends with two complete readable `assistant/chunk` records for one step, followed by a partial JSON append and no assembled message
- **THEN** the two chunks remain readable as two ordered assistant turns through CLI and both TUI doors, with no fabricated assembled answer, and the partial append is not a malformed complete line

#### Scenario: Assembly replaces only its own step's chunks
- **WHEN** a later bounded snapshot contains an assembled message citing all its step-one chunks and only readable chunks for step two within the same DSH turn
- **THEN** step one's chunks are replaced by its one assembled message and step two's chunks remain visible once in their source positions, without merging the two steps or changing journal accounting

#### Scenario: A call repeated inside a message is not another operation
- **WHEN** an assembled DSH message declares a tool call and the bounded stream also contains a complete dedicated `tool/call` record with that same recorded call id and turn/step
- **THEN** the dedicated event supplies the one displayed call and the message still supplies its other text and reasoning

#### Scenario: Missing reasoning and billing facts stay missing
- **WHEN** a DSH assembled message carries answer text but no readable reasoning
- **THEN** the answer appears without manufactured reasoning, and request settings or token counts are not substituted for reasoning text

#### Scenario: Packed and unpacked interrupted fragments produce identical turns
- **WHEN** an interrupted DSH file stores three text fragments in one `text-chunks` row at `seq0: 10`, `time0: 1000`, `dt: [-1, 5]` and three reasoning fragments in one `reasoning-chunks` row, and a second file stores the equivalent six ordinary events with no assembly
- **THEN** both reads produce the same six assistant turns, blocks and order, with the first three stamps `"1000"`, `"999"`, `"1004"`, zero diagnostic counts and no concatenation; either file's turn three is the third text fragment

#### Scenario: A packed tool-argument run is not a completed call
- **WHEN** a valid `tool-call-chunks` row records several raw argument fragments and the bounded snapshot contains no completed call
- **THEN** no tool call is invented, the row adds neither diagnostic count, and packing it or storing its equivalent ordinary chunks produces the same zero turns; a later complete dedicated call supplies its own turn

#### Scenario: An incomplete packed row supplies no partial members
- **WHEN** a source ends without a newline in the middle of a packed row after two syntactically complete member strings, or the 32 MiB source cap cuts that row there
- **THEN** neither member projects or supplies a citation target, neither count increases for that fragment, and only the source-cap case reports truncation; a later complete row is validated and decoded afresh

#### Scenario: Invalid packed encodings refuse rather than discard a run
- **WHEN** a complete valid JSON packed row has a wrong `dt` length, a non-string member, invalid or overflowing sequence/time reconstruction, or a key outside the specified shape
- **THEN** the entire DSH read returns `unsupported-format`, no turns, and one unrecognized physical row for that row with no malformed-line increment; setting `ignorable: true` on a malformed packed row does not bypass its storage validation

#### Scenario: Ranged citations suppress only observed members
- **WHEN** readable same-step chunks have sequence ids 10, 11, 12 and 14, and a later readable assembly at sequence 20 cites `sourceEventSeqs: [[10, 12]]`
- **THEN** chunks 10 through 12 are suppressed, chunk 14 remains at its source position and the assembly appears once at its later position; the result is the same for citations `[10, 11, 12]`

#### Scenario: Empty absent and partial citations preserve uncited content
- **WHEN** three otherwise identical snapshots have two readable same-step chunks at sequences 10 and 11 and a later readable assembly with respectively `[]`, absent `sourceEventSeqs`, or `[10]`
- **THEN** the first two retain both chunks followed by the assembly and the third retains chunk 11 followed by the assembly; a shared step or equal words cannot erase the uncited content

#### Scenario: Citation scope and ambiguous identities do not invent association
- **WHEN** an assembly cites a chunk in another step, a sequence shared by two earlier events, a non-chunk event or a sequence absent from the bounded snapshot
- **THEN** those citations suppress no chunk; valid unique citations to earlier chunks of the assembly's own turn/step still suppress only those chunks, without sorting or completing the event log

#### Scenario: Citation validation and large ranges have bounded outcomes
- **WHEN** a readable assembly has null or malformed citations, a reversed range, a non-integer endpoint or a self/future sequence
- **THEN** its row counts once as unrecognized and the read returns `unsupported-format` with no turns, not a guessed association
- **AND** a valid range `[0, 9007199254740990]` on an assembly at sequence 9007199254740991 instead tests only the earlier events actually present, using no allocation or iteration proportional to that interval's width

#### Scenario: Citation entry order does not become transcript order
- **WHEN** a complete DSH snapshot has unique readable same-step chunks at sequences 10, 11, 12 and 14 and a later readable assembly at sequence 20 with `sourceEventSeqs: [[12, 14], [10, 12], 11]`, with both budgets satisfied and no other diagnostic condition
- **THEN** the reader succeeds with only the assembly at its own position and timestamp, zero diagnostic counts and no notices, exactly as for citations `[10, 11, 12, 14]`; overlapping or out-of-order entries neither refuse this audit read nor duplicate content, and missing sequence 13 creates no event or lookup
- **AND** CLI whole output, `--turn 1` and the TUI show that same assembly; a live replacement of the four chunk turns clears the old selection and closes its overlay under the existing refresh rule, without validating or changing the provider's replay history

#### Scenario: Interrupted assembly and compaction keep the audit order
- **WHEN** a complete readable version-zero interrupted assembly cites some same-step chunks and a later version-zero message carries a surface replacement citing earlier messages
- **THEN** only the interrupted assembly's proved chunk sources disappear; uncited chunks, earlier messages and tool events retain audit order, and the later replacement message appears at its recorded position rather than rewriting history

#### Scenario: The display cap can stop between packed members
- **WHEN** a complete valid packed row yields a first text turn of exactly 4,000,000 UTF-8 bytes and a second nonempty text turn, while the encoded row fits the source cap
- **THEN** turn one is retained, turn two is not, `truncated` is true and diagnostics count no unknown record; the packed row is fully validated but is neither one oversized turn nor a means to exceed the display cap

#### Scenario: Version-3 message rows refuse until their payload definitions are read
- **WHEN** a version-three session holds a `tool/call` at sequence 2 followed by a readable-looking `user/message` with string content, and otherwise identical files instead end with an `assistant/message` carrying familiar `text` and `reasoning` blocks, `interrupted: true`, a `stream` array and a `usage` object, followed by a partial JSON append; an `assistant/message` at sequence 20 carrying `surfaceOp: {"op":"replace","startSeq":2,"endSeq":4}` and `sourceEventSeqs: [[3, 5], 7]`; a `tool/result` whose `tool-result` block names the call's `callId` with matching `turn` and `step`; or that `user/message` with top-level `ignorable: true`
- **THEN** every read returns `unsupported-format` with `DSH transcript format is not supported`, its confirmed path/hint, no turns, `unrecognized_records: 1`, `skipped_lines: 0`, `truncated: false` and exactly `unrecognized transcript records: 1`; no text, reasoning, tool block, image, fragment or interruption marker is projected, the `stream` yields no turn, no row is removed or reordered on `surfaceOp`, the citations are neither applied nor reported, the call's embedded copy is neither owned nor suppressed, the marker does not omit the row, and the partial append is not a malformed complete line
- **AND** a file holding the header and that `tool/call` alone projects the one call with zero counts, and the same four message rows under a version-zero header keep their version-zero projection

#### Scenario: A version-3 attempt is counted and shows nothing
- **WHEN** a version-three session holds a `tool/call`, an `assistant/attempt` row carrying a stream, and a second `tool/call`
- **THEN** the two calls project once each in source order, the attempt supplies no turn, block or text, `unrecognized_records` is one with its notice, and the read succeeds; the same attempt row with top-level `ignorable: true` yields the same result

#### Scenario: Version-zero fragment rows are unknown under a version-3 header
- **WHEN** a version-three session contains a `tool/call` followed by an `assistant/chunk` row, and three otherwise identical files instead contain a valid `text-chunks`, `reasoning-chunks` or `tool-call-chunks` row
- **THEN** each read returns `unsupported-format` with no turns, one unrecognized record and its notice, without decoding the row as a fragment or a packed run; with top-level `ignorable: true` on that row each read instead projects the call once with one unrecognized record

#### Scenario: Version-three time keeps the recorded-token exactness of version zero
- **WHEN** a version-three source whose header spells its version `3e0` holds three `tool/call` rows with `time` spelled `1e-400`, `1e3` and `-0.0`, and version-zero sources hold the existing ordinary `assistant/chunk` at `time: 1e-400` and packed `text-chunks` row at `time0: 1e-400`
- **THEN** the version-three header admits and its three calls project in order with stamps `""`, `"1000"` and `"0"`, zero counts and no notice; the version-zero chunk still renders an empty stamp and the version-zero packed row still refuses with one unrecognized record; no other field of any row is judged on its recorded token

### Requirement: Partial records and read failures remain distinguishable

The following row-classification rules SHALL apply to Claude/Codex bounded
snapshots and to DSH snapshots after admitted-version header admission,
each DSH row classified under the vocabulary of the version its opening
header carries. A rejected DSH header version SHALL instead retain the
zero-count state fixed by discovery and content admission above; it SHALL
not start this event-classification pass.

Complete malformed JSON lines SHALL be skipped without repairing them,
counted as `skipped_lines`, and reported by CLI/TUI with the exact notice
`malformed transcript lines skipped: <n>` when the count is positive.
Complete valid JSON physical rows with an unrecognized type, envelope or
content variant SHALL instead be counted in `unrecognized_records`, at most
once per physical row even when several blocks or logical events are
unrecognized. DSH's required-event and storage refusals below additionally
make that read unavailable; counting alone SHALL not imply success. A
partially recognized message SHALL retain its supported blocks and count that record
once for the unsupported portion. Valid JSON scalars and rows with no
recognizable envelope count as unrecognized, not malformed; the DSH marker
rule below decides whether such a row also refuses the read. For Claude, the
closed classification table in "Claude content preserves the existing
projection" SHALL determine every exemption; merely calling an unknown type
metadata SHALL not exempt it. For Codex/DSH, recognized headers (only the
admitted opening header for DSH), context/lifecycle/accounting records,
encrypted reasoning and proven duplicate representations SHALL count in
neither diagnostic. The exact notice
`unrecognized transcript records: <n>` SHALL appear when that count is
positive; neither notice SHALL quote an unknown or malformed payload.

A last line that is not yet valid JSON and lacks a newline SHALL be treated as an incomplete append, omitted until a subsequent
read completes it, and not counted as a malformed complete line. A complete
valid JSON final line SHALL be readable even without a trailing newline.
Invalid UTF-8 in consumed source bytes or a file I/O failure SHALL return
`unreadable`, not replacement prose; the explicit source-cap boundary exception
below still applies. Both counts SHALL describe all complete physical rows
examined within a successfully acquired and UTF-8-valid bounded source
snapshot admitted for decoding, before display capping and `--turn`
selection; neither counts an incomplete append or source-cap fragment.
Counts start at zero when no usable snapshot was acquired or DSH header
version admission failed.

For DSH, after recognizing the header and, under version zero, supported
packed storage rows, a valid JSON row whose event type/envelope is
unrecognized under the admitted version's vocabulary SHALL be omitted
successfully only when it is an object with top-level `ignorable` exactly
boolean `true`. Absent, false, null, string, numeric or nested markers
SHALL not permit omission; a scalar cannot carry that marker. Such a
required unknown row SHALL cause `unsupported-format`, with no projected
prose from anywhere in the snapshot. A recognized event whose payload is
admitted, every version-zero content kind and `tool/call` under version
three, with an unsupported nested content/block/chunk variant SHALL keep
the earlier supported-sibling and counted-omission rule; it is not an
unknown event envelope. Recognized quiet event kinds SHALL be enumerated per admitted
version from evidence in design, not inferred from a type prefix, from an
arbitrary claim that a record is metadata, or from the same name being
quiet under another version. The version-zero vocabulary is the captured
0.1.2-rc.1 catalogue: five content kinds, three packed storage rows and 46
quiet kinds. The version-three vocabulary is the read 0.1.5-rc.2 catalogue
as far as it was transcribed: one projecting content kind (`tool/call`),
three refused content kinds (`user/message`, `assistant/message` and
`tool/result`, whose payload definitions were not read), one counted
omission (`assistant/attempt`) and 51 quiet kinds, namely the 44
version-zero quiet kinds other than `tool/code-dispatch` and
`tool/code-dispatch-start` plus `system/message`,
`deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch` and
`tool/ptc-dispatch-start`. A refused version-three content row is not an
unknown envelope: it counts once as unrecognized and refuses the read
regardless of an ignorable marker, exactly as an invalid packed-row or
citation encoding does. The writer reports two further catalogue names
that were not transcribed; they, `assistant/chunk`, the three packed-row
names, the two `tool/code-dispatch*` names and every other name are
required unknowns under version three, and the eight version-three names
above are required unknowns under version zero. Invalid packed-row or
citation encodings SHALL also cause `unsupported-format` as specified
above, regardless of an ignorable marker.

On event/storage `unsupported-format` after header admission, the reader
SHALL keep the selected reference, legacy flag, confirmed path and its full-session hint, return no turns, and
use the explanation `DSH transcript format is not supported`. It SHALL
classify all complete physical rows of the usable bounded snapshot for both
counts, including rows before and after the offending row; each offending
valid-JSON row counts once as unrecognized. The read SHALL retain only
observed source-cap truncation: no display projection or display-cap flag is
claimed after semantic refusal. Notices SHALL use the same ordered strings
for that source-cap flag and the two complete-prefix counts. A complete
required unknown row after any would-be display cutoff SHALL still refuse
the read. Malformed JSON lines remain skipped/countable, and incomplete
appends remain provisional; neither proves an unknown required event exists.
No record outside the source prefix SHALL affect refusal or association.

Source I/O or consumed invalid UTF-8 SHALL take precedence over DSH header
or event/storage refusal. If either prevents a usable bounded snapshot,
the result SHALL be
`unreadable` with no turns, zero diagnostic counts and only source truncation
already established by the cap probe (and its notice); no partial semantic
scan or provider payload SHALL be exposed. A safely confirmed path/hint
SHALL remain available. If a usable snapshot exists, header-version refusal
SHALL precede event classification; either `unsupported-format` case SHALL
precede display capping and any `--turn` request, including an otherwise
in-range or out-of-range index. These checks SHALL execute no provider replay,
repair, resumption or journal mutation.

A readable file with no projected turns SHALL succeed with `no readable turns`,
distinct from unavailability and from a cap that retained no complete turn.
When `unrecognized_records` is positive, CLI text/JSON and the TUI pane SHALL
carry its count/notice; every open overlay of a readable projection SHALL
also carry it. A refused DSH snapshot SHALL keep its notices in the pane/error
output with both reading doors disabled. A successful zero-turn case SHALL
not appear as an empty recording with no explanation. For CLI/TUI, notices
SHALL be ordered: truncation, malformed-line count, unrecognized-record count, omitting only notices whose condition is
false. The existing Claude browser response retains its three-field envelope;
these added diagnostic fields belong to the explicit transcript CLI/TUI
result, not that compatibility response.

#### Scenario: A growing final line becomes a turn later
- **WHEN** an active transcript initially ends halfway through a JSON record and a later read sees that record completed
- **THEN** the first read omits the incomplete record and the later read displays it once, without a permanent parse failure or fabricated content

#### Scenario: Malformed records do not hide later valid content
- **WHEN** a complete malformed JSON line precedes a valid user message and an unknown lifecycle record that is explicitly `ignorable: true` for DSH
- **THEN** the user message remains readable, `skipped_lines` and `unrecognized_records` each equal one, and their notices report both omissions without quoting either payload

#### Scenario: Unrecognized records cannot masquerade as an empty session
- **WHEN** an owned readable file has a recognized session header followed by five thousand valid JSON records of an unsupported content type, each explicitly `ignorable: true` if it is a DSH unknown event
- **THEN** it succeeds with zero turns, `unrecognized_records: 5000`, `skipped_lines: 0` and `unrecognized transcript records: 5000` beside `no readable turns`; an empty file or one containing only recognized header/context records has both counts zero and no unknown-record notice

#### Scenario: Partial support remains visible without quoting unknown payloads
- **WHEN** one complete message contains readable text and two unsupported block variants, followed by one malformed complete JSON line
- **THEN** the text remains visible, `unrecognized_records` is one, `skipped_lines` is one, and the two counted notices disclose the omissions without echoing either unknown block or the malformed line

#### Scenario: Invalid bytes are not repaired
- **WHEN** consumed source bytes contain invalid UTF-8 before any source-cap boundary, even in a file without a trailing newline
- **THEN** the local result is `unreadable` and no replacement-character version of its prose is presented as the transcript

#### Scenario: An empty session is different from a missing file
- **WHEN** a valid retained file contains only headers and context records recognized for its kind
- **THEN** it returns zero turns without an unavailability reason, and a missing file instead returns `not-found`

#### Scenario: A required unknown DSH event refuses all prose
- **WHEN** an owned DSH snapshot contains visible user/assistant content followed by one unknown event with no `ignorable` marker
- **THEN** the whole read returns `unsupported-format`, no turns, the confirmed path/hint and selected reference, `unrecognized_records: 1`, `skipped_lines: 0`, `truncated: false` and exactly `unrecognized transcript records: 1`; it does not present the earlier content as a successful partial history

#### Scenario: Only an explicit true marker permits an unknown DSH event
- **WHEN** otherwise identical DSH snapshots give that unknown event top-level `ignorable` values true, false, null, `"true"`, 1, or omit the field
- **THEN** only boolean true succeeds with the recognized turns and one counted unknown row; every other case returns `unsupported-format` with no turns and the same count, and an unknown scalar or an event with only a nested true marker is likewise refused

#### Scenario: Known DSH event content keeps counted partial support
- **WHEN** a recognized version-zero DSH assistant message contains readable text plus two unsupported blocks and valid association metadata
- **THEN** the text remains readable, its one physical row adds one to `unrecognized_records`, and an absent ignorable marker does not turn the recognized envelope into an unknown required event

#### Scenario: A refused DSH snapshot keeps complete-prefix diagnostics
- **WHEN** a UTF-8-valid DSH source exceeds 32 MiB and its complete bounded rows contain one unknown required event between two malformed JSON lines and one later unknown ignorable event, followed by a cap-cut row
- **THEN** it returns `unsupported-format`, zero turns, `truncated: true`, `skipped_lines: 2`, `unrecognized_records: 2` and notices in truncation/malformed/unrecognized order, preserving its path/hint; the cap-cut row contributes no count or event

#### Scenario: An unconfirmed partial event cannot refuse a readable snapshot
- **WHEN** a readable DSH prefix ends in an incomplete unknown-event JSON append or a packed row cut by the source cap, and contains no complete semantic-refusal record
- **THEN** its retained complete content remains readable, the fragment adds neither diagnostic, and only the cap-cut case sets truncation; completing an unknown required event on a later read then produces `unsupported-format`

#### Scenario: Source failure outranks semantic refusal
- **WHEN** a DSH source contains an unknown required event but an I/O failure or invalid UTF-8 prevents a usable bounded snapshot
- **THEN** it returns `unreadable`, no turns and zero counts, keeps any safely confirmed path/hint and only already-established source-cap truncation/notice; it neither quotes partial content nor classifies the unknown event from an incomplete scan

#### Scenario: Rejected DSH header admission prevents event diagnostics
- **WHEN** a usable source below 32 MiB starts with an owned version-1 header followed by malformed JSON, invalid packed/citation rows, unknown events and readable-looking text whose display size would exceed 4,000,000 bytes
- **THEN** header admission returns `unsupported-format` with no turns, zero counts, `truncated: false` and no notices, keeping the reference/path/hint; subsequent rows supply neither diagnostics nor a display-cap flag because no event decoding began

#### Scenario: Rejected DSH versions retain only measured source truncation
- **WHEN** otherwise usable sources with rejected header versions end below or exactly at 33,554,432 bytes, or an extra-byte probe establishes additional source beyond that prefix
- **THEN** every read returns `unsupported-format`, no turns and zero counts with its confirmed path/hint; below or exactly at the cap `truncated` is false with no notices, while the extra-byte case has `truncated: true` and exactly `["transcript truncated (size cap)"]`

#### Scenario: Source failure outranks a rejected DSH header version
- **WHEN** an owned DSH source has a foreign or mistyped version but an I/O failure or consumed invalid UTF-8 prevents a usable bounded source snapshot
- **THEN** the result is `unreadable`, no turns, zero counts and only already-established source truncation and its notice, with any safely confirmed path/hint retained; it does not classify the remaining rows or report the header-version refusal instead

#### Scenario: A later DSH header cannot change format admission
- **WHEN** a usable owned source starts with version 1 and later contains a version-zero `session` row and readable-looking events
- **THEN** opening-header admission still returns `unsupported-format` with zero counts and no turns; the later header cannot restart decoding
- **AND** if the opening header is instead version zero or version three, a later `session` row is one unknown event: without top-level `ignorable: true` it causes R14's event refusal and one unrecognized row, while with that marker it is a counted omission and other recognized content remains readable, with no version switch in either direction

#### Scenario: Version-3 names are ruled under the version-3 vocabulary
- **WHEN** a version-three session holds a `tool/call` followed by one row of each type `system/message`, `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch`, `tool/ptc-dispatch-start`, `todo/write`, `turn/end` and `request/header`, none carrying an `ignorable` marker
- **THEN** the read projects the call once with zero diagnostic counts and no notice; every listed row is recognized quiet, and `system/message` supplies no `system` turn and no prose, whatever its payload or `surfaceOp`

#### Scenario: Version-3 names are unknown under version zero
- **WHEN** a version-zero session holds a readable `user/message` followed by one `assistant/attempt`, `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch` or `tool/ptc-dispatch-start` row without an `ignorable` marker
- **THEN** each read returns `unsupported-format` with no turns, one unrecognized record and its notice; with top-level `ignorable: true` on that row it projects the user message once with one unrecognized record, and no name is quiet under version zero because it is quiet under version three

#### Scenario: Version-zero-only names are unknown under version three
- **WHEN** a version-three session holds a `tool/call` followed by one `tool/code-dispatch` or `tool/code-dispatch-start` row, or by a row whose type is outside every transcribed version-three name, without an `ignorable` marker
- **THEN** each read returns `unsupported-format` with no turns, one unrecognized record and its notice, and with top-level `ignorable: true` on that row it projects the call once with one unrecognized record; an untranscribed catalogue name is refused by name, never admitted with the version
