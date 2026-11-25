# Unreleased

Changes merged since v0.15.2 that the next release's notes carry.

## Added

- **Qoder CLI is a provisional harness** (#1178). `brokkr driver qoder`
  runs Qoder CLI 1.1 in print mode with a fixed tool list, the prompt on
  stdin and no session kept. The adapter ships as an example,
  `docs/examples/adapters/qoder.json`, which a world copies into its
  adapters directory. It maps five provisional models, holds no gate,
  declares no hands and is never resumed. A seat whose served model is
  not exactly its pin is refused, and so is a signed-out CLI. The pin
  must be a plain model id, and a stdout line that is not a JSON object
  refuses the seat (decision 0075's 2026-11-21 addendum).
  `BROKKR_QODER_BIN` overrides the binary. Every launch carries
  `--strict-mcp-config --mcp-config <the engine's file> --setting-sources
  ""`, the MCP isolation measured on Linux for the cold and replacement
  shapes without hands, so a qoder seat compiles there ([provider
  adapters](../guides/provider-adapters.md#qoder-cli-a-provisional-harness)).
- **Antigravity CLI is a trusted harness** (#1200, #1252). `brokkr driver
  agy` runs Antigravity CLI 1.5 as one stream-json turn read from stdin,
  under a HOME the engine makes for that turn alone and removes after it,
  or that the next launch removes once its owner is dead, with the
  operator's login and other top-level entries linked in; a seat that
  binds `HOME` is refused. The
  adapter ships as an example, `docs/examples/adapters/agy.json`, which a
  world copies into its adapters directory. It declares agy `trusted`,
  maps the four Gemini 4 models as promoted, names `agy-neon` and
  `agy-carbon` its judges (#1252, decision 0021's addendum of
  2026-11-22), and is never resumed; its route is class `contracted`. A seat whose served model is
  not exactly its pin is refused, and a turn agy stopped at a denial is
  never read as an answer: each row is judged as agy wrote it, one launch
  is one turn, and a denial once seen stands. Grants ride one
  `--settings` value, and one `PreToolUse` hook, `brokkr driver agy --
  --gate`, denies every tool outside the level-1 list, web search
  included. A workspace carrying agy's own `.agents` configuration in the
  working directory or any directory above it, through the filesystem
  root, past where agy's walk ends, is refused before spawn. `BROKKR_AGY_BIN` overrides the binary. The cold and replacement
  shapes without hands are measured on Linux, so an agy seat compiles
  there ([provider
  adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).
- **An Antigravity CLI seat holds boxed hands** (#1224). Agy's example
  adapter declares hands that ride no argument, and its cold and
  replacement shapes boxed, measured on Antigravity CLI 1.5.14 on Linux,
  so an agy work seat at a site that boxes its hands compiles, and a
  boxed gate compiles on a model the adapter names a judge (`agy-neon`,
  `agy-carbon`) and is refused on any other. A boxed launch leaves the model one tool, the
  engine's workspace tool: the engine HOME's MCP document names the hands
  server, the one `--settings` value holds its one rule, the gate lists
  `call_mcp_tool` alone, and there is no `--mode accept-edits`. Agy's own
  process still runs on the host, and its native tools are switched off
  by agy's hook, not by a boundary of the operating system. A boxed seat
  that passes `--allow` is refused before agy starts, and so is a launch
  under a boundary that boxes whose hands were not served, so a boxed seat
  never runs with agy's native tools. The unboxed launch
  is unchanged ([provider
  adapters](../guides/provider-adapters.md#a-boxed-agy-seat)).
- **An agy seat never starts agy's self-updater** (#1224): every agy child carries `AGY_CLI_DISABLE_AUTO_UPDATE=true`, and a seat that binds that name is refused.
- **Antigravity CLI transcripts are kept and readable in the run view** (#1227,
  decision 0075 level 2, proposed addendum of 2026-11-24).
  `docs/examples/adapters/agy.json` declares
  `record_kinds: {"transcript": "agy-conversation"}` (rejoin remains
  unsupported, with no `root_session` kind or `resume_shape`); the declaration
  crosses from the engine to the driver through one shared type
  (`DeclaredTranscriptKind`, validated by the one declared-kind grammar in
  `brokkr-core` shared with the adapter loader and runtime, refusing `"none"`
  and invalid spellings at launch), and seats without a declaring adapter keep
  nothing and record `none`. When declared, `brokkr driver agy` keeps nothing
  in every case the driver can see — probes (unchanged from main), refusals to
  start (judged by one pure `refusal_to_start` predicate over a single
  `result_path` observation carried on `Invocation`), missing `init`, non-UUID
  id, symlinked or non-regular source, source over 32 MiB, a reference the
  run view's shared home/path rule rejects (relative `HOME`, non-native drive
  or UNC spelling, or control characters such as a newline), or a row that `probe::seat_row` masking would
  alter — and otherwise copies
  `brain/<id>/.system_generated/logs/transcript.jsonl` before removing the
  engine HOME into
  `~/.local/state/brokkr/agy-conversations/<id>/transcript.jsonl` (owner-only
  parent with no group/other access, 0700 conversation directory, 0600 file,
  bounded at 32 MiB, exact single lowercase UUID read beside the fold, and
  no-symlink regular-file traversal) and records `agy-conversation` (an engine
  refusal of the row after the driver returns can leave an operator-only file
  for `#1323`'s reaper). `brokkr runs transcript`, the TUI and the browser read
  the kept file in file order (showing `SYSTEM_MESSAGE` turns, `RUNNING` rows
  as unfinished, counting a malformed newline-less final line as skipped,
  admitting only exact 20-byte `YYYY-MM-DDTHH:MM:SSZ` timestamps with a counted
  notice for any other present value including `null`, and rendering
  `tool_calls[].args` members verbatim without re-parsing or re-serializing
  string values) and mask both `Turn.ts` and block text through the shared
  `mask_secrets` path for every transcript kind; the kept file at rest is
  unmasked and readable only by the operator's UID. Nothing reaps kept
  conversations yet (#1323, alongside `dsh-homes`).

## Changed

- **`brokkr graph` prints its states and rules as two aligned tables**
  ([#1167](https://github.com/feedback-loop-ai/brokkr/issues/1167)),
  where it printed one `·`-separated line each.
  - **Width.** The tables fit a whole-number `COLUMNS`, else the window
    of the stdout they are written to, else 80 cells. A `COLUMNS` that is
    empty, 0 or not a number is ignored. The controlling terminal is never
    read, so a table piped to a narrower stdout fits that stdout.
  - **Fitting.** Every text column gives way before a row overflows. A
    cell is cut by its graphemes' terminal widths and marked `…`, and a
    cell that fits is never cut. The padding reads the width of the text
    printed, so the next column never shifts, combining marks and joined
    scripts included.
  - **Unchanged:** `--json`, `NO_COLOR` (no escapes; the header is bold
    only where colour is on), an empty table's header, and `brokkr
    seats`'s width (`COLUMNS`, else 80).

## Fixed

- **Review charters state what decision 0109 rulings 1 and 2 bind**
  ([#1322](https://github.com/feedback-loop-ai/brokkr/issues/1322)).
  `agents/charters/review-chief.md` now states that `security: true` on a
  finding is the typed hold requiring `security-hold` and that a lower
  security-relevant finding stays at its honest severity with `security:
  false` (the field is required), named in `notes`, with
  `has_security_residual: true` under `residual`. The four panel charters
  (`review-adversarial.md`, `review-correctness.md`, `review-security.md`,
  and `review-spec-compliance.md`) now state panel class inheritance
  (every member shares its enclosing panel's class; mixed panels fail to
  compile, and a work-class panel refuses any `findings` key, even `[]`,
  under ruling 1), instruct members to keep their result to `result`,
  `inputs`, and `notes` alone (omitting `findings` unless the whole panel
  is known to be gate-class) and put each finding in `notes` with its
  severity, file and line, and one concrete failing case, and state
  ruling 2's `security: true` binding for their declared result sets.
