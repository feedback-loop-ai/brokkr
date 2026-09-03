# 0038 — Evidence follows content: the vouch binds to the patch, and a docs delta is re-judged, not re-forged

Status: accepted (operator ruled in chat, 2026-09-03)
Date: 2026-09-03

## Context

Decision 0033 binds a pull request to a completed run by one fact: the
anchor's `repo_head` equals the pull request head. That is the right
fact for a branch that never moves. On 2026-09-03 it produced this
sequence for one docs-only slice ([#153](https://github.com/feedback-loop-ai/brokkr/pull/153)):

| step | what happened | what the machine knew |
|---|---|---|
| run `…-ae760931` ships | intake, implement, verify, review (one fix), ship — head `88e7db1` | judged and vouched |
| main lands #150, #152 | two decisions append rows to `docs/decisions/README.md` | nothing changed in the slice |
| rebase 1 | one conflict, the index table; resolved by keeping every row | the vouch is void: `repo_head` ≠ head |
| run `…-b6aa241f` lit | a second `ember` run to re-vouch the same content | judging again what it judged an hour ago |
| operator rules by-hand | the second run is concluded at intake; label applied; PR closed and reopened for a fresh event | the gate skipped |
| main lands #154 | 0035 and 0036 move to accepted, same two rows | nothing changed in the slice |
| rebase 2 | the same conflict again; the resolution once shipped conflict markers | the gate skipped |

Three facts fall out of the table. The judgment the machine made was
true for the whole hour: not one hunk of the slice's own diff changed
across two rebases. The gate could not tell, because it holds a commit
id, and a rebase renews the id without touching the content. And the
escape hatch, meant for the exceptional case, was used for the ordinary
one, which is how a rule erodes without anyone deciding to weaken it.

The operator asked for tiering: a doc changed after judgment is not a
reason to forge again. Four shapes were weighed.

- **Lighter rules for docs slices as a class.** Rejected. The `ember`
  run on this very slice found a crate edge the diagram omitted, which
  `tests/diagrams.rs` could not have caught because it checks drawn ⊆
  declared. A docs slice still earns its first judgment; what it should
  not pay twice for is a judgment that is still true.
- **The operator's approving review as the docs-tier ruling.** Not
  possible: GitHub refuses an approval from the pull request's author,
  and every pull request here is opened from the operator's account. A
  machine identity that opens pull requests would make the operator the
  reviewer; that is the direction, and a slice of its own.
- **The merge as the ruling.** Rejected by the operator: it collapses
  the judgment into the click, and a tier with no judgment is not a
  tier.
- **Re-judge the delta at the cost of the delta.** Accepted below. A
  vouch that follows content survives a clean rebase for free; a delta
  confined to prose is judged by `preflight` — verify and a read-only
  review over the head as it stands, no implement, no ship — which is
  the cheapest run that still rules; anything else is a new delivery.

## Rulings

1. **The ship anchor carries the slice's patch identity, per file.** A
   new anchor is `forge.journal-anchor/v3`. Beside `repo_head` it
   records `base`, the merge-base of the vouched head with the
   repository's default branch at conclusion, and `patch`, a map from
   every path the slice touches to the stable patch id of that file's
   diff from `base` to `repo_head`; an added or deleted file is a path
   like any other. The anchor still names the head: the patch map says
   what was judged, the head says where.

   **Enforcement binding:** `brokkr-runtime::anchor` writes v3;
   `brokkr anchor --check` and the offline verifier read v2 and v3, and
   refuse an unknown version. The id is `git patch-id --verbatim`:
   per-file and order-independent like `--stable`, with whitespace kept,
   because a space is semantic in shell, YAML and Python and `--stable`
   alone strips it — a re-indented hunk must not keep the vouch. A
   workspace test pins the map against `git diff` of a fixture
   repository, and the gate test pins that one added space cuts the tier.

2. **A head whose patch equals the vouched patch is vouched.** The gate
   computes the pull request head's per-file patch map against its
   merge-base with the pull request base and accepts the run when the
   two maps are equal, whatever `repo_head` says. A rebase that changes
   ancestry and nothing else costs nothing.

   **Enforcement binding:** the `delivered by brokkr` job compares the
   maps before it compares heads; a test drives the same comparison
   over two fixture branches carrying one patch with different parents.

3. **The tier is cut by the delta since the judgment, not by the
   slice's nature.** The delta is the set of paths whose patch id
   differs between the vouched map and the head's map, added and
   removed paths included. Three tiers, decided in order:

   - *empty delta* — vouched by ruling 2;
   - *docs delta* — every path in the delta matches the repository's
     docs class; the pull request then also names a completed
     `preflight` run whose anchor's `repo_head` equals the pull request
     head and whose last ruling is `REVIEW-CLEAN` or
     `REVIEW-RESIDUAL-OK`, declared as `Brokkr-Preflight: <run id>`;
   - *code delta* — anything else; the pull request names a new
     shipping run that vouches for the head exactly as 0033 ruling 1
     reads today, or carries the operator's `by-hand` label.

   The docs class is repository-owned data, not a pattern in a
   workflow: `.github/delivery-classes.json` lists the pathspecs
   (`**/*.md`, `docs/**`, `assets/**`), and a path that matches nothing
   is code.

   **Enforcement binding:** the gate reads the class file and applies
   the tiers; the preflight branch checks the second journal offline
   with the same verifier, requires `status == completed`, a last
   `transition/decided` with `from == "review"` and `next == "done"`,
   and the anchor's `repo_head` equal to the head. `recipes/preflight`
   already stops on `fixes_applied`, so a preflight cannot have altered
   what it judged. A test pins the class file's shape and the tier
   order.

4. **The gate answers the label without a ceremony.** The workflow
   subscribes to `labeled` and `unlabeled` beside `opened`,
   `synchronize` and `reopened`, so applying `by-hand` re-runs the gate
   on the same head and event payload.

   **Enforcement binding:** `on.pull_request.types` in
   `.github/workflows/ci.yml`; `tests/contributing.rs` asserts the
   list.

5. **The decision index is derived, and appending to it is not a
   conflict.** A test asserts that the table in
   `docs/decisions/README.md` is exactly the rendering of every decision
   file's number, title and `Status:` line, in number order. With that
   check standing, `.gitattributes` marks the file `merge=union`, so two
   branches appending rows merge without a conflict, and a duplicated or
   stale row fails the test instead of blocking the merge or, as
   happened today, shipping markers.

   **Enforcement binding:** `crates/brokkr-cli/tests/decisions_index.rs`
   and the `.gitattributes` line. The union driver only concatenates:
   when both sides edit the same row, the test names the duplicate and
   a one-line fix follows, which is the honest cost of never conflicting
   on an append.

6. **The escape hatch stays the operator's and becomes countable.** The
   `by-hand` label skips the gate at every tier, as 0033 ruling 5
   holds. The gate now logs the tier it would have applied, so the
   label's use on an empty or docs delta is visible in the run log and
   can be read back as a rule that needs work rather than a rule that
   was bypassed.

   **Enforcement binding:** the gate's log line; judgment-guidance
   beyond that, and it says so.

## Consequences

- 0033 rulings 3 and 4 are amended by rulings 1 to 3 above: the anchor
  gains a version, and the gate compares patches before heads. 0033
  rulings 1, 2, 5 and 6 stand.
- Today's sequence under these rulings: rebase 1 changed one file in
  the delta, `docs/decisions/README.md`, a docs path — a `preflight` run
  over the rebased head, roughly a third of the `ember` run's cost,
  would have re-vouched it; rebase 2 would have produced no conflict at
  all under ruling 5; and no pull request would have been closed and
  reopened.
- `Brokkr-Preflight` is a second declaration, not a replacement: the
  shipping run remains the evidence that the slice was delivered, and
  the preflight is the evidence that the head as merged was judged.
- A machine identity that opens pull requests from the ship seat, so
  the operator reviews rather than authors, is the next step this
  decision points at and does not take.
- The pull request that lands this decision cannot pass the gate it
  introduces: the gate runs the script from the trusted base branch,
  which does not carry it yet. It lands under the operator's `by-hand`
  label, exactly as 0033's own landing did (0033 ruling 6), and every
  pull request after it is judged by the new gate.
- The `patch` map makes an anchor's size proportional to the slice's
  file count, which is bounded by what a run may commit; a slice
  touching hundreds of files was already the wrong slice.
