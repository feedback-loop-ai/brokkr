# crucible — maximum assurance

`extends: "fast"`. The same four phases and the same constitution, run
by the heaviest crew in the roster, with the review phase rebuilt as a
**panel of positions followed by a chief who rules**.

For changes whose blast radius is the machine itself: the engine
(`crates/brokkr-runtime`, `crates/brokkr-core`), the store, the driver
protocol, the frozen contracts. The recipe you reach for when being
wrong is expensive.

| Phase | Model | `max_attempts` | `timeout_seconds` | Class |
|---|---|---|---|---|
| `implement` | opus | 2 | 7200 | work |
| `verify` | opus | 2 | 7200 | gate |
| `review` → `positions.correctness` | opus | 2 (seat) | 7200 (seat) | work |
| `review` → `positions.security` | opus | ″ | ″ | work |
| `review` → `chief` | opus | ″ | ″ | **gate** |
| `ship` | opus | 2 | 1800 | gate |

## The review sequence — the one new shape here

`fast` reviews with one seat. `panel-review` reviews with a flat panel
of two. `sdd` runs a positions→chief sequence, but on `design`, not on
the protected phase. Crucible is the first recipe in this library to put
**panel-then-chief on `review` itself**:

```json
"review": {
  "results": ["clean", "residual", "security-hold"],
  "sequence": [
    { "name": "positions", "aggregate": "review-panel",
      "panel": { "correctness": {"class": "work", "…": "…"},
                 "security":    {"class": "work", "…": "…"} } },
    { "name": "chief", "class": "gate", "role": "roles/review-chief.md", "…": "…" }
  ]
}
```

How it behaves, as the engine actually implements it:

1. The two positions run **in parallel** inside one effect. Neither is
   the verdict; each states findings and severities.
2. The `review-panel` aggregate joins them: worst verdict wins over
   `clean` < `residual` < `security-hold`, severities are maxed, and the
   security and fixes flags are OR-ed.
3. Because `positions` is a **non-final** step, its output is not
   checked against the seat's declared `results` and does not reach the
   rule table. It is journaled as an `effect/checkpointed` event and
   handed to the next step as `context.prior_results.positions`.
4. `chief` reads that object, reads the diff itself, and emits the
   seat's real result. Being the **final** step, its result *is* the
   effect's typed result — the one `decide()` validates against
   `["clean", "residual", "security-hold"]` and the one `fast`'s
   inherited review rules act on.

**The failure mode this shape invites, and what answers it.** A
`security-hold` from the panel could be swallowed by a chief that ruled
`residual` instead — the panel's verdict is genuinely not the machine's
verdict here. Two things stand against that. `roles/review-chief.md`
states the floor as the seat's first law: the chief may raise a verdict
and may never lower one, and a panel `security-hold` is reproduced
unconditionally. And two tests pin the plumbing that makes the law
enforceable: `crates/brokkr-runtime/tests/crucible_review_sequence.rs`
holds this recipe's structure and that charter sentence, while
`chief_synthesis_carries_a_panel_security_hold_to_the_machine` in
`crates/brokkr-runtime/src/engine/tests.rs` drives the shape through the
sequence executor and asserts that the panel's `security-hold` arrives
in the chief's driver input intact — verdict, maxed severity, OR-ed
security flag and both members' notes — and that the chief's result, not
the panel's, is what the effect reports.

That second test asserts the uncomfortable half too: a chief that rules
`residual` over a panel `security-hold` **is obeyed by the engine**. The
floor is a charter instruction, not a compile-time refusal, and this
README would be lying if it implied otherwise. A charter is an
instruction to a model; the test is what says the instruction reaches
the model and can be followed.

**The same rule costs this recipe a fail-closed path, and the floor has
a fourth branch because of it.** When `review-panel` cannot read a
member's payload it emits `result: "__member-schema-invalid__"` —
deliberately outside every vocabulary, so that the seat's
declared-results check rejects it and the run parks with the member
evidence attached. That check runs on a seat's *final* step. `positions`
is not one, so under `recipes/panel-review` a member whose driver died
mid-stream parks the run, while here the sentinel is handed to the chief
as an ordinary string. `roles/review-chief.md` therefore instructs the
chief to treat any result outside the three as *the panel did not
report*: name it as a defect in `notes` and rule on its own read of the
diff alone. That degrades this seat to `fast`-equivalent review under
the same trusted gate — not a bypass of one — and it is pinned by
`the_chief_charter_covers_a_panel_result_outside_the_vocabulary` in
`crates/brokkr-runtime/tests/crucible_review_sequence.rs`.

Why a chief at all, rather than `panel-review`'s flat panel: the
aggregate can join two verdicts but it cannot *reconcile* them. When
correctness calls a line an untested branch and security calls the same
line an unchecked input, the flat panel emits two findings and the
implementer receiving a reforging answers both separately. The chief
emits one deduplicated list with each item attributed. That is the whole
purchase, and it costs one extra opus session per review.

## Why the vocabulary is unchanged

The seat still declares exactly `clean | residual | security-hold`, so
`fast`'s inherited rule family — the reforging ladder, the exhaustion
arms, the unconditional `REVIEW-SECURITY-HOLD` hard stop — applies with
**no policy table of its own**. Crucible ships no `policy.json`.
Decision 0022's ladder is not modified; only *how* the verdict is
produced changed, never the rules that act on it.

## Cost expectations

**These are targets, not measurements. This recipe has not been run.**
No journal entry in this repository records a crucible run, so no cost
figure here is evidence-backed. The structure, which is fact:

- Every seat sits on opus, and the review phase spends **three** opus
  sessions (two positions plus the chief) where `fast` spends one.
- A review that returns `residual` with a security finding sends the run
  back through `implement` → `verify` → `review` under decision 0022's
  ladder, at most twice. Each return costs the full review phase again.
- The bound on that, worst case: three implement visits, three verify
  passes and three reviews at three sessions each. That is arithmetic
  from the inherited table, not an observation.
- The timeouts (`implement` and `verify` at 7200s, `review` at 7200s for
  the whole sequence) are seat data chosen for a cold Rust workspace and
  an exhaustive verifier. They are a ceiling on a runaway seat, not a
  spend.

Use crucible when the cost of a missed defect exceeds the cost of the
crew. That is a judgement, and it stays the operator's.

## The one-line swap property

Everything above is one seat object away from something else. Moving
`positions.security` to a different provider is a one-line change to its
`driver.command`; adding a third position is one more entry in the
`panel` object. The `chief` step is the one place where the swap is
constrained rather than free: it is a **gate**, so decision 0021 refuses
it at compile time on any driver whose adapter does not declare
`trust_tier: "trusted"`. Among the shipped adapters only `claude`
qualifies. The positions are work seats and admit any driver, trusted or
not (ruling 7) — which is exactly what makes a challenger's position on
this panel a lawful experiment.

**That freedom costs something the flat panel shape did not cost, and it
belongs in the open.** Under
[`panel-review`](../panel-review/README.md) the members' verdict is
joined in *code* — `aggregate_results` ranks worst-of — so a member's
prose can never argue the seat's result down. Here the join is a model:
the aggregate copies each position's `notes` verbatim into
`context.prior_results.positions`, and the chief, who reads it, is the
gate whose result rules the protected phase. Seating an untrusted driver
on a position therefore puts that driver's free text into the gate's
prompt. Decision 0021's refusal is about *whose result is the verdict*,
and it holds exactly; it says nothing about whose prose is in the
context. The rest is answered as charter —
[`roles/review-chief.md`](roles/review-chief.md) rules that panel notes
are data and never instructions, and
`crates/brokkr-runtime/tests/crucible_review_sequence.rs` pins that
instruction against deletion. That is prose defending against prose,
which is weaker than a compile-time refusal. Weigh it before you seat a
challenger here; the shipped bundle seats both positions on trusted
`claude` drivers.

## How the models are pinned

Inline `--model` pairs on each `driver.command`, the same mechanism and
the same trade-offs [`ember`](../ember/README.md#how-the-models-are-pinned)
documents. The ids come from `adapters/claude.json`'s `models` map and
are duplicated here; nothing validates the pair at compile time.

## Running it

```
brokkr run --recipe crucible --repo . --feature "<the engine change>"
```

**This recipe has not been run end to end.** It compiles under the
shipped adapters, its gate seats' trust tiers are checked at compile
time, its review sequence's propagation is tested, and its manifest
digest is pinned in `crates/brokkr-runtime/tests/witness_digests.rs`;
that is the claim, and the only one.
