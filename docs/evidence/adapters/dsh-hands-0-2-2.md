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

   The refusal order is the dump-wide break, control and tab rule, then the
   line-property closure, then the row grammar. The closure runs after the
   first and before the second, and it admits nothing the row grammar
   refuses; it only refuses a line that leaves a quote, a non-empty flow
   collection or an empty sequence item open.

   - **Breaks, controls and tabs first, on every line of the dump.** Any
     line holding a C0 control other than tab (U+0000–U+0008 and
     U+000A–U+001F, so a bare CR or VT), a tab anywhere (U+0009), DEL
     (U+007F), a C1 control (U+0080–U+009F, so NEL), LS (U+2028), PS
     (U+2029), a BOM (U+FEFF), U+FFFE or U+FFFF refuses `Unreadable`
     before any other test, wherever it sits: before the first row, in any
     row, inside a comment, inside any row's unread block, or inside
     block-scalar content. A YAML reader breaks on several of these and
     treats none as white space; a tab ends a tag at any white space and
     can open a quoted value, so the dump's language refuses it
     everywhere. U+FFFE/U+FFFF are outside YAML's printable set. The tab
     rule folds the earlier narrower tab refusals and tab trim into this
     one: `split_plain_key`, `strip_markers`, `is_skipped` and
     `ascii_trim` read ASCII spaces only now.
   - **The closure, on every non-empty line.** After the rule above and
     before the row grammar, a second pass reads every line on its own and
     refuses `Unreadable` if it leaves a quote, a non-empty flow collection
     or an empty sequence item open. PyYAML has two carriers: a quoted
     scalar and a flow collection can continue onto a line indented at or
     under its parent. js-yaml 4.3.2, dsh's reader, has a third: it folds
     the next less-indented `- ` line into an empty sequence item. So
     `- id: sandbox` / `  inject:` / `    -` / `- id: sandbox-policy` /
     `  config:` / `    mode: read-only` reads
     `inject: [null, {id: sandbox-policy, …}]`, and the policy row is
     gone; PyYAML reads the policy row. The same fold happens through a
     `# ==` line, with `    -   # c`, inside `    - - a` / `      -`, and
     after `    k:` / `    -`. A block scalar's and a plain multi-line
     scalar's content still ends at the first non-blank line at or under
     the parent. So if no line, read as a structure line at its own
     position, leaves one of those three carriers open, no line can carry
     a following row's keys away, whether YAML reads the line as structure
     or as block content. The pass judges no value and admits nothing the
     row grammar refuses.
     - **The line indicators.** Strip the leading ASCII spaces, then every
       `- `, `? ` or `: ` marker, with the spaces after each. A bare `-`
       refuses, and so does a last `- ` marker followed by nothing or by a
       comment: that is an empty sequence item. A bare `?` or `:` leaves
       nothing open.
     - **A node.** Strip every `!` or `&` property; its token ends at an
       ASCII space or a flow indicator (`,`, `[`, `]`, `{` or `}`), and
       the spaces after it are stripped. Then a quoted scalar must close on
       the line, with YAML's scanning (`''` inside single quotes and `\`
       escaping the next character inside double quotes). A flow collection
       is admitted only as exactly `[]` or `{}`; any other flow at a node's
       start refuses, so `[a, # ]`, `[?'a]', b` and `&a[b,` refuse and no
       non-empty flow is a dump line. An alias (`*`, ending as a property
       token does) is read next. A block-scalar header (`|` or `>`) and a
       comment (`#`) leave nothing open. A plain scalar is read up to the
       FIRST of `: ` (a key, whose value is read as a node) or ` #` (a
       comment); a quote inside it is plain content, so `k: 'a #b'` reads
       the closed quoted value after the key.
     - **After every node.** Skip ASCII spaces. If `:` follows, then a space
       or the line's end, the text after it is a node read by the same
       rule. This applies after a quoted scalar, a flow collection, an
       alias and a plain scalar, so `[]: "x`, `{}: "x`, `"k" : "x` and
       `*a : "x` all have their value read.
     - **The controller's measurement, not re-run by this seat.** Fix round
       1052g7's prototype (`/var/tmp/1052g7-proto/open.py`) measured the
       earlier two-carrier reading. 1052g8's prototype
       (`/var/tmp/1052g8-proto/closure.py`, both read-only and not
       committed) is the rebuilt pass above. It admits all four real dumps
       (the committed 0.2.1 dump, the two 0.2.2-rc.1 overlay dumps and the
       plain 0.2.2-rc.1 dump) and refuses every 1052g7 finding, the empty
       item included. On 600,000 generated trees from a token generator,
       the closure and a row filter standing in for the grammar admitted
       161,154. On 400,000 trees from a generator of mostly-valid YAML
       lines (markers including `-   `, `- - ` and `? `; quoted, flow,
       anchored, tagged and plain keys; `: `, `:` and ` : ` separators;
       values with open quotes, open and closed flows, comments inside
       flows, `[]: "x`, anchors against flow brackets, block headers and
       escapes), 96,763 were admitted. PyYAML found 0 differentials and
       js-yaml 4.3.2 found 0 differentials; most admitted trees fail to
       parse in both readers (fail-closed), and about 21,000 and 15,500
       respectively read exactly as the gate reads them. Those counts are
       the controller's, not this seat's.
   - **Skips everywhere.** Only an empty line, a line of ASCII spaces,
     or a line whose first character after those is `#` is skipped.
     Text after the `#` is comment content, non-ASCII characters
     included. No other white space is skip material, so a line of NBSP,
     or NBSP then `#`, is refused. A tab is refused by the rule above, so
     it is not skip material. (A CRLF dump is still normalized by
     `str::lines` before this grammar; that is pre-existing, S6.)
   - **Column zero.** `- id: <v>` starts a row, its value read by the
     block value rule below and then taken with the ASCII trim. Any other
     non-skip column-zero line refuses:
     `-   id:`, `---`, a bare key. Before the first row only skip lines
     are admitted.
   - **Every row's two-space level is closed.** A line of exactly two
     spaces then a non-space is admitted only as `  name: <v>`,
     `  disabled: <v>`, `  config:` or `  inject:`, each at most once per
     row. `config:` and `inject:` are bare. Every other two-space line
     refuses: a second key (no last-wins), a quoted or spaced key
     (`  "disabled": false`, `  disabled : false`), `  ? `, `  <<:`, a
     flow-style value on `config:`/`inject:`, and an unknown key. The
     `sandbox-policy` row admits `inject:` too, but only as the one
     measured block described below.
   - **Blocks.** `  config:` or `  inject:` opens the row's block.
     `  name:`, `  disabled:` and the next row close it; a skipped line
     leaves it as it is. A line indented three or more spaces is admitted
     only while a block is open. In `sandbox-policy` only the `config:`
     block is read by the span grammar: one `    mode: <w>` line. Its
     `inject:` block is admitted only as the measured
     `    - headlessStartup` line, once: any other line, a second item, a
     quoted item, a wrongly indented item and an empty block all refuse
     `Unreadable`, so a mode-looking line under `inject:` is not the
     composed mode. Every other row's blocks are unread. A deeper line
     with no open block refuses: directly after `- id:`, `  name:` or
     `  disabled:` it is a plain- or quoted-scalar continuation that would
     change the value the gate read.
   - **An unread block is read as js-yaml's dump language, not as YAML.**
     Every deeper line of an unread block must be a line js-yaml's dump
     writes, so no quoted scalar, flow collection, anchor or alias can
     carry a later row's keys away. The bound is the measured serializer,
     not a general YAML reader: `dsh --dump-config` is
     `renderConfigDump` → `groupedDump` in `@deepseek-ai/dsh-app-boot`,
     which prints the `# == <label>` lines and then
     `yaml.dump(group, { schema: entryListSchema, noRefs: true })` with
     js-yaml 4.3.2, the version installed for 0.2.1-rc.2 and 0.2.2-rc.1.
     That dump writes no quoted scalar that spans lines (a break or a
     foldable long line becomes a block scalar), no non-empty flow
     collection (`[]` and `{}` only), and with `noRefs` no anchor or
     alias; it writes no comment of its own. A future dsh that changes its
     serializer or its options is refused until it is measured here.
     - **Block-scalar content.** A line whose value is a block-scalar
       header (`|`, `|-`, `|+`, `>`, `>-` or `>+`, optionally after a tag
       such as `!!js `) opens content: every following blank line, or line
       indented more than the header's node column, is skipped unread,
       with only the break-and-control rule applied. The content ends at
       the first non-blank line indented at or under the node column,
       which is then read normally. The node column is the key's column
       when the header is a key's value, and the last marker's column for
       a bare item: for `    - k: >-` it is 6, so a line at column 6 is
       read and a line at column 7 or more is content. A `-` marker
       followed by more than one space records no column: the grammar
       refuses that line (see the marker rule below), so the sentence is
       true for the one-space markers js-yaml writes and not for a wider
       one. The measured trees' only headers are `>-`, `>` and `!!js >-`.
       A header with an indentation indicator (`|2`, `>1-`) refuses
       `Unreadable`, since no real tree carries one.
     - **A `-` marker followed by more than one space refuses.** Any `-`
       marker followed by more than one ASCII space refuses `Unreadable`,
       anywhere in the dump, as the column-zero `-   id:` already did.
       js-yaml's dump writes exactly `- `, so a wider marker is not a dump
       line, and `read_unread` would record the key at the marker's column
       rather than the key's. No real dump carries one.
     - **Every other line is read for structure.** Strip the indentation
       and any `- ` sequence markers. If what remains starts with a quote,
       the quoted scalar must close on the line, followed by nothing but
       ASCII spaces (a sequence item) or by zero or more ASCII spaces,
       `:`, and a value (a quoted key). A plain `key:` is followed by end
       of line or `: ` and a value. After one optional tag (`!` and
       non-spaces, then a space), a value is empty; a block-scalar header;
       exactly `[]` or `{}`; a single- or double-quoted scalar that closes
       on the line, followed by nothing but ASCII spaces; or a plain
       scalar starting with none of `'`, `"`, `[`, `{`, `]`, `}`, `,`,
       `&`, `*`, `|`, `>`, `?`, `!`, `%`, `@` or a backquote, and not the
       explicit markers `-`, `?` or `:` alone or followed by a space, and
       holding no `: `. Quote scanning follows YAML: in single quotes
       `''` is an escaped quote, in double quotes `\` escapes the next
       character, and a double-quoted scalar whose line ends inside it (a
       `\` continuation included) is open. Quotes inside a plain scalar
       (`it's`, `echo "hi"`, `!!js a ?? 'b'`) are plain content and are
       not scanned.
     - **Every other line refuses `Unreadable`.** An open quote, an open
       or non-empty flow collection, an anchor (`&`), an alias (`*`),
       `? `, a directive, a reserved indicator, a block-scalar indentation
       indicator, a double-space separator that a lenient YAML reader
       takes for structure, a plain value holding ` #` (js-yaml's dump
       writes no comment), and a quoted scalar followed by junk are all
       refused. A tab is refused earlier, dump-wide.
     - The block's lines stay unread in meaning: no other row's config
       value is judged, and the rule bounds only where the block ends.
   - **One space.** A one-space line refuses `Unreadable`. A line led by a
     tab is refused earlier, by the dump-wide rule.
   - **Values.** The two-space `- id:`, `name:` and `disabled:` values are
     read by the same block value rule as an unread block's value: a
     closed quote, one optional tag, or a plain start, with no open quote,
     open flow, anchor, alias, block header or ` #`. An open quote refuses
     `Unreadable` before its word is taken, and a plain value holding ` #`
     refuses, since js-yaml's dump writes no comment. Other rows'
     `disabled:` keeps
     its reading: exactly `true`
     after the ASCII trim is disabled, and `false` and `!!js …` are not,
     so a row that must be disabled refuses `Enabled`. The
     `sandbox-policy` row's and the hands row's `disabled:` are admitted
     only when absent or exactly `false`; any other word refuses
     `PolicyDisabled` or `HandsDisabled` with the unchanged text.

   Every value the reader takes from the dump — every row's `- id:`, and
   `  name:`, `  disabled:` and `    mode:` — is trimmed with ASCII spaces
   only: a tab is refused dump-wide, so it is never trimmed into a word,
   and Unicode white space is not trimmed, so a padded word cannot be
   read as the word the gate compares. The gate refuses a tree
   whose composed mode is not the seat's own and a tree that names no
   mode. It reads nothing else about the composed tree's sandbox. This
   step does not measure what dsh itself does with that mode. A future dsh
   release that adds a two-space key or an unread-block line this dump
   language does not write is refused by this grammar until it is measured
   here and the row grammar admits it.
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
  `the_policy_inject_block_is_only_the_measured_one`,
  `an_unread_block_refuses_a_quoted_scalar_that_does_not_close`,
  `a_quote_in_the_policy_or_preceding_sandbox_block_refuses`,
  `an_unread_block_refuses_an_open_flow_collection`,
  `an_unread_block_refuses_every_other_non_dump_line`,
  `an_unread_block_admits_every_dump_line_it_reads`,
  `a_repeated_two_space_key_is_refused`,
  `a_deeper_line_is_admitted_only_while_a_block_is_open`,
  `a_one_space_tab_or_column_zero_line_is_refused`,
  `another_rows_disabled_keeps_its_reading`,
  `the_closure_refuses_every_carrier_case`,
  `the_closure_refuses_a_quoted_key_and_the_quote_escapes`,
  `the_closure_admits_every_closed_shape`,
  `the_real_tree_fixtures_pass_the_closure_and_the_gate`,
  `the_block_content_judgement_reads_a_line_at_the_nodes_column`,
  `a_tab_anywhere_in_the_dump_is_refused`,
  `plain_values_the_dump_never_writes_refuse`,
  `a_two_space_value_refuses_an_open_quote_or_a_header` and
  `a_row_id_and_name_refuse_an_open_quote`: the
  #1052 part 1 follow-ups F4, F5 and the composed-mode finding, fix
  round 1052g2's C1, C2, C4 and C6, fix round 1052g3's F1 to F3, fix
  round 1052g4's S1, S2 and S4, fix round 1052g5's C1, C2, C3 and
  the whole-dump row grammar, fix round 1052g6's F1 to F4, and fix round
  1052g7's closure, tab rule, node column, row value rule and
  plain-value set. The F1
  suite binds the unread-block dump language: every quoted-scalar,
  flow, anchor, alias, indicator, tab or double-space shape above
  refuses `Unreadable`, and the admitted shapes (balanced quoted value
  and key, `!!js '!ctx.get(''p'')'`, a plain scalar with quotes, empty
  flow collections, and `>-` content holding a closed quote and bracket
  before a two-space key is read again) pass. The F2 test binds the
  policy `inject:` block to the one measured `    - headlessStartup`
  line: the measured block passes and a mode-looking line, a second
  item, a quoted item, a wrongly indented item, an empty block and a
  repeated `inject:` refuse. Each test was made to fail by the removal
  control recorded in the commit that added it and then restored; the
  class test's controls remove one arm of the break-and-control refusal
  set at a time, and each removal makes its class's cases fail. F3 is
  the corrected comment in `composed_rows.rs`; F4 is `read_deep`'s
  spelled admitting arms, so a new `Block` variant is not admitted by a
  wildcard.

Fix round 1052g7's tests bind the closure alone and through `gate`. The
refusal set covers C1a to C1f, C2 and the adversarial HIGH, the quoted
key with spaces before its colon, an open flow after a quoted key, and
the `\"` and backslash-continuation quotes; the admitted set covers the
measured line-77 shape in a tagged block, a closed quoted key with
spaces, `!!js '!ctx.get(''p'')'`, and both committed real-tree fixtures.
(1052g8 moves this round's admitted closed flow `[a, "b"]` from block
content to the refusal set: a non-empty flow at a node's start is now
refused.) The node-column test reads a line at the header's column and
skips one at node column plus two. The tab tests cover C3a, C3b and a tab
inside block content. The plain-value test refuses `- a`, `- "x`,
`j: "x`, `]`, `,` and `:` and admits `-1` and `a-b`. The row grammar's
two inline tests read the `disabled:`, `- id:` and `name:` values and
refuse the open quote and the header on their own; the C2 gate cases
still refuse through the closure when the bare word is restored. Each
removal control and its observed failing test is recorded in the code
commit message.

Fix round 1052g8 rebuilds the closure's tests around the three carriers and
adds the row grammar's wider-marker and ` #` refusals. Refused through the
pass alone and through `gate`: the adversarial HIGH; `[]: "x`, `{}: "x`
and `*a : "x`; the four empty-item folds (`-`, `-   `, `-   # c` and
`- - a` / `      -`) before the policy row; `k: [a, # ]`,
`"k": [a, # ]`, `k: [?'a]', b`, `k: &a[b,`, `k: [a]`, `k: {a: b}`,
`k: &a "x` and `? "x`. Admitted through both: `k: 'a #b'`, `k: "a #b"`,
`- id: 'a #b'` (read by the passes, then refused by the judge as
`Unmeasured` for its id), `- null`, `- []`, `k: []`, `k: {}`, `"k" : v`,
the measured lines 76 and 77 in a `!!js >-` block, and
`      with a # title. Make` in folded content. The row-grammar tests
refuse `-   k: >-` / `j: [a]`, `-  k: >-` / `j: *a` and `-  "k": >-`,
and the two-space `disabled: true # c` on the tool-bash and sandbox rows.
The inline units read the wider marker, the bare item's node column and a
`}` plain start. Each removal control (a) to (k), R2, the ` #` rule, `}`
and `Header::Item` was a compiling mutation observed failing and restored;
the code commit message records each with its failing test.

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

## A real 0.2.2-rc.1 composed tree at this round's head (2026-11-12)

Fix round 1052g5 re-ran the method on `git archive` of its own code
commit `270f7fe98`, never committed. The throwaway test built both
overlays with `hands_rows(Release::V0_2_2, mode, "/bin/brokkr",
["hands", "serve"])`, ran `dsh --profile headless --patch <overlay>
--dump-config` from the isolated `/var/tmp/dsh-0.2.2-rc.1` prefix under
the controller's scratch home, and handed that same command to
`composition_gate`. This box's `/var/tmp` is read-only, so the prepared
home was copied byte-for-byte to `/tmp` inside the one measuring call and
used as `DSH_HOME` there; the prepared home was not modified. The test
ran each mode twice, once capturing the dump for its bytes and digest and
once handing the command to the gate.

| Mode | Dump | Rows | Gate |
| --- | --- | --- | --- |
| `read-only` | exit 0, 16,703 bytes, sha256 `2c484c25b7a202b2…` | 100 | `Ok(())` |
| `workspace-write` | exit 0, 16,757 bytes, sha256 `f01745717859d7b2…` | 100 | `Ok(())` |

Both dumps are 0.2.2's plain 99 plus the `brokkr-hands` insert. Their
`sandbox-policy` row carries the seat's mode in its `config:` block and
the measured

```yaml
  inject:
    - headlessStartup
```

block. The plain 0.2.2-rc.1 dump carries that same inject block (its
`config:` block also names `workspaceRoot` and
`additionalWritableRoots`, which the seat overlay replaces with the one
mode). At `270f7fe98` the policy row's config block was the only one read
by the span grammar and its inject block was unread, as every other
row's was. Fix round 1052g6 narrows that: the policy `inject:` block is
now admitted only as the one measured `    - headlessStartup` line, and
any other content refuses. 1052g3 and 1052g4 admitted no `inject:` on the
policy row and would have refused this real tree as `Unreadable`; the
measured tree bounds the grammar, and the measured block is what 1052g6
admits. The 0.2.1 committed fixture still passes: its `sandbox-policy`
row has no inject block, and the measured 99 rows are unchanged.

The 17096db85 table above measures a different head, before 1052g3
closed the policy span; its dumps carry the same composed rows. This
round did not re-run that head, so the byte-count difference is not
compared here.

## A real 0.2.2-rc.1 composed tree at 1052g6's head (2026-11-13)

Fix round 1052g6 re-ran the method on `git archive` of its code commit
`be0a9aa9a`, never committed. The throwaway test built both overlays with
`hands_rows(Release::V0_2_2, mode, "/bin/brokkr", ["hands", "serve"])`,
ran `dsh --profile headless --patch <overlay> --dump-config` from the
isolated `/var/tmp/dsh-0.2.2-rc.1` prefix (dsh 0.2.2-rc.1, js-yaml
4.3.2) under the controller's scratch home, and handed that same command
to `composition_gate`. This box's `/var/tmp` is read-only, so the
prepared home was copied byte-for-byte to `/tmp` inside the one measuring
call and used as `DSH_HOME` there; the prepared home was not modified.
The test ran each mode twice, once capturing the dump for its bytes and
digest and once handing the command to the gate.

| Mode | Overlay bytes | Dump | Rows | Gate |
| --- | --- | --- | --- | --- |
| `read-only` | 913 | exit 0, 16,821 bytes, sha256 `abf29929512fdfd1…` | 100 | `Ok(())` |
| `workspace-write` | 919 | exit 0, 16,875 bytes, sha256 `012c2b655d415036…` | 100 | `Ok(())` |

Both dumps are 0.2.2's plain 99 plus the `brokkr-hands` insert, and both
pass the gate under the new unread-block rule and the measured policy
`inject:` rule. Their `sandbox-policy` row carries the seat's mode in its
`config:` block and the measured `inject:`/`- headlessStartup` block; the
0.2.1 committed fixture still passes with its own 99 rows. This round did
not compare its byte counts with the 1052g5 table above, which measured a
different head and working directory.

## A real 0.2.2-rc.1 composed tree at 1052g7's head (2026-11-13)

Fix round 1052g7 re-ran the method on `git archive` of its code head
`8ff369ac5`, never committed. The throwaway test built both overlays with
`hands_rows(Release::V0_2_2, mode, "/bin/brokkr", ["hands", "serve"])`,
ran `dsh --profile headless --patch <overlay> --dump-config` from the
isolated `/var/tmp/dsh-0.2.2-rc.1` prefix (dsh 0.2.2-rc.1, js-yaml
4.3.2) under the controller's scratch home, and handed that same command
to `composition_gate`. This box's `/var/tmp` is read-only under the seat's
file sandbox, so the prepared home was copied byte-for-byte to `/tmp`
inside the one measuring call and used as `DSH_HOME` there; the prepared
home was not modified. The test ran each mode twice, once capturing the
dump for its bytes and digest and once handing the command to the gate.
The dump's bytes depend on `DSH_HOME` and the overlay path, so this table
is not byte-comparable with the 1052g6 table above. The coverage commit
`8ff369ac5` was re-measured the same way: it also edits `rows.rs` and its
new `rows/carriers.rs`, so it is not a test-only diff; its no-behaviour-
change claim is about the closure's decisions, not its file list. The two
dumps keep their lengths (16,975 and 17,029 bytes) and the digests change
because the random `/tmp` home path changed in the `# ==` lines.

| Mode | Overlay bytes | Dump | Rows | Gate |
| --- | --- | --- | --- | --- |
| `read-only` | 913 | exit 0, 16,975 bytes, sha256 `38a4914494cae17d…` | 100 | `Ok(())` |
| `workspace-write` | 919 | exit 0, 17,029 bytes, sha256 `4e769ed08417386a…` | 100 | `Ok(())` |

Both dumps are 0.2.2's plain 99 plus the `brokkr-hands` insert, both
carry no tab, and both pass the gate under the new line-property closure,
the dump-wide break, control and tab rule, the node-column unread-block
rule and the measured policy `inject:` rule. Their `sandbox-policy` row
carries the seat's mode in its `config:` block and the measured
`inject:`/`- headlessStartup` block; the 0.2.1 committed fixture still
passes with its own 99 rows. The seat's own gate commands measured the
same commit: `cargo test -p brokkr-protocol --lib --all-features
--locked -- adapters::` passed 659 tests, including
`the_real_tree_fixtures_pass_the_closure_and_the_gate`.

## A real 0.2.2-rc.1 composed tree at 1052g8's head (2026-11-13)

Fix round 1052g8 re-ran the method on `git archive` of its code commit
`03b732efb`, never committed. The throwaway test built both overlays with
`hands_rows(Release::V0_2_2, mode, "/bin/brokkr", ["hands", "serve"])`,
ran `dsh --profile headless --patch <overlay> --dump-config` from the
isolated `/var/tmp/dsh-0.2.2-rc.1` prefix (dsh 0.2.2-rc.1, js-yaml
4.3.2) under the controller's scratch home, and handed that same command
to `composition_gate`. This box's `/var/tmp` is read-only under the seat's
file sandbox, so the prepared home was copied byte-for-byte to `/tmp`
inside the one measuring call and used as `DSH_HOME` there; the prepared
home was not modified. The test ran each mode twice, once capturing the
dump for its bytes and digest and once handing the command to the gate.
It also read the plain 0.2.2-rc.1 dump at
`/var/tmp/dsh-scratch-1052g5/dump.txt`, whose closure is `Ok(())`. The
dump's bytes depend on `DSH_HOME` and the overlay path, so this table is
not byte-comparable with the 1052g7 table above: the `/tmp` copy's path is
longer than that round's, and the `# ==` lines carry it.

| Mode | Overlay bytes | Dump | Rows | Gate |
| --- | --- | --- | --- | --- |
| `read-only` | 913 | exit 0, 16,840 bytes, sha256 `f815f6ce18a81afe…` | 100 | `Ok(())` |
| `workspace-write` | 919 | exit 0, 16,894 bytes, sha256 `887d682a7289c696…` | 100 | `Ok(())` |

Both dumps are 0.2.2's plain 99 plus the `brokkr-hands` insert, both carry
no tab and neither carries a bare `-`, a marker followed by more than one
space, a non-empty flow at a node's start or an empty sequence item. Both
pass the gate under the rebuilt line-property closure (empty item
included), the dump-wide break, control and tab rule, the wider-marker
refusal, the two-space ` #` rule, the node-column unread-block rule and
the measured policy `inject:` rule. Their `sandbox-policy` row carries the
seat's mode in its `config:` block and the measured
`inject:`/`- headlessStartup` block; the 0.2.1 committed fixture still
passes with its own 99 rows. The seat's own gate commands measured the
same commit: `cargo test -p brokkr-protocol --lib --all-features
--locked -- adapters::` passed 666 tests, including
`the_real_tree_fixtures_pass_the_closure_and_the_gate` and the two
1052g8 closure tests.

## Pending

- Exact coverage and `quality/ratchet.sh crap`: this seat's box has no
  LCOV output, so the crap ratchet refuses with "no LCOV at
  target/coverage/lcov.info: run scripts/coverage-exact.sh first". Every
  new line has a test, but the measured exact-coverage result and the
  crap ratchet are the host's.
- macOS, on every figure. This record proves the release tables and the
  checks on Linux only; macOS is still a supported host (decision 0063).
- A live `mcp__brokkr__workspace` call through this driver on 0.2.2-rc.1.
  The controller measured the tool lists, not a call of Brokkr's server
  under a seat.
- dsh's native egress and any OFF control for it (decision 0065 slice one;
  owed to the controller).
- A planted write through any remaining path in a read-only seat with the
  composed hands row, on 0.2.2-rc.1 as on 0.2.1.
