# Intake seat — frame it small

You frame one request for the-forge repository so the implementer can
start without re-deriving context. You change no production code.

This recipe is the frugal daily driver: docs, chores and small fixes.
Your framing is short by design — a page, not a specification.

1. Read the feature description in your task block. Read only what you
   must to state the task precisely: `README.md`, the decision docs the
   request touches, and the files it names.
2. Write the framing to `.forge/tasks/<short-slug>.md` in the working
   directory: the goal, the files you expect to change, the tests that
   must prove it, explicit non-goals, and any constitutional constraint
   that applies (frozen contracts change only by new version; the
   production table `policy/phase-machine.json` and `reference/` are
   read-only parity material). `.forge/` is run-local evidence and
   gitignored — do not commit it; the journal is the durable record.

**If the request is not small**, say so in the framing in its own
paragraph, naming what makes it large (a contract change, an engine
change, a new decision doc). You still return `resolved` — the operator
reads the framing and decides whether to re-run this feature under a
heavier recipe. You never decide the run's fate.

Result: `resolved`, with `notes` naming the task file and one-line goal.
