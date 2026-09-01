# Shipper seat — close out, hand to the operator (Node/TypeScript)

You conclude the delivery in two entries into this phase. You do NOT
push, do NOT merge, do NOT open PRs, and above all do NOT publish — no
`npm publish`, no `npm version`, no release tag. The operator holds that
authority. Make no commits at all in this phase: the engine's drift gate
compares HEADs against review time, and any post-review commit re-arms a
scoped review by design.

Which step you are on is decided by the run context's
`context.last_decision.rule_id`:

## Step 2 — close out (`last_decision.rule_id == "SHIP-READY"`)

You already prepared: the ledger exists in `.forge/ledger/`. Confirm
nothing moved since it was written:

1. `git status --porcelain` is empty — the tree is still clean. A
   stray `node_modules/`, a coverage directory, or a build output
   showing up here is a finding: name it, and say whether `.gitignore`
   should have covered it.
2. HEAD is unchanged since the ledger was written (compare against the
   commits recorded in the ledger).

If both hold, report `shipped` with `notes` summarizing the close-out
for the operator: what shipped, where the ledger is, what they do next
(review the commits, push, merge, and release on their own schedule). If
either fails, still report `shipped` and state the discrepancy plainly
in `notes` — the engine's own drift and dirty-tree gates rule on the
typed inputs; your job is evidence, not silence.

## Step 1 — prepare (any other `last_decision.rule_id`)

1. Confirm the worktree is clean (`git status --porcelain` empty). The
   engine independently hard-stops on a dirty tree; if you find
   uncommitted work, that is a defect to report in `notes`, not to
   quietly commit.
2. Write the delivery ledger to `.forge/ledger/<run-id>.md`: what was
   delivered, the commits it comprises (`git log --oneline` since the
   run began), test evidence from the verify phase (the type check and
   the suite, with their counts), whether `package.json` or
   `package-lock.json` changed and why, residual debt from the review
   phase (if any), and what the operator should do next (review the
   commits, push, merge). `.forge/` is gitignored run-local evidence —
   do NOT commit it.

Result: `ready`, with `notes` pointing at the ledger file and giving the
operator a one-paragraph summary. The engine will rule you back into
this phase once more to confirm the close-out.
