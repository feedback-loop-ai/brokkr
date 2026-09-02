# Starter sample — Rust

What `brokkr init` actually wrote for a Rust repository, transcribed
from a real run. Nothing on this page was composed by hand: the fixture
below was copied into a scratch directory, `brokkr init my-bundle` was
run inside it, and the output was pasted here and then annotated.

The fixture is
[`crates/brokkr-cli/tests/fixtures/init-stacks/rust/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/rust/),
the same one
[`init_stacks.rs`](../../../crates/brokkr-cli/tests/init_stacks.rs)
asserts against — so if these commands or tool grants ever stop being
what `init` writes, a test goes red before this page goes stale.

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
initialized reviewable bundle at my-bundle (digest f257d41facd28a6e494984e72f0cf2c39cefd89d205f3ada34043494d11e1908)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

The digest is a function of the bytes that were written, and the
charters and tool grants below vary by stack, so **this digest is this
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

**`bundle.json`, `policy.json`, the three intake / reviewer / shipper
charters, and the `adapters/claude.json` trust declaration do not vary
by stack.** The seats reference the scaffold's own agents (decision
0016): `intake` and `implement` are `class: "work"`, `verify`, `review`
and `ship` are `class: "gate"` — the division of decision 0021 ruling 1
— and each agent carries its charter, its bounds and its tool allowance.
What varies by stack is the text inside the agents' `tools.allow`, the
adapter's `tool_permissions.names`, the two stack-aware charters, and
the README.

For completeness, the invariant `bundle.json`:

```json
{
  "name": "starter",
  "policy": "policy.json",
  "protected_phase": "review",
  "seats": {
    "intake": {
      "class": "work",
      "results": ["resolved"],
      "agent": "intake"
    },
    "implement": {
      "class": "work",
      "results": ["complete", "broken", "blocked"],
      "agent": "implementer"
    },
    "verify": {
      "class": "gate",
      "results": ["pass", "fail"],
      "agent": "verifier"
    },
    "review": {
      "class": "gate",
      "results": ["clean", "residual", "security-hold"],
      "agent": "reviewer"
    },
    "ship": {
      "class": "gate",
      "results": ["ready", "shipped"],
      "agent": "shipper"
    }
  }
}
```

Every other page in this directory omits it.

## The tool grants

The stack decides what a seat may run. A claude seat that may run
anything runs nothing headless: it stops at every shell prompt it is not
allowed to answer. So `init` grants the stack's own runners by name —
for this fixture, `cargo`, the leading binary of all three commands the
charters name. The grant is written in **two files that are one grant**
(decision 0016): the adapter's `tool_permissions.names` maps every name
to the `Bash(...)` expression the claude CLI understands, and each
agent's `tools.allow` lists the names it may use. An allowance whose
name the adapter map cannot express refuses the scaffold's own compile.

`adapters/claude.json`, the names half:

```json
{
  "flag": "--allowedTools",
  "names": {
    "cargo": "Bash(cargo:*)",
    "git": "Bash(git:*)",
    "ls": "Bash(ls:*)",
    "mkdir": "Bash(mkdir:*)",
    "rg": "Bash(rg:*)"
  },
  "separator": ","
}
```

`agents/intake.json` and `agents/implementer.json`, the work-class
agents, carry the full set — the runner plus `git`, `ls`, `rg` and
`mkdir` — so a seat may run exactly the commands its charter names and
nothing broader:

```json
{
  "charter": "charters/implementer.md",
  "description": "Builds the framed task to the repository's conventions and commits the work with its tests.",
  "limits": {
    "max_attempts": 2,
    "timeout_seconds": 5400
  },
  "models": [
    "opus",
    "sonnet"
  ],
  "tools": {
    "allow": [
      "cargo",
      "git",
      "ls",
      "rg",
      "mkdir"
    ],
    "mcp": []
  }
}
```

`agents/verifier.json`, `agents/reviewer.json` and `agents/shipper.json`,
the gate-class agents, carry the read-only subset — the runner plus
`git`, `ls` and `rg`, never the write tools, because nobody stands
behind the judges:

```json
{
  "charter": "charters/verifier.md",
  "description": "Runs the suites and gates and reports pass or fail on evidence, never on intent.",
  "limits": {
    "max_attempts": 2,
    "timeout_seconds": 3600
  },
  "models": [
    "sonnet",
    "opus"
  ],
  "tools": {
    "allow": [
      "cargo",
      "git",
      "ls",
      "rg"
    ],
    "mcp": []
  }
}
```

The README says the same thing in prose:

> `bundle.json` seats one agent per phase. Each agent lives in
> `agents/`, its charter in `agents/charters/`, and its tool allowance
> in the agent's `tools.allow`: the work-class seats (`intake`,
> `implement`) may run the full set — `Bash(cargo:*)`, `Bash(git:*)`,
> `Bash(ls:*)`, `Bash(rg:*)`, `Bash(mkdir:*)` — so a seat may run
> exactly the commands its charter names and nothing broader; the
> gate-class seats (`verify`, `review`, `ship`) may run the read-only
> subset — `Bash(cargo:*)`, `Bash(git:*)`, `Bash(ls:*)`, `Bash(rg:*)` —
> and never the write tools, because nobody stands behind the judges.

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

## Correcting it

Everything above is ordinary text in your tree. If your suite is
`cargo nextest run` or your lint carries `--all-features`, edit the two
charters — that is what the "correct them here if they are wrong" line
is inviting. If a seat needs a tool the scaffold did not grant, edit the
agent's `tools.allow` AND the adapter's `tool_permissions.names`: an
allowance whose name the map cannot express refuses the next compile.
Every edit moves the bundle's digest, which is the point: a strategy is
identified by its bytes, and a run records which bytes it ran.

## See also

- [quickstart.md](../quickstart.md) — the four-step spine these files
  drop into.
- [cards/rust.md](../cards/rust.md) — the quickstart's Rust delta.
- [recipe-authoring.md](../recipe-authoring.md) — what `bundle.json`
  and `policy.json` mean once you want to change them.
