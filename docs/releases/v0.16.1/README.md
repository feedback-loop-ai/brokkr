# v0.16.1 release handoff

Release description: [v0.16.1.md](../v0.16.1.md).

## Version and comparison range

- **Target version:** `0.16.1` (tag `v0.16.1`), a patch on `v0.16.0` with no
  release name. Operator approved the release and its tag on 2026-11-26.
  Tagging, pushing, publication, channel merges and live profile edits
  remain operator actions.
- **Previous published release:** `v0.16.0` (annotated tag `618a1298c6c9339c11bb6e57953e3b301883a6e7`,
  commit `6be99f96f5f74316645ff73d77f5f93a78807ad8`). Verified via GitHub's
  releases API as published (`2026-11-25T23:49:05Z`, `draft: false`,
  `prerelease: false`) with eight platform archives/packages and `SHA256SUMS`,
  and verified against `git ls-remote origin`.
- **Base commit:** `1a104b57129ca6e8618cc0da06250b7be87aa92a` (`refs/heads/main`
  on `origin`), 11 commits after `v0.16.0`. Comparison range read:
  `v0.16.0..1a104b571`. The release manager prepared the candidate on
  `13e1e448a`, ten commits after the tag; the controller then merged
  `1a104b571` (`#1293`, decision 0121) and added its entries to the notes
  by hand. No run reviewed that addition.
- **Candidate branch:** the release manager's `release-0161b` on base `13e1e448a` (review range
  `13e1e448a..HEAD`; round 1 candidate `fa12f5587` merged with `13e1e448a`
  at `1114cab43`, then updated in round 2). The release pull request records
  the final head commit and its remote CI verdict.

## Candidate edits and derived identities

- **Version files (`Cargo.toml`, `Cargo.lock`):** moved `[workspace.package].version`
  and the six `brokkr-*` workspace path dependencies from `0.16.0` to `0.16.1`.
  `cargo update --workspace --offline` updated only the eight workspace crates
  in `Cargo.lock`; zero registry dependencies moved.
- **Witness and compose digests (`crates/brokkr-runtime/tests/witnesses.json`):**
  re-measured in two stages with `BROKKR_BLESS=1` (`CI` unset) using
  `witness_digests::`:
  1. After bumping only `Cargo.toml` and `Cargo.lock`, exactly the 19 bundle
     digests changed (because the engine version participates in bundle
     identity); zero policy-table or charter digests changed.
  2. After updating the sample command in `recipes/release/README.md` from
     `v0.16.0` to `v0.16.1`, only `recipes/release` moved once more
     (`5e44b350…` → `647f3b92…`), with all other 18 bundle digests unchanged.
  3. After merging `13e1e448a` (`#1410`) and updating the release notes and
     documentation in round 2, zero bundle digests moved (verified by the
     `witness_digests` suite).
- **Notes and changelog reset:** authored `docs/releases/v0.16.1.md` and reset
  `docs/releases/unreleased.md` to track changes after `v0.16.1`.

## Documentation audit

| Target | Disposition | Evidence |
|---|---|---|
| `README.md` | updated | Replaced the opening release summary with `v0.16.1` (`brokkr orders` as fourteenth root, `forge.realms/v9` per-provider dialect grants, Claude 5.5 on agy, Qwen 4.0 Max on `dsh`, residual safety join, contended checkpoint stderr, one-hour workspace timeout) while keeping `v0.16.0` “Draupnir” as the previous release. Install table historical "live from" labels and bootstrap scaffold list (`claude`, `codex`, `dsh`) remain accurate. |
| `ARCHITECTURE.md` | unchanged | Verified all 359 lines against `v0.16.0..13e1e448a`: same workspace crate graph (7 crates + `brokkr-seatbelt-probe`), same seven harness kinds, evaluator corpus still frozen at 97 cases (`fixtures/work-order/` is additive contract fixtures), and no stale version strings or command counts. |
| `CONTRIBUTING.md` | unchanged | Already reconciled by `#1383` (`8ffd34dd7`): lists the twelve required checks with `build (macos-latest)`, nightly/dispatch `macos-tests.yml` and pre-publish `release.yml` macOS test shards; holds no current-version string. |
| `docs/guides/agent-library.md` | updated | Added `forge.realms/v9` for per-provider `dialects` maps beside `forge.realms/v6` and `forge.realms/v8` in the realm grant summary. |
| `docs/guides/quickstart.md` | updated | Updated `brokkr --version` and `brokkr doctor` sample outputs to `0.16.1`; verified by `doc_samples`. |
| `docs/guides/versioning.md` | updated | Updated the four current-version references and the release-notes link from `0.16.0` to `0.16.1`, reconciled cleanly with `#1410` (`13e1e448a`), which added `forge.realms/v9` as `current` and moved `forge.realms/v8` to `reader`. `work-order/v1` and `brokkr.journal-backup.v2` were already recorded by `#1327` (`199f1a620`). |
| other `docs/guides/` | unchanged | Audited `provider-adapters.md` (already carries `agy-claude-*` and `qwen4-max` from `#1408`; its `0.16.0` row at line 1012 is a historical measurement record on the released `0.16.0` binary and is preserved), `read-surfaces.md` (already documents `brokkr orders` from `#1327`), `contributing-by-hand.md` (already reflects `#1383` and `build (macos-latest)`), and all remaining guides (no stale `0.16.0`, `600,000`, or `journal-backup.v1` references). |
| `packaging/README.md` | unchanged | Contains no current-version references and matches the packaging scripts and `release.yml` publish flow. |

Additional file updated: `recipes/release/README.md` (sample invocation moved
to `v0.16.1`). Preserved as historical or out-of-band: `flake.nix` (stays at
`0.16.0` until the post-release channel PR updates its tarball digests) and all
earlier files under `docs/releases/`.

## Validation results on the candidate

Configured checks:

| Check | Status | Notes |
|---|---|---|
| format (`cargo fmt --all -- --check`) | passed | Clean exit 0. |
| clippy (`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`) | passed | Clean exit 0 across all workspace targets and features. |
| tests (`cargo test --workspace --all-features --locked`) | partial (pending full CI run) | Running `cargo test --workspace --all-features --locked` with `--test-threads=16` locally did not finish within the per-call limit: the implement seat's MCP client timed out at 180 seconds before the 280-second command timeout inside the 300-second workspace response limit (reported unverified in full locally; the full suite runs in CI on the release PR). Targeted local suites passed on the candidate: `brokkr-core` library (460 passed) and integration (32 passed), `brokkr-view` library (467 passed, 3 ignored), `brokkr-bridge` library (23 passed), `brokkr-runtime` integration (`witness_digests` and `frozen_contracts`: 34 passed; full integration suite in round 2: 176 passed), and `brokkr-cli` integration modules `layering` (36 passed, including `every_test_it_filter_names_a_test_of_its_crates_binary`), `decisions`, `doc_truth` and `doc_samples` (67 passed), and `contract_lifecycle` and `realms` (39 passed). |
| self bundle (`library compile --bundle recipes/self`) | passed | Digest `b7879cd3350d…`; 48 valuations over 11 groups, 0 unruled. |
| verify bundle (`library compile --bundle recipes/verify`) | passed | Digest `898ad10e7ae3…`; 16 valuations over 5 groups, 0 unruled. |
| exact coverage (`scripts/coverage-exact.sh`) | pending | Requires host/CI execution outside the nested-namespace box (`exact coverage report` and `exact coverage gate` in CI). |
| release binary (`cargo build --release --locked -p brokkr-cli`) | passed | Built cleanly (~41 MB); `target/release/brokkr --version` prints `brokkr 0.16.1`. |
| remote CI | pending | Must pass on the final PR head before tagging. Required checks on `main` (`.github/branch-protection.json`): `delivered by brokkr`, `MSRV (1.89)`, `format, clippy, contracts`, `test (ubuntu-latest)`, `build (macos-latest)`, `exact coverage gate`, `dependency licenses (cargo-deny)`, `non-Rust lints`, `baseline ratchets`, `RustSec dependency audit`, `release binary artifact`, and `mutants in the diff: brokkr-core`. The candidate PR head itself has no remote CI runs yet. |

Additional local gates on the candidate:

- `typos --hidden`: passed (`0` findings).
- `bash scripts/lint-non-rust.sh --seat`: `lint-non-rust: 5 of 6 lints ran clean`; `zizmor --offline` reported as not run (`zizmor` not on `PATH`; runs in CI `non-Rust lints`).
- `quality/ratchet.sh files`: `ratchet: file size holds`.
- `quality/ratchet.sh clones`: `ratchet: duplication holds`.
- `quality/ratchet.sh api`: `ratchet: public API holds`.
- `git diff --check`: unfiltered `git diff --check 13e1e448a..HEAD` exited `0` (`0` findings).

## Extensions

1. **Workflow toolchain agreement — verified.** `rust-nightly-version.txt` pins
   `nightly-2026-09-05`. Every coverage job reads that file:
   `.github/workflows/ci.yml` (lines 436, 438, 523, 525),
   `.github/workflows/release.yml` (lines 80, 82), and
   `scripts/coverage-exact.sh` (line 224). No workflow hard-codes a nightly.
2. **Frozen contracts — additive only.** `git diff v0.16.0..13e1e448a` over
   `contracts/`, `fixtures/`, `policy/phase-machine.json` and `reference/`
   adds `contracts/work-order.v1.schema.json` (decision 0116),
   `contracts/realms.v9.schema.json` (decision 0126), 29 work-order
   validation fixtures under `fixtures/work-order/`, and the work-order and
   realms v9 sections in `contracts/README.md`. Zero frozen contract schemas,
   zero existing fixtures, zero bytes of `policy/phase-machine.json`, and
   nothing under `reference/` changed. Compatibility notes for
   `work-order/v1`, `forge.realms/v9` and `brokkr.journal-backup.v2` are in
   [v0.16.1.md](../v0.16.1.md#upgrade-and-compatibility).

## Organization profile patch

- **Target:** `feedback-loop-ai/.github`, file `profile/README.md`.
- **Base commit:** `c4e3df75ce6805efba9db6870e63c28a8b9ea736` (re-verified
  as `refs/heads/main` of `feedback-loop-ai/.github` via `git ls-remote`).
- **Reviewed patch:** [`organization-profile.patch`](organization-profile.patch);
  verified in round 1 with `git apply --check` on the recorded base, then
  trimmed of its leading blank context line (`@@ -27,6 +27,6 @@`, keeping two
  context lines above and three below) so unfiltered `git diff --check` passes
  cleanly (re-checked synthetically with `git apply --check`).
- **Scope of diff:** updates only Brokkr's State cell from `v0.16.0` (and its
  release URL) to `v0.16.1`. Audited the full file: every other project row,
  badge, start-here command (`brokkr runs inspect`), harness list, and prose
  paragraph remains accurate for `v0.16.1`.
- **Application timing:** pending — apply only after `v0.16.1` publication is
  verified.

## Remaining operator steps (ordered, all pending)

1. **Open the release pull request** from the candidate branch onto `main`
   and verify all required remote CI checks pass on the final candidate head
   (including `test (ubuntu-latest)`, `build (macos-latest)`, `MSRV (1.89)`,
   `dependency licenses (cargo-deny)`, `RustSec dependency audit`,
   `non-Rust lints` including `zizmor`, `baseline ratchets`, `packaging channels`,
   `release binary artifact`, and `exact coverage gate`).
2. **Squash-merge the release PR once its checks pass, then push the signed
   tag `v0.16.1`** on the merge commit. That squash merge is the signed commit
   `main` requires, as `CONTRIBUTING.md` says; the candidate's own commits are
   unsigned seat commits.
3. **Wait for `.github/workflows/release.yml` (including the macOS test shards).**
   Under `#1383`, the release workflow now runs the five macOS test shards and
   Seatbelt Gate A (`test (macos-latest)`) from a cold tree before `build` and
   `publish`. Because `#1374`'s `dispatch::` tests have failed one macOS shard
   in recent runs and passed on re-run, **if a macOS shard fails in the release
   run, re-run that failed shard before concluding anything else**; publication
   waits on `test (macos-latest)` passing.
4. **Verify the GitHub release:** confirm all four platform tarballs, Linux
   `.deb` and `.rpm` packages, `SHA256SUMS`, and signed build provenance are
   published, and attach the description from `docs/releases/v0.16.1.md`.
5. **Verify crates.io:** confirm all seven published workspace crates serve
   `0.16.1`.
6. **Verify apt and rpm:** confirm the signed GitHub Pages repositories index
   the `0.16.1` packages.
7. **Merge Homebrew and Nix channel PRs:** confirm the generated PRs for
   `feedback-loop-ai/homebrew-tap` and `flake.nix` match `0.16.1` and the
   published `SHA256SUMS`, then merge them.
8. **Apply the organization profile patch:** once publication is verified, apply
   `docs/releases/v0.16.1/organization-profile.patch` to
   `feedback-loop-ai/.github` (checking for any intervening changes after
   `c4e3df75ce68`) and verify the live organization page.
