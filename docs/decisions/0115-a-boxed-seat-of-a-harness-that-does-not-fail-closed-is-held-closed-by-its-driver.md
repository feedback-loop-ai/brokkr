# 0115 — A boxed seat of a harness that does not fail closed is held closed by its driver

Status: proposed
Date: 2026-11-22
Built: built

## Context

Decision 0075 ruling 4 boxes a provisional harness's hands as a trusted
harness's are: where a site boxes its hands (decisions 0043 and 0046), the
seat's commands run in Brokkr's box through the engine's `workspace` tool.
For Claude Code and Codex the compiled hands fragment and the harness
itself keep the seat boxed. Qoder CLI (#1178, #1235) is different in two
ways, both measured on Qoder CLI 1.1.95 on Linux
(`docs/evidence/adapters/qoder-cli-1-1.md`, Second measurement):

- its adapter declares the empty hands fragment, because `brokkr driver
  qoder` composes the whole launch, so nothing compiled keeps the seat
  boxed;
- Qoder does not fail closed. A server that does not start is listed
  `disconnected` in init and the turn runs to `success`, exit 0
  (`box-server-does-not-start`). A server that dies mid-turn gives the
  model a tool error and the turn runs to `success` (`box-server-dies`).
  A call to a tool the launch does not hold is an entry in the result's
  `permission_denials`, and the turn still ends `success`
  (`box-other-natives`, `box-ungranted-mcp-tool`).

So Qoder's own exit, `is_error` and subtype say nothing about whether the
box held. Only init's lists, the tool results' words and the result row's
denials show it, and the driver must read them.

Alternatives weighed:

- **Fail every turn, boxed or not, on a denial.** Level 1's unboxed seat
  folds a denied call as every call is folded, and its turn ends as its
  result says (`the_denial_turns_fold_their_refused_call_and_are_not_refused`).
  An unboxed seat's denial is Qoder's permission check working, not a
  breach, and changing it would change level 1's seat for no gain.
- **Read `isHardFailure`.** The dying server's errored result carries
  `tool_use_result: {"isHardFailure": true}` in the one recording that has
  it. One recording does not say what else Qoder marks so, or whether it
  marks every lost server so, so it is not read.
- **Refuse the unmeasured boxed seat only at launch.** Round one of #1235
  did, and a bundle that compiles and then cannot launch is a refusal moved
  late.

## Rulings

1. **The boxed launch.** One value, made once from the engine's MCP set,
   decides whether a qoder seat is boxed (`Hands::of`). A boxed launch is
   the unboxed one with exactly three things changed:
   `--permission-mode dont_ask` for `accept_edits`; the engine's grant,
   `--allowed-tools mcp__brokkr__workspace`, as the launch's one grant;
   and `--tools ""` for Qoder's six tools and the meta-tools' denial. A
   boxed seat that passes `--allowed-tools` of its own is refused before
   Qoder starts, naming the control and never its value.

   **Enforcement binding:** `adapters::qoder::tests::boxed::a_boxed_launch_holds_the_engines_grant_and_no_tool_of_qoders_own`,
   `the_unboxed_and_boxed_launches_differ_in_exactly_the_mode_the_grant_and_the_tools`
   and `a_boxed_seat_with_a_grant_of_its_own_is_refused_naming_the_control`.

2. **What init must show.** A boxed turn's init lists the engine's server
   `brokkr` alone and `connected`, and no tool but `mcp_call`, `mcp_get`
   and `mcp_list`. Anything else stops the stream at init: a missing or
   unreadable server list, which names the engine's server; another server
   beside it, named with its status; the engine's server under another
   status; a tool of Qoder's own. A later Qoder that keeps a native tool
   under `--tools ""` lists it there and fails, rather than running
   unboxed.

   **Enforcement binding:** `adapters::qoder::tests::boxed::a_boxed_turn_whose_server_did_not_start_stops_at_init`
   and `a_boxed_init_beside_a_native_tool_or_another_server_stops`.

3. **A boxed turn succeeds only on its own terminal evidence.** The seat's
   result is taken only when init arrived and passed ruling 2, the
   stream's result row arrived, and that row holds `permission_denials` as
   an empty list. A stream that ends before its result row, a result row
   without the member, and a member that is not a list each end the turn
   as a failure that says which. The seat's result file is never read
   before that: a failure behind a folded turn ends the seat lost, with no
   result sent. A non-zero exit already fails the turn, so it keeps the
   refusal it states. The unboxed turn ends as before.

   **Enforcement binding:** `adapters::qoder::tests::boxed::a_boxed_turn_without_its_terminal_evidence_fails_saying_which`
   and `a_driven_boxed_turn_that_fails_late_sends_no_result`.

4. **A denial fails a boxed turn, and not an unboxed one.** In a boxed
   launch each denial is a call to a tool the seat does not hold, so the
   turn fails on it. Level 1's unboxed seat folds a denied call and ends as
   its result says, as the Context records.

   **Enforcement binding:** `adapters::qoder::tests::boxed::a_boxed_turn_with_a_permission_denial_fails`
   and `adapters::qoder::tests::the_denial_turns_fold_their_refused_call_and_are_not_refused`.

5. **The lost-server check rests on Qoder's words, and answers to its
   call.** A server that dies mid-turn shows only as an errored
   `tool_result` whose text is exactly `Error: MCP server 'brokkr' is not
   connected`. It counts only where its `tool_use_id` is the id of an
   earlier `tool_use` in the same stream that called `mcp_call` with the
   engine's granted tool as `toolName`. The same words on a result of any
   other call do not end the turn by this check. A later Qoder that words
   it differently is not caught by it; ruling 3 still requires the result
   row. `isHardFailure` was seen and is not used: it is unmeasured.

   **Enforcement binding:** `adapters::qoder::tests::boxed::a_boxed_turn_whose_server_died_fails_at_qoders_words`
   and `the_lost_server_words_count_only_on_a_result_of_an_engine_call`.

6. **Rows are read typed, and a row of no measured shape fails.** Each
   row a boxed turn is judged by is deserialized once from Qoder's bytes
   into the typed row its checks read. A row of a checked type whose shape
   no measured stream carries fails the turn, since it shows nothing.

   **Enforcement binding:** the typed rows in
   `crates/brokkr-protocol/src/adapters/qoder/hands.rs`, and the recorded
   boxed turns that end as answers
   (`the_recorded_boxed_turns_that_reached_the_hands_tool_end_as_answers`).

7. **An unmeasured boxed seat is refused when its bundle compiles.** A
   harness whose driver composes its boxed launch whole, Qoder today, has
   no compiled fragment to keep an unmeasured seat boxed. So its seat with
   boxed hands on a host or shape its adapter declares no measurement for
   is refused by the compile whichever way the MCP fence stands, in the
   words a missing measured shape draws past the lifted fence. Every other
   harness's verdict, and every qoder set without boxed hands, waits on the
   fence as before. The driver's launch guard stays as the second line: a
   boxed set no measured shape admitted is refused before Qoder starts,
   never launched with Qoder's own tools.

   **Enforcement binding:** `bundle::tier::tests::the_standing_fence_refuses_a_boxed_qoder_seat_on_a_host_with_no_measured_shape`
   and `adapters::qoder::tests::an_unadmitted_hands_set_under_the_standing_fence_never_starts_its_harness`.

## What is not claimed

- No boundary of the operating system stands around Qoder's own process.
  It runs on the host and talks to its provider from there. The box holds
  the hands tool's commands alone. Qoder's native tools are absent because
  its own `--tools ""` leaves them out of its tool list, inside its own
  process, and ruling 2 checks that it says so.
- Nothing checks the installed Qoder version (#1240). The checks above are
  what a later version would fail on, not a pin.

## Consequences

- A boxed qoder turn that Qoder ran to `success` can still fail: on a
  server it did not reach, a server it lost, a denial, or a missing or
  unreadable result row. Each failure names its cause in the driver's own
  words, which name only plain ids.
- On macOS, where nothing of Qoder's is measured, a boxed qoder seat no
  longer compiles. A qoder seat without hands compiles there as before.
- A future harness whose driver composes its boxed launch whole joins
  ruling 7 by its `AdapterKind` arm.
