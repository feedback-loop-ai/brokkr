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
- **Antigravity CLI is a provisional harness** (#1200). `brokkr driver
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
  included. A workspace carrying agy's own `.agents` configuration up to
  where agy's walk ends, a `.git` file or the root git names, is refused
  before spawn. `BROKKR_AGY_BIN` overrides the binary. The cold and replacement
  shapes without hands are measured on Linux, so an agy seat compiles
  there ([provider
  adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).
- **An Antigravity CLI seat holds boxed hands** (#1224). Agy's example
  adapter declares hands that ride no argument, and its cold and
  replacement shapes boxed, measured on Antigravity CLI 1.5.13 on Linux,
  so an agy seat at a site that boxes its hands compiles. A boxed launch leaves the model one tool, the
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
