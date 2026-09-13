## Context

See proposal.md — Why for the motivation. The facts that shape this design:

- **The reader at `d330765`.** Discovery in `crates/brokkr-cli/src/ui.rs`
  tries `session.v3.jsonl` then `session.jsonl` and treats the first name
  whose opening row is a `session` object with depth zero or omitted as the
  directory's sole candidate; version is deliberately not consulted there.
  Content admission in `crates/brokkr-view/src/transcript.rs` (`dsh_header`)
  admits an exact numeric zero, cross-checking the raw `version` token
  against the parsed value. Row dispatch (`dsh_row`) is keyed on the `type`
  string alone: four content kinds, `assistant/chunk`, the three packed
  storage rows, the 46-name quiet list from archived design D6 (#222), and a
  required-unknown rule for every other type. Nothing in the projector
  knows which version admitted the file.
- **The living rules that move.** "Discovery identifies one owned local
  file" (the admitted version), "DSH sessions expose assembled or
  provisional content once" (fragments, packed rows, citations) and
  "Partial records and read failures remain distinguishable" (row
  classification "after version-zero header admission"; quiet kinds
  "enumerated from evidence in design"). This design is the evidence that
  last sentence asks for.
- **The proposal at `23fafd1`.** Authored from a box that could not reach
  the writer, it concludes refuse-and-defer, and its S4 names the release
  condition: a capture of the installed packages, or a seat that can read
  the install, after which "this change is returned to specification with
  the finding". The design council produced that seat (D2). The condition
  S4 set is met inside this run.
- **This sitting's box** is the specify seat's box: HOME `/runtime/home`,
  user `runner`, no `dsh`, `npm`, `volta` or `cargo`, no `~/.dsh`, no
  network. It cannot re-read the writer or run Cargo. OpenSpec 1.12.0 and
  Git are present.
- **Constraints carried unchanged:** frozen `contracts/`, `policy/`,
  `reference/`, `fixtures/`; the global `dsh` install and its sessions are
  read-only to every seat and never become test inputs (decision 0032); a
  semantic change carries a `Status: proposed` decision.

## Goals / Non-Goals

**Goals:**
- Rule, on the run's own writer evidence, that version 3 is admitted beside
  zero, and say exactly which of its meanings are measured, which are ruled
  quiet, and which stay refused.
- Shape the projector so that a version-3 row can never inherit a
  version-zero disposition by string equality, without building a
  per-version projector.
- Hand the returned specification the exact revisions its proposal,
  capability delta, decision and tests need so the change is coherent end
  to end.

**Non-Goals:**
- No replay of `surfaceOp`, no reconstruction of the model-visible surface,
  no expansion of embedded streams into fragments, no assembled message the
  writer did not write.
- No migration of version-zero files, no reading of the 17 installed
  sessions from a seat, no committed capture of the install.
- No `system` role, no CLI or TUI observable change, no contract, schema,
  policy or fixture edit.
- No re-measurement of the writer here: this box cannot, and the design
  says so rather than pretending.

## Decisions

### D1 — Reconcile the council by claim

| Position / claim | Resolution and evidence |
|---|---|
| Simplicity: the installed writer is reachable and was read; its source answers the six questions. Robustness and the proposal: the writer is unreachable from the box. | **Both are true of their seats; adopt the read as the run's primary evidence.** Reachability is a property of a seat's box, not of the host: this sitting reconfirms the specify box (no `dsh`, no `~/.dsh`), while the simplicity seat resolved `/home/vyanakiev/.volta/bin/dsh` to 0.1.5-rc.1 and read the packages beneath it. The commission names the installed writer as the only admissible primary source; the run now holds a read of it, recorded with digests and line citations in `.forge/design/positions/simplicity.md`, which is the same class of run-local evidence the #222 capture was (never committed, cited by digest and line from the change). D2 records its reach and its limits. |
| Simplicity: admit exact numeric `3` beside `0`. | **Adopt, with the exactness guard extended (D3).** The zero contract's raw-token cross-check is kept for three; robustness §5 is the reason. |
| Simplicity: a union quiet list is enough; per-version dispatch is machinery the framing does not demand. Robustness §2: widening the gate on kind-only dispatch lets a v3 name inherit a v0 disposition by string equality. | **Combine: one projector, one vocabulary per version (D3).** No second projector and no registry; the quiet and content predicates take the admitted version, so a name outside the admitted version's measured vocabulary is a required unknown exactly as any unknown is today. The union is rejected because it admits `assistant/chunk` and packed rows under version 3 without evidence that the 0.1.5 writer emits them, and quiet-lists the v3-only names under version zero, whose writer never emitted them. The cost is one enum and one extra argument. |
| Simplicity: `assistant/chunk` is gone because the stream is embedded in `assistant/message` and `assistant/attempt`; the reader's existing `data.message` path already reads whole messages. | **Adopt the measurement; state the behaviour change in the requirement (D5).** The fragment rule's text stands and its behaviour changes for version 3: an interrupted step retains one finalized message, not fragments. That sentence must live in the living requirement, not in a design note, as simplicity's own risk 4 says. |
| Simplicity: `assistant/attempt` is quiet. | **Reject quiet; rule counted omission (D5, D7).** An attempt carries model output text the writer did not surface. Calling it operational metadata would silently drop text from an audit read; counting it as an unrendered recognized record is what the reader already does for a recognized envelope whose nested variant it cannot project, and it keeps the read available. |
| Simplicity: `system/message` is the quiet successor of `request/context`; a displayed `system` turn is new capability. | **Adopt.** The v2→v3 migration's `emitSystem` places the rendered system prompt as surface node 0; the version-zero precedent for the prompt copy is quiet. |
| Simplicity: no `surfaceOp` handling; the writer itself calls the surface the wrong source for a human transcript. Robustness §5: preserve the raw/parsed cross-check for anything that gates suppression. | **Adopt both (D6).** Nothing new gates suppression: the citation grammar is unchanged and the reader never parses `surfaceOp`. The cross-check therefore applies to `version` alone, and the requirement says so. |
| Simplicity: `isSeeded` is fork lineage, separate from delegation depth; it needs only the inert list. Robustness §4: inertness must cover null and absent. | **Combine.** Inert, pinned over true, false, string, object, null and absent (D4 Q3). |
| Robustness §1: the newest-name-wins exclusive candidate can make a readable older sibling unreadable when the newer name carries a refused version; pin the behaviour with mismatched versions. | **Adopt the pin, keep the exclusive rule (D8).** Falling through to the older file would present superseded evidence as if it were the newest; the honest read is a refusal that names the path. Under this design the v3/v0 pairing reads the v3 file, so the scenario uses a future version. |
| Robustness §3: gate the six questions independently so a partial capture cannot license an all-or-nothing admission. | **Adopt as the shape of D4 and D7.** Each question records its measured answer, its evidence and what remains unmeasured; unmeasured parts are refused per row (persistence rows, untranscribed names), not waved through with the version. |
| Robustness §6: the evidence-and-growth rule is a numbered decision, not change prose. | **Adopt (D9).** A second version is now admitted, which is exactly the semantic change 0055 ruling 3 did not cover; the proposed decision carries both the admission and the evidence standard. |
| Simplicity: cut the four refusal-shaped scenarios and the prerequisite; keep one change directory; no separate evidence document. | **Adopt, except the version-zero baseline scenario.** "Version zero already rules the three sampled types" stays true and useful under version zero. The proposal's S1 probe table becomes the record of *why two seats could not read the writer*, not the change's conclusion; S4's prerequisite is discharged. The six answers live here; the digests live here and in the decision. |
| Proposal S5: no numbered decision. | **Reject, superseded by the admission.** 0055 ruling 3 decodes "version zero only"; admitting three changes what the reader decodes. |

Two of these findings fault the proposal rather than the design, so this
sitting reports `upstream` (D11) and writes only the dialect-declared
`design.md`.

### D2 — Evidence: the run's writer read is primary, and its reach is stated

**What the simplicity seat read.** On 2026-09-13, on this host, `which dsh`
resolved to `/home/vyanakiev/.volta/bin/dsh`, `dsh --version` printed
`0.1.5-rc.1`, and the install root under
`/home/vyanakiev/.volta/tools/image/packages/@deepseek-ai/dsh/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/`
holds `dsh-session`, `dsh-session-format`, `dsh-session-format-catalog`,
`dsh-session-persistence-jsonl` and three migration packages, each
`0.1.5-rc.2`. Every access was a read. The digests it recorded:

| Source file (under `.../node_modules/@deepseek-ai/`) | SHA-256 |
|---|---|
| `dsh-session/lib/types/known-event-types.js` | `7bcf6e061a34b6107896b09ec048f5e62420ce1505191c1d63910765e014d909` |
| `dsh-session/lib/types/types.js` | `27de3ecc17fe395856b6182362ecfbe7f7b83a90238bbd09ccfa55c007193725` |
| `dsh-session/lib/types/seq-ranges.js` | `68a127c76affa98edeeb50e302eb43f154f4d24f7d04cb95ba8b323e88f3d09e` |
| `dsh-session/lib/types/surface.js` | `aad7aaabe6cd9b39ae4cc3b50a2873c9b5d73b69d929051f31b18ecc13647c72` |
| `dsh-session-format-v2-to-v3/lib/index.js` | `2d35e1e0ed497af569d5735fc590187de1568489cfe60d070b5f61330cd5a338` |
| `dsh-session-format-catalog/lib/index.js` | `bf4bde9e6563d7793f820c4a1b3141f6527283dd6c58f16bf43a67bc557cc48c` |

**Why it is primary.** The commission admits one primary source, the
installed 0.1.5-rc.1 writer, and forbids sampling as a substitute. This
read is of that source, on this host, at that version. Version zero was
admitted on evidence of the same class: a run-local capture that was never
committed and is cited from the archived #222 proposal (S11) and design
(D6) by file, line and digest. This design cites the same way. The two
positions that could not reach the writer are not contradicted by it; they
describe their boxes.

**What this sitting verified against the repository.** Every claim of the
read that touches the reader was checked here: the 46-name quiet list and
the five content kinds match `dsh_quiet_event` and `dsh_row`;
`request/context`, `todo/write` and `turn/end` are quiet today;
`tool/code-dispatch` and `tool/code-dispatch-start` are in the version-zero
list and reported absent from the v3 set; `dsh_citations` accepts exactly
individual integers and two-integer ranges; the reader never reads
`surfaceOp`; `dsh_header` ignores every header field but `type`,
`delegationDepth` and `version`. Nothing the read reports contradicts the
repository's own record of version zero.

**What the read did not establish.** These are named so that the returned
specification refuses them rather than the design papering over them:

1. **The catalogue was not transcribed.** The read reports 58 names and an
   eight-name difference from what the reader handles, with three reader
   names (`assistant/chunk`, `tool/code-dispatch`,
   `tool/code-dispatch-start`) absent from v3. The reader handles 51 names;
   51 − 3 + 8 = 56, not 58. Two names are unaccounted for. The arithmetic
   reconciles if the two `tool/code-dispatch*` names are in fact still in
   the v3 catalogue beside their PTC successors, or if two further names
   were not transcribed; only a transcription against digest `7bcf6e06…`
   settles which. Until then any name outside D7's enumeration is a
   required unknown under version 3 (refuses without `ignorable: true`).
   This fails loud on exactly the sessions that carry such a name and
   invents nothing.
2. **`dsh-session-persistence-jsonl` 0.1.5-rc.2 was identified but not
   read:** its row kinds, header write and file naming are uncited.
   Consequence: no packed storage row is admitted under version 3 (D7).
   The filename itself is already admitted by #278 on the operator's
   ruling.
3. **The block vocabulary inside `message.content`** (text, reasoning,
   tool-call, tool-result, image under version zero) was not cited from the
   0.1.5-rc.2 `types.d.ts`; the read cites the envelope fields the four
   content kinds carry. Consequence: an unknown block type under version 3
   is a counted omission that keeps its siblings, the existing rule;
   nothing is invented.
4. **The element shape of the embedded `stream`** on `assistant/message`
   and `assistant/attempt` was not cited. Consequence: the stream is never
   expanded (D5).
5. **The relation between a v3 citation and its owning `seq`** was not
   cited (the grammar was). The reader's "earlier than the owning event"
   rule stays; a v3 row that violates it refuses, as any row would.
6. **The `.d.ts` files cited by line** (`types.d.ts`, `surface.d.ts`) are
   not in the digest table; their `.js` siblings from the same package
   version are.

The controller's corroboration on the host (reading the 17 sessions after
the writer) is where items 1, 2 and 5 surface if they bite; the design does
not wait on it and no seat performs it.

### D3 — Admission: the set is {0, 3}, spellings are exact, dispatch is keyed on the admitted version

**Set.** The admitted DSH version set is closed and exactly `{0, 3}`. Zero
by the #222 capture; three by D2. The catalog package declares
`currentVersion: 3` with codecs `v0..v3` and lossless migrations
`v0→v1→v2→v3` (`dsh-session-format-catalog/lib/index.js:34-46`);
`SESSION_FORMAT_VERSION = 3` at `dsh-session/lib/types/types.js:54` and
`lib/index.js:56`. Versions 1 and 2 are not admitted: no core of this fleet
wrote them to disk under a name discovery admits, and no seat read their
codecs. A version outside the set refuses exactly as today.

**Spellings.** The existing zero rule stays. For three: the parsed value
equals three and the recorded token, when the raw top-level token is found,
spells an exact integer three, meaning its mantissa digits with the decimal
point and leading and trailing zeros removed are exactly `3`. So `3`,
`3.0`, `3e0`, `30e-1` and `0.3e1` admit; `"3"`, `true`, `null`, `3.1`,
`-3`, `3e1`, and the rounding artefacts `2.9999999999999999` and
`3.0000000000000001` (parsed value three, token not three) refuse.
Implementation: generalize `zero_number_token` into an exact-integer-token
check over a digit signature (empty for zero, `3` for three) and have
`dsh_header` return `Option<DshVersion>` instead of `bool`. Alternative
rejected: parsed-value-only for three, because it drops the defence the
zero path already has (robustness §5) and would admit a token the writer
never wrote.

**Dispatch.** `dsh_row` takes the admitted `DshVersion`. Under `Zero` it is
unchanged. Under `Three`: the four content kinds project through the same
code, because their envelope fields are the ones the read cites
(`user/message` content in `data`; `assistant/message` and `tool/result`
in `data.message`; `tool/call` with `callId`, `name`, `arguments`;
`types.d.ts:309-338`); `assistant/attempt` is a counted omission; the
version-3 quiet set of D7 is quiet; every other type, including
`assistant/chunk`, the three packed rows and the two `tool/code-dispatch*`
names, is a required unknown. Same-name reuse across versions is licensed
per name by D7's evidence column, never by the string. Alternatives
rejected: a per-version projector or registry (nothing else differs between
the versions the reader reads; the shared content projection is one code
path by measurement), and the union list (D1).

**Filename and header stay independent facts.** `session.jsonl` with a
version-3 header now reads under the version-3 rules; `session.v3.jsonl`
with a version-zero header reads under the version-zero rules; either name
with version 4 refuses. Discovery still infers nothing from the header and
admission nothing from the name.

### D4 — The six questions of #279, answered on the read

Each row names the measured answer with its evidence (as recorded by the
simplicity seat unless marked *repo*), the reader consequence, and what
stays unmeasured. Unmeasured parts are refused by D7, not admitted with the
version.

| # | Question | Measured answer and evidence | Reader consequence | Unmeasured, therefore refused |
|---|---|---|---|---|
| 1 | Record types and shapes | `SESSION_FORMAT_VERSION = 3` (`types.js:54`, `lib/index.js:56`); catalogue `currentVersion: 3`, codecs `v0..v3`, migrations `v0→v1→v2→v3` (`dsh-session-format-catalog/lib/index.js:34-46`); the vocabulary is the generated set in `known-event-types.js` (digest `7bcf6e06…`), reported as 58 names; payload shapes are `SessionEventMap` in `types.d.ts`; surface-eligible types are exactly `system/message`, `user/message`, `assistant/message`, `tool/result` (`types.d.ts:413`); all others are log-only (`deriveEventMessage` returns null for non-surface types, `surface.d.ts:52-64`). *Repo:* the reader handles 51 names, of which the read reports 48 shared. | Per-version vocabulary (D3); dispositions in D7. | The full transcription (D2 item 1); the persistence package's row kinds (D2 item 2); the block vocabulary inside `message.content` (D2 item 3). |
| 2 | `assistant/chunk` absent | Removed, not renamed and not conditional: the v1→v2 package is the assistant-stream migration; in v3 the timed stream is embedded, `assistant/message` carrying `message`, `stream`, optional `usage` and `interrupted?: true` (`types.d.ts:309-317`), `assistant/attempt` carrying a stream with no surface message (`types.d.ts:323-327`); "a turn cancelled mid-stream finalizes its delivered text/reasoning prefix as this event with `interrupted: true`" (`types.d.ts:303-307`). | Whole messages only under version 3; the fragment rule's behaviour changes and the requirement says so (D5). `assistant/chunk` under version 3 is a required unknown. | Stream element shape (D2 item 4): never expanded. |
| 3 | `isSeeded` | A required v3 header boolean meaning the session contains a fork-inherited event prefix (`types.d.ts:72-76`); the prefix length is `inheritedEventCount` session state, not header metadata (`types.d.ts:105-109`); delegation depth is a separate field (`types.d.ts:82-87`). *Repo:* ownership is `type: session` plus depth, and `dsh_header` reads no other field. | Inert for ownership, depth and admission; joins the inert list; pinned over true, false, string, object, null and absent. No provenance or delegation claim rests on it. | Whether inherited prefix events keep their originating `seq`/`turn`/`step` is not cited. It does not bear on version 3, where no chunk exists to be cited across a seed boundary, and under version zero the uniqueness rule already refuses to suppress on an ambiguous identity. |
| 4 | `surfaceOp`, `sourceEventSeqs` | `surfaceOp` is `'append' \| {op: 'replace', startSeq, endSeq}`, required only on the four surface types (`types.d.ts:429-441`). `sourceEventSeqs` is individual non-negative integers plus inclusive `[start, end]` pairs, strictly increasing (`seq-ranges.js`, digest `68a127c7…`): the grammar `dsh_citations` accepts. The rise to about 21% of rows is the writer placing every surface message on the surface with explicit markers. The writer's own contract: the surface "is the wrong source for a human transcript — a landed replacement would erase conversation the user already saw. Append-origin events are that transcript's durable source material; replacement copies stay model-only" (`surface.d.ts:27-33`). | No change: the citation rules apply as written and, with no chunk rows, suppress nothing under version 3; `surfaceOp` is never read; 0055 ruling 3's "never rewrites audit history" is the writer's own transcript rule (D6). | Whether a v3 citation can be non-earlier than its owning `seq` (D2 item 5): such a row refuses under the existing rule. |
| 5 | `system/message`, `todo/write`, `turn/end` | `system/message` is the rendered system prompt as surface node 0, replacing the head (`types.d.ts:288-294`; v2→v3 `emitSystem`, `dsh-session-format-v2-to-v3/lib/index.js:780-810`), the successor of version zero's `request/context`. `todo/write` and `turn/end` are in both catalogues and log-only under v3 (not in the four surface types). | All three quiet under version 3. `system/message` under version zero stays a required unknown: the 0.1.2 catalogue does not contain it. | Nothing the quiet disposition depends on: quiet reads no payload. |
| 6 | Superset? | **No.** Versions are physical generations joined by migrations, not sets: v3 removes `assistant/chunk` (and, per the read, the two `tool/code-dispatch*` names), adds at least eight names, moves the prompt from `request/context` to `system/message` and the stream from chunk rows into the message. The four content kinds keep the fields the reader reads (`types.d.ts:309-338`), which is why one projector serves both. | Admit `3` beside `0` with a per-version vocabulary; no per-version projection. Every version-zero meaning is preserved *for version-zero files*; version-3 files carry the differences above, stated in the requirement. | Whether the message block vocabulary moved (D2 item 3). |

### D5 — Fragments under version 3: the writer finalizes, the reader never assembles

Under version zero a step's text arrives as `assistant/chunk` rows or packed
runs, each one logical event, and a readable assembly that cites them
suppresses them. Under version 3 the writer persists no fragment rows: a
completed step is one `assistant/message`, and a step cancelled mid-stream
is one `assistant/message` with `interrupted: true` whose `message` is the
delivered prefix the writer finalized. The reader projects `data.message`
as it does today. Nothing is concatenated, because nothing arrives in
pieces.

Rulings the requirement must carry:

- The sentence "fragments SHALL not be concatenated into an invented
  assembled message" is unchanged, and its version-3 behaviour is stated
  beside it: a version-3 interrupted step retains one finalized message,
  the writer's own, and no fragment rows exist to retain. The scenario "An
  interrupted DSH step retains its chunks" stays as a version-zero
  scenario; a sibling scenario pins the version-3 shape.
- `interrupted` is quiet metadata: the reader neither hides the message nor
  adds a marker. Alternative rejected: an "interrupted" notice or block,
  because version zero shows an interrupted step's retained fragments with
  no marker either, and inventing a display state the writer did not
  record is the invention the reader refuses.
- `stream` and `usage` on `assistant/message` are quiet metadata, never
  expanded: expansion would either duplicate the message or fabricate
  fragments from an uncited shape.
- `assistant/attempt` is a counted omission (`DshRow::Omission`):
  recognized, no content, `unrecognized_records` plus one, read available.
  The text streamed in a failed or retried attempt is therefore not part of
  the version-3 audit read, and the requirement states that as a measured
  difference from version zero, where such deltas would have been chunk
  rows. Alternatives rejected: quiet, because it would drop model output
  text silently; displayed as fragments, because the stream shape is
  uncited and the writer marks the attempt non-surface.
- `assistant/chunk` and the packed rows under version 3 are required
  unknowns. They are version-zero evidence; the 0.1.5 catalogue does not
  contain the chunk type and the persistence package was not read (D2 item
  2). A version-3 file carrying them is refused, not decoded by a code path
  its writer cannot have exercised.

### D6 — `surfaceOp` and `sourceEventSeqs`: grammar shared, surface never replayed

The citation grammar the reader validates is the grammar the v3 writer
emits (D4 Q4), so `dsh_citations` applies unchanged under both versions,
including the whole-read refusal for an invalid field. Suppression, which
only ever removes uniquely cited earlier chunks in the same turn/step, has
nothing to remove under version 3 and does nothing; a version-3 citation to
a non-chunk earlier event, or to an unobserved `seq`, is the existing "no
suppression" outcome, not a refusal. `surfaceOp` stays unread: the writer
itself says replacement copies are model-only and append-origin events are
the transcript's durable material, which is what 0055 ruling 3 already
rules from the reader's side. No new field gates meaning, so the raw-token
cross-check applies to `version` alone (D3) and the requirement says which
field it covers.

Alternative rejected: a `surfaceOp`-aware suppression model. It would
import the model's rewritten view into an audit read, and the writer's own
contract calls that view the wrong source.

### D7 — Dispositions under version 3, enumerated from evidence

"Recognized quiet event kinds SHALL be enumerated from evidence in design."
This is that enumeration for version 3; version zero's is unchanged from
archived D6.

| Type under a version-3 header | Disposition | Evidence |
|---|---|---|
| `user/message`, `assistant/message`, `tool/call`, `tool/result` | Content, projected as today | Envelope fields cited (`types.d.ts:309-338`); surface set (`types.d.ts:413`) |
| `assistant/attempt` | Counted omission | Stream with no surface message (`types.d.ts:323-327`); D5 |
| `system/message` | Quiet | Rendered system prompt (`types.d.ts:288-294`; v2→v3 `emitSystem`); `request/context` precedent |
| `deliverables/presented`, `feedback/message-delete`, `feedback/message-put`, `subagent/catalog`, `tool/ptc-dispatch`, `tool/ptc-dispatch-start` | Quiet | In the v3 catalogue and outside the four surface types; `tool/ptc-dispatch*` succeed `tool/code-dispatch*` (v2→v3 PTC migration) |
| The 44 version-zero quiet names other than `tool/code-dispatch` and `tool/code-dispatch-start` | Quiet | Shared by both catalogues per the read; log-only under v3 (outside the surface set); quiet reads no payload, so the disposition does not depend on payload shape |
| `assistant/chunk`, `text-chunks`, `reasoning-chunks`, `tool-call-chunks`, `tool/code-dispatch`, `tool/code-dispatch-start` | Required unknown (refuse unless `ignorable: true`) | Reported absent from the v3 catalogue; persistence rows unread (D2 item 2); D5 |
| A later `session` row | Unknown envelope under the ignorable rule | Unchanged from version zero |
| Any other name, including whatever D2 item 1 leaves untranscribed | Required unknown | D2 item 1; the safe direction |

The version-3 quiet set is therefore the 44 shared names plus the seven
quiet names in rows three and four: 51 names. The implementing seat encodes
it as a second list keyed on `DshVersion::Three`, not by editing the
version-zero list, so that each version's vocabulary can be read and
tested on its own.

### D8 — The exclusive candidate stays exclusive, and the mismatched case is pinned

Discovery keeps the #278 rule: the first admitted name whose opening row is
a session header with valid depth is the directory's only candidate, and
content admission then rules on that file alone. Under this design the
pairing the fleet actually produces, `session.v3.jsonl` at version 3 beside
`session.jsonl` at version zero, reads the version-3 file. The gap
robustness §1 found remains for a future version: `session.v3.jsonl` at
version 4 beside a readable `session.jsonl` refuses with the confirmed
`session.v3.jsonl` path and does not fall through.

That is the ruling, for the same reason the version-zero rule refuses to
rank roots by version: falling through would show older evidence as if it
were the seat's newest transcript. Two scenarios pin it, both in
`crates/brokkr-cli/src/ui/tests.rs` beside the existing both-names test,
which today writes the same header into both files and so proves ordering
but not this: (a) a version-3 header in the versioned name and a
version-zero header in the plain name reads the versioned file; (b) a
version-4 header in the versioned name and a version-zero header in the
plain name returns `unsupported-format` with the versioned path, zero
counts and no turns. No change to `ui.rs`.

### D9 — Proposed decision 0060 carries the admission and the evidence standard

A second admitted version changes what 0055 ruling 3 decodes, so a numbered
decision with `Status: proposed` is filed, supplementing ruling 3 without
editing it, at the next free number, 0060 at this sitting: main's index
holds 0055 and 0057–0059, and 0056 is claimed by the #226 branch's proposed
same-instance resumption decision, so the decision-owning phase claims 0060
in its PR as the index asks. It preserves 0032's ownership,
retention and privacy constraints: no real session is read by a seat,
copied or committed; tests use synthetic rows in test-owned homes. Its
rulings, to be authored by the decision-owning phase:

1. **Admitted versions are measured, closed and named.** The DSH local
   reader decodes on-disk versions zero and three only, each admitted on a
   read of its writer's own source: 0.1.2-rc.1 for zero (#222 capture),
   0.1.5-rc.1 with session packages 0.1.5-rc.2 for three (D2 digests). A
   version joins only through a change recording the writer's package
   identity and files with digests, the record vocabulary, how fragments
   persist, the meaning of every added header field, the meaning of
   `surfaceOp` and `sourceEventSeqs` where carried, a disposition for every
   type, and whether admitted meanings are preserved. A versioned filename,
   a header number and the resemblance of sampled rows are not evidence.
2. **Vocabulary is per version.** A row is classified under the vocabulary
   of the version that admitted its file. A name shared by two versions
   keeps a disposition in each only by measurement; a name outside the
   admitted version's vocabulary is a required unknown. Version-zero
   storage rows and `assistant/chunk` are version-zero only.
3. **Version three persists whole messages.** No fragment rows; an
   interrupted step is one finalized `assistant/message` with
   `interrupted: true`; embedded streams and usage are quiet metadata;
   `assistant/attempt` is a counted omission. The reader assembles nothing
   under either version.
4. **The surface is not the transcript.** The citation grammar is shared;
   suppression remains cited-unique-earlier-same-step chunks only;
   `surfaceOp` is never read. The raw-token exactness cross-check gates
   `version` and no other field.
5. **`isSeeded` is inert.** It neither establishes nor removes ownership,
   does not alter the depth rule and does not supply admission, whatever
   its JSON shape, including null and absent.
6. **Unmeasured parts refuse.** The persistence package's rows and any
   catalogue name not transcribed are refused under version three until
   read; the record says which.

Enforcement bindings: the version matrix, disposition, fragment,
sibling-discovery and seeded tests named in D10, `openspec validate --all
--strict`, and the exact-coverage gate. The README row is added beside the
file.

### D10 — Proof, host-independent, and the gates

All tests use inline synthetic rows in test-owned homes; nothing under
`~/.dsh` or `fixtures/` is read. In `crates/brokkr-view/src/transcript/tests.rs`:

- **Version matrix.** `dsh_header_version_matrix_admits_only_numeric_zero`
  grows into a two-version matrix: admitted `0`, `0.0`, `0e0`, `-0`, `3`,
  `3.0`, `3e0`; refused `null`, `false`, `"0"`, `"3"`, `[]`, `{}`, `1`,
  `2`, `4`, `-1`, `-3`, `0.5`, `3.1`, `3e1`, `2.9999999999999999`,
  `3.0000000000000001` and an absent version; every refusal keeps zero
  counts and the confirmed path.
- **Dispositions.** Under a version-3 header each of the eight new names
  (`assistant/attempt` counted, seven quiet); the six required-unknown names
  of D7 with and without `ignorable: true`; the four content kinds project.
  Under a version-zero header the eight new names are required unknowns and
  `system/message` stays one.
- **Fragments.** A version-3 `assistant/message` with `interrupted: true`
  followed by a partial append projects one assistant turn with the
  recorded prefix and no marker; present `stream` and `usage` change
  nothing; an `assistant/attempt` before the final message counts one and
  shows nothing; a version-3 `assistant/message` whose `sourceEventSeqs`
  cite earlier non-chunk sequences suppresses nothing; an invalid citation
  still refuses.
- **Seeded.** `isSeeded` true, false, `"yes"`, `{}`, null and absent under
  both versions admit; version 3 with depth 1 is not a candidate.
- **Version-zero regressions.** Every existing DSH test unchanged and green.

In `crates/brokkr-cli/src/ui/tests.rs`: the two D8 sibling scenarios and the
filename/version independence pair (`session.jsonl` at version 3 reads;
`session.v3.jsonl` at version 4 refuses). In
`crates/brokkr-cli/tests/transcript_command.rs` and the TUI suite: one
version-3 source through CLI text, CLI JSON and the TUI, projecting; the
refusal paths already covered stay covered.

Gates before the activation commit: `openspec validate --all --strict`,
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings`, `cargo test --workspace
--all-features --locked`, both bundle compiles. Exact coverage runs outside
the box and is recorded pending until its result exists; the gate is not
lowered. Remote CI, publication and closure are the controller's.

### D11 — Upstream finding: what the specification must revise

The proposal at `23fafd1` is not wrong on its evidence; its conclusion is
superseded by evidence this run gathered after it was written, and its own
S4 says that in this event the change returns to specification. This
sitting therefore reports `upstream`. The specify seat adopts
`admit-dsh-session-v3` (it does not re-author it) and revises, in
dependency order:

1. **`proposal.md`.** S1 becomes the record of why the specify and
   robustness boxes could not read the writer and that the simplicity seat
   could, with D2's digests and limits. S2 rules the set `{0, 3}` under D3.
   S3 becomes the six answers of D4 with D2's refusals per row. S4's
   prerequisite is discharged and replaced by the corroboration step the
   controller performs on the host. S5 becomes the proposed decision 0060
   of D9. Impact names `transcript.rs`, the three test suites, the decision
   file and the README row; no `ui.rs`, contract, policy, reference or
   fixture change.
2. **`specs/transcript-reading/spec.md`, "Discovery identifies one owned
   local file".** Admitted set `{0, 3}`; the exact-three spelling rule; the
   evidence paragraph names both writers; the growth rule stays; `isSeeded`
   inert over every shape including null and absent. Replace the scenario
   "A version-3 session is located and refused until its writer is read"
   with one that projects; revise "The filename and the header version are
   independent facts" so `session.jsonl` at version 3 reads and
   `session.v3.jsonl` at version 4 refuses; add the D8 sibling scenario;
   extend the `isSeeded` scenario; keep "Version zero already rules the
   three sampled types"; extend "Missing and mistyped DSH versions are not
   legacy zero" and "A foreign-version DSH root alone keeps its confirmed
   location" with the version-three refusals of D10.
3. **Add MODIFIED "DSH sessions expose assembled or provisional content
   once".** State the version-3 whole-message rule, `interrupted`,
   `stream`, `usage`, `assistant/attempt` and the version-zero-only status
   of chunk and packed rows (D5, D6); add the version-3 interrupted-step,
   attempt and non-chunk-citation scenarios.
4. **Add MODIFIED "Partial records and read failures remain
   distinguishable".** "After version-zero header admission" becomes
   "after admitted-version header admission"; the quiet vocabulary is per
   admitted version and enumerated in design (D7); add the disposition
   scenarios for the eight names under both versions and the
   required-unknown scenario for the six version-zero-only names under
   version three.
5. **Catalogue count.** If any seat on the return can read the install,
   transcribe the 58-name set against digest `7bcf6e06…`, reconcile D2
   item 1, and add any further name to D7's quiet row only if it is
   outside the four surface types; otherwise the requirement names the
   51-name quiet set and states the gap and its refusal consequence.
   Either way the vocabulary is closed and named.

Nothing downstream admits version 3 until these land; `tasks.md` then
breaks D3, D5, D7, D8, D9 and D10 into work.

## Risks / Trade-offs

- [The writer read is one seat's, unverifiable from this box, and the
  digested files are the `.js` siblings of the cited `.d.ts`] → The design
  cites it as run-local evidence of the #222 class, verifies every
  reader-touching claim against the repository, names each unread part and
  refuses it (D2). Corroboration on the host is the controller's step and
  cannot turn a refusal into an admission.
- [Untranscribed catalogue names may be common in real sessions, refusing
  every read that carries one] → A loud, path-confirmed refusal with a
  counted row is the reader's safe default; D11 item 5 asks the return to
  transcribe; the 17 host sessions corroborate before closure.
- [The persistence package was not read; if 0.1.5 still packs rows, every
  such session refuses] → Same safe direction; the catalogue has no chunk
  type to pack, and corroboration surfaces it.
- [Text streamed in failed attempts disappears from the version-3 read] →
  Counted, not silent; stated in the requirement and the decision; a later
  change may project attempt streams once their shape is cited.
- [Per-version dispatch adds an enum through the row loop and a second
  quiet list] → One argument and one list; the alternative admits
  version-zero-only rows under a writer that cannot emit them.
- [A second admitted version widens the surface for crafted files] →
  Admission still requires ownership, bounded UTF-8 acquisition and an
  exact header; version-3 rows pass through the same validators; no new
  parser.
- [Reporting upstream costs a specification and a design visit] → The
  alternative is a design that contradicts its proposal; the recipe returns
  one phase and the visit counts are far from the park threshold.

## Migration Plan

1. Specification revises the change per D11 and validates strictly.
2. Design revisits for coherence; tasks break the work down.
3. Implementation: `DshVersion`, the exact-three token check, per-version
   dispatch, D7's list, the tests of D10, decision 0060 and its README row,
   guides only where a line says "version zero only".
4. Gates in the box; exact coverage outside it, recorded pending;
   activation commit unsigned.
5. Controller: remote CI on the final head, host corroboration against the
   17 sessions (reading, not committing), review, landing, closure of #279.

Rollback: revert the activation commit. Version-zero behaviour is untouched
by construction and version 3 returns to the specified refusal. No data
migration exists to undo.

## Open Questions

- Whether `deliverables/presented` carries operator-facing content that a
  later change should display. Quiet is the measured default (non-surface);
  displaying it would be a new capability, not a change to this design.
- Whether any guide says "version zero only" in a way the implementation
  seat should touch. Grep at implementation; no design consequence.

## Validation of this sitting

Read the dialect through `openspec instructions design --change
admit-dsh-session-v3 --json`; it declares only `design.md`, and no workflow
runner was invoked. Read both council positions completely and reconciled
every material claim in D1. Reconfirmed this box cannot reach the writer
(no `dsh`, `volta`, `npm`, `cargo` or `~/.dsh`; HOME `/runtime/home`).
Verified against the tree: `dsh_quiet_event`'s 46 names and `dsh_row`'s
content kinds match archived D6; `dsh_header` reads only `type`,
`delegationDepth` and `version`; `dsh_citations` accepts only integers and
pairs; discovery's exclusive loop and the both-names test use one header
for both files. `openspec validate --all --strict --no-interactive` passes
on entry (15 items). This sitting writes and commits only `design.md`,
unsigned; proposal, capability delta, code, decisions and frozen bytes are
untouched. The result is `upstream` with `inputs.change:
admit-dsh-session-v3`.
