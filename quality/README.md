# Code-health baselines

The measurements every code-moving story in epic #330 is judged against (#335). They were recorded on `main` at `addd33ca` (2026-09-25), before any code moved. #337 and #338 turn them into ratchets that may only shrink. Decision 0071 ruling 4 takes its ceilings from them.

| File | What it holds |
|---|---|
| `crap-baseline.json` | cargo-crap's per-function report: cyclomatic complexity, coverage and CRAP for every production function, sorted by file |
| `file-lines.txt` | Lines per Rust file, production and test apart, sorted by path |
| `jscpd-baseline-prod.json` | Clone fingerprints in production Rust |
| `jscpd-baseline-tests.json` | Clone fingerprints in test Rust |
| `jscpd-baseline-data.json` | Clone fingerprints in JSON and Markdown. `contracts/`, `reference/` and `fixtures/` are left out because they are deliberately frozen copies, and so are their in-crate twins that tests pin byte for byte to them: `crates/brokkr-store/src/seat-record.v*.schema.json` and `crates/brokkr-cli/dialects/`. `.github/lint/`, the lint tools' generated npm lockfile, is left out too |
| `too-many-lines.txt` | Every function over clippy's default 100 lines, production and test apart, with its line count and location |
| `suppressions.txt` | Every `#[expect]` and `#[allow]` in the Rust sources, by lint, production and test apart. The suppressions test (`crates/brokkr-cli/tests/suppressions.rs`) holds the tree to it exactly, so a count moves only by an edit of this file (#337) |
| `duplicate-skips.txt` | Every duplicate crate version `deny.toml`'s `[bans]` skips, as `name@version`. The layering test (`crates/brokkr-cli/tests/layering/`) holds the skip list to it exactly, so a new skip needs an edit of this file (#337) |
| `measure.sh` | The exact commands that produce all of the above |

## Refresh

```sh
TMPDIR=/var/tmp/<dir> scripts/coverage-exact.sh   # writes target/coverage/lcov.info
quality/measure.sh
git diff quality/
```

The output is deterministic: two runs over the same tree are byte-identical. The diff is the review.

Keep `TMPDIR` off `/home`: the hands tests assert that `/home` is empty inside their box.

## Tools

| Tool | Version | Role |
|---|---|---|
| cargo-crap | 0.5.0 | complexity and CRAP |
| jscpd | 5.3.2 (the Rust rewrite, `cargo install --locked jscpd --version 5.3.2`) | duplication |
| clippy | 1.98.0 stable | function length |
| cargo-llvm-cov | 0.9.1 | the LCOV, through the exact gate |
| Rust nightly | `rust-nightly-version.txt` (nightly-2026-09-05) | the LCOV, through the exact gate |

## Definitions

- **Production or test.** A test file is one the exact coverage gate treats as a test, by `scripts/coverage-exact.sh`'s own pattern: under a `tests/` directory, or named `tests.rs` or `*_tests.rs`. Everything else in `crates/` is production.
- **Paths and order.** Paths are repository-relative. cargo-crap writes them with a leading `./`, which a ratchet using the same command matches. A function's key is its file, name and line together, because `cfg` twins share a name in one file (`exec_bwrap`, `Dir::child`). Every list is sorted with `LC_ALL=C`, so hosts with different locales produce the same bytes.
- **CRAP.** CRAP = CC² × (1 − coverage)³ + CC. The gate holds production at 100% coverage, so CRAP equals cyclomatic complexity here. This is a complexity baseline, and it becomes a true CRAP baseline only if the gate ever relaxes. cargo-crap counts `if`, loops, match arms, `&&`, `||` and `?`.
- **Function lines.** Function lines are clippy's count: code lines only, with comments and blank lines excluded. A raw count of the same functions runs higher, which is why the issue's rough count said 64 production functions over 100 lines, not the 41 measured here.

## Traps found while measuring

- **jscpd's `--max-lines` caps whole files, not clone blocks.** At its default of 1,000, jscpd silently skips every file longer than that. That is 19 of the 57 production files, the very modules this baseline watches. `measure.sh` raises it to 100,000, and with that jscpd reads 61,193 of the 61,201 production lines.
- **cargo-crap in `--workspace` mode records absolute paths.** It also walks test directories nested under `src/`, and scores those at 0% coverage because they are not in the LCOV. `measure.sh` runs `--path .` with every test path excluded. That gives repository-relative paths a ratchet on another checkout can match, and all 54 analyzed files match the LCOV.
- **An `#[allow(clippy::too_many_lines)]` hides a function from a plain clippy run.** `measure.sh` uses `--force-warn`, which an attribute cannot silence, so the list is complete.

## The numbers at a glance

The per-function line and parameter rows come from a one-off clippy pass with every threshold at 0, posted on #335. No committed artifact holds them, and `measure.sh` does not produce them. Every other row is derived from the files here.

| Measure | Count | p50 | p90 | p95 | p99 | Max |
|---|---|---|---|---|---|---|
| CC per production function | 1,615 | 3 | 11 | 16 | 35 | 195 |
| Lines per production function | 1,656 | 8 | 47 | 72 | 163 | 751 |
| Lines per test function | 3,808 | 18 | 66 | 91 | 179 | 729 |
| Parameters per production function | 1,573 | 2 | 4 | 5 | 8 | 11 |
| Lines per production file | 57 | 607 | 2,872 | 4,014 | 4,904 | 6,353 |
| Lines per test file | 123 | 466 | 3,187 | 4,694 | 15,075 | 20,404 |

| Duplication (jscpd, 50 tokens and 5 lines minimum) | Clones | Duplicated lines | Share |
|---|---|---|---|
| Production Rust | 76 | 753 | 1.23% |
| Test Rust | 1,021 | 10,531 | 6.48% |
| JSON and Markdown | 268 | 10,902 | 10.09% |

#335 carries the 20 worst offenders of each measure, and the proposed ceilings with their evidence.
