# Quickstart — one spine, and everything else is a diff over it

You need a git repository you are willing to let an agent edit, and one
agent CLI on your `PATH` (`claude` or `codex`). Everything else is one
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
it runs as the `bootstrap-budgets` job in CI. Read what it does *not*
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
| cargo | `cargo binstall brokkr-cli` | **wired at the bench** — binstall reads the crate through crates.io, and no workflow publishes `brokkr-cli` there yet |
| nix | `nix profile install github:feedback-loop-ai/brokkr` | works from the first release after this slice, once that release's flake-digest pull request is merged — nix reads the default branch |
| apt | `sudo apt-get install brokkr` | **wired at the bench** — needs the signing secret and the Pages site |
| dnf | `sudo dnf install brokkr` | **wired at the bench** — same |
| brew | `brew install feedback-loop-ai/tap/brokkr` | **wired at the bench** — needs the tap repository |
| scoop | `scoop bucket add brokkr https://github.com/feedback-loop-ai/scoop-bucket && scoop install brokkr` | **wired at the bench** — needs the bucket repository |

"Wired at the bench" means exactly what it says: the tooling is in this
repository and tested in CI, and the last step — a repository secret, a
Pages site, a sibling repository — belongs to the operator. Nothing
above is written as if it had been run against a live channel when it
had not. The apt and dnf rows also need their one-time setup line (the
keyring and the sources entry); both are in
[packaging/README.md](../../packaging/README.md).

The rest of this step is the tarball path, which needs none of that.

Grab the archive for your platform from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
(linux x86_64/aarch64, macOS arm64/x86_64, windows x86_64), verify it
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

```
$ brokkr --version
brokkr 0.6.0
```

> **Building from a checkout** is a fallback, not a co-equal option: it
> needs Rust 1.88 or newer and a compile, and it produces a binary no
> attestation covers. If you want it:
>
> ```
> cargo install --path crates/brokkr-cli    # installs the `brokkr` binary
> ```
>
> This is the path for people changing Brokkr, not for people using it.

**Before you go on**, spend the cheapest thirty seconds available:

```
$ brokkr doctor
ok       contracts: engine 0.6.0, event_schema 1, database_schema 1, driver_protocol 1
ok       git: git version 2.51.0
ok       claude: 2.1.252 (Claude Code) · serves fable, haiku, opus, sonnet
warn     exec: binary 'sh' not found — seats resolving to this provider will fail to spawn · serves no abstract model yet
ok       agent implementer: would run opus via claude here (chain opus → sonnet)
…
```

Lines are prefixed `ok`, `warn`, or `MISSING`. A `MISSING` line is a
refusal to guess: an absent driver binary means seats resolving to that
provider will fail to spawn, and doctor says so rather than letting you
find out mid-run. Warnings are optional capabilities. `doctor` executes
no agent. Two flags: `--bundle <dir>` also compiles a bundle and reports
the result, and `--db <path>` chooses the workspace journal (default
`.forge/forge.db`). It takes no `--realms`.

### Step 2 — `brokkr init .`

A **recipe** is a delivery strategy as reviewable data: a phase table, a
seat per phase, and — since decision 0016 — an agent per seat, where
the charter, the model chain, the per-seat limits and the tool grant
live. `brokkr init` writes one you are meant to open and edit.

```
$ brokkr init .
initialized reviewable bundle at . (digest 4a0f568f35fd6efec2fc66574651c3d786fbfcf54fcdc2bb34a247f0fcf426c9)
run brokkr from inside . — its adapters/ and agents/ declare the trust tier and the tool grants its seats run under
```

`init` takes the directory as a **positional argument**, not a flag. It
refuses rather than overwriting a directory that already has a
`bundle.json`, an `adapters/claude.json`, or an agent definition under
`agents/` — a trust tier and a tool grant are operator rulings, not a
scaffold's — and it compiles the bundle before printing the digest, so
what you were handed is a thing that runs.

What it wrote:

```
./bundle.json          # name, policy path, protected_phase, five seats — each names an agent
./policy.json          # forge.phase-machine/v1, seven phases, nineteen rules
./adapters/claude.json # the trust tier your gates judge on, and the tool map — yours to edit
./agents/README.md     # what was written, and which tools the seats were granted — your own README is untouched
./agents/intake.json   # one agent per seat: charter, model chain, tool grant, limits
./agents/implementer.json
./agents/verifier.json
./agents/reviewer.json
./agents/shipper.json
./agents/charters/intake.md
./agents/charters/implementer.md
./agents/charters/verifier.md
./agents/charters/reviewer.md
./agents/charters/shipper.md
```

The table has five working phases — `intake`, `implement`, `verify`,
`review`, `ship` — plus the two terminals `done` and `stop`. `review` is
the protected phase: compilation rejects any table with a path to a
non-`stop` terminal that skips it. Each seat declares its class — work
or gate (decision 0021 ruling 1) — and its result vocabulary, and names
the agent that fills it; the agent carries the charter, the `limits`
(attempts and a deadline in seconds), the model chain and the tool
grant, and `brokkr agents show <name>` reads one back.

**The seats are granted the tools their charters name.** The same
detection below decides what the seats may *run*: the binary each
command invokes (`cargo`, `bun`, `pnpm`, …) plus `git`, `ls`, `rg` and
`mkdir` go into the adapter's `tool_permissions.names` as
`Bash(<bin>:*)` entries, and each agent's `tools.allow` names them —
the whole set for the work seats, the read-only subset (the test
runner's tools, `git`, `ls`, `rg` — never `mkdir`) for the gates. A
repository `init` does not recognize gets an EMPTY map and a README
that says so, rather than a guessed permission.

**`init` looks before it scaffolds.** The repository you ran it from is
read for the manifests and lockfiles at its root, and the implementer's
and verifier's charters name *that stack's* build, test and lint
commands, quoting back which files the guess came from. Nothing is
executed to find out. Which files it reads, and what it concludes, is
your [card](#per-stack-cards) — and
[`docs/guides/starters/`](starters/) shows the actual output for each,
transcribed from real runs.

The digest above is printed in full and will not be yours: it is a
function of what was scaffolded, and two repositories with different
stacks get different charters and tool maps and so different digests.

If you would rather start from a maintained strategy than a scaffold,
the library ships several:

```
$ brokkr recipes list
fast	6324f76f7bfa	6 phases	implement, review, ship, verify	recipes/fast
node	ed3c623bceaa	6 phases	implement, review, ship, verify	recipes/node
panel-review	39bb61a43c1c	7 phases	implement, intake, review[correctness+security], ship, verify	recipes/panel-review
sdd	ed604f45bfce	8 phases	design[positions>chief>speckit-check], implement, intake, review[security+spec-compliance], ship, verify	recipes/sdd
…
```

The twelve hex characters after the name are the leading bytes of the
recipe's content digest. Use `--recipe <name>` instead of `--bundle
<dir>` to run one of these; `--recipes-dir` (default `recipes`) says
where the library lives.

### Step 3 — `brokkr run`

```
$ brokkr run --bundle . --repo . --feature "prefix selectors for the read surfaces"
run started: prefix-selectors-for-the-read-su-8bf6d692
…
```

`brokkr run` requires **exactly one** of `--bundle <dir>` or `--recipe
<name>` — clap enforces the group — and `--feature <text>`. The rest:

| Flag | What it does | Default |
|---|---|---|
| `--repo <path>` | The working directory seats run in and commit to. | The process's current directory. There is no flag default; the engine falls back to the cwd it was launched from. |
| `--db <path>` | The workspace journal. Outranks a realms map's journal. | With neither `--db` nor a map, `.forge/forge.db`. |
| `--realms <file>` | The world's map: repositories and the one journal they share. | `./realms.json` when there is one. A map named explicitly and missing or malformed is a refusal, never a silent fallback. |
| `--recipes-dir <path>` | Where `--recipe` names are resolved. | `recipes` |
| `--secrets-file <path>` | The operator-side secrets store, for seats with declared bindings. | `<workdir>/.forge/secrets.env` |
| `--dispatch <file>` | A canonical `forge-dispatch/v2` JSON, for Looper-bound runs. | none |

`--repo .` is worth typing even though the cwd is the fallback: it makes
the working directory explicit in the command someone reads six months
later.

The run drives real agent sessions to completion in the foreground.
Watch it from a second terminal:

```
brokkr watch --run latest     # the same readout, redrawn as the journal head moves
brokkr tui                    # the fleet, navigable with the keyboard
brokkr ui --port 8383 --open  # loopback-only browser console
```

All three are read-only. None of them can issue an operator command or
start a run.

### Step 4 — read the journal

`brokkr run` exits **0** when the run reaches `done`, **2** when it
parks for the operator, **3** when it stops, and **1** on an error — so
a shell script can tell them apart without parsing anything.

Then ask the run what happened:

```
$ brokkr inspect --run latest
run  prefix-selectors-for-the-read-su-8bf6d692
     completed · phase done · seq 38
ruling  SHIP-COMPLETE  ship → done · shipped

seats
  participant status    attempts turns cost activity
  intake      succeeded 1        —     —    resolved · 0s
  implement   succeeded 1        —     —    complete · 0s
  verify      succeeded 1        —     —    pass · 0s
  review      succeeded 1        —     —    clean · 0s
  ship        succeeded 1        —     —    shipped · 0s

trail
   1 run/started        prefix selectors for the read surfaces…
   2 phase/entered      intake
   7 effect/succeeded   intake · resolved
   8 transition/decided INTAKE-OK intake → implement · resolved
  …
  36 transition/decided SHIP-COMPLETE ship → done · shipped
  37 phase/entered      done
  38 run/completed      completed

graph
  intake ×1
    → intake · finished
  …
  done ×1  ←current
```

Four blocks, and each one answers a different question:

- **header** — the run id, its status (`running`, `awaiting_operator`,
  `completed`, `stopped`), the phase it is in, and the journal sequence
  number it has reached.
- **ruling** — the last `transition/decided`: which rule id fired, from
  which phase to which, on which typed result. When a run parked, this
  is where the park reason is.
- **seats** — per seat: attempts, turns and cost from the journal's
  checkpoints, and the last activity.
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
> brokkr run --bundle . --repo . \
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
did not write. Steps 1, 3 and 4 are unchanged; between 1 and 2:

> **1′.** Write the `realms.json` that says which repositories this
> world contains and which journal they share.
>
> ```json
> {
>   "schema": "forge.realms/v1",
>   "realms": [
>     { "name": "my-app", "path": ".", "default_branch": "main" }
>   ],
>   "journal": ".forge/forge.db"
> }
> ```
>
> Three fields per realm, all required, and paths are relative to the
> map file's own directory so the map travels with the workspace. A
> single project is the degenerate one-entry map and pays nothing for
> the shape. Check what yours says with `brokkr realms`.

Two things that are **not** deltas and must not be skipped when you
adopt a repository you did not write:

- **The adapters are yours, not the scaffold's.** If you take a
  maintained recipe (`brokkr recipes add …`) rather than `brokkr init`,
  no `adapters/` tree arrives with it, and `verify`, `review` and `ship`
  are `class: "gate"` — a gate requires a driver holding the trusted
  tier, checked when the bundle compiles, before any prompt exists. Copy
  `adapters/claude.json` out of this repository into yours first, or
  `recipes add` refuses and leaves you no recipe.
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
  the same way your CI does. Run it against a dependency tree you would
  install by hand.

The long form, with the four files a Node repository needs and the three
edits that actually come up, is
[adopting-a-node-repo.md](adopting-a-node-repo.md).

---

## After the spine

### Where the run wrote things

Relative to the working directory (`--repo`, or the cwd):

- `.forge/forge.db` — the journal, unless `--db` or a map says otherwise.
- `.forge/tasks/<slug>.md` — the intake seat's framing, run-local.
- `.forge/results/<effect_id>.json` — one typed result file per seat
  attempt. This file is the only channel the engine reads from a seat;
  anything a seat prints to stdout is not a result.
- `.forge/ledger/<run-id>.md` — the shipper's close-out, if the run got
  that far.
- `.forge/secrets.env` — only if you used `brokkr secrets`.

Add `.forge/` to your `.gitignore`. It is evidence, not source.

### The escape hatches

#### Operator commands — `retry` and `stop`

```
brokkr operator --run <id> retry --reason "the flaky test passes on re-run"
brokkr operator --run <id> stop  --reason "requirements changed"
```

The command is a **positional argument** and `--reason` is **required**;
`--db` defaults to `.forge/forge.db`. There are exactly two commands:
`retry` re-runs the current phase, `stop` ends the run. Both are
recorded as `operator/commanded` plus the engine's disposition —
`operator/accepted` when it lands, `operator/rejected` when it does not.
Approval is an entry in the record, not a prose convention, and so is a
refusal.

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
  `forge.phase-machine/v2`).

A parked run sits in `awaiting_operator` with the raw evidence attached
and leaves only through one of the two commands above.

#### Resume

```
brokkr resume --run <id> --bundle .
brokkr resume --run <id> --recipe sdd
```

Resume continues a parked or crashed run under **its exact pinned
bundle**. The run manifest holds a `sha256` per bundle file; a mismatch
is refused with a diagnostic rather than helpfully picking up your
edited files. If you changed the bundle, that is a new run, not a
resumed one.

`brokkr resume` takes `--bundle`/`--recipe` (exactly one), `--run`,
`--db` (default `.forge/forge.db`), `--repo` and `--secrets-file`. It
takes **no `--realms`**: a run started in a mapped world whose journal
is not `.forge/forge.db` is resumed by naming that journal with `--db`.

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
`conclude` is for a run believed dead. `resume` still carries the
unfenced hazard on its fresh-process branch; decision 0029 (proposed)
rules on fencing that tail. Either way, `brokkr runs` is how you look
before closing.

#### Re-run under another strategy

Not an escape hatch so much as the next experiment:

```
brokkr rerun --run <id> --recipe panel-review   # same feature, other strategy, new run id
brokkr compare <a> <b>                          # trails, first divergence, per-seat costs
```

### What it cost

```
$ brokkr costs --run latest
{
  "run_id": "prefix-selectors-for-the-read-su-8bf6d692",
  "seats": {
    "implement": { "attempts": 1, "turns": 24, "cost_usd": 0.0 },
    …
  },
  "total_cost_usd": 0.0
}
```

Per seat: `attempts` counted from `effect/started` events, and `turns`
and `cost_usd` summed from the `num_turns` and `total_cost_usd` fields
the driver reported in its checkpoints. `--db` defaults to
`.forge/forge.db`; `brokkr costs` takes no `--realms`.

Be clear-eyed about this:

- **A run spawns real, billed agent sessions.** Every seat is a Claude
  Code or Codex session against your account. A five-phase recipe with
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
- **`--realms` reaches only some commands.** `run` and the read surfaces
  the ruling names — `runs`, `realms`, `tui`, `watch`, `inspect`,
  `export`, `muninn run` — accept it. `resume`, `conclude`, `rerun`,
  `doctor`, `ui`, `costs`, `compare`, `anchor` and `bridge` take `--db`
  alone.
- **A Looper-dispatched run (`--dispatch`) cannot adopt agents and
  carries no realms map.** The v2 manifest lineage would silently drop
  both, so the engine refuses instead.
- **Secrets-store permission enforcement is Unix-only.** `brokkr
  secrets` creates the store `0600` and refuses to read one whose
  permissions are broader, but that check is `#[cfg(unix)]`. On Windows
  the store is written without an equivalent guard.
- **Column alignment is byte-based.** Colour follows `NO_COLOR` and
  `TERM`, width follows `COLUMNS`; without a Unicode-width dependency,
  CJK and emoji columns misalign in the readouts.

## Next

- [starters/](starters/) — what `brokkr init` actually wrote, per
  stack, transcribed from real runs.
- [cards/](cards/) — the per-stack deltas over this spine.
- [recipe-authoring.md](recipe-authoring.md) — write or extend a
  delivery strategy.
- [driver-authoring.md](driver-authoring.md) — put a harness that is not
  Claude Code or Codex behind a seat.
- [versioning.md](versioning.md) — what is stable, what may still move.
