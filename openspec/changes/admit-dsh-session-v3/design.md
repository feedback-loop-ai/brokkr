## Context

See proposal.md — Why for the motivation. The facts that shape this design:

- **The reader at `d330765`.** Discovery in `crates/brokkr-cli/src/ui.rs`
  (`DSH_SESSION_FILES`, `:726`) tries `session.v3.jsonl` then
  `session.jsonl` and treats the first name whose opening row is a
  `session` object with depth zero or omitted as the directory's sole
  candidate; version is deliberately not consulted there. Content admission
  in `crates/brokkr-view/src/transcript.rs` (`dsh_header`, `:1915`) admits
  an exact numeric zero, cross-checking the raw `version` token
  (`zero_number_token`, `:1876`) against the parsed value. Row dispatch
  (`dsh_row`, `:2394`) is keyed on the `type` string alone: four content
  kinds, `assistant/chunk`, the three packed storage rows, the 46-name quiet
  list (`dsh_quiet_event`, `:1764`) from archived design D6 (#222), and a
  required-unknown rule for every other type. `project_dsh` (`:1970`) turns
  any `DshRow::Refused` into a whole-read `unsupported-format` that keeps
  `unrecognized_records`. Nothing in the projector knows which version
  admitted the file.
- **The change at `36bf374`.** The proposal and the capability delta
  specify a partial admission: the version-three header is admitted on the
  council's writer read; rows are classified under a per-version
  vocabulary; `tool/call` projects; `user/message`, `assistant/message`
  and `tool/result` are refused rows; `assistant/attempt` is a counted
  omission; 51 names are quiet; the change that lifts the payload refusal
  is named with three prerequisite reads. The clarify judge cleared that
  record on its third visit (strict validation 15/15). Proposal S8 lists
  the sections of the previous design (`c1d57e9`) it supersedes and
  assigns their revision to this revisit.
- **The recipe's return budget is spent.** `recipes/triage/policy.json`
  rule `DESIGN-UPSTREAM-EXHAUSTED` parks a run whose design reports
  `upstream` after three specification visits; specification has sat
  three times (`23fafd1`, `923d72b`, `36bf374`). This design therefore
  cannot return the change again. It revises itself to the specification,
  reports `drafted`, and says plainly where the specification's scope and
  the run's newest evidence now differ (D1, D2, D13) instead of editing the
  delta from below.
- **This sitting's box** is the specify seat's box: HOME `/runtime/home`,
  user `runner`, no `dsh`, `volta`, `npm`, `cargo` or `~/.dsh`;
  `/home/vyanakiev` holds only `source`. It cannot re-read the writer or
  run Cargo. OpenSpec 1.12.0 and Git are present.
- **Constraints carried unchanged:** frozen `contracts/`, `policy/`,
  `reference/`, `fixtures/`; the global `dsh` install and its sessions are
  read-only to every seat and never become test inputs (decision 0032); a
  semantic change carries a `Status: proposed` decision.

## Goals / Non-Goals

**Goals:**
- Design the specified partial admission so the implementation seat can
  build and pin it without interpretation: the admitted-version type, the
  exact-three token check, dispatch keyed on the admitted version and the
  kind, the payload-refusal arm, the second quiet list, and the tests that
  pin every disposition the delta names.
- Record this council's writer reads with digests, reach stated per seat
  and per operation, and what each read established and did not, so the
  change that lifts the payload refusal starts from a citable record.
- Reconcile both positions claim by claim on evidence, and keep every
  dependent artifact coherent with the clarified specification.

**Non-Goals:**
- No widening of the delta: version-three message payloads are not
  projected by this change (D1, D13 say why and what lifts them).
- No replay of `surfaceOp`, no reconstruction of the model-visible surface,
  no expansion of embedded streams into fragments, no assembled message the
  writer did not write.
- No `system` role, no CLI, TUI or protocol change beyond the observable
  shape the delta pins, no contract, schema, policy or fixture edit.
- No migration of version-zero files, no reading of the host sessions from
  a seat, no committed capture of the install, no re-measurement here.

## Decisions

### D1 — Reconcile the council by claim

| Position / claim | Resolution and evidence |
|---|---|
| Simplicity: the 0.1.5-rc.2 message and block definitions (`dsh-llm`) map field for field onto the reader's block parser; cut the payload refusal and the lifting change. | **Adopt the measurement; reject the lift in this change.** The read is recorded in D2 (digests) and D13 (the mapping) as the read the delta names for its item (a). It does not lift the refusal here, for three reasons. (i) The delta lifts only through "a change that records ... the version-three message and block definitions ... maps every consumed nested field ... and pins each with synthetic scenarios"; that is specification work, and a fourth specification visit parks the run (Context). (ii) The delta's lifting condition is three reads. This position discharges (a); it does not discharge (b), because it cites the codec's validation of replace markers and not what a replace-marked message carries or how an append-origin row is told from it for display, and it does not discharge (c), because its §6.9 accepts the seed-identity risk instead of measuring it. (iii) The read is run-local and one seat's; the delta's own standard is a read cited from the lifting change and re-verified from a box that reaches the source (D2's reach ruling). The cost is real and is stated where an operator will meet it (D10, Risks): every real version-three session refuses until the lifting change lands. |
| Simplicity: the catalogue is exactly 56 names; 51 − 3 + 8 reconciles; cut the untranscribed-name refusal. | **Adopt the count; the rule stands with no name left to refuse.** The delta closes the version-three vocabulary at the transcribed 56 names and refuses any other. With the count reconciled (D2 E1) that rule now bites only on a name a future writer adds, which is the safe direction and costs nothing. The previous design's "two unaccounted names" is withdrawn. |
| Simplicity: the version-three codec admits only `type`, `seq`, `time`, `data` at the top level; the timed stream is nested under `data.stream`. | **Adopt.** It turns the packed-row disposition from a safe default into a measurement: the 0.1.5-rc.2 codec cannot emit a top-level `text-chunks`, `reasoning-chunks` or `tool-call-chunks` row, so refusing one under version three refuses a row its writer cannot have written (D2 E3, D6). |
| Simplicity: `assistant/attempt` is quiet. | **Reject; counted omission stands.** The delta rules it, and the reason holds: an attempt carries model output the writer never surfaced. The reader's existing signal for "recognized envelope, content not projected" is the counted `DshRow::Omission` (`transcript.rs:2245`), and its count is the only trace an operator gets that model text exists which the read does not show. Quiet would erase that trace. Simplicity's §8 accepts this outcome. |
| Simplicity: `assistant/message` forbids `sourceEventSeqs`; the codec requires unique earlier source sequences and replacement endpoints below the event's `seq`. | **Adopt as the measurement of the previous design's item 5.** The reader's earlier-than-owning citation rule matches the writer's validator; under version three it reaches no row while the payload refusal stands (D7). |
| Robustness §1: reach is a property of an operation, not of a seat; a third shape exists (binary resolvable, source unreadable). | **Adopt.** D2's reach table gains the shape and this sitting's row, and the decision's ruling 1 says reach is performed by the reading seat at the file it cites and is never inherited (D11). |
| Robustness §2: the payload refusal renders the same string as a never-admitted version; surface the distinction. | **Adopt the type-level separation; reject a new explanation string in this change.** A distinct `DshRow` disposition keeps the cause separable in code and tests (D4, D10). The explanation string is pinned by the `transcript-reading`, `transcript-command` and `transcript-tui` requirements and by the delta's scenarios; the specified, rendered distinction is the count and its notice, which every surface shows (D10). |
| Robustness §3: D10's "Dispositions" bullet still says the four content kinds project; the tool-call-only synthetic session is a wiring proof and should say so. | **Adopt.** D12 names only `tool/call` as projecting and marks the tool-call-only test as a wiring proof in its name and comment. |
| Both positions: one projector, a vocabulary per version, no registry. | **Adopt** (D4). |
| Simplicity: the six answers live in `design.md` and the decision, with no separate evidence document. | **Adopt** (D5, D11). |

### D2 — Evidence: what the run read, from where, and what it did not reach

**Reach, per seat and per operation, on 2026-09-13.** The commission
admits one primary source, the installed `@deepseek-ai/dsh` 0.1.5-rc.1
writer, and forbids sampling as a substitute. Three box shapes occurred
across this run's seats:

| Seat | `which dsh` | Read the source tree | Box |
|---|---|---|---|
| specify, three sittings; design chief, `c1d57e9` and this sitting | not found | no | HOME `/runtime/home`, user `runner`; no `volta`, `npm`, `cargo` or `~/.dsh`; `/home/vyanakiev` holds only `source` |
| design robustness, first sitting | not found | no | the same |
| design robustness, this sitting | resolved `/home/vyanakiev/.volta/bin/dsh` | no: `test -r` on `dsh-session/lib/types/types.js` failed and `find` over the install root was refused | the third shape |
| design simplicity, both sittings | resolved; `dsh --version` printed `0.1.5-rc.1` | yes; digests and line citations taken; every access a read | the install root under `/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/`, packages at 0.1.5-rc.2 |

**Ruling.** Reach is an operation the reading seat performs at the moment
of the read, at the granularity of the file it cites. A resolved binary is
not evidence of source access; a seat's report of reach or of no reach is
not inherited by another seat or by a later sitting of the same seat. The
lifting change's prerequisite, "one read of the installed writer by a seat
whose box reaches it", names an operation to perform then, not a fact to
look up from this run's record.

**Digests the simplicity seat recorded.** The first six were taken at its
first sitting and re-verified byte-identical at this one; the last four
were taken at this sitting. Source files are under
`.../node_modules/@deepseek-ai/`.

| Source file | SHA-256 |
|---|---|
| `dsh-session/lib/types/known-event-types.js` | `7bcf6e061a34b6107896b09ec048f5e62420ce1505191c1d63910765e014d909` |
| `dsh-session/lib/types/types.js` | `27de3ecc17fe395856b6182362ecfbe7f7b83a90238bbd09ccfa55c007193725` |
| `dsh-session/lib/types/seq-ranges.js` | `68a127c76affa98edeeb50e302eb43f154f4d24f7d04cb95ba8b323e88f3d09e` |
| `dsh-session/lib/types/surface.js` | `aad7aaabe6cd9b39ae4cc3b50a2873c9b5d73b69d929051f31b18ecc13647c72` |
| `dsh-session-format-v2-to-v3/lib/index.js` | `2d35e1e0ed497af569d5735fc590187de1568489cfe60d070b5f61330cd5a338` |
| `dsh-session-format-catalog/lib/index.js` | `bf4bde9e6563d7793f820c4a1b3141f6527283dd6c58f16bf43a67bc557cc48c` |
| `dsh-session/lib/types/types.d.ts` | `54cf6f064abba4bfdca99eea4fb55d16629a91044c10a4d16940a5ffd11b74b6` |
| `dsh-llm/lib/types/types.d.ts` | `9268e1133511808731807ab31239fb0dd99895cc6ebab9dfa6e91b3f58d763f2` |
| `dsh-llm/lib/types/message.d.ts` | `110c2a4f9ac1e856ce04b1117530f7b1732dfd95a46528a2deb29379aed9a371` |
| `dsh-session-persistence-jsonl/lib/worker.cjs` | `b067a35a421d5a5313b1e197a00bcc829d8d6921d2829503d4604c13e226fd64` |

`seq-ranges.js` carries the digest the #222 capture recorded for the
0.1.2-rc.1 file: the citation grammar is byte-identical across the two
writers. The read is the run's primary evidence for the same reason the
#222 capture was version zero's: a run-local read of the installed writer,
never committed, cited from the change by file, line and digest; the
positions that could not reach the writer describe their boxes and are not
contradicted by it.

**What this council's reads established** (line citations are the
simplicity seat's, into the 0.1.5-rc.2 packages above):

- **E1 The catalogue is 56 names and reconciles.** `KNOWN_SESSION_EVENT_TYPES`
  (`known-event-types.js:21-78`) holds exactly 56 names; the reader handles
  51, three of which (`assistant/chunk`, `tool/code-dispatch`,
  `tool/code-dispatch-start`) are absent from version three and eight of
  which version three adds: 51 − 3 + 8 = 56. No name is untranscribed.
- **E2 The physical header.** The codec requires `type`, `version`, `id`,
  `createdAt`, `isSeeded` and `delegationDepth` and permits `cwd`,
  `parentSession`, `origin` and `agentPreset` (`worker.cjs:10461-10470`).
  The reader's admission shape stays deliberately smaller (`type`, depth,
  `version`) and every other field inert, absent or present.
- **E3 The envelope and the stream.** The version-three codec admits only
  `type`, `seq`, `time` and `data` at the top level (`worker.cjs:9824`).
  The timed stream is an array nested under `data.stream` on
  `assistant/message` and `assistant/attempt`, whose elements are the
  compact `{type: 'text-chunks' | 'reasoning-chunks' | 'tool-call-chunks'
  | 'chunk', time0, index, dt, ...}` objects of `AssistantStreamAccumulator`
  (`worker.cjs:4114-4200`). A top-level packed row cannot be written by
  this codec. The stream element shape the previous design left uncited is
  now cited; the stream is still never expanded (D6).
- **E4 Surface markers are validated by the writer.** `surfaceOp` is
  required on the four surface-eligible types and forbidden elsewhere;
  `assistant/message` forbids `sourceEventSeqs`; source sequences must be
  unique and earlier than the event, and replacement endpoints below its
  `seq` (`types.d.ts:429-446`, `worker.cjs:9824-9930`).
- **E5 The message and block definitions.** `Message` is `{id, role,
  content, source}` (`dsh-llm/lib/types/message.d.ts:120-153`); the block
  union is `text`, `reasoning`, `image`, `tool-call`, `tool-result` and
  `file` (`dsh-llm/lib/types/types.d.ts:39-98`). The field-by-field
  mapping onto the reader's parser is D13's table. This is the read the
  delta names for lifting item (a).
- **E6 Row admission by the codec.** The version-three codec refuses the
  two `tool/code-dispatch*` names and treats `assistant/chunk` as an
  opaque unknown envelope (`worker.cjs:10001`): the writer does not emit
  it, and a file carrying it is not a version-three file the writer wrote.
- **E7 Seeded consistency.** The codec enforces agreement between
  `isSeeded` and the `session/end-seed` marker (`worker.cjs:10057-10110`);
  the prefix length is `inheritedEventCount` session state
  (`types.d.ts:105-109`, `:139`).

**What remains unread**, so the delta's refusals stand on it:

- **U1 Replacement copies (delta item b).** E4 measures the codec's
  validation of a replace marker, not the append and replace writer paths:
  what a replace-marked surface row carries, whether it duplicates or
  summarizes the rows it replaces, and how an append-origin row is told
  from it for display. The disposition of a persisted replacement copy is
  the lifting change's to rule from that source.
- **U2 Inherited identities (delta item c).** Whether a seeded session's
  inherited prefix keeps its originating `seq`, `turn`, `step` and call
  identities, or remaps them, is not cited; E7 measures consistency of the
  marker, not identity of the rows.
- **U3 `surface.d.ts`** is cited by line but not by digest; `surface.js`
  from the same package version is.
- **U4 No scenario pins a version-three payload projection.** The delta
  carries none, by design; the lifting change writes them.

**Corroboration, after the writer.** The simplicity seat's type-only,
content-free histogram over the `session.v3.jsonl` files under
`~/.dsh/sessions/brokkr` (20 at its sitting, 17 when the commission was
written; the fleet keeps running seats) shows no top-level packed row and
no `assistant/chunk`, every header key set equal to E2's required set plus
`cwd`, and payload key sets matching E5. It changes no disposition and
proves no admission; the controller's corroboration step in proposal S4
stands.

### D3 — Admission: the set is `{0, 3}`, spellings are exact, exactness has a fixed scope

**Set.** Closed and exactly `{0, 3}`: zero by the #222 capture, three by
D2. Versions 1 and 2 are not admitted: no core of this fleet wrote them to
disk under an admitted name and no seat read their codecs. A version
outside the set refuses exactly as today.

**Type.** `enum DshVersion { Zero, Three }` in `transcript.rs`, private to
the projector. `dsh_header` returns `Option<DshVersion>` instead of `bool`;
`project_dsh` keeps the admitted version for the row loop. Alternative
rejected: an integer, because a `u8` invites arithmetic and a default; the
enum makes every match exhaustive so a third version cannot be added
without visiting every dispatch site.

**Spellings.** `zero_number_token` generalizes into an exact-integer token
check over a digit signature: strip the sign, split the mantissa from any
exponent, require mantissa characters in `0-9.` and exponent digits only,
then compare the mantissa's digits with the decimal point and leading and
trailing zeros removed against the signature, empty for zero and `3` for
three. Three is admitted when the parsed value is three and, when the raw
token is found, its signature is `3`. So `3`, `3.0`, `3e0`, `30e-1` and
`0.3e1` admit; `"3"`, `true`, `null`, `3.1`, `-3`, `3e1` (signature `3`,
parsed thirty) and the rounding artefacts `2.9999999999999999` and
`3.0000000000000001` (parsed three, signature not `3`) refuse. The zero
path keeps its rule unchanged, including `-0`; `dsh_millis_exact` keeps
calling the zero form. Alternative rejected: parsed value only for three,
because it admits a token the writer never wrote and drops a defence the
zero path already has (robustness §5 at the first sitting).

**Exactness scope.** The recorded-token check judges the header `version`,
an ordinary event's `time` and a packed row's `time0`, under either version,
and no other field. Admitting three widens the version check from one
admitted integer to two and adds no field: `surfaceOp`, citations and every
other event field are never judged on their token. The existing underflow
regressions (`tests.rs:2126`, `:2143`, `:2180`) stay, and a version-three
`tool/call` time scenario pins the scope (D12).

**Filename and header stay independent facts.** `session.jsonl` with a
version-three header reads under the version-three vocabulary;
`session.v3.jsonl` with a version-zero header reads under the version-zero
vocabulary; either name with version 4 refuses. Discovery infers nothing
from the header and admission nothing from the name.

### D4 — Dispatch keyed on the admitted version and the kind

`dsh_row(value, raw, version)` takes the admitted `DshVersion`; the row
loop in `project_dsh` passes it. Under `Zero` every arm is unchanged.
Under `Three`:

| Kind under a version-three header | Arm | Notes |
|---|---|---|
| `tool/call` | the existing arm | Projects `callId`, `name` and `arguments` carried on the event (D5 Q1); `turn` and `step` are read as today and reach nothing, because no non-dedicated version-three event exists to be owned. |
| `user/message`, `assistant/message`, `tool/result` | `DshRow::RefusedPayload`, returned **before** any payload, citation, time or marker read | The delta pins that the citations are neither applied nor reported, no block is projected, and the `ignorable` marker does not omit the row; returning first makes each of those true by construction and keeps `dsh_message_blocks` and `dsh_citations` version-zero-only code paths. `project_dsh` treats the variant exactly as `Refused`: `unrecognized += 1`, `refused = true`. |
| `assistant/attempt` | `DshRow::Omission` | Counted, no content, read available, whatever the payload or marker (D6). |
| the 51 quiet names | `DshRow::Quiet` | `dsh_quiet_event(kind, version)` matching on `Three` (below). |
| `assistant/chunk`, `text-chunks`, `reasoning-chunks`, `tool-call-chunks`, `tool/code-dispatch`, `tool/code-dispatch-start` | `DshRow::Unrecognized` | The ignorable rule applies as to any unknown; the row is never decoded by the fragment or packed path (E3, E6). |
| any other name | `DshRow::Unrecognized` | Including a later `session` row, unchanged. |

Under `Zero` the eight version-three names (`assistant/attempt`,
`deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `system/message`,
`tool/ptc-dispatch`, `tool/ptc-dispatch-start`) stay `Unrecognized`, as
they are today.

**The quiet lists.** `dsh_quiet_event(kind, version)` holds two complete
lists, one per arm of the version: the version-zero list is the existing
46 names, untouched; the version-three list is written out in full, 51
names, the 44 shared names plus `system/message` and the six log-only
names. A test asserts the set relation between the two lists (version
three equals version zero minus `tool/code-dispatch` and
`tool/code-dispatch-start` plus the seven), so an edit to one list that
silently changes the other's meaning fails. Alternatives rejected: a union
list (admits `assistant/chunk` and packed rows under a writer that cannot
emit them and quiet-lists version-three names under a writer that never
emitted them); a shared core plus per-version extras (hides the
per-version enumeration the requirement asks for and lets a name join both
versions by editing one place); a per-version projector or registry
(nothing else differs between the versions the reader projects, and the
shared `tool/call` arm is one code path by measurement).

**Association.** `associate_dsh_tools` (`:2142`) runs as today. Under
version three it has nothing to do by construction: every projected
version-three event is a dedicated `tool/call`, and the rule only ever
removes embedded copies from non-dedicated events. No version guard is
added: it would be dead code that no scenario can distinguish, and the
exact-coverage gate would still demand a test for it. The lifting change,
which is the first to create a non-dedicated version-three event, adds the
seed-path ruling (U2) before any such event exists (D13).

**Refusing at the row, not at the header.** Refusing a version-three file
at its header, as an unadmitted version is refused, was rejected: it would
return zero counts and no notice, erasing the one specified signal that
distinguishes "admitted, payload refused" from "never admitted" (D10), and
it would refuse the tool-call-only and quiet-only files the delta says
project.

### D5 — The six questions of #279, answered on the reads

Each row gives the measured answer with its evidence, the reader
consequence under this change, and what stays unmeasured and therefore
refused. Evidence labels are D2's.

| # | Question | Measured answer and evidence | Reader consequence | Unmeasured, therefore refused |
|---|---|---|---|---|
| 1 | Record types and shapes the v3 writer emits | `SESSION_FORMAT_VERSION = 3` (`types.js:54`); catalogue `currentVersion: 3`, codecs `v0..v3`, lossless migrations `v0→v1→v2→v3` (`dsh-session-format-catalog/lib/index.js:33-46`); the vocabulary is the 56-name generated set (E1); payload envelopes are `SessionEventMap` (`types.d.ts:309-338`): content in `data` for `user/message`, `data.message` for `assistant/message` and `tool/result`, `callId`, `name` and `arguments` on `tool/call`; the physical envelope is `type`, `seq`, `time`, `data` (E3); surface-eligible types are exactly `system/message`, `user/message`, `assistant/message`, `tool/result` (`types.d.ts:413`), every other type log-only (`surface.d.ts:52-64`). | Per-version vocabulary (D4); dispositions in D8; `tool/call` projects on its event-map fields. | The payload definitions are read (E5) but not admitted by this change's delta: the three message-carrying kinds refuse row by row until the lifting change pins them (D1, D13). |
| 2 | `assistant/chunk` absent from v3 | Removed, not renamed and not conditional. It is outside the 56-name catalogue (E1); the codec treats it as opaque unknown (E6) and cannot write a top-level packed row (E3). The v1→v2 package is the assistant-stream migration; in v3 the timed stream is embedded: `assistant/message` carries `message`, `stream`, optional `usage` and `interrupted?: true` (`types.d.ts:309-317`); `assistant/attempt` carries a stream with no surface message (`types.d.ts:323-327`); "a turn cancelled mid-stream finalizes its delivered text/reasoning prefix as this event with `interrupted: true`" (`types.d.ts:303-307`). | Whole messages only under version three; the fragment rule's text stands and has nothing to concatenate; the reader assembles nothing under either version. `stream`, `usage` and `interrupted` are never expanded under any admission of the row. `assistant/attempt` is a counted omission. `assistant/chunk` and the packed rows are required unknowns under version three (D6). The finalized message is not displayed by this change (Q1). | Nothing for the disposition; the interrupted-prefix behaviour is recorded as a measurement and pinned as a refusal today, to become a projected turn when the lifting change lands. |
| 3 | `isSeeded` new in the header | A required v3 header boolean (E2) meaning the session contains a fork-inherited event prefix (`types.d.ts:72-76`); the prefix length is `inheritedEventCount` session state (`types.d.ts:105-109`, `:139`), not header metadata; the cut is the `session/end-seed` marker and the codec enforces their agreement (E7); delegation depth is a separate field (`types.d.ts:82-87`). Ownership is `type: session` plus depth, and the reader's header admission reads no other field. | Inert for ownership, depth and admission; pinned over true, false, string, object, null and absent under both versions; no provenance or delegation claim rests on it; it changes no row's classification. | Whether inherited rows keep their originating `seq`, `turn`, `step` and call identities (U2). The dedicated-tool association keys on call id and turn/step independently of chunks, so association is not applied under version three until the seed path is read; today the payload refusal leaves nothing to associate (D7). |
| 4 | `surfaceOp` and `sourceEventSeqs` rise to about 21% of rows | `surfaceOp` is `'append' \| {op: 'replace', startSeq, endSeq}`, required on the four surface types and forbidden elsewhere; `assistant/message` forbids `sourceEventSeqs`; the codec validates unique earlier sources and replacement endpoints below the event's `seq` (E4). `sourceEventSeqs` is individual non-negative integers plus inclusive `[start, end]` pairs, strictly increasing (`seq-ranges.js`, byte-identical to 0.1.2-rc.1): the grammar `dsh_citations` (`:2342`) accepts. The rise is the writer placing every surface message on the surface with an explicit marker. The writer's contract: the surface "is the wrong source for a human transcript — a landed replacement would erase conversation the user already saw. Append-origin events are that transcript's durable source material; replacement copies stay model-only" (`surface.d.ts:27-33`). | The reader never removes, reorders or replaces a row on `surfaceOp` and never replays the surface: measured, and 0055 ruling 3's audit rule. Keeping earlier rows is measured; excluding model-only copies is a separate operation the reader does not claim. It neither displays nor suppresses a version-three row on the marker; a replacement copy today is a refused message row or a quiet `system/message`. Citation validation is unchanged in grammar and reaches no version-three row (D7). | What a persisted replacement copy carries and how an append-origin row is told from it (U1). |
| 5 | `system/message`, `todo/write`, `turn/end` in v3 and not in version zero | `system/message` is the rendered system prompt as surface node 0 (`types.d.ts:288-298`; the v2→v3 migration's `emitSystem`, `dsh-session-format-v2-to-v3/lib/index.js:780-810`), the successor of version zero's `request/context`, which is quiet (`transcript.rs:1787`). `todo/write` and `turn/end` are in both catalogues, quiet today (`:1803`, `:1810`), and log-only under v3. | All three quiet under version three, whatever their payload or `surfaceOp`; `system/message` under version zero stays a required unknown because the 0.1.2-rc.1 catalogue does not contain it. A displayed `system` turn would be new command and TUI capability, not asked for. | Nothing the quiet disposition depends on: quiet reads no payload. |
| 6 | Is version 3 a superset of version zero? | **No, as a vocabulary; yes, as an envelope for the four content kinds; not yet admitted as a payload.** Versions are physical generations joined by migrations: v3 removes `assistant/chunk` and the two `tool/code-dispatch*` names, adds eight names, moves the prompt from `request/context` to `system/message` and the stream from chunk rows into the message (E1, E3, E6). The event map keeps the envelope nesting the reader reads for the four content kinds (Q1), and E5 reads the block definitions as field-compatible with the version-zero parser (D13). | Admit `3` beside `0` with a per-version vocabulary and one projector; this change projects `tool/call` and refuses the three message kinds; every version-zero meaning is preserved for version-zero files. | Whether the field-compatible payload projection is *specified*: the delta does not carry it, and the lifting change adds it with scenarios (U4) after U1 and U2 are read; if those reads show a meaning moved, that change adds a per-version payload projection rather than widening this one. |

The eight names version three adds to what the reader handles are
`assistant/attempt`, `deliverables/presented`, `feedback/message-delete`,
`feedback/message-put`, `subagent/catalog`, `system/message`,
`tool/ptc-dispatch` and `tool/ptc-dispatch-start`; the last two succeed
`tool/code-dispatch` and `tool/code-dispatch-start` through the v2→v3 PTC
migration. Of the eight, only `system/message` is surface-eligible, and it
maps to the `request/context` precedent; `assistant/attempt` is counted;
the other six are log-only and quiet.

### D6 — Fragments under version three: the writer finalizes, the reader never assembles

Under version zero a step's text arrives as `assistant/chunk` rows or
packed runs, each one logical event, and a readable assembly that cites
them suppresses them. Under version three the writer persists no fragment
rows: a completed step is one `assistant/message`, and a step cancelled
mid-stream is one `assistant/message` with `interrupted: true` whose
`message` is the delivered prefix the writer finalized (D5 Q2). Nothing is
concatenated, because nothing arrives in pieces. The rulings the delta
carries and this design implements:

- The sentence "fragments SHALL not be concatenated into an invented
  assembled message" is unchanged; under version three it has nothing to
  concatenate. The scenario "An interrupted DSH step retains its chunks"
  is a version-zero scenario.
- `interrupted`, `stream` and `usage` are never expanded, duplicated as
  turns or rendered as prose under any admission of the row. The stream
  element shape is now cited (E3); expansion is still rejected, because it
  would duplicate the message the writer finalized or fabricate fragments
  the writer chose not to persist as rows.
- Under this change the finalized `assistant/message` is a refused row
  (D4), so no interrupted prefix is displayed yet; the delta's refusal
  scenario covers the `interrupted`, `stream` and `usage` shapes so the
  lifting change inherits the pins and turns them into projection.
- `assistant/attempt` is `DshRow::Omission`: recognized, no content,
  `unrecognized_records` plus one, read available, with or without an
  `ignorable` marker. The text streamed in a failed or retried attempt is
  therefore not part of the version-three audit read, a measured difference
  from version zero, where such deltas were chunk rows. Alternatives
  rejected: quiet (drops model output silently, D1); displayed as
  fragments (the writer marks the attempt non-surface and the reader
  invents nothing).
- `assistant/chunk` and the three packed rows under version three are
  required unknowns, refused unless `ignorable: true` and never decoded by
  the fragment or packed path. They are version-zero evidence, and the
  version-three codec cannot write them (E3, E6).

### D7 — `surfaceOp`, citations and seeded association under version three

**Measured.** The citation grammar is shared byte for byte; the writer
validates every marker and citation before persisting it (E4); the
writer's own contract calls the surface the wrong source for a human
transcript (D5 Q4). `dsh_citations` therefore applies unchanged wherever
it is reached, and it is reached by no version-three row while the payload
refusal stands, because the refusal arm returns before the citation read
(D4). `surfaceOp` is never parsed under either version.

**Unread, so not claimed.** Whether a persisted replacement copy should
display, be omitted or refuse is U1; whether inherited identities are
association evidence is U2. The reader keeps every earlier row (measured,
the audit rule) and neither displays nor suppresses a version-three row on
the marker; under version three a replacement copy can reach it only as a
refused message row or a quiet `system/message`, and no association is
applied because no non-dedicated version-three event exists (D4).

**Alternatives rejected.** A `surfaceOp`-aware suppression model imports
the model's rewritten view into an audit read, which the writer itself
calls the wrong source. Applying the dedicated-tool association under
version three on the version-zero identity assumptions would rest on U2;
simplicity's §6.9 accepts a cosmetic duplicate in that case, but the
delta rules the association unapplied until the seed path is read, and
with the payload refusal the two positions produce the same behaviour
today.

### D8 — Dispositions under version three, enumerated from evidence

"Recognized quiet event kinds SHALL be enumerated per admitted version
from evidence in design." This is that enumeration for version three;
version zero's is unchanged from archived D6 (#222).

| Type under a version-three header | Disposition | Evidence |
|---|---|---|
| `tool/call` | Content, projected through the existing arm | Fields carried on the event (`types.d.ts:333`); E3 |
| `user/message`, `assistant/message`, `tool/result` | Refused row (`DshRow::RefusedPayload`): `unsupported-format`, one unrecognized count, no `ignorable` bypass | Envelope cited (`types.d.ts:281`, `:309-317`, `:351`); payload projection not specified by this change (D1, U4) |
| `assistant/attempt` | Counted omission | Stream with no surface message (`types.d.ts:323-327`); D6 |
| `system/message` | Quiet | Rendered system prompt (`types.d.ts:288-298`; v2→v3 `emitSystem`); `request/context` precedent |
| `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch`, `tool/ptc-dispatch-start` | Quiet | In the 56-name catalogue (E1) and outside the four surface types; `tool/ptc-dispatch*` succeed `tool/code-dispatch*` (v2→v3 PTC migration) |
| The 44 version-zero quiet names other than `tool/code-dispatch` and `tool/code-dispatch-start` | Quiet | Shared by both catalogues (E1); log-only under v3; quiet reads no payload |
| `assistant/chunk`, `text-chunks`, `reasoning-chunks`, `tool-call-chunks`, `tool/code-dispatch`, `tool/code-dispatch-start` | Required unknown (refuse unless `ignorable: true`) | Absent from the v3 catalogue (E1); cannot be written by the v3 codec (E3, E6) |
| A later `session` row | Unknown envelope under the ignorable rule | Unchanged from version zero |
| Any other name | Required unknown | The vocabulary is closed at 56 (E1); the safe direction for a future writer's addition |

The version-three quiet set is therefore 51 names: the 44 shared names plus
`system/message` plus the six log-only names of row five. It is encoded as
the second complete list of D4.

### D9 — The exclusive candidate stays exclusive, and the mismatched case is pinned

Discovery keeps the #278 rule: the first admitted name whose opening row is
a session header with valid depth is the directory's only candidate, and
content admission then rules on that file alone. The pairing the fleet
produces, `session.v3.jsonl` at version 3 beside `session.jsonl` at
version zero, reads the version-three file and never the plain name's
content. The gap the first robustness sitting found remains for a future
version: `session.v3.jsonl` at version 4 beside a readable `session.jsonl`
refuses with the confirmed `session.v3.jsonl` path and does not fall
through. Falling through would present superseded evidence as the seat's
newest transcript, the same reason the version-zero rule refuses to rank
roots by version. The delta pins both cases and the filename/version
independence pair; the tests live beside the existing both-names test in
`crates/brokkr-cli/src/ui/tests.rs` (`:3666`), which today writes the same
header into both files and so proves ordering but not this. No change to
`ui.rs`.

### D10 — The refused read's observable shape, and keeping its cause separable

**What the operator sees.** A version-three session carrying a message
row returns `unsupported-format` with the pinned explanation
`DSH transcript format is not supported`, its confirmed path and shared
DSH hint, no turns, `skipped_lines` as counted, `unrecognized_records`
equal to the number of refused rows plus any counted omissions, and the
notice `unrecognized transcript records: N`. CLI text renders the notices
beside the explanation (`crates/brokkr-cli/src/render.rs:830`;
`transcript-command` scenarios at `:227` and `:312`), CLI JSON carries the
counts and `notices`, and the TUI keeps the notices in its pane and error
overlay (`transcript-tui` scenarios at `:158` and `:240`). A never-admitted
version returns the same explanation with zero counts and no notice
(delta, "A rejected header version"). The two states are therefore
distinguishable at every surface by the count and its notice, which is the
distinction the delta specifies.

**Type-level separation (robustness §2, adopted).** The payload refusal is
its own `DshRow` variant, `RefusedPayload`, distinct from the `Refused`
that version zero reaches from an invalid citation (`:2414`, `:2476`) or
an invalid packed row (`:2588` onward). `project_dsh` maps both to the
same outcome. The variant costs one
arm; it keeps the two causes separable in code and in the tests that reach
them, it names the arm for its cause so the lifting change lifts it by
deleting one variant rather than by untangling a shared one, and it keeps
an extension point open for a distinct string should a later change
specify one.

**A new explanation string is rejected for this change.** The string is
pinned by the living `transcript-reading`, `transcript-command` and
`transcript-tui` requirements and by every refusal scenario in the delta;
widening it is a specification change across two capabilities this change
does not touch and this design cannot open. The specified distinction
(count and notice) is terse but present; the lifting change removes the
state altogether, which is the better fix. The wording gap is recorded
under Risks, not left implicit.

### D11 — A proposed decision carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes ("DSH
numeric on-disk version zero only"), so the implementation seat files a
numbered decision with `Status: proposed`, supplementing ruling 3 without
editing it, with its row in `docs/decisions/README.md`. **Number:** the
next free number when the decision is written, re-read from the index at
the PR; at this sitting main's index holds 0055 and 0057 through 0059,
0056 is claimed by the #226 branch and 0060 by a `review-first` run's
journal anchor, so neither is taken. Its rulings, as proposal S5 states
them, with the reach clause D2 adds:

1. **Admitted versions are measured, closed and named.** Zero on the
   0.1.2-rc.1 capture (#222), three on the 0.1.5-rc.1 read with session
   packages 0.1.5-rc.2 (D2 digests). A version joins only through a change
   recording the writer's package identity and files with digests, the
   record vocabulary, the message and block definitions behind every
   projected payload mapped field by field, how fragments persist, the
   meaning of every added header field, the meaning of `surfaceOp` and
   `sourceEventSeqs` where carried including whether replacement copies
   are persisted as rows, the identity of a seeded session's inherited
   rows, a disposition for every type, and whether admitted meanings are
   preserved. Reach to the writer's source is an operation the reading
   seat performs at the file it cites, never inherited from another seat
   or sitting; a resolved binary is not source access. A versioned
   filename, a header number and the resemblance of sampled rows are not
   evidence.
2. **Vocabulary is per version.** A row is classified under the vocabulary
   of the version that admitted its file; a shared name keeps a disposition
   in each version only by measurement; a name outside the admitted
   version's vocabulary is a required unknown. Version-zero storage rows
   and `assistant/chunk` are version-zero only.
3. **A payload projects only on its own writer's definitions.** Under
   version three `tool/call` projects from its event-map fields;
   `user/message`, `assistant/message` and `tool/result` are refused rows,
   counting once and refusing the read regardless of an `ignorable`
   marker, until a change records the version-three message and block
   definitions, maps each consumed field, and pins each with scenarios. A
   familiar block name is not evidence.
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
8. **Unmeasured parts refuse.** Any catalogue name outside the transcribed
   56, the version-zero storage rows and the three payloads above are
   refused under version three until read; the record says which read.

Enforcement bindings: the tests of D12, `openspec validate --all --strict`,
and the exact-coverage gate. 0032's ownership, retention and privacy
constraints are preserved: no real session is read by a seat, copied or
committed; tests use synthetic rows in test-owned homes.

### D12 — Proof, host-independent, and the gates

All tests use inline synthetic rows in test-owned homes; nothing under
`~/.dsh` or `fixtures/` is read. Names are suggestions; each pins the
delta scenario it is named after. In `crates/brokkr-view/src/transcript/tests.rs`:

- **Version matrix.** `dsh_header_version_matrix_admits_only_numeric_zero`
  (`:1019`) becomes the two-version matrix: admitted `0`, `0.0`, `0e0`,
  `-0`, `3`, `3.0`, `3e0`, `30e-1`, `0.3e1`; refused `null`, `false`,
  `"0"`, `"3"`, `[]`, `{}`, `1`, `2`, `4`, `-1`, `-3`, `0.5`, `3.1`, `3e1`,
  `2.9999999999999999`, `3.0000000000000001` and an absent version; every
  refusal keeps zero counts and the confirmed path.
- **Quiet-list relation.** A test over the two lists asserting version
  three equals version zero minus the two `tool/code-dispatch*` names plus
  the seven quiet version-three names, and that neither list holds
  `assistant/attempt`, `assistant/chunk` or a packed tag.
- **Dispositions.** Under a version-three header: the ten names of the
  delta's "Version-3 names are ruled under the version-3 vocabulary"
  scenario are quiet beside one projecting `tool/call`, with
  `system/message` supplying no turn whatever its payload or `surfaceOp`;
  the six version-zero-only names and one name outside the catalogue are
  required unknowns with and without `ignorable: true`. Under a
  version-zero header: each of the seven quiet version-three names and
  `assistant/attempt` is a required unknown, and `system/message` stays one
  ("Version zero already rules the three sampled types").
- **Payload refusal.** `dsh_v3_message_rows_refuse_until_their_definitions_are_read`:
  the five files of the delta's refusal scenario (string-content
  `user/message`; `assistant/message` with familiar blocks,
  `interrupted`, `stream`, `usage` and a partial append; `assistant/message`
  with a replace marker and citations; `tool/result` naming the call's id
  with matching turn/step; `user/message` with `ignorable: true`), each
  returning `unsupported-format`, `unrecognized_records: 1`,
  `skipped_lines: 0`, `truncated: false` and exactly one notice, with no
  turn; the same four rows under a version-zero header project as today.
- **Wiring proof.** `dsh_v3_tool_call_alone_projects_as_a_wiring_proof`:
  a header and one `tool/call` project the call with zero counts. Its
  comment says it is a minimal proof that the version-three dispatch path
  reaches the `tool/call` arm, not a claim that any real version-three
  session reaches the successful path today (robustness §3).
- **Attempt.** `tool/call`, `assistant/attempt` with a stream, `tool/call`:
  two calls in order, one count, one notice, read available; the same with
  `ignorable: true` on the attempt.
- **Fragment rows under version three.** `assistant/chunk` and each of the
  three packed rows after a `tool/call` refuse with one count; with
  `ignorable: true` each projects the call with one count.
- **Time exactness.** A `3e0` header with `tool/call` rows at `1e-400`,
  `1e3` and `-0.0` projects stamps `""`, `"1000"`, `"0"`; the existing
  underflow regressions (`:2143`, `:2180`) stay.
- **Seeded.** `isSeeded` true, false, `"yes"`, `{}`, null and absent under
  both versions admit and change no classification; version 3 with
  `isSeeded: true` and depth 1 is not a candidate.
- **Version-zero regressions.** Every existing DSH test unchanged and
  green.

In `crates/brokkr-cli/src/ui/tests.rs`: the delta's two sibling scenarios
(version 3 beside version zero reads the versioned name; version 4 beside
version zero refuses with the versioned path, zero counts, no turns) and
the filename/version independence triple (`session.jsonl` at version 3
projects; `session.v3.jsonl` at version 0 projects its message;
`session.v3.jsonl` at version 4 refuses). In
`crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
delta's located scenario end to end, one root projecting its call through
CLI text, CLI JSON and the TUI with `--turn 1` reaching it, and a second
root refusing with `unrecognized_records: 3`, the notice, and both TUI
doors disabled.

Gates before the activation commit: `openspec validate --all --strict`,
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings`, `cargo test --workspace
--all-features --locked`, both bundle compiles. Exact coverage runs
outside the box and is recorded pending until its result exists; the gate
is not lowered. Remote CI, host corroboration, publication and closure are
the controller's.

### D13 — What the lifting change starts from

The change that lifts the payload refusal is a further OpenSpec change
against the same requirement (proposal S4). This record gives it a
starting point it must re-verify, not a licence to skip the reads.

**Read (a), performed this sitting by the simplicity seat, to be
re-verified against the D2 digests from a box that reaches the source.**
The 0.1.5-rc.2 `SessionEventMap` payloads and block union against the
reader's parser (`dsh_row`, `:2394`; `dsh_message_blocks`, `:2278`):

| Writer definition | Reader read today | Finding |
|---|---|---|
| `user/message.data` is the `UserMessage` itself: `{id, role, content, source}`, no `turn` or `step` (`types.d.ts:281`; `message.d.ts:120-153`) | `data.content`; `data.turn`, `data.step` | Field-compatible; positions are `None` under version three, so a user message can never own or be owned in association |
| `assistant/message.data` is `{turn, step, message, stream, usage?, interrupted?}` (`types.d.ts:309-317`) | `data.message.content`, `data.turn`, `data.step` | Field-compatible; `stream`, `usage`, `interrupted` unread by design |
| `tool/result.data` is `{turn, step, message, error?, meta?}` (`types.d.ts:351`) | `data.message.content` | Field-compatible; `error` and `meta` unread |
| `text.text` (`dsh-llm/lib/types/types.d.ts:39-98`) | `text` | Compatible |
| `reasoning.text` | `text` | Compatible |
| `image.attachment` | none; `[image omitted]` | Compatible |
| `tool-call.id`, `.name`, `.arguments` | `id`, `name`, `arguments` | Compatible |
| `tool-result.toolCallId`, `.content` | `toolCallId`, `content` or `text` | Compatible |
| `file` (`types.d.ts:66-70`) | not named | A counted unrecognized block that keeps its siblings under the existing rule; the lifting change rules whether it stays counted or is shown as an omission marker |
| `Message.id`, `.source` | not read | Inert |

**Reads (b) and (c), still to perform:** the append and replace writer
paths and the disposition of a persisted replacement copy (U1); the seed
and fork path and inherited identities (U2). Each cited by package, file,
digest and line.

**Scenarios to write:** the four content kinds projecting under version
three with ordered blocks; an interrupted `assistant/message` projecting
one turn with no marker; a `file` block's count; an append-plus-replacement
sequence and a compaction pinning displayed rows, order, duplicates and
diagnostics; seeded scenarios in which inherited and new embedded and
dedicated calls and results suppress a genuine duplicate once and keep
unrelated or ambiguous identities, or the association left unapplied.

**Code seam:** delete `DshRow::RefusedPayload`, route the three kinds
through the existing arms under `Three`, and add the association ruling
from (c) before any non-dedicated version-three event exists. Nothing
else in this design moves.

## Risks / Trade-offs

- [Every real version-three session on the host carries a message row and
  will refuse under this change, so the admission is invisible at the one
  surface an operator looks at except through the count and its notice] →
  Stated here, in D10 and in the proposal's S4, not discovered later. The
  refusal is specified, tested and distinguishable from a never-admitted
  version by `unrecognized_records` and `unrecognized transcript
  records: N` at every surface. The lifting change removes the state; D13
  gives it read (a) and names (b) and (c), so its prerequisite is two reads
  and a set of scenarios, not a fresh investigation.
- [The explanation string does not name the cause] → Pinned across three
  requirements; not this change's to widen. `DshRow::RefusedPayload` keeps
  the cause separable so a later change can give it a string without
  re-deriving the classification.
- [The writer reads are one seat's, run-local, and this box cannot verify
  them] → Cited by digest and line (D2), checked against the repository
  wherever they touch the reader, and named as re-verifiable: the lifting
  change re-takes the digests from a reaching box (D2 ruling). A
  corroboration step on the host is the controller's and cannot turn a
  refusal into an admission.
- [The council's newest evidence exceeds the specification's scope, and the
  recipe forbids another return] → The design does not silently widen the
  delta and does not silently drop the evidence: D1 says which claims are
  adopted as measurement and which are rejected as scope, and D13 hands the
  measurement forward. The operator can open the lifting change on this
  record immediately.
- [Text streamed in failed attempts disappears from the version-three
  read] → Counted, not silent; stated in the delta and the decision; a
  later change may project attempt streams now that their element shape is
  cited (E3).
- [Per-version dispatch adds an enum through the row loop and a second
  quiet list] → One argument, one list and one variant; the set-relation
  test keeps the two lists honest; the alternative admits version-zero-only
  rows under a writer that cannot emit them.
- [A second admitted version widens the surface for crafted files] →
  Admission still requires ownership, bounded UTF-8 acquisition and an
  exact header; the refusal arm returns before any payload, citation or
  marker read, so a version-three message row is parsed less, not more,
  than a version-zero one; no new parser.
- [The decision number is contested by two other branches] → Re-read at
  the PR; the file and its README row are written by the implementation
  seat, not reserved here.

## Migration Plan

1. Tasks break D3, D4, D8, D9, D10, D11 and D12 into work in dependency
   order: the version type and token check; dispatch and the refusal
   variant; the second quiet list and its relation test; the projector
   tests; the discovery tests; the CLI and TUI end-to-end tests; the
   decision and its README row; guides only where a line says "version
   zero only" (none found at this sitting).
2. Implementation in `crates/brokkr-view/src/transcript.rs` and the four
   test suites; no `ui.rs`, protocol, contract, policy, reference or
   fixture change.
3. Gates in the box (D12); exact coverage outside it, recorded pending;
   activation commit unsigned; nothing pushed.
4. Controller: remote CI on the final head, host corroboration against the
   version-three sessions (reading, never committing), review, landing,
   closure of #279 with the lifting change opened on D13.

Rollback: revert the activation commit. Version-zero behaviour is untouched
by construction and version three returns to the header refusal. No data
migration exists to undo.

## Open Questions

- Whether `deliverables/presented` carries operator-facing content that a
  later change should display. Quiet is the measured default (non-surface);
  displaying it would be a new capability, not a change to this design.
- Whether the `file` block, once payloads project, is shown as an omission
  marker like `image` or stays a counted unrecognized block. The lifting
  change rules it; no consequence here.

## Validation of this sitting

Read the dialect through `openspec instructions design --change
admit-dsh-session-v3 --json`; it declares only `design.md`, and no workflow
runner was invoked. Read the proposal, the capability delta and both
council positions completely, and reconciled every material claim in D1.
Confirmed from `recipes/triage/policy.json` that `upstream` from design
parks after three specification visits, which have been spent, so this
sitting revises the design to the clarified specification and reports
`drafted`. Reconfirmed this box cannot reach the writer (no `dsh`,
`volta`, `npm`, `cargo` or `~/.dsh`; HOME `/runtime/home`). Verified
against the tree every reader fact this design cites: `dsh_quiet_event`'s
46 names, `dsh_row`'s arms and the two citation-grammar `Refused` returns,
`project_dsh`'s header refusal with zero counts and its whole-read refusal
keeping the counts, `associate_dsh_tools`' keys, `dsh_message_blocks`'
reads, `notices` and `explanation_for`, the discovery constant and the
both-names test, and the string and notice pins in the `transcript-command`
and `transcript-tui` requirements. Strict OpenSpec validation was run over
the whole tree after authoring. This sitting writes and commits only
`design.md`, unsigned; proposal, capability delta, code, decisions and
frozen bytes are untouched. The result is `drafted` with `inputs.change:
admit-dsh-session-v3`.
