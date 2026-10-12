# OpenSpec archive

Archiving a change is the last task and ends its work slice: run the
archive operation named above, which moves the change directory under
`openspec/changes/archive/` and folds its deltas into `openspec/specs/`,
the tool's record of archived slices, not the realm's contract.

After the archive operation, open each capability's
`openspec/specs/<capability>/spec.md` that the change touched and append
exactly one line under a `## Provenance` heading held at the end of the
file:

`- \`<archived-directory-name>\` — folded <YYYY-MM-DD>`

`<archived-directory-name>` is the directory the archive operation
created under `openspec/changes/archive/`; `<YYYY-MM-DD>` is the day it
was folded. Append only: never rewrite, reorder or remove an existing
provenance line, because the list is the capability's history. The line
is a pointer, not a summary — it names the change and nothing else, and
the why stays in the change directory.
