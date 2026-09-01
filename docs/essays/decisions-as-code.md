# Decisions as Code

Every engineering organization above a certain size has a graveyard, and it
is usually called `docs/adr/`. Architecture Decision Records were a good
idea — write down what you decided and why — with a fatal flaw: nothing
enforces them. An ADR is advice to the future, and the future is busy. Six
months later the record says one thing, the system does another, and nobody
can tell you when they diverged. We built a discipline for remembering
decisions and forgot to give it hands.

Policy-as-Code went the other way. OPA, Sentinel, branch protection,
admission controllers — rules with real teeth, compiled and enforced. But
PaC keeps only the rule and discards the *ruling*: who decided, when,
against what alternatives, and what got amended since. Compiled law with
the legislative history thrown away. When the rule bites someone two years
later, the only answer to "why is this here?" is archaeology.

Running an autonomous delivery machine forced us to fuse the two, because
agents make both failure modes fatal at machine speed: un-enforced
decisions get ignored by every session that never read them, and
un-explained rules get "fixed" by every session that finds them
inconvenient. What emerged has a lifecycle worth naming:

> **Decisions as Code: deliberate → rule → encode → enforce → evidence →
> amend.** The decision documents are the constitution's source code; the
> tables and loaders are its compiled artifact; the journal is its test
> suite.

The name has neighbors worth naming. "Decisions as code" appears in the
decision-*optimization* world (Nextmv and kin), where it means compiling
mathematical decision models — routing, scheduling — with developer
ergonomics; a different universe borrowing the same words. And the
*enforce* step here has honest prior art: evolutionary architecture's
**fitness functions** (Ford, Parsons, Kua) taught us a decade ago that
architectural decisions should be verified by automated checks. What
that lineage lacked is everything around the check: the ruling grammar,
the amendment record, and the evidence loop that grades the law. DaaC is
fitness functions given a constitution.

## The lifecycle, concretely


In [Brokkr](https://github.com/feedback-loop-ai/brokkr), every semantic
change to the machine gets a numbered decision in
[`docs/decisions/`](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/decisions).
A decision is a short document with a hard grammar: a **status line that
names who ruled and when** ("accepted — operator ruled 2026-09-01"), the
context, numbered rulings, and consequences. So far, an ADR. The
differences are everything after the writing:

**Every determinable ruling must compile.** This is the second law of our
execution paradigm applied to governance: *a rule that can be specified
deterministically must never be enforced stochastically* — not by an
agent's discipline, not by a reviewer's memory, not by a document's
existence.
[Decision 0022](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0022-reforging.md)
(security findings send a run back to the implement phase, bounded, with a
severity ladder at exhaustion) is not a description of intended behavior —
it *is* a JSON transition table the engine folds. Decision 0021 (which
models may hold judging seats) is a bundle-compiler refusal. The ruling
that all commits be signed became branch protection the same hour it was
made. A decision whose rules stay prose when they could be a loader
refusal is a bug in the constitution.

**Amendments happen on the record, attributed.** When the operator caught a
bias in a draft ruling (two vendors treated asymmetrically with zero
evidence differential), the fix wasn't a silent edit — it's a dated
erratum in the document, plus a disclosure ruling recording that the
drafting agent had a conflict of interest. When a lore claim proved wrong,
the correction names who corrected it and when. The document you read
carries its own legislative history.

**And the loop closes: the law is graded by its own execution.** Every
ruling the machine takes cites a rule id; every rule id traces to a
decision. So "did decision 0022 work?" is not a retrospective opinion — it
is a query. The answer, as it happens: 0022 was ruled in the morning, and
by evening a live run had exercised the entire ladder — two remediation
returns, a park for the operator, a resumed remediation, a lawful ship —
and the run's own review found a fail-open gap in the new rules, which
became 0022's first amendment before the rules ever merged. ADRs cannot
be graded. PaC cannot explain itself. A decision that is simultaneously
literature and law can do both.

## What it takes

The practice is small enough to start this week, agent or no agent:

- **A status grammar**: `proposed` until a named human rules; `accepted`
  carries the ruler and the date. Nothing merges as law without it.
- **An enforcement binding per determinable ruling**: each ruling names
  the mechanism that will refuse violations — a config, a CI gate, a
  schema, a loader. If no mechanism exists, either build it or admit the
  ruling is judgment-guidance and mark it so.
- **Amendment by erratum**: corrections are dated, attributed additions.
  The diff history is not the amendment record; the document is.
- **Rule ids in your telemetry**: whatever enforces a ruling should emit
  the decision's id when it fires, so the law's real-world hit rate is
  queryable.

The honest limits: not everything compiles. Editorial standards, naming
taste, the reasons behind a bound — genuine judgment-guidance stays prose,
and pretending otherwise produces brittle theater. The demand is only
that *determinable* rulings compile, and that the boundary between the
two is drawn explicitly rather than by fatigue. And there is ceremony
cost: a ruling is slower than a Slack message. That is the point. The
ceremony is exactly proportional to how much you will later wish you had
it.

## Why now

Because agents made governance load-bearing. A policy nobody enforces
was survivable when the actors were five humans with institutional
memory; it is not survivable when the actors are fifty model sessions
with no memory at all, each reading the repo fresh. For an autonomous
machine, un-compiled law simply does not exist — and un-explained law
gets refactored away by the first session that finds it in the way. The
only decisions that survive contact with agents are the ones that are
code where they can be, literature where they must be, and evidence all
the way down.

The decisions are the source. The enforcement is the build. The journal
is the test suite. Ship your constitution the way you ship everything
else.

---

*Brokkr's decision record is public:
[docs/decisions](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/decisions)
— twenty-five decisions, their errata, and the
[journals](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/evidence)
that grade them.*
