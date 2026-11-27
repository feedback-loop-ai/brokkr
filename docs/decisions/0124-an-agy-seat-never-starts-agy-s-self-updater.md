# 0124 — An agy seat never starts agy's self-updater

Status: accepted — operator ruled in chat 2026-11-26, "approve all that were built" (https://github.com/feedback-loop-ai/brokkr/issues/1428)
Date: 2026-11-21

Built: built

## Context

On 2026-11-21 at 08:41Z the operator's `agy` replaced itself, 1.5.13 to
1.5.14, while seats of the #1200 wager were starting. Every fact the agy
adapter declares was measured on one version, and a plain `agy` turn spawns
a background updater (`updater-default` in
`docs/evidence/adapters/agy-cli-1-5.md`). The environment variable
`AGY_CLI_DISABLE_AUTO_UPDATE=true` stops a turn from starting it
(`updater-disabled-true`); the value `1` does not (`updater-disabled`).

The alternatives weighed: leave the updater alone and re-measure after each
replacement, which lets a run change versions between two seats; pin the
binary by digest, which refuses every run after the operator's own `agy`
updates; or switch the updater off for the engine's seats only, which stops
the background updater from starting in a turn the engine launches and
leaves the operator's own use as it is; `agy update` run inside a turn is
not measured. The controller ruled the third on 2026-11-21
(#1224, D1), under the operator's ruling of 2026-11-20 on #1200.

## Rulings

1. Every agy child the driver spawns, boxed or not, carries
   `AGY_CLI_DISABLE_AUTO_UPDATE=true` in its environment, spelled exactly
   so, over any value the engine's own environment holds. The launch's argv
   and both HOME documents are unchanged; the child's environment gains this
   one name and nothing else. Enforcement binding: the agy adapter's tests
   record the fake's environment in boxed and unboxed launches, with the
   engine's own environment holding another value.
2. The engine owns that name for an agy seat as it owns `HOME` and `PWD`: a
   seat that binds it is refused before agy starts by the typed
   `LaunchError::Owned`, which names it, and no engine HOME is made.
   Enforcement binding: the owned-name admission check in
   `crates/brokkr-protocol/src/adapters/agy.rs` and its test.
3. The gate's hook process is unchanged.

## Consequences

- A turn the engine launches does not start agy's updater. That is all
  this decision builds: it does not keep a run on the binary it started
  with. Each seat launches the executable path afresh, so the installed
  binary can still change between two seats of one run by other hands,
  such as the operator's own use of `agy`, and the later seat runs the new
  version. An updated binary is a version the evidence record may not have
  measured, and nothing checks the version a seat launches (#1240).
  `brokkr doctor` shows the version it observes.
- The provider-adapters guide says so in its Antigravity section.
- A seat cannot switch the updater back on.
