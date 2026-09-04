# 0003 — SWE Refactor Bench: Can Coding Agents Complete a Long-Horizon, Whole-Repository Stack Migration?

Source: https://arxiv.org/abs/2608.23564
Authors: Hong, Chi, Li, Wang, Gao, Yang, He, Zheng, Xiao, Na (Navers Lab, Einsia.AI, Tsinghua University)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

Behaviour-only evaluation is blind: an agent that touches nothing
scores perfectly on a regression suite. A whole-repository stack
migration (language, framework, platform or build toolchain) has to
prove two things at once, that the old stack is gone and that the
behaviour is intact, and current agents mostly fail one or the other.

The benchmark's acceptance protocol has three stages, all required in
order. A migration audit checks structurally that the target stack is
adopted and the old one removed, which defeats the do-nothing solution.
Fixed behavioural tests, 130,118 checks recorded from the original
systems, are replayed against the migrated build. Then agentic
verification: six independent coding agents, one hour each, write
differential tests after submission to hunt for drift the fixed suite
missed.

Twenty tasks over targets such as SQLite, zlib, libsodium and
GraphHopper, 520 runs. 28 runs pass all three stages, 5.4 percent; 13
of 20 tasks have no accepted solution. The best model scores 47 of 100.
Of 340 runs that did migrate, 58 percent reach 99 percent of the fixed
checks and only 26 percent reach 100. Thirty runs preserved behaviour
by skipping the migration; 252 migrated and broke behaviour.
Build-toolchain tasks average 31.4 of 100, language rewrites 5.6.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Pair a gate that proves the intended change happened with the gate that proves nothing broke, so a no-op or partial patch cannot pass on green tests | alternative | decision 0038: the vouch binds to the per-file patch map; `agents/charters/reviewer.md` judges whether the change does what its commits claim; no structural intent audit |
| 2 | Gate on 100 percent of the fixed suite and report the residual count; 99 percent is a fail | implemented | `scripts/coverage-exact.sh` refuses anything under literal 100 percent; a verify seat writes pass only after every fixed command exits zero |
| 3 | After submission, spawn independent time-boxed agents whose only job is to write differential tests against the patch, and treat their findings as gate evidence | alternative | `agents/review-adversarial.json` seats an adversarial judge in the engine council; decision 0041: gates read and never write, so it writes no tests |
| 4 | Record run outcomes as a three-way classification (skipped, changed-but-broken, accepted) rather than pass or fail | not-planned | |
| 5 | Stratify tallies by task class, since difficulty differs by six times between classes | alternative | decision 0041: triage rules a class per commission and the journal records it; tallies over classes are the ledger's, outside the engine (decision 0021) |

## Candidates

Finding 1 is the one the operator should weigh: a structural audit
bound to the commission's stated intent would close the no-op hole in
decision 0038. Finding 3 fits the boxed hands of decision 0043 as a
work-class seat that writes tests into a scratch tree and hands them to
the verifier.
