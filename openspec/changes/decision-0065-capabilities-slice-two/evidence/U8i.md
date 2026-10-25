# U8i — Pin the store's folder in a Codex seat's profile

Tasks 45.11 and 45.12; requirements SI1 and SI2, with SD2 and SD4 for the
verification duty. Branch `s2/U8i`, cut from main at b8e2caff, recipe
`claude-flash-dsh-adv`, run `0065-slice-two-unit-u8i-see-the--b015d453`.
Everything below was observed in this run's implement seat, on this tree,
on Linux 6.17. What could not be run is listed as pending at the end.

## What changed

The renderer and the bound reading in
`crates/brokkr-protocol/src/native_controls/grammar/config.rs` share one
derivation, `pinned(class, bound)`. For the workspace-write class only, and
only where a store is handed, it answers the folders strictly between the
workspace root and a store lying strictly inside it, from the root outwards,
each spelled as a prefix of the store's handed path (empty for a store
directly in the root), and `None` for a store outside the root. Containment
is by whole components (`Path::strip_prefix`), so `/w2/x` is not inside `/w`.
A relative root, a `..` between the root and the store, and a store that is
the root itself pin nothing. The root is compared both as spelled and as its
canonical form (`std::fs::canonicalize`, the spelling alone where the root
cannot be resolved), because U8h hands the store at its canonical place while
the workspace root stays as spelled: on macOS `/var/folders/…/work` holds
`/private/var/folders/…/work/.forge/secrets.env`. This follows the
commission's correction of 2026-10-25, which supersedes the staged design
row's "paths compared as spelled" for this containment test; the emitted
entries keep the spellings Codex is handed (the root as spelled, the folders
and the deny as the store's canonical spelling).

`rendered` writes each pinned folder between the root's write and the store's
deny. `rendering` accepts a write only of the root or of a pinned folder, and
the new private `classed` reads the class from the count of writes once every
path is known to be named once: nothing written is read-only, a pin written
without the root is `Unrooted`, the root with every pin is workspace-write,
and the root without every pin is the new `Unisolated::Unpinned`. Every
existing refusal keeps its cause and text.

In `crates/brokkr-protocol/src/adapters/codex.rs`, `launch_on_host` now calls
`launch_on` with this host's `Sandboxing` (`Paths` on macOS, `Mounts`
elsewhere, chosen by `cfg!` in a constant). After the existing door
(`admitted`), `pinnable` refuses with the new `Unlaunchable::Unpinnable` a
launch that is sealed (launch record beside its sealed serving inputs), whose
class written in its arguments is workspace-write, and whose handed store lies
inside the root by `pinned`, where the sandbox matches paths. Only then is the
launch composed, so the version probe never runs for a refused launch.

`crates/brokkr-protocol/src/native_controls/grammar.rs` gained `pinned` in its
existing `pub(crate) use config::Unisolated;` re-export, now
`pub(crate) use config::{pinned, Unisolated};`; its line count holds at 1,427.
No other production file moved and no new module was added.

## The argv that moved

| Suite | Test | What it now pins |
| --- | --- | --- |
| `adapters/tests/codex_isolation.rs` | `a_workspace_write_seat_pins_every_folder_to_a_store_inside_its_root` | Exact cold, cold replacement and rejoin argv through `codex_launch_and_cold` (so through the inline judgment and the final check) for `/w/.forge/secrets.env` (pin `/w/.forge`), `/w/a/b/secrets.env` (pins `/w/a`, `/w/a/b`, in that order), `/w/secrets.env` and `/w2/a/secrets.env` (none), and the read-only class with `/w/.forge/secrets.env` (none). |
| same | `a_profile_without_its_pins_refuses_at_the_final_check_and_the_inline_judgment` | The final check admits the rendered command and refuses it with the pin removed, `cannot be read: it` `Unpinned(5)`; `judge_inline_codex_launch` admits the pinned profile and refuses the root-only one with `Unpinned(2)`. |
| same | `a_root_spelled_through_a_link_pins_the_folder_of_its_canonical_store` | A root `<tmp>/link` linked to `<tmp>/real`, store `<tmp>/real/.forge/secrets.env`: the rejoin and its cold replacement write `<tmp>/link` and `<tmp>/real/.forge` and deny the store. |
| same | `a_sealed_workspace_write_launch_with_an_inside_store_refuses_where_paths_are_matched` | Under `Paths`, the linked root's launch refuses `Unpinnable` from `pinnable` and as `Unlaunched::Door(Unpinnable)` from `launch_on` with the recording shim never run; a store directly in the root refuses too; the read-only class, a store outside the root, no written class, a launch missing either sealed input, and every launch under `Mounts` pass, the last probing its version. |
| same | `an_agents_harness_fragment_is_launched_as_the_store_denying_profile` | Now also run with the store inside the root: the shipped `hands.harness.work` fragment's profile pins `/w/.forge`, the `hands.harness.gate` read-only split pins none. |
| same | `each_codex_door_refusal_keeps_its_text` | `Unpinnable`'s whole text, pinned once. |
| `native_controls/tests/isolation.rs` | `a_profile_reads_as_workspace_write_only_with_every_folder_to_an_inside_store_written` | For store `/w/a/b/store`: the exact rendering of both classes; both pin orders read workspace-write, cold and rejoined; missing either pin or both refuses `Unpinned`, a read-only profile writing the pins and an extra written folder refuse `Unrooted`, a pin named twice refuses `Classless`, each through `profile` and `read_state`. |
| same | `no_folder_is_pinned_for_a_store_not_strictly_inside_the_root` | A store directly in the root, a lookalike `/w2`, a `..`, the root itself and a relative root render the root alone; each reads workspace-write, but the store that is the root, which names one path twice (`Classless`). |
| same | `a_root_spelled_through_a_link_pins_the_folder_of_the_canonical_store_inside_it` | A linked root holds both the canonical store and the store spelled through the link; each renders its folder at the store's spelling and refuses the root-only profile `Unpinned`. |
| same | `each_isolation_fault_reads_in_its_own_words` | `Unpinned`'s whole text, pinned once. |
| `brokkr-cli/tests/driver_conformance.rs` | `the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root` | The real driver's rejoin argv, whose store the engine keeps at `<canonical work>/.forge/secrets.env`, now carries `"<canonical work>/.forge"="write"`. The edit is size-neutral (the file holds at its 3,404-line baseline: three comment lines became one, the format call took three). |

The other suites that pin a profile hand a store outside the workdir and did
not move: `crates/brokkr-runtime/tests/capability_launch/codex_store.rs`
(`/s/secrets.env` beside `/w`) and
`crates/brokkr-runtime/src/engine/secret_threading_tests.rs` (the engine's
store beside `/w`); `crates/brokkr-runtime/src/engine/broker/admission/tests.rs`
and `crates/brokkr-protocol/src/adapters/tests.rs` carry no profile literal
with a store inside the root. No test moved between files. Two helpers were
added to `codex_isolation.rs` (`marking`, `recording`) so the new tests share
the existing host test's shim instead of repeating it (the clone gate's two
findings on the first draft).

## Removals

Each removal compiled, failed the tests named, and passed again once
restored (`cargo test -p brokkr-protocol --lib isolation`: 35 passed after
each restore).

| Removal | Failed | Failing assertion |
| --- | --- | --- |
| M1: the renderer's pins (`let pins: Vec<&str> = Vec::new();` in `rendered`) | 7 tests: both reader suites' rendering pins and five adapter tests | `isolation.rs` `assert_eq!(rendered, Some(profile(&pinned)))`; the adapter launches refuse their own pin-less profile at the final check, so `codex_launch_and_cold(..).unwrap()` panics |
| M2: the reading's acceptance of a pin (`"write" if path != bound.workdir`) | 6 tests | `isolation.rs` profile read `left: Err(Unrooted(5)) right: Ok(Some(WorkspaceWrite))`; the `Mounts` launch `left: Some(Composition(".. a permission profile that writes a path other than the seat's workspace root ..")) right: None` |
| M3: the reading's requirement (`(true, _) => Ok(WorkspaceWrite)` in `classed`, re-run after the split) | 3 tests | `isolation.rs:316` `left: None right: Some(Unpinned(5))` in both reader tests; `codex_isolation.rs:762` the final check refused only by its rebuild comparison ("departs at argument 6"), not with `Unpinned` |
| M4: the macOS refusal (`(Sandboxing::Paths, true) => Ok(())`) | `a_sealed_workspace_write_launch_with_an_inside_store_refuses_where_paths_are_matched` | `left: Ok(()) right: Err(Unpinnable)` |
| M4b: the door's call (`pinnable` not called from `launch_on`) | the same test | `left: None right: Some(Door(Unpinnable))` |
| M5: the canonical root (`canonicalize(..).ok().filter(\|_\| false)`) | 3 tests: both linked-root tests and the macOS refusal | the rendering lacks `"<tmp>/real/.forge"="write"`; `left: Ok(()) right: Err(Unpinnable)` |

The moved driver pin binds as well: with its old pin-less expectation,
`the_compiled_live_inline_codex_shapes_rejoin_their_provider_confirmed_root`
failed at `driver_conformance.rs:2694`, and passed once the pin was restored.

## Gates on this tree

`cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
are clean. The brokkr-protocol suite passed whole (`--all-features --locked`:
1,077 unit tests, 3 ignored, and its three integration binaries).
brokkr-runtime passed its library (1,408) and every integration binary,
`capability_launch` (76) included. brokkr-cli passed its library and binary
(1,074, 1 ignored), `driver_conformance` (27), `transcript_surfaces` and the
five heap binaries, and the `it` binary with `capability_broker` skipped (697,
3 ignored). The `capability_broker` module's namespace tests stalled past 60
seconds in this seat's box and were stopped by the timeout; they nest a
namespace the box refuses, touch none of this unit's code, and are left to
the host and CI. brokkr-core, brokkr-view, brokkr-store, brokkr-bridge and
brokkr-seatbelt-probe passed (17 binaries). `compile --bundle recipes/self`
succeeded, `openspec validate --all --strict` passed 36 of 36, `typos --hidden`
and `git diff --check` (staged and unstaged) are clean, and
`quality/ratchet.sh files`, `clones` and `api` hold: no
`quality/public-api/*.txt` line moved. `quality/file-lines.txt` carries the
four moved rows: `codex.rs` 309 to 374, `config.rs` 375 to 451,
`codex_isolation.rs` 601 to 859 (its row was stale at 601 against 630 on
main) and `tests/isolation.rs` 829 to 968. Every new function is under 25
lines; `rendering`'s class decision moved into `classed` to keep its
complexity well inside 15. No suppression was added, and no witness or
compose input changed.

## Limitations and out of scope

The store is compared as handed. The engine hands its canonical place (U8h),
so a store spelled through some other link than the root's, or with a `..`
inside the root, is not recognised as inside and pins nothing; under U8h no
such store is handed. A root whose canonical form cannot be read is compared
as spelled. Symlinked folders between the root and the store are not
followed (U9c2's L2). The hands box's pins are U8h's; dsh's native tools are
U1d3's; a store outside the root has nothing to pin.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) on a host outside the box or in
CI; the macOS build and tests (the `Paths` refusal is exercised on Linux by
injection, which is not macOS host evidence); remote CI on the final head;
and U0b's re-measurement of X29 under the rendered profile once U8i lands.
