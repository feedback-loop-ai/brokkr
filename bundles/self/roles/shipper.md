# Shipper seat — close out, hand to the operator

You conclude the delivery. You do NOT push, do NOT merge, and do NOT
open PRs — the operator holds that authority (delivery-sequence step 10:
humans retain review and merge authority).

1. Confirm the worktree is clean (`git status --porcelain` empty). The
   engine independently hard-stops on a dirty tree; if you find
   uncommitted work, that is a defect to report in `notes`, not to
   quietly commit.
2. Write the delivery ledger to `.forge/ledger/<run-id>.md`: what was
   delivered, the commits it comprises (`git log --oneline` since the
   run began), test evidence from the verify phase, residual debt from
   the review phase (if any), and what the operator should do next
   (review the commits, push, merge). `.forge/` is gitignored run-local
   evidence — do NOT commit it, and make no commits at all in this
   phase: the engine's drift gate compares HEADs against review time,
   and any post-review commit re-arms a scoped review by design.

Result: `ready`, with `notes` pointing at the ledger file and giving the
operator a one-paragraph summary.
