# Mutation baseline

The exact coverage gate proves every production line runs. It cannot prove any test would notice that line changing. [cargo-mutants](https://mutants.rs) answers that second question: it makes one small change at a time (a comparison flipped, a return value replaced, a body emptied) and runs the tests, and a mutant no test catches is a line no test checks (#289).

This directory is the committed baseline, measured on `main` at `155aa5f0` (2026-09-26) before any gate:

| File | Holds |
|---|---|
| `brokkr-core.missed.txt` | Every missed mutant in brokkr-core, as cargo-mutants writes them |
| `brokkr-protocol.missed.txt` | Every missed mutant in brokkr-protocol's measured scope |

`scripts/mutants.sh` holds the scope and compares a run's misses with these files. A miss's identity is its file and mutation, without the line and column, so an unrelated edit above it does not make it new. `.github/workflows/mutants.yml` runs it on pull requests (only what the diff touched) and weekly in eight shards. Both report only: a miss prints, it never fails a check. The operator rules a gate from this baseline. A tool error, a red unmutated tree, a bad diff, a run that tested nothing, or a missing allow-list, shard or output fails the job instead, because the report would not be true.

Because the identity has no line, only a run over the whole scope subtracts the committed misses, and it counts them: a second miss of a committed file and mutation is new. A pull request mutates only its diff, so its report names every miss and marks the ones that share a committed miss's file and mutation, for a reader to check the line. A weekly shard mutates an eighth of the scope, so no shard subtracts; the week's report joins all eight shards and compares once. A new miss is hidden only when a committed miss of the same file and mutation is fixed in the same week. Timeouts are listed beside the misses, because a timeout can hide one.

## Refresh

```sh
TMPDIR=/var/tmp/<dir> MUTANTS_JOBS=6 BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 \
  scripts/mutants.sh baseline brokkr-core
TMPDIR=/var/tmp/<dir> MUTANTS_JOBS=6 BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 \
  scripts/mutants.sh baseline brokkr-protocol
git diff quality/mutants/
```

- **`TMPDIR`:** cargo-mutants copies the tree into `TMPDIR` once per job. Keep it off `/tmp`, which is a tmpfs on the operator's host.
- **`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`:** a skipped boundary proof then fails. Otherwise a host with no namespaces would let every `hands.rs` mutant survive unseen.

## Tools

cargo-mutants 27.1.0 with `.cargo/mutants.toml`: test paths excluded, `timeout_multiplier = 3.0`. Rust 1.98.0.

## Scope and results

| Crate | Scope | Mutants | Caught | Missed | Unviable | Hung (counts as caught) | Time |
|---|---|---|---|---|---|---|---|
| brokkr-core | the whole crate | 371 | 348 | 5 | 18 | 0 | 9m27s at `-j 6` |
| brokkr-protocol | `secret.rs`, `hands.rs` | 362 | 306 | 28 | 19 | 9 | 1h30m at `-j 6` plus a 43m re-test at `-j 3` |

- **A timeout was re-tested before it counted.** The first runs shared a host at load average 60 to 150. Protocol's suite took 43 seconds there, against 8 seconds unloaded, so a 131-second timeout could hide a miss. Every timeout was re-run at `-j 3` with a 300- or 420-second limit:
  - core's three were all caught;
  - of protocol's 22, 10 were caught, 3 were misses (added to the allow-list), and 9 hang for real and count as caught.
- **Six of protocol's 28 misses are caught in another crate.** cargo-mutants runs only the mutated package's tests by default. `run_boxed` and `run_boxed_in` are exercised from `crates/brokkr-cli/tests/hands.rs`. Against that suite (`--test-package brokkr-cli --cargo-test-arg=--test --cargo-test-arg=hands`), six of those seven mutants are caught, and only `hands.rs:1189` (a signal's exit code, `-1` becoming `1`) survives. They stay in the allow-list because pull-request runs are package-scoped too.
- **Seven of the misses cannot be caught by any test here.**
  - Two are `secret.rs:498` and `:499` (`|` becoming `^` in `b64`). They are equivalent: the three shifted bytes never overlap, so OR and XOR are the same value.
  - Three are at `hands.rs:678`, the `cfg(not(unix))` twin of `ids()`, which neither supported host compiles. Decision 0063 retires it when the file is next edited.
  - Two are at `hands.rs:56` (`boundary_evidence_required`). They are equivalent on a host whose boundary proofs never skip.
- **The rest of brokkr-protocol is unmeasured.** That is 1,907 mutants, in `adapters.rs`, `adapters/composite.rs`, `adapters/route_overlay.rs`, `dsh_sandbox.rs`, `adapters/composite/image.rs` and the small files. At 8 to 43 seconds a suite, the whole crate is several hours at `-j 3` to `-j 6`, and it is left for a measured-later follow-up. The weekly shards cover only the measured scope until it grows.
