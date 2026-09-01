# Card — Node

**Read [quickstart.md](../quickstart.md) first.** This is a delta over
its four-step spine, not a walkthrough. Steps 1 and 4 are unchanged;
this page is what steps 2 and 3 do differently here.

This is the **base node card**. [bun.md](bun.md) extends it.

## Step 2 — what `init` reads and writes

`init` looks for `package.json` at the root. With no lockfile beside it
you get the npm arm, because plain `package.json` is the widest node
marker set and so the last one tried:

```
    npm run build      # implementer
    npm test           # implementer + verifier
    npm run lint       # verifier
```

Two things to check before step 3:

- **`npm run lint` is a guess with a name on it.** If your
  `package.json` has no `lint` script, edit `roles/verifier.md` — that
  is a five-second fix, and a lint that silently does not run is not.
- **No install step is scaffolded.** What `npm ci` should do in your
  repository depends on your CI and your network posture, and a charter
  that guessed would be guessing about the network.

The full transcript, annotated: [starters/node.md](../starters/node.md).

## Step 3 — which recipe

`recipes/node` is the maintained reference recipe for a JavaScript
stack, and it is a better starting point than the scaffold if you are
adopting rather than experimenting:

```
brokkr recipes add ~/src/brokkr/recipes/node --name node
brokkr run --recipe node --repo . --feature "…"
```

It has four seats rather than five (no intake — the feature text is the
framing), and its charters name `npm ci` and `npx tsc --noEmit`
explicitly. That means it needs a committed `package-lock.json` and
`typescript` in `devDependencies`; without a local install `npx`
resolves outward to the registry, which is not what you want a gate seat
doing.

**`recipes add` brings no `adapters/` tree with it**, and `verify`,
`review` and `ship` are gate-class. Copy `adapters/claude.json` into
your repo *before* `recipes add`, or it refuses and leaves you nothing.
This is [flow 3's](../quickstart.md#flow-3--adopt) first bullet.

## Monorepos

A `turbo.json` or `nx.json` at the root changes step 2's answer
entirely: `init` names the **orchestrator's** commands, not a single
package's, because a command that proved one package would call the
whole repository green.

```
    pnpm exec turbo run build      # with pnpm-lock.yaml at the root
    bunx turbo run test            # with bun.lock
    yarn exec nx run-many -t test  # with yarn.lock, nx instead of turbo
    npx turbo run test             # with no lockfile at all
```

The prefix is read from whichever lockfile is present; `npx` is what is
left, and it resolves your local install before it reaches for the
registry. The charters also say, in those words, that this is a monorepo
— so a seat does not helpfully narrow the command back down to one
package.

Raise the seat `limits` in `bundle.json` if your install alone runs ten
minutes. They are seat data, not law.

## See also

- [bun.md](bun.md) — this card with the package-manager lines
  overridden.
- [starters/node.md](../starters/node.md) — the actual scaffold output.
- [adopting-a-node-repo.md](../adopting-a-node-repo.md) — the long form
  of step 3's `recipes/node` path.
