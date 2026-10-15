# Design — admit DSH session format version four (#678)

The template is `2026-09-14-admit-dsh-session-v3` (D2 evidence, D3
admission, D4 dispatch, D8 dispositions, D13 mapping). Decision 0086
carries the rulings; this design carries the evidence behind them.

## D1. What fails today, and why

A DSH 0.2.0-rc.2 seat writes
`<root>/<project>/session-<uuid>/session.v4.jsonl`, or
`session.v4.jsonl.zstd` under the persistence backend's default encoding,
with an opening header whose `version` is `4`. Three Brokkr facts refuse
it:

1. `DSH_SESSION_FILES` in `crates/brokkr-cli/src/local_transcript/discover.rs`
   admitted `session.v3.jsonl` and `session.jsonl`: discovery returned
   `not-found` beside the recorded reference.
2. `dsh_header` in `crates/brokkr-view/src/transcript.rs` admitted
   `{0, 3}`: a v4 header placed under an admitted name was
   `unsupported-format`.
3. The driver's tail (`DSH_TRANSCRIPT_NAMES`,
   `crates/brokkr-protocol/src/adapters.rs`) asked for the same two names.
   `run_seat_with` holds every row written before work begins (decision
   0053) and flushes on the first checkpoint whose step `begins_work`; for
   DSH that is the `seat-turn` the tail folds from `assistant/message`. A
   0.2 seat's file was never found, no `seat-turn` was emitted, and the
   held transcript row reached the journal only with
   `deepseek-harness-session-finished`. That is the commission's "a working
   DSH seat has no session reference", and its cause is Brokkr's: the
   locator is recorded before the spawn (`record_locator`) and only the
   hold kept it back.

Proof of 3, run in this session: with `DSH_TRANSCRIPT_V4` removed from
`DSH_TRANSCRIPT_NAMES`,
`adapters::dsh_v4_tests::a_version_four_seat_journals_its_transcript_while_it_works`
fails; its checkpoint steps are `transcript`, `harness-started`,
`deepseek-harness-session-finished`, no `seat-turn`, and the seat exits 9
because its handshake waits for the transcript row to be sent while it
works. Restored, the steps are `transcript`, `harness-started`,
`seat-turn`, `deepseek-harness-session-finished` and the seat succeeds.

## D2. Evidence

Read by this seat at the files cited, copied into the worktree's
git-ignored `.forge/evidence/` from DSH 0.2.0-rc.2's installed packages.
Every package's `package.json` names version `0.2.0-rc.2`.

| Package | File | SHA-256 | Read for |
|---|---|---|---|
| `@deepseek-ai/dsh-session-format-v3-to-v4` | `README.md` | `01dca3f508a5d2c79b25920b284e9277169154c0d3eac694b33d43e307ee1aee` | "V3-to-V4 specification" (first paragraph: the named conversions and preservation), "Tool-result representation", "Message-source conversion", "Sequence references and inheritance", "Delivery generations", "Source audit and refusal", "Native V4 admission" (header, envelope, tool result, system message, producer attribution), "Developer changes and deferred schemas", "Validation entry points and recovery" |
| same | `types/tool-role.d.ts` | `83ff424546c313d265d0306d5a53f166e4355cb6ea1a2aec508f3436c2642a5f` | `liftToolResult`, `assertV4ToolResultMessage` |
| same | `types/sources.d.ts` | `4cddd62da48f29d55239092896c68e2d60927fc5c0da7da20cdac07679de6167` | the source walker and the plugin-source rewrite |
| same | `types/message-sources.d.ts` | `be87c98d129c46211797d0412274b5c874403eec9104351c63e80f606823be37` | native source admission refuses retired plugin wrappers |
| same | `types/extension-identities.d.ts` | `f59a492c138e0ac0766af4a925fcf8e0605f60055d679812747b60d08b22e02f` | `RELEASED_V3_EVENT_TYPES`, `namespaceV3OpaqueEvent` |
| same | `types/references.d.ts` | `25c5bccebedd6101b85b60b775cb58191d0a6ce0dbe4aa8259342dbff138f91c` | `remapV3References` |
| same | `types/validation.d.ts` | `88c87f43b69e419731df403798e44a68087f7bfe5f6db4c65c8c038d42924015` | native header and relationship validation, `knownEventTypes` supplied by the installed Session package |
| same | `types/retired-syntax.d.ts` | `337fc73879cd4112d43fd1b40d71d1804f0eaa97d46f7ed2500421a723cc4e5e` | retired tags refused in interpreted content slots |
| same | `types/developer.d.ts` | `fc66285874c19f22257677d2ad23124d791a143dfb8ff7a089da0ed3e5b81f44` | `developer/message` validation |
| same | `types/migration.d.ts` | `baa18e380d20c7b56391c80b736a45a95babd6286fe68990e2789752f6d4dcee` | the migration declaration exists |
| `@deepseek-ai/dsh-session-persistence-jsonl` | `README.md` | `0cf3b8adb2074d6a518be9b65d52ca92d132a14ae9e73d54a3a3450c3a1d516b` | "On-disk layout", "Durability and crash semantics", "Reading the logs", "Physical encoding" |
| same | `types/format.d.ts` | `deccb6e31d3a84a11122b2fb5f4b244aefe5dd3c8d34fe598da7ff4e6b58d294` | `logSuffix`, `generationLogFilename`, `parseGenerationLogFilename`, `HeaderLine` (lines 42-53) |
| same | `types/zstd.d.ts` | `4b1f793d0a1d12b242f0c9eb239ab6f67c2e8e38a3fc4ca4e0f0c0fadf120630` | concatenated checksummed frames, torn-final-frame recovery |
| same | `types/index.d.ts` | `806dfa610c3ed3ce80ffac89900993288d7013d4278ab21e2b55b193c542f79a` | `Config` names `root` and `compression` only (lines 19-30) |
| `@deepseek-ai/dsh-session-format` | `README.md` | `560ca0d0027d03bb2f7a35102f409bb1756b97f818ccf615330e5e0b22ac7f7c` | package identity; compression and generation naming lie outside it |
| same | `types/filename.d.ts` | `ea080ac1fcc223636e9ee15598410e2c63dc1b9422ee89fc15bcd0dad2081291` | `session.jsonl` for zero, `session.vN.jsonl` after; compression-suffixed, uppercase, leading-zero and `.v0` names are not canonical |
| `@deepseek-ai/dsh-session-format-v2-to-v3` | `README.md` | `b27f139e248030415397fea43c1c8ac44916b987a4dbfd8a36943ab0a9997a01` | its summary only: the V2-to-V3 edge, which version three's admission already covers; no rule here rests on it |

**Not reached** (decision 0061 ruling 8; each withholds, none admits):

- (a) The full version-four event catalogue, `knownEventTypes`, lives in
  `@deepseek-ai/dsh-session`, which is not among the read packages.
  Lifted by reading `dsh-session/lib/types/known-event-types.js` at
  0.2.0-rc.2. Until then every name outside D4's vocabulary is a required
  unknown.
- (b) The message and block definitions behind the four content kinds'
  payloads live in `@deepseek-ai/dsh-llm`. Lifted by reading
  `dsh-llm/lib/types/message.d.ts` at 0.2.0-rc.2. Until then D5 maps the
  blocks the edge carries over (README "V3-to-V4 specification", first
  paragraph: fields outside the named conversions are retained; "Extension
  data": only unknown tags are renamed) and counts every other block.
- (c) Whether any DSH component compresses an idle plain root. The
  persistence README describes no such step: compression is a per-root
  configuration (`'zstd'` default, `'none'`), "a root belongs to one
  encoding", and compression conversion is unsupported. The `.zstd` name
  is admitted on that evidence alone. Lifted by reading the dsh core's
  own source.
- (d) Whether 0.2.0-rc.2's configuration loader refuses or ignores the
  seat overlay's `packChunks: false`, which its `Config` no longer
  declares. The commission's report that a 0.2 seat finished with a
  recorded reference says the backend started. Lifted by reading the
  core's config schema loader, or by a live `--dump-config`.
- (e) The 0.2 `assistant/message` usage keys and whether a native 0.2
  assistant source carries `model`. The driver's served-model read
  (`model_in_json`, on `data.message.source.model`) is preserved for
  direct sources by the edge (README "Message-source conversion": direct
  sources keep every own JSON field); the native shape is (b)'s read.
  This is telemetry, not the reference, and is a follow-up (D7).

A versioned filename, the header number and resemblance of rows to
version three are not evidence (0061 ruling 1), and none is relied on.

## D3. Admission

`DshVersion` gains `Four` and moves, with the version-four delta, into
`crates/brokkr-view/src/transcript/dsh_versions.rs`; `transcript.rs`
shrinks (3,356 to 3,353 lines). `dsh_header` admits four when the parsed
`version` is `4.0` and the digit signature of the one bound token is
`4`: `4`, `4.0`, `4e0`, `40e-1`, `0.4e1`, `4.000` admit; `-4`, `4e1`,
`4.1`, `"4"`, `5`, `3.9999999999999999`, `4.0000000000000001` refuse with
zero counts. The native header (README "Native V4 admission", Logical
header; `format.d.ts:42-53`) is `version`, `id`, `createdAt`, `isSeeded`,
`delegationDepth` and optional `cwd`, `parentSession`, `origin`,
`agentPreset`; the reader's admission shape stays `type`, depth and
`version`, every other field inert (0061 design E2). `isSeeded` gates the
association under four exactly as under three: the seed path is not read
here either.

## D4. Dispatch and vocabulary

`dsh_row` keeps one dispatch for all versions: its version-three
pre-match becomes `Three | Four`, so `assistant/attempt` is a counted
omission and the version-zero storage rows, `assistant/chunk` and the two
retired `tool/code-dispatch*` names are required unknowns under four,
the marker deciding as everywhere else. The README states the edge
"rewrites no event type" and keeps each admitted source event's fields
outside the named conversions, and that unknown required V3 types are
refused before the V4 vocabulary is consulted; a released version-three
type therefore stays a version-four type. Version four's quiet vocabulary
is version three's 51 names plus `developer/message` (README "Developer
changes": native to version four; its tool-change blocks are notices, and
"empty developer nodes ... produce no model message"), written as a delta
over `DSH_QUIET_THREE` so no table is copied (0071 ruling 5). No line of
`project_dsh` or `dsh_row` grows; the payload differences go through two
functions in the new module, `message_blocks` and `result_blocks`, each
matched exhaustively on the version (0071 ruling 2).

A namespaced unknown event, `plugin:<original-name>` with its payload
unchanged and `ignorable: true` (README "Source audit and refusal"), is a
name outside the vocabulary: under 0055 ruling 3 it is a counted omission
with the marker and a required unknown without it. No new rule.

The eighteen types a real seat log carries, as the commission lists them,
each have a disposition: `user/message`, `assistant/message`, `tool/call`
and `tool/result` project; `session` is the header; the other thirteen
are quiet.

## D5. Field-by-field mapping

| Writer field (version four) | Reader |
|---|---|
| `user/message.data.content[]` | blocks, as version three (`dsh_part`); a string content is one text block |
| `user/message.data.turn`, `.step` | not read (version three's rule, 0061 ruling 3) |
| `assistant/message.data.message.content[]` | blocks, as version three |
| `assistant/message.data.turn`, `.step` | position, as version three |
| `tool/call.data.callId`, `.name`, `.arguments` | the call block, as version three (no conversion names `tool/call`) |
| `tool/result.data.message.toolCallId` | the result's id (was the wrapper's `content[0].toolCallId`) |
| `tool/result.data.message.content` | the result's output, rendered by `content_text` as version three renders the wrapper's `content` |
| `tool/result.data.message.isError`, `data.error` | not read, as the wrapper's `isError` was not |
| `tool/result.data.message.role` (`tool`), `id`, `source` | not read; the row type decides the role |
| a `tool-result` block in any content list | a counted unrecognized member; inside a tool-role result the row is counted and shows no result (README "Source audit and refusal": the retired tag is refused wherever content is interpreted) |
| `source` on every message slot | not read under any version, so a producer's renamed kind, a `plugin:` kind or the retired `{kind: "plugin"}` changes no projection |
| `sourceEventSeqs` | validated by `dsh_citations` as today: earlier numbers and `[start, end]` pairs; the storage form of runs of three or more as pairs (persistence README "Physical encoding") is the pair form already read |
| `surfaceOp`, `ignorable` | as today: `surfaceOp` never parsed; `ignorable` the marker |
| `system/message`, `developer/message` | quiet |

A version-three wrapper row and its version-four lift project the same
turn; a scenario pins it.

**Remapped references.** Only the historical conversion renumbers (an
inserted interrupted `turn/end`), remapping `sourceEventSeqs`,
replacement endpoints, `command/done.sourceEventSeq`, title `messageSeqs`,
compaction `shadowedRange`/`shadowedSeqs` and image-offload `seq`
(README "Sequence references and inheritance"). The reader validates
only `sourceEventSeqs`, against earlier observed sequences, and parses no
other reference, so a remapped log projects as any log; a scenario pins
it, and a forward citation refuses under four as under three.

## D6. A compressed source

**Where.** `brokkr-protocol` already holds the one decoder (`ruzstd`
0.9.0, decode-only, the operator's 2026-10-04 ruling on #484). It moves
from `probe/observe.rs` into `crates/brokkr-protocol/src/zstd.rs` as
`pub fn unpack(packed, cut, bound) -> Result<Unpacked, Refused>`, a
`thiserror` enum whose `Display` keeps the probe's texts; the probe's
`unpacked` is a thin wrapper and its eleven compressed tests pass
unchanged. `brokkr-cli` reaches it through its existing dependency; no
edge is added and `brokkr-view` decompresses nothing (0071 rulings 1, 5).
The public API grows by the module, its function and two types
(`quality/public-api/brokkr-protocol.txt`), each with its consumer.

**Frames.** DSH writes one checksummed frame for the header line and one
per durable batch (persistence README "Physical encoding"). Frames are
decoded in turn; a frame read to its end is kept only when it states a
content checksum and its bytes match it.

**Bounds.** The header check reads at most `2 × DSH_HEADER_CAP` (131,072)
compressed bytes and admits at most `DSH_HEADER_CAP` decompressed header
bytes, with the same probe byte as a plain header; a raw encoding of a
cap-sized header needs the cap plus fewer than 32 bytes of frame and
block headers and checksum, so the input bound holds
any honest header frame, and a header not decoded within it is
`discovery-limit`. A header check therefore never reads a body across the
10,000-entry discovery budget. The body read reads at most `SOURCE_CAP`
compressed bytes and keeps at most `SOURCE_CAP` decompressed bytes;
passing either is `overflow`, which the projector reads as source
truncation exactly as for a plain source (`truncated: true`, the size-cap
notice, only newline-terminated rows participating). When the input
bound cuts a frame, that frame contributes nothing; when the output bound
stops a frame, the frame is not read to its end and its checksum is not
reached, as a plain source's bytes past the cap are never read.

**Refusals.** At discovery, a candidate whose frame states no checksum,
fails it or is no zstd is `unreadable` (`HeaderCheck::Io`), decisive for
its directory and closing admission (0055 ruling 2). At the body, the
same is the body-stage `unreadable` with the confirmed path
(`refused_source`). A frame DSH is still appending reads as cut short and
is `unreadable` until it lands; the plain file's provisional final row
has no compressed analogue that can be verified.

**Order.** `session.v4.jsonl`, `session.v4.jsonl.zstd`,
`session.v3.jsonl`, `session.jsonl`. A root belongs to one encoding, so
both version-four names in one directory is not a state DSH writes;
plain is asked first because it is the one a Brokkr-launched seat writes
(D7) and needs no decoding. Generations: the backend keeps immutable
historical generations and selects the highest (persistence README
"On-disk layout", "Reading the logs"), so `session.v3.jsonl` beside
`session.v4.jsonl` is a real state after a migrating open, and
newest-first reads version four. `session.v3.jsonl.zstd` and
`session.jsonl.zstd` are names the backend also writes; this slice reads
version four's compressed name only.

`source_size`, which the growth watch compares, measures the compressed
file for a compressed source: growth is still growth.

## D7. The driver

`DSH_TRANSCRIPT_NAMES` becomes `[session.v4.jsonl, session.v3.jsonl,
session.jsonl]`, one table beside `DSH_TRANSCRIPT`. `adapters.rs` stays
within its baseline (7,113 of 7,117). The tail reads plain lines; the
seat overlay pins `compression: none`, which 0.2.0-rc.2's `Config`
declares (`types/index.d.ts:19-30`), so a Brokkr-launched seat writes the
plain name the tail follows. `packChunks: false` is no longer a declared
field (D2 (d)); the overlay is not changed by this slice.

The rejoin census (`dsh_depth_zero_sessions_reading`) keeps joining
`DSH_TRANSCRIPT` alone: the warm admission is tied to the selected core
(0.1.5-rc.1 composite), which writes version three, the DSH rejoin route
stays disabled, and decision 0056's hold is not loosened. A test pins
that a stored version-four file is no rejoinable session.

**Telemetry.** The fold reads `request/header`'s
`data.header.config.reasoningEffort`, `assistant/message`'s
`data.message.source.model` and `data.usage`, and `tool/call`. The README
keeps request-header config with `reasoningEffort` and direct sources
with every own field; the native usage keys and assistant source are D2
(e). Whether a live 0.2 seat's turns and tokens fold correctly is a
follow-up, not widened here.

## D8. Tests

Every new test was made to fail by a compiling removal of the behaviour
it proves, run in this session, and restored; the result's notes list
each. The version-zero and version-three suites keep every assertion;
three CLI tests and the view's header matrix used version 4 as the
foreign case and now use version 5 (fixture, not assertion; the archived
v3 slice's task 4.3 is the precedent).

Compressed fixtures are synthetic JSONL under
`crates/brokkr-cli/tests/fixtures/dsh-v4/`, compressed one checksummed
frame per line by `ruzstd` 0.9.0's encoder at `Fastest` from a scratch
generator that is not committed; the zstd CLI the probe's fixtures used
was not available to this seat. Each committed twin is held to its plain
source by a test; corrupt variants are made in-test.
