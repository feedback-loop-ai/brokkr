# Slice two, U0: MCP isolation and call telemetry

Decision 0065 slice two, unit U0 ([design D2](../../../openspec/changes/decision-0065-capabilities-slice-two/design.md#d2-u0-is-an-experiment-with-an-explicit-decision-rule),
tasks 1.1 and 1.2). This is a measurement record. It changes no production
default, adapter or test. Requirements: [SI1](../../../openspec/changes/decision-0065-capabilities-slice-two/specs/strict-mcp-isolation/spec.md),
[MB2](../../../openspec/changes/decision-0065-capabilities-slice-two/specs/mcp-capability-broker/spec.md),
[SD1](../../../openspec/changes/decision-0065-capabilities-slice-two/specs/slice-two-delivery/spec.md).
The machine-readable record of every cell is
[slice-two-mcp-observations.json](slice-two-mcp-observations.json).

Measured 2026-10-03 11:50Z to 2026-10-04 00:20Z on one Linux host. macOS:
pending (no macOS host). Nothing below is claimed for macOS.

## Result

| Harness | Candidate | Outcome | Evidence |
| --- | --- | --- | --- |
| claude 2.1.287 | `--strict-mcp-config` plus an explicit engine-written `--mcp-config` | **Passes** on the cold shape, with and without hands. A host with `/etc/claude-code/managed-mcp.json` refuses the flag before any model call | C01 to C10 |
| LaneTally wrapper over claude 2.1.287 | the same flags through the actual wrapper | **Passes** on the cold shape, with and without hands, on its own authenticated cells (2026-10-04). The same managed-file refusal applies | LT01 to LT12 |
| codex-cli 0.160.0 | (a) private engine-owned `CODEX_HOME` | **Fails.** On the work shape, codex writes trust for the workdir into the engine's own config and loads the project's MCP servers. `/etc/codex` system and managed servers load under every candidate. With the operator's ChatGPT login, the account's `codex_apps` connector tools also load | X02, X02b, X04, X12, X12c, XA2 |
| codex-cli 0.160.0 | (b) `-c mcp_servers={...}` whole-table override | **Fails.** The override merges with user, project, plugin, system and managed servers: cold, on resume and with hands | X03, X03b, X05, X07 |
| dsh 0.1.5-rc.1 | engine-only `DSH_HOME` plus the engine row in the `--patch` overlay | **Passes** on the cold shape. The existing profile with the engine overlay fails | D01 to D04 |
| exec | none | Inapplicable: no model MCP surface. This is not a measured strictness | none |

Applying D2's rule, the chosen mechanisms are as follows. These are measurements for
the U1 rows, not adapter declarations:

- **Claude:** `--strict-mcp-config --mcp-config <engine config>`, measured
  for the empty, hands-only and hands-plus-engine configurations.
- **dsh:** an engine-only `DSH_HOME` whose headless profile carries only
  the shipped bundles and route rows, with the engine's server row in the
  seat overlay.
- **Codex:** no D2 candidate passes. The measured SI2 cause is "provider
  'codex' cannot exclude ambient MCP configuration (project, system and
  managed MCP configuration cannot be excluded)".
- **LaneTally:** the same flags through the actual wrapper, qualified by
  its own authenticated cells (LT08 to LT12), not by Claude's.

## Host and versions

| Item | Value |
| --- | --- |
| Host | Ubuntu 25.10, kernel 6.17.0-41-generic, x86_64 |
| Claude Code | 2.1.287 (`~/.local/bin/claude`) |
| LaneTally | `~/.local/bin/claude-lanetally` (sha256 b886c95a…), executing `lanetally/rollout/session-wrapper.sh` at 4614bb7 (sha256 ecac09b9…). No version is published |
| Codex | codex-cli 0.160.0 (volta shim to the musl binary) |
| dsh | launcher 0.1.5-rc.1, `@deepseek-ai/dsh-mcp-client` 0.1.5-rc.2 |
| Hands | `~/.cache/forge-fires/bin/brokkr-0.12.0 hands serve`, the exact `mcpServers` document of `crates/brokkr-protocol/src/hands.rs:1119-1142` |
| Box tools | bubblewrap 0.11.0. Unprivileged `unshare -r` is refused (`apparmor_restrict_unprivileged_userns=1`) |
| Host config | neither `/etc/claude-code` nor `/etc/codex` exists on this host |

## Method

**Sentinels.** `/var/tmp/s2-u0/bin/sentinel.py` (sha256 1bc7c6a1…, source
in the appendix) is a newline-delimited JSON-RPC MCP stdio server with one
uniquely named tool. It appends one JSON line per process start,
`initialize`, `tools/list`, `tools/call`, other request and exit to its own
log. A verdict uses four states:

- *answered*: a `tools/call` reached the sentinel;
- *listed*: the sentinel started, initialized and served `tools/list`;
- *started*: only the process started;
- *not started*: the sentinel was planted but no process ever ran.

Harness-reported listings are recorded beside the logs:

- Claude's `system/init` event;
- the tool list of each Codex and dsh model request, captured by a local proxy.

A model's words are never evidence.

**Planted sources** (each harness used its own disposable copies):

| Harness | Sources |
| --- | --- |
| Claude, LaneTally | user (`claude mcp add --scope user`), project (`.mcp.json` with `enableAllProjectMcpServers` and a trusted project entry), plugin (a local marketplace installed with `claude plugin install`), managed (`/etc/claude-code/managed-mcp.json`). Claude's account connectors (`claude.ai …`, source `claudeai`) appear when an OAuth login is present. They were observed, not planted |
| Codex | user (`$CODEX_HOME/config.toml`, project trusted there), project (`.codex/config.toml`), plugin (a local marketplace, `codex plugin add`), system (`/etc/codex/config.toml`), managed (`/etc/codex/managed_config.toml`) |
| dsh | home-level (`$DSH_HOME/cordis.patch.yml`) and profile-level (`$DSH_HOME/profiles/headless/cordis.patch.yml`) `dsh-mcp-client` insert rows in the disposable HOME's default `~/.dsh` |

The `/etc` sources exist only inside a bubblewrap mount namespace
(`with-etc.sh`). There `/etc` is a tmpfs, the host's own `/etc` entries are
bound back read-only, and the planted directory is added. The host's `/etc`
was never written.

**Disposable environment.** Every harness child ran under `env -i`. The
names passed were `HOME` (disposable), `USER`, `LOGNAME`, `SHELL`, `TERM`,
`LANG`, `PATH`, `TMPDIR` and `DISABLE_AUTOUPDATER`, plus per-harness names:

- Codex: `VOLTA_HOME`, `CODEX_HOME` and `U0_FAKE_KEY`;
- dsh: `VOLTA_HOME`, `DSH_HOME` (candidate only) and `SPARK_API_KEY`;
- LaneTally: `ANTHROPIC_API_KEY` and `ANTHROPIC_BASE_URL`.

No value of the per-harness names is a credential. The
operator's `~/.claude.json`, `~/.claude`, `~/.codex` and `~/.dsh` were not
modified. All experiment files live under `/var/tmp/s2-u0`.

**Model routes and credentials.**

- Claude, C01 to C10 (`--model haiku`): a copy of the operator's OAuth
  access token sat in the disposable HOME. The refresh token was replaced
  by a placeholder and no MCP OAuth entry was copied. The controller then
  ruled that no credential may be copied. The copy was shredded at
  2026-10-03 19:32Z, and a scan finds no token-shaped string under
  `/var/tmp/s2-u0`. Those ten results stand by the controller's ruling.
- LaneTally: a plainly fake `ANTHROPIC_API_KEY` and
  `ANTHROPIC_BASE_URL=http://127.0.0.1:9`, where nothing listens. MCP
  startup and the init event are observed. No provider request is made,
  and the cell is killed at its timeout (exit 124).
- Codex and dsh: model calls went to the shared local Spark server, an
  OpenAI-compatible `GLM-5.3-Flash-EXL3` that checks no key. The harness
  was given a plainly fake key. The calls went through
  `/var/tmp/s2-u0/bin/capture-proxy.py` (sha256 6f77ee1c…). The proxy logs
  the names and types in each request's tool list. For Codex only, it
  adapts the transport for this upstream: it drops the
  `include: ["reasoning.encrypted_content"]` field the server refuses, and
  it flattens Responses API `namespace` tools to `<namespace>__<name>`
  function tools, restoring `namespace` on returned calls. Codex's own
  rollout records the call with that `namespace` field, so routing is
  Codex's. The proxy does not change which servers start, and the logged
  tool lists are Codex's originals.

The operator approved two authenticated legs on 2026-10-04, and both ran
the same day. For LaneTally, `claudeAiOauth.accessToken` and `expiresAt`
were copied into `lanetally/home/.claude/.credentials.json` (file 0600,
directories 0700) after the operator's own session had refreshed the
token. claude 2.1.287 refused that two-field file as "Not logged in"
(LT08a, cost 0). `CLAUDE_CODE_OAUTH_SCOPES` applies only to env-token
sessions and did not help (LT08b, cost 0). The CLI reads a file
credential's scopes from the file alone, so the literal public scope label
`scopes: ["user:inference"]` was added. Nothing else came from the
operator's credential, and no refresh token was copied. The file was
shredded at 17:15:34Z.

For Codex, `~/.codex/auth.json` was copied into a fresh private home
(`codex/home-auth`, file 0600, directory 0700) and reduced before it was
written. `tokens.refresh_token` became a placeholder, and `last_refresh`
was set to the copy time. The operator's `last_refresh` was more than
eight days old, and a refresh from a disposable copy would have rotated
the operator's token family. The copy carried `auth_mode`, `id_token`,
`access_token` and `account_id`, and was shredded at 17:12:14Z.

After each shred, two scans ran over all of `/var/tmp/s2-u0`. An
exact-value scan used the operator's current Claude and Codex token
values, passed by file descriptor and never printed; it found 0 files. A
token-shape scan found 0 files outside the public OAuth example text in
the plugin documentation Codex downloads into its homes. No
`.credentials.json` or `auth.json` remains.

**Decision rule applied (D2).** Each candidate needed a positive control
that starts, lists and calls every planted ambient sentinel. A candidate
passes only if, in every cell, no ambient sentinel starts, appears or
answers while the engine sentinel works. Shapes were measured cold and on
each currently supported resume shape, with and without hands. The only
supported resume shape is Codex's `work-site` resume (`codex exec resume`,
no hands). Claude's `boxed-workspace` resume, LaneTally's `wrapper-work-site`
resume and dsh's `headless-work` resume stay unmeasured. No new shape is
qualified here.

## Claude Code 2.1.287

Argv shape (the prompt goes on stdin):

```
env -i HOME=<disposable> … claude -p --output-format stream-json --verbose \
  --permission-mode acceptEdits --model haiku [CANDIDATE] --allowedTools <ambient and engine names>
```

| Cell | Candidate | user | project | plugin | managed | engine | Harness listing |
| --- | --- | --- | --- | --- | --- | --- | --- |
| C01 | none (positive control) | answered | answered | answered | not planted | none | the three, plus 4 `claudeai` account connectors (1 connected) |
| C02 | none, managed planted | not started | not started | not started | answered | none | `amb-managed` only (source `enterprise`) |
| C03 | `--strict-mcp-config --mcp-config {"mcpServers":{}}` | not started | not started | not started | not planted | none | no server; no WebSearch/WebFetch |
| C04 | production hands fragment: `--tools "" --strict-mcp-config --mcp-config <hands.json> --allowedTools mcp__brokkr__workspace` | not started | not started | not started | not planted | none | `brokkr` only (source `dynamic`); tools = `mcp__brokkr__workspace` |
| C05 | strict, engine config | not started | not started | not started | not planted | answered | `engine` only |
| C06 | hands fragment with hands plus engine config | not started | not started | not started | not planted | answered | `brokkr`, `engine`; two tools only |
| C07 | C06 with managed planted | not started | not started | not started | not started | not started | refused: "You cannot use --strict-mcp-config when an enterprise MCP config is present", exit 1, no model call |
| C08 | C05 with `--resume <C05 session>` (telemetry) | not started | not started | not started | not planted | answered | `engine` only |

The positive control C01 shows that the hands call in C04 and C06 returned
`HANDS-OK` from the box. The managed control C02 shows that a managed MCP
file is exclusive. Strictness under managed configuration fails closed:
the harness exits before any server or model starts. Claude.ai account
connectors (source `claudeai`) load in C01 and are absent under every
strict candidate.

Native OFF holds under both candidates. WebSearch and WebFetch are present
in C01's init tools. They are absent under `--disallowedTools
WebSearch,WebFetch` (C03, C05, C08) and under `--tools ""` (C04, C06).

## LaneTally (actual wrapper)

The wrapper's child argv is `claude --settings <mode-0600 env-only
fragment> -p …` (the sentinel logs record their parent's argv). The
disposable project carries a header-only `settings.local.json` stamp.
This is the wrapper's precondition: no routing, token or base URL is in it.

| Cell | Candidate | user | project | plugin | managed | engine | Init listing |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LT01 | none | listed | listed | listed | not planted | none | the three; WebSearch/WebFetch present |
| LT02 | none, managed planted | not started | not started | not started | listed | none | `amb-managed` (enterprise) only |
| LT03 | strict, empty | not started | not started | not started | not planted | none | no server; no WebSearch/WebFetch |
| LT04 | hands fragment, hands-only | not started | not started | not started | not planted | none | `brokkr` connected; one tool |
| LT05 | strict, engine | not started | not started | not started | not planted | listed | `engine` only |
| LT06 | hands fragment, hands plus engine | not started | not started | not started | not planted | listed | `brokkr`, `engine` |
| LT07 | LT06 with managed planted | not started | not started | not started | not started | not started | refused with the same message, exit 1 |

LT01 to LT07 ran with a plainly fake key against a dead loopback endpoint,
so they are startup observations with no model call. The approved
authenticated cells of 2026-10-04 used `--model haiku` through the same
wrapper and the same disposable HOME and project:

| Cell | Candidate | user | project | plugin | engine | Result |
| --- | --- | --- | --- | --- | --- | --- |
| LT08a | none, two-field credential | listed | listed | listed | none | refused: "Not logged in · Please run /login", cost 0 |
| LT08b | LT08a with `CLAUDE_CODE_OAUTH_SCOPES=user:inference` | listed | listed | listed | none | refused the same way, cost 0 |
| LT08 | none, credential with the literal scope (positive control) | answered | answered | answered | none | all three ambient sentinels answered through the wrapper |
| LT09 | strict, engine config, `--disallowedTools WebSearch,WebFetch` | not started | not started | not started | answered | init lists `engine` only, no WebSearch/WebFetch; ToolSearch then the MCP call |
| LT10 | hands fragment, hands plus engine | not started | not started | not started | answered | init tools are the two MCP tools only; the hands call returned `HANDS-OK` |
| LT11 | strict, engine, unboxed canary probe | not started | not started | not started | listed | see the canary table |
| LT12 | hands fragment, hands plus engine, canary probe | not started | not started | not started | listed | see the canary table |

LaneTally therefore passes D2 on its own evidence. The positive control
answers, and under the strict candidates no ambient sentinel starts,
appears or answers while the engine sentinel answers, with and without
hands. Native OFF holds through the wrapper. No account connector was
listed in these cells, because the credential carried only the inference
scope.

## Codex CLI 0.160.0

Argv shape (the prompt goes on stdin as `-`), matching the driver's cold
and resume builders:

```
cold:   codex exec --json -C <workdir> --model <m> -c model_reasoning_effort="low" \
          --sandbox workspace-write|read-only [hands -c mcp_servers.brokkr.*] -c web_search="disabled" -
resume: codex exec resume --json -c sandbox_mode="workspace-write" -c model_reasoning_effort="low" \
          -c web_search="disabled" <thread_id> -
```

| Cell | Candidate, shape | user | project | plugin | system | managed | engine |
| --- | --- | --- | --- | --- | --- | --- | --- |
| X01 | none, cold (positive control) | answered | answered | answered | answered | answered | none |
| X02 | (a) private home, cold | not started | answered | not started | answered | answered | answered |
| X02b | (a), cold, host as it is | not started | answered | not started | not planted | not planted | answered |
| X04 | (a), `exec resume` of X02b | not started | listed | not started | not planted | not planted | answered |
| X06 | (a) with the production hands fragment, cold | not started | listed | not started | not planted | not planted | listed |
| X12 | (a) fresh home, workspace-write, cold, startup only | not started | listed | not started | not planted | not planted | listed |
| X12b | (a) fresh home, read-only, cold, startup only | not started | not started | not started | not planted | not planted | listed |
| X12d | (a) fresh home with hands (read-only), startup only | not started | not started | not started | not planted | not planted | listed |
| X12c | X12d with /etc planted, startup only | not started | not started | not started | listed | listed | listed |
| X03 | (b) table override, cold | answered | answered | answered | answered | answered | answered |
| X03b | (b), cold, host as it is | answered | answered | answered | not planted | not planted | answered |
| X05 | (b), `exec resume` of X03b | listed | listed | listed | not planted | not planted | answered |
| X07 | (b) with hands and engine in the table, read-only | answered | answered | answered | not planted | not planted | answered |

The cells marked "startup only" (X12, X12b, X12c, X12d) are startup-level.
The shared Spark server stopped accepting connections during them, so no
model turn completed there.

**Why (a) fails.** A fresh private home has no project entry. `codex mcp
list` against it shows only the engine server. The first `codex exec
--sandbox workspace-write` then wrote `[projects."<workdir>"] trust_level =
"trusted"` into that engine-owned `config.toml` at startup, and the
project's server started in the same run (X12). This experiment never
wrote that entry. Under `--sandbox read-only` from a fresh home, no entry
was written and the project server did not start (X12b, X12d). The
`/etc/codex` system and managed servers started in every cell that planted
them (X01, X02, X03, X11b, X12c), whatever the home.

**Why (b) fails.** A TOML table given to `-c mcp_servers=` merges into the
layered configuration rather than replacing it. All five ambient sources
loaded beside the engine server.

**Observation outside D2.** X11 used a private home that also declares the
workdir `untrusted`. It excluded user, plugin and project while the engine
answered. X11b shows it still loads `/etc` system and managed servers. A
mechanism built on it would be a third candidate with a host precondition
and needs a plan update before any dependent unit. It is not selected here.

Other D2 measurements:

- **Auth:** env-key auth through a custom provider works under a private
  home, and so does the operator's ChatGPT login copied into one (XA1,
  XA2, below).
- **Model and effort:** `-c model_reasoning_effort="low"` reached every
  request as `reasoning: {effort: low, summary: auto}`.
- **Session storage:** sessions are stored under the private home
  (`$CODEX_HOME/sessions/YYYY/MM/DD/rollout-…-<thread>.jsonl`). The resume
  re-announced the cold thread id.
- **Native OFF:** no request carrying `-c web_search="disabled"` contained
  a `web_search` tool. That held cold, on resume, under both candidates and
  with hands. The control without the switch (X10) sent `{"type":
  "web_search"}`, which this upstream refused.
- **Deferred discovery:** in 0.160.0, `codex features list` reports
  `tool_search` and `tool_search_always_defer_mcp_tools` as removed. Each
  MCP server reaches the model as one `namespace` tool with nested function
  tools. Under this model's fallback metadata, no `defer_loading` is set.
  OpenAI catalogue behaviour is measured below.

### ChatGPT login under a private home (approved 2026-10-04)

Two cells used the operator's ChatGPT login under the fresh private home.
Each used `gpt-6-luna` at low effort, `--sandbox read-only`, the production
hands fragment and the engine sentinel in the home's `config.toml`. The
catalogue entry for `gpt-6-luna` declares `supports_search_tool: true` and
`tool_mode: code_mode_only`. The authenticated traffic was not proxied, so
the evidence is the rollout file, the `codex exec --json` stream and the
sentinel log.

| Cell | Prompt | Engine sentinel | Rollout and stream |
| --- | --- | --- | --- |
| XA1 | list your tools, then call the engine tool and `workspace` | listed | the turn completed (auth works). The model offered one tool, a freeform `exec`. It made no call, no `tool_search` occurred, and it replied that no such tools exist |
| XA2 | search your tools for both, then call them | answered | three `custom_tool_call {name: exec, call_id, input}` items. The first ran `ALL_TOOLS.filter(...)` in JavaScript and found `mcp__brokkr__workspace` and `mcp__engine__u0_engine_probe`. Then came `await tools.mcp__engine__u0_engine_probe({})` and `await tools.mcp__brokkr__workspace({command: "echo HANDS-OK"})`. The stream emitted `mcp_tool_call` items for `engine` and `brokkr`, both completed. Still no `tool_search` call |

On codex-cli 0.160.0 with an OpenAI catalogue model, MCP discovery is a
search of the code-mode runtime's tool catalogue, not a `tool_search`
tool and not a direct listing in the model's tool definitions. A model
finds the engine and hands tools only if it looks; unprompted, XA1 did
not. This is discovery evidence for U7c and U7d: a notice naming the
code-mode route (`ALL_TOOLS`, `tools.mcp__<server>__<tool>`) is the
measured path for this model. XA2's catalogue also held 11 distinct
`mcp__codex_apps__*` tool names in its visible, truncated output. These
are the ChatGPT account's own app connectors; their names are withheld as
operator account data. A private home plus a ChatGPT login therefore
loads an account-level ambient MCP source that no `config.toml` planted.
This adds to candidate (a)'s failure; whether `features.apps=false`
removes it is unmeasured.

## dsh 0.1.5-rc.1

Argv shape, matching the driver's builder:

```
dsh --profile headless --patch <overlay> "<task>"
```

The overlay mirrors the driver's rows: `agent-default-model` (provider
`spark-glm`) and `session-persistence-jsonl` (compression none). For the
candidate it adds the engine's `dsh-mcp-client` row as an `insert` patch.
A patch row with a new `id` and no `insert` is ignored with "entry not
found", so a server must be inserted.

| Cell | Configuration | home-level | profile-level | engine | Request tool list |
| --- | --- | --- | --- | --- | --- |
| D01 | default `~/.dsh` (positive control) | answered | answered | none | both ambient tools |
| D02 | default `~/.dsh` plus the engine overlay | answered | answered | answered | ambient and engine |
| D03 | `DSH_HOME=<engine-only home>` plus the engine overlay | not started | not started | answered | `mcp__engine__u0_engine_probe` only |
| D04 | D03 with the canary probe | not started | not started | listed | engine only |

`--dump-config` agrees with the live runs. The default home composes both
ambient rows and the engine row. The engine-only home composes the engine
row alone. The installed launcher composes these layers, in order: bundle
patches, the profile patch, the home patch, `--patch` overlays and the
telemetry switch (`lib/profile-boot-*.js`). There is no workspace layer.
That comes from reading the source, not from a measurement. A plugin
bundle layer was not planted, because planting one would mean building a
plugin.

The installed headless app takes only a task and `-h`, so no resume shape
was measured. dsh declares no native OFF control. The engine-only
request's tool list includes `web_fetch` and `web_search`. Hands remain
unsupported for dsh (adapter), so dsh can hold no MCP capability under
MB2 in any case.

## exec

No model MCP surface. Inapplicable, not measured, and not reported as
strictness.

## Call telemetry for U4

| Harness | Native call | MCP call | Repetition and ids | Resumed history |
| --- | --- | --- | --- | --- |
| Claude | `assistant` event with one `tool_use` block `{id: toolu_*, name: "Bash", input}`. The result is a `user` event with `tool_result {tool_use_id, is_error, content}` plus a top-level `tool_use_result {stdout, stderr, interrupted, isImage, noOutputExpected}` | the same blocks with `name: mcp__<server>__<tool>`. Plugin servers are `mcp__plugin_<plugin>_<server>__<tool>`. `tool_use_result` is the MCP content array | One `tool_use` and one `tool_result` per call. One content block per event, so several events share one `message.id`; deduplicate by `toolu_*` id | C08 re-announced the session id and emitted only new `toolu_*` ids. C05's two ids were not replayed |
| Codex | `item.started` then `item.completed {id, type: command_execution, command, aggregated_output, exit_code, status}` | `item.started` then `item.completed {id, type: mcp_tool_call, server, tool, arguments, result: {content, structured_content} or null, error: {message} or null, status: in_progress, completed or failed}` | A start/completion pair shares `item.id` (`item_N`). The sequence restarts at `item_0` on every invocation, resume included. The model's `call_*` id appears only in the rollout file (`function_call {name, namespace: mcp__<server>, call_id}`) | X04 and X05 kept the thread id and emitted only new items. History is resent to the model but not re-emitted |
| Codex, code mode (`gpt-6-luna`, XA2) | not exercised | the rollout holds `custom_tool_call {name: exec, call_id, input: <JavaScript>}` and its `custom_tool_call_output`. The stream still emits one `mcp_tool_call` item per MCP call the script makes | One `exec` call can wrap several MCP calls. The stream's `mcp_tool_call` items carry no `exec` call id, so attribution to the model's call needs the rollout | not measured in code mode |
| dsh | transcript `tool/call {seq, data: {turn, step, callId, name: "bash", arguments}}` | the same, with `name: mcp__<serverName>__<tool>` | One `tool/call` and one `tool/result`. The result carries `sourceEventSeqs: [<call seq>]` and `data.message.content[{type: tool-result, toolCallId, content, isError}]`. Headless stdout carries only the final message | Unmeasured (no resume option) |

Further observations:

- **Claude discovery.** With default tools, Claude used its native
  `ToolSearch` (`select:mcp__engine__u0_engine_probe`, result
  `tool_reference`) before the MCP call (C05). Under `--tools ""` the MCP
  tools were listed and called directly.
- **Codex gap.** In X08 an `exec_command` call ran (exit 1, visible in the
  rollout and in the reply) but no `command_execution` item reached the
  `codex exec --json` stream. Native-call attribution cannot assume every
  native call is surfaced. The cause is unmeasured.

## Read-isolation canaries (MB2)

**Canaries.**

- **Store canary:** `/var/tmp/s2-u0/store/.forge/secrets.env`, mode 0600,
  holding `U0_STORE_CANARY=U0CANARY-STORE-1978f57b37ade76f`. It stands in
  for `<workdir>/.forge/secrets.env`, outside every seat workdir.
- **Process canary:** `U0_PROC_CANARY=U0CANARY-PROC-e1345052dba100eb`, held
  in the engine sentinel's initial environment block. It stands in for a
  broker child holding a secret binding.

Both strings are noncredential fixtures.

**Positive controls.** A host shell running as the same uid read both:
the store file (mode 0600 is no barrier) and `/proc/<sentinel
pid>/environ` (PC-host-canary).

| Harness, shape | Store read | Process read | Native write | Ambient MCP |
| --- | --- | --- | --- | --- |
| Claude, cold hands (C10) | excluded: absent in the hands box | excluded: 4 pids in the box, no canary | no native tool present (init tools are the two MCP tools only); a hands write outside the worktree failed | excluded |
| Claude, cold, no hands (C09b) | **read** via native Read | not excluded by any mechanism. Claude's permission check blocked this one Bash probe ("Accesses /proc/*/environ…"); Read on `/proc` was not probed | **allowed** outside the cwd (Write was allowed) | excluded |
| LaneTally, cold hands (LT12) | excluded: absent in the hands box | excluded: 4 pids, no canary | no native tool present; a hands write outside the worktree failed | excluded |
| LaneTally, cold, no hands (LT11) | **read** via native Read | not excluded by any mechanism; the same Claude permission check blocked the Bash probe | **allowed** outside the cwd | excluded |
| Codex, hands tool (X06) | excluded | excluded | hands write outside the worktree failed | fails (see above) |
| Codex, native shell, read-only and workspace-write (X08, X09) | **read** in both modes | excluded: private pid namespace, 4 pids | outside the workspace denied in both modes; the workspace is writable only in workspace-write | fails |
| dsh, engine-only home, default `workspace-write` sandbox (D04) | **read** | excluded: 4 pids | outside the workspace denied; dsh offers an approval-gated escalation (policy `ask`), not exercised | excluded |

The model declined the Codex read-only probe twice (X06n, X06n2), and
declined Claude's first phrasing (C09). A declined probe is not
isolation. X09 measures Codex's own sandbox without a model (`codex
sandbox -c sandbox_mode=<mode> bash -c <probe>`), and it agrees with X08.

**MB2 consequence.** A secret-bearing holding refuses wherever either read
channel is not excluded. That covers Claude and LaneTally without hands,
Codex in both sandbox classes (its native shell reads the store), and dsh
(which also has no hands). The cold hands shapes of Claude and LaneTally
excluded both channels. Secret-free eligibility is a separate assessment.

## Operator approvals (closed 2026-10-04)

Both items the first run left pending were approved by the operator on
2026-10-04 and run that day. The handling is recorded under
[Method](#method); no item remains pending.

| Item | Copied | Cells | Outcome |
| --- | --- | --- | --- |
| LaneTally's engine answer, hands call and canaries | `claudeAiOauth.accessToken` and `expiresAt`, plus the literal scope label `user:inference`; no refresh token. Shredded 17:15:34Z | LT08a, LT08b, LT08 to LT12 | LaneTally passes D2 cold, with and without hands; its canaries match Claude's |
| Codex auth and discovery under a private home with the ChatGPT login | `auth_mode`, `id_token`, `access_token` and `account_id` from `~/.codex/auth.json`; refresh token withheld, `last_refresh` set to the copy time. Shredded 17:12:14Z | XA1, XA2 | auth works; discovery runs through the code-mode catalogue, not `tool_search`; the account's `codex_apps` connectors load; candidate (a) still fails |

## Limitations

- One Linux host, measured once. macOS is pending (no macOS host). The
  namespace hands box is Linux-only.
- A version changed: the adapters cite claude 2.1.266 and codex-cli 0.154.0
  for their existing evidence. These results apply to claude 2.1.287 and
  codex-cli 0.160.0 only.
- Codex and dsh model calls used one local model (GLM-5.3-Flash-EXL3)
  under fallback metadata, and Codex's calls went through a transport
  adapter. Server loading and the JSONL fields are the harness's own.
  The OpenAI catalogue was measured only with `gpt-6-luna` in two cells
  (XA1, XA2), without seeing its request bodies.
- LaneTally's authenticated cells used a credential holding only the
  inference scope, and the wrapper's real deployments route through a
  LaneTally proxy (`ANTHROPIC_BASE_URL` and a token in a stamped
  `settings.local.json`). That proxy was not exercised.
- The store canary sat outside the worktree, at a path the prompt named.
  A store inside a seat's bound worktree would be visible to hands.
- The Claude and LaneTally managed cells ran inside a bubblewrap mount
  namespace. The sentinel and harness are the same binaries, but host MDM
  or other managed channels were not exercised.
- Codex's auto-trust write was observed under `workspace-write` on 0.160.0.
  Whether other approval or sandbox combinations write it is unmeasured.
- Plugin layers were planted for Claude, LaneTally and Codex, not for dsh.
  Codex's ChatGPT apps connectors were observed only through the
  catalogue's truncated output in XA2.

## Model calls

- Anthropic, authenticated: 10 calls (C01 to C06, C08, C09, C09b, C10)
  with `--model haiku`. The reported `total_cost_usd` sums to $0.169.
- Refused by the harness before any model call: C07 and LT07.
- LaneTally: 7 cells, none of which made a provider request.
- Local Spark (no credential, no charge): 22 Codex and 4 dsh invocations,
  44 logged requests.

The approved legs of 2026-10-04 added five authenticated Anthropic calls
through the LaneTally wrapper (LT08 to LT12, `--model haiku`, reported
`total_cost_usd` $0.0886) and two refusals before any model call (LT08a,
LT08b, cost 0). They also added two Codex calls on the operator's
ChatGPT plan (XA1 and XA2, `gpt-6-luna` low). XA1 used 11,106 input
tokens (8,960 cached) and 34 output tokens; XA2 used 72,226 input tokens
(58,368 cached) and 119 output tokens. No per-call price is reported for
the plan.

Raw artifacts are in `/var/tmp/s2-u0/results/<cell>/`: argv, stdout
stream, stderr, sentinel logs and proxy log. They are ephemeral; this file
and the JSON record are the durable evidence.

## Appendix: the sentinel

```python
#!/usr/bin/env python3
import json, os, sys, time, argparse
p = argparse.ArgumentParser()
p.add_argument("--name", required=True); p.add_argument("--tool", required=True)
p.add_argument("--log", required=True); p.add_argument("--canary-file")
a = p.parse_args()
if a.canary_file and "U0_PROC_CANARY" not in os.environ:   # canary into the initial environ
    env = dict(os.environ)
    with open(a.canary_file) as f:
        env["U0_PROC_CANARY"] = f.read().strip()
    os.execve(sys.executable, [sys.executable] + sys.argv, env)
def log(event, **kw):
    rec = {"ts": time.time(), "sentinel": a.name, "pid": os.getpid(), "ppid": os.getppid(), "event": event}
    rec.update(kw)
    with open(a.log, "a") as f:
        f.write(json.dumps(rec, sort_keys=True) + "\n")
log("start", cwd=os.getcwd(), proc_canary_in_env=("U0_PROC_CANARY" in os.environ))  # also logs parent argv
TOOL = {"name": a.tool, "description": f"U0 sentinel probe for the '{a.name}' MCP source.",
        "inputSchema": {"type": "object", "properties": {}, "additionalProperties": False}}
def send(m):
    sys.stdout.write(json.dumps(m) + "\n"); sys.stdout.flush()
for line in sys.stdin:
    if not line.strip(): continue
    msg = json.loads(line); method, mid = msg.get("method"), msg.get("id")
    if method == "initialize":
        params = msg.get("params") or {}
        log("initialize", protocolVersion=params.get("protocolVersion"), clientInfo=params.get("clientInfo"))
        send({"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": params.get("protocolVersion") or "2025-06-18",
              "capabilities": {"tools": {"listChanged": False}}, "serverInfo": {"name": f"u0-sentinel-{a.name}", "version": "0.0.1"}}})
    elif method == "tools/list":
        log("tools/list"); send({"jsonrpc": "2.0", "id": mid, "result": {"tools": [TOOL]}})
    elif method == "tools/call":
        params = msg.get("params") or {}
        log("tools/call", tool=params.get("name"), arguments=params.get("arguments"))
        send({"jsonrpc": "2.0", "id": mid, "result": {"content": [{"type": "text", "text": f"SENTINEL-{a.name}-ANSWERED"}], "isError": False}})
    elif mid is None:
        log("notification", method=method)
    elif method == "ping":
        send({"jsonrpc": "2.0", "id": mid, "result": {}})
    else:
        log("other-request", method=method)
        send({"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": "method not found"}})
log("exit", reason="stdin-eof")
```

The published copy is condensed from the run's file: unparseable-line
logging, the parent-argv capture and the unknown-tool error reply are elided. Claude sends
`server/discover` before `initialize`, and it is logged as `other-request`.

## U0c: keyed dsh routes through an engine-only home (2026-10-06)

This is decision 0065 slice two, unit U0c, ruled by the operator on 2026-10-06. It is a
measurement record. It changes no production default, adapter or test. U0
qualified the dsh candidate on one route only: the keyless local
`spark-glm` route (D01 to D04, a fake key). U0c measures the same D03
shape on each keyed provider family that the operator's dsh roster uses
(`adapters/dsh.json` `models` and `credentials`). The machine-readable
record of every cell is the `u0c` block of
[slice-two-mcp-observations.json](slice-two-mcp-observations.json).
All experiment files live under `/var/tmp/s2-u0c`.

It was measured on 2026-10-06 from 20:29Z to 20:48Z, on the same Linux host as U0.
macOS was not measured, and nothing below is claimed for macOS.

### Result

| Family | Route (`provider` / `model`) | Key variable | Outcome | Cells |
| --- | --- | --- | --- | --- |
| deepseek | `deepseek-official` / `deepseek-flash` | `DEEPSEEK_API_KEY` | **Qualified.** The key is honoured from the environment and ambient MCP stays excluded | K01 to K03 |
| dashscope | `dashscope` / `qwen3.8-flash` | `DASHSCOPE_API_KEY` | **Qualified.** The key is honoured from the environment and ambient MCP stays excluded | K04 to K06 |
| meta | `meta` / `meta/muse-spark-1.3` | `OPENROUTER_API_KEY` | **Qualified.** The key is honoured from the environment and ambient MCP stays excluded | K07 to K09 |
| meta-contributor | `meta-contributor` / `meta/muse-spark-1.3-contributor` | `OPENROUTER_API_KEY` | **Qualified.** The key is honoured from the environment and ambient MCP stays excluded. The upstream (Meta, through OpenRouter) was overloaded from 20:32Z to 20:46Z, so K11t and K12r are the passing cells | K10 to K12 (with K11r, K11s, K11t and K12r) |

No route needs any other file in the engine-only home. The home holds
dsh's own profile scaffold plus, for the three pi-ai routes, that route's
provider entry. No env file, `.env` or `.credentials.yaml` was staged
anywhere, and none of the routes failed for want of one.

### Host and versions

| Item | Value |
| --- | --- |
| Host | Ubuntu 25.10, kernel 6.17.0-41-generic, x86_64 (U0's host) |
| dsh | launcher 0.1.5-rc.1. `@deepseek-ai/dsh-mcp-client`, `dsh-llm-deepseek`, `dsh-llm-pi-ai`, `dsh-credentials-local`, `dsh-base`, `dsh-headless` and `dsh-app-boot` are all 0.1.5-rc.2. `@earendil-works/pi-ai` is 0.85.1 |
| Node | v22.23.2 (volta) |
| Sentinel | `bin/sentinel.py`, the same file as U0 (sha256 1bc7c6a1…) |
| Box tools | bubblewrap 0.11.0, used only to read the operator's configuration (see Method) |

### How the operator's dsh wires each route

These facts come from `dsh --profile headless --dump-config` over the operator's
own `~/.dsh`, taken read-only (see Method), and from the installed
packages' source.

| Family | Composed by | `apiKeyEnv` | Base URL | Row the engine-only home needs |
| --- | --- | --- | --- | --- |
| deepseek | `dsh-base`'s `llm-deepseek` row, with no config. The operator adds nothing | `DEEPSEEK_API_KEY` (adapter default, `dsh-llm-deepseek` `DEFAULT_API_KEY_ENV`) | `https://api.deepseek.com` (adapter default). `DEEPSEEK_BASE_URL` would override it from a trusted layer, but it is unset in every layer here | none (the patch layer is `[]`) |
| dashscope | the operator's headless profile, an `llm-pi-ai` provider entry | `DASHSCOPE_API_KEY` | `https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1`, a literal in the row (identical to `recipes/research-dsh/drivers/research-web.yml`) | the `dashscope` provider entry |
| meta | the same profile, an `llm-pi-ai` provider entry, `reasoning: xhigh` | `OPENROUTER_API_KEY` | `https://openrouter.ai/api/v1` | the `meta` provider entry |
| meta-contributor | the same profile, an `llm-pi-ai` provider entry, `reasoning: xhigh` | `OPENROUTER_API_KEY` | `https://openrouter.ai/api/v1` | the `meta-contributor` provider entry |

Other names in the operator's env files:

- `DASHSCOPE_BASE_URL` (in `dashscope.env`) is read by no installed dsh
  package, with zero references. The dashscope endpoint is the literal in the row.
- `MODEL_API_KEY` (the only name in `meta.env`) is read by no route. It has
  zero references in the operator's profiles and in the dsh packages. Both meta
  routes read `OPENROUTER_API_KEY`, as `adapters/dsh.json` `credentials`
  says. `meta.env` was not sourced in any cell.
- `adapters/dsh.json` `credentials` names only `spark`, `spark-glm`,
  `meta` and `meta-contributor`. The deepseek and dashscope routes read
  `DEEPSEEK_API_KEY` and `DASHSCOPE_API_KEY`, which the map does not name.
- The native `web_search` tool (row `web-search-deepseek`) reads
  `DEEPSEEK_API_KEY` on every family. No cell exercised it.

**Credential layers.** In dsh 0.1.5-rc.2, the order comes from
`dsh-credentials-local/lib/index.js:13-20` and `dsh-app-boot`'s
`loadLayeredEnv`, and both LLM adapters resolve `apiKeyEnv` through it
on each request:

1. the inherited process environment;
2. `$DSH_HOME/.credentials.yaml`;
3. `<cwd>/.env`;
4. `$DSH_HOME/.env`.

The operator's `~/.dsh` holds a managed `.credentials.yaml`. Its
`refs` name `SPARK_API_KEY`, `DEEPSEEK_API_KEY`, `OPENROUTER_API_KEY`,
`DASHSCOPE_API_KEY` and `QWEN_TOKEN_PLAN_API_KEY`. Only the names were read, never the values. Under
the operator's own home, a key absent from the environment is therefore
resolved from that file. An engine-only home has no such file, so of
the home's layers only the environment remains: K01, K04, K07 and K10
show this. The engine-only home does not close layer 3: a `.env` in the
working directory, the seat's own worktree, would still supply a key.
The pre-check below asserted that none existed here. A key file in
the engine-only home, whether `.credentials.yaml` or `.env`, would be a
credential copy, and no route needs one.

The operator's composed headless configuration holds no
`dsh-mcp-client` row, and no home-level `~/.dsh/cordis.patch.yml`
exists. No ambient MCP is configured there today. The ambient sources
below were planted.

### Method

The method is the same as U0's: the same sentinel and four states, with ambient
sentinels planted in the disposable HOME's default `~/.dsh`. The home-level
row (`amb_home`) sits in `$DSH_HOME/cordis.patch.yml`, and the profile-level row (`amb_profile`)
in `profiles/headless/cordis.patch.yml`. The differences follow.

- **Environment, with no key on any argv.** U0's `env -i NAME=value`
  would put a key on argv, so it was not used. `bin/clean-exec.sh` runs as a
  subshell and works in four steps:
  1. It unsets every exported variable and function except `HOME`, `USER`,
     `LOGNAME`, `SHELL`, `TERM`, `LANG`, `PATH`, `TMPDIR`, `VOLTA_HOME`,
     `DSH_HOME` and `DISABLE_AUTOUPDATER`.
  2. It sets those names to fixed, non-secret values.
  3. For K-a and the positive control only, it runs `set -a; .
     ~/.dsh/<file>.env; set +a`.
  4. It `exec`s dsh.

  The child's exported names were checked by name only (`compgen -e`). They were the
  allowed names plus the file's own: `DEEPSEEK_API_KEY`;
  `DASHSCOPE_API_KEY` and `DASHSCOPE_BASE_URL`; `OPENROUTER_API_KEY`.
  The launching shell itself exports five provider keys, and none reached
  a cell except through its env file.
- **Files sourced:** deepseek used `deepseek.env`, dashscope used `dashscope.env`, and both meta
  routes used `openrouter.env`.
- **Homes, per family:**
  - a disposable HOME (`dsh/userhome-<family>`), whose default `~/.dsh` dsh
    scaffolded itself. It carries the home-level sentinel row, and a profile patch
    with the route's provider entry plus the profile-level sentinel row;
  - an engine-only `DSH_HOME` (`dsh/home-engine-<family>`), also scaffolded by
    dsh (a `package.json` naming `@deepseek-ai/dsh-base` and
    `@deepseek-ai/dsh-headless`, an empty root `cordis.yml`). Its profile patch
    carries only the route's provider entry, verbatim from the operator's
    profile (deepseek: `[]`).

  `--dump-config` confirmed the composition for every family before any
  model call. The engine-only home plus the engine overlay composes
  `serverName: engine` alone. The default home composes `amb_home` and
  `amb_profile`.
- **Pre-check, before every cell:** `bin/cell.sh` asserted that none of
  `$DSH_HOME/.credentials.yaml`, `$DSH_HOME/.env`, `<cwd>/.env`,
  `$HOME/.env`, `$HOME/.dsh/.env` and `$HOME/.dsh/.credentials.yaml` exists.
  The workdir is a copy of U0's `dsh/proj`: a git repo with one empty commit and no `.env`.
- **Order:** on each family, K-b ran first, on homes that had never been given a key. K-a ran
  next, then the positive control.
- **Listing:** no capture proxy was used (the upstreams are keyed HTTPS). Each request tool
  list is dsh's own transcript record, `request/header`, for the first request.
  On U0's D03 transcript, that record lists exactly the 26 names U0's proxy
  captured on the wire.
- **Task:** `Call each available tool whose name contains u0_ exactly
  once, with no arguments. Then reply done.` Each cell ran under `timeout 240`.
- **Reading the operator's configuration.** `dsh --dump-config` is not
  read-only. The launcher's `prepareProfile` rewrites
  `profiles/<name>/cordis.yml` before it composes the dump. Under a read-only bind it failed with `EROFS`.
  The operator's dump therefore ran inside bubblewrap with `--overlay-src
  ~/.dsh --tmp-overlay ~/.dsh`, so writes landed in a discarded tmpfs. It ran in a scrubbed
  environment with no key, and its output passed a key-shape redactor before it was
  written. There were four redactions, all long package names. The listing of `~/.dsh` (paths,
  mtimes and sizes) was identical before and after.

### Argv shape

```
clean-exec.sh <disposable HOME> <engine-only DSH_HOME | -> <~/.dsh/<file>.env | -> -- \
  dsh --profile headless --patch <overlay> "<task>"
```

The only credential-related argument is the env file's path. No value is ever on
argv. The overlay mirrors the driver's rows:

- `agent-default-model`, restating `provider` and `model`;
- `session-persistence-jsonl`, with `compression: none` and `packChunks: false`;
- for K-a and K-b, the engine's `dsh-mcp-client` row as an `insert` patch (`serverName: engine`, tool
  `u0_engine_probe`).

### Cells

The configurations are:

- **K-b:** engine-only `DSH_HOME` plus the engine overlay, with no key anywhere;
- **K-a:** the same, with the key in the process environment only;
- **PC:** the disposable default `~/.dsh` (no `DSH_HOME`), with no engine row and the key in the environment.

The request tool list holds the MCP names from the transcript's
`request/header`. Every list also carries 25 native tools, among them
`web_fetch` and `web_search`.

| Cell | Family | Config | home-level | profile-level | engine | Model call | Request tool list (MCP) | Exit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| K01 | deepseek | K-b | not started | not started | listed | refused before any provider request: `MISSING_CREDENTIAL` | `mcp__engine__u0_engine_probe` (26 tools) | 1 |
| K02 | deepseek | K-a | not started | not started | answered | succeeded (2 agent requests) | `mcp__engine__u0_engine_probe` (26) | 0 |
| K03 | deepseek | PC | answered | answered | none | succeeded (2) | `mcp__amb_home__u0_home_probe`, `mcp__amb_profile__u0_profile_probe` (27) | 0 |
| K04 | dashscope | K-b | not started | not started | listed | refused: `MISSING_CREDENTIAL` | engine only (26) | 1 |
| K05 | dashscope | K-a | not started | not started | answered | succeeded (2) | engine only (26) | 0 |
| K06 | dashscope | PC | answered | answered | none | succeeded (2) | both ambient (27) | 0 |
| K07 | meta | K-b | not started | not started | listed | refused: `MISSING_CREDENTIAL` | engine only (26) | 1 |
| K08 | meta | K-a | not started | not started | answered | succeeded (2) | engine only (26) | 0 |
| K09 | meta | PC | answered | answered | none | succeeded (2) | both ambient (27) | 0 |
| K10 | meta-contributor | K-b | not started | not started | listed | refused: `MISSING_CREDENTIAL` | engine only (26) | 1 |
| K11 | meta-contributor | K-a | not started | not started | listed | failed upstream: 6 attempts, `503 service_overloaded` ×4, `504` ×2 | engine only (26) | 1 |
| K11r | meta-contributor | K-a, rerun | not started | not started | listed | failed upstream: 6 attempts, 503 ×5, 504 ×1 | engine only (26) | 1 |
| K11s | meta-contributor | K-a, second rerun | not started | not started | listed | failed upstream: 6 attempts, 503 ×3, 504 ×3 | engine only (26) | 1 |
| K12 | meta-contributor | PC | listed | listed | none | failed upstream: 6 attempts, 503 ×4, 504 ×2 | both ambient (27) | 1 |
| K11t | meta-contributor | K-a, third rerun | not started | not started | answered | succeeded (2) after 4 failed attempts inside dsh's retry policy (503 ×3, 504 ×1) | engine only (26) | 0 |
| K12r | meta-contributor | PC, rerun | answered | answered | none | succeeded (2) | both ambient (27) | 0 |

The K-b refusals, verbatim (stderr, exit 1):

```
dsh: MISSING_CREDENTIAL: llm-deepseek: no API key for provider route "deepseek-official"; store DEEPSEEK_API_KEY through the credentials service (the web Models page writes it), or export DEEPSEEK_API_KEY in the launching environment
dsh: MISSING_CREDENTIAL: llm-pi-ai: no credential for provider route "dashscope"; its profile resolves DASHSCOPE_API_KEY, which is not set — store DASHSCOPE_API_KEY through the credentials service (the web Models page writes it) or export it, and remove apiKeyEnv only if this provider should authenticate from pi-ai's own environment discovery
dsh: MISSING_CREDENTIAL: llm-pi-ai: no credential for provider route "meta"; its profile resolves OPENROUTER_API_KEY, which is not set — […same wording…]
dsh: MISSING_CREDENTIAL: llm-pi-ai: no credential for provider route "meta-contributor"; its profile resolves OPENROUTER_API_KEY, which is not set — […same wording…]
```

In each K-b transcript, MCP startup comes before the refusal: the engine is
listed and the `request/header` is composed. The first `assistant/attempt`
then finishes with code `MISSING_CREDENTIAL` and carries no usage chunk. The
refusal comes from the credential seam before any provider request. The
credentials service had no source: no environment value, no
`.credentials.yaml`, no `.env` in either place. The key in each K-a cell
therefore came from the process environment and not from an ambient file.

**Request configuration** comes from `request/header.config`:

- deepseek ran with the adapter's `reasoningEffort: high` (no effort pinned);
- the meta routes ran with the row's `reasoningEffort: xhigh`;
- dashscope ran with no effort.

Muse Spark's reasoning arrives encrypted, so stderr is empty on the meta
cells, as the operator's profile records.

**Engine served.** In every completed K-a cell (K02, K05, K08, K11t), the engine
sentinel logged `start`, `initialize`, `tools/list` and one
`tools/call(u0_engine_probe)`. The model then replied `done`. No sentinel
log for `home` or `profile` was written in any K-a or K-b cell.

### Verdicts

- **deepseek:** qualified. The key is honoured from the environment and ambient
  MCP stays excluded (K02; K01 refuses without the key; the K03 positive control
  answers both ambient sentinels).
- **dashscope:** qualified. The key is honoured from the environment and ambient
  MCP stays excluded (K05; K04; K06).
- **meta:** qualified. The key is honoured from the environment and ambient MCP
  stays excluded (K08; K07; K09).
- **meta-contributor:** qualified. The key is honoured from the environment and ambient
  MCP stays excluded (K11t; K10 refuses without the key; the K12r positive control answers both
  ambient sentinels). Before that, the route's upstream was overloaded. K11, K11r, K11s and K12
  each ran six attempts under dsh's retry policy and ended with exit 1, on
  OpenRouter's `503 Provider returned error`, `provider_name: Meta`,
  `provider_error_code: service_overloaded`, `Retry-After: 60`, and on `504`.
  In those four cells no ambient sentinel started, and the engine (K11, K11r and K11s) or both
  ambient sentinels (K12) were listed. The provider's 503 also indicates
  that OpenRouter accepted the environment key and forwarded the request.
  This is inferred from the error's shape: OpenRouter refuses an unauthenticated
  request itself, and it never reaches a provider.

### Credential handling

- Keys reached dsh only by sourcing the operator's existing env file into the
  dsh child's environment, inside the `clean-exec.sh` subshell:
  `deepseek.env`, `dashscope.env` and `openrouter.env`. `meta.env` and
  `spark.env` were never sourced.
- No env file and no key value was copied anywhere: no file, home, overlay, log or
  note. No key was ever on argv. Each cell's `argv.txt` holds only the
  env file's path.
- No key value was printed, echoed, cat'd or grepped. Env files were inspected
  by variable name only (`sed -n 's/=.*//p'`). The operator's
  `.credentials.yaml` was inspected by key name only. Profile files were
  screened for key shapes by line number before being read.
- The operator's `~/.dsh`, `~/.claude*` and `~/.codex` were not modified. The
  operator's dsh configuration was read inside a discarded overlay.
- **Key-shape scan.** This ran on 2026-10-06 after the last cell, and again after both
  deliverables were written, using `bin/scan.sh`. It runs `grep -rlE
  'sk-[A-Za-z0-9_-]{16,}|[A-Za-z0-9_-]{32,}'` over all of
  `/var/tmp/s2-u0c`, with symlinks not followed. Each match is classified by shape and never printed. Of
  333 files, 101 hold a key-shaped string, and none is a credential:
  - **0** strings carry the `sk-` prefix that DeepSeek, Model Studio and OpenRouter
    keys carry.
  - **164** are UUIDs (session, retry and anonymous-user ids).
  - **7** are pure hex: five git object ids in the copied project, and the
    sentinel's sha256 in `bin/build-json.sh` and the observations' `u0c` block.
  - **16** are mixed-case strings without separators, all at the `tool-call` `id`
    and `responseId` fields of the four completed meta transcripts. These are
    provider-issued ids.
  - **283** are identifiers joined by `_` or `-`: tool names, row and package
    ids, session directories and paths.
  - **1** binary match is dsh's shipped `node-addon-require-builtin` prebuilt
    addon, which dsh caches into `TMPDIR`. It is byte-identical to the shipped file.

  No exact-value scan was run. Reading a key into any process other than the
  dsh child is outside the credential rules.

### Limitations

- One Linux host, measured once. No macOS.
- The request tool list is dsh's transcript record, not a wire capture.
  It agreed with the wire on U0's D03.
- Only cold `headless` cells were run. dsh has no resume shape, as in U0.
- Each family ran one model: `deepseek-flash`, `qwen3.8-flash`,
  `muse-spark-1.3` and `muse-spark-1.3-contributor`. Other models on the same
  route share its provider entry, key and endpoint, but they were not run.
- A plugin bundle layer was not planted, as in U0.
- The route rows sat in the engine-only home's profile layer, as in U0's D03.
  Production folds validated route rows into the seat's one `--patch`
  overlay instead, and that placement was not run here. Both compose after
  the shipped bundles and neither is ambient.

### Model calls

- **Keyless, K-b:** 4 invocations, each refused before any provider request.
- **Completed:** 6 keyed invocations on the first three families (K02, K03, K05, K06, K08, K09), each with 2
  completed agent requests. These totals are summed from the transcripts' `usage`:

  | Family | Requests | Input tokens | Output tokens | Cache-read tokens |
  | --- | --- | --- | --- | --- |
  | deepseek | 4 | 10,494 | 238 | 18,816 |
  | dashscope | 4 | 13,623 | 570 | 16,384 |
  | meta | 4 | 20,184 | 653 | 10,308 |

  No cost is reported by dsh (`cost: null`), and none is estimated here.
- **Session titles:** dsh also issues one session-title request per invocation on the
  same route. A provider-sourced title was recorded only on deepseek
  (K02, K03). On the pi-ai routes only the fallback title was recorded.
- **meta-contributor:** 4 requests completed (K11t and K12r), 22,938 input, 450
  output and 7,409 cache-read tokens. A further 28 attempts failed upstream with no tokens
  reported: 6 each in K11, K11r, K11s and K12, and 4 in K11t before it completed.
  In total there were 12 keyed invocations: 8 completed (K02, K03, K05, K06, K08, K09,
  K11t, K12r) and 4 failed upstream. They made 16 completed agent requests.

### U0c addendum: the `spark` route (2026-10-07)

U0 measured the engine-only `DSH_HOME` only on the `spark-glm` route, so the
earlier attribution of `spark` to U0 D03 and D04 was the controller's error.
These three cells measure `spark` (provider `spark`, model `qwen3.8-flash`,
the operator's headless provider entry verbatim) with the same kit, homes,
sentinels, task and credential handling as K01 to K12r. Measured on
2026-10-07 from 11:34:51Z to 11:35:27Z, on the same Linux host. The route's
model server, SGLang on `spark:30000`, refused every connection during the
measurement, while `spark-glm`'s vLLM on `spark:8888` answered.

| Cell | Config | home-level | profile-level | engine | Model call | Request tool list (MCP) | Exit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| S01 | K-b | not started | not started | listed | refused before any provider request: `MISSING_CREDENTIAL` for route `spark` (`SPARK_API_KEY`) | `mcp__engine__u0_engine_probe` (26 tools) | 1 |
| S02 | K-a, `spark.env` sourced | not started | not started | listed | failed: `TRANSPORT: Connection error` after 6 attempts (5 retries) | `mcp__engine__u0_engine_probe` (26) | 1 |
| S03 | PC, `spark.env` sourced | listed | listed | none | failed: `TRANSPORT: Connection error` after 6 attempts | both ambient (27) | 1 |

**Verdict: partial.** Under the engine-only home no ambient sentinel started
(S01, S02) while the default home loads both (S03), and the key reaches dsh
only from the environment (S01 refuses without it; S02 passes credential
resolution and fails only at transport). The engine's tool call was not
observed, because the route's model server was down. By operator ruling
(2026-10-07) the `spark` route is therefore not qualified for serving: dsh
refuses tool servers on it until a cell observes the engine's tool call.
`spark-glm` stays qualified by U0 D03 and D04.

Credential handling as for K01 to K12r: `spark.env` was sourced only into
the scrubbed child environment; no value was copied, put on argv or
printed. A key-shape scan of the three cells' results and transcripts found
no `sk-` string.
