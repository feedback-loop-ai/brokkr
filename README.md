<p align="center"><img src="assets/logo.svg" width="132" alt="Brokkr — sealed anvil mark, terminal rail node pulsing"></p>

# Brokkr

**Coordination tools help agents work together; Brokkr proves what they did.**

Brokkr is a deterministic delivery engine for autonomous multi-agent
software work: it drives real agent sessions through reviewable recipes,
accepts only typed results, lets a pinned policy—not a model—rule every
transition, and records the run as a hash-chained journal you can replay
and audit.

## Install

Pick one channel. The release tarball works today; rows marked **wired
at the bench** need the operator-side publication step named in the
[packaging guide](packaging/README.md).

| Channel | One command | State |
|---|---|---|
| release tarball | `tar xzf brokkr-linux-x86_64.tar.gz` | [latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest); download the matching archive first |
| cargo | `cargo binstall brokkr-cli` | **wired at the bench** — waits on crates.io publication |
| nix | `nix profile install github:feedback-loop-ai/brokkr` | installs after the release's flake-digest update lands |
| apt | `sudo apt-get update && sudo apt-get install brokkr` | **wired at the bench** — add the signed repository first |
| dnf | `sudo dnf install brokkr` | **wired at the bench** — add the signed repository first |
| Homebrew | `brew install feedback-loop-ai/tap/brokkr` | **wired at the bench** — waits on the tap token |
| Scoop | `scoop bucket add brokkr https://github.com/feedback-loop-ai/scoop-bucket && scoop install brokkr` | **wired at the bench** — waits on the bucket token |

The [quickstart install step](docs/guides/quickstart.md#step-1--install)
has platform names, checksum and attestation verification, repository
setup, and the source-build fallback.

## Sixty-second quickstart

From this checkout, with Claude Code or Codex available, light one real
run with one command:

```console
$ brokkr run --recipe fast --repo . --feature "add one small behavior and the test that proves it"
run started: add-one-small-behavior-and-the-tes-8bf6d692
…
```

When it finishes, put the proof on screen:

```console
$ brokkr inspect --run latest
run  add-one-small-behavior-and-the-tes-8bf6d692
     completed · phase done · seq 38
ruling  SHIP-COMPLETE  ship → done · shipped

seats
  participant status    attempts turns cost activity
  implement   succeeded 1        24    —    complete
  verify      succeeded 1        8     —    pass
  review      succeeded 1        11    —    clean
  ship        succeeded 1        5     —    shipped

trail
  28 effect/succeeded   review · clean
  29 transition/decided REVIEW-CLEAN-NO-FIXES review → ship · clean
  38 run/completed      completed

graph
  implement ×1
  verify ×1
  review ×1
  ship ×1
  done ×1  ←current
```

That screen is the journal, the phase graph, and the reviewer's verdict.
The [full quickstart](docs/guides/quickstart.md) covers scaffolding a
recipe for another repository, exit codes, parks and the exact evidence
behind each line.

## Read next

- [Guides](docs/guides/README.md) — Install, run, operate and extend Brokkr without turning the front page into a manual.
- [Decision record](docs/decisions/README.md) — The numbered operator rulings are the constitution the engine cites and enforces.
- [Essays](docs/essays/README.md) — The architecture's claims are argued against this repository's decisions, evidence and history.
- [Lore](docs/lore/edda.md) — The Edda makes the name memorable while keeping myth commentary, never specification.
- [Evidence shelf](docs/evidence/README.md) — Canonical and publishably redacted journals let the machine's claims be checked.
- [Contributing](CONTRIBUTING.md) — The complete path from clone to a green contribution lives in one place.
- [Architecture](ARCHITECTURE.md) — Crates, journal, effect discipline and verification layers, from the pure core outward.

## Acknowledgments

The standing-overseer concept reached this product by way of the
lieutenant in Robert C. Martin's
[SwarmForge](https://github.com/unclebob/swarm-forge). The idea is
credited here and nothing else is taken: SwarmForge carries no license,
which means all rights reserved, so no code, scripts, prompts or prose
from it has entered — or may enter — this tree.

**Muninn** is an independent design with an inverted authority model,
described in decision 0020 and built as `brokkr muninn`: it reads the
journal, proposes to the operator, and rules nothing.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option; [decision 0018](docs/decisions/0018-dual-license.md) records why.
