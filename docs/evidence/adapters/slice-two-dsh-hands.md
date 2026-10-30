# dsh hands: the tool rows the seat overlay disables (#896)

Issue #896, beside slice two's
[MCP isolation record](slice-two-mcp-isolation.md). This record backs the
`cold` shape with `"hands": "boxed"` in `adapters/dsh.json` and the hands
rows `brokkr driver dsh` writes into a seat's one `--patch` overlay
(`crates/brokkr-protocol/src/adapters/dsh_hands.rs`). Its row shape and
its pending list are superseded in part by
[round 2](dsh-hands-round2.md), which added the measured `transport`, the
composed-tree check, the activation refusal and the access carrier.

## Provenance

The implementing seat could not run dsh: its box refuses to spawn the
installed `dsh` and has no network or route to the local Spark. **No dsh
figure below was re-run by that seat.** Every live figure is the
controller's measurement of 2026-10-26, quoted from the story's framing
and from #896. The offline bindings at the end were run by the seat.

| Figure | Source |
| --- | --- |
| Tool list without and with the overlay | controller, 2026-10-26 |
| `--dump-config` showing each row `disabled: true` | controller, 2026-10-26 |
| rc.2 and rc.3 answer identically | controller, 2026-10-26 |
| dsh's own sandbox modes and enforcement | controller, 2026-10-26 (installed packages) |
| `mcp__brokkr__workspace` answering from a `dsh-mcp-client` insert | U0b D09, D10-p5 ([slice two](slice-two-mcp-isolation.md)) |
| The overlay's bytes and the launch that carries them | offline tests, this change |

## Host and versions

| Item | Value |
| --- | --- |
| Host | Linux (the controller's workstation). macOS: pending, no macOS host |
| dsh | 0.2.1-rc.2 (installed, npm `latest`) and 0.2.1-rc.3 (npm `next`, in an isolated prefix) |
| Profile and route | `--profile headless`, the local `spark-glm/GLM-5.3-Flash-EXL3` route |

## The overlay

Every model-facing dsh 0.2.1 tool is its own profile row, and an overlay
row `disabled: true` removes it, as the headless profile's `credentials`
row is removed. The measured overlay:

```yaml
- id: tool-bash
  disabled: true
- id: tool-pwsh
  disabled: true
- id: tool-jobs
  disabled: true
- id: tool-fs
  disabled: true
- id: tool-fs-search
  disabled: true
- id: tool-skill
  disabled: true
- id: tool-subagent
  disabled: true
- id: tool-subagent-fork
  disabled: true
- id: tool-workflow
  disabled: true
- id: tool-web
  disabled: true
- id: sandbox-policy
  config:
    mode: read-only
```

The driver writes exactly these rows, under a comment header, with the
mode `read-only` for a gate and `workspace-write` for a work seat (the
seat class the engine marks as `seat_class`). It then inserts Brokkr's
workspace server as the engine's `dsh-mcp-client` row: `serverName:
brokkr`, `command` the engine's executable, `args` the `hands serve`
arguments the final check binds the box's hands to. The bounded reader of
`route_overlay.rs` admits the ten rows and the mode in this one shape and
refuses any other row, a repeated row and a missing row, each by name.

## Tool lists

Asked in the same session, the model's own tool list:

| Overlay | Tools |
| --- | --- |
| none (25) | `bash` `create_goal` `edit` `exit_plan_mode` `get_goal` `glob` `grep` `interrupt_agent` `job_kill` `job_list` `job_output` `list_agents` `read` `read_image` `send_message` `skill` `subagent` `subagent_fork` `todo_write` `update_goal` `web_fetch` `web_search` `workflow` `working_directory` `write` |
| the hands rows (9) | `create_goal` `exit_plan_mode` `get_goal` `interrupt_agent` `list_agents` `send_message` `todo_write` `update_goal` `working_directory` |

That leaves no shell, no file, no web and no subagent spawn. rc.2 and rc.3
answer identically. `dsh --profile headless --patch <overlay>
--dump-config`, which makes no model call, shows each row `disabled: true`
in the composed tree.

### The bookkeeping tools kept

None of the nine reaches outside Brokkr's box, so they are kept rather
than disabled. That reading rests on the tools' names and the controller's
listing, not on a call of each:

- `create_goal`, `get_goal` and `update_goal` hold the session's goal;
- `todo_write` holds the session's todo list;
- `exit_plan_mode` leaves the session's plan mode;
- `list_agents`, `send_message` and `interrupt_agent` steer subagents, and
  with `tool-subagent` and `tool-subagent-fork` disabled none can be
  spawned;
- `working_directory` reports the session's working directory.

## dsh's own sandbox

dsh 0.2 ships `dsh-sandbox`, `dsh-sandbox-local`, `dsh-sandbox-policy`,
`dsh-bash-sandbox` and `dsh-fs-sandbox`. The `sandbox-policy` row's `mode`
is `read-only`, `workspace-write` or `danger-full-access`; the shipped
headless default comes from `DSH_PERMISSION_MODE`, else `workspace-write`.
It is enforced by bubblewrap, then Landlock, on Linux and by Seatbelt on
macOS. It reports `full` or `partial` enforcement and fails closed with
`SANDBOX_UNAVAILABLE`. Under the hands rows it is defence in depth: no
native tool that it confines remains.

## Offline bindings (this change)

Run by the implementing seat with `cargo test`:

- `adapters::dsh_hands::tests`: the rows equal the measured overlay above
  byte for byte under each mode; the reader admits them and refuses any
  other row by name; only 0.2.1-rc.2 and rc.3 are served the rows; a gate
  runs `read-only` and a work seat `workspace-write`; a launch whose office
  declares boxed hands carries the transcript row and the hands rows and
  stages no runner, and a launch without hands carries its transcript row
  alone.
- `native_controls::tests::an_unmeasured_dsh_command_checks_only_under_its_own_reason`
  and `native_controls::mcp::tests::dsh_carries_the_hands_alone_in_its_overlay`:
  the final check reads a sealed dsh hands transport as carried by the
  staged overlay, and not beside a broker.
- `bundle::model_policy_tests::dsh_hands_tests`: a boxed office is hired
  from the shipped dsh adapter, alone and behind a codex link.

## Pending

- macOS, on any figure.
- A `bash` or `write` call refused because the tool does not exist, and
  `mcp__brokkr__workspace` answering, through the overlay this driver
  writes. U0b D09 measured an equivalent candidate row; the repository's
  evidence records the row's `serverName` but not its `command` and `args`
  keys, which the driver's row assumes.
- A planted write through any remaining path in a `read-only` seat, with
  dsh's sandbox reporting `full` or `partial` enforcement. The driver does
  not yet surface that report.
- The `--dump-config` drift check at each launch. The driver binds the
  rows to the measured release instead, refusing any other `--version`.
- Ambient MCP exclusion beside the hands rows, route by route (the shape
  declares it unmeasured, as U1d3 declares the replacement shape).
