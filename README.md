<p align="center"><img src="assets/logo.svg" width="132" alt="Brokkr — sealed anvil mark, terminal rail node pulsing"></p>

# Brokkr

[![ci](https://github.com/feedback-loop-ai/brokkr/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/feedback-loop-ai/brokkr/actions/workflows/ci.yml)
[![release](https://img.shields.io/github/v/release/feedback-loop-ai/brokkr?label=release&color=blue)](https://github.com/feedback-loop-ai/brokkr/releases/latest)
[![license: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![clippy · -D warnings](https://img.shields.io/badge/clippy%20%C2%B7%20--D%20warnings-orange)](.github/workflows/ci.yml)
[![coverage · literal 100%](https://img.shields.io/badge/coverage%20%C2%B7%20literal%20100%25-brightgreen)](scripts/coverage-exact.sh)
[![deps · permissive-only](https://img.shields.io/badge/deps%20%C2%B7%20permissive--only-brightgreen)](deny.toml)
[![platforms](https://img.shields.io/badge/linux%20x86__64%2Faarch64%20%C2%B7%20macos%20x86__64%2Farm64%20%C2%B7%20windows-blue)](https://github.com/feedback-loop-ai/brokkr/releases/latest)
[![rust](https://img.shields.io/badge/rust-1.85%2B-orange)](Cargo.toml)

**Coordination tools help agents work together. Brokkr proves what they
did** — runs you can replay, rulings you can audit, releases you can
prove.

*The machine is the outer loop. Struck, not spun.*

**A deterministic delivery engine for autonomous multi-agent software
delivery.** The outermost layer is a pure, event-sourced phase state
machine; agent sessions (Claude Code, Codex, any harness speaking the
driver protocol) are leaf effects whose outputs are typed results —
never decisions. Every claim the system makes — done, verified, parked,
stopped, paid — is a journaled, replayable, anchored fact.

> Agent output never decides a transition. Prompt content never decides
> who pays (that's [LaneTally](https://github.com/feedback-loop-ai/lanetally)'s
> law, one layer down). The same value, stacked.

## Contents

- [The name](#the-name)
- [What it does](#what-it-does)
- [Determinism laws](#determinism-laws)
- [60-second quickstart](#60-second-quickstart)
- [The read surfaces](#the-read-surfaces)
  - [`brokkr runs` — the fleet](#brokkr-runs--the-fleet)
  - [`brokkr inspect` — one run, explained](#brokkr-inspect--one-run-explained)
  - [`brokkr watch` — the same, live](#brokkr-watch--the-same-live)
  - [`brokkr tui` — the readouts made explorable](#brokkr-tui--the-readouts-made-explorable)
  - [`brokkr ui` — the browser console](#brokkr-ui--the-browser-console)
  - [`brokkr muninn` — the fleet, read and advised on](#brokkr-muninn--the-fleet-read-and-advised-on)
- [Recipes and composition](#recipes-and-composition)
- [The agent library](#the-agent-library)
- [Provider adapters](#provider-adapters)
- [Secrets](#secrets)
- [The journal and verification](#the-journal-and-verification)
- [Repo layout](#repo-layout)
- [The decision culture](#the-decision-culture)
- [Contributing](#contributing)
  - [Quality gates](#quality-gates)
  - [The flow](#the-flow)
  - [Contribution licensing](#contribution-licensing)
- [Acknowledgments](#acknowledgments)
- [License](#license)

## The name

This engine was renamed to **Brokkr** — in the myth, the dwarf whose
whole task was to work the bellows and not stop; Loki, as a biting fly,
made him flinch once, and Mjölnir's handle came out short. Steadiness
under distraction, and the cost of one lapse, is this engine's core loop
told as a story a thousand years old. The old name was also the most
collided word in software, and so was never findable.

**"Forge" survives as the verb.** Slices are forged, runs are forged,
Brokkr forges. The proper noun retired from the marquee, not from the
vocabulary — and the mechanism keeps its plain names, so a new operator
can still guess what a command does with no glossary: `.forge/`,
`forge.db`, `refs/forge/`, the `forge-*` crates, the wire protocols.

**The binary is `brokkr`.** The old binary name works for one more
release: every archive carries a `forge` shim beside it, which writes a
single notice to stderr — never to stdout, so pipes and `--json`
consumers read exactly what `brokkr` writes — and then behaves
identically. Repoint your scripts before the release after this one.

[Decision 0019](docs/decisions/0019-brokkr.md) is the ruling, with the
reasoning and the five laws that bound it. [The Edda](docs/lore/edda.md)
is the lore layer those laws govern: commentary, never specification —
if it burned, the constitution would still be whole.

## What it does

You hand Brokkr a feature and a **recipe** (a delivery strategy:
policy table, seats, charters, limits, drivers — reviewable text,
identified by content digest). The engine drives real agent sessions
through the recipe's phases — implement, verify, review, ship — ruling
on each typed result with a pinned first-match-wins policy. Unknowns
never advance: schema violations, unmatched results, exhausted retries,
and security findings park or stop the run with raw evidence attached.
The operator's judgment enters only as signed journal events.

```
brokkr run --recipe fast --repo . --feature "…"     # deliver
brokkr watch --run <id>                             # watch it live, in the terminal
brokkr tui                                          # explore the fleet with the keyboard
brokkr ui                                           # watch it live, in a browser
brokkr rerun --run <id> --recipe panel-review       # swap the strategy
brokkr compare <a> <b>                              # journal-backed A/B
```

- **Live telemetry**: Claude and Codex seats stream bounded per-turn/item
  checkpoints; DSH emits an explicit harness lifecycle. Checkpoints retain
  only bounded turn/tool/usage fields or a file-path-only target—never prose,
  commands, or reasoning; the Looper bridge hashes the target before export.
  The journal is evidence, not transcript. Parallel
  review panels stream both members
  side by side. The full session transcript stays one
  `claude --resume <session_id>` away.
- **The strategy loop** (decisions 0010/0017): a library of recipes
  (`brokkr recipes list|add|show`, installable from git), swap by name,
  re-run a past feature under another recipe, compare outcomes —
  decision trails with first divergence, per-seat costs, verdict
  deltas. A pure read over two journals; works on live runs.
  Recipes **compose**: `extends: "sdd"` plus one override is a whole
  strategy (`recipes/sdd-paranoid`, sixty lines against SDD's 103).
  Named things merge by name; redefining one the base has needs an
  explicit marker, so an accidental collision fails compilation instead
  of silently winning. Composition resolves at compile time into ONE
  flat bundle — no inheritance at run time — and the run manifest
  records the chain, so a run states what it was composed from.
- **Bounded autonomy** (decisions 0006/0007): per-seat attempt limits
  and deadlines; determinate failures retry, indeterminate outcomes
  always park; every evaluation input is engine-computed or
  seat-declared — everything else is dropped before it reaches the
  table or the record.
- **Self-hosting**: Brokkr forges its own changes (`bundles/self`)
  and verifies every delivered slice with its own adversarial agents
  (`bundles/verify`) — which have hard-stopped their author's work on
  real security findings, twice. The operator keeps push and merge
  authority.

[ARCHITECTURE.md](ARCHITECTURE.md) is the deep dive: crates, journal,
effect discipline, verification layers. This README stays the tour.

## Determinism laws

1. **Decisions are pure.** Given the same journal and pinned bundle,
   the next action is always the same. Transition logic is a data table
   evaluated first-match-wins by `forge-core`; changing a ruling is a
   reviewed one-line diff.
2. **State is derived, never mutated.** `state = fold(events)`; resume
   is replay; counters, drift, and reviewed heads are journal-computed,
   never accepted from a caller.
3. **No LLM repair of the control plane** (decision 0001). Invalid
   results park in `awaiting_operator` with the raw evidence — never
   guessed at, coerced, or handed to a model to fix.
4. **Human gates are control states.** Parks exit only through operator
   events; approval is a journal entry, not a prose convention.

## 60-second quickstart

One native binary — no Python, no Node, no services.

**Install from a release.** Grab the archive for your platform from the
[latest release](https://github.com/feedback-loop-ai/brokkr/releases/latest)
(linux x86_64/aarch64, macOS arm64/x86_64, windows x86_64), verify it
against the release's `SHA256SUMS`, then unpack:

```
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/brokkr-linux-x86_64.tar.gz
curl -LO https://github.com/feedback-loop-ai/brokkr/releases/latest/download/SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS      # brokkr-linux-x86_64.tar.gz: OK
tar xzf brokkr-linux-x86_64.tar.gz            # → ./brokkr, plus the one-release ./forge shim
```

Every archive and the `SHA256SUMS` manifest carry a signed GitHub
Sigstore build-provenance attestation, so the checksum itself can be
checked against the workflow that produced it:

```
gh attestation verify brokkr-linux-x86_64.tar.gz -R feedback-loop-ai/brokkr
```

**Or build it.** Rust 1.85 or newer:

```
cargo install --path crates/forge-cli    # installs the `brokkr` binary (and the `forge` shim)
```

**Then deliver something.**

```
$ brokkr doctor                           # tools, agent CLIs, database, contracts
ok       contracts: engine 0.3.4, event_schema 1, database_schema 1, driver_protocol 1
ok       git: git version 2.51.0
ok       claude: 2.1.251 (Claude Code) · serves fable, haiku, opus, sonnet
ok       agent implementer: would run opus via claude here (chain opus → sonnet)
…

$ brokkr init my-bundle                   # scaffold a reviewable starter recipe
initialized reviewable bundle at my-bundle (digest 5de309d50685ec831e14b905e0c8f4ee01f5745ea7bac0d0885ed17b275f8a75)

$ brokkr run --bundle my-bundle --repo . --feature "prefix selectors for the read surfaces"
run started: prefix-selectors-for-the-read-su-8bf6d692
…

$ brokkr tui                              # explore what just happened
```

`brokkr run` exits 0 when the run reaches `done`, 2 when it parks for the
operator, and 3 when it stops — so a shell script can tell the three
apart without parsing anything.

`brokkr init` writes a starter recipe you are meant to read: a seven-phase
policy table (five working phases plus `done` and `stop`) with the review
gate constitutionally protected, one seat per working phase, and a role
charter per seat. It compiles the bundle before printing the digest, so
the thing you were handed is a thing that runs.

## The read surfaces

Every readout shares ONE derivation (decision 0013): `forge-view` turns
a journal into view models, and each surface only renders them — so
"what did this seat cost" has a single answer, tested once. `runs`,
`inspect` and `watch` each take `--json` to emit that model verbatim.

`--run` takes a **selector**, not only the 41-character id: any unique
run-id prefix, or `latest` for the newest run in the workspace database
(decision 0015) — one resolver, shared by `watch`, `inspect`, `anchor`,
`export` and `replay`.

Colour follows `NO_COLOR` and `TERM`, width follows `COLUMNS`; without a
Unicode-width dependency, CJK and emoji columns misalign — stated rather
than pretended away.

`--realms <file>` chooses the **world** these surfaces read (decision
0023): a map of repositories and the journal they share, defaulting to
`./realms.json` when there is one. `--db` is retained and outranks the
map's journal; with neither, the journal is `.forge/forge.db` exactly as
it always was — a world that never drew a map notices nothing.

Phase 1 wires the flag into `run` and the read surfaces the ruling names
— `runs`, `realms`, `tui`, `watch`, `inspect`, `export`, `muninn run`.
The others (`resume`, `rerun`, `doctor`, `ui`, `costs`, `compare`,
`anchor`, `bridge`) still take `--db` alone, so a run started in a world
whose map names a journal other than `.forge/forge.db` is resumed by
naming that journal with `--db`.

### `brokkr realms` — the world

Every realm with its path, default branch and current HEAD, and the
journal the world writes. Read-only, like every other readout, and
`--json` emits the same derivation for scripts.

```
$ brokkr realms
map      ./realms.json
journal  ./.forge/forge.db
realm    the-forge  .  main  5a4bf4a28558d123c432d8992cfd9f13ffd81eb7
```

### `brokkr runs` — the fleet

One clamped line per run, newest first.

```
$ brokkr runs
prefix-selectors-for-the-read-su-8bf6d692 completed done seq 38 3s prefix selec…
```

### `brokkr inspect` — one run, explained

Header, ruling, seats, decision trail, and the phase graph as a tree.
`--phase` and `--seat` are the scoping verbs the console's clicks
became.

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
   9 phase/entered      implement
  14 effect/succeeded   implement · complete
  15 transition/decided IMPL-OK implement → verify · complete
  16 phase/entered      verify
  21 effect/succeeded   verify · pass
  22 transition/decided VERIFY-PASS verify → review · pass
  23 phase/entered      review
  28 effect/succeeded   review · clean
  29 transition/decided REVIEW-CLEAN-NO-FIXES review → ship · clean
  30 phase/entered      ship
  35 effect/succeeded   ship · shipped
  36 transition/decided SHIP-COMPLETE ship → done · shipped
  37 phase/entered      done
  38 run/completed      completed

graph
  intake ×1
    → intake · finished
  implement ×1
    → implement · finished
  verify ×1
    → verify · finished
  review ×1
    → review · finished
  ship ×1
    → ship · finished
  done ×1  ←current
```

Every line above is a rule id and a journal sequence number: the run
states which rule fired, from where, on which typed result. Nothing in
that trail was written by a model.

### `brokkr watch` — the same, live

The same readout, redrawn whenever the journal head moves, exiting when
the run reaches a terminal status. Read-only, like every other readout.

```
$ brokkr watch --run latest
── 2026-08-29T22:27:40.474827119Z ──
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

graph
  intake ×1
    → intake · finished
  implement ×1
    → implement · finished
  verify ×1
    → verify · finished
  review ×1
    → review · finished
  ship ×1
    → ship · finished
  done ×1  ←current
```

### `brokkr tui` — the readouts made explorable

Decision 0014: arrow keys or `j`/`k` move, `Enter` descends from the run
list to a run to one seat's own stream, `Esc` comes back, `/` filters,
`?` opens help, and a footer names the keys of wherever you are. It is
read-only exactly as every other readout is — no operator commands, no
run starts, nothing written to the journal, and a missing database
refuses rather than creating one.

The fleet:

```
┌runs──────────────────────────────────────────────────────────────────────────────────────────┐
│id                       status    phase        seq    age      feature                       │
│prefix-selectors-for-the completed done         38     1m20s    prefix selectors for the read │
│                                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
runs
↑↓/jk move · Enter open run · g/G top/bottom · / filter · r refresh · ? help · q quit
```

`Enter` on a run — the phase rail, the seats, the trail, all three panes
of the same derivation, and the brand mark riding the graph pane's
border, its third rail node pulsing whenever the fleet is forging:

```
┌graph─────────────────────────────────────────────────────────────────────────[ ∙ ∙ ⏺ BROKKR ]┐
│                                                                                              │
│ ⏺ intake──ᐳ⏺ implement──ᐳ⏺ verify──ᐳ⏺ review──ᐳ⏺ ship───ᐳ∙                                   │
│ intake      implement    verify     review     ship     done                                 │
│                                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌seats─────────────────────────────────────────────────────────────────────────────────────────┐
│participant            status        attempts turns  cost       activity                      │
│intake                 succeeded     1        —      —          resolved · 0s                 │
│implement              succeeded     1        —      —          complete · 0s                 │
│verify                 succeeded     1        —      —          pass · 0s                     │
│review                 succeeded     1        —      —          clean · 0s                    │
│ship                   succeeded     1        —      —          shipped · 0s                  │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌trail─────────────────────────────────────────────────────────────────────────────────────────┐
│1  run/started  prefix selectors for the read surfaces…                                       │
│2  phase/entered  intake                                                                      │
│7  effect/succeeded  intake · resolved                                                        │
│8  transition/decided  INTAKE-OK intake → implement · resolved                                │
│9  phase/entered  implement                                                                   │
│14  effect/succeeded  implement · complete                                                    │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
runs · run prefix-selectors-for-the-read-su-8bf6d692
←→ rail · ↑↓ lanes · Enter scope phase · Tab pane · Esc back · / filter · r refresh · ? help · q
```

### `brokkr ui` — the browser console

`brokkr ui` serves an embedded, loopback-only, read-only surface on port
8383: runs, live seat activity, the causal event timeline. Same
derivation, same answers, a mouse instead of a keyboard.

```
$ brokkr ui --port 8383 --open
```

### `brokkr muninn` — the fleet, read and advised on

The read surfaces above show you the fleet. `brokkr muninn` reads it for
you and writes down what it would suggest — and then stops there.

One invocation opens the workspace database **read-only**, derives a
dossier from the same `forge-view` models every other readout uses (runs
with status, phase, age and cost; park reasons and the operator commands
each parked run admits; consecutive failures; the residual findings the
verify and review rulings recorded), and hands it to one bounded seat
under the driver fleet — a deadline, one attempt, no retry ladder. What
comes back is a fleet summary, a suggested operator command per parked
run with its reasoning, and the residual findings as a work queue.

Nothing it proposes is executed, and nothing here can execute it. Muninn
issues no operator command, starts no run, is given no repository tree
and no secrets, and writes to no run journal — proposals go to its own
append-only file, `.forge/muninn.ndjson`, beside the journal and inside
none of it. Every proposal names the run ids and sequence numbers it was
derived from; a report that cites a fact the dossier does not carry is
refused and recorded nowhere. Acting on any of it stays the operator's
own `brokkr operator` command (decision 0020).

```
$ brokkr muninn run
2026-08-31T14:26:48Z · 1 proposals for parked runs · 2 findings queued
  summary: four runs; one is parked on a flaky verify, two are green
  parked prefix-selectors-8bf6d692 seq 41 · suggest 'retry' · the park
    reason names one test that has passed on re-run twice before
  queue review-the-lane-cursor-1f0a seq 33 · max_residual_severity: high
    · the only high residual in the fleet
  cites: prefix-selectors-8bf6d692 seq 41, review-the-lane-cursor-1f0a seq 33

$ brokkr muninn list          # every past invocation, citations included
```

## Recipes and composition

A recipe is a delivery strategy as reviewable data, identified by
content digest. The library is a directory of them.

```
$ brokkr recipes list
fast	5779cd13be64	6 phases	implement, review, ship, verify	recipes/fast
panel-review	b44de756c398	7 phases	implement, intake, review[correctness+security], ship, verify	recipes/panel-review
sdd	3743484daa2b	8 phases	design[positions>chief>speckit-check], implement, intake, review[security+spec-compliance], ship, verify	recipes/sdd
sdd-paranoid	368569ad218d	8 phases	design[positions>chief>speckit-check], implement, intake, review[adversarial+security], ship, verify	recipes/sdd-paranoid
self	e36523e469d0	7 phases	implement, intake, review, ship, verify	bundles/self
verify	66052438d68d	4 phases	review, verify	bundles/verify
```

Recipes **compose** (decision 0017). `recipes/sdd-paranoid` is sixty
lines: it extends `sdd` and replaces exactly one seat, and it has to say
so out loud.

```json
{
  "name": "sdd-paranoid",
  "extends": "sdd",
  "override": { "seats": ["review"] },
  "seats": {
    "review": { "…": "an adversarial panel instead of SDD's" }
  }
}
```

Named things merge by name; redefining one the base already has without
listing it under `override` fails compilation rather than silently
winning. Composition resolves at compile time into ONE flat bundle — no
inheritance at run time — and the run manifest records the chain, so a
run states what it was composed from.

Swap a strategy and compare the outcomes:

```
brokkr rerun --run <id> --recipe panel-review    # same feature, other strategy
brokkr compare <a> <b>                           # trails, first divergence, per-seat costs
```

## The agent library

A seat used to inline everything it was: charter text, driver argv,
limits, declared inputs. Decision 0016 lets it name an agent instead.

```
$ brokkr agents list
chief-architect	fable → opus → sonnet	Synthesises the panel's positions into the committed spec, plan and tasks, and rules on the open questions.
implementer	opus → sonnet	Builds the framed task to the repository's conventions and commits the work with its tests.
intake	sonnet → opus	Frames a raw request into a recorded, actionable task before any code is written.
review-security	opus → sonnet	Review panel member: the adversarial security read of the change.
reviewer	opus → sonnet	The single-seat reviewer: correctness and security in one pass, for recipes without a review panel.
shipper	sonnet → opus	Closes a delivery out: ledger, gates, and the report the operator reads before merging.
verifier	sonnet → opus	Runs the suites and gates and reports pass or fail on evidence, never on intent.
…
```

```
brokkr agents show <name>    # the definition, plus its per-entry resolution
brokkr doctor                # which providers and models are actually here
```

An agent is one file in `agents/`: a description, a charter, an ORDERED
preference chain of abstract model names, abstract tool and MCP
configuration, its decision-0006 limits and its decision-0007 declared
inputs. A seat, panel member or sequence step says `"agent": "<name>"`.
Inline seats stay first-class — `recipes/sdd`'s `speckit-check` step is
a shell script with no model, and it stays inline.

Resolution happens at compile time, is pinned into the run manifest, and
is a pure function of *(library, adapters, availability)* — availability
that `Bundle::compile` deliberately supplies none of, so one bundle
cannot resolve two ways on two machines.

**The honesty rules are the point, and they are enforced rather than
documented.** A tool restriction the provider cannot express fails
compilation naming the agent, the provider and the capability — the
agent would run with MORE power than it declares, so `optional` is
structurally unrepresentable there. An MCP server the provider cannot
serve fails the same way unless the agent marked it optional, and then
it is a notice that lands in the run manifest and in every readout —
never nothing. Both checks run over **every** entry in the chain, so a
chain that would widen an agent's blast radius the moment it fell back
fails at design time rather than at 2am. The chain is a fallback chain,
not a portability claim: Brokkr never says the second choice equals
the first, and `brokkr compare` reports a model difference as a
first-class divergence.

Fallback is narrow on purpose. An attempt that FAILS TO START — the
driver binary is absent, or the provider rejects the model before
accepting — retries on the next model in the chain, inside decision
0006's existing attempt bounds, journaled as a fact. A mid-session
failure is not fallback material: a seat that ran for forty turns and
then hit a wall produced work a different model does not inherit, so it
follows 0006 unchanged. The predicate is structural — `Failed`, never
`Accepted`, no checkpoint — so once a session opens, fallback is
unreachable by construction rather than by convention.

**Three limits ship with it, stated as limits.**

1. **A Looper-dispatched run cannot adopt agents.** The v2 run-manifest
   lineage reconstructs a bundle manifest from six named keys and would
   silently drop the `agents` pin, making the run unresumable with a
   diff that blames no file. `build_run_manifest_v2` refuses instead.
   Lifting it needs a jointly agreed v2-lineage manifest version.
2. **Provenance does not cross the Looper bridge.** The bridge's payload
   allowlist drops it, asserted by a test rather than assumed.
3. **"No `Accepted` ever arrives" parks, it does not fall back.** That
   shape is `indeterminate` today, and decision 0003 rules that it parks
   because Brokkr cannot tell "did nothing" from "already opened a
   billed session". The honest fix is at the driver — report a
   provider's pre-session model rejection as a determinate failure — not
   at the engine, because a bound that applies "unless a new feature is
   in play" has stopped being a bound.

## Provider adapters

A provider adapter is **data**, one file per provider in `adapters/`:
the driver invocation, the abstract→concrete model mapping, how tool
permissions and MCP servers are expressed, and — the load-bearing part —
which of those the provider **cannot** express, written as the explicit
string `"unsupported"` rather than left to be inferred from an empty
map. Adding a provider or a model is a file edit, not a release.

`brokkr doctor` reports what each adapter can actually reach on this
machine, and refuses to guess about the rest:

```
$ brokkr doctor
ok       claude: 2.1.251 (Claude Code) · serves fable, haiku, opus, sonnet
ok       codex: codex-cli 0.148.0 · serves no abstract model yet
ok       dsh: 0.1.0-rc.6 · serves no abstract model yet
warn     lanetally: binary 'claude-lanetally' not found — seats resolving to this provider will fail to spawn …
```

The built-in adapters are reachable directly as
`brokkr driver <claude|lanetally|codex|dsh|exec> -- <extra args>`, which
is exactly how a bundle names them.

Looper-bound runs start with `brokkr run --dispatch <forge-dispatch-v2.json>`.
The immutable dispatch is sealed into the v2 run manifest and therefore travels
with `brokkr export`. `brokkr bridge --run <id> --looper-url <url>` tails only the
verified public store API and synchronizes ordered evidence plus fenced commands;
it reads its bearer credential from `LOOPER_API_KEY` (or `--token-env`), never
from a command-line value or the journal.

## Secrets

Decision 0012: bundles and journals carry secret **names** only. A
driver template writes `{{secret:NAME}}`; the runner — and nothing
upstream of it — resolves that to a value from an operator-side store
that lives outside version control.

```
$ brokkr secrets set GITHUB_TOKEN    # value read from STDIN, never argv; store created 0600
set GITHUB_TOKEN in .forge/secrets.env

$ brokkr secrets list                # names, one per line — there is no value-printing verb
GITHUB_TOKEN

$ brokkr secrets remove GITHUB_TOKEN
removed GITHUB_TOKEN from .forge/secrets.env
```

The store defaults to `.forge/secrets.env` in the workspace; `brokkr run
--secrets-file` points elsewhere. A seat declares which names it binds,
compilation fails on an undeclared one, and any bound value that appears
in captured stderr is masked to `[secret:NAME]` on raw bytes before the
string ever exists. The `{{secret:NAME}}` spelling itself is not
secret-bearing, which is why it is journalable and the resolved command
line is not.

## The journal and verification

The journal is an append-only, hash-chained SQLite table. State is the
fold of it; nothing else is authoritative.

```
$ brokkr anchor --run latest                     # record the head in refs/forge/<run>
anchored prefix-selectors-for-the-read-su-8bf6d692 at 94c5bd9dff99bef4d4b9d224d8cc1661681fd194

$ brokkr anchor --run latest --check             # tamper evidence, re-checked
$ brokkr export --run latest --out ./out --redact  # plus a marked publishable derivative:
#   <run>.redacted.ndjson — paths and usernames as stable placeholders, hashes
#   verify only on the verbatim pair, and the manifest says so
{
  "chain_length": 2,
  "journal_head_hash": "74d186fb254b62fb486b3f5f3fe1e1ad7c91fa5deef2d850d60c16e5478be918",
  "ref": "refs/forge/prefix-selectors-for-the-read-su-8bf6d692",
  "seq": 38,
  …
}

$ brokkr export --run latest --out ./out         # canonical NDJSON + pinned manifest
exported ./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson

$ brokkr verify-run ./out/prefix-selectors-for-the-read-su-8bf6d692.ndjson
{
  "chain": "verified",
  …                                              # envelopes and fold, offline
}

$ brokkr replay --run latest                     # rebuild twice, compare
{
  "chain": "verified",
  "events": 38,
  "replay": "deterministic",
  …
}
```

`brokkr costs --run <id>` reports per-seat attempts, turns and USD — the
LaneTally join surface, computed from journal checkpoints with stable
seat ids.

Four verification layers back all of this, each mechanical: the 97-case
differential corpus pins the evaluator; the machine-proof suite drives
the real binary through every failure mode (30+ scenarios, three OSes,
coverage-gated CI); self-forge runs deliver changes under the full
constitution; and the verify agents adversarially review every landed
slice — their verdicts are journaled runs like any other.

Release admission additionally requires canonical formatting, warning-free
Clippy across all targets and features, a RustSec dependency audit, literal
nonzero 100% source-line/branch/function coverage, frozen/additive contract
compilation, producer-bridge conformance, and a checksum-verified platform
matrix. Release archives and `SHA256SUMS` carry GitHub Sigstore build-provenance
attestations; verify an asset with
`gh attestation verify <asset> -R feedback-loop-ai/brokkr`.

## Repo layout

| Path | What it is |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | The implemented architecture — crates, journal, effect discipline, verification layers. |
| `crates/` | The engine: `forge-core` (pure) · `forge-store` · `forge-protocol` (+ built-in claude/codex/dsh/exec adapters) · `forge-runtime` · `forge-view` (one display derivation, no I/O) · `forge-bridge` · `forge-cli` (builds `brokkr` and the one-release `forge` shim). Crate names lag the rename by a release (decision 0019 ruling 9). |
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2`, `forge-run-manifest/v2`, `/v3` and `/v4`, `forge-effect-provenance/v1`, `forge.phase-machine/v2` (the rule-driven park, decision 0022), and `forge.realms/v1` (the world's map, decision 0023). |
| `realms.json` | This repository's own map (decision 0023): one realm — this repository — and the journal it writes. A workspace of many projects is another file, named with `--realms`. |
| `bundles/` | System recipes: `self` (self-delivery) and `verify` (the verification agents). |
| `recipes/` | The user recipe library (`fast`, `panel-review`, `sdd`, `sdd-paranoid` — which `extends` `sdd` — yours). |
| `agents/` | The agent library (decision 0016): one definition per agent plus the charters seats used to inline. |
| `adapters/` | One data file per provider: driver invocation, abstract→concrete model mapping, and what the provider CANNOT express. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| [`docs/decisions/`](docs/decisions/) | The constitution: numbered operator rulings 0001–0019, indexed. |
| [`docs/lore/`](docs/lore/) | The lore layer of [decision 0019](docs/decisions/0019-brokkr.md): [the Edda](docs/lore/edda.md) and the sagas. Commentary, never specification. |
| `assets/` | The brand mark: [`logo.svg`](assets/logo.svg) (the anvil and the three rail nodes) and `social-preview.png`, the 1280×640 card the repository shows when it is linked. |
| `reference/` | Read-only heritage documents: handoff-protocol lore, recorded schemas. |
| `scripts/coverage-exact.sh` | The exact-coverage gate: literal 100% line/branch/function, or refusal. |

## The decision culture

Every semantic change is a numbered operator ruling in
[`docs/decisions/`](docs/decisions/), kept in full, cited by number in
the code that enforces it. [The index](docs/decisions/README.md) lists
them with their status.

An implementer may write a decision, but only ever with status
`proposed`; acceptance is the operator's, recorded in the file. A ruling
is never edited into a different meaning — a new number supersedes it
and says so. That is why the README, the error messages and the tests
can all cite "decision 0007" and mean the same paragraph.

## Contributing

The engine forges its own changes and reviews them adversarially, so
the bar for a human contribution is the bar the machine is already held
to.

### Quality gates

All of these are required jobs in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml). Run them before
you open anything.

| Gate | Command | Where it lives |
|---|---|---|
| Canonical formatting | `cargo fmt --all -- --check` | `ci.yml` → job `quality` |
| Clippy, warnings as errors | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | `ci.yml` → job `quality` |
| Contracts compile | `brokkr compile --bundle bundles/self` and `bundles/verify` | `ci.yml` → jobs `quality`, `engine` |
| Full suite, three OSes | `cargo test --workspace --all-features --locked` | `ci.yml` → job `engine` (ubuntu · macos · windows) |
| Exact coverage | `bash scripts/coverage-exact.sh` | `ci.yml` → job `coverage` |
| RustSec dependency audit | `rustsec/audit-check` | `ci.yml` → job `dependency-audit` |

The coverage gate is not a percentage to trend upwards. It re-folds a
candidate-bound LCOV report and demands literal integer equality on
lines, branches and functions; it also refuses attribute-based source
exclusions outright, so production code cannot shrink its own
denominator. There is no threshold to lower.

Lint configuration lives once, in `[workspace.lints]` in the root
`Cargo.toml`, and every crate inherits it through `[lints] workspace =
true` — so no crate can quietly hold a different opinion. The
warnings-as-errors escalation is the `-D warnings` flag on the command
above; run that exact line and your Clippy is CI's Clippy.

### The flow

- **A worktree per slice.** Work on one slice happens in its own git
  worktree off this repository, so the main checkout stays clean and
  parallel slices never share a dirty tree. Branch, deliver, verify,
  then hand the branch back.
- **Tests are part of the change, not an afterthought.** Extend the
  suite that proves the code you touched.
- **Frozen means frozen.** The v1 `contracts/`, the `fixtures/`
  evaluator corpus, `policy/phase-machine.json` and `reference/` are
  read-only. A contract change is a new version file, never an edit.
- **Semantic changes need a decision.** Write it as `proposed` under
  `docs/decisions/` and let the operator rule.
- **The operator keeps push and merge.** Nothing here pushes on your
  behalf.

### Contribution licensing

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.

## Acknowledgments

The standing-overseer concept reached this product by way of the
lieutenant in Robert C. Martin's
[SwarmForge](https://github.com/unclebob/swarm-forge). The idea is
credited here and nothing else is taken: SwarmForge carries no license,
which means all rights reserved, so no code, scripts, prompts or prose
from it has entered — or may enter — this tree.

**Muninn** is an independent design with an inverted authority model,
described in decision 0020 and built as `brokkr muninn`: it reads the
journal, proposes to the operator, and rules nothing.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms
or conditions.

Permissive, never copyleft, and the pair every Rust developer already
knows: [decision 0018](docs/decisions/0018-dual-license.md) has the
reasoning, including why public-domain-style licenses were rejected.
