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
  `BROKKR_QODER_BIN` overrides the binary. On Linux a qoder seat does
  not compile yet, because Qoder's MCP isolation is unmeasured ([provider
  adapters](../guides/provider-adapters.md#qoder-cli-a-provisional-harness)).
