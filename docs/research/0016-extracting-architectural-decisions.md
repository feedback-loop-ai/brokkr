# 0016 — Can LLMs Extract Architectural Design Decisions from Source Code Commits? A Preliminary Exploratory Study

Source: https://arxiv.org/abs/2609.03721
Authors: Amey Karan, Rudra Dhar, Karthik Vaidhyanathan (International Institute of Information Technology Hyderabad); Mohamed Soliman (Paderborn University)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Architectural design decisions capture the rationale behind a
system's structure but are rarely documented; they hide in commits
that describe low-level implementation changes. This preliminary
study asks whether LLMs can recover them. Four models (Gemini 3 Pro,
DeepSeek R1, Kimi K2, Qwen3) run zero-shot and few-shot on 30
developer-written architectural design decisions from open-source
projects, scored with ROUGE-L, BLEU, METEOR and BERTScore, with one
author manually reviewing the Gemini outputs.

All models reach a BERT-F1 above 0.81, and few-shot prompting
improves alignment (Gemini BERT-F1 0.828 to 0.847). The surface
scores flatter: the generated decisions are too long,
implementation-focused, and miss the rationale behind the decision,
the part that makes a decision a decision. The authors see
opportunities for architecture-aware LLM systems and automated
architectural knowledge management.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Decision rationale is rarely recorded and hides implicitly in commits, where recovering it after the fact is unreliable | implemented | `docs/decisions/README.md`: semantic changes carry a numbered rationale with context, rulings and consequences, recorded when the decision is made; capability-spec links to source changes are a separate, still-unimplemented requirement of decision 0042 |
| 2 | Model-generated decision summaries miss the rationale even when surface-similarity scores are high, so acceptance is a read, not a score | implemented | decision 0044 and `docs/decisions/README.md`: research classification and decision acceptance belong to the operator; grammar tests check structure and citations, not the quality or truth of the rationale |
| 3 | Score decision extraction with ROUGE, BLEU, METEOR and BERTScore against a gold set | not-planned | |
| 4 | Use few-shot examples of the house's recorded decisions to improve alignment | not-planned | |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

Decision 0042 now makes the decision authoritative over its supporting
change documents. Its later addendum rules source-change links in promoted
capability specs, but [issue #231](https://github.com/feedback-loop-ai/brokkr/issues/231)
remains open for the archive instruction, backfill and validator. The
current dialect still invokes the framework's archive command without
that added provenance mechanism. The decision index's accepted status
records a ruling, not implementation. No extraction scorer or dedicated
few-shot extraction workflow has been added.

## Candidates

No new uptake proposed. The study supports recording rationale while
making the change and reviewing it directly. Existing decisions provide
that context; #231 already tracks the complementary source-change links.
A matching schema or a high similarity score would still not establish
that the rationale is sound.
