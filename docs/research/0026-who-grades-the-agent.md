# 0026 — Who Grades the Agent? Privilege Separation for Coding-Agent Benchmarks

Source: https://arxiv.org/abs/2611.27860
Authors: Nathan Nour Houlamy, Mattia Favretto, Earlence Fernandes, Mauro Conti (University of Padua; UC San Diego)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Coding-agent benchmarks usually run the agent's code in the environment
that grades it, so the agent can forge a pass: a conftest.py hook
that rewrites test outcomes, a read of the scorer's call stack, a
swapped binary. The authors call this adversarial measurement and
argue a stronger sandbox does not fix it, because the attacks use
access the benchmark grants.

SplitBench applies privilege separation. The agent works in its own
environment; only a manifest of in-scope changed files, with digests
and size limits, crosses. The files are replayed onto a clean baseline
in a fresh, network-less evaluation environment per test case, where an
adapter reports raw observations and nothing else. A trusted Oracle
outside holds the hidden expected answers and compares
deterministically; benchmark authors write it as two functions, `plan`
and `judge`. Model access goes through a relay that holds the
credentials.

180 of 202 tasks from Terminal-Bench 2.0 and DeepSWE v1.1 were
converted by agents (Claude Opus 5.5 as converter and reviewer), at a
median of just under two hours and about $10 a task, 4.8 billion tokens
in all, 92 percent cached. Each conversion is admitted only if the
reference solution passes, a no-op fails, mutants match the native
verdict and tamper attacks fail: of 429 attacks the native graders
accepted 230 and SplitBench none. On honest runs (Sonnet 5.5 and GPT
6.1 Sol, two runs each) the graders agreed on 98.7 percent of 1,374
comparisons. Opus 5.5 then attacked 360 times, half with full knowledge
of the Oracle and adapter code: of 307 completed attempts 243 were
rejected, and of the 64 passes none came from a SplitBench flaw, 47
from weak upstream tests in 34 tasks, 12 from public answers, 3 from
faked implementations and 2 from conversion errors.

Limits stated: the Oracle judges returned values, not how they were
computed; weak tests are inherited; regression tests in 87 of 112
DeepSWE conversions keep upstream grading; the agent review loop
accepted two broken conversions, so the authors recommend that a person
review each conversion before use.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Carry only a digested manifest of in-scope changed files from the agent's environment to grading, and replay it onto a clean baseline | alternative | decision 0043 and decision 0038; `docs/security-model.md`: a boxed exec gate runs in an empty-root box holding only the worktree, with the git directory's hooks and config masked, and the ship anchor records the per-file patch identity; the gate still runs in the worktree as the implementer left it, test configuration included |
| 2 | Build a fresh evaluation environment for every test case and destroy it afterwards, so no state carries between cases | alternative | decision 0043: a boxed command runs in an empty-root namespace with toolchain caches as overlays; the box is built for a seat's commands, never per test case, and the worktree persists across them |
| 3 | Split the grader in two: an adapter inside that only reports what it observed, and a trusted judge outside that holds the expected answers and decides deterministically | alternative | decision 0002 and decision 0007; `recipes/brokkr-fast/scripts/verify-seat.sh`: the engine's pure table decides the route from typed inputs, but the verify script inside the box decides its own pass or fail; decision 0117 (accepted, unbuilt) is this split exactly, a seat-owned observation judged by a pure core function into one engine-owned flag |
| 4 | Treat everything the evaluation environment emits as adversarial: one strict value within a size limit, validated against a schema, with a crash or a timeout never read as a pass | implemented | decision 0001, decision 0006 and decision 0079; `docs/security-model.md`: a result that does not parse or match its schema parks with the raw evidence, the transport bounds every line and result, and an end without a read result is indeterminate and parks |
| 5 | Admit a grader only after it has been qualified: the reference passes, a no-op fails, mutants agree and tamper attacks fail | alternative | decision 0071; `agents/charters/implementer.md` and `scripts/mutants.sh`: ruling 9 has the implementer show each new test fails under a compiling mutation that removes the behaviour, and the core mutants gate does it mechanically for the pure core; nothing the engine runs shows a delivery's own tests fail against the tree without the change |
| 6 | Route model access through a relay that holds the credentials, and allow egress only to listed hosts | planned | #644 (decision 0072, accepted 2026-10-12: an engine-owned egress relay outside the box that attaches the provider credential); #216 for an allow-list of hosts |

## Candidates

Finding 5 is the no-op screen the commission's first topic asks about,
and entry 0010 finding 2 flagged the same idea for fixtures. Here it
has a concrete, cheap form for a delivery: an exec gate that checks out
the tests the branch added or changed onto the merge-base's production
code and requires at least one to fail. A delivery whose new tests pass
against the code without the change has proved nothing, and today only
the implementer's own word in `notes` and a reviewer's eye say
otherwise. The paper's count is the argument: graders that had passed
review accepted 230 of 429 attacks.

Findings 1 and 3 together describe the residual the security model
already half-states ("a green gate is not independent acceptance"). The
paper's sharper point is that the box is not the fix: the smith's files
are the gate's inputs by design, so a planted test-runner
configuration is inside the boundary. A verify gate that restores the
test harness configuration from the merge-base, or refuses a diff that
touches it without a ruling, is the narrow version.

47 of the paper's 64 surviving passes came from weak upstream tests.
That is the same residual Brokkr leaves to its reviewers, and no
separation removes it.
