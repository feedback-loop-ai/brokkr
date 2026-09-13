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

This change now delivers the first outcome. Its first sitting (`23fafd1`)
delivered the second: the seat that authored it could not reach the
`@deepseek-ai/dsh` 0.1.5-rc.1 writer from its box, kept version 3 refused,
recorded the six open questions and named the exact prerequisite (a
controller capture of the installed packages, or a seat whose box can read
the install). The design council met that prerequisite inside this run: its
simplicity seat resolved the installed `dsh` on this host, read the session
packages beneath it and recorded digests and line citations; the design
(`c1d57e9`) verified every reader-touching claim of that read against the
tree, ruled it the run's primary evidence and returned the change to
specification, as the first sitting's own prerequisite clause required. The
six answers are recorded here and in the requirement. The parts the read
did not reach stay refused, and are named, rather than guessed.

## What Changes

- The admitted DSH version set becomes closed and exactly `{0, 3}`. Zero
  stays admitted on the #222 capture of the 0.1.2-rc.1 writer; three is
  admitted on the council's read of the installed 0.1.5-rc.1 writer
  (session packages 0.1.5-rc.2), cited by package, file, digest and line
  (S1). Three is admitted only in exact numeric spellings under the same
  recorded-token cross-check zero has; versions 1, 2 and every other value
  keep the refusal. The growth rule stays: a version joins only through a
  change that records the writer's own source.
- Row classification is keyed on the admitted version as well as the type.
  A version-3 file is read under a version-3 vocabulary: the four content
  kinds project through the envelope fields the read cites;
  `assistant/attempt` is a counted omission; `system/message` and six other
  new names are quiet; `assistant/chunk`, the three packed storage rows and
  the two `tool/code-dispatch*` names are required unknowns under version
  3; the eight version-3 names are required unknowns under version zero. No
  name inherits a disposition across versions by string equality (S2, S3).
- Version 3 persists whole messages. The rule that fragments are never
  concatenated stands unchanged in text, and its version-3 behaviour is
  stated beside it: an interrupted step is one finalized `assistant/message`
  with `interrupted: true`, shown as recorded with no marker; embedded
  `stream` and `usage` are quiet metadata; the reader assembles nothing
  under either version (S3, Q2).
- `isSeeded` is inert over every JSON shape, including null and absent (S3,
  Q3). `surfaceOp` is never read; the citation grammar is shared, and
  suppression, which only ever removes cited earlier chunks, removes nothing
  under version 3 (S3, Q4).
- Discovery is unchanged. The versioned name stays the exclusive candidate
  when its opening row is a valid session header, and the requirement now
  pins the mismatched-version pairings: version 3 beside version zero reads
  the version-3 file; a refused future version beside a readable older name
  refuses with the versioned path and does not fall through (S6).
- The projector gains what the version-3 vocabulary needs and nothing more,
  and host-independent tests pin every disposition with synthetic rows. A
  `Status: proposed` decision 0060 carries the admission and the evidence
  standard, supplementing 0055 ruling 3 (S5).
- What the read did not establish stays refused and is named: the
  persistence package's row kinds, two catalogue names not transcribed, the
  element shape of embedded streams, the block vocabulary of version-3
  message content and the seq relation of version-3 citations (S1, S4).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `transcript-reading`: "Discovery identifies one owned local file" (the
  admitted set `{0, 3}`, exact-three spellings, the evidence behind both
  versions, `isSeeded` over every shape, the sibling and filename/version
  pinnings); "DSH sessions expose assembled or provisional content once"
  (whole messages under version 3, `interrupted`, `stream`, `usage`,
  `assistant/attempt`, version-zero-only storage rows, the shared citation
  grammar with nothing to suppress, `surfaceOp` unread); "Partial records
  and read failures remain distinguishable" (classification after
  admitted-version header admission under a per-version vocabulary, with
  the version-3 quiet set named and the untranscribed remainder refused).

## Impact

- `openspec/specs/transcript-reading/spec.md` on fold, three requirements.
- `crates/brokkr-view/src/transcript.rs`: header admission yields the
  admitted version instead of a yes/no; the exact-integer token check
  generalizes from zero to zero-or-three; row dispatch takes the admitted
  version; a second quiet list keyed on version three, the version-zero
  list untouched.
- `crates/brokkr-view/src/transcript/tests.rs`,
  `crates/brokkr-cli/src/ui/tests.rs`,
  `crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
  version matrix, disposition, fragment, seeded, sibling-discovery and
  end-to-end scenarios of the delta, all with inline synthetic rows in
  test-owned homes; every existing DSH test unchanged.
- `docs/decisions/0060-*.md` with `Status: proposed` and its row in
  `docs/decisions/README.md` (S5).
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
Three boxes of this run could not reach the writer and one could:

| Seat | Reach on 2026-09-13 |
|---|---|
| specify, first sitting (`23fafd1`) and this sitting | HOME `/runtime/home`; no `dsh`, `volta`, `npm` or `cargo`; no `~/.dsh` under either home; `node` v22 and OpenSpec 1.12.0 only; no network; no capture under `.forge/`; no branch carrying the writer. Reconfirmed at this sitting. |
| design, robustness seat and chief | The same box shape; both reconfirmed it. |
| design, simplicity seat | `which dsh` resolved `/home/vyanakiev/.volta/bin/dsh`; `dsh --version` printed `0.1.5-rc.1`; the install root under `/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/` holds `dsh-session`, `dsh-session-format`, `dsh-session-format-catalog`, `dsh-session-persistence-jsonl` and three migration packages, each 0.1.5-rc.2. Every access was a read; no install, profile or credential was modified. |

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
`delegationDepth` and `version`. Nothing the read reports contradicts the
repository's own record of version zero.

**What the read did not establish**, so the requirement refuses it instead
of admitting it with the version:

1. **The catalogue was not transcribed.** The read reports 58 names and an
   eight-name difference from the reader's 51, with three reader names
   (`assistant/chunk`, `tool/code-dispatch`, `tool/code-dispatch-start`)
   absent from version 3; 51 − 3 + 8 is 56, so two names are unaccounted
   for. This sitting cannot transcribe the set against digest
   `7bcf6e06…` (table above). The version-3 vocabulary is therefore closed
   at the 56 names the read enumerates (four content, one counted, 51
   quiet), and any other name is a required unknown under version 3. That
   fails loud, with the confirmed path and a counted row, on exactly the
   sessions that carry such a name, and invents nothing.
2. **`dsh-session-persistence-jsonl` 0.1.5-rc.2 was identified, not
   read.** Its row kinds, header write and file naming are uncited, so no
   packed storage row is admitted under version 3. The filename is already
   admitted by #278 on the operator's ruling.
3. **The block vocabulary inside `message.content`** was not cited from the
   0.1.5-rc.2 types; the read cites the envelope fields the four content
   kinds carry. An unknown block under version 3 is the existing counted
   omission that keeps its siblings.
4. **The element shape of the embedded `stream`** on `assistant/message`
   and `assistant/attempt` was not cited. The stream is never expanded.
5. **The relation between a version-3 citation and its owning `seq`** was
   not cited; the grammar was. The existing earlier-than-owner rule stays,
   and a row violating it refuses as any row would.
6. **The `.d.ts` files cited by line** (`types.d.ts`, `surface.d.ts`) are
   not in the digest table; their `.js` siblings from the same package
   version are.

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
robustness position, §5) and gates `version` and no other field, because
no other field gates meaning. The alternative, a parsed-value-only check
for three, was rejected because it would admit a token the writer never
wrote and drop a defence the zero path has.

**Per-version vocabulary.** A row is classified under the vocabulary of
the version that admitted its file. The shared content kinds keep the
envelope fields the reader reads (`user/message` content in `data`;
`assistant/message` and `tool/result` in `data.message`; `tool/call` with
`callId`, `name` and `arguments`; `types.d.ts:309-338`), so one projector
serves both versions; only the vocabulary differs. A union quiet list was
rejected (design D1): it would admit `assistant/chunk` and packed rows
under a writer whose catalogue has no chunk type and whose persistence
package is unread, and it would quiet-list version-3 names under a writer
that never emitted them. The cost of the per-version vocabulary is one
extra argument through dispatch and one second list.

### S3 — The six questions of #279, answered on the read

Each row gives the measured answer with its evidence (line citations are
the simplicity seat's, into the 0.1.5-rc.2 packages of S1), the reader
consequence, and what stays unmeasured and therefore refused.

| # | Question | Measured answer and evidence | Reader consequence | Unmeasured, therefore refused |
|---|---|---|---|---|
| 1 | Record types and shapes the v3 writer emits | The vocabulary is the generated set in `known-event-types.js` (digest `7bcf6e06…`), reported as 58 names; payload shapes are the event map in `types.d.ts`; the surface-eligible types are exactly `system/message`, `user/message`, `assistant/message` and `tool/result` (`types.d.ts:413`), every other type being log-only (`surface.d.ts:52-64`). The reader handles 51 names, of which 48 are shared with v3. | A per-version vocabulary (S2); the dispositions of Q5 and design D7. | The full transcription (S1 item 1); the persistence package's rows (item 2); the message block vocabulary (item 3). |
| 2 | `assistant/chunk` absent from v3 | Removed, not renamed and not conditional. The v1→v2 package is the assistant-stream migration; in v3 the timed stream is embedded: `assistant/message` carries `message`, `stream`, optional `usage` and `interrupted?: true` (`types.d.ts:309-317`); `assistant/attempt` carries a stream with no surface message (`types.d.ts:323-327`); "a turn cancelled mid-stream finalizes its delivered text/reasoning prefix as this event with `interrupted: true`" (`types.d.ts:303-307`). | Whole messages only under version 3. The fragment rule's text stands and its version-3 behaviour is stated in the requirement: an interrupted step retains one finalized message, the writer's own, with no marker; `stream` and `usage` are quiet; `assistant/attempt` is a counted omission because it carries model output the writer did not surface, and calling it quiet would drop text silently; `assistant/chunk` under version 3 is a required unknown. | The stream element shape (S1 item 4): never expanded. |
| 3 | `isSeeded` new in the header | A required v3 header boolean meaning the session contains a fork-inherited event prefix (`types.d.ts:72-76`); the prefix length is session state (`inheritedEventCount`, `types.d.ts:105-109`), not header metadata; delegation depth is a separate field (`types.d.ts:82-87`). Ownership is `type: session` plus depth, and header admission reads no other field. | Inert for ownership, depth and admission; pinned over true, false, string, object, null and absent under both versions. No provenance or delegation claim rests on it. | Whether inherited prefix events keep their originating `seq`/`turn`/`step`. It does not bear on version 3, where no chunk exists to cite across a seed boundary, and under version zero the uniqueness rule already refuses to suppress on an ambiguous identity. |
| 4 | `surfaceOp` and `sourceEventSeqs` rise to about 21% of rows | `surfaceOp` is `'append' \| {op: 'replace', startSeq, endSeq}`, required only on the four surface types (`types.d.ts:429-441`). `sourceEventSeqs` is individual non-negative integers plus inclusive `[start, end]` pairs, strictly increasing (`seq-ranges.js`, the same digest as the 0.1.2-rc.1 file). The rise is the writer placing every surface message on the surface with explicit markers. The writer's own contract: the surface "is the wrong source for a human transcript — a landed replacement would erase conversation the user already saw. Append-origin events are that transcript's durable source material; replacement copies stay model-only" (`surface.d.ts:27-33`). | No change to the citation rules; with no chunk rows they suppress nothing under version 3; `surfaceOp` is never read. 0055 ruling 3's "never rewrites audit history" is the writer's own transcript rule. The recorded-token cross-check gates `version` alone. | Whether a v3 citation can be non-earlier than its owning `seq` (S1 item 5): such a row refuses under the existing rule. |
| 5 | `system/message`, `todo/write`, `turn/end` in v3, not in version zero | `system/message` is the rendered system prompt as surface node 0, replacing the head (`types.d.ts:288-294`; the v2→v3 migration's `emitSystem`, `dsh-session-format-v2-to-v3/lib/index.js:780-810`), the successor of version zero's `request/context`, which is quiet. `todo/write` and `turn/end` are in both catalogues and log-only under v3. | All three quiet under version 3. `system/message` under version zero stays a required unknown: the 0.1.2-rc.1 catalogue does not contain it. A displayed `system` turn would be new command and TUI capability, not asked for. | Nothing the quiet disposition depends on: quiet reads no payload. |
| 6 | Is version 3 a superset of version zero? | **No.** Versions are physical generations joined by migrations, not sets: v3 removes `assistant/chunk` and, per the read, the two `tool/code-dispatch*` names; adds at least eight names; moves the prompt from `request/context` to `system/message` and the stream from chunk rows into the message. The four content kinds keep the fields the reader reads (`types.d.ts:309-338`). | Admit `3` beside `0` with a per-version vocabulary and one projector; no per-version projection. Every version-zero meaning is preserved for version-zero files; version-3 files carry the differences above, stated in the requirement as measurements. | Whether the message block vocabulary moved (S1 item 3). |

The eight names version 3 adds to what the reader handles are
`assistant/attempt`, `deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `system/message`,
`tool/ptc-dispatch` and `tool/ptc-dispatch-start`; the last two succeed
`tool/code-dispatch` and `tool/code-dispatch-start` through the v2→v3 PTC
migration. Of the eight, only `system/message` is surface-eligible, and it
maps to the `request/context` precedent; `assistant/attempt` is counted;
the other six are log-only and quiet.

### S4 — The prerequisite is discharged; corroboration follows the writer, never leads

The first sitting's prerequisite, a capture or a seat that can read the
install, was met by the simplicity seat's read (S1). What remains is
corroboration, and it is the controller's step on the host, not a seat's:
after landing, read the 17 version-3 sessions under `~/.dsh/sessions/brokkr`
through the built reader and confirm they project, then compare the row
types they carry against the 56-name vocabulary of S3. Two of S1's refused
items surface there if they bite: a session carrying an untranscribed
catalogue name or a packed storage row refuses loudly with its confirmed
path and a counted row. Corroboration can confirm an admission; it cannot
turn a refusal into one. Turning either refusal into an admission takes a
further read of `known-event-types.js` or of the persistence package, cited
by digest and line, through a change that names the row and its
disposition. No seat reads the 17 sessions, copies them or commits them
(decision 0032); every test in this change uses synthetic rows in
test-owned homes.

### S5 — Proposed decision 0060 carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes ("DSH numeric
on-disk version zero only"), so this change files a numbered decision with
`Status: proposed`, supplementing ruling 3 without editing it. 0060 is the
next free number: main's index holds 0055 and 0057–0059, and 0056 is
claimed by the #226 branch's proposed resumption decision. The decision is
written by the implementation phase beside its code, with its README row,
and rules:

1. **Admitted versions are measured, closed and named.** Zero on the
   0.1.2-rc.1 capture, three on the 0.1.5-rc.1 read with session packages
   0.1.5-rc.2 (the S1 digests). A version joins only through a change
   recording the writer's package identity and files with digests, the
   record vocabulary, how fragments persist, the meaning of every added
   header field, the meaning of `surfaceOp` and `sourceEventSeqs` where
   carried, a disposition for every type, and whether admitted meanings are
   preserved. A versioned filename, a header number and the resemblance of
   sampled rows are not evidence.
2. **Vocabulary is per version.** A row is classified under the vocabulary
   of the version that admitted its file; a shared name keeps a disposition
   in each version only by measurement; a name outside the admitted
   version's vocabulary is a required unknown. Version-zero storage rows
   and `assistant/chunk` are version-zero only.
3. **Version three persists whole messages.** No fragment rows; an
   interrupted step is one finalized `assistant/message` with
   `interrupted: true`; embedded streams and usage are quiet metadata;
   `assistant/attempt` is a counted omission. The reader assembles nothing
   under either version.
4. **The surface is not the transcript.** The citation grammar is shared;
   suppression remains cited-unique-earlier-same-step chunks only;
   `surfaceOp` is never read. The recorded-token cross-check gates
   `version` and no other field.
5. **`isSeeded` is inert** for ownership, depth and admission, whatever its
   JSON shape, including null and absent.
6. **Unmeasured parts refuse.** The persistence package's rows and any
   catalogue name not transcribed are refused under version three until
   read; the record says which.

Enforcement bindings: the version-matrix, disposition, fragment,
sibling-discovery and seeded tests named in the delta's scenarios,
`openspec validate --all --strict`, and the exact-coverage gate. 0032's
ownership, retention and privacy constraints are preserved.

### S6 — Reasoned refusals

- **No fall-through in discovery.** When the versioned name carries a
  refused version and the plain name beside it is readable, the read
  refuses with the versioned path. Falling through would present
  superseded evidence as the seat's newest transcript, the same reason the
  version-zero rule refuses to rank roots by version. The pairing this
  fleet produces, version 3 beside version zero, now reads the version-3
  file; the refusal case is pinned with a future version (design D8).
- **No `surfaceOp` model.** Nothing new gates suppression, and the writer
  itself calls the surface the wrong source for a human transcript.
  Importing it would replay the model's rewritten view into an audit read.
- **No displayed `system` turn.** `system/message` is the prompt copy that
  `request/context` was; showing it adds a role to the command and TUI
  that no requirement asks for. Quiet is the measured default.
- **No stream expansion and no attempt projection.** The stream element
  shape is uncited (S1 item 4); expanding it would duplicate the message
  or fabricate fragments. An attempt's text is counted, not silent, and a
  later change may project it once its shape is cited.
- **No union vocabulary and no per-version projector.** The union admits
  rows a writer cannot emit; a second projector duplicates a code path the
  measurement shows is shared (S2).
- **No migration and no versions 1 or 2.** Version-zero files read as
  before; no on-disk file of this fleet carries 1 or 2 and no seat read
  their codecs.
- **No `transcript-command` or `transcript-tui` delta.** Their observable
  behaviour for a readable or refused DSH source is unchanged; the TUI
  requirement's phrase "version-zero event/storage refusal SHALL keep
  R14's complete-prefix counts" stays true, and the reading requirement
  now states the same rule for every admitted version. Widening that
  phrase is wording, not behaviour, and is left to the fold.

### S7 — Validation of this sitting

Read the dialect through `openspec instructions proposal` and
`openspec instructions specs` for this change; they declare `proposal.md`
and `specs/**/*.md`, and no workflow runner was invoked. Adopted the
existing change and revised it in dependency order per design D11: the
proposal, then the capability delta, with each of the three living
requirements copied whole from `openspec/specs/transcript-reading/spec.md`
before editing. Reconfirmed this box cannot reach the writer (no `dsh`,
`volta`, `npm`, `cargo` or `~/.dsh`; HOME `/runtime/home`), so S1 item 1
stays refused rather than transcribed. Verified against the tree the
projector facts S1 lists. Strict OpenSpec validation was run over the
whole tree after authoring. Cargo is absent from this box, so no format,
clippy, test or bundle gate ran here; they belong to the implementation
seat before its activation commit, and the exact-coverage gate to host
validation outside the box. This sitting commits only the proposal and
the capability delta, unsigned, and pushes nothing.
