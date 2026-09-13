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

This change delivers both, part by part, and says which part is which. Its
first sitting (`23fafd1`) could not reach the `@deepseek-ai/dsh` 0.1.5-rc.1
writer from its box and kept version 3 refused. The design council met the
prerequisite inside this run: its simplicity seat resolved the installed
`dsh` on this host, read the session packages beneath it and recorded
digests and line citations; the design (`c1d57e9`) ruled that read the
run's primary evidence and returned the change to specification. The
second sitting (`923d72b`) admitted version 3 on it. The clarify judge then
found four defects in that admission's record, all of one kind: parts the
read had not reached were being admitted on version-zero shape. This
sitting answers each finding (S8). The header, the vocabulary, the quiet
set, `assistant/attempt`, `isSeeded` and `tool/call` are admitted on what
was read; the payloads of `user/message`, `assistant/message` and
`tool/result`, the persistence of replacement copies and the identity of a
seeded session's inherited rows were not read, so they are refused row by
row, and the exact read that lifts each refusal is named. The reader
refuses unmeasured admission everywhere else; this change is not the
exception.

## What Changes

- The admitted DSH version set becomes closed and exactly `{0, 3}`. Zero
  stays admitted on the #222 capture of the 0.1.2-rc.1 writer; three is
  admitted on the council's read of the installed 0.1.5-rc.1 writer
  (session packages 0.1.5-rc.2), cited by package, file, digest and line
  (S1). Three is admitted only in exact numeric spellings under the same
  recorded-token cross-check zero has; versions 1, 2 and every other value
  keep the refusal. The growth rule stays and now also demands the message
  and block definitions behind every projected payload, the persistence of
  replacement copies and the identity of inherited rows (S2).
- Row classification is keyed on the admitted version as well as the type.
  Under version three, `tool/call` projects from the fields the event map
  carries on the event itself; `assistant/attempt` is a counted omission;
  `system/message` and six other new names are quiet; `assistant/chunk`,
  the three packed storage rows and the two `tool/code-dispatch*` names are
  required unknowns; the eight version-three names are required unknowns
  under version zero. No name inherits a disposition across versions by
  string equality (S2, S3).
- **Version-three message payloads are refused until their definitions are
  read.** The read cites the event map's envelope fields for
  `user/message`, `assistant/message` and `tool/result` and not the message
  and block types the map imports, whereas version zero's block projection
  rests on the 0.1.2-rc.1 message and block definitions. Under version
  three each of those three rows is a refused row: `unsupported-format`,
  one unrecognized count, regardless of an `ignorable` marker, nothing
  projected, suppressed, cited or associated. The lifting read is named
  field by field (S1 item 3, S3, S8).
- Version 3 persists whole messages. The rule that fragments are never
  concatenated stands unchanged in text and has nothing to concatenate
  under version three; `interrupted`, `stream` and `usage` are never
  expanded under any admission of the row; the reader assembles nothing
  under either version (S3, Q2).
- `isSeeded` is inert over every JSON shape, including null and absent (S3,
  Q3). Whether a seeded session's inherited rows keep their `seq`, `turn`,
  `step` and call identities was not read, so embedded tool association is
  not applied under version three, which the payload refusal makes moot,
  and the lifting change must read the seed path before applying it (S8).
- `surfaceOp` never removes, reorders or replaces a row. Whether the writer
  persists replacement copies as message rows was not read; keeping earlier
  rows is measured, excluding model-only copies is not, and the two are
  separate operations. A replacement copy can reach the reader today only
  as a refused message row or a quiet `system/message`; the lifting change
  rules its disposition from the persistence source (S3, Q4; S8).
- The recorded-token exactness check judges the header `version`, an
  ordinary event's `time` and a packed row's `time0`, under either version,
  and no other field; admitting three widens the version check from one
  integer to two and touches nothing else (S2, S8).
- Discovery is unchanged. The versioned name stays the exclusive candidate
  when its opening row is a valid session header, and the requirement pins
  the mismatched-version pairings (S6).
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
  versions and what the version-three read did not reach, the exactness
  scope, `isSeeded` over every shape, the sibling and filename/version
  pinnings); "DSH sessions expose assembled or provisional content once"
  (whole messages under version three, `assistant/attempt`,
  version-zero-only storage rows, the payload refusal and its lifting read,
  `surfaceOp` and replacement copies, seeded association, time exactness);
  "Partial records and read failures remain distinguishable"
  (classification after admitted-version header admission under a
  per-version vocabulary, with the version-three quiet set named, the three
  refused content kinds named and the untranscribed remainder refused).

## Impact

- `openspec/specs/transcript-reading/spec.md` on fold, three requirements.
- `crates/brokkr-view/src/transcript.rs`: header admission yields the
  admitted version instead of a yes/no; the exact-integer token check
  generalizes from zero to zero-or-three; row dispatch takes the admitted
  version; under version three `tool/call` takes the existing path,
  `assistant/attempt` is an omission, the three message kinds are refused
  rows, a second quiet list keyed on version three is added and the
  version-zero list is untouched.
- `crates/brokkr-view/src/transcript/tests.rs`,
  `crates/brokkr-cli/src/ui/tests.rs`,
  `crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
  version matrix, disposition, payload-refusal, attempt, time-exactness,
  seeded, sibling-discovery and end-to-end scenarios of the delta, all with
  inline synthetic rows in test-owned homes; every existing DSH test
  unchanged.
- `docs/decisions/NNNN-*.md` at the next free number with
  `Status: proposed` and its row in `docs/decisions/README.md` (S5).
- Guides only where a line says "version zero only".
- No change to `crates/brokkr-cli/src/ui.rs`, `crates/brokkr-protocol`,
  frozen `contracts/`, `policy/`, `reference/` or `fixtures/`. No delta to
  `transcript-command` or `transcript-tui` (S6).
- The exact-coverage gate is unchanged; it runs outside the box and is
  recorded pending until its result exists.

## Decisions

### S1 — Evidence: one seat of this run read the installed writer; this seat cannot re-read it

The commission admits one primary source, the `@deepseek-ai/dsh`
0.1.5-rc.1 writer as installed on this host, and forbids sampling as a
substitute. Reachability is a property of a seat's box, not of the host.
Four boxes of this run could not reach the writer and one could:

| Seat | Reach on 2026-09-13 |
|---|---|
| specify, all three sittings (`23fafd1`, `923d72b`, this one) | HOME `/runtime/home`, user `runner`; no `dsh`, `volta`, `npm` or `cargo`; `/home/vyanakiev` holds only `source`; no `~/.dsh` under either home; `node` v22 and OpenSpec 1.12.0 only; no network; no capture under `.forge/`; no branch carrying the writer. Reconfirmed at this sitting. |
| design, robustness seat and chief | The same box shape; both reconfirmed it. |
| design, simplicity seat | `which dsh` resolved `/home/vyanakiev/.volta/bin/dsh`; `dsh --version` printed `0.1.5-rc.1`; the install root under `/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/` holds `dsh-session`, `dsh-session-format`, `dsh-session-format-catalog`, `dsh-session-persistence-jsonl` and three migration packages, each 0.1.5-rc.2. Every access was a read; no install, profile or credential was modified. |

The read is recorded with digests and line citations in the run-local
`.forge/design/positions/simplicity.md` and restated in design D2 and D4.
It is the run's primary evidence for the same reason the #222 capture was
version zero's: a run-local read of the installed writer, never committed,
cited from the change by file, line and digest. The digests it recorded:

| Source file (under `.../node_modules/@deepseek-ai/`) | SHA-256 |
|---|---|
| `dsh-session/lib/types/known-event-types.js` | `7bcf6e061a34b6107896b09ec048f5e62420ce1505191c1d63910765e014d909` |
| `dsh-session/lib/types/types.js` | `27de3ecc17fe395856b6182362ecfbe7f7b83a90238bbd09ccfa55c007193725` |
| `dsh-session/lib/types/seq-ranges.js` | `68a127c76affa98edeeb50e302eb43f154f4d24f7d04cb95ba8b323e88f3d09e` |
| `dsh-session/lib/types/surface.js` | `aad7aaabe6cd9b39ae4cc3b50a2873c9b5d73b69d929051f31b18ecc13647c72` |
| `dsh-session-format-v2-to-v3/lib/index.js` | `2d35e1e0ed497af569d5735fc590187de1568489cfe60d070b5f61330cd5a338` |
| `dsh-session-format-catalog/lib/index.js` | `bf4bde9e6563d7793f820c4a1b3141f6527283dd6c58f16bf43a67bc557cc48c` |

`seq-ranges.js` carries the digest the #222 capture recorded for the
0.1.2-rc.1 file: the citation grammar is byte-identical across the two
writers.

Every claim of the read that touches the reader was checked against the
tree by the design and again at this sitting: the projector's quiet list
holds 46 names and its content dispatch five kinds plus three packed rows;
`request/context`, `todo/write` and `turn/end` are quiet today;
`tool/code-dispatch` and `tool/code-dispatch-start` are in the version-zero
list; citations accept exactly individual integers and two-integer ranges;
nothing reads `surfaceOp`; header admission reads only `type`,
`delegationDepth` and `version`; the nested block projection reads
`text.text`, `reasoning.text`, `tool-call.{id,name,arguments}`,
`tool-result.{toolCallId,content,text}`, `image` and string content, and
the dedicated-tool association keys on call id, direction, turn and step
across the whole event list, independently of chunks and citations.
Nothing the read reports contradicts the repository's own record of
version zero.

**What the read did not establish**, so the requirement refuses it instead
of admitting it with the version. Items 3, 7 and 8 are the clarify judge's
findings; the read that lifts each is named.

1. **The catalogue was not transcribed.** The read reports 58 names and an
   eight-name difference from the reader's 51, with three reader names
   (`assistant/chunk`, `tool/code-dispatch`, `tool/code-dispatch-start`)
   absent from version 3; 51 − 3 + 8 is 56, so two names are unaccounted
   for. The version-3 vocabulary is closed at the 56 names the read
   enumerates, and any other name is a required unknown under version 3.
2. **`dsh-session-persistence-jsonl` 0.1.5-rc.2 was identified, not
   read.** Its row kinds, header write and file naming are uncited, so no
   packed storage row is admitted under version 3.
3. **The message and block definitions behind version-three payloads were
   not read.** The read cites the event map (`types.d.ts:309-338`): content
   in `data` for `user/message`, `data.message` for `assistant/message` and
   `tool/result`, `callId`, `name` and `arguments` on `tool/call`. It does
   not cite the message and block types that map imports. Version zero's
   projection of `text`, `reasoning`, `tool-call`, `tool-result`, `image`
   and string content was measured against the `dsh-v0.1.2-rc.1` message
   and block definitions (`packages/llm/llm/src/message.ts` and
   `types.ts`, archived #222 design D6 and its evidence list); no
   equivalent read exists for 0.1.5. A familiar block name is not evidence
   that its fields or meaning are unchanged, and counting an unknown name
   does not measure a familiar one. **Consequence:** under version three
   the three message-carrying kinds are refused rows (S2, S3). **Lifting
   read:** the 0.1.5-rc.2 message and block definitions the session
   `types.d.ts` imports, by package, file, digest and line, mapping
   `content` (array or string), `text.text`, `reasoning.text`,
   `tool-call.id`, `tool-call.name`, `tool-call.arguments`,
   `tool-result.toolCallId`, `tool-result.content`, `tool-result.text`,
   `image` and every other block variant to its version-three meaning and
   disposition.
4. **The element shape of the embedded `stream`** on `assistant/message`
   and `assistant/attempt` was not cited. The stream is never expanded.
5. **The relation between a version-3 citation and its owning `seq`** was
   not cited; the grammar was. Citation validation reaches no version-three
   row while item 3 stands.
6. **The `.d.ts` files cited by line** (`types.d.ts`, `surface.d.ts`) are
   not in the digest table; their `.js` siblings from the same package
   version are.
7. **The append and replace paths were not read.** `surface.d.ts:27-33`
   distinguishes append-origin events, "that transcript's durable source
   material", from replacement copies that "stay model-only"; whether a
   replacement copy is persisted as a message row of the session file, what
   it carries and how an append-origin row is told from it are uncited,
   because the persistence package (item 2) and the surface writer's
   append and replace functions were not read. **Consequence:** the reader
   keeps every earlier row (measured, and the audit rule) and rules nothing
   about excluding model-only copies; today such a copy is a refused
   message row or a quiet `system/message`. **Lifting read:** the
   persistence and surface source for append and replace, with the
   disposition of a persisted replacement copy and synthetic
   append-plus-replacement and compaction scenarios.
8. **The seed and fork path was not read.** `isSeeded` means a
   fork-inherited event prefix exists (`types.d.ts:72-76`) and
   `inheritedEventCount` is session state (`types.d.ts:105-109`); whether
   inherited rows keep their originating `seq`, `turn`, `step` and call
   identities, or are remapped or reused, is uncited. **Consequence:**
   embedded tool association is not applied under version three; with item
   3 standing no message or `tool/result` row projects, so there is nothing
   to associate. **Lifting read:** the seed and fork writer path, with
   seeded scenarios in which inherited and new embedded and dedicated calls
   and results suppress a genuine duplicate once and keep unrelated or
   ambiguous identities.
9. **The `time` and `time0` exactness checks were never in question.** The
   previous sitting's clause "gates `version` and no other field" read as
   removing them; it meant only that admitting three adds no field to the
   check. The requirement now names the three fields the recorded-token
   check judges under either version (S2).

### S2 — The admitted set is `{0, 3}`, spelled exactly, classified per version

**Set.** Zero by the #222 capture: the 0.1.2-rc.1 session package declares
`SESSION_FORMAT_VERSION = 0` and its persistence package writes that header
and refuses a foreign version before decoding (archived #222 design, D6).
Three by S1: the 0.1.5-rc.2 session package declares
`SESSION_FORMAT_VERSION = 3` (`types.js:54`, `lib/index.js:56`) and the
catalog package declares `currentVersion: 3` with codecs `v0..v3` and
lossless migrations `v0→v1→v2→v3` (`dsh-session-format-catalog/lib/index.js:34-46`).
Versions 1 and 2 are not admitted: no core of this fleet wrote them to disk
under a name discovery admits, and no seat read their codecs. Every other
value refuses exactly as today, and no version is admitted by a versioned
filename, a header number or the resemblance of sampled rows.

**Spellings.** Zero keeps its rule. Three is admitted when the parsed value
is three and the recorded token, with its decimal point and leading and
trailing zeros removed, spells exactly `3`: `3`, `3.0`, `3e0`, `30e-1` and
`0.3e1` admit; `"3"`, `3.1`, `-3`, `3e1` and the rounding artefacts
`2.9999999999999999` and `3.0000000000000001` refuse. The recorded-token
cross-check the zero path already performs is kept for three (the design's
robustness position, §5). **Its scope is unchanged by this change:** the
reader judges three fields on their recorded token, the header `version`,
an ordinary event's `time` and a packed row's `time0`, so that a nonzero
literal whose parsed value collapses to zero is neither version zero, nor
a zero millisecond stamp, nor a valid packed base time. Admitting three
widens the version check from one admitted integer to two and adds no
field; `surfaceOp`, citations and every other event field are not judged
on their recorded token under either version. The alternative, a
parsed-value-only check for three, was rejected because it would admit a
token the writer never wrote and drop a defence the zero path has.

**Per-version vocabulary.** A row is classified under the vocabulary of
the version that admitted its file. `tool/call` keeps the one path under
both versions because the fields it projects, `callId`, `name` and
`arguments`, are carried on the event itself in the version-three event
map (`types.d.ts:309-338`); its `turn` and `step` are read only for
association, which has no admitted version-three counterpart. The three
message-carrying kinds share an envelope nesting the map cites but a
payload it does not (S1 item 3), so under version three they are refused
rows: recognized as content kinds of the vocabulary, `unsupported-format`,
one unrecognized count each, regardless of an `ignorable` marker, nothing
projected, suppressed, cited or associated. A union quiet list was
rejected (design D1): it would admit `assistant/chunk` and packed rows
under a writer whose catalogue has no chunk type and whose persistence
package is unread, and it would quiet-list version-3 names under a writer
that never emitted them. Projecting version-three payloads on the
version-zero block parser was rejected at this sitting: it is the
unmeasured admission the commission forbids, and the previous sitting's
own result record listed the block vocabulary as refused while the delta
projected it. Counting refused message rows as omissions instead of
refusing the read was rejected because an audit read that silently shows
zero turns for a session full of conversation is the empty-recording
failure the diagnostics rules exist to prevent; `assistant/attempt` is
counted rather than refused only because its text was never surfaced by
the writer. The cost of the per-version vocabulary is one extra argument
through dispatch, one second list and one refusal arm.

### S3 — The six questions of #279, answered on the read

Each row gives the measured answer with its evidence (line citations are
the simplicity seat's, into the 0.1.5-rc.2 packages of S1), the reader
consequence, and what stays unmeasured and therefore refused.

| # | Question | Measured answer and evidence | Reader consequence | Unmeasured, therefore refused |
|---|---|---|---|---|
| 1 | Record types and shapes the v3 writer emits | The vocabulary is the generated set in `known-event-types.js` (digest `7bcf6e06…`), reported as 58 names; the event map in `types.d.ts` gives each type's envelope fields; the surface-eligible types are exactly `system/message`, `user/message`, `assistant/message` and `tool/result` (`types.d.ts:413`), every other type being log-only (`surface.d.ts:52-64`). The reader handles 51 names, of which 48 are shared with v3. | A per-version vocabulary (S2); the dispositions of Q5 and design D7; `tool/call` projects on its event-map fields. | The full transcription (S1 item 1); the persistence package's rows (item 2); the message and block definitions behind the three message-carrying kinds (item 3), so those rows refuse. |
| 2 | `assistant/chunk` absent from v3 | Removed, not renamed and not conditional. The v1→v2 package is the assistant-stream migration; in v3 the timed stream is embedded: `assistant/message` carries `message`, `stream`, optional `usage` and `interrupted?: true` (`types.d.ts:309-317`); `assistant/attempt` carries a stream with no surface message (`types.d.ts:323-327`); "a turn cancelled mid-stream finalizes its delivered text/reasoning prefix as this event with `interrupted: true`" (`types.d.ts:303-307`). | Whole messages only under version 3, so the fragment rule has nothing to concatenate and the reader assembles nothing under either version. `stream`, `usage` and `interrupted` are never expanded under any admission of the row; `assistant/attempt` is a counted omission because it carries model output the writer did not surface, and calling it quiet would drop text silently; `assistant/chunk` under version 3 is a required unknown. The finalized message itself is not displayed until item 3 is read: the interrupted-prefix behaviour is recorded as a measurement and pinned as a refusal today. | The stream element shape (S1 item 4); the message payload (item 3). |
| 3 | `isSeeded` new in the header | A required v3 header boolean meaning the session contains a fork-inherited event prefix (`types.d.ts:72-76`); the prefix length is session state (`inheritedEventCount`, `types.d.ts:105-109`), not header metadata; delegation depth is a separate field (`types.d.ts:82-87`). Ownership is `type: session` plus depth, and header admission reads no other field. | Inert for ownership, depth and admission; pinned over true, false, string, object, null and absent under both versions; no provenance or delegation claim rests on it; it changes no row's classification. | Whether inherited rows keep their originating `seq`, `turn`, `step` and call identities (S1 item 8). This bears on the dedicated-tool association, which keys on call id and turn/step independently of chunks; the previous "no chunks, so irrelevant" reasoning was wrong and is withdrawn. Association is not applied under version three until the seed path is read; today the payload refusal leaves nothing to associate. |
| 4 | `surfaceOp` and `sourceEventSeqs` rise to about 21% of rows | `surfaceOp` is `'append' \| {op: 'replace', startSeq, endSeq}`, required only on the four surface types (`types.d.ts:429-441`). `sourceEventSeqs` is individual non-negative integers plus inclusive `[start, end]` pairs, strictly increasing (`seq-ranges.js`, the same digest as the 0.1.2-rc.1 file). The rise is the writer placing every surface message on the surface with explicit markers. The writer's contract: the surface "is the wrong source for a human transcript — a landed replacement would erase conversation the user already saw. Append-origin events are that transcript's durable source material; replacement copies stay model-only" (`surface.d.ts:27-33`). | The reader never removes, reorders or replaces a row on `surfaceOp` and never replays the surface: that much is measured and is 0055 ruling 3's audit rule. The contract separates append-origin material from model-only copies; ignoring the marker keeps the former and says nothing about excluding the latter. The reader therefore neither displays nor suppresses a version-three row on `surfaceOp`; a replacement copy today is a refused message row or a quiet `system/message`. Citation validation is unchanged in grammar and reaches no version-three row while the payload refusal stands. | Whether replacement copies are persisted as rows, what they carry and how append origin is told (S1 item 7); whether a v3 citation can be non-earlier than its owning `seq` (item 5). |
| 5 | `system/message`, `todo/write`, `turn/end` in v3, not in version zero | `system/message` is the rendered system prompt as surface node 0, replacing the head (`types.d.ts:288-294`; the v2→v3 migration's `emitSystem`, `dsh-session-format-v2-to-v3/lib/index.js:780-810`), the successor of version zero's `request/context`, which is quiet. `todo/write` and `turn/end` are in both catalogues and log-only under v3. | All three quiet under version 3, whatever their payload or `surfaceOp`. `system/message` under version zero stays a required unknown: the 0.1.2-rc.1 catalogue does not contain it. A displayed `system` turn would be new command and TUI capability, not asked for. | Nothing the quiet disposition depends on: quiet reads no payload. |
| 6 | Is version 3 a superset of version zero? | **No.** Versions are physical generations joined by migrations, not sets: v3 removes `assistant/chunk` and, per the read, the two `tool/code-dispatch*` names; adds at least eight names; moves the prompt from `request/context` to `system/message` and the stream from chunk rows into the message. The event map keeps the envelope nesting the reader reads for the four content kinds (`types.d.ts:309-338`). | Admit `3` beside `0` with a per-version vocabulary and one projector for what is measured: `tool/call`. Every version-zero meaning is preserved for version-zero files. Whether the message payloads' meanings are preserved is exactly what was not measured, so they are refused rather than projected on version-zero shape. | Whether the message block vocabulary moved (S1 item 3); if the lifting read shows it did, that change adds a per-version payload projection rather than widening this one. |

The eight names version 3 adds to what the reader handles are
`assistant/attempt`, `deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `system/message`,
`tool/ptc-dispatch` and `tool/ptc-dispatch-start`; the last two succeed
`tool/code-dispatch` and `tool/code-dispatch-start` through the v2→v3 PTC
migration. Of the eight, only `system/message` is surface-eligible, and it
maps to the `request/context` precedent; `assistant/attempt` is counted;
the other six are log-only and quiet.

### S4 — What this change delivers, what the lifting change must read, and corroboration

**Delivered now, on the read:** the header admission of three with its
exact spellings; the per-version vocabulary with the 51-name quiet set;
`tool/call` projection; `assistant/attempt` counted; `isSeeded` inert;
`surfaceOp` never rewriting audit order; the sibling and filename/version
pinnings; the decision of S5; and the payload refusal itself, which is a
specified, tested behaviour with a diagnostic, not a gap. Its observable
effect on a real seat transcript is that every version-three session
carrying a message row refuses with `unsupported-format`, a confirmed path
and a count of the refused rows, until the lifting change lands. That is
the same outcome the operator has today, stated precisely, with the
scaffolding for the admission in place and tested.

**The lifting change** is a further OpenSpec change against the same
requirement. Its prerequisite is one read of the installed writer by a
seat whose box reaches it, or a controller capture in the #222 form, of:
(a) the message and block definitions the 0.1.5-rc.2 session `types.d.ts`
imports, mapped field by field (S1 item 3); (b) the persistence package
and the surface append and replace paths, ruling whether replacement
copies are persisted and how each displays, is omitted or refuses (item
7); (c) the seed and fork path, ruling whether inherited identities are
valid association evidence (item 8). Each read is cited by package, file,
digest and line, and each ruling is pinned with synthetic scenarios naming
exact counts, order and suppression. The two untranscribed catalogue names
and the stream element shape (items 1 and 4) may be settled in the same
read; they are not prerequisites for lifting the payload refusal.

**Corroboration** follows the writer and never leads. After landing, the
controller reads the 17 version-3 sessions under `~/.dsh/sessions/brokkr`
through the built reader on the host and confirms each refuses with the
confirmed path and a count equal to its message rows, with no other
refusal cause, and compares the row types they carry against the 56-name
vocabulary. An untranscribed name or a packed row surfaces there. No seat
reads the 17 sessions, copies them or commits them (decision 0032); every
test in this change uses synthetic rows in test-owned homes.

### S5 — A proposed decision carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes ("DSH numeric
on-disk version zero only"), so this change files a numbered decision with
`Status: proposed`, supplementing ruling 3 without editing it, at the next
free number. 0060 was free on main at the last sitting, but a
`review-first` run's journal anchor on this repository now names a
`docs/decisions/0060-review-first-…md`, so the decision-owning
implementation seat re-reads the index at its PR and takes the next free
number then. The decision is written beside its code, with its README row,
and rules:

1. **Admitted versions are measured, closed and named.** Zero on the
   0.1.2-rc.1 capture, three on the 0.1.5-rc.1 read with session packages
   0.1.5-rc.2 (the S1 digests). A version joins only through a change
   recording the writer's package identity and files with digests, the
   record vocabulary, the message and block definitions behind every
   projected payload mapped field by field, how fragments persist, the
   meaning of every added header field, the meaning of `surfaceOp` and
   `sourceEventSeqs` where carried including whether replacement copies
   are persisted as rows, the identity of a seeded session's inherited
   rows, a disposition for every type, and whether admitted meanings are
   preserved. A versioned filename, a header number and the resemblance of
   sampled rows are not evidence.
2. **Vocabulary is per version.** A row is classified under the vocabulary
   of the version that admitted its file; a shared name keeps a disposition
   in each version only by measurement; a name outside the admitted
   version's vocabulary is a required unknown. Version-zero storage rows
   and `assistant/chunk` are version-zero only.
3. **A payload projects only on its own writer's definitions.** Under
   version three `tool/call` projects from its event-map fields;
   `user/message`, `assistant/message` and `tool/result` are refused rows,
   counting once and refusing the read regardless of an `ignorable`
   marker, until the version-three message and block definitions are read
   and each consumed field is mapped. A familiar block name is not
   evidence.
4. **Version three persists whole messages.** No fragment rows; an
   interrupted step is one finalized `assistant/message` with
   `interrupted: true`; embedded streams and usage are never expanded;
   `assistant/attempt` is a counted omission. The reader assembles nothing
   under either version.
5. **The surface is not the transcript.** `surfaceOp` never removes,
   reorders or replaces a row and is never replayed; suppression remains
   cited-unique-earlier-same-step chunks only. Whether replacement copies
   are persisted, and their disposition, is ruled only from the persistence
   and surface source; until then the reader neither displays nor
   suppresses a row on the marker.
6. **`isSeeded` is inert** for ownership, depth and admission, whatever its
   JSON shape, including null and absent. Inherited identities are
   association evidence only once the seed path is read; until then
   embedded association is not applied under version three.
7. **Recorded-token exactness has a fixed scope.** The header `version`,
   an ordinary event's `time` and a packed row's `time0` are judged on
   their recorded token under either version; no other field is.
8. **Unmeasured parts refuse.** The persistence package's rows, any
   catalogue name not transcribed, and the three payloads above are
   refused under version three until read; the record says which read.

Enforcement bindings: the version-matrix, disposition, payload-refusal,
attempt, time-exactness, sibling-discovery and seeded tests named in the
delta's scenarios, `openspec validate --all --strict`, and the
exact-coverage gate. 0032's ownership, retention and privacy constraints
are preserved.

### S6 — Reasoned refusals

- **No fall-through in discovery.** When the versioned name carries a
  refused version and the plain name beside it is readable, the read
  refuses with the versioned path. Falling through would present
  superseded evidence as the seat's newest transcript, the same reason the
  version-zero rule refuses to rank roots by version. The pairing this
  fleet produces, version 3 beside version zero, reads the version-3 file
  and never the plain name's content (design D8).
- **No projection of version-three payloads on version-zero block
  names.** The block parser was measured against 0.1.2-rc.1 definitions;
  applying it to 0.1.5 payloads is resemblance, which the commission names
  as non-evidence. The three message-carrying kinds refuse instead.
- **No `ignorable` bypass for a refused content row.** The marker licenses
  omitting an operational record the reader does not know; letting it omit
  a message would drop conversation content silently, the failure the
  audit read exists to prevent.
- **No omission-counting of refused message rows.** A zero-turn success
  over a session full of messages is the empty-recording failure the
  diagnostics rules forbid; refusal with a count is the honest state.
- **No `surfaceOp` model and no surface replay.** Nothing gates
  suppression on the marker, the writer calls the surface the wrong source
  for a human transcript, and whether replacement copies are even
  persisted is unread; the lifting change rules it from source.
- **No association across a seeded prefix on unread identities.** The
  dedicated-tool rule keys on call id and turn/step; whether inherited rows
  keep those identities is unread, so the rule is not applied under
  version three until it is.
- **No displayed `system` turn.** `system/message` is the prompt copy that
  `request/context` was; showing it adds a role to the command and TUI
  that no requirement asks for. Quiet is the measured default.
- **No stream expansion and no attempt projection.** The stream element
  shape is uncited (S1 item 4); expanding it would duplicate the message
  or fabricate fragments. An attempt's text is counted, not silent, and a
  later change may project it once its shape is cited.
- **No union vocabulary and no second projector now.** The union admits
  rows a writer cannot emit; a second projector for payloads is warranted
  only if the lifting read shows the block vocabulary moved (S3 Q6).
- **No migration and no versions 1 or 2.** Version-zero files read as
  before; no on-disk file of this fleet carries 1 or 2 and no seat read
  their codecs.
- **No `transcript-command` or `transcript-tui` delta.** Their observable
  behaviour for a readable or refused DSH source is unchanged; a refused
  version-three read is the existing `unsupported-format` presentation
  with its counts and notices. The TUI requirement's phrase "version-zero
  event/storage refusal SHALL keep R14's complete-prefix counts" stays
  true, and the reading requirement now states the same rule for every
  admitted version; widening that phrase is wording, left to the fold.
- **No `upstream` result from this seat.** The commission supports a sound
  specification; what is missing is a read a design seat of this run has
  already shown to be reachable. Under this recipe `upstream` from
  specification parks the run, which would trade a precise refusal for no
  delivery.

### S7 — Validation of this sitting

Read the dialect through `openspec instructions proposal` and
`openspec instructions specs` for this change; they declare `proposal.md`
and `specs/**/*.md`, and no workflow runner was invoked. Adopted the
existing change and revised it in dependency order: the proposal, then the
capability delta, each edit applied as an exact-match replacement over the
three requirements the previous sitting copied whole from
`openspec/specs/transcript-reading/spec.md`. Reconfirmed this box cannot
reach the writer (no `dsh`, `volta`, `npm`, `cargo` or `~/.dsh`; HOME
`/runtime/home`; `/home/vyanakiev` holds only `source`), so S1 items 3, 7
and 8 are refused rather than measured here. Verified against the tree the
projector facts S1 lists, including the nested block reads at
`transcript.rs:2278-2338`, the association keys at `:2142-2171` and the
`time`/`time0` exactness tests at `tests.rs:2143` and `:2180`. Confirmed
the living specification never states the `time` exactness rule in
recorded-token terms, so the delta now states it. Strict OpenSpec
validation was run over the whole tree after authoring. Cargo is absent
from this box, so no format, clippy, test or bundle gate ran here; they
belong to the implementation seat before its activation commit, and the
exact-coverage gate to host validation outside the box. This sitting
commits only the proposal and the capability delta, unsigned, and pushes
nothing.

### S8 — The clarify findings, answered

| Finding | Answer | Where |
|---|---|---|
| CLARIFY-279-1: which version-three nested message and block shapes are measured, and what refuses when they are not? | None are measured; the read cites the event map's envelope only. Under version three `user/message`, `assistant/message` and `tool/result` are refused rows, regardless of an `ignorable` marker; `tool/call` projects on the event-map fields carried on the event itself. The lifting read is named field by field. The projecting version-three message scenarios are replaced by a refusal scenario that covers string content, familiar block names, `interrupted`/`stream`/`usage`, a `surfaceOp` replacement, citations, an embedded tool-result naming a dedicated call, and the marker, with exact counts. | Delta: Discovery evidence paragraph; content requirement, "The message and block definitions…" paragraph; classification vocabulary sentence; scenarios "Version-3 message rows refuse…", "A version-3 session is located, its measured rows project and its message rows refuse". Proposal: S1 item 3, S2, S3 Q1/Q6, S5 ruling 3. |
| CLARIFY-279-2: what does the writer persist for replacement copies, and why does ignoring `surfaceOp` produce the intended transcript? | Unread. The record now separates the two operations: keeping earlier rows is measured and stays the rule; excluding model-only copies is not established and is not claimed. The reader neither displays nor suppresses a version-three row on the marker; a replacement copy reaches it today only as a refused message row or a quiet `system/message`. The lifting change rules the disposition from the persistence and surface source with append-plus-replacement and compaction scenarios. The retained version-zero compaction scenario is now scoped to version zero by name. | Delta: content requirement, "The citation grammar above…" paragraph; scenario "Interrupted assembly and compaction keep the audit order" (version-zero); refusal scenario's `surfaceOp` case. Proposal: S1 item 7, S3 Q4, S5 ruling 5, S6. |
| CLARIFY-279-3: what proves inherited identities are valid association evidence across a seeded prefix? | Nothing yet; the seed path was not read, and the "no chunks, so irrelevant" reasoning is withdrawn because the dedicated-tool rule keys on call id and turn/step independently of chunks. `isSeeded` stays inert for ownership, depth and admission. Embedded association is not applied under version three; with the payload refusal there is nothing to associate. The lifting change reads the seed path and pins seeded scenarios or leaves the association unapplied. | Delta: Discovery `isSeeded` paragraph; content requirement, "Dedicated call/result ownership…" paragraph; seeded scenario's classification and inherited-prefix clauses; refusal scenario's embedded tool-result case. Proposal: S1 item 8, S3 Q3, S5 ruling 6, S6. |
| CLARIFY-279-4: does "gates `version` and no other field" remove the existing `time`/`time0` exactness checks? | No, and the wording that read that way is gone. Admitting three widens the version check from one integer to two and touches no other field; the recorded-token check judges `version`, `time` and `time0` under either version and no other field. The existing underflow regressions stay and a version-three time scenario pins the scope with `tool/call` rows, the one content kind projecting under version three today. | Delta: Discovery spellings paragraph; content requirement, "Version-three admission changes nothing about time" paragraph; scenario "Version-three time keeps the recorded-token exactness of version zero". Proposal: S1 item 9, S2, S5 ruling 7. |

**Design sections this return supersedes**, for the design revisit that
follows a clear clarification (the design cannot return the change to
specification again without parking the run, so it revises itself):
D2 item 3 and D3's "Dispatch" paragraph (the three message kinds are
refused rows, not projected through the shared path); D5's first and
second rulings (the finalized interrupted message is a measurement and a
refusal today, not a projected turn); D6 (replacement copies are unread,
and the exactness clause now names three fields); D7's first row
(`tool/call` projects, the other three refuse); D10's "Fragments" and
"Seeded" bullets (replaced by the payload-refusal, attempt, time-exactness
and seeded-classification scenarios); D9's ruling 4 and its numbering
(next free number, re-read at the PR); D11 items 3 and 4 (discharged as
revised here). D1, D2's digests and verifications, D3's set and spellings,
D4 as corrected in S3, D7's remaining rows and D8 stand.
