# Repo layout

| Path | What it is |
|---|---|
| [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | The implemented architecture — crates, journal, effect discipline, verification layers. |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | The mandatory sixty-second path: choose a recipe, run Brokkr, publish its evidence, and name the run in the pull request. |
| `crates/` | The engine: `brokkr-core` (pure) · `brokkr-store` · `brokkr-protocol` (+ built-in claude/codex/dsh/exec adapters) · `brokkr-runtime` · `brokkr-view` (one display derivation, no I/O) · `brokkr-bridge` · `brokkr-cli` (builds `brokkr`, the only binary). |
| `contracts/` | Frozen v1 contracts plus additive dispatch, manifest, phase-machine, seat-record and realms versions. `forge.realms/v3` adds optional house and dialect declarations; run-manifest v8 pins per-step result vocabularies. |
| `realms.json` | This repository's own map: its repository, journal, and optional `house` and `dialect`. A workspace of many projects is another file, named with `--realms`. |
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
