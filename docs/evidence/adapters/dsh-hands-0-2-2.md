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
composed 99 top-level rows. 0.2.1's plain dump is inferred to have been 98 —
its measured 99 less the overlay's `brokkr-hands` insert — and that figure is
an inference from the measured set, not a measured dump. A 0.2.2-rc.1 seat's
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
   `agent-default-model` row alone and once with the release's hands rows
   (`Release::pinned_rows`: the ten common rows, and on 0.2.2-rc.1 the
   `tool-executor-ptc` pin as well). Record both lists and every tool that
   changed.
5. Check the sandbox rows: `sandbox`, `sandbox-policy`, `bash-sandbox`,
   `pwsh-sandbox`, `permission` and `fs-sandbox` keep their ids and
   packages, and the hands overlay's `sandbox-policy` row carries the
   seat's own mode, `read-only` or `workspace-write`. Brokkr writes only
   those two words into that row: the overlay's bounded reader
   (`route_overlay::hands`, through `Mode::named`) refuses any other word
   in the overlay Brokkr writes. So dsh's third mode (`danger-full-access`)
   is never written by Brokkr's overlay.

   The composed-tree check reads **the whole dump**, every row and every
   line, by one closed row grammar, not only the `sandbox-policy` span.
   Fix round 1052g4 claimed the ASCII trim "only narrows" the rows other
   than `sandbox-policy`; that was false for one row and this round
   corrects it. A `brokkr-hands` row's `disabled:` padded with NBSP, an
   em space or an ideographic space was trimmed to `true` and passed on
   1052g4's head, where 51cd6014c, d048d02d8 and 01d5ffb41 refused it.
   The hands row's `disabled:` is now admitted only when absent or exactly
   `false` after the ASCII trim, as the policy row's is.

   The grammar, in order:
   - **Breaks and controls first, on every line of the dump.** Any line
     holding a C0 control other than tab (U+0000–U+0008 and
     U+000A–U+001F, so a bare CR or VT), DEL (U+007F), a C1 control
     (U+0080–U+009F, so NEL), LS (U+2028), PS (U+2029), a BOM (U+FEFF),
     U+FFFE or U+FFFF refuses `Unreadable` before any other test, wherever
     it sits: before the first row, in any row, inside a comment, or
     inside any row's unread block. A YAML reader breaks on several of
     these and treats none as white space, and U+FFFE/U+FFFF are outside
     YAML's printable set.
   - **Skips everywhere.** Only an empty line, a line of ASCII spaces and
     tabs, or a line whose first character after those is `#` is skipped.
     Text after the `#` is comment content, non-ASCII characters
     included. No other white space is skip material, so a line of NBSP,
     or NBSP then `#`, is refused. (A CRLF dump is still normalized by
     `str::lines` before this grammar; that is pre-existing, S6.)
   - **Column zero.** `- id: <v>` starts a row, its value taken with the
     ASCII trim. Any other non-skip column-zero line refuses:
     `-   id:`, `---`, a bare key. Before the first row only skip lines
     are admitted.
   - **Every row's two-space level is closed.** A line of exactly two
     spaces then a non-space is admitted only as `  name: <v>`,
     `  disabled: <v>`, `  config:` or `  inject:`, each at most once per
     row. `config:` and `inject:` are bare. Every other two-space line
     refuses: a second key (no last-wins), a quoted or spaced key
     (`  "disabled": false`, `  disabled : false`), `  ? `, `  <<:`, a
     flow-style value on `config:`/`inject:`, and an unknown key. The
     `sandbox-policy` row admits no `inject:`.
   - **Blocks.** `  config:` or `  inject:` opens the row's block.
     `  name:`, `  disabled:` and the next row close it; a skipped line
     leaves it as it is. A line indented three or more spaces is admitted
     only while a block is open. In `sandbox-policy` the block is read by
     the span grammar: only one `    mode: <w>` line. In every other row
     the block stays unread. A deeper line with no open block refuses:
     directly after `- id:`, `  name:` or `  disabled:` it is a plain- or
     quoted-scalar continuation that would change the value the gate read.
   - **One space and tabs.** A one-space line, and a non-skip line whose
     first character is a tab, refuse `Unreadable`.
   - **Values.** Other rows' `disabled:` keeps its reading: exactly `true`
     after the ASCII trim is disabled, and `false` and `!!js …` are not,
     so a row that must be disabled refuses `Enabled`. The
     `sandbox-policy` row's and the hands row's `disabled:` are admitted
     only when absent or exactly `false`; any other word refuses
     `PolicyDisabled` or `HandsDisabled` with the unchanged text.

   Every value the reader takes from the dump — every row's `- id:`, and
   `  name:`, `  disabled:` and `    mode:` — is trimmed with ASCII spaces
   and tabs only: Unicode white space is not trimmed, so a padded word
   cannot be read as the word the gate compares. The gate refuses a tree
   whose composed mode is not the seat's own and a tree that names no
   mode. It reads nothing else about the composed tree's sandbox. This
   step does not measure what dsh itself does with that mode. A future dsh
   release that adds a two-space key or a deeper line outside a block is
   refused by this grammar until it is measured here and the row grammar
   admits it.
6. Add the release and its set here, then in `MEASURED_VERSIONS` and the
   release's composition delta table. Add a `Release` variant with its own
   exact versions in `Release::versions` (and its `Release::label`), and
   add the version string once, to the const the variant names. Add a test
   that a seat on the new release is admitted and that another release's
   row is refused. Pin any newly added effectful row disabled in the
   overlay, by adding it to that release's `Release::pinned_rows`, the
   one table the judge, the overlay writer and the bounded reader all
   read.

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
- `the_composition_gate_keys_on_the_release_it_is_given`: the launch's
  composed-tree gate judges the same dump against the release the launch
  threaded in from its one `--version` read; the gate itself probes
  nothing. (A driver the launch's read cannot name is refused by the rows
  path, before the gate.)
- `a_hands_launch_reads_its_release_exactly_once`,
  `a_hands_resume_reads_its_release_from_the_launch_not_a_second_probe`,
  `the_bounded_reader_admits_the_0_2_2_ptc_pin_it_writes`,
  `a_malformed_0_2_2_ptc_pin_is_refused_by_the_bounded_reader`,
  `the_0_2_2_reader_requires_the_ptc_pin_after_the_sandbox_row`,
  `a_composed_mode_that_is_not_the_seats_is_refused`,
  `a_composed_tree_with_no_mode_is_refused`,
  `the_measured_trees_still_pass_their_own_mode`,
  `a_column_zero_line_after_the_mode_is_refused_as_the_old_gate_refused_it`,
  `a_sandbox_policy_span_refuses_a_second_mode_and_any_other_key`,
  `every_unadmitted_line_in_the_sandbox_policy_span_refuses`,
  `a_skipped_line_does_not_close_the_sandbox_config`,
  `a_disabled_sandbox_policy_row_is_refused`,
  `a_sandbox_policy_disabled_line_is_read_with_or_without_its_space`,
  `a_hands_seat_reports_its_sandbox_refusal_before_a_later_root_refusal`,
  `a_launch_refuses_a_composed_mode_that_differs_from_the_seats`,
  `a_policy_span_line_holding_a_break_or_control_is_refused`,
  `a_policy_span_line_of_only_non_ascii_space_is_refused`,
  `a_policy_span_refuses_a_mode_after_a_two_space_key_closed_config`,
  `a_policy_span_admits_a_mode_before_a_later_two_space_key`,
  `a_policy_span_still_skips_ascii_space_and_a_non_ascii_comment`,
  `a_policy_span_refuses_a_non_ascii_padded_value`,
  `a_policy_span_trims_ascii_padding`,
  `the_hands_row_disabled_is_admitted_only_absent_or_exactly_false`,
  `the_hands_row_refuses_a_break_or_control_before_its_word`,
  `the_whole_dump_refuses_a_break_or_control_on_every_line`,
  `a_non_ascii_padded_name_is_not_trimmed`,
  `a_mode_after_the_name_closed_the_policy_config_is_refused`,
  `the_s7_shapes_on_another_row_refuse`,
  `a_line_before_the_first_row_that_is_not_a_skip_is_refused`,
  `an_unadmitted_two_space_key_is_refused`,
  `a_repeated_two_space_key_is_refused`,
  `a_deeper_line_is_admitted_only_while_a_block_is_open`,
  `a_one_space_tab_or_column_zero_line_is_refused` and
  `another_rows_disabled_keeps_its_reading`: the
  #1052 part 1 follow-ups F4, F5 and the composed-mode finding, fix
  round 1052g2's C1, C2, C4 and C6, fix round 1052g3's F1 to F3, fix
  round 1052g4's S1, S2 and S4, and fix round 1052g5's C1, C2, C3 and
  the whole-dump row grammar. Each was made to fail by the removal
  control recorded in the commit that added it and then restored; the
  class test's controls remove one arm of the break-and-control refusal
  set at a time, and each removal makes its class's cases fail.

## A real seat tree, judged (controller, 2026-11-12)

The seat's full overlay, composed by a real 0.2.2-rc.1, was judged by
the gate itself. The overlay is the bytes `hands_rows(Release::V0_2_2,
mode, …)` writes:
- the ten rows;
- `sandbox-policy`;
- the `tool-executor-ptc` pin;
- the `brokkr-hands` insert, naming `brokkr hands serve`.

The method is a throwaway test on a `git archive` copy of 17096db85,
never committed. It ran `dsh --profile headless --patch <overlay>
--dump-config` on the isolated `/var/tmp/dsh-0.2.2-rc.1` prefix, under
the scratch `DSH_HOME`, and handed that command to
`composition_gate`. The test also asserted that `composition::release`
reads `V0_2_2` from the same program.

| Mode | Overlay sha256 (first 16) | Dump | Rows | Gate |
| --- | --- | --- | --- | --- |
| `read-only` | `a75646a695cdd447` | exit 0, 17,019 bytes, sha256 `6824e53812605e9f…` | 100 | `Ok(())` |
| `workspace-write` | `b34cf8f68de3dc5d` | exit 0, 17,073 bytes, sha256 `1e20453e8c9aa2d3…` | 100 | `Ok(())` |

In both trees:
- the ten rows and `tool-executor-ptc` are `disabled: true`;
- `sandbox-policy` carries the seat's mode;
- `tool-executor-native` is enabled, as ruled;
- `brokkr-hands` is present.

The 100 rows are 0.2.2's plain 99 plus the insert, as the composition
diff above predicts. A 0.2.2-rc.1 hands seat is therefore admitted by the
gate on the real tree, not only on the offline fixture.

## Pending

- macOS, on every figure. This record proves the release tables and the
  checks on Linux only; macOS is still a supported host (decision 0063).
- A live `mcp__brokkr__workspace` call through this driver on 0.2.2-rc.1.
  The controller measured the tool lists, not a call of Brokkr's server
  under a seat.
- dsh's native egress and any OFF control for it (decision 0065 slice one;
  owed to the controller).
- A planted write through any remaining path in a read-only seat with the
  composed hands row, on 0.2.2-rc.1 as on 0.2.1.
