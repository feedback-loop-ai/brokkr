# 0039 — The docs tier inside the run: a review that fixed only prose does not buy the whole verify again

Status: proposed
Date: 2026-09-03

## Context

Decision 0038 cut the contribution gate's tiers by what changed since a
judgment. The same day, the run that delivered 0038
(`decision-0038-evidence-follows-c-0d0aa35f`) showed the same waste one
layer down. Its review seat ruled `clean` with fixes four times. Three of
those fixes were code and earned their re-verify. The fourth changed two
lines of two decision documents, and the `fast` table's `REVIEW-CLEAN`
rule sent it back through the full verify seat and a fourth review all
the same — about a third of the run's $33 for a judgment nobody needed
twice. The operator ruled: add the docs tier to the `fast` recipe.

The table cannot rule on a fact the engine does not compute. The
condition vocabulary is closed (decision 0004), evaluation inputs are
engine-owned or seat-declared (decision 0007), and "the review changed
only prose" is a fact about the tree that must not come from the seat
that made the change. So the tier needs one new engine-owned input, and
the input needs to know two things the journal did not yet carry: the
head the review was handed, and which paths count as docs.

Alternatives weighed:

- **Let the review seat declare `fixes_docs_only`.** Rejected: a seat
  that can say its own fixes were harmless has been handed the gate.
  Decision 0007 exists to keep that door shut.
- **Hard-code the docs class in the engine.** Rejected by 0038 ruling 3,
  which made the class repository-owned data because the repository
  knows what its prose is and the engine does not.
- **Diff from the last recorded `reviewed_heads`.** Rejected: that head
  is recorded at the review's ruling, after its fixes, so it names the
  end of the change and not the start. The start is the head at entry,
  and nothing recorded it.
- **Skip verify after any clean-with-fixes review.** Rejected as the
  plain weakening it is; code fixes keep their re-verify.

## Rulings

1. **The protected phase is entered at a recorded head.** `phase/entered`
   for the protected phase carries `head`, the repository's HEAD at that
   moment, as an optional, absent-by-default payload field at
   `event_schema: 1` under the amended rule in `contracts/README.md`.
   Absent when the run has no repository or no readable head. `fold`
   does not read it.

   **Enforcement binding:** `contracts/phase-entered-head.v1.schema.json`;
   the engine's `phase_entered_payload`; a test that the protected phase
   carries the head, another phase does not, and a run without a
   repository carries none.

2. **`fixes_docs_only` is an engine-owned boolean.** At the protected
   phase's ruling the engine diffs the recorded entry head to the
   current head, renames unpaired, and answers `true` when every path
   lies in the repository's docs class and `false` otherwise. It is in
   the closed condition vocabulary and in the engine-owned list: a seat
   that claims it has the claim dropped, a bundle that declares it fails
   compilation.

   **Enforcement binding:** `BOOLEAN_INPUTS` in `brokkr-core`,
   `ENGINE_OWNED_INPUTS` in `brokkr-runtime`, the engine's
   `fixes_docs_only`; tests for the true case, the false case, and the
   overwritten claim.

3. **The docs class is the repository's, read as the gate reads it.**
   The engine reads `.github/delivery-classes.json` from the repository
   the run works in — `classes.docs.paths`, regular expressions over the
   repository-relative path — so one file governs both the pull request
   gate (0038 ruling 3) and the tier inside the run. When the question
   has no honest answer — no entry head recorded, no readable head, the
   same head, an empty diff, a diff git cannot take, or no class declared
   or parseable — the input is absent, and an absent input never
   satisfies a rule (decision 0004). A repository that declares no class
   therefore keeps exactly the behaviour it had.

   **Enforcement binding:** the engine's `docs_class`; a test for each
   absence.

4. **`fast` ships a clean review whose fixes were docs-only.** The rule
   `REVIEW-CLEAN-DOCS-FIXES` — `clean`, `fixes_applied: true`,
   `fixes_docs_only: true`, `next: ship`, severity `flagged` — sits
   between `REVIEW-CLEAN-NO-FIXES` and `REVIEW-CLEAN`. Every recipe that
   extends `fast` inherits it; recipes with their own tables adopt it by
   their own edit. The five affected witness digests move, and that
   movement is this ruling.

   **Enforcement binding:** `recipes/fast/policy.json`; the
   constitutional lint still finds review on every path;
   `crates/brokkr-runtime/tests/witness_digests.rs` re-pins the five and
   names this decision as the reason.

## Consequences

- Under this table the run that motivated it would have shipped after
  its fourth review directly, saving one verify and one review pass.
- The ship seat's own gates and the pull request's CI still run every
  test, doc tests included, so a docs-only fix that breaks a doc test is
  caught at the landing rather than inside the run. That is the same
  backstop the gate's docs tier relies on, stated here so the saving is
  read for what it is.
- `REVIEW-CLEAN-NO-FIXES` still trusts the seat's `fixes_applied: false`
  claim. With an engine-owned view of what the review committed now in
  hand, that rule can be tightened to an engine-owned fact in a later
  ruling; this decision does not.
- Recipes that do not extend `fast` — `node`, `panel-review`, `sdd`,
  `sdd-paranoid`, `preflight`, `bundles/self`, `bundles/verify` — are
  unchanged and may adopt the rule when their owners choose.
