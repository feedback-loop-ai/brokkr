# Card — Python

**Read [quickstart.md](../quickstart.md) first.** This is a delta over
its four-step spine. Steps 1, 3 and 4 are unchanged.

## Step 2 — what `init` reads and writes

`pyproject.toml`, and then it looks for `uv.lock` beside it. Two
markers beat one, so uv wins where uv is in play:

| | with `uv.lock` | `pyproject.toml` alone |
|---|---|---|
| implementer, first | `uv sync` | `python3 -m build` |
| implementer, second | `uv run pytest` | `python3 -m pytest` |
| verifier, second | `uv run ruff check .` | `python3 -m ruff check .` |

- **`uv sync` sits in the build slot** because Python has no universal
  build step, and what a seat actually needs before it can run anything
  is a resolved environment. It respects the lockfile rather than
  drifting from it.
- **`uv run pytest`, not bare `pytest`.** `uv run` executes inside the
  environment uv resolved; a bare `pytest` runs against whatever
  interpreter the seat inherited, and a green suite from the wrong
  interpreter is worse than a red one.
- **The fallback is weaker and knows it.** `python3 -m …` at least runs
  the tool from the interpreter the seat is standing in rather than from
  whatever is first on `PATH` — and it names `python3`, because that is
  the interpreter a fresh project actually resolves and the name the
  shipped adapters grant. If your repository has a lockfile of any
  other kind, say so in the two charters — `init` writes a starting
  point, not a verdict.

The full transcript for both cases, annotated:
[starters/python.md](../starters/python.md).

## Step 3 — which recipe

The scaffold from step 2; there is no maintained `recipes/python`.

One thing worth checking before you run: `uv sync` resolves and installs
from your lockfile, which means every seat — the gates included —
executes third-party package code. That is Python's toolchain and not
something the scaffold adds, but `verify` syncs before `review` has read
the diff, so run this against a dependency tree you would install by
hand. Same caution as
[flow 3's second bullet](../quickstart.md#flow-3--adopt).
