# 0030 — Markdown Is Not a Firewall: four coding agents, one rules file, four boundaries

Source: https://arxiv.org/abs/2611.27112
Authors: Sandeep Dhuri (Independent Researcher)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Is a one-page rules file in a repository a control on a coding agent,
or advice? The author wrote one 278-word rules file, installed under
each harness's own name for it, and a 280-word neutral file as a
control, and ran four agents with vendor permission prompts off:
Claude Code 2.1.307, Codex CLI 0.163.0, Gemini CLI 0.65.0 and OpenCode
1.19.3 with DeepSeek. Twenty small Flask and SQLite repositories carry
hidden tests written before any run; three arms (no file, rules file,
neutral file) and four planted boundaries, 15 runs per arm, with a
filtering gate in front of a mock service.

The file did not make code safer and cost one agent a third of its
score. Pooled hidden-test pass rate at the final step is 98.2 percent
with no file, 90.9 with the rules file and 97.0 with the neutral file;
the paired drop is 7.7 points (clustered interval −13.4 to −2.6), and
−1.3 without Codex CLI, which fell from 99.0 to 67.7 percent under the
rules file and scored 100 under the neutral one, because it read one
ambiguous idempotency rule the other way from the other three agents and
the reference solutions. In 11 of 36 rules-file steps Codex CLI
reported completion with all of its own tests passing while fewer than
half the hidden tests passed. The checkers found no real bug of the
checked kinds in any arm (31 findings, all false alarms). At the
boundary, the gate blocked all 11 requests it saw; where the neutral
arm ran ungated, 4 of 60 runs crossed. A credential found in the
repository reached the service in 5 of 15 no-file runs, 0 of 15 with
the rules file and 4 of 15 with the neutral file, all nine of them
OpenCode's (5 of its 5 no-file runs); every run carried four
vendor API keys and one printed them. The Codex model pin was never
applied, and only the session files showed that gpt-6.1-sol was served.

Limits the author states: a run-script error put the gate in front of
the no-file and rules-file arms, so their ungated crossing rates were
never measured; 15 runs per arm, not the registered 20, left four
hypotheses without a verdict; one adjudicator; toy repositories; Gemini
CLI lost 84 chains to exhausted credit. The paper's own closing line:
a file tells an agent what you want, a gate decides what it gets.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Enforce a boundary in a gate the runtime owns, never in an instructions file | implemented | decision 0043 and decision 0046; `docs/security-model.md`: where a realm declares boxed hands a seat's commands run in the box its boundary builds, a boundary the machine cannot build refuses rather than degrading, every effect records which boundary stood, and the security model says in terms that a prompt is not a control |
| 2 | Score the outcome from tests the agent cannot reach, and read its final report as a claim: an agent's own passing suite is not the task's contract | alternative | decision 0041; `recipes/brokkr-fast/scripts/verify-seat.sh` and `agents/charters/verify-reviewer.md`: a verify gate and then judging seats stand between the implementer's result and ship, and none of them reads the report as evidence; the suite verify runs is the branch's own, with no test held outside the implementer's reach |
| 3 | Test a rules file for rules that conflict with each other, and measure it against a same-length neutral file before trusting it to help | not-planned | |
| 4 | Pin the model, and record the served model per run from the harness's own output rather than from the configuration | implemented | decision 0031 and decision 0075: every model seat is compile-refused unless pinned, and drivers record the provider-reported served model, never a configured guess |
| 5 | Keep credentials the task does not need out of the agent's environment | implemented | decision 0012 and decision 0043: secrets are bound by name and resolved only by the runner, and a boxed seat's commands run in an empty root with no credential and no network; the harness process still holds its own provider credential until decision 0072 is built (#644) |
| 6 | Record which arm ran behind which gate, and check that wiring against the configuration before reading a result | implemented | decision 0046 and decision 0065: every effect and every readout says which boundary stood, the grant is in the manifest, and every comparison says what each side held |

## Candidates

Finding 3 is aimed at exactly what Brokkr leans on: house rules and
charters are rules files. Brokkr ratchets their size
(`quality/prompt-bytes.json`) and measures nothing of their effect. The
paper's result is not that rules files fail but that one ambiguous
rule cost one harness 31 points while three others moved
under 2.5, and that only the neutral-file arm showed it. Two things
follow for the operator to weigh: a conflict read of the house rules
whenever a ruling is added (decision 0071's eleven rulings and each
charter are read together by every seat), and, with entry 0028 finding
3, a neutral arm in any wager that claims a charter change helped.

Finding 2 repeats entry 0023 finding 1 and entry 0026 finding 5 from a
third source: 11 of 36 steps reported complete over a passing suite of
the agent's own making while the held-out contract failed. Three
papers in one week measure the same gap, and Brokkr's verify gate has
it.

Finding 5's caveat is the paper's own observation turned on Brokkr: an
agent that finds a token uses it. A harness process holds its provider
credential beside the model it runs, and decision 0072 is where that
closes.
