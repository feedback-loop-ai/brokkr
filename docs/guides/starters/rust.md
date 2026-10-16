# Starter sample — Rust

What `brokkr init` actually wrote for a Rust repository, transcribed
from a real run. Nothing on this page was composed by hand: the fixture
below was copied into a fresh `git init` directory, `brokkr init .` was
run at its root, and the files it wrote were pasted here and then
annotated.

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

`brokkr init .` in this repository prints what the quickstart's
[Step 2](../quickstart.md#step-2--brokkr-init-) shows, which the doc
tests run from this same fixture: the fifteen paths `init` wrote, the
agent CLI it scaffolded for and the commit to make. The
digest is a function of the bytes that were written, and the
charters and tool map below vary by stack, so **the digest is this
repository's**. Your Rust repository will print a different one and that is
correct.

## What it wrote

The fifteen paths of that list, every one under `.brokkr/` and nothing at the
root of the repository (decision 0083). `.brokkr/.gitignore` holds the
one line `state/`, so what a run writes into `.brokkr/state/` stays out
of git, and the rest is the tracked scaffold you commit. `.brokkr/` is
the bundle, so the script paths `bundle.json` names are read from there.

A repository carrying `.specify/` or `openspec/config.yaml` also gets
`.brokkr/dialects/`, the detected dialect's pinned data. The adapters
follow the agent CLI on `PATH`: this fixture was scaffolded for `claude`.
A codex scaffold writes `.brokkr/adapters/codex.json` in its place, and a
dsh scaffold writes `.brokkr/adapters/dsh.json` beside it.

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
      "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify-seat.sh", "{prompt_file}"]},
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
      "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash", "./scripts/ship-seat.sh", "{prompt_file}", "{brokkr}"]},
      "hands": {"kind": "workspace", "network": false, "binds": []}
    }
  }
}
```

The three model offices name agents; their charter, model chain, limits and
tool grant live under `.brokkr/agents/`, where `brokkr agents show <name>` reads
them back. Verify and ship are deterministic scripts with their limits and
hands declared at the site, which Brokkr boxes under the `namespace`
boundary only. Class remains the seat's authority.

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
  `["cargo", "git", "ls", "rg", "mkdir"]`, pre-approved as exactly the
  commands its charter names.
- **the review gate** — the same set without `mkdir`:
  `["cargo", "git", "ls", "rg"]`. The verify and ship scripts carry no
  grant.

The grant is per BINARY, not per subcommand: `Bash(cargo:*)` answers to
`cargo build` as readily as to `cargo test`. What keeps a gate from
building is its charter — "prove it, fix nothing" — and the scaffold
README says so rather than promising a boundary the glob cannot draw.
Nor is the list a boundary on an unboxed seat: `--allowedTools`
pre-approves these commands and removes no other tool, so the
operator's own Claude Code permission settings reach the seat too.
Decision 0065 makes its MCP servers the engine's to compose, never the
operator's, but that is not enforced yet: until strict MCP isolation
lands, a seat may still start the servers in the operator's own
harness configuration, and each adapter's `mcp` records what its
harness was measured to exclude ([security model](../../security-model.md)).
An allowance is ONE grant with the adapter's map: a name the map cannot
express refuses the scaffold's own compile (decision 0016), so when you
edit one side, edit both.

## `.brokkr/agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.brokkr/state/tasks/`). Match the project's
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

## `.brokkr/scripts/verify-seat.sh`

The deterministic verifier contains these detected command pins:

```bash
test_command='cargo test --workspace'
lint_command='cargo clippy --workspace --all-targets -- -D warnings'
```

It runs both, types `pass` only when both exit zero, and quotes decisive
output on `fail`. Where the realm's boundary is `namespace`, Brokkr
boxes it with no network and it uses the bound Cargo registry cache.
Under `harness` — a codex scaffold, or macOS — it runs under no box of
Brokkr's, and its note claims no denied network.

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
  "charter": "charters/implementer.md",
  "description": "Builds the framed task to the repository's conventions and commits the work with its tests.",
  "efforts": {
    "opus": "high",
    "sonnet": "high"
  },
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

The bounds are the ones the starter used to declare inline on the seat
(decision 0006); since the seats name agents, they ride here. The model
chains are the repository's own library's — work leads with the stronger
model where the ship does, and every chain is a preference, not a
promise. `efforts` is the reasoning effort each model is asked for.

## Correcting it

All of it is ordinary JSON or Markdown in your tree. If your suite is
`cargo nextest run` or your lint carries `--all-features`, edit
`.brokkr/agents/charters/implementer.md` and `.brokkr/scripts/verify-seat.sh`; if a
seat needs a narrower or wider grant, edit its `tools.allow` in
`.brokkr/agents/<name>.json` AND the matching entry in the adapter's map — the
scaffold's own compile is the check that the two still agree. Every edit
moves the bundle's digest, which is the point: a strategy is identified
by its bytes, and a run records which bytes it ran.

## What the fallback looks like

A repository `init` does not recognize — no manifest, no lockfile, no
`Makefile` — gets the same fifteen files with an **empty** tool map
(`"names": {}`), no `tools` list on any agent, and a README that
says so in those words rather than granting a guessed permission. The
two charters carry `<this project's …>` placeholders. See
[quickstart.md](../quickstart.md#step-2--brokkr-init-) for the shape.

## See also

- [quickstart.md](../quickstart.md) — the four-step spine these files
  drop into.
- [cards/rust.md](../cards/rust.md) — the quickstart's Rust delta.
- [recipe-authoring.md](../recipe-authoring.md) — what `bundle.json`
  and `policy.json` mean once you want to change them.
