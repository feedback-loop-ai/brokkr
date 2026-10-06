# U4f: sequence and resumed observations (tasks 18.1–18.2)

Run `0065-slice-two-unit-u4f-see-the--6fde4d4d` built U4f on branch
`s2/U4f` from main at `74827722`, under the operator's row amendment of
2026-10-06. That amendment is committed here, in design.md (the row and the
hot-files rows) and in the operator-ruling addendum. The unit changed exactly
the amended row's three production files:
`crates/brokkr-runtime/src/engine.rs`,
`crates/brokkr-runtime/src/engine/resume.rs` and
`crates/brokkr-runtime/src/engine/capability_calls.rs`.
`engine/sequence.rs` did not move. No contract, fixture, policy table,
reference schema, extension, adapter, protocol or store file moved, and no
shipped driver emits an observation yet (U4f2).

## What changed

Sequence steps, retried seats and replacements already reach U4e's consumer
through `run_driver`, the single-site sink. What they lacked was history and
deduplication, so this unit adds those two things to the one consumer.

`resume.rs` derives the history. `SiteContext` gains a `history` field of
the new `RootHistory`, which holds two sets: attempts and call ids.
`SiteContext::new` builds a context with empty history.
`SiteContext::offered(events, offer)` fills it from this run's journal. It
takes every stamped checkpoint at the context's own `site_ref`. The
attempts are those whose row there names the offered root's
`root_session.id`. The calls are every `call_id` that those attempts' rows
at this site carry. A site offered nothing keeps empty history, and so does
a root that has no stamped row there, such as decision 0030's legacy offer.
The derivation is pure and reads only the events it is handed.

`engine.rs` builds each site's context with `SiteContext::new` in
`site_plans`, then hands the plan `context.offered(events, offer.as_ref())`.
The offer is the same one `offer_for_site` decided. Both sinks now hold
`let mut calls`, because consuming a call records it. These edits are
line-for-line, so `engine.rs` stays at its 4,859-line baseline.

`capability_calls.rs` keeps the root's history on the `Owner`.
`Owner::call_id_in(attempt, call)` is the one canonical `NativeCall` digest,
and `call_id` now calls it with the attempt being served. `Owner::replays`
matches the observation's `Format` exhaustively:

- **Claude.** A `toolu_*` id is history when its id, recomputed under any of
  the root's earlier attempts, is among the root's journaled call ids.
- **Codex.** `item_N` ids restart at `item_0` on every invocation, so a
  repeated one is a fresh call (U0, `docs/evidence/adapters/slice-two-mcp-isolation.md`
  row Codex).
- **dsh.** Its resume is unmeasured (same evidence, row dsh), so nothing
  measured calls its ids replayed. They are judged fresh.

`Calls::consume` now takes `&mut self`. Missing identity is checked first:
no harness id, an empty one, or no owner. Each still refuses with exactly
`capability telemetry cannot be attributed`. The unheld-tool refusal is
unchanged. Then two kinds of call stay ordinary rows, with the observation
removed and no group:

- a replayed call;
- a call id this site-attempt has already attributed, so a call's start and
  completion yield one observed checkpoint.

Distinct ids stay distinct. Ownership is still read from the sealed
selected outcome. A replaced seat whose root was not offered has no history,
so its replayed `toolu_01` is a fresh call of the new attempt.

## Assumptions read from the framing

Replayed history and the second observation of a call are journaled as
ordinary rows. They are not dropped. CC1 says history is "never restamped
as current-attempt activity", and the framing says the second row "remains
ordinary". Neither tells the engine to discard telemetry.

The framing placed "pure derivation" in `resume.rs` and the consumer in
`capability_calls.rs`, and this unit follows that split. `resume.rs` sits
over the 800-line production ceiling at a 1,062-line baseline, and the
derivation needs room. Its inline `#[cfg(test)] mod tests` therefore moved
to `crates/brokkr-runtime/src/engine/resume/tests.rs`, following the
existing `engine/replay/tests.rs`. `resume.rs` is now 942 lines.

## Tests

| Suite (lines) | Test | What it asserts exactly |
| --- | --- | --- |
| `engine/resume/tests.rs` (new, 225) | `an_offered_roots_history_is_its_own_attempts_calls_at_its_own_site` | Nine hand-built rows: two attempts of root `r`, one attempt of root `s`, two rows at another site and one unowned row. The history equals `RootHistory { attempts: {first, second}, calls: {n-first, n-second} }`. With no offer it equals `RootHistory::default()`. |
| `engine/capability_tests/call_tests.rs` (392 to 443) | `a_calls_start_and_completion_count_once_and_new_calls_stay_distinct` | Codex `item_1` arrives twice and `item_2` once, through a real driver process, the v6 fence and an offline verified export. The journal is exactly `[group(n(item_1)), ordinary row, group(n(item_2))]`, and `refused` is `None`. |
| same | `replayed_session_history_is_no_new_use_and_a_restarted_codex_item_is_fresh` | The plan's history is attempt `earlier`, holding the ids of `toolu_01` and `item_1` under that attempt. The run sends Claude `toolu_01`, Claude `toolu_02` and Codex `item_1`. The journal is exactly `[ordinary row, group(n(toolu_02)), group(n(item_1))]`, and `refused` is `None`. |
| `tests/capability_launch/legacy_journal.rs` (719 to 723) | `every_site_shape_journals_the_engines_group_on_an_observed_held_call` (changed) | This is D9's compile-to-journal matrix, through production's Claude driver. Every site keeps its exact rows. The resumed seat's second session rejoins `resumed-root-0` and replays `toolu_01`; that search is now ordinary, with no group. The replaced seat was not offered its unpersisted root, so both its searches keep the group. Distinct attributed ids drop from 12 to 11. No `response_sha256` or observation reaches any row, and the run exports and verifies offline. |
| same | `every_site_shape_journals_its_legacy_native_rows_through_export_and_verify` (unchanged) | The legacy path at every site shape, sequence and resumed/replaced included, still journals, exports and verifies exactly. |

The missing-identity cases in `an_unheld_or_unattributable_call_fails_its_attempt`
are unchanged and still pass. They now run through the reordered check.

Test moves and edits that change no behaviour:

- `resume.rs`'s four inline tests moved verbatim, de-indented one level,
  into `engine/resume/tests.rs`. `a_non_object_checkpoint_is_neither_stamped_nor_reshaped`
  now builds its context with `SiteContext::new`.
- `resume_tests.rs`'s `context` helper calls `SiteContext::new`, which takes
  that file from 4,089 to 4,085 lines.
- `call_tests.rs`'s `planned` helper also calls `SiteContext::new`. Its MCP
  test holds `let mut calls`, and its `native` oracle delegates to a new
  `native_in(attempt, …)`.

## Removal controls

Each mutation compiled, was run, failed the named assertion, and was
restored. After all six restores, `git diff` held no mutation text, and the
gates below ran on the restored tree.

| # | Mutation | Failed (test: assertion) |
| --- | --- | --- |
| M1 | history filtering removed (`false && owner.replays(..)`) | `replayed_session_history…`: `call_tests.rs:278`, journal equality. Matrix: `legacy_journal.rs:706`, `journaled == wanted` (the resumed replay carries a group) |
| M2 | deduplication removed (`!seen.insert(..) && false`) | `a_calls_start_and_completion…`: `call_tests.rs:245`, journal equality (the completion carries a second group) |
| M3 | engine.rs hands `offered` no offer (`offer.as_ref().filter(\|_\| false)`) | Matrix: `legacy_journal.rs:706`, `journaled == wanted` |
| M4 | the derivation keeps every root's attempts, not the offered root's | `an_offered_roots_history…`: `resume/tests.rs:115`, history equality |
| M5 | the derivation ignores the site (`site == self.site_ref \|\| true`) | `an_offered_roots_history…`: `resume/tests.rs:115`, history equality |
| M6 | the single-site sink consumes a clone, so the observation is not removed (consumer bypass on the sequence/resume path) | Matrix: `legacy_journal.rs:666`, `driven`'s status is not `Completed`, because the fence refused the private observation |

Commands: `cargo test -p brokkr-runtime --lib -- call_tests:: resume::tests::`
and `cargo test -p brokkr-runtime --test capability_launch legacy_journal::`.

## Gates on the final tree

These ran on base `74827722` plus this unit's changes, before commit.
`cargo fmt --all -- --check` was clean. Workspace clippy with all targets,
all features, `--locked` and `-D warnings` finished without a diagnostic.
`cargo +1.88 check --workspace --all-targets --all-features` finished.

`cargo test -p brokkr-runtime` passed its lib (794), `capability_launch`
(72), `it` (120, with `witness_digests` unblessed), `operated_repo` (1) and
`queued_launch` (3). `cargo test -p brokkr-cli` passed its lib (627, 1
ignored), `it` (459, 2 ignored), `driver_conformance` (27),
`transcript_surfaces` (13) and the three heap binaries. The other crates,
store included, passed under
`cargo test --workspace --exclude brokkr-runtime --exclude brokkr-cli`.

`bundles/self` and `bundles/verify` compiled with exit status 0.
`quality/ratchet.sh` held in `files`, `clones` and `api`. `baselines
74827722` reported no baseline raised. The local `main` ref is stale at
`19bca5ff`, so `baselines main` compared against the wrong tree.
`openspec validate --all --strict` reported 20 passed and 0 failed.
`typos --hidden`, `git diff --check` and `git diff --cached --check`
printed nothing.

This seat refuses a shell redirect into a script, so
`quality/file-lines.txt` was edited by hand from `wc -l`. It records:

- `capability_calls.rs` at 272 and `resume.rs` at 942;
- the new `resume/tests.rs` at 225;
- `call_tests.rs` at 443, `resume_tests.rs` at 4,085 and
  `legacy_journal.rs` at 723.

`engine.rs` is unchanged at 4,859.

`quality/too-many-lines.txt` was re-measured with
`cargo clippy -p brokkr-runtime --all-targets --all-features --locked -- -A clippy::all --force-warn clippy::too_many_lines`,
using clippy 0.1.98, the baseline's own. It records the six long
`resume_tests.rs` tests at their new lines. Two of those entries were
already stale at the base: `the_real_dsh_driver_journals_no_route_byte_and_no_carrier`
started at 2,944, not 2,941, and `…_on_the_gated_shapes` started at 3,325
with 299 lines, not 3,322 with 302. They now carry the measured values.
`engine.rs`'s three entries did not move.

No new long function and no suppression was added. No witness or compose
input moved. `scripts/measure-budgets.sh` was not run. Its inputs are
prompts, `Cargo.lock` and transcript heap, and this diff touches none of
them; the budget tests passed in the suites above. The exact-coverage gate
(`scripts/coverage-exact.sh`) and remote CI on both operating systems are
pending.

## Findings carried

LOW (decision 0065 ruling 8, the commission's note). Deduplication is per
site-attempt. Codex's ruling-8 in-attempt cold replacement admits a second
invocation inside one attempt only when the rejected child began no work.
That child emitted no `item_N`, so the restarted `item_0` of the second
invocation is not wrongly folded into it. If that admission ever widens,
deduplication must also key on the invocation.

LOW, follow-up for U4f2. dsh ids are judged fresh because dsh's resume is
unmeasured. If U0 later measures a dsh resume, `Owner::replays` must take
its measured semantics, and the exhaustive `Format` match makes that a
compile-visible edit.
