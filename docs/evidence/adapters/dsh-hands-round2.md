# dsh hands, round 2: the measured composed tree, the activation refusal and the access carrier

Date: 2026-10-26. Host: Linux. dsh: 0.2.1-rc.2 (the installed `dsh`,
`--profile headless`, the operator's default home, route
`deepseek-official/deepseek-flash`). The controller ran the measurements
below; this seat read them, copied them here and built the checks from
them. The raw files are in this directory under `dsh-hands-round2/`.

## What was measured

- `dump-stdio.txt` is `dsh --profile headless --patch overlay-stdio.yml
  --dump-config`: 99 rows (ids and package names), the operator's default
  home included, 0.06–0.07 s and about 64 MB per call, no model call. Two
  route `baseURL`s in the copy named a private single-label host, so they
  read `spark.invalid` here (#532: a route is named by its route name); the
  99 row ids and package names, which are the allowlist, are unchanged.
- `overlay-branch.yml` is the branch's row as first written: ten disabled
  tool rows, the sandbox row, and a `dsh-mcp-client` insert with
  `serverName`, `command` and `args` and **no `transport`**. Its server
  started 0 times and the model's tool list held no `mcp__probe__ping`
  (`live-branch.txt`); dsh exited 0 and said nothing on stderr.
- `overlay-stdio.yml` adds `transport: stdio`. The server started twice
  (the README's negotiation probe, then the serving process) and the tool
  list held `mcp__probe__ping` (`live-stdio.txt`).
- `overlay-failclosed.yml` has `transport: stdio`,
  `failOnStartupError: true` and a command that does not exist. dsh still
  ran the session and exited 0, printing on stderr
  `dsh: warning: 1 entry did not activate` and
  `brokkr-hands (@deepseek-ai/dsh-mcp-client): Error: mcp-client(probe):
  initial connection or tool synchronization failed`
  (`live-failclosed.txt`). `overlay-failopen.yml`, the same row without
  `failOnStartupError`, ran with no warning at all
  (`live-failopen.txt`).
- With an MCP row composed the kept bookkeeping set is **12** tools, not
  nine: `mcp-resources` adds `list_mcp_resources`,
  `list_mcp_resource_templates` and `read_mcp_resource`
  (`live-stdio.txt`). It can reach only the servers in the seat's scope:
  the composed-tree check admits exactly one `dsh-mcp-client`, Brokkr's
  `brokkr-hands`.

## What the round changed

1. The written row now carries `transport: stdio` and
   `failOnStartupError: true`, in the field order the measured overlay
   uses. `the_hands_rows_are_the_measured_overlay_and_the_workspace_server`
   pins the whole row's bytes, and
   `the_workspace_server_row_carries_the_measured_transport_and_startup_policy`
   names the two fields.
2. Every hands launch runs `dsh --profile headless --patch <the seat's
   overlay> --dump-config` under the seat's own environment and home and
   refuses every row outside the measured 99, a second `dsh-mcp-client`,
   an enabled measured effectful row, a repeated id, an unreadable dump
   and a failed dump. The allowlist in
   `crates/brokkr-protocol/src/adapters/dsh_hands/composition.rs` is held
   to `dump-stdio.txt` row for row by
   `the_composed_tree_is_the_measured_dump_and_refuses_an_ambient_row`,
   and a planted ambient row is refused by id and package by
   `a_launch_composes_the_seats_own_tree_and_refuses_an_ambient_row`.
3. The driver reads the startup dsh wrote:
   `the_dsh_startup_report_fails_closed_and_carries_partial` binds the
   `entry did not activate` warning, the row's own error line,
   `SANDBOX_UNAVAILABLE` and the launcher's
   `landlock-run: partial enforcement (older Landlock ABI)` line. An
   activation failure and an unavailable sandbox refuse the seat; partial
   enforcement is written as `sandbox: partial` on the finished row.
4. An office's access is typed data. The eight read-only offices now
   declare `"access": "read-only"`; absent is `writes`. The engine marks
   the site with `seat_access`, the dsh driver decodes it once into
   `read-only` or `workspace-write`, and a work-class panel position
   hired from a read-only office keeps `read-only`
   (`a_read_only_office_hired_at_work_class_carries_read_only_access`,
   `only_a_boxed_site_with_hands_carries_its_access`).

## Exact words bound from the installed packages

`report-words.txt` records the package and line each word came from:
`SANDBOX_UNAVAILABLE` and the `SandboxUnavailableError` sentence in
`@deepseek-ai/dsh-sandbox/lib/index.js`, and
`landlock-run: partial enforcement (older Landlock ABI)` from
`@deepseek-ai/node-addon-system`'s `LAUNCHER_BIN` and
`@deepseek-ai/dsh-sandbox-local/lib/index.js`.

## Still unmeasured

- macOS, on every figure. This round proves the overlay and the checks on
  Linux only; macOS is still a supported host (decision 0063).
- A live `mcp__brokkr__workspace` call through **this** driver. The
  controller measured an equivalent one-tool stdio probe server answering
  `mcp__probe__ping`, not Brokkr's server under a seat.
- A planted write refused by dsh's sandbox under a read-only office with
  the composed hands row; the box's own read-only bind reaches `hands.rs`,
  which decision 0065 slice two's other session owns, so this round does
  not build a read-only `HandsSpec`.
- dsh's native egress and any OFF control for it (decision 0065 slice one;
  owed to the controller).
- Route scope: the shape carries none, as the adapter's earlier shapes
  say.
