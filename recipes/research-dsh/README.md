# research-dsh — the research intake on the dsh lane

`recipes/research` with one seat replaced: the researcher runs on dsh
(`dashscope/qwen3.8-max`, the hire the operator ruled for this office,
effort `xhigh`, forwarded through dsh's settings layer since decision
0035's addendum and echoed back in the record) instead of the
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
takes the shape dsh has: the headless profile's own web tools. Since
dsh 0.1.2-rc.1 that profile ships `web-fetch-http` with page fetch on
(0.1.0-rc.6 shipped no fetch provider, and the first sweep read with
curl). `drivers/research-web.yml` therefore carries only the Model
Studio route the default profile lacks, keyed from `DASHSCOPE_API_KEY`;
search keeps `DEEPSEEK_API_KEY`.
The overlay path is read from the repository root, where the sweep
runs, and the file is inside the bundle, so it is in the digest.

## Why the effort is `xhigh`, and why the overlay names the levels

The lane was first pinned `high`. The pin stopped at the digest until
decision 0035's addendum (#200) carried it to dsh, so the route had
never been asked for a level; the first sweep that asked (workflow run
34115364530, 2026-09-07, Research issue #234) died in one second with
`UNSUPPORTED_REASONING_EFFORT: provider "dashscope" model "qwen3.8-max"
does not support reasoning effort "high"`. Two facts, measured on dsh
0.1.2-rc.1 and re-measured on 0.1.3-alpha.2 with the same result:

- A custom provider's model entry that declares no `reasoningEfforts`
  is resolved against dsh's installed catalog under the same provider
  key. No catalog ships for `dashscope`, so the entry materialised as a
  non-reasoning model and any effort at all was refused.
- dsh's bundled Qwen catalog lists Qwen3.8-Max with `low`, `medium` and
  `xhigh`; `high` and `max` are pinned unsupported. `high` can never be
  sent to this model, whatever the overlay says.

So the overlay states the table on the entry (`low`, `medium`, `xhigh`,
each carrying its own wire value; the undeclared levels stay refused),
and the seat pins `xhigh`, the nearest declared level to the `high`
the operator ruled. The level is the operator's to change: `low` or
`medium` are one edit to `bundle.json`, and the digest row in
`crates/brokkr-runtime/tests/witness_digests.rs` moves with it.

Evidence, 2026-09-08, the operator's session: a one-turn
`brokkr driver dsh -- --model dashscope/qwen3.8-max --effort xhigh
--patch recipes/research-dsh/drivers/research-web.yml` passed dsh's
effort validation, the request header echoed `xhigh`, and the turn then
failed on the provider with `QUOTA: 429 insufficient_quota`: the Model
Studio token-plan weekly quota is spent until 2026-09-12 14:37 UTC.
The weekly workflow cannot succeed before that reset, with or without
this fix.

`roles/researcher.md` is the library charter's bytes, held equal by the
roster test: the configurable prompt stays one text, edited in
`agents/charters/researcher.md` and copied here.
