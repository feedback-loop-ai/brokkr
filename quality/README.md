# Code-health baselines

The measurements every code-moving story in epic #330 is judged against. #335 recorded them on `main` at `addd33ca` (2026-09-25), before any code moved. #338 regenerated them at `1d890c6a` and turned them into ratchets that CI holds the tree to, and that may only shrink. Decision 0071 ruling 4 takes its ceilings from them. #337 ratchets function length, nesting and parameters separately, through `#[expect]`.

| File | What it holds |
|---|---|
| `crap-baseline.json` | cargo-crap's per-function report: cyclomatic complexity, coverage and CRAP for every production function, sorted by file. The packages the exact gate excludes by name (`coverage_exclusions` in `scripts/coverage-exact.sh`) are excluded here too, since the LCOV holds none of their code |
| `file-lines.txt` | Lines per Rust file, production and test apart, sorted by path |
| `jscpd-baseline-prod.json` | Clone fingerprints in production Rust |
| `jscpd-baseline-tests.json` | Clone fingerprints in test Rust |
| `jscpd-baseline-data.json` | Clone fingerprints in JSON and Markdown. What is left out, and why, is listed in `lib.sh`: frozen copies and their pinned in-crate twins, append-only history (`openspec/changes/archive/`, `docs/evidence/`), the lint tools' generated lockfile, and build and run output |
| `public-api/<crate>.txt` | Each library crate's public API, from cargo-public-api on the pinned nightly |
| `too-many-lines.txt` | Every function over clippy's default 100 lines, production and test apart, with its line count and location. #337's reference |
| `suppressions.txt` | Every `#[expect]` and `#[allow]` in the Rust sources, by lint, production and test apart. The suppressions test (`crates/brokkr-cli/tests/suppressions.rs`) holds the tree to it exactly, so a count moves only by an edit of this file (#337) |
| `duplicate-skips.txt` | Every duplicate crate version `deny.toml`'s `[bans]` skips, as `name@version`. The layering test (`crates/brokkr-cli/tests/layering/`) holds the skip list to it exactly, so a new skip needs an edit of this file (#337) |
| `ceilings.json` | The ceilings decision 0071 ruling 4 ruled: CC 15 for a new function, 800 lines per production file, 2,000 per test file |
| `lib.sh` | One home for every measuring command, shared by the two scripts below |
| `measure.sh` | Regenerates every baseline except `duplicate-skips.txt`, which is edited by hand beside `deny.toml` |
| `ratchet.sh` | Holds the tree to the baselines. Its one table names every file under `quality/` (`ratchet.sh table`, `ratchet.sh listings`) |
| `mutants/*.missed.txt` | #289's mutation allow-lists, which `ratchet.sh baselines` also holds: a new miss needs a ruling |

Every baseline names the tool and version that produced it: a `producedBy` object in each JSON file, and a first `# produced by` line in each text file.

## The ratchets

| Check | What fails | Where it runs |
|---|---|---|
| `ratchet.sh crap` | A function's cyclomatic complexity above its allowance, a measurement not at 100% coverage, a scan and an LCOV that disagree about which files exist, or a report that is empty or not a JSON object | The coverage job, after the exact gate, on its LCOV |
| `ratchet.sh api` | A library crate's public API that differs from its snapshot, or a snapshot with no crate | The coverage job, on the same pinned nightly |
| `ratchet.sh files` | A Rust file longer than its allowance | The `ratchets` job ("baseline ratchets") |
| `ratchet.sh clones` | A jscpd clone whose fingerprint is not in its scope's baseline, a scan that read no file, or a report that is empty or not a report | The `ratchets` job ("baseline ratchets") |
| `cargo shear --deny-warnings --locked` | An unused dependency | The `ratchets` job ("baseline ratchets") |
| `ratchet.sh baselines <rev>` | A baseline raised since `<rev>` with no ruling named | The `ratchets` job ("baseline ratchets"), on pull requests |

**Allowances.** A function may reach CC 15 or its baseline, whichever is higher, as the operator ruled on 2026-09-26 (decision 0071 ruling 4). A new function stays within 15, an existing one may change freely up to 15, and one already over 15 may only shrink. A file may grow to its ceiling or its baseline, whichever is higher.

**Matching.** cargo-crap matches a function to its baseline by file and name, not by line, so an edit above a function does not make it new. A function moved to another file, or renamed, is new. If it is over CC 15, its baseline entry moves with a ruling (see below).

**Every listing, and when it may be empty.** `ratchet.sh`'s one table names every file under `quality/` as a rule, a doc or a listing. `baselines` iterates that table, and the tests enumerate it through `ratchet.sh listings`, so a new baseline cannot arrive unguarded. Each listing has an entry counter and an empty policy:

| Listing | Empty policy |
|---|---|
| `crap-baseline.json`, `file-lines.txt`, `public-api/*.txt` | never: a tree always has functions, Rust files and public items, so zero entries is a measurement that failed |
| `too-many-lines.txt`, `jscpd-baseline-*.json`, `suppressions.txt`, `duplicate-skips.txt`, `mutants/*.missed.txt` | from-empty: zero entries is allowed only where the base was already empty or absent. Today `mutants/brokkr-core.missed.txt` is the one listing empty at the base |

A from-empty listing that genuinely reaches zero (the last long function split, the last clone removed) is read as a failed measurement, like one that silently printed nothing. Letting it through is a ruled change to the table.

`measure.sh` also refuses at the source:
- it writes `too-many-lines.txt` only from a clippy run whose JSON stream reports `build-finished` with `success: true`, and that raised no unknown-lint (`E0602`) or renamed-lint diagnostic for the forced lint;
- every jscpd scan runs with `--fail-on-empty`.

**Moving a baseline.** Run `measure.sh` and commit the result (edit `duplicate-skips.txt` by hand with `deny.toml`):
- a lowered number is always welcome;
- a raised one fails `ratchet.sh baselines` unless the pull request carries a line of its own reading `Ruling: <where it was ruled>`, naming something after the colon (line ends are read with any `\r` dropped);
- a baseline the check cannot read fails, and no ruling passes it: a file under `quality/` that the table does not name, a listing whose head reads zero entries where the base had some, a line that is not an entry, a file listed under the section its path does not belong to, a CRAP entry without its file, function, line and complexity, a clone count that is not a number, a public-API snapshot that lists no public item, or a baseline emptied on either side.

"Raised" means any of these:
- a function over CC 15 that is new or grew;
- a file over its ceiling that is new or grew, its ceiling following its path;
- a function over clippy's 100 lines (`too-many-lines.txt`) that is new or grew;
- a clone fingerprint not in the earlier baseline;
- more public items in any crate's snapshot, or a snapshot for a new crate;
- more items in brokkr-core's public API naming `serde_json::Value`, which decision 0071 ruling 3 ratchets;
- a lint count that rose in `suppressions.txt`, or a new skip in `duplicate-skips.txt` (#337);
- a removed baseline;
- any change to `ceilings.json`, `lib.sh`, `ratchet.sh` or `measure.sh`, since changing how a number is measured changes the number.

The check reads the pull request body from the event, so an edited body needs a new event (close and reopen), as `delivered by brokkr` does.

**The public API's `Value` count.** It is the number of lines in `public-api/brokkr-core.txt` naming `serde_json::value::Value`: 26 at `1d890c6a`. Each is one public item whose signature carries the untyped value. #338's issue said 11, a count of signatures by hand; this is the count the tool reproduces.

## Refresh

```sh
TMPDIR=/var/tmp/<dir> scripts/coverage-exact.sh   # writes target/coverage/lcov.info
quality/measure.sh
git diff quality/
```

Run it on a clean Linux checkout. The measuring host is Linux:
- clippy and cargo-crap read only the code the host compiles, and production carries `cfg(not(target_os = "linux"))` code;
- `too-many-lines.txt` sorts with GNU `sort -V`.

Two runs over the same tree are byte-identical. The diff is the review.

Keep `TMPDIR` off `/home`: the hands tests assert that `/home` is empty inside their box.

## Tools

| Tool | Version | Role |
|---|---|---|
| cargo-crap | 0.5.0 | complexity and CRAP |
| jscpd | 5.3.2, the Rust rewrite. CI installs the release binary by digest (`.github/actions/setup-jscpd`); `cargo install --locked jscpd --version 5.3.2` builds the same | duplication |
| cargo-public-api | 0.52.0, run on the pinned nightly | public-API snapshots |
| cargo-shear | 1.14.0 | unused dependencies |
| clippy | 1.98.0 stable | function length |
| cargo-llvm-cov | 0.9.1 | the LCOV, through the exact gate |
| Rust nightly | `rust-nightly-version.txt` | the LCOV and the API snapshots |

## Definitions

- **Production or test.** A test file is one the exact coverage gate treats as a test: under a `tests/`, `examples/` or `benches/` directory, or named `tests.rs`, `*_tests.rs` or `*-tests.rs`. `lib.sh` reads that vocabulary from `scripts/coverage-exact.sh` (`test_dirs`, `test_files`), its one home, and refuses to run if it cannot. Everything else in `crates/` is production.
- **Paths and order.** Paths are repository-relative. cargo-crap writes them with a leading `./`, which the ratchet, running the same command, matches. Every script runs under `LC_ALL=C`:
  - `file-lines.txt` and the snapshots sort by byte order;
  - `too-many-lines.txt` sorts by `path:line` in version order (`sort -V`), so `:9` comes before `:10`.
- **CRAP.** CRAP = CC² × (1 − coverage)³ + CC. The gate holds production at 100% coverage, so CRAP equals cyclomatic complexity here, and the ratchet refuses any measurement below 100%. cargo-crap counts `if`, loops, match arms, `&&`, `||` and `?`.
- **Function lines.** Function lines are clippy's count: code lines only, with comments and blank lines excluded.

## Traps found while measuring

- **jscpd's `--max-lines` caps whole files, not clone blocks.** jscpd 5.3.2 sets no cap by default (`--debug` reports `max_lines: null`). Any explicit value, though, silently skips every file longer than it: `-x 1000` would read 19 of the 57 production files less. `lib.sh` pins `--max-lines 100000`, so a future default cannot do the same.
- **cargo-crap in `--workspace` mode records absolute paths.** It also walks test directories nested under `src/`, and scores them at 0% coverage because they are not in the LCOV. `lib.sh` runs `--path .` with every test path excluded.
- **`too_many_lines` is a pedantic lint that an attribute could silence.** `measure.sh` force-warns it, which no attribute can silence, so the list is complete.
- **jscpd honours `.gitignore`.** The data scan also names `target/` and `.forge/` explicitly, and the scripts run on a clean checkout, so an untracked file cannot enter a baseline.

## The numbers at a glance

These are one-off measurements, not held by any file here:
- the per-function rows came from a clippy pass with every threshold at 0 on `addd33ca`, posted on #335;
- the duplicated-lines and share columns came from jscpd's report at `1d890c6a`, where the baselines were regenerated.

The clone counts come from the same report. The baselines hold distinct fingerprints, which count repeated clones once.

| Measure | Count | p50 | p90 | p95 | p99 | Max |
|---|---|---|---|---|---|---|
| CC per production function | 1,615 | 3 | 11 | 16 | 35 | 195 |
| Lines per production function | 1,656 | 8 | 47 | 72 | 163 | 751 |
| Lines per test function | 3,808 | 18 | 66 | 91 | 179 | 729 |
| Parameters per production function | 1,573 | 2 | 4 | 5 | 8 | 11 |

| Duplication (jscpd, 50 tokens and 5 lines minimum) | Clones | Duplicated lines | Share |
|---|---|---|---|
| Production Rust | 76 | 753 | 1.22% |
| Test Rust | 988 | 10,117 | 6.07% |
| JSON and Markdown | 183 | 4,778 | 5.94% |

#335 carries the 20 worst offenders of each measure, and the ceilings with their evidence.

## Budgets (#342)

The baselines above measure code shape. These four files hold what Brokkr costs to run, and a test or a CI step refuses any number past them.

| File | What it holds | Held by |
|---|---|---|
| `prompt-bytes.json` | Bytes of the prompt each model site in every shipped recipe and bundle is handed | `crates/brokkr-runtime/tests/budgets.rs`, which also prints each site's o200k tokens, a report only |
| `crate-count.json` | `Cargo.lock`'s `[[package]]` tables | the same test |
| `heap-bytes.json` | Peak heap of projecting the largest transcript the reader admits, per kind, measured under dhat, with a tenth of headroom | `crates/brokkr-cli/tests/heap_*.rs` |
| `binary-size.json` | Bytes of CI's Linux release binary | `scripts/binary-size.sh` in the `release-binary` job, within one per cent either way |

Each file holds its ceilings in one `budgets` object of whole numbers. `ratchet.sh` reads the four as its table's `budgets` listing: a ceiling that rose, or a new one, is raised and needs a `Ruling:` line; a lowered or dropped one is a shrink. The gates above hold the tree to the numbers; the ratchet holds the numbers to their history.

`scripts/measure-budgets.sh` rewrites the first three from the tests' own reports; the tests print what they measure even when it is over budget. The binary size is read from CI's artifact, never a local build, whose embedded paths differ. The CPU budgets have no file: the `cpu-budgets` job counts instructions under Callgrind for the pull request and for its base in one run, and fails a rise over 2%.

```sh
scripts/measure-budgets.sh
git diff quality/
```

A budget that rises is named in the pull request that raises it.
