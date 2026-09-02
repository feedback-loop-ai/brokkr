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
initialized reviewable bundle at my-bundle (digest 9ef00f82d48db4bfc2c7a8db709ee8aed690184deab8218e68832997c8566879)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

## What it wrote

The same files as every other stack — `bundle.json`, `policy.json`,
`README.md`, `adapters/claude.json`, and under `agents/` the five agent
definitions and their five charters under `agents/charters/`.
`bundle.json`, `policy.json`, the trust declaration inside
`adapters/claude.json`, and the intake / reviewer / shipper charters
**do not vary by stack**; see [rust.md](rust.md#what-it-wrote) for the
invariant `bundle.json` in full, and [rust.md § the tool
grants](rust.md#the-tool-grants) for how the allowances work. What was
written to *this* repository is the stack-specific half below.

## The tool grants

The stack's runner here is `npm` — the leading binary of `npm run
build`, `npm test` and `npm run lint` — so the adapter's
`tool_permissions.names` maps `npm` beside the read trio and `mkdir`,
and the agents' `tools.allow` lists are sized by class:

```json
{
  "names": {
    "git": "Bash(git:*)",
    "ls": "Bash(ls:*)",
    "mkdir": "Bash(mkdir:*)",
    "npm": "Bash(npm:*)",
    "rg": "Bash(rg:*)"
  }
}
```

`agents/intake.json` and `agents/implementer.json` (work class) carry
`["npm", "git", "ls", "rg", "mkdir"]`; `agents/verifier.json`,
`agents/reviewer.json` and `agents/shipper.json` (gate class) carry
`["npm", "git", "ls", "rg"]` — the read-only subset, never the write
tools. `Bash(npm:*)` reaches `npm ci`, which executes your dependency
tree's lifecycle scripts: that is the JavaScript toolchain, not
something the scaffold adds — see
[adopting-a-node-repo.md § what you are granting](../adopting-a-node-repo.md#what-you-are-granting).

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

## The monorepo variant

The same node stack with a `turbo.json` at the root is a **different**
answer, and this is a real capture too — fixture
[`turbo-pnpm/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/turbo-pnpm/),
carrying `package.json`, `turbo.json` and `pnpm-lock.yaml`:

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 0ec0d8ce01870f376fb1bc8f22576d70d77d5856027c3f3a8a9633272d9dfd32)
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
  asserted.
- **The grants follow the runner.** The monorepo's tool grants are the
  runner's: a turbo repo locked to pnpm grants `pnpm`, one locked to bun
  grants `bunx`, one with no lockfile grants `npx` — each beside `git`,
  `ls`, `rg` and, for the work class, `mkdir`.
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
