# U6f1b evidence: lift the incomplete-serving refusal

Run `0065-slice-two-unit-u6f1b-see-th-d416b3f5` built U6f1b on branch
`s2/U6f1b`, cut from main at `88078da5`, on Linux 6.17.0-41-generic x86_64.
The worktree's filesystem is ext4 on the host's LVM root. Run 1 made three
implement visits; its third review held on one HIGH and four LOWs, and
run `0065-slice-two-unit-u6f1b-see-th-c1c48294` (the second run) repairs
them on run 1's work, re-applied to current main, as one signed commit.
The sections below are in visit order, the second run's last. Each
section's counts and descriptions are that visit's own: the first visit's
`Option`-shaped handoff was replaced in the second, and the final head's
gates are those under "Second run". The unit contributes to tasks 31.1 and
31.2, which it shares with U6f1a. Neither is ticked here, because evidence
those tasks require is still pending (see "Pending"). `evidence.md` is not
edited.

## What changed

The public `brokkr broker serve` now serves. Its handler no longer ends
with `Refusal::ServingIncomplete`. The session starts the box, hands over
the bindings and initializes the server, all as before. It then serves the
harness on the broker's own standard input and output through the existing
filtered, masked call path (`rpc/calls.rs` `served`), and it settles a clean
ending as Clean. A plan with `retained: true` is refused with SD3's
retention cause, `broker retained-response support is incomplete`, so a
retained plan is never served without retention. In run 1 that refusal
came once the server had initialized; the second run moves it before any
MCP message is exchanged with the server (L4, "Second run" below), as SD3's
scenario asks. The run 1 sections below describe run 1's head as it was.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker.rs` (115 → 113) | The Serve arm calls `session::serve` and exits successfully. The module and `run` documentation say what serving now does. |
| `crates/brokkr-protocol/src/broker.rs` (324 → 325) | `Refusal::ServingIncomplete` is replaced by `Refusal::RetentionIncomplete`, "broker retained-response support is incomplete", in the same last place of MB3's order. |
| `crates/brokkr-cli/src/broker/rpc.rs` (650 → 666) | `initialized` is replaced by `serving`. It initializes the server (`started`), refuses a retained plan, holds the broker's stdin and stdout as owned duplicates (one it cannot hold is the protocol cause, as `served_within` already treats a harness stdio it cannot make non-blocking), and calls `calls::served` with an `Edge` of the bound bindings, the server's own stderr and the broker's stderr. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (755 → 752) | The ruled `#[cfg_attr(not(test), expect(dead_code, …))]` on `served` is removed. `served_within` no longer calls `Ledger::bind`, because the session's binder bound the ledger once already. |
| `crates/brokkr-cli/src/broker/session.rs` (774 → 775) | Re-exports `serve` and `Bound` in place of `admit` and `Admitted`, and updates the documentation. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (178 → 192), existing child | `admit` becomes `serve` and returns once the harness is served. `Binder::bind` returns a `Bound` (the ledger and the kept bindings), which `ready` hands to the serving closure. `settled` latches no cause for `Ok`, so a session served until its harness ended closes Clean unless something was latched. |
| `crates/brokkr-cli/src/broker/session/readiness.rs` (533 → 567), existing child | `ready` passes `bind`'s answer and the server's stderr pipe to `served`. `delivered` closes L1 (below). `handed` returns what the sender published beside its answer. The new `Published` is the lock-held slot. |
| `crates/brokkr-protocol/src/hands/namespace/entry.rs` (504 → 508) | The launch gives the launcher, and through it the bootstrap and the server, a standard error pipe in place of `Stdio::null()`. `Launched` gains `errors`, its read end. |
| `crates/brokkr-protocol/src/secret/handoff.rs` (340 → 337), child of the row's `secret.rs` | `send_environment` closes L1 (below). `framed` now only makes the frame. |
| `crates/brokkr-protocol/src/broker/ledger.rs` (608 → 616) | `Ledger::bind` binds once (L2, below). |

No production file is new, no production code moved, and `secret.rs` itself
is unchanged: its re-export already names `send_environment`.
`Launched` is reachable only through `ServerBox::launch`'s return type, so
its new field adds no public API entry. The brokkr-protocol public API stays
at 1,116 lines (`quality/ratchet.sh api`: "public API holds"). Two lines of
the snapshot changed. `Refusal::ServingIncomplete` became
`Refusal::RetentionIncomplete`, which the operator approved on 2026-10-12.
`send_environment`'s signature changed shape, which the ruling of 2026-10-13
allows. Production `expect(dead_code)` goes from 2 to 1 in
`quality/suppressions.txt` (`cargo test -p brokkr-cli --test it
suppressions::`: 6 passed).

## L1: resolved bindings are published before the write can stall

U6f1a's seventh run claimed L1 complete. That claim is superseded here. It
gave the bindings to the binder "as soon as the sender answers". A sender
whose control-pipe write stalls never answers before the startup gives up,
and `handed` then returned only `Err(Handoff)`. The bindings the sender had
already resolved were dropped, and the ledger's Closed was appended guarded
by none.

The fix has two parts. `secret::send_environment(store, path, names, found,
send)` resolves the bindings, makes the frame, gives the bindings to `found`,
and only then asks `send` to write. It gives them whether or not the frame
could be made, and none where the lookup itself refused. In
`readiness::delivered`, the sender thread publishes them into a `Published`
slot behind a mutex. `handed` takes that slot on every path: an answer in
time, a sender that failed (its join erred), and a give-up at the deadline or
on cancellation. On a give-up it stores `given_up` before it takes the slot.
A sender that publishes after the take therefore sees `given_up` under the
mutex's ordering and writes nothing, so no value reaches the pipe unbound.
`delivered` then always calls `bind` with what was taken, before the send's
answer is read. Shutdown stays non-waiting: the supervisor takes the lock
only to swap an `Option`, and the sender is never joined after a give-up.
The doc comment on `readiness::delivered` now says this.

The review of this first visit found the fix incomplete (its L1). A give-up
that came after the sender had resolved its bindings and before it
published them took an empty slot, bound an empty guard, and appended the
handoff Closed unguarded; the later publication was never taken. The second
visit closes that window; see "Second visit" below.

## L2: `Ledger::bind` binds once

`Ledger` has a private `bound` flag. It is set at opening where the opening
named bindings, and on the first `bind`. A second `bind` returns the
existing `Unrecorded::Record` and keeps the first guard. The production path
binds exactly once, through `Binder::bind`. `served_within` no longer
rebinds, which `machine_proof::output_edge` now pins.
`Unrecorded::Record` was chosen over `Reopened` because `Reopened` means a
second lifetime. As for any `Record` the ledger refuses before an append,
the closure is left to what is latched (`Ended::failure`). The call path
never makes such a record.

## Server stderr

The server's stderr is now its own pipe from the launch. `ready` hands it to
the serving closure, which drains it with U6f1a's `session/stderr.rs` drain
from the server's initialization on (second visit, M2), and writes the
masked tail to the broker's stderr, without blocking, however the session
ends (second visit, M2 and M3). Through the public binary, a server printing
`server says $DOCS_TOKEN é` produces exactly `server says
[secret:DOCS_TOKEN] é\n` on the broker's stderr. The two-byte character
survives whole, and the value appears nowhere in stdout, stderr or the
ledger file.

## Tests added or changed, with their removals

Every new test below asserts exact values. Each removal compiled, made the
named test fail as shown, and was restored. The restored tree then passed:
`--test it` 607 passed and 3 ignored; the brokkr-cli lib 761 passed and 1
ignored; the brokkr-protocol lib 909 passed and 3 ignored.

| Test | Removal | Failure observed |
| --- | --- | --- |
| `capability_broker::protocol::a_nonretaining_plan_serves_its_harness_masked_and_closes_clean` (new) | `.stderr(stderr)` in `entry.rs` back to `Stdio::null()` | stderr left `""`, right `"server says [secret:DOCS_TOKEN] é\n"` |
| same | the edge given `bindings: &[]` in `rpc::serving` (masking removed at the public boundary) | stderr left `"server says u6c8-canary-Qx7Zv é\n"` |
| same | `settled` latches `Failure::Protocol` after every session | ledger left `Some(Failed(Protocol))`, right `Some(Clean)`; `broker::rpc` lib tests 16 failed |
| same, and `machine_proof::output_edge::the_servers_own_standard_error_reaches_the_drain_alone_and_the_ledger_is_bound_once` (new) | `served_within` calls `ledger.bind(edge.bindings)?` again | public left `(Some(1), [], "", "error: broker call could not be recorded\n")`; machine proof found the extra `calls.rs` call |
| `capability_broker::protocol::a_retained_plan_is_initialized_and_refused_before_the_harness_hears_anything` (replaces `…serving_still_refuses_before_the_harness`) and `capability_broker::readiness::a_real_box_reports_ready_and_its_retained_plan_refuses_with_nothing_of_it_left` (renamed) | the `ensure(!plan.retained, …)` dropped | both served: exit 0, and the retained harness received the greeting and listing on stdout |
| `capability_broker::handoff::the_server_inherits_only_its_stdio_and_its_environment_from_the_private_home` (descriptor pin updated) | same stderr removal as above | left `[(0,"pipe"),(1,"pipe"),(2,"/dev/null")]` |
| `broker::session::readiness::tests::bindings_looked_up_before_a_stalled_write_still_guard_the_closure_given_up_on` (new; DOCS_TOKEN `0002` and `closed`; deadline and cancellation; unrelated control) | `handed`'s give-up returns `None` | `("0002","deadline", (Err(Refused(Handoff)), ([], Some(Failed(Refused(Handoff))), None)))` against `(Err(Failed(Failed(Masking))), ([], None, Some(Lifecycle)))` |
| same, and `secret::tests::the_sender_frames_the_injectors_environment_of_the_declared_names_alone` and `…refuses_a_frame_past_mb4s_bounds…` (updated) | `found(bindings)` moved after `send` | readiness: the cancellation case lost the bindings, as above; secret: `Some(false)` for "given before the write", and `None` given for a refused frame |
| `broker::session::readiness::tests::what_a_sender_published_before_it_failed_or_was_given_up_on_is_taken` (new) | give-up returns `None` | left `(None, Err(Handoff))`, right `(Some(7), Err(Handoff))` |
| same | a failed join returns `None` | the panicking sender's case: left `(None, Err(Handoff))` |
| `broker::ledger::tests::a_ledger_is_bound_once_and_a_second_binding_never_weakens_its_guard` (new) | the `ensure(!self.bound, …)` dropped | left `Ok(())`, right `Err(Record)` |
| same | the second bind refused, but the guard replaced first | the close: left `Ok(Clean)`, right `Err(Masking)` |
| `capability_broker::serving::two_servers_and_a_replacement_keep_their_private_directories_apart` (new) | the server box binds one host directory over `/tmp`, and the bootstrap's `decide` admits anything (the bootstrap's private-directory check alone already refused it with `Establishment`) | `beside` saw `/tmp: u6f1b-mark`, the first server's marker |
| `capability_broker::serving::only_a_shared_network_reaches_the_hosts_loopback_and_abstract_sockets` (new) | `--unshare-net` never added, and the bootstrap's `separate` accepts the host's network for an isolated plan (the first alone already refused with `Establishment`) | `isolated`: `loopback=reached`, `abstract=reached`, `(1, 1)` accepted |
| `capability_broker::serving::the_server_reads_its_installed_bytes_as_they_are_and_admission_vouches_for_none` (new) | the drain strips `aws_secret_access_key=` (a generic redaction this suite must not claim) | left `u6f1b-installation-canary\n…`, right `aws_secret_access_key=u6f1b-installation-canary\n…` |
| `capability_broker::binding_names_are_checked_then_fixed_keys_refuse_before_lookup` (repeated, unchanged) | the fixed-key screen skips `HOME`; separately, it skips `TMPDIR` | `("HOME", "…filesystem identity is not protected", 0)` and `("TMPDIR", …, 0)` against `startup inputs are not protected from seat writes`, zero lookups either way |
| `hands::tests::server::a_process_is_read_once_it_has_settled` (new, the flake fix's control) | `settled` returns `descriptors(pid)` at once | left `[…, (3, "/dev/null")]`, right the three standard descriptors |
| `hands::tests::sources::the_box_mounts_the_observed_object_and_carries_its_facts` (flake fix) | `retried` makes one attempt | left `(205084, 2, "3060ca2e…")`, right `(205083, 2, "af426975…")`: one record more, the CI flake's shape |

`machine_proof::egress::the_binding_frame_has_one_sender_and_one_receiver`
now names `delivered<B, E: From<Refusal>>`. Its own sender proof is
unchanged. The `rpc` unit tests' harness now binds the ledger before the
server starts, as the binder does, and their `admitted` closure is
`Clean`. `capability_broker::ledger::a_bound_plan_refused_closes_failed…`
expects the retained fixture's `RetentionIncomplete`. `admitted()` in
`capability_broker.rs` is now the retention cause, because the shared
fixture's plan retains (`"retained": true`), so every test built on it
still ends where its server has initialized.

## Second visit: the review's M1–M4 and L1

The review of `bdfce90b` returned four MEDIUM findings and one LOW security
residual. This visit answers each. It is a second signed commit on the same
branch.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker/rpc.rs` (666 → 718) | M1: `initialized` (new, private) serves under `(now + SERVING, the startup's cancellation)`. `SERVING` is a century, standing for no deadline of the broker's own; the startup's 30 s bounds only the startup, and each call keeps MB3's 120 s. M2: `serving` makes the drain before anything else and `started`, `listed` and `exchanged` take it, draining one line every poll turn of every startup wait. M3: `diagnosed` writes the tail on every ending, the broker's stderr made non-blocking for the write and its own flags restored after. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (752 → 745) | `served_within` takes the drain from the `Edge` and no longer makes it or writes its tail. |
| `crates/brokkr-cli/src/broker/output.rs` (395, unchanged count), existing child | `Edge` holds the bindings and a `&mut Drain`; its `diagnostics` field is gone. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (141 → 171), existing child | `Drain::told` writes the masked tail within a fixed 1 s (`TOLD`), resuming on partial writes, waiting where a write would block, and giving up at the bound or on cancellation. |
| `crates/brokkr-cli/src/broker/session/readiness.rs` (567 → 622), existing child | L1: `Handed<P>` (`Nothing`, `Published`, `Uncertain`). `Published` is a lock-held `Slot` (`Unbegun`, `Begun`, `Published`, `Taken`); the sender marks its lookup begun before it reads the store, and may not begin once its supervisor has taken the slot. `handed` takes the slot once: a give-up, or a failed join, with the lookup begun and nothing published is `Uncertain`. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (192 → 213), existing child | L1: `Binder::bind` takes `Handed`. `Uncertain` sets the lifetime's `withheld` flag and ends the session with the handoff cause, and `lived` then neither latches nor closes: recovery finds the Opened alone (`Invalid::Lifecycle`), never a success. Shutdown still waits for nothing. |

No production file is new. brokkr-protocol is not changed in this visit, so
its public API count stays at 1,116.

L1 is closed by withholding, not by waiting. When a sender is given up on
while its lookup is under way, the supervisor cannot know what it holds, so
no guard can be built. Every later record, the closure included, is
withheld rather than written guarded by nothing. A sender given up on
before it began holds nothing, so its session still closes Failed(Handoff).
A sender that returned without publishing had its lookup refused, which
resolves nothing, and its session also closes. This visit's evidence
first presented the withheld closure as a rule bent knowingly, against a
controller's note. That was wrong, and the second run corrects it: after
this unit's first run the operator ruled the outcome, in the addendum of
`operator-ruling-2026-10-03.md` dated 2026-10-13, whose words are:

> when the hand-off's outcome is uncertain, because startup gave up on a deadline or a cancellation after the store read but before the bindings were published, the broker cannot know the bindings and must not wait for them. The ledger then keeps only Opened, recovery reports the lifecycle incomplete, and no closure is appended unguarded. That is the ruled outcome, not a residual: nothing unguarded is written, and shutdown never waits.

Recovery reports the lifetime unended (`Invalid::Lifecycle`), never Clean.
The third visit pinned this outcome through the public deadline and
cancellation tests, and the second run binds them exactly again (see
"Second run" below).

The tests below were all added or changed in this visit, and each asserts
exact values. Each removal compiled, made its test fail as shown, and was
then restored.

| Test | Removal | Failure observed |
| --- | --- | --- |
| `capability_broker::serving::a_served_session_keeps_no_startup_deadline_and_answers_a_call_made_past_it` (new; the call is made 31 s after spawn) | `initialized`'s attempt bound by `startup.0.min(…)` | left `(Some(1), 2, <the listing>, "error: MCP server call timed out\n")`, right `(Some(0), 3, <the call's answer>, "")` |
| `capability_broker::serving::a_server_that_floods_its_standard_error_before_it_answers_is_drained_and_its_tail_kept` (new; 4,096 lines of 64 bytes before initialize; nonretaining and retained) | `exchanged` never drains (`false && stderr.drained()`) | left `(Some(1), 0, "…004094\n…004095\nerror: MCP server call timed out\n")` after the 30 s startup |
| same | `diagnosed` only where the session ended `Ok` | retained: left `(Some(1), 0, "error: broker retained-response support is incomplete\n")`, right the 1,024-line tail before it |
| `capability_broker::protocol::a_running_server_lacking_a_dependency_or_speaking_no_mcp_is_the_protocols_cause` (expectation changed: the shell's own line now precedes the cause) | same | left `"error: MCP server protocol is invalid\n"`, right `"<server>: 2: /opt/docs/lib/helper: not found\nerror: …"` |
| `capability_broker::serving::a_standard_error_nobody_reads_never_holds_the_sessions_end` (new; the broker's stderr a pipe the test filled and never reads; a watchdog kills the harness shell and its broker after 20 s) | `diagnosed` sets the flags without `NONBLOCK` | left `(None, 3, true)`, right `(Some(0), 3, true)`: the broker hung in its write and was killed |
| `capability_broker::serving::only_a_shared_network_reaches_an_external_endpoint_the_hosts_loopback_and_abstract_sockets` (renamed and extended) | isolated given the host network in `namespace.rs`, and the bootstrap's `separate` accepting it | `isolated`: left `external=reached`, `loopback=reached`, `abstract=reached`, `(1, 1, 1)` accepted. The namespace change alone, either way round, is refused first with "MCP server box could not be established" |
| `capability_broker::serving::a_box_resolves_names_from_its_generated_files_and_a_shared_one_through_the_checked_resolver_alone` (new) | `Resolver::Dns` binds no `resolv.conf` | `shared`: listing without `resolv.conf`, and `resolv=absent` |
| same | `Resolver::Dns`'s nsswitch `hosts: files` | `shared`: left `hosts: files` |
| `broker::session::readiness::tests::a_sender_given_up_on_between_its_lookup_and_its_publication_leaves_the_ledger_unclosed` (new; the sender begins, is held past a 200 ms deadline, then publishes) | `taken` maps `(Begun, false)` to `Nothing` | left `(Err(Refused(Handoff)), ([], Some(Failed(Refused(Handoff))), None))`, right `(…, ([], None, Some(Lifecycle)))` |
| same | `Binder::bind` binds `Uncertain` as none | the same left |
| same | `lived` settles when withheld | the same left |
| `broker::session::readiness::tests::a_lookup_begun_and_never_published_is_uncertain_unless_its_sender_returned` (new) | `(Begun, false)` to `Nothing` | the failed sender: left `(Nothing, Err(Handoff))`, right `(Uncertain, …)` |
| same | `begin` allowed after the take | the late sender: left `Ok(true)`, right `Ok(false)` |
| same | `returned` always false | the refused sender: left `(Uncertain, …)`, right `(Nothing, …)` |
| `machine_proof::output_edge::the_servers_own_standard_error_reaches_the_drain_alone_and_the_ledger_is_bound_once` (updated) | none new: its first-visit pin of `Drain::new` in `served_within` failed against the moved code | now pins `Drain::new` and `stderr.told(` to `rpc.rs`'s `serving` and `diagnosed` |

The existing readiness tests take `Handed` in place of `Option`, with the
same cases. The rpc unit harness makes the drain before `started`, gives it
through the `Edge`, and writes the tail with `told`, as `serving` does. The
CLI suite's `Talk` now reads the broker's stderr from spawn. Without that, a
failing case whose 64 KiB tail filled the test's stderr pipe deadlocked:
the broker blocked on its last error line, and the test waited for its
stdout to end. The flood removal first hung this way, before the harness
was fixed. `talking_beside` gives the broker a stderr of the test's own.
`Talk::asked` speaks after a wait. `serving.rs`'s `closed_clean` and
`readiness/tests.rs`'s `proved` fold two clones that `quality/ratchet.sh
clones` found.

This visit's gates, on the final tree: `cargo fmt --all -- --check` and
workspace clippy with `-D warnings` were clean. The brokkr-cli lib passed
763 with 1 ignored. `--test it` passed 85 `capability_broker::` tests and
526 others, with 3 ignored. `driver_conformance`, `transcript_surfaces` and
the three heap binaries passed 27, 13, 1, 1 and 1. brokkr-protocol and
every other crate are unchanged in this visit, and their suites were not
rerun. `witness_digests::` passed unblessed (6). `recipes/self` and
`recipes/verify` compiled. `openspec validate --all --strict` passed 19 of
19, `typos --hidden` was silent and `git diff --check` was clean. In
`quality/ratchet.sh`, `files` and `clones` held after the rows above. The
macOS pipe check was not repeated: the flood test writes past both pipe
sizes, and nothing new depends on a pipe's size.

## Third visit: the review's M1–M3, L1 and L2

The chief review of `6ede4ec6` returned three MEDIUM and two LOW findings.
This visit answers each in a third signed commit. brokkr-protocol is not
changed, so its public API stays at 1,116 entries.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker.rs` (113 → 116) | M1: the Serve arm no longer hands a failed session's error to the common reporter, whose write blocks on a full standard error. It reports it through `session::stderr::reported` and returns that exit code. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (171 → 205), existing child | M1: `told` (the tail's bounded write, moved out of `Drain::told`), `unheld` (the broker's stderr made non-blocking for one write and its mode restored, moved out of `rpc::diagnosed`), and `reported`, which renders the line with the crate's own `report_to`, so the text and exit code are unchanged, and writes it with `told` under `unheld`. |
| `crates/brokkr-cli/src/broker/rpc.rs` (718 → 711) | `diagnosed` calls `stderr::unheld`. |

Every test this visit adds or extends pins exact values, and each removal
listed against it compiled, failed that test as recorded, and was reverted.

| Test | What was removed | Its failure |
| --- | --- | --- |
| `capability_broker::serving::a_standard_error_nobody_reads_never_holds_the_sessions_end` (extended: clean, retained, protocol and unbound, each against a full unread stderr pipe, with exact exit, frames, unchanged pipe bytes and ledger) | the Serve arm returns the error to `lib.rs`'s reporter (`return Err(ended.into())`) | `("retained", (None, 0, true))` against `("retained", (Some(1), 0, true))`: killed by the 20 s watchdog |
| same | the same, for `Ended::Failed` alone | `("protocol", (None, 0, true))` against `(Some(1), 0, true)` |
| same | the same, for `Refused(Unbound)` alone | the unbound case: `(None, 0, true)` against `(Some(1), 0, true)` |
| `capability_broker::serving::a_box_resolves_names_from_its_generated_files_and_a_shared_one_through_the_checked_resolver_alone` (extended: `_localdnsstub` asked of the box's resolver) | `Resolver::Dns` binds no `resolv.conf` (`namespace.rs`) | `shared`: left ends `stub=unresolved`, right `stub=127.0.0.53` |
| same | `Resolver::Dns`'s nsswitch line `hosts: files` | `shared`: left ends `stub=unresolved`, with the host's `resolv.conf` still bound |
| `capability_broker::serving::a_dedicated_native_server_in_a_package_is_served_beside_the_systems_own_hard_links` (new) | `sources.rs`'s program-tree link refusal applied to every multiply-linked file (`& (program \| true)`) | left `(Some(1), 0, "error: MCP server box program tree contains a multiply-linked file\n")`, right `(Some(0), 3, "<MB4's names and DOCS_TOKEN> \n2 3410637\n")` |
| `capability_broker::handoff::a_store_read_held_past_the_deadline_is_refused_at_the_deadline` and `…_when_the_attempt_is_cancelled_is_refused_at_once` (L2: `refused_handoff` now asserts the ledger) | `taken` maps `(Begun, false)` to `Nothing` | both: left `([], Some(Failed(Refused(Handoff))), None)`, right `([], None, Some(Lifecycle))` |

The native control's server is a singly linked copy of the host's
`/bin/sh` ELF, its loader and libraries the bound system set's, run on a
script in the package as its argument. A native MCP server needs some
program to speak the protocol, and this is the reading taken. The multiply
linked system file is the first root-owned, group- and other-unwritable
regular file with more than one link that the test finds directly under
`/usr/bin`, `/usr/sbin` or `/usr/lib`. On this host it has 2 links, at
inode 3410637, and the server reads the same link count and inode inside
the box. A host with no such file skips. The DNS proof depends on the
host's `resolv.conf` naming systemd-resolved's stub, whose synthetic
`_localdnsstub` record has a fixed answer. A test cannot run a DNS server of
its own on port 53 without privilege, and the box binds the host's
`resolv.conf` in place. A host whose resolver is not that stub skips.

`protocol.rs`'s `served` return type is folded into a `Served` alias.
Workspace clippy, run on this tree, refused it under `type_complexity`.

The gates on the final tree were all run in this visit.
`cargo fmt --all -- --check` and workspace clippy with `-D warnings` were
clean. In brokkr-cli, the lib passed 763 with 1 ignored. `--test it`
passed 86 `capability_broker::` tests and 526 others, with 3 ignored.
`driver_conformance` passed 27, `transcript_surfaces` 13, and the three
heap binaries 1 each. brokkr-protocol and every other crate are unchanged
in this visit, and their suites were not rerun. `witness_digests::` passed
unblessed (6). `recipes/self` and `recipes/verify` compiled. `openspec
validate --all --strict` passed 19 of 19. `typos --hidden` was silent and
`git diff --check` was clean. In `quality/ratchet.sh`, `files` and `clones`
held after the six rows this visit moved.

## The two flakes

`a_server_box_stands_without_the_seats_or_the_hosts_private_paths` now reads
the waiting child's descriptors through `settled`. Within ten seconds,
`settled` waits until `/proc/<pid>/stat` shows the child asleep (`S`) and two
reads of its descriptors 50 ms apart agree. Its control,
`a_process_is_read_once_it_has_settled`, holds a fourth descriptor while it
runs a busy loop and then sleeps.

`the_box_mounts_the_observed_object_and_carries_its_facts` takes each fact
as a pair of observations across the fixture's own change alone. Its
`retried` helper makes at most five attempts, observing again where the
whole system set moved in between. Each assertion keeps its meaning,
relative to the observation just before the change: unchanged is equal, a
rewrite keeps the entries and changes the digest, and a new directory and
file add exactly 2 entries. The first pair's set is made to move by one
record (`opt/docs/churn`), so every run exercises the retry, and the
one-attempt removal fails deterministically.

## Task 31.1's scenarios

| Scenario | Proof through the public `serve` | Status |
| --- | --- | --- |
| Nonretaining serving, Clean closure, retained-plan refusal, unbound precedence | the two `protocol` tests above; `a_manual_invocation_…_is_unbound` and the binding suite unchanged and passing | observed |
| Fatal causes persist; successful exec then failed initialization | `protocol::a_server_off_its_pin_or_lacking_a_granted_tool…`, `…lacking_a_dependency_or_speaking_no_mcp…`, `…never_answers_ends_at_the_startup_deadline` (29–32 s) | observed |
| Complete frame/EOF, trailing byte, status tags, bootstrap death on both sides, store held by deadline/cancel | `capability_broker::handoff::` (10 tests, U6c8's), now through the lifted handler | observed (passing); their removals are U6c8's |
| Exact environment, cwd, descriptors; code-loading name reaches only the box | `handoff::the_server_inherits_only…` (stderr now a pipe), `handoff::a_protected_installed_server_alone_receives…` (`PYTHONUSERBASE`), tracer leak scan of every other exec's argv and environment | observed |
| Single injector | `machine_proof::expose_for_spawn_has_exactly_one_production_call_site`, `machine_proof::egress::` | observed |
| Two-server and replacement separation; no host markers | `serving::two_servers_and_a_replacement…` | observed |
| Host-loopback and abstract Unix sentinels | `serving::only_a_shared_network…`: isolated denies both and accepts nothing; shared reaches both, one accept each | observed with `/usr/bin/bash` and `/usr/bin/python3` in the box (the test skips, failing under required boundary evidence, where either is missing) |
| Installation credential/history limits | `serving::the_server_reads_its_installed_bytes…`: the canary and the bytes dated 2001 are read unredacted and unjudged; the seat-alias and store causes stay with the existing identity and store tests | observed |
| Masking edge (literal/encoded/split/multibyte, duplicate keys, numbers, unsafe correlation, retention on/off/veto seams) | U6f1a's `rpc::tests::masking` and `machine_proof::output_edge` suites, unchanged and passing; masking removal at the public boundary above | observed |
| HOME/TMPDIR collision removals at the public boundary | above, zero lookups; the normal-binding and private-directory controls are `handoff::a_protected_installed_server…` and `serving::two_servers…` | observed |
| Readiness removals | U6c6a/U6c6b's public and unit removals (`neither_a_host_nor_stdout_nor_a_marker_carries_readiness`, `decided`) under the unchanged handler path | observed earlier, passing now |
| Each egress class | the broker serves the plan's sealed network, isolated or shared, and both are exercised above. The projection from local, contracted, uncontracted or omitted egress to that network is runtime compile's, behind the MCP fence until U9b1 | broker side observed; class projection through the real engine is 46.1's |
| External endpoint | second visit: `serving::only_a_shared_network_reaches_an_external_endpoint…`, a listener on the host's own address on its route out (no loopback), which an isolated box cannot reach and a shared one does | observed; no packet leaves the host, and a host without a route skips |
| Shared-network DNS uses only its checked resolver input | second visit: `serving::a_box_resolves_names…`: the box's whole `/etc` listing, its generated `nsswitch.conf` and `hosts`, and its `resolv.conf` (the host's bytes, shared only); third visit: a DNS query from the box for `_localdnsstub`, which no hosts file holds, answered exactly `127.0.0.53` by the systemd-resolved stub the bound `resolv.conf` names, and unresolved in an isolated box | observed; a host whose `resolv.conf` names no systemd-resolved stub skips |
| Singleton system entry and dedicated native/script/package controls with system hard links intact | script package: the `protocol` and `serving` tests above; singleton system entry: SD4's `/usr/bin/dash` profile below; native package: third visit, `serving::a_dedicated_native_server_in_a_package…` | observed |

## SD4 on Linux x86_64

The samples used a scratch test that was not committed
(`.forge/u6f1b-sd4-scratch.rs`). It drove the public `serve` of a
nonretaining plan under the no_new_privs harness shell, timed from spawn to
the harness's first answer (initialize). That covers reobservation, launch,
readiness, handoff, exec, server initialization, the catalog listing and the
greeting. Engine preparation was timed separately as the plan's own
observation (`Sealed::observing`, the consumed `broker observe`). Sources
were unpruned and system hard links intact; the writer was uid 1000, capless
under `no_new_privs`. A first attempt started the broker directly from the
test process instead of under the harness shell. All twelve of its samples
exited 1 without answering. Their stderr was not captured, and they are not
counted. The binary was the test profile's debug build.

| Profile | Entries / mounts | Preparation (s) | Broker startup samples (s) | Total (s) | Broker VmHWM (KiB) | Broker descriptor high-water |
| --- | --- | --- | --- | --- | --- | --- |
| protected package (`opt/docs`) | 205,082 / 3 | 1.412 | 1.427, 1.398, 1.345, 1.414, 1.420, 1.373 | 2.757–2.839 | 25,416–27,144 | 52–53 |
| singleton system entry (`/usr/bin/dash`) | 205,079 / 3 | 1.369 | 1.397, 1.389, 1.314, 1.494, 1.437, 1.414 | 2.683–2.863 | 25,208–27,416 | 51 |

Each total is under SD4's 20 s, and each single observation is under its
10 s. The 30 s deadline and every bound are unchanged. No sample is cold:
dropping the dentry, inode and page caches needs root, which the seat does
not hold. The observer's own descriptor and memory figures are U6c5b's.

## macOS pipe check

As a local experiment, not committed and then restored, the RPC unit tests'
pipes and the real box's stdin, stdout, stderr, control and status pipes
were set to 16 KiB with `F_SETPIPE_SZ`, each size asserted. `broker::` lib
tests passed (118), and so did `capability_broker::protocol::` and
`capability_broker::handoff::` (17). This checks that nothing depends on a
Linux pipe size. It is not macOS evidence.

## Gates on the final tree

The final tree passed the following. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features --locked -- -D
warnings` were clean. The tests passed crate by crate, each under `timeout
590`: brokkr-protocol (909 lib and its three other binaries), brokkr-cli
(lib 761, `it` 607 with 3 ignored, and the five other binaries, which ran
27, 1, 1, 1 and 13 tests), and every other
workspace crate (`--workspace --exclude brokkr-cli --exclude
brokkr-protocol`). Both `recipes/self` and `recipes/verify` compiled.
`openspec validate --all --strict` passed 19 of 19. `typos --hidden` was
silent. `git diff --check` was clean, staged and unstaged. In
`quality/ratchet.sh`, `files`, `clones` and `api` held.
`witness_digests::` passed unblessed (6), so no pin moved.
`scripts/measure-budgets.sh` reported 101 prompt sites, 329 packages and
unchanged heap peaks. It also rewrote `prompt-bytes.json`, `crate-count.json`
and `heap-bytes.json` with main's existing drift: smaller prompt budgets
from earlier merges, and re-dated notes. This unit changes none of their
inputs, so those three files were restored and are not part of the commit.

Test files changed in `quality/file-lines.txt`: `capability_broker.rs`
1861 → 1865, `protocol.rs` 179 → 335, the new `serving.rs` 174, and the
others as the diff shows. All stay under 2,000. `machine_proof.rs` is
untouched at its baseline. Its new proof lives in its existing child
`machine_proof/output_edge.rs`.

## Second run: the third review's H1 and L1 to L4

The third review of run 1's head (`ed83689a`) held on one HIGH and four
LOWs, and the operator ruled on 2026-10-13 that this run repairs all of
them. The repair makes one invariant true. Every byte `serve` writes to
its own standard error is a line one voice passed: display sanitization,
then the shared masker, then the final guard, on the very bytes written.
Those lines are the server's tail and the broker's own last line,
`error: ` included. They are written by one writer, once, after the
ledger has closed and while the bindings that guarded it are still held.
The harness's stdout and the ledger keep U6f1a's paths, the output edge's
`framed` and the ledger's own guard, and both now run while those
bindings are held too. A byte that cannot pass is not written.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker.rs` (116 → 118) | The Serve arm returns `session::serve`'s exit code; nothing is reported after the session returns. Its module documentation says where every stderr byte goes and that a retained plan is refused before any MCP message. |
| `crates/brokkr-cli/src/broker/rpc.rs` (711 → 707) | `serving` leaves the drained tail with the lifetime (`Bound::said`) and no longer writes it; `diagnosed` is gone. `initialized` refuses a retained plan before `started` (L4). The module documentation says so. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (213 → 255), existing child | H1: `serve` returns the exit code. A new `Held` keeps the bindings once known, the withheld flag and the tail, and outlives the ledger. `lived` takes it, `Binder` holds it, and `Bound` hands serving its `said`. `Held::reported` makes the last line and writes the tail and it once (`stderr::told`). The bindings are dropped only after that. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (205 → 318), existing child | `Voice` holds the bindings and their guard. `Voice::said` shows a line (`render::Safe` per line, the shared masker before and after) and keeps it only where the guard passes those bytes. Otherwise it shows a fixed line the same way, and otherwise nothing. `Said`, whose field is private to this module, is the only thing `told` writes. `last` renders the failure line whole through `crate::report_to` and voices it, with `UNREPORTED` as its fixed line. It writes nothing where the bindings are uncertain. `told`/`told_to` replace `unheld` and `reported`, and `pieces` is new (L2). The drain keeps `Said` lines through its own `Voice`. |
| `crates/brokkr-cli/src/broker/session/stderr/tests.rs` (new, 119) | A new private test child declared from `stderr.rs`, in its own module tree. |

No other production file moved in this run. Run 1's changes to the
protocol's `broker.rs`, `entry.rs`, `secret/handoff.rs` and
`broker/ledger.rs`, and to `session.rs`, `output.rs` and `calls.rs`, are
kept as they were. brokkr-protocol's public API holds at 1,116
(`bash quality/ratchet.sh api`: "public API holds").

**H1.** The last line is no longer written after `lived` drops the
bindings. It is rendered whole by the crate's own `report_to`, so its text
and exit code stay those of every failed command. It then passes the
session's voice. With `DOCS_TOKEN=error` the broker writes
`[secret:DOCS_TOKEN]: MCP server protocol is invalid`. A digit run is the
other case: a granted tool `v4242424242` that the server lacks, with
`DOCS_TOKEN=04242424242`. Masking cannot remove it, so the guard refuses
it and the fixed line `error: MCP broker session failed and its cause
cannot be safely reported` is written. Where masking the fixed line leaves
a bound value too, nothing is written and the exit code says it. That
happens with `KEY2=error` beside `DOCS_TOKEN=0002`, where `[secret:KEY2]`
spells 2. Lines from the drain's marker take the same path. U6f1a's
masking test therefore now expects its marker masked
(`[MCP server stderr line cannot be [secret:WORD]]`), and keeps nothing
where the masked marker holds a value by number.

**L2.** The broker's stderr is written through a duplicate of fd 2 whose
status flags are never changed. The write runs on a detached thread of its
own, in blocking writes of at most `PIPE_BUF` bytes (4096 on Linux, 512 on
macOS), which a pipe takes whole. This thread waits at most `TOLD` (1 s)
for it and then gives up. The writer is told and begins no further write,
so after the give-up at most the one write under way can land. Each write
but the last ends on a byte no number is written with (`pieces`). Wherever
the writes stop, a reader therefore never holds a number cut to another
value, and a run of such bytes longer than one write is not begun. The
controller suggested polling for writability. `rustix::event::poll` needs
rustix's `event` feature, which `crates/brokkr-cli/Cargo.toml` does not
enable, and the manifest is outside the row, so the thread is the reading
taken. The bounded write ignores the attempt's cancellation, as run 1's
reporter did for the last line. A cancelled attempt's last line is still
offered for 1 s; run 1's tail skipped it.

**L3.** `Voice::shown` decodes the raw-masked line, applies `render::Safe`,
the one sanitizer `lib.rs`'s `safe_lines` uses, to each line, and masks the
result again before the guard reads it. A value split by a control byte is
joined by the sanitizer and then masked whole. Without the second mask,
the guard reads the joined value and refuses the line (removal R5b below).

**L4.** `initialized` checks `plan.retained` before `started`. A retained
plan's server, once exec'd, is sent no MCP message, initialize included,
and the harness's stdio is never taken. The box is still launched and the
bindings handed over, as SD3's scenario allows ("after ending any box it
launched").

**L1.** Behaviour is unchanged from run 1's third visit, and now also
covers the broker's stderr. An uncertain hand-off writes no line, because
no bindings are known to guard one: "nothing unguarded is written", in the
ruling's words quoted above. The two public tests assert the Opened-only
ledger and an empty stderr.

Each test below asserts exact values. Every removal compiled, failed the
named test with the output shown, and was restored. All were run in this
session.

| Test | Removal | Failure observed |
| --- | --- | --- |
| `broker::session::stderr::tests::a_line_is_sanitized_for_display_before_it_is_masked_and_guarded` (new) | R5: `Safe::new` dropped from `shown` | left `Some("clear \u{1b}[2J copy \u{1b}]52;c;aGk=\u{7} back\rover\n")`, right `Some("clear [2J copy ]52;c;aGk= backover\n")` |
| same | R5b: the mask after sanitizing dropped | left `Some("fixed\n")`, right `Some("split [secret:DOCS_TOKEN] end\n")` |
| `capability_broker::serving::a_servers_terminal_sequences_are_shown_inert_and_a_value_they_split_is_masked_whole` (new; ESC[2J, OSC 52, bare CR, the token split by `\001`) | R5 | left `(Some(0), 3, "clear \u{1b}[2J … back\rover\nsplit u6c8-ca\u{1}nary-Qx7Zv end\n")` |
| `broker::session::stderr::tests::the_last_line_is_rendered_whole_and_written_only_by_known_bindings` (new) | R1: `last` voices with `Voice::new(&[])` | left `Some("error: MCP server protocol is invalid\n")`, right `Some("[secret:DOCS_TOKEN]: MCP server protocol is invalid\n")` |
| `capability_broker::protocol::the_brokers_last_line_is_masked_whole_or_replaced_while_its_bindings_are_held` (new; `error`, a cause word, a digit run, and none) | R1 | the leak scan found `Some("error")` on the broker's stderr |
| `capability_broker::handoff::a_store_read_held_when_the_attempt_is_cancelled_is_refused_at_once` (stderr now `""`) | R2: `Held::reported` ignores `withheld` | left `"error: MCP server secret environment could not be delivered\n"`, right `""` |
| same, and `…_past_the_deadline_is_refused_at_the_deadline` | R7: `Binder::bind` binds `Uncertain` as none | both: left `([], Some(Failed(Refused(Handoff))), None)`, right `([], None, Some(Lifecycle))` |
| `capability_broker::serving::a_standard_error_nobody_reads_never_holds_the_sessions_end` (extended: the pipe's shared status flags read every millisecond from before spawn until after exit; its protocol case no longer retains) | R3: `told` sets `NONBLOCK` on the shared description for the bound and restores it, as run 1 did | left `("clean", (Some(0), 3, true, false))`, right `(…, true, true)` |
| `broker::session::stderr::tests::a_sink_that_takes_nothing_is_given_up_on_at_its_bound_and_one_that_reads_takes_it_all` (new; flags equal before and after) | R4: the bound tripled | left `(false, false)`, right `(false, true)`, after 3.009 s |
| `broker::session::stderr::tests::writes_are_cut_only_where_no_number_is_written` (new) | R8: a cut anywhere in the window | left `[4096, 9]`, right `[4091, 14]` |
| `broker::rpc::tests::delivery::a_retained_plan_is_refused_before_its_server_or_its_harness_is_told_anything` (new) | R6: the retention check after `started` | left: the server read `initialize`, `notifications/initialized` and `tools/list`; right `[]` |
| `capability_broker::protocol::a_retained_plan_is_refused_before_any_mcp_message_reaches_its_server` (renamed from `…is_initialized_and_refused…`; its server reports every line it reads) | R6 | left `(Some(1), [], "", "heard {\"id\":1,…\"method\":\"initialize\",…}\nerror: MCP server call timed out\n")` |
| `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice` (new) | R9a: `serving` writes the cause to `std::io::stderr()` | left `[("…/rpc.rs", "serving"), ("…/stderr.rs", "told")]` |
| same | R9b: `eprintln!` in `serving` | left `[("…/rpc.rs", "println!("), ("…/rpc.rs", "eprintln!(")]`, right `[]` |
| `broker::rpc::tests::masking::standard_error_is_proved_as_the_text_it_decodes_to` (changed: the marker passes the masker) | R10: the fixed line not shown | left `"next\n"`, right `"[MCP server stderr line cannot be [secret:WORD]]\nnext\n"` |

Other tests changed without a new removal. `machine_proof::output_edge`'s
two pins now name `shown` for `mask_text` and `serving` for `stderr.tail(`.
Because retention now refuses first, the post-initialization controls use
non-retaining plans: `handoff.rs`'s `serving` helper, with
`sealed_serving(…, true)` for the one SPEAKS admission test, and
`protocol.rs`'s new `unretained` for the off-pin, dependency, never-answers
and closure tests. The flood test's second half was a retained plan, whose
tail is now racy. It became a server that floods and then speaks no MCP,
and it expects the whole 64 KiB tail before the protocol cause. The
lifetime and readiness unit tests pass `&Held::default()` to `lived`. The
rpc harness reads the drain's tail, and the masking tests read `Said`
through its `as_str`, which only tests use.

The 16 KiB pipe check was temporary, uncommitted and then restored. It
added rustix's `pipe` feature to both crates' manifests and sized the rpc
tests' pipes, the real box's eight launch pipes and the unread-stderr
fixture to 16,384 bytes with `F_SETPIPE_SZ`, each size asserted. Under it,
`broker::` lib tests passed 125 and `capability_broker::` passed 88. With
`PIPE_BUF` set to macOS's 512 on Linux, the stderr unit tests passed 4 and
`capability_broker::serving::` passed 9. `git diff` of both manifests
against HEAD was empty afterwards. This is no macOS evidence.

The one-CPU timing controls were not observed. The seat's sandbox refuses
`taskset` ("What 'taskset' starts can't be checked before it runs"). On
this host's 24 CPUs, `delivery::what_waits…` took 13.75 s.

SD4, second run. Six warm samples were taken from a temporary,
uncommitted test, since removed. Each sample timed the plan's own
observation (`Sealed::observing`, which runs `broker observe`) and then
`serve` from spawn to the harness's first answer, that is, re-observation,
launch, readiness, handoff, exec, initialize, listing and greeting. The
profile was the script package (`opt/docs`), isolated, 205,081 entries and
2 mounts, unpruned, with system hard links intact, on the debug test build
and 24 CPUs.

| Sample | Preparation (s) | Broker startup (s) | Total (s) |
| --- | --- | --- | --- |
| 1 | 1.315 | 1.347 | 2.662 |
| 2 | 1.341 | 1.330 | 2.672 |
| 3 | 1.272 | 1.374 | 2.647 |
| 4 | 1.291 | 1.337 | 2.628 |
| 5 | 1.328 | 1.342 | 2.670 |
| 6 | 1.286 | 1.326 | 2.613 |

Each total counts both actual source observations, the preparation's and
the broker's own. Each observation is under 10 s and each total under
20 s, and MB3's 30 s bound is unchanged. Memory and descriptor high-water
marks were not re-measured in this run; run 1's figures stand above. No
sample is cold.

The second run's gates on the final tree were these. `cargo fmt --all --
--check` and workspace clippy with `-D warnings` were clean. The brokkr-cli
lib passed 768 with 1 ignored. In `--test it`, `capability_broker::`
passed 88 and the rest passed 527 with 3 ignored, `machine_proof::`'s 66
included. `driver_conformance`, `transcript_surfaces` and the three heap
binaries passed 27, 13, 1, 1 and 1. brokkr-protocol passed 909 with 3
ignored, then 6, 1 and 1. Every other crate passed with no failure,
brokkr-runtime's `witness_digests::` unblessed among them, so no pin moved.
`recipes/self` and `recipes/verify` compiled. `openspec validate --all
--strict` passed 19 of 19, `typos --hidden` was silent, and `git diff
--check` was clean, staged and unstaged. `bash quality/ratchet.sh` held
for `files`, `clones` and `api`. `suppressions::` passed 6 and no
suppression was added. No function was added to `too-many-lines.txt`. The
file-lines rows moved are in the table above, together with the test files
`rpc/tests.rs` 1969 → 1972, `rpc/tests/delivery.rs` 147 → 174,
`rpc/tests/masking.rs` 1457 → 1466, `lifetime/tests.rs` 98 → 99,
`capability_broker/handoff.rs` 860 → 874, `protocol.rs` 386 → 465,
`serving.rs` 536 → 594 and `machine_proof/output_edge.rs` 129 → 173.
`machine_proof.rs` stays at its 3,451 baseline.
`scripts/measure-budgets.sh` was not run: no prompt, crate or heap input
moved.

## Third run: the second review's H1, M1, L1, L3 and the L2 limit

The review of run 2's head (`b93a603e`) held on one HIGH, one MEDIUM and
LOWs, all from one cause: without poll, run 2 wrote the broker's standard
error from a detached thread, in pieces cut by digits alone. The operator
ruled on 2026-10-13 that this row adds `crates/brokkr-cli/Cargo.toml` for
rustix's `event` feature only. That feature is the one manifest change
(`"event"` added to the existing list); `git diff HEAD -- Cargo.lock` is
empty. Run 2's repairs, which its review confirmed, are kept unchanged:
the final failure line through one voice, sanitizing before the guard,
the retained-plan refusal before any MCP message, bind-once and the
uncertain hand-off as ruled. The invariant this run makes true is that
the broker never changes the status flags of a descriptor it shares. It
waits for each by `rustix::event::poll` and writes on its own thread while
it holds the bindings. No sink is left holding bytes the final guard
would refuse at any point a stopped write can leave.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/Cargo.toml` | rustix's features gain `event`, nothing else. |
| `crates/brokkr-cli/src/broker/rpc.rs` (707 → 761) | New `ATOMIC` (512), `ready` (one `poll` of one descriptor, its end, error or closing counted ready, `EINTR` not ready) and `offered` (writes of at most `ATOMIC` bytes, each once `poll` finds the sink writable, never waiting). `Lines::new` sets no flag and is infallible: `next` reads only once `ready` finds the pipe readable. `Server::new` sets no flag; `Server::flushed` writes through `offered`. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (745 → 744), existing child | `served_within` no longer sets `FIONBIO` on the harness's stdout, and `Lines::new` on its stdin no longer can. |
| `crates/brokkr-cli/src/broker/rpc/calls/outbox.rs` (66 → 68), existing child | `Outbox::flushed` writes through `offered` and takes `Write + AsFd`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (318 → 329), existing child | `Voice::said` cuts the shown line into lines of at most `ATOMIC` bytes, newline included, on character boundaries (`cut`), before guarding. It says the line only where the whole shown line and every piece on its own pass the guard, else its fixed line the same way, else nothing. `told_to` has no thread: it writes one piece a write, each once `ready` finds the sink writable by its deadline, and after every write reads what the piece left written by the guard, writing nothing more where that refuses. `told` takes the bindings and builds that voice. `pieces`, the `PIPE_BUF` constants and the thread, channel and flag are gone. `Drain::new` is infallible. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (255 → 256), existing child | `Held::reported` hands `told` the kept bindings, which it drops only once `told` has returned. |
| `crates/brokkr-cli/src/broker/session.rs` (775 → 776) | L3: `Cause`'s documentation says the retention cause comes before any MCP message reaches the server, not after it initializes. |

No other production file moved, and no new module was added.
brokkr-protocol is unchanged in this run, and its public API holds at
1,116 entries (`bash quality/ratchet.sh api`: "public API holds").

**H1.** The review's reproduction cut a guarded line where its prefix
decoded to the bound value. Now every write is one whole piece the guard
passed on its own. The regression binds `DOCS_TOKEN=0012` and a line of
509 `x` then `123`. The whole line passes, as 123. Its first piece of 511
bytes ends in `12`, which the guard refuses, so the fixed marker is said
in its place. A sink that takes one write and refuses the next holds
`fixed\n` alone, which the guard passes. The public repetition binds
`0084726193`, spelled apart from the line, and the server writes 503
spaces then `847261933`. The broker's standard error is a pipe with room
for one write and no more (filled a page at a time, then one page read
back). It holds what it held plus `[MCP server stderr line cannot be
safely masked]\n`, and the session's last line is given up on at its
bound. A write a sink takes short (a terminal or a file may) is read as
it stands; where the guard refuses that, nothing more is written.

**M1.** No writer is detached. `told_to` writes on the caller's thread,
which is `Held::reported`'s, within `TOLD` (1 s) of its start. Each piece
waits for writability with the time left, and none is begun once that
has passed. Every write has returned before `told_to` returns, and only
then does `reported` return and drop the bindings.

**L1.** The broker's stdin, stdout and stderr, and every duplicate of
them, are waited on by poll. Their flags are never set, so a read takes
what poll found and a write is at most 512 bytes after `POLLOUT`. The
box's own pipes in `session/readiness.rs`, whose other ends only the box
holds, keep their `FIONBIO`. A new machine-proof pin holds those three
sites as the only ones.

**L3.** `session.rs`'s `Cause` comment is corrected above.
`tests/capability_broker.rs`'s `admitted` now says the retained plan is
refused before any MCP message reaches its server, so before it
initializes.

**L2, recorded limit.** The ledger's Opened is appended before the
bindings are known, as the ruled opening order requires, and this run
does not change that order. The operator's addendum of 2026-10-13 rules
it in these words: "The ledger's Opened record is appended before the
bindings are known, as the ruled opening order requires, so a bound value
that equals the plan's owner or digest can stand in it. That is a
recorded limit of that order, not a defect; only operators read the
ledger."

`PIPE_BUF` is not read from the platform; that part of the fix is
deviated from. Neither `libc` nor rustix's `pipe` feature, which holds
`PIPE_BUF`, is a dependency of brokkr-cli. The ruling adds `event` alone,
so either would be a second manifest change. The writes use POSIX's
`_POSIX_PIPE_BUF`, 512, the least `PIPE_BUF` any POSIX platform may set.
That is macOS's own and at most Linux's 4096, so a pipe takes each write
whole on both hosts without a per-platform constant. The cost is that a
line over 511 bytes is cut on Linux too, where 4,095 would have fitted.
The broker's 512-scalar failure line for a long binding name is one such
line. `capability_broker::a_long_binding_name_is_cut_to_one_bounded_line`
now expects it as two lines, of 510 and 507 bytes with their newlines.
`masking::a_line_past_its_bound_is_passed_over_and_the_tail_keeps_its_bound`
now keeps 128 lines of 512 bytes rather than 64 of 1 KiB.

The third run's removals follow, S1 to S4c. Each compiled, made the
named test fail as shown, and was then undone, and the test passed
again; every new test asserts exact values.

| Test | Removal | Failure observed |
| --- | --- | --- |
| `broker::session::stderr::tests::a_line_one_write_cannot_take_is_said_only_where_each_piece_passes_on_its_own` (new) | S1: `Voice::said` no longer guards each piece | left `("xxx…xx12\n", false, false)`, right `("fixed\n", false, true)` |
| `capability_broker::serving::a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed` (new) | S1 | the pipe held the filler then `…32, 32, 56, 52, 55, 50, 54, 49, 57, 51, 10`, 503 spaces and `84726193\n`, where the marker was expected |
| `broker::session::stderr::tests::a_write_taken_short_is_guarded_as_it_stands_and_one_refused_ends_the_writing` (new) | S2: `told_to` no longer guards what a write left | left `("0012", ("x123\nnext\n", true))`, right `("0012", ("x12", false))` |
| `broker::session::stderr::tests::every_write_has_returned_before_the_writer_does` (new) | S3: `told_to` hands the bytes to a spawned, detached thread and returns, run 2's shape | left `(true, 0, true)`, right `(true, 3, true)` |
| `capability_broker::serving::the_brokers_standard_streams_keep_their_status_flags_whether_served_or_failed` (new) | S4a: `Lines::new` sets `FIONBIO` again | clean case, stdin `[OFlags(0x0), OFlags(NONBLOCK)]` |
| same | S4b: `told` sets `FIONBIO` on its duplicate of stderr | protocol case, stderr's write-only flags gained `NONBLOCK` |
| same | S4c: `served_within` sets `FIONBIO` on stdout again | clean case, stdout's write-only flags gained `NONBLOCK` |
| `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice` (extended: `ioctl_fionbio(` only in `readiness.rs`, three times) | S4a | failed at `output_edge.rs:150`, four sites |

Other tests changed without a new removal. `stderr/tests.rs` drops
`writes_are_cut_only_where_no_number_is_written` with `pieces`, and its
bounded-sink test passes the voice and deadline. The rpc and masking
tests' `Drain::new` and `Lines::new` lose `.unwrap()`. `delivery.rs`'s
deadline test gives `Outbox::flushed` a pipe instead of a `Vec`.
`protocol.rs`'s `talking_beside` takes its command from a new
`harnessing`, which `serving.rs`'s `flagged` shares. The `unheard` helper's
read of what a pipe still holds became `left`, which the one-write test
shares. `output_edge.rs` pins `Voice::new(` in `told` too.

The 16 KiB pipe check was temporary and uncommitted, and was then
restored with `git checkout` to the staged tree. It added rustix's `pipe`
feature to brokkr-cli's manifest and sized to 16,384 bytes, each size
asserted, the rpc tests' `pipe()`, the bounded-sink unit test's pipe,
`serving.rs`'s `unread` and `flagged`'s three pipes. Under it,
`broker::` lib tests passed 127 and `capability_broker::serving::` passed
11. The box's own launch pipes were not resized in this run. This is no
macOS evidence.

The one-CPU timing controls were not observed: the seat's sandbox refuses
`taskset` ("What 'taskset' starts can't be checked before it runs"). On
this host's 24 CPUs, `a_listing_of_the_grant_past_mb3s_bound_on_a_response_ends_the_session`
took 0.81 s, `an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session`
2.38 s, `delivery::what_waits_for_the_harness_is_bounded_and_taken_whole_in_order`
13.87 s and `delivery::an_answer_is_written_as_fast_as_the_harness_takes_it`
0.55 s.

The third run's gates on the final tree were these. `cargo fmt --all --
--check` and workspace clippy with `-D warnings` were clean. The
brokkr-cli lib passed 773 with 1 ignored. In `--test it`,
`capability_broker::` passed 90, `machine_proof::` passed 66, and the rest
passed 466 with 3 ignored. `driver_conformance`, `transcript_surfaces` and
the three heap binaries passed 27, 13, 1, 1 and 1. brokkr-protocol passed
912 with 3 ignored, then 6, 1 and 1. brokkr-runtime's `witness_digests::`
passed 6 unblessed, so no pin moved. `recipes/self` and `recipes/verify`
compiled. `openspec validate --all --strict` passed 20 of 20, `typos
--hidden` was silent, and `git diff --check` was clean on the staged
tree. `bash quality/ratchet.sh` held for `files`, `clones` and `api`. No
suppression was added. The file-lines rows moved are in the table above,
together with the test files `rpc/tests/delivery.rs` 174 → 181,
`rpc/tests/masking.rs` 1466 → 1470, `session/stderr/tests.rs` 119 → 242,
`capability_broker.rs` 1865 → 1870, `protocol.rs` 465 → 472, `serving.rs`
594 → 685 and `machine_proof/output_edge.rs` 173 → 184.

## Fourth run: the third run's H1, M1, M2 and L1

The review of run 3's head (`5157d4db`) held on one HIGH, two MEDIUMs and
a LOW. Run 3's repairs, which that review confirmed, are kept: the one
sanitized and masked final-error voice, the bindings held through the
synchronous standard-error writes, no shared status flag changed, the
retained-plan refusal before initialize, bind-once, the uncertain hand-off
as ruled, and `event` as the manifest's one change (`git diff HEAD --
Cargo.lock` is empty). The operator's addendum of 2026-10-13
(`operator-ruling-2026-10-03.md`) rules the repair in these words:

> 2026-10-13, after U6f1b's third run: every frame and line the broker
> writes is cut only at safe points: never inside a number or an escape as
> the final guard reads them, raw or decoded, so a pipe is never left
> holding part of a number. Each piece is at most 512 bytes and is written
> whole after poll reports the sink ready. If a sink that is not a pipe
> takes only part of a piece, the session ends as a delivery failure, and
> those bytes are not an emission of the broker's. Two limits are
> recorded. A write of at most 512 bytes, made after poll reports the sink
> ready, can still wait if another process writes the same pipe at that
> moment; the deadline and cancellation are rechecked around every write.
> And 512 bytes, POSIX's least PIPE_BUF, is used on both platforms instead
> of each platform's own value.

`design.md`'s U6f1b section carries the same ruling: "Every frame and line
the broker writes is cut only at safe points: never inside a number or an
escape as the final guard reads them, raw or decoded, so a pipe is never
left holding part of a number. Each piece is at most 512 bytes and is
written whole after poll reports the sink ready. If a sink that is not a
pipe takes only part of a piece, the session ends as a delivery failure,
and those bytes are not an emission of the broker's. Two limits are
recorded. A write of at most 512 bytes, made after poll reports the sink
ready, can still wait if another process writes the same pipe at that
moment; the deadline and cancellation are rechecked around every write.
And 512 bytes, POSIX's least PIPE_BUF, is used on both platforms instead
of each platform's own value. (operator ruling 2026-10-13)"

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker/rpc/pieces.rs` (new, 173), new private child declared from `rpc.rs` | The one cut rule and the one piece writer. `safe` lists a frame's safe points, `cut` cuts at the last one within each 512 bytes or refuses, `Pieces` holds a harness frame with its ends, and `put` offers one piece in one write. Added under the standing rule of 2026-10-09 as extended 2026-10-11: inline, it would push `rpc.rs` (761) past 800. |
| `crates/brokkr-cli/src/broker/rpc.rs` (761 → 765) | Declares `pieces`. `ATOMIC`'s and `offered`'s comments say 512 is the operator's limit and that `offered` now writes only the server's input, which no guard reads. |
| `crates/brokkr-cli/src/broker/output.rs` (395 → 399), existing child | `framed` returns the guarded frame as `Pieces`, or the masking cause where it cannot be cut safely. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (744 → 747), existing child | Writes `Pieces`. `flush` passes the attempt's cancellation into `Outbox::flushed` and no longer checks it only beforehand. |
| `crates/brokkr-cli/src/broker/rpc/calls/outbox.rs` (68 → 79), existing child | `flushed` writes one piece at a time through `put`, with the frame's deadline and the cancellation. A deadline or cancellation between pieces stalls the frame; a failed or partial write is `Gone`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (329 → 314), existing child | `cut` cuts each shown line with `pieces::cut` and gives up where it cannot (the fixed line is said instead). `told_to` takes the attempt and writes through `put`, so it no longer guards a partial write. `told` takes the cancellation, not the bindings. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (256 → 256), existing child | `serve` hands its cancellation to `Held::reported`, which hands it to `told`. |

No other production file moved and nothing was added to `Cargo.toml`.
brokkr-protocol is not touched in this run, and `bash quality/ratchet.sh
api` reported "public API holds" with `brokkr-protocol.txt` at 1,116
entries.

**H1.** A point is safe where the byte after it is inert and no escape
spans it. Inert means no ASCII digit, `.`, `+`, `-`, `e`, `E`, backslash or
UTF-8 continuation byte. Escapes are tracked from the frame's first byte
exactly as the guard's decoder spans them: a `\u` and four hex digits is
six bytes, any other backslash and the byte after it two, and a last
backslash one. A piece ends at the last safe point within 512 bytes. Where
512 bytes in a row hold no safe point, a harness frame is refused with the
masking cause, "MCP response cannot be safely masked", and a standard
error line is replaced by its fixed line. Neither is ever cut unsafely.

The rule is the commission's sufficient rule with one side relaxed. The
commission named cuts between two inert bytes. This run requires only the
byte after the cut to be inert. The guard reads every number, raw or
decoded, from bytes that are not inert and from escapes. The byte after a
safe point is inert, begins no escape and decodes to itself. So no number
the guard reads goes on past that point. The bytes before it are read
exactly as far as the whole frame's bytes are, holding the same numbers,
needles and escapes. The relaxed side matters for real frames. Under the
two-inert rule, a compact JSON array of small numbers (`[0,1,2,…]`) has no
safe point. The existing `masking::the_final_guard_ends_a_frames_scan_at_its_deadline_or_cancellation`
frames 300 such numbers and would then have been refused. The property
test below exercises the relaxed rule against the guard itself, both
removals of it fail that test, and a decoded sign before a digit (an
escaped `+` before `5`) is never a cut, because the digit after it is not
inert.

A sink that is not a pipe and takes part of a piece fails the write. On
the harness's output that is `Delivery::Gone`, the output-fails ending the
session already had for a write error. On standard error it ends the
writing. Nothing more is offered either way, and what the sink took is
not treated as an emission. This run reads "the session ends as a delivery
failure" as that existing ending. No new cause was added, because one
would be a public `Failure` variant.

**M1.** `put` is the one writer for both streams. It reads the deadline
and cancellation before the poll, caps the poll's wait at the deadline,
and reads both again after the poll and before the write. A late or
cancelled attempt writes nothing. On the harness's output each piece's
deadline is its frame's own. `Session::bound` already caps that at the
attempt's. The session's last failure frame has its fixed `CLOSING`
second past an attempt deadline that has gone. A first version also capped
each piece at the attempt's deadline. That suppressed that last frame:
`rpc::tests::a_server_that_takes_no_call_ends_it_and_an_ended_harness_leaves_it_begun`
and `rpc::tests::no_traffic_extends_a_calls_deadline_and_none_spends_past_its_budget`
failed, at `tests.rs:1488` and `:1253`, with `left: None` where the
timeout frame was expected. So the cap was taken out.

On standard error the deadline is `TOLD` (1 s). The cancellation `told`
passes stops the writing when the attempt is cancelled while the lines are
being written. A cancellation that had already ended the session leaves its
cause still to be told within `TOLD`, as before. Passing the raw
cancellation through failed three existing MB3 proofs that a cancelled
attempt still reports its cause:
`capability_broker::readiness::a_cancelled_attempt_ends_a_box_held_at_its_entry`
(`readiness.rs:292`),
`readiness::the_brokers_own_line_is_read_whole_while_its_tracer_writes_notices`
(`:310`) and
`observer::a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running`
(`observer.rs:910`). With the cancellation measured from when `told` begins,
all three pass. One write of at most 512 bytes after the poll is the only
wait past a deadline, and it is the operator's recorded limit.

**L1.** 512 bytes on Linux and macOS alike is the operator's limit, as
quoted above, and not a deviation. Run 3's paragraph calling it one is
superseded.

**The handoff flake.** `EXECS` now traces `set_tid_address` beside
`execve`. A program's C library makes that call once as it starts, so a
process whose exec failed never makes it afterwards. `exec_succeeded`
accepts an exec that ended `= 0`, or one that ended `= ?` only where the
same process's trace has `set_tid_address` after it. No other ending is
accepted. The protected-server test and the terminal test both use it, and
`the_kernel_alone_runs_a_script…` also asserts that each refused exec
(ENOENT, ENOEXEC) is not taken as succeeded. A probe checked that the real
trace holds the call: the test temporarily required `set_tid_address` even
for `= 0`, and the protected-server and terminal tests still passed, then
the check was restored.

The fourth run's removals follow. Each compiled, made the named tests fail
as shown, and was then undone, after which the tests passed again. Every
new test asserts exact values.

| Removal | Tests | Failure observed |
| --- | --- | --- |
| R1: digits count as inert (`inert` drops `b'0'..=b'9'`) | `broker::rpc::pieces::tests::every_safe_prefix_of_a_frame_the_guard_passes_passes_it_too` (new, 4,000 generated frames read as JSON and as text; it asserts at least 1,000 frames the guard passes and at least 100 whose unsafe cut it refuses); `…pieces::tests::a_frame_is_cut_only_before_an_inert_byte_no_escape_spans` (new); `broker::session::stderr::tests::a_line_one_write_cannot_take_is_cut_only_at_its_safe_points` (new) | the property test found `json true: "1\\e\\e}+e}12"`; the other two failed at `pieces/tests.rs:92` and `stderr/tests.rs:125` |
| R2: escapes untracked (`spanned` taken as 1) | the property test and the stderr test | `json true: ",123\\u0031:E\\1\\u0032\\"`; `stderr/tests.rs:125` |
| R3: every boundary safe (`inert(…) \| true`), so pieces are cut at 512 again | `capability_broker::serving::an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number` (new); `capability_broker::serving::a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed` (now expects the safe piece) | both failed their pipe-content assertions (`serving.rs:695`, case `number`, and `serving.rs:592`): what each pipe held after the filler differed from the safe piece; the printed byte lists were cut off before the bytes that differ |
| R4: `put` ignores its attempt entirely | `pieces::tests::a_cancellation_the_poll_outlived_writes_nothing` (new), `…a_piece_is_offered_whole_once_ready_and_never_once_its_attempt_has_ended` (new), `rpc::tests::delivery::a_frame_is_written_a_piece_at_a_time_and_nothing_more_past_a_cancellation_or_deadline` (new), `stderr::tests::a_cancellation_or_a_deadline_between_pieces_writes_nothing_more` (new) | left `(Taken, 0, "one\n")`, right `(Late, 2, [])`; left `Taken`, right `Late`; left `Open`, right `Stalled`; left `("one\ntwo\n", true)`, right `("one\n", false)`. A first run of the whole `broker::` filters under R4 did not finish and was ended by `timeout 590`; which test held it was not observed. |
| R5: no recheck after the poll | `a_cancellation_the_poll_outlived_writes_nothing` | left `(Taken, 1, "one\n")`, right `(Late, 2, [])` |
| R6: no check before the poll | `stderr::tests::a_sink_that_takes_nothing_is_given_up_on_at_its_bound_and_one_that_reads_takes_it_all` (extended: cancelled as it waits); `a_cancellation_the_poll_outlived_writes_nothing` | `1.001030317s`: left `(false, false)`, right `(false, true)`; left `(Taken, 1, …)` |
| R7: `Outbox::flushed` reads no frame deadline between pieces | `delivery::a_frame_is_written_a_piece_at_a_time…` | left `Open`, right `Stalled` |
| R8: `exec_succeeded` drops `= 0` | `capability_broker::handoff::an_exec_whose_return_the_tracer_lost_succeeded_only_where_its_program_went_on` (new) | left `("returned", false)`, right `("returned", true)` |
| R9: `exec_succeeded` accepts `= ?` alone | same | left `("lost alone", true)`, right `("lost alone", false)` |
| R10: `told` passes the raw cancellation | the three cancelled-attempt proofs named under M1 | failed at `readiness.rs:292`, `readiness.rs:310` and `observer.rs:910` |

Other tests changed without a new removal. The run-3 stderr test that
expected `x12` to stay with `0012` bound is gone. In its place
`a_write_taken_short_ends_the_writing_whatever_it_took` asserts that a
3-byte sink is offered one write and nothing more: `("abc", false, 1)`.
`delivery::an_answer_with_512_bytes_and_no_safe_point_ends_the_session_masked`
(new) has a server answer with 513 digits in one string. The harness is
told MB4's cause and the call fails with `Failure::Masking`. The stderr
test pins an empty line kept as its newline, and a 512-digit line said as
its fixed line. `pieces/tests.rs` holds the shared `bound`, `sink`,
`held` and `within`, which the stderr and delivery tests reuse; the stderr
tests' own copies are gone. `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice`
now finds `Voice::new(` only in `last` and `new`. It also pins
`Pieces::new(` to `framed` and `pieces::cut(` to the stderr `cut`, the
rule's two consumers. `masking::the_final_guard_ends_a_frames_scan…`
compares `Pieces::bytes`. In `serving.rs`, `unread`'s fill is shared as
`filled`.

**The public cases.** `an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number`
binds `DOCS_TOKEN=0012`. The server answers the call with 123 written from
byte 510 of the harness's frame, in two cases. In `number` it is a JSON
number (`"n":123` in `structuredContent`), read raw. In `escaped` it
follows a tab's `\t` escape in the text, read decoded. Each whole frame
passes the guard and its first 512 bytes do not. The harness's output is
a pipe filled, after the greeting and listing are read, but for one page:
a sink that takes the first piece and refuses the next. The test then
kills the harness shell, which cancels the attempt. The pipe holds exactly
the frame's first 509 bytes in `number` (cut before `:`), or 507 in
`escaped` (before the `x` ahead of `\t`), and nothing more. The guard
passes both as JSON and as text. The ledger closes `Failed(Timeout)` with
the call `Succeeded`, and no box is left. Through public serving no frame
can carry a `\u` escape of a digit: the broker re-serializes every
response, and serde_json never writes a digit as an escape. So the
`123` case is bound in the stderr unit test, where server
bytes reach the guard as written, and by the property test's tokens.

**M2, SD4.** No cold sample was observed in this run. Dropping the dentry,
inode and page caches needs root, which this seat does not hold, so cold
samples stay pending. This run does not change the startup path: the
greeting is one small frame through the new writer. Run 1's warm x86_64
samples above are therefore not re-measured here. The aarch64 samples stay
pending for U9b1.

**M2, the final tree's tests**, run crate by crate on this tree, each
command under `timeout 590`:

| Crate | Command | Result |
| --- | --- | --- |
| brokkr-core | `cargo test -p brokkr-core --locked` | lib 136, `it` 25, doc 0 |
| brokkr-store | `cargo test -p brokkr-store --locked` | lib 97, `it` 5 (and its four self-spawned runs of 1 each), doc 0 |
| brokkr-protocol | `cargo test -p brokkr-protocol --locked` | lib 914 with 3 ignored, `hands_exits` 6, `secret_drop` 1, doc 1 |
| brokkr-runtime | `--lib`, then `--tests --bins --examples`, then `--doc` | lib 940; `capability_launch` 75, `it` 132, `operated_repo` 1, `queued_launch` 3; doc 0 |
| brokkr-view | `cargo test -p brokkr-view --locked` | lib 310 with 3 ignored, `it` 6, doc 0 |
| brokkr-bridge | `cargo test -p brokkr-bridge --locked` | lib 17, doc 0 |
| brokkr-cli | `--lib`; `--test it` as `capability_broker::`, `machine_proof::` and the rest skipping both; the five other binaries with `--bins`; `--doc` | lib 781 with 1 ignored; `capability_broker::` 92 in 225.8 s, `machine_proof::` 66, the rest 466 with 3 ignored; `driver_conformance` 27, `transcript_surfaces` 13, `heap_claude`, `heap_codex` and `heap_dsh` 1 each, the binary 0; doc 0 |
| brokkr-seatbelt-probe | `cargo test -p brokkr-seatbelt-probe --locked` | `seatbelt_lifetime_probe` 99 with 2 ignored |

No failure was seen. No workspace-wide `cargo test` was run, following the
controller's crate-scoped direction.

**16 KiB pipes.** This was a temporary local check and was not committed.
It added rustix's `pipe` feature to brokkr-cli's manifest. It sized to
16,384 bytes, each size asserted, the rpc tests' `pipe()`, the shared
test sink's pipe and every pipe `serving.rs` makes. Under it `broker::`
lib tests passed 135 and `capability_broker::serving::` passed 12. Every
edit was then undone, and the manifest and `Cargo.lock` match the staged
tree. This is not macOS evidence.

**One CPU.** Not observed. The seat's sandbox refused `taskset` again
("What 'taskset' starts can't be checked before it runs"). Run 3's
controller observations, 2.7 s and 14.2 s at one CPU, are context only,
not this tree's results.

**Gates.** `cargo fmt --all -- --check` and workspace clippy with `-D
warnings`, all targets and features, were clean. After the crate runs,
`cargo fmt` rewrapped one call in `stderr.rs` over four lines with no
token changed. Clippy and the `broker::session::stderr` tests (7) were
then run again and passed. `recipes/self` compiled
with 47 valuations over 11 groups, 0 unruled, and `recipes/verify` with 16
over 5. `witness_digests::` passed 6 unblessed, so no pin moved. In `bash
quality/ratchet.sh`, `files`, `clones` and `api` held. No suppression was
added. `scripts/measure-budgets.sh` was not run: no prompt, crate or heap
input moved. The rows changed in `quality/file-lines.txt` are those in the
table above, plus these test files: `pieces/tests.rs` (new) 282,
`rpc/tests/delivery.rs` 181 → 252, `session/stderr/tests.rs` 242 → 220,
`capability_broker/handoff.rs` 874 → 927, `capability_broker/serving.rs`
685 → 815 and `machine_proof/output_edge.rs` 184 → 187. All stay under
2,000.

## Fourth run, second visit: the review's M1 to M4, L1 and L2

The review of `26697e6b` returned four MEDIUM findings and three LOW ones.
This visit answers M1 to M4 and L2 in a second signed commit on the
branch. L1 is recorded under the follow-ups below. L3 concerns the review
panel's own notes and changes no code. Everything the fourth run's first
visit made is kept: the safe-point cut, the deadline and cancellation read
around every write, the handoff trace rule, and every earlier run's repair.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker/rpc/pieces.rs` (173 → 177) | `Put::Short` is a write that took part of a piece, apart from `Put::Failed`, which is now only a sink that ended or erred. `put` returns it for a write of `1..` bytes short of the piece. |
| `crates/brokkr-cli/src/broker/rpc/calls/outbox.rs` (79 → 83) | `Delivery::Short`, which `flushed` returns on `Put::Short`, where it returned `Gone` before. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (747 → 751) | `going` ends the session with `Fatal::Undelivered` on `Delivery::Short`, ahead of the timeout check, and `drained` does the same. |
| `crates/brokkr-cli/src/broker/rpc.rs` (765 → 772) | `Fatal::Undelivered`, "MCP broker output could not be delivered", which the ledger latches as `Failure::Timeout`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (314 → 317), existing child | `told_to` and `told` return the `Put` that ended the writing, `Taken` where every piece was written whole. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (256 → 273), existing child | `Held::reported` writes through a new closure seam, `reported_with`. Where the writing ended `Put::Short`, the exit code is `Exit::Failed`, whatever the session's was. The bindings are still dropped only once the write has returned. |
| `crates/brokkr-protocol/src/broker/ledger.rs` (616 → 621) | L2: a second `bind` latches `Failure::Masking` before it refuses with `Unrecorded::Record`. |

No production file is new. `Fatal`, `Put` and `Delivery` are private to
the broker's module tree, and `Failure` is unchanged. `bash
quality/ratchet.sh api` reported "public API holds", with brokkr-protocol
at 1,116 entries. `crates/brokkr-cli/Cargo.toml` and `Cargo.lock` match the
first visit's commit; against `fe65a053` the manifest adds `event` alone.

**M1, a short write ends the session.** The first visit read "the session
ends as a delivery failure" as the existing ending for a harness that has
closed its output, which can close Clean. That reading is withdrawn. On the
harness's output, the session now ends `Ended::Failed(Fatal::Undelivered)`.
It exits 1, its last line says "MCP broker output could not be delivered"
through the one voice, and its ledger closes `Failed(Timeout)`. The ledger
records that cause because its `Failure` is a closed public catalogue with
no delivery cause, and this run's direction forbids a new public entry.
`Timeout` is the catalogue's cause for a harness frame that was not taken
whole by its deadline, `Delivery::Stalled`, which is the nearest one. That
mapping is this visit's reading, not a ruling, and is carried as a
follow-up. The broker's standard error is written after the ledger closes,
by design: the last line describes the session's ending, and is written
while the bindings that guarded the closure are still held. So a short
write there cannot reach the ledger. It makes the exit code a failure's,
which is the session's ending on that stream. The operator's ruling of
2026-10-14 replaced both this order and the `Timeout` cause; the fifth run
below implements it.

No pipe takes part of a write of at most `PIPE_BUF` bytes. A public test
therefore cannot make the public binary's standard output or standard
error take a piece short. Both are proved at the session level, through
the code `serve` runs. On standard output, `calls::served` runs over a
started fake server, a bound ledger and a sink that takes 3 bytes a write,
and `settled` closes the ledger. On standard error, `Held::reported_with`
runs `stderr::told_to`, the writer `told` uses, into the same kind of sink.

**M3, the public escaped case.**
`capability_broker::serving::a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed`
now has two cases. `plain` is the first visit's case, unchanged. `escaped`
binds `DOCS_TOKEN=0012`. The real server writes 499 spaces and then
`123` as literal text, with `printf '%499s%s\n'`, to its
standard error. The test asserts that the guard passes the whole 517-byte
line and refuses its first 511 bytes, which end in the escapes of 12. The
broker's standard error has room for one write and no more. It holds
exactly 498 spaces and a newline after what it held: the line is cut
before the space ahead of the first backslash, never inside an escape. The
guard passes that piece. The session exits 1, its harness heard nothing,
and no box is left. Each case reads `bound`, a new helper in `serving.rs`,
which `an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number`
now shares too.

**M4 and the new tests, with their removals.** Each removal below compiled,
made the named tests fail as shown, and was undone. The tests then passed
again: `broker::` lib tests 137, `capability_broker::serving::` 12, and the
ledger test 1.

| Removal | Tests | Failure observed |
| --- | --- | --- |
| RA: `Outbox::flushed` maps `Put::Short` to `Delivery::Gone` | `broker::rpc::tests::delivery::an_output_that_takes_a_piece_short_ends_the_session_as_a_delivery_failure` (new); `…delivery::a_frame_is_written_a_piece_at_a_time_and_nothing_more_past_a_cancellation_or_deadline` (now expects `Short`) | left `(Ok(()), [123, 34, 106], 1)`, right `(Err(Failed(Undelivered)), [123, 34, 106], 1)`; left `Gone`, right `Short` (`delivery.rs:152`) |
| RB: `reported_with` returns the session's code on `Put::Short` | `broker::session::lifetime::tests::a_standard_error_that_takes_a_piece_short_fails_the_session_however_it_ended` (new) | case `served`, its sink taking 3 bytes: left exit `ExitCode(unix_exit_status(0))`, right `ExitCode(unix_exit_status(1))`, the sink holding the tail's first 3 bytes either way |
| RC: `put` takes a partial write as `Put::Failed` | the two tests above; `pieces::tests::a_piece_is_offered_whole_once_ready_and_never_once_its_attempt_has_ended` (now expects `Short`); `stderr::tests::a_write_taken_short_ends_the_writing_whatever_it_took` | the five failed: `Failed` against `Short` (`pieces/tests.rs:227`); RA's and RB's lefts; `([97, 98, 99], Failed, 1)` against `([97, 98, 99], Short, 1)` (`stderr/tests.rs:176`) |
| RE: `told_to` goes on to the next piece after a short write | `stderr::tests::a_write_taken_short_ends_the_writing_whatever_it_took`; the lifetime test | left `([97, 98, 99, 110, 101, 120], Taken, 2)`, right `([97, 98, 99], Short, 1)`: the sink was offered `next`'s piece; the lifetime test as under RB |
| RF: `Pieces::new` never refuses (`cut(…).unwrap_or_else(\|\| vec![bytes.len()])`) | `pieces::tests::a_frame_holding_512_bytes_with_no_safe_point_cannot_be_cut`; `delivery::an_answer_with_512_bytes_and_no_safe_point_ends_the_session_masked` | left `Some(Pieces { bytes: [120, 49, 49, …` against `None` (`pieces/tests.rs:111`); the harness read the 513-digit answer, where MB4's cause `MCP response cannot be safely masked` was expected (`delivery.rs:224`) |
| RD: escapes untracked (`spanned(…).min(1)`) | `capability_broker::serving::a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed`, case `escaped` | `plain` passed, and `escaped` failed at `serving.rs:625`: the pipe held 499 spaces, then `1\` (`92, 117, 48, 48, 51, 49, 92`) and a newline, a piece cut inside the escape of 2, where 498 spaces and a newline were expected |
| RG: a second `bind` latches nothing | `broker::ledger::tests::a_ledger_is_bound_once_and_a_second_binding_never_weakens_its_guard` (extended) | left `Ok(Clean)`, right `Ok(Failed(Masking))` (`ledger/tests.rs:217`) |

The `stderr::tests` written for the first visit now assert the `Put` that
ended each writing, where they asserted a bool: `Late` for a deadline or a
cancellation, `Failed` for a sink that refused its second write, `Short`
for one that took 3 bytes, and `Taken` for a whole writing.

**L2, a second binding never closes Clean.** The current production path
binds once, through the consuming `Binder`, so a second `bind` cannot
happen in public serving. The ledger now holds the invariant itself. A
refused second binding latches the masking cause, so the ledger can only
close Failed, or not at all where its guard refuses the closure. The
pre-existing follow-up, that an `Ended::Unrecorded(Record)` with nothing
latched could still close Clean, no longer has this path.

**M2, SD4 on Linux x86_64.** The samples were taken in this visit by a
temporary test module, `capability_broker/sd4_scratch.rs`, declared from
`capability_broker.rs`. The test was then removed: the module is moved
out of the tree to `.forge/scratch/` and the declaration deleted. The host
runs Linux 6.17.0-41-generic x86_64 on 24 CPUs. The worktree and `/usr` are
ext4 on the host's LVM root. The binary is the test profile's debug build.
Sources are unpruned, with ordinary protected system hard links in place.
The writer is uid 1000, capless, under `no_new_privs`. Each plan retains
nothing and has an isolated network.

Each sample has two parts. Preparation is timed as the plan's own
observation (`Sealed::observing`, the consumed `broker observe`), the first
actual source observation. Broker startup is timed from spawning `serve`
under the harness shell to reading the harness's first answer, the
initialize result. That covers the second observation (reobservation),
launch, readiness, handoff, exec, the server's initialization and listing,
and the greeting. A watcher thread read the broker's `/proc/<pid>/fd` and
`VmHWM` every millisecond until that answer: the descriptor figure is the
most seen open at once, and `VmHWM` the kernel's own high-water mark.

The cold sample of each profile came after `posix_fadvise(DONTNEED)` on
every regular file under `/usr`, `/etc`, `/opt`, `/var/lib`, the fixture's
root and the broker copy's directory: 272,729 files, in 4.64 s, before the
package profile, and 272,730 in 0.64 s before the system profile. That
evicts their clean pages from the page cache, as far as an unprivileged
process can. It does not drop the dentry and inode caches, and the
observation is mostly metadata. These are therefore page-cache-cold
samples, not fully cold ones. A fully cold sample needs `drop_caches`,
which needs root, and this seat holds none. The system profile's eviction
followed the package profile's samples, and its cold sample was barely
slower than its warm ones.

| Profile | Sample | Preparation (s) | Broker startup (s) | Total (s) | Broker VmHWM (KiB) | Descriptors |
| --- | --- | --- | --- | --- | --- | --- |
| protected package (`opt/docs`), 205,081 entries, 2 mounts | page-cache cold | 1.771 | 2.204 | 3.975 | 26,232 | 51 |
| same | warm 1 | 1.591 | 1.803 | 3.394 | 26,500 | 52 |
| same | warm 2 | 1.558 | 1.494 | 3.052 | 26,316 | 52 |
| same | warm 3 | 1.551 | 1.533 | 3.084 | 26,300 | 52 |
| same | warm 4 | 1.405 | 1.490 | 2.895 | 26,112 | 51 |
| same | warm 5 | 1.456 | 1.438 | 2.894 | 27,332 | 51 |
| singleton system entry (`/usr/bin/dash`), 205,078 entries, 2 mounts | page-cache cold | 1.413 | 1.470 | 2.884 | 25,476 | 50 |
| same | warm 1 | 1.386 | 1.440 | 2.827 | 24,944 | 51 |
| same | warm 2 | 1.319 | 1.472 | 2.792 | 24,324 | 50 |
| same | warm 3 | 1.491 | 1.474 | 2.965 | 25,428 | 50 |
| same | warm 4 | 1.408 | 1.487 | 2.895 | 24,912 | 50 |
| same | warm 5 | 1.421 | 1.433 | 2.854 | 24,504 | 50 |

Every sample exited 0. Each total counts both actual source observations
and is under SD4's 20 s, each observation is under its 10 s, and the 30 s
bound is unchanged. No cache was primed, no source pruned and no timeout
raised for these samples. The fully cold samples, after dropping the
dentry, inode and page caches, stay pending, as do the aarch64 samples for
U9b1.

**M2, the final tree's tests.** Each crate's suite was run on this tree,
crate by crate, each command under `timeout 590`. Six crates ran whole as
`cargo test -p <crate> --locked`. brokkr-runtime ran as `--lib`, then
`--tests --bins --examples`, then `--doc`. brokkr-cli ran as `--lib`, then
`--test it` three times (`capability_broker::`, `machine_proof::`, and the
rest with both skipped), then `--bins` with its five other test binaries,
then `--doc`. No workspace-wide `cargo test` was run, following the
controller's direction.

| Crate | Library tests passed | Other test binaries passed | Doc tests |
| --- | --- | --- | --- |
| brokkr-core | 136 | `it`: 25 | 0 |
| brokkr-store | 97 | `it`: 5, beside four self-spawned runs of one test | 0 |
| brokkr-protocol | 914, 3 ignored | `hands_exits`: 6; `secret_drop`: 1 | 1 |
| brokkr-runtime | 940 | `capability_launch`: 75; `it`: 132, `witness_digests::` among them, unblessed; `operated_repo`: 1; `queued_launch`: 3 | 0 |
| brokkr-view | 310, 3 ignored | `it`: 6 | 0 |
| brokkr-bridge | 17 | none | 0 |
| brokkr-cli | 783, 1 ignored | `it`: `capability_broker::` 92 in 253.7 s, `machine_proof::` 66, and 466 others with 3 ignored; `driver_conformance`: 27; `transcript_surfaces`: 13; `heap_claude`, `heap_codex`, `heap_dsh`: 1 each; the `brokkr` binary's own: 0 | 0 |
| brokkr-seatbelt-probe | none | `seatbelt_lifetime_probe`: 99, 2 ignored | none |

No failure was seen. The `capability_broker::handoff::` tests, the exec
trace's among them, passed in that run.

**16 KiB pipes.** As a temporary local check, not committed, rustix's
`pipe` feature was added to brokkr-cli's manifest. The rpc tests' `pipe()`
and the shared test `sink` were sized to 16,384 bytes with
`fcntl_setpipe_size`, each size asserted. `broker::` lib tests passed 137.
The manifest and `rpc/tests.rs` were then restored with `git checkout` and
`pieces/tests.rs` by hand. `git diff` of the manifest and `Cargo.lock` was
then empty. This is not macOS evidence.

**One CPU.** Not observed. The seat's sandbox refused `taskset -c 0 cargo
test …` again ("What 'taskset' starts can't be checked before it runs").

**Gates on this visit's final tree.** Formatting was checked clean with
`cargo fmt --all -- --check`. Workspace clippy, with all targets and
features, locked and `-D warnings`, finished with no warning. Both
`recipes/self` (47 valuations over 11 groups, 0 unruled) and
`recipes/verify` (16 over 5) compiled. `openspec validate --all --strict`
passed 20 of 20, `typos --hidden` was silent, and `git diff --check` was
clean. In `bash quality/ratchet.sh`, `files` reported "file size holds",
`clones` "duplication holds" and `api` "public API holds". No suppression
was added. `scripts/measure-budgets.sh` was not run, because no prompt,
crate or heap input moved. The rows changed in `quality/file-lines.txt` are
the production files in the table above, plus these test files:
`rpc/tests/delivery.rs` 252 → 301, `session/lifetime/tests.rs` 99 → 141,
`session/stderr/tests.rs` 220 → 225, `capability_broker/serving.rs` 815 →
853 and `broker/ledger/tests.rs` 216 → 222. All stay under 2,000.

## Fifth run: the fourth review's C1, M1, M2 and L1 to L3

The fourth run's head, `8f14934e`, was held on one HIGH, two MEDIUM and
three LOW findings. This run arrived with that run's attempt applied,
uncommitted, on main, and keeps all of it. The operator ruled on
2026-10-14, recorded in `operator-ruling-2026-10-03.md` and in
`design.md`'s U6f1b section, in these words:

> A session's ledger closes Clean only after every byte the broker writes
> on its standard output and its standard error, the server's last lines
> among them, has been taken whole. If a sink takes any piece short before
> then, the ledger closes `Failed(Undelivered)`: brokkr-protocol's ledger
> gains that delivery cause, worded as the broker's own line, "MCP broker
> output could not be delivered", one public entry (1116 to 1117 entries).
> A short write on standard output is recorded with that cause, never as
> `Timeout`. A session that has already failed keeps its first cause, and
> its last line is still written after its closure. A harness that has
> closed its end keeps its existing ending.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-protocol/src/broker/ledger.rs` (621 → 627) | `Failure::Undelivered`, serde `"undelivered"`, whose `#[error]` text, "MCP broker output could not be delivered", is the one source of those words. |
| `crates/brokkr-cli/src/broker/rpc.rs` (772 → 772) | `Fatal::Undelivered` is removed. `UNDELIVERED`, `Fatal::Failed(Failure::Undelivered)`, takes its place, so the broker's line and the ledger's cause are the same variant's words, the way `TIMEOUT` and `MASKING` are. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (751 → 752), existing child | `going` and `drained` end the session with `UNDELIVERED` on `Delivery::Short`. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (273 → 302), existing child | `serve` makes one writing of its standard error, `Tell`, and hands it to `lived` and `Held::reported`. `lived` now runs `Held::delivered` before `settled`: a session that ended served has its server's tail written while the bindings are held and before its ledger is closed. Taken whole, or `Put::Failed`, a standard error that has closed its end, it stays served. Taken short, late or unready, the session ends `UNDELIVERED`, the ledger closes `Failed(Undelivered)`, and `Held::reported` writes nothing more there. A session that failed is settled first and its tail and last line written after, as before. `reported_with` is folded into `reported`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (317 → 320), existing child | Module documentation of the new order. |

No production file is new, and no production file outside the row's eight
and their existing children moved. `crates/brokkr-cli/Cargo.toml` and
`Cargo.lock` are as the fourth run left them: the manifest adds `event`
alone.

**Reading of "taken whole".** The ruling names one cause, for a piece a
sink takes short. On standard output, a frame not taken by its deadline
stays `Timeout`, as `Delivery::Stalled` was. On standard error, which has
no harness deadline of its own, a served session's tail that its fixed
one-second bound does not see taken (`Put::Late`) is not taken whole, so
it cannot close Clean, and it closes `Failed(Undelivered)`, the cause that
says so. A standard error that refuses every write (`Put::Failed`, a
closed reader or an unopenable descriptor) is read as a harness that has
closed its end, which keeps its existing ending, as a closed standard
output does. This is this run's reading, stated in the result's notes.
The operator's ruling after this run replaces it: only a broken pipe is a
closed reader (see the sixth run below).
`capability_broker::serving::a_standard_error_nobody_reads_never_holds_the_sessions_end`
pins it through the public binary: its served case, whose standard error
is a full pipe nobody reads, now exits 1 and closes `Failed(Undelivered)`,
the call succeeded, where the fourth run closed it Clean. It still ends
well within the twenty seconds.

**C1, the tests and their removals.** Each removal compiled, made the
named test fail as shown, and was undone; the tests then passed again.

| Removal | Test | Failure observed |
| --- | --- | --- |
| R1: the old order, `held.delivered(settled(ledger, ended), tell)` in `lived` | `broker::session::lifetime::tests::a_session_closes_clean_only_once_its_standard_error_took_every_piece_whole` (new; replaces the fourth run's exit-only test) | case `served` with a sink taking 3 bytes a write: left exit `ExitCode(unix_exit_status(1))`, the tail's first three bytes and `Some(Clean)`; right the same exit and bytes and `Some(Failed(Undelivered))`. Only the recovered disposition differs. |
| R2: `going` maps `Delivery::Short` to `TIMEOUT` | `broker::rpc::tests::delivery::an_output_that_takes_a_piece_short_ends_the_session_as_a_delivery_failure` (now pins `Undelivered`) | left `(Err(Failed(Failed(Timeout))), [123, 34, 106], 1)`, right `(Err(Failed(Failed(Undelivered))), …)` (`delivery.rs:200`) |
| R3: `drained` maps `Delivery::Short` to `TIMEOUT` | the same test, its new `full` case: the harness has ended its input and the answer waits for an output that is full until a reader drains it 500 ms later | before the case was added, no test in `broker::` failed under R3 (137 passed). With it: left `(true, (Err(Failed(Failed(Timeout))), (Some([123, 34, 106]), 1), Some(Failed(Timeout))))`, right `(true, (Err(Failed(Failed(Undelivered))), …, Some(Failed(Undelivered))))` (`delivery.rs:181`) |

The lifetime test drives `lived`, a real `Ledger` in a canonical temporary
attempt directory, the binder given STATE as an unrelated value, the
server's line kept through the voice, `told_to` into the shared test
`Sink`, then `Held::reported`, and recovers the ledger. Its five cases
assert the exit, every byte the sink took and the recovered closure: served
and taken whole, exit 0, the line `server says` and Clean; served and
taken short, exit 1, the line's first three bytes and
`Failed(Undelivered)`, nothing more written; served to a standard error
that refuses every write, exit 0, nothing and Clean; failed and taken
whole, exit 1, the server's line and then `error: MCP server protocol is
invalid`, and `Failed(Protocol)`, the last line after the closure; failed
and taken short, exit 1, the first three bytes and `Failed(Protocol)`, the
first cause kept.

The cause's words and wire name are pinned in
`capability_broker::ledger::each_ledger_cause_reads_in_cr3s_words` (18
causes) and
`capability_broker::ledger::each_served_cause_closes_a_durable_ledger_first_and_is_never_replaced`,
where `undelivered` is written, read back and recovered from a durable
ledger and a later `Timeout` does not replace it.

**M2, the public API.** `cargo +nightly-2026-09-05 public-api -p
brokkr-protocol -sss --color never`, the command `quality/lib.sh`'s
`api_file` runs, differed from the snapshot by one line, `pub
brokkr_protocol::broker::ledger::Failure::Undelivered`. The snapshot now
holds it: 1,116 lines to 1,117, which `bash quality/ratchet.sh baselines
f6429ec4` counts as 1,115 public items to 1,116, its one raised baseline.
`bash quality/ratchet.sh api` then reported "public API holds". The pull
request names "Ruling: operator ruling 2026-10-14", which approved this
raise.

**L3, the read race.** The operator recorded a third limit on 2026-10-14:

> The broker polls its shared standard input and then reads it without
> changing the shared status flags, so another reader of the same input
> can take the ready bytes first, and that read can then wait past the
> deadline and cancellation, as the ready write can.

It sits beside the write race the operator recorded on 2026-10-13: a write
of at most 512 bytes, made after poll reports the sink ready, can still
wait if another process writes the same pipe at that moment. No status
flag is changed for either.

**M1, SD4's samples.** The fourth run's page-cache-cold and warm samples
above stand for this unit. This run changes how a session ends, not its
startup, so they were not taken again. The operator ruled: "SD4's fully
cold Linux x86_64 sample, taken after dropping the dentry, inode and page
caches, needs root. U6f1b records page-cache-cold and warm samples; the
fully cold sample stays pending, owned by U9b1 beside the aarch64 samples,
where it blocks enablement as SD4 says."

**L2, labelling the server's lines.** Not in this unit. The operator
ruled: "Labelling the server's standard-error lines, so that no server line
reads as the broker's own diagnostic, moves to U6f2." No server line is
labelled here.

**L1, one CPU.** The controller observed on its host, at the fourth run's
head `8f14934e` and pinned to one CPU with `taskset -c 0`, that
`broker::rpc::tests::an_answer_that_with_the_callers_own_id` took 2.90 s
and `broker::rpc::tests::delivery::what_waits_for_the_harness_is_bounded_and_taken_whole_in_order`
14.31 s; at two CPUs the same head passed 49 `secret::`, 3 protocol
`broker::`, 56 `hands::tests::{server,sources,entry}`, 137 CLI `broker::`,
66 `machine_proof::` and 92 `capability_broker::` tests. These are the
controller's observations of the current per-piece writer, not this seat's,
and close L1.

**The public test that hung.** On the controller's host, the fourth run's
first head bound `0012` in
`capability_broker::serving::an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number`,
and that run's random plan digest held `0012`, so the broker refused at its
start with the masking cause, correctly, while `one_write` waited, with no
deadline, on a pipe it held a writer of. Two fixes:

- `one_write` and `flagged` read the broker's answers through
  `heard_within`, which polls the pipe and, once nothing comes for 50 ms,
  checks the harness shell. A shell that has exited, or twenty seconds with
  nothing, fails the test, naming the exit and the broker's standard error,
  now kept in a temporary file for `one_write` and read from its pipe for
  `flagged`. Observed: with the fixture's store temporarily binding `0002`,
  which the fixed owner's attempt and effect ids hold by value, so the
  broker refuses at its start, the test failed in 4.28 s with "the broker
  ended or fell silent: Some(ExitStatus(unix_wait_status(256)))" and
  "error: MCP response cannot be safely masked". The old wait, by the
  controller's report, blocked for 48 minutes on the same kind of early
  end; this seat did not rerun it. The store was then restored to `0012`.
- `protocol::unheld` seals a fixture afresh, up to eight times, until the
  Opened its ledger first writes, the plan's owner and digest, passes the
  final guard's JSON reading for each bound value that the fixed owner
  alone passes. A value the fixed owner holds is refused on every seal
  alike, as a test means it to be, and is left so. Every public test that
  binds a short value through a sealed plan was checked:
  `a_standard_error_that_takes_one_write_holds_only_a_piece_the_guard_passed`
  (`0084726193`, `0012`) and
  `an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number`
  (`0012`) now seal through `unheld`, and so does `protocol::last_line`
  (`error`, `invalid`, `04242424242`, and `0002` beside `error`, whose
  refusal at the Opened by the fixed owner is deliberate). The other bound
  values, `TOKEN` (`u6c8-canary-Qx7Zv`), `value`, `closed` and the ledger
  tests' constants, are either not hex or sealed under fixed digests, and
  cannot coincide by chance.

**Tests moved or changed elsewhere.** `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice`
now pins `stderr::told(` to `serve`, which makes the one writing, where it
pinned `Held::reported`. `session::readiness::tests` hands `lived` a
writing through a new helper, `plan_lived`, which also folds the two
lifetimes the jscpd gate found cloned once the extra argument was added.

**16 KiB pipes.** As a temporary local check, not committed, rustix's
`pipe` feature was added to brokkr-cli's manifest, and the rpc tests'
`pipe()`, the shared test `sink`, `serving.rs`'s `unread`, `one_write`'s
output and `flagged`'s output and error pipes were sized to 16,384 bytes
with `fcntl_setpipe_size`. `broker::` lib tests passed 137 and
`capability_broker::serving::` 12. Every change was then undone by hand; a
search for the marker and the feature found nothing, and `git diff` of the
manifest and `Cargo.lock` was empty. This is not macOS evidence.

**The final tree's tests.** Crate by crate, each under `timeout 590`, as
the fourth run's table above lists them, with these differences in what
passed. brokkr-runtime's library passed 946 where it passed 940, and
brokkr-view's 312 with 3 ignored where it passed 310; this unit touches
neither crate, and the fourth run's table was measured on an older base. brokkr-cli's `it` binary passed
`capability_broker::` 92 in 242.4 s on its second whole run, and 532 others
with 3 ignored, `machine_proof::` 66 among them, run apart with
`capability_broker::` skipped. Every other count, brokkr-core, brokkr-store,
brokkr-protocol (914, 3 ignored, and 1 doc test), brokkr-bridge,
brokkr-seatbelt-probe and brokkr-cli's library (783, 1 ignored) and other
five binaries, is as the fourth run's table records it, observed again on
this tree. `witness_digests::` passed unblessed in brokkr-runtime's `it`.

One failure was seen and did not recur. In the first whole run of
`capability_broker::` on the final tree,
`capability_broker::a_plan_under_an_unprotected_root_or_file_is_unbound`
failed at `capability_broker.rs:648`, the assertion that a bound plan's
answer is the retained plan's refusal; its left and right were not
captured. It passed alone in 9.35 s, and the next whole run passed 92 of
92 in 242.4 s. That test's path, a retained plan refused before any MCP
message, is settled before its ledger closes and writes after, as the
fourth run did. It is recorded as a follow-up.

**Gates on this run's final tree.** Formatting was checked clean;
workspace clippy, with all targets and features, locked and `-D warnings`,
finished with no warning. `recipes/self` (47 valuations over 11 groups, 0
unruled) and `recipes/verify` (16 over 5) compiled. `openspec validate
--all --strict` passed 26 of 26, `typos --hidden` was silent, and `git
diff --check` and `git diff --cached --check` were clean. In `bash
quality/ratchet.sh`, `files` reported "file size holds", `clones`
"duplication holds" after the readiness fold, and `api` "public API holds".
No suppression was added. `scripts/measure-budgets.sh` was not run: no
prompt, crate or heap input moved, and the three heap tests passed their
budgets. The test rows changed in `quality/file-lines.txt` are
`rpc/tests/delivery.rs` 301 → 344, `session/lifetime/tests.rs` 141 → 202,
`session/readiness/tests.rs` 1,041 → 1,052, `capability_broker/ledger.rs`
1,025 → 1,030, `capability_broker/protocol.rs` 472 → 513,
`capability_broker/serving.rs` 853 → 888 and `machine_proof/output_edge.rs`
187 → 190, all under 2,000. A workspace-wide `cargo test` was not run: the
crate-by-crate runs above together take longer than one command's ten
minutes in this seat.

## Sixth run: the fifth review's H1, M1, L1 and two flakes

The fifth run's head, `76db3f1e`, was held on one HIGH, one MEDIUM and one
LOW finding. This run arrived with that attempt applied, uncommitted, on
main, and keeps all of it. The operator ruled on 2026-10-14, after the
fifth run, in these words, recorded in `operator-ruling-2026-10-03.md`:

> only a harness that has closed its end, seen as a broken pipe, keeps its existing ending. Every other failure to deliver, on either stream, is a delivery failure, recorded as `Failed(Undelivered)` unless an earlier cause stands: a write or poll error, such as a full device or an I/O error; a write that takes nothing; and a stream that cannot be set up.

The controller observed on its host, at two CPUs, that `76db3f1e` passed 49
`secret::`, 3 protocol `broker::`, 56 `hands::tests::{server,sources,entry}`,
137 CLI `broker::`, 66 `machine_proof::` and 91 of 92 `capability_broker::`
tests, the trace control the one failure, which passed three times alone;
that the plan-root test passed two whole runs there; and, at one CPU, that
the MB3-bound rpc test took 2.94 s and `delivery::what_waits…` 14.27 s.
These are the controller's observations, not this seat's.

**H1, closed reader against delivery failure.** The fifth run's reading
above, that every refused write is a closed reader, is replaced.

| File (lines before → after) | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker/rpc/pieces.rs` (177 → 199), existing child | `Put::Closed` is new. `put` observes, and the new `written` decides: poll's `Ok(false)` and a retried error (EINTR, EAGAIN) are unready, as before; any other poll error is `Failed`, never closure, and no write follows it; the write's own EPIPE alone is `Closed`; any other write error, ENOSPC and EIO among them, and a write that took nothing, are `Failed`; part of a piece is `Short`. |
| `crates/brokkr-cli/src/broker/rpc/calls/outbox.rs` (83 → 85), existing child | `Delivery::Short` becomes `Delivery::Undelivered`, which `Put::Failed` and `Put::Short` both reach; only `Put::Closed` is `Gone`. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (752 → 753), existing child | `going` and `drained` read `Undelivered` as the delivery failure; their documentation names the new cases. A session that already failed still keeps its cause: `drained`'s later result is dropped, as before. |
| `crates/brokkr-cli/src/broker/rpc.rs` (772 → 790) | The harness's streams are set up by `held_apart` and handed to `initialized` as a closure (`Stdio`), so a standard output that cannot be held apart is `UNDELIVERED`; a standard input that cannot stays the protocol's cause. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (302 → 307), existing child | `Held::delivered` keeps a served ending on `Taken` or `Closed` alone; `Failed`, `Short`, `Late` and `Unready` end it `UNDELIVERED`. `Held::reported` fails the exit on `Failed` as on `Short`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (320 → 335), existing child | `told` sets up its descriptor and hands it to the new `told_through`, which is `Failed` where that could not be set up and there is something to write; `told_to` returns `Closed` like any other end. |

No production file is new and none outside the row and its existing
children moved; `Cargo.toml` and `Cargo.lock` are unchanged by this run, and
`bash quality/ratchet.sh api` reported "public API holds" with
`quality/public-api/brokkr-protocol.txt` at 1,117 lines.

**The tests and their removals.** Each removal compiled, made the named
test fail at the assertion shown, and was undone; the test then passed.

| Removal | Test | Failure observed |
| --- | --- | --- |
| S1, standard output: `Put::Failed => Delivery::Gone` in `Outbox::flushed` | `broker::rpc::tests::delivery::only_a_closed_harness_output_keeps_a_sessions_ending` (new): EPIPE, ENOSPC, EIO and a zero write, each the exit, the bytes taken and the recovered closure | left `("ENOSPC", Ok(()), Some([]), 0, Some(Clean))`, right `("ENOSPC", Err(Failed(Failed(Undelivered))), Some([]), 0, Some(Failed(Undelivered)))` |
| S1 | `capability_broker::serving::a_standard_output_on_a_full_device_fails_the_session_undelivered` (new, public, Linux) | left `(Some(0), "")`, right `(Some(1), "error: MCP broker output could not be delivered\n")` |
| S2, standard error: `Put::Failed` accepted in `Held::delivered` | `broker::session::lifetime::tests::only_a_closed_standard_error_keeps_a_served_sessions_ending` (new): EPIPE, ENOSPC, EIO, a zero write and a failed set-up, served and failed | left `("ENOSPC", ExitCode(unix_exit_status(0)), "", Some(Clean))`, right `("ENOSPC", ExitCode(unix_exit_status(1)), "", Some(Failed(Undelivered)))` |
| S2 | `capability_broker::serving::a_standard_error_on_a_full_device_fails_a_served_session_undelivered` (new, public, Linux) | left `(Some(0), 3, "")`, right `(Some(1), 3, "")` |
| S3, a poll error read as closure: `Err(_) => return Put::Closed` for the poll in `written` | `broker::rpc::pieces::tests::only_a_broken_pipe_the_write_meets_is_a_closed_reader` (new) | left `(Os { code: 32, kind: BrokenPipe, … }, Closed, 0)`, right `(…, Failed, 0)`: poll's own EPIPE is no closure, and no write was made |
| S4, standard output set-up as the protocol's cause in `initialized` | `broker::rpc::tests::delivery::a_harness_output_that_cannot_be_set_up_ends_the_session_undelivered` (new) | left `("output", Err(Failed(Failed(Protocol))), Some(Failed(Protocol)))`, right `("output", Err(Failed(Failed(Undelivered))), Some(Failed(Undelivered)))` |
| S5, standard error set-up as closure: `(false, Err(_)) => Put::Closed` in `told_through` | `only_a_closed_standard_error_keeps_a_served_sessions_ending` | left `("set-up", ExitCode(unix_exit_status(0)), "", Some(Clean))`, right `("set-up", ExitCode(unix_exit_status(1)), "", Some(Failed(Undelivered)))` |
| S6, a later delivery failure replaces the first cause in `served_within` | `broker::rpc::tests::delivery::an_earlier_cause_stands_where_the_harness_output_then_fails` (new): an answer with no safe point ends the session masked, and the output then refuses with ENOSPC | left `(Err(Failed(Failed(Undelivered))), (Some([]), 0))`, right `(Err(Failed(Failed(Masking))), (Some([]), 0))` |

Changed beside them: the shared test `Sink` refuses with an `Errno`, the
system's own error, and `refusing` builds one that refuses every write;
`a_piece_is_offered_whole_once_ready…` now expects EPIPE `Closed`, ENOSPC
and EIO `Failed` and a zero write `Failed`;
`a_line_one_write_cannot_take_is_cut_only_at_its_safe_points` expects
`Closed` where its sink's second write meets a broken pipe; the retained
plan's delivery test hands `initialized` a set-up that panics if called, so
its harness's streams are still never taken. A poll error cannot be made to
happen on a real descriptor; it is bound in `written`, the decision `put`
calls, and its `Failed` reaches each stream's ending through the S1 and S2
paths above. The public `/dev/full` tests run where this suite does, which
`capability_broker.rs` declares `#[cfg(target_os = "linux")]`; macOS has no
`/dev/full`. `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice`
now pins `told_through(` to `told`, `told_to(` to `told_through`,
`std::io::stdin(`/`stdout(` to `held_apart`, and `, held_apart)` to
`serving`.

**M1, every public wait bounded.** In `capability_broker/serving.rs`,
`heard_within` reads the absolute twenty-second deadline before every read,
so bytes that never hold a newline cannot outrun it, and reads a pipe to its
end where it is asked for no number of lines; `one_write` reads to that end
through it instead of `read_to_end`. A failure names the harness shell's
status, the bytes heard and the broker's standard error. `flagged` waits
for the broker's exit through the new `exited_within`, twenty seconds, then
kills the shell and its child, the broker, and fails naming both. In
`capability_broker/protocol.rs`, every `Talk` has a watchdog that kills the
shell and the broker after 90 s, past the startup's 30 s and the 31 s one
test idles, so each `read_line`, the read to the output's end and the wait
for the exit are bounded; a killed broker reads as exit `None` beside its
captured standard error. No test was added for the helpers themselves;
`capability_broker::` passed 94 of 94 with them.

**L1, the broker's documentation.** `crates/brokkr-cli/src/broker.rs`
(118 → 120) no longer says all standard error follows the ledger's
closure: a served session's server tail is written before the ledger
closes, Clean only once taken, and a failed session's tail and last line
after, its first cause kept, with `session::lifetime` named as the one
description of that order.

**The trace flake.** In
`capability_broker::handoff::a_protected_installed_server_alone_receives_its_bindings_beside_its_fixed_environment`,
the plan was the retained one: the broker refuses SD3's cause as soon as it
has seen the server's exec, and tears the box down. So the server can be
killed between its successful exec and the tracer's report of that exec's
return or of its C library's `set_tid_address`, which leaves the execve
line ending `= ?` and nothing after it, as the controller saw; the answer
`admitted()` it matched was that refusal, which the server takes no part
in. The test now seals a plan that retains nothing, so the server is
initialized by its own MCP answer and lives until its harness, whose input
`traced_serve` now ends at once, has gone; the answer is `(Some(0), "")`.
`exec_succeeded` accepts `= ?` where `set_tid_address` follows, as before,
or where `answered`: the call is the server's one traced exec and that
initialization succeeded. `= 0` stays accepted. The removal `answered &
false` made `an_exec_whose_return_the_tracer_lost_succeeded_only_where_its_program_went_on`
fail with left `("lost, answered", false)`, right `("lost, answered",
true)`; that test also now pins that a refused exec is never accepted
however the server answered. Its two other callers pass `false`.

**The plan-root failure, investigated.** `capability_broker.rs:648` is
`binds`' live-box assertion that a restored plan's answer is `admitted()`,
the retained plan's refusal. Its cause was not established, because the
fifth run did not keep the assertion's left and right and no run since has
failed: two whole runs here passed 94 of 94, in 239.2 s before this run's
last test-only edits outside `capability_broker::` and in 232.6 s on the
final tree. What was ruled out, by reading the fixture: run 5's
short-value collision cannot happen, since `Sealed::new` stores only
`DOCS_TOKEN=u6c8-canary-Qx7Zv`, seventeen bytes no hex digest or the fixed
owner can spell; nor can a ledger left by an earlier serve, since
`serve_in` removes it first; and every chmod, link and rename the test
makes is restored before its `binds` call. What remains is the one
timing-dependent part of that answer, the startup the box needs before the
broker sees the exec and refuses, under a whole suite's concurrent boxes;
this seat could not reproduce that contention (running the test eight
times at once, or the suite pinned to four CPUs with `taskset`, was not
permitted here). The fifth run's follow-up stands, now with this
narrowing. `assert_eq!` prints both sides, so the next failure's log will
show the answer.

**The final tree's tests**, each crate on its own under `timeout 590`:

| Crate, target | Passed |
| --- | --- |
| brokkr-core | lib 136, tests 25, doc 0 |
| brokkr-store | lib 97, `it` 5 (and four self-spawned singles), doc 0 |
| brokkr-protocol | lib 914 (3 ignored), `hands_exits` 6, `secret_drop` 1, doc 1 |
| brokkr-runtime | lib 946, `capability_launch` 75, `it` 132 (`witness_digests::` among them, unblessed), `operated_repo` 1, `queued_launch` 3, doc 0 |
| brokkr-view | lib 312 (3 ignored), tests 6, doc 0 |
| brokkr-bridge | lib 17, doc 0 |
| brokkr-seatbelt-probe | `seatbelt_lifetime_probe` 99 (2 ignored) |
| brokkr-cli | lib 788 (1 ignored), `broker::` 142 among them; `it` without `capability_broker::` 532 (3 ignored), `machine_proof::` 66 among them; `it capability_broker::` 94 in 232.6 s; `driver_conformance` 27, `transcript_surfaces` 13, `heap_claude` 1, `heap_codex` 1, `heap_dsh` 1, doc 0 |

A workspace-wide `cargo test` was not run as one command; the crate runs
above cover every target. The test rows changed in `quality/file-lines.txt`
are `rpc/pieces/tests.rs` 282 → 348, `rpc/tests/delivery.rs` 344 → 464,
`session/lifetime/tests.rs` 202 → 254, `capability_broker/handoff.rs` 927 →
948, `capability_broker/protocol.rs` 513 → 550, `capability_broker/serving.rs`
888 → 979 and `machine_proof/output_edge.rs` 190 → 197, all under 2,000.
`rpc/tests/delivery.rs`'s in-memory ledger is one helper, `in_memory`,
after the clone gate found it written twice.

**Gates on this run's final tree.** Formatting was clean, and workspace
clippy with all targets and features, locked and `-D warnings`, finished
with no warning. `recipes/self` compiled with 47 valuations over 11 groups,
0 unruled, and `recipes/verify` with 16 over 5. `openspec validate --all
--strict` passed 26 of 26; `typos --hidden` was silent; `git diff --check`
and `git diff --cached --check` were clean. `bash quality/ratchet.sh files`
reported "file size holds", `clones` "duplication holds" and `api` "public
API holds". No suppression was added and no function grew past clippy's
ceilings. No witness or budget input moved, so neither was measured again;
the three heap tests passed their budgets. SD4's page-cache-cold and warm
samples recorded under the fourth run stand: this run changes how output
is delivered, not the startup.

## Seventh run: the sixth review's SC-M1, SC-M2 and SC-L1

The review of `6601dfe1` kept the sixth run's repairs. It returned two
MEDIUM findings and one LOW finding to this seat. SEC-L1, labelling the
server's standard-error lines, stays with U6f2 by the operator's ruling of
2026-10-14. CH-L1 concerned the review panel's own prose and asks nothing of
this code. The INFO, that a read or poll error on standard input reads as
the input's end, predates this unit and is listed under the follow-ups.

**SC-M1, the first cause after a harness answer.** A local answer such as
a ping's flushed the harness's output, which could find it failed and set
`Delivery::Undelivered`. `requested` still returned success, so the same
turn went on to hear the server, and a server error heard there became the
session's cause. In `crates/brokkr-cli/src/broker/rpc/calls.rs` (753 → 757
lines), `serving` now reads `going` again after a harness frame is answered
and before the server is heard. A delivery failure found while answering
therefore ends the session as `UNDELIVERED` before another event is
processed. A closed reader still leaves the loop as served. The new test
`broker::rpc::tests::delivery::a_delivery_failure_found_answering_the_harness_stays_the_first_cause`
binds both orders against a recovered ledger. In the first case, the
server's stray response arrives in the same write as its listing, so it is
already buffered when the session starts, and the harness's ping meets
ENOSPC. In the second, the server answers a call by a stray id, and the
frame that tells the harness is then refused with ENOSPC.

| Removal | Failure observed |
| --- | --- |
| R1: the new `going` check in `serving` deleted | left `("delivery first", Err(Failed(Failed(Protocol))), Some(Failed(Protocol)))`, right `("delivery first", Err(Failed(Failed(Undelivered))), Some(Failed(Undelivered)))` |
| R2: `served_within`'s failed path returns `drained`'s later cause (`.ok()` → `?`) | left `("server first", Err(Failed(Failed(Undelivered))), Some(Failed(Undelivered)))`, right `("server first", Err(Failed(Failed(Protocol))), Some(Failed(Protocol)))` |

Each removal compiled and was undone. The test then passed, as did the
other eleven `delivery::` tests.

**SC-M2, every public wait bounded.** `crates/brokkr-cli/tests/capability_broker.rs`
gains `bounded`. It reads a child's piped standard output and error on
threads of their own from the start, writes the given input and holds it
open, and waits for the exit. Every wait falls within `TALK_MAX`, 90 s,
which moves here from `capability_broker/protocol.rs` together with
`killed`; `protocol.rs` now uses both from the root. A child that has not
ended by then is killed with its children, and the test fails naming the
status and the standard error read so far. A child that has ended, but
whose streams something left behind holds open, fails the same way, and
its reaped process id is not killed. These waits now go through `bounded`:

- `Root::ran`, which `Sealed::serve` and every serve helper reach. It used
  `Command::output()`.
- In `handoff.rs`: `traced_serve` (`status()`), the stdio test's
  `attempt.wait()`, `Handing::ended` and the terminal test's `run.wait()`.
- `readiness.rs`'s `Held::ended` (`wait_with_output()`).
- In `observer.rs`: the three serves of the blocked-observation tests, the
  launcher trace's `status()`, and the observe command's `status()` and two
  `output()`s.

`serving.rs`'s `/dev/full` standard-output test now runs through `bounded`,
so `(read once it ends)` is gone and a timeout reports the captured
standard error.

Two checks show that the bounds hold. The first is a production removal:
with `STARTUP` in `broker/session.rs` raised from 30 s to 3,600 s,
`capability_broker::protocol::a_server_that_never_answers_ends_at_the_startup_deadline`
failed after 91.55 s at `capability_broker.rs:245` with `the broker did not
end: Ok(ExitStatus(unix_wait_status(35072)))`, where it would otherwise have
waited an hour. The removal was then undone. The second is a scratch check
that was not committed. A test ran `bounded` on `sh -c 'echo said >&2; sleep
200 & exit 0'`, whose orphan holds standard output, and failed after 90.00 s
with `the broker's output did not end: ExitStatus(unix_wait_status(0))`
followed by `said`.

**SC-L1, the plan-root failure.** The fifth run's transcript on this host
holds the failure as it was seen: `(307018) panicked at
crates/brokkr-cli/tests/capability_broker.rs:648:17`, in a whole run that
passed 91 and failed 1 in 219.15 s at 2026-10-14T09:12:49Z. That seat's own
`grep` filtered out the assertion's left and right, and the commands it ran
just before were sequential, so no other run in that seat overlapped it.
This seat tried to reproduce the failure on the final tree:

| Run | Result |
| --- | --- |
| whole `capability_broker::` | 94 of 94, 261.65 s |
| with `--test-threads 32` | 94 of 94, 232.26 s; the suite's `BOXES` cap of three live boxes still applied |
| `BOXES` raised to 12 for the run, `--test-threads 24` | 93 of 94, 90.05 s; the one failure was another test, see below |
| `BOXES` raised to 32, `--test-threads 48` | 94 of 94, 69.68 s |
| two scratch tests, each running the plan-root test 15 times, beside the whole suite | 96 of 96, 401.35 s |

None of these failed the plan-root test, which ran at least 35 times. This
seat was not permitted to pin CPUs with `taskset` or to run test processes
concurrently. The cause therefore stays unestablished. Two hypotheses
remain, and neither is confirmed. The first: this test serves one fixture
again and again without waiting for the last box to be gone, unlike the
tests that check `boxed(&sealed)`, and the session's cleanup is U6f2's to
prove. The second: another lane's suite contends on the same host, beyond
the per-process `BOXES` cap. No fixture change was made, since none is
supported by evidence. `assert_eq!` prints both sides, and `bounded` now
reports a hang with its standard error, so the next failure's log will
carry the answer. The scratch tests and the raised caps were removed. The
`BOXES = 12` run surfaced a different flake:
`capability_broker::observer::the_admitted_launcher_runs_through_its_handle_with_no_environment`
read a second trace line, `???( <detached ...>`, after the launcher's
`execve`. It is listed under the follow-ups.

**The final tree's tests**, each crate on its own under `timeout 590`.
brokkr-core passed lib 136 and tests 25. brokkr-store passed lib 97 and
`it` 5, with four self-spawned singles. brokkr-protocol passed lib 914 (3
ignored), `hands_exits` 6, `secret_drop` 1 and doc 1. brokkr-runtime passed
lib 946, `capability_launch` 75, `it` 132, `operated_repo` 1 and
`queued_launch` 3. brokkr-view passed lib 312 (3 ignored) and tests 6, and
brokkr-bridge lib 17. brokkr-seatbelt-probe's `seatbelt_lifetime_probe`
passed 99 (2 ignored). brokkr-cli's lib passed 789 (1 ignored), 143
`broker::` tests among them. Its `it` binary passed 532 (3 ignored) with
`capability_broker::` skipped, and `it capability_broker::` 94 in 271.79 s.
`driver_conformance` passed 27, `transcript_surfaces` 13, and each heap test
1.

**Gates.** `cargo fmt --all -- --check` was clean after `cargo fmt` had
reflowed the new test. Workspace clippy with all targets and features,
locked and `-D warnings`, finished with no warning. `recipes/self` compiled
with 47 valuations over 11 groups, 0 unruled, and `recipes/verify` with 16
over 5. `openspec validate --all --strict` passed 26 of 26, `typos --hidden`
was silent and `git diff --check` was clean. In `quality/ratchet.sh`,
`files` held, `clones` held and `api` held, with brokkr-protocol still at
1,117 entries. The `quality/file-lines.txt` rows changed are `rpc/calls.rs`
753 → 757, `rpc/tests/delivery.rs` 464 → 510, `tests/capability_broker.rs`
1,870 → 1,966, `capability_broker/observer.rs` 975 → 980 (the old row was
already stale), `capability_broker/protocol.rs` 550 → 536 and
`capability_broker/serving.rs` 979 → 973. No production file is new.
`Cargo.toml`, `Cargo.lock` and the frozen paths did not move, and no
witness or budget input changed.

## Eighth run: the seventh review's SC-M1, SC-L1 and L1 to L3

The review of `c72bfe7f` kept the earlier repairs: only EPIPE on a write
keeps a closed reader's ending, every other delivery error is `Undelivered`
on both streams, the first cause stands, and the API holds at 1,117
entries. It returned one MEDIUM and four LOW findings.

**SC-M1, the protocol suite's waits.** `capability_broker/protocol.rs`'s
`Talk` had relied on one watchdog thread that killed the harness shell and
its children after `TALK_MAX`. Its EOF wait was an unbounded
`read_to_string`, and its standard-error thread was joined without a bound.
A writer that a descendant held open could therefore hang the test after the
child had exited. `Talk` now reads its standard output and error apart from
the start, using the root's `apart`. It has no watchdog. `Talk::heard_to`
serves both the first frames and `asked`: it sets an absolute deadline of
`max` from its start and reads it again on every look, before checking for a
newline, so bytes that never end a line cannot pass it. `Talk::ended` drops
the input and waits for the exit within a fresh absolute deadline. It then
waits for the end of each stream within what remains of that deadline. Each
failure goes through `Talk::failed`, which kills the shell and its children
only if they have not been reaped, and names the status and the standard
error captured so far. `TALK_MAX`, 90 s, stays the public tests' bound.

**SC-L1, the redirected standard error.** In the root,
`bounded(child, said)` now calls `bounded_by(child, said, (max, file))`.
There is also a new `bounded_apart(child, said, apart)`, which every wait
uses once its starter has sent the broker's own standard error to
`broker.err`: `traced_serve`, the stdio test, `Handing::ended` and the
terminal test in `handoff.rs`; `Held::ended` in `readiness.rs`; and the
launcher trace and the three blocked-observation tests in `observer.rs`.
`observer.rs` gains `apart(sealed)` as the one home of that path. A timeout
now names the file's contents after the piped standard error.

The new test
`capability_broker::protocol::every_wait_of_a_public_test_fails_within_its_bound_naming_the_status_and_stderr`
binds both repairs with a bound of one second. In each case a shell says
`said` on its standard error. The cases are a stream of bytes with no
newline (`while :; do printf x; done`, one frame expected), a shell that
never ends (`exec sleep 30`), and a shell that exits while an orphaned
`sleep 30` holds its standard output, or else its standard error. Each must
panic within ten seconds with the exact message, for example `the broker
said too little within its bound: ExitStatus(unix_wait_status(9))` followed
by `said`. Every shell runs in a group of its own, so the test then kills
what it left behind. A last case runs `bounded_by` on a shell that writes
`told` to a redirected file and never ends. It must name `said` and then
`told`. No shell that is killed runs a child of its own, so its status is
its own kill. A first draft used `sleep 30` without `exec`, and dash then
reported `Killed` with 35072 (exit 137) before its own kill.

| Removal | Failure observed |
| --- | --- |
| `heard_to`'s deadline check made unreachable (`&& false`) | the test hung until `timeout 150` killed it, exit 124 |
| `ended`'s stream wait given 60 s more than the deadline | `protocol.rs:375`, `called Result::unwrap_err() on an Ok value: ()` after 32.06 s: the stdout case waited for the orphan |
| `bounded_by`'s failure drops the file's contents | left `("the broker did not end: ExitStatus(unix_wait_status(9))\nsaid\n", true)`, right `(…\nsaid\ntold\n", true)` |
| `Talk::failed` drops the standard error | left `("bytes", ("the broker said too little within its bound: ExitStatus(unix_wait_status(9))\n", true))`, right `(…\nsaid\n", true))` |

Each removal compiled and was undone, and the test then passed alone in
5.09 s.

**L1, the delivery cause's documentation.** The doc comments on
`Failure` and `Failure::Undelivered` in `brokkr-protocol/src/broker/ledger.rs`
(627 → 631 lines) had garbled prose and named only a short take. They now
state the October 14 ruling: output that was not delivered whole for any
reason but a closed reader. That covers a piece taken short or not at all,
a write or poll error, and a stream that could not be set up. A broken pipe
keeps the existing ending. `broker/rpc.rs` (790 → 792) says the same for
`UNDELIVERED` and `Fatal`. Only comments changed, and `ratchet.sh api` held.

**L2, standard input held apart, is not changed.** When `held_apart`
cannot duplicate the broker's standard input, `initialized` ends the
session with `Failure::Protocol`, whose words blame the server. Every
existing cause names something else. The ledger's output cause covers
output, and the October 14 ruling's "either stream" names standard output
and standard error. An honest cause would be a new `Failure` variant, which
is a new public entry, and this row may add none. It is listed under the
follow-ups.

**L3, the plan-root failure.** This run did not establish the cause, and it
claims no repair. The whole `capability_broker::` suite passed 95 of 95 in
238.55 s on the final tree, the plan-root test among them. The seventh
run's record of the failure above stands as it was written.

**The final tree's tests**, each crate on its own under `timeout 590`.
brokkr-core passed lib 136 and `it` 25. brokkr-store passed lib 97 and `it`
5, with four self-spawned singles of one each. brokkr-protocol passed lib
914 (3 ignored) in 66.17 s, `hands_exits` 6, `secret_drop` 1 and doc 1.
brokkr-runtime passed lib 946, `capability_launch` 75, `it` 132,
`operated_repo` 1 and `queued_launch` 3. brokkr-view passed lib 312 (3
ignored) and `it` 6, brokkr-bridge lib 17, and brokkr-seatbelt-probe's
`seatbelt_lifetime_probe` 99 (2 ignored). brokkr-cli's lib passed 789 (1
ignored). Its `it` binary passed 532 (3 ignored) with `capability_broker::`
skipped, and `it capability_broker::` passed 95. `driver_conformance` passed
27, `transcript_surfaces` 13, and `heap_claude`, `heap_codex` and
`heap_dsh` 1 each.

**Gates.** `cargo fmt --all -- --check` was clean, and so was workspace
clippy with all targets and features, `--locked` and `-D warnings`.
`recipes/self` compiled with 47 valuations over 11 groups, 0 unruled, and
`recipes/verify` with 16 over 5. `openspec validate --all --strict` passed
26 of 26. `typos --hidden` was silent, and so was `git diff --check`.
`quality/ratchet.sh` held `files`, `clones` and `api`. `cargo doc` with
private items gave no warning for either edited production file. The
`quality/file-lines.txt` rows changed are `broker/rpc.rs` 790 → 792,
`broker/ledger.rs` 627 → 631, `tests/capability_broker.rs` 1,966 → 1,981,
`capability_broker/handoff.rs` 948 → 951, `capability_broker/observer.rs`
980 → 987, `capability_broker/protocol.rs` 536 → 658 and
`capability_broker/readiness.rs` 313 → 314. No file is new, and
`Cargo.toml`, `Cargo.lock` and the frozen paths did not move.

## Re-vouch: CI's inherited pipes on ubuntu

PR #676's ubuntu test job and exact-coverage gate both failed at
`hands::tests::server::a_process_is_read_once_it_has_settled`. The shell
it measured held two pipes (`142` and `145`) besides its three
`/dev/null` streams. It had inherited them from other tests running at the
same time in the same binary. The ledger tests' `swap` used `dup2`, whose
new descriptor is not close-on-exec, to put a pipe in place of the ledger
file. And the secret test
`every_descriptor_above_the_standard_streams_is_sealed_before_the_exec_or_nothing_starts`
clears close-on-exec on a pipe on purpose. Production is not involved,
because the box's children seal every descriptor above the standard streams.
This visit changes test files only, adds no public entry, and leaves
`Cargo.lock` alone.

**The leak's source.** `swap` in `broker/ledger/tests.rs` now uses
`rustix::io::dup3(medium, &mut held, DupFlags::CLOEXEC)`. It then asserts
that the swapped descriptor reads `FdFlags::CLOEXEC`, so no child another
test starts can inherit it. Each ledger case proves what it proved before.
The secret test's deliberate inheritable pipe stays as it is, because it
is that test's subject.

**Immunity in each exact-descriptor test.** Two tests compare a process's
whole descriptor table. `a_process_is_read_once_it_has_settled` now starts
its shell with a `pre_exec` that makes one `close_range` call over every
descriptor from 3 up. The call is the crate's own
`secret::handoff::close_on_exec_above_stdio`, so the shell holds only what
its script opens. The task offered the closing form of `close_range` or a
baseline comparison. The close-on-exec form was chosen because the
sealing function already exists and is async-signal-safe (ruling 5). It
also leaves std's own exec-status pipe open until the exec, which the
closing form would shut early. Once the exec runs, the descriptor table
is the same either way. The exact assertion is kept, where a baseline
comparison would have weakened it. `a_server_box_stands_without_the_seats_or_the_hosts_private_paths`
measures a process inside the box, whose launch hook (`inherit` in
`hands/namespace/entry.rs`) already seals strays. That test needed no
isolation of its own, only the control below.

**The control.** A new helper, `holding_a_stray`, opens a pipe in the test
process and clears close-on-exec on both of its ends. It runs a start
while it holds them, closes them, and returns what `/proc` links them to.
Both exact-descriptor tests now start their measured process inside it.
Each test's assertion message names the stray. So every run of the two
tests holds an inheritable stray across the start, and both passed in all
of this visit's runs.

**Removals, each compiled, run, observed and restored.**

| Removal | Test that failed | What it showed |
| --- | --- | --- |
| The `pre_exec` seal in the settled test made a no-op (`true \|\| close_on_exec_above_stdio()`) | `a_process_is_read_once_it_has_settled` | `this process's stray: pipe:[73729526]`, left held `(4, "pipe:[73729526]")` beside the three `/dev/null` streams |
| The launch hook's seal in `inherit` made a no-op, in production, temporarily | `a_server_box_stands_without_the_seats_or_the_hosts_private_paths` | `this process's stray: pipe:[73729554]`, left held `(21, "pipe:[73729554]")` and `(30, "pipe:[73729554]")` beside the expected six |
| `swap`'s `DupFlags::CLOEXEC` replaced by `DupFlags::empty()` | `each_record_is_written_whole_then_synced_before_the_writer_answers` | left `Ok(FdFlags(0x0))`, right `Ok(FdFlags(CLOEXEC))` |

All three were restored, and `git diff` confirmed that `entry.rs` matched
its staged state afterwards. The first two removals were first run against
an earlier shape of the control, before `holding_a_stray` existed, and
failed the same way with other inode numbers. They were run again on the
final shape, which is what the table records.

**Whole-crate runs, as CI runs them.** `cargo test -p brokkr-protocol --lib`
ran whole on the final tree five times, and each run passed 914 with 3
ignored. Three ran at the default 24 test threads (64.14 s, 68.39 s and
69.98 s), one at `--test-threads=4`, the width of CI's runners
(161.39 s), and one at `--test-threads=2` (236.93 s). `taskset -c 0,1`
was refused by this seat's sandbox, so load came from thread counts
instead. Runs that oversubscribed this 24-core host, at 48 and 64 threads,
failed only `process::tests` wall-clock bounds:
`a_driver_that_stops_reading_cannot_hold_the_shutdown_write` (3.06 s and
3.37 s against 3 s) and `a_grandchild_holding_the_pipes_does_not_outlast_the_deadline`
(5.56 s against 5 s). Those overruns are milliseconds. An inherited pipe
would have held the tests until their 10 s or 600 s sleeps ended. Neither
test is touched here, and both are listed under the follow-ups. The same
two descriptor tests passed in every one of those runs.

**The final tree's tests**, each crate on its own under `timeout 590`.
brokkr-protocol passed `hands_exits` 6, `secret_drop` 1 and doc 1 besides
the runs above. brokkr-core passed lib 138 and `it` 26. brokkr-store passed
lib 97 and `it` 5, with four self-spawned singles of one each. brokkr-view
passed lib 312 (3 ignored) and `it` 6. brokkr-bridge passed lib 17, and
brokkr-seatbelt-probe's `seatbelt_lifetime_probe` passed 99 (2 ignored).
brokkr-runtime passed lib 964, `capability_launch` 75, `it` 132,
`operated_repo` 1 and `queued_launch` 6. brokkr-cli passed lib 809 (1
ignored), `it` 652 (3 ignored, `capability_broker::` included),
`driver_conformance` 27, `transcript_surfaces` 13, and `heap_claude`,
`heap_codex` and `heap_dsh` 1 each.

**Gates.** Formatting and workspace clippy, with all targets and features,
`--locked` and `-D warnings`, were clean. The box test first went over
clippy's 100-line limit, at 102 and then 101 lines. Its info pipe is now
read in one line with `std::io::read_to_string`, which brings it back to
the limit, and no suppression was added. `recipes/self` compiled with 47
valuations over 11 groups, 0 unruled. `openspec validate --all --strict`
passed 26 of 26, and `typos --hidden` and `git diff --check` were silent.
`quality/ratchet.sh files` and `clones` held. Two `quality/file-lines.txt`
rows changed, `hands/tests/server.rs` from 974 to 1,005 and
`broker/ledger/tests.rs` from 222 to 225. Both are test files under the
2,000-line ceiling.

## Second re-vouch: the exact-coverage gate's misses

CI's exact gate on PR #676 at d5a9ac3f counted six lines, two branch arms
and one function short, all in this unit's own code. This visit closes
each one. No behaviour, ruling or public entry changes (the brokkr-protocol
API stays at 1,117 entries), and `Cargo.lock` does not move.

**Poll's decision, apart from the poll (rpc.rs :309, :310).** `ready` now
takes what poll found and hands it to `pieces::readiness`, a new private
function beside `written`, which already decides the write the same way
(ruling 10). Its two arms, a wait a signal interrupted and any other poll
error, can now be driven directly. A real poll of one descriptor cannot be
made to fail in a test. A bad descriptor reports `NVAL` in its events, not
an error. The only errors left are `ENOMEM`, or `EINVAL` when
`RLIMIT_NOFILE` is lowered for the whole test process. Hitting `EINTR`
would need a signal handler, which takes `unsafe`. So the test feeds the
decision the results poll returns: `INTR` and `NOMEM` among them, with
each event flag.

**The harness's poll error (rpc.rs :376).** `Lines::next` returned `Ended`
on a poll error in an arm of its own. For the reasons above, no test could
reach that arm. The poll's error now flows into the read's result
(`polled.and_then(|_| read)`), where it ends the pipe exactly as before.
`ready` never returns an interrupted error, because it reads `INTR` as not
ready, so a poll error is never mistaken for an interrupted read. The read's
own error arms are driven by a fake reader, `Erring`: a pipe holding one
byte, so poll finds it readable, whose every read errs with its kind.

**The loop's end in `serving` (calls.rs :375, :397) is reachable**, so it
is tested, not restructured. The check inside the loop runs before the
turn hears the server. When the server's answer is written in that turn,
`answered` writes it to the harness. If that write meets a broken pipe,
delivery is `Gone`, and only the next loop head sees it. The new test calls
a tool and closes the harness's output before the answer arrives. The
session ends there, served, with its ledger Clean.

**`offered`'s partial take (rpc.rs :329)** and **`told_to`'s retry
(stderr.rs :329)** are driven through the shared `Sink` fake. `Sink` gained
an `unready` count, the number of first writes it refuses with `EAGAIN`, as
a holder's status flags would.

**The tests, each bound by a compiling removal that was run, seen to fail
and then restored.**

| Test | Removal | What failed |
| --- | --- | --- |
| `pieces::tests::a_poll_a_signal_interrupts_is_unready_and_any_other_poll_error_is_its_callers` | `readiness`'s `Err(INTR) => Ok(false)` arm deleted | `INTR` left `Err(Some(4))`, right `Ok(false)` |
| the same test | `readiness`'s `Err(error) => Err(error.into())` made `Err(_) => Ok(false)` | `NOMEM` left `Ok(false)`, right `Err(Some(12))` |
| `pieces::tests::what_the_server_is_offered_and_partly_takes_before_a_write_errs_is_taken` | `offered`'s `Err(_) if taken > 0 => break` arm deleted | left `Err(BrokenPipe)`, right `Ok(512)` |
| `stderr::tests::a_read_a_signal_interrupts_is_quiet_and_any_other_read_error_ends_the_pipe` | `Lines::next`'s closure made `\|_\| false` | `Interrupted` left `Ended`, right `Quiet` |
| `stderr::tests::a_piece_the_sink_was_not_ready_for_is_offered_again` | `told_to`'s `Put::Unready => {}` folded into the ending arm | left `(Unready, 0, [])`, right `(Taken, 1, "one\n")` |
| `delivery::a_harness_output_closed_before_its_calls_answer_keeps_the_sessions_ending` | `serving`'s final `Ok(())` made `Err(Ended::Failed(UNDELIVERED))` | left `Err(Failed(Failed(Undelivered)))` with closure `Failed(Undelivered)`, right `Ok(())` with `Clean` |

The last removal also proves that the test ends at the loop head, the line
the gate missed, and not at the check inside the loop.

**Coverage, as far as this seat can observe it.** The seat did not run
`scripts/coverage-exact.sh`. The gate runs the whole workspace in shards and
needs namespaces outside a box, and running a script here needs approval,
which a headless seat cannot get. Instead, `cargo +nightly-2026-09-05
llvm-cov --branch` ran twice on the final tree. The first run covered
brokkr-cli's lib tests under `broker::` (148 passed). The second covered
the `it` binary's `capability_broker::` and `machine_proof::` suites (161
passed). Both wrote an LCOV, the records the gate counts. Every one of the
gate's misses is hit in the first run. `offered`'s guard at :326 shows 1
true and 5 false. The loop condition at calls.rs :375 and the interrupted
closure (the `Erring` instance) are both hit. In rpc.rs, rpc/calls.rs,
rpc/calls/outbox.rs, rpc/pieces.rs, session/stderr.rs, session/lifetime.rs
and session/readiness.rs, no line or branch is at zero in both runs. The
first run's zeros are the public serve path, which only the binary
reaches. They are rpc.rs 643–661, 671–677 and 708–709, stderr.rs 280–302
(with the branch at :300), lifetime.rs 51–76 and 95–96, and readiness.rs
145, 158–159, 252–276, 489–507 and 560–604. The second run hits all of
them. The second run's own zeros are the lines and branches only unit
tests drive. In rpc.rs they are lines 115, 128, 150–169, 270–273, 379,
400–413, 525–557, 587, 759 and 770–773, in parts, and branches at 322,
326, 331, 410, 553, 584 and 767. In stderr.rs they are lines 201–204, 233,
243, 304 and 329, and branches at 225, 232 and 242. Lifetime.rs has lines
136 and 236. Readiness.rs has lines 357, 409 and 534, and branches at 365
and 408. The first run hits every one of them. The exact gate's own union
over every shard stays the controller's, at verify.

**This visit's crate-scoped runs** (`timeout 590` each), every one green:

| Crate | Targets and passes |
| --- | --- |
| brokkr-protocol | lib whole ×3: 914 / 3 ignored each, in 64.13 s, 62.63 s and 61.06 s; hands_exits 6; secret_drop 1; doc 1 |
| brokkr-core | lib 138; it 26 |
| brokkr-store | lib 104; it 5, plus its four one-test self-spawns |
| brokkr-view | lib 313 / 3 ignored; it 6 |
| brokkr-bridge | lib 17 |
| brokkr-seatbelt-probe | seatbelt_lifetime_probe 99 / 2 ignored |
| brokkr-runtime | lib 964; capability_launch 75; it 132; operated_repo 1; queued_launch 6 |
| brokkr-cli | lib 822 / 1 ignored; it 653 / 3 ignored; driver_conformance 27; transcript_surfaces 13; heap_claude, heap_codex, heap_dsh 1 apiece |

**Gates.** Formatting and workspace clippy, with all targets and features,
`--locked` and `-D warnings`, were clean. `recipes/self` compiled with 47
valuations over 11 groups, and `recipes/verify` with 16 over 5, none
unruled. `openspec validate --all --strict` passed 26 of 26, and
`typos --hidden` and `git diff --check` were silent. `quality/ratchet.sh
files` and `clones` held. These `quality/file-lines.txt` rows moved:
`broker/rpc.rs` from 792 to 787, `rpc/pieces.rs` from 199 to 216, and the
test files `rpc/pieces/tests.rs` from 348 to 398, `rpc/tests/delivery.rs`
from 510 to 524 and `session/stderr/tests.rs` from 225 to 275.

## Third re-vouch: the complexity ratchet's refusal

CI on PR #676 at 5421565f passed every job but the exact-coverage gate.
Coverage there was exact. The step after it, `quality/ratchet.sh crap`,
refused `Session::serving` (rpc/calls.rs :373) at a cyclomatic complexity
of 16, over the ceiling of 15 that a new function may reach (decision 0071
ruling 4). This visit splits the function. Its behaviour, the public API
(1,117 entries) and `Cargo.lock` do not change.

**The split.** The turn's read and its handling move into a private helper,
`Session::asked`. It checks `untaken`, reads the harness's next frame
unless the outbox is full, and takes it: a request is handled, an oversized
frame refused. It returns whether the turn was quiet, or `None` once the
harness has ended its input. `serving` keeps every check in the same order:
`going()`, the room check, the helper (untaken, then the request's
handling), the second `going()`, `heard`, the drained standard error,
`flush` and the pace. The `?` on the request's handling and on the refusal
now sits on the helper's call, which propagates the same `Ended`.

**Measured.** `cargo crap --path . --format json` ran over the final tree
without an LCOV, since complexity does not depend on coverage. It scored
`Session::serving` at 8 and `Session::asked` at 8. Across rpc.rs,
rpc/calls.rs, rpc/calls/outbox.rs, rpc/pieces.rs, session.rs,
session/stderr.rs, session/lifetime.rs, session/readiness.rs,
secret/handoff.rs, namespace/entry.rs and broker/ledger.rs, the highest
scores were `ServerBox::launch` and `Lifecycle::admit` at 15, both
unchanged, then `Session::requested`, `decoded` and `Server::heard` at 14.
No function in those files is over 15.

**Coverage of the split, as far as this seat can observe it.** The seat
did not run `scripts/coverage-exact.sh`, for the reasons the second
re-vouch gives. `cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli --lib
--test it --branch` ran on the final tree, filtered to `broker`, with 148
lib tests and 97 `it` tests passing. In its LCOV's rpc/calls.rs record,
every line of `serving` (373–390) and of `asked` (396–408) has a non-zero
count. Each branch arm there was hit: the loop head at :375 (22,137 and
1), the let-else at :378 (22,030 and 46) and the second `going()` at :381
(2 and 22,021). Every arm of the helper's match ran, the oversized
frame's at :403 three times. The path existed before the split, so no test
was added. The exact gate's union over every shard stays the
controller's, at verify.

**Suites and gates on the split tree.** `cargo test -p brokkr-cli --lib
broker::` passed 148. Each of the eight crates then ran its whole suite
under `timeout 590`, with nothing failing. Every count matched the second
re-vouch's table, except brokkr-runtime's lib, which now passes 968 tests
after the main merge instead of 964. The formatting check and workspace
clippy (`-D warnings`) were clean. Both recipes compiled with no unruled
valuations, 47 for self and 16 for verify. The strict openspec validation
passed all 26 items, and typos and the diff check stayed silent. The
file-size and duplication ratchets held. The only baseline row that moved
is rpc/calls.rs in `quality/file-lines.txt`, from 757 lines to 767.

## Pending

- Exact coverage (`scripts/coverage-exact.sh`), on CI or a host outside a
  box.
- macOS CI and the final head's remote CI.
- SD4's fully cold Linux x86_64 samples, with the dentry, inode and page
  caches dropped, and its Linux aarch64 samples: U9b1's, where they block
  enablement (operator ruling 2026-10-14). Page-cache-cold and warm samples
  are observed above.

Tasks 31.1 and 31.2 stay unticked until these exist. Their U6f1b part is
recorded under 31.2 in `tasks.md`.

## Follow-ups noticed, not changed here

- A session ending `Ended::Unrecorded(Record)` with nothing latched still
  writes a Clean closure where the ledger is not broken. This is
  pre-existing (`Ended::failure`). The call path makes no such record. The
  bind-once refusal was one more record of that kind until the fourth
  run's second visit, which latches the masking cause on it.
- The fourth run's follow-up that a short write was latched as
  `Failure::Timeout` is closed by the fifth run's `Failure::Undelivered`.
- L1 of the fourth run's review, labelling the server's lines so none reads
  as the broker's own diagnostic, moved to U6f2 by the operator's ruling of
  2026-10-14.
- `capability_broker::a_plan_under_an_unprotected_root_or_file_is_unbound`
  failed once in the fifth run's whole-suite runs and passed alone and in
  the next whole run; the cause was not captured. The sixth run narrowed it
  but could not establish it (see that run's section). The seventh ran it
  at least 35 more times under raised contention without a failure, and
  the eighth's whole suite passed it once more. Its cause is still not
  established.
- When the broker's standard input cannot be duplicated, the session ends
  with `Failure::Protocol`, whose words blame the server (`rpc.rs`'s
  `initialized`; the seventh review's L2). An accurate cause needs a new
  `Failure` variant, which is a new public entry, and that needs the
  operator's ruling.
- `capability_broker::observer::the_admitted_launcher_runs_through_its_handle_with_no_environment`
  failed once when the seventh run raised the suite's box cap to 12. Its
  trace held `???( <detached ...>` after the launcher's `execve`. It passed
  at the normal cap of three.
- A read or poll error on the harness's standard input is read as the
  input's end (`rpc.rs`'s `Lines::next`). This predates U6f1b (the sixth
  review's INFO).
- Run 2 noted here that `served_within` set `O_NONBLOCK` on duplicates
  of the broker's stdin and stdout. The third run's L1 fixed that.
- A blocking write of at most 512 bytes after `POLLOUT` can still wait
  where another process writes the same pipe between the poll and the
  write. The operator recorded this as a limit on 2026-10-13 (quoted under
  the fourth run).
- The server's input is still written by `offered` at any byte. No guard
  reads what the server is sent, and the server holds its bindings.
- A panic in `serve` would be printed by the default panic hook, outside
  the voice.
- Two `process::tests` wall-clock bounds,
  `a_driver_that_stops_reading_cannot_hold_the_shutdown_write` (3 s) and
  `a_grandchild_holding_the_pipes_does_not_outlast_the_deadline` (5 s), are
  overrun by milliseconds when the protocol lib runs at 48 or 64 threads on
  a 24-core host. They passed in every run at 2, 4 and 24 threads. This is
  pre-existing, and the re-vouch does not change those tests.
- `quality/file-lines.txt` holds `capability_broker/observer.rs` at 975,
  but the file is 982 lines on main. This unit does not touch it.
