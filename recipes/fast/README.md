# Fast

The default Rust delivery recipe. Its verifier runs, cheapest first,
every check CI requires that works offline and inside the box (#427):
`cargo fmt --all -- --check`, the non-Rust lints of
`scripts/lint-non-rust.sh` (typos, shellcheck, actionlint, zizmor
`--offline`, lychee `--offline` — the one list CI's `non-Rust lints` job
runs too), `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings`, `cargo test --workspace`, and the
`bundles/self` compile. Its shipper renders the journal with `brokkr
ledger`. Both are deterministic boxed exec gates, and every recipe that
extends `fast` without its own verifier, `landing` among them, runs this
one.

A lint tool the box cannot reach, because it is not on the box's `PATH`
or does not report the version CI pins, is named in the verifier's notes
as not run; the result is `pass` only if every check that did run passed.
The box binds `~/.cargo`, so a tool installed there at CI's pin is
reached. The diagram render (Node and `npm ci`) and Renovate's validator
(docker) need the network and stay CI-only; the exact-coverage gate, the
licence check and the other operating system are CI's as well.

Cargo runs offline inside the box from the bound registry cache. If a
dependency is not cached, network remains refused, the command fails
closed, and the verifier's `fail` notes quote Cargo's decisive cache or
offline error.
