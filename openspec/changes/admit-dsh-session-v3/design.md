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
  `unrecognized_records`, then runs `associate_dsh_tools` (`:2142`) over
  every projected event. Nothing in the projector knows which version
  admitted the file or what its header's `isSeeded` says.
- **The change at the fourth specification sitting.** The proposal and the
  two capability deltas specify a full admission of version three on this
  council's writer reads: the header is admitted; rows are classified under
  a per-version vocabulary; the four content kinds project on the
  0.1.5-rc.2 message and block definitions (D2 E5, D13);
  `assistant/attempt` is a counted omission; 51 names are quiet;
  `surfaceOp` is never parsed and a replace-marked row projects by its
  type; the dedicated-tool association runs under version three only when
  the header records `isSeeded: false`, because the seed and fork path is
  the one unread path a rule rests on (D2 U2). The analyze judge returned
  the previous specification to its author because its payload refusal
  rested on the claim that the definitions were not read, which D2 E5
  refutes; that sitting lifted the refusal and revised this design to
  match, so the council's next sitting starts from a design that agrees
  with its delta.
- **The change at the fifth specification sitting.** The clarify judge
  returned four findings against the record itself (proposal S8,
  CLARIFY-279-5 to 8): E3 and E4 contradicted each other on the envelope;
  D4, D13 and task 2.1 inferred that an unchanged message arm reads no
  user positions, which `transcript.rs:2427-2428` refutes; D3 and tasks
  1.2 and 1.4 promised the shipped zero behaviour byte for byte, which
  `zero_number_token` (`:1876-1895`, a zero digit somewhere in the
  mantissa, not everywhere) makes incompatible with the exact-zero
  requirement; and D3's "when the raw token is found" left escaped and
  duplicate `version` members to the parsed value. The specify seat
  revised D1, D2, D3, D4, D5, D12, D13, the Risks and the Migration Plan
  to the corrected delta; each correction is recorded as a correction and
  nothing was re-measured.
- **The change at the sixth specification sitting.** The clarify judge
  returned four further record defects (proposal S8: CLARIFY-279-5 on its
  second visit, and CLARIFY-279-9 to 11): E4 presented an inferred
  `sourceEventSeqs` permission as measured and left the other 52 types
  unrecorded; S5 ruling 7 generalised the digit signature to every integer
  timestamp, which read literally invalidates `1e3` and `1000.0`; the
  helper algorithm of D3 and task 1.2 gave `.` the empty signature while
  requiring `None`; and task 4.3 forbade editing tests that call the
  helpers tasks 1 and 2 rename or re-sign. The specify seat corrected D2
  E4 into a measured-versus-inferred table with a new unread item U4, D3
  into one helper with its own grammar and a predicate per site, and D1,
  D5, D12 and D13 with them, and carried each through the proposal, the
  delta and the tasks; nothing was re-measured.
- **The change at the seventh specification sitting.** The clarify judge
  returned CLARIFY-279-5 a third time, now as the incomplete
  `sourceEventSeqs` permission inventory for the 55 types other than
  `assistant/message`, and named the evidence that completes it: the
  member's declaring type in `dsh-session/lib/types/types.d.ts` around
  `:429-446` and the per-type branch of the validation pass at
  `worker.cjs:9824-9930`. That evidence exists only in the installed
  writer. This sitting's box cannot reach it by any route (below), the
  adopted record holds no more than E4's measured cells, and the
  commission forbids the specify seat a repeated writer read; an inferred
  cell would repeat the defect the sixth sitting withdrew. The specify
  seat therefore recorded a reasoned refusal in proposal S6 and decided
  the read's owner and sitting instead: read (d) is assigned to the
  design council's next sitting, the one phase of this run whose seats
  have reached the writer (the reach table in D2), with E4 completed
  there by measurement or its cells left unread if no council seat
  reaches the writer (U4, D13). Nothing was re-measured and no cell
  changed standing.
- **The change at the eighth specification sitting.** The clarify judge
  returned CLARIFY-279-5 a fourth time: the seventh sitting's refusal had
  placed the `sourceEventSeqs` cells out of scope while the delta's growth
  rule still demanded the meaning of the two members on every row that
  carries them as a condition of a version joining the set, and no
  operator amendment had replaced the reconciliation commission's
  inventory criterion. The operator then ruled that the original
  commission's allowance governs: a part the performed reads did not
  reach is left refused or unread with the specific further read named,
  and that is a correct and complete outcome. The specify seat repaired
  the owning rule first, the proposal's growth rule and rulings 1 and 8
  and the delta's growth-rule, evidence and envelope sentences, so that
  they state what the reads established and leave the rest unread
  pending read (d), and added the delta's unread-cell scenario, which
  carries a refusing member on a quiet, a counted-omission and a
  required-unknown row (D12). This design's evidence record follows: U4
  records the ruling, read (d) stays assigned as before, nothing was
  re-measured and no cell changed standing.
- **The recipe's return budget is spent.** `recipes/triage/policy.json`
  rule `DESIGN-UPSTREAM-EXHAUSTED` parks a run whose design reports
  `upstream` after three specification visits; specification has now sat
  four times. This design cannot return the change; it designs to the
  delta and reports `drafted`.
- **This sitting's box** is the specify seat's box: HOME `/runtime/home`,
  user `runner`, no `dsh`, `volta`, `npm`, `cargo` or `~/.dsh`;
  `/home/vyanakiev` holds only `source`; no npm cache and no network (the
  registry host does not resolve), so neither the installed packages nor
  a digest-matched copy of them is reachable by any route. It cannot
  re-read the writer or run Cargo. OpenSpec 1.12.0, Node and Git are
  present.
- **Constraints carried unchanged:** frozen `contracts/`, `policy/`,
  `reference/`, `fixtures/`; the global `dsh` install and its sessions are
  read-only to every seat and never become test inputs (decision 0032); a
  semantic change carries a `Status: proposed` decision.

## Goals / Non-Goals

**Goals:**
- Design the specified admission so the implementation seat can build and
  pin it without interpretation: the admitted-version type, the
  exact-three token check, dispatch keyed on the admitted version and the
  kind, the second quiet list, the seed-gated association pass, and the
  tests that pin every disposition and projection the deltas name.
- Record this council's writer reads with digests, reach stated per seat
  and per operation, and what each read established and did not, so the
  change that lifts the seeded-association gate starts from a citable
  record.
- Reconcile both positions claim by claim on evidence, and keep every
  dependent artifact coherent with the specification.

**Non-Goals:**
- No replay of `surfaceOp`, no reconstruction of the model-visible surface,
  no expansion of embedded streams into fragments, no assembled message the
  writer did not write, no parse of any marker.
- No `system` role, no CLI, TUI or protocol change beyond the one
  `transcript-tui` sentence, no contract, schema, policy or fixture edit.
- No migration of version-zero files, no reading of the host sessions from
  a seat, no committed capture of the install, no re-measurement here.
- No association across a seeded or unattested version-three prefix until
  the seed path is read.

## Decisions

### D1 — Reconcile the council by claim

| Position / claim | Resolution and evidence |
|---|---|
| Simplicity: the 0.1.5-rc.2 message and block definitions (`dsh-llm`) map field for field onto the reader's block parser; project the four content kinds. | **Adopt.** The read is recorded in D2 (digests) and D13 (the mapping); the delta projects the four kinds on it and pins each consumed field with the projection, interrupted-message and partial-support scenarios. The previous design's reasons for not lifting no longer hold: the specification has been revised by its own author on the analyze judge's return, and the re-verification standard it applied to the payload read was never applied to the header, vocabulary and `tool/call` admission that rest on the same seat's read from the same run. |
| Simplicity: the catalogue is exactly 56 names; 51 − 3 + 8 reconciles. | **Adopt.** The delta closes the version-three vocabulary at 56 and refuses any other name; no name is untranscribed (E1). |
| Simplicity §3.2: the version-three codec admits only `type`, `seq`, `time`, `data` at the top level; the timed stream is nested under `data.stream`. | **Adopt as corrected.** The base check at `worker.cjs:9824` is the four members; the same validation pass at `:9824-9930`, which simplicity §3.4 cites, admits and validates the conditional top-level members `surfaceOp` and `sourceEventSeqs` per type (E4). "Only" overstated the base check as the whole envelope; E3 now records the base plus the conditional members, and the conclusion stands that a top-level packed row cannot be written, because it lacks the base `seq` and `time` and carries members no type permits (E3, D6; CLARIFY-279-5). |
| Simplicity: `assistant/attempt` is quiet. | **Reject; counted omission stands.** An attempt carries model output the writer never surfaced. The reader's existing signal for "recognized envelope, content not projected" is the counted `DshRow::Omission` (`transcript.rs:2245`), and its count is the only trace an operator gets that model text exists which the read does not show. Quiet would erase that trace. Simplicity's §8 accepts this outcome. |
| Simplicity §6.9: apply the dedicated-tool association under version three everywhere and accept a cosmetic duplicate in the seeded case. | **Adopt for `isSeeded: false`; reject for every other header value.** The identities the rule keys on are carried on the version-three rows (D13), and a non-seeded session has no prefix from another session, so the version-zero rule applies there on measured fields. A seeded prefix is the one way foreign identities enter a file and its path was not read (U2); the delta withholds the pass there, which shows every embedded copy and hides nothing. The fleet's driver never seeds a session, so the gate is open for every transcript it writes. |
| Simplicity §3.4: `assistant/message` forbids `sourceEventSeqs`; the codec requires unique earlier source sequences and replacement endpoints below the event's `seq`. | **Adopt, as far as it reaches.** The reader's earlier-than-owning citation rule matches the writer's validator; under version three a valid citation suppresses nothing because no chunk row exists (D7). Both members are top-level members of the event row, as the reader has always read `sourceEventSeqs` and as the delta's scenarios place `surfaceOp`. The report records one prohibition and the validation of a present member; it records no permission of `sourceEventSeqs` on any other type, and the previous sitting's "optional on the other surface-eligible types" was an inference from that one prohibition, not a measurement. E4 now separates the measured cells from that inference and records the other 55 types as unread (U4); no reader rule rests on the difference (CLARIFY-279-5, second visit). The third visit asked for the unread cells themselves; no seat of a specification sitting can read them, so the proposal refuses to carry them as a requirement and assigns the read to the council's next sitting, where the seat that reaches the writer completes E4 by measurement (proposal S6; U4, D13; CLARIFY-279-5, third visit). At the eighth sitting the operator ruled that leaving those cells unread with the read named is complete; the growth rule and the envelope requirement no longer demand them, and this row's adoption stands (CLARIFY-279-5, fourth visit). |
| Simplicity §3.7: a version-three `user/message` carries no `turn`/`step`, so its positions are `None` and association still works. | **Adopt the measurement, correct the inference.** The writer omits the fields (E5, corroborated by the key histogram), but the shipped message arm reads `data.turn` and `data.step` for both roles (`transcript.rs:2427-2428`) and the association pass has no role exception (`:2142-2173`), so a supplied pair would let a crafted row own or lose an embedded copy. Under `Three` the arm reads no position from a `user/message` (D4; CLARIFY-279-6). |
| Robustness §1: reach is a property of an operation, not of a seat; a third shape exists (binary resolvable, source unreadable). | **Adopt.** D2's reach table carries the shape, and the decision's ruling 1 says reach is performed by the reading seat at the file it cites and is never inherited (D11). |
| Robustness §2: a payload refusal renders the same string as a never-admitted version; surface the distinction. | **Moot.** No payload refusal exists in the revised delta; `unsupported-format` under version three arises only from the version-zero causes, an invalid citation or packed encoding or a required unknown without the marker (D10). |
| Robustness §3: the tool-call-only synthetic session is a wiring proof and should say so. | **Superseded.** The delta's located scenario projects a full session end to end; no separate wiring proof remains (D12). |
| Both positions: one projector, a vocabulary per version, no registry. | **Adopt** (D4). |
| Simplicity: the six answers live in `design.md` and the decision, with no separate evidence document. | **Adopt** (D5). The rulings themselves live in proposal S5 and are cited, not restated (D11). |

### D2 — Evidence: what the run read, from where, and what it did not reach

**Reach, per seat and per operation, on 2026-09-13.** The commission
admits one primary source, the installed `@deepseek-ai/dsh` 0.1.5-rc.1
writer, and forbids sampling as a substitute. Three box shapes occurred
across this run's seats:

| Seat | `which dsh` | Read the source tree | Box |
|---|---|---|---|
| specify, three sittings; design chief, `c1d57e9` and this sitting | not found | no | HOME `/runtime/home`, user `runner`; no `volta`, `npm`, `cargo` or `~/.dsh`; `/home/vyanakiev` holds only `source` |
| specify, fourth to eighth sittings | not found | no | the same; from the seventh sitting also no npm cache and no network route to the registry, so no digest-matched copy of the packages either |
| design robustness, first sitting | not found | no | the same |
| design robustness, this sitting | resolved `/home/vyanakiev/.volta/bin/dsh` | no: `test -r` on `dsh-session/lib/types/types.js` failed and `find` over the install root was refused | the third shape |
| design simplicity, both sittings | resolved; `dsh --version` printed `0.1.5-rc.1` | yes; digests and line citations taken; every access a read | the install root under `/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/`, packages at 0.1.5-rc.2 |

**Ruling.** Reach is an operation the reading seat performs at the moment
of the read, at the granularity of the file it cites. A resolved binary is
not evidence of source access; a seat's report of reach or of no reach is
not inherited by another seat or by a later sitting of the same seat. A
later change's prerequisite read names an operation to perform then, not a
fact to look up from this run's record.

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
- **E3 The envelope and the stream.** The version-three codec requires the
  base members `type`, `seq`, `time` and `data` on every event row
  (`worker.cjs:9824`) and, in the same validation pass, admits exactly two
  further top-level members, `surfaceOp` and `sourceEventSeqs`, under the
  per-type permissions E4 records where the report states them and marks
  unread where it does not; no other top-level member is permitted. The timed stream is an array nested under
  `data.stream` on `assistant/message` and `assistant/attempt`, whose
  elements are the compact `{type: 'text-chunks' | 'reasoning-chunks' |
  'tool-call-chunks' | 'chunk', time0, index, dt, ...}` objects of
  `AssistantStreamAccumulator` (`worker.cjs:4114-4200`). A top-level packed
  row, `{type, seq0, time0, data}`, lacks the base `seq` and `time` and
  carries members no type permits, so this codec cannot write one. The
  stream is still never expanded (D6). The record's earlier sentence that
  the codec admits "only" the four base members was the base check
  overstated as the whole envelope; it is corrected here, from the cited
  lines already in the record and not from a new read (CLARIFY-279-5).
- **E4 The conditional top-level members.** The report records, from
  `types.d.ts:429-446` and the validation pass at `worker.cjs:9824-9930`,
  exactly three facts about them: `surfaceOp` is required on the four
  surface-eligible types and the codec "enforces exactly that" over a
  catalogue whose other 52 types are log-only (`types.d.ts:413`,
  `surface.d.ts:52-64`); `assistant/message` forbids `sourceEventSeqs`;
  and a present `sourceEventSeqs` must cite unique sequences earlier than
  the event, with replacement endpoints below its `seq`. It records no
  permission, requirement or prohibition of `sourceEventSeqs` on any other
  type. By type, each cell with its standing:

  | Version-three type | `type`, `seq`, `time`, `data` | `surfaceOp` | `sourceEventSeqs` |
  |---|---|---|---|
  | `assistant/message` | required (measured, E3) | required (measured) | forbidden (measured) |
  | `user/message`, `tool/result`, `system/message` | required (measured, E3) | required (measured) | not recorded: the read cites neither a permission nor a prohibition; a present member is validated (measured) |
  | the other 52 types | required (measured, E3) | forbidden (measured as the codec enforcing the four-type requirement exactly; the report quotes no per-type predicate) | not recorded |

  Any member outside these six is outside the envelope the codec admits
  (E3). The previous sitting's table read "optional on the other
  surface-eligible types" out of the one prohibition being stated for
  `assistant/message` alone; a prohibition on one type establishes nothing
  about another, so that cell is withdrawn as a measurement and the
  permission is recorded as unread (U4). Nothing the reader relies on
  moves: writer validity is not reader admission; the reader validates no
  event row's key set, requires and refuses no marker, and reads
  `sourceEventSeqs` only on the three content kinds its version-zero rules
  read it on, `assistant/message` included (D7). The delta's
  writer-validity scenario pins that distinction, and none of its rows
  depends on whether the writer permits a citation on the type that
  carries it (CLARIFY-279-5, second visit). For the four kinds the reader
  projects, the cells and their standing are: the base four, measured on
  all four; `surfaceOp`, required on `user/message`, `assistant/message`
  and `tool/result` and forbidden on `tool/call`, measured;
  `sourceEventSeqs`, forbidden on `assistant/message` by measurement and
  unread on `user/message`, `tool/result` and `tool/call`. The unread
  cells, here and on the other 52 types, are completed by read (d) at the
  council's next sitting or stay unread; they are not inferred (U4;
  CLARIFY-279-5, third visit).
- **E5 The message and block definitions.** `Message` is `{id, role,
  content, source}` (`dsh-llm/lib/types/message.d.ts:120-153`); the block
  union is `text`, `reasoning`, `image`, `tool-call`, `tool-result` and
  `file` (`dsh-llm/lib/types/types.d.ts:39-98`). The field-by-field
  mapping onto the reader's parser is D13's table. This is read (a) of D13,
  adopted by the delta.
- **E6 Row admission by the codec.** The version-three codec refuses the
  two `tool/code-dispatch*` names and treats `assistant/chunk` as an
  opaque unknown envelope (`worker.cjs:10001`): the writer does not emit
  it, and a file carrying it is not a version-three file the writer wrote.
- **E7 Seeded consistency.** The codec enforces agreement between
  `isSeeded` and the `session/end-seed` marker (`worker.cjs:10057-10110`);
  the prefix length is `inheritedEventCount` session state
  (`types.d.ts:105-109`, `:139`).

**What remains unread**, so the delta withholds exactly one rule on it and names the read that lifts it:

- **U1 Replacement copies (read (b) of D13).** E4 measures the codec's
  validation of a replace marker, not the append and replace writer paths:
  what a replace-marked surface row carries, whether it duplicates or
  summarizes the rows it replaces, and how an append-origin row is told
  from it for display. No rule rests on it: the reader parses no marker and a
  persisted row displays where it was persisted (D7).
- **U2 Inherited identities (read (c) of D13).** Whether a seeded session's
  inherited prefix keeps its originating `seq`, `turn`, `step` and call
  identities, or remaps them, is not cited; E7 measures consistency of the
  marker, not identity of the rows. The association pass is gated on
  `isSeeded: false` under version three until this is read (D4, D7).
- **U3 `surface.d.ts`** is cited by line but not by digest; `surface.js`
  from the same package version is.
- **U4 The per-type permission of `sourceEventSeqs` (read (d) of D13).**
  The read cites the member's prohibition on `assistant/message` and its
  validation when present, and no permission, requirement or prohibition
  on any of the other 55 types (E4). No rule rests on it: the reader
  validates the member wherever its version-zero rules read it and never
  consults the writer's permission. The read that records it is the
  member's declaring type in `dsh-session/lib/types/types.d.ts`, the event
  base or the per-type definitions around `:429-446`, together with the
  per-type branch of the validation pass at `worker.cjs:9824-9930`, cited
  by package, file, digest and line. **Owner and sitting.** No
  specification box of this run reaches those files and the commission
  forbids the specify seat a repeated writer read, so the specification
  refuses to carry the cells as a requirement (proposal S6) and assigns
  the read to the design council's next sitting: whichever council seat
  reaches the writer performs it, and the chief completes E4's table,
  each of the 55 cells entered as permitted, required or forbidden by
  measurement. A council whose seats all report no reach records that in
  the reach table, as this run's seats have, and leaves the cells unread;
  the change proceeds either way, because the delta's writer-validity
  scenario pins that no reader outcome moves with them and the design
  cannot return the change on an evidence cell no rule rests on
  (CLARIFY-279-5, third visit). **The requirement demands no inventory.**
  At the eighth sitting the operator ruled that a part the performed
  reads did not reach is left unread with its read named and that this
  is a complete outcome; the delta's growth rule and envelope paragraph
  now say so instead of demanding a record of the members on every row
  that carries them, and the unread-cell scenario (D12) falsifies the
  closure across the quiet, counted-omission and required-unknown
  classes (proposal S2, S6; CLARIFY-279-5, fourth visit).

**Corroboration, after the writer.** The simplicity seat's type-only,
content-free histogram over the `session.v3.jsonl` files under
`~/.dsh/sessions/brokkr` (20 at its sitting, 17 when the commission was
written; the fleet keeps running seats) shows no top-level packed row and
no `assistant/chunk`, every header key set equal to E2's required set plus
`cwd`, and payload key sets matching E5. It counted header and payload key sets,
not each type's top-level event members, so it does not speak to U4
either. It changes no disposition and proves no admission; the commission permits it after the writer, decision
0032 ruling 3 constrains the journal and the driver rather than a seat's
corroborating read, and the controller's corroboration step in proposal
S4 stands.

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

**Spellings: one signature helper, a predicate per site.**
`zero_number_token` is replaced by a digit-signature helper over the
recorded token. The helper's own grammar is: an optional sign; a mantissa
of characters in `0-9.` that contains at least one digit; and, if an `e`
or `E` follows, an exponent of an optional sign and at least one digit. A
token outside that grammar has no signature, so `.`, `3e`, `e5` and the
empty token return `None`, the dot-only mantissa because it carries no
digit. Within the grammar the signature is the mantissa digits with the
decimal point and every leading and trailing zero removed. The helper
validates no more of the JSON number grammar than this: it is applied only
to a token the JSON parser has already read as a number, so a malformed
number such as `1.2.3` or `0.` is refused by the parser before any
signature is taken, and the helper's own unit test is the only place it
meets such a token (CLARIFY-279-10). The helper knows nothing about parsed
values: `0`, `0.0`, `-0`, `0e0` and `0.000` have the empty signature;
`1e-400`, `10e-400`, `0.1e-400`, `1.0e-400`, `1e3` and `1000.0` have `1`;
`3`, `3.0`, `3e0`, `30e-1`, `0.3e1`, `-3`, `3e1` and `300` have `3`; `3.1`
has `31`; `2.9999999999999999` and `3.0000000000000001` have their
seventeen digits.

The predicate differs by site, and S5 ruling 7 states all three. At the
header, admission is the combined check: the parsed value is zero or three
and the signature is empty or `3` respectively. So `-3` and `3e1` refuse
on the parsed value, the rounding artefacts refuse on the signature, and
the zero-digit underflow tokens refuse on the signature where the shipped
predicate admitted them. The combined check is complete for both
integers: a token whose digits are one `3` and zeros has the value 3·10^k,
which is three only at k = 0, and a token with the empty signature is
exactly zero. At the two time sites, an ordinary `time` and a packed
`time0`, the parsed value must be a signed safe integer as the content
rules already require, and the signature decides one thing only: a parsed
zero is a zero spelling when its token's signature is empty, so `10e-400`
is invalid exactly as `1e-400` is. A nonzero integer is judged on its
parsed value alone: `1e3` and `1000.0` have the signature `1` and render
`1000`, as `tests.rs:2160-2176` already pins, and no comparison against
the integer's digits is made there; a nonzero token that rounds to an
integer, such as `1000.0000000000001`, renders that integer today and
under this change, because the time sites' exactness has always been the
zero spelling and widening it is not asked for. The previous sitting's
ruling 7, "a token spells an integer exactly when its parsed value is
that integer and its digit signature is that integer's digits", was
proved only for the two header integers and, read literally at a time
site, would invalidate every exponent-spelled timestamp; it is restated
per site (CLARIFY-279-9). Alternative rejected: parsed value only for
three, because it admits a token the writer never wrote and drops a
defence the zero path already has (robustness §5 at the first sitting).

**Token binding.** `raw_top_level_token` decodes JSON string escapes in
member names before comparing and returns one of three outcomes: exactly
one top-level member of that name with its raw value token, no such
member, or more than one. The scan already skips nested depth and string
contents; the change is the decode and the count. `dsh_header` admits only
when the parsed `version` is zero or three, the scan finds exactly one
`version` member and its signature matches; a duplicated member is an
ambiguous recording and refuses whatever its values, and a parsed number
whose token the scan cannot establish refuses rather than passing, so the
check never falls open to the parsed value. `dsh_millis_exact` takes the
same three-way outcome: a duplicated member is invalid whatever the value;
for a parsed zero the one token must have the empty signature; a parsed
number with no established token is invalid. Binding to the first or the
last duplicate was rejected because `serde_json` retains the last
occurrence while the shipped scan returned the first, so either binding
can disagree with the parser and admit a rounded token; no writer records
a member twice, so refusing costs nothing real (CLARIFY-279-8).

**Compatibility, stated plainly.** Two shipped behaviours change, under
version zero as well as three, and both are corrections toward
requirements the living text already states. `zero_number_token` requires
a zero digit somewhere in the mantissa, not everywhere (`:1883-1885`), so
`10e-400` and `0.1e-400` were admitted as version zero, rendered `"0"` as
an ordinary `time` and passed as a packed `time0`; with the signature they
refuse, render `""` and refuse the row. `dsh_header` treated a missing raw
token as passing (`:1931`), and the first-match scan missed escaped names
and bound the first duplicate; with the binding a missing or duplicated
token refuses at the header, invalidates an ordinary time and refuses a
packed row, and an escaped name binds. The previous design's "the zero
path keeps its rule unchanged" and tasks 1.2 and 1.4's "byte-for-byte"
are withdrawn; the existing underflow regressions (`tests.rs:2126`,
`:2143`, `:2180`) still pass because `1e-400` has no zero digit, and new
regressions pin the tokens that do (D12; CLARIFY-279-7).

**Exactness scope.** The recorded-token check judges the header `version`,
an ordinary event's `time` and a packed row's `time0`, under either
version, and no other field. Admitting three widens the version check from
one admitted integer to two and adds no field: `surfaceOp`, citations and
every other event field are never judged on their token. The helper and
the binding are shared by all three sites; the predicate and the
consequence differ by site: at the header the combined value-and-signature
check and refusal, at an ordinary time the safe-integer rule with the
signature deciding a parsed zero and an empty stamp on failure, at a
packed `time0` the same rule with refusal of the row.

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
| `user/message`, `assistant/message`, `tool/call`, `tool/result` | the existing arms, with one version condition | The envelopes and block definitions carry every field the arms read (D13). The `user/message` definition declares no `turn` or `step`, so under `Three` the shared message arm reads neither from a `user/message`, whatever its `data` carries: the two `Position::parse` reads at `:2427-2428` are taken only for `assistant/message` or under `Zero`. The shipped arm reads both for both roles, so "unchanged" would not keep the delta's promise (CLARIFY-279-6). `assistant/message` and `tool/result` read `data.message`, `data.turn` and `data.step` as today; `tool/call` reads `callId`, `name`, `arguments`, `turn` and `step`. `dsh_message_blocks` (`:2278`) is unchanged: a `file` block falls to its unrecognized-block branch, keeps its siblings and counts the row once; `interrupted`, `stream`, `usage`, `error`, `meta`, `id` and `source` are never read. `dsh_citations` (`:2342`) runs on the three surface kinds as today. |
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
(nothing the reader projects differs between the versions once the
payloads are measured).

**Association.** `dsh_header` returns the admitted version together with
the header's `isSeeded` value as `Option<bool>` (a JSON boolean, else
`None`), in a small private struct; `project_dsh` calls
`associate_dsh_tools` when the version is `Zero`, or when it is `Three`
and the value is `Some(false)`, and skips it otherwise. The pass itself is
unchanged: it keys on call id, direction, turn and step and removes an
embedded copy from a non-dedicated event only when exactly one dedicated
event carries that key (`:2142-2171`), with no role exception. A
version-three `user/message` never participates because the arm gives it
no positions under `Three` (above), not because the writer happens to
omit them: a supplied `turn` and `step` beside an embedded `tool-result`
would otherwise let the one matching dedicated result remove the copy
under an unseeded header, and the delta's supplied-positions scenario pins
that the copy stays under every version-three header and is removed under
version zero. Alternatives rejected: a version guard inside the pass (the
gate is a header fact, and the pass should not read headers); a role
exception inside the pass (version zero reads and uses user positions);
applying the pass under every version-three header (rests on U2);
refusing seeded files (turns an inert header field into an admission
gate, which the commission's Q3 ruled out).

**No refusal arm.** The previous design's `DshRow::RefusedPayload` is
withdrawn with the refusal it served. `unsupported-format` after header
admission arises under version three only from the version-zero causes:
an invalid citation (`:2414`, `:2476`) or a required unknown without
`ignorable: true`; an invalid packed encoding cannot occur because packed
rows are unknowns under `Three`.

### D5 — The six questions of #279, answered on the reads

Each row gives the measured answer with its evidence, the reader
consequence under this change, and what stays unmeasured. Evidence labels
are D2's; the specification consequences are proposal S3's.

| # | Question | Measured answer and evidence | Reader consequence | Unmeasured |
|---|---|---|---|---|
| 1 | Record types and shapes the v3 writer emits | `SESSION_FORMAT_VERSION = 3` (`types.js:54`); catalogue `currentVersion: 3`, codecs `v0..v3`, lossless migrations `v0→v1→v2→v3` (`dsh-session-format-catalog/lib/index.js:33-46`); the vocabulary is the 56-name generated set (E1); payload envelopes are `SessionEventMap` (`types.d.ts:309-338`): content in `data` for `user/message`, `data.message` for `assistant/message` and `tool/result`, `callId`, `name` and `arguments` on `tool/call`; the physical envelope is the base `type`, `seq`, `time`, `data` on every row plus the conditional top-level `surfaceOp` and `sourceEventSeqs`, with E4's per-type table: measured for `surfaceOp` on every type and for `sourceEventSeqs` on `assistant/message`, unread for `sourceEventSeqs` elsewhere (E3, E4, U4); the message and block definitions are E5 and map onto the parser (D13); surface-eligible types are exactly `system/message`, `user/message`, `assistant/message`, `tool/result` (`types.d.ts:413`), every other type log-only (`surface.d.ts:52-64`). | Per-version vocabulary (D4); dispositions in D8; the four content kinds project through the existing arms; `file` is a counted unrecognized block. | The permission of `sourceEventSeqs` on the 55 types other than `assistant/message` (U4, read (d) at the council's next sitting); nothing the projection depends on. |
| 2 | `assistant/chunk` absent from v3 | Removed, not renamed and not conditional. It is outside the 56-name catalogue (E1); the codec treats it as opaque unknown (E6) and cannot write a top-level packed row (E3). The v1→v2 package is the assistant-stream migration; in v3 the timed stream is embedded: `assistant/message` carries `message`, `stream`, optional `usage` and `interrupted?: true` (`types.d.ts:309-317`); `assistant/attempt` carries a stream with no surface message (`types.d.ts:323-327`); "a turn cancelled mid-stream finalizes its delivered text/reasoning prefix as this event with `interrupted: true`" (`types.d.ts:303-307`). | Whole messages only under version three; the fragment rule's text stands and has nothing to concatenate; the reader assembles nothing under either version. A finalized message, interrupted or not, projects as one turn with no marker. `stream`, `usage` and `interrupted` are never expanded. `assistant/attempt` is a counted omission. `assistant/chunk` and the packed rows are required unknowns under version three (D6). | Nothing for the disposition. |
| 3 | `isSeeded` new in the header | A required v3 header boolean (E2) meaning the session contains a fork-inherited event prefix (`types.d.ts:72-76`); the prefix length is `inheritedEventCount` session state (`types.d.ts:105-109`, `:139`), not header metadata; the cut is the `session/end-seed` marker and the codec enforces their agreement (E7); delegation depth is a separate field (`types.d.ts:82-87`). Ownership is `type: session` plus depth, and the reader's header admission reads no other field. | Inert for ownership, depth, admission, classification and counts; pinned over true, false, string, object, null and absent under both versions; no provenance or delegation claim rests on it. Under version three it gates the association pass: `Some(false)` runs it, anything else withholds it (D4, D7). | Whether inherited rows keep their originating `seq`, `turn`, `step` and call identities (U2); the gate confines that gap to seeded and unattested files. |
| 4 | `surfaceOp` and `sourceEventSeqs` rise to about 21% of rows | `surfaceOp` is `'append' \| {op: 'replace', startSeq, endSeq}`, required on the four surface types and forbidden elsewhere; `assistant/message` forbids `sourceEventSeqs`; the codec validates unique earlier sources and replacement endpoints below the event's `seq` (E4). `sourceEventSeqs` is individual non-negative integers plus inclusive `[start, end]` pairs, strictly increasing (`seq-ranges.js`, byte-identical to 0.1.2-rc.1): the grammar `dsh_citations` (`:2342`) accepts. The rise is the writer placing every surface message on the surface with an explicit marker. The writer's contract: the surface "is the wrong source for a human transcript — a landed replacement would erase conversation the user already saw. Append-origin events are that transcript's durable source material; replacement copies stay model-only" (`surface.d.ts:27-33`). | The reader never parses `surfaceOp`, never removes, reorders or replaces a row on it and never replays the surface: measured, and 0055 ruling 3's audit rule. A replace-marked message projects as a message at its recorded position, as version zero's compaction message does; a replace-marked `system/message` is quiet. Citations validate identically and suppress nothing under version three (D7). | What a persisted replacement copy carries (U1), and whether any type but `assistant/message` permits `sourceEventSeqs` (U4, read (d) at the council's next sitting); no rule rests on either. |
| 5 | `system/message`, `todo/write`, `turn/end` in v3 and not in version zero | `system/message` is the rendered system prompt as surface node 0 (`types.d.ts:288-298`; the v2→v3 migration's `emitSystem`, `dsh-session-format-v2-to-v3/lib/index.js:780-810`), the successor of version zero's `request/context`, which is quiet (`transcript.rs:1787`). `todo/write` and `turn/end` are in both catalogues, quiet today (`:1803`, `:1810`), and log-only under v3. | All three quiet under version three, whatever their payload or `surfaceOp`; `system/message` under version zero stays a required unknown because the 0.1.2-rc.1 catalogue does not contain it. A displayed `system` turn would be new command and TUI capability, not asked for. | Nothing the quiet disposition depends on: quiet reads no payload. |
| 6 | Is version 3 a superset of version zero? | **No as a vocabulary; yes as a payload.** Versions are physical generations joined by migrations: v3 removes `assistant/chunk` and the two `tool/code-dispatch*` names, adds eight names, moves the prompt from `request/context` to `system/message` and the stream from chunk rows into the message (E1, E3, E6). The event map keeps the envelope nesting the reader reads for the four content kinds (Q1), and E5 reads the block definitions as field-compatible with the version-zero parser (D13). | Admit `3` beside `0` with a per-version vocabulary and one projector; every version-zero meaning is preserved for version-zero files and every projected version-three meaning is measured. | Nothing; if a later writer moves a block's meaning, its admission adds a per-version payload projection rather than widening this one. |

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
- A finalized `assistant/message`, interrupted or not, projects through
  the existing arm as one turn at its own position and time with no
  marker; the delta's interrupted-message scenario pins it, including the
  partial append after it.
- `interrupted`, `stream` and `usage` are never expanded, duplicated as
  turns or rendered as prose. The stream element shape is cited (E3);
  expansion is still rejected, because it would duplicate the message the
  writer finalized or fabricate fragments the writer chose not to persist
  as rows.
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
transcript (D5 Q4). `dsh_citations` therefore applies unchanged on the
three surface kinds; under version three a valid citation proves no chunk,
so it suppresses nothing, and an invalid one refuses as today. `surfaceOp`
is never parsed under either version: a replace-marked message projects as
a message at its recorded position, a replace-marked `system/message` is
quiet, and nothing is removed or reordered.

**Unread, so not claimed.** What a persisted replacement copy carries (U1)
does not change the rule, because the rule reads no marker; a later change
that reads the append and replace paths may add a marker-aware
presentation, never a removal. Whether inherited identities are
association evidence (U2) is the one unread fact a rule rests on, so the
association pass is gated on `isSeeded: false` under version three (D4).

**Alternatives rejected.** A `surfaceOp`-aware suppression model imports
the model's rewritten view into an audit read, which the writer itself
calls the wrong source. Refusing or omitting a replace-marked row parses
the marker and decides on unread content. Applying association under every
version-three header rests on U2; withholding it under every version-three
header doubles every tool call in every fleet transcript for a risk that
arises only in seeded files.

### D8 — Dispositions under version three, enumerated from evidence

"Recognized quiet event kinds SHALL be enumerated per admitted version
from evidence in design." This is that enumeration for version three;
version zero's is unchanged from archived D6 (#222).

| Type under a version-three header | Disposition | Evidence |
|---|---|---|
| `user/message`, `assistant/message`, `tool/call`, `tool/result` | Content, projected through the existing arms | Envelopes cited (`types.d.ts:281`, `:309-317`, `:333`, `:351`); message and block definitions E5, mapped in D13 |
| `assistant/attempt` | Counted omission | Stream with no surface message (`types.d.ts:323-327`); D6 |
| `system/message` | Quiet | Rendered system prompt (`types.d.ts:288-298`; v2→v3 `emitSystem`); `request/context` precedent |
| `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch`, `tool/ptc-dispatch-start` | Quiet | In the 56-name catalogue (E1) and outside the four surface types; `tool/ptc-dispatch*` succeed `tool/code-dispatch*` (v2→v3 PTC migration) |
| The 44 version-zero quiet names other than `tool/code-dispatch` and `tool/code-dispatch-start` | Quiet | Shared by both catalogues (E1); log-only under v3; quiet reads no payload |
| `assistant/chunk`, `text-chunks`, `reasoning-chunks`, `tool-call-chunks`, `tool/code-dispatch`, `tool/code-dispatch-start` | Required unknown (refuse unless `ignorable: true`) | Absent from the v3 catalogue (E1); cannot be written by the v3 codec (E3, E6) |
| A later `session` row | Unknown envelope under the ignorable rule | Unchanged from version zero |
| Any other name | Required unknown | The vocabulary is closed at 56 (E1); the safe direction for a future writer's addition |

The version-three quiet set is therefore 51 names: the 44 shared names plus
`system/message` plus the six log-only names of row four. It is encoded as
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

### D10 — The observable shape of a version-three read

**What the operator sees.** A version-three session of this fleet projects
its user, assistant, call and result turns in source order through CLI
text, CLI JSON and the TUI, with `unrecognized_records` counting only
`assistant/attempt` rows, rows carrying a `file` block and any required
unknown that carried `ignorable: true`, and the notice
`unrecognized transcript records: N` when that count is positive. A
seeded or unattested version-three session shows each embedded tool-call
copy inside its assistant turn beside the dedicated call turn, with no
count and no notice; the transcript shows more, never less.
`unsupported-format` after header admission arises only from an invalid
citation or a required unknown without the marker, exactly as under
version zero, and keeps the counts and notices the `transcript-command`
and `transcript-tui` requirements pin (`crates/brokkr-cli/src/render.rs:830`;
`transcript-command` scenarios at `:227` and `:312`; `transcript-tui`
scenarios at `:158` and `:240`). A never-admitted version returns the same
explanation with zero counts and no notice.

**No new explanation string and no new notice.** The seeded gate is silent
by design: a notice would be a new string across two capabilities for a
state that hides nothing and that the seed-path change removes. The string
set is unchanged, and the `transcript-tui` delta changes one sentence's
scope, not its strings.

### D11 — A proposed decision carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes ("DSH
numeric on-disk version zero only"), so the implementation seat files a
numbered decision with `Status: proposed`, supplementing ruling 3 without
editing it, with its row in `docs/decisions/README.md`. **Number:** the
next free number when the decision is written, re-read from the index at
the PR; at this sitting main's index holds 0055 and 0057 through 0059,
0056 is claimed by the #226 branch and 0060 by a `review-first` run's
journal anchor, so neither is taken. **Rulings:** the eight rulings are
proposal S5's and are the one authoritative copy; the decision transcribes
them verbatim, cites D2's digest table as its evidence and D13 as the
payload mapping, and this design does not restate them. Enforcement
bindings: the tests of D12, `openspec validate --all --strict`, and the
exact-coverage gate. 0032's ownership, retention and privacy constraints
are preserved: no real session is read by a seat as a test input, copied
or committed; tests use synthetic rows in test-owned homes.

### D12 — Proof, host-independent, and the gates

All tests use inline synthetic rows in test-owned homes; nothing under
`~/.dsh` or `fixtures/` is read. Names are suggestions; each pins the
delta scenario it is named after. In `crates/brokkr-view/src/transcript/tests.rs`:

- **Version matrix.** `dsh_header_version_matrix_admits_only_numeric_zero`
  (`:1019`) becomes the two-version matrix: admitted `0`, `0.0`, `0e0`,
  `-0`, `0.000`, `3`, `3.0`, `3e0`, `30e-1`, `0.3e1`; refused `null`,
  `false`, `"0"`, `"3"`, `[]`, `{}`, `1`, `2`, `4`, `-1`, `-3`, `0.5`,
  `3.1`, `3e1`, `2.9999999999999999`, `3.0000000000000001`, `1e-400`,
  `10e-400`, `0.1e-400`, `1.0e-400` and an absent version; every refusal
  keeps zero counts and the confirmed path.
- **Signature helper.** A unit test on the helper alone, separate from
  admission: the empty signature for the zero spellings, `1` for the four
  underflow tokens, `3` for `3`, `3.0`, `3e0`, `30e-1`, `0.3e1`, `-3`,
  `3e1` and `300`, `31` for `3.1`, the seventeen-digit strings for the two
  rounding artefacts, `1` for `1e3` and `1000.0`, and no signature for
  `3e`, `.`, `e5` and the empty token, `.` because its mantissa has no
  digit, the case the helper's at-least-one-digit condition exists for and
  the one that separates it from `0.` and `0.0`. The test exercises the
  helper's own grammar, not the JSON grammar, which the parser enforces
  before any token reaches the helper. That `-3` and `3e1` share `3` is
  asserted here; their refusal is the matrix's, on the parsed value.
- **Token binding.** Raw-string headers, never built through a parsed
  object that would lose the evidence:
  `{"type":"session","\u0076ersion":3,"delegationDepth":0}` admits as
  three and reads a following `system/message` quietly; the same header
  with `3.0000000000000001` refuses; `"version":3,"version":3.0000000000000001`,
  its reverse, `"version":3,"\u0076ersion":3` and `"version":0,"version":3`
  each refuse with zero counts and the confirmed path. A scanner unit test
  asserts the three outcomes (one, none, more than one) on those rows and
  on `{"data":{"version":3}}` and `{"cwd":"version"}`, which must count
  none.
- **Quiet-list relation.** A test over the two lists asserting version
  three equals version zero minus the two `tool/code-dispatch*` names plus
  the seven quiet version-three names, and that neither list holds
  `assistant/attempt`, `assistant/chunk` or a packed tag.
- **Dispositions.** Under a version-three header: the ten names of the
  delta's "Version-3 names are ruled under the version-3 vocabulary"
  scenario are quiet beside one projecting `user/message`, with
  `system/message` supplying no turn whatever its payload or `surfaceOp`;
  the six version-zero-only names and one name outside the catalogue are
  required unknowns with and without `ignorable: true`. Under a
  version-zero header: each of the seven quiet version-three names and
  `assistant/attempt` is a required unknown, `system/message` stays one,
  and `todo/write` and `turn/end` are quiet ("Version zero already rules
  the three sampled types").
- **Projection.** `dsh_v3_content_projects_on_the_writers_definitions`:
  the delta's projection scenario, four turns in order with ordered
  blocks, `[image omitted]`, the `file` block counted once with its
  notice, string content as one text block, `id`, `source`, `stream`,
  `usage` and `surfaceOp` inert, `turn`/`step` on a user message inert; the
  same rows under a version-zero header project identically.
- **Interrupted message.** One `assistant/message` with
  `interrupted: true`, `stream`, `usage` and text, followed by a partial
  append: one turn with that text, zero counts, no notice, the append not
  counted.
- **Attempt.** `user/message`, `assistant/attempt` with a stream,
  `assistant/message`: two turns in order, one count, one notice, read
  available; the same with `ignorable: true` on the attempt.
- **Citations.** A `user/message` citing `[[3, 5], 7]` over earlier rows,
  a `tool/result` citing an unobserved sequence and an `assistant/message`
  citing earlier rows all project every row with zero counts; `null`, a
  reversed range and a self reference on the user message each refuse with
  one count.
- **Replacement copy.** Two messages, a replace-marked `system/message`
  and a later replace-marked `assistant/message`: three turns in order,
  zero counts, nothing removed.
- **Supplied user positions.** The delta's supplied-positions scenario: a
  version-three `user/message` with a text block `see`, an embedded
  `tool-result` for `c1` with content `out` and `turn: 1, step: 1`, then a
  dedicated `tool/result` for `c1` at turn 1 step 1, under `isSeeded`
  `false`, `true` and absent: two turns, the user turn keeping both blocks
  (`see`, `out [c1]`) and the tool turn `out [c1]`, zero counts; the same
  two rows under a version-zero header give the user turn `see` alone.
- **Writer validity.** The delta's writer-validity scenario: a
  `user/message` without `surfaceOp`, a `tool/call` with
  `surfaceOp: "append"` and `sourceEventSeqs: null`, a readable
  `tool/result` with a valid citation, an `assistant/message` with a
  valid citation and a `step/end` with `sourceEventSeqs: null` project
  four turns with zero counts; the same file with a `null` citation on
  the assistant message, and the same file with a `null` citation on the
  tool result, each refuse with one count. The two cited rows sit on a
  forbidden and an unread cell of E4, the two `null` members the reader
  never reads on two unread cells; no assertion moves if any cell
  resolves differently.
- **Unread cells across dispositions.** The delta's unread-cell scenario:
  a version-three `user/message` at sequence 1, a `todo/write` at 2 and a
  `deliverables/presented` at 3 each with `sourceEventSeqs: null`, and an
  `assistant/attempt` at 4 with a stream and `sourceEventSeqs: [[5, 1]]`
  project the user message with one count, the attempt's; the same file
  plus a `text-chunks` packed row with `sourceEventSeqs: null` refuses
  with two counts and no turns; the same file plus an `assistant/chunk`
  with `sourceEventSeqs: null` and `ignorable: true` projects the user
  message with two counts. Every one of those members would refuse the
  read if the reader validated it, and `project_dsh` reads only `seq`
  and `ignorable` from a quiet, counted-omission or unrecognized row
  (`transcript.rs:2011-2032`), so the claim that no disposition rests on
  an unread E4 cell is falsifiable on every class, not only the content
  kinds; no assertion moves if any cell resolves differently.
- **Seeded association.** The delta's seeded scenario as a matrix over
  `isSeeded` `false`, `true`, `"yes"`, `{}`, null and absent under version
  three: only `false` suppresses the embedded call block; every value
  projects the same rows with zero counts; the ambiguous-identity case
  keeps both dedicated calls and the copy under `false`; a user message's
  embedded `tool-result` keeps its block under every header; the same rows
  under a version-zero header associate whatever `isSeeded` carries.
- **Fragment rows under version three.** `assistant/chunk` and each of
  the three packed rows after a `user/message` refuse with one count; with
  `ignorable: true` each projects the message with one count.
- **Time exactness.** A `3e0` header with `tool/call` rows at `1e-400`,
  `1e3`, `1000.0` and `-0.0` projects stamps `""`, `"1000"`, `"1000"`,
  `"0"`, the two nonzero integers on their parsed value alone; the
  existing underflow regressions (`:2143`, `:2180`) and the existing
  integral-spelling test (`:2160`) stay. The delta's zero-digit
  scenario, on raw rows: version-three `tool/call` rows at `10e-400`,
  `0.1e-400`, `1.0e-400`, a row recording `time` twice (`1000` then
  `2000`) and a row whose member is `"\u0074ime"` with `1e-400` each
  stamp `""`, rows at `0`, `0.0`, `-0` and `0e0` stamp `"0"`, all with
  zero counts; a version-zero `assistant/chunk` at `10e-400` and one
  recording `time` twice stamp `""`; version-zero packed `text-chunks`
  rows with `time0` at `10e-400` and with `time0` recorded twice each
  refuse with one count.
- **Partial support.** The living "Known DSH event content keeps counted
  partial support" case under version zero and again under version three
  with a `file` block as one of the two unsupported blocks.
- **Seeded header admission.** `isSeeded` true, false, `"yes"`, `{}`,
  null and absent under both versions admit and change no classification
  or count; version 3 with `isSeeded: true` and depth 1 is not a candidate.
- **Version-zero regressions.** Every pre-existing DSH test keeps its
  behavioural assertions and is green. "Unmodified" is withdrawn as the
  measure, because tasks 1 and 2 rename or re-sign helpers that six
  existing tests call directly; those calls adapt mechanically and assert
  what they asserted: `raw_top_level_token_and_zero_number_edges`
  (`:2735`; `dsh_quiet_event` takes the version, `zero_number_token`'s
  true cases become the empty signature and its false cases `Some("1")`,
  `Some("123")` or `None`, the scanner's `Some`/`None` become the one and
  none outcomes), `raw_token_scans_whitespace_and_nonmatching_keys`
  (`:3150`, the same), `home_time_and_key_scans_cover_their_false_branches`
  (`:3166`, the scanner outcome and `dsh_time`'s token argument),
  `position_and_time_cover_the_float_edges` (`:3077`, whose direct
  `dsh_time` calls supply the tokens `5` and `-5` beside the parsed values,
  because a parsed number without an established token is now invalid and
  the assertion is that a valid millisecond renders),
  `packed_empty_members_allocate_no_events_but_stay_observed` (`:2228`,
  whose direct `dsh_packed` calls pass the row's own JSON text as `raw`
  instead of `""`, for the same reason) and
  `dsh_row_refuses_out_of_contract_rows_and_quiet_empty_chunks` (`:2871`,
  `dsh_row` takes `Zero`). One existing test is extended on purpose:
  `dsh_header_version_matrix_admits_only_numeric_zero` becomes the
  two-version matrix above. No compatibility wrapper keeps an old call
  compiling: a shim would be dead code under the exact-coverage gate. No
  existing test spells a zero-digit underflow token, a duplicated member
  or an escaped name, so no existing behavioural assertion changes; that
  is why the record can say both that every existing assertion passes and
  that version-zero behaviour changes (CLARIFY-279-11).

In `crates/brokkr-cli/src/ui/tests.rs`: the delta's two sibling scenarios
(version 3 beside version zero reads the versioned name's message, never
the plain name's; version 4 beside version zero refuses with the versioned
path, zero counts, no turns) and the filename/version independence triple
(`session.jsonl` at version 3 projects its message; `session.v3.jsonl` at
version 0 projects its message; `session.v3.jsonl` at version 4 refuses).
In `crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: the
delta's located scenario end to end, one root projecting four turns
through CLI text, CLI JSON and the TUI with `--turn 1` through `--turn 4`
reaching each, and the interrupted-message scenario's whole, `--turn` and
both-doors reads.

Gates before the activation commit: `openspec validate --all --strict`,
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings`, `cargo test --workspace
--all-features --locked`, both bundle compiles. Exact coverage runs
outside the box and is recorded pending until its result exists; the gate
is not lowered. Remote CI, host corroboration, publication and closure are
the controller's.

### D13 — What was read, and what a later change starts from

**Read (a), the payload mapping, performed by the simplicity seat and
adopted by the delta.** The 0.1.5-rc.2 `SessionEventMap` payloads and
block union against the reader's parser (`dsh_row`, `:2394`;
`dsh_message_blocks`, `:2278`):

| Writer definition | Reader read today | Finding |
|---|---|---|
| `user/message.data` is the `UserMessage` itself: `{id, role, content, source}`, no `turn` or `step` (`types.d.ts:281`; `message.d.ts:120-153`) | `data.content`; `data.turn`, `data.step` | Field-compatible for `content`. The definition declares no positions, so under version three the reader reads none (D4): the shipped arm would read a supplied pair, and only declining to read it keeps a user message from owning or being owned in association |
| `assistant/message.data` is `{turn, step, message, stream, usage?, interrupted?}` (`types.d.ts:309-317`) | `data.message.content`, `data.turn`, `data.step` | Field-compatible; `stream`, `usage`, `interrupted` unread by design |
| `tool/result.data` is `{turn, step, message, error?, meta?}` (`types.d.ts:351`) | `data.message.content` | Field-compatible; `error` and `meta` unread |
| `text.text` (`dsh-llm/lib/types/types.d.ts:39-98`) | `text` | Compatible |
| `reasoning.text` | `text` | Compatible |
| `image.attachment` | none; `[image omitted]` | Compatible |
| `tool-call.id`, `.name`, `.arguments` | `id`, `name`, `arguments` | Compatible |
| `tool-result.toolCallId`, `.content` | `toolCallId`, `content` or `text` | Compatible |
| `file` (`types.d.ts:66-70`) | not named | A counted unrecognized block that keeps its siblings under the existing rule; the delta rules it so, and a later change may show it as an omission marker |
| `Message.id`, `.source` | not read | Inert |

**Reads (b) and (c), for later changes, each cited by package, file,
digest and line:** (b) the append and replace writer paths and what a
persisted replacement copy carries (U1), which gates no rule and may
warrant a marker-aware presentation; (c) the seed and fork path and
inherited identities (U2), which lifts the association gate.

**Read (d), for this run's design council at its next sitting:** the
declaring type and per-type validation branch that fix the permission of
`sourceEventSeqs` on every type but `assistant/message` (U4), which gates
no rule and completes E4's table. The council seat that reaches the
writer performs it, cited by package, file, digest and line, and the
chief enters each of the 55 cells as permitted, required or forbidden by
measurement; a council whose seats all report no reach records that in
D2's reach table and leaves the cells unread. Neither outcome changes a
delta scenario, a task or a line of the implementation, and the design
cannot return the change on it (Context).

**Scenarios a later change writes:** seeded scenarios in which inherited
and new embedded and dedicated calls and results suppress a genuine
duplicate once and keep unrelated or ambiguous identities; if (b) warrants
it, an append-plus-replacement presentation pinning displayed rows, order
and diagnostics, never a removal.

**Code seam:** the gate is one condition in `project_dsh` before
`associate_dsh_tools`; lifting it deletes the condition and adds the seeded
tests. Nothing else in this design moves.

## Risks / Trade-offs

- [A seeded or unattested version-three session shows each embedded
  tool-call copy beside its dedicated call] → The withheld pass shows
  more, never less; the fleet's driver never seeds a session; the gate is
  one condition and the lifting read is named (D13). A wrong suppression on
  remapped identities would hide a block whose twin is displayed; the gate
  prevents even that.
- [A replace-marked row displays whatever the writer persisted, which may
  repeat earlier content] → The version-zero compaction scenario already
  accepts this under the audit rule; the reader parses no marker and
  removes nothing; a later read of the append and replace paths may add
  presentation.
- [The writer reads are one seat's, run-local, and this box cannot verify
  them] → Cited by digest and line (D2), checked against the repository
  wherever they touch the reader, and named as re-verifiable: a later
  change re-takes the digests from a reaching box (D2 ruling). The
  controller's corroboration on the host reads the fleet's sessions through
  the built reader and cannot turn a refusal into an admission.
- [Text streamed in failed attempts disappears from the version-three
  read] → Counted, not silent; stated in the delta and the decision; a
  later change may project attempt streams now that their element shape is
  cited (E3).
- [Per-version dispatch adds an enum through the row loop, a second quiet
  list and a header flag] → One argument, one list, one `Option<bool>` and
  one condition; the set-relation test keeps the two lists honest; the
  alternative admits version-zero-only rows under a writer that cannot
  emit them.
- [A second admitted version widens the surface for crafted files] →
  Admission still requires ownership, bounded UTF-8 acquisition and an
  exact header; the version-three arms are the version-zero arms with the
  same bounded block traversal; no new parser.
- [Two shipped version-zero behaviours change: zero-digit underflow
  tokens stop passing as zero, and a missing or duplicated `version`,
  `time` or `time0` token stops passing on the parsed value] → Both are
  corrections toward requirements the living text already states, said so
  in the proposal and the decision rather than hidden under "unchanged";
  no writer of this fleet emits either shape, because JSON serialisation
  writes a zero as `0` and never repeats or escapes an ASCII member name;
  every affected token is pinned by a regression in D12.
- [The decision number is contested by two other branches] → Re-read at
  the PR; the file and its README row are written by the implementation
  seat, not reserved here.

## Migration Plan

1. Tasks break D3, D4, D8, D9, D10, D11 and D12 into work in dependency
   order: the version type, header flag and token check; dispatch and the
   second quiet list with its relation test; the association gate; the
   projector tests; the discovery tests; the CLI and TUI end-to-end tests;
   the decision and its README row; guides only where a line says "version
   zero only" (none found at this sitting).
2. Implementation in `crates/brokkr-view/src/transcript.rs` and the four
   test suites; no `ui.rs`, protocol, contract, policy, reference or
   fixture change.
3. Gates in the box (D12); exact coverage outside it, recorded pending;
   activation commit unsigned; nothing pushed.
4. Controller: remote CI on the final head, host corroboration against the
   version-three sessions (reading, never committing), review, landing,
   closure of #279 with the seed-path change opened on D13.

Rollback: revert the activation commit. Version-zero files read as before
except for the two exactness corrections of D3, which the revert also
undoes, and version three returns to the header refusal. No data migration
exists to undo.

## Open Questions

- Whether `deliverables/presented` carries operator-facing content that a
  later change should display. Quiet is the measured default (non-surface);
  displaying it would be a new capability, not a change to this design.

## Validation of this sitting

Read the dialect through `openspec instructions design --change
admit-dsh-session-v3 --json`; it declares only `design.md`, and no workflow
runner was invoked. Read the proposal, the capability delta and both
council positions completely, and reconciled every material claim in D1.
Confirmed from `recipes/triage/policy.json` that `upstream` from design
parks after three specification visits, which have been spent, so this
sitting designs to the specification and reports `drafted`. Reconfirmed
this box cannot reach the writer (no `dsh`, `volta`, `npm`, `cargo` or
`~/.dsh`; HOME `/runtime/home`). Verified against the tree every reader
fact this design cites: `dsh_quiet_event`'s 46 names, `dsh_row`'s arms and
the two citation-grammar `Refused` returns, `project_dsh`'s header refusal
with zero counts and its whole-read refusal keeping the counts,
`associate_dsh_tools`' keys, `dsh_message_blocks`' reads, `notices` and
`explanation_for`, the discovery constant and the both-names test, and the
string and notice pins in the `transcript-command` and `transcript-tui`
requirements. Strict OpenSpec validation was run over the whole tree after
authoring.

**Revision at the fourth specification sitting.** On the analyze judge's
return, the specify seat lifted the payload refusal and revised this
design to the lifted delta: the Context, Goals, D1, D4 to D8, D10 to D13,
the Risks, the Migration Plan and the Open Questions; D2, D3 and D9 stand
as the council recorded them. The council's next sitting re-verifies this
design against the delta and the tree and owns it from there.

**Revision at the fifth specification sitting.** On the clarify judge's
four record findings, the specify seat corrected this design against the
tree and the delta: the Context, D1 (the envelope row and a new row for
the user-position inference), D2 E3 and E4 (the base envelope and the
conditional members' per-type table), D3 (the signature rule, the token
binding and the compatibility statement), D4 (the version condition on
user positions), D5 Q1, D12 (the new regressions), D13 (row one), the
Risks and the Migration Plan. Every reader fact cited was verified in the
tree at this sitting; nothing was re-measured from the writer, and the
envelope correction is a reconciliation of citations already in the
record. The council's next sitting re-verifies this design against the
delta and the tree and owns it from there.

**Revision at the sixth specification sitting.** On the clarify judge's
four further record findings, the specify seat corrected this design
against the adopted report and the tree: the Context, D1 (the §3.4 row),
D2 E3 and E4 (the measured-versus-inferred table for the conditional
members and the new U4), D3 (one helper with its own grammar, a predicate
per site, and the time sites' nonzero integers judged on the parsed
value), D5 Q1 and Q4, D12 (the helper, time-exactness and
version-zero-regression bullets, the last naming the six directly-calling
tests that adapt and the one matrix that is extended) and D13 (read (d)).
Every reader fact cited was verified in the tree at this sitting,
including the six direct helper calls in `tests.rs`; nothing was
re-measured from the writer, and the E4 correction withdraws an inference
rather than adding a measurement. The council's next sitting re-verifies
this design against the delta and the tree and owns it from there.

**Revision at the seventh specification sitting.** On the clarify judge's
third visit of CLARIFY-279-5, the specify seat changed no cell's standing
and re-measured nothing: the Context, D1 (the §3.4 row), D2 (the reach
table's seventh-sitting row, E4's four-kinds sentence and U4's owner and
sitting), D5 Q1 and Q4, D12 (the writer-validity bullet, extended to a
tool result with a valid citation and a `null` on the call) and D13 (read
(d) moved from a later change to the council's next sitting) record the
refusal and the assignment that proposal S6 decides. Reconfirmed this box
cannot reach the writer by any route (no `dsh`, `volta`, `npm`, `cargo`,
`~/.dsh`, npm cache or network). The council's next sitting performs read
(d) if any of its seats reaches the writer, re-verifies this design
against the delta and the tree, and owns it from there.

**Revision at the eighth specification sitting.** On the clarify judge's
fourth visit of CLARIFY-279-5 and the operator's ruling that answers it,
the specify seat changed no cell's standing and re-measured nothing: the
Context, D1 (the §3.4 row), D2 (the reach table's specify row and U4),
D12 (the unread-cell bullet) and this record follow the proposal's
reconciled growth rule and refusal (S2, S5 rulings 1 and 8, S6, S8).
Reconfirmed this box cannot reach the writer by any route (no `dsh`,
`volta`, `npm`, `cargo`, `~/.dsh`, npm cache or network). Verified in the
tree that `project_dsh` reads only `seq` and `ignorable` from a quiet,
counted-omission or unrecognized row (`transcript.rs:2011-2032`) and
that `dsh_citations` is reached from the message and `tool/result` arms
only (`:2412`, `:2474`), which the new scenario relies on. The council's
next sitting performs read (d) if any of its seats reaches the writer,
re-verifies this design against the delta and the tree, and owns it from
there.
