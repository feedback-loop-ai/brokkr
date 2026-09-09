## Purpose

Let an operator read the retained transcript belonging to a selected seat,
regardless of harness, with one ordered content projection and explicit
limits that preserve local ownership and journal privacy.

## ADDED Requirements

### Requirement: One transcript derivation serves the local readers

`brokkr-view` SHALL derive transcript content from supplied records into one
serializable `Turn` shape: `role` and `ts` strings and an ordered `blocks`
array whose elements contain `kind` and `text` strings. File discovery and
reading SHALL remain outside the pure view derivation. CLI and TUI SHALL
consume this same derivation; the existing Claude browser drill SHALL reuse
its Claude projection. Prose SHALL belong to this explicit local transcript
read, never to the journal-derived run/participant models.

A displayed turn SHALL be one content-bearing source record after the
per-kind projection and duplicate suppression below, not a journal checkpoint
or a provider's billable turn. A readable streaming fragment is one such source
record; fragments SHALL not be concatenated into an invented assembled record.
Its timestamp SHALL be the recorded timestamp or an empty string when absent; record order SHALL outrank timestamp order.
Blocks SHALL use `text`, `reasoning`, `tool`, `tool-result`, or `omitted`.
User/assistant messages SHALL retain their roles; standalone reasoning and
calls SHALL have role `assistant`, and standalone tool outputs role `tool`.
Non-text media in the new Codex/DSH projections SHALL produce an `omitted`
marker naming the media class, without expanding binary data or fetching it.
No turn SHALL be emitted with no visible blocks.

Canonical preference SHALL be evaluated over the complete records in the
bounded source snapshot before applying the display budget. A canonical
record replaces only its proven fallback representations; it occupies its
own file position and uses its own timestamp. Other records retain their
relative file order. No lookup beyond the source cap SHALL establish a
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

### Requirement: The recorded reference selects the harness home

A reader SHALL use the selected participant's common `kind`, `locator` and
`home` reference, including a recorded custom home, as the authority for
local lookup. The latest reference recorded for that participant SHALL
replace earlier attempts even when its locator is empty. It SHALL NOT stitch
attempts, other participants, previous phase visits or delegated sessions
into a transcript. A resumed thread's entire retained file is the readable
session; the reader SHALL NOT infer attempt boundaries within it.

A present common reference SHALL suppress all legacy fallback. Kind `none`,
an absent reference, an unannounced locator and a missing home SHALL produce
explicit unavailability. An unsupported kind SHALL remain unsupported rather
than borrowing another parser. Only a participant without a common reference
and with Claude, LaneTally, or absent legacy provider provenance SHALL use
its valid legacy `session_id` under the local `HOME/.claude/projects` root.
Absence of legacy provenance means the pre-provider Claude era; an explicit
Codex/DSH/other provider SHALL never be treated as Claude. For eligible legacy
provenance, an absent/empty `session_id` SHALL return `no-reference`, but a
present nonempty invalid id SHALL return `invalid-reference` before any file
lookup or reference synthesis (`transcript: null`, `legacy: false`). This
failure is not converted to `no-reference` merely because there was no common
reference.

#### Scenario: A custom home survives an environment change
- **WHEN** a Codex seat recorded home `/retained/codex` and the local process has a different `CODEX_HOME`
- **THEN** lookup uses `/retained/codex/sessions` only, and reports unavailable if that location cannot be read instead of searching the new environment's home

#### Scenario: An empty retry reference supersedes an earlier session
- **WHEN** an earlier attempt named a readable file and the selected participant's later attempt recorded its real kind with an empty locator
- **THEN** the reader reports `unannounced` and does not display the earlier attempt as the current transcript

#### Scenario: Explicit absence defeats a stale compatibility id
- **WHEN** a participant carries kind `none`, or an invalid or unsupported common reference, beside an old `session_id`
- **THEN** no legacy lookup occurs and the corresponding unavailability is returned

#### Scenario: Old Claude journals remain readable
- **WHEN** a participant has no common reference, a valid legacy session id, and Claude, LaneTally or absent provider provenance
- **THEN** its file is looked up under the local Claude projects home, and the synthesized reference is visibly identified as legacy in the command output

#### Scenario: An old Codex id is not a Claude session
- **WHEN** a participant has only a legacy `session_id` with explicit Codex provenance, even if an identically named Claude file exists
- **THEN** the reader reports `no-reference` and opens neither that Claude file nor a guessed Codex home

#### Scenario: Reading a resumed thread keeps its recorded history
- **WHEN** the selected reference resolves to a file containing several prior attempts of the same thread
- **THEN** the bounded whole-file projection is shown without filtering by checkpoint numbers or changing any resumption behavior

### Requirement: Discovery identifies one owned local file

Claude discovery SHALL search immediate project directories of the recorded
projects home for the exact `<session-id>.jsonl` filename. Codex discovery
SHALL search only below `<recorded-home>/sessions`, through at most six
nested directory levels, for a `rollout-*.jsonl` file naming the complete
thread id. A recognized Codex session header SHALL identify that same thread;
a conflicting or absent identifiable session header SHALL not establish a
match. Thread-id substrings and modification time SHALL never establish
identity.

DSH discovery SHALL search only the retained root at
`<recorded-home>/<locator>`, using its project/session directory layout and
`session.jsonl` filename. Its first JSONL record SHALL be a `session` header
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

Discovery SHALL inspect at most 10,000 directory entries per lookup and
read at most 65,536 bytes of any candidate header. Exhausting discovery
limits SHALL return `discovery-limit`, not the first candidate found. Exactly
one qualifying file SHALL be required; several qualifying files SHALL return
`ambiguous-source` regardless of enumeration order. No match SHALL return
`not-found`. A file/header not yet complete can become available on a later
read. These bounds apply equally to CLI lookup and TUI refresh.

#### Scenario: Concurrent Codex threads do not share a result
- **WHEN** a dated sessions tree contains the requested thread, a newer unrelated rollout and a file whose name contains the requested id only as a fragment
- **THEN** only the file whose complete id and session header identify the requested thread is eligible, independent of timestamps and directory order

#### Scenario: A misleading filename cannot select another thread
- **WHEN** a rollout filename matches the selected id but its session header identifies a different thread or provides no identifiable session
- **THEN** the file does not establish a readable match and none of its prose is displayed

#### Scenario: A DSH child cannot replace its parent
- **WHEN** the recorded DSH root contains one depth-zero session and a newer depth-one delegated session
- **THEN** only the depth-zero session file supplies turns and the full-session path

#### Scenario: A legacy DSH header omits the depth
- **WHEN** the only otherwise qualifying DSH file starts with a session header lacking `delegationDepth`
- **THEN** it is eligible as depth zero, while negative, string and nonzero depths are not eligible

#### Scenario: A driver-folded invalid DSH depth is explicitly refused
- **WHEN** a participant records a DSH root whose only session header has `delegationDepth: "zero"`, even if controller evidence shows the shipped driver already folded that file
- **THEN** the reader returns `not-found` with `no valid depth-zero DSH session header`, no turns, no confirmed path and no full-session hint; it does not coerce the value or alter the adapter

#### Scenario: Multiple candidates are not merged or ranked
- **WHEN** the recorded search scope contains two qualifying Claude files, two matching Codex rollouts or two depth-zero DSH sessions
- **THEN** the reader reports `ambiguous-source`, displays no transcript prose and does not choose by recency or filesystem enumeration order

#### Scenario: Discovery is bounded even in an unrelated large home
- **WHEN** establishing a unique candidate would exceed 10,000 examined entries or requires an over-limit header
- **THEN** discovery reports `discovery-limit` and performs no unbounded scan or header allocation

### Requirement: Local lookup rejects paths that escape ownership

Claude session and Codex thread identifiers SHALL be 1 through 64 ASCII
hexadecimal-or-hyphen characters, beginning with a hexadecimal character.
This rule SHALL replace the existing shared session-id guard for all local
consumers: CLI reading, TUI lookup and convenience lines, the Claude
`/api/session/<id>` and `/sse/session/<id>` routes, and the browser's client
validation. The browser guard SHALL accept the same language
`[0-9a-fA-F][0-9a-fA-F-]{0,63}`; it SHALL not offer a drill or command for an
invalid id. Leading-hyphen ids previously admitted by the old guard are now
invalid on every surface. No separate permissive legacy-id rule remains.
DSH locators SHALL be relative forward-slashed paths with no empty, dot,
parent, drive-prefix or absolute component. A common home SHALL be an
absolute path; empty homes SHALL report `missing-home`, and malformed homes
or locators SHALL report `invalid-reference` before content is opened.
Control characters and NUL SHALL be refused in these path inputs.

Every candidate SHALL remain below the canonical recorded home, and a DSH
candidate also below its recorded seat root. Discovery SHALL not traverse
symlink entries below the canonical home, follow a symlink transcript, or
open a non-regular file such as a FIFO/device. A candidate failing these
checks SHALL supply no content. Race-safe opening SHALL preserve the same
restriction when a candidate changes between discovery and reading. No
reader SHALL create missing directories, invoke a provider, decompress an
unrequested alternative, delete a retained file or change its bytes.

#### Scenario: Traversal is refused before a read
- **WHEN** a reference supplies `../other-seat` as a DSH locator, a shell fragment as a Codex id, a drive-qualified locator, or a relative common home
- **THEN** the read returns `invalid-reference` without opening content outside the declared scope or trying an environment fallback

#### Scenario: The browser drill rejects a leading-hyphen id
- **WHEN** a Claude journal id is `-abc`, even with a matching local `-abc.jsonl` file
- **THEN** CLI/TUI report `invalid-reference` without reading it or forming a command, the browser offers no drill or resume line, and both `/api/session/-abc` and `/sse/session/-abc` return HTTP 404 before opening the file or a stream

#### Scenario: A legacy invalid id uses the same refusal
- **WHEN** a pre-common-reference Claude participant has legacy `session_id: "-abc"`
- **THEN** CLI/TUI report `invalid-reference`, with no synthesized reference, no command and no file lookup; it is not silently classified as `no-reference`

#### Scenario: The browser retains valid-id compatibility
- **WHEN** a Claude id is `a-bC09` and its unique owned local file is readable within the shared limits
- **THEN** the client and server both accept it, the session endpoint retains its `session_id`, `turns` and `truncated` response shape and shared Claude content, and the existing growth stream remains available

#### Scenario: A symlink and a FIFO are not transcript files
- **WHEN** a matching path is a symlink escaping the seat root, a symlink within the root, or a FIFO, including replacement after discovery
- **THEN** the reader opens no substitute content, does not block waiting for FIFO bytes and returns an unavailable result

#### Scenario: Reading retains the operator's evidence
- **WHEN** either local surface reads a valid transcript or fails to find one
- **THEN** the transcript file, retained root and provider configuration have the same bytes and existence afterward, and no harness process was started

### Requirement: Full-session information belongs to the shared local read result

`transcript-reading` SHALL own the informational `full_session` value; CLI
text/JSON and the TUI SHALL consume it without independently deciding whether
a hint exists or which command to name. The value SHALL follow this table.
A valid reference here is an effective supported reference with a nonempty
validated locator and absolute validated home, including a synthesized legacy
Claude reference. A confirmed path is the safe unique path established by
lookup; file-read failure after confirmation does not erase that path.

| Kind and resolution | Exact `full_session` value |
|---|---|
| Valid `claude-session`, with or without a confirmed path | `full session: claude --resume <id>` |
| Valid `codex-thread`, confirmed path | `full session: <quoted-path>; codex exec resume <id>; home: <quoted-home>` |
| Valid `codex-thread`, no confirmed path | `full session: rollout unavailable; codex exec resume <id>; home: <quoted-home>` |
| Valid `dsh-session`, confirmed path | `full session: <quoted-path>` |
| Valid `dsh-session`, no confirmed path | null |
| Absent, `none`, unsupported, unannounced, missing-home or invalid reference | null |

The quoted path/home placeholders SHALL be double-quoted JSON string
literals: escape quotation marks and reverse solidus, encode ASCII controls
using the usual JSON short escapes where available and lowercase `\u00xx`
otherwise, and leave other Unicode characters literal. No optional slash or
ASCII-to-Unicode escaping SHALL be added. This encoding introduces no shell
escaping, expansion or command syntax. The complete line is an ordinary string, then JSON-escaped
again only when serialized as a JSON member. Paths SHALL never be guessed
from a date or interpolated into a command; the recorded home is informational
and is not an assertion that an ambient resume command uses that home.
Unavailability remains explicit beside a hint; a valid id is not evidence
that credentials, live resumption or sandbox re-imposition work.

These are the kind-specific command-construction semantics proposed for 0055
to supersede decision 0032 ruling 4's Claude-only enforcement binding.
Decision 0030 supplies the recorded Codex command spelling. Reading or
opening a hint SHALL invoke no provider or change any execution boundary.
The common transcript fact SHALL remain visible independently of the hint.

#### Scenario: The proposed convenience binding remains kind-specific
- **WHEN** valid Claude, Codex and DSH references each resolve to an owned file
- **THEN** Claude receives only its Claude command, Codex receives only its Codex command with the confirmed rollout and recorded home, and DSH receives only its confirmed session path; the common reference and journal bytes are unchanged

#### Scenario: Unresolved references have deterministic hints
- **WHEN** a valid reference has no matching local file
- **THEN** Claude retains its exact command line, Codex uses the exact `rollout unavailable` form with its id and home, and DSH has a null hint; each read still reports `not-found` with a null path

#### Scenario: Invalid references cannot form a command
- **WHEN** an id contains a semicolon, command substitution or leading hyphen, or its required home is missing or malformed
- **THEN** `full_session` is null on both CLI and TUI, and the matching reference failure is returned without invoking a provider

### Requirement: Claude content preserves the existing projection

Claude SHALL retain the existing user/assistant JSONL projection: message
string content, nonblank `text` blocks, and `tool_use` markers containing the
tool name and optional `input.file_path`, in source order. The role SHALL
come from the message or fall back to the record type, and an absent stamp
SHALL be empty. Other record/block kinds SHALL remain absent from Claude's
projected prose as today; this feature SHALL NOT expand it to arguments,
tool-result bodies or thinking. Known deliberately omitted kinds count in
neither diagnostic, but an unrecognized type or content variant still counts
in `unrecognized_records`; omitting prose does not mean hiding format drift.
The shared limit and safe-location rules apply to Claude as to the new kinds.

#### Scenario: Existing Claude text and tool markers survive extraction
- **WHEN** a Claude file contains a string message, text blocks, a blank block, a Read tool with a file path, a Bash tool with arguments, and a thinking block
- **THEN** the same text and `Read · <file-path>` and `Bash` markers appear in order, with no Bash arguments or thinking prose added

#### Scenario: The browser drill keeps its wire shape
- **WHEN** the existing local Claude session endpoint reads a supported session within the shared limits
- **THEN** it keeps its `session_id`, `turns` and `truncated` response fields and the same Claude turn contents while using the common derivation

### Requirement: Codex rollouts expose ordered retained content once

Codex SHALL read retained rollout JSONL content, including user/assistant
messages, readable reasoning summaries, function/custom tool calls and their
outputs. Canonical `response_item` records SHALL supply content. A recognized
content-bearing `event_msg` message, readable reasoning or tool execution
record SHALL supply a fallback turn when no complete canonical record in the
bounded source snapshot carries a recognized projection of that content. This applies to event-only,
live and interrupted files as well as completed files. When a complete
canonical record covers it, its event representation SHALL supply no turn,
even if the event came first or last. An event-only record that carries no
readable content SHALL not cause prose to be invented.

Association SHALL require the provider's recorded message/call identity or
an unambiguous item position within its recorded turn/step sequence verified
from provider source. A shared timestamp, equal text, a shared turn number
alone or recency SHALL not prove duplication. Content for which no such
association exists SHALL remain separate, never be discarded by a heuristic.
Design SHALL document the measured association for each supported mirrored
record family; evidence that a provider's actual mirrors cannot be associated
SHALL be returned upstream before claiming duplicate-free support for that
format.

Context/settings, usage, lifecycle and encrypted reasoning fields SHALL not
become prose. A readable reasoning summary SHALL be emitted
as `reasoning`; unavailable or encrypted reasoning SHALL not be invented or
decrypted.

Tool call blocks SHALL retain the recorded name, call id when present and
arguments/input; tool-result blocks SHALL retain the recorded call id when
present and textual output, including error output. Missing call partners
SHALL not suppress a recorded call or output or cause invented pairing. JSON
values used as tool payloads SHALL be rendered as JSON rather than inferred
commands. Tool content SHALL remain inert display data. Payload details for
the installed provider versions SHALL be verified from installed help/source
or controller evidence during design before parser support is claimed.

#### Scenario: A Codex ruling and its supporting tools are readable
- **WHEN** a rollout contains a user request, a readable reasoning summary, a tool call with arguments and call id, the output with that id, and an assistant ruling
- **THEN** all five appear in that order with their recorded content, and the operator can read the ruling through both CLI and TUI

#### Scenario: Event mirrors do not duplicate response items
- **WHEN** a rollout contains canonical response messages, reasoning and tool output alongside matching event notifications
- **THEN** each canonical content record is displayed once and the notifications add no duplicate prose or turns

#### Scenario: A Codex event-only snapshot is readable
- **WHEN** an owned rollout's bounded snapshot contains recognized content-bearing `event_msg` user, reasoning, call, output and assistant records and no corresponding complete `response_item` records
- **THEN** all five source records project in file order, including the assistant ruling, in CLI and TUI; they are not silently omitted because they are events

#### Scenario: A late canonical record replaces its event fallback
- **WHEN** a later read contains a complete canonical assistant message associated with an event fallback shown by the earlier read
- **THEN** the new snapshot contains the canonical message once at its own file position and timestamp, the associated fallback is absent, and unrelated event-only content remains visible

#### Scenario: A canonical record outside the source cap cannot suppress visible content
- **WHEN** a recognized content event is inside the complete source prefix and its canonical item lies beyond the source cap or is an incomplete append
- **THEN** the event supplies its fallback turn if it fits the display budget, without scanning beyond the cap; source truncation is reported when applicable

#### Scenario: Unassociated records are not suppressed by similar words
- **WHEN** an event message and a canonical message contain identical text but have distinct recorded identities and no measured mirror relationship
- **THEN** both appear at their source positions; text equality alone does not erase either message

#### Scenario: Repeated words remain separate messages
- **WHEN** two distinct canonical messages contain identical text, or two calls use the same arguments with different call ids
- **THEN** both remain in source order; duplicate suppression does not deduplicate by text equality

#### Scenario: Incomplete tool pairs and encrypted reasoning are honest
- **WHEN** a capped or interrupted rollout contains an output without its call and a reasoning record with only encrypted content
- **THEN** the output is displayed with its recorded identifier, no call is synthesized, and no readable reasoning is claimed for the encrypted record

### Requirement: DSH sessions expose assembled or provisional content once

DSH SHALL read retained `user/message`, assembled `assistant/message`,
`tool/call` and `tool/result` events in file order, including readable
reasoning retained in assembled messages when present. For a recorded step
with no complete readable assembled `assistant/message` in the bounded
snapshot, each recognized `assistant/chunk` carrying text or readable
reasoning SHALL project as its own assistant turn in chunk file order with
its recorded timestamp. This rule applies between live steps and after
interruption; no step-ending or assistant message is required for access to
already retained chunk content. A later complete readable assembled message
SHALL replace all its associated chunks in the new snapshot, at the
assembled record's file position; chunks from other steps SHALL remain.

Step association SHALL use the session's recorded turn and step identity
(the pair, never `turn` alone), or its unambiguous step boundary sequence
verified from provider source. Missing association SHALL not be guessed from
text or timestamps: such readable chunks remain separate source turns, as
with unassociated Codex events. Chunk payload mapping and boundary evidence
SHALL be documented during design; an actual mirrored format that cannot be
associated SHALL be returned upstream. Tool argument fragments SHALL not be
promoted to invented complete calls. Request/context copies and
lifecycle/accounting records SHALL supply no content. Dedicated call/result
events SHALL own the displayed tool blocks; duplicate tool declarations embedded in
an assembled message SHALL not be appended again. Message text and reasoning
blocks SHALL preserve their recorded order. Calls and outputs SHALL retain
recorded names, identifiers, arguments and textual results under the same
inert-content and absent-partner rules as Codex.

#### Scenario: A DSH step is assembled once
- **WHEN** a session has a user message, several assistant chunks, an assembled assistant message containing text and reasoning, a tool call, its output and another assembled answer
- **THEN** the user message, assembled blocks, call, result and answer appear once in source order; the chunks do not duplicate the assistant's text

#### Scenario: An interrupted DSH step retains its chunks
- **WHEN** an owned session ends with two complete readable `assistant/chunk` records for one step, followed by a partial JSON append and no assembled message
- **THEN** the two chunks remain readable as two ordered assistant turns through CLI and both TUI doors, with no fabricated assembled answer, and the partial append is not a malformed complete line

#### Scenario: Assembly replaces only its own step's chunks
- **WHEN** a later bounded snapshot contains an assembled message for step one and only readable chunks for step two within the same DSH turn
- **THEN** step one's chunks are replaced by its one assembled message and step two's chunks remain visible once in their source positions, without merging the two steps or changing journal accounting

#### Scenario: A call repeated inside a message is not another operation
- **WHEN** an assembled DSH message declares a tool call and the stream also contains its dedicated `tool/call` record
- **THEN** the dedicated event supplies the one displayed call and the message still supplies its other text and reasoning

#### Scenario: Missing reasoning and billing facts stay missing
- **WHEN** a DSH assembled message carries answer text but no readable reasoning
- **THEN** the answer appears without manufactured reasoning, and request settings or token counts are not substituted for reasoning text

### Requirement: Partial records and read failures remain distinguishable

Complete malformed JSON lines SHALL be skipped without repairing them,
counted as `skipped_lines`, and reported by CLI/TUI with the exact notice
`malformed transcript lines skipped: <n>` when the count is positive.
Complete valid JSON records with an unrecognized type, envelope or content
variant SHALL instead be counted in `unrecognized_records`, at most once per
source record even when several blocks are unrecognized. A partially
recognized message SHALL retain its supported blocks and count that record
once for the unsupported portion. Valid JSON scalars and records with no
recognizable envelope count as unrecognized, not malformed. Recognized
headers, context/lifecycle/accounting records, encrypted reasoning,
known explicitly omitted Claude blocks and proven duplicate representations SHALL
count in neither diagnostic. The exact notice
`unrecognized transcript records: <n>` SHALL appear when that count is
positive; neither notice SHALL quote an unknown or malformed payload.

A last line that is not yet valid JSON and lacks a newline SHALL be treated as an incomplete append, omitted until a subsequent
read completes it, and not counted as a malformed complete line. A complete
valid JSON final line SHALL be readable even without a trailing newline.
Invalid UTF-8 in consumed source bytes or a file I/O failure SHALL return
`unreadable`, not replacement prose; the explicit source-cap boundary exception
below still applies. Both counts SHALL describe all complete records
examined within the bounded source snapshot, before display capping and
`--turn` selection; neither counts an incomplete append or source-cap
fragment. Counts start at zero when no source was read.

A readable file with no projected turns SHALL succeed with `no readable turns`,
distinct from unavailability and from a cap that retained no complete turn. When `unrecognized_records` is positive, CLI text/JSON, the TUI pane and
both reading overlays SHALL carry its count/notice even if some turns are
readable; the zero-turn case SHALL not appear as an empty recording with no
explanation. For CLI/TUI, notices SHALL be ordered: truncation, malformed-line
count, unrecognized-record count, omitting only notices whose condition is
false. The existing Claude browser response retains its three-field envelope;
these added diagnostic fields belong to the explicit transcript CLI/TUI
result, not that compatibility response.

#### Scenario: A growing final line becomes a turn later
- **WHEN** an active transcript initially ends halfway through a JSON record and a later read sees that record completed
- **THEN** the first read omits the incomplete record and the later read displays it once, without a permanent parse failure or fabricated content

#### Scenario: Malformed records do not hide later valid content
- **WHEN** a complete malformed JSON line precedes a valid user message and an unknown lifecycle record
- **THEN** the user message remains readable, `skipped_lines` and `unrecognized_records` each equal one, and their notices report both omissions without quoting either payload

#### Scenario: Unrecognized records cannot masquerade as an empty session
- **WHEN** an owned readable file has a recognized session header followed by five thousand valid JSON records of an unsupported content type
- **THEN** it succeeds with zero turns, `unrecognized_records: 5000`, `skipped_lines: 0` and `unrecognized transcript records: 5000` beside `no readable turns`; an empty file or one containing only recognized header/context records has both counts zero and no unknown-record notice

#### Scenario: Partial support remains visible without quoting unknown payloads
- **WHEN** one complete message contains readable text and two unsupported block variants, followed by one malformed complete JSON line
- **THEN** the text remains visible, `unrecognized_records` is one, `skipped_lines` is one, and the two counted notices disclose the omissions without echoing either unknown block or the malformed line

#### Scenario: Invalid bytes are not repaired
- **WHEN** consumed source bytes contain invalid UTF-8 before any source-cap boundary, even in a file without a trailing newline
- **THEN** the local result is `unreadable` and no replacement-character version of its prose is presented as the transcript

#### Scenario: An empty session is different from a missing file
- **WHEN** a valid retained file contains only a session header and context records
- **THEN** it returns zero turns without an unavailability reason, and a missing file instead returns `not-found`

### Requirement: Every kind obeys the same source and display caps

A read SHALL retain at most **4,000,000 UTF-8 bytes** summed over emitted
block texts, the existing Claude budget. It SHALL retain complete turns in
file order and stop before the first turn that would exceed that budget;
it SHALL NOT skip that turn to include later smaller turns. Equality with
the budget SHALL fit. Independently, reading a selected source SHALL consume
at most **33,554,432 bytes (32 MiB)** plus at most one byte solely to detect
whether more source remains, without first allocating the whole file. Only
complete records within that source prefix SHALL be parsed. A boundary
splitting UTF-8 or JSON because of the cap SHALL be truncation, not malformed
source.

Exceeding either budget SHALL set `truncated` true and supply the shared
notice `transcript truncated (size cap)` exactly, with no provider suffix.
The shipped Claude suffix ` — claude --resume carries the rest` SHALL be
retired; the separate full-session value carries that convenience. The
existing Claude browser truncation sentence SHALL likewise use the exact
shared notice. A first over-limit turn SHALL produce zero retained turns with that notice. Reaching EOF exactly at a
limit without omitted content SHALL not claim truncation. The budgets SHALL
apply before `--turn` selection; choosing one turn SHALL not bypass the
whole-transcript bound. Header discovery SHALL have the separate bounds
stated above. The 32 MiB source bound adds a finite input limit to the old
displayed-text cap, including logs dominated by records that render nothing.

#### Scenario: The existing display cap applies to each harness
- **WHEN** a Claude, Codex or DSH file projects a 4,000,000-byte prefix followed by another nonempty turn
- **THEN** that exact prefix is retained and every local surface reports truncation; a file ending at that same prefix without further content is not truncated

#### Scenario: An oversized first turn cannot erase the warning
- **WHEN** the first projected turn contains 4,000,001 UTF-8 bytes of block text
- **THEN** no turns are retained and text, JSON and the TUI still indicate truncation rather than reporting an empty or missing session

#### Scenario: Multibyte text spends bytes rather than characters
- **WHEN** a turn's block texts fit by character count but exceed the remaining UTF-8 byte budget
- **THEN** that entire turn is excluded and truncation is reported without splitting a character or including a later turn

#### Scenario: Ignored records and enormous lines cannot force an unbounded read
- **WHEN** a file exceeds 32 MiB using context records or one enormous JSON line while its projected block text would otherwise fit
- **THEN** only complete records within the bounded source prefix are considered, no whole-file allocation occurs and `truncated` is true

### Requirement: Transcript prose stays local and inert

Reading SHALL open the journal read-only and SHALL append no events or
checkpoints. Prompt text, reasoning, tool arguments and output SHALL not
enter inspect/seats/watch JSON, exports, dossiers, result telemetry, the
journal or a new persistent transcript cache. Only the explicit transcript
command and existing local transcript surfaces SHALL expose their requested
prose. Plain-text terminal rendering SHALL sanitize control/escape sequences
using the same terminal safety rules as the existing TUI; JSON SHALL preserve
content as JSON-escaped strings. Reading or displaying full-session hints
SHALL execute no provider command, tool call, URL, shell fragment or file
contents, and SHALL not change adapter resumption or sandbox behavior.

#### Scenario: A tool output cannot become terminal control or a journal event
- **WHEN** a transcript contains an ANSI escape, a shell command, a credential-shaped sentinel and tool output
- **THEN** explicit local JSON contains the requested content as escaped strings, text/TUI output neutralizes terminal controls, and the journal event count/hash and its exports gain none of that transcript content

#### Scenario: Inspect remains a reference-only readout
- **WHEN** inspect, seats, watch or a dossier is derived for a seat whose transcript can now be read
- **THEN** its common reference and existing accounting remain unchanged and no transcript body is attached to the journal-derived view

## Decisions

### R1 / clarification 1 — Event-only Codex content is readable

Canonical-only rendering is rejected: an interruption can leave retained
readable events without canonical items, which would reproduce #222's
unreadability. Prefer a complete canonical representation only for an
established association; otherwise show the recorded fallback. Source order
means the chosen record's position, not a guessed timeline. Text-based
matching is rejected because identical words can be separate messages. The
new event-only, replacement, capped-canonical and distinct-identity scenarios
state the behavior; proposal S3 names the missing installed-format proof.

### R2 / clarification 2 — Unassembled DSH steps remain readable

The measured stream in `crates/brokkr-protocol/src/adapters.rs` grows by
chunks before each assembled message, with multiple steps under one turn.
Waiting for assembly would hide an interrupted step's only retained words.
Expose readable chunks as source turns, then replace only that step's chunks
on assembly. Inventing an assembled answer or conflating a provider turn
with a step is rejected. Payload mapping is still a design measurement, as
S3 states; chunk visibility and replacement are now requirements.

### R3 / clarification 3 — Unknown content is a separate counted omission

An unknown type is valid JSON, so it is not a malformed line. It is also not
proof that nothing was recorded. Count unknown records (including partially
supported ones) without quoting their payloads. Recognized metadata and
intentional omissions remain quiet. Reject the previous bare-empty rendering
for an all-unknown file: it hides format drift. The diagnostic is local read
metadata, never journal telemetry. This changes an unpublished proposed
`brokkr.transcript/v1` shape; no published/frozen schema is edited.

### R4–R5 / clarifications 4 and 5 — One owner and deterministic hints

This capability owns the string/null decision independently of TUI rendering.
A syntactically valid Claude/Codex id keeps its informational command even
when lookup fails; Codex explicitly names rollout unavailability and its
recorded home. A DSH locator is only a root, so inventing its deeper session
path or a command is refused. Null/string is no longer an implementation
choice. Proposal S2 explicitly proposes the replacement of decision 0032
ruling 4's command-construction binding in proposed 0055, while preserving
its common reference and all of decision 0030's execution restrictions.

### R6 / clarification 6 — One truncation notice, separate convenience

The shared notice is verbatim for every kind, including Claude and its
browser drill. Keeping Claude's old suffix would contradict that promise;
its separate full-session line retains the resume convenience. No truncation
notice advertises a provider command.

### R7 / clarification 7 — The identifier guard moves everywhere together

The existing `ui::valid_session_id` guards both paths and pasteable commands;
`ui.html` duplicates its language. Leading hyphens can look like command
options, so reject them in all consumers, including legacy ids, instead of
maintaining conflicting guards. This is an explicit compatibility tightening
in the proposal and proposed 0055. The browser 404/no-drill scenario pins the
change; valid ids keep their existing wire shape and Claude projection.

### R8 / clarification 8 — Invalid depth cannot prove DSH ownership

The shipped `names_the_seats_own_session` uses `as_u64().unwrap_or(0)` and
therefore folds a non-u64 depth as root. Aligning that permissive coercion is
rejected for the new reader: an invalid value does not establish that a
session was not delegated. Only absent legacy depth or explicit unsigned
zero establishes eligibility. The driver-folded-invalid-depth scenario names
the deliberate discrepancy and exact explanation. The adapter and #226's
resumption/launch semantics remain outside this issue; neither the journal
nor the operator's retained file is rewritten to hide the refusal.
