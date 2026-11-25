# The research registry

Brokkr is built in the deep-tech mould: what the product does is
argued against what the field has found, in writing, with receipts.
This directory is where that argument is kept. Every article the
product has read gets one numbered file here, and every finding in
that article gets one row that says what Brokkr does about it and
where the reader can check (decision 0044).

The registry is a mirror of `docs/decisions/`: numbered files, an
index that is derived from them, and a status line whose acceptance is
the operator's alone. A researcher, human or seat, proposes; the
operator rules.

## The grammar of an entry

```
# NNNN — <title of the article>

Source: <url>
Authors: <names (affiliation)>
Read: <YYYY-MM-DD>
Status: proposed | ruled (<YYYY-MM-DD>)
Intake: <how it arrived: the run id, or the skill and the operator's date>

## Summary

<what the article claims, how it shows it, and its numbers>

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | <one adoptable practice, in one sentence> | <class> | <where to check> |

## Candidates

<findings the researcher flags for the operator to consider planning;
optional>
```

The classification vocabulary is closed:

| Class | Meaning | The citation must name |
|---|---|---|
| `implemented` | Brokkr does this | a decision number or a path that exists in the tree |
| `alternative` | Brokkr does something else for the same reason | the decision or path that does it |
| `declined` | against the mission or the charter, on purpose | the decision or charter line it conflicts with |
| `planned` | the operator has ruled it in | the issue number (`#NNN`) |
| `not-planned` | read, not taken up, no ruling against it | nothing |

`Status: proposed` means the classifications are the researcher's
suggestions with the evidence it found. `Status: ruled (<date>)` means
the operator has read the rows and they stand as the product's word.
Only the operator writes `ruled`. A ruled row is never edited into a
different class; a later ruling is a dated erratum inside the file.

`crates/brokkr-cli/tests/research_registry.rs` holds this file's index
equal to the entries in the directory, refuses a class outside the
vocabulary, and refuses a citation that does not resolve.

## Reconciled sweep, 2026-09-08

Entries 0008–0017 retain the source summaries and intake from the
2026-09-04 sweep. Their 46 findings were checked against main at
`7b53e92`, after the boundary enactment (decision 0046 slice (i)). Each
entry records the current evidence and remaining gaps under a dated
reconciliation. All ten entries remain `proposed`; this update records
no operator acceptance or planning ruling. A cited decision's acceptance
is not by itself evidence that its capability has been implemented.

## Ruled, 2026-11-24

The operator ruled entries 0001–0022 as recommended on 2026-11-24
([#1329](https://github.com/feedback-loop-ai/brokkr/issues/1329)): every
row stands as classified. Before the ruling, 0016 and 0022 each gained a
dated correction of a stale clause; neither changes a class. The ruling
plans no candidate. A row reads `planned` only once the operator opens
its issue.

## Index

| # | Article | Source | Findings | Status |
|---|---|---|---|---|
| [0001](0001-what-survives-the-next-model.md) | What Survives the Next Model? Benchmarking LLM-Based Techniques Against Single-Prompts | [arXiv 2609.00468](https://arxiv.org/abs/2609.00468) | 5 | ruled |
| [0002](0002-github-evaluate-llms-before-production.md) | How to evaluate LLMs before production | [github.blog](https://github.blog/ai-and-ml/llms/how-to-evaluate-llms-before-production/) | 7 | ruled |
| [0003](0003-swe-refactor-bench.md) | SWE Refactor Bench: Can Coding Agents Complete a Long-Horizon, Whole-Repository Stack Migration? | [arXiv 2608.23564](https://arxiv.org/abs/2608.23564) | 5 | ruled |
| [0004](0004-context-privilege-escalation.md) | What's in Your Agent's Context? Context Privilege Escalation Attacks against AI Agent Harness | [arXiv 2609.01222](https://arxiv.org/abs/2609.01222) | 6 | ruled |
| [0005](0005-model-based-agentic-software-engineering.md) | Model-Based Agentic Software Engineering (MAGE) | [arXiv 2608.25174](https://arxiv.org/abs/2608.25174) | 6 | ruled |
| [0006](0006-harness-of-harness.md) | Harness-of-Harness: Multi-Day Autonomous Software Development with Continual Improvement | [arXiv 2609.01481](https://arxiv.org/abs/2609.01481) | 6 | ruled |
| [0007](0007-loop-engineering.md) | Loop Engineering: Building Blocks, Adoption, and Impact | [arXiv 2608.21884](https://arxiv.org/abs/2608.21884) | 6 | ruled |
| [0008](0008-swe-gate.md) | SWE-Gate: Passing Functional Tests Is Not Enough for Software Engineering Agents | [arXiv 2609.04167](https://arxiv.org/abs/2609.04167) | 4 | ruled |
| [0009](0009-illusion-of-independent-quorums.md) | The Illusion of Independent Quorums: Epistemic Fault Domains and Correlated Cognitive Failures in Agentic Quorums | [arXiv 2609.02925](https://arxiv.org/abs/2609.02925) | 5 | ruled |
| [0010](0010-toolgate.md) | ToolGate: An Executable Acceptance Pipeline for Tool-Dependent Scientific Benchmark Construction | [arXiv 2609.02067](https://arxiv.org/abs/2609.02067) | 5 | ruled |
| [0011](0011-trajectory-aware-evaluation.md) | Efficient SWE Agent Benchmarking via Trajectory-Aware Evaluation | [arXiv 2609.01603](https://arxiv.org/abs/2609.01603) | 4 | ruled |
| [0012](0012-harnessdev.md) | HarnessDev: Can LLMs Create and Evolve Their Own Agent Harness? | [arXiv 2609.01437](https://arxiv.org/abs/2609.01437) | 5 | ruled |
| [0013](0013-heart-tool-primitives.md) | Harness Engineering in LLM Tool Use via Agent-Native Reusable Tool Primitives | [arXiv 2609.01736](https://arxiv.org/abs/2609.01736) | 5 | ruled |
| [0014](0014-progressive-risk-vesting.md) | Spawn Freely, Act Sparingly: Progressive Risk Vesting for Recursive LLM-Agent Trees | [arXiv 2609.01035](https://arxiv.org/abs/2609.01035) | 5 | ruled |
| [0015](0015-black-box-agentic-processes.md) | A Black Box for Agentic Processes: Blockchain-Anchored Evidence for AI Agent Communication, Human Oversight, and GRC Audits | [arXiv 2609.04017](https://arxiv.org/abs/2609.04017) | 5 | ruled |
| [0016](0016-extracting-architectural-decisions.md) | Can LLMs Extract Architectural Design Decisions from Source Code Commits? A Preliminary Exploratory Study | [arXiv 2609.03721](https://arxiv.org/abs/2609.03721) | 4 | ruled |
| [0017](0017-scx-router.md) | SCX Router: Streaming Zero-Shot Model Selection with a Decoder-KV Classifier and a Real-World Task Ontology | [arXiv 2609.02292](https://arxiv.org/abs/2609.02292) | 4 | ruled |
| [0018](0018-github-graph-agent-charter-comparison.md) | GitHub peers for Brokkr's graphs, agents and charters | [GitHub repositories](https://github.com/microsoft/amplifier-foundation) | 8 | ruled |
| [0019](0019-github-peer-implementation-investigations.md) | GitHub peers: implementation, failure handling and setup | [GitHub repositories](https://github.com/microsoft/amplifier-bundle-recipes) | 6 | ruled |
| [0020](0020-containing-an-attempts-process-tree.md) | Containing an attempt's process tree: what Linux and macOS offer, measured | [GitHub Actions run 38057456223](https://github.com/feedback-loop-ai/brokkr/actions/runs/38057456223) | 9 | ruled |
| [0021](0021-effect-replay-safety.md) | Effect replay safety: what each attempt's end proves, measured from the code | [issue #434](https://github.com/feedback-loop-ai/brokkr/issues/434) | 10 | ruled |
| [0022](0022-contract-version-lifecycle.md) | Contract version lifecycle: who writes and reads each file in `contracts/`, and under which ruling, measured from the code | [issue #361](https://github.com/feedback-loop-ai/brokkr/issues/361) | 11 | ruled |
| [0023](0023-truthgate.md) | Truthgate: Checking Coding Agents Against a Record They Cannot Edit - Cheating, Deterrence and Disclosure on Impossible Tasks | [arXiv 2611.23335](https://arxiv.org/abs/2611.23335) | 6 | proposed |
| [0024](0024-passing-checks-do-not-establish-completion.md) | When Passing Checks Do Not Establish Completion: Evidence Interpretation in Coding Agents | [arXiv 2611.22835](https://arxiv.org/abs/2611.22835) | 6 | proposed |
| [0025](0025-unfaithful-work-reports.md) | Characterizing and Detecting Unfaithful Work Reports by Coding Agents | [arXiv 2611.25760](https://arxiv.org/abs/2611.25760) | 6 | proposed |
| [0026](0026-who-grades-the-agent.md) | Who Grades the Agent? Privilege Separation for Coding-Agent Benchmarks | [arXiv 2611.27860](https://arxiv.org/abs/2611.27860) | 6 | proposed |
| [0027](0027-transport-safe-evaluation.md) | Transport-Safe Evaluation of LLM Agents: Trajectory-Dependent Judge Error and Anytime-Valid Auditing | [arXiv 2611.30742](https://arxiv.org/abs/2611.30742) | 6 | proposed |
| [0028](0028-does-your-agent-need-to-know-its-budget.md) | Does Your Agent Need to Know Its Budget? A Placebo-Controlled Experiment on LLM Coding Agents | [arXiv 2611.21614](https://arxiv.org/abs/2611.21614) | 4 | proposed |
| [0029](0029-exercised-rarely-recovered.md) | Exercised, Rarely Recovered: What Public Reports Say About Recovery in 50 AI Agent Harnesses and Runtimes | [arXiv 2611.27090](https://arxiv.org/abs/2611.27090) | 7 | proposed |
| [0030](0030-markdown-is-not-a-firewall.md) | Markdown Is Not a Firewall: four coding agents, one rules file, four boundaries | [arXiv 2611.27112](https://arxiv.org/abs/2611.27112) | 6 | proposed |
| [0031](0031-does-the-agents-map-match-the-territory.md) | Does the Agent's Map Match the Territory? Architectural Claims in AI Agent Context Files and Their Fit with the Code | [arXiv 2611.29916](https://arxiv.org/abs/2611.29916) | 6 | proposed |
| [0032](0032-fixed-or-deleted.md) | Fixed or Deleted? External Feedback Lets Cheaper AI Coding Agents Repair Rule Violations | [arXiv 2611.29162](https://arxiv.org/abs/2611.29162) | 6 | proposed |
