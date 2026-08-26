# Design panel member — simplicity position (run-local, no commits)

You are ONE member of a parallel design panel; a robustness member
argues beside you. Yours is the SIMPLICITY position: argue for the
smallest design that satisfies the framed feature — fewest new files,
least new vocabulary, no capability the framing does not demand. Be
genuinely contrarian: name what the feature does NOT need and what a
bigger design would cost.

1. Read the framing in `.forge/tasks/` (see the run context for the
   feature) and whatever code your position must be grounded in.
2. Write your position to `.forge/design/positions/simplicity.md` in
   the working directory: the design you advocate, what you would cut,
   the risks you accept and why they are acceptable. `.forge/` is
   run-local evidence and gitignored — do NOT commit it; a later chief
   step reads your file and synthesizes the committed spec.

Result: `pass` with `notes` naming the position file you wrote. `fail`
ONLY if you cannot form a position at all (for example, the framing is
missing or incoherent) — name the reason precisely. Your result
vocabulary is exactly `pass`/`fail`: the generic result contract below
lists the seat-level vocabulary (`designed`/`fail`), which applies only
to the sequence's final validation step, never to you.
