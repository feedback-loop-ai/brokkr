# Provider adapters

A provider adapter is **data**, one file per provider in `adapters/`:
the driver invocation, the abstract→concrete model mapping, how tool
permissions and MCP servers are expressed, and — the load-bearing part —
which of those the provider **cannot** express, written as the explicit
string `"unsupported"` rather than left to be inferred from an empty
map. Adding a provider or a model is a file edit, not a release.

`brokkr doctor` reports what each adapter can actually reach on this
machine, and refuses to guess about the rest:

```
$ brokkr doctor
ok       claude: 2.1.251 (Claude Code) · serves fable, haiku, opus, sonnet
ok       codex: codex-cli 0.153.2 · serves astra, luna, sol, terra
ok       dsh: 0.1.2-rc.1 · serves flash, glm, muse, muse-contributor, pro, qwen-flash, qwen-max, qwen-plus, qwen36-flash, qwen37-max, spark-flash, studio-flash, studio-pro
warn     lanetally: binary 'claude-lanetally' not found — seats resolving to this provider will fail to spawn …
```

The built-in adapters are reachable directly as
`brokkr driver <claude|lanetally|codex|dsh|exec> -- <extra args>`, which
is exactly how a bundle names them.

The `dsh` adapter reaches two provider routes through one grammar. A
bare id (`deepseek-v4-flash`, `deepseek-v4-pro`) is DeepSeek's own API,
keyed by `DEEPSEEK_API_KEY` in the engine's launching environment; a
`dashscope/<id>` lane is Model Studio's Token Plan catalogue
(`deepseek-v4-flash-0731`, `qwen3.8-max`, `glm-5.2`, …), keyed by
`DASHSCOPE_API_KEY`, and needs that provider route declared in the dsh
headless profile's own patch layer (`$DSH_HOME/profiles/headless/cordis.patch.yml`).
The driver turns `--model <lane>` into the one-seat overlay dsh's
launcher reads; neither key ever enters argv, the recipe, or the
journal.

The same adapter reaches Muse Spark 1.3 through OpenRouter
(`https://openrouter.ai/api/v1`, OpenAI-compatible, Meta as the sole
upstream at Meta's own prices) as two routes that share one endpoint
and one key name, `OPENROUTER_API_KEY`: `meta/meta/muse-spark-1.3` and
`meta-contributor/meta/muse-spark-1.3-contributor`. They are two
routes because the model id is the terms — the bare id is not used to
improve Meta's products, the contributor id is, at roughly a twelfth
of the price — and egress is a property of the route (decision 0036),
so the two must be tellable apart by prefix. The route is the first
segment only; the second `meta/` is OpenRouter's name for the model.
Neither is ruled `contracted`; both are `uncontracted` by silence.
Muse Spark always reasons, so the adapter's `efforts` carries `xhigh`
for it and the profile row declares no `off` level. A seat's `--effort`
reaches the wire: the driver writes it into a settings document of the
seat's own, which dsh reads over its composition and over the route's
`reasoning:` default (decision 0035 addendum), and the request header
echoes the level back into the record. The reasoning itself comes back
encrypted, so the record carries its signature and no text.

A key taken from the launching environment is the one channel that
moves no digest and reaches no journal row. It is not forbidden, but an
adapter may name it — `"credentials": {"<route>": "<VARIABLE>"}`, a
name only — and then `brokkr doctor` warns, by route, whenever that
variable is satisfied from the process environment rather than bound by
a seat (decision
[0036](../decisions/0036-egress-is-a-property-of-the-route.md) ruling
5). `brokkr doctor --bundle <dir>` asks whether any seat of that bundle
declares the name in its `secrets`, because a name sitting in the
bindings store that no seat binds is handed to no driver and the
launching shell's copy is what the provider reads. Store membership is
still necessary — a name a seat declares and the store cannot answer for
is bound to nothing either, and doctor says which of the two halves is
the one missing. Without a bundle to inspect it falls back to store
membership alone and says so in the line, naming whether you passed no
bundle or one that would not compile (decision
[0040](../decisions/0040-the-flag-is-always-read.md) ruling 4).

The route names in `routes` and `credentials` are whatever a concrete
model id may begin with — ASCII letters of either case, digits, `-`,
`_`, `.` and `:`, never `/` — so `us.east` and `openai_compat` are
routes an operator can rule on (decision 0040 ruling 5). And the route a
seat's argv names is read on the flag the adapter declares in
`model_flag` AND on `--model`: the same string is one read, a concrete
pin on either names the route, and two pins naming different ids are
refused naming both flags (ruling 1). A short declared flag (`-m`)
carries its value attached in the getopt way, so `-mspark/x` is a pin
whose value is `spark/x`; a long one has only `--model x` and
`--model=x`, and `--model-fallback` is a different flag every reader
walks past (ruling 2).

Looper-bound runs start with `brokkr run --dispatch <forge-dispatch-v2.json>`.
The immutable dispatch is sealed into the v2 run manifest and therefore travels
with `brokkr export`. `brokkr bridge --run <id> --looper-url <url>` tails only the
verified public store API and synchronizes ordered evidence plus fenced commands;
it reads its bearer credential from `LOOPER_API_KEY` (or `--token-env`), never
from a command-line value or the journal.

## Hands

`hands` (decision 0043) is the adapter's answer to a site that boxes its
hands: the argv fragment that disables the harness's own tools and reaches
`brokkr hands serve` over MCP. Two tokens are expanded by the engine at
spawn — `{hands_mcp_json}`, a Claude-style MCP config naming this binary,
and `{hands_args_toml}`, the server's arguments as a TOML array for
`codex -c`. The codex fragment also sets
`mcp_servers.brokkr.default_tools_approval_mode="approve"`: since
codex-cli 0.153 an MCP tool call "requires approval" by default and a
non-interactive `codex exec` runs under an approval policy of `never`, so
without that key every workspace call — reads and the result write alike
— is refused with "MCP tool call requires approval, but approval policy
is never". The first two astra-judged gates died on exactly that
(2026-09-05), and `auto` does not lift it; `approve` does, measured
against `brokkr hands serve`. `{"unsupported": "<measured reason>"}` declares that the
harness cannot swap its tool surface, and a site with hands then refuses
to compile against it, exactly as an unexpressible tool list does.
