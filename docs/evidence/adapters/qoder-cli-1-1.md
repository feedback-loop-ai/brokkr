# Qoder CLI 1.1: the headless turn, its refusals and its switches, measured by hand (#1178)

Issue #1178. This record backs the `qoder` adapter (`adapters/qoder.json`)
and `brokkr driver qoder`. `brokkr probe harness` knows only claude, codex
and dsh, so every figure here was measured by hand. The recorded stdout and
stderr of each turn sit beside this file in
[`qoder-cli-1-1/`](qoder-cli-1-1/), byte for byte as the CLI wrote them.

## Provenance

| Figure | Source |
| --- | --- |
| Every live turn below | controller, 2026-11-20, Linux workstation |
| The auth and config names | `strings` of the installed 1.1.92 binary, 2026-11-20 |
| The npm package | `npm install --prefix /var/tmp/probe-qoder/prefix @qoder-ai/qodercli@latest`, which gave 1.1.93; no turn was run on it |

No implementing seat can re-run these figures, because none has a signed-in
Qoder.

## Host and versions

| Item | Value |
| --- | --- |
| Host | Linux x86-64. macOS: pending |
| Qoder CLI | 1.1.92, a native ELF binary installed by Qoder's own installer under `~/.qoder/bin/qodercli/` |
| Account | a teams account, signed in through the browser (`qodercli status`: `Login Method: browser`, `Auth Source: local`) |
| Working directory | a scratch directory under `/var/tmp/probe-qoder/` for each turn |
| Model | `-m Fast` for every turn, except the unknown-model turn |

## The turns

Each turn ran `qodercli -p --output-format stream-json --no-session-persistence`
plus the flags named below, with stdin from `/dev/null` unless a prompt was
piped in.

| File | Flags beyond the common ones | Exit | What it shows |
| --- | --- | --- | --- |
| `tool-less-turn.ndjson` | `--tools ""`, prompt after `--` | 0 | init, `artifacts_update`, `hook_diagnostics`, one `assistant` text, `result/success` reading `READY` |
| `stdin-prompt-turn.ndjson` | `--tools "" --disallowed-tools mcp_call,mcp_get,mcp_list`, prompt on stdin | 0 | the prompt is read from stdin with no positional argument; init `tools` is `[]` |
| `tools-turn.ndjson` | `--permission-mode bypass_permissions --tools Write Bash` | 0 | `tool_use` and `tool_result` blocks for `Write` and `Bash`, the same block shape as Claude Code's |
| `denied-command-default.ndjson` | `--permission-mode default --tools Bash Read --allowed-tools "Bash(ls:*)"` | 0 | `ls -1` and `id -u` ran, and `touch probe2.txt` was refused; one entry in `result.permission_denials`; the file was not created |
| `denied-command-dont-ask.ndjson` | the same, with `--permission-mode dont_ask` | 0 | the same three outcomes, with `dont_ask`'s own refusal words |
| `named-model-turn.ndjson` | `--tools "" --disallowed-tools mcp_call,mcp_get,mcp_list -m Qwen4-Max --reasoning-effort high`, prompt on stdin | 0 | init `model` is `Qwen4-Max`, the name requested; `modelUsage` is keyed `qmodel_latest` |
| `comma-allow-list.ndjson` | `--permission-mode dont_ask --tools Bash --allowed-tools "Bash(ls:*),Bash(touch:*)"` | 0 | a comma-separated allow-list: `touch p3.txt` ran and created the file, `rm -f p3.txt.nope` was refused |
| `unknown-model-fallback.ndjson`, `.stderr` | `--tools ""`, `-m NoSuchModel` | 0 | init `model` is `Auto`; stderr says the model "is not available right now; using \"auto\" instead" |
| `signed-out.stderr` | `--config-dir` on an empty directory | 1 | stdout empty; stderr one ANSI-coloured line, `Not logged in · Please run /login` |
| `bad-effort.stderr` | `--reasoning-effort bogus` | 1 | stdout empty; stderr an "unexpected critical error" with a JavaScript stack, naming the valid efforts |

## Facts

- **The stream is Claude Code's shape.**
  - Init carries `tools`, `mcp_servers`, `model`, `permissionMode` (camelCase,
    such as `acceptEdits`), `apiKeySource`, `protocol_version` (`1.5.0`),
    `qodercli_version`, `session_id`, `agents`, `skills`, `plugins`,
    `capabilities`, `output_style` and `fast_mode_state`.
  - The result carries `subtype`, `is_error`, `result`, `num_turns`,
    `duration_ms`, `duration_api_ms`, `stop_reason`, `permission_denials`,
    `total_cost_usd`, `total_credits`, `modelUsage` and `session_id`.
  - The two system subtypes Claude Code does not write are `artifacts_update`
    and `hook_diagnostics`.
- **Cost is credits.** `total_cost_usd` is 0 and every token count in `usage`
  and `modelUsage` is 0 on every turn. `total_credits` equals
  `modelUsage.<model>.credits`: 0.0185 for the one-word turn on Fast and 0.0886
  for the two-tool turn on Fast. A one-word turn at `high` cost 0.0710 on
  DeepSeek-V4-Pro, 0.2078 on GLM-5.3, 0.2517 on Qwen4-Max and 0.3785 on
  Kimi-K3.1-Pro. A credit has no published dollar rate in this record.
- **The served model is not checked against the request.** An unknown `-m`
  falls back to `Auto` with exit 0. Init's `model` names what was served:
  `Fast` for `-m Fast`, `Auto` for the fallback, and the requested name for
  each of `Qwen4-Max`, `GLM-5.3`, `DeepSeek-V4-Pro` and `Kimi-K3.1-Pro`.
  `modelUsage` is keyed by an internal code (`fast`, `qmodel_latest`,
  `gmodel`, `dmodel`, `kmodel_latest`), so only init's `model` names the
  model.
- **Efforts.** `--reasoning-effort` takes `auto`, `none`, `low`, `medium`,
  `high`, `xhigh`, `max` or `ultracode`. Any other value exits 1 before a
  model call.
- **Tools.**
  - With no `--tools`, init lists 34 tools under `accept_edits`. Among them
    are `Agent`, `Workflow`, `CronCreate`, `ScheduleWakeup`, `WebFetch`,
    `WebSearch`, `ImageGen` and `VideoGen`.
  - `--tools` with names restricts the built-ins to those names.
  - `--tools ""` leaves the three MCP meta-tools: `mcp_call`, `mcp_get` and
    `mcp_list`. `--disallowed-tools` with all three, comma-separated or
    repeated, leaves `tools: []`.
- **MCP.** `--strict-mcp-config --mcp-config '{"mcpServers":{}}'` gives
  `mcp_servers: []`, but it does not keep a user-scope server from
  starting. Isolation takes `--setting-sources ""` beside it. See
  [MCP isolation](#mcp-isolation-si2).
- **Permissions.**
  - `--permission-mode` takes `default`, `accept_edits`, `bypass_permissions`,
    `dont_ask` or `auto`.
  - `--allowed-tools` with `Bash(<command>:*)` admits that command. Several
    patterns go in one comma-separated value.
  - A read-only command runs without being listed (`id -u`).
  - A writing command that is not listed is refused in both `default` and
    `dont_ask`. The refusal is a `tool_result` with `is_error: true` and an
    entry in `permission_denials`, and the turn still ends `success`.
- **Auth.**
  - The signed-in state lives in the config root: `~/.qoder` by default, or
    `--config-dir` or `QODER_CONFIG_DIR`. An empty root is signed out.
  - The binary names `QODER_PERSONAL_ACCESS_TOKEN`, `QODER_PAT`,
    `QODER_SDK_ACCESS_TOKEN`, `QODER_DEVICE_TOKEN` and `QODER_AUTH_STDIN`.
    None was exercised.
- **Always loaded.** 21 built-in skills and 5 built-in agents (`general-purpose`,
  `statusline-setup`, `Explore`, `Plan`, `qoder-guide`) load on every turn,
  also under `--setting-sources ""`.
- **Models.** `--list-models` lists Auto, Ultimate, Fast, Efficient, Sonus,
  Cantus, Qwen4-Max, Qwen4-Max-Preview, Qwen3.8-Max, Qwen3.8-Flash,
  Kimi-K3.1-Pro, Kimi-K2.8-Preview, GLM-5.3, GLM-5.3-Flash, DeepSeek-V4-Pro,
  DeepSeek-Flash and MiniMax-M3.1-Flash. Which models stand behind Auto,
  Ultimate, Fast, Efficient, Sonus and Cantus is not published in the CLI.
- **Persistence.** `--no-session-persistence` keeps no session on disk, in
  print mode only.
- **Updates.** The CLI has `update` and `rollback`. Whether it updates itself
  in the background is unmeasured.

## MCP isolation (SI2)

The controller measured this on 2026-11-20, the same host, Qoder CLI 1.1.92.
Round one's seat found that the compile refuses every qoder seat on Linux
without it ("provider 'qoder' has no measured strict MCP configuration for
'cold'").

**The sentinels.** Each is a stdio MCP server that writes a marker file
when it starts, records its parent's command line, lists one `<name>_ping`
tool and records each `tools/call`. Four were planted, and each was removed
after the measurement:

| Scope | Where Qoder keeps it | How it was planted |
| --- | --- | --- |
| user | `~/.qoder/settings.json`, `mcpServers` | `qodercli mcp add -s user` |
| local | `<workdir>/.qoder/settings.local.json` | `qodercli mcp add -s local` |
| project | `<workdir>/.mcp.json` | `qodercli mcp add -s project` |
| plugin | an installed plugin, user scope, with an `.mcp.json` | `qodercli plugin install <dir>` |

**The turns.** Every turn ran the seat's head (`-p --output-format
stream-json --no-session-persistence`) with `-m Fast --reasoning-effort low`.
Q01 to Q11 also ran `--permission-mode accept_edits --allowed-tools
'Bash(ls:*)' --tools Read Edit Write Glob Grep Bash`, and all but Q02 added
`--disallowed-tools mcp_call,mcp_get,mcp_list`. Each turn used the same
HOME and workdir. "Started" is a marker file the turn's own `qodercli`
wrote through its sentinel.

| Case | MCP flags | Sentinels started | Init `mcp_servers` |
| --- | --- | --- | --- |
| Q01, positive control (the round-one argv) | none | user, local, project, plugin | all four, `connected` |
| Q02, positive control, meta-tools kept | none | user, local, project, plugin | all four, `connected` |
| Q03, cold | `--strict-mcp-config --mcp-config <empty file>` | **user** | `[]` |
| Q04, replacement (Q03's argv again) | the same | **user** | `[]` |
| Q05, cold | `--strict-mcp-config --mcp-config '{"mcpServers":{}}'` | **user**, twice | `[]` |
| Q06, Q03 again, parent recorded | the same as Q03 | **user**, twice; parent is the turn's own `qodercli -p …` | `[]` |
| Q07, cold | `--strict-mcp-config --mcp-config <empty file> --setting-sources ""` | none | `[]` |
| Q08, cold | the same with `--setting-sources project,local` | none | `[]` |
| Q09, cold | `--setting-sources ""` alone | project | the project sentinel |
| Q10, cold | Q07's | none | `[]` |
| Q11, replacement (Q10's argv again) | Q07's | none | `[]` |

**Carriage.** Q14 (cold) and Q15 (replacement, Q14's argv again) ran
`--permission-mode dont_ask --tools "" --allowed-tools
"mcp__brokkr-engine-sentinel__brokkr_engine_sentinel_ping" --strict-mcp-config
--mcp-config <engine file> --setting-sources ""`. The engine file names one
sentinel, `brokkr-engine-sentinel`. In each turn:

- the engine sentinel started;
- init listed it `connected`, and its tools were `mcp_call`, `mcp_get` and
  `mcp_list`;
- the model called `mcp_get`, then `mcp_call` with `toolName`
  `mcp__brokkr-engine-sentinel__brokkr_engine_sentinel_ping`;
- the sentinel recorded the `tools/call`, and the turn's result was
  `pong from brokkr-engine-sentinel`;
- no planted sentinel started.

**An MCP tool needs its own grant.** Q12 and Q13 ran Q14's flags under
`accept_edits` with the native tools of Q01 and no grant for the engine
tool. The engine sentinel started and was listed `connected`, but every
`mcp_call` was refused: "Allow MCP tool
brokkr-engine-sentinel/brokkr_engine_sentinel_ping?". Q12 then ran to its
180 s bound trying other tools. Qoder's permission check refused all of
them, each with an entry in `permission_denials`:

- reads outside the workdir;
- `Bash` commands outside the allow-list;
- `Write` and `Edit` on `<workdir>/.qoder/settings*.json` and `.mcp.json`,
  as "sensitive".

That check is not an isolation mechanism, and nothing here relies on it.

**What this measures.**

- `--strict-mcp-config` keeps project, local and plugin servers out, and
  the engine's file is the only server source it lists. It still starts
  the user-scope server, unlisted, inside the turn's own process tree
  (Q03 to Q06).
- `--setting-sources ""` keeps the user, local and plugin servers out, but
  not the project's `.mcp.json` (Q09).
- Together they started no planted sentinel, cold or replacement, with an
  empty file (Q07, Q10, Q11) or an engine file (Q14, Q15).
- The engine's sentinel started, was listed, and answered `tools/call`
  only under an `--allowed-tools` grant naming it as
  `mcp__<server>__<tool>`.

## Not measured

- A token-authenticated turn (`QODER_PERSONAL_ACCESS_TOKEN` under an empty
  config root).
- A managed or enterprise MCP configuration, which this account does not
  have.
- Native writes, store reads and process reads outside the workdir, as an
  isolation property. Qoder's permission check refused the attempts above,
  and that check is not confinement.
- A boxed turn, its egress and its process reach.
- macOS.
