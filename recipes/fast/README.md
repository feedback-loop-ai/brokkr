# Fast

The default Rust delivery recipe. Its verifier runs, cheapest first,
every check CI requires that works offline and within its declared
hands (#427):
`cargo fmt --all -- --check`, the non-Rust lints of
`scripts/lint-non-rust.sh` (typos, shellcheck, actionlint, zizmor
`--offline`, lychee `--offline` — the one list CI's `non-Rust lints` job
runs too), `cargo clippy --workspace --all-targets --all-features
--locked -- -D warnings`, `cargo test --workspace`, and the
`recipes/self` compile. Its shipper renders the journal with `brokkr
ledger`. Both are deterministic exec gates, and every recipe that
extends `fast` without its own verifier, `landing` among them, runs this
one. Brokkr boxes both, with no network, where the realm's boundary is
`namespace`; under `harness`, which `brokkr init` writes on macOS, each
runs unboxed in a rebuilt environment and no network denial is reported.

Its two model seats hire library offices (the operator's ruling of
2026-10-07, #360). `implement` seats `fast-implementer`, an overlay of
`implementer` on the intakeless implementer charter, and `review` seats
`fast-reviewer`, an overlay of `reviewer` that declares the review's
inputs. Neither writes a chain, tools or hands, so each hires its base's
(decision 0087 rulings 2 and 3): in a folder `brokkr init` made, what
`init` wrote for the agent CLI it found. Brokkr's own runs hire fast's
Brokkr picks through [`brokkr-fast`](../brokkr-fast/README.md), which
`landing` and the wager harnesses extend.

**Outside Brokkr, verify fails for now.** `fast` installs onto any
folder's shelf (`brokkr recipes add <checkout>/recipes/fast --name
fast`), but its verifier and shipper are still Brokkr's own: the verifier
runs `scripts/lint-non-rust.sh` and the `recipes/self` compile under the
Rust binds, and the shipper's notes name `.forge/ledger/`. So in another
repository a `fast` run reaches implement and review, and its verify
fails. Decision 0087's slice 3 gives `fast` a portable verifier and moves
this one to `brokkr-fast`.

A lint tool the verifier cannot reach, because it is not on the seat's
`PATH` or does not report the version CI pins, is named in the verifier's
notes as not run; the result is `pass` only if every check that did run
passed. The seat's hands bind `~/.cargo`, so a tool installed there at
CI's pin is reached. The diagram render (Node and `npm ci`) and Renovate's
validator (docker) need the network and stay CI-only; the exact-coverage
gate, the licence check and the other operating system are CI's as well.

Cargo runs offline from the bound registry cache because the verifier
sets `CARGO_NET_OFFLINE`. If a dependency is not cached, the command
fails closed, and the verifier's `fail` notes quote Cargo's decisive
cache or offline error.
