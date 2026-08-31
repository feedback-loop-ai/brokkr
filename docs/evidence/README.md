# Evidence

Redacted exports of journals cited in the project's essays and decisions —
produced by `brokkr export --redact`, which this repository built for itself
([PR #85](https://github.com/feedback-loop-ai/brokkr/pull/85), the wager's
synthesis). Each `.redacted.ndjson` pairs with a manifest that marks the
derivative and names the rule: recorded hashes verify only on the verbatim
export; these copies are evidence of shape, never of authorship.

| Journal | What it witnesses |
|---|---|
| `verify-two-delivered-slices-slic-ffc01c67` | The verify run whose review found a stored XSS in agent-written UI code and stopped the ship (`REVIEW-RESIDUAL-SECURITY`). |
| `implement-decision-0022-reforgin-54a88e9b` | The run that implemented the reforging law — and was stopped by the severity-blind rule it retires: the last guillotine. Its review's medium finding was answered before merge ([PR #87](https://github.com/feedback-loop-ai/brokkr/pull/87)). |
| `brokkr-export-gains-redact-a-san-3d19f5c1` | The wager, crew A (claude, `recipes/fast`): the same commission as crew B, judged by artifacts. |
| `brokkr-export-gains-redact-a-san-c5d011df` | The wager, crew B (codex, `recipes/fast-codex`, parity rigging). The landed feature is the judged synthesis of both. |

One honest wrinkle, visible in the evidence itself: two review notes quote the
`file://` carve-out they were flagging, so the generic string `/home/` survives
inside prose *about* the declared bound — with every identity already a
placeholder. The redaction's manifest declares exactly this limit; the evidence
demonstrates its own disclosure.
