# Starter sample — Bun

**This page extends [node.md](node.md).** The scaffold is the same
fourteen files; what differs is the stack's own data — the two charters'
package-manager lines, the adapter's tool map and every agent's
`tools.allow`. If you have not read the node sample, read it first —
everything it says about the invariant `bundle.json`, the missing install
step and the `NO STACK WAS RECOGNIZED` banner still holds.

Transcribed from a real run against fixture
[`crates/brokkr-cli/tests/fixtures/init-stacks/node-bun/`](../../../crates/brokkr-cli/tests/fixtures/init-stacks/node-bun/).

## The repository init read

```
./bun.lock
./package.json
```

```json
{
  "name": "recognizable-bun-app",
  "private": true,
  "scripts": {
    "test": "bun test",
    "typecheck": "tsc --noEmit"
  }
}
```

`bun.lock` beside `package.json` is the whole of the difference.

## The invocation

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest fcdb0fc2d428c0b73746ec923a9be398f9dc64d2455da8a6dd1177d6e5d89ce0)
run brokkr from inside my-bundle — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

## `agents/charters/implementer.md`

```markdown
# Implementer seat — build it

Implement the framed task (see `.forge/tasks/`). Match the project's
idiom. Tests are part of the change.

This repository reads as a node/bun project (`package.json` + `bun.lock`), so use its own
tooling:

    bun install --frozen-lockfile
    bun run test

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

Build and test before declaring anything. Commit your work; never push.

Result: `complete` (implemented, tests green, committed) · `broken`
(could not get it working — name the specific gap in `notes`) ·
`blocked` (something outside your control — name it precisely). Never
report `complete` with failing tests or uncommitted changes.
```

The three lines that differ from [node.md](node.md), and why:

- **`node/bun` instead of `node/npm`, evidence ``` `package.json` +
  `bun.lock` ```.** Two markers is the narrower set, and the narrower
  set wins. This is the whole fix: a bun-managed repository has only
  `package.json` as far as the npm fallback can see, so before this arm
  existed `init` wrote `npm run build` and `npm test` into the charters
  of a repository with no npm lockfile to install from. The bun sample
  contains the string `npm` exactly zero times, and a test asserts that
  — not "no npm command", no npm *anywhere*, because a charter that
  merely mentions it is one a seat can misread at 3am.
- **`bun install --frozen-lockfile` where node's arm names no install
  step at all.** This is the one node arm that does, and the divergence
  is deliberate. `bun run test` against an unpopulated `node_modules`
  fails for a reason that has nothing to do with the change under test,
  and bun's install is fast enough that a charter can honestly ask a
  seat to pay for it. `--frozen-lockfile` is what makes it a *check*
  rather than a resolution: an implementer that quietly updated the
  lockfile to make its own build pass would be editing the evidence.
- **`bun run test` where npm gets bare `npm test`.** Bun has no
  lifecycle-script shorthand for `test` that means "run the package's
  `test` script" — bare `bun test` is bun's own test runner, which is a
  different program. `bun run test` runs what `package.json` says.

## `agents/charters/verifier.md`

```markdown
# Verifier seat — prove it, fix nothing

Run the project's full test and lint suites from the repository root.

This repository reads as a node/bun project (`package.json` + `bun.lock`), so use its own
tooling:

    bun run test
    bun run typecheck

`brokkr init` chose those from the files at the repository root —
it ran nothing to find out. Correct them here if they are wrong.

You change no code, fix nothing, commit nothing: one honest run is the
signal. Result: `pass` (everything green; `notes` lists commands and
counts) or `fail` (`notes` quotes the failing output's decisive lines
exactly — never soften a failure).
```

- **`bun run typecheck` sits where the other node arms put `lint`.** The
  seat's slot is "the second proving command", and for a bun/TypeScript
  repository a type check is the one that catches more. If your
  repository lints too, add the line — this file is yours.

## The tool grant

This stack's adapter map names `bun` — every command runs through it —
plus `git`, `ls`, `rg` and `mkdir`:

```json
"names": {
  "bun": "Bash(bun:*)",
  "git": "Bash(git:*)",
  "ls": "Bash(ls:*)",
  "mkdir": "Bash(mkdir:*)",
  "rg": "Bash(rg:*)"
}
```

The work agents carry all five names in `tools.allow`; the gate agents
carry the same minus `mkdir`. Read that sentence twice, because bun is
the arm where it matters most: the gate seats hold `Bash(bun:*)` —
their test runner IS bun — and that same glob answers to `bun install`
as readily as to `bun run test`. The grant cannot draw the boundary, so
it is each gate's charter — "prove it, fix nothing", with no install
line — and not the grant that keeps a verify seat from installing. The
boundary is not expressible finer than per binary: a gate that may not
run `bun` at all could not run `bun run test` either, and an allowance
whose name the adapter map lacks refuses the scaffold's own compile.

## The install step is a real cost

The implementer pays for `bun install --frozen-lockfile` on every
attempt, and a `bun install` executes your dependency tree's lifecycle
scripts. That is the JavaScript toolchain, not something this scaffold
adds, and it is the same exposure your CI has — but the `verify` seat
installs before `review` has read the diff, so a dependency the
implementer added has already run its install scripts by the time anyone
reviews its provenance. Run this against a dependency tree you would
install by hand. The same warning, in more detail, is in
[adopting-a-node-repo.md § what you are granting](../adopting-a-node-repo.md#what-you-are-granting).

## A bun monorepo

`turbo.json` plus `bun.lock` is `bunx turbo run test` — the orchestrator
arm crossed with bun's runner. See
[node.md § the monorepo variant](node.md#the-monorepo-variant); the only
difference is the prefix.

## See also

- [node.md](node.md) — the base this page overrides.
- [cards/bun.md](../cards/bun.md) — the quickstart's bun delta, which
  extends the node card the same way.
