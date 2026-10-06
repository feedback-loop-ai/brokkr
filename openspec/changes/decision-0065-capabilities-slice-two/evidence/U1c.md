# U1c: Claude, Codex and LaneTally serving configurations (tasks 4.1–4.2)

U1c was built over four visits on branch `s2/U1c`. Run
`0065-slice-two-unit-u1c-see-the--21127bcf` built it from `d1e211bc` and
answered the review's F2–F6. It then returned oversized on F1, dsh's
engine-only home. The operator's 2026-10-06 addendum in
[operator-ruling-2026-10-03.md](../operator-ruling-2026-10-03.md) re-scoped
U1c to Claude, Codex and LaneTally, with a dsh intent refusing as missing
evidence. It moved dsh's home to the new U1c2 and its keyed-route
measurement to U0c. Run `0065-slice-two-unit-u1c-see-the--d4ec5be0`, the
third visit, reviewed the applied work against the amended row on main at
`edb1dbce`. It re-ran M1–M25 and the gates it recorded, and committed the
unit as `0306282c`. That run's review returned R1–R8 (below). Its fourth
visit, the same run's second implement, answered them, ran M26–M28 and
every gate again, and amends `0306282c` so the unit stays one signed
change.

The unit touches two of the row's three production files:
`crates/brokkr-protocol/src/adapters.rs` and the new
`crates/brokkr-protocol/src/adapters/mcp.rs`, a private child registered as
`mod mcp;`. `native_controls/mcp.rs` needed no change: the transport it
owns is reached through the re-exported `Transport` and its public fields.
No adapter, contract, policy, fixture, reference or extension file moved,
and the MCP compile fence is untouched.

## What changed

The private serving edge moved whole into `adapters/mcp.rs`: `served`, the
`UNPAIRED` and `UNSEALED` refusals, and the hands transport binding, now
`engine_executable` and `hands_transport`. Every serving builder that
called `served` still calls it: Claude and LaneTally cold and rejoin, Codex
rejoin and cold replacement, and the dsh cold and rejoin commands. Beside it
the module adds the engine's typed isolation intent and the builder that
consumes it:

| Item | What it is |
| --- | --- |
| `Edge` | One launch's driver input as the private edge reads it. The sealed serving inputs are decoded at most once, on first use, through a `OnceCell`, and `isolated` and `served` share that one value. |
| `Isolation` | The intent under the driver-input key `mcp_isolation`, read once through `serde` with `deny_unknown_fields`: `servers` (`ServerSet::Empty` or `Hands`, the only sets this no-broker plan has), then `cold`, `replacement` and `resume`, each an `Assessment`. |
| `Assessment` | `measured`, `{"unsupported": <reason>}` or `unmeasured`. A reason must be one bounded line: non-empty, at most 400 scalar values, no control character. |
| `Invocation` | `Cold`, `Replacement` or `Resume(<shape>)`. The cold replacement of a declined or rejected rejoin is judged by its own `replacement` assessment, as U1b's runtime `McpInvocation::Replacement` measures it, and never by `cold`. |
| `isolated` | The builder every launch builder calls after `composed_launch`. With no intent the arguments are returned unchanged, since mandatory strict admission activates at U1g. With one, a launch offered no rejoin is admitted `Cold`, and an offered launch `Replacement`, because every offer can end as its replacement. The intent's server set must then equal the sealed inputs' hands. The empty set gains `--strict-mcp-config --mcp-config {"mcpServers":{}}`, the document U0 cell C03 measured, beside no other MCP configuration, and only once the provider's grammar places the whole argv (`with_empty_set`). The hands set rides the adapter's measured workspace fragment, whose flag and document `untransported` and `Transport::carried_by` already prove at the final check. |
| `Isolated::resumes` | Whether a rejoin keeps the isolation. Claude's launch asks it after the resume gate and before any probe. A resume shape that is not measured declines the offer as `restrictions-unavailable` (the seat-record v5/v6 token for restrictions not measured here), and the launch is the already-admitted replacement. |
| `McpRefusal` | The `thiserror` refusals. `Unsupported` and `Unmeasured` are SI2's two site causes word for word. `NotSealed` is the final-configuration cause, `Unplaced` refuses arguments that do not place the appended configuration as options, and `Unreadable` and `UnboundedReason` cover a malformed intent. `at_launch` renders `refusing to invoke the agent CLI: <cause>`. |

Per harness, under the amended row:

| Harness | What U1c builds |
| --- | --- |
| Claude | The U0-qualified mechanism (C03–C06): the strict flag with an engine document, cold, as the replacement, and on a rejoin whose resume shape is measured. |
| LaneTally | The same grammar, which its wrapper forwards (LT03–LT10). It is served by the same `claude_launch`. |
| Codex | No U0 candidate passed, so no shape is qualified. A measured limitation refuses with its own reason. A measured claim refuses as missing evidence rather than borrowing Claude's result. `codex_launch_and_cold` judges the intent right after composition, before the cold or rejoin command is built. |
| dsh | Every intent refuses as missing evidence (operator ruling, 2026-10-06), with the shape named: `cold` for a launch offered nothing and `replacement` for an offered one. The assessment the intent records, measured, unsupported or unmeasured, never replaces that cause. `dsh_argv` judges it right after composition, before any route claim, version probe, overlay staging or composite read. `dsh_argv` is also doctor's reading of a dsh command, so doctor sees the same judgment. The engine-only `DSH_HOME` is U1c2's. |

`admit` refuses dsh first, before it reads the assessment. It then matches
`ServingShape` exhaustively, so a new serving shape cannot inherit Claude's
admission without an edit here.

`adapters.rs` went from 7,202 lines on main to 7,116. No function crossed
100 lines. The three listed long functions kept their counts, and only
their anchors moved in `quality/too-many-lines.txt` (`fold_stream_event`
:1518, `dsh_launch_with` :4415, `run_seat_with` :6477). Nine long tests in
`adapters/tests.rs` moved anchor the same way, with their counts unchanged.
A crate-scoped `cargo clippy -p brokkr-protocol --all-targets
--all-features` with `too_many_lines` force-warned listed the same 36
adapters entries at those anchors. It counts `fold_stream_event` at 105
against the committed 108. That function's body is untouched by this unit,
and the baseline is written by a workspace run, which this visit did not
repeat, so the entry keeps its committed count. `quality/file-lines.txt` records
`adapters.rs` at 7,116, `adapters/mcp.rs` at 433, `adapters/mcp/tests.rs`
at 500 and `adapters/tests.rs` at 19,089, under its 19,093-line baseline.
The fourth visit moved only the two `mcp` counts, measured by `wc -l`.

## Assumptions

- **Where the intent comes from.** Nothing in the runtime writes
  `mcp_isolation` yet. U1f threads the strict intent and U1g binds it at
  dispatch. The private key is the typed engine input this row defines, and
  its consumers are the existing launch builders.
- **The sealed empty set.** The final check recomposes a sealed command
  from its sealed inputs alone. Those inputs do not carry the empty set, so
  a sealed launch carrying the empty intent is refused there, as
  `a_sealed_empty_set_is_refused_by_the_final_check_until_it_is_sealed`
  pins. Teaching `check_final` the empty set means editing
  `native_controls.rs`, which belongs to U1g's row, so it is not done here.
  The refusal fails closed: nothing is served unchecked.
- **Codex's qualified shapes.** The amended row's "U0-qualified isolation
  shapes for Claude, Codex and LaneTally" is read as building what U0
  qualified. For Codex U0 qualified nothing, so its exact refusals are the
  whole of its configuration until a later measurement qualifies a
  candidate.
- **Auth and session.** For the Claude grammar the isolated configuration
  adds only the strict flag and a document. The seat's model, effort and
  permission mode keep their places, no settings or credential source is
  touched, and a rejoin is the same argv plus exactly `--resume <id>`.

## Tests

The tests live in `crates/brokkr-protocol/src/adapters/mcp/tests.rs`, a
child of the new module, which keeps the owning `adapters/tests.rs` at its
baseline. That suite changed only size-neutrally. Seven fixture builders
(`version_preamble`, `enabled_input`, `Seal`, `Seal::authored`,
`Seal::hands`, `sealed_pair` and `claude_plan`) became `pub(super)` so the
child can share them, and `executable` already was `pub(crate)`. The one
`UNPAIRED` assertion now names `mcp::UNPAIRED`, and the seven direct
`served` calls pass an `Edge`. Each rejoin/replacement pair in
`an_eligible_codex_rejoin_and_its_cold_replacement_are_each_served_as_checked`
became one tuple assertion, so that function stays at 192 lines. No test
moved between files. `native_controls/tests.rs` and `capability_launch.rs`
needed no edit and pass unchanged. No runtime producer writes the intent,
so no runtime test can reach it yet.

| Test | What it proves |
| --- | --- |
| `the_empty_set_is_served_cold_with_exactly_the_strict_flag_and_the_empty_document` | Claude and LaneTally cold commands are the head, the seat's own arguments (permission mode, model, effort), then exactly the strict flag and the empty document. A launch with no intent is unchanged. |
| `a_rejoin_keeps_the_isolation_only_where_its_resume_shape_is_measured` | Through a version shim on a canonicalised temporary root, with cold unmeasured: a measured replacement and resume give the isolated argv plus `--resume <id>`. An unmeasured or unsupported resume is the cold replacement with `restrictions-unavailable`, no rejoin and no probe. An offered launch whose replacement is unmeasured or unsupported refuses with `replacement` or its reason. A launch offered nothing refuses on its unmeasured cold shape even with replacement measured. |
| `each_unqualified_shape_refuses_with_its_exact_cause` | Through `claude_launch`, `codex_command` and `dsh_argv`, each of these gives its exact refusal: Claude's unsupported reason, missing cold evidence for Claude and LaneTally, Codex's U0 reason, a measured claim for Codex, and dsh's missing cold evidence whether its cold shape is measured, unmeasured or unsupported with a reason. An offered Codex launch refuses with its replacement's reason, not its measured cold one. An offered dsh launch, driven through `dsh_launch_with` with a composite closure that is never reached, refuses naming `replacement` whether that shape is measured or unsupported. |
| `a_server_set_its_sealed_inputs_do_not_declare_is_refused` | `NotSealed` covers six cases: hands with none sealed, hands or empty beside unreadable inputs, empty beside sealed hands, and empty beside an existing strict flag or document. `Unplaced` covers a trailing `--`, a dangling `--model` and an unmodelled `--frobnicate`, and through `claude_launch` a seat argv of `--`. The positive control is the seat argv followed by exactly the empty set. The sealed hands launch is served exactly as without an intent, with the strict flag and an independently written hands document. |
| `a_sealed_empty_set_is_refused_by_the_final_check_until_it_is_sealed` | A sealed launch with no intent is served exactly. With the empty intent, the final check refuses with its exact text. |
| `the_intent_is_read_closed_and_its_reasons_are_bounded` | An unknown member, an unknown set word, an absent `replacement`, an unknown assessment word and a valued `measured` are `Unreadable`. Empty, 401-character and two-line reasons are `UnboundedReason` in each of `cold`, `replacement` and `resume`, and 400 characters are read in each. |
| `each_refusal_reads_as_the_operator_sees_it` | The module's operator text, pinned once for every variant. |

This visit changed two tests. A mutation that removed only `cold`'s or
`resume`'s reason bound passed the first-visit
`the_intent_is_read_closed_and_its_reasons_are_bounded`, which placed every
reason in `replacement`, so the test now walks all three positions (M23,
M24). The offered-dsh assertion in
`each_unqualified_shape_refuses_with_its_exact_cause` now goes through
`dsh_launch_with` rather than `dsh_argv` alone. That binds the
`session.is_some()` the launch passes (M25), and the unreachable composite
closure shows the refusal precedes the composite read.

The fourth visit extended two tests, answering R1 and R6. In
`each_unqualified_shape_refuses_with_its_exact_cause` the table gained an
unsupported dsh cold case, and the offered-dsh assertion now walks a
measured and an unsupported replacement. In
`a_server_set_its_sealed_inputs_do_not_declare_is_refused` the `NotSealed`
table gained "empty, unreadable". No test moved between files.

## Mutations

M1–M25 were re-run in the third visit on the tree it committed; the
fourth visit did not repeat them, and its production change (the dsh
check in `admit`) touches none of the lines they mutate. M26–M28 ran in
the fourth visit. Each compiled and was applied alone to one file. The module's
tests (`cargo test -p brokkr-protocol --lib adapters::mcp`, M14 the whole
lib) failed as named. The file was then restored from the staged index
with `git checkout --`, and `git diff --quiet` confirmed the bytes. After
the last restore the module's seven tests and the whole lib (651 passed, 2
ignored) passed. M9–M12, M19–M21 and M25 are in `adapters.rs`, and the rest
are in `adapters/mcp.rs`.

| # | Mutation | Failing test: assertion |
| --- | --- | --- |
| M1 | the empty set gains no configuration | `the_empty_set_…`: the Claude cold command; `a_server_set_…`: the `--model m` positive control; `a_sealed_empty_set_…`: the final check's refusal; `a_rejoin_…`: the warm rejoin's command |
| M2 | `resumes` always true | `a_rejoin_…`: the declined offer's `restrictions-unavailable` tuple |
| M3 | Codex and dsh admitted as if built | `each_unqualified_…`: the Codex `measured` case, which read `Unplaced` against missing evidence |
| M3b | dsh alone admitted as if built | `each_unqualified_…`: the dsh `measured` case, which read `Unplaced` against missing evidence |
| M4 | an unsupported reason read as unmeasured | `each_unqualified_…`: Claude's unsupported case; `a_rejoin_…`: the unsupported-replacement refusal |
| M5 | server set not compared with the sealed hands | `a_server_set_…`: "hands, none sealed", `Ok([])` against `NotSealed` |
| M6 | an existing strict flag accepted beside the empty set | `a_server_set_…`: "empty, strict", `Unplaced` against `NotSealed` |
| M7 | every reason bound removed | `the_intent_…`: `cold: ""`, `Ok(true)` against `UnboundedReason` |
| M8 | `deny_unknown_fields` removed from `Isolation` | `the_intent_…`: the `brokers` member, `Ok(true)` against `Unreadable` |
| M9 | `claude_launch` skips `resumes` | `a_rejoin_…`: the `"unmeasured"` resume rejoined instead of declining |
| M10 | `codex_launch_and_cold` drops the refusal | `each_unqualified_…`: Codex's unsupported case, `Ok(())` |
| M11 | `dsh_argv` drops the refusal | `each_unqualified_…`: the dsh `measured` case, `Ok(())` |
| M12 | `claude_launch` serves the un-isolated composition | `the_empty_set_…`, `a_sealed_empty_set_…` and `a_rejoin_…`, as M1 |
| M13 | the missing-evidence text loses "measured" | `each_refusal_…` |
| M14 | `hands_transport` binds no hands | `a_server_set_…` and the existing `a_cold_codex_argv_carries_the_off_pair_exactly_when_search_is_not_held`, `a_boxed_codex_offer_stays_ineligible_and_its_denied_cold_fallback_carries_off_once` and `claude_admits_only_held_native_tools_beside_its_hands` (lib: 647 passed, 4 failed) |
| M15 | `isolated` admits every launch as `Cold` | `a_rejoin_…`: the warm rejoin's `unwrap`; `each_unqualified_…`: the offered Codex launch's replacement reason |
| M16 | `Replacement` reads the cold assessment | the same two assertions |
| M17 | `replacement`'s reason escapes the bound | `the_intent_…`: `replacement: ""` |
| M18 | `with_empty_set` admits without the parse | `a_server_set_…`: `["--model", "m", "--"]`, `Ok` against `Unplaced` |
| M19 | `claude_launch` passes `false` for its offer | `a_rejoin_…`: the warm rejoin's `unwrap`, refused on `cold` |
| M20 | `codex_launch_and_cold` passes `false` | `each_unqualified_…`: the offered Codex launch, `cold` missing evidence against its replacement reason |
| M21 | `dsh_argv` ignores `offered` | `each_unqualified_…`: the offered dsh launch, `cold` against `replacement` |
| M22 | `Unplaced`'s text changes a word | `each_refusal_…` |
| M23 | `cold`'s reason escapes the bound | `the_intent_…`: `cold: ""` |
| M24 | `resume`'s reason escapes the bound | `the_intent_…`: `resume: ""` |
| M25 | `dsh_launch_with` passes `false` to `dsh_argv` | `each_unqualified_…`: the offered dsh launch, `cold` against `replacement` |
| M26 | `admit`'s dsh check disabled (`if false && …`) | `each_unqualified_…`: the dsh unsupported cold case read "provider 'dsh' cannot exclude ambient MCP configuration (project MCP configuration cannot be excluded)" against "has no measured strict MCP configuration for 'cold'" |
| M27 | `admit`'s dsh check applied to `Invocation::Cold` only | `each_unqualified_…`: the offered dsh launch with an unsupported replacement read the unsupported cause against "… for 'replacement'" |
| M28 | `sealed_hands` reads unreadable inputs as no hands | `a_server_set_…`: "empty, unreadable", `Ok([strict flag and empty document])` against `NotSealed` |

The fourth visit restored each of M26–M28 by hand-editing the line back,
and `git diff --stat` then showed only the intended change. The module's
seven tests and the whole lib (651 passed, 2 ignored) passed after the
last restore.

M3 and M3b show a second fence: an admitted Codex or dsh intent would still
be refused by the Claude-grammar placement, fail-closed, but the tests pin
the missing-evidence cause itself.

## Review findings

The review of `1933f3f5` returned seven findings. The third visit checked
each against the tree being committed.

**F1 (dsh's measured cold shape)** was reported oversized by the second
visit. The operator re-scoped it on 2026-10-06: dsh's engine-only home is
U1c2's work over `transcript.rs`, `adapters/composite.rs`, `adapters.rs`
and `adapters/mcp.rs`, after U0c measures the keyed routes. Within U1c a
dsh intent refuses as missing evidence. In `0306282c` that held only for a
measured or unmeasured assessment, since an unsupported one returned its
reason first (R1 below); the fourth visit's dsh check in `admit` makes it
hold on every assessment. M3b, M11, M21 and M25–M27 bind it, and it is no
longer a gap of this unit.

**F2 (replacement)** holds. `Isolation` has `replacement`, `Invocation`
has `Replacement`, and an offered launch is admitted by it at
Claude/LaneTally (`session.is_some()`), Codex (`session.is_some()`) and
dsh (`dsh_launch_with` passes `session.is_some()`, doctor's
`dsh_cold_command` passes `false`). M15, M16, M19, M20, M21 and M25 bind
it.

**F3 (option placement)** holds. `with_empty_set` appends the strict flag
and the empty document, then requires `placed(provider, argv)`, the
native-controls grammar parse, to place the whole argv. Anything else is
`McpRefusal::Unplaced`. M18 binds it.

**F4 (decode once)** holds by structure. `Edge` decodes `serving_inputs`
at most once per launch through a `OnceCell`. `isolated`'s sealed-hands
comparison and `served` read the same value, as do both of Codex's
`served` calls. No behavioural test can count decodes, because a compiling
mutation that decodes afresh inside `served` changes no output, so this is
named here rather than claimed as a bound test.

**F5 (second document skeleton)** remains a LOW residual under decision
0071 ruling 5. The hands document is built by `mcp_config` in
`crates/brokkr-protocol/src/hands.rs`, outside this row, beside the empty
set's `EMPTY_DOCUMENT` here. Giving both sets one construction home needs
that file.

**F6 (budgets)** holds. `scripts/measure-budgets.sh` ran in the third
visit and reported "measured 101 prompt sites, 327 packages,
peaks:claude-session=2011418 codex-thread=11230208 dsh-session=4197432".
The package count and the three heap peaks equal the committed values, and
only each file's date note moved. Fifty-two prompt sites fell, each by
exactly 13 bytes. `git diff --stat edb1dbce -- bundles recipes agents
adapters realms.json crates/brokkr-runtime` is empty, so that drop is not
this unit's, and the three rewritten files were restored rather than
committed as another unit's measurement.

**F7** concerned panel prose and needs no change.

## Second review: R1–R8

The review of `0306282c` returned residual at medium. The fourth visit
answered each finding as follows.

| Finding | Disposition |
| --- | --- |
| R1 (medium): a dsh intent with an unsupported cold or replacement assessment read Codex-style "cannot exclude ambient MCP configuration" instead of missing evidence | Fixed. `admit` refuses dsh as `Unmeasured` for the named shape before it reads the assessment. Bound by M26 (cold, through `dsh_argv`) and M27 (replacement, through `dsh_launch_with`). |
| R2 (low): tasks.md listed U1e's dependency as U1d alone | Fixed: the U1e section now reads "Dependencies: U1d, U1c2", matching design.md's row and the addendum. This is the U1e entry, not the U0c or U1c2 entries, and not the preamble. |
| R3 (low): design.md stated 54 PRs | Fixed: 56, counting the unit-table rows U0 through U10a at this head against 54 at `edb1dbce`. The historical "retain 54 units" in the council table is kept as the count of its time. |
| R4 (low): F5's second MCP document skeleton | Kept as the recorded residual above: `hands.rs` is outside the row. |
| R5 (info): `MCP_ISOLATION` was `pub(super)` with no outside consumer | Fixed: now private; its only consumers are the module and its child tests. |
| R6 (info): no direct test of the empty set beside unreadable sealed inputs | Fixed: "empty, unreadable" added, bound by M28. |
| R7 (info): the module doc said an exec intent refuses, but exec's launch reads no intent | Fixed: the doc now says exec has no model MCP surface (SI2), so its launch reads no intent. No behaviour changed. |
| R8 (low): panel members' attempted directions on gate disposition | A run-integrity note on the review's own panel; nothing in this unit's files to change. |

## Documentation amendment

The worktree arrived with the operator's 2026-10-06 amendment applied:
the addendum, design.md's amended U1c row and section, the new U0c and
U1c2 rows and sections, U1e's added dependency, the two new
file-ownership rows, and tasks.md's 1.3–1.4 and 4.3–4.4. One correction
was made to it. The U0c section's last two paragraphs restated U0's word
for word, and `quality/ratchet.sh clones` refused them as a new 7-line
data clone (`design.md:1357` against `:1368`). They became one sentence
pointing to U0's documents, with the same meaning, and the ratchet then
reported "duplication holds".

## Gates in the third visit

All gates ran on the working tree over `edb1dbce`, with this commit's
content staged. `cargo fmt` reported no difference. Workspace clippy with
all targets and features, locked and warnings denied, finished without a
diagnostic, and so did `cargo +1.88 check --workspace --all-targets
--all-features`.

| Crate | Suites run |
| --- | --- |
| brokkr-protocol | lib (651 passed, 2 ignored), `hands_exits` (6), `secret_drop` (1), doc test (1) |
| brokkr-runtime | lib (831), `capability_launch` (75), `it` (120, which includes `witness_digests`, so the witness pins hold unblessed), `operated_repo` (1), `queued_launch` (3) |
| brokkr-cli | lib (627, 1 ignored), `driver_conformance` (27), `heap_claude`, `heap_codex` and `heap_dsh` (1 each), `it` (479, 2 ignored, which includes the suppressions count), `transcript_surfaces` (13) |
| every other crate | `cargo test --workspace --exclude` of the three above: every result line ok |

`compile --bundle bundles/self` exited zero. `openspec validate --all
--strict` reported 20 passed, 0 failed. `typos --hidden` and `git diff
--check` reported nothing. `quality/ratchet.sh` reported "file size holds",
"duplication holds" and "public API holds", and its `baselines origin/main`
mode reported "no baseline raised since origin/main".

## Gates in the fourth visit

The fourth visit re-ran the gates on its working tree over `0306282c`.

| Gate | Observed |
| --- | --- |
| formatting and workspace clippy (all targets and features, locked, warnings denied) | no difference, no diagnostic |
| MSRV check (`cargo +1.88 check`, workspace, all targets and features) | finished without a diagnostic |
| `cargo test -p brokkr-protocol` | lib 651 passed and 2 ignored; `hands_exits` 6; `secret_drop` 1; doc test 1 |
| `cargo test -p brokkr-runtime` | lib 831; `capability_launch` 75; `it` 120; `operated_repo` 1; `queued_launch` 3 |
| `cargo test -p brokkr-cli` | lib 627 and 1 ignored; `driver_conformance` 27; three heap tests 1 each; `it` 479 and 2 ignored; `transcript_surfaces` 13 |
| self bundle compile | printed the manifest, digest `11c7d0e7…`, without error |
| `openspec validate --all --strict` | 20 passed, 0 failed |
| `typos --hidden`, `git diff --check` | nothing reported |
| `quality/ratchet.sh` files, clones, api, `baselines origin/main` | "file size holds", "duplication holds", "public API holds", "no baseline raised since origin/main" |

The remaining workspace crates' suites were not re-run in this visit.

## Pending

- **Exact coverage.** `scripts/coverage-exact.sh` on a capable host or CI
  has not been run in this unit.
- **Remote CI and macOS.** Not run here.
- **Activation.** The runtime writes no `mcp_isolation` until U1f/U1g, and
  the final check learns the sealed empty set at U1g.
- **dsh.** Its engine-only home and qualified serving shape are U1c2's,
  after U0c.
