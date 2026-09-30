# Contributing to Brokkr by hand

This repository's engine forges its own changes and reviews them
adversarially. The bar for a human contribution is the bar the machine
is already held to — twelve required checks, none of them a percentage you
can nudge. This document is the whole walk from `git clone` to a green
pull request, with every command written out.

Nothing here is lowered for a first contribution. What this document
does instead is make the bar reachable: every gate stated with the exact
command that reproduces it locally, every refusal shape named with its
fix, and a way to have the machine review your branch before a human
ever looks at it.

- [What you need installed](#what-you-need-installed)
- [Fork, clone, branch](#fork-clone-branch)
- [The twelve checks](#the-twelve-checks)
- [The pre-flight: let the machine review you first](#the-pre-flight-let-the-machine-review-you-first)
- [The coverage gate, practically](#the-coverage-gate-practically)
- [Commits, signing, and how your PR actually lands](#commits-signing-and-how-your-pr-actually-lands)
- [The decision culture](#the-decision-culture)
- [What is frozen](#what-is-frozen)
- [Recipes and adapters: data the Rust suite witnesses](#recipes-and-adapters-data-the-rust-suite-witnesses)
- [Contribution licensing](#contribution-licensing)

## What you need installed

The engine is Rust-only (decision
[0009](../decisions/0009-rust-only.md)): no Python, no Node, no
toolchain beyond cargo for the ordinary path. The MSRV, the coverage
gate, the licence gate, the ratchets, the mutants gate and the non-Rust
lints need something beyond a stable toolchain; each tool is pinned in
the workflow that runs it, in a `.github/actions/setup-*` action it
calls, or in a version file at the root, and the version below is that
pin.

| Tool | Needed by | Check it is there |
|---|---|---|
| The pinned stable Rust, with clippy and rustfmt | format, clippy, tests, the bundle compiles, the release build | `cargo --version` |
| Rust 1.88.0 | the MSRV check | `cargo +1.88.0 --version` |
| The pinned nightly with `llvm-tools-preview` | the coverage gate | `cargo +$(cat rust-nightly-version.txt) --version` |
| `cargo-llvm-cov` at the pinned version | the coverage gate | `cargo llvm-cov --version` |
| `jq` | the coverage gate, the ratchets, the binary size budget and the mutants gate (their scripts fail without it) | `jq --version` |
| `cargo-deny` | the licence gate | `cargo deny --version` |
| cargo-crap 0.5.0 and cargo-public-api 0.52.0 | the complexity and public-API ratchets, inside the coverage job | `cargo crap --version`, `cargo public-api --version` |
| jscpd 5.3.2 and cargo-shear 1.14.0 | the baseline ratchets | `jscpd --version`, `cargo shear --version` |
| cargo-mutants 27.1.0 | the brokkr-core mutants gate | `cargo mutants --version` |
| typos 1.50.2, shellcheck 0.11.0, zizmor 1.30.1, actionlint and lychee | the non-Rust lints (the last two at the digests in `.github/actions/setup-*`) | each tool's `--version` |
| Node 22.23.3, with `npm ci --prefix .github/lint --ignore-scripts --no-audit --no-fund` | the non-Rust lints' diagram render (`scripts/lint-diagrams.sh`); only the lint needs it, never the engine | `node --version` |

The extra toolchains and tools install the usual way — `rustup toolchain
install 1.88.0`, `rustup toolchain install "$(cat rust-nightly-version.txt)" --component
llvm-tools-preview`, `cargo install cargo-llvm-cov --version "$(cat
cargo-llvm-cov-version.txt)"`, `cargo install cargo-deny`, and `jq` from
your package manager. `rust-toolchain.toml` names the stable release CI
judges by, so inside this tree plain `cargo` is CI's compiler and CI's
Clippy (`rustup toolchain install`, run inside the tree, installs it), and the `+1.88.0` and
dated-nightly prefixes are how the other two get selected.

You do **not** need `cargo-audit`; the RustSec check runs only in CI.
Installing it locally is a convenience, not a requirement — see
[the checks you cannot fully reproduce](#the-checks-you-cannot-fully-reproduce).

## Fork, clone, branch

```
gh repo fork feedback-loop-ai/brokkr --clone
cd ./brokkr
git switch -c <your-branch>
```

Branch naming: the machine's own slices use `slice-<short-name>`, which
is why the history is full of them. A fork's branch name is yours; use
something that says what the branch does. `main` is not a place to
work — the repository's own flow branches for every slice and hands the
branch back.

Two habits from the house flow that transfer directly:

- **One slice per branch.** The engine works in a git worktree per
  slice so the main checkout stays clean and parallel work never shares
  a dirty tree. A fork with one branch per change gets the same
  property for free.
- **Tests are part of the change, not an afterthought.** Extend the
  suite that proves the code you touched, in the same commit. A branch
  that adds behaviour and no test fails the coverage gate anyway (see
  below), so this is not a style preference.

## The twelve checks

All twelve are required status checks on `main`. Eleven come from ten
jobs in [`../../.github/workflows/ci.yml`](../../.github/workflows/ci.yml) (`engine`
runs once per operating system), and one is in
[`../../.github/workflows/mutants.yml`](../../.github/workflows/mutants.yml). They run on
every pull request, and again in [the merge queue](#the-merge-queue) on
the commit that lands. This is the full list, in the workflows' own order;
each local command is the job's own, and the sections below explain them:

| # | CI check | Job | Local command |
|---|---|---|---|
| 1 | `delivered by brokkr` | `delivered-by-brokkr` | — (a shipped run at your head; see [the landing](#the-landing-let-the-machine-finish-what-you-wrote-by-hand)) |
| 2 | `MSRV (1.88)` | `msrv` | [`cargo +1.88.0 check --workspace --all-targets --all-features --locked`](#the-msrv) |
| 3 | `format, clippy, contracts` | `quality` | [`cargo fmt --all -- --check`](#formatting), [`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`](#clippy-warnings-as-errors), [the suppression check](#an-added-suppression-names-a-ruling), then both [bundle compiles](#the-bundles-compile) |
| 4 | `test (ubuntu-latest)` | `engine` | [`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 cargo test --workspace --all-features --locked --no-fail-fast`](#the-workspace-suite) with bubblewrap installed, then both [bundle compiles](#the-bundles-compile) |
| 5 | `test (macos-latest)` | `engine` | on a Mac, `cargo test --workspace --all-features --locked --no-fail-fast`, then [the startup gate](#the-macos-startup-gate), then both [bundle compiles](#the-bundles-compile) |
| 6 | `exact coverage gate` | `coverage` | [`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh`](#exact-coverage), then `quality/ratchet.sh crap` and `quality/ratchet.sh api` |
| 7 | `dependency licenses (cargo-deny)` | `license-compliance` | [`cargo deny check licenses bans sources`](#dependency-licences) |
| 8 | `non-Rust lints` | `lint-non-rust` | [`bash scripts/lint-non-rust.sh`](#the-non-rust-lints), then [the diagram render and Renovate's validator](#the-non-rust-lints) |
| 9 | `baseline ratchets` | `ratchets` | `quality/ratchet.sh files`, `quality/ratchet.sh clones`, `cargo shear --deny-warnings --locked`, `PR_BODY="<your pull request's body>" quality/ratchet.sh baselines origin/main` |
| 10 | `RustSec dependency audit` | `dependency-audit` | — (CI-only; see below) |
| 11 | `release binary artifact` | `release-binary` | [`cargo build --release --locked -p brokkr-cli`](#the-release-binary), then `bash scripts/binary-size.sh target/release/brokkr quality/binary-size.json`; the size budget holds only for CI's build, whose embedded paths yours do not share |
| 12 | `mutants in the diff: brokkr-core` | `core-gate` (mutants.yml) | `bash scripts/mutants.sh gate origin/main brokkr-core` |

The four checks the list above adds to the older eight:

- **`delivered by brokkr`** (decision
  [0033](../decisions/0033-contributing-through-brokkr.md), tiers from decision
  [0038](../decisions/0038-evidence-follows-content.md)) passes when the `Brokkr-Run:` line in your
  pull request names a run that shipped your head. The operator's `by-hand` label
  is the visible exception.
- **`non-Rust lints`** (#339) reads the workflows, shell scripts,
  spelling, Markdown links and their anchors, the Mermaid diagrams and
  Renovate's configuration. The offline part is one list,
  `scripts/lint-non-rust.sh` (#427): typos, shellcheck, actionlint,
  zizmor and lychee, each checked against its pin: typos, shellcheck
  and zizmor in `ci.yml`, actionlint and lychee in
  `.github/actions/setup-actionlint` and `.github/actions/setup-lychee`. A
  landing's verify seat runs the same list with `--seat`, which names a
  lint whose tool the seat cannot reach as not run and runs the others; that
  lint's only judge is then this check. The job then renders the
  diagrams and validates Renovate's configuration; both are written out
  in [the non-Rust lints](#the-non-rust-lints) below.
- **`baseline ratchets`** (#338) holds file size and duplication to
  `quality/`'s committed baselines, refuses an unused dependency, and
  refuses any raised baseline unless the pull request body carries a
  `Ruling:` line. CI reads that body from the pull request and judges
  against the merge commit's first parent; locally, `origin/main` is the
  base and `PR_BODY` carries the body, so a raised baseline without its
  `Ruling:` line is refused here as it is there. `quality/README.md`
  explains each baseline and how to refresh it. The complexity and
  public-API ratchets run inside the coverage job, because they read its
  LCOV and its pinned nightly.
- **`mutants in the diff: brokkr-core`** (#289) fails a pull request that
  adds a mutant no test catches to brokkr-core. Reproduce it with the
  command in row 12 (cargo-mutants 27.1.0). A miss that shares a
  committed miss's file and mutation, in
  `quality/mutants/brokkr-core.missed.txt`, is accounted for once; any
  other miss fails, and the fix is a test that catches it.

The sections below are in a different order on purpose: run them from
the repository root in the order written, cheapest refusal first, so a
misformatted file costs you seconds rather than a full instrumented
rebuild.

### Formatting

```
cargo fmt --all -- --check
```

Prints nothing and exits 0 when the tree is canonical. On a
non-canonical file it prints a unified diff of what `rustfmt` would
change and exits 1. The fix is `cargo fmt --all` — never hand-editing
to match the diff.

### Clippy, warnings as errors

```
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Lint configuration lives once, in `[workspace.lints]` in the root
`Cargo.toml`, and every crate inherits it through `[lints] workspace =
true` — so no crate can quietly hold a different opinion. The
warnings-as-errors escalation is the `-D warnings` on that line: run it
exactly and your Clippy is CI's Clippy. `--all-targets` matters, because
a lint that only fires in a test target is still a red check.

An `#[allow(...)]` to silence a lint is a change with a reason, and the
reason belongs in a comment beside it. A blanket crate-level allow will
be a review finding.

### An added suppression names a ruling

```
BROKKR_SUPPRESSION_BASE=origin/main cargo test --locked -p brokkr-cli --test suppressions -- --ignored --exact an_added_suppression_of_a_ratcheted_lint_names_a_ruling
```

The `quality` job runs this after Clippy, on pull requests only
(decision 0071 ruling 4, #337). Its base is your pull request's base commit; `origin/main`
stands in for it locally. A suppression of a ratcheted lint that your
branch adds to production code must name a ruling in its `reason`, and
the refusal lists each one that does not.

### The workspace suite

```
BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 cargo test --workspace --all-features --locked --no-fail-fast
```

The full suite, untruncated. On Linux, install bubblewrap first: the
hands tests build a real namespace (decision 0043), and a boundary proof
that cannot open one skips, which a Rust test reports as `ok`.
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1` turns that skip into a failure, as
CI's Linux leg does (decision 0054 ruling 9); on macOS, where there is no
namespace to open, leave it unset. On this tree it builds and runs 45 test
binaries; the last run before this document was written reported
`test result: ok` for every one of them, 787 tests passing and none
failing. Read the whole output rather than the last line: a suite can be
green overall while a binary you expected to gain a test gained none.

`--all-features` and `--locked` are not decoration. `--locked` refuses
to update `Cargo.lock`, so your run uses the dependency graph CI will
use; if it errors about the lockfile being out of date, your `Cargo.toml`
change needs its lockfile update committed too.

### The macOS startup gate

```
set -euo pipefail
cargo test --locked -p brokkr-seatbelt-probe --test seatbelt_lifetime_probe -- --ignored --exact --nocapture seatbelt_probe::native::native_startup_feasibility_probe 2>&1 | tee target/r3-startup.log
grep -Eq 'test result: ok\. 1 passed' target/r3-startup.log
grep -q 'Gate A startup verdict' target/r3-startup-report.txt
```

Paste it into a shell of its own (start `bash` first): its first line ends
that shell at the first failure, as it ends CI's step. Without it, `tee`
masks a failed probe, and the quiet `grep`s fail silently while the last
one can pass against a report an earlier run left.

The macOS leg of the `engine` job runs one step the Linux leg does not:
Seatbelt's Gate A, the startup matrix of decision
[0046](../decisions/0046-the-boundary-is-named.md) slice II,
against the real `/usr/bin/sandbox-exec` and your per-user `launchctl`. The
probe is `#[ignore]`d, so the workspace suite above never runs it; CI selects
it by name after the suite, and it is a hard gate. It passes only when the
log reports exactly one test passed and the probe wrote its verdict to
`target/r3-startup-report.txt`: a name that selects nothing still prints
`test result: ok`, with none passed. Run it on a Mac, outside any sandbox,
where a missing `sandbox-exec` or an outer box is a named failure. On Linux
the probe returns before it runs a cell or writes a report, so it proves
nothing there. CI prints `sw_vers` and `uname -m` first, and a step of its
own, `R3 native diagnostics`, uploads an artifact whether the gate passes
or not.

### The MSRV

```
cargo +1.88.0 check --workspace --all-targets --all-features --locked
```

The README's badge says 1.88+, and this check is what makes that a fact
rather than prose. CI installs 1.88.0 and runs this same command, across
all targets and features. The explicit `+1.88.0` is needed
in both places because `rust-toolchain.toml` selects the newer stable pin,
which would not notice.

A refusal here is almost always a language or standard-library feature
newer than 1.88, or a dependency bump that raised its own MSRV. The fix
is to use the older form, or — if the newer floor is genuinely
necessary — to say so in the pull request and let the operator rule on
moving the badge, the CI pin and the README together. Do not raise the
floor silently.

### The bundles compile

```
cargo run --locked -p brokkr-cli -- compile --bundle bundles/self
cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify
```

Each prints the resolved manifest and its content digest, and exits 0.
This is the constitutional lint: compilation is where the structural
laws are enforced, so a bundle that compiles is one whose review gate is
unavoidable, whose aggregates match their declared results, whose
conditions are all in the closed vocabulary, and whose composition
markers all describe something real.

If you touched a recipe, a bundle, a role charter or an adapter, compile
that one too:

```
cargo run --locked -p brokkr-cli -- compile --bundle recipes/<name>
```

and expect a witness digest to move — see
[recipes and adapters](#recipes-and-adapters-data-the-rust-suite-witnesses).

### Exact coverage

```
BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh
```

Literal 100% of lines, branches and functions across the workspace, or
refusal. There is no threshold to lower. The script does not set
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` itself; CI does, because the gate
counts regions a skipped boundary proof never enters, so set it as the
suite above does. This one has its own section:
[the coverage gate, practically](#the-coverage-gate-practically).

One operational note before you run it. The script builds a second,
instrumented copy of the workspace, and a checked copy of its production
targets, and keeps both warm between runs in a directory it owns:

```
${BROKKR_COVERAGE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}}/brokkr-coverage-cache
```

The build directory is keyed by the pinned nightly, the lockfile and the
workspace's target set, so the dependency build is reused until one of
them changes. When a new key is made, the script prunes old keys: only
directories inside `brokkr-coverage-cache` whose names have the key's exact
shape, so nothing else under the root you give it is ever touched. Every
workspace artifact and profile is still dropped before the run
(`cargo llvm-cov clean --workspace`), so the report is always this
checkout's. Only one run holds the cache at a time: a second run on the
same host says it is waiting, waits up to an hour for the first, and then
refuses, because two runs sharing a target would merge each other's
profiles. The instrumented copy takes several
gigabytes, so point `BROKKR_COVERAGE_CACHE` at a disk-backed directory if
your cache home is small or RAM-backed.

The tests themselves still create ordinary temporary directories under
`TMPDIR`, and so does the script's report scratch. On a machine where
`/tmp` is a small tmpfs, point `TMPDIR` at a disk-backed directory outside
any Git worktree:

```
TMPDIR=/var/tmp BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh
```

The directory must be outside a Git worktree: tests that create ordinary
temporary directories expect Git discovery to find no parent repository.

### Dependency licences

```
cargo deny check licenses bans sources
```

Prints `bans ok, licenses ok, sources ok` and exits 0 when all three
hold. `sources` refuses a crate from an unknown registry or a git
repository. `bans` refuses a second version of any crate beyond the
listed skips, and any dependency edge that decision 0071 ruling 1's
one-way crate graph does not allow; both are recorded in `deny.toml`
with their reasons. `licenses` passes when every crate in `Cargo.lock`
carries a licence on the allowlist in
[`deny.toml`](../../deny.toml): MIT, Apache-2.0 (including the
LLVM-exception form), BSD-2-Clause, BSD-3-Clause, ISC, Zlib,
Unicode-3.0 and CDLA-Permissive-2.0. Permissive only, in the spirit of
decision [0018](../decisions/0018-dual-license.md) — openness that
imposes nothing.

A refusal names the crate, the licence it carries and the fact that the
allowlist does not contain it. The fix, in order of preference: drop the
dependency, replace it with a permissively licensed one, or — if the
dependency is genuinely necessary and the licence genuinely permissive —
propose the allowlist entry in your pull request and let the operator
rule on it. Adding a licence to `deny.toml` is a change to the
repository's licensing posture, so it is the operator's call, not a
tidy-up. Suppressing the check is never the fix.

CI runs this through `EmbarkStudios/cargo-deny-action` v2.1.1, pinned by
commit, whose image carries cargo-deny 0.20.2; a local binary of another
version can disagree with it, so install that one to match. `cargo deny check
licenses bans sources` locally is the same check reading the same `deny.toml`.

### The non-Rust lints

```
bash scripts/lint-non-rust.sh
```

The offline list (#427), cheapest first: typos, shellcheck on every
script and composite action, actionlint, zizmor and lychee. Each tool's
version is checked against its pin before it runs, and a tool that is
missing or off its pin is refused by name.

The job then renders every Mermaid diagram, which needs Node 22.23.3
and the Chrome that `.github/lint/puppeteer.json` names:

```
npm ci --prefix .github/lint --ignore-scripts --no-audit --no-fund
bash scripts/lint-diagrams.sh
```

and validates Renovate's configuration with the image pinned in
`.github/renovate-image.txt`, which needs docker:

```
image="$(tr -d '[:space:]' < .github/renovate-image.txt)"
docker run --rm -v "$PWD":/usr/src/app -w /usr/src/app --entrypoint renovate-config-validator "$image" --strict --no-global .github/renovate.json5
docker run --rm -v "$PWD":/usr/src/app -w /usr/src/app --entrypoint renovate-config-validator "$image" --strict .github/renovate-global.json5
```

### The RustSec advisory audit

CI runs `rustsec/audit-check` v2.0.0, pinned by commit, which fetches the RustSec advisory
database and scans `Cargo.lock`. **It has no exact local
equivalent** — the Action reports through the GitHub Checks API
and reads its own copy of the database. The closest local approximation
is `cargo install cargo-audit` and then:

```
cargo audit
```

which loads the advisory database into `~/.cargo/advisory-db` and scans
the same lockfile. A clean run reports the number of crate dependencies
scanned and finds nothing; a hit prints the advisory id, the affected
crate and version, and the versions that fix it.

The fix for an advisory is to move off the affected version: bump the
dependency, or bump whatever pulls it in transitively, and commit the
`Cargo.lock` change. An advisory that has no fixed version yet is a
conversation for the pull request, not something to silence. There is no
ignore list in this repository and adding one would be a decision.

### The release binary

```
cargo build --release --locked -p brokkr-cli
bash scripts/binary-size.sh target/release/brokkr quality/binary-size.json
```

Builds `target/release/brokkr`, the only binary this repository ships.
It rarely fails on its own once the suite is green — the release profile
compiles the same code. What shows up here and nowhere else is anything
conditioned on the debug profile: an item behind
`#[cfg(debug_assertions)]` that non-debug code depends on, or a `cargo
test`-only path that hid a warning.

The second line is the size budget (#342): the binary must not pass
`quality/binary-size.json`'s ceiling. Run it to see the number, but read
it as a hint: the ceiling holds CI's build, whose embedded paths yours do
not share.

### The checks you cannot fully reproduce

Be honest with yourself about these, and say so in the pull request
if you think they are at risk:

- **`delivered by brokkr`** has no local command: it reads a run that
  shipped your head, or the operator's `by-hand` label. See
  [the landing](#the-landing-let-the-machine-finish-what-you-wrote-by-hand).
- **The two-OS matrix.** Checks 4–5 run the same suite on Ubuntu and
  macOS, and the macOS leg adds [the startup gate](#the-macos-startup-gate),
  which only a Mac can run. You ran one leg. Anything touching the filesystem or the
  platform's process lookup is where this bites. Windows is not a host
  (decision [0063](../decisions/0063-windows-is-not-a-host.md)); WSL2 is Linux. Note that
  [`.gitattributes`](../../.gitattributes) normalises every text file to LF in
  the working tree on every platform, precisely because bundle digests
  are taken over file bytes — so do not "fix" a line ending.
- **The RustSec audit**, as above.
- **The binary size budget**, as above.
- **The diagram render and Renovate's validator.** `.github/lint/puppeteer.json`
  names `/usr/bin/google-chrome`, a Linux path, so
  `scripts/lint-diagrams.sh` does not run as written on a Mac. The
  validator needs docker. See [the non-Rust lints](#the-non-rust-lints).

## The pre-flight: let the machine review you first

Before you open the pull request, have the machine review the branch.
`recipes/preflight` seats Brokkr's own `verify` and `review` agents —
the same two that judge the machine's own work — and points them at an
unmerged branch:

```
brokkr run --recipe preflight --repo . --feature "<what the branch does, and its base if not main>"
```

You need the binary first — `cargo install --path crates/brokkr-cli`
puts `brokkr` on your path, or run it out of the tree with `cargo run
--locked -p brokkr-cli -- run --recipe preflight …`. Either way, run it
from the repository root: `--recipe <name>` resolves to
`<recipes-dir>/<name>`, and `--recipes-dir` defaults to the relative
`recipes`.

The recipe has two phases and stops:

```
verify  →  review  →  done / stop
```

There is no intake to reframe your work, no implement to change your
branch, and no ship to merge it. The review seat is a gate chartered not
to write, yet it runs unboxed under `acceptEdits` with `Bash(git:*)`
pre-approved, and the engine parks it only when it moves HEAD: an edit
it leaves uncommitted is not caught. The policy table
(`recipes/preflight/policy.json`) ends after `review` with a terminal
ruling, and
[`crates/brokkr-runtime/tests/preflight_shape.rs`](../../crates/brokkr-runtime/tests/preflight_shape.rs)
asserts that shape so it cannot quietly grow a working phase later.

What you get back is the same thing the machine's own slices get: typed
results, journalled, with findings named and ranked on a closed severity
vocabulary.

| Ruling | What it means |
|---|---|
| `verify` → `fail` → `stop` | A gate is red. The notes quote the failing lines. Fix and run again. |
| `review` → `clean` → `done` | Nothing remains. Open the pull request. |
| `review` → `residual` → `done` (flagged) | Findings at or below medium, none of them security. Open the pull request and name them in it. |
| `review` → `residual` / `security-hold` → `stop` | Above medium, or any security finding. Not ready. |

Read [`recipes/preflight/README.md`](../../recipes/preflight/README.md) for
what each seat runs and how it differs from `bundles/verify` (which
judges an already-merged change and is the operator's tool, not yours).

This is a recommendation, not a gate: nobody's pull request is rejected
for skipping it. Its value is that the obvious findings are yours to fix
before a human spends their attention on them.

## The landing: let the machine finish what you wrote by hand

Everything above is what a hand-authored branch has to satisfy. Since
decision 0051 you do not have to run it yourself to propose the branch:
light `brokkr run --recipe landing --repo . --feature "landing: <what
the branch is>"` on it. A gate of seconds reads the branch's class
against `.github/delivery-classes.json`; prose goes straight to the
review seat, code goes through the verifier first — `cargo fmt --all
-- --check`, `bash scripts/lint-non-rust.sh --seat`, `cargo clippy
--workspace --all-targets --all-features --locked -- -D warnings`,
`cargo test --workspace` and `cargo run -p brokkr-cli -- compile
--bundle bundles/self`, in that order
(`recipes/fast/scripts/verify-seat.sh`, #427), in the box with no
network because this repository's realm declares no boundary and so
reads `namespace`; under `harness` the same script would run unboxed.
The rest of the twelve
stay CI's to prove: the suite in CI's own form
(`BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`, `--all-features`, `--locked` and
`--no-fail-fast`; inside the `namespace` box a boundary proof cannot
open a namespace, and it skips), any non-Rust lint whose tool the seat
cannot reach (the seat names it `not run`), the MSRV, the suppression check and the
`bundles/verify` compile, the other OS, the exact coverage gate,
cargo-deny, the diagram render and Renovate's validator, the ratchets,
the RustSec audit, the release build and its size budget, and the
mutants gate. A failure or a finding
above low comes back to an
implement seat commissioned by that finding, twice at most. A clean
judgment ships: the anchor carries the branch's patch map, and the
contribution gate vouches for your pull request at tier `vouched`
without the operator's `by-hand` label. Name the run as `Brokkr-Run:` in
the pull request. `preflight` remains the way to ask without landing.

## The coverage gate, practically

`scripts/coverage-exact.sh` demands literal integer equality on lines,
branches and functions for the whole workspace. Not a percentage, not a
trend, not a diff-scoped number: `covered == count`, three times, or the
script prints the summary to stderr and exits 1 with

```
coverage refusal: literal nonzero 100% source-line/branch/function equality not met
```

### Why this is easier than it sounds

The baseline is already 100%. Every line, branch and function in
`crates/` is covered today, which means the only way your pull request
can turn this check red is a line **your own diff introduced** that no
test reaches. That is the whole differential trick: you never have to
hunt through the workspace for uncovered code, because there isn't any.
Whatever the report shows as missing, you wrote it in this branch.

The script writes the evidence before it judges, so a red gate still
leaves you everything you need:

| File | What it holds |
|---|---|
| `target/coverage/coverage-summary.json` | The three counted pairs. This is what the gate compares. |
| `target/coverage/lcov.info` | The canonical LCOV records the gate folds — grep it for the misses. |
| `target/coverage/coverage-exact.json` | The full LLVM JSON report. |

CI uploads that directory as the `coverage-exact` artifact on every run,
pass or fail.

One thing about that JSON that will confuse you if nobody says it: the
percentages LLVM prints in `coverage-exact.json` are **not** the numbers
the gate reads, and they are not 100. LLVM's JSON summary counts each
compiler instantiation of a generic or inlined function as its own line,
so a fully covered workspace reads well under 100% there. The ratified
contract is *source* coverage, so the script folds the LCOV records
instead — one `DA` record per source line, one `BRDA` per branch, and
functions deduplicated by file and start line. On this tree at the time
of writing the LCOV fold counts 13381 of 13381 lines and 2022 of 2022
branches covered, while the same report's LLVM JSON percentages read
97.8% and 96.7%. Read `coverage-summary.json`, not the JSON percentages.

### The four refusal shapes

**A new `if` or `match` arm no test reaches.** The most common one, and
usually an error arm. The fix is the test case that takes the arm — not
deleting the arm. Find it by searching `lcov.info` for the `BRDA` record
whose taken count is `0` or `-` under your file's `SF:` heading.

**A reader error arm reached only by a filesystem failure or race.** When
the arm is in a transcript reader, prefer the routes in the order the change
`prove-transcript-reader-faults` fixes: an ordinary input first; then a
deterministic real filesystem change; and only then a scripted error, because
no portable real fault exists for enumeration, identity and bounded read. A
real change that must fall between two reader operations is timed through
`brokkr_cli::ui::safe_fs::fault`, the unit-test-only reader fault seam, and a
scripted error fills the gaps the real routes cannot. Every entry a plan
installs must fire, or its test fails. The seam exists only in the
`brokkr-cli` unit-test build, so no release binary, package or integration-test
build compiles it. The `fault` module is not a test module: it is counted code
inside the production file `crates/brokkr-cli/src/ui/safe_fs.rs`, so the
test-module placement rule below neither applies to it nor exempts it from the
counter. Its own tests live in `ui/tests.rs`, the established harness.

**A new function nothing calls.** Same shape, one level up, with one
wrinkle: the fold identifies a source function by file and start line
and sums the hits of every compiled instantiation of it, so a single
`FNDA:0,` against a mangled name is not the miss. The miss is a `FN:`
start line under your file's `SF:` heading whose every `FNDA` record
reads `0`. Either a test calls it, or it should not exist yet. This
repository does not carry code ahead of its use.

**Test-harness source in the production report.** The script has one
test-path vocabulary, and hands `cargo-llvm-cov` exactly that, with the
tool's own default switched off: a file under a `tests/`, `examples/` or
`benches/` directory, or named `tests.rs`, `*_tests.rs` or `*-tests.rs`.
Such a file leaves the report. After the run, every file the report counts
must be a production source under `crates/`, or the script names it and
refuses:

```
coverage refusal: the report counts a file that is not a production source of this workspace
```

Test modules in this workspace
therefore sit in a sibling file, declared from the production file as
`#[cfg(test)] mod tests;` or `#[cfg(test)] mod foo_tests;`, or under
`crates/<crate>/tests/` — never as an inline `#[cfg(test)] mod tests {
… }` block with the bodies in the production file. Follow whichever of
the two the crate you are editing already uses; every crate here uses
one of them.

**A `coverage(off)` attribute or a `cfg(coverage)` switch.** Forbidden
outright, in every spelling: `#[coverage(off)]`, and any `cfg`, `cfg_attr`
or `cfg!` predicate that names `coverage` or `coverage_nightly`, however it
is spaced, nested, split across lines or broken up by comments. The script
reads every Rust file under each member, through symbolic links, as tokens
with comments and literals blanked, and again every source the production
check's dep-info names, whatever its file name. It names each file and line
it finds, and refuses:

```
coverage refusal: attribute and cfg(coverage) source exclusions are forbidden
```

The same scan refuses any lint level on `unexpected_cfgs`, and any allow of
every warning. Those would silence the compiler's refusal of an undeclared
cfg: the script checks every production target with that lint forbidden,
so a cfg that names `coverage` fails there however a macro built it. A
manifest may not declare the cfg (`check-cfg`, or a key spelled through a
TOML escape), and the workspace has no build script that could:

```
coverage refusal: a production target does not compile with every undeclared cfg forbidden
coverage refusal: a manifest declares a cfg, changes the unexpected_cfgs lint or sets rustflags
```

**A member, a dependency or a config the scans cannot see.** Every scan
reads the workspace's shape from `cargo metadata`, so the shape itself is
fixed: each member's manifest is `crates/<name>/Cargo.toml`, a path
dependency that is not a member may not be compiled at all, and the
repository carries no `.cargo/config` or `.cargo/config.toml`, which could
set rustflags or a compiler wrapper for the measured build. And after the
run, every member with library or binary code must have at least one file
in the report:

```
coverage refusal: a workspace member sits outside crates/<name>/
coverage refusal: the product compiles local code that no member measures
coverage refusal: a repository cargo config could set rustflags or a compiler wrapper for the measured build
a counted member contributes no file to the report: <name>
coverage refusal: a member with production code is missing from the report
```

**A dep-info file the gate cannot read whole.** The dep-info is read
strictly: every line must be blank, a comment, a rule for one of rustc's
outputs, or one source path, and the rule must name as many sources as the
file lists. A path with a newline in it breaks that and is refused, never
skipped:

```
coverage refusal: a dep-info file the gate cannot read whole
```

**A build or a profile from another run.** The cache is shared by every
checkout on a host. Every member is cleaned out of the production check's
target before the check, and every member artifact the check reports must
be one it built. Every profile this run writes carries the run's own name,
and a merge that read any other profile, such as one a process from an
earlier run wrote after this run's clean, is refused:

```
coverage refusal: the production check reused a build it did not make
coverage refusal: the merge read a profile from another run
```

**A production target or source the report would drop.** A `[[bin]]` or
`[lib]` whose source sits at a test path, or any source a production target
compiles (through a `mod foo_tests;` without `#[cfg(test)]`, a `#[path]`
or an `include!`) that the report would leave out, would compile into the
product unmeasured. The script reads every target from `cargo metadata`,
and every source file each production target compiles from the compiler's
dep-info, and checks each against the whole ignore set: the test
vocabulary, the named exclusions, and the standard library, registry,
toolchain and build-output anchors. The gate drops only what it names:

```
<package>: bin target <name> sits at test path <path>
coverage refusal: a production target escapes the denominator, or an exclusion is not what it claims
a counted production target compiles <path>, which the report would drop
coverage refusal: a production source sits where the report drops it
```

The only way out of the denominator is a package named in the script's
`coverage_exclusions`, with its reason, and it must be `publish = false`.
Today that is one package, `brokkr-seatbelt-probe`: the Seatbelt probe and
its helper executable, whose Gate B roles run only inside a macOS
`sandbox-exec` cell under launchd, so no Linux run can reach most of it. Its
tests still run, so the production code they reach is still measured.

There is no discussion to have here: production code may not shrink its
own denominator. If a line is genuinely unreachable, the fix is to make
it structurally unreachable — remove it, or restructure so the type
system rules it out — not to hide it from the counter.

## Commits, signing, and how your PR actually lands

### Commit messages

The house style, readable in `git log`:

```
<area>: <what changed, lower case, no trailing period>

<why, in prose, wrapped at ~72 columns. What the change makes true
that was not true before, and what it deliberately does not do.>
```

`<area>` is the part of the tree the change lands in — `engine`,
`store`, `view`, `tui`, `cli`, `docs`, `recipes`, `protocol` — and may
be a comma-separated list for a change that crosses several. Cite
decisions by number where one governs the change; the README, the error
messages and the tests all do, and they all mean the same paragraph.

### Signing: what you actually need to do

**Nothing.** You do not need to set up GPG or SSH commit signing to
contribute here.

That is worth stating precisely, because `main` in this repository
carries only signed commits and it would be reasonable to conclude you
need a key. What was observed in this tree's history:

- Every commit reachable from `main` carries a signature. There are no
  unsigned commits on the branch.
- The recent history is uniformly platform-created: those commits have
  committer `GitHub <noreply@github.com>` and are signed with GitHub's
  own web-flow key, `B5690EEEBB952194` — the signature the platform
  applies to a commit it creates itself. (That key id has rotated over
  the repository's life; `git log main --pretty='%GK|%cn'` shows the
  current one and where it changed.)
- Every one of those has exactly one parent and a subject ending in
  `(#NNN)`, and `main` has no merge commits at all —
  `git rev-list --count --merges main` prints `0`. That is squash-merge:
  GitHub collapses the pull request into a single new commit, authors it
  to you, commits it as itself, and signs it with its own key on the way
  in.

So the signature `main` requires is applied *by the merge*, to a commit
that does not exist until the merge happens. Your branch's own commits
are inputs to that; their signatures — or absence of them — never reach
`main`. Setting up local signing to satisfy a rule about `main` is
effort spent on a commit that will be discarded.

What still matters:

- **Author identity.** The squashed commit is authored to you, so set
  `user.name` and `user.email` to something you want in the history.
- **Your branch does not need to be green per commit.** CI triggers on
  `pull_request` (and in the merge queue, and on pushes to `main`), and
  tests the branch, not each commit in it. Squash-merge means the intermediate commits leave
  no trace in `main` anyway. Write the history that is easiest to
  review; you are not being graded on bisectability of commits that
  will be collapsed.
- **The operator keeps push and merge.** No shipped script pushes on
  your behalf, and no seat is chartered to merge. That is a charter rule,
  not a control: an unboxed claude seat with `Bash(git:*)` pre-approved
  runs under your own credentials and settings. Open the pull
  request and it is ruled on by a human.

If your own fork or organisation requires signed commits for its own
reasons, sign them — it changes nothing here either way.

### The merge queue

A pull request's checks run on its merge with `main` as `main` stood when
they started, so two pull requests that are each green can still break
each other, and first meet on `main`. The merge queue (#451) closes that
gap: a pull request whose checks pass is not merged directly, it enters
the queue, and GitHub builds a temporary commit of `main` plus the pull
requests queued ahead of it plus this one, squashed as it will land.
The required checks run again on that commit, raised by the
`merge_group` event, and only a commit whose checks pass reaches `main`.
A pull request whose queued commit fails leaves the queue, and the ones
behind it are rebuilt without it.

Once the operator has approved it, the pull request enters the queue:

```
gh pr merge <number> --auto --squash
```

`--auto` queues it as soon as its own checks pass. What the queue runs
again is everything that judges the combined tree: the tests on both
operating systems, the exact coverage gate with the complexity and
public-API ratchets, format and clippy, the bundle compiles, the MSRV,
the licences, the non-Rust lints, the file-size and duplication
ratchets, the RustSec audit and the release binary. What judges the
pull request's own head or description has done so already and skips in
the queue, and a skipped check counts as passed: `delivered by brokkr`,
`mutants in the diff: brokkr-core`, the suppression check and the
raised baseline's `Ruling:` line.
`crates/brokkr-cli/tests/contributing.rs` holds which job and which step
does which, each skip with its reason.

## The decision culture

Every semantic change is a numbered operator ruling in
[`docs/decisions/`](../decisions/), kept in full and cited by number
in the code that enforces it. That is why the README, an error message
and a test can all say "decision 0007" and mean one paragraph.

The rule that matters to you: **an author may write a decision, only
ever with status `proposed`.** Acceptance is the operator's, recorded in
the file by the operator. A proposal that arrives marked `accepted` is a
review finding, not a shortcut.

The door is open to contributors, not just to the machine's own seats.
[`docs/decisions/README.md`](../decisions/README.md) carries the
grammar a proposal must have — status, context, numbered rulings,
consequences, the enforcement binding for each ruling that can be
enforced deterministically, and how to claim the next free number — and
it is the authority; this document does not restate it.

When does a change need one? When it changes what the engine *means*:
a new phase-machine capability, a change to how results are evaluated,
a new trust rule, a change to what fails closed. When it does not: an
implementation that carries out an existing ruling, a new recipe, a new
adapter, documentation, a bug fix that makes the code match a decision
already written. If you are unsure, write the pull request without one
and say in the description why you think it does not need a decision;
that question is a normal part of review.

A ruling is never edited into a different meaning. Corrections are dated
errata inside the document; a superseding rule takes a new number and
says which one it supersedes.

## What is frozen

Four things in this tree are read-only. A change to any of them is a new
version file beside the old one, never an edit —
[`contracts/README.md`](../../contracts/README.md) states the freeze and the
decisions (0003–0005) it stands on.

| Path | Why |
|---|---|
| `contracts/` v1 | Frozen wire and file contracts. Additive versions (`/v2`, `/v3`, `/v4`) ship as new files; the v1 documents do not move. |
| `fixtures/` | The evaluator behaviour corpus. A frozen contract, never regenerated. A policy-semantics change ships a new corpus version beside it. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from. Its stability is the contract. |
| `reference/` | Read-only heritage: the retired Python oracle, handoff-protocol lore, recorded schemas. |

`recipes/*/policy.json` is *not* the production table — it is bundle
data, and adding or editing a recipe's own table is an ordinary change.

## Recipes and adapters: data the Rust suite witnesses

`recipes/`, `bundles/`, `agents/` and `adapters/` are data. A change to
any of them is JSON and Markdown, not Rust: there is no clippy run over
a policy table, no MSRV question for a role charter, and the coverage
gate reads `crates/` source, so a new recipe adds no uncovered lines to
it. It still runs through `cargo test --workspace` like any other
change, because the Rust suite compiles that data and pins its
identity, and so it faces the same required checks.

This is the honest contribution surface for a first change, and it
should not need a Rust edit. What a data change faces:

1. **`brokkr compile`.** Every recipe and bundle in the tree is compiled
   by `every_bundle_in_the_tree_compiles` in
   [`crates/brokkr-runtime/tests/witness_digests.rs`](../../crates/brokkr-runtime/tests/witness_digests.rs),
   automatically, as soon as the directory exists under `recipes/`.
   Compilation enforces the structural laws — the protected review gate,
   the closed condition vocabulary, aggregate/result agreement, seat
   classes and the trust tier a gate seat requires (decision
   [0021](../decisions/0021-model-policy.md)).
2. **The witness table.** A recipe's identity is the SHA-256 of its
   canonical manifest, which covers every file in it — the policy table,
   the charters, the driver command names — and the adapter declarations
   its seats consult. Every bundle under `recipes/` and `bundles/` has
   that digest pinned under `bundles` in
   [`crates/brokkr-runtime/tests/witnesses.json`](../../crates/brokkr-runtime/tests/witnesses.json),
   and every charter under `agents/charters/` is pinned under
   `charters`; a new recipe is a new row, and a row dropped from the
   table fails as `absent`. Editing one of them, or an adapter a
   pinned bundle resolves through, moves a digest and fails
   `witness_digests.rs` **on purpose**: the point is that a charter
   cannot be softened or a tool added to a driver's list without the
   change being visible as an identity change. The failure lists every
   moved witness at once, as a table of old and new values. Re-pin them
   all with one command:

   ```
   BROKKR_BLESS=1 cargo test -p brokkr-runtime --test witness_digests
   ```

   It rewrites `witnesses.json` in place and refuses where `CI` is set.
   Commit the rewritten file in the same commit as the change, say in
   the message why each value moved, and paste the failure's table into
   the pull request. The reviewed diff of that file is the witness.
3. **The adapter properties.** `library_data.rs` holds every adapter to
   what its data must satisfy rather than to a copy of it: each model
   resolves to a route whose egress the file declares, each route named
   in `routes`, `credentials` or `effortless_routes` is one a mapped
   model reaches, each effortless route gives a dated reason naming the
   release it was measured on, and each alias is either hired by an
   agent or listed in the
   [alias catalogue](provider-adapters.md#the-alias-catalogue). A
   route's class is the operator's ruling, so every classed route is
   also a dated row of the
   [route rulings](provider-adapters.md#the-route-rulings). Adding
   an alias no agent hires is an edit to the adapter file, to its row
   in that catalogue (and a route ruling row if it classes a new route),
   and a bless.
4. **Whatever recipe-specific tests exist.** `recipes/node` has
   `node_recipe_gates.rs` proving its gate seats refuse an untrusted
   driver; `recipes/preflight` has `preflight_shape.rs` proving its
   table stays terminal after review. A new recipe making a structural
   claim should make it a test the same way.

To add a recipe, start from
[`recipe-authoring.md`](recipe-authoring.md) —
the anatomy of `bundle.json`, the policy grammar, composition and
digests — and from a worked example: `recipes/fast` (the flat case),
`recipes/triage` (selected singles, panels, and sequences), `recipes/night-shift`
(composition through `extends`), `recipes/node` (a different language),
`recipes/preflight` (a two-seat table that rules and stops).

To add an adapter, start from
[`driver-authoring.md`](driver-authoring.md) for
the driver protocol, and from an existing file in `adapters/` for the
declaration shape. An adapter declares a provider's `trust_tier`, and a
gate seat's authorisation reads it (decision
[0021](../decisions/0021-model-policy.md)) — so an adapter change is a
change to who is allowed to judge, and it will be reviewed as one. It
also declares where its traffic goes: an `egress` class for its own
destination, and a `routes` map from route name (the prefix of a
concrete model id) to class, for a binary that fronts several
(decision [0036](../decisions/0036-egress-is-a-property-of-the-route.md)).
The `egress` class answers for that one destination and no other: a
model id whose prefix the `routes` map does not name is uncontracted,
whatever the adapter declares for itself, so ruling one endpoint
acceptable never clears the others the same binary can reach. Those
classes decide which seats may put a secret in front of the driver, so
an adapter change is a change to what may be sent as well as to who may
judge. A `credentials` map names, per route, the environment
variable that route needs — a name only, so `brokkr doctor` can say when
the value is coming from the ambient environment rather than from a seat
that binds it (decision
[0040](../decisions/0040-the-flag-is-always-read.md) ruling 4; the route
name's own grammar is in
[`provider-adapters.md`](provider-adapters.md)).

An adapter declares its own destination with `egress`, and an absent
`egress` is `uncontracted`. The superseded `binding_grant` boolean is no
longer read: an adapter still carrying it is refused at load time as an
unknown key.

## Contribution licensing

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as
[Apache-2.0](../../LICENSE-APACHE) or [MIT](../../LICENSE-MIT) at the recipient's
option, without any additional terms or conditions.
