# U8i — Pin the store's folder in a Codex seat's profile

Tasks 45.11 and 45.12; requirements SI1 and SI2, with SD2 and SD4 for the
verification duty. Branch `s2/U8i`, cut from main at b8e2caff, recipe
`claude-flash-dsh-adv`, run `0065-slice-two-unit-u8i-see-the--b015d453`.
Everything below was observed in this run's implement seats, on this tree,
on Linux 6.17: the first visit, and the second, which answered the review's
return (see "The review's return" below; every removal and gate was re-run in
the second visit against the final code). What could not be run is listed as
pending at the end.

## What changed

The renderer and the bound reading in
`crates/brokkr-protocol/src/native_controls/grammar/config.rs` share one
derivation, `pinned(class, bound)`, which answers a private-to-the-crate enum
`Pins`: `Unneeded` (another class, no store handed, or a store outside the
root), `Folders` (every folder strictly between the workspace root and a store
lying strictly inside it, from the root outwards, each spelled as a prefix of
the store's handed path, so none for a store directly in the root) and
`Unplaceable` (a root or store that is no absolute path in its normal form, so
a relative path, a `.` or `..`, a doubled or trailing separator, or a store
that is the root itself). An unplaceable store is refused, never taken to lie
outside. `pinned` makes the one observation, the root's canonical form
(`std::fs::canonicalize`, absent where the root cannot be resolved), and hands
it to the pure decision `pins_in`, which compares the root both as spelled and
as that form by whole components (`Path::strip_prefix`, so `/w2/x` is not
inside `/w`). Both spellings are compared because U8h hands the store at its
canonical place while the workspace root stays as spelled: on macOS
`/var/folders/…/work` holds `/private/var/folders/…/work/.forge/secrets.env`.
This follows the commission's correction of 2026-10-25, now written into the
design row itself (design.md's U8i paragraph), which superseded the staged
row's "paths compared as spelled" for this containment test; the emitted
entries keep the spellings Codex is handed (the root as spelled, the folders
and the deny as the store's canonical spelling). A relative root is
unplaceable whatever it resolves to, so no answer turns on the process's
directory.

`rendered` writes each pinned folder between the root's write and the store's
deny, and the root alone beside an unplaceable store. `rendering` accepts a
write only of the root or of a pinned folder, and the private `classed` reads
the class from the count of writes once every path is known to be named once:
nothing written is read-only, a pin written without the root is `Unrooted`,
the root with every pin is workspace-write, and the root without every pin, or
beside an unplaceable store, is the new `Unisolated::Unpinned`. The read-only
class needs no pins and reads back beside every bound. Every existing refusal
keeps its cause and text.

In `crates/brokkr-protocol/src/adapters/codex.rs`, `launch_on_host` calls
`launch_on` with this host's `Sandboxing` (`Paths` on macOS, `Mounts`
elsewhere, chosen by `cfg!` in a constant). After the existing door
(`admitted`), `pinnable` refuses with the new `Unlaunchable::Unpinnable` a
launch that is sealed, whose class written in its arguments is workspace-write,
and whose handed store `pinned` answers `Folders` or `Unplaceable` for, where
the sandbox matches paths. It matches `Sandboxing` and `Pins` exhaustively,
with no wildcard arm. Whether a launch is sealed (launch record beside its
sealed serving inputs) is now derived once, by the private `sealed`, which
`admitted` and `pinnable` both call. Only after both doors is the launch
composed, so the version probe never runs for a refused launch.

`crates/brokkr-protocol/src/native_controls/grammar.rs` names `pinned` and
`Pins` in its existing `pub(crate) use config::Unisolated;` re-export, now
`pub(crate) use config::{pinned, Pins, Unisolated};`; its line count holds at
1,427. No other production file moved and no new module was added.

## The argv that moved

| Suite | Test | What it now pins |
| --- | --- | --- |
| `adapters/tests/codex_isolation.rs` | `a_workspace_write_seat_pins_every_folder_to_a_store_inside_its_root` | Exact cold, cold replacement and rejoin argv through `codex_launch_and_cold` (so through the inline judgment and the final check) for `/w/.forge/secrets.env` (pin `/w/.forge`), `/w/a/b/secrets.env` (pins `/w/a`, `/w/a/b`, in that order), `/w/secrets.env` and `/w2/a/secrets.env` (none), and the read-only class with `/w/.forge/secrets.env` (none). |
| same | `a_profile_without_its_pins_refuses_at_the_final_check_and_the_inline_judgment` | The final check admits the rendered command and refuses it with the pin removed, `cannot be read: it` `Unpinned(5)`; `judge_inline_codex_launch` admits the pinned profile and refuses the root-only one with `Unpinned(2)`. |
| same | `a_root_spelled_through_a_link_pins_the_folder_of_its_canonical_store` | A root `<tmp>/link` linked to `<tmp>/real`, store `<tmp>/real/.forge/secrets.env`: the rejoin and its cold replacement write `<tmp>/link` and `<tmp>/real/.forge` and deny the store. |
| same | `a_sealed_workspace_write_launch_with_an_inside_store_refuses_where_paths_are_matched` | Under `Paths`, the linked root's launch refuses `Unpinnable` from `pinnable` and as `Unlaunched::Door(Unpinnable)` from `launch_on` with the recording shim never run; a store directly in the root and one through a `..` (unplaceable) refuse too; the read-only class, a store outside the root, no written class, a launch missing either sealed input, and every launch under `Mounts` pass, the last probing its version. |
| same | `an_agents_harness_fragment_is_launched_as_the_store_denying_profile` | Now also run with the store inside the root: the shipped `hands.harness.work` fragment's profile pins `/w/.forge`, the `hands.harness.gate` read-only split pins none. |
| same | `each_codex_door_refusal_keeps_its_text` | `Unpinnable`'s whole text, pinned once. |
| `native_controls/tests/isolation.rs` | `a_profile_reads_as_workspace_write_only_with_every_folder_to_an_inside_store_written` | For store `/w/a/b/store`: the exact rendering of both classes; both pin orders read workspace-write, cold and rejoined; missing either pin or both refuses `Unpinned`, a read-only profile writing the pins and an extra written folder refuse `Unrooted`, a pin named twice refuses `Classless`, each through `profile` and `read_state`. |
| same | `a_store_outside_the_root_pins_nothing_and_one_its_paths_cannot_place_refuses` (renamed from `no_folder_is_pinned_for_a_store_not_strictly_inside_the_root`) | Every bound renders the root alone. A store directly in the root and a lookalike `/w2` read workspace-write; a store through `..`, through `//`, a relative store, a relative root and a root with a trailing `/` refuse `Unpinned(5)`; the store that is the root names one path twice (`Classless(5)`); the read-only class reads `ReadOnly` beside every one. |
| same | `a_root_spelled_through_a_link_pins_the_folder_of_the_canonical_store_inside_it` | A linked root holds both the canonical store and the store spelled through the link; each renders its folder at the store's spelling and refuses the root-only profile `Unpinned`. A store that is the root's canonical form (`<tmp>/real` beside root `<tmp>/link`, no path named twice) refuses `Unpinned`. |
| same | `each_isolation_fault_reads_in_its_own_words` | `Unpinned`'s whole text, pinned once. |
| `brokkr-cli/tests/driver_conformance.rs` | `the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` | The real driver's rejoin argv, whose store the engine keeps at `<canonical work>/.forge/secrets.env`, now carries `"<canonical work>/.forge"="write"`. On macOS the two sealed workspace-write `Single` shapes are refused at the door, so there the test asserts they recorded no launch row and the provider shim never ran (`argv.log` absent), and moves on; the no-hands member shapes and every Linux assertion are unchanged. The file holds at its 3,404-line baseline: the doc comment was rewrapped to the 100-column width (no word changed), and two format calls took one line each. |
| `adapters/tests.rs` (frozen at 18,848) | `an_eligible_codex_rejoin_and_its_cold_replacement_are_each_served_as_checked` | Its store moved from `<root>/secrets.env` to `<root>.secrets.env`, beside the root, so its end-to-end `invoke` is admitted on macOS too; it tests U9c2's rejoin serving, not U8i, and its profile literal is unchanged. One line for one line. |

The other suites that pin a profile hand a store outside the workdir and did
not move: `crates/brokkr-runtime/tests/capability_launch/codex_store.rs`
(`/s/secrets.env` beside `/w`) and
`crates/brokkr-runtime/src/engine/secret_threading_tests.rs` (the engine's
store beside `/w`); `crates/brokkr-runtime/src/engine/broker/admission/tests.rs`
carries no profile literal with a store inside the root, and
`crates/brokkr-protocol/src/adapters/tests.rs` carried one only directly in the
root, which pins nothing on Linux and was moved beside the root for macOS (row
above). No test moved between files. Two helpers were
added to `codex_isolation.rs` (`marking`, `recording`) so the new tests share
the existing host test's shim instead of repeating it (the clone gate's two
findings on the first draft).

## Removals

Re-run in the second visit against the final code. Each removal compiled,
failed the tests named, and passed again once restored
(`cargo test -p brokkr-protocol --lib isolation`: 35 passed after each
restore, and the whole protocol suite, 1,077, after the last).

| Removal | Failed | Failing assertion |
| --- | --- | --- |
| M1: the renderer's pins (`Pins::Folders(_) => Vec::new()` in `rendered`) | 7 tests: both reader tests' rendering pins and five adapter tests | `isolation.rs:433` the rendering lacks `"/w/a"="write","/w/a/b"="write"`; `isolation.rs:558` it lacks `"<tmp>/real/.forge"="write"`; the adapter launches refuse their own pin-less profile at the final check, so `codex_launch_and_cold(..).unwrap()` panics (`codex_isolation.rs:615`, `:726`, `:743`, `:808`), and the `Mounts` launch `left: Some(Composition(".. without every folder to the secrets store inside it ..")) right: None` (`:863`) |
| M2: the reading's acceptance of a pin (the guard's `!folders.contains(&path)` replaced by an always-true comparison) | 6 tests | `isolation.rs:448` `left: Err(Unrooted(5)) right: Ok(Some(WorkspaceWrite))`; `codex_isolation.rs:863` `left: Some(Composition(".. a permission profile that writes a path other than the seat's workspace root ..")) right: None` |
| M3: the reading's requirement (`\|\| count > 0` added to the workspace-write arm's guard in `classed`) | 4 tests | `isolation.rs:316` `left: None right: Some(Unpinned(5))` in both reader tests; `isolation.rs:517` `left: Ok(Some(WorkspaceWrite)) right: Err(Unpinned(5))` in the not-inside test; `codex_isolation.rs:762` the final check refused only by its rebuild comparison ("departs at argument 6"), not with `Unpinned` |
| M4: the macOS refusal (the `Paths` arm's guard answering `Ok(())`) | `a_sealed_workspace_write_launch_with_an_inside_store_refuses_where_paths_are_matched` | `codex_isolation.rs:837` `left: Ok(()) right: Err(Unpinnable)` |
| M4b: the door's call (`pinnable` not called from `launch_on`) | the same test | `codex_isolation.rs:840` `left: None right: Some(Door(Unpinnable))` |
| M5: the canonical root (`canonicalize(..).ok().filter(\|_\| false)` in `pinned`) | 3 tests: both linked-root tests and the macOS refusal | `isolation.rs:558` and `codex_isolation.rs:809` the rendering lacks `"<tmp>/real/.forge"="write"`; `codex_isolation.rs:837` `left: Ok(()) right: Err(Unpinnable)` |
| M6a: an unplaceable store taken as outside (`Pins::Unplaceable => Some(1)` in `classed`) | `a_store_outside_the_root_pins_nothing_and_one_its_paths_cannot_place_refuses`, the linked-root reader test | `isolation.rs:517` `left: Ok(Some(WorkspaceWrite)) right: Err(Unpinned(5))`; `isolation.rs:316` `left: None right: Some(Unpinned(5))` (the store that is the root's canonical form) |
| M6b: the normal-form gate (its condition made unsatisfiable by `&& store.is_empty()`) | `a_store_outside_the_root_pins_nothing_and_one_its_paths_cannot_place_refuses` | `isolation.rs:515` the `..` store renders `"/w/.."="write","/w/../a"="write"`, folders outside the root, against the root alone |
| M6c: a store outside the root taken as unplaceable (`None => Pins::Unplaceable` in `pins_in`) | 13 tests, the not-inside test among them | `isolation.rs` `left: Err(Unpinned(5)) right: Ok(Some(WorkspaceWrite))` for `/w2/a/store`; every U9c2 test whose store lies beside `/w` refuses `Unpinned` (`codex_isolation.rs:285` `left: Err("carries '--config' (argument 2), a permission profile writing the workspace root without every folder ..") right: Ok(())`) |
| M6d: a store that is the root's canonical form taken as directly inside (`Some(0) => Pins::Folders(Vec::new())`) | the linked-root reader test | `isolation.rs:316` `left: None right: Some(Unpinned(5))` |
| M6e: the read-only class refused beside an unplaceable store (`Pins::Unplaceable => return Err(Unpinned(at))` in `classed`) | `a_store_outside_the_root_pins_nothing_and_one_its_paths_cannot_place_refuses` | `left: Err(Unpinned(5)) right: Ok(Some(ReadOnly))` |
| M7: the door's refusal of an unplaceable store (`Pins::Unneeded \| Pins::Unplaceable => false` in `pinnable`) | `a_sealed_workspace_write_launch_with_an_inside_store_refuses_where_paths_are_matched` | `codex_isolation.rs:848` the `..` store, `left: Ok(()) right: Err(Unpinnable)` |

M6a, M6b, M6c and M6e bind the not-inside test the review found unbound
(the chief's finding 1): each fails it with its own assertion.

The moved driver pin binds as well: in the first visit, with its old pin-less
expectation, `the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`
failed at `driver_conformance.rs:2694` and passed once the pin was restored.

### The macOS door, simulated on Linux

No macOS host is available, so the second visit simulated one: `HOST` was
set to `Sandboxing::Paths` (`if true || cfg!(target_os = "macos")`), and
the driver test's branch condition likewise, and the suites that launch Codex
were run. `driver_conformance` passed 27 of 27 with the macOS branch taken.
With the branch restored to `cfg!(target_os = "macos")` but `HOST` still
`Paths` (Linux's expectations under the macOS door), the test failed at
`driver_conformance.rs:3225` ("a cold launch row"), the review's finding 7
observed. Under the simulated door, the protocol library failed one test, the
frozen `an_eligible_codex_rejoin_and_its_cold_replacement_are_each_served_as_checked`
(`adapters/tests.rs:17962`, its `invoke` refused); with its store moved
beside the root it passed. brokkr-runtime (its library, `capability_launch`,
`it`, `operated_repo`, `queued_launch`), brokkr-cli's `it` (697,
`capability_broker` skipped), its library and binary (1,074),
`heap_codex` and `transcript_surfaces` all passed under the simulated door.
Both changes were then restored, and that restore was checked by a search
for `true ||` finding nothing. The runtime and CLI suites ran under the
simulation before `pinned`'s last simplification (it dropped a guard that
only skipped resolving a relative root, which the decision already refuses);
they were re-run on the real build afterwards (next section). This is a
simulation of the door's decision on Linux, not macOS host evidence.

## Gates on this tree

In the second visit, on the final code: `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
are clean. The brokkr-protocol suite passed whole (`--all-features --locked`:
1,077 unit tests, 3 ignored, and its three integration binaries).
brokkr-runtime passed its library (1,408) and every integration binary,
`capability_launch` (76) included. brokkr-cli passed its library and binary
(1,074, 1 ignored), `driver_conformance` (27), `heap_codex`,
`transcript_surfaces`, and the `it` binary with `capability_broker` skipped
(697, 3 ignored). The other four heap binaries and brokkr-core, brokkr-view,
brokkr-store, brokkr-bridge and brokkr-seatbelt-probe were run only in the
first visit, where they passed (17 binaries); nothing they build on moved
since. The `capability_broker` module's namespace tests stalled past 60
seconds in the first visit's box and were stopped by the timeout; they nest a
namespace the box refuses, touch none of this unit's code, and are left to
the host and CI. `compile --bundle recipes/self` succeeded,
`openspec validate --all --strict` passed 36 of 36, `typos --hidden` and
`git diff --check` are clean, and `quality/ratchet.sh files`, `clones` and
`api` hold: no `quality/public-api/*.txt` line moved. `quality/file-lines.txt`
carries the four moved rows: `codex.rs` 309 to 386, `config.rs` 375 to 498,
`codex_isolation.rs` 601 to 865 (its row was stale at 601 against 630 on
main) and `tests/isolation.rs` 829 to 993; `driver_conformance.rs` (3,404),
`adapters/tests.rs` (18,848) and `grammar.rs` (1,427) hold at their rows.
Every new function is under 25 lines; `rendering`'s class decision moved into
`classed` to keep its complexity well inside 15. No suppression was added,
and no witness or compose input changed.

## The review's return

The review of `9be12127` (run `0065-slice-two-unit-u8i-see-the--b015d453`,
residual, medium) returned nine findings; this visit answers them as follows.

| Finding | Answer |
| --- | --- |
| 1, medium: the not-inside test had no recorded binding | M6a, M6b, M6c and M6e each fail it, recorded above. |
| 2, low: design.md still said "compared as spelled" | The U8i paragraph now states the canonical-root containment (the commission's correction of 2026-10-25) and the refusal of a store its paths cannot place, at the reading and at the macOS door. |
| 3, low: a wildcard arm over `Sandboxing` | `pinnable` matches `Mounts`, `Paths` with its guard, and `Paths`; the `Pins` match is exhaustive too. |
| 4, low: the sealed predicate written twice | The private `sealed(input)`, called by `admitted` and `pinnable`. |
| 5, low: observation inside the decision | `pinned` makes the one observation (the root's canonical form) and hands it to the pure `pins_in`; a relative root is refused by the decision whatever it resolves to, so the relative-root case no longer depends on the process's directory. Each of the renderer, the reading and the door still observes once per call: a root re-linked between two of those calls makes the reading's pins differ from the rendered ones, which refuses (`Unrooted` or `Unpinned`), so a disagreement fails closed (read from the code, not provoked by a test); at the macOS door, a root re-linked between the door and the composition is not caught (recorded below). |
| 6, low (security residual): uncertain containment taken as outside | `Pins::Unplaceable` separates it, the reading refuses it with `Unpinned` and the macOS door with `Unpinnable`, and the read-only class still reads beside it (M6a, M6b, M6e, M7). A root that cannot be resolved is still compared as spelled alone (below). |
| 7, info: the driver test's macOS expectation | Adapted, and observed under a simulated `Paths` door (above), which also found the frozen `adapters/tests.rs` test it now admits. |
| 8, info: the reading accepts pins in any order | Unchanged: the renderer emits X31's order, and the reading's acceptance of another order is recorded as unmeasured below. |
| 9, info: pending host and CI evidence | Still pending, below. |

## Limitations and out of scope

The store is compared as handed, in its normal form. The engine hands its
canonical place (U8h), so a store spelled through some other link than the
root's is not recognised as inside and pins nothing; under U8h no such store
is handed. A store or root not in normal form is refused rather than placed.
A root whose canonical form cannot be read is compared as spelled alone: the
engine resolves the workdir before dispatch, and refusing every unresolved
root would refuse the U9c2 suites' `/w` fixtures, which no host holds, so this
fallback is kept and named here. The renderer, the reading and the door each
resolve the root once per call; their disagreement fails closed at the
reading, but at the macOS door, a root re-linked between the door's
resolution and the composition's is not detected. The reading accepts the
pins in any order beside the root's; only the rendered order (root, pins
outwards, deny) was measured by X31. Symlinked folders between the root and
the store are not followed (U9c2's L2). The hands box's pins are U8h's; dsh's
native tools are U1d3's; a store outside the root has nothing to pin.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) on a host outside the box or in
CI; the macOS build and tests (the `Paths` refusal is exercised on Linux by
injection, which is not macOS host evidence); remote CI on the final head;
and U0b's re-measurement of X29 under the rendered profile once U8i lands.
