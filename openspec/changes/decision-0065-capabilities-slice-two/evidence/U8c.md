# U8c — Publish verified content-addressed artifacts

Tasks 40.1 and 40.2; requirements CR2 and CR5. Branch `s2/U8c`, cut from
main at baa5a08f, run `0065-slice-two-unit-u8c-see-the--f768473e` (the
unit's second run; the first stopped before editing anything on the ruling
below).

## What was built

The engine publishes each response a broker's verified ledger cites, once
every process the attempt owned is proven settled and while its writer
lease still stands, before the lease is released. Settlement
(`engine/broker/settlement.rs`, an existing private child of
`engine/broker.rs`) consumes the new `engine/artifacts.rs`.

Publication reads the attempt's evidence only from handles: the run's
directory opened below the coordination root the lease stands in
(`reserved_at`), then the attempt's directory, each name opened without
following a link and judged on the opened handle as this user's own
directory, or regular file of one link, readable by no one else. The
inventory is read, each plan is read beside it, its bytes must digest to
its pin and its owner must name this worktree, run and attempt, and each
plan's ledger is read through `brokkr_protocol::broker::ledger::recover`,
the single ledger reader. Only the verified prefix's citations are used
(`Lifecycle::cited`); the outcome never stands in for one. A plan that
retains nothing but whose ledger cites a response, or a citation where
admission protected no artifact root, is refused. Each cited stage is read
no further than one byte past its cited length, and its bytes must digest
to the cited sha256 (`Ledger::answered` takes the digest it is handed, so
the engine checks the bytes itself). A stage no Terminal cites is never
read.

Those very bytes are published at `artifacts/sha256/<hex>` under the
artifact root admission sealed. The root is opened and must be the very
directory (device and inode) admission recorded, owned by this user. The
digests directory is made owner-only where missing and the root synced. A
fresh artifact is written to an exclusive owner-only, read-only temporary
file beside the digests (never in them, where admission requires every name
to be a digest), synced, linked to its digest path (a link never replaces
what stands there), its temporary name removed, and the digests and the root
synced. The digest path must then name the very file written. An artifact
already standing is reused only after its bytes, read on the handle judged,
digest to its name.

Refusals are typed (`artifacts::Unpublished`):

- `broker evidence is not bound to this attempt`
- `broker response digest mismatch`
- `retained artifact digest mismatch`
- `retained artifact path is not a regular contained file`
- `retained artifact could not be published durably`

The CR5 reader (`artifacts::read`, consumed by publication's reuse check,
left for U8g to open to inspect) says `retained response artifact is
missing`, `retained response artifact digest mismatch` or the path cause.

A settlement whose publication is refused fails its run through the
existing `EngineError::Unreleased`, whose private cause now also carries
`Unpublished`. Its text is "the attempt's retained responses were not
published: <cause>". The release still happens: a refused publication says
nothing about whether the attempt's processes settled.

## Scope, files and API

| File | Change |
| --- | --- |
| `crates/brokkr-runtime/src/engine/artifacts.rs` | New (row file), 450 lines; `quality/file-lines.txt` row set |
| `crates/brokkr-runtime/src/engine.rs` | 3,523 lines before and after. Hunk at lines 38–39 merges two `use crate::…` lines into one. Hunk at line 47 adds `mod artifacts;`. The two `RunStarted` appends (#697) are untouched |
| `crates/brokkr-runtime/src/engine/broker.rs` | 800 lines before and after; `INVENTORY`, `REPLY_BYTES_MAX` and `directory()` become `pub(super)` |
| `engine/broker/settlement.rs` (existing child) | Publication before release; `Unreleased` gains a private `Cause` and the `unpublished` constructor |
| `engine/broker/admission.rs` (existing child) | `reserved_at` becomes `pub(super)` |
| `engine/broker/historical.rs`, `engine/broker/inventory.rs` (existing children) | `ARTIFACT_BYTES_MAX`, `DIGESTS` and `hex` move to `artifacts.rs`, their one home now that publication shares them |
| `crates/brokkr-protocol/src/broker/ledger.rs` (ruling A) | `Lifecycle` keeps each admitted Terminal's citation; new `Lifecycle::cited` |
| `crates/brokkr-protocol/src/broker/ledger/retention.rs` (ruling A) | `Response`'s fields become `pub(super)` |

Public API growth is one item, admitted by the operator's ruling A of
2026-10-20:
`pub fn brokkr_protocol::broker::ledger::Lifecycle::cited(&self) -> impl Iterator<Item = (u64, &str, u64)> + '_`,
recorded in `quality/public-api/brokkr-protocol.txt`.
`quality/ratchet.sh baselines baa5a08f` names exactly that raise (1,184
public items, was 1,183), so the pull request body needs a literal
`Ruling:` line naming ruling A. brokkr-runtime's API snapshot is unchanged
(`quality/ratchet.sh api`: "public API holds"). An earlier draft added an
`EngineError` variant, which would have grown the runtime's API. It was
withdrawn for the `Unreleased` route above.

## Owning tests

While the MCP compile fence stands no public engine can provision a
retaining plan, so the proofs are in-crate child modules (operator ruling
A); `crates/brokkr-runtime/tests/capability_ledger.rs` is unchanged. No
test was moved.

- `crates/brokkr-runtime/src/engine/artifacts/tests.rs` (new, 641 lines):
  twelve tests. They write the attempt's evidence with the protocol's own
  `Ledger` (`open`, `retain`, `answered`, `end`, `close`) on a canonical
  temporary root.
- `crates/brokkr-runtime/src/engine/broker/admission/tests/sealing.rs`:
  `settlement_publishes_what_its_attempt_retained_before_its_release`,
  plus the `Unreleased::unpublished` text pinned in
  `a_failed_settlement_says_whether_its_lease_was_released`.
- `crates/brokkr-protocol/src/broker/ledger/tests.rs`:
  `the_lifecycle_cites_the_response_of_each_verified_terminal_and_no_other`
  (new), and `a_terminal_cites_a_sound_response_for_an_answered_call_alone`
  extended to assert that only the sound case is cited.

## Removal controls

Each mutation below compiled, was run against the named suites, failed as
shown, and was restored. The suites then passed (runtime lib 1,212 passed;
protocol ledger 10 passed).

| # | Mutation | Failing test, assertion observed |
| --- | --- | --- |
| M1 | `Lifecycle::admit` keeps no citation | `the_lifecycle_cites_…`: `([], None)` vs both citations; `a_terminal_cites_…`: `("sound", …, 1, 0)` vs `1, 1` |
| M1b | citation recorded before `ended` admits the Terminal | `the_lifecycle_cites_…`: the conflicting duplicate's digest cited instead of the first |
| M2 | stage digest not checked (`\|\| true`) | `a_stage_that_is_not_exactly_…`: `("digest", Err(Collision), [<hex>])` vs `Err(Response), []`; settlement test `("mismatch", Err(Collision) …)` |
| M3 | non-retaining plan's citation accepted | `a_citation_no_retaining_plan_…`: `Ok(())` vs `Err(Unbound)` |
| M4 | root identity not checked | `an_artifact_root_other_than_…`: `Ok(())` vs `Err(Uncontained)` |
| M5 | one-link rule dropped (`nlink >= 1`) | `a_digest_path_that_is_no_regular_file_…`: `("hardlink", Ok(()), true)`; `a_stage_that_is_not_exactly_…`: `("hardlink", Ok(()), [<hex>])` |
| M6 | reader opens without `NOFOLLOW` | `a_digest_path_…`: `("symlink", Ok(()), true)`; `a_reader_tells_…`: linked name `Err(Corrupt)` vs `Err(Uncontained)` |
| M7 | corrupt standing artifact reused | `an_artifact_already_standing_…`: `Ok(())` vs `Err(Collision)` |
| M8 | verified standing artifact not reused | `an_artifact_already_standing_…` and `a_publication_not_made_durable_…`: `Err(Uncontained)` vs `Ok(())` |
| M9 | published file's identity not compared | `an_artifact_replaced_while_it_is_published_is_refused`: `Ok(())` vs `Err(Uncontained)` |
| M10 | failed directory sync after link ignored | `a_publication_not_made_durable_…`: `Ok(())` vs `Err(Undurable)` |
| M11 | temporary name never removed | `a_publication_not_made_durable_…`: names `[".<hex>.publishing", "sha256"]` vs `["sha256"]`, plus three tests `Err(Uncontained)` vs `Ok(())` |
| M12 | stale temporary file not cleared first | `a_publication_not_made_durable_…`: `Err(Undurable)` vs `Ok(())` (once the case had its own fixture; the first form did not bind and was rewritten) |
| M13 | plan's pinned digest not checked | `a_plan_not_as_pinned_…`: `Ok(())` vs `Err(Unbound)` |
| M14 | plan's owner not compared | `a_plan_not_as_pinned_…`: `Ok(())` vs `Err(Unbound)` (foreign-run case; the first form refused at the locator and did not bind, so a case was added) |
| M15 | pin locator not checked | `a_plan_not_as_pinned_…`: `Ok(())` vs `Err(Unbound)` |
| M16 | missing attempt directory refused | `an_attempt_that_provisioned_nothing_…`: `Err(Unbound)` vs `Ok(())` |
| M17 | reader resolves a non-digest name | `a_reader_tells_…`: `("../true", Err(Missing))` vs `Err(Uncontained)` |
| M18 | settlement never publishes | settlement test: `("verified", Ok(()), false, true)` vs `… true, true` |
| M19 | settlement publishes though a driver ended unresolved | settlement test: `("unresolved", Ok(()), true, false)` vs `… false, false` |
| M20 | temporary file linked though its write failed | `a_publication_not_made_durable_…`: digests `[<hex>]` vs `[]` |
| M21 | settlement drops the publication refusal | settlement test: `("mismatch", Ok(()), …)` vs `Err(Unreleased { lease: Unpublished, cause: Unpublished(Response) })` |

An intermediate form of M18 (`Ok(()).or(published(&held))`) still ran
publication eagerly and was replaced by a true removal. The stage's
explicit length comparison was dropped before these controls because the
digest comparison decides every case it could. The cited length remains
the read bound.

## Gates observed at the tested tree

On the final tree, before the commit, formatting and the workspace's
clippy run were clean. Every crate's suite passed in turn. In brokkr-protocol
that was 1,021 lib tests and its integration targets. In brokkr-runtime it
was 1,212 lib tests, and `it` (135), `capability_launch` (76),
`operated_repo` and `queued_launch`. The witness digests are inside `it`, so
no witness pin moved. In brokkr-cli it was 944 lib tests, and `it` (744),
`driver_conformance` (27), `transcript_surfaces` (14) and the three heap
targets. Core, store, view, bridge, seatbelt-probe and the workspace's doc
tests passed as well. Both `recipes/self` and `recipes/verify` compiled with
no unruled valuation. The file-size, duplication and public-API ratchets hold
with the protocol snapshot updated. No known flake showed in this seat.

Cyclomatic complexity was measured with `cargo crap` 0.5.0 (no coverage).
Every new or changed function is at most 15: `Lifecycle::admit` 15,
`Engine::settle` 14 (11 at base), `published_with` 12, `held` 8,
`provisioned` and `linked` 7. No suppression was added, and no function
exceeds clippy's 100-line ceiling.

The budget inputs are untouched: no prompt, charter or Cargo.lock change.
The budget and heap tests passed in the suites above.

Still pending: the exact-coverage gate (`scripts/coverage-exact.sh`, run
on a host outside a box), and the macOS and remote CI results on the final
head. `cargo crap` against that gate's LCOV is pending with it.

## Carried findings

- LOW (carried by the operator's direction, not consolidated): the 256 MiB
  budget, 8 MiB floor and share rule written in both protocol `retention.rs`
  and runtime `engine/broker.rs`.
- LOW (new, of the same family): `artifacts.rs` bounds a ledger read at
  64 MiB (`LEDGER_BYTES_MAX`), restating the protocol's private
  `LEDGER_MAX`. Ruling A admitted no second public item, so it was not
  exported (0071 ruling 5).
- A refused publication leaves verified artifacts already published for
  earlier citations of the same attempt. They are content-addressed and
  verified, and only a later checkpoint (U8d) cites any.
