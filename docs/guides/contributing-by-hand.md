# Contributing to Brokkr by hand

This repository's engine forges its own changes and reviews them
adversarially. The bar for a human contribution is the bar the machine
is already held to — nine required checks, none of them a percentage you
can nudge. This document is the whole walk from `git clone` to a green
pull request, with every command written out.

Nothing here is lowered for a first contribution. What this document
does instead is make the bar reachable: every gate stated with the exact
command that reproduces it locally, every refusal shape named with its
fix, and a way to have the machine review your branch before a human
ever looks at it.

- [What you need installed](#what-you-need-installed)
- [Fork, clone, branch](#fork-clone-branch)
- [The nine checks](#the-nine-checks)
- [The pre-flight: let the machine review you first](#the-pre-flight-let-the-machine-review-you-first)
- [The coverage gate, practically](#the-coverage-gate-practically)
- [Commits, signing, and how your PR actually lands](#commits-signing-and-how-your-pr-actually-lands)
- [The decision culture](#the-decision-culture)
- [What is frozen](#what-is-frozen)
- [Recipes and adapters: the surface that never faces the core gates](#recipes-and-adapters-the-surface-that-never-faces-the-core-gates)
- [Contribution licensing](#contribution-licensing)

## What you need installed

The engine is Rust-only (decision
[0009](../decisions/0009-rust-only.md)): no Python, no Node, no
toolchain beyond cargo for the ordinary path. Three of the nine checks —
the MSRV, the coverage gate and the licence gate — need something beyond
a stable toolchain.

| Tool | Needed by | Check it is there |
|---|---|---|
| A stable Rust toolchain | format, clippy, tests, the bundle compiles, the release build | `cargo --version` |
| Rust 1.88.0 | the MSRV check | `cargo +1.88.0 --version` |
| A nightly toolchain with `llvm-tools-preview` | the coverage gate | `cargo +nightly --version` |
| `cargo-llvm-cov` | the coverage gate | `cargo llvm-cov --version` |
| `jq` | the coverage gate (the script refuses without it) | `jq --version` |
| `cargo-deny` | the licence gate | `cargo deny --version` |

The extra toolchains and tools install the usual way — `rustup toolchain
install 1.88.0`, `rustup toolchain install nightly --component
llvm-tools-preview`, `cargo install cargo-llvm-cov cargo-deny`, and `jq`
from your package manager. There is no `rust-toolchain.toml` in this
tree, so your default toolchain is what `cargo` uses and the `+1.88.0`
and `+nightly` prefixes are how the other two get selected.

You do **not** need `cargo-audit`; the RustSec check runs only in CI.
Installing it locally is a convenience, not a requirement — see
[the two checks you cannot fully reproduce](#the-two-checks-you-cannot-fully-reproduce).

## Fork, clone, branch

```
gh repo fork feedback-loop-ai/brokkr --clone
cd brokkr
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

## The nine checks

All nine are required jobs in
[`../../.github/workflows/ci.yml`](../../.github/workflows/ci.yml). They run on every
pull request. This is the full list, in CI's own order:

| # | CI check | Job | Local command |
|---|---|---|---|
| 1 | `MSRV (1.88)` | `msrv` | `cargo +1.88.0 check --workspace --locked` |
| 2 | `format, clippy, contracts` | `quality` | `cargo fmt`, `cargo clippy`, two `compile --bundle` runs |
| 3 | `test (ubuntu-latest)` | `engine` | `cargo test --workspace --all-features --locked` |
| 4 | `test (macos-latest)` | `engine` | — (your machine is one OS) |
| 5 | `test (windows-latest)` | `engine` | — (your machine is one OS) |
| 6 | `exact coverage gate` | `coverage` | `bash scripts/coverage-exact.sh` |
| 7 | `dependency licenses (cargo-deny)` | `license-compliance` | `cargo deny check licenses` |
| 8 | `RustSec dependency audit` | `dependency-audit` | — (CI-only; see below) |
| 9 | `release binary artifact` | `release-binary` | `cargo build --release --locked -p brokkr-cli` |

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

### The workspace suite

```
cargo test --workspace --all-features --locked
```

The full suite, untruncated. On this tree it builds and runs 45 test
binaries; the last run before this document was written reported
`test result: ok` for every one of them, 787 tests passing and none
failing. Read the whole output rather than the last line: a suite can be
green overall while a binary you expected to gain a test gained none.

`--all-features` and `--locked` are not decoration. `--locked` refuses
to update `Cargo.lock`, so your run uses the dependency graph CI will
use; if it errors about the lockfile being out of date, your `Cargo.toml`
change needs its lockfile update committed too.

### The MSRV

```
cargo +1.88.0 check --workspace --locked
```

The README's badge says 1.88+, and this check is what makes that a fact
rather than prose. CI pins the toolchain with
`dtolnay/rust-toolchain@1.88.0` and then runs plain `cargo check
--workspace --locked`; locally you need the explicit `+1.88.0`, because
your default toolchain is newer and would not notice.

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

and expect a digest test to move — see
[recipes and adapters](#recipes-and-adapters-the-surface-that-never-faces-the-core-gates).

### Exact coverage

```
bash scripts/coverage-exact.sh
```

Literal 100% of lines, branches and functions across the workspace, or
refusal. There is no threshold to lower. This one has its own section:
[the coverage gate, practically](#the-coverage-gate-practically).

One operational note before you run it. The script builds a second,
instrumented copy of the workspace under a temporary directory:

```
forge_coverage_dir="$(mktemp -d "${TMPDIR:-/tmp}/forge-coverage.XXXXXX")"
```

— `scripts/coverage-exact.sh:17`. On a machine where `/tmp` is a tmpfs
(RAM-backed, and commonly a few gigabytes), that instrumented target
directory can fill it and the run dies with `ENOSPC` partway through a
link step. This project has hit exactly that. If your `/tmp` is small or
RAM-backed, point `TMPDIR` at a disk-backed scratch directory before
running:

```
mkdir -p target/coverage-scratch
TMPDIR="$PWD/target/coverage-scratch" bash scripts/coverage-exact.sh
```

`target/` is git-ignored, so the scratch directory never reaches a
commit. The script deletes its own temporary directory on exit either
way.

### Dependency licences

```
cargo deny check licenses
```

Prints `licenses ok` and exits 0 when every crate in `Cargo.lock`
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

CI runs this through `EmbarkStudios/cargo-deny-action@v2` rather than
your local binary, so a version skew is possible; `cargo deny check
licenses` locally is the same check reading the same `deny.toml`.

### The RustSec advisory audit

CI runs `rustsec/audit-check@v2.0.0`, which fetches the RustSec advisory
database and scans `Cargo.lock`. **This is the one check with no exact
local equivalent** — the Action reports through the GitHub Checks API
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
```

Builds `target/release/brokkr`, the only binary this repository ships.
It rarely fails on its own once the suite is green — the release profile
compiles the same code. What shows up here and nowhere else is anything
conditioned on the debug profile: an item behind
`#[cfg(debug_assertions)]` that non-debug code depends on, or a `cargo
test`-only path that hid a warning.

### The two checks you cannot fully reproduce

Be honest with yourself about these two, and say so in the pull request
if you think they are at risk:

- **The three-OS matrix.** Checks 3–5 are the same command on Ubuntu,
  macOS and Windows. You ran one. Path separators, line endings and
  anything touching the filesystem are where this bites. Note that
  [`.gitattributes`](../../.gitattributes) normalises every text file to LF in
  the working tree on every platform, precisely because bundle digests
  are taken over file bytes — so do not "fix" a line ending.
- **The RustSec audit**, as above.

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
branch, and no ship to merge it. The policy table
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

**A new function nothing calls.** Same shape, one level up, with one
wrinkle: the fold identifies a source function by file and start line
and sums the hits of every compiled instantiation of it, so a single
`FNDA:0,` against a mangled name is not the miss. The miss is a `FN:`
start line under your file's `SF:` heading whose every `FNDA` record
reads `0`. Either a test calls it, or it should not exist yet. This
repository does not carry code ahead of its use.

**Test-harness source in the production report.** The script checks for
this before it checks coverage
(`scripts/coverage-exact.sh:43-50`):

```
coverage refusal: test harness source leaked into the production report
```

The check reads the report's filenames and refuses any that match
`tests.rs`, `*_tests.rs`, or a `tests/` directory component — the paths
`cargo-llvm-cov` treats as harness. Test modules in this workspace
therefore sit in a sibling file, declared from the production file as
`#[cfg(test)] mod tests;` or `#[cfg(test)] mod foo_tests;`, or under
`crates/<crate>/tests/` — never as an inline `#[cfg(test)] mod tests {
… }` block with the bodies in the production file. Follow whichever of
the two the crate you are editing already uses; every crate here uses
one of them.

**A `coverage(off)` attribute.** Forbidden outright. The script greps
for it before it runs anything
(`scripts/coverage-exact.sh:9-14`) and refuses:

```
coverage refusal: attribute-based source exclusions are forbidden
```

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
  `pull_request` (and on pushes to `main`), and tests the branch, not
  each commit in it. Squash-merge means the intermediate commits leave
  no trace in `main` anyway. Write the history that is easiest to
  review; you are not being graded on bisectability of commits that
  will be collapsed.
- **The operator keeps push and merge.** Nothing in this repository
  pushes on your behalf, and no agent merges anything. Open the pull
  request and it is ruled on by a human.

If your own fork or organisation requires signed commits for its own
reasons, sign them — it changes nothing here either way.

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

## Recipes and adapters: the surface that never faces the core gates

`recipes/`, `bundles/`, `agents/` and `adapters/` are data. A change to
any of them is JSON and Markdown, not Rust, and it does not face the
gates a `crates/` change faces: there is no clippy run over a policy
table, no MSRV question for a role charter, and the coverage gate reads
`crates/` source, so a new recipe adds no uncovered lines to it.

This is the honest contribution surface for a first change. What a data
change *does* face:

1. **`brokkr compile`.** Every recipe and bundle in the tree is compiled
   by `every_bundle_in_the_tree_compiles` in
   [`crates/brokkr-runtime/tests/witness_digests.rs`](../../crates/brokkr-runtime/tests/witness_digests.rs),
   automatically, as soon as the directory exists under `recipes/`.
   Compilation enforces the structural laws — the protected review gate,
   the closed condition vocabulary, aggregate/result agreement, seat
   classes and the trust tier a gate seat requires (decision
   [0021](../decisions/0021-model-policy.md)).
2. **The digest tests.** A recipe's identity is the SHA-256 of its
   canonical manifest, which covers every file in it — the policy table,
   the charters, the driver command names. `recipes/fast`,
   `recipes/node`, `recipes/preflight` and `bundles/verify` have that
   digest pinned in `witness_digests.rs`. Editing one of them moves the
   digest and fails a test **on purpose**: the point is that a charter
   cannot be softened or a tool added to a driver's list without the
   change being visible as an identity change. Re-pin it deliberately,
   in the same commit, with the reason in the commit message.
3. **Whatever recipe-specific tests exist.** `recipes/node` has
   `node_recipe_gates.rs` proving its gate seats refuse an untrusted
   driver; `recipes/preflight` has `preflight_shape.rs` proving its
   table stays terminal after review. A new recipe making a structural
   claim should make it a test the same way.

To add a recipe, start from
[`recipe-authoring.md`](recipe-authoring.md) —
the anatomy of `bundle.json`, the policy grammar, composition and
digests — and from a worked example: `recipes/fast` (the flat case),
`recipes/sdd` (panels and sequences), `recipes/sdd-paranoid`
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

The shipped adapter files still carry the superseded `binding_grant`
boolean, which reads for one more release — `true` as `contracted`,
`false` or absent as `uncontracted`. If you copy one of them as your
starting shape, replace that key rather than adding `egress` beside it:
an adapter declaring both is refused at load time, because the two could
disagree and only one of them could win.

## Contribution licensing

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as
[Apache-2.0](../../LICENSE-APACHE) or [MIT](../../LICENSE-MIT) at the recipient's
option, without any additional terms or conditions.
