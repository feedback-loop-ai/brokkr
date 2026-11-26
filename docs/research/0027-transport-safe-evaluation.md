# 0027 — Transport-Safe Evaluation of LLM Agents: Trajectory-Dependent Judge Error and Anytime-Valid Auditing

Source: https://arxiv.org/abs/2611.30742
Authors: Yulliwas Ameur, Samia Bouzefrane, Soumya Banerjee (Efrei Research Lab, Université Paris-Panthéon-Assas; CEDRIC, Conservatoire National des Arts et Métiers)
Read: 2026-11-24
Status: ruled (2026-11-25)
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Agents are often ranked by an LLM judge whose sensitivity and
specificity are measured once on a hand-labelled sample and then reused
to correct every agent's judged success rate. The paper shows that this
correction can be biased when the judge's errors depend on the
trajectory it reads (its length, task family, number of tool calls) and
agents differ in the trajectories they produce, even when the judge
never learns which agent it grades; it is unbiased only when an agent's
shift in trajectories leaves the judge's average sensitivity and
specificity unchanged. It derives the exact limit of the bias and the
condition under which it reverses a ranking, and notes that more judge
compute is no generic repair: a majority of repeated calls pushes each
trajectory-level accuracy further from one half, in whichever direction
it already leaned. A prior analysis of public web-agent data it cites,
not rerun, has three judges' sensitivity falling from 0.840, 0.884 and
0.876 on the shortest third of trajectories to 0.182, 0.364 and 0.477 on
the longest (or capped) third.

On a four-agent semi-synthetic benchmark over 250 replications, the
scalar correction with oracle calibration has maximum bias 0.245, mean
pointwise coverage 0.016 against a nominal 0.95, and recovers the full
ranking in 0 of 250. Augmented inverse-probability weighting over a
randomized human audit keeps maximum bias below 0.003 with coverage
0.939 (uniform audit) and 0.950 (design-aware). Stratifying richly is
data-hungry: over 48 cells, calibration support covers 0.209 of the
target at 600 labels and 0.805 at 4,800, and a per-cell correction has
mean absolute error 0.188 falling to 0.077, where a 12 percent random
audit stays at 0.018 to 0.020. Three judges identify the truth without
labels under conditional independence, and where it fails the paper
reports a bounded set given an externally justified bound, and otherwise
only the independence point and a sensitivity curve; the cited data show
dependence of 0.41 to 0.70, including two judges that share a backbone.
Repeated inspection breaks fixed-time intervals (joint coverage 0.535
over 200 streams) where an e-process confidence sequence holds 0.995,
for one frozen agent. Treating 10 tasks from each of 40 repositories as
independent gives a false-split rate of 0.188; clustering by repository
gives 0.035.

Limits stated: the main results are semi-synthetic, the public-data
analyses are not rerun, and no causal effect of length is claimed. The
paper is the authors' accepted version for ICAART 2027 and discloses
that an LLM drafted its text and proof sketches. Read from the PDF.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Do not carry a judge's error rates, measured once, to every agent it grades: they can depend on the trajectories each agent produces | not-planned |  |
| 2 | Audit a random sample of the judge's verdicts by a person, drawn with known probabilities fixed before the audit, and estimate from judge and audit together | alternative | decision 0033; `docs/security-model.md` and `docs/guides/bug-provenance.md`: the operator reviews every branch a run ships before merging it, and a later defect is traced by hand to the run that introduced it; neither is recorded as a label against the gate's verdict, and no estimate is drawn |
| 3 | Where calibration does not cover an agent's trajectories, report a set or a bound and certify only the orderings that hold for every value the bound allows | not-planned | |
| 4 | Do not count judges that share a backbone as independent evidence, and do not expect more calls of one judge to repair its error | implemented | decision 0041 and decision 0045: every panel carries two model families and the last judge before ship is hired across the vendor line from the smith; registry entry 0009 reads the same question |
| 5 | When verdict quality is inspected repeatedly and stopped when it looks decisive, use an anytime-valid sequence, not a fixed-sample interval | not-planned | |
| 6 | Cluster a comparison at the unit that generates the tasks: tasks from one repository need not be independent | not-planned |  |

## Candidates

Finding 2 is the one that fits what Brokkr already does and does not
keep. The operator is a full human audit of every shipped branch, which
is stronger than the paper's 12 percent sample, but the outcome
(merged as shipped, merged after rework, rejected) is not journaled
against the run's gate verdicts, so the journal cannot say how often a
passing review was wrong. Decision 0116 already rules that a work order
is done when merged, not when shipped, and entry 0019 cites #436 for
outcome evaluation; a typed operator outcome on a concluded run is the
label this paper needs, and the calibration of every judge hire follows
from it by counting.

Finding 1 qualifies how such a scorecard may be read. A judge's record
on small docs deliveries says little about its record on a 2,000-line
change; decision 0045 calls the journal's first served gates a hire's
scorecard, and this paper says to keep that scorecard per shape of run
(recipe, diff size, turns) rather than as one number.

Finding 6 bears on every wager Brokkr runs: all its tasks come from one
repository, the unit the paper says to cluster on. A wager's arms can
be compared; the result should not be read as a ranking of models at
large.
