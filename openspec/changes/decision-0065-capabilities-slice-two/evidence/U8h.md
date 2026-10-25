# U8h — Hide the secrets store in every workspace box

Tasks 45.9 and 45.10; requirements SI1 and MB4, with SD2 and SD4 for the
verification duty. Branch `s2/U8h`, cut from main at 9545c18c, recipe
`claude-flash-dsh-adv`. Run 1 (`0065-slice-two-unit-u8h-see-the--7c02db9f`)
built dea88c51, run 2 (`0065-slice-two-unit-u8h-see-the--dc30b1ab`) built
95eb52ac, run 3 (`0065-slice-two-unit-u8h-see-the--aad92280`) built
634bd615, run 4 (`0065-slice-two-unit-u8h-see-the--84169be8`) built
529defe7 and run 5 (`0065-slice-two-unit-u8h-see-the--2907b944`) built
5c825688; each review stopped its run REVIEW-SECURITY-HOLD. This is run 6
(`0065-slice-two-unit-u8h-see-the--08774615`). It continues run 5's work,
which the controller replayed onto 9545c18c, and repairs every finding of
run 5's review under the operator's rulings of 2026-10-25 on run 5 (design
(18) and (19)). Results marked run 6 were observed in run 6's implement
seat, on this tree, on Linux 6.17. Results marked run 5 or earlier were
observed in those runs' seats and are carried. Every test they name passes
on this tree (the suites below), except where run 6 says it replaced or
changed a test. What could not be run is listed as pending at the end.

## Run 6 at a glance

| Finding | What run 6 did | Proof |
| --- | --- | --- |
| H1 (high) | The engine keeps the store's place once per run in a private `OnceLock` (`Engine::store_place`), filled the first time anything asks (`Engine::kept_store`, `engine/boxed.rs`). Making the store, the path a seat's input names (`secrets_store_path`), the path MB4's excluded store reads and the place admission seals all read that one kept place. Nothing resolves the place again for the run. Before every boxed spawn, and before every unboxed spawn once an input has named the store, the configured spelling is checked to still lead to the kept place (`Engine::led`), and the spawn refuses exactly if it does not. | `the_run_keeps_one_store_place_and_refuses_a_spelling_that_left_it`; removals Q2, V2, W2, S2 |
| L1 (low) | `store_place` returns `std::io::Result` and has no fallback. A way no lookup passes (through a file, a folder the user may not search, more than 40 links) is an error, which the engine keeps and which refuses every spawn as `Unprepared::Unresolved`. `make_store` makes only the file, never a folder, so a missing folder refuses as `Unstored(.., NotFound)`. The box refuses a sealed spelling that no longer resolves as `Unobserved`. | `a_store_whose_folder_cannot_be_resolved_refuses_every_spawn`, `the_engine_makes_a_missing_store_owner_only_or_refuses`, `a_boxed_spawn_makes_its_store_before_launch_or_refuses`, the protocol ruling 17 test; removals F, P, U |
| L2 (low) | The parity test now asserts every filesystem step the box emits, in order, by its exact flag (bind vs bind-try kept apart), with nothing filtered: `/proc`, `/dev` and the box's folders, the scratch identity files, home and tmp, a bind's mask, the store's hide, the Git hooks tmpfs and config, and the artifact root. It then holds the description to exactly the steps it describes. | `every_mount_the_box_makes_is_the_store_s_description_in_order`; removal M2 |
| L3 (low) | Recorded under "Limits and scope", apart from H1's in-run refusal. | — |
| I1 (info) | Recorded under "Limits and scope". `engine/broker.rs` is unchanged. | — |
| R1 | This file records findings and observations. It does not argue for any verdict. | — |

## What changed

The store's path rides in the sealed reach (operator ruling 1 of
2026-10-23). `hands::seal` takes a fifth argument. Since run 6 it is
`store: Option<(&Path, &Path)>`, the kept place and the configured
spelling, sealed as handed and never resolved again inside `seal`. Doctor's
box probe and the isolation probe pass `None`, each in its one changed
`seal` line. `Sealed` gains a `store` field that must be written, `null`
where none is sealed. A reach serialized before U8h has no `store` key and
no longer reads. A reservation holding one refuses with MB3's identity
cause, and `hands serve` and `hands exec` refuse its `--reach` before any
session or box is made. The store's file identity is not sealed: a store
`brokkr secrets set` replaces stays hidden in every later box (ruling 5).

Run 4 sealed the store's canonical path (ruling 15). `hands::store_place`
makes the configured spelling absolute and walks it one entry at a time,
outside any box, following every symbolic link on it, the final entry's
included, up to Linux's 40. A missing entry leaves the rest as spelled. No
link can stand there, since nothing stands there. `drifted` re-seals with
no store, since it compares only the sources. Run 3's writable-mount link
classification is removed, not extended.

Run 5 named one path authority (ruling 17), but it did not keep it.
Correction to run 5's record. Run 5's evidence said the engine's launch
refusal "ends the spawn before any driver reads the store, so the seat
input built with the moved place is never acted on". That overstated it.
Run 5's `secrets_store_path` resolved the configured spelling again on
every call. Admission resolved it again inside `seal`. The launch check
compared only the current resolution with the sealed place, never the path
already in a seat's input. Nothing compared an unboxed seat's input with any
authority at all. Review H1 named this. Run 6 replaces it as follows.

- The engine keeps the store's place for the run (ruling 18). `Engine`
  gains one private field, `store_place: OnceLock<KeptStore>`, with one
  initialiser in each of its three struct literals. `KeptStore` holds the
  configured spelling, made absolute, and either the place `store_place`
  resolved it to or the error kind it failed with. `Engine::kept_store()`
  fills it the first time anything asks and never again for that engine.
- `Engine::secrets_store_path()` moved from `engine.rs` to `engine/boxed.rs`
  and returns the kept place. `engine/input.rs` (the seat's
  `secrets_file`), `engine/broker.rs` (MB4's excluded store) and the Codex
  launch judgment, which reads that input, therefore all name the kept place
  with no edit to `input.rs` or `broker.rs`. Where the spelling could not be
  resolved, `secrets_store_path` returns the spelling. No driver or box is
  handed it, because every spawn that would read it refuses `Unresolved`
  first (next bullet).
- `Engine::prepared` runs for every spawn, boxed or not. Its one call in
  `compose_at` changed in place to
  `self.prepared(&artifacts, hands.filter(|_| boxed))`. A boxed spawn makes
  the artifact root, then checks the kept place (`Engine::led`). An unboxed
  spawn checks only once a place is kept, which is when a seat's input has
  named the store. `led` refuses `Unprepared::Unresolved(spelling, kind)`
  where the kept place was never resolved, or where the configured spelling
  no longer resolves. It refuses `Unprepared::Strayed(spelling, place)`
  where the spelling now leads elsewhere. A boxed spawn then refuses
  `Strayed` where the reach admission sealed holds another place, and only
  then makes the store at the kept place.
- Admission seals the kept place beside the spelling
  (`self.kept_store().sealed()` into `boxed_reach`, then `hands::seal`). An
  unresolved store is sealed as its spelling at both. No box is built from
  that reach, because the spawn refuses first.
- Every box still refuses `Strayed` where `store_place` of the sealed
  spelling is no longer the sealed place (`Store::led`, both observations).
  Since run 6 it refuses `Unobserved(spelling, kind)` where the spelling no
  longer resolves at all.
- A reservation whose `store` is a bare path, as run 4 wrote it, no longer
  reads, the same as one without `store`.

`engine.rs` stands at 3314 lines, two below its frozen 3316. That is +3
for the field and its doc, +3 initialisers, and −8 for `secrets_store_path`
moving out. Ruling 18 allowed up to 3324. The three initialisers sit just
above each literal's `world` field. Beside `secrets_file` they fell inside a
clone two of the literals already share, which moved its baselined jscpd
fingerprint (`quality/ratchet.sh clones` reported "13 lines,
brokkr-runtime/src/engine.rs:539 and brokkr-runtime/src/engine.rs:590").
Moved above `world`, the same three lines leave that clone as it was, and
`clones` holds. No `store.create_run` or `run_started` line moved.

The engine makes the store before a boxed launch, as it makes the artifact
root (ruling 2). `Engine::prepared` in `engine/boxed.rs` makes the root,
then checks the kept place, then calls `make_store` there. Since run 6,
`make_store` makes one empty file at 0600 in a folder that already stands
and never makes a folder (ruling 18, L1). It keeps an existing store
untouched, whatever its mode (ruling 8). A store it cannot make refuses the
spawn with `Unstored`: `dispatch refused: secrets store cannot be hidden
from workspace hands: the store '<path>' cannot be made: <kind>`. A missing
folder gives `entity not found`.

`Unprepared::Unresolved` reads `dispatch refused: secrets store cannot be
hidden from workspace hands: the configured store '<spelling>' cannot be
resolved to its place: <kind>`.

The box hides the store in the private child `hands/sealed/hidden.rs`,
driven from `sealed::mount_binds`. `hands.rs` changed lines in place only
and stays at 1094 lines. Run 6 split `hidden.rs` (672 lines) before adding
to it, into two new private children declared from it:
`hands/sealed/hidden/place.rs` (124 lines: `Store`, `store_place`, the walk
and `unlinked`) and `hands/sealed/hidden/table.rs` (83 lines: `judged` with
its non-Linux stand-in, and the mount-table read). `hidden.rs` is now 498
lines. The moved code is unchanged except for `store_place`'s return type,
`Store::sealed`'s argument and `Store::led`'s new error arm.


1. `Hidden::observe` runs first in `mount_binds`. The sealed path must be
   valid UTF-8 whatever the box mounts, outside every mount too (ruling 12,
   review L1), and must still resolve to itself (`unlinked`, ruling 15): it
   must be absolute with only plain entries, and no entry on it that stands
   may be a symbolic link, the final one included. The check reads no
   mount: a link refuses wherever it lies.
2. It describes every place the box mounts once, by host source and box
   destination, in the order the box mounts them (rulings 7 and 13; review
   L2): each `HOST_TOOLCHAIN_BINDS` entry read-only at its own path, the
   worktree at the workdir, the bundle root read-only at `/runtime/bundle`,
   the Git common directory outside the worktree, then the folders the
   worktree pins, then each declared bind followed by its own pins. The
   generated pins are entries of that description (`Held::Pinned`): the
   artifact root's chain, now carried in canonical form, and the store's own
   folder pins.
3. Every judgment reads that one description. For each entry that holds the
   store, a pin's included, the place the box would hide it at must be
   shown, by the last entry over that place, a generated pin's or a bind's
   alike, as the store itself; otherwise the box refuses `Shadowed`. Every
   place a store pin or a hide would be spelled at must be valid UTF-8.
4. `Hidden::pin_worktree` and `Hidden::pin` mount the pins straight from the
   description: the worktree's together with the artifact root's, outermost
   first and each once, before any bind, and each bind's after that bind's
   own mount and before its masks.
5. `Hidden::hide` runs after every bind. It mounts `/dev/null` read-only
   over the store at every destination the description shows it, which
   reads as EACCES, not as an empty file (ruling 10). A store outside every
   mount is not mounted at all. The artifact root's read-only mount still
   comes after.
6. `Hidden::confirm` runs where `box_argv` confirmed the sealed reach once
   the argv is final. It confirms the reach, then observes the store and the
   whole description again with the same bundle root and artifact chain.
   Any change refuses.

The toolchain, worktree, bundle and Git mounts are still emitted by
`box_argv` and `Namespace` (`hands.rs` is frozen and `namespace.rs` is
outside this row). The description mirrors them in their order. Every pin
the box generates is emitted from the description itself. Run 5 bound the
copy with a parity test (review L1). That test filtered out some steps
before comparing, and collapsed bind and bind-try into one kind, so a
change to an omitted step could not fail it (run 5's review L2). Run 6's
version asserts every filesystem step of a real `box_argv` argv, in order,
each with its exact flag and every argument it takes, against a list the
test writes out: `--proc`, `--dev`, the six `--dir` folders, each
toolchain `--ro-bind-try`, the four identity files, home and tmp under the
box's scratch, the worktree, the bundle root, the Git common directory with
its hooks `--tmpfs` and its config `--ro-bind-try`, the two artifact-root
pins, the read-only bind with its `/dev/null` mask, the writable bind with
the store's folder pin, the overlay with its layers, the store's hide and
the artifact root's read-only mount. Each step is marked with how the
description holds it, if it does. The description must equal exactly the
marked steps, in order, by `Held` kind, canonical source and destination.

One emitted byte moved. The artifact root's chain pins are emitted from
their canonical host source, `--bind <canonical folder> <folder as the
workdir spells it>`, where U8a emitted `--bind <spelled> <spelled>`. They are
the same folder: the spelled one resolved to it through the workdir. Where
the workdir is canonical, as in every argv test, the bytes are identical.

Every refusal carries the one new cause `UNHIDDEN_STORE`, "secrets store
cannot be hidden from workspace hands", before the command runs:

| Variant | When |
| --- | --- |
| `Absent` | the store is missing where any mount, the toolchain included, would show it. bwrap would create the file on the host to mount over it, and the box never makes one (ruling 2). |
| `Routed` (run 4) | a symbolic link stands at an entry of the sealed path, the final entry included, wherever it lies (ruling 15). It names the link and the sealed path, and says the path no longer resolves to itself. |
| `Unresolved` (run 4) | the sealed path is not absolute with only plain entries (ruling 15). Since run 6 no engine seals such a path: a store no walk resolves refuses at the engine (`Unprepared::Unresolved`). The protocol fixture still plants one, to show the box refuses it alone. |
| `Strayed` (run 5) | the sealed configured spelling, resolved again by `store_place`, no longer leads to the sealed place (ruling 17). It names the spelling and the place. The engine's own launch refusal, `Unprepared::Strayed`, carries the same cause with `dispatch refused:` before it. |
| `Unspelled` | the sealed path, a store pin's box place or a hide target is not valid UTF-8 (ruling 12). |
| `Linked` | the store is not one regular file of one link the user owns, by store.rs's `single`. |
| `Shared` | the store's mode grants its group or others any permission bit (ruling 8), with the mode in octal. |
| `Shown` | another mount of the store's device shows it, or a directory holding it, at another place within a mount's source. Its own place, which a directory mounted back onto itself shows again, is not another. Run 4 gates it to Linux like `Linked` and `Shared` (review L3). |
| `Rooted` | the artifact root holds the store, and its read-only mount after the hide would show it again (ruling 9). |
| `Shadowed` | the last entry of the description over a place the box would hide the store at, a generated pin's included (run 4), shows another file there (ruling 13). |
| `Overlaid` | an overlay bind shows the store. A pin would bind the host's folder over the overlay and write through to the host. |
| `Gitted` | the store lies in the Git common directory, or one of its folder pins holds it. That pin would cover the hooks tmpfs and read-only config `box_argv` mounted there before. |
| `Unmapped` | the mount table proves no local mount the store lies on (store.rs's `within`). This is also the non-Linux stand-in, as decision 0063 has no box there. |
| `Unobserved` | an entry of the sealed path cannot be observed, with the error kind, such as a file where a folder is needed. Since run 6 it also names the sealed configured spelling, with the error kind, where that spelling no longer resolves at all (ruling 18). |
| `Changed` | the second observation of the store or the description is not the first. |

MB4's `admitted` is untouched (operator ruling 5 of 2026-10-23).
`store.rs` needed no change, and `namespace.rs` was not touched (ruling 16).

## Proof

The tests are in `crates/brokkr-protocol/src/hands/tests/store.rs`, which
is Linux-only and builds real bubblewrap namespaces, and in
`crates/brokkr-runtime/src/engine/boundary_tests/sealed_tests/store_tests.rs`.
There is one test each in `admission/tests/reach.rs` and
`brokkr-cli/src/hands/tests.rs`. Each fixture plants the fake value
`DOCS_TOKEN=fake-u8h-value` at 0600 under a canonical temporary root. None
reads `.forge/` or the real HOME. Run 4 ran the store tests with
`--nocapture` and none printed `skipped:`, so every namespace proof built a
real box. Run 5 ran the new protocol test with `--nocapture`, and it printed
no `skipped:`; removal H below shows a real box's `cat` reading the fake
value once the hide is gone. Run 6 ran the whole protocol store suite with
`--nocapture`, and `grep -ci skipped` over its output counted 0. Run 6's
removal H again showed real boxes reading the fake value. The runtime
namespace test `a_store_the_engine_made_is_hidden_in_the_box_it_composes`
printed no skip under `--nocapture` either.

Three tests outside the owning suites changed in run 6, because they
configure another store on one engine to stand for a later command's
process: `carry/drive_tests.rs`'s
`accepted_args_and_secret_store_drift_park_before_a_member_spawns` and two
places in `operator/retry/tests.rs`'s
`member_admission_keeps_order_checks_secret_inputs_and_writes_nothing_on_refusal`.
With the place kept for the engine's run, each first failed. The drive test
failed on `left: String("panel[1]:ReviewPanel")`, `right: "none"`, since
the input no longer moved. Each now calls the new
`carry/fixture_tests.rs` helper `relaunched_with`, which sets `secrets_file`
and empties the kept place, as a new process would have it (ruling 19). Both
then passed with their assertions unchanged.

Ruling 10's denial is asserted exactly. A workspace call (`execute`, as
`hands serve` runs it) of `cat <path>` returns empty stdout, exit 1 and
`cat`'s EACCES line on stderr. An exec (`run_boxed_in`, as `hands exec`
boxes it) of `exec 2>seen; exec cat <path>` returns `Ok(1)` with that line
in `seen`. The box runs the host's own `cat`. This host's is uutils, which
says `cat: <path>: Permission denied (os error 13)`; GNU coreutils drops the
parenthesis. The assertion admits exactly those two strings.

| Proof | Test | Observed |
| --- | --- | --- |
| A store in the worktree is denied to `hands serve`'s call and to `hands exec`, sealed at its own spelling or through a link a system folder holds at sealing (`outside/link -> ../work/.forge`), and the host's store is unchanged. | `a_store_in_the_worktree_is_denied_to_a_workspace_call_or_an_exec` | Run 4: the reach's serialized `store` is the canonical `work/.forge/secrets.env` for both spellings; both verbs got the exact EACCES; the folder pin follows the worktree bind and the hide follows both. Run 5: the serialized `store` is `{"place": <work>/.forge/secrets.env, "spelling": <the spelling sealed>}` for both spellings, and the test passes. |
| A configured spelling that no longer leads to the sealed place refuses every later box, while an update at the unchanged spelling is hidden (ruling 17, H1). | `a_configured_spelling_that_no_longer_leads_to_the_sealed_store_refuses` | Run 5. Sealed at `outside/route/secrets.env`, `outside/route -> ../work/.forge` (a link outside every mount, as a system link is). `.secrets.env.new` renamed over the store: both verbs got the exact EACCES and the host store read the fake value. Then `route` relinked to `work/moved`, which holds a second fake store: both verbs gave `Strayed(<root>/outside/route/secrets.env, <work>/.forge/secrets.env)`, no marker, exact text, and both stores still read the fake value. Run 6: `route` then relinked to a regular file `outside/file`, and both verbs gave `Unobserved(<root>/outside/route/secrets.env, NotADirectory)`, no marker. |
| The run keeps one store place. Every consumer reads it after the route moves, and every spawn that would use it refuses before anything is made (ruling 18, H1). | runtime `the_run_keeps_one_store_place_and_refuses_a_spelling_that_left_it` (replaces run 5's `a_moved_configured_spelling_refuses_the_engine_s_next_launch`) | Run 6. The seat `work` binds `TOKEN`. `--secrets` is `route/secrets.env`, `route -> real`. The seat's input names `real/secrets.env`. `route` relinked to `moved`. Then the input and `secrets_store_path()` still give `(real/secrets.env, real/secrets.env)`. The boxed compose refused with `Unprepared::Strayed(<base>/route/secrets.env, <base>/real/secrets.env)`. The reach it sealed holds `real/secrets.env` (`sealed_store`). An unboxed compose refused with the same text. Nothing was made at `real/secrets.env` or `moved/secrets.env` (`NotFound`). `route` relinked to a regular file: the boxed compose refused `Unresolved(<base>/route/secrets.env, NotADirectory)`. Relinked back to `real`: no refusal, and `real/secrets.env` was made empty at 0600. Exact text for `Strayed`. |
| A store whose folder cannot be resolved refuses every spawn, with its own cause and nothing standing in for its place (ruling 18, L1). | runtime `a_store_whose_folder_cannot_be_resolved_refuses_every_spawn` | Run 6, for `file/secrets.env` (`file` a regular file) and `shut/secrets.env` (`shut` at mode 0000). The kept place is `Err(NotADirectory)` and `Err(PermissionDenied)`. A boxed compose and an unboxed one each refused `Unprepared::Unresolved(<spelling>, <kind>)`. The reach admission sealed holds the spelling, and no box was built from it. With `shut` reopened it held no entry, so nothing was made. Exact text for the `NotADirectory` case. |
| The engine makes a missing store in a folder that stands, keeps an existing one, and makes no folder (ruling 18, L1). | runtime `the_engine_makes_a_missing_store_owner_only_or_refuses` | Run 6: `adopter/secrets.env` (folder present) was made empty at 0600. An existing 0640 store kept its bytes and its mode. `missing/secrets.env` gave `Unstored(.., NotFound)` and `missing` was not made. `file/secrets.env` gave `NotADirectory`, and a 300-byte name gave `InvalidFilename`. Exact text for `NotFound` ("entity not found"). |
| A boxed spawn makes its store before launch, or refuses where its folder is missing. | `a_boxed_spawn_makes_its_store_before_launch_or_refuses` | Run 6: the default store was made empty at 0600 by the first boxed compose. A second engine configured at `.forge/missing/secrets.env` refused `Unstored(<kept place>, NotFound)`, and `missing` was not made. |
| Every filesystem step the box emits is named, in order, with its exact flag, and the description is exactly the steps it holds (L1, L2). | `every_mount_the_box_makes_is_the_store_s_description_in_order` | Run 6, from `box_argv` with a linked worktree, a bundle root, the artifact root at `work/s/run/artifacts`, a store at `cache/deep/secrets.env`, and `tools` (ro, masking `masked`, which exists), `cache` (rw) and `layer` (overlay) binds. The 45 emitted steps equal the test's list, `--proc /proc` through the artifact root's `--ro-bind`, and the description equals its marked subsequence. |
| A link that comes to lie on the sealed path refuses wherever it lies, and a store renamed over is still hidden in the next box (H1, rulings 15 and 5). | `a_link_on_the_sealed_store_path_refuses_wherever_it_lies` | Run 4. Store sealed at `work/conf/secrets.env`, then `conf` moved and linked back on the host: both verbs gave `Routed(<store>, <work>/conf)`, no marker. The final entry linked: `Routed(<store>, <store>)`. Restored, then `.secrets.env.new` renamed over the store: both verbs denied it. A store outside every mount whose folder `outside/sys` became a link: `Routed(<store>, <root>/outside/sys)`. A store in `work/shared/deep`, shown twice, through the worktree and through a read-write bind declared at `tools -> work/shared`: denied at both places; once `shared/deep` became a link, `Routed(<store>, <work>/shared/deep)` from both verbs and the store unchanged. |
| A store sealed as spelled because no walk resolves it refuses, and a store of two links, a directory, a path through a file each refuse. | `a_store_of_two_links_or_no_regular_file_refuses_before_the_command_runs` | Since run 6 the fixture plants the spelled reach itself (`store_place(..).unwrap_or(spelling)` in `Stored::reseal`), since no engine seals one, and the outcomes below are unchanged. Run 4: `Linked` for a second hard link and a directory; `Unobserved(NotADirectory)` through a file; `work/file/x/../secrets.env`, sealed as spelled, gave `Unresolved` both while `file` was a file and after `file/x/` and `file/secrets.env` were made; `outside/a -> b`, `outside/b -> a` sealed through `outside/a` gave `Routed(<store>, <root>/outside/a)`. |
| A sealed path that is not valid UTF-8 refuses whatever the box mounts, and so does a place in the box spelled through a workdir that is not (L1, ruling 12). | `a_store_whose_real_place_is_not_utf8_refuses_and_the_host_is_unchanged` | Run 4: a link outside to `work/.forge/secret-\xFF.env` resolves at sealing, and both verbs gave `Unspelled(<that place>)` with `.forge`'s entries unchanged; `outside/secret-\xFF.env`, outside every mount, gave `Unspelled` from both verbs; with the workdir spelled `wt-\xFF -> work`, a store in `.forge` gave `Unspelled(<wt>/.forge)` (a pin) and one at the worktree's root `Unspelled(<wt>/secrets.env)` (a hide). |
| The worktree's folders above the store and above the artifact root are pinned together, outermost first, each once, shown by the box's own mount tree. | `the_store_s_and_the_artifact_root_s_folders_are_pinned_together` | Run 4. Artifacts at `work/s/run/artifacts`, store at `work/s/keys/secrets.env`. `mv s/run`, `mv s/keys` and `mv s` each gave the exact EBUSY line and the store was denied. Inside the box, `/proc/self/mountinfo` shows `<work>/s` mounted once, and the mounts at `s/keys` and `s/run` both have that mount as their parent (INFO: behavioural, not an argv count). |
| A generated pin is judged as any mount is (L2). | `a_generated_pin_that_would_shadow_a_hide_refuses` | Run 4: `placed` over a description holding the worktree (`/h` at `/w`) and its own pin (`/h/s` at `/w/s`) gives `Ok(["/w/s/secrets.env"])`; with a pin showing `/elsewhere/s` at `/w/s` it gives `Err(Shadowed("/h/s/secrets.env", "/w/s/secrets.env"))`. No path through `box_argv` generates such a pin, since every pin's source is its parent mount's own folder, so this proof is a unit test of the judgment. |
| A mount that would shadow a planned hide refuses. | `a_mount_that_would_shadow_a_hide_refuses_and_the_host_is_unchanged` | Run 4 (passing) and run 3 (details): a store in the bundle root and a read-write bind declared at `/runtime/bundle` gave `Err(Shadowed(<store>, /runtime/bundle/secrets.env))`, no marker, the bundle root unchanged. |
| A store's folders cannot be moved, and the next box still hides it. | `a_call_cannot_move_the_store_s_folders_and_the_next_box_still_hides_it` | Run 4: `mv conf conf-moved` and `mv conf/deep conf/moved` gave the exact EBUSY, the next box denied the store, and a read-write bind's pin followed its `--bind-try` and gave EBUSY too. |
| A store a read-only bind holds is hidden after the bind. A store outside every mount is not mounted. | `a_store_a_bind_holds_is_hidden_and_one_outside_every_mount_is_not_mounted` | Run 4: exact EACCES for both verbs. The outside case's argv carries no `/dev/null` and no `outside`, and `test -e` in the box finds nothing. |
| The bundle root is accounted for. | `a_store_a_bundle_root_holds_is_denied_wherever_the_box_shows_it` | Run 4: exact EACCES at `/runtime/bundle/secrets.env` for an outside bundle root, and at both places for one inside the worktree. |
| The host toolchain is accounted for. | `a_store_the_host_toolchain_would_show_is_accounted_for` | Run 4: a store sealed at `/usr/local/brokkr-u8h-absent/secrets.env` gives `Err(Absent(..))`, not an argv. |
| A missing store under a folder that stands is made empty at 0600 by the engine and hidden. | `a_store_the_engine_made_is_hidden_in_the_box_it_composes` | Run 4 (passing) and run 2 (details): the composed spawn's `--reach` names the store; the real exec box from that reach returned `Ok(1)` with exact EACCES. Run 6: passes, its folder `.forge` made by the artifact root's making first. |
| The box makes no store. | `a_store_absent_in_reach_refuses_and_the_box_makes_none` | Run 4: `Absent` for an absent store in reach, two directories deep too, and nothing created. An absent store outside every mount runs. |
| A store open to its group or others refuses and is never changed. | `a_store_open_to_its_group_or_others_refuses_and_is_never_changed` | Run 4: 0640, 0644 and 0666 each gave `Shared(<store>, mode)`, the mode stayed as set, and back at 0600 the store was denied. |
| A store a bind mount shows at another place in reach refuses. | `a_store_a_mount_shows_in_reach_refuses_before_the_command_runs` | Run 4: with a planted table aliasing the store's directory into the worktree or a bind, both verbs gave `Shown`. An alias outside reach runs. A table naming no mount for it gives `Unmapped`. |
| A directory mounted back onto itself is no alias. | `a_directory_mounted_back_onto_itself_shows_the_store_at_no_second_place` | Run 4: denied as usual. |
| The store's mount is the table's own, on a local filesystem. | `the_store_s_mount_is_the_table_s_own_on_a_local_filesystem` | Run 4: device `4095:4095` in the table, denied; type `fuse`, `Unmapped`. |
| A store an overlay shows refuses. | `a_store_an_overlay_shows_refuses_before_the_command_runs` | Run 4: `Overlaid`, both verbs, exact text. |
| A store whose pins would uncover the Git directory's hooks refuses. | `a_store_whose_folder_pins_would_uncover_the_git_directory_refuses` | Run 4: `Gitted` for a store in `<work>/.git`, for one under a `gitdir:` pointer whose pin holds the common directory, and for one in a linked worktree's outside common directory. |
| A store the artifact root holds refuses with a cause that names the root (ruling 9). | `a_store_the_artifact_root_holds_refuses_before_the_command_runs` | Run 4: `Rooted(<store>, <root>)`, both verbs, no marker, host file unchanged. |
| A store replaced between the box's two observations refuses. | `a_store_replaced_between_the_box_observations_refuses` | Run 4: `Changed`, no marker; the next box denied it. |
| A reservation sealed without the store refuses with MB3's cause. | `a_reservation_sealed_without_the_store_no_longer_reads` (runtime), `a_reach_sealed_without_the_store_refuses_both_verbs_before_any_box` (CLI) | Run 4 (passing) and run 2 (details): `Err(Unadmitted::Identity(Unknown::Participant(<reservation>)))`, and each verb returned "--reach: … missing field \`store\` …" with no session made. Run 5: the runtime test also writes `store` as a bare path, as run 4 sealed it, and gets the same `Participant` refusal; the store it reads first is `{"place": secrets_store_path(), "spelling": secrets_store_spelling()}`. |
| A store renamed over during one running box does what ruling 3 records. | `a_store_renamed_over_during_one_box_shows_that_box_the_new_store` | Run 4: the running call read nothing before the rename and `DOCS_TOKEN=renamed` after it; the next call got exact EACCES. |

## Removals

Each removal compiled, failed the named test on the quoted assertion, and
was restored by the reverse edit. After the run 4 removals, `hidden.rs`,
`sealed.rs` and `engine/boxed.rs` were compared byte for byte with copies
taken before them (`cmp` reported them the same), and the store tests then
passed 21 of 21. Protocol removals ran with `cargo test -p brokkr-protocol
--lib --locked hands::tests::store::<test>`, selecting the named test alone
unless the row says otherwise; the runtime one ran with `cargo test -p
brokkr-runtime --lib --locked store_tests`.

Run 5 took byte copies of `hidden.rs` and `engine/boxed.rs` before its
removals and restored each from its copy; `cmp` reported them the same.
The protocol store tests then passed 23 of 23, and the runtime's
`store_tests`, `secret_threading_tests` and `admission::tests::reach` passed
15 of 15. Run 5 ran its protocol removals over the whole store suite
(`cargo test -p brokkr-protocol --lib --locked hands::tests::store`) and
names every test that failed. Its runtime removals ran with `cargo test -p
brokkr-runtime --lib --locked` filtered to `store_tests`, and N added
`secret_threading` and `reach::`.

Run 6 copied `hidden.rs`, `hidden/place.rs`, `hidden/table.rs`, `sealed.rs`
and `engine/boxed.rs` before its removals. It ran each removal alone, never
two in one build. It restored each from its copy, and `cmp` then reported
all five the same as their copies. Protocol removals ran the whole store
suite (`cargo test -p brokkr-protocol --lib --locked hands::tests::store`)
and runtime removals ran `cargo test -p brokkr-runtime --lib --locked
store_tests`. Each row below names every test that failed. After the
restores the protocol store suite passed 23 of 23 and the runtime
`store_tests` 5 of 5. The L2 control M2 was run again after the parity test
was split under clippy's line ceiling, with the same result.

| Removal | Change | Failed test and assertion |
| --- | --- | --- |
| Q2, the kept place (run 6, H1) | `kept_store` returns a fresh `Box::leak(Box::new(KeptStore::resolved(..)))` on every call, never the `OnceLock` | `the_run_keeps_one_store_place_…`: left `(String("…/moved/secrets.env"), "…/moved/secrets.env")`, right `(String("…/real/secrets.env"), "…/real/secrets.env")`. Also `a_store_whose_folder_cannot_be_resolved_…`: left `None`, right the `Unresolved` text, at the unboxed compose, since nothing was kept. 3 of 5 passed. |
| V2, the engine's spelling check (run 6, H1) | `Ok(_) => Err(Strayed(..)).or(Ok(place))` in `Engine::led` | only `the_run_keeps_one_store_place_…`: left `None`, right `Some("dispatch refused: … the configured store '…/route/secrets.env' no longer leads to '…/real/secrets.env', where the box's reach sealed it")` |
| W2, the unboxed check (run 6, H1) | `.filter(\|_\| false)` on the kept place in `prepared`'s unboxed branch | `a_store_whose_folder_cannot_be_resolved_…`: left `None`, right the `Unresolved` text; `the_run_keeps_one_store_place_…`: left `None`, right the `Strayed` text, each at its unboxed compose |
| S2, admission seals the kept place (run 6, H1) | `KeptStore::sealed` returns `(spelling, spelling)` | only `the_run_keeps_one_store_place_…`, "admission sealed the kept place": left `Some("…/route/secrets.env")`, right `Some("…/real/secrets.env")` |
| F, no fallback (run 6, L1) | `store_place` returns `walked(..).or(Ok(absolute))` | `a_store_whose_folder_cannot_be_resolved_…`: left `Ok("…/file/secrets.env")`, right `Err(NotADirectory)`; `the_run_keeps_one_store_place_…`: left the `Strayed` text, right the `Unresolved` text |
| P, no folder made (run 6, L1) | `make_store` makes the store's folder recursively at 0700 first, as run 5 did | `the_engine_makes_a_missing_store_…`: left `Ok(())`, right `Err(Unstored("…/missing/secrets.env", NotFound))`; `a_boxed_spawn_makes_its_store_…`: left `None`, right `Some("dispatch refused: … the store '…/work/.forge/missing/secrets.env' cannot be made: entity not found")` |
| U, the box's unresolvable spelling (run 6, L1) | `.or(Ok(()))` on `Store::led`'s error arm | only `a_configured_spelling_that_no_longer_leads_…`: left `Ok(0)`, right `Err("hands namespace: … '…/outside/route/secrets.env' cannot be observed: not a directory")` |
| M2, a step the old parity test left out (run 6, L2) | `false &` in the mask filter of `mount_binds`, so the bind's `/dev/null` mask is not emitted | only `every_mount_the_box_makes_is_…` (22 of 23 passed), at `assert_eq!(emitted(&argv), steps)`: left lacks `["--ro-bind", "/dev/null", "…/tools/masked"]`, right holds it. Run 5's version filtered out every `/dev/null` source, so it could not see this. |
| H, the hide (run 6) | `#[cfg(any())]` on `hide`'s mount loop | 13 tests failed, 10 passed. `a_store_a_bind_holds_is_hidden_…`, "the call": left `("DOCS_TOKEN=fake-u8h-value\n", 0)`, right `("", 1)`. Six more panics read in the output failed at the same assertion, each a real box's `cat` reading the fake value. The output was cut after them, so the assertions of the last four failures were not read. `a_store_a_bundle_root_holds_…`, "the exec": left `Ok(0)`, right `Ok(1)`. The parity test failed: left lacks the hide step. |
| L, the link-count check (run 6) | `single(..) \| true` in `table.rs` | only `a_store_of_two_links_…`: left `Ok(0)`, right `Err("hands namespace: … '…/outside/secrets.env' is not one regular file of one link its user owns")` |
| A, the alias check (run 6) | `false &` before the alias predicate in `table.rs` | only `a_store_a_mount_shows_in_reach_…`: left `Ok(0)`, right `Err("hands namespace: … a mount shows '…/outside/secrets.env' at '…/work/alias/secrets.env', in the box's reach")` |
| O2, the second observation (run 6) | `return Ok(());` after `sealed.confirm` in `Hidden::confirm` | only `a_store_replaced_between_…`: left `Ok(0)`, right `Err("hands namespace: … '…/work/.forge/secrets.env' changed between the box's two observations")` |
| E, the engine's store making (run 6) | `return Ok(());` first in `make_store` | four of five `store_tests` (all but the unresolved-folder test), each at "the engine made the store": left `(None, None)`, right `(Some([]), Some(384))` |
| V, the spelling check (run 5, ruling 17) | `sealed.led()?` in `present` replaced by `let _ = sealed;` | only `a_configured_spelling_that_no_longer_leads_…` (22 of 23 passed): left `Ok(0)`, right `Err("hands namespace: … the configured store '…/outside/route/secrets.env' no longer leads to its sealed place '…/work/.forge/secrets.env'")`. Run again after the test's fixture moved into `linked_outside`, with the same result. |
| Q, the engine's launch check (run 5, ruling 17, on the final `prepared`) | the place filter in `prepared` made `.filter(\|_\| false)` | only `a_moved_configured_spelling_refuses_…`: left `None`, right `Some("dispatch refused: … the configured store '…/route/secrets.env' no longer leads to '…/real/secrets.env', where the box's reach sealed it")` |
| N, one authority (run 5, ruling 17; run while `prepared` still returned a `Result`, which neither failing assertion reads) | `secrets_store_path` returns `secrets_store_spelling()` unresolved | `a_moved_configured_spelling_refuses_…`: left `"…/route/secrets.env"`, right `"…/real/secrets.env"`; `secrets_file_override_wins_over_the_default`: left `String("…/route/elsewhere.env")`, right `String("…/elsewhere.env")` (13 of 15 passed) |
| D1, the description's order (run 5, L1) | the bundle root described before the worktree in `mounted` | only `every_mount_the_box_makes_is_the_store_s_description_in_order`, at its `assert_eq!(emitted, planned)` |
| D2, the description's common directory (run 5, L1) | the common directory left out of `mounted` | only the same test, at the same assertion |
| H, the hide (run 5) | `#[cfg(any())]` on `hide`'s mount loop | `a_store_a_bind_holds_is_hidden_…` alone: "the call", left `("DOCS_TOKEN=fake-u8h-value\n", 0)`, right `("", 1)` |
| L, A, O2 (run 5, built together) | `single(..) \| true`; `false &` before the alias predicate; `return Ok(())` after `sealed.confirm` in `Hidden::confirm` | exactly three tests failed, 20 passed. `a_store_of_two_links_…`: left `Ok(0)`, right `Err("… '…/outside/secrets.env' is not one regular file of one link its user owns")`. `a_store_a_mount_shows_in_reach_…`: left `Ok(0)`, right `Err("… a mount shows '…/outside/secrets.env' at '…/work/alias/secrets.env', in the box's reach")`. `a_store_replaced_between_…`: left `Ok(0)`, right `Err("… '…/work/.forge/secrets.env' changed between the box's two observations")` |
| E, the engine's store making (run 5) | `return Ok(())` first in `make_store` | all four `store_tests`, each at "the engine made the store": left `(None, None)`, right `(Some([]), Some(384))` |
| C, canonical sealing (run 4) | `resolved` (now `store_place`) returns the absolute spelling, discarding the walk | `a_store_in_the_worktree_…`: "the reach seals the canonical path", left `"…/outside/link/secrets.env"`, right `"…/work/.forge/secrets.env"` |
| R15, the link on the sealed path (run 4) | `false &` before `found.is_symlink()` in `unlinked` | `a_link_on_the_sealed_store_path_…`: left `Ok(1)`, right `Err("… the symbolic link '…/work/conf' lies on the sealed path '…/work/conf/secrets.env', so it no longer resolves to itself")` |
| P, the plain path (run 4) | `true \|` before the plain-path predicate | `a_store_of_two_links_…`: the `..` case, left `Err("… '…/work/file/x/../secrets.env' cannot be observed: not a directory")`, right the `Unresolved` text. With that first case set aside for one build, the second case (once `file/x/` and `file/secrets.env` exist) gave left `Ok(0)`: the command ran. |
| U1, the sealed path's UTF-8 (run 4, L1) | `.or(Some(()))` after `to_str().map(drop)` in `present` | `a_store_whose_real_place_is_not_utf8_…`: the store outside every mount, left `Ok(0)`, right `Err("… '…/outside/secret-�.env' is not valid UTF-8, so no mount can be spelled at it")` |
| U2, a pin's UTF-8 (run 4) | `false &` before the pin check in `pinned` | the same test, the lossy workdir case: left `Err(Unspelled("…/wt-\xFF/.forge/secrets.env"))`, right `Err(Unspelled("…/wt-\xFF/.forge"))`. The hide-target check behind it still refused; this binds by cause. |
| U3, a hide's UTF-8 (run 4) | `false &` before the target check in `placed` | the same test, `work/secrets.env` with the lossy workdir: left `Ok([argv])` whose argv held `--ro-bind /dev/null …/wt-�/secrets.env`, right `Err(Unspelled("…/wt-\xFF/secrets.env"))` |
| G, generated pins judged (run 4, L2) | the shadow scan filters out `Held::Pinned` entries | `a_generated_pin_that_would_shadow_a_hide_refuses`: left `Ok(["/w/s/secrets.env"])`, right `Err(Shadowed("/h/s/secrets.env", "/w/s/secrets.env"))` |
| J, the chains together (run 4, behavioural) | `#[cfg(any())]` on the sort and the dedup in `assembled` | `the_store_s_and_the_artifact_root_s_…`: "s/run is pinned on the topmost shared pin", left `Some("7221")`, right `Some("7223")`: the box's mount at `s/run` sat on the first `s` pin, under a second `s` pin that shadowed it |
| K, the walk's bound (run 4) | `false &` before `*links == LINKS` | `a_store_of_two_links_…`: sealing the loop recursed without end; the test thread overflowed its stack and the binary aborted |
| H, the hide (run 4, M1) | `#[cfg(any())]` on `hide`'s mount loop | `a_store_a_bind_holds_is_hidden_…` alone: "the call", left `("DOCS_TOKEN=fake-u8h-value\n", 0)`, right `("", 1)`. `a_store_renamed_over_during_one_box_…` alone: left `"DOCS_TOKEN=fake-u8h-value\nfirst\nDOCS_TOKEN=renamed\nsecond\n"`, right `"first\nDOCS_TOKEN=renamed\nsecond\n"`. `a_link_on_the_sealed_store_path_…` alone, at the renamed-over case: "the call", the same left and right as the first. |
| O2, the second observation (run 4) | `return Ok(())` after `sealed.confirm` in `Hidden::confirm` | `a_store_replaced_between_…`: left `Ok(0)`, right `Err("… '…/work/.forge/secrets.env' changed between the box's two observations")` |
| P1, the worktree's pins (run 4) | `#[cfg(any())]` on `pin_worktree`'s call | `a_call_cannot_move_…`: "conf", left `("", 0)`, right `("mv: cannot move 'conf' to 'conf-moved': Device or resource busy\n", 1)` |
| P2, a bind's pins (run 4) | `#[cfg(any())]` on `pin`'s call | `a_call_cannot_move_…`: "…/tools/conf", left `("", 0)`, right the same EBUSY line for `…/tools/conf` |
| L, the link-count check (run 4) | `single(..) \| true` | `a_store_of_two_links_…`: left `Ok(0)`, right `Err("… '…/outside/secrets.env' is not one regular file of one link its user owns")`. Built together with A; each test failed on its own check. |
| A, the alias check (run 4) | `false &` before the alias predicate | `a_store_a_mount_shows_in_reach_…`: left `Ok(0)`, right `Err("… a mount shows '…/outside/secrets.env' at '…/work/alias/secrets.env', in the box's reach")` |
| Y, the shadow check (run 4) | `false &` before the last-mount comparison | `a_mount_that_would_shadow_a_hide_…`: left `Ok(0)`, right the `Shadowed` text; `a_generated_pin_…` failed too, left `Ok(["/w/s/secrets.env"])` |
| R, the artifact-root refusal (run 4) | `false &` before the held-root comparison | `a_store_the_artifact_root_holds_…`: left `Ok(0)`, right the `Rooted` text. Built together with B and T; each test failed on its own check. |
| B, the bundle root (run 4) | `bundle.filter(\|_\| false)` in `mounted` | `a_store_a_bundle_root_holds_…`: "the exec", left `Ok(0)`, right `Ok(1)` (the shadow test failed under it too, as its store lies in the bundle root) |
| T, the toolchain (run 4) | `.take(0)` on `HOST_TOOLCHAIN_BINDS` in `mounted` | `a_store_the_host_toolchain_…`: left `Ok([bwrap argv])`, right `Err(Absent("/usr/local/brokkr-u8h-absent/secrets.env"))` |
| E, the engine's store making (run 4) | `make_store` returns `Ok(())` first | `the_engine_makes_a_missing_store_…`, `a_boxed_spawn_makes_its_store_…` and `a_store_the_engine_made_…`, each at "the engine made the store": left `(None, None)`, right `(Some([]), Some(384))` |
| M, the mode check (run 2) | `mode & 0o077 == 0 \|\| true` | `a_store_open_to_its_group_…`: left `Ok(0)`, right `Err("… grants its group or others access (mode 0640)")` |
| S1, the same place (run 2) | `alias != place \|\| true` | `a_directory_mounted_back_onto_itself_…`: `Err("… a mount shows '…/work/.forge/secrets.env' at '…/work/.forge/secrets.env', in the box's reach")` |
| D, the table's device (run 2) | the store's `stat` device handed to `within` | `the_store_s_mount_is_the_table_s_own_…`: `Err("… the mount table does not prove where …")` |
| F, the local filesystem (run 2) | `record.local \| true` in store.rs's `within` (temporary) | `the_store_s_mount_is_the_table_s_own_…`: left `Ok(0)`, right `Err("… the mount table does not prove where …")` |
| O, the overlay refusal (run 2) | an overlay bind mapped to `Held::ReadOnly` | `a_store_an_overlay_shows_…`: left `Ok(0)`, right `Err("… an overlay shows …")` |
| G1, a store in the Git directory (run 2) | `place.starts_with(common) & false` | `a_store_whose_folder_pins_…`, the linked-worktree case: left `Ok(0)`, right `Err("… pinning the folders of …")` |
| G2, a pin holding it (run 2) | `holds & false` | the same test, the `gitdir:` case: left `Ok(0)`, right `Err("… pinning the folders of …")` |
| X, the absent refusal (run 2) | `!held \| true` | `a_store_absent_in_reach_…`: left `Ok(0)`, right `Err("… is absent where the box would show it")` |
| S, the store must be written (run 2) | `#[serde(deserialize_with = "Option::deserialize")]` replaced by `#[serde(default)]` | runtime `a_reservation_sealed_without_the_store_…`: left `Err(Identity(Unproven))`, right `Err(Identity(Participant(…)))`; CLI `a_reach_sealed_without_the_store_…`: `no session is made` panicked |

The run 2 rows were not repeated in run 4. The code each removes moved
between functions in run 4 (the Gitted check into `pinned`, the absent
check into `present`) but its predicate did not change, and each named test
passes on this tree. Run 3's removal W (the writable-link predicate) no
longer applies: run 4 removed that code (ruling 15). Run 5's rows Q and N
name `a_moved_configured_spelling_refuses_the_engine_s_next_launch`, which
run 6 replaced. Run 6's Q2, V2 and S2 bind what those rows bound, against
the kept place. Run 6 moved the code of run 4's removals L, A and K into
`hidden/table.rs` and `hidden/place.rs` unchanged, and L and A were run
again there (above).

## Gates on this tree (run 6)

After the last code edit, the format check, workspace clippy (all targets,
all features, `--locked`, `-D warnings`), `typos --hidden`, `git diff
--check` on the staged and unstaged trees, and `openspec validate --all
--strict` (36 passed) reported no finding. Clippy first refused the parity
test at 106 lines, over the 100-line ceiling, and run 6 moved its step list
into the helper `planned`. `recipes/self` and `recipes/verify` compiled,
their policy sweeps reporting 48 valuations over 11 groups and 16 over 5,
each with 0 unruled.

The suites ran crate by crate under the seat's 10-minute bound, with no
failures. brokkr-protocol ran 1069 lib tests (3 ignored), `hands_exits`
(6), `secret_drop` (1) and its doctests (1). brokkr-runtime ran 1386 lib
tests, `capability_launch` (76), `it` (142, `witness_digests` included, so
no pin moved), `operated_repo` (1) and `queued_launch` (8). brokkr-cli ran
1054 lib tests (1 ignored), `it` (801, 3 ignored), `driver_conformance`
(27), `transcript_surfaces` (14), `heap_claude` (1), `heap_codex` (1),
`heap_dsh` (1) and `heap_fleet` (3). brokkr-seatbelt-probe ran 99 (2
ignored). The last production edit moved the three initialisers within
`engine.rs`'s struct literals. The brokkr-runtime suites ran again after it,
with the same counts. The protocol, brokkr-cli and seatbelt suites ran
before it. The protocol crate does not build `engine.rs`. Workspace clippy
and the format check ran after it. The other CLI
integration binaries (`acknowledge` through `writer_lease`) were not run in
run 6. No known load flake fired, and no git step failed for an author
identity.

`bash quality/ratchet.sh files`, `clones` and `api` printed "file size
holds", "duplication holds" and "public API holds". `clones` first reported
the moved `engine.rs` clone described under "What changed"; with the
initialisers above `world` it held.

## Measured baselines

The protocol API rose by three lines against main, which is ruling 4's +3
cap (`git diff 9545c18c --numstat` on the snapshot: 4 added, 1 removed):

- `pub const brokkr_protocol::hands::UNHIDDEN_STORE: &str` (run 1);
- `pub fn brokkr_protocol::hands::sealed_store(&brokkr_protocol::hands::Sealed) -> core::option::Option<&std::path::Path>` (run 5): the sealed place the engine's launch check compares;
- `pub fn brokkr_protocol::hands::store_place(&std::path::Path) -> core::io::error::Result<std::path::PathBuf>` (run 5, its return type changed in run 6): the one resolution, which the engine runs once per run and the box runs to check the sealed spelling.

`brokkr_protocol::hands::seal` changed its one line. Its fifth argument is
`core::option::Option<(&std::path::Path, &std::path::Path)>` since run 6.
The snapshot's two changed lines were edited by hand to the ratchet's
measured text, since the seat cannot run `bash -c`, and `api` then held. No
other crate's API moved. The kept place is private engine state and adds no
public line.

The `quality/file-lines.txt` rows below are `wc -l` counts set by hand,
because the seat cannot run `quality/measure.sh`:

| File | Main (9545c18c) | This tree |
| --- | --- | --- |
| `hands/sealed.rs` | 230 | 291 |
| `hands/sealed/hidden.rs` (new, production) | — | 498 |
| `hands/sealed/hidden/place.rs` (new, production, run 6) | — | 124 |
| `hands/sealed/hidden/table.rs` (new, production, run 6) | — | 83 |
| `engine/boxed.rs` | 249 | 397 |
| `engine/broker/admission.rs` | 796 | 798 |
| `engine.rs` | 3316 | 3314 |
| `hands.rs`, `doctor.rs`, `probe/observe.rs` | 1094, 1064, 522 | unchanged |
| `hands/tests.rs` | 1844 | 1846 |
| `hands/tests/store.rs` (new) | — | 1130 |
| `boundary_tests/sealed_tests.rs` | 104 | 132 |
| `sealed_tests/store_tests.rs` (new) | — | 301 |
| `boundary_tests/evidence_tests.rs` | 273 | 258 |
| `admission/tests.rs` | 1820 | 1837 |
| `admission/tests/reach.rs` | 163 | 203 |
| `engine/secret_threading_tests.rs` | 299 | 303 |
| `carry/fixture_tests.rs` (run 6) | 153 | 162 |
| `carry/drive_tests.rs`, `operator/retry/tests.rs` (run 6) | 525, 290 | unchanged |
| `cli/src/hands/tests.rs` | 147 | 183 |
| `cli/src/init/tests.rs` | 712 | 716 |
| `adapters/mcp/tests.rs` | 1066 | 1067 |
| `protocol/tests/hands_exits.rs` | 178 | 179 |
| `brokered_tests.rs`, `writers_tests.rs` | 879, 1531 | 885, 1532 |

Test files already over 2,000 lines kept their size: `boundary_tests.rs`,
`adapters/tests.rs`, `engine/tests.rs`, `capability_launch.rs` and the
seatbelt probe's tests. There is one test move, from run 1:
`evidence_tests.rs`'s inline bwrap probe moved into
`sealed_tests::namespace_bwrap`, which the runtime namespace test shares.
Run 6 moved no test, and replaced one runtime test (above). No suppression
was added. Witness and compose pins did not move. `hands.rs` changed lines
in place only: two names added to its existing `sealed` re-export.

`secret_threading_tests.rs` is outside the owning suites. Run 5 changed it
because it pins the seat's `secrets_file`, which is the canonical place. Its
three tests compare against the canonical temporary root, which matters
where the temporary root is a link (macOS's `/var`). Its override test
configures the store through a link. It passes unchanged in run 6.

## Limits and scope

What the run refuses, and what it records. Inside one engine's run, the
kept place is the one authority. The configured spelling is checked to lead
to it before every boxed spawn, before every unboxed spawn once a seat's
input names it, and again in each box's two observations. Where it does not,
the spawn or box refuses (`Strayed`, `Unresolved`, `Unobserved`), and
nothing is made. That is H1's in-run refusal. The four limits below are
apart from it. Each is a window or a case these checks do not cover.

1. Ruling 3, measured. A store renamed over by `brokkr secrets set` or `rm`
   during one running box is visible to that one call or exec command, and
   every later box hides it.
2. Ruling 14, recorded. The limit also covers the sibling `.secrets.env.*`
   files `brokkr secrets set` stages beside the store during an update (a
   `tempfile` with the prefix `.secrets.env.` in the store's folder,
   `crates/brokkr-protocol/src/secret.rs` at lines 320 to 322, not this
   row's file). A call running during that update may read their contents,
   since the box hides only the store's own name. No test in this row
   exercises those siblings.
3. Ruling 19 (a), recorded. A symbolic link on the configured spelling that
   lies in a writable mount but off the canonical place is not pinned. The
   box pins only folders on the canonical place. A call in a box can
   retarget such a link. The next spawn or box then sees the spelling lead
   elsewhere and refuses (`Strayed` or `Unresolved`), as H1's checks do, and
   between those checks the engine and `brokkr secrets` follow the link. No
   test in this row retargets a link from inside a box; the runtime and
   protocol tests retarget it on the host.
4. Ruling 19 (b), recorded. A later, separate run resolves the configured
   spelling afresh in its own engine. A store moved between runs is adopted
   at its new place by that run, and sealed and hidden there. The carry and
   retry tests above model a later command's process this way
   (`relaunched_with`).

Also: the check before a spawn and the driver's own read are two moments.
A route moved between them is not seen until the next check. An engine that
resumes a run in a new process resolves afresh. Its launch then refuses
`Strayed` where the reservation it reads sealed another place.

Ruling 15 refuses every boxed seat once a symbolic link stands anywhere on
the sealed path, including one the operator adds above the store after
admission (a dotfiles manager relinking `.forge`, for instance). Such a seat
refuses `Routed` until a new run seals the store again. Links that exist at
sealing keep working, as the worktree test shows for `outside/link`.

Ruling 18 makes a missing folder a refusal: `--secrets` naming a folder that
does not stand refuses every boxed spawn with `Unstored(.., NotFound)`, where
run 5 made the folder. The default store's folder is the run-local
directory, which the artifact root's making creates first, so a default
store is unaffected. A store whose way cannot be walked refuses every spawn
that would use it, unboxed included, with `Unresolved`.

The store checks in `judged` do not depend on reach. They run for every
sealed store that is present, one outside every mount included: one
regular file of one link the user owns (`Linked`), no group or other bit
(`Shared`), and a mount the table proves to be local (`within`, else
`Unmapped`). Only the alias check (`Shown`) reads the box's mounts. So a
store on overlayfs, FUSE or a network mount refuses every boxed seat whose
reach seals it, wherever it lies, as `Unmapped`, and so does a store that is
not owner-only or is hard-linked.

The INFO of run 3's review asked for a toolchain test that hides an
existing store under a toolchain root. No test can write under `/usr`, and
an existing file there is root's, which `Linked` refuses before any hide,
so the toolchain stays proved through the absent rule and removal T.

I1 (run 5's review, info). `engine/broker.rs` normalises the store a second
time for MB4's excluded store (`stored`, near line 571), with no comparison
against the kept place. It is outside this row and behind the closed MCP
compile fence, and run 6 did not change it. Its input is now
`secrets_store_path()`, the kept place.

Out of scope, as the row says: dsh's own native tools reading the store
(U1d3 records it), Codex's native tools (U9c2's profile), unboxed Claude,
which makes no read-isolation claim, the MCP server box (MB4's
`StoreInBox`), and, under ruling 16, the unconfirmed report that a nested
user namespace or an overlay inside the box could see beneath a mask, which
concerns `namespace.rs` and which the controller measures after U8h lands.

The MCP compile fence is unchanged. Under review R1, this file records
findings and observations only.

## Pending

These have not been run: exact coverage (`scripts/coverage-exact.sh` on a
capable host outside the workspace box), the CRAP ratchet that reads its
LCOV, the macOS build and tests (this host has no macOS target, so the
`#[cfg(not(target_os = "linux"))]` stand-in, now in `hidden/table.rs`, is
uncompiled here, and the canonical-root comparisons in
`secret_threading_tests.rs` and run 6's `store_tests.rs` are unproved where
the temporary root is a link), the brokkr-cli integration binaries other
than the seven named under the gates, a whole `cargo test --workspace` in
one invocation (the crates ran one by one), and remote CI on the PR head.
Run 6's `shut` case expects `PermissionDenied` from a folder at mode 0000,
which holds for a user that is not root; a root CI runner would see it pass
the walk and fail the test, so its result on the hosted runners is pending
too.
