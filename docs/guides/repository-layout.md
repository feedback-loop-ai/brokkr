# Repo layout

This repository is a recipe library, not an adopter's scaffold, so it
keeps the library layout: its `recipes/`, `agents/`, `adapters/` and
`realms.json` stand at the root, and its runs write their state into
`.forge/` (decision
[0083](../decisions/0083-an-adopters-brokkr-lives-in-one-folder.md), the
operator's ruling of 2026-10-13). An adopter's repository holds none of
this: `brokkr init .` writes everything Brokkr needs there into one
`.brokkr/` folder, as the [quickstart](quickstart.md#step-2--brokkr-init-)
shows.

| Path | What it is |
|---|---|
| [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | The implemented architecture — crates, journal, effect discipline, verification layers. |
| [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | The mandatory sixty-second path: choose a recipe, run Brokkr, publish its evidence, and name the run in the pull request. |
| `crates/` | The engine: `brokkr-core` (pure) · `brokkr-store` · `brokkr-protocol` (+ built-in claude/codex/dsh/exec adapters) · `brokkr-runtime` · `brokkr-view` (one display derivation, no I/O) · `brokkr-bridge` · `brokkr-cli` (builds `brokkr`, the only binary). |
| `contracts/` | Frozen v1 contracts plus additive `forge-dispatch/v2`, `forge-run-manifest/v2`, `/v3` and `/v4`, `forge-effect-provenance/v1`, `forge.phase-machine/v2` (the rule-driven park, decision 0022), `forge.realms/v1` (the world's map, decision 0023), `forge.realms/v2` (a realm may name its own journal — many hearths, decision 0026), the `forge.realms/v3` and run-manifest v8 contracts for house, dialect, and per-step result vocabulary, and decision 0046's four: `realms.v4` (the realm's `boundary`), `run-manifest.v9` (the boundary pinned per hands site), `seat-record.v4` (`boundary` beside `model`) and `effect-boundary.v1` (the `effect/started.boundary` extension field). |
| `realms.json` | This repository's own map (decision 0023): one realm — this repository — its journal, and its optional `house`, `dialect` and `boundary` (decision 0046: what stands between a boxed seat's hands and this machine; absent reads `namespace`). A workspace of many projects is another file, named with `--realms`; a personal map lives outside the repository (see below). |
| `docs/house-rules.md` | This realm's repository-specific prompt text: toolchain, frozen surfaces, gates and delivery conventions. The map names it; the engine pins and renders it. |
| `recipes/` | The recipe library (`fast`, `triage` — the routing form with selected chore, feature, design, and engine crews — `night-shift`, `node`, `panel-review`, `preflight`, the explicit wager harnesses, Brokkr's own `self` (self-delivery) and `verify` (the verification agents), and yours). `fast` and `node` are portable: they install onto any folder `brokkr init` made. Every other recipe here is Brokkr-only ([decision 0087](../decisions/0087-the-base-offices-are-the-ones-init-writes.md) ruling 5). |
| `docs/releases/` | Release descriptions and reviewed handoffs, including organization-profile patches. |
| `recipes/release/` | Release preparation through the portable release manager; the realm's house file configures project details. The recipe is Brokkr-only (decision 0087 ruling 5): its release manager binds Brokkr's Rust toolchain. |
| `agents/` | The agent library (decision 0016): one definition per agent plus the charters seats used to inline. |
| `dialects/` | The dialect library (decision 0042): one JSON declaration and one instruction page per artifact per dialect. `crates/brokkr-cli/dialects/` carries the same bytes, because `brokkr init` embeds them and a published crate can hold only its own directory; the packaging contract ties the two, file for file. |
| `openspec/` | This repository's spec dialect (decision 0042), which tracks slices of work and is never the living truth (decision 0081). `openspec/changes/<change>/` is one slice in flight. `openspec/specs/` is the tool's record of the requirements archived slices left, not a contract: where it disagrees with a decision or the code, the decision and the code win. Retention: a change is archived under `openspec/changes/archive/` by the dialect's archive step when its work is done, every task ticked, and a test refuses a finished change left in place; an evidence ledger belongs in `docs/evidence/` or a run's `refs/forge` anchor, not in a change directory. |
| `adapters/` | One data file per provider: driver invocation, abstract→concrete model mapping, and what the provider CANNOT express. |
| `extensions/` | Decision 0009's extension boundary: language-neutral, repository-owned provider extension bytes (starting with the adapted `dsh-plugin-cli-session` under `extensions/dsh/plugin-cli-session/`, with `extensions/dsh/PROVENANCE.md`). No Brokkr crate builds, loads or executes them; deployment is an operator action. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the corpus derives from; stability is contract. |
| [`docs/decisions/`](../decisions/) | The constitution: numbered operator rulings, indexed with their status in its README. |
| [`docs/lore/`](../lore/) | The lore layer of [decision 0019](../decisions/0019-brokkr.md): [the Edda](../lore/edda.md) and the sagas. Commentary, never specification. |
| `assets/` | The brand mark: [`logo.svg`](../../assets/logo.svg) (the anvil and the three rail nodes) and `social-preview.png`, the 1280×640 card the repository shows when it is linked. |
| `reference/` | Read-only heritage documents: handoff-protocol lore, recorded schemas. |
| `scripts/coverage-exact.sh` | The exact-coverage gate: literal 100% line/branch/function, or refusal. |

A personal map lives outside the worktree, beside the personal recipe
shelf: `$XDG_CONFIG_HOME/brokkr/realms/<name>.json`, else
`$HOME/.config/brokkr/realms/<name>.json` (decision
[0101](../decisions/0101-personal-realms-and-adapters-live-outside-the-tracked-tree.md)
ruling 1). Nothing finds it by lookup. `--realms <path>` names it on
every verb that opens a journal or a world, and the named map is read
alone: never merged with this repository's map, never a fallback, and
missing or malformed refuses. `compile` and `recipes show` do not take
the flag yet.

Its paths are read relative to the map file, so a map moved out of this
repository writes its realm and journal as absolute paths (or paths
relative to the map) — otherwise `.forge/forge.db` lands under the
personal directory. Its `house` stays repository-relative; a named
dialect resolves beside the map (copy it there, or write the path form
that reads the tracked file); a map that declares `capabilities` carries
its `capabilities/` and `dialects/tools/` trees beside itself. The
[quickstart](quickstart.md#a-personal-map-lives-outside-the-repository)
gives the full migration.

A personal map's paths and credentials never go into the tracked
`realms.json` or a bundle (ruling 8). Credentials stay names under the
existing secret bindings (decision
[0012](../decisions/0012-sealed-secret-bindings.md)), never map fields.

A seat prompt is assembled from the portable office in
`agents/charters/`, the realm-owned house file, and the engine-owned result
contract. Recipe-local `roles/` remain appropriate when the office itself is
recipe-specific, such as a verifier script.

`brokkr doctor` reads that same map and makes the specification side visible:

```text
ok       dialect brokkr: openspec · tool 'openspec' 1.12.0 · pinned 1.12.0 · probed inside the box
ok       dialect brokkr requires openspec/config.yaml: present at …/openspec/config.yaml
```

One line names the realm's dialect, installed tool and pinned version; the
following lines report every file the dialect requires as present or missing.
