# Repo layout

| Path | What it is |
|---|---|
| [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | The implemented architecture — crates, journal, effect discipline, verification layers. |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | The mandatory sixty-second path: choose a recipe, run Brokkr, publish its evidence, and name the run in the pull request. |
| `crates/` | The engine: `brokkr-core` (pure) · `brokkr-store` · `brokkr-protocol` (+ built-in claude/codex/dsh/exec adapters) · `brokkr-runtime` · `brokkr-view` (one display derivation, no I/O) · `brokkr-bridge` · `brokkr-cli` (builds `brokkr`, the only binary). |
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2`, `forge-run-manifest/v2`, `/v3` and `/v4`, `forge-effect-provenance/v1`, `forge.phase-machine/v2` (the rule-driven park, decision 0022), `forge.realms/v1` (the world's map, decision 0023), `forge.realms/v2` (a realm may name its own journal — many hearths, decision 0026), and the new `forge.realms/v3` and run-manifest v8 contracts for house, dialect, and per-step result vocabulary. |
| `realms.json` | This repository's own map (decision 0023): one realm — this repository — its journal, and its optional `house` and `dialect`. A workspace of many projects is another file, named with `--realms`. |
| `docs/house-rules.md` | This realm's repository-specific prompt text: toolchain, frozen surfaces, gates and delivery conventions. The map names it; the engine pins and renders it. |
| `bundles/` | System recipes: `self` (self-delivery) and `verify` (the verification agents). |
| `recipes/` | The user recipe library (`fast`, `triage` — the routing form with selected chore, feature, design, and engine crews — `night-shift`, `node`, `panel-review`, `preflight`, the explicit wager harnesses, and yours). |
| `agents/` | The agent library (decision 0016): one definition per agent plus the charters seats used to inline. |
| `adapters/` | One data file per provider: driver invocation, abstract→concrete model mapping, and what the provider CANNOT express. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| [`docs/decisions/`](../decisions/) | The constitution: numbered operator rulings 0001–0019, indexed. |
| [`docs/lore/`](../lore/) | The lore layer of [decision 0019](../decisions/0019-brokkr.md): [the Edda](../lore/edda.md) and the sagas. Commentary, never specification. |
| `assets/` | The brand mark: [`logo.svg`](../../assets/logo.svg) (the anvil and the three rail nodes) and `social-preview.png`, the 1280×640 card the repository shows when it is linked. |
| `reference/` | Read-only heritage documents: handoff-protocol lore, recorded schemas. |
| `scripts/coverage-exact.sh` | The exact-coverage gate: literal 100% line/branch/function, or refusal. |

A seat prompt is assembled from the portable office in
`agents/charters/`, the realm-owned house file, and the engine-owned result
contract. Recipe-local `roles/` remain appropriate when the office itself is
recipe-specific, such as a verifier script.

`brokkr doctor` reads that same map and makes the specification side visible:

```text
ok       dialect brokkr: openspec · tool 'openspec' OpenSpec 1.12.0 · pinned 1.12.0
ok       dialect brokkr requires openspec/config.yaml: present at …/openspec/config.yaml
```

One line names the realm's dialect, installed tool and pinned version; the
following lines report every file the dialect requires as present or missing.
