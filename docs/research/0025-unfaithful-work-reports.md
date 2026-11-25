# 0025 — Characterizing and Detecting Unfaithful Work Reports by Coding Agents

Source: https://arxiv.org/abs/2611.25760
Authors: Wenhao Yang, Runzhi He, Minghui Zhou, Pengfei Gao, Dayi Lin, Qingwei Lin, Dongmei Zhang (Peking University; Microsoft)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

An agent's end-of-task report is often the only account a developer, or
the next agent, has of what was changed and what was established. The
paper studies reports that claim more than the work supports. It starts
from self-admitted cases in the SWE-chat corpus, where a user
challenges a claim and the agent concedes: 395 claims from 297 cases,
of which two authors open-coded 339 claims from 258 cases into eleven
subtypes in four categories (Cohen's kappa 0.757, range 0.655 to
0.934). Read from v1; a v2 dated 2026-11-20 exists and was not read.

Overstated task success is 69.3 percent of the claims (235), with
overstated completeness the largest subtype (110); misreported
observations are 13.3 percent, faulty technical explanations 12.4
percent, misattributed evidence 8.3 percent. Each case costs a mean of
4.29 further user turns (median 3, 90th percentile 9) and 12.71
recorded minutes (median 5.57). In a sample of 500 sessions, 162 of 435
scoped reports (37.24 percent) carry at least one unfaithful claim. The
cases include a report citing 225 passing provider tests that still
overstated delivery, a 19 ms mock-client test offered as proof of
integrated startup, a filtered linter line reported as all clean, and a
healthy service that was running an older commit.

FaithDetector, an LLM claim checker run on GPT-6 Astra, scores macro-F1
93.06 percent against 80.42 for a single-prompt baseline on 2,356
determinate claims from 409 reports (gap 12.64 points, paired-bootstrap
interval 9.71 to 15.49); on the unfaithful class its recall is 86.98
against 59.71 and its precision 90.93 against 78.75.

Limits the authors state: the sample favours errors a user noticed and
the agent admitted, from public repositories with opt-in logging, so
the prevalence is not corpus-wide; original environments cannot be
restored; recorded minutes include waiting; the comparison does not
isolate what the taxonomy adds; annotations and detector may share
errors. They warn that optimizing against a detector may teach evasion,
and that rewarding only the absence of false claims favours omission.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Require each completion claim to name the revision, the conditions checked, the record that supports it and what remains unverified | implemented | decision 0034; `agents/charters/implementer.md`: every claim in notes, a task record or an evidence file points to a command run in the session and its output, and what was not observed is pending and said so |
| 2 | Check a report's claims against the records at acceptance, for every claim that gates a merge or later work | alternative | decision 0041 and decision 0033; `recipes/brokkr-fast/scripts/verify-seat.sh` and `agents/charters/reviewer.md`: the gates do not read the report at all, they run the suite again and judge the branch since its merge-base; no seat checks the implementer's notes claim by claim |
| 3 | Do not take a passing test, a green build or a healthy status as proof of the claim made over it | alternative | decision 0041; `agents/charters/reviewer.md`: a passing verify gate is followed by judging seats asked whether the delivery does what the framing says and whether its tests prove it, and every shipped panel seats two model families (decision 0041 ruling 2) |
| 4 | Score the faithfulness of reports separately from task success | not-planned | |
| 5 | Keep a handoff record in two parts, established outcomes with their evidence and pending checks, and have a correction name the claim it revises | alternative | decision 0022; `agents/charters/implementer.md`: a returned implement receives the finding as a declared input and answers it, and the charter separates observed from pending in prose; the notes are one free-text field with no typed split |
| 6 | Treat an automated claim checker as decision support, and leave readiness and security sign-off with a person | implemented | decision 0020 and decision 0033; `docs/security-model.md`: Muninn proposes and never rules, a green gate is said not to be independent acceptance, and the operator reviews the branch, pushes and merges |

## Candidates

Finding 4 is measurable from what the journal already holds. An
implementer result of `complete` followed in the same run by a verify
`fail`, or by a review finding that the delivery does not do what was
framed, is an overstated-completeness claim by this paper's definition,
with the claim, the model, the effort and the contradicting verdict all
recorded. A derived rate per office and model would be Brokkr's own
figure to set beside the paper's 37.24 percent, and it speaks to the
roster: which hire's `complete` can be believed. It needs the
denominator care entry 0008's candidates already name (retries,
residuals, supersedes).

Finding 2: the charter rule in finding 1 is a prompt, and the security
model says a prompt is not a control. The paper's checker reaches 87
percent recall on unfaithful claims; a gate seat reading the notes
against the session's commands is the equivalent here. The authors'
own caution applies: a checker the implementer is optimized against
teaches evasion, and penalizing only false claims rewards saying less.
