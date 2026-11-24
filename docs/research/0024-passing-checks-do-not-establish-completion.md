# 0024 — When Passing Checks Do Not Establish Completion: Evidence Interpretation in Coding Agents

Source: https://arxiv.org/abs/2611.22835
Authors: Xueping Gao (Alibaba Cloud Computing)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

A passing check does not by itself show that a coding agent completed
the operation it was asked for. The paper asks whether the facts an
agent reports, the verdict it draws and the scope it claims all support
its completion claim, and frames one piece of evidence as a finite
tuple: the candidate source digest, the selected check, the observed
phase, the relevant operation and the recorded witness. Its method is
source-level intervention: revert one edit, substitute startup code,
reset observations at an operation boundary, add a consumer that
exercises the claimed behaviour. The author presents empirical case
analysis, not a method with a proven benefit.

Models are Qwen3-Coder-30B and Devstral Small 2 24B, with Qwen2.5 in
one study. Across 118 repair episodes on 31 tasks, a completion counts
only with an accepted edit, a finish action and PASS receipts bound to
the final source digest. In the receipt-supply study (16 checks, nine
task blocks, two models) all 122 delivered labels were correct, yet
only 95 of 128 outputs met the joint fact-and-scope contract: sixteen
misreported facts and eleven claimed more scope than the receipt
covered. In one cell of a 425-call study facts were right in 40 of 50
responses and verdicts in 32 of 50. Cases: a named assertion passed
through earlier write-side events while the patch had zero explicit
flush events; a warm measurement read 0 bytes where a cold first use
allocated 4; restoring one deleted return moved an outcome from UNKNOWN
to FAIL while the controls passed.

The negative results are stated plainly. Generated diagnostic notes did
not help repair (0 of 3 with notes against 1 of 3 withheld; later
operation notes 1 of 6, ordinary notes 2 of 6, withheld 0 of 6) and the
complete-note direction reversed under a fixed three-seed replication; notes
that accompanied a successful repair blamed code a replay found
correct; automatic receipt-conflict feedback reduced contradictions
without an overall gain; ordinary checks already caught all 12 failures
in one cohort. This entry found no confidence intervals in it; tasks were
selected after earlier local gains, and the archive is private for now.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Bind every passing receipt to the digest of the source it ran on, and count a completion only on receipts bound to the final source | implemented | decision 0038 and decision 0093; `scripts/delivered-by-brokkr.sh`: the ship anchor records the slice's per-file patch identity and the vouch holds only for that patch, and a carried panel verdict is admitted only when the head and tree recorded at both ends of its attempt equal the head now |
| 2 | Gate completion on a predicate over recorded facts, never on the agent's own claim | implemented | decision 0002 and decision 0007; `docs/security-model.md`: a seat never chooses the next phase, the pinned table rules on typed inputs that are engine-computed or seat-declared, and a verify gate stands between the implementer's result and review |
| 3 | Read a runner error before the target is entered as unknown, not as a failing assertion | alternative | decision 0006; `recipes/brokkr-fast/scripts/verify-seat.sh`: the engine keeps a determinate failure apart from an indeterminate end and parks the second, but only for an attempt as a whole; the verify gate's own results are pass and fail, so a suite that could not start reads as a failure returned to the smith |
| 4 | Check the scope and lifetime a witness covers: an observation establishes only the operation and phase it observed | alternative | decision 0071; `agents/charters/verify-reviewer.md`: whether the tests prove what the delivery claims is judged by a reviewer seat, with no mechanical scope contract on a receipt |
| 5 | Before trusting a pass, run a reversion control: remove the edit and watch the check change | alternative | decision 0071; `agents/charters/implementer.md` and `scripts/mutants.sh`: ruling 9 has the implementer show each new test binds by a compiling mutation that removes the behaviour, named in its notes, and the core mutants gate does it mechanically for the pure core only; no gate runs a delivery's own tests against the tree without the change (registry entry 0026, finding 5) |
| 6 | Keep an agent's explanation apart from its result: a repair that succeeds validates none of the diagnosis written beside it | implemented | decision 0007; `docs/security-model.md`: notes are prose the table never reads, only declared typed inputs reach a rule, and the result is treated as data to check |

## Candidates

Finding 3 is the gap. A verify script that cannot build its
environment, reach its registry cache or start its runner reports
`fail`, and the run returns to the implementer with a finding the
implementer cannot answer. Decision 0092 already gives a work seat the
word for this (`blocked_by: environment` parks for the operator); the
exec gate has no counterpart. A third verify result for "the check did
not run" would route the same way and spend no reforging.

Finding 1's tuple is wider than what Brokkr binds. The patch identity
covers the source digest; the check, the phase and the operation are in
the verify script's prose notes. Decision 0109 (accepted, unbuilt) puts
covered digests on a gate's verdict, and is the place to ask whether
the covered check belongs beside them.

The paper's null result on generated diagnostic notes bears on decision
0067's handoff note between visits: a note is cheap to require and this
is weak evidence that it helps the next visit. Worth measuring when
0067 is built rather than assuming.
