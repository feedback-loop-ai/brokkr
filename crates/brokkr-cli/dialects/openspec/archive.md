# OpenSpec archive

Folding a change is the last task: run the archive operation named above,
which moves the change directory under `openspec/changes/archive/` and
folds its deltas into `openspec/specs/`. Folding also records which change
wrote which capability, so every capability the change touched carries a
`## Provenance` list.

After the archive operation, open each capability's
`openspec/specs/<capability>/spec.md` that the change touched and append
exactly one line under a `## Provenance` heading held at the end of the
file:

`- \`<archived-directory-name>\` — folded <YYYY-MM-DD>`

`<archived-directory-name>` is the directory the archive operation
created under `openspec/changes/archive/`; `<YYYY-MM-DD>` is the day it
was folded. Append only: never rewrite, reorder or remove an existing
provenance line, because standing truth accumulates and is not edited into
a different meaning. The line is a pointer, not a summary — it names the
change and nothing else, and the why stays in the change directory.
