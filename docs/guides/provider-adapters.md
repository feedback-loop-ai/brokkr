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
ok       boundaries: namespace (bubblewrap 0.11.0) · harness · open offered; seatbelt built by slice (ii) of decision 0046 ruling 6 (sandbox-exec not on PATH); container built by slice (iii) (docker found)
```

The `boundaries` line (decision
[0046](../decisions/0046-the-boundary-is-named.md) ruling 2) names what
this machine can put between a boxed seat's hands and itself. A boundary
is **offered** when `run`, `resume` and `rerun` would start a bundle under
it here: `namespace` needs bubblewrap on `PATH` (0.10 or newer for an
overlay bind), and `harness` and `open` are offered everywhere because
they need nothing of Brokkr's. A boundary is **ready** only once the tree
has built it, and the line says which slice does: `seatbelt` (slice ii)
and `container` (slice iii) are named by the decision and refuse at
start today whether or not `sandbox-exec`, `docker` or `podman` is
found — the tool's presence is reported so the operator knows what the
slice will find, not as an offer. The realm's word is judged against
this line before any journal row is written, naming the seats that
declare hands and the tool or slice they wait on. `brokkr doctor
--bundle <dir>` compiles in the discovered realm, so its `hands` line
judges the bundle's sites against the realm's boundary, not against a
default.

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

The `workspace` fragment is what a site with hands runs under the
`namespace` boundary, where Brokkr builds the box. Which boundary stands
is the realm's fact, not the bundle's (decision
[0046](../decisions/0046-the-boundary-is-named.md) ruling 1): a
`"boundary"` word beside `house` and `dialect` in `realms.json` under
`forge.realms/v4`, absent reading `namespace`. Under `harness` Brokkr
builds nothing and the harness's own sandbox is the only wall, so the
adapter's second answer is `hands.harness`: how that sandbox is addressed
from argv.

### `hands.harness`

```json
"hands": {
  "workspace": ["--sandbox", "read-only", "-c", "mcp_servers.brokkr.command=\"{brokkr}\"", …],
  "harness": {
    "gate": ["--sandbox", "read-only", "--output-last-message", "{result_path}"],
    "work": ["--sandbox", "workspace-write"],
    "result": "last-message"
  }
}
```

Three members, nothing else admitted:

| Member | Meaning |
|---|---|
| `gate` | The argv fragment that puts a gate-class seat in the harness's read-only class. A model gate is admitted under `harness` only when **every** link of its resolved chain declares one (decision 0046 ruling 4); under `open` a model gate is refused outright. |
| `work` | The fragment for a work-class seat with hands — the class that writes. A work seat under `harness` needs it on every link too; under `open` the same seat runs at the harness's own default, and whether that default writes is the harness's fact. |
| `result` | How the gate's verdict reaches the engine: `file` (default — the seat writes the result file itself) or `last-message` (the harness's own capture writes its final message to the result path). |

`gate` and `work` follow the three-shape convention `tool_permissions`
already uses: an argv fragment declares the member, `{"unsupported":
"<measured reason>"}` declares that the harness has no such class and
says why, and an absent member reads unsupported with no reason —
fail-closed, so a provider that has not been measured admits nothing by
silence. A fragment may carry two tokens the engine expands at spawn,
`{result_path}` and `{brokkr}`. The two workspace tokens,
`{hands_mcp_json}` and `{hands_args_toml}`, are refused in a harness
fragment by name: no workspace tool is served under `harness`, so a
fragment naming one would run with the literal token in its argv. A
`hands` object as a whole may still be `{"unsupported": …}`, and then
neither boundary compiles a site with hands against it.

**codex** declares both members and the `last-message` door, above.
`--sandbox read-only` is ruling 4's own word for codex and
`--sandbox workspace-write` is the tool's documented writable class;
`--output-last-message` is the flag `codex exec --help` documents and
the driver already admits on a resume. One fact is declared from the
tool's record rather than measured: whether the capture lands while the
sandbox class is `read-only`. That measurement is **pending and the
operator's** — one gate seat under the fragment, delivering its result
and nothing else; if the capture does not land, `gate` becomes
`{"unsupported": "<that reason>"}` and nothing false enters the record
either way, because a failed door is a missing result, loud.

**claude** declares **no** `hands.harness` member yet. The measurement
is the operator's, against the installed 2.1.x line (the transcript
above records 2.1.251), because the implementing seat's tool grant is
`cargo` and `git` and `claude` is not a command it may run. Until it is
recorded every shipped bundle whose hands agent's chain reaches claude
— every hands agent chains `opus` — refuses under `harness` naming
`claude`, the member and the site; the record of which bundles those
are is the pin test in
`crates/brokkr-runtime/src/bundle/model_policy_tests.rs`. What is known
going in: claude's `--permission-mode` choices are `acceptEdits`,
`auto`, `bypassPermissions`, `manual`, `dontAsk` and `plan` — there is
no `read-only` value. Candidates for `gate`: `--permission-mode
dontAsk` with `--allowedTools` naming the read tools and one edit rule
scoped to `{result_path}` (`result` `file`); `--permission-mode plan`
if it can still write the result file; and the `--restricted` /
`--permission-prompts none` pair reported on 2.1.263, unconfirmed.
Candidates for `work`: `--permission-mode acceptEdits` with the shell
allowed, or the harness's own sandbox settings with the shell
auto-allowed when sandboxed — a bare `acceptEdits` prompts for every
shell call, and a non-interactive seat answers a prompt with a denial,
which is why the empty fragment is an answer only if the measurement
shows the driver's own mode grants the shell. The recipe: under each
candidate, run one gate seat whose prompt asks it to read a file
outside the worktree, write one inside it and deliver its result — the
mode passes as `gate` when only the result file lands; run one work
seat that must run `cargo test` and commit — the mode passes as `work`
when both succeed with no prompt. When a member is measured, this
paragraph records the claude version it was measured against and what
the mode denies and allows; a member no combination satisfies is
declared `{"unsupported": "<measured reason>"}`, and the bundles that
seat claude keep refusing under `harness` by name.

**dsh** and **lanetally** declare no `hands.harness`, and a `harness`
gate reaching either refuses naming the link, the provider and
`hands.harness.gate` — exactly as a boxed gate on them refuses today.
Of claude, codex and dsh, then, a `harness` gate stands on codex today
and on claude once measured. A dsh work seat with hands may run under
`open` at its harness's default; its gate refuses under `namespace`
for the untrusted tier and under `harness` for the missing gate fragment.
