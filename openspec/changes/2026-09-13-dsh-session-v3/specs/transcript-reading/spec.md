## Purpose

Read the DSH session file the installed core actually writes. The reader
knew one filename; the core now versions it, so the newest sessions were
recorded in the journal and unreadable on disk.

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
top-level `version` is a JSON number equal to zero. Numeric zero spellings
such as `0`, `0.0`, `0e0` and `-0` SHALL denote version zero; strings, booleans
and null SHALL not be coerced. Missing `version`, any nonnumeric value and
any numeric value other than zero SHALL return `unsupported-format` with
`DSH transcript format is not supported`. There is no legacy default version
or automatic migration. An omitted depth still means zero under the settled
ownership rule; a missing version does not. Other header fields, including
`id`, `createdAt`, `cwd`, `parentSession`, `seedLength`, `origin`,
`agentPreset` and old execution-policy fields, SHALL not qualify or veto
this audit read, even when absent or mistyped. Extra fields SHALL be ignored,
never interpreted as an event, identity, path, timestamp or execution policy.
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

An admitted version-zero opening header SHALL be a quiet record. Subsequent
`session`-typed rows SHALL be unknown event envelopes under the DSH ignorable
rule, not additional format headers or version switches. R14/R15's event,
packed-row and citation admission rules and whole-prefix diagnostic counts
SHALL apply only after version-zero header admission. Elsewhere in these
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
- **WHEN** a unique safe candidate starts with a `session` object whose depth is zero or omitted, but whose version is absent, null, false, `"0"`, an array or an object, and its bounded source is usable without source truncation
- **THEN** ownership resolves its path, but content admission returns `unsupported-format`, `DSH transcript format is not supported`, the unchanged reference and confirmed path/hint, no turns, zero counts, `truncated: false` and no notices; no version is synthesized and an ignorable marker cannot make the header readable

#### Scenario: A foreign-version DSH root alone keeps its confirmed location
- **WHEN** the unique safe candidate's opening `session` object has depth zero and version 1, -1 or 0.5, with a usable bounded source below the source cap, including if unused current-version header metadata is missing or malformed
- **THEN** the reader returns `unsupported-format` with its unchanged reference, `legacy: false`, confirmed path and DSH path hint, no turns, zero diagnostic counts, `truncated: false` and no notices; it neither returns `not-found` nor decodes the apparently familiar later events

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

