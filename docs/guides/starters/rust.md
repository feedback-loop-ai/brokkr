# Starter sample — Rust

What `brokkr init` actually wrote for a Rust repository, transcribed
from a real run. Nothing on this page was composed by hand: the fixture
below was copied into a scratch directory, `brokkr init my-bundle` was
run inside it, and the output was pasted here and then annotated.

The fixture is
[`crates/brokkr-cli/tests/fixtures/init-stacks/rust/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/rust/),
the same one
[`init_stacks.rs`](../../../crates/brokkr-cli/tests/init_stacks.rs)
asserts against — so if these commands ever stop being what `init`
writes, a test goes red before this page goes stale.

## The repository init read

```
./Cargo.toml
```

```toml
# Marker only. `brokkr init` reads that this file is here and nothing
# else; `[workspace]` keeps it out of the workspace it happens to sit in.
[package]
name = "stack-fixture"
version = "0.0.0"
edition = "2021"

[workspace]
```

## The invocation

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 4f5d5e9ae9e13af46f6fdc52e416c89aa43de28c68daa3ac570c4cad44231ef2)
run brokkr from inside my-bundle — its adapters/ declares the trust tier the verify, review and ship seats judge on
```

The digest is a function of the bytes that were written, and the
charters below vary by stack, so **this digest is this fixture's**. Your
Rust repository will print a different one and that is correct.

## What it wrote

```
my-bundle/adapters/claude.json
my-bundle/bundle.json
my-bundle/policy.json
my-bundle/roles/implementer.md
my-bundle/roles/intake.md
my-bundle/roles/reviewer.md
my-bundle/roles/shipper.md
my-bundle/roles/verifier.md
```

**`bundle.json`, `policy.json`, `adapters/claude.json` and three of the
five charters do not vary by stack.** No interpolation happens into
them: the seat roster, the phase table, the trust declaration, and the
intake / reviewer / shipper charters are byte-identical for every
repository `init` has ever been run in. Framing a task, reading a diff
and closing out read the same in Rust as they do in Go.

For completeness, the invariant `bundle.json`:

```json
{
  "name": "starter",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": {
    "intake": {
      "role": "roles/intake.md",
      "class": "work",
      "results": ["resolved"],
      "limits": {"max_attempts": 2, "timeout_seconds": 1800},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "implement": {
      "role": "roles/implementer.md",
      "class": "work",
      "results": ["complete", "broken", "blocked"],
      "limits": {"max_attempts": 2, "timeout_seconds": 5400},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "verify": {
      "role": "roles/verifier.md",
      "class": "gate",
      "results": ["pass", "fail"],
      "limits": {"max_attempts": 2, "timeout_seconds": 3600},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "review": {
      "role": "roles/reviewer.md",
      "class": "gate",
      "results": ["clean", "residual", "security-hold"],
      "limits": {"max_attempts": 2, "timeout_seconds": 3600},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    },
    "ship": {
      "role": "roles/shipper.md",
      "class": "gate",
      "results": ["ready", "shipped"],
      "limits": {"max_attempts": 2, "timeout_seconds": 1800},
      "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--permission-mode", "acceptEdits"]}
    }
  }
}
```

Every other page in this directory omits it. The stack-specific content
lives entirely in the two charters below.

## `roles/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.forge/tasks/`). Match the project's
idiom. Tests are part of the change.

This repository reads as a rust project (`Cargo.toml`), so use its own
tooling:

    cargo build --workspace
    cargo test --workspace

This is a CARGO WORKSPACE (`[workspace]` in `Cargo.toml`). The
`--workspace` flag above already spans every member crate — there
is no per-crate command to go looking for.

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

Build and test before declaring anything. Commit your work; never push.

Result: `complete` (implemented, tests green, committed) · `broken`
(could not get it working — name the specific gap in `notes`) ·
`blocked` (something outside your control — name it precisely). Never
report `complete` with failing tests or uncommitted changes.
```

Line by line, the parts that were chosen rather than fixed:

- **`a rust project (`Cargo.toml`)`** — the name comes from the `rust`
  arm of the detection table, the parenthesis is the evidence quoted
  back. One marker, so one file named. `Cargo.toml` is the first entry
  in the table: a language manifest outranks the `Makefile` catch-all,
  because a repo carrying both usually wraps the one in the other.
- **`cargo build --workspace` / `cargo test --workspace`** — the
  implementer gets the arm's `build` and `test`. (The verifier gets
  `test` and `lint`; nobody gets all three, because the seat that builds
  and the seat that proves are asked for different things.)
- **the CARGO WORKSPACE paragraph** — this fixture's `Cargo.toml`
  carries a `[workspace]` table, so `init` says so. It adds no command:
  `--workspace` was already spanning every member crate. What was
  missing was the charter *saying* it, so a seat does not go hunting for
  a per-crate invocation nobody needed. A plain `[package]` manifest
  with no `[workspace]` table gets these same two commands and **no**
  such paragraph — proven by the `rust-package` fixture, whose manifest
  mentions `[workspace]` only inside a comment and is correctly not
  called one.
- **"it ran nothing to find out"** — literally true. Detection is file
  presence and, for the workspace line, one `[workspace]` line read out
  of `Cargo.toml`. No `cargo` subprocess is spawned.

## `roles/verifier.md`

```markdown
# Verifier seat — prove it, fix nothing

Run the project's full test and lint suites from the repository root.

This repository reads as a rust project (`Cargo.toml`), so use its own
tooling:

    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings

This is a CARGO WORKSPACE (`[workspace]` in `Cargo.toml`). The
`--workspace` flag above already spans every member crate — there
is no per-crate command to go looking for.

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

You change no code, fix nothing, commit nothing: one honest run is the
signal. Result: `pass` (everything green; `notes` lists commands and
counts) or `fail` (`notes` quotes the failing output's decisive lines
exactly — never soften a failure).
```

- **`cargo test --workspace`** — the same command the implementer was
  given, on purpose. The implementer runs it to know it is done; the
  verifier runs it as the honest, independent signal.
- **`cargo clippy --workspace --all-targets -- -D warnings`** — the
  arm's `lint`. `-D warnings` is deliberate: a lint that warns and
  exits 0 gives a gate seat nothing to fail on.

## Correcting it

The charter is ordinary Markdown in your tree. If your suite is
`cargo nextest run` or your lint carries `--all-features`, edit these
two files — that is what the "correct them here if they are wrong" line
is inviting. Every edit moves the bundle's digest, which is the point: a
strategy is identified by its bytes, and a run records which bytes it
ran.

## See also

- [quickstart.md](../quickstart.md) — the four-step spine these files
  drop into.
- [cards/rust.md](../cards/rust.md) — the quickstart's Rust delta.
- [recipe-authoring.md](../recipe-authoring.md) — what `bundle.json`
  and `policy.json` mean once you want to change them.
