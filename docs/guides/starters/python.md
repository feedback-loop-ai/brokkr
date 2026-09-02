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
initialized reviewable bundle at my-bundle (digest 7d9ee500e8314867ff20cb492af4855a6127a67e03a0d9a44e5532bd4089608b)
run brokkr from inside my-bundle — its agents/ and adapters/ declare seat tools and the trust tier the gates judge on
```

Fixture:
[`python-uv/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/python-uv/).
The fourteen files it wrote have the usual shape; `bundle.json`,
`policy.json`, and the intake / reviewer / shipper charters do not vary
by stack (see [rust.md](rust.md#what-it-wrote)). The README, adapter,
five agent grants, and these two command-bearing charters did. In this
case the detected runner is `uv`: work agents allow `uv`, `git`, `ls`,
`rg`, and `mkdir`; gate agents allow `git`, `ls`, `rg`, and `uv`.

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
  step and `python -m build` produces a wheel nobody asked for; what a
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
initialized reviewable bundle at my-bundle (digest 23860374b7fc6d12c920e1a2bfc38475131549b53c1590b7ac6c56dca5cb695c)
run brokkr from inside my-bundle — its agents/ and adapters/ declare seat tools and the trust tier the gates judge on
```

`agents/charters/implementer.md` and `agents/charters/verifier.md`, the changed part only:

```markdown
This repository reads as a python project (`pyproject.toml`), so use its own
tooling:

    python -m build
    python -m pytest
```

```markdown
This repository reads as a python project (`pyproject.toml`), so use its own
tooling:

    python -m pytest
    python -m ruff check .
```

- **`python -m …` and not bare `pytest` / `ruff`.** Without a lockfile
  `init` cannot know which environment manager is in play, so it names
  the one thing it can be sure of: the interpreter the seat is standing
  in. `python -m pytest` at least runs pytest *from that interpreter*
  rather than from whatever is first on `PATH`.
- **This is a fallback, and it is a weaker answer than the uv one.** If
  your repository has a lockfile — uv's or otherwise — say so in these
  two files. `init` writes a starting point, not a verdict.

## See also

- [cards/python.md](../cards/python.md) — the quickstart's Python
  delta.
- [bun.md](bun.md) — the other lockfile-decides-it arm, with the same
  precedence rule spelled out from the node side.
