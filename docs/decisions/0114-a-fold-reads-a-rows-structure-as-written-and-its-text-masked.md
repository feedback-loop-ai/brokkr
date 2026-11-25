# 0114 — A fold reads a stream row's structure as the harness wrote it, and its text only masked

Status: accepted (operator, 2026-11-23, "go")
Date: 2026-11-21, revised 2026-11-22 (round two, operator ruling "1242d
option 1"; round three, "go with recommended everywhere"; finished by
hand, "go with all recommended")

Built: unbuilt (#1242)

This decision is a document only. It mints no contract version and adds no
code, test, guide or recipe. It builds on 0012's layer 5 (known-plaintext
masking on captured streams) and on 0071's rulings 2, 3, 5 and 9, and takes
#1213's rule as given: the masker covers the union of a value's overlapping
occurrences. Round two brought in the driver's own checkpoints. Round
three masks every projection of a name over the whole name before it is
split, and states each remaining finding as a named acceptance test of
its owning slice.

## Context

`secret::mask_json` (`crates/brokkr-protocol/src/secret.rs:759`) rewrites
every occurrence of a bound value, and of each of its shared encodings, by
substring, in every string value and in every object key
(`secret.rs:773`). Numbers are left as numbers. A binding is at least
`MIN_VALUE_BYTES` (4, `secret.rs:33`) long.

Each built-in driver with a fold calls it on the whole stream row before
its fold reads the row (Exec has no fold and masks bytes, below):

- Claude and LaneTally: `invoke_stream_json`, `adapters.rs:2494`.
- Qoder: `QoderFold::read`, `adapters/qoder.rs:404`, after its own rules.
- Codex: `invoke_codex`, `adapters.rs:3697`.
- dsh: the transcript tail in `DshFold::drain`, `adapters.rs:2350`, and
  the stream line in `fold_dsh_line`, `adapters.rs:4957`.
- What the folds kept is masked again in `run_seat_with`: every
  `session_meta` value at `adapters.rs:6589`, the refusal at 6591
  (`masked_refusal`, 6353), and the seat's result file at 6768.

So the fold reads a row the masker has already rewritten. A binding equal
to `type`, `result`, `error` or `assistant`, or a binding inside one such
as `limit` inside `rate_limit`, turns the key or the word into
`[secret:NAME]`, wholly or in part. The fold then misses the lookup or
the match: a turn is not counted, a refusal to start is not seen, or a
rate limit reads as another class. #1178's Qoder review met it (operator
ruling "1" on #1178 round eight, which filed #1242).

The driver's own checkpoints meet it too, and there it fails open. Every
row a fold emits passes `seat_row` (`adapters/probe.rs:91`, mask at 98)
inside `run_seat_with`'s emit closure (`adapters.rs:6508`), and only then
is its `step` read by `begins_work` (`adapters.rs:6509-6512`, 6271). A
binding inside `harness-started` (`star`, `harn`) or `transcript` (`scri`)
turns the pre-session row into an unknown step, which `begins_work` reads
as work. On a cold, non-probe refusal, `LaunchHold::finish` (1135, row
built at 1154-1155) emits that row: for Codex at `adapters.rs:5491`, and
on the same shared path for Claude and LaneTally at 5480. The seat then
sends `accepted` and the buffered rows (6513-6521), `worked =
began_work || delivered` is true (6615-6616), and
`lost::refusal_to_start` drops the classified refusal (6617,
`adapters/lost.rs:93`). The attempt reads as a mid-session failure under
0016, not a refusal to start under 0053, and its class is lost. The hold
itself reads the same row a second time, raw, in `LaunchHold::observe`
(1072-1087): its own `began_work`, and the root it confirms from
`/transcript/locator`, a value the fold copied from a masked harness row.

### What each fold reads as structure (D1)

Paths are under `crates/brokkr-protocol/src/` at `b4af73e1`: main
`33cd15db` (#1199, #1213, #1169) and #1178's Qoder adapter (`6498d32a`).
"Masked" means the read happens after `mask_json`; "raw" means before it
or with no mask at all.

#### Claude and LaneTally

LaneTally runs the same fold. The `Claude | Lanetally` arm calls
`invoke_stream_json` (`adapters.rs:5472-5480`), the probe kind is Claude
(`probe.rs:73`), and only `capture: "lanetally"` is added to the
session-finished checkpoint after the fold (`adapters.rs:6695-6697`).

| Read | Keys looked up | Words matched | Where |
| --- | --- | --- | --- |
| refusal to start (turn 0 only, `adapters.rs:1455`) | `type`, `apiErrorStatus`, `/rate_limit_info/resetsAt`, `message`, `error`, `isApiErrorMessage`, `/message/content/0/text`, `is_error`, `result` | `type` ∈ `rate_limit_event`, `assistant`, `result`; `isApiErrorMessage == true`; `is_error == true`; any string `error`; `apiErrorStatus == 429` | `adapters/refusal.rs:98-163` |
| refusal class | the token, after `bounded_wire_line` | token `== "rate_limit"` | `adapters/refusal.rs:59-71` |
| probe answer | `type` | `assistant`, and no refusal | `adapters/probe.rs:73-76` |
| session start | `type`, `subtype`, `session_id` | `system` with `init` | `adapters.rs:1461-1474` |
| turn | `/message/id`, `/message/usage`, `/message/content`; per block `type`, `/input/file_path` | `assistant`; block `tool_use`; a repeated message id | `adapters.rs:1477-1558`, `adapters/claude_fold.rs:68-72` |
| tool call | block `/id`, `/name` | the `mcp__` prefix and the first `__` split into `Tool::Mcp`, `McpUnidentified`, `Named` or `Missing` | `adapters/capability_calls.rs:78-84, 169-184` |
| result | `session_id`, `usage`, `/num_turns`, `/usage/output_tokens_details/thinking_tokens`, `total_cost_usd` | `result` | `adapters.rs:1560-1565, 1574-1589`, `adapters/claude_fold.rs:78-112` |
| model, effort | `/model`, `/message/model`, `/usage/model`, `/data/model`, `/data/usage/model`, `/data/message/source/model`, `/message/source/model`; `/effort`, `/turn_context/effort` | `model_token` and `effort_token`'s alphabets; a value outside one is `not reported` (`MODEL_NOT_REPORTED`, `EFFORT_NOT_REPORTED`) | `adapters.rs:538-567, 580-615`; sentinels at 1511, 1519, 1864, 1868, 1972-1977, 6656-6689 |
| usage | `input_tokens`, `cache_read_input_tokens`, `output_tokens`, `cache_creation_input_tokens` | — | `adapters.rs:687-708` |

Read as structure and quoted as text: `error` is the discriminator
(`refusal.rs:121`, 138) and the refusal's leading token (`refusal.rs:154-163`);
the excerpt quotes `result`, `/message/content/0/text`, or `message` or
`error`. The model, the effort, `session_id` (the transcript locator, and
the id `LaunchHold::confirm` compares with the offered root,
`adapters.rs:1078-1087`, 1100-1104), `/input/file_path` (the checkpoint
`target`) and the tool `name` are quoted as text.

#### Qoder

| Read | Keys looked up | Words matched | Where |
| --- | --- | --- | --- |
| own rules, raw | `type`, `subtype`, `model`; `session_id` removed | `system` with `init`; a second init; `model` equal to the pin; `assistant`, `user` or `result` before the init confirms | `adapters/qoder.rs:389-395, 413-430` |
| message lookup and model replacement, raw | `type`, `message`; `/message/model` overwritten with the pin | `assistant` | `adapters/qoder.rs:393, 398-401` |
| unmeasured line, raw | — | a line that is not a JSON object refuses in fixed words (`stream_unmeasured`) | `adapters/qoder.rs:389-391, 502-509` |
| stderr, raw | — | `is not available right now; using`, `Not logged in`, `unexpected critical error` | `adapters/qoder.rs:82-87, 441-476, 554-575` |
| shared fold, masked | everything in the Claude table | everything in the Claude table | `adapters/qoder.rs:402-406` |
| worked, masked | the shared fold's turn count | — | `adapters/qoder.rs:370-372`, read at 694 |

Qoder quotes the served `model` only when it is a plain id
(`qoder.rs:155-162`, 484-498), masks every word it ends with
(`qoder.rs:277-291`), and names a classified refusal by its class alone
(`qoder.rs:538-547`). Two cases this rule fixes, both found at #1178's
landing:

- **A bound `assistant` erases a turn.** `worked()` counts the masked
  fold's turns. A binding inside `assistant` (`assi`, `tant`) hides the
  row from the shared fold, so no turn is counted, and a late stop with no
  delivered result reads as a refusal to start (`qoder.rs:694-697`).
  Accepted for #1178; #1242 owns it.
- **A row's own model wins over the confirmed pin (#1256, item 1).** Only
  `/message/model` on an assistant row is overwritten (`qoder.rs:398-401`).
  The shared fold's `model_in_json` tries `/model` first
  (`adapters.rs:556-557`), and `fold_result_meta` reads a `result` row's
  model (`adapters.rs:1580-1581`), so a top-level or result-row model
  replaces the pin init confirmed in `session_meta` and the seat-turn row.

#### Codex

| Read | Keys looked up | Words matched | Where |
| --- | --- | --- | --- |
| refusal to start (turn 0 only, `adapters.rs:1806`) | `type`; `status`, `message`, `codex_error_info`, each on the row or under `error` | `type` ∈ `error`, `turn.failed`; `status == 429`; `codex_error_info` ∈ `usage_limit_exceeded`, `rate_limit_exceeded`; token `== "rate_limit"` | `adapters/refusal.rs:59-71, 169-211` |
| reset in prose | `message` | the substring `try again at `, then month names, `st`/`nd`/`rd`/`th` and `AM`/`PM` | `adapters/refusal.rs:228-230, 391-466` |
| probe answer | `type`, `/item/type` | `item.completed` with `agent_message` | `adapters/probe.rs:77-80`, `adapters/capability_calls.rs:162-164` |
| thread | `type`, `thread_id` | `thread.started`; `plain_thread_id` | `adapters.rs:1812-1829, 1640-1651, 2881-2886` |
| rollout lookup by id | the announced `thread_id` against each rollout's file name | a whole-token match | `adapters/refusal/codex_thread.rs:206-219, 271-282` |
| turn | `type`, `usage` and `input_tokens`, `cached_input_tokens`, `output_tokens`, `cache_write_input_tokens`, `reasoning_output_tokens` | `turn.started`, `item.started`, `item.completed`, `turn.completed` | `adapters.rs:1831-1897` |
| tool call | `/item/type`, `/item/server`, `/item/tool`, `/item/id` | `mcp_tool_call` | `adapters/capability_calls.rs:32, 88-106` |
| result | model and effort pointers, `session_id`, `num_turns`, `total_cost_usd` | `result` | `adapters.rs:1574-1589, 1900-1907` |
| model, stderr fallback, raw | the `model:` field of the last stderr line whose token parses, scanning backward over every line | `model_token`, then equal to the validated pin; used only where no stream or rollout model stands (`or_insert`, then `contains_key`) | `adapters.rs:668-674, 3754-3768` |

The rollout echo is read raw and never masked at read; what it yields
reaches `session_meta` and the refusal, both masked in `run_seat_with`:

| Rollout read | Keys | Words | Where |
| --- | --- | --- | --- |
| row kind | `type`, `payload.type`, `payload.role`, `/payload/item/type` | `session_meta`, `world_state`, `turn_context`, `error`, `response_item`, `event_msg`; `message` with `developer` or `user`; `task_started`, `token_count`, `error`, `task_complete`, `item_started`, `item_completed`; `UserMessage` | `codex_thread.rs:41-75, 92-98` |
| completion | `payload.error`, `payload.last_agent_message` | non-null | `codex_thread.rs:79-90` |
| windows | `/payload/rate_limits`, `primary`, `secondary`, `used_percent`, `resets_at` | `used_percent >= 100` | `codex_thread.rs:136-142`, `refusal.rs:235-256` |
| boundary | `timestamp`, parsed to nanoseconds | at or after the invocation's start | `codex_thread.rs:154-162` |
| error | `type`, `payload`, `payload.type`, `payload.error`; the row or its nested payload handed back whole | `error`, `event_msg`, `task_complete` | `codex_thread.rs:164-178` |
| header | `type`, `payload.cwd`, `payload.timestamp`, `payload.id`; the id in the file name | `session_meta`; the id is the record's own: equal to the announced thread, or with none announced to the file name's id | `codex_thread.rs:289-320, 329-344` |
| correlation | the header's `cwd` and start | announced: `cwd == workdir` and start `>= since` (`adapters.rs:1755-1758`); unannounced: the same on every candidate, and exactly one survives (`codex_thread.rs:183-200`) | as shown |
| model, effort | `/payload/model`, `/payload/thread_settings/model`, `/payload/effort`, `/payload/thread_settings/reasoning_effort`; the raw substrings `turn_context`, `thread_settings` | — | `adapters.rs:642-661, 1681-1728` |

Read as structure and quoted as text: `message` is the excerpt and the
reset's source; `/item/type` is matched and quoted as the shown `tool`
(`capability_calls.rs:124-127`, 136); `thread_id` decides which rollout
is read and is the transcript locator and the resume handle.

#### dsh

| Read | Keys looked up | Words matched | Where |
| --- | --- | --- | --- |
| row is current, masked | `seq` | — | `adapters.rs:1997-2004` |
| transcript fold, masked | `type`; `/data/usage` and `inputTokens`, `outputTokens`, `cacheReadTokens`, `reasoningTokens`; model pointers | `session`, `request/header`, `assistant/message`, `tool/call` | `adapters.rs:1952-1961, 2049-2146` |
| effort header, masked | `data.header`, `config.reasoningEffort`, `adapterDefaults.reasoningEffort` | `adapterDefaults.reasoningEffort == true` | `adapters/dsh_effort.rs:44-76` |
| refusal to start, masked | `/data/reason`, `kind`, `error`, `code`, `status`, `message` | `kind == "error"`; `status` 402 or 429, or `code == "QUOTA"`, emits the token `rate_limit`; any other `code` is the token | `adapters/refusal.rs:471-494` |
| probe answer, masked | `type` | `assistant/message` | `adapters/probe.rs:82` |
| tool call, masked | `/data/callId`, `/data/name` | the `mcp__` prefix | `adapters/capability_calls.rs:109-115` |
| stream line, masked | `type`, `subtype`, `session_id` | `system` with `init` | `adapters.rs:4920-4941, 4953-4958` |
| confirmation, masked | the stream `session_id` | equal to the offered id | `adapters.rs:5195-5207`, 5278 |
| transcript choice and store census, raw | `type`, `delegationDepth`, `id`, `seq` | `session`; `delegationDepth == 0` | `adapters.rs:2234-2253, 3944-3975, 4142-4188` |
| stderr, raw then masked | — | `dsh: reasoning:` and the `dsh: ` prefix | `adapters/dsh_stderr.rs:11, 40-47` |

Read as structure and quoted as text: `code` is matched against `QUOTA`,
and is the token only when it is not a rate limit (`refusal.rs:484-488`);
`/data/name` is split and quoted as `tool`; the stream `session_id` is
compared with the offered id and stored (`adapters.rs:4939`).

#### Exec

Exec has no fold. Its stdout, stderr and state are masked as raw bytes and
then decoded (`adapters/exec.rs:50-68`); it reads no harness row as
structure. Its only JSON reads are the engine's own start input.

#### The driver's own checkpoints

The driver writes these rows itself; each `step` is a literal of the
engine's.

| Step | Written | Pre-session | Masked by `seat_row` |
| --- | --- | --- | --- |
| `transcript` | `transcript.rs:135` | yes | yes |
| `harness-started` | `adapters.rs:1154-1155`, emitted by `finish` at 5480, 5491, 5538 and `dsh_launch.rs:91`, 153 | yes | yes |
| `exec-started` | `adapters/exec.rs:24` | no | yes |
| `seat-turn` | `adapters.rs:1504`, 2095, 2137 | no | yes |
| `turn-started`, `item-started`, `item-completed`, `turn-completed` | `adapters.rs:1833`, 1837, 1859 | no | yes |
| `<driver>-session-finished` | `adapters.rs:6679-6681`, after the fold | no | no: built from masked `session_meta`; its step is unmasked |

Where work is decided from one:

| Decision | Reads | Where | Today |
| --- | --- | --- | --- |
| a checkpoint proves work began: send `accepted`, flush the buffer, stream it | `step` after `seat_row` | `adapters.rs:6508-6529`, `begins_work` 6271 | masked: a bound pre-session word reads as work |
| no work and no delivery, so the refusal stands | `began_work` from the row above | `adapters.rs:6615-6617`, `lost.rs:87-94` | masked: the refusal is dropped |
| the hold's `began_work`, which gates Codex's one cold replacement | `step` of the row as the fold wrote it, read apart from the seat's | `adapters.rs:1072-1077`, 5511 | raw: correct, but a second reading of one fact |
| the launch confirms its root from the transcript locator | `step == "transcript"`, `/transcript/locator`, compared with the offered handle | `adapters.rs:1078-1087`, 1100-1104 | raw row; the locator came from a masked harness row, so a binding inside a handle recorded before the binding existed reads `Mismatch` |

Outside the driver, the engine and the views read the journaled,
masked checkpoint: the resume offer
(`crates/brokkr-runtime/src/engine/resume.rs:387-389`, `root_session`),
the token meter (`crates/brokkr-core/src/seat/meter.rs:128-135`,
`seat-turn`, `turn-completed`, `-session-finished`) and costs
(`crates/brokkr-core/src/seat/costs.rs:199-200`). "What stays out" names
them.

One engine decision is made from the masked checkpoint and is in scope:
a capability call's verdict. `Calls::consume`
(`crates/brokkr-runtime/src/engine/capability_calls.rs:243-266`) looks the
observation's `Tool::Named` name up in the seat's held and known tools,
after `seat_row`'s mask (`probe.rs:98`). A binding inside a native tool's
name (`search` inside `web_search`, or `Bash` itself) makes it neither, so
the call reads `Ok(None)`: an unheld call goes unrefused, and a held one
loses its `Stamp` (#1261). D3's invariant 3 sets the floor for it.

#### What leaves

No fold forwards a row whole. Each builds its checkpoints, transcript rows
and `session_meta` from what it read. `seat_row` masks every emitted row
again (`adapters/probe.rs:98`, called at `adapters.rs:6508` and
`probe.rs:201`), and the seat sends it directly (`send`, 6522, and
`accept_and_flush`, `probe.rs:277-299`): no `secret::Guard` reads a seat
checkpoint. The guard's one production boundary is the broker's ledger,
which masks each broker record and passes its line through the guard
before it is appended (`broker/ledger.rs:498, 616-631`).

## The choice (D2)

### (a) Structure from the raw row

The fold reads its control fields as the harness wrote them. Every
text-bearing value is masked when the fold reads it as text, and so before
anything leaves.

- A field the fold both matches and quotes is read twice, by two reads:
  matched raw, quoted masked. A classifier asks whether `error` is
  `rate_limit` of the raw row, and quotes `error` and `result` masked. The
  Codex reset parser reads `try again at ` in the raw `message` and the
  excerpt quotes it masked.
- Cost to every adapter: each fold's `event.get(…)` and `pointer(…)` reads
  move to the shared reads once. The adapter keeps no list.
- A mistake: a structural read done through the masked text read gives
  today's behaviour for that one field, so it can hide a turn or a refusal
  but leaks nothing. A text read done through a raw read cannot leak,
  because every raw read hands back a `bool`, a number, a variant of the
  fold's own enum or an opaque value, never the bytes.
- A new harness: it writes its fold against the shared reads, with
  nothing else to register.

### (b) A structure-preserving mask

The masker never rewrites a key, nor a value in the fold's closed
vocabulary. A value outside that vocabulary in a structural field is an
unmeasured shape, and the fold refuses it in fixed words, so it never
reaches a word.

- Cost to every adapter: each adapter declares its structural paths and
  their vocabulary as data the masker reads. That is a second home for
  every word the fold already matches (0071 ruling 5), and it must be kept
  in step with the fold by hand or by a test per word.
- A mistake: a word missing from the list is masked, and the fold misses
  it, as today, or the unmeasured-shape refusal fires on a stream that was
  fine. A word wrongly on the list stays unmasked in that field; it is the
  binding itself when a binding equals it, and it reaches a quote unless a
  later mask catches it.
- Substring reads have no place in it. The Codex reset is found inside
  free text, `message`, which must be masked, so a binding `again`
  still changes the retry-after.
- A field matched and quoted still needs a second, masked read for its
  quote, so (b) carries (a)'s two reads for those fields as well.
- A new harness: it writes its vocabulary list, its unmeasured-shape
  refusal, and the tests holding both to its fold.

### Recommended: (a), as one fold helper, and the driver's own step typed

(a) leaves one rule in one place; (b) leaves one list per adapter, beside
the fold that already holds the same words. The rule:

> A fold reads a harness row only through `adapters::row::Row`. A raw
> value may be compared, parsed to a type or held opaque, and never
> copied out; a value copied out is masked first, over the whole raw
> string, and only then split or shown. The driver decides work from the
> typed step it emitted, never from the masked row.

`Row` is a fold helper in `crates/brokkr-protocol/src/adapters/row.rs`. It
holds the parsed raw row privately and the seat's bindings. It does no
I/O (0071 ruling 1) and reads no secret value itself: its text read calls
`secret::mask_text`, so 0012's rule that every read of a value stays in
`secret.rs` holds, and #1213's union rule reaches every text read. Every
operation a fold needs lives in `Row`, so the rule stays in one place; no
fold gets a generic parse with a caller's closure, because such a closure
could return the bytes. Its reads:

| Operation | Reads raw | Hands back | Leaves only masked |
| --- | --- | --- | --- |
| `kind(path, table)` | the string at `path` | the fold's own enum variant, or `None` (0071 rulings 2 and 3) | nothing |
| `is(path, word)`, `contains(path, token)`, `equals(path, &str)` | the string at `path` | a `bool` against one of the fold's literals, or against a value the engine holds (the rollout header's `cwd` against `workdir`) | nothing |
| `flag(path)`, `count(path)`, `number(path)` | a boolean, an integer, a float | the value; `mask_json` never rewrote one | nothing |
| `since(path, nanos)` | a rollout `timestamp`, parsed by `parse_nanos` (`refusal.rs:305`) | `Some(bool)`, at or after `nanos`, or `None` when untimed; the time itself never leaves | nothing |
| `token(path, table)` | the string at `path`, put through `bounded_wire_line(…, REFUSAL_TOKEN_LIMIT)` first | the class variant (`rate_limit` → `RateLimit`) | the token's words, through `text` |
| `tool(path)` | a Claude or dsh tool name; Codex's `/item/server` and `/item/tool` | the `Tool` variant (below) | the components, cut from the whole name masked |
| `reset(path)` | Codex's `message`, on the row or under `error` (`machine_field`), parsed by `stated_reset` | a `LocalTime`, or `None` (below) | the excerpt, through `text` |
| `pin(path, &ModelPin)` | Qoder's init `model` | `Pinned`, `Other { plain: bool }` or `Absent` | the served id, through `text`, quoted only when `plain` |
| `ident(path)` | an identifier: Codex's `thread_id`, Claude's and dsh's `session_id`, a rollout header's `payload.id` | an opaque `Ident` (below) | the locator and root, through `text` |
| `text(path)` | the string at `path` | the string, masked, as a `String`: the only read that hands out a row's bytes | — |
| `present(path)`, `has(path)` | the value at `path` | a `bool`: there and not null (`complete_row`'s `error` and `last_agent_message`), or the key there at all, null included (`thread_error`'s `error`) | nothing |
| `at(path)`, `items(path)` | any value, an array | the nested value of any type, null included (`thread_error`'s row, payload or `error`), or each member, as a `Row` under the same rule | — |

- **The token's normalization stays, and runs in `Row`.** Today the class
  compares the token after `bounded_wire_line` strips controls, collapses
  whitespace and clamps it to 80 characters (`refusal.rs:59-71`). `token`
  does the same on the raw string before the table lookup, so a class
  reads as it does now. The words keep today's order: `text` masks, the
  refusal builder bounds the words (`refusal.rs:59-66`, 77), and
  `masked_refusal` masks them last (`adapters.rs:6368`). The class then
  comes from `token`, never from the words, so `refusal_reason_with` takes
  the class as an argument.
- **The reset is typed in `Row`, and a bound date gives none.**
  `stated_reset` and its word parsers (`refusal.rs:391-466`) take the raw
  `&str` inside `Row` and hand back `LocalTime`, which holds only numbers.
  A time is not a match against a literal: its retry-after, with the
  observation time, gives the date back (`refusal.rs:224-230`, 259-262).
  So `reset` asks `secret.rs` for the binding's mask intervals over the
  whole raw `message`, by the union rule, and hands back `None` when any
  interval intersects the span the parser consumed after `try again at `,
  one that begins before the date and ends inside it included. A binding
  only in the surrounding prose (`usage`) leaves the reset as an unbound
  run's. The exhausted window's `used_percent` and `resets_at` are
  `number` and `count` reads, never masked.
- **A name is masked whole, then cut.** `tool` decides the variant on the
  raw name, at `mcp__` and the first `__` after it, and asks `secret.rs`
  for the mask intervals over the whole raw name. When none intersects
  the prefix or that separator, `server` and `tool` are the masked whole
  name's two sides of it; when one does, the call is `McpUnidentified`
  with the whole masked name, never a native tool. No component is
  masked alone, so a binding across the separator leaves no readable
  piece in the serialized `Observation` or the shown `tool`, both built
  from those masked parts. The cut is made at the raw separator's offset,
  mapped into the masked name, and never by splitting the masked string
  again, because a binding's name may itself hold `__` (`secret.rs:137-143`).
  A model id and an identifier are masked whole the same way, and
  Codex's `server` and `tool` are each a whole field.
- **A model and an effort are validated after the mask.** `text` reads
  them, and `model_token` and `effort_token` then judge the masked
  string, as today. A binding inside one leaves brackets neither alphabet
  admits, so the seat-turn row reads `not reported`; `session_meta`
  keeps no value or the last valid one (`adapters.rs:1483-1490`), and
  the session-finished checkpoint reads that or `not reported`, never a
  bracketed display. Codex's stderr fallback is not a row and stays as
  it is: it hands back only a value equal to the pin. Qoder's served
  model is the pin's (below), so `pin`'s `plain` governs only its quote.
- **Qoder's pin is compared in `Row`.** `pin` compares the raw init
  `model` with the `ModelPin` byte for byte, and judges `plain`
  (`qoder.rs:155-162`) on the raw id, so a plain id with a binding in it
  reads `Other { plain: true }` and its quote is the masked id. The fold
  then takes its served model from the confirmed `ModelPin` rather than
  from any later row's `model`, so Qoder no longer rewrites
  `/message/model` before the shared fold.
- **An identifier is opaque.** `Ident` keeps the raw bytes and has no
  `Display`, `Debug` or `Serialize`. It has three operations: equality
  with another `Ident`, `is_offered(&str)` against the handle the engine
  offered (dsh's confirmation, `adapters.rs:5199`, and
  `LaunchHold::confirm`'s compare, 1100-1104), and `names_rollout(&str)`,
  the whole-token file-name match `find_codex_thread` walks with
  (`codex_thread.rs:271-282`), with `plain_thread_id`'s shape check on the
  raw bytes. Both rollout correlations keep their predicates: announced,
  the header's `Ident` equals the announced one; unannounced, it equals
  the id the file name carries (`names_own_record`, `codex_thread.rs:307-320`);
  and in both, `equals` on `cwd` and `since` on the start
  (`adapters.rs:1755-1758`, `codex_thread.rs:192-195`). The locator and the
  root's id leave through `text`. A masked
  id fails `recordable_session_id` (`adapters.rs:888-893`) and
  `plain_thread_id`, so no root is recorded and no resume is offered from
  it, as today. The Claude message id is an `Ident` too.
- **0114a captures each `Ident` before the family masks.** Until a
  family's slice removes its row mask, each mask site (`adapters.rs:2494`,
  3697, 4957, `qoder.rs:404`) first wraps the raw row in `Row`, reads its
  one identifier (`session_id` on an init or on a `result`, which
  `fold_result_meta` may make the only locator, `adapters.rs:1574-1588`;
  `thread_id` on `thread.started`) by `ident`, and hands the `Ident` to the fold beside
  the masked row. The fold puts it on the transcript `Emitted`,
  `LaunchHold::confirm` takes an `&Ident`, and `find_codex_thread` walks
  by `names_rollout` (`adapters.rs:1666`). dsh's callers move with it:
  `DshRootWatch::named` takes the `Ident` and compares the offer by
  `is_offered`, and the store path that confirms the offered handle
  itself (`adapters.rs:5278`) builds its `Ident` from the engine's
  handle, which is not row text. The masked id stays only the display.

The raw `Value` never leaves `Row`, so a raw value cannot reach a quote
without a type error. Qoder's raw-then-mask split folds into the shared
reads; its own rules keep their words. The dsh raw readers of the
transcript header and the store already read raw; they move to `Row` so
that the one rule holds of every row read.

**The driver's own step.** The producer owns it. Every writer of a
driver checkpoint (the transcript writer, `LaunchHold::publish`, the
folds, `exec-started`) hands the emit path an `Emitted { step: Step, row:
Value }`, where `Step` is a closed enum (`Transcript`, `HarnessStarted`,
and the work steps above), and the row's `step` string is written from
`Step` alone, so the word has one home. A transcript checkpoint also
carries its locator as an `Ident`. Both consumers read that one value:
`LaunchHold::observe` takes its `began_work` from `Step` and confirms its
root from the `Ident` by `is_offered`, never from `/transcript/locator`;
the emit closure in `run_seat_with` takes `began_work` from the same
`Step`, and only then masks the row through `seat_row` for the wire.
`begins_work` takes a `Step` and matches it exhaustively (0071 ruling 2).
The buffered rows keep their `Step` beside the masked row, so the flush,
`worked` and `lost::refusal_to_start` read the typed identity alone.

## The invariants, whichever is chosen (D3)

1. **No row text leaves unmasked.** No byte of a row's text reaches a
   refusal classifier's words, `session_meta`, a transcript row, an emit
   or a word without passing the seat's bindings. Under (a) the only read
   that hands out text is `text`, which masks.
2. **The backstops stay where they are.** `seat_row`, the `session_meta`
   and refusal masks in `run_seat_with` stay in their order and are the
   last mask on a seat checkpoint, which the seat sends directly. The
   ledger's mask and `secret::Guard` (`broker/ledger.rs:616-631`) stay
   the final boundary for broker records. Each becomes a second mask of
   text already masked.
3. **A bound protocol word changes no decision.** A binding equal to any
   word in D1's tables, or inside one, leaves the semantic decisions as an
   unbound run's: the turns, the `Step` sequence, the refusal or none, its
   class and retry-after, and acceptance. A masked wire field (a quoted
   word, a shown tool, a usage key) is its exact masked form, and absent
   or retained metadata is as D4 measures it per adapter. The journal's
   readers are outside this promise (What stays out). A capability call
   is the one decision a binding may change, and only toward refusal: a
   `Tool::Named` name the mask rewrote is refused as `Unattributable`
   before the held and known lookups (#1261). So an unheld call is never
   passed, and a held one is refused rather than stamped as another.
4. **Work is decided before the wire form is masked.** Whether an
   attempt began work, and so whether a refusal to start stands, and the
   root a launch confirms, are read from the `Step` and `Ident` the
   producer emitted, and never from its masked row.
5. **No value computed from bound bytes leaves.** A raw read hands back a
   match against a literal or an engine-held value, or a number the mask
   never rewrote. A parse that computes a value from the row's text (the
   reset) hands back none when a binding lies in what it consumed.

## The test plan (D4)

Each adapter's tests share one builder that runs its fold over rows with
and without bindings and returns the turns, the emitted rows and `Step`s,
`session_meta`, and the refusal or none with its class and retry-after.
The unbound run is first asserted against literal values, so no equality
with it passes empty. The bindings: every key and word of at least
`MIN_VALUE_BYTES` in the adapter's D1 table, and a four-byte slice of each
longer one, bound alone over its measured shapes (a turn with a tool call,
a result, each refusal to start its classifier names), `id` and `seq`
named as skipped; and a value in free text (assistant text, a tool
argument, `result`, a refusal's `message`). Each case asserts apart:

- **semantic:** turns, `Step`s, the refusal's variant, class and
  retry-after, and acceptance, equal to the unbound run's, except where a
  binding equals or lies inside one of the reset parser's words (a month
  name, an ordinal suffix, `AM`, `PM`): there the retry-after is none, as
  `bound_reset_span` asserts;
- **masked wire:** each quoted word, shown `tool`, serialized
  `Observation` and usage key as its exact masked string, and
  `secret::Guard` finds no form of the value (any shared encoding, JSON
  escape or number) in any emitted row, `session_meta` or word;
- **verdict:** each emitted call is judged through `Calls::consume` as the
  engine judges it, so a rewritten native name is refused, never
  `Ok(None)` and never stamped;
- **metadata:** as measured for the adapter, in its slice's test below.

The named acceptance tests, by owning slice:

- **0114a.** `bound_driver_step_keeps_the_refusal`: `star` inside
  `harness-started` on a cold Codex refusal with no announced thread
  (`turn.failed`, `usage_limit_exceeded`) and on Claude's shared finish
  path, `scri` inside `transcript` on an announced Codex stream, each on
  a stream isolating the driver's row. Each ends in one `failed`
  `Result`, class `RateLimit`, exact words and retry-after, with no
  `Accepted` and no `Checkpoint`; a flushing Claude run asserts the
  decided `Step`s equal and the wire `step` as `harness-[secret:S]ted`
  and `tran[secret:S]pt`. `bound_id_part_confirms_by_raw_ident`: a
  binding over part of an offered Claude and dsh session id reads
  `Resumed`, and over part of an announced Codex thread id selects that
  thread's rollout, each asserted apart from the exact masked locator,
  with no root recorded; and a result-only Claude stream, whose only
  locator is the `result` row's `session_id`, confirms by the raw `Ident`
  the same way. `bound_native_tool_name_fails_closed`: Claude's and dsh's
  `WebSearch` and `Bash`, and Codex's `web_search`, each held and unheld,
  with a binding inside the name, through `run_seat_with` and
  `Calls::consume`: refused as `Unattributable`, never `Ok(None)` and
  never stamped. #1261 lands that floor first, with its own tests, and
  every family's slice keeps it. Exec takes the free-text case over its
  stdout.
- **0114b.** `bound_word_claude_family`: Claude and LaneTally keep
  nonzero usage, the locator, model and effort; a binding inside a model
  or effort reads `not reported` on the seat-turn row, and `session_meta`
  keeps the prior valid value, or none. Qoder's usage is zero, so not
  reported, its `session_id` is dropped, so no locator, and its model is
  the pin, also against a top-level or result-row `model` (#1256 item 1);
  a bound `assi` with a late stop and no delivery ends lost, not refused.
  `bound_name_across_separator`: `mcp__srv__tool` with a binding over
  `v__t` yields the exact wire `Observation`, `McpUnidentified` with the
  whole masked name, and the shown `tool` equal to it.
- **0114c.** `bound_word_codex`: usage and locator kept. A binding inside
  the stream's model rejects it, and the model falls through to the
  rollout's, then to a stderr model equal to the pin, else `not reported`,
  each case asserted. A binding inside the effort reads `not reported`,
  with no bracketed value kept. `bound_reset_span`: a binding inside the date, and one
  starting in the prose before it and ending inside it, give no
  retry-after (asserted on the value, which a literal scan cannot see);
  one only in the prose gives the unbound run's.
  `later_start_after_error_is_work`: a rollout whose rate-limit error
  row stands alone reads as a refusal to start, class `RateLimit`. The
  same rollout with only a later `task_started` added reads as work, and
  the test asserts the change. `complete_row_present_not_has`: a
  `task_complete` whose `last_agent_message` is a string reads delivered,
  and one whose `last_agent_message` is null reads not delivered, so
  `present` read as `has` fails it. `thread_error`'s key-present read
  keeps its own null case.
- **0114d.** `bound_word_dsh`: usage, model and the stream `session_id`
  kept. The effort each step paints is the retained or seeded value as
  measured, asserted per step, and a binding inside it reads
  `not reported` on that step and leaves the retained value. A bound
  `QUOTA` is still `RateLimit`, and `bound_name_across_separator` runs
  over a dsh `/data/name`.

Each test binds (0071 ruling 9) by a removal control dropping what the
fold or driver keeps: a matched field read through `text`; a usage key,
the MCP split, the effort or the locator dropped from the projection; a
`text` read without the mask; masking a name's components alone; `reset`
without its interval check; `present` read as `has`; `begins_work` from
the masked row; confirmation and rollout lookup from the masked id, with
the retained `Ident` dropped.

## The slices (D5)

Ordered after #1178, #1199 and #1213, which this base holds.

Each operation lands with its first production consumer (0071 ruling 6).

- **0114a, the shared rule.** `adapters/row.rs` with `Row` and the
  operations the shared classifiers use (`kind`, `is`, `contains`,
  `flag`, `count`, `number`, `token`, `text`, `items`), its unit tests,
  and the shared classifiers (`claude_refusal`, `codex_refusal`,
  `dsh_refusal`, `probe::is_answer`) read through it. It carries the
  driver-checkpoint rule (`Emitted`, `Step`, `begins_work`, the buffer)
  and `ident` with `is_offered` and `names_rollout`, captured raw at
  every mask site, whose consumers are `LaunchHold::confirm`, dsh's
  watch and `find_codex_thread`.
- **0114b, the Claude family.** Claude, LaneTally and Qoder:
  `invoke_stream_json`, `fold_stream_event`, `fold_result_meta`,
  `claude_fold.rs`, and the Claude half of `capability_calls.rs` with
  `tool`; Qoder's own reads with `pin`, and the confirmed pin as the
  served model. The row masks at `adapters.rs:2494` and `qoder.rs:404`
  go. It closes #1256's item 1 and the Qoder `worked()` case.
- **0114c, Codex.** `fold_codex_event`, the Codex half of
  `capability_calls.rs`, the header `Ident`'s equality, and the rollout
  echo (`CodexThreadEcho`, `refusal/codex_thread.rs`) with `at`,
  `present`, `has`, `equals`, `since` and `reset`. The row mask at
  `adapters.rs:3697` goes.
- **0114d, dsh.** `fold_dsh_event`, `dsh_row_is_current`, `dsh_effort.rs`,
  the stream line and its confirmation by `is_offered`, and the raw
  header and store readers. The row masks at `adapters.rs:2350` and 4957
  go.

Each slice carries its D4 tests, and each `Row` operation its unit tests.

## What stays out

- **The journal's readers.** The resume offer, the meter and costs read
  the journaled checkpoint (D1), whose `step` and keys `seat_row` masks.
  A binding inside `seat-turn` or `root_session` leaves a turn unmetered
  or an offer withheld there. Exempting the engine's words from the mask
  would put a bound value in clear in the journal, which weakens a mask;
  whether `brokkr secrets set` should refuse a value equal to or inside an
  engine word is a follow-up.
- **The seat's result file.** `adapters.rs:6768` masks a seat-written
  result's keys too, so a binding equal to `notes` rewrites that key. The
  result file is the seat's own JSON, not a harness row, and is a
  separate question.
- **Masking twice.** A second mask over `[secret:NAME]` can nest a
  placeholder when a binding is inside the placeholder's own words. It is
  today's behaviour, and leaks nothing.

## Consequences

- A bound protocol word or driver step changes no turn, refusal, class or
  acceptance; the journal's readers still match masked keys and steps.
- A binding across a name's separator leaves no readable piece of it.
- A native tool name a binding rewrote fails closed (#1261), never open.
- A raw value copied out is a type error; the masker is unchanged, and
  every text read takes #1213's union rule. Four slices touch every
  adapter once.
