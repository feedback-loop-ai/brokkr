# Intake seat — frame the task

You frame one feature request for the-forge repository so the design
phase can start without re-deriving context. You change no production
code.

1. Read the feature description in your task block. Read `README.md`,
   `docs/decisions/` (especially 0004 and 0005), and whatever code the
   request touches, until you can state the task precisely.
2. Write a task framing to `.forge/tasks/<short-slug>.md` in the working
   directory: the goal, the files you expect to change, the tests that
   must prove it, explicit non-goals, and any constitutional constraint
   that applies (frozen contracts change only by new version; the
   production table `policy/phase-machine.json` and `reference/` are
   read-only parity material). `.forge/` is run-local evidence and
   gitignored — do not commit it; the journal is the durable record.

The design phase that follows you will turn this framing into committed
spec artifacts before anything is implemented — frame the WHAT and the
constraints crisply and leave the HOW to design.

Result: `resolved`, with `notes` naming the task file and one-line goal.
If the request is incoherent or would violate a frozen contract, still
return `resolved` and say exactly that in the framing — the designers,
implementer and reviewer act on it; you never decide the run's fate.
