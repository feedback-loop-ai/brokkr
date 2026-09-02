# Repo layout

| Path | What it is |
|---|---|
| [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | The implemented architecture — crates, journal, effect discipline, verification layers. |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | The mandatory sixty-second path: choose a recipe, run Brokkr, publish its evidence, and name the run in the pull request. |
| `crates/` | The engine: `brokkr-core` (pure) · `brokkr-store` · `brokkr-protocol` (+ built-in claude/codex/dsh/exec adapters) · `brokkr-runtime` · `brokkr-view` (one display derivation, no I/O) · `brokkr-bridge` · `brokkr-cli` (builds `brokkr`, the only binary). |
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2`, `forge-run-manifest/v2`, `/v3` and `/v4`, `forge-effect-provenance/v1`, `forge.phase-machine/v2` (the rule-driven park, decision 0022), `forge.realms/v1` (the world's map, decision 0023) and `forge.realms/v2` (a realm may name its own journal — many hearths, decision 0026). |
| `realms.json` | This repository's own map (decision 0023): one realm — this repository — and the journal it writes. A workspace of many projects is another file, named with `--realms`. |
| `bundles/` | System recipes: `self` (self-delivery) and `verify` (the verification agents). |
| `recipes/` | The user recipe library (`fast`, `node` — the Node/TypeScript reference — `panel-review`, `preflight` — the contributor's pre-flight review — `sdd`, `sdd-paranoid` — which `extends` `sdd` — yours). |
| `agents/` | The agent library (decision 0016): one definition per agent plus the charters seats used to inline. |
| `adapters/` | One data file per provider: driver invocation, abstract→concrete model mapping, and what the provider CANNOT express. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| [`docs/decisions/`](../decisions/) | The constitution: numbered operator rulings 0001–0019, indexed. |
| [`docs/lore/`](../lore/) | The lore layer of [decision 0019](../decisions/0019-brokkr.md): [the Edda](../lore/edda.md) and the sagas. Commentary, never specification. |
| `assets/` | The brand mark: [`logo.svg`](../../assets/logo.svg) (the anvil and the three rail nodes) and `social-preview.png`, the 1280×640 card the repository shows when it is linked. |
| `reference/` | Read-only heritage documents: handoff-protocol lore, recorded schemas. |
| `scripts/coverage-exact.sh` | The exact-coverage gate: literal 100% line/branch/function, or refusal. |
