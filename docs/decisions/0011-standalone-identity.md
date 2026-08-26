# 0011 — Standalone identity: no origin, only heritage

**Status**: accepted (operator ruling, 2026-08-26; strengthened the same
day to full obliteration)

## Ruling

The Forge is a standalone product with no reference to the workspace it
was extracted from — not in the tree, not in the documentation, and not
in git history. The referee-checking-an-LLM-loop idea the engine grew
from is not novel to any workspace; this project is its own.

Enacted:

- The origin-specific heritage material (the delivery skill prose, the
  seat charters, the security catalog) is deleted outright, including
  from history (`git filter-repo` path purge). What remains under
  `reference/` is genuinely generic: the referee-era control plane, the
  retired oracle, the handoff protocol, provider config, the workflow
  drivers, and the recorded schemas — scrubbed of origin identifiers,
  schema `$id`s re-homed under feedback-loop.ai/forge/heritage/.
- Every historical blob and commit message is rewritten to neutral
  wording; all commit shas change; `main` and tags are force-pushed and
  the v0.1.0 release re-cut from the rewritten history.
- `policy/phase-machine.json` remains byte-identical: it never named
  the origin, and the frozen evaluator corpus derives from it — its
  stability is contract.
- The forward roadmap item is **the first external workspace profile**;
  no workspace is privileged.

## Why

Provenance earned its place while the engine was a port with a parity
obligation; that ended at decision 0009. A standalone product's history
should read as its own. The journals of pre-rewrite runs reference
pre-rewrite commit shas — those journals are local operational
evidence, unaffected and honest about when they were recorded.
