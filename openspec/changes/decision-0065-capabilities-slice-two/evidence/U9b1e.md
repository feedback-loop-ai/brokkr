# U9b1e — The broker closes its ledger when its harness ends it by signal (tasks 46.8, 46.9)

Two runs built this unit. Run 0065-slice-two-unit-u9b1e-see-th-d9b8e16d (base main at
6071d04df) built and proved it on its signed commit e31bf8ec2, then stopped on the one
manifest edge outside the row. Run 0065-slice-two-unit-u9b1e-see-th-95888c33 (base main at
2fe8308d2, U9b1d merged) took that commit replayed uncommitted, under ruling R6, and
re-ran every proof, removal and gate below on this tree, on Linux x86_64. This tree's
production code, tests and `Cargo.lock` are byte-identical to e31bf8ec2's: `git diff
e31bf8ec2 -- crates/` shows no tracked change, and the two new test files compare equal
to that commit's copies. Only design.md, tasks.md and this file differ.

## Ruling R6: the manifest edge

R5 names three production files. The change could not be made inside them: neither the
standard library nor the CLI's rustix features (`event`, `fs`, `net`, `process`, `std`,
`thread`) install a signal handler, rustix's `runtime` module is Linux-only and so cannot
serve macOS (decision 0063), and brokkr-protocol re-exports no signal facility. The
controller's ruling R6 (2026-11-01, under the operator's standing ruling of 2026-10-28,
option (a) of run 1's finding; now in design.md's U9b1e section and row) admits
`crates/brokkr-cli/Cargo.toml` for one dependency edge, `signal-hook = "0.3.18"` under
`[target.'cfg(unix)'.dependencies]`, and its one `Cargo.lock` line (`"signal-hook"` in
brokkr-cli's dependency list). That is the version brokkr-protocol already holds, so no
registry version moves and no new crate enters the lockfile. R6 refuses option (b), libc
with an unsafe `sigaction` and a hand-written self-pipe, and option (c), a brokkr-protocol
re-export, which is public API growth outside the row. Tasks 46.8 and 46.9 are ticked on
this run's evidence.

## The controller's host evidence

The controller ran its real Claude check (Claude Code 2.1.316, haiku) on run 1's commit
e31bf8ec2 on 2026-11-01, recorded in its own `host-check-e31bf8ec2-2026-11-01.md`. Both
runs completed end to end, phase done, ruling CLEAN: use-the-library-docs-mcp-tool-re-63bdfaa3,
a seat holding the brokered `library-docs` grant alone, and
use-the-library-docs-mcp-tool-re-08d32d8c, one also holding the native `web-search` beside
it. Claude Code's MCP log shows `Tool 'resolve' completed successfully` and then "Sending
SIGINT to MCP server process"; the call settled `succeeded`, and its retained response's
digest (53192b79…) was read back through `inspect`. U9b1d's check runs (7792d0d5, baba94ca)
had parked at the same point on "broker ledger lifecycle is invalid". This tree carries
e31bf8ec2's production code unchanged, so the check stands for it; the controller re-checks
only if the production code moves.

## What changed

Production, inside the row's files and their existing private children:

| File | Change |
|---|---|
| `crates/brokkr-cli/src/broker.rs` | Adds `Signalled`. Before anything of the plan is read, `serve` installs signal-hook's SIGINT and SIGTERM handlers. Each handler only writes to signal-hook's own pipe. `at_least(n)` drains that pipe (`Signals::pending`, which does not block) on whichever thread asks, behind one mutex, and counts what arrived. No handler touches the ledger: its one writer stays the session's thread. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` | `serve` derives three readings from the count. For the startup, one signal is a cancellation (`orphaned() \| at_least(1)`), so admission, box establishment, handoff and initialization each end with the cause they already give a cancelled starter. For serving, one signal is the harness hanging up (`at_least(1)`), and a second, or the starter's end, is the attempt's cancellation (`cut`). The broker's own standard error is written under `cut`. |
| `crates/brokkr-cli/src/broker/rpc.rs` | `serving` and `initialized` take the private `Ending` pair (cut, hung_up). The served attempt's cancellation becomes `cut`, and the hang-up travels with the harness's input. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (existing private child) | The `Harness` alias is `(input, output, hung_up)`, and `Session` keeps `hung_up`. `asked` reads no frame once a signal has hung up. With no call active, that reads as the input's end, so the existing `hung_up()` ending runs: Clean, or Failed by a fatal cause latched first. With a call active, the turn reads as quiet: the server is still heard, so the call's answer settles it within the call's own deadline, and anything else fatal ends it as before. A second signal reaches `going()` as the attempt's cancellation and fails the call by MB5's timeout. That is the existing local ending. |

No public item was added or changed. `adapters.rs`, `adapters/*.json`, `adapters/mcp/admit.rs`,
`broker/ledger.rs`, `exit.rs`, `hands serve` and `session.rs` itself are unchanged. The
ledger check is unchanged.

Tests:

- `rpc/tests/hangup.rs` is a new child of `rpc/tests.rs`, holding three in-process tests.
- `rpc/tests.rs` gives `Beside` a `hung_up` predicate (`&live` in `plain()`). The file sits
  at its 2,000-line ceiling.
- `rpc/tests/masking.rs` and `rpc/tests/delivery.rs` updated their call sites in place.
  `delivery.rs` gains the `UNSIGNALLED` constant.
- The enabled suite's fixture `HARNESS` in `enabled.rs` gains two switches. `mid-call`
  stops talking at the call's Started without killing anything. `signal` sends each named
  signal a second apart while it holds the broker's input open, then waits for the broker
  or kills it at 20 s.
- `enabled/signals.rs` is a new child module, holding four tests.
- `enabled/windows.rs` gains `signalled_at` and its two tests. The ending assertions
  `expired_at` already made moved into `closed_on_its_cause`, which both functions share,
  because the clone gate found them repeated.

`quality/file-lines.txt` rows were set for every touched file and for the two new ones.
`quality/ratchet.sh baselines 6071d04df` reports no baseline raised.

## Proofs

| Proof | Test | Observed |
|---|---|---|
| SIGINT after the call is answered, input held open | `capability_broker::enabled::signals::a_harness_ending_its_broker_by_sigint_after_its_call_completes_with_the_ledger_closed_clean` | The run completed. Broker exit 0. Ledger `opened, started, terminal, closed` with disposition `"clean"`. `inspect` gives `resolve` `succeeded` with the response's digest, and `--capability-call` returns the retained response. The broker ended under 10 s after the signal. |
| The same by SIGTERM | `…signals::a_harness_ending_its_broker_by_sigterm_after_its_call_completes_with_the_ledger_closed_clean` | The same assertions passed. |
| SIGINT while the call is in flight, answered 2 s later | `…signals::a_call_in_flight_at_sigint_settles_by_its_answer_and_the_ledger_closes_clean` | The same Clean completion, with the retained response read back. The broker ended at least 1 s after the signal, and the answer reached the harness after the signal. |
| SIGINT then SIGTERM with the call unanswered (Claude Code's SIGINT, then SIGTERM) | `…signals::a_call_unanswered_when_a_second_signal_comes_fails_by_the_timeout_and_the_ledger_closes` | Parked on "MCP server call timed out" past the harness's success. Broker exit 1. Ledger `opened, started, terminal, closed` with `{"failed":"timeout"}`. `inspect` gives `resolve` `failed`. The broker ended under 5 s after the second signal. |
| SIGINT during admission (observer held by the tracer) | `…windows::a_signal_at_a_brokers_observation_ends_it_by_its_identity_cause_unread` | The broker ended under 10 s after the signal. Parked on the identity cause, broker exit 1. Ledger `opened, closed` with `{"failed":{"refused":"identity"}}`. No call row, store unread, server never started. |
| SIGINT during box establishment (box stopped at its entry) | `…windows::a_signal_at_a_standing_box_ends_it_by_its_establishment_cause_unread` | The same, with the establishment cause. |
| Hang-up with the input open ends the session served | `broker::rpc::tests::hangup::a_harness_hung_up_by_a_signal_ends_its_session_served_with_its_input_open` | `(Ok(()), no calls, Clean)`. |
| An active call is heard until its answer; a fatal cause stays first | `broker::rpc::tests::hangup::a_call_active_when_its_harness_hangs_up_is_heard_until_its_answer` | Answered: `lookup` `Succeeded` and Clean, with the answer delivered. A stray response id: Failed by the protocol cause, never Clean. |
| The attempt's end after a hang-up fails the call | `broker::rpc::tests::hangup::a_call_still_unanswered_when_the_attempt_ends_after_a_hang_up_fails_by_the_timeout` | Failed by the timeout, with the harness told nothing, under 3 s. |
| A ledger without Closed still refuses | the M1 run below, and the existing `kill-broker` proofs | "broker ledger lifecycle is invalid", with the harness reporting success. |

The existing windows proofs `an_observation_held_to_the_startup_deadline_…` and
`a_box_held_at_its_entry_to_the_startup_deadline_…` pass after the `closed_on_its_cause`
refactor.

## Mutations: compiling removals, each failing its own test, then restored

| Removal | Test | Failure observed |
|---|---|---|
| M1, the signal handling: `Signals::new(ENDING).ok().filter(\|_\| false)`, so no handler is installed | `…by_sigterm_…closed_clean` | Parked "broker ledger lifecycle is invalid": SIGTERM killed the broker before Closed, the controller's defect exactly. |
| M1 | `…by_sigint_…closed_clean` | The same park. The fixture's `sh` starts the broker as an asynchronous list, which inherits SIGINT ignored (POSIX 2.12), so the broker kept serving until the harness's 20 s kill. |
| M1 | `…windows::a_signal_at_a_brokers_observation…` | `Observation: 30.001026073s`: the broker ended only at its startup deadline. |
| M2, its closure: `asked` reads `(self.hung_up)() & false` | `hangup::a_harness_hung_up_by_a_signal_ends_its_session_served…` | `Err(Failed(Timeout))` with a Failed closure where `Ok`/Clean was expected. |
| M2 | `hangup::a_call_active_when_its_harness_hangs_up…` | Closure `Failed(Timeout)` where Clean was expected. |
| M2 | `…by_sigint_…closed_clean` | Parked "broker ledger lifecycle is invalid". |
| M3, the wait for an active call: the hang-up arm returns `Ok(None)` | `hangup::a_call_active_…` | The harness was told "MCP server call timed out" in place of the answer. |
| M3 | `hangup::a_call_still_unanswered_…` | A frame was told where none was expected. |
| M3 | `…a_call_in_flight_at_sigint…` | Parked on "MCP server call timed out". |
| M4, the second signal: `cut` drops `at_least(2)` | `…a_call_unanswered_when_a_second_signal_comes…` | Parked "broker ledger lifecycle is invalid" where the timeout was expected. |
| M5, startup cancellation by signal: `cancelled` drops `at_least(1)` | `…windows::a_signal_at_…observation…` | `Observation: 24.924334879s`. |
| M5 | `…windows::a_signal_at_…standing_box…` | `Entry: 27.365711309s`. |

Run 2 applied each removal again on this tree, one at a time, and observed every failure in
the table: the parks and causes word for word, and the durations shown. M2 and M3's
in-process failures read `left: (Err(Failed(Failed(Timeout))), ([], Some(Failed(Timeout)), None))`
against `right: (Ok(()), ([], Some(Clean), None))` for the served hang-up, and the harness
told `"MCP server call timed out"` against the answer `lookup` (M3), or against `None`.
After the last restoration, `git diff e31bf8ec2 -- crates/` again showed no tracked change,
and the full suites below passed.

## Gates on this tree

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Clean. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Clean. |
| brokkr-cli `--lib`, all features | 1113 passed and 1 ignored, in two splits (`broker::` 178; the rest 935 + 1 ignored). One unsplit run hit the seat's 590 s bound under the host's load. |
| brokkr-cli `--test it`, all features | 944 passed and 4 ignored, in three splits (`capability_broker::enabled::` 81 + 1 ignored; the rest of capability_broker 108; the remainder 755 + 3 ignored). |
| brokkr-cli separate binaries | `driver_conformance` (27), `transcript_surfaces` (15) and the eight `heap_*` binaries pass. |
| `cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` | Compiled, with a policy sweep of 48 valuations over 11 groups, 0 unruled. |
| `openspec validate --all --strict` | 36 passed. |
| `typos --hidden`, `git diff --check` and `git diff --cached --check` | Clean. |
| `quality/ratchet.sh files`, `quality/ratchet.sh clones` and `quality/ratchet.sh baselines 2fe8308d2` | "file size holds", "duplication holds", "no baseline raised since 2fe8308d2". |

`crates/brokkr-cli/src/broker/rpc/tests.rs` stays at exactly 2,000 lines. Only brokkr-cli
moved, so run 2 ran no other crate's suite. Run 1 ran core, view, store, protocol, bridge,
seatbelt-probe and brokkr-runtime on the same code, with no witness or compose pin moved.
Neither run ran `scripts/measure-budgets.sh`, because no budgeted path (fold, run view,
compile, TUI frame, binary startup, `inspect`) changed.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) outside the box is pending, and so is macOS
CI. signal-hook supports macOS, and the change has no Linux-only code. The host check is
done (above).

## Notes

These are follow-ups, not work for this unit. signal-hook's `pending` may return a signal
once even when it arrived several times ("a signal instance may returned only once even if
it was received multiple times", docs.rs, signal-hook 0.3.18, `SignalsInfo::pending`,
re-read in run 2). So two arrivals of one signal before the session asks count as one, and
a harness that sends SIGINT twice may need a third signal or the existing deadlines to cut
what the first ending waits for. Claude Code's SIGINT-then-SIGTERM pair are two different
signals. Installing the handlers also overrides a SIGINT that the broker inherited ignored
(POSIX 2.12). This is what lets a POSIX shell's background job, such as the fixture, end it
by SIGINT.

Sources:

- https://docs.rs/signal-hook/0.3.18/signal_hook/iterator/struct.SignalsInfo.html
- https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html (section 2.12)
