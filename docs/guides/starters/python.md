# Starter sample — Python (uv-first)

Two real `brokkr init .` runs, transcribed, each at the root of a fresh
`git init` copy of a fixture: one a uv-managed repository and one the
same repository with the `uv.lock` removed. Nothing here was written by
hand but each copy's absolute path, shown as `/home/you/my-app`, and the
paths every stack writes, elided because the
[quickstart](../quickstart.md#step-2--brokkr-init-) lists them.

Python is the second stack (after node) where `init` reads a lockfile to
decide, so this page shows both answers and the file that separates
them.

## The uv case — fixture `python-uv`

```
./pyproject.toml
./uv.lock
```

```toml
[project]
name = "recognizable-uv-app"
version = "0.1.0"
requires-python = ">=3.11"
```

```
$ brokkr init .
```

```text
initialized /home/you/my-app/.brokkr (digest d1fc9704ac0eb601ebfa6d4c8a267243ac74a6e8494b10ce3101068a0de8aaf3), writing:
  … fifteen paths and a note, elided here
Decision 0065 rules that a seat never inherits your own MCP configuration, but this build does not enforce it yet: a seat may still start the MCP servers your harness configuration names.
commit the folder: git add .brokkr && git commit
```

Fixture:
[`python-uv/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/python-uv/).
It writes the same fifteen files under `.brokkr/` as every stack (see
[rust.md](rust.md#what-it-wrote) for the invariant `bundle.json` and the
fixed parts of the agent files). What this repository changed is the
stack's own data: the two charters below, the adapter's tool map and
every agent's `tools.allow`.

### `.brokkr/agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.brokkr/state/tasks/`). Match the project's
idiom. Tests are part of the change.

This repository reads as a python/uv project (`pyproject.toml` + `uv.lock`), so use its own
tooling:

    uv sync
    uv run pytest

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

Build and test before declaring anything. Commit your work; never push.

Result: `complete` (implemented, tests green, committed) · `broken`
(could not get it working — name the specific gap in `notes`) ·
`blocked` (something outside your control — name it precisely). Never
report `complete` with failing tests or uncommitted changes.
```

### `.brokkr/scripts/verify-seat.sh`

The deterministic verifier pins `uv run pytest` and
`uv run ruff check .`. It runs both, types `pass` only when both exit
zero, and quotes decisive output on `fail`. Brokkr boxes it, with no
network, where the realm's boundary is `namespace`; under `harness`, on
a codex scaffold or macOS, no box of Brokkr's stands and its note claims
no denied network.

Annotated:

- **`python/uv`, evidence ``` `pyproject.toml` + `uv.lock` ```.** Two
  markers, so the narrower set, so it wins over the bare
  `pyproject.toml` arm — the same precedence rule that makes
  `bun.lock` out-vote npm.
- **`uv sync`** sits in the `build` slot. Python has no universal build
  step and `python3 -m build` produces a wheel nobody asked for; what a
  seat actually needs before it can run anything is a resolved
  environment, and `uv sync` is that. It is a lockfile-respecting
  command, so it checks the lockfile rather than drifting from it.
- **`uv run pytest`, not bare `pytest`.** `uv run` executes inside the
  environment uv resolved. A bare `pytest` runs against whatever
  interpreter the seat happened to inherit, which on a CI box or an
  agent's shell is a coin flip — and a green suite from the wrong
  interpreter is worse than a red one.
- **`uv run ruff check .`** — ruff over the same environment. If your
  repository uses a different linter, edit this line; that is what the
  "correct them here" invitation is for.

The tool grant follows the one binary every command runs through:
`uv` maps to `Bash(uv:*)` beside `git`, `ls`, `rg` and `mkdir`. Work
agents carry all five names; gate agents carry the same minus `mkdir`.

## The fallback case — fixture `python`

Same repository, no `uv.lock`:

```
./pyproject.toml
```

```
$ brokkr init .
```

```text
initialized /home/you/my-app/.brokkr (digest 07c1023f78ee90dfad0991de782974ba7118be8c867c0c00a09f2bfa648ffc83), writing:
  … fifteen paths and a note, elided here
Decision 0065 rules that a seat never inherits your own MCP configuration, but this build does not enforce it yet: a seat may still start the MCP servers your harness configuration names.
commit the folder: git add .brokkr && git commit
```

`.brokkr/agents/charters/implementer.md` and `.brokkr/scripts/verify-seat.sh`, the
changed part only:

```markdown
This repository reads as a python project (`pyproject.toml`), so use its own
tooling:

    python3 -m build
    python3 -m pytest
```

```markdown
This repository reads as a python project (`pyproject.toml`), so use its own
tooling:

    python3 -m pytest
    python3 -m ruff check .
```

- **`python3`, not `python`.** Without a lockfile `init` cannot know
  which environment manager is in play, so it names the one thing it can
  be sure of: the interpreter the seat is standing in — and the
  interpreter a fresh project actually resolves is `python3` (the
  scaffolded adapter grants it that name). `python3 -m pytest` at least
  runs pytest *from that interpreter* rather than from whatever is first
  on `PATH`.
- **This is a fallback, and it is a weaker answer than the uv one.** If
  your repository has a lockfile — uv's or otherwise — say so in these
  two files. `init` writes a starting point, not a verdict.

The grant names both halves of that fallback: `python3` maps to
`Bash(python3:*)` and — because the venv's own suite binary is the
honest spelling of "run the suite" in a fresh project — `pytest` maps to
the narrower `Bash(.venv/bin/pytest:*)`. Work agents carry
`python3`, `pytest`, `git`, `ls`, `rg` and `mkdir`; gate agents carry
the same without `mkdir`.

## See also

- [cards/python.md](../cards/python.md) — the quickstart's Python
  delta.
- [bun.md](bun.md) — the other lockfile-decides-it arm, with the same
  precedence rule spelled out from the node side.
