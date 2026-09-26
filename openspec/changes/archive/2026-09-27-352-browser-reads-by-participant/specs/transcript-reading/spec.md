## MODIFIED Requirements

### Requirement: Browser participant drills obey shared eligibility

For a selected participant, the browser SHALL consume reference eligibility
and `full_session` from the same Rust derivation used by CLI/TUI, including
common-reference precedence, legacy provenance, home and identifier
validation. It SHALL render the common transcript fact and any reference or
lookup unavailability from that result. A null `full_session` SHALL produce
no convenience line; a non-null value SHALL be displayed verbatim as inert
text. The browser SHALL NOT construct the old
`full session: <id> · held by <holder>, no resume verb yet` sentence or infer
eligibility from `part.session_id` alone. Client id validation remains a
guard on an already eligible drill, never evidence that a participant owns
a Claude session.

The browser's transcript body and growth routes SHALL be keyed by
participant, for every kind: `/api/transcript/<run>/<key>` and
`/sse/transcript/<run>/<key>`, where `<run>` is a full run id and `<key>`
the participant key, each path component percent-decoded exactly once. A
malformed escape, an empty or extra component, an unknown run or an unknown
participant SHALL be refused before any transcript is read: the body route
answers HTTP 404 with `{"error":"participant not found"}`. The routes SHALL
select the participant's effective reference from the read-only journal by
the same shared selection CLI/TUI use, and accept no path, home or id
override. The body route SHALL read that reference through the one local
read `brokkr transcript` uses, masked against the same secrets store beside
the journal, and answer with that command's `--json` document byte for byte
(the command adds one trailing newline): HTTP 200 when the read is readable,
and HTTP 404 carrying the same document when it is refused. The id-only
`/api/session/<id>` and `/sse/session/<id>` routes and their three-field
Claude envelope are retired; no route looks a transcript up by a bare id.

The browser SHALL offer the `· session <locator>` label and use these
participant routes only for an eligible drill. For an effective valid
Claude reference whose canonical home cannot be established to be the same
as the server's local `HOME/.claude/projects`, it SHALL show
`browser transcript unavailable for this recorded home` and the shared hint
when non-null, keep the checkpoint fallback, and make no body request. An
ineligible or invalid reference SHALL show its shared unavailability and
checkpoint fallback without a session label or drill. Changing
participant/reference eligibility SHALL clear a stale transcript and close
its growth watch before another result is displayed.

That gate is the drill's eligibility, and it is a property of the reference
alone: a drill is eligible when the effective reference is a valid
`codex-thread` or `dsh-session` one, or a valid `claude-session` one whose
canonical home is established to be the same as that local projects home,
whatever the shared presentation's admission state. Every body request,
growth watch and `· session <locator>` label below SHALL require an
eligible drill as well as an admitted source. A Claude reference whose home
equality cannot be established is never eligible, however discoverable and
readable its recorded file is. The server SHALL enforce the same rule, from
the same derivation the presentation reports, before any read: both
participant routes answer an ineligible reference with HTTP 404 and
`{"error":"transcript not found"}`, open no transcript file and write no
event-stream header, so the page's gate is never the only refusal.

Stale browser prose SHALL also be invalidated when an admitted drill's
admission is lost without any participant, reference or journal change.
When a growth watch closes or errors, or a body request is refused, the
browser SHALL close that watch without reconnecting it to the same
source, discard its cached and displayed turns for that source and request the
shared presentation result again. If that fresh result admits no unique safe
source, the page SHALL render the current shared unavailability, hint and
checkpoint fallback. It SHALL NOT keep showing the previous source's turns,
reopen a watch on a refused source, or treat a closed stream as continued
admission. A body or presentation response whose request has been superseded
by another participant, reference or admission state SHALL NOT be displayed
or cached. Recovery SHALL be an ordinary fresh presentation and body request
once shared lookup admits a source again, never a restored cached body.

If that fresh result still admits a unique safe source for the same
participant and reference, the drill is still eligible, and the refusal floor
below does not silence that source, the page SHALL display the current turns
from an ordinary body request and, while that participant is still working,
SHALL open one new growth watch on the admitted source; a concluded
participant SHALL open none, as today. That opening is an admission granted by the fresh shared result,
never a continuation of the closed stream, so a transport drop or a local
server restart SHALL NOT silently end growth for a working seat.

Admission is the presentation result's own state, established by reference
eligibility, identifier and home validation and safe unique discovery. Its
unavailability reason is one of the reference and lookup tokens
`no-reference`, `none`, `unsupported-kind`, `unannounced`, `missing-home`,
`invalid-reference`, `not-found`, `ambiguous-source`, `discovery-limit` and
`unsafe-path`, plus `unreadable` when directory I/O or the bounded DSH
opening-header I/O/UTF-8 check prevents discovery from establishing a safe
unique source. That discovery-stage `unreadable` SHALL be a presentation
unavailability outcome and SHALL close admission. Once safe unique discovery
has admitted a source, a later body-level outcome — `unreadable`,
`unsupported-format` or a readable zero-turn source — belongs to the body: it
SHALL NOT appear as this presentation's unavailability reason and SHALL NOT
change its admission state. The presentation reads no transcript body; the
bounded DSH opening-header read is discovery metadata needed to establish
ownership, not a content projection. Admission is therefore kind-agnostic and
cannot report the browser's own two gates: the
equivalence tuple's `admission state` member carries this shared state alone,
and its `drill eligibility` member carries the reference's kind and, for a
`claude-session` reference, the established home equality above, so a
change in either repaints. An admitted source whose drill is ineligible
SHALL keep its shared full-session information, hint and checkpoint
fallback and SHALL drive no body request and no growth watch on any
occasion, including a recurring re-check, so a foreign-home Claude source
is never drilled and never shows the body-failure prose. A body request is
refused when it does not deliver a successful transcript body: any HTTP 404
(the refused read's document or an error envelope), any other non-success
status, a transport failure with no status, or a body the page cannot
parse. The page SHALL NOT name a reason for a refusal from the body
response; it SHALL show the existing body-failure prose with the
checkpoint fallback beside the shared hint, and display no turns. Where the
fresh presentation it then requests reports its own unavailability, that
reason SHALL be displayed instead, because it is the more current account of
the same source.

A refusal SHALL silence that source until the next re-check. After a refused
body request the page SHALL clear and re-request as above, but SHALL NOT
request that participant and reference's body again, or open a growth watch
on it, before its next recurring re-check, even when the fresh presentation
still admits it. That re-check performs one deferred body request, and only a
successful one may be followed by a watch opening for a working participant.
A discoverable source whose reads keep failing therefore costs one refused
request per re-check, never a refusal-clear-re-request loop. An operator
selection or a participant, reference or eligibility change lifts this
silence, as it lifts the watch bound below.

Automatic recovery SHALL be bounded. The page SHALL open at most one growth
watch for the same selected participant between consecutive recurring
re-checks, counting every automatic opening including one a re-check itself
takes; each re-check begins a new interval and restores that budget. A
closure whose watch budget is already spent SHALL still clear, re-request and
display as above, without a body request wherever the refusal floor silences
that source, but SHALL leave the watch closed until the next re-check, so a
source that closes as soon as it is opened cannot drive an unbounded
reconnect loop, whichever event spent that interval's opening. Presentation
requests need no separate cap because each one follows a closure or a refused
body request, and this rule bounds both: at most one watch opens between
consecutive re-checks, so at most two can close in that span, and the first
refusal silences further body requests until the next re-check. A body
request answering a growth event on an open admitted watch is the page's
ordinary growth repaint, not recovery, and stays bounded by that stream's own
poll cadence. Watches and body requests issued after an operator selection or
a participant, reference or eligibility change are not automatic recovery and
are not bounded by this rule.

While a participant is selected, the browser SHALL re-request its shared
presentation on a recurring re-check that requires no journal-head change,
no re-selection and no other operator action, and SHALL continue that
re-check for a concluded run whose journal head never moves again. Its
interval is a design choice but SHALL recur at least as often as the page's
existing runs poll. Two presentation results are equivalent when their
selected reference, admission state, unavailability reason, shared hint and
drill eligibility all match. A re-check equivalent to the presentation
currently displayed SHALL repaint no displayed turns and change nothing else
it displays. Its only actions are the two deferred repairs, which mend a
missing body or watch rather than a changed result: it SHALL perform one body
request when the drill is eligible, the source is admitted and its body is
missing, and it SHALL open one growth watch, within the bound above, when the
participant is working, its drill is eligible, its source is admitted and no
watch is open, so a deferred reopening resumes at the next re-check. A body is
missing for a participant and reference when no body request for it has
succeeded since the page last discarded its turns for them, and none is
outstanding. A successful body mends it, including a readable zero-turn one —
which the body route answers with HTTP 200 and an empty `turns` array — so
the page SHALL display that success as it displays any other, with no turns,
no unavailability reason and none of the body-failure prose, and no later
equivalent re-check SHALL request that body again. Only a clear or a refusal
makes that body missing once more: a refused request leaves it missing and is
silenced by the floor above until the next re-check, so a permanently
unreadable admitted source costs one request per interval while an admitted
readable empty one costs a single request. A re-check that finds both missing
SHALL make that body request first and open the watch only if it succeeded. A
re-check that turns an unavailable presentation into an admitted one, or an
ineligible drill into an eligible one, SHALL perform an ordinary fresh body
request and, if it succeeds, open one growth watch for a working participant
within that same bound; a re-check that loses admission SHALL apply the
clearing rule above.

The browser's selected-participant presentation result SHALL remain local
and separate from existing inspect/seats/watch JSON and the body route's
transcript document. It SHALL not add transcript prose to journal-derived
run/participant models or journal/export telemetry. Transport of this
presentation result is a design choice; duplicating the eligibility or hint
rules in JavaScript is not.

#### Scenario: A legacy Codex participant never starts a Claude browser drill
- **WHEN** a pre-0032 participant has explicit Codex provenance, legacy `session_id: "abcd-1234"` and no common reference, even with a unique readable local Claude file of that id
- **THEN** the browser, CLI and TUI agree on `no-reference` and null `full_session`; the page shows `transcript unavailable: no-reference` and its checkpoint fallback, with no session label, holder sentence, transcript request or growth watch
- **AND** a separately requested `/api/transcript/<run>/<key>` for that participant returns HTTP 404 with the `no-reference` transcript document and no turns, because the route reads the participant's own selection and never a bare id; `/api/session/abcd-1234` is not a route

#### Scenario: A common reference defeats the browser's stale flat id
- **WHEN** a participant has a stale valid Claude-looking `session_id` beside common kind `none` or an unannounced, invalid or unsupported common reference
- **THEN** the page renders the shared unavailability with null `full_session`, closes any old watch and performs no legacy drill; a valid Codex/DSH common reference instead shows its own shared hint when non-null and never drills the stale id

#### Scenario: An eligible Claude browser participant uses the shared hint
- **WHEN** a common or eligible legacy Claude reference identifies the unique readable file `abcd-1234.jsonl` under the same canonical home as the browser's local projects root
- **THEN** the page shows `· session abcd-1234`, the shared `full session: claude --resume abcd-1234` value and the shared Claude turns; a working participant can use the participant growth route without deriving a second command

#### Scenario: A closed growth stream cannot leave stale browser prose
- **WHEN** an admitted Claude browser drill is displaying turns and watching growth, and a second qualifying file appears, discovery exceeds its bound, or the selected file becomes a symlink, so the next poll closes the stream while the participant, its recorded reference and the journal head are unchanged
- **THEN** the page closes that watch without reconnecting to the same source, discards its cached and displayed turns, requests the shared presentation again and shows the current `ambiguous-source`, `discovery-limit` or `unsafe-path` explanation beside the shared Claude hint and checkpoint fallback; a superseded in-flight response restores neither those turns nor the closed watch
- **AND** when shared lookup later admits a unique safe source again, the next recurring re-check performs an ordinary fresh presentation and body request that displays the current turns and opens one new growth watch while the participant is working, without reusing the discarded cache, a journal-head change or a re-selection

#### Scenario: A dropped stream on a still-admitted source resumes growth
- **WHEN** a working Claude participant's admitted growth watch closes because its transport dropped or the local server restarted, while the participant, its recorded reference and the journal head are unchanged and shared lookup still admits the same unique safe file
- **THEN** the page closes that watch without reconnecting to it, discards its cached turns, requests the shared presentation once, displays the current turns from a fresh body request and opens exactly one new growth watch on the admitted source, rather than relying on the browser's own reconnection of the closed stream
- **AND** if that new watch also closes at once, the page again clears, re-requests and displays that closure's fresh turns but opens no further watch until its next recurring re-check, so repeated immediate closure yields at most one watch opening per re-check

#### Scenario: An unreadable admitted source is asked once per re-check
- **WHEN** a selected working Claude participant's unique safe file stays discoverable while every read of it fails — its consumed bytes are invalid UTF-8, or the read raises an I/O error — so the shared presentation keeps admitting that source and `/api/transcript/<run>/<key>` returns HTTP 404 with the `unreadable` transcript document
- **THEN** the page discards its cached and displayed turns, re-requests the presentation once, shows the existing body-failure prose and checkpoint fallback beside the shared Claude hint, names no reason it cannot distinguish from a lookup failure, and makes no second body request and opens no growth watch for that participant and reference before its next recurring re-check
- **AND** each later re-check makes exactly one further body request, so a permanently unreadable source costs one refused request per interval rather than an unbounded refusal, clear and re-request cycle; a read that fails only after an admitting presentation behaves the same way, because the refusal and not the presentation is what silences the source

#### Scenario: A re-check's own reopening spends that interval's watch budget
- **WHEN** a working participant's deferred growth watch is opened by a recurring re-check on a still-admitted source, and that watch closes immediately while its participant, recorded reference and journal head are unchanged
- **THEN** the page closes it without reconnecting to the same source, discards its turns, re-requests the presentation and displays the current turns from one body request, but opens no second watch before its next recurring re-check
- **AND** an interval whose first opening followed a closure instead behaves identically, so every interval carries at most one automatic opening whichever event took it

#### Scenario: A persisting browser refusal is re-checked without the operator
- **WHEN** a selected participant's transcript is displayed as `ambiguous-source` because the duplicate file remains in place, and its run has concluded so the journal head never moves again
- **THEN** the page re-requests the shared presentation on its recurring re-check and, while each result is equivalent to the displayed one, repaints nothing and issues no body request or growth watch
- **AND** once the duplicate is removed, the next such re-check displays that admitted source's current turns without a re-selection, opening a growth watch only if the participant is still working

#### Scenario: A recorded custom Claude home cannot drill an ambient twin
- **WHEN** a valid Claude participant records a readable file under `/retained/claude-projects` and the browser's different local projects home contains an unrelated file with the same id
- **THEN** CLI/TUI read the recorded file, while the browser shows the same shared Claude hint and `browser transcript unavailable for this recorded home` with its checkpoint fallback; it offers no session drill and reads neither ambient twin nor a guessed browser route
- **AND** a direct `/api/transcript/<run>/<key>` or `/sse/transcript/<run>/<key>` request for that participant returns HTTP 404 with `{"error":"transcript not found"}`, no prose and no event-stream header, while its presentation still admits the source with `drill_eligible: false`

#### Scenario: An admitted Codex or DSH source drills by participant
- **WHEN** a working participant's common `codex-thread` reference names one discoverable safe rollout under its recorded home, or its `dsh-session` reference one discoverable session file, so the shared presentation admits that source with a null unavailability reason and `drill_eligible: true`
- **THEN** the page shows `· session <locator>`, its shared full-session information and the turns `/api/transcript/<run>/<key>` returns, which are the bytes `brokkr transcript --json` prints for that participant without the trailing newline, and opens one growth watch on `/sse/transcript/<run>/<key>`
- **AND** a valid Claude participant whose recorded home is not the browser's local projects home keeps `browser transcript unavailable for this recorded home` while its own presentation state drives no body request or watch

#### Scenario: An admitted empty body is fetched once, not once per re-check
- **WHEN** an eligible Claude drill's admitted unique safe file is readable but projects no turns, so `/api/transcript/<run>/<key>` returns HTTP 200 with its transcript document, an empty `turns` array and `truncated: false`, and the participant's run then concludes so its journal head never moves again
- **THEN** the page displays that successful empty body with no turns, no unavailability reason and none of the body-failure prose, and each later equivalent re-check performs no further body request and opens no growth watch, because the success mended the missing body and a concluded participant watches nothing
- **AND** a further body request follows only a clear or a refusal — a participant, reference, eligibility or admission change, a watch closure, or a refused request — so an admitted readable empty source costs one body request rather than one per re-check interval

#### Scenario: DSH discovery unreadability is a browser presentation refusal
- **WHEN** a selected DSH participant has an otherwise valid common reference but I/O failure or invalid UTF-8 in a bounded opening-header read prevents discovery from establishing a unique depth-zero source
- **THEN** browser participant presentation reports `unreadable` as its shared unavailability reason, with admission closed, null path and DSH hint, the checkpoint fallback and no session label, body request or growth watch; its recurring re-check performs only fresh bounded presentation discovery
- **AND** directory I/O that prevents Claude or Codex discovery from establishing a unique source has the same presentation-level `unreadable` outcome, while a source read that fails only after safe unique discovery admitted it remains the body refusal governed by the once-per-re-check floor

### Requirement: Local lookup rejects paths that escape ownership

Identifier validation SHALL be provider-specific, with one accepted language
per kind across all its local consumers. Claude session identifiers SHALL
be 1 through 64 ASCII hexadecimal-or-hyphen characters, beginning with a
hexadecimal character: `[0-9a-fA-F][0-9a-fA-F-]{0,63}`. This Claude rule
SHALL replace the existing shared session-id guard for CLI reading, TUI
lookup and convenience lines, the browser's participant routes for a
Claude reference, and the browser's Claude client guard.
Leading-hyphen Claude ids previously admitted by the old guard are now
invalid on every surface. No separate permissive legacy Claude rule remains.

Codex thread identifiers SHALL be 1 through 128 ASCII alphanumeric-or-dash
characters, beginning with an ASCII alphanumeric character:
`[A-Za-z0-9][A-Za-z0-9-]{0,127}`. This is the shipped engine's Codex resume
and thread-echo guard language, including non-hex letters and lengths above
64. CLI reading, TUI lookup, and every Codex full-session hint, including
browser participant presentation, SHALL use that language. The browser's
Claude drill guard SHALL not be applied to a Codex reference; a valid Codex
reference drills by participant under its own language. Empty common locators retain
`unannounced` precedence; otherwise an id outside its kind's language SHALL
return `invalid-reference` before lookup or command construction.

Validation SHALL use the entire recorded string without trimming, changing
case, truncating, extending or inferring a suffix. Decision 0032's built-in
80-character recorded-locator clamp SHALL remain unchanged: it describes
what the driver records, not a new 64- or 80-character reader guard. A
recorded prefix SHALL never be repaired by searching for a longer id or
borrowing a flat legacy id. This rule changes no engine/adapter guard,
resumption argv or session ownership behavior owned by #226.
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
- **THEN** CLI/TUI report `invalid-reference` without reading it or forming a command, the browser offers no drill or resume line, and that participant's `/api/transcript/<run>/<key>` and `/sse/transcript/<run>/<key>` both return HTTP 404 before opening the file or a stream, the body route carrying the `invalid-reference` document

#### Scenario: A legacy invalid id uses the same refusal
- **WHEN** a pre-common-reference Claude participant has legacy `session_id: "-abc"`
- **THEN** CLI/TUI report `invalid-reference`, with no synthesized reference, no command and no file lookup; it is not silently classified as `no-reference`

#### Scenario: The browser retains valid-id compatibility
- **WHEN** a Claude id is `a-bC09` and its unique owned local file is readable within the shared limits
- **THEN** the client and server both accept it, the participant body route serves its transcript document with the shared Claude content, and the participant growth stream remains available

#### Scenario: Codex ids retain the engine's accepted language
- **WHEN** common Codex references contain `0199mine` or an ASCII alphanumeric-or-dash id of 65 or 80 characters with an alphanumeric first character, valid absolute homes, and unique safe filename-matching rollouts
- **THEN** CLI and TUI read those files and retain the complete recorded id in the shared Codex hint; browser participant presentation shows that hint and drills by participant without the Claude guard or `invalid-reference`, while the same strings under kind `claude-session` are rejected by its own guard

#### Scenario: Codex validation keeps the engine's length boundaries
- **WHEN** otherwise valid common Codex references have alphanumeric-or-dash ids of 1, 64, 81 or 128 characters with an alphanumeric first character, or ids of 129 characters, `-abc`, `ab_cd`, or non-ASCII letters
- **THEN** the first four pass reference validation and proceed to bounded lookup with their full ids; the latter four return `invalid-reference` before lookup with null hints, without applying the built-in recording clamp as a new reader limit

#### Scenario: A recorded prefix is never expanded into an unrecorded thread
- **WHEN** a common Codex locator is 80 ASCII `a` characters and the only retained rollout names an 81-character id made of those 80 characters followed by another `a`
- **THEN** the recorded id is valid but that filename fails the whole-token match, so the read returns `not-found` with null path and the unresolved Codex hint for exactly the recorded 80 characters; no suffix is recovered and the journal is unchanged

#### Scenario: A symlink and a FIFO are not transcript files
- **WHEN** a matching path is a symlink escaping the seat root, a symlink within the root, or a FIFO, including replacement after discovery
- **THEN** the reader opens no substitute content, does not block waiting for FIFO bytes and returns an unavailable result

#### Scenario: Reading retains the operator's evidence
- **WHEN** either local surface reads a valid transcript or fails to find one
- **THEN** the transcript file, retained root and provider configuration have the same bytes and existence afterward, and no harness process was started

### Requirement: Full-session information belongs to the shared local read result

`transcript-reading` SHALL own the informational `full_session` value; CLI
text/JSON, the TUI and browser participant presentation SHALL consume it
without independently deciding whether a hint exists or which command to
name. The browser receives its hint through the separate local presentation
result, and its participant body route carries the same `full_session` the
CLI's `--json` document carries.
The value SHALL follow this table.
A valid reference here is an effective supported reference with a nonempty
validated locator and absolute validated home, including a synthesized legacy
Claude reference. A confirmed path is the safe unique path established by
lookup; file-read failure after confirmation does not erase that path.

| Kind and resolution | Exact `full_session` value |
|---|---|
| Valid `claude-session`, with or without a confirmed path | `full session: claude --resume <id>` |
| Valid `codex-thread`, confirmed path | `full session: path <display-path>, codex exec resume <id>, home <display-home>` |
| Valid `codex-thread`, no confirmed path | `full session: rollout unavailable, codex exec resume <id>, home <display-home>` |
| Valid `dsh-session`, confirmed path | `full session: path <display-path>` |
| Valid `dsh-session`, no confirmed path | null |
| Absent, `none`, unsupported, unannounced, missing-home or invalid reference | null |

The display path/home placeholders SHALL be reversible portable display
literals. Each is a valid double-quoted JSON string literal whose content
emits only ASCII letters, digits, `/`, `.`, `_`, `-` and `:` directly. Every
other Unicode scalar SHALL use JSON `\u` escapes with lowercase hexadecimal digits, with a surrogate
pair for a scalar outside the basic multilingual plane; JSON short escapes
SHALL NOT be used. Decoding that literal as JSON SHALL recover the exact
Unicode path/home. In particular, whitespace, quotation marks, reverse
solidus, `$`, backtick, `%`, `!` and ASCII shell operators SHALL never occur
raw inside a path/home literal. Fixed fields SHALL use the exact comma
separators in the table and SHALL introduce no semicolon, pipe, ampersand or
redirection operator. The complete line is ordinary display data, then
JSON-escaped again only when serialized as a JSON member. It is not a shell
literal or a pasteable command. Paths SHALL never be guessed from a date or
interpolated into a command; the recorded home is informational and is not an
assertion that an ambient resume command uses that home.
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
- **THEN** `full_session` is null in CLI, TUI and browser participant presentation, and the matching reference failure is returned without invoking a provider

#### Scenario: Agent-controlled paths remain reversible inert display data
- **WHEN** a valid Codex or DSH reference resolves to a safe owned path or recorded home containing spaces, quotes, reverse solidus, `$()`, backticks, semicolons, pipes, percent signs, exclamation marks, non-ASCII text or a line break
- **THEN** the shared `full_session` value uses the exact table framing and portable display literals, JSON-decoding each literal recovers the exact path/home, and no path/home-supplied substitution, quote, command separator, redirection or line boundary remains raw in the complete hint
- **AND** CLI text and JSON, the TUI pane and whole-transcript door, and browser participant presentation consume that identical shared value; submitting the complete displayed hint to a shell cannot evaluate or split any path/home-supplied fragment, and no surface invokes a provider

### Requirement: Claude content preserves the existing projection

Claude SHALL retain the existing user/assistant JSONL projection: message
string content, nonblank `text` blocks, and `tool_use` markers containing the
tool name and optional `input.file_path`, in source order. The role SHALL
come from the message or fall back to the record type, and an absent stamp
SHALL be empty. Other record/block kinds SHALL remain absent from Claude's
projected prose as today; this feature SHALL NOT expand it to arguments,
tool-result bodies or thinking. Claude diagnostic classification SHALL use
the following closed list. "Quiet" means neither `skipped_lines` nor
`unrecognized_records` increases; it does not mean a turn is emitted.

| Record or content case | Projection and diagnostic rule |
|---|---|
| Top-level object with `type: "user"` or `"assistant"` | Project its supported message content as below; count at most once if an unrecognized content representation or block occurs. Extra record/message fields, including usage, role and timestamp metadata, are not content variants and are quiet. |
| Top-level object with type exactly `summary`, `system`, `progress`, `file-history-snapshot` or `queue-operation` | Deliberately omit the entire record, including nested payloads; quiet. This is the complete non-message omission list. |
| User/assistant record with absent or null `message`, or an object message with absent/null `content` or an empty content array | No blocks and no turn; quiet. A recognized envelope need not carry visible content. |
| String-valued `message.content` | Keep the existing single text block, including an empty or whitespace-only string; quiet. The nonblank rule applies only to array text blocks. |
| Array block `type: "text"` | Emit only a nonblank string `text`. Absent/null or blank `text` supplies no block and is quiet; a present non-null non-string `text` counts the source record as unrecognized. |
| Array block `type: "tool_use"` | Emit the name/file-path marker only. A missing/non-string name falls back to `?`; absent/non-string `input.file_path` adds no target. All other input/argument fields are deliberately ignored and quiet, including missing input; they are not recursively classified. |
| Array block type exactly `thinking`, `redacted_thinking`, `tool_result`, `image` or `document` | Deliberately omit the entire block, including its payload and nested content; quiet. This is the complete omitted-block list. |
| Any other top-level type; absent/non-string top-level type; non-object JSON record; non-null non-object `message`; non-null `content` other than string/array; any other, absent or non-string block type, including non-object array elements | Omit unsupported content and count the source record once in `unrecognized_records`, retaining any supported sibling blocks. |

A valid recognized record with no visible blocks SHALL be quiet unless it
contains one of the table's explicit unrecognized cases. No wildcard
"metadata", lifecycle label or unknown type SHALL extend the omission list.
Malformed JSON alone increases `skipped_lines`. Missing/ill-typed optional
role and timestamp fields retain the shipped fallbacks without diagnostic
increments. These classifications are proposed reader policy, not a measured
census of installed Claude record types. Design SHALL check installed-source
evidence when available and return here with evidence before changing this
list. The shared limit and safe-location rules apply to Claude as to the new
kinds.

#### Scenario: Existing Claude text and tool markers survive extraction
- **WHEN** a Claude file contains a string message, text blocks, a blank block, a Read tool with a file path, a Bash tool with arguments, and a thinking block
- **THEN** the same text and `Read · <file-path>` and `Bash` markers appear in order, with no Bash arguments or thinking prose added and no unrecognized-record increment for those omissions

#### Scenario: The shipped Claude projection fixture has fixed diagnostic counts
- **WHEN** the reader consumes the transcript literal in `crates/brokkr-cli/src/ui/tests.rs:628-648` at commissioned base `5bc8cf3`: one complete `not json` line, `{"type":"summary"}`, `{"type":"user"}` with no message, the plain assistant string, and the user array containing text, whitespace text, text with no value, Read/file-path and Bash markers, and thinking
- **THEN** exactly the same two turns and three user blocks survive, `skipped_lines` is 1, `unrecognized_records` is 0, `truncated` is false and `notices` is exactly `["malformed transcript lines skipped: 1"]`; the summary, absent message, blank/missing text, omitted tool arguments and thinking add no unknown-record notice

#### Scenario: Claude's closed omission lists stay quiet
- **WHEN** a valid file contains each of the five omitted top-level record types, recognized user/assistant envelopes with no message or no content, and messages containing only the five omitted block kinds or blank/missing-text blocks
- **THEN** the projection has no turns and both diagnostic counts are zero; it says `no readable turns` without an unrecognized-record notice

#### Scenario: A new Claude kind remains a counted omission
- **WHEN** a complete valid record has `type: "future-record"` and another user record contains supported text plus two `future-block` blocks
- **THEN** only the supported text projects, `skipped_lines` is 0 and `unrecognized_records` is 2, with `unrecognized transcript records: 2`; familiar-looking payload fields cannot exempt either record from the closed-list rule

#### Scenario: Known Claude envelopes do not hide an unsupported content shape
- **WHEN** three complete assistant records respectively have an object-valued `content`, an array with a text block whose `text` is numeric, and an array with an untyped block
- **THEN** each record counts once as unrecognized, producing zero turns, `skipped_lines: 0` and `unrecognized_records: 3`; the no-visible-block rule does not exempt unsupported representations

#### Scenario: The browser drill serves the command's document
- **WHEN** the participant body route reads an eligible Claude participant's supported session within the shared limits
- **THEN** it serves the document `brokkr transcript --json` prints for that participant, without the trailing newline, with the same Claude turn contents from the common derivation

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
prose from anywhere in the snapshot. A recognized content event under
either version with an unsupported nested content/block/chunk variant,
including a version-three `file` block, SHALL keep the earlier
supported-sibling and counted-omission rule; it is not an unknown event
envelope. Recognized quiet event kinds SHALL be enumerated per admitted
version from evidence in design, not inferred from a type prefix, from an
arbitrary claim that a record is metadata, or from the same name being
quiet under another version. The version-zero vocabulary is the captured
0.1.2-rc.1 catalogue: five content kinds, three packed storage rows and 46
quiet kinds. The version-three vocabulary is the read 0.1.5-rc.2
catalogue, transcribed in full at 56 names: four content kinds
(`user/message`, `assistant/message`, `tool/call`, `tool/result`), one
counted omission (`assistant/attempt`) and 51 quiet kinds, namely the 44
version-zero quiet kinds other than `tool/code-dispatch` and
`tool/code-dispatch-start` plus `system/message`,
`deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch` and
`tool/ptc-dispatch-start`. No catalogue name is untranscribed;
`assistant/chunk`, the three packed-row names, the two
`tool/code-dispatch*` names and every name outside the catalogue are
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
false. The browser's participant body route serves the CLI's `--json`
document, so these diagnostic fields and notices reach it unchanged; no
three-field compatibility response remains.

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
- **WHEN** a recognized DSH assistant message under version zero, and again under version three with one of the two being a `file` block, contains readable text plus two unsupported blocks and valid association metadata
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
- **WHEN** a version-three session holds a readable `user/message` followed by one row of each type `system/message`, `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch`, `tool/ptc-dispatch-start`, `todo/write`, `turn/end` and `request/header`, none carrying an `ignorable` marker
- **THEN** the read projects the user message once with zero diagnostic counts and no notice; every listed row is recognized quiet, and `system/message` supplies no `system` turn and no prose, whatever its payload or `surfaceOp`

#### Scenario: Version-3 names are unknown under version zero
- **WHEN** a version-zero session holds a readable `user/message` followed by one `assistant/attempt`, `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch` or `tool/ptc-dispatch-start` row without an `ignorable` marker
- **THEN** each read returns `unsupported-format` with no turns, one unrecognized record and its notice; with top-level `ignorable: true` on that row it projects the user message once with one unrecognized record, and no name is quiet under version zero because it is quiet under version three

#### Scenario: Version-zero-only names are unknown under version three
- **WHEN** a version-three session holds a `tool/call` followed by one `tool/code-dispatch` or `tool/code-dispatch-start` row, or by a row whose type is outside the 56-name version-three catalogue, without an `ignorable` marker
- **THEN** each read returns `unsupported-format` with no turns, one unrecognized record and its notice, and with top-level `ignorable: true` on that row it projects the call once with one unrecognized record; a name outside the catalogue is refused by name, never admitted with the version

