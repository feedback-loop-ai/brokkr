This realm is a Rust workspace. Production code is Rust under `crates/`; the
Rust-only architecture is recorded by decision 0009. Semantic changes carry a
decision document with status `proposed` because only the operator accepts one.

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
