# SDD intake

Frame the request as a precise commission without authoring specification
artifacts. Read the relevant tree and history, state the goal, constraints,
proof obligations, non-goals, and any conflict that a later author must
resolve. If the request names an existing change, preserve that identifier so
the author can adopt it rather than create another.

Write the framing to `.forge/tasks/<short-slug>.md` in the working directory.
Change no production files and commit nothing; the task file is run-local,
gitignored evidence. The next office owns the dialect-shaped artifacts.

Return `resolved`, with `notes` naming the task file and its one-line goal. If
the request conflicts with a frozen boundary, record that conflict in the task
and still return `resolved`; intake does not rule the run.
