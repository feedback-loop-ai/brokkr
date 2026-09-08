This realm is a Rust workspace. Production code is Rust under `crates/`; the
Rust-only architecture is recorded by decision 0009. Semantic changes carry a
decision document with status `proposed` because only the operator accepts one.
Intake seats read `README.md` and decisions 0004 and 0005 before framing a
commission, along with the code and history it touches.

The frozen v1 contracts under `contracts/`, the production table at
`policy/phase-machine.json`, `policy/schemas/`, and `reference/` are read-only.
A contract change lands as a new version file beside the old one, never as an
edit. The evaluator corpus under `fixtures/` is also frozen and is never
regenerated, only versioned.

Tests are part of every change. Extend the suite that proves the code, run
`cargo test --workspace`, compile `bundles/self`, and leave formatting, clippy,
and the exact-coverage gate clean before reporting success.

Commit completed work with `git` using the repository's message style. Never
push.

## Release configuration

The release manager prepares the candidate and profile patches; the operator
publishes them. Local validation is preparation work. Remote CI on the final PR
head, publication, channel updates and live-profile verification are handoff
steps, recorded as pending until their remote results exist. Historical
"live from" channel versions and old journal examples are historical facts,
not current-version references to replace.

```json
{
  "repository": "feedback-loop-ai/brokkr",
  "default_branch": "main",
  "tag_format": "v{version}",
  "version_files": ["Cargo.toml", "Cargo.lock"],
  "version_update": "Bump the workspace package and six path-dependency versions together. Run cargo update --workspace --offline without upgrading registry dependencies. Measure and update the witness and compose manifest digests because the engine version participates in their identity.",
  "release_notes": "docs/releases/v{version}.md",
  "handoff_directory": "docs/releases/v{version}",
  "scratch_directory": ".forge/release",
  "documentation": ["README.md", "ARCHITECTURE.md", "CONTRIBUTING.md", "docs/guides/", "packaging/README.md"],
  "validation": [
    {"name": "format", "command": "cargo fmt --all -- --check"},
    {"name": "clippy", "command": "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"},
    {"name": "tests", "command": "cargo test --workspace --all-features --locked"},
    {"name": "self bundle", "command": "cargo run --locked -p brokkr-cli -- compile --bundle bundles/self"},
    {"name": "verify bundle", "command": "cargo run --locked -p brokkr-cli -- compile --bundle bundles/verify"},
    {"name": "exact coverage", "command": "bash scripts/coverage-exact.sh"},
    {"name": "release binary", "command": "cargo build --release --locked -p brokkr-cli"},
    {"name": "remote CI", "instructions": "Before tagging, require all applicable CI jobs on the final candidate commit, including all three operating systems, MSRV, licenses, audit, packaging and exact coverage."}
  ],
  "organization_profiles": [
    {"repository": "feedback-loop-ai/.github", "path": "profile/README.md", "project_url": "https://github.com/feedback-loop-ai/brokkr", "apply": "after verified publication"}
  ],
  "channels": [
    {"name": "GitHub release", "evidence": "Five platform archives, Linux deb/rpm packages, SHA256SUMS and signed build provenance."},
    {"name": "crates.io", "evidence": "All seven workspace crates serve the new version."},
    {"name": "apt and rpm", "evidence": "The signed Pages repositories index the new packages."},
    {"name": "Homebrew, Scoop and Nix", "evidence": "The generated channel PRs are merged and their versions and digests match the release manifest."}
  ],
  "extensions": [
    {"name": "workflow toolchain agreement", "instructions": "Check that CI, release admission and local coverage select the same pinned compiler.", "evidence": "Both workflows and coverage-exact.sh consume rust-nightly-version.txt."},
    {"name": "frozen contracts", "instructions": "Compare contracts, fixtures, policy/phase-machine.json and reference against the previous release; explain additive versions and flag modifications to frozen bytes.", "evidence": "A reviewed diff with migration notes for any new contract versions."}
  ]
}
```
