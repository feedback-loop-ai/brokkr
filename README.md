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
- **The strategy loop** (decision 0010): a library of recipes
  (`forge recipes list|add`, installable from git), swap by name,
  re-run a past feature under another recipe, compare outcomes —
  decision trails with first divergence, per-seat costs, verdict
  deltas. A pure read over two journals; works on live runs.
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
forge replay · export · verify-run · anchor · costs
```

The readouts share ONE derivation (decision 0013): `forge-view` turns a
journal into view models, and each surface only renders them — so
"what did this seat cost" has a single answer, tested once. `runs`,
`inspect` and `watch` each take `--json` to emit that model verbatim;
`inspect` takes `--phase` and `--seat` as the scoping verbs the
console's clicks became. Colour follows `NO_COLOR` and `TERM`, width
follows `COLUMNS`; without a Unicode-width dependency, CJK and emoji
columns misalign — stated rather than pretended away.

`forge ui` serves an embedded, loopback-only, read-only surface: runs,
live seat activity, the causal event timeline. `forge anchor` records
journal heads in `refs/forge/<run>` commit chains — tamper evidence.
`forge costs` reports per-seat attempts, turns, and USD — the LaneTally
join surface.

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
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2` and `forge-run-manifest/v2`. |
| `bundles/` | System recipes: `self` (self-delivery) and `verify` (the verification agents). |
| `recipes/` | The user recipe library (`fast`, `panel-review`, yours). |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| `docs/decisions/` | The constitution: numbered operator rulings 0001–0013. |
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
