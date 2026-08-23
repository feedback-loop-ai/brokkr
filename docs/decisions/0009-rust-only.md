# 0009 — Rust only: the oracle retires, drivers move into the binary

**Status**: accepted (operator directive, 2026-08-23 — "clean it up;
shouldn't we be on rust only?")

## Ruling

The repository is Rust-only. Decision 0003's retirement clause is
exercised: the differential and replay parity suites have long been
green, so the Python evaluator, its test suite, and the corpus generator
retire to `reference/oracle/` — read-only provenance, like everything
else in `reference/`. The committed evaluator corpus
(`fixtures/evaluator/corpus.ndjson`) is what it always was per
contracts/README: a FROZEN behavior contract. It is never regenerated;
a policy-semantics change would ship a new corpus version beside it.

The driver adapters move into the engine binary: `forge driver
<claude|codex|exec>` — the same protocol behavior, prompt composition,
and result-file contract as the retired Python adapters, byte-compatible
with the existing charters. Bundles reference them as
`{forge} driver <kind> -- <extra args>`; `{forge}` resolves to the
running engine's own executable. `forge init` scaffolds no driver files
at all.

Consequences:

- Installing and running Forge requires no Python anywhere — 0003's "one
  executable" promise is now literal. `forge doctor` demotes python3 to
  an optional warning (exec script templates may still want it).
- The loader-rejection lint suite and the driver conformance suite are
  ported to Rust (`policy_lint.rs`, `driver_conformance.rs` with shell
  shims); CI drops the Python job.
- The driver protocol remains language-neutral: `exec` still runs any
  external harness (dsh/Surface, ssh-remote, scripts), and a
  third-party driver in any language still speaks forge-driver/v1 out
  of process. Rust-only describes THIS repository, not the extension
  boundary.

## Why

The Python layer earned its keep as the executable specification during
the port and stopped earning it the day parity went green. Two runtimes
for one semantics is drift surface; a bundle that needs python3 for its
own drivers is an installation lie against 0003. The reference tree is
exactly the place this repo retires proven-out implementations.
