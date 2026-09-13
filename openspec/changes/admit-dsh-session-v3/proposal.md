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
that lifts each named.

## What Changes

- The admitted DSH version set becomes closed and exactly `{0, 3}`. Zero
  stays admitted on the #222 capture of the 0.1.2-rc.1 writer; three is
  admitted on the council's read of the installed 0.1.5-rc.1 writer
  (session packages 0.1.5-rc.2), cited by package, file, digest and line in
  the design (D2). Three is admitted only in exact numeric spellings under
  the same recorded-token cross-check zero has; versions 1, 2 and every
  other value keep the refusal. The growth rule stays and names what a
  further version must record (S2).
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
  is a counted unrecognized block that keeps its siblings. A version-three
  `user/message` records no `turn` or `step` (S3, Q1, Q6).
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
- `surfaceOp` is never parsed and never removes, reorders or replaces a
  row. A replace-marked row projects by its type at its recorded position,
  exactly as version zero's compaction message does; the append and replace
  writer paths were not read, and the rule reads no marker, so that gap
  changes no disposition (S3, Q4; S4).
- The recorded-token exactness check judges the header `version`, an
  ordinary event's `time` and a packed row's `time0`, under either version,
  and no other field (S2).
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
  versions and the two unread paths, the exactness scope, `isSeeded` over
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
  the exact-integer token check generalizes from zero to zero-or-three; row
  dispatch takes the admitted version; under version three the four content
  kinds take the existing arms, `assistant/attempt` is an omission, a
  second quiet list keyed on version three is added and the version-zero
  list is untouched; the association pass is skipped for a version-three
  file whose header does not record `isSeeded: false`.
- `crates/brokkr-view/src/transcript/tests.rs`,
  `crates/brokkr-cli/src/ui/tests.rs`,
  `crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
  version matrix, disposition, projection, interrupted-message, attempt,
  citation, replacement-copy, seeded-association, time-exactness,
  sibling-discovery and end-to-end scenarios of the delta, all with inline
  synthetic rows in test-owned homes; every existing DSH test unchanged.
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
3. The envelope is `type`, `seq`, `time`, `data`; the timed stream is an
   array nested under `data.stream` on `assistant/message` and
   `assistant/attempt`, and a top-level packed row cannot be written by the
   version-three codec (E3).
4. `surfaceOp` is required on the four surface-eligible types and
   forbidden elsewhere; `assistant/message` forbids `sourceEventSeqs`;
   sources must be unique and earlier than the event, replacement
   endpoints below its `seq` (E4).
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

**Spellings.** Zero keeps its rule. Three is admitted when the parsed value
is three and the recorded token, with its decimal point and leading and
trailing zeros removed, spells exactly `3`: `3`, `3.0`, `3e0`, `30e-1` and
`0.3e1` admit; `"3"`, `3.1`, `-3`, `3e1` and the rounding artefacts
`2.9999999999999999` and `3.0000000000000001` refuse. The recorded-token
cross-check the zero path already performs is kept for three. **Its scope
is unchanged:** the reader judges three fields on their recorded token, the
header `version`, an ordinary event's `time` and a packed row's `time0`;
admitting three widens the version check from one admitted integer to two
and adds no field. A parsed-value-only check for three was rejected because
it would admit a token the writer never wrote and drop a defence the zero
path has.

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

**Growth rule.** A version joins only through a change that records, from
the writer's own source, its package identity and files with digests; the
record vocabulary; the message and block definitions behind every
projected payload, mapped field by field; how fragments persist; the
meaning of every added header field; the meaning of `surfaceOp` and
`sourceEventSeqs` where carried; the identity of a seeded session's
inherited rows; a disposition for every type; and whether admitted
meanings are preserved. A part that was not read keeps the rule it could
break withheld, never admitted with its version.

### S3 — The six questions of #279, as the specification answers them

The measured answers and their citations are design D5; this table records
the specification consequence of each.

| # | Question | Specification consequence |
|---|---|---|
| 1 | Record types and shapes | A per-version vocabulary closed at the 56 transcribed names. The four content kinds project on the version-three envelope and the 0.1.5-rc.2 message and block definitions (D13): `content` as an array of blocks or a string; `text.text`; `reasoning.text`; `tool-call.id/name/arguments`; `tool-result.toolCallId/content`; `image` as the omission marker; `file` as a counted unrecognized block that keeps its siblings; `Message.id` and `.source` inert. `tool/call` projects from `callId`, `name` and `arguments` on the event. |
| 2 | `assistant/chunk` absent | Removed, not renamed and not conditional. Whole messages only under version three; an interrupted step is one `assistant/message` with `interrupted: true`, projected as recorded with no marker; `interrupted`, `stream` and `usage` are never expanded; `assistant/attempt` is a counted omission; `assistant/chunk` and the packed rows are required unknowns under version three. The fragment rule's text is unchanged and has nothing to concatenate. |
| 3 | `isSeeded` new in the header | Inert for ownership, depth, admission, classification and counts, whatever its JSON shape. Because the seed path is unread, it has one effect under version three: the dedicated-tool association runs only when the header records `isSeeded` exactly `false`. A seeded or unattested version-three session shows every embedded copy beside its dedicated event, which is the existing absent-partner outcome, never a hidden row. Under version zero the field has no effect. |
| 4 | `surfaceOp` and `sourceEventSeqs` at about 21% of rows | The writer places every surface message on the surface with an explicit marker. `sourceEventSeqs` is validated identically under both versions; under version three a valid citation suppresses nothing, because no chunk row exists and the writer never puts a citation on `assistant/message`, and an invalid one refuses as today. `surfaceOp` is never parsed: a replace-marked row projects by its type at its recorded position, no earlier row is removed, reordered or replaced, and the surface is never replayed. |
| 5 | `system/message`, `todo/write`, `turn/end` | All three quiet under version three, whatever payload or marker they carry; `system/message` is the rendered prompt that `request/context` was, and a displayed `system` turn would be new command and TUI capability. `system/message` stays a required unknown under version zero; `todo/write` and `turn/end` were already quiet there. |
| 6 | Superset? | No as a vocabulary, yes as a payload. Version three removes three names, adds eight, moves the prompt into `system/message` and the stream into the message; the four content kinds keep their envelope nesting and their block definitions map field for field. So `3` is admitted beside `0` with a per-version vocabulary and one projector; every version-zero meaning is preserved for version-zero files and every projected version-three meaning is measured. |

### S4 — What this change delivers, what stays withheld, and corroboration

**Delivered now, on the read:** the header admission of three with its
exact spellings; the per-version vocabulary with the 51-name quiet set;
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
removal.

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
   recording the writer's package identity and files with digests, the
   record vocabulary, the message and block definitions behind every
   projected payload mapped field by field, how fragments persist, the
   meaning of every added header field, the meaning of `surfaceOp` and
   `sourceEventSeqs` where carried, the identity of a seeded session's
   inherited rows, a disposition for every type, and whether admitted
   meanings are preserved. Reach to the writer's source is an operation the
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
   not evidence; the mapping is.
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
7. **Recorded-token exactness has a fixed scope.** The header `version`,
   an ordinary event's `time` and a packed row's `time0` are judged on
   their recorded token under either version; no other field is.
8. **Unread parts withhold, never admit.** Any name outside the 56-name
   catalogue and the version-zero storage rows are refused under version
   three; the seeded association is withheld; the record says which read
   lifts each.

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
- **No `upstream` result from this seat.** The commission supports a sound
  specification; the evidence it asks for exists in this run's record.

### S7 — Validation of this sitting

Read the dialect through `openspec instructions proposal`, `specs` and
`tasks` for this change; they declare `proposal.md`, `specs/**/*.md` and
`tasks.md`, and no workflow runner was invoked. Adopted the existing change
and revised it in dependency order: the proposal, the two capability
deltas, the design and the tasks, each delta edit applied as an
exact-match replacement over requirements copied whole from the living
specifications. Reconfirmed this box cannot reach the writer (no `dsh`,
`volta`, `npm`, `cargo` or `~/.dsh`; HOME `/runtime/home`). Verified
against the tree the projector facts the design cites, including the
nested block reads at `transcript.rs:2278-2338`, the association keys at
`:2142-2171`, the header read at `:1915` and the `time`/`time0` exactness
tests at `tests.rs:2143` and `:2180`; confirmed from
`crates/brokkr-protocol` that the fleet's driver never seeds a session.
Strict OpenSpec validation was run over the whole tree after authoring.
Cargo is absent from this box, so no format, clippy, test or bundle gate
ran here; they belong to the implementation seat before its activation
commit, and the exact-coverage gate to host validation outside the box.
This sitting commits the proposal, both deltas, the design and the tasks,
unsigned, and pushes nothing.

### S8 — The judges' findings, answered

The clarify judge's four findings on the second sitting and the analyze
judge's seven on the third are answered here; the earlier answers that this
sitting supersedes are recorded in the change's history (`36bf374`,
`f34279e`).

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
