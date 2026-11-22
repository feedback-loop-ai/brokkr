# Unreleased

Changes merged since v0.15.2 that the next release's notes carry.

## Added

- **Qoder CLI is a provisional harness** (#1178). `brokkr driver qoder`
  runs Qoder CLI 1.1 in print mode with a fixed tool list, the prompt on
  stdin and no session kept. The adapter ships as an example,
  `docs/examples/adapters/qoder.json`, which a world copies into its
  adapters directory. It maps five provisional models, holds no gate
  and is never resumed. A seat whose served model is
  not exactly its pin is refused, and so is a signed-out CLI. The pin
  must be a plain model id, and a stdout line that is not a JSON object
  refuses the seat (decision 0075's 2026-11-21 addendum).
  `BROKKR_QODER_BIN` overrides the binary. Every launch carries
  `--strict-mcp-config --mcp-config <the engine's file> --setting-sources
  ""`, the MCP isolation measured on Linux for the cold and replacement
  shapes without hands, so a qoder seat compiles there ([provider
  adapters](../guides/provider-adapters.md#qoder-cli-a-provisional-harness)).
- **A Qoder CLI seat holds boxed hands** (#1235). The example adapter
  declares `"hands": {"workspace": []}` and its cold and replacement
  shapes boxed, measured on Qoder CLI 1.1.95 on Linux; its shapes without
  hands and its carriage now cite 1.1.95 too. Where a site boxes its
  hands, `brokkr driver qoder` launches `--permission-mode dont_ask`,
  `--tools ""` and one grant, the engine's `workspace` tool, beside the
  engine's server alone; a boxed seat that passes `--allowed-tools` of
  its own is refused before Qoder starts. Qoder does not fail closed, so
  the driver does: a boxed turn whose init does not show the engine's
  server `connected`, or shows any other server or tool, is refused, and
  one that records a permission denial or a tool error saying the
  engine's server is not connected fails. A seat compiled boxed that the
  standing MCP fence serves no measured shape is refused, never launched
  with Qoder's own tools. The unboxed seat is unchanged ([Qoder boxed
  hands](../guides/provider-adapters.md#qoder-boxed-hands)).

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
