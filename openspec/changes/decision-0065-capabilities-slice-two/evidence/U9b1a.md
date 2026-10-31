# U9b1a — Lift the MCP compile fence where the namespace path is proved

Branch `s2/U9b1a`, from main at 1d8955b82, in run
0065-slice-two-unit-u9b1a-see-th-590de7a3. By the operator's ruling of
2026-10-30, this lands U9b1 run five's tree (run
0065-slice-two-unit-u9b1-see-the-d09b2390, signed partial commit 2da7a504e)
replayed onto main. The first run of this unit
(0065-slice-two-unit-u9b1a-see-th-2abb1e94, signed d838c4e36, from bc36e9302)
stopped on the chief's security hold H1. The second
(0065-slice-two-unit-u9b1a-see-th-49b8e92e, signed acc5eea4a, from 0675e03e8)
fixed H1 and stopped on the chief's H2. The third
(0065-slice-two-unit-u9b1a-see-th-d93f79f1, signed e8eefdd01, from 1d8955b82)
fixed H2 and stopped on the chief's H3. This fourth run replays that commit,
fixes H3 as the controller ruled, and runs the observations below again, on this
Linux x86_64 workstation. U9b1a closes no task: 42.2, 46.1 and 46.2 are U9b1b's,
and tasks.md now names them so.

## Re-vouch of PR #924 on 56870a69b, after the coverage refusal

Run 0065-slice-two-unit-u9b1a-see-th-cb86890c vouches the PR's net diff on main
at 56870a69b, with U9b1a's behaviour as run four judged it. On #924, CI's exact
coverage gate refused by one line, 78445 of 78446, by the controller's report:
`crates/brokkr-cli/src/doctor/capabilities/mcp.rs:312`, the `Some(_) =>
report.warn(&line, detail)` arm of `report_serving`. U9b1a did not edit that
file. Its cause is the lift. On Linux the compile now refuses a site whose
serving U1g1's strict check refuses, before doctor plans its servings, so no
surveyed bundle reaches that arm there; it remains what a seat's line reads on
macOS, where the fence stands. The controller's test
`a_serving_the_strict_check_refuses_is_warned_with_its_cause`, in
`doctor/capabilities/mcp/tests.rs`, calls `report_serving` with an admitted and
a refused serving and pins both whole lines. This seat bound it again: with the
arm mutated to `report.ok`, its `assert_eq!` on `report.lines` failed at
`tests.rs:103`, the second line reading `ok` where `warn` is pinned; with the arm
restored it passes. The file's `quality/file-lines.txt` row is 114.

`git diff --cached a866a9189`, the judged commit against this tree, shows that
test file, its row, and #922's core files and quality rows, and nothing else.
This seat's suites passed: runtime `--lib` 1445, `--test it` 144,
`capability_launch` 76 and `operated_repo` 1; CLI `--lib` 1105 with 1 ignored,
`--test it` 825 with 3 ignored, and `driver_conformance` 27. `recipes/self`
compiled, and the fmt, clippy, typos, openspec and `files`, `clones` and `api`
ratchet gates are clean. `writer_lease`'s real boxed exec ran and passed in the
`--test it` suite. `a_bind_alias_of_protected_evidence_is_refused` only skipped
here, its `unshare` printing `write failed /proc/self/uid_map: Operation not
permitted`, so CI's Linux leg is its run. The other crates' suites, macOS and
remote CI were not run in the seat; nor were exact coverage and the crap ratchet.

## The chief's H3, fixed as the controller ruled

The hold: H2's refusal ran only where admission held an opened root. Where
`Root::of` gives none (any step of the walk from HOME is not protected, a 0775
HOME under umask 002 among them), a claim protecting nothing took `unreserved()`
and `compared` never ran. A boxed writer whose bind covered the root was admitted
holding no lease and leaving no record, could repair the walk's permissions from
inside its box, and could then rewrite the serves of a later broker that opened
the root and found no record of it.

The ruling (controller, 2026-10-30, in design.md): before an attempt proceeds
holding no lease, each boxed writer is compared with the root's spelled path by
the same test, and refused with MB3's identity cause where it reaches the root or
where its reach cannot be proved. `sources::records_kept` is now that one check
on both paths. It takes the root's path and a mount-table reader instead of an
opened `Root`: `exclusion::compared` hands it the opened root's under the lock,
and `sources::records_unopened`, called by `admit_in` before `unreserved()`, hands
it `root::coordination(home)` and `sources::observed`. No lock or record is
involved on the second path, as there is no root to hold them.

Its path test is the new `exclusion::covers_spelled`, because that path need not
stand and `Seen::of` gives nothing for a path with a missing component. Each side
is read by its nearest standing ancestor (`anchored`, moved here from
`sources.rs`), so a HOME spelled through a link still leads to the root. The
writer's identity is looked for among the root's standing ancestors. The opposite
direction, a writer beneath one of those ancestors, counts only through `covers`,
which needs the root itself to stand: otherwise every sibling of the root under
its HOME would be refused. `staked` and the rest of `compared` did not move. A
root path that is not absolute, where the host names no HOME, proves no boxed
writer kept and refuses by `Unmounted`. That is a decision of this visit under
"where its reach cannot be proved", and it applies wherever `records_kept` runs.

| Test | What it pins | Mutation, and how it failed |
|---|---|---|
| `admission::tests::sources::a_boxed_writer_reaching_a_root_that_cannot_be_opened_is_refused` | Through `admit_in` with the real `Root::of` on a 0775 parent: a seat whose rw bind is the root, then HOME, journals `EffectFailed` with the exact `Serving` text, `held` none, and `.local` under that HOME not made. A box beside the root under that HOME, and an unboxed writer binding HOME (`Boundary::Open`), proceed with `held` none. | `records_unopened`'s refusal discarded in `admit_in` (`.or(Ok::<(), Unknown>(()))`): panicked at its first refusal assertion, reach `<root>`. `covers_spelled` made always true: `assertion failed: admitted_under(&mut apart, &home)`. |
| `admission::tests::sources::a_root_reaching_writer_refused_unopened_leaves_a_later_broker_alone` | The chief's transition. Unopened: the seat binding HOME is refused by the exact text and its driver never ran. Parent chmod 0700: `Root::of` opens at the same path, a later broker is admitted and extends `Ok(Synced)`, and the same reach is now refused under the lock with the lease's bytes equal. Uses `Host::resolving`, and it was not skipped here. | The same `admit_in` discard: `assertion failed: !admitted_under(&mut reaching, &home)`. |
| `admission::tests::sources::a_boxed_writers_reach_of_an_unopened_root_that_cannot_be_proved_is_refused` | Beside a root path that does not stand: `Ok(())` on a table that proves it, `Unmounted(<root>)` on one that does not parse, and an unboxed writer `Ok(())` on either. | `covers_spelled` made always true: its `Ok(())` assertion failed. |
| `admission::tests::coordination::an_unopened_roots_path_keeps_boxed_writers_on_every_host` | In a suite built off Linux too: reach at, above and below an absent root by the exact cause; link spellings; a sibling under HOME kept; and `""` and `"."` as HOME refusing by `Unmounted`, an unboxed writer `Ok(())`. | The `is_absolute` guard removed: `left: Ok(())`, `right: Err(Unmounted("./.local/state/brokkr/capabilities"))`. The `lexical` arm removed: `assertion failed: covers_spelled(&coordination(&link).join("below"), &records)`. `covers_spelled` made always true: `assertion failed: !covers_spelled(&beside, &coordination(&link))`. |

Each mutation was applied, run over the tests named and restored. Not observed in
this seat: `covers_spelled`'s identity arms need a real bind. They are asserted in
`a_bind_alias_of_protected_evidence_is_refused`'s namespace half, which skipped
here ("unshare: write failed /proc/self/uid_map: Operation not permitted", then
"skipped: … no user and mount namespace"), so those assertions have never
executed, and no mutation of those arms was run. That proof's own comment names
the controller's host run as where it executes; on Linux the mount table also
refuses such a bind, so these arms decide alone only off Linux. The
`Unmounted`-on-unparsable-table outcome through `records_unopened` itself was not
exercised, as its reader is fixed to production's; the suite drives the same
`records_kept` through the test's reader. The off-Linux `reach_records` has not
been compiled or run here, so macOS CI is pending.

Two fixtures moved with this. The lease in
`a_forged_record_has_no_boxed_writer_to_forge_it` now asserts its bytes unchanged,
as its doc says. And `machine_proof::dialect_validate_expands_the_chiefs_change_and_records_tool_evidence`
failed here deterministically, its boxed `design:validate` writer's reach holding
the fixture's own HOME: "can write '/tmp/.tmp…/home/.local/state/brokkr/capabilities'".
This seat's temporary directories left that root unopenable, so the test had been
passing through the very hole H3 names. It now runs with its HOME beside the
operated tree (`Workspace::run_homed`), and `home_in`'s comment says when a test
must do so. The refusal itself is the ruling's, so the expectation did not move.

These residuals stay, as ruled. A writer Brokkr cannot box can forge a record, but
it holds the user's write anyway; refusing it on every run is the later decision
ruling 21 names. And a writer admitted without a lease, of any reach, is invisible
to a later broker if a third party repairs the root's permissions while it lives:
a boxed writer kept apart cannot do that repair, but nothing stops another
party. Its absence from the inventory says nothing about whether it is live, as
`unreserved()`'s comment now says. This is recorded as a follow-up, not fixed here,
by the controller's ruling on H3.

## The chief's H2, fixed as the operator ruled

The hold: H1's check compares the writers a reservation names, and that record
is a plain file. A boxed writer that reached the coordination root was admitted
while nothing was served (ruling 21), so it could clear its own record's writers
before a broker extended beside it. `extended_with` would then find no such
writer, admit the broker, and leave the writer free to clear the broker's serves.
Nothing about the record says who wrote it, and this fix does not try to make it
say so.

The ruling (operator, 2026-10-30, in design.md): a boxed writer whose writable
reach covers the capability coordination root is refused at admission with MB3's
identity cause, whether or not evidence is at stake and whether or not any
reservation serves. `sources::records_kept` does that, called from
`exclusion::compared` after the lease and before its unstaked return where the
root is open, and before `unreserved()` where it is not (H3, above). It holds the
root to what `kept` does, `exclusion::covers` (at, above or below, by path or
identity) and, on Linux, the mount table, with `covers_spelled` beside them for
a path that need not stand. `Comparison.coordination` is now the opened `Root`,
for the root's mount observer; nothing else of `staked` or `compared` moved. A
boxed writer on a staked run that reaches the root now meets this cause rather
than CR2's, as the ruling words it.

Two choices follow from what the ruling reaches. Off Linux no mount table is read,
so `records_kept` decides there by path and identity rather than taking
`sources::observed`'s refusal, which would refuse every boxed writer on a run with
nothing at stake, a behaviour change macOS was not ruled to take. `exclusion`'s
comment says no such box is built there; this visit did not trace that further.
That branch has not been compiled or run here, as this host has no Darwin target
(`cargo check --target aarch64-apple-darwin` stops at the first dependency): macOS
CI is pending. And `Unknown::Serving`'s text said only
"which an MCP server's box is built from", untrue of a run with no server, so it
now reads "which an MCP server's box or lease rests on", in the same number of
lines, as `admission.rs` sits at its ceiling. On Linux a mount table that does not
prove where the root lies now refuses every boxed admission by `Unmounted`; it
used to refuse only beside a served lease or evidence.

The residual stays, as ruled. A writer Brokkr cannot box is still admitted on a
run with nothing at stake and can still forge a record there; it already holds the
user's write, so that gives it nothing more, and refusing it on every run is the
later decision ruling 21 names. It still refuses beside a live broker by
`Unconfined`. A record an earlier build sealed with a reaching writer is not
rewritten by this: `kept` and CR2's comparison still refuse beside it, which is
how the planted tests below bind them. No shipped adapter, office, recipe or
scaffold binds the root, by the ruling's text; this visit did not re-survey that.

| Test | What it pins | Mutation, and how it failed |
|---|---|---|
| `admission::tests::sources::a_boxed_writer_that_reaches_the_coordination_root_is_refused_on_every_run` | At, above, below and through a link, with nothing served and nothing at stake: `Serving { site: "work", run: "run-w", served: <root> }`, no reservation sealed; a writer apart is admitted. | The `records_kept` call removed from `compared`: `left: Ok(Held { run: "run-w", attempt: "w1", .. })`. Every boxed writer refused: `unwrap()` on the control's `Identity(Serving { .. })`. |
| `admission::tests::sources::a_bind_of_the_coordination_root_is_refused_as_the_mount_table_shows_it` | A root the shown table alone leads to the coordination root refuses; a table that does not parse refuses with `Unmounted(<root>)`. | The call removed, as above; and `reach_records` deciding by `covers` alone, without the table: `left: Ok(Held { attempt: "w1", .. })`. |
| `admission::tests::sources::a_forged_record_has_no_boxed_writer_to_forge_it` | H2's route: the forger is refused and seals nothing, while a planted record with cleared writers does admit a broker's extension, `Ok(Synced)`, which is why the door has to hold. | The call removed: `left: Ok(Held { run: "run-b", attempt: "b1", .. })`. Every boxed writer refused: `unwrap()` on the honest writer's `Identity(Serving { .. })`. |
| `admission::tests::coordination::either_fence_refuses_unboxed_writers_at_stake_and_root_reaching_boxes_on_every_run` (rebound and renamed) | Under both fences, the boxed reach refuses by the exact cause in all four places, nothing at stake among them, nothing sealed; the unboxed writer is admitted there and refused by CR2's cause in the other three; a box apart is admitted. | The call removed: `left: Ok(Held { run: "run-b", .. })`. Every boxed writer refused: `unwrap()` on run-a's control. Unboxed writers refused by the door: `unwrap()` on run-n's `Identity(Lost)`. |
| `admission::tests::sources::a_server_is_refused_beside_a_standing_writer_that_reaches_the_coordination_root` (reworked) | Its standing writer is planted now, so `kept`'s root check stays bound; refusal by the exact cause with the reservation's bytes unchanged, then `Ok(Synced)` once the planted writer is kept apart. | `.chain([&records])` removed from `kept`: `left: Ok(Synced)`. |

Each mutation was applied, run with `cargo test -p brokkr-runtime --locked --lib`
over `engine::broker::admission` or the tests named, and restored. Removing the
call also failed `a_protecting_attempt_refuses_its_own_unprotectable_writers`,
`a_writer_that_can_reach_protected_evidence_is_refused` and
`a_writer_reaching_an_operating_system_root_is_refused_before_the_survey`, whose
expectations moved to this cause. Fixtures that needed a standing root-reaching
writer (`writer_first`, `a_protecting_run_is_refused_beside_a_writer_that_can_reach_its_root`)
now plant its record with `planted_writers`, and `exposing` and `exposed` moved up
to `admission/tests.rs` because `tests/sources.rs` is Linux only. Outside that
module, `writers_tests::a_boxed_exec_attacker_that_can_reach_the_coordination_root_never_starts`
and the CLI's `writer_lease::a_real_boxed_exec_strikes_from_inside_its_lease_and_every_protected_byte_stands`
now expect this cause too. A seat that cannot nest bubblewrap skips the latter
("skipped: writer_lease: no bubblewrap namespace here"), so its pass alone proves
nothing. This visit's seat does not: a temporary `panic!` after
`refused_at_the_door`'s assertions fired ("REACHED refused_at_the_door past its
assertions"), so that real-box refusal executed here and passed once the panic was
removed. The Linux CI leg with `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1` and the
controller's host run are where it must also run.

## The chief's H1, fixed

This closes H1 for writers whose records are true, in both arrival orders. It
does not by itself hold against a writer that rewrote its own record; that is H2,
above.

The hold: with a non-retaining MCP holding and nothing else at stake, a claim
protects nothing, so `exclusion::compared` returned before comparing writers,
and `sources::kept` compared a boxed writer with the system roots and the
reserved serves but not with the coordination root. A boxed writer whose
authored hands bind that root writable could stand beside a live broker and
rewrite `reservation.json`, clearing the serves that `sources::unserved` later
trusts. The operator's standing ruling of 2026-10-28 governs: protect the
coordination records whenever a broker's source lease depends on them,
independent of retained-response policy.

`sources::kept` now takes the opened `Root` in place of its mount observer and
compares every writer with that root, after the system roots and the serves,
whenever `serves` is non-empty, refusing with MB3's exact identity cause
(`Unknown::Serving`, naming the root). Both arrival orders reach it:
`unserved` for a writer arriving beside a standing broker, and `extended_with`
for a broker extending its lease beside a standing writer. `staked` and
`compared` were not widened by this fix, and an unboxed writer still refuses
beside a live broker by `Unknown::Unconfined`, as before. H2's ruling then narrowed
ruling 21 for boxed writers.

| Test (`admission::tests::sources`) | What it pins | Mutation, and how it failed |
|---|---|---|
| `a_writer_that_reaches_the_coordination_root_is_refused_beside_a_standing_server` | A zero-grant writer reaching the root, at it and above it, refuses with `Serving { site: "work", run: "run-b", served: <root> }` and seals no reservation; one kept from the root is admitted. | Run two: `.chain([&records])` removed from `kept`'s comparison: `left: Ok(Held { run: "run-b", attempt: "b1", staked: false, .. })` against the refusal. In this run the two checks back each other here: the test passed with the `kept` root row removed, as the door refuses, and with the `records_kept` call removed, as `kept` refuses. With both removed it fails: `left: Ok(Held { run: "run-b", attempt: "b1", staked: false, .. })`. |
| `a_server_is_refused_beside_a_standing_writer_that_reaches_the_coordination_root` | Reworked by H2's fix; see its row above. | See its row above. |

The H1 mutations were recorded in run two, with `cargo test -p brokkr-runtime
--locked --lib the_coordination_root`, and restored.

## What landed in production

The MCP compile fence is chosen by host, with no knob.
`McpFence::current()` (bundle/mcp.rs) returns `Lifted` on Linux and
`Standing` elsewhere. `McpFence::lift` stays a `cfg(test)` thread-local
for the in-crate suites. The enum carries only the variant its host can
read (capabilities.rs, binding.rs, isolation.rs), so no arm is dead code.
`isolation.rs` changes by ruling (4) of 2026-10-28's two-line cfg edit.
Past the lift on Linux, `Authority::load` admits an `mcp` grant (binding.rs
`stands`) and a holding brokers its capability (`Holding::native` returns
`None`), so its native namesake stays OFF. A nonempty MCP restriction refuses
a `requires` and drops a `wants` with D11's deferred cause
(`Unserved::Restricted`), and a serving's first SI2 cause refuses
(`McpFence::admit`). On macOS every one of these stays as it was.

The other changes, by the ruling or finding each one answers:

| Ruling or finding | Change | Where |
|---|---|---|
| Ruling (2) of 2026-10-28 | SI2 asks for evidence only for a resume whose status is `supported`. An `unmeasured` or `unsupported` resume is never performed, so it is not a served shape. | bundle/mcp.rs `strictness` |
| Ruling 14 | A work seat is also served as its cold replacement. `strictness` already asks that shape; the lift makes SI2's cause refuse it on Linux, with no change to this arm. | bundle/mcp.rs `strictness`, `McpFence::admit` |
| U5a2 F1 | MB4's below-minimum cause, like D11's deferred-restriction cause, ends both GP1 forms in its own words, with no incompatibility suffix and no native tail. | binding.rs `Unserved::unheld` |
| U7d3 L3 | An inline site passes its own adapter's authentication names to MB4's collision check. | bundle/mcp.rs `inline_capabilities` |
| U7d3 L2 and ruling 5 | `brokkr init` writes the measured `harness_auth` of claude (with the twelve cloud credential-chain names), codex, dsh and exec into its scaffolds. | cli init/adapters.rs |
| Ruling 4 (A1) | An attempt that brokers a holding refuses at provisioning, before any survey or server starts, where `/proc/sys/dev/tty/legacy_tiocsti` is absent, unreadable or not `0`. The cause is `Unprovisioned::Terminal`. `Host` moved there to keep broker.rs within 800 lines. | new private child engine/broker/terminal.rs; observation.rs reads it |
| Ruling 21 of 2026-10-29 on run four | U8a2a's coordination refusals stay keyed to evidence at stake on Linux, as on macOS. `exclusion::everywhere()` and its every-run module text are gone, and `compared` asks only `staked`. | exclusion.rs; callers in admission.rs and settlement.rs |

`sources.rs` `held_extended` keeps a brokering attempt's source lease past the
lifted fence (`Unknown::Lost`). That is U8a2b's lease, not U8a2a's
evidence-keyed coordination, and the controller has accepted it. The chief's
I2 is also answered: the comments of `Unbound::Mcp`, `capabilities.rs`'s module
text and the MB4 matrix now say where the fence stands, on macOS.

## Fixture migration (tests only)

Past the lift, SI2 refuses on Linux every model serving whose adapter
measures no strict shape, an inline opaque command (`'<custom>'`) included.
The suites' fixtures move to forms that compile past the fence, with no
fixture boxed: a seat with no model surface uses the exec driver, and a
scripted driver is hired as an office over an operator adapter whose synthetic
plumbing facts are measured under its own harness, never U0's.

| New support file | What it holds |
|---|---|
| `tests/support/synthetic_mcp.rs` | The facts. |
| `tests/support/hired_office.rs` | The one writer both crates' office helpers share. |
| `crates/brokkr-cli/tests/support/fake_office.rs` | The fake-driver office, its inline form, and `measure_all`, which measures every fixture adapter that has no typed facts before a verb runs. |
| `crates/brokkr-runtime/tests/support/office.rs` | An office over a fixture's own command. In the runtime crate, `{brokkr}` names the test binary, so the exec driver cannot carry a protocol-speaking driver. |

The CLI suites migrated are agent_fallback, agent_readouts, brokkr_folder,
delivered_by_brokkr, effect_replay, folder_seats, hands, machine_proof (and
entered_run), muninn, realms, recipes, shelf, witness_journal,
driver_conformance, the lib `tests.rs`, `run_view`, `dsh_key`, and doctor's
`tests.rs` and `capability_tests`. The runtime suites are capability_launch
(redrive, servers), capability_broker_launch, capability_ledger,
node_recipe_gates, operated_repo, recovery and two_engines_one_journal. No test
moved between files.

Six expectations changed because of the lift itself, not the fixture form:

| Suite | What changed |
|---|---|
| init_doctor | dsh's `headless-work` resume is `unmeasured`, so it is not a served shape (ruling (2)), and dsh's records drop its row. |
| dsh_key | dsh's `spark` route refuses its seats on Linux, so it is no longer served from the operator's home. |
| capability_broker | An unused MCP grant compiles on Linux and still refuses exactly on macOS. |
| capability_launch | A Claude empty-set launch whose shape is measured gains `--strict-mcp-config --mcp-config {"mcpServers":{}}`. The final-check helper hands those arguments on, as the serving does. |
| doctor SD3 | The tests split by host. On Linux each seat meets its own cause: below the minimum refuses a `requires` and drops a `wants`; a grant to no office refuses with "the realm grants it to no office"; a hands-less seat clearing the minimum refuses with MB2; an unmeasured replacement or an opaque command refuses with SI2. |
| witness_journal | The non-adopting journal shape runs under the exec driver, which reports three checkpoints per attempt where the fake driver reported one. A run of identical checkpoint events reads as one; every engine event and the added-fields check stay pinned. |

## Ruling 21's cases, and their binding mutations

Each mutation below was applied and run on this base, then restored; `git diff`
against the staged tree was empty afterwards.

| Case | Proof (Linux, past the lift) | Binding mutation, and how the proof failed |
|---|---|---|
| First: nothing at stake admits unboxed writers | `capability_broker_launch::a_run_with_nothing_at_stake_admits_its_unboxed_writers`: a public engine and two hands-less offices end `(Completed, Some("done"))`. The 19 shipped recipes compile through `recipes::show_and_compile_agree_for_every_recipe_under_every_realm_map` and `witness_digests::every_bundle_in_the_tree_compiles`, and the fresh scaffolds through `init_doctor` and `init_stacks`. No shipped recipe's first seat was dispatched here, because each hires a real harness. | The `staked` early return removed from `exclusion::compared`: `(AwaitingOperator, Some("work"))` against `(Completed, Some("done"))`, with "site 'work' of run 'resolved-92faf39d' writes outside a workspace box, which cannot be kept from protected evidence or the coordination root". |
| Second: an MCP grant on boxed hands, through the real broker | **U9b1b's.** | None here. |
| Third: the same grant on a seat without hands | MB2's compile refusal stands as the proof, by the operator's ruling: "MCP capability 'library-docs' requires workspace hands at this site". `bundle::agent_tests::mcp_tests::a_set_without_the_hands_server_or_a_model_channel_carries_no_broker` pins it, and doctor's `each_seat_is_planned_past_the_realm_wide_refusal_by_the_compiles_own_seat_pass` pins it for the seat that only wants the grant, which refuses rather than drops. | `(McpIntent::Empty, Some(_))` made `Ok(McpContribution::Empty)`: both fail, doctor's with "expected a capability refusal, got Ok(())". |
| Fourth: a provider-native grant retains nothing | `doctor::capability_tests::mcp_tests::a_provider_native_grant_on_a_seat_without_hands_holds_and_retains_nothing`: `(held.len(), dialect, admitted, retention.declared) == (1, "claude-fetch", None, false)`. | `Retention::of` declaring native dialects retained: `(1, "claude-fetch", None, true)`. |

## What was observed on this base

The replay needed no change for its base, which is the third run's own. This
visit's edits are H3's fix and its tests, the two fixtures named under H3, the
wording findings (`unreserved()`'s comment, `admit_in`'s, `exclusion.rs`'s module
comment, `records_kept`'s, and the renamed either-fence test), this file and
`quality/file-lines.txt`.

| Run | Result |
|---|---|
| Formatting, clippy with `-D warnings`, whitespace | Clean. |
| `cargo test -p brokkr-runtime --locked` by binary | lib 1445, `it` 144, capability_launch 76, operated_repo 1, queued_launch 8, none failed. |
| `cargo test -p brokkr-cli --locked --test it` | 825 passed, 0 failed, 3 ignored, on the final tree. The pass before it failed only the `machine_proof` fixture named under H3. `capability_broker::cleanup::an_attempt_cancelled_during_initialization_or_a_call_settles_the_server_and_its_child`, which run three saw fail once, passed in both. |
| `cargo test -p brokkr-cli --locked --lib` | 1104 passed, 0 failed, 1 ignored. |
| driver_conformance, transcript_surfaces | 27 and 15, none failed. |
| heap_claude, heap_codex, heap_costs, heap_dsh, heap_fleet, heap_open, heap_run_view, heap_seat_view | 1, 1, 1, 1, 3, 1, 2 and 1, none failed. Some were cut off by a 590-second limit and re-run by name; each result here is from a run that finished. |
| `quality/ratchet.sh files`, `clones`, `api` | All three hold. |
| `compile --bundle recipes/self` | Printed its bundle and "policy sweep (decision 0050): 48 valuations over 11 groups, 0 unruled". |
| openspec validation, typos | 36 of 36 pass; typos clean. |
| `scripts/coverage-exact.sh`, then `quality/ratchet.sh crap` | Not run in the seat: the controller runs them on the host, and CI runs them. U9b1b's text covers them over the lift. |
| CI on macOS | Pending: this host cannot run it. |

The workspace's other crates (core, view, store, protocol, bridge,
seatbelt-probe) have no file in this tree, so their suites were not re-run.

Run five's twelve new clone fingerprints were folded, never re-baselined: the
office writer is shared through `hired_office`, witness_journal reads realms'
`POLICY`, `solo_agent` and `with_bundle` fold the solo-bundle and write-bundle
tails, the fence suite reads `search-native`'s schema back, the door test loops
its lifted case, and agent_fallback and agent_readouts keep main's bytes and
measure at the verb. `quality/file-lines.txt` gains the new files, the chief's
I1 answered (the fence suite's, the two office helpers' rows re-measured, and
`hired_office.rs` given its row), and the row files' counts. Two `#[expect(too_many_lines)]` went unfulfilled after the
folds and were removed, so `quality/suppressions.txt` goes from 245 to 243 and
`quality/too-many-lines.txt` drops the two functions.

## Tasks

No box in tasks.md is ticked. 42.2, 46.1 and 46.2 stay unticked and are U9b1b's.
The dependency lines of sections 42, 46 and 47 and the requirement coverage
audit name U9b1b. Run five recorded 8.1 and 8.2's matrix passing with strict
admission active on Linux, without the independent compiling removals 8.2
requires, and this visit recorded none either. The SD4 samples of 28.9, 28.10,
28.17, 31.1 and 31.2 stay pending, the fully cold Linux x86_64 sample being the
controller's by ruling 3.

## Owed by U9b1b

U9b1b carries the enabling matrix through real compile, launch, broker, fold
and inspect with a fake dialect, including every boxed-server seam; MB3's
two-worktree lifetime matrix; MB4's complete-frame/EOF matrix with the private
exec-status controls; the HOME/TMPDIR collisions with zero store lookups and
child starts; the success-reporting harness cases; scalar-secret refusal on the
real retention and inspect path; 42.2's real transport faults through the
settlement barrier; ruling 21's second case; SD4's timings on both Linux
profiles; and exact coverage and the crap ratchet over the lift. By the
controller's ruling it also carries three of the chief's findings. L1:
`observation.rs` is the sole production connection from `legacy_tiocsti` to
`Host::served`, and no test binds that wiring on an unsafe or unreadable host.
L2: the mutation records for the six named tests, the two new fence tests, the
two TIOCSTI tests, the changed inline-auth test and the either-fence
coordination test. L3: ruling 21's first and fourth cases through real dispatch
of a shipped recipe's first seat, a fresh scaffold and a native grant, where this
unit proves synthetic public dispatch and native retention. Until U9b1b
lands, no release is cut from main, no host engine the house pipeline runs is
upgraded past U9b1a, and no realm the house pipeline runs grants an MCP
capability.

## Findings carried, ruled LOW

These are recorded for U9b2 and follow-up issues, and not fixed here. Doctor
drifts from compile on Linux: `doctor/capabilities{.rs,/mcp.rs}`, outside this
row, still prints the realm-wide `MISSING … broker support is not implemented
until decision 0065 slice two` line for a valid `mcp` grant, "no seat holds it
until … lifts the compile fence", and "U1g1's strict MCP isolation, which
refuses only once … lifts the compile fence". Each is stale on Linux, and
nothing fails open. An inline opaque-command seat refuses at compile on Linux
past the lift (SI2 `'<custom>'`), with grants or without; no shipped recipe has
one, but an operator recipe with a custom inline driver does, and U9b2's guides
should say so. An inline seat naming an operator adapter qualifies only where
another site opens the adapters root (bundle/opened.rs).
