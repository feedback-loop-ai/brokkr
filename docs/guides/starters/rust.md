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
initialized reviewable bundle at my-bundle (digest d4b6f758d2014a3726a6e9e798fdd3c8aae682d0ada499da536806b38b2f9c52)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

The digest is a function of the bytes that were written, and the
charters and tool map below vary by stack, so **this digest is this
fixture's**. Your Rust repository will print a different one and that is
correct.

## What it wrote

```
my-bundle/README.md
my-bundle/adapters/claude.json
my-bundle/agents/charters/implementer.md
my-bundle/agents/charters/intake.md
my-bundle/agents/charters/reviewer.md
my-bundle/agents/charters/shipper.md
my-bundle/agents/charters/verifier.md
my-bundle/agents/implementer.json
my-bundle/agents/intake.json
my-bundle/agents/reviewer.json
my-bundle/agents/shipper.json
my-bundle/agents/verifier.json
my-bundle/bundle.json
my-bundle/policy.json
```

**`bundle.json`, `policy.json`, the intake / reviewer / shipper charters
and the fixed parts of every agent file do not vary by stack.** The seat
roster, the phase table, the three charters that frame, read and close
out, and each agent's charter link, model chain and limits are
byte-identical for every repository `init` has ever been run in. What
varies is the stack's own data: the implementer's and verifier's
charters, the adapter's tool map, and the tool allowance each agent
carries.

For completeness, the invariant `bundle.json`:

```json
{
  "name": "starter",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": {
    "intake": {
      "agent": "intake",
      "class": "work",
      "results": ["resolved"]
    },
    "implement": {
      "agent": "implementer",
      "class": "work",
      "results": ["complete", "broken", "blocked"]
    },
    "verify": {
      "agent": "verifier",
      "class": "gate",
      "results": ["pass", "fail"]
    },
    "review": {
      "agent": "reviewer",
      "class": "gate",
      "results": ["clean", "residual", "security-hold"]
    },
    "ship": {
      "agent": "shipper",
      "class": "gate",
      "results": ["ready", "shipped"]
    }
  }
}
```

Every seat names an agent and nothing else about what the agent IS: an
agent reference is total, so the charter, the model chain, the 0006
`limits` and the tool grant all live in the agent's own file under
`agents/`, where `brokkr agents show <name>` reads them back. The class
written here is the seat's authority, never the agent's (decision 0021
ruling 1) — which is why the scaffold's tests cross-check the two.

Every other page in this directory omits this file and points here.

## The tool grant

The scaffold's claude adapter maps the binaries the charters' commands
invoke — and nothing broader — as `Bash(<bin>:*)` entries under
`tool_permissions.names`. For this stack:

```json
{
  "tool_permissions": {
    "flag": "--allowedTools",
    "separator": ",",
    "names": {
      "cargo": "Bash(cargo:*)",
      "git": "Bash(git:*)",
      "ls": "Bash(ls:*)",
      "mkdir": "Bash(mkdir:*)",
      "rg": "Bash(rg:*)"
    }
  }
}
```

The same names, in the same order, are each agent's `tools.allow` —
sized by the class of the seat the agent backs:

- **work seats (`intake`, `implement`)** — the whole set:
  `["cargo", "git", "ls", "rg", "mkdir"]`. A work seat may run exactly
  the commands its charter names, and nothing broader.
- **gate seats (`verify`, `review`, `ship`)** — the read-only subset:
  `["cargo", "git", "ls", "rg"]`, never `mkdir`.

The grant is per BINARY, not per subcommand: `Bash(cargo:*)` answers to
`cargo build` as readily as to `cargo test`. What keeps a gate from
building is its charter — "prove it, fix nothing" — and the scaffold
README says so rather than promising a boundary the glob cannot draw.
An allowance is ONE grant with the adapter's map: a name the map cannot
express refuses the scaffold's own compile (decision 0016), so when you
edit one side, edit both.

## `agents/charters/implementer.md`

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

## `agents/charters/verifier.md`

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

## One agent file, whole

The implementer's definition, as written:

```json
{
  "description": "Builds the framed task to the repository's conventions and commits the work with its tests.",
  "charter": "charters/implementer.md",
  "limits": {
    "max_attempts": 2,
    "timeout_seconds": 5400
  },
  "models": ["opus", "sonnet"],
  "tools": {
    "allow": ["cargo", "git", "ls", "rg", "mkdir"],
    "mcp": []
  }
}
```

The bounds are the ones the starter used to declare inline on the seat
(decision 0006); since the seats name agents, they ride here. The model
chains are the repository's own library's — work leads with the stronger
model where the ship does, and every chain is a preference, not a
promise.

## Correcting it

All of it is ordinary JSON or Markdown in your tree. If your suite is
`cargo nextest run` or your lint carries `--all-features`, edit
`agents/charters/implementer.md` and `agents/charters/verifier.md`; if a
seat needs a narrower or wider grant, edit its `tools.allow` in
`agents/<name>.json` AND the matching entry in the adapter's map — the
scaffold's own compile is the check that the two still agree. Every edit
moves the bundle's digest, which is the point: a strategy is identified
by its bytes, and a run records which bytes it ran.

## What the fallback looks like

A repository `init` does not recognize — no manifest, no lockfile, no
`Makefile` — gets the same fourteen files with an **empty** tool map
(`"names": {}`), no `tools` restriction on any agent, and a README that
says so in those words rather than granting a guessed permission. The
two charters carry `<this project's …>` placeholders. See
[quickstart.md](../quickstart.md#step-2--brokkr-init-) for the shape.

## See also

- [quickstart.md](../quickstart.md) — the four-step spine these files
  drop into.
- [cards/rust.md](../cards/rust.md) — the quickstart's Rust delta.
- [recipe-authoring.md](../recipe-authoring.md) — what `bundle.json`
  and `policy.json` mean once you want to change them.
