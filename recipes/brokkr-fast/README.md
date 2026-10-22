# brokkr-fast — fast with Brokkr's picks

Brokkr-only (decision 0087 rulings 4 and 5). `fast` with its two model
seats re-seated on Brokkr's own offices: `implement` on
`brokkr-fast-implementer` and `review` on `brokkr-fast-reviewer`. Each is
an overlay of the portable base that restates what `fast-implementer` and
`fast-reviewer` hired before the base went portable: Fable at `high`
first, then Opus at `high` (the smith then Sonnet), under the `cargo`,
`git`, `ls`, `rg` and `mkdir` allow-list, unboxed. When Fable cannot
start, its limit spent, a seat falls back to the next link instead of
parking.

It carries Brokkr's verifier, `scripts/verify-seat.sh` (decision 0091
ruling 3): its verify seat is its own, never the folder's. The verifier
runs, cheapest first, every check CI requires that works offline and
within its declared hands (#427): `cargo fmt --all -- --check`, the
non-Rust lints of `scripts/lint-non-rust.sh` (typos, shellcheck,
actionlint, zizmor `--offline`, lychee `--offline` — the one list CI's
`non-Rust lints` job runs too), `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings`, `cargo test --workspace`, and
the `recipes/self` compile, under the `~/.cargo` and `~/.rustup` binds.

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

The table, the shipper and every limit are `fast`'s, inherited.
`landing`, `wager-harness`, `wager-harness-dsh` and `wager-harness-muse`
extend this recipe, because each inherits at least one of fast's model
seats. `panel-review`, `triage`, `release`, `review-first` and `standby`
extend it for its verifier (decision 0091 ruling 3), and `self`,
`gpt-flash` and `night-shift` reach it through them.
