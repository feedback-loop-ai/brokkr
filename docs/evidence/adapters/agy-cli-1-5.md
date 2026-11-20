# Antigravity CLI 1.5: the headless turn, its refusals, its switches and its MCP isolation, measured by hand (#1200)

Issue #1200. This record backs an `agy` adapter and `brokkr driver agy` (decision 0075,
level 1 and after). `brokkr probe harness` knows only claude, codex and dsh,
so every figure here was measured by hand. The recorded stdout and stderr of
each turn sit beside this file in [`agy-cli-1-5/`](agy-cli-1-5/), byte for
byte as the CLI wrote them.

## Provenance

| Figure | Source |
| --- | --- |
| Every live turn below | controller, 2026-11-20, Linux workstation |
| The flag list | `agy --help`, recorded as `help.txt` |
| The model list | `agy models`, recorded as `models.txt` |
| The environment names | `strings` of the installed 1.5.13 binary, 2026-11-20 |
| Statements marked "docs" | <https://antigravity.google/docs/cli/reference> and the pages it links, read 2026-11-20; not measured |

No implementing seat can re-run these figures, because none has a signed-in
`agy`.

## Host and versions

| Item | Value |
| --- | --- |
| Host | Linux x86-64, Ubuntu, `kernel.apparmor_restrict_unprivileged_userns = 1`. macOS: pending |
| Antigravity CLI | 1.5.13 (`version.txt`), one native binary of 270,103,240 bytes that Google's installer (`https://antigravity.google/cli/install.sh`, SHA-512 checked against its manifest) put at `~/.local/bin/agy` |
| Account | a personal Google account on the Google AI Ultra plan, signed in through the browser; the login is one file, `~/.gemini/antigravity-cli/antigravity-oauth-token`, mode 0600 |
| Working directory | a scratch directory under `/var/tmp/probe-agy/` for each turn |
| Model | `--model gemini-3.8-flash-low` for every turn, except the named-model and model-refusal turns |
| HOME | the operator's own for the turns of the first table; a scratch HOME holding only a link to the login file for every isolation, web and hook turn |

`turns.log` is the controller's own list of each turn's name, exit code and
argv. It is not CLI output.

## The turns

Each turn ran `agy --output-format stream-json --print-timeout 150s` plus the
flags named below, with stdin from `/dev/null` unless a prompt was piped in.
The prompt is the value of `-p` unless the row says otherwise.

| File | Flags beyond the common ones | Exit | What it shows |
| --- | --- | --- | --- |
| `tool-less-turn.ndjson` | `-p <prompt>` | 0 | `init`, `step_update` rows, one `result` reading `READY` |
| `stdin-pipe-empty-p.ndjson`, `.stderr` | `-p ""`, prompt piped on stdin | 1 | an `ERROR` result, "empty prompt": a pipe is not read as the prompt |
| `p-takes-next-flag.stderr` | `-p --input-format stream-json` | 2 | `-p` takes the next argument as its prompt; stdout empty |
| `stdin-stream-json.ndjson` | `--input-format stream-json`, no `-p`, one `{"event":"user",…}` line on stdin | 0 | the prompt is read from stdin; closing stdin ends the session |
| `stdin-stream-json-two-turns.ndjson` | the same, two lines | 0 | one `result` per line; the second turn recalls the first |
| `named-model-{neon,carbon,hydrogen,argon}-base.ndjson` | `--model gemini-4-<name> --effort low` | 0 | init `model` is the base slug as requested |
| `base-slug-no-effort.ndjson` | `--model gemini-4-neon` | 0 | a base slug with no effort is accepted |
| `unknown-model.ndjson`, `.stderr` | `--model NoSuchModel` | 1 | an `ERROR` result, "invalid model selection", listing the models; no turn ran |
| `bad-effort.ndjson`, `.stderr` | `--effort bogus` | 1 | an `ERROR` result naming the valid efforts |
| `effort-unsupported-for-model.ndjson`, `.stderr` | `--model gemini-4-carbon --effort xhigh` | 1 | "gemini-4-carbon has no \"xhigh\" effort (available: low, medium, high)" |
| `effort-max.ndjson`, `.stderr` | `--model gemini-4-neon --effort max` | 1 | "gemini-4-neon has no \"max\" effort (available: low, medium, high, xhigh)" |
| `effort-conflicts-with-slug.ndjson`, `.stderr` | `--model gemini-4-neon-high --effort low` | 1 | "--model gemini-4-neon-high conflicts with --effort=low" |
| `bad-flag.stderr` | `--no-such-flag` | 2 | stdout empty; stderr "flags provided but not defined" and the usage |
| `tools-turn.ndjson` | `--dangerously-skip-permissions` | 0 | `tool` steps for `write_file` and `run_command`; init `permission_mode` is `always-proceed` |
| `denied-command-default.ndjson`, `.stderr` | none | 0 | `touch` was not run; `result.denied_actions` has one entry; `response` is empty |
| `denied-command-accept-edits.ndjson`, `.stderr` | `--mode accept-edits` | 0 | the file write ran, `touch` did not; the `touch` step reads `DONE` with no error |
| `allow-list-allowed.ndjson` | `--settings '{"permissions":{"allow":["command(ls)"]}}'` | 0 | `ls -1` ran |
| `allow-list-unlisted.ndjson`, `.stderr` | the same | 0 | `uname -s` did not run; `denied_actions` |
| `deny-over-skip-permissions.ndjson` | `--dangerously-skip-permissions --settings '{"permissions":{"deny":["command(uname)"]}}'` | 0 | the deny rule refused `uname -s` as a tool error and the turn went on to answer |
| `settings-unknown-key.stderr` | `--settings '{"mcpServers":{}}'` | 1 | stdout empty; `Error: --settings: unknown setting "mcpServers"` |
| `settings-unparsable-rule.ndjson`, `.stderr` | `--settings '{"permissions":{"allow":["command uname"]}}'` | 0 | the unparsable rule was ignored without a word, and `uname -s` was denied |
| `print-timeout.ndjson`, `.stderr` | `--print-timeout 8s`, a `sleep 120` command | 3 | an `ERROR` result, "print timeout after 8s"; stderr ends with an `AGY_ERROR:` JSON line whose `status` is `DEADLINE_EXCEEDED` |
| `signed-out.ndjson`, `.stderr` | a scratch HOME with no login | 1 | stderr prints a sign-in URL and waits 60 seconds; then an `ERROR` result, "authentication failed or timed out" |
| `resume-cold.ndjson`, `resume-warm.ndjson`, `resume-warm-2.ndjson` | the second and third with `--conversation <id>` | 0 | the same `conversation_id` each time; the warm turn recalls the cold turn's word |
| `resume-other-cwd.ndjson` | `--conversation <id>` from another directory | 0 | the conversation is found from a different working directory |
| `resume-unknown-id.ndjson`, `.stderr` | `--conversation 00000000-0000-0000-0000-000000000000` | 0 | a **new** conversation with a new id; one stderr line, `warning: conversation "…" not found` |
| `mode-auto.ndjson`, `.stderr` | `--mode auto` | 0 | stderr: "--mode auto is not available because the terminal sandbox is not available on this host; continuing in the default mode" |
| `sandbox-skip-permissions.ndjson` | `--sandbox --dangerously-skip-permissions` | 0 | no warning; a write outside the workspace succeeded, `ps -e` saw the host's processes, and `curl` reached the network |
| `ambient-rules-files.ndjson` | none; the directory holds an `AGENTS.md` and a `GEMINI.md` | 0 | the answer quotes both files: the workspace's rules files are loaded |

## Facts

- **The stream is agy's own shape, not Claude Code's.** Each line is one JSON
  object with a top-level `event`:
  - `init`, once, with `conversation_id` and an `init` object of `model`,
    `cwd`, `tools` (60 names on this account) and `permission_mode`.
  - `step_update`, with `conversation_id`, `step_index`, `state` (`ACTIVE`,
    `DONE` or `ERROR`) and `step_type` (`user_input`, `agent_response` or
    `tool`). An `agent_response` carries `text_delta`, and its `DONE` row a
    `usage` object and `duration_seconds`. A `tool` step carries `tool_name`
    and `tool_info` (`name`, `parameters`, and `error` with `type` and
    `message` on a failure).
  - `result`, once per turn, with `conversation_id`, `status`, `response`,
    `duration_seconds`, `num_turns`, `usage`, and on a failure `error` and
    `finished_reason`. A soft-denied tool adds `denied_actions`.
- **Usage is tokens, with no price.** `usage` holds `input_tokens`,
  `output_tokens`, `thinking_tokens`, `cache_read_tokens` and `total_tokens`.
  No cost field and no cache-write count exist. On a resumed conversation
  `num_turns` counts every turn so far, while `usage` is the invocation's own:
  8,867 input tokens cold, then 11,121 and 13,368 on the two resumes.
  `cache_read_tokens` exceeded `input_tokens` on `resume-warm-2` (13,872
  against 13,368), so the two are not a part and its whole.
- **A soft denial is a success by every outer sign.** A tool that needs an
  approval nobody can give in headless mode is not run. The turn stops there,
  `status` is `SUCCESS`, the exit code is 0 and `response` is empty. The signs
  are `result.denied_actions` (`[{"action":"command","display_name":"Bash"}]`)
  and one stderr line beginning `jetski: no output produced`. The denied
  step's own `state` is not reliable: it read `ERROR` with a message in
  `denied-command-default` and `DONE` with no error in
  `denied-command-accept-edits`, `allow-list-unlisted` and
  `settings-unparsable-rule`.
- **A deny rule is an ordinary tool error.** It refuses under
  `--dangerously-skip-permissions` too, with "Matches user-configured deny
  rule", and the turn continues. `denied_actions` is absent.
- **The served model is checked against the request.** An unknown `--model`,
  an unknown `--effort`, an effort the model does not have and a slug that
  already names an effort beside a different `--effort` each exit 1 with an
  `ERROR` result before any turn. Nothing falls back. Init's `model` repeats
  the slug as requested; it does not show the effort.
- **Models.** `models.txt` lists 27 slugs. The four Gemini 4 models take a base
  slug and `--effort`: `gemini-4-neon` and `gemini-4-hydrogen` take low,
  medium, high and xhigh; `gemini-4-carbon` and `gemini-4-argon` take low,
  medium and high. No listed model takes `max`. The list also serves
  `gemini-3.8-flash`, three `gemini-pro-orchestrator` slugs,
  `claude-opus-5-5`, `claude-sonnet-5-5` and `gpt-oss-120b`.
- **The prompt.** `-p` takes the prompt as its value, where every process on
  the host can read it. `--input-format stream-json` without `-p` reads one
  `{"event":"user","message":{"content":"…"}}` line per turn from stdin
  instead, and ends when stdin closes.
- **Permissions ride `--settings`.** The flag is in `--help` and not in the
  docs' flag table. It takes a JSON object of `settings.json` keys for one
  session and saves nothing: `permissions` and `allowNonWorkspaceAccess` were
  accepted, `mcpServers` was refused as an unknown setting. A rule is
  `action(target)`; `command(ls)` matched `ls -1`, and `mcp(<server>/*)`
  matched a server's tools. A rule the CLI cannot parse is ignored without a
  word.
- **Modes.** With `--mode accept-edits`, a file write inside the workspace ran
  and a shell command was soft-denied. With no flag a shell command was
  soft-denied; no write was tried. Init reported `request-review` in both.
  `--mode auto` needs the terminal sandbox.
- **The sandbox is not available on this host.** `--mode auto` says so.
  `--sandbox` says nothing and restricts nothing. So no native write, store
  read or process read was measured under it. (docs: the Linux sandbox uses
  kernel namespaces, and a host that blocks unprivileged user namespaces has
  none.)
- **Refusals.**
  - Signed out: 60 seconds of waiting on stderr, then exit 1 and an `ERROR`
    result, "authentication failed or timed out". Stdout holds only that
    result. The stderr text holds a sign-in URL.
  - A bad flag: exit 2, stdout empty.
  - A print timeout: exit 3, an `ERROR` result, and an `AGY_ERROR:` line.
- **Resume.** `--conversation <id>` continues a conversation from any
  directory. An id that does not exist starts a new conversation with exit 0;
  only the result's different `conversation_id` and one stderr warning say so.
- **State lives under HOME.** `~/.gemini/antigravity-cli/` holds the login
  file, `conversations/<id>.db`, and for each conversation
  `brain/<id>/.system_generated/logs/transcript.jsonl` and
  `transcript_full.jsonl`. No flag switches persistence off. A scratch HOME
  that holds a link to the login file runs signed in, and every state file
  lands in the scratch HOME.
- **Environment names in the binary:** `GEMINI_API_KEY`, `GOOGLE_API_KEY`,
  `GOOGLE_GEMINI_BASE_URL`, `GOOGLE_GENAI_USE_VERTEXAI`,
  `GOOGLE_CLOUD_PROJECT` and `GOOGLE_APPLICATION_CREDENTIALS`. None was
  exercised. (docs: only `GEMINI_API_KEY` is read, and only when
  `modelProvider` is `gemini`; `AGY_ADC_AUTH=true` turns on Application
  Default Credentials.)
- **Updates.** The installer says the CLI "automatically self-updates in the
  background during regular runs". The binary's size, date and version did not
  change during this record, and `updater/update_status.json` read "Already on
  the latest version." No switch that pins a version was found.

## MCP isolation (SI2)

`sentinel.py` is a stdio MCP server that appends its scope's label to a log
when it starts and when its tool is called. One was planted in each place the
CLI reads servers from. Each turn's `.starts` file is that log; a turn that
started none has no such file.

| Scope | Where it was planted |
| --- | --- |
| user | `$HOME/.gemini/config/mcp_config.json` |
| plugin | a plugin installed with `agy plugin install`, at `$HOME/.gemini/config/plugins/sentinel-plugin/` |
| workspace | `<cwd>/.agents/mcp_config.json` |
| parent | `.agents/mcp_config.json` at the git root above the working directory |
| wsplugin | `<cwd>/.agents/plugins/sentinel-wsplugin/` |
| adddir | `.agents/mcp_config.json` in a directory named by `--add-dir` |

| Turn | HOME | Working directory | Started |
| --- | --- | --- | --- |
| `iso-01-positive-control` | a scratch HOME with the user and plugin sentinels | the workspace, with `--add-dir` | all six |
| `iso-02-engine-home-workspace-cwd` | a scratch HOME with no MCP configuration | the workspace | workspace, parent, wsplugin |
| `iso-03-engine-home-neutral-cwd-add-dir` | the same | an empty directory, the workspace by `--add-dir` | workspace, parent, wsplugin |
| `iso-04-engine-home-neutral-cwd` | the same | an empty directory | none |
| `iso-05-above-git-root` | the same | a subdirectory of a git repository that holds no `.agents`; sentinels one and two directories **above** its root | none |
| `iso-06-parent-without-git` | the same | a directory in no git repository; a sentinel in an ancestor | none |
| `iso-10-neutral-cwd-non-workspace-access` | the same | an empty directory, with `--settings '{"allowNonWorkspaceAccess":true,"permissions":{"allow":["write_file(<workspace>)","command(pwd)"]}}'` | none; the turn wrote a file in the workspace and ran `cd <workspace> && pwd` |
| `iso-07-engine-carriage-cold` | a scratch HOME whose `mcp_config.json` names an engine sentinel | a clean repository, with `--settings '{"permissions":{"allow":["mcp(sentinel-engine/*)"]}}'` | engine, and its tool answered |
| `iso-08-engine-carriage-no-rule` | the same | the same, no rule | engine started; its call was soft-denied |
| `iso-09-engine-carriage-resume` | the same | `--conversation` of `iso-07`, with the rule | engine, and its tool answered |

- **A scratch HOME shuts out the user's and the user's plugins' servers.** No
  flag, variable or setting does.
- **The workspace's servers cannot be shut out while the workspace is the
  working directory or an added directory.** The CLI reads
  `.agents/mcp_config.json` and `.agents/plugins/*/` from the working
  directory and each added directory, and from every directory above each up
  to its git root, and starts what it finds. It did so in a workspace no
  `trustedWorkspaces` entry names. It reads nothing above the git root, and
  nothing above a directory that is in no repository.
- **Two shapes started no planted sentinel:** a scratch HOME with a workspace
  that holds no `.agents` MCP file and no `.agents/plugins` up to its git root
  (`iso-05`, `iso-06`), and a scratch HOME with a working directory outside
  the workspace, reaching the workspace through `allowNonWorkspaceAccess` and
  path rules (`iso-10`).
- **Carriage.** A server in the scratch HOME's `mcp_config.json` starts cold
  and on a resume. Its tools are not declared to the model directly: the
  model reads a schema file under `$HOME/.gemini/antigravity-cli/mcp/<server>/`
  and calls the built-in `call_mcp_tool` with `ServerName`, `ToolName` and
  `Arguments`. A call needs an `mcp(<server>/<tool>)` allow rule; without one
  the server still starts and the call is soft-denied.
- `--settings` cannot carry a server.

## The state directory and a command's environment

| File | Environment | What it shows |
| --- | --- | --- |
| `env-ANTIGRAVITY_EXECUTABLE_DATA_DIR.ndjson` | that name set to an empty directory | the turn ran; the directory stayed empty and the state went to `$HOME/.gemini` |
| `env-ANTIGRAVITY_CONFIG_REL_DIR.ndjson` | that name set to `.probe-rel` | the same; no `.probe-rel` appeared |
| `env-XDG_CONFIG_HOME.ndjson` | that name set to an empty directory | the same |
| `child-env-scratch-home.ndjson` | a scratch HOME, the operator's `~/.cargo/bin` on `PATH` | a command the agent ran saw `HOME` as the scratch HOME; `cargo` failed with "rustup could not choose a version of cargo to run", and git found no global identity |
| `child-env-scratch-home-tool-homes.ndjson` | the same, plus `CARGO_HOME`, `RUSTUP_HOME` and `GIT_CONFIG_GLOBAL` naming the operator's own | `cargo --version` answered and git found its identity |
| `child-env-linked-home.ndjson` | a scratch HOME that links each top-level entry of the operator's HOME except `.gemini` and `.agents`; the same `PATH` | `cargo --version` answered and git found its identity |

- **HOME is the only measured way to move the CLI's state.** Three names the
  binary holds were tried and none moved it.
- **A command the agent runs inherits that HOME.** A tool that finds its own
  files through HOME loses them, and finds them again when its own variable
  names them.
- **A scratch HOME that links the operator's own entries keeps them.** With
  every top-level entry of the operator's HOME linked into the scratch HOME
  except `.gemini` and `.agents`, `cargo --version` answered and git found its
  identity with no tool variable set (`child-env-linked-home.ndjson`).

## Native tools and hooks

Every turn here ran under the scratch HOME named.

| File | What was planted or passed | What it shows |
| --- | --- | --- |
| `web-search-default.ndjson` | nothing | `search_web` ran with no rule and no prompt |
| `web-search-deny-rules.ndjson` | `--settings '{"permissions":{"deny":["read_url(*)","execute_url(*)"]}}'` | `search_web` ran all the same |
| `hook-deny-search-web.ndjson`, `.hooklog` | `$HOME/.gemini/config/hooks.json` with a `PreToolUse` hook matching `search_web` that prints `{"decision":"deny",…}` (`deny-search.sh`) | the call failed with "tool call denied by pre-tool hook" and the model answered `NO SEARCH` |
| `hook-allowlist-search.ndjson`, `hook-allowlist-others.ndjson`, `hook-allowlist-allowed.ndjson`, `.hooklog` | a `PreToolUse` hook with matcher `""` that allows a fixed list of tool names and denies the rest (`allowlist.py`) | `search_web`, `invoke_subagent` and `write_file` were denied by name; `run_command` passed the hook and then met the permission check |
| `hook-crash.ndjson`, `.hooklog` | a hook that exits 1 and prints nothing (`crash.sh`) | the call failed: `JSON hook "…" failed: command failed: exit status 1` |
| `gate-prior-grant.ndjson`, `.hooklog` | one hook on every tool (`gate.py`): a tool outside its list gets `deny`, a listed tool gets `deny_unless_prior_grant`; `--mode accept-edits --settings '{"permissions":{"allow":["command(ls)"]}}'` | `uname -s` failed as a tool error, "Permission denied for unsandboxed(uname -s)"; `ls -1` ran; the file write ran; `search_web` was denied by name; the turn ended with a full answer and no `denied_actions` |
| `gate-no-rules-write.ndjson`, `.hooklog` | the same hook, `--mode accept-edits`, no rule | the file write inside the workspace ran |
| `ambient-workspace-hook.ndjson`, `.hooklog` | `<workspace>/.agents/hooks.json` whose hook prints `{"decision":"auto_approve"}` (`ws-approve.sh`); a scratch HOME with no hooks | the workspace's hook ran, and `touch` ran with no allow rule |

- **Native web search is on, and no permission rule switches it off.** A
  `PreToolUse` hook in the scratch HOME does: it sees every tool call's name
  before the call and can deny it. A hook that fails denies.
- **One hook can be the whole gate.** `deny_unless_prior_grant` turns what
  the permission check would have asked into a refusal the model sees as a
  tool error, so the turn goes on where a soft denial would have ended it.
  What a rule or the mode already grants still runs.
- **The hook is told the tool's own name, and it is not init's.** A hook that
  allows everything and logs each name (`log-names.py`,
  `hook-tool-names.ndjson`) saw `read_file`, `write_file`, `edit_file` and
  `run_command` for a read, a write, two edits and four shell commands; the
  other turns saw `search_web` and `invoke_subagent`. The stream's
  `tool_name` uses the same names, and `call_mcp_tool` for an MCP call. Init's
  `tools` lists 60 other spellings (`view_file`, `write_to_file`,
  `replace_file_content`, `list_dir`, `grep_search`) that no hook and no
  stream row carried: the model listed, found and searched through
  `run_command`.
- Asked to call `list_dir`, `find_by_name`, `grep_search` and `view_file` by
  name, or else to name its tools, `gemini-4-carbon` called nothing and
  listed fourteen: `run_command`, `manage_task`, `read_file`, `write_file`,
  `edit_file`, `search_web`, `ask_question`, `schedule`, `send_message`,
  `invoke_subagent`, `define_subagent`, `manage_subagents`, `swarm` and
  `run_workflow` (`hook-tool-names-2.json`, recorded with `--output-format
  json`). That is the model's own account, not a measurement of the CLI.
- **A workspace can approve its own commands.** `.agents/hooks.json` in a
  workspace nobody trusted ran in headless mode, and its `auto_approve` let a
  command run that no rule allowed. The hook command itself is the
  workspace's.
- `read_url_content` was never called: asked twice by name, the model
  answered without calling it (`read-url-default`, `read-url-deny-rule`). In
  `url-fetch-default` it called `search_web` instead, and in
  `url-fetch-deny-rule` it ran `curl` through `run_command`.

## Second measurement, for round two (2026-11-20)

Round one's review asked what the record did not say. Same host, binary and
account; every turn ran under a scratch HOME.

### The documents the hook and MCP turns used

They are in [`agy-cli-1-5/documents/`](agy-cli-1-5/documents/), byte for byte
as planted, with the lab's own paths.

| File | Where it was planted | Turns |
| --- | --- | --- |
| `gate.hooks.json` | `$HOME/.gemini/config/hooks.json` | `gate-prior-grant`, `gate-no-rules-write`, `rule-multi-word-prefix` |
| `named-hooks.hooks.json` | the same path | `hook-document-named-hooks` |
| `workspace-planted.hooks.json` | `<workspace>/.agents/hooks.json` | `ambient-workspace-hook` |
| `engine.mcp_config.json` | `$HOME/.gemini/config/mcp_config.json` | `iso-07`, `iso-08`, `iso-09` |

- **`hooks.json` maps a name of the author's choosing to that hook's events.**
  The top-level key is the hook's name (`gate`), and its value holds
  `PreToolUse`, a list of objects each with a `matcher` and a `hooks` list of
  `{"type":"command","command":…,"timeout":…}`. A document whose one name is
  the word `hooks` loads the same way: under it the gate saw `search_web` and
  denied it (`hook-document-named-hooks`).
- **`timeout` is seconds.** (docs: "optional integer seconds, default `30`".)
  No turn here ran a hook past its timeout.
- **The hook's `command` is run as a shell command**, with the hook's JSON on
  its stdin and its decision read from its stdout.

### What ends the walk up from the working directory

Each turn's working directory is a `sub` directory. A sentinel was planted in
`.agents/mcp_config.json` at each place named.

| Turn | Layout | Started |
| --- | --- | --- |
| `git-root-linked-worktree` | a linked worktree, whose `.git` is a file naming the main repository's `worktrees/wt`; sentinels at the worktree's root, in the directory above it, and at the main repository's root | the worktree's root only |
| `git-root-nested-repository` | a repository inside a repository; sentinels at both roots | the inner root only |
| `git-root-empty-dot-git-dir` | a repository; between its root and the working directory, a directory holding an **empty `.git` directory**; sentinels there and at the real root | both: the empty directory did not end the walk |
| `git-root-junk-dot-git-file` | the same, with a **`.git` file** holding one line that names no git directory | only the directory with the file: any `.git` file ended the walk |

- The walk ends at the nearest ancestor that holds a `.git` file, whatever it
  says, or a `.git` directory that git itself would take for a repository. A
  `.git` directory that is not one does not end it.

### A rule of more than one word

`rule-multi-word-prefix`: under `permissions.allow` of `command(git status)`,
with the gate, `git status --short` and `git status` ran and `git log
--oneline -1` was refused as a tool error. A rule's words match the command's
first words, one by one.

### The login file

Each scratch HOME's link to the login file was still a link after every turn.
Whether the CLI, when it refreshes the login, writes through such a link or
replaces it is not measured.

## Not measured

- macOS.
- Any turn on a `GEMINI_API_KEY`, an Application Default Credential or a
  Google Cloud project.
- The terminal sandbox: this host has none, so native write, store read and
  process read under it are unmeasured.
- `read_url_content`, the `browser_*` tools, `generate_image` and what a
  subagent's own tools pass through. The allow-list hook denies each by name,
  but none of them was seen running.
- Whether a `PreToolUse` hook also sees the tool calls of a subagent.
- A self-update: when it happens, and whether a running seat can meet it.
- The `claude-*`, `gpt-oss-*` and orchestrator slugs.
- `agy acp`, the Agent Client Protocol server.
- The format of `transcript.jsonl` beyond its first rows.
- Rules files (`AGENTS.md`, `GEMINI.md`) above the working directory, and any
  switch for them.

## Terms, as the docs state them (not measured)

- <https://antigravity.google/terms>, section 5, for a personal account: "We
  use Interactions to evaluate, develop, and improve Google and Alphabet
  research, products, services and machine learning technologies", and "If
  you don't want your Interactions used in this way, navigate to settings to
  change your preference." In the CLI that preference is `/config`, "Enable
  Telemetry". It has no `settings.json` key and is saved with the account.
- <https://antigravity.google/docs/plans> describes the plans by quota and
  says nothing about training.
- <https://antigravity.google/docs/workspace>: a Google Cloud project, pay as
  you go, runs under the Google Cloud Terms of Service with telemetry "always
  off".
- No page read states that a paid personal plan's interactions are not used
  for training.
- The operator, 2026-11-20: "telemetry is off". That is the account
  preference section 5 names, stated by the operator and not measured: it is
  saved with the account and has no local file (#1200).
