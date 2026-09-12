# 0018 — An Organizational Second Brain: Building an AI That Learns From Experts

Source: https://engineering.fb.com/2026/09/02/ml-applications/organizational-second-brain-ai-learns-from-experts/
Authors: Shaurya Sengar, Jason Nawrocki, Jay Shah, Prashant Kommireddi (Meta)
Read: 2026-09-13
Status: proposed
Intake: research intake, run research-intake-of-the-articles--386e1dfb, 2026-09-13

## Summary

Meta's engineering post describes an internal agent that stands in for
a domain expert in assessment work (the named domains are regulatory
compliance, financial risk, security review, engineering-standards
compliance and procurement evaluation). The design has four layers: a
knowledge system, a reasoning layer, an evaluation framework and an
improvement loop. The stated principle is that the whole system's
complexity lives "in text files that are readable by both humans and
agents, rather than fine-tuned model weights", so that every
improvement is "a text edit that a domain expert can review in 30
seconds", version-controlled, diffable and reversible.

The knowledge system is a filesystem partitioned by information
density and expected usage frequency. Dense, frequently read material
is curated into a wiki of typed files: position files (authoritative
organisational stances), taxonomy and vocabulary files (a glossary),
routing indexes (input to procedure) and gateway files (threshold
tests that run before analysis begins). Sparse, situational material
(detailed references, historical records) stays behind semantic and
lexical retrieval. Every file declares its dependencies and consumers
in YAML frontmatter, which yields a bidirectional dependency graph.

The reasoning layer is a set of imperative procedures the authors call
recipes. A recipe references knowledge files but carries no domain
facts; a knowledge file states a position but prescribes no procedure.
Recipes compose into pipelines with progressive disclosure, so a stage
receives only the instructions and knowledge it needs; the authors
report this cut tokens per turn by around 80%. Two human-oversight
mechanisms are built in: checkpoints, where the agent surfaces
intermediate reasoning for expert review, and escalations, where
genuine ambiguity hands the question to an expert.

The improvement loop treats maintenance as a compilation problem in
four phases. Diagnosis separates extraction from classification with
one attribution test, "could the agent have reached the correct
conclusion from its source materials?": yes and it erred is a recipe
defect, no is a knowledge gap, and expert disagreement is flagged
ambiguity. Compilation has sub-agents turn a diagnosis into minimal
edits after an impact analysis over cross-references, conflicts,
token budgets, test coverage and duplication, guarded by an
independent adversarial review of the diff by an agent with no
context and by a deterministic linter that refuses contradictions,
dangling references and dependency cycles. Evaluation is a targeted
replay of the original scenario, judged by a separate judge that does
not know what changed, followed by parallel regression benchmarks with
independent LLM judges; a regression sends the change back to
compilation. Landing opens a pull request with an audit trail, and the
failing scenario is added to the regression suite automatically, so
"every fix permanently raises the bar".

Reported results after six weeks of development: experts rated the
output useful "almost all the time", assessment time fell from days to
minutes, validated knowledge edits arrived at a rate that "previously
required full engineering sprints", and there were zero regressions
across improvement cycles. No benchmark, sample size or baseline is
given; the numbers are the vendor's own report of an internal
deployment. Meta lists the same four preconditions for reuse: a
structured knowledge system with explicit boundaries and a dependency
graph, a procedural layer separating knowledge from method, an
evaluation suite that grows with each cycle, and human checkpoints
calibrated to the domain's risk.

Two of the article's practices have no counterpart here and are read
as not taken up: partitioning knowledge by density into a curated wiki
plus a retrieval index (Brokkr has no retrieval layer; a seat reads
the files its commission and charter name), and the blind targeted
replay of a failing scenario as the gate on a fix (Brokkr's gates judge
the head and the delta, never a replay of the run that failed).

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep what the agent knows and how it reasons in separate files, so a procedure carries no domain facts and a knowledge file prescribes no procedure | implemented | decision 0041; `agents/charters/` holds each office's method, `docs/house-rules.md` holds the realm's facts, and the engine renders the two into one contract per seat |
| 2 | Compose procedures into a pipeline with progressive disclosure, so each stage receives only the instructions and knowledge it needs (an 80% token cut per turn) | alternative | decision 0041 and decision 0007; `recipes/fast`: one office per seat, each phase hires its own charter, and the seat sees only declared or engine-computed inputs; no token budget per stage is measured or bounded |
| 3 | Declare every file's dependencies and consumers in frontmatter, and lint the graph for dangling references and cycles | alternative | decision 0038 and decision 0057; `crates/brokkr-cli/tests/research_registry.rs`: the decision index is derived and union-merged, a citation that does not resolve fails the test, and a realm crossing is pinned by its bytes; there is no declared consumer graph over the tree's documents |
| 4 | Run a gateway threshold test before analysis begins, so out-of-scope or mis-sized input is turned away first | alternative | decision 0041 and decision 0051; `recipes/triage`: triage is a chief-grade gate ruling a closed strategy class before work, and landing enters at a classify gate; the test is judged, not a deterministic file |
| 5 | Build checkpoints where the agent surfaces intermediate reasoning for expert review, and escalations that hand genuine ambiguity to an expert | implemented | decision 0001, decision 0006 and decision 0021: an unmatched result parks with the raw evidence, an indeterminate outcome always parks, and the engine parks rather than substitutes; every seat has an escalate result and the operator rules the park |
| 6 | Diagnose expert feedback with one attribution test (could the agent have reached the right answer from its sources?) to sort recipe defects from knowledge gaps from ambiguity | alternative | decision 0022 and decision 0041: every finding has an edge, a wrong spec returns to design, oversized work to triage, a security finding to implement; the routing is by the finding's kind, not by a test of whether the sources held the answer |
| 7 | Guard a compiled change with an independent adversarial review of the diff and a deterministic structural validator before any judge reads | implemented | decision 0042 and decision 0045; `agents/charters/review-adversarial.md`: the adversarial member reviews to break, a panel hires across the vendor line, and each judged loop runs its deterministic half before the judge |
| 8 | Gate a fix on a targeted replay of the failing scenario, judged blind to the change, then parallel regression benchmarks with independent judges that send a regression back to compilation | not-planned | |
| 9 | Add every landed fix's failing scenario to the regression suite automatically, so the suite only grows and the bar only rises | alternative | `docs/house-rules.md` and `fixtures/`: tests are part of every change and the evaluator corpus is frozen and only ever versioned, so the suite grows by rule; a failed run's scenario is not added by the machine |
| 10 | Land every improvement as a reviewable text edit in a pull request with an audit trail, never as a change to model weights | implemented | decision 0010, decision 0016 and decision 0033: recipes and agents are data files, decisions are numbered text, and every pull request names a run whose journal vouches for the head |
| 11 | Partition knowledge by information density and usage frequency: a curated wiki for the dense, retrieval for the sparse | not-planned | |
| 12 | Treat maintenance as compilation: sub-agents turn expert corrections into minimal verified edits after an impact analysis | alternative | decision 0005 and decision 0020: the product delivers its own changes through its runs, and Muninn reads the journal and proposes with provenance; no seat compiles a correction into a charter or recipe edit |

## Candidates

Finding 8, the blind replay, is worth the operator's attention. Brokkr
has 1,069 typed gate decisions in its journal and no outcome edge
that says whether the gate was right, which is the question issue
#271 opens. Meta's loop closes that edge cheaply: the scenario that
failed becomes the test, a judge that does not know what changed
scores the replay, and the regression suite is what the journal has
already recorded. A ruling would need to say what a "scenario" is in
Brokkr's terms (a run's declared inputs and head, replayed under the
same pinned bundle) and which judge may score it; the wagers issue
#237 already proposes the comparison harness that would carry it.

Finding 12 is the article's real claim and the one closest to the
self-forge loop: not that a model improves, but that the text it runs
on improves, one reviewed diff at a time, from the findings its own
gates produce. Brokkr routes a finding back to a phase (decision 0041)
and Muninn proposes (decision 0020), but nothing turns a park or a
residual into a proposed charter or recipe edit with the finding as
provenance. The attribution test in finding 6 is the small piece that
would make such a seat honest: was the charter wrong, or was the
house rule missing? Left `not-planned` and `alternative` as read; the
operator decides whether either becomes an issue.
