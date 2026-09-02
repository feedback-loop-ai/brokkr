# Card — Go

**Read [quickstart.md](../quickstart.md) first.** This is a delta over
its four-step spine. Steps 1, 3 and 4 are unchanged.

## Step 2 — what `init` reads and writes

`go.mod` at the root. One marker and no ambiguity: Go has one
toolchain, so no lockfile tiebreaker is needed and `go.sum` is not read.

```
    go build ./...   # implementer
    go test ./...    # implementer + verifier
    go vet ./...     # verifier
```

- **`./...`, never a named package.** A charter that named one package
  would prove one package.
- **`go vet` is the honest lint default**, because it ships with the
  toolchain. `golangci-lint` is the better gate in most repositories and
  a broken command in one that never installed it — swap it into
  `agents/charters/verifier.md` if you have it.
- **A `go.work` adds a sentence, not a command.** `./...` beside a
  workspace file already spans every module it lists, so what a
  workspace root gets is the charter saying so. A `go.mod` with no
  `go.work` is told no such thing.

The full transcript, annotated: [starters/go.md](../starters/go.md).

## Step 3 — which recipe

The scaffold from step 2. There is no maintained `recipes/go`; the
scaffold's five seats and protected review gate are the same shape the
library's recipes have, and the two charters are already yours to edit.
