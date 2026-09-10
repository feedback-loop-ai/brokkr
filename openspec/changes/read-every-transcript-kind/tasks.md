# Tasks: Read every retained transcript kind (#222)

Groups are the design's landing order (D10 and the Migration Plan), which
is the order another smith executes them in: the proposed decision filed
before any semantic production edit, then the pure model every surface
consumes, then the shared snapshot/classification frame the three
projections stand on, then Claude, Codex and DSH content, then the safe
filesystem acquisition that feeds them, then the command, then the TUI,
then the browser, then the cross-surface and privacy proofs, then the
guide, then one closing group of gates, the archive fold and the commit.

Every task names the requirement it serves as `<capability> /
<Requirement>`. Group 1 and group 13 serve every requirement of the
change and say so, because a filing prerequisite and a gate are not
requirements of their own.

Conventions binding on every task below, restated once rather than per
task:

- No frozen byte is edited: `contracts/`, `policy/phase-machine.json`,
  `policy/schemas/`, `reference/` and `fixtures/` keep their bytes; no
  contract or view version is bumped to introduce this reader.
- No accepted decision is edited. Only `docs/decisions/0055-…` is added,
  with `Status: proposed`; 0054 and 0056 stay the controller's
  reservations and no other number is taken.
- Tests are written with the code they prove, in the crate that holds
  it, and use synthetic test-owned homes and files. The frozen evaluator
  corpus is never regenerated or reused for transcript examples, and no
  live operator session is copied into the tree.
- No reader, hint or renderer under test starts a provider process, writes a
  journal event, creates a directory or changes a retained byte, and no test
  mutates real operator evidence. Test setup may create and mutate only its
  own synthetic test-owned homes and files between reader invocations so the
  required appearance, append, shrink, disappearance, replacement and
  same-length-rewrite transitions are exercised.
- Related cases are grouped into table-driven tests rather than 175
  bespoke functions (D11); a mark repeated at several call sites is one
  helper, because the coverage gate is literal.
- Cargo commands run with `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.
- Nothing in `crates/brokkr-protocol/src/adapters.rs`, the engine's
  resumption argv or the sandbox boundary is changed: #226 owns those,
  and this work neither reads nor assumes its worktree.

## 1. The proposed decision, filed before any semantic edit (D10)

- [x] 1.1 Write `docs/decisions/0055-read-every-transcript-kind.md` from
      the chief-authored text between the `proposed-decision-0055`
      markers in `design.md`, copied without changing a ruling: the
      decision heading becomes level one and Context/Rulings/
      Consequences become the registry's ordinary section headings,
      `Status: proposed` and `Date: 2026-09-09` unchanged — every
      requirement of this change (the commission's proposed-decision
      prerequisite).
- [x] 1.2 Insert the registry row exactly as `design.md` D10 spells it
      into `docs/decisions/README.md`, in number order after 0053, with
      0054 and 0056 left unwritten — every requirement of this change.
- [x] 1.3 Run `cargo test -p brokkr-cli --test decisions_index` and leave
      it green: the numbered file and its row exist and agree — every
      requirement of this change.
- [x] 1.4 Land 1.1–1.3 in the tree before the first production edit of
      group 2 and after none of them; a later group may not re-open the
      decision's rulings to match an implementation that drifted — every
      requirement of this change.

## 2. The pure result model, the reference and the hint (D2, D3, D7)

- [x] 2.1 Add `crates/brokkr-view/src/transcript.rs` as the only new
      production source file, declared from
      `crates/brokkr-view/src/lib.rs`, and move the serializable
      `Turn { role, ts, blocks }` / `Block { kind, text }` shape there
      from `crates/brokkr-cli/src/ui.rs:167-180`, with the five closed
      block kinds (`text`, `reasoning`, `tool`, `tool-result`,
      `omitted`) serialized as strings and `PartialEq` derived so the
      surfaces can be compared against one another —
      transcript-reading / One transcript derivation serves the local
      readers.
- [x] 2.2 Add `TranscriptKind`, `Unavailable` over exactly the command
      delta's thirteen reason tokens, and `TranscriptRead` with pure
      constructors for a readable and for each refused result, carrying
      selected reference, `legacy`, confirmed path, turns, source and
      display truncation facts, `skipped_lines`, `unrecognized_records`,
      ordered notices, refusal token, explanation and `full_session`;
      the view derivation performs no I/O and reads no environment —
      transcript-reading / One transcript derivation serves the local
      readers; transcript-command / Text output and errors report the
      same bounded result.
- [x] 2.3 Implement reference selection before validation: the latest
      common reference wins even when its locator is empty, a present
      common reference suppresses every legacy fallback, and the three
      recorded strings survive every refusal with `legacy: false`.
      Only an absent common reference with Claude, LaneTally or absent
      legacy provenance may synthesize a Claude reference from the
      supplied local `HOME/.claude/projects` root; an absent or empty
      legacy id is `no-reference` while a present nonempty invalid one is
      `invalid-reference` with a null transcript — transcript-reading /
      The recorded reference selects the harness home.
- [x] 2.4 Implement one accepted language per kind, over the whole
      recorded string with no trimming, case folding, truncation or
      suffix inference: Claude `[0-9a-fA-F][0-9a-fA-F-]{0,63}`, Codex
      `[A-Za-z0-9][A-Za-z0-9-]{0,127}`, DSH a relative forward-slashed
      locator with no empty, dot, parent, drive-prefix or absolute
      component beneath an absolute home; control characters and NUL are
      refused, and decision 0032's 80-character recording clamp is not
      re-imposed as a reader guard — transcript-reading / Local lookup
      rejects paths that escape ownership.
- [x] 2.5 Implement the `full_session` table and its shared quoting
      helper: Claude's `full session: claude --resume <id>` with or
      without a path, Codex's confirmed-rollout and `rollout unavailable`
      forms with `codex exec resume <id>` and the recorded home, DSH's
      confirmed file alone and null without one, null for every rejected
      or absent reference; paths and homes are double-quoted JSON string
      literals with no added shell or slash escaping — transcript-reading /
      Full-session information belongs to the shared local read result.
- [x] 2.6 Generate notices once, in the order truncation, malformed
      lines, unrecognized records, with the exact strings
      `transcript truncated (size cap)`,
      `malformed transcript lines skipped: <n>` and
      `unrecognized transcript records: <n>`; delete both shipped Claude
      suffixes, the TUI's
      ` — claude --resume carries the rest` and the browser's
      ` — resume the session for the rest`, and add renderer/source
      assertions that neither literal remains while every surface emits only
      the exact shared notice —
      transcript-reading / Every kind obeys the same source and display
      caps; transcript-tui / Notices survive every reading surface.
- [x] 2.7 Tests in `crates/brokkr-view/src/transcript/tests.rs`, table
      driven: each of the five reference refusals keeps its three
      recorded strings, `legacy: false`, null path and null hint; a
      stale valid legacy id beside each of them supplies nothing; the
      legacy-synthesis and legacy-invalid-id rows; Claude ids of 1, 64
      and 65 characters, `-abc` and `a-bC09`; Codex ids of 1, 64, 80, 81,
      128 and 129 characters, `-abc`, `ab_cd` and a non-ASCII letter;
      DSH `../other-seat`, a drive-qualified locator and a relative home;
      the six `full_session` rows; and quoting of a path containing a
      quotation mark, a reverse solidus and an ASCII control character —
      transcript-reading / The recorded reference selects the harness
      home; Local lookup rejects paths that escape ownership; Full-session
      information belongs to the shared local read result.

## 3. The bounded snapshot, its diagnostics and both caps (D4)

- [x] 3.1 Add the pure snapshot admission: the caller supplies at most
      33,554,432 bytes plus the one-byte overflow probe fact, and the
      reader validates consumed UTF-8 before any semantic admission,
      treats a code point cut solely by the source cap as a boundary
      fragment and any other invalid byte as `unreadable`, and exposes
      no replacement-character prose — transcript-reading / Partial
      records and read failures remain distinguishable; Every kind obeys
      the same source and display caps.
- [x] 3.2 Iterate complete physical rows only: a final valid JSON value
      at true EOF is complete without a newline, an invalid final
      non-newline append is provisional and uncounted, and above the
      source cap only newline-terminated rows in the prefix participate —
      transcript-reading / Partial records and read failures remain
      distinguishable.
- [x] 3.3 Count `skipped_lines` for complete malformed JSON rows and
      `unrecognized_records` at most once per complete valid row with an
      unsupported type, envelope or content variant, over the whole
      usable prefix, before display capping and before `--turn`; both
      start at zero when no usable snapshot was acquired — transcript-reading /
      Partial records and read failures remain distinguishable.
- [x] 3.4 Apply the 4,000,000-byte sum-of-block-texts budget after
      association and after all classification: retain whole turns in
      source order, stop before the first turn that would exceed it,
      never skip forward to a smaller turn, let equality fit, and set
      `truncated` with its notice for either budget — transcript-reading /
      Every kind obeys the same source and display caps.
- [x] 3.5 Implement D7's failure-stage matrix in the constructors so a
      stage cannot invent a path, a count or a truncation flag it did not
      establish: rejected reference, discovery refusal, source I/O or
      UTF-8 failure, DSH header refusal, DSH event/storage refusal,
      readable projection, and `turn-not-retained` after a readable
      projection — transcript-reading / Partial records and read failures
      remain distinguishable; transcript-command / Text output and errors
      report the same bounded result.
- [x] 3.6 Tests in the same file: a first turn of exactly 4,000,000 and
      of 4,000,001 bytes; a multibyte turn that fits by characters and
      not by bytes; a file ending exactly at a limit reporting no
      truncation; a growing final line that becomes a turn on the later
      read; invalid UTF-8 before any cap boundary returning `unreadable`
      with zero counts; and an empty valid file distinguished from a
      missing one — transcript-reading / Every kind obeys the same source
      and display caps; Partial records and read failures remain
      distinguishable.

## 4. Claude content, moved and unchanged (D7)

- [x] 4.1 Move the projection of `ui.rs:239-316` into the pure reader and
      implement the closed classification table exactly: the five omitted
      top-level record types, the five omitted block kinds, string
      content kept including empty and whitespace, array `text` blocks
      emitted only when nonblank, `tool_use` reduced to its name and
      optional `input.file_path`, and every other top-level type, message
      shape, content shape or block type counted once as unrecognized
      while supported siblings survive — transcript-reading / Claude
      content preserves the existing projection.
- [x] 4.2 Make `crates/brokkr-cli/src/ui.rs`'s `session_transcript` build
      its body from that shared projection, keeping exactly its
      `session_id`, `turns` and `truncated` fields and adding none of the
      new diagnostic members to that compatibility response —
      transcript-reading / Claude content preserves the existing
      projection; Browser participant drills obey shared eligibility.
- [x] 4.3 Tests in `crates/brokkr-view/src/transcript/tests.rs` and
      `crates/brokkr-cli/src/ui/tests.rs`: the shipped transcript literal
      at `crates/brokkr-cli/src/ui/tests.rs:628-648` yields the same two
      turns and three user blocks with `skipped_lines: 1`,
      `unrecognized_records: 0`, `truncated: false` and exactly
      `["malformed transcript lines skipped: 1"]`; a file of only the ten
      omitted record and block kinds has zero counts and `no readable
      turns`; `future-record` plus two `future-block` blocks gives
      `unrecognized_records: 2`; the object-content, numeric-text and
      untyped-block records give three; and the existing endpoint keeps
      its wire shape — transcript-reading / Claude content preserves the
      existing projection.

## 5. Codex rollout content and its association (D5)

- [x] 5.1 Decode the retained `{timestamp, type, payload}` envelope and
      the canonical `response_item` families: ordered `message.content`
      `input_text`/`output_text`, `reasoning` `summary[].text` only,
      function/custom calls with `name`, `call_id` and raw
      `arguments`/`input`, their outputs with `call_id` and output
      including structured content, `local_shell_call.action`,
      `web_search_call.action`, `tool_search_call.arguments` and
      `tool_search_output.tools` as recorded structured data, inert
      `omitted` markers for image and audio, and the recognized quiet
      response-context types — transcript-reading / Codex rollouts expose
      ordered retained content once.
- [x] 5.2 Decode the content-bearing `event_msg` families exactly as
      D5's table spells them, with their case-sensitive PascalCase item
      discriminants and camelCase/snake_case field split: completed user,
      agent, reasoning, function output, command execution, dynamic tool,
      MCP tool and plan items, the legacy message/reasoning events, and
      the begin/end execution and tool pairs; command output prefers
      stdout/stderr when either carries text, then a nonempty aggregate,
      then formatted output, and an all-empty result stays recorded and
      empty — transcript-reading / Codex rollouts expose ordered retained
      content once.
- [x] 5.3 Enumerate the quiet top-level and event names D5 lists and
      count every other complete valid record once as unrecognized; no
      prefix match, lifecycle label or familiar nested field extends
      either list, and encrypted or raw reasoning is never projected —
      transcript-reading / Codex rollouts expose ordered retained content
      once; Partial records and read failures remain distinguishable.
- [x] 5.4 Implement association as a separate pass over the complete
      bounded prefix, on recorded identity plus direction and compatible
      family only: a proved canonical counterpart occupies its own
      source position and timestamp and removes only the blocks it
      actually covers; colliding or absent identities keep both records;
      canonical records never deduplicate each other; and no positional,
      textual, timestamp or recency matcher is added — transcript-reading /
      Codex rollouts expose ordered retained content once.
- [x] 5.5 Tests: a five-record ruling read in order; canonical records
      beside their matching event notifications shown once; an
      event-only snapshot with all five kinds readable; a later snapshot
      where the canonical message replaces its fallback and unrelated
      event content survives; a canonical item beyond the source cap or
      as an incomplete append that cannot suppress the visible event;
      identical text with distinct identities kept twice; two calls with
      equal arguments and different call ids; an output without its call
      and an encrypted-only reasoning record; an unknown completed item
      counted once; and aggregate-only and formatted-only command output —
      transcript-reading / Codex rollouts expose ordered retained content
      once.

## 6. DSH storage, format admission and assembly (D6)

- [x] 6.1 Admit content only under an opening first-record `session`
      object whose top-level `version` is a JSON number equal to zero,
      accepting `0`, `0.0`, `0e0` and `-0` and coercing no string,
      boolean or null; a missing, mistyped or foreign version returns
      `unsupported-format` with `DSH transcript format is not supported`,
      the unchanged reference, the confirmed path and DSH hint, no turns,
      zero counts and only source-cap truncation, and no later `session`
      row can repair or switch it — transcript-reading / Discovery
      identifies one owned local file.
- [x] 6.2 Project the ordinary version-zero events: `user/message` from
      `data`, `assistant/message` from `data.message`, `tool/call` from
      `data.name`/`callId`/`arguments`/`turn`/`step`, `tool/result` from
      its nested message under the `tool` role, ordered `text.text` and
      `reasoning.text` blocks, complete tool-call and tool-result blocks,
      inert `image` omissions, and readable `assistant/chunk`
      `text-delta`/`reasoning-delta` fragments with whitespace preserved
      and empty strings omitted; the five quiet chunk types and D6's
      recognized quiet event vocabulary are enumerated from that captured
      catalog and count in neither diagnostic — transcript-reading / DSH
      sessions expose assembled or provisional content once.
- [x] 6.3 Decode `text-chunks`, `reasoning-chunks` and `tool-call-chunks`
      rows under R15's exact-key tables: validate the whole physical row
      before any member is yielded, reconstruct `seq0 + k` and the
      cumulative `dt` time in checked safe-integer arithmetic preserving
      negative gaps, keep each member's stored turn, step and index,
      render `Turn.ts` as the signed base-ten epoch millisecond integer
      string, and refuse a structurally invalid row as
      `unsupported-format` with one unrecognized physical row and no
      malformed-line increment — transcript-reading / DSH sessions expose
      assembled or provisional content once.
- [x] 6.4 Implement `sourceEventSeqs` admission and suppression: a
      bounded index over observed logical events, inclusive two-integer
      ranges matched by set membership without expanding the interval or
      allocating by its width, entries validated whole on each surface
      event, and suppression only of uniquely identified, earlier,
      readable chunks of a readable assembly's own recorded turn and
      step. A partial list is not all-or-nothing: it suppresses exactly
      the subset it proves and leaves every uncited chunk at its own
      source position, so `[10]` over readable chunks 10 and 11 retains
      chunk 11 and then the assembly. An absent field and an empty `[]`
      suppress nothing at all, and an unproved entry inside an otherwise
      valid list — a sequence absent from the bounded snapshot, one
      recorded in another turn or step, one that two earlier events
      share, or one naming a non-chunk event — has no suppression effect
      at all: whatever existing event it names stays visible at its own
      source position, while that list's proved targets still disappear.
      Duplicate, overlapping and out-of-order entries are set membership
      that neither duplicates nor reorders content; user and tool-result
      citations and `surfaceOp` rewrite no history; and an invalid field
      refuses the read with one unrecognized row — transcript-reading /
      DSH sessions expose assembled or provisional content once.
- [x] 6.5 Associate dedicated DSH `tool/call` and `tool/result` events
      with the tool blocks embedded in assembled messages, at block level
      and on recorded evidence only: a dedicated event owns an embedded
      `tool-call` or `tool-result` block solely when the dedicated
      event's recorded `callId` equals the identifier the block itself
      stores (the tool-result block's `toolCallId` under D6) and the
      dedicated event's recorded `(turn, step)` equals the owning
      message's pair; no identifier is inferred where the row records
      none. The dedicated event then supplies the one displayed
      call or result at its own source position and timestamp, its owning
      message keeps every other text, reasoning and unmatched block in
      recorded order, and a turn is not emitted once its only block is
      suppressed. An embedded complete call or result whose partner is
      absent, whose call id collides with two dedicated events, or whose
      turn/step disagrees stays visible under the absent-partner rule;
      dedicated events never suppress one another, and no textual,
      positional, timestamp or recency matcher is added — this is the
      once-only projection that the citation rule of 6.4 does not cover —
      transcript-reading / DSH sessions expose assembled or provisional
      content once.
- [x] 6.6 Implement the required-unknown rule after header admission: a
      valid JSON row with an unrecognized event envelope is a counted
      omission only when it is an object carrying top-level `ignorable`
      exactly boolean `true`; otherwise the whole read returns
      `unsupported-format` with no prose, the confirmed path and hint,
      the complete usable prefix's two counts and only source-cap
      truncation, while source I/O and UTF-8 failure outrank it and
      header refusal precedes it — transcript-reading / Partial records
      and read failures remain distinguishable.
- [x] 6.7 Tests, table driven over both ordinary and packed encodings:
      version `0`, `0.0`, `0e0`, `-0`, absent, null, false, `"0"`, array,
      object, `1`, `-1` and `0.5`; depth zero, omitted, positive,
      negative, string and floating; a header-only file at EOF without a
      newline; an absent or malformed opening row that cannot borrow a
      later header; packed and ordinary equivalents of three text and
      three reasoning fragments giving the six stamps `"1000"`, `"999"`,
      `"1004"` and their reasoning counterparts with zero counts; a wrong
      `dt` length, a non-string member and an overflowing reconstruction;
      a tool-argument run that invents no call; an incomplete and a
      cap-cut packed row supplying no members; citations with the exact
      retained turn sequence and one-based indices pinned for each, in
      both encodings — `[[10, 12]]` and `[10, 11, 12]` over readable
      same-step chunks 10, 11, 12 and 14 retaining chunk 14 then the
      assembly, `[[12, 14], [10, 12], 11]` over the same four retaining
      the assembly alone with zero counts and no notices, `[]` and an
      absent field over chunks 10 and 11 retaining both then the
      assembly, and the partial `[10]` over those two retaining chunk 11
      then the assembly — plus one list mixing a proved same-step chunk
      with a cross-step chunk, a sequence two earlier events share, an
      absent sequence and a non-chunk event, pinning the retained
      sequence in which only the proved chunk disappears while the
      cross-step chunk and every other existing named event keeps its
      own source position ahead of the assembly, and self, future,
      reversed and `[0, 9007199254740990]`; the display cap stopping
      between members; a dedicated `tool/call` sharing an embedded
      block's call id and turn/step showing that call once while its
      message keeps its text and reasoning blocks in order; the same pair
      with a differing call id and with a differing turn/step, each keeping
      both the dedicated and embedded copies; no dedicated record at all
      keeping the sole embedded block exactly once; two dedicated
      events colliding on one call id keeping every record; a dedicated
      `tool/result` matching an embedded result block whether it precedes
      or follows its message; and an assembled message whose only block
      is so suppressed emitting no empty turn; an unknown event with
      `ignorable` true, false, null, `"true"`, `1` and absent; a
      recognized envelope with two unsupported blocks; and
      the capped snapshot returning `unsupported-format` with
      `skipped_lines: 2`, `unrecognized_records: 2` and all three notices
      in order — transcript-reading / DSH sessions expose assembled or
      provisional content once; Discovery identifies one owned local
      file; Partial records and read failures remain distinguishable.

## 7. Safe discovery and the bounded read (D3, D4)

- [x] 7.1 Add the small `cfg(unix)`/`cfg(windows)` handle helper nested
      in `crates/brokkr-cli/src/ui.rs`: canonicalize the recorded home
      once, open it as the traversal root, then open each descendant
      component relative to a held directory handle without following
      symlinks or reparse points, enumerate through those handles rather
      than a rebuilt pathname, admit only regular files, and keep the
      candidate's verified handle for the body read. Use the already
      locked `rustix` fs and `windows-sys` filesystem bindings as
      target-specific direct dependencies without upgrading a registry
      version; a pathname-only fallback is not admitted — transcript-reading /
      Local lookup rejects paths that escape ownership.
- [x] 7.2 Implement the three closed discovery scopes: Claude's exact
      `<id>.jsonl` in immediate project directories of the recorded
      projects home; Codex's `rollout-*.jsonl` under `<home>/sessions`
      through depth six, matching the entire case-sensitive id as a whole
      token delimited by non-ASCII-alphanumeric characters or the
      filename's ends, with no content or header gate; DSH's
      `<home>/<locator>/<project>/<session>/session.jsonl` whose first
      complete row is a `session` object with unsigned-integer zero or
      omitted `delegationDepth`, the `version` field neither qualifying
      nor vetoing ownership — transcript-reading / Discovery identifies
      one owned local file.
- [x] 7.3 Bound and resolve the lookup from collected facts rather than
      enumeration order: at most 10,000 examined entries per lookup and
      65,536 first-record bytes per DSH candidate detected without
      allocating a longer header, no transcript content read for Claude
      or Codex, `discovery-limit` outranking a provisional match, I/O or
      invalid UTF-8 preventing a unique answer returning `unreadable`,
      two safe qualifying files returning `ambiguous-source` in either
      order, unsafe entries supplying no content and `unsafe-path` when
      no safe unique source exists, and the fixed `no valid depth-zero
      DSH session header` explanation with a null path when only invalid
      or delegated DSH headers remain — transcript-reading / Discovery
      identifies one owned local file; Local lookup rejects paths that
      escape ownership.
- [x] 7.4 Read the selected source through the retained handle: at most
      33,554,432 bytes plus one probe byte, no whole-file allocation,
      ancestry and candidate identity rechecked at the read boundary with
      a bounded fail-closed acquisition rather than an unbounded retry,
      and a non-Unicode path reported as `unreadable` before any path is
      confirmed rather than lossily spelled — transcript-reading / Every
      kind obeys the same source and display caps; Local lookup rejects
      paths that escape ownership.
- [x] 7.5 Retire `valid_session_id`, `transcript_path`, `transcript_len`,
      `transcript_grew` and `session_turns` as the shipped first-match,
      unbounded, symlink-following lookup, and route every remaining
      caller — the HTTP routes, the TUI and the new command — through
      the shared reference/discovery/read path — transcript-reading /
      Discovery identifies one owned local file.
- [ ] 7.6 Tests in `crates/brokkr-cli/src/ui/tests.rs` over synthetic
      test-owned homes: the three per-kind scopes; `rollout-0199mine`,
      `rollout-0199other` and `rollout-0199mineX` with only the first
      eligible; an 80-character recorded id against an 81-character
      filename; a recorded custom home preferred over a different ambient
      one; a depth-one delegated sibling; a header with
      `delegationDepth: "zero"`; version-zero and version-one roots in
      both enumeration orders; entry-bound exhaustion and an oversized
      DSH first record; a symlink project directory, a symlink transcript
      inside and outside the root, and a FIFO; ancestor and leaf
      replacement between discovery and read on the platform helper; a
      non-Unicode path; and a retained-bytes-unchanged assertion after
      every one of them — transcript-reading / Discovery identifies one
      owned local file; Local lookup rejects paths that escape ownership.

## 8. `brokkr transcript` (D7)

- [x] 8.1 Add `TranscriptArgs` in `crates/brokkr-cli/src/cli_args.rs`
      beside `InspectArgs` — required `--run` and `--seat`, optional
      `--turn`, `--json`, `--realms` and `--db` — and its `Cmd::Transcript`
      arm, keeping the one-verb-per-builder shape the module exists for —
      transcript-command / The transcript command selects one run and
      participant.
- [x] 8.2 Dispatch in `crates/brokkr-cli/src/lib.rs` through the same
      read-only world resolution `Cmd::Inspect` uses — `journal_of` then
      `selector::resolve_run` — refusing a missing journal without
      creating one, and never launching, retrying or resuming anything —
      transcript-command / The transcript command selects one run and
      participant.
- [x] 8.3 Select the participant within the resolved run: an exact
      participant key wins, otherwise an exact label only when unique,
      with no prefix or fuzzy matching; an ambiguous label fails naming
      every matching key; a panel or sequence parent reads only its own
      common reference and borrows no member's — transcript-command / The
      transcript command selects one run and participant.
- [x] 8.4 Parse `--turn` as a positive unsigned 64-bit integer, making
      zero, negative, non-integer and overflowing arguments usage errors
      that write nothing, and apply the index to the shared projection
      only after both caps, all diagnostics and every DSH refusal, with a
      valid out-of-range index returning `turn-not-retained` and a
      truncated projection saying the bounded read does not establish
      whether that turn exists later — transcript-command / Turn selection
      addresses the displayed sequence.
- [x] 8.5 Emit the `brokkr.transcript/v1` document with exactly the
      delta's members, each present even when null or empty, serialized
      directly from the shared result and adding no field to
      `RunView`, inspect or seats JSON — transcript-command / JSON exposes
      a distinct local transcript document.
- [x] 8.6 Render default text through `crates/brokkr-cli/src/render.rs`:
      run, participant, kind and confirmed source, then one-based turn
      numbers, roles, stamps and every retained block, with the shared
      hint and notices, `no readable turns` for a valid empty projection
      and a truncated zero-turn projection saying so; apply `Safe` to
      content, references, paths, roles, stamps, hints and explanations
      while JSON keeps its escaped originals — transcript-command / Text
      output and errors report the same bounded result.
- [x] 8.7 Fix the exit and stream rules: a readable result including
      empty, skipped-line, counted-omission and truncated results exits
      zero; the thirteen unavailable reasons exit one with a sanitized
      stderr explanation, no turns and — under `--json` only — the
      document on stdout; usage, run-selection and participant-selection
      failures exit nonzero with empty stdout even under `--json` —
      transcript-command / Text output and errors report the same bounded
      result.
- [ ] 8.8 Tests in `crates/brokkr-cli/src/render/tests.rs` and a new
      `crates/brokkr-cli/tests/transcript_command.rs`: `--run latest
      --seat review:chief`; a repeated label listing both keys; a panel
      parent reporting `no-reference` while the member key reads its
      file; a cross-realm ambiguous prefix and an explicit `--db`; a
      missing journal creating nothing; `--turn 4` selecting the tool
      result; turn one skipping metadata and malformed lines; a selected
      turn keeping the truncation and unrecognized notices; `--turn` past
      a truncated and past a complete projection; `--turn 2` and `--turn
      4` over a packed row's members and its ordinary equivalent — transcript-command /
      The transcript command selects one run and participant; Turn
      selection addresses the displayed sequence.
- [ ] 8.9 JSON and text state tests in the same file: the five rejected
      common references with their reasons and preserved strings; a
      header-less Codex rollout of only `turn_context` exiting zero with
      its confirmed path and hint; empty versus missing; five ignorable
      unknown records with `unrecognized_records: 5`; the shipped Claude
      fixture's `(1, 0)` counts in whole and selected reads; the three
      Claude lookup refusals keeping the non-null Claude hint; a legacy
      Codex participant returning `no-reference`; the exact
      `rollout unavailable` string; Claude and DSH missing-file hints;
      the invalid-depth DSH stderr; the legacy-synthesis flag; a turn
      carrying an escape sequence surviving only as JSON data; the DSH
      event-refusal document with all three notices; the header-version
      document with zero counts and both `--turn` variants; ambiguity
      between a version-zero and a version-one root; and one invalid
      packed row counting once — transcript-command / JSON exposes a
      distinct local transcript document; Text output and errors report
      the same bounded result.

## 9. The terminal pane and both doors (D8)

- [x] 9.1 Replace `tui::Ask.session` with the selected subject — realm
      and journal identity, full run id, participant key and the complete
      effective reference — and `tui::Views.transcript`'s
      `Option<(Vec<Turn>, bool)>` with the shared `TranscriptRead`,
      deleting `claude_session` (`crates/brokkr-cli/src/tui.rs:420-431`)
      so no surface gates the pane on one kind — transcript-tui / Every
      readable kind reaches the pane and both doors.
- [x] 9.2 Replace `transcript_moved`
      (`crates/brokkr-cli/src/lib.rs:734-755`) and the file-length gate
      of `tui_views` with a re-resolution at every existing refresh
      opportunity while the participant works, keeping an in-memory
      source stamp of opened-file identity and bounded source/member
      identities rather than mtime or length; the read is bounded, the
      journal read-only, and no new clock, watcher or cache is added —
      transcript-tui / Live refresh follows the selected reference.
- [x] 9.3 Publish each refreshed result atomically and invalidate on
      change: removal, replacement, reorder, shrink, changed authority,
      ambiguity, ownership loss or format refusal clears the turn cursor
      and closes an open overlay before new indices are shown, while a
      pure append with an unchanged projected and source prefix keeps
      navigation, and notice-only changes keep the cursor; a final read
      happens when the participant concludes, automatic polling then
      stops and an explicit refresh still re-resolves the reference —
      transcript-tui / Live refresh follows the selected reference.
- [x] 9.4 Render the pane and both doors from the shared result: turns
      numbered from one in the command's sequence including each packed
      member, preview clipping that removes nothing from either door, the
      shared notices in the pane and both overlays, the `full_session`
      value rendered verbatim when non-null and no line at all when null,
      the whole-transcript overlay carrying that hint, a readable
      zero-turn result keeping its openable explanation, and an
      unavailable or refused result showing the reference, reason,
      explanation, retained path/hint and counts with both doors disabled
      and no stale prose; delete `session_line` and the suffixed
      `TRUNCATED_NOTICE` (`crates/brokkr-cli/src/tui.rs:653-676`) —
      transcript-tui / Every readable kind reaches the pane and both
      doors; transcript-tui / Full-session information is truthful for its
      kind; transcript-tui / Notices survive every reading surface.
- [x] 9.5 Headless tests in `crates/brokkr-cli/src/tui/tests.rs`: Codex
      and DSH turns browsed and opened with the existing keys; a turn
      taller than the pane opened whole and scrolled; the highlighted
      index agreeing with `--turn`; moving from a readable Claude seat to
      an unavailable participant; six packed members through both doors;
      the three kinds' hints and the `rollout unavailable` form; a valid
      65–80-character Codex id; rejected references without doors; a
      shell-fragment id; a boxed Codex seat's hint starting no process;
      the three Claude lookup refusals keeping the Claude hint; the
      shared truncation notice for Claude and Codex with no suffix; the
      unknown-record notice in pane and both doors; the shipped Claude
      fixture's counts after selection; and an oversized first turn —
      transcript-tui / Every readable kind reaches the pane and both
      doors; transcript-tui / Full-session information is truthful for its
      kind; transcript-tui / Notices survive every reading surface.
- [x] 9.6 Refresh tests in the same file: a late Codex rollout appearing
      without a journal event or navigation round trip; a DSH assembled
      message appended between checkpoints; chunks replaced by their
      citing assembly with the selection cleared and the overlay closed;
      a Codex canonical arrival replacing only its associated fallback; a
      replacement reference, a shrink and a disappearance; a concluded
      participant's final and manual reads; an appended required unknown
      event clearing both doors; an appended ignorable event keeping the
      turns and raising the count; a ranged partial assembly keeping the
      uncited chunk; an absent citation preserving the prior selection; a
      refused snapshot that cannot reopen the capped explanation; a
      same-length version-0-to-1 rewrite and its later recovery; and a
      second foreign-version root producing ambiguity — transcript-tui /
      Live refresh follows the selected reference.

## 10. The browser's participant presentation and its Claude routes (D9)

- [x] 10.1 Route `/api/session/<id>` and `/sse/session/<id>` through the
      shared identifier guard, safe discovery and Claude projection,
      keeping them journal-independent lookups under the server's local
      projects home: every reference, discovery or read failure returns
      HTTP 404 with the existing JSON error envelope before a body or an
      event-stream header is written, with the two routes' mappings kept
      apart rather than merged — `/api/session/<id>` answers
      `{"error":"session not found"}` for an invalid id and
      `{"error":"transcript not found"}` for a valid id whose lookup or
      read fails, while every `/sse/session/<id>` admission refusal,
      an invalid id included, answers `{"error":"transcript not found"}`
      as shipped `crates/brokkr-cli/src/ui.rs:429` already does; every API
      body, successful or refused, sends
      `Cache-Control: no-store`, and success keeps the three-field envelope;
      an admitted stream keeps its size-event and
      heartbeat shape, revalidates unique safe discovery on each poll and
      closes on loss without reporting another size — transcript-reading /
      Discovery identifies one owned local file; Browser participant
      drills obey shared eligibility.
- [x] 10.2 Add one GET participant-presentation route in
      `crates/brokkr-cli/src/ui.rs`, keyed by full run id and an encoded
      participant key, decoding each path component exactly once and
      rejecting malformed or extra components, resolving that exact
      participant in its read-only journal, accepting no path or home
      override and keeping the existing loopback, Host and method guard;
      construct the CLI-private response from shared reference selection,
      validation, bounded safe discovery and hint helpers only, without
      reading body bytes or running a content projector; serialize only the
      selected reference, legacy and admission facts, lookup-unavailability
      reason and explanation, shared hint and Claude drill eligibility, carry
      no turns, blocks, transcript prose or body-stage outcome; allow the
      discovery-refusal constructor to carry `unreadable` when directory I/O
      or the bounded DSH opening-header I/O/UTF-8 check prevents discovery
      from establishing a safe unique source, and send `Cache-Control:
      no-store` —
      transcript-reading / Browser participant drills obey shared
      eligibility.
- [x] 10.3 Rewrite the page's participant block in
      `crates/brokkr-cli/src/ui.html`: consume that result instead of
      `part.session_id`, delete the
      `full session: <id> · held by <holder>, no resume verb yet`
      sentence, render the shared hint verbatim through `textContent` or
      no line when null, and require both kind-agnostic source admission and
      the independent Claude-kind/canonical-local-home drill eligibility for
      every `· session <id>` label, id-only body request and growth watch;
      show
      `browser transcript unavailable for this recorded home` with the
      checkpoint fallback otherwise, keep Codex and DSH on their hint and
      fallback with zero Claude requests or watches even when admitted, and
      move the client id guard to the leading-hexadecimal rule. Key private
      state by full run id, participant key and complete effective reference;
      keep a monotonic generation, body state
      (missing/pending/succeeded/refused), the exact owned `EventSource`
      handle and a per-re-check automatic-opening budget. On key, admission
      or eligibility change, bump the generation, close only the owned old
      watch and clear cached/displayed prose before repaint; ignore every
      stale body, presentation or watch callback. On watch close/error, close
      the exact handle, clear cached/displayed prose, bump the generation and
      re-request presentation without native same-source reconnection; if the
      fresh result remains admitted and drill-eligible and the refusal floor
      does not silence it, recover through fresh no-store presentation and body
      fetches and then at most one automatic watch opening for a working
      participant in that interval. On body refusal, close the exact handle,
      clear cached/displayed prose, bump the generation, mark that source
      refused for the current interval and re-request presentation; render a
      changed admission or eligibility result, but when the fresh presentation
      remains admitted and eligible retain the unqualified body-failure prose
      and perform no further body or watch work until the next recurring
      re-check. Run one recurring presentation re-check tied to the active
      selection, without accumulating timers, at least as often as the
      existing runs poll, including after conclusion, and restore exactly one
      automatic opening at each tick. Treat results as equivalent only when selected
      reference, admission, lookup reason, shared hint and drill eligibility
      match; an equivalent result repaints nothing and repairs only a missing
      body, then a missing working-seat watch. Define a body as missing only
      when none has succeeded since turns were last discarded and none is
      outstanding; mark every HTTP 200 body successful even when `turns` is
      empty, paint that success without body-failure prose, and let only a
      clear or refusal reset that success. After a
      refused/failed/unparseable body clear prose and silence body/watch work
      until the next re-check; count every automatic opening, including the
      tick's, against that interval's single budget, while a second immediate
      closure may repaint from a fresh body but opens no watch until the next
      tick. Isolate these facts and transitions in one
      start/end-marker-delimited block in the served page which exposes only
      the private `createTranscriptController(effects)` factory and reaches
      no ambient browser global; keep the production fetch, `EventSource`,
      timer and DOM adapters thin and outside that extracted block. Every
      explicit operator selection, including selection of the already active
      identical subject, unconditionally starts a fresh generation and
      re-check interval: close the exact prior watch, clear body/prose and
      pending work, clear the refusal floor, restore one opening budget and
      fetch fresh no-store presentation before any new callback can paint —
      transcript-reading / Browser participant drills obey shared
      eligibility; Local lookup rejects paths that escape ownership.
- [x] 10.4 Add `boa_engine = "=0.21.1"` to `brokkr-cli`'s
      dev-dependencies with default features disabled and update
      `Cargo.lock` without upgrading unrelated registry dependencies. Add a
      Rust harness in `crates/brokkr-cli/src/ui/tests.rs` that extracts the
      marker-delimited controller block from the exact `PAGE` bytes,
      evaluates that block with Boa, injects deterministic presentation/body
      promises, identity-bearing watches, timers and paint/clear effects,
      drains queued Promise jobs after every delivered event, and asserts both
      the effect trace and the controller's small state snapshot. Keep no
      copied JavaScript fixture, Rust transition twin, DOM/network simulation
      or test engine in production dependencies; verify the harness executes
      the extracted served bytes and the release dependency tree excludes Boa
      — transcript-reading / Browser participant drills obey shared
      eligibility.
- [x] 10.5 HTTP and thin-adapter tests in
      `crates/brokkr-cli/src/ui/tests.rs`: a legacy Codex
      participant ineligible on the page while `/api/session/abcd-1234`
      still answers 200 or 404 on its own; a common reference defeating a
      stale flat id; an eligible Claude participant showing the shared
      hint and turns; the participant-presentation route resolving the exact
      encoded full run id and participant key after decoding each component
      once, not treating a double-encoded participant key as its decoded-twice
      peer, and rejecting malformed encodings and extra path components; a
      recorded custom Claude home showing the
      home explanation beside the shared hint and drilling nothing;
      `-abc` refused by client, page, API and SSE, asserting the exact
      bodies `{"error":"session not found"}` from the API and
      `{"error":"transcript not found"}` from SSE with no stream header;
      `a-bC09` keeping the existing envelope and stream; the three
      lookup refusals answering
      404 with `{"error":"transcript not found"}` on both routes, no
      turns and no stream header; an admitted stream losing its unique source
      mid-watch; `Cache-Control: no-store` on presentation and every API
      body response with no-store presentation/body refetches; and the retired
      holder sentence and both retired truncation suffixes absent from the
      page. Assert the production adapter constructs the controller exactly
      once, percent-encodes path components, requests no-store presentation
      and body reads, owns exact `EventSource` and timer handles, and paints
      untrusted values only through `textContent` — transcript-reading /
      Browser participant drills obey shared eligibility; Every kind obeys
      the same source and display caps.
- [x] 10.6 Execute controller transition traces through the 10.4 harness,
      driven by controlled presentation/body promises, fake `EventSource`
      open/error callbacks and recurring timer ticks rather than
      source-string containment alone:
      admitted Codex, DSH and foreign-home Claude selections make zero id-only
      requests and watches across re-checks; a concluded successful zero-turn
      Claude body makes one total request; a persistently unreadable admitted
      Claude source makes one refused request per interval; a distinct
      discovery-refusal trace supplies an otherwise valid DSH reference whose
      bounded opening-header I/O/UTF-8 check cannot establish unique ownership
      and proves closed admission, shared `unreadable` plus its explanation,
      null path/session label and DSH hint, checkpoint fallback, zero id-only
      body requests and zero watches while every recurring tick performs only
      fresh bounded presentation discovery; table cases prove the same result
      when directory I/O defeats Claude or Codex uniqueness; re-check-first
      and closure-first traces each open exactly one automatic watch per interval;
      a second immediate closure repaints from a fresh body without opening a
      second watch; admission loss with unchanged participant/reference/
      journal clears cached and displayed prose, closes the exact old watch
      and rejects its late response; later admission recovers through fresh
      presentation/body work; and identity or eligibility change resets only
      the new key's generation and budget. Add an adversarial
      identical-subject reselection trace that first exhausts the current
      refusal floor and watch budget, then reselects that same full subject and
      proves a fresh generation, presentation, body and eligible watch opening
      while late body, presentation and watch callbacks from the prior
      generation remain inert. Verify every trace against ordered effects,
      request/watch counts and state snapshots after drained Promise jobs —
      transcript-reading / Browser participant drills obey shared eligibility;
      Every kind obeys the same source and display caps.

## 11. Inertness and one result across the surfaces (D11)

- [x] 11.1 Prove locality and inertness in
      `crates/brokkr-cli/tests/transcript_privacy.rs`: the journal is
      opened read-only and gains no event or checkpoint; sentinel prompt,
      reasoning, tool-argument and output strings appear in no
      `inspect`, `seats`, `watch`, `export`, `dossier`, result-telemetry
      or journal-derived output; the retained file, its root and the
      provider configuration keep their bytes and existence across a
      successful read, a refusal and a growth watch; no provider process
      is started by any read or hint; and terminal output is sanitized
      while JSON keeps escaped strings — transcript-reading / Transcript
      prose stays local and inert.
- [ ] 11.2 Prove one derivation in
      `crates/brokkr-cli/tests/transcript_surfaces.rs`, comparing each
      surface only where it is authorized to carry content: for one
      synthetic source of each kind, the command's whole read, its
      `--turn` selection, the TUI pane and both overlays carry the same
      serialized turns, blocks, numbering, notices, hint and
      unavailability — the `--turn` comparisons scoped to the requested
      index — including a readable zero-turn result, a truncated result,
      a counted-omission result and each DSH refusal, while Claude's
      existing `/api/session/<id>` body agrees on turns and `truncated`
      over the same source and keeps its three-field envelope. The
      browser participant presentation is compared only on the shared
      metadata it transports — selected reference, `legacy` and admission
      facts, lookup-unavailability reason, explanation, hint and Claude drill
      eligibility — and is asserted to carry no turns, blocks or
      transcript prose for any of the three kinds, so this proof cannot
      be satisfied by widening that transport (10.2, D9) —
      transcript-reading / One transcript derivation serves the local
      readers; transcript-command / JSON exposes a distinct local
      transcript document; transcript-tui / Every readable kind reaches
      the pane and both doors.

## 12. The guide

- [x] 12.1 Add a third-level `brokkr transcript` section to
      `docs/guides/read-surfaces.md`, in the heading style of the verb
      sections beside it: the selectors and their `inspect` precedence, the
      one-based snapshot indices and that they are not durable message
      ids, the text and `brokkr.transcript/v1` faces, the closed
      unavailable vocabulary and exit rules, the shared notices, and the
      per-kind full-session lines as inert information that executes
      nothing — transcript-command / Text output and errors report the
      same bounded result; JSON exposes a distinct local transcript
      document.
- [x] 12.2 State the Claude compatibility costs in that guide's
      transcript and `brokkr ui` sections — leading-hyphen ids now
      invalid everywhere, duplicate candidates, the discovery bound and
      below-home symlinks now refused, and the 32 MiB source limit — with
      the operator-owned remediation and a pointer to proposed 0055, and
      say that the TUI, the command and the browser read the same result —
      transcript-reading / Local lookup rejects paths that escape
      ownership; Discovery identifies one owned local file; Browser
      participant drills obey shared eligibility.
- [x] 12.3 Keep `cargo test -p brokkr-cli --test contributing` green: the
      guide keeps every section
      `crates/brokkr-cli/tests/contributing.rs:362-369` pins and gains
      the new one — transcript-command / Text output and errors report the
      same bounded result.

## 13. Gates, the fold and the commit

- [x] 13.1 `cargo fmt --all -- --check` clean — every requirement of this
      change (the house rule that gates the work).
- [x] 13.2 `cargo clippy --workspace --all-targets --all-features
      --locked -- -D warnings` clean — every requirement of this change.
- [x] 13.3 `cargo test --workspace --all-features --locked` green as one
      whole-workspace run with `RUST_TEST_THREADS=2`; a crate-scoped run
      may precede it while iterating and never stands in for it — every
      requirement of this change.
- [x] 13.4 `cargo run --locked -p brokkr-cli -- compile --bundle
      bundles/self` and the same for `bundles/verify` both compile —
      every requirement of this change.
- [ ] 13.5 `scripts/coverage-exact.sh`, unchanged and never lowered, at
      literal 100% of lines and branches with `TMPDIR=/var/tmp` and
      `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`; inside a nested sandbox that
      refuses the boundary tests, record the host proof as pending for
      the controller and never report a skipped boundary test as evidence —
      every requirement of this change.
- [ ] 13.6 Before the commit, validate the changed dependency graph with Rust
      1.88 using
      `cargo check --workspace --all-targets --all-features --locked`, so the
      CLI test target and its dev-only Boa harness are compiled, plus
      `cargo deny check licenses` and `cargo audit`; confirm `boa_engine` is
      exactly 0.21.1 with default
      features disabled, appears only in `brokkr-cli`'s development graph,
      and is absent from the release binary's production graph; if a local
      admission gate rejects Boa, return upstream to design instead of
      weakening the exact-served-code proof — every requirement
      of this change.
- [x] 13.7 Confirm the frozen set is untouched — `contracts/`,
      `policy/phase-machine.json`, `policy/schemas/`, `reference/`,
      `fixtures/` — and that the only decision file added is proposed
      0055 with its single registry row — every requirement of this
      change.
- [ ] 13.8 Fold the change into the living truth with the dialect's
      archive operation, `openspec archive read-every-transcript-kind
      --yes`, so the three deltas seed `openspec/specs/`; append under
      each touched capability's `## Provenance` heading the one pointer
      line the dialect's archive instructions spell — the archived
      directory name and the day it was folded — for
      `transcript-reading`, `transcript-command` and `transcript-tui`,
      without rewriting an existing line, and re-run
      `openspec validate --archived --strict --no-interactive` — every
      requirement of this change.
- [ ] 13.9 Commit the work unsigned in the repository's message style,
      and never push, merge, close the issue or start another run: the
      controller owns integration, host proof, PR, CI and delivery —
      every requirement of this change.

Post-commit controller evidence is deliberately outside the tracked
checkboxes above. After 13.9 fixes the candidate head, the controller records
the final CI evidence for workspace tests on Linux, macOS and Windows and for
the MSRV, license and audit jobs, with the remote MSRV job using the same
`--all-targets --all-features` arguments as 13.6. Those results remain pending
until they exist; they validate the unchanged commit produced by 13.9 and do
not cause another tracked edit. If any remote platform or admission gate
rejects Boa, the controller returns the change upstream to design instead of
weakening the exact-served-code proof.
