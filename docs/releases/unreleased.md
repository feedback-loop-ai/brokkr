# Unreleased

Changes merged since v0.11.0 that the next release's notes carry.

## Breaking

- **The pre-rename names no longer configure anything.** Decision 0019's
  one-release window closed (#355). The `FORGE_*` environment overrides
  (`FORGE_CLAUDE_BIN`, `FORGE_LANETALLY_BIN`, `FORGE_CODEX_BIN`,
  `FORGE_DSH_BIN`, `FORGE_EXEC_NAME`, `FORGE_BROWSER_BIN`) are refused
  when set without their `BROKKR_*` name: the seat fails to start naming
  both, and neither the pinned binary nor the built-in one runs. Set the
  `BROKKR_*` name instead, or unset the old one. The `{forge}` bundle argv token no
  longer expands; write `{brokkr}`. An adapter file that still declares
  `binding_grant` is refused at load; declare `egress` (`"contracted"`
  for a true grant, `"uncontracted"` for a false one).
- **An override whose value is not UTF-8 is refused.** A `BROKKR_*`
  override that holds bytes that are not UTF-8 was read as unset, so the
  built-in ran in place of the pin. The seat, a linked-worktree DSH
  seat's scoped runner (`BROKKR_DSH_RUNNER`), `brokkr doctor`'s DSH
  selection and `brokkr ui`'s browser now refuse it by name (#355).
- **`python3` and `pytest` leave the shipped allow-lists** (decision 0041
  ruling 2): the claude and lanetally adapters no longer map them, and
  `recipes/fast` and `bundles/verify` no longer grant them. A realm
  scaffolded by `brokkr init` for a Python project keeps its own grants.
- **`intake-sdd` leaves the agent library.** No recipe seated it after
  `recipes/sdd` was retired.
