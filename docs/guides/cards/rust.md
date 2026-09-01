# Card — Rust

**Read [quickstart.md](../quickstart.md) first.** This is a delta over
its four-step spine. Steps 1, 3 and 4 are unchanged.

## Step 2 — what `init` reads and writes

`Cargo.toml` at the root. One marker, no lockfile tiebreaker: cargo is
the only toolchain, and `Cargo.lock` changes nothing about which
commands are right.

```
    cargo build --workspace                              # implementer
    cargo test --workspace                               # implementer + verifier
    cargo clippy --workspace --all-targets -- -D warnings # verifier
```

- **`-D warnings` is deliberate.** A lint that warns and exits 0 gives
  a gate seat nothing to fail on.
- **A `[workspace]` table adds a sentence, not a command.** `--workspace`
  was already spanning every member crate, so what a workspace root gets
  is the charter *saying* it is one — so a seat does not go hunting for
  a per-crate invocation nobody needed. A plain `[package]` manifest
  gets the same two commands and no such sentence.
- **`Cargo.toml` outranks a `Makefile`** that happens to sit beside it:
  a repository carrying both usually wraps the one in the other, and
  `make lint` in a repo whose Makefile has no `lint` target is exactly
  the dishonest command this detection exists to stop writing.

The full transcript, annotated:
[starters/rust.md](../starters/rust.md).

## Step 3 — which recipe

The scaffold from step 2 is the right starting point; there is no
maintained `recipes/rust`. This repository's own
[`bundles/self`](../../../bundles/self/) is the closest worked example
of a Rust delivery strategy, and `brokkr recipes list` shows it beside
the library.

Raise the seat `limits` in `bundle.json` if a cold build is long — the
scaffold's 5400s implement timeout assumes a warm target directory.
