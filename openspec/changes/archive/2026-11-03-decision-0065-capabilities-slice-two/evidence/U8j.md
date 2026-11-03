# U8j — Forbid nested user namespaces in every box

Tasks 45.13 and 45.14; requirements SI1 and MB4, with SD2 and SD4 for the
verification duty. Branch `s2/U8j`, cut from main at 5d706bab, recipe
`claude-flash-dsh-adv`, run `0065-slice-two-unit-u8j-see-the--7cc8ab3a`.
Everything below was observed in this run's implement seat, on this tree, on
Linux 6.17 with bubblewrap from `/usr/bin/bwrap`, a host whose AppArmor
restricts unprivileged user namespaces. What could not be run is listed as
pending at the end.

## What changed

`Namespace::open` in `crates/brokkr-protocol/src/hands/namespace.rs` builds
the one bubblewrap argv every box uses: the workspace box (`hands.rs`'s
`box_argv`, which a workspace call and an exec share) and the server box
(`Namespace::server`). Its isolation flags were an inline array. They are now
one table, `ISOLATION`, in a new private child
`crates/brokkr-protocol/src/hands/namespace/isolation.rs` (23 lines), declared
`mod isolation;` from `namespace.rs` and consumed by `open`. The table leads
with `--unshare-user --disable-userns --assert-userns-disabled`, followed by
the six flags it held before, in their old order: `--unshare-pid
--unshare-ipc --unshare-uts --unshare-cgroup-try --cap-drop ALL`. So both
profiles carry the three options immediately before `--unshare-pid`. A
workspace box keeps `--new-session` ahead of them, and a server box has
none. Moving the array out is what makes room: `namespace.rs` goes from 799
to 793 lines. No other production file moved, and no item is public:
`ISOLATION` is `pub(super)` and the module is private.

A bubblewrap too old to know `--disable-userns` (before 0.8) rejects the
option, so the box does not start. That is fail-closed, and no version probe
was added. The first removal below also shows `--assert-userns-disabled`
failing closed on its own: a bubblewrap that left user namespaces creatable
refuses to start the box.

## The proofs

| Test | What it pins |
| --- | --- |
| `hands::tests::store::a_workspace_box_makes_no_nested_user_namespace_and_still_writes_commits_and_hides` | A real workspace box over a committed repository whose `.forge/secrets.env` store is sealed. For each entry of the new `NESTED` table in `hands/tests.rs`, a workspace call's stderr and exit code are exactly the box's own ENOSPC text and 1: `unshare -Ur true` gives `unshare: unshare failed: No space left on device`, and `bwrap --ro-bind / / true` gives `bwrap: Creating new namespace failed: nesting depth or /proc/sys/user/max_*_namespaces exceeded (ENOSPC)`. An exec (`run_boxed_in`) of the same command returns 1, and its stderr, written to a file, is the same text. In the same box a commit then lands, and `git log` reads `boxed Host Operator`. The file it wrote is owned by the engine's uid on the host. `mv .forge moved` fails with "Device or resource busy", and the store reads as denied to a call and an exec through U8h's `denies`. |
| `hands::tests::server::a_server_box_makes_no_nested_user_namespace` | A real server box prepared through `ServerBox::prepare_with` and launched on its entry, with `sh` standing for the bootstrap. The shell runs each `NESTED` command and reports its output and `exit $?` on the ready pipe. The test reads the ready pipe and the launcher's stderr. The ready pipe must be exactly both ENOSPC texts, each followed by `exit 1`, and the stderr must be empty. |
| `hands::tests::the_workspace_and_exec_argv_keep_their_exact_order_and_identity` | The existing whole-argv pin for the workspace call and the exec, moved to carry `--unshare-user --disable-userns --assert-userns-disabled` between `--new-session` and `--unshare-pid`. |
| `hands::tests::server::the_server_box_holds_the_projected_system_set_and_its_own_private_paths` | The existing whole-argv pin for the server box (`server_argv`), moved to carry the three options between `--die-with-parent` and `--unshare-pid`. |

Both real-box proofs ran real namespaces in this seat. The run used
`--nocapture`, and neither test printed `skipped:`, which is what
`skip_boundary_proof` writes when it skips. The seat's shell would not run a
command with `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1` set (the prefix and `env`
both needed approval), so the declared-evidence run is pending below. Each
proof skips through the existing `can_create_namespace` and `boxes` or
`unservable` guards, which fail outright on a host that declares the variable.

## Removals, each compiled and restored

| Removal from `ISOLATION` | Failed, with the assertion's own words | Passed |
| --- | --- | --- |
| `--disable-userns` alone (`--assert-userns-disabled` kept) | All four tests above. The workspace proof failed at its call: left `("unshare -Ur true", ("bwrap: creation of new user namespaces was not disabled as requested\n", 1))`, right the ENOSPC text. The server proof failed with left `("", "bwrap: creation of new user namespaces was not disabled as requested\n")`: the box never started. Both argv pins failed on the missing token. | None of the four |
| `--disable-userns` and `--assert-userns-disabled` | Both real-box proofs. The workspace proof failed at its call: left `("unshare -Ur true", ("unshare: write failed /proc/self/uid_map: Operation not permitted\n", 1))`. The server proof's left was `unshare: write failed /proc/self/uid_map: Operation not permitted`, `exit 1`, `bwrap: No permissions to create new namespace, likely because the kernel does not allow non-privileged user namespaces. …`, `exit 1`. This is the host's EPERM, which the exact ENOSPC assertions tell apart. | — |
| `--assert-userns-disabled` alone | Both argv pins, on the missing token. The real-box proofs passed, since the box still disables user namespaces itself. The assertion only fails closed where disabling did not take hold, which the first removal shows. | Both real-box proofs |

After restoring the table, the four tests passed together
(`4 passed; 0 failed`).

## Gates run here

`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` both finished with no output.

The crate suites all passed:

- brokkr-protocol: lib 1079 passed and 3 ignored; integration and doc targets 6, 1 and 1 passed.
- brokkr-runtime: lib 1408; `it` 142; `capability_launch` 76; `operated_repo` 1; `queued_launch` 8.
- brokkr-cli: lib 1074 with 1 ignored; `it` 807 with 3 ignored, among them the broker's real server boxes, which now open with the three options; `driver_conformance` 27; each `heap_*` target; `transcript_surfaces` 14.
- brokkr-core, brokkr-view, brokkr-store, brokkr-bridge and brokkr-seatbelt-probe: every target passed.

Every ignored test was already ignored before this change. The bench targets
were not run.

`cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` compiled
the bundle, and its policy sweep reported 0 unruled. `openspec validate --all
--strict` reported 36 passed and 0 failed.

Typos and whitespace were checked by hand. `typos --hidden` over
`crates/brokkr-protocol/src/hands` and this change's directory found nothing.
A whole-tree run from outside the root reported hits only in files this unit
does not touch: `quality/public-api/*.txt` (serde's module name) and one TUI snapshot.
`git diff --check` and `git diff --cached --check` printed nothing.

Line counts were measured with `wc -l`, and their rows were set in
`quality/file-lines.txt`: `namespace.rs` 799 → 793, `namespace/isolation.rs`
new at 23, `hands/tests.rs` 1846 → 1863, `hands/tests/server.rs` 1085 → 1134
and `hands/tests/store.rs` 1143 → 1181. Every file stays under its ceiling.
No function crosses 100 lines and no suppression was added.
`quality/public-api/` is unchanged in `git status`. No witness, compose or
budget input names these files (`git grep` over the runtime tests, `scripts`
and `quality/*.json`), so no pin was re-measured.

## Moves and scope

No test moved between files. The new `NESTED` table sits in `hands/tests.rs`
beside `can_create_namespace`, and both proofs read it. The workspace proof
sits in `tests/store.rs` so it reuses U8h's `Stored` fixture.

Out of scope, as the commission directs:

- dsh's own sandbox profile and its argv pins (`dsh_sandbox.rs` and its suites), and Codex's and Claude's native tools. dsh's native tools are U1d3's recorded limitation.
- Unboxed seats.
- A host that allows unprivileged user namespaces. None was available to measure; what this unit adds is the box's own limit, shown here on a host that restricts them.

## Pending

- The two real-box proofs and the protocol hands suite under `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`. They ran real boxes here without the variable, as recorded above.
- `quality/ratchet.sh files` and `quality/ratchet.sh clones`. The seat's shell would not run the script or `jscpd` without approval.
- Exact coverage (`scripts/coverage-exact.sh`).
- macOS. The new proofs are Linux-only, and the server pin is already Linux-only.
- Remote CI.
