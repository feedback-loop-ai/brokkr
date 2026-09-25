This realm is a Rust workspace. Production code is Rust under `crates/`; the
Rust-only architecture is recorded by decision 0009. Semantic changes carry a
decision document with status `proposed` because only the operator accepts one.
Intake seats read `README.md` and decisions 0004 and 0005 before framing a
commission, along with the code and history it touches.

The frozen v1 contracts under `contracts/`, the production table at
`policy/phase-machine.json`, and `reference/` (its schemas included) are
read-only.
A contract change lands as a new version file beside the old one, never as an
edit. The evaluator corpus under `fixtures/` is also frozen and is never
regenerated, only versioned.

Tests are part of every change. Extend the suite that proves the code, run
`cargo test --workspace`, compile `bundles/self`, and leave formatting, clippy,
and the exact-coverage gate clean before reporting success.

Commit completed work with `git` using the repository's message style. Never
push.

The supported hosts are Linux and macOS (decision 0063); WSL2 is Linux. Write
no Windows-conditional code, test or evidence.

## Architecture

Decision 0071 rules how code here is shaped. Build to it, and cite its ruling
numbers in what you judge. Where a gate named below holds a rule, the gate is
the authority, and a finding it would catch is the gate's.

- **Pure core (ruling 1).** `brokkr-core` and `brokkr-view` do no I/O and read
  no clock, environment or randomness. They run no process. Pass `now`, paths
  and environment in as arguments. Effects live in store, runtime, protocol
  and cli, and are journaled.
- **Enums and data, not traits (ruling 2).** A closed set is an enum matched
  exhaustively, with no wildcard arm that decides behaviour. Anything an
  operator extends is data under `adapters/`, `agents/`, `recipes/` or
  `realms.json`, validated and digested. Add a trait only for a seam with more
  than one real implementation, and name that seam. Otherwise invert a
  dependency with a closure (`*_with`, `*_in`).
- **Typed at the edge (ruling 3).** Parse wire, file and harness data once
  into a type, with `serde` and `deny_unknown_fields` where the contract is
  closed. `serde_json::Value` never goes past the edge, into state, or back
  out by string key. Never put `format!("{:?}")` in anything digested or
  journaled. A doubly-optional field or a string vocabulary is a missing enum.
- **Small functions (ruling 4).** New and changed functions stay within 100
  lines, nesting depth 5 and 7 parameters. Production files stay within 800
  lines. Existing offenders carry `#[expect(clippy::…, reason = "…")]`. Add
  one only by a ruling, and remove it when you fix the function. Suppress with
  `#[expect(…, reason)]`, never `#[allow]`. These numbers are provisional until
  the operator rules them from the measured baseline.
- **Once (ruling 5).** Every fact has one home. A repeated vocabulary, skeleton
  or helper becomes one function or one table. An agent is defined once under
  `agents/`, and seats hire it by reference. A recipe that differs from another
  by a model, a seat or an entry is an `extends` overlay, never a copy. An
  agent has no `extends`, since the library refuses the key. A variation of an
  agent lives at the seat that hires it, overridden in a recipe overlay, and
  never in a second agent file. A copy that must exist, such as a script a
  bundle pins, is held to its source by a test.
- **Nothing unused (ruling 6).** Add no alias, agent, contract version, flag or
  `pub` item without a consumer, and prefer `pub(crate)`. A deprecation names
  the release that removes it.
- **One derivation (ruling 7).** A fact shown on two surfaces is derived once
  in `brokkr-view`. The CLI, the TUI and `ui.html` only paint it. The engine
  never admits anything by a display-crate function.
- **Typed errors (ruling 8).** Return a `thiserror` enum whose `Display` keeps
  the operator's text. Tests match variants and pin each module's text once.
  Add no new `Result<_, String>`.
- **Tests that bind (ruling 9).** Assert exact values or variants, never
  `is_err()` alone. Show that every new test binds: a compiling mutation that
  removes the behaviour must make it fail. Restore the behaviour and record
  the failing test. Share fixtures through builders. Never change the process
  environment without a guard that restores it on unwind.
- **The house's patterns (ruling 10).**
  - A harness is a module behind `AdapterKind`.
  - A CLI verb is a `Cmd` variant plus a handler.
  - A long parameter list becomes a parameter object.
  - Argv and mount tables are built with a builder.
  - Observing the world is split from deciding on it.
  - Never use a trait-object registry for a closed set.

Judges rate a finding against these rulings by this table, and cite the ruling
number:

- **info:** something a house gate would catch. The gate's verdict is the
  authority.
- **low:** a new or changed function over a ceiling, a rule written a second
  time, an item added without a consumer, or a `Value` carried past its edge
  within one module.
- **medium:** an untyped vocabulary that crosses crates, recipe or agent data
  copied instead of extended, a fact derived a second time on another surface,
  an error matched by its text, or a new test that cannot fail.
- **high or above:** any violation that hides a fail-open path or weakens a
  refusal. It is carried as a security finding.

A principle the implementer bent knowingly, and named with its reason in
`notes`, is still rated, and the judge carries that reason with the finding.

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
    {"name": "exact coverage", "instructions": "Run bash scripts/coverage-exact.sh in CI or host validation outside the workspace box before tagging. Boundary tests require creating a namespace, which the box deliberately refuses to nest. Record this check as pending in the preparation handoff until that external result exists; never lower the coverage gate."},
    {"name": "release binary", "command": "cargo build --release --locked -p brokkr-cli"},
    {"name": "remote CI", "instructions": "Before tagging, require all applicable CI jobs on the final candidate commit, including both supported operating systems, MSRV, licenses, audit, packaging and exact coverage."}
  ],
  "organization_profiles": [
    {"repository": "feedback-loop-ai/.github", "path": "profile/README.md", "project_url": "https://github.com/feedback-loop-ai/brokkr", "apply": "after verified publication"}
  ],
  "channels": [
    {"name": "GitHub release", "evidence": "Four platform archives, Linux deb/rpm packages, SHA256SUMS and signed build provenance."},
    {"name": "crates.io", "evidence": "All seven workspace crates serve the new version."},
    {"name": "apt and rpm", "evidence": "The signed Pages repositories index the new packages."},
    {"name": "Homebrew and Nix", "evidence": "The generated channel PRs are merged and their versions and digests match the release manifest."}
  ],
  "extensions": [
    {"name": "workflow toolchain agreement", "instructions": "Check that CI, release admission and local coverage select the same pinned compiler.", "evidence": "Both workflows and coverage-exact.sh consume rust-nightly-version.txt."},
    {"name": "frozen contracts", "instructions": "Compare contracts, fixtures, policy/phase-machine.json and reference against the previous release; explain additive versions and flag modifications to frozen bytes.", "evidence": "A reviewed diff with migration notes for any new contract versions."}
  ]
}
```
