# research-dsh — the research intake on the dsh lane

`recipes/research` with one seat replaced: the researcher runs on dsh
(`dashscope/qwen3.8-max`, the hire the operator ruled for this office,
effort `high`, which dsh records as not reported) instead of the
library's Claude Code office. Everything else is
inherited: the boxed registry gate, the ten-entry cap, the table that
proposes and never ships (decision 0044).

```
brokkr run --recipe research-dsh --repo . --feature "$(cat recipes/research/commissions/weekly-sweep.md)"
```

The weekly workflow runs this lane; the operator's `research-intake`
skill runs the Claude Code lane.

## Why the seat is inline

dsh expresses no tool allow-list (`adapters/dsh.json` declares
`tool_permissions` unsupported), so the library office's `webfetch` and
`websearch` grants cannot be compiled onto it. The seat is therefore an
inline dsh site, as night-shift's implementer is, and the fetch grant
takes the shape dsh has: `drivers/research-web.yml`, one overlay that
turns the headless profile's page fetch on (search is on by default)
and states the Model Studio route the default profile lacks, keyed
from `DASHSCOPE_API_KEY`; search keeps `DEEPSEEK_API_KEY`.
The overlay path is read from the repository root, where the sweep
runs, and the file is inside the bundle, so it is in the digest.

`roles/researcher.md` is the library charter's bytes, held equal by the
roster test: the configurable prompt stays one text, edited in
`agents/charters/researcher.md` and copied here.
