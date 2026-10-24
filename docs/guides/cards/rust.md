# Card — Rust

**Read [quickstart.md](../quickstart.md) first.** This is a delta over
its four-step spine. Steps 1, 3 and 4 are unchanged.

## Step 2 — what `init` reads and writes

`Cargo.toml` at the root. One marker, no lockfile tiebreaker: cargo is
the only toolchain, and `Cargo.lock` changes nothing about which
commands are right.

```
    cargo build --workspace                                       # implementer
    cargo test --workspace --locked                               # implementer + verifier
    cargo clippy --workspace --all-targets --locked -- -D warnings # verifier
```

- **`--locked` is deliberate.** The verifier refuses a lockfile that
  would have to be generated or changed, and the implementer charter
  says to commit `Cargo.lock` whenever the change creates or changes it,
  so the verifier resolves the versions the change was tested with.

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
[`recipes/self`](../../../recipes/self/) is the closest worked example
of a Rust delivery strategy, and `brokkr recipes list` shows it in the
library. It is Brokkr-only
([decision 0087](../../decisions/0087-the-base-offices-are-the-ones-init-writes.md)
ruling 5): an example to read, not one to install onto your folder's
shelf.

Raise an agent's `limits` in `.brokkr/agents/implementer.json` if a cold build is long — the
scaffold's 5400s implement timeout assumes a warm target directory.
