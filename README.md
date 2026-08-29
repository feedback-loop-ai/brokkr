<p align="center"><img src="assets/logo.svg" width="132" alt="The Forge — sealed anvil mark, terminal rail node pulsing"></p>

# The Forge

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

## What it does

You hand the Forge a feature and a **recipe** (a delivery strategy:
policy table, seats, charters, limits, drivers — reviewable text,
identified by content digest). The engine drives real agent sessions
through the recipe's phases — implement, verify, review, ship — ruling
on each typed result with a pinned first-match-wins policy. Unknowns
never advance: schema violations, unmatched results, exhausted retries,
and security findings park or stop the run with raw evidence attached.
The operator's judgment enters only as signed journal events.

```
forge run --recipe fast --repo . --feature "…"     # deliver
forge watch --run <id>                              # watch it live, in the terminal
forge tui                                           # explore the fleet with the keyboard
forge ui                                            # watch it live, in a browser
forge rerun --run <id> --recipe panel-review        # swap the strategy
forge compare <a> <b>                               # journal-backed A/B
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
  (`forge recipes list|add|show`, installable from git), swap by name,
  re-run a past feature under another recipe, compare outcomes —
  decision trails with first divergence, per-seat costs, verdict
  deltas. A pure read over two journals; works on live runs.
  Recipes **compose**: `extends: "sdd"` plus one override is a whole
  strategy (`recipes/sdd-paranoid`, sixty lines against SDD's 227).
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
- **Self-hosting**: the Forge delivers its own changes (`bundles/self`)
  and verifies every delivered slice with its own adversarial agents
  (`bundles/verify`) — which have hard-stopped their author's work on
  real security findings, twice. The operator keeps push and merge
  authority.

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

## Install & operate

One native binary — no Python, no Node, no services. Grab a
[release](../../releases) (linux x86_64/aarch64, macOS arm64/x86_64,
windows; verify against `SHA256SUMS` and the signed GitHub build attestation), then:

```
forge init my-bundle        # scaffold a reviewable starter recipe
forge doctor                # tools, agent CLIs, database, contracts
forge run …                 # deliver (exit 0 done · 2 parked · 3 stopped)
forge runs                  # one clamped line per run, newest first
forge inspect --run <id>    # header, ruling, seats, trail, phase tree
forge watch --run <id>      # the same, live, until the run concludes
forge tui [--run <id>]      # the same three levels, navigable with keys
forge replay · export · verify-run · anchor · costs
```

`forge tui` is the readouts made explorable (decision 0014): arrow keys
or `j`/`k` move, `Enter` descends from the run list to a run to one
seat's own stream, `Esc` comes back, `/` filters, `?` opens help, and a
footer names the keys of wherever you are. It is read-only exactly as
every other readout is — no operator commands, no run starts, nothing
written to the journal, and a missing database refuses rather than
creating one.

The readouts share ONE derivation (decision 0013): `forge-view` turns a
journal into view models, and each surface only renders them — so
"what did this seat cost" has a single answer, tested once. `runs`,
`inspect` and `watch` each take `--json` to emit that model verbatim;
`inspect` takes `--phase` and `--seat` as the scoping verbs the
console's clicks became. `--run` takes a selector, not only the
41-character id: any unique run-id prefix, or `latest` for the newest
run in the workspace database (decision 0015) — one resolver, shared by
`watch`, `inspect`, `anchor`, `export` and `replay`. Colour follows `NO_COLOR` and `TERM`, width
follows `COLUMNS`; without a Unicode-width dependency, CJK and emoji
columns misalign — stated rather than pretended away.

`forge ui` serves an embedded, loopback-only, read-only surface: runs,
live seat activity, the causal event timeline. `forge anchor` records
journal heads in `refs/forge/<run>` commit chains — tamper evidence.
`forge costs` reports per-seat attempts, turns, and USD — the LaneTally
join surface.

## The agent library

A seat used to inline everything it was: charter text, driver argv,
limits, declared inputs. Decision 0016 lets it name an agent instead.

```
forge agents list           # name · model chain · description
forge agents show <name>    # the definition, plus its per-entry resolution
forge doctor                # which providers and models are actually here
```

An agent is one file in `agents/`: a description, a charter, an ORDERED
preference chain of abstract model names, abstract tool and MCP
configuration, its decision-0006 limits and its decision-0007 declared
inputs. A seat, panel member or sequence step says `"agent": "<name>"`.
Inline seats stay first-class — `recipes/sdd`'s `speckit-check` step is
a shell script with no model, and it stays inline.

A provider adapter is **data**, one file per provider in `adapters/`:
the driver invocation, the abstract→concrete model mapping, how tool
permissions and MCP servers are expressed, and — the load-bearing part —
which of those the provider **cannot** express, written as the explicit
string `"unsupported"` rather than left to be inferred from an empty
map. Adding a provider or a model is a file edit, not a release.

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
not a portability claim: the forge never says the second choice equals
the first, and `forge compare` reports a model difference as a
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
   because the forge cannot tell "did nothing" from "already opened a
   billed session". The honest fix is at the driver — report a
   provider's pre-session model rejection as a determinate failure — not
   at the engine, because a bound that applies "unless a new feature is
   in play" has stopped being a bound.

Looper-bound runs start with `forge run --dispatch <forge-dispatch-v2.json>`.
The immutable dispatch is sealed into the v2 run manifest and therefore travels
with `forge export`. `forge bridge --run <id> --looper-url <url>` tails only the
verified public store API and synchronizes ordered evidence plus fenced commands;
it reads its bearer credential from `LOOPER_API_KEY` (or `--token-env`), never
from a command-line value or the journal.

## Repo layout

| Path | What it is |
|---|---|
| `ARCHITECTURE.md` | The implemented architecture — crates, journal, effect discipline, verification layers. |
| `crates/` | The engine: `forge-core` (pure) · `forge-store` · `forge-protocol` (+ built-in claude/codex/dsh/exec adapters) · `forge-runtime` · `forge-view` (one display derivation, no I/O) · `forge-bridge` · `forge-cli`. |
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2`, `forge-run-manifest/v2` and `/v3`, and `forge-effect-provenance/v1`. |
| `bundles/` | System recipes: `self` (self-delivery) and `verify` (the verification agents). |
| `recipes/` | The user recipe library (`fast`, `panel-review`, `sdd`, `sdd-paranoid` — which `extends` `sdd` — yours). |
| `agents/` | The agent library (decision 0016): one definition per agent plus the charters seats used to inline. |
| `adapters/` | One data file per provider: driver invocation, abstract→concrete model mapping, and what the provider CANNOT express. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| `docs/decisions/` | The constitution: numbered operator rulings 0001–0017. |
| `reference/` | Read-only heritage documents: handoff-protocol lore, recorded schemas. |

## Verification

Four layers, each mechanical: the 97-case differential corpus pins the
evaluator; the machine-proof suite drives the real binary through every
failure mode (30+ scenarios, three OSes, coverage-gated CI); self-forge
runs deliver changes under the full constitution; and the verify agents
adversarially review every landed slice — their verdicts are journaled
runs like any other.

Release admission additionally requires canonical formatting, warning-free
Clippy across all targets and features, a RustSec dependency audit, literal
nonzero 100% source-line/branch/function coverage, frozen/additive contract
compilation, producer-bridge conformance, and a checksum-verified platform
matrix. Release archives and `SHA256SUMS` carry GitHub Sigstore build-provenance
attestations; verify an asset with
`gh attestation verify <asset> -R feedback-loop-ai/the-forge`.

Private for now; the OSS boundary decision (naming, license,
threat-model README) comes after the engine drives a real feature end
to end in an external workspace.
