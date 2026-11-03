# U9b1b — Prove the enabled namespace path end to end

Branch `s2/U9b1b` from main at f0e0c9645. The first run
(0065-slice-two-unit-u9b1b-see-th-9ea6406d) made two partial visits and stopped on
F1; its record follows from "The proof the first run built". The second run
(0065-slice-two-unit-u9b1b-see-th-9d7328d4), recorded in "The second run" just
below, carries the controller's rulings. Every observation here was made on this
Linux x86_64 workstation (kernel 6.17, ext4 on LVM, shared with other lanes).

## The second run

### F1: `brokkr driver` sets `no_new_privs`

The ruling, verbatim from design.md:

> F1. Production never set `no_new_privs` on a harness, so on an ordinary Linux host MB3 refused an MCP seat under a plain `brokkr run` ("MCP server box filesystem identity is not protected") before any lookup. `brokkr driver` now sets `no_new_privs` on itself, through rustix's safe `thread::set_no_new_privs`, before it spawns a harness the engine brokers MCP for, on Linux. A driver that cannot set it refuses the launch. A launch with no brokered MCP is unchanged, and a seat that no `brokkr driver` starts keeps today's refusal. `crates/brokkr-cli/src/verbs/setup.rs` and `crates/brokkr-cli/Cargo.toml` (the `thread` feature on brokkr-cli's rustix; the lockfile does not move) join this row. Where the driver can learn of a brokered launch only through `crates/brokkr-protocol/src/adapters.rs`'s `serve`, that file joins too, size-neutral. Ruling 21's second case is then proved through real dispatch with no `setpriv` wrapper, and the fix has its own compiling removal recorded. Before this PR lands, the controller runs a real Claude and a real Codex harness through a brokered launch under the flag on a Linux host. A harness that breaks under it returns to the operator as a finding.

The driver learns of a brokered launch only from the `start` message's sealed
serving inputs, so `adapters.rs` joined the row. `serve` takes a `Confine`
(`fn() -> io::Result<()>`), a closure seam per decision 0071 ruling 2, and the
private `serve_with` loop calls `start::admitted` before `run_seat`. That checks the
resume exchange, then calls `Confine` only where the sealed set is
`SealedServers::Brokered`. A refusal is a `failed` result with no `accepted`, the
exchange's own structural path. `verbs/setup.rs`'s `confine` is the only call of
rustix's `set_no_new_privs`, on Linux; off Linux, where the fence stands, it refuses
with `Unsupported`. `adapters.rs` stays 6,989 lines. `start::serve_io`,
`#[cfg(test)]`, keeps the suites' 4-argument call as it was, so `adapters/tests.rs`
does not move. `quality/public-api`'s
one line for `serve` changes, measured with `cargo public-api` on the pinned nightly:
this is the signature change the ruling admits, adding no item.

| Test | Observed | Compiling removal, and how it failed |
|---|---|---|
| `enabled::a_seat_holding_an_mcp_grant_with_boxed_hands_dispatches_through_the_real_broker_and_completes` | `brokkr run` with no wrapper: `completed`, broker exit 0, the harness's own `NoNewPrivs:\t1` | `confine` returning `Ok(())`: `left: ("awaiting_operator", "work", Some("MCP server box filesystem identity is not protected"))`. Asking the kernel for `set_no_new_privs(false)`, which returns `EINVAL`, is the refusal through real dispatch: `left` holds "…the driver then failed: seat refused to start: its driver could not set no_new_privs before a harness the engine brokers MCP for: Invalid argument (os error 22)" |
| `enabled::dispatch::a_seat_without_hands_holding_a_native_grant_dispatches_and_retains_nothing` | A launch with no brokered MCP: the harness's flag is this process's own | `admitted` confining everything with a model surface: `left: "NoNewPrivs:\t1"`, `right: "NoNewPrivs:\t0"` |
| `adapters::mcp::tests::brokers::a_brokered_start_confines_its_driver_first_and_refuses_where_it_cannot` | A refusing `Confine`: `["capabilities", "result"]`, no `accepted`, the exact error; a hands-only start never asks | The refusal dropped (`let _ = confine()`): `left` holds the charter's refusal, reached past the check; confining everything: panics at the hands-only assertion |

Moved off `setpriv`: every launch in the `enabled` suite, whose 40 tests passed with
no wrapper (`enabled::` 40 passed, 1 ignored, in 271 s, at load near 13).
`enabled::an_unconfined_seat_refuses_by_identity_before_any_lookup` is gone with the
condition it pinned. A seat no driver starts keeps the refusal, pinned by
`capability_broker::every_managed_writer_is_observed_confined_and_sealed`, whose
broker an unconfined shell starts. Still on `setpriv`, as no `brokkr driver` starts
their harnesses: `capability_broker`'s shells and `observer.rs`, and
`writers_tests.rs`'s harness shells. The runtime's `capability_launch` and
`resume_tests` drivers pass `|| Ok(())`: they are test stand-ins, their launches
unchanged. Pending, by ruling F1, the controller's: a real Claude and a real Codex
harness through a brokered launch under the flag.

### F6: the lease answers first

MB3's table and its "Resolution failures" scenario in `specs/mcp-capability-broker/spec.md`
now record the order. `enabled::reach`'s `leased_off` pins the whole prefix,
"dispatch refused: MCP server box filesystem identity is not protected: ", for each
reach case. With the lease removed the first run saw the kept cause behind it.

### 46.1's remainder and 46.2, through real dispatch

New test files, registered through `enabled.rs`: `enabled/boxes.rs` and
`enabled/windows.rs`. The harness gained switches for them and now keeps its FIFOs per
process, as panel members share its directory. `enabled::` passed whole on the final
tree, 51 passed and 1 ignored (the sampler), in 408 s, the load falling from near 12 to
near 5. A pass before it, at load near 20, failed `a_spent_share…` with 8 members on the
survey's deadline (F8), so the test now uses 4. The earlier whole pass took 306 s at load
near 4.

The other suites on this tree, all before the 4-member edit, which touched only
`boxes.rs`: `cargo test -p brokkr-cli --locked --test it -- --skip enabled::` 825 passed,
3 ignored, in 349 s; `--lib` 1,105 passed, 1 ignored; `brokkr-protocol` 1,123 passed, 3
ignored, then 6, 1 and 1; `brokkr-runtime` `--lib` 1,447 passed, `capability_launch` 76,
`it` 144, `operated_repo` 1, `queued_launch` 8. Formatting, clippy over the workspace
with warnings denied, `quality/ratchet.sh files` and `clones`, `git diff --check`,
`typos --hidden` and strict openspec validation (36 of 36) were clean; `recipes/self`
compiled with "48 valuations over 11 groups, 0 unruled". No test moved; new test files
are named above. `crates/brokkr-cli/tests/capability_broker.rs` sits at its 2,000-line
ceiling after the part-one visibility edits, its one `pub(crate) mod observer` costing
no line.

| Test | Observed | Compiling removal, and how it failed |
|---|---|---|
| `boxes::a_native_binary_in_a_users_package_is_served` | A singleton copy of the system shell at the package's program, ELF bytes: its own `/proc/$$/exe`, broker exit 0, one call | `ServerProgram::resolve` refusing all but `/usr`: `left` holds "MCP server box program tree cannot be resolved" |
| `boxes::a_second_servers_private_directories_hold_nothing_of_the_first` | Two brokers in one attempt, the first standing: the second's HOME and TMPDIR empty before it writes, each then holding only its own file; two calls settled | The server box's `/tmp` tmpfs replaced with a bind of `/var/tmp`: `left` holds "MCP server box could not be established", an earlier cause, as the box's own check refuses a `/tmp` not private |
| `boxes::a_bind_alias_of_the_package_refuses_with_no_lookup_or_start` | Under an outer `bwrap` whose root and path to the world are the user's, so the coordination root stays protected, a seat binds a bind alias of the package writable: "dispatch refused: …can write '…/docs-mcp'", zero lookups, harness and server starts | `covers`'s `identical` and `writes`'s `table.reaches` removed together: `left: ("awaiting_operator", false, false)`, the survey's bare identity cause. With `table.reaches` alone removed it still passes, by identity |
| `boxes::a_spent_share_refuses_its_next_call_with_no_forwarding` | Four retaining members, so 64 MiB each: eight answers of 8,000,000 bytes settle, the ninth call is refused "capability evidence budget is exhausted", the server answered only eight | `ledger.reserve`'s denial ignored: `left: ("awaiting_operator", "work", Some("panel members failed — m0: broker ledger lifecycle is invalid"))` |
| `boxes::a_server_starts_with_its_exact_descriptors_and_environment` | Descriptors `0=pipe 1=pipe 2=pipe` alone; environment exactly `BROKKR_HANDS_BOX=1`, `DOCS_TOKEN=[secret:DOCS_TOKEN]` (relayed masked), `HOME=/runtime/home`, `LANG=C.UTF-8`, `LC_ALL=C.UTF-8`, `LOGNAME=runner`, `PATH=/usr/local/bin:/usr/bin:/bin`, `TMPDIR=/tmp`, `USER=runner` | `close_on_exec_above_stdio` skipped in `exec_confined`: `left` holds "MCP server secret environment could not be delivered", as an open status pipe refuses the handoff. Scan: an earlier cause |
| `windows::a_harness_dying_at_{its_brokers_observation, a_standing_box, a_held_store_read}…`, `…during_{initialization, a_call}…` | The harness SIGKILLed with its broker held at that window, under `strace -f -DDD` where a tracer holds it, the broker's input kept open by another session: the broker gone within 5 s, each process standing below it gone within 5 s more, the ledger a prefix of or closed on that window's record (`identity`, `establishment`, none at the store read, `timeout`, `timeout` with its failed call), the store unread before the handoff | See item 5 below |

F5 applies as ruled: "The real broker and bootstrap never write invalid status tags or a
malformed complete frame, so those rows of 42.2 and of MB4's complete-frame matrix are
proved at the private seams. 42.2 and 46.2 accept those seams." They stand in
`broker::session::readiness::tests`, `capability_broker::handoff` and the bootstrap's
own tests.

### Item 5, the two proofs part two dropped

**A harness dying at a standing box.** Two routes hid the broker's own end from the
first run. The harness held the broker's input, so its death was EOF on it; and the
driver's exit makes the engine sweep the attempt's tree (`tree.rs`'s `Host::REAL`,
`kill_group` and `table::kill`). `windows.rs` closes the first with a `setsid` keeper
of the input. The remaining two layers are independent. With `lifetime.rs`'s
`cancelled` made false alone, all five tests pass, on the sweep. With the sweep made a
no-op alone, the observation, entry, initialization and call tests pass, on the
parent check, and the store-read test fails on its park. With both removed all five
fail at the 5 s bound on the broker's end: that removal is the binding. The first
version's Initialization test passed under that removal because its predicate read a
relay the broker writes only at close, so it waited out the 30 s deadline; it now waits
for the server's own `sleep`, and fails.

The parks vary and the tests say so: the engine reads a ledger only once it proved the
attempt settled, so a harness's death parks with the broker's cause or with "indeterminate:
driver exited after accepting…", and the sweep may kill the broker before it closes, so
the ledger may be a prefix (seen for Observation). Neither claims success. See F9.

**The host loader environment.** `Entry`'s launch in `hands/namespace/entry.rs` calls
`env_clear()` on the box's launcher, ahead of `--clearenv`, which is what kept the
variable out with the first run's two removals. `--clearenv` removed alone still passes
for that reason. `boxes::the_host_loader_environment_stays_out_of_the_box` has the harness
export `LD_LIBRARY_PATH` to its broker. With `--clearenv` removed and the launcher given
exactly that variable, it fails with "MCP server box could not be established", the
bootstrap's own refusal of a name past MB4's. `cargo test`'s own `LD_LIBRARY_PATH` made
that removal fail the suite's other tests too, as the broker inherits it.

### Findings from this run

**F8 (fails closed, LOW).** The attempt's budget admits 32 retaining members, but on this
host a panel of 32 parked with "MCP server box survey did not answer before its startup
deadline" (seen once, at load near 6), and a panel of 8 parked the same way once at load
near 20, having passed at load near 4. Each member's plan surveys the host within the
one shared 30 s bound. So large retaining panels are not reachable under load here. The
spent-share test uses 4 members for that reason. Options: (a) record it in U9b2's guidance
as a measured limit; (b) share the survey among a panel's plans, a production change past
this row. I recommend (a) now, with SD4's quiet-host sample deciding if (b) is owed. The
same contention is why the whole suite can still flake on a heavily loaded host.

**F9 (LOW, for the controller's reading).** See "The parks vary" above: a harness that
dies mid-startup can park indeterminate instead of on the broker's closed cause. It is
fail-closed and honest, and nothing here changes it. No option needs ruling unless the
operator wants the exact cause always, which would mean the engine waiting for a
broker's closure past the driver's end.

**F7, updated.** `enabled::` takes 306 to 408 s here, serialized by `World`'s lock as
each attempt's survey shares a host-wide 30 s bound. `.github/workflows/ci.yml`'s long
jobs hold `timeout-minutes: 75`, which this does not approach by itself, so no CI change
is asked. CI itself has not run it; that remains pending.

### SD4 warm samples, this run

Five samples by `enabled::sd4_warm_samples`, load average 3.7 before and after: 4,591,
4,505, 4,547, 4,677 and 4,867 ms total, engine 3,084 to 3,287 ms, broker 1,400 to 1,579 ms,
all within budget. The host was not quiet, so these are no verdict.

### The review's return

The review returned this run with one medium and five lows (M1, L1 to L5); this section
answers them, every line observed in the returned visit. All of them were in the proof, and
no production file moved: each removal below was applied, run and restored with
`git checkout`.

**M1, the interrupted call.** The test deleted the call's `id`, and the decoder turns an
id-less frame into a notification, so no call began. Under `kill-broker` the harness now says
the whole call, does not wait for its answer, and kills the broker once the plan's ledger
holds `"started"`. The test observes broker exit 137, the ledger's records exactly `opened`
and `started`, the lifecycle refusal, one harness start, and through `inspect` the one row
`("resolve", "interrupted", None)`. So 42.2's verified-prefix sentence is proved by a begun
call. Started is what it synchronizes on; that the server had received the frame before the
kill is not observed, as its box has no channel out until the broker relays at close.

**L1, the restart proof.** The restart proof now asserts the fault first: the broker's exit 1
and its closed disposition, before it relies on the restart. It runs for each real fault:
exec, handoff and status EOF, closing on `{"failed":{"refused":"exec"}}`,
`{"failed":{"refused":"handoff"}}` and `{"failed":"protocol"}`. The invalid-status rows stay
at F5's seams. `parked_by`'s comment now says it inspects; `resume` is the restart tests'.

**L2, the windows' claim.** `enabled/windows.rs`'s module text now claims what `cancelled_at`
asserts: the broker's end and its tree's within the bounds, a ledger that is a prefix of the
window's records, and a park on the window's cause or a driver's lost result, never success.
F9's limitation stands.

**L3, the loader comment.** The comment no longer says the driver spawn clears the
environment. `engine.rs` inherits it; the box's launcher and `--clearenv` clear it, as item 5
found.

**L4, the hands-only control.** It now pins the launch's own next step, the exact charter
refusal, past a `Confine` that would have refused.

**L5, controls that reach the claim.** The earlier removals of these rows fail at an earlier
cause, which shows sensitivity and not the final assertion. Each row below reaches its own.

| Test | Compiling removal | The assertion reached, and how it failed |
|---|---|---|
| `transport::a_closed_that_cannot_persist…` | `fold::ledger` dropping its rows where `invalid` is set | Its last row assertion: `left: []`, `right: [("resolve", "interrupted", None)]` |
| The same | The fixture's call without its `id` (M1's old frame) | `left: ["opened"]`, `right: ["opened", "started"]`, the old fixture's own defect shown |
| `transport::a_restarted_engine_settles_an_exec…` | The fixture's faulty server not installed, so a healthy broker | `left: Some(0)`, `right: Some(1)`, at the fault's assertion |
| `boxes::a_second_servers_private_directories…` | Both private directories one bind of a shared directory, with `bootstrap`'s `dir.empty`, `fresh` and `each` removed | The second server's own listing: `left: ["/runtime/home: mine-first  /tmp: mine-first ", "/runtime/home: mine-first mine-second  /tmp: mine-first mine-second "]`. The first server's files, as the test claims they are not |
| `matrix::a_normal_binding_and_private_directories…` | Each private directory seeded with `--dir <target>/seeded`, with `dir.empty` removed | Its relayed listing, `said` holding `seeded` under `/runtime/home:` and `/tmp:`. `boxes::a_second_servers…` failed the same way, `left: ["/runtime/home: mine-first seeded  /tmp: mine-first seeded "]`. With only the seed, both refuse earlier, "MCP server box could not be established", by the bootstrap's own check |
| `boxes::a_server_starts_with_its_exact…`, descriptors | `sealing` leaving a `dup2(0, 40)` across the exec | `left: [Some("0=pipe"), Some("1=pipe"), Some("2=pipe"), Some("40=pipe")]`, `right: […, None]` |
| The same, environment | `SERVER_ENVIRONMENT`'s `LANG` made `POSIX` | The exact list: `left` holds `"LANG=POSIX"` |
| `matrix::a_home_or_tmpdir_binding…` | `fixed_key` false and `mcp::unbind` a no-op, so the name reaches the broker, whose own screen gives the same startup cause | Past the cause, its zero starts: `HOME: no harness`, `left: 1`, `right: 0`. With `fixed_key` false for `TMPDIR` alone, the removal of `unbind` kept: `TMPDIR: no harness`, `left: 1`, `right: 0` |
| `adapters::mcp::tests::brokers::a_brokered_start_confines…` | `admitted` confining every sealed start | `left` holds the no_new_privs refusal, `right` the charter refusal |

**L6, the suite's time.** Nothing changes. The suite's serialization and F8's survey deadlines
remain as recorded, unqualified on a loaded host. No 75-minute overrun is demonstrated, and
this visit added two restart tests' time to it; see "What this visit ran".

### What this visit ran

On the tree this visit commits, the host's load near 18: `enabled::` 53 passed, 1 ignored
(the sampler), in 358 s, and again with `--nocapture` in 371 s, which printed no `skipped:`
line, so every proof in it ran here; the CLI's `--test it` without
`enabled::` 825 passed, 3 ignored, in 306 s; its `--lib` 1,105 passed, 1 ignored;
`brokkr-protocol` 1,123 passed, 3 ignored, then 6, 1 and 1; `brokkr-runtime` `--lib` 1,447,
`capability_launch` 76, `it` 144, `operated_repo` 1, `queued_launch` 8, all passed. Formatting,
workspace clippy with warnings denied, both ratchets, `git diff --check`, `typos --hidden` and
strict openspec validation (36 of 36) were clean, and `recipes/self` compiled with "48 valuations
over 11 groups, 0 unruled". No warm sample was retaken in this visit, as the host was loaded
and no production file moved. No test moved and no new test file was added. The two new
restart tests' own time was not measured apart; `transport::`'s eight took 72 s together.

### The review's second return

The chief returned head f36596c8f with M1, M2 and M3 at medium and L2 to L5 at low. Answering
M3's audit found a defect no earlier visit or review had reached, F10 below. It is a fail-open
path in U9b1a's lift, and this visit fixes it. The suite moved (L2), so its tests are now
`capability_broker::enabled::…`; the older sections keep the `enabled::` names they ran under.

**F10 (security), fixed in the row.** MB2 orders hard checks after the hands check: "requires
workspace hands with network false", "cannot protect broker evidence from this provider's
native tools", and, for a dialect with secrets, "cannot protect its secret bindings from this
provider's native tools". D3 step 4 repeats that order, and 46.1 asks that "unsafe secret reads
refuse". None of those strings stood anywhere in `crates/`, and production read only the
`ambient` axis of an adapter's shape (`bundle/mcp.rs:161`). So since the lift a brokered seat
was served past each of them. A scratch probe through `brokkr run` on the previous head ended
`completed` with one harness start in each of four cases: boxed hands with `network: true`, and
every shape's `native_write`, `store_read` and `process_read` in turn declared unsupported. The
holds kept this from any house realm. `McpSite::contribute` now makes those three refusals, on
every shape the seat is served as, whatever the ask's strength, in MB2's order. The network
fact is read from the composition's spec at an agent site. At an inline site it is read as
`McpIntent::NetworkedHands`, as `capability_record.rs` is not in this row and `McpIntent::inline`
already takes the spec. `engine/isolation.rs` seals that variant as the hands set. No public
item moves.

The fix has one consequence the operator should read. Claude's shipped boxed replacement has
`store_read` unmeasured (U0b C17 read the store before U8h; C17-p5, on 2.1.310, was denied after
it, which is not that shape's 2.1.308). So on shipped facts a Claude work seat binding a secret
now refuses, and the same seat binding none is served, while Codex's boxed shapes are all
measured. `the_shipped_claude_adapter_serves_a_secret_free_work_seat_and_refuses_a_secret_bearing_one`
pins exactly that on the shipped file. Ruling 21's second case reads "Claude or Codex", so it
stands, but the `World` fixture now carries one synthetic `store_read` fact so that its Claude
seat can exercise the secret path; the suite's secret-bearing proofs are the broker's and not a
claim about shipped Claude. Options: (a) re-measure Claude's boxed replacement on the current
binary and record it in a U1d row, which lifts the refusal; (b) leave it refusing until a later
unit; (c) judge only the cold shape, which MB2 and SI2's served-shape ruling do not allow. I
recommend (a) before any house realm grants a secret-bearing server. The controller's brokered
real-harness check under F1 meets this refusal on Claude unless its dialect binds no secret or
(a) has landed.

| Test (`matrix::`) | Observed | Compiling removal, and how it failed |
|---|---|---|
| `a_site_lacking_each_of_mb2s_hard_facts_refuses_by_its_cause_whatever_it_asked` | Network ahead of the evidence and the secrets, for `requires` and `wants`; `native_write` on the cold and on the replacement shape, ahead of the secrets; `store_read` and `process_read` on each; each the exact site-prefixed cause, no harness, lookup or read of the server | The network guard false: it panics wanting the network cause while `said` holds the evidence cause, so the order is shown. The evidence guard false: wanting the evidence cause, `said` holds the secrets cause. `process_read` dropped from the proof: "secret bindings" not said, at the process row. `store_read` dropped: the same, at the store row. `proved` taking only the first shape: the replacement row's cause not said |
| `an_inline_site_whose_hands_ask_the_network_refuses_an_mcp_holding` | The inline site's spec alone gives the network cause, zero starts and reads | `Some(spec) if false && spec.network`: the cause not said, at the helper's assertion |
| `a_secret_free_holding_needs_no_secret_read_proof` | Store and process axes unsupported, no secret bound: completes with one call | The secret-bearing test made true of every holding: parks "no state" with "cannot protect its secret bindings" |
| `the_shipped_claude_adapter_serves…` | The shipped file: refused with a secret, served with one call without | `store_read` dropped from the proof: the secret row's refusal not said |

**F11 (LOW, fails closed).** MB2's first cause, "requires boundary 'namespace'; this site uses
'<boundary>'", stands nowhere either, as `McpIntent::composed` collapses the boundary and
`capability_record.rs` is outside this row. Through `brokkr run` each other boundary still
refuses before any harness, by another cause: `seatbelt` and `container` as "built by slice…
not by this engine", `harness` as its fragment's capability gap, and `open` as "requires
workspace hands at this site". Only the words differ. Options: (a) one row for a following unit
that adds `capability_record.rs` and the exact cause; (b) amend MB2's table to these words. I
recommend (a). **F12 (LOW, fails closed).** MB2 says its hard checks run before GP1, but
`authority.resolve` holds GP1 ahead of `contribute`. A gate's wanted ask therefore drops by GP1
where MB2 would refuse it, and a dropped holding serves nothing. I recommend amending the order
in the spec at U10a's audit, as moving it changes which cause every gate reports.

**M1, the missing proofs.** `windows.rs` holds each window through real dispatch and ends it
three ways, with `Standing` the one setup. (Corrected by the controller, 2026-10-31: not at `Handoff`, which only
the startup deadline ends; see "Tasks".) A sixth window, `Handoff`, delays the bootstrap's exec
of the server with the whole frame delivered and its status pipe open.

| Tests (`windows::`) | Observed | Compiling removal, and how it failed |
|---|---|---|
| `an_observation_held_to…`, `a_box_held_at_its_entry_to…`, `a_store_read_held_to…`, `a_status_pipe_open_to…`, `a_server_silent_to_the_startup_deadline…` | The deadline with the harness alive: the broker ends at 30 s or later and before the 60 s hold; parks on `Refusal::Identity`, `Establishment`, the lifecycle refusal (a held read withholds its closure), `Handoff` and `Failure::Timeout`; exact ledger and disposition, exit 1, no call row; store reads and server reads each held to the window's table | `STARTUP` at 5 s: all five fail the bound at about 5.03 s, after the first version let the store-read row pass, as `broker.code`'s time followed the tracer's hold, so the test now times the broker's own state. `STARTUP` at 70 s: all five fail, three at about 70.03 s and the two held rows when the read's release changes the cause |
| `the_engines_timeout_at_{a_brokers_observation, a_standing_box, a_held_store_read}…`, `…during_{initialization, a_call}…` | The seat's `limits.timeout_seconds` of 25, the engine's own sweep: parks "attempt exceeded its Ns deadline and was killed; its process tree is gone, and what it did is unknown", N what admission and provisioning left (19 and 20 seen); the broker and everything below it gone within 5 s | `timeout_seconds * 4`: the four startup windows park on their broker's own cause instead, and the call window panics "the engine did not end the window" at 90 s |
| `the_engines_timeout_during_a_call_leaves_it_interrupted_through_inspect` | `inspect` keeps `("resolve", "interrupted", None)` | `fold` clearing its rows where `invalid` is set: `left: []` |
| Every window, by `Standing::settled` | Private-mount teardown observed: a box's `/proc/<pid>/ns/mnt` read at the hold, non-empty wherever a box stands, and no process on the host left in it | `--die-with-parent` removed alone passes (the broker's teardown and the sweep stand behind it). With `tree::kill_group` and `table::kill` also no-ops, `a_harness_dying_during_initialization…` fails "3432349 holds the box's mounts" |
| `sites::a_gate_holds_a_native_power_by_its_class_and_named_office_only` | GP1's five rows through `claude-native-fetch`, the capability's classes as the dialect's, `requires` and `wants`: exact refusal with no harness, or notice with `--tools` empty and `WebFetch,WebSearch` disallowed, or `--tools WebFetch` with `WebSearch` disallowed, no broker and no call row | The egress arm of `gates::check` removed: both tables fail at the egress row's refusal. `native_plan` given no holdings: the native table fails (a panic in `attribution.rs:40`) while the MCP table passes |

The windows release `World`'s lock once held, as each then waits on a clock, so these 15 tests do
not add their waits end to end. F7's measure below is updated.

**M2, the marker that could not fire.** It was as the chief said: a server's box mounts nothing
writable of the host, so its `touch` could not leave the marker. `STARTED` and `marking` are gone.
`World::server_reads` watches the program's inode through the existing `Reads` inotify seam, which
sees reads across namespaces. It also sees the kernel's own read at an exec: an unmodified
`the_exec_failure_tag…` saw a read with no interpreter to make it, so that test makes no claim
from this observation and it reads "an exec attempted or done". The healthy control asserts it
moves (`a_seat_holding_an_mcp_grant…`: `started.lookups() > 0`). The independent binding keeps the
cause: a removal running the server by `/bin/sh` at `provision_with`'s and `refuse_admission`'s
failures and at `serve`'s, with every refusal's text left alone. Then `refused_unread`'s "no server
start" fails `left: 1`, `reach::leased_off` fails `left: (0, 0, 1)`, `a_store_in_the_worktree…`
`left: (0, 1)`, `boxes::a_bind_alias…` `left: (0, 0, 1)`, `an_empty_or_missing_store…` and
`the_handoff_failure_tag…` `left: 1`, across all 10 tests that claim no start.

**M3, the audit.** Answered by F10, F11 and F12 above, and:

| Audit item | What was checked on this head | Result |
|---|---|---|
| Adapters' qualification | `jq` over `adapters/{claude,codex,dsh}.json`'s shapes, axis by axis, against the U0, U0b and U0c records in `evidence.md` and the U1d rows | Claude: cold boxed all measured at 2.1.287; replacement boxed at 2.1.308 with `store_read` unmeasured. Codex: cold, harness, none and replacement all measured at 0.164.0. dsh: cold none at 0.1.5-rc.1 and replacement none at 0.2.1-rc.2, each with unmeasured axes, and cold boxed with `ambient` unmeasured, so a boxed dsh seat refuses by SI2 (#896's). LaneTally: cold and replacement no-hands only, over its wrapper's own identity |
| Which axes bind | Production consumers of each axis, by grep | Only `ambient` bound before this visit (F10). A shape's `measured_on.version` binds nothing: nothing compares it to the installed harness, so a later harness is served on the older measurement. That is the "exact evidence scope" U1d records and is not a defect of the lift, but operators should read it as such; U9b2 owes the words. The harness versions on this host could not be read, as this seat may not run them |
| SI2 switched on | `init_doctor::`, `init_stacks::` and the recipes' compile on this head: 60 passed. Removing `McpFence::admit`'s refusal passes all 58 of the first two, as after U1d6 no shipped scaffold reaches it | Not bound by that matrix, so `a_served_shape_without_measured_isolation_refuses_with_sis_exact_cause` binds it through the binary, for both causes and no holding. The removal: it fails, the cause not said. Init's "scaffolded bundle failed to compile" wrapper around an SI2 cause is unreachable from shipped facts and stays unpinned; 8.1 and 8.2 stay U10a's to reconcile |
| Open prerequisites | The tasks.md unticked rows before 46 | 8.1, 8.2 (above); 28.9, 28.10 (SD4 samples; native alias creation, recorded open since U6c5b and not tested here); 28.17, 31.1, 31.2 (SD4 samples). All carry stated reasons; none is closed here |
| Unit removal evidence | Each row of design.md's unit table has its record: a file under `evidence/`, or a section of `evidence.md` for U0, U1a, U2, U3a, U4a, U5a, U5a2, U5c, U5f and U6a | Present. Their mutations were not re-executed here; 48.1 owns that |
| Real namespace results | This visit's runs above and below, on a host with `legacy_tiocsti` 0 and no hands box. Every removal above failed where it says, so those tests executed; the suite was not re-run with `--nocapture` in this visit | Linux x86_64 only |
| macOS refusal | `McpFence::current` by `cfg`, `Standing` off Linux; `the_compile_fence_refuses_an_unused_mcp_grant_only_where_it_stands` passes its Linux half here | The macOS half is CI's, pending |
| Scenario map | The rows naming 46.1–46.2 | MB1's three servers now asserted in `a_second_servers…` (`brokkr`, `cap-library-docs`, `cap-more-docs`). The rest are named in the tables above and earlier; the complete-frame row is F5's |

**L2, the shard.** The suite moved whole, to `capability_broker/enabled.rs` and
`capability_broker/enabled/`, declared by one `mod enabled;` in `capability_broker.rs`, which
stays at 2,000 lines, its stale "compile fence still refuses every MCP grant" sentence paying for
the line. `test-shards.sh`'s two filters now select it with `capability_broker::`, with no script
or CI change. F8's four members and its reason stand. `cargo test --test it -- capability_broker::`
passed 177 with 1 ignored in 567 s, at load 22.7 falling to 7.2.

**L3.** The admitted run now holds until the other has ended or been admitted, so neither
reservation can go first; `Second::run`'s comment now says what it does. With the lease's
`writes` check false: "both admitted, live at once", both `completed`.

**L4.** It is now the same-run sibling, `a_boxed_sibling_in_the_run_cannot_alter_the_attempts_evidence`.
The boxed review seat strikes by glob at everything under the coordination root, with its own
control that it ran, and the plan and ledger are compared with what the harness copied as its
broker closed. With `/var/tmp` bound writable into every workspace box, the strike reaches the
root and its `rm -rf` parks the run: "the admission lock is not this user's own single file".

**L5** stays as the chief carried it: `engine/broker.rs:509` repeats `broker/session.rs:285`'s
fixed-key predicate, as sharing it crosses crates and grows the API. **The INFO**, the stale
comment, is the one L2 replaced. **G1** needed nothing.

**F7, updated.** The whole isolated shard, `enabled::` within it, now takes 567 s here at that
load, and its `enabled::` part 427 s in a run at load 14. CI's 75-minute job is far from it, and CI
itself has not run this, so no change is asked.

Warm samples, by the sampler, load 6.8 before and 7.5 after: 7,669, 5,127, 5,006, 5,102 and 5,021
ms total; engine 3,443 to 5,819 ms, broker 1,552 to 1,849 ms. All within budget, none a verdict, as
the host was not quiet.

After the mutations, each restored, and with the fix in: `brokkr-runtime` `--lib` 1,447,
`capability_launch` 76, `it` 144, `operated_repo` 1, `queued_launch` 8; the CLI's `--lib` 1,105 passed
and 1 ignored, and `--test it` outside `capability_broker::` 717 passed and 3 ignored; witness
pins unmoved, 6 passed. After the last fold of a test helper, `matrix::` and `sites::` passed again,
21 of 21. `brokkr-protocol` moved no line, so it was not re-run; its last pass is above.

### Pending, none of it this seat's

The fully cold x86_64 sample; every aarch64 sample; warm samples on a quiet host, each
observation timed alone; FD high-water and peak metadata memory; exact coverage and the
crap ratchet; the brokered real-harness check under the flag (ruling F1), which now meets F10 on
Claude as said there; macOS, which this host cannot compile; CI on the final head; and the committed
`a_bind_alias_of_protected_evidence_is_refused` on the host. This seat ran it again and it
skipped ("unshare: write failed /proc/self/uid_map: Operation not permitted"), so what stands
is the scratch variant under "U9b1a's owed items".

### Tasks

42.2 is ticked on the proofs above, with F5's seams, and SD4's timings pending as listed. 42.2
was ticked before the first review on a test that began no call; it now meets its text on this tree.
46.1 is not ticked (controller, 2026-10-31): the third review's M1 found the harness's death and
the engine's timeout proved through real dispatch at every window but `Handoff`, which only the
startup deadline ends, so the claim above that each window ends three ways stops short of
`Handoff`. By operator ruling 2026-10-31 ("B") U9b1b lands by hand with 46.1 open, and U9b1c
proves both `Handoff` cases, with their compiling removals, and ticks it. 46.1 closed there, in
U9b1c's 46.5 ([U9b1c.md](U9b1c.md)). 46.2 stays unticked: its non-host work is
done above, audit included, and by its own last sentence "missing host evidence leaves this task
pending".

## The proof the first run built

`crates/brokkr-cli/tests/enabled.rs` and its children `enabled/transport.rs`,
`enabled/matrix.rs` and `enabled/dispatch.rs` are new, registered in `tests/it.rs`.
They drive the actual binary: `brokkr run` compiles a fixture realm's MCP grant,
admits and provisions under a protected HOME, and dispatches the real Claude driver
and the shipped Claude adapter over a `claude` on PATH that starts the sealed broker
from its `--mcp-config`, speaks MCP to it over FIFOs, and then reports success
whatever happened. The broker stands the real server box and execs the installed
script by the kernel's shebang; the engine folds the ledger at settlement; the suite
reads the result through `inspect --json`. `World` serializes its own runs, as each
walks about 205,000 system entries twice inside the shared 30 s startup bound.

## Findings for the controller

**F1, ruled and fixed above; as the first run found it: production never confines a
harness, so on an ordinary host no MCP seat can complete.** MB3 admits a multiply-linked system file only where
the broker, its harness and the harness's parent are proved confined, including
`no_new_privs`. No production code sets it (a grep for it over `crates/` finds the
reader in `hands/namespace/sources/host.rs` and tests only). Ubuntu's `/usr`
holds such files, so a seat launched as an operator launches it refuses with
"MCP server box filesystem identity is not protected" before any lookup.
`enabled::an_unconfined_seat_refuses_by_identity_before_any_lookup` pins that as it
stands. Ruling 21's second case therefore completes here only with the engine run
under `setpriv --no-new-privs`, which is how this suite launches it, and it does not
yet hold for an unconfined `brokkr run`. It fails closed, so it is no security hole,
but it is the case the operator ruled must work. The fix lies outside this row's
production files: a process must set the flag before the harness starts.

Options: (a) `brokkr driver <harness>` sets `no_new_privs` on itself before spawning
a harness whose sealed serving is brokered, with the safe `rustix::thread` call;
(b) the engine wraps such a seat's spawn; (c) operators launch the engine confined,
documented by U9b2. **I recommend (a):** the driver is the harness's parent and is
already in the chain the observer reads, nothing outside brokered seats changes, no
`unsafe` is needed, and (b) needs `pre_exec` or an external `setpriv`. It needs a
ruling on the row, as it touches the driver in `brokkr-protocol`. Whether a real
Claude or Codex harness runs under the flag has not been measured.

**F2.** A run's default store, `.brokkr/state/secrets.env`, lies inside the seat's
reach, so it refuses with "MCP secret store is reachable by workspace hands", even
though U8h masks it in the hands box
(`enabled::matrix::a_store_in_the_worktree_refuses_with_no_lookup_or_start`). The
suite uses `--secrets-file` outside the worktree. It fails closed. Options: U9b2
documents `--secrets-file` as required for MCP, or the broker admits a store its box
masks. I recommend documenting it, as admitting a masked store narrows MB4.

**F3, fixed in the row.** A dialect binding `HOME` compiled, and its broker refused
with "broker plan is not bound to this attempt", so the attempt parked on "broker
ledger is missing for this attempt". As the code reads, a binding's name is removed
from its harness's environment, so that broker's HOME was gone; I did not instrument
this. Either way the harness had started, and no startup cause was given. `Slot::seal` in
`engine/broker.rs` now refuses a fixed key of the server box with
`Unprovisioned::Box(Refusal::StartupInputs)` before anything is made, so both names
give the startup cause with no lookup and no harness.

**F4.** SD4's budgets did not hold on this loaded host; see the samples below. The
same load also failed two tests in one full `--test it` pass, and both passed when
rerun: see "What was run".

**F5.** Task 42.2 names an invalid status before and after sender completion through
real dispatch. The bootstrap writes only valid tags, so real dispatch reaches an
invalid byte only by tampering with the process. It stays proved at the private seam
(`broker::session::readiness::tests`) and is not repeated here. The controller can
accept that seam, or commission a ptrace-injected variant. The same holds for MB4's
complete-frame matrix (an open sender after a complete frame, immediate EOF, a
trailing byte, a second frame, truncation): the real broker writes one well-formed
frame and closes, so only `capability_broker::handoff` and the bootstrap's own tests
reach them. I recommend accepting those seams for both, as a tampering harness would
prove the injector more than the broker.

**F6 (second visit).** A seat whose own writable reach holds its server's program
refuses by the source lease, "dispatch refused: MCP server box filesystem identity is
not protected: site … can write …", not by MB3's table entry "MCP server launch
resolves inside seat-writable reach". With the lease removed, the same seat gives
exactly that kept cause, so the box check still stands behind it. Both refuse with
zero lookups and starts, so this is a wording gap and fails closed. The suite pins
the lease's cause. Options: amend MB3's table to say the lease answers first for a
writable reach, or have the lease skip an attempt's own writers. I recommend the
amendment, since the second option adds a path to the protection.

**F7 (second visit).** The serialized `enabled` suite takes about 270 s, which pushes
`cargo test -p brokkr-cli --test it` past this seat's 590 s limit on this host (the
rest took 303 s, at load near 14), so the two parts were run apart. CI has not run
this. If it strains CI, the suite can become a shard of its own.

## Task 42.2

| Test (`enabled::transport::`) | Observed, past the harness's success | Mutation, and how it failed |
|---|---|---|
| `the_handoff_failure_tag_fails_its_attempt_past_the_harness_success` | Parks on `Refusal::Handoff`, no call row, one harness start, no server started; `inspect`'s last-seat note agrees | Disposition judgment removed (`Some(Disposition::Failed(_)) => None` in `fold.rs`): `left: ("completed", "done", None)` |
| `the_exec_failure_tag_fails_its_attempt_past_the_harness_success` | Parks on `Refusal::Exec`, as above | The same removal; and status propagation (`Status::ExecFailed => Refusal::Handoff`): `left: ("awaiting_operator", "work", Some("MCP server protocol is invalid"))` |
| `status_eof_then_a_failed_initialization_keeps_the_protocols_cause` | Parks on `Failure::Protocol` | The disposition removal: `completed` |
| `a_zero_call_version_failure_is_not_hidden_by_the_harness` | Parks on `Failure::Version` | The disposition removal: `completed` |
| `a_closed_that_cannot_persist_reads_as_the_lifecycle_refusal_and_no_invented_cause` | The first run's fixture began no call; see "The review's return" for what replaces this row's observation. Broker SIGKILLed mid-call: "broker ledger lifecycle is invalid" | `invalid` dropped from `folded`'s refusal: `left: ("completed", None)` |
| `a_restarted_engine_settles_{an_exec, a_handoff, status_eof}…` (one test, for exec, in the first run) | Engine SIGKILLed with its harness alive: reservation still stands; the broker's exit and closed disposition asserted first; `resume` parks "indeterminate: engine restarted while the attempt was in flight…", no call row, one harness start, the reservation standing after it; a second `resume` rules the same and starts nothing | The restarted judgment made `EffectFailed` in `fold::ended_restarted`: all three, `left: ("awaiting_operator", None)`, `right: (…, Some("engine restarted…"))` |

Settlement ordering is bound separately. With the `Cleanup::Settled | unproven` guard
removed from `fold_with`, `engine::broker::fold::tests::routes::a_live_half_record_is_not_read_while_any_driver_of_its_attempt_is_unresolved`
and `every_member_is_witnessed_before_any_member_is_read` fail with evidence read
early. The 0/1/4,096-call controls in `fold::tests` stay as they were. Each mutation
was applied, run and restored; `git status` showed those production files clean after.

## Task 46.1, as far as it went

| Test (`enabled::matrix::`) | Observed | Mutation, and how it failed |
|---|---|---|
| `a_multiply_linked_program_refuses_with_no_lookup_or_start` | `Refusal::Linked`, zero store reads (inotify), server never started, no call row | Link check neutered in `sources.rs`: `left: ("completed", None)` |
| `a_store_in_the_worktree_refuses_with_no_lookup_or_start` | `Refusal::StoreReachable`, zero reads, no start (F2) | Reach check dropped in `sources/store.rs`: `left: ("completed", None)` |
| `a_home_or_tmpdir_binding_refuses_the_launch_with_no_lookup_or_child` | `Refusal::StartupInputs` for each name, zero reads, zero harness starts (F3) | `fixed_key` made false: `left: "broker ledger is missing for this attempt"` |
| `a_normal_binding_and_private_directories_reach_the_server` | Server's env names are MB4's eight and `DOCS_TOKEN`; `/runtime/home`; both directories empty; token masked in the relay | TMPDIR's fixed value changed: `left: "awaiting_operator"`. It refused at box establishment, an earlier cause |
| `a_wanted_grant_is_served_beside_the_hands_with_native_power_off` | Whole set is `brokkr` and `cap-library-docs`; hands `network:false`; `--tools ""`, `--disallowedTools WebFetch,WebSearch` | `Holding::native`'s empty-restriction arm returning a native pair: `left: ["brokkr"]`, `right: ["brokkr", "cap-library-docs"]` |
| `a_fatal_protocol_failure_after_a_call_is_not_hidden_by_the_harness` | `Failure::Protocol`, one `failed` row, no digest | `rows.push` removed from `folded`: `left: []` |
| `the_call_past_the_ledgers_limit_fails_its_attempt_with_its_rows_kept` | `Failure::Limit`, exactly 4,096 rows | `CALLS_MAX` 4095: `left: 4095`. 8192 does not compile, by the ledger's own const assertion |
| `a_scalar_secret_is_refused_and_no_unsafe_body_is_retained` | `Failure::Masking`, one `failed` row, no digest, no `.response` stage, value in nothing left or heard | Two layers, so both removed together (`output::framed`'s guard and the ledger's in `ledger.rs`): `left: ("completed", None)`, `right: ("awaiting_operator", Some("MCP response cannot be safely masked"))` |
| `a_boxed_sibling_cannot_alter_the_attempts_evidence` | A boxed review striking at the first run's plan, ledger and directory: not reached, bytes unchanged | `/var/tmp` bound writable into every workspace box, unseen by any reach: panics "kept apart" at the strike |

Healthy control: `enabled::a_seat_holding_an_mcp_grant_with_boxed_hands_dispatches_through_the_real_broker_and_completes`
ends `completed`, broker exit 0, `resolved` delivered, one `succeeded` row whose
digest is the retained object's. `rows.push` removed: `left: []`.

The second visit added `enabled/reach.rs`, `enabled/sites.rs` and
`enabled/lifetime.rs`, all driven through `brokkr run`. Each mutation below was
applied, run and restored, and the suite then passed whole.

| Test | Observed | Mutation, and how it failed |
|---|---|---|
| `reach::a_writable_bind_at_above_or_below_the_package_reaches_it` | Each direction: "dispatch refused:" naming the bind, zero reads, harness starts and server starts. Replaces the first visit's `matrix::a_writer_reaching…` | The lease's `writes(…)` check made false in `sources.rs`: `left` holds "MCP server launch resolves inside seat-writable reach", the kept cause beneath it (F6) |
| `reach::a_read_only_or_overlay_bind_over_the_package_overlaps_the_box` | `Refusal::BindOverlapsReach` for `ro` and `overlay`, at, above and below; no reads, no server | Two guards. `namespace.rs`'s and `launcher.rs`'s removal left it passing. `namespace.rs`'s and the observer's `(false, true, _)` arm in `sources.rs` removed together: `left: ("completed", None)` |
| `reach::a_launch_resolved_through_a_link_in_reach_…` and `a_program_in_the_worktree_…` | The lease's cause naming the link and the program; no reads, no starts | The same lease removal: as the first row |
| `reach::a_singleton_system_entry_is_served_beside_the_systems_hard_links` | `/bin/sh`'s canonical path with its script as an argument: completes, one call, broker exit 0 | `ServerProgram::resolve` refusing `/usr`: `left` holds "program tree cannot be resolved" |
| `reach::an_empty_or_missing_store_starts_no_server` | Both: "secret 'DOCS_TOKEN' is not in the store at …", no server, no row | `store::bind_names` skipping an absent name: `left` holds "MCP server secret environment could not be delivered" |
| `reach::a_local_route_is_isolated_a_contracted_one_shared_and_the_hands_stay_off_the_network` | The server's own `/proc/$$/ns/net`: `local` differs from the host's and holds only `lo`; `contracted` is the host's; hands `network:false` in both | `binding::network` sharing `Local`: `left: true`, `right: false` with the host's interfaces listed |
| `reach::a_declared_loading_binding_reaches_the_server` | The server's names are MB4's and `PYTHONPATH` | `secret::denylisted` naming `PYTHONPATH`: FAILED |
| `reach::a_real_calls_retained_response_opens_through_inspect_masked` | `inspect --capability-call --json`: `[secret:DOCS_TOKEN]` in the response, state `succeeded`, digest of those exact bytes | Text masking removed in `output.rs`: parks "MCP response cannot be safely masked". The guard refuses the unmasked body, so this proves masking and not the digest |
| `sites::a_panel_member_is_served_beside_a_sibling_that_holds_nothing`, `a_sequence_step_is_served_and_the_next_step_runs_on`, `a_fallback_link_is_served_by_a_plan_of_its_own`, `a_cold_retry_is_served_afresh_and_keeps_only_its_own_call` | Each completes with one call row. The fallback's first adapter cannot spawn and the harness runs once on `claude-opus-5-5`; the retry mints two inventories | `Engine::slots` provisioning nothing: each parks "the site's serving candidate brokers the MCP capabilities ['library-docs'], but this attempt provisioned plans for []" |
| `sites::more_retaining_members_than_the_budget_shares_refuse_before_anything_starts` | 33 retaining members: "capability evidence budget is exhausted", no reads, no harness | The floor's check off in `share`: `left` holds the survey's deadline cause |
| `sites::a_gate_holds_by_its_class_and_named_office_only` | GP1's five rows, `requires` and `wants`, each through `brokkr run`: exact refusal with no harness; or the exact notice in `inspect` with no broker; or served | Each `gates::check` arm removed in its own run: the egress row and the writes row fail on their exact text |
| `lifetime::a_writer_arriving_beside_a_serving_broker_…`, `a_server_arriving_beside_a_running_writer_…`, `a_server_and_a_writer_started_together_…`, `a_killed_engines_reservation_refuses_a_later_writer_through_an_uncertain_settlement` | Two worktrees under one HOME, in both start orders, started together, and past SIGKILL and a restart's indeterminate settlement: the reaching side refuses naming the package and its seat never starts; where the server arrives second, zero reads and zero harness starts as well | The lease removal: each fails, the writer completing beside the server. The simultaneous case fails with both runs `completed`. That removal drops the comparison, so it says nothing about the lock's atomicity alone, which stays at the admission tests' seam |
| `lifetime::a_disjoint_writer_runs_beside_a_serving_broker_and_both_complete` | Both complete | The lease's check made true: panics waiting for the broker to serve |
| `lifetime::a_killed_engines_run_released_by_the_operator_lets_a_fresh_run_be_served` | After restart, a new run refuses naming the first; `operator release` admits a fresh run served with one call | `exclusion::leased` admitting: panics "held by the first run" |

Two proofs were tried and dropped, as neither could be shown to bind. A harness dying
with its server holding initialization ended its broker within 5 s, but that test still
passed with `lifetime.rs`'s parent check removed, with `tree::kill_group` removed as
well, and with `Attempt::kill` removed as well, so some further route ends the broker
that I did not identify. A host `LD_LIBRARY_PATH` absent from the server stayed absent
with the box's `--clearenv` and the bootstrap's filter both removed, so something
before them clears it, which I also did not identify. Both remain owed.

Owed by 46.1 when the first run stopped, all now proved in the second run's section: a user-installed package whose program is a native binary beside the
singleton (the fixture's is a shebang script); cancellation at every startup window
through dispatch, where `capability_broker`'s `a_cancelled_attempt_ends_a_box_held_at_its_entry`,
`a_store_read_held_when_the_attempt_is_cancelled_is_refused_at_once`,
`an_attempt_cancelled_during_initialization_or_a_call_settles_the_server_and_its_child`
and `a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running` hold
them at the public broker today; a server's private tmpfs against a second server,
through dispatch; the host loader environment above; the bind-alias row through
dispatch; and quota beyond the door (spent shares) through dispatch.

## Ruling 21

The second case is the healthy control above, under F1's condition. The first and
fourth now go through real dispatch:

| Test (`enabled::dispatch::`) | Observed | Mutation, and how it failed |
|---|---|---|
| `a_fresh_scaffolds_first_seat_dispatches_with_nothing_at_stake` | `brokkr init .` then `brokkr run`: harness started, no "dispatch refused" | `staked` early return removed from `exclusion::compared`: fails |
| `each_shipped_recipes_first_seat_dispatches_with_nothing_at_stake` | All 19 shipped recipes, unconfined, under a namespace realm: no "dispatch refused", each started a harness or moved past its start | The same removal: fails |
| `a_seat_without_hands_holding_a_native_grant_dispatches_and_retains_nothing` | `claude-native-fetch` on a hands-less seat, hands-less review: completes, no call row, no broker in its config | The same removal: fails |

The third case stands by MB2's compile refusal, as ruled.

## U9b1a's owed items

Ruling 4's wiring is now bound. `observation::observed_under` takes the setting;
`dispatch_hands_provisioning_a_host_only_where_the_setting_reads_off` pins `Ok` for
`0\n` and `Unprovisioned::Terminal` for `1\n`, not-found and permission-denied, with
admission keeping its HOME. The controller's stand-in `.or(Ok(b"0".to_vec()))`:
`left: None`, `right: Some(Terminal)`, on this host's own setting of 0.

| Named test | Mutation | Failure |
|---|---|---|
| `past_the_lifted_fence_an_mcp_holding_is_the_brokers_and_its_native_namesake_stays_off` | The empty-restriction arm of `Holding::native` returning a native pair | FAILED |
| `past_the_lifted_fence_a_nonempty_mcp_restriction_refuses_or_drops_with_the_deferred_cause` | `false => Ok(None)` | FAILED |
| `a_host_whose_legacy_tiocsti_is_not_switched_off_serves_no_mcp_server` | `Ok(_) => Ok(self)` | `left` holds `Ok(Host…)` for `1\n` |
| `the_terminal_refusal_reaches_only_a_brokered_attempt_and_before_any_survey` | `host?` moved above `slots.is_empty()` | Panics at its first `assert!`, line 862 |
| `an_inline_site_is_served_with_its_own_adapters_authentication_names` | `auth: &[]` in `inline_capabilities` | `left` holds the fence's words, not the collision |
| `either_fence_refuses_unboxed_writers_at_stake_and_root_reaching_boxes_on_every_run` | `if false && !staked(comparison)` | FAILED |

L1: `Unknown::Homeless` is the case's own cause, in `records_kept`.
`a_boxed_writer_is_refused_by_its_own_cause_where_the_host_names_no_home` binds it
through `Engine::admit` with `observed.home` None, its text pinned; with the old
`Unmounted`, that test and `an_unopened_roots_path_keeps_boxed_writers_on_every_host`
fail. `Unadmitted::Evidence` took `#[from]` to pay for the variant's lines.

L3: `exclusion::identity` is the one metadata-to-(dev, ino) reading.

L2 is recorded, from a scratch variant. As committed,
`a_bind_alias_of_protected_evidence_is_refused` skips in this seat ("unshare: write
failed /proc/self/uid_map: Operation not permitted"), and under `bwrap --cap-add ALL`
its `mount --bind` is denied. So for the record only, and not committed, bwrap itself
made the bind (`--unshare-user --dev-bind / / --bind <kept>/.forge <alias>`) and the
assertions before the bind and the `mount` step were left out. The namespace half then
ran and passed. Removing `covers_spelled`'s `above` arm panicked at "by the bind's
identity"; removing `covers`'s `identical` panicked at "from below". The committed
test's own namespace run on the host stays pending.

The stale comments named were checked: `Unbound::Mcp` and the MB4 comment in
`capabilities/tests/binding.rs` already say where the fence stands, so neither moved.

## SD4, x86_64 warm samples

Sources: 205,082 entries across 2 mounts, unpruned, ordinary hard links intact,
sealed and observed writer uid 1000, seen on the diagnostic run; ACL facts were left
to the observer and not recorded. Taken with `sd4_warm_samples`, under a load average near
17. "Engine" is `brokkr run`'s start to the harness's start: compile, admission and
the provisioning survey. "Broker" is the harness's start to the broker's first
answer: reobservation, box, handoff and initialization. Neither is a pure observation.

| Sample | Engine ms | Broker ms | Total ms |
|---|---:|---:|---:|
| 1 | 5,089 | 1,995 | 7,084 |
| 2 | 5,066 | 2,092 | 7,158 |
| 3 | 3,885 | 2,222 | 6,108 |
| 4 | 6,376 | 15,922 | 22,299 |
| 5 | 16,483 | 2,142 | 18,625 |

Samples 1 to 3 are within the budgets. Sample 4's total exceeds the 20 s total, and
samples 4 and 5 each hold a stage over 10 s, on a contended host. That does not prove
a defect or a pass: the samples need retaking on a quiet host, each observation timed
alone. One run during mutation testing, with a parallel build, also parked on
"admission did not complete before its startup deadline".

The second visit's five, the same way, under a load average near 7:

| Sample | Engine ms | Broker ms | Total ms |
|---|---:|---:|---:|
| 1 | 3,541 | 1,632 | 5,174 |
| 2 | 3,493 | 1,598 | 5,092 |
| 3 | 3,468 | 1,646 | 5,114 |
| 4 | 3,474 | 1,603 | 5,078 |
| 5 | 3,567 | 1,651 | 5,219 |

All five are within the budgets, so the first set's outliers track contention. These
are still whole stages, not each observation timed alone, and the host was not quiet.

Pending, not taken here: the fully cold x86_64 sample (operator drops the caches, the
controller samples, ruling 3); every aarch64 sample; each complete observation timed
apart from compile, box and handoff; FD high-water; peak metadata memory.

## What was run

Formatting and clippy with `-D warnings` over the workspace were clean, as were
`quality/ratchet.sh files` and `clones` (two clones folded, by sharing
`writer_lease::POLICY` and adding `World::completes`). `cargo test -p brokkr-runtime
--locked`: `--lib` 1447 passed, `--test it` 144 passed. `cargo test -p brokkr-cli
--locked --test it` once: 844 passed, 2 failed, 4 ignored, in 483 s. The two were
`enabled::matrix::a_store_in_the_worktree…` and the existing
`capability_broker::serving::the_brokers_standard_streams_keep_their_status_flags_whether_served_or_failed`
("filesystem identity is not protected"), and both passed when rerun together. A
second full pass on the final tree had none failed: 846 passed, 4 ignored, in 310 s.
This suite adds survey load to that shared pool, so the second failure may be this
visit's doing; treat it as an open risk, as a loaded CI could see it too. Typos and
strict openspec validation (36 of 36) were clean, and `recipes/self` compiled with
"48 valuations over 11 groups, 0 unruled".

Not run by this seat: exact coverage and the crap ratchet (the controller's, and
CI's); macOS (`enabled.rs` is `cfg(target_os = "linux")`, and this host cannot compile
Darwin); the CLI's `--lib`, the protocol's suite and the heap suites, as the production
files of those crates are unchanged after the mutations were restored.

Production files changed: `engine/broker.rs` (F3), and its children `observation.rs`,
`exclusion.rs`, `sources.rs`, `admission.rs`. Test files beyond the new ones gained
only `pub(crate)` on shared helpers: `capability_broker.rs`, its `readiness.rs` and
`serving.rs`, `writer_lease.rs`. No test moved. `quality/file-lines.txt` gains the
four new files and the changed counts.

The second visit's runs, on the tree it commits: `cargo test -p brokkr-cli --locked
--test it -- --skip enabled::` 825 passed, 3 ignored, in 303 s; `enabled::` 41 passed,
1 ignored (the sampler), in 271 s; one whole pass together hit the 560 s bound (F7).
No production file moved from part one, so the runtime suites were not repeated, and
the protocol's and the CLI's `--lib` remain unrun, as before. No test moved.
`enabled/matrix::a_writer_reaching_the_servers_package_starts_nothing` was replaced by
`reach::a_writable_bind…`, which covers its case among the three directions.

## Owed, and the ruling this needs

Done: this suite; 42.2's real faults, its restart removal, and its settlement and
disposition removals; ruling 21's first and fourth cases, and the second under F1;
U9b1a's named mutations, L1, L2 (scratch variant), L3 and the terminal seam; F3's fix;
GP1's table, the sites, reach, stores, networks and retention through dispatch; MB3's
two-worktree matrix; ten warm samples.

The first run ended there, every item above then owed on F1, F5 and F6. The second run's
section at the top of this file carries them.

## Controller ruling, 2026-10-31

The second run (0065-slice-two-unit-u9b1b-see-th-9d7328d4) parked at
REVIEW-REFORGE-EXHAUSTED-MEDIUM after its third review. The operator ruled "B": U9b1b lands by
hand under the by-hand label, and the run was stopped with that ruling. Review 3's dispositions:

- M1 (medium): 46.1 is unticked and its `Handoff` remainder, the harness's death and the
  engine's timeout through real dispatch, moves to U9b1c, which ticks 46.1.
- L3: 42.2 and 46.1 now link this file from `tasks.md`.
- L1, L2, L4, L6 and L7 carry to U10a's audit, each with the reason recorded above or in the
  review: L1 and L6 need `capability_record.rs`, outside this row; L4 crosses crates and the
  public API; L2 orders a refusal's cause, not whether it refuses; L7 needs the spec-order
  amendment F12 recommends.
- L5 (low, security, predates this change): a measured shape's `measured_on.version` is not
  compared with the installed harness's version. It stays a residual, and U9b2's guides say so.

The real Claude and Codex launches under F1 ran on the controller's host on 2026-10-31 and
found two defects that the deterministic harness cannot see; U9b1c fixes them. SD4's aarch64
samples (debug and release) were taken that day on a temporary Linux aarch64 host; the fully
cold x86_64 sample is the operator's. 46.2 stays open on those records until they are filed.
