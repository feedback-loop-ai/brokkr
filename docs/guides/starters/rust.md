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
my-bundle/adapters/claude.json
my-bundle/adapters/exec.json
my-bundle/agents/README.md
my-bundle/agents/charters/implementer.md
my-bundle/agents/charters/intake.md
my-bundle/agents/charters/reviewer.md
my-bundle/agents/implementer.json
my-bundle/agents/intake.json
my-bundle/agents/reviewer.json
my-bundle/bundle.json
my-bundle/policy.json
my-bundle/scripts/ship-seat.sh
my-bundle/scripts/verify-seat.sh
```

**The policy, intake/reviewer charters, ship script, and fixed model-agent
fields do not vary by stack.** What varies is the stack's own data: the
implementer charter, verifier script, model adapter tool map, model-agent
allowances, and the verifier's cache binds.

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
      "class": "gate",
      "results": ["pass", "fail"],
      "limits": {"max_attempts": 2, "timeout_seconds": 3600},
      "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "scripts/verify-seat.sh", "{prompt_file}"]},
      "hands": {"kind": "workspace", "network": false, "binds": [
        {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
        {"path": "~/.rustup", "mode": "ro"}
      ]}
    },
    "review": {
      "agent": "reviewer",
      "class": "gate",
      "results": ["clean", "residual", "security-hold"]
    },
    "ship": {
      "class": "gate",
      "results": ["ready", "shipped"],
      "limits": {"max_attempts": 2, "timeout_seconds": 1800},
      "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "scripts/ship-seat.sh", "{prompt_file}", "{brokkr}"]},
      "hands": {"kind": "workspace", "network": false, "binds": []}
    }
  }
}
```

The three model offices name agents; their charter, model chain, limits and
tool grant live under `agents/`, where `brokkr agents show <name>` reads
them back. Verify and ship are deterministic scripts with their limits and
boxed hands declared at the site. Class remains the seat's authority.

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

## `scripts/verify-seat.sh`

The deterministic boxed verifier contains these detected command pins:

```bash
test_command='cargo test --workspace'
lint_command='cargo clippy --workspace --all-targets -- -D warnings'
```

It runs both with network denied, using the bound Cargo registry cache,
types `pass` only when both exit zero, and quotes decisive output on
`fail`.

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
`agents/charters/implementer.md` and `scripts/verify-seat.sh`; if a
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
