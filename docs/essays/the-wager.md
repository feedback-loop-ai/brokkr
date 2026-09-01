# The Wager

We ran the same feature twice — once with a Claude crew, once with a
Codex crew — and the honest version of that story requires two
confessions from the referee. Both are in the journal, which is the
only reason I trust the result enough to write about it.

The commission was real work, not a benchmark: `brokkr export
--redact`, a sanitized-evidence exporter with genuine security
stakes — path scrubbing, username scrubbing, derivative marking so a
sanitized copy can never pass as verbatim evidence. Crew A was Claude
under our standard recipe. Crew B was Codex under a recipe that
differed by exactly one line: the implement seat's driver command.
Same repository base, same phases, same gates, same judge. That
one-line swap is the whole trick, and I will come back to it, because
most agent stacks cannot do it at all.

## First confession: the referee drafted the rules

The wager existed because of [decision
0021](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0021-model-policy.md):
which models may hold which seats is operator-granted, tier by tier,
and a newcomer's tier is *earned by evidence* — its first outings are
wagers, the same feature run under each crew and compared.

Here is the problem: the machine's standing steward — the agent that
drafted that policy — runs on Claude. A Claude-based agent wrote the
first draft of the rules deciding whether Codex gets a seat. I
read the draft and called the bias plainly: it treated the
newcomers as suspect by default, with zero evidence differential. The
ruling that followed is now law: newcomers enter symmetrically, and
any AI agent drafting policy about a rival model *discloses the
conflict on the record*. The disclosure is in the decision document.
An argument about fairness that starts by hiding who wrote the rules
is over before it begins.

## Second confession: the referee rigged round one

Round one, crew B blocked mid-implement. The journal says why, and
the why was me: I had launched the Codex sessions in a tighter
sandbox than the Claude sessions — write access denied where crew A
had it freely. Crew A ran with edit permissions; crew B hit the walls
of a cage crew A never saw. Not malice — a default I hadn't
questioned, which is what bias usually is.

The machine is why this did not stay invisible. The blocked run is a
hash-chained journal ending in a typed refusal, and it survives in
the canonical journal to this day — and where the record is cited,
it is labeled for what it is: round one, rigging-blocked. The
rematch ran under parity, and only the rematch counted. A fair test
is not one where the referee is unbiased — no such referee exists,
human or model. A fair test is one where the rigging leaves a record.

## The judging of the gifts

Both crews delivered. Then the artifacts were judged — the code, the
tests, the residual lists — with the vendor names mattering not at
all, and the result embarrassed both fanbases:

**Crew A** (Claude) supplied the better architecture: redaction
placed in the store beside export verification, and whole-token
username scrubbing that covered a leak *only A caught* — the
journal's own operator field records a username outside any path,
and B's version would have shipped it.

**Crew B** (Codex, rematch) supplied the better recognizer:
boundary-based, quote-aware, cross-platform — POSIX paths, Windows
drive letters, UNC shares — and naturally safe on non-ASCII input,
which closed a residual *A had left open*.

What landed ([PR #85](https://github.com/feedback-loop-ai/brokkr/pull/85))
is the synthesis: A's architecture carrying B's recognizer,
co-credited, with both crews' residual lists answered by the union.
Neither crew's artifact was shippable alone. Both were wrong
somewhere; each was wrong somewhere the other was right. And the
synthesis itself did not escape argument — at landing, a scanner flaw
surfaced (a colon-separated `PATH` list swallowed whole as one path)
and the fix, colon termination that still respects drive letters,
landed with the semantic rulings recorded as tests.

The Edda has an entry for this, because the myth got there first:
nothing the dwarves made was Mjölnir until the gods had weighed it
against every other gift and ruled. The making and the judging are
never the same act.

## Why this is hard everywhere else

Try to run this experiment on a model-at-the-root framework. You
cannot — not because the models refuse, but because *swapping the
model swaps the system*. When the model is the control flow, a
different model gives you different phase boundaries, different
retry behavior, different everything; the comparison drowns in
confounds. The wager is only possible because our crews are leaf
effects behind one driver contract: the machine holds the phases,
the gates, the journal, and the judge constant, and the crew is a
variable you can actually isolate.

That is the quiet thesis under the loud one. Fair model comparison
is an *architecture property*. You get it when the racetrack does
not care who runs — and the racetrack not caring is exactly what
deterministic control flow means.

Two byproducts, because real experiments leak value sideways. The
wager forced the driver contract to be genuinely symmetric — the
Codex adapter had to report checkpoints and results in the same
typed vocabulary, which hardened the contract for every future
driver. And it exposed that Codex sessions reported no dollar cost,
which eventually became a feature: cost cells that render journaled
token counts when no price exists, never a fabricated conversion.
Billing truth, forced by a fairness exercise.

## Tiers are earned, and re-earned

The wager's verdict fed decision 0021's tier grants — evidence, not
reputation, and not permanence either: a tier is an operator grant
that can be re-examined the same way it was earned. The instrument
is standing. Any newcomer — the next model, the next vendor, a local
model on our own hardware — walks in through the same door: same
commission, same gates, parity rigging, artifacts judged blind of
the logo.

I know how the alternative works, because everyone runs it: models
are chosen by benchmark screenshots, vibes, and whichever vendor's
narrative is loudest that quarter. We replaced that with a
one-line-diff experiment, two confessions, and a co-credited merge —
and the whole argument, rigging included, is a journal you can
export and verify.

The gods judged the gifts. They did not ask which dwarf made them.

---

*The wager pair — crew A and the rematch — is exported on the
[evidence shelf](https://github.com/feedback-loop-ai/brokkr/tree/main/docs/evidence);
the rigging-blocked round one stands in the canonical journal beside
[decision 0021](https://github.com/feedback-loop-ai/brokkr/blob/main/docs/decisions/0021-model-policy.md)
and the synthesis,
[PR #85](https://github.com/feedback-loop-ai/brokkr/pull/85).*
