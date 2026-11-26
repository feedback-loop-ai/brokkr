# v0.16.0 release handoff

[Release description](../v0.16.0.md).

## Version and range

- **Version:** 0.16.0, tag `v0.16.0`. The operator approved the release,
  the tag included, on 2026-11-25. Tagging, publication and channel
  updates are the operator's.
- **The name:** “Draupnir”, the gold ring Brokkr and his brother forged
  next after Gullinbursti, ruled by the operator on 2026-11-25. No lore
  saga is written for it.
- **Previous published release:** `v0.15.2`, tag object `645d2ce4`,
  commit `659aed44018ccd0d6dc9b9fbae1561c2bad92617`. GitHub's release
  API read it published on 2026-11-20T17:24:07Z, not a draft and not a
  prerelease, with eight platform archives and packages and
  `SHA256SUMS`, and as the latest release.
- **Base:** `b287b3f5524fff5a25b28587fac6988675f4e425`, main, which
  `git ls-remote` read as `refs/heads/main` during preparation. It is 79
  commits after `v0.15.2`. The range read is `v0.15.2..b287b3f5`.
- **Candidate:** the release branch on base `b287b3f5`. The release pull
  request records the final head and its check results.

## What the candidate changes

- **The version.** `Cargo.toml`'s workspace version and its six
  `brokkr-*` path dependencies move to 0.16.0. `cargo update --workspace
  --offline` moved only the eight workspace packages in `Cargo.lock`; no
  registry dependency moved.
- **The witnesses.** All 19 bundle digests in
  `crates/brokkr-runtime/tests/witnesses.json` moved, re-blessed with
  `BROKKR_BLESS=1 cargo test -p brokkr-runtime --test it
  witness_digests::` (CI unset for that command alone). A bundle's
  identity covers the engine version. At `b287b3f5`, with the old
  version, every witness held. With only the version changed, exactly
  the 19 bundle digests moved, and no policy table or charter digest did.
  Updating the example command in `recipes/release/README.md` to v0.16.0
  moved `recipes/release` once more; it was re-pinned with the same
  command, and no other witness moved.
- **The documentation**, audited below, and `unreleased.md` reset to
  count from v0.16.0.

## Documentation audit

| Target | Result | Evidence |
|---|---|---|
| `README.md` | changed | The release paragraph names v0.16.0 and keeps v0.15.2 as the previous release. The install table's "live from" labels are historical and unchanged. The bootstrap's `claude`, `codex` or `dsh` scaffold list still holds: `init` scaffolds those three (`crates/brokkr-cli/src/cli_args.rs:49`); agy and Qoder ship as example adapters. |
| `ARCHITECTURE.md` | changed | The diagram's harness node and the adapters sentence name Qoder CLI and Antigravity CLI, which are `AdapterKind`s (`crates/brokkr-protocol/src/adapters.rs:126`). No crate was added in the range, the corpus under `fixtures/` did not change, and its command spellings had moved already. |
| `CONTRIBUTING.md` | unchanged | No version reference and no flat command spelling. |
| `docs/guides/quickstart.md` | changed | `brokkr --version` and `brokkr doctor` samples read 0.16.0; `doc_samples` ran the binary. Completion steps were already there. |
| `docs/guides/versioning.md` | changed | Five current-version references. The contract table is held by `contract_lifecycle`. |
| `docs/guides/read-surfaces.md` | changed | One sentence names `brokkr system host upgrade` and `init`'s seeded host file (#1369), which no guide named. Decision 0125's demand rule was already there. |
| other `docs/guides/` | unchanged | No current-version reference and no flat spelling (grep of every moved root). `provider-adapters.md` covers agy and Qoder, and `dispatching.md` covers the named-ending and `merged` waits. `dsh-console.md`'s "a 0.15 release spells them" is decision 0113's ruling for the console, which keeps 0.15's spellings until 0.17. |
| `packaging/README.md` | unchanged | It already documents the Homebrew completer (#1308), and holds no current-version reference. |

Also changed: `recipes/release/README.md`'s sample command names v0.16.0,
as each release has. Left as they are:

- `flake.nix` keeps `0.15.2` and its digests. The digests catch up in their
  own pull request once the release artifacts exist, as #1209 did.
- `crates/brokkr-store/src/seat_record/v8_tests.rs` names `0.15.2` as a
  test's engine string, its subject.
- Every `docs/releases/` file before this one is a historical record.

Observed, not changed: the decision index reads 0114 `unbuilt` and 0125
`partial`, while slices 0114a to 0114d and 1193c (0125's last named slice)
are on main. The notes quote the index and say which slices landed.

## Checks on the candidate tree

Validation checks from the Release configuration:

| Check | Result |
|---|---|
| format: `cargo fmt --all -- --check` | passed |
| clippy: `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| tests: `cargo test --workspace --all-features --locked` | partial; full suite pending: runs in CI on the release pull request (`test (ubuntu-latest)` and `test (macos-latest)`). Local preparation verified the library suites, `witness_digests`, `layering`, `decisions_index`, `doc_truth`, `doc_samples` and the remaining integration targets except `heap_fleet`, `heap_seat_view`, `heap_run_view` and `fleet::kept_tests::a_refresh_reads_what_landed_since_the_last_one_and_not_the_journal`. |
| self bundle: `cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self` | passed: 48 valuations over 11 groups, 0 unruled |
| verify bundle: `cargo run --locked -p brokkr-cli -- library compile --bundle recipes/verify` | passed: 16 valuations over 5 groups, 0 unruled |
| exact coverage: `scripts/coverage-exact.sh` | pending: runs in CI on the release pull request (`exact coverage report` and `exact coverage gate`) |
| release binary: `cargo build --release --locked -p brokkr-cli` | passed; `brokkr --version` prints `brokkr 0.16.0` |
| remote CI | pending: runs on the release pull request. Required checks (`.github/branch-protection.json`): `delivered by brokkr`, `MSRV (1.89)`, `format, clippy, contracts`, `test (ubuntu-latest)`, `test (macos-latest)`, `exact coverage gate`, `dependency licenses (cargo-deny)`, `non-Rust lints`, `baseline ratchets`, `RustSec dependency audit`, `release binary artifact`, and `mutants in the diff: brokkr-core`. Other pull-request jobs in `ci.yml`: `docs lane or full (scripts/docs-lane.sh)`, `plugin tests (ubuntu-latest)`, `plugin tests (macos-latest)`, `test (<os>) on <machine>` (the Linux and macOS machine shards), `seatbelt lifetime probe (Gate B, reporting)`, `exact coverage on <machine>` (the coverage machine shards), `exact coverage report`, `bootstrap budgets (60s install / 5min first run, ubuntu-latest)`, `bootstrap budgets (60s install / 5min first run, macos-latest)`, `packaging channels`, and `nix flake check` (plus `docs lane (the tests that read documentation)` on docs-only pull requests, and `mutants in the diff: brokkr-protocol (report only)` in `mutants.yml`). |

Additional local checks passed on the candidate tree:
`git diff --check b287b3f5..HEAD -- . ':!docs/releases/v0.16.0/organization-profile.patch'`
(excluding `organization-profile.patch`, whose four single-space blank
context lines at lines 6, 16, 21 and 25 are required unified-diff syntax
that makes an unfiltered `git diff --check b287b3f5..HEAD` exit 2),
`quality/ratchet.sh files`, `quality/ratchet.sh clones`, `typos --hidden`,
offline `lychee`, and 5 of 6 lints in `scripts/lint-non-rust.sh --seat`.
`zizmor` is pending: runs in CI on the release pull request (`non-Rust lints`).
`quality/ratchet.sh api` and `quality/ratchet.sh crap` are pending: run in CI
on the release pull request (`exact coverage report`).

## Extensions

- **Workflow toolchain agreement: holds.** `ci.yml` (lines 458 and 545),
  `release.yml` (line 80, its exact release coverage job) and
  `scripts/coverage-exact.sh` (line 224) each read
  `rust-nightly-version.txt`, `nightly-2026-09-05`. No workflow names a
  nightly by hand. `cpu-budgets.yml`, new in the range, pins stable
  `1.98.0` for the Callgrind budgets, which is not a coverage job.
- **Frozen contracts: additive only.** `git diff v0.15.2..b287b3f5` over
  `contracts/`, `fixtures/`, `policy/phase-machine.json` and `reference/`
  adds `contracts/seat-record.v7.schema.json` (decision 0109),
  `seat-record.v8.schema.json` (decision 0075's addendum of 2026-11-21),
  `host.v5.schema.json` (decision 0125) and `effect-usage.v1.schema.json`
  (decision 0112). It modifies only `contracts/README.md`, the index:
  rows for the new versions, and the v1 row's pointer from v6 to v8 as
  the version that supersedes it for new runs. No frozen schema's bytes,
  no fixture, no policy table and nothing under `reference/` changed.
  Migration notes for the new versions are in the release description's
  Upgrade and compatibility.

## Organization profile

- **Repository:** `feedback-loop-ai/.github`, path `profile/README.md`,
  base commit `eee8c58a46279d2f2210f8121fbd768852894076` ("profile:
  brokkr's row names v0.15.2"), cloned into the ignored scratch
  directory `.forge/release/org-github`.
- **Patch:** [`organization-profile.patch`](organization-profile.patch).
  `git apply --check` passes on that base. It changes the three Brokkr
  references that move with this release:
  - the State cell names v0.16.0 and links its release;
  - the start-here line types `brokkr runs inspect`, since `brokkr
    inspect` is a hidden alias in 0.16 that 0.17 removes;
  - the vendor-agnostic sentence adds `agy` and a Google Gemini model.
- **Left as they are:** the quickstart and decision-record links, which
  point at `main`, and the neutrality line's "`dsh` beside `claude` in
  the engine", which still holds. The other projects' rows and the prose
  are untouched.
- **Apply after verified publication**, as the configuration says.

## After the tag

The operator's steps, in order, each pending until its result exists:

1. **Re-sign and open the release pull request** from this branch. It
   must not equal its base.
2. **Remote CI on the final head**: both operating systems' test jobs,
   MSRV, licenses, audit, packaging and exact coverage. The macOS shards'
   #1309 and #1374 are not blockers, by the operator's ruling of
   2026-11-25. Any other failure is.
3. **Merge**, then **tag `v0.16.0`** on the merge commit. The release
   workflow's admission checks that the tag matches the workspace
   version.
4. **The GitHub release**: four platform archives, the Linux deb and rpm
   packages, `SHA256SUMS` and signed build provenance. Paste the release
   description.
5. **crates.io**: all seven workspace crates serve 0.16.0.
6. **apt and rpm**: the signed Pages repositories index the 0.16.0
   packages.
7. **Homebrew and Nix**: merge the generated channel pull requests and
   check that their versions and digests match the release's
   `SHA256SUMS`. The flake's digests move from 0.15.2 here.
8. **The organization profile**: once publication is verified, apply
   `organization-profile.patch` to `feedback-loop-ai/.github`, and check
   the live page.
