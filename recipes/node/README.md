# node — the same constitution, driving a Node/TypeScript repository

Four seats, no intake phase: the feature text is the framing.
`implement` → `verify` → `review` → `ship` → `done`/`stop`, with `review`
constitutionally protected — the compiler refuses any path to a
non-`stop` terminal that bypasses it.

The phase table is `recipes/fast`'s, rule for rule, including decision
0022's reforging ladder. Nothing about a delivery's *constitution* is
language-specific, and pretending otherwise would have produced a second
table to keep in sync. What is Node here is the two things that touch a
codebase: the seats' driver commands and the model seats' role charters.

| | `recipes/fast` | `recipes/node` |
|---|---|---|
| Tools the drivers may run | `cargo`, `git`, `python3`, `pytest`, … | `npm`, `npx`, `node`, `git`, … |
| Verifier runs | `cargo test --workspace` | `npm ci`, `npx tsc --noEmit`, `npm test` |
| Reviewer reads for | Rust idiom, `crates/` | `package.json` scripts, ESM/CJS, `tsconfig.json` strictness, lockfile provenance |
| Charter warnings | frozen contracts, `fixtures/` | `node_modules/` never committed, `npm ci` not `npm install`, no `npm publish` |

## The package-manager fork — one place, named here

This recipe ships wired for **npm**. npm needs no bootstrap install: it
arrives with Node, so a stranger's first run has nothing to set up
before the first seat can work. That is the whole reason for the choice;
it is not a judgement about pnpm or yarn.

To run against a pnpm or yarn repository, fork this recipe (copy the
directory, or `extends: "node"` with an `override` per decision 0017)
and change these, and only these:

| Where | npm (shipped) | pnpm | yarn (berry) |
|---|---|---|---|
| `bundle.json` — every seat's `--allowedTools` | `Bash(npm:*),Bash(npx:*),Bash(node:*),…` | `Bash(pnpm:*),Bash(node:*),…` | `Bash(yarn:*),Bash(node:*),…` |
| Charters — install | `npm ci` | `pnpm install --frozen-lockfile` | `yarn install --immutable` |
| Charters — types | `npx tsc --noEmit` | `pnpm exec tsc --noEmit` | `yarn tsc --noEmit` |
| Charters — tests | `npm test` | `pnpm test` | `yarn test` |
| Charters — lockfile named | `package-lock.json` | `pnpm-lock.yaml` | `yarn.lock` |

Two parallel bundles are deliberately not shipped: a fork you can read
in one table beats two files that drift apart.

## Limits

`max_attempts` is 2 everywhere, as in `fast`. The timeouts are cut for a
JavaScript suite rather than a cold Rust workspace build: `implement`
4800s, `verify` 2400s, `review` 3000s, `ship` 1200s. A monorepo whose
`npm ci` alone takes ten minutes should raise them — they are seat data,
not law.

## Why every gate is still a gate

Decision 0021 makes `verify`, `review` and `ship` gate-class. Review
uses the trusted model driver. Decision 0043 permits the other two to
hold that class only because their complete `exec` dispatches declare
boxed hands; neither script seats a model. The roster test pins this
split, while `node_recipe_gates.rs` proves the remaining model gate
still refuses an untrusted driver.

## Running it

```
brokkr run --recipe node --repo /path/to/your-node-repo --feature "<the task>"
```

Your repository writes its own `realms.json`; see
[adopting a Node repo](../../docs/guides/adopting-a-node-repo.md) for
the walk from zero to a first run.

**This recipe has not been run against a live Node repository from this
repository.** It compiles under the shipped adapters and its gate
refusals are tested; that is the claim, and the only one. A run against
a real Node codebase is the operator's, and its journal is what gets to
say so.

The verifier is a boxed exec script beside this recipe's roles. Network
is refused: `npm ci --offline` can use only the bound `~/.npm` cache
artifacts, and a missing dependency fails closed with npm's decisive
line quoted in the `fail` notes.
