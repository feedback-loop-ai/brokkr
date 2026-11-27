# research-dsh — the research intake on the dsh lane

This overlay of `recipes/research` seats the `researcher` office inline on
dsh. The hire is `dashscope/qwen3.8-max` at effort `xhigh`, forwarded
through dsh's settings layer since decision 0035's addendum and echoed
back in the record. The overlay inherits the registry gate, ten-entry
cap, and outcomes that propose entries for the operator (decision 0044).

```
brokkr run --recipe research-dsh --repo . --feature "$(cat recipes/research/commissions/weekly-sweep.md)"
```

The weekly workflow runs this lane; the operator's `research-intake`
skill runs the Claude Code lane.

## Why the seat is inline

dsh cannot express the library agent's tool allow-list. An agent reference
also owns its driver, so the dsh seat remains inline with `forced_crew`
and a local policy overlay that names its work phase `researcher`.
That label is the inline office the realm grant reaches.

The seat asks for `web-fetch` and `web-search` as `wants`. This repository's
realm grants both to the `researcher` office through `dsh-native-fetch` and
`dsh-native-search` (decision 0126 ruling 6). The driver stages both native
controls. Fetch sends the requested URL to the destination from the
operator's host. Search sends the query to DeepSeek's search API using
`DEEPSEEK_API_KEY`.

The research-dsh seat can reach the web through the shell without any
grant. The operator said “accept it as known risk for now” (#1329).

`drivers/research-web.yml` carries the Model Studio route alone, keyed by
`DASHSCOPE_API_KEY`. Its path is read from the repository root, where the
sweep runs, and its bytes are in the bundle digest.

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
