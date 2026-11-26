# Antigravity CLI 1.5: the headless turn, its refusals, its switches and its MCP isolation, measured by hand (#1200)

Issue #1200. This record backs an `agy` adapter and `brokkr driver agy` (decision 0075,
level 1 and after). `brokkr probe harness` knows only claude, codex and dsh,
so every figure here was measured by hand. The recorded stdout and stderr of
each turn sit beside this file in [`agy-cli-1-5/`](agy-cli-1-5/), byte for
byte as the CLI wrote them.

## Provenance

| Figure | Source |
| --- | --- |
| Live turns of the first three measurements | controller, 2026-11-20, Linux workstation (the fourth through eighth measurements carry their own dates, versions and slugs below) |
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
| Antigravity CLI | 1.5.13 (`version.txt`) for everything but the fifth, sixth and seventh measurements, which ran on 1.5.14, and the eighth, which ran on 1.5.17; one native binary of 270,103,240 bytes that Google's installer (`https://antigravity.google/cli/install.sh`, SHA-512 checked against its manifest) put at `~/.local/bin/agy` |
| Account | a personal Google account on the Google AI Ultra plan, signed in through the browser; the login is one file, `~/.gemini/antigravity-cli/antigravity-oauth-token`, mode 0600 |
| Working directory | a scratch directory under `/var/tmp/probe-agy/` for each lab turn (the eighth measurement's two engine seats ran in linked git worktrees) |
| Model | `--model gemini-3.8-flash-low` for the first through seventh measurements, except the named-model and model-refusal turns; the eighth measurement ran on `claude-opus-5-5`, `claude-sonnet-5-5`, `claude-sonnet-5-5-high` and `claude-sonnet-5-5-low` as its section states |
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
  medium and high. In the eighth measurement on 1.5.17, `claude-opus-5-5` and
  `claude-sonnet-5-5` were measured as far as that section goes: each takes a
  base slug and `--effort` `low`, `medium` and `high` (refusing `xhigh` and
  `max`), `claude-sonnet-5-5-low` alone ran the short security set, and each
  ran one boxed work seat at `low` effort on a four-step task through the
  released 0.16.0 engine. No listed
  model takes `max`. The list also serves `gemini-3.8-flash`, three
  `gemini-pro-orchestrator` slugs and `gpt-oss-120b`; `gpt-oss-*` and the
  orchestrator slugs remain unmeasured.
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
  No turn here ran a hook past its timeout; `hook-timeout` in the fourth
  measurement did.
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

## Third measurement, for round four (2026-11-20)

Round three's review found three more cases in the driver's copy of git's
answer to "where does this repository end". Before ruling, the controller
measured the one thing every such copy assumed: which directory agy walks up
from. Same host, binary and account; both turns ran under the operator's own
HOME, which holds no MCP document.

### The walk starts at `$PWD`, not at the directory the process is in

The layout, under one lab directory:

- `real/repo`, a git repository, and `real/repo/sub` inside it. Neither holds
  an `.agents`.
- `logical/sub`, a symbolic link to `real/repo/sub`.
- `logical/.git`, a file holding one line that names no git directory, and
  `logical/.agents/mcp_config.json`, naming a sentinel.

| Turn | Process directory | `PWD` | `init.cwd` | Started |
| --- | --- | --- | --- | --- |
| `pwd-logical` | `real/repo/sub` | `logical/sub` | `logical/sub` | the sentinel at `logical` |
| `pwd-physical` | `real/repo/sub` | `real/repo/sub` | `real/repo/sub` | none |

- **agy takes its working directory from `PWD` where `PWD` names the directory
  the process is in**, and walks up that path's own parents. A parent the
  process directory's resolved path never passes through was read
  (`pwd-logical`, `pwd-logical.starts`).
- With `PWD` set to the resolved path, the same process directory read nothing
  (`pwd-physical`).
- A `PWD` that names some other directory was not measured.

### What this means for a check of the working directory's ancestors

A check that reads the ancestors of the resolved path covers what agy reads
only where the child's `PWD` is that resolved path. Every earlier turn of this
record held: the lab entered each directory by its resolved path.

## Fourth measurement, for level 3 (2026-11-20 and 2026-11-21)

Level 1 seats agy unboxed. Before asking for boxed hands (#1224), the
controller measured whether an agy turn can be left one tool, the engine's.
Same host, binary and account. Every turn ran `agy` by hand, never through
`brokkr driver agy`, with `-p`, no `--mode`, and one allow rule in
`--settings`: `mcp(brokkr/*)` for the first three turns and
`mcp(brokkr/workspace)` for the two notice turns. `hook-timeout` carried
`command(uname)` instead. The model was `gemini-3.8-flash-low`, except
`box-default-notice-carbon`, which ran `gemini-4-carbon`.

The scratch HOME held three things: the login link,
[`documents/box.mcp_config.json`](agy-cli-1-5/documents/box.mcp_config.json)
as `.gemini/config/mcp_config.json`, and
[`documents/box.hooks.json`](agy-cli-1-5/documents/box.hooks.json) as
`.gemini/config/hooks.json`. The MCP document names one server, `brokkr`:
`brokkr hands serve` of the 0.14.0 release for a scratch worktree, with no
network, no binds and no store in its reach. The hook is `box3-gate.py`: it
answers `deny_unless_prior_grant` for `call_mcp_tool` where `ServerName` is
`brokkr`, and `deny` for every other tool, by name. Each `.hooklog` is that
gate's own list of the calls it saw.

| File | What was asked | What it shows |
| --- | --- | --- |
| `box-engine-workspace-tool.ndjson`, `.hooklog` | three commands through the workspace tool, then `uname -s` through the native shell and a file through the native file tool | the three ran as `call_mcp_tool` with `ServerName` `brokkr`, `ToolName` `workspace` and the command in `Arguments.command`; `ls /home \| wc -l` answered 0; `run_command` and `write_file` failed "tool call denied by pre-tool hook"; the result is `SUCCESS` with a full answer and no `denied_actions` |
| `box-reach.ndjson`, `.hooklog` | six commands through the workspace tool | `/proc` holds 4 numbered entries and pid 1 is `bwrap`; `ls -a ~` shows an empty directory; `ls /var/tmp/probe-agy` shows the worktree's parent `box3` and nothing else of the lab; `curl` fails "Could not resolve host"; the command runs as uid 1000 in the worktree |
| `box-other-natives.ndjson`, `.hooklog` | ten native tools, each called once by name | the gate saw `manage_task`, `schedule`, `define_subagent`, `run_workflow`, `send_message`, `ask_question`, `edit_file`, `search_web` and `swarm`, and each failed as denied by the hook; the model answered that it has no `browser_get_dom` and called nothing for it |
| `hook-timeout.ndjson`, `.hooklog` | `uname -s` under `command(uname)`, with [`documents/slow.hooks.json`](agy-cli-1-5/documents/slow.hooks.json): `slow.sh` sleeps 20 seconds under a `timeout` of 3 and would then allow | the call failed: `JSON hook "jsonhook__brokkr-gate_PreToolUse_0_0" failed: command failed: signal: killed`; the log holds the hook's start and never its end |
| `box-default-notice.ndjson`, `box-default-notice-carbon.ndjson`, `.hooklog` | read a file and write `result.json`, followed by the engine's own hands paragraph, which names the tool `mcp__brokkr__workspace` | both models wrote the file through `call_mcp_tool`; each first called `read_file` on `<HOME>/.gemini/antigravity-cli/mcp/brokkr/workspace.json` and `list_resources`, both denied, and `call_mcp_tool` once with no arguments, which the server refused |

- **A turn can be left one tool.** With the gate listing `call_mcp_tool`
  alone, every native tool the model called was denied by name as a tool
  error, and the turn went on to an answer. Eighteen native calls reached the
  gate across these turns, under thirteen names: `read_file`, `write_file`,
  `edit_file`, `run_command`, `manage_task`, `schedule`, `define_subagent`,
  `run_workflow`, `send_message`, `ask_question`, `search_web`, `swarm` and
  `list_resources`.
- **The engine's tool is reached through `call_mcp_tool`.** The hook is told
  that name, with the server and the tool in `ServerName` and `ToolName`.
  No call named `mcp__brokkr__workspace` or `mcp_brokkr_workspace`.
- **What the tool's command sees is the box's**, not agy's: a private
  process space, an empty HOME, no network, and of the host's tree only the
  worktree. Agy itself runs on the host, outside the box.
- **A gate that outlives its timeout denies.** Agy kills it and fails the
  call, as it fails the call of a gate that exits 1 (`hook-crash`).
- **The engine's default hands paragraph is enough.** Neither model was told
  `call_mcp_tool`, `ServerName` or the command argument. Agy keeps each
  server's tool schemas as files under the HOME, and both models tried to
  read that file with the native `read_file` first.

Not measured here: any of this through `brokkr driver agy`, whose gate mode
reads the tool's name alone; a box whose reach names a store, a bind or the
network; a boxed turn on Neon, Hydrogen or Argon; and a resume.

## Fifth measurement, on 1.5.14: a store in the box's reach, the driver live, and the self-update (2026-11-21)

**The binary changed under the record.** On 2026-11-21 at 08:41Z `~/.local/bin/agy`
was replaced by 1.5.14 (273,240,776 bytes), by agy's own background updater,
while seats of the #1200 wager were starting. Everything above this section
was measured on 1.5.13. Everything in this section ran on 1.5.14, on the
same host and account. No axis of any declared shape was measured again on
1.5.14 except as this section says.

### The self-update

Each turn ran in a fresh scratch HOME holding only the login link, with the
prompt "Reply with exactly READY" on `gemini-3.8-flash-low`. Each
`.updaterlog` is the `auto_updater` lines of that HOME's `cli.log`, and the
updater's state files the turn left.

| File | Environment | What it shows |
| --- | --- | --- |
| `updater-default.ndjson`, `.updaterlog` | nothing set | `Spawned background update process`; the HOME holds `updater/` and `last_check.timestamp`. A plain turn starts the updater |
| `updater-disabled.ndjson`, `.updaterlog` | `AGY_CLI_DISABLE_AUTO_UPDATE=1` | the same: the updater was spawned. The value `1` does not switch it off |
| `updater-disabled-true.ndjson`, `.updaterlog` | `AGY_CLI_DISABLE_AUTO_UPDATE=true` | `Auto-update disabled via environment variable AGY_CLI_DISABLE_AUTO_UPDATE`; no `updater/` and no `last_check.timestamp` |

So a seat's turn can replace the operator's binary, and it did. The variable
with the value `true` stops that turn from starting the updater. The name is
not in `agy --help`; it was found in the binary's strings. Not measured:
`agy update` itself, the settings member `AutoUpdate`, and whether an update
can land in the middle of a running turn.

### A store in the box's reach

The box is `brokkr hands serve` of the build at 1acdeeaa (#1224, 0.15.1), for
a scratch worktree holding `seed.txt` and a fake store canary at the
production path, `<workdir>/.forge/secrets.env`, mode 0600, one link. No
network and no binds.

| File | What ran | What it shows |
| --- | --- | --- |
| `box-store-drive-control.txt` | no model: [`drive-hands.py`](agy-cli-1-5/drive-hands.py) drives the server over MCP stdio; the reach seals **no** store | `cat .forge/secrets.env` printed the canary. The positive control: without the seal the tool reads it |
| `box-store-drive.txt` | the same drive; the reach seals the store at that path | `cat`: `Permission denied`, exit 1. `ls -la .forge`: the path is a character device `1, 3`, the box's mask. `seed.txt` still reads |
| `box-store.ndjson` | `agy -p` on `gemini-4-hydrogen`, no `--mode`, `--settings` holding `mcp(brokkr/workspace)` alone; the HOME holds [`documents/box-store.mcp_config.json`](agy-cli-1-5/documents/box-store.mcp_config.json) and [`documents/box-store.hooks.json`](agy-cli-1-5/documents/box-store.hooks.json), whose hook is the driver's own gate, `brokkr driver agy -- --gate call_mcp_tool` | through the workspace tool: `Permission denied`, and the masked device in the listing. The native `read_file`: "tool call denied by pre-tool hook: the engine's gate does not offer the tool read_file". The native `run_command`: the same for `run_command`. The canary's value appears nowhere in the stream |

This is the first turn recorded under the driver's own gate rather than
`box3-gate.py`.

### The driver, live

Four runs of round six's story (#1200) with a boxed agy implementer, one per
model, through `brokkr driver agy` of the same build inside the engine. They
are engine runs in scratch journals, not recorded turns: their run ids and
outcomes are on #1224. Observed on the four live processes and each one's HOME:
the argv carried no `--mode` and one `--settings` value,
`{"permissions":{"allow":["mcp(brokkr/workspace)"]}}`; `mcp_config.json`
named one server, `brokkr`; the gate's command ended `--gate call_mcp_tool`.
Gemini 4 Carbon, Hydrogen and Argon each implemented, committed and passed
verify and review with the workspace tool as their only tool; Neon's change
was complete and its run stopped on a verify failure outside its change
(#1239).

## Sixth measurement, on 1.5.14: the short security set, and a command's stdin (2026-11-21)

**Why.** The operator ruled on 2026-11-21 that a harness version the
adapter was not measured on is warned about where the versioning says it is
compatible and refused where it is not (#1240), and that the new version is
measured again. Not every patch gets the full pass: a short security set is
re-run per version, and the full pass per minor line. This is that set, on
1.5.14, so that the axes the declared shapes rest on have one version
between them.

**How.** Eleven turns on `gemini-3.8-flash-low`, each with
`AGY_CLI_DISABLE_AUTO_UPDATE=true`, with the HOME documents and sentinel
servers of the measurements above. `security-set-v14.sh` in the recordings
folder ran ten of them; the resume was run by hand after it, with the cold
turn's arguments and `--conversation`. `turns.log` holds each argv. The
three boxed turns ran under the driver's own gate, built from 1acdeeaa
(`brokkr driver agy -- --gate call_mcp_tool`), with the hands server of the
fourth measurement. Each recording is `<turn>-v14.ndjson`, with
`.starts` for the sentinels that started and `.hooklog` where a hook ran.
`agy --version` printed 1.5.14 after the pass. Every result is the one
1.5.13 gave.

| Turn | What it asks | Result on 1.5.14 |
|---|---|---|
| `iso-01-positive-control-v14` | the operator-like HOME, with `--add-dir` | all six sentinels started |
| `iso-02-engine-home-workspace-cwd-v14` | a scratch HOME, the seeded workspace as the working directory | `workspace`, `parent` and `wsplugin` started; none of the user's |
| `iso-05-above-git-root-v14`, `iso-06-parent-without-git-v14` | a scratch HOME, a clean workspace | none started |
| `iso-07-engine-carriage-cold-v14` | the engine's server in the scratch HOME's `mcp_config.json`, a cold turn | the server started and its tool answered `SENTINEL-NONCE-ENGINE` |
| `iso-09-engine-carriage-resume-v14` | `--conversation` on that conversation | the same conversation; the server started and its tool answered |
| `hook-crash-v14` | a gate hook that crashes, and a web search | `NO SEARCH`: a gate that crashes denies |
| `hook-timeout-v14` | a gate hook that hangs, and `uname -s` | the call failed with the hook killed at its timeout: a gate that hangs denies |
| `box-engine-workspace-tool-v14` | three commands through the workspace tool, then the native shell and the native file tool | the three ran; both natives `REFUSED` |
| `box-reach-v14` | what the box can see | 4 entries in `/proc`, pid 1 is `bwrap`, an empty home, only the worktree's parent, no network |
| `box-other-natives-v14` | nine other native tools by name | each refused by the gate by name; `browser_get_dom` was not offered |

With `box-store` of the fifth measurement, every axis of the boxed shapes
has a measurement on 1.5.14. The launch grammar, the stream's shape and the
refusals of the first measurement were not run again on 1.5.14; the driver
refuses a turn whose stream does not parse as recorded.

### A native command's stdin is a terminal

`command-stdin-v14` asked for one shell command through the native
`run_command`, under `--mode accept-edits` and allowed by `command(sh)`: whether stdin is a terminal, and
what it is. The answer was `STDIN-IS-A-TTY` and `/dev/pts/17`. So a command
an agy seat runs without a box is given a pseudo-terminal, and a command
that reads its stdin to the end waits there until the turn's limit. A boxed
seat's commands run through the hands server and have no terminal.

This cost one arm of the #1200 wager three attempts: `cargo test` reached a
`brokkr-cli` test that reads the process's stdin (#1245) and stayed there
for the seat's two-hour limit each time.

## Seventh measurement, on 1.5.14: level 2 (rejoin, transcript, isolation on rejoin, printed secrets, concurrent rejoins) (2026-11-22 and 2026-11-24)

Two passes measured by the controller on Antigravity CLI 1.5.14 on Linux (#1227,
decision 0075 level 2): the first on 2026-11-22 (15 turns `l2-*`, scripts
`ctl/l2.sh`, recordings `rec/l2/`), and the second on 2026-11-24 (10 turns
`l2b-*` on `gemini-3.8-flash-low`, scripts `ctl/l2b.sh` and `ctl/l2b-b.sh`,
recordings `rec/l2b/`). Each HOME held only the login link, the driver's one
gate hook and the turn's MCP set, and nothing else of the operator's.

### Provenance

| Figure | Source |
| --- | --- |
| First pass (`l2-01` through `l2-15`, `l2-turns.log`) | controller, 2026-11-22, Linux workstation, Antigravity CLI 1.5.14 |
| Second pass (`l2b-01` through `l2b-09`, `l2b-turns.log`, `l2b-facts.log`, `l2b-sentinel-cold.log`, `l2b-sentinel-rejoin.log`) | controller, 2026-11-24, Linux workstation, Antigravity CLI 1.5.14 |
| Six transcripts copied byte for byte from `brain/<id>/.system_generated/logs/transcript.jsonl` | controller (`l2-rejoins.transcript.ndjson`, `l2-neon-effort.transcript.ndjson`, `l2-box.transcript.ndjson`, `l2-killed.transcript.ndjson`, `l2b-two-rejoins.transcript.ndjson`, `l2b-printed-value.transcript.ndjson`) |

| Transcript file | From |
| --- | --- |
| `l2-rejoins.transcript.ndjson` | `l2-01` to `l2-04`: a cold turn and three rejoins, one on another model |
| `l2-neon-effort.transcript.ndjson` | `l2-12`, `l2-13`: `--effort high`, then the rejoin with `--effort low` |
| `l2-box.transcript.ndjson` | `l2-10`, `l2-11`: a boxed seat and its rejoin, with one denied native tool |
| `l2-killed.transcript.ndjson` | `l2-08`, `l2-09`: a turn killed while a command ran, and its rejoin |
| `l2b-two-rejoins.transcript.ndjson` | `l2b-05` to `l2b-08`: two rejoins at once, and a turn after |
| `l2b-printed-value.transcript.ndjson` | `l2b-09`: a command printed the made-up sentinel value `l2b-sentinel-value-7f3a9c2e41d8` (no credential) |

### First pass: rejoin, transcript layout and kill behaviour (2026-11-22)

| Turn | What was done | What happened |
| --- | --- | --- |
| `l2-01-cold`, `l2-02-rejoin` | a code word, then `--conversation <id>` in the same HOME and directory | same `conversation_id` in `init` and `result`, `num_turns` 1 then 2, the word came back |
| `l2-03-rejoin-other-model` | the rejoin naming `gemini-4-neon` where the conversation began on another model | it rejoined; `init.model` is the model the invocation named, and the transcript row says `gemini-4-neon-high` |
| `l2-04-rejoin-other-cwd` | the rejoin from another working directory | it rejoined |
| `l2-05-unknown-id`, `l2-06-malformed-id` | an id the HOME does not hold; a word that is no id | exit 0, a **new** conversation under a new id, and one stderr line `warning: conversation "<id>" not found` |
| `l2-07-rejoin-moved-files` | a fresh HOME given only `conversations/<id>.db` and `brain/<id>/` | it rejoined, with the word |
| `l2-10-box-cold`, `l2-11-box-rejoin` | a boxed seat (gate lists `call_mcp_tool`, the hands server as the one MCP server), then its rejoin | both turns ran the workspace tool; the rejoin kept the word; a native `read_file` the model tried first was denied by the gate and is in the transcript as an `ERROR` row |
| `l2-12-neon-cold`, `l2-13-neon-rejoin` | `--effort high`, then the rejoin with `--effort low` | the rows' `model` reads `gemini-4-neon-high`, then `gemini-4-neon-low` |
| `l2-08-killed`, `l2-09-rejoin-killed` | SIGKILL while a command ran, then a rejoin | the transcript is whole JSON lines; the command's row stays `RUNNING` for good; the rejoin answered with the word and said the command did not finish |
| `l2-14-kill-group` | SIGKILL to agy's process group while `sleep` ran | agy died; **the command lived on**, reparented to init |
| `l2-15-term-agy` | SIGTERM to agy alone | agy exited 1 and the command was gone |

Facts established by the first pass:

- A conversation `<id>` under `.gemini/antigravity-cli/` is `conversations/<id>.db` (mode 0600) and `brain/<id>/`. Its transcript is `brain/<id>/.system_generated/logs/transcript.jsonl` (mode 0644; `transcript_full.jsonl` beside it), one JSON row per step (`step_index`, `source`, `type`, `status`, `created_at`, and by type `content`, `error`, `thinking`, `tool_calls`, `model`, token counts).
- A rejoin works under the engine's HOME, boxed and unboxed, and is confirmed by `init.conversation_id` (an unknown id exits 0 with a new id). The model served is the invocation's, not the conversation's.
- A killed turn leaves whole JSON lines with its last tool row `RUNNING` for good.
- **Level 1 finding (#1245's family):** an unboxed agy seat's command runs in its own session and process group on a pseudo-terminal, so SIGKILL of agy or agy's process group leaves the command alive as an orphan, while SIGTERM to agy ends the command and exits 1. A boxed seat's commands run through the hands server and are not affected.

### Second pass: MCP isolation on rejoin, printed secrets and concurrent rejoins (2026-11-24)

**A. MCP isolation on a rejoin holds:**

| Turn | What was done | What happened |
| --- | --- | --- |
| `l2b-01-iso-cold` | a HOME whose MCP set declares one sentinel server, and a prompt that calls its tool | the server started and was called once |
| `l2b-02-iso-rejoin-fresh-home` | a rejoin in a fresh HOME holding only `conversations/<id>.db` and `brain/<id>/`, whose MCP set declares a different sentinel | the cold turn's server did **not** start; the rejoin's own server started; asked for the cold server's tool, the model answered `NO SENTINEL TOOL` |
| `l2b-03-iso-rejoin-same-home-empty-set` | a rejoin in the cold turn's own HOME after its MCP set was rewritten to none | the cold turn's server did not start |

A stored conversation carries no MCP server into a later turn; the turn's own HOME decides its servers.

**B. A value a tool printed is in the conversation's files in clear:**

In `l2b-09-printed-value-allowed` (`l2b-04` was refused by agy's own permission check), a command printed `l2b-sentinel-value-7f3a9c2e41d8` from its environment. That value appeared in clear in:

- the stream, in one `step_update` row (masked by level 1);
- `brain/<id>/.system_generated/logs/transcript.jsonl` and `transcript_full.jsonl` (mode 0644);
- `brain/<id>/.system_generated/logs/chunks/transcript/00000000.jsonl` and `chunks/transcript_full/00000000.jsonl` (mode 0644);
- `brain/<id>/.system_generated/steps/2/output.txt` (mode 0644);
- `conversations/<id>.db` (mode 0600).

So whatever the engine keeps of a conversation holds bound secrets unmasked, in files agy creates world-readable (0644). Keeping `transcript.jsonl` therefore requires an owner-only parent (no group or other access), conversation directory (0700) and file (0600), and the run view must mask what it reads through `mask_secrets` (visiting `Turn.ts` alongside block text). Because agy already stores each member of `tool_calls[].args` as JSON text written once (`"sleep 90"`, `{"command":"cat seed.txt"}`), rendering each string member verbatim without re-parsing or re-serializing keeps every character raw or escaped once — the exact two forms `mask_secrets` covers — and every measured `created_at` is exact 20-byte `YYYY-MM-DDTHH:MM:SSZ`, which the reader enforces before copying into `Turn.ts` (counting any other present value, including `null`, in a notice, while only an absent member is uncounted).

**C. Two rejoins of one conversation at once do not stay apart:**

`l2b-06` (ALPHA) and `l2b-07` (BETA) rejoined one conversation in one HOME concurrently (`l2b-05` cold, `l2b-08` after). Both exited 0 with `SUCCESS` and empty stderr despite `presence/<id>.lock`; BETA's response held both `ALPHA LIME-8802` and `BETA LIME-8802` (`num_turns` 3); `l2b-two-rejoins.transcript.ndjson` carries two rows with `step_index` 4, one after row 6; and `l2b-08` listed only three of the four sent messages. Therefore `step_index` is neither unique nor ordered, and a future rejoin must serialize access per conversation.

## Eighth measurement, on 1.5.17: the two Claude slugs (2026-11-25 and 2026-11-26)

**Provenance.** Two passes by the controller on the Linux workstation with
Antigravity CLI 1.5.17 (#1382): on 2026-11-25, twenty-four live lab turns by
hand (`claude-opus-5-5`, `claude-sonnet-5-5` and `claude-sonnet-5-5-high` for
the fourteen effort-grammar turns; `claude-sonnet-5-5-low` alone for the ten
short-security-set turns); and on 2026-11-26, two boxed work seats through the
released `brokkr` 0.16.0 binary (one on `claude-opus-5-5` and one on
`claude-sonnet-5-5`, each at `--effort low`). The Gemini seats have run on agy
1.5.16 and 1.5.17 without a re-measurement, by the operator's call of
2026-11-25.

**How.** Fourteen turns exercised the effort grammar of `claude-sonnet-5-5` and
`claude-opus-5-5` with `--output-format stream-json`. Ten further turns ran the
short security set on `claude-sonnet-5-5-low` through
`security-set-claude-sonnet-v17.sh`, each with
`AGY_CLI_DISABLE_AUTO_UPDATE=true` and `--print-timeout 150s`, using the HOME
documents, sentinel servers, gate hook and hands server of the sixth
measurement. `claude-sonnet-v17-turns.log` holds each security-set turn's exit
code and argv, and `box-engine-workspace-tool-sonnet-v17.transcript.ndjson`
holds the kept conversation log of the boxed workspace-tool turn.

| Recording | What was asked | Exit | Result |
| --- | --- | --- | --- |
| `claude-sonnet-first.ndjson` | `--model claude-sonnet-5-5`, no `--effort`, one-word reply | 0 | `SUCCESS`; init `model` is `claude-sonnet-5-5`; response `ready` |
| `claude-sonnet-suffix.ndjson` | `--model claude-sonnet-5-5-high`, effort appended to slug, one-word reply | 0 | `SUCCESS`; init `model` is `claude-sonnet-5-5-high`; response `ready` |
| `claude-sonnet-effort-low.ndjson`, `claude-sonnet-effort-medium.ndjson`, `claude-sonnet-effort-high.ndjson` | `--model claude-sonnet-5-5` with `--effort low`, `medium` and `high`, one-word reply | 0 | `SUCCESS` each; init `model` is `claude-sonnet-5-5`; response `ready` |
| `claude-sonnet-effort-xhigh.ndjson`, `claude-sonnet-effort-xhigh.stderr` | `--model claude-sonnet-5-5 --effort xhigh` | 1 | `ERROR` result before any turn: `claude-sonnet-5-5 has no "xhigh" effort (available: low, medium, high)` |
| `claude-sonnet-effort-max.ndjson`, `claude-sonnet-effort-max.stderr` | `--model claude-sonnet-5-5 --effort max` | 1 | `ERROR` result before any turn: `claude-sonnet-5-5 has no "max" effort (available: low, medium, high)` |
| `claude-sonnet-effort-bogus.ndjson`, `claude-sonnet-effort-bogus.stderr` | `--model claude-sonnet-5-5 --effort bogus` | 1 | `ERROR` result before any turn: `invalid --effort "bogus" (valid: low, medium, high, xhigh, max)` |
| `claude-opus-noeffort.ndjson` | `--model claude-opus-5-5`, no `--effort`, one-word reply | 0 | `SUCCESS`; init `model` is `claude-opus-5-5`; response `ready` |
| `claude-opus-effort-low.ndjson`, `claude-opus-effort-medium.ndjson`, `claude-opus-effort-high.ndjson` | `--model claude-opus-5-5` with `--effort low`, `medium` and `high`, one-word reply | 0 | `SUCCESS` each; init `model` is `claude-opus-5-5`; response `ready` |
| `claude-opus-effort-xhigh.ndjson`, `claude-opus-effort-xhigh.stderr` | `--model claude-opus-5-5 --effort xhigh` | 1 | `ERROR` result before any turn: `claude-opus-5-5 has no "xhigh" effort (available: low, medium, high)` |
| `claude-opus-effort-max.ndjson`, `claude-opus-effort-max.stderr` | `--model claude-opus-5-5 --effort max` | 1 | `ERROR` result before any turn: `claude-opus-5-5 has no "max" effort (available: low, medium, high)` |
| `iso-01-positive-control-sonnet-v17.ndjson`, `iso-01-positive-control-sonnet-v17.starts` | operator-like HOME with `--add-dir` on `claude-sonnet-5-5-low` | 0 | `READY`; all six sentinels started (`user`, `plugin`, `adddir`, `workspace`, `wsplugin`, `parent`) |
| `iso-02-engine-home-workspace-cwd-sonnet-v17.ndjson`, `iso-02-engine-home-workspace-cwd-sonnet-v17.starts` | scratch HOME, seeded workspace as cwd | 0 | `READY`; `workspace`, `parent` and `wsplugin` started; none of the user's |
| `iso-05-above-git-root-sonnet-v17.ndjson`, `iso-06-parent-without-git-sonnet-v17.ndjson` | scratch HOME, clean workspace | 0 | `READY` each; no sentinel started |
| `iso-07-engine-carriage-cold-sonnet-v17.ndjson`, `iso-07-engine-carriage-cold-sonnet-v17.starts`, `iso-07-engine-carriage-cold-sonnet-v17.hooklog` | engine server in scratch HOME `mcp_config.json`, cold turn | 0 | `SENTINEL-NONCE-ENGINE`; server started and tool answered |
| `hook-crash-sonnet-v17.ndjson`, `hook-crash-sonnet-v17.hooklog` | crashing gate hook and a web search | 0 | `NO SEARCH`; crashing gate denied the call |
| `hook-timeout-sonnet-v17.ndjson`, `hook-timeout-sonnet-v17.hooklog` | hanging gate hook and `uname -s` | 0 | `REFUSED` with `JSON hook "jsonhook__brokkr-gate_PreToolUse_0_0" failed: command failed: signal: killed` |
| `box-engine-workspace-tool-sonnet-v17.ndjson`, `box-engine-workspace-tool-sonnet-v17.transcript.ndjson` | three workspace-tool commands, then native `run_command` and `write_file` | 0 | three workspace commands ran (`seeded-by-controller`, `agy-was-here`, `0`); both natives `REFUSED` by the pre-tool hook |
| `box-reach-sonnet-v17.ndjson` | box reach via the workspace tool (six checks) | 0 | 4 entries in `/proc`, pid 1 is `bwrap`, empty home, only `box3` under the parent, no network (`curl` exit 6, status `000`), uid `1000` in the worktree |
| `box-other-natives-sonnet-v17.ndjson` | ten other native tools by name | 0 | nine refused by the gate by name; `browser_get_dom` reported `NOT-AVAILABLE` |

**Facts.**

- Both `claude-opus-5-5` and `claude-sonnet-5-5` accept `--effort low`, `medium`
  and `high` and run with no `--effort`. `xhigh` and `max` exit 1 before any
  turn with an `ERROR` result and the same `has no "<effort>" effort
  (available: low, medium, high)` wording `gemini-4-carbon` receives. An
  unknown effort exits 1 naming all five words agy knows (`low, medium, high,
  xhigh, max`).
- A slug with the effort appended (`claude-sonnet-5-5-high`) is accepted and
  init's `model` repeats it as given.
- The stream has the Gemini turns' shape (`init`, `step_update`, `result`), and
  init's `model` is the slug asked for.
- Every turn of the short security set on `claude-sonnet-5-5-low` gave the
  result the sixth measurement recorded for Gemini.
- The kept conversation log of the boxed workspace-tool turn
  (`box-engine-workspace-tool-sonnet-v17.transcript.ndjson`) has the row types
  and key sets a Gemini turn's has (10 rows across `USER_INPUT`,
  `PLANNER_RESPONSE` and `GENERIC`, including `thinking` on one
  `PLANNER_RESPONSE` row).
- Which upstream serves a Claude turn was not measured.

**Token totals (lab pass).** As agy reported them, the 24 lab turns took
125,592 input tokens, 5,128 output tokens and 313,438 cache read tokens;
`claude-opus-5-5` took four one-word turns of that (23,936 input, 16 output and
34,076 cache read, plus two zero-token effort refusals).

### Boxed work seats through the released 0.16.0 engine (2026-11-26)

One boxed `implement` work seat per Claude slug ran through the released
`brokkr` 0.16.0 binary (`/var/tmp/probe-agy/bin-0160/brokkr`) on Antigravity
CLI 1.5.17 (Linux), using the shipped `agy` adapter plus the two Claude entries
(`agy-claude-opus` and `agy-claude-sonnet`), each pinned at `--effort low` on a
four-step task (read the first three lines of `README.md`, write `measured` to
`notes/claude-on-agy.txt`, commit it as `probe: one line (#1382)`, and write
the `complete` result JSON through the workspace tool). In all eight committed
files the home directory's name is replaced by `operator`, and each
`.seat.json` is the controller's extract of the run view's seat row.

| Model | Recorded files | Launch (`.argv`) |
| --- | --- | --- |
| `claude-opus-5-5` | `claude-engine-0160-opus.argv`, `claude-engine-0160-opus.mcp_config.json`, `claude-engine-0160-opus.seat.json`, `claude-engine-0160-opus.transcript.txt` | `agy --input-format stream-json --output-format stream-json --model claude-opus-5-5 --effort low --settings {"permissions":{"allow":["mcp(brokkr/workspace)"]}}` |
| `claude-sonnet-5-5` | `claude-engine-0160-sonnet.argv`, `claude-engine-0160-sonnet.mcp_config.json`, `claude-engine-0160-sonnet.seat.json`, `claude-engine-0160-sonnet.transcript.txt` | `agy --input-format stream-json --output-format stream-json --model claude-sonnet-5-5 --effort low --settings {"permissions":{"allow":["mcp(brokkr/workspace)"]}}` |

Each `.mcp_config.json` (`claude-engine-0160-opus.mcp_config.json` and
`claude-engine-0160-sonnet.mcp_config.json`) registers one MCP server,
`brokkr`, running `/var/tmp/probe-agy/bin-0160/brokkr hands serve` against the
seat's linked worktree and `.forge/artifacts`, with `--spec` `kind: "workspace"`,
`network: false`, and three binds (`~/.cargo` overlay masking
`credentials.toml` and `credentials`, `~/.rustup` read-only, and
`~/.cache/forge-fires/bin` read-only).

| Model | Run | Outcome & rule | Turns | Boundary | Tokens (in / out / cache read / total) | Transcript kind |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-opus-5-5` | `one-turn-measurement-not-a-story-0fb3de13` | `succeeded` (1 attempt, ruled `passed` / `IMPL-OK`, `28s`) | 5 | `namespace` | 109,065 / 1,160 / 88,680 / 110,225 (`cache_write_tokens` and `reasoning_output_tokens` `null`) | `agy-conversation` |
| `claude-sonnet-5-5` | `one-turn-measurement-not-a-story-cd3129f5` | `succeeded` (1 attempt, ruled `passed` / `IMPL-OK`, `26s`) | 6 | `namespace` | 131,858 / 1,288 / 110,932 / 133,146 (`cache_write_tokens` and `reasoning_output_tokens` `null`) | `agy-conversation` |

What the kept transcripts show:

- Both conversations were kept as `agy-conversation` under
  `/home/operator/.local/state/brokkr/agy-conversations/<id>/transcript.jsonl`
  and read back through `brokkr runs transcript`
  (`claude-engine-0160-opus.transcript.txt`,
  `claude-engine-0160-sonnet.transcript.txt`). Each `.seat.json` records
  `ruled.model` as `"<slug>, not reported"`.
- In `claude-engine-0160-opus.transcript.txt`, Opus's first tool call (turn 2)
  was its native `read_file` on the `workspace.json` schema path, which was
  denied by the gate by name (`tool call denied by pre-tool hook: the engine's
  gate does not offer the tool read_file`); every later tool call (turns 4, 7
  and 9) went through `call_mcp_tool` on `brokkr/workspace`, combining steps 2
  and 3 into one chained command (`8c1ebbf75 probe: one line (#1382)`) and
  writing the `complete` result file.
- In `claude-engine-0160-sonnet.transcript.txt`, Sonnet's first tool call (turn
  2) was likewise native `read_file` on the `workspace.json` schema path and
  received the same gate denial by name; every later tool call (turns 4, 7, 9
  and 12) went through `call_mcp_tool` on `brokkr/workspace` (`509d8c0e1 probe:
  one line (#1382)`) and wrote the `complete` result file.

**Not measured in this pass.** A seat through the engine on either Claude slug
is measured at `low` on a four-step boxed task only; still not measured are any
other effort (`medium` or `high`) through the engine, a real story on either
model, a resume (including `iso-09`), the short security set on
`claude-opus-5-5` (whose lab pass was four one-word turns and two effort
refusals), which upstream serves a Claude turn, and macOS.

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
- A self-update landing in the middle of a running turn, `agy update`, and
  the settings member `AutoUpdate`. The fifth measurement has the rest.
- The launch grammar, the stream's shape and the refusals of the first
  measurement, on 1.5.14.
- The `gpt-oss-*` and orchestrator slugs. `claude-opus-5-5` and
  `claude-sonnet-5-5` are measured as far as the eighth measurement goes
  (effort grammar on both, the short security set on `claude-sonnet-5-5-low`
  alone, and one boxed work seat per model at `low` on a four-step task through
  the released 0.16.0 engine); still not measured for them are any other effort
  through the engine, a real story on either model, a resume (including a
  resume of the engine-carriage turn), the short security set on
  `claude-opus-5-5`, which upstream serves a Claude turn, and macOS.
- `agy acp`, the Agent Client Protocol server.
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
