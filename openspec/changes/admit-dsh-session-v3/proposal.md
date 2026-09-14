# Change: Admit DSH session format version 3 on measured evidence (#279)

## Why

PR #278 (merged `d330765`) taught discovery the filename the installed DSH
core writes: `session.v3.jsonl` is found beside `session.jsonl` and its
path is confirmed. The reader then refuses the content with
`unsupported-format`, because the living `transcript-reading` requirement
admits only an opening header whose `version` is numerically zero and says
there is no legacy default version and no automatic migration. Issue #279
asks for one of two outcomes: a measured admission of version 3, or a
precise statement of what must be built first.

This change delivers the measured admission. Its first sitting (`23fafd1`)
could not reach the `@deepseek-ai/dsh` 0.1.5-rc.1 writer from its box and
kept version 3 refused. The design council met the prerequisite inside
this run: its simplicity seat resolved the installed `dsh` on this host,
read the session, persistence, catalogue, migration and message packages
beneath it and recorded digests and line citations; the design ruled that
read the run's primary evidence (`c1d57e9`, `f34279e`). The second and
third sittings admitted the header, vocabulary and `tool/call` on it but
kept the three message-carrying kinds refused on the ground that their
message and block definitions had not been read. The analyze judge found
that ground refuted by the design's own record: the definitions were read
(`dsh-llm/lib/types/message.d.ts`, `dsh-llm/lib/types/types.d.ts`,
design D2 E5), mapped field by field onto the reader's parser (D13) and
adopted as measurement (D1). This sitting answers that finding and the six
beside it (S8): the four content kinds project under version three on
their own writer's definitions, and the two parts of the writer that were
not read keep exactly the rule each could break withheld, with the read
that lifts each named. The fifth sitting answers the clarify judge's four
findings on that record (CLARIFY-279-5 to 8, S8), each a contradiction
between things this change itself recorded: the envelope evidence both
excluded and required top-level members beyond the base four; the promise
that a version-three user message never associates was not enforced by
the parser the plan left unchanged; the promise to keep version-zero
exactness byte for byte contradicted the shipped zero predicate, which
admits `10e-400`; and an optional raw-token check could not keep the
exact-number promise for escaped or duplicate `version` members. Each is
repaired in its owning artifact and carried through the rest. The sixth
sitting answers the clarify judge's four findings on that repair
(CLARIFY-279-5 on its second visit, and CLARIFY-279-9 to 11, S8): a
`sourceEventSeqs` permission inferred from one prohibition and presented
as measured; a ruling that generalised the digit signature past the
header integers it was proved for, which read literally would invalidate
`1e3` as a timestamp; a helper algorithm that gave `.` the empty
signature while its own test required none; and a promise to leave every
existing test untouched that the plan's own signature changes could not
keep. The seventh sitting answers the clarify judge's third visit of
CLARIFY-279-5, which asks for the `sourceEventSeqs` permission cells
themselves: they exist only in the installed writer, which no
specification box of this run reaches, so the specification refuses to
carry them as a requirement and assigns their read to the design council
at its next sitting (S6, S8). The eighth sitting answers the clarify
judge's fourth visit of CLARIFY-279-5 (S8), which found that the seventh
sitting's repair had left a contradiction of its own: the growth rule
still demanded, as a condition of a version joining the set, a record of
the two conditional members on every row that carries them, while S6
placed the `sourceEventSeqs` cells this change does not deliver out of
scope. The operator ruled that the original commission's allowance
governs: a part the performed reads did not reach is left refused or
unread with the specific further read named, and that is a correct and
complete outcome. The growth rule and the envelope requirement now state
what the reads established and what is unread pending the named read, S6
records the ruling, and a scenario falsifies the claim that no
disposition rests on an unread cell (S2, S6, S8). The ninth sitting
answers the clarify judge's one finding on that scenario
(CLARIFY-279-12, S8): its modifier "each carrying
`sourceEventSeqs: null`" could be read to reach the readable user
message, which the settled citation rule refuses, so the three stated
outcomes could not all follow from the stated input. The scenario now gives the user
message no citation member, places the refusing members only on the
rows the reader never consults, and adds a fourth file that moves the
same member onto the user message and refuses, as the negative control;
design D12 and task 2.10 carry the same concrete rows.

## What Changes

- The admitted DSH version set becomes closed and exactly `{0, 3}`. Zero
  stays admitted on the #222 capture of the 0.1.2-rc.1 writer; three is
  admitted on the council's read of the installed 0.1.5-rc.1 writer
  (session packages 0.1.5-rc.2), cited by package, file, digest and line in
  the design (D2). Zero and three are admitted only in exact numeric
  spellings, judged on the recorded token's digit signature and the parsed
  value together, with the token bound to the one decoded top-level
  `version` member; versions 1, 2 and every other value keep the refusal.
  The growth rule names what a further version's change records from its
  own reads and how it leaves what those reads did not reach: refused or
  unread, with the lifting read named, never demanded as an inventory
  (S2).
- **BREAKING, bounded to shapes no writer emits:** two corrections to the
  shipped reader's exactness judgment, applied under version zero as well
  as three. A token with a nonzero mantissa digit that underflows to zero,
  such as `10e-400` or `0.1e-400`, is no longer a zero spelling: as a
  header version it refuses instead of admitting, as an ordinary `time` it
  renders an empty stamp instead of `"0"`, and as a packed `time0` it
  refuses the row. A `version`, `time` or `time0` member recorded more than
  once, or whose recorded token cannot be found although the parser read a
  number, no longer passes on the parsed value: the header refuses, the
  time is invalid, the packed row refuses; an escaped member name now binds
  to its field. The version-zero requirement text does not change; the
  reader's behaviour toward it does, and this change says so rather than
  claiming byte-for-byte preservation (S2).
- Row classification is keyed on the admitted version as well as the type.
  Under version three the four content kinds project, `assistant/attempt`
  is a counted omission, `system/message` and six other new names are
  quiet, `assistant/chunk`, the three packed storage rows and the two
  `tool/code-dispatch*` names are required unknowns; the eight
  version-three names are required unknowns under version zero. No name
  inherits a disposition across versions by string equality (S2, S3).
- **Version-three message payloads project on the 0.1.5-rc.2 message and
  block definitions.** `user/message` carries its message directly in
  `data`, `assistant/message` and `tool/result` carry theirs in
  `data.message`; the block union is `text`, `reasoning`, `image`,
  `tool-call`, `tool-result` and `file`, and every field the reader
  consumes maps onto the version-zero parser (design D13). A `file` block
  is a counted unrecognized block that keeps its siblings. The
  version-three `user/message` definition declares no `turn` or `step`, so
  the reader reads none from it under version three, even when a row
  supplies them, and it never takes part in the dedicated-tool association
  (S3, Q1, Q6).
- Version 3 persists whole messages. An interrupted step is one finalized
  `assistant/message` with `interrupted: true`, projected as recorded with
  no marker; `interrupted`, `stream` and `usage` are never expanded; the
  reader assembles nothing under either version (S3, Q2).
- `isSeeded` is inert for ownership, delegation depth, admission and
  classification over every JSON shape. The seed and fork path was not
  read, so under version three the dedicated-tool association, which
  suppresses an embedded copy on a proved call identity, runs only when the
  header records `isSeeded` exactly `false`; a seeded or unattested session
  keeps every embedded copy visible (S3, Q3; S4).
- The version-three physical envelope is recorded as a base of `type`,
  `seq`, `time` and `data` on every event row plus two conditional
  top-level members, `surfaceOp` and `sourceEventSeqs`, with the per-type
  permissions the read recorded: `surfaceOp` for every type,
  `sourceEventSeqs` for `assistant/message` only, its permission on every
  other type recorded as unread and assigned to the design council's
  read (d) at its next sitting (design D2 E3, E4, U4, D13; S6); the reader
  validates no row's key set and neither requires nor refuses either
  member.
  `surfaceOp` is never parsed and never removes, reorders or replaces a
  row. A replace-marked row projects by its type at its recorded position,
  exactly as version zero's compaction message does; the append and replace
  writer paths were not read, and the rule reads no marker, so that gap
  changes no disposition (S3, Q4; S4).
- The recorded-token exactness check judges the header `version`, an
  ordinary event's `time` and a packed row's `time0`, under either version,
  and no other field. One digit-signature helper and one member-binding
  rule serve all three sites, with a predicate per site: the header is
  judged on parsed value and signature together, the two time sites on
  their parsed value as signed safe integers, the signature deciding only
  whether a parsed zero is a zero spelling, so `1e3` and `1000.0` stay the
  millisecond `1000` (S2, S5 ruling 7).
- Discovery is unchanged. The versioned name stays the exclusive candidate
  when its opening row is a valid session header, and the requirement pins
  the mismatched-version pairings (S6).
- The `transcript-tui` requirement's phrase "version-zero event/storage
  refusal" widens to every admitted version; nothing observable changes
  (S6).
- The projector gains what the version-three vocabulary needs and nothing
  more, and host-independent tests pin every disposition with synthetic
  rows. A `Status: proposed` decision at the next free number carries the
  admission and the evidence standard, supplementing 0055 ruling 3 (S5).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `transcript-reading`: "Discovery identifies one owned local file" (the
  admitted set `{0, 3}`, exact-three spellings, the evidence behind both
  versions, the two unread paths and the unread permission, the growth
  rule that demands no inventory, the exactness scope, `isSeeded` over
  every shape and its one effect, the sibling and filename/version
  pinnings); "DSH sessions expose assembled or provisional content once"
  (whole messages under version three, the payload projection on the
  version-three definitions, `assistant/attempt`, version-zero-only storage
  rows, citations, `surfaceOp` and replacement copies, the seed-gated
  association, time exactness); "Partial records and read failures remain
  distinguishable" (classification after admitted-version header admission
  under a per-version vocabulary, with the version-three vocabulary and
  quiet set named in full).
- `transcript-tui`: "Every readable kind reaches the pane and both doors"
  (the event/storage-refusal sentence names every admitted version instead
  of version zero; no scenario changes).

## Impact

- `openspec/specs/transcript-reading/spec.md` on fold, three requirements;
  `openspec/specs/transcript-tui/spec.md`, one sentence.
- `crates/brokkr-view/src/transcript.rs`: header admission yields the
  admitted version and the header's `isSeeded` value instead of a yes/no;
  the zero-token predicate becomes a digit-signature check that also stops
  accepting nonzero mantissas; the raw-token scanner decodes member names
  and reports a duplicated member, and a missing or duplicated token
  refuses or invalidates instead of passing; row dispatch takes the
  admitted version; under version three the four content kinds take the
  existing arms with one version condition, no position read from a
  `user/message`; `assistant/attempt` is an omission; a second quiet list
  keyed on version three is added and the version-zero list is untouched;
  the association pass is skipped for a version-three file whose header
  does not record `isSeeded: false`.
- `crates/brokkr-view/src/transcript/tests.rs`,
  `crates/brokkr-cli/src/ui/tests.rs`,
  `crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
  version matrix, disposition, projection, interrupted-message, attempt,
  citation, replacement-copy, seeded-association, time-exactness,
  sibling-discovery and end-to-end scenarios of the delta, all with inline
  synthetic rows in test-owned homes; every existing DSH test keeps its
  behavioural assertions, the six tests that call the changed helpers
  directly adapt their calls (tasks 4.3), and the version matrix is
  extended on purpose.
- `docs/decisions/NNNN-*.md` at the next free number with
  `Status: proposed` and its row in `docs/decisions/README.md` (S5).
- Guides only where a line says "version zero only" (none found).
- No change to `crates/brokkr-cli/src/ui.rs`, `crates/brokkr-protocol`,
  frozen `contracts/`, `policy/`, `reference/` or `fixtures/`. No delta to
  `transcript-command` (S6).
- The exact-coverage gate is unchanged; it runs outside the box and is
  recorded pending until its result exists.

## Decisions

The evidence record lives in the design and is not restated here: the
reach table and digests are D2, the six answers with their line citations
are D5, and the field-by-field payload mapping is D13. This section states
what the specification decides on that record and why. The eight rulings
of S5 are the one authoritative copy; the decision document transcribes
them and the design cites them.

### S1 — Evidence: one seat of this run read the installed writer; what it reached and what it did not

The commission admits one primary source, the `@deepseek-ai/dsh`
0.1.5-rc.1 writer as installed on this host, and forbids sampling as a
substitute. Reach is a property of an operation a seat performs at the
file it cites, not of the host (design D2's ruling). Four box shapes of
this run could not reach the writer's source and one could: the design's
simplicity seat resolved `/home/vyanakiev/.volta/bin/dsh`, confirmed
`0.1.5-rc.1`, and read the packages beneath it at 0.1.5-rc.2, every access
a read and nothing modified. Its ten digests, the files they cover and the
line citations are D2. This sitting's box, like the earlier specify
sittings, has no `dsh`, `volta`, `npm`, `cargo` or `~/.dsh` (HOME
`/runtime/home`), so nothing is re-measured here; every claim of the read
that touches the reader was checked against the tree by the design and
again at this sitting, and none contradicts the repository's record of
version zero.

**What the read established** (D2 E1 to E7, and D13):

1. The catalogue is exactly 56 names and reconciles with the reader's 51:
   three names (`assistant/chunk`, `tool/code-dispatch`,
   `tool/code-dispatch-start`) are absent from version three and eight are
   added; 51 − 3 + 8 = 56. No name is untranscribed (E1).
2. The physical header: `type`, `version`, `id`, `createdAt`, `isSeeded`
   and `delegationDepth` required; `cwd`, `parentSession`, `origin` and
   `agentPreset` permitted (E2, `worker.cjs`).
3. The envelope is a base of `type`, `seq`, `time` and `data` on every
   event row plus the two conditional top-level members of item 4; the
   timed stream is an array nested under `data.stream` on
   `assistant/message` and `assistant/attempt`; a top-level packed row
   lacks the base `seq` and `time` and carries members no type permits, so
   the version-three codec cannot write one (E3).
4. `surfaceOp` is a top-level member required on the four surface-eligible
   types and forbidden on every other type; `sourceEventSeqs` is a
   top-level member forbidden on `assistant/message` and validated
   wherever the codec finds one, sources unique and earlier than the
   event, replacement endpoints below its `seq` (E4). Whether any other
   type permits, requires or forbids `sourceEventSeqs` was not recorded:
   the read states one prohibition and no permission, and the earlier
   "optional on the other surface-eligible types" was an inference from
   that prohibition, now withdrawn as a measurement and recorded as unread
   (U4). The record's earlier sentence that the codec admits only the four
   base members overstated the base check as the whole envelope; E3 and E4
   cite one validation pass over the same lines and now read together
   (S8, CLARIFY-279-5, first and second visits). The unread cells are
   assigned, not inferred: read (d) at the council's next sitting (S6;
   third visit).
5. The message and block definitions: `Message` is `{id, role, content,
   source}` and the block union is `text`, `reasoning`, `image`,
   `tool-call`, `tool-result` and `file`; every field the reader consumes
   maps onto its existing parser (E5, D13).
6. The codec refuses the two `tool/code-dispatch*` names and treats
   `assistant/chunk` as an opaque unknown (E6).
7. The codec enforces agreement between `isSeeded` and the
   `session/end-seed` marker; the prefix length is `inheritedEventCount`
   session state (E7).

**What the read did not reach** (D2 U1 to U3), so the requirement withholds
exactly the rule each could break and names the read that lifts it:

- **The append and replace writer paths** (U1). E4 measures the codec's
  validation of a replace marker, not what a replace-marked row carries or
  how an append-origin row is told from it. The reader parses no marker and
  reconstructs nothing, so this gap changes no disposition: a replace-marked
  row projects by its type at its recorded position, as version zero's
  compaction message does. A later change that reads those paths may add
  presentation, never removal (S3 Q4).
- **The seed and fork path** (U2). Whether an inherited prefix keeps its
  originating `seq`, `turn`, `step` and call identities is uncited. The one
  rule that rests on those identities is the dedicated-tool association, so
  under version three it runs only for a session whose header records
  `isSeeded` exactly `false`; every other session keeps each embedded copy
  visible. The lifting read is the seed and fork writer path, with seeded
  scenarios in which inherited and new embedded and dedicated calls and
  results suppress a genuine duplicate once and keep unrelated or ambiguous
  identities (S3 Q3).
- **`surface.d.ts`** is cited by line but not by digest; `surface.js` from
  the same package version is (U3). No rule rests on the difference.
- **The per-type permission of `sourceEventSeqs`** (U4). Beyond its
  prohibition on `assistant/message` and its validation when present, the
  read cites no permission on any type. No rule rests on it: the reader
  validates the member wherever its version-zero rules read it, never on
  the writer's permission. The read that records it is the member's
  declaring type in the session package's `types.d.ts` and the per-type
  branch of the codec's validation pass. It is assigned to the design
  council's next sitting, the one phase of this run whose seats have
  reached the writer; the specification does not carry the cells as a
  requirement, and neither the growth rule nor the envelope requirement
  demands them: each states what the read established and names the
  read for the rest (S2, S6).

### S2 — The admitted set is `{0, 3}`, spelled exactly, classified per version

**Set.** Zero by the #222 capture: the 0.1.2-rc.1 session package declares
`SESSION_FORMAT_VERSION = 0` and its persistence package writes that header
and refuses a foreign version before decoding (archived #222 design, D6).
Three by D2: the 0.1.5-rc.2 session package declares
`SESSION_FORMAT_VERSION = 3` and the catalogue package declares
`currentVersion: 3` with codecs `v0..v3` and lossless migrations. Versions
1 and 2 are not admitted: no core of this fleet wrote them to disk under a
name discovery admits, and no seat read their codecs. Every other value
refuses exactly as today, and no version is admitted by a versioned
filename, a header number or the resemblance of sampled rows.

**Spellings.** One rule for both integers: a token spells an admitted
integer exactly when its parsed value is that integer and its digit
signature, the mantissa digits with the decimal point and every leading
and trailing zero removed, is that integer's digits, empty for zero and
`3` for three. `0`, `0.0`, `0e0`, `0.000` and `-0` admit as zero; `3`,
`3.0`, `3e0`, `30e-1` and `0.3e1` admit as three; `"3"`, `3.1`, `-3` and
`3e1` (signature `3`, refused on the parsed value), the rounding artefacts
`2.9999999999999999` and `3.0000000000000001` (parsed three, refused on
the signature) and every zero-collapsing token with a nonzero digit,
`1e-400`, `10e-400`, `0.1e-400` and `1.0e-400` (parsed zero, refused on
the signature), refuse. A parsed-value-only check was rejected because it
would admit a token the writer never wrote and drop a defence the zero
path has.

**Timestamps.** The signature decides one thing at the two time sites: a
parsed zero `time` or `time0` is a zero spelling only when its token's
signature is empty. A nonzero integer timestamp is judged on its parsed
value as a signed safe integer, exactly as the content rules already
require, so `1e3` and `1000.0` are the millisecond `1000`; no comparison
against the integer's digits is made there, because the signature of
`1e3` is `1`, not `1000` (S5 ruling 7; CLARIFY-279-9).

**Token binding.** The token is that of the one top-level member whose
name, after JSON string-escape decoding, is `version`. An escaped name
binds like a literal one; a header recording `version` twice, under any
names and with any values, is ambiguous and refuses; a parsed numeric
`version` whose token cannot be established refuses. Binding to the first
or the last duplicate was rejected because the parser's retained
occurrence and the scanner's first occurrence need not agree, so either
choice can admit a rounded foreign token under the exact-number promise;
no writer records a member twice, so refusing costs nothing real
(CLARIFY-279-8).

**Compatibility, stated plainly.** The shipped zero predicate accepted any
mantissa containing a zero digit, so `10e-400` and `0.1e-400` were
admitted as version zero, rendered `"0"` as a time and passed as a packed
`time0`; and the shipped raw-token check treated a missing token as
passing. Preserving the exact-zero requirement therefore entails changing
that behaviour, and this change does: those tokens refuse, render an empty
stamp and refuse the packed row, and a missing or duplicated token no
longer passes. The version-zero requirement text is unchanged and no
writer of this fleet emits either shape; the earlier claims that
version-zero behaviour is preserved byte for byte are withdrawn (S5
ruling 7; CLARIFY-279-7). **The exactness scope is unchanged:** the reader
judges three fields on their recorded token, the header `version`, an
ordinary event's `time` and a packed row's `time0`; admitting three widens
the version check from one admitted integer to two and adds no field.

**Per-version vocabulary.** A row is classified under the vocabulary of
the version that admitted its file. The four content kinds keep the one
projector under both versions because their envelopes (event map,
`types.d.ts:281`, `:309-317`, `:333`, `:351`) and their message and block
definitions (D13) carry the fields the projector reads. A union quiet list
was rejected (design D1): it would admit `assistant/chunk` and packed rows
under a writer whose catalogue has no chunk type and whose codec cannot
write them, and it would quiet-list version-three names under a writer that
never emitted them. A per-version projector was rejected because nothing
the reader projects differs between the versions once the payloads are
measured; the cost of the per-version vocabulary is one argument through
dispatch and one second list.

**Growth rule.** A version joins only through a change that reads the
writer's own source and records, cited by package, file, digest and line,
what that read established of each of the following and, for each it did
not reach, that fact and the specific read that would: the package
identity and files; the record vocabulary; the message and block
definitions behind every projected payload, mapped field by field; how
fragments persist; the meaning of every added header field; the physical
envelope and the meaning of `surfaceOp` and `sourceEventSeqs` where the
read found them; the identity of a seeded session's inherited rows; a
disposition for every type; and whether admitted meanings are preserved.
A part not reached is left refused or withheld with the lifting read
named, never admitted with its version: a rule that rests on it is
withheld, and a part on which no rule rests is recorded unread. The
version is admitted on the parts read, and that admission is complete,
not incomplete: the rule demands no inventory the change's reads did not
deliver and states nothing they did not record. The seventh sitting's
rule demanded the meaning of the two members "on every row that carries
them" as a condition of joining while S6 placed the `sourceEventSeqs`
cells out of scope; the operator ruled at the eighth sitting that the
original commission's allowance governs, and the rule now says so (S6,
S8; CLARIFY-279-5, fourth visit).

### S3 — The six questions of #279, as the specification answers them

The measured answers and their citations are design D5; this table records
the specification consequence of each.

| # | Question | Specification consequence |
|---|---|---|
| 1 | Record types and shapes | A per-version vocabulary closed at the 56 transcribed names. The four content kinds project on the version-three envelope and the 0.1.5-rc.2 message and block definitions (D13): `content` as an array of blocks or a string; `text.text`; `reasoning.text`; `tool-call.id/name/arguments`; `tool-result.toolCallId/content`; `image` as the omission marker; `file` as a counted unrecognized block that keeps its siblings; `Message.id` and `.source` inert. Under version three no `turn` or `step` is read from a `user/message`, whose definition declares neither, even when a row supplies them. `tool/call` projects from `callId`, `name` and `arguments` on the event. |
| 2 | `assistant/chunk` absent | Removed, not renamed and not conditional. Whole messages only under version three; an interrupted step is one `assistant/message` with `interrupted: true`, projected as recorded with no marker; `interrupted`, `stream` and `usage` are never expanded; `assistant/attempt` is a counted omission; `assistant/chunk` and the packed rows are required unknowns under version three. The fragment rule's text is unchanged and has nothing to concatenate. |
| 3 | `isSeeded` new in the header | Inert for ownership, depth, admission, classification and counts, whatever its JSON shape. Because the seed path is unread, it has one effect under version three: the dedicated-tool association runs only when the header records `isSeeded` exactly `false`. A seeded or unattested version-three session shows every embedded copy beside its dedicated event, which is the existing absent-partner outcome, never a hidden row. Under version zero the field has no effect. |
| 4 | `surfaceOp` and `sourceEventSeqs` at about 21% of rows | Both are top-level members beside the base envelope of `type`, `seq`, `time` and `data`: `surfaceOp` required on the four surface types and forbidden elsewhere, `sourceEventSeqs` forbidden on `assistant/message` and unrecorded elsewhere pending the council's read (d) (S1 items 3 and 4, U4, S6); the reader validates no row's key set and neither requires nor refuses either. The writer places every surface message on the surface with an explicit marker. `sourceEventSeqs` is validated identically under both versions; under version three a valid citation suppresses nothing, because no chunk row exists and the writer never puts a citation on `assistant/message`, and an invalid one refuses as today. `surfaceOp` is never parsed: a replace-marked row projects by its type at its recorded position, no earlier row is removed, reordered or replaced, and the surface is never replayed. |
| 5 | `system/message`, `todo/write`, `turn/end` | All three quiet under version three, whatever payload or marker they carry; `system/message` is the rendered prompt that `request/context` was, and a displayed `system` turn would be new command and TUI capability. `system/message` stays a required unknown under version zero; `todo/write` and `turn/end` were already quiet there. |
| 6 | Superset? | No as a vocabulary, yes as a payload. Version three removes three names, adds eight, moves the prompt into `system/message` and the stream into the message; the four content kinds keep their envelope nesting and their block definitions map field for field. So `3` is admitted beside `0` with a per-version vocabulary and one projector; every version-zero meaning is preserved for version-zero files and every projected version-three meaning is measured. |

### S4 — What this change delivers, what stays withheld, and corroboration

**Delivered now, on the read:** the header admission of three with its
exact spellings, the digit-signature and token-binding rules and their two
version-zero corrections; the per-version vocabulary with the 51-name quiet set;
the projection of the four content kinds; the interrupted message as one
turn; `assistant/attempt` counted; `isSeeded` inert except for the
association gate; `surfaceOp` never rewriting audit order; citations
validated and suppressing nothing; the sibling and filename/version
pinnings; the `transcript-tui` sentence; and the decision of S5. A real
version-three seat transcript of this fleet reads end to end; the fleet's
driver never seeds a session, so the association gate is open for every
transcript it writes.

**Withheld, and what lifts it.** One rule is withheld: the dedicated-tool
association under version three for a session whose header does not record
`isSeeded: false`. It is lifted by a further change that reads the seed and
fork writer path, cited by package, file, digest and line, and pins seeded
scenarios naming exact counts, order and suppression. The append and
replace writer paths remain unread; no rule rests on them, and a later
change that reads them may add a marker-aware presentation but never a
removal. The per-type permission of `sourceEventSeqs` beyond
`assistant/message` is likewise unread (U4); no rule rests on it, the
requirement demands no inventory of it, and its read is the design
council's at its next sitting, not a later change's (S2, S6).

**Corroboration** follows the writer and never leads. After the writer
read, the simplicity seat ran a type-only, content-free histogram over the
version-three sessions under `~/.dsh/sessions/brokkr` (20 at its sitting;
no message text, working directory or credential read; nothing copied or
committed): no top-level packed row and no `assistant/chunk` appears,
every header key set is E2's required set plus `cwd`, and the payload key
sets match E5 (design D2). That read changed no disposition and proved no
admission; the commission permits it after the writer, and decision 0032
ruling 3, which keeps transcript prose out of the journal and forbids a
driver from removing a transcript, constrains neither it nor this change.
After landing, the controller reads those sessions through the built reader
on the host and confirms each projects with zero refusals and counts equal
to its `assistant/attempt` and `file`-block rows. Every test in this change
uses synthetic rows in test-owned homes.

### S5 — A proposed decision carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes ("DSH numeric
on-disk version zero only"), so this change files a numbered decision with
`Status: proposed`, supplementing ruling 3 without editing it, at the next
free number re-read from the index at the PR (0056 and 0060 are claimed by
other branches). The decision is written beside its code, with its README
row, cites D2's digest table, and rules:

1. **Admitted versions are measured, closed and named.** Zero on the
   0.1.2-rc.1 capture, three on the 0.1.5-rc.1 read with session packages
   0.1.5-rc.2 (the D2 digests). A version joins only through a change
   that reads the writer's own source and records what that read
   established of the writer's package identity and files with digests,
   the record vocabulary, the message and block definitions behind every
   projected payload mapped field by field, how fragments persist, the
   meaning of every added header field, the physical envelope and the
   meaning of `surfaceOp` and `sourceEventSeqs` where found, the identity
   of a seeded session's inherited rows, a disposition for every type,
   and whether admitted meanings are preserved; a part the read did not
   reach is recorded as such with the read that would reach it named,
   and the version is admitted on the parts read, which is a complete
   admission. Reach to the writer's source is an operation the
   reading seat performs at the file it cites, never inherited from another
   seat or sitting; a resolved binary is not source access. A versioned
   filename, a header number and the resemblance of sampled rows are not
   evidence.
2. **Vocabulary is per version.** A row is classified under the vocabulary
   of the version that admitted its file; a shared name keeps a disposition
   in each version only by measurement; a name outside the admitted
   version's vocabulary is a required unknown. Version-zero storage rows
   and `assistant/chunk` are version-zero only.
3. **A payload projects only on its own writer's definitions.** Under
   version three the four content kinds project on the 0.1.5-rc.2 message
   and block definitions, mapped field by field and pinned by scenarios; a
   `file` block is a counted unrecognized block. A familiar block name is
   not evidence; the mapping is. Under version three the reader reads no
   `turn` or `step` from a `user/message`, whose definition declares
   neither; a supplied pair is not the writer's identity and never enters
   the association.
4. **Version three persists whole messages.** No fragment rows; an
   interrupted step is one finalized `assistant/message` with
   `interrupted: true`, projected as recorded; embedded streams and usage
   are never expanded; `assistant/attempt` is a counted omission. The
   reader assembles nothing under either version.
5. **The surface is not the transcript.** `surfaceOp` is never parsed,
   never removes, reorders or replaces a row and is never replayed; a
   replace-marked row projects by its type at its recorded position;
   suppression remains cited-unique-earlier-same-step chunks only, and a
   version-three citation suppresses nothing.
6. **`isSeeded` is inert** for ownership, depth, admission, classification
   and counts, whatever its JSON shape. Under version three the
   dedicated-tool association runs only when the header records `isSeeded`
   exactly `false`, until a change reads the seed and fork path; a withheld
   association hides nothing.
7. **Recorded-token exactness has a fixed scope and one predicate per
   site.** The header `version`, an ordinary event's `time` and a packed
   row's `time0` are judged on their recorded token under either version;
   no other field is. At the header a token spells an admitted integer
   exactly when its parsed value is zero or three and its digit signature,
   the mantissa digits with the decimal point and every leading and
   trailing zero removed, is empty or `3` respectively. At the two time
   sites the parsed value must be a signed safe integer as the content
   rules require; a parsed zero is a zero spelling only when its token's
   digit signature is empty, and a nonzero integer is judged on its parsed
   value alone, so `1e3` and `1000.0` are the millisecond `1000`. At all
   three sites the token is bound to the one top-level member whose
   decoded name is the field's, and a member recorded more than once, or a
   parsed number whose token cannot be established, refuses or invalidates
   rather than passing on the parsed value. This corrects the shipped
   reader's acceptance of zero-digit underflow tokens and its fall-open on
   a missing token, under version zero as well; the version-zero
   requirement is unchanged and no writer emits the affected shapes.
8. **Unread parts withhold, never admit.** Any name outside the 56-name
   catalogue and the version-zero storage rows are refused under version
   three; the seeded association is withheld; the per-type permission of
   `sourceEventSeqs` beyond `assistant/message` is recorded unread, with
   no rule resting on it and no inventory demanded; the record says which
   read lifts each. A part left refused, withheld or unread with its read
   named is a complete outcome, not an incomplete one.

Enforcement bindings: the version-matrix, disposition, projection,
interrupted-message, attempt, citation, replacement-copy,
seeded-association, time-exactness, sibling-discovery and end-to-end tests
named in the delta's scenarios, `openspec validate --all --strict`, and the
exact-coverage gate. 0032's ownership, retention and privacy constraints
are preserved.

### S6 — Reasoned refusals

- **No fall-through in discovery.** When the versioned name carries a
  refused version and the plain name beside it is readable, the read
  refuses with the versioned path. Falling through would present
  superseded evidence as the seat's newest transcript, the same reason the
  version-zero rule refuses to rank roots by version (design D9).
- **No refusal of version-three payloads on an unread-definitions ground.**
  The definitions were read and mapped (D2 E5, D13); refusing on the
  contrary claim would fold a refuted fact into living truth, and the
  header, vocabulary and `tool/call` admission rest on the same seat's read
  from the same run.
- **No association across an unattested seeded prefix.** The rule keys on
  identities the seed path may remap; withholding it shows more, never
  less. Applying it everywhere with a stated residual was rejected because
  the residual is exactly the unread part, and refusing seeded sessions was
  rejected because it would turn a header field the commission ruled inert
  into an admission gate.
- **No `surfaceOp` model, no surface replay and no refusal of a
  replace-marked row.** Nothing gates on the marker; the writer calls the
  surface the wrong source for a human transcript; a row the writer
  persisted displays where it was persisted. Refusing or omitting on the
  marker would parse it and decide on unread content.
- **No displayed `system` turn.** `system/message` is the prompt copy that
  `request/context` was; showing it adds a role no requirement asks for.
- **No stream expansion and no attempt projection.** Expanding the stream
  would duplicate the message the writer finalized or fabricate fragments
  it chose not to persist as rows; an attempt's text is counted, not
  silent, and a later change may project it now that its element shape is
  cited (E3).
- **No union vocabulary and no second projector.** The union admits rows a
  writer cannot emit; a second projector has nothing to differ on.
- **No migration and no versions 1 or 2.** Version-zero files read as
  before; no on-disk file of this fleet carries 1 or 2 and no seat read
  their codecs.
- **No `transcript-command` delta.** Its observable behaviour for a
  readable or refused DSH source is unchanged. The `transcript-tui` sentence
  is amended by a one-paragraph delta because the fold applies only deltas;
  deferring wording to the fold named a mechanism that does not exist.
- **No preservation of the shipped zero predicate and no first-or-last
  duplicate binding.** Keeping `10e-400` a zero would keep a defect the
  requirement already forbids; binding a duplicated member to either
  occurrence can disagree with the parser and admit a rounded token.
  Refusing what no writer records costs nothing real (S2).
- **No reading of undeclared user positions.** A `turn` or `step` on a
  version-three `user/message` is a familiar name, not the writer's
  identity (S5 ruling 3); reading it would let a crafted row own or lose
  an embedded copy the requirement promises it keeps.
- **No `sourceEventSeqs` permission inferred from a prohibition.** The read
  forbids the member on one type; treating that as permission on the
  others would record an inference as a measurement. The permission is
  recorded unread at no cost, because no reader rule consults it (S1, U4).
- **No per-type `sourceEventSeqs` inventory as a requirement of this
  change, no inferred cell, and no requirement that demands one.** The
  reconciliation commission of the fifth sitting asked for the allowed,
  required, optional and conditional top-level key sets by relevant event
  type. The clarify judge's fourth visit found that the seventh sitting
  had refused that inventory here while the delta's growth rule still
  demanded a record of the two members on every row that carries them as
  a condition of a version joining the set, so the artifacts demanded in
  one place what they placed out of scope in another. The operator ruled
  at the eighth sitting that the original commission's allowance
  governs: a part the performed reads did not reach is left refused or
  unread with the specific further read named, and that is a correct and
  complete outcome. So reconciled, in that direction. For the four kinds
  the reader projects, the record states every cell with its standing
  (design D2 E4): the base four and `surfaceOp` measured on all four;
  `sourceEventSeqs` forbidden on `assistant/message` by measurement and
  unread on `user/message`, `tool/result` and `tool/call`, as on the
  other 52 types. The growth rule and the envelope paragraph state what
  the read established, record the rest as unread pending read (d), and
  demand no inventory (S2). No rule rests on the unread cells: the reader
  validates no row's key set and reads the member on its three kinds
  wherever present, and two scenarios now falsify that across every
  disposition class, the writer-validity scenario over the content kinds
  and the unread-cell scenario over a quiet, a counted-omission and a
  required-unknown row, each carrying a member that would refuse the
  read if the reader consulted it there, beside a fourth file that puts
  the same member on the readable user message and refuses (S8,
  CLARIFY-279-12). The cells exist only in the
  installed writer's `types.d.ts` and `worker.cjs`, which no
  specification box of this run reaches (S1; this sitting reconfirmed no
  `dsh`, `volta`, npm cache, `~/.dsh` or network) and which the operator
  rules this seat is not to re-read; an inferred cell would repeat the
  defect the second visit withdrew. Read (d) stays assigned to the design
  council's next sitting, the one phase of this run whose seats have
  reached the writer, to be performed by the seat that reaches it, cited
  by package, file, digest and line, with E4 completed there; a council
  without reach records that and leaves the cells unread, and the change
  is complete either way (design D13; S8, CLARIFY-279-5, third and fourth
  visits).
- **No compatibility wrappers to keep old test lines compiling.** Keeping
  `zero_number_token` and the one-argument helpers as shims so that no
  existing test line changes would leave dead code under the exact-coverage
  gate; the direct calls adapt mechanically and keep their assertions
  (tasks 4.3).
- **No `upstream` result from this seat.** The commission supports a sound
  specification: every rule the delta states rests on evidence in this
  run's record, and the one evidence cell the record lacks is no rule's
  premise, is refused above as a requirement, has an owner within this
  run, and is left unread under an operator ruling that calls that
  complete.

### S7 — Validation of this sitting

Read the dialect through `openspec instructions proposal`, `specs` and
`tasks` for this change; they declare `proposal.md`, `specs/**/*.md` and
`tasks.md`, and no workflow runner was invoked. Adopted the existing change
at `edee622` and revised it for the one finding of the ninth sitting,
CLARIFY-279-12, in dependency order: the `transcript-reading` delta's
unread-cell scenario first, as the owning artifact, by an exact-match
replacement of its two lines that gives the readable user message no
`sourceEventSeqs` member and adds a fourth file as the negative control;
then design D12's unread-cell bullet, task 2.10, and this proposal's
narrative, S6 and S8, which summarise it, with the design's Context and
validation record. No requirement sentence, no other scenario, the
growth rule, the rulings, the refusal and read (d) are unchanged; the
eighth sitting's reconciliation stands as recorded in its commit
(`edee622`). The `transcript-tui` delta is unchanged. No task reads the
writer, and no scenario's outcome depends on the unread cells.
Reconfirmed this box cannot reach the writer by any route: no `dsh`, `volta`, `npm`, `cargo`
or `~/.dsh`, HOME `/runtime/home`, `/home/vyanakiev` holding only
`source`, no npm cache, and no network (the registry host does not
resolve); nothing was re-measured and no E4 cell changed standing.
Verified in the tree that `project_dsh` reads only `seq` and `ignorable`
from a quiet, counted-omission or unrecognized row
(`transcript.rs:2011-2032`), that `dsh_citations` is called from the
message arm and the `tool/result` arm only (`:2412`, `:2474`), and that
a present non-array member, `null` included, returns the refusal from
that helper (`:2349-2351`), which the message arm turns into a refused
row (`:2412-2414`) and `project_dsh` into a whole-read
`unsupported-format` that keeps the counts (`:2033-2045`): the
counterexample the finding derived, which the scenario's fourth file now
pins; the earlier sittings' verifications stand as recorded in the
change's history (`edee622`, `1f68fee`, `2e7163e`, `95ba4a8`).
Strict OpenSpec validation was run over the whole tree after authoring.
Cargo is absent from this box, so no format, clippy, test or bundle gate
ran here; they belong to the implementation seat before its activation
commit, and the exact-coverage gate to host validation outside the box.
This sitting commits the proposal, the reading delta, the design and the
tasks, unsigned, and pushes nothing.

### S8 — The judges' findings, answered

The clarify judge's four findings on the second sitting, the analyze
judge's seven on the third, the clarify judge's four on the fourth, its
four on the fifth, its one on the sixth, its one on the seventh and its
one on the eighth are answered here; the earlier answers that later
sittings superseded are recorded in the change's history (`36bf374`,
`f34279e`, `50d2c28`, `95ba4a8`, `2e7163e`, `1f68fee`, `edee622`).

| Finding | Answer | Where |
|---|---|---|
| ANALYZE-279-1 (high): the payload refusal's ground is refuted by D2 E5 and D13. | Lifted. The four content kinds project on the read definitions; the delta's evidence paragraphs, S1, S3 and S5 ruling 3 now say the definitions were read and mapped. The two genuinely unread paths withhold exactly one rule each could break (S1, S4). | Delta: Discovery evidence paragraph; content requirement, payload, citation, replacement and association paragraphs; projection, interrupted, citation, replacement and seeded scenarios. |
| ANALYZE-279-2 (medium): the catalogue is 56, not 58. | Corrected everywhere; the closed-vocabulary rule stays and bites only on a name a future writer adds. | Delta: Partial-records vocabulary paragraph and its last scenario. Proposal: S1 item 1. |
| ANALYZE-279-3 (low): the unmeasured-items inventory is stale. | Re-derived from D2's E and U lists; the proposal no longer keeps its own inventory of evidence. | S1. |
| ANALYZE-279-4 (low): the corroboration read is misstated and 0032 miscited. | Recorded as it happened and cited for what 0032 ruling 3 says. | S4. |
| ANALYZE-279-5 (low): tasks 2.7 drops `assistant/attempt`; the sampled-types quiet clause is unassigned. | Both assigned. | tasks.md 2.5 and 2.6. |
| ANALYZE-279-6 (low): the `transcript-tui` wording fix was deferred to a fold that cannot perform it. | A one-paragraph MODIFIED delta widens the sentence. | `specs/transcript-tui/spec.md`. |
| ANALYZE-279-7 (low): three tables duplicated across proposal and design had diverged. | One copy each: digests and six-answer evidence in the design (D2, D5, D13), rulings in the proposal (S5); the decision transcribes S5 and the design cites it. | This section's preamble; design D11. |
| CLARIFY-279-1: which nested shapes are measured? | All the reader consumes, by D13's mapping; `file` is the one block the reader does not name and is counted. | Delta projection scenarios; S3 Q1. |
| CLARIFY-279-2: what does the writer persist for replacement copies? | Unread, and no rule rests on it: the marker is never parsed and a persisted row displays where it was persisted. | S1, S3 Q4; delta replacement paragraph and scenario. |
| CLARIFY-279-3: what proves inherited identities are association evidence? | Nothing yet, so association runs only for `isSeeded: false`; the seed read lifts the gate. | S1, S3 Q3, S4; delta association paragraph and seeded scenario. |
| CLARIFY-279-4: does the exactness clause remove the `time`/`time0` checks? | No; the three-field scope is stated and pinned. | S2; delta time paragraph and scenario. |
| CLARIFY-279-5: the envelope record both excludes and requires top-level members beyond the four. | The "only" sentence overstated the base check. The reconciled envelope is the base four plus `surfaceOp` and `sourceEventSeqs` as conditional top-level members with per-type permissions; the packed-row conclusion stands on the missing base members; writer validity and reader admission are stated apart. | S1 items 3 and 4, S3 Q4; design D1, D2 E3/E4, D5 Q1; delta envelope paragraph and the writer-validity scenario. |
| CLARIFY-279-6: the never-associates promise for a version-three user message is not enforced by the unchanged arm. | The promise stands and is enforced: under version three the reader reads no position from a `user/message`, so the message arm gains one version condition; the inference that an unchanged arm yields none is withdrawn. | S3 Q1, S5 ruling 3, S6; design D1, D4, D13; delta payload paragraph, seeded scenario and the supplied-positions scenario; tasks 2.1 and 2.8. |
| CLARIFY-279-7: byte-for-byte zero preservation contradicts the shipped predicate. | Stated plainly: the predicate is corrected, version-zero behaviour changes for zero-digit underflow tokens, and the helper's signature contract is separated from the combined admission check. | S2, S5 ruling 7, S6; design D3, D12; delta version and time paragraphs and their two new scenarios; tasks 1.2 to 1.5. |
| CLARIFY-279-8: escaped or duplicate `version` members escape the exact-number promise. | The token is bound to the one decoded member; duplicates refuse as ambiguous; a missing token for a parsed number refuses; the same binding serves `time` and `time0`. | S2, S5 ruling 7, S6; design D3, D12; delta version and time paragraphs, the decoded-member scenario and the duplicated-time scenario; tasks 1.3 to 1.5. |
| CLARIFY-279-5, second visit: E4 presented an inferred `sourceEventSeqs` permission as measured and left 52 types unrecorded. | Answered as far as the adopted evidence reaches, and no further: the read records `surfaceOp` for every type, the `sourceEventSeqs` prohibition on `assistant/message` and the validation of a present member, and no permission on any other type. The "optional" cell is withdrawn as a measurement; the permission on the other 55 types is recorded unread as U4 with the read that fixes it named, and no reader rule rests on it. That left the full-envelope question open on this record, bounded to that one unread permission; the third visit, below, decides who closes it and when. | S1 item 4 and U4, S3 Q4, S4, S6; design D1, D2 E3/E4 and U4, D5 Q1/Q4, D13 read (d); delta evidence and envelope paragraphs; the simplicity report's second erratum. |
| CLARIFY-279-5, third visit: the `sourceEventSeqs` permission inventory for the 55 types other than `assistant/message` is incomplete, and the evidence that completes it is the member's declaring type in `types.d.ts` around `:429-446` with the per-type validation branch at `worker.cjs:9824-9930`. | Refused as a specification obligation and assigned as a read, not relabelled. The requirement is complete without the cells: the reader validates no key set and reads the member on its three kinds wherever present, which the writer-validity scenario now pins across a forbidden cell, an unread cell and two members the reader never reads. The cells exist only in the installed writer, which no specification box of this run reaches and which the commission forbids this seat to re-read; an inferred cell would repeat the defect the second visit withdrew. Read (d) is therefore assigned to the design council's next sitting, the one phase of this run whose seats have reached the writer, to be performed by the seat that reaches it and cited by package, file, digest and line, with E4 completed there; a council without reach records that and leaves the cells unread, and the change proceeds either way because no rule rests on them. | S1 item 4 and U4, S3 Q4, S4, S6; design Context, D1, D2 reach table, E4 and U4, D5 Q1/Q4, D12, D13 read (d); delta evidence and envelope paragraphs and the writer-validity scenario; tasks 2.9; the simplicity report's third erratum. |
| CLARIFY-279-5, fourth visit: S6 removes the commissioned envelope inventory from scope and declares closure while the requirement still demands the record, and no operator amendment replaced the commission's criterion. | Answered on the operator's ruling of the eighth sitting, which amends the criterion: the original commission's allowance governs, so a part the performed reads did not reach is left refused or unread with the specific further read named, and that is a correct and complete outcome. The contradiction was real and is repaired in the owning rule first: the growth rule no longer demands the meaning of the two members on every row that carries them as a condition of joining, and states instead what a change records from its reads and how it leaves the rest; the envelope paragraph states no permission the read did not record and demands no inventory; S5 rulings 1 and 8 and S6 say the same, and every dependent summary follows. A new scenario falsifies the closure claim across the disposition classes the writer-validity scenario does not cover: a quiet, a counted-omission and a required-unknown row each carry a member that would refuse the read if the reader consulted it there, and none is read. Read (d) stays assigned as before, nothing was re-measured and no cell changed standing. | Why, What Changes, S1 U4, S2 growth rule, S4, S5 rulings 1 and 8, S6, S7; design Context, D1, D2 reach table and U4, D12, validation record; delta growth-rule, evidence and envelope sentences and the unread-cell scenario; tasks 2.10 and 6.1; the simplicity report's fourth erratum. |
| CLARIFY-279-9: S5 ruling 7 generalised the signature to every integer timestamp, which would invalidate `1e3`. | Ruling 7 is restated per site: header admission for `{0, 3}` on parsed value and signature together; the time sites on the parsed value as a signed safe integer, the signature deciding only whether a parsed zero is a zero spelling; the binding shared by all three. `1e3` and `1000.0` render `1000`, and the version-three time scenario now spells `1000.0` beside `1e3`. | S2 Timestamps, S5 ruling 7; design D3, D12; delta version and time paragraphs and the version-three time scenario; tasks 1.5 and 6.1. |
| CLARIFY-279-10: the helper algorithm gave `.` the empty signature while requiring `None`. | The helper's grammar requires at least one mantissa digit and, after an `e`, at least one exponent digit; it validates no more of the JSON grammar because the parser has already read the token as a number; `.` has no signature because it has no digit. | Design D3, D12; tasks 1.2. |
| CLARIFY-279-11: task 4.3 required every existing test unmodified while tasks 1 and 2 change the helpers those tests call. | Preservation means behavioural assertions, not bytes: the six tests that call the changed helpers directly adapt their calls mechanically and are named, the version matrix is the one test extended on purpose, no compatibility wrapper is added, and no existing assertion changes because none spells an affected token. | Impact, S6; design D12; tasks 1.2 to 1.5, 2.1, 2.3 and 4.3. |
| CLARIFY-279-12: the unread-cell scenario's modifier "each carrying `sourceEventSeqs: null`" reads onto its readable user message, which the settled citation rule refuses, so its three stated outcomes cannot all follow from its stated input; design D12 and task 2.10 repeat the ambiguity. | The user message was never meant to carry the member, and the scenario now says so: the readable `user/message` at sequence 1 carries no `sourceEventSeqs` member, `null` sits only on the two quiet rows and on the appended packed and ignorable rows, and the attempt keeps its reversed range. A fourth file, the first with `sourceEventSeqs: null` added to the user message alone, refuses with `unsupported-format`, two unrecognized records and no turns, as the settled rule requires; it is the negative control that makes the other three outcomes falsifiable, since the member that refuses there is the member the unread rows carry. The citation rule, the quiet, counted-omission and required-unknown dispositions, the inventory ruling and read (d) are untouched. | Delta unread-cell scenario; design Context and D12; tasks 2.10; this proposal's Why, S6 and S7. |
