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
| 1 | Decision rationale is rarely recorded and hides implicitly in commits, where recovering it after the fact is unreliable | implemented | `docs/decisions/README.md`: Brokkr does not recover rationale; every semantic change carries a numbered decision with context, rulings and consequences, written at change time |
| 2 | Model-generated decision summaries miss the rationale even when surface-similarity scores are high, so acceptance is a read, not a score | implemented | decision 0044 and `docs/decisions/README.md`: classification and acceptance are the operator's read of the argument; entries and decisions stay proposed until ruled |
| 3 | Score decision extraction with ROUGE, BLEU, METEOR and BERTScore against a gold set | not-planned | |
| 4 | Use few-shot examples of the house's recorded decisions to improve alignment | not-planned | |

## Candidates

The paper's negative result is Brokkr's positive argument: precisely
because extraction misses rationale, the product forces rationale to
be written at change time. No uptake; the entry stands as a citation
for the constitution's design.
