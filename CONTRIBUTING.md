# Contributing to Brokkr

A pull request to `main` goes through Brokkr, unless the operator labels
it `by-hand` or an administrator merges past the check (below). The run
implements, verifies, reviews, and prepares the change; you review the
result and open the pull request that names the run. A branch you wrote
yourself — a decision, a pin, a fix, a sweep of prose — goes through
Brokkr too: the `landing` recipe reads it, verifies it when it is code,
judges it, sends findings back to a smith, and ships it, so the pull
request names a run all the same (decision 0051).

<!-- required-checks:start -->
`main`'s branch protection ([snapshot](.github/branch-protection.json), the setting as it stands once #1383 has landed and the operator's controller has added `build (macos-latest)`, dated 2026-11-25) requires 12 checks: `delivered by brokkr`, `baseline ratchets`, `MSRV (1.89)`, `format, clippy, contracts`, `test (ubuntu-latest)`, `build (macos-latest)`, `exact coverage gate`, `dependency licenses (cargo-deny)`, `non-Rust lints`, `RustSec dependency audit`, `release binary artifact`, and `mutants in the diff: brokkr-core`. It requires signed commits, does not require a branch to be up to date with `main`, does not require an approving review, and does not apply to administrators.
<!-- required-checks:end -->

`delivered by brokkr` is the check that enforces the run: it passes a pull request that names a completed run, or one the operator has given the visible `by-hand` label, which the gate itself reads. Because administrators are exempt, an administrator can still merge past it. `build (macos-latest)` compiles the workspace and every test target on macOS without running tests; the macOS test shards and Seatbelt Gate A (`test (macos-latest)`) run nightly against `main` (`macos-tests.yml`), on `workflow_dispatch`, and in `release.yml` before publication (#1383, operator ruling 2026-11-25).

## 1. Install Brokkr

Download the checksum-verified archive from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
exactly as the [main README](README.md#60-second-bootstrap) shows, or build the
binary from your clone after step 2:

```console
cargo install --path crates/brokkr-cli
```

Those two README paths are the authority. `cargo binstall brokkr-cli`
reads the crate the release workflow publishes to crates.io at each tag
from v0.9.0 ([packaging](packaging/README.md)).

## 2. Fork and clone

```console
gh repo fork feedback-loop-ai/brokkr --clone
cd ./brokkr
git switch -c <your-branch>
```

## 3. Pick one recipe

Costs are relative, not quotes; provider rates and retries vary. Pick a delivery recipe. `landing` is the delivery for a branch that already
exists; `preflight` is an optional branch check, not a delivery. `night-shift` and `wager-harness-dsh` run only on a host serving the dsh `spark-glm` route; [what works today](docs/status.md) says what each harness can do.

<!-- recipe-table:start -->
| Recipe | When to use it | What it seats | Rough cost |
|---|---|---|---|
| `brokkr-fast` | Brokkr's own fast: fast's table and scripts, with its two model seats hiring Brokkr's offices (decision 0087 ruling 4). | implement, review, ship, verify | medium |
| `fast` | Default Rust delivery from implementation through verification, review, and ship. | implement, review, ship, verify | medium |
| `gpt-flash` | GPT Sol plans, DeepSeek Flash 4.1 implements, and Sol rules final review with Astra as its last fallback. | analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement{chore=gpt-flash-implementer;design=gpt-flash-implementer-sdd;engine=gpt-flash-implementer-engine;feature=gpt-flash-implementer}, review{chore=positions>chief;design=positions>chief;engine=positions>chief;feature=positions>chief}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify] | variable |
| `landing` | A branch authored by hand is classified, verified when it is code, judged, remediated on findings and shipped by a run that vouches for it: fast entered at a classify gate, the road for shop work instead of the by-hand label. | classify, implement, review, ship, verify | low |
| `night-shift` | Unattended triage routing that parks on the first unusual result and uses the dsh implementation lane. | analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement, review{chore=brokkr-reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify] | medium-high |
| `node` | Node and TypeScript repositories using JavaScript-specific seats and tools. | implement, review, ship, verify | medium |
| `panel-review` | General delivery needing independent correctness and security reviewers. | implement, intake, review[correctness+security], ship, verify | high |
| `preflight` | Verify and review an existing branch without implementing or shipping it. | review, verify | medium |
| `release` | Prepare a configured release: versions, commit-derived notes, documentation and organization-profile patches, followed by verification and review. | implement, review, ship, verify | medium |
| `research` | Read articles and propose registry entries with cited classifications; the operator rules them. | research, verify | low |
| `research-dsh` | The research intake on the dsh lane: the same charter and gate, the researcher seated on Qwen3.8-Max with the research office’s web requests. | researcher, verify | low |
| `review-first` | An existing branch judged first: Sol reviews, the suite verifies, Muse remediates findings on return, and the run ships. | implement, review, ship, verify | medium |
| `standby` | Fast's shape with every model seat on the other vendor at judge-grade effort, so delivery survives one account's exhausted limit. | implement, review, ship, verify | medium |
| `triage` | Routing delivery: a chief-grade triage gate rules the class before Fast's crew, adding the current SDD design council when ruled. | analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement{chore=brokkr-implementer;design=implementer-sdd;engine=implementer-engine;feature=brokkr-implementer}, review{chore=brokkr-reviewer;design=positions>chief;engine=positions>chief;feature=review-correctness+review-security}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify] | variable |
| `wager-harness` | Driver evaluation that swaps only implementation to Codex for a fair wager. | implement, review, ship, verify | medium |
| `wager-harness-dsh` | Driver evaluation that swaps only implementation to DSH for a fair wager. | implement, review, ship, verify | medium |
| `wager-harness-muse` | Driver evaluation that swaps only implementation to Muse Spark 1.3 on its contributor terms, through dsh, for a fair wager. | implement, review, ship, verify | low |
<!-- recipe-table:end -->

## 4. Light the run

```console
brokkr run --recipe <name> --feature "<what you want>"
brokkr run --recipe landing --repo . --feature "landing: <what the branch is, and why>"
```

The second form is for a branch you already wrote: it starts by reading
the branch, not by writing one. Let the machine finish its verify,
review, and ship seats. Fixes return through the same policy; do not
substitute hand-run checks for the run.

## 5. Review, publish the evidence, and propose

Review the branch and the run. Brokkr writes the final anchor locally;
publish its synthetic commit beside your branch without changing HEAD:

```console
git push origin refs/forge/<run-id>:refs/heads/brokkr-runs/<run-id>
```

Open the pull request using the repository template and replace its line
with `Brokkr-Run: <run-id>`. CI verifies the published journal offline,
requires a completed run, and cuts a tier by what changed since it
judged (decision 0038): a rebase that leaves your slice's patch unchanged
keeps the vouch; a delta confined to docs needs `brokkr run --recipe
preflight` on the new head, named as `Brokkr-Preflight: <run-id>`; a code
delta needs a new run. Only the operator may apply the visible `by-hand`
escape-hatch label, and only where no landing can stand (decision 0051).

Seat commits are unsigned; `main` requires signatures; the operator squash-merges, and that merge is the signed commit. A pull request that changes only documentation, `.md` pages under `docs/` or at the top level that no code names, runs [the docs lane](docs/guides/contributing-by-hand.md#the-docs-lane): the tests that read documentation, in minutes, in place of the test and coverage machines, and the two suite checks pass on that lane's own result. It still lands through the merge queue's full run (#1295).

## 6. Dependency admission

One production consequence of a dev-only edge is admitted here.
`boa_engine = "=0.21.1"`, the CLI's browser-controller test dependency,
requires `icu_normalizer = "~2.0.0"`; the production `url -> idna` edge
resolves `idna_adapter 1.2.2`, which requires `icu_normalizer = "2.2"`.
Cargo unifies semver-compatible versions, so the whole workspace —
including the release-linked graph — carries `icu_normalizer 2.0.1` and
`idna_adapter 1.2.1` instead of the `icu_normalizer 2.3.0` and
`idna_adapter 1.2.2` that `main` carried. No known vulnerability is
asserted for either version: both remain on the `deny.toml` licence
allowlist, and cargo-deny and the RustSec audit read this same lockfile.
Removal is blocked while Boa stays pinned at `=0.21.1` for the MSRV and
the exact-served-code proof; this admission is revisited when Boa's icu range admits the newer normalizer.

## 7. Workflow closed form

Every file under `.github/workflows/` and `.github/actions/` (found by walking both directories, never a hand-kept list) and Renovate's two configuration files `.github/renovate.json5` and `.github/renovate-global.json5` by name are pinned by SHA-256 over their checked-out bytes in `.github/workflow-digests.txt` (#1417, operator ruling 2026-11-27); every path component under those two directories must be one or more ASCII letters, digits, `.`, `_`, or `-` (never `.` or `..`), any other name is refused with its bytes escaped before anything is hashed or written, and the digest file admits only those two named files outside the two directories. The `workflow_digests` guard is the boundary: any edit to a pinned file — or a missing, extra, invalidly named, symlinked, or non-regular entry (including either Renovate file missing or non-regular), a format deviation, or a local `uses: ./…` resolving outside those two directories — fails the guard until the same change refreshes `.github/workflow-digests.txt` with `bash scripts/refresh-workflow-digests.sh` (verify without writing via `--check`). Moving a digest is a visible, reviewed act: the author states in the pull request what changed and why, and the reviewer reads the change itself, knowing no guard or reader of ours read all of it for them. Underneath the pin, the semantic guards over `ci.yml`, `macos-test-shards.yml`, `macos-tests.yml`, and `release.yml` do not emulate GitHub Actions: they read a small closed form and refuse anything outside it.

- **Total admission:** when a covered workflow is read through `ClosedWorkflow`, every top-level key, trigger, matrix, condition, and artifact name or pattern in the file is checked statically against this closed form up front, whether or not that guard later asks about that job or step. One item outside the form refuses the file in any guard reading it through `ClosedWorkflow`. Admission produces typed values that those guards evaluate directly without re-parsing raw strings.
- **Final refusals:** any refusal from admission or evaluation is carried forward to fail the guard with its refusal sentence, rather than turned into `false`, a pass, or another diagnostic; the opaque `Verdict` type structures evaluation paths around this rule rather than making a dropped refusal impossible to write.
- **Top-level keys and environment:** only `name`, `on`, `permissions`, `concurrency`, and `jobs` are admitted; no workflow-level `env:` (use job-level or step-level `env:` only, e.g. `BROKKR_TEST_MACHINE: ${{ matrix.machine }}` on a step).
- **Conditions (`if:`):** omit `if:`, or write a scalar string without `${{ }}` (a non-scalar `if:` such as a YAML sequence, mapping, boolean, number, or null is refused with the closed-form sentence naming the job or step) in one of these shapes: `always()`; one comparison or comparisons joined by `&&` only, optionally prefixed by `always() &&` (e.g. `always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'`); comparisons joined by `||` only (e.g. `steps.flake-pr.outputs.pull-request-operation == 'created' || steps.flake-pr.outputs.pull-request-operation == 'updated'`); or one bare context path alone, allowed only for a `workflow_call` input declared `type: boolean` (e.g. `inputs.use-cache`) or a `matrix.<axis>` where at least one row gives a non-empty string and every other row gives a non-empty string or omits it (e.g. `matrix.nfpm_arch`). No parentheses, no `!`, no mixing `&&` with `||`, and no function except leading `always()`. Without leading `always()`, GitHub's implicit `success() && (...)` always applies, including when `if:` is omitted.
- **Comparisons, paths, and literals:** `<context path> == '<literal>'` or `<context path> != '<literal>'` (ASCII case-insensitive string comparison). The left side must be statically known to be a string in that position: `github.event_name` or `runner.environment`; `matrix.<axis>` only where every row of that job's matrix gives `<axis>` a non-empty string; `inputs.<name>` only for a declared input of `type: string` or `choice` (a boolean input is never compared, only used bare); `needs.<job>.result` and `needs.<job>.outputs.<name>` only where `<job>` is in this job's `needs` and, for an output, declares `<name>` in `outputs:`; `steps.<id>.outputs.<name>` only on a step, and only where an earlier step in the same job declares `id: <id>`. The right-side literal must be a non-empty ASCII string of letters, digits, `.`, `_`, `/` and `-` that is not a number.
- **Artifact names and patterns:** upload/download `name:` is a plain `[A-Za-z0-9._-]+` literal with no blanks (e.g. `brokkr-macos-arm64`), `${{ matrix.<axis> }}` as the whole value over plain literals (e.g. `${{ matrix.name }}`), or `<literal>${{ strategy.job-index }}` (e.g. `coverage-machine-${{ strategy.job-index }}`). Download `pattern:` is a plain literal or `<literal>*` (e.g. `coverage-machine-*`). Leading or trailing blanks are refused, never trimmed.
- **Triggers, matrices, and scripts:** `on:` events carry no filter (e.g. `workflow_dispatch:`) or only list-valued `branches`, `tags`, `types` (a scalar filter value is refused at its place, never admitted as a one-item list), `schedule` as a non-empty list of mappings with `cron` and no other key (e.g. `schedule: [{ cron: '17 4 * * *' }]`), or `workflow_call` with only `inputs`; any unlisted trigger key is refused, never dropped. Matrices carry no `exclude` key (empty or non-empty) in any job of a covered file and no `include` key except in jobs whose `include` rows a guard reads and accounts for — today `ci.yml`'s `pr-caches` and `release.yml`'s `build` (pure `include` lists with no `axes` and no `exclude`). Checked scripts are identified whole (exact text) before a guard runs or reasons about them; no check, positive or negative, matches a script by substring.
- **Refusal:** anything outside this form fails the guard with one sentence: `<file> (<place>): <detail> is not a form this guard reads; allowed forms: <allowed>.` A file carrying two faults is refused, and which fault a guard names first is not promised (no out-of-form shape under `on:` reaches a generic error on its own).
- **What the guards do not read:** a job's `needs` beyond what a guard evaluates, `shell` and `defaults`, a job-level `concurrency`, a matrix on a job no guard expands, a step's `if` inside the nightly's notifier, steps a guard does not name, `inputs` in a workflow that has another trigger, and the scripts a step calls (those are code, reviewed and tested as code). Anything the guards do not read is the pin's to surface and the reviewer's to read by hand.

Curious about the machinery? [The by-hand guide](docs/guides/contributing-by-hand.md) preserves the twelve exact checks, coverage practicalities and refusals, signing walkthrough, decision culture, and frozen surfaces; a landing's verify seat runs `cargo fmt --all -- --check`, `bash scripts/lint-non-rust.sh --seat`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace` and `cargo run -p brokkr-cli -- library compile --bundle recipes/self`, and CI proves the rest (any non-Rust lint whose tool the seat cannot reach, which the seat names as not run, the MSRV, the suppression check, the `recipes/verify` compile, the suite in CI's own form on Linux (with `build (macos-latest)` compiling the workspace on macOS, and the macOS test shards running nightly, on dispatch and at release), exact coverage, cargo-deny, the diagram render and Renovate's validator, the ratchets, the RustSec audit, the release build and the mutants gate), so contributors do not need to run them.

Contributions are dual licensed under Apache-2.0 OR MIT unless you say otherwise.

For finished branch work, `brokkr run --recipe landing --feature "finish existing work" --enter classify --continues latest` attributes a new landing to its earlier run. [Gate entry](docs/guides/quickstart.md#re-drive-finished-branch-work-at-a-gate) works with any recipe whose gate passes admission; the shipping anchor supplies the vouch.
