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
- **A boxed dsh seat's `brokkr-hands` overlay row passes the composed-tree
  line rule** ([#1277](https://github.com/feedback-loop-ai/brokkr/issues/1277)).
  dsh's `--dump-config` folds long JSON `--reach` and `--spec` values onto
  `>-` block-scalar content lines starting with `{`, which the driver's
  line-property closure refuses as `Unreadable`. The dsh overlay now carries
  those exact JSON bytes as unpadded base64url under `--reach-base64url` and
  `--spec-base64url`, while `--workdir` and `--artifacts` stay plain paths
  (fail closed if folded onto a refused line). `brokkr hands serve` accepts
  either the plain JSON or unpadded base64url spelling for each document:
  `--reach` requires exactly one spelling, `--spec` refuses both spellings
  and keeps main's `"workspace"` default when omitted, and both encoded
  flags accept leading-hyphen values so they reach the typed decoder.
  After base64url decoding, each document goes through the exact same
  parse as its plain JSON spelling (`serde_json::from_str::<Sealed>` for
  reach, with no intermediate `Value` or map; `serde_json::from_str::<Value>`
  plus `HandsSpec::checked` for spec), so both spellings admit and refuse
  the same documents. Encoded refusals take two exit codes and never echo raw
  values or decoded text: clap argument errors when an encoded option is
  present exit with code 2 and print only the payload-free error kind and usage
  line, while a refused encoded document or passing both spellings of one
  document with values exits with code 1 and its typed refusal
  (`Base64UrlError`, serde `Category`, `SpecRefusalKind`, or
  `DocumentError::BothSpellings`). When more than one thing is wrong, the
  command reports the first refusal it reaches, so a spec that is not a hands
  spec is refused before the reach spellings are compared. On `hands serve` and
  `hands exec`, refusals about a document given in the plain spelling (not
  JSON, not the reach, or not a hands spec) — even when the other document is
  given in its encoded spelling — and the missing-reach diagnostic preserve
  main's diagnostics byte for byte (still never showing an encoded document's
  value), while
  the usage synopsis inside any other clap error on `hands serve` (such as
  repeated `--reach`) is clap's for the present argument set and is not
  promised to match main's. Other
  harnesses' JSON MCP configurations and the dsh line rule itself are
  unchanged (see the [hands argument reference](../reference/hands.md) and
  proposed decision
  [0127](../decisions/0127-a-dsh-hands-row-carries-its-documents-as-unpadded-base64url.md)).
