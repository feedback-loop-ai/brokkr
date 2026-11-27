# Unreleased

Changes merged since v0.16.1 that the next release's notes carry.

## Added

- **Antigravity CLI `--conversation` rejoin launch built (not switched on)**
  (#1227, decision 0075 addendum of 2026-11-27, proposed). `brokkr driver agy`
  now declares `resume` offer receipt and rejoins a kept conversation through
  the shared resume machinery (`resume_gate`, `LaunchPlan`, `LaunchHold`,
  `Confirmation`, and `LaunchTerminal`) when an enabling assessment covers the
  site's boundary, hands, MCP isolation, and `agy --version`: it acquires an
  exclusive advisory lock (`.rejoin.lock`) for the whole of restore, turn, and
  keep, restores kept `conversation.db` and `brain/` whole into the fresh
  engine HOME (operator-only `0700`/`0600`, reusing slice 1's bounds and
  no-symlink checks, rolling back partial writes on any failure), composes all
  restrictions fresh (HOME, gate hook, MCP set, `--settings`), launches with
  `--conversation <id>`, confirms `init.conversation_id` (stopping a stray
  conversation immediately as unsettled without keeping or recording it),
  counts only the turn's own `step_update` token sums on confirmed rejoins
  (omitting cumulative `result.num_turns` and `duration_seconds`), and after a
  confirmed rejoin replaces `rejoin/` whole on success or removes the old copy
  on keep failure (leaving it untouched when unconfirmed). Every decline runs
  cold with no `--conversation` and records a bounded `resume_refusal` token.
  `docs/examples/adapters/agy.json` keeps `resume.stream-work-site.status` at
  `"unsupported"` and the engine offers agy seats no session by default until a
  live rejoin through the driver is measured (slice 3)
  ([provider adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).

- **Antigravity CLI work seats keep rejoin state and record `root_session`**
  (#1227, decision 0075 addendum of 2026-11-26, proposed). `docs/examples/adapters/agy.json`
  now declares `record_kinds.root_session: "agy-conversation"` with
  `resume_shape: "stream-work-site"` (status still `"unsupported"`). After a
  declaring work turn, `brokkr driver agy` keeps `conversations/<id>.db` and
  `brain/<id>/` operator-only (`0700`/`0600`) under
  `~/.local/state/brokkr/agy-conversations/<id>/rejoin/` (bounded at 128 MiB,
  4,096 entries and 16 directory levels) and records a `harness-started`
  checkpoint with `root_session` (`kind: "agy-conversation"`, the conversation
  UUID, the version from `agy --version`, and `persistent: true` only when kept
  whole; an unsuccessful version probe or a masked candidate omits the row
  entirely). When an emitted `root_session`'s rejoin keep fails because a part
  is missing, unreadable, non-regular, over a bound, changed underfoot, or
  carries a bound secret value, rejoin persistence is refused (`persistent: false`
  on that emitted `root_session`, never `persistent: true`) and cleanup of staged
  files is attempted; an unlink I/O failure can leave a partial `rejoin/`
  directory on disk even though persistence is refused. Keeping guards against accidents
  (partial, linked, non-regular, over-bound, secret-bearing, or unpinned state)
  and promises no outcome against a same-user process modifying the engine HOME
  or kept tree during the keep (operator rulings of 2026-11-25 and 2026-11-26).
  Probes, gate seats, turns whose
  `init` did not confirm the pinned model, turns without a UUID, and undeclared
  seats are unchanged, and every offered session is still
  refused before launch because `--conversation` rejoin is not built yet
  ([provider adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).
