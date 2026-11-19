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
  `mcp_servers: []`. The account had no user-scope server. No planted user,
  project or plugin sentinel was tried, so ambient isolation is unmeasured.
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

## Not measured

- A token-authenticated turn (`QODER_PERSONAL_ACCESS_TOKEN` under an empty
  config root).
- Ambient MCP isolation against planted sentinels.
- A boxed turn, its egress and its process reach.
- macOS.
