# The Model Is a Detail

Twenty years ago we were taught that the database is a detail. The framework is a
detail. The web is a detail. Good architecture, the argument went, keeps policy at
the center and pushes every volatile, replaceable, IO-shaped thing to the edges,
behind boundaries that point inward.

Then large language models arrived, and we forgot all of it in about eighteen
months.

Look at the topology of almost every agent framework shipping today: the model
sits at the **root**. It decides what happens next. It spawns sub-agents. It
routes work, judges completion, retries itself, and hands off when it feels
done. The control flow of the system *is* the model's output stream. We took the
most volatile, least explainable component ever admitted into a production
system and installed it in the position we spent two decades learning to
protect.

I think this is exactly backwards, and I've spent the last weeks building — or
more precisely, operating a machine that builds — the inverted version. The
claim of this essay is simple:

> **First law: determinism belongs in control flow; stochastic execution
> belongs in leaf effects. Second law: any feedback loop that can be
> specified deterministically must be — judgment never guards its own
> walls. The model is a detail.**

## What a leaf effect is

Take the oldest discipline we have for mixing pure logic with a messy world:
the *effect*. Pure code never touches the world; it **requests** an interaction,
the boundary executes it, and only the recorded outcome re-enters the logic.
Databases journal intent before acting. Event-sourced systems fold immutable
logs into state. Functional-core/imperative-shell pushes IO to a thin rind
around a pure center.

Now put the agent there. In the system I operate —
[Brokkr](https://github.com/feedback-loop-ai/brokkr), a delivery engine that
ships changes to its own repository — an agent session is a **leaf effect**:

- The core is a phase state machine whose transition table is
  [JSON data](https://github.com/feedback-loop-ai/brokkr/tree/main/bundles/self),
  folded deterministically from an append-only, hash-chained journal.
- When the machine needs work done, it journals `effect/requested` and spawns a
  driver — a Claude session, a Codex session, whatever sits behind the
  [driver contract](https://github.com/feedback-loop-ai/brokkr/tree/main/contracts).
- The session works, streams checkpoints, and concludes with a **typed result**:
  `complete`, `blocked`, `has_security_residual: true, max_residual_severity: low`.
- The machine consumes the result. Never the session. The next transition is
  decided by the table, and the decision — rule id, inputs, cause — is journaled
  like everything else.

A leaf controls nothing. It cannot transition a phase, spawn another agent,
retry itself, or touch the journal's decisions. Even parallelism is data: a
panel of five reviewers is five leaves side by side, fanned out and joined by
the engine because the *bundle* says so, not because an agent called an agent.

Intelligence works. The trunk rules.

## Why the inversion matters

Every engineering lineage that had to build trustworthy systems out of
untrustworthy parts converged on this shape, independently:

- **Erlang/OTP**: supervisors are boring and deterministic; workers crash
  freely. Nobody puts the flaky thing in the supervision tree's root.
- **Databases**: the write-ahead log is law; execution strategies are whatever
  the planner felt like today. Recovery replays the log, not the mood.
- **CPUs**: speculative execution runs wild — but *retirement is in order*, and
  the architectural state only ever records the deterministic story.

The reason is arithmetic, not taste. An error rate in **leaf position** is
bounded per leaf and catchable by the gate behind it. The same error rate in
**control position** compounds multiplicatively down every decision path. A
worker that flakes 2% of the time is a retry. A scheduler that flakes 2% of the
time is an unexplainable system — and today's agent frameworks are shipping
schedulers that flake far more than 2% of the time, then adding more agents to
watch the first ones, which is more stochastic control flow, which is more
compounding.

Model-at-the-root doesn't just risk failure. It forecloses the questions that
matter afterward: *why did the system do that? can you reproduce it? who
approved this change and on what evidence?* When control flow is sampled from a
distribution, the honest answer to all three is a shrug.

When control flow is a fold over a journal, the answers are `grep`.

The axis now has a mainstream name. Andrew Ng's AI Engineering Skills
Map (August 2026) draws the spectrum from *workflows* — a predefined
sequence of LLM calls — to *agent harnesses* that let the model
repeatedly decide its own next step, and makes choosing your point on
that spectrum a core engineering skill, alongside the disciplined
eval and error-analysis loops that turn an unpredictable system into
a governable one. Brokkr is a deliberate stake at the far workflow
end — for the delivery loop itself. The sequence is a signed table;
the harness's freedom lives only *inside* each leaf; and the eval
loop Ng asks for is the journal, where every decision is graded by
its own record. The industry default drifts toward harnesses because
they demo well. Our claim is that the outermost loop of software
delivery is precisely where the workflow end wins.

## What it buys, concretely

This is not a thought experiment. The machine builds itself — roughly ninety
pull requests to its own repository, every one driven by the phase machine,
executed by leaf sessions, and recorded. Some receipts, each checkable:

- **Adversarial gates catch what authors miss.** A review seat — a leaf whose
  only job is to refute — found a stored XSS in agent-written UI code before it
  shipped. The run's journal records the ruling that stopped the ship
  (`REVIEW-RESIDUAL-SECURITY`), the reasoning, and the remediation that
  followed.
- **The law is data, so changing the law is a diff.** When a severity-blind
  stop rule proved too blunt, the fix was a
  [decision document](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0022-reforging.md)
  and a change to a JSON table: security findings now send the run *back* to
  the implement phase, bounded at two returns, with the finding threaded to the
  returning seat as declared input. No retraining, no prompt archaeology — a
  reviewed, versioned, digest-pinned rule change.
- **The system audits its own legislation.** The run that implemented that very
  rule change was reviewed by a leaf that found a fail-open bug *in the new
  rules* — an absent severity field would have shipped where the old law
  stopped. The finding is in the journal; the fix landed before merge
  ([PR #87](https://github.com/feedback-loop-ai/brokkr/pull/87)).
- **Models become comparable, because they're interchangeable.** With agents as
  leaves behind one contract, we ran the same feature under two rival crews —
  Claude and Codex — and judged the artifacts, not the vendors. The synthesis
  of both landed ([PR #85](https://github.com/feedback-loop-ai/brokkr/pull/85)),
  co-credited, with both runs' journals as the evidence line. Try running that
  experiment in a framework where the model *is* the control flow: you can't,
  because swapping the model swaps the system.
- **And yesterday, the machine used the new law on itself.** A review found a
  residual in a fresh feature; the journal records `REVIEW-REFORGE · review →
  implement`; a new leaf session received the finding and answered it. The
  argument — finding, return, fix, re-verify, re-judgment — is one journal's
  story, replayable end to end.

None of this requires believing me. The journals export, redacted of machine
detail, and the fold is deterministic: same events, same state, on your machine
as on mine.

## The honest limits

Three caveats, because a paradigm that hides its edges is a pitch, not a
paradigm.

**The boundary is an altitude — but not a free one.** Inside one implement
session, the model makes hundreds of micro-decisions — which file, which
test, which approach. The precise claim is: *stochastic control below the
accountability boundary, deterministic control above it* — and the second
law fixes where the boundary may sit: at the determinability frontier.
Whatever can be specified — a signature check, an expiry, a severity
comparison, a merge tally — sinks into deterministic code by obligation;
judgment is lawful only where determinism is impossible, like deciding
whether code is secure, never whether eight checks are green. I learned
this one personally: the AI agent operating this machine enforced the
merge tally by discipline and failed three times in a single day —
deleting branches under open pull requests each time. The same tally as
branch-protection law cannot fail. The distinction between a stochastic
judge (lawful) and a stochastic enforcer (a bug) is the sharpest line in
the whole paradigm.

**The law is still written stochastically.** Humans and models argue; an
operator rules; the table changes. Determinism governs the *execution* of ruled
law and the honesty of its records — never the wisdom of the rules. That is
what constitutions are for.

**Fluidity has real value.** For open-ended exploration, a standing
conversational agent is genuinely pleasant, and rigid phases cost ceremony.
The answer is layering, not denial — looser tables for looser work, and
conversational surfaces *on top of* the deterministic core. Which points at the
asymmetry that makes this a bet worth taking:

> You can add fluidity to a deterministic core. You cannot retrofit determinism
> onto a fluid one. Bolting an audit log onto stochastic control flow records
> the chaos; it does not tame it.

## The pattern recurses

The sharpest objection to all this: *"fine — but who runs the machine at
3am? The moment you add a standing agent to operate it around the clock,
you've put a stochastic scheduler back on top, and your whole inversion
collapses."*

It doesn't — because the pattern recurses. Distinguish agency from
control: control flow is the power to change what happens next outside
your own envelope. Our standing steward has agency — choice among
granted actions — and zero control: it acts only through a signed,
expiring grant it cannot edit, below a compiled ceiling it cannot
reach, through command vocabularies a deterministic loader validates,
with every exercise journaled against the grant's hash. Determinism
verifies; judgment never guards its own walls.

Which reveals the architecture as self-similar. At run scale: the
engine requests, a model works inside a typed envelope, the result
folds back as data. At fleet scale, the identical shape one level up:
the human operator *grants* — a request in slow motion — the steward
works inside an envelope it cannot modify, and the journal folds its
actions back. The steward is not above the machine; it is a leaf effect
of the operator's loop. Even the operator's own authority enters as
signed artifacts and journaled rulings.

So the motto deserves its precise scope. The machine is the outer
**SDLC loop**: nothing in the software delivery lifecycle wraps it — no
model schedules it, no agent gates its phases, no judgment overrides
its journal. It is not the outer **operator loop**: the human's loop
wraps everything, as it should, and the standing steward serves that
loop, not the machine's. Call the machine the middle loop if you like —
the invariant doesn't move: **every stochastic mind in the system —
model or human — touches execution only through instruments determinism
can verify.**

## The prediction

Here is the falsifiable version. As agent autonomy and agent count scale,
systems with stochastic control flow will hit a wall — compounding failures,
unexplainable outcomes, releases nobody can sign for — and the demand side
(production incidents, security review, regulation, plain professional
self-respect) will start asking journal-shaped questions. Systems built
model-at-the-root will face a rebuild. Systems built model-as-a-detail will
face a feature request.

If instead models become so reliable that control-position flake stops
mattering, this paradigm's advantage shrinks to auditability alone — and I'll
take that consolation prize, because "prove what the machine did" is not a
requirement that gets repealed.

The database was a detail. The framework was a detail. The model is a detail —
the most capable detail we have ever had, worth every seat we can give it at
the edges of the system. The center belongs to law you can read, records you
can replay, and rulings someone signed.

The machine is the outer loop.

---

*Brokkr is open source: [github.com/feedback-loop-ai/brokkr](https://github.com/feedback-loop-ai/brokkr).
The repo's history is the demo — start with the
[decision documents](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/decisions)
and the [Edda](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/lore/edda.md),
which explains why the bellows are named Brokkr.*
