## 1. The view

*Serves: "Every other run's reservation is read through the participation view". Design: D2, D3.*

- [x] 1.1 Add `broker/inventory/standing.rs`: `Standing`, `Attempt`, `Writing`, `Reaching`, `Tolerated<T>`, the `format` probe, `From<Reservation>`, `From<&Writer>`, and `standing(dir, at, own)`; split `root::bytes` out of `root::read`. Verify: `cargo build -p brokkr-runtime --tests`.
- [x] 1.2 `inventory` takes the claim's `[repo, run]`, reads that run's directory whole, and reads every other through the view; `Found.standing` is `Vec<Standing>`. Verify: `foreign::this_runs_own_reservation_is_read_whole`, `an_unknown_participant_refuses_before_anything_is_sealed`.
- [x] 1.3 The comparison, the sources' comparison and recovery consume `Standing` and `Writing`; `Reservation` keeps no accessor. Verify: the existing admission, coordination, recovery, sealing, reach, processes and sources suites pass.

## 2. Refusals

*Serves: "This run's own reservation is read whole" and the view's unreadable cases. Design: D3, D5.*

- [x] 2.1 Add `Unknown::Formatted(path, format)` and pin its text once. Verify: `foreign::an_unreadable_foreign_reservation_refuses_by_name`.
- [x] 2.2 Move the widened case of `an_unknown_participant_refuses_before_anything_is_sealed` to this run's own file, check each case seals nothing of this run by its bytes, and assert another run's widened file is admitted and never sealed. Verify: that test.

## 3. Recovery

*Serves: "A record read only through the view is never sealed again". Design: D4.*

- [x] 3.1 `recovered` seals only a record read whole and releases a viewed record's proven attempts in memory. Verify: `foreign::a_foreign_reservation_proven_settled_is_released_but_never_sealed`.

## 4. Binding

- [x] 4.1 Show each new test binds by a compiling removal of its behaviour, restored afterwards. Verify: the result's notes name each mutation (own read through the view; the view disabled; the format check removed; an unknown `protects` read as nothing; an unknown `cleanup` read as settled; the in-memory release removed) and the tests it failed.

## 5. The decision

- [x] 5.1 Write decision 0090, `Status: proposed`, `Amends: 0065`, its index row, and `Amended by: 0077, 0090` on 0065; add dated markers to slice two's D6 and U8a2a text. Verify: `decisions_index` passes.

## 6. Gates and the archive

- [x] 6.1 `cargo fmt --all -- --check`, clippy `-D warnings`, the runtime, protocol and CLI suites by crate, and both bundle compiles. Verify: each exits clean, or is recorded with its output in the result's notes.
- [x] 6.2 Fold the delta into `openspec/specs/capability-writer-lease/spec.md` and archive the change here. The exact-coverage gate and the ratchets that run outside this seat's box are recorded pending and never lowered. Verify: `openspec_retention` and `provenance` pass.
