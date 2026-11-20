# Unreleased

Changes merged since v0.15.1 that the next release's notes carry.

## Added

- **Qoder CLI is a provisional harness** (#1178). `brokkr driver qoder`
  runs Qoder CLI 1.1 in print mode with a fixed tool list, the prompt on
  stdin and no session kept. The adapter ships as an example,
  `docs/examples/adapters/qoder.json`, which a world copies into its
  adapters directory. It maps five provisional models, holds no gate,
  declares no hands and is never resumed. A seat whose served model is
  not exactly its pin is refused, and so is a signed-out CLI.
  `BROKKR_QODER_BIN` overrides the binary. Every launch carries
  `--strict-mcp-config --mcp-config <the engine's file> --setting-sources
  ""`, the MCP isolation measured on Linux for the cold and replacement
  shapes without hands, so a qoder seat compiles there ([provider
  adapters](../guides/provider-adapters.md#qoder-cli-a-provisional-harness)).
- **Antigravity CLI is a provisional harness** (#1200). `brokkr driver
  agy` runs Antigravity CLI 1.5 as one stream-json turn read from stdin,
  under a HOME the engine makes for that turn alone and removes after it,
  with the operator's login and other top-level entries linked in. The
  adapter ships as an example, `docs/examples/adapters/agy.json`, which a
  world copies into its adapters directory. It maps the four Gemini 4
  models as provisional, holds no gate, declares no hands and is never
  resumed; its route is class `contracted`. A seat whose served model is
  not exactly its pin is refused, and a turn agy stopped at a denial is
  never read as an answer. Grants ride one `--settings` value, and one
  `PreToolUse` hook, `brokkr driver agy -- --gate`, denies every tool
  outside the level-1 list, web search included. A workspace carrying
  agy's own `.agents` configuration up to its git root is refused before
  spawn. `BROKKR_AGY_BIN` overrides the binary. The cold and replacement
  shapes without hands are measured on Linux, so an agy seat compiles
  there ([provider
  adapters](../guides/provider-adapters.md#antigravity-cli-a-provisional-harness)).
