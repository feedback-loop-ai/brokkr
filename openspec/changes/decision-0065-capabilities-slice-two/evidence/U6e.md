# U6e evidence: serve the filtered protocol

U6e is delivered by run `0065-slice-two-unit-u6e-see-the--d6424e8f` on
branch `s2/U6e`, cut from main at `e313dee0`, as one signed commit that
also carries the operator's staged addendum of 2026-10-11 in
`operator-ruling-2026-10-03.md`. It closes tasks 30.1 and 30.2. Everything
below was observed in this run's seats on Linux, where real boxes stand; what
was not observed is marked pending at the end. The run's review returned the
implementation twice: first with seven medium and two low findings, whose
answers are recorded under "The review's return", then with four medium and
five low findings, whose answers are recorded under "The review's second
return". The sections before them describe the first visit's tree except
where they say otherwise.

## What changed

The protocol is the new private module `crates/brokkr-cli/src/broker/rpc.rs`,
declared `mod rpc` from `broker.rs`, with two private children: `rpc/calls.rs`,
the harness-facing call path, and the test module `rpc/tests.rs`. The call
path is a child because `rpc.rs` and it together are 975 lines, past the 800
ceiling (the operator's standing rule of 2026-10-09, extended 2026-10-11).

`rpc::started` speaks MCP 2025-06-18 over newline-delimited JSON-RPC 2.0 with
the admitted box's server, over the server's own stdin and stdout and nothing
else, all within the startup's absolute deadline: it asks to initialize at
MB3's version, refuses a server answering another protocol version or a
`serverInfo.version` other than the dialect's pin, sends the initialized
notification, then lists the whole catalog page by page (at most 32 pages and
1,024 tools, no cursor twice) and keeps only the plan's granted tools, as the
server describes them, in its order. A granted tool the complete listing lacks
ends the session. Every response must answer the one request outstanding by
its exact typed id (`Id::Number` and `Id::Text` never compare equal); a frame
that answers nothing is never the harness's: the server's ping is answered
with `{}`, any other request it makes refused with the method cause, and a
notification dropped, each spending the operation's 1 MiB budget of
non-response bytes, which never extends a deadline. Frames are read without
blocking by one bounded reader (`Lines`): a frame over its bound (8 MiB from
the server, 1 MiB from the harness) is refused once and passed over to its
end, and JSON nesting past 64 is refused before parsing.

`rpc/calls.rs::served` is the harness-facing path, built and tested now and
consumed when U6f removes the serving refusal; its entry carries the one
`#[cfg_attr(not(test), expect(dead_code, reason = "consumed when U6f removes
the serving refusal"))]` the operator ruled. It answers initialize (at MB3's
version only), ping and the filtered listing itself, passes a cancellation of
the active call on, and refuses every other method. A tools/call is checked
whole before anything is recorded: MCP's parameter shape (`name`,
`arguments` an object, `_meta`), a name within SC4's 256 bytes (else the
request-limit cause), and seat-record's tool vocabulary (else the
invalid-call cause). Acceptance asks the ledger for room before each frame is
read (at 4,096 calls no frame is read again, though an active call is still
answered and delivered, and the session then ends with the ledger's limit),
then makes the Started durable. An accepted call to a tool the plan does not
grant, or made while another is active, is refused with a durable refused
Terminal and forwarded nowhere; any other is forwarded with a request id of
the broker's own, under a deadline of 120 seconds bounded by the attempt's.
Its Terminal is durable before its response is delivered under the harness's
own id; a cancelled call is given two seconds to settle and its response is
never delivered. A Started or Terminal that cannot be recorded is answered
with "broker call could not be recorded" and ends the session with nothing
forwarded or delivered. A fatal ending fails the active call with a durable
Terminal, tells the harness why unless it cancelled the call, and ends the
session.

The caller-facing refusals are the private `Refused` enum in `rpc.rs` with the
operator's five texts, plus MB3's "MCP protocol version is not supported by
this broker" for a harness that asks another version. Session endings are the
private `Fatal`: a cause the ledger's `Failure` names, or MB3's own
unsupported-version, missing-granted-tool and catalog-limit causes, each
latched through `Failure` (`Fatal::failure`). Nothing new is public:
`quality/ratchet.sh api` holds.

The seams are the two existing children the operator's addendum names.
`session/readiness.rs`'s Linux `ready` now takes a continuation and hands it
the live launched box's stdio (`Link`) once the exec status pipe permits
initialization, so the box is still settled on every path when `ready`
returns; `polled` and `SLICE` moved to its top level so the portable
protocol reuses them. (A shared `written` the earlier visits put there is
gone again since the M1 repair below: `sent` writes as it does on main, and
the protocol queues its writes to the harness instead.)
`session/lifetime.rs`'s `lived` hands the opened ledger
to the session, and the latching and closing became `settled`, which reads
the session's typed ending (`Ended::Failed(Fatal)` is new) and latches its
first cause, or SD3's incomplete serving where the session answered. `serve`
runs `rpc::initialized`, which starts the server and lets it go, so the box is
ended and `serve` still refuses with `ServingIncomplete` before any message
reaches the harness; a startup failure ends `serve` with its own cause instead,
latched in the ledger. `session.rs` re-exports those helpers in its existing
`use` lines and is now 799 lines.

## Assumptions and rulings applied

The ledger's catalogue has no variant for a missing granted tool, a catalog
past its bounds or a server's unsupported protocol version, and nothing new
may be public. They latch as `Failure::Protocol`, `Failure::Response` and
`Failure::Protocol` respectively, while `serve` prints MB3's own text for each.
The call deadline is `min(now + 120 s, attempt deadline)`: `served` takes the
attempt's deadline and cancellation as a parameter, as MB3 bounds every call
by the attempt's deadline too. A harness that ends its input while a call is
active leaves that call begun (CR4 reads it interrupted) and the session
unclosed. A name the catalog lists twice ends the session with the catalog
cause (third visit; the first two ignored a granted tool's second listing).
The retention
budget denial (`Denial::Budget`) is not exercised here: reserving retention
shares is a later unit's.

## Tests

The protocol's unit suite `crates/brokkr-cli/src/broker/rpc/tests.rs` (new,
1,297 lines) runs real anonymous pipes: a fake server on its own thread, a
harness on another, and the ledger in memory with one append's fault
injected, settled exactly as `serve` settles it (`lifetime::settled`) and read
back through `recover`. Its 21 tests, all passing (`cargo test -p brokkr-cli
--lib broker::` 68 passed, 0 failed):

| Test | Exact assertion |
| --- | --- |
| `each_cause_reads_in_mb3s_words_and_latches_through_the_ledgers_own` | Every `Refused` text and JSON-RPC code; every `Fatal` text, through `Ended`, and the `Failure` it latches |
| `startup_initializes_lists_and_offers_exactly_the_grant` | Catalog is exactly `lookup` and `read` as described (admin left out); the server read exactly initialize, initialized, tools/list |
| `a_server_off_its_pinned_version_or_mb3s_protocol_ends_the_session_before_any_listing` | `Failed(Version)` and `Unsupported`; the server read only initialize |
| `the_whole_catalog_is_paged_and_filtered_and_a_granted_tool_it_lacks_ends_the_session` | Two pages filtered and deduplicated; the second asked with `{"cursor":"c1"}`; `Missing("read")` |
| `the_catalogs_page_tool_and_cursor_bounds_hold_exactly` | 32 pages of 1,024 tools pass; 33 pages, 1,025 tools and a repeated cursor are `Catalog` |
| `a_response_must_answer_the_one_request_outstanding_by_its_exact_typed_id` | `"1"` for `1`, another id, an error, both bodies, an extra member, `jsonrpc` 1.0, no server info, no id, not JSON, a nameless tool and the output's end are each `Failed(Protocol)` |
| `a_servers_requests_never_reach_the_harness_and_every_frame_spends_its_budget` | Ping answered `{}`, sampling refused `-32601`, notification dropped; exactly 1 MiB of notices passes, one byte more is `Failed(Response)` |
| `a_response_over_its_bytes_or_depth_ends_the_session_and_one_at_both_bounds_passes` | An 8 MiB, 64-deep listing passes; one byte or one level more is `Failed(Response)` |
| `no_traffic_extends_the_startup_deadline_and_a_server_that_takes_nothing_ends_it` | Chatty and silent servers time out within 0.4–2 s of a 0.4 s deadline; a server whose input is closed is `Failed(Protocol)` |
| `the_harness_is_answered_here_and_its_granted_call_forwarded_once_recorded` | Exact greeting, pong, listing, method, version and invalid refusals, and the echoed result under the harness's id; one forwarded tools/call with the broker's id 3; ledger `[lookup Succeeded]`, closed `Refused(ServingIncomplete)` |
| `an_ungranted_call_is_recorded_refused_and_an_invalid_one_refused_unrecorded` | `admin` and the exact 256-byte name each one durable `Refused(Ungranted)`; the 257-byte name the request-limit cause; six invalid shapes or names the invalid cause, none recorded or forwarded |
| `requests_past_their_bounds_refuse_before_acceptance_and_ones_at_them_are_served` | 1 MiB + 1 and 2 MiB + 7 frames, depth 65 and non-JSON refused with a null id, the next frame taken whole; exact 1 MiB and depth-64 calls served |
| `a_call_made_while_another_is_active_is_recorded_refused_and_never_forwarded` | Active refusal first, then the first call's result; ledger `[lookup Succeeded, read Refused(Active)]`, one forwarding |
| `a_tools_error_is_its_calls_outcome_and_the_session_goes_on` | `isError` and a JSON-RPC error delivered and recorded `Failed`; the next call succeeds; the session ends normally |
| `a_mismatched_or_late_response_ends_the_session_and_forwards_nothing` | Wrong-type, unknown and initialize's ids end with `Failed(Protocol)`, the call `Failed`; a duplicate answer after delivery keeps the call `Succeeded` and latches `Protocol` |
| `no_traffic_extends_a_calls_deadline_and_none_spends_past_its_budget` | Progress and pings every 20 ms end at the attempt's deadline with `Failed(Timeout)` in under 3 s; 64 KiB notices past 1 MiB end with `Failed(Response)` |
| `a_cancelled_call_is_passed_on_and_never_delivered` | Unknown and malformed cancellations are nothing; the server receives `notifications/cancelled` for id 3; a settled answer is recorded but not delivered; an unsettled one ends in 2–4 s with `Failed(Timeout)` and nothing heard |
| `a_server_that_takes_no_call_ends_it_and_an_ended_harness_leaves_it_begun` | A closed server input is `Failed(Protocol)`; a held one is `Failed(Timeout)` by the deadline; an ended harness leaves `[lookup None]` with no closure and `Invalid::Lifecycle` |
| `a_servers_requests_during_a_call_are_answered_by_the_broker_alone` | The server reads the broker's `{}` and `-32601` answers; the harness hears only its result |
| `no_call_is_forwarded_or_delivered_unrecorded` | A Started not made: told, no forwarding, no record; a Terminal not made: forwarded, not delivered, begun; a refused Terminal not made: told; a fatal ending's Terminal not made: nothing heard |
| `at_the_call_bound_every_accepted_call_is_kept_and_the_next_frame_stays_unread` | 4,095 refused calls and a 4,096th forwarded one all kept; the ping after them unanswered; the active call still delivered; closed `Failed(Limit)` |

The CLI suite gains the child `crates/brokkr-cli/tests/capability_broker/protocol.rs`
(new, 156 lines, registered as `mod protocol` in `capability_broker.rs`). Its
five tests are real-box tests, which the controller runs on the host at two
CPUs; all five passed here, where real boxes stand:

| Real-box test | Exact assertion |
| --- | --- |
| `a_server_that_initializes_is_listed_and_serving_still_refuses_before_the_harness` | `ServingIncomplete`, one lookup, ledger closed `Refused(ServingIncomplete)`, nothing on stdout |
| `a_server_off_its_pin_or_lacking_a_granted_tool_ends_the_session_with_its_own_cause` | Version, unsupported-version and missing-tool texts, one lookup each, latched `Version`, `Protocol`, `Protocol` |
| `a_running_server_lacking_a_dependency_or_speaking_no_mcp_is_the_protocols_cause` | A missing in-box helper and a corrupt frame each "MCP server protocol is invalid", one lookup, latched `Protocol` |
| `a_server_that_never_answers_ends_at_the_startup_deadline` | "MCP server call timed out" between 29 and 32 s after `serve` began, one lookup, latched `Timeout`, nothing of the box left |
| `a_refusal_before_the_box_looks_nothing_up_and_starts_no_server` | The fixed-key refusal with zero lookups and its ledger, beside the endings above |

Existing tests that pinned "no MCP exchanged" move with the behaviour (the
test exception of 2026-10-05). The fixture's server in `capability_broker.rs`
now speaks the startup (`SPEAKS`), as do `plant`'s executables, and
`the_program_tree_is_mb3s_layout_of_the_executable` seals each program with
`-c SPEAKS`: before that change the test failed here with `/usr/bin/dash`
answering "MCP server protocol is invalid" in place of the admission it
pins. In `handoff.rs`, the bindings test's server speaks the startup; the
held-server test now finds the broker awaiting initialization rather than
held at the box's end, and with the bootstrap-killed and terminal tests now
expects "MCP server protocol is invalid", as no MCP server answers. The
readiness unit tests pass the new continuation. All 74 `capability_broker::`
tests passed here (`cargo test -p brokkr-cli --test it capability_broker::`,
182 s).

## Mutations

Each mutation compiled, was run against the named suite, and was reverted;
the restored tree passed every test again (68 broker unit tests; the eight
CLI tests below in one 43 s run).

| Mutation | Failing tests and assertion |
| --- | --- |
| `Refused::Method` code `-32600` | cause pins; startup requests; requests during a call; harness test |
| Version pin check removed | version test, `serverInfo` case: `Ok([..])` for `Err(Failed(Version))` |
| Protocol-version check removed | version test, `protocolVersion` case: `Ok` for `Err(Unsupported)` |
| Grant filter removed | startup, paging, bounds, startup requests and harness tests |
| Missing-tool check dropped | paging test; real-box `lacking` case got `ServingIncomplete` |
| Repeated cursor allowed | bounds test: `Ok(2)` for `Err(Catalog)` |
| Tool bound one higher | bounds test: `Ok(2)` for `Err(Catalog)` |
| One page more | bounds test: `Ok(2)` for `Err(Catalog)` |
| Startup id not correlated | typed-id test, `"1"` for `1`: `Ok` for `Err(Failed(Protocol))` |
| Budget one byte larger | startup budget case: `(1, Ok(2))` for `Failed(Response)` |
| Budget removed | also the call budget test: timed out in place of the response limit |
| Server requests unanswered | startup requests and requests-during-a-call frames |
| Depth bound one higher | depth-65 response passed; depth-65 request served |
| Over-long frame accepted | byte-over response passed; byte-over request read as invalid |
| Over-long tail not passed over | 2 MiB + 7 request: invalid in place of one limit |
| Call deadline never kept | call deadline and cancellation tests: the harness heard nothing in 20 s |
| Cancellation not bounded to 2 s | cancellation test: settled after 10 s |
| Cancellation not passed on | cancellation test: no `notifications/cancelled` frame |
| Cancelled answer delivered | cancellation test: the echo in place of the pong |
| Active-call denial removed | concurrent test |
| Ungranted denial removed | ungranted test; call-bound test |
| Unrecorded Started not answered | unrecorded test: `None` for the error |
| Delivery before Terminal | unrecorded test: the result in place of the error |
| Call response id not correlated | late-response and unrecorded tests |
| Failed call's Terminal skipped | five tests: ledgers without the failed call |
| Failed forwarding ignored | takes-no-call test: timed out in place of protocol |
| Held write read as protocol | takes-no-call test, held case |
| Tool vocabulary widened | ungranted/invalid test |
| Tool name bound one higher | ungranted/invalid test (257 bytes) |
| Request bound one lower | bounds test (exact 1 MiB) |
| Tool error recorded succeeded | tool-error test |
| Call bound ends with another cause | call-bound test: `Protocol` for `Limit` |
| Frames read past the call bound | call-bound test: 4,097 answers for 4,096 |
| Harness version not checked | harness test |
| Harness end fails the active call | takes-no-call test: `[lookup Failed]` for begun |
| Served fatal ending not latched | four tests' closures |
| Admitted session closes Clean | seven tests' closures |
| Startup waits 3 s past its deadline | startup deadline test: chatty took 3.4 s |
| `serve` skips the startup | six real-box tests: unpinned, dependency, timeout, held-server, bootstrap-killed, terminal |
| Healthy server refused at startup | real-box healthy test and `a_real_box_reports_ready…` |
| Both binding-name screens removed | real-box refusal test: the store was looked up |

One further mutation, the call bound never ending the session, made its test
hang rather than fail: the broker then never reads the harness's end. It was
stopped by `timeout 590` and replaced by the call-bound row above.

## Gates

Formatting and clippy (workspace, all targets, all features, locked) were
clean, and `cargo +1.88 check --workspace --all-targets --all-features`
finished clean. The brokkr-cli suites passed on the final tree: the library
(711 passed, 1 ignored), the integration binary in two halves (505 passed
and 2 ignored outside `capability_broker::`; its 74 passed in 174 s), and
`driver_conformance`, `transcript_surfaces` and the three heap binaries. A
first run outside the broker suite failed `status_pages::no_living_doc_says_a_tool_list_bounds_an_unboxed_seat`
on the phrase "box lacks" in the new suite's prose; it was reworded.
`openspec validate --all --strict` passed 19 of 19, and `typos --hidden`
and `git diff --check` (staged and unstaged) were clean. Both `recipes/self` and
`recipes/verify` compiled. `cargo crap --path crates/brokkr-cli/src` puts
every new or changed function at cyclomatic complexity 14 or below
(`decoded` 14, `Server::heard` 13, `served` 13, `Lines::next` 12, Linux
`ready` 12). A branch-coverage diagnostic (`cargo llvm-cov` on
nightly-2026-09-05 over the broker unit tests and `capability_broker::`)
left one line of the new code unexercised, the full-ledger path of
`served`; the call-bound test now drives it.

`quality/ratchet.sh` holds file sizes, duplication (after one test clone
was folded into `lookup_failed`) and the public API. Its `baselines
origin/main` check names one raised baseline: `quality/suppressions.txt`'s
production `expect dead_code` at 2 (was 1), the call path's expectation the
operator ruled on 2026-10-11, so the pull request carries "Ruling:
operator-ruling-2026-10-03.md, addendum of 2026-10-11". `dead_code` is not a
ratcheted lint. `quality/file-lines.txt` has rows for the new files and the
remeasured ones. No witness or compose input moved.

## The review's return

The review of `bf89d966` returned the implementation with findings M1–M7,
L1–L2 and I1–I2. The second visit amends that commit, inside the same row
files (`rpc.rs`, `rpc/calls.rs`) and test files; `session/readiness.rs` and
`session/lifetime.rs` did not move again.

| Finding | What changed |
| --- | --- |
| M1, the attempt unread while idle or waiting | `served` asks `Session::going` before every step, idle or not: past the attempt's deadline or on its cancellation the session ends with the timeout cause, failing an active call. `Lines::next` now makes at most one read per call and answers `Partial` when that read held no whole frame, so a harness or server that keeps a pipe full never keeps the loop from its deadline. |
| M2, writes bounded by a fresh 30 s | Every write to the harness is bounded by `Session::bound`: the active call's deadline, else MB3's 30 s, never past the attempt's. A write not taken whole by then stalls the output, and a write to a closed output marks it gone; nothing is written after either, so no frame is left unfinished with another after it. A stalled output ends the session with the timeout cause, a gone one as the harness's end does. Only the session's closing frame, which tells the harness why its call failed, has a fixed `CLOSING` second of its own once every other deadline has passed. |
| M3, the budget reset per page | `Server::begin` starts an operation's budget: once for initialization, once for the whole listing and once per call. `request` no longer resets it. |
| M4, constructed answers unbounded | `offering` refuses a filtered listing over 8 MiB with the response-limit cause at startup. A listing or a call's answer whose bytes with the harness's own id pass 8 MiB is never delivered: the harness is told the response-limit cause and the session ends with it, failing an active call. |
| M5, null arguments and scalar metadata accepted | `Called`'s `arguments` and `_meta` deserialize through `present`: absent is none, an explicit null or a non-object is the invalid-call cause before acceptance. |
| M6, response bodies untyped | A server frame's `id`, `result` and `error` deserialize through `present`, so an explicit null is never absent; a result must be an object and an error JSON-RPC's error object (`Fault`: `code`, `message`, optional `data`, no other member). A tools/call result must be MCP's shape (`Answered`: `content` an array of objects, `isError` a boolean, `structuredContent` and `_meta` objects where present); otherwise the session ends with the protocol cause, the call failed. The answer is serialized from the typed `Reply` under the harness's id (`Answer`). |
| M7, silence after an unrecorded Terminal | `Session::failed` tells an uncancelled caller "broker call could not be recorded" where the failed call's Terminal could not be made, and the session's cause stays its first fatal one. |
| L1, held frames lost after an over-long one | `Lines::framed` scans on past a frame it passes over before any further read, so the frames after it reach the caller before the pipe's end. |
| L2, the held-server test's broker lookup | `handoff.rs::stopped` takes only the process whose arguments are exactly `broker serve --plan <this fixture's locator>`, never the tracer or shell that carry them, nor another test's broker. |
| I1, `ended` built off Linux | `capability_broker.rs::ended` is `#[cfg(target_os = "linux")]`, as all its consumers are. |
| I2, gates unrun | The workspace suites, the budgets and every gate below were run on this visit's tree; the link check remains pending, and why. |

The protocol's unit suite is now 27 tests and 1,586 lines. Changed and new
tests on this visit, each passing on the final tree (`cargo test -p
brokkr-cli --lib broker::`, 74 passed):

| Test | Exact assertion |
| --- | --- |
| `a_pipe_is_read_once_a_turn_and_frames_held_past_an_over_one_are_read_on` (new) | Four bytes are `Partial`, then `Quiet`; twelve are `Over`; the over-long frame's end with `{a}` and `{b}` and the pipe's end yield `Frame({a})`, `Frame({b})`, `Ended` |
| `the_attempts_deadline_and_cancellation_end_an_idle_a_waiting_or_a_flooded_session` (new) | Idle: `Failed(Timeout)`, no calls, closed `Timeout`, in 0.4–3 s. Cancelled while a silent server holds a call: `[lookup Failed]`, `Timeout`, nothing heard, under 3 s. A never-ending over-long frame: one null-id request-limit refusal, then `Timeout` under 3 s |
| `a_harness_that_takes_no_answer_or_closes_its_output_ends_the_session` (new) | A harness reading nothing while a call waits: `[lookup Failed]`, `Timeout`, under 3 s. A closed output: `Ok`, the call after the ping never forwarded, no calls recorded |
| `a_listings_pages_share_one_budget_of_frames_that_answer_nothing` (new) | Half the budget before each of two pages passes; one byte more is `Failed(Response)`; four frames read either way |
| `a_listing_of_the_grant_past_mb3s_bound_on_a_response_ends_the_session` (new) | Two pages within the bound whose filtered listing is exactly 8 MiB pass; one byte more is `Failed(Response)` |
| `an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session` (new) | A listing and a call's answer each exactly 8 MiB with the harness's id are delivered; one byte more is the response-limit error to that id, then the end; ledgers `[lookup Succeeded]` and `[lookup Failed]`, closed `Response` |
| `an_ungranted_call_is_recorded_refused_and_an_invalid_one_refused_unrecorded` (changed) | Also null `arguments`, scalar and null `_meta`: the invalid cause, unrecorded; a call omitting `arguments` with object `_meta` forwarded as `{"name":"lookup","arguments":{}}` and recorded `Succeeded` |
| `a_mismatched_or_late_response_ends_the_session_and_forwards_nothing` (changed) | Also a null id, a text error, an error without a message or with another member, a scalar result, a result without content, content not of objects, a non-boolean or null `isError`, non-object `structuredContent`, a null result beside an error and a null error beside a result: each `[lookup Failed]`, `Protocol`, and "MCP server protocol is invalid" to the caller |
| `a_tools_error_is_its_calls_outcome_and_the_session_goes_on` (changed) | The tool error carries `structuredContent` and `_meta`, the JSON-RPC error `data`; each delivered whole and recorded `Failed` |
| `no_traffic_extends_a_calls_deadline_and_none_spends_past_its_budget` (changed) | Also a listing and a call each spending exactly their whole budget: the call delivered and `Succeeded` |
| `a_servers_requests_never_reach_the_harness_and_every_frame_spends_its_budget` (changed) | The same assertions, its notices now written by the shared `noise` helper |
| `no_call_is_forwarded_or_delivered_unrecorded` (changed) | A fatal ending whose Terminal is not made: the caller hears "broker call could not be recorded", the session ends `Failed(Protocol)`, the ledger keeps `[lookup None]` unclosed |

Each mutation below compiled, failed its test as named, and was reverted;
the restored tree passed all 27 protocol tests again.

| Mutation | Failing test and assertion |
| --- | --- |
| `going` ignores the attempt's deadline and cancellation | attempt test: the idle harness's "the broker answered nothing" after 20 s |
| `Lines::next` reads on until the pipe blocks | pipe test: `Quiet` for `Partial` |
| `Lines::framed` stops after a passed-over frame | pipe test: `Partial` for `Frame({a})` |
| Writes bounded by a fresh 30 s | harness test, stalled case: took 30 s, past the 3 s bound |
| A write to a closed output ignored | harness test, closed case: the call was forwarded |
| `request` resets the budget on every page | paged-budget test: `(1, Ok(2))` for `Failed(Response)` |
| The call does not begin its own budget | call-budget test: the response-limit error for the result |
| Startup listing bound one byte larger | grant-listing test: `(1, Ok(2))` for `Failed(Response)` |
| All three answer bounds one byte larger | grant-listing and caller-id tests |
| Only the call answer's bound one byte larger | caller-id test, second session: the result for the response-limit error |
| `arguments` reads null as absent | invalid-shapes test: the null-arguments call was answered |
| `_meta` accepts any value | invalid-shapes test |
| A malformed tool result recorded `Failed` | mismatched test, a result without content: the result was delivered |
| `error` reads null as absent | mismatched test, a null error beside a result: the result was delivered |
| An unrecorded Terminal tells nothing | unrecorded test: `None` for the recording error |
| The closing frame without its own second | call-deadline test: `None` for "MCP server call timed out" |

The second visit's gates ran on the final tree. Formatting and clippy
(workspace, all targets, all features, locked) were clean, and so was
`cargo +1.88 check --workspace --all-targets --all-features`. Suites:
brokkr-core, brokkr-protocol, brokkr-view, brokkr-store, brokkr-bridge and
brokkr-seatbelt-probe in one run, every binary passing (brokkr-protocol's
library 878 passed, 3 ignored); brokkr-runtime's library (930) and its
`it`, `capability_launch`, `operated_repo` and `queued_launch` binaries;
brokkr-cli's library (717 passed, 1 ignored), its `it` binary outside
`capability_broker::` (505 passed, 2 ignored), `capability_broker::` (74
passed in 193 s, the five real-box protocol tests and the changed handoff
test among them; the protocol child run alone with `--nocapture` passed 5
in 34 s and printed no skip notice), `driver_conformance` (27), `transcript_surfaces` (13) and the three
heap binaries. `scripts/measure-budgets.sh` measured 101 prompt sites, 329
packages and the three heap peaks; its rewrite moved only the files'
measured-on dates and some prompt sites downward, none of them an input
this unit changes, so the files were restored and the budget tests passed
against the committed budgets (`budgets::` 4 passed; heap 1 each). Both
`recipes/self` and `recipes/verify` compiled. `openspec validate --all
--strict` passed 19 of 19; `typos --hidden` was clean after one test name
was reworded; `git diff --check` was clean. `cargo crap --path
crates/brokkr-cli/src/broker` puts every function at 15 or below (`served`
15, `Server::heard` 14, `decoded` 14, `Session::requested` 11,
`Lines::framed` 9, `Session::answered` 8). A branch-coverage diagnostic
(`cargo llvm-cov --branch` on nightly-2026-09-05 over the broker unit
tests) took both directions of every branch in `rpc.rs` and `rpc/calls.rs`
once a read-error guard no test can reach was folded back into the
unguarded match it replaced; its only unhit function, `rpc::initialized`,
is `serve`'s, which the real-box protocol tests above drive. The ratchet
holds file sizes, duplication and the public API; `baselines origin/main`
still names only the ruled production `expect dead_code` at 2.
`quality/file-lines.txt` has the remeasured rows of `rpc.rs` (590),
`rpc/calls.rs` (554), `rpc/tests.rs` (1,586), `capability_broker.rs`
(1,852) and `capability_broker/handoff.rs` (854).

## The review's second return

The review of `931d424b` returned the implementation with M1–M4 and L1–L5.
The third visit amends that commit again, inside the row's `rpc.rs` and
`rpc/calls.rs`, the existing child `session/readiness.rs` (the operator's
addendum of 2026-10-11) and the protocol's unit suite; `session.rs` only
adds `timely` to its existing re-export line and stays at 799 lines.

| Finding | What changed |
| --- | --- |
| M1, duplicate catalog names accepted | `listed` keeps every listed name, granted or not, across all pages, and a name listed twice ends the session with the catalog cause, latched `Failure::Response` like a repeated cursor: design.md names repeated cursors and duplicate names together, and MB3 gives catalog failures one text. The count bound now reads the names kept, which equals the tools listed once no name repeats. |
| M2, a forwarded call written synchronously | Nothing the broker sends the server waits any more. `Server` queues whole frames in order (`queue`, `ask`) on its input, now non-blocking, and `flushed` writes as much as the server takes, once, before each read of its output (`heard`). The call path reads the harness on every turn while a large call is still being taken, so a cancellation lowers the call's deadline to two seconds from its arrival and is queued after the call's unwritten bytes. Startup is unchanged in effect: `exchanged` polls `heard` within the startup deadline, which now bounds the request's write too. |
| M3, delivery and closing bounds read after the call is taken | `answered` reads `Session::bound` before taking the active call, so delivery keeps the call's own deadline. `failed` gives the closing frame exactly `CLOSING` (one second) from now, never more, still cut short by the attempt's cancellation. |
| M4, a cancellation's failed write discarded | `cancel` only queues. The queue's next write, before the server's output is read again, ends the session with the protocol cause where the server's input has ended, so the first cause is never lost, and a response already waiting is never read as the call's outcome. |
| L1, null and positional shapes | An empty `Keyed` struct is flattened into each shape read from either side (`Frame`, `Fault`, `Info`, `Tool`, `Asked`, `Greeting`, `Cancelled`, `Called`). serde then reads them through `deserialize_map`, which refuses an array, while still refusing a duplicate key and, where the shape is closed, an unknown one; a scratch crate outside the tree confirmed all three refusals before the change. `Frame.method` and `Asked.id` read through `present`, so an explicit null is never absent. |
| L5, `timely` written twice | `timely` moved to `session/readiness.rs`, where `polled` now reads it, and `calls.rs` imports it through `session.rs`'s existing re-export. |
| L2–L4 | Carried, with the reasons below. |

To bind M3's delivery case, a test needs a call bound shorter than MB3's
thirty seconds on an answer. `served` now delegates to `served_within`,
which takes the call bound as data and is `pub(super)` for the tests; `served`
keeps the one ruled `expect(dead_code)`, and clippy reports nothing for the
delegate.

The protocol's unit suite is now 30 tests and 1,761 lines. Changed and new
tests on this visit, each passing on the final tree (`cargo test -p
brokkr-cli --lib broker::`, 77 passed):

| Test | Exact assertion |
| --- | --- |
| `a_call_the_server_does_not_take_keeps_neither_the_harness_nor_its_cancellation_waiting` (new) | A server holding its input after listing, a 256 KiB call cancelled at once, a 20 s attempt: `[lookup Failed]`, `Timeout`, nothing heard, in 2–5 s |
| `an_answer_or_a_last_frame_the_harness_leaves_unread_waits_no_longer_than_its_bound` (new) | A harness that reads nothing, its id 256 KiB, a 20 s attempt. With a 1 s call bound, an immediate answer: `[lookup Succeeded]`, `Timeout`, under 5 s. With MB3's bound, a mismatched answer: `[lookup Failed]`, `Protocol`, under 5 s |
| `a_cancellation_the_servers_ended_input_cannot_take_ends_the_session_with_its_first_cause` (new) | The server ends its input after the call, its output silent or answering 600 ms later; a cancellation at 300 ms: `[lookup Failed]`, `Protocol`, nothing heard, under 2 s, each case |
| `the_whole_catalog_is_paged_and_filtered_and_a_granted_tool_lacking_or_a_name_twice_ends_it` (renamed, changed) | Pages of unique names offer exactly `lookup` and `read`; a granted name on two pages and an ungranted one twice on a page are each `Catalog` |
| `the_catalogs_page_tool_and_cursor_bounds_hold_exactly` (changed) | The fixture names every tool once (`other-<n>`); the same bounds |
| `a_response_must_answer_the_one_request_outstanding_by_its_exact_typed_id` (changed) | Also a null method, server info by position, a ping by position before the answer, and tools by position: each `Failed(Protocol)` |
| `an_ungranted_call_is_recorded_refused_and_an_invalid_one_refused_unrecorded` (changed) | Also parameters by position (the invalid cause with its id), a null id and a frame by position (the invalid cause with a null id), none recorded or forwarded |
| `the_harness_is_answered_here_and_its_granted_call_forwarded_once_recorded` (changed) | Also initialize parameters by position: the invalid cause |
| `a_mismatched_or_late_response_ends_the_session_and_forwards_nothing` (changed) | Also an error object by position: `[lookup Failed]`, `Protocol` |
| `a_cancelled_call_is_passed_on_and_never_delivered` (changed) | A cancellation by position cancels nothing: the first call's answer is delivered; the second call is cancelled as before; frames show both calls and `requestId` 4; both `Succeeded` |
| `a_server_that_takes_no_call_ends_it_and_an_ended_harness_leaves_it_begun` (changed) | Same assertions; its servers and large call are now the shared `listing_then` and `large` helpers |

Each mutation below compiled, failed its test as named, and was reverted.
The shape mutations ran in four rounds, each round mutating at most one shape
that a given test reads, so every failure is attributable; the restored tree
passed all 30 protocol tests again.

| Mutation | Failing test and assertion |
| --- | --- |
| Duplicate-name check dropped | catalog test: `Ok([lookup, read, lookup])` for `Err(Catalog)` |
| Forwarding written synchronously until the call's deadline | held-call test: after 20.01 s the harness heard "MCP server call timed out", the cancellation never read |
| Delivery bound read after the call is taken | unread test, first case: took 20.005 s |
| Closing frame bound by the remaining deadline, at least one second | unread test, second case: took 20.007 s |
| A failed write to the server dropped with its queue | ended-input test: silent case `Timeout` for `Protocol`; answering case, run first, `[lookup Succeeded]` for `[lookup Failed]` |
| `Frame.method` reads null as absent | response test, a null method: `Ok` for `Failed(Protocol)` |
| `Asked.id` reads null as absent | ungranted test: the null-id call was answered with nothing, so the harness's last expected answer never came |
| `Fault` read by position | mismatched test, an error by position: the error relayed in place of "MCP server protocol is invalid" |
| `Frame` read by position | response test, a ping by position: `Ok` for `Failed(Protocol)`. A first case, a positional response, did not bind (its null method refuses either way), so it was replaced |
| `Called` read by position | ungranted test: the answers differ |
| `Info` read by position | response test, server info by position: `Ok` for `Failed(Protocol)` |
| `Asked` read by position | ungranted test: the answers differ |
| `Tool` read by position | response test, tools by position: `Ok([["lookup"], ["read"]])` |
| `Greeting` read by position | harness test: the answers differ |
| `Cancelled` read by position | cancellation test: `None` for the first call's answer |

The third visit's gates ran on the final tree, with only brokkr-cli
changed. Formatting and clippy (workspace, all targets, all features, locked)
were clean, and so was `cargo +1.88 check --workspace --all-targets
--all-features`. brokkr-cli's library passed 720 with 1 ignored; its `it`
binary outside `capability_broker::` failed once on
`agent_readouts::a_run_off_a_terminal_prints_what_it_always_has`, a run
summary this unit does not touch, then passed 505 with 2 ignored on the rerun
and alone; `capability_broker::` passed 74 in 207 s, its protocol child
alone 5 in 35 s with `--nocapture`, printing no skip notice;
`driver_conformance` passed 27, `transcript_surfaces` 13 and each heap binary 1. No
real-box test was added on this visit. `recipes/self` and `recipes/verify`
compiled. `openspec validate --all --strict` passed 19 of 19, and `typos --hidden`
and `git diff --check` were clean. `cargo crap --path crates/brokkr-cli/src/broker`
puts every function at 15 or below (`Server::heard` 15, `served_within` 15,
`decoded` 14, `listed` 9, `Session::answered` 8, `Server::flushed` 6). A
branch-coverage diagnostic (`cargo llvm-cov --branch` on nightly-2026-09-05
over the broker unit tests) took both directions of every branch in
`rpc.rs` and `rpc/calls.rs`; its only unhit lines are `rpc::initialized`'s,
which the real-box protocol tests drive. The ratchet holds file sizes,
duplication and the public API; `baselines origin/main` still names only the
ruled production `expect dead_code` at 2. `quality/file-lines.txt` has the
remeasured rows of `rpc.rs` (619), `rpc/calls.rs` (582), `rpc/tests.rs`
(1,761) and `session/readiness.rs` (501).

## The M1 repair (2026-10-12)

The council of run `0065-slice-two-unit-u6e-see-the--d6424e8f` (head
`5c22affa`) left one medium, M1: writes to the harness waited synchronously
within their bound, so during a silent active call a ping whose large valid
string id fills the harness's unread output pipe kept the serving loop from
reading a following `notifications/cancelled` until the call's deadline (up
to 120 s), breaking MB3's two-second cancellation settlement. The operator
ruled on 2026-10-12 to repair it in U6e. The earlier visits' observation that
recorded this limitation is withdrawn: it no longer holds.

| Where | What changed |
| --- | --- |
| `rpc/calls.rs` | What the harness is told is queued in an `Outbox`: whole frames in order, each with the deadline that applied when it was told (`Session::bound`, or `CLOSING` for the last frame of a failed call), and how much of the first is taken. `Outbox::flushed` writes as much as the harness takes, never waiting, never beginning a frame before the one ahead of it is whole; a write error makes the output gone, and a frame still waiting past its deadline makes it stalled, which ends the session with the timeout cause as before. The serving loop (`Session::serving`, split out of `served_within`) flushes on every turn and after every frame queued, and keeps reading the harness while fewer than 64 frames and fewer than MB3's 8 MiB wait (`Outbox::room`); past either bound it reads nothing more until the harness takes some, so no queue is unbounded (the review's third return below found a cancellation left unread there waiting for the call's deadline, and repaired it). When the session ends, `Session::drained` waits for what is still queued: by each frame's own deadline where the harness ended its input, and within `CLOSING` (1 s) and no longer where a cause ended it. Nothing is written once the attempt is cancelled, as before. The harness's output is set non-blocking once, where `written` used to set it on every write. |
| `session/readiness.rs` (existing child) | `written` is removed and `sent` is main's again, its only consumer; `session.rs` drops `written` from its existing re-export line and stays at 799 lines. |
| `rpc.rs` | The module comment no longer says an explicit null is never absent: members read through `present` (a frame's id, method, result and error, among others) refuse a null, while a frame's params, an error's data and a page's next cursor read a null as absent. |

The 1 MiB request and 8 MiB response bounds, the absolute deadlines, the
durable Started before forwarding and Terminal before delivery, and the rule
that a cancelled call's response is never delivered are unchanged. No new
file, public item or suppression.

Two tests are new in `rpc/tests.rs`, and its `Disk` fixture now keeps when
each record became durable (`Ran::durable`); `passed(id)` names the
cancellation the server reads, shared with the existing cancellation test.
The suite is 32 tests and 1,880 lines.

| Test | Exact assertion |
| --- | --- |
| `a_cancellation_behind_an_answer_the_harness_leaves_untaken_settles_in_two_seconds` (new) | A silent active call, then a ping whose id is 512 KiB that the harness never reads, then its cancellation, with a 10 s call bound and a 30 s attempt: the Terminal (the third record) is durable between 2 s and 3 s after the cancellation began, the 1 s margin being the stated scheduling tolerance for a loaded two-CPU host; the session ends under 4 s after it (two seconds, `CLOSING`'s one, and that tolerance); the server read exactly the one forwarded call and `notifications/cancelled` for id 3, no retry; `[lookup Failed]` closed `Failed(Timeout)`; the harness's pipe holds a non-empty strict prefix of the ping's answer and nothing after it |
| `what_waits_for_the_harness_is_bounded_and_taken_whole_in_order` (new) | A ping answer larger than the pipe, then a small one: both read whole, in order, and the session ends `Ok`. Behind a blocked answer with a 1 s call bound: 63 frames waiting let the cancellation reach the server, 64 do not; eight answers of a near-1 MiB ping (under 8 MiB) let it, nine do not; each `[lookup Failed]`, `Failed(Timeout)` (its second half is rebound in the third return below) |

Each removal below compiled, failed as named, and was restored; the restored
tree passed every broker unit test again.

| Removal | Failing test and assertion |
| --- | --- |
| The old write-then-wait ordering: `Session::write` also waits for the queue to drain by the frame's deadline | settlement test: "settled in 10.006504776s", the call's deadline, not the cancellation's |
| `Outbox::room`'s frame bound raised by 1,000 | bounds test, "64 frames": the server read the cancellation |
| `Outbox::room`'s byte bound doubled | bounds test, "9 answers": the server read the cancellation |
| `Outbox::flushed` pops the last frame, not the first, once one is taken | bounds test, order case: the big answer again where `b`'s was expected |
| `Session::flush` writes after the attempt is cancelled (as first written) | `the_attempts_deadline_and_cancellation_end_an_idle_a_waiting_or_a_flooded_session`, cancelled case: the harness heard "MCP server call timed out" for `None` |
| The harness's output not set non-blocking (as first written) | the settlement test never finished; it was killed after 40 s |

The gates ran on the final tree, with only brokkr-cli changed. Formatting and
clippy (workspace, all targets, all features, locked) were clean, and so was
`cargo +1.88 check --workspace --all-targets --all-features`. `cargo test -p
brokkr-cli --lib broker::` passed 79, twice; the whole library passed 722 with
1 ignored; the `it` binary passed 513 with 2 ignored outside
`capability_broker::`, and `capability_broker::` passed 74;
`driver_conformance` passed 27, `transcript_surfaces` 13 and each heap
binary 1. `recipes/self` compiled. `openspec validate --all --strict` passed
19 of 19, and `typos --hidden` and `git diff --check` were clean. `cargo crap
--path crates/brokkr-cli/src` puts every changed or new function at 15 or
below (`Server::heard` 15, `decoded` 14, `Session::serving` 14,
`Outbox::flushed` 8, `served_within` 5). The branch-coverage diagnostic
(`cargo llvm-cov --branch` on nightly-2026-09-05 over `broker::`) hit all 316
lines and 28 branches of `rpc/calls.rs` and all 12 branches of `rpc.rs`,
whose only unhit lines are still `rpc::initialized`'s; a first run found
`Session::write`'s guard never false, and the guard was removed, since
`Session::flush` already writes nothing to a closed or stalled output. The
ratchet holds file sizes, duplication and the public API; `baselines
origin/main` names only the ruled production `expect dead_code` at 2.
`quality/file-lines.txt` has the remeasured rows of `rpc.rs` (622),
`rpc/calls.rs` (679), `rpc/tests.rs` (1,880), `session/readiness.rs` (491)
and `capability_broker.rs` (1,861).

## The review's third return (2026-10-12)

The review of `5bff10cb` (run `0065-slice-two-unit-u6e-see-the--853dc19d`)
kept the M1 repair's queue but returned M1–M3 and S1 against it, with no
specification defect. Each is answered in `rpc/calls.rs`; the only other
production change is a comment.

| Finding | Answer |
| --- | --- |
| M1, a cancellation left unread behind a full queue waits for the call's deadline | Once 64 frames or 8 MiB wait for the harness, its next frame stays unread as before (bounded storage, frame order kept), but `Session::untaken` now keeps when that began, and `Session::due` gives the active call its own deadline or MB3's two seconds from then, whichever is first; `Session::heard` ends the call at `due`. A cancellation left unread behind the full queue therefore settles within two seconds of the queue filling, which is no earlier than the cancellation was sent unless it was already waiting in the pipe behind the frames that filled it, with the call failed and the timeout cause; the queue's bounds are unchanged. The harness is also read at the ledger's call bound now, where it used not to be, so the last call's cancellation is read too; a call made there ends the session with the ledger's limit through `Session::failed`, which records the active call's Terminal before telling the harness. |
| M2, an expired frame written before its expiry was checked | `Outbox::flushed` checks every waiting frame's deadline before writing anything: one past it makes the output stalled with nothing written; otherwise it writes as before and is open. |
| M3, a drain failure after the harness's end returned `Ok` | `Session::drained` now returns the timeout cause where what waits for the harness is not taken before its deadline, the attempt's deadline or cancellation, unless the harness closed its output. `served_within` returns it where the session had no cause of its own, and keeps the first cause where it had one. A call still active when the harness ended its input still stays begun. |
| S1, every repeated cancellation queued again | `Session::cancel` passes a cancellation on only while the active call is not yet cancelled; a repeat is nothing to cancel. The server's queue then holds at most the call, one cancellation and the broker's answers to the server's own requests, which spend its 1 MiB budget. |
| Info, `Ended::failure`'s comment | Corrected in `session/lifetime.rs`: the ledger latches its own limit, records nothing after an append that failed, and a record refused before its append leaves the closure to what is latched, which the call path's order never makes so. |

Tests in `rpc/tests.rs`, now 34 tests and 1,955 lines:

| Test | Exact assertion |
| --- | --- |
| `what_waits_for_the_harness_is_bounded_and_taken_whole_in_order` (changed) | Its four queue cases now run under MB3's 120 s call bound and a 30 s attempt. At 63 frames and at eight near-1 MiB answers the cancellation reaches the server (`[forwarded 3, cancelled 3]`) and the Terminal is durable between 2 s and 3 s after it was sent; at 64 frames and at nine answers it stays unread (`[forwarded 3]`) and the Terminal is durable under 3 s after it was sent. Every case ends under 4 s after the cancellation, `[lookup Failed]` closed `Failed(Timeout)`, no retry |
| `at_the_call_bound_every_accepted_call_is_kept_and_the_harness_still_read` (renamed, changed) | After 4,095 refused calls and a forwarded 4,096th, a ping is answered (4,097 frames, the ping's answer then the call's); with a further call after the ping, the harness hears the ping's answer and then "broker ledger exceeds the attempt limit" for `last`, and the ledger holds `lookup Failed`; both closed `Failed(Limit)` with all 4,096 calls kept |
| `a_cancelled_call_is_passed_on_and_never_delivered` (changed) | The never-settled case cancels twice: the server reads `[forwarded 3, cancelled 3]` exactly |
| `a_frame_past_its_deadline_is_never_begun_though_the_harness_would_take_it` (new) | An `Outbox` holding a frame in time and one past its deadline, flushed to a `Vec`: `Stalled`, nothing written, both frames kept; with both in time, `Open`, both written whole in order |
| `an_answer_left_untaken_once_the_harness_ends_its_input_ends_the_session_timed_out` (new) | A 512 KiB-id ping, the harness's input ended and its output unread, an 800 ms attempt: `Err(Failed(Timeout))`, no calls, closed `Failed(Timeout)`, under 3 s |

Each removal below compiled, failed as named, and was restored; the restored
tree passed every broker unit test again.

| Removal | Failing test and assertion |
| --- | --- |
| `Session::untaken` never records when the queue filled | bounds test: "64 frames: 29.980818816s", the attempt's deadline |
| `Outbox::flushed` checks expiry after writing (as before) | deadline test: `Open` where `Stalled` was expected |
| `served_within` turns a drain failure after the harness's end into `Ok` | end test: `Ok(())` closed `Failed(Refused(ServingIncomplete))` for `Err(Failed(Timeout))` closed `Failed(Timeout)` |
| `Session::cancel` ignores whether the call is cancelled | cancellation test: the server read `notifications/cancelled` for id 3 twice |
| The harness unread at the ledger's call bound (as before) | bound test: 4,096 frames heard where 4,097, the ping's answer first, were expected |
| `Outbox::room`'s frame bound raised by 1,000 (rerun) | bounds test, "64 frames": the server read the cancellation |
| `Outbox::room`'s byte bound doubled (rerun) | bounds test, "9 answers": the server read the cancellation |

All of this visit's gates ran on the final tree, with only brokkr-cli
changed. Formatting, clippy (workspace, all targets, all features, locked) and
`cargo +1.88 check --workspace --all-targets --all-features` were clean.
brokkr-cli's library passed 724 with 1 ignored (`broker::` alone, 81); the `it`
binary passed 513 with 2 ignored outside `capability_broker::`, and
`capability_broker::` passed 74. `recipes/self` and `recipes/verify`
compiled, `openspec validate --all --strict` passed 19 of 19, and `typos
--hidden` and `git diff --check` were clean. `cargo crap --path
crates/brokkr-cli/src/broker/rpc` puts every function at 15 or below
(`decoded` and `Session::serving` 14, `Session::call` 8, `Outbox::flushed` 7,
`Session::drained` 3, `Session::due` 2, `Session::untaken` 1). The
branch-coverage diagnostic (`cargo llvm-cov --branch` on nightly-2026-09-05
over `broker::rpc`) left no line or branch of `rpc/calls.rs` unhit, and only
`rpc::initialized`'s lines in `rpc.rs`. The ratchet holds file sizes,
duplication and the public API; `baselines origin/main` names only the ruled
production `expect dead_code` at 2. `quality/file-lines.txt` has the remeasured
rows of `rpc/calls.rs` (716), `rpc/tests.rs` (1,955) and `session/lifetime.rs`
(92); the review's note that the `rpc/calls.rs` row read 679 against 677 lines
is answered by that remeasurement.

To let the tests reach `Outbox` directly, `Outbox`, its `frames`,
`Outbox::flushed` and `Delivery` are `pub(super)`, visible within `rpc` alone.
The harness's output is still left non-blocking when the session ends (the
review's O_NONBLOCK note), which is U6f's to settle when `serve` hands it a
real harness.

## Findings carried

Low (ruling 5): SC4's tool vocabulary is written a second time in
`rpc/calls.rs::vocabulary`; its home, `capabilities::binding::representable`,
is private to brokkr-runtime, outside this row. Low (ruling 8): the private
enums implement `Display` and `Error` by hand, as the rest of brokkr-cli
does, because brokkr-cli has no `thiserror` dependency and adding one moves
`Cargo.toml` outside the row. Low (ruling 3): a tools/call's arguments and
result, and each listed tool's description, cross `rpc` as `serde_json::Value`
from one wire to the other without being read by key or kept beyond the
session's listing bytes. The broker's own reading of a response (its id,
whether it is a result or an error, the error object, and the tool result's
shape and `isError`) is typed at the edge since the second visit (M6); what
remains `Value` is the relayed data alone.

L3 of the last council (a tools/call result is read as `Answered` again
after `Frame` read it) stays recorded, not changed: the edge, `Server::heard`,
cannot know which method a response answers before `Session::answered`
matches its id against the active call's, and initialize and listing results
are not `Answered`'s shape, so the outcome is parsed where the response is
first known to be a call's. The council's info note on `rpc.rs`'s null
comment is corrected (above).

## Pending

Pending until their results exist: the exact coverage gate
(`scripts/coverage-exact.sh`) on CI; the macOS build and tests; the remote CI
jobs; the controller's two-CPU host run of the five real-box tests above and
of the changed `handoff` and `capability_broker` tests; CI's CPU budgets,
which compare each pull request against its base and have no local file; and
the Markdown link check (`lychee --offline --include-fragments`), which no
visit's seat was permitted to run, the third return's included (the review
reported it passing on `931d424b`). The evidence file holds no link, and
`tasks.md` changed only its two checkboxes. The M1 regression's timing was
measured on this seat's host; its two-CPU run is the controller's.
