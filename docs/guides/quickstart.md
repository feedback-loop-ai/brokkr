# Quickstart — your first slice in twenty minutes

You need a git repository you are willing to let an agent edit, and one
agent CLI on your `PATH` (`claude` or `codex`). Everything else is one
native binary — no Python, no Node, no services.

This walkthrough installs Brokkr, checks the machine, scaffolds a
starter recipe, runs one slice, reads what the run decided, and shows
the two ways out when a run stops short of `done`.

- [1. Install](#1-install)
- [2. Check the machine](#2-check-the-machine)
- [3. Scaffold a recipe you can read](#3-scaffold-a-recipe-you-can-read)
- [4. Run a slice](#4-run-a-slice)
- [5. Where the run wrote things](#5-where-the-run-wrote-things)
- [6. Read the ending](#6-read-the-ending)
- [7. The escape hatches](#7-the-escape-hatches)
- [8. What it cost](#8-what-it-cost)
- [Limits worth knowing before you start](#limits-worth-knowing-before-you-start)

## 1. Install

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

**Or build from a checkout.** Rust 1.85 or newer:

```
cargo install --path crates/brokkr-cli    # installs the `brokkr` binary
```

Put the binary somewhere on your `PATH`. The rest of this guide assumes
plain `brokkr`.

## 2. Check the machine

`brokkr doctor` verifies tools, provider adapters, the agent library and
the workspace database, and executes no agent. Run it first — it is the
cheapest way to find out that `claude` is missing or that your database
path is wrong.

```
$ brokkr doctor
ok       contracts: engine 0.5.0, event_schema 1, database_schema 1, driver_protocol 1
ok       git: git version 2.51.0
ok       claude: 2.1.251 (Claude Code) · serves fable, haiku, opus, sonnet
ok       agent implementer: would run opus via claude here (chain opus → sonnet)
…
```

Lines are prefixed `ok`, `warn`, or `MISSING`. A `MISSING` line is a
refusal to guess: an absent driver binary means seats resolving to that
provider will fail to spawn, and doctor says so rather than letting you
find out mid-run. Warnings are optional capabilities.

Two flags: `--bundle <dir>` also compiles a bundle and reports the
result, and `--db <path>` chooses the workspace journal (default
`.forge/forge.db`). `brokkr doctor` takes no `--realms`.

## 3. Scaffold a recipe you can read

A **recipe** is a delivery strategy as reviewable data: a phase table, a
seat per phase, a role charter per seat, and per-seat limits. `brokkr
init` writes one you are meant to open and edit.

```
$ brokkr init my-bundle
initialized reviewable bundle at my-bundle (digest 5de309d50685ec831e14b905e0c8f4ee01f5745ea7bac0d0885ed17b275f8a75)
```

`init` takes the directory as a **positional argument**, not a flag. It
refuses rather than overwriting a directory that already has a
`bundle.json`, and it compiles the bundle before printing the digest, so
what you were handed is a thing that runs.

What it wrote:

```
my-bundle/
  bundle.json          # name, policy path, protected_phase, five seats
  policy.json          # forge.phase-machine/v1, seven phases, nineteen rules
  roles/intake.md
  roles/implementer.md
  roles/verifier.md
  roles/reviewer.md
  roles/shipper.md
```

The table has five working phases — `intake`, `implement`, `verify`,
`review`, `ship` — plus the two terminals `done` and `stop`. `review` is
the protected phase: compilation rejects any table with a path to a
non-`stop` terminal that skips it. Each seat declares its own result
vocabulary and its own `limits` (attempts and a deadline in seconds).
Read [recipe-authoring.md](recipe-authoring.md) when you want to change
any of that.

If you would rather start from a maintained strategy than a scaffold,
the library ships several:

```
$ brokkr recipes list
fast	5779cd13be64	6 phases	implement, review, ship, verify	recipes/fast
panel-review	b44de756c398	7 phases	implement, intake, review[correctness+security], ship, verify	recipes/panel-review
sdd	3743484daa2b	8 phases	design[positions>chief>speckit-check], implement, intake, review[security+spec-compliance], ship, verify	recipes/sdd
…
```

The twelve hex characters after the name are the leading bytes of the
recipe's content digest. Use `--recipe <name>` instead of `--bundle
<dir>` to run one of these; `--recipes-dir` (default `recipes`) says
where the library lives.

## 4. Run a slice

```
$ brokkr run --bundle my-bundle --repo . --feature "prefix selectors for the read surfaces"
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

## 5. Where the run wrote things

Relative to the working directory (`--repo`, or the cwd):

- `.forge/forge.db` — the journal, unless `--db` or a map says otherwise.
- `.forge/tasks/<slug>.md` — the intake seat's framing, run-local.
- `.forge/results/<effect_id>.json` — one typed result file per seat
  attempt. This file is the only channel the engine reads from a seat;
  anything a seat prints to stdout is not a result.
- `.forge/ledger/<run-id>.md` — the shipper's close-out, if the run got
  that far.
- `.forge/secrets.env` — only if you used `brokkr secrets`.

## 6. Read the ending

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

## 7. The escape hatches

### Operator commands — `retry` and `stop`

```
brokkr operator --run <id> retry --reason "the flaky test passes on re-run"
brokkr operator --run <id> stop  --reason "requirements changed"
```

The command is a **positional argument** and `--reason` is **required**;
`--db` defaults to `.forge/forge.db`. There are exactly two commands:
`retry` re-runs the current phase, `stop` ends the run. Both are
recorded as `operator/commanded` + `operator/accepted` journal events —
approval is an entry in the record, not a prose convention.

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

### Resume

```
brokkr resume --run <id> --bundle my-bundle
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

### Re-run under another strategy

Not an escape hatch so much as the next experiment:

```
brokkr rerun --run <id> --recipe panel-review   # same feature, other strategy, new run id
brokkr compare <a> <b>                          # trails, first divergence, per-seat costs
```

## 8. What it cost

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

## Limits worth knowing before you start

- **One journal per world.** A realms map names a set of repositories
  and exactly one `journal` they share (`forge.realms/v1`). There is no
  per-realm journal.
- **`--realms` reaches only some commands.** `run` and the read surfaces
  the ruling names — `runs`, `realms`, `tui`, `watch`, `inspect`,
  `export`, `muninn run` — accept it. `resume`, `rerun`, `doctor`, `ui`,
  `costs`, `compare`, `anchor` and `bridge` take `--db` alone.
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

- [recipe-authoring.md](recipe-authoring.md) — write or extend a
  delivery strategy.
- [driver-authoring.md](driver-authoring.md) — put a harness that is not
  Claude Code or Codex behind a seat.
- [versioning.md](versioning.md) — what is stable, what may still move.
