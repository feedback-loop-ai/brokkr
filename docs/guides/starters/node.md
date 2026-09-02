# Starter sample — Node (npm)

What `brokkr init` actually wrote for a Node repository, transcribed
from a real run: the fixture was copied into a scratch directory,
`brokkr init my-bundle` was run inside it, and its output was pasted
here and then annotated.

The fixture is
[`crates/brokkr-cli/tests/fixtures/init-stacks/node-npm/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/node-npm/).

This is the **base** node sample. [bun.md](bun.md) is a diff over it;
the monorepo section at the bottom is the other diff.

## The repository init read

```
./package.json
```

No lockfile at the root. That is the whole reason this page says npm:
`package.json` alone is the *widest* node marker set, so it is what is
left after `bun.lock`, `pnpm-lock.yaml` and `yarn.lock` have each had
their say.

## The invocation

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 73c4d35763c6684dbf95b299968ba59739af86124c5dc764899565263f2e875d)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

## What it wrote

The same fourteen files as every stack: `agents/README.md`, `bundle.json`,
`policy.json`, `adapters/claude.json`, five agent definitions under
`agents/` and their five charters under `agents/charters/`. The
invariant `bundle.json` — five seats each naming an agent — is in
[rust.md](rust.md#what-it-wrote), and so are the fixed parts of the
agent files. What this repository changed is the stack's own data:
the two charters below, `adapters/claude.json`'s tool map and every
agent's `tools.allow` (shown after the charters).

## `agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.forge/tasks/`). Match the project's
idiom. Tests are part of the change.

This repository reads as a node/npm project (`package.json`), so use its own
tooling:

    npm run build
    npm test

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

Build and test before declaring anything. Commit your work; never push.

Result: `complete` (implemented, tests green, committed) · `broken`
(could not get it working — name the specific gap in `notes`) ·
`blocked` (something outside your control — name it precisely). Never
report `complete` with failing tests or uncommitted changes.
```

- **`a node/npm project (`package.json`)`** — one marker in the
  evidence, and that single marker is the reason this arm lost to none
  of the others: nothing narrower matched. Had `pnpm-lock.yaml` been
  beside it, the evidence would read ``` `package.json` +
  `pnpm-lock.yaml` ``` and the commands would be pnpm's.
- **`npm run build` / `npm test`** — note the asymmetry, which is npm's
  and not Brokkr's: `test` is one of npm's built-in lifecycle scripts
  so `npm test` works bare, while `build` is not, so it needs `npm run`.
- **No install step.** The three lockfile-free node arms name none: what
  `npm ci` should do in your repository depends on your CI, your
  `node_modules` state and your network posture, and a charter that
  guessed would be guessing about the network. ([bun.md](bun.md) is the
  one node arm that *does* name an install, for a reason it explains.)
- **No `NO STACK WAS RECOGNIZED` banner.** That paragraph — and it is a
  loud one — appears only when nothing matched, so a placeholder can
  never be mistaken for a command chosen for your project.

## `agents/charters/verifier.md`

```markdown
# Verifier seat — prove it, fix nothing

Run the project's full test and lint suites from the repository root.

This repository reads as a node/npm project (`package.json`), so use its own
tooling:

    npm test
    npm run lint

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

You change no code, fix nothing, commit nothing: one honest run is the
signal. Result: `pass` (everything green; `notes` lists commands and
counts) or `fail` (`notes` quotes the failing output's decisive lines
exactly — never soften a failure).
```

- **`npm run lint`** — a guess with a name on it. If your
  `package.json` has no `lint` script this command fails loudly at the
  verify gate, which is the correct failure: it is a five-second edit to
  this file, and a silently-skipped lint is not.

## The tool grant

Every command this stack's seats run goes through one binary — `npm` —
so the adapter's `tool_permissions.names` maps it and the four tools
every seat needs:

```json
"names": {
  "git": "Bash(git:*)",
  "ls": "Bash(ls:*)",
  "mkdir": "Bash(mkdir:*)",
  "npm": "Bash(npm:*)",
  "rg": "Bash(rg:*)"
}
```

The work agents (`intake`, `implement`) carry all five names in
`tools.allow`; the gate agents (`verify`, `review`, `ship`) carry the
same minus `mkdir`. The grant is per binary, not per subcommand:
`Bash(npm:*)` answers to `npm run build` as readily as to `npm test`,
and what keeps a gate from building is its charter, not the glob.

## The monorepo variant

The same node stack with a `turbo.json` at the root is a **different**
answer, and this is a real capture too — fixture
[`turbo-pnpm/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/turbo-pnpm/),
carrying `package.json`, `turbo.json` and `pnpm-lock.yaml`:

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 1359509b294a10b7d46885bd112459f90198b39d882db8d7b363c3b6e392c07c)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

`agents/charters/implementer.md`:

```markdown
This repository reads as a node/turbo project (`package.json` + `turbo.json` + `pnpm-lock.yaml`), so use its own
tooling:

    pnpm exec turbo run build
    pnpm exec turbo run test

This is a MONOREPO: `turbo.json` names an orchestrator, and the
commands above are the orchestrator's own — they span every
workspace package. Do not substitute a single package's script.
```

Two axes crossed, both read from the root:

- **`turbo run build`** — `turbo.json` is checked *before* the
  per-package-manager table, because in a monorepo the root
  `package.json`'s scripts belong to one member. A command that proved
  one package and called the repository green is the failure this arm
  exists to prevent.
- **`pnpm exec`** — *which* package manager runs turbo comes from
  whichever lockfile is at the root. `bun.lock` gives `bunx turbo run
  test`, `yarn.lock` gives `yarn exec`, and a repository with no
  lockfile at all gets `npx`, which resolves the local install before it
  reaches for the registry. All four are fixtures and all four are
  asserted — and the granted binary follows the same prefix: this
  scaffold's map names `pnpm`, a `turbo-bun` scaffold's names `bunx`, a
  lockfile-free one's names `npx`.
- **The MONOREPO paragraph** — right command, but a seat that was not
  told it is in a monorepo may still "helpfully" narrow it to one
  package. `nx.json` gets the same treatment with `nx run-many -t test`.

## See also

- [bun.md](bun.md) — the same stack under bun; overrides only the
  package-manager lines.
- [cards/node.md](../cards/node.md) — the quickstart's Node delta, and
  the base every other card extends.
- [adopting-a-node-repo.md](../adopting-a-node-repo.md) — driving a
  Node repo with the maintained `recipes/node` instead of a scaffold.
