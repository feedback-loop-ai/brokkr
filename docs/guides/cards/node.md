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

Three things to check before step 3:

- **`npm run lint` is a guess with a name on it.** If your
  `package.json` has no `lint` script, edit `.brokkr/scripts/verify-seat.sh` — that
  is a five-second fix, and a lint that silently does not run is not.
- **No install step is scaffolded.** What `npm ci` should do in your
  repository depends on your CI and your network posture, and a charter
  that guessed would be guessing about the network.
- **Where `npm` is installed was read once, at `init`.** It looked
  `npm` and `node` up on its own `PATH`, following links (#717). What
  it found is bound `ro` on verify's hands in `bundle.json`: a binary
  standing in its `PATH` directory as that one file, a volta shim as
  `~/.volta`, and an nvm version as its `bin` and `lib/node_modules`. The
  verifier puts the directory each was found in at the head of `PATH`,
  and exports `VOLTA_HOME` for a volta shim. Only an install's own
  directory is bound: one that holds anything else, such as a
  `credentials` file, is refused, naming it. With no `npm` on `PATH`, or
  one refused, `init` says so in one line, and the verifier fails with
  that line.

The full transcript, annotated: [starters/node.md](../starters/node.md).

## Step 3 — which recipe

`recipes/node` is the maintained reference recipe for a JavaScript
stack, and it is a better starting point than the scaffold if you are
adopting rather than experimenting. In a repository with step 2's
folder it goes on the folder's shelf, `.brokkr/recipes/node/`, which
`run --recipe` reads with no flag:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
added recipe 'node' (…) at /home/you/my-app/.brokkr/recipes/node
added recipe 'fast' (…) at /home/you/my-app/.brokkr/recipes/fast
…
```

The elided lines name each agent and charter it copies into
`.brokkr/agents/`, which
[adopting-a-node-repo.md](../adopting-a-node-repo.md#the-files-your-repo-needs)
shows whole. Then `brokkr run --recipe node --feature "…"`. `node` and
`fast` are Brokkr's two portable recipes (decision
[0087](../../decisions/0087-the-base-offices-are-the-ones-init-writes.md)
ruling 5): `node`'s agents extend the `implementer` and `reviewer` that
step 2 wrote and add only their charters and inputs, so its model seats
hire what `init` wrote for your agent CLI.

It has four seats rather than five (no intake — the feature text is the
framing), and its charters name `npm ci` and `npx tsc --noEmit`
explicitly. That means it needs a committed `package-lock.json` and
`typescript` in `devDependencies`; without a local install `npx`
resolves outward to the registry, which is not what you want a gate seat
doing.

`recipes add` brings `fast`, the recipe `node` extends, onto the shelf
with it, and the agents both seat. **It brings no adapters**, and
`verify`, `review` and `ship` are gate-class: your folder's adapters must
vouch for them, or the compile refuses and leaves you nothing. This is
[flow 3's](../quickstart.md#flow-3--adopt) first bullet.

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

Raise an agent's `limits` in `.brokkr/agents/*.json` if your install alone runs
ten minutes — the bounds ride on the agent the seat names, not on the seat.

## See also

- [bun.md](bun.md) — this card with the package-manager lines
  overridden.
- [starters/node.md](../starters/node.md) — the actual scaffold output.
- [adopting-a-node-repo.md](../adopting-a-node-repo.md) — the long form
  of step 3's `recipes/node` path.
