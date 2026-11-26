# Unreleased

Changes merged since v0.16.0 that the next release's notes carry.

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
  `persistent: true` only when kept whole). Probes, gate seats, turns without a
  UUID, and undeclared seats are unchanged, and every offered session is still
  refused before launch because `--conversation` rejoin is not built yet
  ([provider adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness)).
- **`dsh` maps `qwen4-max` to `dashscope/qwen4-max`** (#1395). The
  shipped `dsh` adapter adds Qwen 4.0 Max as a work model and
  review-panel member on the Model Studio (`dashscope`) route, with
  `judges` empty, `trust_tier` unchanged (`untrusted`) and no route
  overlay or endpoint shipped. A host serving the route declares the
  `dashscope` provider, `DASHSCOPE_API_KEY` by name and the `qwen4-max`
  model row with its three measured `reasoningEfforts` levels (`low`,
  `high` and `xhigh`) ([provider
  adapters](../guides/provider-adapters.md#the-alias-catalogue)).
- **Antigravity CLI maps its two Claude 5.5 models as work models**
  (#1382). `docs/examples/adapters/agy.json` adds `agy-claude-opus`
  (`claude-opus-5-5`) and `agy-claude-sonnet` (`claude-sonnet-5-5`) for
  work seats at `low`, `medium` and `high` effort; `xhigh` is refused as
  a launch refusal, as for `agy-carbon` and `agy-argon`. Neither is in
  `judges`, and promotion is a separate ruling under decision 0021 after
  an eval ([provider
  adapters](../guides/provider-adapters.md#antigravity-cli-a-trusted-harness),
  [evidence
  record](../evidence/adapters/agy-cli-1-5.md#eighth-measurement-on-1517-the-two-claude-slugs-2026-11-25-and-2026-11-26)).
- `brokkr orders` lists work orders and can show, propose, edit and rule on them in the journal (#1272).

## Changed

- **macOS test shards move off pull requests to nightly, manual dispatch and
  release runs, replaced by `build (macos-latest)`**
  ([#1383](https://github.com/feedback-loop-ai/brokkr/issues/1383), operator
  ruling 2026-11-25). Contributors and the merge queue now run Linux test
  shards alone under `test (ubuntu-latest)` (so a macOS test failure can no
  longer fail a Linux verdict or block a pull request) plus the required macOS
  compile check `build (macos-latest)`, which takes `test (macos-latest)`'s
  place among `main`'s twelve required status checks and compiles the workspace
  and every test target with `--all-features --locked --no-run` without running
  tests.
  Releasers and maintainers keep the unchanged five macOS test shards and
  Seatbelt Gate A under `test (macos-latest)` in one shared reusable workflow
  (`.github/workflows/macos-test-shards.yml`), called nightly against `main`
  and on manual `workflow_dispatch` (`.github/workflows/macos-tests.yml`, plus
  `workflow_dispatch` on `ci.yml`), and in `.github/workflows/release.yml`
  from a cold tree before `build` and `publish`. A failing nightly opens or
  updates one issue carrying the `macos-nightly` label with the run URL and
  failed jobs; a passing nightly leaves any open issue for a person to close.
