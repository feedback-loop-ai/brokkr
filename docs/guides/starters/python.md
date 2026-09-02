# Starter sample — Python (uv-first)

Two real `brokkr init` runs, transcribed: one against a uv-managed
repository and one against the same repository with the `uv.lock`
removed. Nothing here was written by hand.

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
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest ba4fb4b75481a19b0dbe47b3ca3453b54920288881cea605324d877068c5463f)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

Fixture:
[`python-uv/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/python-uv/).
The files it wrote are the usual set — `bundle.json`, `policy.json`,
`README.md`, `adapters/claude.json`, and under `agents/` five
definitions plus five charters; `bundle.json`, `policy.json`, the trust
declaration and the intake / reviewer / shipper charters **do not vary
by stack** (see [rust.md](rust.md#what-it-wrote)). The stack-specific
half below is this page's subject.

The tool grants name the runner `uv`: `adapters/claude.json` maps
`uv → Bash(uv:*)` beside `git`, `ls`, `rg` and `mkdir`, the work-class
agents carry all five names, and the gate-class agents carry
`["uv", "git", "ls", "rg"]`.

### `agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.forge/tasks/`). Match the project's
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

### `agents/charters/verifier.md`

```markdown
# Verifier seat — prove it, fix nothing

Run the project's full test and lint suites from the repository root.

This repository reads as a python/uv project (`pyproject.toml` + `uv.lock`), so use its own
tooling:

    uv run pytest
    uv run ruff check .

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

You change no code, fix nothing, commit nothing: one honest run is the
signal. Result: `pass` (everything green; `notes` lists commands and
counts) or `fail` (`notes` quotes the failing output's decisive lines
exactly — never soften a failure).
```

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

## The fallback case — fixture `python`

Same repository, no `uv.lock`:

```
./pyproject.toml
```

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 87cad18aba3c3bfdecd524c3d4e82d585b429a27f6710c4094aff4eb7b0e9faf)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

`agents/charters/implementer.md` and `agents/charters/verifier.md`, the
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

The tool grants name the interpreter and the venv's suite binary:
`adapters/claude.json` maps `python3 → Bash(python3:*)` and
`pytest → Bash(.venv/bin/pytest:*)` beside `git`, `ls`, `rg` and
`mkdir`. The work-class agents carry all six names; the gate-class
agents carry `["python3", "pytest", "git", "ls", "rg"]` — the runner
that tests and lint run through, never the write tools.

- **`python3 -m …` and not bare `pytest` / `ruff`.** Without a lockfile
  `init` cannot know which environment manager is in play, so it names
  the one thing it can be sure of: the interpreter the seat is standing
  in, spelled `python3` because that is the binary a `Bash(python3:*)`
  allowance can answer. `python3 -m pytest` at least runs pytest *from
  that interpreter* rather than from whatever is first on `PATH`.
- **pytest is granted beside it.** The venv's suite binary has a
  narrower allowance than the interpreter that can also build with it —
  `Bash(.venv/bin/pytest:*)` — so a seat can run the suite directly
  without holding the interpreter.
- **This is a fallback, and it is a weaker answer than the uv one.** If
  your repository has a lockfile — uv's or otherwise — say so in these
  two files. `init` writes a starting point, not a verdict.

## See also

- [cards/python.md](../cards/python.md) — the quickstart's Python
  delta.
- [bun.md](bun.md) — the other lockfile-decides-it arm, with the same
  precedence rule spelled out from the node side.
