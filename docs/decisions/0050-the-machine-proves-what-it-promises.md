# 0050 — The machine proves what it promises: presence, order and totality at load, every ending named, and the inner machines declared

Status: proposed
Date: 2026-09-08

## Context

On 2026-09-08 the operator asked whether the phase machine is a formal
finite-state machine, what it would take to make it one, and what the
record shows it cost when it was not. The question was read against the
tree at 818f0a7, the canonical journal (154 runs, 814
`transition/decided`, 51 `run/parked`, 65 `run/stopped`), the decision
record, and the merged pull requests.

**What the machine is.** `Machine::evaluate`
(`crates/brokkr-core/src/policy.rs`) is a pure function
`(phase, result, inputs) → Ruling | Park | NoRule` over a table of
first-match-wins rules whose guards are five closed condition forms —
counter at-least, severity above, severity at-most, flag, enumeration
member — on a registry of inputs the loader enforces at load (decision
0004). Control state is finite and declared; the data the guards read
lives in the journal and arrives as an argument. It is a guarded
transition system with priority resolution, not a plain automaton, and
its topology is linear by constitution (decision 0002), chosen so that
verification stays enumeration: "the exhaustive sweep that found a real
table gap on day one enumerates `phases × results × inputs`; over
markings and interleavings it becomes model checking."

**What is proven today.** At load: the closed vocabulary; no rule leaves
a terminal; no rule shadowed by a preceding unconditional rule for its
`(phase, result)`; a park only in a v2 table and never with a severity or
an artifact gate (decision 0022). At compile
(`crates/brokkr-runtime/src/bundle.rs`): every result a seat may emit has
a covering rule; a rule may not read an input its seat cannot supply
(decision 0007); and `assert_phase_unavoidable` — with the protected
phase deleted, no non-stop terminal is reachable from `initial`. Under
test: 97 corpus cases against the heritage table
(`crates/brokkr-core/tests/differential.rs`), the table-wide lint (hard ⇒
stop, flagged ⇒ non-terminal, `security-hold` ⇒ hard stop), and arm
tests for two shipped tables.

**What the record says.** The outer table has never ruled `NoRule` on a
well-formed result: 813 rulings, and one `NoRule` on a schema-invalid
result file (seq 29 of `the-first-pr-experience-a-strang-5880d7a`), which
is decision 0001 working. The 51 parks divide as 14 rule-driven (the
0022 and 0042 exhaustion ladders), that one no-ruling, one
`GATE-MOVED-HEAD`, 26 failed effects and 9 indeterminate ones; the 65
stops as 33 hard stops by rule and 32 operator stops. The formal part did
not bite. What bit was around it, and each incident is cited:

1. **A total, deterministic, proven table that ruled wrongly.** Between
   2026-08-23 and 2026-08-31 `REVIEW-RESIDUAL-SECURITY` hard-stopped
   thirteen runs, among them
   `implement-decision-0022-reforgin-54a88e9b`, the run implementing its
   replacement. Decision 0022 records the consequence: every
   stopped-but-good diff landed through an operator pull request,
   "bypassing the ship phase entirely." The cut-vertex proof held while
   the road to `main` went around the machine. The property that would
   have caught it — a run with fixes applied and only low or info
   residuals can reach ship — is a reachability question over the table
   and its inputs, and it was never stated.
2. **The fail-open decision 0004 predicted, caught by a model.** 0004
   documented that "when a deny rule is conditional and the permissive
   fallback is unconditional, an absent input still reaches the
   fallback," and deferred a lint: "any input referenced by a hard
   conditional rule must be engine-computed or schema-required." Decision
   0007's lint refuses a rule that reads an input its seat never
   declares; it does not require presence. On 2026-08-31 the 0022 run's
   reviewer found (seq 335,
   `docs/evidence/implement-decision-0022-reforgin-54a88e9b.ndjson`):
   "Absent `max_residual_severity` on a security residual now SHIPS where
   it used to hard-stop … lands on REVIEW-REFORGE-EXHAUSTED-DEBT → ship,
   flagged. Confirmed empirically against the real bundles/self table."
   The fix was authored into the tables — `max_residual_severity_above:
   none` on the permissive arms — and the lint that would catch the next
   one was not written.
3. **Rule order is unproven.** A copy of `bundles/self` with
   `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM` and
   `REVIEW-REFORGE-EXHAUSTED-MEDIUM` exchanged compiles (`brokkr
   compile`, exit 0, 2026-09-08). Under that table a high residual at the
   bound parks where the constitution says it stops. The hard ⇒ stop lint
   reads each rule alone; the loader refuses shadowing only by an
   unconditional rule. Nine `(phase, result)` groups in the heritage
   table and six in `bundles/self` are decided by order, and composition
   prepends derived rules (`crates/brokkr-runtime/src/bundle/compose.rs`),
   one more seam.
4. **The exhaustive sweep retired with the oracle.** Decision 0009 retired
   `tools/generate_evaluator_corpus.py` with the Python evaluator, and
   3c55d04 removed the last heritage code. The corpus is frozen at 97
   cases over the heritage table, which no run has used: the journal's
   154 runs are 57 on `self`, 32 on `wager-harness`, 11 on `fast`, and so
   on down, and none on `policy/phase-machine.json`. Every v2 table since
   is proven by hand-written arm tests; two of the nine tables with their
   own `policy.json` have them. The liveness lints 0004 listed for the
   port — "all phases reachable; every phase reaches a terminal" — were
   not ported: `crates/brokkr-core/tests/policy_lint.rs` holds two tests.
5. **The inner machines have no table, and that is where the fail-opens
   were.** The fold's `Cursor` (`crates/brokkr-core/src/fold.rs`) is nine
   hand-written states; 92f731a records the fold refusing a journal the
   engine itself wrote ("`OperatorAccepted` is impossible at cursor
   `EffectInFlight`"), which made a live run unreadable. The sequence's
   end predicate (`ends_sequence`, `crates/brokkr-runtime/src/engine.rs`)
   was identically false at #186 — an author step reporting `upstream`
   was walked past — a high fail-open behind thirteen green checks, found
   by a judging pass and fixed in #199. The gate-head check was
   effect-wide instead of per gate step (#199). The engine-case review
   sequence holds `security-hold ⇒ stop` by charter: "a `security-hold`
   from the panel therefore only stops the run if the chief reproduces
   it" (`crates/brokkr-runtime/tests/crucible_review_sequence.rs`). The
   failure counter read only `failed | broken`, so the artifact phases'
   `fail` retry was unbounded — the MEDIUM that stopped
   `decision-0042-docs-decisions-004-1c311081` — and was closed as a
   special case (`scoped_failure`). Two lifecycle predicates have no
   statement at all: "the driver reports accepted at spawn and the
   provider's limit refusal arrives after, so decision 0016's
   fail-to-start predicate never held"
   (`judge-the-branch-fire-0042-sdd-m-b3e824d4`, 2026-09-04); "retry
   accepted at seq 1816 was never driven — no engine process, no journal
   movement … in the five hours since"
   (`close-the-forge-to-brokkr-rename-206fc661`, 2026-09-03).

**The experiment.** `brokkr compile` admitted the exchanged table. The
four checks this decision rules were then run on 2026-09-08 over every
shipped table — the nine with their own `policy.json` and the six
composed by `extends`, 2,613 valuations in all — by
`crates/brokkr-runtime/tests/table_lints.rs`, which walks the tables
through the real composer and the real `Machine::evaluate`, reading only
the guards from the flat table, and PINS today's findings so that a
table change that moves one is a reviewed change:

| Check | Shipped tables | Negative cases |
|---|---|---|
| Order — a rule dead behind a weaker guard | all fifteen admitted | the exchanged `self` refused: `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM is dead behind REVIEW-REFORGE-EXHAUSTED-MEDIUM`; a high residual at the bound rules `PARK` |
| Liveness — every phase reachable, every phase ends | all fifteen admitted | — |
| Presence — a hard rule's seat input is read by every advancing rule | the three v1 tables refused: the heritage table, `bundles/verify` and `recipes/preflight`, where `REVIEW-RESIDUAL-OK` advances reading neither the severity nor the security flag (the heritage table's `REVIEW-CLEAN-UNVERIFIED` likewise); every v2 table admitted | the 0022-era `self` refused; under it an absent severity with the security flag set rules `REVIEW-REFORGE-EXHAUSTED-DEBT → ship` — the finding at seq 335, reproduced |
| Totality — every valuation of present, well-typed inputs is ruled | every v2 delivery table has unnamed valuations, all of one shape: a `residual` verdict whose `max_residual_severity` is `none` — four in each `fast`-shaped table, 128 in `triage` and `night-shift` | — |
| Stated properties — `clean` can go on; a low non-security residual does not stop | all fifteen hold | — |

Two things the experiment says. Totality and presence pull against each
other: the heritage table is total because its fallback is
unconditional, which is exactly the fail-open; the v2 tables are closed
because their permissive arms require a severity, which leaves the
`none` valuation unruled. Both hold at once only when the closed
valuation is *named* — a rule that parks it with a reason. And nothing
in the experiment needed a model checker: the largest table sweeps in
1,072 valuations.

Alternatives weighed:

- **A model checker (TLA+, Alloy) over the table.** Rejected. Decision
  0002 fixed the topology so that the sweep is enumeration; a finite
  linear table with closed guards is exhausted by its own domain, and
  the properties worth stating are reachability queries a test can ask.
  A checker would be a second model of the machine, free to drift from
  the loader.
- **Regenerate the corpus in place.** Rejected by the house rule and by
  decision 0009: the corpus is frozen and only ever versioned beside.
  This decision adds no case to it and reads it as it is.
- **Leave it to the judging pass.** Rejected as the only line. The pass
  costs a gate at xhigh per catch; it found the 0022 fail-open because
  one reviewer chose to probe, and the `upstream` fail-open stood behind
  thirteen green checks until a second judgment was bought. A lint costs
  nothing per run.
- **A general graph runtime for the inner machines.** Not ruled on here,
  and not ruled out. Decision 0002 sanctions concurrency inside an
  executor — "seat fan-out and parallel sub-machines, journaled as
  sub-events under the phase's span … concurrency inside, serialization
  at the boundary" — and the target architecture names the primitives
  (`parallel`, `join`, `loop`, `submachine`, `emit-result`). What this
  decision asks of an inner machine is the same whether it is today's
  cursor and sequence or a graph: a declared state space, total arms,
  endings computed at compile, and the constitution's hard words crossing
  into it (rulings 6 and 7). A graph runtime that arrives under that
  discipline is welcome; one that arrives without it repeats #186.

## Rulings

1. **Presence.** In every `(phase, result)` group of a table, every
   seat-declared input read by a `hard` rule is read by every rule in the
   group that *advances* — a rule whose next phase reaches a non-stop
   terminal without re-entering the group's phase. A rule that *returns*
   (every road onward re-enters the phase, where the hard rule rules
   again) may read fewer. Engine-owned inputs are always present and are
   exempt. The law is `forge.phase-machine/v2` vocabulary: a v1 table
   loads as it always has, so the frozen heritage table keeps the
   behaviour its corpus records, and `bundles/verify` and
   `recipes/preflight` move to v2 and satisfy it — `REVIEW-RESIDUAL-OK`
   reads `has_security_residual: false` and
   `max_residual_severity_above: none`, so an omitted severity on a
   residual parks with the notes instead of passing flagged. This is
   decision 0004's deferred lint, stated so that it needs no schema.

   **Enforcement binding:** `Machine::from_table` in
   `crates/brokkr-core/src/policy.rs`, refusing with `PolicyError`
   naming the group, the input and the advancing rule;
   `crates/brokkr-core/tests/policy_lint.rs` — the 0022-era shape (from
   `docs/evidence/implement-decision-0022-reforgin-54a88e9b`) refused,
   the shipped shape admitted; the two amended tables and their witness
   pins; the frozen table read under v1 as before.

2. **Order.** No rule is preceded in its group by a rule whose guard
   subsumes it — every condition of the earlier guard implied by a
   condition of the later one on the same input: a lower counter
   threshold, a lower severity floor, a higher severity ceiling, an equal
   flag, a superset enumeration; an empty guard subsumes every guard.
   This generalises the unconditional-shadow refusal that stands today
   and reads a composed table as the flat table `compose` produces.

   **Enforcement binding:** `Machine::from_table`; `policy_lint.rs` — the
   exchanged `bundles/self` refused naming both rules, every shipped table
   admitted; `crates/brokkr-runtime/src/bundle/compose_tests.rs` — a
   derived rule that subsumes the base rule it precedes is refused at
   compile.

3. **Liveness.** Every phase is reachable from `initial` over transition
   edges, and every non-terminal phase reaches a terminal or a parking
   rule. The two lints decision 0004 listed for the port.

   **Enforcement binding:** `Machine::from_table`; `policy_lint.rs`; every
   shipped table admitted.

4. **Totality, and every ending named.** At compile, for every group,
   `brokkr compile` enumerates the domain of every input the group's
   rules read — both flags, the counter at, below and above each
   threshold, the six severities, the closed enumerations — and refuses
   a valuation of present, well-typed inputs that no rule rules. A table
   names its closed valuations with a rule that parks them and a reason,
   so the journal records a rule id where it recorded "no ruling".
   `NoRule` remains for what a table cannot foresee — a schema-invalid
   result, an unknown phase, an unreadable input — under decision 0001
   unchanged. The enactment adds the named park to every shipped
   delivery table for a `residual` verdict at severity `none`. `brokkr
   compile` prints the sweep's size and result beside the policy digest;
   the frozen corpus is neither read nor regenerated by it.

   **Enforcement binding:** the compiler in
   `crates/brokkr-runtime/src/bundle.rs`, refusing with the first unruled
   valuation named; `crates/brokkr-runtime/tests/table_lints.rs`, which
   pins the sweep size and the holes per shipped table today, and whose
   pins the enactment drives to zero — a table with a hole refused,
   every shipped table admitted.

5. **Stated properties.** Every shipped table with a protected phase
   carries three tests over the composed table: a `clean` verdict rules
   to a phase from which a non-stop terminal is reachable; a non-security
   residual at or below `low` on a first visit does not stop;
   `security-hold` rules a hard stop from every phase that admits it, in
   every table and, under ruling 7, from every step of a sequence.
   Decision 0022's arc is the reason: the table that ruled a diligent
   reviewer's note a death was total, deterministic and wrong, and the
   property that says so is a test.

   **Enforcement binding:** `crates/brokkr-core/tests/table_properties.rs`
   (new), walking every `policy.json` and every composed recipe; the
   constitutional lint in `differential.rs` unchanged.

6. **The fold's arms are total.** The fold's transition relation — cursor
   by event type — is written as one table, and a test enumerates every
   pair and asserts each is a transition, an explicit `OutOfPlace`, or an
   explicit terminal annotation; no wildcard arm. 92f731a is the case it
   pins.

   **Enforcement binding:** `crates/brokkr-core/src/fold.rs`;
   `crates/brokkr-core/src/fold/tests.rs` — the enumeration, and the
   recorded journal
   `fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson`
   still folding.

7. **The sequence's endings are compiled, and the constitution crosses
   into it.** For every non-final step the compiler computes `ends_on`:
   the words in the seat's vocabulary the step may report that no later
   step can emit, pinned on the compiled step; the executor ends the
   sequence on a word in `ends_on` and never compares vocabularies at run
   time. Compile refuses a table arm for the phase whose word no step can
   reach — the reverse of the coverage rule that stands today. And a
   non-final step whose result is in the seat's *hard* vocabulary — a
   word whose every table arm is `hard` — ends the sequence with that
   word wherever it is reported: a panel's `security-hold` is the
   machine's. The chief's charter floor stays as prose and stops being
   the only defence.

   **Enforcement binding:** the sequence parser in `bundle.rs` and the
   executor in `engine.rs`;
   `crates/brokkr-runtime/tests/crucible_review_sequence.rs` gains the
   behavioural leg — a panel reporting `security-hold` under a chief that
   would rule `clean` ends the effect at `security-hold` and the run
   stops; a compile test for the unreachable arm.

## Consequences

- **What moves.** The loader gains three refusals and the compiler two.
  `bundles/verify` and `recipes/preflight` move to
  `forge.phase-machine/v2`, and every shipped delivery table gains one
  named park; their policy digests move, and the witness pins are
  re-recorded as the identity change they are. The fold gains a table
  and a test; the sequence executor loses a run-time comparison and
  gains a compiled fact and one hard-word rule.
- **What it costs.** The sweep is bounded by the tables' own domains —
  2,613 valuations across every shipped table today, 1,072 in the
  largest — and runs at compile, once per run. Nothing is added to a
  seat's work or a run's wall clock.
- **What stays.** Decision 0001: `NoRule` parks. Decision 0002: the outer
  machine is linear. Decision 0004: absence never satisfies, an
  unreadable input parks, first match wins. Decision 0009: the corpus is
  frozen. The heritage table is read under v1 exactly as the corpus
  says.
- **Left open, named.** The two lifecycle predicates in the record — when
  an attempt has *started* (decision 0016's fail-to-start fallback that
  never held), and a `retry` accepted with no engine driving it (the fold
  reading `running` with nothing behind it) — are lifecycle rulings, not
  table rulings, and get their own number. And the ledger's scope, read
  from the gate's own log (decision 0038 ruling 6): since the label has
  existed (2026-09-03), 67 pull requests merged. Ten passed at
  `vouched` with no label; nine predate the gate script. Of the 48 the
  gate saw labelled, 40 named no run at all — 29 said so in the body
  ("Brokkr-Run: none — by hand"), 11 carried no line — three cited a
  run that judged the branch but could not land it (stopped at the
  reforging bound under the operator's ruling, or a judging pass that
  never ships), three were labelled on a tier the gate would have
  ruled `vouched`, and one carried a code delta after its judgment.
  The tiers misfired once; the label marks a class of work the
  constitution has no road for — hand-authored shop work, forty of
  whose commits carry a session's co-author line. A proof about the run
  says nothing about that class, and a recipe that enters at `verify`
  and ships is the road, ruled under its own number.
