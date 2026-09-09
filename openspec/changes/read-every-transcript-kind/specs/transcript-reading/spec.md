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
or a provider's billable turn. Its timestamp SHALL be the recorded timestamp
or an empty string when absent; record order SHALL outrank timestamp order.
Blocks SHALL use `text`, `reasoning`, `tool`, `tool-result`, or `omitted`.
User/assistant messages SHALL retain their roles; standalone reasoning and
calls SHALL have role `assistant`, and standalone tool outputs role `tool`.
Non-text media in the new Codex/DSH projections SHALL produce an `omitted`
marker naming the media class, without expanding binary data or fetching it.
No turn SHALL be emitted with no visible blocks.

#### Scenario: The same content reaches text and both TUI doors
- **WHEN** a selected seat has a readable transcript of any of the three supported kinds
- **THEN** the command and TUI receive the same ordered turns and blocks for the same file snapshot, and opening one turn yields exactly that turn's content from the whole transcript

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
Codex/DSH/other provider SHALL never be treated as Claude.

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
depth SHALL not match. Other roots, compressed files and delegated sessions
SHALL not be substitutes for the seat's plain JSONL file.

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

#### Scenario: Multiple candidates are not merged or ranked
- **WHEN** the recorded search scope contains two qualifying Claude files, two matching Codex rollouts or two depth-zero DSH sessions
- **THEN** the reader reports `ambiguous-source`, displays no transcript prose and does not choose by recency or filesystem enumeration order

#### Scenario: Discovery is bounded even in an unrelated large home
- **WHEN** establishing a unique candidate would exceed 10,000 examined entries or requires an over-limit header
- **THEN** discovery reports `discovery-limit` and performs no unbounded scan or header allocation

### Requirement: Local lookup rejects paths that escape ownership

Claude session and Codex thread identifiers SHALL be 1 through 64 ASCII
hexadecimal-or-hyphen characters, beginning with a hexadecimal character.
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

#### Scenario: A symlink and a FIFO are not transcript files
- **WHEN** a matching path is a symlink escaping the seat root, a symlink within the root, or a FIFO, including replacement after discovery
- **THEN** the reader opens no substitute content, does not block waiting for FIFO bytes and returns an unavailable result

#### Scenario: Reading retains the operator's evidence
- **WHEN** either local surface reads a valid transcript or fails to find one
- **THEN** the transcript file, retained root and provider configuration have the same bytes and existence afterward, and no harness process was started

### Requirement: Claude content preserves the existing projection

Claude SHALL retain the existing user/assistant JSONL projection: message
string content, nonblank `text` blocks, and `tool_use` markers containing the
tool name and optional `input.file_path`, in source order. The role SHALL
come from the message or fall back to the record type, and an absent stamp
SHALL be empty. Other record/block kinds SHALL remain omitted as today;
this feature SHALL NOT expand Claude's projection to arguments, tool-result
bodies or thinking. The shared limit and safe-location rules apply to Claude
as to the new kinds.

#### Scenario: Existing Claude text and tool markers survive extraction
- **WHEN** a Claude file contains a string message, text blocks, a blank block, a Read tool with a file path, a Bash tool with arguments, and a thinking block
- **THEN** the same text and `Read · <file-path>` and `Bash` markers appear in order, with no Bash arguments or thinking prose added

#### Scenario: The browser drill keeps its wire shape
- **WHEN** the existing local Claude session endpoint reads a supported session within the shared limits
- **THEN** it keeps its `session_id`, `turns` and `truncated` response fields and the same Claude turn contents while using the common derivation

### Requirement: Codex rollouts expose ordered retained content once

Codex SHALL read retained rollout JSONL content, including user/assistant
messages, readable reasoning summaries, function/custom tool calls and their
outputs. Canonical `response_item` records SHALL supply content; mirrored
`event_msg` message, reasoning and execution notifications SHALL not append
a second copy. Context/settings, usage, lifecycle and encrypted reasoning
fields SHALL not become prose. A readable reasoning summary SHALL be emitted
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

#### Scenario: Repeated words remain separate messages
- **WHEN** two distinct canonical messages contain identical text, or two calls use the same arguments with different call ids
- **THEN** both remain in source order; duplicate suppression does not deduplicate by text equality

#### Scenario: Incomplete tool pairs and encrypted reasoning are honest
- **WHEN** a capped or interrupted rollout contains an output without its call and a reasoning record with only encrypted content
- **THEN** the output is displayed with its recorded identifier, no call is synthesized, and no readable reasoning is claimed for the encrypted record

### Requirement: DSH sessions expose assembled content once

DSH SHALL read retained `user/message`, assembled `assistant/message`,
`tool/call` and `tool/result` events in file order, including readable
reasoning retained in assembled messages when present. `assistant/chunk`
streaming fragments, request/context copies and lifecycle/accounting records
SHALL not create duplicate displayed messages. Dedicated call/result events
SHALL own the displayed tool blocks; duplicate tool declarations embedded in
an assembled message SHALL not be appended again. Message text and reasoning
blocks SHALL preserve their recorded order. Calls and outputs SHALL retain
recorded names, identifiers, arguments and textual results under the same
inert-content and absent-partner rules as Codex.

#### Scenario: A DSH step is assembled once
- **WHEN** a session has a user message, several assistant chunks, an assembled assistant message containing text and reasoning, a tool call, its output and another assembled answer
- **THEN** the user message, assembled blocks, call, result and answer appear once in source order; the chunks do not duplicate the assistant's text

#### Scenario: A call repeated inside a message is not another operation
- **WHEN** an assembled DSH message declares a tool call and the stream also contains its dedicated `tool/call` record
- **THEN** the dedicated event supplies the one displayed call and the message still supplies its other text and reasoning

#### Scenario: Missing reasoning and billing facts stay missing
- **WHEN** a DSH assembled message carries answer text but no readable reasoning
- **THEN** the answer appears without manufactured reasoning, and request settings or token counts are not substituted for reasoning text

### Requirement: Partial records and read failures remain distinguishable

Complete malformed JSON lines SHALL be skipped without repairing them,
counted as `skipped_lines`, and reported by CLI/TUI with a shared notice when
the count is positive. Unknown record kinds and deliberately omitted content SHALL
not count as malformed. A last line that is not yet valid JSON and lacks a
newline SHALL be treated as an incomplete append, omitted until a subsequent
read completes it, and not counted as a malformed complete line. A complete
valid JSON final line SHALL be readable even without a trailing newline.
Invalid UTF-8 in consumed source bytes or a file I/O failure SHALL return
`unreadable`, not replacement prose; the explicit source-cap boundary exception
below still applies. A readable file with no projected turns
SHALL be a successful empty transcript, explicitly rendered as such, distinct
from unavailability and from a cap that retained no complete turn.

#### Scenario: A growing final line becomes a turn later
- **WHEN** an active transcript initially ends halfway through a JSON record and a later read sees that record completed
- **THEN** the first read omits the incomplete record and the later read displays it once, without a permanent parse failure or fabricated content

#### Scenario: Malformed records do not hide later valid content
- **WHEN** a complete malformed JSON line precedes a valid user message and an unknown lifecycle record
- **THEN** the user message remains readable, `skipped_lines` equals one and a notice reports one malformed line without quoting its payload

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
notice `transcript truncated (size cap)`. A first over-limit turn SHALL
produce zero retained turns with that notice. Reaching EOF exactly at a
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
