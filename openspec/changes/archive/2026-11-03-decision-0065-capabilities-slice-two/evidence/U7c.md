# U7c evidence: provision protected per-attempt plans

U7c is delivered by run `0065-slice-two-unit-u7c-see-the--8dd2cf29`, its
fifth run, on branch `s2/U7c` cut from main at `8819b7ee`. The fourth run
(`0065-slice-two-unit-u7c-see-the--ae14d2ff`) built `cdbc82f5` from
`6da7fc35` and opened #713. The controller's signed `559c16e5` fixed two
test-only defects CI found, and the fifth run replayed `6da7fc35..559c16e5`
onto `8819b7ee` and closed the two production defects the host's exact
gate found ("The fifth run: D1 and D2" below). The third run (`0065-slice-two-unit-u7c-see-the--ac80d841`) built
on `4257cba3` (U7c0b merged), replaying the second run's work
(c8b42582). Its first visit closed that run's review findings M1–M6 and
L1–L4 (e5aeed68). Its own review returned seven findings, R-M1 to R-M4
and R-L1 to R-L3 below. Its second visit (35a6d439) closed the five the
row reaches and stopped blocked on R-M1 and R-M4. The operator ruled both
on 2026-10-16 (design.md, U7c's "Operator rulings 2026-10-16, after the
third run's review"). This run replayed `4257cba3..35a6d439` onto
`6da7fc35`, kept it, and closed R-M1 and R-M4 as ruled ("The survey's
tether" below). Tasks 35.1 and 35.2 are ticked (requirements SD3, MB1,
MB3, CR3). Everything below was observed in these runs' seats on Linux
x86_64; what was not observed is named pending at the end.

## What changed in production

`engine/broker.rs` (new, declared `mod broker;` from `engine.rs`) is the
provisioning. After the attempt's durable
`EffectStarted` and its deadline, and before anything spawns,
`Engine::provision` collects one slot for each MCP holding the selected
link of each executing site brokers: a single seat's, each panel member's
and each sequence step's. It reads them from `selection.plans`, with the
label and instance reference `site_plans` already computed, so it reuses
the existing launch facts and adds no inventory or launch state machine of
its own. The budget is judged first, then every holding's seal, all before
the host is consulted. `share(n)` is 256 MiB divided by the number of retaining
slots, `None` where none retains, and "capability evidence budget is
exhausted" where a share would fall under 8 MiB (more than 32 slots). No
inventory field carries it.

Each slot's box is then surveyed by `surveyed`: the canonical current
binary run as `broker survey`, in a process group of its own, with the
closed `Survey` record written as one newline-terminated line on its
stdin on one thread and its stdout read on another, up to one byte past
the 1 MiB bound. The writer hands the survey's stdin back once written,
and the engine holds it open until the reply is read or the deadline
passes. The engine waits in slices of at most 100 ms until the absolute
deadline, which is the startup bound of 30 s or the attempt's own bound,
whichever is sooner. Provisioning has no other cancellation source: the
engine's own end closes the survey's stdin, which ends the survey
(operator ruling 2026-10-16). Whatever the survey answers, its stdin is
then closed, its whole group killed and the survey reaped. Only one
closed line is an answer, never the exit status, which that close may
turn into a kill: a `Provisioned`
naming the bootstrap the engine started, MB3's `Refused` cause
(`Unprovisioned::Box`), or the `Unread` cause. Anything else, or a survey
that does not start, is `Unanswered`.

The plan seals the reply's executable, tree, bootstrap and sources. It
also seals the owner, the holding's server, capability, dialect, argv,
tools, empty restrictions, binding names, its effective retention, and the
network and clearance U7c0 sealed on the holding. Its reach is the site's
effective hands: the canonical workspace, the Git common directory and
read-write binds are writable, read-only and overlay binds are readable,
and `~/` means HOME. Its writers are the engine's uid under
`Privilege::Confined`, beside the fixed environment names and the
exclusions.

The store is sealed as the broker reads it (`stored`): absolute, with
everything up to its last `..` resolved as the kernel resolves it and the
rest kept as spelled, so a symlink after the `..` stays a symlink. Only
once every plan is sealed is anything made. The directories from HOME to
the run's are walked from `/` without following a symlink and held to the
broker's protection rule. The attempt directory is made exclusively: only
`EEXIST` refuses as "already provisioned", and any other failure is a
failed write. Each plan is written owner-only through the records' one
`Serialize`, and the inventory last. A refusal appends `EffectFailed` with
its words, and no driver starts.

The host is read once per attempt, from values handed in (`Host::of`).
`HOME` is read as the broker reads it: `hands::home_dir` over the
environment's UTF-8 entries, then absolute and canonical. The binary is
the canonical current one. `provision` reads it eagerly, as a
`Result<Host, Unprovisioned>`, and `provisioned` consults it only after
the budget and the seals, and only where a site brokers a holding, so a
HOME that cannot be read refuses no attempt that brokers nothing. Tests
hand in their own and never read this process's.

Provisioning goes through `provision_with`, the closure seam that takes
the host, the survey and the instant provisioning began. `provision` is
its production caller, handing it `&surveyed` itself. `provision_with`
computes the surveys' deadline with `startup`, MB3's 30 s or the
attempt's bound, whichever comes first, and every `Surveyor` is handed
that deadline.
Where a plan was provisioned, the attempt's deadline is left holding what
remains of the bound (`left`), and `execute` hands that to the panel,
sequence or single driver. An attempt with nothing left is refused,
"the attempt's bound was spent provisioning its broker plans"
(`Unprovisioned::Spent`). A site that provisions nothing keeps its bound
whole.

| File | Change |
| --- | --- |
| `engine.rs` (4,329 to 4,346 lines) | `mod broker;`; provisioning moved after the deadline's construction, handed that bound and leaving in it what the driver gets; `site_label`, the one home of `<seat>:<tag>`, now used by `site_plans`, `boundary_entries` and `broker.rs`; `record_sites`, the provenance and boundary recording moved out of `execute` (D2) |
| `engine/broker.rs` (new, 689 lines) | the provisioning described above |
| `engine/marks.rs` (185 to 275) | `brokered` reads the holdings behind `McpContribution::Brokered`'s server set, in its order, with the variants matched exhaustively, instead of testing `contribution(..).is_ok()` and filtering MCP holdings again; `servers` clears and writes the private `capability_servers` carrier from it, beside the serving candidate's own discovery tool |
| `bundle.rs` | the one ruled line: `pub(crate) use mcp::{McpContribution, McpIntent};` (line count unchanged) |
| `brokkr-runtime/Cargo.toml` | rustix's `process` feature, which names the group kill on expiry safely; brokkr-protocol and brokkr-cli already build rustix with it, and `Cargo.lock` did not move (observed by `git status`) |
| `brokkr-cli/src/broker/survey.rs` (112 to 144) | `request` reads exactly one bounded line; `tethered` arms the watcher; `run` lets standard input's lock go before arming it |

The manifest's feature line is operator-approved: the ruling of
2026-10-16 permits rustix's `process` feature in
`crates/brokkr-runtime/Cargo.toml`, the version brokkr-protocol and
brokkr-cli already build, so that the survey's group is killed and
reaped on expiry without `unsafe`. The same ruling permits the change to
`survey.rs`, U7c0b's verb. Beside these and the ruled `bundle.rs` line, no
production file outside the row's three paths moved.

## The review's findings

| Finding | How it is closed |
| --- | --- |
| M1 | No in-engine `ServerBox::prepare` or `provision` remains. The production arrangement test drives the real binary as the survey from this ordinary, unconfined test process. On this host, whose system set holds multiply-linked root files, it provisioned, the plan sealing the copy and its own sha256 as the bootstrap and `/usr/bin/gnutrue` as the system `true`. The second run's deferral is gone. |
| M2 | Provisioning follows the deadline's construction. The survey runs as a child in its own group under the startup deadline and the attempt's bound. A stalled stand-in refused `Expired` at its deadline and `Cancelled` on cancellation, and neither it nor the child in its group survived. The attempt directory, Git discovery and the store path remain engine filesystem steps. |
| M3 | `stored` replaces `path::absolute`. Under the second run's `path::absolute`, the real `broker serve` refused the engine's plan as unbound (mutation B). |
| M4 | The engine's plan and inventory went through the real reader, `session::bound` inside `broker serve`. A substituted plan and a missing inventory each refused exactly "broker plan is not bound to this attempt", and the plan as written bound: its ledger opened beside it. A crash between `EffectStarted` and the inventory settles indeterminate through the existing recovery and is never provisioned again. |
| M5 | The budget fixture marks its held keys `docs-0` to `docs-32` `Wants`, and the refusal fixtures keep `library-docs` wanted. Mutation H, which drops wanted holdings, failed two tests. |
| M6 | Every test asserts `Unprovisioned` variants (`Unsealed`, `Box(Refusal::…)`, `Root`, `Provisioned`, `Write`, `Store`, `Expired`, `Cancelled`, `Unanswered`, `Unread`, `Bootstrap`). The operator text is pinned once, in `each_provisioning_refusal_says_its_own_words`. |
| L1 | Closed by the `bundle.rs` line and `marks.rs` above. |
| L2 | The site label now has one home. The rest are private to the CLI or the protocol and unreachable from the row: the protected-root layout, inventory name and ownership rule (`session.rs`), MB3's 30 s startup bound, and `hands::expand_home`. Each copy is bound to its authority by a writer/reader test: the real reader binds what the writer made, and the sealed reach names every root at the place `hands::box_argv` binds it. |
| L3 | `Host::live` is gone. `Host::of` takes HOME and the binary as values, and no test reaches the real HOME or `current_exe` (mutation J). |
| L4 | `written` maps only `EEXIST` to `Provisioned`. An unwritable run directory refuses `Write`. |

The environment note held: `owned()` is unchanged. The first visit's
claim that `/var/tmp`'s ancestry is protected wherever it exists was
inaccurate, and R-L1 below corrects it.

## This run's review findings

| Finding | How it is closed |
| --- | --- |
| R-M1 production cancellation; engine stop leaves no survey | Closed in the fourth run as the operator ruled on 2026-10-16. See "The survey's tether" below. |
| R-M2 the driver kept the full bound | `provision` leaves in the attempt's deadline what provisioning did not spend, and `execute` passes that same variable to each driver path. Nothing left refuses `Spent` after the durable start, and no driver starts. `startup` and `left` are pure, and `provision_with` is the engine wiring, each pinned by its own test (W, L and S below). |
| R-M3 missing removal controls | Batches 1–3 below: share arithmetic (P1, P2), every sealed fact (F, all fifteen), construction and clearing of the selected servers (C1, C2), member and selected-link ownership (O1–O3), and the startup bound (S). |
| R-M4 rustix `process` feature in the runtime manifest | Kept: the operator approved it on 2026-10-16 for the survey's group kill on expiry. |
| R-L1 fixture root | `base()` now picks the first of `/var/tmp` and the temporary directory whose every ancestor passes the writer's own `owned` rule, and returns none where neither does. Tests that write under a HOME take `Fixture::protected`. On such a host they skip with their name and reason, and they fail instead where `BROKKR_REQUIRE_BOUNDARY_EVIDENCE` is set. Fixtures that provision nothing use `scratch()`. On this host `/var/tmp` qualifies, and no test skipped. |
| R-L2 stale CLI binary | `built_brokkr` accepts the sibling `brokkr` only when no file named in cargo's dep-info for it (`brokkr.d`) is newer than it. A scratch run that replaced the skip with a panic confirmed this. After `touch crates/brokkr-cli/src/broker/survey.rs` it said "the brokkr beside this test binary is older than its sources: build it"; after `cargo build -p brokkr-cli` the test ran (5.55 s). The skip was then restored. |
| R-L3 HOME rule | `Host::of` reads HOME through `hands::home_dir` over the UTF-8 entries, as `session::trusted_home` does. A non-UTF-8 HOME is refused `Root` (mutation H). |

I1 (a `forge_run_id` that is not a portable path component) and I2 are
info findings and outside this row, so they were not acted on. The third
re-vouch's review raised I1 again as L3, and it is closed below, in
"The third re-vouch's review".

## Tests

`capability_tests/provision_tests.rs` (1,115 lines) and its new child
`capability_tests/provision_tests/survey_tests.rs` (553) hold the
provisioning tests. `capability_tests/resume_tests.rs` (58 to 167) holds
the resume proofs. `capability_tests.rs` (1,878 to 1,883, below its 1,918
baseline) gains `canonical_engine_in`, which the fixture builds through.
`call_tests.rs`'s `planned` is `pub(super)`. No test moved.
`engine/resume_tests.rs` and `capability_launch.rs` were not needed:
`capability_launch` passed unchanged.

| Test | What it pins |
| --- | --- |
| `each_retaining_slot_gets_a_disjoint_fixed_share_and_a_33rd_exhausts_the_budget` | `share` for 0, 1, 2, 3, 32 and 33 retaining slots. |
| `each_provisioning_refusal_says_its_own_words` | Every `Unprovisioned` variant's operator text, once. |
| `an_attempts_brokered_holdings_are_sealed_owner_only_and_pinned_by_its_inventory` | Two plans equal to hand-written JSON, read back through the closed types after each pin's digest is checked; locators; 0700 directories and 0600 files; each survey request exactly (launch, HOME, reach, network, writers) and the bootstrap it ran. |
| `each_sealed_fact_moves_exactly_its_own_field_of_the_plan` | Fifteen single-fact moves, the survey's bootstrap digest and sources among them, each moving only its own field, all in one assertion, so one removal build shows each fact it unbinds. |
| `every_refusal_comes_before_anything_is_made_and_spares_no_wanted_holding` | On a host whose HOME could not be read (`Err(Root)`), 33 wanted retaining slots refuse `Budget`, and an unadmitted and a URL wanted holding refuse `Unsealed`, each before the host is consulted; an unresolvable program and a refused box refuse `Box(ProgramTree)` and `Box(Linked)`; nothing is made; a symlinked and a 0750 protected root refuse `Root`. |
| `an_attempt_directory_that_cannot_be_made_is_a_failed_write` | A 0500 run directory refuses `Write`; an existing attempt directory refuses `Provisioned`. |
| `the_host_is_the_home_and_binary_it_is_given_canonical` | `Host::of` canonicalizes the HOME and binary handed in, beside a non-UTF-8 entry it ignores. A missing, relative or non-UTF-8 HOME (an existing directory) is `Root`, and an unreadable binary is `Bootstrap`. |
| `the_surveys_answer_within_the_bound_and_the_driver_gets_only_what_is_left` | `startup` is the bound under 30 s and 30 s over it; `left` gives 6 s of 10 s after 4 s, and `Spent` at and past the bound. |
| `provisioning_leaves_the_driver_only_what_it_did_not_spend` | Through `provision_with`: a site with nothing brokered keeps 10 s whole on a host whose HOME could not be read, and launches; a brokered one on that host is refused. One begun 4 s ago is surveyed with exactly the deadline `began + 10 s` and left between 5 s and 6 s. One begun 10 s ago keeps its bound untouched, answers no spawn, and journals exactly `{effect_id, attempt_id, error}` with `Spent`'s words. |
| `the_store_is_sealed_absolute_and_normal_its_symlinks_kept` | `..` and `.` resolved through the last `..`, a symlink after it kept, an unresolvable `..` refusing `Store`. |
| `the_sealed_reach_names_each_hands_root_where_the_hands_box_binds_it` | Each of the four reach roots appears in `hands::box_argv`'s argv for the same spec and HOME. |
| `a_site_without_a_brokered_holding_provisions_nothing_and_the_requested_digest_holds` | Native and `Empty`-set sites provision nothing, their host's unread HOME refusing nothing; the requested input does not move. |
| `the_selected_servers_are_named_from_the_serving_candidates_own_set_and_cleared_first` | The carrier for a Codex link with `tool_search`, `null` discovery, none for a DSH fallback with a stale one removed, the inline site's own, none after a wanted drop. |
| `each_member_provisions_its_own_selected_links_holdings_under_its_own_site` | Panel members' plans under `work:a` and `work:b` with their own instances, never the primary's. |
| `an_attempt_whose_plans_cannot_be_provisioned_fails_before_any_driver_starts` | Through `execute`: the budget and a wanted unsealed holding each journal `EffectStarted` then exactly `{effect_id, attempt_id, error}`, and no driver starts. |
| `survey_tests::a_silent_survey_is_killed_with_its_group_at_the_deadline` | A stand-in that records its pid and its `sleep` child's and never answers: `Expired` at a 1.5 s deadline, within 10 s, and both recorded processes ended. (The third run's name ended `…_or_on_cancellation` and also proved `Cancelled`; the fourth run removed that variant with the fake callback.) |
| `survey_tests::only_one_closed_reply_of_the_surveys_own_bootstrap_is_an_answer` | Stand-ins that read one request line (`read -r _`) and reply with their input still open: its own `Provisioned` is the answer; `Refused`, `Unread`, another bootstrap, two lines, a trailing blank line, an open line and a missing binary give their exact variants. |
| `survey_tests::an_engine_ended_mid_survey_leaves_nothing_of_the_survey_running` | The ruled proof, described under "The survey's tether". |
| `survey_tests::the_real_survey_provisions_from_an_unconfined_engine_and_the_broker_binds_its_plan` | The production arrangement above (M1, M3, M4, L2). |
| `resume_tests::an_attempts_budget_is_minted_once_and_never_again_on_resume` | Re-provisioning refuses `Provisioned` and moves no byte; a directory without an inventory refuses too and gains none; a new attempt gets its own. |
| `resume_tests::a_start_whose_inventory_never_landed_settles_indeterminate_and_mints_nothing_again` | `drive` reaches `EffectStarted`, and the forced failure of its refusal's own append ends it there. Its plan stands without an inventory. A resumed engine journals exactly the existing in-flight `EffectIndeterminate` and parks on it. The attempt is never provisioned again, and a new one has its own. |

The survey tests each run in an engine of their own
(`process::in_its_own_engine`, #403). In parallel with the rest of the
test binary, a concurrent test's live attempt took the survey child,
which sits outside the engine's group, for an orphan it left and killed
it. The survey then answered nothing (observed: an empty reply under
`cargo test … capability_tests`, while the same tests passed alone). In
production no attempt of the engine is live while it provisions, and the
next driver spawns only after the survey is reaped; `surveyed`'s comment
says so.

The real binary test runs the `brokkr` cargo built beside the test binary,
as a singly linked owner-only copy, but only when no source in its
dep-info is newer than it (R-L2). Where none is built, or the one built is
stale, it prints its skip and returns. Under
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` it fails instead of skipping. It skips inside a seat's hands box, as the box prepares no
server box. In this run it ran: mutations B and K made it fail. Off Linux
it asserts `Box(Unavailable)`. No test uses `setpriv`; the second run's
confined rerun test is gone with `ServerBox::prepare`.

## Removal controls

Each mutation compiled and was run against
`cargo test -p brokkr-runtime --lib --locked` filtered to the tests named.
Each failed as shown and was restored, and the full lib suite then passed
(1,007 tests).

| Mutation | Failed test, assertion |
| --- | --- |
| A the reply's bootstrap left unchecked | `only_one_closed_reply_…`: another bootstrap's reply answered `Ok` |
| C only the survey killed, not its group | `a_silent_survey_…`: the stand-in's `sleep` survived (`[3268290]` against `[]`) |
| D cancellation reported as expiry | `a_silent_survey_…`: `Expired` against `Cancelled` |
| E expiry reported as cancellation | `a_silent_survey_…`: `Cancelled` against `Expired` |
| N the one-line check dropped | `only_one_closed_reply_…`: two lines answered `Box(Linked)` |
| O `Unread` folded into `Unanswered` | `only_one_closed_reply_…`: `Unanswered` against `Unread(Truncated)` |
| B the store sealed by `path::absolute`, as the second run did | `the_real_survey_…`: the sealed store kept `work/../`; with that assertion suspended for one run, `broker serve` refused the plan "broker plan is not bound to this attempt" |
| F `stored` never resolves a `..` | `the_store_is_sealed_…`: `work/../linked/…` kept |
| F2 `stored` resolves one component past the last `..` | `the_store_is_sealed_…`: the symlink resolved to `ro/` |
| G an existing attempt directory let through | `an_attempt_directory_…`, `an_attempts_budget_…`, `a_start_whose_inventory_…`: `Write` against `Provisioned` |
| I every directory failure called `Provisioned` | `an_attempt_directory_…`: `Provisioned` against `Write` |
| H wanted holdings dropped from the slots | `every_refusal_…`: `Ok(None)` against `Budget`; `an_attempt_whose_plans_…`: `EffectSucceeded`, the driver ran |
| K the reach's `~/` left unexpanded | `the_sealed_reach_…`: `~/cache` not bound; `an_attempts_brokered_…`: the reach; `the_real_survey_…`: `broker serve` refused the plan unbound |
| J `Host::of` reads this process's HOME | `the_host_is_…`: `/home/vyanakiev` against the fixture's HOME |
| L `brokered` bypasses `McpContribution` | `a_site_without_…`: the `Empty` set read the host |
| M a refusal's words changed | `each_provisioning_refusal_…`: the cancellation text |

K and J ran in one build, as they bind disjoint tests: J failed only the
host test, and K the reach, plan and real reader tests. D, E and M bound
`Unprovisioned::Cancelled` and its words, which the fourth run removed
with the callback that could never rise. The words test now pins the
variants that remain; the fifth run re-bound it on a surviving variant
(L3 below).

The second visit's controls (R-M2, R-M3, R-L3) ran in three builds.
Within a build, each mutation binds a test of its own, and that test
failed on it. Each was run against
`cargo test -p brokkr-runtime --lib --all-features --locked
engine::capability_tests::`, then `broker.rs` and `marks.rs` were restored
from copies taken before the first mutation. The 46 tests then passed.

| Build | Mutation | Failed test, assertion |
| --- | --- | --- |
| 1 | P1 the share floor `<` made `<=` | `each_retaining_slot_…`: `share(32)` gave `Err(Budget)` against `Ok(Some(8388608))` |
| 1 | S `startup` ignores the bound | `the_surveys_answer_…`: the deadline 30 s out against 10 s |
| 1 | H `Host::of` takes the raw `HOME` entry | `the_host_is_…`: `Ok(Host { home: "…/home/\xFF", … })` against `Err(Root)` |
| 1 | C1 the selected servers never written | `the_selected_servers_…`: `None` against the named carrier |
| 1 | O1 every site's slots read from the primary's link | `each_member_…`: `work:b` sealed `stale-docs` against `docs-b` |
| 1 | W `provision_with` never shortens the bound | `provisioning_leaves_…`: the bound stayed outside 5–6 s |
| 2 | P2 the budget not divided by the slots | `each_retaining_slot_…`: `share(2)` 256 MiB against 128 MiB; the budget refusals in `every_refusal_…` and `an_attempt_whose_plans_…` failed too |
| 2 | L `left` returns the whole bound | `the_surveys_answer_…`: `Ok(10s)` against `Ok(6s)` |
| 2 | C2 the carrier not cleared first | `the_selected_servers_…`: the stale `{"servers": "stale"}` kept against `None` |
| 2 | O2 every member labelled with the seat's own label | `each_member_…` failed: it found no member's plan |
| 3 | F all fifteen sealed facts replaced by constants: network, clearance, retention, tools, the four dialect facts, binding names, argv, executable, tree, bootstrap digest, writers, sources and reach | `each_sealed_fact_…`: every one of the fifteen entries `[]` against its own field in the one assertion; `an_attempts_brokered_…`: the plans |
| 3 | O3 every slot takes the first plan's instance | `each_member_…`: `work:b` owned by instance `0…0` against `1…1` |

Under P2 the one test that goes through `execute`, `an_attempt_whose_plans_…`,
was no longer refused for its budget. It went on through production
`provision`, which ran the test binary as the survey and got
`Unanswered`. That happens only under the mutation. Unmutated, the
budget refuses before the host is consulted.

## Gates observed

The fifth run's gates ran after its last source edit, every one locked.
The runtime lib passed 1,010 tests, and `capability_launch`, `it`,
`operated_repo` and `queued_launch` passed 76, 132, 1 and 6. The CLI lib
passed 856 with 1 ignored, of which `broker::` was 158; the CLI `it`
binary passed 105 under `capability_broker::` and 583 with 3 ignored in
everything else. The format check, workspace clippy with all targets and
features and warnings denied, `typos --hidden` and `git diff --check`
(worktree and index) found nothing. `openspec validate --all --strict`
passed 26 of 26. `quality/ratchet.sh` said "file size holds",
"duplication holds" and "public API holds", and files and clones held
again after this record's last edit. `recipes/self` compiled to 47
valuations over 11 groups, 0 unruled. `cargo +nightly-2026-09-05
llvm-cov --branch -p brokkr-runtime --lib` measured `engine/broker.rs` at
41 of 41 functions, 362 of 362 lines and 4 of 4 branches, and
`record_sites` at every line with both arms of both its branches taken.
None of the survey proofs skipped: the runtime filter
`provision_tests` printed no skip line.

The fourth run's own gates, all features and locked, after the last
source edit and against a `brokkr` built after it:

| Check (fourth run) | Result |
| --- | --- |
| Runtime lib | 1,010 passed |
| Runtime `capability_launch`, `it`, `operated_repo`, `queued_launch` | 76, 132, 1 and 6 passed; `witness_digests` held unblessed |
| CLI lib (whole) | 856 passed, 1 ignored |
| CLI `it`, `capability_broker::` | 105 passed |
| CLI `it`, everything else | 582 passed, 3 ignored, and `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice` failed: it pins every caller of `std::io::stdin(` in the broker, and `tethered` is a new one. Its pin was extended to `survey.rs`'s `run` and `tethered`, and `machine_proof::output_edge` then passed, 4 tests |
| CLI `driver_conformance`, `transcript_surfaces`, `heap_claude`, `heap_codex`, `heap_dsh` | 27, 14, 1, 1, 1 passed |
| `recipes/self` compile | 47 valuations over 11 groups, 0 unruled |
| Format check, workspace clippy with warnings denied, `typos --hidden`, `git diff --check` | no finding |
| `openspec validate --all --strict` | 26 passed, 0 failed |
| `quality/ratchet.sh` files, clones and api | "file size holds", "duplication holds", "public API holds" |

bridge, core, store, view and protocol were not touched and not re-run.
`quality/file-lines.txt` was again not replayed by prep, in either run.
In the fifth run its rows for every file this unit changed were set from
`wc -l` over `git ls-files '*.rs'`, and compared against that listing
row by row (they matched): `engine.rs` 4,346, `engine/broker.rs` 689 (new
row), `engine/marks.rs` 275, `bundle.rs` 6,663 (main's stale 6,666),
`survey.rs` 144, `survey/tests.rs` 148, `capability_broker/survey.rs`
255, `capability_tests.rs` 1,883, `provision_tests.rs` 1,115 and
`provision_tests/survey_tests.rs` 553 (new rows),
`capability_tests/resume_tests.rs` 167 and `machine_proof/output_edge.rs`
201. The full listing differs from main's file in hundreds of rows that
other merged changes left stale. The seat's sandbox refused
`measure.sh`'s step as a script, and the fourth run moved only this
unit's rows, so no other row moved.

The third run's first visit observed the table below. Its second visit changed only
runtime files, the tasks and this record. It re-ran every runtime test
target with all features, locked: lib 1,009, `capability_launch` 76, `it`
132, `operated_repo` 1 and `queued_launch` 6, all passed. The three survey
tests passed against a `brokkr` built after the last source edit. It also
re-ran format, workspace clippy with warnings denied, typos, the
whitespace check, strict OpenSpec validation and `quality/ratchet.sh`
files and clones, as the closing commit records.

| Check (first visit) | Result |
| --- | --- |
| Runtime lib, all features | 1,007 passed |
| Runtime `it`, `capability_launch`, `operated_repo`, `queued_launch` | 132, 76, 1 and 6 passed; `witness_digests` held unblessed, so no witness pin moved |
| CLI lib; CLI `it`; `driver_conformance`; `heap_claude`, `heap_codex`, `heap_dsh`; `transcript_surfaces` | 855 passed, 1 ignored; 683 passed, 3 ignored; 27; 1, 1, 1; 14 |
| bridge, core, store, view, protocol (all features, locked) | every binary passed (protocol lib 943, 3 ignored) |
| `recipes/self` compile | 47 valuations over 11 groups, 0 unruled |
| Format, workspace clippy with warnings denied, typos, whitespace check | clean |
| `quality/ratchet.sh` files, clones, api | file size holds, duplication holds, public API holds |

`quality/file-lines.txt` was re-measured by `wc -l` for every file this
unit changed. The rows for `engine/broker.rs`, `provision_tests.rs` and
`provision_tests/survey_tests.rs` were added, and `engine.rs`'s stale
4,858 was lowered to 4,333. No function crossed 100 lines (clippy's
`too_many_lines` is denied and was clean), and no suppression was added,
so `too-many-lines.txt` and `suppressions.txt` do not move. No witness or
budget input moved.

## The survey's tether (operator ruling 2026-10-16; R-M1, R-M4)

The third run's second visit stopped blocked: the survey led its own
unregistered group with no tie to the engine's life. The engine's stop,
`process::attempts::stop_on_signals`, is installed only from the first
driver on, and before it SIGTERM's default action ends the engine alone.
So stopping the engine mid-survey could leave the survey running, and
the `|| false` cancellation could never rise. The operator ruled on
2026-10-16 that the survey ends with the engine, whatever ends it, as
`serve`'s observer ends with its broker (`session::tethered`), and that
the runtime manifest keeps rustix's `process` feature.

`broker survey` (`crates/brokkr-cli/src/broker/survey.rs`) now reads
exactly one request line: `request` reads through `take(1 MiB)` up to the
first newline and nothing after it. A line that ends in a newline is
parsed whole, and anything but the one closed record is `Malformed`. A
bound's worth with no newline is `Oversized`, and input that ends before
a newline is `Truncated`. A read error is `Input`. Once a request has
read, `tethered` checks that the survey leads its own process group,
refusing `Refusal::Identity` otherwise so that a caller's group is never
killed. It then spawns a watcher that copies the rest of standard input
to a sink and, at its end or on an error, calls
`kill_current_process_group(Signal::KILL)`. `run` reads the request in a
statement of its own, so the `StdinLock` temporary is let go before the
watcher takes the lock (see W2 below).

`engine/broker.rs` writes the request as one newline-terminated line. The
writer thread returns the survey's `ChildStdin`, and its join handle is
held until `replied` returns, so the survey's input stays open until the
reply is read or the deadline passes. The input is then closed, the group
killed and the survey reaped. The engine's end, by a stop signal before
its handler exists, by that handler's stop or by its death, closes the
pipe, and the watcher kills the group. `Startup`'s cancellation callback
and `Unprovisioned::Cancelled` are gone, because provisioning has no
other cancellation source; `provision`'s comment says so. The group kill
on expiry stays.

Test changes, with no test moved:

| Test | What it pins |
| --- | --- |
| CLI `broker::survey::tests::a_request_line_is_read_within_its_bound` (was `a_request_is_read_whole_within_its_bound`) | A line padded to exactly 1 MiB, its newline included, reads; one byte more is `Oversized`; 1 MiB with no newline is `Oversized` |
| CLI `…::a_request_whose_input_ends_before_its_newline_is_truncated` (was `a_request_that_ends_inside_its_record_is_truncated`) | The whole record without its newline, the record cut short, and empty input are each `Truncated` |
| CLI `…::a_request_line_ending_inside_its_record_is_malformed` (new) | A cut record followed by a newline is `Malformed` |
| CLI `…::a_line_holding_two_requests_is_malformed_and_nothing_after_the_line_is_read` (replaces `a_request_followed_by_another_is_malformed`) | Two records on one line are `Malformed`; after a good line, the bytes that follow are left unread in the input, exactly |
| CLI `…::a_request_repeating_a_field…`, `…naming_a_field…`, `input_that_cannot_be_read…` | As before, each line now newline-terminated (the broken input wrapped in a `BufReader`) |
| CLI `it` `capability_broker::survey::*` (existing three) | The survey run in its own group as the engine runs it, its input held until it ends, except the truncated case, which closes it; exit codes, replies and the empty stderr are unchanged |
| CLI `it` `capability_broker::survey::a_survey_leading_no_group_of_its_own_refuses_identity` (new) | Run in the test's group, it replies exactly `{"refused":"identity"}` and exit 1, and the test lives on |
| CLI `it` `capability_broker::survey::the_end_of_its_input_after_the_request_kills_the_survey_and_its_group` (new) | Its stdout is a stream whose buffer is already full, so its reply write blocks after the request is read and the watcher armed. It is still running 500 ms on. Closing its input ends it within 10 s by signal 9, and `kill(-pgid, 0)` then gives `ESRCH` |
| Runtime `survey_tests::an_engine_ended_mid_survey_leaves_nothing_of_the_survey_running` (new, Linux) | See below |
| Runtime `only_one_closed_reply_…` | Stand-ins read one line (`read -r _`) and reply with their input open |
| CLI `it` `machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice` | Its `std::io::stdin(` pin extended to `tethered` |

The runtime proof runs a real engine process. It is this test binary,
re-executed under `strace` (`-f --seccomp-bpf`, `trace=%file`,
`inject=%file:delay_enter=600000000`, `-P <held HOME>`) with an env pair
that makes it call `surveyed` with the real `brokkr` and a request whose
HOME is the held path. Only the survey names that path, in
`canonical_home`, its first act after the request is read and the watcher
armed. So the survey stalls there with its watcher running. The parent
waits until the survey's first thread has been in tracing stop for a
whole second with two threads standing. It then sends the engine SIGTERM
(the test binary installs no handler, as the engine installs none before
its first driver) or, in a second round, SIGKILL.

Each round asserts that, within 10 s, no member of the survey's group runs
unkilled. It then kills the tracer and asserts that no member runs at all
within another 10 s, and that the engine is gone. "Unkilled" excludes a
member on which SIGKILL pends. A diagnostic run showed why: after the
engine's end, the watcher had exited and the survey's remaining thread
stood in `t (tracing stop)` with `ShdPnd: 0000000000000100` (SIGKILL) and
`PF_SIGNALED` in its stat flags. strace's delayed stop holds a killed
tracee at its exit, which no untraced process sees. Under W, nothing is
pending, and the survey counts as a survivor. The parent runs in an engine
of its own (`in_its_own_engine`), as the other survey tests do. The test
skips with its reason, and fails under
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE`, inside a seat's box, without a
current `brokkr`, or where no `strace` traces a child. In this run it ran:
W and X made it fail.

| Mutation (compiled, run, restored) | Failed test, assertion |
| --- | --- |
| W the watcher kills nothing (`kill_current_process_group` not called) | runtime `an_engine_ended_…`: `("Signal::TERM", [788498], [])` against `("Signal::TERM", [], [])`; CLI `the_end_of_its_input_…`: `None` against `Some(9)` |
| W2 `StdinLock` held across the survey (the first draft of `run`, matching on `request(&mut stdin().lock())`) | runtime `an_engine_ended_…`: the survey stayed, its watcher in `futex_do_wait` on the lock; the CLI test passed, as its stall comes after the lock is let go. Found by the proof and fixed |
| X the engine closes the survey's input once the request is written | runtime `the_real_survey_…`: `Err(Unanswered)` (the survey killed itself before replying); runtime `an_engine_ended_…`: "the survey … was never held" |
| Y the leader check dropped | CLI `a_survey_leading_no_group_…`: `{"refused":"program-tree"}` against `{"refused":"identity"}` |
| R1 the line bound read one byte past 1 MiB | CLI lib `a_request_line_is_read_within_its_bound`: `Ok(Survey {…})` against `Err(Oversized)`; CLI `it` `a_request_it_cannot_read_…` then waited past its timeout for a byte past the bound |
| R2 the request read to the end of input, not one line | CLI lib `a_line_holding_two_requests_…`: `Err(Truncated)` against `Ok(Survey {…})` |
| N2 the engine's request sent without its newline | runtime `only_one_closed_reply_…`: `Err(Expired)` against `Ok(Provisioned {…})`; runtime `the_real_survey_…`: `Err(Expired)` |

Y and R1 ran in one build, as they bind different tests. Each mutation was
restored before the next, and the gates above ran on the restored tree.

## The fifth run: D1 and D2

After the fourth run opened #713, CI and the controller's host check found
two test-only defects, and the controller fixed them in the signed
`559c16e5`. The survey proofs now find the built `brokkr` and its dep-info
under cargo's build-dir layout, which the exact gate's toolchain uses
(`profile` and `dep_info` in `survey_tests.rs`). The non-UTF-8 HOME
directory in `the_host_is_the_home_and_binary_it_is_given_canonical` is
made only on Linux, because APFS refuses that name with `EILSEQ`. Prep
replayed `6da7fc35..559c16e5` onto `8819b7ee`. `git diff 559c16e5`
showed only main's four dsh-console files beside this run's own edits, so
the replay, the controller's fix included, stands as committed.

The host's exact gate then reached every line and branch of the
workspace but 7,393 of 7,395 functions, and the CRAP ratchet refused
`Engine::execute`.

**D1, two functions never called.** These were the two closures in
`Engine::provision`, `live` (`Host::of` over this process's environment
and `current_exe`) and `survey` (`surveyed` under the deadline). Tests
drove `provision_with` with stand-ins, and in a test binary
`current_exe` is the test binary, so no test could reach either closure.
Both are gone, as the controller recommended. `provision` now reads the
host eagerly as a `Result<Host, Unprovisioned>` and passes `&surveyed`
itself. `Surveyor` takes the deadline (`&dyn Fn(&Path, &Survey, Instant)
-> …`). `provision_with` computes the deadline with `startup` and hands it
to `provisioned`, which hands it to each survey. `provisioned` still
consults the host only after the budget and every seal, and only where a
site brokers a holding. Every line of `provision` therefore runs on every
attempt the engine tests execute, and the lib-only coverage run above
counted every `broker.rs` function as called. The fixture's `unread`
now hands in `Err(Unprovisioned::Root)` where it handed a host closure
that panicked. The budget, unsealed, native and `Empty` cases keep their
assertions, now on a host that could not be read.
`provisioning_leaves_the_driver_only_what_it_did_not_spend` binds the
ruled behaviour: a bad HOME refuses an attempt that brokers a holding and
never one that brokers none. It also binds the deadline each survey is
handed.

**D2, `execute` over its baseline.** U7c's `if !self.provision(..)? {
return Ok(()); }` had raised `Engine::execute` from its baseline of 22 to
24. Its two `if let`s before `EffectStarted`, which record the
provenance and the boundary entries, moved with their comment into the
private `Engine::record_sites` in `engine.rs`. The `match body` arms and
the body-name match were left as they stand, for #709. By hand,
`execute` is 24 − 2 = 22 and `record_sites` is 1 + 2 = 3. `cargo crap`
0.5.0, the ratchet's tool, over this run's lib-only LCOV gave the same
counts: `Engine::execute` cyclomatic 22.0, `Engine::record_sites` 3.0 at
100% coverage, and every `broker.rs` function at 10 or under at 100%
(`provisioned` 10, `written` 8). The CRAP ratchet itself, over the exact
gate's LCOV, is the controller's to run.

Each mutation below compiled, was run, failed as shown and was restored.
The suites in "Gates observed" then passed on the restored tree. V1–V5
ran against `cargo test -p brokkr-runtime --lib --locked`, filtered to
`capability_tests` for V1–V3 and unfiltered for V4 and V5. L2 ran against
`cargo test -p brokkr-cli --lib --locked broker::survey`, which then
passed 8 of 8 restored. L3 ran filtered to the words test.

| Mutation | Failed test, assertion |
| --- | --- |
| V1 the host consulted before the budget (`host?` moved to the top of `provisioned`) | `a_site_without_…`: `Err(Root)` against `Ok(None)`; `every_refusal_…`: `Err(Root)` against `Err(Budget)`; `provisioning_leaves_…`: the attempt with nothing brokered refused on the unread HOME |
| V2 a refusal of `Root` launches anyway (`provision_with` returns `Ok(true)` for it) | `provisioning_leaves_…`: `assert!(!refused)` for the brokered attempt on the unread HOME |
| V3 the surveys handed MB3's deadline alone (`began + STARTUP`, the bound ignored) | `provisioning_leaves_…`: the deadline 30 s after `began` against `began + 10 s` |
| V4 `record_sites` never records the boundary | `boundary_tests::effect_started_carries_the_boundary_beside_provenance` |
| V5 `record_sites` drops the provenance | `notice_tests::a_started_seat_that_fails_is_retried_on_its_own_link_with_its_own_notice`, `resume_tests::a_chain_fallback_is_handed_no_session_at_all` |
| L2 a line that ends inside its record classified `Truncated` (`survey.rs`'s `request`) | CLI `a_request_line_ending_inside_its_record_is_malformed`: `Err(Truncated)` against `Err(Malformed)`, and the three other `Malformed` tests |
| L3 `Unprovisioned::Store`'s words changed to "…does not exist" | `each_provisioning_refusal_says_its_own_words`: `"the secret store's path does not exist"` against `"the secret store's path does not resolve"` |

The fourth run's review left L1 open, and it stays open here. `survey.rs`
maps a failure to lead its own group, and a failure to spawn the watcher
thread, to `Refusal::Identity`. A cause of their own needs a protocol
change, which this unit may not make.

## Re-vouch after #709

The fifth run judged 968e6a51 on 8819b7ee. Story #349's slice 6 (#709)
merged first as e1448f0f, and the controller rebased this unit onto it as
98969088. This run's worktree arrived with the net diff staged on
e1448f0f, and `git diff --cached 98969088` printed nothing, so the staged
tree is that commit's. `git range-diff 968e6a51^..968e6a51
e1448f0f..98969088` showed that four files moved against the judged
patch, and nothing else did.

In `bundle.rs` the one conflict kept #709's `pub use
invocation::Invocation;` and this unit's `pub(crate) use
mcp::{McpContribution, McpIntent};` side by side, which is the widening
the operator's ruling of 2026-10-15 allows. The file's length did not
change: it is 6585 lines on main and here. In `engine.rs`, #709 respelled
execute's panel arm as `ExecutableBody::Invocation(Invocation::Panel {..})`.
This unit's `record_sites` call, its provisioning call after the deadline
and the `site_label` helper sit beside that arm and are the same as in the
judged patch. The file measures 4331 lines with `wc -l` (main has 4314).
`grep -n` places `seat_input` at 1094, `execute` at 1290 and `decide` at
2122. Those are the rows `quality/too-many-lines.txt` now carries, with
`execute` at 151 lines against main's 152. In `quality/file-lines.txt`
only the engine.rs and bundle.rs rows changed from the judged patch, and
both now give the measured counts. No other row was regenerated.

On the CI failure, `survey_tests.rs:430` is `child_of`'s 30-second wait.
It is reached either because no tracee under strace appeared, or because
the engine's survey ended before the 20 ms poll saw it. Two causes were
ruled out by reading the code. First, nothing beside the survey can be
a second child. The re-executed engine reaches `surveyed` before anything
else, and it spawns exactly one child. Second, the stray reaper of the
parent test binary cannot reach the survey: `attempts::adopted` kills
only an engine's own children outside its group, and the survey is that
binary's great-great-grandchild. Commit #714, the other commit between
the two bases, changes no file on this test's path. The failure did not
reproduce here. The test passed inside the whole runtime lib suite (1011
passed, 12.76 s), with the other 1010 tests running concurrently as CI
runs them. It also passed alone five times, in 2.16 s to 2.18 s each. That
time shows the body ran rather than skipped, since `held` waits a full
second for each of the two signals. The CI failure is therefore recorded
as an unreproduced CI flake, and the test file is unchanged.

The gates were rerun on the staged tree, with these results. Format and
workspace clippy were clean. The runtime lib passed 1011 tests, and the
runtime `capability_launch` and `it` targets passed 76 and 132. The CLI
lib passed 164 tests under `broker::`, and its `it` target passed 172
under `capability_broker::` and `machine_proof::`. Typos and `git diff
--cached --check` reported nothing. `openspec validate` passed 26 of 26.
The files, clones and api ratchets all held, and `recipes/self` compiled.
Exact coverage, the CRAP ratchet, remote CI and macOS stay pending as
above.

The first re-vouch's head went 21 of 21 green on the workstation runner,
then fell out of the merge queue. On the hosted runner that runs the
merge group's test (ubuntu-latest), both real-survey proofs refused under
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` with "the brokkr beside this test
binary is older than its sources". Cargo writes the uplifted
`target/debug/brokkr.d` only when the binary is built on its own, with
`cargo build`. A `brokkr` built only because an integration test needs
it, as `cargo test --workspace --no-run` and the CI shards build it, has
no such file. `built_brokkr` then read an empty dep-info and judged the
binary stale. The workstation runner had passed only because its kept
target still held an older `brokkr.d`. The controller's commit 9860e5e5
changes the test file alone. When there is no dep-info beside the binary,
`dep_info` now looks in the profile's `deps` for the hashed copy the
binary is a hard link of, matched by device and inode. It looks there as
it already looked in the build-dir layout's
`build/brokkr-cli/<hash>/out`, and it reads the compiler's dep-info
beside that copy. The file's `quality/file-lines.txt` row is 552, the
count `wc -l` gives. This run reproduced the controls on its own target.
It built with `cargo test --workspace --locked --no-run`, deleted
`target/debug/brokkr.d`, and ran the runtime lib's `survey_tests` with the
variable set through `--config 'env.BROKKR_REQUIRE_BOUNDARY_EVIDENCE="1"'`.
All four passed. Touching `crates/brokkr-cli/src/bin/brokkr.rs` made both
real-survey proofs panic with the stale refusal. Restoring the file's time
with `touch -r` brought all four back to passing. Reverting `dep_info` to
the pre-fix form, which returns only the file beside the binary, made the
same two proofs fail with the hosted runner's message (2 passed, 2
failed). Restoring the fix returned them to passing. The gates listed in
the paragraph above were rerun on this tree and gave the same counts. The
CLI `it` target's 172 were run in two parts: 106 under
`capability_broker::` and 66 under `machine_proof::`.

The second re-vouch's head, 709696c2, then failed both real-survey proofs
in PR CI on the workstation runner, with the same "older than its
sources". A self-hosted runner keeps one target across jobs and pull
requests. A `cargo build` of another tree leaves `target/debug/brokkr.d`
naming that tree's sources, and a later test build relinks the binary
without rewriting that file. `dep_info` preferred any `brokkr.d` it found,
so a source this tree lacks read as missing and the binary as stale. The
controller's commit 6db34933 trusts cargo's `brokkr.d` only where it is
no older than the binary, since cargo writes the two together; otherwise
`dep_info` reads the compiler's dep-info beside the hard-linked copy. The
doc comment says so, and the file's `quality/file-lines.txt` row is 560,
which `wc -l` gives. This run repeated the controls on its own target
after `cargo test --workspace --locked --no-run`, with
`BROKKR_REQUIRE_BOUNDARY_EVIDENCE` set through `--config` as above. It
wrote a `brokkr.d` naming a missing source and dated it an hour before
the binary with `touch -r ... -d '-1 hour'`; all four `survey_tests`
passed. The same file dated after the binary made both real-survey proofs
refuse as stale, which is the rule's other side: a fresh `brokkr.d` is
believed. With `brokkr.d` deleted, all four passed. Touching
`crates/brokkr-cli/src/bin/brokkr.rs` made both real-survey proofs refuse
as stale (2 passed, 2 failed), and restoring its time with `touch -r`
returned them to passing. With the hour-old file back and the guard
temporarily replaced by the previous rule (any `brokkr.d` that exists is
used), the same two proofs refused (2 passed, 2 failed); the test file
was then restored to the staged bytes and the real `brokkr.d` put back.
The gates were rerun on this tree: format and workspace clippy clean; the
runtime lib 1011, `capability_launch` 76 and `it` 132; the CLI lib 164
under `broker::`, and its `it` target 106 under `capability_broker::` and
66 under `machine_proof::`; typos and `git diff --cached --check` silent;
`openspec validate` 26 of 26; the files, clones and api ratchets held;
`recipes/self` compiled.

## The third re-vouch's review

The review of aea95a7e returned the unit as residual, with M1 medium and
L1, L2 and L3 low.

| Finding | How it is answered |
| --- | --- |
| M1 a stale binary passes when cargo's `brokkr.d` is absent or older | Closed in the test file. The compiler's dep-info for the hard-linked copy names only `crates/brokkr-cli/src/bin/brokkr.rs`. `built_brokkr` now also requires that no workspace source named by the compiler's dep-info of any library cargo built in the binary's profile is newer than the binary. Those libraries are read from `deps`, and from `build/<package>/<hash>/out` under the build-dir layout (`library_sources`, `outputs`). Cargo does not record which library variant the binary linked, so every variant counts. A variant the binary did not link can only make it read stale, never current. A source this tree lacks is skipped, since a kept target holds other trees' libraries. |
| L1 survey's group and watcher failures map to `Refusal::Identity` | Open, as recorded above: a cause of their own needs a protocol change outside this row. |
| L2 protected-root layout, inventory name, startup bound and home expansion restated | Open, as recorded in the first review's L2: the owners are private to the CLI and the protocol, and each copy is bound by a writer/reader test. |
| L3 the run id walked as a path | Closed in `engine/broker.rs`. `protected` refuses `Root` for any name that is not one plain name, `Path::new(name).file_name() == Some(name)`, before it makes or opens it. That covers the run id, which `DispatchEnvelopeV2::verify` admits with separators. The new test `a_run_id_that_is_not_one_plain_name_reaches_no_directory` provisions under the run ids `../escaped`, `a/b`, `.` and `..`. Each refuses `Root`, and the capabilities directory then holds only the repo's directory, which is empty. |
| G1 gate-directed prose in the panel | Not acted on, as the chief ruled. |

These are the M1 controls. The real `target/debug/brokkr.d` was set
aside, the binary was current, and the runtime lib's `survey_tests` ran
with `BROKKR_REQUIRE_BOUNDARY_EVIDENCE` set through `--config`. All four
passed. A changed library source was then tested by touching
`crates/brokkr-cli/src/broker/survey.rs`, which a runtime-only test build
does not relink. Both real-survey proofs refused as stale (2 passed, 2
failed). With `survey_tests.rs` checked out at aea95a7e, the same state
passed all four. That reproduces M1: the previous rule ran a binary
older than a library source it links. The new file was then restored.
After `survey.rs`'s time was restored with `touch -r`, all four passed.
Touching `crates/brokkr-protocol/src/lib.rs` made the test build rebuild
brokkr-protocol and brokkr-runtime, the library newer than the binary.
Both real-survey proofs refused as stale (2 passed, 2 failed), and
restoring its time brought them back. With cargo's real `brokkr.d` back,
all four passed. A `brokkr.d` naming a source this tree lacks, dated an
hour before the binary, also passed all four. With that same file and
`survey.rs` touched, both real-survey proofs refused (2 passed, 2
failed). The touched times and the real `brokkr.d` were then restored.

The L3 mutation replaced the guard's `?` line with `let _ = plain;`.
`a_run_id_that_is_not_one_plain_name_reaches_no_directory` then failed
at its first case: `../escaped` provisioned
`capabilities/<repo>/../escaped/attempt-1` instead of refusing `Root`.
The guard was restored.

The gates were rerun on this tree. Format and workspace clippy were clean
after clippy's `cloned_ref_to_slice_refs` in the new test was fixed. The
runtime lib passed 1012 tests, its `provision_tests` passed 20 with
boundary evidence required, and `capability_launch` and `it` passed 76
and 132. The CLI lib passed 164 under `broker::`, and its `it` target
passed 106 under `capability_broker::`. Typos and `git diff --check`
reported nothing, `openspec validate` passed 26 of 26, and `recipes/self`
compiled. `quality/file-lines.txt` rows were set by hand to the `wc -l`
counts: `engine/broker.rs` 694, `provision_tests.rs` 1137 and
`survey_tests.rs` 605. This seat was not permitted to execute
`quality/ratchet.sh`, so the files, clones and api ratchets are pending
for this tree.

## Assumptions and follow-ups

L1 of the fourth run's review is a follow-up: the survey's group-leader
failure and its watcher-spawn failure should each have a protocol
`Refusal` of their own, not `Identity`.

The inventory pins only the selected link's holdings at each site (D6). A
fallback is a later attempt with its own inventory and budget, so the
permitted fallback slots are counted where the fallback is the selected
link. U8b derives the same share from the sealed inventory on the
broker's side.

The `capability_servers` carrier (`servers` of `{server, capability,
tools}` and `discovery_tool`) is U7d's to parse once.
`engine/sequence.rs` still spells a step's label itself; it was not
changed in this unit.

Mutation C left its stand-in's `sleep 600` running; it ends by itself
within ten minutes of the run.

## Pending

The exact coverage gate and the CRAP ratchet on the host (run by the
controller), remote CI and macOS are pending until their results exist,
and so are the files, clones and api ratchets on the third re-vouch's
answer.
The strace proof and its helpers are behind `cfg(target_os = "linux")`;
the tether and the CLI survey tests use only calls macOS also has, but
the macOS build was not observed.
