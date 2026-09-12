# dsh-plugin-cli-session

A resume-capable CLI runner for [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness). It replaces the shipped one-shot headless runner with one that can continue a previous session and, when asked, print a machine-readable envelope instead of prose.

It is the merge of two hand-copied profile runners (a conversational one and a `claude -p`-style one) whose `runner.js`/`startup.js` had drifted into near-duplicates. Everything they actually shared — session resolution, the drive loop, summarisation, exit handling — is one implementation now; what differed is configuration.

## Install

Not on npm yet. The built `lib/` is committed, so it installs straight from
GitHub — no download, no build step:

```sh
dsh plugin --profile chat add github:ghbhiee/dsh-plugin-cli-session
```

Or from a local clone (a `link:`, so a local rebuild is picked up):

```sh
git clone https://github.com/ghbhiee/dsh-plugin-cli-session.git
dsh plugin --profile chat add ./dsh-plugin-cli-session
```

Sibling plugins install the same way: [dsh-plugin-workbench](https://github.com/ghbhiee/dsh-plugin-workbench),
[dsh-plugin-mobile-shell](https://github.com/ghbhiee/dsh-plugin-mobile-shell),
[dsh-plugin-cli-session](https://github.com/ghbhiee/dsh-plugin-cli-session).

## Develop

```sh
pnpm install
pnpm run check   # typecheck → vitest → tsdown build
```

Because `lib/` is versioned (it is what a git install serves), rebuild and
commit it with every source change.

## Two flavours, one plugin

Conversational (`~/.dsh/profiles/chat/cordis.patch.yml`):

```yaml
- id: cli-runner
  config:
    request: !!js ctx.cliStartup
    sessionTag: chat-cli
    announceSessionId: true
```

Machine-readable (`~/.dsh/profiles/api/cordis.patch.yml`):

```yaml
- id: cli-runner
  config:
    request: !!js ctx.cliStartup
    sessionTag: api
    announceSessionId: false
```

A patch replaces the targeted row's whole `config`, so `request` has to be restated in both.

| Field | Default | Meaning |
|---|---|---|
| `sessionTag` | `cli` | Written to each session's `agentPreset`; also scopes `--list` and `--resume`, so two profiles in the same directory never see each other's sessions |
| `announceSessionId` | `false` | Write `session: <id>` to stderr (stdout stays clean for the answer) |
| `exitGraceMs` | `1500` | How long to wait for a graceful exit before forcing one; must be a non-negative number |

## Usage

```sh
dsh --profile chat "explain this file"          # new session
dsh --profile chat -r "follow-up"               # resume the newest session here
dsh --profile chat -s <id> "follow-up"          # resume a specific session
dsh --profile chat -w ./scratch "start here"    # new session in another directory
dsh --profile chat -l                           # list this profile's sessions here
dsh --profile api -o json "summarize this"      # {"type":"result",…}
dsh --profile api -o stream-json "summarize"    # init line, then the result line
```

| Flag | Meaning |
|---|---|
| `-o, --output-format <text\|json\|stream-json>` | Shape of what is printed; `text` is the conversational one |
| `-n, --new` | Start a new session (the default) |
| `-r, --resume` | Continue the newest session of this profile in this directory |
| `-s, --session <id>` | Continue a named session |
| `-w, --workdir <dir>` | Create and enter a directory before starting a new session |
| `-l, --list` | List this profile's sessions here and exit |
| `--json-schema <schema>` | Ask the model for JSON in this shape (a prompt, not a constraint) |

`-o json` / `-o stream-json` emit `{type, subtype, is_error, duration_ms, num_turns, result, session_id, usage?}`. `stream-json` prefixes a `{type:"system",subtype:"init",…}` line carrying the session id, cwd, provider, and model.

## Misconfiguration fails at boot

The row's `request` comes from `!!js ctx.cliStartup`. Wire it wrong and the config schema's defaults quietly produce `action: 'new'` with an empty task — a session created and a model call spent on nothing. The runner refuses that at load instead, and says what to write:

```
cli-runner: no task to run. The row's config.request should be wired to the
startup service, as in `request: !!js ctx.cliStartup`.
```

A blank `sessionTag` (the label `--list` and `--resume` scope by) and a negative `exitGraceMs` are refused the same way.

## Tests

`pnpm test` covers both halves without an API key: the runner's pure core (which assistant message is returned, how sessions are scoped by tag, that `--json-schema` is prompt text) and the command-line contract (every accepted shape, every rejected combination, and that `--workdir` chdirs *before* publishing so the sandbox root follows).

## Known limitations

- **`--json-schema` is a prompt, not a constraint.** The schema is appended to the task text asking for conforming JSON; it is not the provider's structured-output mode. A model can still answer with code fences or commentary, so parse defensively.
- **Only the last assistant message is returned.** Intermediate turns are in the session log, not in `result`.
- **`is_error` covers any non-`completed` turn reason**, including a turn that never started.
- **`sessionTag` is stored in `agentPreset`**, a field the harness defines for agent presets. It works as a session label today, but it is a borrowed field, not a documented extension point.
- **The hard exit is a fallback, not a design.** `exitGraceMs` after the graceful exit request the process is killed, which can truncate a very slow flush.
