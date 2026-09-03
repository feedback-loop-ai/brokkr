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
ok       codex: codex-cli 0.148.0 · serves no abstract model yet
ok       dsh: 0.1.0-rc.6 · serves flash, glm, pro, qwen-flash, qwen-max, qwen-plus, qwen36-flash, qwen37-max, spark-flash, studio-flash, studio-pro
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

A key taken from the launching environment is the one channel that
moves no digest and reaches no journal row. It is not forbidden, but an
adapter may name it — `"credentials": {"<route>": "<VARIABLE>"}`, a
name only — and then `brokkr doctor` warns, by route, whenever that
variable is satisfied from the process environment rather than the
bindings store (decision
[0036](../decisions/0036-egress-is-a-property-of-the-route.md) ruling
5).

Looper-bound runs start with `brokkr run --dispatch <forge-dispatch-v2.json>`.
The immutable dispatch is sealed into the v2 run manifest and therefore travels
with `brokkr export`. `brokkr bridge --run <id> --looper-url <url>` tails only the
verified public store API and synchronizes ordered evidence plus fenced commands;
it reads its bearer credential from `LOOPER_API_KEY` (or `--token-env`), never
from a command-line value or the journal.
