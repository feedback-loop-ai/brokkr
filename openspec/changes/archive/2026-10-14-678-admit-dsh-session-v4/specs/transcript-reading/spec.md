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
a closed, ordered set of session filenames: `session.v4.jsonl`, then
`session.v4.jsonl.zstd`, then `session.v3.jsonl`, then `session.jsonl`. A
session directory SHALL yield at most one candidate, the first name in that
order carrying a valid header, so a directory holding several admits the
newest and never becomes ambiguous with itself. A name outside the set SHALL
NOT be read, including any other compression suffix or spelling and the
compressed twins of the older names. The set is versioned because the DSH
core versions the file: 0.2.0-rc.2 writes `session.v4.jsonl`, or
`session.v4.jsonl.zstd` under its persistence backend's default encoding,
where 0.1.5-rc.1 wrote `session.v3.jsonl` and earlier cores
`session.jsonl`, and a reader that knows only the older names loses exactly
the newest evidence. Of the two version-four encodings, which DSH never
writes into one root together, the plain one SHALL be asked first, being the
one a Brokkr-launched seat writes. The `.jsonl.zstd` name states the
encoding: such a candidate SHALL be read as DSH's concatenated zstd frames,
by the requirement for compressed sources below. Its first JSONL record SHALL be a `session` header
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
Other roots, compressed files under any other name and delegated sessions SHALL not be substitutes for the seat's session file.

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
header-format restriction. For a compressed candidate the 65,536 bytes are
decompressed header bytes, decoded from at most 131,072 compressed bytes; a
header not decoded within either bound SHALL return `discovery-limit`, so a
header check never decodes a body.
Claude and Codex discovery SHALL read no transcript content; their selected
source reads use the separate 32 MiB budget below, with no 65,536-byte
opening-record gate. Exhausting discovery limits SHALL return
`discovery-limit`, not the first candidate found. Exactly
one qualifying file SHALL be required; several qualifying files SHALL return
`ambiguous-source` regardless of enumeration order. No match SHALL return
`not-found`. A file not yet created or a DSH first-record header not yet
complete can become available on a later read. These bounds apply equally
to CLI lookup, TUI refresh and the browser's participant routes. For
Claude this deliberately replaces the shipped first matching file, unbounded
project enumeration and symlink following with unique, bounded, safe
discovery. These are breaking lookup
changes proposed for 0055, not implied compatibility with the old reader.

After safe unique DSH ownership and a usable bounded UTF-8 source snapshot
are established, content admission SHALL require an opening header whose
top-level `version` is a JSON number in the admitted version set. That set
is closed and is exactly `{0, 3, 4}`. Admission SHALL be judged on the
recorded `version` token and its parsed value together, for zero, three
and four alike. A recorded number token spells an admitted integer exactly
when its parsed value is that integer and its digit signature, the
mantissa digits with the decimal point and every leading and trailing zero
removed, is that integer's digits: empty for zero, `3` for three, `4` for
four. So `0`, `0.0`, `0e0`, `0.000` and `-0` SHALL denote version zero,
`3`, `3.0`, `3e0`, `30e-1` and `0.3e1` SHALL denote version three, and `4`,
`4.0`, `4e0`, `40e-1`, `0.4e1` and `4.000` SHALL denote version four; `-4`,
`4e1`, `4.1`, `3.9999999999999999` and `4.0000000000000001` SHALL refuse
as their version-three counterparts do. `-3` and `3e1`
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
Admitting three and four widens the recorded-token version check from one
admitted integer to three and touches no other field: an ordinary event's `time` and
a packed row's `time0` keep the exactness the content rules below already
require of them under either version, their parsed value a signed safe
integer, a parsed zero a zero spelling only when its token's digit
signature is empty, a nonzero integer judged on its parsed value alone,
and every judged token established by the same one-member binding; no
citation, marker or other event field is judged on its recorded token
under either version. Strings,
booleans and null SHALL not be coerced. Missing `version`, any nonnumeric
value and any numeric value outside the set, including 1, 2 and 5, SHALL
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
exactly `false`. Version four: the 0.2.0-rc.2 packages
`@deepseek-ai/dsh-session-format`, `dsh-session-format-v2-to-v3`,
`dsh-session-format-v3-to-v4` and `dsh-session-persistence-jsonl`, their
READMEs and typed declarations, read for #678 and cited by file, digest
and section in that change. That read reached the V3-to-V4 edge's named
conversions and preservation, the native version-four header, envelope,
tool-result, message-source and developer-message admission, the retired
syntax, the sequence-reference remapping, the generation filenames and the
compressed physical encoding. It did not reach the version-four event
catalogue or the message and block definitions, which live in packages
the read did not include; that change names the reads that lift them.
Under version four every
released version-three type therefore keeps its version-three
disposition, because the edge renames none, every name outside that
vocabulary and the one the read added stays a required unknown, and the
blocks the edge carries over project as under version three. Versions 1 and 2 are not admitted:
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
version three is readable under the version-three rules,
`session.v4.jsonl` carrying version three under the version-three rules,
and any admitted name carrying a version outside the set is refused;
neither infers the other.

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

For the participant body route, every shared reference/discovery/read
failure SHALL return HTTP 404 carrying the refused transcript document,
which names that failure, with no turns. A readable empty, counted-omission
or truncated projection SHALL still return HTTP 200 carrying the readable
document.

Participant growth-route admission SHALL apply the same reference and
discovery rules. Any refusal, including `ambiguous-source`, `discovery-limit`
and `unsafe-path`, SHALL return HTTP 404 with
`{"error":"transcript not found"}` before an event-stream header or event is
written. For an admitted source the size-event/heartbeat shape stays
unchanged. Each subsequent poll SHALL
revalidate safe unique discovery; if that fails, the stream SHALL close
without reporting further sizes from the previous candidate. HTTP status
cannot change after stream admission; closing does not emit a second 404
response. SSE remains a growth notification, not a new transcript body API.

#### Scenario: Browser lookup refusals have fixed HTTP responses
- **WHEN** a valid Claude id has either two qualifying project files, a search that needs more than 10,000 entries to establish uniqueness, or only a matching symlink project/file below the canonical home
- **THEN** the shared lookup returns `ambiguous-source`, `discovery-limit` or `unsafe-path` respectively; `/api/transcript/<run>/<key>` returns HTTP 404 with the transcript document naming that reason and no turns, and a new `/sse/transcript/<run>/<key>` request returns HTTP 404 with `{"error":"transcript not found"}` and no event-stream header
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
- **WHEN** the unique safe candidate's opening `session` object has depth zero and version 1, 2, 5, -1, -3, 0.5, 3.1, 3e1, `2.9999999999999999` or `3.0000000000000001`, with a usable bounded source below the source cap, including if unused current-version header metadata is missing or malformed
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
- **WHEN** one recorded root's only session directory holds `session.jsonl` whose opening header has version 3 and depth zero followed by a readable `user/message`, another's holds `session.v3.jsonl` whose opening header has version 0 and depth zero followed by a readable `user/message`, and a third's holds `session.v3.jsonl` whose opening header has version 5 and depth zero
- **THEN** the first projects its message under the version-three vocabulary and the second its message under the version-zero vocabulary, each with zero counts and no notice, and the third returns `unsupported-format` with its confirmed path and zero counts; neither name infers a version and neither version infers a name

#### Scenario: The versioned name beside the plain name is decisive, whatever its version
- **WHEN** one session directory holds `session.v3.jsonl` with a version-3 header followed by a readable `user/message` and `session.jsonl` with a version-zero header followed by a readable `user/message` with different text, and another directory holds `session.v3.jsonl` with a version-5 header beside `session.jsonl` with a version-zero header and a readable `user/message`
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
- **WHEN** an otherwise unique safe DSH candidate has a valid depth-zero version-zero, version-three or version-four opening session header of exactly 65,536 UTF-8 bytes and ends there without a newline
- **THEN** discovery admits that candidate and the read succeeds with zero turns, zero diagnostic counts, no truncation and its confirmed path/hint
- **AND** the identical header followed by a newline also fits; that delimiter is not a header byte

#### Scenario: One extra header byte refuses even at EOF
- **WHEN** the same otherwise valid candidate's header is 65,537 UTF-8 bytes, with no newline and no byte after it
- **THEN** discovery returns exactly `discovery-limit`, null path and DSH full-session hint, no turns, zero counts, `truncated: false` and no notices, rather than success, `not-found`, `unreadable` or body-stage truncation
- **AND** the over-limit header with a terminating newline has the same refusal

#### Scenario: DSH admits only the declared version-four header shapes
- **WHEN** a DSH source's opening header records version `4`, `4.0`, `4e0`, `40e-1`, `0.4e1` or `4.000`, and otherwise identical sources record `-4`, `4e1`, `4.1`, `"4"`, `5`, `3.9999999999999999` or `4.0000000000000001`
- **THEN** each of the first six is admitted under the version-four vocabulary, a header alone being a readable zero-turn source with zero counts; each of the others returns `unsupported-format` with no turns and zero counts, its rows never classified

#### Scenario: Both version-four names are read into the same turns
- **WHEN** one root's only session directory holds the synthetic version-four session as `session.v4.jsonl`, and another root's holds the same rows as `session.v4.jsonl.zstd`, one checksummed zstd frame per line
- **THEN** each read confirms its own path and projects the same user, assistant, call, tool-role result and assistant turns in order, with zero counts, no truncation and no notice

#### Scenario: The admitted names are asked newest first
- **WHEN** one session directory holds valid session headers under all four admitted names, each followed by a user message naming its file, and the names are removed one at a time, newest first
- **THEN** the reads admit `session.v4.jsonl`, then `session.v4.jsonl.zstd`, then `session.v3.jsonl`, then `session.jsonl`, each projecting its own file's message and none ambiguous with another

#### Scenario: A compressed header is bounded by what it decompresses to
- **WHEN** a `session.v4.jsonl.zstd` candidate's one frame decompresses to a valid header of exactly 65,536 bytes and a newline, another's to 65,537 bytes and a newline, and a third's frame carries no header byte within its first 131,072 compressed bytes before a valid header
- **THEN** the first is admitted as a readable zero-turn source, and the second and third return `discovery-limit` with null path and hint, no turns and zero counts

#### Scenario: A compressed candidate whose frame fails is unreadable at discovery
- **WHEN** a `session.v4.jsonl.zstd` candidate's frame fails its content checksum, states no checksum, or is not zstd at all, each beside a valid `session.v3.jsonl`
- **THEN** each returns `unreadable` with null path and hint, no turns and zero counts, and the version-three file is never read

#### Scenario: No other compressed name is a candidate
- **WHEN** a session directory's only file is the compressed version-four session named `session.v4.jsonl.zst`, `session.v3.jsonl.zstd`, `session.jsonl.zstd`, `session.v4.jsonl.ZSTD` or `session.v04.jsonl.zstd`
- **THEN** each read returns `not-found`

## ADDED Requirements

### Requirement: DSH version four reads as version three, save its named conversions

Under an admitted version-four header every rule the content requirements
state for version three SHALL apply, except the conversions this
requirement names, because the 0.2.0-rc.2 writer's V3-to-V4 edge renames no
event type and keeps every field outside its named conversions. Every
released version-three event type SHALL keep its version-three
disposition. `developer/message`, native to version four, SHALL be a quiet
record: its tool-addition and tool-removal blocks are the writer's notices
of a changed tool set, not a turn. Version-zero storage rows,
`assistant/chunk`, `tool/code-dispatch` and `tool/code-dispatch-start`
SHALL be required unknowns under version four, as under version three, and
`assistant/attempt` a counted omission. A namespaced unknown event type,
`plugin:<original-name>`, SHALL be a name outside the vocabulary: a counted
omission with top-level `ignorable: true` and a required unknown without
it, by the existing marker rule.

A version-four `tool/result` SHALL project its tool-role message: the
message's `toolCallId` names the call the result settles and its own
`content` is the result's output, rendered as version three renders the
retired wrapper's `content`, so a version-three wrapper row and its
version-four lift project the same turn. `isError`, `data.error`, the
message `role`, `id` and `source` SHALL supply no content. The retired
`tool-result` wrapper block SHALL be a counted unrecognized member wherever
version four reads a content list; inside a tool-role result such a member
SHALL make the row a counted record that supplies no result. No `source`
SHALL be read under any version, so a producer's renamed source kind, a
`plugin:` kind or the retired plugin wrapper changes no projection.
`sourceEventSeqs` SHALL be validated exactly as under version three; a log
whose references the historical conversion remapped projects as any log
does.

#### Scenario: A version-four session projects its turns in order
- **WHEN** a version-four session with `isSeeded: false` holds, in order, rows typed `turn/start`, `request/header`, `request/context`, `permission/preset`, `sandbox/mode`, `approval/policy`, `step/start`, `system/message`, a readable `user/message`, an `assistant/message` whose content holds a text block and an embedded `tool-call`, the dedicated `tool/call` for that call, its tool-role `tool/result`, `step/end`, `agent/inbox/spliced`, `compaction/prune`, a second readable `assistant/message`, `turn/end` and `session/title`
- **THEN** the read projects the user message, the assistant text, the call, the tool-role result and the second assistant message in order, the embedded call shown once beside its dedicated row, with zero counts and no notice

#### Scenario: Each quiet version-four name supplies no turn
- **WHEN** a version-four session holds one row of `developer/message`, or of any of `turn/start`, `turn/end`, `step/start`, `step/end`, `request/header`, `request/context`, `permission/preset`, `sandbox/mode`, `approval/policy`, `system/message`, `agent/inbox/spliced`, `compaction/prune`, `session/title`, `subagent/catalog` or `tool/ptc-dispatch`
- **THEN** the read is a readable zero-turn source with zero counts
- **AND** the same `developer/message` row under a version-three header refuses with one unrecognized record

#### Scenario: A namespaced unknown is omitted only with its marker
- **WHEN** a version-four session holds a readable `user/message` and a `plugin:acme/note` row with `ignorable: true`, and other sessions hold one row of `plugin:acme/note`, `assistant/chunk`, `text-chunks` or `tool/code-dispatch` without the marker, and one of `assistant/attempt`
- **THEN** the first projects the user message with one unrecognized record; each unmarked row returns `unsupported-format` with one unrecognized record; `assistant/attempt` is a readable source with one unrecognized record

#### Scenario: The tool-role result maps field by field
- **WHEN** version-four tool-role results carry `toolCallId` `c1` with a text member and an image member, `toolCallId` `c2` with empty content and `isError: true`, and an empty `toolCallId` with a text member
- **THEN** they project the results `out [image omitted] [c1]`, `[c2]` and `who`, role `tool`
- **AND** the version-three wrapper row for `c1` with the same content projects the same turn under a version-three header

#### Scenario: The retired wrapper is counted, never a result
- **WHEN** a version-four tool-role result's content holds a `tool-result` wrapper, and a `user/message` holds a text block beside a `tool-result` wrapper
- **THEN** the read projects the user text alone with two unrecognized records
- **AND** the same rows under a version-three header project both results as today with zero counts

#### Scenario: Producer sources change no projection
- **WHEN** a version-four `user/message` carries source `{kind: "cli"}`, `{kind: "compact-checkpoint"}`, `{kind: "runtime-context"}`, `{kind: "plugin:acme"}`, the retired `{kind: "plugin", plugin: "acme"}` or a non-object source
- **THEN** each projects the same user turn with zero counts

#### Scenario: A remapped reference log projects as today
- **WHEN** a version-four session holds a user message, an inserted interrupted `turn/end`, an `agent/inbox/spliced`, a `turn/start`, a `session/title` citing the first message, a `compaction/prune` with `shadowedRange` and `shadowedSeqs`, and a user message citing `[[0, 2], 4]`
- **THEN** the read projects both user messages with zero counts
- **AND** a citation of the citing row's own sequence, or a range reaching past it, refuses the read under version four exactly as under version three

#### Scenario: A version-four session reaches every surface
- **WHEN** the synthetic version-four session is read as `session.v4.jsonl` and, in another root, as `session.v4.jsonl.zstd`
- **THEN** CLI text, CLI JSON, `--turn` selection, the TUI pane, both doors and the browser body route show the same five turns for both, the fourth the tool-role result

### Requirement: A compressed DSH session reads within the plain source's bounds

A `session.v4.jsonl.zstd` candidate SHALL be decompressed by
`brokkr-protocol`'s one zstd decoder before any byte is interpreted; the
pure view decompresses nothing. Its frames SHALL be decoded in turn, and a
frame read to its end SHALL be kept only when it states a content checksum
and its bytes match it. A frame that does not, and bytes that are no zstd
frame, SHALL be `unreadable` at the stage that read them: at discovery with
null path, closing admission; at the body with the confirmed path. The body
read SHALL read at most 32 MiB of compressed bytes and keep at most 32 MiB
of decompressed bytes; passing either bound SHALL be the source truncation
a plain source past the cap has, with the same notice, the same
participating rows and the same counts. A frame the input bound cuts SHALL
contribute nothing; the one frame the output bound stops is not read to its
end, so its checksum is never reached, as a plain source's bytes past the
cap are never read.

#### Scenario: A compressed body past the source cap truncates as a plain one
- **WHEN** a `session.v4.jsonl.zstd` decompresses to a header, a readable user message and a row of more than 32 MiB, and another root holds the same decompressed bytes as `session.v4.jsonl`
- **THEN** both reads project the one user message with `truncated: true`, the same notice and zero counts

#### Scenario: A body frame that fails is unreadable with its path
- **WHEN** a `session.v4.jsonl.zstd` whose first frames hold a valid header and more than 65,536 decompressed bytes ends in a frame that fails its content checksum
- **THEN** discovery admits it and the read returns `unreadable` with the confirmed path, no turns and zero counts
