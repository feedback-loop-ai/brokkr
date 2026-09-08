# 0051 — Landing: a branch written by hand is judged and shipped by a run, and the label goes back to being the exception

Status: proposed
Date: 2026-09-08

## Context

On 2026-09-08 the operator read the contribution gate's numbers and
asked the right question: since decision 0038's label has existed, four
pull requests in five carry `by-hand` — "it may not be the right gate if
we have to make a workaround on 81% of the PRs." The record answers
from the gate's own log line, the one decision 0038 ruling 6 added so
that "the label's use … can be read back as a rule that needs work
rather than a rule that was bypassed."

**What the gate said.** Since 2026-09-03, 67 pull requests merged to
`main`. Ten passed at tier `vouched` with no label; nine predate the
gate script. Of the 48 the gate saw labelled:

| The gate's log line | Pull requests | What it means |
|---|---|---|
| `Brokkr-Run must carry a native run id` — the body says `Brokkr-Run: none — by hand (decision 0033)` | 29 | no run existed; the author said so |
| `the tier would have been unknown` — no `Brokkr-Run` line at all | 11 | no run existed |
| the cited run is `stopped`, not completed (#160, #161); `never ruled shipped` (#212) | 3 | a run judged the branch but could not land it: stopped at the reforging bound under the operator's ruling, or a judging pass, which ends at review |
| `the tier would have been vouched` (#162, #186, #199) | 3 | the label was applied where the gate would have passed |
| `code delta since run …` (#167) | 1 | commits after the judgment; the vouch died, as ruling 3 says it must |
| the gate's own bootstrap (#158) | 1 | — |

The tiers misfired once. Forty of forty-eight labels are one fact: no
run existed. The work was not the machine's — decisions (15), release
and packaging (8), the dsh lane (4), recipes (3), adapters, CI, prompt
text, the store — and forty of those forty-six commits carry a session's
co-author line. Shop work, authored in a session with the operator, by
hand in the constitution's sense.

**Why it has no road.** Decision 0033 ruling 1 reads "every pull
request to `main` is delivered by a Brokkr run," and every recipe that
can ship begins by implementing a commission. `preflight` verifies and
judges an existing branch but ends at `review`; the gate's first check
on a `Brokkr-Run` is that the run `shipped`, so a preflight named as the
run is refused — #212 carried the label for exactly that. The docs tier
0038 ruling 3 cut needs a delivery run beside the preflight. `fast`
starts at `implement`; commissioning it with "change nothing" pays a
smith to read and misdescribes the work in the journal. So hand-authored
work took the only door that opened, and the escape hatch became a road.
Pull request #182 — a research sweep, prose only, eleven files under
`docs/research/` — stands open today, unlabelled, with no run to name.

**What it would cost to do it properly.** The 21 run-delivered pull
requests of the same five days cost $368 on the journal's own seat
records: median $9.19, from $2.50 to $89.45, implement included. A
judgment without an implement is one review seat, $1.70 to $15 on the
same records; a verify is an exec seat and costs minutes, not dollars.

**The proof.** `recipes/landing` — `fast` composed by `extends`, its
table entered at a gate of seconds that reads the branch's class —
compiles under the tree's own binary on 2026-09-08 and lists as
`classify, implement, review, ship, verify`, cost `low`. The
constitutional lint admits it unchanged: review is unavoidable on every
path to `ship`.

Alternatives weighed:

- **Keep the label and count it.** Rejected: the count was taken, and it
  says the rule needs work — ruling 6 of 0038 in its own words.
- **A gate tier for "no run, a preflight over the head".** Rejected. The
  gate's identity is the ship anchor with its patch map (0038 ruling 1);
  a preflight anchors a judgment, not a landing, and has no seat to
  answer a finding. A tier that accepts it would ship unremediated
  findings with a green check.
- **Commission `fast` with "the branch is done; change nothing".**
  Rejected: an implement seat is spent reading, the journal records an
  implementation that did not happen, and the commission lies about who
  authored the work.
- **Skip review for prose.** Rejected. Review unavoidable on the path to
  ship is constitutional (`assert_phase_unavoidable`), and 0038's own
  docs tier demands a judge's read of a docs delta. What prose may skip
  is the build, and decision 0039 already ruled why.
- **Light the landing from the gate itself, on every unvouched pull
  request.** Deferred, and named. A run spends the operator's
  credentials and money on the operator's machine; the platform gate has
  no seats. The operator lights a landing today; a landing lit by the
  gate is a later ruling with its own evidence.

## Rulings

1. **`recipes/landing` is `fast` entered at a classify gate.** The
   recipe composes `fast` by `extends` and changes one word of the
   table, `initial`, adding one seat and two rules in front of `fast`'s:
   `classify`, a boxed exec gate of seconds with results `docs` and
   `code`, reads every path the branch changes against the default
   branch against the repository's own docs class,
   `.github/delivery-classes.json` (0038 ruling 3), and answers `docs`
   only when every path matches — a missing base, a missing class file,
   an empty diff or one code path is `code`. `CLASSIFY-DOCS` routes to
   `review`; `CLASSIFY-CODE` routes to `verify`. `implement` is the
   remediation seat, entered only by `VERIFY-FAIL` or `REVIEW-REFORGE`,
   bounded as `fast` bounds them; `fast`'s implementer charter already
   answers `returned_from`. Every other rule, seat and limit is `fast`'s.

   **Enforcement binding:** `recipes/landing` — `bundle.json`,
   `policy.json`, `scripts/classify-seat.sh`, `README.md`;
   `crates/brokkr-runtime/tests/landing_shape.rs` — the initial phase,
   the phase and rule identity with `fast`, the two classify arms, the
   remediation edges, and the classify script driven over a real
   repository: a docs-only branch answers `docs`, one code path answers
   `code`, a repository without a class file answers `code`; the
   tree-wide compile in `witness_digests.rs`; the recipe table test in
   `crates/brokkr-cli/tests/contributing.rs`.

2. **A branch written by hand is landed by a run.** A pull request whose
   branch was authored outside a run names the landing run that judged
   and shipped it, as `Brokkr-Run: <run id>`, and the gate reads it as
   it reads any run: the shipped anchor's patch map equals the head's,
   tier `vouched`. Decision 0033 ruling 1 is read as *judged and shipped
   by a run*, whoever typed the commits; its text is unchanged. The gate
   script is untouched.

   **Enforcement binding:** `CONTRIBUTING.md` — the opening paragraph,
   the recipe table's row, the command in step 4 and the label sentence
   in step 5; `.github/pull_request_template.md`'s comment beside the
   `Brokkr-Run` line; the section "The landing" in
   `docs/guides/contributing-by-hand.md`; the row in
   `docs/guides/recipe-authoring.md`; `crates/brokkr-cli/tests/contributing.rs`
   pins the table, the template line and the gate's bindings as before.

3. **Prose is judged, not built.** A docs-only landing enters at
   `review`: decision 0039's reasoning, applied at the front of a run
   instead of on a return. The platform suite on the pull request
   remains the backstop for the tests that read prose — the decision
   index, the rename guard, the guide tables, the diagrams — and a
   branch that mixes one code path into its prose is code, whole.

   **Enforcement binding:** `CLASSIFY-DOCS` and the classify script,
   pinned by ruling 1's test; the docs class stays the repository's
   file, never a pattern in the recipe.

4. **A judgment that could not land is relit as a landing.** A run
   stopped at the reforging bound under the operator's ruling, or a
   judging pass over a branch, is followed by a landing on the ruled
   head; the findings and the ruling are the feature text, and the
   landing's anchor is the vouch. Judgment-guidance: the recipe's README
   says how, and nothing refuses the older path.

5. **The label is the residue, and it is read back.** The operator
   applies `by-hand` only where no landing can stand — the gate's own
   bootstrap, a platform outage, a run the engine cannot resume — and
   never on a tier the gate would have ruled `vouched`. Decision 0038
   ruling 6's log line remains the count. Judgment-guidance, and it says
   so; Muninn may read the count, under its own decision.

## Consequences

- **What moves.** One recipe (four files), one test, four sentences and
  a row in `CONTRIBUTING.md`, a comment in the pull request template, a
  section in the by-hand guide, a row in the recipe-authoring guide, and
  this decision. No engine change, no contract change, no adapter
  change. No witness pin: the recipe is composed, the tree-wide compile
  covers it, and `standby` set the precedent. The sweep test proposed by
  decision 0050 will pin one more table — `fast`'s 46 valuations and
  four unruled, plus the two classify valuations — when both land.
- **What it costs.** A docs landing is one review seat. A code landing
  is a verify's minutes and one review seat. Remediation is spent only
  on a finding, and never more than twice. Against forty labelled pull
  requests in five days, roughly the price of one delivery run.
- **What stays.** Decision 0033 ruling 5: the label exists and is the
  operator's. Decision 0038: the tiers, the patch map, the gate script
  byte for byte. `preflight`: the optional check for a contributor who
  wants to know before proposing.
- **Named, not ruled.** A landing lit by the gate on an unvouched pull
  request; a weekly count of labels in Muninn's reading; a `classify`
  that reads the branch's base from the realm's map instead of from the
  checkout's `main`.
