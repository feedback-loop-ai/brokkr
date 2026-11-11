# dsh hands on 0.2.2-rc.1: a measured release, its own composed set, and the re-measure procedure (#1052)

Issue #1052, unit 1052a. This record backs the release-aware hands check in
`crates/brokkr-protocol/src/adapters/dsh_hands.rs` and its
[composition](dsh-hands-round2.md) child. It extends the record of
[round 2](dsh-hands-round2.md); the 0.2.1 figures there stand unchanged.

## Provenance

The implementing seat has no dsh and no route to the local Spark: its box
refuses to spawn an installed dsh and has no network. **No dsh figure below
was re-run by that seat.** Every live figure is the controller's
measurement of 2026-11-11, quoted from the story's framing. The offline
bindings at the end were run by the seat.

| Figure | Source |
| --- | --- |
| The 0.2.2-rc.1 install and its 656 packages | controller, 2026-11-11, Linux workstation |
| The plain `--dump-config`'s 99 rows (before the seat overlay's insert) and the three row deltas | controller, 2026-11-11 |
| Both tool lists, with and without the hands rows | controller, 2026-11-11, `spark-glm/GLM-5.3-Flash-EXL3` |
| `tool-executor-ptc` disabled by default | controller, 2026-11-11 |
| The sandbox rows keeping their ids and packages | controller, 2026-11-11 |
| The release tables, the overlay bytes and the launch checks | offline tests, this change |

## Host and versions

| Item | Value |
| --- | --- |
| Host | Linux (the controller's workstation). macOS: pending, no macOS host |
| dsh | 0.2.2-rc.1 (npm `latest`), installed in the isolated prefix `/var/tmp/dsh-0.2.2-rc.1` |
| Install | `npm install --prefix /var/tmp/dsh-0.2.2-rc.1 @deepseek-ai/dsh@0.2.2-rc.1`; 656 packages |
| dsh's sandbox package | `@deepseek-ai/dsh-sandbox-local` 0.2.2-rc.1 |
| Home | a scratch `DSH_HOME` holding a copy of the operator's `headless` profile; the installed 0.2.1-rc.2 and its home were untouched |
| Profile modules | the copied profile's `node_modules` carries none of the measured packages, so every row below is 0.2.2-rc.1's own |
| Model | headless takes it only from the overlay's `agent-default-model` row (it has no `--model` flag), as `brokkr driver dsh` already writes it |

## The composition diff

`dsh --profile headless --dump-config` makes no model call. Its plain dump
composed 99 top-level rows; 0.2.1's plain dump was 98, and its measured 99
includes the seat overlay's `brokkr-hands` insert. A 0.2.2-rc.1 seat's
composed tree is therefore 100 rows. The deltas against the measured 0.2.1
set:

| Change | Row | Package | State |
| --- | --- | --- | --- |
| new | `tool-executor-native` | `@deepseek-ai/dsh-tool-executor-native` | enabled |
| new | `tool-executor-ptc` | `@deepseek-ai/dsh-tool-executor-ptc` | `disabled: true` by default |
| gone | `tool-subagent-list-agents` | `@deepseek-ai/dsh-tool-subagent-control/list-agents` | absent on 0.2.2 |
| unchanged | `brokkr-hands` | `@deepseek-ai/dsh-mcp-client` | absent from a plain dump: it is the seat overlay's own insert, as on 0.2.1 |

So 0.2.2-rc.1's measured set is 0.2.1's 99, minus
`tool-subagent-list-agents`, plus `tool-executor-native` and
`tool-executor-ptc`: 100 rows. A seat on 0.2.1 may not carry either
executor row, and a seat on 0.2.2-rc.1 may not carry
`tool-subagent-list-agents`.

The sandbox rows keep their ids and packages on both releases: `sandbox`
(`@deepseek-ai/dsh-sandbox-local`), `sandbox-policy`
(`@deepseek-ai/dsh-sandbox-policy`), `bash-sandbox`
(`@deepseek-ai/dsh-bash-sandbox`), `pwsh-sandbox`
(`@deepseek-ai/dsh-pwsh-sandbox`), `permission`
(`@deepseek-ai/dsh-permission-presets`) and `fs-sandbox`
(`@deepseek-ai/dsh-fs-sandbox`).

## The overlay

The hands overlay is unchanged for 0.2.1. On 0.2.2-rc.1 it writes one more
row, after the sandbox row and before the workspace server:

```yaml
- id: tool-executor-ptc
  disabled: true
```

`tool-executor-ptc` is the programmatic tool-calling executor: dsh ships it
disabled by default, and pinning it disabled keeps a profile that enables it
from giving a hands seat a code-running executor. `tool-executor-native`
stays enabled: it runs the tool calls the model is still offered, and the
ten disabled rows remove every effectful tool it could run.

## Tool lists

Asked of the model in one session on the local
`spark-glm/GLM-5.3-Flash-EXL3` route, with the overlay `agent-default-model`
row alone, then with the ten hands rows:

| Overlay | Tools |
| --- | --- |
| none (24) | `bash` `create_goal` `edit` `get_goal` `glob` `grep` `interrupt_agent` `job_kill` `job_list` `job_output` `list_agents` `read` `read_image` `send_message` `skill` `subagent` `subagent_fork` `todo_write` `update_goal` `web_fetch` `web_search` `workflow` `working_directory` `write` |
| the hands rows (8) | `create_goal` `get_goal` `interrupt_agent` `list_agents` `send_message` `todo_write` `update_goal` `working_directory` |

That is 0.2.1's 25 tools without `exit_plan_mode`, and 0.2.1's nine
bookkeeping tools without `exit_plan_mode`. No shell, no file, no web and
no subagent spawn remain. The ten rows do on 0.2.2-rc.1 what they did on
0.2.1. The kept eight are bookkeeping only: goals, todos, subagent
steering (with both spawn rows disabled) and the working directory.

## Re-measuring a new dsh release

The trigger is a new dsh `latest` on npm; a unit that admits it records the
measurement here before adding the release to `MEASURED_VERSIONS`. Any
controller can follow this procedure.

1. Install the release in an isolated prefix, leaving the installed dsh and
   its home untouched:

   ```sh
   npm install --prefix /var/tmp/dsh-<version> @deepseek-ai/dsh@<version>
   ```

2. Make a scratch `DSH_HOME` holding a copy of the operator's `headless`
   profile. Confirm the copy's `node_modules` carries none of the measured
   packages, so every composed row is the new release's own.
3. Run `dsh --profile headless --dump-config` (no model call) and diff its
   top-level `(id, name)` pairs against the measured set of the release it
   replaces. Record every added, removed and changed row.
4. Ask the model for its tool list in one session, once with the overlay's
   `agent-default-model` row alone and once with the ten hands rows. Record
   both lists and every tool that changed.
5. Check the sandbox rows: `sandbox`, `sandbox-policy`, `bash-sandbox`,
   `pwsh-sandbox`, `permission` and `fs-sandbox` keep their ids and
   packages, and the `sandbox-policy` mode still refuses anything but
   `read-only` and `workspace-write`.
6. Add the release and its set here, then in `MEASURED_VERSIONS` and the
   release's composition delta table. Add a `Release` variant with its own
   exact versions in `Release::versions` (and its `Release::label`), and
   add the version string once, to the const the variant names. Add a test
   that a seat on the new release is admitted and that another release's
   row is refused. Pin any newly added effectful row disabled in the
   overlay.

## Offline bindings (this change)

Run by the implementing seat with
`cargo test -p brokkr-protocol --lib --locked -- adapters::dsh_hands`:

- `only_a_measured_release_is_served_the_hands_rows` and
  `a_0_2_2_release_is_measured_and_its_seat_is_served_its_rows`:
  0.2.2-rc.1 is matched whole and a seat on it is served its rows; the
  refusal for any other version names all three measured releases.
- `the_0_2_2_measured_composition_is_admitted`: 0.2.2-rc.1's 100-row set is
  admitted, its native executor enabled and its programmatic executor
  disabled.
- `a_0_2_1_tree_carrying_an_executor_row_is_refused` and
  `a_0_2_2_tree_carrying_the_removed_row_is_refused`: each release admits
  only its own measured set.
- `an_unmeasured_row_is_refused_on_0_2_2`: a row nobody measured is still
  refused on the new release.
- `a_0_2_2_tree_with_the_programmatic_executor_enabled_is_refused`: a
  profile that enables `tool-executor-ptc` is refused by its id on
  0.2.2-rc.1, so the pin is checked on the composed tree and not only in
  the overlay text.
- `the_0_2_2_overlay_disables_the_programmatic_tool_row`: the overlay pins
  `tool-executor-ptc` disabled on 0.2.2-rc.1 and names no such row on 0.2.1.
- `the_composition_gate_keys_on_the_release_it_reads`: the launch's
  composed-tree gate judges the same dump against the release its binary
  answers; an unmeasured release's dump is read and then refused against
  no set. (A driver the version probe cannot start is refused as an
  unreadable tree; that path is bound in `dsh_launch.rs`'s own tests.)

## Pending

- macOS, on every figure. This record proves the release tables and the
  checks on Linux only; macOS is still a supported host (decision 0063).
- A real 0.2.2-rc.1 composition of a seat's own overlay. The controller's
  2026-11-11 measurement was the PLAIN `--dump-config` (99 rows, before the
  `brokkr-hands` insert) and the two tool lists with the ten rows; the
  seat's full overlay — the ten rows, `sandbox-policy`, the
  `tool-executor-ptc` pin and the `brokkr-hands` insert — has never been
  composed by a real 0.2.2-rc.1. The offline 0.2.2 fixture is generated
  from `ADDED_0_2_2`/`REMOVED_0_2_2` themselves, so the gate has not yet
  judged a real 0.2.2 tree. Every way that can go wrong fails closed
  (`DumpFailed`, `Unmeasured`, `Enabled`), but it could leave a 0.2.2 hands
  seat refused by a different sentence. Owed to the controller before the
  PR leaves draft: one `dsh --profile headless --patch <the seat's overlay>
  --dump-config` on the isolated 0.2.2-rc.1 prefix.
- A live `mcp__brokkr__workspace` call through this driver on 0.2.2-rc.1.
  The controller measured the tool lists, not a call of Brokkr's server
  under a seat.
- dsh's native egress and any OFF control for it (decision 0065 slice one;
  owed to the controller).
- A planted write through any remaining path in a read-only seat with the
  composed hands row, on 0.2.2-rc.1 as on 0.2.1.
