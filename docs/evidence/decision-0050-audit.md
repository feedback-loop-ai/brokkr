# Decision 0050 against `main`: the audit (#429)

[Decision 0050](../decisions/0050-the-machine-proves-what-it-promises.md) was
**proposed** when this audit was taken. The operator accepted it on
2026-09-29 on this audit, taking every recommendation below, and the
decision's addendum records the readings. The audit was taken against
`main` at `e12f24f1` on 2026-09-29, under the operator's ruling of
2026-09-28 for #429: audit first, then rule. The change that carries this
file builds only checks that add no refusal and no transition to a table
that loads today. Every check below that would refuse such a table is
listed under [The operator's ruling](#the-operators-ruling).

## The seven rulings

| Ruling | Status | Where it is built | Gap |
|---|---|---|---|
| 1. Presence | partial: reported, not refused | `Machine::audit_with`, `Finding::Unread` | no refusal in `Machine::from_table`; `bundles/verify` and `recipes/preflight` are still v1 |
| 2. Order | partial: the unconditional shadow is refused, every other dead rule is reported | `Machine::from_table`; `Machine::audit_with`, `Finding::Shadowed`, `Finding::Covered` and `Finding::Unsatisfiable` | no refusal of a dead rule at load or compile; the decision's condition-wise wording misses vacuous guards and collective cover |
| 3. Liveness | partial: reported, not refused | `Machine::audit_with`, `Finding::Unreachable` and `Finding::DeadEnd` | no refusal |
| 4. Totality | partial: swept, bounded and reported by `brokkr compile` | `Machine::audit_with`, `Finding::Unruled`, `AuditError::Budget` | no refusal; no shipped table names its closed valuations |
| 5. Stated properties | partial: the three properties hold on every shipped table at the valuations the test evaluates; `clean` is held at the plain verdict only | `crates/brokkr-runtime/tests/table_lints.rs` | the clean property's domain awaits a ruling; the sequence leg waits on ruling 7; no `table_properties.rs` |
| 6. Fold arms total | unbuilt | none | no transition table and no enumeration test |
| 7. Sequence endings compiled | unbuilt | none | the ending is still a run-time comparison, and a hard word reported by a non-final step does not end the sequence |

### Ruling 1: presence

- **Built, as a diagnostic.** `Machine::unread` in
  `crates/brokkr-core/src/policy/audit.rs` reports every rule that advances
  without reading a seat input that a hard rule of its group reads. The
  caller names the engine-owned inputs: `brokkr compile` and the runtime
  tests pass `brokkr_runtime::bundle::is_engine_owned`.
- **Tests.**
  `presence_names_an_advancing_rule_that_skips_a_hard_input` in
  `crates/brokkr-core/src/policy/audit/tests.rs` covers the negative case
  and five valid counterparts: the rule reads the input, returns to the
  phase, re-enters it directly, stops or parks, or the engine owns the
  input. In `table_lints.rs`,
  `presence_refuses_exactly_the_three_v1_tables` pins today's three
  findings, and `the_0022_era_permissive_arms_fail_open` reproduces
  seq 335.
- **Reading taken.** A rule whose next phase is its own phase *returns*.
  The ruling says a returning rule is one whose "every road onward
  re-enters the phase"; a self-loop re-enters it at once. The prototype
  sweep counted a self-loop as advancing. No pinned finding moves, because
  the only self-loop in a shipped table (`SHIP-READY`) sits in a group
  whose hard inputs are engine-owned.
- **Gap.** `Machine::from_table` refuses nothing for presence.
  `bundles/verify` and `recipes/preflight` are still
  `forge.phase-machine/v1`, and their `REVIEW-RESIDUAL-OK` advances to
  `done` reading neither `has_security_residual` nor
  `max_residual_severity`. The frozen heritage table carries the same
  shape in `REVIEW-CLEAN-UNVERIFIED` and `REVIEW-RESIDUAL-OK`, and keeps
  it by design.

### Ruling 2: order

- **Built.** `Machine::from_table` in `crates/brokkr-core/src/policy.rs`
  refuses a rule preceded in its group by an unconditional rule
  (`ruled_unconditionally`). `crates/brokkr-core/tests/policy_lint.rs`
  covers it in `loader_rejects_structural_defects`.
- **Built, as a diagnostic.** `Machine::sweep` and `dead` in `audit.rs`
  read deadness from the totality sweep: a rule that rules no swept
  valuation of its group is dead. The sweep samples every threshold at and
  around it, so this is exact for the five guard forms over the present,
  well-typed valuations of a group's inputs, with every counter integral
  (every shipped counter is engine-owned and integral). An absent input
  satisfies no condition, so a rule reported dead behind `skip_verify:
  true` and `skip_verify: false` still fires when a seat omits the flag.
  Absence belongs to presence (ruling 1). Each dead rule is
  reported one of three ways. `Finding::Shadowed` names the first earlier
  rule that holds wherever it holds. `Finding::Covered` names the earlier
  rules that together hold wherever it holds, when no one of them does.
  `Finding::Unsatisfiable` is a guard that holds on no valuation.
- **Found in review, and fixed.** The first cut compared guards condition
  by condition (`Condition::implies`), as decision 0050 words ruling 2,
  and held that two condition forms never imply each other. So a vacuous
  guard of another form ahead of a hard rule left the hard rule dead and
  unreported. Examples are `max_residual_severity_at_most: critical` ahead
  of a hard `max_residual_severity_above: medium`, `visits_work_gte: 0`,
  and a `strategy_in` over the whole vocabulary. So did earlier arms that
  together partition an axis, such as `skip_verify: true` and
  `skip_verify: false`. Presence was satisfied because the permissive arm
  read the input, and totality because it ruled everything. The audit
  reported nothing. No shipped table had the shape: the semantic sweep
  finds no dead rule in any of the 19 tables
  (`every_shipped_table_is_ordered_and_live`).
- **Tests.** In `audit/tests.rs`,
  `a_rule_is_dead_where_it_rules_no_swept_valuation` covers the three
  vacuous guards, the partitioned axis and two unsatisfiable guards, each
  with its valid order where one exists, and pins the report text of
  `Covered` and `Unsatisfiable`.
  `a_guard_is_shadowed_only_by_an_earlier_guard_it_implies` has 17 pairs,
  both directions of each form, across inputs and across forms.
  `a_shadowed_rule_names_the_first_rule_that_subsumes_it` pins the
  report text.
  `the_swapped_self_arms_load_today_and_the_audit_names_both_rules`
  exchanges `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM` and
  `REVIEW-REFORGE-EXHAUSTED-MEDIUM` in `bundles/self`. Today's loader
  admits the table, and it parks a high residual at the bound where the
  ordered table stops. The audit names both rules.
  `the_ordered_self_table_leaves_only_the_severity_none_hole` is the
  valid counterpart. For composed tables,
  `an_overlay_that_shadows_or_opens_a_hole_is_reported_on_the_flat_table`
  in `crates/brokkr-runtime/src/bundle/compose_tests.rs` shows a derived
  rule prepended ahead of the base rule it subsumes.
- **Measured on 2026-09-29.** `brokkr compile --bundle` on a copy of
  `bundles/self` with the two rules exchanged exits 0 and prints
  `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM is dead behind
  REVIEW-REFORGE-EXHAUSTED-MEDIUM`.
- **Gap.** Neither the loader nor the compiler refuses a dead rule.
  Decision 0050's own definition of subsumption ("each of the earlier
  guard's conditions is implied by a condition of the later one") is
  incomplete in the same way the first cut was. It misses vacuous guards
  and collective cover. See item 2 of the list below.

### Ruling 3: liveness

- **Built, as a diagnostic.** `Machine::liveness` in `audit.rs` reports a
  phase unreachable from `initial`. It also reports a phase whose every
  road reaches neither a terminal phase nor a phase with a parking rule.
- **Tests.** `liveness_names_the_unreachable_phase_and_the_dead_end` in
  `audit/tests.rs` covers an unreachable phase, a spinning dead end and
  the same spin made live by a parking rule.
  `every_shipped_table_is_ordered_and_live` in `table_lints.rs` admits
  every shipped table.
- **Gap.** There is no refusal.

### Ruling 4: totality, and every ending named

- **Built, as a diagnostic.** `Machine::sweep` in `audit.rs` enumerates
  each group's axes and evaluates every valuation with the real
  `Machine::evaluate`. The axes are both flags; a counter at zero and at
  one below, at and one above each threshold; the six severities; and the
  closed `strategy` and `drift_in` vocabularies. The sweep is measured
  before it runs. A table past `SWEEP_BUDGET` (65,536 valuations) returns
  `AuditError::Budget`, which names the group where the budget was
  crossed, and nothing is evaluated. `brokkr compile` prints the sweep's
  size, every finding and the first unruled valuation on stderr
  (`sweep_report` in `crates/brokkr-cli/src/verbs/setup.rs`), and exits
  as it did before.
- **Tests.** `the_sweep_walks_each_axis_at_and_around_its_thresholds`
  pins the domain size of every axis and the exact holes, and has a total
  counterpart.
  `the_sweep_is_measured_before_it_runs_and_refuses_past_its_budget`
  checks that a budget of exactly 47 passes `bundles/self` and 46 does
  not. A group of 92,160 valuations is refused before any is evaluated.
  `compile_reports_the_sweep_or_why_it_was_not_swept` covers the CLI's
  two arms.
  `the_unruled_valuations_are_pinned_per_table` in `table_lints.rs` pins
  the counts below.
- **Gap.** Compile refuses nothing for totality, and `NoRule` is still
  what a run records for the `residual`/`none` valuation. The shipped
  delivery tables name no parking rule for it.

### Ruling 5: stated properties

- **Built, under test.** `the_stated_properties_hold_on_every_shipped_table`
  in `table_lints.rs` walks every bundle and every composed recipe. It
  checks that a plain `clean` verdict (`fixes_applied: false`, nothing
  else) rules to a phase from which a non-stop terminal is reachable. The
  ruling's wording is unqualified, and the test does not sweep the clean
  group. It pins the one shipped counterexample instead:
  `REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED` parks `review`/`clean` at
  `strategy` `design`, `spec_defect: true`, `visits_specify: 3` in
  `recipes/triage` and in `recipes/gpt-flash` and `recipes/night-shift`,
  which carry the same groups. See item 9 of the list below. It checks
  that a low non-security residual on a first visit
  does not stop. This change adds the third property: `security-hold`
  rules a hard stop at every swept valuation of its group, in every table.
  No valuation of the group is unruled, and every rule of the group is a
  hard stop. Before, only the
  heritage table was checked (`table_wide_lint_matches_python_suite` in
  `crates/brokkr-core/tests/differential.rs`).
- **Gap.** The ruling names a new `crates/brokkr-core/tests/table_properties.rs`.
  The properties live in `table_lints.rs`, beside the composer they need.
  The "from every step of a sequence" leg waits on ruling 7.

### Ruling 6: the fold's arms are total

- **Today.** `apply` in `crates/brokkr-core/src/fold.rs` matches on the
  event type, and within each arm on the cursor. There are 15 event types
  and 9 cursor states. Six wildcard arms (`fold.rs` lines 351, 369, 406,
  416, 438 and 462) refuse as `OutOfPlace` or `AfterTerminal`. They fail
  closed, but no table states them. `the_mid_flight_arm_refuses_everything_it_is_not`
  and `open_effect_guards_refuse_foreign_effects_in_the_same_cursor_shape`
  in `crates/brokkr-core/src/fold/tests.rs` cover parts of the relation.
  The recorded journal
  `fixtures/journals/tui-graph-the-selection-box-gets-80f98deb.ndjson`
  exists.
- **Gap.** There is no transition table and no test that enumerates all
  135 pairs. Writing the table so that it restates today's arms adds no
  refusal, so it needs no ruling. It moves `fold.rs`, which is outside
  this story's files, and is a follow-up.

### Ruling 7: the sequence's endings are compiled

- **Today.** The executor in `crates/brokkr-runtime/src/engine.rs`
  (`let ends_sequence`, line 2469) compares the reported word with the
  later steps' vocabularies at run time. There is no compiled `ends_on`.
  Compile does not refuse a table arm whose word no step can reach. A
  non-final step's hard word does not end the sequence, so a panel's
  `security-hold` stops the run only if the chief reproduces it. The
  chief's charter states that floor as prose
  (`the_chief_charter_states_the_floor_it_may_not_rule_below` in
  `crates/brokkr-runtime/tests/crucible_review_sequence.rs`).
- **Gap.** All of it. The hard-word ending is new transition semantics
  and needs the ruling. The sequence parser lives in
  `crates/brokkr-runtime/src/bundle.rs`, which is held for #226 and #349.
  The executor is moving under #288 slice B.

## Measured valuation counts

Measured on 2026-09-29 by `brokkr compile --bundle <table>`. The frozen
heritage table is not a bundle; its count comes from `table_lints.rs`,
which pins every row below through the same `Machine::audit_with`.

| Table | Groups | Valuations | Unruled | Other findings |
|---|---|---|---|---|
| `policy/phase-machine.json` | — | 57 | 0 | presence: `REVIEW-CLEAN-UNVERIFIED`, `REVIEW-RESIDUAL-OK` |
| `bundles/self` | 11 | 47 | 4 | — |
| `bundles/verify` | 5 | 16 | 0 | presence: `REVIEW-RESIDUAL-OK` |
| `recipes/fast` | 10 | 46 | 4 | — |
| `recipes/gpt-flash` | 29 | 1,072 | 128 | — |
| `recipes/landing` | 12 | 48 | 4 | — |
| `recipes/night-shift` | 29 | 1,072 | 128 | — |
| `recipes/node` | 10 | 46 | 4 | — |
| `recipes/panel-review` | 11 | 47 | 4 | — |
| `recipes/preflight` | 5 | 16 | 0 | presence: `REVIEW-RESIDUAL-OK` |
| `recipes/release` | 10 | 46 | 4 | — |
| `recipes/research` | 5 | 5 | 0 | — |
| `recipes/research-dsh` | 5 | 5 | 0 | — |
| `recipes/review-first` | 10 | 46 | 4 | — |
| `recipes/standby` | 10 | 46 | 4 | — |
| `recipes/triage` | 29 | 1,072 | 128 | — |
| `recipes/wager-harness` | 10 | 46 | 4 | — |
| `recipes/wager-harness-dsh` | 10 | 46 | 4 | — |
| `recipes/wager-harness-muse` | 10 | 46 | 4 | — |
| **19 tables** | | **3,825** | **428** | |

Every unruled valuation has one shape: a `residual` verdict at
`max_residual_severity` `none`. The largest table sweeps 1,072
valuations, 1.6% of the budget. No shipped table has an order or
liveness finding.

## The operator's ruling

On 2026-09-29 the operator accepted decision 0050 whole and took every
recommendation below: item 2's semantic definition with absence outside
the domain, item 5's named park before the totality refusal, and item
9's plain verdict. The list stays as it was put to the operator.

1. **Accept decision 0050**, whole or by ruling. The rest of this list
   assumes the ruling it names is accepted.
2. **Order (ruling 2): refuse a dead rule in `Machine::from_table`.**
   The rule is any of `Finding::Shadowed`, `Finding::Covered` or
   `Finding::Unsatisfiable`, raised as a `PolicyError` that names the dead
   rule and the rules it sits behind. Today it refuses no shipped table.
   It would refuse the exchanged `bundles/self`, any overlay that
   prepends a subsuming rule, and a vacuous or partitioning arm ahead of
   a hard rule. The decision words ruling 2 condition by condition, and
   that wording misses the last shape. *Recommendation: enable on the
   semantic definition (a rule that rules no valuation of its group's
   domain is dead), and amend the decision's wording to match.* Enabling
   the condition-wise wording alone would ship a refusal with a known
   fail-open shape. Nothing shipped moves either way. The semantic
   definition must also say whether an absent input is in the domain.
   Today's sweep leaves it out, so a fallback behind arms that partition
   a flag is reported dead although it fires when a seat omits the flag.
   Refusing it would refuse a valid defensive park. *Recommendation:
   absence is outside the domain, because presence (item 4) covers it;
   the refusal says the rule fires on no present valuation.*
3. **Liveness (ruling 3): refuse an unreachable phase or a dead end in
   `Machine::from_table`.** Today it refuses no shipped table.
   *Recommendation: enable.*
4. **Presence (ruling 1): refuse `Finding::Unread` in a v2 table.** Today
   it refuses no v2 table. The v1 tables (the frozen heritage table,
   `bundles/verify`, `recipes/preflight`) load as before.
   *Recommendation: enable for v2 now.* Then, in their own slice, move
   `bundles/verify` and `recipes/preflight` to v2 with `REVIEW-RESIDUAL-OK`
   reading `has_security_residual: false` and
   `max_residual_severity_above: none`. Their policy digests and witness
   pins move with that slice.
5. **Totality (ruling 4): refuse an unruled valuation at compile.**
   Today it would refuse 14 of the 19 tables (428 valuations), every one
   of them the `residual`/`none` shape. *Recommendation: first rule the
   named park for that valuation* (its id, reason and position), add it
   to each delivery table and re-pin the witnesses the digests move.
   Then enable the refusal.
6. **The budget.** Today a table over `SWEEP_BUDGET` is reported and
   compiles. *Recommendation: refuse it once totality is enabled.* A
   table that cannot be swept cannot be shown total.
7. **The sequence (ruling 7): end a sequence on a hard word from any
   step, compile `ends_on`, and refuse an unreachable arm.** This is new
   transition semantics in the engine. *Recommendation: accept, and
   build it after #288 slice B, with the `bundle.rs` parser change after
   #226 and #349.*
8. **The self-loop reading under ruling 1.** A rule whose next phase is
   its own phase returns. *Recommendation: confirm.*
9. **The clean property's domain (ruling 5).** The ruling says "a clean
   verdict rules to a phase from which a non-stop terminal is reachable".
   If that ranges over every valuation of the `review`/`clean` group,
   `recipes/triage`, `recipes/gpt-flash` and `recipes/night-shift` break
   it today: `REVIEW-CLEAN-SPEC-DEFECT-EXHAUSTED` parks a clean verdict
   whose specification stayed defective after three visits. If it ranges
   over the plain verdict, every shipped table holds it.
   *Recommendation: the plain verdict, and amend the ruling to say so.*
   The park is the operator's to take by design, and a clean verdict that
   carries a spec defect is a return, not a clean result.
