# Unreleased

Changes merged since v0.16.1 that the next release's notes carry.

## Added

- **Antigravity CLI work seats keep rejoin state and record `root_session`**
  (#1227, decision 0075 addendum of 2026-11-26, proposed). `docs/examples/adapters/agy.json`
  now declares `record_kinds.root_session: "agy-conversation"` with
  `resume_shape: "stream-work-site"` (status still `"unsupported"`). After a
  declaring work turn, `brokkr driver agy` keeps `conversations/<id>.db` and
  `brain/<id>/` operator-only (`0700`/`0600`) under
  `~/.local/state/brokkr/agy-conversations/<id>/rejoin/` (bounded at 128 MiB,
  4,096 entries and 16 directory levels, all or nothing, and skipped when any
  bound secret value appeared in the conversation) and records a
  `harness-started` checkpoint with `root_session` (`kind: "agy-conversation"`,
  the conversation UUID, the version from `agy --version`, and
  `persistent: true` only when kept whole). Keeping guards against accidents
  (partial, linked, non-regular, over-bound, secret-bearing, or unpinned state)
  and promises no outcome against a same-user process modifying the engine HOME
  or kept tree during the keep (operator rulings of 2026-11-25 and 2026-11-26).
  Probes, gate seats, turns whose
  `init` did not confirm the pinned model, turns without a UUID, and undeclared
  seats are unchanged, and every offered session is still
  refused before launch because `--conversation` rejoin is not built yet
  ([provider adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).
