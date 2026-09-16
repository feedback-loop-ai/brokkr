## MODIFIED Requirements

### Requirement: One transcript derivation serves the local readers

`brokkr-view` SHALL derive transcript content from supplied records into one
serializable `Turn` shape: `role` and `ts` strings and an ordered `blocks`
array whose elements contain `kind` and `text` strings. File discovery and
reading SHALL remain outside the pure view derivation. CLI and TUI SHALL
consume this same derivation; the existing Claude browser drill SHALL reuse
its Claude projection, and browser participant presentation SHALL consume
the shared reference eligibility and `full_session` result as specified below.
Prose SHALL belong to this explicit local transcript
read, never to the journal-derived run/participant models.

A physical source record SHALL be one complete JSONL row. A logical event
SHALL be the event represented by an ordinary row or one member decoded from
a supported DSH packed row. A displayed turn SHALL be one content-bearing
projection after per-kind duplicate suppression and the packed-chunk
coalescing rule in "DSH sessions expose assembled or provisional content
once" below. That rule is the sole exception to one logical event per
displayed turn; coalescing supplies a provisional chunk, not a provider's
assembled message. A displayed index SHALL not mean a physical line number,
journal checkpoint or provider's billable turn. Order SHALL be physical row
order, then stored member order within a packed row, never a sort by time,
sequence number or provider turn. Diagnostics SHALL count physical rows;
turn numbering and the display budget SHALL apply to the final displayed
turns, including coalesced chunks.
The timestamp SHALL be the recorded timestamp (DSH's numeric representation
is pinned below) or an empty string when absent; source order SHALL outrank
timestamp order.
Blocks SHALL use `text`, `reasoning`, `tool`, `tool-result`, or `omitted`.
User/assistant messages SHALL retain their roles; standalone reasoning and
calls SHALL have role `assistant`, and standalone tool outputs role `tool`.
Non-text media in the new Codex/DSH projections SHALL produce an `omitted`
marker naming the media class, without expanding binary data or fetching it.
No turn SHALL be emitted with no visible blocks.

Canonical preference SHALL be evaluated over the complete records in the
bounded source snapshot before applying the display budget. A canonical
logical event replaces only its proven fallback representations; it occupies
its own physical-row/member position and uses its own timestamp. Other
records retain their relative file order. No lookup beyond the source cap SHALL establish a
replacement, and display-capping a canonical record SHALL not restore an
earlier fallback. A subsequent read is a fresh projection, not an append to
the previous list of displayed turns.

#### Scenario: The same content reaches text and both TUI doors
- **WHEN** a selected seat has a readable transcript of any of the three supported kinds
- **THEN** the command and TUI receive the same ordered turns and blocks for the same file snapshot, and opening one turn yields exactly that turn's content from the whole transcript

#### Scenario: Display capping does not reinstate a superseded fallback
- **WHEN** the bounded source contains an event fallback and its associated canonical message, but the canonical turn would exceed the remaining display budget
- **THEN** canonical preference removes that fallback before display capping, the oversized canonical turn is not retained, and the truncation notice remains; the fallback is not shown instead to evade the display cap

#### Scenario: Display indices are not accounting
- **WHEN** a file contains a user message, a reasoning record, a tool call, its output and an assistant reply, with equal or missing timestamps
- **THEN** the five content-bearing records retain file order and their displayed indices do not alter the seat's journaled turn, usage, cost or model facts

#### Scenario: Unsupported media remains visible as an omission
- **WHEN** a Codex or DSH message contains text, an image and more text
- **THEN** the turns contain the text and an image-omitted block in that order without embedding image bytes, following a URL or inventing an image description

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
admit at most 65,536 bytes of each DSH candidate's first-record header,
excluding its terminating newline when present. A byte beyond that bound
SHALL only detect a newline or overflow, never become admitted header content.
The same limit SHALL hold at true EOF without a newline: 65,537 header bytes
SHALL return `discovery-limit` even if those bytes form valid JSON and no
further source byte exists. This enforces the existing limit, not a new
header-format restriction.
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
is closed and is exactly `{0, 3}`. Admission SHALL be judged on the
recorded `version` token and its parsed value together, for zero and for
three alike. A recorded number token spells an admitted integer exactly
when its parsed value is that integer and its digit signature, the
mantissa digits with the decimal point and every leading and trailing zero
removed, is that integer's digits: empty for zero, `3` for three. So `0`,
`0.0`, `0e0`, `0.000` and `-0` SHALL denote version zero, and `3`, `3.0`,
`3e0`, `30e-1` and `0.3e1` SHALL denote version three. `-3` and `3e1`
share the signature `3` and SHALL refuse on their parsed value; `3.1`, the
rounding artefacts `2.9999999999999999` and `3.0000000000000001`, whose
parsed value is three, and every token whose mantissa carries a nonzero
digit yet parses to zero, such as `1e-400`, `10e-400`, `0.1e-400` and
`1.0e-400`, SHALL refuse on their signature. A zero digit somewhere in a
mantissa is not a zero spelling. The shipped reader admitted `10e-400`
and `0.1e-400` as version zero because a zero digit appeared in them;
this change corrects that judgment toward the rule instead of preserving
it, and the correction applies under version zero as well as three.

The recorded token SHALL be bound to the member the JSON parser reads.
The token of a top-level member is established only when the header
records exactly one top-level member whose name, after JSON string-escape
decoding, is `version`; that member's raw value is the token, so an
escaped name such as `"\u0076ersion"` binds to `version` and its token is
judged like any other. A header that records `version` more than once at
its top level, under literal or escaped names and whatever the values,
even two exact and equal spellings, is an ambiguous recording and SHALL
return `unsupported-format`. A header whose parsed `version` is a number
but whose recorded token cannot be established SHALL likewise refuse: the
recorded-token check SHALL never fall open to the parsed value alone.
Admitting three widens the recorded-token version check from one admitted
integer to two and touches no other field: an ordinary event's `time` and
a packed row's `time0` keep the exactness the content rules below already
require of them under either version, their parsed value a signed safe
integer, a parsed zero a zero spelling only when its token's digit
signature is empty, a nonzero integer judged on its parsed value alone,
and every judged token established by the same one-member binding; no
citation, marker or other event field is judged on its recorded token
under either version. Strings,
booleans and null SHALL not be coerced. Missing `version`, any nonnumeric
value and any numeric value outside the set, including 1, 2 and 4, SHALL
return `unsupported-format` with `DSH transcript format is not supported`.
There is no legacy default version or automatic migration. An omitted
depth still means zero under the settled ownership rule; a missing version
does not.

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
grammar; the message and block definitions the event map imports for the
payloads of `user/message`, `assistant/message` and `tool/result`, mapped
field by field onto the reader's projection; the persistence codec's
header shape, physical envelope (a base of four members on every event row
plus two conditional top-level members, recorded in that change's design
with each per-type permission's standing, measured or unread, and
summarised in the content rules below), row admission and seed-marker
consistency; and the catalogue and migration
packages. It did not reach the writer's append and replace paths, which
decide what a replace-marked row carries, the seed and fork path that
writes a seeded session's inherited prefix, or the permission of
`sourceEventSeqs` on any type but `assistant/message`, which no reader
rule consults and which this requirement records as unread, stating
none, with the read that records it, the member's declaring type and the
codec's per-type validation branch, named in that change. Under version
three the four content kinds therefore
project by the content rules below; a replace-marked row projects by its
type at its recorded position because the reader parses no marker; and the
one rule that rests on inherited identities, the dedicated-tool
association, runs only for a session whose header records `isSeeded`
exactly `false`. Versions 1 and 2 are not admitted:
no core of this fleet wrote them to disk under an admitted name and their
codecs were not read. A version SHALL join the set only through a change
that reads the writer's own source and records, cited by package, file,
digest and line, what that read established of each of the following
and, for each it did not reach, that it was not reached and the specific
read that would: the writer's package identity and files; the record
vocabulary it emits; the message and block definitions behind every
payload it projects, mapped field by field; how streaming fragments
persist; the meaning of every header field it adds; the physical
envelope, and the meaning of `surfaceOp` and `sourceEventSeqs` where the
read found them; the identity of a seeded session's inherited rows; a
disposition for every record type; and whether every admitted-version
meaning is preserved or a per-version projection is required. A payload
or part of the writer such a change did not reach SHALL be left refused
or withheld, with the specific further read that lifts it named, rather
than be admitted with its version: a rule that rests on it is withheld,
and a part on which no rule rests is recorded as unread. The version is
admitted on the parts read, and that admission is complete, not
incomplete: this requirement demands no inventory of a part the change's
reads did not reach and states no meaning or permission the change did
not record. A versioned filename, a header number and the
resemblance of sampled rows to admitted shapes are not admission
evidence. The filename and the header
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
rows was not read for version three, so the one rule that rests on those
identities is withheld where they could be inherited: under version three
the dedicated-tool association of the content rules below SHALL run only
when the opening header records `isSeeded` as boolean `false`, and every
other value, including `true`, a string, an object, null and absent, SHALL
leave each embedded copy visible beside its dedicated event. That gate
changes no row's classification, count or admission and adds no notice;
under version zero the field has no effect. Extra fields SHALL be ignored, never
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

#### Scenario: A zero digit inside an underflowing token is not a zero spelling
- **WHEN** a unique safe candidate's opening `session` object has depth zero and version `10e-400`, `0.1e-400` or `1.0e-400`, each a nonzero literal whose parsed value collapses to zero and whose mantissa contains a zero digit, and otherwise identical sources spell the version `0`, `0.0`, `0e0`, `0.000`, `-0` or `-0.0`
- **THEN** each of the first three returns `unsupported-format` with its confirmed path and hint, no turns, zero counts, `truncated: false` and no notices, because its digit signature is `1` and not empty, exactly as `1e-400` refuses; each genuine zero spelling is admitted as version zero and reads its rows under the version-zero vocabulary; the parsed zero alone admits nothing

#### Scenario: The version token is bound to the one decoded member
- **WHEN** a unique safe candidate's opening header is `{"type":"session","\u0076ersion":3,"delegationDepth":0}`, a second's is that header with the value `3.0000000000000001`, and four more record the member twice at their top level: `"version":3` then `"version":3.0000000000000001`, that pair in the reverse order, `"version":3` then `"\u0076ersion":3`, and `"version":0` then `"version":3`, every source complete and below both caps
- **THEN** the first is admitted as version three, because the escaped name decodes to `version` and its one token spells exactly `3`; the second returns `unsupported-format`, because the decoded member binds and its token is a rounding artefact, so the parsed three cannot admit alone; each of the four duplicate headers returns `unsupported-format` as an ambiguous recording, whichever occurrence the parser retains and even where both spellings are exact and equal; every refusal keeps the confirmed path and hint, no turns, zero counts, `truncated: false` and no notices, and no later row rescues it

#### Scenario: DSH admits only the declared version-three header shapes
- **WHEN** a unique safe DSH source begins with `{"type":"session","version":3,"delegationDepth":0,"isSeeded":false}`, or the equivalent numeric version `3.0` or `3e0`, with depth zero or omitted depth, followed by version-three quiet rows and readable content rows within both caps
- **THEN** each source is admitted under the version-three vocabulary and projects those content rows once each with its confirmed path/hint and zero diagnostic counts; absent or mistyped unused header metadata adds no gate or notice, and a file containing only such a header at EOF without a newline is a readable zero-turn source

#### Scenario: A version-3 session is located and projected
- **WHEN** the unique safe DSH candidate is `session.v3.jsonl` whose opening row is `{"type":"session","version":3,"delegationDepth":0,"isSeeded":false,"id":"s","createdAt":"2026-09-13T00:00:00Z","cwd":"/w"}`, followed by complete rows typed `system/message`, `todo/write`, `turn/end`, a readable `user/message` whose `data` carries `id`, `role`, `content` and `source`, a `tool/call`, its `tool/result` and a readable `assistant/message` carrying `surfaceOp: "append"`, `stream` and `usage`, within both caps, with discovery complete and no I/O or UTF-8 failure
- **THEN** discovery resolves that path and the read projects the user message, the call, the result and the assistant message once each in source order with zero diagnostic counts and no notice; the three quiet rows, the header's `isSeeded`, `id`, `createdAt` and `cwd`, the message's `id` and `source` and the assistant row's `surfaceOp`, `stream` and `usage` supply no content
- **AND** CLI text, CLI JSON and the TUI show that same projection, and a `--turn` request reaches each of those four turns

#### Scenario: The filename and the header version are independent facts
- **WHEN** one recorded root's only session directory holds `session.jsonl` whose opening header has version 3 and depth zero followed by a readable `user/message`, another's holds `session.v3.jsonl` whose opening header has version 0 and depth zero followed by a readable `user/message`, and a third's holds `session.v3.jsonl` whose opening header has version 4 and depth zero
- **THEN** the first projects its message under the version-three vocabulary and the second its message under the version-zero vocabulary, each with zero counts and no notice, and the third returns `unsupported-format` with its confirmed path and zero counts; neither name infers a version and neither version infers a name

#### Scenario: The versioned name beside the plain name is decisive, whatever its version
- **WHEN** one session directory holds `session.v3.jsonl` with a version-3 header followed by a readable `user/message` and `session.jsonl` with a version-zero header followed by a readable `user/message` with different text, and another directory holds `session.v3.jsonl` with a version-4 header beside `session.jsonl` with a version-zero header and a readable `user/message`
- **THEN** the first directory reads `session.v3.jsonl` and projects the versioned file's message under the version-three vocabulary, never the plain name's, and the second returns `unsupported-format` with the confirmed `session.v3.jsonl` path, zero counts and no turns; neither directory is ambiguous with itself, and the plain name is never read once the versioned name carries a valid session header, because falling through would present superseded evidence as the seat's newest transcript

#### Scenario: A seeded header changes neither ownership nor admission
- **WHEN** the unique safe candidate's opening header has depth zero or omitted depth and `isSeeded` true, false, a string, an object, null or absent, followed by a readable `user/message` under version 0 and again under version 3
- **THEN** every source is admitted and projects that message with zero counts and no notice, no seeded value adds a gate, a notice, a claim about provenance or a change to any row's classification or count, and the reader claims nothing about an inherited prefix; whether an embedded tool copy is suppressed under version three is the content rules' seeded-association scenario, not an admission fact
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

#### Scenario: A newline-less DSH header fits exactly at the discovery limit
- **WHEN** an otherwise unique safe DSH candidate has a valid depth-zero version-zero or version-three opening session header of exactly 65,536 UTF-8 bytes and ends there without a newline
- **THEN** discovery admits that candidate and the read succeeds with zero turns, zero diagnostic counts, no truncation and its confirmed path/hint
- **AND** the identical header followed by a newline also fits; that delimiter is not a header byte

#### Scenario: One extra header byte refuses even at EOF
- **WHEN** the same otherwise valid candidate's header is 65,537 UTF-8 bytes, with no newline and no byte after it
- **THEN** discovery returns exactly `discovery-limit`, null path and DSH full-session hint, no turns, zero counts, `truncated: false` and no notices, rather than success, `not-found`, `unreadable` or body-stage truncation
- **AND** the over-limit header with a terminating newline has the same refusal

### Requirement: DSH sessions expose assembled or provisional content once

The rules of this requirement are the version-zero rules, measured on the
captured 0.1.2-rc.1 writer, except where a paragraph names version three;
the version-three paragraphs below say what version three changes, on
which of its writer's definitions each version-three row projects, and
which single rule is withheld because the path it rests on was not read.

DSH SHALL read retained `user/message`, assembled `assistant/message`,
`tool/call` and `tool/result` events in source order, including readable
reasoning in assembled messages. It SHALL support ordinary event rows and
`text-chunks`, `reasoning-chunks` and `tool-call-chunks` storage rows, including
files mixing both encodings. Packed rows SHALL validate and retain their
logical member identities before content projection; they SHALL not be
classified as unknown events or treated as completed assistant messages.
Their displayed chunks SHALL follow the coalescing rule below.
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

Each recognized readable ordinary `assistant/chunk` row SHALL project as its
own assistant turn unless a complete readable assembly in the bounded
snapshot proves it is a source. Packed text/reasoning members SHALL instead
follow this single coalescing rule: after member-level citation suppression,
each maximal consecutive run of unsuppressed members within one validated
`text-chunks` or `reasoning-chunks` physical row SHALL yield one assistant
chunk turn, with one block of that row's kind and the member texts
concatenated exactly in stored order. The row records one common
`(data.turn, data.step, data.index)` and kind for that run. No separator,
whitespace normalization, completed-message status or invented text SHALL be
added. Empty members supply no text or turn and do not split a run; a
suppressed readable member splits it, so surviving members on opposite sides
remain separate chunks. An all-empty or entirely suppressed run supplies no
turn. The chunk's position and timestamp SHALL be those of its first
surviving nonempty member; later timestamps, including backwards clock
movement, SHALL not reorder text or create new turns.

Coalescing SHALL NOT cross a physical packed-row boundary, include an
ordinary event row, combine text with reasoning, or combine different
recorded turn, step or chunk index values. This deliberately changes the
former packed/plain turn-count equivalence: equivalent encodings keep their
content and citation outcomes, but ordinary rows keep individual turns and
packed rows expose the coalesced chunks. It bounds per-token packed
expansion using recorded grouping, without inferring a whole provider
message from adjacency. Text/reasoning deltas retain every surviving
fragment's bytes, including whitespace. Tool-argument fragments, block
boundaries, usage and finish chunks SHALL remain recognized quiet omissions,
not assembled calls. Interrupted or live steps need no step-end or assistant
message before retained text/reasoning can be read.

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
its width. Packed decoding work and identity storage SHALL be bounded by actual
members of complete rows in the source prefix; no lookup, synthetic event
or memory allocation SHALL depend on an unobserved sequence gap. Coalescing
SHALL NOT erase member identities or make a citation suppress a whole run
when it proves only a subset. Quiet and empty members SHALL still contribute
their sequences to ambiguity checks. The source cap counts encoded physical
bytes, while the display budget charges final turns as specified below and
CLI/TUI indices address those turns. An incomplete or source-cap fragment of
a packed row SHALL supply no members or associations. A complete row can
supply several chunks when suppression splits its runs, and the display cap
SHALL stop only between final turns, never split a coalesced chunk to make it
fit. Full-row validation and whole-prefix classification SHALL still precede
any successful capped result.

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
finalized. The reader SHALL project such a message as recorded, at its own
position and time, with no interruption marker, notice or block, exactly
as an interrupted version-zero step shows its retained fragments without
one. Version-zero packed-chunk coalescing does not apply under version
three, which has no admitted fragments to concatenate: the reader SHALL
invent no completed message under either version. `interrupted`, the embedded
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

The message and block definitions behind version-three payloads were read
for #279: the session event map imports its message and block types from
the 0.1.5-rc.2 `dsh-llm` package, whose files are cited by digest and line
in the change's design, and every field the reader consumes maps onto the
version-zero projection. `user/message` carries its message directly in
`data`, which is `{id, role, content, source}` with no `turn` or `step`;
`assistant/message` and `tool/result` carry theirs in `data.message`;
`content` is an array of blocks or a string; the block union is `text`
(`text`), `reasoning` (`text`), `image` (`attachment`), `tool-call` (`id`,
`name`, `arguments`), `tool-result` (`toolCallId`, `content`) and `file`.
Under version three the four content kinds SHALL therefore project exactly
as under version zero: ordered `text` and `reasoning` blocks, string
content as one text block, `tool-call` and `tool-result` blocks with their
identifiers, names, arguments and textual results, `image` as the inert
omission marker, and `file`, which the version-zero parser does not name,
as a counted unrecognized block that keeps its siblings under the
supported-sibling rule. `Message.id`, `Message.source`, `error` and `meta`
on `tool/result`, and `interrupted`, `stream` and `usage` on
`assistant/message` SHALL supply no content. `tool/call` projects from the
`callId`, `name` and `arguments` the event map carries on the event. The
version-three `user/message` definition declares no `turn` or `step`, so
under version three the reader SHALL read no position from a
`user/message`, even when its `data` supplies members of those names: a
field the writer's definition does not declare is not the writer's
identity. A version-three `user/message` therefore has no positions and
SHALL never own or be owned in the dedicated-tool association, whatever
its `data` carries and whatever the header records; under version zero
its positions are read as today. A familiar block or field name is not
what admits these payloads; the field-by-field mapping is, and a further
version SHALL be admitted only on its own mapping.

The citation grammar above is the grammar the version-three writer emits,
byte-identical in its source, so `sourceEventSeqs` SHALL be validated
identically under both versions wherever the version-zero rules validate
it, including the whole-read refusal for a present invalid field. Under
version three a valid citation SHALL suppress nothing: suppression only
ever removes cited earlier chunks, no chunk row exists under version three,
and the writer never places a citation on `assistant/message`; a
version-three citation on a user message or tool result to an earlier
row, to an unobserved sequence, or from an assembled message is the
existing no-suppression outcome, not a refusal. The version-three physical
event envelope, as the codec read for #279 validates it, is a base of
`type`, `seq`, `time` and `data` on every event row plus two conditional
top-level members: `surfaceOp`, required on exactly the four
surface-eligible types `system/message`, `user/message`,
`assistant/message` and `tool/result` and forbidden on every other type,
and `sourceEventSeqs`, forbidden on `assistant/message` and validated
wherever the codec finds one. The writer's permission of
`sourceEventSeqs` on each other type was not read for #279: this
requirement states none, demands no inventory of them, rests no rule on
one, and leaves them unread pending the read named above; the reader's
outcome for a row is the same whether the writer permits, requires or
forbids the member on that row's type, which the two scenarios below pin
across every disposition class, the content kinds and the quiet,
counted-omission and required-unknown rows alike. A
top-level packed row, which carries `seq0` and `time0` in place of `seq`
and `time`, is outside every type's permitted set and cannot be written
by that codec. Writer validity is not reader admission: the reader SHALL
validate no event row's key set, SHALL neither require a marker where the
writer requires one nor refuse a row for a marker the writer forbids, and
SHALL read `sourceEventSeqs` only on the three content kinds whose
version-zero rules read it, including on an `assistant/message`, where
the writer forbids the member and the reader still validates it, and on
a `user/message` or `tool/result` whatever the writer's permission there.
`surfaceOp`, that append or replace marker,
SHALL never be parsed and SHALL never remove, reorder or replace an
earlier row: the reader SHALL not replay the writer's model-visible
surface, whose own contract calls it the wrong source for a human
transcript. A row whose `surfaceOp` is a replace object SHALL project by
its type at its recorded position, a message as a message and a
`system/message` as quiet, exactly as version zero's compaction message
appears at its recorded position rather than rewriting history. The
writer's append and replace paths, which decide what such a row carries,
were not read; that gap changes no disposition, because the reader reads
no marker and reconstructs nothing, and a later change that reads those
paths may add a marker-aware presentation but SHALL never remove a
persisted row. Under version zero the compaction and replacement rules
above are unchanged.

Dedicated call/result ownership of an embedded block, above, rests on a
unique recorded call id and turn/step, which the version-three writer
carries on `assistant/message`, `tool/call` and `tool/result` and on the
`tool-call` and `tool-result` blocks. Whether a seeded version-three
session's inherited prefix keeps the originating `seq`, `turn`, `step` and
call identities of its rows, or remaps or reuses them, was not read, and
that prefix is the one way rows from another session enter a file. Under
version three the association SHALL therefore run only when the opening
header records `isSeeded` as boolean `false`; when it records anything
else, every embedded call or result copy SHALL remain visible beside its
dedicated event under the absent-partner rule, with no suppression, no
count and no notice. When it runs, the version-zero rule applies unchanged:
one proved matching call id and turn/step suppresses the embedded copy
once, and a missing, ambiguous or cross-step identity keeps both. Under
version zero the association runs whatever `isSeeded` carries. A change
that reads the seed and fork path SHALL lift the gate only with seeded
scenarios in which inherited and newly emitted embedded and dedicated
calls and results suppress a genuine duplicate once and keep unrelated or
ambiguous identities.

Version-three admission adds no rule about time. An ordinary
version-three event's `time` keeps the signed-safe-integer rule above and
its exactness: a nonzero literal whose parsed value collapses to zero is
not an integer millisecond count, so it is invalid and renders an empty
stamp under either version, and a packed `time0` spelled that way keeps
its version-zero refusal. A nonzero integer `time` or `time0` is judged
on its parsed value alone under either version, so `1e3` and `1000.0`
are the millisecond `1000`; the digit signature below decides only
whether a parsed zero is a zero spelling. This change does correct how the reader judges
that exactness, under both versions: a recorded token is a zero spelling
only when its digit signature is empty, so `10e-400`, `0.1e-400` and
`1.0e-400`, which the shipped reader accepted as zero because a zero
digit appeared in their mantissas, are invalid exactly as `1e-400` is; and
a `time` or `time0` token is established only from exactly one top-level
member whose decoded name is that field's, so a row that records `time`
more than once has no valid time and renders an empty stamp, a packed row
that records `time0` more than once refuses, an escaped member name binds
like a literal one, and a parsed number whose token cannot be established
is invalid rather than accepted on its parsed value. The recorded-token
exactness checks of this reader judge the header `version`, an ordinary
event's `time` and a packed row's `time0`, and no other field, under
either version.

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
- **WHEN** each packed text/reasoning row has only one nonempty member, with no suppressing assembly, and another complete source uses its equivalent ordinary chunk rows, with both budgets satisfied
- **THEN** both sources retain identical turns and stamps because each coalesced run has only one visible member; the following scenario replaces the former general equivalence for multi-member runs

#### Scenario: Packed interrupted fragments coalesce while ordinary rows retain their boundaries
- **WHEN** a version-zero file stores texts `["a", " ", "b"]` in one `text-chunks` row at `seq0: 10`, `time0: 1000`, `dt: [-1, 5]`, followed by one reasoning row with `["r", "e", "s"]` at time 2000, with no assembly and both budgets satisfied
- **THEN** it produces two assistant turns: one text block `a b` stamped `"1000"` and one reasoning block `res` stamped `"2000"`, with zero diagnostic counts and no notices
- **AND** the equivalent six ordinary chunk rows still produce six turns and their original stamps, including `"1000"`, `"999"` and `"1004"` for the text rows; coalescing changes packed indices, not source bytes or accounting facts

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
- **THEN** the first two retain both fragments in the chunk or chunks defined by the coalescing rule, followed by the assembly, and the third retains chunk 11 followed by the assembly; a shared step or equal words cannot erase the uncited content

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
- **AND** CLI whole output, `--turn 1` and the TUI show that same assembly; a live replacement of the previously displayed chunks clears the old selection and closes its overlay under the existing refresh rule, without validating or changing the provider's replay history

#### Scenario: Interrupted assembly and compaction keep the audit order
- **WHEN** a complete readable version-zero interrupted assembly cites some same-step chunks and a later version-zero message carries a surface replacement citing earlier messages
- **THEN** only the interrupted assembly's proved chunk sources disappear; uncited chunks, earlier messages and tool events retain audit order, and the later replacement message appears at its recorded position rather than rewriting history

#### Scenario: The display cap can stop between packed members
- **WHEN** a packed row has readable members at sequences 10, 11 and 12, a later readable same-step assembly uniquely cites member 11, member 10's text is 3,999,488 UTF-8 bytes and member 12 has nonempty text
- **THEN** suppression separates the surviving members into two chunks, member 10's chunk fits exactly, and the cap stops before member 12's chunk with truncation and no unknown-record diagnostic; it never splits a coalesced chunk
- **AND** without the citation splitting the run, the entire coalesced chunk is over budget and none of it is retained, as the next scenario pins

#### Scenario: A packed chunk is indivisible under the charged display budget
- **WHEN** a complete valid text packed row has two unsuppressed members whose concatenated text is 3,999,489 UTF-8 bytes and the encoded row fits the source cap
- **THEN** their one chunk costs 4,000,001 display accounting bytes, no turn is retained, `truncated` is true with the shared size-cap notice, and no diagnostic count increases
- **AND** reducing their combined text by one byte retains the whole one-chunk turn at equality with no truncation when no other content follows; the reader never retains only the first member to evade the new chunk boundary

#### Scenario: Version-3 content projects on the writer's own definitions
- **WHEN** a version-three session holds a `user/message` whose `data` is `{id, role, content, source}` with `content` an array of one `text` block, a `tool/call`, its `tool/result` whose `data.message.content` holds one `tool-result` block naming the call's `callId` with textual content, and an `assistant/message` whose `data.message.content` holds `reasoning`, `text`, `image` and `file` blocks in that order with `surfaceOp: "append"`, a `stream` array and a `usage` object, and an otherwise identical file whose user message carries string `content`
- **THEN** each read projects four turns in source order: the user text, the call with its name and arguments, the result with its identifier and text, and the assistant turn with its reasoning, text and `[image omitted]` blocks in recorded order; the `file` block adds one to `unrecognized_records` with its notice and removes no sibling, the string content projects as one text block, `id`, `source`, `stream`, `usage` and `surfaceOp` supply nothing, and no row refuses
- **AND** the same rows under a version-zero header project identically, and a version-three `user/message` carrying `turn` and `step` beside its message neither gains nor loses content

#### Scenario: A version-3 interrupted step retains the writer's finalized message
- **WHEN** an owned version-three session ends with a readable `assistant/message` carrying `interrupted: true`, a `stream` array, a `usage` object and text equal to the delivered prefix, followed by a partial JSON append and no further row
- **THEN** the read projects one assistant turn with exactly that text at the message's own position and time, with no interruption marker, no fragment turns derived from `stream`, zero diagnostic counts and no notice, and the partial append is not a malformed complete line; CLI whole output, `--turn` and both TUI doors show that one turn

#### Scenario: A version-3 attempt is counted and shows nothing
- **WHEN** a version-three session holds a readable `user/message`, an `assistant/attempt` row carrying a stream, and a readable `assistant/message`
- **THEN** the user and assistant messages project once each, the attempt supplies no turn, block or text, `unrecognized_records` is one with its notice, and the read succeeds; the same attempt row with top-level `ignorable: true` yields the same result

#### Scenario: Version-3 citations share the grammar and suppress nothing
- **WHEN** a version-three `user/message` at sequence 20 cites `sourceEventSeqs: [[3, 5], 7]` naming earlier `system/message`, `assistant/message` and `tool/call` rows, a `tool/result` cites an unobserved sequence, an `assistant/message` cites earlier rows, and otherwise identical files instead give the user message `sourceEventSeqs: null`, a reversed range or a self reference
- **THEN** the first three project every content row once in source order with zero diagnostic counts, removing none of the cited rows, and each of the others returns `unsupported-format` with one unrecognized row and no turns

#### Scenario: A version-3 replacement copy keeps the audit order
- **WHEN** a version-three session holds a readable `user/message`, a readable `assistant/message`, a `system/message` carrying `surfaceOp: {"op":"replace","startSeq":1,"endSeq":2}`, and a later readable `assistant/message` carrying the same replace marker over the first two rows and text unlike theirs
- **THEN** the read projects three turns in source order, the two earlier messages and the later replace-marked message at its recorded position, with zero diagnostic counts and no notice; the `system/message` is quiet whatever its marker, no earlier row is removed, reordered or replaced, and neither marker is parsed, displayed or counted

#### Scenario: Writer-required and writer-forbidden members are not reader admission
- **WHEN** a version-three session holds a readable `user/message` without `surfaceOp`, which the writer requires there; a `tool/call` carrying `surfaceOp: "append"`, which the writer forbids there, and `sourceEventSeqs: null`, whose permission on that type the read did not record; a readable `tool/result` carrying a valid `sourceEventSeqs` to earlier rows, whose permission on that type the read did not record; an `assistant/message` carrying a valid `sourceEventSeqs` to earlier rows, which the writer forbids there; and a `step/end` row carrying `sourceEventSeqs: null`; and two otherwise identical files give the `assistant/message`, respectively the `tool/result`, `sourceEventSeqs: null` instead
- **THEN** the first read projects the user message, the call, the result and the assistant message once each in source order with zero diagnostic counts and no notice, because the reader validates no row's key set, requires no marker, refuses no marker and reads a citation only on the three content kinds, so the members on the `tool/call` and the quiet `step/end` are never read; each of the other two returns `unsupported-format` with one unrecognized record and no turns, because the citation rules validate the member on an assistant message and on a tool result wherever it appears; no outcome in this scenario depends on whether the writer permits, requires or forbids the member on the row's type

#### Scenario: A member on a type whose writer permission was not read changes no disposition
- **WHEN** a version-three session holds a readable `user/message` at sequence 1 carrying no `sourceEventSeqs` member, then a `todo/write` at sequence 2 and a `deliverables/presented` at sequence 3, those two each carrying `sourceEventSeqs: null`, and an `assistant/attempt` at sequence 4 carrying a stream and `sourceEventSeqs: [[5, 1]]`; a second file appends a `text-chunks` packed row carrying `sourceEventSeqs: null`; a third appends instead an `assistant/chunk` row carrying `sourceEventSeqs: null` and top-level `ignorable: true`; and a fourth is the first file with `sourceEventSeqs: null` added to the user message and nothing else changed
- **THEN** the first read projects the user message once with one unrecognized record, the attempt's, and its notice, the two quiet rows counting in neither diagnostic; the second returns `unsupported-format` with two unrecognized records and no turns, refused on the packed row's type alone; the third projects the user message once with two unrecognized records; and the fourth returns `unsupported-format` with two unrecognized records, the user message's and the attempt's, and no turns, because the citation rules validate the member on a user message wherever it appears; the fourth file is the negative control for the other three: the same `null` member that refuses the read on the user message is carried by the quiet, counted-omission and required-unknown rows of every file, as the attempt carries a reversed range that the citation scenario above refuses on a user message, and none of those members is read, because such a row supplies only its sequence and its ignorable marker, and the writer's permission of the member on those types, which the read for #279 did not record, is no rule's premise

#### Scenario: Embedded copies are owned only in an unseeded version-3 session
- **WHEN** a version-three session whose header records `isSeeded: false` holds an `assistant/message` at turn 1 step 1 embedding text and a `tool-call` block with id `c1`, a dedicated `tool/call` with `callId: "c1"` at turn 1 step 1 and its `tool/result`; and otherwise identical files record `isSeeded` as `true`, a string, an object, null or absent
- **THEN** the first read projects the assistant text without its embedded call block, then the dedicated call once and the result once, with zero counts and no notice; each of the others projects the same three rows with the embedded call block still present in the assistant turn beside the dedicated call, with zero counts and no notice, and no seeded value changes any row's classification
- **AND** in the `isSeeded: false` file a second dedicated `tool/call` with `callId: "c1"` at the same turn and step makes the identity ambiguous, so both dedicated calls and the embedded copy remain; a version-three `user/message` embedding a `tool-result` block keeps it under every header, because the reader reads no position from it even when its `data` supplies `turn` and `step`; and under a version-zero header the association runs whatever `isSeeded` carries

#### Scenario: A version-3 user message's supplied positions do not make it an association party
- **WHEN** a version-three session whose header records `isSeeded: false` holds a `user/message` at sequence 1 whose `data` carries `content` of a `text` block `see` and a `tool-result` block with `toolCallId` `c1` and content `out`, together with `turn: 1` and `step: 1`, followed by a dedicated `tool/result` at sequence 2 at turn 1 step 1 whose `data.message.content` is that same `tool-result` block; otherwise identical files record `isSeeded: true` or omit it; and a fourth file carries the same two rows under a version-zero header
- **THEN** each version-three read projects two turns in order, a user turn with the blocks `see` and `out [c1]` and a tool turn with `out [c1]`, each at its recorded stamp, with zero counts and no notice, because the reader reads no position from a version-three user message and so nothing owns its copy, under the unseeded header where the association runs as much as under the others where it is withheld; the version-zero read projects the user turn with `see` alone and the same tool turn, because version zero reads the user message's positions and the one dedicated result owns the copy

#### Scenario: Version-zero fragment rows are unknown under a version-3 header
- **WHEN** a version-three session contains a readable `user/message` followed by an `assistant/chunk` row, three otherwise identical files instead contain a valid `text-chunks`, `reasoning-chunks` or `tool-call-chunks` row, and a fifth contains instead a `text-chunks` row that is not a valid packed encoding, with no `time0` member and `texts` a string rather than an array
- **THEN** each of the five reads returns `unsupported-format` with no turns, one unrecognized record and its notice, without decoding the row as a fragment or a packed run; with top-level `ignorable: true` on that row each of the five instead projects the user message once with one unrecognized record, the invalid packed row included, because the version rules the row before any packed member is read; the same invalid `text-chunks` row under a version-zero header refuses the read with one unrecognized record whether or not it carries the marker, as the packed-encoding rule above requires, so a reader that decoded the row before consulting the version would fail the fifth file under version three

#### Scenario: Version-three time keeps the recorded-token exactness of version zero
- **WHEN** a version-three source whose header spells its version `3e0` holds four `tool/call` rows with `time` spelled `1e-400`, `1e3`, `1000.0` and `-0.0`, and version-zero sources hold the existing ordinary `assistant/chunk` at `time: 1e-400` and packed `text-chunks` row at `time0: 1e-400`
- **THEN** the version-three header admits and its four calls project in order with stamps `""`, `"1000"`, `"1000"` and `"0"`, the two nonzero integers on their parsed value alone although their digit signature is `1`, zero counts and no notice; the version-zero chunk still renders an empty stamp and the version-zero packed row still refuses with one unrecognized record; no other field of any row is judged on its recorded token

#### Scenario: Zero-digit underflow and duplicated time members are not zero stamps
- **WHEN** a version-three source holds `tool/call` rows whose `time` is spelled `10e-400`, `0.1e-400` and `1.0e-400`, a `tool/call` that records `time` twice at its top level as `"time":1000` then `"time":2000`, a `tool/call` whose member is named `"\u0074ime"` with the value `1e-400`, and `tool/call` rows spelling `time` as `0`, `0.0`, `-0` and `0e0`; a version-zero source holds an ordinary `assistant/chunk` at `time: 10e-400` and another that records `time` twice; and two version-zero sources hold a packed `text-chunks` row spelling `time0` as `10e-400` and one recording `time0` twice as `1000` then `2000`
- **THEN** the three zero-digit underflow calls, the twice-recorded call and the escaped-name call each project with an empty stamp, because a zero digit inside an underflowing mantissa is not a zero spelling, a member recorded twice has no established token and an escaped name binds to the same judgment as a literal one, while the four genuine zero spellings render `"0"`; every version-three row projects with zero counts and no notice; the version-zero chunks each render an empty stamp with zero counts; and each version-zero packed row returns `unsupported-format` with one unrecognized record and no turns

#### Scenario: Partial packed citations split only the proved members
- **WHEN** one packed text row at sequences 10 through 13, times 1000 through 1003 and one recorded turn/step/index has texts `["a", "b", "c", "d"]`, followed by a readable same-turn/step assembly at sequence 20 citing `[11, 13]`
- **THEN** the read yields chunk `a` stamped `"1000"`, chunk `c` stamped `"1002"`, then the assembly at its own position and stamp; neither `b` nor `d` reappears and the survivors do not join across a suppressed member
- **AND** absent or empty citations instead leave one `abcd` chunk before the assembly; a citation proving all four members leaves only the assembly

#### Scenario: Coalescing preserves empty-member identity and the first visible stamp
- **WHEN** a packed row at sequences 10 through 13 has texts `["", "a", "", "b"]` and times 1000 through 1003, without a suppressing assembly
- **THEN** it produces one `ab` chunk stamped `"1001"`, no empty turns and zero counts
- **AND** a later blockless event sharing sequence 11 keeps an assembly's citation to 11 ambiguous, so that citation cannot remove `a`; all four packed member sequences still participate in identity checks

#### Scenario: Separate packed rows do not become one inferred message
- **WHEN** adjacent packed rows share a provider turn and step, whether they share or differ in index or block kind, and both have unsuppressed readable members
- **THEN** each row contributes its own coalesced chunks with its own first visible stamp; an intervening ordinary chunk, message or tool event keeps its own source position and never joins either packed row

#### Scenario: A blockless DSH event keeps facts without retaining a payload
- **WHEN** an admitted DSH snapshot contains many `tool/result` or `user/message` rows with absent data, around readable chunks and assemblies
- **THEN** those rows retain no payload-bearing event, block or displayed turn and spend no display budget, while their diagnostic disposition and observed sequences remain unchanged
- **AND** a blockless row duplicating a readable chunk's sequence prevents a citation from proving that chunk; a blockless assistant assembly suppresses nothing; an invalid citation on a blockless content row still produces `unsupported-format` and its unrecognized-row count

### Requirement: Every kind obeys the same source and display caps

A read SHALL retain at most **4,000,000 display accounting bytes**. The
numeric `DISPLAY_CAP` SHALL remain 4,000,000. The cost of each final
content-bearing event, represented as one displayed turn, SHALL be the fixed
structural charge **`DISPLAY_EVENT_COST = 512` accounting bytes** plus the
sum of its emitted block texts' UTF-8 byte lengths. All displayed turns of
all three harnesses SHALL pay the charge, including tool or omitted-only
turns with little text; a blockless event supplies no turn and pays nothing.
A coalesced packed chunk SHALL pay once as its final turn, not once per token.
No change in block count, provider role or `--turn` selection SHALL waive
that per-turn charge. The 512-byte constant conservatively rounds above the
approximately 330-byte event footprint cited in #277; it is a portable
accounting allowance, not a measured exact allocator or whole-process RSS
size. Tests and measurement SHALL distinguish those units.

Canonical suppression and coalescing SHALL determine the final turns before
charging the display budget. The read SHALL retain complete turns in file
order and stop before the first turn that would exceed the remaining budget;
it SHALL NOT skip that turn to include later smaller turns. Equality with
the budget SHALL fit, and cost arithmetic SHALL never wrap into admission.
Independently, reading a selected source SHALL consume
at most **33,554,432 bytes (32 MiB)** plus at most one byte solely to detect
whether more source remains, without first allocating the whole file. Only
complete physical rows within that source prefix SHALL be parsed; a packed
row SHALL not be partially decoded at that boundary. A boundary
splitting UTF-8 or JSON because of the cap SHALL be truncation, not malformed
source.

Exceeding either budget SHALL set `truncated` true and supply the shared
notice `transcript truncated (size cap)` exactly, with no provider suffix.
The shipped Claude suffix ` — claude --resume carries the rest` SHALL be
retired; the separate full-session value carries that convenience. The
shipped browser sentence
`transcript truncated (size cap) — resume the session for the rest` SHALL
likewise become exactly the shared notice, retiring its
` — resume the session for the rest` suffix. A first over-limit turn SHALL
produce zero retained turns with that notice. Reaching EOF exactly at a
limit without omitted content SHALL not claim truncation. The budgets SHALL
apply before `--turn` selection; choosing one turn SHALL not bypass the
whole-transcript bound. Header discovery SHALL have the separate bounds
stated above. The independent 32 MiB source bound also applies to logs
dominated by records that render nothing. Display accounting bounds what the local
surfaces retain; encoded-source bytes, transient JSON parsing and compact
association metadata are distinct costs whose peak must be measured.
A fixed structural charge alone is not proof that projector allocation is
bounded before its final output is capped.

#### Scenario: The existing display cap applies to each harness
- **WHEN** a Claude, Codex or DSH file's final projected prefix has N turns and T UTF-8 block-text bytes such that `512 * N + T = 4,000,000`
- **THEN** that complete prefix is retained, EOF without further projected content is not truncated, and another content-bearing turn sets truncation with the same notice on every local surface

#### Scenario: An oversized first turn cannot erase the warning
- **WHEN** the first projected turn contains 3,999,489 UTF-8 bytes of block text, so its charged cost is 4,000,001
- **THEN** no turns are retained and text, JSON and the TUI still indicate truncation rather than reporting an empty or missing session

#### Scenario: Multibyte text spends bytes rather than characters
- **WHEN** a turn's block texts fit by character count but exceed the remaining display accounting budget after the 512-byte charge
- **THEN** that entire turn is excluded and truncation is reported without splitting a character or including a later turn

#### Scenario: Ignored records and enormous lines cannot force an unbounded read
- **WHEN** a file exceeds 32 MiB using context records or one enormous JSON line while its projected block text would otherwise fit
- **THEN** only complete records within the bounded source prefix are considered, no whole-file allocation occurs and `truncated` is true

#### Scenario: Tiny tool calls exhaust structural budget without exhausting text bytes
- **WHEN** an admitted DSH source contains 10,000 ordinary `{"type":"tool/call"}` rows with absent data and no other content
- **THEN** each projects its existing one-byte `?` tool block, costs 513 accounting bytes, and the read retains exactly 7,797 turns, with truncation and the shared notice; total retained text is only 7,797 bytes
- **AND** the same source ending after 7,797 calls is not truncated; merging packed members cannot satisfy this bound because there are no packed members

#### Scenario: An over-budget turn is not skipped for a smaller successor
- **WHEN** one retained turn costs 3,999,000 accounting bytes, the next costs 1,001, and a later turn costs 513
- **THEN** only the first remains, with truncation, even though the last would fit the unused 1,000 bytes

#### Scenario: Canonical preference precedes structural charging
- **WHEN** a bounded snapshot contains a large fallback chunk and a later small readable assembly that uniquely cites every member of that chunk within the same turn/step
- **THEN** the assembly removes the fallback before charging, and the assembly is retained without display truncation if it fits; temporarily seeing an over-budget fallback does not stop the source scan or consume the canonical turn's budget

#### Scenario: Required refusal and diagnostic counting continue beyond the displayed prefix
- **WHEN** enough tiny content-bearing events exhaust the display budget, followed within the complete source prefix by malformed physical lines, counted omissions and a required unknown DSH event or invalid packed row
- **THEN** the result is `unsupported-format` with no turns and the complete-prefix diagnostic counts, retaining only any established source-cap truncation under the existing refusal rule; an early display stop never hides that refusal

## ADDED Requirements

### Requirement: DSH projection bounds event retention before returning

DSH projection SHALL enforce the structural-and-text display budget on
accumulated content-bearing event/turn storage during a read, allowing at
most one additional current candidate turn while deciding whether it fits.
Thus accumulated charged turns SHALL number at most
`floor(4,000,000 / 512) = 7,812`, plus that one candidate; their accumulated
charged text and structural cost SHALL also fit, not merely their count.
The bound SHALL hold before a result is handed to CLI JSON serialization or
TUI refresh, cloning or equality comparison. Creating an unbounded payload
list and applying the cap only when returning it SHALL NOT satisfy this
requirement. Packed coalescing SHALL avoid materializing one payload-bearing
event per token even transiently; coalescing an already expanded list is not
a bound. A current candidate can itself exceed the display budget and is
then discarded whole under the cap rule.

Complete-prefix classification, required refusals and exact association
outcomes SHALL remain unchanged. Source parsing and compact identity or
citation facts can scale with the actual complete bounded source, including
quiet or blockless rows' sequences, but SHALL NOT retain disguised copies of
per-token/per-row content-bearing events outside the budget. They SHALL not
scale with an unobserved sequence gap or citation interval. Measurement SHALL
report their contribution separately when possible and SHALL NOT claim the
display accounting limit is an exact process-memory ceiling.

The implementation SHALL provide reproducible measurements on the unchanged
base and completed projector using the same synthetic version-zero DSH
session, one packed member per token, of a stated exact encoded size. The
record SHALL name the generator parameters, token/member counts, build
profile, measurement method and scope, actual before/after peak allocation
or a defensible proxy, and retained result counts/size. Source/fixture setup
inside the measured interval and the proxy's limitations SHALL be explicit.
Separate tiny-call and blockless-row cases SHALL exercise their independent
retention risks. Estimates or final vector lengths alone SHALL not be
reported as peak allocation measurements. Each automated bound proof SHALL
be shown to fail for its own assertion when its protecting behavior is
removed, then pass after restoration; a compile failure is not that proof.

#### Scenario: Ordinary token streaming does not create a payload object per token
- **WHEN** an admitted version-zero packed row contains many one-character text members for one chunk, without an assembly
- **THEN** coalescing constructs the chunk without retaining one payload-bearing event per member; if its combined text fits with its 512-byte charge it returns one complete turn, otherwise zero turns and truncation
- **AND** the regression observes peak event retention while projection runs and fails when per-member event construction is restored, even if final output is later coalesced

#### Scenario: Tiny ordinary rows stay bounded during projection
- **WHEN** ordinary tiny tool-call rows in a complete bounded DSH source exceed the display budget
- **THEN** accumulated payload-bearing events stay within the charged budget plus one current candidate throughout the read, while all rows still contribute required classification and association facts
- **AND** a regression fails when uncapped event accumulation before final display capping is restored, even though the returned prefix still fits

#### Scenario: Blockless rows retain no wasted events during the pass
- **WHEN** a synthetic DSH source contains thousands of blockless user messages and tool results between two content-bearing events
- **THEN** only the two content-bearing events require retained payloads, and sequences and diagnostics still reflect all rows
- **AND** removing only the blockless retention guard makes the regression fail during collection, even though the displayed turns are unchanged

#### Scenario: Allocation evidence is a repeatable observation
- **WHEN** the implementation reports the bound repaired
- **THEN** its evidence includes actual before and after measurements from identical ordinary token-stream input, named measurement scope and method, and relevant failing/passing removal-test results for merging, charging and transient retention independently
