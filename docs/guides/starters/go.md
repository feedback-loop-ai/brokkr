# Starter sample — Go

Transcribed from a real `brokkr init` run against fixture
[`crates/brokkr-cli/tests/fixtures/init-stacks/go/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/go/).
Nothing here was written by hand.

## The repository init read

```
./go.mod
```

```
module example/stack-fixture

go 1.22
```

## The invocation

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 1bed7b3dd4fbdb728745d892dce26961bd9e38518dd9fd7d4c4373fddb75c846)
run brokkr from inside my-bundle — its adapters/ declares the trust tier the verify, review and ship seats judge on
```

## What it wrote

The same files as every stack. `bundle.json`, `policy.json` and the
intake / reviewer / shipper charters **do not vary by stack** — see
[rust.md](rust.md#what-it-wrote) for the invariant `bundle.json`. The
adapter's tool map, the agents' grants and the README name this stack's
binary (`go`) the way rust.md's name `cargo`. The two charters below
were written to this repo.

## `agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.forge/tasks/`). Match the project's
idiom. Tests are part of the change.

This repository reads as a go project (`go.mod`), so use its own
tooling:

    go build ./...
    go test ./...

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

Build and test before declaring anything. Commit your work; never push.

Result: `complete` (implemented, tests green, committed) · `broken`
(could not get it working — name the specific gap in `notes`) ·
`blocked` (something outside your control — name it precisely). Never
report `complete` with failing tests or uncommitted changes.
```

- **`a go project (`go.mod`)`** — one marker, and it is unambiguous in a
  way `package.json` is not: Go has one toolchain, so the arm needs no
  lockfile tiebreaker. `go.sum` is not read, because it changes nothing
  about which commands are right.
- **`./...`** — the package wildcard, not a named package. A charter
  that named one package would prove one package. This matters more
  than it looks: it is also why the `go.work` case below needs no new
  command.
- **No install step**, for the same reason as node: `go build` resolves
  the module cache itself.

## `agents/charters/verifier.md`

```markdown
# Verifier seat — prove it, fix nothing

Run the project's full test and lint suites from the repository root.

This repository reads as a go project (`go.mod`), so use its own
tooling:

    go test ./...
    go vet ./...

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

You change no code, fix nothing, commit nothing: one honest run is the
signal. Result: `pass` (everything green; `notes` lists commands and
counts) or `fail` (`notes` quotes the failing output's decisive lines
exactly — never soften a failure).
```

- **`go vet ./...`** is the arm's `lint`, and it is the honest default
  because it ships with the toolchain: `golangci-lint` would be a better
  gate in most repositories and a broken command in a repository that
  never installed it. Swap it in this file if you have it.

## A Go workspace

A repository with a `go.work` beside its `go.mod` gets **the same two
commands** — `go build ./...` already spans every module the workspace
lists — plus one paragraph it did not have before, between the commands
and the "chose those" line:

```markdown
This is a GO WORKSPACE (`go.work` at the root). The `./...` above
already spans every module the workspace lists — there is no
per-module command to go looking for.
```

That paragraph is the entire deliverable for this case. Inventing a new
command here would have been worse than useless: the wildcard was
already right, and what was missing was the charter saying so, so a seat
does not go hunting for a per-module invocation nobody needed. The
fixture is
[`go-workspace/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/go-workspace/)
and both directions are asserted — a `go.mod` with no `go.work` beside
it is told no such thing.

## See also

- [cards/go.md](../cards/go.md) — the quickstart's Go delta.
- [rust.md](rust.md) — the other stack whose workspace case is a
  sentence rather than a command.
