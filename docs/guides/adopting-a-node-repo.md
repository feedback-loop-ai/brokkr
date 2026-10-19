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
- [The files your repo needs](#the-files-your-repo-needs)
- [Your `realms.json`](#your-realmsjson) — the map a library writes for
  itself
- [What each seat runs](#what-each-seat-runs)
- [Fitting the recipe to your repo](#fitting-the-recipe-to-your-repo)
- [What this guide does not claim](#what-this-guide-does-not-claim)

Zero really is zero on the Brokkr side: there is nothing to `npx`, no
package to add to your `devDependencies`, no postinstall hook. Brokkr is
one native binary written in Rust (decision 0009) that stands *outside*
your project and drives it. Your `package.json` never learns it exists.
The verifier script stays in the installed recipe: under the `namespace`
boundary Brokkr mounts that bundle read-only in its exec box, and under
`harness` it runs the pinned script from the bundle unboxed, so adopting
the recipe requires no separate script copy into the repository.

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
  `roles/verify-seat.sh` and `docs/house-rules.md` rather than let it
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
- **A run wants the network the same way `npm ci` does.** The unboxed
  implement seat has it. The verify seat's hands grant none, so under
  `namespace` its `npm ci` runs in the box from the bound `~/.npm` cache;
  under `harness` it runs unboxed, network and all.

Run it against a repository whose dependency tree you would install by
hand.

## The files your repo needs

**Two routes.** A repository with a `.brokkr/` folder, from the
quickstart's step 2, keeps named recipes on the folder's shelf,
`.brokkr/recipes/<name>/` (decision
[0083](../decisions/0083-an-adopters-brokkr-lives-in-one-folder.md)'s
addendum of 2026-10-16). `brokkr recipes add` installs there from
anywhere in the repository, with `fast`, the base `node` extends, and
copies the agents both seat, with their charters, into the folder.
`node` and `fast` are Brokkr's two portable recipes (decision
[0087](../decisions/0087-the-base-offices-are-the-ones-init-writes.md)
ruling 5): their agents extend the `implementer` and `reviewer` that
`init` wrote, so the install compiles against the folder's own offices
and adapters:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
added recipe 'node' (…) at /home/you/my-app/.brokkr/recipes/node
added recipe 'fast' (…) at /home/you/my-app/.brokkr/recipes/fast
added charter charters/implementer-intakeless.md of agent 'fast-implementer' at /home/you/my-app/.brokkr/agents/charters/implementer-intakeless.md
added charter charters/node-implementer.md of agent 'node-implementer' at /home/you/my-app/.brokkr/agents/charters/node-implementer.md
added charter charters/node-reviewer.md of agent 'node-reviewer' at /home/you/my-app/.brokkr/agents/charters/node-reviewer.md
added agent 'fast-implementer' at /home/you/my-app/.brokkr/agents/fast-implementer.json
added agent 'fast-reviewer' at /home/you/my-app/.brokkr/agents/fast-reviewer.json
added agent 'node-implementer' at /home/you/my-app/.brokkr/agents/node-implementer.json
added agent 'node-reviewer' at /home/you/my-app/.brokkr/agents/node-reviewer.json
```

Then `brokkr run --recipe node --feature "…"` from anywhere in the
repository, and the folder's map and journal serve it.

The rest of this guide is the other route, which makes your
repository a recipe library. A `recipes/<name>/bundle.json` at the root
is a library's mark, and a library keeps its own layout (the operator's
ruling of 2026-10-13): `realms.json`, `adapters/`, `agents/`,
`capabilities/` and `recipes/` at the root, and run state in `.forge/`,
with no `.brokkr/` folder and no `init` to write its base agents, so it
takes Brokkr's agent library whole, by hand.

Brokkr resolves its data relative to the **workspace** it is invoked in
— your repository root:

```
your-node-repo/
├── recipes/node/          # the recipe, and recipes/fast/, the base it extends
├── adapters/claude.json   # which drivers your world trusts
├── agents/                # the agent library the recipes' seats hire from
├── capabilities/          # the definition of each capability those agents request
├── realms.json            # the world this invocation opens
├── docs/house-rules.md    # this repository's conventions
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

(`brokkr init` scaffolds a `.brokkr/adapters/claude.json` for you
instead, and writes nothing into a library, so the library route takes
its adapters by hand: a tier is an operator's ruling, not a
scaffold's.)

**The agents and their capability definitions, second.** `node`'s and
`fast`'s seats hire agents from the library in `agents/` (#360), and an
agent that requests a capability needs its abstract definition at
`capabilities/<name>.json` (decision 0065). `recipes add` copies the
agents a recipe's seats name from `~/src/brokkr/agents/`, their
charters, and the definition of each capability they request from
`~/src/brokkr/capabilities/`, but never an agent's own base, and
`fast-implementer` extends `implementer`. With only the adapters copied,
the compile refuses, and the install removes everything it copied,
`recipes/` included:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
error: recipe 'node' does not compile (removed): bundle: bundle: agent 'fast-implementer' (/home/you/my-app/agents/fast-implementer.json) extends 'implementer', which is not an agent in this library (composed: node -> fast)
```

So copy Brokkr's agent library whole, and its definitions with it:

```
cp -r ~/src/brokkr/agents ~/src/brokkr/capabilities .
```

The definitions are not optional once the library is: a compile reads
every agent in it, and Brokkr's `researcher` requests `web-fetch` and
`web-search`. Copy `agents/` alone and the compile names each one
missing:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
error: recipe 'node' does not compile (removed): bundle: bundle: agent 'researcher': capability 'web-fetch' has no abstract definition at 'capabilities/web-fetch.json' in the operator configuration; declare its classes before requesting it; agent 'researcher': capability 'web-search' has no abstract definition at 'capabilities/web-search.json' in the operator configuration; declare its classes before requesting it (composed: node -> fast)
```

**The recipe, third.** Install it from the local path:

```
$ brokkr recipes add ~/src/brokkr/recipes/node --name node
added recipe 'node' (…) at /home/you/my-app/recipes/node
added recipe 'fast' (…) at /home/you/my-app/recipes/fast
```

`node` extends `fast`, and a base resolves from the library its recipe
sits in, so `recipes add` copies `fast` into `recipes/` beside it, at
your repository's root from any directory in it, unless your library
already holds a `fast`. Every agent, charter and definition it would
copy is already in your library, byte for byte, so it copies none of
them again; one that differed would refuse the install, naming each, and
replace nothing of yours. It compiles what it copied and removes all of
it again if any does not compile, so a recipe that lands is a recipe
that runs, and the adapters go first for that reason: a gate they do not
vouch for refuses the compile.

**The `.gitignore` line.** `.forge/` is run-local evidence — the
journal, the task framing, one typed result file per seat attempt, the
shipper's ledger. It is never committed:

```
echo '.forge/' >> .gitignore
```

## Your `realms.json`

Where `brokkr init` writes the map into its folder, a library writes its
own. The long version: a **realms map** is the world an invocation
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

One kind of `doctor` line is worth knowing before you run the spine's
check: **most `agent` lines warn, and that is expected.** Brokkr's
library names models that only its other adapters map, so with
`adapters/claude.json` alone each such agent warns that no adapter maps
its model; the agents `node` hires, `node-implementer` and
`node-reviewer`, print `ok`. A warning is not a refusal.
`brokkr doctor --bundle recipes/node` compiles the recipe as part of the
check, which is worth the extra flag here.

## What each seat runs

Then the spine's step 3, with `--recipe node`, since a library has no
folder to deliver under:

```
brokkr run --recipe node --repo . \
  --feature "cache the /health probe result for 5s; add a test that proves the second call does not hit the DB"
```

`--recipe node` resolves against `--recipes-dir` (default `recipes` in
a library, and the shelf, `.brokkr/recipes`, where a folder is found).
This recipe has **four** seats, not the scaffold's five: there is no
intake seat to interrogate you, so the feature text IS the framing —
write two or three sentences that would let a new colleague start.

| Seat | Class | What it runs | Commits? |
|---|---|---|---|
| `implement` | work | `npm ci`, writes code and tests, `npx tsc --noEmit`, `npm test` | yes |
| `verify` | gate | `npm ci`, `npx tsc --noEmit`, `npm test`, and `npm run lint` if you declare one | no — fixes nothing |
| `review` | gate | reads the diff for correctness, simplicity and security | no, by charter — reports findings for the implementer; a commit parks as `GATE-MOVED-HEAD`, an uncommitted edit is not caught |
| `ship` | gate | writes `.forge/ledger/<run-id>.md`, confirms the tree is clean | no |

No seat's charter pushes, merges or publishes — no `npm publish`, no
`npm version`, no tag — and that authority is yours. The implement and
review seats run unboxed with `Bash(git:*)` and `Bash(npm:*)`
pre-approved, so it is their charters and your own credentials and
settings, not the tool list, that keep those commands unrun.

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
swap point for pnpm and yarn: the `--allowedTools` list in the implement
and review drivers, and the install/type-check/test commands plus the lockfile name
in the house rules and verifier script. There is deliberately no second bundle to keep in
sync. For bun, [cards/bun.md](cards/bun.md) names the same three
command swaps.

**Your repo's own scripts.** If your suite is `npm run test:ci`, or your
type check is `npm run typecheck`, edit `roles/verify-seat.sh` and
`docs/house-rules.md` to say so. A house naming a script that exists
beats a house naming a command a seat has to guess at. A repository
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
