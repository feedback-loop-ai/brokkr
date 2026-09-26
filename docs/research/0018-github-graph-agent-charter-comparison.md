# 0018 — GitHub peers for Brokkr's graphs, agents and charters

Source: https://github.com/microsoft/amplifier-foundation (multi-repository survey; pinned sources below)
Authors: repository maintainers cited below; comparison synthesized by Codex
Read: 2026-09-26
Status: proposed
Intake: operator-requested GitHub research, 2026-09-26; scope corrected and repository intake requested, 2026-09-27


Research date: 2026-09-26. Assessment of architectural similarity, not a performance or security certification.

Scope updated 2026-09-27: Spec Kit is part of Brokkr's existing toolchain and is excluded from this peer comparison.

## Summary

Yes. Several repositories implement substantial parts of this approach, and a few combine enough of them to be direct architectural peers. The strongest shortlist is:

1. **Microsoft Amplifier Foundation + Recipes:** reusable agents, composable bundles, executable recipes, dependency identity and resume provenance.
2. **Kilroy + the StrongDM Attractor specification:** declarative graphs, deterministic traversal, coding-agent execution, checkpoints and run evidence.
3. **BMAD Loop:** a deterministic software-delivery state machine around disposable coding/review sessions.
4. **Squad:** explicit agent charters, ownership, routing and persistent team context, with a charter compiler.

GitHub Agentic Workflows and aistack add strong comparisons for permission boundaries, structured effects and audit trails. My research did not establish a complete equivalent to Brokkr's particular combination, but it does establish that the individual ideas and several substantial combinations have other implementations. Claims of uniqueness need to be much narrower than graphs, charters, deterministic orchestration or hashed evidence.

## Findings

The classifications below relate the observed practices to Brokkr. They are
proposed research classifications; the scope correction excludes Spec Kit,
and recording this survey does not approve adopting another framework.

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep reusable agent mandates and recipe composition in reviewable data | implemented | decision 0016 and decision 0017; `agents/charters/` and `crates/brokkr-runtime/src/bundle.rs` provide charter loading and recipe composition; duplicated definitions remain a separate maintenance concern |
| 2 | Let deterministic code control progression around fallible agent effects | implemented | decision 0001 and decision 0002; `crates/brokkr-core/src/policy.rs` evaluates the policy and `crates/brokkr-runtime/src/engine.rs` drives effects; the previously identified enforcement gaps remain open |
| 3 | Support arbitrary branching and concurrent outer workflow graphs | alternative | decision 0002: one active outer phase and a totally ordered journal, with panel concurrency inside an effect |
| 4 | Bind execution to recorded recipe and dependency identity | implemented | decision 0016 and decision 0017; `crates/brokkr-runtime/src/bundle.rs` resolves and pins configuration into the manifest; this is configuration identity, not proof of agent-output correctness |
| 5 | Treat checkpoint recovery, audit integrity and event-derived state as separate guarantees | alternative | decision 0002 and decision 0008; `crates/brokkr-core/src/fold.rs` and `crates/brokkr-store/src/lib.rs` supply journal-derived state and hash integrity; unsigned evidence does not authenticate its writer |
| 6 | Qualify lifecycle, replay and recovery behavior under declared failure conditions | planned | #435 owns operational qualification; #431 owns restore evidence and #434 owns effect replay semantics |
| 7 | Measure independent delivery outcomes and the scope of real maintenance changes | planned | #436 owns independent delivery and usability evaluation; #437 owns structural change and maintenance evidence; #439 owns independent qualification |
| 8 | Prototype equivalent recipes on Amplifier, Kilroy, BMAD Loop and Squad to compare their guarantees directly | not-planned | |

## Candidates

Finding 8 is the concrete follow-up proposed by this survey. The experiment
below can inform the existing qualification work without treating any peer's
architecture or reliability claims as already proven. No framework adoption,
code reuse or provider run is authorized by this research entry.

## Method and comparison baseline

I searched GitHub-indexed primary sources using combinations of graph, workflow, charter, constitution, deterministic policy, recipes, pinned dependencies and hash-chained journals; checked metadata for more than twenty repositories; captured default-branch commit identities; and read selected implementations, schemas, examples and documentation for the closest matches. The revised comparison retains eighteen pinned repository revisions. Source links below pin the inspected commits. Some repository default branches are development branches; this review does not imply all inspected features are in published releases.

No downloaded project code was installed or executed. Descriptions of implemented mechanisms come from source inspection. Claims about production reliability, security completeness, adoption or comparative outcomes were not independently validated. Absence in the reviewed files means unestablished, not proven absent everywhere. Stars were collected for discovery context and did not determine architectural fit.

For Brokkr, the reference is the earlier source review at `8d7da844` and its later comparison with `6ba54c1c`, alongside the local architecture/decision documents. Brokkr's outer execution model is constitutionally a **linear phase machine**, with internal panel concurrency. A rendered graph does not make it an arbitrary concurrent graph executor. Its important combination is reusable role/charter/configuration data, composed recipes, deterministic policy decisions, bounded effects, pinned run identity, and event-derived evidence. Previously identified lifecycle, policy-validation and resource/retry gaps remain relevant; the intended guarantees must not be treated as universally proven today.

## Comparison at a glance

These are qualitative fit judgments from the cited inspection. “Different” describes scope or semantics, not inferiority.

| Project | Closest overlap | Important distinction |
|---|---|---|
| Amplifier Foundation + Recipes | Agent definitions as reusable configuration; bundle composition; recipe execution; dependency locks and resume checks | Extensible session/module architecture; recipe engines have permissive result/error modes; no equivalent pure journal fold established |
| Kilroy / Attractor | Text graphs; deterministic edge selection; agent/tool/human nodes; retries, parallel branches and checkpoints | General directed graphs and Git/CXDB recovery; Attractor upstream is a specification repository |
| BMAD Loop | Coded lifecycle, verification, review loops, retry policy, adapters, TUI and resume | Specializes in BMAD story delivery; fixed lifecycle rather than Brokkr's general recipe-policy model |
| Squad | Named agents, literal charters, ownership, boundaries, model preferences, routing and team memory | Human-directed coordinator/team model; editable team state rather than an established immutable run constitution |
| GitHub Agentic Workflows | Declarative source compiled to executable workflows; constrained agents and validated write effects | Runs on GitHub Actions; not a local general-purpose charter/phase engine |
| aistack | Workflow DSL, reusable agents, adversarial review, checkpoints and a hash-linked SQLite audit log | Broader service/MCP architecture; inspected DSL allows some missing inputs to become empty strings |
| LangGraph | Stateful graph runtime, persistence, interrupts and replay/resume building blocks | Application authors supply role/charter semantics and delivery policy |
| Microsoft Agent Framework | Typed workflow/executor infrastructure and declarative YAML authoring | General agent SDK; project governance and delivery semantics are application work |
| CrewAI | Role-oriented agents and stateful code-controlled Flows | Roles/backstories and flows do not establish Brokkr's entire evidence/policy contract |
| Metaswarm | Detailed specialists, independent review, bounded rework and delivery skills | Reviewed execution policy is substantially expressed in skills interpreted by the coding assistant |
| Gas Town | Named operating roles, TOML formulas, durable work coordination and agent workspaces | Coordinator/workforce organization and Beads-backed work state |
| Charter | Architectural intent linked to deterministic checks and adversarial validation | Small policy-enforcement proof of concept, not a multi-agent graph runtime |
| Agent Constitution | Structured challenge/defense/judgment plus hash-linked governance records | Decision-review harness; no equivalent event-sourced delivery machine established |
| InterAgents | Role-specific agents, explicit policy constraints and execution traces | Narrow local runtime/research-critique-writing scope |
| SwarmForge | Composed constitutions, role prompts, agent packs and durable handoffs | Already acknowledged in Brokkr; product variants live on separate branches |

## 1. Amplifier: strongest match for reusable building blocks

Foundation uses one configuration model for bundles and agents. Markdown with structured frontmatter carries the configuration, specialized context and composition references. This closely matches the motivation for keeping agent definitions and their mandates outside orchestration code. [Agent authoring](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/docs/AGENT_AUTHORING.md) and [bundle composition](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/docs/BUNDLE_GUIDE.md).

Recipes supply executable YAML steps, delegation, conditions, repeated/parallel work, approval pauses and persisted session state. The inspected v2 design resolves agents from the recipe's declared dependency closure. Its lock implementation separates locked, explicit-update and unlocked modes; its run manifest records recipe/dependency identity, and `check_resume_provenance` rejects changed recipes and changed, added or missing dependencies. This is a substantial overlap with pinned execution identity, beyond prompt packaging. [Lock implementation](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/lockfile.py), [resume provenance](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/src/amplifier_recipe_runner/provenance.py).

The parity document distinguishes a standalone library executor from the legacy in-session executor. It records declaration-order execution with advisory `depends_on`, optional aggressive JSON extraction, and configurable error handling that can continue or finish early. Those choices deserve explicit comparison with Brokkr's strict typed results and closed policy vocabulary. Their presence does not invalidate the orchestration approach. [Executor semantics](https://github.com/microsoft/amplifier-bundle-recipes/blob/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63/docs/EXECUTOR_PARITY.md).

**What to study:** authoring ergonomics, declared dependencies, reusable bundle fragments and engine conformance testing. **Question to test:** can an equivalent Brokkr recipe, including protected review and uncertain-effect behavior, be expressed without custom control-plane code?

## 2. Kilroy and Attractor: strongest match for explicit graphs

StrongDM's upstream Attractor repository contains natural-language specifications, not a runnable reference implementation. Its graph specification covers DOT nodes/edges, handlers, model selection, validation, human input and deterministic traversal. It permits several agent backend strategies. [Repository scope](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/README.md) and [graph specification](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md).

Kilroy is an implementation in Go. Its edge-selection code evaluates conditions and records routing reasons. The engine checks goal gates before completing and creates execution checkpoints. Git worktrees/commits preserve code history; CXDB supplies typed run events and artifact storage. The input-lineage implementation tracks digests and branch revisions. [Routing](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/next_hop.go), [engine and checkpoints](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/engine.go), [input lineage](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/input_snapshot_lineage.go).

The graph can contain general branching and parallel execution. That has a wider scheduling surface than Brokkr's intentionally linear outer machine. Checkpoint-based recovery also needs separate comparison with replaying a canonical journal. Neither mechanism implies external effects are safe to repeat. Its Unix code explicitly manages process groups, making lifecycle behavior a useful comparison against Brokkr #403. [Process ownership](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/process_group_unix.go).

**What to study:** graph authoring, preflight feedback, deterministic route explanations and artifact inspection. **Question to test:** how do both systems behave when a process commits an external effect and loses the acknowledgement?

## 3. BMAD Loop: a direct software-delivery control-plane peer

BMAD Loop puts ordinary Python in charge of delivery progression and delegates creative work to coding sessions. Its runtime imports typed phases, policy and recovery components. The transition table explicitly rejects illegal moves, while allowing configured paths such as disabling review. The architecture is substantially closer to Brokkr than a generic prompting toolkit. [Engine](https://github.com/bmad-code-org/bmad-loop/blob/87e5687f05717f5503b696974220cf2a91399664/src/bmad_loop/engine.py) and [transition table](https://github.com/bmad-code-org/bmad-loop/blob/87e5687f05717f5503b696974220cf2a91399664/src/bmad_loop/statemachine.py).

It supports separate implementation/review contexts, configured verification, multiple CLI adapters, optional worktree isolation and operator-facing run inspection. It identifies itself as early beta. [Product scope and status](https://github.com/bmad-code-org/bmad-loop/blob/87e5687f05717f5503b696974220cf2a91399664/README.md).

Its reviewed journal writes JSONL alongside saved state. The append implementation documents limitations around torn writes, concurrency and fsync; this is materially different from Brokkr's SQLite append transaction and hash-chain/fold design. This comparison is about the inspected mechanism, not a complete recovery audit. [Journal implementation](https://github.com/bmad-code-org/bmad-loop/blob/87e5687f05717f5503b696974220cf2a91399664/src/bmad_loop/journal.py).

**What to study:** small deterministic delivery policies, adapter contracts, operator diagnostics and recovery UX. **Question to test:** which reviewer-bypass and completion behaviors can be changed through configuration, and which remain mandatory invariants?

## 4. Squad: the most literal charter match

Squad's charter template records an agent's identity, owned responsibilities, working approach, boundaries and model preference. Its persistent team directory adds routing, shared decisions and individual history. [Charter template](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/.squad-templates/charter.md) and [team architecture](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/docs/src/content/docs/concepts/architecture.md).

This has executable support: `compileCharterFull` parses the document into a typed configuration and assembles the prompt with team/routing/decision context. Routing code also parses rules and compiles matching structures. Calling this “only Markdown” would miss real implementation. The degree to which every prose boundary is enforced independently of model behavior still needs targeted verification. [Charter compiler](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/agents/charter-compiler.ts) and [routing implementation](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/config/routing.ts).

**What to study:** charter authoring, separation of stable responsibility from accumulated memory, discoverability and team setup. **Question to test:** can changing shared team memory alter an in-flight run's effective authority, and what is recorded when it does?

## 5. GitHub Agentic Workflows: useful enforced-effect separation

`gh-aw` compiles Markdown/frontmatter into GitHub Actions workflows. Its supported default puts agents in a constrained read-oriented job, while configured writes are validated and applied through separate jobs with scoped permissions. That maps closely to the architectural concern of separating model proposals from authorized effects. [Execution model](https://github.com/github/gh-aw/blob/d3f48b29ed646b67afb67e452021842febe87c4b/README.md) and [architecture](https://github.com/github/gh-aw/blob/d3f48b29ed646b67afb67e452021842febe87c4b/docs/src/content/docs/introduction/architecture.mdx).

The implementation target is repository automation on Actions. It should be compared as a source compiler and permission/effect architecture, rather than assumed to replace a local delivery runtime. Its code and safe-output reference also illustrate the complexity of enforcing an expanding effect vocabulary. [Compiler](https://github.com/github/gh-aw/blob/d3f48b29ed646b67afb67e452021842febe87c4b/pkg/workflow/compiler.go) and [safe-output contracts](https://github.com/github/gh-aw/blob/d3f48b29ed646b67afb67e452021842febe87c4b/.github/aw/safe-outputs-runtime.md).

## 6. aistack: direct overlap in governance and evidence

aistack has an executable workflow DSL, iterative coder/adversarial review coordination, checkpoint support and portable agent definitions. The DSL supports conditional skips, parallel blocks and bounded loop-back. [DSL executor](https://github.com/blackms/aistack/blob/dda4ae3372bcbee37463f3ca876a2dd08f3f402d/src/workflows/dsl/executor.ts), [review coordinator](https://github.com/blackms/aistack/blob/dda4ae3372bcbee37463f3ca876a2dd08f3f402d/src/coordination/review-loop.ts), [portable agent schema](https://github.com/blackms/aistack/blob/dda4ae3372bcbee37463f3ca876a2dd08f3f402d/src/agents/portable-schema.ts).

Its audit implementation computes hash-linked records inside an immediate SQLite transaction and supports optional HMAC authentication. Thus hash-chained agent evidence is clearly not unique to Brokkr. An audit log alongside execution does not, by itself, establish that the full runtime state is a pure fold of that log. I did not establish equivalence to that Brokkr property. [Audit implementation](https://github.com/blackms/aistack/blob/dda4ae3372bcbee37463f3ca876a2dd08f3f402d/src/audit/chain.ts).

One concrete semantic difference: missing DSL variable references can resolve to an empty string in the inspected executor. Brokkr's handling of absent policy inputs is deliberately stricter. Broader claims of production readiness on either side require independent failure testing.

## Other relevant repositories

- **LangGraph:** relevant for state schemas, graph execution and persistent orchestration. It leaves the application to define its governance and delivery contract. Useful infrastructure comparison, with no requirement that users adopt LangChain. [Overview](https://github.com/langchain-ai/langgraph/blob/7daa3ab49d678a5da75edb08baa87db4a2be52c3/README.md) and [state graph implementation](https://github.com/langchain-ai/langgraph/blob/7daa3ab49d678a5da75edb08baa87db4a2be52c3/libs/langgraph/langgraph/graph/state.py).
- **Microsoft Agent Framework:** .NET/Python workflow infrastructure, including YAML workflow construction and mixed agents/executors. Compare typed edges, input handling and checkpoints; it is not an out-of-the-box Brokkr charter system. [Declarative workflows](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/declarative-agents/workflow-samples/README.md).
- **CrewAI:** agents with role-oriented configuration and Flows with state, listeners and routers. Relevant for authoring and orchestration ergonomics. [Flow mechanics](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/docs/edge/en/concepts/flows.mdx).
- **Metaswarm:** a close methodological comparison for specialist mandates and independent/adversarial review. Its inspected execution loop lives in a skill document; that does not establish engine-enforced transitions. [Execution skill](https://github.com/dsifry/metaswarm/blob/33d39f776f7fe29098dcf048955756a237e8cb40/skills/orchestrated-execution/SKILL.md).
- **Gas Town:** TOML formulas, work tracking, named operational roles and persistent agent workspaces. Its Mayor-led coordination is a different allocation of decision authority. [Overview](https://github.com/gastownhall/gastown/blob/649b832b7672bc7a2dbef26f5983aba6198b819b/README.md) and [formula composition](https://github.com/gastownhall/gastown/blob/649b832b7672bc7a2dbef26f5983aba6198b819b/docs/design/formula-resolution.md).
- **Charter:** an explicitly small proof of concept that binds architectural decisions to checks and tests the checks adversarially. It is especially relevant to #330's principle-to-enforcement mapping. Its security document carefully separates local approval from protection against malicious committers. [Scope](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/README.md) and [trust model](https://github.com/cspergel/Charter/blob/3a46876de80d17d5d94682160835f5695c3487a4/SECURITY.md).
- **Agent Constitution:** structured review and governance history. Its source implements chained records and explicitly limits their assurance: signatures are unimplemented, and integrity alone does not prove model identity. [Governance chain](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/governance_chain.py).
- **InterAgents:** a small policy-constrained message-passing runtime with research/critique/writing roles and traces. Relevant as a focused core comparison. [Scope](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/README.md).
- **SwarmForge:** constitutions and role prompts compose with shared runtime and product-specific branches. Brokkr already credits its lieutenant concept; the main landing repository alone is not a runnable product. [Composition and products](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/README.md).

OpenSpec, Ruflo and Get Shit Done were also screened at repository/discovery level but were not given equivalent implementation review here. The inspected `gsd-build/get-shit-done` repository was archived at research time; that observation does not establish the status of successor projects. No architecture conclusion about these three rests solely on their popularity or descriptions.

## What this means for Brokkr

The general approach has substantial independent precedent. The strongest potential distinction is the **coherence and enforceability of the complete contract**: linear phase authority, typed evidence, protected review, pinned composition, event-derived state, uncertain-effect handling and independently inspectable delivery history. This is a positioning hypothesis to prove; each part has peers, and the earlier Brokkr audit found gaps in enforcement.

The current evidence does not justify claiming that Brokkr is the only system with charters, declarative graphs, deterministic control or immutable dependency identity. It also does not justify assigning the peers numerical quality grades from source inspection alone.

For further comparison, prioritize Amplifier for composition, Kilroy for graphs/run evidence, BMAD Loop for the delivery loop, and Squad for charters. Add aistack and gh-aw for effect/audit boundaries.

## Concrete comparison experiment

Use the same small delivery task and controlled fake effects first, followed by separately authorized live-provider trials. Measure:

1. Author a reusable implement/verify/review recipe, then vary the implementation role without copying the whole configuration.
2. Make an agent return malformed or incomplete evidence, or claim review passed without the required effect.
3. Modify a charter or dependency between pause and resume and observe refusal, preservation or explicit migration.
4. Kill a driver after an external effect but before its acknowledgement; inspect what can safely resume or retry.
5. Produce a success result while descendants still hold pipes, and measure complete settlement.
6. Reconstruct the chosen route and configuration identity using retained evidence, without rerunning a model.
7. Give another maintainer a bounded new adapter/role task; count semantic edit sites and missing documentation.

These experiments fit existing #435–#439 qualification owners. This research entry does not launch them or modify the backlog. The results would distinguish attractive abstractions from guarantees that hold under the same workload and failure conditions.

## Pinned repositories

The following identities make the source inspection reproducible. Dates are default-branch commit timestamps, not release dates or proof of maintenance quality.

| Repository | Inspected commit | Commit date |
|---|---|---|
| [strongdm/attractor](https://github.com/strongdm/attractor) | [`fb57a55ed973`](https://github.com/strongdm/attractor/tree/fb57a55ed97372a27ac90102f436947e29f48426) | 2026-03-17T21:33:04Z |
| [danshapiro/kilroy](https://github.com/danshapiro/kilroy) | [`b55fb0f2b3d5`](https://github.com/danshapiro/kilroy/tree/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f) | 2026-04-27T17:57:05Z |
| [microsoft/amplifier-foundation](https://github.com/microsoft/amplifier-foundation) | [`89575c3482e3`](https://github.com/microsoft/amplifier-foundation/tree/89575c3482e3e8afe5a03df72e723cf815fa1f6c) | 2026-09-25T13:00:29Z |
| [microsoft/amplifier-bundle-recipes](https://github.com/microsoft/amplifier-bundle-recipes) | [`f8ec2ec8190e`](https://github.com/microsoft/amplifier-bundle-recipes/tree/f8ec2ec8190ec55309cae784a98a1f7bfcdd1d63) | 2026-09-21T10:24:47Z |
| [bradygaster/squad](https://github.com/bradygaster/squad) | [`0f2586ea7ca5`](https://github.com/bradygaster/squad/tree/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b) | 2026-09-25T14:28:46Z |
| [bmad-code-org/BMAD-METHOD](https://github.com/bmad-code-org/BMAD-METHOD) | [`5e33d3c03ba5`](https://github.com/bmad-code-org/BMAD-METHOD/tree/5e33d3c03ba53187a40ab679d5479cdd4b6ac2fb) | 2026-09-25T14:17:11Z |
| [dsifry/metaswarm](https://github.com/dsifry/metaswarm) | [`33d39f776f7f`](https://github.com/dsifry/metaswarm/tree/33d39f776f7fe29098dcf048955756a237e8cb40) | 2026-06-19T22:25:25Z |
| [langchain-ai/langgraph](https://github.com/langchain-ai/langgraph) | [`7daa3ab49d67`](https://github.com/langchain-ai/langgraph/tree/7daa3ab49d678a5da75edb08baa87db4a2be52c3) | 2026-09-23T17:56:01Z |
| [crewAIInc/crewAI](https://github.com/crewAIInc/crewAI) | [`4ed2abc7bbf5`](https://github.com/crewAIInc/crewAI/tree/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec) | 2026-09-25T18:00:20Z |
| [microsoft/agent-framework](https://github.com/microsoft/agent-framework) | [`6f1522a50b66`](https://github.com/microsoft/agent-framework/tree/6f1522a50b66f117da34cc25ea299ba24a528b15) | 2026-09-25T18:52:55Z |
| [unclebob/swarm-forge](https://github.com/unclebob/swarm-forge) | [`f4f5fbcae0de`](https://github.com/unclebob/swarm-forge/tree/f4f5fbcae0de6f7dcc26e82400334227647cfdb2) | 2026-09-04T14:22:18Z |
| [cspergel/Charter](https://github.com/cspergel/Charter) | [`3a46876de80d`](https://github.com/cspergel/Charter/tree/3a46876de80d17d5d94682160835f5695c3487a4) | 2026-06-18T12:13:40Z |
| [AgentPolis/agent-constitution](https://github.com/AgentPolis/agent-constitution) | [`1408e3ace216`](https://github.com/AgentPolis/agent-constitution/tree/1408e3ace216b9a42da5a409210ca842872f4cfe) | 2026-04-14T08:55:36Z |
| [github/gh-aw](https://github.com/github/gh-aw) | [`d3f48b29ed64`](https://github.com/github/gh-aw/tree/d3f48b29ed646b67afb67e452021842febe87c4b) | 2026-09-26T17:07:07Z |
| [gastownhall/gastown](https://github.com/gastownhall/gastown) | [`649b832b7672`](https://github.com/gastownhall/gastown/tree/649b832b7672bc7a2dbef26f5983aba6198b819b) | 2026-07-23T13:03:02Z |
| [blackms/aistack](https://github.com/blackms/aistack) | [`dda4ae3372bc`](https://github.com/blackms/aistack/tree/dda4ae3372bcbee37463f3ca876a2dd08f3f402d) | Not separately recorded |
| [jgzadidiLGDY/InterAgents](https://github.com/jgzadidiLGDY/InterAgents) | [`d0cba2221084`](https://github.com/jgzadidiLGDY/InterAgents/tree/d0cba222108410112db271037c008cc6f6836c5a) | Not separately recorded |
| [bmad-code-org/bmad-loop](https://github.com/bmad-code-org/bmad-loop) | [`87e5687f0571`](https://github.com/bmad-code-org/bmad-loop/tree/87e5687f05717f5503b696974220cf2a91399664) | Not separately recorded |
