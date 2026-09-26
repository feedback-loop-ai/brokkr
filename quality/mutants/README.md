# Mutation baseline

The exact coverage gate proves every production line runs. It cannot prove any test would notice that line changing. [cargo-mutants](https://mutants.rs) answers that second question: it makes one small change at a time (a comparison flipped, a return value replaced, a body emptied) and runs the tests, and a mutant no test catches is a line no test checks (#289).

This directory is the committed baseline, measured on `main` at `155aa5f0` (2026-09-26) before any gate, and shrunk by #419, which closes 26 of its 33 misses:

| File | Holds |
|---|---|
| `brokkr-core.missed.txt` | Every missed mutant in brokkr-core, as cargo-mutants writes them |
| `brokkr-protocol.missed.txt` | Every missed mutant in brokkr-protocol's measured scope |

`scripts/mutants.sh` holds the scope and compares a run's misses with these files. A miss's identity is its file and mutation, without the line and column, so an unrelated edit above it does not make it new. `.github/workflows/mutants.yml` runs it on pull requests (only what the diff touched) and weekly in eight shards. A tool error, a red unmutated tree, a diff that does not carry every change in the scope as a hunk, a run that tested nothing, an output jq cannot parse as one, or a missing allow-list, shard or output fails every job, because its verdict or report would not be true.

Because the identity has no line, only a run over the whole scope subtracts the committed misses outright, and it counts them: a second miss of a committed file and mutation is new. brokkr-protocol's pull-request report mutates only its diff, so it names every miss and marks the ones that share a committed miss's file and mutation, for a reader to check the line. A weekly shard mutates an eighth of the scope, so no shard subtracts; the week's report joins all eight shards and compares once. A new miss is hidden only when a committed miss of the same file and mutation is fixed in the same week. Timeouts are listed beside the misses, because a timeout can hide one.

## The gate

The operator ruled on #289 (2026-09-26):

- **brokkr-core is gated.** The pull-request job `mutants in the diff: brokkr-core` runs `scripts/mutants.sh gate <base> brokkr-core`. It fails, naming each one, on any miss in the diff whose file and mutation occur more often than in `brokkr-core.missed.txt`. A committed miss at a moved line is accounted for once; a second miss of the same file and mutation fails. A diff that touches no brokkr-core mutant passes without a run. The job is required once branch protection names it.
- **The diff is rendered for cargo-mutants, which lists nothing for a change it cannot read.** Every file is rendered as text, with no external diff or textconv, no colour, the `a/` and `b/` prefixes, and no rename detection. A NUL byte in a comment, or a `-diff` attribute, otherwise made git print `Binary files ... differ` and no hunk, and the gate passed (the #420 landing's first hold). A marker git still prints in place of a hunk is refused. So is every path in the scope whose content changed and no hunk heads, before anything is listed. A mode change, or an empty file added or deleted, has nothing to mutate and needs no hunk.
- **brokkr-protocol reports only**, in its own pull-request job and in the weekly shards, until three things hold: its hung mutants are reaped promptly, `hands.rs` is also measured against brokkr-cli's `hands` suite, and two weekly cycles come back clean.

`crates/brokkr-cli/tests/mutants_gate.rs` runs the real script against a stub `cargo` and a planted allow-list (`MUTANTS_ALLOW`), so the gate's verdict is pinned without a mutation run: a new miss fails, a moved committed miss passes, a second occurrence fails, an empty diff passes, and every failure to measure fails, an output the gate cannot read included: exit 2 with no miss in missed.txt, and a mutants.json that jq does not parse, as one document, into a non-empty array of objects (`[{bad}]` passed a bracket check, the landing's second hold). Scratch repositories hold the render: each git setting that would drop a hunk is planted and the change is still measured, and a planted render with a marker, or without a scoped path's hunk, is refused before listing.

A diff mutates only part of the scope, so the gate cannot tell a new miss from a committed one of the same file and mutation that the diff did not reach; that miss passes the gate, and the weekly report over the whole scope names it.

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

cargo-mutants 27.1.0 with `.cargo/mutants.toml`: test paths excluded, `timeout_multiplier = 3.0`. Rust 1.98.0. `jq`, which reads mutants.json; the script refuses to start without it.

## Scope and results

| Crate | Scope | Mutants | Caught | Missed | Unviable | Hung (counts as caught) | Time |
|---|---|---|---|---|---|---|---|
| brokkr-core | the whole crate | 371 | 348 | 5 | 18 | 0 | 9m27s at `-j 6` |
| brokkr-protocol | `secret.rs`, `hands.rs` | 362 | 306 | 28 | 19 | 9 | 1h30m at `-j 6` plus a 43m re-test at `-j 3` |

- **A timeout was re-tested before it counted.** The first runs shared a host at load average 60 to 150. Protocol's suite took 43 seconds there, against 8 seconds unloaded, so a 131-second timeout could hide a miss. Every timeout was re-run at `-j 3` with a 300- or 420-second limit:
  - core's three were all caught;
  - of protocol's 22, 10 were caught, 3 were misses (added to the allow-list), and 9 hang for real and count as caught.
- **Six of protocol's 28 misses were caught only in another crate.** cargo-mutants runs only the mutated package's tests by default, and `run_boxed` and `run_boxed_in` were exercised only from `crates/brokkr-cli/tests/hands.rs`. #419 holds them in brokkr-protocol's own `tests/hands_exits.rs`, with stand-ins for `bwrap` and `git`, so a package-scoped run catches them, and `hands.rs:1189` too.
- **#419 closed 26 of the misses and left seven.** 25 are caught by a test written for each; the table above is the `155aa5f0` measurement.
  - The 26th, `secret.rs:227` (`search = name_end + 2` becoming `name_end - 2` in `scan_secret_refs`), was equivalent and could not be caught. The closing `}}` cannot begin `secret:`, so the search now resumes at `name_end` and the `+ 2` it mutated is gone.
- **Seven of the misses cannot be caught by any test here.**
  - Two are `secret.rs:498` and `:499` (`|` becoming `^` in `b64`). They are equivalent: the three shifted bytes never overlap, so OR and XOR are the same value.
  - Three are at `hands.rs:678`, the `cfg(not(unix))` twin of `ids()`, which neither supported host compiles. Decision 0063 retires it when the file is next edited.
  - Two are at `hands.rs:56` (`boundary_evidence_required`). They are equivalent on a host whose boundary proofs never skip.
- **The rest of brokkr-protocol is unmeasured.** That is 1,907 mutants, in `adapters.rs`, `adapters/composite.rs`, `adapters/route_overlay.rs`, `dsh_sandbox.rs`, `adapters/composite/image.rs` and the small files. At 8 to 43 seconds a suite, the whole crate is several hours at `-j 3` to `-j 6`, and it is left for a measured-later follow-up. The weekly shards cover only the measured scope until it grows.
