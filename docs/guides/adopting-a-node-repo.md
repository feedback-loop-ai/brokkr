# Adopting a Node repo — the long form of flow 3

This is [quickstart.md](quickstart.md)'s **flow 3** for a Node/TypeScript
repository driven by [`recipes/node`](../../recipes/node/), the reference
recipe for a JavaScript stack.

**Read the spine first.** Install, `doctor`, `brokkr run` and reading the
journal are the quickstart's four steps and are not repeated here — they
are not Node-specific and re-deriving them would only give you a second
copy to keep true. What is Node-specific is on this page:

- [What you are granting](#what-you-are-granting) — read this before
  anything else
- [The five files your repo needs](#the-five-files-your-repo-needs)
- [Your `realms.json`](#your-realmsjson) — flow 3's one added step
- [What each seat runs](#what-each-seat-runs)
- [Fitting the recipe to your repo](#fitting-the-recipe-to-your-repo)
- [What this guide does not claim](#what-this-guide-does-not-claim)

Zero really is zero on the Brokkr side: there is nothing to `npx`, no
package to add to your `devDependencies`, no postinstall hook. Brokkr is
one native binary written in Rust (decision 0009) that stands *outside*
your project and drives it. Your `package.json` never learns it exists.
The verifier script stays in the installed recipe: Brokkr mounts that
bundle read-only in its exec box, so adopting the recipe requires no
separate script copy into the repository.

You will need, beyond the spine's requirements:

- A committed `package-lock.json` and a `test` script in
  `package.json`. The recipe's verifier runs `npm ci`, so a repo without
  a lockfile fails at step one, honestly and immediately.
- `typescript` among that repo's `devDependencies`. The recipe's type
  check is `npx tsc --noEmit`, and after `npm ci` that resolves to
  `node_modules/.bin/tsc` — your lockfile's pinned copy, no network.
  Without a local install `npx` resolves the name outward to the
  registry instead, which is not what you want a gate seat doing. A
  plain-JavaScript repo should drop the type-check step from
  `roles/verify-seat.sh` and `roles/implementer.md` rather than let it
  reach out; see [below](#fitting-the-recipe-to-your-repo).
- `node` and `npm` on your `PATH`. The recipe is wired for npm because
  npm ships with Node — nothing to install before the first seat can
  work. pnpm, yarn and bun are a documented fork, not a second bundle.

## What you are granting

Every seat's driver may run `npm` and `npx`, so every seat — the gates
included — executes third-party code by design: `npm ci` runs the
`preinstall`/`install`/`postinstall` scripts of your whole dependency
tree, and `npx` resolves a package from the registry and runs it. That
is the JavaScript toolchain, not something this recipe adds, and it is
the same exposure your CI already has.

Two consequences worth holding:

- **`verify` installs before `review` has read the diff.** A dependency
  the `implement` seat added has already run its install scripts by the
  time anyone reviews its provenance. The reviewer charter names
  lockfile provenance and install scripts as a review dimension for
  exactly this reason.
- **A run wants the network the same way `npm ci` does.**

Run it against a repository whose dependency tree you would install by
hand.

## The five files your repo needs

Brokkr resolves its data relative to the **workspace** it is invoked in
— your repository root:

```
your-node-repo/
├── recipes/node/          # the recipe: bundle.json, policy.json, roles/
├── adapters/claude.json   # which drivers your world trusts
├── realms.json            # the world this invocation opens
├── docs/house-rules.md     # this repository's conventions
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

(This is the only respect in which flow 3 differs from `brokkr init`,
which scaffolds an `adapters/claude.json` for you — and refuses to
overwrite one that is already there, because a tier is an operator's
ruling and not a scaffold's.)

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

## Your `realms.json`

This is flow 3's one added step, and the quickstart states it in five
lines. The long version: a **realms map** is the world an invocation
opens (decision 0023) — the repositories it may see, and the one journal
they share. A single project is the degenerate map with one entry, and
pays nothing for the shape. Write this at your repository root:

```json
{
  "schema": "forge.realms/v3",
  "realms": [
    {
      "name": "my-app",
      "path": ".",
      "default_branch": "main",
      "house": "docs/house-rules.md"
    }
  ],
  "journal": ".forge/forge.db"
}
```

Three fields per realm remain required: a `name` (lowercase, the key every
per-realm fact is recorded under), a `path` (relative to the map file,
so the map travels with the workspace), and the `default_branch` this
realm's work is measured against. Paths and the `journal` are relative
to the map file's own directory. The v3 map adds optional `house` and
`dialect` fields without changing v1 or v2. Copy
[the Node house starter](starters/node-house-rules.md) to
`docs/house-rules.md` and tailor its commands to the scripts your repository
actually exposes. A named house is required to exist; `brokkr doctor` reports
it as a realm defect before a seat starts. The engine pins its digest and
renders its text between the portable charter and run context.

`brokkr run` and the read surfaces default to `./realms.json` when there
is one. Check what yours says:

```
$ brokkr realms
map      ./realms.json
journal  ./.forge/forge.db
realm    my-app  .  main  a1b2c3d
```

Two `doctor` lines are worth knowing before you run the spine's check:

- **The `agents` warning is expected and harmless.** `recipes/node`
  adopts no agent from the shared library (decision 0016): its seats
  carry an inline `role` and `driver`, so there is no `agents/` tree for
  your repo to have. A warning is an optional capability, not a refusal.
- **`brokkr recipes list` will warn** that `./bundles/self` and
  `./bundles/verify` are missing. Those are *this* repository's own
  bundles, looked for by default; your repo has no reason to carry them.

`brokkr doctor --bundle recipes/node` compiles the recipe as part of the
check, which is worth the extra flag here.

## What each seat runs

Then the spine's step 3, with `--recipe node` instead of `--bundle .`:

```
brokkr run --recipe node --repo . \
  --feature "cache the /health probe result for 5s; add a test that proves the second call does not hit the DB"
```

`--recipe node` resolves against `--recipes-dir` (default `recipes`).
This recipe has **four** seats, not the scaffold's five: there is no
intake seat to interrogate you, so the feature text IS the framing —
write two or three sentences that would let a new colleague start.

| Seat | Class | What it runs | Commits? |
|---|---|---|---|
| `implement` | work | `npm ci`, writes code and tests, `npx tsc --noEmit`, `npm test` | yes |
| `verify` | gate | `npm ci`, `npx tsc --noEmit`, `npm test`, and `npm run lint` if you declare one | no — fixes nothing |
| `review` | gate | reads the diff for correctness, simplicity and security | only small, safe fixes, which force a re-verify |
| `ship` | gate | writes `.forge/ledger/<run-id>.md`, confirms the tree is clean | no |

Nobody pushes, nobody merges, and nobody publishes — no `npm publish`,
no `npm version`, no tag. That authority is yours.

The spine's step 4 reads the ending, and its stop rulings mean what they
say here too: `VERIFY-FAIL` means your suite was red and the machine
refused to review red code; `REVIEW-SECURITY-HOLD` means a reviewer
found something high or critical and no path to `done` exists past it.
The `review` phase is constitutionally protected — the compiler refuses
any recipe with a path to a non-`stop` terminal that bypasses it, so
there is no flag that ships around a review.

## Fitting the recipe to your repo

The recipe is yours once copied; these are the three edits that actually
come up.

**A different package manager.** One table in
[`recipes/node/README.md`](../../recipes/node/README.md) names every
swap point for pnpm and yarn: the `--allowedTools` list in each seat's
driver, and the install/type-check/test commands plus the lockfile name
  in the implementer charter and verifier script. There is deliberately no second bundle to keep in
sync. For bun, [cards/bun.md](cards/bun.md) names the same three
command swaps.

**Your repo's own scripts.** If your suite is `npm run test:ci`, or your
type check is `npm run typecheck`, edit `roles/verify-seat.sh` and
`roles/implementer.md` to say so. A charter naming a script that exists
beats a charter naming a command a seat has to guess at. A repository
with no TypeScript at all deletes the `npx tsc --noEmit` step from both
files for the same reason — one step short beats a step that resolves
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

- [quickstart.md](quickstart.md) — the spine this page is a delta over.
- [cards/node.md](cards/node.md) — the same delta at card length.
- [starters/node.md](starters/node.md) — what `brokkr init` writes for
  a Node repo, if you would rather scaffold than adopt.
- [`recipes/node/README.md`](../../recipes/node/README.md) — the
  recipe's own notes: the package-manager fork, the limits, why every
  gate seats an agent driver.
- [recipe-authoring.md](recipe-authoring.md) — the anatomy of the files
  you just copied, and how to compose rather than fork.
- [decision 0021](../decisions/0021-model-policy.md) — work seats, gate
  seats, and the trust tier that decides who may judge.
- [decision 0023](../decisions/0023-realms.md) — the realms map.
