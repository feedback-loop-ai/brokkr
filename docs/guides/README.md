# Guides

The quickstart is the tour. This directory is the walkthrough, written
for someone arriving from outside this repository.

| Guide | For |
|---|---|
| [Quickstart](quickstart.md) | One four-step spine — install, `init .`, `run`, read the journal — stated once, with the escape hatches and what it costs. Everything else in `docs/guides/` is a diff over it. |
| [How Brokkr works](how-brokkr-works.md) | The delivery loop and the four determinism laws that keep agent output in leaf position. |
| [Read surfaces](read-surfaces.md) | The journal-derived fleet, run, live, terminal, browser and Muninn readouts. |
| [Stack cards](cards/) | The spine's steps 2 and 3 per language — node, bun (extends node), rust, go, python — a handful of lines each, never a second walkthrough. |
| [Starter samples](starters/) | What `brokkr init` actually wrote, per stack, transcribed from real runs and annotated line by line. |
| [Recipe authoring](recipe-authoring.md) | `bundle.json` + `policy.json` + `roles/` anatomy, composition via `extends`/`override`, digest identity, the rule grammar including `visits` and the reforging ladder. |
| [Agent library](agent-library.md) | One agent definition, its ordered model chain, limits, declared inputs and tool grant, resolved once and pinned. |
| [Provider adapters](provider-adapters.md) | The data files that map abstract models and permissions onto Claude, Codex, DSH, LaneTally and exec. |
| [Driver authoring](driver-authoring.md) | The `forge-driver/v1` wire contract for a harness that is not Claude Code, Codex or dsh: handshake, `accepted`, checkpoints, results, deadlines, and the conformance suite as the acceptance test. |
| [Secrets](secrets.md) | Names cross the control plane; values stay in the operator-side store and are masked before capture. |
| [Journal and verification](journal-and-verification.md) | Anchor, export, verify, replay, import and keep the evidence that proves a run. |
| [Repository layout](repository-layout.md) | The crates, contracts, recipes, evidence and read-only shelves, with the reason each exists. |
| [Adopting a Node repo](adopting-a-node-repo.md) | The quickstart's flow 3 at length for a Node/TypeScript repo on `recipes/node`: what you are granting, the four files your repo needs, the `realms.json` it writes for itself, what each seat runs, and where the package-manager fork points are. |
| [Versioning](versioning.md) | What is stable and what may still move: the frozen-contract law, the two manifest lineages, semver as of 1.0, and the live deprecation window. |
| [Contributing by hand](contributing-by-hand.md) | The repository gates in full, retained as reference for the visible operator escape hatch. |
