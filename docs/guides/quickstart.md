# Quickstart — one spine, and everything else is a diff over it

You need a git repository you are willing to let an agent edit, and one
agent CLI on your `PATH` (`claude`, `codex` or `dsh`). Everything else is one
native binary — no Python, no Node, no services.

Everything in this guide is **four steps**. They are stated once, below,
and nothing on this page repeats them:

| | Step | Budget |
|---|---|---|
| 1 | [Install](#step-1--install) — verified release tarball | 60s |
| 2 | [`brokkr init .`](#step-2--brokkr-init-) — scaffold a recipe you can read | — |
| 3 | [`brokkr run`](#step-3--brokkr-run) — one slice | 5min to a first effect |
| 4 | [Read the journal](#step-4--read-the-journal) — `brokkr inspect` | — |

Then three things extend the spine rather than restating it:

- **[Per-stack cards](#per-stack-cards)** — what changes in steps 2 and
  3 for your language. A card is a handful of lines, never a second
  walkthrough.
- **[Flow 2 — deliver](#flow-2--deliver)** — the spine plus one step.
- **[Flow 3 — adopt](#flow-3--adopt)** — the spine plus one step.

**About the two budgets.** They are measured, not claimed.
[`scripts/bootstrap-bench.sh`](../../scripts/bootstrap-bench.sh) times
both paths on a clean tempdir and exits non-zero when either blows, and
it runs as the `bootstrap-budgets` jobs in CI, one per host. Read what it does *not*
measure before you trust a number: it prints that itself, and
[§ what the budgets do not cover](#what-the-budgets-do-not-cover) says
it here.

---

## The spine

### Step 1 — install

Pick the row for your machine. Every one of them installs the *same*
binary: the package managers below serve the release's own attested
artifacts, never a second build of their own — see
[packaging/README.md](../../packaging/README.md) for how that is held
together.

| Channel | One line | State |
|---|---|---|
| tarball | `tar xzf brokkr-linux-x86_64.tar.gz` (full recipe below) | works today, and it is the path the 60-second budget measures |
| cargo | `cargo binstall brokkr-cli` | live from v0.9.1 — the release workflow publishes the seven crates to crates.io at each tag, and binstall then reads the attested release asset |
| nix | `nix profile install github:feedback-loop-ai/brokkr` | live from v0.9.0 — the release renders the flake digests and opens their pull request; nix reads the default branch once it is merged |
| apt | `sudo apt-get install brokkr` | live from v0.9.0 — a signed repository on GitHub Pages, after the one-time keyring and source lines below |
| dnf | `sudo dnf install brokkr` | live from v0.9.0 — same site, same signature, after the one-time repo file |
| brew | `brew install feedback-loop-ai/tap/brokkr` | live from v0.9.0 — the release bumps the tap and the operator merges it |

On Windows, use WSL2: it is Linux, and every Linux row above serves it (decision [0063](../decisions/0063-windows-is-not-a-host.md)).

Every row above was exercised by a real release: v0.9.0 (2026-09-05)
lit the apt and dnf site, the tap and the flake, and v0.9.1
(2026-09-06) put the seven crates on crates.io. Each channel serves the
same attested assets as the tarball. The apt and dnf rows need their
one-time setup lines (the keyring and the sources entry, or the repo
file); both are in [packaging/README.md](../../packaging/README.md).

The rest of this step is the tarball path, which needs none of that.

Grab the archive for your platform from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
(linux x86_64/aarch64, macOS arm64/x86_64), verify it
against the release's `SHA256SUMS`, then unpack:

```
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/brokkr-linux-x86_64.tar.gz
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS      # brokkr-linux-x86_64.tar.gz: OK
tar xzf brokkr-linux-x86_64.tar.gz            # → ./brokkr
```

Every archive and the `SHA256SUMS` manifest carry a signed GitHub
Sigstore build-provenance attestation, so the checksum itself can be
checked against the workflow that produced it:

```
gh attestation verify brokkr-linux-x86_64.tar.gz -R feedback-loop-ai/brokkr
```

Put the binary somewhere on your `PATH`. The rest of this guide assumes
plain `brokkr`.

Every shipped bundle declares hands on its verify and ship gates, and
what stands between a seat's hands and your machine is the realm's
**boundary** (decision
[0046](../decisions/0046-the-boundary-is-named.md), accepted
2026-09-05) — one word in `realms.json` beside `house` and `dialect`,
never in a bundle. The default, and what every bundle meant before the
word existed, is `namespace`: decision 0043's empty-root box built by
bubblewrap 0.11 or newer on `PATH` (`bwrap --version`), on Linux and
WSL2. That boundary is never simulated: a run under it refuses at start
when bubblewrap is unavailable, naming the seats that need it. A realm
on macOS declares `"boundary": "harness"` today, under
`forge.realms/v4`, and then Brokkr builds no box at all: a model gate
with hands is judged under the harness's own sandbox as the adapter's
`hands.harness` fragment addresses it, a model seat without hands —
every model seat of the claude scaffold `brokkr init` writes — is
launched with nothing of Brokkr's or its harness's added around it, an
exec gate runs the bundle's
own pinned script with the environment cleared, and every readout —
the run header, `brokkr seats`, the TUI, the web console and the
delivery gate's check summary on the pull request — renders such a run
*unboxed*. `open`, nothing at all, is the fifth word; `seatbelt` (the
same box built by macOS's `sandbox-exec`) and `container` (a pinned
image) are named by the decision and refuse at start until slices (ii)
and (iii) build them. `brokkr doctor` prints one `boundaries` line
saying which this machine offers.

Which shipped bundles run under `harness` today is a fact of the tree,
not a promise: the twelve whose hands sites are their own `./` exec gates
compile — `recipes/fast`, `recipes/landing`, `recipes/standby`, `recipes/node`, `recipes/preflight`,
`recipes/review-first`, `recipes/research`, `recipes/research-dsh`, `recipes/wager-harness`,
`recipes/wager-harness-dsh`, `recipes/wager-harness-muse` and
`recipes/verify`. Six refuse, each naming the ground the compiler
reaches first: `recipes/self` and `recipes/panel-review` because their
reviewer's chain reaches claude and `adapters/claude.json` declares no
`hands.harness.gate` until the operator's measurement lands (see
[provider adapters](provider-adapters.md)); `recipes/triage` and
`recipes/night-shift`, and `recipes/gpt-flash`, which extends `triage`,
because their `analyze:check` dialect step is an exec gate whose argv is
not the bundle's own script, refused under `harness` and `open` until a
decision admits it — and, once
it is, `triage` and `night-shift` still on the claude ground until that
measurement lands; and `recipes/release`, whose `implement` office
declares hands and reaches claude without a measured `hands.harness.work`
fragment before compilation reaches the review gate. The pin test in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs` is the record of
that split. Of these recipes only `fast` and `node` are portable; the
rest are Brokkr-only (decision 0087 ruling 5).

```text
$ brokkr --version
brokkr 0.12.0
```

> **Building from a checkout** is a fallback, not a co-equal option: it
> needs Rust 1.88 or newer and a compile, and it produces a binary no
> attestation covers. If you want it:
>
> ```
> cargo install --path crates/brokkr-cli    # builds and installs the binary
> ```
>
> This is the path for people changing Brokkr, not for people using it.

**Before you go on**, spend the cheapest thirty seconds available:

```text
$ brokkr doctor
warn     folder: none from /home/you/my-app up to /home/you/my-app — brokkr init . writes one
ok       contracts: engine 0.12.0, event_schema 1, database_schema 1, driver_protocol 1
…
warn     adapters: adapters: adapters adapters: No such file or directory (os error 2)
…
warn     agents: agents: agent library agents: No such file or directory (os error 2)
warn     database: none: no .brokkr/ folder from /home/you/my-app up to /home/you/my-app, and /home/you/my-app is not a recipe library (decision 0083); nothing was created: run `brokkr init .` from /home/you/my-app
warn     capabilities native: the adapter declarations at adapters could not be read (adapters adapters: No such file or directory (os error 2)), so no harness's native capabilities are named below; that is NOT a finding that an installed harness has none
ok       capabilities <unmapped>: no realms map, so no capability grants are declared; every native capability is governed by the no-grant default — switched off, or the seat is refused
ok       house rules: no realms map; none declared
ok       dialect: no realms map; none declared
```

That is a real run at the root of a fresh `git init` copy of the Rust
fixture, before step 2, the repository's absolute path shown as
`/home/you/my-app`; each `…` stands for lines your machine decides: its
`git`, the boundaries and boxes it can build, and the tools it carries.

Lines are prefixed `ok`, `warn`, or `MISSING`. A `MISSING` line is a
refusal to guess: an absent driver binary means seats resolving to that
provider will fail to spawn, and doctor says so rather than letting you
find out mid-run. Warnings are optional capabilities. `doctor` executes
no agent. Dialect lines compare the installed specification tool with the
realm's pin and check every file its dialect requires; a missing dialect tool
warns that the design route will refuse without making the whole doctor fail.
A repository with no folder has no adapters, agents or realm map to
read, so its provider and agent lines, and a line for each dialect the
realm declares, appear once step 2 has written the folder: run `brokkr
doctor` again then.
The first line is always the folder: the `.brokkr/` folder doctor found
and from where, `none` before step 2 has written one, or why it cannot
read one. Once the folder stands, it is doctor's bundle: its agents,
providers, capabilities and `bundle` line come from the folder's one
compile, and a folder that does not compile says so on the folder line.
Four flags, each outranking the folder: `--bundle <dir>` compiles that
bundle instead, `--db <path>` chooses the workspace journal (default:
the map's journal, else `.brokkr/state/forge.db`), `--realms <path>`
selects another realm map, and `--secrets-file <path>` names the secrets
store (default `.brokkr/state/secrets.env`) so doctor can say which
declared credentials a route takes from the ambient environment instead.

### Step 2 — `brokkr init .`

A **recipe** is a delivery strategy as reviewable data: a phase table and a
seat per phase. Model offices carry an agent definition with their charter,
model chain and tool grant; deterministic offices carry an exec script,
which Brokkr boxes under the `namespace` boundary and nowhere else.
`brokkr init` writes one you are meant to open and edit.

```text
$ brokkr init .
initialized /home/you/my-app/.brokkr (digest …), writing:
  .brokkr/.gitignore
  .brokkr/adapters/claude.json
  .brokkr/adapters/exec.json
  .brokkr/agents/README.md
  .brokkr/agents/charters/implementer.md
  .brokkr/agents/charters/intake.md
  .brokkr/agents/charters/reviewer.md
  .brokkr/agents/implementer.json
  .brokkr/agents/intake.json
  .brokkr/agents/reviewer.json
  .brokkr/bundle.json
  .brokkr/policy.json
  .brokkr/realms.json
  .brokkr/scripts/ship-seat.sh
  .brokkr/scripts/verify-seat.sh
Scaffolded for `claude`, the first of claude, codex and dsh found on PATH.
…
Decision 0065 rules that a seat never inherits your own MCP configuration, but this build does not enforce it yet: a seat may still start the MCP servers your harness configuration names.
…
commit the folder: git add .brokkr && git commit
```

That is a real run, at the root of a fresh `git init` copy of
the Rust fixture
[`init-stacks/rust/`](../../crates/brokkr-cli/tests/fixtures/init-stacks/rust/),
with `claude` on `PATH`; the repository's absolute path is shown as
`/home/you/my-app`, and the digest is elided. The two `…` lines are
your machine's: on macOS a sentence follows the first note saying
`realms.json` declares the `harness` boundary, and on Linux without
bubblewrap a warning before the last line says the `namespace` boundary
needs it.

`init` takes the directory as a **positional argument**, not a flag, and
that directory must be the root of its repository, because every verb
reads the folder only there (decision
[0083](../decisions/0083-an-adopters-brokkr-lives-in-one-folder.md)).
Everything it writes goes into one folder, `.brokkr/`, and nothing else
in your repository is touched: your own `README.md`, `.gitignore`,
`scripts/`, `agents/` and `policy.json` stay yours. It refuses, writing
nothing, where it would not leave a folder discovery reads:

- a directory below the root names the root: `` <dir> is inside the
  repository <root> but is not its root (decision 0083); a folder is
  read only at the root: run `brokkr init <root>` ``;
- a directory that does not exist is never created: `reading <dir>: No
  such file or directory (os error 2)`;
- a `.brokkr/` that already holds anything but a `.gitignore` and
  `state/` names what it holds: `<root>/.brokkr already holds adapters,
  agents, … (decision 0083); init writes a folder only where none stands,
  or beside moved state (.gitignore and state/ alone): move or remove
  them`;
- a recipe library, a repository with a `recipes/<name>/bundle.json`,
  keeps its own layout, and `init` writes nothing there.

It compiles the folder before printing the digest, so what you were
handed is a thing that runs, and it prints every path it wrote.

What the folder holds:

```
.brokkr/.gitignore           # one line, state/: what a run writes stays out of git
.brokkr/bundle.json          # five seats: three model offices and two exec gates, boxed under namespace only
.brokkr/policy.json          # forge.phase-machine/v1, seven phases, nineteen rules
.brokkr/realms.json          # the realm map: realm path "..", journal "state/forge.db", and the boundary where one is declared
.brokkr/adapters/claude.json # the trust tier your gates judge on, and the tool map — yours to edit
.brokkr/adapters/exec.json   # the deterministic exec driver
.brokkr/agents/README.md     # what was written, and which tools the seats were granted
.brokkr/agents/intake.json   # model offices: charter, model chain, tool grant, limits
.brokkr/agents/implementer.json
.brokkr/agents/reviewer.json
.brokkr/agents/charters/intake.md
.brokkr/agents/charters/implementer.md
.brokkr/agents/charters/reviewer.md
.brokkr/scripts/verify-seat.sh # detected test and lint commands; under namespace, boxed with no network
.brokkr/scripts/ship-seat.sh   # deterministic ledger and closeout
.brokkr/dialects/              # only in a spec-kit or OpenSpec repository: the detected dialect's pinned data
.brokkr/state/                 # not written by init: the journal and everything a run writes, ignored
```

`init` scaffolds for the first agent CLI on `PATH`: `claude`, `codex`
or `dsh`, and `claude` when none is found. `--cli claude`, `--cli codex`
or `--cli dsh` names the one to scaffold for instead, where several share
a directory on `PATH`; a named CLI that `PATH` lacks is scaffolded for all
the same, and `init` says to install it. A codex scaffold writes
`.brokkr/adapters/codex.json` in place of `.brokkr/adapters/claude.json`, hires every
model seat from codex, and on Linux leaves `.brokkr/realms.json` at the
default `namespace` boundary, so each model seat runs codex read-only
beside Brokkr's workspace tool, whose commands, `git commit` among them,
run in Brokkr's box. On macOS codex's own sandbox holds each model seat's
hands instead — read-only for the review gate, workspace-write for
intake and implement. dsh cannot hold the
review gate, because its adapter is untrusted and names no judges. A dsh
scaffold therefore hires intake and implement from dsh and keeps the
reviewer on claude, and says so. On macOS `.brokkr/realms.json` declares
`harness` whatever the CLI, because the default `namespace` boundary
needs Linux bubblewrap 0.11 or newer. Under `harness`, verify and ship
run their pinned scripts under no box of Brokkr's: the engine rebuilds
their environment, and no network denial is confirmed. The scaffold's
`.brokkr/agents/README.md` and the note a passing verify journals follow the
boundary `init` declared: a `namespace` scaffold's note says the
network was denied, and a `harness` scaffold's says "passed unboxed,
with no network denial confirmed"
([security model](../security-model.md)).
Commit the folder before the first run, with the line `init` ends on,
`git add .brokkr && git commit`: the ship gate closes out only on a
clean tree, and `.brokkr/.gitignore` keeps what a run writes under
`.brokkr/state/` out of it. From then on every verb finds the folder
from anywhere in the repository, walking up to the git root.

**Coming from the old layout.** A repository an earlier `brokkr init`
scaffolded, with `bundle.json` or `.forge/` at its root, is refused by
every verb before anything is opened, and the refusal lists the moves
into `.brokkr/` (decision 0083 ruling 4): `this repository has Brokkr's
old root layout and no .brokkr/ folder (decision 0083); nothing was
opened, and Brokkr moves nothing`. Brokkr moves nothing itself; make
the moves, by hand or with an agent, and `brokkr doctor` checks the
result. A run that has not ended is ended first, through the one escape,
a journal named on the command line, which reads the old layout whole:
`brokkr conclude --run <id> --reason "…" --db .forge/forge.db`.

The table has five working phases — `intake`, `implement`, `verify`,
`review`, `ship` — plus the two terminals `done` and `stop`. `review` is
the protected phase: compilation rejects any table with a path to a
non-`stop` terminal that skips it. Each seat declares its class — work
or gate (decision 0021 ruling 1) — and its result vocabulary. Intake,
implement and review name agents; verify and ship name exec scripts.
Limits remain on every seat, while the model chain and tool grant belong
only to agent-backed seats. `brokkr agents show <name>` reads an agent back.

**The seats are granted the tools their charters name.** The same
detection below decides what the seats are pre-approved to *run*: the
binary each command invokes (`cargo`, `bun`, `pnpm`, …) plus `git`,
`ls`, `rg` and `mkdir` go into the adapter's `tool_permissions.names` as
`Bash(<bin>:*)` entries, and each model agent's `tools.allow` names them —
the whole set for the work seats and the same set without `mkdir` for
review. That smaller set does not make the review gate read-only: it runs
unboxed under `acceptEdits`, and the engine checks only that it left HEAD
where it found it. The
verify and ship gates are scripts with no model grant. A
repository `init` does not recognize gets `git`, `ls`, `rg` and `mkdir`
alone, NO runner, and a README that says so, rather than a guessed
permission. The grant is claude's
`--allowedTools`, which pre-approves and removes no other tool: an
unboxed seat also runs under the operator's own Claude Code permission
settings. Decision 0065 makes its MCP servers the engine's to compose,
never the operator's, but that is not enforced yet: until strict MCP
isolation lands, a seat may still start the servers in the operator's
own harness configuration, and each adapter's `mcp` records what its
harness was measured to exclude ([security model](../security-model.md)).

**`init` looks before it scaffolds.** The repository you ran it from is
read for the manifests and lockfiles at its root, and the implementer
charter and verifier script name *that stack's* build, test and lint
commands, quoting back which files the guess came from. Nothing is
executed to find out. Which files it reads, and what it concludes, is
your [card](#per-stack-cards) — and
[`docs/guides/starters/`](starters/) shows what `init` wrote for each,
transcribed from real runs.

`init` prints the digest in full; it is elided above because it is a
function of what was scaffolded, and two repositories with different
stacks get different charters and tool maps and so different digests.

If you would rather start from a maintained strategy than a scaffold,
Brokkr's own repository is a recipe library that ships several. A
library keeps its own layout, with no `.brokkr/` folder, and from a
checkout of it:

```
$ brokkr recipes list
fast	e0c1eb0757cd	6 phases	implement, review, ship, verify	recipes/fast
node	7abfe34bba48	6 phases	implement, review, ship, verify	recipes/node
panel-review	1bf81bd7249f	7 phases	implement, intake, review[correctness+security], ship, verify	recipes/panel-review
triage	<digest>	12 phases	analyze[check>judge], clarify[check>judge], design[positions>chief>validate], implement{cases}, review{cases}, ship, specify[author>validate], tasks[author>validate], triage, verify[checks>dialect-verify]	recipes/triage
…
```

The twelve hex characters after the name are the leading bytes of the
recipe's content digest. Of the library's recipes, `fast` and `node` are
portable and the rest Brokkr-only, as
[Try another recipe](#try-another-recipe) says. Use `--recipe <name>`
instead of a bundle to run one of these; `--recipes-dir` says where the library lives (in a
library, `recipes`), so a library is used from its own root, or named:
`brokkr run --recipe <name> --recipes-dir <library>/recipes`. A
repository with a `.brokkr/` folder keeps its own named recipes on the
folder's shelf, `.brokkr/recipes/<name>/`, the default of every recipe
verb and of `--recipe` there: see [Try another recipe](#try-another-recipe).

### Step 3 — `brokkr run`

```
$ brokkr run --feature "prefix selectors for the read surfaces"
note: the map /home/you/my-app/.brokkr/realms.json found in this workspace is adopted (journal unchanged); --realms names it explicitly
run started: prefix-selectors-for-the-read-su-…
…
```

That is a real run, with one stand-in: the `claude` CLI is the stub
[`scripts/bootstrap-bench.sh`](../../scripts/bootstrap-bench.sh) puts
first on `PATH` through the adapter's own `BROKKR_CLAUDE_BIN`, which
answers each model seat at once and writes no code. The repository is a
fresh `git init` holding a `Makefile` whose `build`, `test` and `lint`
pass, with Step 2's folder committed, so the scaffold's own verify and
ship gates run for real and the run reaches `done`. Its id ends in a
hash, elided here, and the last `…` stands for what your host prints
after it.

With the folder committed, `brokkr run` needs only `--feature <text>`,
from anywhere in the repository: it walks up to the git root, finds
`.brokkr/` there, and takes the bundle, the map, the journal, the agents,
the adapters and the secrets store from it. Every flag still outranks
the folder, each for its own fact (decision 0083 ruling 2), and
`--bundle <dir>` and `--recipe <name>` may not both be given. The
defaults, with the folder and in a recipe library, which has none:

| Flag | What it does | With `.brokkr/` | In a recipe library |
|---|---|---|---|
| `--bundle <dir>`, `--recipe <name>` | The bundle the run delivers under. | `.brokkr/` | none: name one, or the run refuses and names `brokkr init .` |
| `--repo <path>` | The working directory seats run in and commit to. | The repository root the folder stands in. | The process's current directory. |
| `--db <path>` | The workspace journal. Outranks a realms map's journal. | The map's journal, else `.brokkr/state/forge.db`. | The map's journal, else `.forge/forge.db`. |
| `--realms <file>` | The world's map: repositories and the one journal they share. | `.brokkr/realms.json` when there is one. | `./realms.json` when there is one. |
| `--recipes-dir <path>` | Where `--recipe` names are resolved. | `.brokkr/recipes` | `recipes` |
| `--secrets-file <path>` | The operator-side secrets store, for seats with declared bindings. | `.brokkr/state/secrets.env` | `.forge/secrets.env` |
| `--dispatch <file>` | A canonical `forge-dispatch/v2` JSON, for Looper-bound runs. | none | none |

A map named explicitly and missing or malformed is a refusal, never a
silent fallback. A repository with neither a folder nor a library's
`recipes/<name>/bundle.json` has no place for a journal, so a run there
refuses before it creates anything: ``no .brokkr/ folder from <cwd> up to
<root>, and <root> is not a recipe library (decision 0083); nothing was
created: run `brokkr init .` from <root>``.

#### A personal map lives outside the repository

A map that is yours, not the repository's, lives outside the worktree,
beside the personal recipe shelf: `$XDG_CONFIG_HOME/brokkr/realms/<name>.json`,
else `$HOME/.config/brokkr/realms/<name>.json` (decision
[0101](../decisions/0101-personal-realms-and-adapters-live-outside-the-tracked-tree.md)
ruling 1). Nothing finds it by lookup. `--realms <path>` names it, and
the named map is read alone: it is never merged with the project's map,
and it is not a fallback when it is missing or malformed — that refuses.
If both exist, only the named one is a candidate.

`--realms` is on every verb that opens a journal or a world today: `run`
and `queue add`; `resume` and `rerun`; `doctor`; `realms`; the readouts
`runs`, `inspect`, `transcript`, `seats`, `watch`, `costs`, `ledger`,
`compare`, `analytics` and `graph`; the operator verbs `anchor`, `replay`,
`conclude`, `operator`, `export`, `import` and `bridge`; `keep-refs
plant`, `keep-refs list` and `keep-refs delete`; `queue list`, `queue
judge`, `queue move`, `queue hold`, `queue release`, `queue repin` and
`queue drop`; `ui`, `tui` and `muninn run`. `compile` and `recipes show`
do not take it yet.

Moving a project map out of the repository is the migration 0101 rules.
Copy the project's map (`realms.json`, or a folder's
`.brokkr/realms.json`) to the personal path, then make its paths reach
this repository: a map's realm path and its journal (the world `journal`,
or a realm's own) are read relative to the map file's directory, so write
them as absolute paths, or as paths relative to the personal map's
directory that reach this repository — otherwise `.forge/forge.db` lands
under the personal directory. Keep `house` repository-relative — this repository's
`docs/house-rules.md` for its realm. A named dialect such as `openspec`
is looked up at
`<personal>/dialects/openspec.json`: either copy `dialects/openspec.json`
and its prompt directory beside the personal map, or write the path form
`dialects/openspec.json`, which resolves inside the realm and reads the
tracked file. A map that declares `capabilities` carries its
`capabilities/` and `dialects/tools/` trees beside itself too; crossings
need no move.

A personal map's paths and credentials never go into the tracked
`realms.json` or a bundle (ruling 8). A map field stores no secret value:
credentials stay names under the existing secret bindings (decision
[0012](../decisions/0012-sealed-secret-bindings.md)) in the
operator-side store, never in the map.

The run drives real agent sessions to completion in the foreground, and on
a terminal it shows the TUI's run view while it does: `q` closes the view
and leaves the run driving. Watch it from a second terminal too:

```
brokkr watch --run latest     # the same run view, live; frames off a terminal
brokkr tui                    # the fleet, navigable with the keyboard
brokkr ui --port 8383 --open  # loopback-only browser console
```

All three are read-only. None of them can issue an operator command or
start a run.

### Step 4 — read the journal

`brokkr run`'s exit code says whether the run reached `done`, parked
for the operator, stopped, or met an error — so a shell script can tell
them apart without parsing anything. The codes are
[one table](../reference/cli.md#exit-codes) in the CLI reference.

Then ask the run what happened:

```
$ brokkr inspect --run latest
run  prefix-selectors-for-the-read-su-…
     completed · phase done · seq 51
     boundary …
ruling  SHIP-COMPLETE  ship → done · shipped

seats
  model is the provider's claim, not proof · effort is configuration
  participant status    attempts turns cost tokens model          boundary       effort         activity
…
trail
   1 run/started        prefix selectors for the read surfaces…
   2 phase/entered      intake
   8 effect/succeeded   intake ·… · model not reported · boundary not applicable
   9 transition/decided INTAKE-OK intake → implement · resolved
…
  49 transition/decided SHIP-COMPLETE ship → done · shipped
  50 phase/entered      done
  51 run/completed      completed

graph
  intake ×1
    → intake · finished
…
  done ×1  ←current
```

That is Step 3's run, read back, with its seats and the middles of its
trail and graph elided, and the `boundary` line, which is your host's:
[the read surfaces](read-surfaces.md#brokkr-inspect--one-run-explained)
show the whole readout.

Five blocks, and each one answers a different question:

- **header** — the run id, its status (`running`, `awaiting_operator`,
  `completed`, `stopped`), the phase it is in, and the journal sequence
  number it has reached.
- **ruling** — the last `transition/decided`: which rule id fired, from
  which phase to which, on which typed result. When a run parked, this
  is where the park reason is.
- **seats** — per seat: attempts, turns and cost from the journal's
  checkpoints, the model the provider claimed, the boundary its hands
  stood behind, the effort, and the last activity; under a model seat,
  its transcript reference, the agent and the model it was seated by,
  and the effort pinned.
- **trail** — the numbered journal events. Every line is a sequence
  number and a fact. Nothing in it was written by a model: seats produce
  typed results, the table produces rulings.
- **graph** — phases with their visit counts (`×N`), and where the run
  currently is.

`--run` takes a selector, not only the 41-character id: any unique
run-id prefix, or `latest`. `--phase <name>` and `--seat <name>` narrow
the readout; `--json` emits the same view model verbatim, which is what
scripts should read.

**A run that ends at `stop` is a normal outcome, not a crash.** The
table stops runs on purpose: two consecutive broken implement attempts,
a failing verify, a security hold at review, a dirty worktree at ship.
The `ruling` line names the rule that did it and its reason.

That is the spine. Everything below is a diff over it.

---

## Per-stack cards

Step 2 writes different charters for different repositories, and step 3
has a different natural recipe. Nothing else about the four steps
changes. Each card is that difference and nothing more:

| Card | Reads | Then |
|---|---|---|
| [node](cards/node.md) | `package.json` | the base card the others extend |
| [bun](cards/bun.md) | `package.json` + `bun.lock` | extends node; overrides the package-manager lines |
| [rust](cards/rust.md) | `Cargo.toml` | + the workspace line |
| [go](cards/go.md) | `go.mod` | + the `go.work` line |
| [python](cards/python.md) | `pyproject.toml` (+ `uv.lock`) | uv-first, pip as fallback |

A monorepo is a card-level difference too: `turbo.json` or `nx.json` at
the root makes step 2 name the orchestrator's own commands, run through
whichever package manager your lockfile says. See
[cards/node.md § monorepos](cards/node.md#monorepos).

If you want the actual scaffold output rather than the delta,
[`starters/`](starters/) has one page per stack, each transcribed from a
real `brokkr init` run.

## Flow 2 — deliver

The spine, plus one step. Steps 1, 2 and 4 are unchanged; step 3
becomes:

> **3′.** Point `--feature` at a real task in your repository and let
> the run go to `done` or `stop`.
>
> ```
> brokkr run \
>   --feature "cache the /health probe for 5s; add a test proving the second call does not hit the DB"
> ```
>
> Pick something small and real — a bug with a reproducible failure, one
> endpoint, one component. The feature text IS the framing the intake
> seat starts from; two or three sentences that would let a new
> colleague start are the right size. Then read step 4's `ruling` line:
> a `stop` names the rule that did it, and the rules that stop runs are
> the ones you would want to stop them.

That is the entire delta. The escape hatches below are what you reach
for when 3′ ends somewhere other than `done`.

## Flow 3 — adopt

The spine, plus one step, for an existing repository whose recipe you
did not write. Steps 1, 3 and 4 are unchanged; after 2:

> **2′.** Read the map that says which repositories this world contains
> and which journal they share. `brokkr init` wrote it as
> `.brokkr/realms.json`:
>
> ```json
> {
>   "journal": "state/forge.db",
>   "realms": [
>     {
>       "default_branch": "main",
>       "name": "starter",
>       "path": ".."
>     }
>   ],
>   "schema": "forge.realms/v3"
> }
> ```
>
> Three fields per realm, all required, and paths are relative to the
> map file's own directory, `.brokkr/`, so the realm path `..` is the
> repository and the journal is `.brokkr/state/forge.db`. A single
> project is the degenerate one-entry map and pays nothing for the
> shape; rename the realm, or add the repositories it works beside.
> Check what yours says with `brokkr realms`.

Two things that are **not** deltas and must not be skipped when you
adopt a repository you did not write:

- **The adapters are yours, not a recipe's.** `verify`, `review` and
  `ship` are `class: "gate"`, and a gate requires a driver holding the
  trusted tier, checked when the bundle compiles, before any prompt
  exists. `brokkr init` declares that tier in
  `.brokkr/adapters/claude.json`, yours to read and edit. A maintained
  recipe brings no adapters with it: `brokkr recipes add` copies onto
  the folder the agents and charters a recipe names, never an adapter,
  and an install whose compile refuses is removed whole.
  [Try another recipe](#try-another-recipe) shows one that compiles.
- **Know what you are granting.** Every seat's driver may run your
  project's toolchain, so every seat — the gates included — executes
  third-party code by design. In a Node repo `npm ci` runs the
  `preinstall`/`install`/`postinstall` scripts of your whole dependency
  tree and `npx` resolves a package from the registry and runs it. Two
  consequences worth holding: `verify` installs *before* `review` has
  read the diff, so a dependency the implement seat added has already
  run its install scripts by the time anyone reviews its provenance —
  which is why the reviewer charter names lockfile provenance and
  install scripts as a review dimension — and a run wants the network
  the same way your CI does. The unboxed implement seat has it; a verify
  seat has it too unless the `namespace` box denies it to hands that grant
  none, and then its install reads a bound package cache. Run it against
  a dependency tree you would install by hand.

The long form, with the files a Node repository needs and the three
edits that actually come up, is
[adopting-a-node-repo.md](adopting-a-node-repo.md).

---

## After the spine

### Where the run wrote things

Under the folder's `state/`, at the repository root:

- `.brokkr/state/forge.db` — the journal, unless `--db` or a map says
  otherwise.
- `.brokkr/state/tasks/<slug>.md` — the intake seat's framing, run-local.
- `.brokkr/state/results/<effect_id>.json` — one typed result file per
  seat attempt. This file is the only channel the engine reads from a
  seat; anything a seat prints to stdout is not a result.
- `.brokkr/state/scratch/` — each seat's scratch directory.
- `.brokkr/state/ledger/<run-id>.md` — the shipper's close-out, if the
  run got that far.
- `.brokkr/state/secrets.env` — only if you used `brokkr secrets`.

`.brokkr/.gitignore`, which `init` wrote, keeps all of it out of git: it
is evidence, not source, and your own `.gitignore` needs no line. A
recipe library writes the same files under `.forge/` in the working
directory instead, and ignores that itself.

### Try another recipe

The folder's `bundle.json` is the default recipe. Named recipes stand
beside it on the folder's shelf, `.brokkr/recipes/<name>/`, which
`recipes list`, `recipes show` and `run --recipe` read with no flag from
anywhere in the repository (decision 0083's addendum of 2026-10-16). A
recipe is a bundle directory, so a variant of your own starts as a copy
of the folder's bundle, its policy and its scripts, edited outside the
folder; `brokkr recipes add` installs it onto the shelf, compiled
against the folder's agents and adapters first:

```
$ mkdir -p ~/variants/careful
$ cp .brokkr/bundle.json .brokkr/policy.json ~/variants/careful/
$ cp -r .brokkr/scripts ~/variants/careful/
$ brokkr recipes add ~/variants/careful --name careful
added recipe 'careful' (<digest>) at /home/you/src/app/.brokkr/recipes/careful
$ brokkr recipes list
careful	<digest>	7 phases	implement, intake, review, ship, verify			/home/you/src/app/.brokkr/recipes/careful
$ brokkr run --recipe careful --feature "prefix selectors for the read surfaces"
run started: …
```

Commit the shelf with the folder. The shelf never moves the folder
bundle's digest, though an agent `recipes add` copies (below) does,
because the folder pins its agents. `brokkr doctor` prints one line per
shelf recipe after its folder line, and `.brokkr/bundle.json` may itself
extend a shelf recipe with `"extends": "careful"`.

`recipes add` also copies, from the source's agent library, each agent
definition the recipe names that `.brokkr/agents/` lacks and the
charter that definition names, and the definition of each capability
those agents request. Definitions go where a compile reads them,
`capabilities/` beside the map, else at the root: the source's from the
root its library stands in, and into this repository's, which is
`.brokkr/capabilities/` beside the `realms.json` that `init` writes. It
refuses before writing anything when one already there differs, naming
each, and replaces nothing of yours.

Two of Brokkr's own recipes, `fast` and `node`, are **portable**: their
agents extend the `implementer` and `reviewer` that `init` writes and
write no chain, tools or hands of their own, so each model seat hires what
`init` wrote for the agent CLI it found. From a clone of Brokkr, in a
fresh folder:

```
$ brokkr recipes add ~/src/brokkr/recipes/fast --name fast
added recipe 'fast' (…) at /home/you/my-app/.brokkr/recipes/fast
added charter charters/implementer-intakeless.md of agent 'fast-implementer' at /home/you/my-app/.brokkr/agents/charters/implementer-intakeless.md
added agent 'fast-implementer' at /home/you/my-app/.brokkr/agents/fast-implementer.json
added agent 'fast-reviewer' at /home/you/my-app/.brokkr/agents/fast-reviewer.json
```

In a folder, `fast`'s verify and ship are the folder's own, the seats
`brokkr init` wrote (decision 0091,
[`recipes/fast/README.md`](../../recipes/fast/README.md)). The other
seventeen recipes in Brokkr's `recipes/` are **Brokkr-only**, written
for Brokkr's own runs and realm. Decision
[0087](../decisions/0087-the-base-offices-are-the-ones-init-writes.md)'s
ruling 5 gives each recipe's class and its reason.

### The escape hatches

#### Operator commands — `retry`, `stop`, `supersede` and `acknowledge`

```
brokkr operator --run <id> retry --reason "the flaky test passes on re-run"
brokkr operator --run <id> stop  --reason "requirements changed"
brokkr operator --run <id> supersede --reason "fixed on main" \
  --findings 41 --by-run <other id> --by-seq 97
brokkr operator --run <id> acknowledge --reason "history from an older engine"
```

The command is a **positional argument** and `--reason` is **required**;
`--db` defaults to the journal step 3 ruled: the map's, else
`.brokkr/state/forge.db`. There are four commands:
`retry` re-runs the current phase, `stop` ends the run, `supersede`
records that residual findings on a run that has already finished were
closed by another run's ruling (decision 0047), and `acknowledge`
answers a quarantined run, one whose journal does not load or does not
fold, so the fleet lists it under *older* instead of *needs you*
(decision 0076 item 11). `acknowledge` writes nothing to the run's
journal: the store keeps who, when and why beside it, at the run's last
event, and the acknowledgement lapses if the journal moves. It refuses a
run that folds, which `brokkr conclude` closes, and it does not travel
with `export` or `adopt`. `retry` and `stop` are
recorded as `operator/commanded` plus the engine's disposition —
`operator/accepted` when it lands, `operator/rejected` when it does not.
Approval is an entry in the record, not a prose convention, and so is a
refusal.

For a parked panel, `brokkr operator retry --member <tag>` runs only the
named members; repeat `--member` for several. A seat panel uses the member
name; a sequence panel uses `<step>:<member>` (for example,
`positions:correctness`). Settled verdicts are carried only when the reviewed
head, clean tree, pinned bundle and judged inputs still match. A refusal
writes nothing; plain `retry` re-runs the whole phase. With `--member`,
`--repo`, `--bundle`, `--recipe`, `--recipes-dir` and `--secrets-file` resolve
as on `resume`; give the same secrets store the run was driven with. The
journal and participant readouts name carried verdicts and their source
attempt (proposed decision 0093).

The command is **fenced**: the engine re-reads the run's state after the
`operator/commanded` lands and before it writes a disposition, because
an engine process driving the same run can conclude it or un-park it in
that window. A command the run can no longer take is refused — `retry`
anywhere but `awaiting_operator`, either command on a run that has
already finished — and `brokkr operator` then exits **1** and prints the
journaled reason. It never accepts a command the fold would refuse to
read back.

**There is no `park` command.** Parking is something the engine does,
never an operator verb. A run parks when the machine cannot rule:

- a seat result that fails schema validation — not an object, no
  `result` string, or a result outside the seat's declared vocabulary;
- an outcome the engine cannot establish as complete (`indeterminate`) —
  a driver that accepted the attempt and then exited without a result;
- a `(phase, result)` pair no rule matches, or a present input the
  condition vocabulary cannot read;
- a rule that explicitly rules a park (`"park": true`, added by
  `forge.phase-machine/v2`), such as `fast`'s `IMPL-BLOCKED-ENVIRONMENT`
  for an implementer its environment blocked (decision 0092). Such a park
  carries the seat's notes as its reason; fix what they name, then
  `retry` and `resume` run that seat again.

A parked run sits in `awaiting_operator` with the raw evidence attached
and leaves only through `retry` or `stop`.

#### Resume

```
brokkr resume --run <id>
brokkr resume --run <id> --recipe triage
```

Resume continues a parked or crashed run under **its exact pinned
bundle**. The run manifest holds a `sha256` per bundle file; a mismatch
is refused with a diagnostic rather than helpfully picking up your
edited files. If you changed the bundle, that is a new run, not a
resumed one.

`brokkr resume` takes `--bundle` or `--recipe` (at most one, and the
folder without either), `--run`, `--realms`, `--db`, `--repo` and
`--secrets-file`. The journal is the one `run` wrote: `--db` when typed,
else the journal the map names, else `.brokkr/state/forge.db` — so a run
started in the folder's world is resumed with no `--db`.

#### Conclude — closing a run whose bundle no longer compiles

```
brokkr conclude --run <id> --reason "the engine moved on without it"
```

The refusal above is right for `resume`, which spends money against a
pinned policy — but it also means a run journaled under an older engine,
or against a recipe file that has since changed, can never reach a
lawful ending at all. `brokkr conclude` is the other door: it opens the
journal, verifies the chain, folds the state, and appends **only** the
operator stop conclusion — the command and its acceptance, the
indeterminate close of any attempt that was in flight, and
`run/stopped` citing the operator by name. It takes no `--bundle` or
`--recipe`, compiles nothing, and spawns no effect, so it needs no
pinned recipe to be honest about what it wrote.

It refuses a run that is already concluded and refuses a broken hash
chain whole. It cannot retry: retrying re-enters the policy loop, and
the policy loop needs the bundle by construction — a parked run whose
operator wants to retry still needs a working `resume`. Exit codes are
the usual mapping, so a concluded run exits 3.

Every write it makes is **fenced**: the conclusion lands only on the
exact head it folded, so a journal that moves beneath it — something
still driving the run — makes `conclude` refuse with the look-first
instruction rather than close over live work. A refusal is evidence,
not an inconvenience: a moved head means the run is not dead, and
`conclude` is for a run believed dead. `resume` is fenced the same way
(decision 0029): what it decides on its fold, closing an attempt it
finds open included, lands only on the head that fold read, and a
moved journal ends it with the drift named. Either way, `brokkr runs`
is how you look before closing.

#### Re-run under another strategy

Not an escape hatch so much as the next experiment:

```
brokkr rerun --run <id> --recipe panel-review   # same feature, other strategy, new run id
brokkr compare <a> <b>                          # trails, first divergence, per-seat costs
```

The rerun compiles the new recipe in the discovered realm exactly as
`run` does: its manifest carries the realms pin, the dialect and the
boundary of the world it starts in, and it is judged against this
machine's `boundaries` line the same way.

### What it cost

```
$ brokkr costs --run latest
{
  "run_id": "prefix-selectors-for-the-read-su-…",
  "seats": {
    "implement": {
      "attempts": 1,
      "boundary": "not applicable",
      "cost_usd": 0.0,
      "effort": "not reported",
      "model": "not reported",
      "turns": 1
    },
…
  },
  "total_cost_usd": 0.0
}
```

Per seat: `attempts` counted from `effect/started` events, and `turns`
and `cost_usd` summed from the `num_turns` and `total_cost_usd` fields
the driver reported in its checkpoints, beside the `model`, `effort` and
`boundary` the journal records. Step 3's stub reports one turn and no
cost for each model seat. `--db` defaults to the journal
the map names, else `.brokkr/state/forge.db`; `--realms` names the map.

Be clear-eyed about this:

- **A run spawns real, billed agent sessions.** Every seat is a Claude
  Code, Codex or dsh session against your account. A five-phase recipe with
  retries is five or more sessions on a repository the agent is reading
  and editing.
- **Cost is only as complete as the harness reports.** A provider whose
  stream carries no usage totals contributes `0.0`, which means "not
  reported", not "free". The Codex adapter, for example, reports token
  counts rather than a dollar figure.
- **This repository publishes no per-run dollar figure**, and you should
  not infer one from the examples above — the numbers in them come from
  test shims. Run one slice, read `brokkr costs`, and use your own
  number.

### What the budgets do not cover

The 60s and 5min figures at the top of this page are what
[`scripts/bootstrap-bench.sh`](../../scripts/bootstrap-bench.sh)
measures on a clean tempdir, and they are honest about being partial.
The script prints this every run, and so does this page:

- **The install budget excludes the network.** The bench fetches the
  tarball and `SHA256SUMS` over a local `file://` URL, using the same
  `curl -LO`, `sha256sum -c` and `tar xzf` commands step 1 names. So 60s
  covers unpack, checksum verification and a `brokkr --version` smoke
  test — everything around the transfer, which is the part this
  repository controls. A pass is not evidence that GitHub is fast for
  you.
- **The first-run budget excludes the agent.** A real `brokkr run`
  spawns billed sessions, which cannot live inside a timing gate, so the
  bench stubs the `claude` binary through the adapter's own
  `BROKKR_CLAUDE_BIN` override. The bundle, the compile with its
  gate-class trust check, the driver transport and the journal are all
  real; only the session at the far end is not. So 5min is the
  *machinery's* cost to reach a first completed effect, and your slice's
  wall clock is your agent's, not ours.

Neither number is a claim about your machine. Run the script.

### Limits worth knowing

- **One journal per world.** A realms map names a set of repositories
  and exactly one `journal` they share (`forge.realms/v1`). There is no
  per-realm journal.
- **Three drivers per journal is the practical ceiling.** Every driver
  on a world writes the same journal, and SQLite's write lock is not a
  fair queue. On 2026-09-25, three or four drivers on one journal met a
  42 s wait against the 30 s patience (#394). A working seat's
  checkpoints are now held behind a peer's lock without waiting, so the
  seat never stalls. When the seat stops, the
  checkpoints still held get three patiences, its terminal event three,
  and the same outcome three more, so an attempt can wait nine
  patiences to settle. Each marker the engine journals for a panel
  member or a sequence step gets three of its own. The run carries on
  once the outcome lands. A lock that outlasts them all ends the engine
  with the attempt open, and the next `resume` settles it as restarted
  and parks.
  Nothing caps writers until decision 0068's dispatcher is built (#430).
- **`ui` serves one journal.** Every verb that opens a journal takes
  `--realms` and opens the journal the map names unless `--db` outranks
  it (#374), but `ui` refuses a world whose realms name several hearths
  until `--db` picks one; `tui` and `runs` read them all.
- **A Looper-dispatched run (`--dispatch`) cannot adopt agents and
  carries no realms map.** The v2 manifest lineage would silently drop
  both, so the engine refuses instead.
- **Column alignment is byte-based.** Colour follows `NO_COLOR` and
  `TERM`, width follows `COLUMNS`; without a Unicode-width dependency,
  CJK and emoji columns misalign in the readouts.

## Compact first-run tour

This is the compact tour that formerly lived on the front page.

One native binary — no Python, no Node, no services.

**Install from a release.** Grab the archive for your platform from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
(linux x86_64/aarch64, macOS arm64/x86_64), verify it
against the release's `SHA256SUMS`, then unpack:

```
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/brokkr-linux-x86_64.tar.gz
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS      # brokkr-linux-x86_64.tar.gz: OK
tar xzf brokkr-linux-x86_64.tar.gz            # → ./brokkr
```

Every archive and the `SHA256SUMS` manifest carry a signed GitHub
Sigstore build-provenance attestation, so the checksum itself can be
checked against the workflow that produced it:

```
gh attestation verify brokkr-linux-x86_64.tar.gz -R feedback-loop-ai/brokkr
```

**Or build it.** Rust 1.88 or newer:

```
cargo install --path crates/brokkr-cli    # builds and installs the binary
```

**Then deliver something.**

```
brokkr doctor                           # tools, agent CLIs, database, contracts
brokkr init .                           # scaffold a reviewable starter recipe into .brokkr/
git add .brokkr/ && git commit -m "the starter folder"
brokkr run --feature "prefix selectors for the read surfaces"
brokkr tui                              # explore what just happened
```

Steps 1 to 3 above show what `doctor`, `init .` and `run` print, run
for real; `tui` opens the fleet view Step 3 describes.

`brokkr run`'s [exit code](../reference/cli.md#exit-codes) tells a shell
script whether the run reached `done`, parked for the operator or
stopped, without parsing anything.

`brokkr init` writes a starter recipe you are meant to read: a seven-phase
policy table (five working phases plus `done` and `stop`) with the review
gate constitutionally protected, three model offices, and exec verify and
ship gates, boxed only under the `namespace` boundary, all of it in one
`.brokkr/` folder at the repository root. The agent files in
`.brokkr/agents/` carry each
model office's charter, model chain and tool grant; all seat limits remain
in `.brokkr/bundle.json`. It compiles the bundle before printing
the digest, so the thing you were handed is a thing that runs. It also
reads the repository you ran it from — the manifests and lockfiles at
that root, nothing executed — so the implementer's and verifier's
charter and verifier script name that stack's own build, test and lint
commands, and say
plainly when no stack was recognized. Lockfiles have the deciding vote
where a manifest is ambiguous (`bun.lock` out-votes the npm fallback,
`uv.lock` out-votes pip), a monorepo orchestrator (`turbo.json`,
`nx.json`) out-votes any single package's script and is run through
whichever package manager the root's lockfile names, and a Cargo
workspace or a `go.work` gets a charter that says so rather than a
command it did not need. The same stack decides what the seats are
pre-approved to RUN, not what else the operator's own Claude Code
settings allow them:
the binaries its commands invoke are written into the scaffold's
`.brokkr/adapters/claude.json` `tool_permissions.names` as `Bash(<bin>:*)`
entries and granted in the agents' `tools.allow` — the whole set to the
work seats, and the same set without `mkdir` (the test runner's tools
plus `git`, `ls` and `rg`) to the review gate's agent, which that smaller
set does not make read-only; verify and ship are scripts with no grant. A repository no row
recognizes gets that base set and NO runner, and a scaffold README that
says so, because a tool name is a permission and one guessed is one
granted. The digest
is therefore a function of what was scaffolded and differs from
repository to repository; the one printed above is elided for that
reason. [`docs/guides/starters/`](starters/) shows the
actual output per stack, transcribed from real runs.

## Next

- [starters/](starters/) — what `brokkr init` actually wrote, per
  stack, transcribed from real runs.
- [cards/](cards/) — the per-stack deltas over this spine.
- [recipe-authoring.md](recipe-authoring.md) — write or extend a
  delivery strategy.
- [driver-authoring.md](driver-authoring.md) — put a harness that is not
  Claude Code, Codex or dsh behind a seat.
- [versioning.md](versioning.md) — what is stable, what may still move.

### Re-drive finished branch work at a gate

Start a fresh run with `brokkr run --recipe fast --feature "land the finished branch" --enter verify`.
Add `--continues <run>` for each earlier concluded native run whose work you
are continuing; ids, unique prefixes and `latest` use the usual selector.
After `run started: <id>`, stderr prints the entered gate and each continued
run's title and id. The new run rejudges the branch and keeps its own results.
Work-phase entries refuse before a run is created. This option belongs to
`run`; `queue add` cannot encode it in `queued-launch/v1` (proposed decision 0094).
