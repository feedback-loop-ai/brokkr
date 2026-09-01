# Adopting a Node repo — from nothing installed to a first run

This guide walks a stranger with a Node/TypeScript repository from zero
to one Brokkr run driven by [`recipes/node`](../../recipes/node/), the
reference recipe for a JavaScript stack.

Zero really is zero on the Brokkr side: there is nothing to `npx`, no
package to add to your `devDependencies`, no postinstall hook. Brokkr is
one native binary written in Rust (decision 0009) that stands *outside*
your project and drives it. Your `package.json` never learns it exists.

- [1. What you need](#1-what-you-need)
- [2. Install the binary](#2-install-the-binary)
- [3. Give your repo the four files](#3-give-your-repo-the-four-files)
- [4. Write your `realms.json`](#4-write-your-realmsjson)
- [5. Check the machine](#5-check-the-machine)
- [6. Run one slice](#6-run-one-slice)
- [7. Read the ending](#7-read-the-ending)
- [8. Fitting the recipe to your repo](#8-fitting-the-recipe-to-your-repo)
- [What this guide does not claim](#what-this-guide-does-not-claim)

## 1. What you need

- A git repository you are willing to let an agent edit, with a
  committed `package-lock.json` and a `test` script in `package.json`.
  The recipe's verifier runs `npm ci`, so a repo without a lockfile
  fails at step one, honestly and immediately.
- `typescript` among that repo's `devDependencies`. The recipe's type
  check is `npx tsc --noEmit`, and after `npm ci` that resolves to
  `node_modules/.bin/tsc` — your lockfile's pinned copy, no network.
  Without a local install `npx` resolves the name outward to the
  registry instead, which is not what you want a gate seat doing. A
  plain-JavaScript repo should drop the type-check step from
  `roles/verifier.md` and `roles/implementer.md` rather than let it
  reach out; see [section 8](#8-fitting-the-recipe-to-your-repo).
- `node` and `npm` on your `PATH`. The recipe is wired for npm because
  npm ships with Node — nothing to install before the first seat can
  work. pnpm and yarn are a documented fork, not a second bundle; see
  [section 8](#8-fitting-the-recipe-to-your-repo).
- One agent CLI on your `PATH` — `claude`. The gate seats require it,
  for a reason [section 3](#3-give-your-repo-the-four-files) explains.
- `git`.

Know what you are granting. Every seat's driver may run `npm` and `npx`,
so every seat — the gates included — executes third-party code by
design: `npm ci` runs the `preinstall`/`install`/`postinstall` scripts of
your whole dependency tree, and `npx` resolves a package from the
registry and runs it. That is the JavaScript toolchain, not something
this recipe adds, and it is the same exposure your CI already has. Two
consequences worth holding: `verify` installs before `review` has read
the diff, so a dependency the `implement` seat added has already run its
install scripts by the time anyone reviews its provenance (the reviewer
charter names lockfile provenance and install scripts as a review
dimension for exactly this reason); and a run wants the network the same
way `npm ci` does. Run it against a repository whose dependency tree you
would install by hand.

## 2. Install the binary

Grab the archive for your platform from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest),
verify it against `SHA256SUMS`, unpack it, and put `brokkr` on your
`PATH` — the same three commands as the
[quickstart](quickstart.md#1-install). Nothing about this step is
Node-specific.

## 3. Give your repo the four files

Brokkr resolves its data relative to the **workspace** it is invoked in
— your repository root. Four things live there:

```
your-node-repo/
├── recipes/node/          # the recipe: bundle.json, policy.json, roles/
├── adapters/claude.json   # which drivers your world trusts
├── realms.json            # the world this invocation opens
└── .gitignore             # + a line for .forge/
```

**The adapters, first.** Clone this repository somewhere and copy
`adapters/claude.json` out of it into `adapters/` in your repo:

```
git clone https://github.com/feedback-loop-ai/brokkr ~/src/brokkr
cd your-node-repo
mkdir -p adapters && cp ~/src/brokkr/adapters/claude.json adapters/
```

This is not boilerplate, and it is not optional. Decision 0021 makes a
driver's **trust tier** operator-granted data, and `verify`, `review`
and `ship` are declared `class: "gate"` — a gate requires a driver
holding the trusted tier, checked when the bundle compiles, before any
prompt exists. The tier is a ruling *your* world makes, in your own
`adapters/` tree, which is why it is a file you own rather than a
constant in the engine.

**The recipe, second.** Install it from the local path:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
added recipe 'node' (66a30b26ed1c) at recipes/node
```

`recipes add` compiles what it copied and removes it again if it does
not compile, so a recipe that lands is a recipe that runs — which is
exactly why the adapters go first. Run it the other way around and you
get a refusal and no recipe:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
error: recipe 'node' does not compile (removed): bundle: adapters
./adapters: No such file or directory …; this bundle names an agent,
seats a gate, or declares a secret binding
```

**The `.gitignore` line.** `.forge/` is run-local evidence — the
journal, the task framing, one typed result file per seat attempt, the
shipper's ledger. It is never committed:

```
echo '.forge/' >> .gitignore
```

## 4. Write your `realms.json`

A **realms map** is the world an invocation opens (decision 0023): the
repositories it may see, and the one journal they share. A single
project is the degenerate map with one entry, and pays nothing for the
shape. Write this at your repository root:

```json
{
  "schema": "forge.realms/v1",
  "realms": [
    {
      "name": "my-app",
      "path": ".",
      "default_branch": "main"
    }
  ],
  "journal": ".forge/forge.db"
}
```

Three fields per realm, all required: a `name` (lowercase, the key every
per-realm fact is recorded under), a `path` (relative to the map file,
so the map travels with the workspace), and the `default_branch` this
realm's work is measured against. Paths and the `journal` are relative
to the map file's own directory. The schema refuses unknown fields —
`contracts/realms.v1.schema.json` is frozen, and a later addition
arrives as a new version, never as drift.

`brokkr run` and the read surfaces default to `./realms.json` when there
is one. Check what yours says:

```
$ brokkr realms
map      ./realms.json
journal  ./.forge/forge.db
realm    my-app  .  main  a1b2c3d
```

## 5. Check the machine

```
$ brokkr doctor --bundle recipes/node
ok       contracts: engine 0.5.0, event_schema 1, database_schema 1, driver_protocol 1
ok       git: git version 2.51.0
ok       claude: 2.1.252 (Claude Code) · serves fable, haiku, opus, sonnet
warn     agents: agents: agent library agents: No such file or directory
ok       database: .forge/forge.db opens (WAL, append-only triggers)
ok       bundle: 'node' compiles, digest 66a30b26ed1c…
```

Two lines are worth reading closely:

- The `agents` warning is **expected and harmless**. `recipes/node`
  adopts no agent from the shared library (decision 0016): its seats
  carry an inline `role` and `driver`, so there is no `agents/` tree for
  your repo to have. A warning is an optional capability, not a refusal.
- `brokkr recipes list` will likewise warn that `./bundles/self` and
  `./bundles/verify` are missing. Those are *this* repository's own
  bundles, looked for by default; your repo has no reason to carry them.

`brokkr doctor` runs no agent, so it is the cheapest way to find out
that `claude` is missing before a run does.

## 6. Run one slice

Pick something small and real — a bug with a reproducible failure, one
endpoint, one component. The feature text IS the framing: this recipe
has no intake seat to interrogate you, so write two or three sentences
that would let a new colleague start.

```
$ brokkr run --recipe node --repo . \
    --feature "cache the /health probe result for 5s; add a test that proves the second call does not hit the DB"
run started: cache-the-health-probe-result-fo-3f21a9c4
…
```

`--recipe node` resolves against `--recipes-dir` (default `recipes`).
`--repo .` is the working directory the seats run in and commit to — it
is also the fallback, but typing it makes the command readable six
months later. The run drives real agent sessions in the foreground;
watch it from a second terminal with `brokkr watch --run latest` or
`brokkr tui`, both read-only.

What the four seats will do to your repository:

| Seat | Class | What it runs | Commits? |
|---|---|---|---|
| `implement` | work | `npm ci`, writes code and tests, `npx tsc --noEmit`, `npm test` | yes |
| `verify` | gate | `npm ci`, `npx tsc --noEmit`, `npm test`, and `npm run lint` if you declare one | no — fixes nothing |
| `review` | gate | reads the diff for correctness, simplicity and security | only small, safe fixes, which force a re-verify |
| `ship` | gate | writes `.forge/ledger/<run-id>.md`, confirms the tree is clean | no |

Nobody pushes, nobody merges, and nobody publishes — no `npm publish`,
no `npm version`, no tag. That authority is yours.

## 7. Read the ending

`brokkr run` exits **0** at `done`, **2** when it parks for you, **3**
when it stops, **1** on an error. Then ask the run what happened:

```
brokkr inspect --run latest     # the ruling, the seats, the costs
brokkr tui                      # the fleet, keyboard-navigable
```

A stop is a result, not a failure: `VERIFY-FAIL` means your suite was
red and the machine refused to review red code; `REVIEW-SECURITY-HOLD`
means a reviewer found something high or critical and no path to `done`
exists past it. The `review` phase is constitutionally protected — the
compiler refuses any recipe with a path to a non-`stop` terminal that
bypasses it, so there is no flag that ships around a review.

Everything the run wrote is under `.forge/`: `tasks/<slug>.md` (the
framing the seats were handed), `results/<effect_id>.json` (one typed
result per seat attempt — the only channel the engine reads),
`ledger/<run-id>.md` (the close-out), and `forge.db` (the journal).

## 8. Fitting the recipe to your repo

The recipe is yours once copied; these are the three edits that actually
come up.

**A different package manager.** One table in
[`recipes/node/README.md`](../../recipes/node/README.md) names every
swap point for pnpm and yarn: the `--allowedTools` list in each seat's
driver, and the install/type-check/test commands plus the lockfile name
in the four charters. There is deliberately no second bundle to keep in
sync.

**Your repo's own scripts.** If your suite is `npm run test:ci`, or your
type check is `npm run typecheck`, edit `roles/verifier.md` and
`roles/implementer.md` to say so. A charter naming a script that exists
beats a charter naming a command a seat has to guess at. A repository
with no TypeScript at all deletes the `npx tsc --noEmit` step from both
charters for the same reason — one step short beats a step that resolves
to something the repo never installed.

**Limits.** `max_attempts` is 2 everywhere; the timeouts (4800s to
implement, 2400s to verify, 3000s to review, 1200s to ship) assume a
JavaScript suite rather than a cold Rust build. A monorepo whose
`npm ci` alone runs ten minutes should raise them — they are seat data
in `bundle.json`, not law.

Every edit moves the recipe's digest, which is the point: a strategy is
identified by its bytes, and a run records which bytes it ran.

## What this guide does not claim

`recipes/node` compiles under the shipped adapters, and its gate
refusals are tested against invented fixture providers
(`crates/brokkr-runtime/tests/node_recipe_gates.rs`). That is the
evidence this repository holds for it.

It has **not** been run end to end against a live Node codebase from
here. When that happens it will be a run with a journal, and the journal
will be what says so — not a sentence in a guide.

## See also

- [`recipes/node/README.md`](../../recipes/node/README.md) — the
  recipe's own notes: the package-manager fork, the limits, why every
  gate seats an agent driver.
- [quickstart.md](quickstart.md) — the general tour, with the escape
  hatches (`retry`, `stop`, `resume`, `rerun`) and what a run costs.
- [recipe-authoring.md](recipe-authoring.md) — the anatomy of the files
  you just copied, and how to compose rather than fork.
- [decision 0021](../decisions/0021-model-policy.md) — work seats, gate
  seats, and the trust tier that decides who may judge.
- [decision 0023](../decisions/0023-realms.md) — the realms map.
