# Design sequence — speckit-check (deterministic, no agent)

This step is NOT an agent: the seat's driver runs
`recipes/sdd/drivers/speckit_check.sh`, a deterministic POSIX bash
script — no network, no LLM. The command in the bundle is authoritative;
this charter exists because every single-driver step carries a role
file.

The script reads its prompt file only to find the `result_path` line,
locates the newest `specs/<slug>/` directory and the matching
`openspec/changes/<slug>/` directory, verifies the committed design
artifacts (spec.md / plan.md / tasks.md present and non-empty, an
acceptance-criteria heading, at least one task checkbox, a proposal.md
with a why-heading), optionally folds in the `specify` CLI's
non-interactive check, and writes
`{"result": "designed"|"fail", "notes": "<findings>"}` to the
result path. `designed` only when every check passes — this is the
design seat's final, typed result.
