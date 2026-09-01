# Card — Bun

**This card extends [the node card](node.md).** Same four spine steps,
same everything the node card says about recipes, adapters and
monorepos. Only the package-manager lines are overridden. Read
[node.md](node.md), then this.

## Step 2 — overrides

`bun.lock` beside `package.json` is two markers where npm's arm has one,
and the narrower set wins. So `init` writes bun's commands, not npm's:

| | node card | this card |
|---|---|---|
| implementer, first | `npm run build` | `bun install --frozen-lockfile` |
| implementer, second | `npm test` | `bun run test` |
| verifier, second | `npm run lint` | `bun run typecheck` |

Three notes, and nothing else on this page differs:

- **An install step, where node's arm names none.** `bun run test`
  against an unpopulated `node_modules` fails for a reason that has
  nothing to do with the change under test, and bun's install is fast
  enough that a charter can honestly ask a seat to pay for it.
  `--frozen-lockfile` makes it a check rather than a resolution: an
  implementer that quietly updated the lockfile to make its own build
  pass would be editing the evidence.
- **`bun run test`, not bare `bun test`.** Bare `bun test` is bun's own
  test runner — a different program. `bun run test` runs what your
  `package.json` says.
- **`typecheck` where node gets `lint`.** The slot is "the second
  proving command"; for a bun/TypeScript repository a type check
  catches more. Add your lint as a third line if you have one.

The full transcript, annotated: [starters/bun.md](../starters/bun.md).

## Step 3 — overrides

None at the spine level: `brokkr run --bundle . --repo . --feature "…"`
is the same command.

One thing that is **not** an override and is worth re-reading from the
node card: every seat's driver runs `bun install`, which executes your
dependency tree's lifecycle scripts, and `verify` installs before
`review` has read the diff. That exposure is bun's and npm's alike.

There is no shipped `recipes/bun`. If you want the maintained
`recipes/node` shape under bun, copy it and swap the install,
type-check and test commands plus the lockfile name in its charters —
the swap points are tabulated in
[`recipes/node/README.md`](../../../recipes/node/README.md).

## Monorepos

`turbo.json` plus `bun.lock` is `bunx turbo run test`. Same table as
[node.md § monorepos](node.md#monorepos); only the prefix differs.
