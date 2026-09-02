# Guides

The [quickstart](quickstart.md) is the walkthrough: install, `init .`,
run one slice and read its journal. Everything else here is the focused
guide for one operation or extension point.

## Start and adopt

| Guide | For |
|---|---|
| [Quickstart](quickstart.md) | One four-step spine — install, `init .`, `run`, read the journal — stated once, with the escape hatches and what it costs. Everything else in `docs/guides/` is a diff over it. |
| [Stack cards](cards/) | The spine's steps 2 and 3 per language — node, bun (extends node), rust, go, python — a handful of lines each, never a second walkthrough. |
| [Starter samples](starters/) | What `brokkr init` actually wrote, per stack, transcribed from real runs and annotated line by line. |
| [Adopting a Node repo](adopting-a-node-repo.md) | The quickstart's flow 3 at length for a Node/TypeScript repo on `recipes/node`: what you are granting, the four files your repo needs, the `realms.json` it writes for itself, what each seat runs, and where the package-manager fork points are. |

## Operate and prove

- [What Brokkr does](overview.md) — The product tour and the deterministic laws behind its control plane.
- [Read surfaces](read-surfaces.md) — The journal rendered as fleet, run, seat, graph, browser and Muninn views.
- [Secrets](secrets.md) — Secret names enter bundles and journals; values stay in the operator-side store.
- [Journal and verification](journal-and-verification.md) — Anchor, export, verify, replay, import and retain every exhibit a run cites.

## Extend

- [Recipe authoring](recipe-authoring.md) — `bundle.json` + `policy.json` + `roles/` anatomy, the maintained strategies, composition, digest identity and the rule grammar.
- [Agent library](agent-library.md) — Reusable charters, ordered model chains, grants, limits and fail-to-start fallback.
- [Driver authoring](driver-authoring.md) — The `forge-driver/v1` wire contract, conformance suite and provider-adapter data, including what a provider cannot express.
- [Versioning](versioning.md) — What is stable and what may still move: the frozen-contract law, the two manifest lineages, semver as of 1.0, and the live deprecation window.
- [Repository layout](repository-layout.md) — The source, contracts, policy, evidence and extension directories at a glance.
