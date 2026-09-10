## Purpose

Let an operator read the retained transcript belonging to a selected seat,
regardless of harness, with one ordered content projection and explicit
limits that preserve local ownership and journal privacy.

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
logical event after per-kind projection and duplicate suppression, not a
physical line number, journal checkpoint or provider's billable turn. A
readable streaming fragment is one logical event; fragments SHALL not be
concatenated into an invented assembled message. Order SHALL be physical row
order, then stored member order within a packed row, never a sort by time,
sequence number or provider turn. Diagnostics SHALL count physical rows;
turn numbering and the display budget SHALL apply to projected logical events.
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

Selecting a present common reference SHALL precede validating it. Its
recorded `kind`, `locator` and `home` strings SHALL remain the selected
reference even when validation refuses lookup; the command's `transcript`
member SHALL preserve them as specified by `transcript-command`. Reference
presence is not validity or a confirmed path. A common reference refused as
`none`, `unsupported-kind`, `unannounced`, `missing-home` or
`invalid-reference` SHALL keep `legacy: false`, null `path` and
`full_session`, no turns and no legacy lookup; it SHALL not be replaced,
normalized or nulled to conceal the refusal.

#### Scenario: A custom home survives an environment change
- **WHEN** a Codex seat recorded home `/retained/codex` and the local process has a different `CODEX_HOME`
- **THEN** lookup uses `/retained/codex/sessions` only, and reports unavailable if that location cannot be read instead of searching the new environment's home

#### Scenario: An empty retry reference supersedes an earlier session
- **WHEN** an earlier attempt named a readable file and the selected participant's later attempt recorded its real kind with an empty locator
- **THEN** the reader reports `unannounced` and does not display the earlier attempt as the current transcript

#### Scenario: Explicit absence defeats a stale compatibility id
- **WHEN** a participant carries kind `none`, or an invalid or unsupported common reference, beside an old `session_id`
- **THEN** no legacy lookup occurs and the corresponding unavailability is returned

#### Scenario: A rejected common reference remains the selected fact
- **WHEN** a participant records `{"kind":"codex-thread","locator":"0199mine","home":""}` beside a stale valid legacy Claude id
- **THEN** the reader returns `missing-home` without lookup, retains exactly that common reference with `legacy: false`, null path and null full-session hint, and no legacy fallback supplies prose

#### Scenario: Old Claude journals remain readable
- **WHEN** a participant has no common reference, a valid legacy session id, and Claude, LaneTally or absent provider provenance
- **THEN** its file is looked up under the local Claude projects home, and the synthesized reference is visibly identified as legacy in the command output

#### Scenario: An old Codex id is not a Claude session
- **WHEN** a participant has only a legacy `session_id` with explicit Codex provenance, even if an identically named Claude file exists
- **THEN** the reader reports `no-reference` and opens neither that Claude file nor a guessed Codex home

#### Scenario: Reading a resumed thread keeps its recorded history
- **WHEN** the selected reference resolves to a file containing several prior attempts of the same thread
- **THEN** the bounded whole-file projection is shown without filtering by checkpoint numbers or changing any resumption behavior

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

The existing `/api/session/<id>` and `/sse/session/<id>` routes SHALL remain
journal-independent, Claude-only lookups under the server's local
`HOME/.claude/projects`. They SHALL use shared validation, discovery and
Claude projection as applicable, with the existing successful wire shapes.
They SHALL neither inspect a participant's provenance nor guess a recorded
home from an id. A direct request is an explicit local Claude-id lookup,
not a request for the transcript of any participant sharing that string.

The browser SHALL offer the `· session <id>` label and use these id-only
routes only for an effective valid `claude-session` reference whose
canonical home is established to be the same as that local projects home.
For an effective valid Claude reference, if this equality cannot be
established, it SHALL show
`browser transcript unavailable for this recorded home` and the shared hint
when non-null, keep the checkpoint fallback, and make no id-only request.
A Codex/DSH reference SHALL retain its shared full-session information and
checkpoint fallback without a Claude drill; browser transcript bodies for
those kinds are outside this change. An ineligible or invalid reference
SHALL show its shared unavailability and checkpoint fallback without a
session label or drill. Changing participant/reference eligibility SHALL
clear a stale transcript and close its growth watch before another result
is displayed.

That gate is the drill's eligibility, and it is a property of the reference
alone: a drill is eligible when the effective reference is a valid
`claude-session` one whose canonical home is established to be the same as
that local projects home, whatever the shared presentation's admission state.
Every id-only body request, growth watch and `· session <id>` label below
SHALL require an eligible drill as well as an admitted source. A Codex or DSH
reference, and a Claude reference whose home equality cannot be established,
are never eligible, however discoverable and readable their recorded file is.

Stale browser prose SHALL also be invalidated when an admitted drill's
admission is lost without any participant, reference or journal change.
When a growth watch closes or errors, or an id-only body request is refused,
the browser SHALL close that watch without reconnecting it to the same
source, discard its cached and displayed turns for that id and request the
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
and its `drill eligibility` member carries the `claude-session` kind and the
established home equality above, so a change in either repaints. An admitted
source whose drill is ineligible SHALL keep its shared full-session
information, hint and checkpoint fallback and SHALL drive no id-only request
and no growth watch on any occasion, including a recurring re-check, so a
discoverable Codex or DSH source is never drilled by a Claude-id lookup and
never shows the body-failure prose. An id-only body request is refused when
it does not deliver a successful transcript body:
either specified 404 envelope, any other non-success status, a transport
failure with no status, or a body the page cannot parse. Because those two
404 envelopes are deliberately indistinguishable, the page SHALL NOT name a
reason for a refusal; it SHALL show the existing body-failure prose with the
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
which the id-only route answers with HTTP 200 and an empty `turns` array — so
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
and separate from existing inspect/seats/watch JSON and the three-field
Claude session response. It SHALL not add transcript prose to journal-derived
run/participant models or journal/export telemetry. Transport of this
presentation result is a design choice; duplicating the eligibility or hint
rules in JavaScript is not.

#### Scenario: A legacy Codex participant never starts a Claude browser drill
- **WHEN** a pre-0032 participant has explicit Codex provenance, legacy `session_id: "abcd-1234"` and no common reference, even with a unique readable local Claude file of that id
- **THEN** the browser, CLI and TUI agree on `no-reference` and null `full_session`; the page shows `transcript unavailable: no-reference` and its checkpoint fallback, with no session label, holder sentence, transcript request or growth watch
- **AND** a separately requested `/api/session/abcd-1234` still returns HTTP 200 with that local Claude file's `session_id`, `turns` and `truncated` because the route is journal-independent; if that Claude file is absent it returns HTTP 404, and neither outcome changes the participant's ineligibility

#### Scenario: A common reference defeats the browser's stale flat id
- **WHEN** a participant has a stale valid Claude-looking `session_id` beside common kind `none` or an unannounced, invalid or unsupported common reference
- **THEN** the page renders the shared unavailability with null `full_session`, closes any old watch and performs no legacy drill; a valid Codex/DSH common reference instead shows its own shared hint when non-null and never drills the stale id

#### Scenario: An eligible Claude browser participant uses the shared hint
- **WHEN** a common or eligible legacy Claude reference identifies the unique readable file `abcd-1234.jsonl` under the same canonical home as the browser's local projects root
- **THEN** the page shows `· session abcd-1234`, the shared `full session: claude --resume abcd-1234` value and the shared Claude turns; a working participant can use the existing growth route without deriving a second command

#### Scenario: A closed growth stream cannot leave stale browser prose
- **WHEN** an admitted Claude browser drill is displaying turns and watching growth, and a second qualifying file appears, discovery exceeds its bound, or the selected file becomes a symlink, so the next poll closes the stream while the participant, its recorded reference and the journal head are unchanged
- **THEN** the page closes that watch without reconnecting to the same source, discards its cached and displayed turns, requests the shared presentation again and shows the current `ambiguous-source`, `discovery-limit` or `unsafe-path` explanation beside the shared Claude hint and checkpoint fallback; a superseded in-flight response restores neither those turns nor the closed watch
- **AND** when shared lookup later admits a unique safe source again, the next recurring re-check performs an ordinary fresh presentation and body request that displays the current turns and opens one new growth watch while the participant is working, without reusing the discarded cache, a journal-head change or a re-selection

#### Scenario: A dropped stream on a still-admitted source resumes growth
- **WHEN** a working Claude participant's admitted growth watch closes because its transport dropped or the local server restarted, while the participant, its recorded reference and the journal head are unchanged and shared lookup still admits the same unique safe file
- **THEN** the page closes that watch without reconnecting to it, discards its cached turns, requests the shared presentation once, displays the current turns from a fresh body request and opens exactly one new growth watch on the admitted source, rather than relying on the browser's own reconnection of the closed stream
- **AND** if that new watch also closes at once, the page again clears, re-requests and displays that closure's fresh turns but opens no further watch until its next recurring re-check, so repeated immediate closure yields at most one watch opening per re-check

#### Scenario: An unreadable admitted source is asked once per re-check
- **WHEN** a selected working Claude participant's unique safe file stays discoverable while every read of it fails — its consumed bytes are invalid UTF-8, or the read raises an I/O error — so the shared presentation keeps admitting that source and `/api/session/<id>` returns HTTP 404 with `{"error":"transcript not found"}`
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

#### Scenario: An admitted Codex source drives no browser drill
- **WHEN** a working participant's common `codex-thread` reference names one discoverable safe rollout under its recorded home, so the shared presentation admits that source with a null unavailability reason, and the operator leaves it selected across consecutive recurring re-checks
- **THEN** the page shows its shared Codex full-session information and checkpoint fallback with no session label, and makes no id-only body request and opens no growth watch on selection or on any of those re-checks, because an admitted source without an eligible drill repairs nothing; it never requests `/api/session/<thread-id>` with that recorded thread id and never shows the body-failure prose
- **AND** a working DSH participant with a discoverable session file, and a valid Claude participant whose recorded home is not the browser's local projects home, behave the same way, the latter keeping `browser transcript unavailable for this recorded home` while its own presentation state drives no body request or watch

#### Scenario: An admitted empty body is fetched once, not once per re-check
- **WHEN** an eligible Claude drill's admitted unique safe file is readable but projects no turns, so `/api/session/<id>` returns HTTP 200 with `session_id`, an empty `turns` array and `truncated: false`, and the participant's run then concludes so its journal head never moves again
- **THEN** the page displays that successful empty body with no turns, no unavailability reason and none of the body-failure prose, and each later equivalent re-check performs no further body request and opens no growth watch, because the success mended the missing body and a concluded participant watches nothing
- **AND** a further body request follows only a clear or a refusal — a participant, reference, eligibility or admission change, a watch closure, or a refused request — so an admitted readable empty source costs one body request rather than one per re-check interval

#### Scenario: DSH discovery unreadability is a browser presentation refusal
- **WHEN** a selected DSH participant has an otherwise valid common reference but I/O failure or invalid UTF-8 in a bounded opening-header read prevents discovery from establishing a unique depth-zero source
- **THEN** browser participant presentation reports `unreadable` as its shared unavailability reason, with admission closed, null path and DSH hint, the checkpoint fallback and no session label, id-only body request or growth watch; its recurring re-check performs only fresh bounded presentation discovery
- **AND** directory I/O that prevents Claude or Codex discovery from establishing a unique source has the same presentation-level `unreadable` outcome, while a source read that fails only after safe unique discovery admitted it remains the body refusal governed by the once-per-re-check floor

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

### Requirement: Local lookup rejects paths that escape ownership

Identifier validation SHALL be provider-specific, with one accepted language
per kind across all its local consumers. Claude session identifiers SHALL
be 1 through 64 ASCII hexadecimal-or-hyphen characters, beginning with a
hexadecimal character: `[0-9a-fA-F][0-9a-fA-F-]{0,63}`. This Claude rule
SHALL replace the existing shared session-id guard for CLI reading, TUI
lookup and convenience lines, the Claude `/api/session/<id>` and
`/sse/session/<id>` routes, and the browser's Claude client guard.
Leading-hyphen Claude ids previously admitted by the old guard are now
invalid on every surface. No separate permissive legacy Claude rule remains.

Codex thread identifiers SHALL be 1 through 128 ASCII alphanumeric-or-dash
characters, beginning with an ASCII alphanumeric character:
`[A-Za-z0-9][A-Za-z0-9-]{0,127}`. This is the shipped engine's Codex resume
and thread-echo guard language, including non-hex letters and lengths above
64. CLI reading, TUI lookup, and every Codex full-session hint, including
browser participant presentation, SHALL use that language. The browser's
Claude drill guard SHALL not be applied to a Codex reference; a valid Codex
reference still SHALL NOT offer a Claude drill. Empty common locators retain
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
- **THEN** CLI/TUI report `invalid-reference` without reading it or forming a command, the browser offers no drill or resume line, and both `/api/session/-abc` and `/sse/session/-abc` return HTTP 404 before opening the file or a stream

#### Scenario: A legacy invalid id uses the same refusal
- **WHEN** a pre-common-reference Claude participant has legacy `session_id: "-abc"`
- **THEN** CLI/TUI report `invalid-reference`, with no synthesized reference, no command and no file lookup; it is not silently classified as `no-reference`

#### Scenario: The browser retains valid-id compatibility
- **WHEN** a Claude id is `a-bC09` and its unique owned local file is readable within the shared limits
- **THEN** the client and server both accept it, the session endpoint retains its `session_id`, `turns` and `truncated` response shape and shared Claude content, and the existing growth stream remains available

#### Scenario: Codex ids retain the engine's accepted language
- **WHEN** common Codex references contain `0199mine` or an ASCII alphanumeric-or-dash id of 65 or 80 characters with an alphanumeric first character, valid absolute homes, and unique safe filename-matching rollouts
- **THEN** CLI and TUI read those files and retain the complete recorded id in the shared Codex hint; browser participant presentation shows that hint without a Claude drill or `invalid-reference`, while the same strings under kind `claude-session` are rejected by its own guard

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
name. The id-only Claude response retains its existing three-field envelope;
the browser receives its hint through the separate local presentation result.
The value SHALL follow this table.
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
- **THEN** `full_session` is null in CLI, TUI and browser participant presentation, and the matching reference failure is returned without invoking a provider

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
- **WHEN** a complete readable interrupted assembly cites some same-step chunks and a later message carries a surface replacement citing earlier messages
- **THEN** only the interrupted assembly's proved chunk sources disappear; uncited chunks, earlier messages and tool events retain audit order, and the later replacement message appears at its recorded position rather than rewriting history

#### Scenario: The display cap can stop between packed members
- **WHEN** a complete valid packed row yields a first text turn of exactly 4,000,000 UTF-8 bytes and a second nonempty text turn, while the encoded row fits the source cap
- **THEN** turn one is retained, turn two is not, `truncated` is true and diagnostics count no unknown record; the packed row is fully validated but is neither one oversized turn nor a means to exceed the display cap

### Requirement: Partial records and read failures remain distinguishable

The following row-classification rules SHALL apply to Claude/Codex bounded
snapshots and to DSH snapshots after version-zero header admission. A rejected
DSH header version SHALL instead retain the zero-count state fixed by discovery
and content admission above; it SHALL not start this event-classification pass.

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

For DSH, after recognizing the header and supported packed storage rows,
a valid JSON row whose event type/envelope is unrecognized SHALL be omitted
successfully only when it is an object with top-level `ignorable` exactly
boolean `true`. Absent, false, null, string, numeric or nested markers SHALL
not permit omission; a scalar cannot carry that marker. Such a required
unknown row SHALL cause `unsupported-format`, with no projected prose from
anywhere in the snapshot. A recognized event with an unsupported nested
content/block/chunk variant SHALL keep the earlier supported-sibling and
counted-omission rule; it is not an unknown event envelope. Recognized quiet
event kinds SHALL be enumerated from evidence in design, not inferred from
a type prefix or an arbitrary claim that a record is metadata. Invalid
packed-row or citation encodings SHALL also cause `unsupported-format` as
specified above, regardless of an ignorable marker.

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
- **WHEN** a recognized DSH assistant message contains readable text plus two unsupported blocks and valid association metadata
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
- **AND** if the opening header is instead version zero, a later `session` row is one unknown event: without top-level `ignorable: true` it causes R14's event refusal and one unrecognized row, while with that marker it is a counted omission and other recognized content remains readable, with no version switch

### Requirement: Every kind obeys the same source and display caps

A read SHALL retain at most **4,000,000 UTF-8 bytes** summed over emitted
block texts, the existing Claude budget. It SHALL retain complete turns in
file order and stop before the first turn that would exceed that budget;
it SHALL NOT skip that turn to include later smaller turns. Equality with
the budget SHALL fit. Independently, reading a selected source SHALL consume
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
Expose readable chunks as logical-event turns. R15 now refines the original
step association using new persisted-source evidence: replace only explicitly
cited chunks of that step, never all chunks merely because an assembly
exists. Inventing an assembled answer or conflating a provider turn with a
step is rejected. The packed mapping and citation rules below answer U2;
any remaining provider mappings stay design evidence under S3.

### R3 / clarification 3 — Unknown content is a separate counted omission

An unknown type is valid JSON, so it is not a malformed line. It is also not
proof that nothing was recorded. Count unknown physical rows (including
partially supported ones) without quoting their payloads. R14 refines the
original universal-success rule for DSH using new provider evidence: unsafe
unknown events also refuse projection, while safe omissions stay readable.
Recognized metadata and intentional omissions remain quiet. Reject the previous bare-empty rendering
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

### R7 / clarification 7 — The Claude identifier guard moves everywhere together

The existing `ui::valid_session_id` guards Claude paths and pasteable
commands; `ui.html` duplicates its language. Leading hyphens can look like
command options, so reject them in all Claude consumers, including legacy
ids, instead of maintaining conflicting guards. This is an explicit
compatibility tightening in the proposal and proposed 0055. The browser
404/no-drill scenario pins the change; valid ids keep their existing wire
shape and Claude projection. R13 preserves this answer while correcting the
unsupported extension of Claude's language to Codex; Codex already rejects
leading hyphens under its own shipped guard.

### R8 / clarification 8 — Invalid depth cannot prove DSH ownership

The shipped `names_the_seats_own_session` uses `as_u64().unwrap_or(0)` and
therefore folds a non-u64 depth as root. Aligning that permissive coercion is
rejected for the new reader: an invalid value does not establish that a
session was not delegated. Only absent legacy depth or explicit unsigned
zero establishes eligibility. The driver-folded-invalid-depth scenario names
the deliberate discrepancy and exact explanation. The adapter and #226's
resumption/launch semantics remain outside this issue; neither the journal
nor the operator's retained file is rewritten to hide the refusal.

### R9 / second-pass clarification 1 — Claude omissions are a closed policy

Adopt the finding: "known deliberately omitted" was undefined. The new table
fixes the quiet top-level and block vocabularies and separates recognized
empty content from unsupported representations. The shipped literal in
`ui/tests.rs:628-648` and projection in `ui.rs:245-305` establish the
`summary`, missing-message/text, tool-marker and thinking compatibility
cases; `adapters.rs:688` separately recognizes Claude `system` lifecycle
records. The additional named omission kinds are explicit proposed policy,
not claimed live observations. An installed record census remains unmeasured.

Counting every dropped field is rejected because ordinary tool arguments
and intentional thinking omissions would permanently advertise format
drift. Exempting every unrendered kind is also rejected because a future
format could then look empty. The exact fixture result is `(1, 0)` for
`(skipped_lines, unrecognized_records)`, and the new-kind scenarios prove that
Claude still participates in the unknown-record protection. No frozen
fixture is edited; implementation extends the existing crate suites.

### R10 / second-pass clarification 2 — The page consumes the same eligibility

Adopt the finding against `ui.html:978-1010`: a flat id is not ownership.
`brokkr-view/src/lib.rs:1959-1971` retains non-Claude legacy ids, whereas
`tui.rs:420-431` already checks their provenance. Leaving the page's
eligibility and holder sentence independent is rejected under decision
0013's single Rust derivation rule. Common references take precedence,
the shared Rust result supplies hints, and the browser performs no Claude
drill for an ineligible legacy participant.

The id-only API intentionally remains journal-independent, as
`ui.rs:71-75` and the drill test state; inferring participant ownership there
would be wrong when several participants share an id. Its direct 200 for an
independent matching Claude file is not permission for the Codex participant
page to fetch it. Repointing those routes to a recorded custom home is also
rejected: an id alone cannot identify that home. Browser presentation remains
an explicit local result, so no inspect/seats JSON change or transcript body
on a journal-derived model is needed. New Codex/DSH browser body routes remain
outside scope.

### R11 / second-pass clarification 3 — Claude compatibility costs are explicit

Adopt all three discovery narrowings and label them `BREAKING` in the
proposal and proposed 0055. The baseline `ui.rs:205-219` returns the first
file, enumerates without a budget and follows symlinks through `is_file`.
Keeping those behaviors only for Claude is rejected: enumeration order
cannot prove a unique source, an unbounded scan can stall every refresh, and
symlink following defeats the same safe-opening rule required for other
kinds. A canonicalized recorded home itself can be a symlink; the refusal
concerns actual symlink entries below it. An ordinary project directory
whose name encodes a symlinked working-directory path is still eligible;
the spelling of a directory name alone does not make that directory a
filesystem symlink.

The API/SSE refusal scenario fixes 404 before stream admission for all three
new classes; an already admitted stream closes when safe unique discovery
fails. The CLI/TUI keep the precise reason and the existing shared Claude
hint. The 32 MiB source bound is also declared as a compatibility limit
because previously readable content can lie beyond it.

Migration guidance for the read-surfaces guide and 0055: the operator can
inspect retained originals outside the bounded reader, resolve duplicate
placement, arrange entries within the recorded home so a unique owned file
can be established within the discovery budget, or use an actual
project/file instead of a symlink below that home. The home remains the
recorded fact; this reader offers no home override and rewrites no
historical reference.
The operator, not the reader, controls any filesystem reorganization; no
automatic deletion, movement, journal rewriting or limit bypass is part of
the change. Accepting 0055 is the operator's ruling on these costs, not this
specification's claim that acceptance has occurred.

### R12 / third-pass clarification 1 — Codex identity keeps its shipped filename rule

Adopt the finding against the undefined mandatory header. At base `5bc8cf3`,
`adapters.rs:957-999` selects through `names_codex_thread` and
`find_codex_thread` without reading content. The token is bounded by ends or
non-ASCII-alphanumerics, not by a guessed UUID parser. The measured-envelope
fixture at `adapters/tests.rs:1157-1168` defines `event_msg` /
`thread_settings_applied` / `payload.thread_id` but proves no required header
position. The echo test at `1180-1216` deliberately finds both `0199other`
without an identifying event and `0199mine` whose settings helper records a
different real thread id. Those are evidence of the shipped identity rule,
not proof of the full current provider format.

Requiring any header or making that optional event a conflict veto is
rejected: either adds an unsupported ownership gate to the very files this
feature must make readable. Remove both the absent-header refusal and the
conflict refusal, rather than deferring their outcome or guessing a new
record type. Filename and recorded scope decide identity; source records
still decide projected content, diagnostics and read failure. The scenarios
pin absence, conflicting payload ids and a large opening record. The DSH
first-record/depth-zero gate and its 65,536-byte bound remain as R8 ruled.
Unique safe files and the common source/display limits still apply to Codex.
This proposal promises local reference resolution, not authentication of
arbitrarily renamed provider bytes, and adds no new provider query.

Proposed 0055 must record this reasoned withdrawal of the earlier Codex
header restriction and bind enforcement to filename-selection and bounded
reader regression tests. No content-schema measurement is claimed; S3's
content-association evidence remains due during design.

### R13 / third-pass clarification 2 — Codex does not borrow Claude's id language

Adopt the finding: at base `5bc8cf3`, `plain_thread_id` in
`adapters.rs:1795-1800` accepts 1–128 ASCII alphanumerics/dashes with no leading
dash. `CodexThreadEcho::locate` and `codex_launch` both use it. Applying
Claude's hex/64 rule for the first time to those same recorded Codex ids is
rejected: it makes an engine-eligible session unreadable without additional
ownership or shell-safety evidence. Reader and hints now accept exactly the
engine's Codex language while R7's Claude tightening stays intact.

The 80-character limit in `transcript.rs:15,73` governs built-in recording;
it neither expands nor narrows the reader's lexical guard, and it is not
changed here. A clipped id cannot be reconstructed from this journal fact.
The prefix scenario fixes the result when an alphanumeric continuation makes
the original filename ineligible. The id-boundary scenarios are synthetic
compatibility cases, not claimed live provider identifiers or resume tests.
Current provider id generation remains unmeasured in this box; matching the
existing Brokkr guard does not require a paid model experiment.

Proposed 0055 must carry the per-kind languages, unchanged recording limit
and refusal/prefix outcomes, with shared-reader and cross-surface tests as
its enforcement binding. It proposes no change to decision 0030's launch or
sandbox law and needs no sibling-worktree changes.

### R14 / design return U1 — Required DSH unknowns refuse projection

Adopt U1. The tagged DSH SessionEvent contract cited in proposal S9 makes
only an explicit true marker evidence that an unknown event can be dropped.
A counted but plausible partial conversation would not establish that its
associations survived that event. The unsupported-format refusal preserves
R3's JSON-vs-malformed distinction and local diagnostics; it changes only
the unsupported DSH semantic cases, with the returned evidence as its reason.
Unknown nested content within a recognized event still follows R3.

Reject a generic `unreadable` for semantic refusal: successfully read bytes
and an unsupported interpretation are different facts. Reject returning any
turns, including an early selected turn, after required-unknown refusal.
Whole-prefix counts and source-only truncation remain reproducible without
claiming a valid display projection. An unusable I/O/UTF-8 snapshot instead
has no complete-prefix classification and retains no partial counts. These
choices are pinned by the marker, collision and refusal scenarios, not left
to renderer order. No additional public fields or frozen schema changes are
needed; the command delta closes the new reason token before publication.
Proposed 0055 must bind these rules to shared refusal-state/diagnostic tests,
CLI whole/selected errors and TUI stale-content clearing.

### R15 / design return U2 — Decode physical storage before applying citations

Adopt U2 with the tagged writer/codec/type evidence in proposal S9. A packed
row preserves several logical events; its byte budget, diagnostic unit and
projection unit must not be conflated. Preserve every readable fragment and
its reconstructed timestamp. Reject concatenating a row, treating it as an
ordinary unknown event, skipping a structurally invalid packed run, or
expanding a citation range into attacker-sized synthetic history. Full-row
validation can coexist with incremental member projection; resource use is
bounded by retained bytes/members and observed identities.

Refine R2: the recorded turn/step pair scopes a citation but cannot create
one. SourceEventSeqs supplies the positive evidence; absent/empty/partial
lists cannot silently erase uncited fragments. The inclusive range syntax
comes from the persistence writer; treating valid entries as set membership
and refusing malformed citation encodings are explicit reader policy. They
are not unmeasured assertions about the provider's full replay validator.
Identity ambiguity preserves content instead of guessing. Surface replacement
and request reconstruction stay outside an audit transcript; only the proved
chunk/tool echo relationships are suppressed.

The packed/unpacked, row-boundary, malformed encoding, range/scope,
interrupted-assembly and display-budget scenarios cover U2's owning outcomes.
Proposed 0055 must record the physical-row diagnostic and source-cap unit,
logical-event turn/display unit, exact timestamps, citation admission and
bounded matching, and audit-order rule. Bind them to synthetic packed/plain
projection equivalence, refusal/count tests and CLI/TUI selection/refresh
conformance. Accepted decisions and provider originals remain unchanged.

Successor source check (proposal S10): controller-captured
`@deepseek-ai/dsh-session` 0.1.2-rc.1 supplies the previously unavailable
`lib/types/seq-ranges.js`, SHA-256
`68a127c76affa98edeeb50e302eb43f154f4d24f7d04cb95ba8b323e88f3d09e`.
Its decoder expands inclusive ranges and rejects non-increasing expanded
lists when any range is present. This verifies the storage syntax; it does
not replace the reader's settled admission rule with the provider's replay
validator. The captured `lib/types/chunk-rows.js`, SHA-256
`5724c4f798ed07e77406ab13a75685622a3e08868f257cbd142949099f0ea4c2`,
confirms the packed shapes and preserved individual fragments specified
above. The capture hashes were checked without accessing installed host
files or executing provider code.

Keep duplicate, overlapping and out-of-order valid citations as sets:
Brokkr tests membership only against observed, uniquely identified earlier
same-step chunks. Reordering a citation list cannot change that proof or
the physical audit order. Importing the provider's range-order refusal or
expanding every range is rejected because neither is needed for that
bounded membership test. Required unknown events, malformed encodings and
self/future citations still refuse under R14/R15. The new citation-entry
scenario concretizes this existing policy and its existing CLI/TUI
consequences; it changes no output field or previous answer. Proposed 0055
must state this deliberate distinction from the captured replay decoder
and bind it to citation-set and cross-surface regression tests. No claim
of a live DSH read or resumption follows from this source inspection.

### R16 / returned clarification — DSH ownership and version admission are distinct

Adopt the returned finding with proposal S11's hash-verified controller
capture. The installed session package declares format version zero; the
persistence backend checks a foreign numeric version before validating the
current replay-header shape or decoding events. A familiar event layout is
not evidence that another version means the same thing. Require numeric
zero and reject missing, mistyped and foreign versions without invoking the
version-zero decoder. Numeric zero spellings share the captured guard's
value semantics; this does not weaken R8's separate unsigned-depth rule.

Ownership still precedes format admission. Unlike the provider backend,
which is handed a chosen session artifact, Brokkr must first identify the
selected seat's unique root without reading a delegated session as its own.
Preserve the shipped first-record `type`/depth ownership predicate with
R8's explicit invalid-depth correction. Do not use a supported version to
break an ownership tie or silently report an owned foreign file as missing.
A foreign header whose depth is invalid cannot prove ownership; a unique
valid-depth header with a foreign version proves only a location, which
remains available for the operator to inspect independently.

Keep unused header metadata outside admission instead of importing the full
replay validator (`id`, `createdAt`, seed/origin/preset and retired policy
fields). This is a deliberate audit-reader policy: these fields establish
neither the recorded root's ownership nor the supported event encoding, and
no execution history or configuration is being restored. It preserves the
settled legacy omitted-depth answer without inventing an absent-version
legacy format. The minimal version-zero and malformed-header scenarios
make the supported shape and its limits explicit; later session rows are
not new opening headers.

Preserve source I/O/UTF-8 precedence and measure source truncation with the
same bounded byte snapshot. After version rejection, do not decode events
merely to populate R14's counters: unknown version semantics cannot justify
version-zero physical/logical event classification. Zero counts denote an
unstarted classification pass, not a valid or empty body. Reject both a
made-up unrecognized-header count and counting every subsequent JSON row
under an unadmitted grammar. Current-version event/storage refusals still
keep R14/R15's whole-prefix counts; their prior scenarios and citation-set
policy remain unchanged. No new output field, error token or version of the
unpublished command document is needed.

Proposed 0055 must carry this supported-version policy, minimal audit-header
shape, explicit divergences from replay validation and adapter folding,
ownership-before-admission order and both distinct `unsupported-format`
states. Bind it to synthetic header/version/depth/uniqueness matrix tests,
source-cap and failure-precedence tests, CLI whole/selected text/JSON states
and TUI refusal/recovery tests. No live provider experiment is required to
choose this policy, and the captured source supplies no new Codex association
proof or evidence of #226's resumption behavior.

### R17 / specify adoption — Lost admission is not continued browser admission

The browser's stale-clearing rule was bound only to a changing participant
or reference, so an already admitted drill whose shared lookup later refuses
had no stated client behavior. At base `5bc8cf3`,
`crates/brokkr-cli/src/ui.html:757-782` keys `transcriptCache` by session id
and drops an entry only inside `sessionSource.onmessage`; `closeSessionWatch`
closes the stream without discarding that entry, and the session stream has
no `onerror` handler, so the browser `EventSource` reconnects by default. The
run stream's handler at `ui.html:1118` closes the session watch but likewise
drops no cached body. Under R11's new refusals the server correctly closes
the stream and answers 404 while the page keeps displaying the former
source's turns. Leaving that to a renderer choice is rejected: the TUI
already clears refused content under T5/T7, and the same evidence must not
remain visible in the browser merely because it was fetched before the
refusal.

Reconnecting the watch to the same source is rejected, because shared lookup
has refused it and a retry is either a 404 loop or a silent return to
ambiguous evidence. Discarding the cache and re-requesting the shared
presentation keeps one owner for eligibility, refusal and hint. Superseded
responses are excluded because an in-flight body can otherwise land after
the refusal and restore exactly the prose it removed. Recovery is an ordinary
fresh read rather than a revived cache, so the page cannot display a snapshot
the current lookup would not admit. This adds no browser transcript
transport, route, wire field or Codex/DSH browser body, changes no accepted
decision and does not reopen R10's eligibility or R11's refusal answers.
Proposed 0055 must carry this browser stale-content rule and bind it to a
client admission-loss test with unchanged participant, reference and journal.

### R18 / seventh-pass clarification 1 — A still-admitted source is rewatched

R17's trigger is every closure, but its outcome was written only for the
refusal branch; the common branch — the fresh presentation still admits the
same source — was left to the deltas' one permissive verb. That verb decided
whether a working seat keeps following prose, and the same rule removes the
mechanism that supplies it today. At base `5bc8cf3` the page installs no
`onerror` on `sessionSource` (`crates/brokkr-cli/src/ui.html:770-782`; the
only `onerror` is the run stream's at `ui.html:1118`), so a dropped
connection is reconnected by `EventSource` itself, and the shipped stream
ends an admitted watch only on a failed client write or the test-only
`sse_limit` (`crates/brokkr-cli/src/ui.rs:429-461`). A client therefore
observes one indistinguishable close for a transport drop, a server exit and
— under R11 — a lookup refusal, because SSE carries no status mid-stream;
re-asking the presentation is the only way to tell them apart, which is why
the broad trigger stays.

Adopt reopening on the fresh admission. Leaving it optional is rejected:
two conforming pages would then differ for a live seat after a transient
drop, one still showing prose land between checkpoints and the other frozen
until the journal head moves, which is the reason the growth route exists
(`ui.rs:7-10`). Restoring the browser's automatic reconnection is rejected
for R17's reason — the source may since have been refused — so the
replacement is an admission-checked reopen. Growth recovery is preserved but
changes mechanism and cadence: it follows a presentation round trip and
R19's re-check instead of the browser's retry timer, and the proposal
declares that difference beside R11's costs.

The bound exists so the replacement cannot spin where `EventSource` imposed
its own retry delay. Bounding the openings is enough: one automatic opening
per selected participant between recurring re-checks turns a stream that
closes on every attempt into a bounded retry at the re-check cadence, and
the clear-and-re-request cycle then terminates on its own, because a
closure can only follow an opening. Capping the presentation request
instead was rejected: it would contradict the clearing rule, which must
re-ask on every closure to learn whether the source is still admitted, and
that request carries no transcript body. Operator selections stay unbounded
because they are not automatic recovery, and the shipped rule that a
concluded participant opens no watch is unchanged. Proposed 0055 must
carry the reopen rule, its bound and the declared recovery-mechanism
change, bound to the client admission-loss tests.

### R19 / seventh-pass clarification 2 — Recovery needs a stated occasion

R17 named exactly one presentation request, the immediate one made when the
watch closed, while its scenario asserted eventual recovery. If that
immediate request also refuses — the ordinary case, since a duplicate file
or a symlink does not disappear within the same second — nothing said when
the page looks again. The shipped page offers no occasion under the
scenario's own unchanged participant, reference and journal premise:
`loadDetail` runs from `select()` and from the run stream's `onmessage`
(`ui.html:818-821`, `ui.html:1117`), that stream emits data only when the
journal head moves (`ui.rs:464-490`), and the five-second interval repaints
only the runs list (`ui.html:1127`, `ui.html:345-357`). For a concluded run
whose head never moves again, the refusal would stand until the operator
re-selected the participant.

Adopt a recurring presentation re-check, at least as frequent as that
existing runs poll, and make the recovery clause depend on it. Requiring an
operator action instead is rejected: the TUI recovers on its own enumerated
refresh occasions under T5/T7, and a console that needs a re-selection to
notice the ambiguity is resolved contradicts this change's own live-refresh
answer. Binding recovery to the run stream is rejected because that stream
is silent exactly when the premise holds.

The re-check carries no transcript body: it is the local presentation result
this requirement already owns, and its bounded discovery is no heavier than
the per-poll revalidation this change requires of an admitted growth stream
at the shipped one-second `SSE_POLL` (`ui.rs:393`). The equivalence rule
keeps an unchanged re-check invisible, so displayed turns, their scroll
position and an open watch survive it and only a changed admission state,
reference, reason, hint or drill eligibility repaints; the one exception is
R18's deferred reopening, which is taken at the next re-check because a
missing watch, not a changed result, is what it repairs. Both directions then
have observable outcomes: a persisting refusal repaints nothing, and a
restored unique safe source displays its turns without a re-selection.
Proposed 0055 must carry this occasion and the equivalence rule with their
client bindings.

### R20 / eighth-pass clarification 1 — Admission is discovery; a refusal has a floor

R17's clearing trigger includes a refused id-only body request, and R18's
branch answers every trigger with another body request, so the two rules
admit a cycle — request, refusal, clear, presentation, request — that R18's
bound does not reach: its premise is that every request follows a closure,
which is false for the refusal trigger. The cycle is reachable without a
race, because the id-only route returns HTTP 404 for read failures as well as
lookup failures and `unreadable` is established after a unique safe source
exists. The branch predicate also never said which member it reads.

Fix the predicate first: admission is the presentation result's own state,
which reference eligibility, identifier/home validation and safe unique
discovery establish. Making it the whole read instead is rejected. R19 priced
the recurring re-check as bounded discovery, no heavier than the per-poll
revalidation of an admitted stream at the shipped one-second `SSE_POLL`
(`crates/brokkr-cli/src/ui.rs:393`); reading a capped source for every
selected participant on every re-check is a different cost, and the browser
takes no Codex/DSH body at all in this change, so a presentation that read
transcript bodies would carry read outcomes it cannot use. An `unreadable`
result established after safe unique discovery, `unsupported-format` and a
readable zero-turn source therefore stay body outcomes and never move the
admission state or the equivalence tuple's unavailability reason. A discovery
I/O/UTF-8 failure that prevents that unique answer is instead the presentation
unavailability later made explicit by R24.

That answer leaves the refusal cycle real, so it is bounded where it starts.
A refusal silences its source until the next re-check: the page still clears
and re-requests, because that is how it learns whether the reference is still
eligible, but it asks no second body and opens no watch on that source in the
same interval. Capping the presentation request instead is still rejected for
R18's reason. Reusing the watch bound alone is not enough, because the
refusal path opens no watch to spend it. Inventing a reason for the refusal is
rejected: this change deliberately gives lookup and read failures the same
`{"error":"transcript not found"}` envelope, so the page keeps the shipped
body-failure prose (`crates/brokkr-cli/src/ui.html:791-795`) with the
checkpoint fallback and claims nothing it cannot observe; the fresh
presentation's own reason, when it has one, is the more current account and
displaces that prose. Growth repaints on
an open admitted watch stay outside the bound: they are the shipped growth
mechanism, already paced by that stream's own poll. Proposed 0055 must carry
the admission predicate, the refusal definition and this floor, bound to a
client proof that a permanently unreadable admitted source issues one body
request per re-check.

### R21 / eighth-pass clarification 2 — Every automatic opening spends the budget

R18's bound was written twice and incompatibly: "Apart from a recurring
re-check itself" exempted the re-check's own opening from the count, while
R19's deferred reopening was required to happen "within the bound above". The
difference is observable — for a source that closes as soon as it is opened,
the exemption yields two automatic openings per interval and the inclusion
one — and the tasks office's client proof needs that number.

Delete the exemption. Every automatic opening, whether a re-check takes it or
a closure drives it, spends the single per-interval budget, and each re-check
begins a new interval that restores it. This is what "within the bound above"
and the dropped-stream scenario's gloss already assert, so the repair keeps
both readings' shared intent and discards the sentence that contradicted
them. Exempting the re-check instead is rejected: it would double the steady
retry rate of a flapping source for no stated benefit, and it would leave the
re-check-first trace ungoverned, which is exactly the case R19 introduced.
The bound now has one number — one automatic opening per participant per
re-check interval — and its scenario pins the re-check-first trace beside the
closure-first one. Proposed 0055 must carry the single counting rule.

### R22 / ninth-pass clarification 1 — Admission alone never drills

R20 defined admission in the shared reader's kind-agnostic vocabulary, whose
ten reference and lookup tokens cannot express the browser's own two gates:
the `claude-session` kind and the established home equality. The recovery
branch and both deferred repairs then keyed on admission alone, so their
literal reading drilled the very participants this requirement excludes — a
Codex or DSH seat with a discoverable retained file, which is #222's own
case, and a valid Claude seat whose recorded home is not the browser's. The
outcome was visible, not merely wasteful: `/api/session/<codex-thread-id>` is
a Claude-only lookup, its 404 would trip the refusal floor, and the page
would show body-failure prose for a seat this requirement says must show its
shared full-session information — one refused request per re-check, forever.

Name the gate and require it beside admission. Drill eligibility is a
property of the reference alone, so an ineligible drill's admitted source
repairs nothing: no id-only request, no watch, on selection or on any
re-check. Reading the "only for" gate as an implicit total constraint was
rejected: it was the reading the amendment removed, and an unstated
constraint cannot govern three explicit action rules. Widening admission to
carry the kind and home gates was also rejected, because the presentation is
the shared derivation's own result for every kind and CLI/TUI consume it
unchanged; the browser's gates are client policy, which is why the
equivalence tuple lists `drill eligibility` beside `admission state`. That
tuple's two members are now defined: the shared state alone, and the client
gate. Proposed 0055 must carry the eligibility gate on both repairs and the
recovery branch, bound to a client proof that a selected working Codex
participant with a discoverable rollout issues no body request and opens no
watch across consecutive re-checks.

### R23 / ninth-pass clarification 2 — A successful body is never missing

The deferred body repair's gloss and its predicate disagreed for a source
this change deliberately keeps readable with no turns: "mend a missing body"
against "no turns are displayed for it". A readable zero-turn source stays
admitted under R20 and the id-only route answers it with HTTP 200 and an
empty `turns` array, reachable without an empty file from a source of only
unrecognized records. Under the predicate a concluded seat with such a source
would re-request its body at the re-check cadence forever, unbounded: the
refusal floor cannot reach it, because that request follows a success.

Adopt the gloss and state its predicate. A body is missing when no request
for it has succeeded since the page last discarded its turns and none is
outstanding; a successful body — including a zero-turn one — mends it, and
only a clear or a refusal makes it missing again. The "outstanding" clause
keeps a growth repaint's own in-flight request from reading as a missing
body at a re-check that overlaps it. Repeated fetching of an admitted empty
source was rejected: it buys nothing the growth watch does not already
deliver for a working seat, and it costs a request per interval for every
concluded seat whose transcript is empty or unrecognized. Showing the
body-failure prose for that success was rejected too: the request succeeded,
this change gives failures a deliberately unnameable reason, and the page
would then claim a failure it did not observe. The tasks office's client
proof therefore has one number for an admitted readable empty source: one
body request, not one per re-check. Proposed 0055 must carry the missing-body
definition and this single-fetch outcome.

### R24 / successor finding F1 — Discovery unreadability closes presentation admission

R20 correctly kept a read failure after safe unique discovery in the body
route, but incorrectly generalized that answer to every `unreadable` result.
The shared discovery requirement already returns `unreadable` when directory
I/O or the bounded DSH opening-header I/O/UTF-8 check prevents a unique
ownership answer. Presentation must perform that bounded discovery to report
the selected participant's shared state. DSH has no browser body route in this
change, so assigning that failure only to a body left it with no conforming
presentation outcome.

Split the token by the stage that established it. Before a safe unique source
is admitted, discovery-stage `unreadable` is the presentation's current
unavailability reason and closes admission. After safe unique discovery,
source I/O/UTF-8 failure remains a body outcome; for an eligible Claude drill
the indistinguishable 404 and R20's refusal floor still apply unchanged.
Removing `unreadable` from discovery is rejected because guessing around a
failed directory or DSH ownership read would violate the shared uniqueness
rule. Adding a DSH body route is rejected because the commission expressly
keeps Codex/DSH browser bodies out of scope. Making presentation project a
bounded transcript body is also rejected for R20's cost and transport reasons:
the DSH opening header is only the already-required discovery metadata.

The scenario above pins both the DSH no-body case and the directory-discovery
case, while preserving R20's admitted-Claude body trace. Proposed 0055 must
carry this stage distinction and the browser presentation proof must cover it.
