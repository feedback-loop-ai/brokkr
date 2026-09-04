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

## Index

| # | Article | Source | Findings | Status |
|---|---|---|---|---|
| [0001](0001-what-survives-the-next-model.md) | What Survives the Next Model? Benchmarking LLM-Based Techniques Against Single-Prompts | [arXiv 2609.00468](https://arxiv.org/abs/2609.00468) | 5 | proposed |
| [0002](0002-github-evaluate-llms-before-production.md) | How to evaluate LLMs before production | [github.blog](https://github.blog/ai-and-ml/llms/how-to-evaluate-llms-before-production/) | 7 | proposed |
| [0003](0003-swe-refactor-bench.md) | SWE Refactor Bench: Can Coding Agents Complete a Long-Horizon, Whole-Repository Stack Migration? | [arXiv 2608.23564](https://arxiv.org/abs/2608.23564) | 5 | proposed |
| [0004](0004-context-privilege-escalation.md) | What's in Your Agent's Context? Context Privilege Escalation Attacks against AI Agent Harness | [arXiv 2609.01222](https://arxiv.org/abs/2609.01222) | 6 | proposed |
| [0005](0005-model-based-agentic-software-engineering.md) | Model-Based Agentic Software Engineering (MAGE) | [arXiv 2608.25174](https://arxiv.org/abs/2608.25174) | 6 | proposed |
| [0006](0006-harness-of-harness.md) | Harness-of-Harness: Multi-Day Autonomous Software Development with Continual Improvement | [arXiv 2609.01481](https://arxiv.org/abs/2609.01481) | 6 | proposed |
| [0007](0007-loop-engineering.md) | Loop Engineering: Building Blocks, Adoption, and Impact | [arXiv 2608.21884](https://arxiv.org/abs/2608.21884) | 6 | proposed |
| [0008](0008-swe-gate.md) | SWE-Gate: Passing Functional Tests Is Not Enough for Software Engineering Agents | [arXiv 2609.04167](https://arxiv.org/abs/2609.04167) | 4 | proposed |
| [0009](0009-illusion-of-independent-quorums.md) | The Illusion of Independent Quorums: Epistemic Fault Domains and Correlated Cognitive Failures in Agentic Quorums | [arXiv 2609.02925](https://arxiv.org/abs/2609.02925) | 5 | proposed |
| [0010](0010-toolgate.md) | ToolGate: An Executable Acceptance Pipeline for Tool-Dependent Scientific Benchmark Construction | [arXiv 2609.02067](https://arxiv.org/abs/2609.02067) | 5 | proposed |
| [0011](0011-trajectory-aware-evaluation.md) | Efficient SWE Agent Benchmarking via Trajectory-Aware Evaluation | [arXiv 2609.01603](https://arxiv.org/abs/2609.01603) | 4 | proposed |
| [0012](0012-harnessdev.md) | HarnessDev: Can LLMs Create and Evolve Their Own Agent Harness? | [arXiv 2609.01437](https://arxiv.org/abs/2609.01437) | 5 | proposed |
| [0013](0013-heart-tool-primitives.md) | Harness Engineering in LLM Tool Use via Agent-Native Reusable Tool Primitives | [arXiv 2609.01736](https://arxiv.org/abs/2609.01736) | 5 | proposed |
| [0014](0014-progressive-risk-vesting.md) | Spawn Freely, Act Sparingly: Progressive Risk Vesting for Recursive LLM-Agent Trees | [arXiv 2609.01035](https://arxiv.org/abs/2609.01035) | 5 | proposed |
| [0015](0015-black-box-agentic-processes.md) | A Black Box for Agentic Processes: Blockchain-Anchored Evidence for AI Agent Communication, Human Oversight, and GRC Audits | [arXiv 2609.04017](https://arxiv.org/abs/2609.04017) | 5 | proposed |
| [0016](0016-extracting-architectural-decisions.md) | Can LLMs Extract Architectural Design Decisions from Source Code Commits? A Preliminary Exploratory Study | [arXiv 2609.03721](https://arxiv.org/abs/2609.03721) | 4 | proposed |
| [0017](0017-scx-router.md) | SCX Router: Streaming Zero-Shot Model Selection with a Decoder-KV Classifier and a Real-World Task Ontology | [arXiv 2609.02292](https://arxiv.org/abs/2609.02292) | 4 | proposed |
