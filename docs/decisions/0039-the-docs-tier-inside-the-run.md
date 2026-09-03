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
   phase's ruling the engine diffs the head recorded on the phase's
   latest entry to the current head and answers `true` when every path
   lies in the repository's docs class and `false` otherwise. The diff
   is the anchor's (0038 ruling 1) and the gate's: plumbing, renames
   unpaired, paths unquoted, so a non-ASCII prose name is looked up as
   itself and a code file moved under a docs name stays a code
   deletion. The entry head is taken as the contract's shape or not at
   all — forty hex digits, checked before git is spawned — because the
   journal chain is unkeyed and a row in it is not an argument list.
   The input is in the closed condition vocabulary and in the
   engine-owned list: a seat that claims it has the claim dropped, a
   bundle that declares it fails compilation.

   **Enforcement binding:** `BOOLEAN_INPUTS` in `brokkr-core`,
   `ENGINE_OWNED_INPUTS` in `brokkr-runtime`, the engine's
   `fixes_docs_only` over the anchor's `changed_paths`; tests for the
   true case, the false case, the overwritten claim, a code file moved
   under a docs name, a non-ASCII docs name, and an entry head shaped
   as an option, which reaches no git and writes no file.

3. **The docs class is the repository's, read at the entry head.** The
   engine reads `.github/delivery-classes.json` — `classes.docs.paths`,
   regular expressions over the repository-relative path — as committed
   at the head the phase was entered at, never from the working tree,
   so one file governs both the pull request gate (0038 ruling 3) and
   the tier inside the run. The gate reads the base branch's copy so
   the judged delta cannot move its own classifier; the run reads the
   entry head's copy for the same reason: the tree at ruling is the
   judged phase's own, and a phase that could widen the class its fixes
   are judged by has been handed the gate — the first rejected
   alternative, re-entering through a data file. A change to the class
   file is therefore a path like any other, classified by the class it
   was entered under, and `.github/` is code under any honest class; it
   buys the verify and the review that judge it as the code change it
   is. A widening the protected phase commits itself governs no entry
   before a later visit; one committed before the phase was entered
   governs that entry and lies in the delta the same review reads, so
   the reviewer who lets it pass is the failure, as for any other code.
   The pull request gate reads the base branch's copy in every case, so
   no widening a run let through moves the gate's tier. When the
   question has no honest answer — the phase's
   latest entry recorded no head or one that is not a commit id, no
   readable head, the same head, an empty diff, a diff git cannot take,
   or no class committed or parseable at the entry head — the input is
   absent, and an absent input never satisfies a rule (decision 0004).
   A repository that declares no class therefore keeps exactly the
   behaviour it had; one that declares an empty class has said nothing
   of it is prose, and reads `false`. The patterns are compiled by the
   `regex` crate here and joined into jq's dialect by the gate: a
   pattern only one accepts is refused here and reads as no class,
   fail-closed.

   **Enforcement binding:** the engine's `docs_class`; a test that the
   tree is not consulted and that a widening committed by the phase
   reads false; a test for each absence, and for the empty class.

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

## Addendum — 2026-09-04, amended by decision 0041, accepted

Decision 0041 ruling 4 retires the review that fixes: no gate changes
the tree, and `fixes_applied` leaves every shipped recipe, so the
`REVIEW-CLEAN-DOCS-FIXES` arm this decision adds has nothing left to
price. The engine-owned input survives and moves to the smith's return
(0041 ruling 5e): a returning implement whose delta lies wholly in the
repository's docs class re-enters review without verify. The mechanism
this decision builds — the recorded entry head and the docs class read
as the gate reads it — is what that edge stands on; only the arm that
priced a judge's fix retires with the judge's fixes.
