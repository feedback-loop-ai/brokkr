# Starter sample — Go

Transcribed from a real `brokkr init .` run at the root of a fresh `git
init` copy of fixture
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

Run at this module's root, `brokkr init .` prints what the quickstart's
[Step 2](../quickstart.md#step-2--brokkr-init-) shows; only the digest
is the module's own.

## What it wrote

The same `.brokkr/` folder as every stack: three model offices and two
exec gates that Brokkr boxes under `namespace` only, with model and exec
adapters, verify/ship scripts and `realms.json`. The
invariant `bundle.json` is in
[rust.md](rust.md#what-it-wrote); so are the fixed parts of the agent
files. What this repository changed is the stack's own data:

- `.brokkr/agents/charters/implementer.md` and `.brokkr/scripts/verify-seat.sh`
  below;
- `.brokkr/adapters/claude.json`'s tool map and every agent's `tools.allow`,
  sized to this stack's commands (shown after the charters).

## `.brokkr/agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.brokkr/state/tasks/`). Match the project's
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

## `.brokkr/scripts/verify-seat.sh`

The deterministic verifier contains these detected command pins:

```bash
test_command='go test ./...'
lint_command='go vet ./...'
```

It runs both from the repository root, types `pass` only when both exit
zero, and quotes decisive output on `fail`. Where the realm's boundary
is `namespace`, Brokkr boxes it with no network. Under `harness` —
which `init` declares on macOS — it runs under no box of Brokkr's, and its
note claims no denied network.

- **`go vet ./...`** is the arm's `lint`, and it is the honest default
  because it ships with the toolchain: `golangci-lint` would be a better
  gate in most repositories and a broken command in a repository that
  never installed it. Swap it into this script if you have it.

## The tool grant

Every command this stack's seats run goes through one binary — `go` —
so the adapter's `tool_permissions.names` maps it and the four tools
every seat needs:

```json
"names": {
  "git": "Bash(git:*)",
  "go": "Bash(go:*)",
  "ls": "Bash(ls:*)",
  "mkdir": "Bash(mkdir:*)",
  "rg": "Bash(rg:*)"
}
```

The work agents (`intake`, `implement`) carry all five names in
`tools.allow`; the review gate's agent carries the same minus `mkdir`.
The grant is per binary, not per subcommand:
`Bash(go:*)` answers to `go build` as readily as to `go test`, and what
keeps a gate from building is its charter, not the glob.

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
