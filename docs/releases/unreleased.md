# Unreleased

Changes merged since v0.16.0 that the next release's notes carry.

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
