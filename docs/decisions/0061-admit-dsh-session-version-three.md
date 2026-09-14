# 0061 — Admit DSH session format version three

Status: proposed
Date: 2026-09-14

## Context

Decision 0055 ruling 3 admitted "DSH numeric on-disk version zero only"
for the shared local transcript read. Issue #279 asks why the reader
refuses the sessions the fleet's newer DSH core writes, and this change
admits version three beside version zero. That widens what 0055 ruling 3
decodes, so this proposal supplements ruling 3 without editing it: the
ruling keeps its meaning, and a new number carries the second admitted
version and the evidence standard a later version must meet.

The evidence is the run's local read of the installed writer, recorded
in the change's design `openspec/changes/admit-dsh-session-v3/design.md`
section D2: the `@deepseek-ai/dsh` 0.1.5-rc.1 core installed on the
fleet's host, whose session packages at 0.1.5-rc.2 were read and cited by
package, file, SHA-256 digest and line. D2's digest table names those
files, including `dsh-session/lib/types/known-event-types.js`,
`dsh-session/lib/types/types.js`, `seq-ranges.js`, `surface.js`,
`dsh-session-format-catalog/lib/index.js`,
`dsh-session-format-v2-to-v3/lib/index.js`,
`dsh-session/lib/types/types.d.ts`, the 0.1.5-rc.2 `dsh-llm` message and
block definitions, and `dsh-session-persistence-jsonl/lib/worker.cjs`.
Reach is an operation the reading seat performs at the file it cites,
never inherited from another seat or sitting, and the design records what
the read reached and what it did not.

The payload mapping onto the reader is design section D13: the
field-by-field table from the read message, block and event definitions to
`dsh_row` and `dsh_message_blocks`, including the `file` block the reader
does not name and the `turn`/`step` a version-three `user/message` does
not declare. The specification and its scenarios are the change's
`openspec/changes/admit-dsh-session-v3/specs/` deltas.

**Rulings.**

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

## Consequences

This is an additive reader change with no stored-data migration and no
frozen contract, schema or corpus edit. Version-zero files read as before
except for the two corrections ruling 7 states toward requirements the
living text already carried; versions 1 and 2 stay refused because no
core of this fleet wrote them to disk under an admitted name and no seat
read their codecs. Under version three a seeded or unattested session
shows each embedded tool-call copy beside its dedicated call, which
shows more, never less; the association gate is one condition and the
seed-and-fork read that lifts it is named in the change's D13. Read (d),
the per-type permission of `sourceEventSeqs` on the 55 types other than
`assistant/message`, stays unread with no rule resting on it and the read
that records it named, which the operator's ruling makes a complete
outcome rather than a pending one. Only the operator accepts this
proposal; filing it does not certify dependency admission, host
corroboration, remote CI or publication.
