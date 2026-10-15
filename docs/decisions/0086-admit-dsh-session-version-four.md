# 0086 — Admit DSH session format version four, plain and compressed

Status: proposed
Date: 2026-10-14

Built: built
Amends: 0055, 0061
Supersedes in part: 0062

## Context

Issue #678: every DSH seat's transcript is unavailable in `brokkr
transcript`, the TUI pane, the browser and the DSH console, because the
installed DSH core, 0.2.0-rc.2, writes
`<root>/<project>/session-<uuid>/session.v4.jsonl`, or
`session.v4.jsonl.zstd` under its persistence backend's default encoding,
and Brokkr admits only the version-zero and version-three names and
headers. A working DSH seat also carries no session reference until it
finishes.

The evidence is the run's own read of the writer's published
specification, the 0.2.0-rc.2 packages `@deepseek-ai/dsh-session-format`,
`dsh-session-format-v2-to-v3`, `dsh-session-format-v3-to-v4` and
`dsh-session-persistence-jsonl`, their READMEs and typed declarations,
recorded by package, file, SHA-256 digest and section in the change's
design, `openspec/changes/archive/2026-10-14-678-admit-dsh-session-v4/design.md`
section D2. The event catalogue (`@deepseek-ai/dsh-session`) and the
message and block definitions (`@deepseek-ai/dsh-llm`) were not among the
read packages; D2 records them unread and names the read that lifts each.
The field-by-field mapping is D5; the specification and scenarios are the
change's `specs/` delta.

**Rulings.**

1. **Version four is admitted on the parts read.** The admitted set is
   `{0, 3, 4}`. Four is admitted on the 0.2.0-rc.2 read under decision
   0061 ruling 1's standard, whose measurement it adds; a header spells it
   when its parsed `version` is four and its digit signature is `4`, the
   token bound as 0061 ruling 7 binds it. This supersedes 0062 ruling 6's
   sentence naming the set `{0, 3}`; the rest of that ruling stands.
2. **Discovery admits two version-four names, ordered.** A DSH session
   directory is asked, in order, for `session.v4.jsonl`,
   `session.v4.jsonl.zstd`, `session.v3.jsonl` and `session.jsonl`, and
   yields at most one candidate, the first carrying a valid header. Of
   the two version-four encodings, which DSH never writes into one root
   together, the plain one is first, being the one a Brokkr-launched seat
   writes. No other name, compressed spelling or compressed twin of an
   older name is a candidate.
3. **A compressed source is read within the plain source's bounds.** A
   `.jsonl.zstd` candidate is decompressed by `brokkr-protocol`'s one
   decoder, the `ruzstd` decode path the operator ruled on #484, reached
   through the existing `brokkr-cli` edge; `brokkr-view` decompresses
   nothing and no dependency is added. Its header check admits at most
   65,536 decompressed header bytes decoded from at most 131,072
   compressed bytes, and a header not decoded within either is
   `discovery-limit`. Its body read reads at most 32 MiB of compressed
   bytes and keeps at most 32 MiB of decompressed bytes; passing either is
   the same source truncation a plain source past the cap has. A frame
   read to its end is kept only when it states a content checksum and its
   bytes match it; one that does not, or bytes that are no frame, are
   `unreadable` at the stage that read them, discovery's closing
   admission as 0055 ruling 2 states. The one frame a bound stops is not
   read to its end, so its checksum is never reached, as a plain source's
   bytes past the cap are never read. This amends 0055 ruling 2.
4. **Version four reads as version three, save its named conversions.**
   Every released version-three event type keeps its version-three
   disposition, because the V3-to-V4 edge rewrites none;
   `developer/message` joins the quiet vocabulary; version-zero storage
   rows and `assistant/chunk` stay version-zero only (0061 ruling 2). A
   `tool/result` projects its tool-role message: `toolCallId` names the
   call and the message's own `content` is the output. The retired
   `tool-result` wrapper block is a counted unrecognized member wherever
   version four reads content, never a result. A namespaced
   `plugin:<name>` type is a name outside the vocabulary and follows 0055
   ruling 3's marker rule unchanged. No `source` is read, and citations
   are validated exactly as under version three. A name outside the read
   vocabulary stays a required unknown (0061 ruling 8).
5. **A working seat's reference reaches the journal at its first turn.**
   The driver's tail asks for `session.v4.jsonl` before the older names,
   so a seat on a 0.2 core emits its first `seat-turn` while it works and
   decision 0053's held transcript row is flushed then, not with the
   finishing row. The hold is not bypassed: a row before any turn is what
   0053 ruling 1 withholds. The warm admission's census keeps naming the
   selected core's `session.v3.jsonl` alone (decision 0056); no pin moves
   and no journal, protocol or contract changes.

Enforcement bindings: the version-four header-matrix, projection,
quiet-vocabulary, namespaced-unknown, tool-role-mapping, retired-wrapper,
source and remapped-reference tests in `brokkr-view`; the discovery,
order, compressed-header-bound, refused-frame, other-name, body-truncation
and body-frame tests in `brokkr-cli`'s local reader; the surfaces test
that carries a plain and a compressed version-four session through every
surface; the decoder's bound and cut-read tests and the working-seat,
tail-order and census tests in `brokkr-protocol`; `openspec validate --all
--strict`; and the exact-coverage gate.

## Consequences

An additive reader change: no stored data migrates, and no contract,
schema, frozen fixture, `Unavailable` reason or `brokkr.transcript/v1`
field changes. Version-zero and version-three files read as before; three
existing tests that used version 4 as the foreign case now use version 5.
`session.v3.jsonl.zstd` and `session.jsonl.zstd`, which the persistence
backend also names, stay unread until a change reads them. A compressed
session that DSH is appending to can be read mid-frame and refuse as
`unreadable` until the frame lands. Only the operator accepts this
proposal; filing it does not certify host coverage, remote CI or
publication.
