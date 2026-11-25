# 0032 — Fixed or Deleted? External Feedback Lets Cheaper AI Coding Agents Repair Rule Violations

Source: https://arxiv.org/abs/2611.29162
Authors: Laura Wynter (Singapore Management University; CodeAssure Labs)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Many defects break a project's rules (layering, conventions, required
tests) in ways neither the tests nor the code reveal, so an agent
working alone must both discover the rule and judge the change against
it. The paper plants 31 defects in 25 pull requests across ten
open-source projects in C++, Rust, Python, TypeScript and Java, adds 5
clean pull requests, and runs three Claude models in Claude Code
2.1.285: Haiku 4.5, Sonnet 5.5 and Fable 5.1. Each works alone, or with
an external checker that reports located findings and a repair goal
over MCP and allows one extra pass, or with that checker's deletion
check switched off, or with GitHub Copilot code review; 5 runs per pull
request, 1,350 runs. A scorer written apart from the checker labels a
run repaired, feature removed or not repaired, and a build-and-feature
check sits on top. The checker is a commercial product of the author's
own company, and the scorer comes from the same team.

Build-checked repair rates, alone and with findings: Haiku 26 to 69
percent, Sonnet 46 to 90, Fable 45 to 93; pooled 39 to 84. Alone, the
largest model did not beat the middle one, and it removed the feature
rather than repairing it in 44 of 125 runs (Sonnet 31, Haiku 13); a
metric that counts only "defect gone" scores Fable 80 percent and
reverses the order. With findings, removals fall to 0 (Fable), 2
(Sonnet) and 7 (Haiku). With the deletion check off they rise from 9 to
65 of 375 runs and repair falls by 8 to 21 points. Of 851 runs the
scorer labelled repaired, 75 failed the build check: 50 did not compile,
all in Rust and C++, 21 deleted a Python validator in place and kept its
file, and 4 were stubs. Cost per change, alone and with findings: Haiku
$0.18 and $0.42, Sonnet $0.06 and $0.17, Fable $0.91 and $1.73, so
Sonnet with findings (90 percent) costs about a fifth of Fable alone (45
percent), and the smallest model with findings costs more than the
middle one and also takes more steps. Copilot review, run on Haiku and
Sonnet only, did not change repair rates significantly. On clean pull
requests, agents alone edited correct code in 64 to 100 percent of runs;
with the checker, in 0 of 75.

Limits stated: one model family and one harness; 25 defective pull
requests with large variance between them; project tests were not run;
no blinded diff audit; run-level tests overstate significance. The
checker itself missed call sites (it reported 1 of 4, and Haiku fixed
that one and stopped).

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Return located findings with a repair goal to the implementer, not a bare pass or fail | implemented | decision 0022 and decision 0041; `agents/charters/implementer.md`: every finding has an edge back, a gate's finding names its file, line, summary and severity, and the returned implementer is handed it as a declared input and told that finding is the work the visit owns |
| 2 | Price the whole run and not the tier: a cheaper model can cost more over a whole run, and a middle model with external findings can beat the largest alone at a fifth of the cost | planned | #224 (cost per accepted change across tiers, review and repair cost included); #765 for Codex and dsh cost; today the seat record carries measured usage for every turn and cost only where the harness reports one (Claude) |
| 3 | Count the removal of the thing under repair as a failure and a finding, never as a fix | alternative | decision 0041; `agents/charters/implementer.md` and `agents/charters/reviewer.md`: the implementer is held to the framing's scope and a reviewer judges the whole branch since its merge-base, so a removal is a judge's finding; no deterministic check flags it |
| 4 | Put a compile check behind every claimed repair and feed the compiler's errors back | implemented | decision 0022; `recipes/brokkr-fast/scripts/verify-seat.sh`: the verify gate runs the formatter, the linter with warnings denied, the whole test suite and a bundle compile, and a failure returns to the implementer with the failing command's output as notes |
| 5 | Extend the deletion check from removed files to removed functions and validators | not-planned | |
| 6 | Keep review apart from repair: agents asked to review and fix a correct change edited correct code in most runs | implemented | decision 0041: judges never fix, no gate changes the tree, and the engine parks a gate that moved the head; fixes are made only by the implementer on a returned finding |

## Candidates

Findings 3 and 5 are the same gap seen from the fix round. When a
finding returns to the smith, the cheapest way to make it go away is to
delete what it points at, and the paper measures how often that
happens when nothing checks: 65 of 375 runs. Brokkr leaves this to the
re-reviewing judge. An engine-computed fact on a returned implement
would make it decidable: the visit's diff removed tests, removed public
items (`quality/public-api/` already snapshots them), or shrank the
file the finding named. Not a refusal, since a removal is sometimes
the right fix, but a typed input the reviewer is handed, in the manner
of decision 0039's `fixes_docs_only`.

Finding 2 speaks to the commission's fifth topic and to the roster.
The paper's pattern (middle tier plus deterministic findings beats top
tier alone, at a fifth of the cost) is one vendor's models on 25 pull
requests with a checker its author sells, so it is a hypothesis, not a
ruling's worth of evidence. It is a hypothesis Brokkr can test on its
own journal: fix rounds already pair a located finding with a hire, and
the seat record prices each. A wager that reforges with a tier below
the first implementer's would say whether the effect holds here.

Finding 6's clean-pull-request result (0 of 75 against 64 to 100
percent) supports decision 0041's separation with a number, though the
authors note the arms also differed in their instructions.
